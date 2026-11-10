// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stored dictionary projection through the shared RDF term spelling and drain.

use super::{RdfDiagnostic, SinkImporter, dictionary_import_failure};
use purrdf_core::ir::NativeBuildError;
use purrdf_core::ir::validate::{self, Kind};
use purrdf_core::sink::{ByteDrain, TextSink};
use purrdf_gts::dictionary::{DictionaryError, TermKey, TermLease, TermStorage};
use purrdf_gts::model::{Diagnostic, OpaqueNode, Signature, StreamableInfo, Suppression, TermKind};
use purrdf_gts::reader::{FrameContext, ReadOptions, StreamingReadResult};
use purrdf_gts::segment_decode::{ResolutionStorage, ResolvedSink, SegmentResolver};
use purrdf_lex::allocation::{Admission, Memory};
use purrdf_lex::cbor::{ReplayStorage, Value};
use purrdf_lex::term_syntax;
use purrdf_lex::text_out::TextOut;
use purrdf_lex::walk::WorkList;

/// Completed RDF text projection and original container sidecar evidence.
#[derive(Debug)]
pub struct GtsTextExport {
    /// Bytes accepted by the caller's drain.
    pub written: u64,
    /// Container metadata, blob references, suppressions and signature records.
    pub envelope: crate::RdfEnvelope,
    /// Original diagnostics and ordered segment state.
    pub read: StreamingReadResult,
}

#[derive(Clone, Copy)]
enum Task {
    Term(TermKey),
    Text(&'static str),
}

struct Emitter<'o, 'm, 'a, T, A: Admission + ?Sized> {
    dictionary: T,
    output: TextSink<'o>,
    memory: &'m mut Memory<'a, A>,
    diagnostic_bytes: &'m std::cell::Cell<usize>,
    metadata: SinkImporter<'static>,
}

impl<T: TermStorage, A: Admission + ?Sized> Emitter<'_, '_, '_, T, A> {
    fn native_error(&self, error: NativeBuildError) -> RdfDiagnostic {
        match error {
            NativeBuildError::Storage(error) => error.into(),
            NativeBuildError::Diagnostic(diagnostic) => match diagnostic.retained_bytes() {
                Some(bytes) => {
                    self.diagnostic_bytes.set(bytes);
                    diagnostic
                }
                None => {
                    drop(diagnostic);
                    purrdf_lex::allocation::StorageError::SizeOverflow.into()
                }
            },
        }
    }

    fn composite_error(&mut self, error: purrdf_core::cdt_blank::CdtBlankError) -> RdfDiagnostic {
        match error.into_diagnostic_with_memory(self.memory) {
            Ok(diagnostic) => self.native_error(NativeBuildError::Diagnostic(diagnostic)),
            Err(error) => error.into(),
        }
    }

    fn term_diagnostic(
        &mut self,
        code: &str,
        detail: &str,
        segment: u64,
        term: usize,
    ) -> Result<RdfDiagnostic, RdfDiagnostic> {
        let mut diagnostic = RdfDiagnostic::try_error_with_memory(code, &detail, self.memory)?;
        let location = crate::RdfLocation::logical(self.memory.string("gts:sink")?)
            .with_gts_segment(segment)
            .with_gts_term(term as u64);
        diagnostic.location = Some(self.memory.boxed(location)?);
        Ok(self.native_error(NativeBuildError::Diagnostic(diagnostic)))
    }

    fn checked_literal(
        &mut self,
        key: TermKey,
        lexical: &str,
        datatype: Option<TermKey>,
        language: Option<&str>,
        direction: Option<&str>,
    ) -> Result<TermKey, RdfDiagnostic> {
        let datatype = datatype.map(|key| self.term(key)).transpose()?;
        if datatype
            .as_deref()
            .is_some_and(|term| term.kind != TermKind::Iri)
        {
            return Err(self.term_diagnostic(
                "rdf-ir-literal-datatype-not-iri",
                "GTS literal datatype must resolve to an IRI",
                key.segment,
                key.term,
            )?);
        }
        let direction =
            crate::gts_resolve::parse_gts_direction_with_memory(direction, language, self.memory)
                .map_err(|error| self.native_error(error))?;
        let datatype = crate::RdfLiteral::datatype_iri_from_parts(
            datatype.as_deref().and_then(|term| term.value.as_deref()),
            language,
            direction,
        );
        let baseline = self.memory.admitted_bytes();
        let bound = purrdf_core::cdt_blank::bind_cdt_blank_labels_with_memory(
            lexical,
            datatype,
            purrdf_core::cdt_blank::BlankBinding::Ambient(super::segment_scope(key.segment)?),
            self.memory,
        )
        .map_err(|error| self.composite_error(error))?;
        validate::validate_literal_with_memory(&bound, datatype, language, direction, self.memory)
            .map_err(|error| self.native_error(error))?;
        drop(bound);
        self.memory
            .release_bytes(self.memory.admitted_bytes() - baseline)?;
        Ok(key)
    }

    fn kind(&mut self, key: TermKey) -> Result<Kind, RdfDiagnostic> {
        Ok(match self.term(key)?.kind {
            TermKind::Iri => Kind::Iri,
            TermKind::Bnode => Kind::Blank,
            TermKind::Literal => Kind::Literal,
            TermKind::Triple => Kind::Triple,
        })
    }

    fn term(&mut self, key: TermKey) -> Result<TermLease, RdfDiagnostic> {
        self.dictionary
            .checkpoint(1)
            .map_err(dictionary_import_failure)?;
        self.dictionary
            .get(key)
            .map_err(dictionary_import_failure)?
            .ok_or_else(|| dictionary_import_failure(DictionaryError::Missing(key)))
    }

    fn write_term(&mut self, key: TermKey) -> Result<(), RdfDiagnostic> {
        let mut work = WorkList::<Task, 16>::new();
        work.try_push_admitted(Task::Term(key), self.memory)?;
        let result = (|| {
            while let Some(task) = work.pop() {
                self.memory.admission_mut().checkpoint(1)?;
                if TextOut::failed(&self.output) {
                    break;
                }
                let key = match task {
                    Task::Text(text) => {
                        self.output.push_str(text);
                        continue;
                    }
                    Task::Term(key) => key,
                };
                let term = self.term(key)?;
                match term.kind {
                    TermKind::Iri => term_syntax::write_iri(
                        term.value.as_deref().unwrap_or_default(),
                        &mut self.output,
                    ),
                    TermKind::Bnode => {
                        let baseline = self.memory.admitted_bytes();
                        let mut generated = None;
                        let label = match term.value.as_deref() {
                            Some(label) => label,
                            None => {
                                generated = Some(self.memory.format_with_progress(
                                    &format_args!("gts_bnode_{}_{}", key.segment, key.term),
                                )?);
                                generated.as_deref().unwrap_or_default()
                            }
                        };
                        let encoded = purrdf_core::blank_label::encode_blank_label_with_memory(
                            label,
                            super::segment_scope(key.segment)?,
                            purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel,
                            self.memory,
                        )?;
                        self.output.push_str("_:");
                        self.output.push_str(&encoded);
                        drop(encoded);
                        // Temporary label storage is destroyed before refund.
                        drop(generated);
                        self.memory
                            .release_bytes(self.memory.admitted_bytes() - baseline)?;
                    }
                    TermKind::Literal => {
                        let datatype = term
                            .datatype
                            .map(|id| {
                                self.term(TermKey {
                                    segment: key.segment,
                                    term: id,
                                })
                            })
                            .transpose()?;
                        let datatype = datatype
                            .as_deref()
                            .and_then(|term| term.value.as_deref())
                            .unwrap_or(purrdf_xsd::datatype::XSD_STRING);
                        let direction = crate::gts_resolve::parse_gts_direction_with_memory(
                            term.direction.as_deref(),
                            term.lang.as_deref(),
                            self.memory,
                        )
                        .map_err(|error| self.native_error(error))?;
                        let datatype = crate::RdfLiteral::datatype_iri_from_parts(
                            Some(datatype),
                            term.lang.as_deref(),
                            direction,
                        );
                        let baseline = self.memory.admitted_bytes();
                        let language = term
                            .lang
                            .as_deref()
                            .filter(|tag| !purrdf_iri::langtag::is_identity_folded(tag))
                            .map(|tag| {
                                purrdf_iri::langtag::identity_fold_with_memory(tag, self.memory)
                            })
                            .transpose()?;
                        let lexical = purrdf_core::cdt_blank::bind_cdt_blank_labels_with_memory(
                            term.value.as_deref().unwrap_or_default(),
                            datatype,
                            purrdf_core::cdt_blank::BlankBinding::Ambient(super::segment_scope(
                                key.segment,
                            )?),
                            self.memory,
                        )
                        .map_err(|error| self.composite_error(error))?;
                        term_syntax::write_literal(
                            &lexical,
                            datatype,
                            language.as_deref().or(term.lang.as_deref()),
                            term.direction.as_deref(),
                            &mut self.output,
                        );
                        drop(lexical);
                        drop(language);
                        self.memory
                            .release_bytes(self.memory.admitted_bytes() - baseline)?;
                    }
                    TermKind::Triple => {
                        let triple = match term.triple {
                            Some(triple) => triple,
                            None => self
                                .dictionary
                                .first_binding(TermKey {
                                    segment: key.segment,
                                    term: term.reifier.unwrap_or(key.term),
                                })
                                .map_err(dictionary_import_failure)?
                                .ok_or_else(|| {
                                    self.metadata.err_unbound_triple(key.segment, key.term)
                                })?,
                        };
                        self.output.push_str(term_syntax::TRIPLE_TERM_OPEN);
                        work.try_push_admitted(
                            Task::Text(term_syntax::TRIPLE_TERM_CLOSE),
                            self.memory,
                        )?;
                        work.try_push_admitted(Task::Text(" "), self.memory)?;
                        for id in [triple.2, triple.1, triple.0] {
                            work.try_push_admitted(
                                Task::Term(TermKey {
                                    segment: key.segment,
                                    term: id,
                                }),
                                self.memory,
                            )?;
                            work.try_push_admitted(Task::Text(" "), self.memory)?;
                        }
                    }
                }
            }
            Ok(())
        })();
        let released = work
            .release_admitted(self.memory)
            .map_err(RdfDiagnostic::from);
        result.and(released)
    }

    fn end(&mut self, graph: Option<TermKey>) -> Result<(), RdfDiagnostic> {
        if let Some(graph) = graph {
            self.output.push(' ');
            self.write_term(graph)?;
        }
        self.output.push_str(" .\n");
        Ok(())
    }
}

impl<T: TermStorage, A: Admission + ?Sized> ResolvedSink for Emitter<'_, '_, '_, T, A> {
    type Id = TermKey;
    type Error = RdfDiagnostic;

    fn intern_iri(
        &mut self,
        segment: u64,
        term: usize,
        iri: &str,
    ) -> Result<TermKey, RdfDiagnostic> {
        if iri.is_empty() {
            return Err(self.term_diagnostic(
                "rdf-ir-iri-missing-value",
                "GTS IRI term requires a non-empty value",
                segment,
                term,
            )?);
        }
        validate::validate_iri_with_memory(iri, self.memory)
            .map_err(|error| self.native_error(error))?;
        Ok(TermKey { segment, term })
    }
    fn intern_blank(
        &mut self,
        segment: u64,
        term: usize,
        _label: &str,
    ) -> Result<TermKey, RdfDiagnostic> {
        super::segment_scope(segment)?;
        Ok(TermKey { segment, term })
    }
    fn intern_literal(
        &mut self,
        segment: u64,
        term: usize,
        lexical: String,
        datatype: Option<TermKey>,
        language: Option<String>,
        direction: Option<String>,
    ) -> Result<TermKey, RdfDiagnostic> {
        self.checked_literal(
            TermKey { segment, term },
            &lexical,
            datatype,
            language.as_deref(),
            direction.as_deref(),
        )
    }
    fn intern_stored_literal(
        &mut self,
        segment: u64,
        term: usize,
        literal: TermLease,
        datatype: Option<TermKey>,
    ) -> Result<TermKey, RdfDiagnostic> {
        self.checked_literal(
            TermKey { segment, term },
            literal.value.as_deref().unwrap_or_default(),
            datatype,
            literal.lang.as_deref(),
            literal.direction.as_deref(),
        )
    }
    fn intern_triple(
        &mut self,
        segment: u64,
        term: usize,
        s: TermKey,
        p: TermKey,
        _o: TermKey,
    ) -> Result<TermKey, RdfDiagnostic> {
        validate::validate_triple_positions_with_memory(self.kind(s)?, self.kind(p)?, self.memory)
            .map_err(|error| self.native_error(error))?;
        Ok(TermKey { segment, term })
    }
    fn push_quad(
        &mut self,
        _segment: u64,
        s: TermKey,
        p: TermKey,
        o: TermKey,
        graph: Option<TermKey>,
    ) -> Result<(), RdfDiagnostic> {
        let graph_kind = graph.map(|key| self.kind(key)).transpose()?;
        validate::validate_statement_positions_with_memory(
            self.kind(s)?,
            self.kind(p)?,
            graph_kind,
            self.memory,
        )
        .map_err(|error| self.native_error(error))?;
        for (index, term) in [s, p, o].into_iter().enumerate() {
            if index != 0 {
                self.output.push(' ');
            }
            self.write_term(term)?;
        }
        self.end(graph)
    }
    fn push_reifier(
        &mut self,
        _segment: u64,
        reifier: TermKey,
        s: TermKey,
        p: TermKey,
        o: TermKey,
        graph: Option<TermKey>,
    ) -> Result<(), RdfDiagnostic> {
        validate::validate_triple_positions_with_memory(self.kind(s)?, self.kind(p)?, self.memory)
            .map_err(|error| self.native_error(error))?;
        let graph_kind = graph.map(|key| self.kind(key)).transpose()?;
        validate::validate_statement_positions_with_memory(
            self.kind(reifier)?,
            Kind::Iri,
            graph_kind,
            self.memory,
        )
        .map_err(|error| self.native_error(error))?;
        self.write_term(reifier)?;
        self.output.push(' ');
        term_syntax::write_iri(purrdf_iri::vocab::rdf::REIFIES, &mut self.output);
        self.output.push(' ');
        self.output.push_str(term_syntax::TRIPLE_TERM_OPEN);
        for term in [s, p, o] {
            self.output.push(' ');
            self.write_term(term)?;
        }
        self.output.push(' ');
        self.output.push_str(term_syntax::TRIPLE_TERM_CLOSE);
        self.end(graph)
    }
    fn push_annotation(
        &mut self,
        segment: u64,
        reifier: TermKey,
        predicate: TermKey,
        value: TermKey,
        graph: Option<TermKey>,
    ) -> Result<(), RdfDiagnostic> {
        self.push_quad(segment, reifier, predicate, value, graph)
    }
    fn err_dangling_term(&self, segment: u64, term: usize, role: &str) -> RdfDiagnostic {
        self.metadata.err_dangling_term(segment, term, role)
    }
    fn err_nesting_limit(&self, segment: u64, term: usize) -> RdfDiagnostic {
        self.metadata.err_nesting_limit(segment, term)
    }
    fn err_unbound_triple(&self, segment: u64, term: usize) -> RdfDiagnostic {
        self.metadata.err_unbound_triple(segment, term)
    }
    fn err_missing_reifier(&self, segment: u64, term: usize) -> RdfDiagnostic {
        self.metadata.err_missing_reifier(segment, term)
    }
    fn suppression(&mut self, segment: u64, value: &Suppression) -> Result<(), RdfDiagnostic> {
        self.metadata.suppression(segment, value)
    }
    fn blob(
        &mut self,
        segment: u64,
        digest: &str,
        metadata: Option<&Value>,
    ) -> Result<(), RdfDiagnostic> {
        self.metadata.blob(segment, digest, metadata)
    }
    fn frame(&mut self, frame: FrameContext<'_>) -> Result<(), RdfDiagnostic> {
        self.metadata.frame(frame)
    }
    fn opaque(&mut self, segment: u64, value: &OpaqueNode) -> Result<(), RdfDiagnostic> {
        self.metadata.opaque(segment, value)
    }
    fn signature(&mut self, segment: u64, value: &Signature) -> Result<(), RdfDiagnostic> {
        self.metadata.signature(segment, value)
    }
    fn segment_head(&mut self, segment: u64, head: &[u8]) -> Result<(), RdfDiagnostic> {
        self.metadata.segment_head(segment, head)
    }
    fn streamable_layout(
        &mut self,
        segment: u64,
        value: &StreamableInfo,
    ) -> Result<(), RdfDiagnostic> {
        self.metadata.streamable_layout(segment, value)
    }
    fn diagnostic(&mut self, value: &Diagnostic) -> Result<(), RdfDiagnostic> {
        self.metadata.diagnostic(value)
    }
}

/// Stream the RDF projection to N-Quads using caller-owned dictionary state.
///
/// No resident dataset or whole output document is built. Original container
/// sidecars are returned separately; inline payload bytes are not copied into
/// RDF text. Output remains provisional until this function returns successfully.
///
/// The host supplies dictionary/resolution/replay storage and the output drain.
/// Rendering work-list growth and cancellation use the original `memory` account.
/// Dictionary leases and host buffers retain their own admission accounts.
/// Each segment emits quad rows, then reifier rows, then annotation rows, in
/// their original append order within each table. Segments retain source order.
/// Repeated events remain repeated text statements; no output sorting occurs.
///
/// # Errors
/// Preserves original reader, storage, allocation, cancellation and drain
/// failures. A late failure never reports a complete successful projection.
/// A diagnostic constructed by the exporter retains its charge in `memory`;
/// keep that account alive and release the retained delta after destroying it.
/// Host errors whose diagnostics own independent accounts add no such charge.
pub fn export_gts_rdf_nquads_with_dictionary<D, A: Admission + ?Sized>(
    bytes: &[u8],
    dictionary: &mut dyn TermStorage,
    resolution: &mut D,
    replay: [Box<dyn ReplayStorage>; 3],
    drain: &mut dyn ByteDrain,
    memory: &mut Memory<'_, A>,
) -> Result<GtsTextExport, RdfDiagnostic>
where
    D: ResolutionStorage<TermKey>,
    D::Error: purrdf_hash::error::NativeError + 'static,
{
    let baseline = memory.admitted_bytes();
    let diagnostic_bytes = std::cell::Cell::new(0);
    let result = (|| {
        memory.add_bytes(purrdf_core::sink::DRAIN_BUFFER_BYTES)?;
        let result = (|| {
            let dictionary = std::cell::RefCell::new(dictionary);
            let mut read_dictionary = &dictionary;
            let emitter = Emitter {
                dictionary: &dictionary,
                output: TextSink::to_drain(drain),
                memory: &mut *memory,
                diagnostic_bytes: &diagnostic_bytes,
                metadata: SinkImporter::new(),
            };
            let [quads, reifiers, annotations] = replay;
            let mut resolver = SegmentResolver::with_storage(
                emitter,
                resolution,
                dictionary_import_failure::<D::Error>,
            )
            .with_row_storage(
                quads,
                reifiers,
                annotations,
                dictionary_import_failure::<std::io::Error>,
            )?;
            let read = purrdf_gts::reader::read_to_sink_with_dictionary(
                bytes,
                ReadOptions::new(true, None),
                &mut read_dictionary,
                &mut resolver,
            );
            if let Some(error) = resolver.take_error() {
                return Err(error);
            }
            let read = read.map_err(dictionary_import_failure)?;
            resolver.finish()?;
            let emitter = resolver.into_sink();
            let output = emitter.output.finish().map_err(dictionary_import_failure)?;
            Ok(GtsTextExport {
                written: output.written,
                envelope: crate::RdfEnvelope::new(emitter.metadata.lookaside),
                read,
            })
        })();
        // Every path destroys the bounded TextSink before refunding its window.
        let released = memory
            .release_bytes(purrdf_core::sink::DRAIN_BUFFER_BYTES)
            .map_err(RdfDiagnostic::from);
        match result {
            Err(error) => Err(error),
            Ok(output) => released.map(|()| output),
        }
    })();
    // The construction closure has destroyed every exporter-owned scratch
    // payload. NativeBuildError::Diagnostic alone retains bytes in this account;
    // independently owned host diagnostics are deliberately not counted here.
    let retained = if result.is_err() {
        diagnostic_bytes.get()
    } else {
        0
    };
    let Some(released) = memory
        .admitted_bytes()
        .checked_sub(baseline)
        .and_then(|bytes| bytes.checked_sub(retained))
    else {
        drop(result);
        if let Some(bytes) = memory.admitted_bytes().checked_sub(baseline) {
            let _ = memory.release_bytes(bytes);
        }
        return Err(purrdf_lex::allocation::StorageError::SizeOverflow.into());
    };
    let cleanup = memory.release_bytes(released).map_err(RdfDiagnostic::from);
    match result {
        Err(original) => Err(original),
        Ok(value) => cleanup.map(|()| value),
    }
}

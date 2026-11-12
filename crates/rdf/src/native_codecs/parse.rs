// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native RDF text → frozen [`RdfDataset`] IR ingress.
//!
//! Parses Turtle / TriG / N-Triples / N-Quads through the FIRST-PARTY
//! [`text_parse`](super::text_parse) front-end (RDF/XML through the first-party
//! [`rdfxml`](super::rdfxml) codec) into a first-party in-memory
//! [`SerGraph`](super::ser_model::SerGraph), then walks that graph into a
//! [`RdfDatasetBuilder`] applying the RDF 1.2 statement-layer fold.
//!
//! The fold is factored into [`fold_statement_layer`], a source-agnostic two-pass
//! classifier over `(subject, predicate, object, graph)` rows — one fold, no drift
//! (the must-pass RDF 1.2 fixture parity is the guard).
//!
//! Base IRI is handled per the plan: Turtle/TriG resolve relative IRIs against the
//! supplied base; RDF/XML threads the base through the first-party
//! [`rdfxml`](super::rdfxml) codec's `ParseContext`; N-Triples/N-Quads require
//! absolute IRIs and ignore the base (N/A by syntax).

use std::borrow::Cow;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use super::acquisition::{NativeParseError, RunContext, SourceFailure, TextError, resident_error};
use super::media_type::{NativeRdfFormat, classify};
use super::retained::{ParseRefusal, RetainedFailure, RetainedOutcome, RetainedParse};
use super::ser_model::{SerGraph, SerTermKind};
use super::source_occurrence::{NativeSourceRow, SourceOccurrences, SourceStatementKind};
use super::span::{NoSpans, ParseOptions, SpanCollector, SpanTable};
use super::text_parse::LineParseMode;
use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, TermId};
use purrdf_core::Nested;
use purrdf_core::blank_label::LabelAlphabet;
use purrdf_core::cdt_blank::BlankBinding;
use purrdf_core::ir::try_fold_nested_with_memory;
use purrdf_iri::{BaseIri, BaseOrigin, BaseScope, ScopedBase};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, Resident, StorageError};
use purrdf_lex::control::{ControlledError, Progress, Unmetered};
use std::cell::RefCell;

/// The `rdf:reifies` predicate IRI: a triple-term object under this predicate is the
/// RDF 1.2 reifier binding the statement layer folds out of the base quad table.
pub(crate) use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;

/// A subject/object node presented to [`fold_statement_layer`], already interned into
/// the builder.
#[derive(Clone, Copy)]
pub(crate) enum FoldNode {
    /// An already-interned leaf or (recursively) interned triple-term object.
    Term(TermId),
    /// A triple term whose components are already interned — folded as a reifier
    /// binding when it appears as the object of an `rdf:reifies` row, otherwise
    /// re-interned as a quoted-triple object.
    Triple { s: TermId, p: TermId, o: TermId },
}

/// One `(subject, predicate, object, graph)` row, source-agnostic over flat quads
/// and folded GTS graphs. Every component id is already interned in the SAME builder
/// the fold pushes into; `is_reifies` carries the source-side `rdf:reifies`
/// classification without cloning the predicate IRI into every row.
pub(crate) struct FoldRow {
    pub subject: TermId,
    pub is_reifies: bool,
    pub predicate: TermId,
    pub object: FoldNode,
    pub graph: Option<TermId>,
}

/// The RDF 1.2 statement-layer fold, shared by every row source so they can never
/// drift (the parity fixture is the guard).
///
/// Pass 1 binds reifiers: a row whose predicate is `rdf:reifies` with a triple-term
/// object becomes a `push_reifier_in_graph(subject, triple, graph)` binding and the
/// `(graph, subject)` pair is recorded as a reifier. Pass 2 classifies the remaining
/// rows: a triple whose subject is a reifier declared in the SAME graph is an
/// annotation (`push_annotation_in_graph`), everything else a base quad (`push_quad`).
///
/// The reifier key carries the graph because the RDF 1.2 statement layer is per-graph:
/// a reifier declared in `GRAPH g1` is not a reifier in `g2`, so a triple about that
/// same subject in `g2` is an ordinary triple of `g2` and nothing more. Keying on the
/// subject alone would let a reifier declared anywhere in the document silently
/// reclassify every other graph's triples about it — and would put this reader at odds
/// with the `CONSTRUCT` template evaluator, which keys on `(graph, reifier)`, so a
/// constructed dataset and its own serialization would no longer read back the same.
/// With a single-graph document the graph half is constant and the key degenerates to
/// the subject, which is exactly the pre-per-graph behaviour.
pub(crate) fn fold_statement_layer<I>(
    builder: &mut RdfDatasetBuilder,
    rows: I,
) -> Result<(), RdfDiagnostic>
where
    I: IntoIterator<Item = FoldRow>,
    I::IntoIter: ExactSizeIterator,
{
    // Other codecs retain their explicit resident wrapper around the same two
    // passes. Their input arrays belong to those resident callers.
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let memory = RefCell::new(&mut memory);
    fold_statement_layer_observed(
        builder,
        rows.into_iter().map(|row| (row, ())),
        &memory,
        |(), _, _| Ok(()),
    )
    .map_err(|cause| resident_error(ControlledError::Native(cause)))
}

/// The same two native passes, retaining an opaque caller association until the
/// original row's object has been materialized and its statement layer decided.
/// Membership, pending rows and builder insertions share the original account.
fn fold_statement_layer_observed<I, A, S: Admission + ?Sized>(
    builder: &mut RdfDatasetBuilder,
    rows: I,
    memory: &RefCell<&mut Memory<'_, S>>,
    mut observe: impl FnMut(A, SourceStatementKind, [Option<TermId>; 4]) -> Result<(), NativeParseError>,
) -> Result<(), NativeParseError>
where
    I: IntoIterator<Item = (FoldRow, A)>,
    I::IntoIter: ExactSizeIterator,
{
    let rows = rows.into_iter();
    let maximum_pending = rows.len();
    let mut reifier_ids = purrdf_core::hash::table::HashTable::new();
    let mut pending = Vec::new();
    let result: Result<(), NativeParseError> = (|| {
        // Pass 1: bind reifiers; collect the rest as pending base/annotation rows.
        for (row, association) in rows {
            memory.borrow_mut().admission_mut().checkpoint(1)?;
            let FoldRow {
                subject,
                is_reifies,
                predicate,
                object,
                graph,
            } = row;
            if is_reifies && let FoldNode::Triple { s, p, o } = object {
                let triple_term =
                    builder.intern_triple_with_memory(s, p, o, &mut memory.borrow_mut())?;
                builder.push_reifier_in_graph_with_memory(
                    subject,
                    triple_term,
                    graph,
                    &mut memory.borrow_mut(),
                )?;
                {
                    let mut account = memory.borrow_mut();
                    let account = &mut **account;
                    let key = (graph, subject);
                    account.admission_mut().checkpoint(1)?;
                    let hash = purrdf_hash::fixed::hash_one(&key);
                    if reifier_ids
                        .find_with_control(
                            hash,
                            |stored, account| {
                                account.admission_mut().checkpoint(1)?;
                                Ok(*stored == key)
                            },
                            account,
                        )?
                        .is_none()
                    {
                        let required = reifier_ids
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        purrdf_core::hash::try_reserve_table_with_memory(
                            &mut reifier_ids,
                            required,
                            |key, account| {
                                account.admission_mut().checkpoint(1)?;
                                Ok(purrdf_hash::fixed::hash_one(key))
                            },
                            account,
                        )?;
                        reifier_ids
                            .try_insert_unique_with_control(
                                hash,
                                key,
                                |key, account| {
                                    account.admission_mut().checkpoint(1)?;
                                    Ok(purrdf_hash::fixed::hash_one(key))
                                },
                                account,
                            )
                            .map_err(|refusal| {
                                purrdf_core::hash::table_storage_error(&refusal.cause)
                            })?;
                    }
                }
                observe(
                    association,
                    SourceStatementKind::Reifier,
                    [Some(subject), Some(predicate), Some(triple_term), graph],
                )?;
                continue;
            }
            let object_id = match object {
                FoldNode::Term(id) => id,
                FoldNode::Triple { s, p, o } => {
                    builder.intern_triple_with_memory(s, p, o, &mut memory.borrow_mut())?
                }
            };
            // The native staged input has an exact length. Reserve its bound
            // once, only when a pending row exists, so opaque associations move
            // once without an unbounded relocation or a second classification.
            if pending.capacity() == 0 {
                memory.borrow_mut().reserve(&mut pending, maximum_pending)?;
            }
            memory.borrow_mut().push(
                &mut pending,
                (subject, predicate, object_id, graph, association),
            )?;
        }

        // Pass 2: graph-local annotations and ordinary quads keep their original
        // pending order. Drain retains the backing owner through every refusal.
        #[expect(
            clippy::iter_with_drain,
            reason = "The original admitted vector backing remains live until release_vec refunds its allocation."
        )]
        for (subject, predicate, object, graph, association) in pending.drain(..) {
            memory.borrow_mut().admission_mut().checkpoint(1)?;
            let is_annotation = {
                let mut account = memory.borrow_mut();
                let account = &mut **account;
                let key = (graph, subject);
                account.admission_mut().checkpoint(1)?;
                let hash = purrdf_hash::fixed::hash_one(&key);
                reifier_ids
                    .find_with_control(
                        hash,
                        |stored, account| {
                            account.admission_mut().checkpoint(1)?;
                            Ok(*stored == key)
                        },
                        account,
                    )?
                    .is_some()
            };
            let kind = if is_annotation {
                builder.push_annotation_in_graph_with_memory(
                    subject,
                    predicate,
                    object,
                    graph,
                    &mut memory.borrow_mut(),
                )?;
                SourceStatementKind::Annotation
            } else {
                builder.push_quad_with_memory(
                    subject,
                    predicate,
                    object,
                    graph,
                    &mut memory.borrow_mut(),
                )?;
                SourceStatementKind::Quad
            };
            observe(
                association,
                kind,
                [Some(subject), Some(predicate), Some(object), graph],
            )?;
        }
        Ok(())
    })();
    // The input/drain and partial pending elements have died before refund.
    // Never refund a whole operation delta: observers may retain source data.
    let cleanup = memory
        .borrow_mut()
        .release_vec(pending)
        .map_err(NativeParseError::Storage);
    let reifier_bytes = purrdf_core::hash::hash_table_retained_bound(&reifier_ids)
        .ok_or(StorageError::SizeOverflow)?;
    drop(reifier_ids);
    let membership_cleanup = memory
        .borrow_mut()
        .release_bytes(reifier_bytes)
        .map_err(NativeParseError::Storage);
    result.and(cleanup).and(membership_cleanup)
}

/// Parse RDF text bytes of `media_type` into a frozen [`RdfDataset`].
///
/// Steps: UTF-8 validate (hard-fail `native-codec-utf8`); parse the line/Turtle family
/// (N-Triples, N-Quads, Turtle, TriG) FIRST-PARTY into an in-memory `SerGraph` via
/// `text_parse`, and RDF/XML through the FIRST-PARTY
/// `rdfxml` codec (no external purrdf-gts text / RDF-XML codec); then
/// walk that graph through `fold_statement_layer` and freeze.
pub fn parse_dataset(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::Auto)
        .map(|(dataset, _base)| dataset)
}

/// Everything one parse of one document produced.
///
/// The ingress mirror of [`SerializeOutcome`](super::SerializeOutcome): a parse yields
/// more than a dataset, and the extras arrive together in ONE value rather than as a
/// growing tuple or a family of `parse_dataset_*` twins.
#[derive(Debug, Clone)]
pub struct ParseOutcome {
    /// The frozen dataset. IDENTICAL to what [`parse_dataset`] returns for the same
    /// input — same triples, same term ids, same order.
    pub dataset: Arc<RdfDataset>,
    /// The opt-in source-position side table (see
    /// [`ParseOptions::track_source_spans`]), or `None` when tracking was off.
    pub spans: Option<SpanTable>,
    /// The base IRI in force at the END of the document, with the
    /// [`BaseOrigin`] provenance of how it came to be in force —
    /// [`BaseOrigin::Caller`] when only the caller supplied one,
    /// [`BaseOrigin::Directive`] for a Turtle/TriG `@base`, [`BaseOrigin::Enclosing`]
    /// for a root `xml:base` or a JSON-LD `@context` `@base`.
    ///
    /// `None` means the document truly ended under NO base — either none was ever
    /// supplied or declared, or a JSON-LD `"@base": null` cleared the caller's.
    ///
    /// This closes the ingress/egress asymmetry: a caller could hand a base IN and
    /// (since the serializer base leg) hand one OUT, but could not ask what base the
    /// DOCUMENT ended up under, so a round trip preserving a document's own `@base`
    /// could not be written without re-reading the text by hand.
    ///
    /// [`BaseOrigin`]: purrdf_iri::BaseOrigin
    /// [`BaseOrigin::Caller`]: purrdf_iri::BaseOrigin::Caller
    /// [`BaseOrigin::Directive`]: purrdf_iri::BaseOrigin::Directive
    /// [`BaseOrigin::Enclosing`]: purrdf_iri::BaseOrigin::Enclosing
    pub document_base: Option<ScopedBase>,
    /// The prefix bindings a Turtle or TriG document's `@prefix` / `PREFIX` directives
    /// left in force at the END of the document: one `(label, namespace)` pair per
    /// declared label, sorted by label, each namespace resolved against the base that
    /// was in force when it was declared. A label declared twice reports the namespace
    /// of its LAST declaration.
    ///
    /// This is the grammar's own record, read back from the parse that decided what
    /// every prefixed name in the document meant — not a second reading of the text, so
    /// characters that merely look like a directive inside a string literal are never
    /// reported. The parser's built-in `rdf:` convenience binding is not a declaration
    /// and is not reported.
    ///
    /// For RDF/XML it is the document's `xmlns` declarations in the same shape — last
    /// declaration of each prefix wins, sorted, the default namespace under the empty
    /// prefix `""` — as the XML parser scoped them, never those inside the content of
    /// an `rdf:parseType="Literal"` value (see the RDF/XML codec's `declared_namespaces`).
    /// EMPTY for every other format: N-Triples and N-Quads have no prefix directive, and
    /// TriX, HexTuples, JSON-LD and YAML-LD declare none this reports.
    pub document_prefixes: Vec<(String, String)>,
}

impl ParseOutcome {
    /// The document base as the `Option<&str>` the serialize leg takes, discarding the
    /// provenance. The join between the two legs of a base-preserving round trip.
    #[must_use]
    pub fn document_base_iri(&self) -> Option<&str> {
        self.document_base.as_ref().map(|base| base.iri().as_str())
    }
}

/// A parse that failed, with the directive state in force where it stopped.
///
/// The failure mirror of [`ParseOutcome`]'s `document_base` and `document_prefixes`:
/// for Turtle and TriG, every `@prefix` / `PREFIX` and `@base` / `BASE` directive the
/// parser read BEFORE the failing token, and nothing after it. A caller that recovers
/// statement by statement — re-parsing each remaining statement to report every
/// independent defect — continues under exactly the directives the document made up to
/// that point, taken from the grammar rather than from a second reading of the text.
///
/// For every other format, and for a failure before parsing began (an unknown media
/// type, invalid UTF-8, an invalid caller base), the state is what the caller supplied:
/// no prefixes, and the caller's base when it was valid.
#[derive(Debug, Clone)]
pub struct ParseFailure {
    /// The diagnostic [`parse_dataset_with`] returns for the same input.
    pub diagnostic: RdfDiagnostic,
    /// The base in force where the parse stopped (see [`ParseOutcome::document_base`]).
    pub document_base: Option<ScopedBase>,
    /// The prefix bindings in force where the parse stopped, in the form
    /// [`ParseOutcome::document_prefixes`] reports them.
    pub document_prefixes: Vec<(String, String)>,
}

impl ParseFailure {
    /// The base in force where the parse stopped, as the `Option<&str>` a parse takes.
    #[must_use]
    pub fn document_base_iri(&self) -> Option<&str> {
        self.document_base.as_ref().map(|base| base.iri().as_str())
    }

    /// A failure before any directive was read, under the caller's `base_iri`.
    fn before_parsing(diagnostic: RdfDiagnostic, base_iri: Option<&str>) -> Box<Self> {
        Box::new(Self {
            diagnostic,
            document_base: base_scope_for(base_iri)
                .ok()
                .and_then(|scope| scope.current().cloned()),
            document_prefixes: Vec::new(),
        })
    }
}

/// [`parse_dataset`] reporting everything the parse learned: the dataset, the opt-in
/// source-position side table, and the base the document ended up under.
///
/// This is the ONE extended parse seam — deliberately not a `parse_dataset_with_base`
/// twin beside a `parse_dataset_with_spans` one. What a parse can tell you grows; the
/// number of ways to ask must not. [`parse_dataset_reporting_failure`] is this same
/// seam with the directive state kept on the error path too; this function is it with
/// that state dropped.
///
/// Span tracking is OPT-IN: it costs memory and pins the sequential line pipeline
/// (the chunk-parallel N-Triples/N-Quads path never records spans), so
/// [`parse_dataset`] — the hot path — is left untouched. Formats without a text
/// tokenizer that carries source spans (RDF/XML, TriX, HexTuples) return an EMPTY
/// [`SpanTable`] under tracking (physical-location fallback is by design). The document
/// base is reported unconditionally: every codec already tracks its own base scope, so
/// reading it back costs nothing.
pub fn parse_dataset_with(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
    options: &ParseOptions<'_>,
) -> Result<ParseOutcome, RdfDiagnostic> {
    parse_dataset_reporting_failure(bytes, media_type, base_iri, options)
        .map_err(|failure| failure.diagnostic)
}

/// [`parse_dataset_with`], keeping the directive state on the error path: a failure is
/// a (boxed, since it is the rare path) [`ParseFailure`] carrying the prefixes and base
/// in force where the parse stopped.
///
/// # Errors
///
/// Exactly [`parse_dataset_with`]'s diagnostic, as [`ParseFailure::diagnostic`].
pub fn parse_dataset_reporting_failure(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
    options: &ParseOptions<'_>,
) -> Result<ParseOutcome, Box<ParseFailure>> {
    let format = classify(media_type).map_err(|e| ParseFailure::before_parsing(e, base_iri))?;

    if options.yaml_ld.is_some() || options.yaml_ld_context.is_some() {
        if format != NativeRdfFormat::YamlLd {
            return Err(ParseFailure::before_parsing(
                RdfDiagnostic::error(
                    "yamlld-input-options-format",
                    "YAML-LD input options require YAML-LD input",
                ),
                base_iri,
            ));
        }
        let mut base =
            base_scope_for(base_iri).map_err(|error| ParseFailure::before_parsing(error, None))?;
        let yaml = options.yaml_ld.unwrap_or_default();
        let dataset =
            super::jsonld::yaml::parse_into_scope(bytes, &mut base, &yaml, options.yaml_ld_context)
                .map_err(|error| ParseFailure::before_parsing(error, base_iri))?;
        return Ok(ParseOutcome {
            dataset,
            spans: options.track_source_spans.then(SpanTable::default),
            document_base: base.current().cloned(),
            document_prefixes: Vec::new(),
        });
    }

    if format.tokenizer_carries_spans() {
        // Line/Turtle family: parsed here, through the SAME `text_parse` front-end and
        // fold `LineCodec::parse` drives, rather than through the codec seam — because
        // this is the one leg that also reads back the document's prefix map, which the
        // codec seam (shared with formats that have no `@prefix`) does not carry. UTF-8
        // is only required by the text tokenizer, so it is validated here.
        parse_text_retained(bytes, format, base_iri, *options)
            .map_err(|failure| {
                ParseFailure::before_parsing(failure.into_resident_error(), base_iri)
            })?
            .into_legacy()
    } else if format == NativeRdfFormat::RdfXml {
        // RDF/XML: no span-carrying tokenizer (an empty table under tracking), and its
        // prefix map is the document's `xmlns` declarations, read back from the XML
        // parser by the same parse — so it is parsed here rather than through the codec
        // seam, which carries no namespaces.
        let text = std::str::from_utf8(bytes).map_err(|e| {
            ParseFailure::before_parsing(super::stream::utf8_error(&e, 0), base_iri)
        })?;
        let mut base =
            base_scope_for(base_iri).map_err(|e| ParseFailure::before_parsing(e, None))?;
        let mut document_prefixes = Vec::new();
        let dataset = parse_rdfxml_without_panicking(text, &mut base, Some(&mut document_prefixes))
            .map_err(|e| ParseFailure::before_parsing(e, base_iri))?;
        Ok(ParseOutcome {
            dataset,
            spans: options.track_source_spans.then(SpanTable::default),
            document_base: base.current().cloned(),
            document_prefixes,
        })
    } else {
        // TriX, HexTuples, JSON-LD, YAML-LD: no span-carrying text tokenizer, so under
        // tracking return an empty table alongside the identical dataset
        // (physical-location fallback by design). None has a prefix directive.
        let (dataset, base) = parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::Auto)
            .map_err(|e| ParseFailure::before_parsing(e, base_iri))?;
        Ok(ParseOutcome {
            dataset,
            spans: options.track_source_spans.then(SpanTable::default),
            document_base: base.current().cloned(),
            document_prefixes: Vec::new(),
        })
    }
}

/// [`parse_dataset`] forcing the single-threaded N-Triples/N-Quads line pipeline
/// regardless of input size (Turtle/TriG/RDF-XML are always sequential, so the mode
/// is a no-op there).
///
/// Bench/test-only surface: the `native_codecs` bench and the determinism-proof tests use
/// it as the baseline the chunk-parallel path must match byte-for-byte. NOT public
/// API — hidden, unstable, and free to disappear.
#[doc(hidden)]
pub fn parse_dataset_forced_sequential(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::ForceSequential)
        .map(|(dataset, _base)| dataset)
}

/// Retain the actual resident byte-parser result and source under its one
/// context. This helper is private and makes no governed-lowering claim.
fn parse_text_retained<'a>(
    bytes: &'a [u8],
    format: NativeRdfFormat,
    base_iri: Option<&str>,
    options: ParseOptions<'_>,
) -> Result<RetainedParse<'a, Resident, Unmetered>, SourceFailure<'a, Resident, Unmetered>> {
    if matches!(format, NativeRdfFormat::Turtle | NativeRdfFormat::TriG) {
        return parse_turtle_retained(
            bytes,
            format,
            base_iri,
            options,
            RunContext::new(Resident, Unmetered),
        );
    }
    let mut source = RunContext::new(Resident, Unmetered).source(bytes)?;
    let mut base = match base_scope_for(base_iri) {
        Ok(base) => base,
        Err(cause) => {
            return Ok(RetainedParse::new(
                source,
                Err(RetainedFailure::new(
                    NativeParseError::Diagnostic(cause).into(),
                    BaseScope::empty(),
                    OrderedTable::default(),
                )),
            ));
        }
    };
    let mut document_prefixes = Vec::new();
    let parsed = source.with_text(|text, context| {
        if options.track_source_spans {
            // Force sequential so subject spans are captured (the parallel path is
            // `NoSpans`-only). This is a distinct `SpanTable` monomorphization of
            // `text_parse_without_panicking`, so the hot `NoSpans` path stays zero-cost.
            let mut table = SpanTable::default();
            text_parse_without_panicking(
                format,
                text,
                &mut base,
                LineParseMode::ForceSequential,
                &mut table,
                &mut document_prefixes,
                context,
            )
            .map(|graph| (graph, Some(table)))
        } else {
            text_parse_without_panicking(
                format,
                text,
                &mut base,
                LineParseMode::Auto,
                &mut NoSpans,
                &mut document_prefixes,
                context,
            )
            .map(|graph| (graph, None))
        }
    });
    let folded = source.with_text(|_, context| {
        parsed.and_then(|(graph, mut spans)| {
            let associations = spans.as_mut().map(SpanTable::source_occurrences_mut);
            let dataset = dataset_from_text_ser_graph_in_context(&graph, associations, context);
            // Original lowering storage dies before its admission is refunded.
            let cleanup = context
                .release(|memory| super::text_parse::release_lowered_graph_rows(graph, memory));
            finish_dataset_in_context(dataset, cleanup, context)
                .map(|dataset| (dataset, spans))
                .map_err(resident_error)
        })
    });
    let output = match folded {
        Ok((dataset, spans)) => Ok(RetainedOutcome {
            dataset,
            spans,
            document_base: base,
            document_prefixes: OrderedTable::default(),
        }),
        Err(diagnostic) => Err(RetainedFailure::new(
            NativeParseError::Diagnostic(diagnostic).into(),
            base,
            OrderedTable::default(),
        )),
    };
    Ok(RetainedParse::new(source, output))
}

/// Parse Turtle 1.2 or TriG with caller-owned storage, work and stop authority.
///
/// Exact ordered source occurrences are always retained. The returned owner
/// exposes borrowed source, directives and dataset views; no concrete dataset
/// or acquisition account can be detached or cloned. Query workspace admission
/// requires a separately owned query session.
///
/// # Errors
/// Every unsupported format, native parse failure, physical storage refusal or
/// caller progress failure returns a [`ParseRefusal`] with the full original
/// input and unchanged typed cause. Refusal exposes no partial dataset.
#[expect(
    clippy::result_large_err,
    reason = "Refusal returns the original parser payloads and admission owner inline without allocating after refusal."
)]
pub fn parse_dataset_controlled<'a, A: Admission, C: Progress>(
    bytes: &'a [u8],
    format: NativeRdfFormat,
    base_iri: Option<&str>,
    admission: A,
    progress: C,
) -> Result<RetainedParse<'a, A, C>, ParseRefusal<'a, A, C>> {
    let owner = parse_turtle_retained(
        bytes,
        format,
        base_iri,
        ParseOptions {
            track_source_spans: true,
            yaml_ld: None,
            yaml_ld_context: None,
        },
        RunContext::new(admission, progress),
    )
    .map_err(ParseRefusal::from_source)?;
    ParseRefusal::classify(owner)
}

/// Retain the native Turtle/TriG pipeline under its original source account.
fn parse_turtle_retained<'a, A: Admission, C: Progress>(
    bytes: &'a [u8],
    format: NativeRdfFormat,
    base_iri: Option<&str>,
    options: ParseOptions<'_>,
    context: RunContext<A, C>,
) -> Result<RetainedParse<'a, A, C>, SourceFailure<'a, A, C>> {
    if !matches!(format, NativeRdfFormat::Turtle | NativeRdfFormat::TriG) {
        return Err(SourceFailure::unsupported(bytes, format, context));
    }
    let mut source = context.source(bytes)?;
    let mut base = BaseScope::empty();
    let mut document_prefixes = OrderedTable::default();
    let mut spans = options.track_source_spans.then(SpanTable::default);
    let parsed = source.with_text(|text, context| -> Result<SerGraph, TextError<C::Error>> {
        context.charge(0)?;
        if let Some(iri) = base_iri {
            context.native(|memory| {
                base.rebind_with_memory(iri, BaseOrigin::Caller, memory)
                    .map_err(|cause| match cause {
                        purrdf_iri::IriReadError::Storage(cause) => {
                            NativeParseError::Storage(cause)
                        }
                        purrdf_iri::IriReadError::Lexical(cause) => {
                            NativeParseError::Base { cause }
                        }
                    })
            })?;
        }
        let parsed = catch_unwind(AssertUnwindSafe(|| {
            if let Some(table) = spans.as_mut() {
                super::text_parse::document_graph_in_context(
                    text,
                    &mut base,
                    format == NativeRdfFormat::TriG,
                    table,
                    &mut document_prefixes,
                    context,
                )
            } else {
                super::text_parse::document_graph_in_context(
                    text,
                    &mut base,
                    format == NativeRdfFormat::TriG,
                    &mut NoSpans,
                    &mut document_prefixes,
                    context,
                )
            }
        }));
        match parsed {
            Ok(parsed) => parsed,
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("non-string panic payload");
                Err(context.diagnostic(|memory| {
                    RdfDiagnostic::try_error_with_memory(
                        "native-codec-panic",
                        &format_args!(
                            "native RDF text parser panicked while parsing {}: {detail}",
                            format.media_type()
                        ),
                        memory,
                    )
                }))
            }
        }
    });
    let folded = source.with_text(|_, context| {
        parsed.and_then(|graph| {
            let associations = spans.as_mut().map(SpanTable::source_occurrences_mut);
            let dataset = dataset_from_text_ser_graph_in_context(&graph, associations, context);
            let cleanup = context
                .release(|memory| super::text_parse::release_lowered_graph_rows(graph, memory));
            finish_dataset_in_context(dataset, cleanup, context)
        })
    });
    let output = match folded {
        Ok(dataset) => Ok(RetainedOutcome {
            dataset,
            spans,
            document_base: base,
            document_prefixes,
        }),
        Err(cause) => Err(RetainedFailure::new(cause, base, document_prefixes)),
    };
    Ok(RetainedParse::new(source, output))
}

/// Finish original lowering cleanup and poll before publishing its dataset.
/// A failed completion destroys the dataset before refunding its exact original
/// grant, leaving caller input, source associations and the first cause intact.
pub(super) fn finish_dataset_in_context<A: Admission, C: Progress>(
    result: Result<(RdfDataset, usize), TextError<C::Error>>,
    cleanup: Result<(), TextError<C::Error>>,
    context: &mut RunContext<A, C>,
) -> Result<RdfDataset, TextError<C::Error>> {
    let (dataset, bytes) = result?;
    match cleanup.and_then(|()| context.charge(0)) {
        Ok(()) => Ok(dataset),
        Err(cause) => {
            drop(dataset);
            let _ = context.release(|memory| memory.release_bytes(bytes));
            Err(cause)
        }
    }
}

/// Build the in-scope base stack for a caller-supplied base string.
///
/// This is the ONE boundary where the public `Option<&str>` base becomes the typed
/// [`BaseScope`] the codecs consume, so the "is this base absolute?" check happens once
/// here rather than inside each codec. A caller-supplied base that is not absolute is a
/// hard `iri-non-absolute-base` — the library never derives a base of its own (it is
/// handed BYTES and has no retrieval IRI; see RFC-3986 §5.1.4).
pub(super) fn base_scope_for(base_iri: Option<&str>) -> Result<BaseScope, RdfDiagnostic> {
    match base_iri {
        None => Ok(BaseScope::empty()),
        Some(raw) => {
            let base = BaseIri::parse(raw).map_err(|e| {
                RdfDiagnostic::error(e.diagnostic_code(), format!("invalid base IRI: {e}"))
            })?;
            Ok(BaseScope::rooted(base, BaseOrigin::Caller))
        }
    }
}

/// Parse and hand back BOTH the frozen dataset and the base scope the document left in
/// force — the two halves every caller of this function needs between them, so neither is
/// recomputed and neither can be reported from a different parse than the other.
fn parse_dataset_mode(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
    mode: LineParseMode,
) -> Result<(Arc<RdfDataset>, BaseScope), RdfDiagnostic> {
    let format = classify(media_type)?;
    let mut base = base_scope_for(base_iri)?;
    if format == NativeRdfFormat::YamlLd {
        let dataset = catch_codec_panic(format, || {
            super::jsonld::yaml::parse_into_scope(
                bytes,
                &mut base,
                &super::jsonld::YamlLdOptions::DEFAULT,
                None,
            )
        })?;
        return Ok((dataset, base));
    }
    let text = std::str::from_utf8(bytes).map_err(|e| super::stream::utf8_error(&e, 0))?;

    // Dispatch to the format's codec (the single `codec_for` chokepoint). Each codec is
    // FIRST-PARTY and panic-guarded: the line/Turtle family parses into the in-memory
    // SerGraph the statement-layer fold consumes (N-Triples/N-Quads above the
    // `text_parse` size threshold tokenize line-aligned chunks in PARALLEL and re-join
    // in document order, so the frozen IR is byte-identical to the sequential pipeline;
    // Turtle/TriG stay sequential for stateful `@prefix`/`@base` + document-ordered
    // bnode minting — see `text_parse::parse_to_gts_graph_mode`); RDF/XML, TriX and
    // HexTuples intern straight into the frozen IR through the SAME shared fold, no
    // intermediate GTS graph. `mode` is honored only by the line/Turtle family.
    //
    // The codec leaves `base` holding whatever the DOCUMENT left in force: a Turtle
    // `@base`, a root `xml:base`, a JSON-LD `@context` `@base` — or, for a grammar with no
    // base directive, the caller's base untouched.
    let dataset = super::codec::codec_for(format).parse(text, &mut base, mode)?;
    Ok((dataset, base))
}

/// Run a first-party codec parse under a panic guard, converting any unwind into a
/// structured `native-codec-panic` diagnostic (mirrors the RDF/XML guard).
pub(super) fn catch_codec_panic<T, F>(format: NativeRdfFormat, parse: F) -> Result<T, RdfDiagnostic>
where
    F: FnOnce() -> Result<T, RdfDiagnostic>,
{
    match catch_unwind(AssertUnwindSafe(parse)) {
        Ok(result) => result,
        Err(payload) => Err(RdfDiagnostic::error(
            "native-codec-panic",
            format!(
                "native codec panicked while parsing {}: {}",
                format.media_type(),
                panic_payload_message(payload.as_ref()),
            ),
        )),
    }
}

pub(super) fn text_parse_without_panicking<S: SpanCollector>(
    format: NativeRdfFormat,
    text: &str,
    base: &mut BaseScope,
    mode: LineParseMode,
    collector: &mut S,
    prefixes: &mut Vec<(String, String)>,
    context: &mut RunContext<Resident, Unmetered>,
) -> Result<SerGraph, RdfDiagnostic> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::text_parse::parse_to_gts_graph_mode(
            format, text, base, mode, collector, prefixes, context,
        )
    }));
    match outcome {
        Ok(result) => result,
        Err(payload) => Err(RdfDiagnostic::error(
            "native-codec-panic",
            format!(
                "native RDF text parser panicked while parsing {}: {}",
                format.media_type(),
                panic_payload_message(payload.as_ref()),
            ),
        )),
    }
}

pub(super) fn parse_rdfxml_without_panicking(
    text: &str,
    base: &mut BaseScope,
    namespaces: Option<&mut Vec<(String, String)>>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::rdfxml::parse_rdfxml_document(text, base, namespaces)
    }));
    match outcome {
        Ok(result) => result,
        Err(payload) => Err(RdfDiagnostic::error(
            "native-codec-panic",
            format!(
                "native RDF/XML codec panicked while parsing {}: {}",
                NativeRdfFormat::RdfXml.media_type(),
                panic_payload_message(payload.as_ref()),
            ),
        )),
    }
}

pub(super) fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

/// Walk a first-party [`SerGraph`] into a frozen [`RdfDataset`] through the shared
/// [`fold_statement_layer`].
///
/// The first-party parse path folds `rdf:reifies` triples into the graph's `reifiers`
/// table (they never appear in `quads`) AND classifies a reifier's sibling triples into
/// the `annotations` table — the parser owns reifier identity, including anonymous `[]`
/// reifiers. To feed the SAME two-pass fold flat quads use — and reach the SAME
/// IR — this re-materializes each reifier binding as a synthetic
/// `<reifier> rdf:reifies <<( s p o )>>` row and each annotation as a
/// `<reifier> <predicate> <value>` row alongside the plain quads, so pass 1 re-binds
/// reifiers and pass 2 classifies the reifier subjects' rows as annotations. Term
/// interning is shared across all rows, so identical terms collapse to one id exactly as
/// on the flat-quad path.
pub(crate) fn dataset_from_ser_graph(graph: &SerGraph) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    dataset_from_ser_graph_impl(graph, false, BlankIngress::Opaque, None)
}

/// [`dataset_from_ser_graph`] for a graph a TEXT codec just parsed: every blank
/// label is decoded through [`RdfDatasetBuilder::intern_text_blank`], inverting
/// the `(label, scope)` encoding the serializers applied at egress.
///
/// Only the line/Turtle family reaches this entry point, and all four of its
/// syntaxes write `_:` tokens, so the decode's image test runs against
/// [`LabelAlphabet::BlankNodeLabel`] — exactly the alphabet those serializers
/// encode into. (RDF/XML, TriX and HexTuples intern their tokens directly, each
/// naming its own alphabet.)
///
/// The carrier entry point above stays [`BlankIngress::Opaque`] on purpose: a GTS
/// bundle stores the IR's `(label, scope)` pair structurally, so its labels were
/// never transformed and decoding them would corrupt a label that merely LOOKS
/// like an encoder's output.
pub(crate) fn dataset_from_text_ser_graph(
    graph: &SerGraph,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    dataset_from_ser_graph_impl(
        graph,
        false,
        BlankIngress::TextDecoded(LabelAlphabet::BlankNodeLabel),
        None,
    )
}

pub(super) fn dataset_from_text_ser_graph_in_context<A: Admission, C: Progress>(
    graph: &SerGraph,
    source: Option<&mut SourceOccurrences>,
    context: &mut RunContext<A, C>,
) -> Result<(RdfDataset, usize), TextError<C::Error>> {
    dataset_from_ser_graph_in_context(
        graph,
        false,
        BlankIngress::TextDecoded(LabelAlphabet::BlankNodeLabel),
        source,
        context,
    )
}

/// Like [`dataset_from_ser_graph`], but folds **every** named graph into the default
/// graph (drops each base quad's graph component). This is the load path the native
/// conformance gate replays against the frozen goldens (which were captured over a
/// flattened dataset). The statement layer (`rdf:reifies`
/// reifiers + annotations) is flattened with everything else, so a reifier and its
/// annotations all land at `graph == None` and the fold's per-graph reifier key
/// degenerates to the reifier id.
pub(crate) fn flattened_dataset_from_ser_graph(
    graph: &SerGraph,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    dataset_from_ser_graph_impl(graph, true, BlankIngress::Opaque, None)
}

/// How a [`SerGraph`]'s blank-node labels reached this fold, and therefore whether
/// they carry the serializers' egress transform.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlankIngress {
    /// The labels are a carrier's stored `(label, scope)` labels, never
    /// transformed — intern them verbatim.
    Opaque,
    /// The labels are a text document's tokens, encoded at egress into the
    /// wrapped alphabet — decode them back to `(label, scope)` against that
    /// same alphabet, so ingress inverts exactly the egress that syntax
    /// performs.
    TextDecoded(LabelAlphabet),
}

impl BlankIngress {
    /// The same rule, stated for the blank labels a composite (`cdt:List` /
    /// `cdt:Map`) literal embeds in its lexical form.
    ///
    /// Bare terms and embedded labels MUST bind identically or the two spellings
    /// of one node would fall apart, so this is a projection of the mode rather
    /// than a second decision.
    fn binding(self) -> BlankBinding {
        match self {
            Self::Opaque => BlankBinding::Ambient(BlankScope::DEFAULT),
            Self::TextDecoded(alphabet) => BlankBinding::Decoded(alphabet),
        }
    }
}

fn dataset_from_ser_graph_impl(
    graph: &SerGraph,
    flatten_to_default_graph: bool,
    blanks: BlankIngress,
    source: Option<&mut SourceOccurrences>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    // Carrier and other codec callers keep their explicit resident adapter.
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    dataset_from_ser_graph_with_memory(graph, flatten_to_default_graph, blanks, source, &mut memory)
        .map(|(dataset, _)| Arc::new(dataset))
        .map_err(|cause| resident_error(ControlledError::Native(cause)))
}

/// Stage the governed dataset and its exact original replacement grant outside
/// the callback. A refused final poll destroys and refunds only that dataset;
/// persistent source associations keep their original account.
fn dataset_from_ser_graph_in_context<A: Admission, C: Progress>(
    graph: &SerGraph,
    flatten_to_default_graph: bool,
    blanks: BlankIngress,
    source: Option<&mut SourceOccurrences>,
    context: &mut RunContext<A, C>,
) -> Result<(RdfDataset, usize), TextError<C::Error>> {
    let mut staged = None;
    let result = context.native(|memory| {
        staged = Some(dataset_from_ser_graph_with_memory(
            graph,
            flatten_to_default_graph,
            blanks,
            source,
            memory,
        )?);
        Ok(())
    });
    match result {
        Ok(()) => Ok(staged.expect("successful native fold staged its result")),
        Err(cause) => {
            if let Some((dataset, bytes)) = staged {
                drop(dataset);
                let _ = context.release(|memory| memory.release_bytes(bytes));
            }
            Err(cause)
        }
    }
}

fn dataset_from_ser_graph_with_memory<S: Admission + ?Sized>(
    graph: &SerGraph,
    flatten_to_default_graph: bool,
    blanks: BlankIngress,
    source: Option<&mut SourceOccurrences>,
    memory: &mut Memory<'_, S>,
) -> Result<(RdfDataset, usize), NativeParseError> {
    let mut builder = RdfDatasetBuilder::new();
    let mapping_memory = RefCell::new(memory);
    let interner = SerInterner {
        graph,
        blanks,
        source: RefCell::new(source.filter(|source| source.is_active())),
        memory: &mapping_memory,
    };
    let mut rows = Vec::new();
    let result: Result<(), NativeParseError> = (|| {
        if !flatten_to_default_graph {
            for &name in &graph.named_graphs {
                mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
                let id = interner.intern(&mut builder, name)?;
                builder.declare_named_graph_with_memory(id, &mut mapping_memory.borrow_mut())?;
            }
        }

        let capacity = graph
            .quads
            .len()
            .checked_add(graph.reifiers.len())
            .and_then(|rows| rows.checked_add(graph.annotations.len()))
            .ok_or(StorageError::SizeOverflow)?;
        mapping_memory.borrow_mut().reserve(&mut rows, capacity)?;

        // Synthetic `rdf:reifies` rows reconstructed from the GTS reifier table, so the
        // shared fold re-binds them identically to the flat-quad path (pass 1). A
        // self-reifier sentinel — a `Triple` term whose `reifier` is its OWN id — is the
        // binding of an inline quoted-triple term used as a quad object, NOT a statement-
        // layer reifier; it carries no `<reifier> rdf:reifies <<…>>` row (the N-Quads
        // serializer skips it identically) and is resolved when its parent quad interns the
        // object. Emitting a synthetic row for it would make a quoted triple the subject of
        // `rdf:reifies`, which the IR rejects.
        for (native_row, &(reifier_id, (s, p, o), reifier_graph)) in
            graph.reifiers.iter().enumerate()
        {
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            if graph.terms.get(reifier_id).is_some_and(|term| {
                term.kind == SerTermKind::Triple && term.reifier == Some(reifier_id)
            }) {
                continue;
            }
            let subject = interner.intern(&mut builder, reifier_id)?;
            let predicate =
                builder.intern_iri_with_memory(RDF_REIFIES, &mut mapping_memory.borrow_mut())?;
            let s = interner.intern(&mut builder, s)?;
            let p = interner.intern(&mut builder, p)?;
            let o = interner.intern(&mut builder, o)?;
            // Preserve the reifier declaration's named graph (`None` = default) through the
            // GTS/round-trip path, unless this pass is flattening every graph to default.
            let graph = match reifier_graph {
                Some(_) if flatten_to_default_graph => None,
                Some(g) => Some(interner.intern(&mut builder, g)?),
                None => None,
            };
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            mapping_memory.borrow_mut().push(
                &mut rows,
                (
                    FoldRow {
                        subject,
                        is_reifies: true,
                        predicate,
                        object: FoldNode::Triple { s, p, o },
                        graph,
                    },
                    Some((NativeSourceRow::Reifier(native_row), None)),
                ),
            )?;
        }

        // Base quad rows.
        for (native_row, &(s, p, o, g)) in graph.quads.iter().enumerate() {
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            let subject = interner.intern(&mut builder, s)?;
            let is_reifies = interner.is_iri(p, RDF_REIFIES)?;
            let predicate = interner.intern(&mut builder, p)?;
            let object = interner.intern_node(&mut builder, o)?;
            let graph = match g {
                Some(_) if flatten_to_default_graph => None,
                Some(g) => Some(interner.intern(&mut builder, g)?),
                None => None,
            };
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            mapping_memory.borrow_mut().push(
                &mut rows,
                (
                    FoldRow {
                        subject,
                        is_reifies,
                        predicate,
                        object,
                        graph,
                    },
                    Some((NativeSourceRow::Quad(native_row), Some(o))),
                ),
            )?;
        }

        // Annotation rows the codec already classified into the GTS `annotations` table.
        // Re-materialized as `<reifier> <predicate> <value>` rows so the shared fold's pass 2
        // classifies them as annotations (the reifier subject is bound above). This is the
        // GENERAL RDF-1.2 parser, so it must accept arbitrary input including the W3C suite's
        // graph-scoped (TriG) reification — the optional graph slot is THREADED through (so a
        // reifier/annotation inside `GRAPH g { … }` is matchable under `GRAPH ?g`), unless
        // this pass is flattening every named graph to the default graph.
        for &(reifier_id, predicate_id, value_id, annotation_graph) in &graph.annotations {
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            let subject = interner.intern(&mut builder, reifier_id)?;
            let is_reifies = interner.is_iri(predicate_id, RDF_REIFIES)?;
            let predicate = interner.intern(&mut builder, predicate_id)?;
            let object = interner.intern_node(&mut builder, value_id)?;
            let graph = match annotation_graph {
                Some(_) if flatten_to_default_graph => None,
                Some(g) => Some(interner.intern(&mut builder, g)?),
                None => None,
            };
            mapping_memory.borrow_mut().admission_mut().checkpoint(1)?;
            mapping_memory.borrow_mut().push(
                &mut rows,
                (
                    FoldRow {
                        subject,
                        is_reifies,
                        predicate,
                        object,
                        graph,
                    },
                    None,
                ),
            )?;
        }

        #[expect(
            clippy::iter_with_drain,
            reason = "The original admitted rows backing remains live until release_vec refunds its allocation."
        )]
        fold_statement_layer_observed(
            &mut builder,
            rows.drain(..),
            &mapping_memory,
            |association, kind, terms| {
                if let Some((row, native_object)) = association
                    && let Some(source) = interner.source.borrow_mut().as_deref_mut()
                {
                    if let (Some(native), Some(term)) = (native_object, terms[2]) {
                        source.bind_final_native(native, term, &mut mapping_memory.borrow_mut())?;
                    }
                    source.bind_row(row, kind, terms, &mut mapping_memory.borrow_mut())?;
                }
                Ok(())
            },
        )?;
        Ok(())
    })();
    // The original input drain has ended before its backing owner is released;
    // pending storage was released by the same two-pass body. Persistent source
    // associations retain their own original grants, including on failure.
    let cleanup = mapping_memory
        .borrow_mut()
        .release_vec(rows)
        .map_err(NativeParseError::Storage);
    if let Err(cause) = result.and(cleanup) {
        // Disposal never replaces the first parse/fold failure.
        let _ = builder.discard_with_memory(&mut mapping_memory.borrow_mut());
        return Err(cause);
    }
    let mut memory = mapping_memory.borrow_mut();
    // Source associations have finished growing. Subtract only the builder's
    // originally admitted buffers before its consuming freeze replaces them.
    let unrelated = memory
        .admitted_bytes()
        .checked_sub(builder.native_buffer_bytes()?)
        .ok_or(StorageError::SizeOverflow)?;
    let dataset = builder.freeze_with_memory(&mut memory)?;
    let bytes = memory
        .admitted_bytes()
        .checked_sub(unrelated)
        .ok_or(StorageError::SizeOverflow)?;
    Ok((dataset, bytes))
}

/// Intern [`SerGraph`] terms into a builder, resolving quoted-triple terms through their
/// reifier binding.
///
/// A blank node's landing scope depends on [`BlankIngress`]: an [`Opaque`](BlankIngress::Opaque)
/// carrier graph folds every blank into [`BlankScope::DEFAULT`] (the folded graph has
/// already collapsed per-segment scope — the documented `bnode-scope-flatten` loss,
/// identical to `import_gts_graph`), while a [`TextDecoded`](BlankIngress::TextDecoded)
/// document's token is decoded back to the `(label, scope)` pair the serializer wrote it
/// from.
struct SerInterner<'a, 'm, 'b, 's, S: Admission + ?Sized> {
    graph: &'a SerGraph,
    /// Whether the graph's blank labels carry the text serializers' egress
    /// transform and must be decoded on the way in.
    blanks: BlankIngress,
    source: RefCell<Option<&'a mut SourceOccurrences>>,
    memory: &'m RefCell<&'b mut Memory<'s, S>>,
}

fn native_diagnostic<S: Admission + ?Sized>(
    code: &str,
    detail: &(impl core::fmt::Display + ?Sized),
    memory: &mut Memory<'_, S>,
) -> NativeParseError {
    match RdfDiagnostic::try_error_with_memory(code, detail, memory) {
        Ok(diagnostic) => NativeParseError::Diagnostic(diagnostic),
        Err(cause) => NativeParseError::Storage(cause),
    }
}

impl<S: Admission + ?Sized> SerInterner<'_, '_, '_, '_, S> {
    fn record_native(
        &self,
        native: usize,
        term: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        if let Some(source) = self.source.borrow_mut().as_deref_mut() {
            source.bind_final_native(native, term, memory)?;
        }
        Ok(())
    }

    /// Intern a term id into the builder, returning its [`TermId`]. A quoted-triple
    /// term resolves its `(s, p, o)` through the reifier table and interns as a triple.
    ///
    /// A quoted triple is interned over [`try_fold_nested_with_memory`]'s work list: its subject,
    /// predicate and object, each fully before the next, then the triple itself, and
    /// the first refusal ends the walk.
    ///
    /// # Termination
    ///
    /// The walk follows a quoted triple's components with no depth bound and no
    /// visited set. It terminates because the term table it walks does: every
    /// producer of a [`SerGraph`] guarantees that, and the one that takes a
    /// caller-supplied graph (`crate::gts::gts_to_ser`) proves it, refusing a
    /// self-reaching table with `gts-self-reaching-term`. The IR's own acyclicity
    /// check runs at `freeze()`, which is far too late to stop a walk around a cycle.
    fn intern(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
    ) -> Result<TermId, NativeParseError> {
        try_fold_nested_with_memory(
            gts_id,
            builder,
            |builder, gts_id, memory| self.resolve_node_with_memory(builder, gts_id, memory),
            |builder, native, s, p, o, memory| {
                let id = builder.intern_triple_with_memory(s, p, o, memory)?;
                self.record_native(native, id, memory)?;
                Ok(id)
            },
            // Scalar handles own no payload; interner/source allocations persist.
            |_, _, _| {},
            |error, _| NativeParseError::Storage(error),
            &mut self.memory.borrow_mut(),
        )
    }

    /// Intern a GTS term id, returning a [`FoldNode`]: a leaf becomes `Term`, a
    /// quoted-triple term becomes `Triple` (its components already interned) so a
    /// caller can fold it as a reifier binding rather than re-interning it.
    fn intern_node(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
    ) -> Result<FoldNode, NativeParseError> {
        match self.resolve_node(builder, gts_id)? {
            Nested::Leaf(id) => Ok(FoldNode::Term(id)),
            Nested::Triple(s, p, o) => {
                let s = self.intern(builder, s)?;
                let p = self.intern(builder, p)?;
                let o = self.intern(builder, o)?;
                Ok(FoldNode::Triple { s, p, o })
            }
        }
    }

    /// Intern a GTS term id that is not a quoted triple, or find a quoted triple's
    /// three component GTS ids through the reifier table, interning none of them.
    fn resolve_node(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
    ) -> Result<Nested<usize, TermId>, NativeParseError> {
        self.resolve_node_with_memory(builder, gts_id, &mut self.memory.borrow_mut())
    }

    fn resolve_node_with_memory(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<Nested<usize, TermId>, NativeParseError> {
        let term = self.graph.terms.get(gts_id).ok_or_else(|| {
            native_diagnostic(
                "native-codec-term-out-of-range",
                &format_args!("GTS term id {gts_id} is out of range"),
                memory,
            )
        })?;
        let resolved: Result<Nested<usize, TermId>, NativeParseError> = match term.kind {
            SerTermKind::Iri => {
                let iri = term
                    .value
                    .as_deref()
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        native_diagnostic(
                            "native-codec-iri-missing-value",
                            &"GTS IRI term requires a non-empty value",
                            memory,
                        )
                    })?;
                Ok(Nested::Leaf(builder.intern_iri_with_memory(iri, memory)?))
            }
            SerTermKind::Bnode => {
                // The label is only ever read through `&label`: borrow the table's
                // string and build a fresh one solely for the missing-value fallback.
                let label: Cow<'_, str> = match term.value.as_deref() {
                    Some(label) => Cow::Borrowed(label),
                    None => Cow::Owned(
                        memory.format_with_progress(&format_args!("gts_bnode_{gts_id}"))?,
                    ),
                };
                let result = (|| -> Result<TermId, StorageError> {
                    Ok(match self.blanks {
                        BlankIngress::Opaque => {
                            builder.intern_blank_with_memory(&label, BlankScope::DEFAULT, memory)?
                        }
                        BlankIngress::TextDecoded(alphabet) => {
                            let (decoded, scope) =
                                purrdf_core::blank_label::decode_blank_label_with_memory(
                                    &label, alphabet, memory,
                                )?;
                            let result = builder.intern_blank_with_memory(&decoded, scope, memory);
                            if let Cow::Owned(decoded) = decoded {
                                let cleanup = memory.release_string(decoded);
                                if result.is_ok() {
                                    cleanup?;
                                }
                            }
                            result?
                        }
                    })
                })();
                let cleanup = match label {
                    Cow::Owned(label) => memory.release_string(label),
                    Cow::Borrowed(_) => Ok(()),
                };
                Ok(Nested::Leaf(result.and_then(|id| cleanup.map(|()| id))?))
            }
            SerTermKind::Literal => {
                let datatype = match term.datatype {
                    Some(dt_id) => Some(self.iri_value(dt_id, memory)?),
                    None => None,
                };
                let direction = crate::gts_resolve::parse_gts_direction_with_memory(
                    term.direction.as_deref(),
                    term.lang.as_deref(),
                    memory,
                )?;
                // A composite literal's embedded blank labels bind through the
                // SAME rule the bare `_:` tokens above use — that agreement is
                // what makes `_:b` written as a subject and `_:b` written inside
                // a `cdt:List` the same node.
                Ok(Nested::Leaf(
                    builder
                        .intern_literal_parts_bound_with_memory(
                            term.value.as_deref().unwrap_or_default(),
                            datatype,
                            term.lang.as_deref(),
                            direction,
                            self.blanks.binding(),
                            memory,
                        )
                        .map_err(|cause| match cause {
                            purrdf_core::cdt_blank::CdtBlankError::Storage(cause) => {
                                NativeParseError::Storage(cause)
                            }
                            cause => NativeParseError::Composite(cause),
                        })?,
                ))
            }
            SerTermKind::Triple => {
                let reifier_id = term.reifier.ok_or_else(|| {
                    native_diagnostic(
                        "native-codec-unbound-triple-term",
                        &"GTS triple term has no reifier binding",
                        memory,
                    )
                })?;
                let (s, p, o) = self
                    .graph
                    .reifier_with_control(reifier_id, memory)?
                    .ok_or_else(|| {
                        native_diagnostic(
                            "native-codec-missing-reifier-binding",
                            &format_args!(
                                "GTS triple term references missing reifier {reifier_id}"
                            ),
                            memory,
                        )
                    })?;
                Ok(Nested::Triple(s, p, o))
            }
        };
        let resolved = resolved?;
        if let Nested::Leaf(id) = resolved {
            self.record_native(gts_id, id, memory)?;
        }
        Ok(resolved)
    }

    /// Intern a GTS term id known to occupy an IRI position (predicate / datatype),
    /// returning its IRI string for the `rdf:reifies` check and literal datatype.
    fn iri_value(
        &self,
        gts_id: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<&str, NativeParseError> {
        let term = self.graph.terms.get(gts_id).ok_or_else(|| {
            native_diagnostic(
                "native-codec-term-out-of-range",
                &format_args!("GTS term id {gts_id} is out of range"),
                memory,
            )
        })?;
        match term.kind {
            SerTermKind::Iri => term
                .value
                .as_deref()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| {
                    native_diagnostic(
                        "native-codec-iri-missing-value",
                        &"GTS IRI term requires a non-empty value",
                        memory,
                    )
                }),
            other => Err(native_diagnostic(
                "native-codec-predicate-not-iri",
                &format_args!("GTS term in an IRI position must be an IRI, got {other:?}"),
                memory,
            )),
        }
    }

    /// Return whether a GTS term id is a specific IRI, failing if the term is not valid
    /// in an IRI position.
    fn is_iri(&self, gts_id: usize, expected: &str) -> Result<bool, NativeParseError> {
        let mut memory = self.memory.borrow_mut();
        let value = self.iri_value(gts_id, &mut memory)?;
        purrdf_lex::control::compare_bytes(value.as_bytes(), expected.as_bytes(), &mut |units| {
            memory.admission_mut().checkpoint(units)
        })
        .map(core::cmp::Ordering::is_eq)
        .map_err(NativeParseError::Storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RdfTextDirection;
    use crate::TermValue;

    #[test]
    fn parses_basic_ntriples() {
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n\
                  <https://e/s> <https://e/p2> \"lit\" .\n";
        let ds = parse_dataset(nt.as_bytes(), "application/n-triples", None).expect("parse");
        assert_eq!(ds.quad_count(), 2);
        assert!(ds.term_count() >= 4);
    }

    #[test]
    fn folds_rdf12_statement_layer_to_parity() {
        // The exact RDF 1.2 fixture from dataset_io.rs:131: a reifier binding + an
        // annotation. The base quad table is empty; the reifier and annotation land in
        // their own tables (parity gate R1).
        let nt = concat!(
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" .\n",
        );
        let ds = parse_dataset(nt.as_bytes(), "application/n-triples", None).expect("parse");
        assert_eq!(ds.quad_count(), 0, "reifier rows are not base quads");
        assert_eq!(ds.reifiers().count(), 1);
        assert_eq!(ds.annotations().count(), 1);
    }

    #[test]
    fn an_annotation_only_attaches_to_a_same_graph_reifier() {
        // The RDF 1.2 statement layer is per-graph. `_:r` is a reifier in `g1` and
        // nothing at all in `g2`, so the SAME `(subject, predicate, object)` shape is
        // an annotation in `g1` and an ordinary quad in `g2`. Keying the fold on the
        // subject alone would swallow the `g2` row into the annotation table, putting
        // this reader at odds with the `CONSTRUCT` template evaluator and making a
        // constructed dataset unequal to the re-parse of its own serialization.
        let nq = concat!(
            "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://e/s> <https://e/p> <https://e/o> )>> <https://e/g1> .\n",
            "<https://e/r> <https://e/confidence> \"0.9\" <https://e/g1> .\n",
            "<https://e/r> <https://e/confidence> \"0.1\" <https://e/g2> .\n",
        );
        let ds = parse_dataset(nq.as_bytes(), "application/n-quads", None).expect("parse");
        assert_eq!(ds.reifiers().count(), 1, "one reifier, declared in g1");
        assert_eq!(
            ds.annotations().count(),
            1,
            "only the g1 row annotates the g1 reifier"
        );
        assert_eq!(
            ds.quad_count(),
            1,
            "the g2 row is an ordinary quad of g2, not an annotation"
        );
    }

    #[test]
    fn turtle_base_resolves_relative_iri() {
        // A relative-IRI Turtle doc parsed with a base resolves the subject against it.
        let ttl = "<rel> <https://e/p> <https://e/o> .\n";
        let ds = parse_dataset(ttl.as_bytes(), "text/turtle", Some("https://example.org/"))
            .expect("parse with base");
        assert_eq!(ds.quad_count(), 1);
        assert!(
            ds.term_id_by_value(&TermValue::Iri("https://example.org/rel".to_owned()))
                .is_some(),
            "relative <rel> must resolve against the base IRI"
        );
    }

    #[test]
    fn literal_direction_survives_parse() {
        let nt = concat!(
            "<https://e/s> <https://e/p> ",
            "\"\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}\"@ar--rtl .\n",
        );
        let ds = parse_dataset(nt.as_bytes(), "application/n-triples", None).expect("parse");
        // RDF 1.2 gives a directional language-tagged literal rdf:dirLangString;
        // its base direction also participates in the literal identity key.
        assert!(
            ds.term_id_by_value(&TermValue::Literal {
                lexical_form: "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
            .is_some(),
            "an rtl directional literal must survive the parse"
        );
    }

    /// Deep quoted operands also traverse inherited annotation origins. The
    /// public controlled boundary retains those sources and the caller's stop.
    #[test]
    fn deep_annotated_terms_preserve_sources_and_original_stop_on_a_small_stack() {
        purrdf_stack::on_stack(128 * 1024, || {
            use purrdf_core::DatasetView;
            const LEVELS: usize = 256;
            let text = format!(
                "<urn:s> <urn:p> {}<urn:o>{} {{| <urn:note> <urn:value> |}} .",
                "<<( <urn:s> <urn:p> ".repeat(LEVELS),
                " )>>".repeat(LEVELS),
            );
            let mut calls = 0;
            let owner = parse_dataset_controlled(
                text.as_bytes(),
                NativeRdfFormat::Turtle,
                None,
                Resident,
                |_| {
                    calls += 1;
                    Ok::<_, usize>(())
                },
            )
            .unwrap_or_else(|_| panic!("finite annotated source parses"));
            {
                let view = owner.dataset().expect("complete dataset");
                assert_eq!(view.quads().count(), 1);
                assert_eq!(view.reifier_quads().count(), 1);
                assert_eq!(view.annotation_quads().count(), 1);
                assert_eq!(owner.source().bytes(), text.as_bytes());
                let source = owner.source();
                let deepest = source
                    .occurrences()
                    .map(|occurrence| {
                        let mut origin = occurrence.origin();
                        let mut depth = 0;
                        while let Some([_, _, object]) = source.origin(origin).unwrap().components()
                        {
                            depth += 1;
                            origin = object;
                        }
                        depth
                    })
                    .max()
                    .unwrap();
                assert_eq!(deepest, LEVELS + 1);
            }
            drop(owner);
            let total = calls;
            for fail_at in [total / 2, total - 1] {
                let mut calls = 0;
                let failure = parse_dataset_controlled(
                    text.as_bytes(),
                    NativeRdfFormat::Turtle,
                    None,
                    Resident,
                    |_| {
                        let at = calls;
                        calls += 1;
                        if at == fail_at { Err(at) } else { Ok(()) }
                    },
                )
                .expect_err("the selected original callback refuses");
                assert!(matches!(failure.cause(), ControlledError::Control(at) if *at == fail_at));
                assert_eq!(failure.input(), text.as_bytes());
                drop(failure);
                assert_eq!(
                    calls,
                    fail_at + 1,
                    "no callback follows the original refusal"
                );
            }
        })
        .expect("the thread starts");
    }

    #[test]
    fn invalid_utf8_hard_fails() {
        use super::super::acquisition::NativeParseError;
        use purrdf_lex::{
            allocation::{Admission, StorageError},
            control::ControlledError,
        };
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        struct Grant {
            live: Arc<AtomicUsize>,
            limit: usize,
        }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.limit {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live.store(bytes, Ordering::SeqCst);
                Ok(())
            }
        }
        let verify = |input: &[u8]| {
            let live = Arc::new(AtomicUsize::new(0));
            let context = RunContext::new(
                Grant {
                    live: Arc::clone(&live),
                    limit: usize::MAX,
                },
                |units| {
                    assert!(units <= 256, "bounded native validation and copy extents");
                    Ok::<_, core::convert::Infallible>(())
                },
            );
            match (std::str::from_utf8(input), context.source(input)) {
                (Ok(expected), Ok(mut source)) => {
                    source.with_text(|text, context| {
                        assert_eq!(text.as_bytes(), input);
                        assert_eq!(text, expected);
                        assert!(context.live >= text.len());
                    });
                    drop(source);
                }
                (Err(expected), Err(failure)) => {
                    assert!(
                        core::ptr::eq(failure.input(), input),
                        "full borrowed original survives"
                    );
                    assert_eq!(
                        failure.admitted_bytes(),
                        0,
                        "partial source copy refunded before reporting"
                    );
                    assert_eq!(live.load(Ordering::SeqCst), 0);
                    let ControlledError::Native(NativeParseError::Utf8 { cause, offset }) =
                        failure.cause()
                    else {
                        panic!("native UTF-8 evidence");
                    };
                    let observed = super::super::stream::utf8_error(cause, *offset);
                    let expected = super::super::stream::utf8_error(&expected, 0);
                    assert_eq!(observed.code, expected.code);
                    assert_eq!(observed.message, expected.message);
                    assert_eq!(
                        observed.presentation().unwrap().to_json(),
                        expected.presentation().unwrap().to_json()
                    );
                    drop(failure);
                }
                _ => panic!("bounded acquisition must preserve native UTF-8 verdict"),
            }
            assert_eq!(live.load(Ordering::SeqCst), 0);
        };
        verify(b"");
        for offset in [0, 253, 254, 255, 256, 257, 509, 510, 511] {
            for scalar in ["é", "€", "🦀"] {
                let input = format!("{}{scalar}tail", "a".repeat(offset));
                verify(input.as_bytes());
            }
            for suffix in [
                b"\xff\xfe".as_slice(),
                b"\xe2(\xa1",
                b"\xf0\x9f\x92",
                b"\xed\xa0\x80",
                b"\xc0\xaf",
            ] {
                let mut input = vec![b'a'; offset];
                input.extend_from_slice(suffix);
                verify(&input);
                let error = parse_dataset(&input, "text/turtle", None).expect_err("invalid UTF-8");
                assert_eq!(error.code, "native-codec-utf8");
            }
        }
        // Every original callback, including the first and final zero poll, can
        // refuse without a later poll, admitted partial copy or lost input.
        struct Stop(usize);
        let input = format!("{}🦀{}", "a".repeat(255), "b".repeat(300));
        let mut calls = 0;
        let source = RunContext::new(Resident, |_| {
            calls += 1;
            Ok::<_, Stop>(())
        })
        .source(input.as_bytes())
        .unwrap_or_else(|_| panic!("accepting source"));
        drop(source);
        let total = calls;
        for fail_at in 0..total {
            let live = Arc::new(AtomicUsize::new(0));
            let mut calls = 0;
            let result = RunContext::new(
                Grant {
                    live: Arc::clone(&live),
                    limit: usize::MAX,
                },
                |_| {
                    let at = calls;
                    calls += 1;
                    if at == fail_at { Err(Stop(at)) } else { Ok(()) }
                },
            )
            .source(input.as_bytes());
            let Err(failure) = result else {
                panic!("selected callback must refuse")
            };
            assert!(core::ptr::eq(failure.input(), input.as_bytes()));
            assert!(
                matches!(failure.cause(), ControlledError::Control(Stop(at)) if *at == fail_at)
            );
            assert_eq!(failure.admitted_bytes(), 0);
            assert_eq!(live.load(Ordering::SeqCst), 0);
            drop(failure);
            assert_eq!(calls, fail_at + 1, "no callback after the first refusal");
        }
        let live = Arc::new(AtomicUsize::new(0));
        let Err(failure) = RunContext::new(
            Grant {
                live: Arc::clone(&live),
                limit: 256,
            },
            |_| Ok::<_, Stop>(()),
        )
        .source(input.as_bytes()) else {
            panic!("source admission must be refused")
        };
        assert!(core::ptr::eq(failure.input(), input.as_bytes()));
        assert!(matches!(
            failure.cause(),
            ControlledError::Native(NativeParseError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(failure.admitted_bytes(), 0);
        assert_eq!(live.load(Ordering::SeqCst), 0);
        drop(failure);
        // Unsupported formats refuse before UTF-8, admission or stop polling.
        for input in [b"valid text".as_slice(), b"\xff"] {
            let mut polls = 0;
            let result = parse_dataset_controlled(
                input,
                NativeRdfFormat::NTriples,
                None,
                Grant {
                    live: Arc::clone(&live),
                    limit: 0,
                },
                |_| {
                    polls += 1;
                    Err(Stop(0))
                },
            );
            let Err(failure) = result else {
                panic!("unsupported input cannot succeed")
            };
            assert!(core::ptr::eq(failure.input(), input));
            assert!(matches!(
                failure.cause(),
                ControlledError::Native(NativeParseError::UnsupportedFormat(
                    NativeRdfFormat::NTriples
                ))
            ));
            drop(failure);
            assert_eq!(polls, 0);
            assert_eq!(live.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn malformed_text_is_rejected_without_unwinding() {
        // The first-party line/Turtle-family parser returns a clean parse
        // diagnostic for malformed input rather than unwinding — strictly better than
        // the prior purrdf-gts codec, which panicked on this fragment and relied on the
        // `catch_unwind` guard to convert the panic to `native-codec-panic`. The guard
        // is retained for any genuine downstream panic; here the path now reports a
        // structured `native-codec-parse` error.
        let err = parse_dataset("0À".as_bytes(), "text/turtle", None)
            .expect_err("malformed input must be rejected without unwinding");
        assert_eq!(err.code, "native-codec-parse");
    }

    #[test]
    fn public_parse_dataset_parallel_path_matches_forced_sequential() {
        // Above the `text_parse` parallel threshold the public `parse_dataset` entry
        // takes the chunk-parallel N-Quads pipeline; the forced-sequential twin is the
        // baseline it must match byte-for-byte: same term ids, same quad order, same
        // canonical N-Quads back out.
        use std::fmt::Write as _;
        let mut text = String::with_capacity(2 << 20);
        for i in 0..14_000 {
            writeln!(
                text,
                "<https://example.org/s{}> <https://example.org/p{}> \"v{i}\"@en \
                 <https://example.org/g{}> .",
                i % 611,
                i % 17,
                i % 5
            )
            .expect("write row");
        }
        assert!(text.len() >= 1 << 20, "fixture must cross the threshold");

        let auto = parse_dataset(text.as_bytes(), "application/n-quads", None).expect("auto");
        let seq = parse_dataset_forced_sequential(text.as_bytes(), "application/n-quads", None)
            .expect("sequential");
        assert_eq!(auto.term_count(), seq.term_count());
        assert!(
            auto.quads().collect::<Vec<_>>() == seq.quads().collect::<Vec<_>>(),
            "frozen quad rows (term ids + order) must be identical"
        );
        let ser_auto = crate::native_codecs::serialize_dataset(
            &auto,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize auto");
        let ser_seq = crate::native_codecs::serialize_dataset(
            &seq,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize sequential");
        assert!(ser_auto == ser_seq, "canonical bytes must be identical");
    }

    #[test]
    fn unknown_media_type_hard_fails() {
        let err =
            parse_dataset(b"", "application/json", None).expect_err("unknown media type must fail");
        assert_eq!(err.code, "native-codec-unsupported-format");
    }

    #[test]
    fn tracking_off_returns_no_table() {
        // With tracking off the side table is absent and the dataset is byte-for-byte
        // what `parse_dataset` produces (same canonical serialization).
        let nt = "<https://e/s> <https://e/p> <https://e/o> .\n";
        let outcome = parse_dataset_with(
            nt.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions::default(),
        )
        .expect("parse");
        let (with_ds, table) = (outcome.dataset, outcome.spans);
        assert!(table.is_none(), "tracking off yields no side table");
        let plain = parse_dataset(nt.as_bytes(), "application/n-triples", None).expect("parse");
        assert_eq!(with_ds.quad_count(), plain.quad_count());
        let a = crate::native_codecs::serialize_dataset(
            &with_ds,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize with");
        let b = crate::native_codecs::serialize_dataset(
            &plain,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize plain");
        assert_eq!(
            a, b,
            "dataset is identical whether or not tracking is requested"
        );
    }

    #[test]
    fn nt_tracking_records_subject_line() {
        // A focus NamedNode renders as a bare IRI string, so the subject key is the
        // bare IRI (no angle brackets). Line 1/2/3 map to their subjects.
        let nt = concat!(
            "<http://example.org/alice> <http://example.org/p> \"a\" .\n",
            "<http://example.org/bob> <http://example.org/p> \"b\" .\n",
            "<http://example.org/carol> <http://example.org/p> \"c\" .\n",
        );
        let options = ParseOptions {
            track_source_spans: true,
            yaml_ld: None,
            yaml_ld_context: None,
        };
        let table = parse_dataset_with(nt.as_bytes(), "application/n-triples", None, &options)
            .expect("parse")
            .spans
            .expect("tracking on yields a table");
        assert_eq!(
            table
                .position_for_subject("http://example.org/alice")
                .expect("alice tracked")
                .line,
            1
        );
        assert_eq!(
            table
                .position_for_subject("http://example.org/bob")
                .expect("bob tracked")
                .line,
            2
        );
        assert_eq!(
            table
                .position_for_subject("http://example.org/carol")
                .expect("carol tracked")
                .line,
            3
        );
    }

    #[test]
    fn nt_tracking_records_document_global_byte_offset() {
        // The recorded byte_offset for a subject must be the document-global byte
        // offset of the line it starts on: 0 for the first line (a real, emitted
        // value — not a dropped sentinel) and the running sum of prior line lengths
        // (payload plus each `\n`) for later lines.
        let line1 = "<http://example.org/alice> <http://example.org/p> \"a\" .\n";
        let line2 = "<http://example.org/bob> <http://example.org/p> \"b\" .\n";
        let line3 = "<http://example.org/carol> <http://example.org/p> \"c\" .\n";
        let nt = format!("{line1}{line2}{line3}");
        let options = ParseOptions {
            track_source_spans: true,
            yaml_ld: None,
            yaml_ld_context: None,
        };
        let table = parse_dataset_with(nt.as_bytes(), "application/n-triples", None, &options)
            .expect("parse")
            .spans
            .expect("tracking on yields a table");
        // First line: offset 0 must be recorded, not dropped.
        assert_eq!(
            table
                .position_for_subject("http://example.org/alice")
                .expect("alice tracked")
                .byte_offset,
            0,
            "first-line subject starts at document byte offset 0"
        );
        assert_eq!(
            table
                .position_for_subject("http://example.org/bob")
                .expect("bob tracked")
                .byte_offset,
            line1.len(),
            "second-line subject starts just past line 1"
        );
        assert_eq!(
            table
                .position_for_subject("http://example.org/carol")
                .expect("carol tracked")
                .byte_offset,
            line1.len() + line2.len(),
            "third-line subject starts just past lines 1 and 2"
        );
    }

    #[test]
    fn turtle_tracking_records_subject_line() {
        // The subject `ex:s` appears on line 3 (after two directive lines).
        let ttl = concat!(
            "@prefix ex: <http://example.org/> .\n",
            "\n",
            "ex:s ex:p ex:o .\n",
        );
        let options = ParseOptions {
            track_source_spans: true,
            yaml_ld: None,
            yaml_ld_context: None,
        };
        let table = parse_dataset_with(ttl.as_bytes(), "text/turtle", None, &options)
            .expect("parse")
            .spans
            .expect("tracking on yields a table");
        let position = table
            .position_for_subject("http://example.org/s")
            .expect("subject tracked");
        assert_eq!(position.line, 3, "subject ex:s is asserted on line 3");
    }

    #[test]
    fn dataset_is_identical_with_tracking() {
        use super::super::source_occurrence::{SourceRole, SourceSemanticReference};
        let nt = concat!(
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
            "<http://example.org/s> <http://example.org/p2> \"lit\" .\n",
        );
        let trig = concat!(
            "@prefix ex: <https://example.org/> .\n",
            "ex:s ex:p ex:o . ex:s ex:p ex:o .\n",
            "ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .\n",
            "ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .\n",
            "ex:r ex:confidence \"0.9\" .\n",
            "ex:g1 { ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .\n",
            "ex:r ex:confidence \"0.8\" . }\n",
            "ex:g2 { ex:r ex:confidence \"0.1\" . }\n",
            "ex:g1 { ex:s ex:nested <<( ex:s ex:p <<( ex:s ex:p ex:o )>> )>> . }\n",
            "ex:empty {}\n",
        );
        use purrdf_core::DatasetView;
        for (text, media) in [(nt, "application/n-triples"), (trig, "application/trig")] {
            let retained = parse_text_retained(
                text.as_bytes(),
                classify(media).expect("native media"),
                None,
                ParseOptions {
                    track_source_spans: true,
                    yaml_ld: None,
                    yaml_ld_context: None,
                },
            )
            .unwrap_or_else(|_| panic!("tracked source acquisition"));
            let plain = parse_dataset(text.as_bytes(), media, None).expect("plain");
            {
                let view = retained.dataset().expect("retained dataset");
                // These are the actual query-admission boundaries: a view
                // with no workspace provider must reject even zero-byte
                // preflight rather than authorize resident execution.
                for bytes in [0, 64] {
                    assert!(matches!(
                        view.reserve_workspace(bytes),
                        Err(super::super::retained::BorrowedReadError::WorkspaceUnavailable)
                    ));
                    assert!(view.reserve_owned_workspace(bytes).is_none());
                }
                assert_eq!(view.term_count(), plain.term_count());
                assert!(
                    view.quads().eq(plain.quads()),
                    "identical native term IDs and quads"
                );
                let plan = view.probe_plan(false, false, false, purrdf_core::GraphMatch::Any);
                let plain_plan = DatasetView::probe_plan(
                    plain.as_ref(),
                    false,
                    false,
                    false,
                    purrdf_core::GraphMatch::Any,
                );
                assert!(
                    view.quads_for_pattern_with_plan(
                        &plan,
                        None,
                        None,
                        None,
                        purrdf_core::GraphMatch::Any
                    )
                    .eq(DatasetView::quads_for_pattern_with_plan(
                        plain.as_ref(),
                        &plain_plan,
                        None,
                        None,
                        None,
                        purrdf_core::GraphMatch::Any
                    ))
                );
                assert!(view.reifier_quads().eq(plain.reifier_quads()));
                assert!(view.annotation_quads().eq(plain.annotation_quads()));
                assert!(
                    view.named_graphs().eq(plain.named_graphs()),
                    "empty graphs survive the facade"
                );
                assert_eq!(view.stats_fingerprint(), plain.stats_fingerprint());
                assert_eq!(
                    view.snapshot_identity().unwrap(),
                    DatasetView::snapshot_identity(plain.as_ref()).unwrap()
                );
                assert_eq!(
                    view.source_statement_count().unwrap(),
                    DatasetView::source_statement_count(plain.as_ref()).unwrap()
                );
                let sources = retained.source();
                assert_eq!(sources.bytes(), text.as_bytes());
                if media == "application/n-triples" {
                    assert_eq!(
                        sources.occurrences().count(),
                        0,
                        "native source associations are Turtle/TriG only"
                    );
                } else {
                    let kinds = (0..9)
                        .map(|statement| sources.statement_kind(statement))
                        .collect::<Vec<_>>();
                    use SourceStatementKind::{Annotation, Quad, Reifier};
                    assert_eq!(
                        kinds,
                        [
                            Quad, Quad, Reifier, Reifier, Annotation, Reifier, Annotation, Quad,
                            Quad
                        ]
                        .map(Some)
                    );
                    assert_eq!(sources.statement_kind(9), None);
                    let subjects = sources
                        .occurrences()
                        .filter(|occurrence| occurrence.role() == SourceRole::Subject)
                        .count();
                    assert_eq!(
                        subjects, 9,
                        "duplicate assertions and diverted reifier roots survive"
                    );
                    let declarations = sources
                        .occurrences()
                        .filter(|occurrence| occurrence.role() == SourceRole::GraphDeclaration)
                        .count();
                    assert_eq!(
                        declarations, 4,
                        "repeated and empty graph declarations survive"
                    );
                    for occurrence in sources.occurrences() {
                        let origin = sources.origin(occurrence.origin()).expect("source origin");
                        let Some(SourceSemanticReference::Term(id)) = origin.semantic() else {
                            panic!(
                                "each emitted root or declaration was actually interned: {origin:?}"
                            );
                        };
                        assert_eq!(
                            view.term_value(id).expect("retained term"),
                            plain.term_value(id).expect("resident source term"),
                            "source association preserves native ID meaning"
                        );
                        if let Some(components) = origin.components() {
                            for component in components {
                                assert!(matches!(
                                    sources
                                        .origin(component)
                                        .expect("component origin")
                                        .semantic(),
                                    Some(SourceSemanticReference::Term(_))
                                ));
                            }
                        }
                    }
                }
            }
            if media == "application/trig" {
                let controlled = parse_dataset_controlled(
                    text.as_bytes(),
                    NativeRdfFormat::TriG,
                    None,
                    Resident,
                    |_| Ok::<_, ()>(()),
                )
                .expect("public controlled TriG");
                assert!(controlled.dataset().unwrap().quads().eq(plain.quads()));
                let actual = controlled.source();
                let expected = retained.source();
                assert!(actual.occurrences().eq(expected.occurrences()));
                for occurrence in actual.occurrences() {
                    assert_eq!(
                        actual.origin(occurrence.origin()),
                        expected.origin(occurrence.origin())
                    );
                }
            }
            // The legacy resident adapter keeps its original owned return
            // contract after the private retained facade has been released.
            let tracked = retained.into_legacy().expect("resident outcome");
            let serialize = |dataset: &RdfDataset| {
                crate::native_codecs::serialize_dataset(
                    dataset,
                    "application/n-quads",
                    crate::SerializeGraph::Dataset,
                )
                .expect("serialize")
            };
            assert_eq!(serialize(&tracked.dataset), serialize(&plain));
        }
        let text = "<urn:s> <urn:p> ( <urn:o> ) .";
        let controlled = parse_dataset_controlled(
            text.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            Resident,
            |_| Ok::<_, ()>(()),
        )
        .expect("public collection expansion");
        let source = controlled.source();
        let mut generated_cell = false;
        for occurrence in source.occurrences() {
            let mut origin = source.origin(occurrence.origin()).unwrap();
            while let super::super::SourceOriginKind::Inherited { from } = origin.kind() {
                origin = source.origin(*from).unwrap();
            }
            if let super::super::SourceOriginKind::Generated { rule, anchor, .. } = origin.kind() {
                assert_ne!(&text[anchor.clone()], "");
                generated_cell |=
                    matches!(rule, super::super::SourceRule::CollectionCell { index: 0 });
            }
        }
        assert!(
            generated_cell,
            "collection cells retain a generation rule instead of an invented spelling"
        );
    }

    /// The document prefix map is the GRAMMAR's record: a `PREFIX` line quoted inside a
    /// long string literal is text, never a declaration, and a label declared twice
    /// reports its LAST namespace — on the hot path and the span-tracking path alike.
    #[test]
    fn document_prefixes_are_the_grammars_declarations() {
        let ttl = concat!(
            "@prefix ex: <https://example.org/a#> .\n",
            "PREFIX rel: <rel/>\n",
            "ex:s ex:p \"\"\"\n",
            "PREFIX evil: <urn:evil:>\n",
            "@prefix ex: <urn:evil:ex#> .\n",
            "\"\"\" .\n",
            "@prefix ex: <https://example.org/b#> .\n",
            "ex:s ex:q ex:o .\n",
        );
        let expected = vec![
            ("ex".to_owned(), "https://example.org/b#".to_owned()),
            ("rel".to_owned(), "https://example.org/base/rel/".to_owned()),
        ];
        for track_source_spans in [false, true] {
            let outcome = parse_dataset_with(
                ttl.as_bytes(),
                "text/turtle",
                Some("https://example.org/base/"),
                &ParseOptions {
                    track_source_spans,
                    yaml_ld: None,
                    yaml_ld_context: None,
                },
            )
            .expect("parse");
            assert_eq!(
                outcome.document_prefixes, expected,
                "tracking: {track_source_spans}"
            );
        }
    }

    /// No declaration, no report: the parser's `rdf:` convenience binding resolves
    /// `rdf:type` without ever being a document declaration, and formats with no
    /// `@prefix` directive report nothing.
    #[test]
    fn document_prefixes_report_nothing_undeclared() {
        let ttl = "<https://example.org/s> rdf:type <https://example.org/C> .\n";
        let outcome = parse_dataset_with(
            ttl.as_bytes(),
            "text/turtle",
            None,
            &ParseOptions::default(),
        )
        .expect("parse");
        assert_eq!(outcome.dataset.quad_count(), 1);
        assert_eq!(outcome.document_prefixes, Vec::<(String, String)>::new());
        let nt = "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n";
        let outcome = parse_dataset_with(
            nt.as_bytes(),
            "application/n-triples",
            None,
            &ParseOptions::default(),
        )
        .expect("parse");
        assert_eq!(outcome.document_prefixes, Vec::<(String, String)>::new());
    }

    /// A failed parse reports the directives read BEFORE the failing statement, and
    /// none after it — including none quoted inside a literal.
    #[test]
    fn a_failure_reports_the_directives_in_force_where_it_stopped() {
        let ttl = concat!(
            "@prefix a: <https://example.org/a#> .\n",
            "@base <https://example.org/doc/> .\n",
            "a:s a:p \"\"\"\nPREFIX q: <urn:q:>\n\"\"\" .\n",
            "PREFIX b: <rel#>\n",
            "a:s a:p .\n",
            "@prefix c: <https://example.org/c#> .\n",
        );
        let retained = parse_dataset_controlled(
            ttl.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            Resident,
            Unmetered,
        )
        .expect_err("the fifth line has no object");
        assert!(matches!(
            retained.cause(),
            ControlledError::Native(NativeParseError::Diagnostic(_))
        ));
        assert_eq!(retained.input(), ttl.as_bytes());
        assert_eq!(
            retained.document_prefixes().collect::<Vec<_>>(),
            [
                ("a", "https://example.org/a#"),
                ("b", "https://example.org/doc/rel#"),
            ]
        );
        assert_eq!(
            retained.document_base_iri(),
            Some("https://example.org/doc/")
        );
        let failure = parse_dataset_reporting_failure(
            ttl.as_bytes(),
            "text/turtle",
            None,
            &ParseOptions::default(),
        )
        .expect_err("the fifth line has no object");
        assert_eq!(
            failure.document_prefixes,
            vec![
                ("a".to_owned(), "https://example.org/a#".to_owned()),
                ("b".to_owned(), "https://example.org/doc/rel#".to_owned()),
            ]
        );
        assert_eq!(
            failure.document_base_iri(),
            Some("https://example.org/doc/")
        );
        let plain = parse_dataset_with(
            ttl.as_bytes(),
            "text/turtle",
            None,
            &ParseOptions::default(),
        )
        .expect_err("the same failure");
        assert_eq!(plain.to_string(), failure.diagnostic.to_string());

        // An original caller refusal after source acquisition remains owned
        // beside that source, and dies before the same account is refunded.
        use purrdf_lex::{
            allocation::{Admission, StorageError},
            control::ControlledError,
        };
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        struct Grant {
            live: Arc<AtomicUsize>,
            refuse: Arc<AtomicBool>,
            source_len: usize,
        }
        impl Admission for Grant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                self.live.store(bytes, Ordering::SeqCst);
                if bytes > self.source_len {
                    self.refuse.store(true, Ordering::SeqCst);
                }
                Ok(())
            }
        }
        struct Stop {
            marker: Box<u8>,
            live: Arc<AtomicUsize>,
        }
        impl Drop for Stop {
            fn drop(&mut self) {
                assert!(
                    self.live.load(Ordering::SeqCst) > 0,
                    "original failure dies before original source grant"
                );
            }
        }
        let live = Arc::new(AtomicUsize::new(0));
        let refuse = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&refuse);
        let marker = Box::new(7);
        let identity = core::ptr::from_ref(marker.as_ref());
        let mut marker = Some(marker);
        let error_live = Arc::clone(&live);
        let retained = parse_dataset_controlled(
            ttl.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            Grant {
                live: Arc::clone(&live),
                refuse: Arc::clone(&refuse),
                source_len: ttl.len(),
            },
            move |_| {
                if flag.load(Ordering::SeqCst) {
                    Err(Stop {
                        marker: marker.take().expect("first original refusal"),
                        live: Arc::clone(&error_live),
                    })
                } else {
                    Ok(())
                }
            },
        )
        .expect_err("original caller refused after source acquisition");
        assert_eq!(retained.input(), ttl.as_bytes());
        let ControlledError::Control(stop) = retained.cause() else {
            panic!("unchanged caller cause");
        };
        assert_eq!(core::ptr::from_ref(stop.marker.as_ref()), identity);
        assert!(live.load(Ordering::SeqCst) >= ttl.len());
        drop(retained);
        assert_eq!(live.load(Ordering::SeqCst), 0);

        let valid = ttl.replace("a:s a:p .", "a:s a:p a:o .");
        let stopped = parse_dataset_controlled(
            valid.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            Resident,
            |_| Err("pre-tripped cancellation"),
        )
        .expect_err("pre-tripped caller");
        assert!(matches!(
            stopped.cause(),
            ControlledError::Control("pre-tripped cancellation")
        ));
        struct LimitedGrant {
            live: Arc<AtomicUsize>,
            limit: usize,
        }
        impl Admission for LimitedGrant {
            fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
                if bytes > self.limit {
                    return Err(StorageError::AdmissionFailed);
                }
                self.live.store(bytes, Ordering::SeqCst);
                Ok(())
            }
        }
        let refused = parse_dataset_controlled(
            valid.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            LimitedGrant {
                live: Arc::clone(&live),
                limit: valid.len(),
            },
            |_| Ok::<_, ()>(()),
        )
        .expect_err("source fits but native parser storage does not");
        assert!(matches!(
            refused.cause(),
            ControlledError::Native(NativeParseError::Storage(StorageError::AdmissionFailed))
        ));
        drop(refused);
        assert_eq!(live.load(Ordering::SeqCst), 0);

        let retained = parse_dataset_controlled(
            valid.as_bytes(),
            NativeRdfFormat::Turtle,
            None,
            Grant {
                live: Arc::clone(&live),
                refuse: Arc::new(AtomicBool::new(false)),
                source_len: usize::MAX,
            },
            |_| Ok::<_, Stop>(()),
        )
        .expect("neighboring valid source under the original account");
        assert!(retained.dataset().is_some());
        assert!(live.load(Ordering::SeqCst) > 0);
        drop(retained);
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }

    /// RDF/XML reports its `xmlns` declarations as the XML parser scoped them. The
    /// control is the same document with `evil` really declared: only that row
    /// differs. Text that merely spells a declaration — in element content, or in the
    /// content of a `parseType="Literal"` value — declares nothing.
    #[test]
    fn rdfxml_document_prefixes_are_the_xml_parsers_declarations() {
        let document = |root_extra: &str| {
            format!(
                "<?xml version=\"1.0\"?>\n\
                 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
                 xmlns:ex=\"http://example.org/first#\" xmlns=\"http://example.org/default#\"{root_extra}>\n\
                 <rdf:Description rdf:about=\"http://example.org/s\" xmlns:ex=\"http://example.org/ex#\">\n\
                 <ex:comment> xmlns:evil=\"http://example.org/evil#\" </ex:comment>\n\
                 <ex:lit rdf:parseType=\"Literal\"><b xmlns:inlit=\"http://example.org/inlit#\">x</b></ex:lit>\n\
                 </rdf:Description>\n\
                 </rdf:RDF>\n"
            )
        };
        let prefixes = |text: &str| {
            parse_dataset_with(
                text.as_bytes(),
                "application/rdf+xml",
                None,
                &ParseOptions::default(),
            )
            .expect("parse")
            .document_prefixes
        };
        let base = vec![
            (String::new(), "http://example.org/default#".to_owned()),
            ("ex".to_owned(), "http://example.org/ex#".to_owned()),
            (
                "rdf".to_owned(),
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_owned(),
            ),
        ];
        assert_eq!(prefixes(&document("")), base);
        let mut control = base;
        control.insert(
            1,
            ("evil".to_owned(), "http://example.org/evil#".to_owned()),
        );
        assert_eq!(
            prefixes(&document(" xmlns:evil=\"http://example.org/evil#\"")),
            control
        );
        // `xmlns=""` as the last word on the default namespace undeclares it.
        let undeclared = document("").replace(
            "rdf:about=\"http://example.org/s\"",
            "rdf:about=\"http://example.org/s\" xmlns=\"\"",
        );
        assert!(
            !prefixes(&undeclared)
                .iter()
                .any(|(prefix, _)| prefix.is_empty())
        );
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The GTS-graph interner against its recursive reference, and at a hundred thousand
    //! levels on a 128 KiB thread.

    use purrdf_core::{Nested, RdfDatasetBuilder, TermId};

    use super::super::ser_model::SerGraph;
    use super::super::ser_model::term_walk_tests::lower;
    use super::{BlankIngress, NativeParseError, SerInterner};
    use purrdf_lex::allocation::{Memory, Resident};
    use std::cell::RefCell;

    /// The recursive reference of [`SerInterner::intern`].
    fn reference_intern(
        interner: &SerInterner<'_, '_, '_, '_, Resident>,
        builder: &mut RdfDatasetBuilder,
        id: usize,
    ) -> Result<TermId, NativeParseError> {
        match interner.resolve_node(builder, id)? {
            Nested::Leaf(id) => Ok(id),
            Nested::Triple(s, p, o) => {
                let s = reference_intern(interner, builder, s)?;
                let p = reference_intern(interner, builder, p)?;
                let o = reference_intern(interner, builder, o)?;
                Ok(builder.intern_triple(s, p, o).unwrap())
            }
        }
    }

    /// The work-list interner interns every generated term into a fresh builder in the
    /// order the recursive reference does: the same id, the same number of terms.
    #[test]
    fn the_interner_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::IriPredicates,
            );
            nested += usize::from(budget < 7);
            let mut graph = SerGraph::default();
            let root = lower(&mut graph, &value);
            let mut mapping_storage = Resident;
            let mut memory = Memory::new(&mut mapping_storage);
            let mapping_memory = RefCell::new(&mut memory);
            let interner = SerInterner {
                source: RefCell::new(None),
                memory: &mapping_memory,
                graph: &graph,
                blanks: BlankIngress::Opaque,
            };
            let (mut interned, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            let id = interner
                .intern(&mut interned, root)
                .expect("a lowered term interns");
            let reference = reference_intern(&interner, &mut expected, root).expect("interns");
            assert_eq!(id, reference, "seed {seed}");
            // Ids are dense, so one more term's id counts the terms before it.
            let sentinel = "http://example.org/sentinel";
            assert_eq!(
                interned.intern_iri(sentinel).unwrap(),
                expected.intern_iri(sentinel).unwrap(),
                "seed {seed}"
            );
        }
        assert!(nested > 0, "some generated term nests a triple term in one");
    }

    /// A triple term a hundred thousand levels deep is interned on a thread whose whole
    /// stack is 128 KiB: one term per level beside the chain's three distinct leaves.
    #[test]
    fn a_hundred_thousand_level_term_interns_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = purrdf_core::term_fixture::triple_chain(LEVELS);
            let mut graph = SerGraph::default();
            let root = lower(&mut graph, &value);
            drop(value);
            let mut mapping_storage = Resident;
            let mut memory = Memory::new(&mut mapping_storage);
            let mapping_memory = RefCell::new(&mut memory);
            let interner = SerInterner {
                source: RefCell::new(None),
                memory: &mapping_memory,
                graph: &graph,
                blanks: BlankIngress::Opaque,
            };
            let mut builder = RdfDatasetBuilder::new();
            interner
                .intern(&mut builder, root)
                .expect("a lowered chain interns");
            // Ids are dense, so one more term's id counts the terms before it.
            assert_eq!(
                builder
                    .intern_iri("http://example.org/sentinel")
                    .unwrap()
                    .index(),
                LEVELS + 3
            );
        })
        .expect("the thread starts");
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF text → frozen [`RdfDataset`] IR ingress.
//!
//! Parses Turtle / TriG / N-Triples / N-Quads through the FIRST-PARTY
//! [`text_parse`](super::text_parse) front-end (RDF/XML through the first-party
//! [`rdfxml`](super::rdfxml) codec) into a first-party in-memory
//! [`SerGraph`](super::ser_model::SerGraph), then walks that graph into a
//! [`RdfDatasetBuilder`] applying the RDF 1.2 statement-layer fold.
//!
//! The fold is factored into [`fold_statement_layer`], a source-agnostic two-pass
//! classifier over `(subject, predicate, object, graph)` rows that BOTH this native
//! path and the legacy `dataset_io::dataset_from_oxigraph_quads` feed — one fold, no
//! drift (the must-pass RDF 1.2 fixture parity is the guard).
//!
//! Base IRI is handled per the plan: Turtle/TriG resolve relative IRIs against the
//! supplied base; RDF/XML threads the base through the first-party
//! [`rdfxml`](super::rdfxml) codec's `ParseContext`; N-Triples/N-Quads require
//! absolute IRIs and ignore the base (N/A by syntax).

use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use super::media_type::{classify, NativeRdfFormat};
use super::ser_model::{SerGraph, SerTermKind};
use super::span::{NoSpans, ParseOptions, SpanCollector, SpanTable};
use super::text_parse::LineParseMode;
use crate::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, RdfTextDirection, TermId,
};

/// The `rdf:reifies` predicate IRI: a triple-term object under this predicate is the
/// RDF 1.2 reifier binding the statement layer folds out of the base quad table.
pub(crate) const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

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

/// One `(subject, predicate, object, graph)` row, source-agnostic over oxigraph quads
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

/// The RDF 1.2 statement-layer fold, shared by the native codec path and the legacy
/// oxigraph-quads path so the two can never drift (the parity fixture is the guard).
///
/// Pass 1 binds reifiers: a row whose predicate is `rdf:reifies` with a triple-term
/// object becomes a `push_reifier(subject, triple)` binding and the subject id is
/// recorded as a reifier. Pass 2 classifies the remaining rows: a reifier subject's
/// other triples are annotations (`push_annotation`), everything else a base quad
/// (`push_quad`). This mirrors `dataset_io.rs:34-65` exactly.
pub(crate) fn fold_statement_layer<I>(
    builder: &mut RdfDatasetBuilder,
    rows: I,
) -> Result<(), RdfDiagnostic>
where
    I: IntoIterator<Item = FoldRow>,
{
    // Pass 1: bind reifiers; collect the rest as pending base/annotation rows.
    let mut reifier_ids: HashSet<TermId> = HashSet::new();
    let mut pending: Vec<(TermId, TermId, TermId, Option<TermId>)> = Vec::new();
    for row in rows {
        let FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        } = row;
        if is_reifies {
            if let FoldNode::Triple { s, p, o } = object {
                let triple_term = builder.intern_triple(s, p, o);
                // Capture the reifier declaration's OWN graph (TriG `GRAPH g { … }`),
                // so `GRAPH ?g { << … >> … }` binds `?g` to it. Turtle / the default
                // graph carry `graph == None`, byte-identical to the old fold.
                builder.push_reifier_in_graph(subject, triple_term, graph);
                reifier_ids.insert(subject);
                continue;
            }
        }
        let object_id = match object {
            FoldNode::Term(id) => id,
            FoldNode::Triple { s, p, o } => builder.intern_triple(s, p, o),
        };
        pending.push((subject, predicate, object_id, graph));
    }

    // Pass 2: a reifier subject's other triples are annotations (carrying their own
    // graph); the rest base quads.
    for (subject, predicate, object, graph) in pending {
        if reifier_ids.contains(&subject) {
            builder.push_annotation_in_graph(subject, predicate, object, graph);
        } else {
            builder.push_quad(subject, predicate, object, graph);
        }
    }
    Ok(())
}

/// Parse RDF text bytes of `media_type` into a frozen [`RdfDataset`].
///
/// Steps: UTF-8 validate (hard-fail `native-codec-utf8`); parse the line/Turtle family
/// (N-Triples, N-Quads, Turtle, TriG) FIRST-PARTY into an in-memory [`SerGraph`] via
/// [`text_parse`](super::text_parse), and RDF/XML through the FIRST-PARTY
/// [`rdfxml`](super::rdfxml) codec (no external purrdf-gts text / RDF-XML codec); then
/// walk that graph through [`fold_statement_layer`] and freeze.
pub fn parse_dataset(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::Auto)
}

/// [`parse_dataset`] with a runtime option to also return a source-position side table.
///
/// The frozen [`RdfDataset`] is IDENTICAL to what [`parse_dataset`] returns — the same
/// triples, term ids, and order. When [`ParseOptions::track_source_spans`] is set, the
/// second element is a populated [`SpanTable`] mapping each statement subject (by its
/// bare-IRI / `_:label` lexical key) to the source [`Position`](purrdf_iri::Position)
/// where it was first asserted; otherwise it is `None`.
///
/// Span tracking is OPT-IN: it costs memory and pins the sequential line pipeline
/// (the chunk-parallel N-Triples/N-Quads path never records spans), so
/// [`parse_dataset`] — the hot path — is left untouched. Formats without a text
/// tokenizer that carries source spans (RDF/XML, TriX, HexTuples) return an EMPTY
/// [`SpanTable`] under tracking (physical-location fallback is by design).
pub fn parse_dataset_with(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
    options: &ParseOptions,
) -> Result<(Arc<RdfDataset>, Option<SpanTable>), RdfDiagnostic> {
    if !options.track_source_spans {
        let dataset = parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::Auto)?;
        return Ok((dataset, None));
    }

    let format = classify(media_type)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|e| RdfDiagnostic::error("native-codec-utf8", e.to_string()))?;

    match format {
        NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads
        | NativeRdfFormat::Turtle
        | NativeRdfFormat::TriG => {
            // Force sequential so subject spans are captured (the parallel path is
            // `NoSpans`-only), then fold into the SAME frozen IR `parse_dataset` builds.
            let mut table = SpanTable::default();
            let graph = text_parse_without_panicking(
                format,
                text,
                base_iri,
                LineParseMode::ForceSequential,
                &mut table,
            )?;
            let dataset = dataset_from_ser_graph(&graph)?;
            Ok((dataset, Some(table)))
        }
        // RDF/XML, TriX, HexTuples: no span-carrying text tokenizer, so return an empty
        // table alongside the identical dataset (physical-location fallback by design).
        NativeRdfFormat::RdfXml | NativeRdfFormat::TriX | NativeRdfFormat::HexTuples => {
            let dataset = parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::Auto)?;
            Ok((dataset, Some(SpanTable::default())))
        }
    }
}

/// [`parse_dataset`] forcing the single-threaded N-Triples/N-Quads line pipeline
/// regardless of input size (Turtle/TriG/RDF-XML are always sequential, so the mode
/// is a no-op there).
///
/// Bench/test-only surface: the criterion bench and the determinism-proof tests use
/// it as the baseline the chunk-parallel path must match byte-for-byte. NOT public
/// API — hidden, unstable, and free to disappear.
#[doc(hidden)]
pub fn parse_dataset_forced_sequential(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    parse_dataset_mode(bytes, media_type, base_iri, LineParseMode::ForceSequential)
}

fn parse_dataset_mode(
    bytes: &[u8],
    media_type: &str,
    base_iri: Option<&str>,
    mode: LineParseMode,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let format = classify(media_type)?;
    let text = std::str::from_utf8(bytes)
        .map_err(|e| RdfDiagnostic::error("native-codec-utf8", e.to_string()))?;

    match format {
        // The line/Turtle family parses FIRST-PARTY straight into the first-party
        // in-memory SerGraph the statement-layer fold consumes — no purrdf-gts text
        // codec, no text→bytes→reader indirection. This also decodes `\uXXXX` UCHAR
        // escapes inside IRIREFs (W3C test060), which the purrdf-gts IRIREF readers
        // rejected. N-Triples/N-Quads above the `text_parse` size threshold tokenize
        // their line-aligned chunks in PARALLEL (phase 1) and re-join in document
        // order before interning (phase 2), so the frozen IR — term ids, quad order,
        // diagnostics — is byte-identical to the sequential pipeline. Turtle/TriG stay
        // sequential (stateful `@prefix`/`@base` + document-ordered bnode minting; see
        // `text_parse::parse_to_gts_graph_mode`).
        NativeRdfFormat::NTriples
        | NativeRdfFormat::NQuads
        | NativeRdfFormat::Turtle
        | NativeRdfFormat::TriG => {
            let graph =
                text_parse_without_panicking(format, text, base_iri, mode, &mut NoSpans)?;
            dataset_from_ser_graph(&graph)
        }
        // RDF/XML parses FIRST-PARTY through the in-repo `rdfxml` codec (W3C RDF/XML
        // grammar over a pure-Rust XML DOM), which interns straight into the frozen IR
        // through the SAME shared statement-layer fold — no intermediate GTS graph.
        NativeRdfFormat::RdfXml => parse_rdfxml_without_panicking(text, base_iri),
        // TriX / HexTuples parse FIRST-PARTY through their in-repo codecs (XML DOM /
        // NDJSON), interning straight into the frozen IR through the SAME shared
        // statement-layer fold.
        NativeRdfFormat::TriX => catch_codec_panic(NativeRdfFormat::TriX, || {
            super::trix::parse_trix_to_dataset(text)
        }),
        NativeRdfFormat::HexTuples => catch_codec_panic(NativeRdfFormat::HexTuples, || {
            super::hextuples::parse_hextuples_to_dataset(text)
        }),
    }
}

/// Run a first-party codec parse under a panic guard, converting any unwind into a
/// structured `native-codec-panic` diagnostic (mirrors the RDF/XML guard).
fn catch_codec_panic<F>(format: NativeRdfFormat, parse: F) -> Result<Arc<RdfDataset>, RdfDiagnostic>
where
    F: FnOnce() -> Result<Arc<RdfDataset>, RdfDiagnostic>,
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

fn text_parse_without_panicking<S: SpanCollector>(
    format: NativeRdfFormat,
    text: &str,
    base_iri: Option<&str>,
    mode: LineParseMode,
    collector: &mut S,
) -> Result<SerGraph, RdfDiagnostic> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::text_parse::parse_to_gts_graph_mode(format, text, base_iri, mode, collector)
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

fn parse_rdfxml_without_panicking(
    text: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::rdfxml::parse_rdfxml_to_dataset(text, base_iri)
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

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
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
/// reifiers. To feed the SAME two-pass fold the oxigraph path uses — and reach the SAME
/// IR — this re-materializes each reifier binding as a synthetic
/// `<reifier> rdf:reifies <<( s p o )>>` row and each annotation as a
/// `<reifier> <predicate> <value>` row alongside the plain quads, so pass 1 re-binds
/// reifiers and pass 2 classifies the reifier subjects' rows as annotations. Term
/// interning is shared across all rows, so identical terms collapse to one id exactly as
/// on the oxigraph path.
pub(crate) fn dataset_from_ser_graph(graph: &SerGraph) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    dataset_from_ser_graph_impl(graph, false)
}

/// Like [`dataset_from_ser_graph`], but folds **every** named graph into the default
/// graph (drops each base quad's graph component) — the oxigraph-free twin of
/// `store_from_dataset(.., GraphPolicy::FlattenToDefaultGraph)`. This is the load
/// path the native conformance gate replays against the frozen oxigraph goldens
/// (which were captured over a flattened store). The statement layer (`rdf:reifies`
/// reifiers + annotations) has no graph dimension, so only the base-quad graph
/// component changes.
pub(crate) fn flattened_dataset_from_ser_graph(
    graph: &SerGraph,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    dataset_from_ser_graph_impl(graph, true)
}

fn dataset_from_ser_graph_impl(
    graph: &SerGraph,
    flatten_to_default_graph: bool,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut builder = RdfDatasetBuilder::new();
    let interner = SerInterner { graph };

    let mut rows: Vec<FoldRow> =
        Vec::with_capacity(graph.quads.len() + graph.reifiers.len() + graph.annotations.len());

    // Synthetic `rdf:reifies` rows reconstructed from the GTS reifier table, so the
    // shared fold re-binds them identically to the oxigraph path (pass 1). A
    // self-reifier sentinel — a `Triple` term whose `reifier` is its OWN id — is the
    // binding of an inline quoted-triple term used as a quad object, NOT a statement-
    // layer reifier; it carries no `<reifier> rdf:reifies <<…>>` row (the N-Quads
    // serializer skips it identically) and is resolved when its parent quad interns the
    // object. Emitting a synthetic row for it would make a quoted triple the subject of
    // `rdf:reifies`, which the IR rejects.
    for &(reifier_id, (s, p, o), reifier_graph) in &graph.reifiers {
        if graph.terms.get(reifier_id).is_some_and(|term| {
            term.kind == SerTermKind::Triple && term.reifier == Some(reifier_id)
        }) {
            continue;
        }
        let subject = interner.intern(&mut builder, reifier_id)?;
        let predicate = builder.intern_iri(RDF_REIFIES);
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
        rows.push(FoldRow {
            subject,
            is_reifies: true,
            predicate,
            object: FoldNode::Triple { s, p, o },
            graph,
        });
    }

    // Base quad rows.
    for &(s, p, o, g) in &graph.quads {
        let subject = interner.intern(&mut builder, s)?;
        let is_reifies = interner.is_iri(p, RDF_REIFIES)?;
        let predicate = interner.intern(&mut builder, p)?;
        let object = interner.intern_node(&mut builder, o)?;
        let graph = match g {
            Some(_) if flatten_to_default_graph => None,
            Some(g) => Some(interner.intern(&mut builder, g)?),
            None => None,
        };
        rows.push(FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        });
    }

    // Annotation rows the codec already classified into the GTS `annotations` table.
    // Re-materialized as `<reifier> <predicate> <value>` rows so the shared fold's pass 2
    // classifies them as annotations (the reifier subject is bound above). This is the
    // GENERAL RDF-1.2 parser, so it must accept arbitrary input including the W3C suite's
    // graph-scoped (TriG) reification — the optional graph slot is THREADED through (so a
    // reifier/annotation inside `GRAPH g { … }` is matchable under `GRAPH ?g`), unless
    // this pass is flattening every named graph to the default graph.
    for &(reifier_id, predicate_id, value_id, annotation_graph) in &graph.annotations {
        let subject = interner.intern(&mut builder, reifier_id)?;
        let is_reifies = interner.is_iri(predicate_id, RDF_REIFIES)?;
        let predicate = interner.intern(&mut builder, predicate_id)?;
        let object = interner.intern_node(&mut builder, value_id)?;
        let graph = match annotation_graph {
            Some(_) if flatten_to_default_graph => None,
            Some(g) => Some(interner.intern(&mut builder, g)?),
            None => None,
        };
        rows.push(FoldRow {
            subject,
            is_reifies,
            predicate,
            object,
            graph,
        });
    }

    fold_statement_layer(&mut builder, rows)?;
    builder.freeze()
}

/// Intern [`SerGraph`] terms into a builder, resolving quoted-triple terms through their
/// reifier binding. Every folded blank node lands in [`BlankScope::DEFAULT`] (the
/// folded graph has already collapsed per-segment scope — the documented
/// `bnode-scope-flatten` loss, identical to `import_gts_graph`).
struct SerInterner<'a> {
    graph: &'a SerGraph,
}

impl SerInterner<'_> {
    /// Intern a term id into the builder, returning its [`TermId`]. A quoted-triple
    /// term resolves its `(s, p, o)` through the reifier table and interns as a triple.
    fn intern(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
    ) -> Result<TermId, RdfDiagnostic> {
        match self.intern_node(builder, gts_id)? {
            FoldNode::Term(id) => Ok(id),
            FoldNode::Triple { s, p, o } => Ok(builder.intern_triple(s, p, o)),
        }
    }

    /// Intern a GTS term id, returning a [`FoldNode`]: a leaf becomes `Term`, a
    /// quoted-triple term becomes `Triple` (its components already interned) so a
    /// caller can fold it as a reifier binding rather than re-interning it.
    fn intern_node(
        &self,
        builder: &mut RdfDatasetBuilder,
        gts_id: usize,
    ) -> Result<FoldNode, RdfDiagnostic> {
        let term = self.graph.terms.get(gts_id).ok_or_else(|| {
            RdfDiagnostic::error(
                "native-codec-term-out-of-range",
                format!("GTS term id {gts_id} is out of range"),
            )
        })?;
        match term.kind {
            SerTermKind::Iri => {
                let iri = term
                    .value
                    .as_deref()
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        RdfDiagnostic::error(
                            "native-codec-iri-missing-value",
                            "GTS IRI term requires a non-empty value",
                        )
                    })?;
                Ok(FoldNode::Term(builder.intern_iri(iri)))
            }
            SerTermKind::Bnode => {
                let label = term
                    .value
                    .clone()
                    .unwrap_or_else(|| format!("gts_bnode_{gts_id}"));
                Ok(FoldNode::Term(
                    builder.intern_blank(&label, BlankScope::DEFAULT),
                ))
            }
            SerTermKind::Literal => {
                let datatype = match term.datatype {
                    Some(dt_id) => Some(self.iri_string(dt_id)?),
                    None => None,
                };
                let direction =
                    parse_gts_direction(term.direction.as_deref(), term.lang.as_deref())?;
                Ok(FoldNode::Term(builder.intern_literal(RdfLiteral {
                    lexical_form: term.value.clone().unwrap_or_default(),
                    datatype,
                    language: term.lang.clone(),
                    direction,
                })))
            }
            SerTermKind::Triple => {
                let reifier_id = term.reifier.ok_or_else(|| {
                    RdfDiagnostic::error(
                        "native-codec-unbound-triple-term",
                        "GTS triple term has no reifier binding",
                    )
                })?;
                let (s, p, o) = self.graph.reifier(reifier_id).ok_or_else(|| {
                    RdfDiagnostic::error(
                        "native-codec-missing-reifier-binding",
                        format!("GTS triple term references missing reifier {reifier_id}"),
                    )
                })?;
                let s = self.intern(builder, s)?;
                let p = self.intern(builder, p)?;
                let o = self.intern(builder, o)?;
                Ok(FoldNode::Triple { s, p, o })
            }
        }
    }

    /// Intern a GTS term id known to occupy an IRI position (predicate / datatype),
    /// returning its IRI string for the `rdf:reifies` check and literal datatype.
    fn iri_string(&self, gts_id: usize) -> Result<String, RdfDiagnostic> {
        let term = self.graph.terms.get(gts_id).ok_or_else(|| {
            RdfDiagnostic::error(
                "native-codec-term-out-of-range",
                format!("GTS term id {gts_id} is out of range"),
            )
        })?;
        match term.kind {
            SerTermKind::Iri => term.value.clone().filter(|v| !v.is_empty()).ok_or_else(|| {
                RdfDiagnostic::error(
                    "native-codec-iri-missing-value",
                    "GTS IRI term requires a non-empty value",
                )
            }),
            other => Err(RdfDiagnostic::error(
                "native-codec-predicate-not-iri",
                format!("GTS term in an IRI position must be an IRI, got {other:?}"),
            )),
        }
    }

    /// Return whether a GTS term id is a specific IRI, failing if the term is not valid
    /// in an IRI position.
    fn is_iri(&self, gts_id: usize, expected: &str) -> Result<bool, RdfDiagnostic> {
        let term = self.graph.terms.get(gts_id).ok_or_else(|| {
            RdfDiagnostic::error(
                "native-codec-term-out-of-range",
                format!("GTS term id {gts_id} is out of range"),
            )
        })?;
        match term.kind {
            SerTermKind::Iri => term
                .value
                .as_deref()
                .filter(|v| !v.is_empty())
                .map(|value| value == expected)
                .ok_or_else(|| {
                    RdfDiagnostic::error(
                        "native-codec-iri-missing-value",
                        "GTS IRI term requires a non-empty value",
                    )
                }),
            other => Err(RdfDiagnostic::error(
                "native-codec-predicate-not-iri",
                format!("GTS term in an IRI position must be an IRI, got {other:?}"),
            )),
        }
    }
}

/// Parse a GTS literal base-direction string (`"ltr"`/`"rtl"`) into the IR's
/// [`RdfTextDirection`], mirroring `purrdf_core`'s `parse_gts_direction`: `None` is
/// legitimate absence, an unrecognized non-empty value hard-fails, and RDF 1.2 admits
/// a direction ONLY on a language-tagged string (a direction without a language is a
/// hard error rather than a silently ill-formed literal).
fn parse_gts_direction(
    value: Option<&str>,
    language: Option<&str>,
) -> Result<Option<RdfTextDirection>, RdfDiagnostic> {
    let direction = match value {
        None => return Ok(None),
        Some("ltr") => RdfTextDirection::Ltr,
        Some("rtl") => RdfTextDirection::Rtl,
        Some(other) => {
            return Err(RdfDiagnostic::error(
                "native-codec-invalid-direction",
                format!("unrecognized GTS literal base direction {other:?}"),
            ))
        }
    };
    if language.is_none_or(str::is_empty) {
        return Err(RdfDiagnostic::error(
            "native-codec-direction-without-language",
            "an RDF 1.2 literal base direction requires a non-empty language tag",
        ));
    }
    Ok(Some(direction))
}

#[cfg(test)]
mod tests {
    use super::*;
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
        // The IR expands a language-tagged literal's datatype to rdf:langString (C0.1)
        // and keeps the base direction in the literal identity key (NOT dirLangString).
        assert!(
            ds.term_id_by_value(&TermValue::Literal {
                lexical_form: "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
            .is_some(),
            "an rtl directional literal must survive the parse"
        );
    }

    #[test]
    fn invalid_utf8_hard_fails() {
        let err =
            parse_dataset(&[0xff, 0xfe], "text/turtle", None).expect_err("invalid utf-8 must fail");
        assert_eq!(err.code, "native-codec-utf8");
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
        let (with_ds, table) =
            parse_dataset_with(nt.as_bytes(), "application/n-triples", None, &ParseOptions::default())
                .expect("parse");
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
        assert_eq!(a, b, "dataset is identical whether or not tracking is requested");
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
        };
        let (_ds, table) =
            parse_dataset_with(nt.as_bytes(), "application/n-triples", None, &options)
                .expect("parse");
        let table = table.expect("tracking on yields a table");
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
    fn turtle_tracking_records_subject_line() {
        // The subject `ex:s` appears on line 3 (after two directive lines).
        let ttl = concat!(
            "@prefix ex: <http://example.org/> .\n",
            "\n",
            "ex:s ex:p ex:o .\n",
        );
        let options = ParseOptions {
            track_source_spans: true,
        };
        let (_ds, table) =
            parse_dataset_with(ttl.as_bytes(), "text/turtle", None, &options).expect("parse");
        let table = table.expect("tracking on yields a table");
        let position = table
            .position_for_subject("http://example.org/s")
            .expect("subject tracked");
        assert_eq!(position.line, 3, "subject ex:s is asserted on line 3");
    }

    #[test]
    fn dataset_is_identical_with_tracking() {
        // The dataset from a tracking parse has the same triples as `parse_dataset`.
        let nt = concat!(
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
            "<http://example.org/s> <http://example.org/p2> \"lit\" .\n",
        );
        let options = ParseOptions {
            track_source_spans: true,
        };
        let (tracked, _table) =
            parse_dataset_with(nt.as_bytes(), "application/n-triples", None, &options)
                .expect("tracked parse");
        let plain = parse_dataset(nt.as_bytes(), "application/n-triples", None).expect("plain");
        assert_eq!(tracked.quad_count(), plain.quad_count());
        assert!(
            tracked.quads().collect::<Vec<_>>() == plain.quads().collect::<Vec<_>>(),
            "tracked dataset has identical quads"
        );
    }
}

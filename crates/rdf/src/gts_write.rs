// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Write a frozen [`RdfDataset`] into a deterministic GTS byte stream.
//!
//! This materialises the concrete IR into a [`purrdf_gts::model::Graph`] and asks
//! [`purrdf_gts::writer::Writer`] to canonicalise it. All interning, term remapping,
//! and frame authoring is delegated to `purrdf-gts`.
//!
//! The writer consumes the IR directly (purrdf P2c part 1): it reads the
//! frozen dataset's quad/reifier/annotation tables and resolves each row to the
//! owned model at the boundary, then interns into the GTS term table. Out-of-band material
//! (GTS metadata, suppressions) is passed in explicitly as an [`RdfLookaside`]
//! (C0.6: it lives in the bundle envelope, not the hot graph).

use std::collections::HashMap;

use ciborium::value::Value;
use purrdf_gts::codec::CodecError;
use purrdf_gts::model::{Graph, Suppression, Term, TermKind, Triple3};
use purrdf_gts::writer::Writer;

use crate::ir::RdfDataset;
use crate::{
    RdfAnnotation, RdfDiagnostic, RdfLiteral, RdfLookaside, RdfMetadataValue, RdfQuad, RdfReifier,
    RdfTerm,
};

const MAX_TERM_NESTING_DEPTH: usize = 16;

/// Convert a frozen [`RdfDataset`] into a canonical GTS [`Writer`].
///
/// `lookaside` carries the out-of-band envelope material (GTS metadata,
/// suppressions) to fold alongside the hot graph; pass [`RdfLookaside::default`]
/// for a bare dataset with no envelope. `profile` is passed through to the GTS
/// header (e.g. `"purrdf"`). The resulting writer can be further configured
/// (signing, indexes) or emitted directly with [`Writer::to_bytes`].
pub fn to_writer(
    dataset: &RdfDataset,
    lookaside: &RdfLookaside,
    profile: &str,
) -> Result<Writer, RdfDiagnostic> {
    let mut graph = Graph::default();

    // First pass: resolve the frozen IR tables to the owned model in stable
    // frozen order so terms intern deterministically and reifier bindings are
    // resolved before quad terms reference them. The resolution is infallible
    // (the dataset is already validated at freeze) and reuses the same
    // owned-boundary helpers the IR exposes for legacy consumers.
    let quads: Vec<RdfQuad> = dataset.owned_quads().collect();
    let reifiers: Vec<RdfReifier> = dataset.owned_reifiers().collect();
    let annotations: Vec<RdfAnnotation> = dataset.owned_annotations().collect();

    let mut state = InternState::new();

    // Explicit reifiers take precedence over auto-generated blank-node
    // reifiers for the same triple content.
    for reifier in &reifiers {
        bind_explicit_reifier(&mut state, reifier)?;
    }

    for quad in &quads {
        let s = intern_term(&mut state, &quad.subject)?;
        let p = intern_iri(&mut state, &quad.predicate)?;
        let o = intern_term(&mut state, &quad.object)?;
        let g = quad
            .graph_name
            .as_ref()
            .map(|g| intern_graph_name(&mut state, g))
            .transpose()?;
        graph.quads.push((s, p, o, g));
    }

    for reifier in &reifiers {
        let rid = intern_term(&mut state, &reifier.reifier)?;
        let s = intern_term(&mut state, &reifier.statement.subject)?;
        let p = intern_iri(&mut state, &reifier.statement.predicate)?;
        let o = intern_term(&mut state, &reifier.statement.object)?;
        // purrdf-gts row-array `(rid, (s,p,o), graph?)`: the graph slot records the
        // named graph the reifier was declared in (`None` = default graph), so a
        // reifier inside a TriG `GRAPH g { … }` block round-trips through GTS.
        let g = reifier
            .graph
            .as_ref()
            .map(|g| intern_graph_name(&mut state, g))
            .transpose()?;
        graph.reifiers.push((rid, (s, p, o), g));
    }

    for annotation in &annotations {
        let r = intern_term(&mut state, &annotation.reifier)?;
        let p = intern_iri(&mut state, &annotation.predicate)?;
        let v = intern_term(&mut state, &annotation.object)?;
        // Row-array `(reifier, predicate, value, graph?)`; the graph slot mirrors the
        // reifier row above.
        let g = annotation
            .graph
            .as_ref()
            .map(|g| intern_graph_name(&mut state, g))
            .transpose()?;
        graph.annotations.push((r, p, v, g));
    }

    apply_lookaside(&state, &mut graph, lookaside.clone());
    graph.terms = state.terms;

    Writer::deterministic(&graph, profile).map_err(|err| codec_error_to_diagnostic(&err))
}

/// Convert a frozen [`RdfDataset`] directly into canonical GTS bytes. See
/// [`to_writer`] for the `lookaside` envelope contract.
pub fn to_gts(
    dataset: &RdfDataset,
    lookaside: &RdfLookaside,
    profile: &str,
) -> Result<Vec<u8>, RdfDiagnostic> {
    to_writer(dataset, lookaside, profile).map(Writer::into_bytes)
}

struct InternState {
    terms: Vec<Term>,
    index: HashMap<RdfTerm, usize>,
    /// Triple component ids → reifier term ids. RDF 1.2 permits several distinct
    /// explicit reifiers for one (s,p,o); they are all retained.
    /// A nested triple term reuses the first-bound reifier for its single
    /// `Term.reifier` slot. Reifiers are IRI/blank-node terms already in `terms`.
    reifier_map: HashMap<Triple3, Vec<usize>>,
}

impl InternState {
    fn new() -> Self {
        Self {
            terms: Vec::new(),
            index: HashMap::new(),
            reifier_map: HashMap::new(),
        }
    }
}

fn bind_explicit_reifier(
    state: &mut InternState,
    reifier: &RdfReifier,
) -> Result<(), RdfDiagnostic> {
    let rid = intern_term(state, &reifier.reifier)?;
    if !is_iri_or_bnode(&state.terms[rid]) {
        return Err(RdfDiagnostic::error(
            "rdf-reifier-not-node",
            "RDF 1.2 reifier must be an IRI or blank node",
        ));
    }
    let s = intern_term(state, &reifier.statement.subject)?;
    let p = intern_iri(state, &reifier.statement.predicate)?;
    let o = intern_term(state, &reifier.statement.object)?;

    // RDF 1.2 allows several distinct explicit reifiers for the same triple
    // content; record each one, deduplicating only an identical
    // (rid, (s,p,o)) pair. Every distinct reifier is emitted as its own
    // `graph.reifiers` row below, so no binding is collapsed.
    let bound = state.reifier_map.entry((s, p, o)).or_default();
    if !bound.contains(&rid) {
        bound.push(rid);
    }

    Ok(())
}

fn intern_iri(state: &mut InternState, iri: &str) -> Result<usize, RdfDiagnostic> {
    intern_term(state, &RdfTerm::Iri(iri.to_owned()))
}

fn intern_graph_name(state: &mut InternState, term: &RdfTerm) -> Result<usize, RdfDiagnostic> {
    if !is_iri_or_bnode_term(term) {
        return Err(RdfDiagnostic::error(
            "rdf-graph-name-not-node",
            format!(
                "named graph name must be an IRI or blank node, got {:?}",
                term.kind()
            ),
        ));
    }
    intern_term(state, term)
}

fn intern_term(state: &mut InternState, term: &RdfTerm) -> Result<usize, RdfDiagnostic> {
    intern_term_depth(state, term, 0)
}

fn intern_term_depth(
    state: &mut InternState,
    term: &RdfTerm,
    depth: usize,
) -> Result<usize, RdfDiagnostic> {
    if depth > MAX_TERM_NESTING_DEPTH {
        return Err(RdfDiagnostic::error(
            "rdf-term-nesting-limit",
            "RDF term nesting depth limit exceeded while building GTS graph",
        ));
    }
    if let Some(id) = state.index.get(term) {
        return Ok(*id);
    }

    match term {
        RdfTerm::Iri(iri) => {
            let id = push_term(
                state,
                term,
                Term {
                    kind: TermKind::Iri,
                    value: Some(iri.clone()),
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: None,
                },
            );
            Ok(id)
        }
        RdfTerm::BlankNode(label) => {
            let id = push_term(
                state,
                term,
                Term {
                    kind: TermKind::Bnode,
                    value: Some(label.clone()),
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: None,
                },
            );
            Ok(id)
        }
        RdfTerm::Literal(literal) => intern_literal(state, literal, depth),
        RdfTerm::Triple(triple) => intern_triple_term(state, triple, depth),
    }
}

fn intern_literal(
    state: &mut InternState,
    literal: &RdfLiteral,
    depth: usize,
) -> Result<usize, RdfDiagnostic> {
    let datatype = if let Some(dt) = &literal.datatype {
        Some(intern_term_depth(
            state,
            &RdfTerm::Iri(dt.clone()),
            depth + 1,
        )?)
    } else {
        None
    };

    // RDF 1.2 literal base direction now round-trips through GTS:
    // map the IR's RdfTextDirection onto the GTS Term.direction string. Lexical
    // form, datatype, language tag, and direction are all preserved.
    let lang = literal.language.clone();
    let direction = literal.direction.map(|d| d.as_str().to_string());

    let id = push_term(
        state,
        &RdfTerm::Literal(literal.clone()),
        Term {
            kind: TermKind::Literal,
            value: Some(literal.lexical_form.clone()),
            datatype,
            lang,
            direction,
            reifier: None,
        },
    );
    Ok(id)
}

fn intern_triple_term(
    state: &mut InternState,
    triple: &crate::RdfTriple,
    depth: usize,
) -> Result<usize, RdfDiagnostic> {
    let s = intern_term_depth(state, &triple.subject, depth + 1)?;
    let p = intern_iri(state, &triple.predicate)?;
    let o = intern_term_depth(state, &triple.object, depth + 1)?;

    let reifier_id = if let Some(rid) = state
        .reifier_map
        .get(&(s, p, o))
        .and_then(|rids| rids.first())
        .copied()
    {
        rid
    } else {
        let rid = create_anonymous_reifier(state);
        state.reifier_map.entry((s, p, o)).or_default().push(rid);
        rid
    };

    let id = push_term(
        state,
        &RdfTerm::Triple(Box::new(triple.clone())),
        Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(reifier_id),
        },
    );
    Ok(id)
}

fn create_anonymous_reifier(state: &mut InternState) -> usize {
    let label = format!("purrdf_auto_{}", state.terms.len());
    let id = state.terms.len();
    state.terms.push(Term {
        kind: TermKind::Bnode,
        value: Some(label.clone()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    });
    state.index.insert(RdfTerm::BlankNode(label), id);
    id
}

fn push_term(state: &mut InternState, key: &RdfTerm, term: Term) -> usize {
    let id = state.terms.len();
    state.terms.push(term);
    state.index.insert(key.clone(), id);
    id
}

fn is_iri_or_bnode(term: &Term) -> bool {
    matches!(term.kind, TermKind::Iri | TermKind::Bnode)
}

fn is_iri_or_bnode_term(term: &RdfTerm) -> bool {
    matches!(term, RdfTerm::Iri(_) | RdfTerm::BlankNode(_))
}

fn apply_lookaside(state: &InternState, graph: &mut Graph, lookaside: RdfLookaside) {
    for entry in lookaside.metadata {
        let key = entry.key;
        let value = metadata_value_to_cbor(&entry.value);
        graph.set_meta(key, value);
    }

    for suppression in lookaside.suppressions {
        let by = suppression
            .by
            .as_deref()
            .and_then(|label| term_id_by_display(state, label));
        graph.suppressions.push(Suppression {
            targets: suppression
                .targets
                .iter()
                .map(metadata_value_to_cbor)
                .collect(),
            reason: suppression.reason,
            by,
        });
    }

    // Blobs travel by content-addressed reference, not by value. The RDF IR never
    // holds payload bytes (a blob may be a multi-terabyte data dump), so we cannot
    // re-emit a `BlobEntry` here: `purrdf_gts::model::BlobEntry` is byte-bearing by
    // construction (`Bytes(..)` / `Lazy { raw, .. }`) — it has NO byte-less
    // reference variant that could carry only the `RdfBlobRecord`'s digest + origin.
    // The blob *reference* therefore round-trips out of band (the loss ledger's
    // `blob-bytes-absent` entry, intentional), and a future deferred-materialization
    // path streams bytes origin→destination on demand. Carrying the bare reference
    // through `graph.blobs` is blocked until purrdf-gts grows a reference-only blob
    // entry — see `docs/design/819-rdf-ir-dataflow.md` Appendix Z.
    let _ = lookaside.blobs;
}

fn term_id_by_display(state: &InternState, label: &str) -> Option<usize> {
    state
        .terms
        .iter()
        .position(|term| term.value.as_deref() == Some(label) && is_iri_or_bnode(term))
}

fn metadata_value_to_cbor(value: &RdfMetadataValue) -> Value {
    match value {
        RdfMetadataValue::Null => Value::Null,
        RdfMetadataValue::Bool(b) => Value::Bool(*b),
        RdfMetadataValue::Integer(i) => match ciborium::value::Integer::try_from(*i) {
            Ok(integer) => Value::Integer(integer),
            Err(_) => Value::Integer(ciborium::value::Integer::from(if *i < 0 {
                i64::MIN
            } else {
                i64::MAX
            })),
        },
        RdfMetadataValue::Float(f) => Value::Float(*f),
        RdfMetadataValue::Text(t) => Value::Text(t.clone()),
        RdfMetadataValue::Bytes(b) => Value::Bytes(b.clone()),
        RdfMetadataValue::Array(a) => Value::Array(a.iter().map(metadata_value_to_cbor).collect()),
        RdfMetadataValue::Map(m) => Value::Map(
            m.iter()
                .map(|(k, v)| (Value::Text(k.clone()), metadata_value_to_cbor(v)))
                .collect(),
        ),
        RdfMetadataValue::Tagged { tag, value } => {
            Value::Tag(*tag, Box::new(metadata_value_to_cbor(value)))
        }
        RdfMetadataValue::Opaque(s) => Value::Text(s.clone()),
    }
}

fn codec_error_to_diagnostic(err: &CodecError) -> RdfDiagnostic {
    RdfDiagnostic::error("gts-writer-codec", err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::{
        BlankScope, RdfAnnotation, RdfLiteral, RdfMetadataEntry, RdfMetadataValue, RdfQuad,
        RdfReifier, RdfSuppressionRecord, RdfTerm, RdfTextDirection, RdfTriple, TermId,
    };
    use std::sync::Arc;

    /// Recursively intern an owned [`RdfTerm`] into a builder, returning its id —
    /// the test-side inverse of the writer's owned-boundary resolution.
    fn intern_owned(b: &mut RdfDatasetBuilder, term: &RdfTerm) -> TermId {
        match term {
            RdfTerm::Iri(iri) => b.intern_iri(iri),
            RdfTerm::BlankNode(label) => b.intern_blank(label, BlankScope::DEFAULT),
            RdfTerm::Literal(lit) => b.intern_literal(lit.clone()),
            RdfTerm::Triple(t) => {
                let s = intern_owned(b, &t.subject);
                let p = b.intern_iri(&t.predicate);
                let o = intern_owned(b, &t.object);
                b.intern_triple(s, p, o)
            }
        }
    }

    /// Freeze owned rows (quads + RDF 1.2 statement layer) into the frozen IR the
    /// writer consumes. Order is irrelevant: `freeze` dedups and sorts.
    fn freeze_rows(
        quads: &[RdfQuad],
        reifiers: &[RdfReifier],
        annotations: &[RdfAnnotation],
    ) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for q in quads {
            let s = intern_owned(&mut b, &q.subject);
            let p = b.intern_iri(&q.predicate);
            let o = intern_owned(&mut b, &q.object);
            let g = q.graph_name.as_ref().map(|g| intern_owned(&mut b, g));
            b.push_quad(s, p, o, g);
        }
        for r in reifiers {
            let rid = intern_owned(&mut b, &r.reifier);
            let s = intern_owned(&mut b, &r.statement.subject);
            let p = b.intern_iri(&r.statement.predicate);
            let o = intern_owned(&mut b, &r.statement.object);
            let triple = b.intern_triple(s, p, o);
            b.push_reifier(rid, triple);
        }
        for a in annotations {
            let rid = intern_owned(&mut b, &a.reifier);
            let p = b.intern_iri(&a.predicate);
            let o = intern_owned(&mut b, &a.object);
            b.push_annotation(rid, p, o);
        }
        b.freeze().expect("rows must freeze into a valid dataset")
    }

    fn roundtrip(dataset: &RdfDataset, lookaside: &RdfLookaside, profile: &str) -> Graph {
        let bytes = to_gts(dataset, lookaside, profile).expect("to_gts should succeed");
        let graph = purrdf_gts::reader::read(&bytes, false, None);
        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        graph
    }

    /// Re-render the round-tripped GTS graph to N-Quads through the kernel's own IR
    /// importer + RDFC-1.0 canonicalizer — never the purrdf-gts codec (purrdf-gts is the
    /// purrdf.gts container layer only, and rdf-core is the oxigraph-free kernel below
    /// purrdf, so the native `serialize_dataset` is out of reach here). For the
    /// single-quad fixtures below the canonical document is identical to the written
    /// quad line; multi-quad callers get a deterministic bytewise-sorted document.
    fn roundtrip_nquads(dataset: &RdfDataset, profile: &str) -> String {
        let graph = roundtrip(dataset, &RdfLookaside::default(), profile);
        let bundle =
            crate::import_gts_graph(graph).expect("import the round-tripped GTS graph into the IR");
        crate::ir::canonicalize(&bundle.dataset).nquads
    }

    fn assert_nquads_eq(dataset: &RdfDataset, profile: &str, expected: &str) {
        let nquads = roundtrip_nquads(dataset, profile);
        assert_eq!(nquads.trim(), expected.trim());
    }

    #[test]
    fn simple_quad_roundtrips_through_gts() {
        let ds = freeze_rows(
            &[RdfQuad::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                RdfTerm::iri("https://example.org/o"),
            )],
            &[],
            &[],
        );
        assert_nquads_eq(
            &ds,
            "purrdf-test",
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .",
        );
    }

    #[test]
    fn direction_roundtrips_through_gts() {
        // RDF 1.2 directional language-tagged literal: the base
        // direction must survive RDF IR -> GTS -> read. This proves the retired
        // `direction-dropped` loss is genuinely gone, not merely undocumented.
        let mut lit = RdfLiteral::language_tagged("\u{645}\u{631}\u{62d}\u{628}\u{627}", "ar");
        lit.direction = Some(RdfTextDirection::Rtl);
        let ds = freeze_rows(
            &[RdfQuad::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                RdfTerm::literal(lit),
            )],
            &[],
            &[],
        );
        let graph = roundtrip(&ds, &RdfLookaside::default(), "purrdf-test");
        let lit_term = graph
            .terms
            .iter()
            .find(|t| t.kind == TermKind::Literal)
            .expect("literal term present after read");
        assert_eq!(lit_term.direction.as_deref(), Some("rtl"));
        assert_eq!(lit_term.lang.as_deref(), Some("ar"));
    }

    #[test]
    fn named_graph_roundtrips() {
        let quad = RdfQuad::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::literal(RdfLiteral::language_tagged("hello", "en")),
        )
        .in_graph(RdfTerm::iri("https://example.org/g"));
        let ds = freeze_rows(&[quad], &[], &[]);
        assert_nquads_eq(
            &ds,
            "purrdf-test",
            "<https://example.org/s> <https://example.org/p> \"hello\"@en <https://example.org/g> .",
        );
    }

    #[test]
    fn reifiers_and_annotations_roundtrip() {
        let statement = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        );
        let reifier = RdfTerm::blank_node("r1");
        let ds = freeze_rows(
            &[RdfQuad::new(
                reifier.clone(),
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies",
                RdfTerm::triple(statement.clone()),
            )],
            &[RdfReifier::new(reifier.clone(), statement)],
            &[RdfAnnotation::new(
                reifier,
                "https://example.org/confidence",
                RdfTerm::literal(RdfLiteral::typed(
                    "0.9",
                    "http://www.w3.org/2001/XMLSchema#decimal",
                )),
            )],
        );

        let nquads = roundtrip_nquads(&ds, "purrdf-test");
        assert!(nquads.contains("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"));
        assert!(nquads.contains("https://example.org/confidence"));
        assert!(nquads.contains("0.9"));
    }

    #[test]
    fn two_reifiers_same_triple_both_survive() {
        // RDF 1.2 permits several distinct explicit reifiers for one (s,p,o).
        //  lets the writer keep both, so `multi-reifier-collapsed`
        // is no longer a loss: both bindings must survive the round-trip.
        let statement = RdfTriple::new(
            RdfTerm::iri("https://example.org/s"),
            "https://example.org/p",
            RdfTerm::iri("https://example.org/o"),
        );
        let ds = freeze_rows(
            &[],
            &[
                RdfReifier::new(RdfTerm::blank_node("r1"), statement.clone()),
                RdfReifier::new(RdfTerm::blank_node("r2"), statement),
            ],
            &[],
        );
        let graph = roundtrip(&ds, &RdfLookaside::default(), "purrdf-test");
        // Two distinct reifier rows over the same triple content survive.
        assert_eq!(graph.reifiers.len(), 2);
        let rids: std::collections::BTreeSet<usize> =
            graph.reifiers.iter().map(|(rid, _, _)| *rid).collect();
        assert_eq!(rids.len(), 2, "the two reifiers must be distinct");
        let triples: std::collections::BTreeSet<Triple3> =
            graph.reifiers.iter().map(|(_, t, _)| *t).collect();
        assert_eq!(triples.len(), 1, "both reify the same (s,p,o)");
    }

    #[test]
    fn determinism_produces_identical_bytes() {
        let ds = freeze_rows(
            &[
                RdfQuad::new(
                    RdfTerm::iri("https://example.org/s"),
                    "https://example.org/p",
                    RdfTerm::iri("https://example.org/o"),
                ),
                RdfQuad::new(
                    RdfTerm::blank_node("b1"),
                    "https://example.org/p2",
                    RdfTerm::literal(RdfLiteral::simple("literal value")),
                ),
            ],
            &[],
            &[],
        );
        let first = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("first write");
        let second = to_gts(&ds, &RdfLookaside::default(), "purrdf-test").expect("second write");
        assert_eq!(first, second);
    }

    #[test]
    fn lookaside_metadata_and_suppressions_are_preserved() {
        let ds = freeze_rows(
            &[RdfQuad::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                RdfTerm::iri("https://example.org/o"),
            )],
            &[],
            &[],
        );
        let mut lookaside = RdfLookaside::default();
        lookaside.metadata.push(RdfMetadataEntry::new(
            "gts:file",
            "producer",
            RdfMetadataValue::Text("purrdf-test".to_owned()),
        ));
        lookaside.suppressions.push(RdfSuppressionRecord {
            reason: Some("test suppression".to_owned()),
            by: None,
            targets: vec![RdfMetadataValue::Map(
                std::iter::once(("kind".to_owned(), RdfMetadataValue::Text("quad".to_owned())))
                    .collect(),
            )],
        });

        let graph = roundtrip(&ds, &lookaside, "purrdf-test");
        assert_eq!(graph.meta.len(), 1);
        assert_eq!(graph.suppressions.len(), 1);
    }

    #[test]
    fn moderately_nested_triple_term_roundtrips() {
        // The writer now consumes a frozen `RdfDataset`, whose `freeze` already
        // enforces the SAME triple-term nesting bound (validate.rs reuses
        // `MAX_TERM_NESTING_DEPTH`), so the writer's own guard is unreachable from a
        // valid dataset — the depth-limit error path is tested upstream at freeze.
        // Here we prove a legal, moderately nested triple term round-trips intact.
        let mut term = RdfTerm::iri("https://example.org/leaf");
        for _ in 0..4 {
            term = RdfTerm::triple(RdfTriple::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                term,
            ));
        }
        let ds = freeze_rows(
            &[RdfQuad::new(
                RdfTerm::iri("https://example.org/s"),
                "https://example.org/p",
                term,
            )],
            &[],
            &[],
        );
        let graph = roundtrip(&ds, &RdfLookaside::default(), "purrdf-test");
        assert!(graph.terms.iter().any(|t| t.kind == TermKind::Triple));
    }
}

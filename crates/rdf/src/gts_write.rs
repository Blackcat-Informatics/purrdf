// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Write a frozen [`RdfDataset`] into a deterministic GTS byte stream.
//!
//! This materialises the concrete IR into a [`purrdf_gts::model::Graph`] and asks
//! [`purrdf_gts::writer::Writer`] to canonicalise it. All interning, term remapping,
//! and frame authoring is delegated to `purrdf-gts`.
//!
//! The writer consumes native identifiers through the shared snapshot interner,
//! retaining flat component keys for quoted terms. Out-of-band material
//! (GTS metadata, suppressions) is passed in explicitly as an [`RdfLookaside`]
//! (it lives in the bundle envelope, not the hot graph).

use purrdf_gts::codec::CodecError;
use purrdf_gts::model::{Graph, Suppression, TermKind};
use purrdf_gts::writer::Writer;
use purrdf_lex::cbor::Value;

use crate::ir::RdfDataset;
use crate::{RdfDiagnostic, RdfLookaside, RdfMetadataValue};

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
    let mut snapshot = crate::gts_compose::SnapshotBuilder::default();
    snapshot
        .add_dataset(dataset)
        .map_err(ingest_error_to_diagnostic)?;
    let mut graph = snapshot.into_graph().map_err(ingest_error_to_diagnostic)?;
    apply_lookaside(dataset, &mut graph, lookaside.clone())?;

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

fn ingest_error_to_diagnostic(error: crate::gts_compose::GtsIngestError) -> RdfDiagnostic {
    use purrdf_lex::allocation::{Memory, Resident, retain_native_cause};
    let result = (|| {
        let cause = retain_native_cause(error)?;
        RdfDiagnostic::error(
            "gts-writer-ingest",
            "native dataset ingestion into GTS failed",
        )
        .with_native_cause(cause, &mut Memory::new(&mut Resident))
    })();
    result.unwrap_or_else(RdfDiagnostic::from)
}

fn apply_lookaside(
    dataset: &RdfDataset,
    graph: &mut Graph,
    lookaside: RdfLookaside,
) -> Result<(), RdfDiagnostic> {
    for entry in lookaside.metadata {
        let key = entry.key;
        let value = metadata_value_to_cbor(&entry.value);
        graph.set_meta(key, value);
    }

    for suppression in lookaside.suppressions {
        let by = match suppression.by.as_deref() {
            Some(label) => legacy_display_kind(dataset, label)?
                .and_then(|kind| term_id_by_display(graph, label, kind)),
            None => None,
        };
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
    Ok(())
}

fn term_id_by_display(graph: &Graph, label: &str, kind: TermKind) -> Option<usize> {
    graph
        .terms
        .iter()
        .position(|term| term.value.as_deref() == Some(label) && term.kind == kind)
}

/// Preserve the historical untyped suppression label's first interned node:
/// explicit bindings, ordinary rows, binding graphs, then annotations.
fn legacy_display_kind(
    dataset: &RdfDataset,
    label: &str,
) -> Result<Option<TermKind>, RdfDiagnostic> {
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut seen = Vec::new();
    memory.reserve(&mut seen, dataset.term_count())?;
    seen.resize(dataset.term_count(), false);
    let result = (|| {
        let roots = dataset
            .reifier_quads()
            .flat_map(|row| [Some(row.s), Some(row.o)])
            .chain(
                dataset
                    .quads()
                    .flat_map(|row| [Some(row.s), Some(row.p), Some(row.o), row.g]),
            )
            .chain(dataset.reifier_quads().map(|row| row.g))
            .chain(
                dataset
                    .annotation_quads()
                    .flat_map(|row| [Some(row.s), Some(row.p), Some(row.o), row.g]),
            );
        for mut id in roots.flatten() {
            loop {
                if seen[id.index()] {
                    break;
                }
                seen[id.index()] = true;
                match dataset.resolve(id) {
                    crate::TermRef::Iri(iri) => {
                        if iri == label {
                            return Some(TermKind::Iri);
                        }
                        break;
                    }
                    crate::TermRef::Blank {
                        label: blank,
                        scope,
                    } => {
                        if scope.qualify_label(blank) == label {
                            return Some(TermKind::Bnode);
                        }
                        break;
                    }
                    crate::TermRef::Literal {
                        datatype, language, ..
                    } => {
                        // The owned boundary omitted the implicit string datatype
                        // and every language literal's implied datatype.
                        if language.is_none()
                            && let crate::TermRef::Iri(iri) = dataset.resolve(datatype)
                            && iri != purrdf_xsd::datatype::XSD_STRING
                            && iri == label
                        {
                            return Some(TermKind::Iri);
                        }
                        break;
                    }
                    crate::TermRef::Triple { s, p, o } => {
                        // Frozen RDF permits nested triples only in object slots.
                        match dataset.resolve(s) {
                            crate::TermRef::Iri(iri) if iri == label => return Some(TermKind::Iri),
                            crate::TermRef::Blank {
                                label: blank,
                                scope,
                            } if scope.qualify_label(blank) == label => {
                                return Some(TermKind::Bnode);
                            }
                            _ => {}
                        }
                        if matches!(dataset.resolve(p), crate::TermRef::Iri(iri) if iri == label) {
                            return Some(TermKind::Iri);
                        }
                        id = o;
                    }
                }
            }
        }
        None
    })();
    memory.release_vec(seen)?;
    Ok(result)
}

fn metadata_value_to_cbor(value: &RdfMetadataValue) -> Value {
    match value {
        RdfMetadataValue::Null => Value::Null,
        RdfMetadataValue::Bool(b) => Value::Bool(*b),
        RdfMetadataValue::Integer(i) => match purrdf_lex::cbor::Integer::try_from(*i) {
            Ok(integer) => Value::Integer(integer),
            Err(_) => Value::Integer(purrdf_lex::cbor::Integer::from(if *i < 0 {
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
        RdfAnnotation, RdfLiteral, RdfMetadataEntry, RdfMetadataValue, RdfQuad, RdfReifier,
        RdfSuppressionRecord, RdfTerm, RdfTextDirection, RdfTriple, TermId,
    };
    use std::sync::Arc;

    /// Intern an owned [`RdfTerm`] into a builder, returning its id — the
    /// test-side inverse of the writer's owned-boundary resolution, which is
    /// exactly what [`RdfDatasetBuilder::intern_owned_term`] is (blank labels
    /// decoded back to their `(label, scope)` pair, triple terms recursed).
    fn intern_owned(b: &mut RdfDatasetBuilder, term: &RdfTerm) -> TermId {
        b.intern_owned_term(term).unwrap()
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
            let p = b.intern_iri(&q.predicate).unwrap();
            let o = intern_owned(&mut b, &q.object);
            let g = q.graph_name.as_ref().map(|g| intern_owned(&mut b, g));
            b.push_quad(s, p, o, g).unwrap();
        }
        for r in reifiers {
            let rid = intern_owned(&mut b, &r.reifier);
            let s = intern_owned(&mut b, &r.statement.subject);
            let p = b.intern_iri(&r.statement.predicate).unwrap();
            let o = intern_owned(&mut b, &r.statement.object);
            let triple = b.intern_triple(s, p, o).unwrap();
            b.push_reifier(rid, triple).unwrap();
        }
        for a in annotations {
            let rid = intern_owned(&mut b, &a.reifier);
            let p = b.intern_iri(&a.predicate).unwrap();
            let o = intern_owned(&mut b, &a.object);
            b.push_annotation(rid, p, o).unwrap();
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
    /// purrdf.gts container layer only, and rdf-core is the kernel below
    /// purrdf, so the native `serialize_dataset` is out of reach here). For the
    /// single-quad fixtures below the canonical document is identical to the written
    /// quad line; multi-quad callers get a deterministic bytewise-sorted document.
    fn roundtrip_nquads(dataset: &RdfDataset, profile: &str) -> purrdf_core::Canonicalized {
        let graph = roundtrip(dataset, &RdfLookaside::default(), profile);
        let bundle =
            crate::import_gts_graph(graph).expect("import the round-tripped GTS graph into the IR");
        crate::ir::try_canonicalize(
            &bundle.dataset,
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .unwrap()
    }

    fn assert_nquads_eq(dataset: &RdfDataset, profile: &str, expected: &str) {
        let nquads_owner = roundtrip_nquads(dataset, profile);
        let nquads = nquads_owner.nquads();
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

        let nquads_owner = roundtrip_nquads(&ds, "purrdf-test");
        let nquads = nquads_owner.nquads();
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
        let triples: std::collections::BTreeSet<purrdf_gts::model::Triple3> =
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
    fn shallow_wire_spelling_and_legacy_suppression_owner_are_preserved() {
        use purrdf_gts::model::Term;
        let mut builder = RdfDatasetBuilder::new();
        let r = builder
            .intern_blank("urn:claim", crate::BlankScope::DEFAULT)
            .expect("reifier");
        let s = builder.intern_iri("urn:claim").expect("same-spelled IRI");
        let p = builder.intern_iri("urn:p").expect("predicate");
        let o = builder.intern_iri("urn:o").expect("object");
        let g = builder.intern_iri("urn:g").expect("graph");
        let triple = builder.intern_triple(s, p, o).expect("quoted term");
        builder
            .push_quad(s, p, triple, Some(g))
            .expect("ordinary row");
        builder
            .push_reifier_in_graph(r, triple, Some(g))
            .expect("binding");
        builder
            .push_annotation_in_graph(r, p, triple, Some(g))
            .expect("annotation");
        let dataset = builder.freeze().expect("valid native statements");
        let mut lookaside = RdfLookaside::default();
        lookaside.suppressions.push(RdfSuppressionRecord {
            by: Some("urn:claim".to_owned()),
            reason: Some("kept".to_owned()),
            targets: vec![RdfMetadataValue::Text("proof".to_owned())],
        });
        let scalar = |kind, value: &str| Term {
            kind,
            value: Some(value.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        };
        // These are the legacy graph's concrete fields: the explicit reifier
        // prepass minted the blank first, and quoted terms omitted `v` entirely.
        let mut quoted = scalar(TermKind::Triple, "");
        quoted.value = None;
        quoted.triple = Some((1, 2, 3));
        let reference = Graph {
            terms: vec![
                scalar(TermKind::Bnode, "urn:claim"),
                scalar(TermKind::Iri, "urn:claim"),
                scalar(TermKind::Iri, "urn:p"),
                scalar(TermKind::Iri, "urn:o"),
                quoted,
                scalar(TermKind::Iri, "urn:g"),
            ],
            quads: vec![(1, 2, 4, Some(5))],
            reifiers: vec![(0, (1, 2, 3), Some(5))],
            annotations: vec![(0, 2, 4, Some(5))],
            suppressions: vec![Suppression {
                targets: vec![Value::Text("proof".to_owned())],
                reason: Some("kept".to_owned()),
                by: Some(0),
            }],
            ..Graph::default()
        };
        let expected = Writer::deterministic(&reference, "purrdf-test")
            .expect("legacy graph bytes")
            .into_bytes();
        assert_eq!(
            to_gts(&dataset, &lookaside, "purrdf-test").expect("native writer"),
            expected
        );
    }

    #[test]
    fn deep_native_terms_roundtrip_with_flat_writer_storage() {
        purrdf_stack::on_stack(128 * 1024, || {
            for depth in [16_usize, 17, 2048] {
                let mut builder = RdfDatasetBuilder::new();
                let s = builder
                    .intern_iri("https://example.org/s")
                    .expect("subject");
                let p = builder
                    .intern_iri("https://example.org/p")
                    .expect("predicate");
                let leaf = builder
                    .intern_iri("https://example.org/leaf")
                    .expect("leaf");
                let g = builder.intern_iri("https://example.org/g").expect("graph");
                let r = builder
                    .intern_blank("claim", crate::BlankScope::DEFAULT)
                    .expect("reifier");
                let mut root = leaf;
                for _ in 0..depth {
                    root = builder.intern_triple(s, p, root).expect("finite term");
                }
                builder.push_quad(s, p, root, None).expect("ordinary row");
                builder
                    .push_reifier_in_graph(r, root, Some(g))
                    .expect("named binding");
                builder
                    .push_annotation_in_graph(r, p, root, Some(g))
                    .expect("named annotation");
                let dataset = builder.freeze().expect("valid object-only nesting");
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let bytes = to_gts(&dataset, &RdfLookaside::default(), "purrdf-test")
                    .expect("deep GTS writer");
                let allocation = window.close();
                assert!(allocation.requested_bytes > 0, "counting allocator active");
                assert!(
                    allocation.peak_working_bytes <= (depth * 8192 + 262_144) as i64,
                    "{depth}: {allocation:?}"
                );
                assert!(
                    allocation.requested_bytes <= (depth * 32_768 + 1_048_576) as u64,
                    "{depth}: {allocation:?}"
                );
                let graph = purrdf_gts::reader::read(&bytes, false, None);
                assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
                assert_eq!(
                    graph
                        .terms
                        .iter()
                        .filter(|term| term.kind == TermKind::Triple)
                        .count(),
                    depth
                );
                let folded = crate::import_gts_graph(graph).expect("consuming flat graph importer");
                assert_eq!(
                    to_gts(&folded.dataset, &RdfLookaside::default(), "purrdf-test")
                        .expect("stable re-emit"),
                    bytes
                );
                let streamed = crate::import_gts_events(&bytes).expect("streamed GTS importer");
                for result in [&folded.dataset, &streamed.dataset] {
                    assert_eq!(result.quad_count(), 1);
                    assert_eq!(result.reifier_quads().count(), 1);
                    assert_eq!(result.annotation_quads().count(), 1);
                    let mut id = result.quads().next().expect("ordinary row").o;
                    for _ in 0..depth {
                        let crate::TermRef::Triple { o, .. } = result.resolve(id) else {
                            panic!("all nested terms survive")
                        };
                        id = o;
                    }
                    assert_eq!(
                        result.resolve(id),
                        crate::TermRef::Iri("https://example.org/leaf")
                    );
                    assert!(result.reifier_quads().all(|row| row.g.is_some()));
                    assert!(result.annotation_quads().all(|row| row.g.is_some()));
                }
            }
        })
        .expect("small-stack GTS round trip");
    }
}

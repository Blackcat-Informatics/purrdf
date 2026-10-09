// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical graph transfers and document-local LOAD identities through the public engine.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, GraphExistenceMode, RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension,
    SparqlRequest, TermValue,
};
use purrdf_sparql_eval::governor::ChargePoint;
use purrdf_sparql_eval::{
    GraphResolveRequest, GraphResolver, LoadError, NativeSparqlEngine, QueryGovernors, QueryOptions,
};

struct CachedDocument(Arc<RdfDataset>);

impl GraphResolver for CachedDocument {
    fn resolve(&self, _: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        Ok(Arc::clone(&self.0))
    }
}

const MODES: [GraphExistenceMode; 2] = [
    GraphExistenceMode::Implicit,
    GraphExistenceMode::RememberEmpty,
];

fn apply(
    engine: &NativeSparqlEngine,
    dataset: &mut Arc<RdfDataset>,
    query: &str,
    mode: GraphExistenceMode,
) {
    engine
        .update_with_options(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_graph_existence(mode),
        )
        .expect("production UPDATE succeeds");
}

fn repeated_cached_document_load_has_fresh_identities() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("b0", BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    let engine =
        NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(b.freeze().unwrap())));
    for mode in MODES {
        let mut ds = RdfDatasetBuilder::new().freeze().unwrap();
        apply(&engine, &mut ds, "LOAD <https://example.org/doc>", mode);
        assert_eq!(ds.rdf_row_count(), 1, "single-document neighbor");
        apply(
            &engine,
            &mut ds,
            "LOAD <https://example.org/doc>; LOAD <https://example.org/doc>",
            mode,
        );
        assert_eq!(
            ds.rdf_row_count(),
            3,
            "each successful resolution is a distinct document"
        );
        let subjects: std::collections::BTreeSet<_> = ds.quads().map(|q| q.s).collect();
        assert_eq!(subjects.len(), 3);
        apply(
            &engine,
            &mut ds,
            "LOAD <https://example.org/other-one>; LOAD <https://example.org/other-two>",
            mode,
        );
        assert_eq!(
            ds.rdf_row_count(),
            5,
            "different source IRIs sharing one cached Arc stay distinct"
        );
    }
}

fn annotation_copy(overlap: bool) {
    for mode in MODES {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/s");
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        let g = b.intern_iri("https://example.org/source");
        if overlap {
            b.push_quad(s, p, o, Some(g));
        }
        b.push_annotation_in_graph(s, p, o, Some(g));
        let mut ds = b.freeze().unwrap();
        let engine = NativeSparqlEngine::new();
        apply(
            &engine,
            &mut ds,
            "COPY GRAPH <https://example.org/source> TO GRAPH <https://example.org/source>",
            mode,
        );
        assert_eq!(ds.annotations().count(), 1, "self-copy neighbor");
        apply(
            &engine,
            &mut ds,
            "COPY GRAPH <https://example.org/source> TO GRAPH <https://example.org/destination>",
            mode,
        );
        assert_eq!(
            ds.annotations().count(),
            2,
            "annotation role survives transfer"
        );
        assert_eq!(ds.quads().count(), usize::from(overlap) * 2);
        assert_eq!(ds.rdf_row_count(), (usize::from(overlap) + 1) * 2);
    }
}

fn orphan_annotation_copy_preserves_its_role() {
    annotation_copy(false);
}

fn equal_value_ordinary_and_annotation_copy_preserves_both_roles() {
    annotation_copy(true);
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn physical_source(graph: Option<&str>) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("source", BlankScope(7));
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    let g = graph.map(|name| b.intern_iri(name));
    b.push_quad(s, p, o, g);
    b.push_annotation_in_graph(s, p, o, g);
    let triple = b.intern_triple(s, p, o);
    let reifier = b.intern_iri("https://example.org/r");
    let reifies = b.intern_iri(purrdf_iri::vocab::rdf::REIFIES);
    b.push_reifier_in_graph(reifier, triple, g);
    // Exact ordinary role, even though its positions could denote a reifier.
    b.push_quad(reifier, reifies, triple, g);
    b.freeze().unwrap()
}

fn all_transfers_preserve_exact_roles_and_source_blanks_in_both_graphs() {
    for mode in MODES {
        for (source, target) in [
            ("DEFAULT", "GRAPH <https://example.org/destination>"),
            ("GRAPH <https://example.org/source>", "DEFAULT"),
        ] {
            for op in ["ADD", "COPY", "MOVE"] {
                let original =
                    physical_source((source != "DEFAULT").then_some("https://example.org/source"));
                let old = purrdf_core::ir::import::record_values(original.as_ref()).unwrap();
                assert_eq!(old.len(), 4);
                let mut ds = Arc::clone(&original);
                apply(
                    &NativeSparqlEngine::new(),
                    &mut ds,
                    &format!("{op} {source} TO {target}"),
                    mode,
                );
                let records = purrdf_core::ir::import::record_values(ds.as_ref()).unwrap();
                let dest = (target != "DEFAULT")
                    .then(|| TermValue::iri("https://example.org/destination"));
                let actual: Vec<_> = records
                    .iter()
                    .filter(|record| record.quad.g == dest)
                    .cloned()
                    .collect();
                let expected: Vec<_> = old
                    .iter()
                    .cloned()
                    .map(|mut record| {
                        record.quad.g.clone_from(&dest);
                        record
                    })
                    .collect();
                assert_eq!(actual, expected, "{mode:?} {op} {source}");
                assert_eq!(records.len(), if op == "MOVE" { 4 } else { 8 });
                assert_eq!(
                    purrdf_core::ir::import::record_values(original.as_ref()).unwrap(),
                    old
                );
            }
        }
    }
}

fn transfers_charge_physical_attempts_even_when_add_deduplicates() {
    for mode in MODES {
        let engine = NativeSparqlEngine::new();
        for op in ["ADD", "COPY", "MOVE"] {
            let source = physical_source(Some("https://example.org/source"));
            let mut seeded = source;
            apply(
                &engine,
                &mut seeded,
                "ADD GRAPH <https://example.org/source> TO GRAPH <https://example.org/destination>",
                mode,
            );
            let query = format!(
                "{op} GRAPH <https://example.org/source> TO GRAPH <https://example.org/destination>"
            );
            let mut measured = Arc::clone(&seeded);
            let outcome = engine
                .update_governed(
                    &mut measured,
                    request(&query),
                    QueryOptions::new().with_graph_existence(mode),
                    &QueryGovernors::METERED,
                )
                .unwrap();
            assert!(outcome.is_applied());
            let fuel = outcome.evidence().consumed.get(ResourceDimension::Fuel);
            let attempts = match op {
                "ADD" => 4,
                "COPY" => 8,
                "MOVE" => 12,
                _ => unreachable!(),
            };
            // Existing UpdateMutatedQuad schedule: physical attempt count, not final set difference.
            assert_eq!(fuel, attempts * ChargePoint::UpdateMutatedQuad.cost());
            for (limit, pass) in [(fuel, true), (fuel - 1, false), (0, false)] {
                let mut ds = Arc::clone(&seeded);
                let out = engine
                    .update_governed(
                        &mut ds,
                        request(&query),
                        QueryOptions::new().with_graph_existence(mode),
                        &QueryGovernors::UNBOUNDED.with_fuel(limit),
                    )
                    .unwrap();
                assert_eq!(out.is_applied(), pass);
                if !pass {
                    assert!(Arc::ptr_eq(&ds, &seeded));
                }
            }
        }
    }
}

fn load_freshens_scoped_bare_triple_and_nested_composite_identities_together() {
    use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p");
    let root = b.intern_iri("https://example.org/root");
    let a = b.intern_blank("same", BlankScope(2));
    let other = b.intern_blank("same", BlankScope(3));
    let discarded = b.intern_blank("graph-only", BlankScope(4));
    b.push_quad(a, p, other, Some(discarded));
    let mut triple = b.intern_triple(a, p, other);
    for _ in 0..7 {
        triple = b.intern_triple(a, p, triple);
    }
    b.push_quad(root, p, triple, Some(discarded));
    let token = encode_blank_label("same", BlankScope(2), LabelAlphabet::BlankNodeLabel);
    for (datatype, lexical) in [
        (
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
            format!(
                "[_:{token}, [_:{token}], '\\u005B_:{token}\\u005D'^^<http://w3id.org/awslabs/neptune/SPARQL-CDTs/List>]"
            ),
        ),
        (
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map",
            format!("{{'key': _:{token}}}"),
        ),
    ] {
        let literal = b.intern_literal(RdfLiteral::typed(lexical, datatype));
        b.push_quad(a, p, literal, Some(discarded));
    }
    let opaque = b.intern_literal(RdfLiteral::simple("_:same [untouched]"));
    b.push_quad(root, p, opaque, Some(discarded));
    let source = b.freeze().unwrap();
    let original = purrdf_core::ir::import::record_values(source.as_ref()).unwrap();
    let engine =
        NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(Arc::clone(&source))));
    for mode in MODES {
        let mut ds = RdfDatasetBuilder::new().freeze().unwrap();
        apply(
            &engine,
            &mut ds,
            "LOAD <https://example.org/doc> INTO GRAPH <https://example.org/one>; LOAD <https://example.org/doc> INTO GRAPH <https://example.org/two>",
            mode,
        );
        let records = purrdf_core::ir::import::record_values(ds.as_ref()).unwrap();
        let mut sets = Vec::new();
        for graph in ["https://example.org/one", "https://example.org/two"] {
            let mut pairs = std::collections::BTreeSet::new();
            let rows: Vec<_> = records
                .iter()
                .filter(|r| r.quad.g == Some(TermValue::iri(graph)))
                .collect();
            assert_eq!(rows.len(), original.len());
            for row in rows {
                for term in [&row.quad.s, &row.quad.o] {
                    let _: std::ops::ControlFlow<()> =
                        term.visit_blank_identities(|label, scope| {
                            pairs.insert((label.to_owned(), scope));
                            std::ops::ControlFlow::Continue(())
                        });
                }
                if let TermValue::Literal {
                    lexical_form,
                    datatype,
                    ..
                } = &row.quad.o
                {
                    if purrdf_core::cdt_blank::is_cdt_datatype(datatype) {
                        let subject = match &row.quad.s {
                            TermValue::Blank { label, scope } => (label.clone(), *scope),
                            _ => panic!("composite fixture subject is blank"),
                        };
                        let embedded =
                            purrdf_core::cdt_blank::cdt_embedded_blanks(lexical_form, datatype);
                        assert_eq!(
                            embedded.len(),
                            if datatype.ends_with("/List") { 3 } else { 1 }
                        );
                        assert!(embedded.into_iter().all(|pair| pair == subject));
                    } else {
                        assert_eq!(lexical_form, "_:same [untouched]");
                    }
                }
            }
            assert_eq!(
                pairs.len(),
                2,
                "graph-only blank is discarded; two source scopes stay distinct"
            );
            assert!(pairs.iter().all(|(_, scope)| *scope == BlankScope::DEFAULT));
            sets.push(pairs);
        }
        assert!(sets[0].is_disjoint(&sets[1]));
        assert_eq!(
            purrdf_core::ir::import::record_values(source.as_ref()).unwrap(),
            original
        );
    }
}

fn load_shares_request_mints_with_data_templates_and_bnode() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_blank("b0", BlankScope::DEFAULT);
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    let engine =
        NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(b.freeze().unwrap())));
    for mode in MODES {
        let mut ds = RdfDatasetBuilder::new().freeze().unwrap();
        apply(
            &engine,
            &mut ds,
            "INSERT DATA { _:x <https://example.org/p> <https://example.org/o> }; LOAD <https://example.org/doc>; INSERT { _:x <https://example.org/p> <https://example.org/o> . ?b <https://example.org/p> <https://example.org/o> } WHERE { BIND(BNODE() AS ?b) }; LOAD <https://example.org/doc>",
            mode,
        );
        assert_eq!(ds.rdf_row_count(), 5);
        assert_eq!(
            ds.quads()
                .map(|q| q.s)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            5
        );
    }
}

fn check_collision_inventory(engine: &NativeSparqlEngine, mode: GraphExistenceMode) {
    for kind in 0..3 {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        if kind == 2 {
            // No bare blank term: the only identity is embedded in a CDT value.
            let s = b.intern_iri("https://example.org/composite");
            let literal = b.intern_literal(RdfLiteral::typed(
                "[_:c1]",
                "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
            ));
            b.push_quad(s, p, literal, None);
        } else {
            let blank = b.intern_blank("c1", BlankScope::DEFAULT);
            if kind == 1 {
                b.push_quad(blank, p, o, None);
            }
        }
        let mut ds = b.freeze().unwrap();
        apply(
            engine,
            &mut ds,
            "DELETE WHERE { ?s <https://example.org/p> ?o }; LOAD <https://example.org/doc>",
            mode,
        );
        assert_ne!(
            ds.term_value(ds.quads().next().unwrap().s),
            TermValue::blank("c1")
        );
    }
}

fn load_avoids_unused_suppressed_and_composite_only_destination_blanks() {
    let mut document = RdfDatasetBuilder::new();
    let blank = document.intern_blank("incoming", BlankScope::DEFAULT);
    let p = document.intern_iri("https://example.org/p");
    let o = document.intern_iri("https://example.org/o");
    document.push_quad(blank, p, o, None);
    let engine = NativeSparqlEngine::new()
        .with_resolver(Arc::new(CachedDocument(document.freeze().unwrap())));
    for mode in MODES {
        check_collision_inventory(&engine, mode);
        let mut b = RdfDatasetBuilder::new();
        b.intern_blank("chosen_c1", BlankScope::DEFAULT); // Unused dictionary identity.
        let s = b.intern_blank("c1", BlankScope::DEFAULT);
        let p = b.intern_iri("https://example.org/p");
        let o = b.intern_iri("https://example.org/o");
        b.push_quad(s, p, o, None);
        let subject = b.intern_iri("https://example.org/composite");
        let lit = b.intern_literal(RdfLiteral::typed(
            "[_:c2]",
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List",
        ));
        b.push_quad(subject, p, lit, None);
        let mut ds = b.freeze().unwrap();
        engine.update_with_options(&mut ds, request("DELETE WHERE { ?s <https://example.org/p> <https://example.org/o> }; LOAD <https://example.org/doc>"), QueryOptions::new().with_graph_existence(mode).with_bnode_mint_prefix(Some("chosen_"))).unwrap();
        let source = ds
            .quads()
            .find(|q| matches!(ds.resolve(q.s), purrdf_core::TermRef::Blank { .. }))
            .unwrap();
        assert_ne!(ds.term_value(source.s), TermValue::blank("chosen_c1"));
        assert_ne!(ds.term_value(source.s), TermValue::blank("c1"));
        assert_ne!(ds.term_value(source.s), TermValue::blank("c2"));
        let mut empty = RdfDatasetBuilder::new().freeze().unwrap();
        engine
            .update_with_options(
                &mut empty,
                request("LOAD <https://example.org/doc>"),
                QueryOptions::new()
                    .with_graph_existence(mode)
                    .with_bnode_mint_prefix(Some("chosen_")),
            )
            .unwrap();
        assert_eq!(
            empty.term_value(empty.quads().next().unwrap().s),
            TermValue::blank("chosen_c1"),
            "free requested prefix neighbor"
        );
    }
}

fn load_preserves_physical_roles_and_charges_each_original_record() {
    for mode in MODES {
        let document = physical_source(Some("https://example.org/discarded"));
        let engine = NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(document)));
        let empty = RdfDatasetBuilder::new().freeze().unwrap();
        let text = "LOAD <https://example.org/doc> INTO GRAPH <https://example.org/target>";
        let mut measured = Arc::clone(&empty);
        let result = engine
            .update_governed(
                &mut measured,
                request(text),
                QueryOptions::new().with_graph_existence(mode),
                &QueryGovernors::METERED,
            )
            .unwrap();
        assert!(result.is_applied());
        assert_eq!(measured.quads().count(), 2);
        assert_eq!(measured.reifier_quads().count(), 1);
        assert_eq!(measured.annotations().count(), 1);
        assert_eq!(measured.named_graphs().count(), 1);
        let fuel = result.evidence().consumed.get(ResourceDimension::Fuel);
        assert_eq!(
            fuel,
            ChargePoint::RemoteRequestIssued.cost() + 4 * ChargePoint::UpdateMutatedQuad.cost(),
            "one host fetch and four physical mutations"
        );
        assert_eq!(
            result
                .evidence()
                .consumed
                .get(ResourceDimension::RemoteRequests),
            1
        );
        for (limit, pass) in [(fuel, true), (fuel - 1, false), (0, false)] {
            let mut ds = Arc::clone(&empty);
            let result = engine
                .update_governed(
                    &mut ds,
                    request(text),
                    QueryOptions::new().with_graph_existence(mode),
                    &QueryGovernors::UNBOUNDED.with_fuel(limit),
                )
                .unwrap();
            assert_eq!(result.is_applied(), pass);
            if !pass {
                assert!(Arc::ptr_eq(&ds, &empty));
            }
        }
    }
}

fn load_charges_original_physical_records_when_destination_deduplicates() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("https://example.org/s");
    let p = b.intern_iri("https://example.org/p");
    let o = b.intern_iri("https://example.org/o");
    b.push_quad(s, p, o, None);
    b.push_annotation_in_graph(s, p, o, None);
    let engine =
        NativeSparqlEngine::new().with_resolver(Arc::new(CachedDocument(b.freeze().unwrap())));
    for mode in MODES {
        let mut ds = RdfDatasetBuilder::new().freeze().unwrap();
        let result = engine
            .update_governed(
                &mut ds,
                request("LOAD <https://example.org/doc>; LOAD <https://example.org/doc>"),
                QueryOptions::new().with_graph_existence(mode),
                &QueryGovernors::METERED,
            )
            .unwrap();
        assert!(result.is_applied());
        assert_eq!(ds.rdf_row_count(), 2);
        assert_eq!(ds.annotations().count(), 1);
        assert_eq!(
            result.evidence().consumed.get(ResourceDimension::Fuel),
            2 * (ChargePoint::RemoteRequestIssued.cost()
                + 2 * ChargePoint::UpdateMutatedQuad.cost())
        );
        assert_eq!(
            result
                .evidence()
                .consumed
                .get(ResourceDimension::RemoteRequests),
            2
        );
    }
}

purrdf_testkit::harness_main!(
    repeated_cached_document_load_has_fresh_identities,
    orphan_annotation_copy_preserves_its_role,
    equal_value_ordinary_and_annotation_copy_preserves_both_roles,
    all_transfers_preserve_exact_roles_and_source_blanks_in_both_graphs,
    transfers_charge_physical_attempts_even_when_add_deduplicates,
    load_freshens_scoped_bare_triple_and_nested_composite_identities_together,
    load_shares_request_mints_with_data_templates_and_bnode,
    load_avoids_unused_suppressed_and_composite_only_destination_blanks,
    load_preserves_physical_roles_and_charges_each_original_record,
    load_charges_original_physical_records_when_destination_deduplicates,
);

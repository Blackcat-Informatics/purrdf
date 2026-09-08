// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Visual reifier roles are independent of the carrier's graph-scoped table fold.

use std::sync::Arc;

use purrdf_rdf::dataset_view::DatasetMut;
use purrdf_rdf::ir::MutableDataset;
use purrdf_rdf::viz::{
    VizGraphId, VizGraphInput, VizGraphPolicy, VizInputAnnotation, VizInputQuad, VizInputReifier,
    VizInputStatement, VizProjection, VizRelation, VizSpec, VizTermValue, VizValueRef,
    project_dataset, project_graph_input,
};
use purrdf_rdf::{
    QuadValues, RdfDataset, RdfDatasetBuilder, TermValue, canonical_flat_nquads, parse_dataset,
};

const EX: &str = "https://example.org/";
const NQUADS: &str = concat!(
    "<https://example.org/alice> <https://example.org/knows> <https://example.org/bob> <https://example.org/facts> .\n",
    "<https://example.org/claim> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/alice> <https://example.org/knows> <https://example.org/bob> )>> <https://example.org/claims> .\n",
    "<https://example.org/claim> <https://example.org/confidence> \"0.8\"^^<http://www.w3.org/2001/XMLSchema#decimal> <https://example.org/provenance> .\n",
);

fn iri(local: &str) -> TermValue {
    TermValue::Iri(format!("{EX}{local}"))
}

fn fixture() -> Arc<RdfDataset> {
    parse_dataset(NQUADS.as_bytes(), "application/n-quads", None).expect("valid fixture")
}

fn input() -> VizGraphInput {
    VizGraphInput {
        quads: vec![
            VizInputQuad {
                subject: iri("alice"),
                predicate: format!("{EX}knows"),
                object: iri("bob"),
                graph_name: Some(iri("facts")),
            },
            VizInputQuad {
                subject: iri("claim"),
                predicate: format!("{EX}confidence"),
                object: TermValue::Literal {
                    lexical_form: "0.8".to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
                    language: None,
                    direction: None,
                },
                graph_name: Some(iri("provenance")),
            },
        ],
        reifiers: vec![VizInputReifier {
            reifier: iri("claim"),
            statement: VizInputStatement {
                subject: iri("alice"),
                predicate: format!("{EX}knows"),
                object: iri("bob"),
            },
            graph_name: Some(iri("claims")),
        }],
        annotations: Vec::new(),
    }
}

fn all_quads(dataset: &RdfDataset) -> Vec<QuadValues> {
    dataset
        .quads()
        .chain(dataset.reifier_quads())
        .chain(dataset.annotation_quads())
        .map(|quad| QuadValues {
            s: dataset.term_value(quad.s),
            p: dataset.term_value(quad.p),
            o: dataset.term_value(quad.o),
            g: quad.g.map(|graph| dataset.term_value(graph)),
        })
        .collect()
}

fn added_fixture(dataset: &RdfDataset) -> MutableDataset {
    let mut added = MutableDataset::new(RdfDatasetBuilder::new().freeze().expect("empty"));
    for quad in all_quads(dataset) {
        assert!(added.insert(quad).expect("valid quad"));
    }
    added
}

fn graph_iri<'a>(model: &'a VizProjection, id: &VizGraphId) -> &'a str {
    let graph = model
        .graphs
        .iter()
        .find(|graph| &graph.id == id)
        .expect("graph");
    let Some(VizValueRef::Term { id }) = &graph.term else {
        panic!("fixture graph must be named");
    };
    let term = model
        .terms
        .iter()
        .find(|term| &term.id == id)
        .expect("term");
    let VizTermValue::Iri { value } = &term.value else {
        panic!("fixture graph must be an IRI");
    };
    value
}

#[test]
fn parsed_added_and_explicit_inputs_share_cross_graph_visual_roles() {
    let parsed = fixture();
    let added = added_fixture(&parsed).freeze().expect("freeze");
    for dataset in [&parsed, &added] {
        // The visual role must not reclassify the carrier's provenance-graph row.
        assert_eq!(dataset.quad_count(), 2);
        assert_eq!(dataset.reifiers().count(), 1);
        assert_eq!(dataset.annotations().count(), 0);
    }
    assert_eq!(
        canonical_flat_nquads(&parsed),
        canonical_flat_nquads(&added)
    );
    let spec = VizSpec::default();
    let model = project_dataset(&parsed, &spec).expect("project");
    assert_eq!(model, project_dataset(&added, &spec).expect("added"));
    let mut explicit = input();
    assert_eq!(model, project_graph_input(&explicit, &spec).expect("quads"));
    let annotation = explicit.quads.pop().expect("confidence quad");
    explicit.annotations.push(VizInputAnnotation {
        reifier: annotation.subject,
        predicate: annotation.predicate,
        object: annotation.object,
        graph_name: annotation.graph_name,
    });
    assert_eq!(model, project_graph_input(&explicit, &spec).expect("typed"));
    assert_eq!(model.statements.len(), 1);
    assert_eq!(model.assertions.len(), 1);
    assert_eq!(model.relations.len(), 2);
    assert_eq!(model.table.rows[0].annotation_count, 1);
    assert_eq!(
        graph_iri(&model, &model.assertions[0].graph),
        format!("{EX}facts")
    );
    for relation in &model.relations {
        let (graph, expected) = match relation {
            VizRelation::Reifies { graph, .. } => (graph, "claims"),
            VizRelation::Annotation { graph, .. } => (graph, "provenance"),
        };
        assert_eq!(graph_iri(&model, graph), format!("{EX}{expected}"));
    }
}

#[test]
fn graph_filters_select_occurrences_without_reassigning_reifier_roles() {
    let parsed = fixture();
    let added = added_fixture(&parsed).freeze().expect("freeze");
    for (graph, statements, assertions, relations) in [
        ("facts", 1, 1, 0),
        ("claims", 1, 0, 1),
        ("provenance", 0, 0, 1),
    ] {
        let spec = VizSpec {
            graph_policy: VizGraphPolicy::Include(vec![format!("{EX}{graph}")]),
            ..VizSpec::default()
        };
        let model = project_dataset(&parsed, &spec).expect("project");
        assert_eq!(model, project_dataset(&added, &spec).expect("added"));
        assert_eq!(model, project_graph_input(&input(), &spec).expect("input"));
        assert_eq!(model.statements.len(), statements);
        assert_eq!(model.assertions.len(), assertions);
        assert_eq!(model.relations.len(), relations);
        assert_eq!(model.graphs.len(), 1);
        assert_eq!(
            graph_iri(&model, &model.graphs[0].id),
            format!("{EX}{graph}")
        );
    }
}

#[test]
fn removing_and_restoring_reification_updates_roles_without_changing_properties() {
    let parsed = fixture();
    let mut mutable = MutableDataset::new(Arc::clone(&parsed));
    let declaration = all_quads(&parsed)
        .into_iter()
        .find(|quad| matches!(&quad.o, TermValue::Triple { .. }))
        .expect("reification declaration");
    assert!(mutable.remove(&declaration));
    let without_reifier = mutable.freeze().expect("freeze");
    assert_eq!(without_reifier.quad_count(), 2);
    assert_eq!(without_reifier.annotations().count(), 0);
    let spec = VizSpec::default();
    let model = project_dataset(&without_reifier, &spec).expect("project");
    assert_eq!(model.statements.len(), 2);
    assert_eq!(model.assertions.len(), 2);
    assert_eq!(model.relations, [] as [VizRelation; 0]);
    let mut explicit = input();
    explicit.reifiers.clear();
    assert_eq!(model, project_graph_input(&explicit, &spec).expect("input"));
    assert!(mutable.insert(declaration).expect("restore"));
    let restored = mutable.freeze().expect("freeze");
    assert_eq!(
        canonical_flat_nquads(&restored),
        canonical_flat_nquads(&parsed)
    );
    assert_eq!(
        project_dataset(&restored, &spec).expect("restored"),
        project_dataset(&parsed, &spec).expect("parsed"),
    );
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

#[path = "support/viz_terms.rs"]
mod viz_terms;
use viz_terms::{EX, iri};

#[path = "support/viz_presentation.rs"]
mod viz_presentation;

const NQUADS: &str = concat!(
    "<https://example.org/alice> <https://example.org/knows> <https://example.org/bob> <https://example.org/facts> .\n",
    "<https://example.org/claim> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://example.org/alice> <https://example.org/knows> <https://example.org/bob> )>> <https://example.org/claims> .\n",
    "<https://example.org/claim> <https://example.org/confidence> \"0.8\"^^<http://www.w3.org/2001/XMLSchema#decimal> <https://example.org/provenance> .\n",
);

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

fn added_fixture(dataset: &RdfDataset) -> MutableDataset {
    let mut added = MutableDataset::new(RdfDatasetBuilder::new().freeze().expect("empty"));
    for quad in QuadValues::surface_of(dataset) {
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
    let declaration = QuadValues::surface_of(&parsed)
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

#[test]
fn nested_presentation_keeps_cross_boundary_semantics_and_exact_visible_exports() {
    use purrdf_lex::json::record::FromJson;
    use purrdf_rdf::viz::*;
    let spec = VizSpec::default();
    let input = viz_presentation::input();
    let model = project_graph_input(&input, &spec).expect("projection");
    let presentation = viz_presentation::architecture(&model);
    let options = VizLayoutOptions::default();
    for mode in [VizMode::Compact, VizMode::Incidence, VizMode::Table] {
        let spec = VizSpec {
            mode,
            ..spec.clone()
        };
        let native = build_export(&model, &spec, &options).expect("native view");
        let empty = build_presented_export(&model, &spec, &VizPresentation::default(), &options)
            .expect("empty presentation");
        assert_eq!(
            empty.export, native,
            "an empty lens must preserve the native view"
        );
    }
    let presented =
        build_presented_export(&model, &spec, &presentation, &options).expect("compound export");
    assert_eq!(presented.export.model, model);
    let outer = &presented.geometry.components[0];
    let inner = &presented.geometry.components[1];
    assert!(outer.rect.x < inner.rect.x && outer.rect.y < inner.rect.y);
    assert!(outer.rect.x + outer.rect.width > inner.rect.x + inner.rect.width);
    assert!(outer.rect.y + outer.rect.height > inner.rect.y + inner.rect.height);
    let app = VizSemanticRef::Term(viz_presentation::term(&model, "app"));
    let node = presented
        .export
        .scene
        .nodes
        .iter()
        .find(|node| node.bindings.contains(&app))
        .expect("component node");
    let node = presented
        .export
        .layout
        .nodes
        .iter()
        .find(|geometry| geometry.id == node.id)
        .expect("component geometry");
    let self_edge = presented.export.scene.edges.iter().find(|edge| matches!((&edge.source, &edge.target), (VizEndpoint::NodePort { node: from, .. }, VizEndpoint::NodePort { node: to, .. }) if from == &node.id && to == &node.id)).expect("actual self edge");
    let route = &presented
        .export
        .layout
        .edges
        .iter()
        .find(|edge| edge.id == self_edge.id)
        .expect("self route")
        .points;
    assert!(route.iter().any(|point| point.y < node.rect.y));
    let self_edges: Vec<_> = presented.export.scene.edges.iter().filter(|edge| matches!((&edge.source, &edge.target), (VizEndpoint::NodePort { node: from, .. }, VizEndpoint::NodePort { node: to, .. }) if from == &node.id && to == &node.id)).map(|edge| edge.id.as_str()).collect();
    let lanes: std::collections::BTreeSet<_> = presented
        .export
        .layout
        .edges
        .iter()
        .filter(|edge| self_edges.contains(&edge.id.as_str()))
        .map(|edge| edge.points[1].x)
        .collect();
    assert_eq!(self_edges.len(), 2);
    assert_eq!(
        lanes.len(),
        2,
        "parallel self edges need distinct external lanes"
    );
    assert_self_route_outside(node.rect, route);
    quoted_self_neighbors();
    assert_ne!(presented.geometry.ports, [] as [VizBoundaryPort; 0]);
    for port in &presented.geometry.ports {
        let edge = presented
            .export
            .layout
            .edges
            .iter()
            .find(|e| e.id == port.edge)
            .expect("real RDF edge");
        assert!(
            edge.points.windows(2).any(|p| {
                (p[0].x == port.point.x
                    && p[1].x == port.point.x
                    && (p[0].y.min(p[1].y)..=p[0].y.max(p[1].y)).contains(&port.point.y))
                    || (p[0].y == port.point.y
                        && p[1].y == port.point.y
                        && (p[0].x.min(p[1].x)..=p[0].x.max(p[1].x)).contains(&port.point.x))
            }),
            "boundary port is not on its actual route: {port:?}"
        );
    }
    let svg = render_presented_export_svg(&presented, &VizSvgOptions::default()).expect("SVG");
    assert!(svg.contains("direction=\"rtl\""));
    assert!(svg.contains("lengthAdjust=\"spacingAndGlyphs\""));
    assert!(svg.contains("href=\"../reference/service.html#api\""));
    for entry in &presented.element_index {
        assert_eq!(
            svg.matches(&format!("id=\"{}\"", entry.element_id)).count(),
            1,
            "{}",
            entry.element_id
        );
    }
    let json = presented_export_json(&presented).expect("JSON");
    let decoded =
        VizPresentedExport::from_json(&purrdf_lex::json::read(&json).expect("native JSON"))
            .expect("typed export");
    assert_eq!(decoded, presented);
    let mut reordered = input;
    reordered.quads.reverse();
    let reordered = project_graph_input(&reordered, &spec).expect("reordered projection");
    assert_eq!(
        presented,
        build_presented_export(&reordered, &spec, &presentation, &options).expect("same export")
    );
    let mut renamed = presentation;
    renamed.components[0].label = VizSceneLabel::plain("Renamed boundary");
    let renamed = build_presented_export(&model, &spec, &renamed, &options).expect("renamed");
    assert_eq!(renamed.export.model_hash, presented.export.model_hash);
    assert_ne!(renamed.presentation_hash, presented.presentation_hash);
    let mut stale = presented;
    stale.geometry.components[0].rect.width += 1;
    assert!(matches!(
        render_presented_export_svg(&stale, &VizSvgOptions::default()),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Export,
            ..
        }))
    ));
}

fn assert_self_route_outside(rect: purrdf_rdf::viz::VizRect, route: &[purrdf_rdf::viz::VizPoint]) {
    for segment in route.windows(2) {
        let horizontal_inside = segment[0].y == segment[1].y
            && segment[0].y > rect.y
            && segment[0].y < rect.y + rect.height
            && segment[0].x.min(segment[1].x) < rect.x + rect.width
            && segment[0].x.max(segment[1].x) > rect.x;
        let vertical_inside = segment[0].x == segment[1].x
            && segment[0].x > rect.x
            && segment[0].x < rect.x + rect.width
            && segment[0].y.min(segment[1].y) < rect.y + rect.height
            && segment[0].y.max(segment[1].y) > rect.y;
        assert!(
            !horizontal_inside && !vertical_inside,
            "self route is hidden inside its endpoint: {segment:?}"
        );
    }
}

fn quoted_self_neighbors() {
    use purrdf_core::TermBox;
    use purrdf_rdf::viz::*;
    for asserted in [false, true] {
        let quoted = TermValue::Triple {
            s: TermBox::new(iri("quote-a")),
            p: TermBox::new(iri("quote-p")),
            o: TermBox::new(iri("quote-b")),
        };
        let mut input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: quoted.clone(),
                predicate: format!("{EX}quote-outer"),
                object: quoted,
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        if asserted {
            input.quads.push(VizInputQuad {
                subject: iri("quote-a"),
                predicate: format!("{EX}quote-p"),
                object: iri("quote-b"),
                graph_name: None,
            });
        }
        let spec = VizSpec::default();
        let model = project_graph_input(&input, &spec).expect("quoted self projection");
        let outer = model.statements.iter().find(|statement| model.terms.iter().any(|term| term.id == statement.predicate && matches!(&term.value, VizTermValue::Iri { value } if value == &format!("{EX}quote-outer")))).expect("outer statement");
        let VizValueRef::Statement { id } = &outer.subject else {
            panic!("quoted subject")
        };
        assert_eq!(
            !model
                .statements
                .iter()
                .find(|s| &s.id == id)
                .expect("quoted statement")
                .asserted_in
                .is_empty(),
            asserted
        );
        let export = build_presented_export(
            &model,
            &spec,
            &VizPresentation::default(),
            &VizLayoutOptions::default(),
        )
        .expect("quoted self neighbor");
        let edge = export
            .export
            .scene
            .edges
            .iter()
            .find(|edge| {
                edge.kind == VizSceneEdgeKind::Assertion
                    && edge
                        .bindings
                        .contains(&VizSemanticRef::Statement(outer.id.clone()))
            })
            .expect("outer assertion edge");
        assert_eq!(edge.source, edge.target);
        let rect = match &edge.source {
            VizEndpoint::NodePort { node, .. } => {
                assert!(!asserted);
                export
                    .export
                    .layout
                    .nodes
                    .iter()
                    .find(|n| &n.id == node)
                    .expect("quoted node")
                    .rect
            }
            VizEndpoint::EdgeAnchor { edge, anchor } => {
                assert!(asserted);
                export
                    .export
                    .layout
                    .edges
                    .iter()
                    .find(|e| &e.id == edge)
                    .and_then(|e| e.anchor.as_ref())
                    .filter(|a| &a.id == anchor)
                    .expect("asserted statement anchor")
                    .rect
            }
        };
        let route = &export
            .export
            .layout
            .edges
            .iter()
            .find(|e| e.id == edge.id)
            .expect("self route")
            .points;
        assert_self_route_outside(rect, route);
        render_presented_export_svg(&export, &VizSvgOptions::default())
            .expect("complete quoted-self SVG");
    }
}

#[test]
fn declared_scenarios_preserve_order_error_and_parallel_grammar_with_visible_rdf_detail() {
    use purrdf_rdf::viz::*;
    let spec = VizSpec::default();
    let mut source = viz_presentation::input();
    let mut rdf = input();
    let mut quoted = rdf.reifiers[0].clone();
    quoted.statement.predicate = format!("{EX}quotedPredicate");
    rdf.reifiers.push(quoted);
    source.quads.extend(rdf.quads);
    source.reifiers.extend(rdf.reifiers);
    let model = project_graph_input(&source, &spec).expect("projection");
    let p = VizPresentation {
        scenario: Some(viz_presentation::scenario(&model)),
        ..VizPresentation::default()
    };
    let export =
        build_presented_export(&model, &spec, &p, &VizLayoutOptions::default()).expect("scenario");
    assert_eq!(export.export.model, model);
    assert_eq!(
        export.export.scene.edges.len(),
        build_scene(&model, &spec).expect("RDF detail").edges.len()
    );
    assert!(
        export
            .geometry
            .regions
            .iter()
            .any(|r| r.kind == Some(VizRegionKind::Error))
    );
    assert!(
        export
            .geometry
            .regions
            .iter()
            .any(|r| r.kind == Some(VizRegionKind::Parallel))
    );
    assert!(
        export
            .geometry
            .regions
            .iter()
            .all(|r| !r.bindings.is_empty())
    );
    assert!(export.geometry.interactions[0].id.ends_with("request"));
    assert!(
        export
            .geometry
            .interactions
            .windows(2)
            .all(|pair| pair[0].points[0].y < pair[1].points[0].y)
    );
    let self_message = export
        .geometry
        .interactions
        .iter()
        .find(|i| i.id.ends_with("cache-update"))
        .expect("self interaction");
    assert_eq!(self_message.points.len(), 4);
    let svg = render_presented_export_svg(&export, &VizSvgOptions::default()).expect("static SVG");
    assert!(svg.contains("no inter-lane order"));
    assert!(svg.contains("RDF detail"));
    assert!(svg.contains("error — query fails"));
    assert!(svg.contains("quoted only") && svg.contains("reifies") && svg.contains("annotation"));
    let mut reordered = p;
    let s = reordered.scenario.as_mut().expect("scenario");
    s.regions.reverse();
    s.interactions.reverse();
    assert_eq!(
        export,
        build_presented_export(&model, &spec, &reordered, &VizLayoutOptions::default())
            .expect("explicit order, not array order")
    );
}

#[test]
fn presentation_refusals_have_valid_binding_containment_order_and_navigation_neighbors() {
    use purrdf_rdf::viz::*;
    let spec = VizSpec::default();
    let model = project_graph_input(&viz_presentation::input(), &spec).expect("projection");
    let valid = viz_presentation::architecture(&model);
    let build = |p: &VizPresentation| {
        build_presented_export(&model, &spec, p, &VizLayoutOptions::default())
    };
    build(&valid).expect("valid containment and local link");
    let statement = &model.statements[0];
    let assertion = model
        .assertions
        .iter()
        .find(|a| a.statement == statement.id)
        .expect("asserted statement");
    let display = |binding, label: &str| VizDisplay {
        binding,
        label: Some(VizSceneLabel::plain(label)),
        description: None,
        role: None,
        navigation: None,
    };
    let mut shared = VizPresentation {
        displays: vec![
            display(
                VizSemanticRef::Statement(statement.id.clone()),
                "Shared label",
            ),
            display(
                VizSemanticRef::Assertion(assertion.id.clone()),
                "Conflicting label",
            ),
        ],
        ..VizPresentation::default()
    };
    assert!(matches!(
        build(&shared),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Binding,
            ..
        }))
    ));
    shared.displays[1].label = shared.displays[0].label.clone();
    let table_spec = VizSpec {
        mode: VizMode::Table,
        ..spec.clone()
    };
    let mut table = VizPresentation {
        displays: vec![display(
            VizSemanticRef::Statement(statement.id.clone()),
            "Reviewed call",
        )],
        ..VizPresentation::default()
    };
    let table_build = |p: &VizPresentation| {
        build_presented_export(&model, &table_spec, p, &VizLayoutOptions::default())
    };
    table_build(&table).expect("plain table decoration");
    table.displays[0].label.as_mut().expect("label").direction = Some(VizTextDirection::Rtl);
    assert!(matches!(
        table_build(&table),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Unsupported,
            ..
        }))
    ));
    table.displays[0].label = Some(VizSceneLabel::plain("Reviewed call"));
    table.displays[0]
        .label
        .as_mut()
        .expect("label")
        .full_text
        .push_str(" in full");
    assert!(matches!(
        table_build(&table),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Unsupported,
            ..
        }))
    ));
    table.displays[0].label = Some(VizSceneLabel::plain("Reviewed call"));
    table.displays[0].description = Some("d".repeat(1000));
    table_build(&table).expect("bounded additive text");
    table.displays[0].description = Some("d".repeat(262_000));
    assert!(matches!(
        table_build(&table),
        Err(VizError::TooLarge {
            limit: "presented table text",
            ..
        })
    ));
    let sized_table = |count| {
        let input = VizGraphInput {
            quads: (0..count)
                .map(|index| VizInputQuad {
                    subject: iri(&format!("table-{index}")),
                    predicate: format!("{EX}table-relation"),
                    object: iri("sink"),
                    graph_name: None,
                })
                .collect(),
            ..VizGraphInput::default()
        };
        let model = project_graph_input(&input, &table_spec).expect("bounded native model");
        build_presented_export(
            &model,
            &table_spec,
            &VizPresentation::default(),
            &VizLayoutOptions::default(),
        )
    };
    sized_table(100).expect("table cells within the scene budget");
    assert!(matches!(
        sized_table(300),
        Err(VizError::TooLarge {
            limit: "presented scene elements",
            ..
        })
    ));
    shared.displays[0].role = Some("sender".to_owned());
    shared.displays[1].role = Some("reviewed".to_owned());
    let composed = build(&shared).expect("identical labels sharing one visible element");
    assert!(composed.export.scene.edges.iter().any(|edge| {
        edge.bindings
            .contains(&VizSemanticRef::Assertion(assertion.id.clone()))
            && edge.label.text.contains("sender")
            && edge.label.text.contains("reviewed")
    }));
    let mut long = VizPresentation {
        displays: vec![display(
            VizSemanticRef::Term(viz_presentation::term(&model, "app")),
            &"a".repeat(4096),
        )],
        ..VizPresentation::default()
    };
    build(&long).expect("maximum display label remains renderable");
    long.displays[0].label = Some(VizSceneLabel::plain(&"a".repeat(4097)));
    assert!(matches!(
        build(&long),
        Err(VizError::TooLarge {
            limit: "display label bytes",
            ..
        })
    ));
    let mut missing = valid.clone();
    missing.components[0].binding = VizSemanticRef::Term(VizTermId("missing".to_owned()));
    assert!(matches!(build(&missing), Err(VizError::Presentation(_))));
    let mut cyclic = valid;
    cyclic.components[0].parent = Some("service".to_owned());
    assert!(matches!(
        build(&cyclic),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Containment,
            ..
        }))
    ));
    for link in [
        "javascript:alert(1)",
        "data:text/html,hello",
        "//example.org/file",
        "https://example.org/\n",
        "\\evil",
    ] {
        assert!(VizNavigation::parse(link).is_err(), "{link}");
    }
    for link in [
        "https://example.org/reference",
        "../reference.html#item",
        "#item",
    ] {
        VizNavigation::parse(link).expect("safe neighbor");
    }
    let mut scenario = VizPresentation {
        scenario: Some(viz_presentation::scenario(&model)),
        ..VizPresentation::default()
    };
    build(&scenario).expect("valid declared order");
    scenario.scenario.as_mut().expect("scenario").interactions[0].order = 1;
    assert!(matches!(
        build(&scenario),
        Err(VizError::Presentation(VizPresentationError {
            issue: VizPresentationIssue::Scenario,
            ..
        }))
    ));
}

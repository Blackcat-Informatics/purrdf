// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit fixed points through the native parse and validation entry points.
//!
//! The bsep1 facts and expected pair are the sheval separation case sa1 at
//! <https://github.com/cogsl/sheval/tree/01bc7d41e6e6a671f9d7d7ee4c11c646f4860265/test_suites/recursive_shapes>.
//! Its manifest asks whether :a conforms to :S. The explicit target below asks
//! that same pair through the production report entry point.

use std::sync::Arc;

use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{ValidationOptions, parse_shapes_with_options, validate_with};
use purrdf_shapes::recursion::{RecursionOptions, RecursionSemantics};
use purrdf_shapes::report::CompleteValidationError;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_shapes::{RecursionRefusalReason, ShapesError, ShapesImports};

const PREFIXES: &str = "@prefix : <http://example.org/> .
                       @prefix sh: <http://www.w3.org/ns/shacl#> .
                       @prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .
                       @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .";

fn options(semantics: RecursionSemantics) -> ValidationOptions {
    ValidationOptions::default().with_recursion(RecursionOptions {
        semantics,
        max_pairs: 4_096,
        max_iterations: 4_096,
    })
}

fn check_programmatic_admission(prepared: bool) {
    use purrdf_shapes::RecursionPolarity;
    use purrdf_shapes::constraints::{conforms, conforms_with_recursion};
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::shapes::{Constraint, ModelOwner};
    use purrdf_shapes::term::{NamedNode, Term};

    let mut shapes = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let id = Term::NamedNode(NamedNode::from("http://example.org/S"));
    let prototype = shapes.owner.definition(&id).unwrap().clone();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    let focus = Term::NamedNode(NamedNode::from("http://example.org/a"));
    for polarity in [
        RecursionPolarity::Positive,
        RecursionPolarity::Negative,
        RecursionPolarity::Both,
    ] {
        let mut handle = prototype.clone();
        handle.targets.clear();
        handle.constraints = vec![Constraint::Reference(id.clone())];
        let mut definition = prototype.clone();
        definition.constraints = vec![match polarity {
            RecursionPolarity::Positive => Constraint::Node(Box::new(handle)),
            RecursionPolarity::Negative => Constraint::Not(Box::new(handle)),
            RecursionPolarity::Both => Constraint::Xone(vec![handle]),
            RecursionPolarity::Target => unreachable!("the cases above are conformance edges"),
        }];
        let owner = ModelOwner::from_definitions(vec![definition]).unwrap();
        let root = owner.root(&id).unwrap();
        shapes.node_shapes = vec![root.clone()];
        shapes.owner = owner;
        for request in [
            ValidationOptions::default(),
            options(RecursionSemantics::Lfp),
            options(RecursionSemantics::Gfp),
        ] {
            let actual = if prepared {
                PreparedShapes::new(Arc::new(shapes.clone()))
                    .with_validation_options(request.clone())
                    .bind_dataset(&dataset)
                    .map(|validator| {
                        validator
                            .validate()
                            .expect("an admitted model evaluates")
                            .conforms
                    })
            } else if let Some(recursion) = request.recursion.as_ref() {
                conforms_with_recursion(&data, &focus, &root, recursion)
            } else {
                conforms(&data, &focus, &root)
            };
            let reason = match request.recursion.as_ref() {
                None => Some(RecursionRefusalReason::MissingSelection),
                Some(_) if polarity == RecursionPolarity::Positive => None,
                Some(recursion) => Some(RecursionRefusalReason::NonStratified {
                    semantics: recursion.semantics,
                }),
            };
            if let Some(reason) = reason {
                let ShapesError::Recursion(refusal) = actual.unwrap_err() else {
                    panic!(
                        "programmatic {polarity:?} must preserve typed admission evidence, prepared={prepared}"
                    );
                };
                assert_eq!(refusal.reason, reason);
                assert!(
                    refusal
                        .members()
                        .iter()
                        .any(|member| member == "<http://example.org/S>")
                );
                assert!(refusal.edges().iter().any(|edge| edge.polarity == polarity));
            } else {
                assert_eq!(
                    actual.unwrap(),
                    request.recursion.as_ref().unwrap().semantics == RecursionSemantics::Gfp
                );
            }
        }
    }
}

#[test]
fn programmatic_direct_calls_require_signed_recursion_admission() {
    check_programmatic_admission(false);
}

#[test]
fn programmatic_prepared_calls_require_signed_recursion_admission() {
    check_programmatic_admission(true);
}

/// Cache eviction cannot erase source-only declared edges or change a native
/// model's signed verdict. The cache retains no model bodies or data graphs.
#[test]
fn admission_metadata_survives_eviction_without_retaining_model_bodies() {
    use purrdf_shapes::constraints::conforms;
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::shapes::{Constraint, ModelOwner};
    use purrdf_shapes::term::{NamedNode, Term};

    let parsed = parse_shapes_with_options(
        &format!(
            "{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ;
                  sh:node :S {{| sh:deactivated true |}} ."
        ),
        None,
        None,
        None,
        &ShapesImports::new(),
        &options(RecursionSemantics::Gfp),
    )
    .unwrap();
    let id = Term::NamedNode(NamedNode::from("http://example.org/S"));
    let prototype = parsed.owner.definition(&id).unwrap().clone();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    let focus = Term::NamedNode(NamedNode::from("http://example.org/a"));
    let root = parsed.owner.root(&id).unwrap();
    let native_before = conforms(&data, &focus, &root).unwrap();
    let mut retained = Vec::new();
    for index in 0..40 {
        let mut definition = prototype.clone();
        definition.id = Term::BlankNode(format!("eviction-{index}"));
        let owner = ModelOwner::from_definitions(vec![definition.clone()]).unwrap();
        retained.push(owner.root(&definition.id).unwrap());
    }
    assert_eq!(conforms(&data, &focus, &root).unwrap(), native_before);
    let prepared = PreparedShapes::new(Arc::new(parsed.clone()));
    assert!(
        prepared
            .bind_dataset(&dataset)
            .unwrap()
            .validate()
            .unwrap()
            .conforms
    );
    let Err(refusal) = prepared
        .with_validation_options(ValidationOptions::default())
        .bind_dataset(&dataset)
    else {
        panic!("source-declared recursion still requires a selection after cache eviction")
    };
    assert!(
        matches!(refusal, ShapesError::Recursion(refusal) if refusal.reason == RecursionRefusalReason::MissingSelection)
    );

    let owner = ModelOwner::from_definitions(vec![prototype.clone()]).unwrap();
    let weak = Arc::downgrade(&owner);
    let root = owner.root(&id).unwrap();
    assert!(conforms(&data, &focus, &root).unwrap());
    drop((root, owner));
    assert!(
        weak.upgrade().is_none(),
        "immutable admission metadata must not retain model bodies"
    );

    let mut replacement = parsed;
    replacement.owner = ModelOwner::from_definitions(vec![prototype]).unwrap();
    replacement.node_shapes = vec![replacement.owner.root(&id).unwrap()];
    replacement.set_validation_options(ValidationOptions::default());
    assert!(validate_with(&data, &replacement).unwrap().conforms);

    // Keep every eviction model live until the original model was asked again.
    assert_eq!(retained.len(), 40);
    let mut negative = replacement.owner.definition(&id).unwrap().clone();
    let mut handle = negative.clone();
    handle.constraints = vec![Constraint::Reference(id.clone())];
    negative.constraints = vec![Constraint::Not(Box::new(handle))];
    let owner = ModelOwner::from_definitions(vec![negative]).unwrap();
    let root = owner.root(&id).unwrap();
    let before = conforms(&data, &focus, &root).unwrap_err();
    for shape in &retained {
        conforms(&data, &focus, shape).unwrap();
    }
    assert_eq!(conforms(&data, &focus, &root).unwrap_err(), before);

    // Equal names in original inline occurrences must not collapse their edges,
    // and equal names in distinct model owners must retain separate domains.
    let mut local = replacement.owner.definition(&id).unwrap().clone();
    local.constraints = vec![
        Constraint::Reference(id.clone()),
        Constraint::NodeKind(vec![purrdf_shapes::shapes::NodeKindValue::Iri]),
    ];
    let mut definition = local.clone();
    definition.constraints = vec![Constraint::And(vec![local.clone(), local])];
    let mut held = Vec::new();
    for _ in 0..2 {
        let owner = ModelOwner::from_definitions(vec![definition.clone()]).unwrap();
        let weak = Arc::downgrade(&owner);
        let root = owner.root(&id).unwrap();
        let ShapesError::Recursion(refusal) = conforms(&data, &focus, &root).unwrap_err() else {
            panic!("original component was not retained")
        };
        drop((root, owner));
        let retained = refusal.clone();
        drop(refusal);
        assert_eq!(retained.members(), ["<http://example.org/S>"]);
        assert_eq!(retained.definitions().len(), 3);
        assert_eq!(retained.scoped_edges().len(), 4);
        assert!(retained.scoped_edges().len() > retained.edges().len());
        for edge in retained.scoped_edges() {
            let source = &retained.definitions()[edge.source];
            let target = &retained.definitions()[edge.target];
            assert_eq!(source.definition(), &id);
            assert_eq!(target.definition(), &id);
            assert_ne!(source.occurrence(), target.occurrence());
            assert!(Arc::ptr_eq(
                source.model().unwrap(),
                target.model().unwrap()
            ));
        }
        held.push((retained, weak));
    }
    assert!(!Arc::ptr_eq(
        held[0].0.definitions()[0].model().unwrap(),
        held[1].0.definitions()[0].model().unwrap()
    ));
    let weak = held
        .iter()
        .map(|(_, weak)| weak.clone())
        .collect::<Vec<_>>();
    drop(held);
    assert!(weak.iter().all(|weak| weak.upgrade().is_none()));
}

/// An exported handle may precede installation of a function body. A retained
/// failed attempt must not poison later admission after construction finishes.
#[test]
fn admission_retries_an_original_body_that_was_not_installed_at_export() {
    use purrdf_shapes::constraints::conforms;
    use purrdf_shapes::expression::{CustomFnKind, CustomFunction, NodeExpr};
    use purrdf_shapes::shapes::ModelOwner;
    use purrdf_shapes::term::{NamedNode, Term};
    use std::sync::OnceLock;

    let parsed = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:nodeKind sh:IRI ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let id = Term::NamedNode(NamedNode::from("http://example.org/S"));
    let definition = parsed.owner.definition(&id).unwrap().clone();
    let function = Arc::new(CustomFunction {
        iri: NamedNode::from("http://example.org/pending"),
        kind: CustomFnKind::ListParameter,
        params: Vec::new(),
        required: 0,
        body: OnceLock::new(),
    });
    let owner = ModelOwner::from_model(vec![definition], vec![Arc::clone(&function)]).unwrap();
    let root = owner.root(&id).unwrap();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = Term::NamedNode(NamedNode::from("http://example.org/a"));
    assert!(matches!(
        conforms(&data, &focus, &root),
        Err(ShapesError::Preparation(
            purrdf_shapes::PreparationRefusal::MissingFunctionBody(_)
        ))
    ));
    function.body.set(NodeExpr::Empty).unwrap();
    assert!(conforms(&data, &focus, &root).unwrap());
}

#[test]
fn acyclic_target_where_retains_its_anonymous_property_domain() {
    let shapes = parse_shapes_with_options(
        &format!(
            "{PREFIXES}
          :S a sh:NodeShape ;
             sh:targetWhere [ sh:class :Person ;
                 sh:property [ sh:path :age ; sh:minCount 1 ; sh:minInclusive 18 ] ] ;
             sh:property :P .
          :P a sh:PropertyShape ; sh:path :vote ; sh:minCount 1 ."
        ),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a a :Person ; :age 18 ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let report = validate_with(&data, &shapes).unwrap();
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 1);
}

#[test]
fn products_retain_declared_cycles_after_native_projection_drops_the_edge() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    let request = options(RecursionSemantics::Gfp);
    let shapes = parse_shapes_with_options(
        &format!(
            "{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ;
                 sh:node :S {{| sh:deactivated true |}} ."
        ),
        None,
        None,
        None,
        &ShapesImports::new(),
        &request,
    )
    .unwrap();
    let prepared = PreparedShapes::new(Arc::new(shapes));
    let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
    let dataset = parse_turtle_to_dataset("", None).unwrap();
    for rebuild in [false, true] {
        let restore = |options: &ValidationOptions| {
            let view = ShapesProduct::open(&bytes).unwrap();
            if rebuild {
                view.rebuild_with_options(&ShapesProfile::CORE, &HostBindings::empty(), options)
            } else {
                view.admit_with_options(&ShapesProfile::CORE, &HostBindings::empty(), options)
            }
        };
        let restored = restore(&request).unwrap();
        assert!(
            restored
                .bind_dataset(&dataset)
                .unwrap()
                .validate()
                .unwrap()
                .conforms
        );
        let Err(CompleteValidationError::Shapes(ShapesError::Recursion(refusal))) =
            restore(&ValidationOptions::default())
        else {
            panic!(
                "restoration must retain typed source-only recursion evidence, rebuild={rebuild}"
            )
        };
        assert_eq!(refusal.reason, RecursionRefusalReason::MissingSelection);
    }
}

#[test]
fn guarded_recursive_xone_is_monotone_by_disjoint_node_kinds() {
    PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:xone (
                   [ sh:nodeKind sh:IRI ; sh:node :S ]
                   [ sh:nodeKind sh:Literal ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    }
    .check();
}

#[test]
fn guarded_recursive_xone_is_monotone_by_required_and_closed_properties() {
    PaperCase {
        data: ":a :p :a . :b :q :b .",
        shapes: ":S a sh:NodeShape ; sh:xone (
                   [ sh:closed true ; sh:property [ sh:path :p ; sh:minCount 1 ] ; sh:node :S ]
                   [ sh:closed true ; sh:property [ sh:path :q ; sh:minCount 1 ] ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("b", "S", true, true)],
    }
    .check();
}

#[test]
fn guarded_xone_keeps_approximations_monotone_when_both_branches_recur() {
    PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:xone (
                   [ sh:nodeKind sh:IRI ; sh:node :S ]
                   [ sh:nodeKind sh:Literal ; sh:node :S ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    }
    .check();
}

#[test]
fn guarded_recursive_xone_is_monotone_by_list_and_closed_properties() {
    PaperCase {
        data: ":a <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> :a ;
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .",
        shapes: ":S a sh:NodeShape ; sh:xone (
                   [ sh:minListLength 1 ; sh:memberShape :S ]
                   [ sh:closed true ; sh:property [ sh:path :p ; sh:minCount 1 ] ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    }
    .check();
}

#[test]
fn xone_guard_proofs_remain_sound_when_the_request_allows_their_severity() {
    use purrdf_shapes::report::{ConformanceDisallows, Severity};
    let source = format!(
        "{PREFIXES}
        :S a sh:NodeShape ; sh:targetNode :a ; sh:xone (
            [ sh:nodeKind sh:IRI ; sh:node :S ] [ sh:nodeKind sh:Literal ] ) ."
    );
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        for counts in [true, false] {
            let request = options(semantics).with_conformance_disallows(
                ConformanceDisallows::new([if counts {
                    Severity::Violation
                } else {
                    Severity::Warning
                }])
                .unwrap(),
            );
            let shapes = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &request,
            )
            .unwrap();
            assert_eq!(
                validate_with(&data, &shapes).unwrap().conforms,
                !counts || semantics == RecursionSemantics::Gfp
            );
        }
    }
}

#[test]
fn xone_declines_guards_with_annotations_deactivation_or_other_severities() {
    for first in [
        "sh:nodeKind sh:IRI {| sh:severity sh:Warning |}",
        "sh:nodeKind sh:IRI {| sh:deactivated true |}",
        "sh:nodeKind sh:IRI ; sh:severity sh:Warning",
        "sh:nodeKind sh:IRI ; sh:deactivated true",
    ] {
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let source = format!(
                "{PREFIXES} :S a sh:NodeShape ; sh:xone (
                [ {first} ; sh:node :S ] [ sh:nodeKind sh:Literal ] ) ."
            );
            let error = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &options(semantics),
            )
            .unwrap_err();
            assert!(matches!(error,
                CompleteValidationError::Shapes(ShapesError::Recursion(refusal))
                if refusal.reason == RecursionRefusalReason::NonStratified { semantics }));
        }
    }
}

/// A self-supporting qualified minimum separates the least and greatest fixed
/// points. Omitting the selection retains complete original cycle evidence.
#[test]
fn bsep1_requires_a_selection_and_separates_the_two_fixed_points() {
    let source = format!(
        "{PREFIXES}
         :S a sh:NodeShape ; sh:targetNode :a ;
           sh:property [ sh:path :p ; sh:qualifiedValueShape :S ;
                         sh:qualifiedMinCount 1 ] ."
    );
    let load = |request: &ValidationOptions| {
        parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), request)
    };
    let refusal = load(&ValidationOptions::default()).unwrap_err();
    let CompleteValidationError::Shapes(ShapesError::Recursion(refusal)) = refusal else {
        panic!("missing selection must retain the native recursion refusal: {refusal:?}");
    };
    assert_eq!(refusal.reason, RecursionRefusalReason::MissingSelection);
    assert!(
        refusal
            .members()
            .iter()
            .any(|shape| shape == "<http://example.org/S>")
    );
    assert_ne!(
        refusal.edges(),
        [] as [purrdf_shapes::RecursionDependency; 0]
    );

    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for (semantics, expected) in [
        (RecursionSemantics::Lfp, false),
        (RecursionSemantics::Gfp, true),
    ] {
        let shapes = load(&options(semantics))
            .unwrap_or_else(|error| panic!("{semantics:?} must admit the positive cycle: {error}"));
        let report = validate_with(&data, &shapes).unwrap();
        assert_eq!(report.conforms, expected, "{semantics:?}: {report:?}");
    }
}

/// Removing the cycle leaves a conforming qualified value under either selected
/// law and under the ordinary acyclic request with no selected fixed point.
#[test]
fn an_acyclic_qualified_minimum_agrees_under_both_modes_and_no_selection() {
    let source = format!(
        "{PREFIXES}
         :S a sh:NodeShape ; sh:targetNode :a ;
           sh:property [ sh:path :p ; sh:qualifiedValueShape :Leaf ;
                         sh:qualifiedMinCount 1 ] .
         :Leaf sh:nodeKind sh:IRI ."
    );
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for request in [
        ValidationOptions::default(),
        options(RecursionSemantics::Lfp),
        options(RecursionSemantics::Gfp),
    ] {
        let shapes =
            parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &request)
                .unwrap();
        let report = validate_with(&data, &shapes).unwrap();
        assert!(report.conforms, "{request:?}: {report:?}");
        assert!(report.results.is_empty());
    }
}

/// Selecting either fixed point never makes an internal negative or nonmonotone
/// edge stratified. The refusal retains the selected mode and original polarity.
#[test]
fn negative_and_nonmonotone_recursive_components_refuse_both_modes() {
    use purrdf_shapes::RecursionPolarity;

    for (constraint, polarity) in [
        ("sh:not :S", RecursionPolarity::Negative),
        ("sh:xone ( :S )", RecursionPolarity::Both),
        (
            "sh:property [ sh:path :p ; sh:qualifiedValueShape :S ; sh:qualifiedMaxCount 0 ]",
            RecursionPolarity::Negative,
        ),
        (
            "sh:property [ sh:path :p ; sh:qualifiedValueShape :S ;
                           sh:qualifiedMinCount 1 ; sh:qualifiedMaxCount 1 ]",
            RecursionPolarity::Both,
        ),
    ] {
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let source = format!("{PREFIXES} :S a sh:NodeShape ; {constraint} .");
            let error = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &options(semantics),
            )
            .unwrap_err();
            drop(source);
            let CompleteValidationError::Shapes(ShapesError::Recursion(refusal)) = error else {
                panic!("{semantics:?} must retain its native recursion refusal: {error:?}");
            };
            assert_eq!(
                refusal.reason,
                RecursionRefusalReason::NonStratified { semantics }
            );
            assert!(refusal.edges().iter().any(|edge| edge.polarity == polarity));
            assert!(
                refusal
                    .members()
                    .iter()
                    .any(|shape| shape == "<http://example.org/S>")
            );
        }
    }
}

/// Admission and truth observations from the original named cases. The finite
/// matrix is factual test input, never an alternate constraint evaluator.
struct PaperCase {
    data: &'static str,
    shapes: &'static str,
    admission: Admission,
    /// Focus, original definition, LFP answer, GFP answer.
    pairs: &'static [(&'static str, &'static str, bool, bool)],
}

#[derive(Clone, Copy)]
enum Admission {
    Acyclic,
    Positive,
    NonStratified,
}

impl PaperCase {
    fn check(&self) {
        self.check_requests([
            ValidationOptions::default(),
            options(RecursionSemantics::Lfp),
            options(RecursionSemantics::Gfp),
        ]);
    }

    fn check_requests(&self, requests: impl IntoIterator<Item = ValidationOptions>) {
        let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} {}", self.data), None)
            .expect("original factual RDF");
        let projected = purrdf_shapes::engine::project_dataset(&dataset)
            .expect("the production SHACL graph projection includes reifier bindings");
        let data = ShaclData::new(projected, Arc::clone(&dataset), None);
        for request in requests {
            let reason = match (self.admission, request.recursion.as_ref()) {
                (Admission::Acyclic, _) | (Admission::Positive, Some(_)) => None,
                (_, None) => Some(RecursionRefusalReason::MissingSelection),
                (Admission::NonStratified, Some(options)) => {
                    Some(RecursionRefusalReason::NonStratified {
                        semantics: options.semantics,
                    })
                }
            };
            if let Some(reason) = &reason {
                let source = format!("{PREFIXES} {}", self.shapes);
                let error = parse_shapes_with_options(
                    &source,
                    None,
                    None,
                    None,
                    &ShapesImports::new(),
                    &request,
                )
                .expect_err("recursive admission must refuse the complete catalog");
                let CompleteValidationError::Shapes(ShapesError::Recursion(refusal)) = error else {
                    panic!("expected complete native cycle evidence: {error:?}");
                };
                assert_eq!(&refusal.reason, reason);
                assert_ne!(refusal.members(), [] as [String; 0]);
                assert_ne!(
                    refusal.edges(),
                    [] as [purrdf_shapes::RecursionDependency; 0]
                );
                continue;
            }
            // A selector asks original pairs, independently of whether another
            // pair in the same model conforms. Each report has exactly one target.
            for &(node, shape, lfp, gfp) in self.pairs {
                let source = format!(
                    "{PREFIXES} {} :{shape} sh:targetNode :{node} .",
                    self.shapes
                );
                let loaded = parse_shapes_with_options(
                    &source,
                    None,
                    None,
                    None,
                    &ShapesImports::new(),
                    &request,
                );
                let shapes = loaded.expect("admitted original catalog");
                let expected = match request.recursion.as_ref().map(|law| law.semantics) {
                    Some(RecursionSemantics::Gfp) => gfp,
                    Some(RecursionSemantics::Lfp) | None => lfp,
                };
                let report = validate_with(&data, &shapes)
                    .unwrap_or_else(|error| panic!("{node}/{shape} {request:?}: {error}"));
                assert_eq!(report.conforms, expected, "{node}/{shape} {request:?}");
                let prepared = purrdf_shapes::engine::PreparedShapes::new(Arc::new(shapes));
                let bound = prepared
                    .bind_shared_dataset(Arc::clone(&dataset))
                    .expect("prepared native binding");
                let prepared_report = bound.validate().expect("prepared native report");
                assert_eq!(
                    prepared_report
                        .to_ntriples()
                        .expect("prepared report bytes"),
                    report.to_ntriples().expect("direct report bytes"),
                    "{node}/{shape} {request:?}"
                );
            }
        }
    }
}

macro_rules! paper_cases {
    ($($name:ident: $case:expr;)*) => {$ (
        #[test]
        fn $name() {
            $case.check();
        }
    )*};
}

const EXISTENTIAL_SELF: &str = ":S a sh:NodeShape ; sh:property [
    sh:path :p ; sh:qualifiedValueShape :S ; sh:qualifiedMinCount 1 ] .";
const REACH_GRAPH: &str = ":a :p :d . :b :p :a, :c . :c :p :d . :d :p :c .";
const REACH_SHAPE: &str = ":R a sh:NodeShape ; sh:or (
    [ sh:hasValue :a ] [ sh:property [ sh:path :p ;
      sh:qualifiedValueShape :R ; sh:qualifiedMinCount 1 ] ] ) .";
const SAFE_SHAPE: &str = ":S a sh:NodeShape ; sh:not [ sh:hasValue :a ] ;
    sh:property [ sh:path :p ; sh:node :S ] .";
const NEGATIVE_CYCLE: &str = ":S a sh:NodeShape ; sh:property [ sh:path :p ;
    sh:qualifiedValueShape [ sh:not [ sh:node :S ] ] ; sh:qualifiedMinCount 1 ] .";
const INCONSISTENT: &str = ":S a sh:NodeShape ; sh:property [ sh:path :p ;
    sh:qualifiedValueShape :S1 ; sh:qualifiedMinCount 1 ] .
    :S1 a sh:NodeShape ; sh:not :S .";
const DUAL_UNIVERSAL: &str = ":S a sh:NodeShape ; sh:and (
    [ sh:not [ sh:hasValue :b ] ] [ sh:property [ sh:path :p ; sh:node :S ] ] ) .";
const DUAL_EXISTENTIAL: &str = ":S a sh:NodeShape ; sh:or (
    [ sh:hasValue :b ] [ sh:property [ sh:path :p ;
      sh:qualifiedValueShape :S ; sh:qualifiedMinCount 1 ] ] ) .";

paper_cases! {
    original_bsep1: PaperCase {
        data: ":a :p :a .", shapes: EXISTENTIAL_SELF, admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    };
    original_bsep2: PaperCase {
        data: ":a :p :a . :b :p :b .", shapes: EXISTENTIAL_SELF,
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("b", "S", false, true)],
    };
    original_bsep3: PaperCase {
        data: ":a :p :c . :b :p :c .",
        shapes: ":S a sh:NodeShape ; sh:and ( :S1 [ sh:property [
                    sh:path :p ; sh:minCount 1 ] ] ) .
                 :S1 a sh:NodeShape ; sh:and ( :S [ sh:property [
                    sh:path :p ; sh:minCount 1 ] ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("b", "S", false, true)],
    };
    original_bsep4: PaperCase {
        data: ":a :p :a . :b :p :b .",
        shapes: ":S a sh:NodeShape ; sh:property [ sh:path :p ;
                    sh:qualifiedValueShape :S ; sh:qualifiedMinCount 1 ] .
                 :S1 a sh:NodeShape ; sh:not [ sh:node :S ] .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("b", "S", false, true),
                 ("a", "S1", true, false), ("b", "S1", true, false)],
    };
    original_reach1: PaperCase {
        data: REACH_GRAPH, shapes: REACH_SHAPE, admission: Admission::Positive,
        pairs: &[("a", "R", true, true), ("b", "R", true, true),
                 ("c", "R", false, true), ("d", "R", false, true)],
    };
    original_reach2: PaperCase {
        data: REACH_GRAPH,
        shapes: ":S a sh:NodeShape ; sh:not [ sh:hasValue :a ] ;
                    sh:property [ sh:path :p ; sh:node :S ] .
                 :R a sh:NodeShape ; sh:not [ sh:node :S ] .",
        admission: Admission::Positive,
        pairs: &[("c", "R", true, false), ("d", "R", true, false)],
    };
    original_safe1: PaperCase {
        data: REACH_GRAPH, shapes: SAFE_SHAPE, admission: Admission::Positive,
        pairs: &[("c", "S", false, true), ("d", "S", false, true)],
    };
    original_safe2: PaperCase {
        data: REACH_GRAPH,
        shapes: ":R a sh:NodeShape ; sh:or ( [ sh:hasValue :a ]
                    [ sh:property [ sh:path :p ; sh:qualifiedValueShape :R ;
                        sh:qualifiedMinCount 1 ] ] ) .
                 :S a sh:NodeShape ; sh:not [ sh:node :R ] .",
        admission: Admission::Positive,
        pairs: &[("c", "R", false, true), ("d", "R", false, true),
                 ("c", "S", true, false), ("d", "S", true, false)],
    };
    original_nstrat1: PaperCase {
        data: ":a :p :b . :b :p :c . :c :p :d . :d :p :e .",
        shapes: NEGATIVE_CYCLE, admission: Admission::NonStratified,
        pairs: &[],
    };
    original_nstrat2: PaperCase {
        data: ":a :p :b . :b :p :a .", shapes: NEGATIVE_CYCLE,
        admission: Admission::NonStratified,
        pairs: &[],
    };
    original_cons1: PaperCase {
        data: ":a :p :a .", shapes: INCONSISTENT, admission: Admission::NonStratified,
        pairs: &[],
    };
    original_cons2: PaperCase {
        data: ":a :p :b . :b :p :a .", shapes: INCONSISTENT,
        admission: Admission::NonStratified,
        pairs: &[],
    };
    original_fresh: PaperCase {
        data: ":a :p :b . :b :p :c . :c :p :a .", shapes: ":S a sh:NodeShape .",
        admission: Admission::Acyclic, pairs: &[("d", "S", true, true)],
    };
    // The original dual1 RDF omits d's self-loop, despite its manifest's LFP
    // expectation and the paper's graph. Preserve that input: both leaf focuses
    // c and d satisfy the universal shape in the first round. Separate tests
    // below exercise the paper's actual dual pair without altering this fixture.
    original_dual1_graph: PaperCase {
        data: ":a :p :b . :b :p :a ; :q :d .", shapes: DUAL_UNIVERSAL,
        admission: Admission::Positive,
        pairs: &[("a", "S", false, false), ("b", "S", false, false),
                 ("c", "S", true, true), ("d", "S", true, true)],
    };
    original_dual2_graph: PaperCase {
        data: ":a :p :b . :b :p :a ; :q :d . :d :p :d .",
        shapes: DUAL_EXISTENTIAL, admission: Admission::Positive,
        pairs: &[("a", "S", true, true), ("b", "S", true, true),
                 ("c", "S", false, false), ("d", "S", false, true)],
    };
    original_norec1: PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:property [ sh:path :p ;
                    sh:qualifiedValueShape :T ; sh:qualifiedMinCount 1 ] .
                 :T a sh:NodeShape ; sh:hasValue :a .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", true, true), ("a", "T", true, true)],
    };
    original_norec2: PaperCase {
        data: ":a :p :a . :b :p :b .",
        shapes: ":S a sh:NodeShape ; sh:property [ sh:path :p ;
                    sh:qualifiedValueShape [ sh:not [ sh:node :T ] ] ;
                    sh:qualifiedMinCount 1 ] .
                 :T a sh:NodeShape ; sh:hasValue :a .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", false, false), ("b", "S", true, true),
                 ("a", "T", true, true), ("b", "T", false, false)],
    };
    original_example_2: PaperCase {
        data: ":a :name \"Alice\" ; :knows :c . :b :name \"Bob\" ; :knows :c .
               :c :name \"Carol\" ; :knows :a .",
        shapes: ":User a sh:NodeShape ; sh:and ( :Person
                    [ sh:property [ sh:path :name ; sh:datatype xsd:string ; sh:minCount 1 ] ]
                    [ sh:property [ sh:path :knows ; sh:minCount 1 ] ] ) .
                 :Person a sh:NodeShape ; sh:and ( :User
                    [ sh:property [ sh:path :knows ; sh:node :Person ] ] ) .",
        admission: Admission::Positive,
        pairs: &[("a", "User", false, true), ("b", "User", false, true),
                 ("c", "User", false, true)],
    };
    paper_dual1: PaperCase {
        data: ":a :p :b . :b :p :a ; :q :c . :d :p :d .",
        shapes: DUAL_UNIVERSAL, admission: Admission::Positive,
        pairs: &[("a", "S", false, false), ("b", "S", false, false),
                 ("c", "S", true, true), ("d", "S", false, true)],
    };
    paper_dual2: PaperCase {
        data: ":a :p :b . :b :p :a ; :q :c . :d :p :d .",
        shapes: DUAL_EXISTENTIAL, admission: Admission::Positive,
        pairs: &[("a", "S", true, true), ("b", "S", true, true),
                 ("c", "S", false, false), ("d", "S", false, true)],
    };
}

/// Finite work refusals name the exhausted resource and retain its actual
/// consumption. Neither a partial report nor a conformance answer is returned.
#[test]
fn recursive_work_limits_are_typed_before_a_partial_report() {
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let source = format!("{PREFIXES} {EXISTENTIAL_SELF} :S sh:targetNode :a .");
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        for (max_pairs, max_iterations, expected) in [
            (
                0,
                4_096,
                RecursionRefusalReason::PairLimit {
                    limit: 0,
                    consumed: 0,
                },
            ),
            (
                4_096,
                0,
                RecursionRefusalReason::IterationLimit {
                    limit: 0,
                    consumed: 0,
                },
            ),
        ] {
            let request = ValidationOptions::default().with_recursion(RecursionOptions {
                semantics,
                max_pairs,
                max_iterations,
            });
            let shapes = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &request,
            )
            .unwrap();
            let ShapesError::Recursion(refusal) = validate_with(&data, &shapes).unwrap_err() else {
                panic!("work refusal must retain its original native reason");
            };
            assert_eq!(refusal.reason, expected);
        }
    }
}

/// A caller holding an exported original shape can select its request directly,
/// with the same fixed-point answer as the full report entry point.
#[test]
fn direct_conformance_and_reports_take_explicit_recursion() {
    use purrdf_shapes::constraints::{conforms_with_recursion, validate_shape_with_recursion};
    use purrdf_shapes::term::NamedNode;

    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let source = format!("{PREFIXES} {EXISTENTIAL_SELF} :S sh:targetNode :a .");
    let focus = NamedNode::from("http://example.org/a").into_term();
    let shape_id = NamedNode::from("http://example.org/S").into_term();
    for (semantics, expected) in [
        (RecursionSemantics::Lfp, false),
        (RecursionSemantics::Gfp, true),
    ] {
        let request = options(semantics);
        let shapes =
            parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &request)
                .unwrap();
        let shape = shapes
            .node_shapes
            .iter()
            .find(|shape| shape.id == shape_id)
            .expect("original exported S");
        assert_eq!(
            conforms_with_recursion(&data, &focus, shape, request.recursion.as_ref().unwrap())
                .unwrap(),
            expected
        );
        let results = validate_shape_with_recursion(
            &data,
            &focus,
            shape,
            None,
            request.recursion.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(results.report.conforms, expected);
        assert_eq!(results.report.results.is_empty(), expected);
        assert_eq!(validate_with(&data, &shapes).unwrap().conforms, expected);
    }
}

/// A selected law does not manufacture iterative work for an acyclic component.
/// Its direct leaf evaluation consumes a pair but no recursive round.
#[test]
fn acyclic_selected_requests_need_no_recursive_rounds() {
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let source = format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ; sh:nodeKind sh:IRI .");
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let request = ValidationOptions::default().with_recursion(RecursionOptions {
            semantics,
            max_pairs: 4_096,
            max_iterations: 0,
        });
        let shapes =
            parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &request)
                .unwrap();
        assert!(validate_with(&data, &shapes).unwrap().conforms);
    }
}

/// GFP keeps mutually supporting declarations only when their nonrecursive
/// obligations actually hold. Removing a name must propagate through the cycle.
#[test]
fn recursive_person_does_not_hide_a_missing_name() {
    for data in [
        ":a :knows :c . :c :name \"Carol\" ; :knows :a .",
        ":a :name \"Alice\" ; :knows :c . :c :knows :a .",
    ] {
        PaperCase {
            data,
            shapes: ":User a sh:NodeShape ; sh:and ( :Person
                        [ sh:property [ sh:path :name ; sh:datatype xsd:string ; sh:minCount 1 ] ]
                        [ sh:property [ sh:path :knows ; sh:minCount 1 ] ] ) .
                     :Person a sh:NodeShape ; sh:and ( :User
                        [ sh:property [ sh:path :knows ; sh:node :Person ] ] ) .",
            admission: Admission::Positive,
            pairs: &[("a", "User", false, false), ("c", "User", false, false)],
        }
        .check();
    }
}

/// Data depth grows on the heap work list, independently of native call depth.
/// The terminating leaf makes every predecessor conform under either law.
#[test]
fn deep_data_chains_settle_on_a_small_native_stack() {
    use std::fmt::Write;

    let mut source = String::from(PREFIXES);
    for index in 0..512 {
        write!(source, ":n{index} :p :n{} .", index + 1).unwrap();
    }
    let dataset = parse_turtle_to_dataset(&source, None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let source = format!(
        "{PREFIXES} :S a sh:NodeShape ; sh:targetNode :n0 ;
                          sh:property [ sh:path :p ; sh:node :S ] ."
    );
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let request = ValidationOptions::default().with_recursion(RecursionOptions {
            semantics,
            max_pairs: 32_768,
            max_iterations: 1_000_000,
        });
        let shapes =
            parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &request)
                .unwrap();
        let report = purrdf_stack::on_stack_scoped(512 * 1_024, || validate_with(&data, &shapes))
            .expect("small native validation stack")
            .expect("finite data chain");
        assert!(report.conforms, "{semantics:?}");
        assert!(report.results.is_empty());
    }
}

/// A property identity edge is part of the recursive formula even when no
/// logical node constraint wraps it. Its report must have finite original-edge
/// evidence rather than unfolding that edge until the stack guard refuses.
#[test]
fn recursive_property_identity_edges_have_finite_reports() {
    PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:property :P .
                 :P sh:path :p ; sh:property :P .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    }
    .check();
    PaperCase {
        data: ":a :p :a . :r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( :a :p :a )>> .",
        shapes: ":S a sh:NodeShape ; sh:property :P .
                 :P sh:path :p ; sh:reifierShape :R .
                 :R a sh:NodeShape ; sh:node :R .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    }
    .check();
    PaperCase {
        data: "",
        shapes: ":S a sh:NodeShape ; sh:property :P {| sh:deactivated true ; sh:message \"disabled\" |} .
                 :T a sh:NodeShape ; sh:property :P .
                 :P sh:path :p ; sh:minCount 1 .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", true, true), ("a", "T", false, false)],
    }
    .check();
}

#[test]
fn property_cycle_certificates_retain_original_edges_models_and_data() {
    use purrdf_shapes::constraints::validate_shape_with_recursion;
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::term::NamedNode;
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let request = options(semantics);
        let shapes = parse_shapes_with_options(
            &format!(
                "{PREFIXES}
            :S a sh:NodeShape ; sh:targetNode :a ; sh:property :P .
            :P sh:path :p ; sh:property :P {{| sh:message \"original cycle edge\" |}} ."
            ),
            None,
            None,
            None,
            &ShapesImports::new(),
            &request,
        )
        .unwrap();
        let weak = Arc::downgrade(&shapes.owner);
        let focus = NamedNode::from("http://example.org/a").into_term();
        let root = &shapes.node_shapes[0];
        let direct = validate_shape_with_recursion(
            &data,
            &focus,
            root,
            None,
            request.recursion.as_ref().unwrap(),
        )
        .unwrap();
        let prepared = PreparedShapes::new(Arc::new(shapes));
        let binding = prepared
            .bind_complete_shared_dataset(Arc::clone(&dataset))
            .unwrap();
        let complete = binding.validate().unwrap();
        let graph = complete.to_graph().unwrap();
        let expected = semantics == RecursionSemantics::Gfp;
        assert_eq!(direct.report.conforms, expected);
        assert_eq!(complete.legacy().conforms, expected);
        assert!(direct.report.results.is_empty());
        assert!(complete.legacy().results.is_empty());
        assert_eq!(
            purrdf_shapes::report::conforms_from_dataset(graph.dataset()),
            Some(expected)
        );
        for cycles in [direct.cycles.as_slice(), complete.cycles(), graph.cycles()] {
            assert!(!cycles.is_empty());
            for cycle in cycles {
                assert_eq!(cycle.semantics, semantics);
                assert_eq!(cycle.conforms, expected);
                assert!(cycle.positions.len() >= 2);
                let first = cycle.positions.first().unwrap();
                let last = cycle.positions.last().unwrap();
                assert_eq!(first.focus().unwrap(), focus);
                assert_eq!(first.focus().unwrap(), last.focus().unwrap());
                assert_eq!(first.shape().unwrap().id, last.shape().unwrap().id);
                assert!(Arc::ptr_eq(first.model(), last.model()));
                let edge = last.property_edge().unwrap().unwrap();
                assert_eq!(edge.messages[0].value(), "original cycle edge");
            }
        }
        drop((binding, prepared));
        assert!(
            weak.upgrade().is_some(),
            "certificates retain the original models"
        );
        assert_eq!(direct.cycles[0].positions[0].focus().unwrap(), focus);
        drop((direct, complete));
        assert_eq!(graph.cycles()[0].positions[0].focus().unwrap(), focus);
        drop(graph);
        assert!(
            weak.upgrade().is_none(),
            "dropping certificates releases their models"
        );
    }
}

#[test]
fn native_compiled_reports_keep_cycles_without_an_rdf_shapes_document() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::shapes::{ModelOwner, Shapes};
    use purrdf_shapes::term::NamedNode;
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let parsed = parse_shapes_with_options(
        &format!(
            "{PREFIXES}
        :S a sh:NodeShape ; sh:targetNode :a ; sh:property :P .
        :P sh:path :p ; sh:property :P ."
        ),
        None,
        None,
        None,
        &ShapesImports::new(),
        &options(RecursionSemantics::Gfp),
    )
    .unwrap();
    let id = NamedNode::from("http://example.org/S").into_term();
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let owner =
            ModelOwner::from_definitions(parsed.owner.definitions().cloned().collect()).unwrap();
        let root = owner.root(&id).unwrap();
        let mut shapes = Shapes::default();
        shapes.owner = owner;
        shapes.node_shapes = vec![root];
        shapes.set_validation_options(options(semantics));
        let report = PreparedShapes::new(Arc::new(shapes))
            .bind_dataset(&dataset)
            .unwrap()
            .validate_with_recursion_evidence()
            .unwrap();
        assert_eq!(report.report.conforms, semantics == RecursionSemantics::Gfp);
        assert!(report.report.results.is_empty());
        assert!(!report.cycles.is_empty());
        assert_eq!(report.cycles[0].conforms, report.report.conforms);
    }
}

/// A newly demanded definition outlives the temporary native expression plan
/// that first asked it; its original body and data namespace stay retained.
///
/// Every selected entry proves the original scope before doing validation work;
/// published refusals retain that scope while the weak-key cache does not.
#[test]
fn selected_entries_refuse_unproved_original_scopes() {
    use purrdf_shapes::PreparationRefusal;
    use purrdf_shapes::constraints::{conforms_with_recursion, validate_shape_with_recursion};
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    use purrdf_shapes::term::NamedNode;

    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    let id = NamedNode::from("http://example.org/S").into_term();
    let focus = NamedNode::from("http://example.org/foreign").into_term();
    let cases = [
        (
            "sh:expression sh:this",
            purrdf_shapes::model::sh::EXPRESSION,
            false,
        ),
        (
            "sh:nodeByExpression :T",
            purrdf_shapes::model::sh::NODE_BY_EXPRESSION,
            true,
        ),
        (
            "sh:sparql [ sh:select \"SELECT ?this WHERE {}\" ]",
            purrdf_shapes::model::sh::SPARQL,
            false,
        ),
        (
            "sh:targetWhere [ sh:nodeKind sh:IRI ]",
            purrdf_shapes::model::sh::TARGET_WHERE,
            false,
        ),
    ];
    for (body, parameter, indexed) in cases {
        let source = format!(
            "{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ; {body} . :T a sh:NodeShape ."
        );
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = options(semantics);
            assert!(matches!(
                parse_shapes_with_options(
                    &source,
                    None,
                    None,
                    None,
                    &ShapesImports::new(),
                    &request
                ),
                Err(CompleteValidationError::Shapes(ShapesError::Preparation(
                    PreparationRefusal::UnqualifiedRecursion(_)
                )))
            ));
            let shapes = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &ValidationOptions::default(),
            )
            .unwrap();
            let root = shapes.owner.root(&id).unwrap();
            let weak = Arc::downgrade(&shapes.owner);
            let recursion = request.recursion.as_ref().unwrap();
            let direct = conforms_with_recursion(&data, &focus, &root, recursion).unwrap_err();
            let report =
                validate_shape_with_recursion(&data, &focus, &root, None, recursion).unwrap_err();
            let prepared = PreparedShapes::new(Arc::new(shapes));
            let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
            for rebuild in [false, true] {
                let product = ShapesProduct::open(&bytes).unwrap();
                let restored = if rebuild {
                    product.rebuild_with_options(
                        &ShapesProfile::CORE,
                        &HostBindings::empty(),
                        &request,
                    )
                } else {
                    product.admit_with_options(
                        &ShapesProfile::CORE,
                        &HostBindings::empty(),
                        &request,
                    )
                };
                let Err(CompleteValidationError::Shapes(ShapesError::Preparation(
                    PreparationRefusal::UnqualifiedRecursion(refusal),
                ))) = restored
                else {
                    panic!("restoration lost the typed original scope, rebuild={rebuild}")
                };
                assert_eq!(refusal.definition(), &id);
                assert_eq!(refusal.parameter(), parameter);
                assert_eq!(refusal.index().is_some(), indexed);
                assert!(refusal.model().unwrap().definition(&id).is_some());
                let classified = purrdf_shapes::product::ShapesProductError::from(
                    PreparationRefusal::UnqualifiedRecursion(refusal),
                );
                assert_eq!(
                    classified.dimension(),
                    purrdf_shapes::product::ProductDimension::UnsupportedCapability
                );
                assert!(matches!(
                    classified.preparation_refusal(),
                    Some(PreparationRefusal::UnqualifiedRecursion(original))
                        if original.definition() == &id && original.parameter() == parameter
                ));
            }
            let prepared = prepared.with_validation_options(request);
            let Err(bound) = prepared.bind_dataset(&dataset) else {
                panic!("unproved scope was bound")
            };
            drop((prepared, root));
            for error in [&direct, &report, &bound] {
                let ShapesError::Preparation(PreparationRefusal::UnqualifiedRecursion(refusal)) =
                    error
                else {
                    panic!("wrong boundary refusal: {error:?}")
                };
                assert_eq!(refusal.definition(), &id);
                assert_eq!(refusal.parameter(), parameter);
                assert_eq!(refusal.index().is_some(), indexed);
                assert!(Arc::ptr_eq(
                    refusal.model().unwrap(),
                    &weak.upgrade().unwrap()
                ));
                assert!(refusal.model().unwrap().definition(&id).is_some());
            }
            drop((direct, report, bound));
            assert!(
                weak.upgrade().is_none(),
                "closed proof failures must not retain their cache key"
            );
        }
    }
    let shapes = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ; sh:nodeKind sh:Literal ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let original = shapes.owner.root(&id).unwrap();
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let recursion = options(semantics).recursion.unwrap();
        assert!(!conforms_with_recursion(&data, &focus, &original, &recursion).unwrap());
        for overlay in 0..3 {
            let mut root = original.clone();
            match overlay {
                0 => root.deactivated = true,
                1 => root.severity = purrdf_shapes::report::Severity::Info,
                _ => root
                    .messages
                    .push(purrdf_shapes::term::Literal::new_simple_literal(
                        "local override",
                    )),
            }
            for error in [
                conforms_with_recursion(&data, &focus, &root, &recursion).unwrap_err(),
                validate_shape_with_recursion(&data, &focus, &root, None, &recursion).unwrap_err(),
            ] {
                let ShapesError::Preparation(PreparationRefusal::UnqualifiedRecursion(refusal)) =
                    error
                else {
                    panic!("overlay lost its original scope")
                };
                assert_eq!(refusal.definition(), &id);
                assert_eq!(refusal.parameter(), "original-model-occurrence");
                assert!(Arc::ptr_eq(refusal.model().unwrap(), &shapes.owner));
            }
        }
        let mut definition = shapes.owner.definition(&id).unwrap().clone();
        definition.deactivated = true;
        let owner = purrdf_shapes::shapes::ModelOwner::from_definitions(vec![definition]).unwrap();
        let root = owner.root(&id).unwrap();
        assert!(conforms_with_recursion(&data, &focus, &root, &recursion).unwrap());
        assert!(
            validate_shape_with_recursion(&data, &focus, &root, None, &recursion)
                .unwrap()
                .report
                .conforms
        );
    }
}

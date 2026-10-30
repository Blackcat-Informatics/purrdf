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

/// Shared admission keeps source-only edges and distinct original model domains.
/// Its retained metadata must not keep model bodies or data graphs alive.
#[test]
fn admission_metadata_keeps_original_domains_without_retaining_model_bodies() {
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
    assert!(conforms(&data, &focus, &root).unwrap());
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
        panic!("source-declared recursion still requires a selection with shared metadata")
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

    let mut negative = replacement.owner.definition(&id).unwrap().clone();
    let mut handle = negative.clone();
    handle.constraints = vec![Constraint::Reference(id.clone())];
    negative.constraints = vec![Constraint::Not(Box::new(handle))];
    let owner = ModelOwner::from_definitions(vec![negative]).unwrap();
    let root = owner.root(&id).unwrap();
    let before = conforms(&data, &focus, &root).unwrap_err();
    assert!(conforms(&data, &focus, &replacement.node_shapes[0]).unwrap());
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
    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a a :Person ; :age 18 ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for operand in [
        "sh:class :Person ; sh:property [ sh:path :age ; sh:minCount 1 ; sh:minInclusive 18 ]",
        "sh:path :age ; sh:minCount 1 ; sh:minInclusive 18",
    ] {
        for request in [
            ValidationOptions::default(),
            options(RecursionSemantics::Lfp),
            options(RecursionSemantics::Gfp),
        ] {
            let shapes = parse_shapes_with_options(
                &format!(
                    "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere [ {operand} ] ;
                    sh:property :P . :P a sh:PropertyShape ; sh:path :vote ; sh:minCount 1 ."
                ),
                None,
                None,
                None,
                &ShapesImports::new(),
                &request,
            )
            .unwrap();
            let report = validate_with(&data, &shapes).unwrap();
            assert!(!report.conforms);
            assert_eq!(report.results.len(), 1);
            assert_eq!(
                report.results[0].focus_node.to_string(),
                "<http://example.org/a>"
            );
        }
    }
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
            "sh:expression [ shnex:conformsToShape ( sh:this [ sh:not :S ] ) ]",
            RecursionPolarity::Negative,
        ),
        (
            "sh:expression [ shnex:conformsToShape ( sh:this [ sh:xone ( :S ) ] ) ]",
            RecursionPolarity::Both,
        ),
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
const NAMED_CONFORMANCE_SELF: &str = ":S a sh:NodeShape ;
    sh:expression [ shnex:conformsToShape ( sh:this :S ) ] .";
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
    named_conformance_acyclic: PaperCase {
        data: ":a :p :b .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this :T ) ] .
                 :C a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( :b :T ) ] .
                 :T a sh:NodeShape ; sh:hasValue :b .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", false, false), ("b", "S", true, true),
                 ("a", "C", true, true), ("b", "C", true, true)],
    };
    named_conformance_self: PaperCase {
        data: ":a :p :a .", shapes: NAMED_CONFORMANCE_SELF,
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true)],
    };
    named_conformance_mutual: PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this :T ) ] .
                 :T a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this :S ) ] .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("a", "T", false, true)],
    };
    named_conformance_property_values: PaperCase {
        data: ":a :p :b . :b :p :b .",
        shapes: ":S a sh:NodeShape ; sh:property :P .
                 :P a sh:PropertyShape ; sh:path :p ;
                    sh:expression [ shnex:conformsToShape ( sh:this :T ) ] .
                 :T a sh:NodeShape ; sh:hasValue :b ;
                    sh:expression [ shnex:conformsToShape ( sh:this :S ) ] .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, true), ("a", "T", false, false),
                 ("b", "S", false, true)],
    };
    named_conformance_negative_cycle: PaperCase {
        data: ":a :p :a .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this :T ) ] .
                 :T a sh:NodeShape ; sh:not :S .",
        admission: Admission::NonStratified,
        pairs: &[],
    };
    static_conformance_typed_empty_operands: PaperCase {
        data: ":a :p :b .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this [ a sh:NodeShape ] ) ] ; sh:expression [ shnex:conformsToShape ( sh:this [ a sh:PropertyShape ; sh:path :age ] ) ] .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", true, true), ("b", "S", true, true)],
    };
    static_conformance_anonymous_node: PaperCase {
        data: ":a :p :b .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this [ sh:hasValue :b ] ) ] .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", false, false), ("b", "S", true, true)],
    };
    static_conformance_property_value_focus: PaperCase {
        data: ":a :p :b ; :age 10 . :b :age 18 . :c :p :d ; :age 18 . :d :age 10 .",
        shapes: ":S a sh:NodeShape ; sh:property [ sh:path :p ; sh:expression [ shnex:conformsToShape ( sh:this [ sh:path :age ; sh:minCount 1 ; sh:minInclusive 18 ] ) ] ] .",
        admission: Admission::Acyclic,
        pairs: &[("a", "S", true, true), ("c", "S", false, false)],
    };
    static_conformance_lower_stratum: PaperCase {
        data: ":a :p :b . :b :p :c .",
        shapes: ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] .
                 _:t sh:or ( [ sh:hasValue :b ] [ sh:node _:t ] ) ; sh:not :Bad .
                 :Bad sh:hasValue :a .",
        admission: Admission::Positive,
        pairs: &[("a", "S", false, false), ("b", "S", true, true), ("c", "S", false, true)],
    };
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
    for body in [EXISTENTIAL_SELF, NAMED_CONFORMANCE_SELF] {
        let source = format!("{PREFIXES} {body} :S sh:targetNode :a .");
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
                let ShapesError::Recursion(refusal) = validate_with(&data, &shapes).unwrap_err()
                else {
                    panic!("work refusal must retain its original native reason");
                };
                assert_eq!(refusal.reason, expected);
            }
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

/// The expression asks the original target through every native consumer. Its
/// false report remains an expression violation at the original source/focus;
/// restoring a product must retain the request and the same report bytes.
#[test]
fn named_conformance_keeps_original_reports_through_product_restore() {
    use purrdf_shapes::constraints::{conforms_with_recursion, validate_shape_with_recursion};
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    use purrdf_shapes::term::NamedNode;

    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b . :b :p :b ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    let id = NamedNode::from("http://example.org/S").into_term();
    let focus = NamedNode::from("http://example.org/a").into_term();
    let fresh_dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a :fresh :other ."), None).unwrap();
    let fresh_data = ShaclData::new(Arc::clone(&fresh_dataset), Arc::clone(&fresh_dataset), None);
    for (source, reads_p) in [
        (NAMED_CONFORMANCE_SELF, false),
        (
            ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( [ shnex:pathValues :p ] _:t ) ] . _:t sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] ; sh:hasValue :b ; sh:property [ sh:path :p ; sh:minCount 1 ] .",
            true,
        ),
        (
            ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( [ shnex:var \"value\" ] _:t ) ] . _:t sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] ; sh:property [ sh:path :p ; sh:minCount 1 ] .",
            true,
        ),
        (
            ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] . _:t sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] ; sh:property [ sh:path :p ; sh:minCount 1 ] .",
            true,
        ),
        (
            ":S a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this _:t ) ] . _:t sh:expression [ shnex:conformsToShape ( sh:this :S ) ] ; sh:property [ sh:path :p ; sh:minCount 1 ] .",
            true,
        ),
    ] {
        assert!(
            matches!(parse_shapes_with_options(&format!("{PREFIXES} {source}"), None, None, None, &ShapesImports::new(), &ValidationOptions::default()), Err(CompleteValidationError::Shapes(ShapesError::Recursion(refusal))) if refusal.reason == RecursionRefusalReason::MissingSelection)
        );
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = options(semantics);
            let shapes = parse_shapes_with_options(
                &format!("{PREFIXES} {source} :S sh:targetNode :a ."),
                None,
                None,
                None,
                &ShapesImports::new(),
                &request,
            )
            .unwrap();
            let weak = Arc::downgrade(&shapes.owner);
            let root = shapes.owner.root(&id).unwrap();
            let expected = semantics == RecursionSemantics::Gfp;
            assert_eq!(
                conforms_with_recursion(&data, &focus, &root, request.recursion.as_ref().unwrap())
                    .unwrap(),
                expected
            );
            let direct = validate_shape_with_recursion(
                &data,
                &focus,
                &root,
                None,
                request.recursion.as_ref().unwrap(),
            )
            .unwrap();
            assert_eq!(direct.report.conforms, expected);
            assert_eq!(direct.report.results.len(), usize::from(!expected));
            if let Some(result) = direct.report.results.first() {
                assert_eq!(result.source_shape, id);
                assert_eq!(result.focus_node, focus);
                assert_eq!(result.value.as_ref(), Some(&focus));
                assert_eq!(
                    result.source_constraint_component.as_str(),
                    purrdf_shapes::model::sh::EXPRESSION_CONSTRAINT_COMPONENT
                );
            }
            let expected_bytes = validate_with(&data, &shapes)
                .unwrap()
                .to_ntriples()
                .unwrap();
            let fresh_report = validate_with(&fresh_data, &shapes).unwrap();
            let fresh_expected = expected && !reads_p;
            assert_eq!(
                fresh_report.conforms, fresh_expected,
                "the fresh graph has no :p"
            );
            assert_eq!(fresh_report.results.len(), usize::from(!fresh_expected));
            let fresh_bytes = fresh_report.to_ntriples().unwrap();
            let prepared = PreparedShapes::new(Arc::new(shapes));
            let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
            assert_eq!(
                prepared
                    .bind_dataset(&dataset)
                    .unwrap()
                    .validate()
                    .unwrap()
                    .to_ntriples()
                    .unwrap(),
                expected_bytes
            );
            assert_eq!(
                prepared
                    .bind_dataset(&fresh_dataset)
                    .unwrap()
                    .validate()
                    .unwrap()
                    .to_ntriples()
                    .unwrap(),
                fresh_bytes
            );
            drop((root, prepared));
            assert!(
                weak.upgrade().is_none(),
                "neither the admission cache nor the expression result owns a hidden model"
            );
            for rebuild in [false, true] {
                let restore = |options: &ValidationOptions| {
                    let product = ShapesProduct::open(&bytes).unwrap();
                    if rebuild {
                        product.rebuild_with_options(
                            &ShapesProfile::CORE,
                            &HostBindings::empty(),
                            options,
                        )
                    } else {
                        product.admit_with_options(
                            &ShapesProfile::CORE,
                            &HostBindings::empty(),
                            options,
                        )
                    }
                };
                assert!(
                    matches!(restore(&ValidationOptions::default()), Err(CompleteValidationError::Shapes(ShapesError::Recursion(refusal))) if refusal.reason == RecursionRefusalReason::MissingSelection)
                );
                let restored = restore(&request).unwrap();
                for (dataset, expected_conforms, expected_report) in [
                    (Arc::clone(&dataset), expected, &expected_bytes),
                    (Arc::clone(&fresh_dataset), fresh_expected, &fresh_bytes),
                ] {
                    let bound = restored.bind_dataset(&dataset).unwrap();
                    let report = bound.validate().unwrap();
                    assert_eq!(
                        bound
                            .validate_focus_nodes(std::slice::from_ref(&focus))
                            .unwrap()
                            .to_ntriples()
                            .unwrap(),
                        *expected_report
                    );
                    assert_eq!(report.conforms, expected_conforms);
                    assert_eq!(
                        report.to_ntriples().unwrap(),
                        *expected_report,
                        "rebuild={rebuild}"
                    );
                }
                assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
            }
        }
    }
}

/// A new target pair is charged to this execution. Refusing it propagates out
/// of the expression; one more admitted pair evaluates the original mutual cycle.
#[test]
fn named_conformance_charges_callback_pairs_to_the_active_run() {
    let source = format!("{PREFIXES}
        :S a sh:NodeShape ; sh:targetNode :a ; sh:expression [ shnex:conformsToShape ( sh:this :T ) ] .
        :T a sh:NodeShape ; sh:expression [ shnex:conformsToShape ( sh:this :S ) ] .");
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for (target, focus) in [
        (":T", "sh:this"),
        ("_:t", "sh:this"),
        (":T", ":a"),
        ("_:t", "[ shnex:pathValues :p ]"),
    ] {
        let source = source.replace(":T", target).replace("sh:this", focus);
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let no_rounds = ValidationOptions::default().with_recursion(RecursionOptions {
                semantics,
                max_pairs: 2,
                max_iterations: 0,
            });
            let shapes = parse_shapes_with_options(
                &source,
                None,
                None,
                None,
                &ShapesImports::new(),
                &no_rounds,
            )
            .unwrap();
            assert!(
                matches!(validate_with(&data, &shapes), Err(ShapesError::Recursion(refusal)) if matches!(refusal.reason, RecursionRefusalReason::IterationLimit { limit: 0, .. }))
            );
            for max_pairs in [1, 2] {
                let request = ValidationOptions::default().with_recursion(RecursionOptions {
                    semantics,
                    max_pairs,
                    max_iterations: 4_096,
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
                let actual = validate_with(&data, &shapes);
                if max_pairs == 1 {
                    assert!(
                        matches!(actual, Err(ShapesError::Recursion(refusal)) if refusal.reason == RecursionRefusalReason::PairLimit { limit: 1, consumed: 1 })
                    );
                } else {
                    assert_eq!(
                        actual.unwrap().conforms,
                        semantics == RecursionSemantics::Gfp
                    );
                }
            }
        }
    }
}

/// Every pure constructor can select a pair in the active relation. The target
/// self-cycle separates LFP/GFP even for foreign and synthesized focus terms.
#[test]
fn pure_conformance_focus_compositions_use_the_selected_relation() {
    use purrdf_shapes::constraints::conforms_with_recursion;
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::expression::{NodeExpr as E, ShapeArg};
    use purrdf_shapes::shapes::{Constraint, ModelOwner, Path};
    use purrdf_shapes::term::NamedNode;

    let mut shapes = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let sid = NamedNode::from("http://example.org/S").into_term();
    let tid = NamedNode::from("http://example.org/T").into_term();
    let a = NamedNode::from("http://example.org/a").into_term();
    let b = NamedNode::from("http://example.org/b").into_term();
    let p = Path::Predicate(NamedNode::from("http://example.org/p"));
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b ; a :C ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
    let prototype = shapes.owner.definition(&sid).unwrap().clone();
    let mut target = prototype.clone();
    target.id = tid.clone();
    target.targets.clear();
    target.constraints = vec![Constraint::Reference(tid)];
    for (label, node, has_focus) in [
        (
            "constant foreign",
            E::Constant(NamedNode::from("http://example.org/foreign").into_term()),
            true,
        ),
        ("this", E::This, true),
        ("path", E::Path(p.clone()), true),
        ("empty", E::Empty, false),
        ("value variable", E::Var("value".to_owned()), true),
        ("focus variable", E::Var("focusNode".to_owned()), true),
        ("unbound variable", E::Var("unbound".to_owned()), false),
        ("list", E::List(vec![a.clone()]), true),
        (
            "union",
            E::Union(vec![E::This, E::List(vec![a.clone()])]),
            true,
        ),
        (
            "intersection",
            E::Intersection(vec![E::This, E::List(vec![a.clone()])]),
            true,
        ),
        (
            "if",
            E::If {
                cond: Box::new(E::Exists(Box::new(E::This))),
                then: Box::new(E::This),
                els: Box::new(E::Empty),
            },
            true,
        ),
        (
            "count",
            E::Count {
                distinct: true,
                of: Box::new(E::Empty),
            },
            true,
        ),
        (
            "distinct",
            E::Distinct(Box::new(E::Concat(vec![E::This, E::This]))),
            true,
        ),
        ("min", E::Min(Box::new(E::List(vec![a.clone()]))), true),
        ("max", E::Max(Box::new(E::List(vec![a.clone()]))), true),
        ("sum", E::Sum(Box::new(E::Empty)), true),
        (
            "limit",
            E::Limit {
                of: Box::new(E::List(vec![a.clone(), b.clone()])),
                n: 1,
            },
            true,
        ),
        (
            "offset",
            E::Offset {
                of: Box::new(E::List(vec![a.clone(), b.clone()])),
                n: 1,
            },
            true,
        ),
        (
            "order",
            E::OrderBy {
                of: Box::new(E::This),
                key: Box::new(E::This),
                descending: false,
            },
            true,
        ),
        ("exists", E::Exists(Box::new(E::Empty)), true),
        (
            "path values",
            E::PathValues {
                path: p.clone(),
                focus: Box::new(E::This),
            },
            true,
        ),
        ("concat", E::Concat(vec![E::Empty, E::This]), true),
        (
            "remove",
            E::Remove {
                nodes: Box::new(E::List(vec![a.clone(), b.clone()])),
                remove: Box::new(E::List(vec![b])),
            },
            true,
        ),
        (
            "flat map",
            E::FlatMap {
                nodes: Box::new(E::This),
                map: Box::new(E::Path(p)),
            },
            true,
        ),
        (
            "instances",
            E::InstancesOf(Box::new(E::Constant(
                NamedNode::from("http://example.org/C").into_term(),
            ))),
            true,
        ),
    ] {
        let mut root = prototype.clone();
        root.constraints = vec![Constraint::Expression {
            expr: E::ConformsToShape {
                node: Box::new(node),
                shape: ShapeArg::Named(Box::new(target.clone())),
            },
            messages: Vec::new(),
            severity: None,
        }];
        let owner = ModelOwner::from_definitions(vec![root, target.clone()]).unwrap();
        let root = owner.root(&sid).unwrap();
        shapes.node_shapes = vec![root.clone()];
        shapes.owner = owner;
        assert!(
            matches!(PreparedShapes::new(Arc::new(shapes.clone())).bind_dataset(&dataset), Err(ShapesError::Recursion(refusal)) if refusal.reason == RecursionRefusalReason::MissingSelection),
            "{label}"
        );
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = options(semantics);
            let expected = has_focus && semantics == RecursionSemantics::Gfp;
            assert_eq!(
                conforms_with_recursion(&data, &a, &root, request.recursion.as_ref().unwrap())
                    .unwrap(),
                expected,
                "{label}"
            );
            let report = PreparedShapes::new(Arc::new(shapes.clone()))
                .with_validation_options(request)
                .bind_dataset(&dataset)
                .unwrap()
                .validate()
                .unwrap();
            assert_eq!(report.conforms, expected, "{label}");
            for result in report.results {
                assert_eq!(
                    result.focus_node, a,
                    "the selected focus must not replace report focus: {label}"
                );
            }
        }
    }
}

/// A computed conformance focus never replaces the property's result focus,
/// value or path; cardinality failures remain hard errors rather than false.
#[test]
fn pure_conformance_focus_keeps_property_results_and_cardinality() {
    use purrdf_shapes::term::NamedNode;
    let source = format!(
        "{PREFIXES}
        :S a sh:NodeShape ; sh:targetNode :a ; sh:property [
            sh:path :p ; sh:expression [ shnex:conformsToShape ( [ shnex:pathValues :q ] :T ) ]
        ] .
        :T a sh:NodeShape ; sh:node :T ; sh:hasValue :c ."
    );
    let a = NamedNode::from("http://example.org/a").into_term();
    let b = NamedNode::from("http://example.org/b").into_term();
    for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
        let shapes = parse_shapes_with_options(
            &source,
            None,
            None,
            None,
            &ShapesImports::new(),
            &options(semantics),
        )
        .unwrap();
        for (facts, matches_target) in [
            (":a :p :b . :b :q :c .", true),
            (":a :p :b . :b :q :d .", false),
            (":a :p :b .", false),
        ] {
            let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} {facts}"), None).unwrap();
            let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
            let report = validate_with(&data, &shapes).unwrap();
            let expected = matches_target && semantics == RecursionSemantics::Gfp;
            assert_eq!(report.conforms, expected);
            assert_eq!(report.results.len(), usize::from(!expected));
            if let Some(result) = report.results.first() {
                assert_eq!(result.focus_node, a);
                assert_eq!(result.value.as_ref(), Some(&b));
                assert_eq!(
                    result.result_path.as_ref(),
                    Some(&NamedNode::from("http://example.org/p").into_term())
                );
                assert_eq!(
                    result.source_constraint_component.as_str(),
                    purrdf_shapes::model::sh::EXPRESSION_CONSTRAINT_COMPONENT
                );
            }
        }
        let dataset =
            parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b . :b :q :c, :d ."), None)
                .unwrap();
        let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
        let error = validate_with(&data, &shapes).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("node argument must yield at most one node, got 2"),
            "{error}"
        );
    }
}

/// Structural admission inspects untaken branches and ordering keys before
/// evaluating any focus; callbacks and ownership wrappers cannot hide there.
#[test]
fn pure_conformance_focus_refuses_hidden_conformance_and_callbacks() {
    use purrdf_shapes::constraints::conforms_with_recursion;
    use purrdf_shapes::expression::{FnCall, NodeExpr as E, ShapeArg};
    use purrdf_shapes::shapes::{Constraint, ModelOwner};
    use purrdf_shapes::term::NamedNode;

    let shapes = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let sid = NamedNode::from("http://example.org/S").into_term();
    let a = NamedNode::from("http://example.org/a").into_term();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let prototype = shapes.owner.definition(&sid).unwrap();
    let mut target = prototype.clone();
    target.targets.clear();
    target.constraints = vec![Constraint::Reference(sid.clone())];
    for (label, node) in [
        (
            "untaken conformance",
            E::If {
                cond: Box::new(E::Exists(Box::new(E::This))),
                then: Box::new(E::This),
                els: Box::new(E::ConformsToShape {
                    node: Box::new(E::This),
                    shape: ShapeArg::Named(Box::new(target.clone())),
                }),
            },
        ),
        (
            "ordering query",
            E::OrderBy {
                of: Box::new(E::This),
                key: Box::new(E::Select {
                    query: "SELECT (<http://example.org/a> AS ?result) WHERE {}".to_owned(),
                    variable: "result".to_owned(),
                    key: "sh:select",
                }),
                descending: false,
            },
        ),
        (
            "untaken call",
            E::If {
                cond: Box::new(E::Exists(Box::new(E::This))),
                then: Box::new(E::This),
                els: Box::new(E::Call(FnCall::Builtin {
                    iri: NamedNode::from("http://example.org/blocked-call"),
                    args: vec![E::This],
                })),
            },
        ),
        (
            "owned focus",
            E::Owned {
                expr: Box::new(E::This),
                owner: Arc::clone(&shapes.owner),
            },
        ),
    ] {
        let mut root = prototype.clone();
        root.constraints = vec![Constraint::Expression {
            expr: E::ConformsToShape {
                node: Box::new(node),
                shape: ShapeArg::Named(Box::new(target.clone())),
            },
            messages: Vec::new(),
            severity: None,
        }];
        let owner = ModelOwner::from_definitions(vec![root]).unwrap();
        let root = owner.root(&sid).unwrap();
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            assert!(
                matches!(
                    conforms_with_recursion(
                        &data,
                        &a,
                        &root,
                        options(semantics).recursion.as_ref().unwrap()
                    ),
                    Err(ShapesError::Preparation(
                        purrdf_shapes::PreparationRefusal::UnqualifiedRecursion(_)
                    ))
                ),
                "{label}"
            );
        }
    }
}

/// Original static references retain their membership, body and owner proof.
/// Invalid term kinds fail even when the containing native catalog defines them.
#[test]
fn named_conformance_refuses_replaced_and_foreign_target_handles() {
    use purrdf_shapes::constraints::{conforms, conforms_with_recursion};
    use purrdf_shapes::expression::{NodeExpr, ShapeArg};
    use purrdf_shapes::shapes::{Constraint, ModelOwner, NodeKindValue};
    use purrdf_shapes::term::{Literal, NamedNode, Term, Triple};

    let parsed = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetNode :a ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &ValidationOptions::default(),
    )
    .unwrap();
    let sid = NamedNode::from("http://example.org/S").into_term();
    let prototype = parsed.owner.definition(&sid).unwrap();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :a ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    let focus = NamedNode::from("http://example.org/a").into_term();
    for (id, present, valid) in [
        (
            NamedNode::from("http://example.org/T").into_term(),
            true,
            true,
        ),
        (Term::BlankNode("original".to_owned()), true, true),
        (
            NamedNode::from("http://example.org/missing").into_term(),
            false,
            false,
        ),
        (
            Term::Literal(Literal::new_simple_literal("invalid")),
            true,
            false,
        ),
        (
            Term::Triple(Box::new(Triple::new(
                focus.clone(),
                NamedNode::from("http://example.org/p"),
                focus.clone(),
            ))),
            true,
            false,
        ),
    ] {
        for variant in 0..if valid { 4 } else { 1 } {
            let mut definition = prototype.clone();
            definition.id = id.clone();
            definition.targets.clear();
            definition.constraints = vec![Constraint::NodeKind(vec![NodeKindValue::Iri])];
            let mut target = definition.clone();
            target.constraints = vec![Constraint::Reference(id.clone())];
            match variant {
                1 => target
                    .messages
                    .push(Literal::new_simple_literal("local replacement")),
                2 => {
                    let foreign = ModelOwner::from_definitions(vec![definition.clone()]).unwrap();
                    target = foreign.root(&id).unwrap();
                }
                3 => target.constraints.clear(),
                _ => {}
            }
            let mut root = prototype.clone();
            root.constraints = vec![Constraint::Expression {
                expr: NodeExpr::ConformsToShape {
                    node: Box::new(NodeExpr::This),
                    shape: ShapeArg::Named(Box::new(target)),
                },
                messages: Vec::new(),
                severity: None,
            }];
            let mut definitions = vec![root];
            if present {
                definitions.push(definition);
            }
            let owner = ModelOwner::from_definitions(definitions).unwrap();
            let root = owner.root(&sid).unwrap();
            if valid && variant == 0 {
                assert!(conforms(&data, &focus, &root).unwrap());
            }
            for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
                let actual = conforms_with_recursion(
                    &data,
                    &focus,
                    &root,
                    options(semantics).recursion.as_ref().unwrap(),
                );
                if valid && variant == 0 {
                    assert!(actual.unwrap());
                } else {
                    let Err(ShapesError::Preparation(
                        purrdf_shapes::PreparationRefusal::UnqualifiedRecursion(refusal),
                    )) = actual
                    else {
                        panic!(
                            "a replacement or invalid static operand acquired the original target proof"
                        )
                    };
                    assert_eq!(refusal.definition(), &sid);
                    assert!(Arc::ptr_eq(refusal.model().unwrap(), &owner));
                }
            }
        }
    }
}

/// An original where target selects graph nodes by the recursive relation,
/// including lower-stratum negation. It never treats a target declaration as a
/// constraint on the body the relation asks.
#[test]
fn named_where_targets_share_fixed_points_and_strata() {
    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b . :b :p :c ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for target in [":W", "_:w"] {
        for (body, extra, least, greatest) in [
            ("sh:node :W", "", Vec::new(), vec!["a", "b", "c"]),
            (
                "sh:node :V",
                ":V a sh:NodeShape ; sh:node :W .",
                Vec::new(),
                vec!["a", "b", "c"],
            ),
            (
                "sh:or ( [ sh:hasValue :b ] [ sh:node :W ] ) ; sh:not :Bad",
                ":Bad a sh:NodeShape ; sh:hasValue :a .",
                vec!["b"],
                vec!["b", "c"],
            ),
        ] {
            let source = format!(
            "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :W ; sh:nodeKind sh:Literal . :W a sh:NodeShape ; {body} . {extra}"
        ).replace(":W", target);
            assert!(
                matches!(parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &ValidationOptions::default()), Err(CompleteValidationError::Shapes(ShapesError::Recursion(refusal))) if refusal.reason == RecursionRefusalReason::MissingSelection)
            );
            for (semantics, expected) in [
                (RecursionSemantics::Lfp, &least),
                (RecursionSemantics::Gfp, &greatest),
            ] {
                let shapes = parse_shapes_with_options(
                    &source,
                    None,
                    None,
                    None,
                    &ShapesImports::new(),
                    &options(semantics),
                )
                .unwrap();
                let report = validate_with(&data, &shapes).unwrap();
                let mut focuses: Vec<_> = report
                    .results
                    .iter()
                    .map(|result| {
                        assert_eq!(result.source_shape.to_string(), "<http://example.org/S>");
                        result.focus_node.to_string()
                    })
                    .collect();
                focuses.sort();
                assert_eq!(
                    focuses,
                    expected
                        .iter()
                        .map(|name| format!("<http://example.org/{name}>"))
                        .collect::<Vec<_>>(),
                    "{semantics:?}: {body}"
                );
            }
        }
        for body in ["sh:not :W", "sh:xone ( :W :W )"] {
            let source = format!(
                "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :W . :W a sh:NodeShape ; {body} ."
            )
            .replace(":W", target);
            for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
                assert!(
                    matches!(parse_shapes_with_options(&source, None, None, None, &ShapesImports::new(), &options(semantics)), Err(CompleteValidationError::Shapes(ShapesError::Recursion(refusal))) if refusal.reason == RecursionRefusalReason::NonStratified { semantics })
                );
            }
        }
    }
}

/// The sweep admits all roots into one execution before settlement. A limit
/// below the candidate count refuses even when each individual root would fit.
#[test]
fn named_where_targets_charge_all_roots_before_publishing() {
    let source = format!(
        "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :W ; sh:nodeKind sh:Literal . :W a sh:NodeShape ; sh:node :W ."
    );
    let dataset =
        parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b . :b :p :c ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for target in [":W", "_:w"] {
        let source = source.replace(":W", target);
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            for (pairs, rounds, expected) in [
                (
                    0,
                    4_096,
                    Some(RecursionRefusalReason::PairLimit {
                        limit: 0,
                        consumed: 0,
                    }),
                ),
                (
                    2,
                    4_096,
                    Some(RecursionRefusalReason::PairLimit {
                        limit: 2,
                        consumed: 2,
                    }),
                ),
                (
                    3,
                    0,
                    Some(RecursionRefusalReason::IterationLimit {
                        limit: 0,
                        consumed: 0,
                    }),
                ),
                (3, 4_096, None),
            ] {
                let request = ValidationOptions::default().with_recursion(RecursionOptions {
                    semantics,
                    max_pairs: pairs,
                    max_iterations: rounds,
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
                let actual = validate_with(&data, &shapes);
                if let Some(expected) = expected {
                    assert!(
                        matches!(actual, Err(ShapesError::Recursion(refusal)) if refusal.reason == expected)
                    );
                } else {
                    assert_eq!(
                        actual.unwrap().results.len(),
                        if semantics == RecursionSemantics::Gfp {
                            3
                        } else {
                            0
                        }
                    );
                }
                if pairs == 0 {
                    let empty = parse_turtle_to_dataset("", None).unwrap();
                    let empty = ShaclData::new(Arc::clone(&empty), empty, None);
                    assert!(validate_with(&empty, &shapes).unwrap().conforms);
                }
            }
        }
    }
}

/// Restoring reusable configuration must bind target answers to fresh data,
/// while reports and bounded membership agree with the direct target driver.
#[test]
fn named_where_targets_rebind_original_reports_after_product_restore() {
    use purrdf_shapes::engine::PreparedShapes;
    use purrdf_shapes::product::{HostBindings, ShapesProduct, ShapesProfile};
    use purrdf_shapes::term::{NamedNode, Term};

    let source = format!(
        "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :W ; sh:nodeKind sh:Literal . :W a sh:NodeShape ; sh:node :W ."
    );
    for target in [":W", "_:w"] {
        let source = source.replace(":W", target);
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let request = options(semantics);
            let shapes = Arc::new(
                parse_shapes_with_options(
                    &source,
                    None,
                    None,
                    None,
                    &ShapesImports::new(),
                    &request,
                )
                .unwrap(),
            );
            let prepared = PreparedShapes::new(Arc::clone(&shapes));
            let bytes = prepared.to_product(&ShapesProfile::CORE).unwrap();
            for (graph, names) in [
                (":a :p :b .", ["a", "b"]),
                (":fresh :q :other .", ["fresh", "other"]),
            ] {
                let dataset =
                    parse_turtle_to_dataset(&format!("{PREFIXES} {graph}"), None).unwrap();
                let data = ShaclData::new(Arc::clone(&dataset), Arc::clone(&dataset), None);
                let direct = validate_with(&data, &shapes).unwrap();
                assert_eq!(
                    direct.results.len(),
                    if semantics == RecursionSemantics::Gfp {
                        2
                    } else {
                        0
                    }
                );
                let expected = direct.to_ntriples().unwrap();
                let focuses: Vec<Term> = names
                    .iter()
                    .map(|name| {
                        NamedNode::new_unchecked(format!("http://example.org/{name}")).into_term()
                    })
                    .collect();
                let bound = prepared.bind_dataset(&dataset).unwrap();
                assert_eq!(bound.validate().unwrap().to_ntriples().unwrap(), expected);
                assert_eq!(
                    bound
                        .validate_focus_nodes(&focuses)
                        .unwrap()
                        .to_ntriples()
                        .unwrap(),
                    expected
                );
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
                    }
                    .unwrap();
                    let bound = restored.bind_dataset(&dataset).unwrap();
                    assert_eq!(
                        bound.validate().unwrap().to_ntriples().unwrap(),
                        expected,
                        "rebuild={rebuild}"
                    );
                    assert_eq!(
                        bound
                            .validate_focus_nodes(&focuses)
                            .unwrap()
                            .to_ntriples()
                            .unwrap(),
                        expected,
                        "rebuild={rebuild}"
                    );
                    assert_eq!(restored.to_product(&ShapesProfile::CORE).unwrap(), bytes);
                }
            }
        }
    }
}

/// A selected root's where declaration retains its exact original operand:
/// local overrides, foreign catalogs and different target families cannot
/// inherit the canonical root's proof merely by retaining its name.
#[test]
fn named_where_targets_refuse_replaced_root_operands() {
    use purrdf_shapes::constraints::conforms_with_recursion;
    use purrdf_shapes::shapes::Target;
    use purrdf_shapes::term::{Literal, NamedNode};

    let source = format!(
        "{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :W ; sh:nodeKind sh:Literal . :W a sh:NodeShape ; sh:node :W ."
    );
    let sid = NamedNode::from("http://example.org/S").into_term();
    let focus = NamedNode::from("http://example.org/a").into_term();
    let dataset = parse_turtle_to_dataset(&format!("{PREFIXES} :a :p :b ."), None).unwrap();
    let data = ShaclData::new(Arc::clone(&dataset), dataset, None);
    for target in [":W", "_:w"] {
        let source = source.replace(":W", target);
        let shapes = parse_shapes_with_options(
            &source,
            None,
            None,
            None,
            &ShapesImports::new(),
            &options(RecursionSemantics::Gfp),
        )
        .unwrap();
        let original = shapes.owner.root(&sid).unwrap();
        let foreign = parse_shapes_with_options(
            &source,
            None,
            None,
            None,
            &ShapesImports::new(),
            &options(RecursionSemantics::Gfp),
        )
        .unwrap();
        let foreign_root = foreign.owner.root(&sid).unwrap();
        let Target::Where(foreign_operand) = &foreign_root.targets[0] else {
            unreachable!()
        };
        let foreign_target = foreign.owner.root(&foreign_operand.id).unwrap();
        for variant in 0..5 {
            let mut root = original.clone();
            match variant {
                1 => {
                    let Target::Where(target) = &mut root.targets[0] else {
                        unreachable!()
                    };
                    target
                        .messages
                        .push(Literal::new_simple_literal("replacement"));
                }
                2 => root.targets[0] = Target::Where(Box::new(foreign_target.clone())),
                3 => root.targets[0] = Target::Node(focus.clone()),
                4 => {
                    let Target::Where(target) = &mut root.targets[0] else {
                        unreachable!()
                    };
                    target.constraints.clear();
                }
                _ => {}
            }
            for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
                let actual = conforms_with_recursion(
                    &data,
                    &focus,
                    &root,
                    options(semantics).recursion.as_ref().unwrap(),
                );
                if variant == 0 {
                    assert!(!actual.unwrap());
                } else {
                    assert!(matches!(
                        actual,
                        Err(ShapesError::Preparation(
                            purrdf_shapes::PreparationRefusal::UnqualifiedRecursion(_)
                        ))
                    ));
                }
            }
        }
    }

    // Invalid term kinds must fail even when their definitions are present.
    use purrdf_shapes::shapes::{Constraint, ModelOwner};
    use purrdf_shapes::term::{Term, Triple};
    let parsed = parse_shapes_with_options(
        &source,
        None,
        None,
        None,
        &ShapesImports::new(),
        &options(RecursionSemantics::Gfp),
    )
    .unwrap();
    let prototype = parsed.owner.definition(&sid).unwrap();
    let Target::Where(bare) = &prototype.targets[0] else {
        unreachable!()
    };
    for (id, present, allowed) in [
        (
            NamedNode::from("http://example.org/W").into_term(),
            true,
            true,
        ),
        (Term::BlankNode("original".to_owned()), true, true),
        (
            NamedNode::from("http://example.org/missing").into_term(),
            false,
            false,
        ),
        (
            Term::Literal(Literal::new_simple_literal("invalid")),
            true,
            false,
        ),
        (
            Term::Triple(Box::new(Triple::new(
                focus.clone(),
                NamedNode::from("http://example.org/p"),
                focus.clone(),
            ))),
            true,
            false,
        ),
    ] {
        let mut operand = bare.as_ref().clone();
        operand.id = id.clone();
        operand.constraints = vec![Constraint::Reference(id.clone())];
        let mut root = prototype.clone();
        root.targets[0] = Target::Where(Box::new(operand));
        let mut definitions = vec![root];
        if present {
            let mut target = parsed.owner.definition(&bare.id).unwrap().clone();
            target.id = id;
            target.constraints = vec![Constraint::NodeKind(vec![
                purrdf_shapes::shapes::NodeKindValue::Iri,
            ])];
            definitions.push(target);
        }
        let owner = ModelOwner::from_definitions(definitions).unwrap();
        let root = owner.root(&sid).unwrap();
        let actual = conforms_with_recursion(
            &data,
            &focus,
            &root,
            options(RecursionSemantics::Gfp).recursion.as_ref().unwrap(),
        );
        if allowed {
            assert!(!actual.unwrap());
        } else {
            assert!(matches!(
                actual,
                Err(ShapesError::Preparation(
                    purrdf_shapes::PreparationRefusal::UnqualifiedRecursion(_)
                ))
            ));
        }
    }

    // An owned root carries its own targets, unlike the original bare operand.
    // Two replacements suffice to detect recursive proof normalization without
    // approaching stack exhaustion. Keep enough stack for the native margin.
    let request = options(RecursionSemantics::Gfp);
    let self_target = parse_shapes_with_options(
        &format!("{PREFIXES} :S a sh:NodeShape ; sh:targetWhere :S ; sh:nodeKind sh:Literal ."),
        None,
        None,
        None,
        &ShapesImports::new(),
        &request,
    )
    .unwrap();
    let original = self_target.owner.root(&sid).unwrap();
    let mut replaced = original.clone();
    for _ in 0..2 {
        let mut outer = original.clone();
        outer.targets[0] = Target::Where(Box::new(replaced));
        replaced = outer;
    }
    purrdf_stack::on_stack_scoped(256 * 1_024, || {
        let recursion = request.recursion.as_ref().unwrap();
        assert!(!conforms_with_recursion(&data, &focus, &original, recursion).unwrap());
        assert!(
            matches!(
                conforms_with_recursion(&data, &focus, &replaced, recursion),
                Err(ShapesError::Preparation(
                    purrdf_shapes::PreparationRefusal::UnqualifiedRecursion(_)
                ))
            ),
            "an owned root tree is not the original bare where operand"
        );
    })
    .unwrap();
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
            "sh:expression [ shnex:conformsToShape ( sh:this [ shnex:pathValues :p ] ) ]",
            purrdf_shapes::model::sh::EXPRESSION,
            false,
        ),
        (
            "sh:expression [ sh:union ( [ shnex:conformsToShape ( sh:this :T ) ] ) ]",
            purrdf_shapes::model::sh::EXPRESSION,
            false,
        ),
        (
            "sh:expression [ shnex:conformsToShape ( sh:this [] ) ]",
            purrdf_shapes::model::sh::EXPRESSION,
            false,
        ),
        (
            "sh:expression [ shnex:conformsToShape ( sh:this [ sh:path :p ] ) ]",
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

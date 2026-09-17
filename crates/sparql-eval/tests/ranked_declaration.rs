// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ranked declaration as a registry side table: it is supplied where a
//! producer is wired up, validated against the relation's declared arity at that
//! point, read back by IRI, and has a canonical description injective over every
//! field it carries.
//!
//! A relation registered without a declaration declares nothing, and nothing is
//! what a consumer reads back — no stub, no sentinel on the relation trait.

use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use purrdf_core::Iri;
use purrdf_sparql_eval::{
    AcceptedTerm, DepthPlacement, DuplicatePolicy, MemoryRelation, PropertyFunction,
    PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet, TermKind, TermPattern,
    TermPlacement,
};

const EX_REL: &str = "http://example.org/ns#search";
const EX_OTHER: &str = "http://example.org/ns#other";
const EX_STRATUM: &str = "http://example.org/stratum/lexical";
const EX_STRATUM_B: &str = "http://example.org/stratum/vector";
const EX_DEPTH_TYPE: &str = "http://example.org/datatype/count";
const EX_LANG_TYPE: &str = "http://example.org/datatype/tag";

/// A five-position relation: one subject-side argument, four object-side. The
/// widest shape the fixtures below place into, so a position is out of range
/// only when a test deliberately puts it there.
fn relation() -> Arc<dyn PropertyFunction> {
    Arc::new(MemoryRelation::new(1, 4, Vec::new()).expect("an empty table is uniform"))
}

fn stratum(iri: &str) -> Iri {
    purrdf_core::parse_iri(iri).expect("fixture IRI")
}

fn placement(facet: RequestFacet, position: usize) -> TermPlacement {
    TermPlacement {
        facet,
        position,
        datatype: None,
    }
}

fn english_literal() -> TermPattern {
    TermPattern {
        kind: TermKind::Literal,
        datatype: None,
        language: Some("en".to_owned()),
        predicate: None,
    }
}

/// The reference declaration, and the VALID NEIGHBOUR every refusal test below
/// is a one-field mutation of: a lexical search whose needle renders at position
/// 1 and whose language tag renders at position 4 — two positions for one request
/// term, because leaving the language free would silently widen the request to
/// every language — with the per-stratum depth at position 2 and the candidate
/// projected from position 0. Every position is in range, and the depth, the
/// placements and the candidate are pairwise distinct.
fn declaration() -> RankedDeclaration {
    RankedDeclaration {
        stratum: stratum(EX_STRATUM),
        accepted_terms: vec![AcceptedTerm {
            pattern: english_literal(),
            placements: vec![
                placement(RequestFacet::Value, 1),
                TermPlacement {
                    facet: RequestFacet::Language,
                    position: 4,
                    datatype: Some(EX_LANG_TYPE.to_owned()),
                },
            ],
        }],
        depth_placement: Some(DepthPlacement {
            position: 2,
            datatype: EX_DEPTH_TYPE.to_owned(),
        }),
        candidate_position: 0,
        ordering: RankOrdering::StrictlyDescending,
        duplicates: DuplicatePolicy::Unique,
        mandatory: true,
    }
}

/// Run `body` with the default panic hook suppressed, so an *expected*, caught
/// panic does not dump to stderr.
fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = body();
    std::panic::set_hook(default_hook);
    out
}

/// The panic message `body` raises, as a `String`.
fn panic_message(body: impl FnOnce()) -> String {
    let payload = without_panic_output(|| {
        std::panic::catch_unwind(AssertUnwindSafe(body)).expect_err("the body must panic")
    });
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .expect("an assert! payload is a string")
}

// ---- the side table ------------------------------------------------------

#[test]
fn register_ranked_is_read_back_by_iri_and_plain_registration_declares_nothing() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    registry.register(EX_OTHER, relation());

    assert_eq!(
        registry.ranked_declaration(EX_REL),
        Some(&declaration()),
        "the declaration is stored beside the relation, verbatim"
    );
    assert_eq!(
        registry.ranked_declaration(EX_OTHER),
        None,
        "a plainly registered relation declares nothing at all"
    );
    assert_eq!(
        registry.ranked_declaration("http://example.org/ns#absent"),
        None,
        "an unregistered IRI declares nothing either"
    );

    // Registration is otherwise identical: both relations resolve and both are
    // counted.
    assert_eq!(registry.len(), 2);
    assert!(registry.resolve(EX_REL).is_some());
    assert!(registry.resolve(EX_OTHER).is_some());
}

#[test]
fn describe_reports_the_declaration_against_its_own_iri() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    registry.register(EX_OTHER, relation());
    let described = registry.describe().expect("no relation panics");

    let ranked_for = |iri: &str| {
        described
            .iter()
            .find(|d| d.iri == iri)
            .expect("described")
            .ranked
            .clone()
    };
    assert_eq!(ranked_for(EX_REL), Some(declaration()));
    assert_eq!(ranked_for(EX_OTHER), None);
}

#[test]
fn debug_lists_the_ranked_iris() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    registry.register(EX_OTHER, relation());
    let rendered = format!("{registry:?}");
    assert!(rendered.contains("ranked"), "got {rendered}");
}

// ---- the duplicate-IRI refusal is one refusal, not two -------------------

#[test]
#[should_panic(expected = "already registered as a property function")]
fn duplicate_registration_through_register_ranked_panics() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    registry.register_ranked(EX_REL, relation(), declaration());
}

#[test]
fn both_registration_paths_raise_the_identical_duplicate_message() {
    // Byte-identical, not merely similar: both paths funnel through one insert,
    // so the message a host reads cannot depend on which method it called.
    let plain = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_REL, relation());
        registry.register(EX_REL, relation());
    });
    let ranked = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(EX_REL, relation(), declaration());
        registry.register_ranked(EX_REL, relation(), declaration());
    });
    let mixed = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_REL, relation());
        registry.register_ranked(EX_REL, relation(), declaration());
    });
    assert_eq!(plain, ranked);
    assert_eq!(plain, mixed);
    assert_eq!(
        plain,
        format!(
            "IRI <{EX_REL}> is already registered as a property function; a relation may not be \
             silently shadowed, because both spellings of the call are identical and the only \
             observable difference is which rows the query returns"
        )
    );
}

// ---- the five registration-time refusals ---------------------------------

#[test]
#[should_panic(expected = "projects its candidate from position 9")]
fn a_candidate_position_outside_the_arity_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        candidate_position: 9,
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
#[should_panic(expected = "binds the value facet of an accepted term at position 7 but")]
fn a_placement_position_outside_the_arity_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        accepted_terms: vec![AcceptedTerm {
            pattern: english_literal(),
            placements: vec![placement(RequestFacet::Value, 7)],
        }],
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
#[should_panic(expected = "binds its per-stratum depth at position 6 but")]
fn a_depth_position_outside_the_arity_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        depth_placement: Some(DepthPlacement {
            position: 6,
            datatype: EX_DEPTH_TYPE.to_owned(),
        }),
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
#[should_panic(
    expected = "binds both its per-stratum depth and the value facet of an accepted term at \
                position 1"
)]
fn a_depth_that_collides_with_a_placement_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        depth_placement: Some(DepthPlacement {
            position: 1,
            datatype: EX_DEPTH_TYPE.to_owned(),
        }),
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
#[should_panic(
    expected = "binds the value facet of an accepted term at position 1 and also projects its \
                candidate from there"
)]
fn a_candidate_that_collides_with_a_placement_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        candidate_position: 1,
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
#[should_panic(
    expected = "binds its per-stratum depth at position 2 and also projects its candidate from \
                there"
)]
fn a_candidate_that_collides_with_the_depth_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    let decl = RankedDeclaration {
        candidate_position: 2,
        accepted_terms: vec![AcceptedTerm {
            pattern: english_literal(),
            placements: vec![placement(RequestFacet::Value, 1)],
        }],
        ..declaration()
    };
    registry.register_ranked(EX_REL, relation(), decl);
}

#[test]
fn a_failed_validation_leaves_the_registry_untouched() {
    let mut registry = PropertyFunctionRegistry::new();
    let outcome = without_panic_output(|| {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            registry.register_ranked(
                EX_REL,
                relation(),
                RankedDeclaration {
                    candidate_position: 9,
                    ..declaration()
                },
            );
        }))
    });
    assert!(outcome.is_err(), "the declaration must be refused");
    assert!(
        registry.is_empty(),
        "nothing was inserted before the refusal"
    );
    assert!(registry.resolve(EX_REL).is_none());
    assert!(registry.ranked_declaration(EX_REL).is_none());
    // And the IRI is still free: the refusal did not half-register it.
    registry.register_ranked(EX_REL, relation(), declaration());
    assert_eq!(registry.ranked_declaration(EX_REL), Some(&declaration()));
}

// ---- the valid neighbours ------------------------------------------------

#[test]
fn a_declaration_whose_positions_are_all_in_range_and_distinct_registers() {
    // The mirror of every refusal above. Over-refusal is as severe a defect as a
    // silent wrong answer, so the neighbouring VALID case is executed, not
    // assumed: two placements for one request term (needle and language tag), a
    // depth argument distinct from both, and a candidate distinct from all three.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    assert_eq!(registry.ranked_declaration(EX_REL), Some(&declaration()));

    // The boundary itself: the last position of a five-position relation is in
    // range, and `None` depth is a producer bounded by the consumer's ceiling.
    let mut edge = PropertyFunctionRegistry::new();
    edge.register_ranked(
        EX_OTHER,
        relation(),
        RankedDeclaration {
            accepted_terms: vec![AcceptedTerm {
                pattern: english_literal(),
                placements: vec![placement(RequestFacet::Value, 4)],
            }],
            depth_placement: None,
            candidate_position: 0,
            mandatory: false,
            ..declaration()
        },
    );
    assert!(edge.ranked_declaration(EX_OTHER).is_some());

    // A declaration with no accepted terms and no depth is still renderable: the
    // candidate position is the only thing that must exist.
    let mut bare = PropertyFunctionRegistry::new();
    bare.register_ranked(
        EX_OTHER,
        relation(),
        RankedDeclaration {
            accepted_terms: Vec::new(),
            depth_placement: None,
            candidate_position: 4,
            ..declaration()
        },
    );
    assert!(bare.ranked_declaration(EX_OTHER).is_some());
}

// ---- the canonical description -------------------------------------------

#[test]
fn canonical_description_is_a_pure_function_of_the_value() {
    let one = declaration();
    let two = declaration();
    assert_eq!(one.canonical_description(), two.canonical_description());
    assert_eq!(
        one.canonical_description(),
        one.canonical_description(),
        "and it is stable across repeated calls"
    );
}

#[test]
fn canonical_description_is_injective_over_every_field() {
    let base = declaration();
    let variants: Vec<(&str, RankedDeclaration)> = vec![
        ("base", base.clone()),
        (
            "stratum",
            RankedDeclaration {
                stratum: stratum(EX_STRATUM_B),
                ..base.clone()
            },
        ),
        (
            "ordering",
            RankedDeclaration {
                ordering: RankOrdering::NonIncreasing,
                ..base.clone()
            },
        ),
        (
            "duplicates",
            RankedDeclaration {
                duplicates: DuplicatePolicy::Allowed,
                ..base.clone()
            },
        ),
        (
            "candidate_position",
            RankedDeclaration {
                candidate_position: 3,
                ..base.clone()
            },
        ),
        (
            "mandatory",
            RankedDeclaration {
                mandatory: false,
                ..base.clone()
            },
        ),
        (
            "depth absent",
            RankedDeclaration {
                depth_placement: None,
                ..base.clone()
            },
        ),
        (
            "depth position",
            RankedDeclaration {
                depth_placement: Some(DepthPlacement {
                    position: 3,
                    datatype: EX_DEPTH_TYPE.to_owned(),
                }),
                ..base.clone()
            },
        ),
        (
            "depth datatype",
            RankedDeclaration {
                depth_placement: Some(DepthPlacement {
                    position: 2,
                    datatype: EX_LANG_TYPE.to_owned(),
                }),
                ..base.clone()
            },
        ),
        (
            "no accepted terms",
            RankedDeclaration {
                accepted_terms: Vec::new(),
                ..base.clone()
            },
        ),
        (
            "term pattern kind",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: TermPattern::of_kind(TermKind::Iri),
                    placements: base.accepted_terms[0].placements.clone(),
                }],
            ),
        ),
        (
            "term pattern language",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: TermPattern {
                        language: Some("fr".to_owned()),
                        ..english_literal()
                    },
                    placements: base.accepted_terms[0].placements.clone(),
                }],
            ),
        ),
        (
            "placement facet",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: english_literal(),
                    placements: vec![
                        placement(RequestFacet::MaxDistance, 1),
                        TermPlacement {
                            facet: RequestFacet::Language,
                            position: 4,
                            datatype: Some(EX_LANG_TYPE.to_owned()),
                        },
                    ],
                }],
            ),
        ),
        (
            "placement position",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: english_literal(),
                    placements: vec![
                        placement(RequestFacet::Value, 3),
                        TermPlacement {
                            facet: RequestFacet::Language,
                            position: 4,
                            datatype: Some(EX_LANG_TYPE.to_owned()),
                        },
                    ],
                }],
            ),
        ),
        (
            "placement datatype absent",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: english_literal(),
                    placements: vec![
                        placement(RequestFacet::Value, 1),
                        placement(RequestFacet::Language, 4),
                    ],
                }],
            ),
        ),
        (
            "placement order",
            with_terms(
                &base,
                vec![AcceptedTerm {
                    pattern: english_literal(),
                    placements: vec![
                        TermPlacement {
                            facet: RequestFacet::Language,
                            position: 4,
                            datatype: Some(EX_LANG_TYPE.to_owned()),
                        },
                        placement(RequestFacet::Value, 1),
                    ],
                }],
            ),
        ),
        (
            "term count",
            with_terms(
                &base,
                vec![
                    base.accepted_terms[0].clone(),
                    AcceptedTerm {
                        pattern: TermPattern::of_kind(TermKind::Iri),
                        placements: Vec::new(),
                    },
                ],
            ),
        ),
    ];

    for (i, (left_name, left)) in variants.iter().enumerate() {
        for (right_name, right) in &variants[i + 1..] {
            assert_ne!(
                left.canonical_description(),
                right.canonical_description(),
                "changing {left_name} -> {right_name} must change the canonical description"
            );
        }
    }
}

/// `base` with its accepted terms replaced.
fn with_terms(base: &RankedDeclaration, accepted_terms: Vec<AcceptedTerm>) -> RankedDeclaration {
    RankedDeclaration {
        accepted_terms,
        ..base.clone()
    }
}

#[test]
fn canonical_description_frames_an_empty_string_distinctly_from_an_absent_one() {
    // The framing rule the helpers enforce: absence carries its own byte, so an
    // empty datatype and no datatype can never collide.
    let empty = with_terms(
        &declaration(),
        vec![AcceptedTerm {
            pattern: english_literal(),
            placements: vec![TermPlacement {
                facet: RequestFacet::Value,
                position: 1,
                datatype: Some(String::new()),
            }],
        }],
    );
    let absent = with_terms(
        &declaration(),
        vec![AcceptedTerm {
            pattern: english_literal(),
            placements: vec![placement(RequestFacet::Value, 1)],
        }],
    );
    assert_ne!(
        empty.canonical_description(),
        absent.canonical_description()
    );
}

#[test]
fn request_facet_spellings_are_stable() {
    assert_eq!(RequestFacet::Value.as_str(), "value");
    assert_eq!(RequestFacet::Language.as_str(), "language");
    assert_eq!(RequestFacet::Predicate.as_str(), "predicate");
    assert_eq!(RequestFacet::MaxDistance.as_str(), "max-distance");
}

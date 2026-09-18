// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A producer's participation in ranked retrieval is caller-supplied wiring
//! held in the registry's side table, and the registry's durable content
//! fingerprint covers it exactly as it covers arity, volatility and modes.
//!
//! Two properties are pinned here. First, the fingerprint separates a registry
//! whose relation was wired up with a [`RankedDeclaration`] from one whose
//! identical relation was registered plainly, and joins two registries that
//! declare the same thing — so "does not fuse" and "fuses like this" can never
//! share a digest, and a plan admitted against one wiring cannot run against
//! the other. Second, the declaration types are owned data all the way down:
//! no function pointer appears in the module that defines them, which is what
//! makes a canonical description — and hence the fingerprint — possible at all.
//!
//! The declaration's own surface (registration, read-back by IRI, validation
//! against the relation's arity, and the injectivity of its canonical
//! description) is pinned in `ranked_declaration.rs`.

use std::sync::Arc;

use purrdf_sparql_eval::{
    AcceptedTerm, BindingPattern, DuplicatePolicy, EvalError, PfArgs, PfArity, PfCursor,
    PropertyFunction, PropertyFunctionRegistry, RankOrdering, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement, Volatility,
};

const EX_REL: &str = "http://example.org/ns#ranked";
const EX_STRATUM: &str = "http://example.org/stratum/a";

/// A two-position relation that never actually opens — the declaration surface
/// is the subject under test, and the relation itself is identical across every
/// registry below so it cannot account for any difference observed.
#[derive(Debug)]
struct FixtureRelation {
    modes: [BindingPattern; 1],
}

impl FixtureRelation {
    fn new() -> Self {
        Self {
            modes: [PfArity::new(1, 1).all_free_mode()],
        }
    }
}

impl PropertyFunction for FixtureRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        0
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        Err(EvalError::function("the fixture is never invoked"))
    }
}

/// A lexical-search declaration whose needle renders at position 1 and whose
/// candidate is projected from position 0 — every position in range for the
/// fixture's `1,1` arity.
fn ranked_declaration() -> RankedDeclaration {
    RankedDeclaration {
        stratum: purrdf_core::parse_iri(EX_STRATUM).expect("fixture IRI"),
        accepted_terms: vec![AcceptedTerm {
            pattern: TermPattern {
                kind: TermKind::Literal,
                datatype: None,
                language: Some("en".to_owned()),
                predicate: None,
            },
            placements: vec![TermPlacement {
                facet: RequestFacet::Value,
                position: 1,
                datatype: None,
            }],
        }],
        depth_placement: None,
        candidate_position: 0,
        ordering: RankOrdering::StrictlyDescending,
        duplicates: DuplicatePolicy::Unique,
        mandatory: true,
    }
}

/// One registry holding the same relation under the same IRI, wired up either
/// with `declaration` or plainly.
fn registry_declaring(declaration: Option<RankedDeclaration>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    let relation = Arc::new(FixtureRelation::new());
    match declaration {
        Some(declaration) => registry.register_ranked(EX_REL, relation, declaration),
        None => registry.register(EX_REL, relation),
    }
    registry
}

#[test]
fn the_ranked_declaration_participates_in_the_content_fingerprint() {
    let ranked_registry = registry_declaring(Some(ranked_declaration()));
    let plain_registry = registry_declaring(None);
    assert_ne!(
        ranked_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        plain_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        "a producer's fusion participation is a declaration the fingerprint must cover, so \
         the fixed non-ranked contribution can never spell the same bytes as a ranked one"
    );
    // Two registries declaring the same ranked wiring fingerprint alike: the
    // digest is a function of the declaration, not of the instance that holds it.
    let same = registry_declaring(Some(ranked_declaration()));
    assert_eq!(
        ranked_registry
            .content_fingerprint()
            .expect("declarations are readable"),
        same.content_fingerprint()
            .expect("declarations are readable"),
    );
}

#[test]
fn a_changed_declaration_changes_the_fingerprint() {
    let base = registry_declaring(Some(ranked_declaration()));
    let changed = registry_declaring(Some(RankedDeclaration {
        ordering: RankOrdering::NonIncreasing,
        ..ranked_declaration()
    }));
    assert_ne!(
        base.content_fingerprint()
            .expect("declarations are readable"),
        changed
            .content_fingerprint()
            .expect("declarations are readable"),
        "the declaration reaches the digest field by field, not as a mere present/absent bit"
    );
}

/// The grep gate on the declaration types: no function pointer may appear in the
/// module that defines them.
#[test]
fn declaration_types_contain_no_function_pointers() {
    const SOURCE: &str = include_str!("../src/property_fn.rs");
    assert!(
        !SOURCE.contains("fn("),
        "property_fn.rs contains a function pointer; ranked declarations must be owned data"
    );
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    AcceptedTerm, BindingPattern, CandidateDomains, DomainTag, DuplicatePolicy, EvalError,
    ExclusionBasis, PfArgs, PfArity, PfCursor, PropertyFunction, PropertyFunctionRegistry,
    RankArithmetic, RankFidelity, RankedDeclaration, RequestFacet, TermKind, TermPattern,
    TermPlacement, Volatility,
};

const EX_REL: &str = "http://example.org/ns#ranked";
const EX_STRATUM: &str = "http://example.org/stratum/a";
/// A caller-named block of a candidate universe. Nothing here mints it.
const EX_DOMAIN: &str = "http://example.org/domain/documents";

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
        duplicates: DuplicatePolicy::Unique,
        // The fixture producer is exhaustive over its own table.
        fidelity: RankFidelity::EXACT,
        arithmetic: RankArithmetic::FloatFree,
        domains: CandidateDomains::Unrestricted,
        block_position: None,
        exclusion: ExclusionBasis::Unavailable,
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
        duplicates: DuplicatePolicy::Allowed,
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

#[test]
fn a_declared_candidate_domain_moves_the_fingerprint_and_a_repeat_of_it_does_not() {
    // The two declarations differ in one field, and it is a field that changes
    // what a consumer is ALLOWED TO DO with the rows: under a restriction a
    // fusion may certify a candidate without reading this producer at all. Two
    // wirings that fuse differently may not share a digest, or a plan admitted
    // against one would run against the other and read a different number of
    // rows for the same question.
    let unrestricted = registry_declaring(Some(ranked_declaration()));
    let restricted = registry_declaring(Some(RankedDeclaration {
        domains: CandidateDomains::within([
            DomainTag::parse(EX_DOMAIN).expect("fixture domain tag")
        ]),
        ..ranked_declaration()
    }));
    assert_ne!(
        unrestricted
            .content_fingerprint()
            .expect("declarations are readable"),
        restricted
            .content_fingerprint()
            .expect("declarations are readable"),
        "the candidate-domain declaration must reach the digest"
    );

    // The canonical description is where it reaches it, and the two spellings
    // differ there too — byte level, not merely digest level, so a failure here
    // says which layer drifted.
    assert_ne!(
        ranked_declaration().canonical_description(),
        RankedDeclaration {
            domains: CandidateDomains::within([
                DomainTag::parse(EX_DOMAIN).expect("fixture domain tag"),
            ]),
            ..ranked_declaration()
        }
        .canonical_description(),
    );

    // And the digest is still a function of the declaration rather than of the
    // instance: the same restriction, built again, fingerprints alike.
    let same = registry_declaring(Some(RankedDeclaration {
        domains: CandidateDomains::within([
            DomainTag::parse(EX_DOMAIN).expect("fixture domain tag")
        ]),
        ..ranked_declaration()
    }));
    assert_eq!(
        restricted
            .content_fingerprint()
            .expect("declarations are readable"),
        same.content_fingerprint()
            .expect("declarations are readable"),
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

#[test]
fn a_declared_block_column_moves_the_fingerprint() {
    // Whether a producer's rows name the block they were drawn from decides what
    // a consumer can VERIFY, not merely what it reads: a restriction no row backs
    // is refused, and one every row backs is held to the rows. Two wirings that
    // verify differently may not share a digest, for the same reason two that
    // fuse differently may not — a plan admitted against one would run against
    // the other.
    // Position 1 carries this fixture's needle, so the pair below is stated over
    // a declaration that places no request facet at all — leaving position 1 free
    // to be read back as the block. The two still differ in exactly one field,
    // which is what makes this a claim about that field.
    let bare = RankedDeclaration {
        accepted_terms: Vec::new(),
        ..ranked_declaration()
    };
    let silent = registry_declaring(Some(bare.clone()));
    let naming = registry_declaring(Some(RankedDeclaration {
        block_position: Some(1),
        ..bare.clone()
    }));
    assert_ne!(
        silent
            .content_fingerprint()
            .expect("declarations are readable"),
        naming
            .content_fingerprint()
            .expect("declarations are readable"),
        "the block column must reach the digest"
    );
    assert_ne!(
        bare.canonical_description(),
        RankedDeclaration {
            block_position: Some(1),
            ..bare
        }
        .canonical_description(),
        "and it reaches it through the canonical description, byte level"
    );
}

/// The digest the fixture registry's declarations fold into, hex digit for hex
/// digit.
///
/// A plan carries the fingerprint of the registry it was admitted against and a
/// host compares it against a live one, so this digest is a compatibility
/// boundary: two runs of the same build, and two builds of the same source, must
/// produce this exact string or plans stop matching registries that did not
/// change. Pinning it is what makes a drift visible as a failure here rather
/// than as a mismatch in a caller's plan cache.
///
/// It is a MEASUREMENT: it covers the framed declaration bytes under their
/// domain separator, so it moves whenever either the declared fields or the
/// framing change. Never edit it to match a run — re-run this test and record
/// what it reports.
const FIXTURE_FINGERPRINT: &str =
    "8d6dea117e9324168c55206f57936e38deed009031f1aa6d5d9bc1082ef5e2d3";

#[test]
fn the_fingerprint_is_the_same_string_every_time_the_registry_is_rebuilt() {
    let first = registry_declaring(Some(ranked_declaration()))
        .content_fingerprint()
        .expect("declarations are readable");
    assert_eq!(
        first, FIXTURE_FINGERPRINT,
        "the fingerprint is a pure function of what was registered, so it is pinnable"
    );
    // Rebuilt from scratch: a fresh registry, a fresh relation, a fresh
    // declaration. Nothing of the first instance survives into the second, so an
    // address, an allocation order or a construction counter leaking into the
    // bytes would separate them here.
    for _ in 0..4 {
        assert_eq!(
            registry_declaring(Some(ranked_declaration()))
                .content_fingerprint()
                .expect("declarations are readable"),
            FIXTURE_FINGERPRINT,
        );
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    AcceptedTerm, CandidateDomains, Completeness, DeclaredArithmetic, DepthPlacement, DomainTag,
    DuplicatePolicy, ExclusionBasis, MemoryRelation, OrderFidelity, PropertyFunction,
    PropertyFunctionRegistry, RankArithmetic, RankFidelity, RankedDeclaration, RequestFacet,
    TermKind, TermPattern, TermPlacement,
};

const EX_REL: &str = "http://example.org/ns#search";
const EX_OTHER: &str = "http://example.org/ns#other";
const EX_STRATUM: &str = "http://example.org/stratum/lexical";
const EX_STRATUM_B: &str = "http://example.org/stratum/vector";
const EX_DEPTH_TYPE: &str = "http://example.org/datatype/count";
const EX_LANG_TYPE: &str = "http://example.org/datatype/tag";
/// Two caller-named blocks of a candidate universe. Nothing here mints them;
/// they are ordinary `example.org` IRIs a host chose for its own partition.
const EX_DOMAIN_DOCS: &str = "http://example.org/domain/documents";
const EX_DOMAIN_PEOPLE: &str = "http://example.org/domain/people";

fn tag(iri: &str) -> DomainTag {
    DomainTag::parse(iri).expect("fixture domain tag")
}

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
        duplicates: DuplicatePolicy::Unique,
        // The reference declaration promises a complete, order-faithful search,
        // which is what every producer promised before the term existed. The
        // tests that are ABOUT the term state their own.
        fidelity: RankFidelity::EXACT,
        // The reference declaration promises nothing about where its candidates
        // lie, which is the widest promise and the one every producer made
        // before the term existed. The tests that are ABOUT the term state
        // their own.
        arithmetic: RankArithmetic::FloatFree,
        domains: CandidateDomains::Unrestricted,
        // And it names no per-row block, which is the honest answer for a
        // producer that restricts nothing: there is no promise for a row to
        // back. The tests that are ABOUT the block column state their own.
        block_position: None,
        exclusion: ExclusionBasis::Unavailable,
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

/// The block column is the candidate column's sibling — a position the consumer
/// *reads* — so it must exist and it may not share a position with a value the
/// invocation *writes*. Each refusal is executed, and so is the valid neighbour
/// that distinguishes "this position is wrong" from "a block column is wrong":
/// the same declaration with the block at a free position registers, under both
/// a restriction and none.
#[test]
fn a_block_position_that_cannot_be_read_back_is_refused_while_a_free_one_registers() {
    // Outside the relation's positions: there is nothing there to read.
    let out_of_range = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                block_position: Some(9),
                ..declaration()
            },
        );
    });
    assert!(
        out_of_range.contains("projects each row's block from position 9"),
        "the refusal names the position, got: {out_of_range}"
    );

    // The candidate's own position: a candidate is not the block it lies in, and
    // one position projects one value.
    let candidate = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                block_position: Some(0),
                ..declaration()
            },
        );
    });
    assert!(
        candidate.contains("both its candidate and each row's block"),
        "the refusal names the collision, got: {candidate}"
    );

    // A placement target: that position carries the request's needle, so reading
    // it back as a block would measure the corpus against the query.
    let placement = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                block_position: Some(1),
                ..declaration()
            },
        );
    });
    assert!(
        placement.contains("value facet") && placement.contains("row's block"),
        "the refusal names the facet it collides with, got: {placement}"
    );

    // The depth target, refused for the same reason and named separately,
    // because the remedy differs: move the depth, or move the block.
    let depth = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                block_position: Some(2),
                ..declaration()
            },
        );
    });
    assert!(
        depth.contains("per-stratum depth at position 2")
            && depth.contains("projects each row's block"),
        "the refusal names the depth collision, got: {depth}"
    );

    // The valid neighbour. Position 3 is free in this relation — it is neither
    // the candidate, a placement target nor the depth — so the declaration
    // registers and reads back unchanged.
    let mut free = PropertyFunctionRegistry::new();
    let declared = RankedDeclaration {
        block_position: Some(3),
        ..declaration()
    };
    free.register_ranked(EX_REL, relation(), declared.clone());
    assert_eq!(
        free.ranked_declaration(EX_REL),
        Some(&declared),
        "a block column at a free position is a perfectly ordinary declaration"
    );

    // And beside a restriction it actually backs, which is what the column is
    // for: several blocks, and a position each row names its own from.
    let mut restricted = PropertyFunctionRegistry::new();
    restricted.register_ranked(
        EX_REL,
        relation(),
        RankedDeclaration {
            domains: CandidateDomains::within([tag(EX_DOMAIN_DOCS), tag(EX_DOMAIN_PEOPLE)]),
            block_position: Some(3),
            ..declaration()
        },
    );
    assert!(
        restricted.ranked_declaration(EX_REL).is_some(),
        "a several-block restriction with a column to back it is the supported configuration"
    );
}

#[test]
fn an_empty_domain_restriction_is_refused_while_a_named_one_registers() {
    // The refusal. `Within` with no blocks says "this producer names nothing",
    // which is not a narrow producer but an unusable one: a consumer holds a
    // producer to this declaration row by row, so the first row it emitted
    // would contradict it.
    let message = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                domains: CandidateDomains::Within(std::collections::BTreeSet::new()),
                ..declaration()
            },
        );
    });
    assert!(
        message.contains("empty set of") && message.contains("CandidateDomains::Unrestricted"),
        "the refusal must name the offence AND the one-line remedy, got: {message}"
    );

    // Nothing was inserted: the refusal runs before the registry is written, the
    // same discipline every other declaration refusal keeps.
    let mut refused = PropertyFunctionRegistry::new();
    let outcome = without_panic_output(|| {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            refused.register_ranked(
                EX_REL,
                relation(),
                RankedDeclaration {
                    domains: CandidateDomains::Within(std::collections::BTreeSet::new()),
                    ..declaration()
                },
            );
        }))
    });
    assert!(outcome.is_err());
    // Both halves, because they are written in order and only one of them is
    // what `is_empty` reports. `register_ranked` inserts the declaration into the
    // ranked side table BEFORE it inserts the relation, and `is_empty` answers
    // over the relations alone — so a refusal that had already recorded the
    // declaration would leave `is_empty` true and this assertion would pass over
    // exactly the state it claims to rule out. The side table is asked directly.
    assert!(
        refused.is_empty(),
        "no relation was inserted before the refusal"
    );
    assert!(
        refused.ranked_declaration(EX_REL).is_none(),
        "and no declaration was recorded in the side table the insert writes first"
    );

    // THE VALID NEIGHBOURS, and they are what this refusal must not touch. A
    // restriction naming one block registers; so does one naming two; so does
    // the unrestricted declaration the whole fixture set uses. Only the empty
    // set is refused.
    for (name, domains) in [
        ("one block", CandidateDomains::within([tag(EX_DOMAIN_DOCS)])),
        (
            "two blocks",
            CandidateDomains::within([tag(EX_DOMAIN_DOCS), tag(EX_DOMAIN_PEOPLE)]),
        ),
        ("unrestricted", CandidateDomains::Unrestricted),
    ] {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(
            EX_REL,
            relation(),
            RankedDeclaration {
                domains: domains.clone(),
                ..declaration()
            },
        );
        assert_eq!(
            registry
                .ranked_declaration(EX_REL)
                .expect("the declaration registered")
                .domains,
            domains,
            "a {name} declaration must register and read back verbatim"
        );
    }

    // A tag reads back as the validated IRI it was built from, not only as
    // text. A consumer holding a declaration off an answer would otherwise have
    // to re-parse a string this type already proved well-formed, and a second
    // parse is a second chance to disagree with the first.
    let parsed = tag(EX_DOMAIN_DOCS);
    assert_eq!(parsed.as_iri().as_str(), EX_DOMAIN_DOCS);

    // And "reads back" means VERBATIM: neither constructing a tag nor reading
    // one normalises, case-folds or re-spells what the host wrote. Over a text
    // that is already canonical — every other tag in this file — an
    // implementation that canonicalised would satisfy every assertion, so the
    // witness below is one tag held against the three respellings generic URI
    // normalisation would conflate it with: an uppercased scheme and authority,
    // a dot segment, and a case-flipped percent-escape.
    const AS_WRITTEN: &str = "http://example.org/domain/documents%2fa";
    let written = tag(AS_WRITTEN);
    assert_eq!(
        written.as_str(),
        AS_WRITTEN,
        "the text accessor hands back the host's spelling, not a canonical one"
    );
    assert_eq!(
        written.as_iri().as_str(),
        AS_WRITTEN,
        "and so does the IRI behind it, so the two accessors cannot disagree"
    );
    assert_eq!(
        DomainTag::new(written.as_iri().clone()).as_str(),
        AS_WRITTEN,
        "and the IRI-taking constructor stores what it is handed, so a tag taken \
         apart and rebuilt is the tag that was taken apart"
    );
    // The consequence a host relies on: each respelling a normaliser would
    // conflate with the above is its own tag, and the difference is one
    // normalisation step in each case, so no single step can be applied
    // anywhere in this type without one of these failing. A domain set that
    // merged a pair would report one block where its host declared two, and a
    // candidate in the second block would then be scored against a declaration
    // that never named it.
    for respelling in [
        "HTTP://EXAMPLE.ORG/domain/documents%2fa",
        "http://example.org/domain/./documents%2fa",
        "http://example.org/domain/documents%2Fa",
    ] {
        let other = tag(respelling);
        assert_ne!(
            written, other,
            "{AS_WRITTEN} and {respelling} are two tags: comparison is over the \
             spelling as written, and nothing canonicalised either side of it"
        );
        assert_eq!(
            other.as_str(),
            respelling,
            "and each reads back as written, so the inequality is two spellings \
             kept rather than one repaired"
        );
    }
}

// ---- one stratum, one producer -------------------------------------------

#[test]
#[should_panic(expected = "already serves; one stratum carries one producer")]
fn a_stratum_a_registered_producer_already_serves_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    registry.register_ranked(EX_OTHER, relation(), declaration());
}

#[test]
fn two_producers_in_two_strata_register_and_read_back_side_by_side() {
    // The valid neighbour: the SAME two declarations, differing only in the one
    // field the refusal is about. A rule that refused a second ranked producer —
    // rather than a second producer under one stratum — would pass the test above
    // and break every multi-modal host there is.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    let second = RankedDeclaration {
        stratum: stratum(EX_STRATUM_B),
        ..declaration()
    };
    registry.register_ranked(EX_OTHER, relation(), second.clone());

    assert_eq!(registry.ranked_declaration(EX_REL), Some(&declaration()));
    assert_eq!(registry.ranked_declaration(EX_OTHER), Some(&second));
}

#[test]
fn the_stratum_refusal_names_the_stratum_both_producers_and_both_exits() {
    // The message IS the finding. A host reading it has to be able to tell which
    // of the two repairs its configuration needs, and naming only "separate
    // strata" would recommend the quiet one: a summing fusion treats each stratum
    // as a summand, so two shards recast as two strata give a candidate they both
    // hold two contributions where the host meant one family's worth.
    let message = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(EX_REL, relation(), declaration());
        registry.register_ranked(EX_OTHER, relation(), declaration());
    });
    assert!(message.contains(EX_STRATUM), "{message}");
    assert!(message.contains(EX_REL), "{message}");
    assert!(message.contains(EX_OTHER), "{message}");
    assert!(
        message.contains("merge them inside ONE producer"),
        "the same-scoring-law exit: {message}"
    );
    assert!(
        message.contains("give each its own stratum"),
        "the different-scoring-law exit: {message}"
    );
}

#[test]
fn the_stratum_refusal_names_the_shared_law_that_still_needs_two_strata() {
    // The third configuration, and the one a two-way "do they share a law" test
    // routes to the wrong exit. Two producers over one embedding space — a
    // heading class and a body class — share a law in every sense, so the merge
    // exit reads as the obvious repair. But if the host means one class to
    // outweigh the other and the score is a bounded metric, the merge cannot
    // carry that weight: the required similarity edge (1 - s)(1 - w_low/w_high)
    // is largest for the worst matches and decays to zero exactly as matches
    // approach perfect, which is where the top-k contest is decided. The loss is
    // silent — an unweighted merge still returns a plausible ranking — so the
    // message must name the case rather than leave it to be inferred.
    let message = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(EX_REL, relation(), declaration());
        registry.register_ranked(EX_OTHER, relation(), declaration());
    });
    assert!(
        message.contains("TWO things, not one"),
        "the first exit takes comparable scores AND a weighting the score can carry: {message}"
    );
    assert!(
        message.contains("weighted differently, and is the score bounded"),
        "the question a host can answer while reading the message: {message}"
    );
    assert!(
        message.contains("(1 - s)(1 - w_low/w_high)"),
        "the margin that vanishes at the top of the list: {message}"
    );
    assert!(
        message.contains("rank space is what a stratum is"),
        "where such a weight can act instead: {message}"
    );
    assert!(
        message.contains("Only an unbounded score"),
        "the score that does carry a differential weight through a merge: {message}"
    );
}

#[test]
fn a_refused_stratum_leaves_the_registry_exactly_as_it_was() {
    // The discipline the duplicate-IRI refusal keeps: validation precedes every
    // write, so a host that catches the panic is not left with a half-registered
    // relation whose declaration is missing.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());
    without_panic_output(|| {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut doomed = registry.clone();
            doomed.register_ranked(EX_OTHER, relation(), declaration());
        }))
        .expect_err("the second claim on one stratum panics");
    });
    assert_eq!(registry.len(), 1, "the original registry is untouched");
    assert_eq!(registry.ranked_declaration(EX_OTHER), None);
}

#[test]
fn a_plainly_registered_relation_claims_no_stratum() {
    // `register` supplies no declaration, so it cannot claim a stratum and cannot
    // be the producer a later claim collides with. The neighbouring valid case
    // for the scan itself: it reads declarations, not registrations.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(EX_OTHER, relation());
    registry.register_ranked(EX_REL, relation(), declaration());
    assert_eq!(registry.ranked_declaration(EX_REL), Some(&declaration()));
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
            exclusion: ExclusionBasis::Unavailable,
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
                exclusion: ExclusionBasis::Unavailable,
                mandatory: false,
                ..base.clone()
            },
        ),
        // Both fidelity axes, and the evidence bytes on each. A declaration that
        // approximates is not the declaration that does not, and two producers
        // disclosing different losses are not the same producer — so all four
        // of these must separate, or two registries that answer differently
        // could share a digest.
        (
            "fidelity lossy",
            RankedDeclaration {
                fidelity: RankFidelity {
                    completeness: Completeness::Lossy {
                        evidence: Arc::from("approximate: beam search, recall unmeasured"),
                    },
                    order: OrderFidelity::Faithful,
                },
                ..base.clone()
            },
        ),
        (
            "fidelity lossy, other evidence",
            RankedDeclaration {
                fidelity: RankFidelity {
                    completeness: Completeness::Lossy {
                        evidence: Arc::from("approximate: sampled, 1 in 8 rows"),
                    },
                    order: OrderFidelity::Faithful,
                },
                ..base.clone()
            },
        ),
        (
            "fidelity perturbed",
            RankedDeclaration {
                fidelity: RankFidelity {
                    completeness: Completeness::Complete,
                    order: OrderFidelity::Perturbed {
                        evidence: Arc::from("quantized: distances compared in 8-bit space"),
                    },
                },
                ..base.clone()
            },
        ),
        (
            "fidelity lossy and perturbed",
            RankedDeclaration {
                fidelity: RankFidelity {
                    completeness: Completeness::Lossy {
                        evidence: Arc::from("approximate: beam search, recall unmeasured"),
                    },
                    order: OrderFidelity::Perturbed {
                        evidence: Arc::from("quantized: distances compared in 8-bit space"),
                    },
                },
                ..base.clone()
            },
        ),
        (
            "domains restricted",
            RankedDeclaration {
                domains: CandidateDomains::within([tag(EX_DOMAIN_DOCS)]),
                ..base.clone()
            },
        ),
        (
            "domains other block",
            RankedDeclaration {
                domains: CandidateDomains::within([tag(EX_DOMAIN_PEOPLE)]),
                ..base.clone()
            },
        ),
        (
            "domains two blocks",
            RankedDeclaration {
                domains: CandidateDomains::within([tag(EX_DOMAIN_DOCS), tag(EX_DOMAIN_PEOPLE)]),
                ..base.clone()
            },
        ),
        (
            "block column declared",
            RankedDeclaration {
                block_position: Some(3),
                ..base.clone()
            },
        ),
        (
            "block column elsewhere",
            RankedDeclaration {
                block_position: Some(4),
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

// ---------------------------------------------------------------------------
// The fidelity term: what a producer promises about its own rows.
// ---------------------------------------------------------------------------

/// A declaration whose completeness axis declares `evidence`.
fn lossy(evidence: &str) -> RankedDeclaration {
    RankedDeclaration {
        fidelity: RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from(evidence),
            },
            order: OrderFidelity::Faithful,
        },
        ..declaration()
    }
}

/// A declaration whose order axis declares `evidence`.
fn perturbed(evidence: &str) -> RankedDeclaration {
    RankedDeclaration {
        fidelity: RankFidelity {
            completeness: Completeness::Complete,
            order: OrderFidelity::Perturbed {
                evidence: Arc::from(evidence),
            },
        },
        ..declaration()
    }
}

#[test]
#[should_panic(expected = "declares a loss on its completeness axis but supplies no evidence")]
fn an_empty_completeness_evidence_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), lossy(""));
}

#[test]
#[should_panic(expected = "declares a loss on its completeness axis but supplies no evidence")]
fn a_whitespace_only_completeness_evidence_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), lossy("   \t\n "));
}

#[test]
#[should_panic(expected = "declares a loss on its order axis but supplies no evidence")]
fn an_empty_order_evidence_is_refused() {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), perturbed(""));
}

#[test]
fn the_refusal_names_the_producer_and_shows_the_way_out() {
    let message = panic_message(|| {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register_ranked(EX_REL, relation(), lossy(""));
    });
    assert!(
        message.contains(EX_REL),
        "the refusal names the producer a host must go and fix: {message}"
    );
    assert!(
        message.contains("declare the exhaustive variant"),
        "the refusal names the neighbouring declaration a producer with nothing to \
         disclose actually wants, so a host cannot read it as 'approximation is \
         unwelcome': {message}"
    );
}

// --- The neighbouring VALID cases. -----------------------------------------
//
// Over-refusal is the mirror of the silent-drop bug and it hides perfectly: a
// refusal LOOKS like correct strictness and every test above passes either way.
// So each refusal is executed beside the nearest input that must still be
// accepted, and these assert acceptance rather than the absence of a panic.

#[test]
fn a_real_evidence_sentence_registers_and_reads_back_verbatim() {
    let evidence = "approximate: recall measured against the exact oracle up to 50,000 rows";
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), lossy(evidence));

    let read_back = registry
        .ranked_declaration(EX_REL)
        .expect("a ranked registration reads its declaration back");
    let Completeness::Lossy {
        evidence: stored, ..
    } = &read_back.fidelity.completeness
    else {
        panic!("the completeness axis kept the declared loss");
    };
    assert_eq!(
        &**stored, evidence,
        "the string the producer published is the string a consumer reads: no \
         normalization, no truncation, no re-wording"
    );
}

#[test]
fn an_exhaustive_declaration_still_registers() {
    // The case the refusal above must NOT catch. A producer with nothing to
    // disclose supplies no evidence on either axis, and that is not an empty
    // evidence string -- it is a different variant, and it registers exactly as
    // it did before the term existed.
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), declaration());

    assert_eq!(
        registry
            .ranked_declaration(EX_REL)
            .expect("an exhaustive producer registers")
            .fidelity,
        RankFidelity::EXACT,
    );
}

#[test]
fn evidence_carrying_the_framing_characters_survives_the_canonical_encoding() {
    // The canonical description frames every string as `<byte-len>:<bytes>` and
    // escapes nothing, so the characters that frame it are exactly the ones a
    // naive encoder would corrupt. A producer's evidence is prose written by a
    // human and will contain them.
    let evidence = "approximate: recall unmeasured at 10^6; see \u{1}note\u{6}\nand the oracle";
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), lossy(evidence));

    let read_back = registry
        .ranked_declaration(EX_REL)
        .expect("a ranked registration reads its declaration back");
    let Completeness::Lossy {
        evidence: stored, ..
    } = &read_back.fidelity.completeness
    else {
        panic!("the completeness axis kept the declared loss");
    };
    assert_eq!(&**stored, evidence, "framing characters survive verbatim");

    // And the encoding still separates it from a neighbour differing by one byte.
    assert_ne!(
        lossy(evidence).canonical_description(),
        lossy(&format!("{evidence}.")).canonical_description(),
    );
}

#[test]
fn both_axes_may_declare_a_loss_at_once() {
    // A quantized approximate index is lossy AND perturbed. The axes are
    // independent, so declaring both is an ordinary declaration, not a
    // contradiction to be refused.
    let decl = RankedDeclaration {
        fidelity: RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::from("approximate: beam search"),
            },
            order: OrderFidelity::Perturbed {
                evidence: Arc::from("quantized: distances compared in 8-bit space"),
            },
        },
        ..declaration()
    };
    let mut registry = PropertyFunctionRegistry::new();
    registry.register_ranked(EX_REL, relation(), decl);

    let read_back = registry
        .ranked_declaration(EX_REL)
        .expect("a doubly-degraded producer registers");
    assert!(read_back.fidelity.may_omit());
    assert!(read_back.fidelity.order_is_unbounded());
    assert_eq!(
        read_back.fidelity.evidence().count(),
        2,
        "a consumer reads both disclosures, in axis order"
    );
}

#[test]
fn exact_declares_nothing_and_claims_nothing() {
    let fidelity = RankFidelity::EXACT;
    assert!(!fidelity.may_omit());
    assert!(!fidelity.order_is_unbounded());
    assert_eq!(fidelity.evidence().count(), 0);
}

/// The reference declaration with every field fixed but its arithmetic.
fn declaration_with(arithmetic: RankArithmetic) -> RankedDeclaration {
    RankedDeclaration {
        arithmetic,
        ..declaration()
    }
}

#[test]
fn the_declared_arithmetic_is_folded_injectively_and_float_free_changes_no_byte() {
    use purrdf_core::distance::{Arithmetic, Exact, Reassociated};

    let float_free = declaration_with(RankArithmetic::FloatFree).canonical_description();
    let exact = declaration_with(RankArithmetic::FloatDistance(
        DeclaredArithmetic::of::<Exact>(),
    ))
    .canonical_description();
    let fast = declaration_with(RankArithmetic::FloatDistance(DeclaredArithmetic::of::<
        Reassociated,
    >()))
    .canonical_description();
    assert_ne!(float_free, exact);
    assert_ne!(float_free, fast);
    assert_ne!(exact, fast, "two laws are two descriptions");
    // A declared law extends the float-free description rather than
    // re-encoding it, so a float-free producer describes itself in exactly the
    // bytes it did before arithmetic was a declared field.
    assert_eq!(
        exact,
        format!("{float_free};a{}:{}", Exact::ID.len(), Exact::ID)
    );
    assert_eq!(
        fast,
        format!(
            "{float_free};a{}:{}",
            Reassociated::ID.len(),
            Reassociated::ID
        )
    );
    // The typed constructor is the variant over the sealed law, not a second
    // spelling of it.
    assert_eq!(
        RankArithmetic::float_distance::<Exact>(),
        RankArithmetic::FloatDistance(DeclaredArithmetic::of::<Exact>())
    );
    assert_ne!(
        RankArithmetic::float_distance::<Reassociated>(),
        RankArithmetic::float_distance::<Exact>()
    );
    assert_eq!(
        DeclaredArithmetic::of::<Exact>().id(),
        "binary64-lane16-tree-v1"
    );
}

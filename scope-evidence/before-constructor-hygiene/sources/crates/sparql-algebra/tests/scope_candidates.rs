// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same candidate comparison and seeded properties run natively and on wasm.

#[path = "support/scope_candidates.rs"]
mod candidates;

use std::collections::BTreeMap;

use candidates::{Case, Contract, cases, distribute, existing, explicit, rename};
use purrdf_testkit::prop::prelude::*;

fn every_legal_control_is_admitted_by_all_candidates() {
    for case in cases().into_iter().filter(|case| case.legal) {
        assert_eq!(existing(&case.after), Ok(()), "{}", case.name);
        assert_eq!(explicit(&case.after), Ok(()), "{}", case.name);
        let contract = Contract::prepare(&case.before).expect("valid frozen input");
        assert_eq!(
            contract.check(&case.after, &case.correspondence),
            Ok(()),
            "{}",
            case.name
        );
    }
}

fn every_intentional_hazard_violates_the_frozen_contract() {
    for case in cases().into_iter().filter(|case| !case.legal) {
        let contract = Contract::prepare(&case.before).expect("valid frozen input");
        assert!(
            contract.check(&case.after, &case.correspondence).is_err(),
            "{}: {}",
            case.name,
            case.claim
        );
    }
}

fn structural_candidates_have_demonstrated_proof_limits() {
    let corpus = cases();
    for name in [
        "lost_connection",
        "merged_witnesses",
        "capture_user",
        "repeated_arm_removal",
    ] {
        let case = corpus
            .iter()
            .find(|case| case.name == name)
            .expect("named mutation");
        assert_eq!(existing(&case.after), Ok(()), "existing: {name}");
        assert_eq!(explicit(&case.after), Ok(()), "explicit: {name}");
    }
    for name in [
        "scope_escape",
        "premature_projection",
        "carrier_alias_collision",
        "template_dataset_confusion",
        "carrier_alias_explicit_output",
    ] {
        let case = corpus
            .iter()
            .find(|case| case.name == name)
            .expect("named mutation");
        assert_eq!(existing(&case.after), Ok(()), "existing: {name}");
        assert!(explicit(&case.after).is_err(), "explicit: {name}");
    }
}

fn recursive_term_claims_are_explicitly_unavailable() {
    for (name, evidence) in candidates::unsupported_terms() {
        assert_eq!(existing(&evidence), Ok(()), "{name}: legal raw match input");
        assert!(
            matches!(
                explicit(&evidence),
                Err(candidates::Diagnostic::UnsupportedTerm(_))
            ),
            "{name}"
        );
        assert!(
            matches!(
                Contract::prepare(&evidence),
                Err(candidates::Diagnostic::UnsupportedTerm(_))
            ),
            "{name}"
        );
    }
}

fn ordinary_projection_and_graph_name_claims_are_explicitly_unavailable() {
    let (before, after) = candidates::projection_boundary();
    assert_eq!(existing(&before), Ok(()));
    assert_eq!(existing(&after), Ok(()));
    assert!(matches!(
        explicit(&before),
        Err(candidates::Diagnostic::UnsupportedProjection(_))
    ));
    assert!(matches!(
        Contract::prepare(&before),
        Err(candidates::Diagnostic::UnsupportedProjection(_))
    ));
    assert!(matches!(
        Contract::prepare(&before).and_then(|contract| contract.check(&after, &BTreeMap::new())),
        Err(candidates::Diagnostic::UnsupportedProjection(_))
    ));
    // No valid frozen predecessor exists with which to certify removal of its
    // projection. The resulting ordinary Join is independently admissible.
    assert_eq!(explicit(&after), Ok(()));
    assert!(Contract::prepare(&after).is_ok());
    let (before, after) = candidates::graph_name_boundary();
    assert!(matches!(
        Contract::prepare(&before).and_then(|contract| contract.check(&after, &BTreeMap::new())),
        Err(candidates::Diagnostic::UnsupportedGraphName(_))
    ));
    for evidence in [before, after] {
        assert_eq!(existing(&evidence), Ok(()));
        assert!(matches!(
            explicit(&evidence),
            Err(candidates::Diagnostic::UnsupportedGraphName(_))
        ));
        assert!(matches!(
            Contract::prepare(&evidence),
            Err(candidates::Diagnostic::UnsupportedGraphName(_))
        ));
    }
}

fn all_measured_inputs_are_stack_safe_and_reusable() {
    for (name, bytes) in candidates::layouts() {
        purrdf_testkit::harness::print_line(&format!(
            "scope_layout pointer_bytes={} type={name} bytes={bytes}",
            size_of::<usize>()
        ));
    }
    for (name, evidence) in candidates::measured_inputs() {
        existing(&evidence).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        explicit(&evidence).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let contract =
            Contract::prepare(&evidence).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(
            contract.check(&evidence, &BTreeMap::new()),
            Ok(()),
            "{name}"
        );
    }
}

fn copied_source_provenance_has_explicit_admission_boundaries() {
    let (before, after) = candidates::raw_owner_boundary();
    assert_eq!(existing(&before), Ok(()));
    assert_eq!(existing(&after), Ok(()));
    assert!(matches!(
        explicit(&before),
        Err(candidates::Diagnostic::AmbiguousSourceOwner(1))
    ));
    assert!(matches!(
        Contract::prepare(&before).and_then(|contract| contract.check(&after, &BTreeMap::new())),
        Err(candidates::Diagnostic::AmbiguousSourceOwner(1))
    ));
    // Both copies inside one actual positive owner remain admissible, while
    // no predecessor certificate exists to establish this merger as legal.
    assert_eq!(explicit(&after), Ok(()));
    assert!(Contract::prepare(&after).is_ok());
    let contradictory = candidates::contradictory_source_boundary();
    assert_eq!(existing(&contradictory), Ok(()));
    assert!(matches!(
        Contract::prepare(&contradictory),
        Err(candidates::Diagnostic::ContradictorySourceSite(1))
    ));
}

fn template_graph_and_predicate_positions_have_explicit_proof_boundaries() {
    for (boundary, expected) in [
        (candidates::template_graph_boundary(), true),
        (candidates::template_predicate_boundary(), false),
    ] {
        let (before, after) = boundary;
        for evidence in [&before, &after] {
            assert_eq!(existing(evidence), Ok(()));
            let diagnostic = explicit(evidence).expect_err("unsupported template position");
            if expected {
                assert!(matches!(
                    diagnostic,
                    candidates::Diagnostic::UnsupportedTemplateGraphName(_)
                ));
            } else {
                assert_eq!(diagnostic, candidates::Diagnostic::MissingProvenance);
            }
        }
        assert!(
            Contract::prepare(&before)
                .and_then(|contract| contract.check(&after, &BTreeMap::new()))
                .is_err()
        );
    }
}

fn query_head_observers_have_explicit_proof_boundaries() {
    let (before, after) = candidates::describe_boundary();
    for evidence in [&before, &after] {
        assert_eq!(existing(evidence), Ok(()));
        assert_eq!(
            explicit(evidence),
            Err(candidates::Diagnostic::UnsupportedQueryForm(
                candidates::QueryForm::Describe
            ))
        );
    }
    assert!(matches!(
        Contract::prepare(&before).and_then(|contract| contract.check(&after, &BTreeMap::new())),
        Err(candidates::Diagnostic::UnsupportedQueryForm(
            candidates::QueryForm::Describe
        ))
    ));
    let ask = candidates::ask_boundary();
    assert_eq!(existing(&ask), Ok(()));
    assert!(matches!(
        Contract::prepare(&ask),
        Err(candidates::Diagnostic::UnsupportedQueryForm(
            candidates::QueryForm::Ask
        ))
    ));
}

fn structural_hazards() -> Vec<Case> {
    cases()
        .into_iter()
        .filter(|case| {
            !case.legal
                && candidates::supports_reencoding(&case.before)
                && candidates::supports_reencoding(&case.after)
        })
        .collect()
}

fn category_cases() -> Vec<Case> {
    cases()
        .into_iter()
        .filter(|case| {
            !candidates::supports_reencoding(&case.before)
                || !candidates::supports_reencoding(&case.after)
        })
        .collect()
}

fn property_partitions_cover_every_hazard() {
    let all: Vec<_> = cases()
        .into_iter()
        .filter(|case| !case.legal)
        .map(|case| case.name)
        .collect();
    let covered: Vec<_> = structural_hazards()
        .into_iter()
        .chain(category_cases().into_iter().filter(|case| !case.legal))
        .map(|case| case.name)
        .collect();
    assert_eq!(all.len(), covered.len());
    for name in all {
        assert_eq!(
            covered.iter().filter(|covered| **covered == name).count(),
            1,
            "{name}"
        );
    }
}

prop_test! {
    #![prop_config(Config::with_cases(128))]
    fn distribution_and_alpha_renaming_preserve_frozen_relationships(
        rounds in 1_u32..8, suffix in 0_u32..4096, distributed in any::<bool>(),
        branches in 2_u32..8, width in 1_u32..8, junctions in 1_u32..8,
        duplicate_arms in 0_u32..6
    ) {
        let original = candidates::generated_fixture(branches, width, junctions, duplicate_arms);
        let contract = Contract::prepare(&original).expect("valid original");
        let input = if distributed { distribute(&original) } else { original };
        let (mut renamed, correspondence) = rename(&input, suffix);
        prop_assert_eq!(contract.check(&renamed, &correspondence), Ok(()));
        for _ in 1..rounds {
            let step = Contract::prepare(&renamed).expect("valid intermediate");
            let (next, next_correspondence) = candidates::carrier(&renamed);
            prop_assert_eq!(step.check(&next, &next_correspondence), Ok(()));
            renamed = next;
        }
    }

    fn intentional_hazards_survive_consistent_reencoding(
        index in 0_usize..structural_hazards().len(), suffix in 0_u32..4096
    ) {
        let corpus = structural_hazards();
        let case = &corpus[index];
        let contract = Contract::prepare(&case.before).expect("valid frozen input");
        prop_assert!(contract.check(&case.after, &case.correspondence).is_err());
        // Renaming both sides does not repair an already incorrect partition.
        let (before, _) = rename(&case.before, suffix);
        let step = Contract::prepare(&before).expect("valid reencoded predecessor");
        let (after, _) = rename(&case.after, suffix);
        prop_assert!(step.check(&after, &BTreeMap::new()).is_err());
    }

    fn template_and_values_categories_survive_row_variation_and_binder_renumbering(
        copies in 1_usize..7, offset in 1_u32..4096, checks in 1_usize..5
    ) {
        // Every category case is exercised in every generated property input.
        // This transform supports their actual operators and never uses the
        // positive SELECT-only variable reencoder on VALUES or a template.
        for case in category_cases() {
            let before = candidates::repeated_input(&case.before, copies);
            let contract = Contract::prepare(&before).expect("valid frozen category input");
            let after = candidates::renumber_binders(&candidates::repeated_input(&case.after, copies), offset);
            for _ in 0..checks {
                prop_assert_eq!(contract.check(&after, &case.correspondence).is_ok(), case.legal);
            }
        }
    }
}

purrdf_testkit::harness_main!(
    every_legal_control_is_admitted_by_all_candidates,
    every_intentional_hazard_violates_the_frozen_contract,
    structural_candidates_have_demonstrated_proof_limits,
    recursive_term_claims_are_explicitly_unavailable,
    ordinary_projection_and_graph_name_claims_are_explicitly_unavailable,
    all_measured_inputs_are_stack_safe_and_reusable,
    copied_source_provenance_has_explicit_admission_boundaries,
    template_graph_and_predicate_positions_have_explicit_proof_boundaries,
    query_head_observers_have_explicit_proof_boundaries,
    property_partitions_cover_every_hazard,
    distribution_and_alpha_renaming_preserve_frozen_relationships,
    intentional_hazards_survive_consistent_reencoding,
    template_and_values_categories_survive_row_variation_and_binder_renumbering,
);

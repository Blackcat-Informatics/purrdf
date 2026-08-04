// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL-SPARQL and SHACL-AF inherit the SPARQL evaluator's execution governors.
//!
//! # What this file is for
//!
//! `sh:sparql`, `sh:SPARQLTarget`, `sh:SPARQLRule`, `sh:ask`/`sh:select` validators and
//! every SHACL-AF node expression run their bodies on `NativeSparqlEngine`. A caller who
//! bounds a query and then reaches the same evaluator through a validation would, without
//! this, have bounded nothing: validation is an unbounded fan-out of one query per focus
//! node, which is exactly the shape a budget exists to contain.
//!
//! Inheritance was **not** there — every SHACL path went through the ungoverned engine
//! entry points, and the crate had no budget concept at all to hang one on. It is wired
//! now, and the tests below drive the public surface to prove it rather than assuming it:
//! a `sh:sparql` constraint under a tripping budget must be observed to trip.
//!
//! # A trip is not a report
//!
//! Every SHACL constraint is a *negative* claim — "no solution of this query violates the
//! shape". A truncated solution bag and a complete one that found nothing produce the
//! identical sentence, so `conforms` computed from a partial answer means nothing. That is
//! why the governed validation surface has no partial report: a trip yields the trip and
//! the evidence, and the verdict is withheld. These tests pin that too, because a
//! surface that quietly returned `conforms: true` for a validation that ran out of budget
//! is the failure this whole tier exists to prevent.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{GovernedValidation, validate_dataset_with_governors};
use purrdf_sparql_eval::{
    CancellationFlag, QueryGovernors, ResourceDimension, StopCause, TrippedGovernor,
};

/// Test fixtures use `example.org`; PurRDF mints no vocabulary IRIs.
const EX: &str = "http://example.org/";

/// A shapes graph whose only constraint is a `sh:sparql` (SHACL-SPARQL) body, so every
/// violation this validation can find has to come through the SPARQL evaluator.
fn sparql_constraint_shapes() -> String {
    format!(
        r#"
@prefix sh:   <http://www.w3.org/ns/shacl#> .
@prefix ex:   <{EX}> .

ex:PersonShape
    a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "every person must have a name" ;
        sh:select """
            SELECT $this ?value
            WHERE {{
                $this <{EX}nickname> ?value .
            }}
        """ ;
    ] .
"#
    )
}

/// `count` people, each with one nickname, so the `sh:sparql` body reports one violation
/// per focus node and the validation is a genuine per-focus fan-out of SPARQL queries.
fn people(count: usize) -> Arc<RdfDataset> {
    let mut triples = String::new();
    for index in 0..count {
        writeln!(
            triples,
            "<{EX}p{index}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Person> .\n\
             <{EX}p{index}> <{EX}nickname> \"nick{index}\" ."
        )
        .expect("writing to a String cannot fail");
    }
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

fn shapes() -> purrdf_shapes::shapes::Shapes {
    purrdf_shapes::engine::parse_shapes(&sparql_constraint_shapes()).expect("parse shapes")
}

/// Validate `data` under `governors`.
fn validate(data: &RdfDataset, governors: &QueryGovernors) -> GovernedValidation {
    validate_dataset_with_governors(data, &shapes(), None, governors)
        .expect("a tripped governor is an outcome, not a validation failure")
}

#[test]
fn shacl_sparql_constraints_inherit_governors() {
    let data = people(12);

    // First: the validation really is driven by SPARQL, and it really does find
    // violations. Without this the trip below could be a validation that did nothing.
    let metered = validate(&data, &QueryGovernors::METERED);
    let GovernedValidation::Complete { report, evidence } = &metered else {
        panic!("METERED bounds nothing and cannot trip: {metered:?}");
    };
    assert!(!report.conforms, "the sh:sparql constraint must fire");
    assert_eq!(
        report.results.len(),
        12,
        "one violation per person, all of them found through the SPARQL evaluator"
    );

    // The meter ran. This is the half that proves inheritance rather than merely
    // accepting a `QueryGovernors` argument and dropping it: an ungoverned SHACL path
    // would report zero fuel however many queries it ran.
    let cost = evidence.consumed_in(ResourceDimension::Fuel);
    assert!(
        cost > 0,
        "the SHACL-SPARQL bodies must charge this validation's budget: {evidence:?}"
    );
    assert!(evidence.is_complete(), "nothing tripped: {evidence:?}");

    // And the budget is per VALIDATION, not per focus node. Twelve people cost more than
    // three, which they could not if each focus node had been handed its own fresh
    // ceiling — the failure mode that would silently multiply a caller's budget by the
    // size of their data.
    let smaller = validate(&people(3), &QueryGovernors::METERED);
    let smaller_cost = smaller.evidence().consumed_in(ResourceDimension::Fuel);
    assert!(
        smaller_cost > 0 && smaller_cost < cost,
        "one state spans the whole validation, so cost grows with the focus set: \
         3 people cost {smaller_cost}, 12 cost {cost}"
    );

    // Now the trip. Half the measured cost cannot finish, and the outcome must be the
    // typed exhaustion naming the governor — never a report.
    let starved = validate(&data, &QueryGovernors::UNBOUNDED.with_fuel(cost / 2));
    let GovernedValidation::BudgetExhausted { tripped, evidence } = &starved else {
        panic!("half the measured cost cannot finish the validation: {starved:?}");
    };
    assert!(
        matches!(
            tripped,
            TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                ..
            }
        ),
        "the trip names the governor that fired: {tripped:?}"
    );
    assert_eq!(
        evidence.tripped,
        Some(*tripped),
        "the evidence and the outcome report one trip"
    );
    assert_eq!(starved.tripped(), Some(*tripped));

    // The whole measured cost completes, so the trip above is the budget and not a
    // validation that cannot run at all.
    let afforded = validate(&data, &QueryGovernors::UNBOUNDED.with_fuel(cost));
    let GovernedValidation::Complete { report, .. } = &afforded else {
        panic!("the measured cost is a sufficient budget: {afforded:?}");
    };
    assert_eq!(
        report.results.len(),
        12,
        "governing a validation must not change its verdict when the budget suffices"
    );

    // A stop signal reaches the SHACL paths too — the governor a host actually uses to
    // cancel work it no longer wants, and the one that must not need a fuel figure to be
    // set first.
    let cancelled = CancellationFlag::new();
    cancelled.cancel();
    let stopped = validate(
        &data,
        &QueryGovernors::UNBOUNDED.with_stop_signal(Arc::new(cancelled)),
    );
    assert_eq!(
        stopped.tripped(),
        Some(TrippedGovernor::Stopped {
            cause: StopCause::Cancelled
        }),
        "a cancelled host must be able to stop a validation: {stopped:?}"
    );
}

#[test]
fn an_exhausted_validation_never_reports_a_verdict() {
    // The failure this surface exists to prevent, stated on its own: a validation that
    // ran out of budget must not come back as `conforms`. Every SHACL constraint is a
    // negative claim, so a truncated solution bag and a complete empty one say the same
    // words — and one of them is a lie.
    //
    // The sweep is over the whole low range because the exact fuel at which the trip
    // lands is a property of the charge schedule, not of this test: every outcome must be
    // either an honest exhaustion or a genuinely complete verdict, and both must occur.
    let data = people(6);
    let cost = validate(&data, &QueryGovernors::METERED)
        .evidence()
        .consumed_in(ResourceDimension::Fuel);
    assert!(cost > 0, "the fixture must cost something");

    let mut exhausted = 0_usize;
    let mut complete = 0_usize;
    for fuel in 0..cost {
        match validate(&data, &QueryGovernors::UNBOUNDED.with_fuel(fuel)) {
            GovernedValidation::BudgetExhausted { evidence, .. } => {
                exhausted += 1;
                assert!(
                    evidence.tripped.is_some(),
                    "an exhausted validation names its governor: {evidence:?}"
                );
            }
            GovernedValidation::Complete { report, .. } => {
                // Only reachable if the validation genuinely finished under this budget;
                // then its verdict must be the true one.
                complete += 1;
                assert_eq!(
                    report.results.len(),
                    6,
                    "a complete verdict under a small budget must still be the true one"
                );
            }
        }
    }
    assert!(
        exhausted > 0,
        "no budget below the measured cost stopped the validation, so the exhaustion \
         path was never reached"
    );
    assert_eq!(
        exhausted + complete,
        cost as usize,
        "every budget produced exactly one of the two outcomes"
    );
}

#[test]
fn an_ungoverned_validation_is_unchanged() {
    // The D0 shape, restated for this tier: the governed entry point under `UNBOUNDED`
    // must agree with the ordinary ungoverned validation, report for report. A tier that
    // only behaves when it is being watched has changed the answer for everyone else.
    let data = people(9);
    let shapes = shapes();
    let plain = purrdf_shapes::engine::validate_dataset_with_shapes_graph(&data, &shapes, None)
        .expect("ungoverned validation");

    let governed = validate(&data, &QueryGovernors::UNBOUNDED);
    let GovernedValidation::Complete { report, evidence } = &governed else {
        panic!("UNBOUNDED engages no ceiling, so nothing can trip: {governed:?}");
    };
    assert_eq!(report.conforms, plain.conforms);
    assert_eq!(report.results.len(), plain.results.len());
    for (governed_result, plain_result) in report.results.iter().zip(plain.results.iter()) {
        assert_eq!(
            format!("{governed_result:?}"),
            format!("{plain_result:?}"),
            "governing a validation must not change a single result"
        );
    }
    assert_eq!(
        evidence.consumed_in(ResourceDimension::Fuel),
        0,
        "UNBOUNDED declines the accounting as well as the ceilings, so it must record no \
         governed fuel consumption"
    );
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The stored-fact and join-step limits through the shared boundaries every host calls:
//! the rules tool ([`apply_rules_to_ntriples`]) and regime materialization
//! ([`materialize_to_nquads_string_with`]). A refusal names the limit, the numbers and the
//! knob in the calling host's own spelling; the neighbour one fact or step inside the
//! limit completes.

use purrdf_validate::{
    MaterializeLimits, RegimeHost, RulesHost, RulesRequest, apply_rules_to_ntriples,
    materialize_to_nquads_string, materialize_to_nquads_string_with,
};

/// Five `ex:p` triples.
const DATA: &str = "<http://example.org/s0> <http://example.org/p> <http://example.org/o0> .\n\
<http://example.org/s1> <http://example.org/p> <http://example.org/o1> .\n\
<http://example.org/s2> <http://example.org/p> <http://example.org/o2> .\n\
<http://example.org/s3> <http://example.org/p> <http://example.org/o3> .\n\
<http://example.org/s4> <http://example.org/p> <http://example.org/o4> .\n";

/// A SPARQL 1.2 RL rule set copying every `ex:p` triple to `ex:q`: ten stored facts.
const COPY: &str = "PREFIX ex: <http://example.org/>\nRULE { ?s ex:q ?o } WHERE { ?s ex:p ?o }\n";

/// The same copy as a SHACL global SPARQL rule.
const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
@prefix ex: <http://example.org/> .\n\
ex:copy a sh:SPARQLRule ; sh:construct \"\"\"PREFIX ex: <http://example.org/>\n\
CONSTRUCT { ?s ex:q ?o } WHERE { ?s ex:p ?o }\"\"\" .\n";

/// Every rules host and the knob spellings its refusals must carry.
const RULES_HOSTS: [(RulesHost, &str, &str); 4] = [
    (
        RulesHost::Rust,
        "RulesRequest::max_stored_facts",
        "RulesRequest::max_join_steps",
    ),
    (
        RulesHost::Python,
        "apply_rules(max_stored_facts=...)",
        "apply_rules(max_join_steps=...)",
    ),
    (
        RulesHost::Wasm,
        "shaclApplyRules's maxStoredFacts",
        "shaclApplyRules's maxJoinSteps",
    ),
    (
        RulesHost::CAbi,
        "purrdf_shacl_apply_rules's max_stored_facts",
        "purrdf_shacl_apply_rules's max_join_steps",
    ),
];

fn rules(
    shapes: bool,
    host: RulesHost,
    facts: Option<u64>,
    steps: Option<u64>,
) -> Result<String, String> {
    apply_rules_to_ntriples(&RulesRequest {
        data_nt: DATA,
        shapes_ttl: shapes.then_some(SHAPES),
        srl: (!shapes).then_some(COPY),
        max_stored_facts: facts,
        max_join_steps: steps,
        host,
        ..RulesRequest::default()
    })
    .map(|outcome| outcome.inferred_ntriples)
    .map_err(|error| error.to_string())
}

#[test]
fn a_rules_refusal_names_the_hosts_knob_and_the_neighbour_completes() {
    for shapes in [true, false] {
        for (host, stored_facts, join_steps) in RULES_HOSTS {
            let refused =
                rules(shapes, host, Some(9), None).expect_err("ten facts, nine permitted");
            assert!(
                refused.contains(
                    "the rules exceeded the stored-fact limit: 10 facts observed, 9 permitted \
                     (the caller's limit)"
                ) && refused.ends_with(&format!("raise it with {stored_facts}")),
                "{host:?} {shapes}: {refused}"
            );
            let inferred = rules(shapes, host, Some(10), None).expect("ten facts permitted");
            assert_eq!(inferred.lines().count(), 5, "{host:?} {shapes}: {inferred}");

            let refused = rules(shapes, host, None, Some(1)).expect_err("one join step");
            assert!(
                refused.contains("the rules exceeded the join-step limit: ")
                    && refused.contains(" permitted (the caller's limit)")
                    && refused.ends_with(&format!("raise it with {join_steps}")),
                "{host:?} {shapes}: {refused}"
            );
        }
    }
}

/// Every materialization host and the knob spellings its refusals must carry.
const REGIME_HOSTS: [(RegimeHost, &str, &str); 6] = [
    (
        RegimeHost::Rust,
        "MaterializeLimits::max_stored_facts",
        "MaterializeLimits::max_join_steps",
    ),
    (
        RegimeHost::Python,
        "materialize(max_stored_facts=...)",
        "materialize(max_join_steps=...)",
    ),
    (
        RegimeHost::PythonText,
        "materialize_nt(max_stored_facts=...)",
        "materialize_nt(max_join_steps=...)",
    ),
    (
        RegimeHost::Wasm,
        "entailMaterialize's maxStoredFacts",
        "entailMaterialize's maxJoinSteps",
    ),
    (
        RegimeHost::CAbi,
        "purrdf_entail_materialize_to_nquads's max_stored_facts",
        "purrdf_entail_materialize_to_nquads's max_join_steps",
    ),
    (RegimeHost::Cli, "--max-stored-facts", "--max-join-steps"),
];

#[test]
fn a_materialization_refusal_names_the_hosts_knob_and_the_neighbour_completes() {
    // `rdfs` runs through the restricted chase and `owl-rl` through the semi-naive
    // evaluator, so both refusal paths are observed.
    for regime in ["rdfs", "owl-rl"] {
        let unlimited = materialize_to_nquads_string(regime, DATA, "").expect("the default");
        let stored: usize = unlimited
            .report()
            .lines()
            .find_map(|line| line.strip_prefix("budget stored-facts "))
            .expect("a stored-facts line")
            .parse()
            .expect("a count");
        for (host, stored_facts, join_steps) in REGIME_HOSTS {
            let limits = |facts: Option<u64>, steps: Option<u64>| MaterializeLimits {
                max_stored_facts: facts,
                max_join_steps: steps,
                host,
            };
            let short = limits(Some(stored as u64 - 1), None);
            let refused = materialize_to_nquads_string_with(regime, DATA, "", &short)
                .expect_err("one fact short");
            assert!(
                refused.starts_with(&format!(
                    "entailment regime \"{regime}\": evaluation exceeded the stored-fact \
                     limit: "
                )) && refused.contains(&format!("{} permitted (the caller's limit)", stored - 1))
                    && refused.ends_with(&format!("raise it with {stored_facts}")),
                "{regime} {host:?}: {refused}"
            );
            let exact = limits(Some(stored as u64), None);
            let closed = materialize_to_nquads_string_with(regime, DATA, "", &exact)
                .expect("exactly the store it needs");
            assert_eq!(closed.nquads(), unlimited.nquads(), "{regime} {host:?}");

            let refused =
                materialize_to_nquads_string_with(regime, DATA, "", &limits(None, Some(1)))
                    .expect_err("one join step");
            assert!(
                refused.contains("evaluation exceeded the join-step limit: ")
                    && refused.ends_with(&format!("raise it with {join_steps}")),
                "{regime} {host:?}: {refused}"
            );
        }
    }
}

/// The limits in force are part of the report's identity: the same closure under other
/// limits names a different calculus, and stating the target's defaults names the
/// default one.
#[test]
fn the_limits_in_force_reach_the_reports_contract_hash() {
    let hash = |limits: &MaterializeLimits| {
        materialize_to_nquads_string_with("owl-rl", DATA, "", limits)
            .expect("closes")
            .report()
            .lines()
            .find(|line| line.starts_with("contract-hash "))
            .expect("a contract hash")
            .to_owned()
    };
    let default = hash(&MaterializeLimits::default());
    let restated = hash(&MaterializeLimits {
        max_stored_facts: Some(purrdf_datalog::seminaive::DEFAULT_MAX_STORED_FACTS),
        max_join_steps: Some(purrdf_datalog::seminaive::DEFAULT_MAX_JOIN_STEPS),
        host: RegimeHost::Rust,
    });
    assert_eq!(default, restated);
    let raised = hash(&MaterializeLimits {
        max_stored_facts: Some(purrdf_datalog::seminaive::DEFAULT_MAX_STORED_FACTS * 2),
        ..MaterializeLimits::default()
    });
    assert_ne!(default, raised);
}

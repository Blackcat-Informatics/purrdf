// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The stored-fact and join-step limits through the rules engine's public API: ordinary
//! terminating workloads complete with unlimited unstated capacity, the same workloads
//! under finite caller-selected settings refuse naming the relevant control, and raising
//! it admits them.
//!
//! Every refusal is paired with the neighbour that must still succeed, and each pair
//! differs in the one limit the test reads: a limit of exactly the observed count admits
//! the run, and one fewer refuses it.

use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_rdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{self, parse_shapes};
use purrdf_shapes::rules::{RuleOptions, infer};
use purrdf_shapes::srl::{self, InferOptions};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/ns#> .
";

/// The copied triples of the copy workload.
const COPIED: usize = 70_000;

/// The edges of the closure workload's chain.
const CHAIN: usize = 1_000;

/// The closure of a `CHAIN`-edge chain: every ordered pair of its `CHAIN + 1` nodes.
const CLOSURE: usize = CHAIN * (CHAIN + 1) / 2;

/// A non-recursive `sh:TripleRule` copying every `ex:p` value to `ex:q`.
const COPY: &str = "ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:p ;
  sh:rule [ a sh:TripleRule ; sh:subject sh:this ; sh:predicate ex:q ;
            sh:object [ sh:path ex:p ] ] .";

/// The transitive closure of `ex:link` as two global `sh:SPARQLRule`s.
const SHACL_CLOSURE: &str = r#"ex:base a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?x ex:connected ?y } WHERE { ?x ex:link ?y }" .
ex:step a sh:SPARQLRule ; sh:construct
  "CONSTRUCT { ?x ex:connected ?z } WHERE { ?x ex:connected ?y . ?y ex:link ?z }" ."#;

/// The same closure as a SPARQL 1.2 RL rule set.
const SRL_CLOSURE: &str = "PREFIX ex: <http://example.org/ns#>
RULE { ?x ex:connected ?y } WHERE { ?x ex:link ?y }
RULE { ?x ex:connected ?z } WHERE { ?x ex:connected ?y . ?y ex:link ?z }
";

fn dataset(ttl: &str) -> Arc<RdfDataset> {
    parse_turtle_to_dataset(&format!("{PREFIXES}\n{ttl}"), None).expect("data parses")
}

fn holder(ttl: &str) -> ShaclData {
    let projected = engine::project_dataset(dataset(ttl).as_ref()).expect("projects");
    ShaclData::new(Arc::clone(&projected), projected, None)
}

/// `n` triples `ex:s{i} ex:p ex:o{i}`.
fn copies(n: usize) -> String {
    let mut text = String::new();
    for i in 0..n {
        writeln!(text, "ex:s{i} ex:p ex:o{i} .").expect("write to String");
    }
    text
}

/// A chain `ex:n0 ex:link ex:n1 … ex:n{n}`.
fn chain(n: usize) -> String {
    let mut text = String::new();
    for i in 0..n {
        writeln!(text, "ex:n{i} ex:link ex:n{} .", i + 1).expect("write to String");
    }
    text
}

/// Run a SHACL shapes graph's rules and return the number of inferred triples.
fn shacl(data: &ShaclData, rules: &str, options: &RuleOptions) -> Result<usize, String> {
    let shapes =
        parse_shapes(&format!("{PREFIXES}\n{rules}"), None).map_err(|error| error.to_string())?;
    infer(data, &shapes, options)
        .map(|inference| inference.inferred().len())
        .map_err(|error| error.to_string())
}

/// Run a SPARQL 1.2 RL rule set and return the number of inferred triples.
fn srl_run(data: &RdfDataset, rules: &str, options: &InferOptions) -> Result<usize, String> {
    let document = srl::parse_and_check(rules, None).map_err(|e| e.to_string())?;
    srl::infer(&document, data, options)
        .map(|inference| inference.inferred().len())
        .map_err(|e| e.to_string())
}

/// An explicitly selected finite capacity profile, independent of target.
fn finite_capacity_profile() -> RuleOptions {
    RuleOptions::default()
        .with_max_stored_facts(131_072)
        .with_max_join_steps(1_048_576)
}

/// A single non-recursive rule copying 70,000 triples holds 140,000 facts: past the
/// selected ceiling of 131,072. Unstated capacity admits every copy; selecting the
/// finite profile refuses the run naming the numbers and the Rust
/// knob; a limit of exactly 140,000 admits it and 139,999 refuses it.
#[test]
fn a_seventy_thousand_triple_copy_completes_natively_and_names_the_knob_on_wasm() {
    let data = holder(&copies(COPIED));
    assert_eq!(
        shacl(&data, COPY, &RuleOptions::default()),
        Ok(COPIED),
        "unstated capacity admits every copy"
    );
    assert_eq!(
        shacl(&data, COPY, &finite_capacity_profile()),
        Err(
            "SHACL rules did not complete: the rules exceeded the stored-fact limit: 131073 \
             facts observed, 131072 permitted (the caller's limit); raise it with \
             RuleOptions::with_max_stored_facts"
                .to_owned()
        ),
    );
    let exact = finite_capacity_profile().with_max_stored_facts(2 * COPIED as u64);
    assert_eq!(shacl(&data, COPY, &exact), Ok(COPIED));
    let short = finite_capacity_profile().with_max_stored_facts(2 * COPIED as u64 - 1);
    let refused = shacl(&data, COPY, &short).expect_err("one fact short");
    assert!(
        refused.contains("140000 facts observed, 139999 permitted"),
        "{refused}"
    );
}

/// Both public rule entry points admit retained surfaces beyond the former arena cap.
#[test]
fn unstated_arena_capacity_reaches_both_rule_frontends() {
    let oversized = "x".repeat((1 << 24) + 1);
    let text = format!("ex:s ex:p \"{oversized}\" .");
    assert_eq!(shacl(&holder(&text), COPY, &RuleOptions::default()), Ok(1));
    assert_eq!(
        srl_run(
            &dataset(&text),
            "PREFIX ex: <http://example.org/ns#> RULE { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
            &InferOptions::default(),
        ),
        Ok(1)
    );
}

/// The transitive closure of a 1,000-edge chain — 500,500 inferred triples — completes
/// through both frontends, and under a finite profile each frontend refuses
/// it at the stored-fact limit naming its own knob.
#[test]
fn a_thousand_node_transitive_closure_completes_natively_through_both_frontends() {
    let text = chain(CHAIN);
    let data = holder(&text);
    assert_eq!(
        shacl(&data, SHACL_CLOSURE, &RuleOptions::default()),
        Ok(CLOSURE)
    );
    let refused =
        shacl(&data, SHACL_CLOSURE, &finite_capacity_profile()).expect_err("past 131,072");
    assert!(
        refused.starts_with(
            "SHACL rules did not complete: the rules exceeded the stored-fact limit: "
        ) && refused.contains(" facts observed, 131072 permitted (the caller's limit)")
            && refused.ends_with("raise it with RuleOptions::with_max_stored_facts"),
        "{refused}"
    );

    let rdf = dataset(&text);
    assert_eq!(
        srl_run(&rdf, SRL_CLOSURE, &InferOptions::default()),
        Ok(CLOSURE)
    );
    let refused = srl_run(
        &rdf,
        SRL_CLOSURE,
        &InferOptions::default()
            .with_max_stored_facts(131_072)
            .with_max_join_steps(1_048_576),
    )
    .expect_err("past 131,072");
    assert!(
        refused.starts_with(
            "SPARQL 1.2 RL evaluation failed: the rules exceeded the stored-fact limit: "
        ) && refused.contains(" facts observed, 131072 permitted (the caller's limit)")
            && refused.ends_with("raise it with InferOptions::with_max_stored_facts"),
        "{refused}"
    );
    let raised = InferOptions::default()
        .with_max_stored_facts((CLOSURE + CHAIN) as u64)
        .with_max_join_steps(1_048_576);
    assert_eq!(
        srl_run(&rdf, SRL_CLOSURE, &raised),
        Ok(CLOSURE),
        "the base graph plus the closure is exactly the store it needs, and the linear \
         closure's join steps fit the selected finite profile"
    );
}

/// The join-step limit names its own knob: the 150-node closure under a limit of 20,000
/// join steps is refused naming the limit, and the default admits it.
#[test]
fn the_join_step_limit_names_its_knob_and_its_neighbour_completes() {
    let rdf = dataset(&chain(150));
    let tight = InferOptions::default().with_max_join_steps(20_000);
    assert_eq!(
        srl_run(&rdf, SRL_CLOSURE, &tight),
        Err(
            "SPARQL 1.2 RL evaluation failed: the rules exceeded the join-step limit: 20001 \
             join steps observed, 20000 permitted (the caller's limit); raise it with \
             InferOptions::with_max_join_steps"
                .to_owned()
        )
    );
    assert_eq!(
        srl_run(&rdf, SRL_CLOSURE, &InferOptions::default()),
        Ok(150 * 151 / 2)
    );
}

/// A rule set that never stops computing new terms is refused by a TERM limit even when
/// both capacity limits are raised far past the native defaults: the round limit for a
/// counter, the generated-term budget for strings that double every iteration. The
/// refusal names the term limit, never the capacity limits.
#[test]
fn a_divergent_rule_set_is_refused_by_a_term_limit_with_the_capacity_limits_raised() {
    let raised = RuleOptions::default()
        .with_max_stored_facts(4_194_304 * 16)
        .with_max_join_steps(1_048_576 * 16);
    let counter = r#"ex:count a sh:SPARQLRule ; sh:construct
             "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n BIND (?n + 1 AS ?m) }" ."#;
    let refused = shacl(&holder("ex:a ex:n 0 ."), counter, &raised).expect_err("never stops");
    assert!(
        refused.starts_with(
            "SHACL rules did not complete: the rules exceeded the term-generating round limit"
        ),
        "{refused}"
    );
    assert!(!refused.contains("stored-fact"), "{refused}");
    assert!(!refused.contains("join-step"), "{refused}");

    let doubling = r#"ex:a a sh:SPARQLRule ; sh:construct
        "CONSTRUCT { ?s ex:name ?m } WHERE { ?s ex:name ?n BIND (CONCAT(?n, 'a') AS ?m) }" .
      ex:b a sh:SPARQLRule ; sh:construct
        "CONSTRUCT { ?s ex:name ?m } WHERE { ?s ex:name ?n BIND (CONCAT(?n, 'b') AS ?m) }" ."#;
    let refused =
        shacl(&holder(r#"ex:s ex:name "" ."#), doubling, &raised).expect_err("doubles forever");
    assert!(
        refused.starts_with(
            "SHACL rules did not complete: the rules exceeded the generated-term budget"
        ),
        "{refused}"
    );
    assert!(!refused.contains("stored-fact"), "{refused}");

    // The bounded neighbour of the counter completes under the same raised limits.
    let bounded = r#"ex:count a sh:SPARQLRule ; sh:construct
             "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n FILTER (?n < 1000) BIND (?n + 1 AS ?m) }" ."#;
    assert_eq!(shacl(&holder("ex:a ex:n 0 ."), bounded, &raised), Ok(1000));
}

/// A SHAPE rule (executed once per focus node) that mints one new focus node every
/// iteration: each `ex:C` counts one past its `ex:n` and names the next counter by that
/// number. `{bound}` is spliced into the `WHERE` clause.
fn per_focus_counter(bound: &str) -> String {
    format!(
        r#"ex:Counter a sh:NodeShape ; sh:targetClass ex:C ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """PREFIX ex: <http://example.org/ns#>
CONSTRUCT {{ ?next a ex:C ; ex:n ?m . $this ex:next ?next . }}
WHERE {{ $this ex:n ?n . {bound} BIND (?n + 1 AS ?m)
        BIND (IRI(CONCAT("http://example.org/ns#c", STR(?m))) AS ?next) }}""" ] ."#
    )
}

/// The per-focus divergence the join-step limit used to stop: a shape rule minting one
/// new focus node per iteration. It is executed only for the focus node each iteration
/// adds, so its work per iteration stays flat, and at the default limits — and with the
/// join-step limit raised 256-fold out of the way — it is refused by the TERM-GENERATING
/// ROUND limit after 16,384 rounds, never by the join-step limit. Its neighbour bounded
/// at 1,000 counters completes under the defaults.
#[test]
fn a_divergent_shape_rule_is_refused_by_the_round_limit_not_the_join_step_limit() {
    let data = holder("ex:c1 a ex:C ; ex:n 1 .");
    let divergent = per_focus_counter("");
    for options in [
        RuleOptions::default(),
        RuleOptions::default().with_max_join_steps(1_048_576 * 256),
    ] {
        let refused = shacl(&data, &divergent, &options).expect_err("never stops");
        assert!(
            refused.starts_with(
                "SHACL rules did not complete: the rules exceeded the term-generating round \
                 limit: 16385 rounds inferred a term the evaluation graph did not hold, past \
                 the limit of 16384 (the default)"
            ),
            "{refused}"
        );
        assert!(!refused.contains("join-step"), "{refused}");
    }
    let bounded = per_focus_counter("FILTER (?n < 1000)");
    assert_eq!(
        shacl(&data, &bounded, &RuleOptions::default()),
        Ok(3 * 999),
        "the bounded neighbour completes: 999 new counters, each typed, numbered and linked"
    );
}

/// Growing IRI terms are refused by a caller-selected term-generating round
/// ceiling, independently of finite or unstated join capacity. The bounded
/// neighbour completes without an implicit arena ceiling.
#[test]
fn a_shape_rule_minting_ever_longer_iris_is_refused_by_a_term_ceiling_not_the_join_step_limit() {
    let fixture =
        include_str!("../../../vectors/shacl/af/rules/err-diverging-fresh-term/input.ttl");
    let load = |ttl: &str| -> (purrdf_shapes::shapes::Shapes, ShaclData) {
        let shapes = parse_shapes(ttl, None).expect("shapes parse");
        let dataset = parse_turtle_to_dataset(ttl, None).expect("data parses");
        let projected = engine::project_dataset(dataset.as_ref()).expect("projects");
        (
            shapes,
            ShaclData::new(Arc::clone(&projected), projected, None),
        )
    };
    let (shapes, data) = load(fixture);
    let run = |options: &RuleOptions| {
        infer(&data, &shapes, options)
            .map(|inference| inference.inferred().len())
            .map_err(|error| error.to_string())
    };
    for options in [
        RuleOptions::default().with_max_term_generating_rounds(8),
        RuleOptions::default()
            .with_max_term_generating_rounds(8)
            .with_max_join_steps(1_048_576 * 256),
    ] {
        let refused = run(&options).expect_err("unbounded IRI generation");
        assert!(refused.starts_with(
            "SHACL rules did not complete: the rules exceeded the term-generating round limit: 9 rounds"
        ), "{refused}");
        assert!(!refused.contains("join-step"), "{refused}");
    }
    let bounded = fixture.replace(
        "WHERE { $this a ex:Counter .",
        "WHERE { $this a ex:Counter . FILTER (STRLEN(STR($this)) < 1000)",
    );
    assert_ne!(bounded, fixture, "the bound is spliced in");
    let (shapes, data) = load(&bounded);
    let inferred = infer(&data, &shapes, &RuleOptions::default())
        .expect("the bounded neighbour completes")
        .inferred()
        .len();
    // `http://example.org/ns#c0` is 24 characters; a counter is minted for every length
    // from 24 up to 999 characters, each with its type and its link.
    assert_eq!(inferred, 2 * (1000 - 24));
}

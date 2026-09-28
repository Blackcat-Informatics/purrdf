// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The incremental re-execution of shape rules against the SHACL execution read
//! literally ([`Reexecution::Full`]: every focus node, every iteration): over the first-
//! party rules corpus and over randomised rule sets, both produce the same inference —
//! the same inferred triples, blank-node labels included, and the same proof — or the
//! same refusal.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use proptest::prelude::*;

use super::eval::Reexecution;
use crate::data::ShaclData;
use crate::rules::{RuleOptions, infer_with};
use crate::shapes::Shapes;

/// The observable result of one run: the inferred triples and the proof, or the
/// refusal.
fn observe(data: &ShaclData, shapes: &Shapes, options: &RuleOptions, how: Reexecution) -> String {
    match infer_with(data, shapes, options, how) {
        Ok(inference) => format!(
            "inferred:\n{}proof:\n{}",
            inference.inferred_ntriples(),
            inference.proof_text()
        ),
        Err(error) => format!("refused: {error}"),
    }
}

/// Both re-executions agree on `data` and `shapes` under `options`; returns the shared
/// observation.
fn agree(data: &ShaclData, shapes: &Shapes, options: &RuleOptions, case: &str) -> String {
    let incremental = observe(data, shapes, options, Reexecution::Incremental);
    let full = observe(data, shapes, options, Reexecution::Full);
    assert_eq!(
        incremental, full,
        "{case}: incremental and full re-execution differ"
    );
    incremental
}

/// A self-contained Turtle document as rule-evaluation data and shapes.
fn load_turtle(ttl: &str) -> (Shapes, ShaclData) {
    let shapes = crate::engine::parse_shapes(ttl, None).expect("shapes parse");
    let dataset = crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("data parses");
    let projected = crate::engine::project_dataset(dataset.as_ref()).expect("projects");
    (
        shapes,
        ShaclData::new(Arc::clone(&projected), projected, None),
    )
}

#[test]
fn the_rules_corpus_infers_the_same_under_both_reexecutions() {
    let root = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/shacl/af/rules"
    ));
    let mut cases: Vec<_> = fs::read_dir(root)
        .expect("the rules corpus")
        .map(|entry| entry.expect("a corpus entry").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();
    assert!(
        cases.len() >= 20,
        "the rules corpus shrank to {}",
        cases.len()
    );
    // A diverging case is compared under a round limit both re-executions reach
    // quickly; a terminating one under the defaults.
    let bounded = RuleOptions::default().with_max_term_generating_rounds(200);
    let mut completed = 0;
    for dir in cases {
        let name = dir
            .file_name()
            .expect("a case name")
            .to_string_lossy()
            .into_owned();
        let trig = dir.join("input.trig");
        let (input, media) = if trig.exists() {
            (trig, "application/trig")
        } else {
            (dir.join("input.ttl"), "text/turtle")
        };
        let text = fs::read_to_string(&input).expect("the case input");
        let base = format!("file://{}", input.display());
        let parsed = ::purrdf::parse_dataset_with(
            text.as_bytes(),
            media,
            Some(&base),
            &::purrdf::ParseOptions::default(),
        )
        .expect("the case input parses");
        let shapes =
            crate::shapes::from_dataset_with_prefixes(&parsed.dataset, &parsed.document_prefixes)
                .expect("the case shapes parse");
        let projected = crate::engine::project_dataset(parsed.dataset.as_ref()).expect("projects");
        let data = ShaclData::new(Arc::clone(&projected), projected, None);
        let options = if name.starts_with("err-") {
            bounded.clone()
        } else {
            RuleOptions::default()
        };
        let observed = agree(&data, &shapes, &options, &name);
        if observed.starts_with("inferred:") {
            completed += 1;
        } else {
            assert!(name.starts_with("err-"), "{name}: {observed}");
        }
    }
    assert!(completed >= 19, "only {completed} corpus cases completed");
}

/// The per-focus divergent rule of the corpus refuses under both re-executions with the
/// same message, naming the round limit.
#[test]
fn a_divergent_shape_rule_refuses_identically_under_both_reexecutions() {
    let (shapes, data) = load_turtle(include_str!(
        "../../../../vectors/shacl/af/rules/err-diverging-fresh-term/input.ttl"
    ));
    let options = RuleOptions::default().with_max_term_generating_rounds(300);
    let observed = agree(&data, &shapes, &options, "err-diverging-fresh-term");
    assert!(
        observed.contains("exceeded the term-generating round limit: 301 rounds"),
        "{observed}"
    );
}

/// The links of [`focus_nodes_discovered_across_iterations_infer_the_same_under_both_reexecutions`]'s
/// chain: enough, with its long node IRIs, that the evaluation graph is compacted
/// several times on the way as well as extended.
const WALK: usize = 60;

/// A chain the rules walk one link per iteration, each link through a focus node the
/// previous iteration made an instance of a target class, a target subject and a target
/// object — so every later execution runs for a focus node discovered from the triples
/// the iteration before gained, and for one whose read triples it gained.
#[test]
fn focus_nodes_discovered_across_iterations_infer_the_same_under_both_reexecutions() {
    let mut ttl = format!(
        "@prefix ex: <http://example.org/ns#> .\n\
         @prefix n: <http://example.org/{}/> .\n\
         @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         n:n0 a ex:B .\n",
        "node".repeat(128)
    );
    for link in 0..WALK {
        writeln!(ttl, "n:n{link} ex:p n:n{} .", link + 1).expect("write to String");
    }
    ttl.push_str(
        r#"ex:Walk a sh:NodeShape ; sh:targetClass ex:B ;
  sh:rule [ a sh:SPARQLRule ; sh:order 0 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:q ?o } WHERE { $this ex:p ?o }" ] .
ex:Step a sh:NodeShape ; sh:targetSubjectsOf ex:q ;
  sh:rule [ a sh:SPARQLRule ; sh:order 1 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { ?o a ex:B } WHERE { $this ex:q ?o }" ] .
ex:Mark a sh:NodeShape ; sh:targetObjectsOf ex:q ;
  sh:rule [ a sh:TripleRule ; sh:order 2 ; sh:subject sh:this ; sh:predicate ex:seen ;
            sh:object [ sh:path [ sh:inversePath ex:q ] ] ] .
ex:Tally a sh:NodeShape ; sh:targetNode n:n0 ;
  sh:rule [ a sh:SPARQLRule ; sh:order 3 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:reached ?n } WHERE { $this ex:q+ ?n }" ] .
"#,
    );
    let (shapes, data) = load_turtle(&ttl);
    let observed = agree(&data, &shapes, &RuleOptions::default(), "walk");
    let count = |predicate: &str| {
        observed
            .lines()
            .filter(|line| !line.starts_with("derived") && line.contains(predicate))
            .count()
    };
    // Every link walked, every node reached, every walked node marked.
    assert_eq!(count("#q>"), WALK, "{observed}");
    assert_eq!(count("#reached>"), WALK, "{observed}");
    assert_eq!(count("#seen>"), WALK, "{observed}");

    // With the shapes graph exposed to the SPARQL rules, their dataset is a second graph
    // extended and compacted beside the first; the rules do not read it, so the inference
    // is the same.
    let exposed = ShaclData::new(
        data.core_arc(),
        data.core_arc(),
        Some("http://example.org/shapes".to_owned()),
    );
    assert_eq!(
        agree(
            &exposed,
            &shapes,
            &RuleOptions::default(),
            "walk, shapes graph exposed"
        ),
        observed
    );
}

/// A focus node whose condition fails while a triple its rule reads is added, and holds
/// again later, is executed again when it does: under the full re-execution it would be,
/// over a graph that has gained what it reads.
#[test]
fn a_focus_node_eligible_again_is_executed_again() {
    let ttl = r#"@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:a ex:step 0 .
ex:Clock a sh:NodeShape ; sh:targetNode ex:a ;
  sh:rule [ a sh:SPARQLRule ; sh:order 0 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:step ?m } WHERE { $this ex:step ?n FILTER (?n < 3) BIND (?n + 1 AS ?m) }" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:order 0 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:q ex:q1 } WHERE { $this ex:step 1 }" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:order 0 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:r ex:v } WHERE { $this ex:step 2 }" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:order 0 ; sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:q ex:q2 } WHERE { $this ex:step 3 }" ] .
ex:Gated a sh:NodeShape ; sh:targetNode ex:a ;
  sh:rule [ a sh:SPARQLRule ; sh:order 1 ;
    sh:condition [ sh:or ( [ sh:property [ sh:path ex:q ; sh:maxCount 0 ] ]
                           [ sh:property [ sh:path ex:q ; sh:minCount 2 ] ] ) ] ;
    sh:construct
    "PREFIX ex: <http://example.org/ns#> CONSTRUCT { $this ex:out ?v } WHERE { $this ex:r ?v }" ] .
"#;
    let (shapes, data) = load_turtle(ttl);
    let observed = agree(&data, &shapes, &RuleOptions::default(), "gated");
    assert!(
        observed.contains(
            "<http://example.org/ns#a> <http://example.org/ns#out> <http://example.org/ns#v> ."
        ),
        "{observed}"
    );
}

/// The per-focus rule templates the randomised rule sets draw from. `$this` is the focus
/// node; every template is bounded by its data except the volatile ones, which mint a
/// fresh blank node per execution and are stopped by the round limit.
const SPARQL_RULES: &[&str] = &[
    // A focus-anchored copy.
    "CONSTRUCT { $this ex:q ?o } WHERE { $this ex:p ?o }",
    // A new type for the objects: new focus nodes for a class target.
    "CONSTRUCT { ?o a ex:B } WHERE { $this ex:q ?o }",
    // A join a step away from the focus node.
    "CONSTRUCT { $this ex:r ?z } WHERE { $this ex:p ?y . ?y ex:q ?z }",
    // Non-monotone: a negation over what other rules add.
    "CONSTRUCT { $this ex:lonely true } WHERE { FILTER NOT EXISTS { $this ex:q ?x } }",
    // A counter bounded by a constant.
    "CONSTRUCT { $this ex:v ?m } WHERE { $this ex:v ?n FILTER (?n < 3) BIND (?n + 1 AS ?m) }",
    // A minted IRI that becomes a focus node of a class target, bounded by its data.
    "CONSTRUCT { ?n a ex:A ; ex:p $this } WHERE { $this a ex:A ; ex:v ?v FILTER (?v < 2) \
     BIND (IRI(CONCAT(STR($this), 'x')) AS ?n) }",
    // Anchored at the object position.
    "CONSTRUCT { $this ex:q ?s } WHERE { ?s ex:p $this }",
    // Non-monotone: an OPTIONAL whose default another rule's triple replaces.
    "CONSTRUCT { $this ex:s ?c } WHERE { OPTIONAL { $this ex:q ?x } \
     BIND (COALESCE(?x, ex:none) AS ?c) }",
    // A variable predicate.
    "CONSTRUCT { $this ex:q ?o } WHERE { $this ?pp ?o FILTER (?pp = ex:p) }",
    // A zero-length path.
    "CONSTRUCT { $this ex:r ?o } WHERE { $this ex:q* ?o }",
    // An aggregate.
    "CONSTRUCT { $this ex:v ?c } WHERE { { SELECT $this (COUNT(?o) AS ?c) \
     WHERE { $this ex:q ?o } GROUP BY $this } }",
    // Volatile: a fresh blank node per execution.
    "CONSTRUCT { $this ex:t [ ex:of ?o ] } WHERE { $this ex:p ?o }",
];

/// The triple-rule heads the randomised rule sets draw from (`sh:subject sh:this`).
const TRIPLE_RULES: &[&str] = &[
    "sh:predicate ex:q ; sh:object [ sh:path ex:p ]",
    "sh:predicate ex:r ; sh:object [ sh:path [ sh:inversePath ex:q ] ]",
    "sh:predicate ex:r ; sh:object [ sh:path ( ex:p ex:q ) ]",
    "sh:predicate rdf:type ; sh:object ex:B",
];

/// The targets the randomised shapes draw from.
const TARGETS: &[&str] = &[
    "sh:targetClass ex:A",
    "sh:targetClass ex:B",
    "sh:targetSubjectsOf ex:p",
    "sh:targetObjectsOf ex:q",
    "sh:targetNode ex:n1",
    "sh:targetSubjectsOf ex:v",
    "sh:targetObjectsOf ex:v",
];

/// The conditions the randomised rules draw from, `None` for none.
const CONDITIONS: &[Option<&str>] = &[
    None,
    None,
    Some("[ sh:property [ sh:path ex:q ; sh:minCount 1 ] ]"),
    Some("[ sh:property [ sh:path ex:p ; sh:maxCount 1 ] ]"),
    // Holds, fails and holds again as `ex:q` values are added: none, one, two or more.
    Some(
        "[ sh:or ( [ sh:property [ sh:path ex:q ; sh:maxCount 0 ] ] \
         [ sh:property [ sh:path ex:q ; sh:minCount 2 ] ] ) ]",
    ),
];

/// One randomised rule: its template, its shape's target, its condition, whether it runs
/// once and its `sh:order`.
#[derive(Debug, Clone)]
struct RandomRule {
    template: usize,
    triple: bool,
    target: usize,
    condition: usize,
    run_once: bool,
    order: u8,
}

fn random_rule() -> impl Strategy<Value = RandomRule> {
    (
        0..SPARQL_RULES.len(),
        any::<bool>(),
        0..TARGETS.len(),
        0..CONDITIONS.len(),
        proptest::bool::weighted(0.2),
        0u8..3,
    )
        .prop_map(
            |(template, triple, target, condition, run_once, order)| RandomRule {
                template,
                triple,
                target,
                condition,
                run_once,
                order,
            },
        )
}

/// The document: data over five nodes, and one shape per rule.
fn document(
    edges: &[(u8, u8, u8)],
    types: &[(u8, bool)],
    subclass: bool,
    rules: &[RandomRule],
) -> String {
    let mut ttl = String::from(
        "@prefix ex: <http://example.org/ns#> .\n\
         @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
         @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    );
    for &(s, p, o) in edges {
        let predicate = ["ex:p", "ex:q", "ex:v"][usize::from(p % 3)];
        if predicate == "ex:v" {
            writeln!(ttl, "ex:n{} ex:v {} .", s % 5, o % 3).expect("write to String");
        } else {
            writeln!(ttl, "ex:n{} {predicate} ex:n{} .", s % 5, o % 5).expect("write to String");
        }
    }
    for &(node, b) in types {
        writeln!(ttl, "ex:n{} a ex:{} .", node % 5, if b { "B" } else { "A" })
            .expect("write to String");
    }
    if subclass {
        ttl.push_str("ex:A rdfs:subClassOf ex:B .\n");
    }
    for (index, rule) in rules.iter().enumerate() {
        let body = if rule.triple {
            format!(
                "a sh:TripleRule ; sh:subject sh:this ; {}",
                TRIPLE_RULES[rule.template % TRIPLE_RULES.len()]
            )
        } else {
            format!(
                "a sh:SPARQLRule ; sh:construct \"\"\"PREFIX ex: <http://example.org/ns#> {}\"\"\"",
                SPARQL_RULES[rule.template]
            )
        };
        let condition =
            CONDITIONS[rule.condition].map_or_default(|shape| format!(" ; sh:condition {shape}"));
        writeln!(
            ttl,
            "ex:S{index} a sh:NodeShape ; {} ;\n  sh:rule [ {body}{condition} ; \
             sh:order {} ; sh:runOnce {} ] .",
            TARGETS[rule.target], rule.order, rule.run_once
        )
        .expect("write to String");
    }
    ttl
}

fn property_config() -> ProptestConfig {
    ProptestConfig {
        cases: 256,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(property_config())]

    #[test]
    fn randomised_rule_sets_infer_the_same_under_both_reexecutions(
        edges in proptest::collection::vec((any::<u8>(), any::<u8>(), any::<u8>()), 0..24),
        types in proptest::collection::vec((any::<u8>(), any::<bool>()), 0..6),
        subclass in any::<bool>(),
        rules in proptest::collection::vec(random_rule(), 1..5),
    ) {
        let ttl = document(&edges, &types, subclass, &rules);
        let (shapes, data) = load_turtle(&ttl);
        // A volatile rule never stops minting: a round limit both re-executions reach
        // quickly stops it, and the join-step limit is raised out of the way so the
        // refusal, when there is one, is the round limit's in both.
        let options = RuleOptions::default()
            .with_max_term_generating_rounds(40)
            .with_max_join_steps(1 << 30);
        agree(&data, &shapes, &options, &ttl);
    }
}

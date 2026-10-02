// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one rules engine, end to end: SHACL 1.2 Inference Rules through the public
//! API, and rule sets built directly in the rule-set IR (`srl::ir`).
//!
//! Every refusal here is paired with the valid neighbour that must still succeed, and
//! every neighbour's control differs from its treatment in the observable the test
//! reads.

#[path = "support/terms.rs"]
mod terms;

use std::fmt::Write as _;
use std::sync::Arc;
use terms::ex_ns as iri;
use terms::integer as int;

use purrdf_rdf::RdfDataset;
use purrdf_shapes::data::ShaclData;
use purrdf_shapes::engine::{self, parse_shapes};
use purrdf_shapes::rules::{RuleOptions, RuleProcessor, infer};
use purrdf_shapes::shapes::Shapes;
use purrdf_shapes::srl::{
    self,
    ir::{
        DeclaredSchedule, Element, ElementRule, IrRule, IrRuleBody, PatternTerm, RuleSet,
        Scheduling, TriplePattern,
    },
};
use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use purrdf_sparql_algebra::{Args, ArithmeticOperator, Child, Expression, Function, Variable};

const PREFIXES: &str = r"
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix ex:   <http://example.org/ns#> .
    @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
";

const EX: &str = "http://example.org/ns#";
use purrdf_xsd::datatype::XSD_INTEGER;

fn shapes(body: &str) -> Result<Shapes, String> {
    parse_shapes(&format!("{PREFIXES}\n{body}"), None).map_err(String::from)
}

fn data(ttl: &str) -> ShaclData {
    let dataset: Arc<RdfDataset> =
        parse_turtle_to_dataset(&format!("{PREFIXES}\n{ttl}"), None).expect("data parses");
    let projected = engine::project_dataset(dataset.as_ref()).expect("projects");
    ShaclData::new(Arc::clone(&projected), projected, None)
}

/// Run the rules and return the inferred triples as `(s, p, o)` strings.
fn run(
    data_ttl: &str,
    shapes_body: &str,
    options: &RuleOptions,
) -> Result<Vec<(String, String, String)>, String> {
    let shapes = shapes(shapes_body)?;
    let inference = infer(&data(data_ttl), &shapes, options)?;
    Ok(inference
        .inferred()
        .iter()
        .map(|[s, p, o]| (s.to_string(), p.to_string(), o.to_string()))
        .collect())
}

fn has(inferred: &[(String, String, String)], s: &str, p: &str, o: &Term) -> bool {
    inferred.contains(&(iri(s).to_string(), iri(p).to_string(), o.to_string()))
}

// ── sh:ruleProcessor ────────────────────────────────────────────────────────────

/// SHACL 1.2 Inference Rules: "A rules engine that encounters rules or rule sets with
/// a value for sh:ruleProcessor that they are unable to handle MUST report a failure."
/// A processor the host registered is handled; the same graph without the
/// registration is refused — at rule and at rule-set level.
#[test]
fn a_known_rule_processor_loads_and_an_unknown_one_is_refused() {
    let at_rule = r"
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
          sh:rule [ a sh:TripleRule ; sh:ruleProcessor ex:Standard ;
                    sh:predicate ex:seen ; sh:object true ] .";
    let at_set = r"
        ex:Set a sh:RuleSet ; sh:ruleProcessor ex:Standard ; sh:hasRule ex:r .
        ex:r a sh:TripleRule ; sh:predicate ex:seen ; sh:object true .
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:r .";
    let known =
        RuleOptions::default().with_rule_processor(iri("Standard"), RuleProcessor::Standard);
    let truth = Term::Literal(Literal::new_typed_literal(
        "true",
        NamedNode::from("http://www.w3.org/2001/XMLSchema#boolean"),
    ));
    for body in [at_rule, at_set] {
        let refused = run("ex:alice a ex:Person .", body, &RuleOptions::default())
            .expect_err("an unregistered processor is refused");
        assert!(refused.contains("sh:ruleProcessor"), "{refused}");
        let inferred = run("ex:alice a ex:Person .", body, &known).expect("a known processor runs");
        assert!(has(&inferred, "alice", "seen", &truth), "{inferred:?}");
    }
    // A string-literal processor value is admitted too ("IRIs, or literals with
    // datatype xsd:string"), and is refused until registered.
    let string_valued = r#"
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
          sh:rule [ a sh:TripleRule ; sh:ruleProcessor "Standard" ;
                    sh:predicate ex:seen ; sh:object true ] ."#;
    assert!(
        run(
            "ex:alice a ex:Person .",
            string_valued,
            &RuleOptions::default()
        )
        .is_err()
    );
    let registered = RuleOptions::default().with_rule_processor(
        Term::Literal(Literal::new_simple_literal("Standard")),
        RuleProcessor::Standard,
    );
    assert!(run("ex:alice a ex:Person .", string_valued, &registered).is_ok());
    // A processor value that is neither is ill-formed at load.
    let err = shapes(
        r"ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
            sh:rule [ a sh:TripleRule ; sh:ruleProcessor 3 ; sh:predicate ex:p ; sh:object ex:o ] .",
    )
    .expect_err("an integer processor value is ill-formed");
    assert!(err.contains("sh:ruleProcessor"), "{err}");
}

// ── SPARQL rule templates ───────────────────────────────────────────────────────

const TEMPLATE: &str = r#"
    ex:Scale a sh:SPARQLRuleTemplate ; rdfs:subClassOf sh:Rule ;
      sh:parameter [ sh:path ex:source ] ;
      sh:parameter [ sh:path ex:target ] ;
      sh:parameter [ sh:path ex:factor ; sh:optional true ] ;
      sh:construct """CONSTRUCT { $this $target ?v } WHERE {
          $this $source ?b . BIND (?b * COALESCE($factor, 1) AS ?v) }""" .
    ex:M a sh:NodeShape ; sh:targetClass ex:Measure ; sh:rule ex:r ."#;

/// "Report a failure if any of the declared parameters that are not declared as
/// sh:optional true have no value in R." A fully bound instance runs; the same instance
/// missing a required parameter is refused; an optional parameter may be absent; two
/// values for one parameter have no single pre-binding map and are refused.
#[test]
fn a_bound_template_runs_and_a_missing_parameter_is_refused() {
    let bound =
        format!("{TEMPLATE} ex:r a ex:Scale ; ex:source ex:m ; ex:target ex:ft ; ex:factor 3 .");
    let inferred = run(
        "ex:x a ex:Measure ; ex:m 2 .",
        &bound,
        &RuleOptions::default(),
    )
    .expect("the bound instance runs");
    assert!(has(&inferred, "x", "ft", &int(6)), "{inferred:?}");

    let optional_absent =
        format!("{TEMPLATE} ex:r a ex:Scale ; ex:source ex:m ; ex:target ex:ft .");
    let inferred = run(
        "ex:x a ex:Measure ; ex:m 2 .",
        &optional_absent,
        &RuleOptions::default(),
    )
    .expect("an absent optional parameter is unbound");
    assert!(has(&inferred, "x", "ft", &int(2)), "{inferred:?}");

    let missing = format!("{TEMPLATE} ex:r a ex:Scale ; ex:source ex:m .");
    let err = shapes(&missing).expect_err("a missing required parameter is refused");
    assert!(err.contains("non-optional parameter"), "{err}");

    let doubled = format!("{TEMPLATE} ex:r a ex:Scale ; ex:source ex:m , ex:n ; ex:target ex:ft .");
    let err = shapes(&doubled).expect_err("two values for one parameter are refused");
    assert!(err.contains("2 values"), "{err}");
}

// ── Global rules, rule sets, layers and orders ──────────────────────────────────

/// A global rule runs once over the graph with no focus node; a global triple rule
/// with an absent position infers nothing ("which is empty for global rules"), while
/// the same rule with every position present infers its triple.
#[test]
fn global_rules_run_without_a_focus_node() {
    let inferred = run(
        "ex:a ex:knows ex:b .",
        r#"
        ex:sym a sh:SPARQLRule ;
          sh:construct "CONSTRUCT { ?o ?p ?s } WHERE { ?s ?p ?o }" .
        ex:empty a sh:TripleRule ; sh:predicate ex:never ; sh:object ex:never .
        ex:constant a sh:TripleRule ; sh:subject ex:k ; sh:predicate ex:is ; sh:object ex:v ."#,
        &RuleOptions::default(),
    )
    .expect("runs");
    assert!(has(&inferred, "b", "knows", &iri("a")), "{inferred:?}");
    assert!(has(&inferred, "k", "is", &iri("v")), "{inferred:?}");
    assert!(
        !inferred
            .iter()
            .any(|(_, p, _)| *p == iri("never").to_string())
    );
}

/// A rule set selects its members and its included sets' members; the default rule
/// set is every rule; an undeclared rule set is refused.
#[test]
fn a_rule_set_selects_its_rules() {
    let body = r"
        ex:One a sh:RuleSet ; sh:hasRule ex:r1 ; sh:includesRuleSet ex:Two .
        ex:Two a sh:RuleSet ; sh:hasRule ex:r2 .
        ex:r1 a sh:TripleRule ; sh:subject ex:a ; sh:predicate ex:p ; sh:object ex:one .
        ex:r2 a sh:TripleRule ; sh:subject ex:a ; sh:predicate ex:p ; sh:object ex:two .
        ex:r3 a sh:TripleRule ; sh:subject ex:a ; sh:predicate ex:p ; sh:object ex:three .";
    let all = run("ex:x ex:y ex:z .", body, &RuleOptions::default()).expect("runs");
    for value in ["one", "two", "three"] {
        assert!(has(&all, "a", "p", &iri(value)), "{value}: {all:?}");
    }
    let one = run(
        "ex:x ex:y ex:z .",
        body,
        &RuleOptions::default().with_rule_set(NamedNode::from(format!("{EX}One").as_str())),
    )
    .expect("runs");
    assert!(has(&one, "a", "p", &iri("one")) && has(&one, "a", "p", &iri("two")));
    assert!(!has(&one, "a", "p", &iri("three")), "{one:?}");
    let err = run(
        "ex:x ex:y ex:z .",
        body,
        &RuleOptions::default().with_rule_set(NamedNode::from(format!("{EX}Nope").as_str())),
    )
    .expect_err("an undeclared rule set is refused");
    assert!(err.contains("not declared"), "{err}");
}

/// "Rules with the same order are executed concurrently and must not see each other's
/// inferences before they have all completed" — while a later order DOES see an
/// earlier one's: the treatment (two orders) infers one triple, the control (one
/// order) both.
#[test]
fn same_order_rules_run_concurrently_and_orders_run_in_sequence() {
    let rules = |p_order: u8, q_order: u8| {
        format!(
            r#"
            ex:P a sh:SPARQLRule ; sh:order {p_order} ; sh:construct
              "CONSTRUCT {{ ?n ex:p true }} WHERE {{ ?n a ex:Node FILTER NOT EXISTS {{ ?n ex:q true }} }}" .
            ex:Q a sh:SPARQLRule ; sh:order {q_order} ; sh:construct
              "CONSTRUCT {{ ?n ex:q true }} WHERE {{ ?n a ex:Node FILTER NOT EXISTS {{ ?n ex:p true }} }}" ."#
        )
    };
    let together = run("ex:n a ex:Node .", &rules(0, 0), &RuleOptions::default()).expect("runs");
    assert_eq!(together.len(), 2, "{together:?}");
    let ordered = run("ex:n a ex:Node .", &rules(0, 1), &RuleOptions::default()).expect("runs");
    assert_eq!(ordered.len(), 1, "{ordered:?}");
    assert!(ordered[0].1.ends_with("#p>"), "{ordered:?}");
}

/// "All temporary triples, i.e., triples whose reifier is marked with sh:tempTriple
/// true, are automatically deleted by the SHACL engine at the end of the inference
/// process", reifiers included — visible to the later layer that reads them. The
/// control marks nothing and keeps everything.
#[test]
fn temporary_triples_are_visible_then_deleted() {
    let rules = |mark: &str| {
        format!(
            r#"
            ex:collect a sh:SPARQLRule ; sh:runOnce true ; sh:construct """
              PREFIX ex: <http://example.org/ns#>
              CONSTRUCT {{ ?a ex:tmp ?b . ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ?t . {mark} }}
              WHERE {{ ?a ex:link ?b BIND (BNODE() AS ?r) BIND (TRIPLE(?a, ex:tmp, ?b) AS ?t) }}""" .
            ex:use a sh:SPARQLRule ; sh:layer 1 ; sh:construct
              "PREFIX ex: <http://example.org/ns#> CONSTRUCT {{ ?a ex:final ?b }} WHERE {{ ?a ex:tmp ?b }}" ."#
        )
    };
    let temporary = run(
        "ex:a ex:link ex:b .",
        &rules("?r <http://www.w3.org/ns/shacl#tempTriple> true ."),
        &RuleOptions::default(),
    )
    .expect("runs");
    assert!(has(&temporary, "a", "final", &iri("b")), "{temporary:?}");
    assert_eq!(
        temporary.len(),
        1,
        "only the final triple survives: {temporary:?}"
    );
    let kept = run("ex:a ex:link ex:b .", &rules(""), &RuleOptions::default()).expect("runs");
    assert!(has(&kept, "a", "tmp", &iri("b")), "{kept:?}");
    assert!(kept.len() >= 3, "{kept:?}");
}

/// `sh:sourceRule` tracking, when asked for, reifies each inferred triple with its rule
/// — and it is not visible to rules, so it changes nothing else.
#[test]
fn source_rules_are_tracked_on_request() {
    let body = r"
        ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
          sh:rule ex:r .
        ex:r a sh:TripleRule ; sh:predicate ex:seen ; sh:object ex:yes .";
    let shapes = shapes(body).expect("loads");
    let plain = infer(
        &data("ex:alice a ex:Person ."),
        &shapes,
        &RuleOptions::default(),
    )
    .expect("runs");
    let tracked = infer(
        &data("ex:alice a ex:Person ."),
        &shapes,
        &RuleOptions::default().with_source_rules(true),
    )
    .expect("runs");
    assert_eq!(
        plain.inferred(),
        tracked.inferred(),
        "tracking infers nothing new"
    );
    assert_eq!(plain.dataset().reifiers().count(), 0);
    let reifiers: Vec<_> = tracked.dataset().reifiers().collect();
    assert_eq!(reifiers.len(), 1, "one inferred triple, one reifier");
    assert_eq!(tracked.dataset().annotations_of(reifiers[0].0).count(), 1);
}

/// A derived triple's explanation names the rule that derived it.
#[test]
fn a_derived_triple_explains_itself() {
    let shapes = shapes(
        r"ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:tag .
          ex:tag a sh:TripleRule ; sh:predicate ex:seen ; sh:object ex:yes .",
    )
    .expect("loads");
    let inference = infer(
        &data("ex:alice a ex:Person ."),
        &shapes,
        &RuleOptions::default(),
    )
    .expect("runs");
    let explanation = inference
        .explain_conclusion(&[iri("alice"), iri("seen"), iri("yes")])
        .expect("the inferred triple is explained");
    assert_eq!(explanation.rule(), &iri("tag"));
    assert!(
        inference
            .explain_conclusion(&[iri("alice"), iri("seen"), iri("no")])
            .is_none(),
        "a triple nothing inferred has no explanation"
    );
    assert_eq!(inference.contract_hash().len(), 64);
}

// ── Rules entailment and rule-node well-formedness ──────────────────────────────

/// `sh:entailment sh:RulesEntailment` runs the rules before validating: the treatment
/// conforms, the identical graph without the declaration does not, and any other
/// entailment regime is still refused at load.
#[test]
fn rules_entailment_runs_the_rules_before_validation() {
    let graph = |entailment: &str| {
        format!(
            r#"
            <http://example.org/shapes> {entailment} .
            ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:rule ex:name ;
              sh:property [ sh:path ex:fullName ; sh:minCount 1 ] .
            ex:name a sh:SPARQLRule ; sh:construct
              "CONSTRUCT {{ $this ex:fullName ?n }} WHERE {{ $this ex:first ?n }}" ."#
        )
    };
    let data_ttl = r#"ex:alice a ex:Person ; ex:first "Alice" ."#;
    let validate = |entailment: &str| -> Result<bool, String> {
        let shapes = shapes(&graph(entailment))?;
        let report = engine::validate_with(&data(data_ttl), &shapes)?;
        Ok(report.conforms)
    };
    assert!(validate("sh:entailment sh:RulesEntailment").expect("validates"));
    assert!(!validate("rdfs:label \"no regime\"").expect("validates"));
    let err = validate("sh:entailment <http://www.w3.org/ns/entailment/RDFS>")
        .expect_err("an unsupported regime is refused");
    assert!(err.contains("sh:RulesEntailment only"), "{err}");
}

/// Rule nodes are checked against the census: an unknown SHACL term, and a known one
/// that belongs on a shape, are refused on a rule; a non-validating term is not.
#[test]
fn rule_nodes_are_checked_against_the_census() {
    let rule = |extra: &str| {
        shapes(&format!(
            "ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
               sh:rule [ a sh:TripleRule ; {extra} sh:predicate ex:p ; sh:object ex:o ] ."
        ))
    };
    let unknown = rule("sh:predicat ex:q ;").expect_err("a misspelled term is refused");
    assert!(unknown.contains("not a term of SHACL 1.2"), "{unknown}");
    let misplaced = rule("sh:minCount 1 ;").expect_err("a shape parameter is refused on a rule");
    assert!(misplaced.contains("minCount"), "{misplaced}");
    rule("sh:targetNode ex:x ;").expect_err("a rule node carrying a target is refused");
    rule(r#"sh:name "documented" ;"#).expect("a non-validating term loads");
}

/// A condition that is a SHACL instance of both `sh:NodeShape` and `rdfs:Class` brings
/// its non-deactivated such superclasses: the focus must conform to the superclass's
/// constraint too. The control, whose superclass is deactivated, infers for both.
#[test]
fn a_class_condition_brings_its_superclasses() {
    let body = |deactivated: bool| {
        format!(
            "ex:Base a sh:ShapeClass ; sh:deactivated {deactivated} ;
               sh:property [ sh:path ex:id ; sh:minCount 1 ] .
             ex:Sub a sh:ShapeClass ; rdfs:subClassOf ex:Base .
             ex:R a sh:NodeShape ; sh:targetClass ex:Thing ;
               sh:rule [ a sh:TripleRule ; sh:condition ex:Sub ;
                         sh:predicate ex:ok ; sh:object true ] ."
        )
    };
    let data_ttl = "ex:a a ex:Thing ; ex:id 1 . ex:b a ex:Thing .";
    let strict = run(data_ttl, &body(false), &RuleOptions::default()).expect("runs");
    assert_eq!(
        strict.len(),
        1,
        "only ex:a conforms to the superclass: {strict:?}"
    );
    let lax = run(data_ttl, &body(true), &RuleOptions::default()).expect("runs");
    assert_eq!(lax.len(), 2, "{lax:?}");
}

// ── Termination ─────────────────────────────────────────────────────────────────

/// A counter stepping `ex:n` from 0 to `bound`: one term-generating round per step.
fn counter(bound: u32) -> String {
    format!(
        r#"ex:count a sh:SPARQLRule ; sh:construct
             "CONSTRUCT {{ ?s ex:n ?m }} WHERE {{ ?s ex:n ?n FILTER (?n < {bound}) BIND (?n + 1 AS ?m) }}" ."#
    )
}

/// A countdown stepping `ex:n` down to 0 from the value in the data. `optional` adds an
/// `OPTIONAL` that matches nothing, which keeps the rule a PRODUCER — run over the whole
/// evaluation graph each iteration — rather than its semi-naive element reading.
fn countdown(optional: bool) -> String {
    let optional = if optional {
        "OPTIONAL { ?s ex:never ?z } "
    } else {
        ""
    };
    format!(
        r#"ex:count a sh:SPARQLRule ; sh:construct
             "CONSTRUCT {{ ?s ex:n ?m }} WHERE {{ ?s ex:n ?n {optional}FILTER (?n > 0) BIND (?n - 1 AS ?m) }}" ."#
    )
}

/// A countdown from 300, and one from 10,000, complete under the DEFAULT limits with
/// every step — deep, terminating rule sets are not refused. The 300 countdown reaches
/// the same inference graph as its producer twin, which the engine runs by executing
/// the CONSTRUCT over the whole graph each iteration.
#[test]
fn deep_countdowns_complete_under_the_default_limits() {
    let short = run(
        "ex:a ex:n 300 .",
        &countdown(false),
        &RuleOptions::default(),
    )
    .expect("300 steps terminate under the default limits");
    assert_eq!(short.len(), 300, "ex:n 299 down to 0");
    assert!(has(&short, "a", "n", &int(0)));
    assert!(
        !has(&short, "a", "n", &int(-1)),
        "and no step past the FILTER"
    );
    let producer = run("ex:a ex:n 300 .", &countdown(true), &RuleOptions::default())
        .expect("the producer twin terminates too");
    assert_eq!(short, producer, "one inference graph, either way");

    let long = run(
        "ex:a ex:n 10000 .",
        &countdown(false),
        &RuleOptions::default(),
    )
    .expect("10,000 steps terminate under the default limits");
    assert_eq!(long.len(), 10_000, "ex:n 9999 down to 0");
    assert!(has(&long, "a", "n", &int(0)));
    assert!(has(&long, "a", "n", &int(5000)));
}

/// Rules read as elements infer exactly what their producer twins do — a join, a
/// recursive closure, a filter, a computed literal feeding a later iteration, a
/// language-tagged and a typed constant — the twin kept a producer by an `OPTIONAL`
/// that matches nothing.
#[test]
fn element_readings_infer_what_their_producers_do() {
    let rules = |optional: &str| {
        format!(
            r#"ex:grand a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?x ex:ancestor ?z }} WHERE {{ ?x ex:parent ?y . ?y ex:parent ?z {optional}}}" .
               ex:closure a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?x ex:ancestor ?z }} WHERE {{ ?x ex:ancestor ?y . ?y ex:ancestor ?z {optional}}}" .
               ex:adult a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?x a ex:Adult ; ex:label 'adult'@en }} WHERE {{ ?x ex:age ?a {optional}FILTER (?a >= 18) }}" .
               ex:next a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?x ex:age ?b }} WHERE {{ ?x ex:age ?a {optional}FILTER (?a < 20) BIND (?a + 1 AS ?b) }}" ."#
        )
    };
    let data_ttl = "ex:a ex:parent ex:b . ex:b ex:parent ex:c . ex:c ex:parent ex:d .
        ex:a ex:age 16 . ex:b ex:age \"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .";
    let elements = run(data_ttl, &rules(""), &RuleOptions::default()).expect("runs");
    let producers = run(
        data_ttl,
        &rules("OPTIONAL { ?x ex:never ?z } "),
        &RuleOptions::default(),
    )
    .expect("runs");
    assert_eq!(elements, producers);
    assert!(has(&elements, "a", "ancestor", &iri("c")));
    assert!(
        elements
            .iter()
            .any(|(s, _, o)| s.ends_with("#a>") && o.ends_with("#Adult>")),
        "the counter's later steps reach the adult rule: {elements:?}"
    );
    assert!(has(&elements, "a", "age", &int(20)));
    assert!(!has(&elements, "a", "age", &int(21)));
}

/// A counter with no bound infers one new term per iteration forever: it is refused at
/// the DEFAULT round limit, naming the limit, the numbers, the rule and the knob — and
/// never called divergent, which no limit proves. Its bounded neighbour terminates.
#[test]
fn an_unbounded_counter_is_refused_at_the_round_limit() {
    let unbounded = r#"ex:count a sh:SPARQLRule ; sh:construct
             "CONSTRUCT { ?s ex:n ?m } WHERE { ?s ex:n ?n BIND (?n + 1 AS ?m) }" ."#;
    let err = run("ex:a ex:n 0 .", unbounded, &RuleOptions::default()).expect_err("never stops");
    assert!(
        err.starts_with(
            "SHACL rules did not complete: the rules exceeded the term-generating round \
             limit: 16385 rounds inferred a term the evaluation graph did not hold, past the \
             limit of 16384 (the default); rule <http://example.org/ns#count> inferred a new \
             term in the last iteration"
        ),
        "{err}"
    );
    assert!(
        err.ends_with("raise the limit with RuleOptions::with_max_term_generating_rounds"),
        "{err}"
    );
    assert!(!err.contains("diverge"), "{err}");
    let bounded = run("ex:a ex:n 0 .", &counter(1000), &RuleOptions::default())
        .expect("the bounded neighbour terminates");
    assert_eq!(bounded.len(), 1000);
}

/// Two rules each extending every string by one letter double the strings every
/// iteration: the run is refused at the DEFAULT generated-term budget within a few
/// iterations, naming the budget, the numbers and both rules. The same rules bounded to
/// strings of eight letters (510 strings) complete under the default.
#[test]
fn an_exponential_rule_set_is_refused_at_the_generated_term_budget() {
    let rules = |filter: &str| {
        format!(
            r#"ex:a a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?s ex:name ?m }} WHERE {{ ?s ex:name ?n {filter} BIND (CONCAT(?n, 'a') AS ?m) }}" .
               ex:b a sh:SPARQLRule ; sh:construct
                 "CONSTRUCT {{ ?s ex:name ?m }} WHERE {{ ?s ex:name ?n {filter} BIND (CONCAT(?n, 'b') AS ?m) }}" ."#
        )
    };
    let err = run(r#"ex:s ex:name "" ."#, &rules(""), &RuleOptions::default())
        .expect_err("the strings double every iteration");
    assert!(
        err.starts_with(
            "SHACL rules did not complete: the rules exceeded the generated-term budget: "
        ),
        "{err}"
    );
    assert!(
        err.contains("past the budget of 65536 (the default, max(65536, 4 per input term))"),
        "{err}"
    );
    assert!(
        err.contains(
            "rule <http://example.org/ns#a>, rule <http://example.org/ns#b> inferred a new term"
        ),
        "{err}"
    );
    assert!(
        err.ends_with("raise the budget with RuleOptions::with_max_generated_terms"),
        "{err}"
    );
    assert!(!err.contains("diverge"), "{err}");
    let bounded = run(
        r#"ex:s ex:name "" ."#,
        &rules("FILTER (STRLEN(?n) < 8)"),
        &RuleOptions::default(),
    )
    .expect("bounded strings terminate");
    assert_eq!(bounded.len(), 510, "every string of one to eight letters");
    let tight = RuleOptions::default().with_max_generated_terms(100);
    let err = run(
        r#"ex:s ex:name "" ."#,
        &rules("FILTER (STRLEN(?n) < 8)"),
        &tight,
    )
    .expect_err("the caller's smaller budget refuses it");
    assert!(
        err.contains("past the budget of 100 (the caller's budget)"),
        "{err}"
    );
}

/// A caller's round limit is exact: under 500 a 1000-step counter is refused naming 500
/// as the caller's, and a counter needing 400 completes; the refusal names the host's
/// own knob when the host names it.
#[test]
fn the_term_generating_round_limit_is_the_callers() {
    let data_ttl = "ex:a ex:n 0 .";
    let limited = RuleOptions::default().with_max_term_generating_rounds(500);
    let err = run(data_ttl, &counter(1000), &limited).expect_err("past the limit");
    assert!(
        err.contains("501 rounds inferred a term the evaluation graph did not hold, past the limit of 500 (the caller's limit)"),
        "{err}"
    );
    assert!(
        err.contains("raise the limit with RuleOptions::with_max_term_generating_rounds"),
        "{err}"
    );
    let short = run(data_ttl, &counter(400), &limited).expect("within the limit");
    assert_eq!(short.len(), 400, "ex:n 1 through 400");
    let named = limited.with_limit_knobs(purrdf_shapes::LimitKnobs::new(
        "--rounds", "--terms", "--facts", "--steps",
    ));
    let err = run(data_ttl, &counter(1000), &named).expect_err("past the limit");
    assert!(err.ends_with("raise the limit with --rounds"), "{err}");
}

/// A counter bounded by its DATA: the depth of every node of a 400-edge chain is one
/// more than its predecessor's — 400 term-generating rounds — and it completes under the
/// default limits with every depth.
#[test]
fn a_data_bounded_counter_completes_under_the_default() {
    const LENGTH: usize = 400;
    let mut data_ttl = String::from("ex:n0 ex:depth 0 .\n");
    for index in 0..LENGTH {
        writeln!(data_ttl, "ex:n{index} ex:next ex:n{} .", index + 1).expect("write");
    }
    let depth = r#"ex:depth a sh:SPARQLRule ; sh:construct
        "CONSTRUCT { ?y ex:depth ?e } WHERE { ?x ex:depth ?d . ?x ex:next ?y BIND (?d + 1 AS ?e) }" ."#;
    let inferred = run(&data_ttl, depth, &RuleOptions::default())
        .expect("a data-bounded counter completes under the default limits");
    assert_eq!(inferred.len(), LENGTH, "one depth per node past n0");
    assert!(has(
        &inferred,
        &format!("n{LENGTH}"),
        "depth",
        &int(i64::try_from(LENGTH).expect("fits"))
    ));
}

/// An iterating shape rule that mints a fresh blank node on every pass never terminates,
/// and every minted blank is itself an `ex:Counter`, so the counters double every
/// iteration: a caller's round limit of 8 refuses it at a few hundred counters, by name.
/// Its `sh:runOnce` neighbour fires once and terminates.
#[test]
fn an_iterating_blank_minting_rule_reports_its_limit() {
    let shapes_body = |once: &str| {
        format!(
            r#"ex:S a sh:NodeShape ; sh:targetClass ex:Counter ;
                 sh:rule [ a sh:SPARQLRule ; {once} sh:construct
                   "CONSTRUCT {{ $this ex:next _:n . _:n a ex:Counter }} WHERE {{ $this a ex:Counter }}" ] ."#
        )
    };
    let limited = RuleOptions::default().with_max_term_generating_rounds(8);
    let err =
        run("ex:c0 a ex:Counter .", &shapes_body(""), &limited).expect_err("mints every pass");
    assert!(
        err.contains("past the limit of 8 (the caller's limit)"),
        "{err}"
    );
    let once = run(
        "ex:c0 a ex:Counter .",
        &shapes_body("sh:runOnce true ;"),
        &limited,
    )
    .expect("fires once");
    assert_eq!(
        once.len(),
        2,
        "one ex:next edge and one typed blank: {once:?}"
    );
}

// ── The rule-set IR ─────────────────────────────────────────────────────────────

fn var(name: &str) -> PatternTerm {
    PatternTerm::Variable(name.to_owned())
}

fn constant(term: Term) -> PatternTerm {
    PatternTerm::Term(term)
}

fn pattern(s: PatternTerm, p: &str, o: PatternTerm) -> TriplePattern {
    TriplePattern::new(s, constant(iri(p)), o)
}

fn element_rule(id: &str, head: Vec<TriplePattern>, body: Vec<Element>) -> IrRule<'static> {
    IrRule {
        id: iri(id),
        body: IrRuleBody::Elements(ElementRule {
            head,
            body,
            data: false,
        }),
        schedule: DeclaredSchedule::default(),
        expected_predicates: Vec::new(),
    }
}

fn evaluate(rules: Vec<IrRule<'static>>, data_ttl: &str) -> Result<Vec<[Term; 3]>, String> {
    let set = RuleSet {
        rules,
        data: Vec::new(),
        scheduling: Scheduling::Stratified,
    };
    srl::evaluate(
        &set,
        &data(data_ttl),
        &Shapes::default(),
        &RuleOptions::default(),
    )
    .map(|inference| inference.inferred().to_vec())
    .map_err(String::from)
}

fn v(name: &str) -> Expression {
    Expression::Variable(Variable::new(name))
}

/// A stratifiable rule set with negation evaluates, the negated rule running after the
/// rule it reads; a set whose negation lies in a cycle is refused, and the refusal
/// names the rules of the cycle.
#[test]
fn a_stratifiable_negation_evaluates_and_a_cycle_through_one_is_refused_by_name() {
    // safe(x) :- component(x), NOT exposed(x, ?v) ;  exposed(x, v) :- vulnerable(x, v)
    let safe = element_rule(
        "safe",
        vec![pattern(var("x"), "status", constant(iri("safe")))],
        vec![
            Element::Pattern(pattern(var("x"), "type", constant(iri("Component")))),
            Element::Negation {
                elements: vec![Element::Pattern(pattern(var("x"), "exposed", var("v")))],
                data: false,
            },
        ],
    );
    let exposed = element_rule(
        "exposed",
        vec![pattern(var("x"), "exposed", var("v"))],
        vec![Element::Pattern(pattern(var("x"), "vulnerable", var("v")))],
    );
    let inferred = evaluate(
        vec![safe.clone(), exposed.clone()],
        "ex:a ex:type ex:Component ; ex:vulnerable ex:v1 . ex:b ex:type ex:Component .",
    )
    .expect("stratifiable");
    assert!(
        inferred.contains(&[iri("b"), iri("status"), iri("safe")]),
        "{inferred:?}"
    );
    assert!(
        !inferred.contains(&[iri("a"), iri("status"), iri("safe")]),
        "{inferred:?}"
    );

    // exposed(x, self) :- status(x, safe) closes the cycle through the negation.
    let back = element_rule(
        "back",
        vec![pattern(var("x"), "exposed", constant(iri("self")))],
        vec![Element::Pattern(pattern(
            var("x"),
            "status",
            constant(iri("safe")),
        ))],
    );
    let err = evaluate(vec![safe, exposed, back], "ex:a ex:type ex:Component .")
        .expect_err("not stratifiable");
    assert!(err.contains("not stratifiable"), "{err}");
    assert!(
        err.contains(&format!("<{EX}safe>")) && err.contains(&format!("<{EX}back>")),
        "{err}"
    );
}

/// Filters, assignments, fresh head blank nodes and triple terms lower onto guards: a
/// filter drops, an assignment binds (an error rejecting the solution), each solution
/// mints its own blank node shared by the head's templates, and a triple term is taken
/// apart in the body and built in the head.
#[test]
fn element_rules_lower_filters_assignments_blanks_and_triple_terms() {
    let xsd_integer = purrdf_sparql_algebra::NamedNode::new(XSD_INTEGER).expect("absolute");
    let ten = Expression::Literal(purrdf_sparql_algebra::Literal::new_typed("10", xsd_integer));
    let scaled = element_rule(
        "scaled",
        vec![
            TriplePattern::new(
                PatternTerm::BlankNode("n".to_owned()),
                constant(iri("of")),
                var("x"),
            ),
            TriplePattern::new(
                PatternTerm::BlankNode("n".to_owned()),
                constant(iri("value")),
                var("y"),
            ),
        ],
        vec![
            Element::Pattern(pattern(var("x"), "size", var("s"))),
            Element::Filter(Expression::Greater(
                Child::new(v("s")),
                Child::new(ten.clone()),
            )),
            Element::Assign {
                variable: "y".to_owned(),
                expression: Expression::arithmetic(v("s"), ArithmeticOperator::Multiply, ten),
            },
        ],
    );
    let inferred = evaluate(
        vec![scaled],
        "ex:a ex:size 20 . ex:b ex:size 5 . ex:c ex:size \"not a number\" .",
    )
    .expect("evaluates");
    let blank = inferred
        .iter()
        .find(|[_, p, o]| *p == iri("of") && *o == iri("a"))
        .map(|[s, _, _]| s.clone())
        .expect("one note for ex:a");
    assert!(matches!(blank, Term::BlankNode(_)));
    assert!(
        inferred.contains(&[blank, iri("value"), int(200)]),
        "{inferred:?}"
    );
    assert_eq!(
        inferred.len(),
        2,
        "ex:b is filtered, ex:c's filter errs: {inferred:?}"
    );

    // A reifier's triple term, taken apart in the body and rebuilt in the head.
    let reverse = element_rule(
        "reverse",
        vec![TriplePattern::new(
            var("r"),
            constant(iri("reversed")),
            PatternTerm::Triple(Box::new(pattern(var("o"), "p", var("s")))),
        )],
        vec![Element::Pattern(TriplePattern::new(
            var("r"),
            constant(iri("about")),
            PatternTerm::Triple(Box::new(pattern(var("s"), "p", var("o")))),
        ))],
    );
    let inferred =
        evaluate(vec![reverse], "ex:r ex:about <<( ex:x ex:p ex:y )>> .").expect("evaluates");
    assert_eq!(inferred.len(), 1, "{inferred:?}");
    assert_eq!(inferred[0][1], iri("reversed"));
    assert_eq!(
        inferred[0][2].to_string(),
        format!("<<( <{EX}y> <{EX}p> <{EX}x> )>>")
    );
}

/// `NOT DATA` matches the base graph only, which no rule can change: a rule defaulting a
/// value its own head also writes stratifies under `NOT DATA` — the base-graph pattern
/// depends on no rule — and evaluates, a value the rules infer never blocking it; the
/// same rule with a plain `NOT` depends on itself through the negation and is refused.
#[test]
fn not_data_matches_the_base_graph_only() {
    let default_value = |data: bool| {
        element_rule(
            "default",
            vec![pattern(var("x"), "km", constant(int(0)))],
            vec![
                Element::Pattern(pattern(var("x"), "type", constant(iri("Leg")))),
                Element::Negation {
                    elements: vec![Element::Pattern(pattern(var("x"), "km", var("k")))],
                    data,
                },
            ],
        )
    };
    let infer_km = element_rule(
        "infer",
        vec![pattern(var("x"), "km", constant(int(1)))],
        vec![Element::Pattern(pattern(var("x"), "miles", var("m")))],
    );
    let data_ttl = "ex:a ex:type ex:Leg ; ex:miles 1 . ex:b ex:type ex:Leg ; ex:km 5 .";
    let base_only = evaluate(vec![default_value(true), infer_km.clone()], data_ttl).expect("runs");
    assert!(
        base_only.contains(&[iri("a"), iri("km"), int(0)]),
        "{base_only:?}"
    );
    assert!(
        !base_only.contains(&[iri("b"), iri("km"), int(0)]),
        "{base_only:?}"
    );
    let err = evaluate(vec![default_value(false), infer_km], data_ttl)
        .expect_err("a plain NOT over its own head is a closed self-dependency");
    assert!(err.contains("not stratifiable"), "{err}");
}

/// "NOW() is permitted and is defined to return the same point in time throughout a
/// rule set evaluation": two rules reading NOW() in different strata agree.
#[test]
fn now_is_one_point_in_time_per_evaluation() {
    let stamp = |id: &str, p: &str| {
        element_rule(
            id,
            vec![pattern(var("x"), p, var("t"))],
            vec![
                Element::Pattern(pattern(var("x"), "type", constant(iri("Event")))),
                Element::Assign {
                    variable: "t".to_owned(),
                    expression: Expression::FunctionCall(Function::Now, Args::default()),
                },
            ],
        )
    };
    let inferred = evaluate(
        vec![stamp("one", "at1"), stamp("two", "at2")],
        "ex:e ex:type ex:Event .",
    )
    .expect("runs");
    let at = |p: &str| {
        inferred
            .iter()
            .find(|[_, q, _]| *q == iri(p))
            .map(|[_, _, t]| t.clone())
            .expect("stamped")
    };
    assert_eq!(at("at1"), at("at2"));
}

/// An ill-formed element rule is refused before evaluation, naming the rule; the
/// well-formed neighbour evaluates.
#[test]
fn an_ill_formed_element_rule_is_refused() {
    let unbound_head = element_rule(
        "bad",
        vec![pattern(var("x"), "p", var("nowhere"))],
        vec![Element::Pattern(pattern(var("x"), "q", var("y")))],
    );
    let err = evaluate(vec![unbound_head], "ex:a ex:q ex:b .").expect_err("ill formed");
    assert!(
        err.contains(&format!("<{EX}bad>")) && err.contains("not well formed"),
        "{err}"
    );
    let fine = element_rule(
        "fine",
        vec![pattern(var("x"), "p", var("y"))],
        vec![Element::Pattern(pattern(var("x"), "q", var("y")))],
    );
    assert_eq!(
        evaluate(vec![fine], "ex:a ex:q ex:b .")
            .expect("evaluates")
            .len(),
        1
    );
}

/// The shape of the transitive-closure bench, pinned for correctness: a chain of `n`
/// nodes closes to exactly `n(n − 1)/2` reaches.
#[test]
fn a_transitive_closure_over_a_chain_is_complete() {
    const N: usize = 8;
    let mut data_ttl = String::new();
    for i in 0..N - 1 {
        writeln!(data_ttl, "ex:n{i} ex:link ex:n{} .", i + 1).expect("writes to a String");
    }
    let inferred = run(
        &data_ttl,
        r#"
        ex:base a sh:SPARQLRule ; sh:construct
          "PREFIX ex: <http://example.org/ns#> CONSTRUCT { ?a ex:reaches ?b } WHERE { ?a ex:link ?b }" .
        ex:step a sh:SPARQLRule ; sh:order 1 ; sh:construct
          "PREFIX ex: <http://example.org/ns#> CONSTRUCT { ?a ex:reaches ?c } WHERE { ?a ex:link ?b . ?b ex:reaches ?c }" ."#,
        &RuleOptions::default(),
    )
    .expect("closes");
    assert_eq!(inferred.len(), N * (N - 1) / 2);
}

/// "A data block is a set of triples. These triples are added to the inference graph as
/// additional facts and are included in the inference process": a data-block triple is
/// inferred output and feeds a rule; one the base graph already holds is not inferred.
#[test]
fn data_blocks_join_the_inference_graph() {
    let tag = element_rule(
        "tag",
        vec![pattern(var("x"), "tagged", constant(iri("yes")))],
        vec![Element::Pattern(pattern(
            var("x"),
            "type",
            constant(iri("Thing")),
        ))],
    );
    let set = RuleSet {
        rules: vec![tag],
        data: vec![
            [iri("d"), iri("type"), iri("Thing")],
            [iri("b"), iri("type"), iri("Thing")],
        ],
        scheduling: Scheduling::Stratified,
    };
    let inferred = srl::evaluate(
        &set,
        &data("ex:b ex:type ex:Thing ."),
        &Shapes::default(),
        &RuleOptions::default(),
    )
    .expect("evaluates")
    .inferred()
    .to_vec();
    assert!(
        inferred.contains(&[iri("d"), iri("type"), iri("Thing")]),
        "{inferred:?}"
    );
    assert!(
        inferred.contains(&[iri("d"), iri("tagged"), iri("yes")]),
        "{inferred:?}"
    );
    assert!(
        inferred.contains(&[iri("b"), iri("tagged"), iri("yes")]),
        "{inferred:?}"
    );
    assert!(
        !inferred.contains(&[iri("b"), iri("type"), iri("Thing")]),
        "{inferred:?}"
    );
}

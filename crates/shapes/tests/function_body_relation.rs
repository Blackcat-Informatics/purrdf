// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A `sh:SPARQLFunction` body reaches a host-registered relation.
//!
//! # The oracle, and why it is built this way
//!
//! The defect this file pins was silent in the worst direction: the body parsed, the
//! relation IRI became an ordinary triple pattern, the pattern matched nothing, and
//! the report said `conforms: true`. Nothing failed. A test whose fixture cannot tell
//! "the relation answered and said nothing" from "the relation was never called"
//! would have passed against the defect, so this one is built so it cannot.
//!
//! The data graph carries the two focus nodes' types and NOTHING ELSE. Every fact
//! that could distinguish them comes from the relation's rows. So:
//!
//! * relation honoured → `ex:a` is flagged, `ex:b` is not → exactly `ex:b` is reported;
//! * relation dropped → neither is flagged → BOTH are reported.
//!
//! The verdict is therefore a function of the relation's rows and of nothing else,
//! and the two outcomes differ in MEMBERSHIP rather than merely in a count. Two
//! assertions are made together, because either alone could be satisfied by a broken
//! implementation: the invocation counter moved, and the verdict names exactly the
//! node the relation distinguished.
//!
//! Neighbouring valid cases are executed alongside every refusal, because a refusal
//! is a claim too: a body naming an unregistered predicate, a body naming a predicate
//! that merely shares a registered one's prefix, and the same body with no registry
//! installed. Each must still answer over the base graph exactly as before.
//!
//! # Declared namespaces
//!
//! The last three tests cover the other half of the parse seam — a host declaring a
//! relation NAMESPACE rather than registering an exact IRI. That distinction was
//! unreachable from SHACL until the extension environment carried the parser options:
//! this module's engine is built once per thread and never reconfigured, so
//! `property_fn_namespaces` was permanently empty here however the host configured
//! itself. A declared namespace is how a host says "everything under this prefix is a
//! call, and one I have not registered is a hard error rather than a silent data
//! triple" — so it is tested in all three of its states: declared-and-unregistered
//! (refused, by name), undeclared (ordinary data), declared-and-registered (resolves).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf::RdfDataset;
use purrdf_core::TermValue;
use purrdf_shapes::engine::validate_dataset;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::sparql::{enter_parser_options_scope, enter_property_function_scope};
use purrdf_sparql_algebra::ParserOptions;
use purrdf_sparql_eval::{
    BindingPattern, EvalError, IndexGeneration, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, Volatility,
};

const EX: &str = "http://example.org/ns#";

/// The relation IRI the function body calls. PurRDF mints no vocabulary: without the
/// host registration below, this predicate is an ordinary triple pattern — which is
/// precisely the defect, and precisely what the neighbouring cases rely on.
const REL: &str = "http://example.org/rel/flagged";

/// A sibling predicate that merely SHARES A PREFIX with `REL`. Registering `REL` must
/// not capture it; a registry's keys are exact IRIs, not namespaces.
const REL_SIBLING: &str = "http://example.org/rel/flaggedElsewhere";

/// The generation the relation declares, so the receipt has something to carry.
const GENERATION: &str = "flag-index@7";

/// A relation over one fixed row, counting the invocations the engine makes.
#[derive(Debug)]
struct FlagRelation {
    modes: [BindingPattern; 1],
    opens: Arc<AtomicU64>,
}

#[derive(Debug)]
struct FlagCursor {
    rows: std::vec::IntoIter<PfRow>,
}

impl PfCursor for FlagCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }

    fn generation(&self) -> IndexGeneration {
        IndexGeneration::declared(GENERATION)
    }
}

impl PropertyFunction for FlagRelation {
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
        1
    }

    fn open(
        &self,
        _args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.opens.fetch_add(1, Ordering::Relaxed);
        // The ONE fact that distinguishes the two focus nodes, and it exists nowhere
        // in the data graph. Both positions are free (`$this`/`?node` pre-binding is a
        // substitution, which the evaluation-order analysis does not see as a
        // binding), so the row is emitted whole and the engine unifies it.
        let rows = vec![vec![
            TermValue::Iri(format!("{EX}a")),
            TermValue::Iri(format!("{EX}yes")),
        ]];
        Ok(Box::new(FlagCursor {
            rows: rows.into_iter(),
        }))
    }
}

/// A registry holding `REL`, plus the shared invocation counter.
fn registry() -> (Arc<PropertyFunctionRegistry>, Arc<AtomicU64>) {
    let opens = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL.to_owned(),
        Arc::new(FlagRelation {
            modes: [BindingPattern::from_code("ff")],
            opens: Arc::clone(&opens),
        }),
    );
    (Arc::new(registry), opens)
}

/// A shapes graph whose `sh:SPARQLFunction` body names `predicate`.
fn shapes(predicate: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <{EX}> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:isFlagged
    a sh:SPARQLFunction ;
    sh:parameter [ sh:path ex:node ; sh:nodeKind sh:IRI ] ;
    sh:returnType xsd:boolean ;
    sh:ask "ASK {{ ?node <{predicate}> ?why }}" .

ex:FlagShape
    a sh:NodeShape ;
    sh:targetNode ex:a, ex:b ;
    sh:expression [ ex:isFlagged ( sh:this ) ] .
"#
    );
    purrdf_shapes::engine::parse_shapes(&turtle, None).expect("parse shapes")
}

/// The two focus nodes and their types — and deliberately nothing that could tell
/// them apart. Every distinguishing fact must come from the relation.
fn data() -> Arc<RdfDataset> {
    let triples = format!(
        "<{EX}a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Thing> .\n\
         <{EX}b> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Thing> .\n"
    );
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// Validate `shapes(predicate)` with `relations` installed.
fn validate_with(
    predicate: &str,
    relations: Arc<PropertyFunctionRegistry>,
) -> Result<ValidationReport, String> {
    let _relations = enter_property_function_scope(relations);
    validate_dataset(&data(), &shapes(predicate))
}

/// The focus nodes a report names, in report order.
fn reported(report: &ValidationReport) -> Vec<String> {
    report
        .results
        .iter()
        .map(|r| r.focus_node.to_string())
        .collect()
}

/// The headline: a `sh:SPARQLFunction` body reaches a registered relation, and the
/// verdict turns on the relation's rows.
///
/// What the relation ATTESTED reaching the calling query's receipt is the other half
/// of the contract, and it is pinned where the receipt actually exists — see
/// `crates/sparql-eval/tests/function_body_relation_witness.rs`. The SHACL surface
/// consumes the receipt internally (it refuses a verdict drawn from an index that
/// declared itself not whole) rather than handing it back, so there is no receipt to
/// assert on here.
#[test]
fn a_function_body_reaches_a_registered_relation() {
    let (relations, opens) = registry();
    let report = validate_with(REL, relations).expect("validation runs");

    // (1) The relation was actually invoked. Before the body was bound to the
    //     environment this was zero, and the two assertions below still "passed"
    //     in the sense that nothing errored.
    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the relation was never opened, so the body never reached it"
    );

    // (2) The verdict turns on the relation's rows. `ex:a` is flagged only because
    //     the relation said so; the data graph knows nothing about it.
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}b>")],
        "exactly the node the relation did NOT flag is reported; reporting both \
         would mean the relation's row was dropped"
    );
}

/// The neighbouring valid case, and the one an over-refusal would break: a body
/// naming a predicate NOBODY registered is ordinary data, answers over the base
/// graph, and reports both focus nodes — unchanged by any of this.
#[test]
fn a_body_naming_no_registered_relation_is_ordinary_data() {
    let (relations, opens) = registry();
    // `REL` is registered; the body names a DIFFERENT predicate.
    let report = validate_with(&format!("{EX}plainEdge"), relations).expect("runs");
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "a body that names no registered relation must not invoke one"
    );
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}a>"), format!("<{EX}b>")],
        "neither node has the plain edge, so the ASK is false for both"
    );
}

/// The EXACT-not-PREFIX guard, executed rather than asserted: registering
/// `…/rel/flagged` must not capture `…/rel/flaggedElsewhere`. Getting this wrong
/// turns a working query into a hard error naming the wrong cause — the mirror of
/// the silent drop, and just as invisible until someone writes the query.
#[test]
fn a_sibling_iri_sharing_a_registered_prefix_stays_ordinary_data() {
    let (relations, opens) = registry();
    let report = validate_with(REL_SIBLING, relations)
        .expect("a merely-same-prefixed predicate must not be refused");
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "the sibling must not resolve to the registered relation"
    );
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}a>"), format!("<{EX}b>")],
        "the sibling is an ordinary predicate that matches nothing"
    );
}

/// The same fixture with NO registry installed: the relation IRI is ordinary data,
/// exactly as it was before the host configured anything. This is the control that
/// makes the headline test's verdict attributable to the registration rather than to
/// the fixture.
#[test]
fn without_a_registry_the_same_body_is_ordinary_data() {
    let _relations = enter_property_function_scope(Arc::new(PropertyFunctionRegistry::EMPTY));
    let report = validate_dataset(&data(), &shapes(REL)).expect("validation runs");
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}a>"), format!("<{EX}b>")],
        "with nothing registered the predicate matches nothing and both nodes fail"
    );
}

/// The relation namespace a host declares in the three cases below.
const REL_NS: &str = "http://example.org/rel/";

/// An IRI under `REL_NS` that nobody registers.
const REL_MISSING: &str = "http://example.org/rel/neverRegistered";

/// Validate with `relations` installed AND `REL_NS` declared as a relation namespace.
fn validate_under_declared_namespace(
    predicate: &str,
    relations: Arc<PropertyFunctionRegistry>,
) -> Result<ValidationReport, String> {
    let _relations = enter_property_function_scope(relations);
    let _options = enter_parser_options_scope(Arc::new(ParserOptions {
        property_fn_namespaces: vec![REL_NS.to_owned()],
        ..ParserOptions::default()
    }));
    validate_dataset(&data(), &shapes(predicate))
}

/// A host-DECLARED namespace makes an unregistered IRI under it a hard error, not a
/// silent data triple.
///
/// This is the distinction a namespace declaration exists to draw, and it was
/// unreachable from SHACL before: the thread-local engine was built once with default
/// parser options, so `property_fn_namespaces` was permanently empty on this surface
/// however the host configured itself. A host could have an exact IRI recognized by
/// registering it, and could not say "everything under this prefix is a call".
#[test]
fn a_declared_namespace_makes_an_unregistered_iri_under_it_a_hard_error() {
    let (relations, opens) = registry();
    let error = validate_under_declared_namespace(REL_MISSING, relations)
        .expect_err("an unregistered IRI under a declared relation namespace is refused");
    assert!(
        error.contains(REL_MISSING),
        "the refusal names the offending IRI: {error}"
    );
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "nothing was invoked; the call could not be resolved at all"
    );
}

/// The neighbouring valid case, and the one that proves the refusal above is caused by
/// the DECLARATION rather than by the fixture: the identical body, with no namespace
/// declared, is ordinary data and answers over the base graph.
#[test]
fn without_the_declaration_the_same_iri_is_ordinary_data() {
    let (relations, opens) = registry();
    let report = validate_with(REL_MISSING, relations)
        .expect("with nothing declared the predicate is an ordinary triple pattern");
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "an ordinary predicate invokes no relation"
    );
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}a>"), format!("<{EX}b>")],
        "no node has the edge, so the ASK is false for both"
    );
}

/// And the third case: an IRI under the declared namespace that IS registered resolves
/// and decides the verdict, exactly as the exact-IRI path does.
#[test]
fn a_registered_iri_under_a_declared_namespace_still_reaches_its_relation() {
    let (relations, opens) = registry();
    let report = validate_under_declared_namespace(REL, relations)
        .expect("the registered relation resolves");
    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the relation was never opened"
    );
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}b>")],
        "the verdict still turns on the relation's rows"
    );
}

// ── The pre-flight report ───────────────────────────────────────────────────────

/// `Shapes::extension_usage` answers the question a silent verdict cannot: under
/// THIS environment, did my relation IRI become a call or a data edge?
///
/// The three states are the same three the validation itself has, checked without
/// running one — which is the point, because a validation that answers
/// `conforms: true` looks identical whether the relation ran or was never asked.
#[test]
fn extension_usage_reports_whether_a_function_body_reaches_its_relation() {
    let (relations, _) = registry();
    let shapes = shapes(REL);

    // Registered: the body's predicate is a CALL.
    let wired = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("the fixture declarations read cleanly");
    let usage = shapes.extension_usage(&wired);
    assert!(
        usage.reaches(REL),
        "the registered relation is reached from the function body: {usage:?}"
    );
    assert!(
        !usage.data().contains(REL),
        "an IRI that became a call is not also data: {usage:?}"
    );

    // Nothing registered: the IDENTICAL shapes graph reports the identical IRI as
    // an ordinary data edge. This is the answer a host debugging a silent
    // `conforms: true` actually needs.
    let bare = purrdf_sparql_eval::ExtensionEnv::empty();
    let usage = shapes.extension_usage(bare);
    assert!(
        !usage.reaches(REL),
        "with nothing registered nothing is reached: {usage:?}"
    );
    assert!(
        usage.data().contains(REL),
        "and the IRI is reported as the data edge it became: {usage:?}"
    );
}

/// The report names the DECLARATION, not just the IRI — a shapes author with twenty
/// functions needs to know which one.
#[test]
fn extension_usage_names_the_declaration_a_predicate_sits_in() {
    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("the fixture declarations read cleanly");
    let usage = shapes(REL).extension_usage(&env);

    let site = format!("sh:SPARQLFunction <{EX}isFlagged>");
    let used = usage
        .site(&site)
        .unwrap_or_else(|| panic!("the report names the function declaration; got {usage:?}"));
    assert!(
        used.calls.contains(REL),
        "and says what that declaration's predicate became: {used:?}"
    );
}

/// The prefix-vs-exact trap, reported rather than merely evaluated: registering
/// `…/rel/flagged` must leave `…/rel/flaggedElsewhere` classified as data.
#[test]
fn extension_usage_does_not_claim_a_merely_same_prefixed_sibling() {
    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("the fixture declarations read cleanly");
    let usage = shapes(REL_SIBLING).extension_usage(&env);
    assert!(
        !usage.reaches(REL_SIBLING),
        "a sibling sharing a registered IRI's prefix is not reached: {usage:?}"
    );
    assert!(usage.data().contains(REL_SIBLING));
}

// ── Positional neighbours ───────────────────────────────────────────────

/// A shapes graph whose function body places the registered IRI somewhere that is
/// NOT predicate position.
///
/// These are where a classification most plausibly over-refuses: an implementation
/// that scanned for the IRI rather than reading the algebra would claim every one of
/// them as a call, and an admission pass that did the same would refuse bodies that
/// are perfectly ordinary SPARQL.
fn shapes_with_body(body: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <{EX}> .

ex:isFlagged
    a sh:SPARQLFunction ;
    sh:parameter [ sh:path ex:node ; sh:nodeKind sh:IRI ] ;
    sh:ask """{body}""" .

ex:FlagShape
    a sh:NodeShape ;
    sh:targetNode ex:a, ex:b ;
    sh:expression [ ex:isFlagged ( sh:this ) ] .
"#
    );
    purrdf_shapes::engine::parse_shapes(&turtle, None).expect("parse shapes")
}

/// A registered relation IRI outside predicate position is ordinary RDF, and must
/// stay so: it is not a call, it invokes nothing, and it is not refused.
///
/// Each body is executed, not merely parsed. A refusal here would be the mirror of
/// the silent-drop defect — a working query broken with a diagnostic naming the
/// wrong cause — and it is exactly the failure a scan-the-text implementation makes.
#[test]
fn a_registered_iri_outside_predicate_position_is_not_a_call() {
    for (label, body) in [
        ("subject position", format!("ASK {{ <{REL}> ?p ?o }}")),
        ("object position", format!("ASK {{ ?s ?p <{REL}> }}")),
        (
            "inside VALUES",
            format!("ASK {{ VALUES ?v {{ <{REL}> }} }}"),
        ),
        (
            "a nested sub-SELECT's projection",
            format!("ASK {{ {{ SELECT ?s WHERE {{ ?s ?p <{REL}> }} }} }}"),
        ),
        (
            "inside a GRAPH block, in object position",
            format!("ASK {{ GRAPH ?g {{ ?s ?p <{REL}> }} }}"),
        ),
        (
            "an expression operand",
            format!("ASK {{ ?s ?p ?o FILTER(?o = <{REL}>) }}"),
        ),
        (
            "a property path, not a bare predicate",
            format!("ASK {{ ?s <{REL}>* ?o }}"),
        ),
    ] {
        let (relations, opens) = registry();
        let shapes = shapes_with_body(&body);

        let report = {
            let _relations = enter_property_function_scope(relations);
            validate_dataset(&data(), &shapes)
        }
        .unwrap_or_else(|error| {
            panic!("{label}: an ordinary body must not be refused: {error}\n  body: {body}")
        });

        assert_eq!(
            opens.load(Ordering::Relaxed),
            0,
            "{label}: the IRI is not in predicate position, so nothing may be invoked"
        );
        // The VERDICT is deliberately not asserted here, and the `VALUES` case is
        // why: `ASK { VALUES ?v { <iri> } }` is TRUE, because a one-row `VALUES`
        // block is a non-empty solution set. What each body answers is ordinary
        // SPARQL semantics, which vary per body and are not what this test is about.
        // The three claims that ARE about the seam are asserted instead: the body
        // was not refused (above), nothing was invoked, and nothing is reported as a
        // call.
        let _ = reported(&report);

        // And the pre-flight report agrees: not a call anywhere in the graph.
        let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*registry().0).clone())
            .expect("the fixture declarations read cleanly");
        assert!(
            !shapes.extension_usage(&env).reaches(REL),
            "{label}: the usage report must not claim a call either"
        );
    }
}

// ── The door census ────────────────────────────────────────────────────

/// EVERY construct that carries SPARQL text reaches the extension environment.
///
/// The defect this file pins was one door out of seven. The other six kept their
/// query TEXT and were re-parsed by the engine against the environment in force, so
/// they answered correctly; `sh:SPARQLFunction` alone froze algebra at load time and
/// could not reach a relation at all. Nothing made that asymmetry visible — each
/// door was correct or not on its own, and no test asked the question across all of
/// them.
///
/// So this asks it. One shapes graph naming the registered relation from every
/// SPARQL-bearing construct at once, and the pre-flight report has to see a call in
/// each. A door added later that forgets the environment fails here rather than
/// being discovered by whoever writes the query that should work and does not.
#[test]
fn every_sparql_bearing_construct_reaches_the_environment() {
    let turtle = format!(
        r#"
@prefix sh:     <http://www.w3.org/ns/shacl#> .
@prefix ex:     <{EX}> .
@prefix sparql: <http://www.w3.org/ns/sparql#> .

ex:isFlagged
    a sh:SPARQLFunction ;
    sh:parameter [ sh:path ex:node ; sh:nodeKind sh:IRI ] ;
    sh:ask """ASK {{ ?node <{REL}> ?why }}""" .

ex:ByTargetType
    a sh:SPARQLTargetType ;
    sh:parameter [ sh:path ex:kind ] ;
    sh:select """SELECT ?this WHERE {{ ?this <{REL}> ?kind }}""" .

ex:CensusShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:target [
        a sh:SPARQLTarget ;
        sh:select """SELECT ?this WHERE {{ ?this <{REL}> ?why }}""" ;
    ] ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:select """SELECT $this ?value WHERE {{ $this <{REL}> ?value }}""" ;
    ] ;
    sh:rule [
        a sh:SPARQLRule ;
        sh:construct """CONSTRUCT {{ $this <{EX}out> ?why }} WHERE {{ $this <{REL}> ?why }}""" ;
    ] ;
    sh:expression [
        sparql:equals (
            [ sh:select """SELECT ?result WHERE {{ $this <{REL}> ?result }}""" ]
            [ ex:isFlagged ( sh:this ) ]
        )
    ] .
"#
    );
    let shapes = purrdf_shapes::engine::parse_shapes(&turtle, None)
        .unwrap_or_else(|error| panic!("the census fixture must load: {error}"));

    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("the fixture declarations read cleanly");
    let usage = shapes.extension_usage(&env);

    // Every construct in the fixture is represented, named by its own declaration.
    let sites: Vec<&str> = usage.sites().map(|(site, _)| site.as_str()).collect();
    for expected in [
        "sh:SPARQLFunction",
        "sh:SPARQLTargetType",
        "sh:target on",
        "sh:sparql on",
        "sh:rule on",
        "sh:select node expression on",
    ] {
        assert!(
            sites.iter().any(|site| site.starts_with(expected)),
            "no site named {expected:?} in the census; the report covers {sites:?}"
        );
    }

    // And every one of them reached the relation. A door that forgot the
    // environment would report the IRI as data here.
    for (site, used) in usage.sites() {
        assert!(
            used.calls.contains(REL),
            "{site} did not reach the relation; it saw {used:?}"
        );
    }

    // The control: with nothing registered, not one of them reaches it. This is
    // what makes the assertions above attributable to the registration.
    let bare = shapes.extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());
    assert!(
        !bare.reaches(REL),
        "with nothing registered no door reaches the relation: {bare:?}"
    );
    assert!(
        bare.data().contains(REL),
        "and every door reports it as the data edge it became: {bare:?}"
    );
}

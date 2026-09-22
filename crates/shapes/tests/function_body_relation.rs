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
    PropertyFunctionRegistry, ServiceLevel, Volatility,
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
/// of the contract. It is pinned twice: against the receipt value itself in
/// `crates/sparql-eval/tests/function_body_relation_witness.rs`, and on THIS surface
/// by `an_incomplete_index_declared_from_a_function_body_refuses_the_verdict` below.
///
/// An earlier version of this comment claimed the SHACL surface consumes the receipt
/// internally and so offers nothing to assert on. That was wrong, and it mattered:
/// the consumption is itself observable, because the governed lane refuses a verdict
/// drawn from an index a relation declared incomplete, and it names the relation when
/// it does. A refusal naming an IRI only a function body ever mentioned is a direct
/// reading of the attestation on the caller's receipt.
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
/// each. A construct added later that `extension_usage` forgets to walk fails here,
/// rather than silently reporting nothing for a declaration a host asked about.
///
/// # What this does and does not prove
///
/// It grades the REPORT's coverage, not the evaluator's. The assertion is on
/// `extension_usage`'s own parse, so a door that the report sees but that evaluated
/// under a different configuration would still pass — and for a while exactly that
/// was true, because the engine carried parse configuration of its own and only the
/// function-body bind read the environment's.
///
/// That gap is now closed structurally rather than by this test: `prepare_for` takes
/// an `ExtensionEnv` and nothing else, so there is no second configuration for a door
/// to read, and the engine has no field to hold one. Execution-level evidence for the
/// two doors where the difference was observable lives in
/// `a_declared_namespace_reaches_a_sparql_constraint_body` and
/// `a_declared_namespace_makes_an_unregistered_iri_under_it_a_hard_error`, which
/// validate rather than re-parse.
#[test]
fn extension_usage_walks_every_sparql_bearing_construct() {
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

// ---------------------------------------------------------------------------
// The sibling door: `sh:sparql`
// ---------------------------------------------------------------------------

/// The same predicate, in a `sh:sparql` CONSTRAINT rather than a function body.
///
/// A `sh:SPARQLFunction` body and a `sh:sparql` body are two spellings of "SPARQL
/// text this shapes graph carries". Nothing about a host's declaration is specific to
/// one of them, so a declaration that reaches one and not the other is not a feature
/// of either construct — it is two different parse configurations wearing one name.
fn sparql_constraint_shapes(predicate: &str) -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <{EX}> .

ex:FlagShape
    a sh:NodeShape ;
    sh:targetNode ex:a, ex:b ;
    sh:sparql [
        sh:select "SELECT $this WHERE {{ $this <{predicate}> ?why }}" ;
    ] .
"#
    );
    purrdf_shapes::engine::parse_shapes(&turtle, None).expect("parse shapes")
}

/// Validate a `sh:sparql` constraint with `relations` installed and `REL_NS` declared.
fn validate_constraint_under_declared_namespace(
    predicate: &str,
    relations: Arc<PropertyFunctionRegistry>,
) -> Result<ValidationReport, String> {
    let _relations = enter_property_function_scope(relations);
    let _options = enter_parser_options_scope(Arc::new(ParserOptions {
        property_fn_namespaces: vec![REL_NS.to_owned()],
        ..ParserOptions::default()
    }));
    validate_dataset(&data(), &sparql_constraint_shapes(predicate))
}

/// A declared namespace reaches a `sh:sparql` body, not only a `sh:SPARQLFunction`
/// body.
///
/// This is the row that was silently wrong. The engine held its own base
/// [`ParserOptions`] and every ordinary query parsed against THOSE, while the
/// environment's declared options reached one door: the function-body bind. The
/// thread-local SHACL engine is built with default options, so on this surface a
/// declared relation namespace applied to `sh:SPARQLFunction` bodies and to nothing
/// else. The identical IRI, in the identical host, under the identical declaration,
/// hard-errored in one construct and conformed green in the other.
///
/// That is this file's own headline defect wearing a different construct — a wired,
/// named relation silently becoming a data edge under a green report — surviving at a
/// sibling door, which is why the fix was to delete the engine's parse configuration
/// rather than to add the environment to one more call.
#[test]
fn a_declared_namespace_reaches_a_sparql_constraint_body() {
    let (relations, opens) = registry();
    let error = validate_constraint_under_declared_namespace(REL_MISSING, relations)
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

/// The valid neighbour, and the control that proves the refusal above comes from the
/// DECLARATION rather than from the constraint fixture: the identical body with
/// nothing declared is an ordinary triple pattern and answers over the base graph.
///
/// It must SUCCEED. A refusal here would mean the fix had over-tightened `sh:sparql`
/// into rejecting predicates that are simply data.
#[test]
fn without_the_declaration_the_same_constraint_body_is_ordinary_data() {
    let (relations, opens) = registry();
    let _relations = enter_property_function_scope(relations);
    let report = validate_dataset(&data(), &sparql_constraint_shapes(REL_MISSING))
        .expect("with nothing declared the predicate is an ordinary triple pattern");
    assert_eq!(
        opens.load(Ordering::Relaxed),
        0,
        "an ordinary predicate invokes no relation"
    );
    assert!(
        report.conforms,
        "no node carries that edge, so the SELECT returns no rows and nothing is \
         reported: {:?}",
        reported(&report)
    );
}

/// And the third row: a REGISTERED IRI under the declared namespace resolves from a
/// `sh:sparql` body and its rows decide the verdict.
///
/// The relation's single row names `ex:a`, a node no data triple distinguishes, so a
/// report naming exactly `ex:a` is reachable only if the call resolved. Dropped, the
/// body matches nothing and the report is empty.
#[test]
fn a_registered_iri_reaches_its_relation_from_a_sparql_constraint_body() {
    let (relations, opens) = registry();
    let report = validate_constraint_under_declared_namespace(REL, relations)
        .expect("the registered relation resolves");
    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the relation was opened from a sh:sparql body"
    );
    assert_eq!(
        reported(&report),
        vec![format!("<{EX}a>")],
        "the relation's own row decided the verdict, and no data triple could have"
    );
}

// ---------------------------------------------------------------------------
// The attestation reaches the CALLER's receipt, observed on the SHACL surface
// ---------------------------------------------------------------------------

/// A relation that answers rows AND declares its index was not whole.
#[derive(Debug)]
struct PartialRelation {
    modes: [BindingPattern; 1],
    opens: Arc<AtomicU64>,
}

#[derive(Debug)]
struct PartialCursor {
    rows: std::vec::IntoIter<PfRow>,
}

impl PfCursor for PartialCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.rows.next())
    }

    fn generation(&self) -> IndexGeneration {
        IndexGeneration::declared(GENERATION)
    }

    fn service_level(&self) -> ServiceLevel {
        ServiceLevel::Incomplete {
            reason: "shard-3 offline".to_owned(),
        }
    }
}

impl PropertyFunction for PartialRelation {
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
        let rows = vec![vec![
            TermValue::Iri(format!("{EX}a")),
            TermValue::Iri(format!("{EX}yes")),
        ]];
        Ok(Box::new(PartialCursor {
            rows: rows.into_iter(),
        }))
    }
}

/// The witness crosses the function-body boundary onto the CALLER's governed receipt,
/// observed through the SHACL surface rather than through the evaluator.
///
/// The relation is reachable ONLY from inside the `sh:SPARQLFunction` body — no other
/// construct in the shapes graph mentions its IRI. It declares its index was not
/// whole, and the governed lane refuses a conformance verdict drawn from such an
/// index, naming the relation. So a refusal that names this IRI can only have been
/// produced by an attestation that travelled out of the function-body child context
/// and onto the receipt the caller's validation read.
///
/// If the witness were dropped at that boundary, the receipt would carry no
/// incompleteness, the verdict would be computed over an index that was not whole,
/// and this validation would return a report instead of an error. A dropped witness
/// is therefore not a missing diagnostic but a wrong answer, which is why it is
/// pinned here rather than left to the evaluator-level test alone.
#[test]
fn an_incomplete_index_declared_from_a_function_body_refuses_the_verdict() {
    let opens = Arc::new(AtomicU64::new(0));
    let mut registry = PropertyFunctionRegistry::default();
    registry.register(
        REL,
        Arc::new(PartialRelation {
            modes: [BindingPattern::from_code("ff")],
            opens: Arc::clone(&opens),
        }),
    );

    let _relations = enter_property_function_scope(Arc::new(registry));
    let error = purrdf_shapes::engine::validate_dataset_with_governors(
        &data(),
        &shapes(REL),
        None,
        &purrdf_sparql_eval::QueryGovernors::UNBOUNDED,
    )
    .expect_err("a verdict over an index declared not whole is refused");

    assert!(
        opens.load(Ordering::Relaxed) > 0,
        "the relation must actually have been invoked from the function body"
    );
    assert!(
        error.contains(REL),
        "the refusal names the relation, which is the attestation being read off the \
         caller's receipt: {error}"
    );
    assert!(
        error.contains("shard-3 offline"),
        "and it carries the relation's own reason, so what arrived is the attestation \
         rather than a generic flag: {error}"
    );
}

// ---------------------------------------------------------------------------
// A site this environment cannot read is reported, not deleted
// ---------------------------------------------------------------------------

/// A shapes graph whose rule names `REL` in the CONSTRUCT **template** rather than in
/// the WHERE clause.
///
/// The parser refuses a property-function call in a template — a template writes
/// triples, it does not read them, so there is nothing for a call to mean there. That
/// refusal is what makes this fixture reach the arm: the loader parsed the rule blind
/// and accepted it, and only an environment that recognizes `REL` turns the same text
/// into something unreadable.
fn shapes_with_relation_in_a_construct_template() -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <{EX}> .

ex:RuleShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:rule [
        a sh:SPARQLRule ;
        sh:construct """CONSTRUCT {{ $this <{REL}> ?why }} WHERE {{ $this <{EX}seed> ?why }}""" ;
    ] .
"#
    );
    purrdf_shapes::engine::parse_shapes(&turtle, None).expect("the fixture loads blind")
}

/// A site the environment cannot read is REPORTED, with the parser's own reason.
///
/// Before this, the walk returned early on a parse failure and the site vanished from
/// the report entirely — so a host asking "will my relation be reached here?" got a
/// report in which the rule simply did not exist, indistinguishable from a shape that
/// carries no SPARQL at all. That is a silent answer from the one instrument built to
/// end silent answers.
#[test]
fn a_site_this_environment_cannot_read_is_reported_with_its_reason() {
    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("environment over the registry");

    let usage = shapes_with_relation_in_a_construct_template().extension_usage(&env);

    assert!(
        !usage.is_complete(),
        "the environment cannot read this graph, and the report must say so"
    );
    let unreadable: Vec<_> = usage.unreadable().collect();
    assert_eq!(
        unreadable.len(),
        1,
        "exactly the one rule is unreadable: {unreadable:?}"
    );
    let (site, why) = unreadable[0];
    assert!(
        site.as_str().contains("sh:rule"),
        "the report names WHICH declaration it could not read: {site}"
    );
    assert!(
        !why.is_empty(),
        "and carries the parser's own reason rather than a bare flag"
    );
}

/// The valid neighbour: the SAME shapes graph under an environment that does not
/// recognize the IRI reads completely, and the template's predicate is ordinary data.
///
/// This is what proves the report above is caused by the ENVIRONMENT rather than by a
/// malformed fixture — and it must succeed, or the change has turned a readable
/// shapes graph into an unreadable one.
#[test]
fn the_same_graph_under_an_empty_environment_reads_completely() {
    let usage = shapes_with_relation_in_a_construct_template()
        .extension_usage(purrdf_sparql_eval::ExtensionEnv::empty());

    assert!(
        usage.is_complete(),
        "nothing is registered, so the template predicate is an ordinary IRI and the \
         rule parses: {:?}",
        usage.unreadable().collect::<Vec<_>>()
    );
    assert!(
        usage.calls().is_empty(),
        "and no predicate in it became a call"
    );
}

// ---------------------------------------------------------------------------
// The remaining four doors, at EVALUATION level
// ---------------------------------------------------------------------------

/// Run `shapes_ttl` under `REL_NS` declared and the registry installed.
///
/// Returns the validation's own result, so a caller asserts on the refusal text or on
/// the report rather than on a proxy.
fn validate_ttl_under_declared_namespace(shapes_ttl: &str) -> Result<ValidationReport, String> {
    let (relations, _) = registry();
    let _relations = enter_property_function_scope(relations);
    let _options = enter_parser_options_scope(Arc::new(ParserOptions {
        property_fn_namespaces: vec![REL_NS.to_owned()],
        ..ParserOptions::default()
    }));
    let shapes = purrdf_shapes::engine::parse_shapes(shapes_ttl, None).expect("the fixture loads");
    validate_dataset(&data(), &shapes)
}

/// The same shapes graph with the registry installed but NOTHING declared.
fn validate_ttl_undeclared(shapes_ttl: &str) -> Result<ValidationReport, String> {
    let (relations, _) = registry();
    let _relations = enter_property_function_scope(relations);
    let shapes = purrdf_shapes::engine::parse_shapes(shapes_ttl, None).expect("the fixture loads");
    validate_dataset(&data(), &shapes)
}

/// A `sh:SPARQLTarget` naming `predicate`.
fn sparql_target_ttl(predicate: &str) -> String {
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:TargetShape
    a sh:NodeShape ;
    sh:target [ a sh:SPARQLTarget ; sh:select """SELECT ?this WHERE {{ ?this <{predicate}> ?why }}""" ] ;
    sh:nodeKind sh:IRI .
"#
    )
}

/// A `sh:SPARQLTargetType` naming `predicate`.
fn sparql_target_type_ttl(predicate: &str) -> String {
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:ByKind
    a sh:SPARQLTargetType ;
    sh:parameter [ sh:path ex:kind ] ;
    sh:select """SELECT ?this WHERE {{ ?this <{predicate}> ?kind }}""" .

ex:TargetTypeShape
    a sh:NodeShape ;
    sh:target [ a ex:ByKind ; ex:kind ex:any ] ;
    sh:nodeKind sh:IRI .
"#
    )
}

/// A `sh:expression` whose `sh:select` node expression names `predicate`.
fn expression_ttl(predicate: &str) -> String {
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:ExprShape
    a sh:NodeShape ;
    sh:targetNode ex:a, ex:b ;
    sh:expression [ sh:select """SELECT ?result WHERE {{ $this <{predicate}> ?result }}""" ] .
"#
    )
}

/// The four SPARQL-bearing doors the rest of this file does not reach at evaluation
/// level each take a declared namespace, and each still answers without one.
///
/// `sh:sparql` and `sh:SPARQLFunction` are covered above. These are the other four,
/// and they are covered HERE rather than through `extension_usage`, because the
/// pre-flight report re-parses the text itself and so cannot observe a door whose
/// EVALUATION reads a different configuration — which is exactly the defect this
/// branch closed. A door proven only by the report is not proven.
///
/// Each is executed twice. Declared-and-unregistered must refuse and name the IRI;
/// the undeclared neighbour must still SUCCEED, because a refusal there would mean
/// the seam had over-tightened into rejecting ordinary data.
#[test]
fn every_remaining_door_takes_a_declared_namespace_and_still_answers_without_one() {
    for (door, ttl) in [
        ("sh:SPARQLTarget", sparql_target_ttl as fn(&str) -> String),
        ("sh:SPARQLTargetType", sparql_target_type_ttl),
        ("sh:expression/sh:select", expression_ttl),
    ] {
        let refused = validate_ttl_under_declared_namespace(&ttl(REL_MISSING))
            .err()
            .unwrap_or_else(|| {
                panic!("{door}: a declared-but-unregistered relation IRI must be refused")
            });
        assert!(
            refused.contains(REL_MISSING),
            "{door}: the refusal must name the offending IRI: {refused}"
        );

        validate_ttl_undeclared(&ttl(REL_MISSING)).unwrap_or_else(|error| {
            panic!("{door}: with nothing declared the same IRI is ordinary data: {error}")
        });
    }
}

/// `sh:rule` is the fourth, and it needs its own entry point.
///
/// `validate_dataset` does not execute rules at all — their production door is
/// `rules::entail_dataset`. Asserting rule behaviour through the validator would look
/// like coverage and grade nothing, so the rule is driven through the entry that
/// actually runs it.
#[test]
fn a_sparql_rule_takes_a_declared_namespace_and_still_answers_without_one() {
    let ttl = |predicate: &str| {
        format!(
            r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:RuleShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:rule [
        a sh:SPARQLRule ;
        sh:construct """CONSTRUCT {{ $this <{EX}out> ?why }} WHERE {{ $this <{predicate}> ?why }}""" ;
    ] .
"#
        )
    };

    let declared = {
        let (relations, _) = registry();
        let _relations = enter_property_function_scope(relations);
        let _options = enter_parser_options_scope(Arc::new(ParserOptions {
            property_fn_namespaces: vec![REL_NS.to_owned()],
            ..ParserOptions::default()
        }));
        let shapes =
            purrdf_shapes::engine::parse_shapes(&ttl(REL_MISSING), None).expect("fixture loads");
        purrdf_shapes::rules::entail_dataset(&data(), &shapes)
    };
    let Err(refused) = declared else {
        panic!("a declared-but-unregistered relation IRI must be refused in a rule body");
    };
    assert!(
        refused.contains(REL_MISSING),
        "the refusal must name the offending IRI: {refused}"
    );

    let undeclared = {
        let (relations, _) = registry();
        let _relations = enter_property_function_scope(relations);
        let shapes =
            purrdf_shapes::engine::parse_shapes(&ttl(REL_MISSING), None).expect("fixture loads");
        purrdf_shapes::rules::entail_dataset(&data(), &shapes)
    };
    undeclared.expect("with nothing declared the same IRI is ordinary data and the rule runs");
}

/// A `sh:select` nested inside a `sh:union` operand is reported.
///
/// `walk_node_expr` used to end in a `_ => {}`, so every operand-bearing node
/// expression that is not a `Call` — `sh:union`, `sh:if`, `sh:filterShape`,
/// `shnex:flatMap`, a custom call — was skipped whole. A `sh:select` inside any of
/// them contributed neither calls nor data, and the report said the graph reached no
/// relation while the evaluator went on to invoke one.
///
/// That is the same silent under-report the swallowed parse error was, arriving by a
/// different route: a report that answers confidently about a graph it did not finish
/// reading.
#[test]
fn a_select_nested_in_a_union_operand_is_reported() {
    let turtle = format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:NestedShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:expression [
        sh:union (
            [ sh:select """SELECT ?result WHERE {{ $this <{REL}> ?result }}""" ]
            [ sh:select """SELECT ?result WHERE {{ $this <{EX}plain> ?result }}""" ]
        )
    ] .
"#
    );
    let shapes = purrdf_shapes::engine::parse_shapes(&turtle, None)
        .unwrap_or_else(|error| panic!("the fixture must load: {error}"));

    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("environment over the registry");
    let usage = shapes.extension_usage(&env);

    assert!(
        usage.reaches(REL),
        "the relation is named by a sh:select inside a sh:union operand: {:?}",
        usage.sites().collect::<Vec<_>>()
    );
    assert!(
        usage.data().contains(&format!("{EX}plain").as_str()),
        "and the sibling operand's ordinary predicate is reported as data: {:?}",
        usage.data()
    );
}

// ---------------------------------------------------------------------------
// Shapes and function bodies the report used to walk past
// ---------------------------------------------------------------------------

/// A `sh:sparql` naming `REL`, inside a shape reached through `sh:filterShape`.
///
/// `inline` picks the half that matters: an anonymous shape written in place, versus
/// the same constraint in a NAMED shape the graph also declares at top level.
fn filter_shape_ttl(inline: bool) -> String {
    let constraint = format!(
        r#"sh:sparql [ a sh:SPARQLConstraint ; sh:select """SELECT $this ?v WHERE {{ $this <{REL}> ?v }}""" ]"#
    );
    let (decl, arg) = if inline {
        (String::new(), format!("[ {constraint} ]"))
    } else {
        (
            format!("ex:Inner a sh:NodeShape ; {constraint} .\n"),
            "ex:Inner".to_owned(),
        )
    };
    format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

{decl}
ex:OuterShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:expression [ sh:filterShape {arg} ; sh:nodes sh:this ] .
"#
    )
}

/// An INLINE shape's SPARQL is in the report, exactly as a named one's is.
///
/// The named case is the control, and it passed before this: a named shape is in
/// `Shapes::node_shapes`, so the shape walk reaches it on its own. An anonymous shape
/// is in no such list — `node_shapes` collects top-level ids — so nothing reached it,
/// and `extension_usage` reported `reaches = false` and `is_complete() = true` about
/// a graph whose evaluation invokes the relation. A confident wrong answer from the
/// instrument whose entire purpose is to replace a silent one.
///
/// The two fixtures differ in exactly one respect, which is what makes this an
/// oracle rather than a pair of smoke tests.
#[test]
fn an_inline_filter_shape_s_sparql_is_reported_like_a_named_one_s() {
    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("environment over the registry");

    for inline in [false, true] {
        let shapes = purrdf_shapes::engine::parse_shapes(&filter_shape_ttl(inline), None)
            .unwrap_or_else(|error| panic!("inline={inline}: the fixture must load: {error}"));
        let usage = shapes.extension_usage(&env);
        assert!(
            usage.reaches(REL),
            "inline={inline}: the constraint inside the filter shape names the relation, \
             so the report must say so: sites={:?} complete={}",
            usage
                .sites()
                .map(|(s, _)| s.to_string())
                .collect::<Vec<_>>(),
            usage.is_complete(),
        );
    }
}

/// A `sh:select` inside a custom node-expression function's own body is reported.
///
/// The function is declared with `sh:bodyExpression` and invoked from a shape. Its
/// body is SPARQL the shapes graph carries and the environment reads, but it hangs
/// off the function declaration rather than off any shape, so walking shapes alone
/// never reached it and the report said the graph reached nothing.
#[test]
fn a_select_inside_a_custom_function_body_is_reported() {
    let turtle = format!(
        r#"
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix ex:    <{EX}> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .

ex:viaBody
    a sh:ListParameterExpressionFunction ;
    rdfs:subClassOf sh:ListParameterExpression ;
    sh:bodyExpression [ sh:select """SELECT ?result WHERE {{ $this <{REL}> ?result }}""" ] ;
    sh:parameter [ a sh:Parameter ; sh:path shnex:arg0 ; sh:nodeKind sh:IRI ] .

ex:BodyShape
    a sh:NodeShape ;
    sh:targetNode ex:a ;
    sh:expression [ ex:viaBody ( sh:this ) ] .
"#
    );
    let shapes = purrdf_shapes::engine::parse_shapes(&turtle, None)
        .unwrap_or_else(|error| panic!("the fixture must load: {error}"));

    let (relations, _) = registry();
    let env = purrdf_sparql_eval::ExtensionEnv::over_relations((*relations).clone())
        .expect("environment over the registry");
    let usage = shapes.extension_usage(&env);

    assert!(
        usage.reaches(REL),
        "the function's own body names the relation: sites={:?}",
        usage
            .sites()
            .map(|(s, _)| s.to_string())
            .collect::<Vec<_>>()
    );
}

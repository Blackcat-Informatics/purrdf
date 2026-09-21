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
//! Three neighbouring valid cases are executed alongside, because a refusal is a
//! claim too: a body naming an unregistered predicate, a body naming a predicate that
//! merely shares a registered one's prefix, and the same body with no registry
//! installed. All three must still answer over the base graph exactly as before.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf::RdfDataset;
use purrdf_core::TermValue;
use purrdf_shapes::engine::validate_dataset;
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::sparql::enter_property_function_scope;
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

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A `sh:sparql` body that invokes a relation whose index declares itself not whole
//! cannot yield a conformance verdict — on the governed lane as much as on the
//! ungoverned one.
//!
//! The ungoverned SPARQL lane refuses such a relation at the seam, because its result
//! type has nowhere to report the shortfall. The governed lane records it instead, on
//! the receipt every governed outcome carries, and leaves the refusal to whoever reads
//! that receipt. SHACL validation is a governed caller that reads only the rows, so the
//! receipt is where the shortfall would have been silently dropped: "this focus node
//! has no violating solution" computed over an index that was not whole is
//! indistinguishable from the same sentence computed over the whole index. So the
//! validation refuses it by name, exactly as it refuses a truncated bag.
//!
//! Both halves are executed: the relation that declares a shortfall is refused, and
//! the same relation declaring nothing validates to a report.

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{GovernedValidation, validate_dataset_with_governors};
use purrdf_shapes::sparql::enter_property_function_scope;
use purrdf_sparql_eval::{
    BindingPattern, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, QueryGovernors, ServiceLevel, Volatility,
};

const EX: &str = "http://example.org/";

/// The relation IRI the constraint body calls. PurRDF mints no vocabulary: without the
/// host registration below, the same predicate is an ordinary triple pattern.
const REL_IRI: &str = "http://example.org/rel/memberOf";

/// The reason the incomplete fixture declares, verbatim.
const SHARD_REASON: &str = "shard 3 of 4 is still rebuilding";

/// A relation over no rows whose cursor declares the service level it is built with.
#[derive(Debug)]
struct AttestingRelation {
    modes: [BindingPattern; 1],
    incomplete: Option<&'static str>,
}

#[derive(Debug)]
struct AttestingCursor {
    incomplete: Option<&'static str>,
}

impl PfCursor for AttestingCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(None)
    }

    fn service_level(&self) -> ServiceLevel {
        match self.incomplete {
            Some(reason) => ServiceLevel::Incomplete {
                reason: reason.to_owned(),
            },
            None => ServiceLevel::Undeclared,
        }
    }
}

impl PropertyFunction for AttestingRelation {
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
        Ok(Box::new(AttestingCursor {
            incomplete: self.incomplete,
        }))
    }
}

fn registry(incomplete: Option<&'static str>) -> Arc<PropertyFunctionRegistry> {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL_IRI.to_owned(),
        Arc::new(AttestingRelation {
            // Both positions free: this file is about the shortfall a cursor declares,
            // not about access modes, so the relation serves any invocation.
            modes: [BindingPattern::from_code("ff")],
            incomplete,
        }),
    );
    Arc::new(registry)
}

/// One shape whose `sh:sparql` body reaches the relation for every focus node.
fn shapes() -> purrdf_shapes::shapes::Shapes {
    let turtle = format!(
        r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .

ex:PersonShape
    a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "no person may be a member of a team" ;
        sh:select """
            SELECT $this ?value
            WHERE {{
                $this <{REL_IRI}> ?value .
            }}
        """ ;
    ] .
"#
    );
    purrdf_shapes::engine::parse_shapes(&turtle, None).expect("parse shapes")
}

fn people() -> Arc<RdfDataset> {
    let triples = format!(
        "<{EX}p0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Person> .\n\
         <{EX}p1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Person> .\n"
    );
    purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&triples)
        .unwrap_or_else(|errors| panic!("fixture data: {}", errors.join("\n")))
}

/// Validate the fixture with `relations` installed, on the governed lane.
fn validate(relations: Arc<PropertyFunctionRegistry>) -> Result<GovernedValidation, String> {
    let _relations = enter_property_function_scope(relations);
    validate_dataset_with_governors(&people(), &shapes(), None, &QueryGovernors::UNBOUNDED)
}

#[test]
fn a_relation_declaring_its_index_not_whole_is_refused_and_a_whole_one_validates() {
    // THE REFUSAL: the relation's cursor declares a shortfall at close; the governed
    // lane records it on the receipt; the validation reads the receipt and refuses.
    let refused = validate(registry(Some(SHARD_REASON)))
        .expect_err("a verdict over an index that was not whole is not a verdict");
    assert!(
        refused.contains(REL_IRI) && refused.contains(SHARD_REASON),
        "the refusal names the relation and carries its own reason verbatim: {refused}"
    );
    assert!(
        refused.contains("was not whole"),
        "the refusal says what the index declared: {refused}"
    );

    // THE NEIGHBOUR: the same relation, declaring nothing, validates to a report — the
    // body finds no membership, so both people conform.
    let whole = validate(registry(None)).expect("a relation declaring no shortfall validates");
    let GovernedValidation::Complete { report, evidence } = whole else {
        panic!("an unbounded governor never trips: {whole:?}");
    };
    assert!(
        report.conforms,
        "no person is a member of anything: {report:?}"
    );
    assert!(evidence.is_complete(), "nothing tripped: {evidence:?}");
}

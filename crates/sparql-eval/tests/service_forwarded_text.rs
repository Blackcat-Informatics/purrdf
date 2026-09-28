// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A `SERVICE` body the parser admitted is forwarded as text the parser admits again.
//!
//! The in-process resolver re-parses the text a `SERVICE` forwards. When that text
//! bracketed every operator, a `FILTER` with a few hundred chained operators — well
//! inside the parser's height budget — was forwarded nested several hundred brackets
//! deep, past the nesting budget; the re-parse refused it as an undecodable request,
//! and `SERVICE SILENT` answered that failure with the join identity: every outer row,
//! unfiltered, with no symptom. An HTTP endpoint would have refused the same text.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, SparqlRequest, SparqlResult,
    TermValue,
};
use purrdf_sparql_eval::{InProcessServiceResolver, NativeSparqlEngine, QueryOptions};

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// How many subjects the fixture has: `<s0>` … `<s9>`, each `<v>` its own index.
const SUBJECTS: usize = 10;

/// The operators in the forwarded filter: 439 additions and the comparison.
const OPERATORS: usize = 440;

fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let v = builder.intern_iri(&format!("{EX}v"));
    for n in 0..SUBJECTS {
        let s = builder.intern_iri(&format!("{EX}s{n}"));
        let value = builder.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER));
        builder.push_quad(s, v, value, None);
    }
    builder.freeze().expect("the fixture dataset")
}

/// Every subject of the outer pattern, joined with a `SERVICE` whose body keeps only the
/// subjects whose value, plus zero `OPERATORS - 1` times, is below `bound`.
fn query(silent: bool, bound: i32) -> String {
    let chain = " + 0".repeat(OPERATORS - 1);
    let silent = if silent { "SILENT " } else { "" };
    format!(
        "SELECT ?s WHERE {{ ?s <{EX}v> ?v \
         SERVICE {silent}<{EX}svc> {{ ?s <{EX}v> ?w FILTER(?w{chain} < {bound}) }} }}"
    )
}

/// The sorted local names `?s` binds, or the diagnostic.
fn run(query: &str) -> Result<Vec<String>, RdfDiagnostic> {
    let resolver = InProcessServiceResolver::new().with_endpoint(format!("{EX}svc"), dataset());
    let SparqlResult::Solutions {
        variables, rows, ..
    } = NativeSparqlEngine::new().query_with_source(
        &dataset(),
        SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        },
        &resolver,
        QueryOptions::EMPTY,
    )?
    else {
        panic!("a SELECT answers with solutions");
    };
    let column = variables
        .iter()
        .position(|name| name == "s")
        .expect("?s is projected");
    let mut names: Vec<String> = rows
        .into_iter()
        .map(|row| match &row[column] {
            Some(TermValue::Iri(iri)) => iri.trim_start_matches(EX).to_owned(),
            other => panic!("?s binds an IRI, got {other:?}"),
        })
        .collect();
    names.sort();
    Ok(names)
}

#[test]
fn a_silent_service_honours_a_long_operator_chain_in_its_body() {
    // Unfiltered — the join identity a failed forward degrades to — would be all ten.
    assert_eq!(
        run(&query(true, 3)).expect("the silent service answers"),
        ["s0", "s1", "s2"]
    );
}

#[test]
fn the_same_body_without_silent_answers_the_same_rows() {
    // The valid neighbour: nothing is being swallowed, the forward itself succeeds.
    assert_eq!(
        run(&query(false, 3)).expect("the service answers"),
        ["s0", "s1", "s2"]
    );
}

#[test]
fn a_body_filter_that_excludes_everything_answers_no_rows() {
    // The control: the filter is evaluated, not dropped — a dropped (or swallowed) filter
    // would answer every subject here.
    assert_eq!(
        run(&query(true, 0)).expect("the silent service answers"),
        Vec::<String>::new()
    );
    assert_eq!(
        run(&query(false, 0)).expect("the service answers"),
        Vec::<String>::new()
    );
}

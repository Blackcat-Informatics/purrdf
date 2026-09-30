// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Evaluating a query with the text relations in scope, and rendering its
//! answer cells in an exact, unambiguous textual form.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_core::{RdfDataset, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};

/// One answer cell in an exact, unambiguous textual form.
pub fn render(cell: Option<&TermValue>) -> String {
    match cell {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(iri)) => format!("<{iri}>"),
        Some(TermValue::Blank { label, scope }) => format!("_:{label}/{}", scope.ordinal()),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => match language {
            Some(tag) => format!("{lexical_form:?}@{tag}"),
            None if datatype == "http://www.w3.org/2001/XMLSchema#string" => {
                format!("{lexical_form:?}")
            }
            None => format!("{lexical_form:?}^^<{datatype}>"),
        },
        Some(TermValue::Triple { s, p, o }) => format!(
            "<<{} {} {}>>",
            render(Some(s)),
            render(Some(p)),
            render(Some(o))
        ),
    }
}

/// The solution rows of `result`, rendered.
pub fn solutions(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions, got {result:?}");
    };
    rows.iter()
        .map(|row| row.iter().map(|cell| render(cell.as_ref())).collect())
        .collect()
}

/// Evaluate `query` against `dataset` with `relations` in scope, rendered.
pub fn answer(
    dataset: &RdfDataset,
    relations: &PropertyFunctionRegistry,
    query: &str,
) -> Vec<Vec<String>> {
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::new().with_env(
                &ExtensionEnv::over_relations(relations.clone())
                    .expect("the fixture declarations read cleanly"),
            ),
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    solutions(&result)
}

/// A typed literal cell as [`render`] writes it.
pub fn typed(lexical: &str, datatype: &str) -> String {
    format!("{lexical:?}^^<{datatype}>")
}

/// An IRI cell under the fixture namespace, as [`render`] writes it.
pub fn subject(local: &str) -> String {
    format!("<http://example.org/{local}>")
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A search needle need not be written as a constant: a `VALUES` row or a `BIND`
//! reaches the relation exactly as the constant does.
//!
//! [`TextSearchRelation`] serves only with its needle bound, so a needle the
//! planner failed to see bound is refused at prepare rather than answered wrongly —
//! and a needle it saw bound must answer with precisely the constant-needle rows.
//! Every comparison below is between two exact row vectors from two different
//! query texts, and the two-needle `VALUES` case is checked against the union of
//! its two single-needle answers, which differ from each other, so a needle that
//! was silently dropped or replaced cannot pass.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    ExtensionEnv, NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions,
};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The caller-supplied predicate the fixture host searches by.
const SEARCH: &str = "http://example.org/pf#search";

/// The one predicate whose literals the fixture indexes.
const NOTE: &str = "http://example.org/note";

/// Four documents: two hold `alpha beta`, one only `gamma`, one neither.
fn fixture() -> (Arc<RdfDataset>, PropertyFunctionRegistry) {
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text) in [
        ("d1", "alpha beta alpha beta"),
        ("d2", "alpha beta gamma delta"),
        ("d3", "gamma gamma river stone"),
        ("d4", "lazy dog sleeps late"),
    ] {
        let s = builder.intern_iri(&format!("http://example.org/{local}"));
        let o = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(s, note, o, None);
    }
    let dataset = builder.freeze().expect("the fixture must validate");
    let config = TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration names a predicate");
    let index = TextIndex::from_dataset(&*dataset, &config).expect("the fixture indexes");
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        SEARCH.to_owned(),
        Arc::new(TextSearchRelation::new(Arc::new(index))),
    );
    (dataset, registry)
}

fn render(cell: Option<&TermValue>) -> String {
    match cell {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(iri)) => format!("<{iri}>"),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => language.as_ref().map_or_else(
            || format!("{lexical_form:?}^^<{datatype}>"),
            |tag| format!("{lexical_form:?}@{tag}"),
        ),
        Some(other) => format!("{other:?}"),
    }
}

/// `SELECT ?candidate ?score ?rank ?lang ?matched WHERE { body }`, answered or
/// refused.
fn run(body: &str) -> Result<Vec<Vec<String>>, String> {
    let (dataset, registry) = fixture();
    let env = ExtensionEnv::over_relations(registry).expect("the fixture declarations read");
    let query = format!("SELECT ?candidate ?score ?rank ?lang ?matched WHERE {{ {body} }}");
    NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                env: &env,
                ..QueryOptions::EMPTY
            },
        )
        .map(|result| {
            let SparqlResult::Solutions { rows, .. } = result else {
                panic!("a SELECT answers with solutions");
            };
            let mut rendered: Vec<Vec<String>> = rows
                .iter()
                .map(|row| row.iter().map(|cell| render(cell.as_ref())).collect())
                .collect();
            rendered.sort();
            rendered
        })
        .map_err(|diagnostic| diagnostic.message)
}

/// The call with its needle argument spelled `needle`.
fn call(needle: &str) -> String {
    format!("?candidate <{SEARCH}> ( {needle} ?score ?rank ?lang ?matched )")
}

fn answered(body: &str) -> Vec<Vec<String>> {
    run(body).unwrap_or_else(|message| panic!("the query must evaluate: {message}"))
}

#[test]
fn a_values_or_bind_needle_answers_exactly_as_the_constant_does() {
    let constant = answered(&call("\"alpha beta\""));
    assert_eq!(
        constant
            .iter()
            .map(|row| row[0].clone())
            .collect::<Vec<_>>(),
        vec![
            "<http://example.org/d1>".to_owned(),
            "<http://example.org/d2>".to_owned()
        ],
        "the constant needle matches the two alpha-beta documents"
    );
    assert_eq!(
        answered(&format!("VALUES ?q {{ \"alpha beta\" }} {}", call("?q"))),
        constant
    );
    assert_eq!(
        answered(&format!("{} VALUES ?q {{ \"alpha beta\" }}", call("?q"))),
        constant
    );
    assert_eq!(
        answered(&format!("BIND(\"alpha beta\" AS ?q) {}", call("?q"))),
        constant
    );
}

#[test]
fn two_values_needles_answer_with_both_constant_answers() {
    let alpha_beta = answered(&call("\"alpha beta\""));
    let gamma = answered(&call("\"gamma\""));
    assert_ne!(alpha_beta, gamma, "the two needles must be told apart");
    let mut both: Vec<Vec<String>> = alpha_beta.into_iter().chain(gamma).collect();
    both.sort();
    assert_eq!(
        answered(&format!(
            "VALUES ?q {{ \"alpha beta\" \"gamma\" }} {}",
            call("?q")
        )),
        both
    );
}

/// A needle column holding `UNDEF` would reach the relation free on that row, which
/// it cannot serve: refused at prepare, and the same table without the `UNDEF` is
/// answered.
#[test]
fn an_undef_needle_is_refused_and_its_neighbour_answers() {
    let refused = run(&format!(
        "VALUES ?q {{ \"alpha beta\" UNDEF }} {}",
        call("?q")
    ))
    .expect_err("an UNDEF needle cannot be served");
    assert!(
        refused.contains("no feasible evaluation order") && refused.contains(SEARCH),
        "the refusal names the relation: {refused}"
    );
    assert_eq!(
        answered(&format!("VALUES ?q {{ \"alpha beta\" }} {}", call("?q"))),
        answered(&call("\"alpha beta\""))
    );
}

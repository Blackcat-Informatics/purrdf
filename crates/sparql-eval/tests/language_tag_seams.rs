// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The extension seams, through the one choke point they share.
//!
//! `STRLANG` asks the grammar and `RdfLiteral::validate_components` asks it on
//! the way into a dataset, but a host that injects Rust reaches neither: a
//! `NativeFunction`, a `ServiceResolver`, a `PropertyFunction` and a
//! `CustomAggregate` each hand the evaluator a whole `TermValue`, and the
//! `language` field of that value is a plain `Option<String>` with no constraint
//! on it anywhere in those four traits. The value then goes straight to the
//! results writers, which spell a tag as `"x"@<tag>` in TSV, as
//! `"xml:lang": "<tag>"` in JSON and as `xml:lang="<tag>"` in XML — bytes no
//! reader takes back.
//!
//! All four converge on `ScratchInterner::intern_checked`, the arena's door for
//! a caller-supplied `TermValue`, and that is where the grammar is asked. This
//! file drives the seam a host reaches most easily, a native function, from
//! real query text through the public engine — and it drives it with BOTH
//! halves, because a gate that refused `x-purrdf-afrikaans` or `en-fr-jura`
//! would be a worse bug than the one it closes.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{
    Arity, NativeSparqlEngine, QueryOptions, UserFunctionRegistry, Volatility,
};

/// The fixture namespace (AGENTS.md: fixtures live under `example.org`).
const EX: &str = "https://example.org/langtag#";

/// Tags real data carries, which the seam must keep binding.
const ACCEPTED: &[&str] = &[
    "en",
    "en-US",
    "zh-Hans-CN",
    "de-CH-x-phonebk",
    "i-enochian",
    "x-purrdf-afrikaans",
    "x-gmeow-english",
    "en-fr-jura",
    "fr-be-fbcl",
    "abcdefgh",
    "en-x-cantbethislong",
];

/// Tags no RDF concrete syntax would have lexed.
const REFUSED: &[&str] = &[
    "en us",
    "1",
    "9-9",
    "123-456",
    "en-",
    "-",
    "!!!",
    "abcdefghi",
];

/// A single-quad dataset, so the query below evaluates over exactly one row and
/// an unbound answer is distinguishable from no answer at all.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(&format!("{EX}s"));
    let p = b.intern_iri(&format!("{EX}p"));
    let o = b.intern_iri(&format!("{EX}o"));
    b.push_quad(s, p, o, None);
    b.freeze().expect("freeze the fixture")
}

/// Register `ex:tagged()` as a native function that returns
/// `"purr"@<tag>` — a `TermValue` the evaluator did not build and never
/// type-checked — then evaluate `SELECT (ex:tagged() AS ?v) WHERE { ?s ?p ?o }`
/// and report what `?v` was bound to.
fn tagged_by_a_native_function(tag: &str) -> Option<TermValue> {
    let value = TermValue::Literal {
        lexical_form: "purr".to_owned(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
        language: Some(tag.to_owned()),
        direction: None,
    };
    let mut functions = UserFunctionRegistry::new();
    functions.register_native(
        format!("{EX}tagged"),
        Arity::Exact(0),
        Volatility::Stable,
        Arc::new(move |_args: &[&TermValue]| Ok(Some(value.clone()))),
    );

    let dataset = dataset();
    let query = format!("SELECT (<{EX}tagged>() AS ?v) WHERE {{ ?s ?p ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &functions,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the query evaluates whatever the function returned");
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "the fixture always yields exactly one row");
    rows[0].first().cloned().flatten()
}

#[test]
fn a_native_function_cannot_bind_an_ungrammatical_language_tag() {
    for tag in REFUSED {
        assert_eq!(
            tagged_by_a_native_function(tag),
            None,
            "a native function returning {tag:?} must leave the expression unbound \
             (SPARQL 1.1 §17.2), never write it into a results row"
        );
    }
}

#[test]
fn a_native_function_still_binds_every_tag_real_data_carries() {
    for tag in ACCEPTED {
        let bound = tagged_by_a_native_function(tag);
        assert_eq!(
            bound,
            Some(TermValue::Literal {
                lexical_form: "purr".to_owned(),
                datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                language: Some((*tag).to_owned()),
                direction: None,
            }),
            "{tag} is a tag real data carries and must still bind, verbatim"
        );
    }
}

/// The row a refused value would have occupied is still a row: the solution is
/// one binding short, not missing, and every other variable keeps its value.
/// That is the difference between §17.2's unbound result and a dropped row.
#[test]
fn a_refused_tag_costs_the_binding_and_nothing_else() {
    let value = TermValue::Literal {
        lexical_form: "purr".to_owned(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
        language: Some("en us".to_owned()),
        direction: None,
    };
    let mut functions = UserFunctionRegistry::new();
    functions.register_native(
        format!("{EX}tagged"),
        Arity::Exact(0),
        Volatility::Stable,
        Arc::new(move |_args: &[&TermValue]| Ok(Some(value.clone()))),
    );

    let dataset = dataset();
    let query = format!("SELECT ?s (<{EX}tagged>() AS ?v) WHERE {{ ?s ?p ?o }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                functions: &functions,
                ..QueryOptions::EMPTY
            },
        )
        .expect("the query evaluates");
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("expected solutions");
    };
    assert_eq!(variables.len(), 2, "both projected variables are declared");
    assert_eq!(rows.len(), 1, "the row survives the refusal");
    assert_eq!(
        rows[0][0],
        Some(TermValue::Iri(format!("{EX}s"))),
        "?s keeps its binding"
    );
    assert_eq!(rows[0][1], None, "?v alone is unbound");
}

/// The other half of "the gate is the grammar, not a list": the verdict the
/// interner reaches is the verdict `purrdf_iri::langtag` reaches on the profile
/// every other stage names, over every tag both lists hold. Without this, the
/// two could drift apart in silence — a refusal is a claim, and this is the
/// claim's oracle.
#[test]
fn the_seam_verdict_is_the_grammars_verdict() {
    for tag in ACCEPTED.iter().chain(REFUSED) {
        let grammar = purrdf_iri::langtag::is_well_formed_with(
            tag,
            purrdf_iri::langtag::Profile::ConcreteSyntaxLangtagBounded,
        );
        assert_eq!(
            tagged_by_a_native_function(tag).is_some(),
            grammar,
            "the seam and the grammar must agree about {tag:?}"
        );
    }
}

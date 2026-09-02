// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Phrase and proximity, expressed in SPARQL, with no embedded query dialect.
//!
//! This crate mints no phrase operator, no slop parameter and no `NEAR`. The
//! executable claim of this file is that declining to invent one did not decline
//! the **capability**: a caller already has a language for stating a relationship
//! between two numbers, and [`TermOccurrenceRelation`] emits the numbers.
//!
//! Adjacency is `FILTER(?p2 = ?p1 + 1)`. Proximity within a window is
//! `FILTER(ABS(?p2 - ?p1) <= 3)`. Both are written against occurrence rows
//! joined on the document position, and both compose with ranked retrieval on
//! the same `?doc`, because a document subject is an ordinary term.
//!
//! As everywhere in this suite, the driving is query **text** through
//! [`NativeSparqlEngine`], the two relations are registered under **fixture**
//! IRIs supplied here by the host, and every assertion is an exact row vector.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf_text::{
    GraphSelector, TermOccurrenceRelation, TextIndex, TextIndexConfig, TextSearchRelation,
};

/// The caller-supplied predicate positional matching is called by.
const OCCURS: &str = "http://example.org/pf#occurs";

/// The caller-supplied predicate ranked retrieval is called by — a second IRI,
/// because these are two relations rather than one with a mode switch.
const SEARCH: &str = "http://example.org/pf#search";

/// The one predicate whose literals the fixture indexes.
const NOTE: &str = "http://example.org/note";

/// The datatype `?rank` comes back as.
const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

// ── rendering ────────────────────────────────────────────────────────────────

/// One answer cell in an exact, unambiguous textual form.
fn render(cell: Option<&TermValue>) -> String {
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

/// A typed literal cell as [`render`] writes it.
fn typed(lexical: &str, datatype: &str) -> String {
    format!("{lexical:?}^^<{datatype}>")
}

/// An IRI cell under the fixture namespace, as [`render`] writes it.
fn subject(local: &str) -> String {
    format!("<http://example.org/{local}>")
}

/// The solution rows of `result`, rendered.
fn solutions(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions, got {result:?}");
    };
    rows.iter()
        .map(|row| row.iter().map(|cell| render(cell.as_ref())).collect())
        .collect()
}

// ── the fixture ──────────────────────────────────────────────────────────────

/// Three documents holding both `quick` and `brown`, at three token distances.
///
/// Token positions run over the analyzed stream, so the tokenizer's own
/// segmentation decides them:
///
/// ```text
/// ex:far   "quick red panda and a brown bear"   quick @ 0, brown @ 5
/// ex:mid   "quick red panda brown bear"         quick @ 0, brown @ 3
/// ex:near  "the quick brown fox"                quick @ 1, brown @ 2
/// ```
///
/// All three would be retrieved by a disjunctive ranked search for
/// `"quick brown"`, and all three hold both terms, so `?matched` cannot separate
/// them either. Only the positions can — which is the point.
///
/// Document ids follow the subjects' canonical order, so `ex:far` is `0`,
/// `ex:mid` is `1` and `ex:near` is `2`, and that is the order occurrence rows
/// are emitted in.
fn corpus() -> (Arc<RdfDataset>, Arc<TextIndex>) {
    let rows: [(&str, &str); 3] = [
        ("far", "quick red panda and a brown bear"),
        ("mid", "quick red panda brown bear"),
        ("near", "the quick brown fox"),
    ];
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    for (local, text) in rows {
        let s = builder.intern_iri(&format!("http://example.org/{local}"));
        let o = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(s, note, o, None);
    }
    let dataset = builder.freeze().expect("the fixture must validate");
    let index = TextIndex::from_dataset(
        &*dataset,
        &TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
            .expect("the fixture configuration is well formed"),
    )
    .expect("the fixture indexes");
    (dataset, Arc::new(index))
}

/// Both relations over one index, under the two fixture IRIs.
fn registry(index: &Arc<TextIndex>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        OCCURS.to_owned(),
        Arc::new(TermOccurrenceRelation::new(Arc::clone(index))),
    );
    registry.register(
        SEARCH.to_owned(),
        Arc::new(TextSearchRelation::new(Arc::clone(index))),
    );
    registry
}

/// Evaluate `query` against `dataset` with `relations` in scope.
fn answer(
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
            QueryOptions {
                property_functions: relations,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    solutions(&result)
}

// ── the positions themselves ─────────────────────────────────────────────────

/// The raw material the two filters below are written against: one row per
/// occurrence, carrying the token ordinal.
///
/// Asserted first and exactly, so a phrase result cannot be right for the wrong
/// reason — if these ordinals were off, every filter over them would be too.
#[test]
fn occurrence_rows_carry_the_token_ordinal() {
    let (dataset, index) = corpus();
    let relations = registry(&index);
    for (term, expected) in [
        (
            "quick",
            vec![
                vec![subject("far"), typed("0", INTEGER)],
                vec![subject("mid"), typed("0", INTEGER)],
                vec![subject("near"), typed("1", INTEGER)],
            ],
        ),
        (
            "brown",
            vec![
                vec![subject("far"), typed("5", INTEGER)],
                vec![subject("mid"), typed("3", INTEGER)],
                vec![subject("near"), typed("2", INTEGER)],
            ],
        ),
    ] {
        assert_eq!(
            answer(
                &dataset,
                &relations,
                &format!("SELECT ?doc ?p WHERE {{ ?doc <{OCCURS}> ( \"{term}\" ?lang ?p ) }}"),
            ),
            expected,
            "the ordinals of {term:?}"
        );
    }
}

// ── phrase ───────────────────────────────────────────────────────────────────

/// A phrase is two occurrence calls joined on the document, under a filter
/// saying the second token immediately follows the first.
///
/// Repeating `?l` across both calls is load-bearing: it keeps the two
/// occurrences in the same **document**, rather than in two documents that
/// merely share a subject in different languages.
#[test]
fn adjacent_positions_express_a_phrase() {
    let (dataset, index) = corpus();
    let rows = answer(
        &dataset,
        &registry(&index),
        &format!(
            "SELECT ?doc WHERE {{ \
             ?doc <{OCCURS}> ( \"quick\" ?l ?p1 ) . \
             ?doc <{OCCURS}> ( \"brown\" ?l ?p2 ) . \
             FILTER(?p2 = ?p1 + 1) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![vec![subject("near")]],
        "only ex:near holds `quick brown` as a phrase; ex:mid and ex:far hold \
         both words at a distance, and a disjunctive ranked search cannot tell \
         the three apart"
    );
}

/// Proximity is the same two calls under a window filter, with no slop
/// parameter anywhere: the window is an arithmetic expression the caller wrote.
#[test]
fn proximity_within_a_window() {
    let (dataset, index) = corpus();
    let rows = answer(
        &dataset,
        &registry(&index),
        &format!(
            "SELECT ?doc WHERE {{ \
             ?doc <{OCCURS}> ( \"quick\" ?l ?p1 ) . \
             ?doc <{OCCURS}> ( \"brown\" ?l ?p2 ) . \
             FILTER(ABS(?p2 - ?p1) <= 3) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![vec![subject("mid")], vec![subject("near")]],
        "ex:mid is three tokens apart and inside the window; ex:far is five and \
         outside it"
    );
}

/// The two relations compose on `?doc`, so a phrase match can be ranked: this
/// is the shape a consumer wanting "phrase queries, ordered by relevance"
/// writes, assembled from parts rather than from a dialect.
#[test]
fn phrase_and_ranked_search_compose_on_the_same_document() {
    let (dataset, index) = corpus();
    let relations = registry(&index);

    // Ranked retrieval alone cannot express the phrase: all three documents
    // hold both terms, so all three are rows and all three report `?matched`
    // of two.
    assert_eq!(
        answer(
            &dataset,
            &relations,
            &format!(
                "SELECT ?doc ?rank ?matched WHERE {{ \
                 ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
            ),
        ),
        vec![
            vec![subject("near"), typed("1", INTEGER), typed("2", INTEGER)],
            vec![subject("mid"), typed("2", INTEGER), typed("2", INTEGER)],
            vec![subject("far"), typed("3", INTEGER), typed("2", INTEGER)],
        ],
        "the shortest document ranks first, since every term frequency is one \
         and only the length normalization separates them"
    );

    // The composition: the phrase narrows, the ranked relation scores what
    // survives, and the rank each row reports is its rank in the full corpus —
    // the join did not renumber anything.
    let rows = answer(
        &dataset,
        &relations,
        &format!(
            "SELECT ?doc ?rank WHERE {{ \
             ?doc <{OCCURS}> ( \"quick\" ?l ?p1 ) . \
             ?doc <{OCCURS}> ( \"brown\" ?l ?p2 ) . \
             FILTER(?p2 = ?p1 + 1) \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![vec![subject("near"), typed("1", INTEGER)]],
        "one document survives the phrase, and it carries its corpus rank"
    );
}

/// A multi-term needle at position 1 is refused rather than silently answered
/// about one of its terms — the contract that makes the phrase idiom above the
/// only way to write a phrase, instead of one of two ways that disagree.
#[test]
fn a_multi_term_needle_is_refused_rather_than_narrowed() {
    let (dataset, index) = corpus();
    let outcome = NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query: &format!("SELECT ?doc WHERE {{ ?doc <{OCCURS}> ( \"quick brown\" ?lang ?p ) }}"),
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions {
            property_functions: &registry(&index),
            ..QueryOptions::EMPTY
        },
    );
    let message = match outcome {
        Err(diagnostic) => diagnostic.message,
        Ok(result) => panic!(
            "a two-term needle must abort, not answer: {:?}",
            solutions(&result)
        ),
    };
    assert!(
        message.contains("ONE term per invocation"),
        "the diagnostic must say why, and point at the join idiom: {message}"
    );
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Ranked retrieval driven the way a host drives it: SPARQL query **text**
//! through [`NativeSparqlEngine`].
//!
//! Nothing here opens a cursor directly. The relation-level unit tests in
//! `src/relation.rs` already pin what a cursor emits; what those cannot show is
//! that the seam a host actually reaches through — parse, feasibility ordering,
//! registry resolution, evaluation, answers — lines up with it. A seam whose
//! halves only agree from inside is a seam no host can use.
//!
//! Every assertion is an **exact row vector**. Over-generation is as much a bug
//! as under-generation, so "at least these rows" would pass for a relation that
//! emitted the corpus twice.
//!
//! # PurRDF mints no vocabulary
//!
//! `http://example.org/pf#search` is a **fixture** IRI, supplied by this test in
//! its role as the host. It is not a default, and the crate contains no
//! namespace of its own — which is exactly what the three configuration cases at
//! the bottom of this file are here to demonstrate.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    NativeSparqlEngine, ParserOptions, PropertyFunctionRegistry, QueryOptions,
};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The caller-supplied predicate this host calls ranked retrieval by.
const SEARCH: &str = "http://example.org/pf#search";

/// The one predicate whose literals the fixture indexes.
const NOTE: &str = "http://example.org/note";

/// The datatype `?score` comes back as.
const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

/// The datatype `?rank` and `?matched` come back as.
const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

// ── rendering ────────────────────────────────────────────────────────────────

/// One answer cell in an exact, unambiguous textual form.
///
/// A plain `xsd:string` renders bare (`"text"`) and every other literal carries
/// its datatype or tag, so no two distinct terms can render alike.
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

// ── fixtures ─────────────────────────────────────────────────────────────────

/// A dataset of `(subject local name, predicate, text, language tag)` rows.
fn dataset_of(rows: &[(&str, &str, &str, Option<&str>)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for &(local, predicate, text, language) in rows {
        let s = builder.intern_iri(&format!("http://example.org/{local}"));
        let p = builder.intern_iri(predicate);
        let o = builder.intern_literal(match language {
            Some(tag) => RdfLiteral::language_tagged(text, tag),
            None => RdfLiteral::simple(text),
        });
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture must validate")
}

/// The configuration every fixture index is built under: one predicate, every
/// graph.
fn config_over(predicates: &[&str]) -> TextIndexConfig {
    TextIndexConfig::new(
        predicates.iter().map(|iri| TermValue::iri(*iri)).collect(),
        GraphSelector::Any,
    )
    .expect("the fixture configuration names at least one predicate")
}

/// A registry with `relation` registered under `iri` — the whole of what a host
/// has to do, since the engine derives parse-time recognition from the registry
/// itself.
fn registry_of(iri: &str, index: Arc<TextIndex>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(iri.to_owned(), Arc::new(TextSearchRelation::new(index)));
    registry
}

/// The hand-computed golden of this crate's scoring suite, respelled in the
/// vocabulary of a text search: four documents of four tokens each, so `avgdl`
/// is exactly four and every document's length normalization is exactly one.
///
/// ```text
/// d1  "quick quick brown fox"    tf(quick) = 2, tf(brown) = 1
/// d2  "quick brown fox jumps"    tf(quick) = 1, tf(brown) = 1
/// d3  "lazy dog sleeps late"
/// d4  "river stone bridge path"
/// ```
///
/// `N = 4`, the token total is `16`, `avgdl = 4`, and `df = 2` for both needle
/// terms, so this reproduces `tests/scoring.rs`'s worked arithmetic digit for
/// digit: `IDF = ln 2 = 0.693147180559`, the saturation is `1.375` at `tf = 2`
/// and `1` at `tf = 1`, and the two scores are
///
/// ```text
/// d1  0.693147180559 × 1.375 + 0.693147180559 × 1 = 1.646224553827
/// d2  0.693147180559 × 1     + 0.693147180559 × 1 = 1.386294361118
/// ```
///
/// `d3` and `d4` hold neither term, so they are not rows at all.
fn golden() -> (Arc<RdfDataset>, Arc<TextIndex>) {
    let dataset = dataset_of(&[
        ("d1", NOTE, "quick quick brown fox", None),
        ("d2", NOTE, "quick brown fox jumps", None),
        ("d3", NOTE, "lazy dog sleeps late", None),
        ("d4", NOTE, "river stone bridge path", None),
    ]);
    let index = TextIndex::from_dataset(&*dataset, &config_over(&[NOTE]))
        .expect("the golden fixture indexes");
    (dataset, Arc::new(index))
}

/// Five documents of four tokens each in one partition, engineered so the four
/// matching documents rank `1, 2, 3, 4` in subject order:
///
/// ```text
/// d1  "quick quick quick brown"   tf(quick) = 3, holds brown
/// d2  "quick quick brown fox"     tf(quick) = 2, holds brown
/// d3  "quick brown fox jumps"     tf(quick) = 1, holds brown
/// d4  "quick fox jumps high"      tf(quick) = 1, holds NO brown
/// d5  "lazy dog sleeps late"      holds neither
/// ```
///
/// Every document is four tokens, so length normalization is one throughout and
/// the order is decided entirely by term frequency and by whether the rarer
/// `brown` is present. `d4` is the conjunctive-retrieval control: it holds one
/// needle term, so it is a row with `?matched = 1`.
fn ranked() -> (Arc<RdfDataset>, Arc<TextIndex>) {
    let dataset = dataset_of(&[
        ("d1", NOTE, "quick quick quick brown", None),
        ("d2", NOTE, "quick quick brown fox", None),
        ("d3", NOTE, "quick brown fox jumps", None),
        ("d4", NOTE, "quick fox jumps high", None),
        ("d5", NOTE, "lazy dog sleeps late", None),
    ]);
    let index =
        TextIndex::from_dataset(&*dataset, &config_over(&[NOTE])).expect("the fixture indexes");
    (dataset, Arc::new(index))
}

// ── driving the engine ───────────────────────────────────────────────────────

/// Evaluate `query` against `dataset` with `registry` in scope, on a default
/// engine — no parser options, so the only reason the predicate is recognized is
/// the registration itself.
fn answer(
    dataset: &RdfDataset,
    registry: &PropertyFunctionRegistry,
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
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    solutions(&result)
}

/// Evaluate `query` on an engine configured with `options` — the success-side
/// counterpart of [`refusal`].
///
/// It exists so that a test which proves a parser-option configuration REFUSES
/// something can, in the same breath, prove that it still ANSWERS the thing it
/// is supposed to. Without it no test in this file ever evaluated a successful
/// query under non-default parser options, and a declaration that over-refused
/// every IRI beneath it — including registered ones — would have left the whole
/// suite green.
fn answer_with_options(
    dataset: &RdfDataset,
    registry: &PropertyFunctionRegistry,
    options: ParserOptions,
    query: &str,
) -> Vec<Vec<String>> {
    let result = NativeSparqlEngine::new()
        .with_parser_options(options)
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    solutions(&result)
}

/// The diagnostic message of a query that must NOT evaluate, on an engine
/// configured with `options`.
fn refusal(
    dataset: &RdfDataset,
    registry: &PropertyFunctionRegistry,
    options: ParserOptions,
    query: &str,
) -> String {
    let outcome = NativeSparqlEngine::new()
        .with_parser_options(options)
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: registry,
                ..QueryOptions::EMPTY
            },
        );
    match outcome {
        Err(diagnostic) => diagnostic.message,
        Ok(result) => panic!(
            "this must be a hard error, not an answer; got {:?}",
            solutions(&result)
        ),
    }
}

/// Parser options declaring nothing at all — the default a host that has
/// configured only a registry runs under.
fn nothing_declared() -> ParserOptions {
    ParserOptions {
        extension_fn_namespaces: Vec::new(),
        property_fn_namespaces: Vec::new(),
        property_fn_iris: Vec::new(),
    }
}

// ── the answer ───────────────────────────────────────────────────────────────

/// The whole row, through query text: subject, exact `xsd:decimal` score, rank,
/// language and match count.
#[test]
fn ranked_rows_for_a_needle() {
    let (dataset, index) = golden();
    let registry = registry_of(SEARCH, index);
    let rows = answer(
        &dataset,
        &registry,
        &format!(
            "SELECT ?doc ?score ?rank ?lang ?matched WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![
            vec![
                subject("d1"),
                typed("1.646224553827", DECIMAL),
                typed("1", INTEGER),
                "\"\"".to_owned(),
                typed("2", INTEGER),
            ],
            vec![
                subject("d2"),
                typed("1.386294361118", DECIMAL),
                typed("2", INTEGER),
                "\"\"".to_owned(),
                typed("2", INTEGER),
            ],
        ],
        "the two scores are the hand-computed golden, and the two documents \
         holding neither needle term are not rows at all"
    );
}

/// **The RDF 1.2 headline.** Text carried only by a statement annotation is
/// found through a SPARQL query, and the reifier — the annotation row's subject
/// — is what comes back at position 0.
///
/// The fixture is
///
/// ```text
/// ex:s ex:body "ledger entry filed under seal" .
/// ex:s ex:p ex:o {| ex:note "the quick brown fox" |}
/// ```
///
/// so the needle's text exists in exactly one place in the dataset: the
/// annotation. `quads_for_pattern` cannot see that row at all, which is why the
/// index reads both RDF 1.2 layers rather than one.
///
/// # The guard
///
/// The same needle is run first against an index configured over `ex:body`, a
/// predicate present **only** in the asserted layer. That answer must be empty.
/// Without it this test would pass for a relation that matched everything, or
/// for an index that had silently found the text somewhere else.
#[test]
fn annotation_text_is_searchable_through_sparql() {
    const BODY: &str = "http://example.org/body";
    const NEEDLE: &str = "quick brown";

    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("http://example.org/s");
    let p = builder.intern_iri("http://example.org/p");
    let o = builder.intern_iri("http://example.org/o");
    let reifier = builder.intern_iri("http://example.org/r1");
    let note = builder.intern_iri(NOTE);
    let body = builder.intern_iri(BODY);
    let asserted_text = builder.intern_literal(RdfLiteral::simple("ledger entry filed under seal"));
    let annotated_text = builder.intern_literal(RdfLiteral::simple("the quick brown fox"));

    builder.push_quad(s, p, o, None);
    builder.push_quad(s, body, asserted_text, None);
    let statement = builder.intern_triple(s, p, o);
    builder.push_reifier(reifier, statement);
    builder.push_annotation(reifier, note, annotated_text);
    let dataset = builder.freeze().expect("the fixture must validate");

    let query = format!(
        "SELECT ?doc WHERE {{ ?doc <{SEARCH}> ( \"{NEEDLE}\" ?score ?rank ?lang ?matched ) }}"
    );

    // The guard: over the asserted-layer-only predicate the same needle finds
    // nothing, so a passing answer below cannot be an answer about `ex:body`.
    let asserted_only = Arc::new(
        TextIndex::from_dataset(&*dataset, &config_over(&[BODY]))
            .expect("the asserted-layer index builds"),
    );
    assert_eq!(
        answer(&dataset, &registry_of(SEARCH, asserted_only), &query),
        Vec::<Vec<String>>::new(),
        "the asserted layer does not carry this text; if it did, the headline \
         assertion below would prove nothing about the annotation layer"
    );

    // The headline: over the annotation-only predicate the reifier is the row.
    let annotated = Arc::new(
        TextIndex::from_dataset(&*dataset, &config_over(&[NOTE]))
            .expect("the annotation-layer index builds"),
    );
    assert_eq!(
        answer(&dataset, &registry_of(SEARCH, annotated), &query),
        vec![vec![subject("r1")]],
        "in the annotation layer the row's subject IS the reifier, so the \
         reifier is the document the search retrieves"
    );
}

/// Ranks are per-partition, so filtering to one language cannot renumber a
/// surviving row: a document reports the same `?rank` with `?lang` bound and
/// free.
#[test]
fn a_bound_language_filters_without_changing_rank() {
    let dataset = dataset_of(&[
        ("e1", NOTE, "quick quick brown fox", Some("en")),
        ("e2", NOTE, "quick brown fox jumps", Some("en")),
        ("f1", NOTE, "quick brown chien noir", Some("fr")),
    ]);
    let index = Arc::new(
        TextIndex::from_dataset(&*dataset, &config_over(&[NOTE])).expect("the fixture indexes"),
    );
    let registry = registry_of(SEARCH, index);

    let free = answer(
        &dataset,
        &registry,
        &format!(
            "SELECT ?doc ?rank ?lang WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        free,
        vec![
            vec![subject("e1"), typed("1", INTEGER), "\"en\"".to_owned()],
            vec![subject("e2"), typed("2", INTEGER), "\"en\"".to_owned()],
            vec![subject("f1"), typed("1", INTEGER), "\"fr\"".to_owned()],
        ],
        "each partition numbers its own ranks from one, and partitions are \
         emitted in ascending key order"
    );

    let bound = answer(
        &dataset,
        &registry,
        &format!(
            "SELECT ?doc ?rank WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank \"en\" ?matched ) }}"
        ),
    );
    assert_eq!(
        bound,
        vec![
            vec![subject("e1"), typed("1", INTEGER)],
            vec![subject("e2"), typed("2", INTEGER)],
        ],
        "the French partition is gone and the English ranks are unchanged; a \
         global rank would have renumbered them"
    );
}

/// Binding position 0 to a document asks about that document, and gets its one
/// row.
#[test]
fn a_bound_doc_returns_its_single_row() {
    let (dataset, index) = golden();
    let rows = answer(
        &dataset,
        &registry_of(SEARCH, index),
        &format!(
            "SELECT ?score ?rank ?matched WHERE {{ \
             <http://example.org/d2> <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![vec![
            typed("1.386294361118", DECIMAL),
            typed("2", INTEGER),
            typed("2", INTEGER),
        ]],
        "one document, one partition, one row — and the rank is still the rank \
         it had in the full answer, not one"
    );
}

/// `VALUES` at `?rank` drives the relation's semantic pushdown: the ranker is
/// asked for those positions rather than for everything, and the answer is the
/// rows that hold them.
#[test]
fn values_rank_selects_those_ranks() {
    let (dataset, index) = ranked();
    let rows = answer(
        &dataset,
        &registry_of(SEARCH, index),
        &format!(
            "SELECT ?doc ?rank WHERE {{ VALUES ?rank {{ 1 2 }} \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        rows,
        vec![
            vec![subject("d1"), typed("1", INTEGER)],
            vec![subject("d2"), typed("2", INTEGER)],
        ],
        "ranks three and four exist in this corpus and are not asked for"
    );
}

/// `?matched` is how conjunctive retrieval is expressed: the relation is
/// disjunctive by construction — a document holding any needle term scores — and
/// a `FILTER` over the match count is what narrows it.
#[test]
fn matched_expresses_conjunctive_retrieval() {
    let (dataset, index) = ranked();
    let registry = registry_of(SEARCH, index);

    let unfiltered = answer(
        &dataset,
        &registry,
        &format!(
            "SELECT ?doc ?matched WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert_eq!(
        unfiltered,
        vec![
            vec![subject("d1"), typed("2", INTEGER)],
            vec![subject("d2"), typed("2", INTEGER)],
            vec![subject("d3"), typed("2", INTEGER)],
            vec![subject("d4"), typed("1", INTEGER)],
        ],
        "d4 holds `quick` and not `brown`, so it is a row that reports one"
    );

    let conjunctive = answer(
        &dataset,
        &registry,
        &format!(
            "SELECT ?doc WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) \
             FILTER(?matched = 2) }}"
        ),
    );
    assert_eq!(
        conjunctive,
        vec![
            vec![subject("d1")],
            vec![subject("d2")],
            vec![subject("d3")],
        ],
        "exactly the documents holding BOTH needle terms; d4 is excluded"
    );
}

/// `ORDER BY ?rank` is the reproducing idiom: within one partition it is
/// already the emission order, so it reorders nothing and states the order the
/// answer has.
///
/// `ORDER BY DESC(?score)` would not do this. Two documents can score exactly
/// equal — `tests/scoring.rs` pins such a pair — and a sort on a value that ties
/// leaves their relative order to the sort, while `?rank` is distinct within a
/// partition by construction.
#[test]
fn order_by_rank_is_the_reproducing_idiom() {
    let (dataset, index) = ranked();
    let registry = registry_of(SEARCH, index);
    let body = format!("?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched )");

    let emitted = answer(
        &dataset,
        &registry,
        &format!("SELECT ?doc ?rank WHERE {{ {body} }}"),
    );
    let ordered = answer(
        &dataset,
        &registry,
        &format!("SELECT ?doc ?rank WHERE {{ {body} }} ORDER BY ?rank"),
    );
    assert_eq!(
        emitted,
        vec![
            vec![subject("d1"), typed("1", INTEGER)],
            vec![subject("d2"), typed("2", INTEGER)],
            vec![subject("d3"), typed("3", INTEGER)],
            vec![subject("d4"), typed("4", INTEGER)],
        ]
    );
    assert_eq!(ordered, emitted, "the sort is a statement, not a change");
}

/// Position 0 is an ordinary term, so it joins back to the graph by basic graph
/// pattern — which is the whole reason retrieval is worth exposing as a relation
/// rather than as a separate query language.
#[test]
fn a_document_subject_joins_back_by_bgp() {
    let dataset = dataset_of(&[
        ("d1", NOTE, "quick quick brown fox", None),
        ("d2", NOTE, "quick brown fox jumps", None),
        ("d3", NOTE, "lazy dog sleeps late", None),
        ("d4", NOTE, "river stone bridge path", None),
        ("d2", "http://example.org/title", "The Second Note", None),
        ("d3", "http://example.org/title", "The Third Note", None),
    ]);
    let index = Arc::new(
        TextIndex::from_dataset(&*dataset, &config_over(&[NOTE])).expect("the fixture indexes"),
    );
    let rows = answer(
        &dataset,
        &registry_of(SEARCH, index),
        &format!(
            "SELECT ?doc ?title WHERE {{ \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) . \
             ?doc <http://example.org/title> ?title }}"
        ),
    );
    assert_eq!(
        rows,
        vec![vec![subject("d2"), "\"The Second Note\"".to_owned()]],
        "d1 retrieves but has no title, and d3 has a title but does not \
         retrieve; only the join has both"
    );
}

// ── configuration: no minted vocabulary, no silent degradation ───────────────

/// An IRI a host **declares** to the parser but never registers is a hard error
/// naming the IRI — never a zero-row `Ok`.
///
/// This is the reachable half of the pair. The other direction — registered but
/// absent from the parser options — cannot be constructed at all, because
/// `NativeSparqlEngine::prepare_for` derives the parser's exact-IRI set from the
/// registry's own keys, so a registered relation is recognized by construction.
#[test]
fn an_iri_declared_but_unregistered_is_a_hard_error() {
    let (dataset, _) = golden();
    let empty = PropertyFunctionRegistry::new();
    let message = refusal(
        &dataset,
        &empty,
        ParserOptions {
            property_fn_iris: vec![SEARCH.to_owned()],
            ..nothing_declared()
        },
        &format!(
            "SELECT ?doc WHERE {{ ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert!(
        message.contains(SEARCH),
        "the diagnostic must name the IRI the host declared: {message}"
    );
    assert!(
        message.contains("no property function is registered"),
        "and must say what is missing rather than answering nothing: {message}"
    );
}

/// A declared **namespace** with an unregistered IRI spelled under it is the
/// same hard error.
///
/// This is the documented reason a host declares a namespace at all when the
/// registry already makes its own keys recognized: so that spelling an
/// unregistered IRI in that namespace is a hard error rather than a silent data
/// triple that matches nothing.
#[test]
fn a_namespace_declared_with_an_unregistered_iri_is_a_hard_error() {
    const UNREGISTERED: &str = "http://example.org/pf#lookup";
    let (dataset, index) = golden();
    let registry = registry_of(SEARCH, index);
    let message = refusal(
        &dataset,
        &registry,
        ParserOptions {
            property_fn_namespaces: vec!["http://example.org/pf#".to_owned()],
            ..nothing_declared()
        },
        &format!(
            "SELECT ?doc WHERE {{ \
             ?doc <{UNREGISTERED}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert!(
        message.contains(UNREGISTERED),
        "the diagnostic must name the IRI actually spelled: {message}"
    );
    assert!(
        message.contains("no property function is registered"),
        "a declared namespace turns an unregistered spelling into a refusal: {message}"
    );

    // The neighbouring valid case, under the SAME declaration: the REGISTERED
    // IRI in that namespace must still answer. A declaration that refused every
    // IRI beneath it would satisfy the assertions above exactly as well as a
    // correct one does, and would break every host that declares a namespace.
    assert!(
        SEARCH.starts_with("http://example.org/pf#"),
        "the registered IRI must be inside the declared namespace, or this proves nothing"
    );
    assert_eq!(
        answer_with_options(
            &dataset,
            &registry,
            ParserOptions {
                property_fn_namespaces: vec!["http://example.org/pf#".to_owned()],
                ..nothing_declared()
            },
            &format!(
                "SELECT ?doc WHERE {{ \
                 ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
            ),
        ),
        vec![vec![subject("d1")], vec![subject("d2")]],
        "declaring the namespace must not disturb the registered IRI's own answer"
    );

    // And the same under an explicit IRI declaration rather than a namespace
    // one, which is the other configuration a host can write.
    assert_eq!(
        answer_with_options(
            &dataset,
            &registry,
            ParserOptions {
                property_fn_iris: vec![SEARCH.to_owned()],
                ..nothing_declared()
            },
            &format!(
                "SELECT ?doc WHERE {{ \
                 ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
            ),
        ),
        vec![vec![subject("d1")], vec![subject("d2")]]
    );
}

/// The contrapositive of "the seam never silently degrades": with the IRI in
/// neither the registry nor the parser options, the very same query text is an
/// ordinary triple pattern reading the graph.
///
/// The object is an RDF collection, so the pattern reads the graph for a list
/// the graph does not hold and answers with nothing — observably different from
/// the two rows the same text produces when the relation is registered.
#[test]
fn an_iri_in_neither_is_an_ordinary_triple_pattern() {
    let (dataset, index) = golden();
    let query = format!(
        "SELECT ?doc WHERE {{ ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}"
    );

    let unconfigured = answer(&dataset, &PropertyFunctionRegistry::new(), &query);
    assert_eq!(
        unconfigured,
        Vec::<Vec<String>>::new(),
        "with nothing configured this is data, and the data has no such list"
    );

    let configured = answer(&dataset, &registry_of(SEARCH, index), &query);
    assert_eq!(
        configured,
        vec![vec![subject("d1")], vec![subject("d2")]],
        "and the difference between the two is observable, which is what makes \
         the configuration real rather than decorative"
    );
}

/// A free needle has no feasible evaluation order: the relation retrieves
/// documents for a needle and cannot enumerate needles, so its only declared
/// mode demands position 1.
#[test]
fn a_free_needle_has_no_feasible_evaluation_order() {
    let (dataset, index) = golden();
    let registry = registry_of(SEARCH, index);
    let message = refusal(
        &dataset,
        &registry,
        nothing_declared(),
        &format!(
            "SELECT ?doc ?needle WHERE {{ \
             ?doc <{SEARCH}> ( ?needle ?score ?rank ?lang ?matched ) }}"
        ),
    );
    assert!(
        message.contains("no feasible evaluation order"),
        "got {message}"
    );
    assert!(message.contains(SEARCH), "got {message}");
    assert!(
        message.contains("fbffff"),
        "the diagnostic must name the declared mode, so a host can see which \
         position it has to bind: {message}"
    );
}

/// A call site of the wrong width is refused at prepare time, naming both the
/// declaration and what was written.
#[test]
fn an_arity_mismatch_at_the_call_site_is_an_error() {
    let (dataset, index) = golden();
    let registry = registry_of(SEARCH, index);
    let message = refusal(
        &dataset,
        &registry,
        nothing_declared(),
        &format!("SELECT ?doc WHERE {{ ?doc <{SEARCH}> ( \"quick brown\" ?score ) }}"),
    );
    assert!(
        message.contains("declared with 1 subject / 5 object"),
        "got {message}"
    );
    assert!(
        message.contains("supplies 1 subject / 2 object"),
        "got {message}"
    );
}

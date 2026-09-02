// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The crate's headline criterion, asserted where a consumer can see it: the
//! **serialized answer bytes**.
//!
//! Every test here drives SPARQL query text through [`NativeSparqlEngine`] and
//! then through `purrdf_sparql_results::to_json`, and compares whole documents
//! with `assert_eq!`. Comparing bytes rather than row structs is deliberate:
//! byte identity is the only claim that also covers the `xsd:decimal` lexical
//! forms, the row order, and the variable order at once, and it is the artefact
//! a downstream cache, diff or signature would actually key on.
//!
//! Two independent axes are pinned:
//!
//! * **across builds** — the same triples interned in different orders, so the
//!   dataset's `TermId`s genuinely differ, still answer byte for byte alike;
//! * **across runs** — the same index queried fifty times is fifty identical
//!   documents.
//!
//! And one **limit** is pinned rather than papered over: blank-node labels reach
//! the answer, so two datasets that differ only in them differ in their answers
//! too. This crate canonicalizes no blank nodes.

use std::sync::Arc;

use pretty_assertions::assert_eq;
use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf_sparql_results::{ResultProvenance, to_json};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The caller-supplied predicate this host calls ranked retrieval by.
const SEARCH: &str = "http://example.org/pf#search";

/// The one predicate whose literals the fixtures index.
const NOTE: &str = "http://example.org/note";

/// The one query text every test below drives.
const QUERY: &str = "SELECT ?doc ?score ?rank ?lang ?matched WHERE { \
                     ?doc <http://example.org/pf#search> \
                     ( \"quick brown\" ?score ?rank ?lang ?matched ) }";

/// The corpus every determinism test is built over — the hand-computed golden of
/// this crate's scoring suite, respelled as a text search.
///
/// Four documents of four tokens each, so `avgdl` is exactly four and every
/// document's length normalization is exactly one; `df` is two for both needle
/// terms, so the inverse document frequency is exactly `ln 2`.
const CORPUS: [(&str, &str); 4] = [
    ("d1", "quick quick brown fox"),
    ("d2", "quick brown fox jumps"),
    ("d3", "lazy dog sleeps late"),
    ("d4", "river stone bridge path"),
];

// ── fixtures ─────────────────────────────────────────────────────────────────

/// A dataset over `rows`, interned in the order given.
///
/// The insertion order decides the dataset's `TermId` assignment and nothing
/// else, which is exactly what the cross-build test needs to vary.
fn dataset_of(rows: &[(&str, &str)]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for &(local, text) in rows {
        let s = builder.intern_iri(&format!("http://example.org/{local}"));
        let p = builder.intern_iri(NOTE);
        let o = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("the fixture must validate")
}

/// The configuration every fixture index is built under.
fn config() -> TextIndexConfig {
    TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
        .expect("the fixture configuration is well formed")
}

/// An index over `dataset`.
fn index_of(dataset: &RdfDataset) -> Arc<TextIndex> {
    Arc::new(TextIndex::from_dataset(dataset, &config()).expect("the fixture indexes"))
}

/// A registry holding a ranked-retrieval relation over `index`, under the
/// fixture IRI.
fn registry_of(index: &Arc<TextIndex>) -> PropertyFunctionRegistry {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        SEARCH.to_owned(),
        Arc::new(TextSearchRelation::new(Arc::clone(index))),
    );
    registry
}

// ── driving the engine, all the way to bytes ─────────────────────────────────

/// Evaluate `query` and hand back the raw result.
fn evaluate(
    engine: &NativeSparqlEngine,
    dataset: &RdfDataset,
    relations: &PropertyFunctionRegistry,
    query: &str,
) -> SparqlResult {
    engine
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
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"))
}

/// The SPARQL Results JSON document for `query`, as a string — the artefact a
/// consumer receives.
fn srj(
    engine: &NativeSparqlEngine,
    dataset: &RdfDataset,
    relations: &PropertyFunctionRegistry,
    query: &str,
) -> String {
    let result = evaluate(engine, dataset, relations, query);
    let outcome = to_json(&result, &ResultProvenance::default(), None)
        .expect("a solution sequence of IRIs and literals serializes");
    String::from_utf8(outcome.bytes).expect("SRJ is UTF-8")
}

/// The `?score` lexical forms of `result`, in answer order.
fn score_lexicals(result: &SparqlResult) -> Vec<String> {
    let SparqlResult::Solutions {
        variables, rows, ..
    } = result
    else {
        panic!("a SELECT answers with solutions");
    };
    let at = variables
        .iter()
        .position(|name| name == "score")
        .expect("the query projects ?score");
    rows.iter()
        .map(|row| match row[at].as_ref() {
            Some(TermValue::Literal {
                lexical_form,
                datatype,
                ..
            }) => {
                assert_eq!(
                    datatype, "http://www.w3.org/2001/XMLSchema#decimal",
                    "a score is an xsd:decimal, never a double"
                );
                lexical_form.clone()
            }
            other => panic!("?score must be a bound literal, got {other:?}"),
        })
        .collect()
}

// ── across builds ────────────────────────────────────────────────────────────

/// Two datasets holding the same triples, interned in opposite orders, answer
/// **byte for byte** alike.
///
/// The two builds are kept genuinely independent: separate datasets, separate
/// indexes, separate registries, separate engines. The only thing they share is
/// the query text.
#[test]
fn byte_identical_json_across_two_independently_built_datasets() {
    let forward = dataset_of(&CORPUS);
    let mut reversed = CORPUS;
    reversed.reverse();
    let backward = dataset_of(&reversed);

    // The insertion orders really did produce different term tables — otherwise
    // this test would be comparing a dataset with itself.
    let probe = TermValue::iri("http://example.org/d1");
    assert_ne!(
        forward.term_id_by_value(&probe),
        backward.term_id_by_value(&probe),
        "the two builds must assign different TermIds, or there is nothing here \
         for determinism to survive"
    );

    let forward_index = index_of(&forward);
    let backward_index = index_of(&backward);
    assert_eq!(
        forward_index.fingerprint(),
        backward_index.fingerprint(),
        "the index fingerprint is a function of the content, not of the term \
         table it was read through"
    );

    let forward_json = srj(
        &NativeSparqlEngine::new(),
        &forward,
        &registry_of(&forward_index),
        QUERY,
    );
    let backward_json = srj(
        &NativeSparqlEngine::new(),
        &backward,
        &registry_of(&backward_index),
        QUERY,
    );
    assert_eq!(
        forward_json, backward_json,
        "the serialized answers must be identical documents"
    );
    assert!(
        forward_json.contains("1.646224553827"),
        "and must actually carry the golden score rather than being two empty \
         answers that agree: {forward_json}"
    );
}

// ── across runs ──────────────────────────────────────────────────────────────

/// Fifty evaluations of one query against one index produce fifty identical
/// documents.
///
/// One engine throughout, so the plan cache is exercised rather than avoided: a
/// cached plan that answered differently from a freshly built one would be the
/// same failure as an unstable ranking.
#[test]
fn byte_identical_json_across_repeated_runs() {
    let dataset = dataset_of(&CORPUS);
    let index = index_of(&dataset);
    let relations = registry_of(&index);
    let engine = NativeSparqlEngine::new();

    let first = srj(&engine, &dataset, &relations, QUERY);
    assert!(
        first.contains("1.646224553827"),
        "the fixture must answer with rows: {first}"
    );
    for run in 1..50 {
        assert_eq!(
            srj(&engine, &dataset, &relations, QUERY),
            first,
            "run {run} diverged"
        );
    }
}

// ── the golden values ────────────────────────────────────────────────────────

/// The scores, digit for digit, through query text.
///
/// These are the hand-computed values `tests/scoring.rs` derives from the
/// fixture's arithmetic: `IDF = ln 2 = 0.693147180559`, the length
/// normalization exactly one, and the saturation exactly `1.375` at `tf = 2`.
/// Pinning them here rather than only inside the crate is what makes a float
/// creeping into the pipeline, or a Unicode table shifting what a token is,
/// show up as a **value change a consumer would see** rather than as an
/// internal detail.
#[test]
fn golden_scores_are_exact_decimals() {
    let dataset = dataset_of(&CORPUS);
    let index = index_of(&dataset);
    let result = evaluate(
        &NativeSparqlEngine::new(),
        &dataset,
        &registry_of(&index),
        QUERY,
    );
    assert_eq!(
        score_lexicals(&result),
        vec!["1.646224553827".to_owned(), "1.386294361118".to_owned()],
        "0.693147180559 × 1.375 + 0.693147180559, and 0.693147180559 × 2"
    );
}

// ── the documented limit ─────────────────────────────────────────────────────

/// The adversarial caveat, asserted rather than assumed away: **blank-node
/// labels reach the answer**.
///
/// Two datasets identical in every respect except the label of the blank node
/// carrying the text produce different indexes and different answers. This crate
/// canonicalizes no blank nodes — a document's identity is its subject term, and
/// for a blank subject that term is `(label, scope)`.
///
/// A caller needing label-independent answers canonicalizes upstream, before the
/// index is built. Pinning the limit here means it changes visibly if it ever
/// changes at all.
#[test]
fn blank_node_labels_are_the_documented_limit() {
    /// One blank subject, labelled `label`, carrying the needle's text.
    fn labelled(label: &str) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_blank(label, BlankScope::DEFAULT);
        let p = builder.intern_iri(NOTE);
        let o = builder.intern_literal(RdfLiteral::simple("quick brown fox"));
        builder.push_quad(s, p, o, None);
        builder.freeze().expect("the fixture must validate")
    }

    let first = labelled("b0");
    let second = labelled("b1");
    let first_index = index_of(&first);
    let second_index = index_of(&second);

    assert_ne!(
        first_index.fingerprint(),
        second_index.fingerprint(),
        "the subject term is part of the index's identity, and these subjects \
         are different terms"
    );

    let first_json = srj(
        &NativeSparqlEngine::new(),
        &first,
        &registry_of(&first_index),
        QUERY,
    );
    let second_json = srj(
        &NativeSparqlEngine::new(),
        &second,
        &registry_of(&second_index),
        QUERY,
    );
    assert_ne!(
        first_json, second_json,
        "two isomorphic datasets are NOT interchangeable here"
    );

    // And the difference is exactly the label, not something incidental: the
    // scores are identical, because the text and the corpus statistics are.
    assert_eq!(
        score_lexicals(&evaluate(
            &NativeSparqlEngine::new(),
            &first,
            &registry_of(&first_index),
            QUERY,
        )),
        score_lexicals(&evaluate(
            &NativeSparqlEngine::new(),
            &second,
            &registry_of(&second_index),
            QUERY,
        )),
        "the ranking is label-independent; only the identity of the retrieved \
         document is not"
    );
    assert!(
        first_json.contains("b0") && second_json.contains("b1"),
        "the label itself is what reaches the answer: {first_json} / {second_json}"
    );
}

// ── the no-float invariant, at the source ────────────────────────────────────

/// Every line of `text` that mentions a float width, as `(line number, the
/// line)` — once per width, since a gate cares that a line offends, not how
/// many times.
///
/// A **token** scan, not a substring one, and the two boundaries are deliberately
/// asymmetric:
///
/// * the character **after** must not continue an identifier, so `f64x` and
///   `f32_of` are not float mentions;
/// * the character **before** must not be a letter or `_`, so `buf64` and
///   `my_f64_helper` are not either — but a **digit** may precede, because that
///   is exactly how a suffixed float literal is spelled (`1.0f64`, `3f32`), and
///   those are the mentions this gate most needs to see.
///
/// Erring toward admission on the identifier cases is the safe direction: the
/// crate-root `deny(clippy::float_arithmetic)` still refuses any arithmetic
/// performed on whatever such a name holds. This scan exists for what that lint
/// cannot see.
fn float_mentions(text: &str) -> Vec<(usize, String)> {
    /// Is `c` a character that would continue an identifier to the right?
    fn continues_right(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Is `c` a character that would continue an identifier to the left,
    /// **excluding** digits — a digit before `f64` is a literal suffix.
    fn continues_left(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }

    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        for width in ["f32", "f64"] {
            let mut from = 0;
            while let Some(offset) = line.get(from..).and_then(|rest| rest.find(width)) {
                let at = from + offset;
                let before = line[..at].chars().next_back();
                let after = line[at + width.len()..].chars().next();
                if !before.is_some_and(continues_left) && !after.is_some_and(continues_right) {
                    found.push((number + 1, line.trim().to_owned()));
                    break;
                }
                from = at + width.len();
            }
        }
    }
    found
}

/// The scan discriminates rather than merely refusing.
///
/// The repository's own rule about refusals: run the case that must be refused
/// **and** the neighbouring one that must be admitted. A gate that flagged every
/// line containing the letters `f64` would pass a test that only checked the
/// first half, and would then reject the crate's own prose the moment someone
/// wrote about a float in a doc comment.
#[test]
fn the_float_scan_refuses_floats_and_admits_their_neighbours() {
    // Refused: a type annotation, a cast, a suffixed literal, a return type.
    for refused in [
        "let x: f64 = 1.0;",
        "let y = count as f64;",
        "let z = 1.0f64;",
        "let w = 3f32;",
        "fn average(&self) -> f64 {",
        "    idf: f32,",
    ] {
        assert_eq!(
            float_mentions(refused).len(),
            1,
            "must be seen as a float mention: {refused}"
        );
    }

    // Admitted: the same letters inside an identifier, and in prose. These are
    // the neighbours an over-broad substring scan would wrongly refuse.
    for admitted in [
        "let buf64 = [0u8; 64];",
        "let my_f64_helper = 1;",
        "/// No floating-point value enters this crate.",
        "//! `Fixed::ln` replaces a libm ln, which may differ by an ulp.",
        "let f64x = 1;",
        "struct Off32Bit;",
    ] {
        assert_eq!(
            float_mentions(admitted),
            Vec::new(),
            "must NOT be seen as a float mention: {admitted}"
        );
    }
}

/// **No floating point reaches this crate's sources.**
///
/// `#![deny(clippy::float_arithmetic)]` at the crate root refuses `+ - * /` on a
/// float, and that is the enforcement the crate documentation cites. It is not
/// the whole of the claim: the lint says nothing about `as f64`, about a float
/// **comparison**, or about a float **method** — and `f64::ln` is a method. A
/// single `x.ln()` on an inferred `f64` would restore exactly the native/wasm
/// answer divergence [`purrdf_text::Fixed::ln`] exists to make impossible, and
/// would do it without tripping a single lint.
///
/// So the second half of the enforcement is this scan over the crate's own
/// sources — including the `#[cfg(test)]` modules inside them, because a float
/// reached for in a test is a float that compiled.
#[test]
fn no_source_file_mentions_a_float_width() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut scanned = 0_usize;
    let mut offences = Vec::new();

    let entries = std::fs::read_dir(&src).expect("the crate's src directory is readable");
    for entry in entries {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable Rust source file");
        scanned += 1;
        for (line, content) in float_mentions(&text) {
            offences.push(format!("{}:{line}: {content}", path.display()));
        }
    }

    // The scan is only a gate if it actually read the sources; a rename that
    // emptied the directory would otherwise pass silently.
    assert_eq!(
        scanned, 8,
        "expected to scan every module of the crate, scanned {scanned}"
    );
    assert_eq!(
        offences,
        Vec::<String>::new(),
        "this crate is exact fixed point on every target; a float width in its \
         sources breaks native/wasm answer identity"
    );
}

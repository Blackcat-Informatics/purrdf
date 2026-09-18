// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Structural refusals are decided before the first byte, and each one is paired
//! with a neighbouring input that must still succeed.
//!
//! Three writers used to decide a structural refusal only after emitting part of
//! the document — SRX after its declaration and root element, TSV after earlier
//! header fields, CSV and TSV after earlier rows. Eagerly the partial string was
//! discarded, so nothing was observable; against an incremental sink those bytes
//! have already gone downstream and cannot be recalled.
//!
//! Hoisting a check is exactly the change that silently produces over-refusal —
//! a scan that runs earlier, over more of the input, can reject something the
//! interleaved check never reached. So every refusal below is executed twice: the
//! input believed invalid, and a neighbour that differs in the one respect that
//! is supposed to matter. A refusal that cannot be shown to spare its neighbour
//! is not strictness, it is a bug that happens to look like one.

use purrdf_core::{RdfDatasetBuilder, TermValue};
use purrdf_sparql_results::{ResultProvenance, SparqlResult, to_csv, to_tsv, to_xml};

fn solutions(variables: &[&str], rows: Vec<Vec<Option<TermValue>>>) -> SparqlResult {
    SparqlResult::Solutions {
        variables: variables.iter().map(|v| (*v).to_owned()).collect(),
        rows,
        aux: RdfDatasetBuilder::new().freeze().expect("empty aux dataset"),
    }
}

fn iri(value: &str) -> Option<TermValue> {
    Some(TermValue::Iri(value.to_owned()))
}

/// A tab in a variable name has no TSV spelling and is refused — but a name that
/// merely contains other punctuation is fine, and the hoisted whole-header scan
/// must not have started rejecting those.
#[test]
fn tsv_refuses_an_unescapable_variable_name_and_spares_its_neighbour() {
    let provenance = ResultProvenance::default();

    for bad in ["has\ttab", "has\nlf", "has\rcr"] {
        let result = solutions(&[bad], vec![vec![iri("https://example.org/a")]]);
        let error = to_tsv(&result, &provenance)
            .expect_err("a variable name TSV cannot escape must be refused");
        let rendered = error.to_string();
        assert!(
            rendered.contains("has no way to escape"),
            "unexpected refusal wording for {bad:?}: {rendered}"
        );
    }

    // The neighbour: names carrying every other character class the scan sees.
    for good in ["plain", "with_underscore", "with.dot", "\u{4e2d}\u{6587}", "s1"] {
        let result = solutions(&[good], vec![vec![iri("https://example.org/a")]]);
        let outcome = to_tsv(&result, &provenance)
            .unwrap_or_else(|e| panic!("variable name {good:?} is representable in TSV: {e}"));
        let text = String::from_utf8(outcome.bytes).expect("utf-8");
        assert!(text.starts_with(&format!("?{good}\n")), "header for {good:?}");
    }
}

/// A row carrying more bindings than the projection has variables is refused by
/// both tabular writers — and a row carrying exactly as many, or fewer, is not.
#[test]
fn tabular_writers_refuse_an_over_wide_row_and_spare_the_exact_and_short_ones() {
    let provenance = ResultProvenance::default();
    let vars = ["a", "b"];

    let over_wide = solutions(
        &vars,
        vec![vec![
            iri("https://example.org/1"),
            iri("https://example.org/2"),
            iri("https://example.org/3"),
        ]],
    );
    for rendered in [
        to_csv(&over_wide, &provenance)
            .expect_err("CSV must refuse an over-wide row")
            .to_string(),
        to_tsv(&over_wide, &provenance)
            .expect_err("TSV must refuse an over-wide row")
            .to_string(),
    ] {
        assert!(
            rendered.contains("3 bindings but only 2 variables"),
            "unexpected refusal wording: {rendered}"
        );
    }

    // The neighbours: exactly-wide, short, and empty rows all remain valid. The
    // boundary is `>`, and a scan that had drifted to `>=` would fail here.
    for rows in [
        vec![vec![iri("https://example.org/1"), iri("https://example.org/2")]],
        vec![vec![iri("https://example.org/1")]],
        vec![vec![]],
        vec![],
    ] {
        let result = solutions(&vars, rows);
        to_csv(&result, &provenance).expect("CSV admits a row no wider than the projection");
        to_tsv(&result, &provenance).expect("TSV admits a row no wider than the projection");
    }
}

/// The first offending row is reported, not merely some offending row — the
/// hoisted scan walks rows in the same order the emission loop did.
#[test]
fn the_first_over_wide_row_is_the_one_reported() {
    let provenance = ResultProvenance::default();
    let result = solutions(
        &["a"],
        vec![
            vec![iri("https://example.org/ok")],
            vec![iri("https://example.org/1"), iri("https://example.org/2")],
            vec![
                iri("https://example.org/1"),
                iri("https://example.org/2"),
                iri("https://example.org/3"),
            ],
        ],
    );
    let rendered = to_csv(&result, &provenance)
        .expect_err("an over-wide row must be refused")
        .to_string();
    assert!(
        rendered.contains("2 bindings but only 1 variables"),
        "the SECOND row is the first offender; got: {rendered}"
    );
}

/// SRX is undefined for CONSTRUCT graphs and refuses one — but still serializes
/// the two result kinds it does define, which the hoisted check sits in front of.
#[test]
fn srx_refuses_a_graph_and_spares_the_kinds_it_defines() {
    let provenance = ResultProvenance::default();

    let graph = SparqlResult::Graph(
        RdfDatasetBuilder::new()
            .freeze()
            .expect("an empty dataset freezes"),
    );
    let rendered = to_xml(&graph, &provenance, None)
        .expect_err("SRX must refuse a CONSTRUCT graph")
        .to_string();
    assert!(
        rendered.contains("undefined for CONSTRUCT graphs"),
        "unexpected refusal wording: {rendered}"
    );

    // The neighbours, on the far side of the hoisted check.
    let ask = SparqlResult::Boolean(true);
    let ask_text =
        String::from_utf8(to_xml(&ask, &provenance, None).expect("ASK is defined in SRX").bytes)
            .expect("utf-8");
    assert!(ask_text.contains("<boolean>true</boolean>"));
    assert!(ask_text.starts_with("<?xml version=\"1.0\"?>\n"));

    let select = solutions(&["a"], vec![vec![iri("https://example.org/1")]]);
    let select_text = String::from_utf8(
        to_xml(&select, &provenance, None)
            .expect("SELECT is defined in SRX")
            .bytes,
    )
    .expect("utf-8");
    assert!(select_text.contains("<uri>https://example.org/1</uri>"));
}

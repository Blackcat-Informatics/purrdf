// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A constant operand of a `FILTER` is parsed once per query, not once per row.
//!
//! The count is of lexical-form parses ([`crate::op_count::parses`]), never of time. A
//! constant the program interned into the scratch table is not a dataset term, so the
//! dataset parse cache never held it and every row parsed it again. These tests hold
//! the parse count of a query constant as the row count grows, and hold the
//! neighbouring case, an operand computed from the row, to its per-row evaluation.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral};

use crate::engine::NativeSparqlEngine;
use crate::eval::EvalOptions;

const EX: &str = "http://example.org/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// `rows` subjects, each with the same `ex:v 7` value, so the dataset side parses once
/// and any growth in the count with `rows` is the operand's own.
fn same_value(rows: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{EX}v")).expect("fixture capacity");
    let seven = b
        .intern_literal(RdfLiteral::typed("7", XSD_INTEGER.to_owned()))
        .expect("fixture capacity");
    for i in 0..rows {
        let s = b
            .intern_iri(&format!("{EX}s{i}"))
            .expect("fixture capacity");
        b.push_quad(s, p, seven, None).expect("fixture capacity");
    }
    b.freeze().expect("freeze")
}

/// The rows `filter` keeps over `rows` subjects, and the parses the evaluation counted.
fn parses_of(rows: usize, filter: &str) -> (usize, u64) {
    let engine = NativeSparqlEngine::default().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    });
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}v> ?v FILTER({filter}) }}");
    let prepared = engine.prepare_query(&query, None).expect("prepare");
    let dataset = same_value(rows);
    crate::op_count::reset();
    let result = engine
        .query_prepared(&dataset, &prepared, &[], &crate::QueryOptions::EMPTY)
        .expect("evaluate");
    let parses = crate::op_count::parses();
    let kept = match result {
        purrdf_core::SparqlResult::Solutions { rows, .. } => rows.len(),
        other => panic!("a SELECT answers solutions, got {other:?}"),
    };
    (kept, parses)
}

/// **A constant operand costs one parse per query, however many rows compare against it.**
#[test]
fn constant_operand_is_parsed_once_per_query() {
    let (few_kept, few) = parses_of(4, "?v > 5.5e0");
    let (many_kept, many) = parses_of(400, "?v > 5.5e0");
    assert_eq!(
        (few_kept, many_kept),
        (4, 400),
        "the filter keeps every row"
    );
    assert_eq!(
        few, many,
        "the constant must not be parsed again for each of the extra rows"
    );
    assert_eq!(
        many, 4,
        "the dataset value and the boolean the comparison yields once each, and the \
         constant twice: on the first row and on the second, which stores it"
    );
}

/// **A constant whose comparison keeps no row is parsed once too**: the refusal is the
/// neighbour of the acceptance above, and the answer is unchanged by the memo.
#[test]
fn constant_operand_memo_keeps_the_answer() {
    let (kept, parses) = parses_of(400, "?v > 8.5e0");
    assert_eq!(kept, 0);
    assert_eq!(parses, 4);
}

/// **An operand computed from the row is still evaluated per row.** `?v + 1` is not a
/// constant of the program, so it is not memoized; the same `?v + 1 > 7.5e0` shape over
/// more rows must parse more.
#[test]
fn row_dependent_operand_is_still_parsed_per_row() {
    let (_, few) = parses_of(4, "?v + 1 > 7.5e0");
    let (kept, many) = parses_of(400, "?v + 1 > 7.5e0");
    assert_eq!(kept, 400);
    assert!(
        many >= few + 396,
        "a computed operand must be parsed for every extra row: {few} then {many}"
    );
}

/// **The memo never changes an answer**, past its inline slots and across constants
/// that share a lexical form. Nine distinct constants overflow the eight slots; the
/// ninth, parsed per use, must still decide the answer in both directions. A boolean
/// literal `"7"` shares its lexical form with the integer `7` but is not a boolean
/// value, so it must not borrow the integer's parse: its effective boolean value is
/// an error and drops the row, while `"1"` keeps it.
#[test]
fn memo_answers_match_past_the_slots_and_across_datatypes() {
    let nine = |last: &str| {
        let mut parts: Vec<String> = (1..=8).map(|n| format!("?v > 0.{n}e0")).collect();
        parts.push(format!("?v > {last}"));
        parts.join(" && ")
    };
    assert_eq!(
        parses_of(40, &nine("6.5e0")).0,
        40,
        "the ninth constant holds"
    );
    assert_eq!(
        parses_of(40, &nine("8.5e0")).0,
        0,
        "the ninth constant refuses"
    );
    let xsd_boolean = "http://www.w3.org/2001/XMLSchema#boolean";
    assert_eq!(
        parses_of(40, &format!("?v = 7 && \"7\"^^<{xsd_boolean}>")).0,
        0,
        "an ill-formed boolean must not reuse the integer 7 parse"
    );
    assert_eq!(
        parses_of(40, &format!("?v = 7 && \"1\"^^<{xsd_boolean}>")).0,
        40
    );
}

/// **A frame that grows makes nested `EXISTS` refuse on a small stack.** The evaluation
/// context sits in every nesting level's stack frame, and the parse memo is a field of
/// it, so the context must stay no larger than it was before the memo existed. The
/// bound is that size, 2448 bytes on a 64-bit target, with the memo's registry a few inline ids.
#[test]
fn the_parse_memo_does_not_enlarge_the_evaluation_frame() {
    if size_of::<usize>() == 8 {
        assert!(
            size_of::<crate::eval::EvalCtx<'static, RdfDataset>>() <= 2448,
            "the evaluation context grew: nested EXISTS would refuse on small stacks"
        );
    }
}

/// **Two computed constants with one lexical form and different datatypes do not share
/// a parse.** Neither `9` literal is in the dataset, so both are scratch constants.
#[test]
fn same_lexical_form_with_different_datatypes_keeps_each_parse() {
    let decimal = "http://www.w3.org/2001/XMLSchema#decimal";
    let filter = format!(
        "?v < \"9\"^^<{XSD_INTEGER}> && ?v < \"9\"^^<{decimal}> && \"9.5\"^^<{decimal}> > \"9\"^^<{XSD_INTEGER}>"
    );
    assert_eq!(parses_of(40, &filter).0, 40);
    let wrong =
        format!("?v < \"9\"^^<{XSD_INTEGER}> && \"9.5\"^^<{decimal}> < \"9\"^^<{XSD_INTEGER}>");
    assert_eq!(parses_of(40, &wrong).0, 0);
}

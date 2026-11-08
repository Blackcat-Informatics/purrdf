// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The one grader for W3C SHACL 1.2 `sht:EvalNodeExpr` entries**, shared by every
//! harness that runs them — the library harness (`w3c12_conformance.rs`) and the
//! command-line harness in `purrdf-cli`, which runs the same entries through the built
//! binary — so the two cannot mean different things by "the output agrees".
//!
//! The output must equal the `mf:result` list TERM FOR TERM (RDF 1.2 term equality:
//! lexical form, datatype, language and direction), in order unless the entry sets
//! `sht:ignoreOrder true`, in which case as a multiset ([`compare_outputs`]). There is
//! no other departure from exact equality. An `mf:result sht:Failure` requires a
//! typed evaluation failure; setup errors, processor failures and declaration
//! refusals cannot satisfy it.

use purrdf_shapes::data::{GraphFilter, native_quads};
use purrdf_shapes::free_expression::{self, FreeExpression};
use purrdf_shapes::term::Term;
use purrdf_shapes::{FailureClass, ShapesError};

use crate::shacl12::{NodeExprCase, NodeExprExpected};

/// Resolve an entry's focus, checking that an absent-focus sentinel cannot
/// accidentally name a node of the fixture.
pub fn focus(tc: &NodeExprCase) -> Result<Term, String> {
    if let Some(focus) = &tc.focus {
        return Ok(focus.clone());
    }
    let absent = Term::blank(ABSENT_FOCUS);
    let mentioned = !native_quads(
        &tc.dataset,
        Some(&absent),
        None,
        None,
        GraphFilter::AnyGraph,
    )
    .is_empty()
        || !native_quads(
            &tc.dataset,
            None,
            None,
            Some(&absent),
            GraphFilter::AnyGraph,
        )
        .is_empty();
    if mentioned {
        return Err(format!("the test graph mentions the harness's {absent}"));
    }
    Ok(absent)
}

/// Evaluate through the same standalone library entry point the CLI uses.
/// The outer result reports fixture/setup failures; the inner result retains
/// the engine's typed outcome so only an evaluation failure can satisfy `sht:Failure`.
pub fn evaluate_case(
    tc: &NodeExprCase,
    profile: purrdf_shapes::ShaclProfile,
) -> Result<Result<Vec<Term>, ShapesError>, String> {
    let text = std::fs::read_to_string(&tc.file)
        .map_err(|error| format!("cannot read {}: {error}", tc.file.display()))?;
    let prefixes =
        purrdf_shapes::text_ingest::parse_turtle_document(&text, Some(&crate::file_iri(&tc.file)))
            .map_err(|error| format!("node-expression graph parse error: {error}"))?
            .prefixes;
    let focus = focus(tc)?;
    let imports = purrdf_shapes::ShapesImports::new();
    let request = FreeExpression {
        shapes: &tc.dataset,
        prefixes: &prefixes,
        root: &tc.expr,
        data: &tc.dataset,
        focus: &focus,
        scope: &tc.scope,
        imports: &imports,
    };
    match free_expression::evaluate_with_profile(&request, profile) {
        Ok(result) => Ok(Ok(result.outputs)),
        Err(purrdf_shapes::xpath::XPathValidationError::Shapes(error)) => Ok(Err(error)),
        Err(error) => Err(format!(
            "selected-law evaluation could not complete: {error}"
        )),
    }
}

/// Grade exact output or a declared semantic evaluation failure. A processor
/// failure, declaration refusal or successful empty output is not an evaluation failure.
pub fn grade_evaluation(
    produced: Result<Vec<Term>, ShapesError>,
    expected: &NodeExprExpected,
    ignore_order: bool,
) -> Result<(), String> {
    match (produced, expected) {
        (Err(error), NodeExprExpected::Failure)
            if error.failure_class() == FailureClass::Evaluation =>
        {
            Ok(())
        }
        (Err(error), _) => Err(error.to_string()),
        (Ok(_), NodeExprExpected::Failure) => {
            Err("expected an evaluation failure, got output".to_owned())
        }
        (Ok(produced), NodeExprExpected::Terms(expected)) => {
            compare_outputs(&produced, expected, ignore_order)
        }
    }
}

/// Compare produced node-expression output against the expected list: exact RDF
/// 1.2 term equality, in order, or as a multiset under `ignore_order`.
pub fn compare_outputs(
    produced: &[Term],
    expected: &[Term],
    ignore_order: bool,
) -> Result<(), String> {
    let agree = if ignore_order {
        let mut p: Vec<&Term> = produced.iter().collect();
        let mut e: Vec<&Term> = expected.iter().collect();
        p.sort_by_key(ToString::to_string);
        e.sort_by_key(ToString::to_string);
        p == e
    } else {
        produced == expected
    };
    if agree {
        return Ok(());
    }
    let render = |terms: &[Term]| {
        terms
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    };
    Err(format!(
        "node-expression output mismatch{}:\n  produced ( {} )\n  expected ( {} )",
        if ignore_order { " (order ignored)" } else { "" },
        render(produced),
        render(expected)
    ))
}

/// The focus node of an entry that gives no `sht:focusNode`: a blank node each harness
/// proves occurs nowhere in the test graph (see `w3c12_conformance.rs`'s module docs).
pub const ABSENT_FOCUS: &str = "shacl12-harness-absent-focus-node";

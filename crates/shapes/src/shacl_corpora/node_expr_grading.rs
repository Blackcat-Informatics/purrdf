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
//! no other departure from exact equality: an approved entry is graded against its
//! `mf:result` as written.

use crate::term::Term;

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

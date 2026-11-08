// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same transitive rule fixture in native query tests and evaluator benchmarks.

use purrdf_datalog::clause::{ClauseAtom, DlClause};

pub(crate) fn closure_rules<const N: usize>(
    base: &str,
    closure: &str,
    atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
) -> Vec<DlClause<N>> {
    vec![
        DlClause::datalog(atom("?s", closure, "?o"), vec![atom("?s", base, "?o")]),
        DlClause::datalog(
            atom("?s", closure, "?o"),
            vec![atom("?s", base, "?m"), atom("?m", closure, "?o")],
        ),
    ]
}

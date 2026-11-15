// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Behaviour checks of the colour labelling.

use crate::colour::{Options, label};
use crate::{checks, families, fold_lines};

/// The document is the same after renaming every blank node and shuffling the
/// lines, and the optimised mode (worklist, forest path, twins, orbit pruning)
/// equals the literal definition, on small instances of each family.
#[test]
fn optimised_equals_literal_and_is_invariant_on_small_families() {
    let folds = [
        families::hubs(3, 2, 2, true),
        families::hubs(3, 2, 1, true),
        families::hubs(3, 2, 2, false),
        families::tree(2, 2),
        families::path(6),
        families::list(5, false),
        families::bipartite(3, true),
        families::bipartite(3, false),
        families::cube(2),
    ];
    for fold in &folds {
        let lines = fold_lines(fold);
        let report = checks::invariance("family", &lines, 5_000_000);
        assert!(report.contains("invariant=yes"), "{report}");
        assert!(!report.contains("LITERAL-DIFFERS"), "{report}");
    }
}

/// Synchronous rounds and worklist rounds give the same cell order, and on a
/// plain forest the single path equals the full leaf minimum.
#[test]
fn worklist_keeps_the_cell_order_and_the_forest_path_is_minimal() {
    let report = checks::refine_order_check(0..300);
    assert!(report.starts_with("synchronous and worklist"), "{report}");
    let report = checks::forest_claim(0..200, 8, 2_000_000);
    assert!(report.starts_with("forest claim holds"), "{report}");
}

/// A refused run reports its units and labels nothing.
#[test]
fn a_work_ceiling_refuses_instead_of_labelling() {
    let lines = fold_lines(&families::bipartite(4, true));
    let refused = label(
        &lines,
        Options {
            limit: 10,
            ..Options::LITERAL
        },
    );
    assert!(refused.is_err());
}

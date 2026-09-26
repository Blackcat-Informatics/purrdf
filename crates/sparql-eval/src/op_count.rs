// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Test-build operation counters for the walks a correlated `EXISTS` pays per outer row.
//!
//! A wall-clock measurement of nested `EXISTS` says nothing reproducible on a shared
//! machine; the number of tree nodes the evaluator copies, substitutes and analyses does.
//! Each walk that can grow with the nesting depth of an `EXISTS` body bumps one counter
//! here per node it visits, so a test reads the work an evaluation did as a count and
//! compares depths by that count rather than by time.
//!
//! Thread-local, so a test that runs its query with
//! [`crate::eval::EvalOptions::force_sequential`] reads exactly its own evaluation and
//! nothing a concurrently running test does. The whole module exists only in a test
//! build: every bump site is `#[cfg(test)]`.

use std::cell::Cell;

/// One kind of counted work.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Op {
    /// A pattern or expression node built by the correlated substitution walk
    /// (`crate::expr::substitute_pattern_impl` / `substitute_expr`).
    Substituted,
    /// A pattern node copied by `crate::stack::clone::pattern`.
    Cloned,
    /// A pattern node visited by the structural analysis
    /// (`crate::governor::soundness::analyze_pattern`).
    Analyzed,
    /// A pattern node visited by `crate::expr::pattern_all_vars`.
    VarWalked,
}

/// A snapshot of every counter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OpCounts {
    /// See [`Op::Substituted`].
    pub(crate) substituted: u64,
    /// See [`Op::Cloned`].
    pub(crate) cloned: u64,
    /// See [`Op::Analyzed`].
    pub(crate) analyzed: u64,
    /// See [`Op::VarWalked`].
    pub(crate) var_walked: u64,
}

impl OpCounts {
    /// Every counted node, of every kind.
    pub(crate) const fn total(self) -> u64 {
        self.substituted + self.cloned + self.analyzed + self.var_walked
    }
}

thread_local! {
    static COUNTS: Cell<OpCounts> = const {
        Cell::new(OpCounts {
            substituted: 0,
            cloned: 0,
            analyzed: 0,
            var_walked: 0,
        })
    };
}

/// Count one node of `op` on this thread.
pub(crate) fn bump(op: Op) {
    COUNTS.with(|counts| {
        let mut current = counts.get();
        match op {
            Op::Substituted => current.substituted += 1,
            Op::Cloned => current.cloned += 1,
            Op::Analyzed => current.analyzed += 1,
            Op::VarWalked => current.var_walked += 1,
        }
        counts.set(current);
    });
}

/// Zero every counter on this thread.
pub(crate) fn reset() {
    COUNTS.with(|counts| counts.set(OpCounts::default()));
}

/// This thread's counters.
pub(crate) fn read() -> OpCounts {
    COUNTS.with(Cell::get)
}

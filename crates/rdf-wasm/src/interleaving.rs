// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ledger of every `thread_local!` in the workspace's crates, and why each is safe
//! under the asynchronous lane's interleaving.
//!
//! An asynchronous job suspends at every host-resolved `SERVICE` or `LOAD` effect and at
//! every yield, and while it waits another job, or a synchronous call on the same engine,
//! runs on the same thread. Per-thread state a job left set would be read by the next
//! caller as its own; a `RefCell` a job held across a suspension would panic on the next
//! borrow. So every thread-local the workspace declares is accounted for here with the
//! reason it cannot be observed half-held: the job swaps it around every suspension
//! ([`Safety::Swapped`]), it is borrowed only inside one call that neither polls nor
//! suspends ([`Safety::PerCall`]), it is written only outside any evaluation
//! ([`Safety::NeverSpansSuspension`]), or it is not compiled into this package at all
//! ([`Safety::NotCompiledIn`]).
//!
//! `scripts/check-thread-locals.py` keeps [`LEDGER`] equal to the source: it reads every
//! `thread_local!` block under `crates/` and refuses a static that is declared but not
//! listed here, and one that is listed here but no longer declared. A file path in an
//! entry is relative to the repository root; a name is the static's identifier.

/// Why one thread-local is safe under the asynchronous lane's interleaving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Safety {
    /// The job installs its own value when its run starts, puts the outer value back at
    /// every suspension, reinstalls its own at every resumption and puts the outer value
    /// back when the run returns. The swap is one struct, `JobAmbient` in the job runtime,
    /// so every swapped static moves together.
    Swapped,
    /// Borrowed or locked only inside one call that returns an owned value and neither
    /// polls a stop signal nor calls a resolver, so no suspension can fall inside the
    /// borrow.
    PerCall,
    /// Written only outside any evaluation — by a test harness before it runs one — and
    /// read as a constant while one runs, so no job's writes can reach another's reads.
    NeverSpansSuspension,
    /// Not part of this package: declared under `#[cfg(test)]`, or in a crate the package
    /// does not depend on.
    NotCompiledIn,
}

/// One `thread_local!` static in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadLocal {
    /// The declaring file, relative to the repository root.
    pub file: &'static str,
    /// The static's identifier.
    pub name: &'static str,
    /// Why it is safe under interleaving.
    pub safety: Safety,
    /// The reason, as a sentence.
    pub reason: &'static str,
}

/// The stack floor and the walk-scope state travel as one [`purrdf_stack::Context`].
const STACK_CONTEXT: &str = "Part of `purrdf_stack::Context`, the stack half of the job's \
                             `JobAmbient`: installed with the region's base as the floor \
                             and no scope open when the run starts, swapped out at every \
                             suspension and back in at every resumption.";

/// The SHACL engine's ambient scopes travel as one `AmbientContext`.
const SHACL_CONTEXT: &str = "Part of `purrdf_shapes::sparql::AmbientContext`, the SHACL \
                             half of the job's `JobAmbient`: a validation job installs \
                             its governors, sources and registries through guards that \
                             stay open across every poll between focus nodes, so the \
                             whole set is swapped out at every suspension and back in at \
                             every resumption, and a job starts with none installed.";

/// A `#[cfg(test)]` declaration.
const TEST_ONLY: &str = "Declared under `#[cfg(test)]`, so it is not compiled into this package.";

/// Every `thread_local!` static under `crates/`, with its reason.
pub const LEDGER: &[ThreadLocal] = &[
    // ── purrdf-stack: the stack context ─────────────────────────────────────────────
    ThreadLocal {
        file: "crates/stack/src/lib.rs",
        name: "FLOOR",
        safety: Safety::Swapped,
        reason: STACK_CONTEXT,
    },
    ThreadLocal {
        file: "crates/stack/src/lib.rs",
        name: "WALK",
        safety: Safety::Swapped,
        reason: STACK_CONTEXT,
    },
    ThreadLocal {
        file: "crates/stack/src/lib.rs",
        name: "RESERVED",
        safety: Safety::Swapped,
        reason: STACK_CONTEXT,
    },
    ThreadLocal {
        file: "crates/stack/src/lib.rs",
        name: "SCOPES",
        safety: Safety::Swapped,
        reason: STACK_CONTEXT,
    },
    ThreadLocal {
        file: "crates/stack/src/lib.rs",
        name: "WIDENED",
        safety: Safety::Swapped,
        reason: STACK_CONTEXT,
    },
    // ── purrdf-shapes: the SHACL engine's ambient scopes and caches ─────────────────
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_FUNCTIONS",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_PARSER_OPTIONS",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_PROPERTY_FUNCTIONS",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_AGGREGATES",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_GOVERNORS",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_SOURCES",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CACHED_ENV",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "CURRENT_CALL_DEPTH",
        safety: Safety::Swapped,
        reason: SHACL_CONTEXT,
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "SPARQL_ENGINE",
        safety: Safety::PerCall,
        reason: "The per-thread `NativeSparqlEngine` whose plan cache is a `RefCell`: every \
                 borrow is a temporary inside one planning statement that parses and admits \
                 a plan, polls no signal and calls no resolver, and ends before the plan is \
                 evaluated.",
    },
    ThreadLocal {
        file: "crates/shapes/src/sparql.rs",
        name: "PREPARED_EXECUTIONS",
        safety: Safety::PerCall,
        reason: "The map of cached `PreparedExecution` handles is borrowed only to take a \
                 handle out or put one back, never across a run; a running validation holds \
                 its handle checked out, so a caller that runs while it waits prepares its \
                 own, and a handle prepared under another context's registries is \
                 re-prepared at the next checkout rather than run.",
    },
    ThreadLocal {
        file: "crates/shapes/src/class_membership.rs",
        name: "THREAD_INDEX_BUILDS",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/shapes/src/parallel.rs",
        name: "FORCE_PARALLEL",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/shapes/src/parallel.rs",
        name: "FORCE_CHUNK_SIZE",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    // ── purrdf-sparql-eval ──────────────────────────────────────────────────────────
    ThreadLocal {
        file: "crates/sparql-eval/src/solution.rs",
        name: "INTERNED_SCHEMAS",
        safety: Safety::PerCall,
        reason: "Column layouts interned by content: the `RefCell` is borrowed only inside \
                 one memo lookup or insert, which polls nothing.",
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/solution.rs",
        name: "INDEX_OF_CALLS",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/substitute.rs",
        name: "INTERNED_VARIABLES",
        safety: Safety::PerCall,
        reason: "Variables interned by name: the `RefCell` is borrowed only inside one memo \
                 lookup or insert, which polls nothing.",
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/plan_memory.rs",
        name: "OBSERVER",
        safety: Safety::PerCall,
        reason: "The plan-memory observer's totals are locked only to add or credit a byte \
                 count, inside one call.",
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/execution.rs",
        name: "MEMO_VERIFICATION_ENABLED",
        safety: Safety::NeverSpansSuspension,
        reason: "Exists only under `debug_assertions` and is written only by a test harness \
                 before it runs an evaluation; an evaluation reads it as a constant.",
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/op_count.rs",
        name: "COUNTS",
        safety: Safety::NotCompiledIn,
        reason: "The whole `op_count` module is declared under `#[cfg(test)]`.",
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/parallel.rs",
        name: "FORCE_PARALLEL",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/parallel.rs",
        name: "FORCE_CHUNK_SIZE",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/binop.rs",
        name: "MERGE_COUNT",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/deferred_exists.rs",
        name: "FORCE_EAGER",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/eval.rs",
        name: "PREPARED_EXISTS_BUILD_COUNT",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/expr.rs",
        name: "FORCE_EXISTS_STRATEGY",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/expr.rs",
        name: "SUPPRESS_FIRST_WITNESS_WRAP",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/modifier.rs",
        name: "NUMERIC_FOLD_TRACE",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/path.rs",
        name: "LEVEL_ADVANCES",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/path.rs",
        name: "POWER_EXPANSIONS",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    ThreadLocal {
        file: "crates/sparql-eval/src/vm/mod.rs",
        name: "TRACE",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    // ── purrdf-core, purrdf-hnsw: test hooks for the distance kernels ───────────────
    ThreadLocal {
        file: "crates/rdf-core/src/distance/binary64.rs",
        name: "BYPASSED",
        safety: Safety::NotCompiledIn,
        reason: "Declared under `#[cfg(all(test, target_arch = \"x86\", not(target_feature = \
                 \"sse2\")))]`, so it is not compiled into this package.",
    },
    ThreadLocal {
        file: "crates/rdf-core/src/distance/reassociated.rs",
        name: "HIDDEN",
        safety: Safety::NotCompiledIn,
        reason: "Declared under `#[cfg(all(test, target_arch = \"x86_64\"))]`, so it is not \
                 compiled into this package.",
    },
    ThreadLocal {
        file: "crates/hnsw/src/lib.rs",
        name: "LACKING",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
    // Test support is never linked into the published wasm package.
    ThreadLocal {
        file: "crates/testkit/src/harness.rs",
        name: "CAPTURE",
        safety: Safety::NotCompiledIn,
        reason: "the test harness is a dev-dependency only",
    },
    ThreadLocal {
        file: "crates/testkit/tests/prop.rs",
        name: "MACRO_CASES",
        safety: Safety::NotCompiledIn,
        reason: "a testkit integration-test counter, never compiled into the package",
    },
    // ── purrdf-alloc-probe: a dev-dependency-only crate ─────────────────────────────
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_ALLOCATIONS",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_REQUESTED_BYTES",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_LIVE_BYTES",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_PEAK_BYTES",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_TROUGH_BYTES",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    ThreadLocal {
        file: "crates/alloc-probe/src/lib.rs",
        name: "THREAD_WINDOW_OPEN",
        safety: Safety::NotCompiledIn,
        reason: "`purrdf-alloc-probe` is a `[dev-dependencies]`-only crate; no package \
                 depends on it at run time.",
    },
    // ── purrdf-wasm: the job runtime's own registries ───────────────────────────────
    ThreadLocal {
        file: "crates/rdf-wasm/src/async_query.rs",
        name: "EXCHANGES",
        safety: Safety::PerCall,
        reason: "The shared-exchange registry is borrowed inside one helper that returns \
                 owned values; it is never held when `suspend` is called or while an \
                 exchange's answer is delivered.",
    },
    ThreadLocal {
        file: "crates/rdf-wasm/src/async_query.rs",
        name: "LAST_EXCHANGE_ID",
        safety: Safety::PerCall,
        reason: "Read and advanced inside one helper that hands out the next exchange id.",
    },
    ThreadLocal {
        file: "crates/rdf-wasm/src/async_query.rs",
        name: "JOBS",
        safety: Safety::PerCall,
        reason: "The job registry is borrowed only for an insert, a lookup or a removal, \
                 never across a run.",
    },
    ThreadLocal {
        file: "crates/rdf-wasm/src/async_query.rs",
        name: "LAST_JOB_ID",
        safety: Safety::PerCall,
        reason: "Read and advanced inside one helper that hands out the next job id.",
    },
    ThreadLocal {
        file: "crates/rdf-wasm/src/async_query.rs",
        name: "SHARED",
        safety: Safety::NotCompiledIn,
        reason: TEST_ONLY,
    },
];

/// How many ledger entries the job's `JobAmbient` swap is responsible for.
#[must_use]
pub const fn swapped_count() -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < LEDGER.len() {
        if matches!(LEDGER[index].safety, Safety::Swapped) {
            count += 1;
        }
        index += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use purrdf_testkit::paths::workspace_root;

    use super::{LEDGER, Safety, ThreadLocal};

    /// No `(file, name)` pair is listed twice.
    #[test]
    fn every_entry_is_listed_once() {
        let mut seen = BTreeSet::new();
        for entry in LEDGER {
            assert!(
                seen.insert((entry.file, entry.name)),
                "{}::{} is listed twice",
                entry.file,
                entry.name
            );
        }
    }

    /// Every listed file exists and declares the listed static.
    #[test]
    fn every_entry_names_a_declared_static() {
        for ThreadLocal { file, name, .. } in LEDGER {
            let source = std::fs::read_to_string(workspace_root().join(file))
                .unwrap_or_else(|error| panic!("{file} is listed but cannot be read: {error}"));
            let declared = source.lines().any(|line| {
                let trimmed = line.trim_start();
                let body = trimmed.strip_prefix("pub(crate) ").unwrap_or(trimmed);
                body.strip_prefix("static ").is_some_and(|rest| {
                    rest.starts_with(name) && rest[name.len()..].starts_with(':')
                })
            });
            assert!(declared, "{file} does not declare `static {name}`");
        }
    }

    /// The swapped entries are exactly the statics of the two contexts `JobAmbient` moves:
    /// the five of `purrdf_stack::Context` and the eight of the SHACL `AmbientContext`.
    #[test]
    fn the_swapped_entries_are_the_two_contexts() {
        let swapped: Vec<_> = LEDGER
            .iter()
            .filter(|entry| entry.safety == Safety::Swapped)
            .map(|entry| entry.file)
            .collect();
        assert_eq!(swapped.len(), super::swapped_count());
        assert_eq!(
            swapped
                .iter()
                .filter(|file| **file == "crates/stack/src/lib.rs")
                .count(),
            5
        );
        assert_eq!(
            swapped
                .iter()
                .filter(|file| **file == "crates/shapes/src/sparql.rs")
                .count(),
            8
        );
    }
}

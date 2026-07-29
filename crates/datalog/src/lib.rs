// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![forbid(unsafe_code)]

//! Deterministic, wasm-clean Datalog evaluation: a columnar relation store and a
//! stratified semi-naive fixpoint.
//!
//! This crate is the shared execution substrate beneath PurRDF's rule-driven
//! engines. A rule set is *data* — a table of clauses over a relation store — so
//! the RDF, RDFS and OWL 2 RL calculi, RIF-Core rules and SHACL-AF `sh:rule`
//! entailment become rule tables over one evaluator instead of four hand-written
//! fixpoints, each with its own indexes, its own termination argument and its own
//! opportunity to diverge.
//!
//! # Determinism
//!
//! Identical input yields byte-identical output, on every target. Per-key rows
//! keep insertion order, the arrangement is sorted, and **no map iteration order
//! reaches an output path**. Where evaluation is parallelised it uses indexed
//! `par_chunks`/`par_iter` reduced in source order — never `par_sort` or
//! `par_bridge`, which are not order-stable — and degrades to inline-sequential
//! on `wasm32-unknown-unknown`.
//!
//! # Budgets are constants, not knobs
//!
//! Step, fact and arena ceilings are fixed constants and their consumption is
//! *reported*, never configured. A caller-supplied budget would mean two callers
//! running the same program over the same input get different answers — the same
//! semantic optionality that the no-Cargo-features rule exists to prevent, merely
//! arriving through a parameter instead. There is no wall-clock budget: it would
//! break both wasm and reproducibility.
//!
//! # Portability
//!
//! No filesystem, no clock, no RNG, no ambient I/O. The crate builds for
//! `wasm32-unknown-unknown` and is part of the workspace wasm gate.
//!
//! # Physical primitives
//!
//! The modules below are the substrate the store, the cursors and the fixpoint
//! are built from:
//!
//! - [`id`] — branded niche IDs, so a term handle can never be passed where a
//!   predicate handle is expected.
//! - [`arena`] — the phase-scoped row/tuple bump arena, reset at every round
//!   boundary.
//! - [`bitset`] — the dense round-delta membership bitset over row ids.
//! - [`binding_pattern`] — the arity-generic adornment lattice shared by demand
//!   keying and index selection.
//!
//! # The relation store and its cursors
//!
//! - [`store`] — the columnar [`RelationStore`](store::RelationStore): one shared
//!   arrangement per predicate, held as sorted immutable batches plus a mutable
//!   tail, deduped by a galloping probe rather than by hashing, and generic over an
//!   abelian [`Weight`](store::Weight) monoid so signed (Z-set) multiplicities —
//!   and hence retraction — are a compiled property of the representation.
//! - [`cursor`] — the zero-allocation lending cursor over one arrangement, and the
//!   globally value-ordered trie cursor the leapfrog join seeks over.
//!
//! # The rule IR
//!
//! - [`clause`] — the **DL-clause** `U₁ ∧ … ∧ Uₙ → ∃ȳ. (C₁ ∨ … ∨ Cₘ)`, the crate's one
//!   rule representation, where each `Cᵢ` is itself a conjunction of head atoms — so
//!   `A ⊑ ∃r.C`, which lowers to `∃y. (r(x, y) ∧ C(y))` with ONE shared witness, is one
//!   rule. That shape covers all five head forms — atomic (a Datalog rule), existential,
//!   disjunctive, conjunctive and empty (`false`) — so the chase and the hypertableau that
//!   will consume the last four need no second IR and no redesign of this one. Only the
//!   atomic form has evaluation semantics here; the other four are refused by name at the
//!   plan pipeline's entrance, never silently accepted and never silently dropped.
//!
//! # Planning
//!
//! - [`plan`] — the consuming type-state pipeline
//!   `Parsed → Stratified → Planned → Executable`, which makes an unstratified or
//!   unplanned program unrepresentable at the executor boundary, plus the
//!   store-independent per-rule join plan it memoizes: body partition, flat binding
//!   frame, sideways-information-passing order, index selection, and the certified
//!   cyclic subplans a worst-case-optimal join consumes.
//!
//! # Evaluation
//!
//! - [`seminaive`] — the stratified semi-naive fixpoint itself:
//!   [`compile`](seminaive::compile) turns a rule program into an
//!   [`Executable`](plan::Executable) or names the negative cycle that makes it
//!   non-stratifiable, and [`evaluate`](seminaive::evaluate) runs each stratum to its
//!   least fixpoint over a seeded store. The positive body of every rule goes through one
//!   of two kernels — the indexed binary join, or a leapfrog triejoin over a
//!   planner-certified cyclic component — and the two are held to producing identical
//!   relations by a differential test. Rounds are rule-parallel through rayon's indexed
//!   `par_iter`, merged strictly in program order.
//!
//! # Reuse
//!
//! - [`cache`] — the caller-owned, content-addressed plan cache. A compiled program is
//!   keyed by a BLAKE3 digest over the planner version, the caller's contract hash and a
//!   canonical digest of the clause program, so an identical program is compiled once. The
//!   cache is owned by the caller's planner and is never a process global: a hidden global
//!   would make a result depend on evaluation history.
//!
//! # The correctness oracle
//!
//! Reproducibility and correctness are different properties, and only one of them is
//! tested by running the same program twice: a systematically wrong evaluator is perfectly
//! reproducible. The crate's correctness oracle is therefore a corpus of Datalog programs
//! whose answers have a CLOSED FORM — transitive closure of a chain, the complete
//! reachability of a cycle, the same-generation pairs of a two-level tree — asserted by
//! exact set equality against a golden built by construction rather than by an engine.

pub mod arena;
pub mod binding_pattern;
pub mod bitset;
pub mod cache;
pub mod clause;
pub mod cursor;
pub mod id;
pub mod plan;
pub mod seminaive;
pub mod store;

#[cfg(test)]
pub(crate) mod synth_corpus;

#[cfg(test)]
pub(crate) mod test_support {
    //! Deterministic helpers shared by the crate's unit tests.
    //!
    //! The determinism contract is asserted by feeding the same inputs in many
    //! different orders and demanding identical observable state. That needs a
    //! shuffle, and the crate has no RNG (and must not acquire one — no ambient
    //! entropy on `wasm32-unknown-unknown`), so the permutation is generated from
    //! an explicit seed by a pure integer mix. Every "random" order in the test
    //! suite is therefore reproducible on every target: a failure names the seed
    //! that produced it.

    /// One step of the SplitMix64 mixing function — a pure, seed-driven integer
    /// hash with no ambient state.
    fn mix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A deterministic permutation of `items` selected by `seed`.
    ///
    /// A Fisher-Yates shuffle driven by [`mix`]; the same `seed` always yields the
    /// same order, on every target.
    pub(crate) fn permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
        let mut out = items.to_vec();
        let mut state = seed;
        for i in (1..out.len()).rev() {
            let j = (mix(&mut state) % (i as u64 + 1)) as usize;
            out.swap(i, j);
        }
        out
    }
}

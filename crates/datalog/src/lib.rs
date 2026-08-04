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
//! A caller-owned [`stop::StopSignal`] is admitted, and is admitted for
//! exactly that reason rather than in spite of it: it changes no answer. An unstopped run
//! returns precisely what it would have returned with no signal attached, and a stopped one
//! returns **nothing** — a typed refusal, never a truncated model. See [`stop`] for the
//! full argument and for the contract an implementation is bound by.
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
//!   predicate handle is expected. A [`RowId`](id::RowId) is dense and minted in
//!   store-wide insertion order, which is what lets the fixpoint address a round's
//!   committed rows as a RANGE rather than as a set — see [`seminaive`] below.
//! - [`binding_pattern`] — the arity-generic adornment lattice shared by demand
//!   keying and index selection.
//!
//! There is no separate row arena and no delta membership set, because this
//! evaluator's shapes need neither. A body row is the fixed arity-4 quad carried in
//! a `Copy` struct and a rule's bindings are a flat frame indexed by plan slot, so
//! no variable-arity tuple is ever allocated; and the round delta is a contiguous
//! `[lo, hi)` span of row ids, so membership is one range compare rather than a
//! word test over an allocated bitmap.
//!
//! # The relation store and its cursors
//!
//! - [`store`] — the columnar [`RelationStore`](store::RelationStore): ONE arity-4
//!   relation `triple(subject, predicate, object, graph)` over one term dictionary,
//!   physically partitioned by its `(predicate, graph)` positions. Each partition is a
//!   shared arrangement held as sorted immutable batches plus a mutable tail, deduped by a
//!   galloping probe rather than by hashing, and generic over an abelian
//!   [`Weight`](store::Weight) monoid so signed (Z-set) multiplicities — and hence
//!   retraction — are a compiled property of the representation. A constant predicate
//!   reaches its arrangement through one ordered-map probe; a variable one sweeps the
//!   matching partitions in lexical order and still indexes inside each.
//! - [`cursor`] — the zero-allocation lending cursor over one arrangement, and the
//!   globally value-ordered trie cursor the leapfrog join seeks over.
//!
//! # The rule IR
//!
//! - [`clause`] — the **DL-clause** `U₁ ∧ … ∧ Uₙ → ∃ȳ. (C₁ ∨ … ∨ Cₘ)`, the crate's one
//!   rule representation. Every atom is the arity-4 quad `triple(?s, ?p, ?o, ?g)` with the
//!   predicate carried as DATA, so a rule may quantify over the property position — which
//!   is what OWL 2 RL's `prp-dom`, `prp-spo1`, `prp-trp` and their siblings require and
//!   what a relation-symbol encoding cannot express at all — and over the graph, so
//!   reasoning is per-graph rather than flattened. Each `Cᵢ` is itself a conjunction of
//!   head atoms — so
//!   `A ⊑ ∃r.C`, which lowers to `∃y. (r(x, y) ∧ C(y))` with ONE shared witness, is one
//!   rule. That shape covers all five head forms — atomic (a Datalog rule), existential,
//!   disjunctive, conjunctive and empty (`false`) — in one type, so a consumer of any of
//!   them needs no second IR. Only the atomic form has evaluation semantics in the
//!   semi-naive evaluator; [`chase`] consumes the existential and conjunctive forms, and
//!   the disjunctive and inconsistency forms are REFUSED BY NAME at the plan pipeline's
//!   entrance — never silently accepted and never silently dropped. No evaluator here
//!   case-splits, because a case split is not a least fixpoint; the consumer of the
//!   disjunctive form is `purrdf-entail`'s OWL-Direct HYPERTABLEAU, which classifies its own
//!   `SHOIQ(D)` DL-clauses through [`clause::HeadForm`] and branches on exactly that form
//!   over concept-id atoms — two of which (`≥n r.C(x)` and the equality `x ≈ y`) no arity-4
//!   quad can express without minting a predicate IRI. Classifying a form this crate declines
//!   is what makes the refusal precise rather than a parse failure.
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
//! - [`chase`] — the restricted existential chase, the consumer the existential head form
//!   was represented for. [`certify`](chase::certify) is a pure function of the clause set
//!   that either proves it terminating by constant-refined weak acyclicity or names the
//!   existential edges that lie in a cycle, and [`chase`](chase::chase) runs the fixpoint
//!   only on a certified program. A witness is a BLANK NODE addressed on the frontier
//!   binding — PurRDF mints no vocabulary, so it mints no individual either — and an
//!   already-witnessed obligation is skipped, which is what makes the fixpoint converge. A
//!   disjunctive head, an inconsistency clause and a negated body atom are refused by name.
//!
//! # Checkable proofs
//!
//! - [`proof`] — the hash-consed proof-term arena. A
//!   [`Derivation`](seminaive::Derivation) is a LOG: believing it means believing the engine
//!   that wrote it. A [`ProofArena`](proof::ProofArena) term is checked by
//!   [`check`](proof::ProofArena::check), which re-derives the conclusion from the premises
//!   and the named clause and returns the fact IT computed — so a step the rule does not
//!   license is rejected however well-formed the record of it is. Terms are interned, so a
//!   shared subproof is stored once; a proof is named by a BLAKE3 content digest over its
//!   canonical encoding, never by a fabricated IRI.
//!
//! # Goal-directed backward resolution
//!
//! - [`term`], [`unify`], [`resolve_fol`] — a SEPARATE, generic compound-term
//!   arena, a Robinson-style order-sorted unification algorithm over it, and an
//!   SLG-tabled backward resolver with three-valued well-founded semantics,
//!   existing beside the forward semi-naive fixpoint above. Some questions —
//!   "does this one goal hold, and why" — are cheaper to answer BACKWARD, from the
//!   goal toward the facts that support it, without materialising the rest of the
//!   program's model the way [`seminaive::evaluate`] must. [`unify`] operates on
//!   [`term::TermDag`]'s function-symbol applications and locally-nameless binders
//!   rather than on the flat quad shape, because the resolver's tabling needs
//!   richer structure than a quad can hold, and because a future description-logic
//!   layer built on this receiving surface will need genuinely compound concept
//!   terms. [`resolve_fol::solve_datalog_goal`] bridges the two worlds: it lowers
//!   this crate's own [`clause::DlClause`] program into the compound-term IR and
//!   answers one goal by SLG resolution. `purrdf-entail`'s chase explanation calls
//!   it to RE-DERIVE its conclusion backward, so every explanation is reached by
//!   two engines that share the clause program and nothing else, and a
//!   disagreement fails the call rather than being reported as a proof.
//!
//!   Whether the search can REFUTE depends on reaching a fixpoint, which a
//!   confirmation does not need. `Simple`, `Rdf` and `D` reach one in microseconds.
//!   `Rdfs` and `OwlRl` are skipped on COST, not inability: measured in release,
//!   RDFS reaches `Complete` in ~4.8s — its refutation branch is live — and OWL 2
//!   RL is budget-cut to `Partial` at ~31s, with both reporting a confirmation.
//!   Neither is affordable on a per-explanation diagnostic, so the certificate
//!   reports `backward skipped` for them rather than implying a check that never ran.
//!
//! # Reuse
//!
//! - [`cache`] — the caller-owned, content-addressed plan cache. A compiled program is
//!   keyed by a BLAKE3 digest over the planner version, the caller's contract hash and a
//!   canonical digest of the clause program, so an identical program is compiled once. The
//!   cache is owned by the caller's planner and is never a process global: a hidden global
//!   would make a result depend on evaluation history.
//!   [`contract_hash`](cache::contract_hash) is the crate's own answer to "which calculus
//!   produced this result": the clause program, the three fixed budgets and a
//!   hand-maintained [`CALCULUS_VERSION`](cache::CALCULUS_VERSION), hashed as DATA rather
//!   than as source text.
//!
//! # The correctness oracle
//!
//! Reproducibility and correctness are different properties, and only one of them is
//! tested by running the same program twice: a systematically wrong evaluator is perfectly
//! reproducible. The crate's correctness oracle is therefore a corpus of Datalog programs
//! whose answers have a CLOSED FORM — transitive closure of a chain, the complete
//! reachability of a cycle, the same-generation pairs of a two-level tree — asserted by
//! exact set equality against a golden built by construction rather than by an engine.

pub mod binding_pattern;
pub mod cache;
pub mod chase;
pub mod clause;
pub mod cursor;
pub mod id;
pub mod plan;
pub mod proof;
pub mod resolve_fol;
pub mod seminaive;
pub mod stop;
pub mod store;
pub mod term;
pub mod unify;

pub use stop::StopSignal;

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

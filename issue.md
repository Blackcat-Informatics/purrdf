# Issue #493: datalog: incremental maintenance of a materialized model under insertions and retractions (Z-set weights already compile)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

An evaluator entry point that maintains a materialized Datalog / SPARQL 1.2 RL model under insertions **and** retractions, by deltas, using the `Weight` machinery the store already compiles for. Today every change re-evaluates from the whole store, and a retraction rebuilds it.

## Current behaviour (`main` at `6273b6173`)

- `crates/datalog/src/store.rs:272-316`: `pub trait Weight` with `impl Weight for ()` (production) and `impl Weight for i64` (a Z-set: `combine` sums, a zero weight annihilates). The docs say "Retraction — and with it incremental (backward/forward, DRed-style) maintenance — is therefore already a compiled property of the representation: the lever is one type parameter, never a redesign of the store."
- The signed instantiation is exercised only in tests (`store.rs:2132-2135`, under `#[cfg(test)]`).
- `store.rs:1284-1287`: "this store has no deletion … a retraction is a new store built from the surviving rows". `schedule.rs:488-497` `fn retract` rebuilds the store from survivors.
- `crates/entail/src/engine.rs:988-999` says it plainly: "What the crate does not yet have is an evaluator entry point that consumes one: [`evaluate`] takes a whole store and seeds its round-1 delta from the whole of it … calling it 'incremental evaluation' would claim a maintenance path that does not exist yet."

## Why it matters

Katamari's "current" view of its claims is a declared SPARQL 1.2 RL rule set over every claim's reifier (ADR-0038 38.5), and its default view is to be **maintained incrementally at the head** as commits arrive and computed on demand at other coordinates (ADR-0038 38.17). Commits are small and constant (an AI's memory writes all day; a mail vault syncs continuously), and they include retractions-as-new-claims (supersession, retraction, suppression change which base triples a view treats as asserted). Re-evaluating a view over 300 million claims per commit is not possible; rebuilding the store per retraction even less so.

The field has settled this: counting and DRed for stratified Datalog with negation, RDFox's backward/forward algorithm, and DBSP (PVLDB 2023, VLDBJ 2025), which makes any program built from its operators incremental, recursion and negation included, over exactly the Z-set weights purrdf's store already has. Katamari also needs the dependency structure for a second purpose: when an object is purged, every derived fact that depended on it must go, and an incremental engine's support structure is that closure.

Proofs: Katamari keeps one witness per derived fact and rebuilds full proofs on demand (Soufflé's approach, about 1.31x runtime), so maintenance should keep at least one valid derivation per fact current.

## Proposed direction (a sketch)

1. `Maintained<Program>`: holds the materialized model at `W = i64` (or a counting/DRed support structure) and exposes `apply(delta: Batch<i64>) -> ModelDelta` where the input delta holds insertions (+1) and retractions (-1) of base facts, and the output is the exact change to the derived model.
2. Stratified negation: process strata in order; a change in a negated stratum propagates as a delta to dependents (DBSP's treatment, or DRed's over-delete and re-derive).
3. Witness maintenance: each derived fact keeps one current derivation; when it is retracted, re-derivation finds another or the fact goes.
4. Equivalence as the contract: after any sequence of deltas, the maintained model equals `evaluate` from scratch over the resulting base, with the same identity.
5. Limits and partial results as in #488: a delta that cannot be applied within the caller's budget reports what is certain.

## Acceptance criteria

- Property test: for random programs (recursive, with stratified negation) and random sequences of insert/retract batches, the maintained model equals from-scratch evaluation after every batch.
- Applying a one-fact delta to a transitive closure over 10^6 edges costs work proportional to the affected part, not the whole (measured in join steps).
- A retraction never rebuilds the store.
- Every derived fact has a checkable witness after every batch (checked with the independent proof checker).
- `purrdf-entail`'s refutation lane can use it, removing the caveat at `engine.rs:988-999`.

## Related

#488 (certified partials and resumable evaluation), #479, #364.


## Comments (3)

### paudley — 2026-10-09T06:49:56Z

## Prior art in gmeow-ontology: this is already built there, in a narrower fragment

gmeow-ontology's reasoner (`crates/logic`, at `38f568b04`) has working implementations of three parts of this issue's sketch. We own both codebases, so the code can be moved here rather than re-derived.

1. **Recursive DBSP maintenance**: `crates/logic/src/physical/incremental.rs`. It uses Z-set `i64` weights, `distinct` for membership, and joins differentiated as `new[..p] x delta[p] x old[p+1..]`. It also **keeps the inner iteration history**, which this issue's sketch does not mention and which is load-bearing. In the module's words: "Merely iterating from the old closure computes a greatest fixed point after a retraction and can retain a mutually-supporting cycle whose last asserted ground was removed." With the history kept, such a cycle disappears at the depth where it originally became grounded.
   - **Fragment:** finite, positive, *binary* Datalog. No NAF, no arithmetic, no existential heads. An uncovered rule hard-fails at construction, and the circuit never rebuilds silently.
   - **What moving it here needs:** widening to purrdf's atoms (not only binary), and stratified negation (step 2 above), as DBSP's stratum-by-stratum treatment.
2. **Truth-maintenance derivation graph**: `crates/logic/src/derivation_graph.rs`. It records the full disjunctive justification structure, `Fact = OR(Asserted, RuleApplication(AND premises...), ...)`. Deletion survival is a least fixpoint over the surviving justifications, so a cycle never bootstraps itself. Derivation ids are content-addressed: reifier IRIs only, never interner ids, pinned by a golden.
   - This is the natural **oracle for acceptance criterion 1** (maintained == from-scratch), and a second, independent path for purge: when an object is purged, which derived facts lose their last support.
3. **Witnesses** (step 3 above): `crates/logic/src/provenance.rs` keeps a minimum-proof-height witness per fact with a deterministic total-order tie-break on `(proof_height, ...)`, and `physical/incremental.rs` keeps it current under deltas.

**What Katamari needs on top** (ADR-0045 45.5 and 45.6, Proposed):
- Maintenance under retraction is required from the first correction, because every Katamari correction is a supersede.
- A derived relation may carry its root, kept as separate monotone relations, so that a policy depending on *how* a fact was reached stays monotone within an evaluation.

**Limits, per ADR-0045 45.21** (Proposed; Katamari's no-chosen-limits rule): the moved code should arrive without gmeow's compiled-in caps. The step budget is already unbounded by default, so it is fine. The others, for example `physical/seminaive/joint/input/admission.rs:17-18`, should not come across as constants.


### paudley — 2026-10-09T21:03:43Z

## The host's change stream: a `ChangeSource` input

From Katamari ADR-0045 45.22 and `docs/design/PURRDF-SEAM.md` §2.4 at `e65fe50`. Patrick approved the design 2026-10-09.

The host yields each committed transaction as a **Z-set batch**:
- insertions are +1;
- retractions are −1;
- a supersede is a retraction plus an insertion;
- a purge is retraction of everything it removed.

Each batch is tagged with the host's commit position as its **watermark**. The maintained model reports, per maintained relation, the watermark it has applied through, so a reader can say "not yet evaluated" rather than "none".

This is the input side of the `apply(delta)` sketched above. It also carries Katamari's dangling-reference view: when an object is purged, the claims that referred to it become dangling, maintained from that same batch.


### paudley — 2026-10-20T02:44:18Z

A second consumer for this: expressing Rust name resolution as a stratified program needs an alternating fixpoint and a drop loop that re-evaluate from scratch today (each evaluation re-interns all syntax facts, and the store is moved into `evaluate_guarded` and lost on error). A hub-of-globs program needs 4 full evaluations where most of the model is unchanged; an API to change a small set of EDB facts and continue from the previous model, at least for additions to relations used only negatively in a lower stratum, would remove that.


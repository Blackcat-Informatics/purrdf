# Focused independent Stage 1 plan review

VERDICT: PASS

Completion-adversary outcome: PASS (Stage 1 plan text only).

Reviewed plan: `.stage/paged-tier-an-lsm-style-stack-of-sealed/plan.md`.
SHA-256: `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
Source HEAD/base: `c1d5bcd259089769e3679c6e1926b5ce24fbc1c2`.
Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Issue: complete supplied #457 body and zero comments; no new forge retrieval.

## Coverage and authority

Read the full authoritative plan, issue-analysis.md, prior-art-assessment.md,
worktree AGENTS.md, standing `/home/paudley/Active/purrdf/.baseline` and `.goals`,
the complete C/G backend contract, relevant paged/mutable/pack/evaluator source,
existing integration tests, and the shared-view carrier design's adoption/cost
sections. Applied `planning-reviews.md` and `completion-adversary.md` Stage 1
rubric. This is independent read-only judgment, not implementation qualification.
No code mutation, compilation, runtime checks, publication, or commits occurred.

## Completeness adjudication

| Requirement | Plan coverage | Judgment |
|---|---|---|
| One or more sealed base generations | Lines 23-27, matrix row 60, Task 1 | Mapped to public constructor/append and counted zero older-page reads. |
| Stack of sealed delta batches | Matrix rows 60-62, Task 1 | Preserves per-generation G3 and supports independent dictionaries, removal-only sealing and subsequent fresh head. |
| Mutable resident head with consumer-owned reconstruction | Lines 42-46, rows 61-63, Task 1 | Value mutation and snapshot copy are concrete; no log/durability obligation is invented. |
| Removals across every older layer and later reinsertion | Lines 33-34, row 62, Task 1 | Ordered removals and newest additions have explicit precedence, exercised across publication boundaries against an independent oracle. |
| One whole-stack G9 snapshot | Lines 28-31 and 44-45, row 63, Task 1 | Immutable state and direct checks of every source generation/page count, including unaccessed sources, are explicit. |
| G7/G8 exact evidence and shared operation limits | Lines 28-31, row 64, Task 1 | Layer-qualified origins, global totals, zero/equality/over-limit checks and typed failure are mapped. |
| G10 exact physical summaries and sound logical filtering | Lines 35-40, row 65, Task 1 | Required cross-stream promotions/demotions are recognized; the plan does not claim masked logical counts remain exact. |
| Fold into a canonical new base with no partial publication | Lines 48-54, row 67, Tasks 1-2 | Actual fold and canonical eager sealing, not translation-only compact, are planned. |
| Read equivalence | Row 66, Tasks 1-2 | Public NativeSparqlEngine evaluation and independent effective-value oracle compare real answers. No eager scratch dataset substitutes for the stack query path. |
| Compaction page-byte equivalence | Lines 48-54, row 67, Tasks 1-2 | Explicit page bound, canonical partition law, actual ordered per-page PackBuilder bytes and canonical global numbering establish the correct observation. |
| Term-identity question | Lines 23-27, row 61, Tasks 1-2 | Fresh snapshot dictionary by value is a coherent answer distinct from either source generation; local summaries remain unchanged. |
| Removal-set and summary-exactness question | Lines 33-40, rows 62/65 | Ordered layer ownership and conservative logical bounds answer both parts. |
| Depth question | Lines 45-46 and alternatives | Consumer policy plus depth evidence is explicit, within the issue's stated boundary. |
| Consumer storage/log/atomic publication/compaction timing excluded | Lines 13-14 and alternatives | Matches the issue's actual exclusions, without cutting the in-library fold or mutable mechanism. |

No UNFALSIFIABLE CRITERION, PRE-EMPTIVE DESCOPE, UNMAPPED REQUIREMENT, or
PLANNED-BY-REFUSAL finding is open. The plan can pass without executed code;
Stage 2 must obtain all mapped real-entry-point evidence.

## Constraints, design and utility

The design honors C0.8 by mapping values into a new snapshot dictionary, rather
than claiming independent numeric IDs share a space. It preserves one admission,
one cache and one budget/error latch by routing all physical pages through the
existing paged query home. Direct vector checks avoid reducing multi-source
snapshot integrity to a collision-prone generation hash. G3 remains enforced
inside sealed generations and is deliberately not misapplied to ordinary
cross-generation repeated facts. Ordered tombstones correctly permit a newer
reinsertion. The head API explicitly refuses to disguise operational membership
failure as an infallible false boolean.

The RDF 1.2 cross-table composition is a necessary domain transformation, not a
minor convenience. Recognizing that physical annotations can become logical
ordinary rows and vice versa is the right integration seam; factoring the actual
classification decision while preserving source-specific probes satisfies the
repository's one-home rule. Native/delta regression evidence must accompany that
factorization, as Task 1 already requires.

Canonical page construction strengthens utility beyond translation-only
renumbering without adding a durable backend or scheduler. PackBuilder is an
appropriate existing deterministic carrier for concrete page-byte observations.
Canonical global ID order must still be checked independently of PackBuilder,
because canonical serialization can conceal different input IDs; the plan
explicitly includes both observations.

The standing goals require "MAXIMAL PERFORMANCE - Try hard to provide optimal
rust solutions" and "MAXIMAL PORTABILITY - Keep wasmable code as your core
value." The plan makes no unsupported speed claim, avoids content resealing on
append/snapshot, documents costs and requires affected wasm builds. No source
features, dependencies, persistence, model/lifecycle actions, or forbidden merge
paths are proposed. Required signed commits and hooks remain intact. Process
references are confined to Stage/GitHub; shipped contract prose excludes them.

## Concrete implementation guidance; no open approval decision

These are bounded refinements within already planned Tasks 1-2, not new
acceptance scope or grounds for a NEEDS-REVISION verdict:

1. Snapshot rebuilding is metadata-only, not necessarily cheap. Record its
   exact work/retention law: a fresh dictionary/global translation rebuild can
   be O(all retained dictionary values + all translation entries) per snapshot.
   Repeatedly appending small batches can accumulate quadratic total metadata
   work, as the existing shared-view carrier design already identifies for
   reconstruction versus extend. Reuse an already published immutable snapshot
   for multiple query operations; consider incremental reuse only if the actual
   measured/concrete cost warrants it. Do not advertise O(delta) snapshots or a
   performance improvement without evidence. The required zero-content-read
   criterion remains valid even when metadata rebuilding is linear.
2. Specify the canonical partition law fully in Task 1's shipped documentation:
   total order of typed RDF rows, what counts toward the positive page-row bound,
   placement of declared-empty graphs, and the representation of a completely
   empty effective set. Use the same declared rule for the independent eager
   input and stack fold, and prove per-page bytes under at least two page bounds
   and two histories. Ensure an annotation-only page retains its typed row even
   when the corresponding reifier is in another output page.
3. Include explicit identity and chronology counterexamples in the already
   planned adversarial tests: numerical IDs/page ordinals colliding across
   sources, equal scoped blanks versus different scopes, directional literals,
   nested triples, q in multiple older layers, same-head insert/remove/insert,
   and removal of one versus the last surviving reifier in a specific graph.
   Existing core law tests provide reusable fixtures; do not require property
   generation to find these essential examples by chance.
4. Include a non-monotone/property-path query in the Task 2 matrix in addition
   to its listed join/OPTIONAL/aggregate forms, and compare bags by value. This
   tests consumers that can expose duplicated or wrongly omitted facts; query
   families are not separate implementations of the stack mechanism.
5. Ensure a global sticky failure gates head and named-graph metadata after a
   fault in any layer. The evidence must identify the pinned head/removal state
   and source descriptors, not merely the shared routing token. Document
   whether head residency consumes page/byte charges separately from sealed
   layer I/O; whatever law is chosen must appear consistently in receipts and
   limits, with no hidden per-layer allowance.

No runtime test is demanded at this planning phase. No change to user scope,
authorization, merge workflow or repository law is needed to implement the plan.

## Deferral scan and final disposition

Scanned the authoritative plan for the workflow's stub/deferral vocabulary;
there were no hits. Its declined alternatives are supported by the issue's
actual consumer-owned scope and by the mechanism's operational invariants.
No unsupported downgrade or hidden unfinished requirement was found.

VERDICT: PASS

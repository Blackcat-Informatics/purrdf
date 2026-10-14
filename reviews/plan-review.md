<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent complete capacity and sparse-stack plan review

VERDICT: PASS

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32/.stage/typed-capacity-errors-instead-of-u32/plan.md`.
This is one focused Stage 1 review, including the completion-adversary plan
rubric. It is not implementation acceptance or executed qualification. No source,
Git state, forge state or test/build process was changed for this review.

## Inputs and governing law

Read the full captured `issue.md` and `issue-495.md`, including the host-identity
comment; `issue-analysis.md`, `prior-art-assessment.md` and the authoritative plan.
Read the actual root `.baseline` and `.goals` because those files are absent from
this isolated worktree, and the applicable AGENTS instructions. Read backend
contract G8–G11 and inspected the existing `DatasetView`/`ViewTermId`, global
dictionary, paged publication and Datalog ID homes.

The governing constraints include `.baseline`'s “No DEFFERALS - FOLLOWUPS - LEFT
FOR FUTURE - work is NOW, do it NOW” and `.goals`' Rust-first, RDF1.2, maximal
utility/performance/portability requirements. AGENTS requires one engine, one
behavior, native shared homes, typed hard failures, normal hooks and an isolated
worktree. The plan preserves those constraints. Its two-issue boundary retains
the other four complete contracts elsewhere; it does not claim that interfaces
alone close the sparse-stack contract.

## Full requirement coverage

| Requirement | Planned producer/consumer and falsifiable acceptance | Judgment |
| --- | --- | --- |
| Every listed local capacity site and public propagation | T1 enumerates term, arena, quad, permutation, delta, import/merge scope and provenance spaces; production counter checks plus public boundary errors, healthy neighbors and rollback | Mapped; no planned panic substitute. |
| Non-test capacity census | T1/T3 require a Rust census over the actual core/Datalog growth homes, distinguishing exhaustion from genuine invariants | Mapped. |
| Global text beyond four GiB | One checked local/global range and append home; real synthetic arithmetic crosses u32, global succeeds, local refuses, terminal overflow preserves state | Mapped; synthetic arithmetic is explicitly permitted by the issue. |
| Wide Datalog entity spaces | Every native brand becomes NonZeroU64; logical minting is separated from checked physical usize addressing; compile/store/evaluate consume it | Mapped; a wide host term type does not excuse internal row/partition widths. |
| Host identity and dictionary path | Associated Datalog carrier/dictionary plus the existing generic evaluator; actual scans retain caller IDs, triple IDs, nonresident/deleting dictionary and certified value ordering; instrumented zero-remap controls | Mapped; confidence ordering is numeric ordering, and raw IDs are not treated as a certificate. |
| Header-only open of 1,000 sources | T2/T3 use actual certified providers and count all reads, requiring zero term/reverse/quad decoding | Mapped; not a test-only wrapper around eager publication. |
| Snapshot cost independent of total term population | T2/T3 require fixed-descriptor/head tenfold-term and tenfold-historical-removal publication work/retention controls | Mapped; R1 discharged by immutable sealed carriers and complete-call measurement. |
| Chronological effective-set semantics | T2 queries duplicates, removals, insert/remove/insert, cross-source reifiers, orphan annotations, empty graphs and deep terms against eager sealing | Mapped to the original G11 classification and ordering law. |
| Whole-operation guarded reads and publication | One source budget and first typed failure, original addresses, skipped/empty descriptor verification, head/metadata latch and final readiness | Mapped to G8–G11; iterator exhaustion is not readiness. |
| Local replacement and lexical purge | T2/T3 rewrite affected segments, keep unrelated bytes unchanged, release originals, scan surviving bytes and dependent dictionary entries; retained old snapshots independently remain valid | Mapped; consumer persistence policy does not replace the requested read-side mechanism. |
| Packs and portability | Segmented is explicitly the sparse route; packs remain documented whole-load. T3 qualifies changed public consumers, native/portable source controls and release WASM builds | Mapped; no feature or extra engine is proposed. |
| Complete delivery | T4 requires both full contracts, actual local/hosted gates, review debt, candidate assessment, ghprsq, verified closure/archive/cleanup | Mapped; source and planning alone earn no closure credit. |

## R1 — Historical removal copying: DISCHARGED

The initial review identified the following design/cost inconsistency. The
original quoted copying policy below is historical, not the current plan.

The issue's proposed publication law is “Snapshot cost `O(number of segments +
head)`”, and its acceptance requires `snapshot()` work to be independent of the
number of distinct terms. The plan's design 4 instead says:

> It reads only headers at open and copies descriptors/head/removals at publication.

Its publication acceptance then says:

> Head/removal work is reported separately.

Copying all sealed layers' value removals at each publication can traverse and
retain their full historical term/dependency/lexical populations even when the
descriptor count and current mutable head are fixed. This reproduces the small
publication growth that the sparse mechanism is meant to remove. Separately
reporting that work does not establish the stated whole-publication bound.
G11 expressly identifies head/removal metadata as part of the old publication
cost; the replacement should preserve its chronological semantics without copying
unchanged historical tombstone payloads.

This is a concrete design/cost inconsistency, not a demand for already-executed
scale evidence. Under the plan-only completion rubric, the separate-reporting
criterion is insufficient to falsify the full publication-cost claim. It is the
only substantive finding from the initial review.

REWRITE TO:

> Freeze each sealed generation's removal/declaration metadata in an immutable,
> ownership-carrying carrier at that generation's construction or sealing. A
> stack publication shallow-pins those existing carriers and certified source
> descriptors; it copies only current mutable head values/removals. Historical
> removal strings and recursive term dependencies are not cloned, decoded or
> translated at publication. Original carriers remain valid for retained old
> snapshots and are released when their final owner is released.

Also extend the publication completeness row and T3:

> At fixed segment/descriptor count and current head, vary both retained source
> terms and historical removal entries/lexical bytes tenfold. Measure the complete
> actual `snapshot()` call's work and newly owned storage, not just its dictionary
> subphase. Unchanged sealed removal payloads must be shared rather than traversed
> or copied. Charge and report construction/sealing of new removal metadata and
> current head separately without subtracting hidden work from publication.

This amendment changes the ownership/cost mechanism, not the issue scope, RDF
semantics, caller persistence responsibility or retained-snapshot validity.

### Targeted recheck and disposition

Rechecked only the amended design 4, publication completeness row, T2/T3 and
explicit review disposition. Design 4 now freezes immutable removal/declaration
carriers once at sealing, shallow-pins them under their original native owners,
and copies only current mutable head/removals/declarations. It expressly forbids
historical payload traversal or deep copying at publication. The acceptance row
fixes descriptors and current head, independently grows historical tombstone
counts and lexical bytes tenfold, measures the complete snapshot call and does
not subtract current-head or hidden historical work. T2 implements this ownership
law; T3 performs those complete-call measurements. These concrete changes address
R1 without narrowing another requirement. R1 is DISCHARGED; no finding remains
open. No unchanged full-plan review was repeated and no executed evidence was
required for this planning recheck.

## Design, utility and completion judgment

The remaining decisions are coherent with current source: `DatasetView::Id` and
`ViewTermId::JoinKeyAtom` already support the shared generic evaluator; the raw
Datalog NonZeroU32 mint and lexical interner need a real migration; the global
dictionary still checks u32 arena totals; and paged snapshot publication still
reinterns retained dictionaries. The selected plan targets those actual producers
and callers rather than adding unused parallel APIs. Certified order is separate
from ID identity, and logical capacity is separate from physical admission.

No additional pre-emptive descope, unmapped requirement or planned-by-refusal
finding was found. In particular, retaining compact local RDF IDs with typed
exhaustion and using synthetic width-boundary arithmetic are allowed by the issue;
they are not refusals of its requested wide paths. The host path must deliver
unchanged IDs into real evaluator and Datalog joins, as the plan already requires.

The complete amended plan passes this Stage 1 design/completeness review. Its
implementation, scale, physical lifetime, portable, full-gate and hosted
acceptance remains pending; this report does not establish any issue closure or
permission to bypass those checks.

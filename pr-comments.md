# PR #529 — comments and review threads

4 comment(s). **Every one needs a disposition** in
`review-debt.md`: addressed (with the commit hash) or declined (with the
reason posted in reply). Bot comments count.

## 1. coderabbitai — 

<!-- This is an auto-generated comment: summarize by coderabbit.ai -->
<!-- review_stack_entry_start -->

<a href="https://app.coderabbit.ai/change-stack/Blackcat-Informatics/purrdf/pull/529?scope=ghh_OusAJGA4HG8ljoOV0wJvVx8NmnZIjc2DdjmxOu0xZWA&amp;cs_source=review_comment"><picture><source media="(prefers-color-scheme: dark)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui-dark.svg?v=3"><source media="(prefers-color-scheme: light)" srcset="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3"><img src="https://storage.googleapis.com/coderabbit_public_assets/review-stack-in-coderabbit-ui.svg?v=3" alt="Review in Change Stack →" height="32"></picture></a>

<!-- review_stack_entry_end -->
<!-- This is an auto-generated comment: rate limited by coderabbit.ai -->

> [!WARNING]
> ## Review limit reached
> 
> Your organization has reached its usage spending cap. Adjust your spending cap in the [billing tab](https://app.coderabbit.ai/settings/billing?tab=usage&orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> **Next included review available in 51 minutes.**
> 
> [Check out review usage here](https://app.coderabbit.ai/dashboard/review-capacity?orgId=b268aba6-a194-4fd6-8ca5-f1a13a313209).
> 
> <details>
> <summary><strong>View limit details</strong></summary>
> <dl>
> <dd>
> 
> **Limit details:** You’ve used the included review currently available. Your 103 included PR review attempts over the past 7 days set your current allowance at 1 review per hour.
> 
> [Learn how review limits work](https://docs.coderabbit.ai/management/plans#rate-limits).
> 
> **Review configuration:**
> 
> <details>
> <summary><strong>⚙️ Run configuration</strong></summary>
> <dl>
> <dd>
> 
> - **Configuration used**: defaults
> - **Review profile**: CHILL
> - **Plan**: Team
> - **Run ID**: `9e1c0e19-0d06-420f-830b-7be600f32b72`
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📥 Commits</strong></summary>
> <dl>
> <dd>
> 
> Reviewing files that changed from the base of the PR and between 45d1fb09ff68d2e150b7453621d0a9501ed6748d and 8b7cb55424079be3ca70e47a551e03ef458036a7.
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>
> 
> <details>
> <summary><strong>📒 Files selected for processing (3)</strong></summary>
> <dl>
> <dd>
> 
> * `Makefile`
> * `docs/WASM_TESTING.md`
> * `docs/design/purrdf-simd.md`
> 
> </dd>
> </dl>
> </details>
> 
> 
> <hr>
> 
> </dd>
> </dl>
> </details>

<!-- end of auto-generated comment: rate limited by coderabbit.ai -->

<!-- walkthrough_start -->

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>

<details>
<summary><strong>📝 Walkthrough</strong></summary>
<dl>
<dd>
<dl>
<dd>

## Walkthrough

The Turtle renderer now uses a work list to emit nested content and combines base, reifier, and annotation rows when deriving rendering structure. New tests cover deep chains and related serialization cases, and a benchmark measures rendering at three chain depths.

### Changes

**Turtle rendering**

|Layer / File(s)|Summary|
|:---|:---|
|**Renderer inputs and blank-node ordering** <br> `crates/rdf-core/src/turtle_render.rs`|The renderer combines base, reifier, and annotation rows. It identifies inline blanks and uses bounded structural keys and value-based tie-breakers for ordering.|
|**Work-list rendering and collections** <br> `crates/rdf-core/src/turtle_render.rs`|The renderer schedules subjects, properties, objects, quoted triples, and collections for emission through a work list. Indentation saturates at level 40.|
|**Serialization tests and benchmark** <br> `crates/rdf/tests/support/turtle_chains.rs`, `crates/rdf/tests/turtle_chains.rs`, `crates/rdf/benches/turtle_chains.rs`, `crates/rdf/Cargo.toml`, `Makefile`, `docs/WASM_TESTING.md`|The tests check deep-chain receipts, round trips, deterministic output, indentation, and blank-node and collection cases. The benchmark measures rendering at depths 8, 100,000, and 1,000,000. The WASM recipe and case table include the new tests.|

<!-- change_assessment_start -->
**Priority:** ➖ Normal





**Estimated code review effort:** 4 (Complex) | ~45 minutes

<!-- change_assessment_commit:"45d1fb09ff68d2e150b7453621d0a9501ed6748d" -->
**Change:** Bug fix · **Severity of issue fixed:** Medium
<!-- change_assessment_end -->

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<hr>

</dd>
</dl>
</details>

<hr>

</dd>
</dl>
</details>

<!-- walkthrough_end -->
<!-- final_review_risk_start -->
**Merge Risk:** **🔵 Low** · up to `45d1f`
<!-- final_review_risk_coverage:{"sourceCommitId":"45d1fb09ff68d2e150b7453621d0a9501ed6748d","coveredCommitId":"45d1fb09ff68d2e150b7453621d0a9501ed6748d","kind":"reviewed"} -->

The renderer change appears sound. The WASM testing guide still reports old test totals and omits the new Turtle-chain byte-identity target. Updating that documentation is a small follow-up and does not block merging.
<!-- final_review_risk_end -->
<!-- pre_merge_checks_walkthrough_start -->

<details>
<summary><strong>Pre-merge checks | <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-passed-14-v1.svg" width="14" height="17" align="middle" alt="Passed" title="Passed"></picture></sup> 4 | <sup><picture><img src="https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-counter-failed-14-v1.svg" width="14" height="17" align="middle" alt="Failed" title="Failed"></picture></sup> 1</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

### ❌ Failed checks (1 warning)

|     Check name     |                                              Status                                              | Explanation                                                                                                                                                                                               | Resolution                                                                         |
| :---------------- | :----------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| Docstring Coverage | ![Warning](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-warning.svg) | Docstring coverage is 41.94% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 31 functions across 4 files. (3 skipped: … | Write docstrings for the functions missing them to satisfy the coverage threshold. |

<details>
<summary><strong>✅ Passed checks (4 passed)</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

|         Check name         |                                             Status                                             | Explanation                                                                                                                                                                                               |
| :------------------------ | :--------------------------------------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|      Description Check     | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | Check skipped - CodeRabbit’s high-level summary is enabled.                                                                                                                                               |
|         Title check        | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The title clearly and concisely describes the main change: iterative rendering of deep Turtle blank-node chains.                                                                                          |
|     Linked Issues check    | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The PR addresses all coding requirements in issue `#472`. `crates/rdf-core/src/turtle_render.rs` replaces recursive subject, property, object, collection, and triple-term rendering with a heap work list… |
| Out of Scope Changes check | ![Passed](https://storage.googleapis.com/coderabbit_public_assets/pre-merge-checks-passed.svg) | The changed Makefile, renderer, RDF manifest, benchmark, shared test support, integration tests, and WASM testing documentation all support issue `#472`. The summary shows no dependency, public API, nam… |

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<details>
<summary><strong>Full details: Docstring Coverage</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

**Explanation**

Docstring coverage is 41.94% which is insufficient. The required threshold is 80.00%. Docstring coverage is scoped to functions touched by this diff. Analyzed 31 functions across 4 files. (3 skipped: 3 unsupported.)

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<!-- pre_merge_checks_walkthrough_end -->
<!-- finishing_touch_checkbox_start -->

<details>
<summary><strong>✨ Finishing Touches 💡 2</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

<!-- finishing_touch_suggestion:docstrings -->
<details open>
<summary><strong>📝 Generate docstrings 💡</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

- [ ] <!-- {"checkboxId":"3e1879ae-f29b-4d0d-8e06-d12b7ba33d98"} --> Commit to this branch
- [ ] <!-- {"checkboxId":"7962f53c-55bc-4827-bfbf-6a18da830691"} --> Create a new PR

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>
<!-- finishing_touch_suggestion:resolve_merge_conflict -->
<details open>
<summary><strong>⚔️ Resolve merge conflicts 💡</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

- [ ] <!-- {"checkboxId":"c3a5b2e1-4d7f-4a8c-b9d6-e1f2c3d4a5b6"} --> Resolve merge conflict in branch `paudley/472-turtle-writer-deep-blank-node-chains`

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>
<details open>
<summary><strong>🧪 Generate unit tests (beta)</strong></summary>
<dl>
<dd>
<dl>
<dd>
<dl>
<dd>

- [ ] <!-- {"checkboxId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Commit to this branch
- [ ] <!-- {"checkboxId": "f47ac10b-58cc-4372-a567-0e02b2c3d479", "radioGroupId": "utg-output-choice-group-unknown_comment_id"} --> Create a new PR

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>

<hr>

</dd>
</dl>
</details>

<!-- finishing_touch_checkbox_end -->

<!-- autopilot:start -->
- [ ] <!-- {"checkboxId":"2708ad07-9f24-4260-9c11-7dc76a49f2e3"} --> <strong title="Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts">Autofix</strong> · Keep fixing CodeRabbit findings and required CI, and resolving merge conflicts
<!-- autopilot:end -->
<!-- tips_start -->

---




<sub>Comment `@coderabbitai help` to get the list of available commands.</sub>

<!-- tips_end -->

## 2. paudley — 

# Stack-safe structural Turtle and TriG output

Complete the original structural renderer's blank-node walk with a heap work
list and retain its permanent structural-key depth guard. The public contract
requires actual 100,000- and 1,000,000-deep blank chains to complete natively and
on wasm32 with identical bytes, shallow packed output unchanged, deterministic
pack handling, and a benchmark. Main remains protected; the old dirty
stack-safe-evaluator worktree and its scratch example remain preserved.

Public acceptance names the packed render/render_canonical_turtle surface and
the separate native serialize_dataset Turtle/TriG surfaces. TriG includes a
named-graph chain and must preserve graph identity. Deep native execution uses
the ordinary default thread stack, with no on_stack size override.

## Context and prior attempts

The current kernel renderer recursively alternates render_props/render_object;
its content-key guard is still40. Preserved turtle_render source contains an
explicit Emit walk, cycle/shared/quoted blank classification and capped indentation,
but is unreviewed and has no qualifying current execution. Reconcile its source
with current native homes; use purrdf_lex::walk::WorkList rather than inventing a
second traversal container. Keep the original literal/IRI/term writer and fixed
hashers. Source-only work is not closure. No semantic features, dependencies or
namespace defaults are added. The baseline's no-deferrals rule applies; no new
production issue references or process prose belongs in shipped documentation.

The Stage brief captures the complete two-comment preservation record and related
serializer/helper issues; those old patches are donors, not acceptance evidence.
Known recurrence is the tool's captured trailer census; do not infer absence from
its finite search. No governing ADR is cited by the issue. Repository goals require
maximal utility/performance/portability and RDF1.2 preservation.

## Task 1: Complete the coherent renderer and acceptance source

Reconcile the preserved native output walk against current main, retaining both
source sides. Remove recursive property/object/collection output paths in favor
of the one WorkList emission body. Evaluate cycle, shared blank and quoted-term
references correctly before deciding which nodes pack. Keep the existing deep
structural-key guard permanently; ensure the guard applies deterministically and
does not silently drop triples. Avoid quadratic indentation/output or repeated
whole-output copies for deep chains. Confirm any remaining key recursion is
physically bounded by the preserved guard, including nested quoted terms.

The finalized kernel enforces events::MAX_TERM_NESTING_DEPTH16; preserve this
existing term law alongside the blank-key depth40 guard. Indentation remains
byte-identical to the original through level40, then saturates at that established
guard. Do not copy the donor's independent32 ceiling. Capture exact original
31/32/33/39/40/41 neighbor output; the41 case records the deliberate linear-output
indentation change. Keep shallow packing and every statement unchanged.

Write a shared Rust native/portable public corpus through the actual structural
render entry and actual Turtle/TriG serializer routes. It must exercise both deep
sizes, shallow packed goldens, cycles/shared references, RDF1.2 triple terms and
statement metadata, deterministic graph/interning permutation, and pack-guard
neighbors. Preserve every input statement. Compare frozen digest/length receipts
for the exact deep outputs on native and wasm; compilation alone is not runtime
parity. Use the existing portable testkit runner, not a new runtime. Add a report-only
benchmark under the existing first-party harness measuring actual render work at
shallow/100k/1M sizes. The old unrelated scratch example is not shipping evidence.

After complete source, run affected strict all-target compilation and native
renderer/serializer tests, actual portable deep controls and the benchmark. Fix
concrete findings coherently in their original homes. Reuse unaffected evidence;
perform the required single full qualification when the complete source settles.
Run metadata only if actual registered artifacts require regeneration. Obtain
independent implementation/completion judgment, then commit/push the intended
files normally with configured hooks/signing. Preserve unrelated work. Update
the issue with exact results; no partial-success claims or skipped gates.

## Task 2: Publish and integrate the complete contract

Publish a PR closing the issue only after all acceptance is proven. Record source,
native/portable/benchmark/full qualification and independent verdicts in validation.md.
Complete Stage2 feedback remediation and current-head hosted CI; Stage3 assesses
the actual clean integration tree and reuses qualifying affected evidence. Prepare
selected Stage archive and notes, merge only with the required ghprsq, verify issue
closure and archive, then clean the owned new worktree/branch. Keep the preserved
donor until its unrelated work is separately handled. Completion means merged main,
closed issue and owned cleanup; planning/source/checks count no closure.

## Completeness map and alternatives

Deep native/wasm output and byte parity map to Task1 shared public controls and
final CI; shallow packing/guard correctness map to Task1 exact goldens, cycle and
guard-neighbor inputs; the bench maps to Task1's actual existing harness; protected
publication/cleanup map to Task2. Replacing only the parser, lifting the permanent
guard, running with an oversized thread stack, truncating output, or asserting
only kernel unit success cannot satisfy the contract. Combining this contained
renderer delivery with the independent regex matcher is declined because their
distinct source and acceptance would hold an otherwise complete renderer behind
a large multi-host regex campaign. Top-tier writers and ready452 integration
continue concurrently; this is preserved middle-tier work, not recency priority.

## 3. paudley — 

The deep Turtle renderer and required local acceptance are complete in pushed
commit45d1fb09f. Normal commit hooks passed. Packed properties and collections
emit through the native heap work list; guard40 stays permanent, original actual
indentation through40 stays exact, and deeper indentation saturates at160 spaces.
Cycles/shared/quoted references, metadata and named graphs are preserved.

Actual100k/1M inputs pass over packed output, native Turtle/default and TriG/named
routes on the default native stack and optimized wasm runtime. All six frozen
byte-length/BLAKE3 receipts match. Exact neighbors31/32/33/39/40/41, reversed
interning/rows including named TriG, cycles/shared/quoted blanks,100k collection,
guarded ties and multi-object continuation controls pass. Direct key Eq/Ord
coherence also passes. No dependency, semantic feature, namespace default or
enlarged stack was added; the events16 law is unchanged.

Required full make check PASS: original session95104 terminal0, including strict
compilation, static/hygiene/generated checks, native workspace tests/doctests,
standalone preserve-order consumer and optimized wasm release build. The new
public corpus passes7/7 inside the full gate in11.73s, and affected strict/native/
portable correction suffixes pass. Independent final task review is PASS. The
first-party benchmark measured8/100k/1M with zero failures; medians4.645µs,
236.3ms and2.928s are report-only observations.

Confidence is lowest in extrapolating benchmark medians under concurrent host
load. What callers should know: indentation saturates beyond actual level40;
the captured original shallower output, terms and punctuation remain unchanged.
Hosted feedback/CI, actual integration, ghprsq merge and issue closure remain
pending and are required before this issue is done.

## 4. paudley — 

fix(rdf): render deep Turtle blank-node chains iteratively

Closes #472

## What changed

Deep structural Turtle rendering now uses the original shared heap WorkList
instead of recursive property/object/list output. Indentation saturates after
forty actual levels; shallow bytes and the forty-level structural-key and
sixteen-level quoted-triple guards remain unchanged. Cycles, shared/quoted
references, strict collections, named graphs and statement metadata retain
their original behavior. Equality uses the same bounded key/identity law as
ordering; singleton objects avoid constructing unused keys.

One native renderer, original physical ownership, byte determinism and portability
satisfy the standing goals. No dependency, semantic feature, vocabulary default,
larger stack or alternate renderer was introduced. All accepted requirements
are implemented without scope cuts; preserved donor work remains separate.

Validation: original full make check PASS, including strict compilation, hygiene,
generated projections, native workspace/doctests, standalone consumer and the
optimized all-release-crate WASM build. Combined strict core/RDF all-target suffix
PASS; native and optimized portable corpora each PASS7/7 at actual100k/1M sizes.
All six frozen packed/default Turtle/named TriG length/digest receipts match.
Original-neighbor captures, key coherence, cycles/shared/quoted controls,
100k collections and metadata reparse/rewrite pass. Report-only benchmarks
PASS3/3, observed medians4.645us/236.3ms/2.928s under concurrent host load.

All required final-head CI, Docs and CodeQL checks PASS, including native
integration, portable/package, conformance, architecture, workspace, seven
assembly configurations plus aggregate and book. Conditional deployment and
profile/projection jobs are SKIPPED, not execution passes. Normal hooks PASS.
Distinct completion and review-debt audits PASS. Stale portable totals and the
omitted benchmark registry entry were fixed and qualified; the published reply
was accepted and its thread resolved. No open review findings remain.

The authoritative plan is plan.md in .stage/turtle-writer-deep-blank-node-chains.
Archive that complete selected directory with these notes: final audits, raw
logs, original captures, benchmark estimates and cleanup report. Both deficiency
ledgers are clear. Donors and recovery archives remain preserved.

## Commits squashed (3)

- `8b7cb5542` Register Turtle rendering benchmark against its shared IRI escape site
- `6a5913485` Describe all registered portable semantic execution targets
- `45d1fb09f` fix(rdf): render deep Turtle blank-node chains iteratively

## Files

```
Makefile                                  |   6 +
 crates/rdf-core/src/turtle_render.rs      | 526 +++++++++++++++++++++++-------
 crates/rdf/Cargo.toml                     |   8 +
 crates/rdf/benches/turtle_chains.rs       |  36 ++
 crates/rdf/tests/support/turtle_chains.rs |  35 ++
 crates/rdf/tests/turtle_chains.rs         | 265 +++++++++++++++
 docs/WASM_TESTING.md                      |  18 +-
 docs/design/purrdf-simd.md                |   1 +
 8 files changed, 769 insertions(+), 126 deletions(-)
```

## Conflicts

The Makefile conflict was resolved by evaluating and retaining both complete
portable registration lists. Combined strict/native/portable qualification
covers that synchronization. Base lexical Memory/Tarjan changes do not alter
the renderer's WorkList path; original unchanged full-gate, capture and benchmark
evidence remains applicable. The clean assessed integration tree is
848b382fe8fbd9177d2ef82ffdacc60103f9d0a7.

---

Defect-Class: none


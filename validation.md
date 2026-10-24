# Current acceptance

## Current definition-bound refinement

Published head cd42aa245 is superseded by committed/pushed refinement de2095b46.
Normal commit/push hooks pass in definition-commit.log and definition-push.log.
Ambient-domain-reproduce.log proves that model-plus-name still aliases distinct
inline bodies under an ambient owner. Definition-identity-native.log preserves
the subsequent counted-sharing failure: borrowed parser handles were not resolved
to their actual shared definition. Both are repaired in current source.

The existing memo now keys the original model and actual evaluated definition/body
addresses, plus unchanged focus/depth. ShapePlan::definition resolves owning and
context-bound reference handles through the one native resolver. Handles with
local deactivation/annotations remain distinct. Addresses never enter artifacts.

| Requirement | Current evidence |
| --- | --- |
| Public original-body correctness | definition-reports-final.log passes all 12 catalog cases, including and/or/xone, both orders, original/unowned bodies, ambient owners, named/blank declarations, interned/foreign focus, input drop and handle-local deactivation. |
| Actual useful sharing | definition-bound-native.log passes all three counted memo controls: shared named definition once, independent anonymous bodies twice, original short-circuit/deactivation behavior. |
| Original resource bounds | The same log passes all 12 allocation controls with unchanged ceilings and the pinned report golden. |
| Product compatibility | All 28 product-model census controls pass, including the unchanged current stage identity. The changed private key and runtime resolver are outside the carried derivation closure. |
| Reports and callers | definition-reports-final.log passes all 24 complete-report, two foreign-focus and 14 logical-argument controls. |
| Strict callers | definition-strict-final.log passes Shapes/validate/Python --all-targets with -D warnings. The prior strict log rejected one redundant test clone; fixed normally. |

There are 95 distinct focused native controls across these selections. The final
test-only clone removal and comment clarification do not invalidate the unchanged
allocation/sharing/census results; the edited catalog test was re-executed.
Hosted run 40056143633 completes successfully on de2095b46 (46 jobs); the full
PR check rollup has no unsuccessful entries. The original workspace checkout log
confirms the assessed merge 69077f350, tree baa4fca159, against main 6c9c66fcf.
Current-head independent review is complete, kind reviewed, with no actionable
findings. Paginated reviews/inline comments and complete GraphQL threads are empty.
Normal commit/push have completed without bypass. No upstream work is authorized or performed. Parent mode,
solver, corpus and host contracts remain open. No owned Cargo check is live now.

## Historical model/name candidate (superseded)

Committed normally as cd42aa245, based on main e3eb29225. Reproduce.log exits 101
on the actual public conjunction: individual exported owners answer correctly,
but combined interned focus returns true instead of false. Identity-candidate.log
exits 0 after adding the model domain and borrowing the original name Term.
That case covers logical and/or/xone, encounter order, named/blank declarations,
interned/foreign focuses, result existence and original report identities.

The latest test also exercises actual unowned bodies, whose meaning is not
fixed by a shared catalog. Catalog-memo-allocation.log passes all 11 catalog, 12 original allocation and
three counted memo controls (exit 0), including the unowned-body extension and
actual once-versus-twice sharing and short-circuit/deactivation controls. Original
ceilings remain untouched. Strict Shapes/validate/Python all-target Clippy passes
with warnings denied in strict-callers.log (exit 0). Report-foreign-logical.log passes all 24 complete-report, two foreign-focus and
14 logical-argument controls, exit 0. All native and strict command handles are
terminal. The normal commit hooks pass (commit.log, exit 0); push is live.

Independent review, hosted CI, integration and cleanup are pending. No PR or
completion claim. No LFP/GFP solver, parser mode, host capability, wire change,
timing claim, external dependency change or semantic WASM execution is included.

All Cargo invocations use /home/paudley/stage/root/bin/cargo with eight build
jobs and four test/Rayon threads. Existing sibling processes and donors remain
preserved. Project memory was recalled for this domain; current source is the
authority over the older preparation milestones.

Product census source was inspected: it binds reachable declarative model types
and selected plan.rs class-analysis derivation bodies. The private run-scoped
MemoKey is outside that closure and no bound derivation source changed. No product
stage or frozen format artifact needs regeneration for this private cache repair.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Area-scan gate source review

Read-only review on 2026-10-12 of main `dd79ef77e` and the isolated 536
worktree. The inspected `topology/relate.rs` files are byte-identical. No
production change or build/test was performed. The recorded Clippy failure
remains real; this report is not a passing gate or completion of all 533 work.

## Finding and smallest coherent correction

`crates/geo-kernel/src/topology/relate.rs:569::scan_for_areas` has eight
arguments and triggers the workspace's seven-argument Clippy limit. The two
actual callers are `relate_inner` at lines 331 and 367. Both pass precisely the
same scan inputs; their positions intentionally differ: singleton pairs scan
area before fragments/events, while ordinary pairs scan after those witnesses.

**Recommend a function-local `#[expect(clippy::too_many_arguments, reason = …)]`
on this one function, with its actual semantic reason.** No body, signature,
caller, assertion, algorithm or limit needs changing. A suitable reason is:

> The complete-matrix scan keeps both source geometries, noded events and edges,
> the output matrix, and independent area and singleton witness policies explicit
> under one admitted work owner.

This is a genuinely justified local exception under the repository's preference
for fixing code before adding an allow. It preserves a small, readable algorithmic
boundary; the argument count itself is not an observed behavioral defect.
Use `expect`, not a blanket module/crate allow: its unfulfilled expectation will
surface if a later signature no longer needs the exception. There is no need to
change Clippy thresholds, lint configuration, harnesses or workflows.

## Why existing state should stay separate here

- `a` and `b` are an ordered geometric pair; their order fixes matrix rows and
  columns. Combining them in an unnamed tuple solely to reach seven arguments
  hides that relationship at these already clear callers.
- `events` and `edges` are different products of the existing noder. The scan
  borrows both; their arrays and ownership/release remain in `relate_inner`.
- `matrix` is the mutable output; `work` is the sole arithmetic/array admission
  owner shared by resident and admitted paths. Combining either into a new scan
  object would add a lifetime/borrowing layer without repairing a defect.
- `AreaTargets` (lines 546–566) records which 2D cells can be witnessed and
  stops only when those cells reach dimension two. `PointTargets` (lines
  455–495) records singleton orientation and conservative interior/boundary
  dimensional ceilings for the larger algorithm. Within the area scan its
  presence/orientation controls `locate_positive`, ensuring a face sampled at
  the singleton itself contributes to that singleton's exterior. These are
  distinct witness laws, not interchangeable flags.

No existing shared semantic scan-state type is present to pass instead.
Introducing a wrapper solely around these two policies, moving the singleton
policy into `AreaTargets`, or grouping unrelated parameters to appease the count
would obscure their different scopes. A newly named full scan-state structure
could be a valid future design if actual reuse demanded it, but is unnecessary
for this gate correction and is not recommended here.

The nearby `topology/arrangement.rs:65` and `:312` use function-local
`too_many_arguments` expectations for real decomposition/publication boundaries.
Their reasons specifically discuss distinct output/temporary ownership counters.
That is a relevant precedent for a narrow semantic exception, but its wording
must not be copied here: this function has one `work` owner, not those counters.

## Preservation and required verification

An attribute-only correction leaves band construction, settled checks, scan
ordering, mean/crossing arithmetic, `persistent_rat`, copied ordinate admission,
positive-dimensional classification, boundary assertion, matrix raises, and
crossing/band release exactly unchanged. It adds no allocation, copied geometry,
new work charge or error route. Both call sites must remain at their original
positions. This also preserves invalid/degenerate handling and the singleton
sample-equals-point law already covered by the existing tests.

Verify the actual patch is only this justified attribute; run the normal format
check and strict affected-package/all-target Clippy as part of the parent's
already-required verification, then continue the required full gate. Do not
bypass the gate or assert it will pass before execution. An attribute-only patch
does not require new tests or repetition of expensive correctness corpora solely
because the lint was documented.

If the implementation instead changes any production grouping/body/call order,
reuse the existing focused controls before broader gates:
`native_matrix_owner_preserves_every_original_golden_and_relation_family`,
`all_small_map_pairs_complete_in_one_default_work_budget`, and
`native_planar_matrix_prices_actual_arrays_and_limb_owners_until_destruction`.
Those protect answers, the cumulative 49-pair default budget and actual admitted
ownership/refusal. Their existence is not a current-run pass.

This bounded correction can accompany 536's gate dependency without representing
private-corpus completion or any other remaining 533 acceptance. It does not
depend on the private 31-bundle run.

## Actual patch disposition

Reviewed the isolated 536 worktree's actual single-file `git diff` on 2026-10-12.
It adds exactly four lines: the function-local `expect` and the recommended
semantic reason immediately before `scan_for_areas`. The signature, body, both
call sites, assertions, and algorithm are unchanged; this diff contains no
lint-configuration change. **PASS for this bounded source correction.** No
material additional source finding was identified. The parent's strict geo
check and full gate remain pending; no compilation or runtime pass is inferred.
No reviewer build/test was run. This disposition does not establish whole-533
completion or alter 536's original requirements.

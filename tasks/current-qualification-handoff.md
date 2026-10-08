# Current integrated SHACL qualification handoff

Status: prepared, execution unrun. LUBM currently owns the sole local build lane.
The existing normal main synchronization remains resolved and uncommitted;
preserve its MERGE_HEAD and unrelated work. Root's source resolution review
passes; its runtime status remains unverified. No new branch is needed.

The complete contract remains plan.md and current-source-readiness.md. The
accepted portfolio removes upstream submission gates and the old requirement
to land the community infrastructure separately. Existing external submissions
remain untouched. This delivery is the additive dated-profile foundation;
the portfolio's later default-policy switch remains required work.

Root refreshed GitHub releases on 2026-10-08: rust-v3.0.1 is still the latest
non-prerelease, published 2026-10-03T00:59:48Z. The existing two-package semver
comparison therefore retains that released baseline. Actual semver executions
on this integrated source remain required; old missing temporary logs are not
passes.

After explicit lane admission, use eight build/test jobs, a 64GiB/no-swap
scope and task-owned disk logs. Reuse unchanged main XPath/kernel evidence
where applicable; run the changed integrated callers and required gates:

Record the actual compiler envelope as well as the outer scope: normal Stage
Cargo moves its children into the shared stage-builds slice. The live LUBM gate
revealed that slice's48GiB RAM/8GiB swap limit, so outer Swap0 alone does not
prove compiler no-swap enforcement. Preserve existing global policy and siblings;
do not silently claim a resource limit from the launcher scope alone.

- Full affected packages: purrdf-shapes, purrdf-validate,
  purrdf-sparql-conformance and purrdf-conformance-kit, locked, with warning-denied
  all-target clippy and rustdoc. Preserve exact corpus/XPASS counts.
- Run the actual community-conformance binary with its two positional arguments:
  corpora/community and a fresh nonexistent output directory. Its source writes
  records.json, earl.nt and scoreboard.txt and exits nonzero for any failed or
  unsupported applicable execution. Check all64 SHACL entries, REC57/WD58,
  115 executions and535 observations against actual records, without treating
  another-date inapplicability or an explicit unknown-profile probe as a pass.
- Cover actual invocation admission, prepared governors/source precedence,
  selected XPath, current C ABI options, reusable ShEx exact validation and
  asynchronous ambient-state transport from the integration review.
- Run actual semver-checks for Shapes and Validate against rust-v3.0.1 using
  the installed tool and additive minor comparison. Preserve real logs and
  actionable failures; no installation or compatibility waiver.
- Recover the required matched report-cost evidence through the existing
  Shapes shared_views bench group shacl_complete_reports. Its production source
  asserts128 results for both Core and SELECT, then measures legacy/complete
  cold binds and warm reports. The allocation test is complete_reports::
  warm_complete_core_allocation_count_has_no_per_focus_growth; capture its
  actual N=16/N=32 allocation output, not merely a test-name inventory.
- Run current default Python report/product/annotation and optimized public
  Node package SHACL/async paths. Builds alone do not prove runtime compatibility.
- Run the affected combined helper/layer/terminal/parser-drop/thread-local,
  generated/interface and document gates. Apply the owning PO extraction/update,
  native glossary and real render; frozen external license whitespace stays
  verbatim. Update counts from actual integrated outputs.

The original accepted handoff uses targeted local qualification and fresh
hosted whole-workspace gates. Do not add a discretionary local whole-suite run
merely because old temporary logs disappeared. Normal hooks remain mandatory.
Root owns actual source publication, current PR prose, complete feedback,
independent completion adjudication and ghprsq integration. Historical green
PR checks do not qualify this changed integrated source.

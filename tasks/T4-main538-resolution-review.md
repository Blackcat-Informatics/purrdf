# Independent resolved-main composition review

Verdict: PASS for the resolved source composition only. Current runtime,
controller, telemetry and complete matched-campaign acceptance remain UNRUN /
NOT MET. This reviewer authored the telemetry patch but did not author the
manifest resolution; the resolution judgment is independent of its author,
not an independent review of this reviewer's telemetry implementation.

Reviewed branch HEAD `221b1ace8bd30e010dba6b9d732cfc9ff576a0ed`, incoming main
`5384882d65750ee22bddfeeac473095ab9c3db03`, and root-reported staged tree
`8b9c5d822f286afbbc1324f176893af5b9c2f4a4`. No unmerged entries or unstaged
tracked changes were observed. The three staged telemetry files' binary diff
against HEAD has SHA-256
`873b32d4d076b813a7509ce667d0a6dca1142ebfea0258914392e4f8ab336152`, exactly
matching the retained `tasks/T4-telemetry-source.patch`.

## Resolution and caller findings

- The CAPI manifest contains one workspace `purrdf-testkit` dev dependency with
  main's explanatory comment, and retains the complete `native_ci_profile`
  example declaration with its explicit path and `test = false`. Against main,
  the sole manifest difference is that example declaration. No duplicate key,
  lost example, semantic feature or changed CAPI library target was found.
- The staged lockfile is byte-identical to incoming main (Git blob
  `3c06dbd980fc25ce884f9cb7137acf0953c54495`). It retains the CAPI testkit edge
  and main's helper-census JSON Schema/Datalog alloc-probe changes without a
  manual lockfile rewrite.
- The staged Makefile is identical to main. Its complete six native shard routes,
  doc/examples compilation and selected-example tests, CAPI header/check/C-smoke
  callers survive. Both check and check-i18n call native `--glossary-gate`,
  retaining the render gate. The old Python glossary file is deleted coherently.
- Against the pre-merge branch workflow, the only CI difference is the workspace
  native glossary invocation. Against main, the retained optional campaign input
  and three profiling jobs remain the branch's intended addition: exact compiler
  admission, twelve fail-fast-false arms, 8-job controller build outside measured
  output roots, actual cold/warm runs, failure-preserving uploads and final
  comparison. Required native/CAPI jobs are preserved. The hook likewise retains
  main's native staged-snapshot glossary route alongside the existing shard gate.

## Applicability and limits

The prior integration assessment's material interactions now apply to the actual
staged composition: current C header/status/twins and smoke.c assertions, shared
regime-boundary fixture, rule-factor kernel, current CLI/host seams, new
integration targets and doc examples. The three telemetry paths survived byte
for byte. Their captured full metadata/source identities and per-child exact
artifact selection are still the correct consumer boundaries; no incoming source
contradiction requiring another patch was found. That source observation does
not prove compilation, strict clippy or runtime success.

Focused staged whitespace checking of the resolution, three telemetry paths,
Makefile and workflow passed. A broader staged `git diff --check` printed eleven
trailing-whitespace findings in the incoming verbatim W3C document license at
`bindings/python/licenses/third-party/w3c-xmlschema2-2e/0-W3C-document-license.html`.
Its staged blob `e1219231bec4b8c0dcae41761645203747ff2d0c` exactly equals main's;
this review did not rewrite that vendor notice or claim the broad check passed.
Owning actual hygiene/hook gates still decide admission.

The tree identity readback mistakenly invoked `git write-tree`, which is
object-writing-capable and therefore outside the requested strictly read-only
command scope. It returned root's already-existing tree identity; no index,
worktree or ref change occurred. Root was informed promptly. Other commands were
source/diff/object reads; no Cargo/build/test/commit/push or dispatch ran.

Next: root admission for current focused profiling fixtures, all-target strict
CAPI clippy, current census/workflow/shard/profile/caller gates, JSON-selected
controller rebuild and bounded real mixed doc/example and CAPI/header/nested
profile seams. Then root's normal hooks/commit/push and a complete fresh matched
hosted campaign on the committed composition. The failed 221b campaign remains
historical evidence and cannot supply new-composition timing acceptance. No
blanket local full suite is requested merely for being behind main.

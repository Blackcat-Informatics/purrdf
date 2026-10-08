# PR504 four-finding remediation — current source and evidence

Current source: staged tree `3ebb74dce49cac11b7945086c760dbc97bb62cca`, still based on signed/pushed 08e7. Owning focused native behavior and strict gates PASS with actual terminals and source readbacks. No remediation commit/push or new hosted dispatch. Independent complete remediation review, cost applicability, actual binding qualification if required and publication remain separate. The local build lane is FREE.

## Actual production changes

1. Compatibility graph selection lives in `sparql-eval::rdflib::select_query_dataset`; Python's contextual QueryMode delegates to it, while ordinary QueryMode retains its direct Arc path and empty selection type. An empty configured snapshot followed by fresh delta insertion prevents value-only unsuppression from restoring excluded original physical roles. The compatibility projection is an RDF value set, including default-only overlaps; dataset-scoped named graphs and empty declarations survive. Existing graph-name admission rejects invalid selectors; a valid missing selector produces an empty default graph without inventing a persistent named declaration. Native tests assert exact selected/query row bags, scoped blank and quoted terms, original source immutability and configured content/derivation recognition.

2. Iterative on-demand read-shape Apply always refuses as contextual, matching the recursive reference. Optional and conjunction column/provenance laws are retained. Native visitor controls cover both laws; an actual PreparedQuery/open_call_cursor refuses contextual plans and accepts the ordinary call neighbor. The differential generator now reaches Apply.

3. Iterative certainly-bound collection evaluates a nonoptional Apply driver first and transports only certified renamed inputs. Optional output promises remain left-only. The real registered bf relation uncovered two additional owning compiler/planner seams: thaw's IF hid certainty, and Apply inputs were treated as context despite runtime whole-RHS substitution through Project. The compiler-local equivalent thaw expression and existing Promise/Written category now preserve that certainty. Declared input aliases withdraw inherited RHS context and writes before actual supply is added; optional supply requires both first and retry mappings. Compact Given/Owned(NonZeroUsize) references preserve the former pointer-width per-frame payload with a compile-time assertion. Ordinary Project context narrowing is unchanged. Native controls cover renamed inputs under projection, nested inherited-input shadowing, UNDEF and optional retry neighbors; the real bf caller returns the exact registered answer while UNDEF refuses before opening the relation. Diagnostic probes and AST panic are removed. See [actual diagnosis](portfolio-pr504-bound-mode-diagnosis.md).

4. Optional Apply RHS SERVICE endpoint summaries become indirect; nonoptional summaries remain direct. The actual cached parent ServedIndex consumer agrees with fresh endpoint lookup. Apply is covered by the differential generator.

## Settled actual executions

`/opt/purrdf-454-current-qualification-20261008/pr504-shadow-rust-2` (session 73060, terminal 1) proved all owning native behavior before strict clippy failed:

| Command | Actual result |
|---|---|
| contextual unit controls | 7 passed, 0 failed/ignored |
| read-shape/spine/argument differentials and deep-stack controls | 9 passed |
| planner/collector differentials and deep-stack controls | 12 passed |
| SERVICE summary differentials/deep-stack controls | 2 passed |
| prepared_parameters | 12 passed |
| property_function_e2e | 16 passed |
| property_function_value_sources | 31 passed |
| rdflib_contextual (including new exact graph controls) | 24 passed |
| service_resolver | 13 passed |
| strict owning all-target clippy | 101, seven owning lint findings |

The exact native caller targets really executed; this is not a fixture-only or recursive-reference-only acceptance. The inherited shadow and truth-table controls also executed. Earlier genuine failures and compile/lint failures remain retained, not relabeled as success.

After terminal, the lint repair collapsed identical conditions, moved an unused test clone, replaced cursor err/expect with a typed Err match, and changed the new graph helper to borrow its Arc; its actual Python and native callers are updated. The graph projection boolean was simplified equivalently. The shared test policy constructor has a narrowly reasoned boxed-return allow because every independent fixture consumes Apply's boxed policy slot; no production suppression was added. These source changes require only the affected graph caller plus strict gates, not repetition of unchanged native semantic closures.

Session 35714 actually ended 1: all24 graph/contextual caller tests passed against the new borrowed-Arc API; strict clippy found one typed-empty assertion lint. The assertion was corrected to an exact typed empty vector, without changing behavior or production code. Session 94302 then actually ended 0: strict evaluator/Python all-target clippy, fmt and helpers/layers/terminal/build-profile/core hygiene all exited0. Its fresh complete evidence is `/opt/purrdf-454-current-qualification-20261008/pr504-final-gates-2`; driver.exit and controller exit both0, full tracked-source hash readback0, actual index3ebb unchanged, no unstaged source. Build-profile hygiene observed1039 units across two gate invocations at O3 with assertions/overflow on, across43 members. Scope now inactive/dead; live scope receipts retained separately. Outer scopes requested jobs8, MemoryMax64GiB and MemorySwapMax0; live versus unloaded properties remain distinct, and inherited compiler-wrapper behavior is not presented as absent. All earlier failures remain historical failures.

## Remaining gates

Independent complete remediation review and ordinary visitor/layout/native-cost applicability judgment; bound current Python caller qualification only if required by changed binding seam; then root's normal configured commit/push, functional review dispositions and fresh current-head hosted checks. No whole analytical twelve-row re-emission is assumed automatically: Project/Lift/worker/numeric execution kernels are untouched, while the changed preparation visitors and worklist payload require their own evidence. Selected Stage preservation/final archive and ghprsq remain separate.

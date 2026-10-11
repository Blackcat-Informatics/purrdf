This records the unresolved evidence boundary of the combined evaluator delivery for #515, #516, #511, #518 and #512. It does not reopen its canceled exclusive-timing campaign.

The user explicitly directed:

> You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work.

## Outstanding finding

The retained older candidate's 36-lane comparison rejected 20 lanes, including DISTINCT at +18.09%. That is an observation about the older artifact, not proof that the final source still has those regressions. Subsequent concrete repairs restore the ordinary row to 40 bytes and remove successful-append error construction. Five alternating fixed-work pairs on the final repaired source measure 32,191,575.742 baseline versus 30,833,871.080 current instructions per DISTINCT query (-4.217578%), with enabled and running counter times equal. No final-source throughput result follows from that instruction result. Reviewed numeric-control comparator and timed-loop bodies were unchanged, so the old control regressions have no demonstrated changed-computation cause.

Final-source throughput and physical scaling timing therefore remain unqualified. The exclusive 36-lane comparison and 78 physical timing executions are canceled acceptance work, not a required follow-up checklist and not passing results. The 19 native physical controls and all 110 scalar plus 15 SIMD portable executions did pass.

## Follow-up scope

Use reproducible observations available on the shared host, with their interference and uncertainty stated, to diagnose any remaining attributable operator cost. Preserve the original inputs and distinguish layout/allocation/instruction evidence from throughput. Correct a production hotspot only when the actual computation establishes it. Do not wait for an exclusive timing window or reinstate the canceled 36/78 campaign.

The final verification round is **FAIL**, terminal status 2. `make metadata`, strict workspace checks and the preceding hygiene gates passed. The complete no-fail-fast native/doctest collection ended with exactly one failed target, detailed below. Because `make check` stopped there, its subsequent preserve-order consumer, final core-hygiene step and nested release-crate `make wasm` were not reached in this round. Earlier portable runtime acceptance is distinct and does not turn those unexecuted steps into passes.

The retained raw receipts and source reviews will be archived with the evaluator delivery's selected Stage evidence. Link the delivery PR and archived evidence after publication.

## Final-round finding: retained cache embeds its old temporary-root path

`purrdf-testkit --test temp` reports six passes and one failure in `the_unit_test_root_is_the_directory_cargo_names_for_integration_tests`. The retained test executable derives `/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence/target/native-build/tmp` from its actual executable location, while its compiled `env!("CARGO_TARGET_TMPDIR")` still contains `/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/build/tmp`. The old path is a preserved alias whose `realpath` equals the current canonical path. The exact lexical assertion correctly fails; physical alias equivalence does not make the asserted contract pass.

The final round used canonical Cargo target/build paths, but reused this unaffected pre-relocation test artifact. Follow-up should refresh the affected artifact through normal Cargo mechanisms under the canonical path and establish that the asserted directory identity agrees, then finish the unexecuted local gate steps. Preserve the test and containment contract; do not weaken it to hide stale metadata. This failure does not establish a query/evidence semantic defect. The entire final gate remains FAIL. Raw receipt: `recovery-closed-empty-final-gates-recipe.log`, failed target at lines 25963–25983 and terminal disposition at lines 28579–28582.

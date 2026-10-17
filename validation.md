# Current validation

Scope: native anyURI datatype and downstream deletion under the authoritative plan.md. Broader GMEOW reasoning ownership remains open.

| Criterion | Evidence | State |
| --- | --- | --- |
| Parsing, normalization, primitive identity, ranges | cargo test -p purrdf-xsd --test any_uri: 3 passed | PASS |
| Existing native XSD regressions | cargo test -p purrdf-xsd: all unit, integration and documentation targets completed successfully | PASS |
| Actual WASM execution | CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh cargo test --locked -p purrdf-xsd --target wasm32-unknown-unknown --test any_uri: same 3 passed in Node | PASS |
| Admitted parsing and storage refusal | exact_governance::full_native_parser_preserves_all_value_families_and_original_heap_owners passed after adding valid, invalid and refused anyURI cases; admitted-parser.log | PASS |
| Entailment production prepass | datatypes::tests::any_uri_typing_and_equality_use_the_native_primitive passed; entail-test.log | PASS |
| Warnings | cargo clippy -p purrdf-xsd -p purrdf-entail --all-targets -- -D warnings passed; clippy.log | PASS |
| Source hygiene | git diff --check passed; all three normal configured commit hooks passed and commits pushed | PASS |
| Independent review | CodeRabbit found no actionable comments through ac4094981; explicitly rate-limited the final shared-method extraction, which passed affected checks. No final-head independent review claimed | PARTIAL COVERAGE |
| Downstream GMEOW deletion | Source edits prepared on the isolated issue 1803 worktree; native pin and focused execution await publication | IN PROGRESS |
| Full workspace/hosted CI | Not run locally; do not equate focused checks with workspace qualification | PENDING |

Integration is clean against unchanged origin/main 410a927d9e97; merge-tree candidate bc13dfb30b5a5162c78b146d0ada5ffdca9c23c1. The native source is clean and pushed. PR 573 is the upstream half of the paired cutover, so issue 572 remains open until GMEOW deletion lands. The user authorized merging with hosted CI pending; pending checks are not counted as passes. review-debt.md records review coverage, the unavailable final delta review, and the completed review's actual scope.

The XSD suite ran before documentation-only edits and the additional admission-refusal fixture; the affected admitted parser test was rerun successfully. The range engine's private length calculation was subsequently moved to XsdValue::facet_length so downstream code also deletes its string/binary counting copy: the affected anyURI and data_range targets passed (3 plus 36 cases), and XSD all-target clippy passed. The exact WASM selection is also rerun for this API extraction. Both initial commits passed normal hooks and were pushed in PR 573. stagectl pr-create reported a response-JSON parse failure after creating the PR; the REST lookup confirmed the existing PR rather than creating a duplicate. CodeRabbit is actively reviewing. No real GMEOW corpus was built, loaded or repaired, and no machine resource probes were run. No performance speedup is claimed.

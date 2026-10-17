# Validation and remaining integration work

Scope: one shared core stop signal, controlled RDFC canonicalization, cooperative recorded DL identity/preparation/search, public RDF facade, and persistent portable regression coverage. No corpus producer, host probes, new dependency, build-profile changes, semantic fallback or source filtering was introduced.

| Requirement | Evidence | State |
| --- | --- | --- |
| Existing canonical identities, RDF 1.2 overlays, read/reserved/poison refusals; immediate and mid-operation stop | `cargo test -p purrdf-core --lib ir::canon`, `core-canon-tests.log`: 61 passed | PASS; subsequent core edits only eagerly construct the identical small budget error and improve a test diagnostic |
| Recorded identity, proof replay, stopped search receipts, exact preparation-budget selection, cancellation at every observed construction poll | `cargo test --locked -p purrdf-entail --test recorded_stop`, `recorded-stop-tests-final.log`: 7 passed | PASS |
| Actual WASM execution of the same public recorded path | Cargo wasm32 target with repository Node runner, `recorded-stop-wasm.log`: 7 passed | PASS; registered permanently in `make wasm-test` |
| Existing reasoner behavior, class unknown semantics, preparation support and proof codecs | `cargo test --locked -p purrdf-entail --lib reasoner`, `reasoner-tests.log`: 93 passed, 1 ignored | PASS for selected inventory; ignored test remains unrun |
| Shared Datalog signal and chase/fixpoint stop behavior | `cargo test --locked -p purrdf-datalog --lib stop`, `datalog-stop-tests.log`: 7 passed | PASS |
| Strict affected all-target Clippy including RDF facade | `clippy-final.log`: passed in 2m 41s | PASS |
| Format | `fmt-check-settled.log`, exit 0 | PASS |
| Normal signed commit hooks | `commit.log`, commit 6fbf766a2; `base-sync-commit.log`, merge 7a84b81a5 | PASS; no bypass |
| Hosted full-suite qualification and CodeRabbit | PR not created yet | NOT RUN; do not claim local focused checks cover the whole workspace |

The initial `recorded-stop-tests.log` ran zero tests during conversion to the portable harness and is not accepted evidence. The final native and WASM logs above each show seven executed cases.

Independent dispatch is not available in this session. Reviews will be explicitly identified as self-review; no independent approval is claimed. Hosted review findings must be retrieved and addressed when available. User has authorized ghprsq integration with hosted CI pending; required hooks will still run normally.

This change is a prerequisite for downstream class delegation. It does not implement a shared-store/delta API or checked rooted source-proof resolution, and cannot establish completion of the wider GMEOW logic queue.

Integration assessment: origin/main advanced through text-index PR579 while this work was in flight. Its overlap in entail/parser, entail/report and xsd/range is documentation-only; it adds separate text-index and numeric-WASM cases. The first merge-tree output contained a documentation conflict, initially misread as clean; that claim was corrected in chat. An ordinary base-to-worktree merge now retains the updated seventeen-case regular-role row and the new seven-case recorded-stop row. Both Make additions were retained automatically. No stop, canonicalizer or constructor implementation changed upstream. The focused runtime evidence remains applicable; whole-workspace hosted qualification is still pending. The normal merge commit hooks passed, and the final merge-tree command exited zero (`merge-tree-final.txt`).

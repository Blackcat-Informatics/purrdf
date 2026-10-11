The evaluator, queryable ledger, scoped retrieval and physical shape work now ships together in PR #532, closing #515, #516, #511, #518 and #512 under the revised boundary below. This supersedes the earlier separate three-issue and two-issue delivery plans.

The user directed: "You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work."

That final round is finished. Metadata, strict checks and preceding hygiene passed. The full local gate is FAIL (terminal 2): one retained cache artifact fails testkit's exact temporary-root identity assertion; all other native/doctest targets completed. Its final preserve-order consumer, core-hygiene and nested release-crate WASM steps were not reached. The 19 native physical controls and 110 scalar plus 15 SIMD portable runtime executions passed separately. Normal commit hooks passed; hosted checks and actual integration remain pending.

Follow-up #531 records the exact failure, unexecuted suffix and unqualified performance evidence before merge. Exclusive 36-lane throughput and 78 physical timings are canceled, not passing, and are not reintroduced as a follow-up requirement. The final-source instruction reduction is a diagnostic, not throughput acceptance.

The authoritative combined plan, material reviews and exact raw receipts are retained in `.stage/sparql-eval-datalog-per-row-evidence` for separate merge evidence archival. PR #532 carries the implementation and current validation disposition.

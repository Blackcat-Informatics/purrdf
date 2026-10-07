# Final local gate PASS

Full make check passes with exit=0 in raw/final-repaired-make-check.exit. All workspace tests, hygiene, clippy and release Wasm build pass on the final production source. Hosted qualification has one Wasm SIMD job still running.

# Final-head qualification update

Current pushed head: 14622de4e8556c4bb57f740ac69c9bca27cc6d17. Production repair is cfa1f7b51; the following commit documents the measured tradeoff. No subagents were resumed after the user stopped them.

The 64-row bounded block setting and 24-byte deferred charge log reduce requested bytes and peak above-window memory versus main in all six numeric fuel/scratch lanes at 1/4/32 workers. Raw compact-block64 comparisons and all typed governor receipts are retained. The previous scratch1 and fuel32 allocation increases are resolved by this setting; previous observations remain historical, not deleted.

Plain BIND returns to exact main allocation metrics. Plain numeric requested traffic is +199,960 bytes/query, allocation calls and retained bytes unchanged. This is the explicit typed fallible group output buffer: a differently typed minted vector cannot be reused safely, and retaining oversized input capacity increases returned-result capacity. The tradeoff preserves actionable allocation failure and releases oversized input. General buffer reuse is a separate optimization; no required governor behavior is deferred.

Small BIND stays sequential below the parallel threshold. SHACL per-focus allocations improve from main 101/126 plain/governed to 85/110. Eight exact allocation/prose/violation tests and 58 governor tests pass. The lib gate exposed two tests comparing evaluation-local scratch IDs across different paths; both now compare every resolved cell in order, including unbound cells, with schema and explicit expected-value checks retained. Focused tests pass.

CodeRabbit thread 4211727541 identified selected-source hash baseline incompatibility. The frontend projects the single current testkit into each production graph, binds its hash dependency to the selected production graph, copies that graph's frozen metaschemas, and guards tooling identity for builds, reuse and prepared execution. The thread is resolved by CodeRabbit at cfa1f7b51. Clean main/candidate build proof remains running until its receipt completes.

Final-head full make check, final 30-case counts with 37 worker coverage proofs, CLI rebuild/200,000-row acceptance and wasm runtime are running. Required hosted checks are pending. These states block final merge; this update makes no completion claim.

Completed final-head checks: all 30 count cases, 37 individual instruction worker proofs, allocation coverage at 1/4/32, and full cross-artifact typed answers/GovernorEvidence match main and geometry. Numeric METERED/plain ratios are 0.936403682693084 at four and 0.5030566761343247 at 32. Bounded numeric requested traffic and median peak are below main in all six fuel/scratch lanes. The fuel32 peak interval spans no change; its median change is -3.9443%, CI [-8.9315%, +1.1611%]. Plain numeric32 peak is -2.4212%, CI [-5.4765%, +1.0159%]. These are reported intervals, not latency claims. See raw/final-head-comparisons.tsv.

Clean hash harness builds and verified artifact reuse pass for main and candidate. The review reply is raw/hash-review-reply.json. Actual numeric_wasm_determinism runtime passes both cases with frozen digest da3900a93723c4ac and corpus length 2727. An initial command selected the native numeric_parallel_determinism harness on wasm, which failed outside tests; it is retained in raw/final-head-wasm-runtime.log and does not qualify the wasm runtime. The corrected registered owner is raw/final-head-numeric-wasm-runtime.log.

The rebuilt normal release CLI passes the final 200,000-row plain/fuel/inclusive-cell full answer comparison and all-subject/value verifier. The 599,999-cell neighbor exits 3 and equals the exact verified 199,999-row prefix. Explain succeeds separately without a ceiling. An initial explain invocation combined --fuel, which the CLI correctly rejected; that original log is retained. Final receipts are raw/final-head-optional-result.log and raw/final-head-optional-explain-corrected.*.

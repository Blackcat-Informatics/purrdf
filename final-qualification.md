# Final qualification

Source commits9bc37b36a and3d62728c3 are signed and pushed with normal hooks. The latter only adds the two missing count-bench rows to SIMD documentation; it does not change the measured production code.

Fresh production governor integration checks pass27+9+11+10+1 cases in raw/final-governor-tests.log. CLI governors pass25 in raw/final-cli-governors.log. Release evaluator Wasm compilation passes in raw/final-evaluator-wasm.log. Numeric Wasm runtime passes2, frozen digestda3900a93723c4ac/corpus2727 in raw/final-numeric-wasm-runtime-retry.log. The initial invocation lacked the Node runner and did not execute; it remains in raw/final-numeric-wasm-runtime.log.

The normal release CLI200000-row OPTIONAL witness passes plain/fuel/inclusive600000-cell full-answer byte equality and the exact answer verifier; the599999-cell neighbor exits3 and matches the verified199999-row prefix. See raw/final-optional-200000-result.log and associated outputs. Initial unsupported --row-cells spelling is retained in its original log; corrected invocations use --max-intermediate-cells.

Native final-compact counts complete30 case/thread combinations, ten samples per instruction and allocation mode. Parked-worker proof passes all37 worker targets at1/4/32. Complete result/receipt guards match preserved main/geometry records. Source changes to the worker witness only affect the isolated proof; production count boundaries, event configuration and the main/control workload callers are unchanged, so their original query counts are reused after current candidate coverage succeeds. No latency equivalence is claimed.

METERED/plain instruction ratios pass0.9361560578 at4 and0.5047660472 at32. Plain BIND has exactly main's allocation calls/requested/retained/peak bytes after typed input-buffer reuse. Numeric compact marks reduce exact log entries32→24bytes; fuel1 requested bytes improve5.293% and peak31.350%; metered1 requested bytes improve2.297% and peak11.102%.

Remaining measured allocation increases are listed in review-debt.md and raw/final-numeric-comparisons.tsv. They are not represented as resolved. The full make check retry is active; the first invocation was interrupted before completion and is not a pass. Hosted checks on3d62728c3 and CodeRabbit review are pending. Prior SIMD failures on9bc37b36a were caused by the missing bench-table rows and have a committed correction.

Integration preview against fetched origin/main is clean, treeeb42f2fca94a55eee4384bacb7666fab2bd0e70b. No final merge has occurred. Stage's merge-ready presence check passes; substantive readiness remains pending the checks and allocation disposition above.

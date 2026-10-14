# Registered fixed-work DISTINCT instruction control

The protected-main baseline is `5144f5c1a7dc824f32165eca91d3c2a6df6226fb`; the current #531 bench was built from the probe-free evaluator source immediately before the resident-panic-guard consolidation. `crates/sparql-eval/benches/query_eval.rs` is byte-identical in both checkouts. Both use the registered `query_eval/h_distinct_dept` case: the original 30,000-person fixture, 200 distinct department keys, one selected query after the bench's normal warmup. The guard consolidation does not participate in that BGP/DISTINCT route.

Both binaries were produced with direct standard `cargo bench --locked -j1 -p purrdf-sparql-eval --bench query_eval --no-run --message-format=json` and selected from Cargo's `compiler-artifact.executable`. Each column below is `perf stat -x, -r 3 -e instructions:u` on the registered binary with `--test --exact`; all selected repetitions reported `1 benchmark tested; 0 failed`, and all no-match repetitions reported zero selected tests. Kernel instructions are excluded. Counter enabled/running was 100% for each output.

| Source | No-match process mean | Selected process mean | Selected minus no-match |
|---|---:|---:|---:|
| Protected main | 19,642,788,548 | 19,683,192,179 | 40,403,631 |
| Current #531 | 19,627,786,623 | 19,657,843,713 | 30,057,090 |

The nominal selected-minus-no-match estimate is 10,346,541 lower (-25.61%) on current #531. This is a like-for-like **single registered routine** comparison with process setup and all-case warmup subtracted; it is not the archived 1,000-query counter window, an exclusive timing run, or a throughput result. Baseline no-match process variation is 0.05% of about 19.64 billion instructions, around 9.8 million, comparable to the nominal difference. These independent three-run process counts therefore do **not** establish a final-source instruction win or its magnitude. The shared host also cannot establish elapsed-time improvement. Raw outputs: `T3-distinct-{baseline,current}-{nomatch,selected}.{out,perf}`; build manifest: `T3-distinct-current-bench-build-final.jsonl`. The first current build attempt was canceled on a known lint failure and is not evidence.

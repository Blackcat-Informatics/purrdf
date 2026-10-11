The first hosted assembly configuration results also expose stale generated/manual projection counts, not a demonstrated scanner-threshold failure:

| Configuration | Row | Recorded | Measured |
|---|---|---|---|
| aarch64 | eval.bgp-join-probe | v3·f0·r0 | v20·f0·r0 |
| x86_64 | rdf.projections | v66·f0·r0 | v68·f0·r0 |
| x86_64 | eval.bgp-join-probe | v7·f0·r0 | v42·f0·r0 |
| x86_64-v4 | rdf.projections | v23·f0·r0 | v28·f0·r0 |
| x86_64-v4 | eval.bgp-join-probe | v2·f0·r0 | v14·f0·r0 |

These three `simd-asm-config` jobs FAIL in [run 38181556067](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067). Retained raw logs are `hosted-asm-aarch64.log`, `hosted-asm-x86_64.log` and `hosted-asm-x86_64-v4.log`. Follow-up must regenerate the projection through the existing generator against the actual source/compiler/configuration and verify all affected configuration rows, preserving scanner thresholds and the measured implementation. Scalar WASM assembly, release-crate WASM and shapes conformance have passed in the same hosted run; other pending jobs are not inferred to pass. No extra implementation/build round was started.

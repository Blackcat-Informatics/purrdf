# Issue #478: SPARQL governor: governed parallel row loops regressed 1.2–1.65× at 32 threads (block scheduling)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

A regression from the exact-tower PR (#441, commit ebe79697a, "run a governed forked loop in small blocks taken in order").

Bench `governed_eval/row_loops_16384` (16,384 cheap rows), RAYON_NUM_THREADS=32, median ms:

| Lane | now | before blocks | main before #441 |
|---|---|---|---|
| filter, fuel + stop signal | 5.07 | 3.49 | 3.31 |
| bind, fuel + stop signal | 10.98 | 7.06 | 8.59 |
| filter, stop signal only | 4.18 | 3.57 | 3.38 |
| bind, stop signal only | 10.96 | 8.05 | 6.65 |

The ungoverned controls are flat (1.03–1.09×), and one thread does not regress.

Likely cause: about 2,048 blocks of 8 items at 32 threads, each with its own worker context, ledger clone, `ScratchInterner::over` and harvest. Blocks are used even when only a stop signal is present and there is no spend to share.

Required:
- use blocks only when a fuel or scratch headroom is engaged;
- set a minimum block length (or size blocks from the row count and thread count);
- re-bench this lane A/B against main. The memory bound (about one ceiling plus in-flight rows), determinism across thread counts and the exact trip points must all hold.
- Update docs/BENCHMARKS.md.

Also fix:
- **Stale doc:** the module doc in `crates/sparql-eval/src/row_checkpoint.rs` (around lines 65–76) still describes the retired per-prefix stopping rule.
- **False claim:** the CHANGELOG and the governor profile say a governed OPTIONAL filter still forks, but on the CLI it never does. The CLI's metered base engages the intermediate-cells dimension, so `cell_row_ceiling` is `Some`. Either make it fork or correct the claim. Measured on 200,000 rows at 32 threads: 0.6 s ungoverned against 3.1 s under `--fuel`.

Refs #423, #469.

## Comments (0)


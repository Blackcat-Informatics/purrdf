# #531 bench-profile frame readback

The unchanged `bench` Cargo profile retained WASM function names for diagnosis.
`cargo rustc -p purrdf-wasm --lib --crate-type cdylib --target wasm32-unknown-unknown --profile bench --locked --message-format=json` completed successfully with the same approved release Rust flags and default target directory. The exact compiler artifact selected from `T3-531-bench-profile-artifact.jsonl` is SHA-256 `df06f76902bd31d251d4d7749be3f8356ce52cfb06fb25f0e75cf980c58a5dd8`. This is an unlinked raw bench artifact, **not** the optimized shipping package or an acceptance run.

`llvm-objdump` prologues for `RdfDataset` monomorphs show these shadow-stack reservations:

| Function | Bytes |
| --- | ---: |
| `expr::eval_filter` | 1,488 |
| `expr::exists_recorded` | 688 |
| `expr::exists_prepared` | 3,952 |
| `binop::eval_correlated` | 64 |
| `binop::eval_substituted_delivered` | 512 |
| `eval::eval_evaluated_with` (selected monomorph) | 336 |
| `eval::eval_evaluated_inner_with` (selected monomorph) | 400 |

`llvm-readobj --symbols` maps `exists_prepared` to function index `0x4450` and `eval_correlated` to `0x4483` (17,539). The former calls the latter at code address `0x1a578c7` on the definition descent. At that call, `exists_prepared` has reserved its full 3,952-byte frame and passes a result pointer at frame offset 768. Source at `expr.rs::exists_prepared` retains the definition memo key, witness/sites, site/ledger and row context for the post-child completion. The source now places only that completion in `finish_exists_definition`, after the original ledger pop and `inner_result?`.

These frame reservations cannot simply be summed into the optimized package's measured 7,104 bytes per nested NOT EXISTS level: some selected monomorphs may not be on the repeated path, and the shipping `wasm-opt` pass may alter frames. The source correction is uncompiled. Rebuild and measure the exact optimized package slope and original 180-level floor before crediting it; if unchanged, revert and diagnose again.

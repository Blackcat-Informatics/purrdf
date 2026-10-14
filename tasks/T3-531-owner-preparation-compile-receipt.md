# #531 owner/preparation compiler checkpoint

Checkout HEAD: `330c55f8c2d99300b8375a545ef255095e23a830`.
Rust source diff SHA-256 at the terminal check:
`36b33aa0eba6df3c62f8a5b2d410b7cb2c15a5cc8227e077a7ac7b2ec5058691`.

Direct standard-directory command:
`CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib`.
The original returned tool result for session `14917` reported exit code `0`:

```text
    Checking purrdf-sparql-eval v3.0.1 (/home/paudley/Active/purrdf/.worktrees/470-v4-0-exact-tower-only-and-an-exact-or/crates/sparql-eval)
    Finished `dev` profile [optimized + debuginfo] target(s) in 8.99s
```

Earlier actual compiler-feedback sessions remain failures: `7473` returned
exit `101` with three missing `&ledger_source` callers and a missing
`FilterRowState::run` adapter; `74673` returned exit `101` for the `ebv`
expression argument; `10261` returned exit `101` for two unused pre-driver
accessors. Those were corrected or removed before session `14917`. The returned
tool outputs are summarized here as a checkpoint receipt, not presented as a
reconstructed raw log. `git diff --check` also returned exit `0`.

Scope: current evaluator production library typing only. The explicit graph
and VM child-entry/resume dispatcher, runtime behavior, allocation controls,
and all three unchanged optimized depth floors remain unqualified.

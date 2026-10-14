# Integrated graph-default execution

Executed in this delivery worktree after synchronization with main77a864e93:

```text
CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=4 cargo test --locked -p purrdf-sparql-eval --test update_default_graph_lifetime
```

Tool session32487 terminated0. Cargo compiled through the installed standard
toolchain and configured directories in the optimized test profile. Actual
execution output:

```text
running 3 tests
test omitted_update_options_retain_clear_and_empty_transfer_destinations ... ok
test omitted_governed_policy_refusal_publishes_no_declaration_prefix ... ok
test default_options_empty_load_declares_destination_and_constant_graph_sees_it ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Normal merge commit41459 terminated0 as9e87ee8a2. Its hook passed the staged
non-Rust ratchet against77a864e93; no hook or signing setting was disabled.
No test-time output is used as performance evidence.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 detached scope-restoration prerequisite

Source base `330c55f8c`, uncommitted `crates/sparql-eval/src/eval.rs` diff.
The existing recursive `EvalScopeGuard` and a future runner-owned token now
consume the same `ScopeRestore::restore` body. `begin_substituted_exists`
admits/pushes the correlated map before installing the flag and deferred
map, then returns a must-use token; the recursive guard wraps that token.
Positive-shape scopes use the same token restoration body. No continuation
runner or VM site conversion exists in this checkpoint.

Actual focused commands and results, direct standard Cargo:

- `cargo check --locked -j 4 -p purrdf-sparql-eval --lib`: terminal 0.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib temporary_positive_numbering`: terminal 0, 3 passed, 0 failed.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib detached_scope_tokens_restore_nested_map_and_shape_in_lifo_order`: terminal 0, 1 passed, 0 failed. This exercises two admitted correlated-map scopes nested with a positive-shape token and explicit LIFO restoration.

These checks establish the primitive's local behavior only. They do not
qualify runner unwind, full evaluator behavior or any #531 stack floor.

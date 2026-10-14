<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 VM site and copied-scope runner checkpoint

Base `330c55f8c`; these are uncommitted evaluator-source changes. This is a
prerequisite checkpoint, not a depth-floor result or an issue completion.

The VM now suspends an `EXISTS` site ordinal. The existing expression-operand
visitor builds admitted owner-relative paths in compiler pre-order. A link with
EXISTS retains an immutable borrow of the expression root with the paths and
their grants in one admitted shared owner; worker `fresh()` clones that owner
without cloning path arrays. Links with no EXISTS allocate no site owner.
Resolution borrows the original pattern for one step and refuses a different
expression owner. The path walk visits each expression node once, including a
wide OR/AND node, while retaining only the active DFS children and path.

The first actual copied-pattern runner caller is
`binop::eval_substituted_delivered`. It admits its one frame before installing
the detached `ScopeRestore`; its `Drop` restores scopes in reverse order before
releasing a copied tree, including during a Rust unwind. It still calls the
recursive evaluator. No iterative node or VM suspension/resume loop has yet
replaced the repeating LATERAL/FILTER/EXISTS chain. A future movable VM link
for a copied tree also needs an admitted owned expression-root handle rather
than borrowing its own frame; the current borrowed link intentionally does not
pretend to solve that self-reference.

Observed focused checks on the respective source at command launch:

- `cargo check --locked -j 4 -p purrdf-sparql-eval --lib`: terminal 0 on the
  revised shared-owner source, and terminal 0 again after the one-pass walk.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib vm::tests::`: 14/14
  passed on the shared-owner source before the one-pass walker was written;
  exact final-source replay is pending below.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib
  copied_pattern_runner_restores_scope_on_unwind_before_reuse`: 1/1 passed on
  the runner source before the shared-owner/walker revision. The first compile
  failed at an unnecessary test qualification; that was corrected before this
  passing retry.
- `cargo fmt --check -p purrdf-sparql-eval` and `git diff --check`: passed
  after the one-pass walker.
- Exact final-source replay of `cargo test --locked -j 4 -p purrdf-sparql-eval
  --lib vm::tests::`: terminal 0, 14 passed, 0 failed. It includes two ordered
  EXISTS sites, a mismatched-owner refusal, the generated expressions, and
  the existing suite-expression differential.
- Exact final-source replay of `cargo test --locked -j 4 -p purrdf-sparql-eval
  --lib copied_pattern_runner_restores_scope_on_unwind_before_reuse`: terminal
  0, 1 passed, 0 failed.
- `cargo clippy --locked -j 4 -p purrdf-sparql-eval --lib -- -D warnings`:
  terminal 0 on the same final source.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib exists`:
  terminal 0, 87 passed, including strategy parity, forced sequential/parallel,
  nested identity, truncation, ledger, and bounded depth controls.
- `cargo test --locked -j 4 -p purrdf-sparql-eval --lib lateral`:
  terminal 0, 19 passed, including nested/layered and generated substitution.

Pending affected first-error and physical refusal controls, whole
continuation integration, all three
unchanged optimized stack floors, cost/alloc controls, full required gates and
publication. The previous source-only VM review was `REVISE`; its two concrete
findings prompted the immutable borrowed owner and shared admitted site
storage, but independent refreshed disposition is pending.

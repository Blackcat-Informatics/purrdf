# Issue #1474: Meet the shared build standard: hook, gates, lint table, dependency and document checks

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Result

This repository meets one build standard shared by the Rust repositories of this organization. The standard is stated here in full, because its home is not public.

## The standard

1. **Commit hook.** The hook runs exactly `make precommit`: formatting, lints on library and binary targets, the dependency check and document checks. It finishes in under 60 seconds on a warm cache, runs no test and takes no lock. A check fails when the hook is not installed.
2. **Targets with fixed meanings.** `precommit`; `clippy` (`cargo clippy --locked --workspace --lib --bins -- -D warnings`); `clippy-all` (`--all-targets`, and each nested workspace); `check` (precommit, clippy-all, the bench build, the census and workplan gates, audits); `test`.
3. **Declared merge gate.** One command runs `make check` and `make test` on the head that merges, with its own exit status recorded. Hosted CI is an additional gate.
4. **Lint table** in `[workspace.lints]`, inherited by every member:
   - rustc: `unsafe_code` forbid (deny, with the reason stated, where the crate needs `unsafe`; each block carries a `SAFETY:` proof); `unreachable_pub`, `unused`, `dead_code` deny; `missing_docs`, `missing_debug_implementations`, `unused_crate_dependencies` warn.
   - clippy: `all` deny; `pedantic` warn; `allow_attributes`, `allow_attributes_without_reason`, `used_underscore_binding`, `todo`, `unimplemented`, `dbg_macro` deny; `unwrap_used`, `expect_used`, `panic`, `print_stdout`, `print_stderr`, `exit`, `mem_forget`, `unwrap_in_result`, `get_unwrap`, `string_slice`, `map_err_ignore`, `redundant_clone` warn.
   - Every gate passes `-D warnings`, so a warn fails the commit. A suppression is `#[expect(lint, reason = "...")]` on the smallest item. The table is a floor: a repository adds lints and lowers none.
5. **Cargo.** Every cargo command in a gate is locked and none is offline. Versions are stated once, in `[workspace.dependencies]`. `cargo deny check bans sources` runs in the hook, with `multiple-versions = "deny"` and a reason on every skip. No process but cargo reads cargo's cache.
6. **Suppliers from the registry by version**, with no Git pin and no patch table for a crate the registry holds: purrdf at one exact major, and the Cap'n Proto crates at one release.
7. **Census and workplan.** `raf-census` and `raf-workplan` from rust-agentic-foundations, as dev-dependencies only, replace a local helper census. Where purrdf is a dependency, its implementation is the home of every job it provides; a local copy is a hard finding.
8. **Output.** Libraries emit `tracing` events with structured fields and print nothing.
9. **Tests.** Each permanent test states what it protects. No test asserts wall-clock time. Tests are outside the hook and run in full on the head that merges.
10. **Documents.** A document states the current contract in the present tense, attributes nothing to a person and is rewritten in place. A text check and a link check run in the hook.

A published crate keeps its minimum-toolchain job and may deny `missing_docs`.

## What differs today

Read at main c38e0780b on 2026-11-22 from the root manifest, the tracked gate files and the hook. Member manifests and CI are not read; the owner completes the list.

- The hook runs census ratchets and policy checks, with no formatting or lint step.
- Clippy `all`, `pedantic` and `nursery` are at warn, with 26 lints allowed; `unreachable_pub` is at warn.
- No `unsafe_code` level in the workspace table, and no panic or print lints.
- No `deny.toml`.
- Its own census (`helper-census`).

## Measure first

Before the lint table changes, post here the count of hits on this tree for each of `unwrap_in_result`, `get_unwrap`, `string_slice`, `map_err_ignore` and `redundant_clone`, and for each pedantic lint the table allows today.

## Not in this issue

A move of the purrdf major, the Cap'n Proto release or the foundation's version is one event across consumers and is announced separately. Move no pin alone.

## Acceptance

- One commit on a warm cache is measured under 60 seconds through the hook.
- The lint table is the one above, every member inherits it, and each lint this repository allows is either removed or kept with its reason stated beside it.
- Every cargo command in a gate is locked; `cargo deny check bans sources` runs in the hook.
- One declared command runs `make check` and `make test`.


## Comments (4)

### paudley — 2026-11-22T17:54:38Z

Two corrections to the standard as stated above, from a reading of rust-agentic-foundations at `v0.8.0`.

**Point 2, `clippy`.** `make clippy` is the production Clippy command of the census, flag for flag:

```sh
CARGO_ENCODED_RUSTFLAGS='' cargo clippy --workspace --lib --bins --locked -- \
  -D warnings -D unused -D unreachable-pub -D clippy::allow_attributes \
  -D clippy::used_underscore_binding -D clippy::needless_pass_by_value \
  -D clippy::trivially_copy_pass_by_ref -D clippy::redundant_clone
```

The census accepts a lint receipt only from these exact arguments, so a different lint line proves a different build. `make clippy-all` passes `-D warnings` over all targets, and the Makefile adds no other lint flag.

**Point 1, the hook.** The hook runs no behavioural test. A static gate that is built as a test binary counts against the 60 seconds like any other step.

What adopting the census from the registry still lacks is tracked as Blackcat-Informatics/rust-agentic-foundations#427.


### paudley — 2026-11-22T17:54:43Z

One more item for the census part of this issue. A census ledger is one closed file with no include, so a rule such as "this job's home is a `purrdf-*` crate" is stated again in every consumer's ledger (Blackcat-Informatics/rust-agentic-foundations#427, points 5 and 6).

Wanted here: a list, in this repository's documents, of the jobs purrdf is the home of, each with its package and its entry points (hashing and hexadecimal text, the term model, text writing, lexical forms, and the rest). Each consumer's ledger then carries those jobs as written, and a local copy of one is a hard finding.


### paudley — 2026-11-22T18:29:47Z

Existing issues here on the same ground, read by title and opening text. This issue does not replace them: where one already carries a part, the work is done there.

- #1365 (deny levels in the workspace table and a stricter Clippy line): the lint table of this issue.
- #1189, #1190 and #1178 (build-policy checks, shared-helper gates, the helper census refusing incomplete discovery): the checks in `make check`.
- #1024 (helper policy with the foundation's native ledger): the census part.


### paudley — 2026-11-27T22:50:14Z

The pre-change diagnostic inventory ran on signed `52645a8e11c59a4c5f33dd586344fc13fe5b7bd2`, before the workspace lint table changed.

The managed compiler was `1.102.0-nightly` (`5df8c02af`). The inventory used the locked workspace library and binary targets with `CARGO_ENCODED_RUSTFLAGS=''` and `--force-warn` for the named lints. The command exited zero and emitted a successful Cargo completion record. These are compiler warnings emitted across targets, not counts of unique source lines or a passing strict gate. The local worktree result is provisional pending resolution of Stage's seeding issue.

| Clippy lint | Reports |
| --- | ---: |
| `unwrap_in_result` | 403 |
| `get_unwrap` | 0 |
| `string_slice` | 651 |
| `map_err_ignore` | 1541 |
| `redundant_clone` | 0 |
| `module_name_repetitions` | 939 |
| `missing_errors_doc` | 977 |
| `missing_panics_doc` | 477 |
| `must_use_candidate` | 1674 |
| `option_if_let_else` | 514 |
| `redundant_pub_crate` | 3408 |
| `doc_markdown` | 2415 |
| `missing_const_for_fn` | 1992 |
| `too_long_first_doc_paragraph` | 740 |
| `cast_possible_truncation` | 1382 |
| `cast_possible_wrap` | 690 |
| `cast_sign_loss` | 232 |
| `cast_precision_loss` | 50 |
| `unnecessary_wraps` | 50 |
| `unused_self` | 60 |
| `many_single_char_names` | 61 |
| `similar_names` | 140 |
| `too_many_lines` | 702 |
| `single_match_else` | 163 |
| `match_same_arms` | 181 |
| `if_not_else` | 42 |
| `items_after_statements` | 134 |
| `zero_sized_map_values` | 16 |
| `unnecessary_debug_formatting` | 21 |
| `float_cmp` | 33 |

The strict library and binary recipe and the all-target recipe are qualified separately on the final change.



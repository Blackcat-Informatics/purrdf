# Recovery synchronization and current acceptance

The existing S1 branch was fast-forwarded from e88a8e0e2 to the evaluator landing
29661e74b. The prior migration was saved with `git stash push --include-untracked`
under the exact object recorded in `recovery-migration-stash-oid.txt`. That stash
remains retained. Pre-synchronization staged and unstaged patches and source status
are retained alongside the apply log.

The original and restored status inventories match exactly: 416 unstaged tracked
paths, an empty staged layer, and the one untracked Datalog dictionary source.
The untracked dictionary's original bytes were verified after application. The
one textual conflict was in the evaluator lexical frame's storage-error adapter:
the resolution keeps the capacity error's original typed priority and routes the
other markers through the newly shared mapper from main. The shared mapper gained
its capacity arm. It does not restore the superseded duplicate mapping body.

`git diff --check` passes. This is preservation/integration evidence only. The
migration has still never compiled; no runtime, portability, capacity, host-ID,
sparse-stack, performance or complete-issue acceptance is established.

## Next bounded checkpoint

Run `cargo check -p purrdf-lex -p purrdf-core --lib` through the admitted contained
runner, with canonical cache roots inside this worktree, 8 jobs and the existing
16 GiB job cap. Geo currently owns the build lane. No additional migration edits
will precede this compiler feedback. The expected diagnostic areas are unfinished
fallible term/global ingress callers, typed error conversions, and newly landed
core evidence consumers. Actual compiler findings, rather than that expectation,
will determine the first correction slice.

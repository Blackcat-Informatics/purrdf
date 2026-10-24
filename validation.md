# Columnar allocation acceptance

Current candidate transfers only bb07611a9 onto main 0c14dc377. Its three-file
source delta is isolated from the donor's unfinished grammar and later writer
work. The donor and its original signed commit remain intact. The transferred
changes were recommitted normally with the full pre-commit hook, producing
6027fda26; the cherry-pick itself is not used as hook evidence.

VERDICT: PASS
Scope: the frozen three-file Columnar allocation delivery.
Evidence: 40 native cases, affected strict Clippy, normal commit/push hooks,
complete exact-head hosted CI including release portability, and independent
current-head review with no actionable finding.
Findings: none in the delivery. The extra local whole-workspace run was
interrupted under the replacement AGENTS validation rule, exit 143; it is not
passing evidence. The installed hook and CI definitions were inspected: local
whole-workspace runtime replay is not an additional mandatory hook or gate.
No required hook was cancelled or bypassed. Remaining landing mechanics are open.
Installed stagectl became unavailable during final refresh: its launcher now
requires missing /home/paudley/stage/root/bin/stagectl and exits 127. That failed
capture is not fresh forge evidence. Finalization is blocked until the installed
tool is restored. The installed /home/paudley/.local/bin/stagectl entry now works,
and fresh preflight/feedback capture has resumed. The existing prose checker at
/home/paudley/.codex/hooks/prose_guard.py runs directly and exits 0 for authored
delivery prose. No tool is recreated or installed, and no guard is bypassed.

| Requirement | Current evidence |
|---|---|
| Five tables / two codecs, zero validation allocation | columnar-native.log, exit 0; all 33 library tests pass |
| Blob pointer/bytes and constant traffic at 1 KiB/1 MiB | Same pass: original pointer, 1 allocation / 244 requested bytes at both sizes |
| Malformed/checksum/original-source ownership and first faults | Same pass: all six structured_errors cases, including input-disposal/source and checksum failures |
| Ten frozen file identities | Same pass: golden_bytes executes its ten-file identity assertions |
| Strict affected compilation | columnar-clippy.log, exit 0; all Columnar targets with -D warnings |
| Repository checks and release WASM build | Normal required hooks and exact-head hosted checks pass; CI wasm job passes the complete release library build for `wasm32-unknown-unknown`. Extra local full-check.log is interrupted, exit 143, not a pass |
| Normal commit/push hooks | transfer-commit.log / transfer-push.log, exit 0; commit 6027fda26 runs complete normal pre-commit gate, no bypass |
| Complete hosted checks and independent current-head review | PR 703: 49 successful statuses/checks, four manual-only skips, no failures; actual current-head CodeRabbit review reports no actionable code finding |
| ghprsq, archive, issue closure and supported cleanup | Pending |

Commands: cargo test -p purrdf-columnar --lib --test structured_errors --test
golden_bytes -- --nocapture; cargo clippy -p purrdf-columnar --all-targets --locked
-- -D warnings. Both owned handles are terminal with status 0. These checks run
through the installed Cargo wrapper with eight jobs/four Rust and Rayon threads.
The native run executes 40 cases total; no tests ignored or filtered.

No current run is claimed passing until its owned tool handle is terminal.
General semantics remain native. No parent completion or upstream work.

Hosted CI run 40113480023 completed successfully at exact head
6027fda2674c26084390bef3189ea488a2070db0. Its wasm job 120929852472 executes
the complete release-crate list with
`cargo build --locked --release --target wasm32-unknown-unknown --lib`.
The workflow source confirms this mirrors make
wasm. Reuse this actual build as release portability evidence; no duplicate local
release build is needed. Four skipped jobs are the manual-only simd-asm-projection,
native-profile-admission, native-profile and native-profile-comparison arms.
All ordinary required jobs, including actual architecture/assembly and host
jobs, pass. The optional local workspace replay was stopped after the replacement
AGENTS rule, with no failure reported before interruption. This does not claim
that unfinished replay passed. Hosted results and affected native/strict results
are independently terminal and cover the frozen delivery's acceptance.

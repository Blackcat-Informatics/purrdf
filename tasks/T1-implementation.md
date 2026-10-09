# Task 1 implementation

Source scope is the existing real `crates/rdf-capi/tests/c_smoke.rs` and new
within-crate `tests/support/phases.rs`, plus the CAPI manifest/lockfile edge to
the existing first-party dev-only testkit. No external/shipping dependency,
feature, workflow, target selection, main or sibling worktree changed.

The shared original Rust helper captures typed phase records and exact commands,
child statuses/output, monotonic elapsed nanoseconds and check failures. Every
phase rewrites the fixed-schema receipt, and an unwritable receipt hard-fails.
Cargo still runs on every invocation, now with `--locked`; `cargo pkgid` provides
the exact package identity and strict artifact selection requires that identity,
the `purrdf` cdylib target, valid profile/features/fresh metadata, an absolute
platform-correct library filename and unique selection. Native JSON decoding
rejects repeated members. The complete selected artifact message is retained.

Receipts bind the actual library/header bytes to BLAKE3 identities, retain the
lock/root manifest/toolchain and ancestor/Cargo-home configuration identities,
actual compiler/Cargo/C compiler version outputs, relevant environment, host
architecture/OS/available parallelism and Git index/HEAD/dirty/untracked source
state. Untracked names use Git's NUL inventory, retaining literal Unicode,
spaces, quotes, backslashes and newlines; non-UTF8 names produce actionable
InvalidData rather than a lossy identity. Environment members are sorted.
Stage artifacts and sibling worktree directories are excluded from the
untracked source census. Available parallelism is not claimed to be Cargo's
effective concurrency; configuration/environment remain evidence for later
controller resolution.

The real smoke and projection programs each have separately measured object
compilation, linking and runtime. C11/header include flags stay on compilation;
library-directory/library ordering remains on linkage. All original fixture and
frozen corpus arguments, platform loader environment, projection output existence,
nonempty bytes and owner-only Unix permission checks remain. Output preparation,
validation, identity and cleanup errors receive truthful failure records.

Four focused fixtures cover exact-package and fresh/cold metadata selection;
malformed UTF-8/JSON, missing/unrelated/ambiguous artifacts, wrong target/features,
relative filenames and repeated metadata; missing/nonzero children, failed output
checks and receipt write failures after both successful children and checks;
NUL-safe actual special-name file identities and refusal of malformed inventories.
Temporary fixture roots/uniqueness/lifetime use the shared testkit temp_dir home.

Root pre-review found the original line-split untracked enumeration and manual
fixture paths. Both are fixed as described above; focused verification was rerun
on the final source. The lockfile was refreshed with offline cargo metadata,
preserving all external versions and adding only the CAPI→testkit edge.
Generator inspection shows no affected projection inputs, so no generated
artifact is hand-edited or changed.

Final validation after the pre-review fixes:

- `CARGO_BUILD_JOBS=8 cargo clippy -p purrdf-capi --all-targets --locked -j8 -- -D warnings`: PASS.
- `CARGO_BUILD_JOBS=8 PURRDF_C_PHASE_RECEIPT=<Stage>/tasks/T1-c-smoke-receipt.json cargo test -p purrdf-capi --test c_smoke --locked -j8 -- --nocapture`: PASS, five tests including both real C programs. The durable receipt has all16 successful phases, exact package/artifact metadata, Cargo `fresh=true`, inherited jobs8, O3/debug assertions/overflow checks and library identity `f07e5d8041e216534399220df1a5e87025c99937b0af83d9f8ebb2eae7282db0`.
- `cargo metadata --locked --offline --no-deps --format-version 1`: PASS; external package versions are unchanged.
- `cargo fmt --all --check` and `git diff --check`: PASS.

Initial no-run checks exposed one missing type annotation, macro scope collision
with a local Result alias and excessive public visibility; all were fixed.
Clippy exposed Option map/unwrap and one fixture empty-assert spelling; both
fixed without lint exceptions.
An earlier real C run also passed, but inherited global Cargo jobs24 because
outer `-j8` did not propagate into the nested build. That observation led to the
explicit environment cap for final verification; it is not compared as timing
evidence. The complete first and final run durations are test-run observations,
not admitted campaign measurements or speedup claims.

No performance campaign, controlled cold/warm comparison, invalidation campaign,
full gate, wasm, hosted run, PR, commit or push is claimed. Remaining delivery
Tasks 2–5 are explicitly not established by this focused task.

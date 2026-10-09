# Collector repair normal publication readiness

Read-only source/Git configuration readiness; no stage, commit, hook execution, push or forge action performed. Current HEAD40de8b0402a29cff51d3c6dd50d96eb49f7c435f. Working diff exactly three intended Rust homes,545 insertions/42 deletions; index remains the original source with no staged diff. Only other untracked path is `.stage/`; no unrelated unstaged or untracked source observed.

| Home | Original indexed blob | Current BLAKE3 |
|---|---|---|
| hosted.rs |2971da8a6c56835726f5a3365e67c1afdfd7ec34|4661f5db0b6cb40e3b1af545e1d181e7f3a84695f50df4251d9f0f88acbee518|
| phases.rs |1cd872ed19816cb628444d4d19d338fbc07a77ea|525f3e4266320c825bf03212e3d2b8ed0692532713da1bc0eeac89d4a1f8b375|
| profile.rs |8457b32b9ca96b1cd726b39dfedc06d63fee6eae|56f67c5ce3f98e4825eeb1e5e1b16dadce51e063537ce90e27ebcf51fce96301|

All paths are `crates/rdf-capi/tests/support/`. Original source preimages are rooted in current40de indexed blobs; actual source/controller hash manifests and earlier patch receipts retain qualified current bytes. Working diff whitespace passes. Stage archival changes are evidence and must not enter the source commit.

Configured core.hooksPath is `.githooks`; pre-commit and pre-merge-commit exist/executable. Inspected pre-commit invokes the actual helper-census non-Rust ratchet on the staged index, then snapshots the exact index for owning fast gates. commit.gpgsign=true; gpg.program=gpg; configured signing fingerprint AF5E0032F7494CEBCAA7BBBE9B87CFBBCFDBAF11. This verifies configuration only; actual hook/signature/push success must be captured by root's normal command. No bypass is permitted.

Four repository-shared preexisting stashes are present: two issue408 geodesy preservation stashes,384 portable-conformance WIP and358 recovered inflight. None is this repair's stash; do not apply/drop/modify them or claim the shared stash list is empty. No stash operation was performed.

Proposed exact logical commit scope: stage ONLY hosted.rs, phases.rs and profile.rs, then normally signed commit `Preserve compiled Cargo runtime refusals in native CI receipts`. Description: separate the successful Cargo compiler prefix from exact native executable bytes/status; bind the actual executable, parent/child concurrency and failure-preserving runtime telemetry; qualify with35 original tests plus real production integration2/strict self-validation. Do not claim full performance reduction or complete hosted acceptance.

Root's later commit/push requires normal configured hooks; exact signed commit/source readback and remote branch head must match these qualified bytes before the fresh `native_profile=true` hosted dispatch. No PR until whole accepted contract is met. Scoped qualification is documented in T4-runtime-refusal-continuation-independent-review.md; full12/24 comparisons, reduction, Task5/full gates and whole delivery remain pending.

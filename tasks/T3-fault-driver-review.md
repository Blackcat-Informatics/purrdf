# Fault-driver source review — 2026-10-08

Disposition: source review PASS for exact-query/delegation and corrected executable
identity. Qualification NOT RUN. The concrete self-delegation gap below has been
resolved in source and independently re-read. No source edits, builds, tests, Make campaign, Git/index
mutation or forge operations were performed by this reviewer.

Read the current Rust example, Task3 preparation, validation, bench manifest and
actual scripts/lubm-lane.sh query/acceptance paths. Existing Task2 qualification
does not include this currently untracked auto-discovered Cargo example, and
the four full fourteen-query campaigns remain unrun.

## Source observations

Production invokes BIN with first argument query and exact last argument read
through Bash command substitution. trim_end_matches LF reproduces that removal
without changing CR, interior newlines or significant whitespace. Both argument
conditions must match; conversion/version/regime probes and unselected queries
delegate original OsString arguments to Command::status with inherited streams.
No passing query answer, generator, parser, oracle or result validator is added.
Invalid/missing mode/query/CLI configuration fails actionably; successful selected
malformed and wrong-uri faults intentionally exit zero, while exit fault returns
83. Production distinguishes these as BAD-RESULTS versus CANNOT-EXECUTE.

The wrong-uri mode emits one syntactically valid SPARQL JSON URI binding under X.
Its intended selected controls are Q1/Q14, whose full-data result path supplies
the independent graph-acceptance oracle. A generic query with no URI-set oracle
may accept this deliberately wrong row as syntactically valid: select Q1/Q14,
not an arbitrary published query, for semantic fault qualification. No assumption
that every query can distinguish a wrong URI is warranted.

## Concrete source gap — resolved in source, runtime unrun

The initially reviewed self-delegation protection compared canonical paths. Canonicalization catches
the same path and symlink aliases, but not an alternate hard link to this same
executable. Such a configured path passes real != own and is_file, then launches
this driver again with the same environment indefinitely. The error text promises
a separate executable and preparation claims recursive self-delegation refusal.
Root replaced the guard with actual device/inode identity through standard Unix
MetadataExt, catching direct, symlink and hard-link aliases while admitting
separate files even with identical bytes. Non-Unix qualification fails explicitly
with an actionable unavailable-identity error. One real-filesystem unit regression
constructs all four cases; source review confirms it reaches the guard's actual
identity home. The reviewer re-read the corrected source and found this gap
resolved. Compilation, unit execution and actual driver alias refusals remain
unrun; retain those controls below. Reviewer modified only this Stage report.

## Exact qualification commands after root lane admission

- CARGO_BUILD_JOBS=8 cargo build --locked --release --jobs 8 -p purrdf-bench
  --example lubm_fault_cli --message-format=json
  Select the actual executable from the Cargo receipt; freeze its identity.
- CARGO_BUILD_JOBS=8 cargo clippy --locked --jobs 8 -p purrdf-bench
  --all-targets -- -D warnings
- CARGO_BUILD_JOBS=8 cargo test --locked --jobs 8 -p purrdf-bench
  --example lubm_fault_cli
- cargo fmt --all -- --check
- Existing focused source-dependent acceptance:
  CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1 cargo test --locked --jobs 8
  -p purrdf-bench --test lubm_check_cli --test lane_common_laws
  --test make_bench_lanes --test make_scale_corpus --test corpus_cli
  (Actual test exit/counts must be captured; nothing here is a pass.)

With FAULT/CLI taken from captured artifacts and QUERY from a positive run's
actual regimes.tsv, execute serially in fresh owned output directories:

LUBM_FAULT_REAL_BIN="$CLI" LUBM_FAULT_QUERY_FILE="$QUERY"
LUBM_FAULT_MODE="$MODE" CARGO_BUILD_JOBS=8 make lubm
LUBM_BIN="$FAULT" LUBM_OUT="$FRESH_FAULT_OUT"

Run all three modes for each Q1 and Q14. Use the same admitted workload/schema
configuration as the source of QUERY, so normalized bytes really match. Capture
both executable identities, selected query bytes/config and all actual argv/log/
exit/run artifacts before and after. Each fault must actually be observed in the
selected published row, not merely inferred from nonzero Make exit.

## Required real controls and acceptance

Direct driver checks: version, one real conversion, actual regime probe, and
unselected query must match real CLI output/status. Selected query must produce
the deliberate mode output/status with an attributable diagnostic. A trailing-LF
selected file must still intercept; a deliberately different significant byte
must delegate. Missing CLI/query/mode, invalid mode, empty query, direct self-path,
symlink and hard-link recursion configurations must refuse. No mock passing
engine response qualifies delegation.

Actual Make fault acceptance: malformed and wrong-uri selected Q1/Q14 rows are
BAD-RESULTS; exit fault is CANNOT-EXECUTE. All unfaulted paths remain actual CLI.
Each required selected-oracle fault must yield nonzero Make, no COMPLETE or
successful oracle declaration, and no reusable graph-acceptance.json after the
exit trap. Preserve independent generation receipt and failed-run diagnostics.
Verify earlier positive siblings byte-identical. Wrong-uri must fail through the
actual exact URI-set oracle, not just a syntactic JSON check. These six negative
runs complement and cannot replace D1/D2/N1/N2 full fourteen-query qualification,
cache hit/corruption, admission, receipt/data tamper and shared-law controls.

All commands and runtime assertions above are NOT RUN. Source review is not
Task3 completion, and actual qualification remains a prerequisite for PASS.

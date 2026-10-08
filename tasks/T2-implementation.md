# Task 2 implementation and qualification

Status: IMPLEMENTED; focused qualification PASS; ready for independent review.
This is Task2 qualification, not a complete workload campaign or issue completion.

## Production source

- New first-party Rust `lubm-check` production executable and `lubm::check` home
  use the existing RDF parser/model/serializer, native JSON record codec and
  SPARQL Results reader. No new external/shipping dependency or feature.
- Receipt admission binds the explicit profile and configuration, canonical sorted
  department filenames, exact source/converted inventories, regular nonempty
  files and byte identities. Every department conversion must equal its actual
  parsed source graph; the aggregate must equal the sorted converted bytes and
  complete graph union. Native department/document identity and profile inventory
  bounds are checked. Actual graph-derived Q14 must be nonvacuous.
- Q1 is the set intersection of explicit graduate types and subjects taking the
  original fixed University0/Department0/GraduateCourse0 target. It is not
  retargeted at nonzero indexes. Q14 is the distinct explicit undergraduate set.
  The native result reader compares exact URI sets, rejects duplicates and
  unbound/non-URI answers, and identifies malformed success as BAD-RESULTS.
- Custom ontology projection walks actual owned RDF terms, all S/P/O positions,
  datatypes and graph names, with the shared iterative nested-term fold. External
  input and its digest remain separate; W3C/foreign IRIs and lexical bytes stay.
- `Spec::from_decimal` is the shared generator/checker CLI decimal admission home.
  Seed/index stay full u64 and normalize by value. Config emits a strict single
  `seed<TAB>index` record; the Make caller checks that protocol and uses those
  values before acquisition/generation. Shared positive count/slice guards retain
  their documented shell-safe maximum without changing WatDiv/scale laws.
- The actual lane now builds Cargo-fresh native tools, captures actual executable
  paths from Cargo JSON using Rust, invokes native generation and real CLI
  conversion, checks graph acceptance before/between/after queries, and preserves
  the existing fourteen normalizations, regime map and full/file/slice ladder.
  Each run owns a fresh reported child directory; older success/failure outputs
  are preserved. Failed runs revoke graph acceptance. Partial/unsupported/subset
  reports cannot claim complete workload qualification.
- Before acquisition, native output admission requires either an output directory
  outside Git repositories or a whole directory ignored by a tracked `.gitignore`.
  Personal excludes, re-inclusion rules, broken Git state and visible repository
  paths fail. Read-only Git queries establish this provenance; no index is changed.
  During execution, external schema/query caches are revalidated through the
  acquisition home and retained CLI/generator/checker/schema/query/ladder identities
  are checked alongside the native graph acceptance.
- Java/JRE/archive/Linux-fix/class inspection and historical corpus/Q1/Q14 pins
  are removed from actual acquisition, lane, Make/guide/readme/benchmark/law
  documentation and affected tests. Legacy Python shrinks; common lane algorithms
  and WatDiv/scale implementations remain unchanged.

## Settled commands and evidence

Root explicitly admitted the single heavy lane. Direct Cargo commands set
`CARGO_BUILD_JOBS=8` and `--jobs 8`; existing Make gates inherited eight-job
configuration. Tests used `RUST_TEST_THREADS=1`. No Java, full suite, fourteen-query
campaign, hook, commit, index mutation or forge action ran for Task2.

Canonical logs under `tasks/T2-logs/` bind the actual terminal results:

- `cargo build --locked --release --jobs 8 -p purrdf-bench --bins`: PASS,
  `build-corrected.log`; actual Make's Cargo launcher rebuilt the checker after
  subsequent lint fixes.
- `cargo test --locked --jobs 8 -p purrdf-bench --lib --tests`: PASS,
  `tests-settled.log`: 35 library + 10 scale-binary + 20 scale-CLI + 35 lane-law
  + 5 checker-CLI + 4 native-generator-CLI + 16 Make-lane + 29 Make-scale = 154 tests;
  none failed or ignored. Complete default/nonzero/custom graphs, exact answers,
  receipt/inventory/conversion/aggregate tampering, quoted-triple/all-position
  projection, malformed results, full-u64/config overflow and output-ignore
  provenance ran. Actual Make default native Cargo admission did not deadlock;
  no selectable prebuilt checker or private fallback was needed.
- `cargo clippy --locked --jobs 8 -p purrdf-bench --all-targets -- -D warnings`:
  PASS, `clippy-settled.log`; no suppression added.
- `cargo fmt --all --check`: PASS, `fmt-settled.log`.
- `make layer-hygiene helpers-hygiene terminal-hygiene build-profile-hygiene`:
  self-tests and live gates PASS, `hygiene.log`.
- `cargo run --locked --jobs 8 -p helper-census -- --non-rust-ratchet
  --merge-base-with origin/main --target worktree`: PASS, `ratchet.log`.
- `python3 scripts/benchmark-acquire.py --self-test`: PASS,
  `acquisition-settled.log`, including WatDiv answers/aggregation, verified cache
  hits, corruption quarantine and no-refetch controls. `python3
  scripts/lubm-queries.py --offline-self-test`: PASS, `normalizer.log`.
- `make metadata`: PASS, `metadata.log`; no generated drift.
- `cargo build --locked --release --jobs 8 -p purrdf-cli --bin purrdf
  --message-format=json`: PASS, `cli-build.log`/`cli-build.jsonl`; the same native
  production Cargo artifact reader selected its actual executable.
- `bash .stage/benchmarks-replace-the-java-lubm/tasks/T2-smoke.sh`: PASS,
  `smoke-corrected.log`. Original Task1 receipt BLAKE3
  `09438d399fc64246fa5f72fa127ca972212c491b97937dd820ec3d55cae22c6e`
  was checked before real CLI conversion of all 16 department payloads. Native
  acceptance observed 106,958 statements and exact full graph/byte identity.
  Actual no-entailment Q1 returned 2 distinct URIs; Q14 returned 6,141. Both
  complete sets matched independent graph oracles; full recheck after both
  queries passed. These are observations, not new answer pins/UBA equivalence.
  Retained artifacts: `/opt/purrdf-native-lubm-task2-smoke.or8g65vq`.
  `smoke-identities.sha256` binds binaries, queries, results and acceptance/data.
- After correcting actual conversion, `cargo test --locked --jobs 8 -p
  purrdf-bench --test lane_common_laws --test make_bench_lanes` reran PASS 35 + 16,
  `lane-source-settled.log`.

`source.sha256` binds all 21 changed/new source/document/manifest files;
`head.txt`/`rustc.txt` record parent/toolchain and `diff-check.log` is clean.

## Failures repaired and limits

Historical unsuccessful logs remain. Compilation required explicit Arc dataset
references and unwrapped testkit directory creation; comment retirement had
removed the existing test support declaration, which was restored. Acquisition's
self-test still required the retired published LUBM count pins; that obsolete
block was removed while WatDiv classification/aggregation stayed. The existing
stderr-reset law caught the new result-check capture, which now calls the shared
reset. Strict clippy found test-module ordering, empty-set assertion and
collapsible-condition issues; fixed without suppressions.

The real CLI refused ineffective `--base` on N-Triples→N-Quads. Production and
repeatable smoke now omit it; document-base identity remains in native admitted
metadata/receipt. First failed smoke/output remain preserved. Reviewed stale
licensing and conversion-probe comments were corrected to current callers.

WatDiv/scale implementations, common locale/cache/certificate algorithms,
fourteen query-normalization bodies and regime assignments are unchanged.
The complete default/nondefault fourteen-query campaign is not claimed by this
focused Q1/Q14 smoke. Full local/hosted qualification and integration remain
the following task boundaries.

Task 3 owns the accepted real fourteen-query default/nondefault campaign after
this complete focused task and independent review. Task 4 owns settled full gates
and integration. Root owns normal commits/push/forge operations. Heavy lane FREE.

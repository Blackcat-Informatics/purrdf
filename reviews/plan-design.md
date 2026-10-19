# Plan design review (SOLID / ETHOS), pass 2 — #570 W3C SHACL EARL reports

Reviewed plan: `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports/.stage/submit-w3c-shacl-implementation-reports/plan.md` (revision 2)

## Code read (pass 2, in addition to pass 1)

Worktree `/home/paudley/Active/purrdf/.worktrees/570-submit-w3c-shacl-implementation-reports`:

- `crates/conformance-kit/src/outcome.rs` (whole), `src/lib.rs`, `src/report.rs` (whole: the existing `report` module is SHACL validation-report comparison), `Cargo.toml` (deps: core, iri, lex, hash, sparql-results; no serializer)
- `crates/sparql-conformance/src/bin/community-conformance.rs` (whole), `src/community.rs:95-212` (`Execution`, `Totals::of`, `Totals::scoreboard`)
- `crates/shapes/tests/w3c12_conformance.rs:540-835`, `tests/w3c_conformance.rs` (whole), `tests/*.rs` `#[path = "support/..."]` idiom (e.g. `w3c12_conformance.rs:61-62`)
- `crates/shapes/src/shacl_corpora/shacl12.rs:1-385` (`Case12`, `manifest_roots`, `shacl12_cases`, `entry_id`), `shacl_corpora/mod.rs:80-220`, `shacl_corpora/report_grading.rs:97`
- `crates/shapes/Cargo.toml` (`purrdf-rdf` is a normal dependency, line 73), `crates/rdf/src/native_codecs/serialize.rs:119` (`serialize_dataset`)
- `crates/iri/src/vocab.rs:1633-1665` (`earl`; no `doap`/`foaf` module)
- `scripts/vendor-shacl12.py` (whole), `scripts/vendor-*` (no 2017 SHACL vendor script), `vectors/shacl/README.md`, `scripts/conformance-frozen/roots.toml:10`
- `scripts/conformance-matrix.py:215-306,350,1721-1768` (`_scrape`, the three SHACL scoreboard regexes and their self-test fixtures)
- `Makefile:1-694` (`release-tags` dirty-tree check :219; `lane-env` :383-412; `LUBM_OUT` :554)
- `vectors/shacl12/tests`: `mf:status`/`sht:approved`/`sht:proposed`/`mf:requires` counts per folder (see ADVISORY-4)

W3C checkout `/home/paudley/Active/w3c/data-shapes`:

- `shacl12-test-suite/reports.js:1-440` (`testKey` 172-181, `collectTests` 185-215, `loadReport` 292-324, render 367-429)
- `data-shapes-test-suite/reports/alltests.ttl:1-40`

## Pass-1 disposition

| Pass-1 finding | Plan text | Disposition |
|---|---|---|
| BLOCKING-1 SRL keys / discovering root | G6: `Case12` records its discovering root, replacing `listed`; 1.2 EARL covers only SHACL-root entries; exact key-set equality test; R32/R33 adds "no Unknown tests row"; SRL EARL declined with the reason | **DISCHARGED**. `manifest_roots` (`shacl12.rs:287-298`) already distinguishes the two roots, so the field is a rename of existing information, not new state. |
| BLOCKING-2 G1 outcome unrepresentable | G1 "Harness representation": `SPEC_CONTRADICTED` table per harness, graded unamended, XPASS is an error, pinned by count, own label, `earl:failed` + `earl:info`; `XFAIL` stays empty | **DISCHARGED**. The four-arm match (`w3c12_conformance.rs:664-682`) gains the fifth state the plan needed. The new label changes a scoreboard line a gate parses: see BLOCKING-4. |
| BLOCKING-3 commit source | G7: `PURRDF_W3C_EARL_COMMIT` (40 hex) paired with `PURRDF_W3C_EARL_DIR`, parsed once into one struct, either alone or malformed is a hard failure; `make w3c-earl` supplies both | **DISCHARGED**. See ADVISORY-6 for the Makefile knob mechanics. |
| ADVISORY-1 Record/Observation mapping | G12 | DISCHARGED (matches the proposed mapping; `cantTell`/`untested` asserted unreachable). |
| ADVISORY-2 `mf:requires` | G5: `requires` on `Case12`, folder default, `requires ⊆ CLAIMED` asserted | DISCHARGED. |
| ADVISORY-3 two serializations | G13: one serialization (N-Triples via the existing writer), `.ttl` extension | DISCHARGED. |
| ADVISORY-4 `af/` by string | G9: discovery field | DISCHARGED. |
| ADVISORY-5 write errors | G13: any write error is a hard failure; fixed file names | DISCHARGED. |

## Findings

### BLOCKING-4. The scoreboard line is a parsed contract with `make conformance`, and the plan changes it without naming the consumer

Plan: R8 "The `non-canonical-expected-decimal` label is gone from the scoreboard source"; G1 "Each gets its own scoreboard label"; G11 "its scoreboard totals line [is a] projection of the same records"; G9 adds a `W3C 2017 subset: passed N, spec-contradicted M of 120` line; R24-R26 "`make check` and `make conformance` run at the end".

Code: `scripts/conformance-matrix.py:296-297` matches exactly `W3C12 TOTAL: passed (\d+), non-canonical-expected-decimal (\d+), xfailed (\d+), ledger (\d+)`; `:268` matches the 2017 `TOTAL: passed (\d+), xfailed (\d+), ledger (\d+)`; `:350` scrapes `COMMUNITY-SHACL: passed N total N`. `_scrape` (`:225-227`) returns the hard-RED `_no_scoreboard` row when the regex does not match. The self-test fixtures at `:1721` and `:1738` hard-code the current formats. The plan does not mention `conformance-matrix.py`, `make conformance` or these regexes anywhere.

Consequence: Task 7 or Task 8 removes the label, the 1.2 row goes RED in `make conformance`, and R24-R26 cannot be met by the plan as written; alternatively the implementer keeps a dead `non-canonical-expected-decimal 0` field in the line to appease the regex, which is the amended-grade category the plan deletes living on as a zero.

REWRITE TO: in G11, state the two totals-line formats the kit projection emits and the one consumer that reads them: `W3C12 TOTAL: passed N, spec-contradicted N, xfailed N, ledger N` and `TOTAL: passed N, spec-contradicted N, xfailed N, ledger N`, and add to Task 8's commit the update of `conformance-matrix.py:262-306` (both regexes, the `build` lambdas placing `spec-contradicted` in the XFail/Skip column with a detail string, the docstring at `:287-292` that still describes the amended grade, and the fixtures at `:1721,:1738`). State that `_scrape` runs the whole test binary with `--nocapture`, so the new EARL tests run inside `make conformance` as well (they must pass with neither env variable set). The constraints reviewer rules on the Python edit under the ratchet; the design requirement is that the contract is named and has one format per harness.

### BLOCKING-5. The 2017 inventory check depends on a vendored `alltests.ttl` that has no producer and no home

Plan: Task 9 "2017: vendor `alltests.ttl` via the vendor path, pin its hash"; R32/R33 (2017) "a vendored copy of that file, pinned by hash"; R3/R4 "`scripts/conformance-frozen/vectors-shacl.sha256` (unchanged)" and "0 CR-insensitive content differences from `4cd46986` `data-shapes-test-suite/tests`"; binding documents: "`scripts/vendor-shacl12.py` changes only its pin line" and "`vectors/` changes only through the vendor script and `check-corpus-frozen.py --update`".

Code: the only SHACL vendor script is `scripts/vendor-shacl12.py`, which extracts exactly two prefixes (`VOCAB_PREFIX`, `TESTS_PREFIX`, lines 65-66) into `vectors/shacl12/`; `vectors/shacl` has no script (`vectors/shacl/README.md:11-17` records a hand-vendored commit and subset; `scripts/vendor-*` lists no 2017 SHACL script). `alltests.ttl` lives upstream under `data-shapes-test-suite/reports/`, outside the `tests/` subtree the R3 diff compares.

Consequence: there is no "vendor path" for this file. Extending `vendor-shacl12.py` with a third prefix contradicts the plan's own pin-line-only rule; copying by hand into `vectors/shacl/` makes the R3/R4 row false (the sha256 manifest changes and the tree is no longer the upstream `tests/` subtree); copying anywhere else leaves a W3C-licensed file outside every provenance and freeze ledger. Each option is a wall the implementer hits in Task 9, and the U2 acceptance check for R32/R33 (2017) is unverifiable as written.

REWRITE TO: give the file one named home and one freeze, and leave the 2017 corpus tree untouched: a new frozen root such as `vectors/shacl-reports/alltests.ttl` with a `README.md` in the style of `vectors/shacl/README.md` (upstream path `data-shapes-test-suite/reports/alltests.ttl`, commit `4cd46986…`, licence), an entry in `scripts/conformance-frozen/roots.toml` and a `check-corpus-frozen.py --update` run. `w3c_2017_earl_matches_wg_inventory` reads that path through one constant beside `CORPUS_DIR`/`SHACL12_DIR`, asserts the approved-subject count is exactly 120 and the one `sht:proposed` subject is excluded, and parse failure is a test failure. State explicitly that the R3/R4 row stays true because `vectors/shacl` does not change. Whether the copy itself may be made by hand under "`vectors/` changes only through the vendor script" is the constraints reviewer's ruling; if it may not, the plan must name a Rust producer, not "the vendor path".

### ADVISORY-1. `Report.claimed_classes` has no reader; put `Report` beside `to_earl`, not beside `report::compare_*`

Plan: G11 "A `Report` in `purrdf-conformance-kit` holds the subject, assertor, claimed classes and sorted records."

Code: `Record` (`outcome.rs:101-116`) carries no conformance class, so the per-class tally in the R32/R33 row ("per-`mf:requires`-group passed counts equal the harness's per-class tally") can only be computed in the harness from `Case12.requires`; `inapplicable` is never emitted (G5). The kit's `report` module (`report.rs`) already means "SHACL validation report comparison" (`SourceRole`, `ReportContext`, `compare_contextual`).

Consequence: a field that every caller fills and nothing reads; a `Report` type whose name collides with the module where "report" means `sh:ValidationReport`.

REWRITE TO: `Report { subject: SubjectDescription, assertor: AssertorDescription, records: Vec<Record> /* sorted by case in the constructor */ }` in `outcome.rs`, with `to_earl(&Report)` and `totals_line(&Report, prefix)` as its two projections. The claimed-class pin and the `requires ⊆ CLAIMED` assertion stay in the 1.2 harness over `Case12.requires`.

### ADVISORY-2. The `community-conformance` migration must be EARL-only

Plan: G11 "`community-conformance` migrates to it."

Code: community totals are per `Execution` (case × profile, several `Record`s each; `community.rs:107-121,147-179`), not per record, and `conformance-matrix.py:350` scrapes `COMMUNITY-SHACL: passed N total N` from `Totals::scoreboard` (`community.rs:183-211`).

Consequence: if the community lane also adopts the `Report` totals line, either the matrix row goes RED or two totals lines coexist for one run (§O), and the gate at `community-conformance.rs:51` (`failed == 0 && unsupported == 0 && passed == executions`) would be computed from a different count.

REWRITE TO: state in G11 that `community-conformance` builds a `Report` only to call `to_earl`; `Totals::scoreboard` and the `COMMUNITY-SHACL` line are unchanged.

### ADVISORY-3. One home in the shapes tests for everything both harnesses spell

Plan: Task 8 "The W3C harnesses (test code) supply" the subject and assertor; G7 "Both are parsed once into one struct"; G6 "a Rust port of the page's `testKey`".

Code: the two harnesses are separate test binaries (`w3c12_conformance.rs`, `w3c_conformance.rs`); the repository's way to share code between them is `#[path = "support/x.rs"] mod x;` (`w3c12_conformance.rs:61-62`, `support/report.rs`, `support/turtle.rs`). The plan names no such module.

Consequence: the subject IRI, the DOAP literals, the assertor, the 40-hex check, the env-pair parser, the key function and the file writer are each spelled twice, and the two files the WG receives can drift (one says `PurRDF 0.x`, the other something else) with no test that compares them.

REWRITE TO: `crates/shapes/tests/support/w3c_earl.rs` (`#[path]`-included by both harnesses) owning `EarlOutput` (env pair), `subject()`/`assertor()` descriptions, `fn test_key(id: &str) -> String` (`"urn:x-shacl-test:/" + id`), `fn write(report, dir, file_name)` and the `SPEC_CONTRADICTED` grading arm's tally helper. The `testKey` port needs only the `urn:` branch for PurRDF's own keys; `Case12.id` already is the page's key for SHACL-root entries because `entry_id` (`shacl12.rs:359-363`) strips the root IRI exactly as `reports.js:176-177` strips `TESTS_BASE`; do not port the `/tests/` fallback (`reports.js:179-180`), which nothing reaches.

### ADVISORY-4. `status` as a closed enum, and the page check counted over all rows

Plan: G5 "`status` (from `mf:status`)"; R21 "No `earl:test` for `sht:proposed` entries … 1.2: the `status` field is asserted"; R32/R33 (1.2) "zero 'no data' cells for approved entries".

Code: `reports.js:196-210` lists every `mf:entries` member regardless of `mf:status`; the page has no notion of "approved". In the vendored tree every SHACL-root entry carries `mf:status sht:approved` (core 162 of 162 `sht:Validate`, sparql 43 of 43, inference-rules 32 of 32, node-expr 143 entries with 146 statuses) and `sht:proposed` occurs nowhere; the 203 `sparql-rl/` entries carry no status at all.

Consequence: an `Option`-typed or stringly `status` lets a future status-less SHACL-root entry be silently dropped or silently emitted; and because "approved entries" is PurRDF's filter, not the page's, the Chromium check as worded passes while a proposed or status-less entry shows "no data" in PurRDF's column.

REWRITE TO: `pub status: Status` with `enum Status { Approved, Proposed }`, discovery panics on an absent or unknown `mf:status` for SHACL-root entries (the SRL root is exempt, as it carries none); pin "SHACL-root proposed entries: 0" so R21's filter is a no-op today and the emitted set equals the page's set; word the Chromium check as "zero 'no data' cells in PurRDF's column over every row the page renders", which is the only wording that would catch the drop.

### ADVISORY-5. Define the `earl:info` text so the bytes are machine-independent

Plan: G1 "emitted as `earl:failed` with an `earl:info` rationale"; G12 "Harness `Err` becomes observed `Mismatch` with the error text as message"; R20 byte determinism and `assert_golden!`.

Code: engine error strings can carry absolute paths (`report_grading.rs:97`, `tc.shapes_path.display()`) and `no_panic` captures arbitrary panic text; in a green build the only failed records are `SPEC_CONTRADICTED` ones.

REWRITE TO: `to_earl` emits `earl:info observed.message` for failed records only; the harness sets `observed.message` for a `SPEC_CONTRADICTED` entry to the table's rationale and logs the engine error on the test's stdout. The double-generation test cannot see a path that differs between machines, so this rule, not that test, is what makes R20 hold across hosts.

### ADVISORY-6. `make w3c-earl` knob mechanics

Plan: Task 10 "it sets `PURRDF_W3C_EARL_COMMIT` from `git rev-parse HEAD` and `PURRDF_W3C_EARL_DIR`" (no source for the directory; no test filter named).

Code: path knobs are exported as environment bytes through `lane-env` (`Makefile:383-412`, rationale 358-382); `LUBM_OUT ?= target/lubm` (`:554`) is the precedent; the dirty-tree check is `release-tags`' `git status --porcelain` (`:219`).

REWRITE TO: `W3C_EARL_DIR ?= target/w3c-earl` through `lane-env`, exported as `PURRDF_W3C_EARL_DIR`; `PURRDF_W3C_EARL_COMMIT=$$(git rev-parse HEAD)` set in the recipe after the porcelain check; the recipe runs `cargo test -p purrdf-shapes --test w3c12_conformance --test w3c_conformance -- --nocapture` (the grading tests write the files; see ADVISORY-7) and prints the two paths.

### ADVISORY-7. Grade once per harness binary

Plan: Task 8 lists some ten `#[test]`s (R11-R21, R30, R31, key equality, env negatives) that each need the graded records; `conformance-matrix.py:222` runs the whole binary with `--nocapture`.

Consequence: each test that builds its own `Report` re-grades 592 (or 129) cases, multiplying `make conformance` time by the test count, and the R31 "two projections of the same records" claim is only true if the records really are the same object.

REWRITE TO: one `fn graded() -> &'static Report` per harness behind a `OnceLock`, consumed by the scoreboard test, the EARL writer and every assertion; the only grading call site in each binary.

## Judged sound (no finding)

- G6/G9 root and subset fields, G12 mapping, G13 one writer: `purrdf_rdf::serialize_dataset` (`serialize.rs:119`) is reachable from the shapes tests (`purrdf-rdf` is a normal dependency, `crates/shapes/Cargo.toml:73`) and is the same function `community-conformance.rs:41` calls, so one writer serves all three callers without a kit dependency change. Determinism belongs to `Report`'s constructor (sorted records, index-labelled blanks as `outcome.rs:161-162` already does), not to a post-sort of lines.
- G7's hard failure on a lone or malformed variable, G10's caller-supplied descriptions with the DOAP/FOAF spelling confined to test code, G15's dev-dependency (unchanged from pass 1).
- The `SPEC_CONTRADICTED` table with XPASS refusal and count pin is the right single representation; `XFAIL` empty and the amended-grade path removed leaves exactly `Pass`/`SpecContradicted` for approved entries.

VERDICT: BLOCKED

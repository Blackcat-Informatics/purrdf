# T8 independent review (uncommitted tree over 744e0d69e)

## Scope
Plan G1, G5-G13, G15, rows R11-R21, R30, R31, "Task 8". Read the full diff, `tests/support/w3c_earl.rs`, both harness EARL tests, the kit tests and golden.

## Evidence (all run, all green)
- `cargo test -p purrdf-conformance-kit` 26+1 pass; `-p purrdf-sparql-conformance` pass.
- `-p purrdf-shapes --test w3c12_conformance --test w3c_conformance --nocapture`: 28 + 11 pass. Printed: `W3C12 TOTAL: passed 591, spec-contradicted 0, xfailed 0, ledger 0`; `W3C12 SHACL root: passed 382, spec-contradicted 0 of 382`; six class lines (162+33+146+31+5+5 = 382); `TOTAL: passed 129, xfailed 0, ledger 0`; `W3C 2017 subset: passed 120, spec-contradicted 0 of 120`.
- Manual generation with both env vars into a mktemp dir (deleted): both files written, 382 and 120 `earl:Assertion`, 3068 and 972 N-Triples lines, byte-sorted. Dir alone fails ("PURRDF_W3C_EARL_DIR is set but PURRDF_W3C_EARL_COMMIT is not"). (No rdflib on this host; line/triple counts only, the in-test reader parses the same bytes.)
- `conformance-matrix.py --self-test`, `check-doc-claims.py`, `check-layers.py` OK; helper-census `--no-features` rc 0; `--non-rust-ratchet` OK; clippy `-D warnings` on kit, shapes, sparql-conformance, iri OK; `cargo check --workspace --all-targets` OK; `cargo fmt --check` clean.
- DOAP/FOAF IRIs: none added under `crates/*/src` (the only hits are the pre-existing prefix table in `slice/src/mapping_support.rs`); the new ones are only in `crates/shapes/tests/support/w3c_earl.rs`. No issue numbers or process wording in added lines.
- `purrdf-conformance-kit` root entry is path-only (`Cargo.toml:86`); layers check passes.

## Contract rows
| Row | Verdict | Test |
|---|---|---|
| R11, R12 | met | both harnesses build `Report` via `w3c_earl::report` and `to_earl`; kit `a_report_renders_both_projections_over_two_iri_styles` |
| R13 | met | `*_earl_keys_equal_wg_page_keys` (382 = pinned), `w3c_2017_earl_covers_the_subset_and_nothing_else` (120), `earl:automatic` asserted |
| R14 | met | `*_earl_outcomes_equal_the_unamended_grades` reads `g.result` directly; `a_spec_contradicted_entry_is_reported_failed_with_its_rationale` covers the non-empty path |
| R15 | met | claimed-set union test, `check_record_mapping` asserts Compared/Mismatch only, kit `inapplicable_and_other_unfinished...` |
| R16 | met | key port is faithful to `reports.js:172-181`; `urn:x-shacl-test:/path` matches `topquadrant-shacl-earl.ttl`; test branches covered by `check_test_key`. Default-class port covered by union test. |
| R17, R18 | met | `check_descriptions` spells exact expected sets independently of `subject()` (name `PurRDF 3.0.1`, homepage, repository, language, developer, bug-database, description with both commits; property set equality, so extras fail); both full suite commits asserted; commit-without-dir, dir-without-commit, 6 bad commits refused |
| R19 | met | assertor type + `foaf:name` set-equality; every assertion's assertor/subject checked |
| R20 | met | double generation byte-equal, sorted, no date/time predicate; kit golden `earl-report.nt` |
| R21 | met | 2017: proposed ids absent; 1.2: synthetic proposed/other-root/unlisted set yields only the approved key |
| R30, R31 | met | `earl_agrees_with_scoreboard` in both. Tally is built in `scoreboard_of`/`scoreboard()` from the `classify` arms before any `Report`. Mutations (`mutate_flip/remove/add`) act on the serialized bytes only and `assert_ne` proves each changed it. Caveat F5. |
| G15 | met | AGENTS.md crate map and section 6, `release-crates.sh` comments updated; dev-dependency in `crates/shapes/Cargo.toml` |
| SPEC_CONTRADICTED semantics | met | pinned 0, XPASS refused, unknown id refused, duplicate/blank rationale refused, `earl:info` carries rationale only (engine error and path asserted absent) |
| community-conformance | met | EARL only; `COMMUNITY-SHACL` line untouched |

## Findings

### F1 BLOCKING: `earl:info` is placed on the `earl:Assertion`, not the `earl:TestResult`
EARL 1.0 Schema section 3.9: `earl:info` has domain `earl:TestResult` ("additional warnings or error messages in a human-readable form"). The `earl:Assertion` domain list is only assertedBy, subject, test, result, mode. `outcome.rs` pushes `(assertion, info_predicate, info)` (the `if let Some(info)` block in `to_earl`); the reader `assertions()` in `w3c_earl.rs` also looks up `t.s == a`, and the golden pins the wrong shape. Fix: push the triple on the `result` blank node; change the reader to look up the statement on the result node (`one`-style, via `earl:result`); regenerate the kit golden through the supported procedure; keep the test that `earl:info` appears exactly once and only for SPEC_CONTRADICTED. Add an assertion that no `earl:Assertion` subject carries `earl:info`. Doc comments on `ReportRecord.info` ("the assertion's `earl:info`") need wording to "result's".

### F2 MAJOR: a 2017 spec-contradicted entry is invisible in the matrix (G9a)
The 2017 row scrapes only the unchanged `TOTAL: passed N, xfailed N, ledger N` (`conformance-matrix.py:267-268`). In `w3c_conformance.rs` a SPEC_CONTRADICTED entry increments neither `passed` nor `xfailed`, so the row shows Pass and XFail/Skip both without it, and the new `W3C 2017 subset` line is not scraped. The harness test would stay green (count gate is pin-based), so the entry would disappear from the report, contrary to "spec-contradicted reported in XFail/Skip, never folded into Pass". It is latent only because the table is empty.
Proposed fix (line-neutral): print `TOTAL: passed N, spec-contradicted N, xfailed N, ledger N` in `w3c_conformance.rs`; change the regex at `conformance-matrix.py:268` to `r"TOTAL: passed (\d+), spec-contradicted (\d+), xfailed (\d+), ledger (\d+)"`, the lambda to `(passed, xfailed + spec, 0, ...)` with the same detail format as the 1.2 row, the contract string at 267, and the self-test fixture at 1716 (`TOTAL: passed 6, spec-contradicted 1, xfailed 2, ledger 2`). Careful: the 1.2 regex `W3C12 TOTAL:` and this one are unanchored; the 2017 regex does not match the 1.2 line because the 1.2 line has `W3C12 TOTAL: passed N, spec-` ... which WOULD match `TOTAL: passed (\d+), spec-contradicted` only if scraped from the same output; they are separate binaries, so fine. Check `check-doc-claims.py` for any 2017 `TOTAL:` pin (none found by grep). Python edits must keep line counts (replace in place).

### F3 LOW: community `earl.nt` byte change is not acknowledged anywhere
It now carries subject/assertor `rdf:type` triples (14 quads in the kit test versus 8) and assertion nodes are labelled over sorted records. No committed golden or test pins `earl.nt` (grep: only `corpora/community/README.md:33`, descriptive; the sparql-conformance tests pass), so nothing breaks. Acknowledge it in the commit body and the handoff, as the plan requires.

### F4 LOW: XFAIL entries in the EARL scope would count under the `spec-contradicted` label
`Report::scoreboard_line` counts every `earl:failed` under the caller's failed label. If a ledgered XFAIL ever exists in scope it prints as `spec-contradicted N` on the scope line while `W3C12 TOTAL` prints it as `xfailed`. XFAIL is empty and the amended path is being removed; consider asserting `XFAIL` has no in-scope id, or label the scope line's failed count `failed`. Not blocking.

### F5 NOTE: the "printed" line in `earl_agrees_with_scoreboard` is the `scope_lines()` string, not captured stdout
It is the very string `w3c_shacl12_conformance` prints (same function), so the comparison is against the printed format, with the independent tally from the match arms. Acceptable; stdout capture is not possible in-process.

### F6 NOTE: `doap:repository` equals the homepage IRI (`CARGO_PKG_REPOSITORY`)
Matches the plan (git repository IRI) as spelled in the manifest; the exact-value test pins it.

## VERDICT: BLOCKED
Reason: F1 (`earl:info` on the wrong class). F2 should be fixed in the same pass. Everything else verified.

## Recheck (orchestrator)
F1 resolved: earl:info written on the earl:TestResult (outcome.rs, golden shows it on _:result1), reader refuses earl:info on an Assertion. F2 resolved: 2017 TOTAL line carries spec-contradicted; matrix regex/self-test updated line-neutral. F4 resolved: in-scope XFAIL is a scoreboard error. Implementer re-runs all green.
VERDICT: PASS

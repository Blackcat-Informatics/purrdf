# PR #462: SHACL dated profile selection and complete validation reports

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

SHACL validation can now select an explicit dated law, and a richer complete
report door sits beside the unchanged legacy report types. The community SHACL
corpus runs under every applicable dated profile with exact report grading.

- **Dated profiles** (`purrdf_shapes::ShaclProfile`): `LEGACY` (the default,
  unchanged), `REC_20170720` and `WD_20260918`. `from_id` refuses every other
  identifier as `UnsupportedProfile`. Pre-binding admission (VALUES, MINUS,
  SERVICE, AS, hidden subquery projection) follows the selected law and
  refuses with a typed `AdmissionRefusal`.
- **Complete reports** (`CompleteValidationReport`, `CompleteReportGraph`):
  `sh:sourceConstraint`, recursive `sh:detail` with its own evidence, the
  dated message order, and blank-node correspondence back to the data or
  shapes acquisition. The shared `purrdf_validate` door adds
  `validate_complete_*`, `complete_report_payload/_to_json/_to_sarif_string`
  and `complete_validation_status`.
- **Community SHACL corpus** (`corpora/community/`, 64 cases) and the
  **conformance kit** (`crates/conformance-kit`, unpublished). The kit grades
  every complete report by graph isomorphism in source context, using the
  producer's own blank-node correspondence. The runner
  (`purrdf_sparql_conformance::community`, the `community_conformance` test
  and the `community-conformance` binary) runs each case through the fresh,
  prepared (x2) and restored-product (admitted, rebuilt) routes.
- This branch carries all of #406 (the code PR #448 also holds) and lands ahead of #448. It has merged current main (`dfc0c21ad`), including #455 (`RdfTriple: Drop`), the pre-bound-variable work (#465 and its predecessors) and the exact numeric tower with its division policy (#441); `cargo check --workspace --all-targets` is clean.

## Merge of main `dfc0c21ad` (#441, conflict resolutions)

28 files conflicted. Every hunk keeps both features: the dated XPath regex law this branch carries and main's caller-chosen division policy.

- `sparql-eval/engine.rs`: `QueryOptions` carries both `xpath_regex` and `division` (fields, `Debug`, `EMPTY`, and the builder test's signature tuple).
- CLI (`cli.rs`, `lib.rs`, `query.rs`, `update.rs`): `--xpath-regex` and `--division` on `query` and `update`, both threaded into every run.
- ShEx (`lib.rs`, `validate.rs`, `validate/node.rs`) and the CLI `shex`: the exact-bounds traversal and the native pattern cache share one `validate_using`. The new `validate_exact_with_xpath` and `validate_shape_map_exact_with_xpath` give the exact schema the dated law, so `shex --xpath-regex` and `shex.validate(xpath_regex=...)` keep main's exact numeric facets.
- C ABI: one bump, `0.10.0`. The fourteen `*_xpath_regex` entry points and the appended `PURRDF_STATUS_REGEX_RESOURCE_ERROR` now ride `0.10.0` beside main's additions; the `0.8.0` → `0.9.0` paragraph is main's again (version.rs, README). `purrdf.h` was regenerated with `make capi-header`; the ABI snapshot holds both sides' prototypes and `abi_signatures` passes; `smoke.c` runs `check_xpath_regex`, `check_xsd_exact_numerics` and `check_division_policy`. Every query, governed, entailment and update entry, including this branch's `*_entry` helpers, applies the handle's division with the selected regex law.
- wasm (`operation.rs`, `query.rs`, `async_query.rs`, `lib.rs`, `js/README.md`): `selected_options(env, xpath_regex, division)` composes main's `options_under` with the regex law at all twelve call sites; operation inputs carry both. Three of main's new tests gained the trailing `xpathRegex` argument (`None`).
- Python (`__init__.pyi`, `py_shex.rs`, `py_store/*`): every method takes both `xpath_regex=` and `division=`; the reference counts match each parent exactly.
- zh-Hans: main's catalogue, filled from this branch's by `msgmerge --compendium` against a fresh `make book-pot`. Audit: 2968 msgids, 0 lost, main's text wins on all of them (0 differ), 14 filled from this branch, 0 fuzzy, 0 untranslated, 0 obsolete; `make check-i18n` passes.
- `BENCHMARKS.md`: 35 of 107 bench targets documented (both sides' additions). `CONFORMANCE.md`: Python 4137. `purrdf-simd.md`: main's measured row, which already contains this branch's change.
- A script checked every line each parent added to each conflicted file. Every line that is not verbatim in the merge is one of the deliberate combinations above.

## Merge of main `863c75e2c` (conflict resolutions)

Main's pre-bound-variable work and this branch's dated laws met in `purrdf-shapes` and `purrdf-sparql-eval`. Each resolution keeps both sides:

- **The pre-binding lane is gone, the law stays.** Main removed the `ShaclPrebinding` lane from every SHACL door and added the `declared_prebound` names; this branch added the dated `Invocation` to the same doors. Every door now takes both (`crates/shapes/src/sparql.rs`, `components.rs`, `rules.rs`, `expression.rs`). The generic cached-select door main deleted stays deleted; the cached scalar expression uses the SHACL door with its dated invocation. `prepare_interned_request` declares the request's `declared_prebound` names, as `query_interned_view` does.
- **Main's load-time checks run under the compatibility law.** The `$shapesGraph`/`$currentShape` assignment refusal, the grouping-check declarations (`with_prebound_variables`) and the node-expression pre-bound names are folded into the `LEGACY` closures of `audit_query` (`sparql_constraint_query`, `parse_validator`, `check_construct`, `reachable_select_expression_violation`), so `LEGACY` behaves exactly as main does. The dated laws keep their own rules. The free node expressions main now walks are walked under the selected law too, and the `scope` argument main added reaches `prepare_with_expressions`.
- **Dated admission reads the query as written.** Main's preparation now moves each assignment of a pre-bound name to a fresh variable. The dated runtime law was reading that rewritten plan, so it no longer saw `BIND(… AS ?this)` (three dated-admission unit tests failed after the merge). `PreparedQuery::source_query` (new) keeps the algebra as parsed when the rewrite changed it, and `PreparedExecution::query` and the request door's admission read it.
- **The draft's unvalued shape context is the query's own variable.** Main declares an unvalued `$shapesGraph`/`$currentShape` pre-bound on every shape run. SHACL 1.2 SPARQL Extensions (18 September 2026), Appendix A, lists only `this`, `value` and the parameters as potentially pre-bound, and this branch's draft law admits an assignment of either name. With the declaration, that run answered no row: a missed result. Under `WD_20260918` the declaration is now omitted (`declared_shape_context`); `LEGACY` and `REC_20170720` keep main's. Test: `an_unvalued_shape_context_is_the_drafts_own_variable` (fails without the change), with the Recommendation's refusal and a reading neighbour under both laws.
- **One test expectation follows main's single rewrite.** `inspecting_a_borrowed_request_preserves_bound_parameters_and_rewrite_identity` expected the ordinary lane to refuse a bound `OPTIONAL` call. Main gave every lane the same pre-binding rewrite, so the ordinary lane now admits it on the same cache entry; the test asserts that, and the free request still refuses on both lanes.
- `sparql-eval` `eval.rs`, `engine.rs`, `property_fn_eval.rs` and `tests/prepared_admission.rs`: both fields and both builders kept (`xpath_regex` and `disjoint_language_strings`/`declared_prebound`); the stack test takes main's fixed 256 KiB stack and keeps this branch's message check.
- `scripts/conformance-matrix.py`: main's `_scrape` helper and this branch's `_passed_total` helper both refactored the same scrapers. `_passed_total` now delegates to `_scrape`, so there is one implementation.
- `CHANGELOG.md` keeps both sides' entries. `docs/CONFORMANCE.md` carries the merged Python count. The zh-Hans catalogue went through `make book-po-update` and `make check-i18n`: 2938 translated, 0 fuzzy, 0 untranslated, 0 obsolete.

## Community corpus result

```
COMMUNITY SHACL PROFILE shacl-20170720: executions 57 passed 57 failed 0 unsupported 0
COMMUNITY SHACL PROFILE shacl12-20260918: executions 58 passed 58 failed 0 unsupported 0
COMMUNITY SHACL TOTAL: cases 64 executions 115 passed 115 failed 0 unsupported 0 observations 535 passedObservations 535
```

Four mutation tests show the grading is not vacuous. Each corrupts one
reviewed expectation, re-binds its digest so acquisition admits it, and
requires a mismatch on every route: an omitted `sh:sourceConstraint`, a result
moved onto another data blank (the report graph stays isomorphic, so only the
source correspondence can catch it), a changed path topology, and a
lower-priority message. Each untouched original still passes.

### Corpus provenance (please read)

The corpus comes from the #384 staged layer. The bytes its earlier review
approved (inventory SHA-256 `22cebc8d…`) were not retained anywhere, and the
surviving candidate differed (`9112e6f6…`, plus ten whitespace-only manifest
edits). Before any engine run, a fresh automated agent reviewer re-derived all
64 cases from the dated clauses without seeing PurRDF output. It confirmed
every result set and field. It amended three things: the REC SERVICE reason
is now `service` (an outright prohibition), the draft SERVICE case is marked
as a declared PurRDF processor policy, and two cases' citations were
corrected. Per the owner's decision, every review record now says
**automated agent review**, never "independent". The re-review and current
inventory hash are recorded in `corpora/community/reviews/shacl.md`. The
SPARQL community suites stay with #384.

## Behaviour changes semver cannot see (CHANGELOG: Breaking Changes)

These arrive with the #406 code this branch carries:

- A native host function's own `Err`, a caught panic, a resource ceiling or an
  invalid host protocol response now surfaces as
  `EvalError::FunctionOperational` instead of `EvalError::Function`.
  `Function` keeps only request refusals (arity, type, access mode).
- `EvalError::diagnostic_code()` / `code()` now return codes for `Internal`,
  `CompositeBound` and `FloatEnvironment`, where they used to return `None`.
  They also return codes for the new `FunctionOperational` and `XPathRegex`.
  Hosts used to see `native-sparql-query-eval` for these failures.

The old classification is not restored, because SHACL validation alternatives
depend on telling an execution failure from a refused request. Both changes
are listed under **Breaking Changes** in `CHANGELOG.md`, and v4.0 is next.

## Criteria

| # | Criterion | Evidence | Status |
|---|---|---|---|
| 1 | Profile selector, additive, default unchanged | `crates/shapes/src/profile.rs:30` (`ShaclProfile`), `:60` (`from_id` refuses unknown); `crates/shapes/tests/profile_admission.rs:33`, `crates/shapes/tests/dated_execution.rs` | MET |
| 2 | Richer report door, additive; legacy types unchanged | `crates/shapes/src/report.rs:553` (`CompleteValidationReport`), `:719` (`CompleteReportGraph`); `crates/validate/src/complete.rs`; legacy parity `crates/validate/tests/complete_shacl.rs` (`existing_default_sarif_body_is_the_complete_compatibility_projection`) | MET |
| 3 | `sh:sourceConstraint`, message precedence, blank identity of report contexts | `crates/shapes/src/report.rs:2179`; `crates/shapes/src/sparql.rs:217` (`row_message`); `crates/shapes/src/report.rs:660` (`CompleteBlankLabels`). Public-door proofs: the corpus's `solution-message-priority` case and `a_missing_row_message_is_a_mismatch`, `an_omitted_source_constraint_is_not_excused`, `a_wrong_source_blank_is_not_repaired_by_report_isomorphism` (`crates/sparql-conformance/tests/community_conformance.rs:248-286`) | MET |
| 4 | Every `sh:ValidationResult` property (plus annotations) on one result, public API | `crates/validate/tests/complete_shacl.rs:407` `one_complete_result_carries_every_validation_result_property` (see note 1) | MET |
| 5 | Pre-binding admission per profile with required-rejection tests | `crates/shapes/src/prebinding.rs`, `crates/shapes/src/shapes/parser/admission.rs`; `crates/shapes/tests/profile_admission.rs`; the corpus's 12 admission cases | MET |
| 6 | Every refusal paired with a valid neighbour | `profile_admission.rs` pairs; corpus rec/draft pairs; runner refusals in `community_conformance.rs:92,134,179,288`, each with an admitted neighbour | MET |
| 7 | Community SHACL corpus wired into the community runner (64 cases, REC 57 + WD 58 = 115) | `crates/sparql-conformance/src/community.rs`; `community_conformance.rs:33` asserts 64/115/57/58 exactly; matrix row `Community SHACL dated profiles (REC 2017 / WD 2026)` | MET |
| 8 | Unsupported profiles reported separately from failures | `community.rs:842` (`select` → `Selection::Unsupported`), `Totals.unsupported`; `community_conformance.rs:92` | MET |
| 9 | Exact report comparison: graph isomorphism plus required fields | `crates/conformance-kit/src/report.rs` (`compare_contextual`, `compare_with_policy`); `community.rs:1008` (`compare_report`); mutation tests above | MET |
| 10 | Python and WASM expectations unchanged under the default profile | At head `e79fef6ba`: `make wasm-pkg-test`, with the containment guard admitting the private capture: npm interface checks 426/426, packed-tarball smoke, identity ABI receipt. `make pytest` (extension rebuilt): 4102 passed, 1 skipped, 4 xfailed. After the main merge, at the merged tree: Python suite with the extension rebuilt (`maturin develop`), 4109 passed (4137 after the #441 merge), 1 skipped, 4 xfailed, which matches the matrix count. The merge touches no binding code; WASM was not re-run (owner rule: targeted checks only). See note 2 | MET |
| 11 | `cargo semver-checks` passes for shapes and validate (vs `rust-v3.0.1`) | `cargo semver-checks check-release -p {purrdf-shapes,purrdf-validate} --baseline-root <rust-v3.0.1 worktree @ dcd24b1cc> --release-type minor` re-run on the merged tree (main `863c75e2c` merged): 196 pass / 58 skip for each, "no semver update required" (first run log below) | MET |
| 12 | Behaviour changes semver cannot see are flagged | `CHANGELOG.md:11` Breaking Changes (above) | MET |

Note 1: SHACL defines `sh:sourceConstraint` only for a SPARQL-based
constraint's result, and only Core's nesting components produce `sh:detail`,
so no single node can carry both. The test asserts all nine on one result
tree: the member-shape parent carries eight (focus node, result path, value,
source shape, source constraint component, severity, message, detail), and its
SPARQL detail carries `sh:sourceConstraint` and a result annotation. It checks
the exact predicate set the complete graph emits on each node.

## Verification (targeted, per owner rule)

At head `a0f9a070b` (main `dfc0c21ad` merged):

- `cargo test`: purrdf-shapes 2106, purrdf-sparql-eval 2805, purrdf-validate 310, purrdf-shex 241, purrdf-cli 553, purrdf-capi 181 (including `c_smoke` and `abi_signatures`), purrdf-wasm 262. All pass, 0 failed.
- `community_conformance`: 115/115 executions, 535/535 observations.
- Python suite, extension rebuilt: 4137 passed, 1 skipped, 4 xfailed. This includes `test_division_policy.py` and the SHACL report tests.
- `cargo clippy -D warnings` for shapes, validate, shex, cli, capi, sparql-eval, wasm and python: clean. wasm32 clippy for purrdf-wasm: clean.
- `cargo check --workspace --all-targets`: clean.
- `scripts/check-generated.sh --check`, `capi-header.py --check`, `make check-i18n`, check-wasm-js-exports, check-python-stub-parity, check-issue-refs, `conformance-matrix --self-test`, check-versions and every pre-commit gate: pass.


At the earlier merged tree (head `737cc1358`):

- `cargo test -p purrdf-shapes`: 2012 passed, 0 failed
- `cargo test -p purrdf-sparql-eval`: 2765 passed, 0 failed
- `cargo test -p purrdf-validate`: 310 passed, 0 failed
- `cargo test -p purrdf-sparql-conformance --test community_conformance`: 10 passed; REC 57/57, WD 58/58, 115/115 executions, 535/535 observations
- Python suite, extension rebuilt: 4109 passed, 1 skipped, 4 xfailed (the SHACL report tests included)
- `cargo clippy -p purrdf-shapes -p purrdf-sparql-eval -p purrdf-validate -p purrdf-sparql-conformance --all-targets -- -D warnings`: clean
- `cargo check --workspace --all-targets`: clean
- `make check-i18n`: all 6 gates pass; `check-doc-claims.py`, `check-issue-refs.py`, `conformance-matrix.py --self-test`, `check-i18n-glossary.py` and every pre-commit gate pass
- `cargo semver-checks` (shapes, validate vs `rust-v3.0.1`): 196 pass / 58 skip each, no semver update required

Before the main merge:

CodeRabbit skipped this PR (351 files exceed its 300-file limit), so there are no review threads.

- `cargo test -p purrdf-sparql-conformance --test community_conformance`: 10 passed
- `cargo test -p purrdf-conformance-kit`: 23 passed
- `cargo test -p purrdf-validate --test complete_shacl`: 9 passed
- `cargo test -p purrdf-shapes --test w3c_conformance --test w3c12_conformance --test conformance`, `cargo test -p purrdf-iri`, `cargo test -p purrdf-sparql-conformance --test manifest_include --test suite_discovery --test suite_inventory`: pass
- `cargo clippy -p purrdf-iri -p purrdf-conformance-kit -p purrdf-sparql-conformance -p purrdf-shapes -p purrdf-validate --all-targets -- -D warnings`: clean
- `scripts/check-shared-helpers.py`, `check-doc-claims.py`, `check-issue-refs.py`, `conformance-matrix.py --self-test`, every pre-commit gate, and the non-Rust ratchet: pass. The matrix script shrank, because its four identical `passed N total N` scrapers now share one helper.

Note 2, the WASM containment failure: the guard in
`scripts/build-private-wasm.py` was right. The host's Stage `cargo` wrapper
drops any caller `--target-dir` and forces its own leased slot, so Cargo's
registered output landed outside the private capture directory. With the
toolchain's own `cargo` first on `PATH`, the capture is admitted unchanged.
The guard, Makefile and CI need no change, because hosted CI has no such
wrapper. The same wrapper explains why `--baseline-rev`/`--baseline-root`
semver-checks found no rustdoc output earlier.

Semver log (reproducible; the toolchain's `cargo` first on `PATH`):

```
# head 4d050371aac1a2d59a7ea505e1af8c490ced954b
# baseline dcd24b1cc56cd7ac24cbe0d38dcf5a89022eaccf (rust-v3.0.1)
# cargo 1.100.0-nightly (7941be6fb 2026-09-11); cargo-semver-checks 0.50.0
## cargo semver-checks check-release -p purrdf-shapes --baseline-root <rust-v3.0.1> --release-type minor
     Checked 196 checks: 196 pass, 58 skip
     Summary no semver update required
## cargo semver-checks check-release -p purrdf-validate --baseline-root <rust-v3.0.1> --release-type minor
     Checked 196 checks: 196 pass, 58 skip
     Summary no semver update required
```

Closes #402.



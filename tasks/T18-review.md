# T18 independent review (Addendum A Task 18; G-07, G-14)

Reviewed: worktree /home/paudley/Active/purrdf/.worktrees/570-t18, branch paudley/570-t18, commit ce4b72357 on 155cf3c3c (20 files). No source modified.

## Evidence run (all in the side worktree)

| Check | Result |
|---|---|
| `cargo test --locked -p purrdf-xsd -p purrdf-sparql-eval` | 121 "test result: ok" blocks, no failures (incl. 1638 sparql-eval lib tests) |
| `cargo test --locked -p purrdf-sparql-conformance` | 27 ok blocks, no failures (governor_corpus, data_r2_harness, evaluator-trace goldens, seconds-01 by value) |
| `cargo test --locked -p purrdf-cli --test query_cli --test exact_numerics_cli` | 8 + 63 passed |
| `cargo test --locked -p purrdf-capi` | 7 ok blocks (C smoke expects `05.123456789012345678901`) |
| `python3 scripts/check-corpus-frozen.py` | OK: 30 vendored corpora byte-frozen |
| `python3 scripts/check-doc-claims.py` | OK: 154 claims |
| `make check-i18n` | OK, all 6 gates; 3048 translated, 0 fuzzy, 0 untranslated, no drift vs fresh template |
| clippy `-D warnings` on xsd + sparql-eval (all targets) | clean |

## Findings by item

1. SECONDS. `purrdf_xsd::temporal::seconds_field_text` is the single home. The old `prepare_seconds_field_text` / `as_seconds_field` / `borrowed_temporal_seconds_to_term` and its test are removed, with no leftover callers. The function calls `split_tz` first: `Z`, then a 6-byte `+-hh:mm` tail checked for a `:` at index 3, with digit and range validation. Only then does it take the text after the last `:`, so a zone colon cannot mis-split the field. Traced by hand:
   - `-00:00` and `+05:00` split cleanly.
   - Zoneless `…05` and `…05.5` have a 6-byte tail starting with `:` or a digit, so they are not mistaken for a zone.
   - A negative year (`-0044-03-15T10:30:07.0Z`) has no `:` before the time, so it is safe. It is covered by a new test.
   - The same function serves dateTime and time (a time-shaped body is just `hh:mm:ss`).
   - The field must pass `is_decimal_lexical`, otherwise the result is `None`. The caller only reaches it after the value has parsed, so the lookup cannot miss for valid input; malformed input never gets this far (the parse already returned error/None).
   - Computed dateTimes are interned literals, so their canonical lexical form is read.
   - New tests cover `05.500`, `00.250`, `05.500-05:00`, `07.0` and the existing `00`/`05`/`13.815`.
   - SPARQL 1.1 seconds-01 stays graded by value, so it passes. The SHACL `seconds-example` "00" holds, because the integer-only field stays `00`.
   - Docs state honestly that no spelling satisfies both term for term.
   - Gap (non-blocking): there are no direct unit tests on `seconds_field_text` in xsd for `None` paths or odd forms; coverage is via the evaluator tests only.
2. Unary plus. `unary_numeric_term` returns the operand term itself when the op is Plus and the value is numeric, so it spends nothing. A non-numeric operand (`"4"^^xsd:string`) and an ill-typed numeral (`"four"^^xsd:integer`) still give None/error, and the new test asserts both. W3C `result-unplus-2.srx` spells the decimal, double and float results as `3`, which supports keeping the operand spelling. The harness change in data_r2_harness.rs is correct: unplus-2 expects the operand spelling at every rank, and the `exponential` placeholder `""` is unused for unplus. Unary minus still computes (`-4.0`). Two further new tests pin STR vs xsd:string cast. Note: `+` is also an admitted step before, but it now returns before `numeric_step_admitted`, which matches "charges nothing" in the notes.
3. `computed_numeric_render_cost`. Its predicate (`unscaled` not i128-representable, or `scale > 18`) mirrors the wide-decimal branch of `numeric_render_cost`. For a wide whole decimal it adds exactly 2 bytes (`.0`), via `render_shape_spelled` with `extra = 2` only when `scale == 0`. Fractions and small values fall through unchanged. The test emits via `prepare_computed_numeric_text` and asserts the text length is canonical + 2, bytes delta = 2*6 (the per-byte charge), work increases, and the neighbours are unchanged. `governed_term` selects it on `computed`. Exact.
4. Governor v22. `GOVERNOR_PROFILE_VERSION` is 22, with a doc-comment "# v22" note, a v22 row in docs/SPARQL-GOVERNOR-PROFILE.md, and the unit test pin updated. I recomputed the documented profile recipe (`purrdf-sparql-governors\n22\n…`) and got `a1c8cc38…ef44b`, which equals the digest in the doc. `sha256sum` of `scripts/conformance-frozen/vectors-sparql-governors.sha256` is `d8dba125…c4ac`, which equals both `GOVERNOR_CORPUS_DIGEST` and the doc §11.1. The regenerate procedure (`PURRDF_UPDATE_GOVERNOR_CORPUS=1 cargo test … governor_corpus`, then `check-corpus-frozen.py --update`, then re-pin) is documented in governor_corpus.rs; the diff matches that shape: 4 expected files plus manifest.tsv (fuel 700 to 621, 699 to 620) and the freeze manifest rows updated, with `check-corpus-frozen.py` passing and governor tests green. `vectors/sparql-governors` is first-party; no W3C-frozen vendor tree was touched. The trace goldens (governors.trace, suite.trace) are regenerated and their tests are green. The numeric-default case (fuel 1763) did not move, which is plausible since it has no wide-whole computed decimal.
5. Docs and i18n. querying.md, CONFORMANCE.md and the governor profile are consistent with the code (SECONDS as written, unary plus identity, STR vs xsd:string cast, text/retrieval scores' fixed 12-digit fraction). `make check-i18n` passes with 0 fuzzy and 0 untranslated. The commit message and diff do not carry a visible translator-review flag for the ~5 changed entries; the .po has no `Last-Translator` or fuzzy marker for them. Recommend relaying "5 zh-Hans entries need native-translator review" in the PR text (non-blocking).
6. Host expectations. C smoke, wasm JS (`division-policy.test.mjs` lines 60, 141), Python (`test_division_policy.py:72`) and CLI (`exact_numerics_cli.rs:133`) all already expect `05.123456789012345678901`, i.e. the lexical form. `3.0E0` remaining in operator_mapping_spec.rs:695 is a double cast, not unary plus, and is correct. No host test expects the old unary-plus re-spelling. The JS and Python suites were not executed here (needs npm/maturin toolchains); grep-level only.

## Residual non-blocking notes
- No direct `seconds_field_text` unit test in `purrdf-xsd` (edge cases only exercised through the evaluator).
- Translator-review flag for the 5 .po entries is not recorded in the commit.
- JS/Python host tests verified by reading, not execution.

VERDICT: PASS

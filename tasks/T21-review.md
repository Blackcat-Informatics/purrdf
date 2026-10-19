# T21 independent review (A: Task 21 / B: SPARQL keyword arity / C: U10 hasLang)

Worktree 570-submit-w3c-shacl-implementation-reports, uncommitted diff on 6912f1aeb. No source modified.

## Evidence run

| Check | Result |
|---|---|
| `cargo test --locked -p purrdf-sparql-conformance -p purrdf-sparql-algebra -p purrdf-sparql-eval` | EXIT=0, all binaries ok (W3C syntax/eval suites included) |
| `cargo test --locked -p purrdf-shapes --no-fail-fast` | EXIT=0, 91 binaries ok. `W3C12 TOTAL: passed 590, spec-contradicted 1, xfailed 0, ledger 0`; `W3C12 SHACL root: passed 381, spec-contradicted 1 of 382`; `class NodeExpr: passed 145 of 146`; Core 162/162, SPARQLValidation 33/33, InferenceRules 31/31, SPARQLNodeExpr 5/5, SPARQLCustomFunctions 5/5; `W3C12 UNLISTED: passed 1, exact 1, with-delta 0, total 1` |
| `cargo test --locked -p purrdf-xsd` | EXIT=0 |
| CLI: `w3c12_validate_cli`, `w3c_srl_check_cli`, `shacl_profile_cli` | ok |
| CLI: `w3c_node_expr_cli` | **FAILED** (finding B1) |
| `python3 scripts/check-doc-claims.py` | OK, 154 claims |
| `python3 scripts/conformance-matrix.py --self-test` | OK |
| helper-census `--non-rust-ratchet` (merge-base with origin/main) | OK, nothing legacy grew |
| `cargo clippy -p purrdf-sparql-algebra -p purrdf-shapes --all-targets -D warnings` | EXIT=0 |
| `make -n w3c-earl` | recipe expands as intended |

## B. Keyword arity (crates/sparql-algebra)

Grammar fetched from https://www.w3.org/TR/sparql12-query/ (rules [141]-[144]) and compared with `builtin_arity`.
Spot-checked 30+ functions, all match: STR/LANG/LANGDIR/DATATYPE/IRI/URI/ABS/CEIL/FLOOR/ROUND/STRLEN/UCASE/LCASE/ENCODE_FOR_URI/
YEAR..TZ/MD5/SHA1..SHA512/isIRI.../hasLANG/hasLANGDIR/isTRIPLE/SUBJECT/PREDICATE/OBJECT = 1; LANGMATCHES/CONTAINS/STRSTARTS/
STRENDS/STRBEFORE/STRAFTER/STRLANG/STRDT = 2; STRLANGDIR/TRIPLE = 3; BNODE 0-1 (`'(' Expression ')' | NIL`); RAND/NOW/UUID/STRUUID
= 0 (NIL); CONCAT/COALESCE ExpressionList (0..n); SUBSTR/REGEX 2-3; REPLACE 3-4; IF 3; sameTerm 2; BOUND 1. ADJUST (2) and SHA3-* (1) are
not in the grammar (SEP extensions); the doc comment says so and the choice is the documented SEP signature.
Every keyword `builtin_function` resolves is in the hand-listed `every_keyword_builtin_has_an_arity` test (I diffed the list against the
dispatch table; none missing). The parser test covers each arity class with the neighbouring right/wrong counts.

- **B1 (BLOCKER).** `crates/cli/tests/w3c_node_expr_cli.rs::every_w3c_node_expression_runs_through_the_cli` fails:
  `FAIL [node-expr/shnex-sparql/hasLang-example]: ... <sparql#hasLang> is the SPARQL hasLANG function and takes exactly 1 argument(s), got 2`
  (`passed 142, failed 1`, `NODE_EXPR_CASES = 143` at line 40, `assert_eq!(passed, NODE_EXPR_CASES)` at line 146). The test was not updated
  for U10: it needs the same SPEC_CONTRADICTED treatment (expect the refusal with the rationale for that id, pass count 142, plus the
  one-argument neighbour), ideally reading the one table rather than copying it. Until fixed, `cargo test -p purrdf-cli` is red and the task's
  "CLI count tests" requirement is unmet. Check also Python/WASM host suites for the same case (grep found no `hasLang` there).
- **B2 (low).** The arity rule is not entirely in one place: `builtin_keyword_arity` hard-codes IF 3, sameTerm 2, BOUND 1, COALESCE any in
  addition to `builtin_arity`, while the parser enforces those through `ArgsKind::If/SameTerm/Coalesce` separately. The "one home" claim in the
  `expect_builtin_arity` doc is true for the `Function` variants only. Acceptable, but a third copy of 3/2/1 exists.
- **B3 (low).** `builtin_arity` ends in `_ => return None`; a new `Function` variant produced by keyword dispatch would silently skip the check.
  The exhaustiveness test relies on a hand-maintained keyword list, not on the dispatch table.
- The `Concat` row has min 0, matching `ExpressionList` (NIL allowed). Correct.
- Shapes side: `SparqlCallForm::render` calls `builtin_keyword_arity`, so the load-time error message and parser agree; `a_sparql_ns_keyword_function_with_the_wrong_arity_is_a_load_error`
  and the query-level split in `custom_node_expressions.rs` pass. The decision "wrong arity fails the query; a value error is unbound" is documented on
  `invoke_native_list_function` and pinned by a test with neighbours. No objection.

## C. U10 hasLang-example

- Rationale quotes the real production: rule [141] `'hasLANG' '(' Expression ')'` verified verbatim; section 17.4.2.10 is correct and the prototype is
  `hasLANG (RDF term term)`, i.e. one parameter named `term`. Examples there are `hasLANG("abc"@en)` etc. Accurate.
- Harness: entry in `SPEC_CONTRADICTED`, count pinned 1, graded unamended, XPASS refused (`spec_contradicted_entries_fail_unamended`),
  unknown id refused, `earl:failed` with `earl:info` (`earl:info exactly for SPEC_CONTRADICTED entries` test) all pass in the shapes run. The scoreboard
  lines (590 / 1 / 381 of 382 / NodeExpr 145 of 146) match the triage record in `tasks/T3-triage.md` section 5.
- G1 (1): holds for a processor that applies the SPARQL grammar, which is what the rationale says ("a processor that applies the grammar's arity").
  It is not airtight as "no conforming processor": the Node Expressions text (section 5) only says evaluation "follows the SPARQL semantics for the corresponding
  SPARQL function" with each list item passed "to the SPARQL function implementation" and defines no arity rule, and the vendored vocabulary's own
  `rdfs:comment` for `sparql:hasLang` ("has a specified language, matching the literal's language tag") reads as a two-argument function. A processor that
  defines an extra language-range argument, or ignores extra arguments, produces `true` without contradicting any sentence of the Node Expressions
  document. The draft WG issue already lists those as options 2 and 3, which is honest, but it means G1 (1) rests on the SPARQL grammar plus Patrick's
  U10 choice, not on a proof of impossibility. **Medium (judgement, not a blocker)**: the triage sentence "a processor that conforms to the grammar cannot
  evaluate ... the only way is to drop or ignore an argument, which is ... a deviation" should note the vocabulary comment and the Node Expressions silence, so the
  claim is tied to evidence. Patrick chose the option knowing the issue is open.
- G1 (2) holds (claimed, implemented function; malformed call). G1 (3) holds (one-argument `hasLang` passes via `node-expr/shnex-sparql/hasLang`, parser neighbour test).
  G1 (4) drafted, not filed (`/home/paudley/Active/w3c/contributions/data-shapes/purrdf-earl/drafts/wg-issue-hasLang.md`); U11 overrides G3.
- Baseline ledger 0 to 1 in `scripts/conformance-baseline.json`: the matrix gate requires the live XFail/Skip count to EQUAL the budget, and that
  column counts spec-contradicted entries, so with U10 the increase is forced; the only alternatives (exclude spec-contradicted from the count, or fix by
  dropping the argument) hide the entry or contradict U10. It is the minimal honest consequence, but it is a ratchet loosening that must be called out in
  the PR text. **Low**: the docs say "0 ledgered" for the same row (the XFAIL ledger) while the baseline key is `"ledgered": 1`; the note explains it but the
  vocabulary is inconsistent.
- `check-doc-claims.py`: the XFail/Skip cross-check is tightened from `== 0` to `== pinned SPEC_CONTRADICTED_COUNT`, read from the harness, so drift in either direction
  still fails; the book regex gained `1\.2 suite` and `[,)]` (more specific, not looser); README/CONFORMANCE regexes now require the spec-contradicted text and
  compare to the pinned count. Not weakened. The script passes.
- zh-Hans.po: large reflow diff (528 lines); not re-translated by me. Doc-claims and the book gate pass. Low: confirm `msgfmt --check` / the book's po gate in CI.

## A. Task 21

- G-12 cache keys: globs are valid uv `file` globs; `bindings/python/{Cargo.toml,rust-toolchain.toml}`, root `rust-toolchain.toml`, `crates/*/spec/` exist.
  I scanned every `include_str!/include_bytes!` target under `crates/`: the non-test ones (spec/, tests/fixtures, crate `*.md`, `src` files) are covered.
  **Low**: the comment says the keys "cover every directory an `include_*!` target lives in"; test-only includes in `vectors/`, `corpora/`, `crates/iri/unicode`,
  `crates/*/tests/*.txt` are not keyed (e.g. `crates/validate/src/rules.rs:275` and `crates/lex/src/unicode/casing.rs:159` are `#[test]` code only, so no wheel effect).
  Reword to "every directory a non-test target lives in", or add `../../vectors/**`.
- G-13: footprint tests, complex-path closed tests (inverse, sequence, zeroOrMore with oneOrMore neighbour), and the two extra 2017-refusal positions (property shape,
  `sh:property` reifier; `written_at` 4 to 6) exist and pass, each with a neighbour. The `GOLDEN_LEN` history line (+8 for the empty entailment rule-set count) is present.
- G-15: shacl-shacl rationale corrected to the profile-dependent rule; JSON Schema loss subject now the owning shape (`owning_shapes`, test with constant neighbours); the
  `INVARIANCE_EXCEPTIONS` exemption is replaced by `normalize_offsets`, which rewrites only digits after `at byte ` and compares everything else exactly; unit test shows
  offset-only differences compare equal and other differences do not; `DECLARED_REFUSAL_CASES` 13 to 14 is consistent. Pass.
- G-16: `make w3c-earl` does `rm -f` of both outputs before the run, then `test -s` for both inside the `&&` chain after cargo test, so a missing or empty file exits 1;
  names match the `SUITE.file_name` values in both harnesses. `$$dir` is defined in the same shell line (continuation), so it is in scope. Correct.
- T16 `exhaustive()` doc: accurate with `findFirst`/`matchAll` decision. T18 `seconds_field_text` unit tests (14 cases) pass. T19 rule-set note in CHANGELOG/CONFORMANCE
  plus `is_rule_set` shared helper (both call sites use it) pass; T17 identical-expression-node test passes. Built-in arity-vs-evaluation decision documented.

## Verdict

VERDICT: BLOCKED

Blocking item: B1, `crates/cli/tests/w3c_node_expr_cli.rs` still expects all 143 node-expression entries to pass through the CLI and fails on `hasLang-example`.
Everything else passes; B2, B3, the G1 (1) wording, the cache-key comment, and the "0 ledgered" vs `ledgered: 1` wording are non-blocking.

## Recheck (B1 and lows 1-4)

| Item | Result |
|---|---|
| B1: `SPEC_CONTRADICTED` / `SPEC_CONTRADICTED_COUNT` now live in `crates/shapes/src/shacl_corpora/shacl12.rs` (one home), used by `w3c12_conformance.rs` and `w3c_node_expr_cli.rs` | Fixed. The CLI lane grades the entry unamended, refuses an XPASS, and checks passed + contradicted = 143. `cargo test --locked -p purrdf-cli --test w3c_node_expr_cli`: 3 passed, including the new binary-level test (two-argument `hasLang` exits 1 naming "takes exactly 1 argument"; one-argument neighbour exits 0 and prints `true`) |
| B2: IF/sameTerm/BOUND/COALESCE arities | Fixed. `FORM_ARITIES` plus `form_arity` hold them; `expect_arity_range` is the one enforcement function, used by the `ArgsKind::If`/`SameTerm` arms |
| B3: wildcard in `builtin_arity` | Fixed. No `_ =>` arm remains in `builtin_arity` (exhaustive over `Function`) |
| G1 (1) caveat | Added to `tasks/T3-triage.md` ("Caveat on (1)") |
| Wording "0 ledgered expected failures, 1 spec-contradicted" | Present in README.md and docs/CONFORMANCE.md; `check-doc-claims.py` OK (154 claims) |
| cache-key comment | Now exact: keys cover non-test `include_*!` targets and names the test-only ones as not keyed |
| `cargo test --locked -p purrdf-sparql-algebra` | EXIT=0, 26 result lines ok |
| `cargo test --locked -p purrdf-shapes --test w3c12_conformance` | 29 passed; `W3C12 TOTAL: passed 590, spec-contradicted 1, xfailed 0, ledger 0`; `SHACL root: passed 381, spec-contradicted 1 of 382` |

I did not rerun clippy, the xsd tests, the matrix self-test or the ratchet after these edits; the coordinator should include them in the final gate. The zh-Hans.po reflow is unchanged and still unreviewed beyond the doc-claims gate.

VERDICT: PASS

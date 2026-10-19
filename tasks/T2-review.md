# T2 review: SHACL 1.2 pre-binding admission (uncommitted diff over c5f2529a6)

## Scope
`git diff` over: crates/shapes/src/prebinding.rs (comment only), shacl_corpora/{mod,report_grading,shacl12}.rs
(new `W3cCase.profile`, set to WD_20260918 for 1.2 `sht:Validate`), crates/shapes/tests/dated_execution.rs
(new test), docs/CONFORMANCE.md. No source modified by this review.

## 1. Normative reading
shacl12-sparql/index.html, `data-syntax-rule="pre-binding-limitations"` (line 2219): "potentially pre-bound
variables includes the variables this, value (for ASK queries), any variables that represent the parameters of
the constraint component ..., and the scope variables of SPARQL-based node expressions (such as arg0)";
"SPARQL queries MUST not contain a VALUES clause that mentions any potentially pre-bound variable". Change-log
(line 2540): "VALUES clauses are only disallowed when they mention pre-bound variables and removed the
restriction on sub-SELECTs".
`Rules::AppendixA` refuses a `GraphPattern::Values` iff its variable list contains a pre-bound name; the walker
is the shared iterative one, so nested sub-SELECT / EXISTS are covered. "Mentions" = in the VALUES variable list
is the natural reading (a VALUES clause cannot otherwise mention a variable; its data rows are constants).
Verdict: matches the text. pre-binding-009 uses `VALUES ?value` with a constraint query (`value` is only
pre-bound for ASK validators), so it is admitted; unsupported-sparql-002 (`VALUES $this`) is still refused.

## 2. Profile scope
- Validate cases: all 213 sht:Validate go through `parse_case` -> profile WD; `load_shapes` and
  `validate_case` apply it (load via `from_dataset_with_options`, run via the options stored with the shapes).
  204 pass, the 6 declared sht:Failure entries still fail correctly, 3 more pass. Complete.
- EvalNodeExpr / Infer / SRL: executed in tests/w3c12_conformance.rs (`eval_node_expr_case`, `infer`) and
  rules_conformance / CLI tests with the default (LEGACY) options; no profile is threaded. Only two 1.2 vector
  files contain VALUES (pre-binding-009, unsupported-sparql-002), both Validate, so no node-expr / infer / SRL
  result depends on the law today. See finding F1.
- Only `W3cCase` construction site is `parse_entry` (LEGACY default, so SHACL 1.0 `w3c_cases`, the 2017 harness
  w3c_conformance, first-party corpus and the result-annotation fixtures tests keep the compat law). The field
  is public; all consumers use `..`-free struct reads only via field access, nothing broke (workspace builds,
  tests below). Unlisted vendored file (rectangle-condition) is sht:Infer, unaffected.
- Real callers of `validate_case`/`run_validate_case`: w3c_conformance.rs, w3c12_conformance.rs,
  vocabulary_import_invariance.rs (via own loader), plus fixture tests. CLI tests w3c_node_expr_cli /
  w3c_srl_check_cli do not touch Validate cases.

## 3. sparql-failure-001, sparql-message-001
Genuine. `crates/shapes/src/sparql.rs:290-292` and `components.rs:706` read the `message` / `failure` columns
only `if dated` (SparqlReportContext::dated); the spec text (`failure` bound true -> failure; query-bound
`message` overrides sh:message) is implemented in the dated branch and already covered by dated_execution tests
(e.g. dated_some_value_still_suppresses..., failure BIND tests). Under LEGACY the columns are ignored, which is
exactly why they failed before ("engine validated successfully"; "no produced result"). Not accidental.

## 4. Ledgers
product_corpus_equivalence, vocabulary_import_invariance, shacl_shacl_differential load shapes through
`from_dataset_with_base` / their own parse (compat law, `Rules::Strict`, refuses every VALUES), never `tc.profile`.
So the pre-binding-009 refusal text ("refuses VALUES with pre-bound variables, stricter than the WD rule") is
accurate for those lanes. All three tests pass (both-direction enforcement of the ledgers, so a stale/XPASS
entry would fail; none does). Neither newly passing case was in any of those ledgers.

## 5. New test
`values_is_refused_only_when_it_mentions_a_pre_bound_variable_under_the_draft_law`: WD refused ($this, parameter
variable `argument`, typed AdmissionReason::Values + variable name) and admitted+run (unbound ?value; parameter
component over name `other`); REC refuses both the local and $this shapes at load. Covers refused + neighbouring
admitted under 1.2 and both refused under 2017. Nested depth is covered by profile_admission.rs (EXISTS case).
Passes.

## 6. docs/CONFORMANCE.md
574/591 matches the run (574 passed + 6 non-canonical + 11 failing = 591); the eleven named entries equal the
FAIL list of the run exactly; matrix row and prose updated consistently; no process text. Cosmetic: re-wrapped
sentence leaves ragged short lines.

## Test evidence
- w3c12_conformance: 17 pass; the 2 failures are `w3c_shacl12_conformance` / `every_planned_inventory_id_passes`,
  failing only on the 11 entries later tasks own (closed-001, 4 rules-entailment, blankNodeArgument-001,
  singleInstanceOf-001, deactivated-select-001, prefixes-003, stratification test_13/14). Baseline (stash) had
  14 failures, 571 passed; now 574. W3C12 TOTAL: passed 574, non-canonical 6, ledger 0.
- w3c_conformance, dated_execution (16), profile_admission (14), lib prebinding (18): ok.
- product_corpus_equivalence, vocabulary_import_invariance, shacl_shacl_differential: ok.
- cli w3c_node_expr_cli: ok. w3c_srl_check_cli: fails on stratification test_13/14 identically before and
  after the change (pre-existing, unrelated, owned by a later task).

## Findings
- F1 (Low, informational): EvalNodeExpr / Infer / SRL harness paths run with the default LEGACY options while the
  1.2 Validate lane is dated. No vector depends on it today (no VALUES in those lanes). Consider a one-line
  note in the harness docs (or threading WD) so a future VALUES case there is not graded under the wrong law.
- F2 (Low): `W3cCase.profile` is only honoured by `report_grading`; the other loaders keyed off W3cCase
  (vocabulary_import_invariance, product_corpus_equivalence, shacl_shacl_differential) deliberately ignore it. The
  field doc says the case "is loaded and validated under" the profile, which is slightly broader than true.
  Consider "the harness grades under".
- No blocking findings.

VERDICT: PASS

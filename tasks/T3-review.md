# T3 independent review

Scope: uncommitted changes since 95bb5c455 in the worktree (32 files modified, one new
test file `crates/shapes/tests/shape_deactivation_expression.rs`), against
`tasks/T3-triage.md` and plan "Task 3" (G1, U3). Source was not modified.

## Evidence

### Normative quotes (shacl12-core, -sparql, -node-expr, -inference-rules `index.html`, checkout 4cd469867a)

Every quote in T3-triage.md was matched against the HTML reduced to text (markdown
backticks in the source are the only difference). All real:

- Core "Deactivating Shapes and Constraints": at-most-one-value sentence; "node
  expression that must have either true or false as the (only) output node"; "produces
  true as its only output node, the shape is called deactivated"; "Deactivated shapes
  are ignored during validation"; the reifier paragraph ("at most one reifier with a
  value for the property sh:deactivated", "the constraints that use the triple are called
  deactivated constraints"); the non-normative "In SHACL Core, the only valid values ...
  constant literal node expressions" (the text does say "The remainder of this section is
  non-normative" immediately before it, as the triage states); "SHOULD produce a failure"
  for ill-formed nodes.
- SPARQL: SPARQL-based constraint `sh:deactivated` sentence and "then the constraint is
  deactivated and there are no validation results"; the prefix path
  `sh:prefixes/(^owl:versionIRI)?/(owl:imports/(^owl:versionIRI)?)*/sh:declare`, the
  "navigate from the version IRI" sentence, "MUST produce a failure ... cannot be
  parsed"; "Note that blank nodes cannot be passed as arguments ... evaluation failure
  and thus an error"; "the argument will be unbound in the scope"; the exactly-one-member
  rule ("Otherwise, or if the evaluation produces an evaluation failure, the result of
  the SPARQL function call is an error").
- Node Expressions §6.2: "An evaluation failure must be produced if there is more than
  one output node."
- Inference Rules: "Rules may be deactivated by setting sh:deactivated to true";
  "The values of sh:deactivated are either of the xsd:boolean literals true or false";
  "linked and non-deactivated shapes" (appears 4 times, including the rule-execution
  sentence quoted).
- 2017 REC (`shacl/index.html`): "The value of sh:deactivated in a shape must be either
  true or false."

Each quote supports the behavior claimed. The "SHOULD produce a failure" reading for
empty/multi/non-boolean results is recorded in the triage as an engine choice with the
opposite reading acknowledged as permitted, and as candidate editorial clarification 2
(whether a dynamic violation is a failure, ill-formed, or "not deactivated").

### Neighbouring valid cases for each refusal (executed, all green)

- REC 2017 refusal: `the_2017_recommendation_allows_only_a_boolean_sh_deactivated`
  refuses the expression on a shape, a SPARQL-based constraint and a reifier, while
  `true`/`false` load under 2017 and the same expression loads under the WD 2026 profile.
- Rule's own `sh:deactivated` boolean-only: `a_rule_s_own_sh_deactivated_is_a_boolean`
  (expression refused citing Inference Rules; `true` switches the rule off, `false`
  leaves it running).
- Load-time constant refusals: `a_constant_value_that_is_no_boolean_is_refused_at_load`
  (IRI, non-boolean literal, `[]` refused on node and property shapes; `true`/`false` load).
- Dynamic failures: `an_expression_that_does_not_yield_one_boolean_is_a_failure` with a
  boolean-valued neighbouring focus node validating.
- Blank-node argument: required parameter unbound while IRI call to the same function
  returns; optional parameter case `total(1, ?blank, 5) = 6`.
- singleInstanceOf: two-node and zero-node bodies unbound, one-node returned.
- Prefixes: each hop of the path, plus a graph declaring one of two labels still refused.

### singleInstanceOf does not swallow processor failures

`eval_custom_function_call` (expression.rs) still propagates every `Err` from the body
(`out?`: resource limits, recursion guard, unresolvable function). Only an `Ok` output
whose length is not one becomes `Ok(None)` (SPARQL error, BIND unbound). The
node-expression path keeps `more_than_one_output_node` as an evaluation failure. The
triage states the remaining non-distinction honestly.

### Generated artifacts and regeneration

- `scripts/check-generated.sh --check`: all byte comparisons pass, including
  `generated/transcode-loss-matrix.json` and `generated/rdf-loss-matrix.json` regenerated
  by `gen_loss_matrix`, so the matrix entry is the generator's output (consistent with
  the new `SHACL_JSON_SCHEMA_PROFILE` row in rdf-core `loss.rs`). The script then exits 1
  at its last step, `check-doc-claims`: "matrix row ... totals 584 ... but W3C12_TOTAL_CASES
  pin 592". This is pre-existing and not caused by T3: the row is RED (7 approved entries
  still fail), and at the base commit it was 574+6 = 580, equally short of 591. It clears
  only when the remaining 7 pass. See finding F2.
- `crates/shapes/tests/fixtures/prepared-shapes-core.product` and the `STAGE_ID` constant:
  `product_determinism` and `product_api_compat` pass, which compares fresh bytes with the
  golden, so the golden equals what the supported regenerate test
  (`regenerate_the_prepared_product_fixture`) produces. The STAGE_ID changed, as the
  product format layout changed (the digest is the meaning per the module doc). I could
  not independently observe which command was run; the byte-equality proof is the evidence.
- `docs/CONFORMANCE.md` counts (578 pass, 397 equivalence, "Seven approved entries do not
  pass") match the measured run (578 passed, 6 non-canonical, 7 failures; 397 compared).

### Ledgers

- `shacl_shacl_differential`: `sparql-query-text` 2 -> 1 (prefixes-003 now loads);
  new `node-expression-deactivated` behind-the-spec row, count 1, pinned exactly, backed by
  a new `lint.rs` `Supersession` that cites the Core sentence verbatim. Justified: it
  covers only the one case whose `sh:deactivated` is a node expression, and
  shacl-shacl.ttl really requires `xsd:boolean`.
- `vocabulary_import_invariance`: three REFUSED_AT_LOAD entries removed (the three now
  load); `COMPARED_ON_REPORT` 394 -> 397. No growth.
- No ledger entry was added to hide a failure.

### Breaking API change

`Shape.deactivated_when`, `PropertyShape.deactivated_when` (`Vec<NodeExpr>`) and
`ConstraintAnnotation.deactivated_when` are added; `ConstraintAnnotation` loses
`PartialEq`/`Eq` (replaced by `same_as` at the one comparison site). `cargo check
--workspace --all-targets` is clean, so every constructor in the workspace (unit tests,
integration tests, schema_import, complete_reports, product codec reader/writer) is
updated; grep finds no user of these types outside `crates/shapes`, including
`bindings/`. CHANGELOG.md is generated by git-cliff from conventional-commit messages
(CONTRIBUTING.md, "Changelog"), so no manual entry is required; the commit message must
carry the breaking marker (see F3). No issue numbers or process text appear in the added
lines (grepped for `#570`, "Task 3", "Stage 1", "T3").

### Commands run

- `cargo test -p purrdf-shapes --no-fail-fast`: one failing target,
  `w3c12_conformance`, with exactly: `core/property/closed-001`, four
  `inference-rules/rules-entailment-ruleset-*`, `sparql-rl/stratification/manifest#test_13`
  and `#test_14` (also reported by `every_planned_inventory_id_passes`, which fails on
  test_13/14 only). Everything else green, including `w3c_conformance`, `custom_node_expressions`
  (29), `shape_deactivation_expression`, `sparql_prefixes`, `values_default_compliance`,
  `product_*`, `shacl_shacl_differential`, `vocabulary_import_invariance`, unit tests (970).
  Scoreboard: 578 passed, 6 non-canonical, 0 xfailed, ledger 0.
- `cargo test -p purrdf-core loss`: 52 passed.
- `cargo check --workspace --all-targets`: clean.
- `cargo clippy -p purrdf-shapes -p purrdf-core --all-targets -- -D warnings`: clean.
- `scripts/check-generated.sh --check`: see above.

## Findings

- F1 (low, non-blocking): `eval_custom_function_call` clones `bound` (`args: bound.clone()`)
  only to pass `&bound` to `eval_custom_call` again; one clone could be avoided.
- F2 (informational): `check-generated.sh` ends non-zero via `check-doc-claims` on the
  RED SHACL 1.2 row. Pre-existing; it will not be green until the 7 remaining approved
  entries pass. Not attributable to T3.
- F3 (informational, for the committer): this is an API-breaking change in a public crate
  (`purrdf-shapes`); the commit subject should use the conventional-commit breaking form
  (`feat(shapes)!:` or a `BREAKING CHANGE:` footer) so the generated changelog and the
  semver rule in CONTRIBUTING.md ("a breaking API change requires a major bump") see it.
- F4 (low, documentation): the triage refers to "the open questions below" but the
  section is titled "Spec text that made the case harder (candidate editorial
  clarifications)"; the SHOULD-fail question is item 2 there. Wording only.
- F5 (note, no action): schema_surface.rs reads only the constant `deactivated` for its
  class coverage; a node-expression-deactivated shape counts as active there, the same
  conservative projection that JSON Schema records as a loss. The loss is recorded for
  the JSON Schema output (the only emitter that has a loss channel for this); no silent
  wrong answer is produced by the validator itself.

No blocking defects found.

VERDICT: PASS

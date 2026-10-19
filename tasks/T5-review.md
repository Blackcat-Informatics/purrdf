# T5 review: rule sets as `sh:entailment` values

Independent review of the uncommitted change on top of dcf35628a. No source was modified.

## Scope

`git diff` over 15 files: `rule_parse.rs`, `engine.rs`, `rules.rs`, `product/ast.rs` (+ tests),
`product/mod.rs` (STAGE_ID), the regenerated fixture, `census.rs` comment, test ledgers and
counts, `docs/CONFORMANCE.md`, `docs/book/src/validation/shacl.md`, `tests/rules_engine.rs`.

## Evidence

1. Normative quotes (w3c/data-shapes `4cd46986`, text extracted from
   `shacl12-inference-rules/index.html` and `shacl12-core/index.html`). Every quoted sentence in
   T5-triage.md is present verbatim: "Rule Entailment Regimes" (two kinds of values; evaluation
   graph; MUST signal a failure), "Entailment using Rule Sets" (any IRI that is a SHACL instance
   of sh:RuleSet; only the rules of that set, including included sets; more than one value, or
   also sh:RulesEntailment, then executed together as a single rule set), "Rule Sets" (default rule
   set = all rules; transitive sh:includesRuleSet), Core "Entailment" (values are IRIs; MUST signal
   a failure; entailments for all of the values). They support the behaviour.
   - `-and-all`: RulesEntailment beside a set runs all rules. Matches "or also has the value
     sh:RulesEntailment ... together as a single rule set" and "all rules of the shapes graph".
     `engine.rs` selects `RuleOptions::default()` when `entailment` is true.
   - `-multiple`: `rule_set_members` takes all IRIs, closes over includes, returns one member list;
     `shacl_rule_set` builds one rule set from it, so it is stratified together. Test
     `several_rule_set_entailments_execute_together_as_one_rule_set` shows Greeting reading
     FullName's output and each set alone failing.
   - Refusal neighbours exist in `rules_engine.rs`: undeclared IRI, blank node, RDFS entailment IRI
     (`rules_entailment_runs_the_rules_before_validation`), and a supported set beside an
     unsupported value (still a failure). The declared-set neighbour loads.
2. Product format gating. `FORMAT_VERSION` (1) only gates the container/section directory
   (`SPEC`). The AST section is decoded at exactly one site (`ProductView::admit_bound`,
   `decode_ast`), and only after `verify_stage_id()`. `STAGE_ID` is content-derived by the census
   (`product_model_census::stage_id_matches_shipped_constant`, passing) from the model sources, and
   was changed. An old product therefore opens, fails `admit` with `ProductDimension::StageId`, and
   is restored only through `rebuild`, which re-derives from the carried dataset and does not read
   the AST memo. The precedent is documented in `product_determinism.rs` (the class analysis moved
   inside the AST section "so the section directory and the format version stay where they are")
   and 215b8bf45 did the same (fixture 4728->4736, no FORMAT_VERSION bump).
   `the_frozen_format_epoch_product_is_refused_then_rebuilt` and `..._stays_readable` pass.
   No mis-decode path: not blocking.
3. Fixture regeneration. The supported command is the `#[ignore]`d test
   `regenerate_the_prepared_product_fixture` (`cargo test -p purrdf-shapes --test product_determinism
   -- --ignored --exact --nocapture regenerate_the_prepared_product_fixture`). I cannot observe how
   it was produced, but `frozen_product_bytes` passes: the committed 4,744-byte file is byte-equal to
   this build's output, and `GOLDEN_LEN` was raised 4,736 -> 4,744 (+8: a u64 count of zero for the
   empty set list), matching the previous +8 step. The frozen v1 epoch fixture is untouched.
4. Ledgers. Four `W3C12_REFUSED_AT_LOAD` / `REFUSED_AT_LOAD` entries and the
   `unsupported-entailment-regime` stricter entry were removed, which are the fixed entries only.
   Counts: agreed/compared 397->401 (the four cases now load). Mutants 783->799: the four cases are
   now bases both sides accept, each yielding 4 mutants (literal-for-IRI, non-integer count,
   list-for-single, misspelled predicate; non-boolean flag finds nothing), +16, and the per-kind
   counts +4 each (203->207, 112->116, 172->176, 255->259) are consistent. Docs and the CONFORMANCE
   matrix (579->583, 397->401, "Two approved entries do not pass", 5->1 refused at load) agree.
5. Test runs (all in this worktree).
   - `cargo test -p purrdf-shapes --no-fail-fast`: every binary ok (86 ok results) except
     `w3c12_conformance`: 17 passed, 2 failed, both from the same two failures
     `sparql-rl/stratification/manifest#test_13` and `#test_14` ("expected a stratification failure").
     `W3C12 TOTAL: passed 583, non-canonical-expected-decimal 6, xfailed 0`. Expected.
   - `w3c_conformance` 2 ok; `rules_conformance` 3 ok; `rules_engine` 32 ok; `product_determinism`
     7 ok (1 ignored regenerator); `product_corpus_equivalence`, `shacl_shacl_differential`,
     `vocabulary_import_invariance`, `product_model_census` ok.
   - `cargo check --workspace --all-targets`: clean, 0 warnings/errors.
   - `cargo clippy -p purrdf-shapes --all-targets`: clean.
6. Breaking API. Grep of `bindings/`, `crates/cli`, `crates/validate`, capi/wasm for
   `RuleOptions::rule_set` / `.rule_set()` / `RuleGraph {` finds no user outside `purrdf-shapes`
   (the `rule_set` hits are unrelated SRL/entail names); `with_rule_set` is kept, so only the
   getter changed. The workspace check confirms it compiles.

## Findings

No blocking findings. Non-blocking notes:

- N1. `RuleOptions::rule_set()` was removed and `RuleGraph` gained a public field; both are
  breaking to downstream Rust users. Check the commit/CHANGELOG records them (`!` type).
- N2. The assertion `err.contains("ex:Undefined") || err.contains("Undefined")` in
  `an_undefined_entailment_regime_still_signals_a_failure` is redundant (the second disjunct
  subsumes the first).
- N3. Error text on load still starts with "the shapes graph declares ..."; the runtime failure
  message changed from "sh:RulesEntailment regime" to "sh:entailment regime"; no test pins the old text.
- N4. The `RuleOptions::with_rule_sets(empty)` runs no rule; the engine never passes an empty list
  (guarded by `is_empty()`), so this is only an API edge.

VERDICT: PASS

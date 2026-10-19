# T19 review (G-09, G-10; A-R9, A-R10)

Scope: commit 02148b328 on 08495db09, worktree /home/paudley/Active/purrdf/.worktrees/570-t19. Files: crates/shapes/src/engine.rs, crates/shapes/src/shapes/parser/rule_parse.rs, crates/shapes/tests/rules_engine.rs. Source not modified.

## Evidence (all run in the side worktree)
- `cargo test --locked -p purrdf-shapes --test rules_engine --test rules_conformance`: 36 + 3 passed.
- `cargo test --locked -p purrdf-shapes --lib engine`: 67 passed, incl. `a_declared_entailment_answers_everything_with_its_reason`.
- `--test change_footprint --test dated_execution --test change_path_alloc`: 18 + 12 + 16 passed.
- `cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings`: clean.

## Findings by question
1. Typed sh:RuleSet decision: sound. ED text (4cd46986, index.html): "Any rule set can be used as a value of sh:entailment, i.e. any IRI that is a SHACL instance of sh:RuleSet in the shapes graph" (Entailment using Rule Sets) and "All SHACL instances of sh:RuleSet have an IRI" (Rule Sets). shacl.ttl (spec/shacl.ttl:2018-2033) gives sh:hasRule and sh:includesRuleSet rdfs:domain sh:RuleSet, and includesRuleSet rdfs:range sh:RuleSet; SHACL does no RDFS inference, so domain does not type a subject. All in-tree vectors and tests that use rule sets type them (checked inference-rules vectors, admission.rs:1067).
2. One definition: partially. Both readers apply the same test (`ShaclInstances::is_instance(node, sh:RuleSet)`), but it is written twice (parse_rule_sets, parse_entailment, rule_parse.rs ~165 and ~254), not one named function/predicate. Behaviourally identical and pinned by test `an_untyped_subject_of_has_rule_...`; a shared helper would be the "one home" ideal. NON-BLOCKING.
3. Undeclared includes: refused at load in parse_rule_sets (always run by parse_rule_graph), error names both IRIs. Test covers five regimes (none, unrelated set, RulesEntailment alone, RulesEntailment+set, the broken set itself), neighbours (declared include loads, including beside RulesEntailment; untyped hasRule-only IRI refused as include). Old code: no load-time check, and RulesEntailment never walked includes, so these would fail on old code (reasoned; the first-listed cases expect_err on a graph that previously loaded).
4. Cycles: rule_set_members keeps a seen-list; test enters from A and from B and gets both rules (Ok(true)), acyclic neighbour unmet (Ok(false)). This is a regression pin (old code also terminated), not a before/after test; acceptable.
5. Behaviour change (with_rule_set on an untyped IRI now refused as "not declared"; an untyped subject as sh:entailment was already refused; an undeclared include now fails at load rather than at run): correct and test-pinned, BUT not documented. CHANGELOG.md [Unreleased] and docs/CONFORMANCE.md (lines ~679, 696) have no entry for it; the commit touches neither. FINDING F1 (MINOR): add an [Unreleased] CHANGELOG line (and a CONFORMANCE.md note) for: load-time include check, typed-only rule sets, with_rule_set on an untyped IRI refused, validate_change Everything under entailment. A public-API behaviour change warrants it.
6. G-10: affected_focus_raw checks `rules.entailment || !entailment_rule_sets.is_empty()` first and answers Everything{"rules entailment"}; validate_change falls back to full validation. Test runs both declarations (RulesEntailment, rule set), asserts expansion, ChangeScope, and report N-Triples equal to full validate(), plus neighbour with no declaration stays Bounded (1 id). Old code returned the false "not bound" error, so the test would panic on old code (the unwrap_or_else).
7. Is Everything right? Sound, yes: rule derivations can create/retract triples anywhere, and entailed data carries no delta source. Not over-broad in the general case. A bounded answer for sh:RulesEntailment with an empty rule set (or no rules in the shapes graph) is possible, since no triple is derived and the unentailed snapshot would apply; the commit does not take it. Acceptable either way; the cost is only lost incrementality for a degenerate declaration. Side effect to note: the guard precedes the delta-source check, so an entailing validator not bound to a snapshot now returns Everything instead of the binding-refusal error; still sound (full validation), but the unbound-binding diagnosis is masked for these graphs. NON-BLOCKING.

## Verdict
No correctness defect. One documentation finding (F1) and one design nit (two copies of the is-rule-set test).

VERDICT: PASS (with F1 CHANGELOG/CONFORMANCE note recommended before merge)

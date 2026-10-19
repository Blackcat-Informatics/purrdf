# T6 review: run-once classification of a variable triple-term head (#570, Stage 1 Task 6, G1)

Reviewer: independent. Scope: the uncommitted `git diff` over 519c52f55 in
`.worktrees/570-submit-w3c-shacl-implementation-reports`: `crates/shapes/src/srl/ir.rs`,
`crates/shapes/tests/srl_language.rs`, `crates/shapes/tests/w3c12_conformance.rs`,
`crates/cli/tests/w3c_srl_check_cli.rs`, `docs/CONFORMANCE.md`. No source modified.
Spec read: `/home/paudley/Active/w3c/data-shapes/sparql12-rl/index.html` and
`sparql-rl-grammar.bnf`.

## 1. Specification reading (quoted)

Run-once rule, "Elements of the Abstract Syntax" (index.html ~962-975): "A rule is a
run-once rule if any of the following hold: the rule body has an assignment element; the
rule body has an aggregation element; a triple template of the rule head has a blank node;
a triple template of the rule head has a template for a triple term in which a variable
occurs."

Overview (~563-571): "Rules involving assignments, rules that create blank nodes from
evaluation of the rule head and rules with a template that involves a triple term
containing variables are run-once rules. Such rules are run after all the rules that could
produce data that they depend on ... This condition ensures that such rules do not loop
back to themselves and cause the creation of an unbounded number of RDF terms." Note: a
constant RDF term in a head does not make a rule run-once.

Closed dependency (~1417-1440): closed if (a) a pattern in a negation element matches a
head template of R2, or (b) "Rule R1 depends on rule R2 and R1 is a run-once rule." (The
aggregation-pattern clause is commented out as "Subsumed by run-once.")

Stratification condition (~1700-1710): "there is no recursive dependency involving a closed
dependency in the dependency graph for a rule set."

Confirmed: a head triple-term template containing a variable makes the rule run-once; all
its dependencies are then closed; a cycle through it fails the stratification condition.
`#test_13` (self-cycle) and `#test_14` (two-rule cycle through the run-once rule) are
therefore correctly refused, and the expected result is derived from the text, not just
the manifest.

Nested triple terms: `PatternTerm::has_variable_triple_term` tests
`inner.variable_names()`, and `PatternTerm::variables` recurses through `Triple`, so a
variable at any depth of nesting counts. Verified with the CLI (binary
`target/debug/purrdf rules --srl F --srl-base http://example/ --check`):

| Rule set | Result |
|---|---|
| `RULE { ?x :p <<( :a :q <<( ?y :r :b )>> )>> } WHERE { ?x :p ?y }` | refused, stratification (self closed cycle) |
| same head, `WHERE { ?x :s ?y }` (no cycle) | accepted, 1 stratum |
| `RULE { ?x :p <<( :a :q <<( :c :r :b )>> )>> } WHERE { ?x :p 1 }` (nested constant) | accepted (note: a constant nested triple term is not run-once) |

Reifier/annotation forms lower to `r rdf:reifies <<( s p o )>>`, which is exactly "a
template for a triple term in which a variable occurs" when s/p/o hold variables. They are
covered by the same clause, which is the specification's abstract-syntax reading
(syntax-template-14 is such a case).

The diff comment on `is_run_once` quotes the definition with an elision and is accurate.
The new unit tests in `ir.rs` and `a_variable_triple_term_head_is_run_once` pin refusal
(self and cycle) and the neighbours (constant triple term; body cannot match the head;
acyclic chain evaluates with 2 inferred triples). The rewritten limits test now pins the
stratification refusal with `rule == depends_on` and keeps the neighbour that evaluates;
it replaced a test pinning the old runtime-limit behaviour, which is correct since the
behaviour is now unreachable at that rule.

## 2. Open point (a): aggregation

Does the language have aggregation? The abstract syntax does: the spec defines an
"aggregation element" (rule element; keys, pattern, assignments), its well-formedness
(~1284-1307), in-scope variable rule (~1222), dependency treatment, the stratification
algorithm lines, and an evaluation procedure (~2168, ~2309). The section "Grouping and
Aggregation" (~587-608) is flagged with an open WG issue box (data-number 840: "Provide
GROUP BY and aggregation functions ... at least COUNT, COUNT DISTINCT, MIN, MAX, SUM")
and its text is unfinished ("then evaluates" ends mid-sentence).

But the concrete syntax does not: `sparql-rl-grammar.bnf` BodyNotTriples is exactly
`Filter | Negation | Assignment` (rules [15], [21], [24]); there is no GROUP BY, COUNT,
SUM or aggregation production, and `SET(...)` takes an Expression whose built-in calls
exclude aggregates. grep of the vendored suite `vectors/shacl12/tests/sparql-rl` and the
upstream `shacl12-test-suite/tests/sparql-rl` finds no aggregation test (no `.srl` with
GROUP BY or COUNT; the only manifest hits are "Working Group" in headers).

PurRDF behaviour, verified with the CLI at bare `--check` (both levels give the same
refusal, since it is a grammar failure):

| Input | Result |
|---|---|
| `WHERE { ?x :p ?y GROUP BY ?x COUNT(?y) AS ?c }` | `SPARQL 1.2 RL syntax error at line 2 column 36: expected '}', found 'GROUP'` |
| subquery `{ SELECT ?x (COUNT(?y) AS ?c) ... GROUP BY ?x }` | syntax error, `expected an RDF term, found '{'` |
| `SET(?c := COUNT(?y))` | `` `COUNT` is not a SPARQL 1.2 RL built-in call `` |
| neighbour `RULE { ?x :n ?y } WHERE { ?x :p ?y }` | accepted, stratified |

So aggregation is not silently unsupported: no rule set can express it in the published
grammar, no conformance test exercises it, and every attempt is refused with a precise
syntax error. There is no silent acceptance and no wrong answer. This is not a BLOCKING
gap: the missing clause cannot be reached. (Were the WG to add the concrete syntax, the
engine needs an aggregation IR element; that is future language, not a present defect.)

Non-blocking correction recommended (documentation accuracy, no behaviour): the module doc
`# Aggregation` in `crates/shapes/src/srl/ir.rs` (lines ~31-36) says "Neither language
defines an aggregation element". That is false for the SPARQL 1.2 RL abstract syntax, which
does define one. Suggested wording: "SPARQL 1.2 RL's abstract syntax defines an aggregation
element, but the published grammar has no concrete syntax for it (WG issue 840) and the
suite has no test of it, so the parser refuses every spelling; the IR therefore carries no
aggregate literal." The T6-triage.md sentence "The implemented language has no aggregation
element" should say the same. Worth a WG note: abstract syntax, well-formedness,
dependency and evaluation text exist with no grammar and no test.

## 3. Open point (b): syntax-template-14 and -16

Rule sets: `RULE { << ?a ?b ?c ~:r >> } WHERE { ?a ?b ?c }` and
`RULE { <<( ?a ?b :o )>> :q <<( :s :p ?c )>> } WHERE { ?a ?b ?c }`. Each head templates a
triple term with variables (making the rule run-once), and the body `?a ?b ?c` matches
every triple including the head's own output: a self-dependency, closed. Under the quoted
text these are unstratifiable. That is correct per the specification, and consistent with
the suite's own rule that syntax tests are graded "regardless of well-formedness and
stratification".

Grading: the library harness (`w3c12_conformance.rs`) checks a positive syntax entry at
`CheckLevel::Syntax`; the CLI test (`w3c_srl_check_cli`) asserts both that bare `--check`
refuses exactly the SYNTAX_ONLY set (now 17: 16 unstratifiable, 1 ill-formed) by a later
stage never the grammar, and that `--check=syntax` accepts them. Both pass, so no
approved result changes. 585 passed.

WG note: worth including in the triage (it already lists it under "Candidate editorial
clarifications"). Suggested sharper wording: the two positive syntax tests are not
stratifiable by the normative text, so a processor following the full pipeline rejects
them; the manifest should say syntax tests are for the grammar only.

## 4. Open point: `check-doc-claims.py` failure (589 vs 592)

Output: `the matrix row 'SHACL 1.2 (...)' totals 589 and the harness pins 1 unlisted
entries, but W3C12_TOTAL_CASES / W3C12_CASES_BY_TYPE ... pin 592`.

The 589 comes from the GENERATED block in `docs/CONFORMANCE.md` line 64 (`| 583 | 6 | 6 | 0
| RED |`), which this diff does not touch. The identical failure therefore exists at
519c52f55 before the change: it is not introduced here. Arithmetic with live counts:
585 passed + 6 non-canonical decimals = 591 approved, + 1 unlisted = 592 = the pin. The
harness now reports `W3C12 TOTAL: passed 585, non-canonical-expected-decimal 6, xfailed 0,
ledger 0`. So the diagnosis (interim state) is right in substance: the failure is the stale
generated block, which is regenerated only with `python3 scripts/conformance-matrix.py
--write-doc` (full matrix run; plan Task 1 and G9a say to regenerate with the generator,
not by hand). The hand-edited prose in this diff (585, test_13/test_14 sentence removed,
the new run-once sentence) is consistent with the live harness and with the post-regeneration
arithmetic; no prose elsewhere states 583 or 589 (grep). It is not a docs inconsistency
introduced by Task 6. Gate action: regenerate the matrix block at the point the plan
assigns (the relabel tasks or the final doc pass) and require `check-doc-claims.py` to be
green then; it must not be recorded as passing before that.

Note the decimal prose line in `docs/CONFORMANCE.md` still describes 6 cases graded by
canonical spelling; that remains true until Tasks 7/8.

## 5. Runs (all in the worktree, after the change)

- `cargo test -p purrdf-shapes --test w3c12_conformance`: 19 passed, 0 failed; scoreboard
  585 passed, 6 non-canonical decimals, 0 xfailed, ledger 0; `sparql-rl/stratification`
  passed 14 of 14.
- `cargo test -p purrdf-cli --test w3c_srl_check_cli`: 1 passed.
- `cargo test -p purrdf-shapes --test srl_language`: 26 passed.
- `cargo test -p purrdf-shapes --test w3c_conformance`: 2 passed (129/129 SHACL 1.0 intact).
- `cargo test -p purrdf-shapes --no-fail-fast`: 2165 tests passed, 0 failed across all
  result lines.
- `cargo clippy -p purrdf-shapes -p purrdf-cli --all-targets`: clean, no warnings.
- `python3 scripts/check-doc-claims.py`: fails as described in section 4 (pre-existing,
  stale generated block).
- CLI probes in sections 1 and 2.

## 6. Findings

1. Fix is correct and minimal: single classification point shared by dependency analysis
   and stratifier; no broader refusal than the text (neighbouring valid cases accepted:
   constant, nested constant, non-reading body, acyclic chain; evaluation of the chain
   verified by test).
2. Aggregation run-once clause is unreachable, not silently unsupported (grammar has no
   syntax; no test; clean syntax errors). Not blocking. Minor: the `ir.rs` module doc and
   the triage sentence misstate that the language has no aggregation element; correct
   them to say the abstract syntax defines it but the grammar does not.
3. syntax-template-14/-16 unstratifiable is spec-correct, graded at syntax level only;
   WG-note-worthy (already in triage).
4. `check-doc-claims.py` red is the pre-existing stale generated matrix block (583 vs
   live 585), not introduced by this diff; regenerate with the generator, do not
   hand-edit.

VERDICT: PASS

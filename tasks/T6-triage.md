# T6 triage: stratification with a triple-term template (`sparql-rl/stratification/manifest#test_13`, `#test_14`)

Two approved `srlt:RulesNegativeStratificationTest` cases of the SHACL 1.2 test suite
(`w3c/data-shapes`, `shacl12-test-suite/tests/sparql-rl/stratification/`, run as part of
the vendored suite). The section is written so it can be lifted into a Working Group
discussion item with minimal editing. It is an engine defect, fixed. It does not meet
G1, so it does not enter `SPEC_CONTRADICTED`.

Short name: RL = SPARQL 1.2 Rule Language (`sparql12-rl/index.html` in the checkout).

## The cases

| Case | Rule set | Expected |
|---|---|---|
| `#test_13` (`stratification-bad-06.srl`) | `RULE { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :p ?y }` | stratification failure |
| `#test_14` (`stratification-bad-07.srl`) | `RULE { ?x :p <<( ?x :p ?y )>> } WHERE { ?x :q ?y }` and `RULE { ?a :q ?b } WHERE { ?a :p ?b }` | stratification failure |

Both carry the comment "Template triple term in cycle." Each rule set is syntactically
valid and well formed. In `#test_13` the one rule depends on itself: its body pattern
`?x :p ?y` matches the triple its head builds. In `#test_14` the first rule's output
`?x :p <<...>>` is read by the second rule's body `?a :p ?b`, whose output `?a :q ?b` is
read by the first rule's body `?x :q ?y`.

## Prior output

Both rule sets were accepted at every stage ("expected a stratification failure, but
every stage accepted it") and, when run, `#test_13` was stopped only at runtime by the
term-generating-round limit, as an evaluation failure rather than a stratification one.
The dependency graph was correct: it had the right edges, found by unifying the body
pattern with the head template through the triple term. The edges were all open, so no
closed edge sat on a cycle. The missed fact was the rule's classification, not an edge.

## Normative text

RL, "Elements of the Abstract Syntax", definition of run-once rule: "A rule is a run-once
rule if any of the following hold: the rule body has an assignment element; the rule body
has an aggregation element; a triple template of the rule head has a blank node; a triple
template of the rule head has a template for a triple term in which a variable occurs."

RL, the overview of assignment and creating RDF terms: "Rules involving assignments, rules
that create blank nodes from evaluation of the rule head and rules with a template that
involves a triple term containing variables are run-once rules. [...] This condition
ensures that such rules do not loop back to themselves and cause the creation of an
unbounded number of RDF terms." The note that follows: "A constant RDF term in a rule head
does not make a rule a run-once rule".

RL, "Rule Dependency": "Closed dependency: [...] Rule R1 depends on rule R2 and R1 is a
run-once rule." A triple pattern depends on a template "if the triple template can
generate a triple that matches the triple pattern", where "Replacing variables by RDF terms
in a triple pattern includes replacing variables inside triple terms."

RL, "Stratification Condition": "there is no recursive dependency involving a closed
dependency in the dependency graph for a rule set." The note adds that this is "exactly
the condition for a stratification to exist".

## What a conforming processor must produce

1. The head `?x :p <<( ?x :p ?y )>>` has a triple-term template containing variables, so
   the rule is a run-once rule in both cases (test 13: the only rule; test 14: the first).
2. A run-once rule's every dependency is closed. Test 13's self-dependency is closed and
   recursive; test 14's edge from the first rule to the second is closed, and the second
   rule's edge back is a recursive path through it.
3. The stratification condition fails, so no stratification exists and the evaluation of
   the rule set has no defined outcome. A conforming processor can therefore signal a
   stratification failure, and the suite's expected result is the one the text derives.
   Nothing in the text lets a processor that classifies these rules correctly accept them.

## Conclusion

Engine defect. The processor's run-once test covered assignments and head blank nodes but
not the fourth clause (head triple-term template with a variable). It was fixed in the
existing classification (`IrRule::is_run_once`), the one place the dependency analysis and
the stratifier read. G1 eligibility (1) fails: a processor that follows the quoted
sentences produces the expected failure, so the case is fixed, not classed.

The neighbouring cases (`stratification-06`: a constant triple term, no variable; and
`stratification-07`: variable inside the triple term whose head object cannot be matched
by the body pattern) are accepted, and are the proof the refusal is no broader than the
text. Both stay green. Rule sets read by an acyclic chain through a run-once rule evaluate.
The abstract syntax describes an aggregation element, but the published grammar gives
it no syntax (`BodyNotTriples` is `Filter | Negation | Assignment`), the section is an
open Working Group issue, and no SPARQL 1.2 RL test uses it. PurRDF refuses every
aggregate spelling with a syntax error, so that run-once clause is not exercised. Candidate
WG discussion point: the aggregation element's grammar and its run-once clause.

Side effect, same cause: two positive syntax cases (`syntax-template-14`, `-16`) are now
unstratifiable at the full check, as their head templates a variable triple term over a
body pattern `?a ?b ?c` that reads every triple. They remain accepted at `--check=syntax`,
the level their type asks. The count of such cases moves from 15 to 17 (sixteen
unstratifiable, one ill formed).

## Candidate editorial clarifications

- The overview of assignment and creating RDF terms says "a triple term containing
  variables", the definition says "a template for a triple term in which a variable
  occurs". The two agree, but a single worked example of a head such as
  `?x :p <<( ?x :p ?y )>>` becoming a closed self-dependency would settle the reading for
  implementers who classify run-once by "creates a term not in the data".
- The note on constant terms says a constant triple term in a head is not run-once. A
  sentence that the same holds for a nested constant triple term would avoid doubt about
  `<<( :x :p 1 )>>`.
- Sixteen positive syntax tests are unstratifiable and the suite says "regardless of
  well-formedness and stratification" for syntax tests. This is worth stating in the
  syntax manifest's description, since two more become unstratifiable once the clause is
  applied (a variable-predicate body such as `?a ?b ?c` reads any head).

## Corners for proposed-test candidates

- A head that nests a variable triple term inside another triple term, over a body that
  reads the head's predicate: the same clause, one nesting level deeper.
- A run-once rule (variable triple-term head) in an acyclic chain read by a later rule:
  must be accepted and the later rule must see the triple term the run-once rule built.

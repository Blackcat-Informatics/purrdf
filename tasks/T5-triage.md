# T5 triage: rule sets as `sh:entailment` values (`inference-rules/rules-entailment-ruleset-*`)

Four approved `sht:Validate` cases of the SHACL 1.2 test suite (`w3c/data-shapes`,
`shacl12-test-suite/tests/inference-rules/`, checkout `4cd469867a`), first run under the
dated profile `shacl-wd-2026-09-18`. Task 13's audit (`T13-triage.md`) found that the
rule-set regime they test is not in the WD text: "Entailment using Rule Sets" was added
to the editor's drafts by `2ada6ac4` (2026-10-12), and the WD (`32318223`) defines
`sh:RulesEntailment` as the only entailment regime. The behaviour is therefore read under
`shacl-ed-2026-10-16` (the editor's drafts at `4cd46986`); `shacl-wd-2026-09-18` refuses a
rule set as an `sh:entailment` value, as the WD's "MUST signal a failure" for an
unsupported regime requires. The section is written so it can be lifted into
a Working Group discussion item with minimal editing. It is an engine gap, fixed. It
does not meet G1, so it does not enter `SPEC_CONTRADICTED`.

Short names: Core = SHACL 1.2 Core; IR = SHACL 1.2 Inference Rules.

## Prior output

Every case was refused when the shapes graph was loaded:

> the shapes graph declares <...> sh:entailment <http://example.com/ns#...>; this
> processor supports the entailment regime sh:RulesEntailment only, and SHACL requires
> a processor to signal a failure for a regime it does not support

The engine treated `sh:RulesEntailment` as the only supported regime and a rule set
as an unsupported one.

## The cases

All four expect `sh:conforms true`. In each, `ex:Alice` is an `ex:Person` with
`ex:firstName "Alice"` and `ex:lastName "Smith"`, and `ex:PersonShape` links
`ex:FullNameRule` (derives `ex:fullName`) and `ex:NicknameRule` (derives
`ex:nickname`) through `sh:rule`.

| Case | `sh:entailment` values | Rule sets | Requirements of the shape | Reading |
|---|---|---|---|---|
| `-validation` | `ex:FullNameRules` | `FullNameRules` has `FullNameRule` | `fullName` minCount 1, `nickname` maxCount 0 | only the set's rule runs; `NicknameRule` does not |
| `-includes` | `ex:AllRules` | `AllRules` has no rule, `sh:includesRuleSet ex:FullNameRules` | same | the included set's rule runs; `NicknameRule` does not |
| `-multiple` | `ex:FullNameRules`, `ex:GreetingRules` | one rule each; `GreetingRule` reads `ex:fullName` | `fullName` and `greeting` minCount 1, `nickname` maxCount 0 | both sets run as one: `GreetingRule` sees the full name the other set derives |
| `-and-all` | `sh:RulesEntailment`, `ex:FullNameRules` | `FullNameRules` has `FullNameRule` | `fullName` minCount 1, `nickname` minCount 1 | the default rule set runs, so `NicknameRule` runs too; the set narrows nothing |

## Normative text

IR, "Rule Entailment Regimes":

- "This section defines two kinds of values of `sh:entailment` that instruct a SHACL
  validation engine to execute SHACL rules prior to starting the validation: the IRI
  `sh:RulesEntailment`, which executes all rules of the shapes graph, and rule sets,
  which execute only the rules of the given rule set."
- "After these entailments have been executed, the data graph that gets validated by
  the SHACL validation engine is the evaluation graph, i.e. the union of the base graph
  and the inference graph."
- "Following the general policy for SHACL, validation engines that do not support a
  given entailment regime MUST signal a failure if the shapes graph uses it as a value
  of `sh:entailment`."

IR, "Entailment using Rule Sets":

- "Any rule set can be used as a value of `sh:entailment`, i.e. any IRI that is a
  SHACL instance of `sh:RuleSet` in the shapes graph. Validation engines that support
  this regime execute the given rule set following the rules execution instructions
  prior to performing the actual validation. Only the rules of that rule set are
  executed, including those of the rule sets that it includes via
  `sh:includesRuleSet`. Other rules in the shapes graph are ignored."
- "If the shapes graph has more than one such value for `sh:entailment`, or also has
  the value `sh:RulesEntailment`, then the rules of all of them are executed together
  as a single rule set."

IR, "Rule Sets": "The default rule set of a graph is the set of all rules in the
graph, regardless of whether they are also members of other rule sets. Rule sets can
use the property `sh:includesRuleSet` to (transitively) include other rule sets."

Core, "Entailment": "The values of the property `sh:entailment` are IRIs." and "If a
shapes graph contains any triple with the predicate `sh:entailment` and object `E` and
the SHACL processor does not support `E` as an entailment regime for the given data
graph, then the processor MUST signal a failure."

## Conclusion

The cases are consistent with the text and the engine was wrong to refuse them.

- A value of `sh:entailment` that is an IRI and a SHACL instance of `sh:RuleSet` in
  the shapes graph is a supported regime. The union of the members of all such sets
  (transitively through `sh:includesRuleSet`) is one rule set, executed before
  validation, and validation reads the evaluation graph.
- `sh:RulesEntailment` executes the default rule set, which already holds every rule,
  so a rule set named beside it changes nothing (`-and-all`).
- Rules outside every named set are ignored (`-validation`, `-includes`, `-multiple`).
- Every other value stays a failure: an IRI that is not an instance of `sh:RuleSet`
  in the shapes graph (an entailment regime IRI such as RDFS entailment, or an
  undeclared name) and a blank node (Core: values are IRIs). One unsupported value
  fails the shapes graph even beside supported ones, because Core requires the
  entailments "for all of the values". Control cases pair each refusal with the
  declared rule set that loads.

Implementation: one rules engine. The shapes graph records the rule sets declared as
`sh:entailment` values (`RuleGraph::entailment_rule_sets`); validation executes the
default rule set for `sh:RulesEntailment`, otherwise the named sets together through
`RuleOptions::with_rule_sets`. The prepared product carries the new field.

## Candidate editorial clarifications

1. "Entailment using Rule Sets" says "any IRI that is a SHACL instance of
   `sh:RuleSet` in the shapes graph". Say whether an IRI that is only the subject of
   `sh:hasRule` or `sh:includesRuleSet` (not typed `sh:RuleSet`) is a rule set for this
   purpose; the engine requires the type.
2. State whether an IRI that is both an entailment regime known to the processor (for
   example from the SPARQL entailment regimes) and a `sh:RuleSet` is read as the
   regime or the rule set.
3. State that a rule named by `sh:includesRuleSet` but not declared as a rule set is a
   failure, not an empty set.
4. "The rules of all of them are executed together as a single rule set" interacts
   with `sh:order` and `sh:layer`; say that the layering is computed over the union,
   as the `-multiple` case requires of a rule that reads another set's output.

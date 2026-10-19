# T13 triage: which law each Task 2-7 behaviour belongs to

Three texts are compared, each from the `w3c/data-shapes` checkout:

- **WD** is the text of the dated working drafts, read at `32318223` (2026-09-19
  07:33 +1000, so 2026-09-18 UTC): the last commit that touches `shacl12-core`,
  `shacl12-sparql` and `shacl12-inference-rules` on or before the SPARQL WD date of
  2026-09-18. `shacl12-node-expr` last changed at `4199e812`, before it. The WD-era
  `sparql12-rl` is read at the same commit.
- **ED** is the editor's drafts at `4cd469867a1cc6d2fff80e8a40723b26c695d67a`
  (`git show -s --format=%cs` prints `2026-10-16`), the suite commit. Law id
  `shacl-ed-2026-10-16`.
- **REC** is SHACL 2017-07-20, `shacl/index.html` at `4cd46986` (last changed by
  `61866d39`, 2017-05-18).

The commands used: `git show 32318223:<doc>/index.html` and
`git show 4cd46986:<doc>/index.html` for `shacl12-sparql`, `shacl12-core`,
`shacl12-inference-rules`, `shacl12-node-expr`, `sparql12-rl`; tags stripped and the
two texts diffed. The only differences in `shacl12-node-expr` are two sentences on what
a processor is (no semantics). `git log -G` on each document gives the commit that
introduced each sentence.

Result: four behaviours added on this branch are first stated after the WD, and are
now read under the ED law (and the compatibility law) only. Everything else is stated
by the WD already, so the ED inherits it through `ShaclProfile::is_shacl12()`.

## Post-WD behaviours (ED only; WD and REC refuse or ignore them)

| # | Behaviour (task) | WD text (`32318223`) | ED text (`4cd46986`) | REC text | Engine under WD | Engine under ED |
|---|---|---|---|---|---|---|
| 1 | Node-expression `sh:deactivated` on a SPARQL-based constraint, and on an `sh:expression` constraint node (T3; no approved suite case exercises it, `deactivated-select-001` writes the expression on a shape) | SPARQL, "Syntax of SPARQL-based Constraints": "SPARQL-based constraints may have at most one value for the property sh:deactivated and this value is either true or false." The Validation section: "There are no validation results if the SPARQL-based constraint has true as a value for the property sh:deactivated." `sh:expression` constraint nodes: no text for `sh:deactivated`. | Added by `07458ebb` (2026-10-10): "this value is a node expression that must have either true or false as its (only) output node, as for shapes." and "If $sparql has a value expr for the property sh:deactivated and evalExpr(expr, data graph, focus node, {}) produces true as its only output node, then the constraint is deactivated and there are no validation results." No text for `sh:expression` nodes either; the ED reading follows the SPARQL sentence. | "The value of sh:deactivated in a shape must be either true or false." | A node expression is refused at load (boolean `true`/`false` load) | Evaluated per focus node |
| 2 | Version-IRI prefix path for SPARQL prefixes (T3, `prefixes-003`) | SPARQL, "Prefix Declarations for SPARQL Queries": "the SPARQL property path sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare of the SPARQL-based constraint or validator" | `07458ebb`: "sh:prefixes/(^owl:versionIRI)?/(owl:imports/(^owl:versionIRI)?)*/sh:declare of the SPARQL-based constraint, validator or SPARQL-based node expression" | "the SPARQL property path sh:prefixes/owl:imports*/sh:declare of the SPARQL-based constraint or validator" | WD path: the root and the `owl:imports` arrivals hold declarations; a version IRI is a stepping stone | ED path: also the graph a version IRI leads to, at the root and after every import. REC: `owl:imports*`, no `owl:versionIRI` step |
| 3 | A blank-node argument to a custom SPARQL function is an error (T3, `blankNodeArgument-001`) | "Where the evaluation has caused an error, the result of the SPARQL function call is also an error, unless the corresponding sh:parameter for the argument has sh:optional true." No sentence about blank nodes (0 hits for "blank nodes cannot be passed"). | `07458ebb`: "Note that blank nodes cannot be passed as arguments: they are not constant node expressions, so an argument that evaluates to a blank node leads to an evaluation failure and thus an error." | No custom SPARQL functions | A blank node is an ordinary argument | A blank node is an error (unbound if the parameter is optional) |
| 4 | A rule set as an `sh:entailment` value (T5, `rules-entailment-ruleset-*`) | Inference Rules, "The sh:RulesEntailment Regime": `sh:entailment` links the shapes graph to entailment regimes; the one defined is the IRI `sh:RulesEntailment`; "validation engines that do not support the SHACL rules entailment regime MUST signal a failure if this triple is present." `sh:includesRuleSet` is defined for rule sets (3 mentions) but no rule set is an entailment (0 hits for "Entailment using Rule Sets"). | Added by `2ada6ac4` (2026-10-12): "Any rule set can be used as a value of sh:entailment, i.e. any IRI that is a SHACL instance of sh:RuleSet in the shapes graph. [...] Only the rules of that rule set are executed, including those of the rule sets that it includes via sh:includesRuleSet." | no rules entailment | Refused at load (an unsupported regime; `sh:RulesEntailment` still loads) | Executed before validation, with the included sets |

Rows 1-4 each have a WD/REC refusal test with a neighbour that is valid under the
same law, and the same input accepted under the ED (list in the Task 13 report).

## Behaviours the WD already states (all of WD, ED and the compatibility law)

| Behaviour (task) | WD text (`32318223`) | ED | REC |
|---|---|---|---|
| VALUES refused only when it mentions a pre-bound variable (T2, `pre-binding-009`) | SPARQL, Appendix A: "SPARQL queries MUST not contain a VALUES clause that mentions any potentially pre-bound variable" | identical | "SPARQL queries must not contain a VALUES clause" (all VALUES banned) |
| Node-expression `sh:deactivated` on node and property shapes (T3) | Core, "Deactivating Shapes and Constraints": "The value of sh:deactivated is a node expression that must have either true or false as the (only) output node." | identical | boolean only (refused) |
| Node-expression `sh:deactivated` on a constraint reifier (T3) | Core: "Let expr be the value of sh:deactivated in a reifier on a triple that has shape subject and a parameter as predicate. If evalExpr(expr, data graph, focus node, {}) produces true as its only output node, the constraints that use the triple are called deactivated constraints." | identical (the ED says "triple annotations" elsewhere, no semantic change here) | no reifiers |
| `$failure` binding, `sh:message` precedence (T3, `sparql-failure-001`, `sparql-message-001`) | SPARQL: "A failure MUST be produced if and only if one of the solutions has true as the binding for failure."; messages of the validator, then the component | the ED adds only an explanation that `failure` is reserved | no `failure` variable |
| A function body that yields other than exactly one node is the call's SPARQL error (T3, `singleInstanceOf-001`) | SPARQL, "Evaluation of Custom SPARQL Functions": "If the list of output nodes rs has exactly one member, then return that node. Otherwise, or if the evaluation produces an evaluation failure, the result of the SPARQL function call is an error." | identical | no custom functions |
| An unbound optional argument keeps the later arguments' indexes (T3) | the same section: "the argument will be unbound in the scope" | identical | no custom functions |
| `sh:closed` on a property shape judges the value nodes (T4, `closed-001`) | Core: "each value node has values only for those properties that have been explicitly enumerated" and "there is a validation result for each triple that has a value node as its subject" | identical | identical (constraint components judge value nodes) |
| `sh:RulesEntailment` executes all rules; `sh:includesRuleSet` includes sets (T5) | Inference Rules: "the IRI sh:RulesEntailment, which executes all rules of the shapes graph"; "Rule sets can use the property sh:includesRuleSet to (transitively) include other rule sets" | identical | no rules |
| The reserved SPARQL parameter names (not a Task 2-7 behaviour) | `this, path, PATH, value` | adds `message, failure` | `this, path, PATH, value` |

## Not law-specific

- **Computed-decimal spelling (T7).** Fixed by the SPARQL and XPath value
  spellings the node-expression and SPARQL functions return, not by a SHACL law. The
  `shacl12-node-expr` texts of the WD and the ED are the same apart from two
  definitional sentences, so no law selects the spelling. It applies under every law.
- **SPARQL-RL run-once rule with a variable triple-term template (T6, stratification
  `test_13`/`test_14`).** The text is `sparql12-rl`, a separate specification that no
  `ShaclProfile` selects. The WD-era `sparql12-rl` (`32318223`) lists run-once rules
  as assignments and head blank nodes; the clause "rules with a template that involves
  a triple term containing variables are run-once rules" is in the ED text
  (`sparql12-rl/index.html` at `4cd46986`). SRL has no SHACL law, so there is nothing
  to gate; the engine applies it under every law, recorded here and in the book.

## Where the engine reads each post-WD rule

- Law predicate: `ShaclProfile::reads_editors_draft_semantics()` is true for the ED and
  the compatibility law, false for the WD and the REC. `is_shacl12()` is true for the WD
  and the ED, and replaces the five `== WD_20260918` tests (`prebinding.rs`,
  `sparql.rs` twice, `report.rs`, `constraints.rs`).
- Row 1: `Parser::deactivation_of` with `DeactivationSite::Constraint` in
  `crates/shapes/src/shapes.rs`.
- Row 2: `reached_holders` in `crates/shapes/src/shapes/prefixes.rs`, which takes the
  profile.
- Row 3: `invoke_expression_function` in `crates/shapes/src/shapes/parser/functions.rs`.
- Row 4: `Parser::parse_entailment` in `crates/shapes/src/shapes/parser/rule_parse.rs`.

## Host neighbour pairs (Task 14): the plan's pair (2) is swapped

`pre-binding-009` conforms under both the WD and the ED (VALUES is a WD rule, see
"VALUES" above), so only `legacy` against the ED separates on it; pair (1) stays.

The plan's pair (2), the WD against the ED on `deactivated-select-001`, does not
separate them: that case writes `sh:deactivated [ sh:select ... ]` on two **node
shapes**, which Core's WD text defines, so the WD validates it. The WD/ED pair for
Task 14 is an approved case that rests on a post-WD sentence:

- `sparql/node/prefixes-003` (row 2): the WD path collects no `ex:` declaration (the
  graph named by the version IRI is a stepping stone), so the WD refuses the query at
  load; the ED validates it.
- or `sparql/functions/blankNodeArgument-001` (row 3): the WD passes the blank node
  (`"true ..."`), the ED makes it an error (`"false ..."`).
- or `inference-rules/rules-entailment-ruleset-validation` (row 4): refused at load
  under the WD, validated under the ED.

`prefixes-003` is the recommended host pair (one load refusal against one conforming
validation, like pair (1)).

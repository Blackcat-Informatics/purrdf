# T3 triage: SHACL-SPARQL constraint and custom-function cases

Four approved `sht:Validate` cases of the SHACL 1.2 test suite
(`w3c/data-shapes`, `shacl12-test-suite/tests/sparql/`, checkout
`4cd469867a1cc6d2fff80e8a40723b26c695d67a`) failed under the dated profile
`shacl-wd-2026-09-18` (SHACL Core WD 2026-09-17, SHACL SPARQL Extensions and
Node Expressions WD 2026-09-18). Task 13's audit (`T13-triage.md`) compared each
quoted sentence with the WD text (`32318223`) and found that three of the four
rest on text first published after the working draft, in the editor's drafts of
`07458ebb` (2026-10-10): the prefix path of `prefixes-003`, the blank-node
argument rule of `blankNodeArgument-001`, and the node expression on a
SPARQL-based constraint that the "same expression on constraints" section below
cites. Those are read under `shacl-ed-2026-10-16` (the editor's drafts at
`4cd46986`) and refused or ignored under `shacl-wd-2026-09-18`, as its text says.
`deactivated-select-001` itself writes the expression on shapes, which Core's
WD text already defines; `singleInstanceOf-001` rests on WD text. Each section is
written so it can be lifted into a Working Group discussion item with minimal
editing. All four are engine defects, fixed. None meets G1, so none enters
`SPEC_CONTRADICTED`.

G1 condition (1) asks whether any processor conforming to the quoted sentence
can produce the approved result. In each case one can, and PurRDF now does, so
(1) fails for all four and the question of (2)-(4) does not arise.
`deactivated-select-001` is also barred by (2): it was a not-evaluated refusal in
a claimed class.

Short names: Core = SHACL 1.2 Core; SPARQL = SHACL 1.2 SPARQL Extensions; NodeExpr
= SHACL 1.2 Node Expressions. A quotation below is from the editor's drafts at
`4cd46986` unless it says "WD"; the sentences first published there are marked.

---

## 1. `sparql/node/deactivated-select-001`

### The case

Two node shapes, both targeting the literal `"text"` with `sh:datatype
xsd:integer` (which `"text"` violates). `ex:ActiveShape` has

```turtle
sh:deactivated [ sh:select "SELECT (false AS ?deactivated) WHERE { }" ]
```

and `ex:DeactivatedShape` the same with `true`. It tests that a node expression
(here a SPARQL select expression) is a legal and evaluated value of
`sh:deactivated`: the shape whose expression produces `true` is ignored, the one
that produces `false` is validated.

### Approved expected result

`sh:conforms false` with one `sh:ValidationResult`: focus node `"text"`, value
`"text"`, `sh:DatatypeConstraintComponent`, source shape `ex:ActiveShape`,
severity `sh:Violation`. Nothing is reported for `ex:DeactivatedShape`.

### PurRDF before the fix

The shapes graph was refused at load:

> sh:deactivated on shape ex:ActiveShape must be an xsd:boolean literal, got
> _:gts_...; a node-expression value of sh:deactivated is not evaluated by this
> engine

No report was produced.

### Normative text

Core, "Deactivating Shapes and Constraints" (`#deactivated`):

> Shapes can have at most one value for the property `sh:deactivated`. The value
> of `sh:deactivated` is a node expression that must have either `true` or
> `false` as the (only) output node.
>
> Let `expr` be the value of `sh:deactivated` in a shape. If `evalExpr(expr, data
> graph, focus node, {})` produces `true` as its only output node, the shape is
> called deactivated. Deactivated shapes are ignored during validation.

Core, same section, non-normative: "In SHACL Core, the only valid values for
`sh:deactivated` are the constant literal node expressions `true` and `false`."
(This is a statement about SHACL Core as a feature set. The test requires
`sht:SPARQLNodeExpr`.)

SPARQL, "Select Expressions" (`#SelectExpression-evaluation`): the output nodes
of a select expression are the bindings of the one projected variable, with
`focusNode` pre-bound as `this`.

NodeExpr, "Getting started" (informative) lists `sh:deactivated` among the
properties whose value is a node expression.

Core, "Handling of Ill-formed Shapes Graphs": "If the shapes graph contains
ill-formed nodes, then the result of the validation process is undefined. A
SHACL processor SHOULD produce a failure in this case."

### What a conforming processor must or may produce

It must evaluate the expression with the focus node as `focus node`, deactivate
the shape exactly where the only output node is `true`, and ignore the shape
there. For `ex:ActiveShape` the output is `false`, so the shape is validated and
reports the datatype violation; for `ex:DeactivatedShape` the output is `true`,
so it reports nothing. The approved result is therefore the only result a
processor that evaluates node-expression values can produce. A processor that
declines to evaluate them is not a processor of the claimed class
(`sht:SPARQLNodeExpr`); it cannot report a different result, only refuse.

### Conclusion: engine defect, fixed

The engine accepted only a boolean literal. It now evaluates a structured
`sh:deactivated` value for every shape and property shape, at each focus node, at
validation time. G1 does not apply.

Engine behaviour where the text leaves the outcome open, chosen and tested:

- The expression yields exactly one `xsd:boolean` literal: `true` deactivates
  (compared as the RDF term `true`, as for the constant spelling), anything else
  leaves the shape active.
- The expression yields no node, several nodes, or one that is not an
  `xsd:boolean` literal: the validation fails, naming the shape, the focus node
  and what was produced. The syntax rule says the value "must have either `true`
  or `false` as the (only) output node"; the text says that an ill-formed shapes
  graph SHOULD produce a failure. This is the choice made; the other reading
  (treat as not deactivated) is also permitted by the text. See the
  candidate editorial clarifications below.
- A constant that cannot satisfy the rule (an IRI, a non-boolean literal, the
  empty expression) is refused when the shapes graph is read, because its only
  output node is itself.
- Neighbours, all executed (`tests/shape_deactivation_expression.rs`): a path
  expression and a select expression deactivate one focus node and not another;
  constant `true` and `false`; a focus-independent `shnex:if` yielding `true`; the
  same for a property shape; a deactivated shape reached by `sh:node` conforms at
  the value node where it is deactivated; a non-boolean, a two-valued and an empty
  result each fail while a boolean result at the neighbouring focus node
  validates; the three bad constants are refused while `true` and `false` load.

### The same expression on constraints, on rules, and under the 2017 Recommendation

The dated texts define a node-expression `sh:deactivated` in more places than the
case exercises. Each is implemented, or its absence is cited.

**SPARQL-based constraints (implemented, editor's draft only).** SPARQL, "Syntax of
SPARQL-based Constraints" (`#SPARQLConstraint-deactivated-maxCount`), as of `07458ebb`
(2026-10-10). The WD says instead "this value is either true or false" (boolean only),
so `shacl-wd-2026-09-18` refuses a node expression there and `shacl-ed-2026-10-16`
evaluates it:

> SPARQL-based constraints may have at most one value for the property
> `sh:deactivated` and this value is a node expression that must have either `true`
> or `false` as its (only) output node, as for shapes.

SPARQL, "Validation with SPARQL-based Constraints":

> If `$sparql` has a value `expr` for the property `sh:deactivated` and
> `evalExpr(expr, data graph, focus node, {})` produces `true` as its only output
> node, then the constraint is deactivated and there are no validation results.

The constraint is deactivated at the focus nodes where the expression produces
`true`, with the same evaluation rules as for a shape (exactly one boolean `true`
deactivates; no node, several, or a non-boolean fails the validation, the same
SHOULD-fail reading). An `sh:expression` constraint node is read the same way.
Tested: a path expression and a select expression deactivate one focus node and not
another; constant `true` and `false`; empty, two-valued and non-boolean results fail
(`tests/shape_deactivation_expression.rs`).

**Constraint reifiers (implemented).** Core, "Deactivating Shapes and Constraints":

> A triple that has a shape as subject, a parameter (such as `sh:minCount`) as
> predicate can have at most one reifier with a value for the property
> `sh:deactivated`. Let `expr` be the value of `sh:deactivated` in a reifier on a
> triple that has shape subject and a parameter as predicate. If `evalExpr(expr,
> data graph, focus node, {})` produces `true` as its only output node, the
> constraints that use the triple are called deactivated constraints. Deactivated
> constraints are ignored during validation.

So the dated text does give a node expression meaning on a reifier. It is evaluated
at the focus node of the shape that holds the constraint, for a node shape's
constraint, a property shape's constraint, the `sh:property` statement that reaches
a property shape, a SPARQL-based constraint and `sh:reificationRequired`, each with
its boolean and its failing neighbours in the same test file. A constraint can have
several statements (T) and several sources (its reifiers, its constraint node); it
is deactivated where any of their expressions produces `true`.

**Rules (implemented for shapes; the rule's own value stays boolean).** Inference
Rules, "Deactivated Rules (`sh:deactivated`)":

> Rules may be deactivated by setting `sh:deactivated` to `true`. [...] The values of
> `sh:deactivated` are either of the `xsd:boolean` literals `true` or `false`.

That text defines no node expression for a rule, so a rule's own
`sh:deactivated` stays boolean-only and a node expression there is refused, citing
that sentence. The neighbours `true` and `false` load, and `true` switches the rule
off. A shape that carries `sh:rule` is another matter: Inference Rules says the rule
"is executed for each target node of each of the linked and non-deactivated
shapes", and "non-deactivated" refers to Core's definition, which is a node
expression. The rule therefore runs at the target nodes where the shape's
expression produces `false` and not where it produces `true`; the same holds for
the expected derived triples of `sh:expectedPredicate` (a property shape deactivated
at a focus node derives nothing there), and a condition superclass deactivated by an
expression is judged at the focus node the condition judges.

**2017 Recommendation (refused).** SHACL REC 2017-07-20, "Deactivating Shapes"
(`#deactivated`): "The value of `sh:deactivated` in a shape must be either `true`
or `false`." Under `shacl-rec-2017-07-20` a node expression is refused at load
wherever it is written (a shape, a SPARQL-based constraint, a reifier), and the
boolean spellings load. The dated 1.2 profile accepts the same expression, which
shows the refusal is the law's.

Remaining engine limits, not specification disagreements: JSON Schema output cannot
state a condition on the focus node, so a shape or constraint with such an expression
is projected as active and the loss is recorded on the shape.

### Spec text that made the case harder (candidate editorial clarifications)

1. The sentence "In SHACL Core, the only valid values for `sh:deactivated` are the
   constant literal node expressions `true` and `false`" sits in the same
   section as the rule that makes any node expression valid. A reader can take it
   as restricting Core processors. It could say that this is the whole of what a
   processor without node-expression support must accept.
2. The syntax rule ("must have either `true` or `false` as the (only) output
   node") is a rule about the shapes graph that cannot be checked until a focus
   node exists. The evaluation sentence defines deactivation only for
   "`true` as its only output node" and says nothing about any other output.
   The text could say whether a dynamic violation is a failure, an ill-formed
   shapes graph, or simply "not deactivated".
3. `evalExpr(expr, data graph, focus node, {})` needs a focus node, but the
   places that consult deactivation without one (rule linking, target
   resolution) are not addressed. It is unclear whether a shape with targets is
   deactivated before or after its targets are computed.
4. Inference Rules defines `sh:deactivated` on a rule as a boolean literal, while the
   shape it is linked to may use a node expression. The text could say so where
   rules are linked to "non-deactivated shapes".
5. SPARQL says a SPARQL-based constraint's value is a node expression "as for
   shapes", but the focus node it is evaluated at for a property-shape constraint
   (the property shape's focus node, not a value node) is stated only by Core's
   sentence for reifiers.

---

## 2. `sparql/node/prefixes-003`

### The case

```turtle
<http://example.com/base>   owl:versionIRI <http://example.com/base/v1> ;
                            sh:declare [ sh:namespace "http://example.com/ns#" ; sh:prefix "ex" ] .
<http://example.com/shapes> owl:imports <http://example.com/base/v1> ;
                            owl:versionIRI <http://example.com/shapes/v2> ;
                            sh:declare [ sh:namespace "http://test.com/ns#" ; sh:prefix "test" ] .
ex:TestSPARQL sh:prefixes <http://example.com/shapes/v2> ;
              sh:select """SELECT $this ?value WHERE {
                  $this ex:property ?value . FILTER (?value = test:Value) }""" .
```

The query uses two prefixes, `ex:` and `test:`. `sh:prefixes` names the *version
IRI* of the shapes graph; that graph imports the *version IRI* of the base graph.
The case tests that the prefix path reaches `sh:declare` through a version IRI
named directly and through an import that is itself a version IRI.

### Approved expected result

One violation: focus node `ex:InvalidResource1`, value `<http://test.com/ns#Value>`,
`sh:SPARQLConstraintComponent`, source constraint `ex:TestSPARQL`.
`ex:ValidResource1` has no `ex:property` and is not reported.

### PurRDF before the fix

The shapes graph was refused at load:

> sh:sparql constraint on shape ex:TestShape has an unparsable sh:select query:
> SPARQL syntax error at byte 589: undeclared prefix "test"

### Normative text

SPARQL, "Prefix Declarations for SPARQL Queries" (`#sparql-prefixes`), editor's draft
as of `07458ebb` (2026-10-10). The WD path is
`sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare`, and the 2017
Recommendation's is `sh:prefixes/owl:imports*/sh:declare`; each law reads its own:

> A SHACL processor collects a set of prefix mappings as the union of all
> individual prefix mappings that are values of the SPARQL property path
> `sh:prefixes/(^owl:versionIRI)?/(owl:imports/(^owl:versionIRI)?)*/sh:declare`
> of the SPARQL-based constraint, validator or SPARQL-based node expression. The
> `(^owl:versionIRI)?` elements support graphs that are referenced by their
> `owl:versionIRI`, either directly as a value of `sh:prefixes` or via
> `owl:imports`: they navigate from the version IRI to the shapes graph IRI
> before the prefix declarations are collected and further `owl:imports` are
> followed.

> The SHACL-SPARQL processor MUST produce a failure if the resulting query string
> cannot be parsed into a valid SPARQL 1.2 query.

### What a conforming processor must or may produce

Following the path from `<shapes/v2>`: the first optional `^owl:versionIRI` step
reaches `<shapes>` (and `<shapes/v2>` itself, by skipping the step); `sh:declare`
there yields `test`. `owl:imports` from `<shapes>` reaches `<base/v1>`; the
optional step after it reaches `<base>`; `sh:declare` there yields `ex`. Both
prefixes are declared, the query parses, and the approved result follows. A
processor that collects fewer mappings must refuse the query (the MUST-fail
sentence), which is what PurRDF did. The expected result is the only one a
conforming processor produces.

### Conclusion: engine defect, fixed

The engine implemented the previous form of the path (the working draft's, which `shacl-wd-2026-09-18` still reads, per `T13-triage.md`),
`sh:prefixes/(^owl:versionIRI?/owl:imports)*/sh:declare`, in which only the
targets of `owl:imports` contribute declarations and the node named by
`sh:prefixes`, and the graph its version IRI leads to, do not. It now collects
the declarations at every node the dated path reaches. G1 does not apply.

Neighbours, executed (`tests/sparql_prefixes.rs`): the existing version-IRI and
import chain; declarations on the version-IRI node, on the graph it versions, on
an import and on the graph an imported version IRI versions are each reached,
and a graph that declares only one of the two labels leaves the other undeclared
and is refused.

### Spec text that made the case harder (candidate editorial clarification)

The explanatory sentence says the version-IRI steps "navigate from the version IRI
to the shapes graph IRI before the prefix declarations are collected". Read
alone, that can suggest the intermediate node is only a stepping stone and its
own `sh:declare` is not collected. The path itself includes it (the step is
optional and the path ends in `sh:declare` at whatever node the previous step
reached). The sentence could say that declarations on both the version IRI and the
graph IRI are collected. The working draft's path
`(^owl:versionIRI?/owl:imports)*` had different reach, and the change log is silent
about the difference.

---

## 3. `sparql/functions/blankNodeArgument-001`

### The case

`ex:identity` is a custom list-parameter function whose body is `$arg0`. A
SPARQL-based constraint calls it twice, with a blank node and with an IRI:

```sparql
BIND (ex:identity(?blankNode) AS ?fromBlankNode) .
BIND (ex:identity(?iri) AS ?fromIRI) .
BIND (CONCAT(str(bound(?fromBlankNode)), ' ', str(?fromIRI)) AS ?value) .
```

The case is labelled "a custom SPARQL function called with a blank node
argument, which is an error".

### Approved expected result

One violation with `sh:value "false http://example.com/ns#IRI"`: the call with the
blank node is an error (so `?fromBlankNode` is unbound), the call with the IRI
returns it.

### PurRDF before the fix

`sh:value "true http://example.com/ns#IRI"`: the blank node was passed as the
argument and returned, so `?fromBlankNode` was bound.

### Normative text

SPARQL, "Evaluation of Custom SPARQL Functions" (`#SPARQLFunctions-evaluation`). The
first two sentences are in the WD; the last ("Note that blank nodes cannot be passed
as arguments ...") was added by `07458ebb` (2026-10-10), so under
`shacl-wd-2026-09-18` a blank node is an ordinary argument and only
`shacl-ed-2026-10-16` makes it an error:

> Evaluate each SPARQL expression from `args`, yielding a new list of nodes
> `nodesList`. Where the evaluation has caused an error, the result of the SPARQL
> function call is also an error, unless the corresponding `sh:parameter` for the
> argument has `sh:optional true`. In that case, the argument will be unbound in
> the scope. Note that blank nodes cannot be passed as arguments: they are not
> constant node expressions, so an argument that evaluates to a blank node leads
> to an evaluation failure and thus an error.

### What a conforming processor must or may produce

An argument that evaluates to a blank node is an evaluation failure, and so an
error; for a required parameter the whole call is an error and a `BIND` of it
leaves the variable unbound, so `bound(?fromBlankNode)` is `false`. The IRI call
is unaffected. For a parameter with `sh:optional true` the error leaves the
argument unbound in the scope instead. The approved result is the only result
a conforming processor produces; a processor that passes the blank node through
contradicts the sentence.

### Conclusion: engine defect, fixed

The engine bound blank-node arguments like any other term. A blank node argument
now makes the call an error (a required parameter) or an unbound argument (an
optional one). G1 does not apply.

While there, one adjacent defect in the same sentence was fixed: an unbound
optional argument followed by a bound one used to end the argument list, dropping
the later argument; the scope is keyed by index, so the later argument now keeps
its index and only the unbound position is absent.

Neighbours, executed (`tests/custom_node_expressions.rs`): a blank node to a
required parameter is unbound while an IRI to the same function is returned; with
optional parameters, `ex:total(1, ?blank, 5)` is 6, not 1.

### Spec text that made the case harder (candidate editorial clarification)

The rule that a blank node argument is an error appears only inside the second
paragraph, introduced by "Note that", and is justified by "they are not constant
node expressions". Constant node expressions are defined for node expressions
written in the shapes graph; the arguments here are SPARQL values, so the
justification does not apply as written. The rule is normative in effect (the
test depends on it) and could be stated as a rule: "an argument that evaluates to a
blank node is an error". It could also say whether a blank node returned by a
function body, or produced by a `BNODE()` call inside the body, is subject to
the same restriction.

---

## 4. `sparql/functions/singleInstanceOf-001`

### The case

`ex:singleInstanceOf` is a custom list-parameter function whose body selects
`?result a $arg0`. The query calls it with a class that has one instance, a class
that has two, and a class that has none:

```sparql
BIND (ex:singleInstanceOf(ex:One)  AS ?one) .
BIND (ex:singleInstanceOf(ex:Two)  AS ?two) .
BIND (ex:singleInstanceOf(ex:None) AS ?none) .
BIND (CONCAT(str(?one), ' ', str(bound(?two)), ' ', str(bound(?none))) AS ?value) .
```

The case is labelled "a custom SPARQL function that is an error unless its body
produces exactly one output node".

### Approved expected result

One violation with `sh:value "http://example.com/ns#OnlyInstance false false"`: the
one-instance call returns its instance, and the two-instance and no-instance
calls are errors, leaving `?two` and `?none` unbound.

### PurRDF before the fix

The validation aborted with a failure that reached the caller:

> error native-sparql-function-operational: host function error: custom SPARQL
> function: custom list parameter function <ex:singleInstanceOf> produced 2
> output nodes; SHACL 1.2 Node Expressions §6.2 reports an evaluation failure when
> there is more than one

No report was produced. The no-instance call already produced an unbound value.

### Normative text

SPARQL, "Evaluation of Custom SPARQL Functions" (`#SPARQLFunctions-evaluation`):

> If the list of output nodes `rs` has exactly one member, then return that node.
> Otherwise, or if the evaluation produces an evaluation failure, the result of
> the SPARQL function call is an error.

NodeExpr, custom list-parameter functions (§6.2): "An evaluation failure must be
produced if there is more than one output node."

SPARQL 1.2 Query, error semantics (the reference for "error" above): an error in
the expression of a `BIND` leaves the variable unbound; it does not fail the
query.

### What a conforming processor must or may produce

The two sentences address two contexts. Evaluated as a node expression, more than
one output node is an evaluation failure (NodeExpr §6.2). Called from SPARQL, the
rule is that anything but exactly one output node, and any evaluation failure,
makes the function call "an error", which is a SPARQL expression error: the `BIND`
leaves its variable unbound and the query continues. The approved result is the
only result a conforming processor produces. A processor that fails the whole
validation because one `BIND` produced a SPARQL error does not follow the
sentence.

### Conclusion: engine defect, fixed

The engine reused the node-expression failure as a hard failure of the query. A
call whose body produces any number of output nodes other than one now yields the
call's SPARQL error (an unbound result). G1 does not apply.

Neighbours, executed (`tests/custom_node_expressions.rs`): a body producing two
nodes and a body producing none are unbound in a `BIND` without failing the query;
a body producing one is returned. The node-expression evaluator's own rule
(more than one output node is an evaluation failure) is unchanged and still tested.

Resolved (Task 17, G-05): evaluation failures and processor failures are now
distinguished by type, so every evaluation failure inside a function body is the
call's SPARQL error and only processor failures still fail the query.

- The sentence is law-independent. "Otherwise, or if the evaluation produces an
  evaluation failure, the result of the SPARQL function call is an error." is in the
  working draft (it is present at `07458ebb^`, before the editor's-draft-only blank-node
  sentence) and in the editor's draft at `4cd46986`
  (`shacl12-sparql/index.html`, "Evaluation of Custom SPARQL Functions"). The engine
  applies it under `legacy`, `shacl-wd-2026-09-18` and `shacl-ed-2026-10-16` alike.
- `ShapesError::failure_class()` classifies by variant, exhaustively:
  `Invalid` (the evaluator's channel: arity, list-argument cardinality, a shape
  argument that names no shape, "produced N nodes", and the like) is
  `FailureClass::Evaluation`; `Storage` and the new `Processor(ProcessorFailure)`
  (governor trip, stop signal, incomplete relation index, call-depth bound,
  expression-depth bound, re-entry cycle, a query-engine failure whose stable code
  requires propagation) are `FailureClass::Processor`; the typed refusals of the
  shapes graph (`Imports`, `ShaclJs`, `IllFormed`, `Prebinding`, `UnsupportedTarget`,
  `SparqlTargetDisagreement`) are `FailureClass::Refusal`. Only `Evaluation` becomes
  `Ok(None)`; the other two classes still fail the query, so a recursion bound or a
  fuel trip is never hidden as an unbound variable.
- The single mapping is `sparql_call_result` in
  `crates/shapes/src/shapes/parser/functions.rs`; the custom-function call and the
  declared built-in list-parameter call share it, so a built-in with an evaluation
  failure or a count other than one output node is an error of the call too.
- Executed (`tests/custom_node_expressions.rs`, `tests/governor_inheritance.rs`,
  `functions.rs` unit test): the G-05 probe (`sparql:abs` over a two-node select,
  `BIND` then `str(bound(?r))`) yields `?value "false"` in a report; an unresolvable
  shape argument is unbound for the custom body and the built-in
  `shnex:conformsToShape`, beside a real shape that returns its verdict; a fuel trip
  inside a body still ends the governed validation; the recursion tests still fail
  closed.

#### Audit of what reaches `sparql_call_result` as `Invalid` (review F2)

`Invalid` is the landing type of every `String` error, so each construction reachable
from a function body was checked. Internal defects and processor failures were moved
out; what remains is an evaluation failure.

Moved to `ProcessorFailure::Internal`: `CustomFunction::body` (no installed
`sh:bodyExpression`); every "internal validation-plan defect" in `plan.rs`
(`ShapePlan` lookups, `slot_defect`, `lowering_defect`, `expr_defect`, `class_id`,
`index`, `scalar_query`, `body_iri`, `constant_class_id`, the `pair*` helpers; their
signatures now return `ShapesError`, with callers in `constraints.rs`, `engine.rs`,
`expression.rs`, `path.rs`, `rules.rs`, `unique_values.rs` following); the
"shape index was never filled" cases of `shnex:conformsToShape` and
`sh:nodeByExpression`. Moved to `ProcessorFailure::Admission`: the dated query law's
refusal (`query_law::Runtime::admit`). Routed through `query_error` (so a
propagation-required code is `Processor::Query`): the query-law
`prepare_interned_request` failure in `run_query_view`.

Remaining `Invalid` constructions reachable from a body, each a genuine evaluation
failure: arity and list-argument cardinality ("argument N produced M nodes"); a
`shnex:conformsToShape` node argument yielding several nodes or a shape argument that is
not exactly one IRI or names no shape of the shapes graph; non-boolean and other value
errors raised by the node-expression kinds; and a nested query whose engine diagnostic
carries a non-propagating code (`native-sparql-query-parse`, `-algebra`,
`-property-function`, the `subst-*` family). The last are query text a body supplied
that the engine could not run; a malformed body is already a load error.

Tests: `a_processor_failure_inside_a_sparql_called_body_fails_validation_with_a_typed_cause`
and `body_level_processor_failures_are_typed` (`custom_node_expressions.rs`),
`a_dated_admission_refusal_inside_a_function_body_still_fails_the_validation`
(`dated_execution.rs`, WD and ED laws), the extended unit test in `functions.rs`.
Mutation check, run and restored: classifying `Processor(Query)` as Evaluation fails
`a_processor_failure_inside_a_sparql_called_body_...` and
`a_cycle_through_sparql_query_text_is_bounded_too`; classifying `Processor(Internal)` as
Evaluation fails `body_level_processor_failures_are_typed`. The governor-trip test
observes the latched outcome only and says so.

#### Call-site arity of a built-in list function (T17 note, decided in Task 21)

`invoke_native_list_function` raises a hard `EvalError::function` for a call of the
wrong number of arguments: an operator form (`sparql:add`, the unary and logical forms,
`sparql:ebv`, `sparql:in`) with the wrong operand count, and `shnex:conformsToShape`
with other than two arguments. Question: under SPARQL 1.2 §7.3 ("if the evaluation
produces an evaluation failure, the result of the SPARQL function call is an error"), is
that an evaluation failure (unbound) or a defect that fails the query?

Decision: it fails the query, unchanged. An evaluation failure is a property of the
argument VALUES (a type error, a missing node, a body producing the wrong number of
nodes); a wrong operand count fails identically for every value, so it is a defect of the
call as written, known when the query is built. The node-expression form of the same
call (`[ sparql:add ( 1 2 3 ) ]`) is refused at load, and a custom function called with
the wrong number of arguments already fails the query ("expects exactly N
argument(s)", the registry's arity); the built-ins now agree with both. A name that is
not callable (an aggregate, `filter-exists`, an unknown name) is refused when the shapes
graph loads, so it never reaches the closure. `shnex:conformsToShape` is registered
`Arity::Exact(2)`, so the registry refuses a wrong count before the closure runs and the
closure's own count check is a backstop.

Test: `a_call_of_the_wrong_arity_fails_the_query_while_a_value_error_leaves_it_unbound`
(`custom_node_expressions.rs`): `sparql:add(1, 2)` is 3, `sparql:add(1, "x")` and a
custom function with an unbound required argument are unbound, and `sparql:add` with one
or three arguments and a custom function with two arguments fail the query. Related defect found while probing, and fixed: the SPARQL parser accepted a keyword
function with the wrong number of arguments (`ABS(1,2)` returned `1`, `ABS()` was
unbound). `finish_call` now refuses it with a syntax error, from the one arity table
(`builtin_arity`, derived from the SPARQL 1.2 Query grammar rules [141]-[144]), and
`SparqlCallForm::render` asks the same table (`builtin_keyword_arity`), so
`sparql:abs(1, 2)` is refused like `sparql:add(1, 2, 3)`.

### Spec text that made the case harder (candidate editorial clarification)

1. NodeExpr §6.2 and SPARQL §7.3 each say something about the same situation (a
   body with several output nodes), in different vocabulary: "evaluation
   failure" in the first, "error" in the second. The reader must know that an
   evaluation failure inside a SPARQL call becomes a SPARQL error. The text
   could say so where the failure is defined.
2. Core distinguishes a *failure* (the processor reports one through an
   implementation-specific channel, including for resource exhaustion) from the
   node-expression *evaluation failure*. The SPARQL sentence turns evaluation
   failures into errors but does not say whether a processor failure inside the
   body (for example a resource limit) is also swallowed. It should say that
   only evaluation failures are.


## 5. `node-expr/shnex-sparql/hasLang-example` (SPEC_CONTRADICTED, U10)

### The case
`sht:EvalNodeExpr`, approved: `[ sparql:hasLang ( "hello"@en "en" ) ]`, `mf:result ( true )`.
The same expression is the `shnex-sparql:example` of `sparql:hasLang` in `shnex-sparql.ttl`.

### Normative text (SPARQL 1.2 Query, https://www.w3.org/TR/sparql12-query/, fetched 2026-10-18)
Rule [141] `BuiltInCall`: `'hasLANG' '(' Expression ')'`. Section 17.4.2.10 `hasLANG`:
prototype `xsd:string hasLANG (RDF term term)`; examples `hasLANG("abc"@en)` and
`hasLANG("abc"@en--ltr)`.

### PurRDF output
Before: the second argument was accepted and silently ignored (`hasLANG(?a0, ?a1)` parsed;
the keyword arity was unchecked), result `true`. Now: the two-argument call is refused
("takes exactly 1 argument(s), got 2") at shapes load, like every wrong-arity call; the
one-argument call `[ sparql:hasLang ( "hello"@en ) ]` evaluates to `true`.

### G1 eligibility
1. Met. The grammar production fixes one argument and the function table lists one
   parameter, so a processor that conforms to the grammar cannot evaluate the
   two-argument call to `true`; the only way to produce the approved result is to drop or
   ignore an argument, which is not a reading of the text but a deviation from it. The
   second argument has no defined meaning (no parameter to bind), and
   `sparql:hasLangdir` in the same vocabulary takes one.
2. Met. This is not an unsupported-feature or not-evaluated refusal: `sparql:hasLang` is a
   claimed, implemented function (`NodeExpr`, `SPARQLNodeExpr` classes are claimed and the
   one-argument form passes); the refusal is of a malformed call.
3. Met. The refusal has its neighbours: `sparql:hasLang ( "hello"@en )` evaluates to `true`
   (`a_sparql_ns_keyword_function_with_the_wrong_arity_is_a_load_error` covers the wrong
   counts and right counts of the keyword class; the keyword table test covers `hasLANG`
   with 0 and 2 refused, 1 accepted).
4. Drafted, not filed: `contributions/data-shapes/purrdf-earl/drafts/wg-issue-hasLang.md`
   and item 9 of `wg-findings.md` (for Patrick's approval).

Caveat on (1): the Node Expressions text defines no arity rule for `sparql:<name>` calls,
and the vendored `shnex-sparql.ttl` reads as two-argument for `sparql:hasLang` (its own
`shnex-sparql:example` is the two-argument call). Condition (1) therefore holds under the
reading that the grammar's arity governs a `sparql:` call, and whether it does is exactly
the question the WG item asks. If the WG answers that extra arguments are ignored, or that
the function takes a second argument, the case is fixed and leaves the table.

### Harness
`SPEC_CONTRADICTED` holds the case with the grammar rationale; the count is pinned at 1;
`spec_contradicted_entries_fail_unamended` refuses an XPASS and an unknown id; the EARL
reports it `earl:failed` with `earl:info` equal to the rationale. Printed lines:
`W3C12 TOTAL: passed 590, spec-contradicted 1`, `SHACL root: passed 381, spec-contradicted 1 of 382`,
class `NodeExpr: passed 145 of 146`.

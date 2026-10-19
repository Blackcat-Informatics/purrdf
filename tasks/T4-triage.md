# T4 triage: `sh:closed` on a property shape (`core/property/closed-001`)

One approved `sht:Validate` case of the SHACL 1.2 test suite (`w3c/data-shapes`,
`shacl12-test-suite/tests/core/property/closed-001.ttl`, checkout `4cd469867a`)
failed under the dated profile `shacl-wd-2026-09-18` (SHACL Core WD 2026-09-17).
The section is written so it can be lifted into a Working Group discussion item
with minimal editing. It is an engine defect, fixed. It does not meet G1, so it
does not enter `SPEC_CONTRADICTED`.

Short name: Core = SHACL 1.2 Core.

## The case

Two data nodes each point at an address node; one address has an unlisted
property.

```turtle
ex:Alice ex:address ex:Address1 .
ex:Address1 ex:city "Berlin" .
ex:Bob ex:address ex:Address2 .
ex:Address2 ex:city "Paris" ; ex:zip "75001" .

ex:TestShape a sh:NodeShape ;
  sh:property ex:TestShape-address ;
  sh:targetNode ex:Alice , ex:Bob .
ex:TestShape-address
  rdfs:comment "A property shape has no sh:property, so only the ignored properties are allowed at its value nodes" ;
  sh:path ex:address ;
  sh:closed true ;
  sh:ignoredProperties ( ex:city ) .
```

`sh:closed` sits on the PROPERTY shape `ex:TestShape-address`, not on a node
shape. Its value nodes are `ex:Address1` (for focus `ex:Alice`) and
`ex:Address2` (for focus `ex:Bob`). The shape has no `sh:property` of its own.

## Expected result (status `sht:approved`)

`sh:conforms false` and exactly one result:

| property | value |
|---|---|
| `sh:focusNode` | `ex:Bob` |
| `sh:resultPath` | `ex:zip` |
| `sh:value` | `"75001"` |
| `sh:resultSeverity` | `sh:Violation` |
| `sh:sourceConstraintComponent` | `sh:ClosedConstraintComponent` |
| `sh:sourceShape` | `ex:TestShape-address` |

## PurRDF's output before

`conforms mismatch: produced=true, expected=false`: an empty report. The engine
evaluated `sh:closed` only for a node shape, over the shape's own focus node. For
a property shape the parser accepted the constraint and the planner lowered it
(with the shape's `sh:property` children), but the evaluator's constraint arm for
`sh:closed` was an unconditional no-op, so the constraint was silently dropped.
This is the "constraint that stopped constraining" failure, not a refusal.

## Normative text

Core, section 7.9.1 "sh:closed, sh:ignoredProperties", introduction:

> The SHACL Core language includes a property called `sh:closed` that can be used
> to specify the condition that each value node has values only for those
> properties that have been explicitly enumerated via the property shapes
> specified for the shape via `sh:property`.

Textual definition:

> If `$closed` is `true` or `sh:ByTypes` and `P` is the set of properties defined
> below, then there is a validation result for each triple that has a value node
> as its subject and a predicate that is not in `P`. If `$ignoredProperties` has a
> value then the properties enumerated as members of this SHACL list are also
> permitted for the value node. The validation result MUST have the predicate of
> the triple as its `sh:resultPath`, and the object of the triple as its
> `sh:value`.
>
> If `$closed` is `true`, then `P` is the set of IRI properties that can be
> reached from the current shape via the SPARQL path `sh:property/sh:path`.
>
> If `$closed` is `sh:ByTypes`, then `P` is the set of IRI properties that can be
> reached from the value node via the following algorithm, plus `rdf:type`: [...]

Core, "Value Nodes of Node Shapes": "For node shapes the value nodes are the
individual focus nodes, forming a set with exactly one member." Core, "Value Nodes
of Property Shapes": the value nodes are the nodes the path reaches from the focus
node (plus `sh:values` / `sh:defaultValue` output). Core, "Path
(sh:resultPath)": results of a property shape carry the shape's `sh:path` "unless
stated otherwise"; the closed-constraint sentence above states otherwise (the
triple's predicate).

## What a conforming processor must produce

The constraint-component definition is stated over "a value node", and the
document defines value nodes for both node shapes and property shapes. Nothing
restricts `sh:closed` to node shapes (the Constraint Component table lists it
with the other parameters usable on any shape). For `ex:TestShape-address`:

- `ex:Address1` has one triple, `ex:city`, which `$ignoredProperties` permits.
  `P` is empty (no `sh:property/sh:path` from the shape), so no result.
- `ex:Address2` has `ex:city` (permitted) and `ex:zip` (not in `P`, not ignored):
  one result, with `sh:resultPath ex:zip` and `sh:value "75001"` as the MUST
  clause requires.
- `sh:focusNode` is the focus node of the property shape (`ex:Bob`), as for every
  result of a property shape; `sh:sourceShape` is the shape declaring the
  constraint.

That is exactly the approved result. A processor may differ in nothing the report
fixes.

## Conclusion

Engine defect; the approved result follows from the dated text. G1 condition (1)
("no processor conforming to the quoted sentences can produce the approved
result") fails, so (2) to (4) do not arise. Fixed in `crates/shapes/src/constraints.rs`.

The same definition appears in the SHACL 2017 Recommendation (section 4.8.1: "each
value node has values only for those properties ... `sh:property` ..."; "a
validation result for each triple that has a value node as its subject"), so the
behaviour is not specific to the 1.2 profile and is not gated on it. The 2017
vector cases `core/node/closed-001` and `-002` (node shapes, value node = focus
node) are unchanged and still pass; the only change is that a property shape's
`sh:closed` is no longer discarded.

## Fix

`eval_constraint` (the per-constraint evaluator) now handles `sh:closed` when the
constraint belongs to a property shape: it runs the existing `eval_closed` check
over each value node of the shape, using the permitted-predicate set the planner
already lowered (the property shape's own `sh:property/sh:path` values plus
`sh:ignoredProperties`, or the `sh:ByTypes` collection through the value node's
`rdf:type`). The property-shape result stamp supplies the focus node; the result
keeps the offending predicate as `sh:resultPath` (the stamp only fills in the
shape's `sh:path` when none is set) and the object as `sh:value`.
`eval_closed` now takes the source shape from the constraint source rather than a
node `Shape`, so both callers share it. Node-shape evaluation is unchanged.

Graph-box roles on a property-shape closed result: source roles are the property
shape's (merged with its ancestors', as for every property-shape result); path
roles describe the offending predicate (`ex:zip`), as a node-shape closed result's
do, not the shape's own `sh:path`. The result stamp keeps the roles the closed
constraint resolved for the predicate. The text prescribes no box roles.

## Neighbouring cases (`crates/shapes/tests/closed_property_shape.rs`)

- the W3C case's exact result (focus, path, value, component, source shape), one
  result only, Alice conforming;
- same shape with `ex:zip` also in `sh:ignoredProperties`: conforms;
- same shape with `ex:zip` declared by a nested `sh:property`: conforms;
- `sh:closed false`: conforms;
- value nodes only: unlisted properties on the focus node itself are not reported,
  and a literal value node (no outgoing triples) is not reported;
- several value nodes and triples: one result per offending triple, each with the
  property shape's focus node;
- box roles: source roles are the property shape's and its parent's, path roles are
  the offending predicate's (the shape's own path carries a different role in the
  data, so the two are distinguishable);
- the dated 2017 law (`REC_20170720`) and the dated 2026 law report the same result
  and roles;
- `sh:closed sh:ByTypes` on a property shape: a property reached through the value
  node's class shape is permitted, an unrelated one is reported, `rdf:type` is
  permitted; the same data under `sh:closed true` additionally reports `zip` and
  `rdf:type`; a fully covered value node conforms.

## Text that made it harder (candidate editorial clarifications)

1. The introduction and the constraint definition speak of "the current shape" and
   "the shape" in a way that reads as a node shape (`$this`-style); the interplay
   with property shapes (the closed shape is the property shape, `P` comes from
   its nested `sh:property`, value nodes are the path's) is left to inference from
   the two Value Nodes subsections. One sentence stating that on a property shape
   the value nodes are the nodes whose triples are checked, and that
   `sh:focusNode` is the property shape's focus node, would remove the ambiguity.
2. "Path (sh:resultPath)" carries the exception for `sh:closed` only as an HTML
   comment (`<!-- sh:closed is an exception -->`). The rendered text says "unless
   stated otherwise" with no pointer to the closed definition, so a reader
   applying the general rule would put the shape's `sh:path` in the result. The
   comment should become text, e.g. "(sh:closed results carry the triple's
   predicate; see sh:closed)".
3. The informative sentence "A property shape has no sh:property, so only the
   ignored properties are allowed at its value nodes" appears in the test
   `rdfs:comment`, not the specification. The specification does not say that
   `sh:closed true` on a property shape with no `sh:property` permits nothing
   beyond `sh:ignoredProperties` (not even `rdf:type`), though that follows from
   the definition of `P`. An example would help.

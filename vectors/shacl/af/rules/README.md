# SHACL Advanced Features (AF) — Rules corpus

This directory holds the **SHACL Rules** (`sh:rule`) conformance corpus consumed
by the `purrdf-shapes` rules harness
(`crates/shapes/tests/rules_conformance.rs`). Unlike the validation corpora,
each case asserts an **inferred graph**: the triples the rules engine
(`purrdf_shapes::apply_rules` / `entail_dataset`) derives from the data under a
shapes-with-rules graph.

## Fixture format

Each case is a subdirectory `<case>/` containing:

- **`input.ttl`** (or **`input.trig`**) — the data + shapes(+rules) graph the
  rules run over. `input.trig` is used when a case needs data in a *named* graph;
  the engine flattens all graphs into the default-graph projection
  (`project_dataset`) before running rules, exactly as the validator does.
- **`expected-inferred.ttl`** — the **derived triples ONLY** (the delta the rules
  add over `input.ttl`'s graph), NOT `base ∪ derived`. The harness reconstructs
  the full expected graph as `base ⊎ expected-derived` and compares it to the
  produced `base ⊎ derived` by RDFC-1.0 canonicalization (RDF isomorphism), so
  blank-node labels never cause a false mismatch.

**`err-*`** cases have NO `expected-inferred.ttl`: the harness asserts that
`apply_rules` returns `Err` (a rule malformed at firing time, or a non-terminating
rule set).

### Why derived-only?

The vendored DASH `dash:InferencingTestCase` corpus states its expected result as
`dash:expectedResult` reified triples — precisely the inferred delta over the data
graph, not the whole graph. This corpus adopts that same convention so the
vendored ground truth transfers 1:1.

## Vendored cases (source & provenance)

The `rectangle-sparql`, `classify-square-sparql`, `rectangle-triple`,
`square-triple`, `functions-permutations`, `person2schema`, and `schema2person`
cases are converted from the pySHACL DASH rules tests.

- Repository: `RDFLib/pySHACL`
- Upstream commit: `5b46638cadde2e32efaed0ee53fc2545d5c0a179` (the SAME pin as the
  sibling AF corpus in `../`)
- Upstream path: `test/resources/dash_tests/rules/` (the `sparql/` and `triple/`
  subtrees)
- License: pySHACL is Apache-2.0; the DASH test content is from TopQuadrant
  (<https://datashapes.org/>), originally authored in TopBraid Composer.

### DASH → first-party conversion

- DASH-specific metadata (`dash:InferencingTestCase`, `dash:expectedResult`) is
  removed; the expected delta is re-expressed as `expected-inferred.ttl`.
- `owl:imports` and the `owl:Ontology` header (labels, `owl:versionInfo`) are
  removed. Where a test `owl:imports`ed a data graph (`person2schema` imports the
  `person` graph), that data is **inlined** into `input.ttl`, since PurRDF does
  not resolve `owl:imports`.
- `sh:prefixes` / `sh:declare` prefix-declaration nodes are removed; the SPARQL
  `sh:construct` / `sh:SPARQLFunction` `sh:select` bodies resolve prefixed names
  from the shapes document's own `@prefix` declarations (the fallback prefix map
  the engine threads through `from_dataset_with_prefixes`).
- Original test namespaces (`http://datashapes.org/shasf/tests/rules/...` and
  `http://schema.org/`) are preserved for provenance fidelity.

### Known divergence (harness XFAIL)

`functions-permutations` is ledgered XFAIL. SHACL-AF §5 (node expressions) does
not define the semantics of a function-call argument node expression that
evaluates to more than one value. TopBraid/DASH (and pySHACL) treat multi-valued
arguments as a **cartesian product**; PurRDF's expression evaluator makes the
opposite, equally spec-compatible choice — each function-call argument must
collapse to exactly one value, else `apply_rules` errors. The case is retained to
document this genuinely-underspecified divergence for the conformance matrix.

## First-party cases

Authored with the `example.org` fixture namespace to cover the rules surface the
vendored suite does not exercise:

- **`fp-fixpoint-chain`** — multi-round fixpoint (rule B fires only after rule A's
  output, in a later round).
- **`fp-condition-gating`** — `sh:condition` gating: one focus node fires, another
  is skipped.
- **`fp-order`** — `sh:order` on two rules; order-independence of the final
  closure is additionally proven by a Rust test in the harness.
- **`fp-deactivated`** — a `sh:deactivated` rule and a `sh:deactivated` shape are
  both skipped; only the active rule fires.
- **`fp-cyclic-data`** — a symmetric-closure rule over a 3-cycle terminates
  (value-preserving over a finite node set).
- **`fp-nonmonotonic`** — a rule gated on `sh:maxCount 0` whose own head would
  break the condition next round: SHACL Rules are monotonic-accumulative, so the
  rule fires once at check time and the derived triple is never retracted.
- **`fp-blank-minting`** — a `sh:SPARQLRule` CONSTRUCT that mints a fresh blank
  node (compared by isomorphism).
- **`fp-named-graph`** — data in a named graph; the rules see the flattened
  default-graph projection.
- **`err-literal-subject`** — a rule producing a literal in subject position;
  `apply_rules` must error.
- **`err-diverging-fresh-term`** — a rule minting a fresh term every round;
  `apply_rules` must error at the divergence bound rather than loop forever.

The `entail_dataset = apply_rules ∘ project_dataset` composition is pinned by a
Rust test (`entail_dataset_composes_project_then_apply_rules`) in the harness.

## Freezing

The whole `vectors/shacl` tree is byte-frozen; this directory's payload files are
covered by `scripts/conformance-frozen/vectors-shacl.sha256` (READMEs are excluded
from the freeze). Do not hand-edit vendored files; to re-vendor or add cases,
change the fixtures and regenerate the checksums:

```bash
python3 scripts/check-corpus-frozen.py --update
```

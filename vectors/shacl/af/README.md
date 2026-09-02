# SHACL Advanced Features (AF) vendored tests

This directory holds a validation-only subset of the SHACL Advanced Features
(AF) tests from the pySHACL DASH test corpus, converted into the W3C
`sht:Validate` manifest format used by the `purrdf-shapes` conformance harness.

## Source

- Repository: `RDFLib/pySHACL`
- Upstream commit: `5b46638cadde2e32efaed0ee53fc2545d5c0a179`
- Upstream URL: https://github.com/RDFLib/pySHACL/tree/5b46638cadde2e32efaed0ee53fc2545d5c0a179/test/resources/dash_tests
- Files vendored from `test/resources/dash_tests/`:
  - `expression/booleans-001.test.ttl`
  - `function/callSPARQLFunction.test.ttl`
  - `function/simpleSPARQLFunction.test.ttl`
  - `target/sparqlTarget-001.test.ttl`
  - `target/sparqlTargetType-001.test.ttl`

## First-party cases (not vendored)

Alongside the vendored DASH subset this directory carries FIRST-PARTY cases
authored for PurRDF against W3C specifications that the DASH corpus predates.
They are written in the same `sht:Validate` manifest format and are discovered by
the same harness; each file names the specification and section it exercises in
its header comment and uses `example.org` test IRIs.

- `expression/node-expr-shnex-001.ttl` — "SHACL 1.2 Node Expressions" §4.5.3
  (`shnex:conformsToShape`) over §4.1.2 (`shnex:var "focusNode"`).
- `expression/node-expr-shnex-002.ttl` — "SHACL 1.2 Node Expressions" §4.3.1
  (`shnex:flatMap`), §4.3.3 (`shnex:matchAll`) and §4.1.4
  (`shnex:pathValues`).
- `expression/node-by-expression-001.ttl` — "SHACL 1.2 Node Expressions" §7.2
  (`sh:nodeByExpression` / `sh:NodeByExpressionConstraintComponent`).

The "do not hand-edit" rule below applies to the VENDORED files only; first-party
cases are ordinary source, and changing one still requires refreshing the frozen
corpus checksums.

## License and attribution

pySHACL is licensed under the Apache-2.0 license. The DASH test content is
from TopQuadrant (https://datashapes.org/) and was originally authored in
TopBraid Composer; original `versionInfo` / copyright comments are preserved
as Turtle comments in the converted files where present.

## Conversion notes

- DASH-specific metadata (`dash:GraphValidationTestCase`,
  `dash:FunctionTestCase`, `dash:expectedResult`, `dash:expression`) was
  removed and replaced with W3C-style `mf:Manifest` / `sht:Validate` entries.
- `owl:imports <http://datashapes.org/dash>` was removed; the DASH ontology is
  not vendored and not required for validation semantics exercised here.
- `function/simpleSPARQLFunction.test.ttl` was a `dash:FunctionTestCase`; it
  was converted into validation tests by wrapping each function call inside an
  `sh:expression` constraint.
- Original test namespaces (`http://datashapes.org/sh/tests/...` and
  `http://datashapes.org/shasf/tests/...`) are preserved.
- SPARQL query prefix mappings are declared locally with `sh:declare` /
  `sh:prefixes` to replace the prefix declarations that were previously
  imported from the DASH ontology.

## Scope

**Included (validation-only):**
- Expression constraints (`sh:expression`)
- SPARQL functions (`sh:SPARQLFunction`) invoked from constraints and
  expressions
- SPARQL targets (`sh:SPARQLTarget`)
- Parameterized custom target types (`sh:SPARQLTargetType`)

**Explicitly excluded:**
- SHACL Rules (`sh:rule`, `sh:TripleRule`, `sh:SPARQLRule`) — out of scope for
  the validation-only AF coverage.

## Regeneration

Do not hand-edit the vendored files. To refresh from upstream or change the
vendored subset, re-fetch the source files, re-run the conversion, and then
update the frozen-corpus checksums:

```bash
python3 scripts/check-corpus-frozen.py --update
```

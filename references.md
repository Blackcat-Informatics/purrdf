# Referenced artifacts — issue #570

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `shacl12-test-suite/reports/` | NO | repo | — | — | — |
| `data-shapes-test-suite/reports/` | NO | repo | — | — | — |
| `vectors/shacl12/` | yes | repo | dir | — | — |
| `shacl12-test-suite/tests/` | NO | repo | — | — | — |
| `scripts/vendor-shacl12.py` | yes | repo | file | 267 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `vectors/shacl/` | yes | repo | dir | — | — |
| `data-shapes-test-suite/tests/` | NO | repo | — | — | — |

### History: `vectors/shacl12/`

- `ae645efe7 test(shapes): vendor the W3C SHACL 1.2 vocabularies and test suite`

### History: `scripts/vendor-shacl12.py`

- `3525eeb84 feat(shapes)!: result annotations, sh:minus, and the remaining SHACL terms`
- `11f54933b feat(cli,python,wasm,capi): rules, node expressions and shapes lint on every host`
- `ae645efe7 test(shapes): vendor the W3C SHACL 1.2 vocabularies and test suite`

### History: `vectors/shacl/`

- `ab5b27ccb fix(conformance)!: validator-001 passes with no import supplied, and the DASH vendoring it no longer needs is removed`
- `3b9a13252 docs(cli,conformance): shapes lint documents its inert section, and the AF corpus notes where DASH is vendored`
- `e4d94665e feat(datalog,shapes)!: a divergent rule set is refused within a horizon its input grants`
- `818b2250a test(shapes): a constant literal sh:subject in a triple rule is pinned as skipped`
- `c6573dcc5 feat(shapes,datalog)!: one rules engine: SHACL 1.2 rules on purrdf-datalog`
- `56c534109 feat(shapes,sparql-algebra,sparql-eval)!: align SHACL-AF with the SHACL 1.2 node-expression, rules and SPARQL-extension drafts (#227)`
- `bfa938990 fix(docs)!: name the DL fragment SHOIQ(D), date the exclusion emitters, and correct three provenance claims`
- `af9847dee fix(docs): correct eight published figures and gate the surfaces that carried them`
- `5961efdea test(shapes): SHACL Rules conformance corpus + inferred-graph harness`
- `1dd87380c feat(shapes): Complete SHACL-AF validation coverage`

## Identifiers (files containing each at HEAD)

- `w3c12_conformance`: 15
- `w3c_conformance`: 9
- `purrdf_conformance_kit::outcome::to_earl`: 1
- `passed`: 348
- `failed`: 617
- `inapplicable`: 13

## Workspace packages named

None.

## URLs

- https://w3c.github.io/data-shapes/shacl12-test-suite/reports.html
- https://w3c.github.io/data-shapes/data-shapes-test-suite/
- https://www.w3.org/TR/EARL10-Schema/

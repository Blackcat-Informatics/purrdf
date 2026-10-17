# Referenced artifacts — issue #588

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/validate/tests/fixtures/dl-proof.vectors` | yes | repo | file | 1105 | SPDX: CC-BY-4.0 |

### History: `crates/validate/tests/fixtures/dl-proof.vectors`

- `2ce33c668 feat(entail): reason over regular OWL role chains`
- `d5ee0c862 Prepare schema-invariant cardinality contradictions once per class`
- `d5f3ab224 feat(entail): decode, render and check the OWL-DL service proof at every host (#214 stage 3B)`

## Identifiers (files containing each at HEAD)

None named.

## Workspace packages named

- `purrdf-validate`

## URLs

- https://github.com/Blackcat-Informatics/purrdf/actions/runs/39054291844/job/117458981838

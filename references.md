# Referenced artifacts — issue #591

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/cli/tests/entails_cli.rs::consistency_decides_the_imports_closure` | NO | repo | — | — | — |
| `crates/entail/src/owl_dl/proof.rs` | yes | repo | file | 8387 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/cli/tests/entails_cli.rs` | yes | repo | file | 1914 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/entail/src/owl_dl/proof.rs`

- `04f6edbff Record and replay ordinary DL clash support`
- `6fbf766a2 feat: retain cancellation through recorded DL preparation`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `a6fe69de7 Preserve role-read refusals and actionable source witnesses`
- `2ce33c668 feat(entail): reason over regular OWL role chains`
- `d5ee0c862 Prepare schema-invariant cardinality contradictions once per class`
- `58de775df fix(entail): step a transitive role's closure over its sub-roles and inverse partners`
- `bf1282f00 refactor: one body per job across the query, reasoning and index engines`
- `4cf50c259 refactor: one framing, byte-cursor and alignment implementation each — purrdf_hash::frame, purrdf_core::bytes (#366)`
- `8b6af3979 test(hash): frozen framing and alignment differentials — every copy against 2,120 structured inputs (#366)`

### History: `crates/cli/tests/entails_cli.rs`

- `6b124d783 fix(entail): bind proofs to signed service verdicts`
- `7a5d95a89 refactor(census)!: near-miss bodies and test, bench and example targets are in the census; what it found is collapsed`
- `56b59e4d6 feat(entail,validate,cli,python,wasm,capi)!: every entailment service closes owl:imports and takes the evaluation limits`
- `5f1990692 feat(entail,cli): entailment closes the premise's owl:imports and refuses unused pairs`
- `cf8782bbe test: a first-party testkit supplies goldens, scratch dirs, frozen vectors and a libtest runner, and tempfile leaves the graph`
- `c0ef7ab63 fix(core,shapes,entail)!: an owl:imports is an import only from the importing document's own IRI or ontology header`
- `5d28bf0f1 fix(entail,validate,cli)!: an owl:imports already present is resolved`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `4316790e0 test(cli): collect early stdin refusals without pipe races`
- `50978ccfa fix(iri,rdf,cli,capi,shex)!: resolve relative IRI references through one base-aware layer (#223)`

## Identifiers (files containing each at HEAD)

None named.

## Workspace packages named

None.

## URLs

- https://github.com/Blackcat-Informatics/purrdf/actions/runs/39057788791/job/117469588750

# Referenced artifacts — issue #491

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/dataset.rs:279-288` | yes | repo | file | 4423 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `dataset.rs:1730` | NO | repo | — | — | — |
| `dataset.rs:1635` | NO | repo | — | — | — |
| `ir/pack/side.rs:563` | NO | repo | — | — | — |
| `dataset.rs:1609-1617` | NO | repo | — | — | — |
| `dataset.rs:282-284` | NO | repo | — | — | — |
| `crates/sparql-eval/src/statement_layer.rs:126-168` | yes | repo | file | 179 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/ir/dataset.rs` | yes | repo | file | 4423 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `ir/pack/side.rs` | NO | repo | — | — | — |
| `crates/sparql-eval/src/statement_layer.rs` | yes | repo | file | 179 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/dataset_view.rs` | yes | repo | file | 2508 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/rdf-core/src/ir/dataset.rs:279-288`

- `525b3cf1b Add a graph-free RDF carrier and rule execution`
- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `4e484ce24 fix(core): make the snapshot charge exact for composite literals and tags`
- `20614b654 Restore the agreement-rule doc to agreed_content_addressing`
- `485c666a3 Keep live row counts for touched graphs instead of scanning at publication`
- `bb61bb389 Materialize delta and composite views under their sources' configuration`
- `bf095b0f7 Keep a configured derivation predicate the dataset never interned`

### History: `crates/sparql-eval/src/statement_layer.rs:126-168`

- `47725646e Stop bounded statement scans and index segmented source probes`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `eb03ab702 fix(331): narrow the RDF 1.2 side tables by graph before the scan`
- `8b9e30946 feat(sparql-eval,purrdf,cli,python)!: bind path witnesses as property functions over the closure (#224)`

### History: `crates/rdf-core/src/ir/dataset.rs`

- `525b3cf1b Add a graph-free RDF carrier and rule execution`
- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `4e484ce24 fix(core): make the snapshot charge exact for composite literals and tags`
- `20614b654 Restore the agreement-rule doc to agreed_content_addressing`
- `485c666a3 Keep live row counts for touched graphs instead of scanning at publication`
- `bb61bb389 Materialize delta and composite views under their sources' configuration`
- `bf095b0f7 Keep a configured derivation predicate the dataset never interned`

### History: `crates/sparql-eval/src/statement_layer.rs`

- `47725646e Stop bounded statement scans and index segmented source probes`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `eb03ab702 fix(331): narrow the RDF 1.2 side tables by graph before the scan`
- `8b9e30946 feat(sparql-eval,purrdf,cli,python)!: bind path witnesses as property functions over the closure (#224)`

### History: `crates/rdf-core/src/dataset_view.rs`

- `525b3cf1b Add a graph-free RDF carrier and rule execution`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `1ef59cbf9 Address GRAPH <iri> only when the IRI names a graph`
- `d6f94f62a Preserve concrete blank identity across governed runtime allocation`
- `04e516a1a feat!: establish v3 storage and distribution contracts`
- `8d61324db refactor!: one home each for the triple-term and schema nesting bounds`
- `b8b661d2a refactor: one body per job across the foundation crates`
- `bf1282f00 refactor: one body per job across the query, reasoning and index engines`

## Identifiers (files containing each at HEAD)

- `DatasetView`: 302
- `main`: 2070
- `reifier_quads_of`: 32
- `annotations_of_with_graph`: 34
- `reifiers_of`: 3
- `reifier_quads_in_graph`: 18
- `annotation_quads_in_graph`: 19
- `cardinality_estimate`: 28
- `RdfDataset`: 743

## Workspace packages named

- `purrdf`

## URLs

None.

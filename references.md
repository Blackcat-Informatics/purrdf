# Referenced artifacts — issue #531

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `/home/paudley/Active/purrdf/.worktrees/515-sparql-eval-datalog-per-row-evidence/target/native-build/tmp` | NO | absolute | — | — | — |
| `/home/paudley/Active/purrdf/.worktrees/purrdf-shapes-dictionary-qualification/build/tmp` | NO | absolute | — | — | — |
| `crates/sparql-eval/src/engine.rs:6534` | yes | repo | file | 10978 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/shapes/src/schema_surface.rs:3462` | yes | repo | file | 6801 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/query_scope/counters.rs:22` | yes | repo | file | 315 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `bindings/python/tests/test_retrieval.py:261` | yes | repo | file | 4721 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/retrieval/tests/real_producers.rs` | yes | repo | file | 3318 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/sparql-eval/src/engine.rs` | yes | repo | file | 10978 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/sparql-eval/src/property_fn_eval.rs` | yes | repo | file | 7801 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `async-concurrency.test.mjs` | NO | repo | — | — | — |
| `query.test.mjs` | NO | repo | — | — | — |
| `bindings/python/tests/test_retrieval.py` | yes | repo | file | 4721 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `/tmp` | yes | absolute | dir | — | — |
| `tasks/T3-531-live-release-stack-attribution.md` | NO | repo | — | — | — |
| `reviews/issue-531-independent-linked-root-cause.md` | NO | repo | — | — | — |
| `crates/shapes/src/schema_surface.rs` | yes | repo | file | 6801 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/query_scope/counters.rs` | yes | repo | file | 315 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/sparql-eval/src/engine.rs:6534`

- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `675df1658 Implement native geodesic kernel and physical spatial ownership`
- `38b2b4da9 Preserve portable VM suspension depth and native projections`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `8b1fdb1d5 Preserve contextual graph sets and application admission`
- `56e636497 fix(rdflib): route graph queries through contextual algebra`
- `865e9ba18 Admit a dated query by the algebra its text parsed to`
- `e72660e87 fix(sparql): compile contextual mappings through native algebra`
- `ceaf81f7f docs(sparql): stop claiming rdflib initBindings agrees on assignments`
- `f2d559ca7 fix(sparql): join a sub-SELECT's assignment of a pre-bound name with the bound value`

### History: `crates/shapes/src/schema_surface.rs:3462`

- `065790157 Restore SCC inference and numeric backend admission`
- `0088bc920 Make datatype ranges and OWL schema coverage exact`
- `c170fce26 fix(shapes)!: every OWL property takes a node or a checked literal (OWL 2 Full floor)`
- `00f20cc81 fix(shapes): the complement of a restriction every individual meets admits no instance`
- `0e07eb3da fix(shapes): an empty qualifier bounds nothing, and an empty range states its cell exactly`
- `fe6916280 fix(shapes)!: read object-property data ranges by OWL 2 Full, and project owl:Nothing as false`
- `5526b23a2 fix(shapes): widen class fillers too, report widened and self-restricted cells as approximations`
- `2c78b616e fix(shapes): cross-kind values are a property-level fact, and cross-kind hasValue and hasSelf are read`
- `97617c43e fix(shapes): an object property restricted to a data range takes its literals`
- `c90227340 fix(shapes): judge string facets exactly, read cross-kind ranges, empty unions`

### History: `crates/rdf-core/src/query_scope/counters.rs:22`

- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`

### History: `bindings/python/tests/test_retrieval.py:261`

- `6136914ed Qualify multilingual analysis and optimize native Unicode execution`
- `ed55a59c3 Integrate explicit source-aligned analysis and auxiliary retrieval`
- `8f595df78 test(retrieval,python): a non-positive crossing weight is refused beside its neighbours`
- `a9b329567 feat(python,sparql-eval,hnsw): a multi-modal request from Python, with no knob to decline its lookups`
- `5e279c048 feat(retrieval): count what a read costs, not only what fusion pulled`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `d76958323 fix: a snapshot row names a subject something actually consulted`
- `ac7f6bf5b fix: a recorded selectivity domain is held to the law its own doc states`
- `5e6b3be94 test(349): the headline assertion reads the bound that stops the read, not a substring it shares`
- `305143484 feat(349): a plan can leave the process and be certified on the way back`

### History: `crates/retrieval/tests/real_producers.rs`

- `ae0d71918 Adapt admitted retrieval cursors and share the resident bridge`
- `5e1f3658a fix(eval): retain query evidence ownership and closed absence`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `08ed781ba retrieval: add candidate union and measured join access cost`
- `ed55a59c3 Integrate explicit source-aligned analysis and auxiliary retrieval`
- `7a5d95a89 refactor(census)!: near-miss bodies and test, bench and example targets are in the census; what it found is collapsed`
- `789fb5551 refactor(test-support): one home each for the purremb fixture, float-control registers, permutation, RNG streams and the counting allocator`
- `117a86061 refactor!: one W3C vocabulary home — purrdf_iri::vocab and purrdf_xsd::datatype replace ~830 retyped IRI literals (#366)`
- `a557fd676 refactor!: one hex implementation — purrdf_hash::hex, with Digest32 for every 32-byte digest identity (#366)`
- `e79b811eb test: assertions use std assert_eq!, and pretty_assertions leaves the graph`

### History: `crates/sparql-eval/src/engine.rs`

- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `675df1658 Implement native geodesic kernel and physical spatial ownership`
- `38b2b4da9 Preserve portable VM suspension depth and native projections`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `8b1fdb1d5 Preserve contextual graph sets and application admission`
- `56e636497 fix(rdflib): route graph queries through contextual algebra`
- `865e9ba18 Admit a dated query by the algebra its text parsed to`
- `e72660e87 fix(sparql): compile contextual mappings through native algebra`
- `ceaf81f7f docs(sparql): stop claiming rdflib initBindings agrees on assignments`
- `f2d559ca7 fix(sparql): join a sub-SELECT's assignment of a pre-bound name with the bound value`

### History: `crates/sparql-eval/src/property_fn_eval.rs`

- `5e1f3658a fix(eval): retain query evidence ownership and closed absence`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `675df1658 Implement native geodesic kernel and physical spatial ownership`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `8b1fdb1d5 Preserve contextual graph sets and application admission`
- `e72660e87 fix(sparql): compile contextual mappings through native algebra`
- `330d9670f sparql-eval: take the language-string equality extension per request`
- `f6fa617e5 shapes: preserve execution refusals across validation alternatives`
- `ef65a4633 sparql: route selected XPath laws through evaluation contexts`
- `04e516a1a feat!: establish v3 storage and distribution contracts`

### History: `bindings/python/tests/test_retrieval.py`

- `6136914ed Qualify multilingual analysis and optimize native Unicode execution`
- `ed55a59c3 Integrate explicit source-aligned analysis and auxiliary retrieval`
- `8f595df78 test(retrieval,python): a non-positive crossing weight is refused beside its neighbours`
- `a9b329567 feat(python,sparql-eval,hnsw): a multi-modal request from Python, with no knob to decline its lookups`
- `5e279c048 feat(retrieval): count what a read costs, not only what fusion pulled`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `d76958323 fix: a snapshot row names a subject something actually consulted`
- `ac7f6bf5b fix: a recorded selectivity domain is held to the law its own doc states`
- `5e6b3be94 test(349): the headline assertion reads the bound that stops the read, not a substring it shares`
- `305143484 feat(349): a plan can leave the process and be certified on the way back`

### History: `crates/shapes/src/schema_surface.rs`

- `065790157 Restore SCC inference and numeric backend admission`
- `0088bc920 Make datatype ranges and OWL schema coverage exact`
- `c170fce26 fix(shapes)!: every OWL property takes a node or a checked literal (OWL 2 Full floor)`
- `00f20cc81 fix(shapes): the complement of a restriction every individual meets admits no instance`
- `0e07eb3da fix(shapes): an empty qualifier bounds nothing, and an empty range states its cell exactly`
- `fe6916280 fix(shapes)!: read object-property data ranges by OWL 2 Full, and project owl:Nothing as false`
- `5526b23a2 fix(shapes): widen class fillers too, report widened and self-restricted cells as approximations`
- `2c78b616e fix(shapes): cross-kind values are a property-level fact, and cross-kind hasValue and hasSelf are read`
- `97617c43e fix(shapes): an object property restricted to a data range takes its literals`
- `c90227340 fix(shapes): judge string facets exactly, read cross-kind ranges, empty unions`

### History: `crates/rdf-core/src/query_scope/counters.rs`

- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`

## Identifiers (files containing each at HEAD)

- `the_unit_test_root_is_the_directory_cargo_names_for_integration_tests`: 1
- `realpath`: 3
- `check_plan_matches_relations`: 11
- `NativeSparqlEngine::admit`: 2
- `items_after_test_module`: 0
- `ConstraintCounts`: 5
- `ScopeError`: 9
- `ScopeCounters`: 4
- `ghprsq`: 0
- `rdf.projections`: 2
- `test_compile_emits_the_sparql_each_stratum_runs`: 1
- `test_compile_says_what_a_plan_costs_without_running_it`: 1
- `producer_call`: 1
- `workspace`: 1053
- `open_admitted`: 12
- `PropertyFunction`: 212
- `E0053`: 3
- `E0308`: 1
- `doc`: 2378
- `pytest`: 88
- `conformance`: 490
- `_producer_call`: 1
- `None`: 1981
- `eval_evaluated_with`: 2
- `SolutionSeq`: 24
- `SharedSchema`: 12
- `RowBag`: 8
- `BagStorage`: 1
- `RowEvidence`: 28
- `Evaluated`: 40
- `Truncation`: 29
- `queryGovernedAsync`: 19
- `eval_application`: 1
- `finish_left_join`: 1
- `linked.filter`: 2
- `exists_prepared`: 2
- `eval_correlated`: 10

## Workspace packages named

- `purrdf`
- `purrdf-testkit`

## URLs

- https://github.com/Blackcat-Informatics/purrdf/pull/532
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017985
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017929
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017965
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067
- https://github.com/Blackcat-Informatics/purrdf/tree/b6df69fdffd63ffdd8f3f35f9472e2317e4a302d
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38182392831
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017886
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595018240
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595018272
- https://github.com/Blackcat-Informatics/purrdf/actions/runs/38181556067/job/114595017943

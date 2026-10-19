# Referenced artifacts — issue #494

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/sparql-eval/src/clock.rs:1-35` | yes | repo | file | 54 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `engine.rs:1360` | NO | repo | — | — | — |
| `eval.rs:996` | NO | repo | — | — | — |
| `eval.rs:1135-1143` | NO | repo | — | — | — |
| `docs/design/purrdf-backend-contract.md:569+` | NO | repo | — | — | — |
| `crates/rdf-core/src/ir/paged/stack.rs` | yes | repo | file | 1206 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `stack.rs:335` | NO | repo | — | — | — |
| `paged/provider.rs:35` | NO | repo | — | — | — |
| `engine.rs` | NO | repo | — | — | — |
| `docs/design/QUERY-AT-SCALE.md` | NO | not here; exists in sibling(s): katamari | — | — | — |
| `crates/sparql-eval/src/clock.rs` | yes | repo | file | 54 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `docs/design/purrdf-backend-contract.md` | yes | repo | file | 1026 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `paged/provider.rs` | NO | repo | — | — | — |

### History: `crates/sparql-eval/src/clock.rs:1-35`

- `271733e76 Remove js-sys through direct wasm host imports`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `71c6b0139 feat(sparql-eval): caller-set query governors with a host-supplied stop signal`
- `be5981e1e fix(sparql): NOW() is the wall clock and RAND() is real entropy — by default, everywhere`

### History: `crates/rdf-core/src/ir/paged/stack.rs`

- `cef34c8aa Preserve authored ordinary rows across sealed stack layers`
- `1a2e326a7 Expose sound statistics for chronological stack queries`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `a0f8a7ed8 rdf: bound paged stack metadata probes`
- `dd9109995 core: layer sealed paged generations with pinned mutable snapshots`

### History: `crates/sparql-eval/src/clock.rs`

- `271733e76 Remove js-sys through direct wasm host imports`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `71c6b0139 feat(sparql-eval): caller-set query governors with a host-supplied stop signal`
- `be5981e1e fix(sparql): NOW() is the wall clock and RAND() is real entropy — by default, everywhere`

### History: `docs/design/purrdf-backend-contract.md`

- `cef34c8aa Preserve authored ordinary rows across sealed stack layers`
- `1a2e326a7 Expose sound statistics for chronological stack queries`
- `7c5a41cc2 Estimate structural RDF patterns across every statement stream`
- `2bb0691b2 Seal paged predicate statistics in the shared dictionary`
- `0f5137046 Record snapshot statistics and exactness in query explanations`
- `d0f544f49 perf(rdf): index structural reifiers and annotation values`
- `4e75877ec Fix triple carrier review boundaries and shared artifact admission`
- `525b3cf1b Add a graph-free RDF carrier and rule execution`
- `eb43e0b54 Preserve native storage diagnostics across language hosts`
- `15bc79027 Propagate typed capacity failures through workspace consumers`

## Identifiers (files containing each at HEAD)

- `PagedStack`: 13
- `main`: 2080
- `EvalCtx::with_now`: 0
- `wall_clock_now`: 5
- `PageGeneration`: 17
- `StaleGeneration`: 9
- `QueryOptions`: 204
- `FILTER`: 1421
- `with_now`: 2

## Workspace packages named

- `purrdf`

## URLs

None.

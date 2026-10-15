# Referenced artifacts — issue #492

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/dataset.rs:100-115` | yes | repo | file | 4498 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `dataset.rs:462-491` | NO | repo | — | — | — |
| `crates/datalog/src/id.rs:196-200` | yes | repo | file | 519 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/datalog/src/plan.rs:397-400` | yes | repo | file | 2437 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/ir/canon.rs` | yes | repo | file | 5216 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/text/src/ranking.rs:19-23` | yes | repo | file | 807 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `ir/pack/triples.rs:12-18` | NO | repo | — | — | — |
| `crates/rdf-core/src/ir/dataset.rs` | yes | repo | file | 4498 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/datalog/src/id.rs` | yes | repo | file | 519 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/datalog/src/plan.rs` | yes | repo | file | 2437 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/text/src/ranking.rs` | yes | repo | file | 807 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `ir/pack/triples.rs` | NO | repo | — | — | — |
| `/home/paudley/Active/katamari/docs/adr/0038-rdf-1-2-claims-one-graph-derived-views-and-refinements.md` | yes | absolute | file | 640 | SPDX: AGPL-3.0-only |

### History: `crates/rdf-core/src/ir/dataset.rs:100-115`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `4e484ce24 fix(core): make the snapshot charge exact for composite literals and tags`
- `20614b654 Restore the agreement-rule doc to agreed_content_addressing`
- `485c666a3 Keep live row counts for touched graphs instead of scanning at publication`
- `bb61bb389 Materialize delta and composite views under their sources' configuration`
- `bf095b0f7 Keep a configured derivation predicate the dataset never interned`
- `eb0ed724f Withdraw named graphs that an Update empties`

### History: `crates/datalog/src/id.rs:196-200`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `bf1282f00 refactor: one body per job across the query, reasoning and index engines`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `94f9b0b84 build(toolchain): pin the dated nightly-2026-09-02 and make the workspace clean under it (#231)`
- `9a659b708 feat(entail)!: certain answers by the combined approach, over a ported SLG-WFS resolver`
- `0055edd85 feat(datalog): add checkable proof terms and a contract hash`
- `7623c4bf8 fix(datalog)!: carry the predicate as data so meta-rules are expressible`
- `8c591cf2f feat(datalog): port the physical primitives — branded ids, arena, bitset, binding patterns`

### History: `crates/datalog/src/plan.rs:397-400`

- `10316f2c1 Correct typed interner and Datalog documentation links`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `e0df2247b Add sourced never-derive admission and pinned sentence boundaries`
- `4ea5a0bc3 fix(datalog): prioritize variable connectivity in rule plans`
- `1254206a7 refactor: the last duplicates the completion audit found`
- `c6573dcc5 feat(shapes,datalog)!: one rules engine: SHACL 1.2 rules on purrdf-datalog`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `345272965 chore: remove unused clones and algebra dependency`
- `94f9b0b84 build(toolchain): pin the dated nightly-2026-09-02 and make the workspace clean under it (#231)`
- `7623c4bf8 fix(datalog)!: carry the predicate as data so meta-rules are expressible`

### History: `crates/rdf-core/src/ir/canon.rs`

- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `954871c1c Add complete-state dataset digest with global blank canonicalization`
- `04e516a1a feat!: establish v3 storage and distribution contracts`
- `b8b661d2a refactor: one body per job across the foundation crates`
- `495b47b23 refactor(core,sparql,entail): write terms, escape literals, walk lists and read XML through the single homes`
- `68c1b96f8 feat(lex): one XML reader, literal/IRI escapers, term syntax and Crockford; strict list walker and SCC index in core`
- `13ba3e5ba chore: remove references to external projects the workspace no longer uses (#366)`
- `117a86061 refactor!: one W3C vocabulary home — purrdf_iri::vocab and purrdf_xsd::datatype replace ~830 retyped IRI literals (#366)`
- `a557fd676 refactor!: one hex implementation — purrdf_hash::hex, with Digest32 for every 32-byte digest identity (#366)`
- `b7a369525 test(hash): frozen hex differential — every copy against all bytes at every length, both cases (#366)`

### History: `crates/text/src/ranking.rs:19-23`

- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `959155521 text: qualify sparse corpus normalization and document its law`
- `c7dac8b01 text: support exact per-field corpus populations`
- `4cf50c259 refactor: one framing, byte-cursor and alignment implementation each — purrdf_hash::frame, purrdf_core::bytes (#366)`
- `841b456d8 Complete native BLAKE3 and optimize hashing for PurRDF workloads`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `394a37ca0 text: bind corpus law and cover zero-score matches`
- `51da1a29e text: add bounded versioned BM25F ranking profiles`

### History: `crates/rdf-core/src/ir/dataset.rs`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `4e484ce24 fix(core): make the snapshot charge exact for composite literals and tags`
- `20614b654 Restore the agreement-rule doc to agreed_content_addressing`
- `485c666a3 Keep live row counts for touched graphs instead of scanning at publication`
- `bb61bb389 Materialize delta and composite views under their sources' configuration`
- `bf095b0f7 Keep a configured derivation predicate the dataset never interned`
- `eb0ed724f Withdraw named graphs that an Update empties`

### History: `crates/datalog/src/id.rs`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `bf1282f00 refactor: one body per job across the query, reasoning and index engines`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `94f9b0b84 build(toolchain): pin the dated nightly-2026-09-02 and make the workspace clean under it (#231)`
- `9a659b708 feat(entail)!: certain answers by the combined approach, over a ported SLG-WFS resolver`
- `0055edd85 feat(datalog): add checkable proof terms and a contract hash`
- `7623c4bf8 fix(datalog)!: carry the predicate as data so meta-rules are expressible`
- `8c591cf2f feat(datalog): port the physical primitives — branded ids, arena, bitset, binding patterns`

### History: `crates/datalog/src/plan.rs`

- `10316f2c1 Correct typed interner and Datalog documentation links`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `e0df2247b Add sourced never-derive admission and pinned sentence boundaries`
- `4ea5a0bc3 fix(datalog): prioritize variable connectivity in rule plans`
- `1254206a7 refactor: the last duplicates the completion audit found`
- `c6573dcc5 feat(shapes,datalog)!: one rules engine: SHACL 1.2 rules on purrdf-datalog`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `345272965 chore: remove unused clones and algebra dependency`
- `94f9b0b84 build(toolchain): pin the dated nightly-2026-09-02 and make the workspace clean under it (#231)`
- `7623c4bf8 fix(datalog)!: carry the predicate as data so meta-rules are expressible`

### History: `crates/text/src/ranking.rs`

- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `959155521 text: qualify sparse corpus normalization and document its law`
- `c7dac8b01 text: support exact per-field corpus populations`
- `4cf50c259 refactor: one framing, byte-cursor and alignment implementation each — purrdf_hash::frame, purrdf_core::bytes (#366)`
- `841b456d8 Complete native BLAKE3 and optimize hashing for PurRDF workloads`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `394a37ca0 text: bind corpus law and cover zero-score matches`
- `51da1a29e text: add bounded versioned BM25F ranking profiles`

### History: `/home/paudley/Active/katamari/docs/adr/0038-rdf-1-2-claims-one-graph-derived-views-and-refinements.md`

- `7b275be61 Follow-on ADR amendments recorded without re-approval; "on-site" struck`
- `f39821a6a Patrick's 2026-10-11 rulings: memetics' start grouped, backups are nodes, suppression replaces`
- `a72c74321 Amend ADRs and docs for 2026-10-11 rulings: observation record and schema facets removed outright, checkpoint read in bounded pieces, nothing excluded from the index, indexes declared in profiles, statistics untouched by purge`
- `cfd2babe1 Carry ADR-0047 47.10 through the ADR set and the code; refuse deleted content on read`
- `7f5f0c6da Accept ADR-0033, 0034, 0045, 0046, 0047, 0048 (Patrick, 2026-10-09: "All approved")`
- `2b4e872ab ADR-0038 38.10 and ADR-0029 amended; "derived" removed where it named claims`
- `c964b8ce6 docs: an unstated modal force is unspecified; ADR-0038 38.26 amended`
- `1d6a54467 docs: profiles are compositions; named compositions in ADRs`
- `25cdc7c6e docs: ADR-0014 amended -- purging an object never implies purging its referrers`
- `e9919a46a docs: OQ7 resolved -- a person is a cluster; the 2026-09-15 purge ruling retracted`

## Identifiers (files containing each at HEAD)

- `main`: 2069
- `ReifierRow`: 21
- `AnnotationRow`: 16
- `sips_order`: 1
- `INDEX_CORPUS_PROFILE_ID`: 3
- `TripleDataset`: 0
- `DatasetView`: 294
- `GraphMatch::Default`: 57
- `SPO`: 30
- `POS`: 818
- `OSP`: 18
- `try_canonicalize_flat_view`: 23
- `try_flat_digest_view`: 6
- `da57b2500`: 0

## Workspace packages named

None.

## URLs

None.

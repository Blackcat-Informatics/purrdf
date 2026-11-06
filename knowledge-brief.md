# Combined repository briefing

Repository: `github.com/blackcat-informatics/purrdf`
Checkout commit: `5d25c3a7d8e76064aa716e3e168d9f443aba7b13`

## Forge evidence

{
  "state": "available",
  "issue": 1107,
  "title": "v4 API: retain native GTS composition and projection failures",
  "artifacts": {
    "issue": "/home/paudley/Active/purrdf/.worktrees/1107-v4-api-retain-native-gts-composition/.stage/v4-api-retain-native-gts-composition/issue.md",
    "prior_art": "/home/paudley/Active/purrdf/.worktrees/1107-v4-api-retain-native-gts-composition/.stage/v4-api-retain-native-gts-composition/prior-art.md"
  },
  "coverage": [
    "issue #1107 + 0 comment(s) via github",
    "link crawl: 45 issue(s) within 2 hop(s) of `#N` references",
    "forge search on ['retain', 'native', 'composition']: 14 other item(s)",
    "GhpRsq-Defect-Class trailers: 1 distinct class(es) in the last 200 commits",
    "prior attempts: 0 commit(s) citing #1107 (second parents walked), 0 other branch(es) named for it",
    "PRs citing #1107: 0",
    "requirement candidates: 6 line(s); 0 comment(s) matched no rule",
    "referenced artifacts: 0 path(s) (0 on disk), 3 identifier(s), 0 workspace package(s), 4 URL(s)"
  ]
}

## Code relationships

State: current
Indexed commit: `5d25c3a7d8e76064aa716e3e168d9f443aba7b13`
Coverage: {"scope": ["all tracked files"], "excluded_links": [], "limitations": ["AST extraction; semantic document interpretation is not performed"], "warnings": ["  warning: 1 file(s) had syntax errors and may be partially extracted: crates/rdf-capi/include/purrdf.h (error region at line 1, 27 symbol(s) extracted)", "  warning: 164 code file(s) yielded no symbols (0.49 MB) — a data file, or source this extractor models no symbols for: bindings/python-rdflib-shadow/pyproject.toml, bindings/python/Cargo.toml, bindings/python/pyproject.toml, crates/alloc-probe/Cargo.toml, crates/bench/Cargo.toml (+159 more)"], "nodes": 123487, "edges": 369114}

Graph: /home/paudley/stage/agents/integration/data/graphs/9ce6381dd000fe38ec9582f9/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/graph.json (123487 nodes) | Traversal: BFS depth=2 | Start: ['failures()', 'API', 'GTS exports through the Rust-owned native v1 five-table projection. SQLite and…', 'gts()', 'Projection', 'retain()', 'Native', 'composition_exclusions_agree_with_the_derived_property()', '`h3` v4.0 grid-family contract'] | 438 nodes found

[!] TRUNCATED: showing 67 of 438 nodes (~1800-token budget). The answer may be among the 371 cut nodes — raise the token budget to 11832 to get all 438 (CLI: --budget 11832, MCP: token_budget=11832) or narrow the query (e.g. context_filter=['call'], or get_node for a specific symbol).

NODE failures() [src=scripts/check-layers.py loc=L197 community=]
NODE API [src=crates/rdf-wasm/README.md loc=L52 community=]
NODE GTS exports through the Rust-owned native v1 five-table projection. SQLite and… [src=bindings/python/python/src/purrdf/_gts_export.py loc=L3 community=]
NODE gts() [src=crates/envelope-probe/src/lib.rs loc=L615 community=]
NODE Projection [src=crates/rdf/src/native_codecs/jsonld/carrier/projection.rs loc=L40 community=]
NODE retain() [src=crates/rdf-core/src/ir/canon/memo.rs loc=L319 community=]
NODE Native [src=crates/rdf/src/native_codecs/source_format.rs loc=L56 community=]
NODE composition_exclusions_agree_with_the_derived_property() [src=crates/lex/tests/unicode_normalization.rs loc=L240 community=]
NODE `h3` v4.0 grid-family contract [src=docs/design/h3-v4-grid-contract.md loc=L4 community=]
NODE StorageError [src=crates/lex/src/allocation.rs loc=L33 community=]
NODE Memory [src=crates/lex/src/allocation.rs loc=L474 community=]
NODE SourceFormat [src=crates/rdf/src/native_codecs/source_format.rs loc=L54 community=]
NODE envelope-probe/src/lib.rs [src=crates/envelope-probe/src/lib.rs loc=L1 community=]
NODE WorkList [src=crates/lex/src/walk.rs loc=L120 community=]
NODE QueryEngine [src=crates/rdf-wasm/js/index.d.ts loc=L1255 community=]
NODE Ceiling [src=crates/lex/src/allocation.rs loc=L378 community=]
NODE import_gts_events() [src=crates/rdf/src/gts_import_sink.rs loc=L531 community=]
NODE memo.rs [src=crates/rdf-core/src/ir/canon/memo.rs loc=L1 community=]
NODE unicode_normalization.rs [src=crates/lex/tests/unicode_normalization.rs loc=L1 community=]
NODE carrier/projection.rs [src=crates/rdf/src/native_codecs/jsonld/carrier/projection.rs loc=L1 community=]
NODE check-layers.py [src=scripts/check-layers.py loc=L1 community=]
NODE DataFactory [src=crates/rdf-wasm/js/index.d.ts loc=L708 community=]
NODE q() [src=crates/rdf-wasm/js/tests/cloudflare.test.mjs loc=L240 community=]
NODE Task [src=crates/rdf/src/native_codecs/jsonld/carrier/projection.rs loc=L23 community=]
NODE .completed() [src=crates/rdf-core/src/ir/canon/memo.rs loc=L426 community=]
NODE _gts_export.py [src=bindings/python/python/src/purrdf/_gts_export.py loc=L1 community=]
NODE diagnostics() [src=crates/rdf-wasm/js/tests/workerd/run-worker-recipe.mjs loc=L136 community=]
NODE S [src= loc= community=]
NODE Sink [src=crates/rdf-wasm/js/index.d.ts loc=L2125 community=]
NODE run() [src=crates/envelope-probe/src/lib.rs loc=L271 community=]
NODE Profile [src=crates/envelope-probe/src/lib.rs loc=L67 community=]
NODE unicode_data() [src=crates/lex/examples/gen_unicode_tables.rs loc=L206 community=]
NODE operation() [src=crates/rdf-core/src/ir/segmented/query/changes/directory.rs loc=L377 community=]
NODE Metric [src=crates/envelope-probe/src/lib.rs loc=L60 community=]
NODE keystone_base() [src=crates/rdf/src/gts_fixtures.rs loc=L65 community=]
NODE CompactionScope [src=crates/rdf/src/native_codecs/jsonld/carrier.rs loc=L191 community=]
NODE .query() [src=crates/rdf-wasm/js/index.d.ts loc=L828 community=]
NODE purrdf (wasm) — RDF 1.2 in the browser & Node, the RDF/JS way [src=crates/rdf-wasm/README.md loc=L6 community=]
NODE Layers [src=scripts/check-layers.py loc=L80 community=]
NODE main() [src=scripts/check-layers.py loc=L515 community=]
NODE shape() [src=bindings/python/tests/test_top_level_exports.py loc=L149 community=]
NODE CancellationToken [src=crates/rdf-wasm/js/index.d.ts loc=L2115 community=]
NODE cycle() [src=scripts/check-layers.py loc=L170 community=]
NODE .release() [src=crates/rdf-core/src/ir/canon/memo.rs loc=L485 community=]
NODE Quota [src=crates/rdf-core/src/ir/canon/memo.rs loc=L284 community=]
NODE .updateGovernedAsync() [src=crates/rdf-wasm/js/index.d.ts loc=L1522 community=]
NODE ShaclEntailment [src=crates/rdf-wasm/js/index.d.ts loc=L2748 community=]
NODE ShaclNodeExprOutcome [src=crates/rdf-wasm/js/index.d.ts loc=L2926 community=]
NODE ShaclRulesInference [src=crates/rdf-wasm/js/index.d.ts loc=L2769 community=]
NODE scalar() [src=crates/lex/tests/yaml_scalar_keys.rs loc=L23 community=]
NODE .queryRawConfigured() [src=crates/rdf-wasm/js/index.d.ts loc=L1357 community=]
NODE .queryRawWithContext() [src=crates/rdf-wasm/js/index.d.ts loc=L1367 community=]
NODE ShaclDiagnostic [src=crates/rdf-wasm/js/index.d.ts loc=L2914 community=]
NODE datasetToStream() [src=crates/rdf-wasm/js/index.mjs loc=L1691 community=]
NODE streamToDataset() [src=crates/rdf-wasm/js/index.mjs loc=L1703 community=]
NODE JsonValue [src= loc= community=]
NODE .blankNode() [src=crates/rdf-wasm/js/index.d.ts loc=L711 community=]
NODE .defaultGraph() [src=crates/rdf-wasm/js/index.d.ts loc=L720 community=]
NODE .fromQuad() [src=crates/rdf-wasm/js/index.d.ts loc=L729 community=]
NODE .fromTerm() [src=crates/rdf-wasm/js/index.d.ts loc=L728 community=]
NODE .namedNode() [src=crates/rdf-wasm/js/index.d.ts loc=L710 community=]
NODE .quad() [src=crates/rdf-wasm/js/index.d.ts loc=L721 community=]
NODE .typedLiteral() [src=crates/rdf-wasm/js/index.d.ts loc=L713 community=]
NODE .variable() [src=crates/rdf-wasm/js/index.d.ts loc=L719 community=]
NODE .explainQuery() [src=crates/rdf-wasm/js/index.d.ts loc=L1356 community=]
NODE .directionalLiteral() [src=crates/rdf-wasm/js/index.d.ts loc=L714 community=]
NODE .quotedTriple() [src=crates/rdf-wasm/js/index.d.ts loc=L727 community=]
... (truncated — 371 more nodes cut by ~1800-token budget. Narrow with context_filter=['call'] or use get_node for a specific symbol)

## Repository memory

State: available
Bank: `purrdf`

- The PR’s logical usage charge excludes ordinary selected metadata (which has per-blob ceilings) and dataset/index/allocator/frame/codec storage; it makes no physical-process quota or streaming claim. Retagging refusals refunds stale payload, explicit digest selection preserves authenticated bytes, and terminal errors retain native causes plus typed budget/provenance context. Existing Arc/CBOR ownership was a bounded scope choice, not a compatibility requirement, so pre-v4 APIs may break; issues 895/883 retain separate streaming and Python/projection requirements. | When: October 30, 2026 | These are important accounting boundaries and scope limitations of PR 898.
  Source document: `selected-gts-blobs-898-delivered-20261030`
  Metadata: {"source": "PR 898; crates/rdf/src/gts_import_blobs.rs; merge 8489adf9b8ebe91acf3bdacc98403aedfa19f0f7"}

- Blackcat-Informatics/purrdf PR #1138 changed public loss-ledger APIs to preserve native failures: completeness checks return owned, ordered missing/unexpected code lists, and built-in and explicit-profile soundness checks return native errors. Projection, schema-import, and LinkML callers retain those causes through structured inspection and the standard error chain; the migration guide documents the signature changes. The changes close #1117. | When: November 6, 2026 | Allows callers to inspect original loss-ledger refusals rather than receiving formatted strings.
  Source document: `merge-5d25c3a7d8e76064aa716e3e168d9f443aba7b13`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/1138", "commit": "5d25c3a7d8e76064aa716e3e168d9f443aba7b13", "source": "git:github.com/blackcat-informatics/purrdf@5d25c3a7d8e76064aa716e3e168d9f443aba7b13", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- PR #939 changes GTS codec failures to retain original native Zstandard and allocator causes, with distinct encoder/decoder dictionary variants and allocation capacity plus `TryReserveError`; it removes six erasing factories while preserving existing behavior. Validation passed, the merge tree matches the qualified source tree byte for byte, and the change closes #937. | When: November 1, 2026 | Preserves structured error causes and inspection details without changing successful encoding or decoding behavior.
  Source document: `merge-edd601b56f4edca437c4432452953e3201006aaa`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/939", "commit": "edd601b56f4edca437c4432452953e3201006aaa", "source": "git:github.com/blackcat-informatics/purrdf@edd601b56f4edca437c4432452953e3201006aaa", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- PurRDF PR 898’s logical usage charge excludes ordinary selected metadata with per-blob ceilings and dataset, index, allocator, frame, and codec storage; it makes no physical-process quota or streaming claim. Retagging refusals refunds stale payload, explicit digest selection preserves authenticated bytes, and terminal errors retain native causes plus typed budget/provenance context. Existing Arc/CBOR ownership was a bounded scope choice rather than a compatibility requirement, so pre-v4 APIs may break; issues 895/883 retain separate streaming and Python/projection requirements.
  Source document: `None`
  Metadata: {}

- PR #943 (commit 23a2b710e9801f5bcd051efe1e48147468db8afc) preserves original GTS FrameDecoderError causes through purrdf_hash::error::io_error for transport reset and block decoding, allowing buffered and streaming consumers to inspect native fields and source chains after input disposal; source-read failures retain precedence.
  Source document: `None`
  Metadata: {}

- PR #939 (commit edd601b56f4edca437c4432452953e3201006aaa) changed GTS Zstandard codec failures to preserve original native Zstandard and allocator causes, with distinct dictionary-failure variants and allocation capacity/error details; all six erasing factories were removed while existing failure behavior and successful encode/decode paths remained unchanged. | When: November 1, 2026 | To retain structured native error causes for inspection instead of rendering them into strings.
  Source document: `merge-edd601b56f4edca437c4432452953e3201006aaa`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/939", "commit": "edd601b56f4edca437c4432452953e3201006aaa", "source": "git:github.com/blackcat-informatics/purrdf@edd601b56f4edca437c4432452953e3201006aaa", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- PR #1122 in Blackcat-Informatics/purrdf changed typed USTAR refusals to preserve typed offsets, sizes, original size-field bytes, and native UTF-8 causes through archive projections, including structured inspection and error sources. This intentionally changed the pre-v4 archive error API; validation passed for the listed Clippy checks, eleven native USTAR cases, projection transport regression, source review, prose checks, and signed commit hooks, but hosted CI had not run. | When: November 5, 2026 | Closes issue #1108; preserves original archive errors through projection packaging.
  Source document: `merge-9fa1f4ce83b4aca745473645b590c63e42d1f9d4`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/1122", "commit": "9fa1f4ce83b4aca745473645b590c63e42d1f9d4", "source": "git:github.com/blackcat-informatics/purrdf@9fa1f4ce83b4aca745473645b590c63e42d1f9d4", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- The implementation integrated the frozen visual-hashing 1.0.0 rendering law and alphabet into Lex using its native BLAKE3 XOF, replaced GTS key-rendering callers, removed the external package and duplicate hash implementation, re-exported the native renderer from the core API, and added a CLI `fingerprint` command with explicit storage and work allowances. Existing GTS public-key fingerprints retain their original byte law. | When: November 3, 2026 | This makes visual fingerprinting available through native APIs and release consumers while preserving compatibility for existing GTS fingerprints.
  Source document: `merge-b60e5a0864f002e1e220e2e4aa7ce0c471c52303`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/1000", "commit": "b60e5a0864f002e1e220e2e4aa7ce0c471c52303", "source": "git:github.com/blackcat-informatics/purrdf@b60e5a0864f002e1e220e2e4aa7ce0c471c52303", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- PR #939 (commit edd601b56f4edca437c4432452953e3201006aaa) changed GTS Zstandard codec failures to preserve original native Zstandard and allocator causes, including distinct encoder/decoder dictionary-failure variants and allocation capacity plus `TryReserveError` details. It removed six erasing factories while preserving existing failure behavior and successful encoding and decoding paths.
  Source document: `None`
  Metadata: {}

- PurRDF PR 1019 / issue 1016 was merged through ghprsq as 420d1ed3c7798b3bc9064ccb50448b72ea9a1021; result tree b17fd82febc92796d4fea75730243cabeff0ccec matched the qualified candidate. Native YAML-LD Basic uses one consuming Lex projection and a JSON-LD value engine, validates the full stream before constructing selected values, and retains native positioned causes through purrdf_lex::allocation::retain_native_cause, re-exported by GTS. | When: November 4, 2026 | Verified delivery milestone for PurRDF PR 1019.
  Source document: `native-yamlld-basic-landed-20261104`
  Metadata: {}

- The native processing unit for `purrdf` was completed: shared retention moved to Lex allocation, the GTS API re-exports it, and YAML calls the lower-level implementation. Affected Lex/GTS/RDF release Clippy, independent reviews, formatting and diff checks, and normal signed hooks passed; current-head hosted CI was still pending. | When: November 4, 2026
  Source document: `merge-420d1ed3c7798b3bc9064ccb50448b72ea9a1021`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/1019", "commit": "420d1ed3c7798b3bc9064ccb50448b72ea9a1021", "source": "git:github.com/blackcat-informatics/purrdf@420d1ed3c7798b3bc9064ccb50448b72ea9a1021", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

- PR #907 added explicit Python GTS blob-export policies: callers must provide all five `GtsBlobLimits` fields or select `"unbounded"`; native exports return an always-present report, while SQLite, DuckDB, and Parquet exports return frozen `GtsExportResult(paths, report)` values. Reports retain typed causes, digest proof, frames, limits, usage, and located losses; specified blob omissions are reported, while storage, structural, integrity, authentication, malformed-payload, and non-blob frame failures remain hard errors. | When: October 30, 2026 | Make blob-export policy explicit and preserve admission evidence in export reports.
  Source document: `merge-9252d9b7b42b6d3c59690e73855eefb267601710`
  Metadata: {"pr": "https://github.com/Blackcat-Informatics/purrdf/pull/907", "commit": "9252d9b7b42b6d3c59690e73855eefb267601710", "source": "git:github.com/blackcat-informatics/purrdf@9252d9b7b42b6d3c59690e73855eefb267601710", "evidence": "committed-change-record", "repository": "github.com/blackcat-informatics/purrdf"}

## Coverage and pending work

[
  {
    "kind": "graph",
    "state": "done",
    "count": 131
  },
  {
    "kind": "graph",
    "state": "superseded",
    "count": 1
  },
  {
    "kind": "retain",
    "state": "done",
    "count": 132
  }
]

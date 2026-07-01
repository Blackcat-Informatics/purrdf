# Codex Task List

This is the working task list for cutover readiness and performance work. Release
crate constraints are load-bearing:

- no Cargo feature gates or optional first-party modes
- no capability removal
- wasm-compatible core crates only
- non-wasm boundaries stay outside the release Rust core: conformance, C API, and
  the PyO3 wrapper
- keep GTS rsyncable zstd support

## Optimization Backlog

- [x] PERF-000: Remove low-hanging GTS authoring copies: borrowed uncompressed
  tar byte input, direct files-profile v2 writing, owned writer byte handoff,
  moved transformed raw frame payloads, fixed tar zero-block padding, rsyncable
  zstd output preallocation, and PAX body preallocation.
- [x] PERF-004a: Add a GTS Criterion authoring benchmark for rsyncable zstd and
  deterministic snapshot emission.
- [x] PERF-001: Add seekable tar import indexing/range replay and borrowed
  in-memory entry payloads while preserving portable USTAR/PAX behavior and
  files-profile v2 round trips.
- [x] PERF-002: Stream files-profile v2 GTS blob frames during tar import/export
  instead of cloning every regular-file payload into writer-owned buffers.
- [x] PERF-003: Add GTS writer scratch/buffer improvements for canonical CBOR
  append, transform sources, index payload construction, and owned writer byte
  handoff; prove byte-stability with existing tests.
- [x] PERF-004: Add benchmark coverage for GTS rsyncable zstd/snapshot authoring,
  RDF native codec parse/serialize, IR layout/mutable paths, SHACL validation,
  and SPARQL planner/EXISTS hot paths.
- [x] PERF-005: Audit and optimize RDF parser/serializer hot paths for avoidable
  `String` materialization and batch interning opportunities.
- [x] PERF-006: Benchmark-gate IR quad layout and predicate/object adjacency work
  before changing storage shape.
- [x] PERF-007: Add SPARQL expression/path caches where repeated work is visible;
  retain the compact solution-term join behavior.
- [x] PERF-008: Add wasm build checks to CI/release for every publishable Rust
  crate plus a real `purrdf-wasm` artifact smoke check.
- [x] PERF-009: Restore/generated-artifact gate for loss matrices and the SSSOM
  corpus anchor so full workspace tests are reproducible.
- [x] PERF-010: Keep PyPI validation as a separate release lane while preserving
  the PyO3 wrapper boundary around the main Rust crates.

## Release Readiness

- [x] REL-001: Re-run `cargo metadata` checks for zero first-party feature gates
  before publishing.
- [x] REL-002: Re-run wasm checks for all publishable Rust crates before publishing.
- [x] REL-003: Re-run packaging checks excluding only the explicit non-core
  boundary crates.
- [x] REL-004: Document the trusted publisher settings needed for the GitHub
  release workflow after the first crates.io publish.

## Publish Handoff

- [ ] PUB-001: Commit the release source so the bootstrap publish script can run
  without `ALLOW_DIRTY=true`.
- [ ] PUB-002: Bootstrap-publish the first `0.1.0` crates.io records with
  `CARGO_TOKEN`, then configure crates.io Trusted Publisher entries.

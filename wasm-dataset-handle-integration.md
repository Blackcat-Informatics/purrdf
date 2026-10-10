# WASM frozen graph ownership closure

`wasm-dataset-handle-owner-draft.patch` is an ordinary four-file current-source
delta. It closes all five WASM type errors captured by correction 9. Matching
postimages, preimages and SHA-256 identities are adjacent in Stage.

`Dataset::from_frozen(DatasetHandle)` moves the existing immutable graph owner
into the mutable base. The paired core `MutableDataset` base migration belongs
to the root's C/Python/core unit and is required for integration. No WASM caller
uses `MutableDataset::base()` as an `Arc` (the new lifetime fixture only borrows
its `RdfDataset`). This packet does not modify core files.

Both graph-producing adapters (`query_result_from_sparql` and
`graph_result_from_sparql`) now use that door with the original handle; neither
extracts an `Arc` nor copies a graph. `QueryResult::take_dataset` preserves that
same owner when moving the host object out of its wrapper. Governed, partial,
synchronous and asynchronous result consumers already route through these
original adapters.

The configured graph serializer and query default/refusal/serializer helpers
borrow `&RdfDataset`, so both resident `Arc` owners and native `DatasetHandle`
owners use the same body. Resident RDF parsing, Dataset snapshots, event sinks
and projection lifting explicitly move their existing `Arc` through `.into()`;
their allocation and content laws are unchanged. UPDATE replacement remains a
resident `Arc` input, moved into the same core base constructor, without a native
owner extraction route.

The new native fixture builds actual RDF buffers through the original `Memory`
doors, admits the exact native shared-control layout, and publishes its original
storage callback. It checks borrowed serialization, native shallow clones,
wrong-kind take, single-use graph transfer, original dataset pointer identity,
host serialization, and release only after the final host Dataset dies. It
executes both typed-wrapper and direct graph-result adapters.

Stage Rust formatting/syntax and ordinary patch dry-run: PASS. Compilation,
runtime, WASM build and hosted qualification: NOT RUN by this helper. The sole
writer must integrate with the paired core base unit before qualifying.

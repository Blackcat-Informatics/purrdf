# Deep blank-chain serializer contract

The issue requires actual100,000- and1,000,000-level blank-node chains to complete
natively and on wasm32, with matching output bytes for Turtle and TriG. Shallow
packed output remains compatible, permanent deep-key guard40 remains, handling
is deterministic, and an actual benchmark ships. Publication and closure remain
separate from local execution. The two issue comments preserve an unreviewed patch;
they do not establish a passed implementation. No governing ADR is cited.

Current structural kernel `purrdf_core::render_canonical_turtle` aliases
`turtle_render::render`, also re-exported by `purrdf::turtle_normalize::render`.
Its property/object/collection emitters recurse and build nested intermediate
Strings, so both stack growth and repeated copies are relevant. The native
`canonical_turtle` normalizer parses before invoking this kernel. Actual general
`serialize_dataset` Turtle/TriG calls instead build the original SerGraph and
route via LineCodec to ser_model. Those are distinct public routes; fixing the
packed renderer cannot by itself prove both general formats.

Structural key computation retains MAX_DEPTH40 for blanks. Contrary to the
suspected stale assumption, current frozen RdfDataset validation enforces the
actual events::MAX_TERM_NESTING_DEPTH16 for consecutive triple-term nesting.
Every key descent increments the same depth, including triple components; after
a blank reaches40 it stops, with at most a16-level direct quoted suffix. The
claimed bound must be documented using these actual premises. Shared/cyclic
blank graphs and quoted blanks still need explicit correctness fixtures.
`ground_sig` has a separate explicit8-level signature bound.

Required real-public acceptance is a shared Rust portable corpus for all deep
sizes and routes, ordinary default-thread stack, frozen full-output digest and
length, shallow bytes and depth-policy neighbors, row/statement fidelity and
TriG named-graph fidelity. The benchmark must invoke actual rendering at the
requested sizes under the existing native harness. Compilation, oversized stack,
ignored deep tests or small stand-ins cannot prove this issue.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# GTS Graph Transport

**GTS (Graph Transport Substrate)** is an ontology-independent binary
container and transport format for RDF 1.2 datasets and content-addressed
binary payloads. PurRDF hosts the reference Rust engine,
[`purrdf-gts`](https://docs.rs/purrdf-gts) (re-exported as `purrdf::gts`).

This chapter is a user-level tour. **The wire format itself is specified in
[`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md)**
— consult the spec for framing, fold semantics, registries, and conformance
classes; nothing here supersedes it.

## The container model, at a high level

A GTS file is a **CBOR Sequence of one or more append-only segments**. Each
segment is a deterministic CBOR header followed by deterministic CBOR frames
chained by **BLAKE3 content identifiers**. The logical dataset is obtained by
a **deterministic fold** over the segment sequence: quads, reifiers,
annotations, and binary blobs all become rows of the folded container graph.

Properties that fall out of this design:

- **Content-addressed and append-only** — history is never rewritten;
  suppression is itself an appended record. Multi-segment files compose by
  simple concatenation.
- **Partial readability, total reader** — the reader verifies the BLAKE3
  chain and folds what it can; undecodable frames (unknown codec, encrypted
  without a key) degrade to *opaque nodes* plus a diagnostic instead of
  aborting.
- **Binary payloads ride along** — the blobs a graph references travel in the
  same file, content-addressed like everything else.
- **RDF 1.2-native** — the spec formalizes the triple-term and `rdf:reifies`
  mapping, blank-node scoping, and multi-segment value union.

## Reading and writing from Rust

The container engine (`purrdf-gts`) owns the wire-format machinery — reader,
writer, fold, verify, COSE, trust policy:

```rust,ignore
use purrdf::gts::reader;

// Fold GTS bytes into the container graph model, verifying the BLAKE3 chain.
let graph = reader::read(&bytes, /* allow_segments */ true, /* expected_head */ None);

// The fold is total: quads, reifiers, annotations, and blobs are all rows,
// and anything undecodable is preserved as an opaque node plus a diagnostic.
println!("{} quads, {} blobs", graph.quads.len(), graph.blobs.len());
```

The writer authors frames and produces **byte-deterministic single-segment
snapshots** (`Writer::deterministic`) — the GTS writer is under the same
determinism invariant as every PurRDF serializer.

The `RdfDataset` import/export path lives one layer up: the umbrella crate's
`gts` module combines the container engine with the RDF-level adapter
(snapshot composition, content-chain verification), so RDF-facing GTS work
goes through `purrdf` directly.

## Signing and encryption

Frames can be signed and encrypted with **COSE**; OpenPGP-based checks and a
trust-policy layer are also part of the engine. All cryptography is
**pure Rust** — no C toolchain, threads, or syscall dependencies — which is
what keeps the whole engine wasm-friendly. An encrypted frame you cannot
decrypt is simply an opaque node in the fold: the container remains readable.

## Conformance vectors

GTS conformance is defined against a **frozen, language-neutral vector
corpus** (`vectors/` in the repository), shared **byte-exact** across the
sibling GTS engines in other languages. The vectors are never regenerated or
"fixed" in this repository — the format is governed in the
[`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) project,
alongside the specification and the other reference engines.

## Known limitation at the C ABI

The GTS star-layer round-trip of a dataset containing quoted triples /
reifier bindings currently fails through the kernel
`to_gts` → `read_graph` → `import_gts_graph` path (star-free round-trips are
lossless). See [Getting Started: C](getting-started/c.md#known-limitation).

## Related

- [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md)
  — the normative wire format (version 0.9-draft, wire-format major 1).
- [Slices, Mappings & Provenance](slices.md) — the RDF↔GTS loss ledger.
- [Getting Started: Python](getting-started/python.md#gts-relational-exports)
  — projecting a container into SQLite/DuckDB/Parquet.

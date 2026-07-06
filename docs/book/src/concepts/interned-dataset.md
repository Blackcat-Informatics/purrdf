<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# The Interned Dataset IR

Everything in PurRDF evaluates over one intermediate representation: an
immutable, value-interned RDF 1.2 dataset owned by the ring-fenced
[`purrdf-core`](https://docs.rs/purrdf-core) kernel.

## Terms are interned once

Every term — IRI, blank node, literal, triple term — is stored **once** in a
string arena and addressed by a copyable `TermId` (a niche-optimized
`NonZeroU32`). Quads are rows of four `TermId`s. That makes term equality a
single integer compare, keeps quads at a fixed small size, and means a term
that appears in a million quads costs its bytes exactly once.

Hot maps use fixed-key `ahash` — deterministic hashing is part of the
[byte-determinism discipline](codecs.md), not just a speed choice.

## Builder → freeze

The IR has a strict two-phase life cycle:

```rust,ignore
use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

// Intern terms once; quads are rows of copyable TermIds.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat");
let says = b.intern_iri("https://example.org/says");
let meow = b.intern_literal(RdfLiteral::simple("meow"));
b.push_quad(cat, says, meow, None);

// Freeze into the immutable, indexed dataset the engines evaluate over.
let ds = b.freeze().expect("well-formed dataset");
assert_eq!(ds.quad_count(), 1);
```

`RdfDatasetBuilder` is the mutable ingestion phase: intern terms, push quads,
attach reifiers and annotations. `freeze()` validates the structure and
produces an immutable `RdfDataset`: quad rows in `Box<[QuadRow]>` tables with
lazy ordinal permutation indexes (roughly 4 bytes per quad per axis). The
frozen dataset is what SPARQL, SHACL, ShEx, and entailment all read, through
the allocation-free `DatasetView` trait.

Freezing is also what makes concurrency simple: a frozen dataset is immutable,
so it can be shared and read from many threads (the C ABI exposes exactly this
as a `Send + Sync` handle).

## Copy-on-write mutation

"Immutable" does not mean "static". Mutation happens through a copy-on-write
delta over a frozen base: edits accumulate in a lightweight overlay, and the
result freezes into a new dataset without copying the untouched base rows or
re-interning shared terms. SPARQL UPDATE and the C ABI's mutable
`PurrdfGraph` handle both ride this path.

## What else lives in the kernel

Beyond the IR itself, `purrdf-core` owns:

- **`DatasetView`** — the static read trait every engine evaluates over.
- **Structured diagnostics** — typed `RdfDiagnostic`s with source locations
  (deliberately SARIF-free; the SARIF boundary is
  [`purrdf-validate`](https://docs.rs/purrdf-validate)).
- **RDFC-1.0** canonicalization, dataset diff, and isomorphism — see
  [Canonicalization & Diff](canonicalization.md).
- **Store and engine seams** — the narrow parser-ingress, serializer-egress,
  and `SparqlEngine` traits that adapters implement in sibling crates.
- **Provenance and the loss ledger** — a generic provenance sidecar and the
  machine-readable RDF↔GTS loss matrix, plus native FnO and SSSOM codecs
  (see [Slices, Mappings & Provenance](../slices.md)).

Text codecs are *not* in the kernel — parsing and serialization live one layer
up in [`purrdf-rdf`](https://docs.rs/purrdf-rdf). The split keeps the kernel
small and its invariants enforceable at the crate boundary: no oxigraph, no
PyO3 (a hygiene gate asserts the dependency tree), `wasm32`-clean, and a
file-IO-free IR layer.

## Why this design

The layout is chosen by measurement, not assertion: the criterion bench
`crates/rdf-core/benches/ir_layout.rs` compares array-of-structs,
struct-of-arrays, and predicate-adjacency layouts on allocation counts,
high-water memory, and end-to-end latency — the shipped layout is whichever
wins. See [Performance](../project/performance.md).

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# The Interned Dataset IR

PurRDF's resident carriers share one value-interned RDF 1.2 term representation,
owned by the ring-fenced
[`purrdf-core`](https://docs.rs/purrdf-core) kernel.

## Terms are interned once

Every term — IRI, blank node, literal, triple term — is interned **once** and
addressed by a copyable `TermId` (a niche-optimized `NonZeroU32`). Textual
payloads use the string arena; triple terms retain their component IDs.
Quads are rows of four `TermId`s. That makes term equality a
single integer compare, keeps quads at a fixed small size, and means a term
that appears in a million quads costs its bytes exactly once.

Hot maps use the fixed-key `FixedHasher` — deterministic hashing is part of the
[byte-determinism discipline](codecs.md), not just a speed choice.

## Builder → freeze

The IR has a strict two-phase life cycle:

```rust
use purrdf::{RdfDatasetBuilder, RdfLiteral};

// Intern terms once; quads are rows of copyable TermIds.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat")?;
let says = b.intern_iri("https://example.org/says")?;
let meow = b.intern_literal(RdfLiteral::simple("meow"))?;
b.push_quad(cat, says, meow, None)?;

// Freeze into the immutable, indexed dataset the engines evaluate over.
let ds = b.freeze().expect("well-formed dataset");
assert_eq!(ds.quad_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
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

## An explicit graph-free carrier

Choose `TripleDatasetBuilder` when the data has no named graphs. Its asserted
rows store subject, predicate and object in 12 bytes; reifier bindings take
8 bytes and annotations take 12 bytes. It shares term interning and validation
with `RdfDatasetBuilder`, but accumulates no quad rows or graph table.

`TripleDataset::try_from` moves the original term owner out of an all-default
quad dataset. It refuses any named row or named graph declaration, including an
empty graph, with `TripleReadError::NamedGraphsUnsupported`.

Open `TripleDataset::read()` for a fallible view. Named-graph query requests
refuse even when the carrier is empty or the graph name is absent. All children
of that read retain the same first failure; a new read over the unchanged
carrier starts an independent operation. The engine's ordinary default-graph
query path uses the compact carrier's SPO, POS and OSP orders.

For rule materialization, `purrdf::shapes::srl::infer_triples` accepts that read
and returns `Inference<TripleDataset>`. Its Datalog rows have three positions
and predicate-only partition keys. The ordinary quad `srl::infer` API keeps
its existing result type. Text indexing selects `TextIndexConfig::for_triples`
explicitly; document keys become subject/language and partition keys language,
with distinct graph-free identities.

## Copy-on-write mutation

"Immutable" does not mean "static". `MutableDataset` accumulates edits in a
copy-on-write delta over a frozen base. Its `snapshot_view()` shares the
unchanged base rows, terms and indexes. Its `freeze()` instead compacts the
effective dataset into a new frozen dataset, re-interning surviving terms and
rebuilding dense IDs. SPARQL UPDATE and the C ABI's mutable `PurrdfGraph` handle
both use this mutation layer.

## What else lives in the kernel

Beyond the IR itself, `purrdf-core` owns:

- **`DatasetView`** — the static read trait every engine evaluates over.
- **Structured diagnostics** — typed `RdfDiagnostic`s with source locations
  (deliberately SARIF-free; the SARIF boundary is
  [`purrdf-validate`](https://docs.rs/purrdf-validate)).
- **Canonicalization** (RDFC-1.0 on the RDF 1.1 subset; the `purrdf-rdfc12`
  profile over reifiers and annotations), dataset diff, and isomorphism — see
  [Canonicalization & Diff](canonicalization.md).
- **Store and engine seams** — the narrow parser-ingress, serializer-egress,
  and `SparqlEngine` traits that adapters implement in sibling crates.
- **Provenance and the loss ledger** — a generic provenance sidecar and the
  machine-readable RDF↔GTS loss matrix, plus native FnO and SSSOM codecs
  (see [Slices, Mappings & Provenance](../slices.md)).

Text codecs are *not* in the kernel — parsing and serialization live one layer
up in [`purrdf-rdf`](https://docs.rs/purrdf-rdf). The split keeps the kernel
small and its invariants enforceable at the crate boundary: no PyO3 (a
hygiene gate asserts the dependency tree), `wasm32`-clean, and a
file-IO-free IR layer.

## Why this design

The layout is chosen by measurement, not assertion: the bench
`crates/rdf-core/benches/ir_layout.rs` compares array-of-structs,
struct-of-arrays, and predicate-adjacency layouts on allocation counts,
high-water memory, and end-to-end latency — the shipped layout is whichever
wins. See [Performance](../project/performance.md).

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Codecs & Determinism

PurRDF ships **first-party** parsers and serializers — no wrapped third-party
codec — for seven formats:

| Format | Media type | Star-capable |
| --- | --- | --- |
| Turtle | `text/turtle` | yes |
| TriG | `application/trig` | yes |
| N-Triples | `application/n-triples` | yes |
| N-Quads | `application/n-quads` | yes |
| RDF/XML | `application/rdf+xml` | no |
| JSON-LD (star) | `application/ld+json` | yes |
| YAML-LD | `application/ld+yaml` | yes |

They live in [`purrdf-rdf`](https://docs.rs/purrdf-rdf), one layer above the
kernel, and are reachable through the umbrella crate:

```rust,ignore
use purrdf::{parse_dataset, serialize_dataset, SerializeGraph};

let turtle = br#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;

// Parse into the frozen, value-interned RDF 1.2 dataset IR.
let ds = parse_dataset(turtle, "text/turtle", None).expect("valid Turtle");
assert_eq!(ds.quad_count(), 1);

// Serialize back out through any native codec — byte-deterministic output.
let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
    .expect("serializes");
```

## Open Knowledge Format bundles

The native OKF codec maps caller-profiled RDF 1.2 datasets to agent-facing
Markdown files with YAML frontmatter and lifts them back through the RDF event
seam. OKF is an in-memory bundle API rather than another media type: callers
choose how to store the files, so the same code remains deterministic and
wasm-clean.

`OkfConfig::new` requires the vocabulary namespace, document base IRI, and
recognized frontmatter keys. There is no built-in ontology or namespace. Use
`lift_okf_bundle` to drive an `RdfEventSink`, or `write_okf_bundle` (backed by
`OkfWriter`, an `RdfDatasetVisitor`) to project a frozen dataset. Both directions
always return a loss ledger. A lossless profile yields an empty ledger; named
graphs, non-profile/OWL rows, and unrelated reifier or annotation rows are
pinpointed explicitly when writing.

## Byte determinism

Every serializer is **byte-deterministic**: the same dataset always produces
the same bytes, on every platform and in every language binding. This is a
hard workspace invariant, not a best effort — no iteration-order, time, or RNG
dependence is allowed in any output path (hashers are fixed-key `ahash` for
exactly this reason), and golden-file tests pin the emitted bytes.

Determinism is what makes the rest of the toolkit composable: content
addressing in [GTS](../gts.md) and the [slice catalog](../slices.md), diffable
serializations in review, and cross-language conformance vectors that can be
compared byte-for-byte.

## Diagnostics, not partial parses

Malformed input is a typed `RdfDiagnostic` with a source location where the
codec can provide one — never a silent partial parse. Parsing can optionally
record a source-position span table for richer diagnostics. Diagnostics stay
structured (SARIF-free) in the core; render them as byte-deterministic SARIF
2.1.0 for editors and CI with
[`purrdf-validate`](https://docs.rs/purrdf-validate) (see
[SHACL](../validation/shacl.md#sarif-output)).

## Lossy projections are loud

RDF 1.2 statement-level data (triple terms, reifier bindings, annotations)
survives every star-capable round-trip. Serializing into a star-incapable
projection drops that layer *loudly*: the realized drop count is handed to the
machine-readable loss ledger
([`generated/rdf-loss-matrix.json`](https://github.com/Blackcat-Informatics/purrdf/blob/main/generated/rdf-loss-matrix.json))
rather than disappearing. The same discipline applies at the SPARQL results
boundary ([Result Formats](../sparql/results.md)) and the RDF↔GTS boundary.

## The succinct pack codec

Alongside the text codecs above, `purrdf-core` ships a **binary** codec for a
different job: a read-only, query-the-compressed-form encoding of a whole
dataset for large-scale reference bundles, not an interchange format with a
media type. `PackBuilder::build_bytes(&dataset)` writes a self-contained,
byte-deterministic pack — a value dictionary, graph-partitioned succinct
bitmap-triples, and RDF 1.2 side-tables (reifier bindings, statement
annotations) — into one `Vec<u8>`. `PackView::from_bytes(&[u8])` opens it
zero-copy over a borrowed slice and answers pattern queries directly against
the packed bytes, with no decompression or materialization step first.

Reach for a pack when a dataset is done changing and needs to be distributed,
archived, or served at a scale where re-parsing text on every load is too
slow: RDF 1.2 (named graphs, quoted triples, reifiers, annotations) is fully
supported, and `verify_pack` independently recomputes the dataset's RDFC-1.0
digest from the pack's own decoded contents — a **certified read-only
projection**, not merely a compressed file. The library never memory-maps a
pack itself (every published crate stays `wasm32-unknown-unknown`-clean); a
native consumer that wants a durable, larger-than-heap tier `mmap`s the file
and hands `PackView::from_bytes` the resulting borrowed slice. See the
"Pack backend" section of
[the backend contract](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-backend-contract.md)
for the full contract.

## Deterministic embedding companions

`.purremb` is the mmap-native companion for embedding projections over one
exact `.purrpck`. It does not modify the pack or RDF canonical identity. Its
sorted section directory binds finite dense `f32` or `f64` matrices to the
source pack's exact SHA-256, an independently verified RDFC digest, complete
model and processing contracts, stable target sets, and per-section plus
whole-artifact integrity evidence. `EmbeddingBuilder` accepts unordered rows;
`EmbeddingStreamWriter` accepts canonical rows with bounded matrix working memory;
both produce the same canonical bytes.

Two subject families are first class. Large text collections use a
corpus–document–chunk hierarchy: UTF-8 text remains external while target
records retain content digests, logical identities, byte and Unicode-scalar
coordinates, chunking contracts, and family-scoped token spans. RDF data uses
one RDF 1.2 model for datasets, default and named graphs, statements, reifier
bindings, annotations, directional literals, blank nodes, and recursive triple
terms. Source-local pack ordinals are verified lookup hints, never identity.

Matryoshka families store only their widest dense matrix. Each declared leading
prefix is a distinct `VectorSpaceId` and `ProjectionId`, so a coarse prefix
cannot be silently compared with or substituted for the full space. Raw prefix
rows are zero-copy strided views; deterministic L2 prefixes are calculated on
demand. Approximate indexes remain opaque, rebuildable derived artifacts bound
to one exact prefix projection. They never replace the authoritative matrix.

Construction follows one evidence path. First obtain a
`CertifiedPurrpckSource` by building or independently verifying the exact source
pack; arbitrary digest claims cannot construct this type. For a corpus, derive
`CorpusTarget`, `DocumentTarget::from_content`, and
`TextChunkTarget::from_document` records, add the required hierarchy relations,
and add a `TokenSpan` for every document or chunk placed in a family matrix. For
RDF, derive dataset, graph, statement, reifier, annotation, and term targets
from that verified RDF 1.2 dataset. RDF-star triple terms use
`RdfTermTarget::Triple`; they do not enter a separate identity system.

An `EmbeddingFamilyContract` defines the complete generation pipeline. A
Matryoshka contract lists its allowed leading dimensions, while its
`MatrixInput` carries rows only at the widest dimension and one
`ProjectionSpec` per declared space. Consumers resolve an exact
`(TargetSetId, VectorSpaceId)` through `effective_matrix` and must call
`require_compatible_vector_spaces` before comparing rows from independent
inputs.

Large collections shard at artifact boundaries: each `.purremb` names its own
exact source pack and local target set, while equal family contracts retain the
same `FamilyId` and `VectorSpaceId`. Corpus manifests and
`ExternalBinding::from_bytes` bind external text or other exact artifacts;
`ExternalBinding::from_purrpck` adds independently certified RDF evidence.
Bindings carry caller-supplied roles and media types. PurRDF does not invent a
policy or ontology vocabulary for them.

`EmbeddingView::from_bytes` borrows any stable byte slice, whether heap-owned,
memory-mapped by the caller, or WebAssembly linear memory. Structural opening,
full artifact verification, exact source verification, and certified source
verification are explicit states of evidence rather than access gates. Callers
that mmap files must keep the backing bytes immutable while a view or resident
verification certificate exists.

Embeddings and ANN structures are sensitive derived content: model inversion,
membership inference, similarity probing, digest dictionary attacks, and index
structure can disclose source properties. Container hashes detect corruption
and stale attachment; they do not authenticate an author, encrypt content, or
grant access. See the byte-exact [PURREMB v1
specification](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/PURREMB.md).

## The columnar Parquet codec

`purrdf::columnar` exposes the bidirectional SQL/DataFrame interchange path.
It maps any `DatasetView` plus a content-addressed blob store to five standard
Parquet files (`terms`, `quads`, `reifiers`, `annotations`, and `blobs`) and
reads that exact profile back without Arrow or a general Parquet runtime. The
mapping retains RDF 1.2 triple terms, reifiers, annotations, graph scope,
directional literals, blank-node scope, and explicitly empty named graphs.

The files are byte-deterministic and readable by engines such as DuckDB. See
the [normative columnar schema](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/COLUMNAR.md)
for every field and the deliberately narrow Parquet profile.

## Conformance

The codecs are gated by the W3C `rdf-tests` syntax corpus, vendored and frozen
in-repo — 250/250 round-trip cases across N-Quads, N-Triples, RDF/XML, TriG,
and Turtle at the time of writing. The live scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).

## Related

- [Canonicalization & Diff](canonicalization.md) — when you need a *canonical*
  serialization rather than just a deterministic one.
- [The Interned Dataset IR](interned-dataset.md) — what the text codecs parse
  into, and the `DatasetView` read seam the pack codec implements alongside
  `RdfDataset`.

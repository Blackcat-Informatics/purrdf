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

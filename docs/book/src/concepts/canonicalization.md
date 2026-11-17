<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Canonicalization & Diff

Byte-deterministic serialization ([Codecs & Determinism](codecs.md)) means the
*same dataset* always emits the same bytes. Canonicalization is the stronger
property: two *different in-memory datasets that are isomorphic* — the same
graph up to blank-node relabeling — canonicalize to the same bytes.

## The canonical line sequence

The default canonical output is the canonical line sequence of the
nested-refinement labelling (`CANON_LABELLING_NAME`, `CANON_LABELLING_VERSION`),
spelled as N-Quads 1.2. Blank nodes are labelled `_:0`, `_:1`, and so on, in
decimal, and the identity of a dataset is the 32-byte BLAKE3 digest of the line
sequence. Reifiers, annotations, triple terms, composite literals that name blank
nodes and declared empty graphs all take part, and no vocabulary is reserved.
[Migrating to PurRDF 5.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/MIGRATION-5.0.md)
states the lowering exactly.

The entry point is `try_canonicalize`, producing canonical blank-node labels and,
one layer up in `purrdf-rdf`, canonical N-Quads over the frozen IR:

```rust
use purrdf::{try_canonicalize, parse_dataset, CanonicalizationControl, SearchPolicy};

let mut control = CanonicalizationControl::new(SearchPolicy::Unlimited, None);
let ds = parse_dataset(b"_:source <https://example.org/name> \"Alice\" .", "application/n-triples", None)?;
let renamed = parse_dataset(b"_:renamed <https://example.org/name> \"Alice\" .", "application/n-triples", None)?;
let canon = try_canonicalize(&ds, &mut control)?;
// Canonical labels are stable across runs, hosts, and language bindings.
assert_eq!(canon.nquads(), try_canonicalize(&renamed, &mut control)?.nquads());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The immutable `Canonicalized` result retains its source's owned storage
reservation. Borrow its text with `nquads()`, its presentation with
`presentation()`, and its labels with `label()` or `labels()`. Cloning shares the
payload and its original reservation; that reservation lives until the last
result owner is dropped.

Use canonicalization when you need a content identity for a graph: hashing,
signing, deduplication, or comparing datasets produced by different writers.

Every operation requires an explicit finite or unlimited search policy and accepts
an optional caller stop signal through its control. Each constituent run records
its own effective policy, configuration and charged work; the receipt array is
not an aggregate ceiling. A finite policy bounds the labelling engine's own
work, and a run that exceeds it is refused with the `limit` outcome.

## Native relabeling with source attribution

Use `canonical_relabel` when the next stage consumes a native dataset. It keeps
RDF 1.2 terms, reifiers, annotations, graph declarations and source locations
without rendering and reparsing a canonical document.

When a compiler or analyzer must associate source nodes with that result, use
`canonical_relabel_with_mapping`. It records the actual term mapping during
the same rebuild, after one canonical-label search:

```rust
use purrdf::{canonical_relabel_with_mapping, datasets_isomorphic, parse_dataset, TermRef, CanonicalizationControl, SearchPolicy};

let mut control = CanonicalizationControl::new(SearchPolicy::Unlimited, None);
let source = parse_dataset(b"<https://example.org/alice> <https://example.org/knows> _:friend .", "application/n-triples", None)?;
let source_node = source.term_id_by_iri("https://example.org/alice").ok_or("source IRI not interned")?;
let result = canonical_relabel_with_mapping(&source, &mut control)?;
let output_node = result.map_term(source_node).ok_or("source node not retained")?;
let canonical_value = result.dataset().resolve(output_node);
assert!(matches!(canonical_value, TermRef::Iri("https://example.org/alice")));
// Associate source receipts and owner metadata with canonical_value here.
let canonical_dataset = result.into_dataset();
assert!(datasets_isomorphic(source.as_ref(), &canonical_dataset, &mut control)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`source_node` must belong to the exact input dataset; `output_node` belongs to
this result. A same-index ID from another dataset cannot be detected. The map
includes nested triple components, literal datatypes, composite embedded blanks
and already-interned IRIs, reifier and annotation terms, and empty named graphs.
Unused dictionary entries omitted by the rewrite return `None`.

The map uses four bytes per source term, with constant-time lookup. The existing
`canonical_relabel` API does not allocate it. The map is not a durable identity
or proof certificate: persist resolved RDF identities with authenticated input
and output receipts, never raw `TermId` values. Source roles and standpoint
ownership remain the caller's metadata, not consequences of canonical labels.

## W3C RDFC-1.0 by name

PurRDF also implements W3C
[RDF Dataset Canonicalization (RDFC-1.0)](https://www.w3.org/TR/rdf-canon/)
natively, tested against the W3C `rdf-canon` fixture suite (65 vectors — 64 eval
plus 1 negative — all green; see
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)).
It lives under `purrdf_core::rdfc12` (also `purrdf_rdf::rdfc12`) and is not the
default output: its labels are `c14nN` and its digest is SHA-256 or SHA-384 of
N-Quads text. Use it to run the W3C algorithm by name, or to recompute a digest
that PurRDF 4.x recorded. Over reifiers and annotations it has two 4.x forms, the
flat form (`canonical_flat_nquads_with`) and the `purrdf-rdfc12` overlay, which
agree with RDFC-1.0 byte for byte only on the RDF 1.1 subset; a digest over
either must not be labelled RDFC-1.0. The normative text of the overlay is
[`docs/RDF12-CANON-PROFILE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/RDF12-CANON-PROFILE.md).

## Isomorphism

`datasets_isomorphic(a, b, control)` decides whether two frozen datasets are
RDF-structurally isomorphic: the same quads, reifier bindings, and annotations
under a single blank-node bijection.
Canonicalization gives the equivalent verdict — two datasets are isomorphic
iff their canonicalizations are equal — but the direct check is the
convenient form for tests and harnesses. PurRDF's own conformance harnesses
use graph isomorphism to compare, for example, SHACL Rules output graphs
against expected inferred graphs.

## Diff

`dataset_diff(a, b, control)` produces a structural diff between two frozen datasets,
including an `isomorphic` verdict. For a human-facing review flow,
`purrdf-rdf` additionally provides per-subject Symmetric-CBD extraction
("describe") and a review-friendly Turtle normalizer, so a graph change reads
like a code change.

## Choosing the right tool

| Need | Use |
| --- | --- |
| Same dataset → same bytes | any native serializer (always true) |
| Same *graph* (up to blank nodes) → same bytes, with or without reifiers and annotations | `try_canonicalize` or `canonical_flat_nquads` (the canonical line sequence) |
| The W3C RDFC-1.0 document, or a 4.x digest recomputed | `purrdf_core::rdfc12` |
| "Are these two datasets the same graph?" | `datasets_isomorphic` |
| "What changed between these datasets?" | `dataset_diff` + describe/normalize |
| Content-addressed transport of a graph | [GTS](../gts.md) (BLAKE3 content ids) |

API details are on
[docs.rs/purrdf-core](https://docs.rs/purrdf-core) (the `try_canonicalize`,
`datasets_isomorphic`, and `dataset_diff` items) and
[docs.rs/purrdf-rdf](https://docs.rs/purrdf-rdf) (describe and normalization).

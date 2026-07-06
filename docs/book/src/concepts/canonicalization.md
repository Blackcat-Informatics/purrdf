<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Canonicalization & Diff

Byte-deterministic serialization ([Codecs & Determinism](codecs.md)) means the
*same dataset* always emits the same bytes. Canonicalization is the stronger
property: two *different in-memory datasets that are isomorphic* — the same
graph up to blank-node relabeling — canonicalize to the same bytes.

## RDFC-1.0

PurRDF implements W3C
[RDF Dataset Canonicalization (RDFC-1.0)](https://www.w3.org/TR/rdf-canon/)
natively in the kernel, tested against the W3C `rdf-canon` fixture suite
(65 vectors — 64 eval plus 1 negative — all green; see
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)).

The entry point is `canonicalize` (with a `canonicalize_with` variant for
choosing the hash), producing canonical blank-node labels and, one layer up in
`purrdf-rdf`, canonical flat N-Quads over the frozen IR:

```rust,ignore
use purrdf::canonicalize;

let canon = canonicalize(&ds);
// Canonical labels are stable across runs, hosts, and language bindings.
```

Use canonicalization when you need a content identity for a graph: hashing,
signing, deduplication, or comparing datasets produced by different writers.

## Isomorphism

`datasets_isomorphic(a, b)` decides whether two frozen datasets are
RDF-structurally isomorphic: the same quads under a blank-node bijection.
Canonicalization gives the equivalent verdict — two datasets are isomorphic
iff their canonicalizations are equal — but the direct check is the
convenient form for tests and harnesses. PurRDF's own conformance harnesses
use RDFC-1.0 isomorphism to compare, for example, SHACL Rules output graphs
against expected inferred graphs.

## Diff

`dataset_diff(a, b)` produces a structural diff between two frozen datasets,
including an `isomorphic` verdict. For a human-facing review flow,
`purrdf-rdf` additionally provides per-subject Symmetric-CBD extraction
("describe") and a review-friendly Turtle normalizer, so a graph change reads
like a code change.

## Choosing the right tool

| Need | Use |
| --- | --- |
| Same dataset → same bytes | any native serializer (always true) |
| Same *graph* (up to blank nodes) → same bytes | RDFC-1.0 `canonicalize` |
| "Are these two datasets the same graph?" | `datasets_isomorphic` |
| "What changed between these datasets?" | `dataset_diff` + describe/normalize |
| Content-addressed transport of a graph | [GTS](../gts.md) (BLAKE3 content ids) |

API details are on
[docs.rs/purrdf-core](https://docs.rs/purrdf-core) (the `canonicalize`,
`datasets_isomorphic`, and `dataset_diff` items) and
[docs.rs/purrdf-rdf](https://docs.rs/purrdf-rdf) (describe and normalization).

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

## Native relabeling with source attribution

Use `canonical_relabel` when the next stage consumes a native dataset. It keeps
RDF 1.2 terms, reifiers, annotations, graph declarations and source locations
without rendering and reparsing a canonical document.

When a compiler or analyzer must associate source nodes with that result, use
`canonical_relabel_with_mapping`. It records the actual term mapping during
the same rebuild, after one canonical-label search:

```rust,ignore
use purrdf::canonical_relabel_with_mapping;

let result = canonical_relabel_with_mapping(&source)?;
let output_node = result.map_term(source_node).ok_or("source node not retained")?;
let canonical_value = result.dataset().resolve(output_node);
// Associate source receipts and owner metadata with canonical_value here.
let canonical_dataset = result.into_dataset();
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

## Over RDF 1.2 constructs: two forms, named apart

RDFC-1.0 has no notion of reifiers, annotations or triple terms. A dataset that
is plain RDF 1.1 canonicalizes identically everywhere in PurRDF; one that
carries those constructs canonicalizes in one of two forms, and which one you
are holding is part of the identity:

- **The flat form** — `canonical_flat_nquads` in `purrdf-rdf` rewrites the
  statement layer to plain `rdf:reifies` / annotation triples first and
  canonicalizes *that* triple set under conformant RDFC-1.0. This is what the
  CLI's `convert --canonical`, the wasm `Dataset.canonicalize()` and the W3C
  conformance gate run.
- **The `purrdf-rdfc12` v1 profile** — `canonicalize` in `purrdf-core` keeps
  the statement layer and lowers it into a reserved `urn:purrdf:rdfc:`
  namespace instead (any input already carrying that namespace is refused).
  It agrees with RDFC-1.0 byte for byte only on the RDF 1.1 subset, and a
  digest taken over its output **must not be labelled RDFC-1.0**;
  `CANON_PROFILE_ID` / `CANON_PROFILE_VERSION` name the profile at runtime.
  The normative text is
  [`docs/RDF12-CANON-PROFILE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/RDF12-CANON-PROFILE.md).

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
| Same *graph* (up to blank nodes) → same bytes, RDF 1.1 subset | RDFC-1.0 (`canonicalize` or `canonical_flat_nquads`, identical here) |
| Same *graph* with reifiers/annotations → same bytes | `canonical_flat_nquads` (RDFC-1.0 over the flattened layer) or `canonicalize` (`purrdf-rdfc12` profile) — not interchangeable |
| "Are these two datasets the same graph?" | `datasets_isomorphic` |
| "What changed between these datasets?" | `dataset_diff` + describe/normalize |
| Content-addressed transport of a graph | [GTS](../gts.md) (BLAKE3 content ids) |

API details are on
[docs.rs/purrdf-core](https://docs.rs/purrdf-core) (the `canonicalize`,
`datasets_isomorphic`, and `dataset_diff` items) and
[docs.rs/purrdf-rdf](https://docs.rs/purrdf-rdf) (describe and normalization).

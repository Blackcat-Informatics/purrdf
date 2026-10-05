<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Complete dataset state identity, version 1

`DatasetStateDigest` identifies complete observable RDF dataset state under one
blank-node bijection. It includes the mandatory default graph, every named graph
declaration (including empty graphs), and the three separate sets of ordinary,
native reifier and native annotation rows. A row present in two roles contributes
twice, once in each role. Repeated rows within one role contribute once.

The construction is additive. RDFC-1.0 canonical documents, the RDF 1.2
canonicalization profiles, graph/row digests, PACK certificates and serializer
bytes retain their own definitions. A complete-state digest does not substitute
for one of those identities.

## API and refusal

The core, RDF and umbrella crates expose `DatasetStateDigest` and the
non-exhaustive `DatasetStateError<Error, Evidence>`. `from_view` accepts a
`FallibleDatasetView`, including a frozen `RdfDataset`, composite, delta and
paged query views. The result contains 32 bytes and supports `as_bytes`, `to_hex`
and `Display`.

```rust
use purrdf::{DatasetStateDigest, RdfDatasetBuilder};

let dataset = RdfDatasetBuilder::new().freeze()?;
let identity = DatasetStateDigest::from_view(&dataset)?;
println!("{identity}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The whole drain uses before/after source checkpoints. An incomplete or failed
source operation refuses the digest even if its iterator returned a prefix or
all rows. A checkpoint failure takes precedence over an inner point-read,
workspace or canonical-search refusal. Point reads and retained-workspace
admission preserve the source's typed error. A missing or inconsistent reverse
lookup for a CDT blank reference returns `IncoherentEmbeddedBlank`; a literal
whose datatype does not resolve to an IRI returns `InvalidTerm`. Unrepresentable
sizes return `Capacity`.

Canonical search has the unchanged numeric `RDFC_CALL_LIMIT` bound, charged
for visited search states, candidate expansion and explicit automorphism checks.
Refinement has a separate finite input-derived bound: every nonterminal round
strictly increases the number of nonempty cells, so `n + 1` rounds suffice for
`n` blanks, including the stable check and the empty state. Ordered color buckets
visit the blanks once per round. A structurally discrete state therefore needs
one search node regardless of its blank count. `SearchBudgetExceeded` publishes
no approximation or partial identity. The old RDFC algorithm and its work
accounting do not change.

Retained capture/search workspace is admitted before allocation and released
before returning the digest. Rendering, incidence and partition scaffolding use
an aggregate record bound plus per-blank metadata. Signature storage is bounded
by the records actually incident to each blank, with framing and temporary
buffer growth included. A record containing many distinct blanks can contribute
to many simultaneously retained signatures; independent anchored records do not
multiply the whole record set by the blank count. Term traversal, rendering and
search use heap work lists rather than call-stack recursion.

## State and blank identity

The named graph set is the union of the view's declared names and every named
graph occurring in any of the three row roles. The default graph has an
unconditional declaration even when it contains no rows. All reached terms are
included structurally; unused dictionary entries, indexes, storage layout and
diagnostic locations do not participate.

A blank node's input label and `BlankScope` determine its input identity. One
bijection replaces every occurrence of that node: graph names, subjects,
objects, nested triple components and references in CDT list/map values,
including nested composite-typed lexical strings. Each embedded pair must
reverse-resolve and then resolve coherently in the same view's identity space.
Blank scopes and authored labels do not enter the canonical bytes.

IRI bytes and literal datatype, lexical, language and direction bytes remain
exact. CDT rewriting replaces only bound blank references with `_:sN`, where
`N` is the global canonical ordinal in decimal. Every other lexical byte stays
as authored, including whitespace and literal escape spelling. A non-CDT
lexical form that resembles a blank reference is ordinary text. The codec
uses byte tags rather than RDF sentinel terms and accepts authored IRIs from
any namespace supported by the source view.

## Normative byte grammar

All integer fields are eight-byte unsigned little-endian values unless a
one-byte tag is explicitly specified. `frame(x)` is `u64_le(byte_length(x))`
followed by `x`. Text fields are exact UTF-8 bytes. Ordinals range from zero to
one less than the number of reached blank nodes.

| Term | Encoding |
| --- | --- |
| IRI | `0x00 \|\| frame(iri)` |
| Blank | `0x01 \|\| u64_le(ordinal)` |
| Literal | `0x02 \|\| frame(lexical) \|\| frame(datatype) \|\| language \|\| direction` |
| Triple term | `0x03 \|\| term(subject) \|\| term(predicate) \|\| term(object)` |

A language or direction field is `0x00` for absence, or `0x01 || frame(value)`
for presence. Direction values are the existing exact `ltr` or `rtl` spellings.
A graph field is `0x00` for the default graph or `0x01 || term(name)` for a
named graph. Term encodings are self-delimiting, so triple components need no
additional frame.

| State record | Encoding |
| --- | --- |
| Default graph declaration | `0x00` |
| Named graph declaration | `0x01 \|\| term(name)` |
| Ordinary row | `0x02 \|\| term(s) \|\| term(p) \|\| term(o) \|\| graph` |
| Native reifier row | `0x03 \|\| term(s) \|\| term(p) \|\| term(o) \|\| graph` |
| Native annotation row | `0x04 \|\| term(s) \|\| term(p) \|\| term(o) \|\| graph` |

The ordinary/reifier/annotation term triples are the raw rows exposed by the
view. Reifier rows therefore retain the view's native `rdf:reifies` predicate
and quoted triple object without flattening that role into an ordinary row.
Exact same-role records are deduplicated before canonical labeling. At each
terminal labeling, records are sorted lexicographically by unsigned bytes. The
payload is `u64_le(record_count)` followed by one framed field per sorted record.

The final preimage is the raw ASCII bytes
`purrdf-core/dataset-state/v1` followed by the canonical payload. Its digest is
first-party BLAKE3-256. The domain is not length-framed; the payload's framing
starts immediately after the domain. The empty dataset payload is
`0100000000000000010000000000000000`: one one-byte default declaration.

The twelve frozen [preimage and digest vectors](../crates/rdf-core/tests/goldens/dataset-state-v1.txt)
pin empty IRI/blank declarations, distinct roles and coexistence, nested triples,
globally shared list/map and embedded composite references, and exact NUL/UTF-8
literal bytes with language and direction metadata. Native tests compare actual
canonical payloads with the independently specified bytes, then check both the
public digest and the pinned BLAKE3 hex.

## Canonical terminal family

The canonical payload is the minimum over an invariant terminal family, rather
than an unconstrained lexical minimum over every permutation of blank ordinals.
The following exact construction specifies that family.

1. Start with all blanks in one ordered cell. Build each blank's exact sorted
   incident record signatures. Render its own occurrences with a distinguished
   focus and every other blank with its current cell ordinal. Direct blank
   terms use a one-byte focus/other marker, with a little-endian cell ordinal
   only for other nodes; CDT references use `_:f` for the focus and `_:cN` for
   another node in cell `N`. Signatures are whole record bytes, not hashes.
2. Within each parent cell, split by the exact lexicographic signature vector.
   Preserve parent-cell order and order the new child cells by those vectors.
   Repeat until no split occurs.
3. If the partition is discrete, cell order gives the global ordinals. Render
   its complete payload. Otherwise choose the first non-singleton cell,
   individualize each candidate before its remaining cell, refine, and continue
   every branch. Retain the least complete terminal payload.
4. Sibling candidates may be identified only after their transposition is
   explicitly proven to preserve the entire typed record set and the current
   partition. Equal refinement colors do not prove this. If every member of
   one cell is interchangeable with a representative, those transpositions
   generate every permutation of the cell; batch individualization yields the
   same terminal family and may use any member order.

## Completeness theorem

The typed codec is injective: disjoint term/record tags, explicit presence,
fixed-width ordinals, field lengths and record counts uniquely recover a
complete labeled state. Every terminal labeling is one global bijection.
Therefore equal canonical payloads induce an isomorphism preserving graph
declarations, roles, positions, nested/CDT blank sharing and all other bytes.

Conversely, any state isomorphism transports incident signatures, ordered
refinement cells and individualized branches to their counterparts. Exact
whole-state automorphism pruning removes only branches with equal terminal
families. Isomorphic states thus have the same terminal family and the same
minimum payload. Canonical payload equality holds exactly for complete-state
isomorphism. Digest equality additionally relies on the usual cryptographic
collision-resistance assumption for BLAKE3; no finite digest is mathematically
injective over arbitrary finite datasets.

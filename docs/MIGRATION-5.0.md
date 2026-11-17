<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Migrating to PurRDF 5.0

PurRDF 5.0 changes what its canonical output is. Every canonical document,
canonical digest and blank-node label that PurRDF 4.x produced for a dataset
with blank nodes, with reifiers or annotations, or with a declared empty graph
is replaced. Upgrade the PurRDF crates together, and read the sections on stored
digests before opening a 4.x artifact.

## At a glance

- The default canonicalization family routes to the labelling named
  `nested-refinement-labelling`, version `1` (frozen specification, revision
  3.11, SHA-256
  `a2dc6ea69735f62cde18be17bc14890d89b4416e8dcf3bba42e3cfe55e283c15`).
- The canonical document is the canonical line sequence, spelled as N-Quads 1.2.
  Blank nodes are `_:0`, `_:1`, and so on, in decimal. `_:c14nN` no longer
  appears in canonical output.
- The canonical digest is the 32-byte BLAKE3 digest of the line sequence, not
  SHA-256 of N-Quads text. It moves for every dataset, including datasets with no
  blank node.
- RDFC-1.0 stays a named entry point and the W3C suite passes unchanged. The 4.x
  algorithm (the `purrdf-rdfc12` overlay and its flat form) stays callable as the
  read-side recompute of stored 4.x digests. No writer records a 4.x digest in a
  new artifact, with the two exceptions in "Containers".
- The search bound is a limit on the engine's own work. Refusal is the labelling's
  typed `limit` outcome.
- `DatasetStateDigest` is a separate algorithm and is unchanged.

## The canonical line sequence

This section is enough to reimplement the 5.0 canonical document. The labelling
itself (positions, refinement, the search) is defined by the frozen
specification named above; this guide states what PurRDF feeds it and what it
writes from the result.

### Lowering a dataset to lines

Lowering is outside the specification, so this is PurRDF's own rule. A dataset
view becomes a set of typed lines:

| Line | Tag | Supplied for |
|---|---|---|
| `quad` | `0x40` | each distinct statement: subject, predicate, object, optional graph |
| `reifier-binding` | `0x41` | each distinct reifier row: the reifier, its triple term, optional graph |
| `annotation` | `0x42` | each distinct annotation row: count, reifier, predicate, object, optional graph. The count is the number of identical rows the view yields (one for a resident dataset) |
| `empty-graph` | `0x43` | each named graph the view addresses that no statement line carries |
| `record` | `0x44` | each caller record: an opaque payload and an ordered list of terms. A record denotes nothing in RDF and adds no graph |

A blank node's source identifier is its scope ordinal, four bytes big-endian,
followed by its label bytes, so two blank nodes are one node exactly when their
label and scope are equal. A term is an IRI, a literal (lexical form, datatype,
optional language, optional direction), a blank node or a triple term. The
default graph is the absent graph option.

### Encoding and order

Every line has one byte encoding, built from `u64(n)` (eight bytes big-endian),
`bytes(b)` (`u64` length, then the bytes) and one-byte tags:

| Tag | Variant | Fields after the tag |
|---|---|---|
| `0x01` | `iri` | `bytes(value)` |
| `0x02` | `literal` | `bytes(lexical)`, `bytes(datatype)`, option of `bytes(language)`, option of a direction |
| `0x03` | `triple` | three terms |
| `0x05` | `fixed` | `u64(position)` |
| `0x20` / `0x21` | `absent` / `present` | none / one value |
| `0x30` / `0x31` | `ltr` / `rtl` | none |
| `0x40` | `quad` | subject, predicate, object, graph option |
| `0x41` | `reifier-binding` | reifier, triple term, graph option |
| `0x42` | `annotation` | `u64(count)`, reifier, predicate, object, graph option |
| `0x43` | `empty-graph` | one term |
| `0x44` | `record` | `bytes(payload)`, `u64(n)`, then `n` terms |

The labelling assigns every blank node `b` a position `pos(b)`, a bijection onto
`0 .. N - 1`. The canonical line sequence replaces each blank leaf of each input
line, lines without a blank node included, by `fixed(pos(b))` and lists the
encodings in ascending bytewise order. The sequence has no repeated element.

### Digest

The digest is BLAKE3 in key-derivation mode with a 32-byte output. The context
is the ASCII string `nested-refinement-labelling 1 lines`, and the message is
`u64(number of lines)` followed by the encodings concatenated in order.

### Spelling as N-Quads 1.2

The document keeps the order of the sequence: one text line per statement line,
not the order of the spelled text. A blank node is `_:` and its position in
decimal. Terms use the canonical N-Quads term syntax: IRIs in angle brackets with
the canonical escapes, a literal as `"lexical"` with `@language` or
`@language--direction`, `^^<datatype>` for any datatype other than `xsd:string`,
and a triple term as `<<( s p o )>>`. Each line is spelled as:

- `quad`: `s p o .` or `s p o g .`
- `reifier-binding`: `reifier <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> triple [g] .`
- `annotation`: `reifier p o [g] .`, once, whatever its count
- `empty-graph` and `record`: nothing

Because `empty-graph` and `record` lines have no spelling, and two lines of
different kinds may spell one statement, the text is not injective on the
sequence. The digest, not the text, carries the identity. Labels are output only
and are never parsed back.

## Stored digests

A digest recorded by 4.x is the SHA-256 of the `purrdf-rdfc12` N-Quads, in its
overlay or flat form. Such a digest is never equal to the 5.0 digest of the same
dataset. To check one, recompute it with the 4.x function:

- `purrdf_core::rdfc12::try_canonical_sha256_view` with `CanonHash::Sha256` for
  the digest, and `purrdf_core::rdfc12::try_canonicalize_flat_view` or
  `purrdf_rdf::canonical_flat_nquads_with` for the flat document.
- The profile constants (`CANON_PROFILE_ID`, `CANON_PROFILE_VERSION`, the
  presentation identifiers, `CANON_CORPUS_DIGEST`, `RDFC_V2_CORPUS_POLICY`,
  `RESERVED_NAMESPACE`) and the 4.x search types are under `purrdf_core::rdfc12`
  and are no longer re-exported at the crate root.

The 4.x path behaves as it always did: labels are `c14nN`, the hash is the
caller's SHA-256 or SHA-384, and a dataset that holds the reserved vocabulary of
`docs/RDF12-CANON-PROFILE.md` is refused. The default entry points have no
reserved vocabulary, so `CanonError::ReservedVocabulary` no longer comes from
them.

## Containers

- **PACK.** Format version `1` means a 4.x digest. The 5.0 writer emits format
  version `2`, whose `rdfc_digest` header field is the line-sequence digest. The
  reader accepts both, and `verify_pack` recomputes each version with the function
  that recorded it. Because the line sequence holds a line for each declared empty
  graph, a pack that declares an empty graph now has a different digest from the
  same pack without the declaration; under version `1` they were equal.
- **GTS compaction certificates.** The refold digests of a certificate are bare
  hex strings with no algorithm field, and the GTS wire format is governed in
  `gmeow-gts`. They stay on the RDFC-1.0 flat form, so certificates written by 4.x
  and 5.0 agree. This is the one place 5.0 writes a 4.x-style digest; moving it
  needs a versioned certificate in `gmeow-gts`.
- **Columnar.** The five-table Parquet files record no canonical digest, only
  content digests of blobs and of the manifest. They are unchanged.
- **Embedding (PURREMB).** The `SOURCE` section records a certified RDF digest and
  blank-node target identities derived from canonical labels. 5.0 computes both
  with the labelling under the unchanged PURREMB version `1`; a 4.x embedding is
  therefore not accepted by the 5.0 verifier, and the embedding version for the
  new digest has not been decided.

## Hosts

- **Labels and receipts.** Every host returns `_:N` labels. Receipts name the
  labelling, its version and the `line-sequence` presentation.
- **The bound.** The `canonical_search` argument and `--canonical-search` map onto
  the engine's typed limit: a finite value is a number of the engine's limited
  work units, and `unlimited` is the engine's own limit. A finite bound that the
  input exceeds is refused with the labelling's `limit` outcome, and no canonical
  bytes are published. An input the labelling can finish without a search
  succeeds under every bound, including `0`.
- **Rust.** `CanonError::Refused` carries the labelling's typed refusal
  (`input`, `graph`, `width` or `limit`). The new constants are
  `CANON_LABELLING_NAME`, `CANON_LABELLING_VERSION`,
  `CANON_PRESENTATION_LINES_ID`, `CANON_PRESENTATION_LINES_VERSION` and
  `CANON_CORPUS_POLICY`.
- **Flat presentation.** `canonical_flat_nquads` returns the line-sequence
  document. The identifier `flat-assertion`, version `1`, now names only the 4.x
  flat form reached through `rdfc12`; the default flat document reports
  `line-sequence`, version `1`.
- **Python, WebAssembly, C and the CLI.** Each routes to the default family.
  Their canonical documents, digests and labels change as above, and a refusal
  under a finite bound is the `limit` refusal.

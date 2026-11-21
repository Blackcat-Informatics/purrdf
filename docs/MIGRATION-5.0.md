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
  3.12, SHA-256
  `088cb74fba0f5f733224157ab7c3a6e06f4a3f3447edf4c01bb15cf15c58809a`).
- The canonical document is the canonical line sequence, spelled as N-Quads 1.2.
  Blank nodes are `_:0`, `_:1`, and so on, in decimal. `_:c14nN` no longer
  appears in canonical output.
- The canonical digest is the 32-byte BLAKE3 digest of the line sequence, not
  SHA-256 of N-Quads text. It moves for every dataset, including datasets with no
  blank node.
- RDFC-1.0 stays a named entry point and the W3C suite passes unchanged. The 4.x
  algorithm (the `purrdf-rdfc12` overlay and its flat form) stays callable as the
  read-side recompute of stored 4.x digests (profile version 4). A stored 4.x
  digest of a dataset holding U+FFFE or U+FFFF in a literal no longer recomputes,
  because it was computed over a spelling that does not conform to the canonical
  form of RDF 1.2 N-Quads; every other stored digest recomputes unchanged. No
  writer records a 4.x digest in a new artifact. A GTS compaction certificate names the function that recorded its
  refold digests (see "Containers").
- The search bound is a limit on the walk of the specification (section 8.3), a
  function of the input as given and of the bound, so every engine refuses the
  same inputs with the same fields. Refusal is the labelling's typed `limit`
  outcome.
- `DatasetStateDigest` is a separate algorithm and is unchanged.
- Every identity and fingerprint of a set of rows is BLAKE3 over the sorted row
  digests, not an additive sum. Content identities, execution receipts, source
  version fingerprints and text index generations recorded by 4.x stop
  matching (see "Row-set identities").
- `SERVICE` and `LOAD` share one caller-supplied HTTP exchange,
  `HttpExchange`, which replaces the POST-only `HttpTransport` (see "The HTTP
  exchange").
- `GeoIndex::source_fingerprint` is a 32-byte digest, not a `u64` (see "Geo index
  source identity").

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
| `record` | `0x44` | each caller record: an opaque payload and an ordered list of terms. A record denotes nothing in RDF and adds no graph. Each composite node also has one (below) |

A blank node's source identifier is its scope ordinal, four bytes big-endian,
followed by its label bytes, so two blank nodes are one node exactly when their
label and scope are equal. The form is decided per input, the statements the
scope selects together with the caller's records: when that input holds a
nested triple term, every blank node of it uses the form below instead, a
nested term only in a graph the scope leaves out does not change the form,
and a nested term only in a record changes it even when the dataset holds
none. A term is an IRI, a literal (lexical form, datatype,
optional language, optional direction), a blank node or a triple term. The
default graph is the absent graph option.

### Nested triple terms

A triple term that is a component of another triple term, in a line or in a
record's terms, is a nested term; nested terms equal as values are one. Each
distinct nested term `x = (s, p, o)` is lowered to a term node, a blank node that
is no blank node of the dataset, with one `record` line:

- The payload is the eleven bytes `82 05 84 81 01 81 01 81 01 81 01`
  (`E([5, [[1], [1], [1], [1]]])`, `E` the deterministic CBOR encoding). The
  terms are the term node, then `N(s)`, `N(p)` and `N(o)`, where `N(c)` is the
  term node of `c` when `c` is a triple term and `c` otherwise.
- Wherever `x` is a component of a triple term, its term node stands in its
  place. A triple term that stands in a slot of a line or directly in a record's
  terms stays there, with its components lowered, so every triple term of the
  lines has depth one.
- The term node's source identifier is `E([d(x)])`, with
  `d(x) = H(E([K(s), K(p), K(o)]))`, `H` BLAKE3 in key-derivation mode under the
  context `gts2 nested term` and `K(c)` the source of a component as a byte
  string: `E([0, h(b)])` for a blank node, `h(b) = H(E([ordinal, label]))`;
  `E([d(c)])` for a triple term; for an IRI the byte string of `E([1, text])`;
  for a literal the byte string of `E([2, lexical, form, x])` with `form` 1 for a
  typed literal and 2, 3 or 4 for a language literal with no direction, `ltr` or
  `rtl`, and `x` the datatype IRI or the language. A composite node takes the
  source of its literal.
- When the input of a labelling holds a nested term, every blank node of that
  input has the source identifier `E([ordinal, label])`, the scope ordinal an
  unsigned integer and the label a byte string, so every term node precedes every
  blank node in source order. The input is the statements the scope selects
  together with the caller's records: a nested term only in a graph the scope
  leaves out does not change the form, and a nested term only in a record changes
  it even when the dataset holds none. An input with no nested term keeps the
  four-byte source identifiers.

An input with no triple term, or whose triple terms all have depth one, lowers
to the same lines as before and keeps its labels and digests. An input with a
nested term gets new labels and a new digest. The lowering reads triple terms by
the identifiers of their components and never expands a term that holds
another. The lowered set of a chain of `n` nested statements is linear: one
record of bounded size for each nested term. The bytes of leaf components are
copied and hashed once for each distinct nested term they stand in, so a long
leaf shared by every term of a chain is processed once per term. The labelling
of the lowered chain has a linear walk, linear allocations and linear retained
bytes, and the bytes it requests from the allocator and the instructions it
executes grow quadratically in the labelling engine (the design document records the
measurements at 1,000, 10,000 and 100,000 terms).

RDF 1.2 nests a triple term only in the object of another. A dataset refuses any
other nesting when it is frozen, and the lowering refuses a triple term as the
subject or the predicate of a triple term in any view or record with the
`input` refusal (a slot holding a variant it does not admit).

A canonical form that holds a nested triple term renumbers its blank labels:
each term node takes a position among the blank nodes, so the labels of the
view's own blank nodes move up, and the order of members that were equal
before the renumbering can swap. A consumer that stores or compares the
canonical text of a result with a nested triple term, such as a recorded
evaluator trace, regenerates it. The statements of the document are the same
up to the renaming of blank nodes.

### Language tags

RDF 1.2 compares language tags without regard to case, so a dataset holds each
tag folded to ASCII lower case and the labelling encodes the tag as the dataset
holds it. A format that compares a language text whole and bytewise, as GTS
does, keeps `"x"@EN-us` and `"x"@en-us` as two terms. The canonical RDF digest
of a dataset folds tags; a digest over terms as written does not. The two
identities are equal exactly when every language tag is already lower case.

Reading a GTS file that holds a tag that is not lower case into a dataset
records one `language-tag-case-folded` entry per affected literal in the
envelope's loss ledger (`RdfEnvelope::losses`), naming the literal, the written
tag and the tag the dataset holds; the `convert` command reports it with the
other read losses. A dataset written to GTS writes the tag it holds.

### Composite literals

A blank-node label written inside a `cdt:List` or `cdt:Map` literal names a
blank node of the dataset, the one with the same label and scope as a blank
node written in any other position. Such a literal is lowered so that the
blank nodes it names take part in the labelling:

- Its tokens are the `BLANK_NODE_LABEL` tokens of its lexical form at every
  nesting depth, in order of their first byte, skipping a token that starts
  inside one already taken. The form is then `t0 k1 t1 ... kn tn`, with `n` at
  least one, each `ki` a token and each `ti` the text around them, possibly
  empty.
- Wherever the literal occurs, its leaf is the blank leaf of a composite node,
  one node per distinct literal (lexical form, datatype, language, direction).
  The composite node's source identifier is four zero bytes, the byte `0xFF`,
  then the literal's leaf encoding (tag `0x02` and its fields, below). No
  blank node of a view has such a source identifier, because `0xFF` never
  begins a UTF-8 label.
- Each composite node that a line holds has one `record` line. Its payload is
  `bytes(datatype)`, then `bytes(t0)` through `bytes(tn)`. Its terms are the
  composite node, then the blank node each of `k1` through `kn` names, in
  order, repeats kept.
- A composite literal that holds no blank-node token is a literal leaf like
  any other literal.

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

A term node is spelled as the triple term it stands for, from the components of
its record, which may hold term nodes in turn. Term nodes take positions, so
the labels of the blank nodes of an input with a nested term need not be
consecutive.

The escapes follow the canonical form of RDF 1.2 N-Quads (section 3). U+FFFE and
U+FFFF in a literal are written `\uFFFE` and `\uFFFF`, since neither matches the
XML 1.1 `Char` production; the C1 controls stay raw. An IRI is escaped exactly
where the `IRIREF` production forbids a scalar, so DEL and the C1 controls ride
raw. The digest is over the binary line encodings and does not depend on these
spellings. The retained 4.x `purrdf-rdfc12` output spells U+FFFE and U+FFFF the
same way (profile version 4), so a stored 4.x digest of a dataset holding either
in a literal no longer recomputes; see "Stored digests".

A composite node is spelled as its literal, with each token `ki` replaced by
`_:` and the position of the blank node it names. No label spells a composite
node, so the labels of one document can skip a number.

Because `empty-graph` and `record` lines have no spelling, and two lines of
different kinds may spell one statement, the text is not injective on the
sequence. The digest, not the text, carries the identity. Labels are output only
and are never parsed back.

## Stored digests

A digest recorded by 4.x is the SHA-256 of the `purrdf-rdfc12` N-Quads, in its
overlay or flat form. Such a digest is never equal to the 5.0 digest of the same
dataset. To check one, recompute it with the 4.x function. A digest of a dataset
holding U+FFFE or U+FFFF in a literal does not recompute, because it was taken
over the raw spelling that profile version 3 wrote; every other stored 4.x digest
recomputes unchanged:

- `purrdf_core::rdfc12::try_canonical_sha256_view` with `CanonHash::Sha256` for
  the digest, and `purrdf_core::rdfc12::try_canonicalize_flat_view` or
  `purrdf_rdf::canonical_flat_nquads_with` for the flat document.
- Every other 4.x item is under `purrdf_core::rdfc12`, also reachable as
  `purrdf_rdf::rdfc12`, with the same name and signature it had at the crate
  root. See "Items that left the crate root".

The 4.x path behaves as it did, apart from the spelling of U+FFFE and U+FFFF in
a literal (profile version 4; see "Stored digests"): labels are `c14nN`, the hash is the
caller's SHA-256 or SHA-384, and a dataset that holds the reserved vocabulary of
`docs/RDF12-CANON-PROFILE.md` is refused. The default entry points have no
reserved vocabulary, so `CanonError::ReservedVocabulary` no longer comes from
them.

## Row-set identities

In 4.x, the identities and fingerprints derived from a set of rows added the
256-bit row digests modulo 2^256 and hashed the sum with the row count. A party
that chooses rows can construct two different row sets with the same sum and
count (D. Wagner, "A Generalized Birthday Problem", CRYPTO 2002), so equal
values did not prove equal rows. In 5.0 each row family is BLAKE3 over its row
digests in ascending order, repeats kept, followed by the row count, under a
domain of its own (`purrdf_hash::multiset::sorted_fingerprint`). Two different
row sets share a value only through a BLAKE3 collision.

`purrdf_hash::multiset::Multiset256::fingerprint` is renamed
`Multiset256::accounting_check`, with the same signature and the same output
bytes under the same domain framing; no alias remains. Replace each call with
the new name. The value is an additive accounting check over the population and
the wrapping sum, for drift and population checks between structures one
process built. It is not the identity of a row set: use `sorted_fingerprint`
where a row set needs an identity.

Every value below moves for every input, except that a segmented stack made only
of image layers, with no removal or declaration owners and no resident head,
hashes image snapshots alone and keeps its 4.x snapshot identity and the
receipts bound to it. A value recorded by 4.x otherwise never equals the 5.0
value of the same data. Nothing recomputes a 4.x value: recompute
the identity from the data and record it again.

| Value | What moved |
|---|---|
| `ContentIdentified::content_identity` of `RdfDataset`, of a triple dataset, and of `CompositeDatasetView`, `DeltaDatasetView`, paged datasets and segmented stack snapshots and queries (see the exception above), and the SHACL data and class-membership views | The resident record and named-graph families, and a mutation snapshot's suppressed, converted and withdrawn-graph families, are sorted-digest fingerprints (domains `purrdf-core/snapshot/rows/v2`, `purrdf-core/snapshot/graphs/v2`, `purrdf-core/delta-snapshot/v2` and its family domains). Composed views hash these identities, so they move with them. The class-membership view hashes its supplementary subclass edges the same way (`purrdf-shapes/class-membership-snapshot/v2`). |
| Execution receipts | `contentIdentity` and `executionIdentity` of a receipt bound by `purrdf_validate::query::bind_execution_receipt(receipt, &view)`, which takes a view that implements `ContentIdentified` (pass the view the query read), by Python `QuadStore.bind_execution_receipt`, by JavaScript `Dataset#bindExecutionReceipt` and by C `purrdf_execution_receipt_bind`; `bind_execution_receipt_hex` and `ReceiptBindError::ContentIdentity` are removed, so replace a call that passed a hex content identity with `bind_execution_receipt` on the view the query read; `QueryCoordinates::execution_identity`, `SuppliedQueryCoordinates::execution_identity` and `RuleCoordinates::execution_identity`, which take these content identities. |
| `SourceVersion::content_fingerprint` | BLAKE3 over the population and the canonical Merkle node of the sorted record digests (`purrdf/source-version/v2`). |
| `TextIndex::source_fingerprint`, `TextSource::source_fingerprint` and `purrdf_text::source_digest` | BLAKE3 over the sorted selected-row digests (`purrdf-text/source/v3`, `purrdf-text/triples/source/v3`). `verify_binding` refuses an index whose stored source identity came from 4.x. |
| `TextIndex::fingerprint`, `snapshot_fingerprint` and the generation retrieval strata attest | The index generation binds the source identity, so it moves with it, and so does every retrieval evidence identity recorded over a 4.x generation. |
| `SourceMatchEvidence::literal` of a match from an index-built surface, and the literal identity Han occurrences carry | The per-row source identity moved from the additive `purrdf-text/source/v2` law to the sorted `purrdf-text/source/v3` law. |
| `SurfaceIndex` generation of a standalone surface (`SourceMatchEvidence::generation`) | The sources enter as a sorted-digest fingerprint (`purrdf-text/surface-generation/v3`). |

`purrdf_text::source_fingerprint` keeps its signature and its 4.x value, and is
deprecated: no 5.0 index carries that value. The value is not
collision-resistant and must not be used as an identity; use
`purrdf_text::source_digest` or `TextIndex::source_fingerprint` instead. The
function is removed in the next major version. `SourceSummary::records` and
`SourceSummary::literal_rows` keep their additive `Multiset256` values, which
combine across physical layers without visiting rows; they are accounting sums
for drift and population checks between structures one process built, and no
identity is derived from them. `TextDeltaError::SourceMismatch` reports check
values of the same accounting. `DatasetStateDigest`, pack identities and
the image snapshots inside a segmented stack already hashed bytes or sorted rows
and are unchanged.

Two cases are settled by the new law. A paged dataset certifies a page it
materializes by recomputing the page's content identity from the rows the
provider returned and comparing it with the identity sealed at admission; that
refusal now holds against a provider that chooses the rows it returns. The
supplementary subclass edges of a class-membership view, taken from a shapes
graph or ontology another party supplies, no longer admit an edge set that
collides with another.

## Geo index source identity

`GeoIndex::source_fingerprint` returns a 32-byte `purrdf_hash::hex::Digest32`
instead of a 64-bit `u64`, and `IndexBindingError::Source` carries two of them.
The value is a BLAKE3 digest, key-derived under `purrdf-geo/index-source/v2`
and framed through `purrdf_hash::frame`, over the same deterministic index
projection: the same scope, full geometry precision and admission, with input
order leaving it unchanged. No index form stores it, so nothing in the library
needs conversion. Code that holds, compares or hex-formats the value as a `u64`
no longer compiles; recompute the digest from the source and store its 32 bytes
or its 64-digit hex form (`Digest32::to_hex`, `Digest32::from_hex`). Code that
only compares or displays the digests of `IndexBindingError::Source` still
compiles, and its `Display` output becomes 64 hex digits.

## Items that left the crate root

These items are no longer exported from the root of `purrdf_core`, `purrdf_rdf`
or `purrdf`. Each keeps its name, signature and 4.x behaviour under
`purrdf_core::rdfc12` (also `purrdf_rdf::rdfc12`). A caller that wants the 5.0
canonical output uses the default-family item in the third column, which is
exported from the crate root. Two of the functions below keep the name of a
default-family function: `purrdf_core::try_canonicalize` and
`purrdf_core::ir::try_canonicalize` run the labelling, while
`purrdf_core::rdfc12::try_canonicalize` (and `purrdf_core::ir::canon::try_canonicalize`)
runs RDFC-1.0.

| 4.x item | Returned | Default family (5.0) |
|---|---|---|
| `try_canonicalize_with(ds, hash, control)` | `Canonicalized` | `try_canonicalize(ds, control)`; the hash argument is gone |
| `try_canonical_sha256_view(view, hash, control)` | SHA-256 `ContentDigest` | `try_canonical_digest_view(view, control)`, a 32-byte BLAKE3 digest |
| `try_canonical_blake3_view(view, hash, control)` | 32-byte digest | `try_canonical_digest_view(view, control)` |
| `try_canonicalize_sha256_view(view, hash, control)` | document and digest | `try_canonicalize_digest_view(view, control)` |
| `try_canonicalize_flat_view(view, hash, control)` | flat document | `try_canonicalize_checked_view(view, control)` |
| `try_canonicalize_flat_graph_view(view, graph, hash, control)` | flat document of one graph | `try_canonicalize_checked_graph_view(view, graph, control)` |
| `check_admissible_flat_view(view)` | admission check | `check_admissible_checked_view(view)` |
| `try_flat_digest_view(view, hash, control)` | flat SHA-256 digest | `try_canonical_digest_checked_view(view, control)` |
| `try_flat_blake3_view(view, hash, control)` | flat 32-byte digest | `try_canonical_digest_checked_view(view, control)` |
| `RDFC_V2_CORPUS_POLICY` | `SearchPolicy::Finite(1_000_000)` | `CANON_CORPUS_POLICY`, the same bound for the engine's work units |
| `CANON_PROFILE_ID`, `CANON_PROFILE_VERSION` | the 4.x profile name and version | `CANON_LABELLING_NAME`, `CANON_LABELLING_VERSION` |
| `CANON_PRESENTATION_OVERLAY_ID`, `CANON_PRESENTATION_OVERLAY_VERSION`, `CANON_PRESENTATION_FLAT_ASSERTION_ID`, `CANON_PRESENTATION_FLAT_ASSERTION_VERSION` | the 4.x presentation names | `CANON_PRESENTATION_LINES_ID`, `CANON_PRESENTATION_LINES_VERSION` |
| `CanonHash` | the hash choice | none: the default digest is always BLAKE3 over the line sequence |
| `CanonPresentation` | overlay or flat form | none: there is one line-sequence presentation |
| `ReservedVocabulary`, `RESERVED_NAMESPACE` | the refused reserved vocabulary | none: the default entry points do not refuse it |
| `CANON_CORPUS_DIGEST` | digest of the 4.x corpus | none |
| `RdfcActiveGroup`, `RdfcActivePath`, `RdfcCandidateProgress`, `RdfcGroupLabels`, `RdfcGroupObservation`, `RdfcGroupObservations`, `RdfcGroupStats`, `RdfcRefusalObservation`, `RdfcRootProgress`, `RdfcSearchContext` | the 4.x search types | none: the labelling reports its work through `SearchWork` and `SearchReceipt` |

The functions that kept their root names (`try_canonicalize`,
`try_canonicalize_view`, `try_canonicalize_graph_view`, `try_graph_digest_view`,
`check_admissible`, `check_admissible_view`, `canonical_relabel` and
`canonical_relabel_with_mapping`) now run the labelling. Their RDFC-1.0
counterparts of the same names are under `rdfc12`. Three of them changed
signature, and one relaxed a bound:

| Root function | 4.x signature | 5.0 signature | A 4.x caller |
|---|---|---|---|
| `try_canonicalize_view` | `(view, hash, control)` | `(view, control)` | Drop the `hash` argument; to keep the 4.x result call `rdfc12::try_canonicalize_view(view, hash, control)` |
| `try_canonicalize_graph_view` | `(view, graph, hash, control)` | `(view, graph, control)` | Drop the `hash` argument; the 4.x result is `rdfc12::try_canonicalize_graph_view(view, graph, hash, control)` |
| `try_graph_digest_view` | returns SHA-256 `ContentDigest` | returns BLAKE3 `Digest32` | Read the 32 bytes through `Digest32`; the 4.x SHA-256 value is `rdfc12::try_graph_digest_view(view, graph, control)` |
| `check_admissible_view` | `D: DatasetView<ReadError = Infallible>`, returns `Result<(), CanonError>` | `D: DatasetView`, returns `Result<(), CanonError<D::ReadError>>` | No change for a view whose read error is `Infallible`; it now admits any view and reports its read error |

`try_canonicalize`, `check_admissible`, `canonical_relabel` and
`canonical_relabel_with_mapping` keep their signatures and change only what they
compute.

## Public types that changed shape

The canonicalization and compaction-certificate types below, and the geo index
identity, changed shape
between `rust-v4.0.2` and 5.0. Enums marked `#[non_exhaustive]` already require
a wildcard arm in a `match`, so an added variant there compiles unchanged; those
are listed last.

Exhaustive enums that gained variants. A 4.x `match` on one of them stops
compiling until it handles the new variant. The enums themselves keep their
4.x variants and payloads.

| Enum (`purrdf_core`) | New variant | A 4.x caller |
|---|---|---|
| `CanonOutput` | `LinesDigest`, `DocumentLinesDigest` | Add arms for both. They are the digest of the canonical line sequence, alone and together with the document and label table; the 4.x arms keep their meaning |
| `SearchConfiguration` | `Labelling { version, selection, output }` | Add an arm. It names the labelling run, where `Rdfc { .. }` names an RDFC-1.0 run; a caller that only inspects 4.x runs can bind it with a wildcard |
| `SearchEvent` | `LabellingUnit` | Add an arm. It counts the limited units of the labelling, read through `SearchWork::count(SearchEvent::LabellingUnit)`; the running total of the walk of 8.3: its total when the labelling completes, and on a refusal with `limit` its value after the charge that passed the limit, the refusal's `units` |

A variant payload that changed. `CanonError` is `#[non_exhaustive]`, but a
payload is matched by shape:

| Variant | 4.x payload | 5.0 payload | A 4.x caller |
|---|---|---|---|
| `CanonError::BudgetExceeded` | `BudgetExceeded` | `Box<BudgetExceeded>` | Field and method access through a binding still compiles by auto-dereference. Where the value is moved or named by type, write `*budget`; through a reference, `**budget`. A pattern that destructures the payload's fields matches `budget` and reads the fields from it. Construction wraps the value with `Box::new`, and the conversion is `From<Box<BudgetExceeded>>`, replacing `From<BudgetExceeded>` |

A method whose return type changed:

| Method | 4.x return | 5.0 return | A 4.x caller |
|---|---|---|---|
| `Canonicalized::presentation` | `CanonPresentation` | `Option<rdfc12::CanonPresentation>` | Handle `None`, which a result of the default family returns because its line sequence has one presentation (`CANON_PRESENTATION_LINES_ID`). A result of an `rdfc12` function returns `Some` with the 4.x value |

Variants added to enums that were already `#[non_exhaustive]` (no change
needed where a wildcard arm exists): `CanonError::Refused`,
`CanonError::Lowering` and `CanonError::SourceInconsistent`; `purrdf_rdf::gts_certify::CertifyError::CanonMismatch`;
`purrdf_rdf::gts_certify::CertificateCborError::UnknownCanon` and `UnexpectedCanon`.

Retyped geo index identity: `GeoIndex::source_fingerprint` returns `Digest32`
instead of `u64`, and the payload of `IndexBindingError::Source` is two
`Digest32` values instead of two `u64` values (see "Geo index source identity").

No canonicalization or compaction-certificate struct with public fields gained,
lost or retyped a public field, and none of these enums lost a variant.

## Containers

- **Triple terms that reach themselves.** A `k:3` term that is reachable from its
  own components, through `tt` or through the first `reifies` row of its `rf`, states
  no triple. The reader keeps the `reifies` row
  and reports no diagnostic, where 4.x reported `DamagedFrame` and dropped the row;
  the term keeps a distinct identity in the multi-segment union and the canonical
  writer, and projections render it as the blank node `_:unbound_triple_{id}`.
  `dataset_from_gts_graph`, `flattened_dataset_from_gts_graph`, `import_gts_graph`
  and `GtsFoldView::new` accept such a graph instead of refusing it with
  `gts-self-reaching-term`, which now names only a literal datatype chain that
  leads back to itself. A caller that walks `Graph::triple_of` recursively reads
  the components the term declares, and one that must terminate on a caller-built
  graph takes `purrdf_gts::resolution::SelfReaching::of(&graph)` once and reads
  components through `Graph::resolved_triple_of`. The streaming importers and
  the N-Quads export decide the same set. Valid files are unchanged.
  `DictionaryError::Limit` with the resource `reifier-traversal-entries` is no
  longer produced.
- **`PendingResolution` is `#[non_exhaustive]`.** The streaming resolver keeps
  the search that finds self-reaching triple terms in the pending record, in the
  new field `component: ComponentState`. An external `PendingResolution { .. }`
  literal no longer compiles. A `ResolutionStorage` implementer changes only
  where it builds a record:

  ```rust
  // 4.x
  PendingResolution { term, parent: None, next: 0, active: false }
  // 5.0
  PendingResolution::new(term)
  ```

  A storage that persists records outside memory writes
  `pending.component.to_words()` (five `u64` words) beside `parent`, `next` and
  `active`, and reads them back with `ComponentState::from_words`, which returns
  `None` for words no state encodes. It must return the value it was given: a
  record read back with the default state is refused with
  `rdf-ir-dangling-term-ref`. No `ResolutionStorage` method is added.
- **Unsupported signing algorithm.** `verify_sig` and `verify_sig_with_key`
  return `SigStatus::Unverified` for a COSE_Sign1 whose declared `alg` this
  engine does not implement, where 4.0.2 returned `Invalid`; the file verifiers
  (`verify_signatures`, `verify_signatures_with_resolver` and the C,
  WebAssembly and Python surfaces over them) record it as `unverified`: the
  engine could not check it. `Invalid` remains for a signature that was checked
  and failed and for a malformed structure. The enum is unchanged. A caller that
  treated `Invalid` as the only failure outcome now sees `Unverified` for this
  case, and a caller that treated `Unverified` as "key not resolved, unchecked"
  now also receives it for an unsupported algorithm. A caller that requires a
  verified signature accepts only `SigStatus::Valid`. The certify callers
  (`gts_certify`, in the authorship and the packaging checks) refuse an
  unsupported algorithm at parse, before any status exists; `Unverified` is
  what a direct verification with `verify_sig` or `verify_sig_with_key` returns.
- **Reifier position in GTS.** The reader drops a `reifies` row whose reifier is
  a literal or a quoted-triple term, and diagnoses it `PositionConstraint`;
  4.0.2 kept the row without a diagnostic. A quoted triple stays legal as a
  `quads` subject and as the object of a `reifies` or `annot` row.
  A quoted-triple term whose `rf` names its own id breaks the specification's
  rule that `rf` names a strictly earlier term. 4.0.2 kept such an `rf` without
  a diagnostic, and already diagnosed an `rf` naming a later id as
  `ForwardReference`. The reader now diagnoses an `rf` naming its own id as
  `ForwardReference` too, and reads it as absent. The term then resolves by its
  `tt` when that is valid and states no triple otherwise, and a `reifies` row
  keyed by it is dropped with `PositionConstraint`. The row diagnostic carries
  the frame index, the reifier id and the row's triple.
  The import outcome follows from the new diagnostic. The event importer
  (`import_gts_events` and its dictionary and blob forms) fails the import on
  any reader diagnostic other than a blob-budget refusal, so it now refuses
  every file holding the spelling with `rdf-ir-gts-fold-diagnostic` carrying the
  `ForwardReference`, whether or not the term has a `tt` and whether or not a
  quad uses it; 4.0.2 imported such a file. `dataset_from_gts_graph` refuses
  a file whose term has no `tt` and is named by a quad or a row, with
  `native-codec-unbound-triple-term`; 4.0.2 imported that file too. Only the
  fold view reads the file, and it renders the term as a fresh blank node. The
  conforming spelling names an earlier IRI or blank-node reifier in `rf` and
  keys the row by that reifier. A file whose `reifies` row is keyed by a
  quoted-triple term, whatever its `tt` or `rf`, loses that row.
  A read-only count through the 4.0.2 reader over the fourteen stored
  first-generation files found no term with an `rf`, no triple-kind term, no
  self-bound term and no reader diagnostic, and their `reifies` rows involve no
  such term. The count has limits: writer versions are not recorded in the
  files' headers, and older archived versions of the live files were not read.
  The conformance vectors, 39 carried and 52 external, are a separate count and
  contain no writer output; none holds the spelling.
- **Damaged blob payload in GTS.** A blob frame that declares its digest and
  whose transformed payload cannot be a stream of its codec folds as an opaque
  node with a `DamagedFrame` diagnostic, and the blob is not in the blob table;
  4.0.2 admitted it and failed only when its bytes or size were requested. The
  fold checks the structure of `zstd`, `zstd-rsyncable` and `gzip` payloads (frame
  and block headers, a named dictionary, the gzip magic and trailer) without
  decoding them. A payload whose structure is sound but whose content is corrupt
  still folds as a blob and returns the codec failure on first request, as in
  4.0.2. A valid blob folds as before.
- **COSE_Sign1 protected header.** The protected header is read as the
  deterministic encoding the specification requires of every signed byte string
  (section 4, RFC 8949 section 4.2): shortest-form integers and lengths,
  definite-length items and map keys in bytewise order. Header integers are not
  bounded beyond what CBOR encodes, as in 4.0.2. Each case moves between the
  three outcomes as follows, for a caller that treats `Unverified` as
  "unchecked" and `Invalid` as "failed":
  - A header that is not deterministically encoded was `Valid` with a resolved
    key and `Unverified` without one in 4.0.2; it is `Invalid` in both cases.
    This moves a result away from `Valid`: a stored signature whose protected
    header is not deterministically encoded stops verifying. No writer in the
    GTS repository, in any released version, emits such a header, so only a
    caller-supplied signature (`FrameOptions::signature`) can hold one. Verify
    stored signed material with the 5.0 verifier before moving a pin.
    `Sign1Error::NonCanonicalProtectedHeader` and, in a file verification,
    `VerificationError::NonCanonicalProtectedHeader` name this cause, so it is
    told from a signature that fails to verify. `cose::signature_refusal` reads
    the cause of a retained envelope. The status stays `invalid`, and
    `Sign1Error` gains a variant and becomes `#[non_exhaustive]`: an exhaustive
    `match` on it no longer compiles and needs the new arm and a wildcard.
    `VerificationError` was already `#[non_exhaustive]` and gains a variant, so a
    `match` with a wildcard arm compiles unchanged.
  - Compaction and the certify callers parse each retained envelope with the
    same rules. A file whose signature has a header that is not deterministically
    encoded is refused by compaction
    (`CompactRefusedError::FrameCose`, or `DetachedCose` for a carried detached
    signature) and by the certify checks at parse, where 4.0.2 compacted it and
    carried the signature. Verify stored signed material before compacting it.
  - A header declaring its algorithm as a text string (RFC 9052 section 3.1)
    was malformed and `Invalid` with no `kid`; it is `Unverified` with its `kid`,
    as an integer algorithm this engine does not implement is.
  - The `kid` of an envelope that declares an unsupported algorithm is
    reported, where 4.0.2 reported none.
    The status is unchanged.
  A COSE_Sign1 with no `kid` is unchanged: `kid` stays optional (RFC 9052
  section 3.1) at every entry point. A short EdDSA signature stays `Invalid`
  before key resolution.
- **PACK.** Format version `1` means a 4.x digest. The 5.0 writer emits format
  version `2`, whose `rdfc_digest` header field is the line-sequence digest. The
  reader accepts both, and `verify_pack` recomputes each version with the function
  that recorded it; a version `1` `rdfc_digest` of a dataset holding U+FFFE or
  U+FFFF in a literal does not recompute. Because the line sequence holds a line for each declared empty
  graph, a pack that declares an empty graph now has a different digest from the
  same pack without the declaration; under version `1` they were equal.
- **GTS compaction certificates.** A certificate's canonical CBOR map carries a
  `refold_canon` text field naming the function that computed its refold digests:
  `rdfc10-sha256` (the 4.x RDFC-1.0 flat form) or `nrl1-lines-sha256` (SHA-256 of
  the default-family canonical document). An absent field means `rdfc10-sha256`,
  so every certificate written by 4.x reads unchanged (its refold digests still
  recompute, except for a dataset holding U+FFFE or U+FFFF in a literal), and an
  `rdfc10-sha256`
  certificate is written without the field, byte for byte as 4.x wrote it.
  `compact_and_certify_versioned` writes `nrl1-lines-sha256` and returns a
  `VersionedCertificate { canon, certificate }`; `compact_and_certify` keeps its
  signature and the `rdfc10-sha256` certificate. `verify_compaction_with`,
  `refold_digest_with` and `effective_digest_with` recompute with the function
  named, and the same value governs `effective_digest`. An unknown `refold_canon`
  value is the typed refusal `rdf-certificate-cbor-unknown-canon`, never a
  fallback. `compose_versioned` refuses certificates that name different
  functions (`rdf-certificate-canon-mismatch`) and composes those that agree.
  `CompactionCertificate::from_canonical_cbor` refuses a certificate that is not
  `rdfc10-sha256` (`rdf-certificate-cbor-unexpected-canon`); read a certificate of
  either kind with `VersionedCertificate::from_canonical_cbor`. The digest the
  pack embeds as `stream:contentRefoldDigest` stays the RDFC-1.0 digest (it now
  follows profile version 4 for such a dataset), because
  the GTS wire format is governed in `gmeow-gts`; both writers author the same
  pack bytes.
- **Segmented images.** The writer emits format version `8` (`PURRSEG8`), whose
  source checkpoint stream holds the canonical BLAKE3 Merkle node of every
  source-directory branch of at least 32 members instead of additive prefix
  sums. The header layout is unchanged apart from its magic and version. The
  reader accepts version `7` (`PURRSEG7`) and version `8` and refuses any other.
  Certification checks a version `7` image's additive checkpoints against its
  rows, and the change feed compares its ranges by hashing their records, so it
  reads more directory records than a version `8` image of the same rows. An
  image written by 5.0 has a different snapshot identity and receipt from the
  4.x image of the same input, and a 4.x reader refuses it. Rewrite a version
  `7` image from its RDF input to restore checkpointed range comparison. A
  range whose digests are all equal still visits each occurrence when the
  compared sides hold different counts.
- **Columnar.** The five-table Parquet files record no canonical digest, only
  content digests of blobs and of the manifest. They are unchanged.
- **Embedding (PURREMB).** The `SOURCE` section records a certified RDF digest
  and the blank-node target identities carry canonical labels. Format version `1`
  means the 4.x run: the RDFC-1.0 SHA-256 digest and `c14nN` labels (a source digest
  of a dataset holding U+FFFE or U+FFFF in a literal does not recompute). The 5.0
  writer emits format version `2`, whose digest is the line-sequence digest and
  whose labels are the labelling's decimal labels. Nothing else in the framing or
  the sections changes. The reader accepts both versions and refuses any other,
  and a file whose header and trailer name different versions. Source
  verification recomputes the digest and the labels with the run the version
  names, so a 4.x embedding keeps verifying against the pack or Columnar files it
  was written for, except for a dataset holding U+FFFE or U+FFFF in a literal,
  whose version `1` source digest no longer recomputes. Version `1` blank-node
  labels (`c14nN`) can also move for such a dataset, because the first-degree
  hashes of RDFC-1.0 include the spelling of an adjacent literal. The same file with its version byte changed to `2` is refused
  as a certified-RDF digest mismatch. A newly written artifact differs from the
  4.x artifact of the same inputs in the header and trailer version fields, the
  source digest, the dataset target identity, every blank-node target identity
  and everything sealed over them. The 4.x reference file `purremb_v1.bin` is
  kept as the 4.x evidence beside the new `purremb_v2.bin`. A 4.x reader
  (checked against `EmbeddingView::from_bytes` at the `rust-v4.0.2`
  tag, which compares the header version with the constant `1` before it reads
  any other field) refuses a version-`2` file with `UnsupportedVersion(2)`
  (`purremb-unsupported-version`, "unsupported PURREMB version 2"); it does not
  misread one. Re-embedding is not needed to keep a 4.x file readable by 5.0, but
  a 5.0 file is not readable by 4.x.

## Hosts

- **Labels and receipts.** Every host returns `_:N` labels. Receipts name the
  labelling, its version and the `line-sequence` presentation.
- **The bound.** The `canonical_search` argument and `--canonical-search` map onto
  the engine's typed limit: a finite value is a number of limited work units of
  the walk, and `unlimited` is the engine's own limit. A finite bound that the
  input exceeds is refused with the labelling's `limit` outcome, and no canonical
  bytes are published. An input the labelling can finish without a search
  succeeds under every bound, including `0`.
- **Rust.** `CanonError::Refused` carries the labelling's typed refusal
  (`input`, `graph`, `width` or `limit`). `CanonError::Lowering` carries the
  refusal of the lowering of nested triple terms, `LoweringRefusal::TermNode`,
  when two different nested terms share a term-node digest, a collision of
  BLAKE3. A caller that labels several inputs as one read with
  `label_view_with_capacity` also meets `LoweringRefusal::Size` and
  `LoweringRefusal::Weight`, judged against its `LabellingCapacity`. The new constants are
  `CANON_LABELLING_NAME`, `CANON_LABELLING_VERSION`,
  `CANON_PRESENTATION_LINES_ID`, `CANON_PRESENTATION_LINES_VERSION` and
  `CANON_CORPUS_POLICY`. The 4.x items that left the crate root are listed in
  "Items that left the crate root".
- **Flat presentation.** `canonical_flat_nquads` returns the line-sequence
  document. The identifier `flat-assertion`, version `1`, now names only the 4.x
  flat form reached through `rdfc12`; the default flat document reports
  `line-sequence`, version `1`.
- **Python, WebAssembly, C and the CLI.** Each routes to the default family.
  Their canonical documents, digests and labels change as above, and a refusal
  under a finite bound is the `limit` refusal.

## The HTTP exchange

`purrdf_sparql_eval::HttpTransport` and its request type `HttpRequest` are
removed. A host implements `HttpExchange` instead, and the same value serves
`SERVICE` through `HttpRemoteQuerySource` and `LOAD` through the new
`HttpGraphResolver`.

`HttpExchange::exchange` takes an `HttpExchangeRequest` and returns
`Result<HttpOutcome, HttpFailure>`. The request carries the method, the
target, the ordered headers, the body, the timeout and the stop signal. The
4.x request fields map onto it as follows:

| 4.x `HttpRequest` | 5.0 `HttpExchangeRequest` |
|---|---|
| `endpoint` | `target` |
| `query_text` | `body` (UTF-8 bytes of the query text); `method` is `HttpMethod::Post` |
| `content_type`, `accept`, `user_agent` | `fixed_headers`, in that order, named `content-type`, `accept` and `user-agent`; read one with `fixed_header` |
| `headers` | `profile_headers`, sent after the fixed headers; `headers()` yields both runs in order |
| `timeout`, `stop` | `timeout`, `stop`, unchanged |

The outcome is one of three values:

- `HttpOutcome::Response(HttpResponse)`: the status (`HttpStatus`, 100
  through 599), the response headers, the body, and `final_url`, the URL the
  host finally fetched when it followed redirects itself.
- `HttpOutcome::OpaqueRedirect`: a redirect whose status and `Location` the
  host cannot see, as a browser reports for a request it was told not to
  redirect. `SERVICE` refuses it as `RemoteError::OpaqueRedirect` and `LOAD` as
  `LoadError::OpaqueRedirect`. `LoadError::OpaqueRedirect` reports under the code
  `native-sparql-load-failed`, which only `LOAD` uses; the `SERVICE` refusal reports
  under `native-sparql-service-failed`. Both
  are failures of the invocation, so `SERVICE SILENT` and `LOAD SILENT` absorb
  them as transport failures.
- `HttpOutcome::Dataset`: a dataset the host already holds, for `LOAD`.

A host failure is an `HttpFailure`, never an HTTP status. A 4.x host that
returned a `RemoteError` returns the matching failure:

| 4.x `post` returned | 5.0 `exchange` returns |
|---|---|
| `Ok(body)` | `Ok(HttpOutcome::Response(HttpResponse::new(HttpStatus::OK, body)))` |
| `Err(RemoteError::Transport(m))` | `Err(HttpFailure::Transport(m))` |
| `Err(RemoteError::HostDenied { message, .. })` | `Err(HttpFailure::Denied(message))` |
| `Err(RemoteError::HostFault { message, .. })` | `Err(HttpFailure::Fault(message))` |
| `Err(RemoteError::Decode(m))` | `Err(HttpFailure::Decode(m))` |
| `Err(RemoteError::Governed(g))` | `Err(HttpFailure::Governed(g))` |
| `Err(RemoteError::GovernedAfterCompletion(g))` | `Err(HttpFailure::GovernedAfterCompletion(g))` |
| `Err(RemoteError::ExchangeIdExhausted)` | `Err(HttpFailure::ExchangeIdExhausted)` |

`SERVICE` and `LOAD` report each failure as the variant they reported in 4.x.
A `SERVICE` response whose status is not a success is
`RemoteError::Transport`, so a 4.x host that mapped an error status to
`RemoteError::Transport` may return the response as it is.

`post_admitted` is renamed `exchange_admitted`, and `workspace_certificate`
takes the `HttpExchangeRequest`. A closure
`Fn(HttpExchangeRequest) -> Result<HttpOutcome, HttpFailure>` is an exchange,
as a closure over the 4.x request was a transport. The field of
`HttpRemoteQuerySource` holding the exchange is private, as before.

`LOAD` over the exchange is `HttpGraphResolver::new(exchange, decoder, accept)`,
where `decoder` is a `DocumentDecoder` (or a closure over body, media type and
base) that parses a fetched document; `purrdf-sparql-eval` carries no RDF
parser. It authorizes every hop it follows against an optional `ServiceCatalog`,
follows up to `MAX_LOAD_REDIRECTS` (five) redirects itself, and parses a document
against the response's `final_url`; when the host reports none, the base is the
IRI that was requested for that hop. A relative `Location` resolves against the
same base. A host must return each redirect to the resolver for every
destination to pass catalog authorization: a redirect the host follows
internally is never shown to the resolver. Each hop after the first is a host
fetch charged against the `RemoteRequests` ceiling through
the provided `GraphResolver::resolve_admitted` method and its `FetchAdmission`
argument (a resolver that wraps another forwards it), and the stop signal is polled after every
completed hop.

The WebAssembly package keeps its JavaScript interface: the effects and the
`deliver*` methods are unchanged. A document delivered with `deliverGraph` and
no base is now parsed against the IRI of the `LOAD` effect that fetched it,
where 4.x parsed it with no base, so a document with relative IRIs that 4.x
refused now loads.

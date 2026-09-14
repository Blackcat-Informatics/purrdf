<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Ordered JSON byte-cover contract, version 1

## Language and resource limits

The input grammar is [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259) §§2–7,
encoded as UTF-8 without BOM. Whitespace is exactly space, TAB, LF and CR.
Member name escapes must decode to Unicode scalar values: a high surrogate
must immediately precede an escaped low surrogate, and an unpaired low surrogate
is refused, because RDF cannot represent such a key in an RFC 6901 path. Scalar
string values retain every grammar-valid escape verbatim, including unpaired
surrogate escapes. Duplicate
object member names are accepted and never overwrite a previous occurrence.
Numbers retain their grammar-valid lexical spelling without numeric conversion.

Profiles carry four limits, checked before exceeding them:

| Limit | Standard offer | Valid profile range |
| --- | ---: | ---: |
| Source bytes | 16,777,216 | 1–4,294,967,295 |
| Value occurrences | 1,048,576 | 1–1,048,576 |
| Nested container depth | 128 | 1–128 |
| Sum of stored pointer bytes | 67,108,864 | 1–4,294,967,295 |

A scalar root has depth zero; its containing array or object adds one. The last
bound prevents repeated deep pointer prefixes from causing unbounded expansion
relative to the declared profile. A decoder admits at most twice the value
limit plus two codec subjects, the maximum possible occurrence/cover node set.
Source length and each bound use fixed-width encodings in the profile identity,
so native and WebAssembly targets implement the same contract.

## Source, occurrences and cover

The caller supplies an absolute, well-formed source IRI and original bytes.
The parser emits every JSON value in preorder, assigning consecutive zero-based
occurrence numbers. A value records its parent's occurrence and its zero-based
member/element ordinal. The root's parent is the document and its ordinal is
zero. Containers also record their immediate element or member count, with
duplicate members counted separately. The path uses [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) §3: decode
object keys, then escape `~` as `~0` and `/` as `~1`. Array components are decimal
indices. A duplicate member name can produce equal paths; occurrence identity
and parent/ordinal disambiguate every value.

Scalar lexical spans cover their exact source bytes. String spans exclude both
quotes and preserve all escapes verbatim. Number and keyword spans include the
whole token. Empty strings have zero-length scalar spans. Containers have their
complete syntax span as structural metadata but own no bytes in the cover.

Structure runs are the maximal complement of the scalar spans in source order,
computed by a single forward scan. They carry every remaining byte: whitespace,
delimiters, keys and their original ordering. Zero-length scalar boundaries
separate adjacent complement runs. Each run has a consecutive zero-based run
number. The source bytes occur once across nonempty covering spans; there is no
second whole-source literal or tree copy.

## Vocabulary and RDF shape

A vocabulary is an explicit namespace ending in `/` or `#`. The named standard
offer is `https://w3id.org/purrdf/json#`. Append these local names in this exact
order to derive its terms:

```text
Document Value Structure source sourceDigest byteLength profile document
byteStart byteEnd text path kind occurrence parent ordinal digest verbatimText verbatim size
```

`Document`, `Value` and `Structure` are classes; `digest` and `verbatimText` are
datatypes; the other terms are predicates. `rdf:type` and the standard
`xsd:string` and `xsd:integer` datatypes retain their standard meanings.
All codec statements live in the default graph. Every table cell below denotes
exactly one required fact; no extra predicate is admitted on a selected subject.
Identical duplicate RDF statements are one fact under RDF set semantics.

| Subject class | Required facts |
| --- | --- |
| Document | `rdf:type Document`; `source` IRI; `sourceDigest` typed `digest`; `byteLength` integer; `profile` string |
| Value | `rdf:type Value`; `document` IRI; `byteStart`, `byteEnd`, `occurrence`, `ordinal` integers; `parent` IRI; `path`, `kind` strings |
| Scalar Value | All Value facts plus `text` string |
| Container Value | All Value facts plus `size` integer |
| Structure | `rdf:type Structure`; `document` IRI; `byteStart`, `byteEnd` integers; `verbatim` typed `verbatimText` |

Kinds are exactly `string`, `number`, `true`, `false`, `null`, `array` and
`object`. All integer lexical forms are canonical unsigned decimal within
`u64`: zero is `0`, otherwise no leading zero, sign or whitespace. String
metadata has `xsd:string` datatype and no language or direction. A digest is
`sha256:` followed by 64 lowercase hexadecimal characters. Profile metadata is
the profile digest's 64 lowercase hexadecimal characters, without a prefix.

## Identity

`frame(s)` is the UTF-8 byte length as eight little-endian bytes followed by
the UTF-8 bytes. Hashing is SHA-256. Profile canonical bytes concatenate:

1. Framed strings `purrdf-json-profile-v1`,
   `rfc8259-utf8-no-bom-scalar-member-names`,
   `preorder-occurrences-rfc6901-duplicates`, `scalar-lexical-cover-sha256-sized-containers-fragment-v-s`,
   the caller's profile name, and the namespace.
2. The nonzero profile version as little-endian `u32`.
3. Source-byte limit as `u64`, value limit as `u32`, depth limit as `u16`,
   pointer-byte limit as `u64`, all little endian.
4. Each local name listed above, framed individually in its stated order.

The profile identity is SHA-256 of those bytes. A document's identity preimage
is framed `purrdf-json-document-v1`, framed source IRI, the raw 32-byte profile
digest, then the raw 32-byte source digest. Let `root` be the namespace prefix
before its first `#`, or the complete namespace when it has no `#`. Append `/`
unless `root` already ends in `/`, then `document-` and the lowercase hexadecimal
SHA-256 of the document preimage. This is the document IRI. Value IRIs append
`#v` + canonical decimal occurrence number; structure IRIs append `#s` +
canonical decimal run number. The full profile preimage still includes the
complete vocabulary, including any fragment. Query-bearing and opaque IRI
namespaces follow the same string derivation; no scheme or hostname is inferred.
The named standard profile uses name `purrdf-json-ordered`, version 1 and the standard limits.
Content and profile binding keep different revisions of the same source IRI
distinct when their graphs are merged. The full 256-bit digest is retained.

## Decode validation

The caller selects a document IRI and expected profile explicitly. The reader
first checks the selected document metadata through an indexed subject lookup,
refusing a profile mismatch or excessive byte length before allocating the owned
subgraph. It then collects that subject and every subject stating its `document` ownership edge
across the base, RDF 1.2 reifier and annotation tables. Identical assertions
across tables are one fact. A selected codec subject used as a named graph is
refused, including an otherwise empty graph declaration.
Non-IRI ownership subjects, ownership in named graphs, extra predicates,
repeated predicates with distinct objects, missing facts, incorrect datatypes,
unknown classes/kinds and malformed numeric metadata all refuse. Other
documents' subjects may coexist in the same dataset.

The reader reconstructs from scalar `text` and structure `verbatim` spans using
`purrdf_core::cover::reconstruct`. No overlap is declared by this codec. The
kernel refuses reversed/out-of-bounds spans, length mismatches, gaps, any
overlap, invalid UTF-8 and a source digest mismatch. Allocation is bounded by
the supplied span text and the profile's source-byte maximum.

The reader then parses the reconstructed bytes under the expected profile and
checks the document identity, complete node set, every node identity, class,
path, kind, occurrence, ordinal, parent, container size and offset against the parsed model.
This step prevents correct covering bytes from authenticating fabricated
structural assertions. A removed container or ownership edge, invented path,
wrong scalar kind or alternate cover partition is a refusal even if the digest
matches. Only after every check succeeds does the function return the bytes.

## Verification and profiling

The frozen first-party corpus in `tests/roundtrip.rs` has a fixed nonzero case
count and crosses real Turtle, N-Triples and JSON-LD serialization and parsing.
It covers ordering, duplicates, empty values/containers, control escapes,
Unicode, nesting and unusual numeric spellings. Adversarial tests remove every
required fact and alter types, cardinality, ownership, offsets and structure.
The same tests execute on WebAssembly; the complete framed RDF output corpus
has a pinned digest. Turtle is tested both with absolute IRIs and with an
explicit document base, which factors repeated document identities into one
`@base` declaration and leaves compact fragment references. A SPARQL regression joins parent, path and size facts while
retaining duplicate member occurrences and raw escaped string spellings.

The latency and allocation profiling executables share deterministic generated inputs. Criterion
reports parse, projection, combined encoding and verified decode latency.
The separate allocation probe reports allocation count, requested bytes,
retained bytes, peak working bytes, RDF statement count and serialized size.
These are measurements, without timing assertions or claimed speedups.

A third native harness measures the same forward parser at three input sizes,
or traverses a caller-supplied JSON corpus and measures accepted input bytes
against production Turtle bytes. Every accepted file must survive Turtle
serialization, reparse and exact byte decoding. Refusals are reported separately;
I/O errors and an empty acceptance set fail the run. Symlinks are counted and
skipped, and `.git`, `target` and `node_modules` directories are excluded.

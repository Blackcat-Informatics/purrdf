<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Native YAML source carrier

`purrdf-yaml`, also available as `purrdf::yaml`, describes original UTF-8 YAML
presentation as RDF 1.2 metadata and reconstructs it strictly. `analyze` owns the
existing Lex `SyntaxStream`; it does not build another syntax arena. `project`
and `encode` return a native `RdfDataset` by value. `decode_stream` requires an
expected carrier root and caller profile, reconstructs through the shared Core
cover law, reparses with Lex and compares the complete selected metadata graph.

A `SourceDocument`, `Profile` and `Vocabulary` are explicit. `Profile::standard`
and `Vocabulary::standard` deliberately select the shipped policy and
`https://w3id.org/purrdf/yaml#`; analysis never supplies them implicitly. A caller
namespace must be an absolute IRI ending in slash or hash. Names are limited to
1,024 UTF-8 bytes, namespaces to 2,048 bytes and source/root IRIs to 4,096 bytes.
Profile versions are nonzero. All phase counts must fit unsigned 32-bit values;
zero ceilings are legitimate refusal policies.

The source model is the native YAML 1.2.2 acyclic subset. It retains complex and
repeated authored mapping keys through deferred lexical ingestion. Tags and
merge candidates remain presentation metadata. Non-finite numeric spellings
remain lexical text. Analysis does not call Core scalar construction, YAML-LD
Basic processing or JSON-LD expansion. Cyclic/in-progress and unknown aliases
retain the original positioned Lex refusals. Presentation changes alter the
carrier identity; they do not alter the separate YAML-LD semantic dataset.

## Identity and schema law

The schema identifier is `yaml-1.2.2-acyclic-lexical-stream-cover-v1`. Its exact
identity and vocabulary law is specified in [SPEC.md](SPEC.md). `source_digest`
is Core `ContentDigest`, SHA-256 over the complete original bytes. `lex_source_id`
is explicitly Lex BLAKE3 over those bytes. They are separate algorithms and
separate RDF fields. Source/profile/ordinal identities are byte-source
identities, not RDF literal-row fingerprints. Any actual literal-row identity
belongs to `purrdf_core::source::LiteralSourceRow`.

Syntax spans may overlap: a block scalar contains its header comment, and a
collection contains child occurrences. `cover_spans` is a separate complete
partition. It contains each nonempty raw scalar/alias span and the maximal
complementary runs between them. Empty streams and omitted scalars invent no
bytes. The shared Core cover law verifies the generated partition and performs
all reconstruction; the YAML crate has no private coverage checker.

## Admission and strict decoding

`Bounds` declares independent phase accounts:

| Account | Included storage |
| --- | --- |
| `syntax` | Whole source input, document/occurrence counts, decoded text, native parser/anchor/tag/trivia state and depth, through Lex |
| `max_partition_bytes` | Raw/partition span arrays, framed stream identity and shared cover verification scratch/output, additional to retained Lex |
| `max_projection_bytes` | RDF interning/builder/index buffers, temporary spelling, node lookup and freeze storage, additional to the source model |
| `max_decode_bytes` | Selected subject/statement/span arrays, indexes, prefix text and shared cover reconstruction/output, additional to caller RDF |

`max_cover_spans` bounds the partition separately. `max_statements` bounds emitted
or selected metadata rows. `max_payload_bytes` bounds the sum, per metadata
statement before deduplication, of subject IRI bytes, predicate IRI bytes and
object IRI bytes or literal lexical-plus-datatype bytes. Metadata literals have
no language or direction. Profile/vocabulary configuration has fixed bounded
text and uses fallible resident construction, outside the document phase
accounts. Digest-mismatch reporting likewise has its own fixed-size, fallible
resident strings, independently of refused reconstruction scratch.

These are additive phase ceilings, not one combined allocation claim. During
reconstruction the caller RDF and selected metadata/source remain alive while
Lex analysis and expected RDF reprojection use their own respective bounds.
The expected native RDF dataset is not serialized to compare it. Borrowed
term lookups and admitted statement indexes compare the selected graph exactly.

Selection includes the root, every subject with the caller vocabulary's `stream`
ownership edge to it, and every subject in its `root#` identity namespace.
Deleting an ownership edge cannot conceal a forged namespace subject. Selected
metadata must be in the default graph; selected/namespace named-graph
declarations are refused, even if empty. Independent carriers can coexist.
Conflicting required fields, extra/missing claims, unknown predicates, wrong
datatypes, forged spans, aliases, ordinals, identities or structural metadata
fail before bytes return. The cover/digest, source reparse, recomputed root and
complete reprojection must all agree. Original syntax, storage, cover, profile,
RDF-builder and metadata failures remain typed in `YamlError`.

Native acceptance exercises representative multi-document presentation and RDF
transport, absent/empty documents, complex/repeated keys and shadowed anchors,
deterministic identities, semantic presentation equivalence, hostile metadata,
phase ceilings, the shared admitted cover seam and the actual umbrella API.

## License

Licensed under `MIT OR Apache-2.0 OR MulanPSL-2.0`, at your option.

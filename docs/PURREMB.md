<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PURREMB v1: deterministic embedding companions for PurRDF packs

## 1. Scope and conformance

PURREMB is the binary format carried by files whose conventional extension is
`.purremb`. A PURREMB artifact is a deterministic, memory-map-friendly embedding
projection over one exact `.purrpck` source artifact. It binds vector rows to stable
subjects, records the complete vector-generation contract, and carries integrity
evidence without changing the source RDF dataset, its RDFC identity, or the
`.purrpck` wire format.

The exact stored vector matrix is authoritative for the embedding projection.
Approximate indexes are derived accelerators. Neither the matrix nor an index is
part of RDF canonical identity.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, and **MAY** are normative.

A conforming v1 writer MUST emit the one canonical representation defined here. A
conforming v1 reader MUST reject every violation called out as an error; it MUST
NOT repair, reorder, reinterpret, or silently truncate malformed input.

PURREMB is vocabulary-neutral. Numeric tags in this document are binary format
codes, not RDF vocabulary terms. Every IRI, role, media type, algorithm identifier,
policy reference, and extension identifier placed in an artifact is supplied by
the caller. PurRDF supplies no ontology-specific identifier or default vocabulary.

## 2. Design boundary

PURREMB v1 provides:

- exact binding to all bytes of one `.purrpck` source and to its independently
  certified RDFC digest;
- corpus, document, and chunk subjects whose text remains external and
  content-addressed;
- RDF 1.2 dataset, graph, statement, reifier, annotation, and term subjects,
  including directional literals and recursive triple terms;
- deterministic hierarchy relations and tokenizer-specific spans;
- complete model, engine, tokenizer, execution, projection, preprocessing,
  chunking, pooling, normalization, truncation, numerical, and metric contracts;
- dense row-major finite IEEE-754 `f32` and `f64` matrices;
- fixed-dimension and Matryoshka leading-prefix vector families;
- exact external-artifact bindings and opaque ANN guards;
- borrowed access from any immutable byte slice, including a caller-owned memory
  map, without a filesystem or mmap dependency in `purrdf-core`.

PURREMB v1 does not store corpus text, select or train a model, define domain
semantics, perform ANN search, reconstruct source content, sign or encrypt bytes,
or mint policy vocabulary. Quantized, compressed, sparse, strided, and non-little-
endian authoritative matrices are not v1 encodings.

## 3. Primitive encodings

### 3.1 Integers, offsets, and arithmetic

All integer fields are unsigned little-endian integers of the stated width. All
offsets are absolute from the beginning of their containing file or section, as
explicitly stated. Half-open spans use `[start, end)`.

Readers MUST perform checked arithmetic for every addition, multiplication,
alignment operation, and integer-to-`usize` conversion before slicing or
allocating. A value that cannot be represented by the host is an error. Counts
from the file MUST NOT directly control an allocation proportional to matrix rows,
target rows, or payload length; fixed tables and pools are borrowed from the input.

The function used throughout this document is:

```text
align_up(value, alignment) = (value + alignment - 1) & ~(alignment - 1)
```

Its operands MUST be checked for overflow before evaluation. File sections are
aligned to `FILE_ALIGNMENT = 64`. Internal fixed tables and TLV values are aligned
to 8 bytes where stated.

### 3.2 Digests and domain separation

`SHA256(bytes)` means the ordinary 32-byte SHA-256 digest of `bytes` with no text
or hexadecimal transformation. Hexadecimal is presentation only and is never
stored where this specification says `digest32`.

Typed identities use the following unambiguous fold:

```text
H(domain; field_1, ..., field_n) = SHA256(
    domain ||
    u64le(len(field_1)) || field_1 ||
    ... ||
    u64le(len(field_n)) || field_n
)
```

Every domain below is the exact lowercase ASCII byte string shown, including its
terminal NUL byte:

| Name | Domain bytes before the terminal NUL |
| --- | --- |
| `D_ARTIFACT` | `purrdf.purremb.v1.artifact` |
| `D_FAMILY_CONTRACT` | `purrdf.purremb.v1.family-contract` |
| `D_FAMILY` | `purrdf.purremb.v1.family` |
| `D_CHUNKING` | `purrdf.purremb.v1.chunking` |
| `D_SPACE` | `purrdf.purremb.v1.vector-space` |
| `D_TARGET_IDENTITY` | `purrdf.purremb.v1.target-identity` |
| `D_TARGET` | `purrdf.purremb.v1.target` |
| `D_TARGET_SET` | `purrdf.purremb.v1.target-set` |
| `D_RELATION_ROLE` | `purrdf.purremb.v1.relation-role` |
| `D_MATRIX_CONTENT` | `purrdf.purremb.v1.matrix-content` |
| `D_MATRIX` | `purrdf.purremb.v1.matrix` |
| `D_PROJECTION_CONTENT` | `purrdf.purremb.v1.projection-content` |
| `D_PROJECTION` | `purrdf.purremb.v1.projection` |
| `D_EXTERNAL_CONTRACT` | `purrdf.purremb.v1.external-contract` |
| `D_EXTERNAL` | `purrdf.purremb.v1.external-binding` |
| `D_INDEX_GUARD` | `purrdf.purremb.v1.index-guard` |
| `D_INDEX` | `purrdf.purremb.v1.index` |

An integer passed to `H` is its fixed-width little-endian byte representation.
Identifiers and digests with the same 32-byte storage are distinct semantic types
and MUST NOT be interchanged by an API.

Exact external-artifact digests, the exact source digest, and section digests are
plain `SHA256` so they can be compared with independent tooling. Typed identities
are domain-separated with `H`.

### 3.3 Canonical TLV blocks

Generation contracts, target identity material, external bindings, and index
guards use canonical type-length-value blocks. A TLV block has no leading length;
its containing record or enclosing TLV supplies the byte length. Each entry is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 2 | `tag: u16` |
| 2 | 1 | `wire_type: u8` |
| 3 | 1 | `flags: u8` |
| 4 | 4 | `value_length: u32` |
| 8 | `value_length` | value bytes |
| next | variable | zero padding to an 8-byte boundary |

Tags MUST be strictly increasing and MUST occur at most once in a block. Flag bit
0 is `TLV_CRITICAL`; every other flag bit is zero in v1. Known fields use the
critical flag. An unknown critical tag is an error. An unknown noncritical tag is
retained as exact bytes, participates in every enclosing digest, and MAY be
ignored semantically by a reader.

The v1 wire types are:

| Code | Name | Canonical value |
| ---: | --- | --- |
| 1 | `BYTES` | exactly `value_length` uninterpreted bytes |
| 2 | `UTF8` | well-formed UTF-8; exact bytes define identity |
| 3 | `U32` | `value_length = 4`, one little-endian `u32` |
| 4 | `U64` | `value_length = 8`, one little-endian `u64` |
| 5 | `DIGEST32` | `value_length = 32` |
| 6 | `BOOL` | `value_length = 1`, value `0` or `1` |
| 7 | `BLOCK` | one canonical nested TLV block |
| 8 | `BLOCK_LIST` | canonical list described below |
| 9 | `U32_LIST` | canonical list described below |

A `BLOCK_LIST` value starts with `count: u32` and a zero `reserved: u32`.
Each item then contains `block_length: u64`, the canonical block bytes, and minimal
zero padding to 8 bytes. A `U32_LIST` value starts with `count: u32` and a zero
`reserved: u32`, followed by exactly `count` little-endian `u32` values. Its outer
TLV entry supplies minimal zero padding.

Padding bytes MUST be zero and MUST be minimal. No block may exceed 16 MiB, and
nested blocks may be at most eight levels deep. UTF-8 is not normalized: two
different UTF-8 byte sequences remain different identities even if a text layer
would display them equivalently. A field documented as nonempty MUST have a
nonzero value length.

## 4. Whole-file framing

### 4.1 Canonical file order

The file consists of:

```text
128-byte header
64-byte directory entries
minimal zero padding to 64 bytes
sections in directory order, each followed by minimal zero padding to 64 bytes
64-byte trailer
EOF
```

The file length is a multiple of 64. No preamble, suffix, concatenated member, or
trailing byte is permitted.

### 4.2 Header

The fixed header is 128 bytes:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 8 | `magic` | ASCII `PURREMB1` |
| 8 | 4 | `version` | `1` |
| 12 | 4 | `header_length` | `128` |
| 16 | 4 | `flags` | `0` |
| 20 | 4 | `section_count` | `10..=65535` |
| 24 | 8 | `directory_offset` | `128` |
| 32 | 8 | `directory_length` | `section_count * 64` |
| 40 | 8 | `first_section_offset` | `align_up(128 + directory_length, 64)` |
| 48 | 8 | `trailer_offset` | canonical offset of the trailer |
| 56 | 8 | `file_length` | `trailer_offset + 64` and exact input length |
| 64 | 32 | `artifact_root` | section 4.6 |
| 96 | 32 | `source_exact_digest` | exact source SHA-256 |

### 4.3 Directory entry

The directory contains exactly `section_count` fixed 64-byte entries:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 4 | `section_kind: u32` |
| 4 | 4 | `section_flags: u32` |
| 8 | 4 | `section_instance: u32` |
| 12 | 4 | `reserved`, zero |
| 16 | 8 | absolute `section_offset: u64` |
| 24 | 8 | `section_length: u64` |
| 32 | 32 | `section_sha256 = SHA256(section bytes)` |

Directory entries are strictly increasing by `(section_kind, section_instance)`. Sections occur physically in that same order. The first
entry starts at `first_section_offset`; every subsequent entry starts at
`align_up(previous_offset + previous_length, 64)`. Every section has nonzero
length. The trailer starts at `align_up(last_offset + last_length, 64)`. All
intervening bytes are zero.

Directory flag bit 0 is `SECTION_CRITICAL`; bit 1 is `SECTION_DERIVED`; every
other bit is zero in v1. The exact required flags are listed with the section
kinds. Duplicate `(kind, instance)` pairs, spans into the header or directory,
overlap, nonminimal gaps, misalignment, arithmetic overflow, and an out-of-bounds
span are errors.

### 4.4 Section kinds

| Kind | Name | Instances | Flags |
| ---: | --- | --- | --- |
| `0x0000_0001` | `SOURCE` | exactly instance 0 | `CRITICAL` |
| `0x0000_0002` | `CONTRACTS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0003` | `TARGETS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0004` | `TARGET_SETS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0005` | `RELATIONS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0006` | `TOKEN_SPANS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0007` | `MATRICES` | exactly instance 0 | `CRITICAL` |
| `0x0000_0008` | `EXTERNAL_BINDINGS` | exactly instance 0 | `CRITICAL` |
| `0x0000_0009` | `INDEX_GUARDS` | exactly instance 0 | `CRITICAL | DERIVED` |
| `0x0000_1000` | `MATRIX_DATA` | contiguous instances `1..=matrix_count` | `CRITICAL` |
| `0x0000_1001` | `INDEX_PAYLOAD` | contiguous instances `1..=inline_index_count` | `CRITICAL | DERIVED` |

The nine singleton metadata sections are always present, including when their
record count is zero. At least one `MATRIX_DATA` section is present. An
`INDEX_PAYLOAD` section exists only for an inline index.

Section kinds `0x8000_0000..=0xffff_ffff` are caller extensions. An unknown
extension section with `SECTION_CRITICAL` is an error. An unknown noncritical
extension section remains covered by its section digest and the artifact root and
is exposed as borrowed bytes. Other unassigned section-kind values are errors.

### 4.5 Trailer

The fixed trailer is 64 bytes:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 8 | `magic` | ASCII `PURREND1` |
| 8 | 4 | `version` | `1` |
| 12 | 4 | `trailer_length` | `64` |
| 16 | 8 | `file_length` | equal to the header and exact input length |
| 24 | 32 | `artifact_root` | equal to the header root |
| 56 | 8 | `reserved` | zero |

### 4.6 Whole-artifact integrity root

Let `header_zero_root` be the exact 128 header bytes with bytes 64 through 95
replaced by zero, and let `directory` be the exact populated directory bytes.
The artifact root is:

```text
H(D_ARTIFACT; header_zero_root, directory)
```

Full integrity verification consists of all of the following:

1. verify every section's plain SHA-256 against its directory entry;
1. verify the artifact root against the canonical header and directory;
1. verify all required zero padding, canonical offsets, and exact EOF;
1. verify that the trailer repeats the root and length.

This is integrity-equivalent to covering the whole canonical file: section bytes
are covered by their digests, all section descriptors and header fields are
covered by the root, and every remaining file byte has one required value. The
artifact root is not called a plain whole-file SHA-256. An exact external digest
of a `.purremb` file is `SHA256(all file bytes)`.

## 5. Source binding

The `SOURCE` section is exactly 128 bytes:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `flags` | bit 0 `RDFC_CERTIFIED`, no other bits |
| 8 | 8 | `source_length` | exact `.purrpck` byte length |
| 16 | 4 | `source_format` | `1` for `.purrpck` v1 |
| 20 | 4 | `reserved` | zero |
| 24 | 32 | `source_exact_digest` | `SHA256(all source bytes)` |
| 56 | 32 | `certified_rdf_digest` | independently verified RDFC SHA-256 |
| 88 | 32 | `dataset_target_id` | one `RDF_DATASET` target |
| 120 | 8 | `reserved` | zero |

`RDFC_CERTIFIED` is always set in v1. The exact digest MUST equal the duplicate in
the file header. The certified digest is obtained by independently reconstructing
and canonicalizing the source pack, not merely by trusting its stored header
claim. The referenced dataset target MUST encode the same certified digest.

The exact digest and the certified RDF digest answer different questions. Two
byte-distinct source artifacts can claim the same RDF graph; only the exact digest
selects the source whose ordinals and byte-level attachment are valid.

## 6. Typed identities

The following formulas are normative. `contract_bytes`, `stage_bytes`, and other
blocks are their exact canonical TLV encodings.

```text
FamilyContractDigest = H(D_FAMILY_CONTRACT; contract_bytes)
FamilyId             = H(D_FAMILY; FamilyContractDigest)
ChunkingContractId   = H(D_CHUNKING; chunking_stage_bytes)

VectorSpaceId = H(
    D_SPACE;
    FamilyId,
    u32le(effective_dimension),
    u32le(prefix_postprocessing)
)

TargetIdentityDigest = H(
    D_TARGET_IDENTITY;
    u32le(target_kind),
    canonical_target_identity_bytes
)

TargetId = H(
    D_TARGET;
    u32le(target_kind),
    TargetIdentityDigest
)

TargetSetId = H(
    D_TARGET_SET;
    u64le(row_count),
    TargetId[0], ..., TargetId[row_count - 1]
)
```

Matrix and projection identities are defined with their records in section 14.
External-binding and index identities are defined in sections 16 and 17.

All identity comparisons use all 32 bytes. Truncation is prohibited.

## 7. Vector-family contracts

### 7.1 `CONTRACTS` section layout

The `CONTRACTS` section begins with this 96-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `flags` | zero |
| 8 | 8 | `family_count` | at least 1 |
| 16 | 8 | `family_records_offset` | `96` |
| 24 | 4 | `family_record_size` | `96` |
| 28 | 4 | `space_record_size` | `80` |
| 32 | 8 | `space_count` | at least `family_count` |
| 40 | 8 | `space_records_offset` | `align_up(96 + family_count * 96, 8)` |
| 48 | 8 | `tlv_pool_offset` | `align_up(space_records_offset + space_count * 80, 8)` |
| 56 | 8 | `tlv_pool_length` | exact bytes through the final contract block |
| 64 | 32 | `reserved` | zero |

The section ends exactly at `tlv_pool_offset + tlv_pool_length`. Family records
are strictly increasing by `family_id`. Contract blocks occur in that record
order, start on 8-byte boundaries, use minimal zero padding between blocks, and do
not overlap or alias.

Each 96-byte family record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `family_id` |
| 32 | 32 | `family_contract_digest` |
| 64 | 8 | section-relative `contract_offset` |
| 72 | 8 | `contract_length` |
| 80 | 4 | `dimensionality_policy` |
| 84 | 4 | `stored_dimension` |
| 88 | 4 | `space_start`, index into the space table |
| 92 | 4 | `space_count` for this family |

The stored IDs and duplicated dimensional fields MUST equal values recomputed from
the contract block. Space ranges are packed contiguously in family-record order,
with no gap or overlap, and cover the complete space table.
`family_count`, `space_count`, every `space_start`, and every per-family space count
MUST be representable as `u32` because the fixed records use `u32` indexes.

Each 80-byte effective-space record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `vector_space_id` |
| 32 | 32 | `family_id` |
| 64 | 4 | `effective_dimension` |
| 68 | 4 | `prefix_postprocessing` |
| 72 | 4 | `prefix_ordinal`, zero-based within the family |
| 76 | 4 | `reserved`, zero |

Within one family range, records are strictly increasing by effective dimension,
their ordinals are contiguous from zero, and they exactly reproduce the prefix
declarations in the contract.

### 7.2 Artifact identity block

Model, inference engine, and tokenizer identities use an `ArtifactIdentity` TLV
block:

| Tag | Type | Cardinality | Meaning |
| ---: | --- | --- | --- |
| 1 | `UTF8` | required, nonempty | caller-supplied artifact identifier |
| 2 | `UTF8` | required, nonempty | media type or caller format identifier |
| 3 | `DIGEST32` | required | SHA-256 of exact artifact bytes or canonical manifest |
| 4 | `BYTES` | optional, nonempty | exact caller-supplied revision bytes |
| 5 | `U32` | required | `1` single artifact, `2` canonical artifact manifest |

A manifest is an external artifact whose bytes canonically enumerate all files,
digests, and roles needed to identify a multi-file model, engine, or tokenizer.
Its SHA-256, not a mutable location, occupies tag 3. Identifiers and revisions
MUST NOT contain an ambient filesystem path, process identifier, wall-clock value,
secret, or other machine-local value.

### 7.3 Applied-stage block

Execution, subject projection, preprocessing, chunking, pooling, normalization,
and truncation use an `AppliedStage` block:

| Tag | Type | Cardinality | Meaning |
| ---: | --- | --- | --- |
| 1 | `U32` | required | `0` not applied, `1` applied |
| 2 | `UTF8` | applied only, required and nonempty | caller-supplied implementation identifier |
| 3 | `DIGEST32` | applied only, required | implementation artifact or manifest SHA-256 |
| 4 | `UTF8` | applied only, required and nonempty | parameter encoding identifier |
| 5 | `BYTES` | applied only, required; may be empty | canonical parameter bytes |
| 6 | `DIGEST32` | applied only, required | `SHA256(tag 5 value)` |

A not-applied block contains only tag 1. An applied block contains all six tags.
The subject-projection stage is always applied. Other stages state their absence
explicitly rather than disappearing from the family contract.

The chunking contract ID is computed from the exact canonical chunking-stage block,
including the explicit not-applied state. Chunking parameters MUST completely state
the coordinate system, overlap, boundary policy, and any content-selection rule
that can change a chunk.

### 7.4 Distance-metric block

The distance metric is a block with these tags:

| Tag | Type | Cardinality | Meaning |
| ---: | --- | --- | --- |
| 1 | `U32` | required | metric code |
| 2 | `UTF8` | extension only, required and nonempty | caller-supplied metric identifier |
| 3 | `UTF8` | extension only, required and nonempty | parameter encoding |
| 4 | `BYTES` | extension only, required; may be empty | canonical parameters |
| 5 | `DIGEST32` | extension only, required | `SHA256(tag 4 value)` |

Metric codes are `1` cosine distance, `2` negative dot-product ranking, `3`
squared Euclidean distance, and `0x8000_0000` caller extension. Built-in metric
blocks contain only tag 1. An extension block contains all five tags.

For vectors `x` and `y`, the mathematical built-in values are
`1 - dot(x, y) / (L2(x) * L2(y))`, `-dot(x, y)`, and
`sum((x[i] - y[i])^2)`, respectively; smaller values rank first. Cosine distance
is undefined for a zero-norm operand and hard-fails rather than inventing a
score. Numerical kernels may optimize evaluation but MUST preserve the declared
metric and deterministic row-number tie-breaking when a stable result is
requested.

The code defines comparison semantics, not an instruction to transform stored
vectors. Generation-time normalization is independently declared by the
normalization stage. Prefix postprocessing is independently declared by the
dimensionality block.

### 7.5 Dimensionality block

The dimensionality block is:

| Tag | Type | Cardinality | Meaning |
| ---: | --- | --- | --- |
| 1 | `U32` | required | policy: `1` fixed, `2` Matryoshka leading prefixes |
| 2 | `U32` | required, nonzero | stored matrix dimension |
| 3 | `BLOCK_LIST` | required | effective-prefix records |

Each effective-prefix block contains exactly:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `U32` | nonzero prefix dimension |
| 2 | `U32` | postprocessing: `0` none, `1` deterministic L2 |

Prefix dimensions are strictly increasing, unique, no larger than the stored
dimension, and end with the stored dimension. A fixed family has exactly one
prefix. A Matryoshka family has at least two. Every prefix produces one effective
space record and its own `VectorSpaceId`; spaces at different dimensions or with
different postprocessing are incompatible even when they share one stored matrix.

### 7.6 Complete family contract

A family contract is one top-level canonical TLV block:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `BLOCK` | model `ArtifactIdentity` |
| 2 | `BLOCK` | inference-engine `ArtifactIdentity` |
| 3 | `BLOCK` | tokenizer `ArtifactIdentity` |
| 4 | `BLOCK` | execution `AppliedStage` |
| 5 | `BLOCK` | subject-projection `AppliedStage`, state applied |
| 6 | `BLOCK` | preprocessing `AppliedStage` |
| 7 | `BLOCK` | chunking `AppliedStage` |
| 8 | `BLOCK` | pooling `AppliedStage` |
| 9 | `BLOCK` | generation-time normalization `AppliedStage` |
| 10 | `BLOCK` | truncation `AppliedStage` |
| 11 | `U32` | dtype: `1` IEEE binary32, `2` IEEE binary64 |
| 12 | `U32` | byte order: `1` little-endian |
| 13 | `U32` | quantization: `0` none |
| 14 | `BLOCK` | distance-metric block |
| 15 | `BLOCK` | dimensionality block |

All fifteen tags are required and critical. Caller extension tags occupy
`0x8000..=0xffff` and are noncritical. Because their exact bytes participate in
`FamilyContractDigest`, two readers never silently treat contracts containing
different extension material as one vector space.

The execution parameters MUST state precision mode, deterministic-inference
settings, and other execution choices that can affect output values. The format
guarantees deterministic serialization of supplied finite values. It does not
claim that an incompletely specified or nondeterministic inference run produces
the same floating-point values on different hardware.

## 8. Targets and canonical subject identities

### 8.1 `TARGETS` section layout

The section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `target_record_size` | `96` |
| 8 | 8 | `target_count` | at least 1 |
| 16 | 8 | `target_records_offset` | `64` |
| 24 | 8 | `target_records_length` | `target_count * 96` |
| 32 | 8 | `identity_pool_offset` | `align_up(64 + target_records_length, 8)` |
| 40 | 8 | `identity_pool_length` | exact bytes through the final retained identity block |
| 48 | 16 | `reserved` | zero |

The section ends exactly at `identity_pool_offset + identity_pool_length`.
Target records are strictly increasing by `target_id`.

Each 96-byte target record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `target_id` |
| 32 | 32 | `target_identity_digest` |
| 64 | 4 | `target_kind` |
| 68 | 4 | `target_flags` |
| 72 | 8 | section-relative `identity_offset` |
| 80 | 8 | `identity_length` |
| 88 | 8 | `source_local_ordinal` |

Target flag bit 0 is `IDENTITY_BYTES_PRESENT`; bit 1 is
`SOURCE_ORDINAL_PRESENT`; all other bits are zero. If identity bytes are absent,
offset and length are zero. If present, blocks occur in target-record order,
start on 8-byte boundaries, have minimal zero padding between them, and reproduce
both the stored identity digest and target ID. If the ordinal is absent its field
is `u64::MAX`; if present it is not `u64::MAX`.

Digest-only targets omit the canonical identity block but retain the exact
`TargetIdentityDigest`. Omission is a disclosure control, not a different target:
the `TargetId` is identical with or without retained bytes.

### 8.2 Target-kind codes

| Code | Kind |
| ---: | --- |
| 1 | `CORPUS` |
| 2 | `DOCUMENT` |
| 3 | `CHUNK` |
| 16 | `RDF_DATASET` |
| 17 | `RDF_GRAPH` |
| 18 | `RDF_STATEMENT` |
| 19 | `RDF_REIFIER` |
| 20 | `RDF_ANNOTATION` |
| 21 | `RDF_TERM` |
| `0x8000_0000` | caller extension |

Other codes are errors. A caller-extension identity block contains exactly:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `UTF8` | nonempty caller-supplied extension-kind identifier |
| 2 | `UTF8` | nonempty canonical payload-encoding identifier |
| 3 | `BYTES` | canonical payload bytes |
| 4 | `DIGEST32` | `SHA256(tag 3 value)` |

### 8.3 Corpus target

A `CORPUS` identity block contains:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `DIGEST32` | SHA-256 of the exact external corpus manifest |
| 2 | `UTF8` | nonempty manifest media type or format identifier |
| 3 | `DIGEST32` | SHA-256 of a stable caller-supplied logical corpus identifier |

The logical identifier is not a filesystem path. Corpus-manifest bytes remain in
an external artifact and are bound through section 16.

### 8.4 Document target

A `DOCUMENT` identity block contains:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `DIGEST32` | parent corpus `TargetId` |
| 2 | `DIGEST32` | SHA-256 of exact external UTF-8 document bytes |
| 3 | `DIGEST32` | SHA-256 of a stable caller-supplied logical document identifier |
| 4 | `UTF8` | nonempty media type or format identifier |
| 5 | `U64` | document byte length |
| 6 | `U64` | Unicode scalar-value count |

Document bytes MUST be well-formed UTF-8 when verified. No Unicode normalization
is implied. Distinct logical documents may share a content digest; tag 3 keeps
their identities distinct inside one corpus.

### 8.5 Chunk target

A `CHUNK` identity block contains:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `DIGEST32` | parent document `TargetId` |
| 2 | `DIGEST32` | `ChunkingContractId` |
| 3 | `DIGEST32` | SHA-256 of the exact UTF-8 bytes in the chunk |
| 4 | `U64` | byte start in the parent document |
| 5 | `U64` | byte end in the parent document |
| 6 | `U64` | Unicode scalar start in the parent document |
| 7 | `U64` | Unicode scalar end in the parent document |

Starts are no greater than ends; a chunk is nonempty in bytes and scalars. Byte
boundaries lie on UTF-8 scalar boundaries. The byte slice's SHA-256 equals tag 3,
and its scalar count equals `scalar_end - scalar_start`. Overlapping chunks are
valid. A different chunking contract, overlap, or span produces a different
target identity.

Every chunk referenced by a matrix target set has a chunking-contract ID equal to
the chunking stage of that matrix family's contract. A chunk shared by matrices in
multiple families is valid only when each family derives the same
`ChunkingContractId`; different chunking contracts produce distinct chunk targets.

### 8.6 RDF dataset target

An `RDF_DATASET` identity block contains exactly tag 1 `DIGEST32`, the certified
RDFC digest from the `SOURCE` section. The exact source-pack digest is deliberately
absent: RDF identity remains stable while byte-level source attachment remains
strict.

### 8.7 RDF term target

An `RDF_TERM` identity block always contains tag 1 `U32`, whose value is the term
form. The remaining required tags depend on that form:

| Form | Code | Additional canonical fields |
| --- | ---: | --- |
| IRI | 1 | tag 2 `UTF8`: exact absolute IRI bytes |
| blank node | 2 | tag 2 `DIGEST32`: dataset `TargetId`; tag 3 `UTF8`: certified canonical blank label |
| literal | 3 | tag 2 `UTF8`: lexical form; tag 3 `UTF8`: datatype IRI; optional tag 4 `UTF8`: lowercase language tag; tag 5 `U32`: direction |
| triple term | 4 | tags 2, 3, 4 `DIGEST32`: subject, predicate, and object RDF-term `TargetId`s |

Literal direction is `0` absent, `1` left-to-right, or `2` right-to-left. A
language tag is lowercase and valid under the RDF 1.2 language-tag rules. A
direction requires a language tag. Literal lexical bytes are not datatype value
canonicalization; RDF term identity preserves the lexical form.

Blank labels are generated by the same independent canonicalization run that
certified the source and are scoped by the dataset target. Source-local blank
labels never enter identity. The canonical label is stored without the `_:`
prefix.

A triple term refers recursively to existing term targets. Its predicate target
is an IRI. Recursive triple terms form an acyclic graph. RDF-star input is
represented through these RDF 1.2 triple terms and through the reifier and
annotation structures below; it has no separate target kind or identity dialect.

### 8.8 RDF graph target

An `RDF_GRAPH` identity block contains:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `DIGEST32` | dataset `TargetId` |
| 2 | `U32` | graph form: `0` default, `1` named |
| 3 | `DIGEST32` | named-graph RDF-term `TargetId`; absent for default |

The named-graph term is an IRI or blank node. One dataset has at most one default-
graph target and at most one graph target for each graph-name term.

### 8.9 RDF statement target

An `RDF_STATEMENT` identity block contains four `DIGEST32` fields:

| Tag | Meaning |
| ---: | --- |
| 1 | graph `TargetId` |
| 2 | subject RDF-term `TargetId` |
| 3 | predicate RDF-term `TargetId` |
| 4 | object RDF-term `TargetId` |

The terms obey RDF 1.2 positional constraints. Graph identity is included, so
equal triples in different graphs are different statement targets.

### 8.10 RDF reifier target

An `RDF_REIFIER` identity block contains three `DIGEST32` fields:

| Tag | Meaning |
| ---: | --- |
| 1 | graph `TargetId` |
| 2 | reified statement `TargetId` |
| 3 | reifier RDF-term `TargetId` |

The graph agrees with the statement's graph. Distinct reifier terms over one
statement remain distinct targets.

### 8.11 RDF annotation target

An `RDF_ANNOTATION` identity block contains four `DIGEST32` fields:

| Tag | Meaning |
| ---: | --- |
| 1 | graph `TargetId` |
| 2 | reifier `TargetId` |
| 3 | predicate RDF-term `TargetId` |
| 4 | object RDF-term `TargetId` |

The terms and graph obey RDF 1.2 annotation constraints. Duplicate annotation
rows collapse to one target because their complete canonical identity is equal.

### 8.12 Source-local ordinals

Ordinals are acceleration hints. They do not participate in a target identity,
target-set identity, matrix identity, or vector-space identity. Exact source
verification MUST reject a recognized ordinal that resolves to a different
canonical target.

The recognized v1 ordinal spaces are:

| Target kind | Ordinal meaning |
| --- | --- |
| `RDF_TERM` | 1-based unified `PackId` in the exact source pack |
| `RDF_STATEMENT` | zero-based row in `PackView::quads()` iteration |
| `RDF_REIFIER` | zero-based row in `PackView::reifier_quads()` iteration |
| `RDF_ANNOTATION` | zero-based row in `PackView::annotation_quads()` iteration |

An ordinal on another target kind is an error in v1. Verification resolves the
row to values, reconstructs the canonical target identity, and compares its
`TargetId`; it never trusts equality of an ordinal alone.

## 9. Target sets and row ordering

The `TARGET_SETS` section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `set_record_size` | `64` |
| 8 | 8 | `set_count` | at least 1 |
| 16 | 8 | `set_records_offset` | `64` |
| 24 | 8 | `set_records_length` | `set_count * 64` |
| 32 | 8 | `row_reference_count` | total target IDs in all sets |
| 40 | 8 | `row_references_offset` | `align_up(64 + set_records_length, 8)` |
| 48 | 8 | `row_references_length` | `row_reference_count * 32` |
| 56 | 8 | `reserved` | zero |

The section ends exactly after the row-reference table. Each 64-byte set record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `target_set_id` |
| 32 | 8 | `row_start`, index into the row-reference table |
| 40 | 8 | `row_count` |
| 48 | 16 | `reserved`, zero |

Set records are strictly increasing by `target_set_id`. Their row ranges are
packed in record order without gaps or overlap and cover the complete row-
reference table. Every set has at least one row. Within a set, `TargetId`s are
strictly increasing and exist in `TARGETS`.

Canonical builders accept `(target, vector)` associations in any order, sort by
`TargetId`, reject duplicates, and reorder every matrix row to this target-set
order. Therefore insertion order and hash-map iteration cannot affect bytes.

`target(row)` and `row(row)` are O(1) after open. Because row IDs are sorted,
reverse `row_for_target(TargetId)` lookup is allocation-free binary search.

## 10. Structural relations

### 10.1 `RELATIONS` section layout

The section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `relation_record_size` | `120` |
| 8 | 8 | `relation_count` | may be zero |
| 16 | 8 | `relation_records_offset` | `64` |
| 24 | 8 | `relation_records_length` | `relation_count * 120` |
| 32 | 8 | `role_pool_offset` | `align_up(64 + relation_records_length, 8)` |
| 40 | 8 | `role_pool_length` | exact bytes through the final extension role |
| 48 | 16 | `reserved` | zero |

The section ends exactly after the role pool. Each 120-byte record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `subject_target_id` |
| 32 | 32 | `object_target_id` |
| 64 | 4 | `relation_kind` |
| 68 | 4 | `relation_flags` |
| 72 | 32 | `role_digest` |
| 104 | 8 | section-relative `role_offset` |
| 112 | 8 | `role_length` |

Records are strictly increasing by `(subject_target_id, relation_kind, object_target_id, role_digest)`. Both endpoints exist. Relation flag bit 0 is
`ROLE_BYTES_PRESENT`; all other bits are zero.

Built-in relations have zero role digest, offset, length, and flags. The extension
kind uses nonempty UTF-8 role bytes supplied by the caller, sets the flag, and
stores `role_digest = H(D_RELATION_ROLE; role_bytes)`. Role values occur in record
order with minimal 8-byte alignment and zero padding.

### 10.2 Relation-kind codes

| Code | Relation and required endpoint kinds |
| ---: | --- |
| 1 | `CORPUS_DOCUMENT`: corpus to document |
| 2 | `DOCUMENT_CHUNK`: document to chunk |
| 16 | `DATASET_GRAPH`: RDF dataset to RDF graph |
| 17 | `GRAPH_STATEMENT`: RDF graph to RDF statement |
| 18 | `STATEMENT_SUBJECT`: RDF statement to RDF term |
| 19 | `STATEMENT_PREDICATE`: RDF statement to RDF term |
| 20 | `STATEMENT_OBJECT`: RDF statement to RDF term |
| 21 | `STATEMENT_REIFIER`: RDF statement to RDF reifier |
| 22 | `REIFIER_TERM`: RDF reifier to RDF term |
| 23 | `REIFIER_ANNOTATION`: RDF reifier to RDF annotation |
| 24 | `ANNOTATION_PREDICATE`: RDF annotation to RDF term |
| 25 | `ANNOTATION_OBJECT`: RDF annotation to RDF term |
| 26 | `GRAPH_NAME`: named RDF graph to RDF term |
| 32 | `TRIPLE_TERM_SUBJECT`: triple-term target to RDF term |
| 33 | `TRIPLE_TERM_PREDICATE`: triple-term target to RDF term |
| 34 | `TRIPLE_TERM_OBJECT`: triple-term target to RDF term |
| `0x8000_0000` | caller extension |

Other codes are errors. When retained target identity bytes permit a cross-check,
built-in relations MUST exactly agree with their parent, component, graph, or
statement fields. Every retained corpus/document/chunk or RDF composite target has
the corresponding built-in relation records. Digest-only targets retain their
relations, allowing hierarchy traversal without disclosing identity bytes.

Relations are evidence and traversal indexes over target identity. They do not
alter target or matrix identity and cannot override a canonical target block.

## 11. Token spans

The `TOKEN_SPANS` section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `token_span_record_size` | `96` |
| 8 | 8 | `token_span_count` | may be zero |
| 16 | 8 | `token_span_records_offset` | `64` |
| 24 | 8 | `token_span_records_length` | `token_span_count * 96` |
| 32 | 32 | `reserved` | zero |

The section ends exactly after the record table. Each 96-byte record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `family_id` |
| 32 | 32 | `target_id` |
| 64 | 8 | `token_start` |
| 72 | 8 | `token_end` |
| 80 | 8 | `model_input_token_count` |
| 88 | 4 | `span_flags` |
| 92 | 4 | `reserved`, zero |

Records are strictly increasing by `(family_id, target_id)` and unique. The
family and target exist; the target is a document or chunk. The token interval is
half-open in the tokenizer output for the complete parent document, with start no
greater than end. `model_input_token_count` is nonzero and records the actual count
after the family's special-token and truncation rules.

Span flag bit 0 is `LEFT_TRUNCATED`, bit 1 is `RIGHT_TRUNCATED`, and bit 2 is
`COUNT_INCLUDES_SPECIAL_TOKENS`; every other bit is zero. Flags agree with the
family truncation contract.

Token spans are family-scoped because all effective Matryoshka spaces in one
family share the exact tokenizer and input projection. Resolving a
`VectorSpaceId` first resolves its family, yielding an unambiguous space-specific
token span without duplicating records for every prefix dimension.

Every document or chunk target used by a matrix has exactly one token-span record
for that matrix family. RDF-only target sets and corpus-level targets do not
require token spans.

## 12. Numerical representation

PURREMB v1 has two authoritative dtypes:

| Dtype code | Scalar | Width | Byte encoding |
| ---: | --- | ---: | --- |
| 1 | IEEE-754 binary32 (`f32`) | 4 | exact bits, little-endian |
| 2 | IEEE-754 binary64 (`f64`) | 8 | exact bits, little-endian |

Every stored scalar is finite. Positive and negative zero are both valid and their
sign bits are preserved. Subnormal finite values are valid. Any NaN payload,
positive infinity, or negative infinity is an error. A writer validates each
scalar before writing it; full verification validates every stored scalar.

Matrices are dense C-order arrays with shape `[row_count, stored_dimension]`.
There is no stride metadata: the byte offset of scalar `(row, column)` is

```text
(row * stored_dimension + column) * scalar_width
```

with checked arithmetic. Row and dimension counts are nonzero. The raw matrix
section length is exactly `row_count * stored_dimension * scalar_width`.

The file-relative 64-byte alignment guarantees typed alignment for a normal
page-aligned mmap base. It does not prove that an arbitrary borrowed `&[u8]` starts
at a naturally aligned address. A portable reader therefore always offers
zero-copy row bytes and little-endian scalar iterators. It offers a borrowed native
`&[f32]` or `&[f64]` only when the actual pointer satisfies the scalar alignment
and the host is little-endian. Native typed slices additionally require full
artifact verification; a structural-only scalar iterator checks finiteness lazily
and returns an error at the first invalid value. A big-endian or unaligned host
uses the same allocation-free decoding iterator and returns identical logical
values.

## 13. Matryoshka effective projections

### 13.1 Raw leading-prefix view

For a prefix dimension `d`, the raw projection contains, in target-set row order,
the first `d` scalars of every stored row. It is a strided borrowed matrix view:
each row prefix is contiguous and zero-copy, while consecutive row prefixes are
separated by `stored_dimension - d` stored scalars. No secondary prefix matrix is
stored.

With postprocessing `NONE`, the logical projection byte stream used for hashing is
the concatenation of each row's first `d` exact little-endian scalar encodings.
Signed zeros are unchanged.

### 13.2 Deterministic L2 view

With postprocessing `DETERMINISTIC_L2`, each row prefix is normalized independently
using the following exact algorithm. All intermediate operations are IEEE-754
binary64, round-to-nearest ties-to-even, performed in the written order without a
fused multiply-add. A binary32 input converts exactly to binary64 before the fold.

```text
scale = +0.0
ssq = +1.0

for i in 0 .. d:
    a = abs(binary64(x[i]))
    if a != +0.0:
        if scale < a:
            r = scale / a
            t = r * r
            ssq = +1.0 + ssq * t
            scale = a
        else:
            r = a / scale
            t = r * r
            ssq = ssq + t

if scale == +0.0:
    error ZERO_NORM

norm = scale * sqrt(ssq)
if norm is not finite or norm == +0.0:
    error INVALID_NORM

for i in 0 .. d:
    y64 = binary64(x[i]) / norm
    y = round y64 once to the matrix dtype, ties-to-even
```

`sqrt` is the correctly rounded IEEE-754 square root. The sign of zero from
`x[i] / norm` is retained. The logical projection bytes are the concatenated
little-endian encodings of `y` in row order. The view is allocation-free: it may
calculate each row or scalar on demand. A zero or invalid norm is a deterministic
verification and access error; it is never replaced with a fallback vector.

## 14. Matrices and projection identities

### 14.1 `MATRICES` section layout

The section begins with this 96-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `matrix_record_size` | `160` |
| 8 | 8 | `matrix_count` | at least 1 |
| 16 | 8 | `matrix_records_offset` | `96` |
| 24 | 8 | `matrix_records_length` | `matrix_count * 160` |
| 32 | 4 | `projection_record_size` | `152` |
| 36 | 4 | `flags` | zero |
| 40 | 8 | `projection_count` | sum of effective spaces for all matrices |
| 48 | 8 | `projection_records_offset` | `align_up(96 + matrix_records_length, 8)` |
| 56 | 8 | `projection_records_length` | `projection_count * 152` |
| 64 | 32 | `reserved` | zero |

The section ends exactly after the projection table.

Each 160-byte matrix record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `matrix_id` |
| 32 | 32 | `matrix_content_digest` |
| 64 | 32 | `target_set_id` |
| 96 | 32 | `family_id` |
| 128 | 4 | `data_section_instance` |
| 132 | 4 | `dtype` |
| 136 | 8 | `row_count` |
| 144 | 4 | `stored_dimension` |
| 148 | 4 | `reserved`, zero |
| 152 | 8 | `data_length` |

Matrix records are strictly increasing by `matrix_id`. Data-section instances are
contiguous from 1 in matrix-record order. The referenced target set and family
exist. Dtype and dimension equal the family contract, row count equals the target
set, and data length has the exact checked shape. A `(family_id, target_set_id)`
pair occurs at most once.

The corresponding `MATRIX_DATA` section is exactly the raw scalar bytes and has no
internal header. Its directory SHA-256 is the plain SHA-256 of those bytes.

### 14.2 Stored-matrix identity

Let `matrix_bytes` be the complete raw `MATRIX_DATA` bytes. Identities are:

```text
MatrixContentDigest = H(
    D_MATRIX_CONTENT;
    u32le(dtype),
    u64le(row_count),
    u32le(stored_dimension),
    matrix_bytes
)

MatrixId = H(
    D_MATRIX;
    TargetSetId,
    FamilyId,
    MatrixContentDigest
)
```

This content digest is distinct from the directory's plain section SHA-256. Full
verification recomputes both in one sequential read of the data.

### 14.3 Projection record

Each 152-byte effective-projection record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `projection_id` |
| 32 | 32 | `projection_content_digest` |
| 64 | 32 | `matrix_id` |
| 96 | 32 | `vector_space_id` |
| 128 | 4 | `effective_dimension` |
| 132 | 4 | `prefix_postprocessing` |
| 136 | 8 | `row_count` |
| 144 | 8 | `logical_byte_length` |

Projection records are grouped in matrix-record order and strictly increasing by
effective dimension within each matrix. Every matrix has exactly one projection
for every effective space in its family. The dimension and postprocessing agree
with that space. Row count agrees with the matrix. Logical byte length is exactly
`row_count * effective_dimension * scalar_width`, even though prefix bytes are not
stored contiguously.

Let `logical_projection_bytes` be the canonical byte stream from section 13:

```text
ProjectionContentDigest = H(
    D_PROJECTION_CONTENT;
    u32le(dtype),
    u64le(row_count),
    u32le(effective_dimension),
    u32le(prefix_postprocessing),
    logical_projection_bytes
)

ProjectionId = H(
    D_PROJECTION;
    MatrixId,
    VectorSpaceId,
    ProjectionContentDigest
)
```

The writer calculates every projection digest while streaming rows through the
highest-dimension matrix. It does not materialize a duplicate prefix matrix. Full
verification recomputes all projection digests. A consumer compares or attaches
an index to a `ProjectionId`, never to dimensions or family names alone.

## 15. Canonical matrix writing

An unordered builder performs these steps before emitting bytes:

1. canonicalize and deduplicate family contracts;
1. derive effective spaces and reject identity collisions or contradictory
   duplicate IDs;
1. canonicalize targets and sort them by `TargetId`;
1. canonicalize each target set and map caller rows into its sorted order;
1. sort matrices by their derived `MatrixId` and assign data instances from 1;
1. stream each matrix in canonical row order while calculating scalar validity,
   matrix content, prefix-projection digests, and the plain section SHA-256;
1. canonicalize relations, spans, bindings, and index guards;
1. write the sorted directory, root, and trailer with minimal zero padding.

A bounded-memory streaming writer receives the same canonical metadata plus target
rows already strictly increasing by `TargetId`. It writes to `Write + Seek`, uses
only bounded row and digest state, and rejects an out-of-order or duplicate row.
All counts and final section lengths are known before directory finalization.

No output decision depends on hash-map iteration, wall-clock time, RNG, host path,
process state, pointer value, native byte order, or thread schedule. Given equal
logical records and exact scalar bits, the unordered builder and sorted streaming
writer emit identical bytes.

## 16. Generic external-artifact bindings

### 16.1 Section layout

The `EXTERNAL_BINDINGS` section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `binding_record_size` | `192` |
| 8 | 8 | `binding_count` | may be zero |
| 16 | 8 | `binding_records_offset` | `64` |
| 24 | 8 | `binding_records_length` | `binding_count * 192` |
| 32 | 8 | `contract_pool_offset` | `align_up(64 + binding_records_length, 8)` |
| 40 | 8 | `contract_pool_length` | exact bytes through the final contract block |
| 48 | 16 | `reserved` | zero |

The section ends exactly after the contract pool. Each 192-byte binding record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `binding_id` |
| 32 | 4 | `scope_kind` |
| 36 | 4 | `binding_flags` |
| 40 | 32 | `scope_id` |
| 72 | 32 | `artifact_sha256` |
| 104 | 8 | `artifact_length` |
| 112 | 32 | `certified_rdf_digest` |
| 144 | 32 | `contract_digest` |
| 176 | 8 | section-relative `contract_offset` |
| 184 | 8 | `contract_length` |

Records are strictly increasing by `binding_id`. Contract blocks occur in record
order with minimal 8-byte alignment and zero padding. Flag bit 0 is
`CERTIFIED_RDF_PRESENT`; all other bits are zero. If absent, the certified digest
is all zero. If present, the bound artifact is an exact `.purrpck` v1 file and the
field is its independently verified RDFC SHA-256. Other artifact formats remain
eligible for exact binding but do not carry this core-certified flag.

Scope codes are:

| Code | `scope_id` semantic type |
| ---: | --- |
| 1 | exact source SHA-256 |
| 2 | `TargetId` |
| 3 | `TargetSetId` |
| 4 | `FamilyId` |
| 5 | `VectorSpaceId` |
| 6 | `MatrixId` |
| 7 | `ProjectionId` |
| 8 | `IndexId` |

Other scope codes are errors. The referenced typed ID exists in the artifact,
except that an index-scoped binding may point to the `IndexId` of the record that
references the binding.

### 16.2 Binding contract

The contract TLV block contains:

| Tag | Type | Cardinality | Meaning |
| ---: | --- | --- | --- |
| 1 | `UTF8` | required, nonempty | caller-supplied role identifier |
| 2 | `UTF8` | required, nonempty | artifact media type or format identifier |
| 3 | `BYTES` | optional, nonempty | stable caller-supplied external identifier |
| 4 | `BYTES` | optional, nonempty | exact artifact-format revision |
| 5 | `BYTES` | optional, nonempty | opaque policy or provenance reference |

The stable identifier and policy/provenance reference are identifiers, not
retrieval paths. They MUST NOT contain secrets, credentials, an ambient local
path, a process identifier, or a wall-clock value. The role and references have
no PurRDF-defined ontology meaning.

Identity formulas are:

```text
ExternalContractDigest = H(D_EXTERNAL_CONTRACT; contract_bytes)

BindingId = H(
    D_EXTERNAL;
    u32le(scope_kind),
    scope_id,
    artifact_sha256,
    u64le(artifact_length),
    certified_rdf_digest_or_32_zero_bytes,
    ExternalContractDigest
)
```

An exact external verification hashes the supplied bytes and checks both length
and SHA-256. Certified RDF verification additionally opens the supplied bytes as
the declared `.purrpck` v1 format, calls PurRDF's independent pack verifier, and
compares the resulting digest. A certified mismatch is reported separately from
an exact-byte mismatch. Certification of arbitrary RDF serializations belongs to
a caller layer that parses them into PurRDF before constructing a pack; the core
container does not dispatch external codecs from a caller-provided media type.

Corpus manifests, document text, model manifests, tokenizer artifacts, engine
artifacts, detached indexes, and caller RDF metadata all use this one binding
mechanism. Their bytes never become an authoritative matrix or an RDF vocabulary
inside PURREMB.

## 17. Opaque derived-index guards

### 17.1 Section layout

The `INDEX_GUARDS` section begins with this 64-byte header:

| Offset | Width | Field | v1 requirement |
| ---: | ---: | --- | --- |
| 0 | 4 | `schema_version` | `1` |
| 4 | 4 | `index_record_size` | `336` |
| 8 | 8 | `index_count` | may be zero |
| 16 | 8 | `index_records_offset` | `64` |
| 24 | 8 | `index_records_length` | `index_count * 336` |
| 32 | 8 | `guard_pool_offset` | `align_up(64 + index_records_length, 8)` |
| 40 | 8 | `guard_pool_length` | exact bytes through the final guard block |
| 48 | 16 | `reserved` | zero |

The section ends exactly after the guard pool. Each 336-byte index record is:

| Offset | Width | Field |
| ---: | ---: | --- |
| 0 | 32 | `index_id` |
| 32 | 32 | `source_exact_digest` |
| 64 | 32 | `family_id` |
| 96 | 32 | `vector_space_id` |
| 128 | 32 | `matrix_id` |
| 160 | 32 | `projection_id` |
| 192 | 32 | `target_set_id` |
| 224 | 32 | `payload_sha256` |
| 256 | 32 | `guard_digest` |
| 288 | 8 | `payload_length` |
| 296 | 8 | section-relative `guard_offset` |
| 304 | 8 | `guard_length` |
| 312 | 4 | `payload_section_instance` |
| 316 | 4 | `storage` |
| 320 | 4 | `determinism` |
| 324 | 4 | `index_flags` |
| 328 | 4 | `prefix_dimension` |
| 332 | 4 | `reserved`, zero |

Records are strictly increasing by `index_id`. Guard blocks occur in record order
with minimal 8-byte alignment and zero padding. Every referenced ID and the exact
source digest agree with the artifact. The vector space belongs to the family; the
projection belongs to the matrix and space; the matrix belongs to the target set;
and `prefix_dimension` equals the space and projection dimension.

Storage is `1` inline or `2` detached. Determinism is `1` deterministic or `2`
nondeterministic. Index flag bit 0 is `REBUILDABLE` and MUST be set; all other bits
are zero.

For inline storage, determinism is `1`, payload length is nonzero, and
`payload_section_instance` refers to the corresponding raw `INDEX_PAYLOAD`
section. Inline instances are contiguous from 1 in the relative order of inline
index records. The section bytes have no wrapper and reproduce length and plain
payload SHA-256.

For detached storage, `payload_section_instance` is zero and no payload section is
present. An index-scoped external binding with the same payload SHA-256 and length
is required. A detached opaque index may declare either determinism value.

### 17.2 Guard contract

An index guard is a canonical TLV block:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `BLOCK` | index implementation `ArtifactIdentity` |
| 2 | `UTF8` | nonempty parameter-encoding identifier |
| 3 | `BYTES` | canonical build parameters; may be empty |
| 4 | `DIGEST32` | `SHA256(tag 3 value)` |
| 5 | `BLOCK` | loss contract |
| 6 | `U32` | use role: `1` generic, `2` coarse-prefix retrieval, `3` full-prefix reranking |
| 7 | `UTF8` | nonempty payload media type or format identifier |
| 8 | `DIGEST32` | optional RDF-certified external metadata `BindingId` |
| 9 | `BOOL` | rebuildable, value true |

The loss-contract block contains:

| Tag | Type | Meaning |
| ---: | --- | --- |
| 1 | `BOOL` | approximate search, value true |
| 2 | `BOOL` | whether the index payload quantizes or transforms vectors |
| 3 | `UTF8` | required and nonempty when tag 2 is true: loss encoding identifier |
| 4 | `BYTES` | required when tag 2 is true: canonical loss parameters |
| 5 | `DIGEST32` | required when tag 2 is true: `SHA256(tag 4 value)` |

If guard tag 8 is present, it names an external binding whose certified-RDF flag
is set and whose scope is the guarded projection, matrix, vector space, family, or
exact source. It MUST NOT be index-scoped, which avoids a cyclic dependency between
`BindingId` and `IndexId`. The binding is an exact `.purrpck` v1 file whose RDF
content uses caller-supplied vocabulary. The guard does not define the meaning of
that vocabulary.

Identity formulas are:

```text
IndexGuardDigest = H(D_INDEX_GUARD; guard_bytes)

IndexId = H(
    D_INDEX;
    source_exact_digest,
    FamilyId,
    VectorSpaceId,
    MatrixId,
    ProjectionId,
    TargetSetId,
    u32le(prefix_dimension),
    payload_sha256,
    u64le(payload_length),
    u32le(determinism),
    IndexGuardDigest
)
```

Storage location and directory instance are excluded from `IndexId`; exact payload
and guard semantics are included. Index identity, payload bytes, parameters,
random seed if an index algorithm uses one, and determinism never enter family,
vector-space, target, matrix, projection, or RDF identity.

An index is usable only when every guard binding matches. A stale source, family,
prefix, target set, matrix, projection, algorithm, parameter block, loss contract,
payload, or certified metadata binding is a hard mismatch. A consumer may carry
separate coarse-prefix and full-prefix indexes; their distinct `VectorSpaceId`,
`ProjectionId`, dimension, and role prevent substitution.

## 18. Borrowed reader and access semantics

### 18.1 Structural open

A structural open over an immutable `&[u8]` performs enough validation that every
subsequent accessor is bounds-safe and panic-free:

1. header magic, version, fixed lengths, flags, count limit, arithmetic, and exact
   file length;
1. directory ordering, known-section cardinality and flags, extension rules,
   canonical offsets, alignment, non-overlap, zero padding, and trailer framing;
1. every fixed section header, record size, count multiplication, table span,
   pool span, reserved field, internal order, uniqueness, and cross-reference;
1. canonical TLV framing, type lengths, tag order, criticality, UTF-8, padding,
   nesting, and size bounds;
1. target-set and matrix shape arithmetic, dtype, space, prefix, and data-section
   relationships.

Structural open does not claim cryptographic integrity, finite matrix values,
projection-digest validity, source availability, or external-artifact validity.
Those checks have explicit modes below. A structurally open view may expose exact
borrowed bytes and safe scalar decoding while accurately reporting its unverified
state.

### 18.2 Full artifact verification

Full artifact verification adds:

- every section SHA-256 and the artifact root;
- every typed ID and content digest that can be recomputed from contained bytes;
- all matrix scalar finiteness and signed-zero-preserving decoding;
- all raw and deterministic-L2 projection content digests;
- all target identity blocks, relations, token spans, matrix projections,
  external-binding records, and index-guard cross-checks;
- canonical trailer and exact EOF.

Full verification returns an opaque resident certificate associated with the exact
byte slice, its length, and artifact root. Only the library constructs a
certificate. It cannot be deserialized or applied to a different allocation,
slice range, or artifact root. A prevalidated resident reopen may use the
certificate to skip section hashing, scalar scans, and projection recomputation;
it still constructs bounds-safe borrowed views from the certified slice.

The certificate assumes the borrowed bytes remain immutable. A caller that maps a
file MUST prevent in-process or external mutation, truncation, replacement, or
hole-punching for the complete lifetime of every view and certificate.

### 18.3 Source verification modes

Source verification is explicit and has two modes:

- `EXACT` takes the complete source byte slice, checks length and plain SHA-256,
  and opens it as the declared `.purrpck` version. An exact mismatch is reported
  without substituting an RDFC comparison.
- `CERTIFIED` performs `EXACT`, independently verifies the pack's RDF dataset,
  compares the certified RDFC digest, checks the dataset target, and validates all
  recognized source-local ordinals. An RDFC or ordinal mismatch has its own error.

Neither source mode changes matrix access or constructs an access typestate. The
verification report records which evidence was checked so callers do not confuse
a stored claim, a structurally safe view, full artifact integrity, exact source
attachment, and certified RDF attachment.

### 18.4 Required access behavior

A conforming Rust surface provides the behavior represented by
`EmbeddingBuilder`, `EmbeddingStreamWriter<W: Write + Seek>`,
`EmbeddingView<'a>`, a full-verification certificate/report, typed identity
newtypes, and a structured `EmbeddingError`.

After structural open, the borrowed view provides:

- iteration over families, effective spaces, target sets, matrices, external
  bindings, and index guards;
- O(1) target and matrix-row access by row number;
- allocation-free binary search from `TargetId` to row;
- range lookup over relations and family token spans;
- lookup of the single projection for `(TargetSetId, VectorSpaceId)`;
- zero-copy raw row and leading-prefix bytes;
- portable little-endian scalar iteration;
- conditionally available aligned native scalar slices;
- allocation-free deterministic-L2 prefix iteration.

Exactly one effective matrix projection may exist for a `(TargetSetId, VectorSpaceId)` pair. If two records would satisfy one lookup, the artifact is
malformed. A compatibility or distance helper requires equal `VectorSpaceId`s;
equal dimensions alone never establish compatibility.

The core API owns no file, mapping, thread pool, wall clock, RNG, model runtime, or
ANN engine. Heap bytes, mmap bytes, WebAssembly linear memory, and any other stable
borrowed byte storage produce identical logical results.

## 19. Corpus sharding and RDF projection rules

A large corpus may be partitioned into multiple exact source packs and PURREMB
artifacts. Stable corpus, document, chunk, family, and vector-space identities are
derived from content and contracts, not shard numbers or row ordinals. Each shard
contains only its local targets, relations, target sets, and matrix rows. Matrix
and projection IDs remain shard-specific through `TargetSetId`. Consumers may
combine rows only when the `VectorSpaceId` is identical and retain the originating
target-set and matrix identities as evidence.

Document and chunk text stays external in every shard. A corpus manifest may bind
all shards and documents by exact digest without introducing filenames into target
identity. Repeated content in different logical documents remains distinguishable
through the document logical-identifier digest.

RDF targets are derived only from the independently verified RDF 1.2 view of the
exact source pack. Named and default graphs, base statements, reifier bindings,
annotations, directional literals, blank nodes, and nested triple terms all
participate according to sections 8 and 10. A reader does not use RDF 1.1 or a
parallel RDF-star identity path. Source-pack ordinals may accelerate verification
but cannot create, merge, or rename an RDF target.

## 20. Determinism and canonical equality

Two conforming writers given the same logical input emit identical bytes. Logical
input equality means:

- equal exact source bytes and certified RDF digest;
- byte-equal canonical TLV fields and caller identifiers;
- equal targets, relations, spans, target sets, external bindings, and guards as
  sets before canonical ordering;
- equal IEEE scalar bits, including the sign of zero;
- equal inline derived-index payload bytes.

The following differences are identity-significant where present: Unicode byte
spelling, model or manifest digest, engine build, tokenizer, applied/not-applied
stage state, stage parameter encoding and bytes, chunk span, pooling,
normalization, truncation, dtype, prefix dimension, prefix postprocessing, metric,
target identity digest, matrix bits, and ANN guard/payload.

Nondeterministic detached index bytes may give a detached index a different
`IndexId`; they do not change source, family, space, target, matrix, projection, or
RDF identity. Inline derived indexes are deterministic so a complete logical input
still has one canonical file encoding.

## 21. Error taxonomy

Errors are structured and retain the most specific available context, such as
file offset, section kind and instance, record index, typed ID, target row, matrix
row and column, expected digest, and computed digest. At minimum, distinct error
categories cover:

- bad file or trailer magic, unsupported version, wrong fixed length, truncation,
  and trailing bytes;
- unknown critical section or TLV, invalid extension range, unsupported code,
  nonzero flags, and nonzero reserved bytes;
- integer conversion or arithmetic overflow, count limit, out-of-bounds span,
  overlap, misalignment, nonminimal offset, nonzero padding, and wrong table size;
- unsorted or duplicate directory entries, records, target IDs, target-set rows,
  relations, spans, family prefixes, matrix pairs, or index IDs;
- malformed TLV, duplicate or missing field, wrong wire type or length, invalid
  UTF-8, excessive nesting, and oversized metadata block;
- target identity, kind, parent, RDF position, blank scope, recursive triple,
  relation, token span, or source-ordinal inconsistency;
- section digest, artifact root, target ID, family ID, vector-space ID, target-set
  ID, matrix digest/ID, projection digest/ID, binding ID, guard digest, or index ID
  mismatch;
- zero row count, zero dimension, wrong matrix byte length, NaN, infinity, zero
  norm, invalid norm, unavailable prefix, dtype mismatch, and incompatible vector
  spaces;
- exact source mismatch, certified RDF mismatch, external artifact mismatch,
  certified external RDF mismatch, and stale index guard component;
- mutable or mismatched resident-certificate use.

Untrusted input MUST NOT cause a panic, out-of-bounds access, count-controlled
large allocation, unbounded recursion, integer wrap, or partially trusted view.
Verification stops with an error and never silently weakens the selected mode.

## 22. Extension rules

Extension behavior is deterministic:

- caller section kinds occupy `0x8000_0000..=0xffff_ffff`;
- caller target and relation kinds use the single extension code and carry a
  nonempty caller identifier inside canonical identity bytes;
- caller TLV tags occupy `0x8000..=0xffff`;
- caller distance metrics use the extension metric code and complete parameter
  block;
- unknown critical material is rejected;
- unknown noncritical material is preserved byte-for-byte and included in the
  enclosing typed identity and artifact integrity root.

An implementation MUST NOT assign semantics to an unknown identifier by string
guessing. It may expose the exact caller identifier and bytes. A change to the
meaning or byte grammar of a known v1 field, record, code, or identity formula
requires a different format version; it is not expressed by silently changing a
writer.

## 23. Privacy and security considerations

Embeddings can disclose source properties through inversion, membership
inference, attribute inference, similarity probing, and correlation with public
models. Approximate indexes can add graph structure, quantized centroids, labels,
or search statistics that leak further information. Treat vectors and indexes as
sensitive derived content even though they cannot reconstruct the source by
format contract.

Retained canonical target identity bytes may directly disclose RDF IRIs, literal
lexical forms, datatypes, language tags, and structural relationships. Corpus
target blocks omit source text, but document and chunk content digests remain
susceptible to dictionary attacks against predictable text. Digest-only target
records reduce direct disclosure but do not make identities secret.

External identifiers, roles, policy references, model identifiers, and ANN
metadata can also reveal deployment information. Callers must avoid credentials,
local paths, personal data, and secret policy values in canonical contracts.

SHA-256 section checks, typed identities, and the artifact root provide corruption
and mismatch detection. They do not authenticate an author, establish trust,
provide freshness, enforce authorization, or encrypt content. Authenticity and
confidentiality are supplied by caller-controlled transport, storage, signature,
and encryption systems outside PURREMB.

Memory mapping does not make mutable backing storage safe. A mapping used through
a borrowed view must remain immutable for the complete borrow. Consumers should
apply normal untrusted-file controls before mapping, including stable file
ownership, bounded metadata parsing, and protection against concurrent
replacement or truncation.

The authoritative exact matrix remains separate from ANN payloads and from RDF
canonical identity. A consumer can discard every derived index, rebuild it from
the verified matrix, and obtain the same source, family, target, matrix, and
projection evidence.

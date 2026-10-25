<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Lossless TOML profile, version 1

## Language and model

The accepted language is [TOML 1.1.0](https://toml.io/en/v1.1.0), encoded in
valid UTF-8. BOM, duplicate definitions and invalid control characters refuse.
The parser recognizes all string forms, radix/decimal integers, floats,
booleans, local dates/times, local/offset date-times, arrays, inline tables,
dotted/quoted keys, explicit/implicit tables and arrays of tables. Integers
must fit signed 64-bit values. Finite floating values use IEEE 754 double
precision; infinity and signed NaN have distinct typed cases. Date/time values
are validated by the shared RFC 3339 reader, supplying omitted seconds for
TOML 1.1 while preserving complete fractional spelling. Local temporal values
acquire no implicit offset or date. String semantics normalize newline pairs
to LF, including multiline literal strings; the verbatim byte source never
changes. A line-continuation escape removes only the whitespace TOML specifies.

The root is arena index zero. Every value/table node has a kind, first authored
span, optional decoded key/token span, parent, sibling ordinal and ordered child
indices. Parent identity distinguishes identically named keys in different
array elements. A node's path uses the shared RFC 6901 token escape; it is an
address into this TOML model, including array ordinals, not a JSON conversion.
An implicit table promoted by a later explicit header retains its arena identity
and original first-reference span; the later header remains an ordered Statement.
Nested inline members remain value nodes and key occurrences; Statements name
the outer assignments and header declarations.

Source spans are zero-based half-open UTF-8 byte ranges. Scalars span their
complete original value syntax, including quotes and escapes. Containers span
their complete bracket/brace syntax; table nodes span their first declaration
or implied key token. Statement spans end before trailing whitespace/comments.
Key occurrences retain their original quoting and source order. Scalar spans
are disjoint and ordered; maximal complementary runs are Structure occurrences.

## Profile and identities

No operation supplies a vocabulary implicitly. The explicit standard vocabulary
is `https://w3id.org/purrdf/toml#`; caller namespaces must be valid absolute IRIs
ending with slash or hash. Every vocabulary term derives by direct concatenation.
Profiles require a nonempty name of at most 1024 bytes and nonzero version.
Source/model byte bounds are portable nonzero counts no greater than the
unsigned 32-bit maximum; node/depth counts are nonzero unsigned 32-bit values.
The standard profile chooses 16 MiB source, 1,048,576 nodes/statements,
depth 256 and 256 MiB retained model admission.

Profile identity is SHA-256 over the following exact preimage. Strings are
framed by `purrdf_hash::frame::frame_le` (unsigned little-endian length followed
by bytes). Frame, in order: `purrdf-toml-profile-v1`,
`toml-1.1.0-flat-occurrences-scalar-cover-v1`, profile name, and namespace.
Append little-endian version (32 bits), maximum source bytes (64 bits), maximum
nodes (32 bits), maximum depth (32 bits), maximum model bytes (64 bits), then
each framed vocabulary local name, in this order:

```text
Document Value Statement Key Structure source sourceDigest byteLength profile
document byteStart byteEnd verbatim verbatimText path kind occurrence parent
ordinal key value node header digest size
```

The document preimage frames `purrdf-toml-document-v1` and the original source
IRI, then appends the 32 profile-identity bytes and 32 original-source SHA-256
bytes. Its SHA-256 lowercase hexadecimal suffix names `document-<digest>` below
the namespace before its first hash, with trailing slashes removed and one slash
inserted. Node IRIs append `#vN` for values, `#kN` for keys, `#dN` for statements
and `#sN` for structure runs; indices follow the respective model's source order.

## RDF projection

Metadata lives in the default graph. The document has type Document, source IRI,
sourceDigest (`sha256:` plus lowercase hex, typed with vocabulary digest),
byteLength (canonical nonnegative XSD integer), and profile (hex string).

All subordinate nodes state class, document owner, occurrence index and byteStart/
byteEnd. Values additionally state path, ordinal, parent (root document or value
IRI), kind, optional decoded key, and size for containers. Scalar values state
their typed semantic value and original verbatim literal; verbatim is typed with
vocabulary verbatimText. Strings use XSD string, integers XSD integer, floats
XSD double, booleans XSD boolean, and temporal values the corresponding XSD
date/time datatype. Kind preserves local/offset distinctions and signed NaN;
canonical temporal RDF lexical values use uppercase T/Z and include seconds.
Structure nodes state verbatim runs with the same datatype. Key nodes state
decoded key text. Statements link to the declared/assigned node and state a
typed boolean header flag. Every node's complete shape is verified on decode.

Kinds are `root-table`, `implicit-table`, `table`, `dotted-table`, `inline-table`,
`table-element`, `array`, `array-of-tables`, `string`, `integer`, `float`,
`nan`, `negative-nan`, `boolean`, `offset-date-time`, `local-date-time`,
`local-date` and `local-time`. Infinities remain float values with XSD INF/-INF;
NaN signs live in kind while the XSD value is NaN.

## Decode and resource law

The caller selects document IRI and expected profile. The decoder validates
root identity fields before collecting its bounded owned metadata. Subjects
whose document edge names the root, or whose IRI lies in its fragment namespace,
belong to the selected closed graph. Other documents may coexist. Identical
facts across native RDF 1.2 roles have set semantics; conflicts, unexpected
terms/predicates, missing facts and named-graph metadata refuse.

Scalar and structure verbatim facts enter the existing kernel's byte-cover law:
each byte must be covered exactly once, spans must be coherent and the full
source digest must match. The reconstructed source is analyzed under the exact
profile; its derived document identity must match the selection. Reprojection
must equal the complete selected graph, so valid bytes cannot launder invented
structure, types, semantics or identities. No partial success escapes.

Admission charges node layout, paths, decoded keys/scalars and authored
statement/key work before retaining it. Explicit depth covers dotted keys as
well as containers; explicit parser frames and arena indices replace recursive
container ownership. The decoder bounds cover bytes by declared source length
and metadata collection by the profile model bound. Physical RDF allocation
refusals preserve their native typed cause. Application schemas are independent
of TOML grammar and are never embedded into this profile.

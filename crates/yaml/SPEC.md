<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# YAML source schema, version 1

This is the law named `yaml-1.2.2-acyclic-lexical-stream-cover-v1`. All text is
UTF-8. A frame is `purrdf_hash::frame::frame_le`: an unsigned 64-bit little-endian
byte length followed by those exact bytes. Integer preimage fields below use
fixed widths and little-endian order, independently of host pointer width.

## Profile identity

The profile preimage has frames for, in order:

1. `purrdf-yaml-profile-v1`;
2. the schema identifier above;
3. the caller profile name;
4. the caller vocabulary namespace;
5. every vocabulary local name in the exact order in the next section.

Those frames are followed by the caller version as unsigned 32-bit, then eleven
unsigned 64-bit values in this order: Lex input bytes, Lex allocation bytes,
Lex occurrences, Lex collection depth, stream document count, cover span count,
partition bytes, RDF statement count, metadata payload bytes, projection bytes
and decode bytes. The final byte is the Lex alias permission, zero or one.
The profile identity is SHA-256 (`ContentDigest`) of this whole preimage.
The fixed identifier/name/namespace limits and deferred lexical key policy in
the README are part of this versioned schema. Changing this law requires a new
schema identifier, not silently reusing version 1.

## Vocabulary

Terms append these local names to the caller namespace. This list is also the
ordered profile identity list; class names and predicate/datatype names share
one closed vocabulary:

```text
Stream Document Node Item Entry Trivia Directive Escape CoverSpan
source sourceDigest lexBlake3 byteLength profile profileName profileVersion vocabulary specification
stream document ordinal byteStart byteEnd prologueStart prologueEnd startMarkerStart startMarkerEnd endMarkerStart endMarkerEnd root
node item entry key value parent kind style text rawStart rawEnd rawText
anchorStart anchorEnd anchorText tagStart tagEnd tagText tagKind tagName explicitKeyStart explicitKeyEnd
blockStart blockEnd chomping indentIndicator contentIndent target attachment attachmentNode decoded escapeKind
verbatim verbatimText sha256Digest blake3Digest
```

`rdf:type` uses its W3C IRI. Integer fields are canonical nonnegative decimal
`xsd:integer` literals. Ordinary text is `xsd:string`, without language or
direction. Digest and verbatim fields have the custom datatypes named above.

## Stream and subordinate identities

The stream preimage consists of four frames, in order:
`purrdf-yaml-stream-v1`, the complete caller source IRI, the raw 32-byte profile
identity and the raw 32-byte SHA-256 digest of all original source bytes.
The root is `urn:purrdf:yaml:` followed by lowercase hexadecimal SHA-256 of that
preimage. This identity is unrelated to RDF literal-row identity.

Every subordinate IRI appends a fragment to the root. Decimal ordinals have no
sign or leading zeros. Node ordinals are Lex grammar-completion order, document
ordinals are source order, and other arena ordinals are their native order:

| Fragment | Identity |
| --- | --- |
| `#dD` | Document descriptor at ordinal D |
| `#nN` | Native node occurrence at ordinal N |
| `#iN-P` | Sequence item at position P in node N |
| `#eN-P` | Distinct mapping entry at position P in node N |
| `#tT` | Trivia occurrence T |
| `#qQ` | Directive occurrence Q |
| `#xX` | Escape occurrence X |
| `#cC` | Byte-partition occurrence C |

## Complete metadata

Every subordinate subject has its class, `stream` ownership, `ordinal`,
`byteStart` and `byteEnd`. Ranges are absolute zero-based half-open byte spans.
Optional native fields are absent when `None`; absence is verified by the
complete reprojected graph, not interpreted as a default.

| Subject | Additional fields |
| --- | --- |
| Stream root | `source`, `vocabulary`, `specification`, `profileName`, `profileVersion`, `byteLength`, `sourceDigest`, `profile`, `lexBlake3`; `document` links to all descriptors |
| Document | `root` node; `prologueStart`/`prologueEnd`; optional `startMarkerStart`/`startMarkerEnd` and `endMarkerStart`/`endMarkerEnd` |
| Every Node | `kind`; optional `anchorStart`/`anchorEnd`/`anchorText`, `tagStart`/`tagEnd`/`tagText`, `tagKind`/`tagName`, `explicitKeyStart`/`explicitKeyEnd` |
| Scalar Node | `text`, `style`, `rawStart`/`rawEnd`/`rawText`; optional `blockStart`/`blockEnd`, `chomping`, `indentIndicator`, `contentIndent` |
| Alias Node | `target` exact prior node; `rawStart`/`rawEnd`/`rawText` |
| Sequence Node | `style`, `item` links |
| Mapping Node | `style`, `entry` links |
| Item | `parent` sequence, `node` child |
| Entry | `parent` mapping, `key`, `value`; span from key start through the later key/value end |
| Trivia | `rawText`, `kind`, `attachment`, optional `attachmentNode` |
| Directive | `rawText` |
| Escape | `rawText`, `escapeKind`, optional `decoded` single Unicode scalar |
| CoverSpan | `verbatim` literal with datatype `verbatimText` |

Node kinds are `scalar`, `alias`, `sequence`, `mapping`. Scalar styles are
`empty`, `plain`, `single-quoted`, `double-quoted`, `literal`, `folded`;
collection styles are `block`, `flow`. Chomping is `clip`, `strip`, `keep`.
Explicit tags are `non-specific` without a name, or `specific` with their
expanded lexical `tagName`; untagged nodes have neither field. Trivia kinds are
`whitespace`, `line-break`, `blank-line`, `comment`, `block-header-comment`;
attachments are `leading`, `trailing`, `document`, with node links only for the
first two. Escape kinds are `character` with `decoded`, or `line-joining`
without it. Literal values are never implicit Core-schema construction.

`sourceDigest` and `profile` have datatype `sha256Digest` and lexical prefix
`sha256:` plus 64 lowercase hexadecimal digits. `lexBlake3` has datatype
`blake3Digest` and lexical prefix `blake3:` plus 64 lowercase hexadecimal digits.
The byte cover is a distinct disjoint partition, with no continuation edges.
An omitted scalar has its zero-width syntax occurrence but emits no cover
bytes; an absent stream has no document roots. Structural spans are not cover
claims. Exact reprojection rejects any metadata outside this complete law.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Original-byte MIME carrier, version 1

The carrier describes a caller-identified byte string. Its original octets,
including malformed headers, non-ASCII bytes, folding, line endings, transfer
spelling, multipart delimiters, preamble and epilogue, are immutable. It never
substitutes repaired bytes, guessed charsets or normalized text for that source.
Interpretation follows RFC 2045/2046 MIME structure and RFC 5322 field unfolding.
An invalid message is still representable; invalid profile/source identity,
caller-declared resource excess and broken cover invariants are hard failures.

Every entry point requires an explicit profile. The profile names its vocabulary
and source-byte/header/part/depth/line maxima. An absent maximum explicitly means
memory-only admission; there is no compiled size, occurrence or depth limit.
Parsing, ownership and destruction use flat part indices and an iterative queue.
The offered standard namespace is available only by explicit selection.

## Cover and identities

Each original physical line emits one unchanged byte span through
`purrdf_core::cover::CoverBuilder`. Empty input has an empty lawful partition.
Finalization and RDF reconstruction call the same kernel byte-cover decoder:
offset bounds, full coverage, overlap/continuation rules and source SHA-256 are
not restated by this codec. Structural ranges describe bytes in that partition;
they do not introduce additional payload spans or guessed continuation edges.

The profile preimage is its registered domain followed by length-framed profile
name and vocabulary namespace, then the five limits in source/header/part/depth/
line order. Each limit is tag0 for unbounded or tag1 followed by its little-endian
u64 maximum. SHA-256 gives its identity. The document preimage is its registered
domain followed by length-framed source IRI, profile identity hex and original
source digest hex. Its IRI is `urn:purrdf:mime:<document digest>`.
Occurrence IRIs are document IRI plus kind and original ordinal. They are never
identities for deduplicating equal source occurrences.

Decoded attachment identity is the shared content digest of exact transfer-
decoded body octets. Message identity, filenames and surrounding headers do not
enter that digest. Unknown/conflicting or broken transfer encodings refuse
decoding; no partial attachment is returned. Base64 ignores nonalphabet octets
under RFC2045, then uses the shared strict native base64 value decoder.
Quoted-printable removes soft CRLF, decodes hex octets and removes transport
trailing horizontal whitespace; it preserves hard CRLF. No charset conversion
is performed. The 7bit and 8bit declarations require line-oriented bodies:
non-NUL bytes, CRLF line endings and at most998 octets between them; 7bit also
excludes high-bit octets. Violations are typed observations and decoded attachment
refusals, including on containers. Analysis still retains every byte and imposes
only caller resource ceilings. Binary bodies retain arbitrary octets unchanged.
Encoded words are checked in actual unstructured, phrase or comment positions,
with their exact RFC2047 charset-token,75-character word and76-character physical
line syntax. Quoted strings, address specifications, MIME parameters and Received
text remain literal. No charset interpretation is guessed from those bytes.

## Structural model

Headers retain their whole physical range, exact field-name range, occurrence
ordinal and ordered original value-segment ranges. Unfolding removes only the
physical line endings between those segments. Equal repeated fields are distinct
occurrences. Field names and media tokens compare in ASCII case-insensitive form;
source bytes remain unchanged. Parameters retain order and duplicate occurrences.
Absent Content-Type means text/plain, except multipart/digest children default
to message/rfc822. Absent transfer encoding means 7bit, as specified by MIME.
These protocol defaults do not supply a missing caller vocabulary or profile.

Conflicting structural singleton fields remain ambiguous with typed problems.
Malformed content-type does not invent a subtype or boundary. Multipart
boundary parameters must satisfy RFC2046's1..70-character alphabet and final
nonspace syntax. Invalid parameters remain typed defects, with no guessed boundary.
Delimiters match the declared boundary exactly, with permitted trailing SP/TAB;
the CRLF introducing a delimiter belongs to its parent delimiter, not the child
payload. Adjacent delimiters preserve empty child ranges. Unterminated parts,
missing/mismatched boundaries, close-before-open and unexpected boundary candidates remain original
bytes with problems. Base64/QP-encoded containers are not guessed into synthetic
original-source child ranges; they have a typed unsupported-container problem.
Nested message/rfc822 and multipart parts share original absolute coordinates.

## RDF projection and refusal law

Projection builds the production RDF IR directly. All claims occupy the default
graph. Every selected document has source IRI, source digest, profile identity
and byte length. Every Span, Part, Header, Segment, Parameter, Structure and
Problem has a unique occurrence IRI, class, document membership and ordinal.
Part/header/structure/problem ranges are absolute source byte coordinates.
Parts retain parent/child, depth, body bounds, declared media parameters and
transfer policy. Header values, names, parameters and cover payloads use canonical
lowercase `xsd:hexBinary`; integers use canonical nonnegative `xsd:integer`.
No byte is forced through UTF-8 to become a plain literal.

The decoder selects a document namespace and explicitly declared members. It
requires singular correctly typed source/profile/digest/length and complete
cover-span facts. It reconstructs through the kernel, checks document identity,
reanalyzes those actual bytes under the requested profile and regenerates the
canonical production projection. Every selected asserted statement must equal
that projection by term value, independently of dataset intern IDs and statement
order. Missing, invented, conflicting, foreign-membership, named-graph,
reifier/annotation or changed occurrence metadata refuses before any bytes escape.
Separate document namespaces may coexist. Extra claims in the selected namespace
are never silently discarded. Public mutable model fields are likewise validated
against original-source analysis before projection.

A repair, OCR result, embedding window or extracted region can cite the kernel's
digest-bound `SpanReference`; it does not overwrite this carrier's covered source.

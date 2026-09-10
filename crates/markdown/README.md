<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-markdown` — A Markdown Document as an RDF Graph of Itself

[![crates.io](https://img.shields.io/crates/v/purrdf-markdown.svg)](https://crates.io/crates/purrdf-markdown)
[![docs.rs](https://docs.rs/purrdf-markdown/badge.svg)](https://docs.rs/purrdf-markdown)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-markdown` turns a Markdown document into RDF 1.2 along the
document's own structure. It is not a chunker that cuts every N bytes: the
unit is the thing the author wrote — a heading, a numbered verse, a
paragraph — and the byte bound only acts on a unit that is too large to
stand alone. The output is a graph you can query, index, embed, and read
back to the exact bytes it came from.

## What comes out

For one document the slicer emits one claim per node, each claim a small
set of N-Triples lines (valid Turtle):

- **One document node**: the source's SHA-256, its media type, its byte
  length, the profile it was sliced under, and its title.
- **One section node per heading**: every ATX heading (`#` through
  `######`) and every movement marker line (`⁂ *name*`), with its level,
  ordinal, parent section, heading text, and byte span. A movement sits one
  level under the nearest heading, so consecutive movements are siblings.
- **One unit node per verse or paragraph**: a numbered line (`12. text`)
  runs to the next blank line and is a verse carrying its number; any other
  run of non-blank lines is a paragraph. Every unit carries **exactly one
  plain `xsd:string` literal**, the verbatim byte span of the source, plus
  its byte span, ordinal, section, and heading lineage as *typed* literals.
  A full-text or embedding index that selects plain strings therefore sees
  one text per unit and nothing else.
- **Citations from a concordance table**: a `## Concordance` section whose
  rows are `| Verses | Canon source | Anchors |` lifts into a `cites` triple
  from every verse in the row's range (`2–5`, `2-5`, or `4`) to each
  backticked anchor, and a `canonSource` triple to each backticked path.
  With a canon IRI base declared, an anchor becomes the IRI `base ++
  anchor` — pure concatenation, and every anchor is checked against the
  IRI it would mint before anything is rendered (see [The anchor lift is
  checked](#the-anchor-lift-is-checked)). Without a base an anchor is a
  typed literal, never a guess, and nothing about its bytes is refused.

Horizontal rules and table rows are structure, not units. Spans are
verbatim: the literal's bytes are `source[start..end]`, with no trimming,
no normalization, and nothing prepended.

## A small example

```markdown
# The Book

⁂ *the crossing*

1. The first verse crosses the socket whole.

2. Two sovereign stars share one trajectory.
```

Sliced under `Vocabulary::under("urn:example:doc:")` with the profile
`Profile::new("example-v1", 1, vocabulary)` and the source id
`urn:example:book`, the second verse's claim is (the two 64-hex digests
shortened to their first eight characters):

```text
<urn:example:doc:unit:sha256:7282dfbe…> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:example:doc:Unit> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:byteEnd> "122"^^<http://www.w3.org/2001/XMLSchema#integer> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:byteStart> "78"^^<http://www.w3.org/2001/XMLSchema#integer> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:document> <urn:example:book> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:lineage> "The Book > the crossing"^^<urn:example:doc:lineage> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:ordinal> "1"^^<http://www.w3.org/2001/XMLSchema#integer> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:section> <urn:example:doc:section:sha256:4ef32077…> .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:text> "2. Two sovereign stars share one trajectory." .
<urn:example:doc:unit:sha256:7282dfbe…> <urn:example:doc:verse> "2"^^<http://www.w3.org/2001/XMLSchema#integer> .
```

```rust
use purrdf_markdown::{Profile, SourceDocument, Vocabulary, slice_markdown};

let vocabulary = Vocabulary::under("urn:example:doc:")?;
let profile = Profile::new("example-v1", 1, vocabulary);
let claims = slice_markdown(
    &SourceDocument { id: "urn:example:book", bytes: markdown.as_bytes() },
    &profile,
)?;
for claim in &claims {
    // claim.turtle: sorted N-Triples lines; claim.span: the byte span.
}
# Ok::<(), purrdf_markdown::MarkdownError>(())
```

## The vocabulary is yours

PurRDF mints no vocabulary. Every class, predicate, and datatype IRI the
slicer emits, and the base it mints node IRIs under, is configuration the
caller supplies in a `Vocabulary`. `Vocabulary::under(base)` derives a whole
vocabulary from one base with the crate's local names (`Document`, `Unit`,
`byteStart`, `text`, ...); every field is public, so any IRI can be replaced.
An empty, unwritable, or merely *relative* IRI is a typed error naming the
field, not a default: the crate asks the workspace IRI law — reached through
`purrdf-core`, so it is the same law the kernel interns under — and refuses up
front what would otherwise be refused a stage later. The source id answers to
the same rule. The only IRIs the crate brings are the standard's: `rdf:type`
and `xsd:integer`.

## The anchor lift is checked

Concatenation is deliberate: RFC-3986 reference resolution would dissolve a
fragment base's `#` and fold dot segments away, so `base ++ anchor` and the
base the caller declared would part company. But concatenation is exactly why
the result cannot be trusted, so with a canon base declared every anchor in the
concordance is walked before a claim is rendered, and `base ++ anchor` must

- be an absolute IRI under the workspace law, and
- lie under the declared base.

Containment is `BaseIri::relativize`: the minted IRI is under the base exactly
when a relative spelling of it against that base exists — decided by
round-tripping that spelling back through resolution, not by comparing strings.
That is the rule that answers for a *fragment* base (`https://example.org/canon#`
++ `tide-line` relativizes to `#tide-line`), and it closes two escapes spelled
entirely in lawful IRI characters, which no character blacklist can see: an
anchor that climbs out of a path base (`../x` under `https://example.org/canon/`),
and one that extends the base's host into another authority
(`.elsewhere.example/x` after `https://example.org`). Both are refused, naming
the anchor and the verse range of the row that wrote it.

The law is IRI-lawfulness, never ASCII: `中文` mints `…/canon#中文` and lifts,
because RFC-3987 `ucschar` is inside an IRI. And a refusal is scoped to minting
alone — the same anchor with no canon base declared is a typed literal and
slices without complaint.

## Deterministic, and identified by its law

The same bytes under the same profile slice to byte-identical claims on
every target: the slicer opens no file, reads no clock, and uses no float.
Every emitted node's triples are sorted bytewise, one triple per line.

A `Profile` names the law it applies: a name, a version, the vocabulary, the
byte bound, and the overlap. Its identity is the SHA-256-derived
chunking-contract id of `purrdf-core` over a canonical, human-readable stage
description that lists every one of those, so a change to any of them mints a
new profile rather than drifting under the old name. The optional canon base
for concordance anchors is a consumer's option, not a parameter of the law,
and stays outside the identity. The document node records the profile as
`<name>:<hex>`.

A unit's IRI is `<node base>unit:sha256:<hex>`: the hex is the SHA-256 of a
length-prefixed preimage of the kind, the source id, the profile's contract
id, the byte span, the digest algorithm tag, and the SHA-256 of the span's
bytes. That has three consequences worth knowing:

- Two byte-identical paragraphs in one document are two distinct nodes,
  because the span is inside the identity.
- An insertion moves every later span and re-mints every later unit while
  their text is unchanged; a same-length substitution re-mints only the unit
  it touches and moves no boundary. Those are opposite signatures, and the
  vectors assert each.
- The digest algorithm is inside both the preimage and the IRI, so another
  producer can mint the same shape under another algorithm without
  redefining the preimage.

A consumer that holds the source bytes can re-derive any unit's IRI from its
recorded span with `unit_iri` and refuse a hit that does not re-derive: the
binding between a search result and its text is proven on read-back, not
assumed.

## Oversize units: the split law

A unit over `max_bytes` (default 2048) splits into pieces: each cut falls at
the last newline at or before the bound, else at the last UTF-8 scalar
boundary, and never inside a scalar. No piece is ever over the bound, which is
why a bound under four bytes — the width of the widest scalar — is refused.
The next piece starts `overlap` bytes (default 128) before the cut, snapped
backward to the start of a line, never before the unit's own start, and
carries a `continues` triple to the previous piece. The overlap sits strictly
under the bound. A split never crosses a heading, and every piece keeps its
verse number and section.

## A leading byte order mark

A byte order mark opening the document states its encoding, so the first
line's structure is read after it and a document that carries one slices into
the same structure as the one that does not. Every span still counts the
document's own bytes, so the mark falls before the first span and a unit's
literal is always the verbatim bytes of its span. At any other offset the mark
is ordinary content.

## Errors

`slice_markdown` refuses, typed, and never guesses. The profile answers first:
a vocabulary field that is empty or cannot be an IRI reference (naming the
field), a name carrying a control character (the stage description states one
fact per line, so a newline in a name would state facts of the law), a
`max_bytes` under four, and an `overlap` at or over `max_bytes`. Then the
document: an empty source id, which would be written `<>` and name no
document; a source id that cannot be written inside `<` and `>`; and bytes
that are not UTF-8 (naming the offset).

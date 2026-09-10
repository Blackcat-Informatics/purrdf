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
  anchor`; without one it is a typed literal, never a guess.

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
An empty or unwritable IRI is a typed error, not a default. The only IRIs the
crate brings are the standard's: `rdf:type` and `xsd:integer`.

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
boundary, and never inside a scalar. The next piece starts `overlap` bytes
(default 128) before the cut, snapped backward to the start of a line, never
before the unit's own start, and carries a `continues` triple to the previous
piece. A split never crosses a heading, and every piece keeps its verse
number and section.

## Errors

`slice_markdown` refuses, typed, on bytes that are not UTF-8 (naming the
offset), on a source id that cannot be written inside `<` and `>`, and on a
vocabulary field that is empty or cannot be an IRI reference (naming the
field). It never guesses.

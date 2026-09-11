<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# The Structural Markdown Slicing Law

**A stand-off specification for turning a written document into RDF 1.2
along its own structure**

Version 2.0.0-draft — 2026-09-10 — Blackcat Informatics® Inc.

## Abstract

A written document already has a structure: its author put it there, in
headings, in numbered lines, in paragraphs separated by blank lines. This
specification defines a total, deterministic function from the bytes of
such a document, plus a declared *profile*, to a typed layer of
annotations over those bytes and to an RDF 1.2 graph projected from that
layer. The document's bytes are never modified, never normalized and
never copied into the model: every node the law states names a byte range
of the source, and a reader holding the source can return from any node
to the exact bytes it was minted over. Node identities are
content-addressed and carry the whole law inside them, so the same bytes
under the same profile yield byte-identical output on every
implementation and on every target, and any change to the law re-mints
rather than drifting. The law recognizes exactly what an author writes —
ATX headings, movement markers, numbered verses, paragraphs, and one
concordance table — and it refuses, by name, everything it cannot state.
It is offered as a shared law for anyone who wants a document to be a
graph of itself.

The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are to be
interpreted as described in RFC 2119 and RFC 8174.

## 1. Terminology and the node model

**Source** — the exact bytes of one document. The source MUST be valid
UTF-8. Nothing in this law modifies it.

**Span** — a byte range of the source, `[start, end)`, half-open, counted
from byte zero of the source. Every span is a range of the source's own
bytes: no trimming, no normalization and nothing prepended, so
`source[start..end]` is exactly what the span annotates.

**Scalar** — one Unicode scalar value. A *scalar span* counts scalars
where a byte span counts bytes, over the same source, from scalar zero.

**Profile** — the declared law: a name, a version, a vocabulary, the byte
bound `max_bytes`, the overlap `overlap`, and an optional canon IRI base.
Every one of them, the canon base included, is inside the profile's
**contract id**; the name, the version and the two constants are also
inside its **chunking id** (§5).

**Document** — the whole source, and the root of the containment order.

**Section** — a span opened by a heading line and closed by containment
(§2.1). A **movement** is a section opened by a movement marker rather
than by a heading; it differs from a heading section in exactly one
respect, the class it is stated with.

**Unit** — a span of text that stands on its own: a numbered **verse**, a
**paragraph**, or one **piece** of either after the split law (§3) has
cut it.

**Citation** — one row of a concordance table (§4): a verse range, canon
source paths, and canon anchors.

### 1.1 The containment order

Spans are ordered by containment: a span contains another when it starts
at or before it and ends at or after it. Under this order

```
document ⊃ section ⊃ unit ⊃ piece
```

and the order is **partial**, not a tree of everything. Two sibling
sections contain neither the other. Two pieces of one oversize unit
deliberately *overlap* rather than nest, because a continuation reaches
back (§3). A unit records the innermost section in force where it began,
which is membership; containment asks which sections hold the unit's
span, and every ancestor of that section does. Implementations SHOULD
expose both, and MUST NOT conflate them.

An implementation SHOULD answer span questions with the thirteen Allen
interval relations — before, meets, overlaps, starts, during, finishes,
equals and their six inverses — because the endpoint cases are the ones a
hand-written comparison gets wrong, and they are the common ones here: a
section ends exactly where the next section's heading line begins, which
is *meets*, and the two share no byte.

## 2. The dialect grammar

Recognition operates on **lines**. A line runs from the byte after the
previous `\n` to the byte before the next `\n`, or to the end of the
source; the terminating `\n` is part of no line.

Recognition operates on a line's **trimmed** text; spans never do. A
document written with CRLF line endings therefore states the structure
its LF twin states, while every span still carries the document's own
bytes, the `\r` among them. A `\r` inside a unit's span is part of that
unit's verbatim text and MUST NOT be stripped.

**The leading edge is bounded rather than trimmed.** The trailing edge
trims away without a bound; past a certain indent, though, a line stops
looking like structure at all, and that is where the leading edge stops.

A line's **leading run** is its run of U+0020 SPACE characters at byte
zero of the line. A line's **marker text** is the line from the byte
after that run. A heading (§2.1), a movement marker (§2.1) and a verse
number (§2.2) are recognized **in the marker text**, and only when the
leading run is **at most three** spaces. So `   # Title` states the
heading `# Title` states, `  1. one` is verse 1, and ` ⁂ *m*` is a
movement marker.

A leading run of **four or more** spaces opens no marker. The line is
ordinary content: it continues the unit it falls in, or opens a
paragraph where none is open. This is not a refusal, and this law
defines no indented code block — an implementation MUST NOT treat such a
line as one. It simply declines to read a marker that far in, which is
where CommonMark places its indented-code threshold.

A **tab** (U+0009) in the leading run opens no marker either, wherever
in the run it falls: a line whose first non-space character is a tab
states no heading, no movement and no verse. CommonMark expands a tab to
the next four-column tab stop, so a run of nought to three spaces
followed by a tab reaches column four exactly, at or past the bound
above; this law states that outcome directly, so an implementation
decides it on a line's bytes and MUST NOT expand a tab to do it.

The leading run is read for **recognition only**, exactly as the byte
order mark below is. Its spaces are part of the source and therefore
part of whatever span holds them: they stay inside a unit's span, inside
that unit's verbatim literal, and inside a section's heading span, and
an implementation MUST NOT trim them from any of the three. Because
recognition begins after the run, a heading's title, a movement's name
and a verse number are exactly what they would be had the run not been
written: the run is never part of a title. A document and its
space-free twin therefore state the same structure over spans shifted by
the runs, and nothing else about them differs.

A **byte order mark** (U+FEFF) at byte zero of the source is a statement
about the encoding. Structure detection MUST skip it, so a document that
opens with one states the same structure as the document that does not,
and MUST NOT otherwise account for it: every span still counts the
source's own bytes, so the mark falls before the first span and it is one
scalar of the scalar count. At any other offset U+FEFF is ordinary
content and stays inside the unit that holds it.

### 2.1 Headings and movements

An **ATX heading** is a line whose marker text (§2) opens with one to
six `#` characters followed by a space or a **tab**. Its level is the
number of hashes. Its title is the rest of that marker text, trimmed,
with trailing `#` characters removed and trimmed again — so the leading
run is no part of a title, and `   #   ` is a heading whose title is
empty. A title MAY be **empty**: `# #` is a heading
with an empty title, which is a title, and not the absence of one. Seven
or more hashes open no heading, and hashes running straight into text
(`#Title`) open no heading — the space or the tab is the mark.

A **movement marker** is a line whose marker text (§2) opens with U+2042
(ASTERISM, `⁂`) followed by a space or a tab. Its name is the rest of
that marker text, trimmed, with one surrounding pair of `*` or `_`
removed and trimmed again, so the leading run is no part of a name. A
movement's level is one deeper than the nearest enclosing **heading**,
ignoring enclosing movements, so consecutive movements are **siblings**
rather than a descending chain.

A section runs from the first byte of its opening line to the byte before
the next section at its level or above, or to the end of the source. Its
parent is the nearest preceding section at a shallower level. A section's
*heading span* is its opening line alone, without the newline; that is
the span its identity is minted over (§5).

The document's **title** is the title of the first heading that is not a
movement, if the document has one.

### 2.2 Verses, paragraphs and rules

A **verse** is a line whose marker text (§2) opens with one or more
ASCII digits followed by `.` and then a space or a **tab**; the leading
run is no part of the number. The digits MUST parse as a `u64`; a
longer run of digits is no verse number at all, and the line is ordinary
prose. Nothing is truncated and nothing wraps, so a verse number a
consumer reads back is the number the document wrote. A verse runs from
its opening line to the last line before the next blank line, rule,
heading, movement, table row or verse.

A **paragraph** is any other run of non-blank lines, bounded the same way.

A **blank line** — one whose trimmed text is empty — closes the open
unit and is part of no unit. A **horizontal rule** — a trimmed line of
three or more characters, all `-` or all `*` — closes the open unit and is
part of no unit. A **table row** — a line whose trimmed text opens with
`|` — closes the open unit and is part of no unit; inside a concordance
section it is read as a citation row (§4).

Every unit therefore covers a contiguous span of the source, and a unit
never spans a section boundary.

## 3. The split law

A unit longer than `max_bytes` is cut into pieces. The law is stated over
one unit's span and applied to each in turn.

In summary, and normatively restated below: pieces of
**at most `max_bytes` bytes**, unconditionally; a cut at the last newline
at or before the bound, where the cut lands on that newline and
no piece carries it; else a cut at the last scalar boundary at or before
the bound; a continuation snapped backward to a line start, else to a
scalar boundary, and never before the unit's own start; a cut is
never inside a scalar; and a split is never across a heading.

1. **The bound holds without exception.** Every piece is
   **at most `max_bytes` bytes**, line or no line. A profile MUST declare
   `max_bytes` of at least **4**, the width of the widest UTF-8 scalar,
   and an implementation MUST refuse a smaller bound: a piece could not
   be both inside the bound and outside a scalar. A profile MUST declare
   `overlap` **strictly under** `max_bytes`, and an implementation MUST
   refuse an overlap at or over it: a continuation could otherwise reach
   back over the whole piece it continues and advance by as little as a
   byte.

2. **The cut.** While the remainder of the unit is longer than the bound,
   let `bound = piece_start + max_bytes`. The cut is the offset of the
   **last newline at or before the bound**, if there is one after the
   piece's first byte; otherwise it is the **last scalar boundary at or
   before the bound**, and never before the end of the piece's opening
   scalar.

3. **A cut newline belongs to no piece.** Where the cut is a newline, the
   cut lands **on** that newline, and no piece carries it: the piece ends
   at the cut, and the next piece resumes after it. Coverage is therefore
   *every byte of the unit except the newlines cut at*, and nothing else
   is ever skipped. Where the cut is a scalar boundary, the next piece
   resumes at the cut and no byte is skipped at all.

4. **The overlap.** The next piece starts `overlap` bytes before the cut,
   **snapped backward to a line start** — the byte after the nearest
   preceding newline inside the piece — and, where the piece holds no
   newline to reach, snapped backward to a scalar boundary. The
   continuation MUST NOT start at or before the piece's own start; where
   the snap would, the continuation starts at the resume point of rule 3
   instead. A continuation therefore always advances, and the split
   always terminates.

5. **Never inside a scalar.** No piece begins or ends inside a UTF-8
   scalar, at any bound and at any overlap.

6. **Never across a heading.** A piece is a piece of one unit, and a unit
   never spans a section boundary, so every piece carries the section,
   the lineage and the verse number of the unit it is part of, and no
   piece falls forward into the section a following heading opens.

Each piece after the first records the piece it **continues**. A unit that
was never cut is a first piece that continues nothing.

## 4. The concordance law

A section whose heading is `Concordance`, compared without regard to
ASCII case — in practice the line `## Concordance` — is a **concordance
section**. Table rows inside it are read as citations; table rows
anywhere else are structure and are read as nothing.

**Inside is containment (§1.1), not innermost.** A concordance MAY be
organised into subsections — one per volume, one per surveyor — and the
rows under such a subsection are inside the concordance section's span
exactly as the rows directly under its heading are. An implementation
MUST read a table row as a citation row whenever a concordance section is
in force at that line, at **any** depth, and MUST NOT ask only which
section is innermost. A row lost to the innermost reading is lost in
silence: it is read as nothing, so it is neither a citation nor a
malformed row, and no surface reports it — which is the one failure this
law forbids by name. A heading at or above the concordance's own level
closes it, and the rows of a table after that are structure again; an
implementation MUST NOT lift them.

A row is `| verses | canon sources | anchors |`, in that column order. Its
cells are the fields of the trimmed line delimited by its **unescaped**
`|` characters, with one leading and one trailing unescaped `|` removed,
each **unescaped** and then trimmed (§4.1).

* The **verse range** cell is `4`, `2-5`, or `2–5` with an en dash
  (U+2013). Both endpoints MUST parse as `u64` and the first MUST be at or
  under the last.
* The **canon source** and **anchor** cells each state zero or more
  backticked names. Only backticked text is read: prose beside an anchor
  lifts nothing, which is what lets a row carry a note.

**Zero is one of those numbers.** A row whose anchor cell names nothing —
because it is empty, or because it carries prose alone — is a readable
row and MUST NOT be refused: it states a verse range and canon sources
for a place the canon has no anchor for yet. It lifts onto every verse of
its range this document carries exactly as any other row does, and the
node that lift mints is stated in full (§11.1), so what such a row says
is in the graph and reachable there.

The first row of a table, and any row whose every cell is a run of `-`
with optional `:` alignment markers, are the table's **frame** — its
header line and its delimiter line — and are read as nothing. The frame
test is asked **only** of a row that could not be read as a citation, so
it can never swallow a row that would have lifted.

A row that is neither frame nor readable — fewer than three cells, or a
first cell that is not a verse range — is **malformed**. A malformed row
is not a refusal, because nothing is minted from it; but it MUST NOT be
dropped in silence either. An implementation MUST report malformed rows
as data, so that a consumer can name them, count them, or fail its own
build on them.

### 4.1 The cell-escape law

A `\` inside a table row escapes the character after it. Exactly two
escapes are recognized, and an implementation MUST recognize both and no
others:

* `\|` is a literal `|`. It is **not** a delimiter — neither between two
  cells nor as the row's own leading or trailing delimiter — and the cell
  holding it carries a `|`. This is GFM's own spelling of a pipe inside a
  table cell, so a row carrying one is ordinary input.
* `\\` is a literal `\`, which is what keeps the `|` of `\\|` a delimiter
  standing after a literal backslash rather than an escaped pipe.

A `\` before any other character is ordinary content and keeps its
backslash: `\n` in a cell is the two characters `\` and `n`, and a `\` at
the end of a row's line escapes nothing and is content. Nothing else about
a cell is interpreted; this is the table's escape, not Markdown's inline
grammar.

A cell's **value** is the text its delimiters bound with those two escapes
resolved, then trimmed. The verse range and the backticked names are read
from that value, so a canon source path or a note MAY carry a `|`.
Splitting a row on every `|` regardless of escape is non-conforming, and
it fails silently rather than loudly: such a row still states three cells,
so it still reads as a citation and still lifts onto its verse, carrying
the wreck of two cells cut in the wrong places. It is neither unmatched
nor malformed, so nothing reports it.

### 4.2 The multi-document canon

A citation lifts onto the **first piece** of each verse of its range that
this document carries: a citation states one fact about a verse, not one
about each of its pieces.

**A concordance MAY cover a canon wider than the document that carries
it.** One table shared across the volumes of a canon names verses that
live in sibling documents; a row whose verses are all elsewhere
lifts nothing here, and is not an error. An implementation MUST NOT
refuse such a row, and MUST report it — along with, for a row lifted in part,
the verses of its range this document does not carry. The reason to
report is that the same shape has a second cause, a typo in a verse
number or a row left behind by an edit, and only the consumer knows which
of the two its corpus is.

A range, though, is two `u64`s the **document** wrote, and this law is
total on every input it does not refuse. `| 1–18446744073709551615 |` is
a readable row and MUST NOT be refused, so an implementation MUST NOT
decide what a row lifted by walking its range, and MUST NOT state the
verses it did not lift one at a time: both are bounded by what the row
claims rather than by what the document carries, and on that row neither
terminates. The walk goes the other way — over the verses this document
carries that fall inside the range — and the verses of the range this
document does not carry are stated as the **maximal runs** they form,
each an inclusive pair, in ascending order, none of them touching. A row
states no more runs than it lifted verses, plus one, so the cost of
answering a row is bounded by the document. A row that lifted nothing at
all states its whole range as one run.

### 4.3 The anchor lift

Where the profile declares a **canon base**, an anchor is lifted to the
IRI `base ++ anchor` — **pure concatenation**, never RFC 3986 reference
resolution. Resolution is wrong here: it would dissolve a fragment base's
`#` and fold a dot segment away, so the minted IRI and the base the
consumer declared would part company.

Concatenation is exactly why the result MUST be checked rather than
trusted. Before any output is produced, every anchor of the table — not
only those whose verses this document carries — MUST be walked, and
`base ++ anchor` MUST

1. be an absolute IRI (it parses as an RFC 3987 IRI and carries a
   scheme), and
2. lie **under** the declared base.

Containment MUST be decided by relativization: the minted IRI is under
the base exactly when a relative spelling of it against that base exists,
which is decided by round-tripping that spelling back through RFC 3986
resolution, and not by comparing strings. That rule is the only one that
answers for a **fragment** base — `https://example.org/canon#` ++
`tide-line` relativizes to `#tide-line` and is admitted, where a prefix
test would have had to special-case the `#` and a resolution test would
have thrown it away — and it is what closes two escapes spelled entirely
in lawful IRI characters, which no character blacklist can see: an anchor
that climbs out of a path base (`../x` under
`https://example.org/canon/` resolves to `https://example.org/x`, which
has no relative spelling against the base), and an anchor that extends
the base's own host into a different authority (`.elsewhere.example/x`
concatenated onto `https://example.org` lands on
`example.org.elsewhere.example`). Both MUST be refused, naming the anchor
and the verse range of the row that wrote it.

Nothing about an anchor's *bytes* is refused. The law is IRI-lawfulness
and never ASCII: RFC 3987 `ucschar` is inside an IRI, so `中文` mints and
mints verbatim. Where no canon base is declared, nothing is minted, an
anchor is a typed literal, and no anchor is refused at all.

## 5. Identity

Node IRIs are **content-addressed** and carry the law that cut them
inside them.

A profile has **two** canonical, line-oriented preimages, and they answer
two different questions. Both are UTF-8 plain text, one fact of the law
per line, unframed: a newline ends one fact and begins the next, and no
field is length-prefixed or quoted.

**The chunking preimage** states only what decides where a unit starts
and ends and what bytes are in it: the profile's name; its version; the
dialect grammar — the heading, movement, verse, paragraph, rule and
table-row forms, the `u64` verse bound, the trimmed-line rule that makes
CRLF state its LF twin's structure, the leading-indent bound with its tab
clause, and the byte-order-mark rule; the clause that a table row is the
content of no unit whether or not a concordance section is in force;
`max_bytes`; `overlap`; and the split law with its cut, resume and snap
clauses. Its digest is the **chunking id**.

**The emission preimage** is the chunking preimage **verbatim, as a
prefix**, followed by every further clause that decides a byte of the
emitted graph: the vocabulary, written as one base **and the local names
derived under it** where every IRI derives from one base and as one line
per IRI otherwise — the names are stated, and not only the base, because
a base names a namespace and not a term set (§6); the canon base, written
`canon base <value>` where one is declared and `canon base none` where
none is — the absence MUST be stated and
MUST NOT be left implied by a missing line; the
section-level rule for movements; the rest of the concordance law — the
trigger heading, the containment reading of *inside it*, the column
order, the cell-escape law, the verse-range spellings, the backtick rule,
the report-never-refuse rule, and the anchor lift with its containment
test; the three identity formulas; the digest algorithm tag; and the
emission law of §11, term list and reification shape included. Its digest
is the **contract id**.

The emission preimage states the whole law because the contract id is the
handle a consumer keeps: two runs that agree on the id MUST agree on
every byte they emit. A clause left out of it would be a clause a
producer could change while the id stood still, and a consumer holding
that id would have no way to learn of it. An implementation MUST
therefore re-mint the contract id when it changes any clause of this
specification, the emission law among them, and MUST re-mint it when the
profile's canon base changes — a declared base turns every citation
object from a typed literal into a minted IRI, so two profiles differing
only there emit different bytes and MUST NOT share an id.

Because the chunking preimage is a prefix of the emission preimage, no
clause that can move a unit boundary is outside either id: the contract
id can never stand still while a boundary moves.

The two ids are two because they are held for two different lengths of
time. The contract id answers for the graph and moves whenever a byte of
it would; the chunking id answers for the *cut*, and a consumer that
embedded a document's units holds vectors that are stale exactly when a
unit's bytes move and at no other time. An implementation MUST NOT
address chunk parameters by the contract id, and MUST NOT invalidate a
chunking identity for a change — a renamed term, a declared canon base —
that moves no boundary.

Because both preimages are line-oriented and unframed, a profile's name
MUST NOT carry a control character: a newline inside a name would state
further facts of the law rather than name it, and two profiles could then
describe themselves the same way. The refusal is a bound of the identity
format, not a taste in names. A canon base needs no such refusal: an
admitted base is an absolute IRI (§4.3) and the IRI grammar admits no
control character.

A **unit's IRI** is `<node base>unit:<alg>:<hex>`, where `<alg>` is the
digest algorithm tag (`sha256` in this version) and `<hex>` is the digest
of a length-prefixed preimage of, in order:

1. the kind (`unit`),
2. the source id,
3. the profile's **chunking id**,
4. the byte span's start, as eight bytes little-endian,
5. the byte span's end, likewise,
6. the digest algorithm tag,
7. the digest of the span's own bytes.

Field 3 is the chunking id and MUST NOT be the contract id. A node's
identity answers *what this is* — this span of these bytes of this
document, cut by this law — and a vocabulary or a canon base changes what
is **said** about a unit, never what the unit is. Were the contract id
the field, a consumer that merely declared a canon base, or renamed one
predicate, would re-mint every unit and section of its corpus and orphan
every reference it had stored, and two graphs describing the same text
under two vocabularies could never be merged. With the chunking id there
they describe the same nodes and merge, and the `sliceProfile` literal on
the document node (§11) still tells the two laws apart. The one node
whose identity must notice the canon base is the citation, and it
notices it in its own content digest, below.

Every field is prefixed with its length as eight bytes little-endian, so
no two field sequences share a preimage. A **section's IRI** is the same
formula with the kind `section`, over the section's *heading span* and
that span's bytes.

A **citation's IRI** is the same formula with the kind `citation`, over
the concordance row's own line span, and with the content digest taken
over a length-prefixed preimage of: the row's line bytes, the IRI of the
unit the row lifted onto, and then the **triple terms the node reifies**,
each rendered exactly as §11.2 writes it, in the order the row wrote its
anchors. A citation names a **pair**, so both halves of it MUST be inside
the identity: one row lifting onto two verses is two citations, and two
rows lifting onto one verse are two more. The pair is the whole of the
count, and the anchors are no part of it: a row naming five anchors on
one verse is **one** citation carrying five reified terms, and a row
naming none on that verse is **one** citation carrying none. Because the
unit's IRI is a field of the preimage, a unit that re-mints re-mints
every citation of it, and a row copied verbatim into a second document
mints different citations there.

The reified terms MUST be inside it, and the row's line bytes are not a
substitute for them. A citation node is a **reifier of one lifting**: it
identifies one row's lifting onto one unit, and it states, as
`rdf:reifies` objects, **every** cited edge that lifting produced — none,
one, or many (§11.1). What the graph states is not what the row wrote —
under a declared canon base the object is the IRI `base ++ anchor` and
under none it is the anchor as a literal typed with the anchor datatype,
and the predicate is whatever the vocabulary's `cites` term names — so
one row read under two profiles can state two different sets of triples
while every byte of the row stands still.

The identity MUST therefore tell apart two liftings that reify
**different sets of terms**, and that is why the terms are in the
preimage. Were they outside it, both readings would mint the **same**
node IRI, and two graphs whose citation nodes carried that one IRI would
assert different reified edges under it. Merging them would then
fabricate in silence: the merged node reifies the union of the two sets
and so asserts edges no single document ever stated of it, while
answering to an IRI that promised one lifting. With the terms inside,
the two liftings are two nodes and a merge states exactly what each
document stated. The whole term is the field, subject, predicate and
object together, because the whole term is what is reified, and the terms
are fields in the order the row wrote its anchors, so the preimage
distinguishes not only *which* edges were reified but the order the row
named them in. A row that lifted no anchor contributes no such field and
reifies nothing — and is still exactly one node, with exactly one
identity (§11.1).

Five consequences are load-bearing, and an implementation MUST
reproduce all five:

* Two byte-identical paragraphs in one document are two distinct nodes,
  because the span is inside the identity.
* An insertion moves every later span and re-mints every later unit while
  leaving their text untouched; a same-length substitution re-mints only
  the unit it touches and moves no boundary. Those are opposite
  signatures, and both are observable.
* The digest algorithm is inside both the preimage and the IRI, so
  another producer can mint the same shape under another algorithm
  without redefining the preimage.
* A unit's content digest is emitted as data (§11) with the same lexical
  hex the unit's IRI carries, so a consumer holding the source proves the
  binding between a node and its bytes by comparison and never by
  re-encoding.
* Declaring a canon base, or changing a vocabulary term other than the
  node base, re-mints every **citation** of a document and leaves every
  unit and section where it was. The two signatures are opposite and both
  are observable, and they are the shape of the two ids: what a unit *is*
  did not change, and what a citation *reifies* did.

### 5.1 The three identities, and the verification law

A profile states **three** identities and no two of them are ever equal.

The **contract id** is the law id: a digest over the unframed,
line-oriented emission preimage, and the id this specification writes
into the `sliceProfile` literal of §11. The **chunking id** is a digest
over the unframed chunking preimage, and the id this specification writes
into node identities. The third is the **chunking-stage id** of an
embedding family, derived over that family's canonical stage encoding,
which is tag-and-length framed rather than line-oriented.

Because the framings differ, no family can reproduce either of the first
two, and because the emission preimage is strictly longer than the
chunking preimage, those two are never each other. An implementation MUST
NOT write the contract id or the chunking id where a chunking-stage id is
expected.

An implementation that offers a document to an embedding pipeline MUST
offer the law as a stage in that pipeline's own encoding, carrying **the
chunking preimage** as its parameters, and use the id derived from *that*
wherever a chunk names its chunking contract. The parameters are the
chunking preimage and not the emission preimage because a chunking stage
that carried the wider law would be mislabelled: it would announce a new
chunking contract for a renamed predicate, and every chunk of an
unchanged cut would be declared stale. Since the chunking preimage is a
prefix of the emission preimage, the stage still cannot stand still while
a boundary moves.

A unit offered as a chunk carries its byte span, its scalar span and its
content digest unchanged. The **verification law** for such a chunk is
the kernel's: given the document's exact bytes, re-derive the span's
digest and both scalar coordinates and refuse any disagreement, an
out-of-bounds span and a span cutting a scalar in half among them. An
implementation that also states this refusal in its own vocabulary MUST
state it by applying that law rather than by restating it, so that both
sides refuse the same bytes for the same reason.

## 6. The vocabulary

This specification mints no ontology. Every class, predicate and datatype
IRI, and the base node IRIs are minted under, is **configuration** an
implementation takes from its caller. There is no default namespace and
no fabricated fallback: an implementation MUST refuse an IRI that is
empty, that cannot be written as an IRI reference, that is no IRI
reference at all, or that is an IRI reference carrying no scheme, naming
the field that carried it.

The last two are different refusals and an implementation MUST state
them apart (§9.1). A field holding `https://example.org/%` is not a
relative IRI: it is not an IRI.

The term set is fixed. Deriving a whole vocabulary from one base appends
these local names to it, and takes the base itself as the node base:

| Role | Local name | Role | Local name |
| --- | --- | --- | --- |
| document class | `Document` | span start | `byteStart` |
| heading section class | `Section` | span end | `byteEnd` |
| movement section class | `Movement` | unit scalar-span start | `scalarStart` |
| unit class | `Unit` | unit scalar-span end | `scalarEnd` |
| source digest | `sourceDigest` | unit text | `text` |
| media type | `mediaType` | unit content digest | `contentDigest` |
| document byte length | `byteLength` | innermost section | `section` |
| slicing profile | `sliceProfile` | verse number | `verse` |
| document title | `title` | heading lineage | `lineage` |
| in-document | `document` | previous piece | `continues` |
| parent section | `parent` | citation of an anchor | `cites` |
| section level | `level` | canon source path | `canonSource` |
| unit ordinal, section ordinal | `ordinal` | citation class | `Citation` |
| section heading text | `heading` | the unit a citation is of | `unit` |

with the datatype IRIs `digest` (a source digest), `media` (a media
type), `profile` (a slicing profile's label), `headingText` (a section's
heading and the document's title), `lineagePath` (a heading lineage),
`anchor` (a canon anchor under no canon base) and `path` (a canon source
path).

`canonSource` is a term **about a citation**, not about a unit: it
annotates the citation node a row's lift mints (§11), which is what keeps
a row's paths beside that row's own anchors. `Citation` and `unit` are
about that node too: the class it is stated with, and the back-edge to
the unit it is an edge of, both of which it carries whatever its row
lifted (§11.1).

The emission preimage states these local names, and not merely the base
they are derived under (§5), because a base names a namespace and not a
term set: two producers deriving different names under one base emit
different graphs. Renaming a term, or extending the set, therefore
changes that preimage and re-mints every contract id and every citation
node under it (§5) — and leaves every unit and section where it was,
because a term set moves no boundary. That is the mechanism working, and
an implementation MUST NOT hold a term set constant to preserve an id.

**No IRI of this term set is written in two roles.** Every local name
above is distinct, so the classes, the predicates and the datatypes of a
derived vocabulary are three disjoint sets of IRIs, and an implementation
deriving a vocabulary from a base MUST use these names and MUST NOT
collapse two of them into one. The datatype of a heading is accordingly
`headingText` and not `heading`, and the datatype of a lineage
`lineagePath` and not `lineage`: the bare names are the predicates that
carry those literals.

That is a requirement rather than a preference because the namespace
designated below is meant to be *exchanged* under, and a namespace that
is exchanged is described: no OWL or SHACL document can declare one IRI
both an `owl:DatatypeProperty` and an `rdfs:Datatype`, so a term set that
asked it to could not be described at all, and no reasoner or shape
validator would accept the description. A caller setting the fields
explicitly remains sovereign over its own terms and MAY spend fewer of
them — this law can neither stop that nor name what it costs, and places
no requirement on how such a caller names anything. Under the designated
namespace the question does not arise: the description is well-formed by
construction.

The namespace this specification **designates** is

```
https://w3id.org/purrdf/markdown#
```

and it exists for one reason: two deployments that each derive their own
namespace publish two graphs no consumer can join without a mapping, and
the mapping is the part that never gets written. Documents meant to be
*exchanged* SHOULD use the designated namespace. A deployment sovereign
over its own terms MAY derive the same term set under any base it likes
and loses nothing — the law, the identities and the split are the same
either way, and the profile's contract id tells the two apart. The
designated namespace MUST NOT be reached for on a caller's behalf; an
implementation MAY offer it, but only by name.

One honest caveat, and it is owed here as much as it is owed of any
namespace this document cites: the designated namespace is not publicly
dereferenceable. `https://w3id.org/purrdf/markdown#` is a stable
identifier, not a URL that resolves today; a redirect for it is being
registered, and no date is promised here. Nothing in this law rests on
that. RDF does not require an IRI to dereference, so the term set, the
three identities, the split and every conformance requirement above hold
exactly as written whether the namespace resolves or not, and a document
exchanged under it is no less readable for it.

The only IRIs this specification brings of its own are the standard's:
`rdf:type`; `rdf:reifies`, which binds a citation node to each triple
term it reifies (§11); `xsd:integer` for every ordinal, level, byte
offset and scalar offset; and `xsd:hexBinary` for a unit's content
digest. None of the four is configuration. `rdf:type` and `rdf:reifies`
are shapes of the
RDF data model rather than terms of this vocabulary, and the two XSD
datatypes are the standard's names for the values they carry; a
deployment that renamed any of them would publish a graph no consumer
could read with the data model it already has.

## 7. Ordering, and why it is not a list

Order among sections, and order among units, is stated with one plain
**ordinal** predicate carrying an `xsd:integer`, counting from zero in
document order. This specification does **not** use `rdf:List`,
`rdf:Seq`, or any collection vocabulary, and that is a decision rather
than an omission. Three technical facts drive it.

**The container vocabulary is legacy.** RDF 1.2 Schema retains `rdf:Seq`
and the `rdf:_1`, `rdf:_2`, … membership properties for compatibility and
describes them non-normatively; nothing in the semantics constrains a
sequence's membership properties to be contiguous, to start at one, or to
be used at all. A consumer that reads `rdf:Seq` is reading a convention,
not a guarantee, and two producers may honour different conventions
without either being wrong.

**List traversal has no ordering guarantee in the query language.** A
SPARQL property path such as `rdf:rest*` matches an *arbitrary* number of
steps and returns a solution *set*; path results are unordered and
duplicate-insensitive, so the query that walks a list does not, by
itself, tell you the order it walked it in. Recovering position from a
list therefore costs either a recursive query the language does not have
or a post-hoc sort on something else — and if there is something else to
sort on, that something else is the ordinal, stated directly.

**Order-as-list has been tried and withdrawn.** The list form is
expensive to write (each cell is a blank node and two triples), expensive
to edit (inserting one element rewrites every cell after it), fragile
under graph merging (two merged copies of one list are two lists), and it
cannot survive the operation this law performs constantly: re-slicing a
revised document, where an insertion moves every later node. An ordinal
survives all of it. It is one triple, it is indexable, it sorts in the
query language with `ORDER BY`, it merges idempotently, and it says
exactly what it means.

Ordinals in this law are dense within a document and are **not** stable
across revisions: an insertion renumbers what follows it, exactly as it
re-mints what follows it (§5). A consumer that needs a stable handle uses
the node IRI, or re-anchors on the unit's text (§8).

## 8. Content anchors

A span is exact and cheap, and it dies at the next edit. This
specification therefore also defines a **content anchor** for a unit: its
exact text, a bounded prefix of the source immediately before it, and a
bounded suffix immediately after it. The bound is a byte count
(32 bytes in this version), and the prefix and suffix MUST be snapped to
scalar boundaries — forward for the prefix, backward for the suffix — so
that an anchor is always text and never a partial scalar. At the source's
edges the context is short, or empty.

The anchor is deterministic in the bytes and is **not** an identity: two
identical paragraphs in one document share their exact text and differ
only in their context, which is what the context is for. Its purpose is
re-anchoring across revisions, where every byte offset has moved. This
specification does not define a matching procedure; a consumer MAY use
any, and the three strings are what it needs.

## 9. Conformance

An implementation conforms to this specification when, for the reference
profile and every input of the vector suite published with it — the file
`crates/markdown/tests/slicer.rs` and the fixtures it reads — it produces
**byte-identical** output, and states the same refusal, by kind and by
the values that name it, for every input the suite refuses.

**Determinism is normative.** The same source bytes under the same
profile MUST produce byte-identical output, on every run, on every host
architecture and on every target. An implementation MUST NOT read a
clock, consult ambient state, iterate an unordered collection whose order
it then emits, or otherwise admit a source of variation into its output.

An implementation MUST refuse, by name and before producing any output:
a source that is not UTF-8; an empty source id; a source id that cannot
be written as an IRI reference; a source id that is no IRI reference at
all; a source id that is an IRI reference carrying no scheme; a
vocabulary field that is empty, unwritable, no IRI reference at all, or
an IRI reference carrying no scheme; a profile name carrying a control
character; `max_bytes` under 4; `overlap` at or over `max_bytes`; a
declared canon base that is no IRI reference at all — the empty base
among them — or is one carrying no scheme; and an anchor that mints no
lawful IRI under a declared canon base or mints one outside it. An
implementation MUST NOT refuse anything else, and in particular MUST NOT
refuse a concordance row that lifts nothing here (§4.2) or one that is
malformed (§4).

**Every refusal is stated before output, and the projection is
therefore total.** An implementation that exposes the two phases
separately — a stand-off model, and a projection of it into the graph —
MUST bind the profile to the model at the moment the model is admitted,
and its projection MUST apply that profile and MUST NOT accept another.
The projection is total only because every law it applies has already
been answered for on these exact bytes, and a profile supplied at
projection time is a law nothing answered for: a document admitted with
no canon base had its anchors walked against nothing, because none are
minted (§4.3), so projecting it under a base would mint the very IRIs
the anchor lift exists to check. That is a containment bypass spelled
entirely in admitted calls, and the projection's signature is what
closes it.

### 9.1 Not absolute, and not an IRI

Three of those refusals — the source id, a vocabulary field, and the
declared canon base — ask one string one question, and the question has
two halves an implementation MUST NOT merge into one answer.

A string the IRI grammar reads whole and finds to carry no scheme is
**relative**. It denotes something only in relation to whatever holds
the claims, and the remedy is to write it absolute.

A string the IRI grammar cannot read at all is **malformed**. It has no
scheme in the sense that it has nothing: `https://example.org/%` ends in
a truncated percent-encoding and `http://[not-an-ipv6` never closes its
IP-literal, and both plainly carry the characters of a scheme. An
implementation MUST state such a refusal as malformed and MUST NOT state
it as relative, because "carries no scheme" is then a false statement
about the input, and a false statement sends its reader to look for a
defect that is not there.

A malformed refusal SHOULD carry the finding of the IRI grammar that
read the string — which byte, and what about it — rather than a sentence
of the slicer's own composition. The slicer does not own that grammar,
and a second wording of one law is a second law that will drift.

A conforming implementation SHOULD expose the stand-off model — the
sections, units, citations, unmatched and malformed rows, scalar spans,
the containment lattice and content anchors — and not only the emitted
graph. The graph is one projection of the model. It now carries the
pairing inside a concordance row (§11) and a unit's scalar span and
content digest, and what it still cannot state is what names no node at
all: a row that lifted nothing here, a row too malformed to read, and a
unit's surrounding context.

## 10. Provenance guidance

Where a deployment records the plan or the derivation behind a slicing
run — who sliced what, under which profile, to what end — this
specification recommends `logic:Plan`,
`https://blackcatinformatics.ca/logic/Plan`, defined by the **gmeow**
ontology's logic slice at `<https://blackcatinformatics.ca/gmeow/slices/logic>`
and composing with the profile identity of §5.

It is recommended for two reasons. The first is structure: a `logic:Plan` is
a transaction program whose primitive operations invoke action schemas and
whose success criterion is a goal expression, so it carries ordering, guards,
branching, concurrency, loops and fallback, and quantifies its own success
over nondeterministic outcomes through `logic:planSuccessMode` — none of
which a plan node reduced to a descriptive label can carry. The second is
that the slice it is defined in is a provable layer, so a recorded plan is
machine-checkable structure rather than annotation a reader must take on
trust.

PROV-O is not a rival here. gmeow authors declared bridge rows from the
plan/enactment spine onto PROV-O — `logic:Plan` to `prov:Plan`,
`logic:Enactment` to `prov:Activity` — alongside its OPMW and OBI bridges,
each row disclosing what the projection loses rather than claiming an
identity it does not have. A deployment that records the provable form
therefore gives up no PROV interoperability; it can project onto PROV-O where
interoperability is what it needs. Nothing in this section is normative: this
law states what a document *is*, and a deployment states how it came to run.

One honest caveat: the gmeow namespace is not yet publicly dereferenceable.
The IRIs above are stable identifiers, not URLs that resolve today.

## 11. The emission law

This section states the graph as it is emitted at **this version of this
specification**. Sections 1 through 9 are the law; this is the current
projection of it, and it is stated separately for that reason. It is
nonetheless inside the profile's contract id (§5): changing it re-mints
every contract id, and every citation node under one.

Output is one **claim** per node, in document order — the document node
first, then sections and units interleaved as they occur, whichever
starts first. A claim is the node's triples written as N-Triples lines,
each line terminated by `\n`, the lines **sorted bytewise** and
de-duplicated. N-Triples is a subset of Turtle, so a claim is served as
`text/turtle`.

A claim's *subject* is the node it is about. One kind of line has another
subject: the citation nodes of the rows that lifted onto a unit travel in
that unit's claim, because a citation is an edge of the unit and nothing
asks after it on its own.

**The document node.** Subject: the source id. `rdf:type` the document
class; the source digest as `sha256:<hex>` typed with the digest
datatype; the media type `text/markdown` typed with the media datatype;
the byte length as `xsd:integer`; the slicing profile as
`<name>:<contract id hex>` typed with the profile datatype; and, if the
document has one, the title typed with the heading datatype.

**A section node.** Subject: the section's IRI (§5). `rdf:type` the
heading class, or the movement class for a movement; the in-document
predicate to the source id; the level and the ordinal as `xsd:integer`;
the heading text typed with the heading datatype; the section span's
start and end as `xsd:integer`; and, where there is one, the parent
predicate to the parent section's IRI.

**A unit node.** Subject: the unit's IRI (§5). `rdf:type` the unit class;
the unit's verbatim text as **exactly one plain literal** — an
`xsd:string`, written with no datatype IRI — so that a full-text or
embedding index that selects plain strings sees one text per unit and
nothing else; the in-document predicate to the source id; the ordinal as
`xsd:integer`; the byte span's start and end and the **scalar** span's
start and end as `xsd:integer`; the unit's **content digest** as the
lowercase hex of §5's span digest, typed `xsd:hexBinary`; where the unit
sits in one, the section predicate to the innermost section's IRI; where
the unit is a verse, its number as `xsd:integer`; where the heading stack
is non-empty, the lineage as its headings joined with ` > ` and typed
with the lineage datatype; and where the unit continues another piece,
the continues predicate to that piece's IRI.

Every one of those but the text is **typed**, which is the point: the
scalar offsets and the digest are data a consumer needs — a UTF-16 host
that counts characters, a reader proving a hit is bound to its bytes —
and putting them in the graph MUST NOT cost the unit its single plain
literal. An implementation MUST NOT emit a second plain `xsd:string` on a
unit.

### 11.1 Citations, and the pairing they keep

A concordance row that lifted onto a unit states, in that unit's claim:

1. **The asserted edge.** For each anchor of the row, a `cites` triple
   from the unit, whose object is the IRI `base ++ anchor` where a canon
   base is declared and otherwise the anchor as a literal typed with the
   anchor datatype. These are set-valued: a unit two rows cover states
   each distinct anchor once, because the claim's lines are
   de-duplicated.

2. **The citation node.** One node per (row, unit) — **exactly one**,
   however many anchors the row named, five or one or none — its IRI
   minted by §5's citation formula. It states, for **every** such node
   and whatever its row lifted,

   ```
   <citation> rdf:type <citation class> .
   <citation> <unit> <the unit> .
   ```

   and then, for each anchor of the row, one line on that same node,

   ```
   <citation> rdf:reifies <<( <unit> <cites> <anchor> )>> .
   ```

   where `<<( … )>>` is the RDF 1.2 **triple term** — the non-asserting
   form. The parentheses are not decoration: the bare `<< s p o >>` is
   the reifying-triple shorthand, which *also* asserts its triple and
   mints a reifier of its own, so re-parsing it would grow the graph. A
   triple term denotes the triple without asserting it, which is exactly
   what an `rdf:reifies` object requires; N-Triples admits the
   parenthesized form only, and only in **object** position.

   A node therefore reifies as many edges as its row lifted onto this
   unit: a row of two anchors gives **one** node carrying **two**
   `rdf:reifies` objects, and an implementation MUST NOT split it into
   two nodes, one per reified triple. The set of terms a node reifies is
   inside its identity (§5), so two liftings that reify different sets
   are two nodes and never one.

3. **The row's sources.** For each canon source path of the row, a
   `canonSource` triple **from the citation node**, whose object is the
   path typed with the path datatype.

The source annotates the citation edge, and that is the whole point of
the node. A verse two rows cover keeps each row's paths beside that row's
anchors: an N-Triples consumer groups the lines by their citation
subject, reads each triple term for the anchor it names, and has the row
back. The flat form — a `canonSource` on the unit — could not say it, and
that loss is what this shape repairs. A source named by two rows is now
stated twice, once per row, because it is two facts and not one.

An implementation MUST NOT emit `canonSource` on a unit, and MUST NOT
substitute another serialization for the triple term.

**A citation node is never an orphan.** The class and the back-edge of
clause 2 are unconditional: an implementation MUST emit both for every
citation node it mints, and MUST NOT make either one conditional on the
row having named an anchor, or a source, or anything else. The case this
answers for is a row that names canon sources and **no** anchors. Such a
row is lawful (§4), it lifts onto every verse of its range this document
carries, and so it mints a node — a node that reifies nothing, because
there is no anchor to reify. Were the class and the back-edge conditional
on an anchor, that node would carry its row's `canonSource` lines and
nothing else: no `rdf:type`, and no triple anywhere naming it as the
object of anything. Traversal from a unit is the only traversal this
section defines, and it would never reach that node, so the row's source
paths would be formally present in the graph and practically lost —
emitted, and unreachable.

With clause 2 stated unconditionally, **every citation node of a document
is reachable from the unit it is an edge of, in one step**, and
everything a row states about a unit is therefore reachable from that
unit: the anchors through the reified triple terms, the paths through
`canonSource`, and the node itself through `unit`. A conforming
implementation's emitted graph contains no citation node that no unit
reaches. The cost is two triples per citation node on **every** document,
including every document whose rows all name anchors, and it is stated
here rather than elided because it is paid on all of them: the invariant
is worth more than the lines, because a conditional invariant is one a
later row shape breaks in silence.

### 11.2 Writing terms

Term writing is the canonical N-Triples form and MUST NOT be
re-implemented loosely.

* A **literal** is written between `"` and `"`, escaping `\` and `"`,
  the readable forms `\n`, `\r` and `\t`, and every remaining C0 control
  character **and the DEL (U+007F)** as `\uXXXX` with uppercase hex
  digits. The C1 block (U+0080–U+009F) is left raw: the N-Triples literal
  grammar permits it. An implementation that leaves U+007F raw is
  non-conforming — the grammar forbids it there, and it is the escape a
  hand-written escaper most often misses.
* An **IRI** is written between `<` and `>`. It MUST NOT contain a
  character the IRIREF grammar forbids: the reserved delimiters
  ``< > " { } | ^ ` \``, the space, and the whole control range.
* A **triple term** is written `<<( ` subject ` ` predicate ` ` object
  ` )>>`, each component written by these same rules.

**The predicate position is an IRI position.** A predicate is written
between `<` and `>` by the IRI rule above, exactly as a subject and an
IRI object are, and an implementation MUST NOT keep a second rendering of
that rule beside the first. Every predicate this law emits is a
vocabulary field or one of the standard's own IRIs (§6), all of them
already answered for (§9), so the rule escapes nothing there and the two
renderings would agree on every byte of every conforming document. That
is precisely why the requirement is on the *writer* and not on the bytes:
a second rendering that is provably the identity today is a second
rendering that can drift tomorrow, and the drift is invisible until a
document carries the one character the two disagree about.

An implementation SHOULD therefore reach every one of these forms through
one canonical writer rather than maintain its own escaper beside it.

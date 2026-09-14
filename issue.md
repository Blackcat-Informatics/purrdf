# Issue #294: fix(rdf): the XML writers emit literals they cannot read back

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## The defect

`escape_xml` is transcribed twice, byte-identically, in the two XML-shaped writers:

- `crates/rdf/src/native_codecs/rdfxml.rs:1190`
- `crates/rdf/src/native_codecs/trix.rs:550`

Neither escapes `#xD`, and neither escapes the C0 controls. Both are wrong, in two different ways, and both are silent.

### 1. A carriage return in a literal round-trips as a line feed

XML 1.0 §2.11 requires a parser to normalize a literal `#xD` in character data to `#xA`. A CR that must survive has to be written `&#xD;`. We write it raw:

```
$ purrdf convert --from ntriples --to rdfxml cr.nt cr.rdf     # literal "a<CR>b"
    <ns0:p>a<CR>b</ns0:p>                                     # raw CR in character data
$ purrdf convert --from rdfxml --to ntriples cr.rdf -
<urn:ex:s> <urn:ex:p> "a\nb" .                                # a DIFFERENT literal
```

Exit zero both ways. The same loss occurs through TriX.

### 2. A C0 control produces non-well-formed XML

```
$ purrdf convert --from ntriples --to rdfxml nul.nt nul.rdf   # literal "a<U+0000>b"
exit=0                                                        # raw U+0000 in character data
$ purrdf convert --from rdfxml --to ntriples nul.rdf -
purrdf: error native-codec-parse: RDF/XML: a non-XML character found at 4:13   exit=1
$ python3 -c "import xml.etree.ElementTree as ET; ET.parse('nul.rdf')"
xml.etree.ElementTree.ParseError: not well-formed (invalid token): line 4, column 12
```

We emit, at exit zero, a document our own reader refuses and no conforming XML parser will accept. U+0000 is not in XML's `Char` production at all, so no escape rescues it — the serializer has to refuse.

## Why this is filed rather than fixed in place

It was found while auditing the scanner-terminal work, which collapsed five other escape transcriptions onto one law. These two are a sixth and seventh, and the defect is the same shape as that work's headline.

But fixing it **changes emitted bytes** for any document whose literals contain a CR, and converts today's exit-zero-with-corrupt-output into a hard error. Either may legitimately move a frozen vector or a conformance row. That belongs in a change whose diff is about byte movement and is reviewed as such, rather than riding along in one about character classes.

## What closing this looks like

- One `escape_xml`, not two. The workspace already has a shared home for egress escaping in `crates/rdf-core/src/iri_escape.rs`.
- Escape `#xD` as `&#xD;` so a literal CR survives a round-trip.
- Refuse the C0 scalars XML's `Char` production excludes, naming the scalar, rather than emitting a document that cannot be read.
- Verification is **writer then reader**, not a unit test: write with the production writer, read back with the production reader, assert the graph is identical. That is the only check that would have caught either defect.
- Any frozen vector or conformance row that moves gets explained, never absorbed.

## Over-refusal watch

The C0 refusal is a new hard error, so it needs its executed valid neighbours: `#x9`, `#xA` and `#xD` are all in XML's `Char` production and must keep round-tripping, as must U+0085, U+00A0 and the astral planes.

Both defects pre-date the branch that found them, and neither is reachable from any acceptance criterion of the scanner-terminal work.


## Comments (0)


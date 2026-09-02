<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Base IRIs & Relative References

An RDF document may spell an IRI *relatively* — `<other.ttl>`, `<#me>`, `<>` —
and the reader must turn it into an absolute IRI before it can be a term. That
turning needs a **base IRI**. PurRDF resolves every relative reference, in every
syntax and on every surface, through one layer: `purrdf-iri`.

> **This is a hard failure, and it used to be silent.** A relative IRI with no
> base in scope is now an error. Documents that previously "worked" this way did
> not: they interned a relative string as if it were an IRI, and emitted
> N-Triples that no conformant parser accepts. If a document of yours starts
> failing with `iri-relative-no-base`, it was already producing invalid RDF —
> give it a base (below) rather than working around the error.

## Where the base comes from

The precedence chain is RFC 3986 §5.1's, in its order. The first step that
yields a base wins:

| | Source | RFC 3986 | Example |
| --- | --- | --- | --- |
| 1 | An in-document base directive | §5.1.1 | Turtle/TriG `@base` or `BASE`, SPARQL `BASE`, RDF/XML `xml:base`, JSON-LD/YAML-LD `@context.@base` |
| 2 | A base the caller supplied | §5.1.2 | the `base` argument to `parse_dataset`, the CLI's `--base` |
| 3 | The document's **retrieval IRI** | §5.1.3 | the `file://` IRI of the file the CLI read |
| 4 | *(none)* — hard failure | §5.1.4 | `iri-relative-no-base` |

Step 1 nests: a `@base` inside a document rebinds relative to the base already
in force, so `@base <sub/>` under `http://example.org/dir/` yields
`http://example.org/dir/sub/`.

Nothing is ever invented. There is no default base, no fabricated `urn:`
placeholder, and no fall back to the current working directory. Step 4 is a
real, specified outcome, not a gap.

### Which surfaces have a retrieval IRI

Step 3 needs a retrieval IRI, and only a surface that opened the file itself has
one. **Three** do. The derivation lives in exactly one of them, and the other two
consume it rather than re-deriving one.

| Surface | Retrieval IRI? | Consequence |
| --- | --- | --- |
| `purrdf-slice` (slice tree, catalog, dependency fixes) | yes — **derives** it; the workspace's one implementation of §5.1.3 | a relative IRI in an on-disk slice document resolves with no flags |
| `purrdf-shapes` shape-union loader | yes — **consumes** `purrdf-slice`'s | each shape file parses under its own `file://` IRI |
| `purrdf` CLI, per input **file** | yes — **consumes** `purrdf-slice`'s | a relative IRI resolves with no flags |
| `purrdf` CLI, stdin (`-`) | no | a relative IRI is `iri-relative-no-base`; pass `--base` |
| Rust byte APIs (`parse_dataset`, `purrdf-rdf`, `purrdf-iri`) | no | pass a base, or the document must carry one |
| WebAssembly | no | as above |
| C ABI | no | as above |
| Python | no | as above |

Every surface that is handed **bytes** rather than a path is in the second group.
Bytes have no retrieval IRI, so §5.1.3 is vacuous there and §5.1.4 — the hard
failure — is the specified answer. This is deliberate rather than an omission: a
base invented from the local filesystem would differ per machine and leak local
paths into published RDF, which would break the byte determinism the whole toolkit
rests on. It is also why `purrdf-iri` and `purrdf-rdf` never touch the filesystem
at all — that is what keeps them wasm32-clean.

`purrdf-slice` derives the retrieval IRI from the **canonicalized** path,
translating Windows paths (including UNC hosts and the extended-length `\\?\`
prefix) into RFC 8089 form and percent-encoding each component. Its consumers
apply it only when nothing of higher precedence was given, and a path with no
usable `file://` IRI is a hard error naming `--base`, never a silent fall back to
"no base".

## Two grammar families

Whether a base can help at all depends on the syntax, not on the base:

| Family | Syntaxes | Relative reference |
| --- | --- | --- |
| Admits relative references | Turtle, TriG, RDF/XML, JSON-LD, YAML-LD, SPARQL | resolved against the base in force |
| Absolute-only by grammar | N-Triples, N-Quads, TriX, HexTuples | **rejected** — no base is ever applied |

The second family's grammars have no base directive and no relative-IRI
production. A relative reference there makes the document invalid for its own
syntax, so PurRDF reports a *different* code and **supplying a base will not
rescue it**: convert the source to Turtle or N-Triples-with-absolute-IRIs
instead.

Absolute references are never touched. In both families an IRI a document
spelled absolutely is taken lexically verbatim — `<http://a/bb/ccc/../d;p?q>`
survives intact, with or without a base in scope. Putting it through resolution
anyway would apply RFC 3986 §5.2.4 dot-segment removal, which is §6.2.2.3
*syntax-based normalization*, forbidden by RDF Concepts §3.2. Identical document
bytes must denote one graph with one canonical digest.

## Beyond documents: the IR boundary

Everything above is about *reading a document*, but a document is not the only
way a term reaches the store. You can also build a dataset directly — from
Python quad objects, from a GTS container, by reopening a `.pack` file, with
SPARQL `INSERT DATA`, or through a projection — and none of those pass a parser
that could resolve a base for you.

The absoluteness rule therefore does not live in the codecs. It lives at the
**interned term table** every one of those paths necessarily arrives at, so a
relative IRI is not merely rejected on the way in: it is unrepresentable. A
frozen dataset carrying one cannot be constructed, and the refusal reports the
same `iri-relative-no-base` code a parser would.

There is no base in scope at that boundary and there cannot be: a frozen dataset
is a set of *resolved* identities, with no document and no `@base` alongside it
to resolve against later. So a relative reference there is not a term awaiting
resolution — it is a term whose identity is unknowable. Resolve it against
whatever base your application means *before* it becomes a term.

### When you see the error

At the mutation that introduced it. `Store.add`, `MutableDataset.add`, the RDF/JS
`dataset.add`, and `purrdf_graph_insert` all refuse a relative IRI at the call,
naming the offending term:

```python
>>> store.add(Quad(NamedNode("rel"), p, o))
ValueError: iri-relative-no-base: relative IRI reference "rel" cannot be resolved:
no base IRI is in scope; add a base to the document (`@base`/`BASE` in
Turtle-family syntaxes, `xml:base` in RDF/XML) or pass a base IRI to the API
```

The refused quad does not land — the store is unchanged and still usable.

The check is repeated at every point where a working set becomes a dataset:
freezing a builder or a store's pending edits, serializing, canonicalizing or
digesting the result, and reopening a `.pack` file (whose bytes may have been
written by another engine, an older version, or corrupted on disk). That
repetition is deliberate. The freeze-time check is the *invariant* — it is what
makes a relative IRI unrepresentable in the IR from any ingress, including ones
that do not go through a mutation call at all. The insert-time check is the
*diagnosis*: it exists so the error can name the line that caused it. Neither
subsumes the other.

### One exception you may rely on

Blank node labels and literal lexical forms are arbitrary strings and are not
touched by any of this. Only IRIs are IRIs.

## The diagnostic codes

These codes are stable and machine-readable. Every codec, the CLI, the C ABI,
Python and wasm report the same code for the same condition.

| Code | Condition | What to do |
| --- | --- | --- |
| `iri-relative-no-base` | a relative reference in a syntax that admits one, with no base in scope | add `@base`/`BASE`/`xml:base`/`@context.@base` to the document, or pass a base to the API (`--base` on the CLI) |
| `iri-not-absolute-by-grammar` | a relative reference — including the empty reference `<>` — in N-Triples, N-Quads, TriX or HexTuples; **or** an RDF/XML element or attribute QName whose `xmlns:` namespace is itself relative | write the IRI in absolute form; **a base cannot help** — this position admits no relative reference |
| `iri-non-absolute-base` | the base *itself* has no scheme (RFC 3986 §5.1 requires an absolute base) | supply a base with a scheme, e.g. `http://example.org/dir/`. A filesystem path is a relative reference, not a base IRI — the CLI rejects one at the argument boundary and suggests the `file://` IRI you meant |

RDF/XML appears in both grammar families for a reason, and the row above is the
narrow half. Its `rdf:about` / `rdf:resource` / `rdf:ID` values *are* references
and do resolve against `xml:base`, which is why the table further up lists it as
admitting relative references. But an element or attribute *name* is composed from
an `xmlns:` declaration plus a local name; it is not a reference, so nothing
resolves it, and a relative `xmlns:ex="rel/"` composes to a relative IRI no base
may rescue:

```console
$ purrdf convert data.rdf --from rdfxml --to ntriples
purrdf: error iri-not-absolute-by-grammar: invalid IRI from an XML qualified name:
relative IRI reference "rel/p" is not permitted by this syntax (the caller-supplied
base, <file:///tmp/data.rdf>, is in scope but is never applied here); write the IRI
in absolute form; this syntax admits no relative IRI reference, so supplying a base
will not help
```

Note that the message names the base that *is* in scope and says it is deliberately
not applied there, so a caller who passed one is not sent hunting for a dropped
parameter.

The message rendered for each already carries its remedy, so a consumer that
prints the error alone still tells its user what to do.

## Writing a base out

Resolution has a mirror on the serialize leg. A syntax that can express a
document base emits one when a base is supplied, and relativizes its IRIs
against it:

| Syntax | Reads a base | Writes a base |
| --- | --- | --- |
| Turtle, TriG | `@base` / `BASE` | `@base` |
| RDF/XML | `xml:base` | `xml:base` |
| JSON-LD, YAML-LD | `@context.@base` | `@context.@base` |
| N-Triples, N-Quads, TriX, HexTuples | no | no — absolute IRIs only |

A format in the last row reaches its writer with no base and emits absolute
IRIs. That is not a silent drop: the format simply has no base surface, so
there is nothing to write, and the output stays valid for its grammar. On the
CLI there is one more step — if a `--base` was given and *neither* leg of the run
can spend it, the command is refused outright rather than accepting an inert flag.
See below.

## `--base` on the command line

`purrdf convert`, `query`, `update` and the other RDF-producing subcommands take
`--base <IRI>`, and it acts on **both legs**:

* **Parsing** — it is the caller-supplied base (§5.1.2), so it outranks the
  input's retrieval IRI but not an in-document directive. A parse leg can spend
  the base only if the source syntax admits a relative reference.
* **Serializing** — if the target syntax can write a base, it is emitted as the
  output document's base and the IRIs are relativized against it. A serialize leg
  can spend the base only if the target syntax emits a base directive.
* **Neither leg can spend it — a usage error, exit 2.** A base spent by *any one*
  leg is honoured, so `--from turtle --to ntriples --base …` is fine (the parse
  leg spends it) and so is `--from ntriples --to turtle --base …` (the serialize
  leg does). But `--from ntriples --to ntriples --base …` has nowhere to put it.
  Rather than exit 0 having silently ignored the flag, the CLI names both legs and
  refuses:

  ```console
  $ purrdf convert data.nt --from ntriples --to ntriples --base http://example.org/dir/
  purrdf: --base has no effect on this run: on the source `data.nt`, ntriples's
  grammar admits no relative IRI reference, so nothing in the document resolves
  against a base; and on the --to target, ntriples can express no base directive,
  so nothing is written under one or relativized against it. Drop --base, or name
  a syntax that carries one (turtle, trig, rdfxml, jsonld, yamlld)
  $ echo $?
  2
  ```

  The verdict is read off the format registry's `admits_relative_iri` and
  `emits_base` columns, so a newly registered syntax is classified by its own row
  rather than by a hand list. It applies to `convert`, `validate`, `reason`,
  `entails`, `consistency` and `project`. `query`, `update`, `shex`'s shape map and
  `describe --iri` are deliberately exempt: each has a command-line-text IRI
  surface with no document of its own, so `--base` is never inert there whatever
  the format rows say.

```bash
# A file input needs no flag: its own file:// retrieval IRI is the base.
purrdf convert data.ttl --to ntriples

# stdin has no retrieval IRI, so a relative IRI in it needs an explicit base.
cat data.ttl | purrdf convert - --from turtle --to ntriples \
  --base http://example.org/dir/

# Re-root a document: parse under one base, write another and relativize.
purrdf convert data.ttl --to turtle --base http://example.org/v2/
```

`--base` is validated where it is typed. A value that is not an absolute IRI is
a usage error, and a path-shaped value gets a derived suggestion — `./vocab/`
is answered with the `file://` IRI it actually denotes, resolved rather than
spliced.

A shape map given to `purrdf shex` is command-line text with no document of its
own, so `--base` is the only base it can ever have. A `--schema` file, by
contrast, is an independent document and resolves against its own retrieval IRI
or its own `BASE` directive.

## Conformance

The RFC 3986 §5.4 normative resolution table is asserted directly against the
resolver in `crates/iri/tests/`. End-to-end, the W3C `rdf-tests`
`IRI-resolution-01/02/07/08` cases — the same table driven through `@base` in a
real document, plus bases with trailing slashes, file paths, empty segments and
colon-bearing segments — are vendored under
`crates/rdf/tests/corpus/w3c/{turtle,trig}/iri/` and graded on every run.

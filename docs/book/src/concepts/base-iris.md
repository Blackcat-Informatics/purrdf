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

Step 3 needs a retrieval IRI, and only one surface has one.

| Surface | Retrieval IRI? | Consequence |
| --- | --- | --- |
| `purrdf` CLI, per input **file** | yes — the file's RFC 8089 `file://` IRI | a relative IRI resolves with no flags |
| `purrdf` CLI, stdin (`-`) | no | a relative IRI is `iri-relative-no-base`; pass `--base` |
| Rust library (`parse_dataset`, …) | no | pass a base, or the document must carry one |
| WebAssembly | no | as above |
| C ABI | no | as above |
| Python | no | as above |

Every surface except the CLI's file inputs is handed **bytes**. Bytes have no
retrieval IRI, so §5.1.3 is vacuous there and §5.1.4 — the hard failure — is
the specified answer. This is deliberate rather than an omission: a base
invented from the local filesystem would differ per machine and leak local paths
into published RDF, which would break the byte determinism the whole toolkit
rests on.

The CLI derives the retrieval IRI from the **canonicalized** path, translating
Windows paths (including UNC hosts and the extended-length `\\?\` prefix) into
RFC 8089 form and percent-encoding each component. It applies it only when
nothing of higher precedence was given, and a path with no usable `file://` IRI
is a hard error naming `--base`, never a silent fall back to "no base".

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

## The diagnostic codes

These codes are stable and machine-readable. Every codec, the CLI, the C ABI,
Python and wasm report the same code for the same condition.

| Code | Condition | What to do |
| --- | --- | --- |
| `iri-relative-no-base` | a relative reference in a syntax that admits one, with no base in scope | add `@base`/`BASE`/`xml:base`/`@context.@base` to the document, or pass a base to the API (`--base` on the CLI) |
| `iri-not-absolute-by-grammar` | a relative reference in N-Triples, N-Quads, TriX or HexTuples | write the IRI in absolute form; **a base cannot help** — this syntax admits no relative reference |
| `iri-non-absolute-base` | the base *itself* has no scheme (RFC 3986 §5.1 requires an absolute base) | supply a base with a scheme, e.g. `http://example.org/dir/`. A filesystem path is a relative reference, not a base IRI — the CLI rejects one at the argument boundary and suggests the `file://` IRI you meant |

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
there is nothing to write, and the output stays valid for its grammar.

## `--base` on the command line

`purrdf convert`, `query`, `update` and the other RDF-producing subcommands take
`--base <IRI>`, and it acts on **both legs**:

* **Parsing** — it is the caller-supplied base (§5.1.2), so it outranks the
  input's retrieval IRI but not an in-document directive.
* **Serializing** — if the target syntax can write a base, it is emitted as the
  output document's base and the IRIs are relativized against it. If the target
  cannot, absolute IRIs are written.

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

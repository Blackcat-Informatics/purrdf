<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# RDF 1.2 Features

PurRDF is RDF 1.2-first: the features
[RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/) adds over RDF 1.1
are part of the core data model, carried through the IR, the codecs, SPARQL,
validation, the language bindings, and the GTS transport.

## Triple terms

RDF 1.2 lets a triple itself be a term in **object position** — the
"quoted triple" of RDF-star, written `<<( s p o )>>` in SPARQL 1.2 syntax.
In the IR a triple term is interned like any other term and gets a `TermId`,
so it composes with everything else (patterns, results, serialization).

- The star-capable codecs (Turtle, TriG, N-Triples, N-Quads, JSON-LD star)
  round-trip triple terms; see [Codecs & Determinism](codecs.md) for what
  happens on a star-incapable projection.
- SPARQL 1.2 quoted-triple syntax is parsed by `purrdf-sparql-algebra` and
  evaluated natively — the W3C SPARQL 1.2 triple-term surface passes in the
  conformance harness ([Conformance & Testing](../project/conformance.md)).
- In JavaScript, `DataFactory.quotedTriple(...)` produces the same term
  ([RDF/JS in JavaScript](../interop/rdfjs.md)).

## Reifiers and annotations

RDF 1.2 replaces old-style reification with **reifiers**: terms that name an
*occurrence* of a triple (`rdf:reifies`), so you can attach metadata to a
statement without asserting anything odd. In PurRDF, reifier bindings and
annotations live in dedicated **side-tables** on the dataset rather than being
smeared into the quad table.

Reifier bindings and annotations survive every star-capable codec round-trip;
projections into star-incapable formats drop them *loudly*, with the realized
count handed to the loss ledger (see
[Slices, Mappings & Provenance](../slices.md)). SHACL support for validating
reified statements — the draft `sh:reifierShape` / `sh:reificationRequired`
surface — is covered in [SHACL](../validation/shacl.md).

## Base-direction literals

RDF 1.2 adds `rdf:dirLangString`: a language-tagged literal that also carries
a base direction (`ltr` or `rtl`) for correct bidirectional-text handling.
These are first-class in the IR and in every binding:

```js
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");
```

Directions survive serialization round-trips through the star-capable codecs
— the [JavaScript quickstart](../getting-started/javascript.md) demonstrates
the N-Quads round-trip.

## RDF 1.2 is a complete target, not a draft excuse

PurRDF treats the RDF 1.2 / SPARQL 1.2 specifications as a complete,
implementable target. Where a feature is scoped (for example, the SHACL 1.2
reifier-shape support is a scoped Working Draft feature, not full SHACL 1.2
conformance), the scope is stated explicitly and gated by tests — never left
as a silent partial implementation. The live per-feature status is the
conformance matrix in
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).

## Where each feature shows up

| Feature | IR | Codecs | SPARQL | SHACL | RDF/JS | GTS |
| --- | --- | --- | --- | --- | --- | --- |
| Triple terms (object position) | interned term | star-capable formats | `<<( s p o )>>` | via paths/values | `quotedTriple` | mapped per spec |
| Reifiers / annotations | side-tables | star-capable formats | reifier surface | `sh:reifierShape` (draft) | — | `rdf:reifies` mapping |
| Base-direction literals | literal kind | round-trips | matched/produced | value nodes | `directionalLiteral` | carried |

The GTS mapping of triple terms and `rdf:reifies` is formalized in the
[GTS specification](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md),
which pins its RDF 1.2 substrate to the 07 April 2026 W3C Candidate
Recommendation Snapshot.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Introduction

**PurRDF** is an [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) toolkit:
primitives, codecs, SPARQL, SHACL, ShEx, entailment, and graph transport,
implemented once in Rust and carried verbatim into Python, WebAssembly/JavaScript,
and C. It is developed by Blackcat Informatics® Inc. and published under
MIT OR Apache-2.0.

> **One RDF engine. One behavior. Every language.**

## Why does PurRDF exist?

RDF tooling fragments along two axes.

**Across languages**: every ecosystem has its own parser, with its own bugs, its
own corner-case interpretations, and its own subset of the spec. Move a graph
from a Rust service to a Python pipeline to a browser and you have silently
changed what the data means three times.

**Across time**: RDF 1.2 — triple terms, reifiers, base-direction literals — is
where the standard is going, and almost no incumbent library carries it.

PurRDF exists so that a graph is **the same graph everywhere**. It is a
from-scratch, dependency-light Rust core — parser to SPARQL engine to SHACL
validator to binary transport — exposed through native bindings rather than
reimplemented per language.

## What's inside

- **RDF 1.2 primitives** — an immutable, value-interned dataset IR (`TermId`
  space, string arena, copy-on-write mutation), with triple terms in object
  position, reifier/annotation side-tables, and base-direction literals.
  See [The Interned Dataset IR](concepts/interned-dataset.md).
- **Native codecs** — first-party parsers/serializers for Turtle, TriG,
  N-Triples, N-Quads, RDF/XML, JSON-LD (star), and YAML-LD, with
  byte-deterministic output. See [Codecs & Determinism](concepts/codecs.md).
- **Canonicalization** — W3C RDFC-1.0 plus dataset diff and isomorphism.
  See [Canonicalization & Diff](concepts/canonicalization.md).
- **SPARQL 1.1/1.2** — native parser → algebra → multiset evaluator, gated by
  the W3C conformance suites. See [SPARQL](sparql/querying.md).
- **SHACL and ShEx** — native validators for both shape languages.
  See [Validation](validation/shacl.md).
- **Entailment** — Simple/RDF/RDFS/OWL-RL materialization, an OWL-Direct
  tableau, and RIF-Core rules. See [Entailment](entailment.md).
- **GTS graph transport** — a single-file, content-addressed, append-only
  container for RDF 1.2 graphs and binary payloads.
  See [GTS Graph Transport](gts.md).
- **Slices, mappings, and provenance** — a slice catalog, an explicit RDF↔GTS
  loss ledger, SSSOM, and FnO. See [Slices, Mappings & Provenance](slices.md).

## Two design rules worth knowing on day one

**No feature flags — ever.** There are deliberately no Cargo feature flags
anywhere in the workspace, and CI enforces this. A data carrier must not have
optional behavior: optionality changes semantics per consumer, so every
consumer gets the same byte-identical semantics instead.

**PurRDF is a toolkit, not an ontology — it mints no vocabulary IRIs.** Every
vocabulary the library reads or writes is caller-supplied configuration with no
fabricated default. A feature exercised without its vocabulary hard-errors or
stays inactive; it never invents an IRI for you. (Test fixtures use
`example.org`.)

The full invariant list is in
[Design Rules & Invariants](project/design-rules.md).

## Why RDF 1.2?

RDF 1.2 (and SPARQL 1.2) add first-class statement-level metadata to the data
model: **triple terms** that can appear in object position, **reifiers** that
name occurrences of a triple, and **base-direction literals**
(`rdf:dirLangString`) for bidirectional text. PurRDF treats these as core data
model, not an extension: they flow through the IR, the codecs, SPARQL, SHACL
(a scoped SHACL 1.2 draft feature), the RDF/JS surface, and the GTS transport.
See [RDF 1.2 Features](concepts/rdf12.md).

## Where PurRDF sits

PurRDF is the library layer of a small family of linked-data projects: it is
the data backbone of the
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology) stack and the
reference home of the Rust [GTS](gts.md) engine — but it assumes nothing about
your ontology or application.

## How to read this book

- New users: start with [Getting Started](getting-started/rust.md) in your
  language, then read the [Concepts](concepts/interned-dataset.md) chapters.
- Engine users: jump to [SPARQL](sparql/querying.md),
  [Validation](validation/shacl.md), or [Entailment](entailment.md).
- Integrators: see [Interop](interop/rdflib.md) and
  [GTS Graph Transport](gts.md).
- Contributors: read the [Project](project/design-rules.md) chapters, then
  [AGENTS.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/AGENTS.md)
  in the repository.

API reference documentation lives on [docs.rs/purrdf](https://docs.rs/purrdf);
the repository is
[github.com/Blackcat-Informatics/purrdf](https://github.com/Blackcat-Informatics/purrdf).

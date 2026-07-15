<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Getting Started: Rust

The single dependency a Rust downstream needs is the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate. It re-exports the RDF 1.2
implementation surface at its root and carries every other published crate
under a stable module (`purrdf::sparql`, `purrdf::shapes`, `purrdf::shex`,
`purrdf::gts`, `purrdf::entail`, `purrdf::validate`, `purrdf::slice`,
`purrdf::iri`, `purrdf::xsd`, `purrdf::events`) — anything a consumer
legitimately imports is reachable from `purrdf` alone, never by reaching into
a sub-crate.

```sh
cargo add purrdf
```

The MSRV is Rust **1.96** (stable toolchain only; the workspace is
nightly-free by policy).

## Build, freeze, serialize, parse

```rust,ignore
use purrdf::{parse_dataset, serialize_dataset, RdfDatasetBuilder, RdfLiteral, SerializeGraph};

// Build a dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let alice = b.intern_iri("https://example.org/alice");
let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
let bob = b.intern_iri("https://example.org/bob");
let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
let hi = b.intern_literal(RdfLiteral::simple("Alice"));
b.push_quad(alice, knows, bob, None);
b.push_quad(alice, name, hi, None);
let ds = b.freeze().expect("freeze");

// Serialize to any native codec and parse back, losslessly.
let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
assert_eq!(back.quad_count(), 2);
```

The builder→freeze split is the heart of the API: you intern terms and push
quads on a mutable `RdfDatasetBuilder`, then `freeze()` into an immutable,
indexed `RdfDataset` that every engine (SPARQL, SHACL, ShEx, entailment)
evaluates over. See [The Interned Dataset IR](../concepts/interned-dataset.md).

## Parsing text directly

```rust,ignore
let turtle = r#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;
let dataset = purrdf::parse_dataset(turtle.as_bytes(), "text/turtle", None)
    .expect("valid Turtle");
assert_eq!(dataset.quad_count(), 1);
```

Malformed input is a typed `RdfDiagnostic` with a source location where the
codec can provide one — never a silent partial parse.

## Reaching the other engines

Every engine hangs off the same facade. For example, the zero-dependency IRI
leaf and the ShEx schema layer:

```rust,ignore
let iri = purrdf::iri::parse("https://example.org/cat").expect("valid IRI");
assert_eq!(iri.as_str(), "https://example.org/cat");

let schema = purrdf::shex::parse_shexc(
    "PREFIX ex: <https://example.org/>\nex:Cat { ex:says . }",
    None,
).expect("valid ShExC");
```

## When to depend on a sub-crate instead

Most applications should stop at `purrdf`. The sub-crates
(`purrdf-core`, `purrdf-rdf`, `purrdf-columnar`, `purrdf-sparql-eval`, `purrdf-shapes`,
`purrdf-shex`, `purrdf-gts`, `purrdf-entail`, `purrdf-validate`,
`purrdf-slice`, `purrdf-iri`, `purrdf-xsd`, `purrdf-events`) exist for
consumers that want exactly one engine — for example, a tool that only needs
IRI parsing can depend on the zero-dependency `purrdf-iri` alone. The crate
map is in the
[repository README](https://github.com/Blackcat-Informatics/purrdf#crate-map).

Every release crate builds cleanly for `wasm32-unknown-unknown`, so the same
Rust code paths work in native and wasm hosts.

## Next steps

- [The Interned Dataset IR](../concepts/interned-dataset.md) — how the IR works
  and why it is fast.
- [SPARQL: Querying](../sparql/querying.md) — running queries over a frozen
  dataset.
- [docs.rs/purrdf](https://docs.rs/purrdf) — the full API reference.

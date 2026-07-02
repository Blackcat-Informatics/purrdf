<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-shex

PurRDF's native **ShEx 2.1** engine: the schema layer and the shape-map
validator.

A pure-Rust, wasm-clean leaf crate implementing the
[Shape Expressions Language 2.1](https://shex.io/shex-semantics/):

- **ShExC** (compact syntax, spec §6): hand-rolled lexer +
  recursive-descent parser covering the full grammar — directives,
  `start`, the `AND`/`OR`/`NOT` shape algebra, node constraints and the
  facet table, value sets with stems/ranges/exclusions, triple
  expressions with all cardinality forms, `$`/`&` labels and inclusions,
  `^` inverse, annotations and `%…{ … %}` semantic actions, with
  relative-IRI resolution against `BASE` via `purrdf-iri`.
- **ShExJ** (JSON wire format, spec Appendix A): strict, round-tripping
  serde support matching the shexTest ground truth.
- **Structural checks** (spec §5.7): dangling references, label
  collisions, reference-only cycles, and the negation-stratification
  requirement (hand-rolled iterative Tarjan SCC).
- **`validate`** (spec §5.2–§5.5): fixed shape-map validation over the
  frozen `purrdf-core` dataset IR in interned `TermId` space — node
  constraints (node kind, datatype with lexical-validity checking,
  string/numeric facets, value sets with stems and exclusions),
  `EXTRA`/`CLOSED` triple-expression matching with `EachOf`/`OneOf`
  partitioning and group cardinalities, inverse constraints, typing-based
  recursion, and an `EXTERNAL` resolver hook.

Gated against the vendored [shexTest](https://github.com/shexSpec/shexTest)
v2.1.0 conformance suite (`vectors/shexTest`): `negativeSyntax/`,
`negativeStructure/`, the `schemas/` ShExC/ShExJ pairs, ShExJ round-trips,
and the full `validation/` manifest (every attempted entry passes; only the
`Import` and `SemanticAction` trait families are skipped).

```rust
use purrdf_shex::{check_structure, parse_shexc, to_shexj};

let schema = parse_shexc(
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:S { ex:p xsd:integer? }",
    None,
)?;
check_structure(&schema).expect("well-formed");
let json = to_shexj(&schema);
```

Hard-fail discipline: every malformed schema is a typed `ShexError`;
no lenient mode, no panics on any input.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# ShEx

[`purrdf-shex`](https://docs.rs/purrdf-shex) (re-exported as `purrdf::shex`)
is PurRDF's native
[Shape Expressions Language 2.1](https://shex.io/shex-semantics/) engine: the
schema layer and the shape-map validator, pure Rust and wasm-clean.

## Schemas: ShExC and ShExJ

- **ShExC** (the compact syntax, spec §6) — a hand-rolled lexer and
  recursive-descent parser covering the full grammar: directives, `start`,
  the `AND`/`OR`/`NOT` shape algebra, node constraints and the facet table,
  value sets with stems/ranges/exclusions, triple expressions with all
  cardinality forms, `$`/`&` labels and inclusions, `^` inverse, annotations
  and `%…{ … %}` semantic actions, with relative-IRI resolution against
  `BASE` via `purrdf-iri`.
- **ShExJ** (the JSON wire format, spec Appendix A) — strict, round-tripping
  serde support matching the shexTest ground truth.
- **Structural checks** (spec §5.7) — dangling references, label collisions,
  reference-only cycles, and the negation-stratification requirement.

```rust,ignore
use purrdf::shex::{check_structure, parse_shexc, to_shexj};

let schema = parse_shexc(
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:S { ex:p xsd:integer? }",
    None,
)?;
check_structure(&schema).expect("well-formed");
let json = to_shexj(&schema);
```

Hard-fail discipline: every malformed schema is a typed `ShexError` — no
lenient mode, no panics on any input.

## Validation with shape maps

Validation (spec §5.2–§5.5) is **fixed shape-map** validation over the frozen
`purrdf-core` dataset IR, in interned `TermId` space: you supply
`(node, shape)` associations and the validator decides conformance for each.
It covers node constraints (node kind, datatype with lexical-validity
checking, string/numeric facets, value sets with stems and exclusions),
`EXTRA`/`CLOSED` triple-expression matching with `EachOf`/`OneOf`
partitioning and group cardinalities, inverse constraints, typing-based
recursion, and an `EXTERNAL` resolver hook.

From Python:

```python
from purrdf_native import shex

result = shex.validate(my_schema_shexc, my_data_ttl,
                       [("https://example.org/alice", "https://example.org/PersonShape")])
print(result["conforms"])
```

## Conformance

The engine is gated against the vendored official
[shexTest](https://github.com/shexSpec/shexTest) suite, pinned at tag
`v2.1.0` (`vectors/shexTest/`): the full `validation/` manifest, the
`schemas/` ShExC/ShExJ pairs and round-trips, and the negative syntax and
negative structure suites. At the time of writing the validation manifest
passes **1,105/1,105** attempted cases with zero expected-failures and an
empty trait-skip list (imports and semantic actions included); the live
scoreboard is
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md).

Reported conformance is logic-level (pass/fail parity), per suite convention;
result-structure conformance is upstream-experimental.

## SHACL or ShEx?

PurRDF implements both natively over the same IR, so the choice is yours, not
the toolkit's: SHACL suits constraint reporting (violations with severities,
[SARIF output](shacl.md#sarif-output)) and SHACL-SPARQL escape hatches; ShEx
suits schema-like conformance decisions over explicit shape maps.

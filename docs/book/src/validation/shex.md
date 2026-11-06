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
- **ShExJ** (the JSON wire format, spec Appendix A) — a strict, round-tripping
  reader and writer matching the shexTest ground truth.
- **Structural checks** (spec §5.7) — dangling references, label collisions,
  reference-only cycles, and the negation-stratification requirement.

```rust
use purrdf::shex::{check_structure, parse_shexc, parse_shexj, to_shexj};

let schema = parse_shexc(
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:S { ex:p xsd:integer? }",
    None,
).expect("well-formed");
check_structure(&schema).expect("well-formed");
let json = to_shexj(&schema);
let restored = parse_shexj(&json, None).expect("well-formed");
check_structure(&restored).expect("well-formed");
assert_eq!(restored, schema);
```

Schema parsing reports malformed syntax through a typed `ShexError`;
`check_structure` reports a list of `StructureError` violations. There is no
lenient mode that accepts a malformed schema.

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
from purrdf import shex

results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")])
print(results[0]["conformant"])
```

## Dated XPath regex laws

String-facet `PATTERN`s run on the compatibility regular expressions unless the
caller selects a dated native XPath law by name: `xpath-2.0-2010-12-14` (XPath
and XQuery Functions and Operators 2.0, Second Edition) or
`xpath-3.1-2017-03-21` (Functions and Operators 3.1). Both admit
backreferences; only 3.1 admits non-capturing groups and the `q` flag.

| Host | Spelling |
|---|---|
| Rust | `validate_with_xpath` and `validate_shape_map_with_xpath`, with `Profile` and `Limits` from `purrdf_core::xsd_regex::xpath`; an `XPathValidator::new(&schema)` serves repeated validations of a fixed map |
| CLI | `--xpath-regex <NAME>` on `shex` |
| Python | `xpath_regex=` on `shex.validate` |

```python
results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")],
                        xpath_regex="xpath-3.1-2017-03-21")
```

Names match exactly, so `xpath-3.1` is refused with a list of the accepted
names. With no selection, `validate`, `validate_with` and `validate_shape_map`
keep the compatibility pattern behaviour. ShExJ hands the pattern and flag text
over as written. ShExC keeps its own concrete-syntax escape grammar, which has
no `\1`, so a backreference is written in ShExJ.

A pattern or flag the selected law does not define fails its facet, as any
invalid pattern does. A selected law also runs under finite resource bounds,
64 KiB of pattern source among them. Exceeding one aborts the whole shape map
with the resource's
[`xpath-*` code](../project/diagnostic-codes.md#xpath---dated-native-xpath-regular-expressions-purrdf-core).
Negation, alternatives and previously decided entries cannot turn that refusal
into a nonconformant entry or a partial result map.

Every validation entry point is fallible. `ValidationError::ShapeMap` retains
shape-map admission errors, `Regex` retains the native XPath cause, and
`MatcherWork { attempted, maximum }` means assignment search could not finish.
No verdict or partial result map is returned on refusal, including under
`NOT`, `AND` or `OR`.

Rust callers select `ValidationOptions::matcher_limits` with
`MatcherLimits::new(maximum_states)`. Python uses `matcher_maximum_states=`;
the CLI uses `--matcher-maximum-states COUNT`. The default admits 200,000 states
per ambiguous neighbourhood. Each entered assignment prefix costs one state,
including its terminal cardinality check; rollback and resuming a candidate
cost nothing. Unique predicate assignments require no search allowance.
Zero refuses the first ambiguous state. A refusal reports the attempted state
as maximum plus one, represented exactly even at the largest `u64` limit.
A later validation may admit more work; refused operations are never memoized
as conformance results. Python raises `ValueError` with message identity
`shex-matcher-work` and unsigned `attempted` and `maximum` presentation parameters.

`ValidationOptions` is non-exhaustive. Construct it with `Default` and select
policy with `with_matcher_limits`, `with_external_resolver`, `with_sem_acts`
or `with_extern_start_acts`; its existing fields can also be updated individually.
For example, `ValidationOptions::default().with_matcher_limits(MatcherLimits::new(1_000_000))`
admits a larger assignment search without selecting a different pattern law.

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

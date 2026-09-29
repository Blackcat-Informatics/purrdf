<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-jsonschema

Native **JSON Schema** validation for PurRDF, drafts **2020-12**, **2019-09**
and **07**: every schema resource is evaluated in the dialect its `$schema`
names, with every vocabulary, dynamic and recursive references, unevaluated
keywords, and the standard output formats.

It depends on `serde_json`, `regex`, `purrdf-iri` and `purrdf-hash` only, forbids `unsafe`,
and builds for `wasm32-unknown-unknown` like every other release crate in the
workspace, so a schema PurRDF emits from SHACL can be checked in the browser
by the same code that checks it natively. Nothing is fetched.

## Meta-schemas are supplied by the caller

The crate compiles in no meta-schema document. Every compiled schema is
checked against its meta-schema, so the application registers the published
meta-schemas of the dialects it uses — once, as a shared `Metaschemas` set
whose parsed documents and compiled validators every registry started from it
reuses — or registers a custom meta-schema with `Registry::add_resource`. A
meta-schema that is needed and not registered is
`SchemaError::MissingMetaschema`, naming its URI.

```rust
use purrdf_jsonschema::{Metaschemas, OutputFormat, Registry, SchemaError};
use serde_json::{Value, json};

fn check(draft_2020_12: Vec<(&str, Value)>) -> Result<(), SchemaError> {
    // The nine published draft 2020-12 meta-schema documents, `(URI, document)`.
    let metaschemas = Metaschemas::new(draft_2020_12)?;

    let mut registry = Registry::with_metaschemas(&metaschemas);
    registry.add_resource(
        "https://example.org/person.json",
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {"name": {"type": "string"}},
            "required": ["name"],
            "unevaluatedProperties": false
        }),
    )?;
    let schema = registry.compile("https://example.org/person.json")?;
    assert!(schema.is_valid(&json!({"name": "Ada"})).expect("evaluation"));

    let basic = schema.evaluate(&json!({"age": 36})).expect("evaluation").to_json(OutputFormat::Basic);
    assert_eq!(basic["valid"], false);
    Ok(())
}
```

`Schema::from_document(&metaschemas, uri, document)` is the one-document
shorthand.

## What is implemented

* **2020-12**: `$id`, `$schema`, `$ref`, `$defs`, `$anchor`, `$dynamicRef`
  and `$dynamicAnchor` over the dynamic scope, `$vocabulary`; the Applicator
  (`prefixItems`, `items`, `contains`, `dependentSchemas`, …), Unevaluated,
  Validation, Meta-Data, Format and Content vocabularies.
* **2019-09**: `$recursiveRef` (`"#"`, the only value the draft defines) and
  `$recursiveAnchor`, `$anchor`, `$vocabulary`, array-form `items` with
  `additionalItems`, `unevaluatedItems` and `unevaluatedProperties` in the
  Applicator vocabulary.
* **draft-07**: `definitions`, `dependencies`, array-form `items` with
  `additionalItems`, `$ref` overriding its siblings, plain-name `$id`
  fragments as location-independent identifiers, and `contentEncoding`
  (`base64`) / `contentMediaType` (`application/json`) as assertions.
* A `$ref` from one dialect into another: each side keeps its own rules.
* **Formats**: an annotation by default; `Registry::set_format_assertion`
  (or a meta-schema declaring format assertion) checks every format the
  dialect defines in full — `hostname`, `idn-hostname` and `idn-email` through
  IDNA2008 in `purrdf_iri::idna`, `ipv4`/`ipv6` through `purrdf_iri::host`.
* **Output**: `flag`, `basic` and `detailed`.

Numbers are compared and divided exactly, in decimal: `0.0075` is a multiple of
`0.0001`, and `1e308` is a multiple of `0.5`.

## Patterns

`pattern` and `patternProperties` are ECMA-262 regular expressions read with
the `u` flag. `purrdf_jsonschema::ecma` parses the grammar, uses `regex` for
regular patterns and a bounded explicit-stack matcher for lookaround,
backreferences and scoped modifiers. ECMA-262's `\d`, `\w` and `\s` sets are
spelled out, while `\p{…}` executes generated Unicode 17 ranges from the
vendored UCD. Both `is_valid` and `evaluate` return an error if matching
exhausts the caller's resource budget.

## Refusals

`SchemaError` is a refusal to *process* a schema, never a verdict on an
instance:

* a `$schema` naming draft-06 or earlier (or an unreleased successor of
  2020-12), including through a `$ref` into such a document;
* a needed meta-schema that is not registered;
* a meta-schema that requires an unknown vocabulary;
* an unresolvable `$ref`, a malformed `$id` or anchor, a duplicate resource,
  a 2019-09 `$recursiveRef` other than `"#"`;
* a schema that fails its own meta-schema;
* a malformed `pattern`, or, under the 2020-12 Format-Assertion
  vocabulary, a format no draft defines.

## Evidence

* The official JSON-Schema-Test-Suite for drafts 2020-12, 2019-09 and 07,
  vendored under `tests/suite/` — every required and optional test,
  `optional/format/` with format assertion on, plus the output-format tests —
  runs as one libtest case per suite test (`cargo test -p purrdf-jsonschema
  --test suite`, `--test suite_draft2019_09`, `--test suite_draft7`), with no
  case ignored.
* `tests/pattern_differential_vectors.txt`: JavaScript's own `RegExp`
  verdicts over the suite's pattern cases, 5,000 seeded regular pairs,
  1,000 nonregular pairs, directed Unicode 17 cases and a syntax list,
  recorded by `tests/pattern_oracle.mjs` and replayed against the compiled
  matcher.
* `tests/numeric_oracle_vectors.txt`: frozen Python `Decimal`/`Fraction`
  answers for exact comparison and `multipleOf`, including coefficients beyond
  machine integers and exponent spellings beyond machine exponents.
* `tests/boon_differential_vectors.txt`: the verdicts of the validator this
  crate replaced, over the suite and the workspace's former call sites,
  replayed with every disagreement named. See `PROVENANCE.md`.
* `benches/validate.rs`: compile and validation baselines (report-only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

The ECMA-262 `\p{…}` property tables compiled into this crate are derived from
the Unicode Character Database, so the crate's licence expression is
`(MIT OR Apache-2.0 OR MulanPSL-2.0) AND Unicode-3.0`: the Unicode-3.0 terms
([`LICENSES/Unicode-3.0.txt`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSES/Unicode-3.0.txt))
apply alongside whichever of the three you choose. See
[`LICENSING.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md).

The vendored meta-schemas under `tests/metaschemas/` and the vendored
JSON-Schema-Test-Suite under `tests/suite/` are test data under their own
licences (see `PROVENANCE.md`) and are not part of the published package.

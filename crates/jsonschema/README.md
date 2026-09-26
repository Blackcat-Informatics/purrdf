<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-jsonschema

Native **JSON Schema** validation for PurRDF — **draft 2020-12**, **draft
2019-09** and **draft-07**: every vocabulary, dynamic and recursive references,
unevaluated keywords, and the standard output formats.

It depends on `serde_json`, `regex` and `purrdf-iri` only, forbids `unsafe`,
and builds for `wasm32-unknown-unknown` like every other release crate in the
workspace, so a schema PurRDF emits from SHACL can be checked in the browser
by the same code that checks it natively. The meta-schemas of all three
dialects are vendored and registered in every `Registry`, so nothing is
fetched.

```rust
use purrdf_jsonschema::{OutputFormat, Registry, SchemaError};
use serde_json::json;

fn main() -> Result<(), SchemaError> {
    let mut registry = Registry::new();
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
    assert!(schema.is_valid(&json!({"name": "Ada"})));

    let basic = schema.evaluate(&json!({"age": 36})).to_json(OutputFormat::Basic);
    assert_eq!(basic["valid"], false);
    Ok(())
}
```

## What is implemented

Each schema resource is read in the dialect its `$schema` names (or the
registry's default, 2020-12 unless `Registry::set_default_dialect` says
otherwise), so a 2020-12 schema may `$ref` a 2019-09 or draft-07 one and the
referenced resource keeps its own rules.

* **Core**: `$id`, `$schema`, `$ref`, `$defs`, `$anchor`, `$vocabulary`,
  `$comment`; `$dynamicRef`/`$dynamicAnchor` (2020-12) and
  `$recursiveRef`/`$recursiveAnchor` (2019-09) over the dynamic scope; in
  draft-07, `$ref` overriding its siblings and `$id` plain-name fragments as
  anchors; a resource registry (`Registry::add_resource`) for schemas that
  reference each other, in which a document may arrive before its custom
  meta-schema.
* **Applicator**: `allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`,
  `dependentSchemas`, `contains`, `properties`, `patternProperties`,
  `additionalProperties`, `propertyNames`; `prefixItems` and `items` in
  2020-12, array-form `items` and `additionalItems` in 2019-09 and draft-07.
* **Unevaluated**: `unevaluatedItems` and `unevaluatedProperties`, over the
  annotations of every in-place applicator, with each dialect's annotations
  (in 2019-09 `contains` evaluates no items).
* **Validation**: `type`, `const`, `enum`, the numeric bounds and
  `multipleOf`, code-point string lengths, `pattern`, the array and object
  bounds, `uniqueItems` (JSON equality, so `1` equals `1.0`), `required`,
  `dependentRequired`, `minContains`/`maxContains`.
* **Meta-data, Content, Format-Annotation**: annotations, as the specifications
  define them. Unknown keywords are annotations too.
* **Format-Assertion**, when a meta-schema declares the 2020-12 vocabulary or
  requires the 2019-09 Format vocabulary: every format is checked in full
  except `hostname`, `idn-hostname` and `idn-email`, whose complete check
  needs the IDNA2008 tables; asserting one of those is a typed refusal, never
  a silent pass.
* **Draft-07 content**: draft-07 lets an implementation assert its content
  keywords, and this crate does for what it can check completely —
  `contentEncoding: base64` must decode (RFC 4648 §4) and a JSON
  `contentMediaType` (`application/json`, `+json`) must parse (RFC 8259).
  Other encodings and media types, and content in 2019-09 and 2020-12, are
  annotations.
* **Output**: `flag`, `basic` and `detailed`.
* The legacy `dependencies` keyword the 2019-09 and 2020-12 meta-schemas still
  describe.

Numbers are compared and divided exactly, in decimal: `0.0075` is a multiple of
`0.0001`, and `1e308` is a multiple of `0.5`.

## Patterns

`pattern` and `patternProperties` are ECMA-262 regular expressions read with
the `u` flag. `purrdf_jsonschema::ecma` parses the full grammar and translates
it to the `regex` crate, spelling out ECMA-262's `\d`, `\w` and `\s` sets code
point by code point rather than borrowing `regex`'s larger Unicode classes, and
checking every `\p{…}` name against the exact aliases ECMA-262 accepts.
Lookaround, backreferences and modifier groups are valid ECMA-262 with no
finite-automaton translation; they are refused with a typed error rather than
approximated.

## Refusals

`SchemaError` is a refusal to *process* a schema, never a verdict on an
instance:

* a `$schema` naming a dialect this crate does not implement (draft-04,
  -06, …), including through a `$ref` into such a document;
* a meta-schema that requires an unknown vocabulary;
* a `$recursiveRef` other than `"#"`, the only value 2019-09 defines;
* an unresolvable `$ref`, a malformed `$id` or anchor, a duplicate resource;
* a schema that fails its own meta-schema;
* an unrunnable `pattern`, or an asserted format that cannot be checked
  completely.

## Evidence

* The official JSON-Schema-Test-Suite, vendored under `tests/suite/`: for
  draft 2020-12, draft 2019-09 and draft-07, every required and optional test
  except `optional/format/` (format is an annotation under all three
  meta-schemas), plus the 2020-12 and 2019-09 output-format tests. Each draft
  runs as its own target, one libtest case per suite test, with nothing
  ignored and nothing expected to fail (`cargo test -p purrdf-jsonschema
  --test suite --test suite_draft2019_09 --test suite_draft7`).
* `tests/pattern_differential_vectors.txt`: JavaScript's own `RegExp`
  verdicts over the suite's pattern cases, 5,000 seeded pairs and a syntax
  list, recorded by `tests/pattern_oracle.mjs` and replayed against the
  translation.
* `tests/boon_differential_vectors.txt`: the verdicts of the validator this
  crate replaced, over the suite and the workspace's former call sites,
  replayed with every disagreement named. See `PROVENANCE.md`.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

The vendored draft 2020-12, 2019-09 and draft-07 meta-schemas under
`metaschemas/` are taken under BSD-3-Clause, and the vendored JSON-Schema-Test-Suite under `tests/suite/` is
MIT; see `PROVENANCE.md`.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of `purrdf-jsonschema`'s vendored material and frozen vectors

Everything under `src/` is first-party. What this crate carries from elsewhere
is data: two vendored trees and two files of frozen verdicts. Each is recorded
here so its origin is checkable rather than asserted.

## `tests/suite/` — the official JSON-Schema-Test-Suite

- Upstream: <https://github.com/json-schema-org/JSON-Schema-Test-Suite>,
  commit **`5b0ee1613e45fcc2bddac00e07c19cd49b00d8a8`**.
- Vendored verbatim: `tests/draft2020-12/` (every file, `optional/` included,
  except `optional/format/`), `remotes/` (every file), and
  `output-tests/draft2020-12/` (the output meta-schema and the content tests).
- Not vendored: `tests/draft2020-12/optional/format/`. Under the 2020-12
  meta-schema `format` is an annotation, which `format.json` (vendored) tests;
  the `optional/format/` files test format as an assertion.
- License: MIT, `Copyright (c) 2012 Julian Berman` — the upstream `LICENSE`
  is `tests/suite/LICENSES/MIT.txt`, and `tests/suite/REUSE.toml` declares it
  for every vendored file.
- Frozen: `scripts/check-corpus-frozen.py` holds every file to the digests in
  `scripts/conformance-frozen/jsonschema-suite.sha256`.
- Run by `tests/suite.rs`: 1,463 validation cases and 4 output cases, with
  the file, group, case and registered-remote counts pinned by its
  `suite-inventory` case. One case, `optional/cross-draft.json` group 0, asks
  for a draft 2019-09 document to be evaluated under 2019-09 rules; this crate
  implements 2020-12 only and refuses other dialects, so that case is the one
  ignored case, and `dialect-refusal/optional/cross-draft.json/0` pins the
  typed refusal it receives instead.

## `metaschemas/draft2020-12/` — the 2020-12 meta-schemas

- Upstream: <https://github.com/json-schema-org/json-schema-spec>, branch
  `2020-12`, commit **`601a66c8b0f25246bf0e1fb488c5b5f030a79b72`**: `schema.json`
  and `meta/{core,applicator,unevaluated,validation,meta-data,
  format-annotation,format-assertion,content}.json`, verbatim. These are the
  documents published at `https://json-schema.org/draft/2020-12/…`, compiled
  into the crate with `include_str!` and registered in every `Registry`.
- License: the upstream repository offers its material under the AFL or the
  BSD license; it is taken here under BSD-3-Clause
  (`metaschemas/LICENSES/BSD-3-Clause.txt`, declared by
  `metaschemas/REUSE.toml`).
- Frozen: `scripts/check-corpus-frozen.py`, manifest
  `scripts/conformance-frozen/jsonschema-metaschemas.sha256`.

## `tests/boon_differential_vectors.txt` — the replaced validator's verdicts

Before this crate existed the workspace validated JSON Schema with `boon`
0.6.1 in four places: the JSON-LD serializer-options schema test in
`purrdf-rdf`, the emitted-schema negation tests in `purrdf-shapes`, and the
TypeScript and GraphQL oracle fixtures in `purrdf-shapes/examples`. While
`boon` was still a dependency, a recorder ran it over:

- every case of the vendored suite (1,463 records), with the suite's remotes
  registered at `http://localhost:1234/`; and
- every distinct (schema, instance) pair those four call sites evaluated,
  captured at the call sites during their own test and example runs (136
  records).

From `boon` this took answers only, not code: each record is a source label,
a retrieval URI, the schema, the instance and `boon`'s verdict. The recorder
was deleted with `boon`; the vectors stay, self-hashed in the workspace's
frozen-vector format, and `tests/boon_differential.rs` replays every record
against this crate.

Where `boon` contradicts the official suite, the suite is the authority and
this crate gives the suite's answer. There are two such records:

| Record | `boon` | Suite | Why |
|---|---|---|---|
| `suite:optional/float-overflow.json/0/0` | invalid | valid | `1e308` is an integer and so a multiple of `0.5`; dividing in binary floating point overflows. This crate divides in decimal. |
| `suite:optional/format-assertion.json/0/1` | valid | invalid | A meta-schema declaring the Format-Assertion vocabulary (even as `false`) makes `format` an assertion for an implementation that knows the vocabulary. |

One further record differs by design rather than by error:
`suite:optional/cross-draft.json/0/0`, where `boon` evaluated the referenced
draft 2019-09 document (and agreed with the suite) and this crate refuses the
dialect with `SchemaError::UnsupportedDialect`. The replay requires exactly
that refusal. Every one of the 136 call-site records agrees with `boon`.

## `tests/pattern_differential_vectors.txt` — JavaScript's `RegExp` verdicts

Written by `tests/pattern_oracle.mjs` with Node, from JavaScript's own
`RegExp` under the `u` flag: every `pattern` and `patternProperties` key of the
vendored suite paired with every string and key of its test instances (151
pairs), a seeded corpus of 5,000 generated pairs, and a fixed list of 71
patterns whose only question is whether they are ECMA-262 at all. From the
JavaScript engine this took answers only, not code. `make
jsonschema-pattern-oracle` (`node crates/jsonschema/tests/pattern_oracle.mjs`)
re-asks Node and fails if any answer moved; `--write` rewrites the file. `tests/pattern_differential.rs`
replays every record against the translation.

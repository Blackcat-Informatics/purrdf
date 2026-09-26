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
- Vendored verbatim: `tests/draft2020-12/`, `tests/draft2019-09/` and
  `tests/draft7/` (every file, `optional/` included, except
  `optional/format/`), `remotes/` (every file), and
  `output-tests/draft2020-12/` and `output-tests/draft2019-09/` (each
  draft's output meta-schema and content tests; draft-07 has none).
- Not vendored: each draft's `optional/format/`. Under the 2020-12, 2019-09
  and draft-07 meta-schemas `format` is an annotation, which each draft's
  `format.json` (vendored) tests; the `optional/format/` files test format as
  an assertion.
- License: MIT, `Copyright (c) 2012 Julian Berman` — the upstream `LICENSE`
  is `tests/suite/LICENSES/MIT.txt`, and `tests/suite/REUSE.toml` declares it
  for every vendored file.
- Frozen: `scripts/check-corpus-frozen.py` holds every file to the digests in
  `scripts/conformance-frozen/jsonschema-suite.sha256`.
- Run by `tests/suite.rs` (draft 2020-12: 1,463 validation cases and 4
  output cases), `tests/suite_draft2019_09.rs` (draft 2019-09: 1,419 and 4)
  and `tests/suite_draft7.rs` (draft-07: 1,047), through the shared
  `tests/suite_runner/`, with each draft's file, group, case and
  registered-remote counts pinned by its `suite-inventory` case. Nothing is
  ignored and nothing is expected to fail. The `optional/cross-draft.json`
  cases evaluate a `$ref` into another draft's document under that draft's
  rules: 2020-12 into 2019-09, 2019-09 into 2020-12 and draft-07, draft-07
  into 2019-09.

## `metaschemas/` — the three dialects' meta-schemas

- Upstream: <https://github.com/json-schema-org/json-schema-spec>, verbatim,
  one branch per dialect:
  - `draft2020-12/`: branch `2020-12`, commit
    **`601a66c8b0f25246bf0e1fb488c5b5f030a79b72`** — `schema.json` and
    `meta/{core,applicator,unevaluated,validation,meta-data,
    format-annotation,format-assertion,content}.json`;
  - `draft2019-09/`: branch `2019-09`, commit
    **`c8eb3d320f60eca7cfb18da25337a426ceb40eaa`** — `schema.json` and
    `meta/{core,applicator,validation,meta-data,format,content}.json` (the
    hyper-schema documents are not vendored);
  - `draft-07/`: branch `draft-07`, commit
    **`20a3fee852db88519dcb3bb329b7a872ebad8953`** — `schema.json` (the
    hyper-schema documents are not vendored).

  These are the documents published at
  `https://json-schema.org/draft/2020-12/…`,
  `https://json-schema.org/draft/2019-09/…` and
  `http://json-schema.org/draft-07/schema`, compiled into the crate with
  `include_str!` and registered in every `Registry`.
- License: the upstream repository offers its material under the AFL or the
  BSD license, on every one of those branches; it is taken here under
  BSD-3-Clause (`metaschemas/LICENSES/BSD-3-Clause.txt`, declared by
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

Every other record agrees with `boon`, including
`suite:optional/cross-draft.json/0/0`, where both evaluate the referenced
draft 2019-09 document under 2019-09 rules, and every one of the 136
call-site records.

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

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
  `tests/draft7/` (every file, `optional/` and `optional/format/` included),
  `remotes/` (every file), and `output-tests/draft2020-12/` and
  `output-tests/draft2019-09/` (the output meta-schemas and the content
  tests).
- License: MIT, `Copyright (c) 2012 Julian Berman` — the upstream `LICENSE`
  is `tests/suite/LICENSES/MIT.txt`, and `tests/suite/REUSE.toml` declares it
  for every vendored file.
- Test data only: excluded from the published package.
- Frozen: `scripts/check-corpus-frozen.py` holds every file to the digests in
  `scripts/conformance-frozen/jsonschema-suite.sha256`.
- Run by `tests/suite.rs` (2020-12), `tests/suite_draft2019_09.rs` and
  `tests/suite_draft7.rs` through `tests/suite_runner/`: every validation case
  of each draft, `optional/format/` with format assertion on, and the output
  cases, with the file, group, case and registered-remote counts pinned by
  each target's `suite-inventory` case. No case is ignored.

## `tests/metaschemas/` — the draft-07, 2019-09 and 2020-12 meta-schemas

- Upstream: <https://github.com/json-schema-org/json-schema-spec>.
  - `draft2020-12/`: branch `2020-12`, commit
    **`601a66c8b0f25246bf0e1fb488c5b5f030a79b72`**: `schema.json` and
    `meta/{core,applicator,unevaluated,validation,meta-data,
    format-annotation,format-assertion,content}.json`, verbatim.
  - `draft2019-09/` (`schema.json` and
    `meta/{core,applicator,validation,meta-data,format,content}.json`) and
    `draft-07/schema.json`: the `2019-09` and `draft-07` branches, verbatim.
    Their upstream commits were not recorded when they were vendored and are
    not recorded here. What is pinned is their content: the digests in
    `scripts/conformance-frozen/jsonschema-metaschemas.sha256`.
  - These are the documents published at
    `https://json-schema.org/draft/2020-12/…`,
    `https://json-schema.org/draft/2019-09/…` and
    `http://json-schema.org/draft-07/schema`.
- License: at the vendored commits the upstream repository carried no
  `LICENSE` file; its README declared only that "the source material in this
  repository is licensed under the AFL or BSD license". The project later
  specified those licences in a root `LICENSE` file, added in commit
  `51326f809003` (2022-07-12): BSD-3-Clause, `Copyright (c) 2022 JSON Schema
  Specification Authors`, followed by the Academic Free License 3.0. PurRDF
  takes the meta-schemas under BSD-3-Clause:
  `tests/metaschemas/LICENSES/BSD-3-Clause.txt` is that file's BSD section
  verbatim, copyright line included, and `tests/metaschemas/REUSE.toml`
  declares it with the holder "JSON Schema Specification Authors".
- Test data only. The crate compiles no meta-schema in: callers register
  the ones they need (`Metaschemas`, `Registry::with_metaschemas`). The
  workspace's tests and examples take these copies through
  `purrdf_testkit::jsonschema_metaschemas`; the directory is excluded from
  the published package.
- Frozen: `scripts/check-corpus-frozen.py`, manifest
  `scripts/conformance-frozen/jsonschema-metaschemas.sha256`.

## `tests/boon_differential_vectors.txt` — the replaced validator's verdicts

Before this crate existed the workspace validated JSON Schema with `boon`
0.6.1 in four places: the JSON-LD serializer-options schema test in
`purrdf-rdf`, the emitted-schema negation tests in `purrdf-shapes`, and the
TypeScript and GraphQL oracle fixtures in `purrdf-shapes/examples`. While
`boon` was still a dependency, a recorder ran it over:

- every case of the vendored draft 2020-12 suite outside `optional/format/`
  (1,463 records), with the suite's remotes registered at
  `http://localhost:1234/`; and
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

Every other record, the 136 call-site records included, agrees with `boon`.
The replay registers the vendored meta-schemas, which this crate does not
carry.

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

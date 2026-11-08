<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# Provenance of `purrdf-jsonschema`'s vendored material and frozen vectors

The implementation under `src/` is first-party; generated Unicode tables
derive from Unicode data. This crate also carries two vendored test trees and
two files of frozen verdicts. Their origins are recorded below.

## Unicode property tables

`src/ecma/unicode_ranges.rs` and `src/ecma/property_tables.rs` are generated
by the workspace's one Unicode table generator,
`cargo run -p purrdf-lex --example gen_unicode_tables -- ecma-ranges` and
`-- ecma-properties`, from the Unicode 17.0.0 data in
`../iri/unicode/17.0.0/`. The input includes
[`ScriptExtensions.txt`](https://www.unicode.org/Public/17.0.0/ucd/ScriptExtensions.txt),
whose SHA-256 is
`ec2107e58825a1586acee8e0911ce18260394ac8b87e535ca325f1ccbeb06bc6`.
The full input tree is pinned in
`scripts/conformance-frozen/iri-unicode.sha256` and verified by
`scripts/check-corpus-frozen.py`. Generated Unicode data retains the
`Unicode-3.0` licence alongside the project licence; see `LICENSING.md`.

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

## Shared meta-schemas

The draft-07, 2019-09 and 2020-12 meta-schema bytes, upstream identities and
BSD-3-Clause notices live in the published shared test support:
[`purrdf-testkit` provenance](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/testkit/PROVENANCE.md). Tests and examples
register those documents through `purrdf_testkit::jsonschema_metaschemas`.
This engine compiles no meta-schema in; callers select and register their own.

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

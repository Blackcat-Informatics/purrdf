<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Shared test data provenance

## `metaschemas/` — the draft-07, 2019-09 and 2020-12 meta-schemas

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
    `scripts/conformance-frozen/jsonschema-metaschemas.sha256` in the repository.
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
  `metaschemas/LICENSES/BSD-3-Clause.txt` is that file's BSD section
  verbatim, copyright line included, and `metaschemas/REUSE.toml`
  declares it with the holder "JSON Schema Specification Authors".
- These exact documents are embedded by `jsonschema_metaschemas` and included
  in this package. The JSON Schema engine itself compiles no meta-schema in:
  callers register the ones they need (`Metaschemas`, `Registry::with_metaschemas`).
- Frozen in the repository by `scripts/check-corpus-frozen.py`, with the receipt
  `scripts/conformance-frozen/jsonschema-metaschemas.sha256`.

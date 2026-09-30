<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored JSONTestSuite parsing corpus

Frozen copy of the upstream JSONTestSuite `test_parsing/` documents, vendored for
`crates/lex/tests/json_test_suite.rs`. **Do not hand-edit**: the freeze is
enforced by `scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-JSONTestSuite.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-jsontestsuite.py`.

## Source

- Upstream: <https://github.com/nst/JSONTestSuite>
- Pinned commit: `1ef36fa01286573e846ac449e8683f8833c5b26a`
- Pinned `test_parsing` tree: `b936f9acdd24b9f5fefe68b90b9beab2c681137a`
- License: **MIT** (the upstream `LICENSE`, vendored verbatim,
  SHA-256 `8bd0e0578be788c617ea01d18b2a8146e3746ae50bddadc65a5f9d3aad08ad49`)
- Retrieval: `git fetch --depth 1 https://github.com/nst/JSONTestSuite.git 1ef36fa01286573e846ac449e8683f8833c5b26a`, the
  commit and tree ids checked against the pins in the script.

## Contents

`test_parsing/`: 318 documents named for the result RFC 8259
requires of a parser. 95 `y_*` must be accepted, 188 `n_*`
must be refused, and 35 `i_*` are left to the implementation, so the
suite's own README calls them implementation-defined.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Vendored YAML test suite

Frozen copy of the official YAML test suite, vendored for
`crates/lex/tests/yaml_test_suite.rs`, which grades the `purrdf_lex::yaml`
reader against it. **Do not hand-edit**: the freeze is enforced by
`scripts/check-corpus-frozen.py` against
`scripts/conformance-frozen/vectors-yaml-test-suite.sha256`, and the tree is
regenerated only by `python3 scripts/vendor-yaml-test-suite.py`.

## Source

- Upstream: <https://github.com/yaml/yaml-test-suite>
- Release: `data-2022-01-17` (the `data` branch layout), pinned commit `5f49729577242103ae23838ac2ad4d9145aec126`
- License: **MIT** (`LICENSE`, the upstream `License` file of `main`
  commit `da267a5c4782e7361e82889e76c0dc7df0e1e870`, vendored verbatim; SHA-256 `c9562189164244554a69ab3f29d2d93ed9492c165723aaaa5fffc932cdbbfc85`)
- Retrieval: `https://codeload.github.com/yaml/yaml-test-suite/tar.gz/5f49729577242103ae23838ac2ad4d9145aec126`; every regular
  file is extracted verbatim (symlinks are skipped) and the set is checked
  against the pinned tree digest `0897d6cb42ba393e6365ec88e1d29b699f76e25a8aa0179c5769ad3710d2ccf4` over 1887 files.

## Contents

One directory per case (`<ID>/`, or `<ID>/<NN>/` for an ID that holds several
inputs): `===` the case name, `in.yaml` the input, `in.json` the expected JSON
value(s) (absent for error cases and for the few cases with no JSON form),
`error` (present when the input is invalid YAML), `test.event`, and for some
cases `out.yaml` / `emit.yaml`.

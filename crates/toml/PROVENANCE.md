<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# TOML grammar and frozen conformance data

The parser is independently implemented from the
[TOML 1.1 specification](https://toml.io/en/v1.1.0). Unicode escapes and calendar
validation call the workspace's existing lexical and RFC 3339 homes. The RDF
carrier follows the shipped JSON and Markdown profile/cover contracts and calls
the shared native RDF builder, content identity and reconstruction law.

`tests/suite` contains the exact TOML 1.1 selection from
[toml-test](https://github.com/toml-lang/toml-test/tree/ff49d109861c1ad25af53f687f2aef19ab650600/tests),
commit `ff49d109861c1ad25af53f687f2aef19ab650600`, acquired October 24, 2026.
The upstream selection lists 712 TOML inputs and 218 tagged JSON answers.
The native harness validates both acceptance/refusal and typed semantic values.
No upstream parser implementation is incorporated. Payload bytes are frozen;
the upstream MIT license is retained at `tests/suite/LICENSES/MIT.txt` and
`REUSE.toml` supplies the source-data annotations.

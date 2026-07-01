<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-sparql-conformance

`purrdf-sparql-conformance` is the native W3C SPARQL 1.1 conformance harness. It
loads manifest files, runs each case through `purrdf-sparql-eval`, and compares
the result against SPARQL Results or canonical graph goldens.

## Source Map

| Module | Responsibility |
| --- | --- |
| `manifest` / `paths` | Discover and parse test manifests. |
| `run` | Execute a modeled case against the native evaluator. |
| `compare` | Compare SELECT/ASK/CONSTRUCT outputs. |
| `service` | Resolve federated SERVICE cases through in-memory data sources. |
| `xfail` | Record expected failures as hard-accounted registry entries. |

## Checks

```bash
make rust-test
make rust-docs
```

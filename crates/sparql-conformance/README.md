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
| `run` | Execute a modeled case against the native evaluator, and register the harness's property-function relations (their tuples are read out of `suite/purrdf-property-functions/relations.ttl`). |
| `mode_restricted` | A harness relation declaring only the `bf` access pattern, so the suite reaches mode restriction, subsumption, and the feasibility reorder. |
| `compare` | Compare SELECT/ASK/CONSTRUCT outputs — and grade the diagnostic of a case whose `mf:result` is a `.err` file, which expects the run to be refused. |
| `service` | Resolve federated SERVICE cases through in-memory data sources. |
| `xfail` | Record expected failures as hard-accounted registry entries. |

## Checks

```bash
make rust-test
make rust-docs
```

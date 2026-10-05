<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

# purrdf-sparql-conformance

`purrdf-sparql-conformance` is the native W3C SPARQL 1.0/1.1/1.2 conformance harness. It
loads manifest files, runs each case through `purrdf-sparql-eval`, and compares
the result against SPARQL Results, RDF result sets or canonical graph goldens.

The frozen SPARQL 1.0 data-r2 import contains 482 standard cases across 29
group manifests and one separately supplied sort extension. The native inventory
proves that discovering those 30 leaves executes every one of the 483 cases
exactly once; the root include manifest is an inventory check, not another run.
[Provenance](suite/w3c-sparql10/PROVENANCE.md) records the upstream commit,
verbatim payload policy, license and root filename mapping.

Literal comparison remains exact, including numeric representations and source
bindings. RDF 1.2/SPARQL 1.2 expectation differences have cited entries in `xfail`;
native independent value/type/source oracles grade their current answers, and a
passing ledgered fixture fails as XPASS. Only manifests explicitly declaring
`mf:LaxCardinality` admit the reduced multiplicity of REDUCED results.

## Source Map

| Module | Responsibility |
| --- | --- |
| `manifest` / `paths` | Discover and parse test manifests. |
| `run` | Execute a modeled case against the native evaluator, and register the harness's property-function relations (their tuples are read out of `suite/purrdf-property-functions/relations.ttl`). |
| `mode_restricted` | A harness relation declaring only the `bf` access pattern, so the suite reaches mode restriction, subsumption, and the feasibility reorder. |
| `compare` | Compare SELECT/ASK/CONSTRUCT outputs — and grade the diagnostic of a case whose `mf:result` is a `.err` file, which expects the run to be refused. |
| `rs_resultset` | Read the older corpus's Turtle and RDF/XML SELECT/ASK result sets, including explicit row indices and entirely unbound rows. |
| `service` | Resolve federated SERVICE cases through in-memory data sources. |
| `xfail` | Record expected failures as hard-accounted registry entries. |

## Checks

```bash
make rust-test
make rust-docs
```

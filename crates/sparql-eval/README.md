<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-sparql-eval

`purrdf-sparql-eval` is the native RDF 1.2 SPARQL evaluator. It consumes
`purrdf-sparql-algebra`, evaluates over `purrdf-core`'s `DatasetView`, and
keeps the hot path in interned `TermId` space.

## Source Map

| Module | Responsibility |
| --- | --- |
| `engine` / `eval` | Public engine, prepared-query, and evaluation entry points. |
| `bgp`, `path`, `modifier`, `construct`, `template` | Graph-pattern, property-path, solution-modifier, and CONSTRUCT execution. |
| `expr`, `binop`, `list_fn` | FILTER/BIND expression evaluation and built-ins. |
| `scratch` / `solution` | Scratch interner, solution terms, bags, and variable schemas. |
| `remote` / `remote_http` | SERVICE source abstraction and native HTTP transport. |
| `update` | Graph update execution surface. |

## Checks

```bash
make rdf-core-hygiene
make rust-test
make rust-docs
```

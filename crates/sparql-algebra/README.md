<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-sparql-algebra

`purrdf-sparql-algebra` is the native SPARQL 1.1/1.2 front-end. It parses query
and update text into a PurRDF-owned algebra AST, including RDF 1.2 quoted triple
terms and project-specific closed extension functions.

## Source Map

| Module | Responsibility |
| --- | --- |
| `lexer` / `parser` | Tokenize and parse SPARQL text. |
| `ast` | Parsed term, triple, quad, and literal syntax nodes. |
| `algebra` | Evaluable query/update algebra consumed by `purrdf-sparql-eval`. |
| `serialize` | Query-pattern serialization helpers. |
| `error` | Typed parse and unsupported-surface failures. |

## Checks

```bash
make rdf-core-hygiene
make rust-docs
```

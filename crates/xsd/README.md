<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf-xsd

`purrdf-xsd` is the zero-dependency XSD 1.1 value-space crate for the native RDF
and SPARQL stack. It replaces the oxigraph-family `oxsdatatypes` dependency with
project-owned parsing, value equality/ordering, canonical lexical forms, and
SPARQL numeric promotion.

## Source Map

| Module | Responsibility |
| --- | --- |
| `datatype` | XSD datatype identifiers and namespace constants. |
| `value` | Lexical parsing into typed value-space representations. |
| `numeric` | Integer/decimal/float/double parsing and arithmetic helpers. |
| `temporal` | Date/time/duration value spaces and comparison helpers. |
| `binary` | Hex/base64 parsing and canonicalization. |
| `ops` | Effective boolean value and SPARQL value comparison/equality. |
| `simple` | Shared primitive parsers. |

## Checks

```bash
make rdf-core-hygiene
make rust-docs
```

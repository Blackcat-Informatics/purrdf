<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Getting Started: Python

The Python package wraps the same native Rust engine — not a reimplementation —
so parsing, serialization, SPARQL, and validation behave identically to the
Rust, JavaScript, and C surfaces.

```sh
pip install purrdf
```

## Parsing

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
```

## Validation: SHACL and ShEx

The native validation engines are exposed from the `purrdf_native` extension
module:

```python
from purrdf_native import shacl, shex

report = shacl.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])

result = shex.validate(my_schema_shexc, my_data_ttl,
                       [("https://example.org/alice", "https://example.org/PersonShape")])
print(result["conforms"])
```

SHACL result dicts keep the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`. See [SHACL](../validation/shacl.md)
and [ShEx](../validation/shex.md) for what the engines cover.

## rdflib compatibility

The package ships an rdflib compatibility layer:

```python
from purrdf.compat.rdflib import Graph
```

For a literal, zero-change `import rdflib`, there is an opt-in extra:

```sh
pip install purrdf[rdflib]
```

This pulls in the separate `purrdf-rdflib` distribution, whose top-level
`rdflib` package re-exports the compat surface, so existing third-party code
doing `import rdflib` / `from rdflib.namespace import RDF` transparently runs
on purrdf. **Caveat:** that shadow claims the `rdflib` import name and must
never be installed alongside the genuine
[`rdflib`](https://pypi.org/project/rdflib/) — the two cannot co-inhabit one
environment. It is a separate distribution (never bundled into the main
`purrdf` wheel) precisely so environments that need the real rdflib simply
omit it.

The compat layer is gated in CI against rdflib 7.6's own vendored test suite
plus a first-party differential parity suite — see
[rdflib Compatibility](../interop/rdflib.md) for details and the known,
ledgered divergences.

## GTS relational exports

The Python package also ships GTS relational exports for analytics pipelines:

```python
from purrdf import gts_to_sqlite, gts_to_duckdb, gts_to_parquet
```

These project a [GTS container](../gts.md) into SQLite, DuckDB, or Parquet
tables.

## Graph, tabular, and research-object archives

`purrdf.project(data, format=..., profile=..., config=...)` returns canonical
USTAR bytes and structured loss records. `purrdf.lift(archive, profile=...,
config=...)` reconstructs RDF for the ten bidirectional profiles. The same
strict configuration and deterministic Rust code paths are used in every host;
see [Graph, Tabular & Research-Object Projections](../concepts/projections.md) for profiles and a
complete example.

## Next steps

- [rdflib Compatibility](../interop/rdflib.md) — the drop-in story in depth.
- [Validation](../validation/shacl.md) — SHACL and ShEx from Python.
- [GTS Graph Transport](../gts.md) — the container format the exports read.
- [Graph, Tabular & Research-Object Projections](../concepts/projections.md) — LPG, CSVW, OBO,
  SKOS, and five research-object carriers.

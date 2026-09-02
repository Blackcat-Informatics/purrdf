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

The native validation engines are exposed as top-level submodules mirroring the
Rust `purrdf` umbrella crate — never through the internal `purrdf_native`
extension module directly:

```python
from purrdf import shapes, shex

report = shapes.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])

results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")])
print(results[0]["conformant"])
```

SHACL result dicts keep the stable keys `focus`, `path`, `value`, `severity`,
`component`, `source_shape`, and `message`. See [SHACL](../validation/shacl.md)
and [ShEx](../validation/shex.md) for what the engines cover.

## Entailment

`purrdf.entail` closes a dataset under a SPARQL entailment regime. It is not
`purrdf.shapes.entail`, which applies the SHACL-AF `sh:rule`s a *shapes* graph
declares; this one takes no shapes and uses the regime's own specification rule
table.

```python
import purrdf
from purrdf import entail

dataset = purrdf.RdfDataset(my_turtle, purrdf.RdfFormat.TURTLE)
closure, report = entail.materialize(dataset, "rdfs", "")
print(closure.to_nquads())
print(report)          # what fired, what did not, boundaries, budget, contract hash
```

The report is the second return value and is never optional — the same
discipline the Rust, WebAssembly, and C surfaces enforce. `entail.materialize_nt(text, regime)`
is the text-in/text-out twin for callers holding an N-Triples/N-Quads document.

Coverage is measurable rather than asserted: `entail.rules(regime)` is the rule
table the specification defines the regime by, and
`entail.implemented_rules(regime)` is the subset that fires. `"owl-direct"` and
`"rif"` return `[]` here — neither has a specification rule table of its own,
since one decides through the tableau and the other entails under the caller's
own rules — not a raised error. See [Entailment](../entailment.md) for the
full picture and the [rule inventory](../entailment-rules.md) for the per-rule
table.

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
- [Entailment](../entailment.md) — the regimes, the report, and what each fires.
- [GTS Graph Transport](../gts.md) — the container format the exports read.
- [Graph, Tabular & Research-Object Projections](../concepts/projections.md) — LPG, CSVW, OBO,
  SKOS, and five research-object carriers.

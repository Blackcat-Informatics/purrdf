<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# PurRDF for Python

<p>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/v/purrdf.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/pyversions/purrdf.svg" alt="Python versions"></a>
</p>

PurRDF is a from-scratch, dependency-light [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/)
engine — parsers and serializers, SPARQL, SHACL, ShEx, RDFC-1.0 canonicalization, and the
GTS graph-transport container — written in Rust and carried verbatim into Python, JavaScript,
and C. The `purrdf` package is the Python surface of that one engine: the same
byte-identical semantics in every language, including triple terms, reifiers, and
base-direction literals that most incumbent libraries do not carry.

## Install

```sh
pip install purrdf
```

Requires Python 3.13+. Wheels bundle the native extension; no Rust toolchain needed.

## Parse RDF

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)
```

`purrdf.parse` accepts Turtle, TriG, N-Triples, N-Quads, TriX, and HexTuples
(`purrdf.RdfFormat`); JSON-LD and RDF/XML travel through the dedicated
`purrdf.from_json_ld` / `purrdf.to_json_ld` and `purrdf.from_rdf_xml` /
`purrdf.to_rdf_xml` converters. All codecs are first-party with
byte-deterministic output.

## Validate with SHACL

The SHACL engine lives at `purrdf.shapes` (mirroring the Rust crate; `purrdf.shacl`
is a back-compat alias):

```python
from purrdf import shapes

report = shapes.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])
```

Complete SHACL Core, SHACL-SPARQL constraints/targets, and SHACL-AF `sh:rule`
entailment via `shapes.entail(...)`. Reusable parsed shapes are available as
`shapes.Shapes(shapes_ttl).validate_nt(data_nt)`.

## Validate with ShEx

```python
from purrdf import shex

results = shex.validate(
    my_schema_shexc,
    my_data_ttl,
    [("https://example.org/alice", "https://example.org/PersonShape")],
)
print(all(entry["conformant"] for entry in results))
```

The ShEx 2.1 validator passes 1,051/1,051 attempted validation tests of the official
shexTest suite (see the repo's `docs/CONFORMANCE.md`).

## rdflib compatibility layer

The package ships an rdflib-shaped API over the native engine:

```python
from purrdf.compat.rdflib import Graph, URIRef

g = Graph()
g.parse(data=my_ntriples, format="nt")
print(len(g), g.serialize(format="turtle"))
```

For a literal, zero-change `import rdflib`, install the opt-in extra:

```sh
pip install purrdf[rdflib]
```

This pulls in the separate [`purrdf-rdflib`](https://github.com/Blackcat-Informatics/purrdf/tree/main/bindings/python-rdflib-shadow)
distribution, whose top-level `rdflib` package re-exports the compat surface, so
existing third-party code doing `import rdflib` / `from rdflib.namespace import RDF`
transparently runs on purrdf. **Caveat:** that shadow claims the `rdflib` import
name and must never be installed alongside the genuine
[`rdflib`](https://pypi.org/project/rdflib/) — the two cannot co-inhabit one
environment. It is a separate distribution (never bundled into the main `purrdf`
wheel) precisely so environments that need the real rdflib simply omit it.

## GTS graph transport and relational exports

GTS is PurRDF's single-file, content-addressed, append-only container for RDF 1.2
graphs. Build one from quads and export it straight to relational stores:

```python
import purrdf

gts_bytes = purrdf.gts_from_quads(my_nquads_bytes, format=purrdf.RdfFormat.N_QUADS)

purrdf.gts_to_sqlite(gts_bytes, "graph.db")
purrdf.gts_to_duckdb(gts_bytes, "graph.duckdb")
files = purrdf.gts_to_parquet(gts_bytes, "out/")
```

The same entry points are grouped under `purrdf.gts` for discoverability.

## Learn more

- Repository: <https://github.com/Blackcat-Informatics/purrdf>
- Project site: <https://blackcatinformatics.ca/purrdf/>
- GTS specification, conformance matrix, and full docs live under
  [`docs/`](https://github.com/Blackcat-Informatics/purrdf/tree/main/docs) in the repo.

Licensed under MIT OR Apache-2.0, at your option.

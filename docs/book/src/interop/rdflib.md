<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# rdflib Compatibility

Python's [rdflib](https://rdflib.readthedocs.io/) is the incumbent RDF library
of the ecosystem, and PurRDF meets it in two tiers: an explicit compat module,
and an opt-in drop-in shadow.

## Tier 1: the explicit compat layer

The main `purrdf` wheel ships an rdflib compatibility layer backed by the
native engine:

```python
from purrdf.compat.rdflib import Graph

g = Graph()
g.parse(data="<https://example.org/a> <https://example.org/b> <https://example.org/c> .",
        format="turtle")
```

This is the recommended path for new code that wants an rdflib-shaped API on
the PurRDF engine: the import name is honest, and it coexists with a genuine
`rdflib` installation.

`Graph.query`, the SPARQL processor, `Dataset` and `ConjunctiveGraph` use
contextual initial-binding semantics compiled into the native evaluator. Query
assignments can replace an initial value while the initial mapping remains
available to enclosing operations. Named graph views and `default_union` select
the query's graph scope. The direct `purrdf.Store.query` and `Store.prepare`
interfaces retain native pre-binding rules.

## Tier 2: the `purrdf[rdflib]` shadow distribution

For a literal, zero-change `import rdflib`, install the opt-in extra:

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

## How compatibility is kept honest

The compat layer is not "best effort" — it is gated in CI as part of the
single conformance matrix
([`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)):

- **The rdflib drop-in (LSP) gate** runs rdflib 7.6's **own vendored test
  suite** against the purrdf drop-in.
- **The parity suite** runs first-party differential tests of
  `purrdf.compat` against the real rdflib 7.6.

Both use strict expected-failure ledgers: every known divergence is listed
with a per-test reason, an unexpected failure breaks the build, and a silently
fixed divergence also breaks the build until the ledger shrinks. The ledgers
state the current divergences and distinguish unsupported configured execution
from deliberate term and query admission differences.

## Performance

A report-only benchmark harness times the native-backed
`purrdf.compat.rdflib` drop-in against the real rdflib on parse, serialize,
SPARQL, and triple-pattern iteration (`make bench-python`). Methodology and a
representative (host-dependent) results table live in
[`docs/BENCHMARKS.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/BENCHMARKS.md)
— numbers vary by host, so reproduce locally rather than trusting a fixed
multiplier. See [Performance](../project/performance.md) for the philosophy.

## Related

- [Getting Started: Python](../getting-started/python.md)
- [Conformance & Testing](../project/conformance.md) — how the ledger
  discipline works.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# SPARQL: Querying

PurRDF's SPARQL stack is native and three-layered, gated by the W3C SPARQL
1.1 and 1.2 conformance suites:

1. **[`purrdf-sparql-algebra`](https://docs.rs/purrdf-sparql-algebra)** —
   parses query and update text into a PurRDF-owned, RDF 1.2-native query
   algebra (`Query`/`GraphPattern`, `Update`/`GraphUpdateOperation`).
   Parse and algebra only.
2. **[`purrdf-sparql-eval`](https://docs.rs/purrdf-sparql-eval)** — the
   multiset evaluator over the frozen IR's `DatasetView`, entirely in interned
   `TermId` space.
3. **[`purrdf-sparql-results`](https://docs.rs/purrdf-sparql-results)** — the
   results boundary ([next chapter](results.md)).

All three are re-exported under `purrdf::sparql`.

## A first query

```rust,ignore
use purrdf::{RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf::sparql::NativeSparqlEngine;

// A tiny dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat");
let says = b.intern_iri("https://example.org/says");
let meow = b.intern_literal(RdfLiteral::simple("meow"));
b.push_quad(cat, says, meow, None);
let ds = b.freeze().expect("freeze");

// Evaluate through the SparqlEngine seam; parsed plans are memoized.
let engine = NativeSparqlEngine::new();
let result = engine.query(&ds, SparqlRequest {
    query: "SELECT ?what WHERE { <https://example.org/cat> <https://example.org/says> ?what }",
    base_iri: None,
    substitutions: &[],
}).expect("evaluates");

if let SparqlResult::Solutions { rows, .. } = result {
    assert_eq!(rows.len(), 1);
}
```

The `SparqlEngine` trait itself lives in `purrdf-core`, so hosts can swap
engines behind one seam; `NativeSparqlEngine` is the shipped implementation.

## What the front-end covers

- **Query** — all four query forms (SELECT/ASK/CONSTRUCT/DESCRIBE), basic
  graph patterns, `OPTIONAL`, `UNION`, `MINUS`, `GRAPH`,
  `FILTER`/`BIND`/`VALUES`, property paths, `GROUP BY`/aggregates,
  `EXISTS`/`NOT EXISTS`, solution modifiers, and RDF 1.2 quoted triple terms
  (`<<( s p o )>>`).
- **Update** — `INSERT DATA`/`DELETE DATA`, the `DELETE`/`INSERT … WHERE`
  family (`WITH`/`USING`, `DELETE WHERE`), `LOAD`, and
  `CLEAR`/`DROP`/`CREATE`/`ADD`/`MOVE`/`COPY`.

Anything outside this surface — and every malformed query — is a typed
`ParseError`, never a silently degraded parse.

## How the evaluator works

- **Multiset (bag) semantics** — solutions are a bag, preserved until
  `DISTINCT`/`REDUCED`, per the SPARQL algebra.
- **Interned evaluation** — constants resolve to a dataset `TermId` once;
  solution comparison is an integer compare; computed FILTER/BIND values that
  already exist in the dataset are promoted to the interned id at mint time.
- **Property paths in-engine** — the full path algebra
  (`* + ? / | ^ !()`) evaluated over the same indexed surface, wasm-safe.
- **Cost-based BGP planning** — join order is chosen by a cost model;
  `NativeSparqlEngine::explain_query` exposes the chosen order as an ordered
  list of triple-pattern strings so you can audit planner decisions without
  running the query.
- **EXISTS decorrelation** — correlated `EXISTS`/`NOT EXISTS` filters are
  decorrelated rather than re-evaluated per row.
- **The SERVICE seam** — `SERVICE` federation is evaluated through a
  **host-injectable transport**: the engine itself performs no I/O, so
  federation stays wasm-portable and the host decides how (and whether)
  remote endpoints are reached. All seven W3C `service` federation cases pass
  through this seam.
- **Hard-fail** — an out-of-scope algebra node or unimplemented builtin is a
  typed `EvalError::Unsupported`, never a partial or wrong answer.

## Entailment regimes

SPARQL queries can be answered under an entailment regime by materializing the
dataset first with [`purrdf-entail`](../entailment.md) — `Regime::from_iri`
maps a `sparql:entailmentRegime` IRI to the matching engine.

## Conformance

The full W3C SPARQL 1.1 query + update evaluation suites plus the SPARQL 1.2
suite are vendored verbatim and run by `purrdf-sparql-conformance`; every
non-pass is a typed, ledgered expected-failure. See
[Conformance & Testing](../project/conformance.md) and
[`docs/CONFORMANCE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/CONFORMANCE.md)
for the live matrix.

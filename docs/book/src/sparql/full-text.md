<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Full-Text Search

`purrdf-text` (`purrdf::text` from the umbrella crate) is PurRDF's
deterministic full-text index. It reads RDF 1.2 literals out of a frozen
dataset, tokenizes them by the Unicode standard's own rules, and answers ranked
retrieval queries with BM25 scores — from SPARQL, through the evaluator's
property-function seam, under IRIs the caller supplies.

It is an **out-of-core sibling crate**, not a kernel change: nothing in
`purrdf-core`, `purrdf-sparql-algebra` or the property-function seam itself was
altered to admit it. That is the shape every extension in this chapter takes.

## Two relations, one index

An index is built once over a dataset and shared by two property functions,
which are two distinct types rather than one type with a mode switch:

```text
?doc <caller-iri>  ( "needle" ?score ?rank ?lang ?matched )   # ranked retrieval
?doc <caller-iri2> ( "term"   ?lang  ?position )              # one row per occurrence
```

- **Ranked retrieval** (`TextSearchRelation`) emits one row per matching
  document. `?score` is an `xsd:decimal` carrying the exact BM25 value,
  `?rank` is the document's 1-based position **within its `(graph, language)`
  partition**, `?lang` is the partition's language tag, and `?matched` counts
  how many distinct needle terms the document holds.
- **Term occurrence** (`TermOccurrenceRelation`) emits one row per occurrence
  of a single term, with its token `?position`.

Phrase and proximity search are delivered by composition in SPARQL rather than
by an embedded query dialect — an embedded syntax would have minted one more
incompatible vendor language, which is exactly what a carrier must not do:

```sparql
# "quick" immediately followed by "brown"
SELECT ?doc WHERE {
  ?doc <https://example.org/pf/occurs> ( "quick" ?l ?p1 ) .
  ?doc <https://example.org/pf/occurs> ( "brown" ?l ?p2 ) .
  FILTER(?p2 = ?p1 + 1)
}
```

`FILTER(ABS(?p2 - ?p1) <= 3)` is proximity; `FILTER(?matched = 2)` on the
ranked relation is conjunctive retrieval.

## Wiring it from Rust

The index takes the predicates whose literals it should read and a graph
selector — `GraphSelector::Any`, `Default`, or `Named(iri)` — and PurRDF
supplies no default for either. A configuration naming no predicate is a
typed `TextError::Config`, not a guess.

```rust,ignore
use std::sync::Arc;
use purrdf::sparql::{NativeSparqlEngine, PropertyFunctionRegistry, QueryOptions};
use purrdf::text::{
    GraphSelector, TermOccurrenceRelation, TextIndex, TextIndexConfig, TextSearchRelation,
};
use purrdf::{SparqlRequest, TermValue};

// Which literals to index: the caller names the predicates.
let config = TextIndexConfig::new(
    vec![TermValue::iri("https://example.org/note")],
    GraphSelector::Any,
)?;
let index = Arc::new(TextIndex::from_dataset(&dataset, &config)?);

// Which IRIs a query calls the index by: the caller names those too.
let mut registry = PropertyFunctionRegistry::new();
registry.register(
    "https://example.org/pf/search".to_owned(),
    Arc::new(TextSearchRelation::new(Arc::clone(&index))),
);
registry.register(
    "https://example.org/pf/occurs".to_owned(),
    Arc::new(TermOccurrenceRelation::new(index)),
);

let result = NativeSparqlEngine::new().query_with_options_view(
    &dataset,
    SparqlRequest {
        query: r#"SELECT ?doc ?score ?rank WHERE {
                    ?doc <https://example.org/pf/search> ( "quick brown" ?score ?rank ?lang ?matched )
                  } LIMIT 3"#,
        base_iri: None,
        substitutions: &[],
    },
    QueryOptions { property_functions: &registry, ..QueryOptions::EMPTY },
)?;
```

A registered IRI is recognized in predicate position exactly, so no parser
option is needed to reach it. An IRI a query names that is declared through
`ParserOptions` but not registered hard-fails naming the IRI; an IRI in
neither stays an ordinary triple pattern.

This is a Rust-host seam. The index and its relations are host closures, so
they do not cross the Python, WebAssembly or C boundary — only the data-shaped
property functions (frozen tables, graph-backed tables and path witnesses) do.
See [Reaching extensions from other hosts](querying.md#reaching-extensions-from-other-hosts).

## Every score is exact, and identical on every target

Ranking is done entirely in base-10 fixed-point integer arithmetic (`i128`,
twelve fractional digits). No floating-point value enters the crate: its root
denies `clippy::float_arithmetic`, so none can.

That is a correctness requirement, not a preference. BM25 needs a natural
logarithm, and a libm `ln` may differ by a unit in the last place between
implementations — enough to reverse the order of two near-tied documents, so
the same query over the same data would return rows in one order from a native
build and another from a `wasm32-unknown-unknown` build of the same engine.
The logarithm here is a fixed-length integer series with a fixed iteration
count, never a convergence test, so its result is a pure function of its input
on every target. The ranking — row order together with every score's decimal
lexical — is pinned by a single test body carrying both `#[test]` and
`#[wasm_bindgen_test]`, so `make wasm-test` executes it on wasm32 against the
same expectations `cargo test` asserts natively.

The BM25 constants `k1 = 1.2` and `b = 0.75` are crate constants rather than
caller parameters. PurRDF is a carrier, and optionality that changes semantics
per consumer is forbidden: two callers must not get different ranks out of the
same index and the same needle.

## Ordering, and the idioms that reproduce it

Corpus statistics are computed per `(graph, language)` partition, so a score is
a number relative to one corpus and `?rank` is a position within that
partition, never a global one. Rows are emitted in `(partition key ASC, rank
ASC)` order; within a partition the order is `(score DESC, document id ASC)`,
and document ids are assigned after sorting on `(graph, subject, language)`, so
the tie-break is reproducible across independently built indexes. A query that
wants one ranked list binds `?lang` (and `?graph` through the selector) or
builds a single-partition index.

Two consequences worth knowing:

- **Bare `LIMIT k` is top-k.** Emission order *is* rank order, so the
  evaluator's row-ceiling licence applies and the relation stops after `k`
  rows. `ORDER BY DESC(?score) LIMIT k` is not pushed down — `ORDER BY` plus
  `LIMIT` has no certified lower bound the governor can license — and the
  `ORDER BY` is redundant anyway.
- **`ORDER BY ?rank` is the reproducing idiom.** A score reaches the consumer
  as a decimal of fixed width, so two rows can report the same `?score` while
  carrying different `?rank`; `ORDER BY DESC(?score)` can therefore disagree
  with the exact internal order, and `?rank` cannot.

## RDF 1.2 first class

`RdfDataset::quads()` returns only the asserted triple table; annotations live
in a separate side table. The index reads both layers, so text carried only by
`:s :p :o {| :note "..." |}` is searchable, with the reifier as the document
subject. An index reading only the asserted layer would index zero annotation
literals and report nothing — the crate's tests guard against exactly that.

## Stated limits

- Document ids are a function of content *except* through blank-node labels,
  which are a parsing artifact: two isomorphic datasets with different labels
  produce different index fingerprints.
- The index is in-memory and built over a frozen dataset. `verify_binding`
  checks that the index a query runs against was built over the dataset in
  front of it, closing the silent channel where a stale index emits documents
  that join back to zero rows.

The scoring design record — including the fixed-point logarithm and the
Unicode table versions folded into the index fingerprint — is
[`docs/design/purrdf-text-scoring.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-text-scoring.md).

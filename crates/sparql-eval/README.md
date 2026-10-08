<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-sparql-eval` — Native SPARQL Evaluator

[![crates.io](https://img.shields.io/crates/v/purrdf-sparql-eval.svg)](https://crates.io/crates/purrdf-sparql-eval)
[![docs.rs](https://docs.rs/purrdf-sparql-eval/badge.svg)](https://docs.rs/purrdf-sparql-eval)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-sparql-eval` is the native, RDF 1.2-first **multiset SPARQL evaluator**
of the PurRDF toolkit. It consumes the
[`purrdf-sparql-algebra`](https://crates.io/crates/purrdf-sparql-algebra)
front-end and evaluates over the
[`purrdf-core`](https://crates.io/crates/purrdf-core) IR's `DatasetView`
**entirely in interned `TermId` space** — constants resolve to a dataset id
once, solutions are a single integer compare apart, and computed FILTER/BIND
values that already exist in the dataset are promoted to the interned id at
mint time.

For repeated work, retain an `Arc<PreparedQuery>` from `prepare_query` or
`prepare_algebra` and pass data through substitutions. Compiler-produced algebra
goes directly through structural and registry admission without rendering or parsing
query text. `PreparedQuery::rewritten` also admits and feasibility-orders its input.
The admitted algebra is immutable and accessible through `PreparedQuery::query()`;
changed algebra must be admitted into a new plan.
`query_prepared_governed_in_operation` charges a caller-owned governor across
multiple queries; immutable plans and the governor can be shared by worker-local
engines without locking evaluation globally. Its fallible-view sibling
`query_prepared_governed_fallible_in_operation` preserves operational failure
precedence and publishes only after the final ready checkpoint.
`query_prepared_governed_fallible_view` executes that same prepared fallible
boundary under fresh per-call governors, returning combined view and governor
evidence. A healthy governor trip carries certified partial answers; an
operational failure discards them. Both entries have federation variants that
accept a service resolver under the same governor.

The engine's prepared-plan and join-order caches each default to 4096 entries and
64 MiB of charged payload. Configure them independently with
`with_plan_cache_limits` and `with_order_cache_limits`; `PlanCache::with_limits`
serves standalone callers. Retention uses LRU eviction and observable
`CacheStats`. Zero capacity or an oversized plan causes recomputation, never
weaker execution. Byte accounting covers keys, owned algebra, vector capacities,
and shared-string storage charged per occurrence; it excludes allocator overhead
and caller-retained handles. `PlanMemoryObserver` separately reports all live
admitted plan payloads and the portions retained by a cache or held exclusively
by callers, even after cache replacement or destruction. Each allocation is counted
once regardless of `Arc` clones. Admission estimates and
`PreparedQuery::retained_size_bytes` describe the immutable admitted payload.
Public totals saturate without losing internal lifetime accounting.
Neither counter is an RSS measurement. Dataset statistics key
join-order hints only, never result reuse. The existing caller-owned
`eval::BgpOrderCache` alias remains available with its original type.

`make bench-prepared-reuse` measures cold/warm preparation and prepared execution
using the release profile (O3/full LTO). The `prepared_admission` benchmark isolates
structural revalidation and minimal repeated execution; the
`prepared_reuse_counters` example separately records allocator requests, cache
activity, plan lifetimes and governor work. Measurements are report-only and use
synthetic data. Allocator-requested bytes and governor intermediate cells are
different denominations, recorded separately.

`construct_prepared_into_view` appends a complete typed CONSTRUCT graph to an
existing `RdfDatasetBuilder`, using the ordinary template and projection-loss
machinery. It stages and validates terms without freezing or serializing an
intermediate dataset. Repeated appends mint destination-disjoint blank nodes
while preserving blank identities carried by input bindings. The operation and
fallible-view variants publish nothing after an error, cancellation or exhausted
governor. `GraphBuildStats` records staged statements and payload, actual copied
payload, intermediate freezes and supplied governor evidence independently of
the graph's identity.

Design pillars:

- **Multiset (bag) semantics** — solutions are a bag, preserved until
  `DISTINCT`/`REDUCED`.
- **Property paths in-engine** — the full path algebra (`* + ? / | ^ !()`)
  evaluated over the same indexed surface, wasm-safe.
- **Query features** — aggregates, `EXISTS`/`NOT EXISTS` answered by a
  memoized existence probe where a prepare-time proof licenses it and by the
  per-row definition otherwise, cost-based BGP planning (with an
  `explain_query` introspection API), SPARQL UPDATE, the SEP-0009 composite
  datatypes (`FOLD`/`UNFOLD` and the `cdt:` function library), and
  host-injectable `SERVICE` and `LOAD` sources so federation stays
  wasm-portable (no HTTP client ships; the host supplies the `HttpTransport`
  and the resolvers). A caller hands a request its sources through
  `QueryOptions` — `remote` answers `SERVICE` in a query and in an UPDATE's
  `WHERE` alike, and `load` answers `LOAD`, taking precedence over a resolver
  installed on the engine — so every entry, governed or not, federates the
  same way. The seam has two consumers: a Rust host that implements
  `HttpTransport`, and the wasm package's asynchronous lane, whose resolvers
  suspend the evaluation through JSPI while a JavaScript host answers each
  `SERVICE` and `LOAD`.
- **SPARQL 1.1 Protocol** — the `protocol` module reads an HTTP request's
  method, `Content-Type`, query string and body into a query or update
  operation, applies the dataset parameters (`default-graph-uri`,
  `named-graph-uri`, `using-graph-uri`, `using-named-graph-uri`) to its text,
  and negotiates the result format from an `Accept` header. It performs no
  I/O; a refusal is a typed `ProtocolError` the host maps to its HTTP status.
- **Caller-keyed extension seams** — scalar functions (`UserFunctionRegistry`,
  whose native bodies carry SPARQL's expression-error channel so a per-solution
  domain error drops the row under `FILTER` or leaves the variable unbound
  under `BIND` rather than aborting the query), property functions
  (`PropertyFunctionRegistry`, with the path-witness relations and the
  embedding-kNN relation over a PURREMB space shipped in-crate), custom
  aggregates (`AggregateRegistry`, plus a namespace-keyed statistical set), and
  per-service `SERVICE` context (`ServiceCatalog`/`ServiceProfile`: headers,
  credentials, timeouts, capabilities, deny by default). Every seam is keyed by
  IRIs the caller supplies; the crate mints none.
- **Governed execution** — every entry point has a governed twin under
  caller-set ceilings (fuel, answer rows, intermediate cells, scratch bytes,
  remote requests, deadline) that trips with certified rows, never a wrong
  answer.
- **Hard-fail** — an out-of-scope algebra node or unimplemented builtin is a
  typed `EvalError::Unsupported`, never a partial or wrong answer.

## How it evaluates

- **A plan arena per evaluation.** Every entry builds a dense tree over the
  query's algebra whose node ids are the governor ledger's own ordinals, so
  charge receipts, `LIMIT` pushdown ceilings, `EXISTS` sites and the `SERVICE`
  endpoint index are indexed tables rather than maps keyed by node address.
- **Expressions compiled once, run per row.** Each `FILTER`, `BIND`, `OPTIONAL`
  condition, `ORDER BY` key, aggregate argument and `UNFOLD` expression is
  compiled into a flat program stored on its plan site. An operator call links
  the program's variables to solution columns, its constants to a pool and its
  constant regular expressions once; each row then runs the program on an
  explicit value stack. Kleene `&&`/`||`, `IF`/`COALESCE`/`IN` laziness, `BNODE`
  memoization and the order of every charge are the SPARQL evaluation order.
- **A term's nesting costs no stack; a plan's height is admitted first.**
  Nested triple terms, property paths (compiled into a flat path program with
  reach caches indexed by program op) and CONSTRUCT templates run over explicit
  work lists, so how deeply a term nests is bounded by memory and by the
  governors. The evaluator's analyses and its evaluation proper recurse once
  per level of the plan, so a plan is measured iteratively before they run,
  at preparation and on every evaluation, against the stack the evaluating
  thread has left above `purrdf_stack::MARGIN_BYTES`: a plan too tall for it
  is refused with `EvalError::StackExhausted`
  (`native-sparql-evaluation-stack-exhausted`), and on `wasm32` a plan past
  the budget kept under the JavaScript engine's call stack with
  `EvalError::HostStackExhausted` (`native-sparql-host-stack-exhausted`). A
  correlated substitution copies its subtree inside a scope that discards the
  half-built copy and refuses the same way when the stack runs low.
- **One per-row checkpoint.** `FILTER`, `BIND`, `UNFOLD` and aggregate loops pass
  every row through the same checkpoint: a latched trip is observed first, and a
  loop that forks across threads forks only the rows the remaining fuel admits
  and commits them in source order, so it trips on the row the sequential loop
  would and spends the same fuel.

The mechanisms are measured, not asserted: `benches/expr_vm.rs`,
`pattern_dispatch.rs`, `deep_nesting.rs`, `governed_eval.rs` and
`query_eval.rs` report the expression, dispatch, nesting and governed-loop costs
(report-only).

The engine is gated by the W3C SPARQL 1.1 and 1.2 conformance suites (run
through the workspace harness) and
builds for `wasm32-unknown-unknown`.

## Usage

```sh
cargo add purrdf-sparql-eval
```

```rust
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;

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

Serialize results to SPARQL JSON/XML/CSV/TSV with the sibling
[`purrdf-sparql-results`](https://crates.io/crates/purrdf-sparql-results) crate.

## Dated native XPath expressions

`NativeSparqlEngine::with_xpath_regex(profile, limits)` selects a dated native
law for `REGEX` and `REPLACE`. The profiles and finite limits live in
`purrdf_core::xsd_regex::xpath`. `QueryOptions::with_xpath_regex` selects the law
and limits for one request; an unset request inherits the engine's selection.
An unselected engine retains its compatibility behavior.

Selection reaches constant-linked and dynamic patterns, prepared execution,
worker and user-function children, governed and fallible queries, on-demand row
filters and UPDATE WHERE clauses. Preparation retains algebra; execution links
patterns under the current request. A retained program is reused only by exact
law/source/flags identity after current source and program-storage admission.
Every match and replacement receives current finite execution/output limits.

Pattern and flag syntax retains SPARQL expression-error behavior. Resource or
allocation exhaustion is an operational query failure with its native machine
code, including oversized source patterns. It cannot become UNDEF, a false
filter result or a partial answer. A refused UPDATE leaves the input dataset
unchanged. Both dated laws admit backreferences; non-capturing groups and `q`
require the 3.1 law. Replacement checks empty-match and replacement-language
errors independently of operational refusal.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
under `purrdf::sparql`; depend on `purrdf-sparql-eval` directly only when you
want the evaluator alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

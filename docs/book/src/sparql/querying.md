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

## SPARQL 1.2 temporal adjustment: `ADJUST`

`ADJUST(value, timezone)` shifts an `xsd:dateTime`/`xsd:date`/`xsd:time` value to a
given timezone offset, or attaches one to an untimezoned value. The SPARQL 1.2
Query specification's own text carries no `ADJUST` section; the function's one
documented definition is [SEP-0002](https://github.com/w3c/sparql-dev/blob/main/SEP/SEP-0002/sep-0002.md)'s
two-argument signature, which maps onto XPath and XQuery Functions and Operators
§9.6's `fn:adjust-*-to-timezone` family (the same table `purrdf-xsd` implements
for every other SPARQL 1.2 temporal builtin).

```sparql
SELECT ?adjusted WHERE {
  BIND(ADJUST("2011-01-10T14:45:13Z"^^xsd:dateTime,
              "-PT05H"^^xsd:dayTimeDuration) AS ?adjusted)
}
```

`timezone` is an `xsd:dayTimeDuration` in `[-PT14H, PT14H]` (whole minutes only),
or the empty simple literal `""` — SPARQL's stand-in for XPath's empty-sequence
"remove the timezone" argument, since SPARQL itself has no empty sequence; this is
the same resolution every other known SEP-0002 implementation (e.g. Apache Jena's
`E_AdjustToTimezone`) reaches. Arity is fixed at two and enforced at parse time —
`ADJUST(?x)` and `ADJUST(?x, ?tz, ?extra)` are both refused before evaluation.
Every domain or type violation (a non-whole-minute offset, an out-of-range offset,
a non-temporal first argument) is a SPARQL type error, which — inside `BIND`,
`FILTER`, or an aggregate argument — poisons to unbound rather than aborting the
query, per the engine's ordinary type-error discipline.

## The `VERSION` declaration

A query or update prologue may declare `VERSION "<string>"` (SPARQL 1.2 Query
specification §4.4). Parsing is syntax-only — any string is accepted, and when the
prologue repeats the declaration the last one wins — but the declared value is no
longer discarded: `Query::version()` / `Update::version()` expose it as a typed
`SparqlVersion`, alongside the existing `dataset()`/`base_iri()` accessors.

Evaluation is the admission boundary. `VERSION "1.2"` and `VERSION "1.2-basic"`
are recognized; any other declared string — `VERSION "1.1"`, a typo, a future
version this build predates — is refused at evaluation admission with a typed
error naming the declared string, before any work is spent.

`VERSION "1.2"` evaluates normally on the full engine. `VERSION "1.2-basic"` is
enforced as a narrower profile: the SPARQL 1.2 Query specification's §4.3.1
"Version Labels" table defines `1.2-basic` as full `1.2` syntax "without triple
terms and without triple patterns that have a triple pattern in their subject
or object position" — the RDF 1.2 triple-term/reification feature area. A
`1.2-basic` query or update that uses a quoted triple term (`<<( s p o )>>`), a
reifying triple or annotation (`<< s p o >>`, `{| ... |}`), a ground triple term
in `VALUES`, or one of the "Functions on Triple Terms" (`TRIPLE`, `isTRIPLE`,
`SUBJECT`, `PREDICATE`, `OBJECT`, §17.4.6) is refused at evaluation admission
with a typed error naming the offending construct — for an update, with no
mutation applied. A `1.2-basic` request that uses none of those constructs
evaluates exactly as a `1.2` one would.

## Extending the evaluator: custom aggregates

Beyond the SPARQL 1.1 built-in aggregates (`COUNT`/`SUM`/`AVG`/`MIN`/`MAX`/
`SAMPLE`/`GROUP_CONCAT`), a Rust host may register additional `GROUP BY`
reductions and reach them from query text as `AGG(<iri>, [DISTINCT] arg, arg, …)`
— the issue-normative positional spelling (a deliberate divergence from Jena
ARQ's `AGG <iri>(args)`). Where `purrdf_sparql_eval::property_fn` injects a
*relation* (a row source in graph-pattern position) and `user_fn` injects a
*scalar function* (one value per call), `agg_fn` injects a **fold**: a group's
rows reduce to one value through a caller-supplied accumulator, exactly as a
built-in aggregate does.

A registered aggregate implements two traits — `CustomAggregate` (the
per-IRI factory: declared arity, `Volatility`, `AlgebraicClass`, and a
declared state bound) and `AggregateAccumulator` (the per-invocation fold:
`step` one already-evaluated argument tuple at a time, `combine` two partial
folds in source order, `finish` to the group's answer) — and is registered
into an `AggregateRegistry` under an IRI of the caller's choosing:

```rust,ignore
use std::sync::Arc;
use purrdf_core::{SparqlRequest, TermValue};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, CustomAggregate, EvalError,
    NativeSparqlEngine, QueryOptions, Volatility,
};

/// A running total over one numeric argument — `example.org`'s own
/// `AGG(<https://example.org/agg#total>, ?x)`.
struct TotalAccumulator {
    sum: i64,
}

impl AggregateAccumulator for TotalAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.sum += n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        if let Ok(Some(TermValue::Literal { lexical_form, .. })) = other.finish()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.sum += n;
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self // a running total IS its own sufficient merge state
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.sum.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        )))
    }
}

struct TotalAggregate;

impl CustomAggregate for TotalAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable // eligible for the within-group parallel fold
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0 // stateless besides the running total
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(TotalAccumulator { sum: 0 })
    }
}

let mut registry = AggregateRegistry::new();
registry.register(
    "https://example.org/agg#total",
    Arc::new(TotalAggregate),
);

let engine = NativeSparqlEngine::new();
let result = engine.query_with_options_view(
    &ds,
    SparqlRequest {
        query: "SELECT (AGG(<https://example.org/agg#total>, ?x) AS ?t) WHERE { ?s <https://example.org/n> ?x }",
        base_iri: None,
        substitutions: &[],
    },
    QueryOptions {
        aggregates: Some(&registry),
        ..QueryOptions::EMPTY
    },
)?;
```

An `AGG(<iri>, …)` call to an unregistered IRI, with the wrong argument
count, or with an invalid `; NAME=value` scalarval clause (an unrecognized
name, a duplicate name, a missing required name, or a wrong-typed value — see
below), is refused when the query is **prepared** — before any budget unit is
spent — and the prepared plan carries the registry's fingerprint, so a plan
admitted under one registry can never silently run under another (see
`purrdf-sparql-eval`'s `agg_fn` module docs for the full trust-boundary and
determinism contract, including `into_any`'s role in merging structural state
a fold's finished answer alone cannot reconstruct).

### Named scalar-value parameters

Beyond its positional `args`, `AGG(<iri>, …)` admits zero or more trailing
`; NAME=value` clauses — a named, per-aggregation scalar parameter, generalizing
`GROUP_CONCAT`'s own `; SEPARATOR="…"` (SPARQL's existing precedent for a named
scalar aggregate parameter) to any custom aggregate: `AGG(<ns>PERCENTILE, ?x;
P=0.95)`, `AGG(<ns>TOPK, ?x; K=3)`. `NAME` is matched case-insensitively and
stored upper-cased; `value` is any SPARQL literal, so a numeric scalarval keeps
its natural datatype. Unlike a positional argument, a scalarval is evaluated
**once for the whole aggregation**, never per row — the correct semantics for
a parameter like a percentile rank or a "top k" count, which must be one fixed
value across the group, not a per-row expression the query author merely
intends to hold constant.

A registered aggregate declares which names it accepts via
`CustomAggregate::scalarvals`, returning a slice of `ScalarvalSpec { name,
kind }` (`kind` is `ScalarvalKind::Numeric` or `ScalarvalKind::String`); every
declared name is required. `CustomAggregate::init` receives the call site's
scalarvals already resolved to `TermValue` and already validated against that
declaration, so an accumulator can read them back by name
(`scalarvals.iter().find(|(k, _)| k == "P")`) without re-checking type or
presence.

`AGG(<iri>, …)` execution is governed like any other fold: profile v6 adds two
charge points — `aggregate-invocation` (once per group per aggregate
expression) and `aggregate-accumulation` (once per value inspected) — shared
by built-in and custom aggregates alike, so a registered aggregate over a
given group shape spends identical fuel to a built-in one. See
[`docs/SPARQL-GOVERNOR-PROFILE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/SPARQL-GOVERNOR-PROFILE.md).

## The statistical aggregate set

`purrdf-sparql-eval` ships ten exact statistical aggregates — `MEDIAN`,
`PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`, `FIRST`,
`LAST`, `TOPK` — as first-party `CustomAggregate` instances behind the same
`agg_fn` seam, closed (no caller extension point) and reached through **one**
call: `AggregateRegistry::register_statistical_aggregates`:

```rust,ignore
let mut registry = AggregateRegistry::new();
registry.register_statistical_aggregates("https://example.org/agg#");
// Now reachable: AGG(<https://example.org/agg#MEDIAN>, ?x), …STDDEV…, …TOPK…
```

As with every vocabulary PurRDF touches, `namespace` is caller-supplied
configuration with no fabricated default — a host that never calls this method
gets none of the ten IRIs registered, exactly as a host that never configures
`ParserOptions::extension_fn_namespaces` gets none of the built-in scalar
extension functions.

Per-member semantics:

- **Numeric members** (`STDDEV*`/`VARIANCE*`, `MEDIAN`, `PERCENTILE`) follow
  the same `integer ⊂ decimal ⊂ float ⊂ double` promotion tower the built-in
  `SUM`/`AVG` fold uses; a non-numeric input **poisons** the fold to unbound
  rather than raising a hard error, matching `SUM`'s own discipline. Only
  `STDDEV`/`STDDEV_POP`'s final square root leaves the exact decimal tower;
  `VARIANCE`/`VAR_POP` never do.
- **`STDDEV`/`VARIANCE`** are the sample statistics (`n - 1` denominator,
  unbound under two values); **`STDDEV_POP`/`VAR_POP`** are the population
  statistics (`n` denominator, defined at one value) — SQL's own naming
  convention.
- **`MEDIAN`** is `PERCENTILE` at `p = 0.5` under linear interpolation between
  the two closest ranks — for an even-sized group this is exactly the mean of
  the two middle values.
- **`PERCENTILE`** takes a named scalarval, `AGG(<ns>PERCENTILE, ?x; P=0.95)`:
  `P` is resolved once for the whole aggregation (never per row — see "Named
  scalar-value parameters" above); a `P` outside `[0, 1]` poisons to unbound,
  the same "poison, don't abort" discipline every numeric member uses. A
  missing or non-numeric `P` is refused at prepare time.
- **`MODE`** works over any term kind, counted by RDF term identity (not value
  equality — `"5"^^xsd:integer` and `"05"^^xsd:integer` count as two different
  terms). A tie is broken toward the smallest term under the same total order
  `MIN`/`ORDER BY` use.
- **`FIRST`/`LAST`** work over any term kind, in input row order.
- **`TOPK`** takes a named scalarval, `AGG(<ns>TOPK, ?x; K=3)`: `K` must be a
  positive `xsd:integer`, resolved once for the whole aggregation (a `K ≤ 0`
  poisons to unbound; a missing or non-integer `K` is refused at prepare
  time). Since every SPARQL aggregate yields exactly one RDF term, `TOPK`
  answers the same way `GROUP_CONCAT` answers a multi-valued question — the
  top `k` values, in descending order, with their lexical forms joined by a
  single fixed space separator.

All ten declare `Volatility::Stable` and merge partial folds through real,
type-recovered structural state (`AggregateAccumulator::into_any`) rather than
a lossy re-derivation from a finished answer — so a large single group folds
in parallel chunks (see `crate::modifier::eval_custom_aggregate`) with a
result byte-identical to the sequential fold.

## Reaching extensions from other hosts

The property-function registry, the SHACL-AF function registry, and the
GENERAL custom-aggregate registry are all **Rust-closure seams**: a registered
relation, function, or aggregate is arbitrary host Rust (`init`/`step`/
`combine`/`finish` closures for an aggregate), so registering one is a
Rust-host-only operation. It genuinely cannot cross a Python, WebAssembly, or C
boundary as a string or any other FFI-shaped value — there is no callback
protocol this project is willing to invent for it, and none of the four host
surfaces below expose it.

purrdf's first-party **statistical set** is different, precisely because it is
NOT an arbitrary closure: `AggregateRegistry::register_statistical_aggregates`
takes only a namespace **string** and wires ten pre-built Rust instances
internally, so it crosses every host boundary this crate ships exactly the way
`property_fn_namespaces` does — no callback, no per-aggregate marshaling. Every
host surface threads it through as a keyword argument / flag / parameter named
`aggregate_namespace` (`aggregateNamespace` in camelCase-spelled JavaScript),
mirroring however that surface already threads `property_fn_namespaces` or the
nearest equivalent optional-string configuration:

- **Rust** (embedding the engine directly): `QueryOptions.aggregates`, as
  shown throughout this page.
- **Python** (`purrdf.Store` / `purrdf.MutableDataset`): the
  `aggregate_namespace` keyword on `query` / `query_governed` / `update` /
  `update_governed`.

  ```python
  store.query(
      "SELECT (AGG(<https://ex.example/agg#>MEDIAN, ?v) AS ?m) "
      "WHERE { ?s ex:value ?v }",
      aggregate_namespace="https://ex.example/agg#",
  )
  ```

- **CLI** (`purrdf query` / `purrdf update`): the `--aggregate-namespace IRI`
  flag.

  ```sh
  purrdf query --data data.ttl --aggregate-namespace 'https://ex.example/agg#' \
    'SELECT (AGG(<https://ex.example/agg#>MEDIAN, ?v) AS ?m) WHERE { ?s ?p ?v }'
  ```

- **WebAssembly** (`QueryEngine.queryGoverned` / `updateGoverned`): the
  `aggregateNamespace` option, alongside the other governed-call options.

  ```js
  const outcome = engine.queryGoverned(dataset, sparql, {
    aggregateNamespace: "https://ex.example/agg#",
  });
  ```

- **C ABI** (`purrdf_query_governed` / `purrdf_update_governed`): a nullable
  `const char *aggregate_namespace` parameter, the same optional-C-string
  convention every other nullable string on this ABI uses.

  ```c
  purrdf_query_governed(dataset, query, /* base_iri */ NULL,
                         "https://ex.example/agg#", &governors, /* … */);
  ```

`namespace` stays caller-supplied configuration with no fabricated default on
every one of these surfaces: omitting it (`None` / not passing the flag /
`undefined` / a null pointer) leaves every one of the ten names an ordinary
unregistered custom-aggregate IRI, refused exactly as before this parameter
existed.

Two structural limits apply everywhere the statistical set is reachable:

- The **entailment-aware query lane** (`query_with_entailment_governed` and
  its Python/CLI/WebAssembly/C wrappers) evaluates under no `QueryOptions` seam
  at all, on any regime — closing that gap is a larger, separate change to
  `purrdf-entail`'s query-under-a-regime plumbing, not a parameter
  pass-through, so every host refuses (or has no parameter for) combining
  `aggregate_namespace` with an entailment-regime query rather than silently
  dropping it.
- SPARQL UPDATE's grammar admits an aggregate only inside a nested
  `SELECT … GROUP BY` in a `WHERE` clause (an ordinary `DELETE`/`INSERT WHERE`
  basic graph pattern has no `GROUP BY` of its own) — every host's `update`
  entry reaches the statistical set through exactly that nested-subquery
  shape.

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

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
  `EXISTS`/`NOT EXISTS`, solution modifiers, RDF 1.2 quoted triple terms
  (`<<( s p o )>>`), and `LATERAL` (a SEP-0006 extension — see
  [below](#lateral-sep-0006)).
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
- **`EXISTS`/`NOT EXISTS`, defensibly** — one substitution-based definition
  (SEP-0007), answered either by a memoized existence probe or by the per-row
  definition itself, chosen per site by a prepare-time admissibility proof —
  see [below](#exists-under-sep-0007).
- **The SERVICE seam** — `SERVICE` federation is evaluated through a
  **host-injectable `ServiceResolver`**: the engine itself performs no I/O, so
  federation stays wasm-portable and the host decides how (and whether)
  remote endpoints are reached — down to per-service headers, credentials and
  capabilities (see
  [below](#per-service-context-the-serviceresolver-seam)). All seven W3C
  `service` federation cases pass
  through this seam. The forwarded body is re-emitted through the
  deterministic serializer — the federation wire format — whose
  parse → serialize → re-parse fidelity is swept over the 823-item corpus
  (every vendored W3C and first-party query and update text, plus this book's
  own examples) with an empty exception ledger.
- **Governed twins and explain receipts** — every query/update entry point has
  a governed counterpart running under caller-set ceilings (fuel, answer rows,
  intermediate cells, scratch bytes, remote requests, deadline) that trips
  with certified rows rather than a wrong answer, and `explain_query` returns
  a `QueryExplanation` whose ledger decomposes the fuel spent per algebra node
  and per charge point, beside the cost planner's estimate for each basic
  graph pattern. The normative charge schedule and the frozen 49-case governor
  corpus are documented in
  [`docs/SPARQL-GOVERNOR-PROFILE.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/SPARQL-GOVERNOR-PROFILE.md).
- **Hard-fail** — an out-of-scope algebra node or unimplemented builtin is a
  typed `EvalError::Unsupported`, never a partial or wrong answer.

## SPARQL 1.2 temporal arithmetic and adjustment

`+`, `-`, `*`, `/`, and unary `-` extend past the numeric tower to
`xsd:dateTime`/`xsd:date`/`xsd:time` (instants), `xsd:duration` and its two
subtypes `xsd:dayTimeDuration`/`xsd:yearMonthDuration`, and the five Gregorian
partial-date types (`xsd:gYearMonth`, `xsd:gYear`, `xsd:gMonth`,
`xsd:gMonthDay`, `xsd:gDay`). The SPARQL 1.2 Query specification's own text
defines no arithmetic beyond the numeric tower; the one documented table is
[SEP-0002](https://github.com/w3c/sparql-dev/blob/main/SEP/SEP-0002/sep-0002.md)'s,
which this section follows for coverage. `ADJUST` (below) is SEP-0002's
remaining, non-arithmetic addition.

### The operator table

SEP-0002's table has 24 rows: 11 are comparisons (`<`/`>` between two
`yearMonthDuration`s or two `dayTimeDuration`s, `=` between two `duration`s,
and `=`/`<`/`>` between two `date`s or two `time`s), already covered by
ordinary value comparison. The remaining 13 are arithmetic:

| Operands | Result |
|---|---|
| `date - date` | `dayTimeDuration` |
| `date + yearMonthDuration` | `date` |
| `date - yearMonthDuration` | `date` |
| `date + dayTimeDuration` | `date` |
| `date - dayTimeDuration` | `date` |
| `time - time` | `dayTimeDuration` |
| `time + dayTimeDuration` | `time` |
| `time - dayTimeDuration` | `time` |
| `dateTime - dateTime` | `dayTimeDuration` |
| `dateTime + yearMonthDuration` | `dateTime` |
| `dateTime - yearMonthDuration` | `dateTime` |
| `dateTime + dayTimeDuration` | `dateTime` |
| `dateTime - dayTimeDuration` | `dateTime` |

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?diff WHERE {
  BIND("2002-01-02T10:00:00"^^xsd:dateTime - "2001-01-01T10:00:00"^^xsd:dateTime AS ?diff)
}
```

Beyond this table, `purrdf` also accepts the general `xsd:duration` on every
row above (SEP-0002 lists `xsd:duration` among the operand types the operator
table covers, and the general type's own value space subsumes both subtypes),
duration `±`
duration, duration `×`/`÷` an exact number, and Gregorian `±` duration for all
five Gregorian types where the result does not require fabricating an absent
field (see [Divergence](#divergence-from-other-implementations) below):

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?next WHERE {
  BIND("2012-10"^^xsd:gYearMonth + "P1Y1M"^^xsd:yearMonthDuration AS ?next)
}
```

### Result datatype

A `+`/`-` between two durations, or between a duration and an instant/Gregorian
value, resolves its result's datatype from the operands' **declared tags**,
never from the computed component values: the result is `dayTimeDuration` iff
every duration operand declares `dayTimeDuration`, `yearMonthDuration` iff
every duration operand declares `yearMonthDuration`, and the general
`xsd:duration` otherwise. Two cases make this rule concrete, because either
one alone is satisfied just as well by a components-based rule that happens to
agree at that one point:

A zero-valued result keeps its operands' declared subtype rather than
collapsing to a generic zero:

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?zero WHERE {
  BIND("P1Y"^^xsd:yearMonthDuration - "P1Y"^^xsd:yearMonthDuration AS ?zero)
}
```

`?zero` is `"P0M"^^xsd:yearMonthDuration` — a components-only rule that
inspects the (zero) result rather than the (matching) declared tags would
reach the same answer here, which is exactly why the second case is needed.
Conversely, a sum whose *components* look exactly like a pure
`yearMonthDuration` still widens to the general type the moment either
operand's declared tag is the general one:

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?mixed WHERE {
  BIND("P1M"^^xsd:yearMonthDuration + "PT0S"^^xsd:dayTimeDuration AS ?mixed)
}
```

`?mixed` is `"P1M"^^xsd:duration`, not `"P1M"^^xsd:yearMonthDuration` — the
zero `dayTimeDuration` operand contributes nothing to the components but still
widens the result's declared type, because the rule reads tags, not values.

### Divergence from other implementations

Gregorian `±` duration can ask for a field the operand's type does not carry —
adding a duration with a months component to an `xsd:gDay` needs a month to
apply it to, and `xsd:gDay` has none. `xsd:gMonthDay` clamps a day that does
not exist in the shifted month down to that month's actual length, the same
rule `date`/`dateTime` already follow (XML Schema Appendix E) — but
`xsd:gMonthDay` carries no year for that clamp to run against, so `purrdf`
decides whether one would need to be fabricated by anchoring the *complete*
months-then-days computation at every year in one full 400-year Gregorian
period (the calendar's leap rule is exactly periodic at that length, so one
period is every anchor that could ever matter) and checking whether every
anchor agrees on the answer. `purrdf` answers exactly when they do, and
returns a typed error exactly when they don't — for a duration of any
magnitude, not only a bounded/ordinary one: the months and days carries are
reduced by the calendar's exact periodicity (400 years, 146,097 days) before
any anchor's arithmetic runs, so an astronomically large `yearMonthDuration`
or `dayTimeDuration` component decides the same way, in the same bounded
work, as a small one. The computation is judged as
a whole, not component by component: a duration's months half can land on an
intermediate day whose clamp is itself year-dependent even though the
*finished* answer, after the days half also runs, is not — `"--01-31"^^xsd:gMonthDay
+ "P1M1D"^^xsd:duration` is `"--03-01"` from every anchor (the day after
either Feb 28 or Feb 29 is always Mar 1), even though `"--01-31"^^xsd:gMonthDay
+ "P1M"^^xsd:yearMonthDuration` alone is genuinely ambiguous. The one
recurring example of a refused class is February: every other month has the
same length in every year, so a shift landing there is always safe, while a
shift landing on February with the day being clamped the 29th or later is
the case whose answer can turn on a year `xsd:gMonthDay` does not carry —
that is an example of the refused class, not the rule itself.
RDF4J answers these by fabricating the missing field (year 0, January, or day
1) through its underlying JAXP calendar and returning a value built on that
fabrication — for example `"---31"^^xsd:gDay + "P1M"^^xsd:yearMonthDuration`
answers `"---29"`, clamped against a fabricated leap year. `purrdf` matches
RDF4J on every case whose answer does not depend on the fabricated field —
including `"2012-10"^^xsd:gYearMonth + P1Y1M = "2013-11"`, the one Gregorian
case RDF4J's own test suite pins — and returns a typed error exactly where the
answer would depend on which fabricated value RDF4J's calendar happened to
pick:

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?bound WHERE {
  BIND("---31"^^xsd:gDay + "P1M"^^xsd:yearMonthDuration AS ?maybe)
  BIND(BOUND(?maybe) AS ?bound)
}
```

`?bound` is `false`: the typed error `?maybe`'s `BIND` raises poisons to
unbound, the engine's ordinary type-error discipline. That poisoning is also
this divergence's limit: at the SPARQL surface, a typed error and a
reference implementation that instead *discarded* its own fabricated answer
(rather than returning it) would both leave the same variable unbound —
identically. The visible difference between "refuses to fabricate" and
"fabricates and returns a value" is real, but it lives at the value-space API
boundary (an `Err` versus an `Ok` carrying a specific answer), not in a
SPARQL query's own results, where both a refusal and a hypothetical discard
render the same way.

### Extensions beyond SEP-0002 and F&O

`purrdf` adds three operators SEP-0002 and XPath and XQuery Functions and
Operators (F&O) do not define, each grounded in an existing rule extended to a
type F&O left out:

- **`SUM`/`AVG` over durations.** SPARQL 1.1 §18.5.1.3 defines `SUM` by
  repeated `op:numeric-add`, whose domain is the numeric tower only; `purrdf`
  extends the same fold to the duration group, which is exact and associative
  under componentwise addition.

  ```sparql
  PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
  SELECT (SUM(?d) AS ?total) WHERE {
    VALUES ?d { "P1M"^^xsd:yearMonthDuration "P2M"^^xsd:yearMonthDuration }
  }
  ```

  `?total` is `"P3M"^^xsd:yearMonthDuration`. A group mixing numeric and
  duration values stays unbound — the extension does not widen `SUM`'s
  existing numeric-only acceptance, it only adds a second, disjoint one.

- **Unary minus on durations.** F&O's unary minus (§4.2.8) is numeric-only and
  defines no duration form. `purrdf` negates a duration's two components
  together, so `-(?duration)` never produces the mixed-sign value the type
  cannot represent. Unary plus deliberately stays numeric-only, so
  `+(?duration)` is a type error while `-(?duration)` is not:

  ```sparql
  PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
  SELECT ?neg WHERE {
    BIND(-("P1Y2M"^^xsd:yearMonthDuration) AS ?neg)
  }
  ```

  `?neg` is `"-P1Y2M"^^xsd:yearMonthDuration`.

- **`duration ÷ duration` by value commensurability.** F&O defines only the
  two same-subtype forms (`op:divide-yearMonthDuration-by-yearMonthDuration`,
  `op:divide-dayTimeDuration-by-dayTimeDuration`). `purrdf` also accepts the
  general `xsd:duration`, dispatching on whether the two operands' *values*
  are commensurable (both purely months, or both purely seconds) rather than
  on their declared tags, so a `dayTimeDuration` and a general `xsd:duration`
  that happens to be purely day-shaped still divide:

  ```sparql
  PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
  SELECT ?ratio WHERE {
    BIND("P30D"^^xsd:duration / "P1D"^^xsd:dayTimeDuration AS ?ratio)
  }
  ```

  `?ratio` is `30`, typed `xsd:decimal`. Two operands whose values are not
  commensurable (a purely-months value against a purely-seconds one, even
  under matching declared tags) are a typed error, not an arbitrary answer.

### `ADJUST`

`ADJUST(value, timezone)` shifts an `xsd:dateTime`/`xsd:date`/`xsd:time` value to a
given timezone offset, or attaches one to an untimezoned value. The SPARQL 1.2
Query specification's own text carries no `ADJUST` section; the function's one
documented definition is SEP-0002's two-argument signature, which maps onto
XPath and XQuery Functions and Operators §9.6's `fn:adjust-*-to-timezone`
family (the same table `purrdf-xsd` implements for every other SPARQL 1.2
temporal builtin).

```sparql
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
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

## LATERAL (SEP-0006)

The SPARQL 1.2 Query specification's own text carries no `LATERAL` production;
the one documented definition is
[SEP-0006](https://github.com/w3c/sparql-dev/blob/main/SEP/SEP-0006/sep-0006.md)'s,
implemented in Apache Jena 4.7.0, which this section follows. `LATERAL` adds one
production to `GroupGraphPatternSub`, positioned alongside `OPTIONAL`/`MINUS`/
`GRAPH` and left-associative the same way:

```
GroupGraphPatternSub ::= ... | 'LATERAL' GroupGraphPattern
```

Unlike an ordinary join, `LATERAL`'s right-hand side is evaluated ONCE PER
SOLUTION of its left-hand side, with that solution's bindings visible inside —
the same relationship a SQL `LATERAL`/`CROSS APPLY` subquery has to its outer
query. This makes a per-group "top N" query expressible in plain SPARQL: each
subject on the left picks its own smallest label on the right, rather than one
globally-smallest label being computed once and joined against every row.

```sparql
PREFIX : <https://example.org/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT * WHERE {
  ?s :p ?o
  LATERAL {
    SELECT * WHERE { ?s rdfs:label ?label }
    ORDER BY ?label LIMIT 1
  }
}
```

Correlation carries a left-hand binding of ANY RDF 1.2 term kind — including a
blank node or a quoted triple — into the right-hand side, even when the only
occurrence of that variable there is inside an expression (a `FILTER`, a
`BIND`, a `BOUND(?s)` call) rather than as a triple-pattern leaf: `BOUND(?s)`
answers correctly for a left-bound blank node or quoted triple the same as it
does for an IRI or a literal, and such a row is not silently dropped for
lacking a leaf occurrence to carry it.

### The scope restriction

`LATERAL`'s right-hand side may freely REUSE a variable already bound on the
left (that is the whole point — it is how correlation happens), but it may not
INTRODUCE a fresh binding for one: no variable target of a `BIND`, of a
sub-`SELECT`'s `(expr AS ?v)` projection, of a `GROUP BY` aggregate's output,
of an expression-valued `GROUP BY (expr AS ?v)` grouping condition, or a
`VALUES` column, at the right-hand side's own scope level, may collide with a
variable already visible on the left. A bare `GROUP BY ?v` grouping key that
just names an already-bound variable is a USE, not an introduction, and never
collides. The one construct that opens a fresh scope level is a sub-`SELECT`'s
own projection — `OPTIONAL`/`UNION`/`GRAPH`/a nested group/a nested `LATERAL`
are all transparent to it. The SEP's own legal and illegal pair:

```sparql
PREFIX : <https://example.org/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
# Legal: the sub-SELECT projects only ?label, so its own (unprojected) reuse
# of ?s is correlation, not an introduction, and cannot collide.
SELECT * WHERE {
  ?s a :T
  LATERAL { SELECT ?label WHERE { ?s rdfs:label ?label } LIMIT 1 }
}
```

```
# Illegal — refused with a typed ParseError naming ?o: ?o is already bound by
# the left-hand triple pattern, and BIND tries to give it a NEW value at the
# LATERAL right-hand side's own scope level.
SELECT * WHERE {
  ?s ?p ?o
  LATERAL { BIND(123 AS ?o) }
}
```

### Points of disagreement with Jena

Two corners of the restriction disagree with Jena's own `SyntaxVarScope`
check, each pinned by a named test rather than left to drift:

- **Laxer.** A `BIND`/`VALUES`/aggregate introduction confined to a `MINUS`
  right operand is ACCEPTED here at any depth — directly under the `LATERAL`
  keyword or nested under a `SELECT *` sub-select — while Jena rejects it.
  SPARQL §18.2.1 puts `MINUS`-right variables out of scope, and §18.5's
  evaluation explains why: the right operand is used only to build the
  compatibility test against the left operand's rows, then its bindings are
  discarded, so nothing built on top of a `MINUS` — a `SELECT *` projection or
  anything else — can ever observe a value that operand introduced. There is
  nothing observable for the rule to reject. (`SELECT *` over such a
  right-hand side is one route to this same shape, not a separate one.)
- **Stricter.** A `SERVICE ?g { ... }` endpoint variable counts as left-hand
  scope here; Jena omits it. `SERVICE ?g { ... }` with a variable endpoint
  requires `?g` to already be bound in the incoming solution — the endpoint
  IRI is resolved from that binding before the remote call is made — so by
  the time a `LATERAL` right-hand side runs, `?g` already holds an
  observable, per-row value from the left, the same as any other left-bound
  variable. A `BIND`/`VALUES` on the right giving it a NEW value is therefore
  exactly the kind of observable rebinding this rule exists to reject.

### In `UPDATE`

`INSERT`/`DELETE ... WHERE` and `WITH ... WHERE` route through the same
group-graph-pattern grammar a `SELECT`'s `WHERE` clause uses, so `LATERAL` —
scope-checked exactly the same way — is legal there too. A `DELETE WHERE` quad
template is a different grammar production (`TriplesTemplate`, not a
`GroupGraphPattern`) with no group-pattern operators of any kind, so `LATERAL`
there is refused by name rather than misparsed as a subject term.

### `SERVICE` forwarding

A pattern containing a written `LATERAL` clause — anywhere in the forwarded
body, including nested inside another `SERVICE` — is refused only under
`SERVICE SILENT`: there, a remote's rejection of the `LATERAL` extension
would otherwise be swallowed into the identity table, a silent wrong answer
rather than a typed refusal. A plain, non-silent `SERVICE` with a fixed IRI forwards the body
with its `LATERAL { … }` text intact, so the endpoint's actual verdict — an
answer from a `LATERAL`-capable endpoint (`LATERAL` is Jena's own extension,
so a Jena-backed endpoint answers it), or an honest failure from one that does
not implement it — surfaces the same way any other unsupported forwarded
construct's rejection would. A variable-endpoint `SERVICE ?g` is refused only
when nothing supplies `?g`'s binding — nothing in the incoming solution names
an IRI for the remote evaluator to resolve. When an enclosing pattern binds
`?g` — a preceding triple pattern in the same group, or a `LATERAL` left-hand
side's per-row correlation — the endpoint resolves to that IRI before the
remote call is made (the same per-row substitution described under "Points of
disagreement with Jena" above) and the query dispatches normally, the same as
a `SERVICE` with a fixed IRI.

### Per-service context: the `ServiceResolver` seam

`SERVICE` is answered through one trait, `ServiceResolver`, which receives the
whole request as a `ServiceRequest` — endpoint, forwarded query text, the
`SILENT` flag, the executing query's stop signal, and the intermediate-cell
ceiling. Two implementations ship: `HttpRemoteQuerySource`, which builds and
decodes SPARQL Protocol requests over a host-injected `HttpTransport`, and
`InProcessServiceResolver`, which answers from datasets already in memory.

Everything a resolver needs to know about an *individual* service — extra
headers, a credential, a timeout, and what it is permitted to do — lives on the
resolver in a `ServiceCatalog`, keyed by the service IRI. Deliberately not in
the IRI itself: a service IRI is a name, not a credential store, and encoding
context there would put it into the query text, where it is visible to whoever
wrote the query, is serialized into plans and receipts, and travels along with a
nested `SERVICE` body.

A `ServiceCatalog` maps each service IRI to a `ServiceProfile`, and each profile
grants an explicit set of capabilities:

| Capability | Grants |
|---|---|
| `Query` | resolving this service at all |
| `Network` | performing network I/O to resolve it |
| `Credentials` | attaching the profile's credential to the request |

A catalog denies by default: a service with no entry and no explicitly
configured fallback profile is refused. Gating is opt-in — a resolver with no
catalog behaves exactly as it did before catalogs existed, contacting whatever
endpoint it is handed and adding no headers — and configuring a catalog changes
no byte of a request whose profile adds nothing.

Withholding `Network` is what makes an in-process façade *provable* rather than
promised: `InProcessServiceResolver` owns a dataset map and nothing else — no
transport, no socket, no injected callback — so there is no code path through it
that performs I/O. A host can therefore offer `SERVICE`-shaped composition
without `SERVICE`-shaped risk. `ServiceRouter` composes the two, sending some
services in process and the rest to the network, with the routing table rather
than the query text deciding which is which. The catalog is consulted on
*every* resolution, nested ones included, so a `SERVICE` buried inside a
forwarded body cannot reach an endpoint the catalog refuses at the top level.

The policy is applied *before* the transport is called, which is the whole
difference between a gate and an audit log: a denied service never has its
socket opened and then discarded.

A service IRI is an IRI like any other, so it is resolved by the workspace's
single RFC 3986 base layer (see [Base IRIs](../concepts/base-iris.md)) and not by
a rule this seam invented. `SERVICE <sparql>` with no `BASE` in the prologue and
no base passed to the API is the shared `iri-relative-no-base` hard error, raised
while the query is parsed — before any resolver is consulted, and not softened by
`SILENT`, which is a promise about an endpoint that does not answer rather than
about an endpoint IRI that cannot be formed. With a base in scope the resolver is
handed the **absolute** IRI, which is the form a `ServiceCatalog` is keyed on.

### The `SILENT` contract

SPARQL 1.1 §10 says a `SERVICE SILENT` clause whose endpoint cannot be reached
"will be considered to have matched with a single, empty, solution" — the join
identity, so the surrounding query proceeds unchanged. PurRDF keeps that promise
exactly, and confines it to what it is a promise *about*:

| Outcome | `SERVICE` | `SERVICE SILENT` |
|---|---|---|
| The endpoint is unreachable, or its response undecodable | query error | join identity |
| A capability was denied | query error | query error |
| This engine's own governor tripped | truncation | truncation |

The first and last rows are long-standing behaviour: `SILENT` is a statement
about an endpoint the caller does not control, never about the caller's own
budget, so a governor trip reached through a `SERVICE` clause propagates as a
truncation whether or not `SILENT` is written.

The middle row follows from that same principle. A capability denial is a
decision taken on *this* side of the seam — by the host running the engine,
deterministically, before any endpoint was consulted — so it is exactly like a
governor trip and nothing like an unreachable endpoint. Swallowing one would put
the join identity into the surrounding join, making it a no-op, and hand back an
answer that looks complete and is wrong; and because a denial is permanent
rather than transient, it would be wrong identically on every run, so nothing
would ever surface a symptom.

There is deliberately no knob that softens this. A host that genuinely wants a
blocked service to behave like an unreachable one already has an exact way to
say so: return a transport error from its own resolver. That is an honest claim
that the endpoint did not answer, and `SILENT` swallows it under the first row.
Adding a visibility flag would have bought no expressive power, only a second
spelling of an existing one — and with it the possibility of two callers running
the same query over the same data through the same resolver and getting
different answers.

## EXISTS under SEP-0007

SPARQL 1.1/1.2 §18.6 defines `EXISTS`/`NOT EXISTS` through two functions:
`substitute`, which rewrites a pattern's variables against the row being
filtered, and `evalExists`, which asks whether the substituted pattern's
evaluation is non-empty. Read as literal term-rewriting, `substitute` has
defects at several precise points:

- **Variable-only positions.** `substitute` is stated over variable
  POSITIONS in a pattern, with no defined action on a `Bgp`/`Path` leaf term
  or a `GRAPH ?g` name treated as anything other than an expression — so a
  correlated variable occurring only in a triple or graph-name position has
  no substitution to apply.
- **The `MINUS` domain flip.** Rewriting a `MINUS`'s shared variables away as
  constants erases the very columns its domain-join compares the two operands
  on, turning a correlated `MINUS` into an unconditionally disjoint one — the
  opposite of what substituting the row in was supposed to preserve.
- **Blank nodes as variables.** No SPARQL surface syntax can spell a blank
  node or a quoted triple as a rewritten constant, so a correlated blank-node
  or quoted-triple binding has no legal substituted form under literal
  term-rewriting.
- **Disconnected variables.** A correlated variable reachable ONLY through an
  expression position inside a nested `EXISTS` — never through that inner's
  own triple-pattern leaves — has nothing in `substitute`'s pattern-rewriting
  reading to carry it there.
- **The assignment restriction.** §18.6's own text states no rule at all
  about an `EXISTS` body rebinding a variable already in scope on the row it
  filters.

SEP-0007 — a SPARQL-dev proposal, not yet folded into the SPARQL 1.1/1.2 REC
text — repairs the first four by restating `substitute` as `Replace`, a JOIN
rather than a rewrite, and adds the fifth as Part 3, a new restriction.
`purrdf` implements SEP-0007's repairs in full, including Part 3.

### The one definition

```text
exists(X, μ) ⟺ eval(D(G), Replace(PrjMap(X), μ)) is non-empty
```

`Replace` is **Values Insertion**: the current row `μ` joins into `X` as a
one-row `VALUES` table at each `Bgp`/`Path`/`Graph(Var, ·)` site, rather than
being spliced in as syntactic constants — total over every RDF 1.2 term kind,
including blank nodes and quoted triples, because it reaches the dataset
through the same evaluation path real `VALUES` data uses, and it needs no
special case for a variable reachable only through an expression, because the
join reaches every leaf regardless of which position introduced the
correlation. `PrjMap` is **`Project`-boundary narrowing**: `μ` is restricted
to a `Project` node's own variable list before it is joined in below that
node — the one scope boundary the surface language has, and the reason a
sub-`SELECT`'s own projection can legitimately shadow an outer variable.

SEP-0007 states `Replace` in terms of `toMultiSet(μ)` — converting the row to
the multiset unit the join needs. `purrdf`'s algebra has no pattern/query
type split for that conversion to bridge: a solution row already IS that
multiset unit, so `Replace` lands directly below whatever solution modifiers
wrap the leaf it targets, structurally, with nothing extra to construct or
adjudicate.

**The `HAVING` position** is outside SEP-0007's stated coverage — SEP-0007
only specifies `Replace` for `Bgp`/`Path`/`Graph` Values-Insertion sites and
ordinary expression positions, not for a sub-`SELECT`'s `HAVING` clause.
`purrdf` follows the literal `PrjMap` reading: a sub-`SELECT`'s `HAVING`
filters that sub-`SELECT`'s own scope only, exactly like every other
expression inside it — no special correlation channel is invented for it.

### Existential Normal Form

Exactly one bit per row is observed under `EXISTS`: whether the inner
pattern's evaluation is empty. Four rewrite laws replace a node with
something provably emptiness-equivalent but cheaper, applied wherever they
appear at the top of the inner (never inside a `Join`/`Filter`/`Extend`/
`Graph`/`Minus` operand, which read more than emptiness from their child):

- **`LeftJoin(A, B, c) → A`.** `OPTIONAL` emits at least one row per left row
  unconditionally — a padded left row, or one row per compatible match — and
  never removes one, so `LeftJoin(A, B, c)` is empty iff `A` is, regardless
  of `B` or its join condition.
- **`OrderBy(P) → P`.** Sorting is a permutation, never a filter: the sort
  keys are never even evaluated once emptiness is the only question asked.
- **`Distinct(P) → P`, `Reduced(P) → P`.** De-duplication only ever removes
  rows, and only when an earlier row already carried the same value, so `P`
  non-empty always leaves the first row's value present.
- **`Slice(0, len ≥ 1)(P) → P`.** An offset-zero `LIMIT` with room for at
  least one row drops rows only from the end of `P`'s bag, so `P` non-empty
  always leaves row zero inside the window.
- **`Slice(_, Some(0))(P) → false`.** A zero-length `LIMIT` is empty for
  every `P` and every row — the whole `EXISTS` folds to constant `false`
  (`NOT EXISTS` to `true`) without touching the dataset at all.

Every law above is proved emptiness-equivalent over row SETS, never over side
effects, so each fires only when the portion it erases (`B` and the join
condition for `LeftJoin`, the sort keys for `OrderBy`, the whole inner for
the zero-length `Slice` fold) is effect-free — no `SERVICE` call, property
function, or custom/`heldIn`/`rdf:List` function reachable within it — so a
hard error or a remote effect that would have propagated outside the `EXISTS`
still does, rather than vanishing merely for having been written inside one.

### Two strategies, one answer

`purrdf` carries exactly two implementations of the one definition above:

1. **The definition itself** — per-row substitution via `Replace`, wrapped in
   a `Slice{0, Some(1)}` first-witness stop (sound because "does this bag
   have a first row" is exactly what `EXISTS` asks) and backed by a
   **restriction-keyed memo**: the memo key is the outer row restricted to
   the inner's own correlated-variable columns, so `k` distinct restrictions
   across `N` outer rows evaluate the inner exactly `k` times, never `N`.
   Always correct, for any inner pattern.
2. **The memoized probe** — evaluate the inner exactly once, unconstrained,
   index it on the columns it shares with the outer schema, and
   existence-probe each row's `μ` against that index. Correct only where a
   prepare-time admissibility proof shows it equivalent to the definition for
   every row the site can see.

A prepare-time analysis decides, once per site per evaluation, which strategy
answers each `EXISTS`/`NOT EXISTS`. `--explain`'s per-algebra-node charge
ledger reports the decision through three evidence counters:
`exists-probe-answered` (one memoized-probe evaluation),
`exists-definition-answered` (one per-row-definition evaluation), and
`exists-inner-solutions-consumed` (one row the definition path's inner
actually materialized — bounded at 1 per evaluation by the first-witness
stop). An `EXISTS`/`NOT EXISTS` inner's own plan nodes — a `MINUS`, a `Bgp`, a
nested `EXISTS` (however deeply nested), whatever the body contains — carry
their own ledger lines too, decomposed exactly like an ordinary node's rather
than folded into the enclosing `FILTER`/`BIND`, under EITHER strategy: the
memoized probe's one once-per-site evaluation attributes exactly like the
per-row definition's, just charged once rather than once per distinct
restriction. A node that legitimately reads `fuel=0 rows=0` is one
Existential Normal Form erased from the evaluated tree entirely (an
`OPTIONAL`/`ORDER BY`/`DISTINCT`/`LIMIT` wrapper the emptiness-preserving
laws proved transparent — see "Existential Normal Form" above) and therefore
never ran, not a charge that went missing.

**The one named exception**: an `EXISTS`/`NOT EXISTS` inner whose OWN
ENF-normalized top-level shape is a `Project` (a sub-`SELECT`) or a `Union`
(a top-level `{ A } UNION { B }`, not one nested further down) does NOT
attribute — every node inside it, however deep, folds into the enclosing
`FILTER`/`BIND` as `fuel=0 rows=0`, under either strategy, regardless of what
the body itself contains. `Project`/`Union` synthesize their output from more
than one child rather than being a 1:1 structural clone of anything in the
original tree, and reconstructing the same original-address correspondence
for them would need the same wrapper/wrapped `counts_rows` arbitration the
`LATERAL` Values-Insertion machinery already does — deliberately not
attempted, so the correspondence is simply absent for that one site rather
than guessed at. This is not a regression: it is exactly the behavior every
`EXISTS`/`NOT EXISTS` inner had before per-node attribution existed at all,
now confined to this one shape instead of the whole feature.

**The performance characteristic, stated plainly**: an uncorrelated inner, or
a correlated one built only from `Bgp`/`Path`/`Values`/`Graph`/`Join`/
`Union`/`OrderBy`/`Project`/`Distinct`/`Reduced` and `Filter`/`Extend`
expressions that read only certainly-bound columns of the inner, is served by
the probe — one evaluation total, however many outer rows filter through it.
A shape the probe cannot serve — `MINUS`, a restricting `Slice` (any offset
or limit, not only one past the first row), `GROUP BY`, `LATERAL`, a
property-function call, a `SERVICE` call, or a `FILTER`/`BIND` expression
that reads a correlated variable the inner does not certainly bind (for
example, one visible only down an `OPTIONAL` branch) — evaluates per row
through the definition, with the restriction-keyed memo and first-witness
stop above bounding the cost.

### The Part 3 assignment restriction

Neither a `BIND`/a sub-`SELECT`'s `(expr AS ?v)` projection target/a
`GROUP BY (expr AS ?v)` grouping target, nor a `VALUES` column, inside an
`EXISTS`/`NOT EXISTS` body may rebind a variable already in scope on the row
being filtered. `NOT EXISTS` shares the exact same grammar production as
`EXISTS` — there is no separate "`NOT EXISTS`" wording — so the restriction
applies identically to both, and to a nested `EXISTS`'s own body against
whatever its immediately enclosing `EXISTS` already has in scope. A rebinding
confined to a `MINUS` right operand inside the body is exempt at any depth,
the same reasoning `LATERAL`'s own scope restriction applies (§18.2.1: a
`MINUS`-right introduction never escapes it, so it can never observably
rebind the row).

```sparql
PREFIX : <https://example.org/>
# Legal: ?fresh is not in scope on the row FILTER EXISTS is testing, so BIND
# giving it a value is an ordinary fresh binding, not a rebinding.
SELECT ?s WHERE {
  ?s :p ?o .
  FILTER EXISTS { BIND(1 AS ?fresh) }
}
```

```
# Illegal — refused with a typed ParseError naming ?o: ?o is already bound by
# the row FILTER EXISTS is testing, and BIND tries to give it a NEW value at
# the EXISTS body's own scope level.
SELECT ?s WHERE {
  ?s :p ?o .
  FILTER EXISTS { BIND(1 AS ?o) }
}
```

The restriction is SEP-0007's own addition, adopted here because it is what
makes `EXISTS`'s substitution semantics defensible at every site — SPARQL
1.1/1.2's own §18.6 text requires no such rule.

## SHA-3 hashing (SEP-0008)

SPARQL 1.1 §17.4.4 ships five hash built-ins — `MD5`, `SHA1`, `SHA256`,
`SHA384`, `SHA512`. `purrdf` adds the four SHA-3 (FIPS 202 Keccak) functions
[SEP-0008](https://github.com/w3c/sparql-dev/blob/main/SEP/SEP-0008/sep-0008.md)
proposes, on exactly the same call convention. SEP-0008 is a proposal, **not
part of the SPARQL 1.1 or 1.2 recommendation** — these four are a first-party
extension, and [they do not travel](#taking-a-sha-3-query-to-another-engine):

| Call | Digest | Hex characters |
|---|---|---|
| `SHA3-224(string)` | SHA3-224 | 56 |
| `SHA3-256(string)` | SHA3-256 | 64 |
| `SHA3-384(string)` | SHA3-384 | 96 |
| `SHA3-512(string)` | SHA3-512 | 128 |

```sparql
SELECT ?s (SHA3-256(?label) AS ?fingerprint)
WHERE { ?s <http://example.org/label> ?label }
```

### The argument contract

Each function takes **one** argument and hashes the UTF-8 bytes of its
**lexical form**, returning the digest as a lowercase hex `xsd:string` — the
same contract `SHA256` has, so a query can swap one for the other without
changing anything else about the row.

The accepted arguments are a simple literal, an explicitly `xsd:string`-typed
literal, an `rdf:langString`, and an RDF 1.2 `rdf:dirLangString`. A tagged
literal is hashed on its **text only**: `SHA3-256("abc"@en)` and
`SHA3-256("abc")` are the same digest, because the tag is not part of the
lexical form.

Anything else is an expression error, which is SPARQL's ordinary
"this row produces no value" outcome rather than a query failure:

- an **unbound** variable (`SHA3-256(?missing)`),
- an IRI or a blank node,
- a non-string literal (`SHA3-256(7)`).

In a `SELECT` projection an errored call leaves the projected variable
**unbound** on that row; under `FILTER` it makes the constraint false; a
`BIND` of it binds nothing. Wrap the argument in `STR(…)` when you mean
"hash whatever this term looks like" — `SHA3-256(STR(?anything))` — because
`STR` is the function that turns a term into a string, and these do not do it
implicitly.

### The hyphen is part of the name

These are the only built-in names in the language containing a `-`, so the
spacing rule is worth stating outright:

| Text | Reading |
|---|---|
| `SHA3-256(?o)` | the built-in call — one token, hyphen included |
| `SHA3 - 256` | **a parse error**: `SHA3` alone is no function or keyword |
| `STRLEN(SHA3-256(?o)) - 4` | subtraction — the `-` follows `)`, not a word character |

The lexer's `PN_PREFIX` scan admits `-` as a name character, so `SHA3-256`
arrives at the parser as a single word. Whitespace around the hyphen makes it
the subtraction operator again, and because `SHA3` is not itself a function,
the spaced form fails loudly rather than meaning something else. Names are
case-insensitive like every other built-in (`sha3-256` is the same call).

SEP-0008's own text spells the four functions with an **underscore**
(`sha3_256`), so `SHA3_256(?o)` is accepted as an alias for `SHA3-256(?o)`:
a query copied out of the proposal parses. The alias is an input spelling
only — the algebra has one function per digest size, so a serialized query
always carries the canonical hyphenated name and stays byte-deterministic
whichever spelling was typed.

### Taking a SHA-3 query to another engine

Expect a **parse error**. `SHA3-256` is a built-in name here, but the SPARQL
1.1/1.2 grammar offers exactly two ways to name a function — a keyword from its
own built-in list, or a `FunctionCall ::= iri ArgList` whose `iri` is an
`IRIREF` or a prefixed name. On an engine that has not adopted SEP-0008,
`SHA3-256` is not in the built-in list, and a bare word with no colon is not an
`iri` either, so `SHA3-256(?o)` has no parse at all. The underscored
`SHA3_256(?o)` spelling fails for the same reason. The failure is therefore
loud and immediate rather than a quietly unbound column.

The portable substitute is one of the SPARQL 1.1 §17.4.4 hashes. `SHA256` takes
the same single argument and returns the same lowercase-hex `xsd:string`, so
swapping the name is the whole edit — it changes the digest, and nothing else
about the query. Reach for the SHA-3 names when you control the engine and want
the Keccak construction specifically; reach for `SHA256` when the query text has
to run anywhere.

### Reaching it from other hosts

There is nothing to configure: unlike the extension-function and
custom-aggregate seams, these are built-ins, so every surface that takes
query text has them — `purrdf query`, `Store.query` / `MutableDataset.query`
in Python, `Dataset.query` / `QueryEngine.select` in WebAssembly, and
`purrdf_query` / `purrdf_query_json` over the C ABI.

## Quad templates: `CONSTRUCT` into named graphs

A SPARQL 1.1 `CONSTRUCT` template is a set of triples, and the result is one
graph. `purrdf` also accepts a **quad template**, so a template may name the
graph each statement lands in, and a single result may span several named
graphs.

### Provenance: a `purrdf` extension, not a SPARQL 1.2 feature

**SPARQL 1.2 does not define the quad template.** Neither the 1.1 nor the 1.2
grammar admits a `GRAPH` block inside a `CONSTRUCT` template
(`ConstructTemplate ::= '{' ConstructTriples? '}'`, and `ConstructTriples` is
triples only), and neither defines the `CONSTRUCT GRAPH …` shorthand. Both
spellings documented below are first-party extensions this engine ships.
Producing quads from a `CONSTRUCT` is a long-running request in the SPARQL
community's proposal process, and other engines — Jena and Stardog among them —
already ship a form of it, but no standardized spelling exists, so the one
described here is `purrdf`'s.

Declaring `VERSION "1.2"` does not subtract the extension: a version
declaration selects semantics, not a feature whitelist. See
[The `VERSION` declaration](#the-version-declaration).

### Taking one of these queries to another engine

Expect a **parse error**, not a different answer. An engine without the
extension rejects the `GRAPH` keyword as soon as it meets it inside a
`CONSTRUCT` template, because its grammar has no production that admits one
there — the query fails before evaluation, so there is no risk of silently
getting the wrong graphs. An engine that ships its own form of the feature may
accept only one of the two spellings, since neither is standardized.

Two portable rewrites:

- **If the result is going into a store**, use SPARQL 1.1 Update rather than
  `CONSTRUCT`. Update's template has always been a quad template, so
  `INSERT { GRAPH … { … } } WHERE { … }` is standard, universally implemented,
  and gives the same per-solution graph targeting — including a graph name
  bound per row:

  ```sparql
  PREFIX ex: <http://example.org/>
  INSERT { GRAPH ?g { ?s ex:friend ?o } }
  WHERE  { GRAPH ?g { ?s ex:knows ?o } }
  ```

- **If the result must come back as a document**, issue one ordinary
  triple-producing `CONSTRUCT` per target graph and assemble the dataset on the
  client. This costs a round trip per graph and cannot express a graph name
  computed per solution row, which is the gap the quad template closes.

Queries that stay inside the triple form are unaffected in either direction: a
template with no `GRAPH` slot is an ordinary SPARQL 1.1 `CONSTRUCT` here and
emits byte-identically, so only the templates that actually name a graph are
the ones that will not travel.

### `GRAPH` blocks inside the template

```sparql
PREFIX ex: <http://example.org/>
CONSTRUCT { GRAPH ex:derived { ?s ex:friend ?o } }
WHERE { ?s ex:knows ?o }
```

### A variable graph name

The graph slot takes a variable as well as an IRI, so the graph a statement
lands in can be decided per solution row:

```sparql
PREFIX ex: <http://example.org/>
CONSTRUCT { GRAPH ?g { ?s ex:friend ?o } }
WHERE { GRAPH ?g { ?s ex:knows ?o } }
```

### Several graphs, and mixed default-graph triples

One template may write into more than one graph, and may mix graph-scoped
quads with unscoped triples that land in the default graph:

```sparql
PREFIX ex: <http://example.org/>
CONSTRUCT {
  ?s ex:seen true .
  GRAPH ex:people  { ?s ex:friend ?o }
  GRAPH ex:reverse { ?o ex:friend ?s }
}
WHERE { ?s ex:knows ?o }
```

### The whole-template shorthand

`CONSTRUCT GRAPH <iri> { … }` scopes the entire template to one graph without
a `GRAPH` block around it. It also works with the short form
(`CONSTRUCT GRAPH <iri> WHERE { … }`), and it takes a variable
(`CONSTRUCT GRAPH ?g { … }`), a prefixed name, or a `BASE`-relative IRI:

```sparql
PREFIX ex: <http://example.org/>
CONSTRUCT GRAPH ex:derived { ?s ex:friend ?o }
WHERE { ?s ex:knows ?o }
```

The shorthand is a **default, not an override**: it supplies the graph for
every template slot that did not name one itself, so an inner `GRAPH` block
still wins over it. `CONSTRUCT GRAPH { … }` with no name is a syntax error
rather than a silently unscoped template.

### Skip semantics

SPARQL §16.2 already skips a template statement whose variables are unbound or
whose instantiation is ill-formed. Ill-formed means "not a legal RDF 1.2
statement", position by position:

* the **subject** is an IRI or a blank node — a literal is illegal there, and so
  is a triple term (a quoted triple is a value; an asserted statement is made
  about a reifier, not about the quoted triple itself). Both are reachable from
  ordinary data: over RDF 1.2 input, `CONSTRUCT { ?o ?p ?s } WHERE { ?s ?p ?o }`
  binds `?o` to a triple term as readily as to a literal;
* the **predicate** is an IRI, and nothing else;
* the **object** may be any term, but when it is a triple term that triple
  term's own components carry the same rules recursively (its subject must not
  be a literal, its predicate must be an IRI).

The graph slot follows the same rule, and this is worth being explicit about:

**An unresolvable graph name skips its statement.** It is not an error, and it
is *not* a fallback to the default graph. The graph slot is resolved first, so
a statement whose graph name is an unbound variable, or is bound to anything
that is not an IRI (a literal, a blank node, a triple term), is not
instantiated at all — it mints no blank-node labels either, so the rest of the
result is exactly what it would be if that quad were absent from the template.

The skip is **per statement**, not per row: a sibling template quad whose own
graph slot resolves is still emitted for the same solution.

### Which output formats can carry the result

A named graph needs a syntax with somewhere to put a graph name. Six of the
nine RDF syntaxes have one:

| Carries named graphs | Does not |
|---|---|
| TriG, N-Quads, TriX, HexTuples, JSON-LD, YAML-LD | Turtle, N-Triples, RDF/XML |

The single-graph serializers **drop** graph-scoped statements — they do not
fold them into the default graph — so writing a graph-carrying result to one
of them would produce a well-formed document silently missing exactly what the
query asked for. No host lets that pass unsignalled. The three hosts whose
egress is a *document* refuse outright, each naming the graphs it would have
dropped, the format it was asked for, and the quad-capable alternatives; the C
ABI, whose egress is the dataset itself, reports the loss as a count instead:

- **CLI** — `purrdf query … --results-format turtle` exits **2** (a usage
  refusal, distinct from an evaluation error) and prints the refusal on
  stderr, pointing at
  `--results-format trig/nquads/trix/hextuples/jsonld/yamlld`.
- **Python** — the result of a graph-carrying `CONSTRUCT` is a `QueryQuads`
  (whose members are `Quad`s with a live `graph_name`) rather than a
  `QueryTriples`. `QueryQuads.serialize(RdfFormat.TURTLE)` raises
  `ValueError`, naming `RdfFormat.N_QUADS/TRIG/TRIX/HEXTUPLES/JSON_LD/YAML_LD`.
- **WebAssembly** — an explicit `serialize("turtle")` / `queryRaw(…, {format:
  "turtle"})` **throws**, with the same sentence and the same alternatives.
  When the caller names NO format the default widens from `turtle` to `trig`
  instead of throwing, because there was no request to contradict and an empty
  document would be the wrong answer; a result with no named graph still gets
  `turtle`, byte for byte.
- **C ABI** — the shape is different, and deliberately so. `purrdf_query`
  hands back a `PurrdfDataset` handle, which is the frozen IR itself and has
  somewhere to put a graph name, so the graphs are never lost at the query
  boundary. Serializing that handle is a separate call, and
  `purrdf_serialize` to a single-graph media type succeeds while reporting
  what it discarded through the `out_named_graph_rows_dropped` out-parameter —
  a count rather than an exception, because that is the signal C has. The
  parameter is independently nullable; a caller that passes null for it has
  asked not to be told, so read it — or serialize to a quad-capable media type
  — whenever the result may carry graphs.

  The convenience path, `purrdf_query_json`, needs neither: it renders a
  `CONSTRUCT`/`DESCRIBE` result as **N-Quads** inside its `{"graph": "..."}`
  envelope, so the graph names, the base quads and the RDF 1.2 statement layer
  all survive and there is no loss to report. That member is `purrdf`'s own
  envelope rather than a caller-selected RDF syntax, which is why it widens the
  way the WebAssembly no-format default does instead of refusing the way an
  explicit `serialize("turtle")` does. A default-graph-only result is
  byte-identical to the N-Triples the member used to hold — an N-Quads line with
  no graph term is the N-Triples line.

A result carrying only default-graph statements is untouched everywhere: every
SPARQL 1.1 `CONSTRUCT` and every `DESCRIBE` serializes to Turtle exactly as
before. A **mixed** result is refused as a whole rather than half-emitted,
because emitting the default-graph half would report a partial answer as a
complete one.

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

## Aggregate determinism: row order, `DISTINCT`, and `GROUP_CONCAT`

SPARQL 1.1/1.2 leave several corners of `GROUP BY`/aggregate evaluation
intentionally underspecified — a conforming engine may pick any answer within
the spec's envelope. `purrdf-sparql-eval` picks ONE deterministic answer for
each and documents it here (mirrored in `purrdf_sparql_eval::modifier`'s crate
docs), so "what does `GROUP_CONCAT` return" has a single, testable meaning
rather than "any order the engine happened to produce."

**Row and group order (§18.6.1 "Aggregate Algebra").** `GROUP BY` partitions
the inner solution sequence into groups; this crate keeps groups in
**first-seen order** (the order each group's key first appears in the inner
solution sequence) and keeps each group's own rows in **inner-operator
order** (the order the ungrouped input produced them). Every order-sensitive
fold — `GROUP_CONCAT`'s concatenation, `SAMPLE`'s "first value wins", a
custom aggregate's `OrderDependent` fold — folds over rows in exactly that
order.

"Inner-operator order" is REPRODUCIBLE (the same query against the same
dataset yields the same order every time) but is not, by itself, a
documented invariant a query text can rely on for a plain triple-pattern
scan: a bare BGP's solution order follows this store's internal index
layout (currently sorted by interned term id along the index the planner
picks), which is an implementation detail, not a promise. The one row order
a query CAN rely on is an explicit `ORDER BY`: when the aggregate's
immediate input is (or is fed by) an `ORDER BY`-sorted solution sequence —
for example a subquery `{ SELECT ?v WHERE { ... } ORDER BY ?key }` feeding
an outer aggregate — "inner-operator order" is exactly that `ORDER BY`'s
SPARQL total order (§15.1), which the specification itself fixes.

**`DISTINCT` (inside an aggregate call).** Per §18.6.1's `Aggregation`
definition, `DISTINCT` folds `Dedup(M(Ψ))` rather than `M(Ψ)` — an
order-preserving, duplicate-free view whose relative order of first
occurrences is preserved. This crate's dedup keeps the FIRST occurrence (in
the row order above) of an equal-by-value tuple; every later occurrence never
reaches the fold's `step`.

### `GROUP_CONCAT` ordering

§18.6.1.7 defines `GroupConcat` as concatenating the sequence's elements with
`sep` between them, but explicitly leaves the sequence's own order
unspecified ("The order of the strings is not specified") — exactly the
freedom the paragraphs above pin down. This crate concatenates in the row
order stated above: **groups first-seen, rows in inner-operator order,
`DISTINCT` keeping the first occurrence** — producing a plain `xsd:string` of
the lexical forms joined by `sep` (default `" "` per §18.6.1.7, absent an
explicit `SEPARATOR`). A term with no lexical form (a blank node or a triple
term) poisons the fold to unbound, the same reading `SUM`/`AVG` use for a
non-numeric running total.

Because a plain BGP's own scan order is an implementation detail rather than
a documented guarantee (see above), a `GROUP_CONCAT` fixture that wants to
demonstrate the determinism reading with an exact-string pin cannot rest the
proof on scanning a triple pattern directly — that would pin an incidental
property of the current index layout, not the specification-backed ordering
this section documents. This project's own conformance fixture,
`crates/sparql-conformance/suite/purrdf-extend/group-concat-order.rq`, drives
its row order from an `ORDER BY DESC(?s)` subquery feeding the outer
`GROUP_CONCAT` — anchoring the pin to SPARQL's own `ORDER BY` total order
(§15.1) over distinct IRIs, and using `DESC` rather than `ASC` so a
regression that silently ignored the subquery's `ORDER BY` (and fell back to
the store's incidental scan order) would produce a detectably different,
wrong string instead of coincidentally passing.

## Extending the evaluator: custom aggregates

Beyond the SPARQL 1.1 built-in aggregates (`COUNT`/`SUM`/`AVG`/`MIN`/`MAX`/
`SAMPLE`/`GROUP_CONCAT`), a Rust host may register additional `GROUP BY`
reductions and reach them from query text as `AGG(<iri>, [DISTINCT] arg, arg, …)`
— the normative positional spelling (a deliberate divergence from Jena
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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        // Recover `other`'s real state through `into_any` rather than
        // re-deriving it from `finish()`'s lexical form: `finish()` is a
        // lossy, string-typed answer, and re-parsing it is exactly the
        // pattern this crate's own `agg_fn`/`stat_agg` merges avoid (see
        // their "Real merges via `into_any`" docs). A running total happens
        // to survive that round trip losslessly, but the type-recovered
        // merge below is the pattern to copy for a fold whose finished
        // answer is NOT itself sufficient mergeable state.
        let other = other.into_any().downcast::<Self>().map_err(|_| {
            EvalError::function(
                "combine received a partial accumulator of a different concrete type",
            )
        })?;
        self.sum += other.sum;
        Ok(())
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
        aggregates: &registry,
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
scalar aggregate parameter) to any custom aggregate: `AGG(<{NS}PERCENTILE>, ?x;
P=0.95)`, `AGG(<{NS}TOPK>, ?x; K=3)`. `NAME` is matched case-insensitively and
stored upper-cased; `value` is any SPARQL literal — the full numeric tower
including its signed forms (`Q=-1`, `P=+0.5`), the boolean literals (`B=true`),
and strings — so a numeric scalarval keeps its natural datatype. Unlike a
positional argument, a scalarval is evaluated
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
- **`PERCENTILE`** takes a named scalarval, `AGG(<{NS}PERCENTILE>, ?x; P=0.95)`:
  `P` is resolved once for the whole aggregation (never per row — see "Named
  scalar-value parameters" above); a `P` outside `[0, 1]` poisons to unbound,
  the same "poison, don't abort" discipline every numeric member uses. A
  missing or non-numeric `P` is refused at prepare time.
- **`MODE`** works over any term kind, counted by RDF term identity (not value
  equality — `"5"^^xsd:integer` and `"05"^^xsd:integer` count as two different
  terms). A tie is broken toward the smallest term under the same total order
  `MIN`/`ORDER BY` use.
- **`FIRST`/`LAST`** work over any term kind, in input row order.
- **`TOPK`** takes a named scalarval, `AGG(<{NS}TOPK>, ?x; K=3)`: `K` must be a
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

PurRDF's first-party **statistical set** is different, precisely because it is
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
      "PREFIX ex: <https://ex.example/> "
      "SELECT (AGG(<https://ex.example/agg#MEDIAN>, ?v) AS ?m) "
      "WHERE { ?s ex:value ?v }",
      aggregate_namespace="https://ex.example/agg#",
  )
  ```

- **CLI** (`purrdf query` / `purrdf update`): the `--aggregate-namespace IRI`
  flag.

  ```sh
  purrdf query --data data.ttl --aggregate-namespace 'https://ex.example/agg#' \
    'SELECT (AGG(<https://ex.example/agg#MEDIAN>, ?v) AS ?m) WHERE { ?s ?p ?v }'
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

The **entailment-aware query lane** (`query_with_entailment`/
`query_with_entailment_governed`, and every Python/CLI/WebAssembly/C wrapper
around the governed entry point) combines with `aggregate_namespace` exactly
as the raw-view lane does: both take the engine's `QueryOptions`, threaded
through the closure query's parse and its evaluation, so `AGG(<{NS}MEDIAN>, …)`
resolves over an entailed closure the same way it resolves over a raw view.

```sh
purrdf query --data ent.ttl --entailment rdfs \
  --aggregate-namespace 'https://ex.example/agg#' \
  'SELECT (AGG(<https://ex.example/agg#MEDIAN>, ?w) AS ?m)
   WHERE { ?x <http://example.org/measure> ?w }'
```

`purrdf.Store.query_entailment_governed(..., aggregate_namespace=NS)` answers
the same query the same way, and the WebAssembly and C-ABI governed
entailment entry points take the same parameter.

One structural limit applies everywhere the statistical set is reachable:
SPARQL UPDATE's grammar admits an aggregate only inside a nested
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

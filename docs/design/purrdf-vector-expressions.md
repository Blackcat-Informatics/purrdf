<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Proposed exact vector expressions and centroid aggregates

**Status: proposed design; no runtime vector aggregate or value-origin SPARQL
relation is introduced by this document.** It specifies a complete path from
embedding values through an exact aggregate to a native nearest-neighbour
producer. API names used for proposed types are schematic.

The decisions follow the existing [embedding arithmetic contract](purrdf-embedding-knn.md),
[HNSW design](purrdf-hnsw.md), [physical ownership rules](purrdf-physical-shapes.md)
and [retained view ownership](purrdf-shared-view-carriers.md). Distance arithmetic
keeps its present law. Exact aggregation is a separate mathematical operation.

## 1. Current boundaries

The following are observations about the existing implementation, not proposed
capabilities:

| Existing home | Current behavior |
|---|---|
| `purrdf_core::TermValue`, `crates/rdf-core/src/ir/term.rs` | RDF IRIs, blank nodes, literals and triple terms; no vector term variant |
| `purrdf_cdt::{CdtValue,CdtTerm}`, `crates/cdt/src/{value,term,render}.rs` | Bounded ordered list literals, verbatim authored component literals and one canonical list writer |
| `purrdf_xsd::exact::Rational` | Exact integers, decimals and explicit finite IEEE imports; exact rational arithmetic and correctly rounded float projection |
| `sparql-eval/src/agg_fn.rs` | Caller-IRI custom aggregates with admitted lifecycle hooks and `step`/`combine`/`finish` |
| `sparql-eval/src/modifier.rs::fold_numeric_admitted` | Scalar SPARQL `SUM`/`AVG` preserve their numeric-add chain; regrouping rounded additions changes that expression |
| `sparql-eval/src/knn/invocation.rs` | The shared property-function protocol resolves a seed term to `query_row`; an absent row retains its existing no-result behavior |
| `hnsw/src/lib.rs::{search_vector,search_vector_work}` | The low-level index already accepts an arbitrary `f64` query vector; its SPARQL relation still enters through row origins |
| `retrieval/src/embedding.rs` | An existing caller-datatype transport writes exact `f32` bit patterns; it does not represent an exact rational centroid |

In particular, `distance::Exact` fixes the order of correctly rounded IEEE
operations. It does not make floating-point addition associative or turn an
arbitrary reduction tree into an exact mathematical sum.

## 2. Representation and caller configuration

Keep the RDF term model unchanged. The external carrier is an existing
`cdt:List` literal. It becomes a vector only when a caller-registered vector
expression, aggregate or producer interprets it under an explicit profile.
Ordinary CDT lists keep their present semantics.

A proposed native `VectorProfile` binds:

- A required positive dimension and the numeric law below.
- The selected embedding space and its existing family/space contract, when
  reading stored embeddings or supplying a search origin.
- Caller-supplied expression, aggregate and relation IRIs, with no fabricated
  namespace or default producer.
- Native work, dimension, lexical and magnitude admission sufficient for the
  intended query. The profile is not a semantic Cargo feature.

A native `VectorValue` is a validated, admitted view or computed value over
exact components. It is not another RDF term kind. Its parser delegates list
grammar to `purrdf_cdt`, then requires exactly the profile's dimension of scalar
numeric literals. Nested composites, maps, nulls, language literals and other
component kinds refuse with their original component position and cause.

The profile identifies coordinate interpretation and participates in existing
registry/plan identity. It cannot prove an authored list came from a particular
model: coordinates alone carry no model provenance. A stored-row expression
obtains that provenance from its registered `EmbeddingSpace`. Accepting an
authored coordinate list explicitly interprets it in the configured space.
Dimension equality must never be described as proof of model compatibility.

### Exact component spelling

The numeric value of every accepted component is a native
`purrdf_xsd::exact::Rational`. Integers and decimals enter exactly. Finite
`xsd:float` and `xsd:double` values enter through the existing explicit
`Rational::from_f32`/`from_f64` conversion, preserving the value of their binary
representation. Neither decimal reinterpretation nor scale quantization occurs.
Nonfinite floats refuse. Positive and negative floating zero denote the same
rational zero under this numeric law.

An `owl:rational` component is read through the native rational parser. Its
canonical lexical form is the reduced `numerator/denominator`, with a positive
denominator always written: `1/3`, `-5/1`, `0/1`. A zero denominator refuses.
Computed components use that canonical form and the existing standard
`owl:rational` datatype. The existing CDT writer then emits their enclosing
list, with explicit typed literals and commas without added whitespace.

For example, the **inner CDT lexical value** of a two-dimensional result is:

```text
["1/3"^^<http://www.w3.org/2002/07/owl#rational>,"-5/1"^^<http://www.w3.org/2002/07/owl#rational>]
```

The outer RDF term carries the existing `purrdf_cdt::CDT_LIST` datatype, with
normal RDF string escaping around that lexical value. No new datatype IRI is
minted. Library implementation uses the existing vocabulary constants and
native rational/CDT writers; it does not reproduce the spelling in a formatter.
Reading a vector never rewrites an authored term. Computed vectors have one
canonical spelling, independent of input order or worker partition.

## 3. Exact sum and mean

For a profile dimension `d`, accumulator state is:

```text
(profile, n, sums[0 .. d])
```

Each sum is an exact rational and `n` is an exact accepted-vector count. A step
first validates the entire input's shape and component types, then adds each
component and increments the count. Count arithmetic must not wrap.

Combining states requires the same profile and dimension, adds corresponding
sums and adds counts. It merges the full native state, not a finished mean.
For complete inputs, this operation is associative and commutative in exact
arithmetic. The native aggregate factory declares `Stable` and `Commutative`
through the existing custom-aggregate seam. Worker count, row order and chunk
boundaries cannot change a successfully completed result.

`SUM` returns the sum vector. `MEAN` returns each `sums[i] / n` as an exact
rational. For example, the mean of `0`, `0`, and `1` is `1/3`. A finite fixed
scale cannot represent that result exactly, and a decimal division policy or
per-step rounding must not silently substitute for this law.

The dimension is known from the profile even for an empty group:

| Input | Vector sum | Vector mean |
|---|---|---|
| No accepted vectors | Exact zero vector of dimension `d` | Unbound, through the existing aggregate empty-result channel |
| One vector | Its exact component values | Its exact component values |
| Mixed dimensions or invalid bound component | Typed refusal | Typed refusal |

Ordinary truly unbound argument evaluation follows the existing `AGG` wrapper's
behavior. The vector accumulator counts only complete vectors delivered to
`step`. This alone does not establish the refusal contract: the generic
`AggregateAccumulator` interface explicitly omits rows whose argument evaluation
failed, and `BIND` can turn an ordinary expression error into an unbound value.

Every new vector-producing expression and vector aggregate argument adapter
must therefore carry a bound input's dimension, type, nonfinite or physical
refusal through expression evaluation, `BIND` and aggregate argument evaluation
using the existing hard native refusal path. If an existing adapter cannot
preserve that distinction, an explicit typed propagation seam is required there
before support can be claimed. Such a refusal must abort the vector operation
with its original cause; returning ordinary `None`/unbound would let the row
disappear before `step` and produce a centroid over a different group. This
requirement does not change ordinary SPARQL handling of genuinely unbound
variables or unrelated expression errors.

`DISTINCT` keeps the existing aggregate term semantics;
this proposal does not silently replace RDF-term identity with coordinate
equality. Native vector-producing expressions emit canonical vectors before
they reach that boundary.

Register native vector-sum and vector-mean factories under the caller's IRIs
and invoke them using existing `AGG(<iri>, expression)` syntax. Do not overload
SPARQL scalar `SUM`/`AVG`, add a `MEAN` keyword, or rely on a host to implement
an independent arithmetic body.

### Refusal and ownership

Dimension mismatch is a hard typed vector refusal, not a truncated `zip`, an
empty producer answer or a partially accumulated centroid. It carries the
expected and actual dimensions, profile context, original argument/component
position and retained original input/cause. Profile mismatch during state
combination also refuses. A malformed number, nonfinite input, invalid rational
denominator, unsupported component, count overflow and resource exhaustion
remain distinct native causes.

Use the evaluator's existing `WorkspaceCapability`, lexical/numeric operation
frames and immutable retained carriers. State, partial states, component limbs,
canonical output and their shared controls retain their original admissions
until they die or transfer to the completed result. `combine` cannot clone an
unadmitted vector, detach a lease or finalize another partial mean as a row.
Failed updates publish no incomplete aggregate; a stopped group cannot label
its prefix as a completed centroid.

The current rational cost descriptors are **not physical admission**. The
native rational home explicitly makes that distinction. Implementation must
provide original physical admission for rational parsing, conversion,
arithmetic, merging and output preparation at `purrdf_xsd::exact::Rational` and
its existing bigint/cost homes before exposing bounded vector aggregation.
Calling an allocating rational operation after merely checking its cost does
not satisfy this design. No second rational implementation belongs in the
evaluator, and no generic ownership framework is required.

## 4. A real value origin for nearest-neighbour search

The aggregate's literal must reach the actual producer. Giving it a temporary
IRI, inserting a synthetic matrix row, or letting `row_of` return `None` does
not implement a centroid search.

Extend the shared native invocation/ranking protocol with a typed distinction
between stored-row and admitted vector origins. Preserve the old row-origin
relation and its unknown-seed behavior. A caller explicitly registers a
vector-origin relation/profile, so a CDT-list term that already names a stored
row cannot acquire a different meaning by a fallback guess. Both adapters use
the same native ranking protocol and producer implementations.

The vector route performs these operations in order:

1. Validate the profile, complete dimension, numeric components and original
   workspace ownership before opening a search.
2. Keep the exact rational vector and original RDF query term available.
3. Project each component **once** into the native `f64` type with the rational
   conversion's round-to-nearest, ties-to-even law. Refuse an unrepresentable
   finite projection rather than clamping or admitting infinity. No round via
   `f32`, and no implicit normalization or truncation, is permitted.
4. Bind that admitted coordinate buffer to the selected producer's existing
   metric/arithmetic kernel. Compute its query norm through that same kernel
   when required. A zero centroid is valid for a metric that accepts zero;
   a norm-dividing metric's zero-norm refusal remains explicit.
5. Search and emit rows under the existing count, bound-position filtering,
   work, rank and evidence contracts. Echo the **original query term** in the
   query position, not a rewritten projected literal that fails bound equality.

The result is an exact retained centroid followed by a declared, deterministic
projection. Ranking under the existing double-precision metric is ranking from that
projected point; it is not a claim to compare distances in rational arithmetic.
The origin/projection law and producer profile must remain visible to plan
identity and result evidence. A rounded presentation cannot replace the exact
aggregate value.

The exact scan must accept the new bound point through its current distance
kernel. The HNSW adapter must reach its existing arbitrary-vector search and
binding computation through an admitted entry, rather than falling back to
the resident unpriced `search_vector` API in a bounded query. HNSW retains its
approximation declaration, search parameters and stable row tie-break; a new
origin does not turn it into an exhaustive scan. The existing retrieval
`f32`-bit transport and its byte identity stay unchanged. Any adapter into that
transport must declare its separate projection explicitly.

### Proposed query shape

These IRIs are caller registration examples, not built-in vocabulary or a
claim that the following query works in the current runtime. The registered
value expression reads the selected space's stored embedding for a passage;
the vector-origin relation consumes the aggregate result directly.

```sparql
SELECT ?hit ?distance WHERE {
  {
    SELECT (AGG(<https://example.org/vector/mean>, ?vector) AS ?centroid)
    WHERE {
      ?passage <https://example.org/matches> true .
      BIND(<https://example.org/vector/value>(?passage) AS ?vector)
    }
  }
  ?hit <https://example.org/vector/nearest-from-value>
       (?centroid 5 ?distance) .
}
```

## 5. Implementation and acceptance boundary

The design requires these concrete changes before runtime support can be
claimed. It authorizes no alternate evaluator or metric.

| Home | Required implementation seam |
|---|---|
| `purrdf-core`, above its existing CDT/XSD dependencies | One profile-bound vector view/value and CDT adapter; preserve `TermValue` variants |
| `purrdf-xsd::exact`, bigint and cost homes | Admitted rational input, operations and canonical/float output; original typed failures |
| `sparql-eval/src/{agg_fn,modifier,native_numeric,composite_value}.rs` | Native caller-IRI factories, complete-state combination and original workspace transfers |
| `sparql-eval/src/knn/{invocation,mod}.rs` | Actual typed value-origin admission, decoding, projection, query-term echo and exact scan dispatch |
| `hnsw/src/{lib,relation}.rs` | Admitted arbitrary-vector origin through the existing query binding and traversal |
| Existing parser/registry identity and evidence homes | Preserve scalar grammar, price native work and identify vector/profile/projection semantics without a vocabulary default |

Focused native acceptance must establish:

- Canonical CDT/rational output, including `1/3`, exact finite binary input
  conversion and the absence of intermediate rounding.
- Identical sum/mean values and bytes under row permutations, different chunk
  boundaries and full-state combinations; duplicate rows keep bag multiplicity.
- Explicit empty-group behavior, mismatched dimension/profile, nonnumeric and
  nonfinite input, zero denominator and projection-overflow refusals, each with
  a valid neighboring case where applicable.
- An end-to-end vector-expression refusal through `BIND` into `AGG`: one bound
  vector with a mismatched dimension, invalid type, nonfinite component or
  physical admission failure retains its typed hard refusal instead of becoming
  unbound and disappearing before `step`. A neighboring truly unbound argument
  retains ordinary SPARQL handling. No centroid over just the surviving rows may
  be published as the requested complete aggregate.
- Original admission across parsing, limb growth, partial states, combination,
  projection, output retention and cleanup, including refusal before unpriced
  growth. No complete result from a withheld group.
- A query that reads actual stored embeddings, computes a non-stored centroid,
  and reaches the native nearest-neighbour producer from that value. Exercise
  the exact scan and the existing HNSW path without changing either rank claim.
- Original query-term echo and bound-position filtering, unchanged stored-row
  mode, zero-vector metric behavior, and caller registration without fabricated
  IRIs or semantic Cargo features.

These are implementation acceptance obligations, not tests reported as run.
Ordinary semantics remain native; this design adds no foreign-platform matrix.
It selects no model, vector dimension, coordinate quantization, hidden
normalization or caller vocabulary by default.

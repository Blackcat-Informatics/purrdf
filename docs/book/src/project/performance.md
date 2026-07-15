<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Performance

PurRDF's performance is **measured, never asserted**. No number in the
project's documentation is a guarantee: benchmarks are report-only,
timing-sensitive, and vary by host, CPU, allocator, and build flags. Treat
every figure as a host-dependent illustration you reproduce locally — not a
promise of "N× faster." The methodology and a representative results table
live in
[`docs/BENCHMARKS.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/BENCHMARKS.md).

## Fast by construction

The engine-level speed comes from the IR design
([The Interned Dataset IR](../concepts/interned-dataset.md)): every term
stored once in a string arena addressed by copyable `NonZeroU32` ids,
fixed-key `ahash` everywhere hot, frozen `Box<[QuadRow]>` quad tables with
lazy ordinal permutation indexes (~4 bytes/quad per axis), and evaluation in
`TermId` space so solution comparison is an integer compare.

Crucially, the *layout itself* was chosen by benchmark: the
`crates/rdf-core/benches/ir_layout.rs` criterion suite measures
array-of-structs vs. struct-of-arrays vs. predicate-adjacency layouts on
allocation counts, high-water memory, and end-to-end latency — and the
shipped layout is whichever wins.

## The two benchmark layers

| Layer | What it measures | How to run |
| --- | --- | --- |
| **Rust criterion suites** | Native engine hot paths — IR layout, copy-on-write mutation, pack index alternatives, codecs, SPARQL lexing/evaluation/planning, SHACL validation, entailment chase, GTS authoring, IRI parsing. | `make bench` |
| **Python compat harness** | `purrdf.compat.rdflib` (the native-backed drop-in) vs. the real rdflib 7.x on parse, serialize, SPARQL, and triple-pattern iteration, over a deterministic `example.org` corpus. | `make bench-python` |

Both layers are report-only: they are never part of `make check`, and no test
gate asserts a speedup.

## The discipline for changes

Any change claiming a performance win must **extend the criterion benches**
rather than asserting the speedup in prose. Where a planner or algorithm
choice matters for correctness-adjacent behavior, it is gated by
*deterministic* tests instead of timings — for example, the cost-based BGP
planner's win over the retired structural heuristic is asserted by unit tests
that count real intermediate rows, and by a differential corpus test, while
the criterion bench merely watches for regressions.

`NativeSparqlEngine::explain_query` exposes the chosen BGP join order so
planner decisions can be audited without running the query
([SPARQL: Querying](../sparql/querying.md)).

## Reproducing locally

```sh
make bench                              # the default criterion set
cargo bench -p purrdf-iri --bench parse # a single package's bench
cargo bench -p purrdf-core --bench pack_index_compare # pack index experiment
make bench-python                       # the rdflib comparison harness
```

Benchmark on a quiet machine, and compare like with like: allocator, CPU
scaling, and build flags all move the numbers.

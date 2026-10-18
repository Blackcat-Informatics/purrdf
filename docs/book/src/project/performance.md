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

The resident `RdfDataset` IR ([The Interned Dataset IR](../concepts/interned-dataset.md))
interns lexical text in a string arena and stores triple terms as component IDs.
Its copyable `TermId` uses `NonZeroU32`; the separate global address space uses
`NonZeroU64` in `GlobalTermId`. Hot maps use the fixed-key `FixedHasher`.
Frozen quad rows have lazy ordinal permutation indexes (four bytes per quad for
each populated permutation). The explicit graph-free `TripleDataset` has
three-component rows and separate reifier and annotation tables. Evaluation
works over interned IDs.

Layout comparisons live in `crates/rdf-core/benches/ir_layout.rs`. The suite
compares the resident quad-row store with structure-of-arrays and
predicate-adjacency alternatives over the same frozen quads, reporting
allocation counts, high-water memory and end-to-end latency. It also reports
the graph-free carrier's physical row tables and populated ordinal indexes.
Each result applies to its measured source, workload and host.

## The two benchmark layers

| Layer | What it measures | How to run |
| --- | --- | --- |
| **Rust bench suites** | Native engine hot paths — IR layout, copy-on-write mutation, pack index alternatives, codecs, graph/tabular/research-object projections, SPARQL lexing/evaluation/planning, SHACL validation, entailment chase, GTS authoring, IRI parsing. | `make bench` |
| **Python compat harness** | `purrdf.compat.rdflib` (the native-backed drop-in) vs. the real rdflib 7.x on parse, serialize, SPARQL, and triple-pattern iteration, over a deterministic `example.org` corpus. | `make bench-python` |

Both layers are report-only: they are never part of `make check`, and no test
gate asserts a speedup.

## The discipline for changes

Any change claiming a performance win must **extend the benches**
rather than asserting the speedup in prose. Where a planner or algorithm
choice matters for correctness-adjacent behavior, it is gated by
*deterministic* tests instead of timings — for example, the cost-based BGP
planner's win over the retired structural heuristic is asserted by unit tests
that count real intermediate rows, and by a differential corpus test, while
the bench merely watches for regressions.

`NativeSparqlEngine::explain_query` exposes the chosen BGP join order so
planner decisions can be audited without running the query
([SPARQL: Querying](../sparql/querying.md)).

## Rust, LLVM and hardware paths

The [Rust and SIMD optimization guide](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-simd.md)
explains the source formulations, mathematical contracts, assembly checks and
measurement discipline. Its hashing and compression sections cover constant
indices and rotations, bounded round expansion, SIMD chunk and parent batching,
streaming state, direct output sinks and allocation floors. It distinguishes
instruction coverage from throughput measured on the target hardware.

## Reproducing locally

```sh
make bench                              # the default bench set
cargo bench -p purrdf-iri --bench parse # a single package's bench
cargo bench -p purrdf-core --bench pack_index_compare # pack index experiment
cargo bench -p purrdf-rdf --bench projections -- --quick # projection/carrier sample
make bench-python                       # the rdflib comparison harness
```

Benchmark on a quiet machine, and compare like with like: allocator, CPU
scaling, and build flags all move the numbers.

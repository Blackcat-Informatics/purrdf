<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Benchmarks

PurRDF's performance is **measured, never asserted** (AGENTS.md §4). Nothing in
this document is a test gate, and no number here is a guarantee: benchmarks are
report-only, timing-sensitive, and vary by host, CPU, allocator, and build
flags. Treat every figure as a host-dependent illustration you reproduce
locally — not a promise of "Nx faster."

There are two benchmark layers:

| Layer | What it measures | How to run |
| --- | --- | --- |
| **Rust criterion suites** | The native engine hot paths — IR layout, codecs, SPARQL evaluation, SHACL validation, GTS authoring, and wasm wrapper overhead. | `make bench` |
| **Python compat harness** | `purrdf.compat.rdflib` (the native-backed drop-in) vs. the real `rdflib` 7.x, on the operations a drop-in user actually calls. | `make bench-python` |

## Native-layer benchmarks (criterion)

The Rust benches are the source of truth for engine-level layout and algorithm
choices — the shipped design is whichever the criterion numbers pick, not
whichever sounds fast (see README, "Fast by measurement, not by assertion").
They live under `crates/*/benches/`:

- `crates/rdf-core/benches/ir_layout.rs` — AoS vs. SoA vs. predicate-adjacency
  IR layouts (allocation counts, high-water mark, end-to-end latency).
- `crates/rdf-core/benches/mutable.rs` — copy-on-write mutation paths.
- `crates/rdf-core/benches/intern_content_id.rs` — content-addressing
  recognition cost for ordinary vs. genuine content-id IRIs.
- `crates/rdf-core/benches/pack_index_compare.rs` — the shipped pack codec's
  FoQ posting indexes vs. an internal bitmap wavelet-matrix candidate.
- `crates/rdf/benches/native_codecs.rs` — text/XML/JSON-LD codec throughput.
- `crates/rdf/benches/projections.rs` — RDF-to-LPG mapping, all four LPG
  carrier writers/readers, exact CSVW write/read, OBO Graphs, and SKOS, with
  per-operation allocation observations.
- `crates/sparql-algebra/benches/tokenize.rs` — SPARQL/Turtle lexer hot path
  (`IRIREF`, string literals, comments).
- `crates/sparql-eval/benches/query_eval.rs` — end-to-end SPARQL SELECT
  evaluation over synthetic datasets.
- `crates/sparql-eval/benches/cost_based_bgp_planner.rs` — regression watch on
  the cost-based BGP join planner; the deterministic win over the retired
  structural heuristic is gated by the `bgp` unit tests (which count real
  intermediate rows) and by the differential corpus test in
  `crates/sparql-conformance/tests/cost_planner_corpus.rs`.
- `crates/sparql-eval/benches/exists_decorrelation.rs` — `FILTER NOT EXISTS`
  anti-join cost with and without the `exists_memo` decorrelation path.
- `crates/sparql-eval/benches/lateral_service.rs` — variable-endpoint
  `SERVICE ?g` evaluated as a LATERAL join vs. a fixed-IRI `SERVICE <ep>`.
- `crates/shapes/benches/validate.rs` — SHACL validation.
- `crates/entail/benches/chase.rs` — RDFS forward-materialization chase scaling.
- `crates/gts/benches/authoring.rs` — GTS container authoring.
- `crates/rdf-wasm/benches/query_engine_reuse.rs` — package-root
  `QueryEngine` reuse vs. fresh-engine construction.
- `crates/iri/benches/parse.rs` — IRI parse/validate hot path over a mixed
  character-class corpus.

`NativeSparqlEngine::explain_query` exposes the chosen BGP order as an ordered
list of triple-pattern strings, so callers can audit planner decisions without
running the query.

Run the default set with `make bench` (report-only; never part of `make check`).
Additional benches are run package-by-package, e.g.
`cargo bench -p purrdf-iri --bench parse`.

### Native criterion benchmark inventory

| Bench | What it measures |
| --- | --- |
| `crates/rdf-core/benches/ir_layout.rs` | AoS / SoA / predicate-adjacency IR layout trade-offs (latency, allocations, peak RSS). |
| `crates/rdf-core/benches/mutable.rs` | Copy-on-write mutation paths on the immutable IR. |
| `crates/rdf-core/benches/intern_content_id.rs` | Extra intern-time cost when content-addressing is enabled: prefix-miss baseline, prefix-hit decode, and side-table insert. |
| `crates/rdf-core/benches/pack_index_compare.rs` | Exact bytes, build latency, and unbound-subject query latency for the shipped FoQ posting indexes vs. a non-shipped bitmap wavelet matrix over the same pack adjacency. |
| `crates/rdf/benches/native_codecs.rs` | Throughput of the native Turtle, TriG, N-Triples, N-Quads, RDF/XML, and JSON-LD serializers/parsers. |
| `crates/rdf/benches/projections.rs` | Graph/tabular mapping and carrier throughput plus allocation counts over deterministic RDF, OBO, and SKOS fixtures. |
| `crates/sparql-algebra/benches/tokenize.rs` | Lexer throughput on long IRI bodies, escaped string literals, and comment tails. |
| `crates/sparql-eval/benches/query_eval.rs` | End-to-end SPARQL SELECT latency including BGP joins, filters, and aggregates. |
| `crates/sparql-eval/benches/cost_based_bgp_planner.rs` | Planner regression watch: cost-based BGP ordering vs. the retired structural heuristic. |
| `crates/sparql-eval/benches/exists_decorrelation.rs` | `FILTER NOT EXISTS` inner-pattern re-evaluation and index-rebuild cost with/without memoization. |
| `crates/sparql-eval/benches/lateral_service.rs` | `SERVICE ?g` LATERAL substitute-and-forward cost as the number of distinct endpoint bindings grows. |
| `crates/shapes/benches/validate.rs` | SHACL Core validation latency on synthetic shapes/graphs. |
| `crates/entail/benches/chase.rs` | RDFS semi-naive materialization scaling on subclass chains. |
| `crates/gts/benches/authoring.rs` | GTS container authoring: append, hash, and CBOR-log construction throughput. |
| `crates/rdf-wasm/benches/query_engine_reuse.rs` | Binding-level SELECT overhead for reused package-root `QueryEngine` instances vs. fresh construction. |
| `crates/iri/benches/parse.rs` | `purrdf_iri::parse` component validation across scheme, authority, path, query, and fragment classes. |

### Graph and tabular projections

`crates/rdf/benches/projections.rs` builds three deterministic `example.org`
datasets without RNG: a 600-quad general graph, a 600-quad OBO/OWL graph, and an
800-quad SKOS source graph. The canonical LPG projection contains 408 nodes plus
edges. Criterion measures the complete mapping/serialization/parser operations;
fixture construction stays outside the timed loops. A counting global allocator
also reports calls and requested bytes for representative single operations.
Those counts are cumulative allocation traffic, not retained or peak memory.

Run it with:

```sh
cargo bench -p purrdf-rdf --bench projections --locked
cargo bench -p purrdf-rdf --bench projections --locked -- --quick
```

The following `--quick` snapshot was measured on 2026-07-16 with rustc 1.96.1,
Linux 7.1.3, and an AMD Ryzen AI MAX+ 395. Values are Criterion point estimates
from one report-only run and are rounded; they are observations, not gates or
performance promises.

| Operation | Input/output elements | Time | Throughput |
| --- | ---: | ---: | ---: |
| RDF → canonical LPG | 600 quads | 3.44 ms | 174 Kquad/s |
| generic CSV write | 408 nodes + edges | 2.00 ms | 204 Kelem/s |
| generic CSV read | 408 nodes + edges | 4.77 ms | 85.5 Kelem/s |
| Neo4j CSV write | 408 nodes + edges | 2.62 ms | 156 Kelem/s |
| Neo4j CSV read | 408 nodes + edges | 6.10 ms | 66.9 Kelem/s |
| openCypher write | 408 nodes + edges | 4.73 ms | 86.3 Kelem/s |
| openCypher read | 408 nodes + edges | 9.66 ms | 42.2 Kelem/s |
| GraphML write | 408 nodes + edges | 4.73 ms | 86.2 Kelem/s |
| GraphML read | 408 nodes + edges | 14.4 ms | 28.3 Kelem/s |
| exact CSVW write | 600 quads | 1.79 ms | 335 Kquad/s |
| exact CSVW read | 600 quads | 3.36 ms | 179 Kquad/s |
| OBO Graphs view | 600 quads | 611 µs | 982 Kquad/s |
| SKOS view | 800 quads | 1.41 ms | 569 Kquad/s |

Representative one-operation allocation traffic from the same run:

| Operation | Allocation calls | Requested bytes |
| --- | ---: | ---: |
| RDF → canonical LPG | 49,106 | 4,819,500 |
| generic CSV write | 29,893 | 2,841,024 |
| generic CSV read | 67,092 | 6,577,230 |
| exact CSVW write | 22,695 | 2,616,210 |
| exact CSVW read | 39,960 | 4,307,718 |
| OBO Graphs view | 17,295 | 2,328,323 |
| SKOS view | 38,273 | 4,129,381 |

This baseline makes carrier/parser and allocation regressions visible without
asserting that any format should outrun another; their grammars and validation
work differ materially.

### Pack FoQ vs. bitmap wavelet matrix

The pack-index comparison is an internal format-selection experiment; the
wavelet matrix is not part of the library API or pack wire format. It stores one
rank/select bitmap per alphabet bit over each existing `Sp` and `So` sequence.
The FoQ comparand reproduces the shipped bit-packed offsets/counts and
delta-varint posting data. Both candidates retain the same `Sp`/`Bp`/`So`/`Bo`
adjacency, so the index-only and adjacency-plus-index totals are reported
separately.

Each deterministic row emits four `example.org` triples. The row contains a
dense predicate and object, bounded cyclic predicate/object families, and a
unique long-tail object. This supplies dense and sparse bindings for `(?,p,?)`,
`(?,?,o)`, and `(?,p,o)` without RNG. Tests compare both candidates with a plain
vector oracle and the real `PackView` results. The closed-form space model is
checked against materialized encodings at alphabet-width boundaries before it
is used for the 100-million and 1-billion-triple rows below.

Run the correctness test, a short timing sample, or the one-pass space/query
smoke mode with:

```sh
cargo test -p purrdf-core --test pack_index_compare --locked
cargo bench -p purrdf-core --bench pack_index_compare --locked -- --quick
cargo bench -p purrdf-core --bench pack_index_compare --locked -- --test
```

#### Representative space result

These exact encoded byte counts use the workload above. “Total” is the shared
adjacency plus the selected index; dictionary and outer pack framing are common
and excluded.

| triples | FoQ index | wavelet index | FoQ total | wavelet total |
| ---: | ---: | ---: | ---: | ---: |
| 65,536 | 280,221 | 263,162 | 490,485 | 473,426 |
| 262,144 | 1,102,389 | 1,139,034 | 2,008,717 | 2,045,362 |
| 1,048,576 | 4,480,461 | 4,912,570 | 8,367,653 | 8,799,762 |
| 100,000,000 | 489,959,547 | 571,486,284 | 935,662,769 | 1,017,189,506 |
| 1,000,000,000 | 5,284,861,054 | 6,230,470,670 | 10,116,892,394 | 11,062,502,010 |

At small sizes, 64-bit word rounding causes narrow ordering oscillations: an
exact scan through 1,048,576 rows finds 68 flips. The final flip in that scan is
from smaller to larger at 61,183 rows (244,732 triples); the wavelet index stays
larger through the rest of the scan and at both target-scale model points. At
the two requested large scales it uses 16.6% and 17.9% more index bytes;
including the shared adjacency, the complete structure is 8.7% and 9.3% larger.
This result applies to the concrete rank-directory bitmap representation
measured here, not to an RRR entropy-coded representation.

#### Representative timing result

This `--quick` snapshot was taken on 2026-07-14 with `rustc 1.96.1`, Linux
7.1.3, and an AMD Ryzen AI MAX+ 395 (16 cores / 32 threads). The query fixture is
262,144 triples. “Production” is the real `PackView` FoQ path; the isolated
columns remove dictionary/view overhead and are the direct index comparison.
Values are Criterion point estimates rounded to three significant figures.

| operation | production FoQ | isolated FoQ | isolated wavelet | wavelet / FoQ |
| --- | ---: | ---: | ---: | ---: |
| build complete pack / index | 336 ms | 12.5 ms | 7.21 ms | 0.58× |
| `(?,p,?)`, dense | 5.22 ms | 3.76 ms | 31.3 ms | 8.31× |
| `(?,?,o)`, dense | 671 µs | 76.3 µs | 45.0 ms | 590× |
| `(?,p,o)`, dense | 658 µs | 162 µs | 70.5 ms | 436× |
| `(?,p,?)`, sparse | 3.13 µs | 2.19 µs | 13.2 µs | 6.01× |
| `(?,?,o)`, sparse | 79.6 ns | 13.6 ns | 431 ns | 31.7× |
| `(?,p,o)`, sparse | 247 ns | 144 ns | 11.7 µs | 81.4× |

The bitmap wavelet matrix builds its isolated indexes about 42% faster, but it
is larger at the target scales and regresses every measured query shape. The
measured production decision is therefore to retain both FoQ posting indexes;
pack bytes and decoding behavior remain unchanged.

## Python compat harness (`bench_compat.py`)

`bindings/python/benchmarks/bench_compat.py` times the shim and the genuine
`rdflib` **in one process** and prints a side-by-side table with the
purrdf/rdflib ratio. It is deliberately kept out of `make pytest` because it is
slow and timing-sensitive, but it is run in the separate, report-only
`benchmarks` CI job. That job produces `bench_compat.json` and uploads it
alongside the Criterion artifacts. The job uses `continue-on-error: true`, so it
never fails the main gate.

### Methodology

- **Corpus** — generated **deterministically** from the triple index. Each
  N-Triples line is a closed-form function of its integer index (`bench_compat.py`,
  `_triple_line`): no `random`, no wall-clock in the data. Three interleaved
  shapes (a typed integer literal, an `rdf:type`, and an object reference to a
  neighbouring subject) give BGP joins, filters, and aggregates real work. Every
  IRI is under `example.org` — PurRDF mints no vocabulary of its own. Because the
  corpus is a pure function of the size, both engines parse byte-identical input
  and successive runs are directly comparable.
- **Sizes** — 1,000 / 10,000 / 100,000 triples by default (`--sizes`).
- **Operations** — `parse` (N-Triples and Turtle), `serialize` (N-Triples and
  Turtle), two SPARQL SELECTs (a BGP+join, and a filter+aggregate `COUNT`/`AVG`),
  and triple-pattern iteration (`triples((None, None, None))`). The Turtle input
  is produced once by the real `rdflib` so both engines parse the same Turtle
  bytes.
- **Timing** — `time.perf_counter` only; several repetitions (`--repetitions`,
  default 5); we report **best-of** and **median** in milliseconds. Read-only
  operations (serialize/query/iterate) run against a pre-parsed graph so setup
  is not folded into the measurement.
- **Report** — a text table to stdout, plus optional machine-readable JSON via
  `--json path.json`.

### How to run

```sh
make bench-python
# or, for a quick pass and a JSON dump:
cd bindings/python
uv run python benchmarks/bench_compat.py --sizes 1000 10000 --repetitions 5 --json out.json
```

The `ratio (p/r)` column is `purrdf_ms / rdflib_ms`: **below 1.0 the shim is
faster; above 1.0 the real `rdflib` is faster** for that cell on that host.

### Representative results (host-dependent illustration)

The table below is a single run on the author's development machine
(Linux, `rdflib` 7.6.0, best-of-5, ms). **It is an illustration, not a
guarantee** — your numbers will differ. Reproduce with `make bench-python`.

| size | operation | purrdf ms | rdflib ms | ratio (p/r) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | parse_nt | 16.8 | 13.8 | 1.22 |
| 1,000 | parse_ttl | 17.5 | 30.4 | 0.58 |
| 1,000 | serialize_nt | 9.5 | 2.7 | 3.53 |
| 1,000 | serialize_ttl | 32.5 | 24.9 | 1.31 |
| 1,000 | query_bgp | 6.7 | 2.2 | 3.13 |
| 1,000 | query_agg | 7.1 | 8.4 | 0.85 |
| 1,000 | triples_scan | 5.5 | 0.7 | 8.20 |
| 10,000 | parse_nt | 178.6 | 93.5 | 1.91 |
| 10,000 | parse_ttl | 187.1 | 244.8 | 0.76 |
| 10,000 | serialize_nt | 64.9 | 34.3 | 1.89 |
| 10,000 | serialize_ttl | 214.3 | 195.6 | 1.10 |
| 10,000 | query_bgp | 41.7 | 4.9 | 8.47 |
| 10,000 | query_agg | 40.0 | 15.2 | 2.63 |
| 10,000 | triples_scan | 41.8 | 7.9 | 5.29 |
| 100,000 | parse_nt | 964.1 | 2081.3 | 0.46 |
| 100,000 | parse_ttl | 1429.9 | 3033.4 | 0.47 |
| 100,000 | serialize_nt | 825.0 | 210.2 | 3.93 |
| 100,000 | serialize_ttl | 2869.9 | 1947.1 | 1.47 |
| 100,000 | query_bgp | 488.1 | 36.4 | 13.42 |
| 100,000 | query_agg | 494.0 | 98.6 | 5.01 |
| 100,000 | triples_scan | 358.6 | 77.6 | 4.62 |

Reading this honestly, on this host and this run:

- **Bulk N-Triples/Turtle parsing** scales better in the shim — at 100k triples
  it parsed both formats roughly 2.1–2.2× faster than real `rdflib`, and it was
  already ahead on Turtle at every size.
- **Per-item operations that cross the Python↔native boundary once per result or
  per triple** — `serialize`, `triples()` iteration, and the SELECT queries —
  pay marshalling overhead in the shim, so real `rdflib` (which stays in pure
  Python objects) wins those cells here.

The takeaway is not a single multiplier but a shape: the shim's advantage is in
native bulk work, and its cost is boundary-crossing per Python object. Which
matters for *your* workload is exactly what `make bench-python` is for — run it
on your host with a corpus close to your data before drawing conclusions.

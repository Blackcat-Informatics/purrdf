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
| **Rust criterion suites** | The native engine hot paths — IR layout, codecs, SPARQL evaluation, SHACL validation, GTS authoring. | `make bench` |
| **Python compat harness** | `purrdf.compat.rdflib` (the native-backed drop-in) vs. the real `rdflib` 7.x, on the operations a drop-in user actually calls. | `make bench-python` |

## Native-layer benchmarks (criterion)

The Rust benches are the source of truth for engine-level layout and algorithm
choices — the shipped design is whichever the criterion numbers pick, not
whichever sounds fast (see README, "Fast by measurement, not by assertion").
They live under `crates/*/benches/`:

- `crates/rdf-core/benches/ir_layout.rs` — AoS vs. SoA vs. predicate-adjacency
  IR layouts (allocation counts, high-water mark, end-to-end latency).
- `crates/rdf-core/benches/mutable.rs` — copy-on-write mutation paths.
- `crates/rdf/benches/native_codecs.rs` — text/XML/JSON-LD codec throughput.
- `crates/sparql-eval/benches/{query_eval,cost_based_bgp_planner,exists_decorrelation}.rs`
  — SPARQL evaluation and planner costs.
- `crates/shapes/benches/validate.rs` — SHACL validation.
- `crates/gts/benches/authoring.rs` — GTS container authoring.

Run them all with `make bench` (report-only; never part of `make check`).

## Python compat harness (`bench_compat.py`)

`bindings/python/benchmarks/bench_compat.py` times the shim and the genuine
`rdflib` **in one process** and prints a side-by-side table with the
purrdf/rdflib ratio. It is deliberately kept out of `make pytest` and CI: it is
slow and timing-sensitive, so it is never collected as a test.

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
| 1,000 | parse_nt | 9.9 | 6.7 | 1.48 |
| 1,000 | parse_ttl | 8.9 | 12.2 | 0.73 |
| 1,000 | serialize_nt | 5.3 | 1.3 | 4.10 |
| 1,000 | serialize_ttl | 18.7 | 11.8 | 1.59 |
| 1,000 | query_bgp | 3.0 | 1.0 | 2.99 |
| 1,000 | query_agg | 3.0 | 3.6 | 0.84 |
| 1,000 | triples_scan | 2.2 | 0.3 | 6.42 |
| 10,000 | parse_nt | 95.3 | 71.9 | 1.33 |
| 10,000 | parse_ttl | 87.3 | 125.6 | 0.70 |
| 10,000 | serialize_nt | 55.8 | 13.6 | 4.11 |
| 10,000 | serialize_ttl | 196.4 | 127.4 | 1.54 |
| 10,000 | query_bgp | 31.2 | 2.4 | 13.14 |
| 10,000 | query_agg | 30.8 | 10.2 | 3.01 |
| 10,000 | triples_scan | 22.7 | 3.6 | 6.28 |
| 100,000 | parse_nt | 777.9 | 1863.5 | 0.42 |
| 100,000 | parse_ttl | 1110.0 | 2894.3 | 0.38 |
| 100,000 | serialize_nt | 746.4 | 202.0 | 3.69 |
| 100,000 | serialize_ttl | 2550.6 | 1693.3 | 1.51 |
| 100,000 | query_bgp | 432.5 | 17.1 | 25.34 |
| 100,000 | query_agg | 426.7 | 76.9 | 5.55 |
| 100,000 | triples_scan | 303.3 | 59.3 | 5.11 |

Reading this honestly, on this host and this run:

- **Bulk N-Triples/Turtle parsing** scales better in the shim — at 100k triples
  it parsed both formats roughly 2.4–2.6× faster than real `rdflib`, and it was
  already ahead on Turtle at every size.
- **Per-item operations that cross the Python↔native boundary once per result or
  per triple** — `serialize`, `triples()` iteration, and the SELECT queries —
  pay marshalling overhead in the shim, so real `rdflib` (which stays in pure
  Python objects) wins those cells here.

The takeaway is not a single multiplier but a shape: the shim's advantage is in
native bulk work, and its cost is boundary-crossing per Python object. Which
matters for *your* workload is exactly what `make bench-python` is for — run it
on your host with a corpus close to your data before drawing conclusions.

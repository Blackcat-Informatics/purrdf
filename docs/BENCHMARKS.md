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
- `crates/rdf-core/benches/purremb.rs` — `.purremb` validation, resident
  reopen, target and prefix access, exact/two-stage retrieval, streaming write,
  binary64 access, and a one-million-chunk catalog.
- `crates/rdf-core/benches/purremb_alloc.rs` — one-shot allocation traffic and
  live-byte high-water probes over the same deterministic `.purremb` fixtures.
- `crates/rdf/benches/native_codecs.rs` — text/XML/JSON-LD codec throughput,
  including separate context compilation and expanded/caller/derived paths.
- `crates/rdf/benches/projections.rs` — RDF-to-LPG mapping, all four LPG
  carrier writers/readers, exact CSVW write/read, OBO Graphs, SKOS, the shared
  research-object model, and all five research-object carriers, with
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
- `crates/shapes/benches/validate.rs` — SHACL validation plus JSON Schema and
  LinkML import/lowering throughput and one-operation allocation traffic.
- `crates/shapes/benches/schema_surface.rs` — complete ontology-aware schema
  compilation for shaped-only, sparse, and dense property surfaces.
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
| `crates/rdf-core/benches/purremb.rs` | Full validation and resident reopen over a 16,384 x 384 binary32 Matryoshka matrix; target/row/prefix access, exact and coarse-prefix/full-prefix top-10 retrieval, canonical streaming output, a 4,096 x 128 binary64 matrix, and a one-million-chunk hierarchy. |
| `crates/rdf-core/benches/purremb_alloc.rs` | Allocation calls, requested bytes, retained-byte deltas, and live-byte high-water deltas for PURREMB fixture construction, verification, and streaming. |
| `crates/rdf/benches/native_codecs.rs` | Throughput of the native Turtle, TriG, N-Triples, N-Quads, RDF/XML, and JSON-LD serializers/parsers; JSON-LD context compilation and expanded/caller/derived modes are reported separately. |
| `crates/rdf/benches/projections.rs` | Graph, tabular, and research-object mapping/carrier throughput plus allocation counts over deterministic fixtures. |
| `crates/sparql-algebra/benches/tokenize.rs` | Lexer throughput on long IRI bodies, escaped string literals, and comment tails. |
| `crates/sparql-eval/benches/query_eval.rs` | End-to-end SPARQL SELECT latency including BGP joins, filters, and aggregates. |
| `crates/sparql-eval/benches/cost_based_bgp_planner.rs` | Planner regression watch: cost-based BGP ordering vs. the retired structural heuristic. |
| `crates/sparql-eval/benches/exists_decorrelation.rs` | `FILTER NOT EXISTS` inner-pattern re-evaluation and index-rebuild cost with/without memoization. |
| `crates/sparql-eval/benches/lateral_service.rs` | `SERVICE ?g` LATERAL substitute-and-forward cost as the number of distinct endpoint bindings grows. |
| `crates/shapes/benches/validate.rs` | SHACL Core validation latency plus JSON Schema/LinkML → SHACL import/lowering throughput and allocation traffic on deterministic fixtures. |
| `crates/shapes/benches/schema_surface.rs` | RDFC-keyed shaped-only compilation and sparse/dense ontology-complete class/property relation plus JSON Schema/OpenAPI emission. |
| `crates/entail/benches/chase.rs` | RDFS semi-naive materialization scaling on subclass chains. |
| `crates/gts/benches/authoring.rs` | GTS container authoring: append, hash, and CBOR-log construction throughput. |
| `crates/rdf-wasm/benches/query_engine_reuse.rs` | Binding-level SELECT overhead for reused package-root `QueryEngine` instances vs. fresh construction. |
| `crates/iri/benches/parse.rs` | `purrdf_iri::parse` component validation across scheme, authority, path, query, and fragment classes. |

### PURREMB companion format

The shared `crates/rdf-core/benches/support/purremb.rs` module constructs three
deterministic fixtures without RNG, wall-clock values, local paths, or
process-specific canonical input. The timed and allocation processes consume
the exact same builders:

- a 16,384-row x 384-coordinate binary32 matrix (27,267,328-byte artifact)
  with one stored matrix and raw-32, deterministic-L2-64, and
  deterministic-L2-384 Matryoshka spaces;
- a 4,096-row x 128-coordinate binary64 matrix (4,722,368-byte artifact); and
- exactly 1,000,000 digest-only chunk subjects distributed across 4,096
  retained document shards (217,642,688-byte artifact).

The binary32 coordinates are a pure function of each `TargetId`. Coordinates
after position 64 are correlated with the leading 64 coordinates, giving the
fixture a deliberate nested-space shape. The retrieval comparison excludes the
query row, obtains 128 candidates by an exact scan of the normalized
64-coordinate prefix, and reranks those candidates in the normalized
384-coordinate space. It reports recall@10 against an exact full-space top 10.
This measures the two-stage access pattern, not an ANN implementation: opaque
ANN engines and their quality remain outside the format.

“Full validation” times the complete structural-open, section/root hashing,
typed-identity, finite-scalar, and projection-recomputation path. The bytes are
already resident and the harness does not flush the operating-system page
cache, so this is a cold-validation-code-path proxy, not disk or object-store
latency. “Resident prevalidated reopen” applies a certificate to the same
immutable allocation and still performs structural validation. The streaming
measurement preallocates its output buffer and clones typed input during
Criterion's untimed batch setup; the timed operation includes canonical
metadata encoding, layout, matrix streaming, all integrity/projection hashes,
and backpatching. A setup assertion requires its bytes to equal the unordered
builder output exactly.

Timed Criterion samples run in `purremb` with the normal process allocator. The
separate `purremb_alloc` executable installs a counting allocator and reports
allocation calls, cumulative requested bytes, retained-byte deltas, and the
maximum live-byte delta observed during selected operations. Keeping the
processes separate prevents the allocator's atomic accounting from changing the
timed workloads. “Peak working bytes” is an in-process allocator high-water
observation, not peak RSS, mapped-file residency, kernel page cache, or a memory
budget. Structural `EmbeddingView::from_bytes` remains borrowed; full
relation-completeness verification intentionally builds temporary relation
catalogs for the million-chunk fixture. The streaming observation includes the
27.3 MB caller-owned `Vec` sink; matrix processing itself retains one row buffer
and its digest states, and a file or network sink need not retain output bytes.

Run the exact fixtures, a timed short sample, or a reduced-catalog smoke pass
with:

```sh
cargo bench -p purrdf-core --bench purremb --locked -- --test
cargo bench -p purrdf-core --bench purremb --locked -- --quick
cargo bench -p purrdf-core --bench purremb_alloc --locked
PURREMB_CATALOG_SUBJECTS=10000 \
  cargo bench -p purrdf-core --bench purremb --locked -- --test
PURREMB_CATALOG_SUBJECTS=10000 \
  cargo bench -p purrdf-core --bench purremb_alloc --locked
```

The environment override is for local smoke testing only. Reported results use
the one-million default.

#### Representative PURREMB result

This `--quick` snapshot was measured on 2026-07-17 with rustc 1.96.1, Linux
7.1.3, and an AMD Ryzen AI MAX+ 395 (16 cores / 32 threads). The host reports
768 KiB L1d, 512 KiB L1i, 16 MiB L2, and 64 MiB L3 in aggregate. The 27.3 MB
binary32 artifact fits within aggregate L3 while the 217.6 MB chunk catalog does
not; both were resident in process memory. Values are Criterion point estimates
from one report-only run and are rounded. Sub-nanosecond target-by-row timing is
an optimized tight-loop observation, not an end-to-end request latency.

| Operation | Fixture | Time | Throughput where meaningful |
| --- | --- | ---: | ---: |
| full validation | 16,384 x 384 `f32` | 62.1 ms | 418 MiB/s |
| resident prevalidated reopen | 16,384 x 384 `f32` | 2.10 ms | 12.1 GiB/s |
| target by matrix row | 16,384 targets | 0.599 ns | — |
| target by `TargetId` | 16,384 targets | 35.3 ns | — |
| raw 32-coordinate prefix borrow | `f32` | 4.76 ns | — |
| deterministic-L2 64-coordinate prefix iteration | `f32` | 258 ns | — |
| native aligned 384-coordinate row borrow | `f32` | 2.05 ns | — |
| exact normalized 384-coordinate top-10 scan | 16,384 rows | 12.7 ms | 1.29 Mrow/s |
| exact 64-prefix candidates + 384-prefix top-10 rerank | 16,384 rows, 128 candidates | 5.90 ms | 2.77 Mrow/s |
| canonical streaming write | 27,267,328 bytes | 72.5 ms | 359 MiB/s |
| full validation | 4,096 x 128 `f64` | 7.78 ms | 579 MiB/s |
| native aligned 128-coordinate row borrow | `f64` | 1.54 ns | — |
| target by `TargetId` | 1,000,000 chunks | 42.1 ns | — |
| one document's relation range | 1,000,000 chunks / 4,096 documents | 497 ns | — |

The correlated synthetic Matryoshka fixture reported recall@10 = 1.000 after
64-coordinate retrieval and 384-coordinate reranking. That result demonstrates
the guard-correct two-stage path for this generator; it is not a model-quality
claim and must not be generalized to independently trained or uncorrelated
embeddings.

Representative allocator observations from the companion allocation process on
the same revision and host:

| Operation | Allocation calls | Requested bytes | Retained bytes | Peak working bytes |
| --- | ---: | ---: | ---: | ---: |
| build verified `f32` fixture plus retained input rows | 147,746 | 123,609,845 | 54,793,658 | 82,063,846 |
| full verification of relation-free `f32` fixture | 2 | 48 | 0 | 40 |
| complete canonical streaming write | 133 | 35,939,982 | 27,267,328 | 32,777,324 |
| build verified `f64` fixture | 37,113 | 17,756,475 | 4,722,368 | 9,447,684 |
| build one-million-chunk catalog | 5,028,985 | 1,895,285,731 | 217,642,688 | 548,434,988 |
| full verification of one-million-chunk catalog | 22 | 210,884,640 | 0 | 139,581,696 |

These observations are report-only. They establish reproducible workloads and
make regressions visible; they do not impose latency, throughput, recall, or
memory thresholds.

### SHACL schema import

The `shacl_schema_import` group constructs a compact, deterministic draft
2020-12 document with 128 classes and 1,024 properties. Its mix covers scalar
facets, finite values, homogeneous arrays, requiredness, closure, and cyclic
local references. Fixture construction and caller-owned namespace/datatype
configuration remain outside the timed loop; Criterion measures the complete
JSON parse, validation, ordered lowering, and loss-ledger path.

The `shacl_linkml_import` group derives one canonical LinkML 1.11 document from
the same source fixture before timing begins. It measures native document
validation, class/slot/type traversal, shared schema lowering, and reverse-loss
construction. Both groups assert the imported shape count in their warmup,
allocation probe, and measured loop so a failed or vacuous import cannot appear
as a speedup.

A counting global allocator also prints calls and requested bytes for one
warmed import. Those counters represent cumulative allocation traffic, not
retained or peak memory, and the benchmark is report-only: neither timing nor
allocation output is a CI threshold or performance promise.

The `linkml_slot_emission` group measures the forward name-planning boundary on
matched fixtures with the same class, slot count, requiredness, and constraint
payload. `safe` uses directly representable names, `rename` uses distinct unsafe
locals, and `collision` gives every unsafe local one sanitized stem while a safe
slot owns that stem. Each mode runs at 32, 1,024, and 60,000 slots (near the
65,536 per-class limit). Schema/config construction stays outside the timed
loop; warmup, one-operation allocation probe, and measured loop all assert the
expected rename and collision counts. Throughput is total source slots. The
printed allocation calls/requested bytes are traffic, not retained or peak
memory, and no speedup claim or gate is attached to them.

```sh
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_schema_import
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_schema_import --quick
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_linkml_import
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_linkml_import --quick
cargo bench -p purrdf-shapes --bench validate --locked -- linkml_slot_emission
cargo bench -p purrdf-shapes --bench validate --locked -- linkml_slot_emission --quick
```

### SHACL ontology schema surface

The `shacl_schema_surface` group keeps namespace configuration and parsed RDF
fixtures outside the timed loop. Each iteration measures the complete public
compilation contract: RDFC-1.0 input identities, property catalog, SCC-condensed
OWL/RDFS propagation, coverage manifest, JSON Schema, and OpenAPI. Three fixed
fixtures distinguish 128 shaped classes with 128 properties, a sparse 256 by
256 ontology relation, and a dense domainless 128 by 256 relation. Inputs are
generated deterministically without RNG, time, or filesystem data.

The suite is report-only and carries no latency threshold. Run its compile and
single-sample smoke forms with:

```sh
cargo bench -p purrdf-shapes --bench schema_surface --locked --no-run
cargo bench -p purrdf-shapes --bench schema_surface --locked -- --test
```

### Graph, tabular, and research-object projections

`crates/rdf/benches/projections.rs` builds four deterministic `example.org`
datasets without RNG: a 600-quad general graph, a 600-quad OBO/OWL graph, an
800-quad SKOS source graph, and a 29-quad research-object intersection. The
canonical LPG projection contains 408 nodes plus edges. Criterion measures the
complete mapping/serialization/parser operations; fixture construction and
strict profile-config parsing stay outside the timed loops. A counting global
allocator also reports calls and requested bytes for representative single
operations. Those counts are cumulative allocation traffic, not retained or
peak memory.

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
| research-object common model | 29 quads | 27.59 µs | 1.051 Mquad/s |
| Croissant 1.1 write | 29 quads | 32.94 µs | 881 Kquad/s |
| Croissant 1.1 read | 29 quads | 69.83 µs | 415 Kquad/s |
| RO-Crate 1.3 write | 29 quads | 38.25 µs | 758 Kquad/s |
| RO-Crate 1.3 read | 29 quads | 72.14 µs | 402 Kquad/s |
| DataCite 4.6 write | 29 quads | 29.63 µs | 979 Kquad/s |
| DataCite 4.6 read | 29 quads | 45.34 µs | 640 Kquad/s |
| DCAT 3 write | 29 quads | 68.16 µs | 425 Kquad/s |
| DCAT 3 read | 29 quads | 115.38 µs | 251 Kquad/s |
| Frictionless Data Package v1 write | 29 quads | 31.69 µs | 915 Kquad/s |
| Frictionless Data Package v1 read | 29 quads | 60.43 µs | 480 Kquad/s |

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
| research-object common model | 898 | 81,277 |
| Croissant 1.1 write | 1,039 | 102,291 |
| Croissant 1.1 read | 2,437 | 277,705 |
| RO-Crate 1.3 write | 1,111 | 110,849 |
| RO-Crate 1.3 read | 2,637 | 294,742 |
| DataCite 4.6 write | 1,017 | 100,560 |
| DataCite 4.6 read | 1,493 | 216,383 |
| DCAT 3 write | 2,109 | 261,230 |
| DCAT 3 read | 3,524 | 435,850 |
| Frictionless Data Package v1 write | 1,025 | 99,013 |
| Frictionless Data Package v1 read | 2,183 | 259,794 |

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

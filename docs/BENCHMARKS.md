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

One thing that is *not* a caveat any more: the profile under `cargo test`.
`[profile.dev]` builds this workspace, its dependencies, and its build
scripts/proc-macros at **opt-level 3** (with `debug-assertions` and
`overflow-checks` on, which the benches below do not disable), so a timing taken
from a test or an ad-hoc `cargo run` is measuring optimized code rather than the
unoptimized default it used to. It is still not a bench: `make bench` runs under
`bench`/`release`, which additionally carries fat LTO and `codegen-units = 1`.
Quote a criterion number, not a test's wall clock.

There are five benchmark layers. The third is not a timing layer at all: it
produces the *input* the other measurements are taken over, and it is here
because a capacity number whose corpus nobody can regenerate is a number nobody
can check. The last two are the only ones that measure PurRDF against *the
outside world*: the first three compare PurRDF to itself, and a workload the
literature already publishes numbers for is the one thing that cannot be tuned
to flatter.

Those two are not interchangeable, and they are not comparable with each other.
LUBM asks what an engine can **derive** — eleven of its fourteen queries have no
answers at all without an entailment regime. WatDiv asks how a planner copes
with query **shape and selectivity** over a deliberately skewed dataset, and
needs no inference whatsoever. A row from one lane set beside a row from the
other compares two different questions.

| Layer | What it measures | How to run |
| --- | --- | --- |
| **Rust criterion suites** | The native engine hot paths — IR layout, codecs, SPARQL evaluation, SHACL validation, GTS authoring, and wasm wrapper overhead. | `make bench` |
| **Python compat harness** | `purrdf.compat.rdflib` (the native-backed drop-in) vs. the real `rdflib` 7.x, on the operations a drop-in user actually calls. | `make bench-python` |
| **Scale corpus** | Nothing, by itself. It *generates* the deterministic mixed corpus (`purrdf-scale-mixed-v1`) that a capacity capture is measured against, and reports what it produced. | `make scale-corpus` |
| **LUBM comparison workload** | The published LUBM generator, its 14 published queries, and the entailment regime each one needs — the workload the OWL knowledge-base literature compares engines on. | `make lubm` |
| **WatDiv comparison workload** | A digest-pinned frozen WatDiv dataset and the 20 published query templates, instantiated deterministically — the workload the RDF-store literature compares planners on. Pure BGP, no entailment. | `make watdiv` |

## Native-layer benchmarks (criterion)

The Rust benches are the source of truth for engine-level layout and algorithm
choices — the shipped design is whichever the criterion numbers pick, not
whichever sounds fast (see README, "Fast by measurement, not by assertion").
They live under `crates/*/benches/`. The workspace registers 47 `[[bench]]`
targets in total; this section and the inventory table below document 20 of
them — the ones with a story worth telling about a hot path or a design
trade-off. The rest run under `make bench` like any other target and are
simply not narrated here:

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
- `crates/rdf/benches/projections.rs` — RDF-to-LPG mapping, selective versus
  explicit-all scope, materialized-package versus direct-sink output for all
  four LPG carriers, carrier readers, exact CSVW write/read, OBO Graphs, SKOS,
  mapped/CONSTRUCT DCAT RDF, VoID generation, the shared research-object model,
  and all five research-object carriers, with per-operation allocation
  observations.
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
- `crates/entail/benches/chase.rs` — RDFS forward-materialization chase scaling
  (`materialize(ds, Regime::Rdfs)` end to end, so the declared clause program and
  `purrdf-datalog`'s semi-naive fixpoint are both inside the timed loop).
- `crates/entail/benches/classify.rs` — OWL-Direct classification latency over
  a synthetic `EL` terminology at three signature sizes, since classification
  cost is superlinear in the signature rather than in the axiom count.
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

This table documents 20 of the 47 `[[bench]]` targets registered across the
workspace's `Cargo.toml` files — the subset narrated in the prose list above,
in the same order. It is not a claim of completeness: `cargo bench -p <crate>
--bench <name>` reaches every registered target whether or not it has a row
here.

| Bench | What it measures |
| --- | --- |
| `crates/rdf-core/benches/ir_layout.rs` | AoS / SoA / predicate-adjacency IR layout trade-offs (latency, allocations, peak RSS). |
| `crates/rdf-core/benches/mutable.rs` | Copy-on-write mutation paths on the immutable IR. |
| `crates/rdf-core/benches/intern_content_id.rs` | Extra intern-time cost when content-addressing is enabled: prefix-miss baseline, prefix-hit decode, and side-table insert. |
| `crates/rdf-core/benches/pack_index_compare.rs` | Exact bytes, build latency, and unbound-subject query latency for the shipped FoQ posting indexes vs. a non-shipped bitmap wavelet matrix over the same pack adjacency. |
| `crates/rdf-core/benches/purremb.rs` | Full validation and resident reopen over a 16,384 x 384 binary32 Matryoshka matrix; target/row/prefix access, exact and coarse-prefix/full-prefix top-10 retrieval, canonical streaming output, a 4,096 x 128 binary64 matrix, and a one-million-chunk hierarchy. |
| `crates/rdf-core/benches/purremb_alloc.rs` | Allocation calls, requested bytes, retained-byte deltas, and live-byte high-water deltas for PURREMB fixture construction, verification, and streaming. |
| `crates/rdf/benches/native_codecs.rs` | Throughput of the native Turtle, TriG, N-Triples, N-Quads, RDF/XML, and JSON-LD serializers/parsers; JSON-LD context compilation and expanded/caller/derived modes are reported separately. |
| `crates/rdf/benches/projections.rs` | Graph, tabular, dataset-description, and research-object mapping/carrier throughput plus LPG scope and materialized-package/direct-sink allocation comparisons over deterministic fixtures. |
| `crates/sparql-algebra/benches/tokenize.rs` | Lexer throughput on long IRI bodies, escaped string literals, and comment tails. |
| `crates/sparql-eval/benches/query_eval.rs` | End-to-end SPARQL SELECT latency including BGP joins, filters, and aggregates. |
| `crates/sparql-eval/benches/cost_based_bgp_planner.rs` | Planner regression watch: cost-based BGP ordering vs. the retired structural heuristic. |
| `crates/sparql-eval/benches/exists_decorrelation.rs` | `FILTER NOT EXISTS` inner-pattern re-evaluation and index-rebuild cost with/without memoization. |
| `crates/sparql-eval/benches/lateral_service.rs` | `SERVICE ?g` LATERAL substitute-and-forward cost as the number of distinct endpoint bindings grows. |
| `crates/shapes/benches/validate.rs` | SHACL Core validation latency plus JSON Schema/LinkML → SHACL import/lowering throughput and allocation traffic on deterministic fixtures. |
| `crates/shapes/benches/schema_surface.rs` | RDFC-keyed shaped-only compilation and sparse/dense ontology-complete class/property relation plus JSON Schema/OpenAPI emission. |
| `crates/entail/benches/chase.rs` | RDFS materialization scaling on subclass chains, measured through the whole `materialize` path: clause-program lowering plus `purrdf-datalog`'s semi-naive fixpoint. |
| `crates/entail/benches/classify.rs` | OWL-Direct classification latency over a synthetic `EL` terminology at three signature sizes; classification cost is superlinear in the signature rather than the axiom count. |
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

### SHACL validation hot paths

The `validate` benchmark contains four deterministic SHACL workloads:

| Group | Fixed fixture and measured boundary |
| --- | --- |
| `shacl_validate/corpus_all` | All 70 committed first-party conformance cases, including text ingestion, shapes parsing, target resolution, constraint evaluation, and report assembly. |
| `shacl_focus_core` | 512, 1,024, 2,048, 3,000, 100,000, and 1,000,000 target nodes. Each node contributes four quads; the shapes exercise a 40-level asserted subclass hierarchy, pattern, datatype, and class constraints. |
| `shacl_focus_sparql` | 64, 512, and 4,096 target nodes with two quads per node and a caller-declared SHACL-SPARQL function. |
| `shacl_focus_realtime` | One prepared 1,000,000-node snapshot (4,000,079 quads and 3,000,088 terms), a compatibility focus filter over one node, and id-native prepared requests containing 1, 8, 64, 512, or 4,096 focus nodes. Dataset and shapes preparation stays outside each request's timed loop. |

Run the complete suite, one group, one large bulk case, or the largest bounded
request with:

```sh
cargo bench -p purrdf-shapes --bench validate --locked
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_focus
cargo bench -p purrdf-shapes --bench validate --locked -- shacl_focus_core/1000000
cargo bench -p purrdf-shapes --bench validate --locked -- 'shacl_focus_realtime/prepared_ids/4096'
```

Appending `--test` performs a single-sample smoke run and prints the fixture,
thread, elapsed-time, allocation-call, and requested-byte probe. It is useful for
correctness and allocation inspection, not a substitute for Criterion's sampled
estimates.

`PreparedValidator` is the realtime surface. Preparation owns the immutable
projected snapshot and parsed shapes, precomputes class closures and target
identities, and evaluates SHACL-SPARQL targets once. Callers that already hold
the projected snapshot should use `from_projected_dataset` and pass that exact
snapshot's dataset-local `TermId` values to `validate_focus_node_ids`. The
compatibility focus-filter entry point must enumerate whole target sets before
discarding unrelated nodes and is intentionally retained as a comparison, not
as the realtime path. Publishing an overlay or replacement snapshot requires a
new prepared validator.

The production scheduler preserves serial semantics. At 1,024 or fewer focus
nodes it stays serial. Above that boundary, a multi-threaded build uses indexed
Rayon chunks with a 64-node minimum and approximately four chunks per worker.
Each chunk evaluates in canonical source order; chunk outputs are reduced in
that same order, so report bytes and the selected earliest hard error do not
depend on worker timing. Canonical focus sorting also stays serial below 4,096
nodes and uses deterministic stable parallel sorting for larger sets. Unit tests
force serial and parallel execution across all 70 corpus cases, 2- and 4-worker
pools, several chunk geometries, SHACL-AF user functions, and competing hard
errors; a separate test proves genuine four-worker participation.

The following selected one-operation probes were observed on 2026-07-20 with
rustc 1.97.1, Linux 7.1.4, 32 Rayon workers, and an AMD Ryzen AI MAX+ 395. They
are report-only host observations, rounded from the emitted probes:

| Operation | Earlier baseline | Current observation |
| --- | ---: | ---: |
| Whole-bundle Core validation, 1,000,000 focus nodes | 14.143 s | 339.85 ms |
| SHACL-SPARQL validation, 4,096 focus nodes | 1.892 s | 79.05 ms |
| Prepare the 1,000,000-node projected snapshot | not available | 2.31 ms; 119 allocation calls; 14,940 requested bytes |
| Prepared id-native validation, 4,096 focus nodes | not available | 1.352 ms |
| Compatibility focus filter, one retained node | not available | 266.93 ms |

The comparison records the workload evolution rather than promising a speedup:
reproduce it on the target deployment hardware, with the consumer's actual
shapes and affected-focus expansion, before choosing latency budgets.

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

### Graph, tabular, dataset-description, and research-object projections

`crates/rdf/benches/projections.rs` builds six deterministic `example.org`
datasets without RNG: a 600-quad general graph, a 12,000-quad graph split evenly
across 20 named graphs, a 600-quad OBO/OWL graph, an 800-quad SKOS source graph,
the 29-quad research-object intersection, and a 10-quad, four-graph VoID source.
The small canonical LPG projection contains 408 nodes plus edges. The large
scope comparison either retains all 20 graphs or scans the same trust boundary
while retaining one 600-quad graph. Criterion measures complete
mapping/serialization/parser operations, all four large LPG
materialized-package/direct-sink pairs, mapped and bounded-CONSTRUCT DCAT RDF
archive generation over the research fixture, and VoID archive generation;
fixture construction and strict profile-config parsing stay outside timed
loops. A counting global allocator also reports calls and requested bytes for
representative single operations. Attached RO-Crate write/read cases include
payload validation, metadata, preview, canonical USTAR construction, and strict
lift validation. Those counts are cumulative allocation traffic, not retained
or peak memory.

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

The native dataset-description slice was measured separately on 2026-07-19
with rustc 1.97.1, Linux 7.1.4, and the same AMD Ryzen AI MAX+ 395. The
production archive path includes mapping or CONSTRUCT evaluation, native Turtle
serialization, package validation, and canonical USTAR encoding. Strict config
parsing remains outside the timed loop. The mapped and CONSTRUCT cases scan the
29-quad research-object source; the VoID case scans its 10 quads across the
default, data, alignment, and metadata graphs. Throughput is normalized to
source quads, while VoID's input limit additionally charges named-graph and RDF
1.2 statement-layer records.

| Operation | Archive bytes | Time | Throughput | Allocation calls | Requested bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| mapped DCAT RDF | 5,632 | 144.85 µs | 200.20 Kquad/s | 2,512 | 219,001 |
| CONSTRUCT DCAT RDF | 2,048 | 15.436 µs | 1.879 Mquad/s | 134 | 13,913 |
| VoID generation | 14,336 | 197.34 µs | 50.674 Kquad/s | 3,213 | 394,387 |

The attached RO-Crate slice was measured separately on 2026-07-19 with rustc
1.97.1, Linux 7.1.4, the same processor, and 10 Criterion samples. The fixture
is the 29-quad research object plus one three-byte payload:

| Operation | Time | Throughput | Allocation calls | Requested bytes |
| --- | ---: | ---: | ---: | ---: |
| RO-Crate 1.3 attached write | 48.42 µs | 599 Kquad/s | 1,153 | 123,470 |
| RO-Crate 1.3 attached read | 127.01 µs | 228 Kquad/s | 3,539 | 452,528 |

The large LPG scope and sink extension was measured on 2026-07-19 with rustc
1.97.1, Linux 7.1.4, and the same AMD Ryzen AI MAX+ 395. The fixture contains
12,000 quads split evenly across 20 named graphs. The selective case scans the
same 12,000-quad trust boundary and retains one 600-quad graph. The carrier rows
all project the explicit-all fixture; the sink is an in-process discard sink, so
the comparison includes encoding and callback dispatch but no storage I/O.

| Operation | Input policy | Time | Throughput over records scanned |
| --- | --- | ---: | ---: |
| Canonical LPG, explicit all | 20 graphs / 12,000 quads retained | 142.94 ms | 83.95 Kquad/s |
| Canonical LPG, selective | 12,000 quads scanned / 600 retained | 5.06 ms | 2.37 Mquad/s |
| Generic CSV package | explicit all / materialized | 193.85 ms | 61.90 Kquad/s |
| Generic CSV sink | explicit all / discard sink | 219.62 ms | 54.64 Kquad/s |
| Neo4j CSV package | explicit all / materialized | 248.10 ms | 48.37 Kquad/s |
| Neo4j CSV sink | explicit all / discard sink | 218.43 ms | 54.94 Kquad/s |
| openCypher package | explicit all / materialized | 257.62 ms | 46.58 Kquad/s |
| openCypher sink | explicit all / discard sink | 244.62 ms | 49.06 Kquad/s |
| GraphML package | explicit all / materialized | 306.88 ms | 39.10 Kquad/s |
| GraphML sink | explicit all / discard sink | 246.94 ms | 48.59 Kquad/s |

One-operation cumulative allocation traffic from that run was:

| Mapping scope | Allocation calls | Requested bytes |
| --- | ---: | ---: |
| Explicit all (20 graphs) | 1,150,971 | 119,746,351 |
| Selective (one graph) | 80,928 | 6,991,783 |

| Carrier | Package calls / bytes | Sink calls / bytes | Requested-byte reduction |
| --- | ---: | ---: | ---: |
| Generic CSV | 1,801,685 / 197,732,907 | 1,801,762 / 172,580,710 | 12.7% |
| Neo4j CSV | 1,918,106 / 207,065,143 | 1,918,131 / 181,972,457 | 12.1% |
| openCypher | 1,849,539 / 229,176,302 | 1,849,751 / 187,501,253 | 18.2% |
| GraphML | 1,957,891 / 254,192,805 | 1,958,303 / 185,519,770 | 27.0% |

These are allocation-traffic observations, not retained or peak-memory
measurements. Mapping the bounded canonical graph dominates the call count, and
chunk/progress dispatch can add small calls. The requested-byte reductions are
consistent with the direct sink avoiding complete artifact bodies and a USTAR
buffer; they do not imply that every sink is faster, as the generic CSV timing
demonstrates.

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
`benchmarks` workflow (`.github/workflows/benchmarks.yaml`). That workflow
produces `bench_compat.json` and uploads it alongside the Criterion artifacts.
It uses `continue-on-error: true`, so it never fails anything.

It runs on a **weekly schedule plus `workflow_dispatch`**, not on every push. A
~2 h report-only job on the per-push path burned a runner per commit for a
result nothing gates on, and — because it competed with the gating jobs for the
same shared runners — GitHub auto-cancelled a full run, so the numbers were not
collected either. A scheduled run on an uncontended runner is better data as
well as cheaper. To benchmark a specific branch, dispatch the workflow against
it rather than waiting for Sunday.

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

## Scale corpus (`purrdf-scale-mixed-v1`)

The other two layers measure code. This one produces **input**: a deterministic,
shardable N-Quads corpus whose IRI shapes are deliberately adversarial, so a
capacity number measured over it cannot have been flattered by a corpus that
front-codes perfectly. It times nothing and gates nothing. What it gives a
reader is the ability to regenerate, byte for byte, the exact corpus a capacity
claim was measured over.

The generator is `crates/bench` (`bench-corpus`, unpublished tooling); the lane
that drives it across shards is `scripts/scale-corpus.sh`, run as
`make scale-corpus`. It is deliberately **not** part of `make bench`: criterion
suites are a different layer, and this one produces bytes rather than timings.

```sh
make scale-corpus                                   # stream 10^6 rows over 8 shards, keep nothing
make scale-corpus SCALE_QUADS=10000000 SCALE_SHARDS=32
make scale-corpus SCALE_MODE=pipe | your-loader     # ordered whole run, nothing stored
make scale-corpus SCALE_MODE=files SCALE_OUT=/mnt/big/corpus
```

`SCALE_MODE=pipe` puts corpus bytes on standard output, so **nothing may share
that stream**. The repository's `Makefile` sets `MAKEFLAGS +=
--no-print-directory` at its top for exactly this reason, which makes the
payload byte-exact at any recursion depth with **no cooperation required from
the caller**: `$(MAKE) scale-corpus SCALE_MODE=pipe | your-loader` from inside
another `Makefile` produces the same bytes as a plain shell invocation. The
setting is repository-global, so every other target loses its directory banners
too.

The background, because the trap is easy to misdiagnose elsewhere: whenever
`-w`/`--print-directory` is in effect, `make` writes `make: Entering directory
'...'` (and a matching `Leaving directory` line) to standard output before the
recipe runs. That flag can be inherited explicitly via a propagated
`MAKEFLAGS`, but GNU make also turns it on **automatically** for any invocation
it detects as a sub-make — i.e. whenever `MAKELEVEL` in the environment is
already nonzero — regardless of what `MAKEFLAGS` says; the banner then reads
`make[1]: Entering directory ...` (the bracketed number is the recursion depth).
A real nested `$(MAKE)` shows an **empty `MAKEFLAGS` with `MAKELEVEL=1`**, and
the banner appears anyway, so checking `MAKEFLAGS` and finding it clean proves
nothing. Piped into a loader, that banner arrives as a corrupt first line the
loader cannot parse as a corpus row. A wrapper around some *other* `Makefile`
that lacks the global setting has to pass the flag itself, e.g.
`$(MAKE) --no-print-directory some-lane | your-loader`.

### Parameters

Every knob is an overridable `make` variable, in the same style as `BENCH_ARGS`.

| Variable | Default | Meaning |
| --- | --- | --- |
| `SCALE_QUADS` | `1000000` | Total rows across all shards. One slot emits exactly one line, so this is also the line count. |
| `SCALE_IRIS` | `100000` | The entity **index space**: entity IRIs are minted from indexes `0..iris`, drawn with a `sqrt`-CDF skew. A target, **not** an achieved distinct-entity count — see below. |
| `SCALE_SEED` | `1592642302` | The `splitmix64` seed folded into every derivation (the generator's own default, in decimal). |
| `SCALE_SHARDS` | `8` | How many independent shards the row sequence is cut into. |
| `SCALE_MODE` | `stream` | `stream` (parallel shards, each piped to a sink, nothing retained), `pipe` (ordered whole run on stdout), `files` (opt-in materialization). |
| `SCALE_OUT` | *(unset)* | Output directory; **required** by `SCALE_MODE=files`, which refuses to run without it. Absolute or relative; a relative path is relative to where the lane runs. |
| `SCALE_SINK` | *(unset)* | Replaces the built-in digest sink in `stream` mode with any command that reads standard input. |
| `SCALE_MANIFEST` | *(unset)* | Writes the manifest to this path instead of the mode's default destination. |
| `SCALE_BIN` | *(unset)* | A prebuilt `bench-corpus` to use instead of building one. Checked up front — a path that does not exist, is not a regular file, is not executable, or cannot produce the whole-run manifest fails the lane by name before any shard starts. |

Every knob above except `SCALE_SINK` is **passed through literally**. `make`
hands each one to the lane script as environment bytes rather than
interpolating it into a recipe line, so a path is used exactly as typed —
spaces, `$`, backticks and quotes are all just characters in a filename, none
of them is expanded, and no shell ever parses them. A path the lane cannot open
is a hard failure that quotes the bytes it used; the lane never writes
somewhere else and reports success. The same holds for the `LUBM_*` and
`WATDIV_*` knobs below.

`SCALE_SINK` is the one deliberate exception, because it **is** a command: it
replaces the built-in digest sink and is therefore executed by a shell. It is
syntax-checked once, before any shard runs, and a value that cannot be run
fails the lane with a diagnostic naming `SCALE_SINK` rather than a bare shell
error.

### What a capture records

A capture of this lane is **the manifest plus the digest of the output** —
neither half is evidence on its own. The manifest (`bench-corpus --manifest`,
emitted by every mode) **records**; it computes no digest of anything. It
carries the profile id, the seed, the quad and IRI parameters, the shard's row
range, the shard's exact `emitted_lines` count, and both mixes:

* `entity_class_mix_per_mille` over the entity **index space** (plain 400,
  numeric-long 200, chinese 200, irregular 150, very-long 50), and
* `row_mix_per_mille` over the **emitted rows** (entity-edge 500,
  plain-literal 150, zh-literal 100, typed-literal 100, long-text-literal 50,
  reified 60, blank-node 40).

Those are **different axes** and the key names say so. An entity's class is a
function of its *index*, and slots draw indexes under the `sqrt`-CDF skew, so
the class distribution measured over rows is not the pinned entity-space
distribution. A capture that recorded an unqualified "class mix" as its corpus
description would misreport what it measured.

The digest says which bytes were actually consumed. A manifest without a digest
describes a corpus nobody proved was produced; a digest without a manifest is a
number with no parameters attached.

### `--iris` is a target, not an achieved distinct count

`SCALE_IRIS` (the generator's `--iris`) is the size of the index space entities
are drawn from. It is **not** the number of distinct entities a run produced,
and no artifact this lane emits reports that number.

The achieved distinct-entity count is at most `min(iris, emitted_lines)` and in
practice well below both, because the skew concentrates draws on the head of
the space and because only some row kinds name two entities. Measured over the
emitted bytes at `--quads 200000 --iris 100000` with the default seed: the
corpus names **89,569** distinct entity IRIs against an index space of 100,000,
and only **75,548** of those ever appear in row-subject position. Which of
those two a "distinct entity count" means is itself a choice a claim has to
state.

Nothing here computes the achieved count on purpose: establishing it means
enumerating the corpus, which is exactly what streaming at full scale exists to
avoid — at 10^10 rows there is no pass over the output to spend. So a capacity
claim made from this lane **must name which number it is about**: the entity
index space that was configured (`iris`, recorded in the manifest), or a
distinct-entity count (which a consumer that ingested the stream can report,
and which is a different number).

### Density, and why full scale is streamed rather than stored

Measured, not estimated, at `--quads 2000000 --seed 31337` across a run of
`SCALE_IRIS` values: density is a **function of the index space**, not a
single number, because a larger entity space means longer indexes inside the
minted IRIs (the zero-padded numeric-long class and the host-scattered
irregular class both carry the index itself). That figure is a property of the
profile and its parameters rather than of the host — the same specification
produces the same bytes on every target.

| `SCALE_IRIS` | Bytes | Bytes per row |
| ---: | ---: | ---: |
| 10^5 | 351,119,043 | 175.560 |
| 10^6 | 354,824,007 | 177.412 |
| 10^7 | 357,694,926 | 178.847 |
| 10^8 | 360,269,245 | 180.135 |
| 10^9 | 362,774,790 | 181.387 |
| 10^10 | 365,534,901 | 182.767 |

The spread across those six decades of index space is **7.2 bytes per row**,
not "a byte or two" — roughly 1.4 bytes per row per decade of `SCALE_IRIS`.
The lane prints the density it actually observed on every run, so a change to
the mixes shows up in the report instead of silently invalidating this table.

The arithmetic that follows pairs each row count with an index space of the
same order of magnitude — the pairing a run sized to keep a meaningful,
non-repeating entity space would use — and applies that row's OWN measured
density rather than one flat number carried up from a smaller scale:

| Rows | `SCALE_IRIS` | Density (B/row) | Approximate N-Quads bytes |
| ---: | ---: | ---: | ---: |
| 10^6 | 10^6 | 177.412 | 177 MB |
| 10^9 | 10^9 | 181.387 | 181 GB |
| 10^10 | 10^10 | 182.767 | **1.83 TB** |

So a run at 10^10 rows over a 10^10-entity index space is a **1.83 TB**
corpus. That is an operator-driven, off-CI activity, and it should be
**streamed into whatever consumes it rather than stored**: `SCALE_MODE=pipe`
hands a loader the same bytes an unsharded run would have produced, and the
default `stream` mode hands each shard to a sink and keeps nothing at all.
`SCALE_MODE=files` exists for the operator who has a filesystem that can hold
the run and a reason to keep it; it is opt-in, it demands an explicit
`SCALE_OUT`, and no continuous-integration runner has the disk for it.

### Sharding: what is a property of the algorithm, and what is not

Every IRI is minted **purely from its index** under a fixed seed. Nothing in
shard `k` depends on anything shard `j` computed, so the shards are independent
processes over disjoint slices of one row sequence and need no coordination —
no shared dictionary, no ordering barrier, no merge step. That independence is a
property of the **algorithm**, and it holds at any scale, because it is a
statement about what the generator reads (an index) rather than about how big
the run is.

What has actually been *run* is a different and smaller claim, and the two
should not be confused. The driver runs shards concurrently, and byte-exact
stitching is verified at sizes a disk can hold: a whole unsharded run and a
sharded run of the same specification produce identical bytes, both through
`SCALE_MODE=pipe` and through `cat` of the `SCALE_MODE=files` output. A
10^10-row run over a 10^10-entity index space has a 1.83 TB storage
requirement and is nobody's smoke test; the table above is the honest reason
it is described as arithmetic rather than reported as a demonstration.

In `files` mode each shard is written as:

```
purrdf-scale-mixed-v1.seed<SEED>.quads<QUADS>.iris<IRIS>.shard-00003-of-00016.nq
```

The shard index and shard count are zero-padded to at least five digits (wider
counts widen the field), so the files sort lexicographically into shard order
and `cat <prefix>.shard-*.nq` reproduces a whole run byte for byte. The
whole-run manifest is written beside them as `<prefix>.manifest.json`, and each
shard gets its own `<shard>.manifest.json` recording that shard's row range.

That guarantee is only worth anything if a run that did not produce every shard
cannot claim it, so **any shard that fails fails the whole run**, in every mode.
The lane names the failing shards (`FAILED shards: 1`), exits non-zero, prints
no shard sizes and no byte-for-byte guarantee, and leaves no
`<shard>.manifest.json` certifying a shard whose corpus file was not written —
including one left behind by an earlier, successful run into the same
directory. A shard manifest is a certificate; it never outlives the shard it
certifies, and neither does the whole-run manifest: a failed run removes both,
and says so.

**Every mode counts what actually left the lane**, and the run fails unless that
count is the row count its manifest certifies. The manifest is produced by asking
the binary what it intends to emit, so on its own it is a claim; the count is the
evidence. In `stream` the built-in sink reports it, in `files` each shard file is
measured, and in `pipe` and under `SCALE_SINK` the payload is copied through a
counter on its way out — the two paths where the bytes leave for something
outside the lane, and where the only place to count them is in passing. A run
that delivers nothing, delivers less than the manifest says, or stops mid-row is
a failed run, and no manifest is published for it.

### What the corpus covers, and what it does not

It covers, at pinned shares: five IRI classes chosen to defeat a single
dictionary trick (front-codable plain, 36-digit zero-padded numerics beyond
machine integer widths, raw-Han Chinese, host-scattered irregular with
reserved-octet escapes, and very-long at ~628 bytes); entity-to-entity edges;
plain, language-tagged (`@zh`), long-text and `xsd:`-typed literals over four
datatypes with lexical forms valid for their datatype; blank nodes in subject
and object position; RDF 1.2 reifier rows binding a triple term; 8 predicates;
and a sixth of rows spread over 16 named graphs, drawn independently of row
kind so literals appear inside named graphs at the corpus-wide rate. The entity
draw is skewed (`sqrt`-CDF), so the corpus has hot subjects rather than a flat
distribution.

It does **not** cover: any workload. There is no query mix, no update stream,
no schema or shapes, and no ingest timing here — this layer generates bytes, and
the harness that consumes them is what records a capacity number. It is also a
single synthetic profile: its mixes are pinned constants chosen to be
adversarial, not a model of any real dataset's shape, and a corpus of your own
data remains the only thing that answers a question about your own data.

### Continuous integration

`.github/workflows/benchmarks.yaml` runs a **smoke** of this lane — 10^6 rows
(~177 MB) streamed through a digest with nothing retained — before the long
bench steps, so a generator that stopped working is reported in a minute rather
than after two hours. It is a liveness check on the generator and its driver,
not a measurement, and like everything else in that workflow it does not gate a
merge. Full scale never runs there: the runners do not have the disk, and a step
that wrote a large file would be a bug in this document's arithmetic, not a
better test.

## LUBM comparison workload

The first three layers compare PurRDF against PurRDF. This one compares it
against the workload the OWL knowledge-base literature actually publishes
numbers for: the **Lehigh University Benchmark**, a synthetic university domain
with a data generator, an OWL ontology, and 14 queries chosen so that most of
them have *no answers at all* without inference.

Run it with `make lubm`. It is **report-only** — like every other layer here it
gates nothing, asserts nothing, and prints what it measured on the host it ran
on.

```sh
make lubm                                  # LUBM(1, 0), seed 0 — ~103k triples
make lubm LUBM_UNIVERSITIES=10             # a larger corpus
make lubm LUBM_SEED=7 LUBM_INDEX=0         # a different draw
```

Cite, in anything derived from it:

> Y. Guo, Z. Pan and J. Heflin. "LUBM: A Benchmark for OWL Knowledge Base
> Systems." *Journal of Web Semantics* 3(2).

### Licence posture: everything is run, nothing is vendored

No LUBM byte lives in this repository, and that is a licensing conclusion rather
than a tidiness preference.

* The **UBA data generator is GPL-2.0-or-later**. This tree is MIT OR
  Apache-2.0, so the generator is **run, never copied in**: vendoring it would
  place a copyleft work inside a permissively licensed distribution. Running a
  GPL program to produce data is not distribution of that program, and the data
  is what the benchmark consumes.
* **`univ-bench.owl` and `queries-sparql.txt` carry no licence grant at all** —
  no copyright line, no rights statement, and none on the project page. Absent
  an explicit grant there is no permission to redistribute, so both are fetched
  by digest at the moment of use and left in an ignored cache. A *mechanically
  normalised* copy of the query file is still a copy of it, so the normalised
  queries are build output too, never tracked files.

`scripts/benchmark-acquire.py` fetches all of it into `target/bench-artifacts/`,
verifying every byte against a pinned digest, and refuses to run at all unless
the repository's own ignore rules already make that cache uncommittable.
`python3 scripts/benchmark-acquire.py --list` prints each artifact's terms.

### The generator writes to the wrong directory on Linux

Stock UBA builds its output path as `user.dir + "\" + name` — a **Windows**
separator. On Linux nothing splits that backslash, so the last `/` in the string
is the one before the working directory's own name, and the files land in the
**parent** of the working directory, named `work\University0_0.owl` with a
literal backslash inside the filename.

The file *contents* are valid RDF/XML; only the name is wrong. The lane
therefore **renames the output after generation** instead of patching
`Generator.java`, which keeps the GPL source unmodified and un-vendored, needs
no Java compiler (the artifact ships prebuilt `classes/`, and a JRE is enough),
and is checkable — the lane counts what it renamed and fails if nothing
appeared. Each run generates inside its own directory, so concurrent runs cannot
collide.

### Determinism, and the one place it was not free

UBA accepts `-index` and `-seed`, and the same pair reproduces the datasets the
LUBM papers use. Conversion needed one fix to inherit that. Every generated file
opens with `<owl:Ontology rdf:about="">` — an **empty relative IRI**, which
resolves against the document base. Left to default that base is the input
file's own `file://` path, so the converted N-Quads embedded the scratch
directory and two runs in differently named directories differed.

`--base` is therefore passed explicitly, built from the file's *name* only. It
affects exactly two triples per file — the document's own `rdf:type
owl:Ontology` and its `owl:imports` — and none of the 14 queries touches either.
`LUBM_DOC_BASE` defaults to an `example.org` IRI: RFC 2606's reserved
documentation domain and this repository's fixture convention, standing in for
the publication IRI a locally generated corpus does not have. An operator who
publishes a corpus sets it to where that corpus actually lives.

### Entailment regimes are part of the query, not metadata about it

This is the part of LUBM that is easiest to get quietly wrong.

LUBM **never asserts** `Student`, `Professor` or `Chair`. Those memberships are
derived — `univ-bench.owl` defines `Student` by `owl:intersectionOf`, makes
`subOrganizationOf` an `owl:TransitiveProperty`, and declares `hasAlumnus` the
`owl:inverseOf` of `degreeFrom`. So an engine that applies no inference answers
eleven of the fourteen queries `0`, instantly, and would **win** any comparison
that ignored the regime.

> **Compare two engines on a query only when both answered it under the same
> regime, over the same data.** A result count is meaningless without the regime
> it was produced under. That rule is printed in the lane's own report, next to
> the numbers it governs.

Each query's regime is taken from the canonical description in the *Journal of
Web Semantics* paper and corroborated against the axioms in `univ-bench.owl`:

| Query | Regime it requires | `--entailment` |
| --- | --- | --- |
| Q1, Q2, Q14 | No inference | *(none)* |
| Q3, Q4 | `subClassOf` | `rdfs` |
| Q5 | `subClassOf` + `subPropertyOf` | `rdfs` |
| Q6–Q10 | The derived `GraduateStudent`-to-`Student` membership | `owl-rl` |
| Q11 | Transitive property (`subOrganizationOf`) | `owl-rl` |
| Q12 | Realization of the defined class `Chair` | `owl-rl` |
| Q13 | `inverseOf` + `subPropertyOf` | `owl-rl` |

### The queries are normalised mechanically, and the normalisation is recorded

`queries-sparql.txt` predates the final SPARQL 1.1 Recommendation and does not
parse: it separates projection variables with commas, writes two IRIs without
angle brackets, separates one triple pattern's terms with commas, and binds
`ub:` to a 2004 draft namespace that no generated dataset has ever carried —
which is the dangerous one, because a query in the wrong namespace does not
fail, it silently answers zero.

`scripts/lubm-queries.py` fixes all of that with five numbered, mechanical rules
and writes `provenance.txt` recording **every application with its before and
after text**, so a reader can audit that each query still asks what Lehigh
published. Nothing is hand-rewritten. Its `--self-test` asserts the rules'
*scope* as well as their effect — that the comma rule touches Q7 and only Q7,
that no projection gained, lost or reordered a variable — because a
"normalisation" that quietly became a rewrite is the failure mode that would
make every number downstream worthless.

### The dataset ladder, and the ceiling that makes it necessary

Materializing an entailment closure passes through a **fixed internal ceiling**
that no command-line flag raises. On this workload it bites well below one
university, so the lane probes each regime against progressively smaller rungs —
the full corpus, then one generated file, then a slice — and reports the first
that closes, printing the observed and permitted counts verbatim for each rung
that did not. The limit is therefore visible in the output rather than inferred
from a missing row.

Every reported row names the rung it was answered over, and the report states
the rule plainly: **a row count on a rung below `full` is not the published LUBM
answer.** It is the answer over a strict subset, so a query whose matching
individuals fall outside that subset legitimately reports `0`. Those counts
establish that the regime works and what it costs; only `full` rows are
comparable against a published LUBM figure.

### Parameters

Every knob is an overridable `make` variable, in the same style as `SCALE_*`.

| Variable | Default | Meaning |
| --- | --- | --- |
| `LUBM_UNIVERSITIES` | `1` | Universities to generate (UBA's `-univ`). One is ~103k triples. |
| `LUBM_SEED` | `0` | UBA's `-seed`. With `-index`, fixes the corpus. |
| `LUBM_INDEX` | `0` | UBA's `-index`, the starting university id. |
| `LUBM_ONTO` | Lehigh's ontology IRI | The `-onto` IRI stamped into the data, and the namespace `ub:` is rebound to. |
| `LUBM_DOC_BASE` | `http://example.org/lubm/` | Base for each document's own two header triples. |
| `LUBM_ENTAIL_SLICE` | `3000` | Triples in the smallest rung of the entailment ladder. |
| `LUBM_OUT` | `target/lubm` | Where the lane works. An absolute path is used verbatim; a relative one resolves against the repository root, so the default keeps everything the lane writes inside `target/` as build output. |
| `LUBM_BIN` | *(unset)* | A prebuilt `purrdf` to use instead of building one. |

### What it covers, and what it does not

It covers a real, published, externally defined workload end to end: generation,
conversion **through PurRDF's own CLI**, and query evaluation under each query's
own regime, with per-query timings and result counts.

It does **not** cover: any other engine. The lane measures PurRDF and prints the
regime and rung each number belongs to — which is what makes a comparison
*possible* — but running another store and putting the two side by side is the
operator's job, and the regime rule above is the thing that makes such a
comparison honest rather than flattering. It is also not a conformance check:
that a query returned *n* rows under `owl-rl` is a measurement, not a claim that
the answer is complete under that regime.

## WatDiv comparison workload

The **Waterloo SPARQL Diversity Test Suite** is the other workload the RDF-store
literature publishes numbers for, and it asks a different question from LUBM.
LUBM asks what an engine can *derive*. WatDiv asks how a planner copes with
query **shape and selectivity**: its 20 basic query templates are deliberately
spread across four structural families — three complex (`C`), five snowflake
(`F`), five linear (`L`) and seven star (`S`) — over a dataset whose predicate
and object distributions are deliberately skewed, so a planner that only handles
uniform data has nowhere to hide.

Run it with `make watdiv`. It is **report-only** — like every other layer here
it gates nothing, asserts nothing, and prints what it measured on the host it
ran on.

```sh
make watdiv                      # the frozen 10M dataset, seed 0
make watdiv WATDIV_SEED=7        # a DIFFERENT WORKLOAD, not a re-run
```

Cite, in anything derived from it — this one is not optional, see below:

> G. Aluç, O. Hartig, M. T. Özsu and K. Daudjee. "Diversified Stress Testing of
> RDF Data Management Systems." In *Proc. The Semantic Web - ISWC 2014 - 13th
> International Semantic Web Conference*, 2014, pages 197–212.

### Licence posture: citation-ware, so the citation is a condition of use

WatDiv's published terms are, in substance, that *provided you include a
citation to the ISWC 2014 paper, you are free to download and use the WatDiv
Data and Query Generator*, supplied "as is" with all use at your own risk.

That has two consequences and this repository observes both.

* The grant is a **use** grant, not a **redistribution** grant. So no WatDiv
  byte lives in this tree — not the toolkit, not the dataset, not the query
  templates — and neither do the queries the lane derives from those templates,
  because a query mechanically derived from a template is still derived from it.
  Everything is fetched by digest into `target/bench-artifacts/` and every file
  the lane writes is build output.
* **The citation is a condition, not a courtesy.** Publishing a number derived
  from this lane without that citation is using WatDiv outside the terms it was
  offered under. The lane prints the citation at the end of every run, and
  `provenance.txt` carries it too, so it travels with the numbers rather than
  living only here.

### The generator cannot be pinned, so a frozen output is pinned instead

This is the load-bearing difference from the LUBM lane, which *runs* its
generator.

Stock WatDiv v0.6 seeds itself from the wall clock and the operating system's
entropy source and exposes **no seed flag of any kind**: `src/model.cpp` builds
its Boost generator as `boost::mt19937(static_cast<unsigned>(time(0)))` and
calls `srand(time(NULL))` again inside the generator, `src/statistics.cpp` calls
`srand(time(NULL))`, and `src/volatility_gen.cpp` constructs an `mt19937` from
`random_device`. Two runs of the same binary over the same model file therefore
produce different data. (It would not build unmodified today in any case — it
calls `std::random_shuffle`, which C++17 removed.)

So **pinning the WatDiv tarball pins the generator's source and nothing else**.
Treating a WatDiv run as reproducible because its source tarball is pinned is a
false claim about the benchmark. The only reproducible WatDiv dataset is one
that was generated **once** and then itself pinned by digest — a frozen
*output*. Upstream publishes exactly those, and
`scripts/benchmark-acquire.py` pins one:

| | |
| --- | --- |
| Artifact | `watdiv.10M.tar.bz2` (contains `watdiv.10M.nt` and `saved.txt`) |
| Size | 58 558 746 bytes |
| SHA-256 | `1d0a8a4725c98974eb7347ce3e6d9cab44f9f40389589809674254151b745af6` |
| Triples | 10 916 457 |

**The generator is never built and never run here.** Upstream publishes no
checksum beside its frozen datasets, so that size and digest are *ours*, taken
from the bytes served on 2026-09-18 — exactly as the LUBM pins are ours. There
is no publisher checksum to cross-check them against, so the artifact's `md5`
field is `None` rather than invented.

Upstream also publishes `watdiv.100M.tar.bz2` and `watdiv.1000M.tar.bz2`. They
are **not** pinned. `WATDIV_SCALE` refuses them by name and says why: using one
means fetching and hashing it yourself and adding it to the artifact table. A
lane that quietly fell back to 10M would report a number for the wrong corpus
under the right name.

### Instantiation is deterministic here, which upstream's cannot be

A WatDiv template is not a query. Each carries `#mapping` directives and `%vN%`
placeholders that something must fill in:

```text
#mapping v1 wsdbm:Website uniform
SELECT ?v0 ?v2 ?v3 WHERE {
    ?v0  wsdbm:subscribes  %v1% .
    ?v2  sorg:caption      ?v3 .
    ?v0  wsdbm:likes       ?v2 .
}
```

Upstream's own instantiator draws from the same time-seeded generators, so its
query sets cannot be regenerated either — a published WatDiv number whose
queries came out of it is not reproducible by the person reading it.

Freezing the dataset fixes that, because it fixes the **candidate set** behind
every mapping: the candidates are a property of those exact bytes.
`scripts/watdiv-queries.py` therefore instantiates the placeholders itself, as a
pure function of the dataset and a seed:

1. Collect the candidates for each mapped type in **one pass** over the frozen
   dataset. WatDiv names its entities `<namespace><Type><decimal>`, so a
   candidate is a term in subject or object position whose IRI has exactly that
   shape. Nothing is inferred from `rdf:type`: WatDiv asserts a type triple for
   products and users but not for websites, topics or cities, so a type-triple
   rule would find candidates for some mappings and none for others.
2. Sort each set into a canonical order, by UTF-8 bytes, so the choice never
   depends on the order the file happened to mention a term in.
3. Select with **`splitmix64`** over `WATDIV_SEED` and a pinned per-mapping
   stream — the same arithmetic-only idiom `crates/bench/src/lib.rs` uses, with
   no RNG syscalls and no platform floats. Each mapping draws from its own
   stream, so no two are correlated and adding a template cannot shift another
   template's choice.
4. Record every choice — which template, which mapping, which candidate, and
   **out of how many** — in `provenance.txt`, together with the seed and the
   dataset digest.

The result is that **the same frozen dataset and the same seed reproduce the
queries byte for byte**, and the lane prints a digest over the emitted query set
so that is checkable rather than merely claimed. This is a deliberate
improvement on upstream's time-seeded instantiation, not a reimplementation of
it.

> **A query set from a different seed is a DIFFERENT WORKLOAD.** Two numbers
> taken under two seeds compare two workloads, not two engines. The seed is
> printed in the lane's summary next to the numbers it governs, for the same
> reason the LUBM lane prints each row's entailment regime.

`uniform` is implemented as *actually* uniform — the draw is rejected and
retaken when it lands in the short tail plain modulo would fold unevenly. At
these candidate counts the bias would have been below one part in 2⁴⁴ and
unobservable, but `uniform` is a claim the mappings make. **A distribution the
instantiator does not implement is a hard failure naming it**, never a silent
fallback to `uniform`: a fallback would emit a query set that looks fine, runs
fine, and is not the workload the template asked for.

### The candidate scrape is checked against the generator's own census

The frozen tarball ships `saved.txt` — the generator's own record of how many
entities of each type it emitted. Every scraped candidate set is cross-checked
against it, and a disagreement stops the run naming the type, the scraped count
and the declared count.

This matters more than it looks. An incomplete candidate set does not make
instantiation *fail*; it makes it quietly **biased**, and every query built from
it would be subtly the wrong query while every step still printed `OK`. All 17
declared types agree exactly on the pinned 10M dataset. A mapping naming a type
the census does not declare is likewise refused: its candidates could still be
scraped, but with nothing to check the scrape against the set would be
unverified rather than merely unusual.

### Pure BGP — no entailment, and that is not an omission

Every one of the 20 templates is a basic graph pattern: triple patterns and
nothing else. No `OPTIONAL`, no `UNION`, no `FILTER`, no `MINUS`, no `GRAPH`, no
subquery. WatDiv stresses structure and selectivity and needs no inference at
all, so **no entailment regime is chosen and none is used**.

The instantiator's `--self-test` *asserts* that emptiness rather than trusting
this paragraph, so a future template that smuggled in a `FILTER` would fail the
self-test instead of quietly changing what the lane measures. The lane's
`queries.tsv` carries a `regime` column that is always `-`, so the contrast is
visible in the data and not only in prose.

This is the exact opposite of the LUBM regime table above, and the two must not
be read across:

| | LUBM | WatDiv |
| --- | --- | --- |
| What it asks | What can be **derived** | How a planner handles **shape and selectivity** |
| Entailment | 11 of 14 queries have *no* answers without it | None, anywhere |
| Data | Generated per run from a pinned generator + seed | A digest-pinned **frozen output**; no generator is run |
| Queries | 14 published queries, mechanically normalised | 20 published templates, deterministically instantiated |
| Comparable with the other lane? | **No** | **No** |

### What the reported timings include

Each query is one process invocation, so its wall time includes opening the data
source — which at this scale dominates. Reporting that as query cost would be a
misleading number, so the lane **measures the open cost once** with a trivial
one-row probe and reports it, then prints both `TOTAL_MS` (process wall time,
open cost included) and `EVAL_MS` (the subtraction).

`EVAL_MS` is an *estimate*, and at this scale it is a small difference between
two large numbers on a host that is not quiet. Read it as an indication of where
the work is, not as a measurement of evaluation cost.

The dataset is loaded **through the CLI under test** into a native pack once per
run, and the 20 queries are answered against that pack. This is not an
optimization dodge: it is what a store does, and handing 20 queries the raw
N-Triples file would re-parse well over a gigabyte twenty times and measure the
parser rather than the planner.

### A vacuous run is a failure, not a fast one

Twenty basic graph patterns over ten million triples cannot all legitimately
match nothing. So the lane **hard-fails** if no query executed, and hard-fails
again if every query that executed returned zero rows — the shape a broken
prefix table, a failed load, or a broken instantiation takes, and the shape that
otherwise looks exactly like a very fast engine.

An *individual* zero is a real answer and is reported as one — and it is the
skew WatDiv exists to exercise, showing up. A property is concentrated on some
entities and absent from others, so a star pattern demanding several at once can
legitimately match nothing: on the pinned dataset, for instance, all 1 673
products typed `wsdbm:ProductCategory10` carry `wsdbm:hasGenre`, 1 016 carry
`sorg:description` and *none at all* carry `sorg:publisher`.

Those queries are listed by name under their own heading, and the report
distinguishes the two ways a zero arises. Where the query has a mapping, the
uniform draw landed on a candidate the rest of the pattern does not join with,
and `provenance.txt` says which candidate and out of how many. Where the query
has **no** mapping — the `MAPPINGS` column reads `0` — nothing was chosen at
all, and the zero is a fact about the published template over this corpus rather
than about anything this lane did.

### Parameters

Every knob is an overridable `make` variable, in the same style as `LUBM_*`.

| Variable | Default | Meaning |
| --- | --- | --- |
| `WATDIV_SCALE` | `10M` | Which pinned frozen dataset to use. Only `10M` is pinned; any other value is refused by name. |
| `WATDIV_SEED` | `0` | The instantiation seed. **Changing it changes the workload**, not just the run. |
| `WATDIV_OUT` | `target/watdiv` | Where the lane works. An absolute path is used verbatim; a relative one resolves against the repository root, so the default keeps everything the lane writes inside `target/` as build output. |
| `WATDIV_BIN` | *(unset)* | A prebuilt `purrdf` to use instead of building one. |

The extraction and the pack are each stamped with the digest they were built
from — the dataset tarball's digest, and for the pack that plus the binary's own
version — so a re-run reuses them only when they provably came from the same
bytes, and a stale artifact is a cache miss rather than a silent stale hit.

**Give concurrent runs separate arenas.** Instantiation begins by deleting
`$WATDIV_OUT/queries`, so a second run starting while a first is partway through
its twenty queries rewrites the query set underneath it — and the first run's
printed rows would then belong to two different workloads. That is the most
expensive kind of wrong number, because nothing about it looks wrong. The lane
therefore re-digests the query set before the first query and again after the
last, and **hard-fails naming the collision** if it changed. Two runs at once
want two arenas: `make watdiv WATDIV_OUT=target/watdiv-$$`.

### What it covers, and what it does not

It covers a real, published, externally defined workload end to end: a frozen
dataset verified by digest, deterministic query instantiation with a full
provenance record, loading **through PurRDF's own CLI**, and evaluation of all
20 queries with per-query row counts and timings.

It does **not** cover: any other engine, WatDiv's `linear_incremental` and
`linear_mixed` studies (separate suites, deliberately excluded from the twenty),
or scales beyond the one pinned dataset. It is not a conformance check either —
that a query returned *n* rows is a measurement, not a claim that *n* is the
answer any other implementation would produce.

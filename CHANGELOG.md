# Changelog

All notable changes to the PurRDF crate suite are recorded here. The suite
ships one lockstep version across crates.io, PyPI, and npm; from 1.0.0 a
breaking change bumps the major version, a minor bump is additive, and a patch
bump is bugfix-only. The C ABI (`purrdf.h`) is versioned separately and remains
0.x.

## [Unreleased]

### Added

- **core:** `purrdf_core::distance`, the binary64 distance arithmetic that every
  ranked-retrieval surface computes with. The module holds:
  - `Scalar`, `Bound` and `Bounded`, which moved here from
    `purrdf_sparql_eval::knn` and are still re-exported at their old paths;
  - the sealed `Arithmetic` trait. Its `ID` is the law's identifier. Its
    `IMAGE_CODES` and `image_code(path)` give the codes an index image records.
    Its `evidence(path)` gives the divergence text, and `resolve()` checks the
    float environment and selects a dispatch path;
  - `Resolved<A>`, an arithmetic bound to its dispatch `Path` for one scan, with
    `distances`, `distances_indexed`, `distance` and `distance_bounded`;
  - `Exact` (`binary64-lane16-tree-v1`). It accumulates 16 f64 lanes over array
    chunks, combines them in a fixed pairwise tree, adds the tail sequentially
    and uses no FMA. Every dispatch path returns the same bits;
  - `Reassociated` (`binary64-reassociated-v1`). It uses `f64::algebraic_*`
    inside each 64-element block and combines the block sums in ascending order,
    so every bounded checkpoint is a true prefix. On x86_64 it dispatches at run
    time to avx512f, then avx2+fma, then sse2. NEON, wasm simd128 and wasm scalar
    are chosen at compile time. Every other target runs its portable compilation,
    also fixed at compile time, so it resolves on every target. Each path has
    exactly one out-of-line copy, so every call site on a path gets the same bits.
    Its image codes are 2 (sse2), 3 (avx2+fma), 4 (avx512f), 5 (neon),
    6 (wasm-simd128), 7 (wasm-scalar) and 8 (portable); `Exact`'s is 1;
  - `Path` (`#[non_exhaustive]`), with one variant per dispatch path: `Portable`
    and `Avx2` for `Exact`; `Portable`, `Sse2`, `Avx2Fma`, `Avx512f`, `Neon`,
    `WasmSimd128` and `WasmScalar` for `Reassociated`. `Portable` names each
    arithmetic's own body compiled for the target's baseline features;
  - the batch kernels, which are the unit of dispatch. `Exact` has a
    bit-identical AVX2 path chosen once per scan. They come with `RowsRef`,
    `Measure`, `EXACT_LANES` and a `norm` that delegates to the one normative
    PURREMB norm fold;
  - the float-environment refusal. `FloatEnvironmentError::FlushToZero` and
    `RoundingMode` name a thread that flushes subnormal results or operands to
    zero, or rounds by any rule other than to nearest, ties to even. Each carries
    a `FloatEnvironmentEvidence`: `Register { name, bits }` where the control
    register was read (MXCSR on x86_64, FPCR on aarch64), or
    `Probe { operation, expected, observed }` for a binary64 operation whose
    bits differed from the IEEE-754 result. The environment is proven by that
    probe on every target: eight operations through `core::hint::black_box`,
    covering flushed results, flushed operands, all three directed roundings and
    ties-to-even. So i686, armv7, riscv64, powerpc64le, s390x, loongarch64 and
    any other target run both arithmetics, and are refused only when a departure
    is observed.

- **sparql-eval:** `Kernel::distance_reassociated` and
  `Kernel::distance_bounded_reassociated` are named entry points for the
  reassociated law. The exact names are unchanged.
  `EmbeddingKnnRelation::new_reassociated` builds a relation over `Reassociated`.
  It resolves the dispatch path once and refuses an unsupported float environment
  by name. The relation carries `OrderFidelity::Perturbed`, composed with the
  resolved path's evidence verbatim, and `FusionTrailer::fidelities` carries that
  evidence too. `RankedDeclaration::arithmetic` is a new field holding a
  `DeclaredArithmetic`, which can only be built from the sealed trait. It appends
  the law's id to `canonical_description`, so exact and reassociated plans have
  different identities. A declaration without it keeps exactly its old bytes, and
  the integer-ranked producers declare none, so no pinned fingerprint moves.
  `composed_order_fidelity` has one implementation, here, and `purrdf-hnsw`
  re-exports it at its old path.

- **hnsw:** a reassociated HNSW index. `HnswIndex::build_reassociated`,
  `HnswIndex::decode_reassociated`, `HnswSpace::from_artifact_reassociated`,
  `guard::load_reassociated` and the crate-level `build_reassociated` construct
  it. Each one refuses the other arithmetic's image codes. The image records the
  dispatch path that built it. Rebuilding or searching on another path is refused
  with `HnswError::ArithmeticPathUnavailable`, never a bare mismatch. Its profile
  is `hnsw-reassociated-v2` (`IMPLEMENTATION_ID_REASSOCIATED`). The loss evidence
  adds the arithmetic's divergence text and says that the image is bound to its
  build's dispatch path. `profile_declaration` folds in the arithmetic id. The
  guard derives its legal (implementation, revision, code set) pairs from both
  arithmetics and refuses cross pairings. `IndexLossContract` is unchanged,
  because the arithmetic transforms no vectors. Reassociated recall meets the
  exact floor in the tests.

- **iri:** byte-class scanners in `purrdf_iri::terminals`:
  `find_first_trivia`, `find_first_iri_body_special`,
  `find_first_json_string_special` and `find_first_xml_special`. `ByteClass`
  (built with `ByteClass::from_table`) and `byte_run_count` let a writer bring
  its own `const [u8; 256]` table and use the same kernel. Each scanner maps a
  16-byte chunk to a lane mask built from compares. It tests the chunk with a
  branch-free OR and finds the offset with `u128::trailing_zeros`. A class with
  one run uses a max-fold instead. The crate stays dependency-free and
  `forbid(unsafe_code)`. The byte predicates now read `const [u8; 256]` tables,
  which `const fn` derives from the range tables. The range tables remain the
  single source.

- **core:** `purrdf_core::ir::canon::write_literal_escaped` is now public.
  `purrdf_core::iri_escape` gains `push_escaped`, `escape` and
  `find_first_candidate`. PURREMB `F32Scalars` gains `check_finite` and
  `decode_into`, which report the same first non-finite column as the iterator.

- **build:** `make simd-asm` and `scripts/check-simd-asm.py`, an asm evidence
  gate. It emits per-crate asm for x86_64 (baseline, x86-64-v3, x86-64-v4),
  aarch64 (generic, neoverse-v1) and wasm32 (baseline, +simd128). For each
  function it counts packed-arithmetic and compare mnemonics, excluding move and
  zeroing idioms. It enforces the floors and ceilings pinned in
  `scripts/simd-asm-manifest.toml`: required mnemonics, FMA floors and ceilings,
  a relaxed-simd ban and single-copy enforcement. It refuses unknown manifest
  keys. With `--doc` it holds `docs/design/purrdf-simd.md` to the measurements.
  That covers parity, coverage, a roster of minimum sites, and a ban on speed
  multipliers in the document and in this section. The gate has its own CI job.
  Its self-test, which proves each refusal beside a valid neighbour, also runs in
  `make check` and in the CI workspace job.

- **ci:** an aarch64 job on an arm64 runner. It runs the distance module's path
  and reference-model tests on NEON, the kNN kernel tests, and the pinned
  cross-target kNN answers, exact and reassociated. It also runs the HNSW
  determinism and conformance tests. It compares against the same goldens that
  x86_64 and wasm32 hold, so an aarch64 fold that diverges fails there. Before
  this job, NEON code was only disassembled, never executed.

- **serialization:** an incremental `io::Write` path beside the byte-vector one,
  for the native RDF codecs and the SPARQL-results serializers. `TextSink` stages
  into a bounded 64 KiB window over a pluggable `ByteDrain`; `WriterDrain` adapts
  `io::Write`, `Measure` reports a document's length without the document, `Digest`
  reports its SHA-256 without it, and `Tee` forwards to two drains so a document can
  be written and digested in one pass over the emitter. The eager whole-output APIs
  are unchanged and are expressed THROUGH the streaming body, so byte identity
  between the two spellings is a property of the structure rather than something a
  test establishes afterwards. `TextSink` deliberately exposes no `truncate`, `pop`,
  `clear` or read-back: a sink cannot un-write a drained byte, so the affordance that
  would silently break streaming is not expressible.

- **build:** `scripts/check-serializer-rewinds.py`, wired into `make check` and
  `make serializer-rewind-hygiene`. Three serializers decided what to emit by
  emitting it and taking it back — a retracted Turtle prefix header, a popped SRJ
  root brace, an XML escaper truncating on an unrepresentable scalar — and each works
  perfectly until a document is large enough for a window to drain. The gate reports
  candidates rather than verdicts, against a reasoned allowlist naming each receiver
  that is not an output buffer, because `pop` on an element stack and `pop` on an
  output buffer are the same six characters and a gate that refuses both teaches
  authors to route around it.

### Measured

Peak allocator bytes, from the deterministic counting allocator rather than timings.

- The serialization graph's interner memoized on an owned copy of every term's
  value, so each term's text existed twice for the life of a build. Building a
  40,000-quad graph fell from a 32,449,626-byte peak to 17,986,057 (-44.6%) at
  200,013 allocations down to 80,012, with the retained graph and the interned term
  count both unchanged — the evidence that the dedup relation did not move. This was
  the ceiling that made N-Quads and TriG report byte-identical serialization peaks
  for documents differing by 931,240 bytes, and with it gone streaming measurably
  helps for the first time: N-Quads streamed now costs 5,153,092 bytes less than
  eager, where it previously measured 65,536 bytes worse.

- Compacting a JSON-LD document cloned the whole carrier to reach a `&mut` through a
  `&self` receiver. On a 1,147,788-byte document the peak fell from 12,379,601 to
  9,310,041.

- YAML-LD serialized to a JSON `String`, reparsed it into a `serde_json::Value` and
  converted that, holding four representations at once. It now emits straight from
  the carrier. The reparse had been credited with producing sorted key order; the
  carrier already emitted sorted keys, which was verified byte-for-byte against the
  old path before the reparse was removed.

- The SPARQL-results CONSTRUCT arm rendered a whole N-Quads document and JSON-escaped
  it afterwards. It now renders through the escaper, so nothing between the producer
  and the drain holds the answer.

### Fixed

- **sparql-results:** the SPARQL-JSON reader accepted raw control characters
  U+0000 to U+001F inside strings, including in skipped members. RFC 8259 §7
  forbids them. The reader now refuses them with a named error. Escaped controls
  and raw DEL still parse.

- **hnsw:** `HnswIndex::search_batch` reused one row-keyed distance memo across
  queries, so later queries saw the distances of earlier ones. Each query now
  gets its own memo.

- **retrieval:** the renderer kept its own copies of the IRI and canonical
  literal escape laws, because the kernel's version was private. It now
  delegates to `purrdf_core::iri_escape` and
  `purrdf_core::ir::canon::write_literal_escaped`, and so does entail. The canonical
  N-Quads writer, the Turtle writer and the native serializer also delegate IRI
  escaping to `iri_escape`. The native serializer's literal law, which also
  escapes C1, and the Turtle literal law remain separate laws. Every writer's
  bytes are unchanged.

- **cli:** a failed write could unlink a symlinked output path, destroying the link
  and leaving the half-written bytes in its target — the exact loss the guard's own
  comment described preventing. The guard asked the descriptor, which `File::create`
  had already resolved through the link; it now asks the path.

- **python:** `serialize(output=…)` called `write` once and discarded the returned
  count, silently truncating the document whenever a file-like object accepted less
  than it was given.

- **build:** three hygiene gates, each wired into both `make check` and CI because the
  first of them exists to make that pairing checkable.
  `scripts/check-gate-parity.py` takes the AGREEMENT between `make check` and the
  pull-request workflows as its subject: CI does not run `make check`, it enumerates each
  gate as a named step, so the two lists encode one rule and nothing compared them. It
  found three pre-existing divergences on its first run -- `check-terminal-predicates.py`
  and its `--self-test` ran locally and in no workflow at all, and
  `check-toolchain-pin.py --self-test` ran in CI and not locally -- and a fourth that was
  live behind a workflow `make` step. Seven one-sided gates are registered with their
  reasons -- two determinism checks needing the wasm toolchain, the book render needing
  mdbook, the conformance matrix needing tens of minutes, two `uv`-driven emitter
  oracles, and the SIMD asm evidence run needing the wasm32 and aarch64 standard
  libraries -- under a register that may only shrink and whose size is pinned, so growth is
  a visible edit rather than the silent one that left this count stale.
  `scripts/check-stream-chunk.py` refuses a streamed read whose chunk size is written out
  instead of named; six copies of that number lived under two names across five files,
  two already drifted to a quarter and a sixty-fourth of the shared size, and because every
  chunk size produces a correct digest nothing reported it.
  `scripts/check-tracked-paths.py` refuses a tracked path that misrepresents itself to the
  tools that read it -- a component beginning with `-`, which a glob hands to a command as
  a FLAG, or a character that does not render as what it is. A flag-shaped artifact had
  been committed at the repository root and swept past roughly thirty hygiene scripts,
  because they all walk a file list rather than a glob.

- **license:** MulanPSL-2.0 is offered as a third option alongside MIT and
  Apache-2.0, at the user's choice. `LICENSE-MULAN` and
  `LICENSES/MulanPSL-2.0.txt` carry the text, pinned by SHA-256 in
  `scripts/check-licenses.py` because the license's own section 6 makes its
  Chinese text controlling -- a silently drifted copy would change the governing
  terms. `scripts/check-licenses.py` also now refuses a first-party file whose
  SPDX identifier is not the expression `Cargo.toml` declares, reading that field
  rather than restating it, so the next license change cannot leave a file behind
  at the old offer. `docs/book/book.toml` is registered as deliberately CC-BY-4.0
  with its reason, under a register that only shrinks.
- **build:** `scripts/check-python-binding-tests.py`, wired into `make check`,
  `make pytest`, `make python-binding-hygiene` and CI. It fails if a `test`
  predicate appears in any `cfg` invocation, or a `#[test]` attribute anywhere,
  under `bindings/python/src`. That crate's manifest sets `test = false` because
  the library is a PyO3 `extension-module`: it leaves the CPython API unresolved
  for the interpreter to supply at `dlopen` time, which is what makes the `abi3`
  manylinux wheel portable, so an ordinary test executable has no interpreter and
  fails at link. A Rust test module there is consequently compiled by nothing and
  run by nothing while looking exactly like coverage in a diff. Neither of the
  textbook remedies is available -- a Cargo feature gating `extension-module` is
  forbidden here, and dropping the attribute would link libpython into the
  `cdylib` -- so the gate names the one route that works: the coverage belongs in
  `bindings/python/tests`. `--self-test` proves it fires on both shapes and, just
  as importantly, that it does not fire on prose describing them, so this crate's
  own source can keep explaining the rule.

- **retrieval:** A new publishable crate, `purrdf-retrieval`: the composition
  layer over the ranked property-function producers. One request becomes one
  answer across them through four stages -- a pure planner returning an
  inspectable `Plan` with a canonical BLAKE3 identity, a semantic admission waist
  that emits independently executable SPARQL per stratum, an executor that
  isolates each stratum's failure in its own status, and an exact fixed-point
  reciprocal-rank fusion over a verified ranked-stream protocol. Producers,
  strata and weights are caller-supplied configuration with no fabricated
  default. Reachable three ways: the crate directly, `purrdf::retrieval` on the
  umbrella, and `purrdf_native.retrieval` from Python.
- **retrieval:** A resolution algebra on the fusion law.
  `FusionProfile::class_width` reports how many consecutive ranks a profile
  cannot tell apart at a given depth, `deepest_rank_within_width` inverts it, and
  `DecayRule::weight_for_depth` inverts the whole relation -- name the depth, get
  the smallest weight that buys it. Reaching a depth is not monotone in the
  weight, so the answer is not bisected: each adjacent-rank constraint names the
  next weight that could satisfy it, and the search walks those candidates
  without ever skipping one, so the weight it returns is the true minimum rather
  than whichever side of an oscillation a probe happened to land on.
  `MonotoneDepth` spells the separating depth's saturation point as a
  distinct case so it cannot be mistaken for a measured depth, `ToleratedDepth`
  does the same for the depth a tolerance buys, and `ClassWidth` does the same
  for a class that never ends inside the range a plan can express. The same
  surface is exposed to Python as `retrieval.weight_for_depth`,
  `retrieval.class_width` and `retrieval.deepest_rank_within_width`.

  Its five refusals are five separate facts and carry five separate variants.
  `FusionError::InvalidWidth` means the tolerance was zero, which describes no
  class at all because a class always contains its own rank.
  `FusionError::DepthUnreachable` means the decay rule itself stopped separating
  adjacent ranks at any weight, and it reports the exact depth it does reach --
  the deepest any weight reaches, not the depth of one particular weight, and
  only the truncated rule can raise it. `FusionError::DepthBeyondPlanRange`
  means the depth is deeper than a plan can record, a plan carrying a
  per-stratum depth as a 32-bit rank; that is a limit of the encoding and not of
  the arithmetic, so the message names the encoding and the folded rule still
  answers at the deepest depth a plan can hold. A depth of zero names no rank
  and is refused as `FusionError::InvalidRank` rather than answered with the
  lightest weight there is. And a smoothing constant of zero describes no law at
  all: all three functions refuse it as `FusionError::InvalidK` before they read
  the question they were asked, so the refusal no longer depends on the other
  argument -- a depth of one, or a tolerance of one, previously answered from a
  short-circuit and quoted a real number under a rule that cannot be evaluated,
  while one step further along either axis reported the decay rule as having
  saturated at depth one. A malformed law is not a saturated rule, and telling
  it as one sent a caller to a remedy -- switch decay rules -- that cannot help.
- **retrieval:** `DecayRule::class_width`, the same resolution curve
  `FusionProfile::class_width` reports but asked of a weight rather than of a
  stratum. A width is a property of the rule, its smoothing constant, the weight
  and the rank and of nothing else, so a caller holding only arithmetic -- a
  language binding, or a profile author sizing a weight before any stratum
  exists -- asks the rule directly instead of building a throwaway one-stratum
  profile to ask through. `DecayRule::weight_for_depth` already sat at that
  altitude; the width now sits beside it. The profile-level call remains, is
  unchanged for callers with a real stratum in hand, and now reaches the same
  arithmetic through it rather than through a second path that could drift.
- **retrieval:** `DecayRule::deepest_rank_within_width`, the inverse of
  `DecayRule::class_width` asked at the same altitude: name the tolerance, get
  the deepest depth that stays inside it. `FusionProfile::deepest_rank_within_width`
  now reaches this rather than a private function of its own, so the two
  arithmetic-only entry points -- `class_width` and its inverse -- sit beside
  `weight_for_depth` the same way on `DecayRule`, and a caller holding only a
  weight and a tolerance, with no stratum to name, asks here. Bound to Python
  as `retrieval.deepest_rank_within_width`, alongside `retrieval.class_width`
  and `retrieval.weight_for_depth`: it takes the same required `decay` keyword
  they do, with no default rule, and raises `ValueError` rather than answering
  where the arithmetic could not.
- **retrieval:** Rank-resolution evidence on the answer. `CompiledRetrieval`
  carries a `PlannedResolution` per weighted stratum, so the cost of a planned
  depth is knowable before executing anything, and `FusionTrailer` carries a
  `StratumResolution` for every weighted stratum it was handed a stream for --
  where the profile stops separating ranks, how deep this run reached (zero for
  a stream that ended without emitting a row, which was still pulled from), and
  how many adjacent ranks its score could not separate, counted by direct
  observation rather than inferred.
  The trailer also reports whether the last row in a top-k beat a *settled*
  rival it tied with exactly, which is the case where the final place was
  decided by the declared tie-break rather than by relevance. Only rivals
  already final when that row was emitted are counted, so a `false` there means
  "no settled rival tied with it" and not "the cut fell on a strict score
  difference" -- settling every live rival would mean reading past the top-k,
  which would move the ranks-pulled evidence in the same trailer.

  Both altitudes are readable from every entry point, and kept apart by name.
  `SearchResult::planned_resolution` carries the compiled plan's own map onto the
  fused answer, so the one call that plans and executes together is not the one
  call that cannot see what it was about to pay; `FusionTrailer::resolution`
  beside it is what the rows really cost. From Python the same pair is
  `"planned_resolution"` and `"observed_resolution"` on the `search` answer, and
  `retrieval.compile` now takes the `weights` and `k` that name a fusion law and
  answers `"planned_resolution"` for it -- per weighted stratum, the depth the
  law still separates, the depth the plan recorded, and whether every rank the
  plan reads is still ordered by score alone -- with nothing executed. Omitting
  both leaves the map empty rather than measuring against an invented law, and
  naming one without the other is a `ValueError`.
- **bench:** A new unpublished tooling crate, `purrdf-bench`, and its
  `bench-corpus` binary: the deterministic, shardable scale-corpus generator
  behind the `purrdf-scale-mixed-v1` profile. Every IRI is minted purely from
  its index under a fixed seed, across five deliberately adversarial classes
  (front-codable plain, long zero-padded numerics beyond machine integer widths,
  raw-Han Chinese, host-scattered irregular with reserved-octet escapes, and
  very-long), so no single dictionary trick can flatter a capacity claim. The
  row mix pins a share for every row kind and spans the RDF 1.2 term space the
  profile targets — reifier rows binding triple terms, `xsd:`-typed literals
  whose lexical forms are valid for their datatype, language-tagged literals,
  and blank nodes in both subject and object position — so term-kind coverage is
  a property of the profile rather than an accident of it. Index-pure minting
  makes generation shardable with no coordination between shards: shard `k` of
  `n` emits exactly its slice, and concatenating every shard is byte-identical
  to one whole run, pinned by a golden digest. `make scale-corpus` is the lane
  that drives it across shards, in streaming, piped, or opt-in file modes.
- **bench:** Two comparison lanes against the workloads the RDF literature
  publishes against, `make lubm` and `make watdiv`. Both are REPORT-ONLY, neither
  is a gate, and neither vendors a byte: every artifact is fetched by digest into
  an ignored cache under `target/` at the moment of use, because the LUBM
  generator is GPL-2.0-or-later and WatDiv is citation-ware. `make lubm`
  generates LUBM(N), converts it to N-Quads through the `purrdf` CLI itself and
  answers the 14 published queries, printing the entailment regime and the
  dataset rung every row was answered under — eleven of the fourteen have no
  answers at all without inference, so a row without its regime is not
  comparable with anything. `make watdiv` consumes upstream's digest-pinned
  frozen 10M dataset (WatDiv's own generator is time-seeded and has no seed
  flag, so a dataset is reproducible only as a frozen output) and instantiates
  the 20 published templates deterministically from it, publishing a query-set
  digest: the same dataset and the same seed reproduce the workload byte for
  byte, and a different seed is a different workload rather than a re-run. All
  three lanes share one implementation of the laws that make their numbers
  evidence — `scripts/lane-common.sh` — including the rule that a certificate
  (a manifest, a reuse stamp, a published digest) is never written for output
  that was not produced and never outlives the run that wrote it.
- **envelope-probe:** A new unpublished tooling crate,
  `purrdf-envelope-probe`: the capture side of the micro-hardware validation
  envelope. A fixed, deterministic workload set runs per named profile over the
  public APIs and the keystone fixture corpus, so a release can demonstrate that
  a constrained deployment class still fits its pinned ceilings. Pass criteria
  are completion and memory; wall time is recorded evidence, never a gate.
- **hnsw:** A new publishable crate, `purrdf-hnsw`: a deterministic HNSW
  approximate nearest-neighbour index over a PURREMB embedding matrix, registered
  on the evaluator's property-function seam under a caller-supplied predicate IRI.
  Levels come from a splitmix64 hash of the stable row index rather than from an
  RNG, the build is round-structured against a frozen snapshot, and the canonical
  byte image is identical across rayon worker counts and across
  `wasm32-unknown-unknown`. It joins the release set as the 24th crate.
- **purrdf:** The umbrella re-exports the new crate as `purrdf::hnsw`, on the same
  seam `purrdf::geo` and `purrdf::text` use, so a consumer can name `HnswIndex`,
  `HnswSpace`, `Params` and the relation while depending on `purrdf` alone.
- **sparql-eval:** `knn::Kernel::distance_bounded` with `knn::Bound` and
  `knn::Bounded`. Squared-euclidean partial sums are non-decreasing, so a caller
  that only needs to know whether a distance clears a threshold can stop early.
  It shares one fold with `Kernel::distance`, so the two cannot drift, and a
  candidate that is not abandoned is scored bit-for-bit as before.
- **sparql-eval:** `knn::Scalar`, a stored scalar that widens to binary64 exactly.
  PURREMB stores matrices at either width, and widening `f32` at load costs twice
  the resident memory while changing no arithmetic; the kernels now accept either
  width and widen per component inside the fold.
- **retrieval:** `ProducerStatus::DepthReached` and `ProducerReceipt::DepthReached`,
  a read ending authored by the producer and stated in **rank** space: ranks one
  through `rank` were read, and the rows did not run out -- the planned depth did.
  It is deliberately not `CeilingReached`, which is fusion's ending and whose
  stopping point is a contribution in the profile's fixed-point space that the
  producer was never given. The two name different knobs -- re-plan deeper, or
  certify further rows against the same streams -- and neither number converts
  into the other. Fusion checks `rank` against the rows it actually pulled and
  refuses a disagreement as `ProtocolError::ForgedReceipt`, exactly as it checks
  an exhaustion count: the depth is a licence to stop reading, never a licence to
  misreport. `Exhausted` is now the only ending that names no stopper, and every
  other ending names one. That is a fact about the read rather than about what
  exists: a producer emitted every row its search produced, which is why a
  status is read beside the stratum's declared fidelity and never instead of it.
- **retrieval:** `FusionTrailer::attestations`, a per-stratum record of what the
  index behind each stream attested -- which generation answered, and the
  verbatim reason if that index declared itself short. It is read from
  `RankedStream::attestation` in `FusionStream::new`, **before any row is
  pulled**, because a generation is pinned when a cursor opens. That ordering is
  load-bearing rather than tidy: a terminal receipt is overwritten by a bounded
  stop -- a stream a top-k stopped never returns a receipt at all -- so an
  incompleteness held terminally would be destroyed exactly in the runs where the
  bound mattered. Held as an attestation, a stratum that was both stopped and
  short reports both facts. The map keys only the streams the fusion was handed; a
  stratum whose unit never ran is absent rather than filled with "declared
  nothing", which would report a producer that was never asked as one that
  declined to answer.
- **retrieval:** `ScoreExactness` on the trailer. A stratum serving from a short
  index omits whatever its missing shard held, so a candidate that shard would
  have named is summed one contribution light; labelling that "exact" would be a
  bound on the read presented as a value. `Exact` says the narrower true thing --
  no handed stream declared itself degraded -- and is never a certificate that
  every index was whole and every search exhaustive, because most producers
  attest nothing and there is no variant with which to attest wholeness.
  `Estimated` carries the error in BOTH directions, because this layer scores by
  rank and nothing else: a stratum that fails to name a row does not merely
  withhold that row's contribution, it moves every row behind the missing one up
  a rank and hands it a larger contribution than it earned. `deficit` names the
  strata that could have added to a candidate, `inflation` the strata that could
  have over-contributed to one, and `unbounded` the strata whose declared ORDER
  is perturbed, for which no finite bound exists at all -- every bound here rests
  on a named row's true rank being no better than its emitted rank, and a
  producer comparing approximated values breaks exactly that. Each stratum's
  verbatim reason is under the same key in `fidelities` or `attestations`. It is
  derived from the declarations and attestations alone, so it does not move as a
  caller certifies further rows.
- **retrieval:** `RankFidelity`, with `Completeness` and `OrderFidelity`, and
  `StreamContract::fidelity` carrying it through the ladder. What a producer
  declares about its own search, on two axes that fail independently: whether it
  names every row that was due, and whether a row it does name arrives at a rank
  no better than it earned. It is declared rather than observed because a
  consumer cannot tell the difference -- a stream that ran out of rows and a
  stream whose search merely stopped finding them both simply stop yielding, and
  the ranks are contiguous either way. There is no `Default`: `RankFidelity::EXACT`
  is the top of the lattice, so defaulting to it would put the strongest claim in
  the mouth of a producer that said nothing, which is the one direction a default
  must never go. `RankedDeclaration` therefore carries `fidelity` as a required
  field, and `StreamContract::new` takes it as its second argument.
- **retrieval:** `FusionTrailer::fidelities`, a per-stratum record of what each
  producer declared, populated in `FusionStream::new` before a row is pulled, so
  a fused answer distinguishes an approximate stratum from an exact one without
  consulting the registry. The producer's own evidence string reaches it
  verbatim: the same `Arc<str>` the producer published, never parsed and never
  re-worded. A stratum whose stream never opened is absent rather than reported
  exact, so the map is three-valued where it needs to be and a consumer never
  reads a missing key as a claim.
- **retrieval:** `FusedRow::interval` and `ScoreInterval`, bounding each row's own
  error rather than the answer's. `Bounded` carries `deficit` and `inflation` in
  the profile's fixed-point space, narrowed per candidate by the same `Dom(x)`
  reasoning the certification bound uses: a stream whose declared domains cannot
  reach a candidate withheld nothing from it and is not charged. `Unbounded`
  names the perturbed strata instead of reporting a number, because there is no
  number and reporting one would be the fabrication this channel exists to
  prevent.
- **retrieval:** `FusionTrailer::certain_prefix`, how many leading rows keep
  their places whatever the degraded strata did or did not find. It is what a
  caller with a completeness obligation can still act on, since the alternative
  is to downgrade a whole answer over one degraded stratum. It claims membership
  and never absence: a row past the prefix is possible rather than excluded.
- **retrieval:** `FusionTrailer::unemitted_ceiling`, the bound the prefix above
  rests on: the most any candidate OUTSIDE the answer could be worth. It counts
  two populations, and both are needed for the prefix to be a claim about the
  answer rather than about the ranking -- the candidates no stream ever named,
  charged each degraded stratum's rank-one contribution because a row such a
  stratum missed could have been due at any rank, and the candidates a bounded
  read named and abandoned, which a degraded stratum could still owe a
  contribution on top of what they had already accumulated. It is `None` where a
  stratum declared a perturbed order: that breaks the one inequality every bound
  here rests on, so no finite ceiling exists and `certain_prefix` is zero. It
  speaks for the reads this trailer describes; a producer stopped at its plan
  depth, or one that declined its terms, states that shortfall in `statuses`.
- **hnsw:** `HnswRelation` declares its own `RankFidelity`, so the workspace's
  one approximate producer states what it is to a consumer of composed rows
  instead of being indistinguishable from an exhaustive one. The evidence is the
  profile's `IndexLossContract` loss evidence, carried verbatim rather than
  re-worded here.
- **text:** `TextSearchRelation::ranked_declaration` takes the fidelity the host
  declares. BM25 over the index this relation holds is exhaustive and ranks by
  exact scores; whether that index covers what the host means by its corpus is a
  fact the relation cannot see, so the term is the host's to supply for the same
  reason `domains` is.
- **sparql-eval:** `EmbeddingKnnRelation::ranked_declaration` takes that same
  fidelity term, in the same position, for the same reason. The relation is the
  exact oracle -- it scans every row of its space, prunes nothing and exits early
  nowhere -- so `RankFidelity::EXACT` is a true statement about its *search* and
  it used to assert exactly that. It is not a statement about whether the vectors
  it searched are the whole of a host's corpus, and a host that embedded a sample
  had nowhere to say so: the answer certified that every stratum was exhaustive
  and whole while a document the whole space would have ranked was silently
  absent. Breaking, in the same way and at the same seam as the text relation
  above.
- **hnsw:** `HnswRelation::ranked_declaration` and `HnswRelation::fidelity` take
  an `OrderFidelity` describing what the
  host did to its vectors **before** `HnswIndex::build` saw them. A host that
  quantizes its embeddings and then builds a graph over the codes has an
  order-perturbed producer, and no loss contract reachable from the index records
  it -- the profile's contract describes what the build did, not what reached it.
  The new term composes with the derived axis through `composed_order_fidelity`,
  which takes the worse of the two, so a host can degrade the axis and never
  upgrade it. The completeness axis stays the relation's: it is `Lossy` over
  every space on every request, so no silence there can be read as completeness,
  and the profile's own evidence keeps the axis and keeps reaching a consumer
  byte for byte. Breaking.
- **hnsw:** `register_ranked_hnsw_relation` takes the five facts a host states
  about its own corpus and pipeline as one `RankedHnswRegistration` rather than
  loose: they are one statement made at one moment about one space, and the new
  disclosure would otherwise have made the call eight positional arguments of
  which five were the same argument. A bare-field struct with no builder and no
  `Default`, like every other declaration in this workspace. Breaking.
- **python:** A `text_producers` value may carry a sixth `(completeness, order)`
  position, each member a `str` or `None`, shaped like the attestation position
  beside it and read on the same terms -- `None` is silence on that axis, a
  string is that axis declared degraded with the host's own words carried
  verbatim. `search` reports `"fidelities"`, `"certain_prefix"`,
  `"unemitted_ceiling"` and a per-row
  `"interval"` beside the existing keys, and `"exactness"` gains `"deficit"`,
  `"inflation"` and `"unbounded"` in place of `"lower_bounds_for"`.
- **retrieval:** `EvidenceId`, the third identity a fused answer carries, on both
  `FusionTrailer` and `SearchResult`. `PlanId` names the question that was asked
  and `FusionProfileId` names the law the rows were fused under, and neither moves
  when an index is rebuilt underneath a running system -- both are derived from
  configuration, which is exactly what a rebuild does not change. `EvidenceId` is
  the content identity of the attestation map, so one equality comparison over the
  triple decides whether two answers are comparable at all.
- **retrieval:** Candidate domains reach fusion. `StreamContract` carries the
  producer's `CandidateDomains` beside its duplicate policy, `StratumUnit` and
  `StratumStream` carry it through, and `FusionTrailer::domains` reports what each
  handed stream fused under, so an answer can be audited against the declarations
  that decided which streams fusion was allowed to skip. `ProtocolError::OutsideDeclaredDomain`
  refuses a stream that names a candidate its declaration cannot reach, carrying
  three fields because each answers a different question: the item says what, the
  stratum says who broke it, and the third names the stratum whose own declaration
  -- already applied -- put the candidate out of reach. The answer is not silently
  widened instead, because the declaration has already been used: fusion certified
  candidates early on the strength of it, so an earlier row may already have been
  emitted on the assumption this stream would never name the candidate.
- **retrieval:** `PRODUCER-CONTRACT.md`, published with the crate and rendered in
  its API documentation as the doc-only `producer_contract` module. Fifteen
  obligations a ranked producer owes this layer, each stating the obligation, the
  failure it prevents, and who enforces it -- the layer, where a breach is a named
  refusal, or the producer, where a breach is a wrong answer and the entry names
  the test that proves the shipped producers keep the promise, or says that none
  does. Three of the fifteen are one doctrine wearing three hats, because "I
  stopped early" and "I am exhausted" are the same empty cursor: the engine
  withholds a ceiling it cannot account for, the executor reads one row past the
  depth it recorded, and the producer declares the shortfall only it can know.
- **sparql-eval:** `PfCursor::generation` and `PfCursor::service_level`, both
  defaulted to an honest absence, through which a relation's cursor attests the
  two facts about an invocation that nothing else can see: which generation of its
  index answered, and whether that index was whole. The generation is read
  immediately after `open`, where the snapshot is pinned and the claim is true of
  every row that follows; the service level is read when the invocation **ends**,
  so a relation that discovers a missing shard on its four-hundredth pull still
  has somewhere to say so. There is deliberately no variant for wholeness and
  never will be: a relation that stopped at the engine's ceiling licence is not
  incomplete and could not honestly certify completeness either, so the seam asks
  only the narrow question it has an honest answer to on every path. `Undeclared`
  is silence, never a certificate.
- **sparql-eval:** `RelationWitness` and `RelationAttestations`, the per-query
  ledger those declarations accumulate into, reachable on the governed receipt
  through `RelationIdentity::witness`. Every field is a set or a count and never a
  sequence, because a query can invoke one relation thousands of times across
  several forked workers in an order the evaluator is free to choose, and an
  ordered log would describe the schedule as much as the index. The record is
  owned per context and merged at every fork join rather than shared behind a
  lock; set union and a saturating count make the merge commutative and
  associative, so the result is a function of what was attested rather than of who
  finished first. It rides the receipt beside the governor evidence already there,
  because an attestation describes an outcome rather than being one.
- **sparql-eval:** `EvalError::RelationIncomplete`, raised where a relation
  declares its index short and the entry point has nowhere to record it. A short
  bag whose receipt nobody can read is precisely what the hard-fail doctrine
  exists to prevent.
- **sparql-eval:** `CandidateDomains` and `DomainTag`, the second promise a ranked
  producer makes about its own rows, declared where it declares its duplicate
  handling. A consumer fusing several ranked streams has exactly one way to learn
  that a stream will not name a candidate -- read that stream to its end -- and
  over strata whose candidate sets do not overlap that is every row of every
  stream, however small the caller's top-k. `Unrestricted` is the wider promise
  rather than the weaker one: it says the producer may name anything, it is
  today's behaviour exactly, and a fusion whose every stream declares it computes
  precisely the numbers it computed before this existed. Nothing is defaulted from
  a stratum or a graph, because the host is the only party that knows whether its
  text index and its vector index name the same entities, and a wrong guess is a
  wrong answer. An empty restriction is refused at `register_ranked`, where the
  declaration is committed: a promise to name nothing describes a producer that
  should not be registered.
- **retrieval:** Every ranked row now says which block of the candidate universe it
  was drawn from, so the axiom the declared-domain arithmetic rests on is verified
  rather than trusted. `RankedStream::next` yields a named `RankedRow` -- rank,
  contribution, item and `RowBlock` -- and `RowBlock::Undeclared` is a first-class
  absence beside `IndexGeneration::Undeclared` rather than an `Option` a reader
  could unwrap into a claim. A `CandidateDomains::Within` stream owes a block on
  every row, and owes one its own declaration admits; an `Unrestricted` stream owes
  none, because it restricts no arithmetic at all, and a block it volunteers is
  still honoured as evidence about the candidate. Three refusals, each a separate
  fact: `ProtocolError::UnbackedDomainDeclaration` -- a restricted stream's row
  names no block, so nothing backs the restriction, and fusion has already used it;
  `ProtocolError::BlockOutsideDeclaredDomain` -- a row names a block its own
  declaration excludes, which is a stream contradicting itself and needs no second
  stream to witness it; and `ProtocolError::CandidateInTwoBlocks`, naming the item,
  both strata and both blocks, because two rows that place one candidate in two
  blocks falsify the partition the threshold's per-block maximum depends on and
  either producer may be the one that tagged wrongly. That last case is the one
  `OutsideDeclaredDomain` cannot see: it compares declarations, and two declarations
  can overlap while the rows disagree -- which is exactly where the bound
  under-counts, since the streams that reach one block and the streams that reach
  the other are different sets. A row a permissive duplicate policy discards is
  checked too: a repeat may be dropped, its claim about the candidate may not.
  `compute_threshold` keeps the largest per-block sum of open heads, unweakened; a
  tagging that violates the axiom now fails loudly instead of returning a plausible
  order, as far as the rows pulled reach and no further.
- **sparql-eval, retrieval:** `RankedDeclaration::block_position`, the argument
  position a producer's rows carry their block in. A declaration naming exactly one
  block entails every row's block and needs no column -- the common configuration,
  one producer per block, costs a host nothing. A declaration naming several
  entails nothing about any one row, so a producer that can say declares the
  position its rows name it from, and one that cannot leaves it unset and is held to
  the consequence rather than believed. `purrdf-retrieval` projects the column
  beside `?candidate` for exactly the producers that declare it and reads it back by
  name, so a unit for a producer that declares none is byte-identical to before. The
  position is refused at registration when it falls outside the relation's arity or
  collides with the candidate, a term placement or the depth, and it reaches the
  registry's content fingerprint -- two wirings that verify differently may not
  share a digest -- so that fingerprint and the plan identities over it move.
  Neither shipped producer declares one: a text index answers with documents and a
  vector index with neighbours, and neither holds any notion of a host's partition.
- **text, sparql-eval:** Both shipped ranked producers attest a content-derived
  generation for the index that answered. The text relation declares its index
  fingerprint, which closes over the documents, the term dictionary, every
  posting, the partition statistics, the configured predicates and the ranking
  law; the two neighbouring digests were rejected against the rule that a
  generation must move exactly when the answerable rows move, and the rejections
  are pinned as tests -- the digest of the source rows does not move when the same
  rows are re-ranked under a different law, though every emitted score does, and
  the analyzer digest holds no document at all. The embedding relation cannot
  declare its matrix alone, because the map from a row to the RDF term it stands
  for is a host argument: PURREMB allows a target to be disclosed by digest alone,
  so two spaces over byte-identical artifacts with different bindings return a
  different term at every position. Its declared value folds the projection digest
  (rather than the matrix digest, because a prefix policy lets two spaces share
  one stored matrix and differ in the prefix taken), the family contract that
  names the metric, and every bound term in row order. The bound on work is
  excluded: it decides how hard a search tries, never which rows exist or how they
  rank. Both values are computed once where the snapshot is pinned, out of any
  per-row path.
- **python:** The retrieval answer carries the evidence the Rust surface does.
  `"attestations"` maps a stratum to `{"generation": str | None, "incomplete":
  str | None}`, with the two axes independently absent because a silent producer
  is making no claim in either direction; `"exactness"` is `{"exact": bool,
  "deficit": list[str], "inflation": list[str], "unbounded": list[str]}`, naming
  the strata whose declarations make every score an estimate rather than an exact
  sum, on each side the error runs; `"domains"` reports the
  candidate-domain declaration each stream fused under, so an answer can be
  audited against its own inputs; and `"evidence_id"` sits beside `"plan_id"` and
  `"profile_id"` in the same 64-character lowercase hex spelling, so one
  comparison over the three decides whether two answers are comparable.
- **python:** `retrieval.search`'s `text_producers` value may carry a fifth
  `(generation, incompleteness)` position -- each member a string or `None`, recorded
  verbatim -- declaring what the host knows about the index behind that producer. A
  declared incompleteness is reported under
  `answer["attestations"][stratum]["incomplete"]` and names that stratum under both
  `answer["exactness"]["deficit"]` and `["inflation"]`, which was previously
  unreachable from Python: no shipped producer can know what was missing from the
  document it was handed, so the surface was structurally present and could never
  carry a value. A declared generation is reported and is not a shortfall; it
  replaces the content digest the shipped text relation attests, because one
  generation is pinned per invocation, and a generation that does not move when the
  host's corpus does will defeat the evidence identity. Declaring nothing is silence,
  never a claim that the index was whole, and a value without the position behaves
  exactly as before. The position is FIFTH -- after an explicitly written `domains`
  -- because a fourth-position sequence is already a domains list and the two would
  be indistinguishable: `("a", "b")` is a well-formed two-tag restriction and a
  well-formed attestation at once, and guessing would report a domain tag back to an
  operator as an index generation.
- **python:** A governed outcome carries `relation_witness`, the record of what each
  relation attested about itself during the run: per relation IRI, how many times it
  was invoked, which index generations answered, and the verbatim reason wherever one
  declared its index incomplete. It is read off the governed receipt on both arms, so
  a query whose governor tripped still reports what the relations attested before it
  did. The mapping is always present and may be empty -- a governed query that invoked
  no relation reports `{}`, which is a fact about the run rather than an absence -- and
  `None` inside `"generations"` is an invocation that attested nothing, kept as a member
  of the set rather than dropped because otherwise "declared one generation every time"
  and "declared it sometimes and said nothing the rest" would read identically. Keys
  and both lists are emitted in the engine's own sorted order, so two runs over one
  snapshot compare byte for byte.

  This is the other half of the refusal beside it. A relation that declares itself
  incomplete is refused on any lane with nowhere to record the declaration, and the
  governed lane is the sanctioned destination -- so a surface that refused on one lane
  and could not show the receipt on the other left the caller with no honest route at
  all, which is the state the hard-fail doctrine exists to prevent.
- **python:** Every relation declaration accepts one trailing position,
  `(generation, incompleteness)`, each a string or `None` and each recorded verbatim,
  so a host-registered relation can attest the two facts only its own cursor knows.
  Declaring neither is the default and means the relation said nothing, which is
  silence rather than a claim that its index was whole. The position reaches all three
  relation kinds and every query, update and entailment entry on both `Store` and
  `MutableDataset`. A trailing value that is not a two-member sequence reports the
  declaration's accepted shapes, and a string is refused explicitly rather than
  destructured -- `"ab"` would otherwise extract as a well-formed attestation of
  `("a", "b")`.
- **python:** A host declares a producer's candidate domains where it declares the
  rest of that producer: the ranked-producer tuple takes an optional fourth
  element, a list of domain-tag IRI strings, with `None` -- and omission -- meaning
  "may name anything". An empty list is refused by name before it reaches
  registration, because a promise to name nothing is not a restriction, and
  because the refusal on the far side of the boundary is a panic that must never
  cross it.
- **python:** A compiled retrieval unit carries `"declared_rows"` beside its
  `"stratum"`, `"sparql"` and `"depth"`: the row count the registry declared for
  that stratum's one producer, or `None` where the producer declared no access mode
  and therefore declared no row count at all. Rust callers have read it off
  `StratumUnit::declared_rows()` all along, and the depth alone does not say which
  of two situations a host is in -- a depth BELOW the declaration leaves rows under
  the read, while a depth EQUAL to it means the producer has already promised there
  is nothing further and the probe row is what checks that promise rather than
  taking it. Neither is recoverable from `"depth"`, from the emitted text, or from
  the plan, so a host reading `"depth"` to know how many rows it may report has the
  same claim on the declaration that `retrieval.search` does. "Declared nothing"
  and "declared zero" stay different facts across the boundary: an absent
  declaration can refuse nothing, while a zero is a measurement of the producer's
  data, so the absence arrives as `None` rather than as a number nobody took.
- **sparql-algebra:** `SparqlParser::parse_query_split` and the `QuerySplit` it answers,
  which parse a query exactly as `parse_query_with` does and also report where the two
  clauses only a WHOLE query may write are written in the text: the byte offset its
  prologue ends at, and the byte range its `FROM` / `FROM NAMED` run occupies. A caller
  wrapping a supplied query in a sub-select has to *move* both, because
  `SubSelect ::= SelectClause WhereClause SolutionModifier ValuesClause` carries neither,
  and neither position is derivable from the algebra -- a prefixed name is resolved away
  at parse time, and re-serialising the body would rewrite surface spellings the caller's
  next reader depends on. Positional only: both numbers are offsets the one parse was
  already standing at, so every piece a caller cuts out is the caller's own bytes and the
  algebra beside them is unchanged. The dataset range ends at the token after the last
  clause rather than at its last IRI, so excising it leaves a query that still parses.
- **python:** `MutableDataset` answers the native validation snapshot protocol, so
  `Shapes.validate_store` accepts one for real: both quad containers on this
  surface hold a frozen dataset behind a copy-on-write overlay, and validation
  borrows that snapshot instead of serialising to N-Triples and parsing it back.
  Either container validated yields the one answer the triples deserve, and the
  snapshot rule holds on both -- a report already produced is a statement about the
  data as it was. A value that answers no such protocol is now refused with a
  `TypeError` naming the type that arrived and the three things accepted (a
  `Store`, a `MutableDataset`, or `validate_nt` for text a caller holds), rather
  than escaping as an `AttributeError` about a private attribute the caller never
  wrote.

### Fixed

- **sparql-eval:** The forked row loop's per-chunk harvest no longer allocates below
  the parallel threshold. Harvesting a worker's relation witness came back as a `Vec`
  of one element on the sequential path — a heap allocation per `FILTER` or `BIND`
  evaluation whose whole content was one, almost always empty, witness. On the
  per-focus-node SHACL path, whose allocation count per focus node is pinned exactly,
  that was one to two allocations per focus node over the pin; the harvest now rides
  inline in a one-slot small vector and the pins hold again.

- **sparql-eval, shapes:** The borrowed governed lane — the per-focus-node entry SHACL
  validation drives — carries the relation witness, and a validation over an index
  that declared itself not whole is refused. Two governed egresses build their own
  context and resolve their own verdict, and only one of them armed witnessing and
  moved the witness onto its receipt; the other handed back a receipt whose ledger
  said nothing about the relations the run had invoked. Both now share one context
  builder and one resolution, so the witness is filled in one place for both. That
  makes the borrowed lane RECORD a relation declaring a shortfall rather than refuse
  it at the seam the way the ungoverned lane does -- and SHACL validation reads only
  the rows out of that lane, so the record was where a shortfall would have been
  silently dropped: "this focus node has no violating solution" over an index that
  was not whole reads exactly like the same sentence over the whole index. The
  validation now reads the receipt and refuses by name, quoting the relation and its
  own reason, exactly as it refuses a truncated solution bag; the same relation
  declaring nothing validates to a report.

- **retrieval:** The declared row bound is read at the mode the producer is actually
  invoked in. `rows_per_invocation` is a function of the mode, and the layer had been
  taking the maximum across every mode a producer declared -- so a depth was admitted
  that the invoked mode had declared it could not serve, a producer beating its invoked
  mode's bound went unrefused whenever another mode declared a larger number, and the
  number handed to a self-bounding producer was capped against the wrong mode, so a
  producer guarding its own bound lost its whole stratum. The bound is now the tightest
  declared under any mode that subsumes the invocation -- the same lattice rule
  placement admits the call by, and a producer serving through a subsuming mode emits
  at most that mode's rows, since the extra bindings only filter. There is one function
  computing it, called from the planner and the waist alike, where there were two.
  Every fixture in the suite had declared a single mode and ignored the parameter,
  which is why the difference was unobservable; one now answers a different number per
  mode.

  Both refusals that name a row bound say which mode it was read at, because for the
  multi-mode producers the seam is built for that mode is routinely not the invoked one.
  A coarser mode declares fewer bindings, so its tuples cover a finer mode's and its
  count bounds the finer read too: a producer promising a hundred rows with its needle
  bound and three with it free has over-declared the bound case, three is the sound
  bound, and the figure appears nowhere in the declaration the call was made at.
  `ExecutionError::RowBoundBreached` and `AdmissionError::DepthBoundViolation` therefore
  carry `BoundMode`, which renders as the mode the number came from and, where that is
  not the invoked one, as the call it serves -- "under mode `ff`, which serves this call
  under mode `fb`". A bundle a caller assembled itself records a count and no mode, and
  says that rather than attributing the caller's own number to a registry read that
  never happened.
- **retrieval:** A query text a caller supplies is wrapped rather than appended to, and
  the prologue it was written with is hoisted above that wrapper. The layer's bound was
  written after the caller's text, so a text carrying a top-level bound of its own
  produced two bound clauses, of which the parser kept the last -- the caller's
  vanished, three rows came back where two were asked for, and the ending named the
  caller's text as the stopper of a read it had not stopped. The text is now a
  sub-select under the layer's bound, so both stand: the caller's applies to the pattern
  it was written against and the layer's to whatever that resolves to.

  Wrapping alone was not enough, and the first attempt at it broke a strictly larger
  class than it fixed. A sub-select carries no prologue in the grammar -- `PREFIX` and
  `BASE` are `Prologue`, which appears once, at the front of a whole query -- so a text
  declaring either, which is how essentially all SPARQL is written, stopped parsing the
  moment it was wrapped, and the caller was handed a syntax error at a byte offset of a
  query the layer wrote. That included the very case the wrap existed for, whenever the
  text was prefixed. So `StratumUnit::new` now reads the text once, with the front end
  the executor runs, and emits the caller's directives *in front of* the wrapping
  `SELECT` with their query form inside it. It is the text that moves and nothing that
  is re-rendered: a prefixed name is resolved away at parse time, so the prologue is not
  recoverable from the algebra, and re-serialising the body would rewrite the argument
  lists a relation call is spelled with into the blank-node chains a registry-unaware
  parse lowers them to.

  That parse is also the refusal the wrap was said to be an alternative to. The earlier
  reasoning -- that refusing a text would mean parsing it, which this layer does not do
  -- was a false choice twice over: the wrap imposed a sub-select grammar constraint
  without parsing anything, and the parser the executor runs is a direct dependency. A
  text that is not a query is now refused at construction by name
  (`UnitError::NotAQuery`), carrying the parser's own diagnostic, instead of surfacing as
  one stratum's `ExecutionFailed` after a bundle was assembled and its other strata were
  read. The parse judges grammar and nothing else -- it runs with no relation registry,
  so a registered predicate is an ordinary triple pattern to it, and the registry-aware
  parse at execution stays the authority on the seam, with its refusals still reported
  as that stratum's own status. `ProducerStatus::ExecutionFailed` for an empty supplied
  text is gone with the condition: an empty text is not a query, so it cannot reach a
  bundle.

  The same parse names the query form, and a form the wrapper cannot hold is refused by
  that name (`UnitError::NotASelect { form }`). An `ASK`, `CONSTRUCT` or `DESCRIBE`
  supplied to the seam used to build a unit and fail at execution with the same
  byte-offset diagnostic a prologue produced -- the grammar admits only a `SELECT`
  inside a sub-select -- and none of the three yields a solution row to rank, so no
  text of those forms could ever have run and nothing valid is refused. Every `SELECT`
  is still admitted whatever it carries: a dataset clause, a trailing `VALUES` and a
  `VERSION` directive are each executed beside the refusal, over a dataset where the
  clause selects different rows from the one the store would have answered with.
- **retrieval:** A supplied query's dataset clause reads the graphs it names. The wrap is
  a sub-select, `SubSelect ::= SelectClause WhereClause SolutionModifier ValuesClause`
  has no `DatasetClause` in it, and the clause was being carried inside the wrapper --
  where the parser read it and then discarded it. So a caller writing
  `FROM <a-graph-that-does-not-exist>` got rows, read out of the very default graph the
  clause excluded, under an ordinary `SuppliedQueryEnded` ending with no refusal and no
  diagnostic anywhere. A wrong answer is worse than a broken one, and this one was
  invisible to a suite whose supplied-text fixtures all ran over an empty dataset
  against a registry-driven relation, where which graphs a clause selects cannot change
  a row.

  The clause is now written onto the wrapper's own `SELECT`, which is the one position
  the grammar leaves for it and the one that scopes the body the caller wrote it around,
  and it is excised from the body. That excision is the only edit the wrap makes to a
  caller's bytes: it moves as *text*, split at the range the parse reported, for the
  reason the prologue does -- re-rendering the body would rewrite a relation call's
  argument lists into the blank-node chains they lower to, and the registry-aware parse
  at execution would no longer see a call. A text carrying neither clause emits the bytes
  it always emitted, at every depth.
- **sparql-algebra:** A dataset clause inside a sub-select is refused instead of parsed
  and discarded. `SubSelect` has no `DatasetClause`, so a `FROM` there is not a
  production; this parser read one anyway -- one function serves both `SELECT`
  positions -- and the sub-select site then unwrapped the `Query` with `..`, dropping the
  clause along with the base IRI and the version. The clause is now refused as a
  `ParseError::Syntax` at its own `FROM` keyword, naming that a dataset clause is written
  on a whole query because that is the scope it applies to, and the site destructures
  every field explicitly so a field added later cannot start being dropped silently.
  Nothing valid is caught: a sub-select without the clause, a whole query with it, and a
  whole query with it that also contains a sub-select are each executed beside the
  refusal.
- **retrieval:** A compiled bundle is checked against the set it was assembled with
  before any unit runs (`ExecutionError::UnitsNotAsAssembled`). The unit list and each
  unit's stratum were writable, so a unit removed from the bundle yielded a narrower
  answer under the genuine plan identity, and two units' strata swapped attached each
  producer's evidence to the other. The bundle now records, privately, which stratum
  each position was assembled under; a count that moved or a stratum that moved is
  refused by name, with the count reported first because a removal shifts every
  position after it.
- **retrieval:** A producer declaring no rows that returns one is refused. The breach
  check sat inside the depth-cut arm on the reasoning that the emitted bound never
  asks for a second row past the declaration, which is false at a declared zero: the
  floored depth of one is emitted one row deeper like any other, so the first row
  already breaches and was being reported as an exhaustion of one row. The check runs
  first now, and a declared zero over an index holding anything is
  `ExecutionError::RowBoundBreached` like a wrong declaration of any other size.
- **retrieval:** A bounded request narrows a stratum's depth to `k` only where that
  stratum also declared `DuplicatePolicy::Unique`. The narrowing rested on the
  candidate-domain declarations alone, and the merge argument behind it counts
  ranks: a candidate ranked past `k` is beaten by the `k` candidates above it in
  its own stratum. That step reads a count of ranks as a count of candidates, which
  is true of a `Unique` stream and false of an `Allowed` one -- a repeat there is
  validated, charged to the producer and then discarded, which is precisely what
  that declaration asks a consumer to do. A depth-`k` prefix of such a stream
  therefore carries `k` rows and can carry fewer than `k` candidates, so it is no
  longer a superset of the top `k`: over one stratum the answer came back short a
  row, and with a second stratum to fill the gap it came back the right length with
  the wrong row in it, reported as exact with nothing in the trailer to distinguish
  it. An `Allowed` stratum now keeps the declared-or-measured depth it always read
  and answers exactly as the same request answers with no declaration at all;
  `Unique` keeps the bounded read unchanged, which is what makes the condition a
  premise rather than a retreat. No shipped producer declares `Allowed`, so no
  released answer moved; the seam publishes the declaration, `register_ranked`
  accepts it, and the fusion layer pays for it, so a host's own producer could
  reach this.

- **geo:** A GeoSPARQL index can now be built before the geometries it will hold
  have landed. `GeoIndex::from_dataset` refused outright when a
  `GraphSelector::Named` graph was not interned in the dataset, on the argument
  that a configuration pointing at an absent graph is a wiring mistake. A graph
  IRI is interned only once a quad is in that graph, so the check made a
  graph-scoped index unbuildable until its data arrived -- and an index standing
  ready before the load is an ordinary operating state. The same function already
  read an absent serialization property as "an ordinary empty match, not a
  configuration error", so the two halves of one condition were answered two
  different ways. Graph resolution is now infallible and an absent graph yields
  the empty index, which is the posture `purrdf-text` takes for the identical
  selector, so a host wiring both crates from one configuration no longer gets an
  index from one and a refusal from the other.

  The empty index is a complete index, built through the ordinary steps rather
  than a second path: no entries, one empty asserted vector per spatial relation
  so every relation stays answerable, and a `source_fingerprint` from the same
  digest call the populated projection uses. The configuration is digested before
  any content, so two empty indexes under different configurations still differ,
  and the value moves the moment the first geometry lands. `verify_binding` is
  untouched and still compares digests over the rows actually projected.

  What is not relaxed: an empty serialization list is still refused. That is a
  configuration with no subject rather than a corpus with no rows, and this
  toolkit mints no vocabulary to guess one. A `GraphSelector::Named` holding a
  non-IRI is still refused at configuration time.

- **text:** The case-folding skew is measured per standard-library Unicode vintage
  instead of against one. The crate carries a fold table (`caseless`, 16.0.0)
  that trails the standard library's case-mapping tables, and pins the exact set
  of code points the two disagree on so that a table moving is seen rather than
  absorbed -- but the standard library's tables move with the toolchain, which
  this repository floats, and the pin named one vintage. The day the nightly
  picked up Unicode 18.0.0 the skew grew from 57 code points to 98 (four IPA and
  Latin Extended-E letters that gained capitals, their capitals, sixteen new Latin
  Extended-G case pairs and one new ligature) and the suite went red on a table
  the crate had measured nothing about. The skew test and the version pin now key
  on `char::UNICODE_VERSION`: 17.0.0 and 18.0.0 are each pinned as an exact set, a
  vintage outside them fails by name, and every failure prints the runs it
  measured so one run on a new toolchain is the measurement. The guarantee the
  skew rests on is restated precisely: a text containing no character the
  standard-library release itself introduced analyzes to the token vector it
  always did -- the four pre-existing lowercase letters still fold to themselves,
  and only their newly minted capitals fail to reach them.

- **text:** A text index can now be built before the documents it will hold have
  landed. A configured predicate the dataset has not interned contributes no rows
  instead of failing the build, and so does a `GraphSelector::Named` graph the
  dataset has not interned. The limiting case is what decided it: an empty dataset
  interns no term at all, so the presence check made an index over an empty corpus
  impossible -- and it made the recommended single-partition configuration (one
  named graph) the hardest one to start from, because a graph IRI is interned only
  once something is in that graph. It also put the shipped text producer
  permanently out of reach of the state the ranked-retrieval contract is written
  for: a producer whose declared row bound is zero is invoked anyway and reports
  its own exhaustion, so emptiness arrives as the producer's receipt rather than as
  a verdict reached without asking it.

  An empty index is a complete index. It holds zero documents, zero terms and zero
  partitions -- a partition carries at least one document, so BM25's average
  document length is never divided by a zero that does not exist -- and it still
  attests a generation, because the configuration, the ranking law and the
  analyzer's Unicode versions are digested before any content. Two empty indexes
  under different configurations are therefore distinguishable, and the value moves
  the moment the first document lands. Every relation measured over it declares a
  row bound of zero in every mode, `ranked_declaration` serves it (the rank law
  holds over a stream with no rows in it), and a search answers with zero rows
  through the ordinary cursor path.

  What the relaxation costs is a mistyped predicate IRI's share of the corpus, and
  that cost is paid by a detector rather than absorbed: see
  `TextIndex::source_coverage` below. A presence check was never the detector -- it
  accepted any IRI the dataset interned anywhere, including in an unrelated
  position, and it condemned a correctly spelled predicate whose objects are all
  IRIs, which contributes no text and has never raised anything. The
  multi-partition refusal in `ranked_declaration` is untouched.

- **text:** Letting an index build over a predicate the dataset carries nothing
  for traded a hard failure for a silent partial corpus, and named a detector that
  could not detect it. `verify_binding` recomputes the source digest under the
  *same* configuration the index was built under, so a mistyped predicate agreed
  with itself: five configured predicates with one document each and one character
  wrong in one IRI gave `document_count = 4` and a clean verdict. The other signals
  -- a zero document count, a declared row bound of zero -- fire only when *every*
  configured predicate is wrong, not the one-of-five case.

  `TextIndex::source_coverage` now reports what the walk found, because the two
  empty states are distinguishable and only one of them is a mistake. A dataset
  holding no statement in any of its three layers is an index standing ready before
  its documents land: every configured predicate is unrepresented for that one
  reason, `SourceCoverage::shortfall` is `None`, and nothing complains -- the
  capability the relaxation was made for is unchanged, generation and all. A
  dataset that holds statements and carries none under a configured predicate is a
  shortfall, and `shortfall` names the predicates. A `GraphSelector::Named` graph
  the dataset does not hold lands there too, because it leaves every configured
  predicate with nothing in scope. `verify_binding` checks the shortfall against
  the dataset in hand after it checks the digest, and names the predicates in its
  message.

  Representation is counted per **statement**, in either RDF 1.2 layer and inside
  the configured graph scope -- not per literal row, and not by the predicate IRI
  being interned somewhere. A predicate whose objects are all IRIs is therefore
  found rather than reported, and a predicate carried only by the annotation side
  table is found rather than reported, which a coverage taken from the asserted
  table alone would have got exactly backwards for this crate's headline case. The
  dataset is probed for a single statement only when something came up
  unrepresented, and each probe stops at the first row.

  One case is reported that is not a mistake, and it is reported deliberately: a
  host loading one predicate's data before another's is the same observation as a
  typo -- a configured predicate with no statement, in a dataset holding other
  things -- and nothing in the data separates them. So this is a report and never a
  refusal at construction: the index builds, answers and attests its generation
  either way, and a host that means it reads the coverage instead of asking for a
  verdict. The coverage is in neither fingerprint, because it cannot change an
  answer: two datasets whose literal rows agree answer alike whether or not one
  also holds a non-literal statement under a configured predicate.

- **retrieval:** A stratum whose planned depth already equalled its producer's
  declared row bound was reported `ProducerStatus::Exhausted` -- the one ending
  that names no stopper -- for a read that bound had cut, with
  nothing anywhere saying so. The depth probe is the row that tells a read which
  ran out from a read which was stopped, and it was emitted at
  `min(depth + 1, declared)`: at that one depth the `min` selected the
  declaration, the unit was emitted at exactly its own depth, and there was no
  probe slot left to answer the question. It is now `depth + 1`, at every depth and
  against every declaration -- the declared row bound does not cap the emitted bound
  at all, because admission has already refused a depth above that declaration, so a
  `min` over the two selects the depth in every case a unit can be emitted for. Only
  the emitted `LIMIT` moves; the recorded depth is what admission holds a plan to and
  is unchanged, as are every plan field, identity and planned-resolution number
  keyed to it.

  A declared row bound of **zero** was the one arm where the two numbers parted, and
  it was the arm the `min` got wrong: the planner floors that depth at one, and a
  bound capped to the declaration was then `LIMIT 1`, a bound *equal* to its own
  depth. No row past it could arrive, so the stratum was certified `Exhausted`
  whatever the index turned out to hold -- an index that really was empty and one
  holding nine rows produced byte-identical trailers. A zero declaration is read
  rather than obeyed everywhere else in this layer, so it is read here too. An empty
  index now reports `Exhausted { rows_emitted: 0 }` as a verified claim, and an index
  that turns out to hold rows breaches its declaration by name exactly as a wrong
  declaration of any other size does.

  A row arriving in that slot is past the declaration rather than merely past the
  depth, which means the producer yielded a row it promised did not exist. That
  is refused by name -- a new `ExecutionError::RowBoundBreached` carrying the
  stratum, the declared bound and the count actually returned -- rather than
  truncated and certified as exhaustion. It is a whole-run refusal and not a
  per-stratum status, because the broken number ordered that call against the
  other operators of its group and admitted every depth in the plan, and because
  `ProducerStatus::ExecutionFailed` says the producer could not run while this
  one ran and returned rows. An honest producer pays nothing for the slot: it
  returns the rows it declared, the slot comes back empty, and its exhaustion is
  now verified rather than believed.

  The depth *argument* handed to a producer that declares a depth placement keeps
  the older `max(1, min(depth + 1, declared))`, and that split is the point: a
  `LIMIT` is a ceiling the evaluator applies to a cursor the producer never hears
  about, while the argument is a request the producer reads and checks. Asking
  for `declared + 1` there asks a producer to exceed its own registration, and
  the nearest-neighbour relation correctly refuses a `k` above its configured
  guard -- so probing on the argument would have turned a valid query into a
  refused one. The unit's own bound still reaches one row past the declaration,
  so a self-bounding producer that returns more rows than it declared is still
  caught; it is simply never asked to.

  `StratumUnit` carries the registry's declaration for the stratum, readable as
  `declared_rows()`, which is what lets the executor tell the two kinds of extra row
  apart. `None` there is a registry that declared no access mode and therefore no
  bound, which promises nothing for a row to breach.

- **retrieval:** At the deepest depth a plan can express, that probe row vanished
  again and `ProducerStatus::Exhausted` was minted from a bounded read. The emitted
  bound is the depth plus one row, and the addition saturated -- so at a depth of
  `u32::MAX` the emitted `LIMIT` *equalled* the depth, no row could ever arrive
  past it, and every such read was reported with the one ending that names no
  stopper however many rows the relation still held. That is the fault the
  probe row exists to close, surviving at the one depth where the mitigation was
  dropped: a saturating operator looked like arithmetic hygiene and was a silent
  completeness claim.

  Both ends of the depth range are refused by name now, rather than one floored and
  the other saturated. The admission waist refuses a depth whose probe row is
  inexpressible as `AdmissionError::DepthWithoutProbe`, the mirror of
  `AdmissionError::ZeroDepth` and enforced beside it, and it hands the compiler an
  admitted depth as a type that cannot carry the refused value -- so a unit whose
  `LIMIT` equals its own depth is unwritable rather than merely unwritten, and the
  row past the depth is added with exact arithmetic.

  The planner no longer truncates a derived bound to `u32::MAX` either, and the two
  sides of that ceiling are now handled differently because they are two different
  kinds of number. A *declared* row bound past the ceiling is recorded **at** the
  ceiling: the truncation's real defect was not the clamp but that it recorded a depth
  below the bound it claimed to serve with nothing anywhere reporting the difference,
  and that at `u32::MAX` exactly it also left no room for the probe row. At the
  readable ceiling the probe row fits, so a read this ceiling cuts arrives with a row
  past the depth and ends `ProducerStatus::DepthReached`, which names the planned
  depth as the stopper -- the comparison the truncation lacked. A *requested* bound
  past the ceiling is refused, as `PlanError::ReadBoundBeyondDepthRange`, once at the
  request itself rather than wherever it happens to bind: that number is the caller's
  own, and deferred it was served silently against a registry whose declarations were
  smaller and truncated against one whose were not.

  Declaring more rows per invocation than a read can be taken to is therefore served
  rather than refused, and it had to be. An honest declaration of a ten-billion-row
  index read for a top-ten answer is an ordinary request, and it is ordinary for
  exactly the shapes that cannot narrow a depth to that answer's bound -- a producer
  whose rows are not its candidates (`DuplicatePolicy::Allowed`), or one that
  restricts no block of the candidate universe (`CandidateDomains::Unrestricted`,
  which is the shipped nearest-neighbour relation's own documented default). Refusing
  those left a host two ways out and both were dishonest: under-declare
  `rows_per_invocation`, or fabricate a cardinality statistic.

  `PlanError::ReadBoundBeyondDepthRange`'s ceiling is the deepest *readable* depth
  rather than `u32::MAX`, so the sentence it prints is true: a bound of exactly the
  number it names plans, records that depth and is emitted with its probe row, while
  the number it used to name was then refused where it bound.

  Nothing an ordinary request reaches moves. Both boundaries begin at a read of four
  billion rows from one producer per invocation; the deepest depth below them
  still plans, admits and compiles, with its probe row present and its emitted
  bound exactly where it was, and no plan identity, recorded depth or emitted
  `LIMIT` changes anywhere else.

- **retrieval:** One class of producer had its exhaustion minted from a
  declaration rather than read off the data, and it is the shipped
  nearest-neighbour relation's own shape. A producer that declares a depth
  *placement* is handed the number instead of being bounded by a `LIMIT`, and that
  argument is never raised past the row count it registered -- asking for more asks
  the relation to contradict its own registration, which a conforming relation
  refuses. So at a depth already sitting on that declaration the read is asked for
  exactly `depth` rows, returns at most `depth` rows, and the slot past the depth can
  never be filled however many rows the index holds. The stratum was reported
  `ProducerStatus::Exhausted` anyway, which made an index of a thousand rows and an
  index of exactly `depth` rows indistinguishable in every field of the answer. The
  depth cannot rise to go looking, either: the derived depth is bounded by the
  declaration, so no plan asks for more.

  How that read ended is genuinely unobservable, and neither existing ending was
  true of it -- `DepthReached` would blame a planned depth that cut nothing, and
  `Exhausted` would claim the rows ran out when nobody could know. So there is a
  sixth ending, named for the stopper it actually had:
  `ProducerReceipt::RowBoundReached`, `ProducerStatus::RowBoundReached` and
  `StreamEnding::RowBoundReached`, all carrying the rank the read stopped at, spelled
  `"row_bound_reached"` in the Python answer's `"statuses"` beside its `"rank"`.
  `Exhausted` remains the only ending that names no stopper, and a consumer
  that wants this read taken further has one honest move, different from either
  neighbour's: raise the producer's declared row bound.

  It is reported only where the ending really is unobservable. A producer the
  evaluator bounds always receives the probe row and is unaffected. A self-bounding
  producer planned *below* its declaration receives the probe too and still reports
  `DepthReached` or `Exhausted`. One that returned fewer rows than it was allowed is
  `Exhausted`, verified, because it stopped before anything stopped it. And one that
  returns more rows than it declared is still caught by the unit's own bound and still
  refused as `ExecutionError::RowBoundBreached`.

- **retrieval:** `StratumUnit`'s depth was a plain public `u32`, which re-opened at
  the compile/execute boundary the exact hole the admission waist closes one stage
  earlier. A bundle whose depth was raised past the range its emitted bound can probe
  reported `Exhausted` for a read the `LIMIT` cut, and one whose declared row bound
  was lowered below its depth bypassed the waist's `DepthBoundViolation` dimension --
  on the one number all of this is about. Admission refuses a hand-edited *plan* on
  both of those, for the reason every dimension is re-derived at the waist; trusting a
  hand-edited *bundle* on the same two was the same fault with one stage skipped.

  The depth and the declared row bound are now private, read through
  `StratumUnit::depth()` and `StratumUnit::declared_rows()`, and reachable only
  through a checked constructor: `StratumUnit::new` takes both numbers and refuses a
  zero depth, a depth whose probe row is inexpressible, and a depth above the declared
  bound as variants of a new `UnitError` -- joined, per the entry above, by a supplied
  text the parser refuses. Who applies the depth -- the
  evaluator as a `LIMIT`, or a producer handed it as an argument -- is not an input
  beside them: it is read off the query the compiler assembled, because it is the same
  fact. The seam stays open, because handing the executor a query of one's own is what
  it is for. The entry below carries that half the rest of the way: no bound the ending
  depends on is text a caller can write, and a caller's own text is never certified.

- **retrieval:** A stratum's ending was decided from four things and only three of
  them were checked. The fourth was the bound written in the unit's own query text,
  which was a public, writable `String`, and the failure was the natural off-by-one: a
  caller assembling a bundle by hand writes `LIMIT <depth>`, because the depth is the
  number this layer reasons about everywhere. That text leaves no slot for the probe
  row, `execute` writes `DepthReached` only when a row arrives *past* the depth, so an
  evaluator-bounded `Unique` producer holding nine real rows read at depth three
  reported `Exhausted { rows_emitted: 3 }` -- the one ending that names no stopper,
  for a read with six rows behind it. Nothing downstream could catch it
  either: the rows emitted equalled the rows pulled, so the fused trailer above it read
  `Exact`.

  The first fix rendered the unit's *outer* bound from the depth instead of storing it,
  and the entry below is why that was not enough: the bound moved inward rather than
  out of reach. The refused alternative was inspecting the trailing `LIMIT` and
  comparing it, which re-reads a number the layer already holds and can only ever
  refuse a caller for writing what the layer writes itself.

- **retrieval:** The bound the read is taken under was pulled out of one writable
  string and left inside another, so the same false completeness claim came back with
  the same numbers. A `StratumUnit` carried a public `body` and rendered only the
  *outer* `LIMIT` from the depth -- but every emitted body carries a bound of its own:
  `LIMIT depth + 1` on the branch of a producer the evaluator bounds, or the rendered
  depth *argument* of one that bounds itself. Where an inner bound and an outer one
  disagree the inner one decides the read. Lowering the branch's `LIMIT` from four to
  three on a bundle compiled for a nine-row producer at depth three returned
  `Exhausted { rows_emitted: 3 }` where the same bundle unedited reported
  `DepthReached { rank: 3 }`, and lowering the depth argument of a self-bounding
  producer did the same -- verbatim the defect two earlier fixes each closed one copy
  of.

  No number the ending depends on is held as text now. A compiled unit carries its
  query as the parts it was assembled from -- the producer IRI, the argument slots the
  request placed, the positions the candidate and block columns are read from -- and
  `StratumUnit::sparql()` renders the whole query on every read, with the branch's row
  ceiling and the unit's own bound both computed from the one proved depth. The branch
  bound is not decoration and is not deleted: it is the row ceiling the evaluator
  pushes down to the relation, so it is rendered rather than written. The compiled text
  is byte-identical to what it was, at every depth and for both producer shapes.

  Driving the executor over a query of one's own stays open through
  `StratumUnit::new`, and such a unit is now a different kind of unit rather than a
  differently-spelled one. Its query form reaches the evaluator byte for byte -- except
  that a dataset clause, which the sub-select grammar has no place for, is moved to the
  wrapper, where it scopes the same body -- this
  layer bounds only its outside, and what the text bounds inside itself is no part of
  what this layer reads of it -- so that read is never certified `Exhausted`. It reports the new ending
  `StreamEnding::SuppliedQueryEnded`, surfaced as `ProducerReceipt::SuppliedQueryEnded`
  and `ProducerStatus::SuppliedQueryEnded`, which names the caller's own text as the
  stopper and claims nothing about what lies below the rank it reached. `Exhausted`
  remains the only ending that names no stopper, in a vocabulary that now has seven
  spellings rather than six.

  The refusal is exactly as narrow as the observation. A caller's query still runs,
  still yields its rows in rank order, still reports `DepthReached` when a row past the
  depth really did arrive -- that is an observation whatever bounded the text -- still
  reports its parse failure as that stratum's status when it cannot be prepared, and
  still trips `ExecutionError::RowBoundBreached` when the producer beats its own
  declaration. Refusing to run such a text instead would have been the over-refusal:
  most of those queries are perfectly good, and what cannot be done honestly is certify
  a completeness claim from one. `DepthApplication` is gone with the field it was
  needed beside.

- **retrieval:** The top-k narrowing was withheld from a single-stratum plan whose
  producer declares `CandidateDomains::Unrestricted`, where the disjointness premise it
  was withheld for is vacuous. Disjointness is a statement about pairs; with one
  surviving stratum there is no pair, every candidate the answer can hold was named by
  that one stream, and its rank order *is* the fused order -- so a `k`-row prefix is `k`
  candidates under `DuplicatePolicy::Unique` alone, with no domain declaration needed.
  The cost fell on the read rather than on the answer, which is why it was quiet:
  identical rows, identical scores, and a top-five over a declared ten-billion-row index
  recording a depth of 4294967294 and emitting `LIMIT 4294967295` where the same answer
  came from five rows. Neither shipped ranked relation declares a domain, so this was
  the ordinary single-index configuration rather than an exotic one. `licensed_prefix`
  now returns the bound for exactly one surviving stratum whatever its domains; the
  `Unique` condition still applies, and a second surviving stratum restores the pairwise
  rule unchanged, re-derived per request from the registry in hand. Proved by a
  differential through `search` at three bounds -- the narrowed answer is the
  un-narrowed answer's prefix, row for row, score for score and stratum rank for
  stratum rank.

- **retrieval:** A producer declaring the genuinely unbounded `u64::MAX` rows per
  invocation, with no cardinality statistic and no licensed prefix, was refused, while
  the same producer declaring ten billion was served at the deepest readable depth and
  reported `DepthReached`. Both reads are cut at the same depth by the same bound with
  the same probe row, so the refusal separated a declaration from its own neighbour --
  `u64::MAX` refused, `u64::MAX - 1` served -- and bought nothing the ending does not
  already report. The refusal predated the clamp that made it obsolete: it existed
  because recording `u32::MAX` claimed a bound no producer declared *and* left the
  compiler no room for the probe row, and recording the read ceiling instead fixed both.
  Such a stratum is now recorded at that ceiling like any other oversized declaration,
  with its probe row one past it, so a read the ceiling cuts ends `DepthReached` and
  claims nothing about the rows below it. The neighbours are executed rather than
  reasoned about: the unbounded declaration, the one below it and an ordinary
  declaration a read can reach all plan, and the last is untouched.

- **retrieval:** A fusion test counted rank collisions in a fixture that cannot
  collide. It walked the profile's curve over the ranks its bounded read pulled and
  required the trailer's collision counter to equal that walk, "not an estimate from the
  profile's bound" -- but at the fixture's weight of `Fixed::ONE` the first colliding
  rank is 1000941, so the walk was provably zero over a read of seven ranks and the
  assertion was `0 == 0`. It would have passed with the counter emptied, and with the
  estimate its own message forbids. It now runs at the weight whose decay collides from
  rank two, which is the weight its two sibling tests use to enter that regime, and
  carries the non-vacuity guard those siblings carry: the pulled ranks must contain a
  collision before the count over them is asserted. The separation half of the test,
  which was never vacuous, is unchanged.

- **python:** A ranked producer declared with more than one candidate-domain tag
  failed at the wrong time. One tag entails where every row of that producer
  lies, so a consumer reads the block off the declaration; several tags say only
  that the rows lie somewhere in the set, which obliges the producer to name each
  row's own block -- and both ranked relations this binding can build project a
  candidate and a score and declare no such column, because a tag describes how a
  host's corpora partition and only the host knows that. The declaration was
  therefore unsatisfiable by construction, and the fusion said so at the first row
  it pulled, after the plan, the compile and the first read had all been paid for.
  It is refused at registration now, where the caller can act on it, naming the
  producer, quoting the blocks in canonical order and naming the three exits that
  work: one tag, one producer per block, or `domains=None`. A list that repeats
  one tag names one block and still registers, and single-tag declarations are
  untouched -- including the shorter read they buy.

- **python:** `MutableDataset._store_capsule` is declared in the type stub. The method
  went live so `Shapes.validate_store` could reach a mutable dataset's frozen snapshot
  by name, and the stub never said so — a member the stub omits is one a checked
  caller cannot see at all, including to see that it is private. The stub-parity
  gate is what caught it.

- **python:** Two `text_producers` entries claiming one stratum are refused with a
  `ValueError` naming both producers and the stratum. The registry underneath
  enforces one-stratum-one-producer with a panic, which is the right shape for Rust
  code assembling a registry and the wrong one at this boundary: it crossed into
  Python as `PanicException`, which derives from `BaseException` and slipped past a
  host's `except Exception`, taking the process down with a thread dump on stderr
  where every neighbouring misconfiguration on the same surface raises by name. The
  scan runs after the declarations are sorted, so the two names reported are a
  function of what was declared and not of the dict's insertion order, and the
  message carries the same guidance the registry's own refusal does: shards whose
  scores are comparable belong inside one producer, producers scoring by different
  laws belong in two strata. The same two producers under two strata fuse as before.

- **python:** A host that ran a compiled retrieval unit's SPARQL itself had no way
  to learn how many of its rows it was allowed to report. The compile stage emits
  the text at most one row deeper than the plan reads, and that last row is a
  probe that exists only to tell an exhausted producer from a depth-cut read: it
  is read and never reported. Rust callers have carried the reportable bound on
  the unit all along; the Python unit dict exposed only `"stratum"` and
  `"sparql"`, so the text's `LIMIT` was the only number in reach and it is the
  wrong one. Each unit now carries `"depth"` beside its text -- the bound the
  plan itself recorded, not a number re-derived from the emitted `LIMIT` -- and the
  surface states the obligation: keep at most `"depth"` rows. The emitted `LIMIT`
  reaches one row past the depth at every depth a plan can carry -- including a
  depth that already sits on the producer's declared row bound -- so the text's own
  number is never the reportable one. `"planned_resolution"` was no fallback
  either: it is empty unless the call names a fusion law.
- **python:** Two binding tests over the compiled unit claimed coverage they did not
  have. The one for a probe row emitted where the producer's declaration leaves no
  room asserted only that the emitted bound is one past the depth -- which holds for
  every unit -- so it was its own sibling under a second name, and it could not
  detect a bound wrongly capped at the declaration, the single regression it exists
  for, because it never checked that the depth had reached the declaration. Both
  tests now assert the configuration they rest on, read off the unit's
  `"declared_rows"`: equal to the depth in the one, strictly above it in the other.
- **python:** The test over the six terminal status spellings asserted them against
  `retrieval.search`'s docstring -- the binding suite's only assertion on a
  `__doc__`. A docstring that contains a string proves nothing about which string
  the mapping emits, and that test would have passed with the mapping deleted or
  emitting the wrong word. It now pins the three endings this surface can actually
  produce (`"exhausted"`, `"depth_reached"`, `"ceiling_reached"`) and says which
  three it cannot, with the reason recorded beside the refusals in the binding's own
  header: `"row_bound_reached"` needs a self-bounding producer -- one whose
  declaration places the depth as an argument the producer reads -- and the one
  relation this surface registers places none; `"terms_rejected"` is a receipt a
  producer writes for itself; and `"execution_failed"` needs a unit whose text could
  not be prepared or run. All three remain live for a host driving the Rust surface
  with a bundle of its own, and all six stay spelled and mapped.
- **python:** The stub-signature sweep held three engines to the built extension and
  not the fourth. Its own docstring explained that the sweep runs over each engine's
  whole surface so a future entry point is covered without anyone remembering to add
  it, while the engine list was a hand-written triple that `retrieval` was missing
  from -- so the entire `class retrieval:` stub shipped with nothing mechanically
  holding it to the PyO3 signatures that mypy approves callers against. `retrieval`
  is in the sweep and its six entry points match their bindings. The vacuity guard
  is now per engine as well as overall, because a total floor cannot tell a sweep
  that visited every engine from one whose stub block stopped parsing.
- **python:** The `retrieval.compile` stub claimed one exception to the emitted
  bound: that a producer declaring no rows at all is read with a `LIMIT` equal to
  its `"depth"`. The text carries `depth + 1` unconditionally, and the number a
  self-bounding producer is *asked* for is the one its declaration caps -- two
  different numbers -- so the sentence contradicted the paragraph two above it and
  told a host running the text itself to expect a bound it will never see.
- **retrieval:** A fused answer could contain the same entity twice. Certifying a
  candidate removes it from the frontier, and the check that held a stream to its
  declared uniqueness read only the frontier, so a stream naming that entity again
  afterwards started a fresh candidate and the answer carried it a second time --
  with a score summed from one stratum instead of all of them. The invariant a
  consumer needs is unconditional, for any stream set, any declared policy and any
  bound, so it is now enforced against every arrival rather than against the window
  that happened to be open: a candidate that leaves the frontier by certification
  leaves behind the set of streams that named it, and a stream that names it again
  is refused by name. A stream declaring that repeats happen is unaffected -- its
  repeat is dropped before it becomes a head, which is what that declaration asks
  the consumer to do. The cost is charged against emissions rather than pulls, so a
  bounded call holds at most as many entries as rows it returns however long the
  streams are, and what a uniqueness declaration buys its way out of is still
  bought: there is no per-row identity set per stream.
- **retrieval:** A fused top-k drained every stream. Certification asks whether any
  open stream could still name a candidate, and with nothing to answer that
  question it had to assume every open stream could -- so over strata whose
  candidate sets do not overlap, no candidate was ever final while another stream
  remained open, and a top-ten over two million-row strata read two million rows
  and grew a frontier to match. Weakening the test was not available: the engine
  has no random access, so the only way to learn that a stream does not name a
  candidate is to read it to its end, and certifying sooner would emit a score
  missing a contribution and call it exact. Exact scores and a bounded read are
  jointly reachable only if fusion is told which candidates a stream can name, so a
  producer now declares its candidate domains and the finality test keeps its
  meaning while gaining a smaller quantifier. Two thousand-row disjoint strata at a
  bound of five read four ranks each where they previously read a thousand, and a
  stratum that names everything keeps the bound for its neighbours instead of
  collapsing it. The answer is unchanged: rows, scores and provenance are identical
  to the draining run and to an independent oracle across hundreds of
  configurations, and the ranks read never rise as a declaration is refined.

  What the bound bounds is stated exactly, because the number above invites a wider
  reading than it earns: it is the ranks **fusion pulls from a stream**, and the
  frontier it therefore has to hold. It is not the producer's work. Each stratum's
  rows are materialized by the evaluator up to the depth the plan recorded before
  fusion pulls anything, so a stratum planned a thousand deep is read a thousand
  deep whatever the bound later turns out to need — the evaluator's egress model is
  a complete answer, with no cursor surface for a consumer's bound to reach back
  through. A ranked producer's own cursor is lazy and stays lazy; what is not lazy
  is the boundary between it and this layer.
- **retrieval:** Fusion no longer refuses a well-formed producer stream when the
  profile's own fixed-point decay gives two adjacent ranks one contribution. A
  producer's ordering declaration is a claim about **ranks**, and the check was
  applied to **contributions** -- a value the consumer computes from the decay
  rule, `K`, the stratum weight and the rank, re-derives on arrival and refuses
  on mismatch, and which the producer supplies no term of. Ranks are separately
  held contiguous and ascending for every stream, so an equal adjacent pair only
  ever meant the arithmetic had stopped separating those ranks at that depth. A
  conforming stream read deep enough was rejected for the consumer's own
  quantization: at a weight of `10^-6` this began at rank 973, and no weight at
  all postpones it past about a million ranks under the truncated rule. The
  condition is now measured and reported rather than refused; past that depth the
  declared tie-break is total, so the answer stays correct and deterministic at a
  coarser rank resolution.
- **sparql-algebra:** A repeated `LIMIT` or `OFFSET` is refused instead of
  overwriting the bound the caller already wrote.
  `LimitOffsetClauses ::= LimitClause OffsetClause? | OffsetClause LimitClause?`
  admits at most one of each, in either order, but the parser read them in an
  unbounded loop that reassigned the field every pass -- so `LIMIT 2 LIMIT 13`
  parsed, the bound of two vanished, and a query asking for two rows returned
  thirteen. A silent drop of the caller's own bound is the mirror of an
  over-refusal, and the loop made it reachable from every SPARQL entry point at
  once: the same parse backs `SELECT`, `CONSTRUCT`, `DESCRIBE`, `ASK` and the
  sub-select. Accepting it was not leniency anyone could rely on either, because
  the query means one thing to this engine and another to a conforming
  processor. The refusal is a typed syntax error naming the clause that repeated
  and pointing at that repeat's keyword, and the reading it replaced is the only
  one that moved: both clause orders, either clause alone, neither clause,
  `LIMIT 0` as a real zero-row bound, and a bound past any plausible row count
  all parse exactly as before. No other solution modifier had the overwrite
  shape -- each of the others is read by a single conditional, so a repeated
  `GROUP BY`, `HAVING`, `ORDER BY` or `VALUES` was already refused, and that is
  now pinned alongside.

### Removed

- **retrieval:** `PlanError::StatisticsUnavailable`. It was the refusal of an unbounded
  row-count declaration that no statistic narrowed, and nothing raises it any more: such
  a stratum is recorded at the deepest depth a read can be taken to, exactly as every
  other declaration larger than a read can reach already was, and the read's own ending
  names the planned depth as the stopper. Leaving the variant in place would have left a
  public error whose documentation described a plan this planner no longer refuses.
  `PlanError` is `#[non_exhaustive]`, so a caller matching on it already carries a
  wildcard arm; one matching this variant by name now matches a refusal that cannot
  arrive.

- **sparql-eval:** `RelationWitness::canonical_bytes`, and the encoding version tag
  behind it. It was a second canonical encoding of "what the indexes attested",
  and the only one nothing shipped: every caller was a determinism test comparing
  it against itself. The encoding an answer is actually compared by lives in
  `purrdf-retrieval`, which collapses the ledger per stratum and digests the result
  into its `EvidenceId` -- and the ledger could not have been that encoding's
  source, because it is keyed by relation IRI rather than by stratum, holds sets
  where an answer's evidence holds one generation and one service level, and counts
  invocations, a quantity that follows the evaluator's chunking of driving rows
  rather than anything an index said. An evidence identity derived from it would
  have moved between two runs over one unchanged index. The ledger's own
  determinism is now pinned as what it is -- an ordered value whose declarations are
  identical across repeated runs and across the fork -- and the shipped digest is
  pinned through the shipped path, including that it does not move with the
  invocation count.

- **python:** Every Rust test module under `bindings/python/src` -- roughly
  fifteen hundred lines across ten files, holding fifty-six `#[test]` functions
  that no gate has ever built. The crate sets `test = false` for a sound and
  documented reason (see the new hygiene gate above), so those modules were
  compiled by nothing and run by nothing; one of them had silently accumulated a
  shadowing error that no gate could have reported. Every property each asserted
  is now asserted where it runs. Most were already covered by the pytest suite or
  by a live test in the crate that owns the logic; the rest are covered by new
  pytest tests -- the whole `purrdf.shex` surface (which had none at all), the
  native term model's own identity and RDF 1.2 refusals, blank-node scoping across
  and within a `Store.load`, the validation snapshot seam behind
  `Shapes.validate_store`, codec fidelity for private-use language tags and
  non-canonical lexical forms, the eight-format egress registry, RDFC-1.0
  determinism over isomorphic graphs, the per-call SPARQL engine configuration,
  `RdfDataset`'s layer classification, and seven further properties of the ranked
  retrieval surface.

- **retrieval:** `ProtocolError::RepeatedContribution` and
  `AdmissionError::DepthBeyondMonotoneRange`, the two refusals above, at the
  fusion boundary and at the admission waist respectively. The plan depth the
  second refused is now recorded as evidence on the compiled value instead.
- **retrieval, sparql-eval:** The rank-ordering declaration, in both places it
  stood: `StreamContract::ordering` on the consumer side and
  `RankedDeclaration::ordering` with its `RankOrdering` enum on the producer
  side. Its only reader in the workspace was the removed check, and a two-valued
  declaration nothing consults is a knob with no behaviour. Rank order is one
  law, not a choice: ranks are 1-based, contiguous and ascending for every ranked
  stream, and fusion checks every row against the next rank it expects from that
  stream -- a lower rank is `ProtocolError::OutOfOrderRanks`, a higher one
  `ProtocolError::NonContiguousRanks`. Producers no longer state the field;
  `TextSearchRelation::ranked_declaration` and
  `EmbeddingKnnRelation::ranked_declaration` build one field fewer.

  A registry's `content_fingerprint` covers a ranked declaration field by field,
  so dropping that field changes the fingerprint of every registry holding a
  ranked producer. A plan records the fingerprint it was admitted against, so
  plans pinned under an earlier build no longer match a registry built by this
  one and must be re-planned. Nothing else moved: a fusion profile's identity is
  a function of the fusion law alone and is byte-for-byte unchanged.
- **retrieval:** `ProtocolError::NonMonotoneContribution`, and with it the
  per-row ordering comparison that raised it. Contributions still must not rise
  with rank -- the threshold summed over the stream heads is an upper bound only
  while they do not -- but that has stopped being a fact a stream can get wrong
  on its own. Fusion re-derives every row's contribution from the decay rule,
  `K`, the stratum weight and the rank and refuses a disagreement as
  `ProtocolError::ContributionMismatch`, over ranks already held contiguous and
  ascending, and the profile's own curve never rises, because a profile refuses a
  weight at or below zero both when it is constructed and when it is decoded from
  canonical bytes. A rising value is therefore necessarily a value the profile
  did not compute, and it is refused as the wrong number it is rather than
  reported as a shape of the stream -- so nothing conforming or hostile reached
  the removed variant, in either direction. Non-increase is now stated where it
  is enforced, and proven where it is true: as a property of the decay rule's
  arithmetic over the weights a profile admits.
- **retrieval:** `FusionError::CeilingExceeded`, and the per-row comparison of a
  candidate's running score against `FusionProfile::ceiling()`. The ceiling is
  still the profile's admitted maximum and `ceiling()` still reports it; it is
  now enforced by construction rather than by testing each sum against it. A
  candidate receives at most one contribution per stratum, which fusion does
  check, and `K >= 1` with a 1-based rank caps every reciprocal at one half, so
  every contribution is at most half its stratum's weight and the largest sum a
  fusion can reach is exactly half the ceiling. The one configuration whose raw
  arithmetic lands on the ceiling exactly needs two contributions under a
  one-stratum profile, so the contribution count refuses it first --
  `CeilingExceeded` was never observed for it, or for anything else.
- **retrieval:** `FusionProfile::new`. It supplied a default decay rule while the
  same type documents that neither rule is a default, and the one it chose has a
  depth ceiling no weight can lift. Call sites name `with_decay` explicitly, which
  preserves every previously computed profile identity byte for byte.

  Every removal above, including the ordering declaration, touches no released
  API: `RankOrdering`, `RankedDeclaration::ordering` and the producer-side field
  they gave `purrdf-sparql-eval` were themselves added earlier in this same
  unreleased cycle and never reached a release. What the ordering declaration's
  removal does change is the registry content fingerprint, within this cycle: a
  plan pinned against an intermediate build of it no longer matches a registry
  built by this one and must be re-planned.

### Changed

- **release:** under this suite's full-semver rule, the breaking changes below
  make the next release a MAJOR version. Several of them change surfaces that no
  release has shipped yet: `purrdf-hnsw`, `purrdf_core::distance`,
  `RankedDeclaration` and `Scalar`. They are listed so that a consumer of an
  intermediate build can see every break. The exact-kNN fold order, the
  float-environment refusal and the MSRV change released behaviour.

- **BREAKING** **toolchain:** the MSRV is now 1.98, raised from 1.96.
  `Reassociated` uses `f64::algebraic_*`, which was stabilized as
  `float_algebraic` in Rust 1.98.0. A 1.97 compiler rejects the crate with
  E0658. Cargo, the CI msrv job, `clippy.toml`, the READMEs, AGENTS,
  CONTRIBUTING and the book all say 1.98.

- **BREAKING** **sparql-eval/hnsw:** the exact kNN scan and every HNSW distance
  site now fold in the `Exact` 16-lane tree order instead of sequentially.
  Distances can differ in the last bits, so the order of near-tied exact kNN
  answers can change. This supersedes the earlier entry in this section that
  said the accumulation order of `knn::Kernel::distance` was unchanged. Every
  dispatch path and target returns the same bits, and a scalar reference model
  of the fold order proves this in the tests. `knn::norm` delegates to the single
  normative PURREMB norm fold.

- **BREAKING** **sparql-eval:** the float-environment refusal. kNN checks the
  calling thread's floating-point environment when an embedding space is built
  from an artifact and on every search, exact or reassociated. It refuses an
  environment that flushes subnormals to zero, treats denormals as zero, or
  rounds other than to nearest, ties to even. The check is behavioural, so it
  runs on every target and refuses only an environment observed to depart; no
  target is refused for having a control register this build does not read. The
  refusal is the new `EvalError::FloatEnvironment`. `EvalError` is
  `#[non_exhaustive]`, so the variant itself breaks no match, but a query that
  used to run under such an environment now fails by name.

- **BREAKING** **core:** `FloatEnvironmentError::FlushToZero` and
  `FloatEnvironmentError::RoundingMode` carry one field, `evidence`, of the new
  `#[non_exhaustive]` `FloatEnvironmentEvidence`, in place of `register` and
  `bits`. A pattern that named `register: "MXCSR"` now names
  `evidence: FloatEnvironmentEvidence::Register { name: "MXCSR", .. }`. A
  refusal on a target whose register is not read carries
  `FloatEnvironmentEvidence::Probe`.

- **BREAKING** **hnsw:** the image and index version is now 2 (`INDEX_VERSION`,
  `profile::PAYLOAD_VERSION`). The header's reserved word becomes the arithmetic
  field, and zero names no arithmetic. A version-1 image, whose distances were
  folded sequentially, is refused with `HnswError::VersionMismatch`. Rebuild the
  index from its vectors. The implementation id is now `hnsw-v2`
  (`IMPLEMENTATION_ID`), `hnsw-reassociated-v2` is added beside it, and the
  profile declaration binds the arithmetic id.

- **BREAKING** **hnsw:** `HnswIndex<A: Arithmetic = Exact>`, and likewise
  `HnswSpace<A>` and `HnswRelation<A>`. `build`, `decode`, `load` and
  `from_artifact` are unchanged and exact. `HnswError` is not
  `#[non_exhaustive]`, and it gains three variants, so an exhaustive `match`
  stops compiling:
  - `ArithmeticMismatch`, when a payload records another arithmetic's code;
  - `ArithmeticPathUnavailable`, when a payload records a dispatch path this
    process does not run;
  - `FloatEnvironment(FloatEnvironmentError)`, raised by every build, rebuild
    and search entry point under a flush-to-zero or non-nearest environment.

- **BREAKING** **sparql-eval:** `Scalar` is sealed and implemented for `f32` and
  `f64` only. `RankedDeclaration` gains the public `arithmetic` field, so a
  struct literal must name it. `EmbeddingKnnRelation` gains a type parameter,
  `EmbeddingKnnRelation<A: Arithmetic = Exact>`, and `new` is unchanged and
  exact.

- **BREAKING** **core:** `Arithmetic` exposes `IMAGE_CODES` and
  `image_code(path)`, and `Path` gains the reassociated variants.

- **hnsw:** goldens that moved, re-pinned by hand. Distances now fold in the
  16-lane tree order, and the image header carries the arithmetic field.
  - `GOLDEN_DIGEST`: `0x0c71_b169_ebb4_4d7e` → `0xa367_d6c5_8963_1389`.
  - `GOLDEN_SERIAL_DIGEST`: `0x7e11_7799_b79a_b829` → `0xf0b2_fd33_0bcc_fcc7`.

  The `knn_wasm_determinism` expectation did not move, because its
  6-dimension fixture only reaches the sequential tail. New 70-dimension
  fixtures pin the lane path natively, on wasm32 and on wasm32+simd128.

- **build:** `make wasm-test` and `make hnsw-determinism` also run a +simd128
  build. The HNSW script reads its module path from cargo's artifact messages.
  The maintainer binaries `capture_sparql_goldens`, `gen_dict_vectors` and
  `gen_streamable_vectors` are renamed `capture-sparql-goldens`,
  `gen-dict-vectors` and `gen-streamable-vectors`. Their source paths are
  unchanged.

- **BREAKING** **retrieval:** The read bound moved into `RetrievalRequest`, so a
  bounded answer costs a bounded *read*. `RetrievalRequest` gains a `bound` field
  of the new `ReadBound` -- a total enum over the bounded case and the complete
  one, because "give me the top five" and "give me everything these strata hold"
  are both things a caller asks for and neither is the absence of the other. `plan`
  derives each stratum's depth from it, `Plan` records it and digests it into
  `Plan::id`, and `search` reads it from the request instead of taking a separate
  `top_k`. The bound used to arrive at `fuse`, four stages after every depth had
  been chosen: a fused top-five over two strata whose producers declare disjoint
  candidate blocks walked a handful of ranks and reported so, while the compiled
  unit was still `LIMIT <corpus>` -- so the elapsed time grew with the corpus while
  the instrument said it had not. Only the measurement had moved.

  When the bound narrows a depth is decided from the producers' own declarations
  and from nothing else, with no caller hint and no mode. Where every stratum
  declares a block set, no two of those sets meet, and every one of those strata
  declares `DuplicatePolicy::Unique`, each candidate has exactly one naming stratum
  and each of that stratum's ranks names a different candidate, so its fused score
  is one weighted contribution that falls with rank and the global top `k` is a
  merge of per-stratum prefixes: nothing below per-stratum rank `k` can enter it,
  even where the decay has saturated and the scores tie, because the tie-break's
  next key is the stratum rank those candidates win on. The depth is therefore
  `min(declared, statistics-narrowed, k)` and it is exact rather than merely
  smaller. Any overlap between two declarations, any `Unrestricted` stratum, and
  any stratum declaring `DuplicatePolicy::Allowed`, and the declared-or-measured
  bound stands exactly as before -- scores sum across strata in the first two cases
  and a discarded repeat makes a count of rows larger than the count of candidates
  in the third, so the merge argument has no premise to run on. Rows, scores and
  provenance are identical either way.

  A bound also gives a finite depth to a producer that declares unboundedly many
  rows, where previously only a measured cardinality could: such a read, taken for
  an answer that provably cannot use more than `k` rows, is a read of `k` rows.
  The same declaration asked for everything is not refused: it is recorded at the
  deepest depth a read can be taken to, and the read's own ending names that depth
  as the stopper. `PlanError::StatisticsUnavailable`, which used to refuse it, is
  removed in this same release.

  The probe row is untouched: the emitted bound is still the depth plus one
  wherever the declaration leaves room, and a depth *argument* is still never
  raised past the producer's own registration. It matters more at a tight depth
  than at a loose one.

- **BREAKING** **retrieval:** `fuse` refuses a bound the streams were not planned
  for, as `FusionError::ReadBoundMismatch`. `CompiledRetrieval::fused_bound`
  resolves a plan's `ReadBound` once into the row count a fusion of its units must
  run at, `execute` tags every `StratumStream` with it, and `RankedStream` gains a
  defaulted `fused_bound` so a stream assembled outside the ladder still fuses at
  whatever its caller names. It is a sibling of `PlanIdMismatch` rather than a case
  of it: that one is a disagreement among the streams about their provenance, this
  one is a disagreement between the streams and the caller's own argument, and the
  repairs differ. It closes a latent defect -- a plan could be fused at any bound,
  including one its depths could not honestly serve, with nothing catching it.

- **BREAKING** **retrieval:** `PLAN_VERSION` is 4. A plan now records **every
  input each stratum's depth was derived from** — the registry's declared row
  bound, the reported cardinality, the selectivity that was actually applied
  together with the request terms it aggregates over, and whether the request's
  row bound was licensed to bound that stratum — in a new `stratum_derivations`
  field. `Plan::certify` recomputes each depth from the inputs beside it through
  the same arithmetic the planner ran (`depth_from`, also public) and refuses a
  plan the two disagree about; `Plan::explain_depth` names which input bound a
  depth, which distinguishes the four roads to a depth of one that were
  previously indistinguishable. Both are cold paths: admission does not call
  them.

  Certifying is the whole coherence question rather than the arithmetic alone.
  Beyond `DepthNotDerivable` it refuses a depth with no derivation and a
  derivation with no depth, a stratum the snapshot names nowhere
  (`DerivationWithoutStatisticsEntry`), a snapshot row that says something else
  than the derivation beside it (`StatisticsEntryContradictsDerivation`, naming
  the dimension that moved), and a recorded selectivity domain that indexes a
  term the request it was planned for does not carry
  (`SelectivityTermOutOfRange`). Which stratum a refusal names is a function of
  the plan: every walk is over sorted keys, so two processes refusing one forged
  plan name the same one.

  Version 3 appended the request's read bound after the per-term unserved
  evidence because a plan that did not record it "recorded depths whose
  derivation could not be reconstructed". That bound is what the caller asked
  for, and whether it was *allowed* to bound a given stratum is decided
  separately, from the shape of the surviving declarations — so two plans could
  agree on every recorded field and still have derived their depths from
  different numbers. Version 4 closes that, along with the declared row bound,
  which no plan recorded at all.

  `StatisticsEntry.cardinality` is now `Option<u64>`, absent where the provider
  reported none, under the same presence discriminator its selectivity already
  used. A provider that reported only a selectivity narrowed a planned depth and
  was then dropped from the snapshot entirely, so the plan was built against
  evidence it did not record. `StatisticsEntry` also carries
  `selectivity_terms`, the request-term indices its aggregate came from: the
  aggregate is a sum, so a provider moving the same total onto a different term
  previously left a byte-identical plan describing a different measurement. That
  domain is a strictly ascending set of indices into the request a plan records,
  and a plan is held to it: a run out of order is `NonAscendingSelectivityTerms`
  and a repeated index is `DuplicateSelectivityTerm`, both at decode, where the
  fact is decidable from the run alone; an index addressing no term of the
  request is `SelectivityTermOutOfRange`, at `Plan::certify`, where the request
  is. An empty run is legal and is the common case — it is what a subject no
  selectivity was reported for records.

  The snapshot's entries name **every subject planning consulted** — each stratum
  a depth was derived for, and each predicate the request names. A stratum's row
  is a projection of its `DepthInputs` rather than a second consultation, which
  is what `StatisticsEntryContradictsDerivation` holds up; a request predicate's
  row derives nothing and is context alone. A subject nothing consulted is absent
  rather than recorded as empty, and both halves of that are checked: a stratum
  named nowhere is `DerivationWithoutStatisticsEntry`, and a row naming anything
  outside those two kinds is `UnconsultedStatisticsSubject`. Enforcing only the
  first would have left a plan able to *add* evidence — a row reads back as a
  consultation whether or not one happened.

  Their order is the value's law and not the encoder's.
  `StatisticsSnapshot.entries` is a `StatisticsEntries`, which establishes the
  ascending order on construction and refuses a repeated subject by name
  (`DuplicateStatisticsSubject`) on every path in, serde's included. A sort at
  the encoder would have made the bytes a pure function of the entries while
  leaving the value order-sensitive, so a descending snapshot and its ascending
  twin would have shared an identity and compared unequal.

  A canonical document must arrive in the order the encoder writes. Every keyed
  section — the depths, the derivations and the snapshot's entries — is required
  to ascend at decode, as `NonAscendingCanonicalKeys` carrying a typed
  `CanonicalSection`, and a repeat in each is refused by that section's own name:
  `DuplicateStratumDepth`, `DuplicateStratumDerivation`,
  `DuplicateStatisticsSubject`. A decoder that accepted any order would have
  admitted as many documents for one plan as its sections have permutations, each
  digesting to that plan's single id.

  Planning gains one refusal of its own: `PlanError::UndeclaredRowBound`, raised
  by `retrieval::plan` and by nothing else -- it reaches a caller of `plan`, never
  a caller of `compile` -- for a producer placed on a stratum whose declaration
  states no row bound at the mode it is invoked under. Placement and the row-bound
  read then disagree about one snapshot of the registry, so it ends the whole plan
  where a placement failure rejects only the producer it is a fact about:
  continuing would have derived every other stratum's depth from the same broken
  reading.

  A version-3 plan's bytes lie at different offsets under this layout, so the
  decoder refuses the old version by name instead of misreading it. Every plan
  identity moves, as does the plan document's JSON, which now carries
  `"stratum_derivations"` and `"selectivity_terms"`.

- **python:** A plan can leave the process and be checked on the way back in.
  `retrieval.plan` returns that plan's canonical, length-framed encoding under
  `"canonical_bytes"` — the bytes `"plan_id"` is the digest of — and
  `retrieval.certify_plan` reads one back, refusing it or returning it rendered
  exactly as `plan` renders it. `retrieval.explain_depth` answers which input
  bound one stratum's depth in a received document, from the same closed
  vocabulary `plan` renders under each derivation's `"cause"`. The plan dict also
  carries `"stratum_derivations"`, one entry per stratum with every input its
  depth was derived from.

  Every refusal from that boundary raises `retrieval.PlanDocumentError`, a
  `ValueError` subclass carrying a pinned kebab-case `.refusal` name — branch on
  that, never on the message. The names are the engine's own and the class
  documents all of them. A host that has not certified a document it received is
  reading a depth nothing checked.

- **BREAKING** **python:** `retrieval.plan` and `retrieval.compile` take the
  `top_k` keyword `retrieval.search` already took, and all three require it. The
  row bound is a planning input, so a `plan` or `compile` call without one would
  report a depth, a `LIMIT` and a `plan_id` for a read nobody asked for.

  The whole ranked-retrieval declaration surface postdates the last release, so
  every change above moves an API no published version carries.

- **retrieval:** A stratum depth is floored at one row, whether the zero came from
  a statistics provider or from the registry's own declaration. A bound narrows a
  read; it never eliminates one. A provider reporting a selectivity of zero parts
  per million, or a cardinality of zero, scaled a stratum's depth to zero: the
  compiled unit read nothing and the trailer still reported exhaustion with zero
  rows, which is the one ending that names no stopper, minted for a query that
  was never run. A tiny non-zero selectivity was already safe through
  ceiling division; zero was the one input that escaped it, and it arrives by three
  roads -- an honest `selectivity_ppm` of zero, a measured cardinality of zero,
  which lands in the bound before the ratio is applied, and a producer whose every
  declared access mode promises zero rows per invocation. Emptiness is now reported
  by the producer's own receipt against rows fusion verified rather than by a plan
  that declined to ask.
- **retrieval:** A producer whose every declared mode promises no rows is selected,
  bound and planned at that floored row, so it runs and reports its own
  `Exhausted { rows_emitted: 0 }`. That declaration describes the producer's data
  -- a text index built before its documents land, or one over a predicate no
  triple carries yet, declares exactly it -- and the producer is perfectly
  invocable. Dropping it instead bound it nowhere, and where it was the registry's
  only producer, which is the shape of a host with one index, the whole plan then
  failed with `PlanError::NoApplicableProducers`: "no registered producer accepts
  any term of the request", about a producer that accepts the term. It also threw
  away the only receipt that could have made the emptiness the producer's claim.
  "Declared no rows" and "declared nothing" still stay distinct -- a producer that
  declared no access mode admits no invocation, so placement refuses it and its
  stratum bounds no depth, and inventing a zero for it would refuse a plan the
  registry never spoke against.
- **retrieval:** Admission admits a depth of one against a declared row bound of
  zero, and refuses every depth past it. The one row is the probe that lets the
  producer speak; a stratum no ranked producer emits under is still refused at
  every depth, because there is no producer there to hand a row to.
- **retrieval:** A compiled unit's emitted bound is floored at one row, so a
  producer declaring zero rows is bounded at `LIMIT 1` rather than at `LIMIT 0`.
  For every declared bound of one or more this changes nothing: an admitted depth
  is at least one, so the probe is at least two and the minimum was already at
  least one.
- **retrieval:** A recorded stratum depth of zero is refused at the admission waist
  as `AdmissionError::ZeroDepth`. Given the floors above, a zero can now only come
  from an edited plan; admitted, it would emit `LIMIT 0`, which hands back no row
  whatever the relation holds, and report the stratum exhausted with nothing -- a
  completeness claim about the bound rather than about the data, indistinguishable
  afterwards from an honest empty answer. A stratum that is to read nothing carries
  no depth entry at all.
- **retrieval:** A compiled unit is emitted one row deeper than its stratum reads,
  bounded by whatever row count the producer declared, and the executor hands on
  only the rows the depth allows. The extra row is read and never reported: it
  decides whether the stream ended because the producer ran out or because the
  plan stopped it, and it appears in no plan field, no identity and no resolution
  number. Where the depth already equals the producer's declared bound there is
  nothing further to promise, so no probe is emitted and exhaustion is honest by
  contract. A stratum read to its planned depth over a larger answer now says so,
  where it previously claimed the strongest completeness the vocabulary has for a
  read the plan itself had cut. The executor also now runs each unit through the
  governed entry with every ceiling declined, so it can read the producer's
  attestation off the receipt that already carries the run's evidence; a witness
  that does not describe exactly one relation, one generation and at most one
  incompleteness means the snapshot moved underneath the query and is refused as
  `ExecutionError::InconsistentWitness`.
- **retrieval:** `ProtocolError::DuplicateItem` now names the stratum whose stream
  repeated the item, beside the item itself. A repeated item names *what* went
  wrong and the stratum names *who*, and a consumer fusing several producers can
  act on the pair and on neither half alone: the item does not say which of five
  strata to go and fix, and the stratum does not say which of its rows to look at.
- **sparql-eval:** `RankedDeclaration` gains a `domains` field
  carrying the producer's `CandidateDomains`, declared beside its duplicate policy
  because the two are the same kind of fact -- a promise the producer makes about
  its own rows that a consumer holds it to. It reaches the registry's content
  fingerprint through `RankedDeclaration::canonical_description`, and so reaches a
  plan's identity and is verified at admission rather than taken on faith: two
  registries that differ only in what their producers may name fuse differently
  and must not share a digest. A host that does not restrict its candidates writes
  `CandidateDomains::Unrestricted`. The whole ranked-retrieval declaration surface
  postdates the last release, so this widens a type no published version carries.
- **BREAKING** **sparql-eval:** `RelationIdentity` gains a `witness` field. The
  fingerprint covers everything a registry *declares*, which is exactly what the
  planner reads -- and a relation's index can be rebuilt underneath it without any
  declaration changing, so two governed runs of one query over one dataset snapshot
  could carry byte-identical identities and different rows with nothing on the
  receipt to say why. The identity says which relations could have been asked; the
  witness says what the ones that were asked reported about themselves. All three
  parts are independently empty, and an empty witness is never a claim that an
  index was whole.
- **sparql-eval:** A query whose relation declares its index incomplete is refused
  at an entry point that cannot record the declaration, rather than returning the
  short bag. The governed lane, which has a receipt to carry the witness, answers
  and records it; the ungoverned lane and `UPDATE` refuse, and the update writes
  nothing. A relation that declares nothing short is unaffected on every lane.
- **text, sparql-eval:** `TextSearchRelation::ranked_declaration` and
  `EmbeddingKnnRelation::ranked_declaration` take the producer's `CandidateDomains`
  as a further argument, passed through into the declaration unchanged. Neither
  producer supplies one of its own, and neither could: the tags describe how a
  host's corpora partition, which is knowledge only the host has. A host that does
  not restrict its candidates passes `CandidateDomains::Unrestricted`, which is
  the behaviour both had before.
- **retrieval:** `DecayRule::class_width` and `FusionProfile::class_width`
  answer with the new `ClassWidth` rather than a bare `u64`, and the Python
  `retrieval.class_width` answers `int | None` rather than `int`. The search for
  a class's far end stops at the deepest rank a plan can express, so a class
  still running there has no counted end -- and it now says that, as
  `ClassWidth::ExceedsAnyPlan` and as `None`, rather than handing back
  `2**32 - 1` as though it were a width somebody measured. Raw weights of one
  and fifty are fifty times apart and both saturate under either rule, because
  every contribution has truncated to the same value; the bare number reported
  them as the identical width `4294967295`, a number a caller can log, plot or
  divide by, quoted precisely where nothing was counted. It also contradicted
  the function's own stated meaning, which is that a width of `w` is `w`
  consecutive ranks the fused score treats as equal. `ClassWidth` is a third
  type rather than a reuse of `MonotoneDepth` or `ToleratedDepth` because a
  width is a count of ranks and neither of those cases is: `SeparatesTo` asserts
  that every adjacent pair up to a depth is distinct and `ReadsTo` asserts that
  a read stopping at a depth stays inside a tolerance, and a width establishes
  neither. Its saturating case is also the opposite polarity -- a class with no
  end is inside no tolerance, where a depth that never collides is inside every
  one -- which `ClassWidth::fits_within` spells out beside `covers`. Confined to
  `purrdf-retrieval` and its binding, which have not yet been published, so no
  released API changes.
- **retrieval:** The documented relationship between a tolerance and the class
  at the depth it buys no longer claims an equality that does not hold. Four
  places -- `DecayRule::deepest_rank_within_width`,
  `FusionProfile::deepest_rank_within_width`, the Python docstring and
  `__init__.pyi` -- said the width there is "at least `max_width + 1`, two at a
  tolerance of one". The bound is right; the parenthetical is not. Under the
  truncated rule with `k` of 60 at a raw weight of `10^2` a tolerance of one
  lands on depth one, whose class is **forty** ranks wide, and at `10^2 + 21` it
  is sixty. Equality holds only where the run that ends the walk is one rank
  longer than the tolerance, which is the smooth case and is exactly the regime
  the two accompanying tests pinned -- a raw weight of `10^6` in Rust and
  `1000 * SCALE` in Python -- so neither could fail on it. The four sites now
  state only the bound, and a light-weight case is executed at both Rust
  altitudes and on the Python surface so that the corrected claim is held by a
  test that can fail.
- **retrieval:** `DecayRule::deepest_rank_within_width` and
  `FusionProfile::deepest_rank_within_width` answer with the new
  `ToleratedDepth` rather than a bare `u64`, and the Python
  `retrieval.deepest_rank_within_width` answers `int | None` rather than `int`.
  A plan records a per-stratum depth as a 32-bit rank, so a walk that runs to
  that ceiling has found no bound inside any plan's reach -- and it now says
  that, as `ToleratedDepth::ReadsBeyondAnyPlan` and as `None`, rather than
  handing back `2**32 - 1` as though it were a depth somebody measured. Two
  weights fifty times apart both reach the ceiling, and the bare number said
  they read to the same depth: a number a caller can log, plot or divide by,
  quoted precisely where no bound exists. `MonotoneDepth` had already drawn that
  line for the separating depth; the tolerated depth is a separate type because
  its cases claim something different -- above a tolerance of one the ranks it
  reports do share contributions, just never more than the tolerance of them
  within the read, so carrying it in `MonotoneDepth::SeparatesTo` would attach a
  separation claim to a depth measured under no such claim. The Python rendering
  matches what a `search` answer already does with `"separates_to"`, which is
  `None` for the same wall. Confined to `purrdf-retrieval` and its binding,
  which have not yet been published, so no released API changes.
- **retrieval:** A `max_width` of zero is refused as the new
  `FusionError::InvalidWidth` -- "a tolerance of zero is not a tolerance,
  because a class always contains its own rank" -- from `DecayRule`,
  `FusionProfile` and `retrieval.deepest_rank_within_width` alike, instead of
  being silently read as one. It is the only zero operand on this surface that
  did not refuse: a rank of zero and a smoothing constant of zero already did,
  and a tolerance of zero is the same shape as a constant of zero, a question
  with no evaluable content. Reading it as one answered the narrowest real
  tolerance in its place, which is the deepest fully-separated depth this
  algebra can report -- the most favourable answer there is, returned precisely
  where nothing was asked. It is a separate variant from
  `FusionError::InvalidRank` because a tolerance is a count of ranks measured
  across the 1-based axis rather than a position on it, and "rank must be at
  least 1" would send a caller to inspect an argument that was never at fault.
  The normalisation had also been documented only on the private implementation:
  neither public entry point's `# Errors` section, nor the `.pyi` stub,
  mentioned it. Confined to `purrdf-retrieval` and its binding, which have not
  yet been published, so no released API changes.
- **retrieval:** The documented meaning of a `max_width` of one now says what
  the code does. It had claimed to report "the deepest rank still separated from
  both its neighbours", and the branch's own tests assert the opposite: the
  answer is the deepest depth a *read* can stop at with every rank it actually
  read separated, and `class_width` at that rank is never one. `class_width`
  measures the unbounded curve, which also looks at the one rank the bounded
  read never reaches, so it reports at least `max_width + 1` there -- exactly
  `max_width + 1` where the run that ends the walk is one rank longer than the
  tolerance, and wider where that run is longer still, which is measured under
  the truncated rule at a raw weight of `10^6` with a tolerance of fifty. The
  two functions agree in all of those cases; they are answering a depth question
  and a rank question. The corrected relationship is carried into
  `DecayRule::deepest_rank_within_width`,
  `FusionProfile::deepest_rank_within_width`, the Python docstring and
  `__init__.pyi`, and is asserted rather than described.
- **retrieval:** Every Python entry point that takes a smoothing constant now
  takes the decay rule that constant belongs to, and none of them defaults it.
  `retrieval.search`, `retrieval.weight_for_depth` and `retrieval.class_width`
  take a required `decay` keyword, and `retrieval.compile` takes it as the third
  part of the fusion law beside `weights` and `k`. It is spelled the way every
  other closed set on that surface is -- `"reciprocal_rank"` or
  `"weighted_reciprocal_rank"`, alongside a request term's `metric` and a
  producer's `graph` -- and an unknown spelling raises `ValueError` naming both.

  Before this, the Python surface hardwired the truncated rule everywhere it
  named one, so a Python caller could not build, search under, or ask any
  question about a folded-rule law. That mattered most where
  `weight_for_depth` refused: its message says the remedy for a depth past the
  truncated rule's wall is to name the folded rule, and Python had no way to
  name it, so the surface handed out a diagnosis with no cure. The folded rule
  now reaches every number in an answer -- contributions, both resolution maps
  and the `"profile_id"` the law names itself by -- rather than only the
  profile's constructor.

  A `compile` call naming some of `weights`, `k` and `decay` but not the rest is
  a `ValueError` that says which part arrived and which did not; naming none of
  them still compiles without a law and reports no resolution. Confined to
  `purrdf-retrieval` and its binding, which have not yet been published, so no
  released API changes.
- **retrieval:** The Python `search` answer spells the trailer's measured rank
  resolution `"observed_resolution"` rather than `"resolution"`. The answer now
  reports two resolutions -- what the plan was going to cost and what the rows
  actually cost -- and an unqualified name beside a qualified one reads as the
  general case of it, which these are not: a top-k that certifies early never
  reaches its planned depth, so the two differ by design. Confined to
  `purrdf-retrieval` and its binding, which have not yet been published, so no
  released API changes.
- **retrieval:** `FusionProfile::class_width` and
  `FusionProfile::deepest_rank_within_width` answer with
  `Result<Option<u64>, FusionError>` rather than `Option<u64>`, so the two facts
  they carry stay two facts. `Ok(None)` is a stratum the profile declares no
  weight for -- an absence, not a failure -- while an operand the decay rule
  cannot evaluate is now the refusal it is. A rank of zero is the reachable one:
  ranks are 1-based, so there is no rank zero for a class to form around, and
  such a call previously came back as a width of **one** -- the claim that the
  rank is perfectly separated from both its neighbours, which is the most
  favourable thing the resolution algebra can say, returned precisely where it
  had measured nothing. The same call from Python, `retrieval.class_width`,
  raises `ValueError` there instead of returning a number. Confined to
  `purrdf-retrieval`, which has not yet been published, so no released API
  changes.
- **sparql-eval:** `knn::Kernel::distance` and `knn::norm` are generic over
  `knn::Scalar`. This is **source-breaking for inference-dependent callers**: an
  expression whose operand type the compiler previously inferred may now need an
  annotation (`norm::<f64>(&[])` where `norm(&[])` sufficed). Every operand type
  that worked before still works, and no numeric result changes -- widening is
  exact and the accumulation order is unchanged. Classified as additive plus
  source-breaking rather than behaviour-breaking; it warrants a MINOR bump under
  this suite's rule, not a MAJOR one, because no existing call can compile to a
  different answer.

### Features

- **shapes:** Prepared SHACL products. `PreparedShapes::to_product` writes a
  parsed shapes graph out as a deterministic, versioned, authenticated byte
  artifact carrying the declarative model, the shapes dataset it was derived
  from, and the binding of every input it was compiled against; a later process
  restores a prepared validator instead of re-parsing Turtle. The bytes come
  back untrusted, so decoding is admission rather than parsing: the reader has
  two entry points at one boundary — `admit`, the common path, which re-derives
  nothing expensive, and `rebuild`, which ignores the model memo and re-derives
  the shapes from the carried dataset when the preparation stage is one this
  build does not know. `rebuild` parses no RDF text and reads no file. The
  writer has one path and always emits every section. The preparation stage
  identity is a content-derived digest over the declarative model and the tables
  its meaning depends on, never a hand-incremented counter. The shipped profile
  is `purrdf-shacl-core-v1`, named explicitly with no default and no fallback.
  Targets are deliberately not carried: they are data-dependent and resolved per
  bound dataset.
- **shapes:** `ShapesProduct::certify` independently corroborates a product's
  shapes dataset against the canonical identity its binding claims. It is a cold
  path and is unreachable from either restore seam by construction, because
  canonicalizing a shapes graph's blank nodes can cost more than the shapes
  parse a prepared product exists to eliminate.
- **shapes:** `ProductDimension`, a closed set of twenty admission refusal
  dimensions ordered from the outside of the container inward, each with a
  stable kebab-case label and a prescriptive message naming the fix. A caller
  branches on the dimension; the label set is a pinned contract recorded by a
  frozen corpus, and matching on message text is not supported.
- **core:** A generic authenticated artifact envelope
  (`purrdf_core::artifact`): one fixed-layout, self-verifying container that a
  prepared-product codec instantiates with a magic, a format version and a
  section count. Fixed header, section directory in canonical kind order with
  per-section SHA-256, zero-verified alignment padding, a decodable identity
  region under its own digest, and a sealed trailer restating the total length.
  The reader fails closed at the first inconsistency. The section directory is
  total: every declared kind is present in every artifact, possibly
  zero-length.
- **shapes:** `Shapes` retains its parse provenance — the base, the document
  prefix map, the box-role vocabulary and the `sh:shapesGraph` IRI the parse
  actually used — so a shapes graph in hand can answer what it was parsed from
  rather than relying on a caller to remember.
- **sparql-eval:** Content-only fingerprints for the user-function, custom
  aggregate and property-function registries, with the user-function table
  reported per population so a product can state the declarations it rebuilds
  separately from the bindings only a host can wire.
- **validate:** A shared prepared-product boundary (`pack_shapes_product`,
  `admit_shapes_product`, `rebuild_shapes_product`, `certify_shapes_product`,
  `explain_shapes_product`, `validate_with_shapes_product`) that the CLI,
  Python, WebAssembly and C surfaces all route through, so those sequences exist
  once. `ShapesProductRefusal` keeps a refusal's dimension where there is one
  and reports `None` for a shapes document that never reached the admission
  boundary.
- **cli:** `purrdf shacl pack`, `purrdf shacl verify` and `purrdf shacl explain`,
  plus `purrdf validate --shapes-product`, which is mutually exclusive with
  `--shapes`. A refusal writes `shacl dimension <label>` to stderr and exits 1.
  The shapes-parse flags `--shapes-from`, `--shapes-graph` and `--import` are
  refused by name against a product rather than accepted and ignored.
- **python:** `PreparedShapes.to_product()` and a `ShapesProduct` class with
  `open`, `explain`, `format_version`, `stage_id`, `stage_known`,
  `identity_digest`, `identity_components`, `admit`, `rebuild` and `certify`.
  Refusals raise `ShapesProductError`, whose `.dimension` carries the label.
- **wasm:** `shaclPackProduct`, `shaclProductExplain`, `shaclProductCertify` and
  `shaclProductValidateToSarif`, rejecting with a refusal that carries its
  dimension.
- **capi:** `purrdf_shapes_product_encode`, `purrdf_shapes_product_open`,
  `purrdf_shapes_product_admit`, `purrdf_shapes_product_certify` and
  `purrdf_shapes_product_error_dimension`.
- **shapes/validate/cli/python/wasm/capi:** A restore can now be BOUND to the
  product the consumer meant. Every other admission check asks about the
  executing process — its build, its registries, its class analysis — and none
  of them asks whether the bytes in hand are the ones the caller wanted, because
  nothing in a product states which product was meant; an unbound restore of the
  wrong artifact therefore succeeded and returned a well-formed report about a
  shapes graph nobody asked about. `ShapesProductView::admit_expecting` takes the
  32-byte digest of the input binding the caller requires and refuses on
  `shapes-graph` when the product carries another, ahead of every other check and
  before anything is decoded; `admit` is the unbound case of the same body.
  `purrdf-validate` exposes `admit_shapes_product_expecting`,
  `validate_with_shapes_product_expecting` and `parse_identity_digest`, and every
  surface routes through it: `purrdf validate --shapes-product FILE
  --expect-identity HEX` on the command line, `ShapesProduct.admit_expecting` in
  Python, `shaclProductValidateToSarifExpecting` in JavaScript, and
  `purrdf_shapes_product_admit_expecting` in C. The selector is the 64
  hexadecimal digits `shacl explain` prints on its `identity-digest` line and
  `shacl verify` prints on stdout, accepted back unchanged; a satisfied
  expectation produces the byte-identical report the unbound call produces.

### Performance

Each site below is stated with its measured vector-instruction evidence from
`make simd-asm`. Every rewrite is proven equal to its previous implementation,
which is kept as a test oracle, and no emitted byte changes.

- **core/sparql-eval/hnsw:** the `Exact` fold. Its 16 lanes pack into
  `addpd`/`mulpd` on x86_64, `vaddpd`/`vmulpd` on the AVX2 path, `fadd .2d`/`fmul .2d`
  on NEON and `f64x2.add`/`f64x2.mul` on wasm simd128. It uses no FMA and no
  `relaxed_` op, and the same order runs scalar on the baseline. The `Reassociated`
  fold uses `vfmadd231pd` on avx2+fma (`%ymm`) and avx512f (`%zmm`), and `fmla .2d`
  on NEON. On wasm simd128 it uses `f64x2` lanes with no FMA, because relaxed SIMD
  is never enabled. The HNSW search and graph build call these kernels through
  function pointers set by non-generic constructors, so both arithmetics are
  compiled inside `purrdf-hnsw` and measured there.

- **iri/sparql-algebra/xsd:** the four byte-class scanners use packed byte
  compares and a mask: `pcmpeqb`+`pmovmskb` on x86_64, `vpcmpeqb`+`vpmovmskb` on
  x86-64-v3, mask-register compares (`vpcmpeqb %k`, `vpcmpltub %k`) on x86-64-v4,
  `cmeq`/`cmhi .16b` on NEON and `i8x16.eq`/`i8x16.lt_u` on simd128. On the baseline wasm build they run as straight-line scalar lanes. The
  SPARQL lexer's trivia skip and IRI body now make one pass over them, and
  `cur`/`peek` gain an ASCII fast path. IRI component validation gains an 8-byte
  clean-run precheck, packed on every SIMD configuration, alongside the
  `i32x4.lt_u` range test of the `ucschar` arm on simd128. XSD whitespace replace
  and collapse gain chunked prechecks, which are `i8x16` compares with
  `v128.any_true` on simd128.

- **core/rdf/entail/retrieval/sparql-results/json/markdown:** escape and string
  scans. These are the canonical IRI and literal escapes, the native serializer's
  IRI escape, `xml_escape::push_into` (which now validates and finds replacements
  in one pass, and still writes nothing for a refused value), CSV quoting, the
  SPARQL-JSON string writer, the `purrdf-json` string parser and the markdown
  line splitter. Each scan is packed byte compares (`pcmpeqb`+`pmovmskb` on
  x86_64, `cmeq .16b` on NEON, `i8x16.eq` on simd128). The drivers around them
  stay scalar, and non-ASCII runs are copied in bulk. Markdown gains no
  dependency.

- **core:** pack, dataset, hex and embedding kernels.
  - The pack dictionary's `common_prefix_len` compares 8-byte words by XOR and
    finds the first differing byte with a trailing-zero count (`rep bsfq` or
    `tzcntq` on x86_64, `rbit`+`clz` on aarch64, `i64.ctz` on wasm).
  - `select_in_word` is a loop-free broadword select. It computes SWAR byte
    popcounts and a multiply prefix sum, then compares and counts to find the
    byte, then reads a const in-byte select table (`imulq`/`popcntq`,
    `madd`/`cnt`, `i64.mul`/`i64.popcnt`).
  - The dataset's sequential pattern scans filter eight rows at a time with a
    branchless key-and-mask compare over four `u32` columns, and emit matches in
    row order (`pcmpeqd`+`movmskps` on x86_64, `vptestnmd` on x86-64-v4,
    `cmeq`/`cmtst` on NEON, `i32x4.eq` with an `i16x8.bitmask` row mask on
    simd128). The
    owned `QuadPatternCursor` shares that one match law.
  - `hex::lower` writes compare-selected digits into a buffer sized up front
    (`pcmpgtb`+`paddb` on x86_64, `cmhi`+`add` on NEON, `i8x16.gt_u`,
    `i8x16.add` and `i8x16.shuffle` on simd128). Content-id hex decoding
    classifies 16 digits at a time with a vertical rejection accumulator
    (`pcmpeqb`/`pminub`/`pmovmskb` on x86_64, `cmhi`/`cmhs` on NEON,
    `i8x16.lt_u`/`i8x16.gt_u` and `i8x16.shuffle` on simd128), and
    accepts and refuses exactly the inputs it did before.
  - PURREMB f32 rows get a bulk finite check and decode (`pcmpeqd`+`pmovmskb`
    on x86_64, `cmeq` on NEON, `i32x4.eq` with `v128.any_true` on simd128).
  - The sorted triples intersect is left as it is, with the fixture's measured
    list lengths recorded.

- **datalog:** a relation's mutable tail is held as structure-of-arrays columns
  of subjects, objects and row ids. The duplicate probe on insert OR-folds
  `(s == subject) & (o == object)` over the two key columns, with no exit on a
  hit. Over the old `(subject, object, row)` tuples the same fold compiled to a
  scalar `xor`/`sete`/`orb` loop, because consecutive keys sat three words apart.
  Over the columns it packs on every SIMD configuration measured:
  `pcmpeqd`+`movmskps` on x86_64, `vpcmpeqd %k` on x86-64-v4, `cmeq .4s`+`addp`
  on NEON, SVE `cmpeq` on neoverse-v1, and `i32x4.eq` answered by
  `v128.any_true` on simd128. The baseline wasm build runs a scalar
  loop with no exit. The cursors read the tail in insertion order, and sealing
  drains it in that order, so the batches built from it are unchanged.

- **shapes:** the SHACL path frontier's linear dedup stage (fewer than 16
  admitted ids) tests every id with no exit on a hit, four `u32` lanes at a time.
  The lanes are OR-ed into an accumulator that is folded once. This is
  `pcmpeqd`+`por`, folded by `movmskps`, on x86_64, `vpcmpeqd %k` on x86-64-v4,
  `cmeq .4s`+`addv` on NEON and `i32x4.eq` with `v128.any_true` on simd128. The baseline wasm build uses straight-line scalar
  compares.

- **columnar:** the PLAIN `INT64` codec. BYTE_ARRAY values keep their loop.
  - Encode and decode are out-of-line functions. A count of the present rows
    sizes the output exactly on encode. On decode it fixes the body's length
    before any word is read, and the `Truncated` and trailing-bytes `Malformed`
    answers stay those of the per-value loop.
  - A null-mask pass then runs eight rows at a time. Runs of null-free chunks go
    through one counted copy, and chunks holding a null go value by value.
  - The encode copy reads each payload only under its discriminant, so it
    vectorizes only where masked loads exist: x86-64-v4 (`vptestmd` masks,
    masked `vpgatherqq`) and neoverse-v1 SVE (`ld2d`, with a `cmpne`-predicated
    payload load).
  - The decode copy vectorizes with 256-bit or wider vectors (`vpermq` with
    `vpunpckhqdq` on x86-64-v3, `vpermt2q` on x86-64-v4, SVE `st2d`).
  - On SSE2, NEON and wasm simd128, both copies stay scalar, and the only vector
    op is the presence count (`i32x4.add` on simd128).

- **sparql-eval/text:** Attesting an index generation is a refcount bump instead of
  a string copy, and an entry point that cannot carry the answer no longer asks the
  question. `IndexGeneration::Declared` holds a shared `Arc<str>`, so a producer over
  a frozen index -- both text relations, the kNN cursor -- renders its generation once
  at construction and every invocation clones a pointer, where each one previously
  copied 64 hex characters into a fresh `String` in a seam entered once per driving
  row. On a lane whose return type has no witness slot, the generation is not read
  and the ledger entry is not built at all: the value could only have been dropped
  with the context unread. The service-level read is deliberately not conditional,
  because the refusal an unwitnessed lane owes a relation that declares its index
  short depends on it. A relation over a frozen index therefore attests at no
  per-invocation allocation, and one that genuinely computes a generation per
  invocation pays for it only where somebody can read it --
  `benches/relation_attestation_alloc.rs` reports the counts per invocation for both
  shapes on both lanes (report-only, no threshold).
  `IndexGeneration::declared` builds the variant from a `&str`, a `String` or an
  `Arc<str>`.

- **core/iri:** Restoring a dataset pack allocates 21 times for the prepared
  product fixture's 3.8 KB section, down from 523, and requests 43,411 bytes
  down from 104,527. The largest single cause was a double parse across a crate
  boundary: every dictionary IRI was parsed once to check absoluteness during
  decode and again at the builder's store-once boundary. The IRI parser is now
  an allocation-free scan plus owned materialization, which benefits every
  intern miss in the workspace. Dictionary records decode into a borrowed view,
  front-coding buffers are hoisted and swapped, reserves are exact and clamped
  by buffer length so a hostile header cannot over-reserve, and replay is sized
  from the source's own counts. No emitted bytes change; every frozen pack
  vector and the frozen product golden are unaffected.

### Documentation

- **design:** `docs/design/purrdf-simd.md` records the stable-channel
  ceiling, the three float regimes, and the contract law for separate fast
  functions. It inventories every hot site with its class, blocker, verdict and
  reason, including the sites that are left alone. Each site has measured
  vector-op counts on all seven configurations. The document also maps each
  bench to its sites and gives a crate coverage table. `make simd-asm` runs
  with `--doc`, which holds it to the measurements. Measurement corrected one planned verdict:
  `json_escape_body` was expected to be covered, but it had no vector scan, so it
  became a rewrite. A comment in the wasm package that claimed a blake3 simd128
  backend is corrected, because that backend is not enabled.

- **sparql-eval/book:** `RelationAttestations::invocations` says, where a host reads
  it, that it is a fact about the schedule rather than about the index: under a
  `FILTER EXISTS` the parallel row loop evaluates each chunk of driving rows on a
  worker whose `EXISTS` memo starts cold, so the relation inside it is re-entered
  once per chunk and the count follows the input size and the worker count. The
  declaration sets beside it do not move with any of that, which is why the
  per-stratum conformance rule and an answer's evidence identity are keyed on them
  alone. The two readings the count does support -- ran at all, and a rough
  magnitude -- are named, and comparing it between two receipts is named as the one
  thing it cannot be used for. The querying chapter carries the same correction,
  which previously promised that two runs over one snapshot compare byte for byte
  across the whole mapping.

- **design:** `docs/design/purrdf-prepared-products.md` records the decisions
  behind the prepared-product surface — admission rather than parsing, why there
  are three seams instead of one flag, the derived stage identity, the census
  closure, the pinned refusal label set, the bounded guarantee that excludes
  targets, the standing tension of three authenticated containers in one
  workspace, and the measured allocation profile.
- **book:** A `Prepared Shapes Products` chapter under Validation covering
  producing, shipping, restoring, corroborating and explaining a product, what
  each refusal dimension means and how to answer it, and the command-line verbs.

### Tests

- **shapes:** A product-model census reads the declarative model out of the
  crate sources and checks it against the transitive closure reachable from
  `Shapes`, so the covered set cannot go stale in either direction; the closure
  found the `sh:SPARQLTargetType` declaration types and the SHACL-AF
  `Rule`/`RuleBody`/`RuleSchedule` family that a hand-written enumeration had
  missed. The preparation stage identity is derived from that census. Self-tests
  prove the scan is not asleep and that a reworded doc comment does not move the
  identity.
- **shapes:** The refusal matrix executes every admission dimension alongside a
  neighbouring valid case, because over-refusal is the mirror of a silent drop
  and hides behind tests that all pass. Determinism, RDF 1.2 term identity
  across a round trip, and the whole product lifecycle on
  `wasm32-unknown-unknown` against a golden written by a native build are pinned
  separately. `open` is proved not to certify: a product whose stored canonical
  digest is tampered with must admit and must fail certification.
- **retrieval, sparql-eval, text:** Five assertions offered as coverage that no
  implementation could fail now assert the property their message names, and each
  was confirmed by mutating the implementation until it went red. A compiled
  unit's emitted `LIMIT` is pinned at the depth plus the one probe row instead of
  accepting either value, so the row bound collapsing back onto the depth is a
  failure rather than a tolerated spelling. A domain tag is checked against the
  three respellings generic URI normalisation would change -- an uppercased
  scheme and authority, a dot segment, a case-flipped percent-escape -- so
  neither constructing a tag nor reading one can canonicalise a host's own
  spelling, where the previous check was reflexivity of a derived `PartialEq`.
  The relation witness's invocation count is asserted exactly, and against the
  forced-sequential lane's count over the same query: a `FILTER EXISTS` re-enters
  evaluation once per driving row, so the fork-join total has to add back up to
  the same number, and a join that kept one worker's ledger and dropped the rest
  now fails instead of passing a `>= 1` floor. An empty text index's attested
  generation is checked for being a digest that was computed -- not the all-zeros
  placeholder -- and for distinguishing that index from an equally empty one
  under a different configuration, which is the pair emptiness leaves
  indistinguishable to a digest over content alone. And the refusal that replaced
  a bare `AttributeError` about a private protocol method is checked for naming
  the type that actually arrived, over two different arriving types, so a
  constant spelled into the message cannot satisfy it.

### Added

- **sparql-eval:** `NativeSparqlEngine::prepare_execution` returns a `PreparedExecution`: a
  query parsed and admitted once, then bound and run many times without re-parsing or
  re-admitting. `PreparedExecution::bind` sets a parameter by its declared slot index and
  `bind_named` by its declared name; `NativeSparqlEngine::execute` runs the current bindings
  against a dataset. A parameter reaches inside `OPTIONAL`, `MINUS`, `EXISTS` and sub-`SELECT`s
  by ordinary correlation, the same as a `query` substitution. Running with a declared parameter
  left unbound is refused, because treating it as unrestricted would silently widen the answer
  to every subject; binding an undeclared name is refused; declaring one parameter twice is
  refused. `bind`, `bind_named` and `execute` take `&mut PreparedExecution`, so the type system
  rejects a program that would hand a re-entrant query body (a SHACL-AF `sh:expression` function
  call, for example) the very tree an outer call is mid-evaluation over.

- **python:** `Store.prepare(query, *, parameters=[...], ...)` returns a `PreparedQuery` whose
  `run(**bindings)` binds every declared parameter by keyword and returns results exactly as
  `Store.query` does. `prepare` accepts the same engine-configuration keywords `query` does —
  `extension_namespaces`, `property_fn_namespaces`, `standpoint_predicates`, `relations`,
  `relations_from_graph`, `path_relations` and `aggregate_namespace` — except `substitutions`,
  whose role a prepared query's per-run parameter bindings now fill; the returned `PreparedQuery`
  carries those registries, so every `run` evaluates under the same ones the plan was admitted
  against. Each `run` re-reads the owning store's current contents rather than a snapshot taken
  at prepare time, because what is prepared is the plan and not the data — the motivating caller
  is a row-driven loop such as SHACL validation or a rule fixpoint, where the store changes
  between runs. A `run` with a declared parameter left unbound is refused, binding an undeclared
  name is refused, and declaring one parameter twice at `prepare` time is refused.

### Changed

- **BREAKING** **sparql-eval:** `PreparedQuery`'s parsed algebra, formerly the public mutable
  field `query`, is now private. Read it through the new `PreparedQuery::query(&self) -> &Query`
  accessor. Admission — the algebra soundness walk and the graph-depth guard — is plan-invariant
  and now runs once when a query is prepared rather than being re-checked on every run, so there
  is no longer a way to swap an admitted plan's algebra after the fact; supply the algebra you
  want at construction, where it is admitted.

### Fixed

- **sparql-eval:** A prepared execution's `execute` skipped the check that a run's registries
  (relations, aggregates) agree with the ones the plan was admitted under. Preparing under one
  registry configuration and running under another used to evaluate silently — a registered
  relation prepared with no registry in scope lowers to an ordinary triple pattern and answers
  the empty bag with no error, and two registries can resolve one IRI to two different
  accumulators of identical declared arity — and it is now refused.

- **python:** `Store.prepare` ignored every one of the engine-configuration keywords `Store.query`
  accepts, so a prepared query naming a registered relation had that relation silently lowered to
  an ordinary triple pattern with no way for a caller to prevent it. `prepare` now takes and
  honours the same configuration `query` does, as described above.

- **python:** `PreparedQuery.run(**bindings)` used to leave every parameter not named in a given
  call holding the value from the previous call, so a later call that omitted a parameter
  silently reused it instead of being refused. Every slot is now returned to unbound before a
  call's keyword bindings are applied, so a run is total in its own arguments and a parameter
  omitted this call is refused regardless of what an earlier call supplied.

### Performance

Allocation count per focus node, from the deterministic counting allocator rather than timings,
pinned as the exact slope of a closed form measured at two focus-node populations, across four
SHACL surfaces: an `sh:sparql` constraint, a custom `sh:ask` component, a custom `sh:select`
component, and an `sh:expression` function call.

- The SHACL change path now runs each of those surfaces on a prepared execution instead of
  re-parsing and re-admitting a query per focus node, and the substituted algebra a parameter
  binding writes into is memoized and rewritten in place rather than rebuilt from scratch on
  every run. Combined with retaining the evaluator's scratch interner across runs and deciding
  once, rather than separately at each call site, what a pre-bound value may be used for, the
  per-focus-node allocation cost fell from 96 / 214 / 116 / 194 to 51 / 114 / 64 / 127 across the
  four surfaces respectively.

- CONSTRUCT template instantiation and SPARQL Update's insert/delete/graph-slot templates
  resolved every variable position by lookup again on every output row; the resolution now
  happens once per template, ahead of the row loop.

## [2.0.2] - 2026-09-14

### Bug Fixes

- **core:** Certified embedding source verification accepts one RDF 1.2
  reifier bound to multiple distinct statements, including citations projected
  from Markdown concordance rows with several anchors. Annotation source
  ordinals verify against the exact statement binding in the same graph;
  altered annotations, mismatched bindings, and stale ordinals remain rejected.

### Performance

- **gts:** Private digest indexes remove repeated blob and metadata scans from
  streaming reads, materialized reads, and segment union. Ordered replacement,
  inherited metadata, lazy payloads, and segment isolation retain their existing
  behavior. Tables of up to 16 digests avoid index allocation.
- **gts:** Standalone and streaming COSE decryption borrow fields from definite
  envelopes and allocate only the plaintext buffer and authentication data.
  Other accepted CBOR forms retain the complete decoder. For a 1 MiB plaintext,
  measured cumulative allocation requests fall by approximately 75% standalone
  and 37% through streaming, preserving authentication and error ordering.

### Tests

- **core/markdown:** Cover multi-anchor certified embedding sources, multiple
  statement bindings, graph separation, altered annotations, and stale ordinals.
- **gts:** Expand decryption compatibility and limit-boundary coverage, verify
  ordered blob replacement across snapshots and segments, and measure reader
  scaling and decryption allocation traffic.

## [2.0.1] - 2026-09-14

### Bug Fixes

- **gts:** Restore the public `cose::decrypt0` API for standalone encrypted
  objects. The original signature delegates to the bounded authenticated
  decryptor used by the GTS reader, preserving existing error behavior and
  compatibility with the frozen encryption vector.

## [2.0.0] - 2026-09-14

### Breaking Changes

- **BREAKING** **text:** Ranking now uses one bounded BM25F arithmetic law,
  including the single-field case. Intermediate rounding can change existing
  scores, and index fingerprints use the `index/v2` domain. Rebuild cached
  rankings and pin the complete named ranking profile; the exported 56-bit
  score width derives from its exact inclusive bound.
- **BREAKING** **shapes:** `json_schema::compile` and
  `compile_with_value_vocab` return `Result` and report unsafe pattern or
  namespace emission as typed errors. Callers must handle compilation failure.
  JSON Schema patterns target Unicode ECMA-262. XPath case-insensitive flags
  are refused because simple Unicode case folding does not preserve their
  semantics; unsupported Pydantic and reverse-import patterns also refuse
  rather than installing a constraint with a different meaning.
- **BREAKING** **core:** `Canonicalized` now reports the presentation that
  produced it in `presentation: CanonPresentation` — the fourth coordinate of
  the canonicalization pin `(profile, version, presentation, hash)`, readable
  directly off a result in hand. Rust callers constructing a `Canonicalized`
  struct literal or destructuring it exhaustively must supply or bind that
  field; callers receiving results from the canonicalization entry points are
  unaffected. This requires a major release under the suite's versioning
  policy.
- **BREAKING** **shapes:** `PropertyShape` now carries its declaring RDF node
  in `id: Term`. Rust callers constructing a property-shape struct literal must
  supply that identity; callers using `from_dataset` or `parse_shapes` receive
  it automatically. This requires a major release under the suite's versioning
  policy. The field preserves correct `sh:sourceShape` reports and
  SHACL-SPARQL `$currentShape` bindings even when property shapes share a path,
  are nested, or are cloned and validated independently.

### Bug Fixes

- **rdf/core/results:** XML egress shares one XML 1.0 character law across
  RDF/XML, TriX, SPARQL XML results, projections and SVG. Carriage returns use
  character references; attribute tabs and line feeds do too, preserving their
  values through XML normalization. Forbidden scalars produce an error naming
  the scalar. Dynamic SVG attributes and XMLLiteral normalization follow the
  same rule. These repairs deliberately change emitted bytes; apostrophes stay
  literal inside double-quoted attributes. Frozen conformance vectors are
  unchanged.
- **iri:** Bracketed hosts must satisfy the IPv6 or IPvFuture grammar;
  character membership alone no longer admits malformed IP literals. Generic
  ports accept arbitrary ASCII digit strings as RFC 3986 requires, including
  values above 65535 and leading zeros. Valid lexical spellings remain intact.
- **core/rdf:** Located duplicate quads keep their source locations on the
  deduplicated row during owned insertion, row remapping and GTS import.
  Unrealized location handles cannot attach to a later unrelated row, and
  validated append safely ignores unmatched handles. `push_quad_with_handle`
  returns the actual row
  handle for callers attaching locations; existing `push_quad` calls remain valid.
- **core:** Mutation suppression and reinsertion apply to ordinary quads,
  reifiers and annotations, including overlapping records. Statement-layer
  classification uses the reifier's graph as part of its identity. Snapshots and
  compaction preserve annotation-only rows and declaration-only named graphs.
  Independent dataset imports reserve explicitly interned blank scopes, including
  those referenced only inside composite literals.
- **shapes:** Borrowed statement projections deduplicate metadata overlapping
  ordinary rows, preserving graph-set semantics in SPARQL counts and validation.
  Disabled statement projection skips metadata reads, and bound subject probes
  use the source's metadata indexes. Property-pair constraints read shared views
  without materializing them, and validation shares one class index when Core
  and SPARQL use the same data view.
- **rdf/viz:** Properties of a known reifier render as annotation relations even
  when their graph differs from the reification declaration. Every relation keeps
  its original graph, and parsed and incrementally built datasets produce the
  same visual model. Corrected cross-graph visual models change newly generated
  visualization JSON, SVG and model hashes; RDF carrier records are unchanged.
- **sparql:** Compiler-built, rewritten and caller-mutated prepared algebra now
  passes structural and registry validation before execution. Malformed binding
  rows and excessively deep execution plans return diagnostics instead of
  panicking or overflowing the stack. Rewrites that need a different property
  function plan must be admitted again before execution.
- **sparql:** Repeated grouping variables use the same normalized columns as
  their result schema, preventing an aggregate after duplicate `GROUP BY` keys
  from indexing outside its output row. Repeated bare projection variables retain
  their existing normalization behavior.
- **gts:** Snapshot composition preserves the graph identity of reifiers and
  annotations during ingestion, canonicalization, deduplication and emission.
  Explicit default-graph relocation applies to all three RDF record tables.
  Named-graph metadata uses the existing graph fields in the wire format;
  default-graph record encodings and frozen vectors are unchanged.
- **core:** Directional language-tagged literals now intern with
  `rdf:dirLangString`, as required by RDF 1.2. Previously the builder used
  `rdf:langString`, so prepared SPARQL constants could not find those terms.
  The corrected datatype can change newly emitted carrier bytes and content
  identities for datasets containing directional literals. Regenerate derived
  artifacts from canonical inputs; previously authenticated bytes remain their
  original artifacts and must not be rewritten under an old identity.
- **core/bindings:** Native validation, pack and embedding readers, and C/Python
  literal inputs reject inconsistent datatype, language and direction fields.
  Python native literal equality and hashing distinguish opposite base directions
  and compare language tags without ASCII case sensitivity, including inside
  triple and quad keys. Length-framed keys preserve embedded control characters.
  Typed Python store operations and query prebindings normalize language tags
  consistently. Pack admission rejects noncanonical language tags, duplicate
  terms and unordered dictionaries before exposing lookup indexes.
- **shapes:** Repeated values of single-parameter constraint components apply
  independently and conjunctively. Importing standard SHACL component
  declarations no longer rejects legal repeated properties or executes native
  constraints twice. Multi-parameter components and native singleton parameters
  reject distinct competing values; identical statements in multiple graphs
  count once. Parameter declarations and validator attachments are checked
  without discarding malformed values.
- **shapes:** Optional target parameters remain unbound when omitted, and target
  instances missing mandatory parameters contribute no focus nodes. Subjects of
  `sh:target` are discovered as shapes even without explicit node-shape typing,
  including standalone property shapes. Scoped
  component validators take precedence over generic ASK validators. Property
  reports name their declaring shape, and reifier-constraint results use the
  enclosing property's severity. First-party report goldens deliberately correct
  34 source-shape references and one severity; conformance outcomes are unchanged.
- **core:** `PipelineBundle::accumulate_named_graph` refuses a contribution that
  carries a quad, a reifier row, an annotation row or a graph declaration outside
  the named graph it is being folded into. Such a contribution was previously
  admitted in silence and its stray rows joined the carrier under graphs the caller
  never named, so the carrier's per-graph digests then answered for content those
  graphs did not hold. This changes the behaviour of an existing entry point: a
  call that used to succeed now fails with `PipelineBundleError::GraphContainment`.
  A refused accumulation leaves the carrier exactly as it was, on the flat carrier
  and the view carrier alike — the union, the per-graph digest memo and the new
  handle are installed together, only after every check has passed, so there is no
  partially folded state for a reader to observe.
- **gts:** The snapshot builder refuses two distinct blank-node intern keys that
  encode onto one wire value. The frozen `"{scope}-{label}"` encoding is not
  injective over `(scope, label)` — `(Some("a"), "b-c")` and `(Some("a-b"), "c")`
  both spell `a-b-c` — and both keys previously interned onto the SAME term row, so
  two distinguishable blank nodes were published as one and every row naming either
  was repointed at the survivor. This changes the behaviour of an existing entry
  point: `add_dataset_scoped`, and the Python producers that call it, now refuse
  such a pair instead of merging it. The encoding itself is unchanged, so no input
  that did not collide moves a byte.
- **gts:** A failed ingestion can no longer be published. `emit_gts` refuses a
  builder that an earlier ingestion poisoned, and the ingestion that failed is
  rolled back: every row it had already interned is taken back out, so the
  infallible `snapshot_content_id` and `snapshot_payload` accessors describe the
  last fully accepted state rather than a truncated interior. Previously a caller
  that carried on past an ingestion error could emit a container holding whatever
  subset happened to intern before the refusal, which answers a question nobody
  asked.

### Features

- **json:** New core-only runtime crate `purrdf-json`, also available through
  `purrdf::json`, carries ordered JSON as RDF 1.2 and reconstructs the original
  bytes. Member order, duplicate names, whitespace, escapes, exact number
  spellings and empty containers survive production RDF serialization and
  parsing. Value occurrences expose paths and parent links, containers expose
  their sizes, and structural runs cover syntax bytes. Decoding requires an
  explicit document and profile, validates ownership, metadata, byte cover and
  digest, then reparses the source to verify every structural assertion before
  returning it.
- **text:** Immutable ranking profiles configure up to sixteen BM25F fields.
  Retained predicate-level token facts support field remapping and reweighting
  without tokenization. Corpus-bound prepared queries compute IDFs once and
  share the same scorer across ranking, explanations and heap ceilings. A
  short independent integer reference reproduces the exact conformance vectors
  executed on native and wasm targets.
- **core:** Canonicalization is idempotent over its own output, and the
  `purrdf-rdfc12` profile version moves from 1 to 2 to say so. A quad written in
  exactly one of the two shapes the RDF 1.2 overlay lowers into — a reifier row
  under `urn:purrdf:rdfc:reifies` over a triple term, or an annotation row in the
  lone `urn:purrdf:rdfc:annotation` graph slot — is folded back into the statement
  layer instead of being refused, because such a quad is that row written out and
  carries identical content. Re-parsing a canonical document and canonicalizing it
  again now returns it byte for byte; the graph-scoped entry points are idempotent
  without qualification, while a named-graph annotation still emits a five-token
  line no quad can carry. Every other use of the reserved namespace refuses exactly
  as before, including a reserved IRI in any slot of a folded row, and no input the
  previous version admitted changes bytes. Consumers pinning
  `(CANON_PROFILE_ID, CANON_PROFILE_VERSION, CANON_CORPUS_DIGEST)` must re-pin the
  latter two; two normative corpus cases move from refusal to golden.
- **sparql:** Prepared-query and join-order caches have deterministic entry and
  byte limits, configurable independently through additive policy APIs. Existing
  constructors use finite defaults. Cache counters expose retention and eviction;
  `PlanMemoryObserver` separately tracks admitted allocations still held by
  callers after eviction or cache destruction. These payload estimates exclude
  allocator overhead and do not claim to measure process memory.
- **sparql:** Immutable prepared plans can execute under a shared operation
  governor, including fallible dataset views and federated sources. Typed
  substitutions, registry checks, cancellation, completeness and operational
  failure precedence retain the existing evaluator's behavior. Evaluation state
  stays local to each worker.
- **core:** Typed dataset import memoizes source terms and streams flat quads;
  canonical relabeling avoids building an unused text representation. Existing
  scope, metadata and deterministic output contracts are preserved.
- **core:** Immutable composite and mutation views retain native source
  dictionaries and indexes. Independent-document composition standardizes blank
  scopes apart, including nested triple and composite-literal references; an
  explicit shared-scope constructor preserves already-established identities.
  Graph placement covers all RDF record tables and graph declarations. Finite
  retention limits and copy/freeze/materialization counters make ownership costs
  observable without changing deterministic RDF identities. Composite, delta and
  SHACL adapters reuse prepared physical probe plans across bindings, accounting
  for graph placement and union projection.
- **shapes:** `PreparedShapes` shares immutable shape and class-reference analysis
  across exact dataset bindings. Borrowed native, composite and delta views avoid
  rebuilding a base dataset for constraint, path and target evaluation. Each
  binding resolves its own dataset IDs and targets; concrete dataset compatibility
  getters materialize lazily.
- **sparql:** Typed prepared CONSTRUCT methods stage and validate graph results
  directly into a destination builder, with destination-aware fresh blank nodes
  and explicit copy counters. Shared-governor and fallible-view variants withhold
  publication after errors, cancellation or exhausted budgets. UPDATE WHERE
  evaluation reads mutation snapshots without compacting the whole base.
- **markdown:** New crate `purrdf-markdown`, a structural Markdown-to-RDF 1.2
  slicer. A document becomes a graph of its own headings, verses and paragraphs
  over verbatim byte spans of the source, and the resulting nodes carry
  content-addressed identities that two independent runs of the same law agree
  on. The law is published, not implied: `SPEC.md` ships inside the crate and
  states the dialect grammar, the split law, the concordance law, the three
  identities and their verification law, ordering, conformance and the emission
  law, and the crate's vectors are read against that document. The vocabulary is
  caller-supplied: `Vocabulary::under(base)` derives the whole term set from one
  base, the term set and not merely the base is inside the contract identity,
  and no term is written in two roles. `Vocabulary::standard()` offers one
  designated namespace, `https://w3id.org/purrdf/markdown#`, by name rather than
  on a caller's behalf, for deployments that want a shared one; it is a stable
  identifier that does not dereference today, a redirect for it is being
  registered, and nothing rests on that because RDF asks no IRI to resolve.
  Beside the IRIs derived from the caller's own vocabulary base — its terms and
  the node identities minted under it — an emitted graph carries no IRI but
  these: five of the standard's — `rdf:type`, `rdf:reifies`, `xsd:integer`,
  `xsd:hexBinary`, and the `xsd:string` that the graph's plain literals — a
  unit's text, a heading's words — have in the abstract syntax and that no
  serializer writes — the caller's own source id, and, where a concordance
  lifts under a declared canon base, the anchors it mints.
- **markdown:** The slicer is a codec. The emission covers every byte of the
  source: a section carries its own heading or movement line as a typed
  `verbatim` literal over that line's span beside its heading's words as the
  section's one plain literal, and every maximal run of bytes neither a unit
  span nor a heading line covers — blank runs, rules, table rows, the newline
  a split cut lands on — is a `Structure` node carrying its bytes the same
  way. Plain means content and typed means bytes, normatively: the only plain
  literals in a claim are a unit's text and a heading's words. The vocabulary
  grows `Structure`, `verbatim`, `verbatimStart`, `verbatimEnd` and
  `verbatimText`; every contract id re-mints and every chunking id — and so
  every embedding addressed by one — stands still. `decode_document` is the
  graph half of the new decode law (`SPEC.md` §11.3) and `analyze` holds every
  admitted document against it in every build, refusing a defective cover as
  `CoverDefect` rather than shipping a graph nothing can rebuild. `ClaimKind`
  gains `Structure` and stays exhaustive on purpose: a consumer's match
  breaking loudly on a new claim kind is the correct failure.
- **core:** New module `purrdf_core::cover`: the cover law — verbatim byte
  spans back into the document they cover, byte for byte — as the kernel's
  own, format-neutral surface. `reconstruct` writes each span at its offset,
  requires the cover whole, admits an overlap only where a `continues`
  declaration names a strictly earlier piece whose shared bytes agree, and
  proves the result against the stated source digest, with eight typed
  refusals and allocation bounded by input. `purrdf-markdown` re-exports it as
  the law's first emitter; the next ordered codec costs an emitter and
  nothing else.
- **purrdf:** The umbrella exposes the slicer as `purrdf::markdown`, so a
  consumer slices a document without naming `purrdf-markdown` as a separate
  dependency.
- **core:** The IRI re-export widens from `IriError` alone to `BaseIri`, `Iri`
  and `parse_iri` beside it. A producer that mints IRIs for the kernel can now
  refuse up front exactly what the kernel would refuse at intern time, asking
  the same question of the same law rather than reimplementing it or taking its
  own dependency on `purrdf-iri`.
- **core:** `PipelineViewBundle<H>`, the view-backed pipeline carrier: the same
  out-of-band material, typed-handle lane and content answers `PipelineBundle`
  carries, over a composed `CompositeDatasetView` instead of an owned dataset.
  Folding a named graph in appends a retained source rather than copying rows, and
  freezing happens exactly once, at an ownership boundary the caller asks for. The
  two carriers are interchangeable by content: equal content yields equal
  per-carrier and per-graph digests and an equal `pipeline_root`, the
  domain-separated fold over a carrier's whole surface that both carriers now
  answer. A view carrier holds caller-supplied sources, so its digests run through
  the fallible canonicalization entry points and report a refusal rather than
  panicking; the bytes agree with the flat carrier's on success.
- **core:** `CompositeDatasetView::extend` composes one more source onto an
  existing view, aliasing only the new contribution against the identity space the
  retained sources already settled. The result is the view a from-scratch
  composition of the same source list would have built — the same rows, canonical
  identity, named graphs and retention — without paying that whole composition
  again at each step. Each retained source also carries its own `ScopeBinding`, so
  one composition can standardize some sources' blank scopes apart while retaining
  others' established identities, decided per source rather than per view.
- **core:** `RetentionLedger`, with `RetentionGuard`, `RetentionSnapshot`,
  `RetainedCharge`, `OwnerKey`, `OwnerMutability` and `ViewAccountingReport`,
  reports what several carriers jointly hold resident: a base is charged once
  however many carriers share it, and released when its last reader drops. The
  ledger admits, refuses and resizes nothing — `ViewLimits` remains the sole
  admission gate and remains per view — and `ViewAccountingReport` pairs the
  deduplicated ledger figure with one view's own incremental charge so that no byte
  is counted in both.
- **core:** Canonicalization has view-generic entry points: `canonicalize_view`,
  `try_canonicalize_view`, `canonicalize_graph_view`, `try_canonicalize_graph_view`,
  `graph_digest_view`, `try_graph_digest_view`, `check_admissible_view` and
  `blank_count_view` all take any `DatasetView` — the frozen dataset, composite
  views and delta views alike — so a composed surface is canonicalized without
  being materialized first. The existing dataset-shaped entry points delegate to
  them and are unchanged in behaviour and in bytes.
- **core:** `datasets_isomorphic` and `dataset_diff` are view-generic with the two
  sides INDEPENDENTLY typed, so a composite view, a delta view and a frozen dataset
  compare against each other in any combination without either side being
  materialized; identity is still decided from the canonical bytes, which carry no
  view-local ids. Ordinary `&RdfDataset` call sites are the dataset instantiation
  and keep compiling unchanged. Compatibility caveat: a generic function has no
  single function type, so a caller that stored either of these in a function
  pointer or passed it where a concrete `fn` was expected must now name the
  instantiation it means.
- **core:** `CompositeSource::from_selection` retains chosen graphs of an
  existing `Arc<CompositeDatasetView>` as one composable source. A single
  selection filters ordinary quads, reifier rows, annotation rows and
  declaration membership together, keeps the retained composite's owners and
  canonical blank scopes rather than copying or re-scoping anything, admits
  under the same view limits as every other source, and composes with
  `with_graph_placement` and `with_scope_binding` unchanged. Selecting a graph
  the composite does not hold is the typed `view-graph-selection` refusal, and
  a carrier assembled over a selection keeps the selection's underlying owners
  registered with the retention ledger, transitively.
- **core:** `PackBuilder::build_view_bytes` writes the existing pack format
  from any `FallibleDatasetView` — composite, delta or graph selection —
  through the one encoder; `build_bytes` over a frozen dataset is now a
  delegation through it, so flat and view output are byte-identical by
  construction. The view's operational status is checkpointed before and after
  the drain, and a view that faults mid-read yields no pack bytes at all.
- **gts:** `SnapshotBuilder::add_view` and `add_view_scoped` ingest any
  `FallibleDatasetView` into a snapshot directly — no temporary dataset, no text
  round trip, no per-row owned term reconstruction — with the same
  graph-assignment and blank-scope hooks the flat surface exposes. They return an
  `IngestReport`: rows consumed, terms minted, peak scratch bytes, and the
  declaration-only graph names the ingestion deliberately did NOT intern, so that
  omission is stated rather than silently taken. `ingest_totals` reads the
  cumulative figures off a builder, and every ingestion refusal is the typed
  `GtsIngestError`. The frozen `add_dataset` / `add_dataset_scoped` surface now
  delegates to the same ingestion core, unchanged in signature and in bytes.
- **python:** `compile_gts_with_report` returns a GTS container together with the
  ingestion receipt for the very build that minted it, from ONE compile — a dict
  with `snapshot_bytes` and `ingest_report`, where the bytes are identical to
  `compile_gts_native`'s for the same sources. `gts_ingest_report` remains the
  accessor for a caller who wants the receipt and no bytes: it takes the compiler's
  ingest-half arguments and returns `rows_consumed`, `terms_interned`,
  `declarations_omitted` and `scratch_bytes`. Both are additive; every existing
  producer entry point keeps its signature and its bytes.

## [1.1.0] - 2026-09-04

The first release after 1.0.0. Two reported bugs, the release fallout 1.0.0 left
behind, and a data-conflation bug in the GTS carrier found while coordinating
with the upstream wire-format freeze.

Additive: no published API changes shape. A `TermRow` widening that would have
forced a major bump was reworked into a parallel column before release — RDF 1.2
base direction is now `purrdf_rdf::gts_view::term_directions()` and a
`directions` key on the Python projection dict, so every 1.0.0 caller keeps
working.

### Bug Fixes

- **shacl:** `sh:message` templates are substituted on the `sh:sparql` path.
  `{$path}`/`{$value}` shipped as literal braces because the constraint's
  message was resolved once and cloned into every result row; it is now rendered
  per solution, from that solution's own bindings. Both `{?var}` and `{$var}`
  spellings, and `{$this}` whether or not the SELECT projects it. An unbound
  placeholder is still left verbatim rather than blanked.
- **cli:** `purrdf validate` resolves a shapes graph's `owl:imports` from
  `--import IRI=FILE`, transitively and cycle-safely. Previously they were
  ignored in silence, so a shapes graph whose shapes lived in its imports
  reported `conforms true` against no shapes at all. PurRDF fetches nothing: an
  unresolved import with no `--import` given is reported on stderr, and with a
  table given is refused by name.
- **gts:** RDF 1.2 literal base direction is part of a term's identity in the
  snapshot composer. It was absent from the intern key, so `"Cat"@en--ltr` and
  `"Cat"@en--rtl` merged into ONE term and every quad was repointed at the
  survivor — a different graph, not a missing column. Emitted bytes are
  unchanged for any input without directional literals.
- **release:** The crates.io bootstrap script packages and publishes its PLAN
  rather than the whole ledger; a deferred crate no longer reaches `cargo
  package`. Its self-test asserts the arguments handed to the irreversible step,
  and the arm that had been failing since the ledger emptied is fixed — that arm
  runs before `cargo test` in `make check`, so it had been failing the gate for
  every contributor.

### Features

- **python:** `gts_to_sqlite`, `gts_to_duckdb` and `gts_to_parquet` are
  implemented. They previously raised unconditionally. Five tables in the
  projection's own row order, so the same container exports to the same content
  twice. SQLite needs only the standard library; the other two take the
  `[duckdb]` / `[parquet]` extras.
- **rdf:** `gts_view::term_directions()` exposes RDF 1.2 base direction from the
  relational projection, which previously dropped it.
- **entail:** `entails::imports::imported_iris()` is public, so a consumer can
  read a document's `owl:imports` without re-deriving which objects count.

### Documentation

- **pack:** The pack identity digest is no longer captioned "RDFC-1.0". It is
  computed through the `purrdf-rdfc12` profile, which agrees with RDFC-1.0 byte
  for byte only on the RDF 1.1 subset. Labelling only — no digest bytes change.

### Build

- **ci:** Benchmarks moved off the per-push path to a weekly schedule plus
  `workflow_dispatch`, and its `uv` installer is pinned.
- **make:** `make doctor` reports which build pins the local machine actually
  enforces, and the wasm gate distinguishes "rustup absent" from "target not
  installed" instead of printing the same line for both.
- **python:** `bench = false` on the binding lib, so `cargo bench --workspace`
  links.

## [1.0.0] - 2026-09-02

The first release under full semantic versioning. The tree is the 0.13.0 tree:
0.13.0 was published only to create the crates.io records for `purrdf-cdt`,
`purrdf-geo` and `purrdf-text` — Trusted Publishing cannot create a crate that
does not exist — and 1.0.0 is the same code republished through Trusted
Publishing across all 21 crates, PyPI and npm. There is no functional change
between the two.

### Documentation

- **release:** From 1.0.0 a breaking change bumps the major version, a minor
  bump is additive, and a patch bump is bugfix-only; the changelog's
  **BREAKING** markers name each one. The C ABI (`purrdf.h`,
  `PURRDF_ABI_MAJOR.PURRDF_ABI_MINOR` = 0.7) is versioned separately and
  remains 0.x. The changelog header, its `cliff.toml` template and the release
  docs now state that policy instead of the pre-1.0 one.
- **release:** `make bump` now regenerates `purrdf.h`, because cbindgen derives
  `PURRDF_MINOR` from the crate version and the 0.13.0 bump left the committed
  header one integer behind, failing `capi-check` in CI.

## [0.13.0] - 2026-09-02

This release re-founds the aggregate algebra on the SPARQL specification's own shape, adds a
caller-extensible aggregate registry (with a first-party statistical set), retains and enforces
the `VERSION` declaration, adds RDF 1.2's `ADJUST` and the underlying F&O temporal operation
table, corrects several results-format spellings, adds `LATERAL` (SEP-0006) surface syntax
alongside a corrected correlated-evaluation substitution, and rebuilds `EXISTS`/`NOT EXISTS` on
SEP-0007's defensible substitution semantics (one definition, two proven-equivalent strategies,
and the Part 3 assignment restriction). It carries a large number of breaking surfaces; each is
called out below with what a consumer must do.

### Bug Fixes
- **BREAKING** **core:** `check_provenance` now measures the sidecar against the dataset in BOTH
  directions, and an empty quad set means "the dataset is empty", not "check nothing". Rule 3
  (every dataset quad has at least one occurrence) previously ran only when the handle slice was
  non-empty, so a caller that forgot to collect its handles — or passed `&[]` to mean "skip" —
  got an `Ok(())` indistinguishable from a real pass. The parameter is now the dataset's
  complete quad-handle set, rule 3 runs unconditionally, and a new rule 4 refuses every
  occurrence whose quad is not in that set with
  `ProvenanceError::DanglingQuad { occurrence_index, quad_index }` — the quad-axis twin of
  `UnknownUnit` / `UnknownArtifact`. An empty sidecar for an empty dataset still passes (every
  rule ran over zero elements, which is a checked pass); a non-empty sidecar checked against
  `&[]` now fails, as does an occurrence for a quad outside a non-empty set. A caller that passed
  a subset of its quads, or `&[]` to run rules 1–2 alone, must now pass the full set. The
  signature is unchanged and `ProvenanceError` is `#[non_exhaustive]`, so the new variant is
  not itself a source break; the accepted-input set is what changed.
- **sparql-algebra:** A language tag's base direction is exactly `ltr` or `rtl`, lower case, and
  anything else is a syntax error. SPARQL 1.2 §2.3.1: "The base direction is restricted to
  either `ltr` or `rtl`. Unlike a language tag, it is always lower case." The parser SILENTLY
  DROPPED an unrecognised `--` suffix — `"x"@en--foo` parsed as `"x"@en`, and `"x"@en--LTR` as
  `"x"@en--ltr` — where the W3C rdf-tests pin both spellings as negative syntax ("undefined base
  direction", "upper case LTR"), and the pre-release sweep had folded the comparison to
  `eq_ignore_ascii_case`, which the specification forbids. The language half is now held to the
  `LANG_DIR` production `[a-zA-Z]+ ('-' [a-zA-Z0-9]+)*` at the same site, so `@en-`, `@en--`,
  `@1en` and `@en--x--ltr` are refused too, while `@en`, `@en-US`, `@zh-Hant-TW`, `@x-klingon`,
  `@en--ltr`, `@ar--rtl`, `@en-US--rtl` and `@EN--ltr` (only the DIRECTION is case-restricted)
  all still parse. The vendored `lang-basedir/langdir-literal-invalid.rq` could not catch this:
  it spells its projection `AS v`, which is a syntax error on its own, so the harness refused it
  for the wrong reason.
- **core:** The `pack_query` dictionary benchmark measures again. Its literal-heavy fixture
  (added by the pre-release sweep) asserted a quoted triple term as a quad's SUBJECT, which
  RDF 1.2 does not admit and the freeze gate refuses (`rdf-ir-triple-subject`), so the fixture
  panicked and the report-only `benchmarks` CI job had been red on `main` since `b4f99093`. The
  triple term is now asserted in the object position (`s q <<s p plain>>`), the one place the
  model puts it, and the dictionary closure still resolves one triple term per row.
- **core:** `MutableDataset` no longer fabricates a literal datatype. When a base literal's
  datatype id resolved to something other than an IRI, `base_value_of` rendered that term's
  `Debug` form and used it as the datatype IRI, where `RdfDataset::term_value` states the same
  invariant with `unreachable!`. The fabricated value could not fail at the site; it would
  surface later as an `IriError` about a string nobody wrote. The path is unreachable from the
  public API (the builder interns every datatype through `intern_iri`, and the pack decoder
  refuses a non-IRI datatype entry before a pack becomes a dataset), so the two sites now agree
  on `unreachable!` with the invariant stated, and a test pins that `base_value_of` and
  `term_value` agree on every literal shape.
- **BREAKING** **sparql-eval,geo:** A host function registered through
  `UserFunctionRegistry::register_native` can now raise a SPARQL **expression error** for one
  solution instead of failing the whole query. `NativeFnBody` returns
  `Result<Option<TermValue>, EvalError>` rather than `Result<TermValue, EvalError>` — the same
  three-exit channel `register_expr`'s `ExprFnBody` already carried — so `Ok(None)` means "this
  call has no value for this solution" and `Err` stays reserved for a query-fatal condition. The
  evaluator, not the closure, then applies the outcome the calling context requires: a `FILTER`
  eliminates that solution (SPARQL 1.1 §17), while a `BIND` or `SELECT` expression leaves the
  variable unbound and continues (§10; algebra §18.5 `Extend`). Previously the seam had no
  `Ok(None)` exit at all, so every domain refusal — a malformed literal, an out-of-range index, a
  type mismatch — had to be spelled `Err` and aborted the query; one bad geometry anywhere in a
  dataset failed every query that scanned past it. This was found while merging `purrdf-geo` and
  affected every function on the seam, not just the `geof:` family. Callers with a
  `register_native` body must wrap their success value in `Some` and should move argument-level
  refusals from `Err` to `Ok(None)`.
- **BREAKING** **geo:** `functions::evaluate` returns `Result<Option<TermValue>, EvalError>` (the
  seam shape above) and maps `GeoError::Literal`/`GeoError::Domain` onto the per-solution
  `Ok(None)`; `GeoError::Unsupported`, `GeoError::Config` and the new `GeoError::Arity` stay
  query-fatal, because each holds for every solution alike and answering "no value" would empty a
  result set and present that as the answer. `GeoError` gains an `Arity` variant (a wrong argument
  count is nobody else's mistake — not the data's, not the wiring's) and a
  `GeoError::is_expression_error` predicate that is the single site deciding how far a refusal
  travels. A caller that needs the refusal itself — with its message and kind intact, which a
  SPARQL expression error by construction cannot carry — calls the new `functions::compute`,
  which answers in `Result<TermValue, GeoError>`.
- **shapes:** A SHACL validation report no longer FUSES a blank node it mints with one it
  carries. The report invents `_:report`, `_:r0`, `_:r1`, … and the interior nodes of a
  complex `sh:path`; blank-node labels arriving from the data or shapes graph are opaque
  strings that pass through the IR verbatim, so a data graph containing `_:r0` produced
  `_:r0 a sh:ValidationResult ; sh:focusNode _:r0` — the validation result and the node it
  reports on silently became ONE node, with the report asserting that a
  `sh:ValidationResult` was an instance of the data's own class. Nothing was dropped and no
  error was raised. The minted nodes now take a reserved label prefix whenever, and only
  whenever, the report actually carries a colliding label, so a report with no collision is
  byte-identical to before (the byte-frozen first-party corpus reports are unchanged).
- **entail:** OWL-Direct now DECIDES the `SHOIQ` nominal / inverse-role / qualified-number-restriction
  corner. Both decision cores implement the nominal-introduction rule — Horrocks & Sattler's `NN`-rule
  in the `cfg(test)` concept-tree reference and Motik–Shearer–Horrocks' Table 5 `NI`-rule in the
  production hypertableau — so an ontology with an at-most over an inverse role (an
  `owl:InverseFunctionalProperty`, or the vendored W3C spy-point `webont-description-logic-035`) is
  decided rather than answered `consistent-within-boundaries`; `webont-description-logic-035` now
  decides **inconsistent**, matching its published verdict, and is graded on every conformance run.
- **entail:** **Removed** the `counting-on-inverse` completeness boundary
  (`report::Construct::CountingOnInverse` and its `"counting-on-inverse"` certificate line, plus the
  `decided-within-boundaries` completeness it forced there). The corner is now decided, so a
  reasoning report over it no longer carries that boundary; a consumer that matched the
  `"counting-on-inverse"` boundary name or expected `decided-within-boundaries` there will instead
  see a plain `decided` verdict (or, past the counting ceiling, an honest `budget-exhausted`).
- **BREAKING** **sparql:** The `VERSION "1.2-basic"` declaration is now actually enforced as the
  SPARQL 1.2 Basic profile (full 1.2 syntax minus RDF 1.2 triple terms), not merely retained and
  round-tripped. Evaluation admission now refuses a triple term in any triple/quad pattern or path
  endpoint, a ground triple term in `VALUES`, and the "Functions on Triple Terms" builtins, naming
  the offending construct — for an update, with no mutation applied. Callers that declared
  `1.2-basic` and relied on it silently running the full engine must drop the declaration or
  remove those constructs from the request.
- **BREAKING** **sparql-eval:** An `UPDATE` prologue declaring an unrecognized `VERSION` (anything
  other than `"1.2"`/`"1.2-basic"`) now refuses at admission instead of mutating the store — the
  byte-identical read-only query form was already refused. Callers issuing such requests must
  stop, declare a recognized version, or omit the declaration.
- **BREAKING** **sparql-algebra:** `'*'` is refused for every aggregate but `COUNT` (it was
  previously accepted everywhere and silently answered the group's row count for `SUM(*)`,
  `AVG(*)`, `MIN(*)`, `MAX(*)`, `SAMPLE(*)`, and `GROUP_CONCAT(*)`). `AggregateExpression::new` is
  now the sole, checked constructor — it returns a `Result` and refuses `*`/an empty argument list
  for any aggregate but `COUNT` — and `args` is no longer a public field; callers that built the
  node with struct-literal syntax or read `args` directly must switch to `AggregateExpression::new`
  and the `args()`/`into_parts()` accessors.
- **BREAKING** **sparql-eval:** The aggregate accumulator trait gained a required `into_any`
  downcast (used to merge partial fold states by concrete type instead of recovering only the
  finished term) and its `combine` method now returns `Result<(), EvalError>` instead of `()`, so
  a host contract violation (a partial state of the wrong concrete type) is a typed refusal rather
  than a panic — which matters on `wasm32-unknown-unknown`, where a panic aborts the whole
  instance. Every implementor of `AggregateAccumulator`, including any host-registered
  `CustomAggregate`, must add `into_any` and update `combine`'s return type and propagate the
  downcast helper's error.
- **BREAKING** **sparql-eval:** A prepared plan's aggregate/property-function admission now keys
  on registry INSTANCE identity (a process-monotonic `RegistryId`), not declared metadata alone —
  two independently built registries that declare identically for a shared IRI but resolve it to
  different implementations previously produced the same plan fingerprint, so a plan prepared
  against one registry could execute unrefused against the other. A caller relying on two
  distinctly constructed registries with identical declarations being interchangeable at execution
  time now gets a typed refusal instead of a silently wrong answer; cloning a registry still
  shares its source's identity.
- **BREAKING** **sparql-eval:** Planner admission failures are now attributed to the extension
  seam that actually raised them. An unregistered custom aggregate or an aggregate arity violation
  previously reported the property-function diagnostic code; callers matching on the documented
  aggregate diagnostic code must now handle those failures there instead of under the
  property-function code.
- **BREAKING** **sparql-eval:** The within-group parallel aggregate fold now sizes its chunks from
  the group's row count alone instead of the live host's thread count, so governed outcomes for
  large aggregate folds no longer vary with worker-pool size. Governed outcomes for large
  within-group aggregate folds change on any host whose worker count differs from the new fixed
  reference parallelism; a consumer pinning `GOVERNOR_CORPUS_DIGEST` or `GOVERNOR_PROFILE_DIGEST`
  must re-pin both.
- **BREAKING** **xsd,sparql-eval:** Integer `SUM`/`AVG` now accumulate at arbitrary precision
  instead of a fixed-width accumulator, so a running total that used to overflow into an unbound
  answer now answers exactly (for example, a large positive, a one, and the matching large
  negative now sum to `1` instead of unbound). This changes only the answering direction — a query
  that previously received `unbound` for an overflowing integer `SUM`/`AVG` now receives a value —
  and needs no caller action beyond expecting an answer where one is now owed.
- **BREAKING** **shapes:** `Shapes` gains a public `aggregates: Arc<AggregateRegistry>` field,
  installed at every SHACL-SPARQL validation entry point (including the parallel focus-node chunk
  fork, which previously dropped the aggregate scope across the thread boundary, making a
  registered aggregate's resolution depend on the number of focus nodes). Callers constructing
  `Shapes` via struct literal must now supply `aggregates`, or use `Shapes::default()` / the
  parser's constructor, both of which populate it with an empty registry.
- **BREAKING** **cli,python,purrdf:** A dataset-derived property function combined with an
  entailment regime returned a SHORT answer, reported complete, at a success exit and with no
  diagnostic. The registry is built by the caller, before the call, and therefore before the
  closure exists — so a `--path-relation` walk (or a `path_relations` / `relations_from_graph`
  registration) read the SOURCE data while every other pattern in the same query read the
  closure. Over `ex:sub rdfs:subPropertyOf ex:p . ex:a ex:p ex:b . ex:b ex:sub ex:c .` under
  `rdfs`, `SELECT ?end WHERE { ex:a ex:p+ ?end }` answered `ex:b, ex:c` and the equivalent walk
  answered `ex:b` alone. Both entry points now materialize the closure FIRST and register the
  relations against it, so the two halves of one query read one dataset: `query_with_entailment`
  and `query_with_entailment_governed` take a new `relations: &ClosureRelations<'_>` argument
  (after `options`, before `governors`). Rust callers whose relations are dataset-independent —
  an in-memory table, an empty registry — pass `ClosureRelations::NONE` and keep their present
  behaviour byte for byte; a caller with a dataset-derived relation passes
  `ClosureRelations::rebuilt_by(&f)`, where `f` is handed the materialized closure and returns
  the registry to answer with. The CLI and Python surfaces are unchanged in shape and now supply
  the rebuilder themselves; the C ABI and WebAssembly surfaces register no relation and pass
  `NONE`. One pairing is refused rather than answered: a rebuilder combined with an OWL
  Direct-Semantics run whose restricted chase MINTED existential witnesses, because a walk over
  that closure could return a minted blank node as an observable binding and the regime's witness
  filtration cannot reach a property function's output. It carries the stable code
  `reasoning-closure-relation-witness` (exit 2 from the CLI, `ValueError` from Python) and names
  the regimes that accept the pairing; an `owl-direct` run that mints no witness is not refused.
- **BREAKING** **iri,rdf,cli:** A relative IRI reference with no base IRI in scope is now a hard
  error instead of being interned verbatim. Documents that previously "worked" this way were
  emitting N-Triples no conformant parser accepts, so the failure surfaces an existing defect
  rather than introducing one. Reference resolution is now a single layer, `purrdf-iri`, shared by
  every codec: the RFC 3986 §5.1 precedence chain is an in-document directive
  (`@base`/`BASE`/`xml:base`/`@context.@base`), else a caller-supplied base, else the document's
  retrieval IRI, else the §5.1.4 failure. The stable codes are `iri-relative-no-base` (fixable by
  supplying a base), `iri-not-absolute-by-grammar` (N-Triples, N-Quads, TriX and HexTuples admit no
  relative reference at all, so a base cannot help) and `iri-non-absolute-base` (the supplied base
  has no scheme). Three surfaces have a retrieval IRI, all of them ones that opened the file
  themselves: `purrdf-slice` derives it (the workspace's single RFC 8089 `file://` derivation),
  and `purrdf-shapes`' shape-union loader and `purrdf-cli` consume that derivation rather than
  repeating it — so a file input needs no flag on any of the three. Every surface handed BYTES —
  the `purrdf-rdf`/`purrdf-iri` library APIs, wasm, the C ABI, Python and CLI stdin — has no
  retrieval IRI and hard-fails as §5.1.4 specifies. Callers on
  those surfaces must give the document a base directive or pass one to the API. On the way out,
  a syntax that can express a base (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD) now emits it and
  relativizes against a supplied base; one that cannot (N-Triples, N-Quads, TriX, HexTuples) keeps
  writing absolute IRIs. See "Base IRIs & Relative References" in The PurRDF Book.
- **BREAKING** **rdf:** `serialize_dataset_base_only` is **removed**, and `serialize_dataset_with`
  is added as the one serialization seam the rest of the family is now expressed through. The
  split family could not state a document base together with a graph selection or the RDF 1.2
  statement layer: `serialize_dataset` took the selection and the layer but no base, while
  `serialize_dataset_to_format` took a base but forced `SerializeGraph::Dataset` and the transcode
  projection — so asking for a base on RDF/XML silently traded away reifier and annotation rows
  the RDF/XML emitter can in fact render. `serialize_dataset_with(dataset, format, base_iri,
  &SerializeOptions { selection, statement_layer, jsonld_options })` states all four axes, and the
  new `StatementLayer` enum makes the third an explicit choice — `Emit` (render it, or fail closed
  where there is no surface for it), `Project` (drop it and REPORT the count), or
  `PerFormatCapability` (the registry's `carries_star()` decision, which is what every
  `*_to_format` spelling applies). `serialize_dataset`, `serialize_dataset_with_jsonld_options`,
  `serialize_dataset_to_format` and `serialize_dataset_to_format_with_jsonld_options` keep their
  signatures and behaviour and are one-expression delegations, so there is no second code path.
  Replace `serialize_dataset_base_only(d, media_type, selection)` with `serialize_dataset_with`
  under `StatementLayer::Project`, which additionally hands back the dropped-row count instead of
  leaving the caller to recompute it.
- **BREAKING** **capi:** The C ABI moves `0.6.0` → `0.7.0`, an **incompatible** bump.
  `purrdf_shacl_validate_to_sarif` and `purrdf_shacl_entail_to_ntriples` each gained a
  `shapes_base_iri` parameter **in the middle** of the existing list, between `shapes_ttl` and
  `data_nt` — a host compiled against `0.6.x` and run against `0.7.0` without recompiling passes
  `data_nt` into the `shapes_base_iri` slot and its `PurrdfBuffer **` out-pointer into `data_nt`,
  which the boundary then reads as a NUL-terminated C string. That silent, unguardable misread is
  the whole reason the version moved; the parameter is positional rather than appended because it
  belongs beside the document it qualifies. `purrdf_serialize_jsonld_configured` likewise gained
  `base_iri` after `media_type`, the slot it holds on `purrdf_serialize`. Both breaks ride the one
  bump because `0.7.0` is unreleased, so a consumer recompiles once rather than twice for one
  reason. Every C host must recompile against the new `purrdf.h`. Signature drift is now caught at
  test time: `crates/rdf-capi/tests/abi_signatures.rs` pins the complete exported prototype list
  against a committed snapshot and against the version triple, so a future incompatible change
  cannot reach a release without an author deliberately moving the version.
- **BREAKING** **shex,cli:** A shape map naming a shape label the schema does not declare — and
  `START` against a schema that declares no start shape — is now a hard refusal
  (`ShexError::UnknownShape`; CLI exit **1**) instead of a `"status":"nonconformant"` result at
  exit **0**.
  Scripts that parsed the JSON result and ignored the exit code will now see a failure where they
  previously saw a definite negative about the data; that is the point, since the old answer spent
  the format's one word for a finding about the DATA on a mistake the data had no part in — a typo
  in a shell's own argument read back as a validation verdict. Labels reachable through the import
  closure count as declared. The refusal happens before selector expansion, so a selector matching
  no node is refused identically to one matching many. The ShEx specification's ShapeMap status
  vocabulary has no value meaning "not evaluated", so this was resolved on the project's
  hard-fail doctrine rather than on anything the specification requires.
- **BREAKING** **cli:** `--base` is now refused by name when NEITHER leg of the operation can
  spend it, instead of being accepted and silently never read. A base is spent on parse (the
  source syntax admits a relative reference) or on serialize (the target syntax can write a base
  directive); `convert --from ntriples --to ntriples --base http://example.org/` satisfies
  neither and previously exited 0 having done nothing and said nothing. It is now a usage error
  (exit **2**) naming each leg and why it cannot take the value. A base ANY leg can spend is still
  honoured, so `--base X --to ntriples` continues to resolve the input. The same refusal covers a
  pack `--from`/`--to`, which carries no document base at all. Scripts passing `--base`
  unconditionally across format pairs must drop it on the pairs that cannot use it.
- **BREAKING** **rdf:** `GtsFoldView::new` and `GtsFoldView::with_config` now return
  `Result<Self, RdfDiagnostic>` instead of `Self`. They refuse a graph whose term table lets a
  term resolve through itself, with the code `gts-self-reaching-term`. The view's accessors —
  `nq_token`, `public_value` and everything built on them — walk a quoted triple's resolved
  components down to the leaves, so a self-reaching term recursed without bound and aborted the
  process; the view now refuses to EXIST rather than hand back an object whose every renderer is
  a process kill (the fold-time refusal GTS-SPEC §7.3 permits, applied once at construction
  instead of as a guard inside every walk). A graph read off the wire cannot contain one — the
  reader already refuses the row that would close the loop — so this reaches only callers who
  assemble a term table themselves. Rust callers must handle or propagate the `Result`; `?` is
  usually the whole change.
- **BREAKING** **python:** `GtsFoldViewNative.from_bytes` and `GtsFoldViewNative.from_parts` now
  raise `ValueError` carrying `gts-self-reaching-term`, for the reason above. `from_parts` is the
  reachable one: it is handed a caller-assembled term table, which `from_bytes`' reader validates
  on its own. Python callers constructing a fold view from parts must handle `ValueError`.
- **BREAKING** **rdf:** The byte-reproducibility classifier for `CONSTRUCT` dataset-description
  views now refuses a custom aggregate call, a custom scalar-function call, or any `SERVICE`
  clause (including `SERVICE SILENT`), matching the registry-dependency doctrine the
  property-function arm already applied — these depend on a registry or endpoint the classifier
  cannot inspect, so a view using them is not byte-reproducible. Callers who relied on such views
  being accepted must configure them without those constructs, or accept the resulting refusal.
  Built-in aggregates and built-in functions are unaffected.
- **rdf:** The byte-reproducibility refusal above, and the planner's two admission seams, now name
  the CONSTRUCT rejection's actual cause (a custom aggregate call, a custom scalar-function call,
  or a `SERVICE` clause) instead of funnelling every rejection through one message that named only
  the nondeterministic-builtin/blank-minting causes, which none of the three later-added causes
  appears in.
- **xsd,sparql-eval:** Integer `AVG`'s arbitrary-precision quotient is now taken on the same
  arbitrary-precision path its sum already uses, so an average that used to answer nothing whenever
  the exact running total escaped `i128` — even when the quotient itself was ordinary — now
  answers exactly (for example, `AVG` of two copies of the largest representable integer now
  answers that integer, rather than unbound).
- **sparql-eval:** `SERVICE` bodies forward custom scalar-function calls again unless `SILENT` is
  present. A prior fix closed a real silent-wrong-answer hazard for `Function::Custom` calls
  inside `SERVICE`, but the refusal it added was unconditional rather than scoped to the
  hazard it described — so a plain (non-`SILENT`) `SERVICE` body containing a call to the
  endpoint's own extension function (e.g. `FILTER(<http://example.org/fn>(?x) > 0)`) now
  hard-failed even though a non-silent `SERVICE` already turns any endpoint-side failure
  into an honest error. Callers who worked around the regression by dropping `SILENT` need
  no further change; callers who still need `SILENT` and hit the refusal should read the
  error message, which now names the same workaround (drop `SILENT`) directly.
- **sparql-eval:** The `SERVICE`-forwarding `LATERAL` guard is rebuilt to close a bypass and
  correct an over-broad refusal. The bypass: the guard's `Lateral` arm recursed into a
  variable-endpoint `SERVICE ?g { … }` auto-wrap's `left` operand only, never its wrapped
  `Service::inner` — so a written `LATERAL` nested inside `SERVICE ?g { … }` (itself nested
  inside a forwarded body) reached a remote endpoint's text unrefused. The guard now walks the
  full forwardable body, including every `Service::inner` (fixed-IRI or variable-endpoint), so a
  written `LATERAL` is found no matter how deeply it is nested. The over-broad refusal: a
  `LATERAL` clause inside a forwarded body was refused unconditionally, even though the hazard —
  a `SILENT` clause swallowing the endpoint's rejection into the join identity, a result that
  looks complete and is wrong — exists only under `SERVICE SILENT`. The refusal is now scoped to
  `SILENT`, naming it as the reason; a plain, non-silent `SERVICE` body forwards its `LATERAL {
  … }` text and surfaces the endpoint's actual verdict — an answer from a `LATERAL`-capable
  endpoint (`LATERAL` is Apache Jena's own extension) or an honest `EvalError::Remote` from one
  that rejects it. Callers who need `LATERAL` federated to a `LATERAL`-capable endpoint should
  drop `SILENT`; callers relying on the previous unconditional refusal under `SILENT` see no
  change, since `SILENT` still refuses.
- **sparql:** Let aggregate partial states merge by their concrete type (see the `into_any`
  entry above for the resulting trait break).
- **BREAKING** **sparql-eval:** The three extension registries (custom aggregates, property
  functions, SHACL-AF functions) move from `Option<&Registry>` to plain `&Registry`, with a
  canonical `Registry::EMPTY` constant replacing the `None` case. `QueryOptions`'s three registry
  fields and `PlanCache::prepare_with_relations`'s registry parameters change type accordingly
  across the core engine, all four host surfaces (CLI, C ABI, wasm, Python), the shapes crate, and
  the conformance harness; callers pass `&Registry::EMPTY` where they previously passed `None`.
  Decimal division's zero-divisor guard was corrected in the same change: it now returns the
  crate's typed error in release builds instead of a debug-only assertion that could panic.
- **BREAKING** **results:** The JSON/XML results writers now spell RDF 1.2 base direction as
  `its:dir` (were `dir`/`purrdf:dir` under a namespace this crate minted for itself), and the
  always-on purrdf-branded provenance extension is gone: `to_json`/`to_xml`/`serialize` take an
  `Option<&ProvenanceNamespace>`, and emission requires a caller-supplied prefix and IRI —
  omitting one emits no extension member and reports the drop, matching the CSV/TSV contract.
  `ProvenanceNamespace` separately moves to private fields and a fallible constructor,
  `ProvenanceNamespace::new(prefix, iri)`: the prior public fields let an unvalidated `prefix`
  splice unescaped into XML element/attribute names (a markup-injection hole), so `prefix` is now
  validated as an XML Namespaces `NCName` and `iri` as an absolute IRI. Callers constructing
  `ProvenanceNamespace` via struct literal must switch to the constructor and handle the `Result`.
  The TSV writer now hard-fails (`Error::Format`) on a variable name containing a tab or line
  break instead of silently corrupting the column/record structure.
- **BREAKING** **results:** The XML writer now declares the `its:` namespace once on the document
  root (with an `its:version` attribute) when any directional literal appears anywhere in the
  result set, instead of inline on every directional literal, matching the spec's worked examples;
  the JSON writer no longer emits an explicit `"datatype"` member on a plain (simple) literal, per
  the results-format's own encoding table. Both are byte-level output changes: a consumer pinning
  writer bytes must re-pin.
- **BREAKING** **sparql-results:** `ProvenanceNamespace` gained the validated, fallible
  constructor described above; unvalidated construction is no longer possible (see the results
  writer-spelling entry).
- **BREAKING** **rdf-core:** Replay delta-added quads in insertion order, not hash order. The
  copy-on-write delta layer kept its added/suppressed keys in standard hash sets (a fresh random
  seed per process), so freezing and pattern scanning walked them in per-process hash-iteration
  order rather than a reproducible one — the same query over the same data mutated the same way
  could return rows in a different order from one run to the next, which broke the documented
  ordering guarantee for `GROUP_CONCAT` and the first/last aggregates. Those sets now use the
  crate's fixed-key hasher and carry each key's insertion ordinal, so a delta replays in call
  order. The CLI was never affected (it parses straight into a builder and never uses the delta
  layer); the Python, WebAssembly, and C interfaces all wrap the delta layer directly and were all
  fixed by this one kernel change. Callers that happened to rely on the previous arbitrary order
  may observe a different, now-stable order.
- **BREAKING** **xsd:** `parse_duration` now enforces the `yearMonthDuration`/`dayTimeDuration`
  subtype pattern facets (XSD 1.1 Part 2 §3.4.26/§3.4.27) at parse time instead of accepting any
  lexical form its caller's declared tag claims: a `D`/`T` component under a `yearMonthDuration`
  tag, or a `Y`/`M` date component under a `dayTimeDuration` tag, is now a typed `InvalidLexical`
  rather than a silently accepted value that violated its own declared subtype. A caller lexical
  that relied on this laxity (e.g. `"P1D"^^xsd:yearMonthDuration`) now fails to parse instead of
  succeeding with a value its own datatype's pattern facet forbids.
- **BREAKING** **xsd:** A `Duration` whose months and seconds components carry opposing signs
  (e.g. `+12` months against `-1` day) is no longer constructible — XSD 1.1 Part 2 §3.3.6 puts
  mixed-sign values outside the lexical mapping's range, so there is no correct string to emit for
  one. The guard now sits in the single smart constructor every construction site — parsing and
  arithmetic alike — routes through, so it cannot be reached through one door (subtraction) while
  missed through another (addition of an already-negated operand). A caller combining a
  `yearMonthDuration` and a `dayTimeDuration` of opposing sign now gets a typed `OutOfRange`
  instead of a value that could never round-trip through its own canonical lexical form.
- **BREAKING** **xsd:** `duration =` (`op:duration-equal`) is now total, matching F&O: two
  durations with equal months and equal seconds compare equal regardless of declared subtype,
  where a cross-subtype comparison previously fell through to an incomparable/type-mismatched
  result — the reading `<`/`>` correctly keeps, since F&O defines ordering only for the
  `yearMonthDuration`/`dayTimeDuration` subtypes, not for the general type. A caller that treated
  duration equality as raising an error must now expect a plain `bool`.
- **xsd:** Two further `Duration` defects, both reachable only through extreme inputs, are closed
  alongside the arithmetic surface below: month accumulation during duration parsing used
  unchecked `i64` arithmetic (`"P9223372036854775807Y"` silently wrapped) and now reports a typed
  `OutOfRange`; and a zero-valued `yearMonthDuration`'s canonical lexical form is now the
  subtype-correct `"P0M"` rather than `"PT0S"`.
- **xsd:** An unreachable branch in exact decimal division — a scale-down path that could fire
  only if the crate-wide `scale <= 18` invariant were already broken, and whose own comment
  incorrectly called that "unusual but possible" — is now a build-surviving `unreachable!()` in
  place of a debug-only assertion, so a future violation of that invariant cannot ship a silent
  truncation in a release build. `xsd:duration ÷ xsd:duration` still reports a typed `OutOfRange`
  for the case that actually is reachable: scaling one operand's mantissa past `i128::MAX`.
- **BREAKING** **sparql-algebra,rdf:** The serializer rendered a join's right operand bare
  whenever it began with its own bare left, so a re-parse of the emitted text re-associated an
  `OPTIONAL`/`MINUS`/`FILTER`/`BIND`/`LATERAL` right operand into a semantically different tree —
  and this text is the `SERVICE` federation wire format, so a remote could receive a different
  query than the plan it was chosen for. Every re-absorbable right operand is now braced, decided
  by an exhaustive predicate; a plain join's right operand stays bare, since join associativity
  makes the re-association semantics-preserving. A variable-endpoint `SERVICE` under `LATERAL` is
  no longer double-wrapped in the keyword its own re-parse would re-wrap. The corpus round-trip
  sweep that caught this also surfaced and fixed five further serializer defects, none of them
  `LATERAL`-dependent: a multi-condition `HAVING` chain silently dropped its aggregate
  reconstruction, a projection-less aggregate emitted an illegal `SELECT *` over `GROUP BY`, a
  `FILTER` flattened as a left operand lost its group, property paths failed to parenthesize by
  precedence (alternation under sequence), and a trailing `VALUES` failed to absorb into its body
  group. A caller comparing serialized query bytes against a previous release should expect these
  five constructs, and the right-operand-braced ones above, to serialize differently — and
  correctly.
- **BREAKING** **sparql-eval:** Correlated evaluation (`LATERAL`, and `EXISTS`/`NOT EXISTS`
  correlated through an expression) substituted outer bindings by rewriting terms in place, which
  could not place a literal or blank-node binding into a triple position, silently skipped path
  and predicate positions, flipped `MINUS` into its disjoint-domain case by erasing the shared
  variables that make the two sides comparable, and crossed sub-select projection boundaries —
  correlating a variable the SEP-0006 scope example says is explicitly NOT correlated.
  Substitution now joins each pattern leaf with a one-row `VALUES` table carrying that leaf's own
  bindings, narrowed at every projection boundary to the variables it actually projects; an
  expression position keeps direct value substitution for an IRI or literal binding, and a
  blank-node or quoted-triple binding referenced ONLY in an expression position (no leaf
  occurrence for the leaf join above to carry it) is now carried the same way — by joining the
  expression's own owning pattern node against a one-row `VALUES` table — since no SPARQL
  expression syntax can spell either term kind as a rewritten constant; `BOUND` in particular now
  answers correctly for a bound variable of ANY term kind rather than only IRI/literal ones. This
  is a strictly corrective behavior change: `MINUS` inside a correlated `LATERAL`/`EXISTS` now
  answers correctly instead of a domain-flipped wrong answer, a `LATERAL`-bearing pattern is now
  refused rather than silently forwarded to a `SERVICE` endpoint (a remote rejecting the extension
  under `SILENT` would otherwise have contributed the identity table as a silent wrong answer),
  and a blank-node or quoted-triple outer binding reachable ONLY through an expression inside a
  `LATERAL` right-hand side no longer silently drops the row.
- **BREAKING** **sparql-algebra:** SEP-0007 Part 3's assignment restriction is now enforced at
  parse time: a `BIND`/a sub-`SELECT`'s `(expr AS ?v)` projection target/a `GROUP BY (expr AS ?v)`
  grouping target, or a `VALUES` column, inside an `EXISTS`/`NOT EXISTS` body — both polarities
  share the same grammar production, so both are covered identically, including a nested
  `EXISTS`'s own body checked against its immediately enclosing `EXISTS`'s scope — that rebinds a
  variable already in scope on the row being filtered is now a typed `ParseError` naming the
  variable and the introducing construct, instead of being silently accepted and evaluated. A
  rebinding confined to a `MINUS` right operand inside the body stays legal at any depth, since
  such an introduction never escapes it to become observable. A caller whose query relied on the
  previously accepted (and ambiguous) rebinding must rename the colliding variable.
- **BREAKING** **sparql-eval:** Three `EXISTS`/`NOT EXISTS` answer defects are corrected. A
  `GRAPH ?g { … }` body correlated through the row being filtered left the graph name unresolved
  against that row — the substitution walk skipped it because only one of its two callers ran the
  compatibility merge the name needed — so an existence filter could accept rows bound to a graph
  that does not actually hold the pattern; the name now resolves to the row's bound IRI for
  indexed selection. A correlation reaching a nested `EXISTS`'s body only through a triple position
  (never that inner's own expression positions) went undetected by the variable walk feeding the
  correlation decision, so that inner ran unconstrained instead of per-row; the walk now agrees
  with the one substitution itself uses. And a bare `OPTIONAL` at the top of a correlated body was
  evaluated as though its right side and join condition mattered to the existence test, when
  `OPTIONAL` pads every left row unconditionally and never removes one — `FILTER EXISTS { OPTIONAL
  { P } }` is always `true` (and its `NOT EXISTS` twin always `false`) regardless of whether `P`
  matches, which Existential Normal Form's spine-top `LeftJoin` erasure now decides directly. A
  query that depended on any of the three previous wrong answers now gets the correct one.

### Features
- **shapes:** New `ValidationReport::to_dataset()` returns the report graph as a frozen
  `Arc<RdfDataset>` — the report's PRIMAL RDF form. Rendering a report in any syntax other
  than N-Triples previously forced a `to_ntriples()` → `parse_dataset()` round-trip; that
  parse was pure waste, because the report was already being materialized as IR quads and
  the N-Triples text was only ever a serialization of them. `to_ntriples()` is now defined
  as "serialize `to_dataset()`", so the two are the same graph by construction rather than
  by coincidence, and the direct path carries every RDF 1.2 term the report holds (a
  triple-term focus node or `sh:value`, and blank-node labels the text grammar can only
  carry escaped) instead of whatever survives a text grammar and a parser's relabelling.
  Equivalence is proven canonically (RDFC-1.0) over a report spanning all four severity
  kinds, IRI/blank/triple-term focus nodes, a complex `sh:path` shared by two results, and
  typed/language-tagged/blank/triple-term values. Rust surface only: no Python, JS/wasm, or
  C ABI equivalent is added.
- **cli:** `purrdf validate --format <rdf-syntax>` no longer serializes the report to
  N-Triples and re-parses that text; it hands the report's own dataset to the shared sink.
  Identical output, one fewer full parse of the report per invocation.
- **cdt:** New crate `purrdf-cdt` implementing SEP-0009 SPARQL Composite Datatypes
  (`cdt:List` / `cdt:Map`) as a closed leaf over `purrdf-iri` + `purrdf-xsd` only. The
  function library is the spec's fifteen — `cdt:List`, `concat`, `contains`, `get`, `head`,
  `tail`, `reverse`, `size`, `subseq`, `cdt:Map`, `containsKey`, `keys`, `merge`, `put`,
  `remove` — and the set is CLOSED: there is no registry to configure and no way for a
  caller to shadow a spec function, so the same query means the same thing on every host.
  Nesting is fully ITERATIVE under three bounds (depth 64, 2²⁰ elements, 64 MiB); a
  million-deep input exits cleanly rather than overflowing the stack, which would abort the
  process rather than raise. PurRDF mints no vocabulary here: the SEP-0009 namespace is the
  spec's own fixed string, recognized and never invented.
- **sparql:** Evaluate SEP-0009 end to end. The fifteen functions are recognized at parse
  time (with arity enforced there) and dispatched by the evaluator; `FOLD` rides the
  aggregate ACCUMULATOR seam as a keyword aggregate carrying its own `ORDER BY`; `UNFOLD` is
  its own `GraphPatternNotTriples` alternative over an expression — structurally where
  `LATERAL` sits — rather than a property function, because the property-function registry
  keys on a predicate IRI and no `UNFOLD` predicate IRI exists to key on. `ORDER BY`, `MIN`
  and `MAX` order composite literals by the value they denote. Blank-node labels written
  inside a composite lexical form bind through the SAME ingress rule as bare `_:` tokens, on
  every codec path, and participate in canonicalization and skolemization — so `_:b` written
  as a subject and `_:b` written inside a `cdt:List` in the same document are one node.
- **BREAKING** **sparql-algebra:** `GraphPattern` gains an `Unfold` variant and
  `Function` gains a `Cdt` variant. Both enums are matched exhaustively across the
  workspace; a downstream consumer matching on either without a wildcard arm must add the
  new arm. There is no Cargo feature to opt out — CDT is unconditional, like every other
  part of the engine.
- **conformance:** The vendored SEP-0009 suite (`vectors/sparql-cdt/`, `awslabs/SPARQL-CDTs`
  at commit `e0a7465`, 658 cases across six groups) is run and reports its own scoreboard
  row: **658 / 658, 0 ledgered**. Note the divergence that number cannot express, stated in
  full in [`docs/CONFORMANCE.md`](docs/CONFORMANCE.md): PurRDF's reader accepts two element
  forms the published lexical space does not — an RDF 1.2 triple term and a directional
  language-tagged literal — because refusing an RDF 1.2 term type is not an admissible
  outcome for this toolkit. **A conformant SEP-0009 reader handed one of those literals will
  call it ill-formed.** The mitigation is executed rather than argued: a scan grades every
  composite literal in every corpus this workspace ships and proves not one of them needs
  either form, with its counts pinned as equalities so it cannot pass vacuously.
- **BREAKING** **sparql-eval:** Generalize the `SERVICE` seam into a per-service-context
  `ServiceResolver`. The trait `RemoteQuerySource` is renamed `ServiceResolver` and its method
  is renamed `resolve`, taking the whole request as one `ServiceRequest` value (endpoint,
  forwarded query text, the `SILENT` flag, stop signal, intermediate-cell ceiling) instead of
  four positional arguments; `LocalRemoteQuerySource` is renamed `InProcessServiceResolver` and
  moves to the new `purrdf_sparql_eval::service` module (re-exported at the crate root).
  Implementors must rename the trait and method and destructure `ServiceRequest`; callers must
  rename the two types. `HttpRequest` gains a `headers` field carrying the per-service headers
  and credential — a transport built with struct-literal syntax must add it, and a transport
  that receives but ignores it will issue an *unauthenticated* request for a service configured
  as credentialed, whose rejection `SERVICE SILENT` is entitled to swallow. `HttpTransport`
  itself is unchanged, and a source with no catalog sends the same bytes it always did.
- **sparql-eval:** Add per-service policy for `SERVICE` federation: a `ServiceCatalog` maps a
  service IRI to a `ServiceProfile` carrying extra headers, a redacting `ServiceCredential`
  (bearer / RFC 7617 basic / arbitrary header), timeout and User-Agent overrides, and an
  explicit `ServiceCapabilities` grant set (`Query`, `Network`, `Credentials`). Context lives on
  the resolver keyed by endpoint, never in the service IRI — which would put credentials into
  the query text, plans, and receipts. Catalogs deny by default and are opt-in: a resolver with
  no catalog behaves exactly as before, and gating a service adds no header its profile does not
  carry. Withholding `Network` makes an in-process façade provable rather than promised —
  `InProcessServiceResolver` holds a dataset map and no transport of any kind — and the new
  `ServiceRouter` composes in-process and network resolvers with the routing table, not the
  query text, deciding which answers what. The catalog is consulted on every resolution,
  including one nested inside a forwarded body. The `SILENT` contract is now stated in full:
  `SILENT` swallows an unreachable or undecodable endpoint to the join identity, and never
  swallows a capability denial or a governor trip, both of which are decisions taken on this
  side of the seam. There is deliberately no knob softening that — a host wanting a blocked
  service to read as unreachable returns a transport error from its own resolver. The
  denial holds at every nesting depth: `EvalError` gains a structured `ServiceDenied`
  variant (the enum is `#[non_exhaustive]`, so this is additive) so that a denial raised by
  a `SERVICE` nested inside a forwarded body is re-raised as a denial rather than decaying
  into a silenceable endpoint failure that an enclosing `SERVICE SILENT` would swallow to
  the join identity. A credential is also validated when it is attached rather than when it
  is rendered — CR/LF/NUL in a bearer token or an arbitrary credential header is refused at
  configuration time, with the credential withheld from the message, while a Basic password
  may still contain any byte because it is base64-encoded before it reaches the wire.
- **sparql-eval:** Add path-WITNESS property functions, which answer the derivation question the
  core grammar's property paths cannot: `?s ex:p+ ?o` reports that some route exists and binds only
  the endpoint pair, while a call to `?start <caller-iri> ( ?end ?pathId ?len ?step ?node ?edge )`
  binds the route itself — one row per hop, with `?edge` the traversed STATEMENT as a first-class
  RDF 1.2 term that joins straight back into the dataset by an ordinary basic graph pattern.
  `GROUP BY ?pathId` with `ORDER BY ?step` reassembles a whole walk inside the query language, so a
  caller can weight, filter, or re-join a route without host code and without a list term. Two
  relations, not one with a mode flag, because the planner reads cardinality off the registration:
  `PathWitnessRelation` enumerates every simple-prefix walk (exponential in the worst case) and
  `ShortestPathWitnessRelation` yields one shortest witness per reachable pair (polynomial).
  Enumeration terminates structurally on cyclic input, and its endpoint projection equals `p+`.
- **cli:** Add `purrdf query --path-relation` and `purrdf update --path-relation`, the binary's
  first property-function registration surface — before it, `QueryOptions::property_functions`
  stayed empty on every call. Repeatable; the value is semicolon-separated `key=value` pairs
  (`iri`, repeatable `forward`/`inverse`, `min-hops`, `max-hops`, `max-paths-per-seed`,
  `max-expansions`, `mode=walk|shortest`). Every key is mandatory and none has a default: PurRDF
  mints no vocabulary IRIs, so the relation IRI is caller-supplied with no default namespace, and
  a traversal envelope the binary invented would be a limit the operator never read. Each
  malformed spelling names the offending token. The flag reaches the ungoverned, governed,
  `--explain`, and `--entailment` lanes of `query` and the `WHERE` clause of `update`; the
  `--explain` receipt's `relations` block now names what was registered.
- **python:** Add the `path_relations` keyword beside `relations` / `relations_from_graph` on
  every `Store` and `MutableDataset` query and update entry, registering a path-witness relation
  over the store's own edges as `{iri: (steps, min_hops, max_hops, max_paths_per_seed,
  max_expansions_per_invocation, mode)}`. It crosses the boundary as pure data — a specification
  of which directed predicates a hop may follow, never a Python callable — so the evaluation still
  runs with the GIL released. Every envelope field is mandatory; an unknown direction or mode
  string, an empty or duplicated step alternation, a non-IRI predicate, and an unbuildable
  envelope each raise `ValueError` carrying the engine's own diagnostic.
- **xsd:** Implement the XPath Functions & Operators section 9 temporal operation table:
  timezone adjustment for `dateTime`/`date`/`time`, `yearMonthDuration`/`dayTimeDuration`
  arithmetic with month-end clamping, instant subtraction, and duration add/subtract/
  multiply/divide computed in exact `Decimal`. The existing partial order is unchanged; parsing
  and one canonical form are, per the `xsd` Bug Fixes entries above — subtype pattern facets are
  now enforced at parse time, and a zero-valued `yearMonthDuration` now canonicalizes to `"P0M"`
  instead of `"PT0S"`.
- **sparql:** Add the `ADJUST` builtin over `dateTime`, `date`, and `time` (SEP-0002's
  two-argument signature over the new F&O adjust-to-timezone family): shifts a timezoned value,
  annotates an untimezoned one, and treats the empty simple literal as timezone removal.
- **BREAKING** **sparql:** Retain the `VERSION` declaration as a typed value. `Query` and
  `Update` gain a `version` field (`SparqlVersion`) on every variant, exposed via a new
  `version()` accessor beside `dataset()`/`base_iri()`; construction sites using struct-literal
  syntax without struct-update (`..`) must add the field. Evaluation admits only `"1.2"` and
  `"1.2-basic"`; anything else is refused at the evaluation chokepoint before any work is spent,
  naming the declared string, while parsing itself stays syntax-only.
- **BREAKING** **sparql:** Re-found the aggregate algebra on the specification's own shape:
  `AggregateExpression` is a struct (`function`, `args`, `scalarvals`, `distinct`) rather than a
  lossy simplification, `CountStar` is gone (`COUNT(*)` is the empty argument list), and
  `GroupConcat` carries no separator payload (the separator is the `"separator"` scalarval). See
  the matching `sparql-algebra` bug-fix entry above for the resulting constructor break.
- **sparql:** Evaluate custom aggregates through a fold-algebra registry: a caller registers an
  `init`/`step`/`combine`/`finish` accumulator under an IRI, reached from query text as
  `AGG(<iri>, [DISTINCT] arg, arg, …)`; an unregistered IRI or wrong-arity call is refused at
  prepare time, before any budget is spent, and the prepared plan carries the registry's
  fingerprint (see the registry-instance-identity bug-fix entry above).
- **BREAKING** **sparql:** Price the aggregate fold in the governor profile. Profile v6 adds two
  charge points — `aggregate-invocation` (once per group per aggregate expression) and
  `aggregate-accumulation` (once per value inspected) — shared by built-in and custom aggregates.
  `GOVERNOR_PROFILE_VERSION` is 6 and the profile/corpus digests a consumer pins have moved.
- **BREAKING** **sparql-eval,xsd:** Charge the aggregate fold's retained per-group row buffer to
  the scratch-bytes governor as it is buffered, on both the built-in and registered-aggregate
  paths; that real, group-size-proportional memory was previously unpriced by any resource
  dimension. Only the scratch-bytes figures move — fuel is byte-identical, so
  `GOVERNOR_PROFILE_VERSION` and its digest correctly stay put — but a governed query near a
  scratch-bytes ceiling may now be refused where it previously completed, and a consumer pinning
  the frozen governor corpus's byte-frozen expectations must re-pin against the new figures.
- **sparql:** Fold single large groups in parallel: rows fold in chunks whose partial states
  combine strictly in chunk order, byte-identical to the sequential fold for every algebraic
  class. `GROUP_CONCAT`'s row order is now pinned by exact strings (see the SPARQL querying book
  chapter's "GROUP_CONCAT ordering" section for the guaranteed reading), and a blank node or
  triple term in its input now poisons the fold to unbound — the same reading `SUM`/`AVG` already
  use for a non-numeric running total — where it was previously silently dropped. This is a
  behavior change to `GROUP_CONCAT` over non-literal input, not an API break: a query that used to
  get a partial concatenation silently omitting a blank-node/triple-term value now gets unbound.
- **sparql:** Ship a statistical aggregate set under caller configuration: ten exact statistical
  aggregates (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`, `VAR_POP`, `MODE`,
  `FIRST`, `LAST`, `TOPK`) register as fold instances under a namespace the caller supplies (no
  default), reached one call away via `AggregateRegistry::register_statistical_aggregates`.
- **BREAKING** **sparql:** Thread an aggregate namespace through every host surface — a keyword
  argument on the Python query/update entry points, a flag on the CLI query/update subcommands, a
  parameter on the governed wasm entry points, and a nullable string on the governed C ABI entry
  points — so the statistical aggregate set is reachable from every binding, not only the embedded
  Rust engine. The C ABI's version moves `0.3.0` -> `0.4.0` to reflect the additive surface, and
  its header is regenerated; at this point in the series, the entailment and explain CLI lanes,
  and the ungoverned/entailment wasm calls, refuse the parameter outright since they structurally
  cannot honour it — closed by the entry below, which carries the C ABI to `0.6.0`.
- **BREAKING** **sparql-eval,purrdf:** The entailment-aware query lanes
  (`purrdf::query_with_entailment`/`query_with_entailment_governed`) hardcoded empty query
  options, so no scalar-function, property-function, or aggregate registry could reach a query
  evaluated over an entailed closure — `AGG(<{NS}MEDIAN>, …)` over an inferred closure was simply
  refused. Both lanes now take a `QueryOptions` and thread it through closure parsing, the
  witness-restriction rewrite, and evaluation. `QueryExplanation` gains an `aggregates()` accessor
  and a rendered block naming each resolved aggregate with its arity, volatility, algebraic class,
  state bound, and scalar parameters, mirroring how resolved relations are already reported. The
  CLI's prior refusal of `--aggregate-namespace` beside `--entailment` is removed. This closes the
  gap at the Rust engine layer; the C ABI, wasm, and Python bindings did not yet expose the
  parameter on their entailment-aware entries until the entry below.
- **sparql,capi,wasm:** Accept the aggregate namespace on the entailment-aware governed query lane
  everywhere it previously took no `QueryOptions` seam at all: the C interface, the WebAssembly
  binding, and the Python binding gain the same namespace parameter their ordinary governed
  entries already had, so `AGG(<{NS}MEDIAN>, ?x)` resolves over an entailed closure exactly as it
  resolves over a raw view. The C ABI's version moves `0.4.0` -> `0.6.0` (`0.5.0` is skipped —
  one commit carries both the host-surface change and the version bump) and its header is
  regenerated.
- **BREAKING** **capi:** `purrdf_serialize` gains `out_directional_literals_dropped` and
  `out_named_graph_rows_dropped` beside the existing `out_statement_rows_dropped`, so the whole
  realized loss of one serialization is partitioned by cause and no row is charged twice: a
  star-capable single-graph target (Turtle, N-Triples) previously reported `0` while silently
  discarding every named graph it was handed. Each count stays independently nullable. This
  transcode lane flattens and counts rather than refusing the way the query lane does, so a
  dataset whose every row is graph-scoped serializes to a single-graph syntax as a well-formed
  EMPTY document with status `OK` and the whole loss in `out_named_graph_rows_dropped` — pinned
  from C in the smoke test. The C ABI's version moves `0.6.0` -> `0.7.0` and its header is
  regenerated; the minor number tracks the exported signatures, so an additive parameter bumps it
  too, and every C caller must widen the call.
- **BREAKING** **sparql-eval,cli:** Give the engine one options-carrying explain entry,
  `NativeSparqlEngine::explain_query_with_options`/`_view`, that carries the SHACL-AF function,
  property-function, and aggregate registries together, and remove the narrower single-registry
  explain and query entries (`explain_query_with_aggregates`, `explain_query_with_property_functions`,
  `query_with_property_functions`, `query_with_user_functions`, and their `_view` twins). A query
  needing a registered relation and a registered aggregate together previously got a receipt (or
  an answer) for a different, narrower query through a narrower entry — a silent wrong report
  rather than a refusal — because an entry that could not be handed every registry in scope has no
  registry-independent way to know a call it did not recognize was ever meant to resolve against
  one it was not given. A caller of any removed entry now calls
  `explain_query_with_options`/`_view` or `query_with_options_view` with the same registry set on
  `QueryOptions`; the CLI's `--explain`, its ordinary query/update lanes, and every other in-tree
  caller already went through the options-carrying entries and need no change. `--aggregate-namespace`
  with `--entailment` or `--explain` is no longer refused (see the two entries above); `--explain`
  with `--aggregate-namespace` now prints the plan with the registered aggregates named in the
  receipt's `aggregates` block.
- **sparql-conformance:** Respell the conformance harness's manifest-extension predicate namespace
  under `example.org`; it was minted under this suite's own project-branded namespace, the same
  mistake a prior change fixed for the SPARQL-XML results writer. A new hygiene test
  (`no_purrdf_dev_iri_is_minted_outside_the_closed_exemption_list`) fails on any occurrence of
  that branded namespace found in the tracked tree outside a closed, stale-checked allowlist of
  two reader-tolerance fixtures. A sibling manifest fixture proves the guard's negative case is
  triggered by the missing description specifically, not by some other structural defect.
- **BREAKING** **sparql:** Accept named scalar-value parameters on aggregate calls: `AGG(<iri>,
  …)` now admits trailing `; NAME=value` clauses, generalizing `GROUP_CONCAT`'s own
  `; SEPARATOR="…"` clause to any custom aggregate (e.g. `AGG(<{NS}PERCENTILE>, ?x; P=0.95)`).
  `CustomAggregate::init` now takes the call site's resolved named scalar-value parameters
  (`&[(String, TermValue)]`) alongside `self`; every implementor of the trait must add the
  parameter (an aggregate declaring no scalar parameters via the new default
  `CustomAggregate::scalarvals` may ignore it). The first-party `PERCENTILE` and `TOPK`
  aggregates now take their fraction/bound through the named clause instead of as trailing
  positional arguments; callers passing them positionally must move them to named form.
- **sparql-algebra:** `parse_literal` — the grammar behind an `AGG(<iri>, …; NAME=value)`
  scalarval, a `VALUES` ground term, and a triple pattern's literal object — now accepts the
  full SPARQL literal grammar it always claimed to: the signed halves of the numeric tower
  (`Q=-1`, `P=+0.5`) and the boolean literals (`B=true`), not only unsigned numerals and
  strings. `VALUES ?x { -1 true }` and `?s :p -3` parse for the same reason: both routed
  through the same unsigned-only `parse_literal`, so the gap was one production, not one call
  site.
- **cli:** `purrdf query --explain` refuses `--results-format`, `--loss-ledger`, and
  `--jsonld-options` by name instead of accepting and silently ignoring them — `--explain`
  returns before any serializer or loss-ledger surface runs, so a named `--results-format`
  never selected a serialization, a named `--loss-ledger` never had a transcode to report, and
  a configured `--jsonld-options` document never reached a serializer. `--results-format` is
  now `Option<QueryFormat>` (defaulting to `json` only when the flag is genuinely absent) so
  "not named" and "named `json`" are distinguishable, which is what the refusal needs.
- **cli:** `purrdf convert --canonical` refuses `--to` by name instead of silently overriding
  it — canonical output is always RDFC-1.0 N-Quads, so a `--to` naming a different target
  format was accepted and never read. `--to` may still be omitted under `--canonical`, exactly
  as before.
- **BREAKING** **xsd:** Add a value-space arithmetic operator surface — `value_add`/`value_sub`/
  `value_mul`/`value_div`/`value_unary_minus` — dispatching, in one call, over the full numeric
  tower plus `xsd:dateTime`/`xsd:date`/`xsd:time`/`xsd:duration` (and its two subtypes) and the
  five Gregorian partial-date types (`xsd:gYearMonth`/`xsd:gYear`/`xsd:gMonth`/`xsd:gMonthDay`/
  `xsd:gDay`) ± duration arithmetic, covering SEP-0002's full temporal operator table plus
  Gregorian ± duration, which has no SEP-0002 row of its own but matches the permissive reading
  of another SPARQL engine's own duration handling everywhere the answer does not depend on a
  fabricated calendar field — for `xsd:gMonthDay` specifically, "does not depend on a fabricated
  field" is decided by anchoring the complete months-then-days computation at every year in one
  full 400-year Gregorian period (the calendar's leap rule is exactly periodic at that length) and
  requiring every anchor to agree on the finished `(month, day)`, not on either component alone
  (the book's SPARQL querying chapter carries the full table and that divergence's record).
  `numeric_add`/`numeric_sub`/`numeric_mul`/`numeric_div` remain public but
  are now documented as the narrower numeric-tier operators the value-space surface delegates to
  for numeric operands, not as the SPARQL-facing entry point — a caller that depended on them
  accepting a temporal operand and returning a type error there should switch to the `value_*`
  surface, which accepts it and returns a value.
- **BREAKING** **sparql-eval:** `+`, `-`, `*`, `/`, and unary `-` in `FILTER`/`BIND` now evaluate
  over `xsd:dateTime`/`xsd:date`/`xsd:time`/`xsd:duration` (and its subtypes) and the five
  Gregorian partial-date types through the new value-space operator surface, where they previously
  fell through a numeric-only dispatch and folded every such operand pair to unbound. A query that
  relied on a temporal arithmetic expression silently answering unbound now receives the computed
  value instead; the result's datatype tag follows the operands' own declared tags (documented in
  the book's SPARQL querying chapter), never the computed component values.
- **sparql-eval:** `SUM`/`AVG` accept a group of `xsd:duration` values (any subtype) alongside
  their existing numeric acceptance, summing componentwise in the duration group and rounding
  `AVG`'s months component to the nearest whole month (ties toward positive infinity) — a PurRDF
  extension, since SPARQL 1.1 §18.5.1.3 defines `SUM` over the numeric tower only. A group mixing
  numeric and duration values still folds to unbound; this widens acceptance, it does not narrow
  the existing numeric one.
- **xsd:** Add unary minus on `xsd:duration` — `-(?duration)` negates both components together,
  so it can never produce the mixed-sign value the type cannot represent — a PurRDF extension,
  since F&O's unary minus (§4.2.8) is numeric-only and defines no duration form. Unary plus
  deliberately stays numeric-only, so `+(?duration)` remains a type error while `-(?duration)`
  is not.
- **xsd:** `xsd:duration ÷ xsd:duration` now also accepts the general `xsd:duration` type on
  either or both sides, not only the matching-subtype pairs F&O defines
  (`op:divide-yearMonthDuration-by-yearMonthDuration`,
  `op:divide-dayTimeDuration-by-dayTimeDuration`), dispatching on the operands' VALUE
  commensurability — both purely months, or both purely seconds — rather than their declared
  tags, so a `dayTimeDuration` and a general `xsd:duration` that happens to be purely day-shaped
  still divide. Two values whose components are not commensurable, even under matching declared
  tags, are a typed error rather than an arbitrary answer.
- **sparql-algebra:** Add `LATERAL { GroupGraphPattern }` surface syntax (SEP-0006, implemented in
  Apache Jena 4.7.0 — the SPARQL 1.2 Query specification's own text defines no `LATERAL`
  production). The parser enforces the SEP's scope restriction: no variable introduced by a
  `BIND`, a sub-`SELECT` projection expression, a `GROUP BY` aggregate output, or `VALUES` at the
  right-hand side's own scope level may collide with a variable already visible on the left;
  correlated USE of a left-hand variable, and a sub-`SELECT`'s legitimate shadowing, both stay
  legal. `LATERAL` is also now legal (and scope-checked) inside `INSERT`/`DELETE … WHERE` and
  `WITH … WHERE`, while a `DELETE WHERE` quad template refuses it by name instead of misparsing it
  as a subject term.
- **BREAKING** **sparql-eval:** `EXISTS`/`NOT EXISTS` now rests on one stated semantics — SEP-0007's
  `Replace`/`PrjMap` substitution (`exists(X, μ) ⟺ eval(D(G), Replace(PrjMap(X), μ))` is non-empty)
  — served by exactly two implementations chosen through a proven boundary instead of two
  heuristics with a guessed one: a memoized existence probe (evaluate the inner once, index it,
  existence-probe each row) where a prepare-time admissibility proof shows it equivalent to
  per-row substitution for every row the site can see, and the per-row definition itself (backed
  by a restriction-keyed memo and a first-witness `Slice{0, Some(1)}` stop) everywhere else.
  `--explain`'s per-algebra-node charge ledger now reports the chosen strategy and its cost
  through three new evidence counters, `exists-probe-answered`, `exists-definition-answered`, and
  `exists-inner-solutions-consumed`. `GOVERNOR_PROFILE_VERSION` moves `6` -> `7` to carry the
  three new charge points, and the frozen governor corpus is regenerated; a consumer pinning
  `GOVERNOR_PROFILE_VERSION`, `GOVERNOR_PROFILE_DIGEST`, or `GOVERNOR_CORPUS_DIGEST` must re-pin
  all three. A query without a correlated `EXISTS`/`NOT EXISTS` charges none of the three new
  points, so its fuel is unchanged in value even though the schedule and its digest moved.

### Performance
- **core:** The interner no longer mints a `String` for the datatype IRI of every untyped or
  language-tagged literal; `rdf:reifies` is interned once per builder rather than by string on
  every reifier push. RDFC-1.0 canonicalization renders blank labels and predicate IRIs by
  borrow inside the n-degree search, and issues ids with one tree descent instead of three.
  `MutableDataset::freeze` memoizes each base term's builder id (dense table, one slot per
  base id) instead of rebuilding an owned value for every one of a quad's four term
  occurrences; `quads_for_pattern` on the mutable view no longer materializes a base-sized key
  vector. No output, ordering or hashing changes.
- **iri, xsd:** RFC 3986 dot-segment removal walks a borrowed cursor (zero allocations per
  resolve, was O(segments) buffer rewrites); `XsdDatatype::from_iri` compares the namespace once;
  the `whiteSpace` facets copy clean runs in bulk; `Decimal::canonical_lexical` builds its
  output in one buffer. Each rewrite is pinned byte-for-byte against its previous
  implementation by a test.
- **sparql-eval:** `OPTIONAL { … FILTER }` is hash-indexed like the unfiltered join whenever the
  right side is fully bound on the shared columns (the same candidate rows in the same order,
  so results, padding and governor charges are unchanged); a `MINUS` whose arms share no
  variable returns the left bag by move instead of scanning every pair; `EXISTS` no longer
  rebuilds a bound-variable set per outer row; `LANGMATCHES` compares bytes without lowercasing
  three strings per row; `^path` shares its inner memo instead of deep-cloning the reach set;
  `CONSTRUCT` reuses its per-row blank-label map.
- **sparql-algebra:** The GROUP BY / HAVING / ORDER BY continuation checks and language-direction
  split no longer allocate a case-folded copy per token; ASCII lookaheads in the lexer compare
  bytes instead of decoding a `char`.
- **shapes:** `sh:nodeKind`, `sh:minLength`, `sh:maxLength` and `sh:languageIn` read the value
  node's kind, length and language from the dataset arena and materialize an owned term only
  for a violation; `sh:closed` probes borrowed predicate names.
- **rdf:** Quoted-triple terms resolve their reifier binding through a one-per-document index
  (the table scan made star-heavy documents quadratic in their statement layer, including inside
  the canonical sort's comparators); the serializer memoizes term ids so a repeated predicate
  never rebuilds its value; the RDF/XML and TriX escapers make a single pass and borrow
  untriggered input; JSON-LD expansion borrows the parent context when an object declares no
  `@context`. Every emitted byte is unchanged.
- **gts, sparql-results:** The canonical GTS writer caches its CBOR sort keys once per row
  instead of re-encoding per comparison and computes the MMR root from peaks alone (no boxed
  tree); the SRJ/SRX escapers and the SRJ string reader copy plain runs in bulk. Frozen vectors
  are byte-identical.
- **sparql-algebra:** Parsing no longer re-walks the whole accumulated pattern for every
  `BIND`/`SELECT *`/`LATERAL` scope check. The group-pattern loop maintains its in-scope variable
  set incrementally (an ordered set, not a hash, to stay a zero-dependency wasm-clean leaf),
  removing a quadratic parse-time cost on adversarial input; debug builds assert the incremental
  set against a fresh walk at every consultation.

### Refactor
- **sparql-eval:** Drive built-in aggregates through the same `AggregateAccumulator` fold trait
  as caller-registered ones — each built-in is now a concrete accumulator monomorphized through
  one generic driver, rather than a hand-rolled counter plus a per-function enum. No behavior
  change; governor charging and the frozen corpus are untouched.

### Documentation
- **sparql:** Document the aggregate seam and the SPARQL 1.2 remainder: the book's query chapter
  gains `ADJUST`, the `VERSION` declaration, the custom-aggregate seam with a worked registration
  example, and the ten statistical aggregates; the results chapter documents the `its:dir`
  spelling and the caller-named provenance extension.
- **sparql:** Correct the book's query chapter, which still described the entailment-aware query
  lane as refusing `aggregate_namespace` on every host — a paragraph an earlier fix removed from
  the Python stub and one book location but left standing in a second. It now shows the
  combination working, with a worked `--entailment`/`--aggregate-namespace` CLI example.
- **sparql:** Document `LATERAL` (SEP-0006) in the book's query chapter: the production, a
  top-1-per-group worked example, the scope restriction with the SEP's own legal/illegal pair,
  the two deliberate divergences from Jena, the `UPDATE WHERE` status, and the `SERVICE`-forwarding
  refusal. The front-end surface enumeration now names it as a SEP extension.
- **sparql:** Document `EXISTS`/`NOT EXISTS` under SEP-0007 in the book's query chapter: the
  precise points SEP-0007 repairs against SPARQL 1.1/1.2 §18.6's literal `substitute`/`evalExists`
  reading (variable-only positions, the `MINUS` domain flip, blank nodes as variables, disconnected
  variables, and the Part 3 assignment restriction), the one `Replace`/`PrjMap` definition,
  Existential Normal Form's rewrite laws, the two evaluation strategies and their `--explain`
  evidence counters, the performance characteristic naming the shapes the memoized probe cannot
  serve, and the Part 3 restriction with the SEP's own legal/illegal example pair. The front-end
  surface enumeration's prior "EXISTS decorrelation" claim — never accurate, since a correlated
  filter is answered either by proof-admitted probing or by genuine per-row substitution, never by
  a blanket decorrelation — is corrected to point at this section. The README's shipped-surface
  bullet and Direction list move `EXISTS`'s SEP-0007 semantics out of "near-term direction" and
  into what SPARQL 1.1/1.2 already ships.

- **conformance:** The embedding kNN lane (PURREMB nearest-neighbour retrieval) and the
  `purrdf-geo` GeoSPARQL 1.1 lane now have conformance-matrix rows, ratchet budgets and
  per-engine scoreboard entries. Both shipped with no matrix representation at all — the only
  trace of either was a forward reference inside the governor row — so the umbrella gate could
  not see a regression in either lane. The kNN row counts test functions rather than fixtures,
  because it grades a seam rather than a document format and has no corpus; the document says so
  rather than inventing a fixture count.

  The GeoSPARQL row counts **corpus geometries**, and the reason is recorded because the first
  version of it did not. Written as a `cargo test` tally over `purrdf-geo`'s five integration
  binaries it measured **33 on one machine and 37 on the CI runner from byte-identical source**,
  turning the doc drift-guard red for a reason unrelated to GeoSPARQL. A number that moves with
  the build environment is not a measurement — the same principle the matrix's `_no_scoreboard`
  path already states from the other direction — so the row now reports the 20 geometries of
  `purrdf_geo::determinism::CORPUS` whose serialized bytes fold into one `u64`, compared against
  the `GOLDEN_DIGEST` pinned in the test source. That comparison is an oracle rather than a
  self-report, and it is stable everywhere. The row additionally records what it does **not**
  measure: **no OGC conformance suite is vendored and none is claimed**, the crate's SHACL shapes
  being first-party `example.org` mirrors of the shipped OGC 22-047r1 validator (PurRDF mints no
  vocabulary IRIs), so the lane has an independent oracle for its determinism but none for its
  semantics, and a misreading of OGC 22-047r1 would pass.
- **release:** Correct `docs/RELEASE.md`'s outstanding-bootstrap section, which named **four**
  crates as having no crates.io record. `purrdf-datalog` has had one since 2026-07-31 and
  answers `0.12.0`; the genuinely unpublished set is `purrdf-cdt`, `purrdf-text` and
  `purrdf-geo`, and the heading, the body, the publish-order ordinals and the in-page anchor
  all said otherwise. That set is now `PURRDF_UNBOOTSTRAPPED_CRATES` in
  `scripts/release-crates.sh`, a ledger `scripts/check-crates-io-records.sh` holds to the
  registry in **both** directions — an unlisted missing crate and a listed present crate each
  fail the preflight on their own — and the prose restates the ledger under a gate. The
  bootstrap examples drop their pinned version literal in favour of the argumentless form,
  which reads the workspace version from `cargo metadata` and cannot rot. The `make doc` / CI
  "N publishable crates" comments are corrected 20 → 21.
- **BREAKING** **release:** Both publish lanes — `scripts/bootstrap-crates-io.sh` and the
  tag-driven `release-cargo.yaml` loop — now **verify** every `cargo publish` (cargo builds the
  packaged crate against the registry before the upload that cannot be undone), and
  `purrdf-geo` moves from 13th to 17th in the publish order to make that possible. The loop's `--no-verify` was load-bearing, not incidental: `purrdf-geo`
  dev-depends on `purrdf-rdf` and `purrdf-shapes`, verification resolves the packaged crate's
  whole graph including dev-dependencies, and while `purrdf-geo` was ordered before both, a
  verifying publish of the set would have failed at crate 13 after twelve irreversible
  uploads. Moving one crate removes the last forward dev-edge. The new
  `scripts/check-publish-order.py` proves on every `make check` that the order is a
  topological order of normal **and** dev-dependencies, that the release set is exactly the
  publishable members, and that the bootstrap ledger is in-set and in order; its
  `--self-test` perturbs each check and requires the refusal, and the release workflow runs it
  again in its verify step at the point of no return. `PUBLISH_NO_VERIFY=true` restores
  the old behaviour for one run. Anyone with a checked-out publish order, a pinned release
  script, or a Trusted Publisher configured by ordinal must re-read
  `scripts/release-crates.sh`.
- **release:** `PUBLISH_COOLDOWN_SECONDS` defaults to `0` (was `620`). crates.io's new-crate
  rate limit is enforced at the publish: a limited `cargo publish` exits non-zero, `set -e`
  stops the run before the next crate, and a re-run resumes because published versions are
  skipped — a visible, resumable refusal, not a corrupted release. The old default modelled the
  limit's ten-minute refill unconditionally and added about half an hour of dead time to a
  three-record run. The environment override is kept.

### Testing
- **results:** Pin its:dir precedence over legacy spellings in SRX
- **sparql,conformance:** Pin the newly-added SPARQL evaluation surface in the conformance
  corpus: the `VERSION` declaration evaluated (not merely parsed), the `AGG` call form's
  grammar, and `GROUP_CONCAT`'s row-order concatenation pinned to an exact string.
- **sparql-algebra:** Pin the whole serializer with a corpus round-trip sweep: parse, serialize,
  re-parse, and compare (modulo left-linearized join spines) over every vendored W3C, first-party,
  and doc-example query text — the empty exception ledger is the point; every disagreement it
  found is fixed above, not ledgered.
- **rdf:** Reduce the golden-capture deferred-construct classifier to the engine's actual typed
  residue — `LATERAL`, `SERVICE`, `DESCRIBE`, and property paths no longer misroute into expected
  deferral now that they are implemented.
- **sparql-conformance:** Add the `purrdf-extend` `LATERAL` manifest cases: the SEP-0006 worked
  examples (including the scoping oracle, proving Project-boundary narrowing rather than mere
  textual substitution), a shared-variable-injection case, and the SEP's own legal/illegal syntax
  pair.
- **sparql-conformance:** Add eight `purrdf-extend` SEP-0007 `EXISTS`/`NOT EXISTS` manifest cases
  through the shipped stack — the correlated graph-variable body, the `OPTIONAL`-padding tautology
  in both polarities, nested negation, a per-row `LIMIT 1` sub-select a one-shot probe would
  truncate wrongly, the `MINUS` shape whose right-operand correlation a one-shot probe would flip,
  and the Part 3 assignment restriction's own colliding-`BIND`/colliding-`VALUES` negative-syntax
  pair — bringing the suite to fifty-five cases (forty-eight evaluation, five negative-syntax).
  Each expected evaluation result was hand-derived from the `Replace`/`PrjMap` substitution
  definition before being pinned against the release binary, so a still-wrong engine could not
  have pinned itself correct.
- **sparql-eval:** Pin the probe/definition strategy boundary from both sides with a test-only
  forced-strategy seam: agreement tests run every admissible shape through both strategies and
  assert row-for-row equality, and divergence witnesses force the probe onto each refused shape
  and assert the specific wrong answer it would give. A bounded-exhaustive generator sweeps
  hundreds of inner shapes at depth two, checking memo equivalence throughout and cross-strategy
  agreement on every admitted one. Twenty-four `FILTER EXISTS`/`FILTER NOT EXISTS` shapes also run
  as real query text through the public engine end to end, including the substitution document's
  own worked examples, every solution modifier inside the body, the `HAVING`-position scope pin,
  and quoted-triple/blank-node outer bindings.

- **conformance:** Bring `scripts/conformance-baseline.json`'s free-text `note:` prose under the
  same gate as its `ledgered` integer. Only the integer was ever machine-checked, and the OWL 2
  DL note rotted a full generation behind it — claiming 261 vendored cases, 30 non-terminating
  and 12 withheld exclusions against a measured 262, 0 and 25, with every gate green the whole
  time. `scripts/check-doc-claims.py` gains `baseline_note_claim`, which checks that the
  baseline's suite names and the generated matrix block's suite names are the same set, that
  every `ledgered` budget equals its matrix row's XFail/Skip column, and that every
  matrix-derivable integer any note restates equals the column that measures it — with the DL
  note's subset/exclusion tally sourced from the same frozen `census.tsv` the three
  `docs/CONFORMANCE.md` restatements already are. A reworded note that stops matching its
  pattern fails as loudly as a wrong number, so the gate cannot be silenced by rewriting prose.
- **release:** `scripts/check-doc-claims.py` gains `outstanding_bootstrap_claim` and
  `publishable_crate_count_claim`, holding `docs/RELEASE.md`'s bootstrap heading, body crate
  count, per-crate publish-order ordinals and in-page anchor — and the `Makefile` / CI
  publishable-crate counts — to `scripts/release-crates.sh`. Membership is deliberately left to
  `scripts/check-crates-io-records.sh`, since whether a crate record exists is a fact about
  crates.io rather than about this tree.

## [0.12.0] - 2026-08-02

### Bug Fixes

- **BREAKING** **canon:** Reserve the overlay's namespace by refusal, not by assertion

### Features

- **BREAKING** Let a new enum variant stop being a breaking change
- **errors:** Let the standard chain reach the failure underneath
- **canon:** Name and version the canonicalization profile, with a frozen vector corpus

### Performance

- **datalog:** Keep the join's binding frame off the heap
- **sparql-eval:** Reject a candidate before paying to copy the row
- **rdf:** Write the text serializers into one buffer instead of many
- **rdf:** Write TriG in place, blocks and all
- **rdf:** Sort the canonical order through a comparator, not through keys
- **rdf:** Cut allocations on the hot paths; fix(canon)!: collision-safe RDF 1.2 canonicalization profile

### Refactor

- **BREAKING** **rdf:** The codec's serializer takes the caller's buffer

### Testing

- **datalog:** Give the semi-naive join a bench of its own
- **conformance:** Put the canonicalization profile on the scoreboard

## [0.11.0] - 2026-08-02

### Bug Fixes

- **entail:** Make the survey's three-way early exit reachable
- **docs:** Hold the xfail sentence to the count the matrix generates
- **wasm:** Assert an error names a term, do not compile the term into a pattern
- **BREAKING** **entail:** `?name` in any position, including the one it was refused in
- **entail:** A boundary that meant two things, and two claims about rule heads that were false
- **python:** Give each newline one way to match, not two
- **wasm:** Assert a refusal names a term exactly, by neither of the two wrong ways
- **wasm:** Pin the whole refusal, so the IRI reaches no matcher at all
- **BREAKING** **entail:** A variable is not a datatype IRI, and a withheld predicate is not a smaller question
- **BREAKING** **entail:** One IRI is one answer, and one variable name is one variable
- **gates:** Two gates that could not fail, and two guards that could not see
- **docs:** Derive which documents the overclaim ban sweeps, from the ban itself
- **docs:** Make the derived sweep total, rather than saying it is
- **docs:** Let the gate's own guards be mutated, and refuse the mutations
- **docs:** Compose the ban patterns around their markers, rather than checking they contain one
- **BREAKING** **entail:** A deep input must be an error, not a dead process
- **BREAKING** **rdf:** Bound term nesting where it is parsed, and survey every position the merge writes
- **entail:** Five entailment-diagnostic fixes — a lossy diagnostic, a split precedence, and three claims nothing checked

### Documentation

- **entail:** The doc gate is not `make check`, and it found six broken links
- **rdf:** The XML-literal walk adds no bound; something in front of it does

### Features

- **entail:** A conclusion-directed entailment service over the RL chase
- **entail:** Close the negative-conclusion lane by refutation over the chase
- **entail:** Decide a schema axiom by freezing its body and chasing the head
- **entail:** Comprehend the anonymous class expressions a conclusion names
- **entail:** Read a reflexive property's self-loops off the conclusion, not the closure
- **BREAKING** **entail:** Decide an rdfs:range axiom by datatype containment
- **entail:** Vendor the document the last unreached premise names
- **BREAKING** **entail:** Return the run that answered, not the verdict alone
- **entail:** The conclusion-directed surface reaches all four hosts
- **conformance:** Print the split an empty ledger makes trivially true
- **BREAKING** **entail:** A conclusion graph is a conjunction, and entailment is monotone over one
- **BREAKING** **entail:** A lane not run is a limit, not a silence
- **BREAKING** **entail:** The import map is the caller's, on every host that has the service
- **cli:** The binary can ask the question, not only compute the closure
- **conformance:** Print what the twenty-three negative agreements are made of
- **BREAKING** **entail:** Close the 16 ledgered W3C OWL 2 RL entailment-corpus gaps

## [0.10.0] - 2026-07-31

### Bug Fixes

- **BREAKING** **datalog:** Carry the predicate as data so meta-rules are expressible
- **entail:** Stop fabricating rdfs:Resource for a derived triple term
- **entail:** Drop the tableau's unique name assumption for nominals
- Accept D from the CLI, close the umbrella gap, correct shipped strings
- **release,docs:** Refuse a half-publish, and make documented numbers checkable
- **entail:** Make the certificate real, and grade the rules against W3C
- **entail:** Make the overclaim state unrepresentable, and surface the certificate everywhere
- **entail:** Derive the DL certificate's completeness instead of storing it
- **wasm:** Reach every reasoner service from the npm package root
- **docs:** Unbreak the rustdoc gate and name the Python suite row for what it runs
- **bindings:** Make the shipped type stub match the extension, and gate what published numbers claim
- **docs:** Gate the numbers that recurred, and stop promising a component this workspace does not have
- Make three gates inspect what they claimed to, and cover a tableau clash nothing reached
- **docs:** Correct eight published figures and gate the surfaces that carried them
- **docs:** Close three ways this pass's own gates could be satisfied without checking anything
- **entail:** Refuse explain-conclusion per conclusion, not per regime
- **BREAKING** **docs:** Name the DL fragment SHOIQ(D), date the exclusion emitters, and correct three provenance claims
- **docs:** A concept ledger over repo-wide facts, and twenty corrected figures
- **BREAKING** **entail:** Refuse the unrepresentable cardinality, bound the counting search, and state what the blocking evidence shows
- **BREAKING** **entail:** Make the combined approach reachable, sound on every result form, and honest about its fragment
- **docs:** The figure sweep — thirty corrected claims, three gate holes closed
- **hygiene:** The issue-reference ban could not see string literals or SPARQL
- **playground:** The console offered seven codecs while the engine registers nine
- **entail:** The counting/inverse limit keyed on spelling, not on meaning
- **python:** `uv run mypy` checked nothing and exited on a usage error
- **wasm:** The session handle needs Debug and Self, as the lint table requires
- **hygiene:** The wasm export gate read the export block and not the imports
- **datalog:** Make the SLG budget bound the work, not just the output
- **datalog:** Keep freshen_clause's doc on freshen_clause, and collapse the filter's ifs
- **build:** Make the wasm artifact's size independent of who built it

### CI & Build

- **wasm:** Record the session's 8,882 bytes
- **wasm:** Raise the ceiling 25% and stop failing on the exact byte count
- **npm:** Raise the package ceilings 25% and stop failing on the exact byte count

### Documentation

- **entail:** Generate the rule inventory and correct every stale claim
- Fix two intra-doc links that failed the rustdoc gate
- **conformance:** Grade the rules against W3C, and gate every number
- Date the DL exclusion tally, which is a recorded measurement rather than a live one
- **provenance:** Record the cutover as it stands, the slme port, and the revision reachability
- Published text carried internal program codes
- The session's three Python tests and its bytes reach the recorded figures
- **conformance:** The prose scoreboard row lagged the generated block
- **provenance:** Say which generated projections this repo can actually regenerate
- Say why the backward resolver has no caller, and stop claiming it has one
- State the backward check's boundary as the cost it is, with numbers that reproduce
- **validate:** The skip test's own doc still told the story the code disproved

### Features

- **BREAKING** **datalog:** Add the purrdf-datalog crate and wire every release gate
- **datalog:** Port the physical primitives — branded ids, arena, bitset, binding patterns
- **datalog:** Port the relation store, cursors, and index-selection planner
- **datalog:** Port the semi-naive evaluator with analytic goldens and hard budgets
- **datalog:** Replace the provisional rule IR with the DL-clause IR
- **entail:** Add the machine-readable rule inventory
- **datalog:** Add checkable proof terms and a contract hash
- **BREAKING** **entail:** Return a reasoning certificate from every materialize call
- **validate:** Add the shared entailment-regime string boundary
- **entail:** Seed the finite axiomatic triples and add four RDFS rules
- **python:** Expose entailment regimes as purrdf.entail
- **entail:** Add RDF-list materialization and the prp, cax and scm rules
- **wasm,capi:** Expose entailment regimes to WebAssembly and the C ABI
- **entail:** Complete OWL 2 RL — all 78 rules, and make D materializable
- **entail:** Stop dropping OWL axioms silently, and add the existential chase
- **entail:** Expose the DL reasoner services behind a certified facade
- **entail:** Dataset semantics, reifier interactions, and explanations
- **entail:** Reach every reasoner service from every host
- **entail:** Close the last W3C entailment gap, wire the plan cache, report termination
- **entail:** Bind the extension inventory on every host and gate its disclosure
- **BREAKING** **entail:** Decide OWL 2 data ranges, and check the tableau against a model-enumeration oracle
- **xsd:** Decide the rational-decimal identity exactly, and write the gmeow cutover guide
- **BREAKING** **entail:** Certain answers by the combined approach, over a ported SLG-WFS resolver
- **BREAKING** **entail:** A clause-based hypertableau is the OWL-Direct decision core
- **entail:** The nominal/inverse/counting limit is a named boundary, not buried prose
- **validate:** A reasoning session, so asking twice costs one parse
- **python:** Expose the reasoning session as `entail.Reasoner`
- **wasm:** Expose the reasoning session as `Reasoner`
- **capi:** Expose the reasoning session as `PurrdfReasoner`
- **purrdf:** Surface the reasoning session on the Rust facade, and gate all four hosts
- **entail:** Cross-check every chase explanation against backward resolution
- **entail:** Re-derive every chase explanation backward, and report the outcome
- **BREAKING** **entail:** Complete the entailment surface — 78/78 OWL 2 RL, certified runs, four language hosts

### Other

- Revert "feat(entail): cross-check every chase explanation against backward resolution"

### Performance

- **BREAKING** **entail:** Classify by one saturation instead of a tableau run per class pair
- **datalog:** Reject impossible clause/call pairs before freshening them

### Refactor

- **BREAKING** **entail:** Run the declared clause program instead of a hand-written chase
- **entail:** Split the calculus into one module per rule family
- **BREAKING** **entail:** Make materialization total over every regime

### Testing

- **entail:** Capture the chase's behaviour as a golden oracle
- **conformance:** Vendor the W3C OWL 2 suite and give entailment its own row
- **entail:** Check the OWL 2 RL closure against an independent second implementation
- **entail:** Assert the tableau is not over-permissive where that is decidable
- **entail:** Pin the owlrl divergence triples, and cover the value class that separates two language-tagged values
- **entail:** Pin the divergence triple count so a regeneration cannot absorb a regression
- Carry the per-conclusion explain contract to the remaining two hosts

## [0.9.0] - 2026-07-28

### Bug Fixes

- **gts:** Use neutral fixtures, drop an unreachable guard, surface breaking changes

### CI & Build

- **changelog:** Mark a breaking release when the squash duplicates a subject

### Documentation

- **gts:** Name the real undicted plan in the frozen-vector test

### Features

- **BREAKING** **gts:** Pin caller-supplied in-band dictionary bytes in a compaction plan

### Testing

- **gts:** Say what the mixed-plan header assertion can actually observe

## [0.8.5] - 2026-07-26

### Bug Fixes

- **slice:** Scope ownership by declared term namespaces, not the framework ns
- **slice:** Never mistake a Turtle comment for the sliceDependsOn block
- **docs:** Repair public GTS links
- **gts:** Harden dictionary append invariants

### Documentation

- **gts:** Register zstd-rsyncable level?/dct? and the dict-vector corpus

### Features

- **gts:** Multi-dictionary packs, rsyncable dict priming, and a declared zstd level
- **gts:** Support multi-dictionary rsyncable packs

### Testing

- **gts:** Freeze dictionary vector fold oracles

## [0.8.3] - 2026-07-23

### Bug Fixes

- **rdf:** Make JSON-LD byte budgets target independent
- **rdf:** Harden JSON-LD size arithmetic
- **capi:** Refresh generated package version
- **rdf:** Complete JSON-LD portability audit
- **rdf:** Make JSON-LD byte budgets portable

### CI & Build

- **release:** Gate coordinated tags on full checks
- **release:** Validate every published surface

### Other

- **shapes:** Restore canonical formatting

### Testing

- **rdf:** Use inclusive multiplicity range

## [0.8.2] - 2026-07-22

### Bug Fixes

- **shapes:** Make SHACL-AF function scope re-entrancy-safe under parallel validation
- **shapes:** Disambiguate ontology classes named after reserved JSON-Schema $def keys
- **rdf:** Raise JSON-LD carrier/document row ceilings to 2^23 for large bundles
- **rdf:** Size the JSON-LD carrier envelope for a whole-ontology bundle
- **rdf:** Raise JSON-LD output/document byte ceilings for whole-ontology bundles

## [0.8.1] - 2026-07-21

### Bug Fixes

- **wasm:** Align npm package size budgets

### CI & Build

- **wasm:** Budget shared SHACL membership view

### Documentation

- **wasm:** Record optimized membership artifact
- **conformance:** Record subclass corpus case

### Features

- **shapes:** Add subclass membership view
- **shapes:** Unify subclass membership semantics

### Other

- Unify SHACL subclass membership across native and SPARQL validation

### Performance

- **shapes:** Benchmark subclass membership hot path
- **shapes:** Prune inactive membership rows

### Testing

- **shapes:** Freeze subclass membership semantics
- **shapes:** Freeze subclass corpus hashes

## [0.8.0] - 2026-07-20

### Bug Fixes

- Harden purremb contracts and lookups
- **shapes:** Standardize schema input blanks apart
- **shapes:** Return typed schema key errors
- **shapes:** Admit identifier-only ontology ranges
- **shapes:** Retain custom datatype ranges
- **shapes:** Bound propagated schema facts
- **shapes:** Lower unsafe LinkML slots deterministically
- **shapes:** Bound schema parsing during construction
- **shapes:** Reject initializer module components
- **jsonld:** Preserve active-context option semantics
- **jsonld:** Preserve compact carrier round trips
- **jsonld:** Bound derived context validation
- **cli:** Bound JSON-LD options input
- **python:** Isolate generated namespace bindings
- **playground:** Route configured JSON-LD formats
- **capi:** Update projection example scope
- **wasm:** Update projection scope fixtures
- **csvw:** Preserve W3C RDF conversion
- **capi:** Refresh generated projection header
- **build:** Measure OKF wasm package growth
- **capi:** Close smoke fixture on seek failure
- **build:** Make Cargo target fallback safe
- **capi:** Honor active smoke-test profile
- **build:** Harden Cargo target discovery
- **release:** Deduplicate generated changelog entries
- **capi:** Regenerate header for 0.8.0

### CI & Build

- **wasm:** Rebaseline scoped projection budgets

### Documentation

- **rdf-core:** Specify the PURREMB v1 format
- **shapes:** Expose ontology schema workflow
- **shapes:** Publish LinkML slot migration contract
- **shapes:** Document rich Pydantic package emission
- **shapes:** Qualify flat byte compatibility
- **jsonld:** Document deterministic context compaction
- **conformance:** Refresh compatibility count
- **conformance:** Refresh Python parity count
- **csvw:** Guide curated terms projection
- **cli:** Enumerate liftable profiles
- **wasm:** Refresh optimized size measurement
- **cli:** Explain OKF lift rejection
- **conformance:** Account for attached parity test

### Features

- **rdf-core:** Add deterministic PURREMB writing
- **rdf-core:** Add borrowed PURREMB reading
- **rdf-core:** Add PURREMB binding verification
- **rdf-core:** Add deterministic .purremb companion format
- **sssom:** Model set-level document comments
- **sssom:** Retain parsed document envelopes
- **sssom:** Serialize typed document envelopes
- **shapes:** Define ontology schema compilation contract
- **shapes:** Derive deterministic ontology schema surface
- **shapes:** Emit ontology-complete schema carriers
- **shapes:** Define LinkML slot naming contract
- **shapes:** Verify LinkML slot reports on import
- **shapes:** Define deterministic Pydantic package topology
- **shapes:** Emit routed rich Pydantic packages
- **rdf:** Compile JSON-LD active contexts
- **rdf:** Compact JSON-LD through a typed carrier
- **rdf:** Derive deterministic JSON-LD contexts
- **cli:** Expose configured JSON-LD serialization
- **bindings:** Expose compiled JSON-LD contexts
- **lpg:** Require explicit projection scope
- **lpg:** Stream projection artifacts
- **lpg:** Expose scoped streaming hosts
- **csvw:** Define curated terms profile
- **csvw:** Project scoped curated term tables
- **projection:** Expose curated CSVW terms
- **rdf:** Define OKF terms projection contract
- **rdf:** Generate deterministic OKF term bundles
- **rdf:** Expose OKF terms across hosts
- **rdf:** Add attached RO-Crate assets
- **bindings:** Expose attached RO-Crate packaging

### Other

- Compile ontology-complete developer schema surfaces
- Make LinkML slot lowering deterministic and reversible
- Emit deterministic rich Pydantic packages
- **jsonld:** Document scoped lint exceptions
- Add deterministic context compaction
- Preserve typed SSSOM set comments
- Add scoped streaming LPG projections
- Add caller-configured curated CSVW projections
- Add deterministic caller-configured OKF term bundles
- Add deterministic attached RO-Crate payload packages
- Add native DCAT and VoID dataset descriptions
- Benchmark whole-bundle SHACL focus execution
- Optimize SHACL focus validation invariants
- Parallelize SHACL focus evaluation deterministically
- Prepare bounded SHACL validation for realtime use
- Eliminate SHACL canonical sort key allocations
- Preserve interned IDs through recursive SHACL checks
- Prove deterministic SHACL parallel execution
- Document realtime SHACL validation operations
- Optimize realtime and whole-bundle SHACL validation

### Performance

- **rdf-core:** Benchmark PURREMB access paths
- **sssom:** Streamline column selection
- **shapes:** Cache ontology row sort keys
- **shapes:** Borrow unchanged LinkML slot locals
- **shapes:** Reuse Pydantic path buffers
- **shapes:** Avoid duplicate schema limit traversal
- **shapes:** Index routed definition owners
- **jsonld:** Remove carrier hot-path allocation
- **lpg:** Measure scoped streaming carriers
- **projection:** Coalesce artifact sink writes
- **csvw:** Reuse URI expansion allocations
- **csvw:** Remove curated selection temporaries
- **csvw:** Borrow curated table memberships
- **rdf:** Remove OKF classifier scratch allocations
- **rdf:** Remove attached crate loop allocations

### Refactor

- **shapes:** Clarify routed name guard
- **jsonld:** Unify context processing

### Testing

- **rdf-core:** Harden PURREMB conformance coverage
- **shapes:** Prove ontology surface across emitters
- **shapes:** Keep namespace fixture vocabulary neutral
- **shapes:** Prove emitter-specific ownership
- **shapes:** Harden LinkML slot lowering
- **shapes:** Exercise routed Pydantic packages
- **jsonld:** Freeze expanded codec baseline
- **rdf:** Pin OKF terms cross-host parity
- **rdf:** Freeze attached crate host parity

## [0.7.0] - 2026-07-17

### Benchmarks

- **core:** Measure wavelet indexes against pack FoQ
- **shapes:** Measure LinkML imports

### Bug Fixes

- **shapes:** Record losses for non-class shape targets instead of dropping silently
- **capi:** Regenerate the ABI header for the 0.6 version bump
- **rdf:** Skip triple-term self-reifier sentinels in the JSON-LD reifier index
- **rdf:** Graph-scope reifier/annotation identity end-to-end
- **rdf:** Record TriX/HexTuples base-direction drop in the loss ledger
- **cli:** Reach reason stdin/stdout via --from/--to + fail-fast format resolve
- **cli:** Treat stdout BrokenPipe as a clean exit, not a runtime error
- **wasm,playground:** Honest size re-baseline + JSON-LD bidirectional round-trip
- **rdf-core:** Keep page planning metadata-only
- **rdf-core:** Make paged view debug inert
- **columnar:** Support published loss ledger API
- **columnar:** Bound untrusted decode allocations
- **rdf:** Scan escaped OKF link destinations
- **rdf:** Normalize exponent-form OKF decimals
- **shapes:** Enforce constrained schema unions
- **shapes:** Align Pydantic loss contracts
- **shapes:** Enforce JSON carrier fidelity
- **shapes:** Harden TypeScript reference closure
- **shapes:** Close TypeScript loss-audit gaps
- **shapes:** Bound TypeScript alias-cycle scans
- **shapes:** Describe GraphQL key validation bidirectionally
- **capi:** Restrict projection output permissions
- **rdf:** Honor CSVW record dialects
- **rdf:** Validate CSVW BCP 47 tags

### CI & Build

- **wasm:** Raise size budget for packed-dataset restoration
- **wasm:** Raise npm tarball size ceiling for packed-dataset restoration

### Documentation

- **core:** Drop stale issue-number token from loss_matrix_json doc comment
- **core:** Unlink private registry_entries from public loss_matrix_json doc
- **rdf:** Unlink classify from the private FORMATS table for rustdoc
- **cli:** Add the purrdf CLI README; fix a private intra-doc link
- **query:** Define paged completeness contract
- **shapes:** Document Pydantic projection
- **shapes:** Document LinkML projection
- **shapes:** Define TypeScript projection contract
- **shapes:** Define the GraphQL carrier boundary
- **rdf:** Complete projection adoption surface
- **conformance:** Refresh projection parity count
- Refresh conformance parity count
- **shapes:** Complete schema reverse surface
- Keep issue tracking out of provenance

### Features

- **core:** Unify the runtime loss ledger and add an enumerable codec-pair registry
- **shapes:** Record schema-projection losses on the unified LossLedger
- **core:** Add reusable ledger soundness + completeness verification helpers
- **core:** Enumerate the codec-pair registry and pin the runtime-ledger schema
- **core:** Make loss_matrix_json the enumerable codec-pair registry
- **core:** Unified loss-ledger surface for all codecs
- **rdf:** Register JSON-LD/YAML-LD as first-class native format variants
- **rdf:** Lossless JSON-LD-star triple-term encoding + orphan-reifier fix
- **rdf:** Lossless nested triple terms + reject annotations inside a @triple
- **rdf:** Unify JSON-LD/YAML-LD into the native format registry
- **rdf:** Make the native serializer generic over DatasetView
- **rdf-core:** Public DatasetView-generic pack reconstructor
- **cli:** Add the purrdf CLI crate (convert/query/reason core)
- **cli:** Convert --base/--entailment/--canonical + full matrix tests
- **cli:** Query --base/--entailment + CONSTRUCT/DESCRIBE RDF sink
- **cli:** Reason --base + per-regime boundary diagnostics + tests
- **cli:** The purrdf CLI — convert / query / reason
- **rdf:** Expose packed dataset restoration
- **rdf-core:** Certify paged provider snapshots
- **rdf-core:** Add fallible paged query views
- **sparql:** Certify complete fallible query results
- Certify fallible paged SPARQL execution
- **columnar:** Define five-table Parquet contract
- **columnar:** Implement deterministic Parquet kernel
- **columnar:** Project RDF datasets to five tables
- **columnar:** Reconstruct RDF from five tables
- **columnar:** Add deterministic bidirectional Parquet codec
- **rdf:** Define OKF loss contracts
- **rdf:** Lift OKF bundles into event sinks
- **rdf:** Write deterministic OKF bundles
- **rdf:** Add native bidirectional OKF codec
- **core:** Register Pydantic projection losses
- **shapes:** Emit Pydantic v2 packages
- **core:** Register LinkML loss profile
- **shapes:** Add canonical LinkML codec
- **shapes:** Project schemas to LinkML
- **shapes:** Add canonical LinkML 1.11 projection
- **core:** Register TypeScript projection losses
- **shapes:** Emit TypeScript declarations
- **shapes:** Emit deterministic TypeScript declarations
- **core:** Register GraphQL loss profile
- **shapes:** Emit deterministic GraphQL SDL
- **rdf:** Add projection carrier foundations
- **rdf:** Add canonical LPG mapping
- **rdf:** Add LPG CSV adapters
- **rdf:** Add LPG graph carriers
- **rdf:** Add bidirectional CSVW projections
- **rdf:** Add OBO Graphs projection
- **rdf:** Add deterministic SKOS projection
- **projections:** Expose deterministic carrier surfaces
- **rdf:** Add graph and tabular projections
- **rdf:** Add research-object semantic pivot
- **rdf:** Add Croissant 1.1 codec
- **rdf:** Add RO-Crate 1.3 codec
- **rdf:** Add DataCite 4.6 codec
- **rdf:** Add DCAT 3 codec
- **rdf:** Add Frictionless Data Package codec
- **rdf:** Expose research-object carrier surfaces
- **rdf:** Complete research-object carrier integration
- **rdf:** Add bidirectional research-object codecs
- **core:** Register schema to SHACL loss profiles
- **shapes:** Import JSON Schema as SHACL
- **shapes:** Import LinkML as SHACL
- **shapes:** Import generated schema packages
- **shapes:** Import schemas as SHACL

### Other

- Expose packed dataset restoration
- Emit caller-configured Pydantic v2 packages

### Performance

- **core:** Memoize the codec-pair loss registry behind OnceLock
- **cli:** Mmap-borrow disk pack→pack passthrough instead of heap-buffering
- **shapes:** Avoid duplicate Pydantic schema work
- **shapes:** Avoid LinkML projection allocations
- **shapes:** Avoid TypeScript render copies
- **shapes:** Avoid redundant GraphQL oracle escaping
- **rdf:** Avoid clean JSON pointer allocation
- **rdf:** Avoid DataCite XML uppercase copy
- **shapes:** Reduce schema import allocations

### Refactor

- **core:** Remove the dead RdfLoss diagnostic type
- **core:** Derive loss-entry intentional from profile membership
- **rdf:** Single FormatDescriptor table as the format metadata source of truth
- **wasm:** Route the wasm format resolver through the one core registry
- **shapes:** Share compiled schema catalog

### Testing

- **rdf:** Assert bnode-scope-flatten is an in-profile loss
- **rdf:** Production-surface tests for JSON-LD/YAML-LD + named-graph reifier fix
- **rdf:** Pin YAML-LD adversarial-scalar literals against the Norway problem
- **wasm:** Assert isomorphism on the JSON-LD/YAML-LD round-trip + fix stale docs
- **cli:** Pin --loss-ledger surfacing tri-state + universal-sink invariant
- **cli,rdf:** Pin query --entailment exit-3 boundary + dedup pack test fixture
- **sparql:** Cover fallible query guarantees
- **columnar:** Prove backend and DuckDB interoperability
- **columnar:** Cover empty files in DuckDB oracle
- **shapes:** Execute Pydantic schema oracle
- **shapes:** Exercise recursive Pydantic refs
- **shapes:** Add official LinkML oracle
- **shapes:** Add TypeScript compiler oracle
- **shapes:** Verify GraphQL coercion with GraphQL.js
- **cli:** Tolerate early stdin closure

## [0.6.0] - 2026-07-14

### Bug Fixes

- **gts:** Keep original authorship signatures bound across repacks
- **rdf:** Fail closed instead of panicking when verifying poison packs
- **rdf:** Anchor compaction projection to the provenance predicate vocabulary
- **gts:** Make the packaging signature a required compaction parameter
- **paged:** Enforce G3 quad-disjointness across the side tables
- **core:** Unify the pack dictionary to one id per term value for DatasetView compatibility
- **core:** Reject a literal datatype id that does not reference an IRI in the pack decoder
- **core:** Fail closed in verify_pack when a reconstructed dataset is structurally invalid
- **core:** Fail closed on FoQ index count-sum overflow in the pack decoder
- **shapes:** Sh:in enum members match the instance projector encoding
- **sparql-eval:** Make the fork-join parallel-safety gate registry-aware
- **gts:** Hard-fail the event bridge on a dangling term reference
- **gts:** Fire StreamingSink::frame before a frame's rows

### Documentation

- **design:** Author the PurRDF backend contract (C-clauses + paged G-clauses)
- **paged:** Fix rustdoc intra-doc link errors denied by the doc gate
- **core:** Document the pack backend + add the pack_query criterion bench
- **gts:** Fix rustdoc private-intra-doc-link errors denied by the doc gate
- **shapes:** Document the value-vocabulary enum projection
- **gts,rdf:** Demote private intra-doc links to code spans for the doc gate
- **sparql-eval:** Fix private intra-doc link denied by the doc gate
- **sparql-eval:** Broaden user_fn module doc to both function kinds
- **gts,rdf:** Demote private intra-doc links denied by the workspace doc gate
- **sparql-eval:** Drop plan-id process-flow refs from native-fn tests
- **core:** Describe the pack module by behavior, dropping plan task refs
- **core:** Reword residual plan-task phrasing in pack module docs
- **gts:** Add the §7.7 streaming-fold cross-reference and repair the rustdoc doc gate
- **gts:** Correct GtsEventSink provenance-ordering doc after the frame fix
- **gts:** Drop public-to-private intra-doc links tripping the doc gate

### Features

- **gts:** Deterministic in-band pack dictionaries for the zstd dct codec
- **gts:** In-band zstd dct codec with finalized pack dictionaries
- **gts:** Train and pin an in-band pack dictionary in streamable compaction
- **gts:** Bind detached signatures under an MMR root with a packaging head sig
- **rdf:** CompactionCertificate and verify_compaction refold-equivalence API
- **rdf:** Witness the suppression-compaction commuting square on both digests
- **gts:** Compress compaction blobs against the pinned in-band dict
- **core:** Make DatasetView an unsealed, id-agnostic read seam
- **core:** Add GlobalTermId + GlobalDictionary u64 identity layer
- **core:** Add PagedDataset — id-agnostic demand-paged DatasetView over a u64 dictionary
- **sparql-eval:** Generify the evaluator over D: DatasetView
- **sparql-eval:** Make the binding layer id-generic over D::Id
- **core:** Add freeze-refusal + deterministic compaction to PagedDataset
- **paged:** Add from_parts warm-restart constructor for PagedDataset
- **core:** Id-agnostic DatasetView seam + paged u64 backend served by the evaluator
- **gts:** Certified signature-preserving compaction with in-band dict codec
- **core:** Succinct rank/select + bit-packed IntVector primitives for the pack codec
- **core:** Four-section PFC value dictionary for the pack codec
- **core:** Graph-partitioned succinct bitmap-triples with FoQ all-pattern indexes
- **core:** RDF 1.2 reifier + annotation side-tables for the pack codec
- **core:** Deterministic pack container framing + zero-copy PackView reader
- **core:** Implement DatasetView for PackView (PackId, all-pattern query seam)
- **core:** Certified read-only projection — verify_pack recomputes the RDFC-1.0 digest
- **shapes:** Project value vocabularies to enum $defs (projection-only)
- **shapes:** Resolve sh:class / rdfs:range value-vocab refs to enum $defs
- **sparql-eval:** Native Rust-closure user-function registry + dispatch
- **sparql-eval:** Native Rust-closure user-function registry
- **gts:** Two-inventory replication diff, splice reconstruction, and diff_json
- **gts:** Surface per-frame provenance (content-id + byte range) on the streaming read path
- **gts:** Stream GTS frames into an RdfEventSink with per-frame provenance
- **gts:** Two-inventory diff/splice fetch-list + streaming RdfEventSink bridge on a shared decode core
- **core:** Read-only succinct HDTQ-style pack codec as a DatasetView backend

### Other

- **shapes:** Apply rustfmt to json_schema.rs
- **sparql-eval:** Rustfmt the native-scorer test helper

### Performance

- **paged:** Fold the G3 seal probe into a single map insert
- **paged:** Stream the paged read path instead of collecting a Vec
- **shapes:** Scan rdfs:range once per dataset for value-vocab $ref mapping
- **shapes:** Compute first_literal via single-pass running minimum
- **sparql-eval:** Lend native-fn args as &[&TermValue], drop per-call deep clone
- **gts:** Drop the streaming bridge's iri_map at each segment close
- **gts:** Hash the streaming decode maps with the fixed-key ahash policy

### Refactor

- **shapes:** Panic! directly for value-vocab enum-key twin guard
- **gts:** Hoist ByteRange to a shared model type and record FrameInventory.prev
- **gts:** Extract shared per-segment decode core and subsume the GTS import sink onto it
- **gts:** Bound the streaming decode core to one segment's memory

### Testing

- **gts:** Freeze in-band dict-compaction vectors with drift guards and docs
- **rdf:** Exercise the pack/tail seam chain across the boundary
- **gts:** Re-freeze the streamable-compacted vector to the current format
- **gts:** Assert the streamable-compacted pack pins no dct header entry
- **sparql-eval:** Prove SPARQL served directly over a multi-page PagedDataset
- **paged:** Cover property paths, aggregates, and subqueries on the paged backend
- **core:** Demonstrate mmap-able zero-copy PackView over a memory-mapped file
- **sparql-eval:** SPARQL-over-pack end-to-end parity with RdfDataset
- **shapes:** Prove value-vocab enum round-trip and open-validator decoupling
- **shapes:** Cover value-vocab class-key clash guard and multi-range tiebreak
- **sparql-eval:** Native scorer push-down + determinism (FILTER/ORDER BY/NaN/parallel)
- **gts:** Assert the streaming decode core's memory is bounded per segment
- **rdf:** Actually exercise GTS base-direction preservation on the sink path

## [0.5.0] - 2026-07-12

### Bug Fixes

- **shapes:** Key JSON-Schema $defs resolution set by def_key so cross-namespace local-name twins never dangle a $ref
- **wasm:** Free empty SELECT result handle without iteration
- **serialize:** Emit RDF 1.2 triple terms as non-asserting <<>>

### Documentation

- **conformance:** Record expanded Python parity suite
- **shapes:** Correct resolve_id doc to match variant-specific lookup

### Other

- Optimize Rust ownership and hot paths for 0.5.0

### Performance

- **core:** Remove owned lookup and freeze overhead
- **query:** Move parser tokens and stream results
- **reasoning:** Store terms once and index RIF joins
- **validation:** Cache sort keys and compile ShEx once
- **gts:** Stream deterministic CBOR without value clones
- **viz:** Linearize projection and ownership analysis
- **bindings:** Stream result ownership across FFI
- **reasoning:** Reuse RIF chase frontier index across fixpoint iterations
- **query:** Bound parser reparse fork to the braced block

### Testing

- **bench:** Cover SPARQL graph serialization and DISTINCT allocation paths
- **bench:** Cover SHACL canonical sort and ShEx prepared-shape paths

## [0.4.3] - 2026-07-10

### Bug Fixes

- **ci:** Serialize SPARQL conformance tallies

### Other

- Add semantic RDF 1.2 visualization exports

## [0.4.2] - 2026-07-10

### Other

- Harden npm wasm RDF 1.2 toolkit
- Expose entailed SPARQL and RIF parsing

## [0.4.1] - 2026-07-09

### Bug Fixes

- **shapes:** Project external object-class values to a node-ref, not a string, in JSON Schema
- **npm:** Align package-root RDFJS typings
- **capi:** Refresh generated ABI header
- **npm:** Accept null dataset inputs
- **npm:** Correct ecosystem probe evidence

### Documentation

- **npm:** Add ecosystem probe evidence

### Features

- **npm:** Add reusable SPARQL query engine

### Performance

- **wasm:** Benchmark query engine reuse

### Testing

- **npm:** Gate packed wasm package
- **npm:** Pin package gate toolchain

## [0.4.0] - 2026-07-07

### Bug Fixes

- **hygiene:** Exclude rustdoc inline-code spans from the issue-ref lint
- **makefile:** Use POSIX sed for wasm-bindgen pin extraction (macOS grep -oP)
- **makefile:** Use awk not tr to parse wc byte counts in wasm-pkg-size
- **hygiene:** Restrict issue-ref inline-code exclusion to Rust doc comments
- **playground:** Clear CodeQL alerts — structural entailment assertion + worker same-origin guard
- **shapes:** Polarity-sound sh:not projection in json_schema emitter
- **shapes:** Negate sh:not inner as a whole conjunction (De Morgan sound)
- **shapes:** Route sh:not maxCount property inner to a loss (no vacuous not)
- **shapes:** Route array-unsafe value-restriction sh:not inners to a loss
- **shapes:** Route existential sh:hasValue sh:not inner to a loss
- **shapes:** Restrict sh:not negand to exact-complement projections
- **shapes:** Polarity-sound sh:not projection (kill vacuous class negation)

### CI & Build

- **capi:** Gate the purrdf.h C-ABI header against drift
- **wasm:** Gate optimized artifact size via a pinned wasm-toolchain composite action
- **release:** Share the pinned wasm-toolchain action and enforce the size budget on release
- **wasm:** Drop unpinned twiggy source-build diagnostics step
- **docs:** Deploy the RDF-1.2 console at /playground in the Pages artifact

### Documentation

- Uplift product docs to top-tier Rust project standard
- **agents:** Document the wasm size-budget gate and deliberate-raise procedure
- Link the RDF-1.2 playground from the root and package READMEs
- **wasm:** List the shacl module in the lib.rs surface doc comment
- **shapes:** Strip issue-ref tokens from emitted schema descriptions
- Reconcile README and docs for 0.4.0

### Features

- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + regenerate canonically
- **capi:** Make purrdf.h reproducible via cargo-c `capi` marker + gate it in CI
- **wasm:** Add reproducible wasm-pkg-size budget gate (binaryen pinned)
- **wasm:** CI-gated wasm artifact size budget
- **wasm:** Expose SHACL + RDFC-1.0 canonicalize/isomorphic on the package surface
- **playground:** Standalone client-side RDF-1.2 console (engine in a Web Worker)
- **playground:** Drop the post-load wasm-size probe so the console makes zero network requests after assets load
- **playground:** Assert the SARIF 2.1.0 contract in the SHACL pane instead of echoing the engine version
- **playground:** Standalone deployed RDF-1.2 console over purrdf-wasm

### Other

- **shapes:** Satisfy fmt, clippy docs, and issue-ref hygiene gates

### Performance

- **shapes:** Cache the sort key for sh:not negand ordering

### Testing

- **shapes:** Add trusted external JSON-Schema validator harness
- **shapes:** Behavioral accept/reject tests for polarity-sound sh:not

## [0.3.3] - 2026-07-05

### Documentation

- **capi:** Regenerate purrdf.h with SHACL validate + entail declarations

### Performance

- **rdf:** Memoize the line index so parser diagnostics stay linear
- **rdf:** Memoize parser line index — fix quadratic diagnostics scan

## [0.3.2] - 2026-07-05

### Bug Fixes

- **shapes:** Reconcile SHACL-AF work with the merged 0.3.1 baseline

### Features

- **shapes:** SHACL Rules — 100% SHACL-AF coverage

## [0.3.1] - 2026-07-05

### Bug Fixes

- **shapes:** Pre-bind $shapesGraph in sh:SPARQLRule CONSTRUCT execution
- **build:** Optimize parse-hot workspace crates in dev/test profile to remove ~300x regression

### CI & Build

- **release:** Edition 2024, publish purrdf-entail, expose entail+validate, bump 0.3.1

### Documentation

- **conformance:** Add SHACL Rules scoreboard row; SHACL-AF is 100% complete
- **release:** Changelog for 0.3.1

### Features

- **shapes:** SHACL Rules engine — sh:TripleRule, sh:SPARQLRule, fixpoint entailment
- **shapes:** Cartesian-product multi-valued function-call node-expression args
- **bindings:** Expose SHACL rule entailment on Python, wasm, and C-API surfaces

### Performance

- **shapes:** Key the rules fixpoint divergence universe on Term, not String
- **shapes:** Reuse bindings buffer and hoist arg keys in function-call cartesian product

### Testing

- **shapes:** SHACL Rules conformance corpus + inferred-graph harness
- **shapes:** Audit every node-expression kind in sh:TripleRule subject/predicate/object positions
- **shapes:** Cover blank-focus blank minting and multi-round fixpoint convergence

## [0.3.0] - 2026-07-05

### Benchmarks

- **sparql-eval:** Isolate Solution row construction and join
- **rdf:** Add report-only span-tracking arm for the NoSpans zero-cost claim

### Bug Fixes

- **sparql-eval:** Correct correlated-EXISTS over address-keyed cache reuse
- **rdf:** Verify content-chain inclusion via blob/segment-head/MMR-leaf union
- **shex:** Route numeric_value through the XSD-1.0 float/double restriction
- **rdf:** Accept empty predicateObjectList item in Turtle parser
- **shex:** Fire group semantic actions only for participating groups
- **shex:** Make result-shape-map JSON fully round-trip through parse_shape_map
- **shex:** Distinguish unresolved IMPORT from conflicting redefinition
- **sparql-algebra:** Accept trailing top-level VALUES clause
- **sparql-conformance:** Dedupe named graphs by IRI + case-insensitive media-type
- **sparql-conformance:** Compare SELECT solutions up to W3C whole-set bnode isomorphism
- **sparql-eval:** XSD constructor casts emit canonical value forms
- **sparql-eval:** Correct built-in value spaces and XSD 1.1 canonical decimal
- **sparql-eval:** Scope UPDATE template blank nodes per request, not per operation
- **sparql-algebra:** Rewind trailing dot after blank-node label
- **sparql:** Exclude MINUS-right-only variables from in-scope set
- **shex:** Surface the concrete import parse error instead of swallowing it
- **shex:** Propagate concrete import cause through ImportResolver
- **conformance:** Make the byte-freeze manifest cross-platform deterministic
- **conformance:** Normalize CRLF in the matrix doc drift-check
- **conformance:** Honest matrix reporting for compile errors and first-party corpus
- **shacl-af:** Record sh:expression as a lossy JSON Schema projection
- **shacl-af:** SPARQL-value orderby, canonical set outputs, value-true is_true
- **shacl-af:** Keep sh:desc and described constant IRIs out of function-call parsing
- **conformance:** Repair --group dev gate, lock compat ratchet, refresh matrix
- **shacl-af:** Unbound mandatory sh:SPARQLFunction parameter yields no result
- **shacl-af:** Treat sh:returnType as informational, not an enforced datatype
- **shacl-af:** Reject empty or SHACL-reserved sh:SPARQLFunction parameter names
- **shacl-af:** Hard-fail malformed sh:order/sh:optional and multi-projection bodies
- **shacl-af:** Merge sh:SPARQLFunction body state back into the caller
- **shapes:** Repair broken merge — stray conflict markers and missed run_select rename
- **shapes:** Repair non-compiling SHACL-SPARQL component merge
- **shapes:** Make the validation-report sort total for byte determinism
- **entail:** Reject non-range-restricted RIF rules instead of panicking
- **entail:** Deterministic RDFS/OWL inferred-triple emission order
- **rdf:** Report located diagnostics at the offending token, not the next one
- **rdf:** Report Turtle/TriG located diagnostics at the offending token
- **rdf,validate:** Emit real byteOffset for N-Triples/N-Quads source spans
- **validate:** Wire SarifOptions::source_root_uri into the emitted SARIF
- **rdf:** Standardize blanks apart on native quad merge to stop cross-source collapse
- **release:** Stamp the pending version in the generated changelog
- **release:** Treat registry-restricted crates as publishable
- **release:** Anchor the package.json version bump to the top-level key
- **release:** Ignore commented-out crates in the publish-list parser
- **release:** Guard release-tags against a missing CHANGELOG section before tagging
- **release:** Assert per-crate version coherence in check-versions.py
- **hygiene:** Purge issue refs from python shim docstring and lint docstrings
- **rdf:** Pass the span collector in the empty-namespace turtle test
- **rdf-core:** Reject rdf:_0 and leading-zero container membership ordinals

### CI & Build

- **release-npm:** Guard binaryen --enable-simd + document SIMD baseline
- **wasm-pkg:** Hard-fail the build if the artifact carries no SIMD opcodes
- **wasm-pkg:** Append to RUSTFLAGS instead of overwriting it
- **release:** Enforce cross-registry version coherence and complete the publish list
- **release:** Pin the git-cliff version in the changelog target
- **release:** Make set-version.py rewrite every version location
- **release:** Gate internal dependency pins, at commit AND publish time

### Documentation

- **deps:** Correct memchr comment; drop issue refs from sparql-algebra
- **wasm-pkg:** Align the SIMD Node floor with the package engine (18)
- **wasm-pkg:** Describe the parse bench as report-only, not a regression gate
- **sparql-eval:** Correct fork_for_worker doc to the portable-row merge mechanism
- Describe behavior, not process, in content-addressing comments
- **rdf:** Document trust-on-first-use semantics of verify_content_chain
- **xsd:** Describe the i128/scale bound instead of a "deferred enhancement"
- Scrub GitHub issue-number references from shapes and sparql-eval comments
- **shex:** Clarify IMPORT conflict and inert-extension doctrine
- **conformance:** Reconcile rdflib LSP gate ledger scoreboard to live 62/24
- **iri:** Harden conformance-vector provenance for W3C IRI gate
- **conformance:** Finalize unified matrix at full-corpus SPARQL numbers
- **sparql-conformance:** Frame SPARQL 1.2 as a complete first-class spec
- **sparql-conformance:** Document that entailment simple1-8 are OWL-Direct, not simple-entailment
- **conformance:** Refresh SPARQL matrix counts to live harness (614 pass / 36 xfail)
- **conformance:** Correct SPARQL 1.2 provenance to zero ledgered residuals
- **sparql:** Finalize W3C SPARQL 1.1 syntax-suite provenance
- **conformance:** Reconcile the ledger and drift-guard the published matrix
- **conformance:** Distinguish normative SHACL-AF node expressions from owned extensions
- **conformance:** Correct stale SHACL first-party corpus count (64 to 69)
- **entail:** Fix misleading comment in the RIF emit path
- **validate:** Fix stale intra-doc link to a non-existent locate module
- **release:** Document MSRV and pre-1.0 semver policy
- **release:** Docs.rs metadata, front-page example, and workspace doc gate
- **ci:** Reconcile doc-target crate count to 16 (15 publishable + purrdf-entail)
- **rdf,shapes:** Fix private/broken intra-doc links failing the doc gate
- **conformance:** Regenerate matrix block for the added SPARQL fixtures
- **release:** Changelog for 0.3.0 and correct the published-crate count

### Features

- **rdf:** Memchr the parallel chunker newline split
- **rdf:** Scan-first serializer escape fast path
- **21:** Memchr/SWAR scan sweep — codec scans + IRI char-class LUT
- **sparql-eval:** Add rayon dep and deterministic two-phase parallel scaffold
- **rdf-core:** Add Blake3ContentId newtype with shared hex decode
- **rdf-core:** Add caller-supplied content-addressing config surface
- **rdf-core:** Recognize content-addressed IRIs at intern time
- **rdf-core:** Carry the content-id side table into the frozen dataset
- **rdf-core:** Add suppression-target and derivation-link traversal helpers
- **rdf-core:** Add a derived predecessor index over derivation annotations
- **rdf:** Add verify_content_chain GTS bridge over content-addressed terms
- **rdf-core:** Content-addressed term support in the IR (GTS-aligned)
- **xsd:** Add opt-in XSD-1.0 float/double lexical restriction
- **sparql-eval:** Pin xsd:float/double cast to XSD 1.0 lexicals
- **xsd:** Shared XSD-1.0 float/double lexical restriction
- **shex:** Resolve transitive cycle-tolerant IMPORT
- **shex:** Dispatch semantic actions via a Test extension registry
- **shex:** Query shape maps with FOCUS triple-pattern selectors
- **shex:** Serialize result shape maps to deterministic JSON
- **shex:** Populate SemActContext value and predicate per matched triple
- **shex:** Add validate_shape_map end-to-end entry point
- **sparql-conformance:** Harden conformance gate — no silent skips + license hygiene
- **sparql-conformance:** Support W3C UpdateEvaluationTest cases
- **sparql-eval:** LATERAL evaluation seam for variable-endpoint and nested SERVICE
- **entail:** Native wasm-clean RDFS + OWL-RL materialization reasoner
- **sparql-conformance:** Wire entailment regime into conformance harness
- **sparql:** Implement RDF-1.2 base-direction functions
- **sparql:** Parse RDF 1.2 triple terms, reifiers, and annotation blocks
- **sparql:** Complete RDF 1.2 triple-term/reifier support across parser, codec, and evaluator
- **sparql-eval:** Evaluate negated-inverse and set-repetition property paths
- **sparql:** Group-by projection check, EXISTS graph scope, GRAPH ?g over empty graphs
- **rdf:** Reifier-consistent CONSTRUCT/UPDATE emission and triple-term equality
- **rdf:** Give the RDF 1.2 reifier/annotation model a graph dimension
- **conformance:** Full W3C SPARQL 1.1/1.2 eval + native entailment + lateral SERVICE
- **shex:** Imports, Test-extension semantic actions, query shape maps
- **sparql:** Vendor W3C SPARQL 1.1 syntax-query suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax-update-1/2 suites
- **sparql:** Vendor W3C SPARQL 1.1 syntax-fed conformance suite
- **sparql:** Vendor W3C SPARQL 1.1 syntax suite as parser conformance fixtures
- **conformance:** Enforce a monotone-shrink ledger ratchet in the gate
- **conformance:** SHA-256 byte-freeze the vendored conformance corpora
- **conformance:** Monotone ledger ratchet, drift-proof published matrix, and byte-freeze verification for the SHACL/shexTest gates
- **shacl-af:** Add node-expression IR skeleton and AF vocabulary
- **shacl-af:** Parse node expressions from the shapes graph
- **shacl-af:** Wire sh:ExpressionConstraintComponent end-to-end
- **shacl-af:** Evaluate built-in function-call node expressions
- **shacl-af:** Evaluate aggregation, paging, and ordering node expressions
- **shacl-af:** Evaluate filterShape and exists with cycle-safe re-entry
- **shacl-af:** Wire vectors/shacl/af seam and refresh conformance matrix
- **shacl-af:** Authority-grounded sh:orderby with sort-key expr and sh:desc
- **shacl-af:** Dispatch XPath-namespace keyword builtins in function calls
- **python:** Complete rdflib plugin entry-point discovery and acceptance matrix
- **shacl-af:** Sh:expression node constraints + node-expression evaluator
- **sparql-eval:** Dynamic SHACL-AF SPARQL function registry seam
- **shapes:** Parse sh:SPARQLFunction declarations into a function registry
- **shapes:** Resolve sh:SPARQLFunction calls in validation, remove the stub
- **shapes:** Implement SHACL-SPARQL custom constraint components
- **shacl-sparql:** Pre-binding substitution semantics and shapes-graph variables
- **shacl-af:** Sh:SPARQLFunction user-defined SPARQL functions
- **shapes:** Complete SHACL-AF validation coverage
- **core:** Expose shared FastHasher/FastMap/FastSet + smallvec primitives
- **shapes:** Id-native SHACL engine over interned TermIds
- **sparql-algebra:** Zero-copy lexer tokens borrowing the source
- **entail:** Bare-RDF axiomatic predicate-typing entailment (rdf01)
- **entail:** SHOIQ(D) OWL-Direct tableau reasoner core (concept, parser, tableau)
- **entail:** Query-directed OWL-Direct DL materialization clears 25 conformance cases
- **entail:** RIF-Core rule engine clears rif01/03/04/06 (zero entailment xfails)
- **entail:** Native OWL-DL tableau + RIF-Core engine + bare-RDF axiomatic entailment
- Close public-maturity epic
- **iri:** Add shared source-position primitive (LineIndex/Position)
- **diagnostics:** Resolve lexer byte offsets to line/column
- **codec:** Attach line/column locations to RDF text parse errors
- **codec:** Opt-in triple->source span table
- **validate:** Scaffold purrdf-validate SARIF boundary crate
- **validate:** Hand-rolled deterministic SARIF 2.1.0 model
- **validate:** Map reports and diagnostics to SARIF results
- **validate:** Source-traced SARIF physical and logical locations
- **validate:** To_sarif surface + Python binding + schema validation
- **bindings:** SARIF surfaces for WASM and C-ABI
- **validate:** SARIF rule metadata with W3C SHACL help links
- **validate:** SARIF 2.1.0 source-traced reporting
- **perf:** Id-native SHACL engine, zero-copy lexer tokens, workspace small-vec + hasher sweep
- **rdf-core:** Graph-scoped rdf:first/rest/nil + container traversal on DatasetView
- **slice:** Graph-scoped nav cursor, RDF-1.2 triple-term interiors, list/container materializer
- **release:** Generate the changelog and GitHub Release notes with git-cliff
- **release:** Single-command version bump and coherent tag cut
- **hygiene:** Extend issue-ref lint to workflow yaml and python comments
- **python:** Complete rdflib drop-in epic
- **release:** Docs.rs polish, MSRV/semver docs, version-coherence gate + changelog
- **rdf-query:** Graph-scoped nav cursor, list/container materializer, one-path blank-safe merge, FILTER/UNION regressions

### Other

- **iri,rdf:** Rustfmt the LUT/escape hot paths
- Remove old docs
- **rdf:** Fmt import order in ser_model tests
- Ignore more
- **sparql-eval:** Apply cargo fmt to expr.rs
- **shapes:** Apply cargo fmt to the pinned-lexical test
- Integrate main (parallel eval, content-addressed terms, XSD-1.0 float/double) into the W3C conformance branch
- Strip stale gmeow-ontology issue refs from Rust comments + lint against regression
- **conformance:** Integrate origin/main; fix issue-ref lint to scan only tracked source
- **shacl-af:** Rustfmt wrapping and register new corpus fixtures
- Rustfmt normalization across the SARIF work

### Performance

- **iri:** Const char-class LUT for ASCII validation
- **sparql:** Byte-cursor + memchr tokenizer
- **rdf:** Hex-LUT UCHAR escape, drop write! from the hot path
- **rdf:** Borrow clean input in escape_scan via Cow
- **sparql-eval:** Memoize constant expression atoms per query
- **sparql-eval:** Memoize dataset literal XSD parses per query
- **sparql-eval:** Allocation-free single-column hash-join keys + pre-sized build map
- **sparql-eval:** O(1) visited-set for property-path transitive closure
- **sparql-eval:** Hoist loop-invariant BGP probe permutation selection
- **sparql-eval:** Pre-parse quoted-triple ORDER BY sort keys
- **sparql-eval:** Single-threaded optimization backlog (items 3–7 + ORDER BY triple residual)
- **wasm-pkg:** Add Node parse-throughput benchmark
- **wasm-pkg:** Build the npm artifact with +simd128
- **sparql-eval:** Parallelize BGP inner loop and read-only join probes
- **sparql-eval:** Parallelize FILTER/filtered-left-join with forked per-worker contexts
- **sparql-eval:** Parallelize UNION, BIND, and per-group aggregates with deterministic scratch merge
- **sparql-eval:** Chunk-based parallel collects to cut per-row allocation
- **sparql-eval:** Reintern minted rows by value to drop the per-cell TermValue clone
- **sparql-eval:** Deterministic parallel evaluation (UNION, joins, BGP, FILTER, aggregates)
- **rdf-core:** Add report-only bench for intern-time content-id overhead
- **rdf-core:** Hash predecessor_chain visited set with ahash
- **entail:** Genuine semi-naive delta chase with new-vertex reflexive derivation
- **sparql:** Reuse blank-label set across update-operation iterations
- **shacl-af:** Hoist recursion guard, reuse intersection set, cache sort keys
- **sparql-eval,shapes:** Adopt small-vectors for hot per-row/per-node collections
- **entail:** Reuse frontier buffers in the RDFS chase loop
- **entail:** Reuse frontier buffers in the RIF chase loop
- **shapes:** Pre-resolve rdf:type id once per Class constraint
- **shapes:** Carry id-native value nodes through the constraint layer
- **shapes:** Adopt fixed-key ahash for the remaining membership sets
- **shapes:** Cache report sort key and borrow the sparql dataset
- **rdf:** Validate UTF-8 lazily in the text-format span-tracking arm
- **rdf-core:** Resolve container type once in is_typed_container

### Refactor

- **sparql-eval:** Rc→Arc on SolutionSeq/ExistsInner for Send+Sync
- **sparql-eval:** Make EvalCtx Send+Sync (Arc caches, RwLock order cache, Sync remote)
- **rdf:** Reuse purrdf_gts::wire::hex in the verify bridge
- **shex:** Route datatype checks through parse_xsd10
- **shapes:** Fold double/float lexical check into purrdf-xsd
- **entail:** Split reasoner into vocab/interner/rdfs modules + Regime::Rif scaffolding
- **shapes:** Collapse the non-interned path walker to reflexive inclusion
- **validate:** Hoist shared validate-to-SARIF helper into purrdf-validate
- **rdf:** Centralize native-codec format dispatch behind an RdfCodec trait

### Testing

- **rdf:** Bench the serializer escape boundary path
- **iri:** First parse criterion bench (dev-dep only)
- **sparql-eval:** Forced-parallel byte-identity determinism gate over a query corpus
- **rdf:** Prove content addressing does not perturb serialized bytes
- **shapes:** Cover xsd:float +INF at the SHACL layer; clarify pinned accept-set test
- **shex:** Drop Turtle-parser workaround in validation conformance harness
- **rdf:** Lock leading-empty rejection in nested callers and drain-to-pipe run
- **shex:** Empty the validation trait-skip list and assert zero skips
- **shex:** Lock Import and SemanticAction trait coverage with exact counts
- **sparql-conformance:** Vendor full W3C SPARQL 1.1 QUERY suite with typed non-pass ledger
- **sparql-conformance:** Vendor W3C SPARQL 1.1 UPDATE evaluation suite
- **sparql-conformance:** Vendor W3C SPARQL 1.2 DRAFT suite + classify
- **rdflib-gate:** Ledger test_group_by — purrdf stricter than rdflib on GROUP BY projection
- **sparql:** Cover blank-node reuse inside RDF-1.2 quoted triples
- **shacl-af:** End-to-end goldens for sh:min/max/distinct/offset
- **shapes:** First-party sh:SPARQLFunction conformance corpus cases
- **shacl-af:** Negative-path coverage for sh:SPARQLFunction
- **bench:** Add SHACL pattern-lookup and value-token lexer micro-benches
- **entail:** Lock in deterministic inferred-triple emission order
- **validate:** Cover attribution UnitId -> slice IRI resolution (S0.5)
- **rdf:** Lock parallel line numbering for a newline-less final line
- **validate:** Lock SHACL helpUri anchors against the live spec format
- **validate:** Avoid a literal #N token in the S0.5 test
- **sparql:** Regression-cover FILTER-NOT-EXISTS arithmetic and all-FILTER UNION branch

## [0.2.1] - 2026-07-02

### Benchmarks

- **python:** Published rdflib-vs-shim benchmark harness + docs

### Bug Fixes

- **python/compat:** Reject bare-form xsd duration instead of zeroing it
- **python/compat:** Honor base/initBindings/native kwargs in SPARQL processors
- **python/compat:** Unwrap Resource-typed predicate and index arguments
- **python:** Sample the host wall clock for NOW and RAND/UUID
- **capi:** Sample the host wall clock for NOW and RAND/UUID
- **sparql:** NOW is the wall clock and RAND is real entropy — by default, everywhere
- **sparql:** Thread standpoint + order cache into the UPDATE WHERE context
- **sparql:** NOW/RAND sample the host wall clock, not the epoch

### CI & Build

- **python:** Run the acceptance matrix under the acceptance dep group

### Documentation

- **python/compat:** Strip in-repo tracker references from the compat shim

### Features

- **python:** Python test harness + xfail-ledger gate for the rdflib drop-in
- **python:** Top-level engine exports mirroring the Rust umbrella crate
- **python:** Term-model completeness — value coercion, RDF 1.2 direction, from_n3
- **python:** NamespaceManager + Namespace parity with rdflib
- **python:** Graph/Dataset facade parity with rdflib
- **python:** Rdflib plugin registry + entry-point discovery
- **python:** SPARQL result serialization, native substitutions, property paths
- **python:** Opt-in top-level `rdflib` shadow (import rdflib -> purrdf)
- **build:** Single conformance matrix — native W3C suites + rdflib gate
- **xsd:** Native binary decode + whitespace facets for the compat value map
- **rdf:** Native TriX and HexTuples codecs for the compat plugin registry
- **sparql-eval:** Add wasm-clean QueryEnv seam for NOW/RNG injection
- **11:** Purrdf as an RDF-1.2-first drop-in rdflib replacement

### Other

- **python:** Rustfmt the native SPARQL-results + term-direction bindings
- Release 0.2.1: rdflib drop-in hardening — native xsd coercion + TriX/HexTuples codecs

### Testing

- **python:** Gate — rdflib's own test suite against the compat shim
- **python:** Downstream acceptance matrix (pyshacl / SPARQLWrapper / sssom)

## [0.2.0] - 2026-07-02

### Other

- Release 0.2.0: complete umbrella facade, OntologyProfile, ShExC serializer, drop openEHR OPT

## [0.1.5] - 2026-07-02

### Bug Fixes

- Fix release lanes: wasm-opt post-MVP features, workspace-inherited version

### Documentation

- Add the PurRDF DOI (10.67342/pkg8gpp4no/v1) to CITATION.cff and the README badge row

### Other

- Package README + metadata, js package 0.1.4
- Shex in flight
- Full ShEx 2.1 + complete SHACL Core + de-gmeow the library namespaces
- Purge the invented namespace: purrdf is a toolkit, not an ontology
- Parallel parse + parallel GTS verification (deterministic by construction)
- Python bindings: shex module, engine configuration, GIL release
- Release 0.1.5: full SHACL/ShEx, de-gmeow'd namespaces, SPARQL eval speedups

## [0.1.3] - 2026-07-02

### Bug Fixes

- Include Python sdist toolchain
- Build PyPI manylinux wheels

### Other

- Parameterize jsonld prefix
- Release 0.1.3: brand, first-class docs, strict lints, perf, npm lane
- Stabilize the toolchain: stable-Rust-clean workspace, real MSRV

## [0.1.1] - 2026-07-01

### Bug Fixes

- Set crates.io release user agent
- Pace crates.io bootstrap publishes
- Set crates.io workflow user agent
- Make purrdf the umbrella crate
- Pace only new crate publishes

### Other

- First commit

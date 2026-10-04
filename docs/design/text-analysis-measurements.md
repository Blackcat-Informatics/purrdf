<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Text analysis measurement dossier

These measurements support the declared kernel choices and expose their costs.
They establish neither a universal optimal analyzer nor a full public-benchmark
retrieval score. Production analysis, ranking and fusion use deterministic
integer arithmetic; evaluation ratios and discounts use host floating point.

## Independently annotated domain evidence

The original fixture in `crates/retrieval/tests/relevance/` contains 20 documents,
14 queries and ten separately annotated segmentation rows (39 reference words).
Judgments were written before inspecting dictionary or retrieval output.
They cover symbolic/memetics concepts, technical terms, unknown compounds,
traditional text, supplementary Han, ambiguity, mixed identifiers and emoji.
They have not received native-speaker consumer validation and are too small to
estimate a population-level advantage. No dictionary or fusion parameters were
tuned on these held-out annotations.

Run `cargo run -p purrdf-retrieval --example text_relevance --locked --
crates/text/lexicons/artifacts`. The lexical and Han lanes use independent
corpus statistics; fusion uses explicit equal weights and reciprocal-rank
constant 60 using the existing exact contribution kernel and checked accumulation.
Precision uses
three result slots; recall uses all positive judgments; F1 is the mean per-query
harmonic score, rather than the harmonic mean of aggregate precision/recall.
MRR uses the whole returned ranking. nDCG uses graded relevance at three.

| Stratum | Lane | Queries | P@3 | R@3 | F1@3 | Hit@3 | MRR | nDCG@3 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| all | fusion | 14 | 0.595238 | 0.964286 | 0.721429 | 1.000000 | 1.000000 | 0.961081 |
| all | han | 14 | 0.571429 | 0.892857 | 0.685714 | 0.928571 | 0.928571 | 0.889653 |
| all | lexical | 14 | 0.595238 | 0.964286 | 0.721429 | 1.000000 | 1.000000 | 0.939077 |

Explicit fusion by query stratum:

| Stratum | Lane | Queries | P@3 | R@3 | F1@3 | Hit@3 | MRR | nDCG@3 |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| ambiguity | fusion | 1 | 0.333333 | 1.000000 | 0.500000 | 1.000000 | 1.000000 | 1.000000 |
| emoji | fusion | 1 | 0.333333 | 1.000000 | 0.500000 | 1.000000 | 1.000000 | 1.000000 |
| memetics | fusion | 2 | 0.666667 | 1.000000 | 0.800000 | 1.000000 | 1.000000 | 1.000000 |
| mixed | fusion | 2 | 0.666667 | 1.000000 | 0.800000 | 1.000000 | 1.000000 | 1.000000 |
| supplementary | fusion | 1 | 0.333333 | 1.000000 | 0.500000 | 1.000000 | 1.000000 | 1.000000 |
| symbolic | fusion | 2 | 0.833333 | 1.000000 | 0.900000 | 1.000000 | 1.000000 | 1.000000 |
| technical | fusion | 3 | 0.555556 | 0.833333 | 0.666667 | 1.000000 | 1.000000 | 0.929052 |
| traditional | fusion | 1 | 0.666667 | 1.000000 | 0.800000 | 1.000000 | 1.000000 | 0.833991 |
| unknown | fusion | 1 | 0.666667 | 1.000000 | 0.800000 | 1.000000 | 1.000000 | 0.833991 |

Exact-word segmentation matched 35 of 44 emitted words against 39 reference
words: precision 0.795455, recall 0.897436, F1 0.843373. Errors remain visible
in this measurement; the fixture supplies no special-case segmentation rules.

## Public Chinese evidence

The independent MIRACL Chinese development judgments provide 393 queries,
3,928 judgments and 3,786 distinct judged passages. This evaluation uses a
**closed per-query judged pool**: score and rank the union corpus, filter each
ranking to the query's judged passages, then use those filtered rank positions
for fusion and metric calculation. Unjudged passages are not labeled nonrelevant.
Corpus statistics use all 3,786 passages. These are
judged-pool ranking measurements, **not full-corpus MIRACL retrieval scores**.
[MIRACL dataset card](https://huggingface.co/datasets/miracl/miracl)
[corpus card](https://huggingface.co/datasets/miracl/miracl-corpus)

| Lane | P@3 | R@3 | F1@3 | Hit@3 | MRR | nDCG@3 |
|---|---:|---:|---:|---:|---:|---:|
| Lexical | 0.294317 | 0.395765 | 0.304730 | 0.631043 | 0.496716 | 0.382859 |
| Han | 0.262935 | 0.348334 | 0.268382 | 0.592875 | 0.471442 | 0.343461 |
| Equal-weight fusion | 0.283291 | 0.372889 | 0.289647 | 0.610687 | 0.496834 | 0.369838 |

Fusion improves the small domain fixture's nDCG and reduces several public
metrics. It therefore supplies useful caller-controlled composition, without a
universally superior default or an automatic change to lexical statistics.

Reproduction inputs are pinned to MIRACL revision
`5be20db9509754dadad47689368639fcec739c00` and corpus revision
`d921ec7e349ce0d28daf30b2da9da5ee698bef0d`. Download the Chinese dev topic and
qrels files under `miracl-v1.0-zh/{topics,qrels}/`; stream all ten
`miracl-corpus-v1.0-zh/docs-N.jsonl.gz` shards and retain only the document IDs
named by those judgments. Export sorted `ID<TAB>title text` document rows,
flattening embedded tabs/newlines to spaces. Topic rows contain `ID<TAB>query`;
qrels retain their original four columns. Keep these Wikipedia-derived external
texts outside the source checkout; no corpus passages are redistributed here.
Then run the evaluator with the additional arguments
`DOCUMENTS.tsv TOPICS.tsv QRELS.tsv judged-pool`.

| Extracted input | SHA-256 |
|---|---|
| Topics | `5b284a9aabf08bb2d1c88ed7ea276025c9d23846457c5350dc8d391b5e0d0a13` |
| Qrels | `5546474d3dc8139014e6571e9f4041848bb0a314ca6d2d4bac708174d71c59eb` |
| Judged document TSV | `be1c2ecf8d310273b98c76d7302aca9b4b9e29464df9ee5a8e745d78ba464b55` |

## Dictionary and distance selection

All 40 complete-dictionary construction/load/lookup/segmentation cases completed.
[The dictionary dossier](../../crates/text/lexicons/BENCHMARKS.md) records
bootstrap intervals, MAD, retained bytes and artifact sizes. Minimal acyclic
storage increases retained memory for four dictionaries; CJK loading regresses
outside uncertainty. The compact radix representation stays the default.

All 96 distance comparison cases completed, separating preparation from reused
candidate comparisons. The endpoint-aware banded kernel remains the default.
For 128 scalars at threshold 2, a nearby-edit case measured 173 ns
[167,245] versus prepared Myers 1,130 ns [1,029,1,665]. A shifted case measured
1,973 ns [1,390,2,068] versus Myers 1,608 ns [1,325,1,772], with overlapping
intervals. Preparation measured 159 ns versus 1,644 ns. At 2,048 scalars and
threshold 2, a shifted comparison measured 17.64 µs versus 118.9 µs; at
threshold 64 the shifted case favored Myers (126 µs versus 271 µs).
That local advantage does not satisfy the declared overall non-regression rule.
Both kernels remain fully differential-tested and directly measurable with
`cargo bench -p purrdf-text --bench phonetic --locked -- --quick`.

## Positional substring indexing

The source-aware 1,024-span construction measured 11.37 ms. Indexed versus
explicit exhaustive queries measured 271.3/416.8 µs for a positional probe,
4.153/113.3 µs for a rare probe, 256.1/398.2 µs for a punctuation probe, and
1.569/132.7 µs for a missing gram. Universal-query refusal measured 22.61 µs.
All timed queries first checked exact result equality or the expected typed
refusal. These initial measurements used the final positional algorithm before
removing an unrelated phonetic candidate-tree allocation. The unchecked
preanalyzed construction seam used for an isolated build comparison was removed;
public construction always applies the stored analyzer and retains source evidence.

These quick timing runs overlapped compilation on the development host.
Uncertainty is substantial: they describe these workload samples and are not
hardware-independent speedup guarantees. Use `cargo bench -p purrdf-text
--bench surface --locked -- --quick` to measure the public paths. Native and
executed WASM tests require the same matches, budgets, counters and evidence.

## Allocation and artifact accounting

`cargo run -p purrdf-text --example analysis_allocations --locked --
crates/text/lexicons/artifacts` uses the workspace counting allocator. Setup,
warming and report formatting stay outside per-thread measurement windows.
Allocation count, requested traffic, retained bytes and peak working bytes are
separate quantities. Requested/layout bytes are not process RSS.

The five canonical artifacts total 10,072,091 bytes. The separate deterministic
24-member source/data/notice archive is 5,983,031 bytes (gzip timestamp zero),
SHA-256 `a9d99c9976d574b0f6174395fa8bdd953c76a8249b369b4bdbd36c79a026226a`.
Cargo and Python source packages exclude the dictionary data and include notices
for the data they actually ship. First-party code retains the three-license
choice; the external data retain their original terms.

The final warmed stream measurement made 1,000 calls per class using reusable
`AnalyzerScratch`: ASCII, Latin, mixed, Chinese and Thai each made zero
allocations and requested zero bytes, under both empty and full-baseline
profiles. This claim applies to those measured inputs, not arbitrary input or
HTML expansion. All three persisted aligned projections retain their own
allocated strings and evidence; alignment is not advertised as allocation-free.

Compared with the initial implementation before removing discarded stream
annotations, the exact per-thread measurements are:

| Operation | Allocations before/after | Requested bytes before/after | Final retained bytes | Final peak bytes |
|---|---:|---:|---:|---:|
| Baseline resolution | 4,335,491 / 1,567,939 | 527,868,869 / 244,911,646 | 26,332,925 | 50,765,326 |
| Source-aware surface build, 1,024 rows | 148,068 / 118,372 | 26,537,527 / 16,222,775 | 2,680,653 | 3,383,296 |
| Lexical build, 1,024 rows | 192,291 / 162,595 | 34,440,581 / 24,125,829 | 4,423,673 | 7,100,661 |
| Han build, 1,024 rows | 304,987 / 275,291 | 46,356,414 / 36,041,662 | 6,789,806 | 8,622,068 |
| Selective substring query | 48 / 48 | 6,772 / 6,773 | 944 | 1,934 |

Retained memory is unchanged in each table comparison. Baseline peak increased by
1,056 bytes of reusable scratch, and the selective query requested one additional
byte. A cold mixed-script aligned projection made fewer allocations (137 to 134)
and requested fewer bytes (21,027 to 20,166), while retaining 416 additional bytes
(7,227 to 7,643). Metadata arrays grow amortized across emoji blocks, avoiding
per-atom exact reallocations and quadratic traffic. These small costs are recorded alongside
the substantial traffic reductions rather than hidden in a single memory number.
The stream and persisted views use the same generic normalization law, with
unit metadata for callers that do not retain source annotations. Cleanup context
uses monotonic scalar walks so repeated orthographic controls remain linear.

## Throughput and source-alignment latency

The final analysis benchmark replay completed all 32 cases in two filtered runs
(`purrdf_text_unicode` and `purrdf_text_profiled`): 24 existing Unicode primitive cases and eight profiled analysis cases.
Each input contains at least 64 KiB. The explicit empty-lexicon profile uses
reusable scratch for streaming; aligned analysis constructs all three persisted
views. The following bootstrap intervals are milliseconds per input.

| Input | Operation | Lower | Median | Upper |
|---|---|---:|---:|---:|
| ascii | stream | 1.487 | 1.498 | 1.514 |
| ascii | aligned | 5.621 | 5.671 | 5.785 |
| latin | stream | 7.091 | 9.823 | 11.008 |
| latin | aligned | 13.651 | 22.703 | 37.137 |
| mixed | stream | 2.041 | 2.793 | 6.532 |
| mixed | aligned | 6.696 | 6.867 | 11.933 |
| cjk | stream | 1.883 | 1.886 | 1.896 |
| cjk | aligned | 4.714 | 4.807 | 4.874 |

The same existing primitive workloads also have a pre-change baseline. Selected
observations below are microseconds per 64 KiB input; brackets are bootstrap
intervals. Timing movement includes the concurrent host load: both apparent
improvements and regressions remain visible, without a causal speedup claim.

| Primitive/input | Original median [interval] | Final median [interval] |
|---|---:|---:|
| case_fold/ascii | 3.95 [3.13, 4.46] | 4.54 [3.78, 5.09] |
| nfc/latin | 562.16 [501.11, 627.69] | 392.49 [380.34, 433.24] |
| analysis_form/latin | 2392.00 [2054.47, 2821.25] | 671.49 [668.47, 689.57] |
| analysis_form/cjk | 1281.47 [806.37, 1310.76] | 656.68 [648.23, 676.95] |
| word_indices/cjk | 250.08 [221.41, 364.79] | 220.26 [201.98, 227.94] |

The emoji-heavy aligned regression uses uppercase `A` followed by repeated
`😀`, forcing actual normalization metadata rather than the identity shortcut.
For 256, 512, 1,024, 2,048 and 4,096 emoji, requested bytes were 336,936; 669,736;
1,335,336; 2,666,536; 5,328,936. Doubling the input approximately doubles traffic.
`analysis_allocations::repeated_emoji_metadata_grows_linearly` enforces this
scaling together with normalized output and complete surface projection counts.


## Emoji recognition kernel selection

The selected predicate checks a small shape hint before finite recognition:
`#`, `*` or an ASCII digit must be followed by U+20E3/U+FE0E/U+FE0F; an initial
skin modifier must be a single scalar. The complete `emoji_status` table remains
the recognition authority. All other protected pictographic/regional-indicator
content uses the existing scalar properties, including Prepend and unknown joins.
This removes unnecessary finite-table searches for ordinary scripts and
pictographic emoji without narrowing the semantic law.

An independent oracle derives protected scalars directly from the official
input files. It proves the hint covers every finite atom without those properties
(53 source rows), checks all 5,760 unique spellings, every 1,112,064 Unicode
scalar and 17,280 mutated spellings, and executes the same body on native/WASM.
Generated tables and their identities are unchanged.

A rejected spare-bit hint increased deduplicated GCB blocks from 134 to 162 and
their index/block storage from 34,560 to 38,144 bytes (+3,584). The selected
implementation adds no tables, allocations or persistent state. Its standalone
benchmark executable grows from 2,935,480 to 2,935,784 bytes (+304, 0.0104%);
the predicate symbol grows from 662 to 966 bytes. The measured loaded-section
sum stays 932,101 bytes because alignment/BSS padding offsets text growth.
That particular executable result does not promise zero code or load growth in
every linked application.

Twenty cases were measured in consecutive original/final release-profile runs
on CPU 16, with 60 samples, 200 ms warm-up and 700 ms measurement. Background
build/test load varied. Large savings from skipped lookups are supported; small
ratios and unchanged ASCII are not universal speedup claims. Selected medians:

| Predicate input | Original ns | Final ns | Final 95% interval ns |
|---|---:|---:|---:|
| ASCII word | 1.836 | 1.815 | 1.794–1.829 |
| Han scalar | 124.900 | 3.090 | 3.081–3.107 |
| Han word | 130.500 | 9.430 | 9.376–9.557 |
| Thai grapheme | 136.200 | 5.892 | 5.796–5.958 |
| Thai word | 153.500 | 14.770 | 14.520–15.040 |
| Joined profession | 135.600 | 3.134 | 3.120–3.160 |
| Recognized keycap | 129.400 | 94.260 | 93.450–94.700 |
| Skin component | 131.500 | 86.790 | 85.590–89.920 |
| Nonemoji keycap start | 101.400 | 4.358 | 4.214–4.549 |

An intermediate start-only hint incurred a measured 30–40 ns cost on
`7` followed by U+0301. The second-scalar check removes that lookup and closes
the finding. Run `cargo bench -p purrdf-text --bench emoji --locked` to measure
all public predicate cases; no test asserts a latency threshold.


## Production WebAssembly facade size

Both the unchanged base and completed source were built with the release
WebAssembly lane. The emitted `purrdf_wasm.wasm` files measure:

| Facade build | Bytes | SHA-256 |
|---|---:|---|
| Original base | 18,502,013 | `4d6b1e9dca5c42b940ecdb7b59bae74c67516aad7717efdd892e5e29896817c7` |
| Completed source | 18,501,919 | `d317e1aa3c63d1b35f46cdc062e6174dffb3bcc9e8df4731f319482bdee519a1` |

The facade delta is −94 bytes (−0.000508%). Linker reachability determines which
Rust APIs this facade carries; this figure does not measure a consuming module
that exposes every new analyzer/producer operation. The standalone emoji
executable's +304-byte code cost is reported separately above. Dictionary data
is caller-loaded and absent from the production library packages. All release
crates build for wasm32; the shared executable suites separately exercise the
new loaders, linguistic kernels, projections, budgets and evidence there.

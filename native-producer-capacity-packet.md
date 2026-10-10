# Issue 508 native producer capacity packet

Source inspection only, 2026-10-09. This packet supplements the approved plan; it is implementation input, not another review. No build, test, forge read, or source mutation was performed for this packet. Source paths below refer to this issue's worktree. The caller's immutable `TextIndex`, dictionaries, profiles, embedding spaces and registry bodies are already constructed caller ownership. Their borrowed content and ordinary `Arc` handle clones are not fresh query allocations; any new strings, rows, vectors, boxes or cloned keys derived from them are.

## Chosen admission seam

Use the existing query-owned workspace account for every producer allocation. The implemented public `WorkspaceCapability` is a cloneable concrete wrapper containing `Option<Arc<dyn GrowthAdmission>>`, where `GrowthAdmission` is the private object-safe trait. Cloning the wrapper shares the actual account and creates no new heap owner. Its allocation token releases bytes on drop, and failed admission preserves the account's first typed operational cause. Thread this concrete capability through the actual property-function open and pull path. Native text/occurrence/kNN implementations use the same existing execution helper with a resident or bounded allocation owner; do not introduce a second ranking engine.

Concretely, add an admitted-open entry to `PropertyFunction`, called by `open_contained` from bounded evaluation, with the borrowed `PfArgs`, actual ceiling and a producer workspace capability. Keep that capability in the returned native cursor, so first-pull allocations are also admitted. A legacy opaque relation is allowed only when an allocation-free declaration certifies its complete peak before `open`: initialization/cursor state, temporaries, retained buffers, returned row payloads and any allocation that overlaps engine ingestion. A row count declaration alone is insufficient. The default admitted-open fails for an uncertified opaque relation before invoking its body; native text/occurrence/kNN override it with the owners below. Resident invocation uses the same implementation with a resident owner. The public capability erases the dataset error type, while its adapter retains the original typed cause in the query account. Admit fixed owner/adapter/cursor-box storage as well; the capability itself must not create an unpriced allocation on each charge.

Concrete public shape aligned with the source writer's current account:

```rust
pub struct WorkspaceCapability { /* Option<Arc<dyn private GrowthAdmission>> */ }
impl WorkspaceCapability {
    pub fn charge(&self, bytes: u64) -> Result<WorkspaceAllocation, EvalError>;
}
// Private fields; its Drop releases the actual shared account. It does not clone.
pub struct WorkspaceAllocation { /* actual account guard, admitted bytes */ }
pub struct AdmittedPfRow { /* Vec<TermValue>, WorkspaceAllocation */ }
// PropertyFunction's admitted entry; native cursor retains the shared capability.
fn open_admitted(
    &self,
    args: &PfArgs<'_>,
    ceiling: Option<u64>,
    workspace: WorkspaceCapability,
) -> Result<Box<dyn PfCursor>, EvalError>;
// PfCursor's bounded pull; raw resident next remains a consumer of the same helper.
fn next_admitted(&mut self) -> Result<Option<AdmittedPfRow>, EvalError>;
```

`AdmittedPfRow` has immutable row access and ownership-carrying consumption by the engine, not a raw lease-dropping public extraction. A default certificate wrapper can implement admitted pull for legacy certified cursors, reserving its declared row peak **before** calling raw `next` and retaining the grant with its returned row. Native cursors create the exact row grant before building the values. No deep clone on that envelope; normal sharing, if exposed, shares allocation plus grant. Resident raw `next` and bounded `next_admitted` should call one native row helper with the appropriate allocation owner.

Use explicit `WorkspaceAllocation` phase/buffer guards at each existing concrete owner, priced from a checked sum of selected live capacities. Acquire the new guard before growth/replacement and retain the old guard until its actual allocation drops; no public guard-resize API is required. Relevant private layouts are `AnalyzerScratch`/`NormalizationScratch<()>`/`SegmentationScratch`; `PreparedCorpus { profile: &RankingProfile, documents: u64, totals: Vec<u128>, populations: Vec<u64> }`; `PreparedQuery { corpus: &PreparedCorpus, terms: Vec<(String,u64,Fixed)> }`; `CandidateOccurrence { document: u32, ordinal: u32, counts: &[(u32,u64)] }`; `Candidate { document: u32, score: Fixed, matched: u32 }`; `ByRank(Candidate)`; `Scored { document: u32, score: Fixed, partition_rank: u32, matched: u32 }`; `Hit { document: u32, score: Fixed, rank: Option<u32>, matched: u32 }`; and `Holding { document, key: PartitionKey, located: Vec<(usize,&[(u32,u64)])> }`. Add a capability and persistent charge to `SearchCursor`, `OccurrenceCursor` and native `RankedCursor`; release analyzer/ranking phase guards when those phases actually drop, while retaining cursor rows/args. A fixed field/array of phase guards is sufficient; avoid an unpriced heap-growing vector of charge tokens. The implemented capability directly erases `Arc<Mutex<Account<E>>>` without another capability heap owner, and charges `size_of::<WorkspaceCharge<E>>()` before boxing its concrete guard. That control allocation is included by the capability and should not be charged twice by the native producer.

Reserve before an allocation, clone, formatting call or capacity increase. Account actual capacities and simultaneous old/new buffers. All capacity multiplication, sums and conversions are checked; overflow is a typed operational failure. Use explicit fallible exact reservations for vectors/strings and a charged capacity chosen by the helper. `Vec::clear`, truncation and partial iteration do not release a vector's backing capacity. For a replacement whose old allocation remains live, admit the full new capacity while retaining the old charge, then release the old owner after it is actually dropped. `collect`, amortized `push`, `append`, `format!` and deep `clone` cannot be left as capacity surprises under a certificate.

The following notation avoids architecture-dependent guessed constants:

* `V<T>(n) = checked(n × size_of::<T>())` for the selected capacity. Use actual private native types in their home module, not an assumed layout.
* `S(b)` is the chosen string capacity in UTF-8 bytes.
* `H(v)` is the heap of the fresh owned copy of borrowed `TermValue v`: each IRI/label/literal/datatype/language string, all nested triple term boxes, and the clone algorithm's temporary work buffers. Count inline cells separately in their containing vector.
* A phase peak adds all simultaneously live owners; sequential phases take a maximum only after their old owners have been released. The selected capacity, rather than merely final length, is the claim.

`scratch::value_bytes` is explicitly a deterministic governor proxy and is not the physical `H(v)` certificate. Reuse the core term-walk home to supply a nonallocating layout inspection and a pre-admitted owned copy, including iterative triple-copy work lists. `TermValue::clone` currently uses a `WorkList<Step,32>` and a growing `Vec<TermValue>` before allocating three `TermBox` children per triple (`rdf-core/src/ir/term_walk.rs:437`). A sum of leaf lexical lengths misses those owners. Native producer rows need this same real copy seam as VM values.

## Text analysis owners before ranking

Homes: `text/src/analysis.rs` (`terms`, `analyze_into`, `normalize_into`, `normalize_run`, `NormalizationScratch`), `text/src/segment.rs` (`SegmentationScratch::prepare`), `text/src/character.rs` (`runs`, `query_terms`), `lex/src/html.rs` (strict reference resolution), `lex/src/unicode/aligned.rs` (`decompose_tagged`, `compose_tagged`), and text's generated Unicode case-fold home. These are concrete local owners; do not build a generic allocator interception framework.

`Analyzer::terms` owns its analyzer scratch and every emitted token string. Let `T` be **all emitted token occurrences**, `Q` the distinct token count, and `W` their exact emitted UTF-8 byte sum. Repeated tokens can make `T` and `W` large even when `Q = 1`. `QUERY_TERMS_MAX = 1024` applies after sorting/deduplication in `score::distinct_terms`; it is not an analyzer allocation bound. Reserve the token-vector capacity and each bounded token's copied UTF-8 bytes before the sink clones it. The bounded word law can be measured on the existing borrowed word, after control filtering/stemming, without formatting or copying it to obtain a size.

`NormalizationScratch<()>` retains `output: String`, `cleaned: String` and the scalar, pending and working `Vec<TaggedScalar<()>>` buffers. Reserve their capacities independently, retaining old capacities across clears. A raw input byte count is not by itself an NFKC/case-fold expansion proof. Implement allocation-free pre-counts at each **existing** expansion pass: invoke the same `decompose_scalar` callback to count output scalars before its scratch vector is filled; count the same `fold_scalar` outputs before folding; pre-reserve composition to its input scalar count because composition only reduces it; calculate exact UTF-8 byte sum from the scalar slice before emitting it. This reuses the current generated tables and transformation law, rather than introducing an invented multiplier or a second normalizer. Price pending/scalars/working overlap while swapping them.

HTML mode must reserve before `resolve_strict` builds decoded strings, source ranges or diagnostics. Plain input stays borrowed. At the existing HTML parsing home, share the reference reader with an allocation-free count/sink pass for decoded bytes, decoded scalar/source ranges and diagnostics, then reserve its actual output capacities before filling them. Alternatively make that same reader's local owners reserve before each growth. Do not call the allocating resolver first merely to discover its output length. Diagnostics constructed for invalid input are query allocations too. Preserve strict-reference failure behavior.

For a segmentation chunk, let `G` be its grapheme count and `Bclean` the exact UTF-8 bytes after the existing word-control filter. Both can be counted by the existing borrowed grapheme iterator before `prepare`. Its full chosen capacity is:

```
V<usize>(G+1)                 boundaries
+ V<usize>(G+1)              clean_offsets
+ S(Bclean)                  clean
+ V<Fallback>(G)             fallbacks
+ V<usize>(G)                barriers
+ V<Path>(G+1)               paths
```

`Fallback` holds `next: usize, emit: bool`; `Path` holds `unknown: usize, cost: u128, tokens: usize, next: usize, emit: bool`. Use their `size_of`, including padding. Add `ranges: Vec<Range<usize>>`, `word: String`, normalization owners and previously emitted term owners. Scratch capacity tracks the largest chunk actually processed; dictionary lookup is borrowed and does not rebuild the dictionary.

For Han-character query analysis, the current `character::query_terms` unnecessarily calls `Analyzer::projections`, retaining aligned sources and lexical/surface/span projections although it only reads `analysis.normalized.text`. Choose the smaller coherent change: obtain that text through the existing `Analyzer::analysis_form`/`normalize_into` law, then run the same Han `runs` and unigram/bigram query-selection law. Add the local admitted owner to that existing normalization helper. `runs` retains a `Vec<Unit>` for the largest Han run (`Unit` contains a char and two ranges); pre-count run length with the same iterator or reserve before growth. Each emitted unigram/bigram string is priced from its actual one/two scalars' `len_utf8`. This removes irrelevant projection allocation without changing keys, controls, emoji, adjacency or normalization. Verify term equivalence against existing Han fixtures; do not reject Han under a bounded workspace.

## Exact ranking phases

Homes: `text/src/score.rs:382` (`select_counted`), `:512` (`distinct_terms`), `:526` (`rank_terms`), `:571` (`candidates`), `:738` (`bounded`), `text/src/ranking.rs` (`PreparedCorpus`, `PreparedQuery`) and `text/src/index.rs` (borrowed statistics/postings).

After analysis, `distinct_terms` first collects **T** borrowed string references, sorts and deduplicates; its capacity remains T. Its string content is borrowed. For eligible partition `p`, define:

```
O_p = sum over Q distinct terms of index.document_frequency(p, term)
C_p <= min(index.partition_stats(p).document_count(), O_p)
F = actual ranking profile field count
P = actual population slice length (zero for dense profile, F for sparse)
U = sum of UTF-8 lengths of the Q distinct terms
```

`document_frequency` and `field_postings` inspect immutable borrowed metadata without allocation. `CandidateOccurrence` stores a document ordinal, term ordinal and borrowed predicate-count slice. Therefore the exact occurrence capacity is `V<CandidateOccurrence>(O_p)`; do **not** copy or charge the immutable posting counts themselves. Once this array is sorted, the distinct document count can be computed in a nonallocating pass and used as the exact Candidate capacity. Using `min(D_p,O_p)` before the sort is a certified upper bound.

The candidate/scoring phase retains:

```
V<CandidateOccurrence>(O_p) + V<Candidate>(C_p)
+ V<u128>(F) + V<u64>(P)                 PreparedCorpus cloned statistics
+ V<(&str,u64)>(Q)                       frequencies during preparation
+ V<(String,u64,Fixed)>(Q) + S(U)        PreparedQuery owned terms
```

`S(U)` here means the sum of Q individually chosen term-string capacities. The frequency vector overlaps query construction, and can be released afterwards. `Fixed` is inline `i128`; admitted text BM25 uses native fixed/wide arithmetic and stack field-input arrays. It does not allocate arbitrary-precision limbs per score. Do not add unrelated BigInt charges. `MAX_FIELDS = 16` is a semantic ranking bound; use actual F in the certificate.

`LIMIT` only affects the ranking heap/output. It does **not** reduce `O_p` or `C_p`, because `candidates` materializes and scores all candidates first. Let `K_p = C_p` for full ranking or `min(C_p, keep)` for an admitted prefix. Price these sequential transition peaks after candidate construction:

| Transition | Live allocation before the old input is released |
|---|---|
| Full sorted ranking | `V<Candidate>(C_p) + V<ByRank>(C_p)` unless an explicit proven same-allocation conversion replaces the current collect |
| Heap ranking | `V<Candidate>(C_p) + V<ByRank>(min(C_p, keep)+1)`; the current text heap permits a push before pop |
| Ranked rows | `V<ByRank>(K_p) + V<Scored>(K_p)` while ordered values are consumed into a new vector |
| Global append | existing global Scored capacity + partition Scored capacity + the newly selected global capacity if append grows it |
| Cursor-hit conversion | complete returned Scored capacity + new Hit capacity unless code explicitly proves allocation reuse |

Unstable sorting is in place. Do not substitute stable sorting without pricing its temporary allocation. Heap `into_vec` reuses its backing allocation. Current iterator collection reuse is not a proof merely because two private types happen to have the same size.

`select_counted` sequentially processes partitions, so the transient ranking peak is the maximum single-partition peak **plus** persistent terms/distinct-reference capacity/global rows. Its append can overshoot a requested emission ceiling by one whole partition before truncation. Either pre-admit that real overshoot or append only the needed already-ranked prefix with explicit capacities; truncation alone keeps the oversized allocation. A bound partition rank requires ranking deep enough for that rank. In `TextSearchRelation::open`, bound document, score or matched-term positions disable the selection ceiling, because those predicates filter after ranking. A bound `LIMIT 1` query must still price and inspect the complete required candidate set. Preserve partition-relative ranks, deterministic ordering and full equality filtering.

### Bound-document path

`relation.rs` uses `Holding { document, key: cloned PartitionKey, located: Vec<(usize,&[(u32,u64)])> }`. Find H matching held documents from the borrowed subject index. For each one, count actual present query terms using `posted_frequencies`; reserve that located vector exactly before filling it. Charge `V<Holding>(H)` and every cloned graph/language in its key with `H(graph)`/string bytes. Planning pre-counts must not increment the producer's actual membership-observation counter a second time.

When rank is unobserved, `scored_in_place` prepares per-document corpus/query owners sequentially and produces at most H hits, avoiding full partition ranking. When rank is observed, it also creates `documents: Vec<u32>` and a key restriction, then ranks all selected partition candidates before retaining held-document hits. Price those owners and the full rank result, rather than H as a shortcut for ranking work. Move existing keys where possible; charge any additional copy or sorting capacity. This mode distinction is a supported optimization, not permission to change bound count/rank semantics.

## Search cursor retention and row ownership

`relation.rs:1426` owns `SearchCursor`: fixed cursor box, Arc handles, `needle: TermValue`, `rows: Vec<Hit>`, and `bound: Vec<Option<TermValue>>` for six positions. The bound vector includes a second owned copy of the bound needle. Price both copies unless implementation deliberately makes them share one immutable owned allocation and adjusts the native cursor accordingly. Bound language/filter strings and cloned partition keys are separate owners. Prebuilt index and generation Arc clones need only handle storage; do not charge the entire index again.

Each emitted candidate row currently builds a six-cell vector with cloned subject, cloned needle, rendered score, integer rank, language and integer matched-term count. Before `build` clones or renders, reserve `V<TermValue>(6)` plus each `H(subject)`/`H(needle)` and generated string capacity, datatype URI storage and term descendants. Price the candidate row even when a bound postfilter subsequently discards it. Only one generated candidate row is needed at a time, but it overlaps the persistent cursor and engine row ingestion.

Text scores are nonnegative and bounded by `SCORE_MAX = 65536` at 12 fractional digits; a plain decimal rendering needs at most 18 ASCII bytes. Calculate the actual chosen decimal capacity without allocating, then write directly into that admitted buffer. Rank and matched count use integer digit counts (rank is u32; matched count is at most Q); language copies its exact UTF-8 bytes or the empty literal. `TermValue::integer`, `typed_literal` and canonical renderers must allocate through a pre-admitted capacity, including copied XSD datatype strings. Do not rely on an unpriced `format!` intermediate.

Carry the produced row's lease through ingestion. The current public `PfRow = Vec<TermValue>` has no owner token. The smallest shared seam is an admitted-row envelope used by bounded pull and consumed by `property_fn_eval`: own the Vec and its allocation lease until the engine has admitted its own retained cells/term copies and dropped/moved the producer row. Legacy certified producers can be wrapped at this same boundary with their previously admitted row capacity. A cursor-only lease is safe only if it explicitly covers every live returned row through that ingestion overlap and outlives it; dropping the cursor before the returned row is consumed must not release ownership. Do not post-price a deep row after it was produced.

## Occurrence relation

Home: `relation.rs:1763` open and `:1850` `OccurrenceCursor`, `:1894` `load_next_partition`. It owns analyzed term string, needle copy, four bound-cell copies, filtered `Vec<PartitionKey>` with cloned graph/language content, and current partition postings. The current open clones all partition keys, then filters into another vector. Replace that with borrowed filtering plus exact matched-key pre-count and one admitted clone vector, or price both allocations and their overlap. `partitions_holding_subject` also allocates/clones before sort/dedup; reserve its actual subject-document count and key content before invoking that path.

For partition p and the single analyzed term, define `L_p = document_frequency(p,term)` and `X_p = sum positions.len()` from borrowed postings. `load_next_partition` currently allocates:

```
V<(u32,Vec<u32>)>(L_p) + V<u32>(X_p)
```

The previous partition's postings remain live while the new `collect` builds its replacement, and the loader clones one partition key temporarily. Either admit old + new + key-copy peak, or explicitly drop the drained postings before replacement and borrow the key. A still smaller compatible cursor stores partition/document/position ordinals and reads positions from the retained immutable Arc index instead of copying posting arrays; that removes these fresh allocations and is legitimate if it preserves cursor order and work observations. Do not use a self-referential borrowed slice inside the cursor.

Each returned row owns four cells: subject copy, needle copy, language literal and position integer. Reserve cell layout, term-copy work and string/datatype bytes before construction and transfer its lease through ingestion as above. A ceiling limits emitted rows, not a loaded posting list: a bound position/document may require scanning a whole partition or later partitions to produce its first matching row. Preserve strict one-term analysis refusal, punctuation-empty result and all bound equality filters.

## Other admitted producers

| Native/host seam | Required physical certificate or pre-growth owner |
|---|---|
| Native embedding kNN | `EmbeddingSpace::search` in `sparql-eval/src/knn/mod.rs:675` allocates `V<Option<f64>>(N)` distances and then `V<Ranked>(N)` scored rows; these overlap during distance consumption. `knn::metric::best` adds a heap of `min(N,select_k)` Ranked cells while the scored input stays live; pre-reserve its chosen capacity instead of implicit BinaryHeap growth. Result heap conversion reuses its allocation. First pull invokes search, so cursor open certification alone is insufficient. Bound neighbour/distance disables ceiling pushdown but retains the actual requested k, as `KnnInvocation::open` specifies. Membership mode uses at most one Ranked cell and never the full search arrays. Cursor invocation copies query, count, all four bound cells; generated row copies neighbour/query/count and canonical-double string/datatype. Price exact rendering via existing numeric home, fixed cursor/source storage and generated row ingestion. The immutable embedding matrix, terms and norms are caller-owned, not query-built. |
| Custom aggregate | Existing `CustomAggregate::state_bound` is declared retained state for **one** accumulator, charged in logical ScratchBytes; it does not certify init/step/combine/finish temporaries or returned term growth and can be zero for a boxed stateless accumulator. Add/read a complete physical peak/output declaration or give admitted accumulators the shared capability. Include concrete state box and each simultaneously live partial accumulator; use sequential fold in bounded execution where it is the coherent way to avoid an unbounded parallel partial-state count, while preserving fold order. Existing `exact_numeric_cost[_under]` and `purrdf_xsd::exact::Cost/Shape` price limb operations/heap results from operand sizes; they remain mandatory for arbitrary-precision numeric work, but do not certify arbitrary string/container state. Do not convert zero numeric cost into a zero-byte opaque-host assumption. Survivor row/scalarval owners and returned term-copy/rendering also remain live as applicable. |
| Native scalar/custom expression body | `NativeFnBody` receives borrowed arguments but may allocate arbitrary strings/containers before returning; a returned-value size check is too late. Supply an allocation-free complete peak/output certificate per actual borrowed arguments, or use the shared admitted capability inside the body. The existing user-function admission hook is not automatically a byte certificate. Retain registry identity, volatility, panic containment and per-call governance. Focus/dataset-aware expressions must receive the same query owner on re-entry; borrowed caller graph content does not require a graph-copy reservation. An unknown body without a certificate is precisely the unsupported boundary, not justification to reject every registry/context. |
| SPARQL-bodied functions | Body AST/algebra is known. Admit nested body through the same compiler/algebra/expression accounting path, caller options and parent workspace; reserve parameter/substitution copies and live caller/body overlap before they are allocated. Native opaque certificate rules do not require refusal of a known SPARQL body. Recursive calls consume admitted work/capacity through the same owner and existing depth/call policy; avoid a second ungoverned evaluator. |
| SERVICE/remote | `ServiceResolver::resolve` currently returns fully materialized `ResolvedBindings`. The HTTP adapter allocates headers and a full response String at transport `post`, then decodes results; its max-intermediate-cells limit is not a byte bound. Require a caller-selected complete transport/response/decoder certificate before request, or an admitted transport/sink that reserves before receiving/decoding each growth. Thread the same capability through `ServiceRequest`, response owner and in-process resolver. Price query text, headers, response body, JSON decoder temporaries, variable conversion, rows, strings and their simultaneous overlap before construction; the raw response can coexist with decoded terms. A prebuilt host-owned immutable response can be borrowed/shared with its own owner; do not pretend a newly fetched body was prebuilt. Preserve configured endpoint/profile authorization and stop semantics. SERVICE SILENT cannot turn workspace/admission/allocation or source operational failure into empty success. |

`Cost::bytes()` is the native exact numeric result-heap bound, and `Cost::then` takes a maximum only when the first working set is released; `saturating_add` is not an acceptable overflow success certificate for physical admission. Use checked composition for capacity claims and preserve existing numeric cost governance. The packet supplies native owners, not a new numeric law.

## Boundary acceptance for the substantive implementation

These are focused additions to the shared real-entry matrix; they are not a request for independent full gates per helper.

1. Native text through direct and prepared complete bounded entry succeeds with a sufficient profile for lexical ASCII, strict HTML, Unicode compatibility/case expansion, dictionary segmentation and Han keys. Use allocation measurement to check admitted live ownership at boundaries; the prebuilt index must not be counted as fresh query heap. An exactly insufficient budget fails before the responsible native growth and returns the original typed operational cause.
2. Many repeated query tokens with Q=1 establish that T-reference/token/string capacities are covered. Sparse/multi-field partitions establish corpus/query stats and candidate occurrence layouts. LIMIT 1 with many postings must still admit full O/C buffers; bound score/matched/document and observed rank must produce the same complete filtered/ranked answer as resident execution.
3. Exercise global append across partitions, candidate-to-heap/result transitions, Scored-to-Hit conversion and long nested-triple subjects/needle copies. A row lease remains live through engine ingestion; rejected postfilter rows release their temporary ownership. Cursor drop, empty result, cancellation, source read failure, typed extension failure and failed growth all release their owners, with first operational failure precedence intact.
4. Occurrence relation with a large position list, multiple partitions and a bound late position verifies that emitted ceiling does not underprice loaded postings. Verify the selected borrowing/drop-before-replacement optimization against existing order and work observations, and all generated four-cell row allocations.
5. kNN first pull with N much larger than k, bound-neighbour/distance postfilter, missing seed and membership mode establishes the actual distance/scored/heap phases. The same native numerical path and canonical double spelling are retained.
6. A certified opaque relation/native function/custom accumulator succeeds through actual bounded calls, and a certificate boundary failure occurs before its body/open/init/pull allocation. An uncertified opaque body yields the explicit typed missing-certificate failure rather than fabricated bytes. Known SPARQL-bodied function/focus context follows the shared admitted path. Exercise remote admitted response construction and ensure SERVICE SILENT preserves operational admission failure. Preserve caller options, extension declarations, exact-numeric division policy and stop behavior across direct/prepared/partial/explain callers.

No test result is claimed by this packet. The source writer chooses the exact local signatures and integrates these owners into the approved one-substantive-task delivery; the certified capacities and actual ownership obligations above are the required implementation behavior.

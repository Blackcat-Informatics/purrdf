<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# BM25F ranking profile and vocabulary-search ownership

`purrdf-bm25f-fixed-v2`, revision 2, is the complete scoring law. A concrete
profile's BLAKE3 identity includes that name and revision, the index corpus
construction law `purrdf-text-corpus-graph-language-v1`, the integer logarithm
algorithm, operation order and rounding, scale, `k1`, the exact-intermediate and
query-bound laws, field names and order, weights, length coefficients, sorted
predicate mappings, and the explicit unclassified destination and population
mode. It contains no chosen input ceiling. Dense mode binds
`relative=length*N/total`; carrier mode binds
`relative=length*field_documents/total`. The revision changes profile and index
identities while preserving every previously admitted score lexical.

There is one scoring implementation. Single-field BM25F is its one-field case.
The previous classic BM25 expression was algebraically equivalent over real
numbers, but rounded different intermediate values. PurRDF 2.0 deliberately
changes that arithmetic and the index fingerprint domain to `index/v2`.
Any score lexical or golden that changes does so because the one shared
fielded law is now authoritative.

## Exact operation order

All raw values are signed integers at scale `S = 10^12`. Addition and
subtraction are exact; `mul(a,b) = trunc(a*b/S)` and
`div(a,b) = trunc(a*S/b)`. All admitted scoring operands are nonnegative.
The public `Fixed` arithmetic uses wide intermediates rather than rejecting a
representable result merely because its unscaled multiplication exceeds i128.

For each distinct analyzed query term, in ascending term order:

1. Prepare `idf = ln(1 + (N - df + 0.5)/(df + 0.5))` once per corpus.
2. In profile field order, compute `relative = floor(length*population*S/total)` from
   exact counts. Dense mode uses the corpus population `N`; carrier mode uses
   the number of documents carrying that field. This is the field's length
   divided by its exact mean. Taking the ratio directly avoids rounding the
   mean before division.
3. Compute `normalization = 1 - b + mul(b, relative)`.
4. Accumulate `pseudo += mul(div(tf, normalization), weight)`.
5. Saturate once: `sat = div(mul(pseudo, k1+1), pseudo+k1)`.
6. Add `mul(idf, sat)` to the document score.

`k1` is exactly 1.2. An absent term's field contributes zero after every input
has been validated; an unused field with total zero therefore has an exact,
well-defined zero contribution. A zero weight also leaves all input checks
active. A query has no chosen term-count ceiling; duplicates do not add weight.
The index's existing partition order and canonical document tie order remain
the ranked relation's total order. Heap ceilings and explanations use the same
prepared scorer as full ranking.

The logarithm reduces its positive fixed-point argument to `m * 2^e`, with
`1 <= m < 2`, at internal scale `10^18`. It sets
`z = floor((m-I)*I/(m+I))`, `square = floor(z*z/I)`, then sums twenty terms
`floor(power/(2*j+1))`, replacing `power` with `floor(power*square/I)` after
each. Its result is
`trunc((e*693147180559945309 + 2*sum)/10^6)`.
There is no convergence test, floating point or target-dependent math call.

## Sparse field populations

`RankingProfile::with_field_populations()` selects carrier normalization and
changes the ranking identity. `PreparedCorpus::with_field_populations(profile,
documents, totals, populations)` accepts exact totals and u64 carrier counts
in field order. `PreparedCorpus::new` keeps dense normalization. Each constructor
refuses the other mode rather than score under an incorrect fingerprint.

A population may not exceed the corpus size, and its total may not exceed
`population * u64::MAX`. A nonzero total therefore requires a nonzero
population. Both zero populations with zero totals and nonzero populations with
zero totals are admitted. Corpus-wide `N` still determines IDF; carrier counts
only change length normalization. If every document carries every field, both
modes give the same raw score, including each intermediate truncation.

`TextIndex` derives carrier counts per partition from positive analyzed field
lengths. Rerouting several predicates to one field counts each document once;
reranking reuses retained facts. Its ranking and explanation paths use these
stored statistics. A document missing a field does not consume one of that
field's carriers. Positive-length field inputs must leave enough carriers to
hold the remaining token total. These checks remain active for zero-frequency
and zero-weight contributions.

## Input-derived capacity and score proof

Fields and distinct query terms are addressable vectors, not fixed-size
admission domains. Corpus populations, field lengths and term frequencies use
u64; exact totals use u128. A field total cannot exceed its population times
`u64::MAX`, and a scored document must leave enough remaining carriers to hold
that total. These are consistency consequences of the supplied representation,
not selected ranking ceilings. All nonnegative weights representable by `Fixed`
are accepted. Length coefficients remain in the specified interval `[0,1]`.

Corpus preparation validates field counts, populations and exact totals. Query
preparation requires explicit nonempty keys in strictly sorted, distinct order,
valid document frequencies and a frequency sum no greater than all corpus tokens.
The token-total sum across unrestricted fields uses exact integers too.
Document scoring validates every field before considering zero contributions,
and checks consistent lengths and the distinct-term frequency sum. An empty
corpus contains no document to score. A positive field whose specified
normalization rounds to zero still has an undefined division and raises a typed
Domain error; no clamp or alternate rounding is used.

The original fixed-point operation boundaries are unchanged. Healthy small
operations use checked i128 arithmetic and the existing wide product/quotient.
An intermediate whose rounded result exceeds i128 promotes through
`purrdf_xsd::exact::Integer`, which uses the workspace's one BigInt. Each
multiply/divide still truncates at its original scale boundary; there is no
second scorer and no final-only rational rounding. The count ratio uses the
same wide kernel on `length*population*S/total`, with `length*population` fitting
u128. Intermediate promotion does not alter lexical scores.

`PreparedQuery::score_bound()` returns a private-construction certificate with
maximum `sum_i floor(idf_i*(k1+1))`, its significant raw-score bit width, and the
complete profile fingerprint. Every pseudo-frequency is nonnegative. Its
saturation is at most `k1+1 = 2.2`: truncating its product downward cannot
increase `pseudo*2.2/(pseudo+1.2)`, and division truncates downward again.
Multiplication by each prepared nonnegative IDF is monotone. Summing the
individual ceilings therefore bounds the complete actual query, including all
rounding. The validator checks its exact inclusive interval, not just its width.
`PreparedQuery::score` returns `BoundedScore { value, bound }`, and each native
`Scored` result carries `score_bound`. A host can choose its own key width
without PurRDF reserving tie bits or refusing the input. Point scoring retains
the same value-and-bound carrier until the existing six-cell RDF row is emitted;
the RDF relation's row positions and score lexical stay unchanged.

The public score representation still fits i128 on every supported 32/64-bit
target. For any u64 population the shifted IDF argument is at most `2N+2`, at
most `2^65`. The actual logarithm uses exact range reduction to a mantissa below
`2*10^18`, positive finite atanh terms truncated downward and a downward-rounded
`ln(2)` constant. Its computed result is below 46 at public scale, not merely an
unverified real-log approximation. Every raw term addend is thus below
`46*2.2*10^12`. Multiplying this by even `u64::MAX` addressable query terms is
below `1.9*10^33`, below i128's maximum. The prepared certificate and score sums
remain checked. Field weighting and pseudo-frequency intermediates can be much
larger, which is why their unbounded promotion is necessary.

## Conformance and retained facts

`tests/reference/bm25f.py` is an independent, dependency-free integer reference.
It uses unbounded integers and reproduces each specified truncation. The
committed TSV corpus includes heterogeneous field weights, fractional
coefficients, absent terms, unused fields, sparse and maximum-size corpora,
the former boundary normalizations/query size, and an ubiquitous term whose
IDF rounds to zero at a large corpus size. Matching is independent of
score: zero-weight fields and rounded zero IDFs preserve matching rows and
their canonical tie order. `python
tests/reference/bm25f.py --check` checks the committed values; Rust compares
every raw unit against the same corpus with no tolerance. The corpus has a
fixed nonzero count. Native Rust also checks every dense vector under carrier populations equal to `N`, and compares sparse and boundary scores against testkit's independent unbounded integer arithmetic with exact raw-unit equality.

The index retains predicate-level field lengths and term frequencies. Mapping
predicates to fields and computing per-partition field totals is a projection
over those facts. Reweighting or remapping does not tokenize text, reorder
postings or mutate source identity. It changes ranking identity and the full
answer fingerprint. The analyzer fingerprint identifies the compatibility
caseless UAX29 pipeline and all of its Unicode table versions independently.

The corpus law names documents by `(graph, subject, language)`, partitions by
`(graph, language)`, merges base direction, and excludes zero-token documents.
This law is explicit in ranking identity, rather than merely implied by the
index's statistics. The pure scorer does not construct or discover a corpus:
external stores supply already-partitioned counts and bind their source and
partition selection in their own host identity.

The pure scorer accepts the full u64 population representation for external
stores. The in-memory index retains its explicit u32 document/position address
space; wider pure corpus statistics do not claim it can allocate that many rows.

## Vocabulary substring search decision

Generic vocabulary matching belongs in PurRDF's text engine, alongside
tokenization and scoring. Persistence, page layout, ingestion and compaction
belong to the store hosting it. An FM-index over a distinct vocabulary is an
engine structure under that boundary; merging or rebuilding it during store
compaction is a host scheduling decision.

The shipped analyzer identity promises **exact token matching only**. It does
not imply substring answerability. A trigram vocabulary identity must declare
minimum answerable substring length three; an FM-index identity must declare
one. They must be distinct identities that also bind normalization and Unicode
tables. A request below an identity's declared minimum must be refused, never
returned as an empty match or silently approximated. Switching identities is
an explicit index transition with an answerability check. Neither substring
structure is implemented or advertised by this release; this is the ownership
and identity contract for that separate engine capability.

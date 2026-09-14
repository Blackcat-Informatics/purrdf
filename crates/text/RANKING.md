<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# BM25F ranking profile and vocabulary-search ownership

`purrdf-bm25f-fixed-v1`, revision 1, is the complete scoring law. A concrete
profile's BLAKE3 identity includes that name and revision, the index corpus
construction law `purrdf-text-corpus-graph-language-v1`, the integer logarithm
algorithm, operation order and rounding, scale, `k1`, all bounds, field names
and order, weights, length coefficients, sorted predicate mappings, and the
explicit unclassified destination. A semantic change changes this identity.

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
2. In profile field order, compute `relative = floor(length*N*S/total)` from
   exact counts. This is the field's length divided by its exact mean. Taking
   the ratio directly avoids rounding a sparse field's mean to zero.
3. Compute `normalization = 1 - b + mul(b, relative)`.
4. Accumulate `pseudo += mul(div(tf, normalization), weight)`.
5. Saturate once: `sat = div(mul(pseudo, k1+1), pseudo+k1)`.
6. Add `mul(idf, sat)` to the document score.

`k1` is exactly 1.2. An absent term's field contributes zero after every input
has been validated; an unused field with total zero therefore has an exact,
well-defined zero contribution. A zero weight also leaves all input checks
active. A query has at most 1024 distinct terms; duplicates do not add weight.
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

## Bounds and proof

The profile admits:

| Input | Inclusive maximum |
|---|---:|
| Fields | 16 |
| Distinct query terms | 1024 |
| Documents in a corpus | `2^40` |
| Field length per document | `2^24` |
| Term frequency per field | `2^24` and the field's length |
| Field weight | `2^24` |
| Field normalization coefficient | 1 |
| Raw score | `65_536 * 10^12` |

Weights and coefficients cannot be negative. Exact field totals use u128
because the maximum total is `2^40 * 2^24 = 2^64`. Corpus preparation verifies
that total, field count, population and document frequencies. Query preparation
requires explicit nonempty term keys in strictly sorted, distinct order. The
arithmetic API accepts already-analyzed keys; the index uses its declared
analyzer to produce them. Document scoring
verifies every field and its consistency with the corpus before considering
zero contributions. Whole-document scoring also checks consistent lengths and
that distinct query term frequencies sum to no more than each field length.
An empty corpus contains no document to score, even for an empty query. Prepared inputs own their IDF cache; callers cannot supply
an IDF from a different corpus or profile.

The positive shifted IDF argument is at most `2*N+2`, even allowing `df=0`.
For `N <= 2^40`, `ln(2*N+2) <= ln(2^41+2) < 29`. The implemented logarithm
underestimates this mathematical logarithm on arguments at least one: every
series term is positive, each truncation rounds down, the omitted tail is
positive, and the nonnegative exponent multiplies a downward-rounded `ln(2)`.

Every pseudo-frequency is nonnegative. Its saturation is at most 2.2: rounding
the product downward cannot increase `pseudo*2.2/(pseudo+1.2)`, and division
rounds downward again. Therefore the exact raw score is bounded above by
`1024 * 29 * 2.2 * 10^12 = 65_331_200_000_000_000`, strictly below
`SCORE_MAX = 65_536_000_000_000_000`. This proof includes the actual
intermediate rounding; it does not infer a bound from sampled scores.

`SCORE_BITS` is derived as the bit length of `SCORE_MAX`, yielding 56. The
public validator checks the exact interval `[0,SCORE_MAX]`, not just the width.
A host may use the remaining 72 bits of a u128 for its own tie key. PurRDF
exports no packed comparator and makes no host-specific tie-key truncation
decision.

All intermediates also fit the public representation under these bounds.
The smallest positive field-relative ratio is at least `1/2^24`, so its raw
value is at least 59604. For raw coefficient `0 <= b <= S`, normalization is
`S-b + floor(b*relative/S)`, at least `min(S,relative)`, including rounding.
Thus its raw value is also at least 59604. The maximum weighted field
pseudo-frequency is bounded by `2^48 * 10^24 / 59604` raw units. Summing
sixteen fields and multiplying by 2.2 stays below `1.7 * 10^35`, below i128's
maximum by three orders of magnitude. The count ratio's intermediate is at
most `2^64 * 10^12`, also representable. Checked arithmetic remains active
throughout; these inequalities are asserted alongside the score bound.

## Conformance and retained facts

`tests/reference/bm25f.py` is an independent, dependency-free integer reference.
It uses unbounded integers and reproduces each specified truncation. The
committed TSV corpus includes heterogeneous field weights, fractional
coefficients, absent terms, unused fields, sparse and maximum-size corpora,
the smallest normalization, a maximum-size query, and an ubiquitous term whose
IDF rounds to zero at the largest corpus size. Matching is independent of
score: zero-weight fields and rounded zero IDFs preserve matching rows and
their canonical tie order. `python
tests/reference/bm25f.py --check` checks the committed values; Rust compares
every raw unit against the same corpus with no tolerance. The corpus has a
fixed nonzero count.

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

The pure scorer admits `2^40` documents for external stores; the in-memory
index still has a u32 document-address space. The scorer's wider corpus bound
does not claim the in-memory index can allocate or address that many rows.

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

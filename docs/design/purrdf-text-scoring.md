<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF text scoring and ranking

`purrdf-text` builds an in-memory inverted index over the literals of a frozen
RDF 1.2 dataset and answers ranked retrieval queries against it. This document
records the decisions behind that surface which a reader would otherwise take
for oversights — the missing knob, the missing column, the constant that is not
configurable — and states the limits of the guarantees it makes.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs; every
predicate a query calls this crate by is caller-supplied configuration.

---

## 1. The arithmetic is exact, and that is a correctness requirement

Every score is computed in base-10 fixed point: a signed integer scaled by a
fixed number of fractional digits, with each step checked so that an
intermediate which does not fit is an overflow error rather than a wrapped or
saturated number. No floating-point value enters the crate at all — the crate
root denies `clippy::float_arithmetic`, so none can.

This is not a stylistic preference for integers. BM25 needs a natural logarithm,
and a libm `ln` is permitted to differ by a unit in the last place between
implementations. One unit in the last place is enough to reverse the order of
two near-tied documents. The same query, over the same data, would then return
rows in one order from a native build and another order from a
`wasm32-unknown-unknown` build of the same engine — an answer divergence, and
one that nothing downstream could detect, because both answers are well formed
and neither is identifiable as wrong.

So the logarithm is an integer series evaluated for a **fixed iteration count**
rather than to a convergence test. A convergence test would terminate after a
number of iterations that depends on the arithmetic it is testing, which
reintroduces exactly the target-dependence being eliminated. A fixed count makes
the result a pure function of its input on every target, and a ranking
reproducible byte for byte.

Because the arithmetic is exact rather than approximate, a document's per-term
contributions **sum exactly** to its score. The explanation surface reports an
equality, not a tolerance.

## 2. The BM25 constants are constants

`K1` (1.2) and `B` (0.75) are crate constants. There is no parameter struct, no
builder, and no knob.

PurRDF is a carrier, and optionality that changes semantics per consumer is
forbidden. A tuning parameter here would be exactly that: two callers pointing
the same query at the same index would get different scores and — worse, because
it is the thing consumers actually order on — a different `?rank`, with neither
answer identifiable as the wrong one. Ranked retrieval's entire output *is* an
order, so a knob that changes the order changes the answer.

This is a different rule from the one governing the indexed predicate IRIs.
Those are caller-supplied because PurRDF mints no vocabulary: an IRI this
project chose for itself would be a term of somebody's ontology invented by a
carrier, and would end up in an RDF graph as data. A saturation constant from
the retrieval literature is not a vocabulary, ends up in no graph, and names
nothing. The two rules point in opposite directions here, and it is worth being
explicit about which one applies.

The values are the canonical Okapi ones from the published literature. Choosing
the literature's numbers rather than inventing better ones is the point: they
are the values a reader can check this implementation against.

## 3. Statistics and `?rank` are per partition

A **partition** is a `(graph, language)` pair. Every BM25 input that describes a
*corpus* rather than a document — the document count `N`, the average document
length `avgdl`, a term's document frequency `df` — is computed within one
partition and never pooled across partitions.

Pooling is not merely untidy, it is wrong in a way that moves results. An
English needle's inverse document frequency would become a function of how much
Japanese text happens to sit beside it in the same dataset, so adding unrelated
documents in another language would silently reorder the English answers. And it
would print two documents' scores side by side as though they were comparable
when they were computed against different vocabularies.

`?rank` follows from that. A score is a number *relative to one corpus*, so a
rank spanning partitions would sort together numbers computed against different
corpora — an arrangement rather than a ranking. `?rank` is therefore the 1-based
position of a document **within its own partition**, and every comparison the
ranker makes stays inside one partition. A query that wants a single ranked list
binds `?lang` (and the graph where relevant), or runs against a single-partition
index, which is the common case for a corpus in one language.

Restricting to partitions *before* ranking is sound precisely because ranks are
per-partition: dropping whole partitions cannot change the rank of any surviving
row, because no surviving row was ever compared against a dropped one.

Within a partition the order is `(score DESC, document id ASC)`. Document ids are
assigned only after every document has been sorted by `(graph, subject,
language)`, so ascending id *is* ascending canonical term order — the tie-break
is meaningful rather than arbitrary, and it is reproducible across independently
built indexes rather than merely stable within one. Ids are distinct, so this is
a strict total order and no two rows can tie. That is what lets a bounded heap
and a full sort agree exactly rather than approximately.

## 4. There is no `?graph` row position

The graph is part of the document key, so text is never merged across graphs:
the same subject IRI in two named graphs is two documents, and a search
restricted to one graph is never scored by the other's text.

It is nonetheless **not** a column of the emitted row, and the reason is RDF's,
not this crate's. A property-function row has a fixed width, and every cell must
hold an RDF term. RDF has no term that denotes the default graph — the default
graph is the absence of a graph name, not a name. A fixed-width row carrying a
graph cell would therefore have to put *something* in that cell for a
default-graph document, and the only candidates are a minted sentinel IRI or an
empty-string literal. A sentinel IRI is forbidden outright: PurRDF mints no
vocabulary, and that IRI would land in consumers' graphs as data. An empty
string collides with a graph legitimately named by an empty-string literal, and
so answers a different question than the one asked.

Per-graph search is expressed instead by configuring the index's
`GraphSelector`, which names a graph by IRI in value space and is resolved
against the dataset in hand. A caller who wants one graph indexes one graph.

## 5. The row ceiling is conditional, and this is what was measured

A ceiling is the evaluator's licence to tell a relation that only `k` rows
matter. When one is offered, ranking runs through a binary heap bounded at `k`
instead of sorting the whole candidate list, so it is genuine work reduction
rather than an early stop over a full sort. Because emission is `(partition ASC,
rank ASC)`, a row at output position `i` has rank at most `i + 1`, so no row
beyond rank `k` in any partition can reach the first `k` of the output — the
bound is exact rather than heuristic.

The ceiling is an **optimization**: the answer is identical whether it is offered
or honoured. What follows is therefore recorded as observed behaviour rather
than as a requirement. The one thing that is a requirement — and is asserted
directly — is that the answer under `LIMIT k` is exactly the first `k` rows of
the answer without one, compared as whole rows rather than by count.

| Query shape | Ceiling offered |
| --- | --- |
| bare `LIMIT k` | yes — the node's own output is the answer's prefix |
| `ORDER BY … LIMIT k` | **no** |
| a variable repeated across two argument positions | **no** |
| a correlated call under a `Lateral` | yes, but shrinking per invocation |

`ORDER BY … LIMIT k` withholds it because a sort makes this a top-k problem with
no certified lower bound: the row that sorts first can be produced last, so no
prefix of the node's output is the answer's prefix and the plan licenses
nothing.

A repeated variable withholds it because the relation is handed two free
positions and cannot know they must agree. A licence to stop after `k` rows
would be counted against rows the engine then discards for a reason the relation
never saw.

Under a `Lateral`, the relation is opened once per driving row and the offered
ceiling **shrinks** across those invocations: the evaluator offers what is left
of the node's licence once the invocations already driven have contributed to
the bag. Three driving rows each contributing one row are offered `3`, `2`, `1`,
and a fourth invocation does not happen.

The practical consequence is worth stating plainly. Emission order is already
rank order, and rank order is score order within a partition, so
`ORDER BY DESC(?score) LIMIT k` pays for a sort it did not need *and* gives up
the pushdown. A bare `LIMIT k` is the idiom for "the top k".

### `ORDER BY ?rank` is the reproducing idiom

A score reaches a consumer as an `xsd:decimal` rendered at a fixed number of
fractional digits. Two documents can therefore report the *same* `?score` while
carrying different `?rank` — either because their scores really are equal, in
which case the order was settled by document id and the printed score never
showed it, or because the value was rendered at fewer digits than separate them.

`ORDER BY DESC(?score)` is consequently **not** a total order over the rows, and
it can disagree with `ORDER BY ?rank`. `ORDER BY ?rank` states the order the
ranker computed rather than recomputing one from a rounded number; it is total,
and it is the only one that survives rounding.

## 6. The analysis pipeline

Every literal that enters the index and every needle that enters a query goes
through one analyzer, and through nothing else. A retrieval engine matches query
terms against index terms by equality, so two pipelines that merely resemble each
other produce a search that returns nothing and reports nothing. Silence is
retrieval's failure mode, which is why the two ends are the same code.

The pipeline is **compatibility normalization plus full case folding**, then
segmentation at Unicode word boundaries (`UAX #15`, `UAX #21`, `UAX #29`).

Both halves of that choice matter:

* Lowercasing is not case folding. `str::to_lowercase` leaves `ß` as `ß`, so
  `STRASSE` lowercases to `strasse` while `Straße` lowercases to `straße` — two
  terms, no match, no diagnostic. Full case folding maps both to `strasse`.
* Canonical normalization alone preserves compatibility distinctions by
  construction, so fullwidth `ｒｕｓｔ` would stay distinct from `rust` and the
  ligature `ﬁ` from `fi`. Compatibility normalization is what collapses them.

Normalization runs over the whole input **before** segmentation, never per token.
A canonically decomposed `é` is `e` followed by a combining acute accent, and a
lone combining mark is not `Alphabetic`; the word-boundary rules would split it
off the base character it modifies, so the decomposed spelling of a word would
segment differently from the precomposed spelling of the same word.

### CJK, as measured rather than assumed

`UAX #29` assigns Han ideographs and Hiragana the `Word_Break` value `Other`, and
rule WB999 breaks between any pair of characters not joined by an earlier rule.
The observable consequences, confirmed against the linked tables and asserted by
the test suite:

* **Han** segments to **one token per ideograph** — `中文全文検索` yields six
  single-character tokens, not one token for the phrase.
* **Hiragana** likewise segments one token per character.
* **Katakana does not.** It carries `Word_Break = Katakana`, and rule WB13 keeps
  a katakana run together, so `サンドイッチ` is a single token.
* **Hangul does not.** Hangul syllables are `ALetter`, and Korean is written with
  spaces, so it segments into whole words.

Because unspaced CJK arrives as a stream of one-character tokens, expanding
*each token* into bigrams would accomplish nothing — every such token is already
one character. Bigrams are therefore formed **across adjacent tokens**: a maximal
run of adjacent all-CJK tokens is rejoined and expanded into overlapping
character bigrams. Adjacency is read from byte offsets, so `中文` and `中 文`
analyze differently, which is the intent.

Bigrams are the standard answer for retrieval over a script with no spaces.
Indexing unigrams would reduce a phrase query to a bag of characters, matching
any document containing those characters anywhere; a dictionary segmenter would
need per-language data this crate does not carry, and would make answers depend
on that data's vintage. A run of exactly one character has no bigram to form and
is emitted whole, so a single ideograph stays retrievable.

## 7. The Unicode table versions do not agree, and that is stated rather than hidden

This crate depends on four Unicode tables, and they are **not** at the same
Unicode version. As linked today, the case-folding tables are at Unicode 16.0.0
while the normalization and segmentation tables are at 17.0.0; the standard
library's own tables track the toolchain. All four versions are reported
individually rather than summarized into a single number, because summarizing
would require picking one, and picking would hide precisely this situation.

Answers remain deterministic regardless, because the lockfile pins every one of
those tables: a given checkout links a fixed set, and a fixed set analyzes a
given literal to a fixed token vector on every target.

The real risk is therefore not the skew itself but a **dependency bump**. None of
these tables is under this repository's control, and a bump can change what a
literal tokenizes to, which changes the term dictionary, which changes which
documents a query retrieves. Nothing about that announces itself: the engine
still returns rows, just not the same rows.

Two mechanisms make that loud instead of silent:

1. **A golden token-vector test.** Exact expected token vectors span Latin case
   folding, Greek final sigma, Cyrillic, right-to-left Arabic, pointed Hebrew,
   Devanagari with dependent vowel signs, Han bigrams, mixed Kana, Hangul,
   numerals, compatibility presentation forms and punctuation — so a change
   confined to any one of those still lands on an assertion. A failure there is
   not a test to update; it is a report that the term dictionary this crate would
   build has changed, and the change has to be understood and deliberately
   accepted before the vector is rewritten.
2. **All four versions are folded into the index fingerprint**, so an index built
   under one set of tables is distinguishable from one built under another.

## 8. Determinism, and exactly how far it goes

Document ids are assigned only after every document has been sorted by
`(graph, subject, language)`, so an id is a function of content rather than of
the order the dataset happened to intern its terms in. The term dictionary is
sorted by analyzed term text, postings by document id, positions ascending. Two
independently built indexes over the same content agree byte for byte, including
on both digests.

**The limit of that claim, stated precisely.** Blank-node labels are a parsing
artifact rather than content — two isomorphic datasets can label the same blank
node `b0` and `b17` — and a blank node's term identity is its label. A blank-node
subject therefore carries its label into the sort key, into the document table,
and into both fingerprints. **Two isomorphic datasets parsed from differently
labelled sources produce different document ids and different fingerprints.**

This crate does not canonicalize blank nodes. RDFC-1.0 canonicalization lives in
`purrdf-core`, is a substantially more expensive operation, and has its own
contract; silently invoking it here would make index construction pay that cost
without asking. The honest statement is that ids and digests are a pure function
of the dataset's *terms*, and that for blank nodes a term includes its label. An
adversarial test pins this behaviour directly, so the boundary is recorded in
executable form rather than only in prose.

Two digests are kept, and they answer different questions. The **index
fingerprint** covers everything that can change an answer: the configuration, all
four Unicode table versions, the document table, the dictionary, every posting
with its positions, and every partition's statistics. The **source fingerprint**
covers the `(graph, subject, predicate, literal)` rows that were walked. The
second exists because pairing an index with the wrong dataset is otherwise a
silent wrong answer rather than a failure — the relation emits well-formed
document terms, those terms join by basic graph pattern against a dataset that
never contained them, zero rows come out, and no layer has anything to report.
Recomputing the source digest over the dataset in hand detects exactly that.

## 9. Phrase and proximity compose in SPARQL

This crate exposes two relations rather than an embedded query dialect. Ranked
retrieval emits one row per matching document; the **term-occurrence** relation
emits one row per occurrence of one term, carrying the token position.

Positions are what make phrase and proximity matching expressible in the query
language a caller already has. Adjacency is a filter:

```sparql
SELECT ?doc WHERE {
  ?doc <https://example.org/pf#occurs> ( "quick" ?l ?p1 ) .
  ?doc <https://example.org/pf#occurs> ( "brown" ?l ?p2 ) .
  FILTER(?p2 = ?p1 + 1)
}
```

Proximity within a window is the same shape with a different filter:

```sparql
FILTER(ABS(?p2 - ?p1) <= 3)
```

and the two relations compose, so a phrase constraint can be intersected with a
ranked search and still order by `?rank`.

The predicate IRIs above are fixtures. Neither relation names an IRI of its own;
the predicate a query calls each by is supplied by the caller at registration,
and a configuration that supplies none is a configuration error rather than a
guess.

Conjunctive retrieval is expressible the same way rather than through a boolean
dialect: ranked rows carry `?matched`, the number of **distinct** needle terms
present in the document, so a three-term needle restricted to documents holding
all three is `FILTER(?matched = 3)`.

Positions run consecutively across the whole of a document's concatenated
literals rather than restarting per literal, so a phrase can span two of a
subject's literals exactly as it would span two sentences of one literal.

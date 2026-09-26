// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! BM25 ranking: exact scores, per-partition ranks, and a total order with no
//! ties in it.
//!
//! # One fielded arithmetic path
//!
//! The immutable [`crate::RankingProfile`] identifies field weights, length
//! normalization, predicate routing, bounds, rounding and query aggregation.
//! [`K1`] stays fixed at 1.2; [`B`] is the single-field profile's default 0.75.
//! Every profile, including that single field, runs the same prepared BM25F
//! scorer. Field frequencies are normalized and weighted before saturation;
//! summing independently saturated field scores is a different ranking law.
//!
//! Terms are distinct and visited in sorted order. Corpus statistics and the
//! prepared IDF cache belong to one partition and one immutable profile.
//!
//! # Every rank is per partition, and that is a correctness decision
//!
//! Corpus statistics are computed per `(graph, language)` partition
//! ([`PartitionKey`]), so a score is a number *relative to one corpus*. A rank
//! that spanned partitions would therefore sort together numbers computed against
//! different corpora — an English document's score against English statistics
//! beside a Japanese document's score against Japanese statistics — which is the
//! cross-corpus BM25 fallacy. The two numbers do not denote the same quantity,
//! and ordering them produces an arrangement rather than a ranking.
//!
//! So [`Scored::partition_rank`] is the 1-based position of a document **within its own
//! partition**, and every comparison this module makes stays inside one
//! partition. A query that wants a single ranked list binds `?lang` (and `?graph`
//! where relevant), or runs against a single-partition index, which is the common
//! case for a corpus in one language.
//!
//! # The total order
//!
//! Within a partition the order is `(score DESC, document id ASC)`.
//!
//! The tie-break is meaningful rather than arbitrary. The index assigns document
//! ids only **after** sorting every document by `(graph, subject, language)`, so
//! ascending document id *is* ascending canonical term order — comparing two ids
//! is comparing their canonical positions, and it costs one integer comparison
//! instead of two term comparisons. Two independently built indexes over the same
//! content assign the same ids, so the tie-break is reproducible across builds
//! and not merely stable within one.
//!
//! Ids are distinct, so this is a **strict total order and no two rows can tie**.
//! That is what lets a bounded heap and a full sort agree exactly rather than
//! approximately.
//!
//! Across partitions the emission order is `(partition key ASC, rank ASC)`.
//!
//! # Equal scores, unequal ranks
//!
//! A score reaches a consumer as an `xsd:decimal` with a fixed number of
//! fractional digits, and two documents can report the *same* `?score` while
//! carrying different `?rank` — either because their scores really are equal (the
//! order is then settled by document id, which the printed score does not show)
//! or because a consumer rendered them at fewer digits than separate them.
//!
//! `ORDER BY DESC(?score)` is therefore not a total order over the rows, and it
//! can disagree with `ORDER BY ?rank`. **`ORDER BY ?rank` is the reproducing
//! idiom**: it is the order this module computed, it is total, and it is the only
//! one that survives rounding.
//!
//! # Selection
//!
//! [`select`] applies a [`PartitionFilter`] **before** ranking. That is sound
//! precisely because ranks are per-partition: dropping whole partitions cannot
//! change the rank of any row that survives, because no surviving row was ever
//! compared against a dropped one. It is also why a bound `?lang` still permits
//! the bounded-heap path below rather than forcing a full ranking first.
//!
//! A ceiling `k` is real work reduction rather than an early stop: each partition
//! is ranked through a binary heap bounded at `k` entries, which never sorts the
//! tail. Because emission is `(partition ASC, rank ASC)`, a row at output position
//! `i` has rank at most `i + 1`, so no row beyond rank `k` in any partition can
//! reach the first `k` of the output — the bound is exact, not a heuristic. When
//! `?rank` is bound to `r`, a heap of size `r` suffices for the same reason. The
//! heap path and the full sort produce identical output, which this crate's test
//! suite asserts directly across a spread of `k`.
//!
//! # A needle that analyzes to no terms
//!
//! It is an empty result, not an error.
//!
//! A needle of pure punctuation is a **well-formed** request. Analysis is total —
//! every string has an analysis form, and `"---"`'s happens to contain no tokens
//! — so nothing about the request could not be interpreted; it simply names no
//! terms, and a sum over no terms matches no document. Refusing it would also
//! contradict the index side, which excludes a document whose literals analyze to
//! nothing rather than failing the build: the two ends must agree about what "no
//! text" means. And because a needle is usually a bound variable rather than a
//! constant, an error here would abort an entire SPARQL evaluation over one row
//! of data-dependent input, converting a legitimate empty result into a failure.
//!
//! A malformed request is a different thing and keeps its own channel: a missing
//! predicate IRI is a [`TextError::Config`], and a needle asking about a document
//! that does not exist is a [`TextError::Data`].

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use purrdf_core::TermValue;

use crate::error::TextError;
use crate::fixed::{Fixed, SCALE_DIGITS};
use crate::index::{PartitionKey, TextIndex};
use crate::ranking::{PreparedCorpus, PreparedQuery, QUERY_TERMS_MAX};

/// The raw constants below are written at [`SCALE_DIGITS`] fractional digits, so
/// the scale and the literals cannot drift apart unnoticed.
const _: () = assert!(
    SCALE_DIGITS == 12,
    "the BM25 constants are spelled out at twelve fractional digits"
);

/// The fixed BM25F term-frequency saturation constant, `k1 = 1.2`.
pub const K1: Fixed = Fixed::from_raw(1_200_000_000_000);

/// Default field length normalization for the explicit single-field profile.
pub const B: Fixed = Fixed::from_raw(750_000_000_000);

/// One half — the `1/2` of the inverse document frequency's two shifts.
#[cfg(test)]
const HALF: Fixed = Fixed::from_raw(500_000_000_000);

// ---------------------------------------------------------------------------
// Public shapes
// ---------------------------------------------------------------------------

/// One ranked document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scored {
    /// The index document id — the id [`TextIndex::document`] resolves.
    pub document: u32,
    /// The exact BM25 score, computed against this document's own partition.
    pub score: Fixed,
    /// The 1-based position of this document **within its own partition**.
    ///
    /// Never a global position, and named so that it cannot be read as one. A
    /// field called `rank` beside a field called `score` invites exactly the
    /// wrong reading — that sorting two rows by it is meaningful whatever
    /// partitions they came from — and the name is the only place that reading
    /// can be headed off, because a `1` from one partition and a `1` from
    /// another are both perfectly ordinary values and nothing downstream can
    /// tell they were computed against different corpora.
    ///
    /// See this module's documentation for why a rank spanning partitions would
    /// order numbers that do not denote the same quantity.
    pub partition_rank: u32,
    /// How many **distinct** needle terms occur in this document.
    ///
    /// This exists so that conjunctive retrieval is expressible in the query
    /// language a caller already has — a three-term needle restricted to
    /// documents holding all three is `FILTER(?matched = 3)` — rather than by
    /// PurRDF minting a boolean query dialect of its own.
    pub matched: u32,
}

/// One needle term's share of one document's score.
///
/// Every field here is a value the scorer already computed on its way to
/// [`Scored::score`]; [`explain`] exposes them rather than recomputing an
/// approximation of them. Because the arithmetic is exact fixed point, the
/// contributions **sum exactly** to the score — an equality, not a tolerance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TermContribution {
    /// The analyzed term.
    pub term: String,
    /// How many times it occurs in the document. Zero for a needle term the
    /// document does not hold, which contributes exactly zero.
    pub term_frequency: u64,
    /// How many of the partition's documents hold it.
    pub document_frequency: u64,
    /// `ln(1 + (N − df + 1/2) / (df + 1/2))`, over this partition's `N`.
    pub inverse_document_frequency: Fixed,
    /// This term's addend in the score sum.
    pub contribution: Fixed,
}

/// How a query constrains one dimension of a [`PartitionKey`].
///
/// Three cases, because a partition key's dimension has three: unconstrained, or
/// bound to the *absent* value (the default graph, or an untagged literal), or
/// bound to a present one. Collapsing the middle case into the last would make a
/// query for the default graph indistinguishable from a query for a graph named
/// by the empty string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Constraint<T> {
    /// The query leaves this dimension unbound; every partition qualifies on it.
    Any,
    /// The query binds this dimension to the absent value — the default graph,
    /// or an untagged literal.
    Absent,
    /// The query binds this dimension to exactly this value.
    Exactly(T),
}

impl<T> Default for Constraint<T> {
    /// [`Constraint::Any`] — an unmentioned dimension constrains nothing.
    ///
    /// Written by hand rather than derived, because the derive would demand a
    /// `T: Default` that none of the constrained types have or should have.
    fn default() -> Self {
        Self::Any
    }
}

/// Which partitions a query is restricted to, before any ranking happens.
///
/// Applying this ahead of ranking is sound because ranks are per-partition:
/// dropping whole partitions cannot change a surviving row's rank, since no
/// surviving row was ever compared against a dropped one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PartitionFilter {
    /// The graph dimension.
    graph: Constraint<TermValue>,
    /// The language dimension.
    language: Constraint<String>,
    /// An explicit allow-list of partition keys, sorted and distinct, or `None`
    /// for no such restriction.
    ///
    /// The two dimensions above are a *product*, and some restrictions are not
    /// products. "The partitions this subject appears in" is the one that
    /// matters here: a subject may hold English text in one graph and French in
    /// another without holding French in the first, which no `(graph, language)`
    /// pair can express without also admitting a partition the subject is
    /// absent from. Ranking that extra partition would be work whose every row
    /// is then discarded.
    keys: Option<Vec<PartitionKey>>,
}

impl PartitionFilter {
    /// A filter that admits every partition.
    #[must_use]
    pub fn unconstrained() -> Self {
        Self::default()
    }

    /// This filter with its graph dimension replaced.
    #[must_use]
    pub fn with_graph(mut self, graph: Constraint<TermValue>) -> Self {
        self.graph = graph;
        self
    }

    /// This filter with its language dimension replaced.
    #[must_use]
    pub fn with_language(mut self, language: Constraint<String>) -> Self {
        self.language = language;
        self
    }

    /// The graph dimension's constraint.
    pub const fn graph(&self) -> &Constraint<TermValue> {
        &self.graph
    }

    /// The language dimension's constraint.
    pub const fn language(&self) -> &Constraint<String> {
        &self.language
    }

    /// This filter restricted to `keys` as well, on top of whatever it already
    /// constrains.
    ///
    /// The list is sorted and deduplicated here, so [`Self::matches`] decides
    /// membership by binary search and the filter's own identity does not depend
    /// on the order a caller happened to collect the keys in.
    ///
    /// An **empty** list admits nothing, and that is the intended reading rather
    /// than a degenerate one: "the partitions holding this subject", for a
    /// subject the index holds no text for, is genuinely empty, and the honest
    /// answer is to rank nothing rather than to rank everything and discard it.
    #[must_use]
    pub fn restricted_to(mut self, keys: Vec<PartitionKey>) -> Self {
        let mut keys = keys;
        keys.sort();
        keys.dedup();
        self.keys = Some(keys);
        self
    }

    /// The explicit allow-list, or `None` when the filter carries none.
    pub fn keys(&self) -> Option<&[PartitionKey]> {
        self.keys.as_deref()
    }

    /// Whether `key` names a partition this filter admits.
    pub fn matches(&self, key: &PartitionKey) -> bool {
        let graph = match &self.graph {
            Constraint::Any => true,
            Constraint::Absent => key.graph().is_none(),
            Constraint::Exactly(name) => key.graph() == Some(name),
        };
        let language = match &self.language {
            Constraint::Any => true,
            Constraint::Absent => key.language().is_none(),
            Constraint::Exactly(tag) => key.language() == Some(tag.as_str()),
        };
        let listed = self
            .keys
            .as_ref()
            .is_none_or(|keys| keys.binary_search(key).is_ok());
        graph && language && listed
    }
}

// ---------------------------------------------------------------------------
// Ranking
// ---------------------------------------------------------------------------

/// Rank one partition's documents against `needle`.
///
/// `needle` is the **analyzed** query — the token texts [`Analyzer`] produced,
/// in any order and with repeats allowed; this function takes the distinct terms
/// and visits them in sorted order. A needle with no terms ranks nothing (see
/// this module's documentation), and so does a partition the index does not hold.
///
/// `limit` bounds the work rather than merely the output: with `Some(k)` the
/// partition is ranked through a binary heap of `k` entries and the tail is never
/// sorted. The rows returned are identical either way.
///
/// Rows come back in rank order, [`Scored::partition_rank`] running from `1` —
/// and `1` here means first *in this partition*, not first in the index.
///
/// [`Analyzer`]: crate::Analyzer
pub fn rank_partition(
    index: &TextIndex,
    partition: &PartitionKey,
    needle: &[String],
    limit: Option<u64>,
) -> Result<Vec<Scored>, TextError> {
    let terms = distinct_terms(needle)?;
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    rank_terms(index, partition, &terms, limit, &mut ScoringWork::default())
}

/// Rank every partition `filter` admits, and emit the rows in
/// `(partition key ASC, per-partition rank ASC)` order.
///
/// * `filter` is applied **before** ranking, which is sound because ranks are
///   per-partition.
/// * `ceiling` is the number of rows to emit — a `LIMIT` over the emission order
///   above. It bounds each partition's heap too, because a row at output position
///   `i` has [`Scored::partition_rank`] at most `i + 1`, so nothing beyond rank
///   `ceiling` in any partition can reach the output.
/// * `partition_rank` restricts the emission to that one 1-based **per-partition**
///   position, so it yields at most one row per admitted partition — not one row
///   overall. It is what a bound rank position compiles to, and it bounds each
///   heap at that many entries. A `partition_rank` of `0`, or one past the end of
///   every partition, yields nothing: ranks are 1-based, so `0` names no row, and
///   asking for a row a partition does not have is a question with an honest empty
///   answer rather than an error.
pub fn select(
    index: &TextIndex,
    needle: &[String],
    filter: &PartitionFilter,
    ceiling: Option<u64>,
    partition_rank: Option<u32>,
) -> Result<Vec<Scored>, TextError> {
    select_counted(
        index,
        needle,
        filter,
        ceiling,
        partition_rank,
        &mut ScoringWork::default(),
    )
}

/// [`select`], adding the work it did to `work`.
pub(crate) fn select_counted(
    index: &TextIndex,
    needle: &[String],
    filter: &PartitionFilter,
    ceiling: Option<u64>,
    partition_rank: Option<u32>,
    work: &mut ScoringWork,
) -> Result<Vec<Scored>, TextError> {
    let terms = distinct_terms(needle)?;
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // A bound rank needs the heap deep enough to know that rank; otherwise the
    // ceiling is the only thing bounding it.
    let limit = partition_rank.map_or(ceiling, |wanted| Some(u64::from(wanted)));
    let emitted = ceiling.map_or(usize::MAX, |k| usize::try_from(k).unwrap_or(usize::MAX));

    let mut rows: Vec<Scored> = Vec::new();
    for (key, _) in index.partitions() {
        if rows.len() >= emitted {
            break;
        }
        if !filter.matches(key) {
            continue;
        }
        let mut partition_rows = rank_terms(index, key, &terms, limit, work)?;
        if let Some(wanted) = partition_rank {
            partition_rows.retain(|row| row.partition_rank == wanted);
        }
        rows.append(&mut partition_rows);
    }
    rows.truncate(emitted);
    Ok(rows)
}

/// Every needle term's share of one document's score, in sorted term order.
///
/// Terms the document does not hold are reported too, with a
/// [`TermContribution::term_frequency`] of zero and a contribution of exactly
/// zero: "this term is not here" is part of why a document scored what it did.
/// Because zero is exact, the contributions still sum exactly to
/// [`Scored::score`].
///
/// A `document` the index does not hold is a [`TextError::Data`] — the caller
/// asked about something that is not there, which is a different thing from a
/// document that scored nothing.
pub fn explain(
    index: &TextIndex,
    document: u32,
    needle: &[String],
) -> Result<Vec<TermContribution>, TextError> {
    let terms = distinct_terms(needle)?;
    let Some(partition) = index.partition_key_of(document) else {
        return Err(TextError::data(format!(
            "document {document} is not in this index, so there is nothing to explain"
        )));
    };
    let stats = index.partition_stats(partition).ok_or_else(|| {
        TextError::data(format!(
            "document {document} names a partition the index does not hold"
        ))
    })?;
    let corpus = prepared_corpus(index, partition, stats.document_count())?;
    let query = prepare_terms(index, partition, &corpus, &terms)?;
    let mut out = Vec::with_capacity(terms.len());
    for (ordinal, term) in terms.into_iter().enumerate() {
        let (document_frequency, inverse_document_frequency) = query
            .term_statistics(ordinal)
            .expect("prepared term ordinal");
        let fields = index.field_inputs(document, term)?;
        out.push(TermContribution {
            term: term.to_owned(),
            term_frequency: index.term_frequency(document, term),
            document_frequency,
            inverse_document_frequency,
            contribution: query
                .contribution(ordinal, &fields[..index.ranking_profile().fields().len()])?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Candidates, the heap, and the total order
// ---------------------------------------------------------------------------

/// A scored document before its rank is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Candidate {
    /// The index document id.
    document: u32,
    /// The exact score.
    score: Fixed,
    /// How many distinct needle terms the document holds.
    matched: u32,
}

/// A [`Candidate`] ordered so that **greater means ranks later**.
///
/// One order, used by both the heap and the sort, which is what makes the two
/// agree by construction rather than by coincidence. `BinaryHeap` yields its
/// maximum, so a heap of these pops the worst-ranked entry — exactly what a
/// bounded top-`k` needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ByRank(Candidate);

impl Ord for ByRank {
    /// `(score DESC, document id ASC)`.
    ///
    /// Ids are distinct, so this never returns [`Ordering::Equal`] for two
    /// different candidates: it is a strict total order, and a bounded heap and a
    /// full sort cannot disagree about it. Ascending document id is ascending
    /// canonical `(graph, subject, language)` order, because the index assigns
    /// ids only after sorting on that key.
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .score
            .cmp(&self.0.score)
            .then_with(|| self.0.document.cmp(&other.0.document))
    }
}

impl PartialOrd for ByRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The analyzed needle's distinct terms, sorted.
///
/// Sorted by the same byte order the index's dictionary is sorted by, so "visit
/// the query's terms in order" and "visit the dictionary in order" are the same
/// traversal.
pub(crate) fn distinct_terms(needle: &[String]) -> Result<Vec<&str>, TextError> {
    let mut terms: Vec<&str> = needle.iter().map(String::as_str).collect();
    terms.sort_unstable();
    terms.dedup();
    if terms.len() > QUERY_TERMS_MAX {
        return Err(TextError::data(format!(
            "the needle holds {} distinct terms, which exceeds the profile bound of 1024",
            terms.len()
        )));
    }
    Ok(terms)
}

/// Rank one partition against already-distinct, already-sorted `terms`.
fn rank_terms(
    index: &TextIndex,
    partition: &PartitionKey,
    terms: &[&str],
    limit: Option<u64>,
    work: &mut ScoringWork,
) -> Result<Vec<Scored>, TextError> {
    let candidates = candidates(index, partition, terms, work)?;
    let limit = limit.map(|value| usize::try_from(value).unwrap_or(usize::MAX));
    let ordered = match limit {
        Some(keep) => bounded(candidates, keep),
        None => sorted(candidates),
    };

    let mut rows = Vec::with_capacity(ordered.len());
    for (position, ByRank(candidate)) in ordered.into_iter().enumerate() {
        let partition_rank = u32::try_from(position + 1).map_err(|_| {
            TextError::overflow("a partition holds more ranked rows than a u32 can number")
        })?;
        rows.push(Scored {
            document: candidate.document,
            score: candidate.score,
            partition_rank,
            matched: candidate.matched,
        });
    }
    Ok(rows)
}

/// One query term's retained predicate facts in one candidate document.
#[derive(Clone, Copy)]
struct CandidateOccurrence<'a> {
    /// Canonical document identifier.
    document: u32,
    /// Ordinal of the sorted distinct query term.
    ordinal: u32,
    /// Borrowed predicate frequencies, avoiding per-candidate lookup or copying.
    counts: &'a [(u32, u64)],
}

/// Every document of `partition` that holds at least one of `terms`, scored.
///
/// A document holding none of them is not a candidate. Candidate membership is
/// independent of field weights: a matching document can legitimately score
/// zero, and still participates in the canonical tie order.
fn candidates(
    index: &TextIndex,
    partition: &PartitionKey,
    terms: &[&str],
    work: &mut ScoringWork,
) -> Result<Vec<Candidate>, TextError> {
    let Some(stats) = index.partition_stats(partition).copied() else {
        // Not a partition this index holds, so it holds no documents there. That
        // is the true answer rather than a failure.
        return Ok(Vec::new());
    };

    // `(document, term ordinal, predicate frequencies)`. Sorting this groups the whole
    // working set by document while leaving each document's terms in the sorted
    // term order the sum is defined to run in.
    let mut occurrences: Vec<CandidateOccurrence<'_>> = Vec::new();
    for (ordinal, term) in terms.iter().enumerate() {
        for (document, counts) in index.field_postings(partition, term) {
            occurrences.push(CandidateOccurrence {
                document,
                ordinal: ordinal as u32,
                counts,
            });
        }
    }
    work.posting_lists += terms.len() as u64;
    work.postings += occurrences.len() as u64;
    occurrences.sort_unstable_by_key(|entry| (entry.document, entry.ordinal));
    let corpus = prepared_corpus(index, partition, stats.document_count())?;
    let query = prepare_terms(index, partition, &corpus, terms)?;

    let mut out: Vec<Candidate> = Vec::new();
    let mut at = 0;
    while at < occurrences.len() {
        let document = occurrences[at].document;
        let run = occurrences[at..]
            .iter()
            .take_while(|entry| entry.document == document)
            .count();
        let held = occurrences[at..at + run]
            .iter()
            .map(|entry| (entry.ordinal as usize, entry.counts));
        out.push(score_document(index, &query, document, held, work)?);
        at += run;
    }
    Ok(out)
}

/// One partition's corpus, prepared from the statistics the index already holds.
///
/// Nothing here walks the corpus: the population is the partition's stored count
/// and the field totals were summed once, when the index was built. So preparing
/// it costs a function of the ranking profile's field count, whether the caller
/// then scores a thousand candidates or one.
fn prepared_corpus<'i>(
    index: &'i TextIndex,
    partition: &PartitionKey,
    documents: u64,
) -> Result<PreparedCorpus<'i>, TextError> {
    let totals = index
        .field_totals(partition)
        .ok_or_else(|| TextError::data("partition has no field totals"))?;
    PreparedCorpus::new(index.ranking_profile(), documents, totals)
}

/// `terms`' inverse document frequencies over `corpus`, in their sorted order.
///
/// A document frequency is the length of the term's posting run within the
/// partition, which [`TextIndex::document_frequency`] reads off the dictionary
/// in two binary searches without visiting a single posting.
fn prepare_terms<'c, 'p>(
    index: &TextIndex,
    partition: &PartitionKey,
    corpus: &'c PreparedCorpus<'p>,
    terms: &[&str],
) -> Result<PreparedQuery<'c, 'p>, TextError> {
    let frequencies: Vec<(&str, u64)> = terms
        .iter()
        .map(|term| (*term, index.document_frequency(partition, term)))
        .collect();
    corpus.prepare_query(&frequencies)
}

/// One document's exact score: the **one** summation every score in this crate is
/// produced by.
///
/// `held` is the document's postings for the query terms it holds, as `(term
/// ordinal, predicate frequencies)` in ascending ordinal order — the sorted term
/// order the sum is defined to run in. The ranker hands it a run of its grouped
/// occurrences; the point scorer hands it the postings its lookups located. Both
/// reach the same additions in the same order, so the two scores are one number
/// rather than two numbers that agree.
fn score_document<'a>(
    index: &TextIndex,
    query: &PreparedQuery<'_, '_>,
    document: u32,
    held: impl Iterator<Item = (usize, &'a [(u32, u64)])>,
    work: &mut ScoringWork,
) -> Result<Candidate, TextError> {
    let field_count = index.ranking_profile().fields().len();
    let mut score = Fixed::ZERO;
    let mut matched: u32 = 0;
    for (ordinal, counts) in held {
        let fields = index.field_inputs_from_counts(document, counts)?;
        score = score.checked_add(query.contribution(ordinal, &fields[..field_count])?)?;
        matched += 1;
    }
    work.documents_scored += 1;
    Ok(Candidate {
        document,
        score: index.ranking_profile().validate_score(score)?,
        matched,
    })
}

/// The work one scoring call did, counted where it is done.
///
/// Every field is a count of an operation, never a duration, so two calls over the
/// same index and request report the same numbers on every run and every target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScoringWork {
    /// Posting lists walked from end to end: one per `(partition ranked, needle
    /// term)`.
    pub(crate) posting_lists: u64,
    /// Individual postings those walks read.
    pub(crate) postings: u64,
    /// Documents a score was computed for.
    pub(crate) documents_scored: u64,
}

/// A document's score and matched-term count, computed **without ranking
/// anything**: the point scorer a caller reaches for when the one position a
/// rank would fill is read by nothing.
///
/// `located` is what that caller's membership lookups found — the document's
/// posting for each needle term it holds, as `(term ordinal, predicate
/// frequencies)` over `terms` in ascending ordinal order — so no posting is
/// searched for twice and none is walked. `terms` must be the needle's
/// [`distinct_terms`], the same list the ranker would have been handed.
///
/// The score is the one the ranker gives this document, exactly: the corpus is
/// prepared from the same stored statistics, the inverse document frequencies
/// from the same posting-run lengths, and the sum runs through
/// [`score_document`], the one summation the ranker uses too. What is missing is
/// only [`Scored::partition_rank`], because a rank is a fact about every other
/// candidate of the partition and computing it is exactly the corpus-wide work
/// this function exists not to do.
///
/// Work is independent of the corpus: one binary search per needle term for its
/// document frequency, and one field-input assembly per held term.
pub(crate) fn score_located(
    index: &TextIndex,
    document: u32,
    terms: &[&str],
    located: &[(usize, &[(u32, u64)])],
    work: &mut ScoringWork,
) -> Result<(Fixed, u32), TextError> {
    let partition = index.partition_key_of(document).ok_or_else(|| {
        TextError::data(format!(
            "document {document} is not in this index, so there is nothing to score"
        ))
    })?;
    let stats = index.partition_stats(partition).copied().ok_or_else(|| {
        TextError::data(format!(
            "document {document} names a partition the index does not hold"
        ))
    })?;
    let corpus = prepared_corpus(index, partition, stats.document_count())?;
    let query = prepare_terms(index, partition, &corpus, terms)?;
    let scored = score_document(index, &query, document, located.iter().copied(), work)?;
    Ok((scored.score, scored.matched))
}

/// Every candidate, in rank order.
fn sorted(candidates: Vec<Candidate>) -> Vec<ByRank> {
    let mut ordered: Vec<ByRank> = candidates.into_iter().map(ByRank).collect();
    // Unstable is safe and canonical here: the order is strict and total, so
    // there is no pair whose relative order a stable sort would preserve and an
    // unstable one would not.
    ordered.sort_unstable();
    ordered
}

/// The best `keep` candidates, in rank order, through a heap of that size.
///
/// The heap holds at most `keep + 1` entries at any moment, so ranking a corpus
/// of a million documents for a ten-row answer touches eleven of them at a time
/// and sorts ten at the end — real work reduction, rather than a full sort that
/// stops reading its own output early.
fn bounded(candidates: Vec<Candidate>, keep: usize) -> Vec<ByRank> {
    if keep == 0 {
        return Vec::new();
    }
    let mut heap: BinaryHeap<ByRank> = BinaryHeap::with_capacity(keep.min(candidates.len()) + 1);
    for candidate in candidates {
        heap.push(ByRank(candidate));
        if heap.len() > keep {
            // The maximum under `ByRank` is the worst-ranked entry held.
            heap.pop();
        }
    }
    let mut ordered = heap.into_vec();
    ordered.sort_unstable();
    ordered
}

#[cfg(test)]
mod tests {
    use purrdf_core::TermValue;

    use super::{B, Constraint, HALF, K1, PartitionFilter, distinct_terms};
    use crate::index::PartitionKey;

    /// The constants denote the literature's values, exactly.
    #[test]
    fn the_constants_are_the_canonical_ones() {
        assert_eq!(K1.to_decimal_lexical(), "1.200000000000");
        assert_eq!(B.to_decimal_lexical(), "0.750000000000");
        assert_eq!(HALF.to_decimal_lexical(), "0.500000000000");
    }

    /// The needle's terms are deduplicated and sorted, whatever order they
    /// arrived in.
    #[test]
    fn the_needle_is_reduced_to_sorted_distinct_terms() {
        let needle = ["gamma", "alpha", "gamma", "beta"].map(str::to_owned);
        assert_eq!(
            distinct_terms(&needle).expect("a short needle"),
            vec!["alpha", "beta", "gamma"]
        );
    }

    /// An absent dimension is not the same as a present empty one, in either
    /// direction — the distinction the three-case constraint exists for.
    #[test]
    fn a_bound_absent_dimension_is_not_a_bound_empty_one() {
        let untagged = PartitionKey::new(None, None);
        let empty_tag = PartitionKey::new(None, Some(String::new()));

        let absent = PartitionFilter::unconstrained().with_language(Constraint::Absent);
        assert!(absent.matches(&untagged));
        assert!(!absent.matches(&empty_tag));

        let exactly_empty =
            PartitionFilter::unconstrained().with_language(Constraint::Exactly(String::new()));
        assert!(!exactly_empty.matches(&untagged));
        assert!(exactly_empty.matches(&empty_tag));
    }

    /// An unconstrained filter admits everything, and each dimension constrains
    /// only itself.
    #[test]
    fn the_filter_constrains_one_dimension_at_a_time() {
        let named = PartitionKey::new(
            Some(TermValue::iri("https://example.org/g")),
            Some("en".to_owned()),
        );
        assert!(PartitionFilter::unconstrained().matches(&named));

        let english =
            PartitionFilter::unconstrained().with_language(Constraint::Exactly("en".to_owned()));
        assert!(english.matches(&named));
        assert!(english.matches(&PartitionKey::new(None, Some("en".to_owned()))));
        assert!(!english.matches(&PartitionKey::new(None, Some("fr".to_owned()))));

        let default_graph = PartitionFilter::unconstrained().with_graph(Constraint::Absent);
        assert!(!default_graph.matches(&named));
        assert!(default_graph.matches(&PartitionKey::new(None, Some("en".to_owned()))));
    }
}

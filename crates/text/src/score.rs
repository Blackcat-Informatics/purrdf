// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! BM25 ranking: exact scores, per-partition ranks, and a total order with no
//! ties in it.
//!
//! # One fielded arithmetic path
//!
//! The immutable [`crate::RankingProfile`] identifies field weights, length
//! normalization, predicate routing, the query-bound law, rounding and query aggregation.
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

use crate::query_workspace::{admitted, overflow, query_error};
use purrdf_sparql_eval::{
    AdmittedVec, NativeDiagnosticKind, WorkspaceAllocation, WorkspaceCapability,
};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use purrdf_core::TermValue;

use crate::error::TextError;
use crate::fixed::{Fixed, SCALE_DIGITS};
use crate::index::{PartitionKey, TextIndex};
use crate::ranking::{BoundedScore, PreparedCorpus, PreparedQuery, ScoreBound};

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
    /// The actual query/corpus bound, including rounding, for host key sizing.
    pub score_bound: ScoreBound,
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
    pub matched: usize,
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

purrdf_hash::default_from_new!(PartitionFilter => unconstrained);

impl PartitionFilter {
    /// A filter that admits every partition; [`Default`] delegates here.
    #[must_use]
    pub const fn unconstrained() -> Self {
        Self {
            graph: Constraint::Any,
            language: Constraint::Any,
            keys: None,
        }
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
    let terms = distinct_terms(needle);
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
    let rows = select_counted_owned(
        index,
        needle.iter().map(String::as_str),
        |key| filter.matches(key),
        ceiling,
        partition_rank,
        work,
        &WorkspaceCapability::default(),
    )?;
    // This API creates only resident owners; moving through their iterator cannot
    // shed an operational grant. Bounded callers use the carrier below directly.
    Ok(rows.into_iter().collect())
}

pub(crate) fn select_counted_owned<'a>(
    index: &TextIndex,
    needle: impl ExactSizeIterator<Item = &'a str>,
    matches: impl Fn(&PartitionKey) -> bool,
    ceiling: Option<u64>,
    partition_rank: Option<u32>,
    work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Scored>, TextError> {
    let terms = distinct_terms_owned(needle, workspace)?;
    let limit = partition_rank.map_or(ceiling, |wanted| Some(u64::from(wanted)));
    let emitted = ceiling.map_or(usize::MAX, |k| usize::try_from(k).unwrap_or(usize::MAX));
    let mut rows = AdmittedVec::new(workspace);
    if terms.is_empty() {
        return Ok(rows);
    }
    for (key, _) in index.partitions() {
        if rows.len() >= emitted {
            break;
        }
        if !matches(key) {
            continue;
        }
        let partition_rows = rank_terms_owned(index, key, &terms, limit, work, workspace)?;
        // Rank the entire required working population first. Move only the prefix
        // this output can emit; no oversized append/truncate allocation remains.
        for row in partition_rows {
            if partition_rank.is_none_or(|wanted| row.partition_rank == wanted) {
                if rows.len() == emitted {
                    break;
                }
                admitted(rows.push(row))?;
            }
        }
    }
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
    let terms = distinct_terms(needle);
    let Some(partition) = index.partition_key_of(document) else {
        return Err(TextError::data(format!(
            "document {document} is not in this index, so there is nothing to explain"
        )));
    };
    let corpus = index.prepared_corpus(partition)?;
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
    /// Original prepared-query certificate carried through selection and sorting.
    score_bound: ScoreBound,
    /// How many distinct needle terms the document holds.
    matched: usize,
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
pub(crate) fn distinct_terms(needle: &[String]) -> Vec<&str> {
    distinct_terms_owned(
        needle.iter().map(String::as_str),
        &WorkspaceCapability::default(),
    )
    .expect("resident term destinations")
    .into_iter()
    .collect()
}

pub(crate) fn distinct_terms_owned<'a>(
    needle: impl ExactSizeIterator<Item = &'a str>,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<&'a str>, TextError> {
    let mut terms = admitted(AdmittedVec::with_capacity(needle.len(), workspace))?;
    for term in needle {
        admitted(terms.push(term))?;
    }
    terms.as_mut_slice().sort_unstable();
    let mut distinct = 0;
    for at in 0..terms.len() {
        if distinct == 0 || terms[at] != terms[distinct - 1] {
            terms.as_mut_slice()[distinct] = terms[at];
            distinct += 1;
        }
    }
    terms.truncate(distinct);
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
    Ok(rank_terms_owned(
        index,
        partition,
        terms,
        limit,
        work,
        &WorkspaceCapability::default(),
    )?
    .into_iter()
    .collect())
}

fn rank_terms_owned(
    index: &TextIndex,
    partition: &PartitionKey,
    terms: &[&str],
    limit: Option<u64>,
    work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Scored>, TextError> {
    let Some(partition) = admitted(index.partition_key_owned(partition, workspace))? else {
        return Ok(AdmittedVec::new(workspace));
    };
    let candidates = candidates_owned(index, partition, terms, work, workspace)?;
    let keep = limit.map(|n| usize::try_from(n).unwrap_or(usize::MAX));
    let ordered = order_owned(candidates, keep, workspace)?;
    let mut rows = admitted(AdmittedVec::with_capacity(ordered.len(), workspace))?;
    for (position, ByRank(candidate)) in ordered.into_iter().enumerate() {
        let rank = position.checked_add(1).ok_or_else(overflow)?;
        let partition_rank = u32::try_from(rank).map_err(|_| {
            query_error(
                workspace,
                NativeDiagnosticKind::Function,
                format_args!(
                    "fixed-point overflow: a partition holds more ranked rows than a u32 can number"
                ),
            )
            .unwrap_or_else(|failure| failure)
        })?;
        admitted(rows.push(Scored {
            document: candidate.document,
            score: candidate.score,
            score_bound: candidate.score_bound,
            partition_rank,
            matched: candidate.matched,
        }))?;
    }
    Ok(rows)
}

/// One query term's retained predicate facts in one candidate document.
#[derive(Clone, Copy)]
struct CandidateOccurrence<'a> {
    /// Canonical document identifier.
    document: u32,
    /// Ordinal of the sorted distinct query term.
    ordinal: usize,
    /// Borrowed predicate frequencies, avoiding per-candidate lookup or copying.
    counts: &'a [(u32, u64)],
}

/// Every document of `partition` that holds at least one of `terms`, scored.
///
/// A document holding none of them is not a candidate. Candidate membership is
/// independent of field weights: a matching document can legitimately score
/// zero, and still participates in the canonical tie order.
fn candidates_owned(
    index: &TextIndex,
    partition: &PartitionKey,
    terms: &[&str],
    work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<AdmittedVec<Candidate>, TextError> {
    if index.partition_stats(partition).is_none() {
        return Ok(AdmittedVec::new(workspace));
    }
    let occurrences_count = terms.iter().try_fold(0usize, |count, term| {
        let run =
            usize::try_from(index.document_frequency(partition, term)).map_err(|_| overflow())?;
        count.checked_add(run).ok_or_else(overflow)
    })?;
    let mut occurrences = admitted(AdmittedVec::with_capacity(occurrences_count, workspace))?;
    for (ordinal, term) in terms.iter().enumerate() {
        for (document, counts) in index.field_postings(partition, term) {
            admitted(occurrences.push(CandidateOccurrence {
                document,
                ordinal,
                counts,
            }))?;
        }
    }
    work.posting_lists = work
        .posting_lists
        .checked_add(u64::try_from(terms.len()).map_err(|_| overflow())?)
        .ok_or_else(overflow)?;
    work.postings = work
        .postings
        .checked_add(u64::try_from(occurrences.len()).map_err(|_| overflow())?)
        .ok_or_else(overflow)?;
    occurrences
        .as_mut_slice()
        .sort_unstable_by_key(|entry| (entry.document, entry.ordinal));
    let mut count = 0usize;
    let mut prior = None;
    for entry in &occurrences {
        if prior != Some(entry.document) {
            count = count.checked_add(1).ok_or_else(overflow)?;
        }
        prior = Some(entry.document);
    }
    let corpus = index.prepared_corpus_owned(partition, workspace)?;
    let query = prepare_terms_owned(index, partition, &corpus, terms, workspace)?;
    let mut out = admitted(AdmittedVec::with_capacity(count, workspace))?;
    let mut at = 0;
    while at < occurrences.len() {
        let document = occurrences[at].document;
        let run = occurrences[at..]
            .iter()
            .take_while(|entry| entry.document == document)
            .count();
        let held = occurrences[at..at + run]
            .iter()
            .map(|entry| (entry.ordinal, entry.counts));
        admitted(out.push(score_document(
            index, &query, document, held, work, workspace,
        )?))?;
        at += run;
    }
    Ok(out)
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
    prepare_terms_owned(
        index,
        partition,
        corpus,
        terms,
        &WorkspaceCapability::default(),
    )
}

fn prepare_terms_owned<'c, 'p>(
    index: &TextIndex,
    partition: &PartitionKey,
    corpus: &'c PreparedCorpus<'p>,
    terms: &[&str],
    workspace: &WorkspaceCapability,
) -> Result<PreparedQuery<'c, 'p>, TextError> {
    let mut frequencies = admitted(AdmittedVec::with_capacity(terms.len(), workspace))?;
    for &term in terms {
        admitted(frequencies.push((term, index.document_frequency(partition, term))))?;
    }
    corpus.prepare_query_owned(&frequencies, workspace)
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
    workspace: &WorkspaceCapability,
) -> Result<Candidate, TextError> {
    let field_count = index.ranking_profile().fields().len();
    let mut score = Fixed::ZERO;
    let mut matched = 0;
    for (ordinal, counts) in held {
        let fields = index.field_inputs_from_counts_owned(document, counts, workspace)?;
        score = score.checked_add_with(
            query.contribution(ordinal, &fields[..field_count])?,
            workspace.is_bounded().then_some(workspace),
        )?;
        matched += 1;
    }
    work.documents_scored = work.documents_scored.checked_add(1).ok_or_else(overflow)?;
    Ok(Candidate {
        document,
        score: query
            .score_bound()
            .validate_with(score, workspace.is_bounded().then_some(workspace))?,
        score_bound: query.score_bound(),
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
pub(crate) fn score_located_owned(
    index: &TextIndex,
    document: u32,
    terms: &[&str],
    located: &[(usize, &[(u32, u64)])],
    work: &mut ScoringWork,
    workspace: &WorkspaceCapability,
) -> Result<(BoundedScore, usize), TextError> {
    let Some(partition) = index.partition_key_of(document) else {
        return Err(query_error(
            workspace,
            NativeDiagnosticKind::Data,
            format_args!("document {document} is not in this index, so there is nothing to score"),
        )?);
    };
    let corpus = index.prepared_corpus_owned(partition, workspace)?;
    let query = prepare_terms_owned(index, partition, &corpus, terms, workspace)?;
    let scored = score_document(
        index,
        &query,
        document,
        located.iter().copied(),
        work,
        workspace,
    )?;
    Ok((
        BoundedScore {
            value: scored.score,
            bound: scored.score_bound,
        },
        scored.matched,
    ))
}

/// Every candidate, in rank order.
struct OwnedOrder {
    values: Vec<ByRank>,
    allocation: Option<WorkspaceAllocation>,
}
struct OwnedOrderIter {
    values: std::vec::IntoIter<ByRank>,
    _allocation: Option<WorkspaceAllocation>,
}
impl IntoIterator for OwnedOrder {
    type Item = ByRank;
    type IntoIter = OwnedOrderIter;
    fn into_iter(self) -> Self::IntoIter {
        OwnedOrderIter {
            values: self.values.into_iter(),
            _allocation: self.allocation,
        }
    }
}
impl Iterator for OwnedOrderIter {
    type Item = ByRank;
    fn next(&mut self) -> Option<ByRank> {
        self.values.next()
    }
}
impl core::ops::Deref for OwnedOrder {
    type Target = [ByRank];
    fn deref(&self) -> &[ByRank] {
        &self.values
    }
}

/// The existing native BinaryHeap and strict ByRank law, with its actual Vec
/// backing pre-admitted. `from`/`into_vec` move that backing without copying it.
fn order_owned(
    candidates: AdmittedVec<Candidate>,
    keep: Option<usize>,
    workspace: &WorkspaceCapability,
) -> Result<OwnedOrder, TextError> {
    let wanted = keep.map_or(candidates.len(), |keep| keep.min(candidates.len()));
    let capacity = if keep.is_some_and(|keep| keep < candidates.len()) {
        wanted.checked_add(1).ok_or_else(overflow)?
    } else {
        wanted
    };
    let mut allocation = None;
    let mut values = Vec::new();
    admitted(workspace.reserve_vec(&mut values, &mut allocation, capacity))?;
    if wanted == 0 {
        return Ok(OwnedOrder { values, allocation });
    }
    let mut ordered = if keep.is_some() {
        let mut heap = BinaryHeap::from(values);
        for candidate in candidates {
            heap.push(ByRank(candidate));
            if heap.len() > wanted {
                heap.pop();
            }
        }
        heap.into_vec()
    } else {
        for candidate in candidates {
            values.push(ByRank(candidate));
        }
        values
    };
    ordered.sort_unstable();
    Ok(OwnedOrder {
        values: ordered,
        allocation,
    })
}

/// The best `keep` candidates, in rank order, through a heap of that size.
///
/// The heap holds at most `keep + 1` entries at any moment, so ranking a corpus
/// of a million documents for a ten-row answer touches eleven of them at a time
/// and sorts ten at the end — real work reduction, rather than a full sort that
/// stops reading its own output early.
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
        assert_eq!(distinct_terms(&needle), vec!["alpha", "beta", "gamma"]);
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

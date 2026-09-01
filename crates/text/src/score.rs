// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! BM25 ranking: exact scores, per-partition ranks, and a total order with no
//! ties in it.
//!
//! # The constants are constants
//!
//! [`K1`] and [`B`] are crate constants, not caller parameters. There is no
//! parameter struct, no builder and no knob, and that is a deliberate
//! restriction rather than an unfinished one.
//!
//! AGENTS.md states the rule plainly: *PurRDF is a carrier; optionality changes
//! semantics per consumer, which is forbidden.* A tuning parameter here would be
//! exactly that. Two callers pointing the same query at the same index would get
//! different scores and — worse, because it is the thing downstream actually
//! consumes — a different `?rank`, and neither answer would be identifiable as
//! the wrong one. Ranked retrieval's whole output is an order, so a knob that
//! changes the order changes the answer.
//!
//! This is **not** the "caller-supplied configuration" rule that governs
//! [`TextIndexConfig`](crate::TextIndexConfig)'s predicate IRIs. That rule exists
//! because PurRDF mints no vocabulary: an IRI PurRDF chose for itself would be a
//! term of somebody's ontology invented by a carrier, and would end up in an RDF
//! graph as data. A saturation constant from the retrieval literature is not a
//! vocabulary, ends up in no graph, and names nothing; the two rules point in
//! opposite directions here and it is worth saying which one applies.
//!
//! The values are the canonical ones from the BM25 literature — `k1 = 1.2` and
//! `b = 0.75`, the Okapi defaults that every published description of the
//! function uses when it does not say otherwise. Picking the literature's numbers
//! rather than inventing better ones is the point: they are the values a reader
//! can check this implementation against.
//!
//! # The formula
//!
//! ```text
//! score(D, Q) = Σ            IDF(t) · ( tf(t,D) · (k1 + 1) )
//!               t ∈ Q       ────────────────────────────────────────────
//!                            tf(t,D) + k1 · (1 − b + b · |D| / avgdl)
//!
//! IDF(t)      = ln( 1 + (N − df(t) + 1/2) / (df(t) + 1/2) )
//! ```
//!
//! `t` ranges over the **distinct** terms of the analyzed needle, visited in
//! sorted order. Every step runs in [`Fixed`], every step is checked, and an
//! intermediate that does not fit is a [`TextError::Overflow`] rather than a
//! wrapped or saturated number: a wrapped score is a wrong ranking presented as a
//! right one.
//!
//! `N`, `df` and `avgdl` are read from the **document's own partition** and never
//! pooled. `avgdl` is strictly positive for every partition an index retains,
//! because a document that analyzes to zero tokens is not retained at all (see
//! [`TextIndex`]'s zero-token invariant). That guarantee is what makes the
//! division by `avgdl` safe, so it is checked here as well as asserted there — a
//! `debug_assert` and a real [`TextError::Domain`] guard, so a later change to the
//! index cannot quietly reintroduce a division by zero.
//!
//! # `?rank` is per partition, and that is a correctness decision
//!
//! Corpus statistics are computed per `(graph, language)` partition
//! ([`PartitionKey`]), so a score is a number *relative to one corpus*. A rank
//! that spanned partitions would therefore sort together numbers computed against
//! different corpora — an English document's score against English statistics
//! beside a Japanese document's score against Japanese statistics — which is the
//! cross-corpus BM25 fallacy. The two numbers do not denote the same quantity,
//! and ordering them produces an arrangement rather than a ranking.
//!
//! So [`Scored::rank`] is the 1-based position of a document **within its own
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

/// The raw constants below are written at [`SCALE_DIGITS`] fractional digits, so
/// the scale and the literals cannot drift apart unnoticed.
const _: () = assert!(
    SCALE_DIGITS == 12,
    "the BM25 constants are spelled out at twelve fractional digits"
);

/// The BM25 term-frequency saturation constant, `k1 = 1.2`.
///
/// The canonical value from the BM25 literature, and a **constant rather than a
/// caller parameter**. See this module's documentation: PurRDF is a carrier, and
/// optionality that changes semantics per consumer is forbidden, so two callers
/// must not get different scores and different ranks out of the same index and
/// the same needle.
pub const K1: Fixed = Fixed::from_raw(1_200_000_000_000);

/// The BM25 document-length normalization constant, `b = 0.75`.
///
/// The canonical value from the BM25 literature, and a constant for the same
/// reason [`K1`] is.
pub const B: Fixed = Fixed::from_raw(750_000_000_000);

/// One half — the `1/2` of the inverse document frequency's two shifts.
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
    /// Never a global position. See this module's documentation for why a rank
    /// spanning partitions would order numbers that do not denote the same
    /// quantity.
    pub rank: u32,
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
        graph && language
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
/// Rows come back in rank order, `rank` running from `1`.
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
    rank_terms(index, partition, &terms, limit)
}

/// Rank every partition `filter` admits, and emit the rows in
/// `(partition key ASC, rank ASC)` order.
///
/// * `filter` is applied **before** ranking, which is sound because ranks are
///   per-partition.
/// * `ceiling` is the number of rows to emit — a `LIMIT` over the emission order
///   above. It bounds each partition's heap too, because a row at output position
///   `i` has rank at most `i + 1`, so nothing beyond rank `ceiling` in any
///   partition can reach the output.
/// * `rank` restricts the emission to that one 1-based per-partition position, so
///   it yields at most one row per admitted partition. It is what a bound `?rank`
///   compiles to, and it bounds each heap at `rank` entries. A `rank` of `0`, or
///   one past the end of every partition, yields nothing: ranks are 1-based, so
///   `0` names no row, and asking for a row a partition does not have is a
///   question with an honest empty answer rather than an error.
pub fn select(
    index: &TextIndex,
    needle: &[String],
    filter: &PartitionFilter,
    ceiling: Option<u64>,
    rank: Option<u32>,
) -> Result<Vec<Scored>, TextError> {
    let terms = distinct_terms(needle)?;
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    // A bound rank needs the heap deep enough to know that rank; otherwise the
    // ceiling is the only thing bounding it.
    let limit = rank.map_or(ceiling, |wanted| Some(u64::from(wanted)));
    let emitted = ceiling.map_or(usize::MAX, |k| usize::try_from(k).unwrap_or(usize::MAX));

    let mut rows: Vec<Scored> = Vec::new();
    for (key, _) in index.partitions() {
        if rows.len() >= emitted {
            break;
        }
        if !filter.matches(key) {
            continue;
        }
        let mut partition_rows = rank_terms(index, key, &terms, limit)?;
        if let Some(wanted) = rank {
            partition_rows.retain(|row| row.rank == wanted);
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
    let length = index
        .document_length(document)
        .ok_or_else(|| TextError::data(format!("document {document} has no recorded length")))?;

    let mut out = Vec::with_capacity(terms.len());
    for term in terms {
        let document_frequency = index.document_frequency(partition, term);
        let inverse_document_frequency =
            inverse_document_frequency(stats.document_count(), document_frequency)?;
        let term_frequency = index.term_frequency(document, term);
        let saturation = saturation(term_frequency, length, stats.average_document_length())?;
        out.push(TermContribution {
            term: term.to_owned(),
            term_frequency,
            document_frequency,
            inverse_document_frequency,
            contribution: inverse_document_frequency.checked_mul(saturation)?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// The arithmetic
// ---------------------------------------------------------------------------

/// `ln(1 + (N − df + 1/2) / (df + 1/2))`, entirely in fixed point.
///
/// Strictly positive for every `df` in range, including `df == N`: the numerator
/// is then `1/2`, the quotient is positive, and the logarithm's argument exceeds
/// one. So a term every document holds still contributes a small positive amount
/// rather than a negative one — which is the reason this shape of the inverse
/// document frequency is preferred over the unshifted `ln(N / df)`.
fn inverse_document_frequency(
    document_count: u64,
    document_frequency: u64,
) -> Result<Fixed, TextError> {
    debug_assert!(
        document_frequency <= document_count,
        "a term cannot occur in more documents than the partition holds"
    );
    if document_frequency > document_count {
        return Err(TextError::domain(format!(
            "document frequency {document_frequency} exceeds the partition's {document_count} \
             documents"
        )));
    }
    let count = from_count(document_count)?;
    let frequency = from_count(document_frequency)?;
    let numerator = count.checked_sub(frequency)?.checked_add(HALF)?;
    let denominator = frequency.checked_add(HALF)?;
    Fixed::ONE
        .checked_add(numerator.checked_div(denominator)?)?
        .ln()
}

/// `tf · (k1 + 1) / (tf + k1 · (1 − b + b · |D| / avgdl))`, entirely in fixed
/// point.
///
/// Exactly zero when `tf` is zero, because the numerator is: the denominator is
/// strictly positive whatever `tf` is, since `1 − b` is `0.25` and the length
/// ratio is non-negative. That is why a needle term the document does not hold
/// needs no special case in either the scorer or [`explain`].
fn saturation(
    term_frequency: u64,
    length: u64,
    average_document_length: Fixed,
) -> Result<Fixed, TextError> {
    // A tripwire, not the guard: the index does not retain a document that
    // analyzes to no tokens, so a non-positive average here means that invariant
    // was broken upstream, and a debug build should say so at the break rather
    // than return a plausible-looking failure from the arithmetic.
    debug_assert!(
        average_document_length > Fixed::ZERO,
        "a retained partition's average document length must be strictly positive"
    );

    let frequency = from_count(term_frequency)?;
    let numerator = frequency.checked_mul(K1.checked_add(Fixed::ONE)?)?;
    let relative = length_ratio(length, average_document_length)?;
    let normalization = Fixed::ONE
        .checked_sub(B)?
        .checked_add(B.checked_mul(relative)?)?;
    let denominator = frequency.checked_add(K1.checked_mul(normalization)?)?;
    numerator.checked_div(denominator)
}

/// `|D| / avgdl`, refusing an average that is not positive.
///
/// The guard that holds in every build, paired with [`saturation`]'s debug-only
/// tripwire. It exists so that a future change to the index's
/// zero-token-document exclusion cannot silently reintroduce a division by zero
/// in a release build: there is no length ratio against an empty corpus, so none
/// is invented.
fn length_ratio(length: u64, average_document_length: Fixed) -> Result<Fixed, TextError> {
    if average_document_length <= Fixed::ZERO {
        return Err(TextError::domain(
            "a partition reports an average document length that is not positive; the index's \
             zero-token-document exclusion is what guarantees it cannot be",
        ));
    }
    from_count(length)?.checked_div(average_document_length)
}

/// A corpus count as a fixed-point number, refusing rather than wrapping.
fn from_count(value: u64) -> Result<Fixed, TextError> {
    let value = i64::try_from(value)
        .map_err(|_| TextError::overflow(format!("{value} does not fit a fixed-point integer")))?;
    Fixed::from_integer(value)
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
fn distinct_terms(needle: &[String]) -> Result<Vec<&str>, TextError> {
    let mut terms: Vec<&str> = needle.iter().map(String::as_str).collect();
    terms.sort_unstable();
    terms.dedup();
    if u32::try_from(terms.len()).is_err() {
        return Err(TextError::data(format!(
            "the needle holds {} distinct terms, which exceeds the u32 space a matched-term count \
             addresses",
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
) -> Result<Vec<Scored>, TextError> {
    let candidates = candidates(index, partition, terms)?;
    let limit = limit.map(|value| usize::try_from(value).unwrap_or(usize::MAX));
    let ordered = match limit {
        Some(keep) => bounded(candidates, keep),
        None => sorted(candidates),
    };

    let mut rows = Vec::with_capacity(ordered.len());
    for (position, ByRank(candidate)) in ordered.into_iter().enumerate() {
        let rank = u32::try_from(position + 1).map_err(|_| {
            TextError::overflow("a partition holds more ranked rows than a u32 can number")
        })?;
        rows.push(Scored {
            document: candidate.document,
            score: candidate.score,
            rank,
            matched: candidate.matched,
        });
    }
    Ok(rows)
}

/// Every document of `partition` that holds at least one of `terms`, scored.
///
/// A document holding none of them is not a candidate. Its score would be a sum
/// over nothing, which is zero, and a zero-scoring row is not a retrieval result
/// — emitting one would make every document in the corpus a hit for every query.
fn candidates(
    index: &TextIndex,
    partition: &PartitionKey,
    terms: &[&str],
) -> Result<Vec<Candidate>, TextError> {
    let Some(stats) = index.partition_stats(partition).copied() else {
        // Not a partition this index holds, so it holds no documents there. That
        // is the true answer rather than a failure.
        return Ok(Vec::new());
    };

    let mut frequencies = Vec::with_capacity(terms.len());
    // `(document, term ordinal, term frequency)`. Sorting this groups the whole
    // working set by document while leaving each document's terms in the sorted
    // term order the sum is defined to run in.
    let mut occurrences: Vec<(u32, u32, u64)> = Vec::new();
    for (ordinal, term) in terms.iter().enumerate() {
        let document_frequency = index.document_frequency(partition, term);
        frequencies.push(inverse_document_frequency(
            stats.document_count(),
            document_frequency,
        )?);
        for (document, positions) in index.postings(partition, term) {
            occurrences.push((document, ordinal as u32, positions.len() as u64));
        }
    }
    occurrences.sort_unstable();

    let mut out: Vec<Candidate> = Vec::new();
    let mut at = 0;
    while at < occurrences.len() {
        let document = occurrences[at].0;
        let length = index.document_length(document).ok_or_else(|| {
            TextError::data(format!(
                "a posting names document {document}, which the index does not hold"
            ))
        })?;
        let mut score = Fixed::ZERO;
        let mut matched: u32 = 0;
        while at < occurrences.len() && occurrences[at].0 == document {
            let (_, ordinal, term_frequency) = occurrences[at];
            let saturation = saturation(term_frequency, length, stats.average_document_length())?;
            let contribution = frequencies[ordinal as usize].checked_mul(saturation)?;
            score = score.checked_add(contribution)?;
            matched += 1;
            at += 1;
        }
        out.push(Candidate {
            document,
            score,
            matched,
        });
    }
    Ok(out)
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
    use pretty_assertions::assert_eq;
    use purrdf_core::TermValue;

    use super::{
        B, Constraint, HALF, K1, PartitionFilter, distinct_terms, length_ratio, saturation,
    };
    use crate::error::TextError;
    use crate::fixed::Fixed;
    use crate::index::PartitionKey;

    /// The constants denote the literature's values, exactly.
    #[test]
    fn the_constants_are_the_canonical_ones() {
        assert_eq!(K1.to_decimal_lexical(), "1.200000000000");
        assert_eq!(B.to_decimal_lexical(), "0.750000000000");
        assert_eq!(HALF.to_decimal_lexical(), "0.500000000000");
    }

    /// A document of exactly average length gets a normalization factor of one,
    /// so its saturation reduces to `tf·(k1+1)/(tf+k1)` — the hand-checkable
    /// case every golden in this crate is built on.
    #[test]
    fn an_average_length_document_normalizes_to_one() {
        let average = Fixed::from_integer(4).expect("representable");
        assert_eq!(
            saturation(1, 4, average)
                .expect("finite")
                .to_decimal_lexical(),
            "1.000000000000",
            "tf = 1 at average length is 2.2/2.2"
        );
        assert_eq!(
            saturation(2, 4, average)
                .expect("finite")
                .to_decimal_lexical(),
            "1.375000000000",
            "tf = 2 at average length is 4.4/3.2"
        );
        assert_eq!(
            saturation(0, 4, average).expect("finite"),
            Fixed::ZERO,
            "a term the document does not hold contributes exactly zero"
        );
    }

    /// The zero-token invariant is guarded here as well as upheld in the index:
    /// an average of zero is refused rather than divided by, in **every** build.
    #[test]
    fn a_zero_average_document_length_is_a_domain_error() {
        let error = length_ratio(1, Fixed::ZERO).expect_err("a zero average has no length ratio");
        assert!(matches!(error, TextError::Domain(_)), "got {error:?}");
        let negative =
            length_ratio(1, Fixed::from_integer(-1).expect("representable")).expect_err("likewise");
        assert!(matches!(negative, TextError::Domain(_)), "got {negative:?}");
    }

    /// And a debug build trips at the break rather than returning a
    /// plausible-looking arithmetic failure from three frames away.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "average document length must be strictly positive")]
    fn a_zero_average_document_length_trips_the_debug_tripwire() {
        let _ = saturation(1, 1, Fixed::ZERO);
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

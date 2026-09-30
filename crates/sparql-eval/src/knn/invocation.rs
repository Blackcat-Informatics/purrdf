// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The nearest-neighbour **call protocol**: everything a ranked relation over a space
//! of RDF-term-named rows does that does not depend on how it ranks.
//!
//! A nearest-neighbour relation — the exact scan in this module, the HNSW graph in
//! `purrdf-hnsw` — answers one call shape, `?neighbour rel: (?query k ?distance)`, under
//! the same two modes, with the same refusals, the same row layout and the same
//! licence discipline. The only thing that differs between them is the ranking itself:
//! which candidates a search examines, and what one pairwise distance costs. So the
//! protocol lives here once, and a relation supplies a [`RankSource`]:
//!
//! * [`TermRows`] — the row↔term correspondence and its binding proofs;
//! * [`KnnInvocation::open`] — reading one call's arguments into the question it asks;
//! * [`RankedCursor`] — the lazy cursor that ranks on first pull, filters every bound
//!   position, spends the engine's licence only on emitted rows and reports the work.
//!
//! Two relations with two copies of this protocol could drift apart on a refusal or
//! on which rows a licence may cut, and the ranked declarations they publish promise a
//! consumer the same semantics — so there is one body.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::{TargetId, TargetSetView, TermValue};
use purrdf_xsd::datatype::XSD_DOUBLE;

use crate::error::EvalError;
use crate::property_fn::{IndexGeneration, PfArgs, PfArity, PfCursor, PfRow};

use super::{KnnGuard, Ranked};

/// The `?neighbour` position: the retrieved term.
pub const KNN_NEIGHBOUR: usize = 0;
/// The `?query` position: the term whose vector seeds the search. Always an input.
pub const KNN_QUERY: usize = 1;
/// The `k` position: how many neighbours to retrieve. Always an input of a ranked read.
pub const KNN_COUNT: usize = 2;
/// The `?distance` position: the retrieved term's distance from the query.
pub const KNN_DISTANCE: usize = 3;
/// The call shape of every nearest-neighbour relation: `?neighbour` on the subject
/// side, `(?query k ?distance)` on the object side.
pub const KNN_ARITY: PfArity = PfArity::new(1, 3);
/// The general access pattern: the seed and the neighbour count are the two positions
/// a ranking cannot enumerate, and the two it projects are free.
pub const KNN_MODE: &str = "fbbf";
/// The **membership** access pattern: the neighbour and the seed bound, and the
/// neighbour count **free**. It asks *do you hold this term at all*, a question `k` is
/// no part of; [`KNN_MODE`] binds the count this leaves free, so it does not subsume
/// this pattern.
pub const KNN_MEMBERSHIP_MODE: &str = "bbff";

/// The rows of a space and the RDF term each one stands for, with the term-to-row
/// lookup a query seed needs.
///
/// Terms are distinct by construction, so [`Self::row_of`] is single-valued. The
/// lookup is a sorted row vector rather than a hash map because [`TermValue`] is `Ord`
/// and not `Hash`, and a binary search over canonical term order is deterministic
/// without a hasher having to be chosen.
#[derive(Debug, Clone)]
pub struct TermRows {
    /// Row `r`'s RDF term.
    terms: Vec<TermValue>,
    /// Row numbers ordered by their term.
    rows_by_term: Vec<usize>,
}

impl TermRows {
    /// Rows `0..terms.len()`, row `r` standing for `terms[r]`.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] when one term is claimed by two rows: a query seed names a
    /// term, so a term with two rows makes the seed ambiguous. The duplicate reported is
    /// the least in canonical term order, the same one on every run.
    pub fn new(terms: Vec<TermValue>) -> Result<Self, EvalError> {
        let mut rows_by_term: Vec<usize> = (0..terms.len()).collect();
        rows_by_term.sort_unstable_by(|&left, &right| terms[left].cmp(&terms[right]));
        if let Some(pair) = rows_by_term
            .windows(2)
            .find(|pair| terms[pair[0]] == terms[pair[1]])
        {
            return Err(EvalError::config(format!(
                "the term {:?} is bound to two different rows; a query seed names a term, so \
                 a term claimed by two rows makes the seed ambiguous",
                terms[pair[0]]
            )));
        }
        Ok(Self {
            terms,
            rows_by_term,
        })
    }

    /// Place each binding at its target's row in `set`, proving the cover is exact:
    /// every binding names a target the set holds, no target is bound twice, every row
    /// is bound, and no term is bound to two rows.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] naming the first binding or row that breaks the cover. A
    /// row without a term is a refusal, not a skipped row: a space with an unnamed row
    /// would search every candidate and be able to report only some of them.
    pub fn bind(
        set: &TargetSetView<'_>,
        bindings: Vec<(TargetId, TermValue)>,
    ) -> Result<Self, EvalError> {
        let row_count = set.row_count();
        let mut terms: Vec<Option<TermValue>> = vec![None; row_count];
        for (target, term) in bindings {
            let row = set.row_for_target(target).ok_or_else(|| {
                EvalError::config(format!(
                    "the binding for target {target} names a target this space's target set \
                     does not hold"
                ))
            })?;
            if terms[row].is_some() {
                return Err(EvalError::config(format!(
                    "target {target} (row {row}) is bound twice; a row stands for exactly one \
                     RDF term"
                )));
            }
            terms[row] = Some(term);
        }
        let mut bound: Vec<TermValue> = Vec::with_capacity(row_count);
        for (row, term) in terms.into_iter().enumerate() {
            let term = term.ok_or_else(|| {
                EvalError::config(format!(
                    "row {row} (target {}) has no bound RDF term; a space with an unnamed row \
                     would search {row_count} candidates and be able to report only some of \
                     them, so the top-k it returned would silently be the top-k of a subset",
                    set.target(row)
                        .map_or_else(|| "<unknown>".to_owned(), TargetId::to_hex)
                ))
            })?;
            bound.push(term);
        }
        Self::new(bound)
    }

    /// Every row's term, in row order.
    #[must_use]
    pub fn terms(&self) -> &[TermValue] {
        &self.terms
    }

    /// How many rows there are.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.terms.len()
    }

    /// Whether there are no rows.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// The RDF term row `row` stands for.
    #[must_use]
    pub fn term(&self, row: usize) -> Option<&TermValue> {
        self.terms.get(row)
    }

    /// The row `term` occupies, if there is one.
    #[must_use]
    pub fn row_of(&self, term: &TermValue) -> Option<usize> {
        self.rows_by_term
            .binary_search_by(|&row| self.terms[row].cmp(term))
            .ok()
            .map(|at| self.rows_by_term[at])
    }
}

/// What one invocation is going to do, decided from the access pattern it arrived in.
///
/// Two shapes rather than one with an optional field, because they are two different
/// questions: one ranks a neighbourhood and the other asks whether a named term has a
/// row at all. Naming them apart is what makes "the ranking was not entered" a branch a
/// reader can see rather than a condition buried in a selection size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnnAnswer {
    /// Rank the seed's neighbourhood: the offer of candidates.
    Search {
        /// The seed's row, or `None` when the space does not hold the seed term.
        query_row: Option<usize>,
        /// How many neighbours the ranking retains — `k`, or the engine's ceiling when
        /// it was safe to push it down.
        select_k: usize,
    },
    /// Answer *do you hold this candidate*: the seed's row and the candidate's row, or
    /// `None` when the space holds no row for one of them — which is equally an
    /// exclusion, because a search from a seed with no row names nothing at all.
    Membership(Option<(usize, usize)>),
}

/// One nearest-neighbour call, read off its arguments: the question it asks and the
/// values every emitted row echoes or is filtered against.
#[derive(Debug, Clone)]
pub struct KnnInvocation {
    /// What the invocation is going to do.
    pub answer: KnnAnswer,
    /// The seed term, echoed verbatim into position 1 of every row.
    pub query_term: TermValue,
    /// The value position 2 carries: the requested count, echoed verbatim, or under
    /// membership the size of the term universe ([`universe_size`]).
    pub count_term: TermValue,
    /// The invocation's bound values by flattened position (`None` = free).
    pub bound: Vec<Option<TermValue>>,
    /// The rows the invocation may emit under the engine's licence.
    pub remaining: Option<u64>,
}

impl KnnInvocation {
    /// Read one call of `relation` (named in refusals, e.g. `"the HNSW relation"`) over
    /// `rows`, under `guard` and the engine's `ceiling`, counting a membership lookup on
    /// `membership_lookups`.
    ///
    /// **The count decides which question this is, and nothing else does.** A bound count
    /// is the ranked read, down to the `k` cut on a bound `?neighbour`; a free count with
    /// a bound `?neighbour` is the membership lookup. Branching on the candidate instead
    /// would make one binding pattern mean two questions and silently widen the answer to
    /// `?n rel: (?q k ?d)` — a query nobody edited.
    ///
    /// `?neighbour` and `?distance` are both filtered after the ranking, so the ceiling
    /// is withheld from the selection when either is bound: pushing it down would rank a
    /// prefix, let the cursor filter it, and report fewer rows than the engine asked for
    /// as an exhausted answer.
    ///
    /// A seed the space does not hold is an honest empty answer, not a refusal.
    ///
    /// # Errors
    ///
    /// [`EvalError::Function`] when the call is not [`KNN_ARITY`], the seed is free, the
    /// count and the neighbour are both free, or the count is not an admissible `k`
    /// ([`neighbour_count`]).
    pub fn open(
        relation: &str,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        rows: &TermRows,
        guard: KnnGuard,
        membership_lookups: &AtomicU64,
    ) -> Result<Self, EvalError> {
        let supplied = args.arity();
        if supplied != KNN_ARITY {
            return Err(EvalError::function(format!(
                "{relation} expects {KNN_ARITY} argument(s), got {supplied}"
            )));
        }
        let Some(query) = args.get(KNN_QUERY) else {
            return Err(EvalError::function(format!(
                "the query term at position {KNN_QUERY} is free; this relation retrieves the \
                 neighbours of a seed and cannot enumerate seeds, which is why both of its \
                 declared modes — `{KNN_MODE}` and `{KNN_MEMBERSHIP_MODE}` — demand it"
            )));
        };
        let query_row = rows.row_of(query);
        let (answer, count_term) = if let Some(count) = args.get(KNN_COUNT) {
            let k = neighbour_count(count, guard)?;
            let post_selection_filtered =
                args.get(KNN_NEIGHBOUR).is_some() || args.get(KNN_DISTANCE).is_some();
            let select_k = if post_selection_filtered {
                k
            } else {
                ceiling.map_or(k, |ceiling| k.min(usize::try_from(ceiling).unwrap_or(k)))
            };
            (
                KnnAnswer::Search {
                    query_row,
                    select_k,
                },
                count.clone(),
            )
        } else {
            let Some(candidate) = args.get(KNN_NEIGHBOUR) else {
                return Err(EvalError::function(format!(
                    "the neighbour count at position {KNN_COUNT} is free and so is the \
                     neighbour at position {KNN_NEIGHBOUR}; how many neighbours to retrieve \
                     is a question this relation is asked, not one it answers, so the only \
                     call it serves without a count is the membership lookup \
                     `{KNN_MEMBERSHIP_MODE}`, which names the one term it is about"
                )));
            };
            membership_lookups.fetch_add(1, Ordering::Relaxed);
            (
                KnnAnswer::Membership(query_row.zip(rows.row_of(candidate))),
                universe_size(rows.len()),
            )
        };
        Ok(Self {
            answer,
            query_term: query.clone(),
            count_term,
            bound: args.flattened().map(<Option<&TermValue>>::cloned).collect(),
            remaining: ceiling,
        })
    }
}

/// Read `k` off the invocation's neighbour-count argument.
///
/// # Errors
///
/// [`EvalError::Function`] when the value is not an integer literal, is negative,
/// exceeds the guard's `max_neighbours` (a clamp would be a short answer reported as a
/// complete one), or does not fit this platform's index range.
pub fn neighbour_count(value: &TermValue, guard: KnnGuard) -> Result<usize, EvalError> {
    let Some(purrdf_xsd::XsdValue::Integer { value: count, .. }) = crate::expr::xsd_of(value)
    else {
        return Err(EvalError::function(format!(
            "the neighbour count at position {KNN_COUNT} is {value:?}, which is not an integer \
             literal; there is no number of neighbours that names"
        )));
    };
    if count < 0 {
        return Err(EvalError::function(format!(
            "the neighbour count at position {KNN_COUNT} is {count}; a search cannot return a \
             negative number of neighbours"
        )));
    }
    let bound = i128::from(guard.max_neighbours());
    if count > bound {
        return Err(EvalError::function(format!(
            "the invocation asks for {count} neighbour(s), and the configured guard admits at \
             most {bound}; returning the {bound} nearest instead would be a short answer \
             reported as a complete one, so the request is refused rather than clamped"
        )));
    }
    usize::try_from(count).map_err(|_| {
        EvalError::function(format!(
            "the neighbour count {count} does not fit this platform's index range"
        ))
    })
}

/// The value the count position carries in a **membership** answer: the number of rows
/// the space holds, as an `xsd:integer`.
///
/// A [`PfRow`] carries a value for every flattened position, so a mode that leaves the
/// count free must still fill it. Under [`KNN_MEMBERSHIP_MODE`] there is no request to
/// echo, so the position is an output, and what it outputs is a fact about the producer
/// rather than a request it was never given: the size of its term universe. It is
/// single-valued, so the mode's declared row bound of one is exact, and it is
/// emphatically **not** a fabricated `k` — a point lookup claims no rank.
#[must_use]
pub fn universe_size(rows: usize) -> TermValue {
    TermValue::integer(rows as u64)
}

/// How a relation ranks: the one part of the call protocol that differs between an
/// exhaustive scan and an index traversal.
///
/// Each method records its own observations (how many searches ran, how many candidates
/// they examined), so a counter a host reads is a measurement of the calls actually
/// made.
pub trait RankSource {
    /// Rank the neighbourhood of `query_row`, retaining `select_k` rows nearest first,
    /// and report how many candidates the ranking examined.
    ///
    /// # Errors
    ///
    /// Whatever the ranking refuses; the error aborts the query.
    fn search(&self, query_row: usize, select_k: usize) -> Result<(Vec<Ranked>, u64), EvalError>;

    /// The distance between two rows: one pairwise evaluation, charged as the one
    /// candidate it examines.
    ///
    /// # Errors
    ///
    /// Whatever the evaluation refuses; the error aborts the query.
    fn distance(&self, query_row: usize, row: usize) -> Result<f64, EvalError>;

    /// The RDF term row `row` stands for.
    fn term(&self, row: usize) -> Option<&TermValue>;

    /// The generation of the space that answers. The space is immutable once built, so
    /// the reading is true for every row a cursor goes on to emit.
    fn generation(&self) -> Arc<str>;
}

/// The cursor a nearest-neighbour relation's `open` returns: the ranked neighbours,
/// filtered on every bound position, cut at the engine's licence, and reporting the
/// ranking's work.
///
/// # Why the ranking is lazy
///
/// It runs on the first [`PfCursor::next`], not in `open`. The engine checks its own
/// ceiling before pulling, so a call whose ceiling is already exhausted never pulls at
/// all and is charged nothing: "no rows were wanted" and "no work was done" are the same
/// statement. This is the **only** caller of [`RankSource::search`] and
/// [`RankSource::distance`], which is what makes a source's counters measurements rather
/// than estimates.
///
/// # The two properties that make the licence sound
///
/// * It filters on **every** bound position, including `?neighbour` and `?distance`,
///   which the ranking cannot see.
/// * It decrements the licence only on rows it actually **emits**; a skipped row
///   disagrees with a bound position and the engine would have dropped it anyway.
#[derive(Debug)]
pub struct RankedCursor<S> {
    /// How this relation ranks.
    source: S,
    /// The call being answered.
    invocation: KnnInvocation,
    /// The ranked neighbours, once the ranking has run.
    ranked: Option<Vec<Ranked>>,
    /// How far into `ranked` this cursor has read.
    at: usize,
    /// Candidates examined and not yet reported to the governor.
    unreported_work: u64,
}

impl<S: RankSource> RankedCursor<S> {
    /// A cursor answering `invocation` through `source`. Nothing is ranked until the
    /// first pull.
    #[must_use]
    pub const fn new(source: S, invocation: KnnInvocation) -> Self {
        Self {
            source,
            invocation,
            ranked: None,
            at: 0,
            unreported_work: 0,
        }
    }

    /// Rank, if this invocation has not ranked yet, recording the work examined.
    fn ensure_ranked(&mut self) -> Result<&[Ranked], EvalError> {
        if self.ranked.is_none() {
            let (ranked, work) = match self.invocation.answer {
                KnnAnswer::Search {
                    query_row: Some(row),
                    select_k,
                } => self.source.search(row, select_k)?,
                KnnAnswer::Search {
                    query_row: None, ..
                }
                | KnnAnswer::Membership(None) => (Vec::new(), 0),
                KnnAnswer::Membership(Some((query_row, row))) => {
                    let distance = self.source.distance(query_row, row)?;
                    (vec![Ranked { distance, row }], 1)
                }
            };
            self.unreported_work = self.unreported_work.saturating_add(work);
            self.ranked = Some(ranked);
        }
        Ok(self.ranked.as_deref().unwrap_or_default())
    }

    /// The full row for one ranked neighbour.
    fn build(&self, scored: Ranked) -> Result<PfRow, EvalError> {
        let neighbour = self.source.term(scored.row).ok_or_else(|| {
            EvalError::data(format!(
                "the ranking named row {}, which the space does not hold",
                scored.row
            ))
        })?;
        Ok(vec![
            neighbour.clone(),
            self.invocation.query_term.clone(),
            self.invocation.count_term.clone(),
            TermValue::typed_literal(
                purrdf_xsd::numeric::canonical_double(scored.distance),
                XSD_DOUBLE,
            ),
        ])
    }
}

impl<S: RankSource> PfCursor for RankedCursor<S> {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.invocation.remaining == Some(0) {
            return Ok(None);
        }
        let ranked_len = self.ensure_ranked()?.len();
        while self.at < ranked_len {
            let scored = self.ranked.as_deref().unwrap_or_default()[self.at];
            self.at += 1;
            let row = self.build(scored)?;
            let agrees = self
                .invocation
                .bound
                .iter()
                .zip(row.iter())
                .all(|(want, have)| want.as_ref().is_none_or(|want| want == have));
            if agrees {
                if let Some(remaining) = self.invocation.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row));
            }
        }
        Ok(None)
    }

    /// One unit per **candidate examined** — one distance computation against one row.
    ///
    /// This is the quantity a nearest-neighbour search's cost is proportional to, and it
    /// is invisible from outside: the rows returned are `k`, and `k` says nothing about
    /// the size of the space they were selected from.
    fn take_work(&mut self) -> u64 {
        core::mem::take(&mut self.unreported_work)
    }

    /// The generation of the space answering, shared rather than copied: attesting it is
    /// a refcount bump, not a fresh copy of the digest.
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Declared(self.source.generation())
    }
}

#[cfg(test)]
mod tests {
    use purrdf_xsd::datatype::{XSD_INT, XSD_INTEGER, XSD_NON_NEGATIVE_INTEGER, XSD_STRING};

    use super::*;

    fn iri(n: usize) -> TermValue {
        TermValue::iri(format!("https://example.org/doc/{n}"))
    }

    fn guard() -> KnnGuard {
        KnnGuard::new(100, 5).expect("valid")
    }

    #[test]
    fn term_rows_find_every_term_and_nothing_else() {
        let rows = TermRows::new(vec![iri(2), iri(0), iri(1)]).expect("distinct");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.row_of(&iri(2)), Some(0));
        assert_eq!(rows.row_of(&iri(0)), Some(1));
        assert_eq!(rows.row_of(&iri(1)), Some(2));
        assert_eq!(rows.row_of(&iri(3)), None);
        assert_eq!(rows.term(1), Some(&iri(0)));
        assert_eq!(rows.term(3), None);
        assert!(TermRows::new(Vec::new()).expect("empty").is_empty());
    }

    #[test]
    fn a_term_bound_to_two_rows_is_refused_and_distinct_terms_are_not() {
        let error = TermRows::new(vec![iri(0), iri(1), iri(0)]).expect_err("duplicate");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(
            error.to_string().contains("bound to two different rows"),
            "got {error}"
        );
        assert!(TermRows::new(vec![iri(0), iri(1), iri(2)]).is_ok());
    }

    #[test]
    fn a_neighbour_count_is_any_integer_literal_within_the_guard() {
        for datatype in [XSD_INTEGER, XSD_INT, XSD_NON_NEGATIVE_INTEGER] {
            let count = TermValue::typed_literal("5", datatype);
            assert_eq!(neighbour_count(&count, guard()).expect(datatype), 5);
        }
        assert_eq!(
            neighbour_count(&TermValue::integer(0), guard()).expect("zero"),
            0
        );
    }

    #[test]
    fn a_neighbour_count_that_names_no_admissible_k_is_refused() {
        for (count, why) in [
            (TermValue::typed_literal("5", XSD_STRING), "not an integer"),
            (
                TermValue::typed_literal("five", XSD_INTEGER),
                "not an integer",
            ),
            (TermValue::integer(-1), "negative"),
            (TermValue::integer(6), "admits at most 5"),
        ] {
            let error = neighbour_count(&count, guard()).expect_err(why);
            assert!(matches!(error, EvalError::Function(_)), "got {error:?}");
            assert!(error.to_string().contains(why), "{why}: got {error}");
        }
    }

    #[test]
    fn the_membership_count_is_the_size_of_the_term_universe() {
        assert_eq!(universe_size(7), TermValue::integer(7));
    }
}

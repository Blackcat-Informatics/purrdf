// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The approximate relation this crate exposes through the evaluator's
//! property-function seam.
//!
//! [`HnswSpace`] is the frozen, queryable form of one HNSW index: its graph, the RDF term
//! each row stands for, the metric the matrix declares, and the search bounds a host puts
//! on one invocation. [`HnswRelation`] is the [`PropertyFunction`] that makes it callable,
//! and [`register_hnsw_relation`] wires it into a
//! [`PropertyFunctionRegistry`] under a **caller-supplied** predicate IRI. PurRDF mints no
//! IRI here: the predicate is the caller's, and the `example.org` IRIs in the tests are
//! fixtures, not defaults.
//!
//! # The approximation contract, in the query surface
//!
//! The approximation contract declares three governed channels; the first is the guard's
//! [`IndexLossContract`](purrdf_core::IndexLossContract) and evidence string, checked by
//! [`crate::guard::load`]. The other two live here:
//!
//! * the relation is registered under the caller's predicate IRI, so the query text itself
//!   names the provider and no namespace is fabricated;
//! * a result is an **offer of candidates**, never a certification of absence. This
//!   relation's cursor can say "here are `k` candidates"; it can never say "no row is
//!   nearer". `crates/hnsw/tests/oracle_contract.rs` asserts that even when a query misses
//!   the exact oracle's nearest row, the empty and non-empty answers are ordinary cursor
//!   results, never a completeness claim.
//!
//! # The call shape
//!
//! Four flattened positions, the same shape as the exact kNN relation so the two are
//! interchangeable in query text:
//!
//! | position | name | role |
//! |---|---|---|
//! | 0 | `?neighbour` | **out**: the RDF term whose vector was retrieved |
//! | 1 | `?query` | **in**: the term whose vector seeds the search |
//! | 2 | `k` | **in**: how many neighbours to retrieve |
//! | 3 | `?distance` | **out**: the distance, as `xsd:double` |
//!
//! The one declared mode is `fbbf`: positions 1 and 2 are inputs this relation cannot
//! enumerate. Rows are emitted in rank order, nearest first, under the shared exact path's
//! [`Ranked`] order `(distance, row)`.
//!
//! # Work accounting
//!
//! The cursor reports one unit per candidate distance actually evaluated, through
//! [`PfCursor::take_work`]. The search is lazy — it runs on the first pull, not in
//! [`PropertyFunction::open`] — so a call whose ceiling is already exhausted performs no
//! work and is charged none, and the count resets only when the engine takes it.

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{
    EmbeddingView, TargetId, TargetSetId, TargetSetView, TermValue, VectorSpaceId, verify_embedding,
};
use purrdf_sparql_eval::{
    EvalError, Kernel, KnnGuard, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
    PropertyFunctionRegistry, Ranked, Volatility,
};

use crate::{HnswIndex, profile};

/// The `?neighbour` position: the retrieved term.
const HNSW_NEIGHBOUR: usize = 0;
/// The `?query` position: the term whose vector seeds the search. Always an input.
const HNSW_QUERY: usize = 1;
/// The `k` position: how many neighbours to retrieve. Always an input.
const HNSW_COUNT: usize = 2;
/// The `?distance` position: the retrieved term's distance from the query.
const HNSW_DISTANCE: usize = 3;
/// The one access pattern [`HnswRelation`] declares.
const HNSW_MODE: &str = "fbbf";

/// `xsd:double`, the datatype every emitted distance carries.
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
/// `xsd:integer`, the only datatype a neighbour count may carry.
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

// ---------------------------------------------------------------------------
// The space
// ---------------------------------------------------------------------------

/// One queryable HNSW index: its graph, its row terms, and the work bounds a host sets.
///
/// Construction is where everything that could otherwise fail mid-query is proven: the
/// artifact verifies, the unique HNSW guard's profile and coordinates agree with the
/// searched matrix, the payload commitment holds, the payload decodes, and every row has a
/// distinct RDF term. An invocation therefore fails only on things about the invocation.
#[derive(Debug, Clone)]
pub struct HnswSpace {
    /// The decoded graph.
    index: Arc<HnswIndex>,
    /// Row `r`'s RDF term, in the artifact's canonical (ascending `TargetId`) row order.
    terms: Vec<TermValue>,
    /// Row numbers ordered by their term, for the seed lookup.
    rows_by_term: Vec<usize>,
    /// The bounds one invocation is held to.
    guard: KnnGuard,
    /// The approximation evidence the profile publishes.
    evidence: String,
}

impl HnswSpace {
    /// Assemble a space directly from a decoded index and its row terms.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] if the term count differs from the graph's row count, or one
    /// term is claimed by two rows.
    pub fn from_index(
        index: HnswIndex,
        terms: Vec<TermValue>,
        guard: KnnGuard,
    ) -> Result<Self, EvalError> {
        if terms.len() != index.rows() {
            return Err(EvalError::config(format!(
                "the HNSW graph holds {} row(s) but {} term(s) were bound; every row must \
                 stand for exactly one RDF term",
                index.rows(),
                terms.len()
            )));
        }
        check_distinct_terms(&terms)?;
        let mut rows_by_term: Vec<usize> = (0..terms.len()).collect();
        rows_by_term.sort_unstable_by(|&left, &right| terms[left].cmp(&terms[right]));
        Ok(Self {
            index: Arc::new(index),
            terms,
            rows_by_term,
            guard,
            evidence: profile::LOSS_EVIDENCE.to_owned(),
        })
    }

    /// Build a queryable space from a PURREMB artifact's `(target_set, vector_space)` HNSW
    /// index.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] for a caller mistake — a space, target set or effective
    /// matrix the artifact does not hold, a binding outside the set, a duplicate target or
    /// term, a row no binding covers, a space larger than the guard admits, or an
    /// unsupported metric.
    ///
    /// [`EvalError::Data`] for a defect in the artifact itself — verification failure, a
    /// missing or ambiguous HNSW guard, a stale or substituted guard, a payload whose
    /// commitment fails, an unreadable row, or a decoded kernel that disagrees with the
    /// family contract.
    pub fn from_artifact(
        artifact: &[u8],
        target_set: TargetSetId,
        vector_space: VectorSpaceId,
        bindings: Vec<(TargetId, TermValue)>,
        guard: KnnGuard,
    ) -> Result<Self, EvalError> {
        let mut view = EmbeddingView::from_bytes(artifact)
            .map_err(|e| EvalError::data(format!("the PURREMB artifact is unreadable: {e}")))?;
        verify_embedding(&mut view)
            .map_err(|e| EvalError::data(format!("the PURREMB artifact does not verify: {e}")))?;

        let space = view.vector_space(vector_space).ok_or_else(|| {
            EvalError::config(format!(
                "the artifact declares no vector space {vector_space}"
            ))
        })?;
        let set = view.target_set(target_set).ok_or_else(|| {
            EvalError::config(format!("the artifact declares no target set {target_set}"))
        })?;
        let effective = view
            .effective_matrix(target_set, vector_space)
            .map_err(|e| EvalError::data(format!("the effective matrix is unreadable: {e}")))?
            .ok_or_else(|| {
                EvalError::config(format!(
                    "the artifact holds no matrix joining target set {target_set} to vector \
                     space {vector_space}"
                ))
            })?;

        // The typed adapter: exactly one HNSW guard, its profile and coordinates checked
        // against the matrix being searched, and its payload commitment verified.
        let guard_view =
            crate::guard::select(&view).map_err(|e| EvalError::data(format!("{e}")))?;
        crate::guard::check_coordinates(&guard_view, target_set, vector_space, &effective)
            .map_err(|e| EvalError::data(format!("{e}")))?;
        crate::guard::validate_guard(&guard_view).map_err(|e| EvalError::data(format!("{e}")))?;

        let family = view.family(space.family_id()).ok_or_else(|| {
            EvalError::data(format!(
                "vector space {vector_space} names family {}, which the artifact does not hold",
                space.family_id()
            ))
        })?;
        let metric = family.metric().map_err(|e| {
            EvalError::data(format!("the family's declared metric is unusable: {e}"))
        })?;
        let kernel = Kernel::of(&metric).ok_or_else(|| {
            EvalError::config(format!(
                "vector space {vector_space} declares the caller-defined distance metric \
                 {metric:?}, whose parameters are opaque bytes this engine cannot evaluate"
            ))
        })?;

        let row_count = set.row_count();
        if row_count as u64 > guard.max_candidates() {
            return Err(EvalError::config(format!(
                "this space holds {row_count} row(s), which is more than the {} candidate(s) \
                 the configured guard admits",
                guard.max_candidates()
            )));
        }

        let matrix = crate::guard::read_effective_matrix(&effective)
            .map_err(|e| EvalError::data(format!("the effective matrix is unreadable: {e}")))?;
        let index = crate::guard::load(&guard_view, matrix)
            .map_err(|e| EvalError::data(format!("the HNSW payload is unusable: {e}")))?;
        if index.kernel() != kernel {
            return Err(EvalError::data(format!(
                "the HNSW payload was built under a different distance kernel than the \
                 family contract declares ({kernel:?})"
            )));
        }
        if index.dims() != space.dimension() as usize {
            return Err(EvalError::data(format!(
                "the HNSW payload indexes {} dimension(s) but the vector space declares {}",
                index.dims(),
                space.dimension()
            )));
        }

        let terms = bind_terms(&set, bindings)?;
        Self::from_index(index, terms, guard)
    }

    /// How many candidate rows this space holds.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.terms.len()
    }

    /// The effective dimension of every vector in this space.
    #[must_use]
    pub fn dimension(&self) -> usize {
        self.index.dims()
    }

    /// The work bounds one invocation over this space is held to.
    #[must_use]
    pub const fn guard(&self) -> KnnGuard {
        self.guard
    }

    /// The approximation evidence the profile publishes.
    #[must_use]
    pub fn evidence(&self) -> &str {
        &self.evidence
    }

    /// The decoded index every invocation searches.
    #[must_use]
    pub fn index(&self) -> &HnswIndex {
        &self.index
    }

    /// The RDF term row `row` stands for.
    #[must_use]
    pub fn term(&self, row: usize) -> Option<&TermValue> {
        self.terms.get(row)
    }

    /// The row `term` occupies, if this space holds it.
    #[must_use]
    pub fn row_of(&self, term: &TermValue) -> Option<usize> {
        self.rows_by_term
            .binary_search_by(|&row| self.terms[row].cmp(term))
            .ok()
            .map(|at| self.rows_by_term[at])
    }

    /// A relation over this space.
    #[must_use]
    pub fn relation(self: &Arc<Self>) -> HnswRelation {
        HnswRelation::new(Arc::clone(self))
    }
}

/// Place each binding at its target's row, proving the cover is exact.
fn bind_terms(
    set: &TargetSetView<'_>,
    bindings: Vec<(TargetId, TermValue)>,
) -> Result<Vec<TermValue>, EvalError> {
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
    let mut bound = Vec::with_capacity(row_count);
    for (row, term) in terms.into_iter().enumerate() {
        let term = term.ok_or_else(|| {
            EvalError::config(format!(
                "row {row} has no bound RDF term; a space with an unnamed row would search \
                 {row_count} candidates and be able to report only some of them",
            ))
        })?;
        bound.push(term);
    }
    check_distinct_terms(&bound)?;
    Ok(bound)
}

/// Prove no term is claimed by two rows.
fn check_distinct_terms(terms: &[TermValue]) -> Result<(), EvalError> {
    let mut ordered: Vec<&TermValue> = terms.iter().collect();
    ordered.sort_unstable();
    if let Some([duplicate, _]) = ordered.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(EvalError::config(format!(
            "the term {duplicate:?} is bound to two different rows; a query seed names a \
             term, so a term claimed by two rows makes the seed ambiguous"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The relation
// ---------------------------------------------------------------------------

/// The [`PropertyFunction`] a host registers to make an [`HnswSpace`] queryable.
///
/// [`Volatility::Stable`]: the graph is frozen and every distance is a pure function of two
/// of its rows computed by a shared exact-path kernel, so an invocation's rows are the same
/// on the main thread, on a fork-join worker, and on `wasm32-unknown-unknown`.
#[derive(Debug, Clone)]
pub struct HnswRelation {
    /// The space every invocation searches.
    space: Arc<HnswSpace>,
    /// The single declared mode, materialized once so [`PropertyFunction::modes`] can
    /// hand out a slice.
    modes: [BindingPattern; 1],
}

impl HnswRelation {
    /// An approximate nearest-neighbour relation over `space`.
    #[must_use]
    pub fn new(space: Arc<HnswSpace>) -> Self {
        Self {
            space,
            modes: [BindingPattern::from_code(HNSW_MODE)],
        }
    }

    /// The space this relation searches.
    #[must_use]
    pub fn space(&self) -> &HnswSpace {
        &self.space
    }
}

impl PropertyFunction for HnswRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 3)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        let rows = self.space.row_count() as u64;
        if mode.is_bound(HNSW_NEIGHBOUR) {
            rows.min(1)
        } else {
            rows.min(self.space.guard().max_neighbours())
        }
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let declared = self.arity();
        let supplied = args.arity();
        if supplied != declared {
            return Err(EvalError::function(format!(
                "the HNSW relation expects {declared} argument(s), got {supplied}"
            )));
        }

        let Some(query) = args.get(HNSW_QUERY) else {
            return Err(EvalError::function(format!(
                "the query term at position {HNSW_QUERY} is free; this relation retrieves \
                 the neighbours of a seed and cannot enumerate seeds, which is why its only \
                 declared mode is `{HNSW_MODE}`"
            )));
        };
        let Some(count) = args.get(HNSW_COUNT) else {
            return Err(EvalError::function(format!(
                "the neighbour count at position {HNSW_COUNT} is free; how many neighbours \
                 to retrieve is a question this relation is asked, not one it answers"
            )));
        };
        let k = neighbour_count(count, self.space.guard())?;
        let query_row = self.space.row_of(query);

        let post_selection_filtered =
            args.get(HNSW_NEIGHBOUR).is_some() || args.get(HNSW_DISTANCE).is_some();
        let select_k = if post_selection_filtered {
            k
        } else {
            ceiling.map_or(k, |ceiling| k.min(usize::try_from(ceiling).unwrap_or(k)))
        };

        Ok(Box::new(HnswCursor {
            space: Arc::clone(&self.space),
            query_row,
            select_k,
            query_term: query.clone(),
            count_term: count.clone(),
            bound: args.flattened().map(<Option<&TermValue>>::cloned).collect(),
            ranked: None,
            at: 0,
            remaining: ceiling,
            unreported_work: 0,
        }))
    }
}

/// Read `k` off the invocation's neighbour-count argument.
fn neighbour_count(value: &TermValue, guard: KnnGuard) -> Result<usize, EvalError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    else {
        return Err(EvalError::function(format!(
            "the neighbour count at position {HNSW_COUNT} is {value:?}, which is not an \
             integer literal; there is no number of neighbours that names"
        )));
    };
    if datatype != XSD_INTEGER {
        return Err(EvalError::function(format!(
            "the neighbour count at position {HNSW_COUNT} is a literal of datatype \
             <{datatype}>; this relation emits an xsd:integer there"
        )));
    }
    let Ok(purrdf_xsd::XsdValue::Integer { value: count, .. }) =
        purrdf_xsd::parse(lexical_form, purrdf_xsd::XsdDatatype::Integer)
    else {
        return Err(EvalError::function(format!(
            "the neighbour count at position {HNSW_COUNT} has lexical form \
             {lexical_form:?}, which is not in the lexical space of xsd:integer"
        )));
    };
    if count < 0 {
        return Err(EvalError::function(format!(
            "the neighbour count at position {HNSW_COUNT} is {count}; a search cannot return \
             a negative number of neighbours"
        )));
    }
    let bound = i128::from(guard.max_neighbours());
    if count > bound {
        return Err(EvalError::function(format!(
            "the invocation asks for {count} neighbour(s), and the configured guard admits \
             at most {bound}; returning the {bound} nearest instead would be a short answer \
             reported as a complete one, so the request is refused rather than clamped"
        )));
    }
    usize::try_from(count).map_err(|_| {
        EvalError::function(format!(
            "the neighbour count {count} does not fit this platform's index range"
        ))
    })
}

/// The cursor [`HnswRelation::open`] returns: the ranked neighbours, filtered on every
/// bound position, cut at the engine's licence, and reporting the search's work.
#[derive(Debug)]
struct HnswCursor {
    /// The space being searched.
    space: Arc<HnswSpace>,
    /// The seed's row, or `None` when the space does not hold the seed term.
    query_row: Option<usize>,
    /// How many neighbours the ranking retains.
    select_k: usize,
    /// The seed term, echoed verbatim into position 1 of every row.
    query_term: TermValue,
    /// The neighbour count, echoed verbatim into position 2 of every row.
    count_term: TermValue,
    /// The invocation's bound values by flattened position (`None` = free).
    bound: Vec<Option<TermValue>>,
    /// The ranked neighbours, once the search has run.
    ranked: Option<Vec<Ranked>>,
    /// How far into `ranked` this cursor has read.
    at: usize,
    /// The rows this invocation may still emit under the engine's licence.
    remaining: Option<u64>,
    /// Candidates examined and not yet reported to the governor.
    unreported_work: u64,
}

impl HnswCursor {
    /// Run the search if it has not run yet, recording the candidates it examined.
    ///
    /// Laziness is what makes "no rows wanted" and "no work done" the same statement: the
    /// engine checks its ceiling before the first pull, so a call with an exhausted ceiling
    /// never reaches this function and is charged nothing.
    fn ensure_ranked(&mut self) -> Result<(), EvalError> {
        if self.ranked.is_some() {
            return Ok(());
        }
        let (ranked, work) = match self.query_row {
            Some(row) => self
                .space
                .index
                .search_rows_work(row, self.select_k)
                .map_err(|e| EvalError::data(format!("the HNSW search failed: {e}")))?,
            None => (Vec::new(), 0),
        };
        self.unreported_work = self.unreported_work.saturating_add(work);
        self.ranked = Some(ranked);
        Ok(())
    }

    /// The full row for one ranked neighbour.
    fn build(&self, scored: Ranked) -> Result<PfRow, EvalError> {
        let neighbour = self.space.term(scored.row).ok_or_else(|| {
            EvalError::data(format!(
                "the ranking named row {}, which the space does not hold",
                scored.row
            ))
        })?;
        Ok(vec![
            neighbour.clone(),
            self.query_term.clone(),
            self.count_term.clone(),
            TermValue::typed_literal(
                purrdf_xsd::numeric::canonical_double(scored.distance),
                XSD_DOUBLE,
            ),
        ])
    }
}

impl PfCursor for HnswCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.remaining == Some(0) {
            return Ok(None);
        }
        self.ensure_ranked()?;
        let ranked = self.ranked.as_ref().expect("ensure_ranked populated it");
        while let Some(&scored) = ranked.get(self.at) {
            self.at += 1;
            let row = self.build(scored)?;
            let agrees = self
                .bound
                .iter()
                .zip(row.iter())
                .all(|(want, have)| want.as_ref().is_none_or(|want| want == have));
            if agrees {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row));
            }
        }
        Ok(None)
    }

    /// One unit per **candidate examined** — one distance computation against one row of
    /// the graph, taken from the index's search memo.
    fn take_work(&mut self) -> u64 {
        core::mem::take(&mut self.unreported_work)
    }
}

/// Register `space` as a relation under the caller's predicate `iri`.
///
/// PurRDF supplies no IRI: the caller's predicate is what a query text names the provider
/// by, which is the second of the three approximation channels the contract requires.
pub fn register_hnsw_relation(
    registry: &mut PropertyFunctionRegistry,
    iri: impl Into<String>,
    space: Arc<HnswSpace>,
) {
    registry.register(iri, Arc::new(HnswRelation::new(space)));
}

#[cfg(test)]
mod tests {
    use purrdf_core::DistanceMetric;

    use super::*;
    use crate::{Params, VectorMatrix, level::splitmix64};

    fn matrix(rows: usize, dims: usize) -> VectorMatrix {
        let mut state = 0x5151_5151_5151_5151_u64;
        let mut data = Vec::with_capacity(rows * dims);
        for _ in 0..rows * dims {
            state = splitmix64(state);
            let unit = (state >> 11) as f64 / (1_u64 << 53) as f64;
            let value = unit.mul_add(2.0, -1.0);
            data.push(if value == 0.0 { 0.25 } else { value });
        }
        VectorMatrix::new(rows, dims, data).expect("valid fixture")
    }

    fn terms(rows: usize) -> Vec<TermValue> {
        (0..rows)
            .map(|row| TermValue::iri(format!("https://example.org/doc/{row}")))
            .collect()
    }

    fn space(rows: usize) -> Arc<HnswSpace> {
        let index = HnswIndex::build(
            matrix(rows, 4),
            &DistanceMetric::SquaredEuclidean,
            Params::new(4, 8, 16, 8).expect("valid"),
        )
        .expect("builds");
        let guard = KnnGuard::new(rows as u64, rows as u64).expect("valid");
        Arc::new(HnswSpace::from_index(index, terms(rows), guard).expect("space"))
    }

    fn invoke(relation: &HnswRelation, seed: usize, k: &str) -> Box<dyn PfCursor> {
        let seed = TermValue::iri(format!("https://example.org/doc/{seed}"));
        let count = TermValue::typed_literal(k, XSD_INTEGER);
        let subject = [None];
        let object = [Some(&seed), Some(&count), None];
        let args = PfArgs::new(&subject, &object);
        relation.open(&args, None).expect("opens")
    }

    #[test]
    fn a_zero_count_is_an_empty_answer_with_zero_work() {
        let space = space(16);
        let mut cursor = invoke(&space.relation(), 0, "0");
        assert!(cursor.next().expect("no error").is_none());
        assert_eq!(cursor.take_work(), 0);
    }

    #[test]
    fn a_ceiling_that_prevents_the_first_pull_charges_no_work() {
        let space = space(64);
        let relation = space.relation();
        let seed = TermValue::iri("https://example.org/doc/0");
        let count = TermValue::typed_literal("5", XSD_INTEGER);
        let subject = [None];
        let object = [Some(&seed), Some(&count), None];
        let args = PfArgs::new(&subject, &object);
        let mut cursor = relation.open(&args, Some(0)).expect("opens");
        assert!(cursor.next().expect("no error").is_none());
        assert_eq!(
            cursor.take_work(),
            0,
            "a ceiling that prevents the first pull must not charge for a search that never \
             ran"
        );
    }

    #[test]
    fn a_search_charges_its_distance_evaluations_exactly_once() {
        let space = space(64);
        let mut cursor = invoke(&space.relation(), 1, "3");
        let mut rows = 0;
        while cursor.next().expect("no error").is_some() {
            rows += 1;
        }
        assert!(rows <= 3);
        let work = cursor.take_work();
        assert!(work > 0, "a search over 64 rows examined some of them");
        assert!(work <= 64);
        assert_eq!(
            cursor.take_work(),
            0,
            "work resets when taken and is not re-reported"
        );
    }

    #[test]
    fn the_relation_is_registered_under_the_callers_iri() {
        let iri = "https://example.org/hnsw/nearest";
        let mut registry = PropertyFunctionRegistry::new();
        register_hnsw_relation(&mut registry, iri, space(8));
        assert!(registry.resolve(iri).is_some());
        assert!(registry.resolve("https://example.org/hnsw/other").is_none());
    }

    #[test]
    fn a_negative_count_and_a_free_seed_are_refused() {
        let space = space(8);
        let relation = space.relation();
        let seed = TermValue::iri("https://example.org/doc/0");
        let count = TermValue::typed_literal("-1", XSD_INTEGER);
        let subject = [None];
        let object = [Some(&seed), Some(&count), None];
        let args = PfArgs::new(&subject, &object);
        assert!(relation.open(&args, None).is_err());

        let free = [None];
        let bound_count = [Some(&count)];
        let args = PfArgs::new(&free, &bound_count);
        assert!(relation.open(&args, None).is_err());
    }
}

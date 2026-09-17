// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **Nearest-neighbour search over a PURREMB embedding space**, reachable from SPARQL
//! through the [property-function seam](crate::property_fn).
//!
//! `purrdf_core`'s PURREMB layer already carries RDF-1.2-addressable vectors, a declared
//! [`purrdf_core::DistanceMetric`], and tamper-evident guards binding a third-party index
//! payload to the exact matrix it was built over. What it does not carry is a way to *ask
//! a question*: it stores vectors and refuses to rank them, because ranking is a query
//! operation and PURREMB is an artifact format. This module is the query operation.
//!
//! ```text
//! PREFIX knn: <https://example.org/space/abstracts>
//!
//! SELECT ?neighbour ?distance WHERE {
//!   ?neighbour knn: ( <https://example.org/doc/7> 5 ?distance )
//! }
//! ```
//!
//! # PurRDF mints no IRI for this
//!
//! The predicate above is the **caller's**, and it is how a query names a space: a host
//! builds one [`EmbeddingSpace`] per `(artifact, target set, vector space)` triple it
//! wants queryable and registers an [`EmbeddingKnnRelation`] over it under whatever IRI
//! its own vocabulary uses. There is no default IRI, no namespace this crate reserves,
//! and no fallback space — a registry with nothing in it resolves nothing, which is a
//! host misconfiguration the evaluator reports rather than an empty search result.
//!
//! Registering one relation per space, rather than one relation taking a space IRI as an
//! argument, is what lets [`PropertyFunction::rows_per_invocation`] be *measured* from
//! the space it will actually search instead of maximized over every space a host might
//! ever register. The planner reads that number to admit the call against a cell ceiling,
//! so a bound inflated by an unrelated space is a query refused for no reason.
//!
//! # The call shape
//!
//! Four flattened positions — one on the subject side, three on the object side:
//!
//! | position | name | role |
//! |---|---|---|
//! | 0 | `?neighbour` | **out**: the RDF term whose vector was retrieved |
//! | 1 | `?query` | **in**: the term whose vector seeds the search |
//! | 2 | `k` | **in**: how many neighbours to retrieve |
//! | 3 | `?distance` | **out**: the distance, as `xsd:double` |
//!
//! The one declared mode is therefore `fbbf`: positions 1 and 2 are inputs this relation
//! cannot enumerate. By the seam's subsumption rule that admits every invocation binding
//! at least those two, so `?neighbour` and `?distance` may each be bound or free and the
//! engine's equality filter (which this relation's cursor mirrors) resolves them.
//!
//! # Ordering, and the exactness of it
//!
//! Rows are emitted in **rank order, nearest first**, under the metric the space's family
//! contract declares — the ordering PURREMB v1 defines, with `smaller ranks first` for
//! all three built-ins. The search is **exact**: every candidate row is scored, and the
//! `k` returned are the true `k` nearest, not an approximation. It is not an approximate
//! search that happens to be exact on small inputs; there is no candidate pruning
//! anywhere, and the guard below is an admission bound on work rather than a licence to
//! return an approximate answer.
//!
//! That matters for two claims elsewhere. It is why the ceiling the engine offers is
//! sound here (emission order *is* rank order, so the first `n` rows are the `n` nearest
//! for every `n`), and it is why "results ordered correctly" is a property this module
//! can be tested for rather than a property of a tuning parameter.
//!
//! # The guard, and what it does and does not bound
//!
//! PURREMB v1 stores derived-index payloads but does not interpret them: an
//! `IndexGuardView` binds an opaque third-party ANN payload to the exact
//! `(source, family, space, matrix, projection, prefix)` tuple it was built over, and
//! declares its own approximation contract — it is not an ANN algorithm PurRDF can run.
//! So this surface does not pretend to run one. Instead:
//!
//! * [`KnnGuard`] is **caller-supplied configuration with no default**: the largest space
//!   a host will let one invocation scan, and the largest `k` it will let one invocation
//!   request. Both are refusals, both are stated in the error when they fire, and both
//!   are what make [`PropertyFunction::rows_per_invocation`] an honest, tight bound
//!   rather than `u64::MAX`.
//! * Every derived-index guard in the artifact that names *this* target set and vector
//!   space is checked against the matrix actually being scanned, so a stale or
//!   substituted guard is a construction-time failure rather than silent agreement.
//! * The evaluation cost the governor charges is proportional to the candidates the
//!   search actually examines — the guard-bounded work — through
//!   [`PfCursor::take_work`]. Before that channel existed, a scan of a million vectors
//!   returning five rows was priced at six units of fuel.
//!
//! # Determinism
//!
//! Every distance is computed by a [`Kernel`]: binary64, one correctly-rounded IEEE
//! operation at a time, in a pinned accumulation order, with no fused multiply-add and no
//! transcendental. Ranking breaks equal distances by ascending row number, and a
//! PURREMB target set numbers its rows by sorted `TargetId` — a digest of canonical
//! content — so the tie-break is a function of the data rather than of the order anything
//! was built in. Two independently produced artifacts over the same targets rank
//! identically, on every target this workspace builds for.

mod metric;

use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{
    DistanceMetric, EmbeddingView, TargetId, TargetSetId, TermValue, VectorDtype, VectorSpaceId,
    verify_embedding,
};

use crate::error::EvalError;
use crate::property_fn::{PfArgs, PfArity, PfCursor, PfRow, PropertyFunction};
use crate::user_fn::Volatility;

pub use metric::{Kernel, Ranked, best, norm};

/// The `?neighbour` position: the retrieved term.
const KNN_NEIGHBOUR: usize = 0;
/// The `?query` position: the term whose vector seeds the search. Always an input.
const KNN_QUERY: usize = 1;
/// The `k` position: how many neighbours to retrieve. Always an input.
const KNN_COUNT: usize = 2;
/// The `?distance` position: the retrieved term's distance from the query.
const KNN_DISTANCE: usize = 3;
/// The one access pattern [`EmbeddingKnnRelation`] declares.
const KNN_MODE: &str = "fbbf";

/// `xsd:double`, the datatype every emitted distance carries.
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

// ---------------------------------------------------------------------------
// The guard
// ---------------------------------------------------------------------------

/// The **work bounds** a host puts on one nearest-neighbour invocation.
///
/// Caller-supplied, with no default and no `Default` impl, because both numbers are
/// statements about what a host is willing to spend and neither has a value this crate
/// could invent on its behalf. They exist for two independent reasons:
///
/// * `max_candidates` bounds the **search**: a space with more rows than this is refused
///   at construction, not truncated at query time. Truncating would be the worse failure
///   by far — a top-`k` computed over an arbitrary prefix of the space is a wrong answer
///   that looks exactly like a right one.
/// * `max_neighbours` bounds the **request**: an invocation asking for more than this is
///   refused, naming both numbers. This is what lets
///   [`PropertyFunction::rows_per_invocation`] declare a real bound. `k` is a per-call
///   argument the declaration cannot see, so without a configured ceiling on it the only
///   honest declaration would be the whole space — and the planner would refuse the call
///   against any modest cell ceiling on the strength of a number no invocation would ever
///   reach.
///
/// Both are hard refusals rather than clamps, and both are exercised in both directions:
/// a value *at* the bound is admitted, a value one past it is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnnGuard {
    max_candidates: u64,
    max_neighbours: u64,
}

impl KnnGuard {
    /// A guard admitting a space of at most `max_candidates` rows and an invocation
    /// requesting at most `max_neighbours` neighbours.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] if either bound is zero. A zero `max_candidates` admits no
    /// space at all and a zero `max_neighbours` admits no request, so either one
    /// configures a relation that can never answer anything — which is a mistake worth
    /// reporting where it is made rather than a valid, permanently-empty surface.
    pub fn new(max_candidates: u64, max_neighbours: u64) -> Result<Self, EvalError> {
        if max_candidates == 0 || max_neighbours == 0 {
            return Err(EvalError::config(format!(
                "a kNN guard admitting at most {max_candidates} candidate(s) and at most \
                 {max_neighbours} neighbour(s) can never answer any invocation; both bounds \
                 must be positive"
            )));
        }
        Ok(Self {
            max_candidates,
            max_neighbours,
        })
    }

    /// The largest space this guard admits, in rows.
    #[must_use]
    pub const fn max_candidates(self) -> u64 {
        self.max_candidates
    }

    /// The largest `k` this guard admits in one invocation.
    #[must_use]
    pub const fn max_neighbours(self) -> u64 {
        self.max_neighbours
    }
}

// ---------------------------------------------------------------------------
// The space
// ---------------------------------------------------------------------------

/// One queryable PURREMB vector space: its vectors, the RDF terms they belong to, the
/// metric they are compared under, and the guard bounding a search over them.
///
/// # Why the vectors are copied out of the artifact
///
/// An [`EmbeddingView`] borrows the caller's bytes, and a [`PropertyFunction`] is stored
/// behind an `Arc<dyn …>` with no lifetime to lend it — so a relation cannot hold a view.
/// Re-opening the artifact per invocation would mean re-running structural validation on
/// every driving row, which is a per-row cost for an answer that cannot change.
///
/// Materializing once has a second benefit that matters more than the first. PURREMB
/// exposes each row two ways: a true zero-copy `&[f32]` when the buffer happens to be
/// aligned and fully verified, and a portable decoder otherwise — and *which one a caller
/// gets depends on the alignment of a heap allocation*. Reading every row once, through
/// the portable path, means there is exactly one arithmetic path in this crate and
/// therefore nothing for the two to disagree about. Values are widened to `f64`, which is
/// exact for an `f32` and identity for an `f64`.
///
/// # What construction proves
///
/// Everything that could otherwise fail mid-query, so that an invocation's only remaining
/// failure modes are about the invocation:
///
/// * the artifact verifies in full (`verify_embedding`), not merely structurally;
/// * the named target set and vector space exist, and an effective matrix joins them;
/// * the declared metric is one this engine can evaluate;
/// * every row of the target set has exactly one caller-supplied term, and no term is
///   claimed by two rows;
/// * the space fits the guard's candidate bound;
/// * under a norm-dividing metric, no stored vector has a zero norm;
/// * every derived-index guard naming this space binds the matrix actually being scanned.
#[derive(Debug, Clone)]
pub struct EmbeddingSpace {
    /// The kernel the family contract declared.
    kernel: Kernel,
    /// The declared metric, retained verbatim for [`Self::metric`].
    metric: DistanceMetric,
    /// The effective (post-prefix, post-postprocessing) dimension of every row.
    dimension: usize,
    /// Every row's components, row-major: row `r` occupies `r * dimension ..`.
    vectors: Vec<f64>,
    /// Every row's L2 norm, in row order. Empty unless [`Kernel::needs_norms`].
    norms: Vec<f64>,
    /// Row `r`'s RDF term.
    terms: Vec<TermValue>,
    /// Row numbers ordered by their term, for the term-to-row lookup a query seed needs.
    /// A sorted vector rather than a hash map because [`TermValue`] is `Ord` and not
    /// `Hash`, and because a binary search over canonical term order is deterministic
    /// without a hasher having to be chosen.
    rows_by_term: Vec<usize>,
    /// The bounds one invocation is held to.
    guard: KnnGuard,
}

impl EmbeddingSpace {
    /// Build a queryable space from `artifact`'s `(target_set, vector_space)` matrix.
    ///
    /// `bindings` names the RDF term each of the space's targets stands for. PURREMB
    /// deliberately allows an artifact to disclose a target by digest alone, so the
    /// term↔target correspondence is host knowledge and is supplied rather than decoded:
    /// a space whose rows could not be named would emit neighbours nobody can join back
    /// to a graph.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] for a caller mistake — a space or target set the artifact
    /// does not hold, a binding for a target outside the set, a duplicate target or term,
    /// a row no binding covers, a space larger than the guard admits, or an extension
    /// metric this engine cannot evaluate.
    ///
    /// [`EvalError::Data`] for a defect in the artifact itself — verification failure, an
    /// unreadable row, a zero-norm vector under a norm-dividing metric, or a derived-index
    /// guard naming this space but binding a different matrix.
    ///
    /// # A row without a term is a refusal, not a skipped row
    ///
    /// Dropping an uncovered row would make the search quietly range over fewer
    /// candidates than the space holds, so a top-`k` would be the top `k` of a subset and
    /// nothing would say so. The coverage check runs here, once, where a host can fix it —
    /// which also means no invocation can fail this way.
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
        let family = view.family(space.family_id()).ok_or_else(|| {
            EvalError::data(format!(
                "vector space {vector_space} names family {} , which the artifact does not hold",
                space.family_id()
            ))
        })?;
        let metric = family.metric().map_err(|e| {
            EvalError::data(format!("the family's declared metric is unusable: {e}"))
        })?;
        let kernel = Kernel::of(&metric).ok_or_else(|| {
            EvalError::config(format!(
                "vector space {vector_space} declares the caller-defined distance metric \
                 {metric:?}, whose parameters are opaque bytes this engine cannot evaluate; \
                 only the three built-in PURREMB metrics can be ranked here"
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

        let row_count = set.row_count();
        if row_count as u64 > guard.max_candidates() {
            return Err(EvalError::config(format!(
                "this space holds {row_count} row(s), which is more than the {} candidate(s) \
                 the configured guard admits; a search here would either exceed the work the \
                 host licensed or rank a prefix of the space and report it as the whole",
                guard.max_candidates()
            )));
        }

        let dimension = space.dimension() as usize;
        let terms = bind_terms(&set, bindings)?;
        let vectors = read_vectors(&effective, row_count, dimension)?;
        let norms = read_norms(kernel, &vectors, dimension, row_count)?;
        check_index_guards(&view, target_set, vector_space, &effective)?;

        let mut rows_by_term: Vec<usize> = (0..row_count).collect();
        rows_by_term.sort_unstable_by(|&left, &right| terms[left].cmp(&terms[right]));

        Ok(Self {
            kernel,
            metric,
            dimension,
            vectors,
            norms,
            terms,
            rows_by_term,
            guard,
        })
    }

    /// How many candidate rows this space holds.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.terms.len()
    }

    /// The effective dimension of every vector in this space.
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// The distance metric the space's family contract declares.
    #[must_use]
    pub const fn metric(&self) -> &DistanceMetric {
        &self.metric
    }

    /// The work bounds one invocation over this space is held to.
    #[must_use]
    pub const fn guard(&self) -> KnnGuard {
        self.guard
    }

    /// The RDF term row `row` stands for.
    #[must_use]
    pub fn term(&self, row: usize) -> Option<&TermValue> {
        self.terms.get(row)
    }

    /// The row `term` occupies, if this space holds it.
    ///
    /// Terms are distinct by construction, so this is single-valued.
    #[must_use]
    pub fn row_of(&self, term: &TermValue) -> Option<usize> {
        self.rows_by_term
            .binary_search_by(|&row| self.terms[row].cmp(term))
            .ok()
            .map(|at| self.rows_by_term[at])
    }

    /// Row `row`'s components.
    fn vector(&self, row: usize) -> &[f64] {
        let start = row * self.dimension;
        &self.vectors[start..start + self.dimension]
    }

    /// Row `row`'s L2 norm, or `0.0` for a kernel that does not divide by one.
    fn norm_of(&self, row: usize) -> f64 {
        self.norms.get(row).copied().unwrap_or(0.0)
    }

    /// The `k` rows nearest `query_row`, in rank order.
    ///
    /// Returns the ranked rows together with **the number of candidates examined**, which
    /// is the work unit this surface reports to the governor: one distance computation per
    /// candidate, every candidate scored exactly once.
    ///
    /// # Errors
    ///
    /// [`EvalError::Data`] if a distance leaves the finite binary64 range. Ranking by an
    /// infinity would sort — last, confidently — from a number that overflowed.
    fn search(&self, query_row: usize, k: usize) -> Result<(Vec<Ranked>, u64), EvalError> {
        if k == 0 {
            // No neighbours were asked for, so no candidate is examined and no work is
            // reported. A zero request is a well-formed question with an empty answer.
            return Ok((Vec::new(), 0));
        }
        let query = self.vector(query_row);
        let query_norm = self.norm_of(query_row);
        let mut scored: Vec<Ranked> = Vec::with_capacity(self.row_count());
        for row in 0..self.row_count() {
            let distance = self
                .kernel
                .distance(query, query_norm, self.vector(row), self.norm_of(row))
                .ok_or_else(|| {
                    EvalError::data(format!(
                        "the distance from row {query_row} to row {row} left the finite range \
                         under {:?}; the artifact's magnitudes cannot be ranked under this \
                         metric",
                        self.metric
                    ))
                })?;
            scored.push(Ranked { distance, row });
        }
        let examined = scored.len() as u64;
        Ok((best(k, scored), examined))
    }
}

/// Place each binding at its target's row, proving the cover is exact.
fn bind_terms(
    set: &purrdf_core::TargetSetView<'_>,
    bindings: Vec<(TargetId, TermValue)>,
) -> Result<Vec<TermValue>, EvalError> {
    let row_count = set.row_count();
    let mut terms: Vec<Option<TermValue>> = vec![None; row_count];
    for (target, term) in bindings {
        let row = set.row_for_target(target).ok_or_else(|| {
            EvalError::config(format!(
                "the binding for target {target} names a target this space's target set does \
                 not hold"
            ))
        })?;
        if terms[row].is_some() {
            return Err(EvalError::config(format!(
                "target {target} (row {row}) is bound twice; a row stands for exactly one RDF \
                 term"
            )));
        }
        terms[row] = Some(term);
    }
    let mut bound: Vec<TermValue> = Vec::with_capacity(row_count);
    for (row, term) in terms.into_iter().enumerate() {
        let term = term.ok_or_else(|| {
            EvalError::config(format!(
                "row {row} (target {}) has no bound RDF term; a space with an unnamed row \
                 would search {row_count} candidates and be able to report only some of them, \
                 so the top-k it returned would silently be the top-k of a subset",
                set.target(row)
                    .map_or_else(|| "<unknown>".to_owned(), TargetId::to_hex)
            ))
        })?;
        bound.push(term);
    }

    // Distinct terms, checked over the canonical order rather than by hashing, so the
    // duplicate reported is the same one on every run.
    let mut ordered: Vec<&TermValue> = bound.iter().collect();
    ordered.sort_unstable();
    if let Some([duplicate, _]) = ordered.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(EvalError::config(format!(
            "the term {duplicate:?} is bound to two different rows; a query seed names a term, \
             so a term claimed by two rows makes the seed ambiguous"
        )));
    }
    Ok(bound)
}

/// Read every row of `effective` into one row-major `f64` buffer.
fn read_vectors(
    effective: &purrdf_core::EffectiveMatrixView<'_>,
    row_count: usize,
    dimension: usize,
) -> Result<Vec<f64>, EvalError> {
    let dtype = effective
        .matrix()
        .dtype()
        .map_err(|e| EvalError::data(format!("the matrix's scalar type is unreadable: {e}")))?;
    let mut out: Vec<f64> = Vec::with_capacity(row_count.saturating_mul(dimension));
    for row in 0..row_count {
        let index = row as u64;
        let before = out.len();
        match dtype {
            VectorDtype::F32 => {
                let values = effective.f32_row(index).map_err(|e| {
                    EvalError::data(format!("row {row} of the matrix is unreadable: {e}"))
                })?;
                for value in values {
                    let value = value.map_err(|e| {
                        EvalError::data(format!("row {row} of the matrix is unreadable: {e}"))
                    })?;
                    // Exact: every binary32 is a binary64.
                    out.push(f64::from(value));
                }
            }
            VectorDtype::F64 => {
                let values = effective.f64_row(index).map_err(|e| {
                    EvalError::data(format!("row {row} of the matrix is unreadable: {e}"))
                })?;
                for value in values {
                    out.push(value.map_err(|e| {
                        EvalError::data(format!("row {row} of the matrix is unreadable: {e}"))
                    })?);
                }
            }
        }
        if out.len() - before != dimension {
            return Err(EvalError::data(format!(
                "row {row} decoded {} component(s); the vector space declares {dimension}",
                out.len() - before
            )));
        }
    }
    Ok(out)
}

/// Every row's L2 norm, for a kernel that divides by one; empty otherwise.
fn read_norms(
    kernel: Kernel,
    vectors: &[f64],
    dimension: usize,
    row_count: usize,
) -> Result<Vec<f64>, EvalError> {
    if !kernel.needs_norms() {
        return Ok(Vec::new());
    }
    let mut norms = Vec::with_capacity(row_count);
    for row in 0..row_count {
        let start = row * dimension;
        let value = norm(&vectors[start..start + dimension]);
        // PURREMB v1: "Cosine distance is undefined for a zero-norm operand and hard-fails
        // rather than inventing a score." Caught here rather than per invocation, so a
        // space that verifies can never fail a query this way.
        if value <= 0.0 {
            return Err(EvalError::data(format!(
                "row {row} has a zero L2 norm, and the declared metric divides by it; a \
                 zero-norm vector has no direction, so its cosine distance is undefined \
                 rather than large"
            )));
        }
        norms.push(value);
    }
    Ok(norms)
}

/// Check every derived-index guard that names this space against the matrix being scanned.
///
/// A guard is PURREMB's tamper-evident binding from an opaque third-party ANN payload to
/// the exact coordinates it was built over. PurRDF cannot run the payload, but it can
/// check the binding — and a guard that names this target set and vector space while
/// pointing at a *different* matrix or projection is either stale or substituted. Either
/// way it is a statement about this space that is no longer true, and admitting it would
/// let a host believe an index covers a matrix it does not.
///
/// Guards naming other spaces in the same artifact are none of this space's business and
/// are passed over.
///
/// # What this adds, stated honestly
///
/// `purrdf_core`'s own writer cannot *produce* a mismatch: it refuses at build time a
/// derived index whose coordinates do not describe a matrix the artifact holds, and the
/// guard digest covers those coordinates, so a tampered one fails `verify_embedding`
/// before this runs. Through this workspace's encoder the refusal below is therefore
/// unreachable, and the tested behaviour is the *other* half — that a guard which agrees
/// is admitted and changes no answer.
///
/// It is kept anyway, and the reason is the format rather than the encoder. PURREMB is a
/// wire format with third-party producers; a reader that assumed its own writer made the
/// bytes would be assuming exactly what a fail-closed borrowed-view design exists to stop
/// assuming. The check is one integer comparison per guard, at construction, once.
fn check_index_guards(
    view: &EmbeddingView<'_>,
    target_set: TargetSetId,
    vector_space: VectorSpaceId,
    effective: &purrdf_core::EffectiveMatrixView<'_>,
) -> Result<(), EvalError> {
    let matrix = effective.matrix().id();
    let projection = effective.projection().id();
    for guard in view.index_guards() {
        if guard.target_set_id() != target_set || guard.vector_space_id() != vector_space {
            continue;
        }
        if guard.matrix_id() != matrix || guard.projection_id() != projection {
            return Err(EvalError::data(format!(
                "derived index {} names target set {target_set} and vector space \
                 {vector_space} but binds matrix {} / projection {}, where this space scans \
                 matrix {matrix} / projection {projection}; the guard is stale or substituted",
                guard.id(),
                guard.matrix_id(),
                guard.projection_id()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The relation
// ---------------------------------------------------------------------------

/// The [`PropertyFunction`] a host registers to make [`EmbeddingSpace`] queryable.
///
/// One relation per space; see the [module docs](self) for the call shape, and why the
/// space is the registration rather than an argument.
///
/// [`Volatility::Stable`]: the space is frozen and every distance is a pure function of
/// two of its rows computed by a [`Kernel`]'s pinned-order binary64 arithmetic, so an
/// invocation's rows are the same on the main thread, on a fork-join worker, and on
/// `wasm32-unknown-unknown`.
#[derive(Debug, Clone)]
pub struct EmbeddingKnnRelation {
    /// The space every invocation searches.
    space: Arc<EmbeddingSpace>,
    /// The single declared mode, materialized once so [`PropertyFunction::modes`] can
    /// hand out a slice.
    modes: [BindingPattern; 1],
}

impl EmbeddingKnnRelation {
    /// A nearest-neighbour relation over `space`.
    #[must_use]
    pub fn new(space: Arc<EmbeddingSpace>) -> Self {
        Self {
            space,
            modes: [BindingPattern::from_code(KNN_MODE)],
        }
    }

    /// The space this relation searches.
    #[must_use]
    pub fn space(&self) -> &EmbeddingSpace {
        &self.space
    }
}

impl PropertyFunction for EmbeddingKnnRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 3)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// The declared row bound, as a real function of the mode.
    ///
    /// Only one of the four positions restricts how many rows an invocation can produce.
    ///
    /// | bound positions | declared bound | why |
    /// |---|---|---|
    /// | `?neighbour` (0) | `min(1, rows)` | terms are distinct within a space, so at most one row carries the bound term — and it is emitted only if it is among the `k` nearest |
    /// | otherwise | `min(max_neighbours, rows)` | an invocation emits at most `k` rows, `k` is capped by the guard, and a space cannot yield more rows than it holds |
    ///
    /// `?query` (1) and `k` (2) are bound in every admitted invocation, so neither
    /// distinguishes a mode. `?distance` (3) restricts nothing: arbitrarily many rows can
    /// sit at the same distance from a query, so binding it cannot bound the count.
    ///
    /// Both bounds are measured from the frozen space rather than assumed — an empty space
    /// declares `0`, not `1` — and both are attained, which is the property an `<=`
    /// assertion cannot check and `u64::MAX` would trivially satisfy.
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        let rows = self.space.row_count() as u64;
        if mode.is_bound(KNN_NEIGHBOUR) {
            rows.min(1)
        } else {
            rows.min(self.space.guard().max_neighbours())
        }
    }

    /// Begin one nearest-neighbour invocation.
    ///
    /// # Refusals
    ///
    /// Each aborts the query rather than contributing zero rows, which would be
    /// indistinguishable from an honest empty answer:
    ///
    /// * `?query` (1) or `k` (2) is free — this relation retrieves neighbours *for* a seed
    ///   and cannot enumerate seeds, nor invent how many to return;
    /// * `k` is not an integer literal, or is negative — there is no such request;
    /// * `k` exceeds the guard's `max_neighbours`, which is named in the error.
    ///
    /// # What is an empty answer rather than a refusal
    ///
    /// A `?query` term the space does not hold is a **well-formed question the data does
    /// not answer**, exactly as an unmatched triple pattern is, and it yields no rows.
    /// Refusing it would abort any query that ranged a seed over terms only some of which
    /// are embedded — which is the ordinary way this relation is used.
    ///
    /// `k = 0` is likewise a request for zero neighbours, honoured with zero rows and zero
    /// work. It is a boundary a clamp-or-refuse rule gets wrong in both directions.
    ///
    /// # The ceiling
    ///
    /// The engine offers a row ceiling whenever the call is admission-transparent, which
    /// includes calls that bind `?neighbour` or `?distance` to a constant. Those two
    /// positions are filtered *after* the ranking, so shrinking the ranking to the ceiling
    /// would let the cursor filter a prefix and then report exhaustion with fewer rows
    /// than the engine asked for — a short bag read as a complete one. So the ceiling is
    /// pushed into the selection only when neither is bound; otherwise the full `k` are
    /// ranked and the cursor does the cutting, spending the licence only on rows it emits.
    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let declared = self.arity();
        let supplied = args.arity();
        if supplied != declared {
            return Err(EvalError::function(format!(
                "the embedding kNN relation expects {declared} argument(s), got {supplied}"
            )));
        }

        let Some(query) = args.get(KNN_QUERY) else {
            return Err(EvalError::function(format!(
                "the query term at position {KNN_QUERY} is free; this relation retrieves the \
                 neighbours of a seed and cannot enumerate seeds, which is why its only \
                 declared mode is `{KNN_MODE}`"
            )));
        };
        let Some(count) = args.get(KNN_COUNT) else {
            return Err(EvalError::function(format!(
                "the neighbour count at position {KNN_COUNT} is free; how many neighbours to \
                 retrieve is a question this relation is asked, not one it answers"
            )));
        };
        let k = neighbour_count(count, self.space.guard())?;

        // A seed the space does not hold: an honest empty result, not a refusal. See the
        // method docs.
        let query_row = self.space.row_of(query);

        let post_selection_filtered =
            args.get(KNN_NEIGHBOUR).is_some() || args.get(KNN_DISTANCE).is_some();
        let select_k = if post_selection_filtered {
            k
        } else {
            ceiling.map_or(k, |ceiling| k.min(usize::try_from(ceiling).unwrap_or(k)))
        };

        Ok(Box::new(KnnCursor {
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

/// The cursor [`EmbeddingKnnRelation::open`] returns: the ranked neighbours, filtered on
/// every bound position and cut at the engine's licence.
///
/// # Why the search is lazy
///
/// The ranking runs on the first [`PfCursor::next`], not in `open`. The engine checks its
/// own ceiling *before* pulling, so a call whose ceiling is already exhausted never pulls
/// at all — and a search performed in `open` would have been done, and charged, for an
/// answer nobody was going to read. Doing it here makes "no rows were wanted" and "no work
/// was done" the same statement.
///
/// # The two properties that make the licence sound
///
/// * It filters on **every** bound position, including `?neighbour` and `?distance`, which
///   the ranking cannot see. A relation may generate candidates and let the engine's own
///   filter cut them, but a relation that also *spent a ceiling* on them would hand back
///   fewer usable rows than the engine asked for.
/// * It decrements the licence only on rows it actually **emits**. A skipped row disagrees
///   with a bound position and the engine would have dropped it anyway.
#[derive(Debug)]
struct KnnCursor {
    /// The space being searched.
    space: Arc<EmbeddingSpace>,
    /// The seed's row, or `None` when the space does not hold the seed term.
    query_row: Option<usize>,
    /// How many neighbours the ranking retains — `k`, or the engine's ceiling when it was
    /// safe to push it down.
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

impl KnnCursor {
    /// Run the search if it has not run yet, recording the candidates it examined.
    fn ensure_ranked(&mut self) -> Result<(), EvalError> {
        if self.ranked.is_some() {
            return Ok(());
        }
        let (ranked, examined) = match self.query_row {
            Some(row) => self.space.search(row, self.select_k)?,
            None => (Vec::new(), 0),
        };
        self.unreported_work = self.unreported_work.saturating_add(examined);
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

impl PfCursor for KnnCursor {
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
    /// the space.
    ///
    /// This is the quantity a kNN search's cost is actually proportional to, and it is
    /// invisible from outside: the rows this relation returns are `k`, and `k` says
    /// nothing about the size of the space they were selected from.
    fn take_work(&mut self) -> u64 {
        core::mem::take(&mut self.unreported_work)
    }
}

#[cfg(test)]
mod tests;

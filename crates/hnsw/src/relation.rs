// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! The approximation contract declares four governed channels; the first is the guard's
//! [`purrdf_core::IndexLossContract`] and evidence string, checked by
//! [`crate::guard::load`]. The other three live here:
//!
//! * the relation is registered under the caller's predicate IRI, so the query text itself
//!   names the provider and no namespace is fabricated;
//! * a result is an **offer of candidates**, never a certification of absence. This
//!   relation's cursor can say "here are `k` candidates"; it can never say "no row is
//!   nearer". `crates/hnsw/tests/oracle_contract.rs` asserts that even when a query misses
//!   the exact oracle's nearest row, the empty and non-empty answers are ordinary cursor
//!   results, never a completeness claim;
//! * **the composed answer.** The three channels above all speak to a caller reading this
//!   relation directly, and none of them survives composition: fuse these rows with an
//!   exhaustive producer's and the result reports one terminal status per stratum, with
//!   nothing to say that this one offered candidates while the other returned everything
//!   it had. So the promise is declared where a consumer of composed rows reads it, through
//!   [`HnswRelation::ranked_declaration`], and carried verbatim into the fused answer.
//!   A relation registered *without* that declaration is not silently fused; it forms no
//!   stratum at all, and the request reports the term as unserved.
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
//! The general declared mode is `fbbf`: positions 1 and 2 are inputs this relation cannot
//! enumerate. Rows are emitted in rank order, nearest first, under the shared exact path's
//! [`Ranked`] order `(distance, row)`.
//!
//! A second mode, `bbff`, is declared beside it: the neighbour and the seed bound, the
//! count **free**. It is a genuinely new capability rather than a narrowing — `fbbf` binds
//! the count it leaves free, so `fbbf` does not subsume it — and it asks a different
//! question: *do you hold this term at all*, answered by [`HnswSpace::row_of`] without
//! entering the beam. A count-BOUND call keeps its `k` cut exactly as before: the two are
//! two points of the lattice, and the invocation's own pattern decides which was asked.
//! See [`PropertyFunction::open`].
//!
//! # Work accounting
//!
//! The cursor reports one unit per candidate distance actually evaluated, through
//! [`PfCursor::take_work`]. The search is lazy — it runs on the first pull, not in
//! [`PropertyFunction::open`] — so a call whose ceiling is already exhausted performs no
//! work and is charged none, and the count resets only when the engine takes it.

use purrdf_hash::Domain;
use purrdf_hash::frame::frame_be_labelled;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::distance::{Arithmetic, Exact};
use purrdf_core::{
    ContentDigest, EmbeddingView, IndexLossContract, Iri, TargetId, TargetSetId, TermValue,
    VectorSpaceId, verify_embedding,
};
use purrdf_sparql_eval::knn::{
    KNN_ARITY, KNN_COUNT, KNN_DISTANCE, KNN_MEMBERSHIP_MODE, KNN_MODE, KNN_NEIGHBOUR, KNN_QUERY,
    KnnInvocation, RankSource, TermRows,
};
use purrdf_sparql_eval::{
    AcceptedTerm, CandidateDomains, Completeness, DepthPlacement, DuplicatePolicy, EvalError,
    ExclusionBasis, Kernel, KnnGuard, OrderFidelity, PfArgs, PfArity, PfCursor, PropertyFunction,
    PropertyFunctionRegistry, RankArithmetic, RankFidelity, Ranked, RankedDeclaration,
    RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
};

use crate::error::HnswError;
use crate::{HnswIndex, IndexArithmetic, profile};

/// An HNSW failure as the evaluator's error, under `context`.
///
/// A float-environment refusal keeps its own variant rather than being flattened into
/// data prose: nothing is wrong with the artifact, the calling thread is, and a host has
/// to be able to tell the two apart without reading a message.
fn eval_error(context: &str, error: HnswError) -> EvalError {
    match error {
        HnswError::FloatEnvironment(refusal) => EvalError::FloatEnvironment(refusal),
        HnswError::Workspace(error) => error,
        other => EvalError::data(format!("{context}: {other}")),
    }
}

/// Native query failures preserve typed storage causes without formatting them.
fn eval_error_admitted(
    context: &str,
    error: HnswError,
    workspace: &purrdf_sparql_eval::WorkspaceCapability,
) -> EvalError {
    match error {
        HnswError::FloatEnvironment(refusal) => EvalError::FloatEnvironment(refusal),
        HnswError::Workspace(error) => error,
        other => purrdf_sparql_eval::NativeDiagnostic::error(
            purrdf_sparql_eval::NativeDiagnosticKind::Data,
            format_args!("{context}: {other}"),
            workspace,
        ),
    }
}

// ---------------------------------------------------------------------------
// The space
// ---------------------------------------------------------------------------

/// One queryable HNSW index: its graph, its row terms, and the work bounds a host sets.
///
/// Construction is where everything that could otherwise fail mid-query is proven: the
/// artifact verifies, the unique HNSW guard's profile and coordinates agree with the
/// searched matrix, the payload commitment holds, the payload decodes, and every row has a
/// distinct RDF term. An invocation therefore fails only on things about the invocation.
///
/// `A` is the index's [`Arithmetic`], [`Exact`] by default; a space over a
/// [`build::<Reassociated>`](crate::build) index is `HnswSpace<Reassociated>`, and its evidence
/// says so.
#[derive(Debug, Clone)]
pub struct HnswSpace<A: Arithmetic = Exact> {
    /// The decoded graph.
    index: Arc<HnswIndex<A>>,
    /// Row `r`'s RDF term, in the artifact's canonical (ascending `TargetId`) row order,
    /// and the term-to-row lookup a seed needs.
    rows: TermRows,
    /// The bounds one invocation is held to.
    guard: KnnGuard,
    /// The approximation evidence the profile publishes for this index's arithmetic and
    /// dispatch path.
    evidence: String,
    /// The content identity of everything that decides this space's answers.
    generation: Arc<str>,
}

impl<A: IndexArithmetic> HnswSpace<A> {
    /// Build a queryable space from a PURREMB artifact's `(target_set, vector_space)` HNSW
    /// index, decoded under `A` with [`crate::guard::load`].
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
    ///
    /// A guard naming the implementation of another arithmetic, or a payload recorded on a
    /// dispatch path this process cannot run, is [`EvalError::Data`].
    ///
    /// [`EvalError::FloatEnvironment`] when the calling thread's float environment is not
    /// the IEEE one the distance arithmetic defines.
    pub fn from_artifact(
        artifact: &[u8],
        target_set: TargetSetId,
        vector_space: VectorSpaceId,
        bindings: Vec<(TargetId, TermValue)>,
        guard: KnnGuard,
    ) -> Result<Self, EvalError> {
        // The float environment first: verifying a normalized projection and reading its
        // rows are exact arithmetic, and a refused environment must be reported as one,
        // not as an artifact that failed to verify.
        let exact = Exact::resolve().map_err(EvalError::FloatEnvironment)?;
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

        let matrix = crate::guard::read_effective_matrix(&effective, exact)
            .map_err(|e| EvalError::data(format!("the effective matrix is unreadable: {e}")))?;
        let index = crate::guard::load::<A>(&guard_view, matrix)
            .map_err(|e| eval_error("the HNSW payload is unusable", e))?;
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

        let rows = TermRows::bind(&set, bindings)?;
        Self::from_index(index, rows.terms().to_vec(), guard)
    }
}

impl<A: Arithmetic> HnswSpace<A> {
    /// Assemble a space directly from a decoded index and its row terms.
    ///
    /// # Errors
    ///
    /// [`EvalError::Config`] if the term count differs from the graph's row count, one
    /// term is claimed by two rows, or the graph holds more rows than the guard admits.
    ///
    /// The candidate bound is checked here and not only in [`Self::from_artifact`],
    /// which delegates to this function: it is a property of the space, and a space
    /// assembled straight from an index is still a space the guard promised to bound.
    pub fn from_index(
        index: HnswIndex<A>,
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
        if index.rows() as u64 > guard.max_candidates() {
            return Err(EvalError::config(format!(
                "this space holds {} row(s), which is more than the {} candidate(s) the \
                 configured guard admits",
                index.rows(),
                guard.max_candidates()
            )));
        }
        let rows = TermRows::new(terms)?;
        // Derived once, here, so BOTH constructors carry it: `from_artifact`
        // delegates to this function, and a field present on one construction
        // path and absent on the other is precisely the modal optionality this
        // workspace refuses.
        let generation = space_generation(&index, rows.terms());
        // The evidence the guard of this index publishes: the profile's statement, and for
        // an arithmetic whose bits depend on the path, that arithmetic's evidence along the
        // path the image records.
        let evidence = profile::loss_evidence_for::<A>(index.arithmetic().path());
        Ok(Self {
            index: Arc::new(index),
            rows,
            guard,
            evidence,
            generation,
        })
    }

    /// How many candidate rows this space holds.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
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

    /// The beam width one search may explore: `ef_search`, as the artifact
    /// declared it.
    ///
    /// A real ceiling on the rows an invocation can return, not a tuning hint —
    /// the beam is never widened to fit a request — so it is declared through
    /// [`HnswRelation::rows_per_invocation`] beside the guard's own bound.
    #[must_use]
    pub fn beam_width(&self) -> u64 {
        self.index.params().ef_search() as u64
    }

    /// The generation of this space: a content digest of everything that decides
    /// which rows a search can return and in what order.
    ///
    /// Declared through [`PfCursor::generation`] so a fused answer can say which
    /// version of this index answered it, and so two answers drawn from two
    /// different graphs are distinguishable even when every other identity
    /// matches.
    ///
    /// # What it folds, and the one thing that differs from the exact kNN space
    ///
    /// Three parts, each framed: the graph's canonical byte image, the guard's
    /// parameter block, and the bound terms in canonical row order. The first
    /// and third are the exact space's own reasoning — an image that moves when
    /// the graph moves, and terms because two spaces built from byte-identical
    /// artifacts under different bindings return different terms at position
    /// zero of every row.
    ///
    /// The **parameters** are the difference, and it is a real contract
    /// distinction rather than an oversight copied across. The exact relation
    /// excludes its bound on work, because that bound decides how hard a search
    /// tries and never which rows exist or how they rank. Here `ef_search` does
    /// decide: the beam is never widened, so a narrower beam finds different
    /// rows and ranks them differently. A parameter that changes the answer
    /// belongs in the identity of the thing that answered.
    #[must_use]
    pub fn generation(&self) -> &str {
        &self.generation
    }

    /// The decoded index every invocation searches.
    #[must_use]
    pub fn index(&self) -> &HnswIndex<A> {
        &self.index
    }

    /// The RDF term row `row` stands for.
    #[must_use]
    pub fn term(&self, row: usize) -> Option<&TermValue> {
        self.rows.term(row)
    }

    /// The row `term` occupies, if this space holds it.
    #[must_use]
    pub fn row_of(&self, term: &TermValue) -> Option<usize> {
        self.rows.row_of(term)
    }

    /// A relation over this space.
    #[must_use]
    pub fn relation(self: &Arc<Self>) -> HnswRelation<A> {
        HnswRelation::new(Arc::clone(self))
    }
}

// ---------------------------------------------------------------------------
// The relation
// ---------------------------------------------------------------------------

/// The [`PropertyFunction`] a host registers to make an [`HnswSpace`] queryable.
///
/// [`Volatility::Stable`]: the graph is frozen and every distance is a pure function of two
/// of its rows computed by a shared kernel under the index's arithmetic. Under [`Exact`] an
/// invocation's rows are the same on the main thread, on a fork-join worker, and on
/// `wasm32-unknown-unknown`; under [`Reassociated`](purrdf_core::distance::Reassociated)
/// they are the same wherever this process runs the dispatch path the index recorded, and
/// a process that cannot run it is refused.
#[derive(Debug, Clone)]
pub struct HnswRelation<A: Arithmetic = Exact> {
    /// The space every invocation searches.
    space: Arc<HnswSpace<A>>,
    /// The declared modes, materialized once so [`PropertyFunction::modes`] can
    /// hand out a slice.
    modes: [BindingPattern; 2],
    /// What this relation's invocations actually did, counted rather than inferred.
    /// Shared with every clone, so a host that cloned the relation into a registry
    /// still reads the counts of the invocations that registry served.
    observations: Arc<HnswObservations>,
}

/// What [`HnswRelation`]'s invocations actually did, counted at the two places the
/// difference between a point lookup and a beam search is decided.
///
/// This exists because "a candidate-bound call does not traverse the graph" is a claim
/// about *work*, and the only honest evidence for a claim about work is a count of the
/// work. Timing is not evidence: a fixture small enough to run in a test is small enough
/// that traversing it and not traversing it take the same measurable time, so a timing
/// assertion would pass over an implementation that walks every layer and one that does
/// not.
///
/// Every counter is monotone for the life of the relation and is never reset here. A
/// caller that wants a delta reads the value before and after, which is the reading that
/// composes: a reset would race with any other invocation of a relation the registry may
/// run across workers.
///
/// # Not part of any identity
///
/// Nothing here reaches a row, a distance, an ordering or a generation. Two relations
/// over the same space answer identically whatever these say, so the counts are an
/// observation about one process's execution rather than a fact about the index — which
/// is why they live on the relation and not on [`HnswSpace`], whose generation folds
/// everything that decides an answer.
#[derive(Debug, Default)]
pub struct HnswObservations {
    /// Candidate-bound invocations answered by a row lookup.
    membership_lookups: AtomicU64,
    /// Invocations that entered the beam.
    searches: AtomicU64,
    /// Graph nodes the beam examined — one distance evaluation each.
    graph_candidates: AtomicU64,
    /// Distance evaluations the membership path performed.
    membership_distances: AtomicU64,
}

impl HnswObservations {
    /// How many **membership lookups** this relation has performed.
    ///
    /// One lookup is one candidate-bound invocation, answered by [`HnswSpace::row_of`] —
    /// a binary search over the space's canonical term order, with no float in it
    /// anywhere. Exactly one is performed per candidate-bound invocation, and nothing
    /// else performs any.
    #[must_use]
    pub fn membership_lookups(&self) -> u64 {
        self.membership_lookups.load(Ordering::Relaxed)
    }

    /// How many invocations have **entered the beam** — that is, have asked
    /// [`HnswIndex::search_rows_work`] to traverse the graph at all.
    ///
    /// Zero is the load-bearing value: an invocation that never entered the beam visited
    /// no node, consulted no layer and claimed no rank, whatever the graph holds. This
    /// relation has exactly one call site into the traversal and it increments this
    /// counter, so a zero here is not an inference about how much searching happened; it
    /// is the statement that none did.
    #[must_use]
    pub fn searches(&self) -> u64 {
        self.searches.load(Ordering::Relaxed)
    }

    /// How many **graph nodes** the searches examined, which is how many distances they
    /// computed.
    #[must_use]
    pub fn graph_candidates(&self) -> u64 {
        self.graph_candidates.load(Ordering::Relaxed)
    }

    /// How many distances the **membership path** computed.
    ///
    /// Zero for a candidate the space does not hold — that answer is settled by the term
    /// order alone and no vector is ever read — and one for a candidate it does, which
    /// is the single pairwise evaluation the emitted row's `?distance` position needs.
    /// Never more, whatever `k` the call carried and however wide the beam is, because a
    /// call that names its own candidate traverses nothing.
    #[must_use]
    pub fn membership_distances(&self) -> u64 {
        self.membership_distances.load(Ordering::Relaxed)
    }
}

impl HnswRelation {
    /// The `?neighbour` position: the retrieved term, and the ranked candidate.
    ///
    /// The positions belong to the call shape, which is the same under every arithmetic;
    /// they are named on the default type so a caller spells them without naming a law.
    pub const NEIGHBOUR: usize = KNN_NEIGHBOUR;
    /// The `?query` position: the term whose vector seeds the search.
    pub const QUERY: usize = KNN_QUERY;
    /// The `k` position: how many neighbours to retrieve.
    pub const COUNT: usize = KNN_COUNT;
    /// The `?distance` position: the retrieved term's distance from the query.
    pub const DISTANCE: usize = KNN_DISTANCE;
}

impl<A: Arithmetic> HnswRelation<A> {
    /// An approximate nearest-neighbour relation over `space`.
    #[must_use]
    pub fn new(space: Arc<HnswSpace<A>>) -> Self {
        Self {
            space,
            modes: [
                BindingPattern::from_code(KNN_MODE),
                BindingPattern::from_code(KNN_MEMBERSHIP_MODE),
            ],
            observations: Arc::new(HnswObservations::default()),
        }
    }

    /// What this relation's invocations have actually done.
    ///
    /// Handed back as a shared handle rather than a snapshot so a host can keep reading
    /// after the relation itself has been moved into a registry, which is the only
    /// arrangement in which the interesting question — did the exclusion lookups this
    /// producer served traverse anything — can be asked at all. See
    /// [`HnswObservations`].
    #[must_use]
    pub fn observations(&self) -> Arc<HnswObservations> {
        Arc::clone(&self.observations)
    }

    /// What this relation promises a consumer about the rows it emits.
    ///
    /// Lossy on the completeness axis, always: an HNSW search offers the
    /// candidates its beam reached and never certifies that nothing else
    /// matched. The evidence is the string this space carries, verbatim — the
    /// one the guard checked at bind time, not a re-wording of it.
    ///
    /// The order axis is **computed from a loss contract** by
    /// [`order_fidelity`] rather than typed as a literal here, so a profile
    /// that begins quantizing vectors reports a perturbed order without anyone
    /// remembering to come back and retype it.
    ///
    /// The contract it is computed from is this build's
    /// [`profile::loss_contract`], not one decoded out of the artifact, and
    /// that is equivalent rather than merely convenient: a loaded artifact
    /// whose guard publishes a different evidence revision or a different loss
    /// contract is REFUSED at bind time by
    /// [`crate::guard::validate_guard`], so an artifact this space could have
    /// been built from cannot disagree with the constant. The equivalence is
    /// enforced, and the enforcement is what this reads through.
    ///
    /// It has to be the compiled-in one for the other constructor in any case:
    /// [`HnswSpace::from_index`] is handed a graph this process built and there
    /// is no artifact to decode a contract out of. A field derived one way on
    /// one construction path and another way on the other is the modal
    /// optionality this workspace refuses.
    ///
    /// # What `vector_order` is, and why it is the only thing the host supplies
    ///
    /// The derivation above reads *this index's* loss contract, which describes
    /// what the HNSW build did to the vectors it was handed. It says nothing
    /// about what happened to them **before** that: a host that quantized,
    /// sketched or otherwise approximated its vectors and then called
    /// [`HnswIndex::build`] over the result has an order-perturbed producer, and
    /// no contract reachable from here records it. That fact has exactly one
    /// holder, so it is a parameter — and it is spelled at every call site
    /// rather than defaulted, because [`OrderFidelity::Faithful`] is the top of
    /// that axis and a default there would put the stronger claim into the mouth
    /// of a host that never spoke.
    ///
    /// The two are combined by [`composed_order_fidelity`], which only ever
    /// degrades: a host cannot declare its way back to `Faithful` over a profile
    /// that transforms vectors.
    ///
    /// # Why the host supplies nothing on the completeness axis
    ///
    /// Because this relation already declares the weaker claim there, from a
    /// fact it genuinely holds: a beam search offers the candidates it reached
    /// and never certifies that nothing else matched, so the completeness axis
    /// is [`Completeness::Lossy`] over every space, on every request, whatever
    /// corpus it covers. A consumer therefore already charges this stratum a
    /// deficit, an inflation and a rank-one contribution it never named — the
    /// failure that makes an undeclared approximation invisible *cannot* occur
    /// here, because there is no silence to read as completeness.
    ///
    /// A host with something further to say about its corpus would also have to
    /// displace the profile's own evidence, which is the one string the
    /// approximation contract requires to reach a consumer byte for byte. So the
    /// axis stays the relation's. A host that wants both disclosures edits the
    /// returned [`RankedDeclaration`] — every field of it is public — and owns
    /// the wording of the merged evidence itself.
    ///
    /// # The arithmetic enters both axes
    ///
    /// The space's evidence is the one the profile publishes for its index's arithmetic
    /// and dispatch path, so under [`Reassociated`](purrdf_core::distance::Reassociated)
    /// the completeness axis carries the reassociated sentence verbatim. And a law whose bits depend on the dispatch path
    /// perturbs the order: near-tied rows may order differently from the exact distances,
    /// so the order axis is composed with [`OrderFidelity::Perturbed`] carrying that
    /// arithmetic's own evidence -- the words the reassociated kNN relation carries, so a
    /// fused answer names one kernel one way. Under [`Exact`] there is no such evidence and
    /// both axes are what they always were.
    #[must_use]
    pub fn fidelity(&self, vector_order: OrderFidelity) -> RankFidelity {
        let evidence: Arc<str> = Arc::from(self.space.evidence());
        let arithmetic_order = self.space.index().arithmetic().evidence().map_or(
            OrderFidelity::Faithful,
            |evidence| OrderFidelity::Perturbed {
                evidence: Arc::from(evidence),
            },
        );
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::clone(&evidence),
            },
            order: composed_order_fidelity(
                composed_order_fidelity(
                    order_fidelity(&profile::loss_contract(), &evidence),
                    arithmetic_order,
                ),
                vector_order,
            ),
        }
    }

    /// The ranked declaration this relation registers under.
    ///
    /// Mirrors the exact kNN relation's shape, because the call shape is
    /// identical — same four positions, same `xsd:integer` neighbour count,
    /// same `xsd:double` distance — and differs in exactly the fact that
    /// matters: what it promises about the rows.
    ///
    /// `domains` is the caller's, never inferred: which blocks of its candidate
    /// universe this space draws from is a fact about the host's corpus and
    /// nothing here can know it.
    ///
    /// `vector_order` is the caller's for the same kind of reason: whether the
    /// vectors handed to [`HnswIndex::build`] were already approximations of the
    /// values the caller meant is a fact about the host's pipeline, upstream of
    /// anything this space can read. See [`Self::fidelity`] for how it composes
    /// with the axis derived here, and for why the completeness axis is not a
    /// parameter beside it.
    #[must_use]
    pub fn ranked_declaration(
        &self,
        stratum: Iri,
        seed: TermKind,
        depth_datatype: String,
        vector_order: OrderFidelity,
        domains: CandidateDomains,
    ) -> RankedDeclaration {
        RankedDeclaration {
            stratum,
            accepted_terms: vec![AcceptedTerm {
                pattern: TermPattern::of_kind(seed),
                placements: vec![TermPlacement {
                    facet: RequestFacet::Value,
                    position: KNN_QUERY,
                    datatype: None,
                }],
            }],
            depth_placement: Some(DepthPlacement {
                position: KNN_COUNT,
                datatype: depth_datatype,
            }),
            candidate_position: KNN_NEIGHBOUR,
            // The space enforces distinct terms at construction, and one search
            // visits a node at most once, so a row cannot repeat within an
            // invocation.
            duplicates: DuplicatePolicy::Unique,
            fidelity: self.fidelity(vector_order),
            // Every distance this relation ranks by is computed under the index's
            // arithmetic, so the declaration names that law and the registry's
            // content fingerprint binds it.
            arithmetic: RankArithmetic::float_distance::<A>(),
            domains,
            // This relation projects a neighbour and a distance; it knows
            // nothing of a host's partition, so it has no position to read a
            // per-row block out of and says so — the same answer its exact
            // sibling gives, for the same reason. A host whose vector index
            // spans several blocks declares one producer per block, or declares
            // `CandidateDomains::Unrestricted`; see
            // `RankedDeclaration::block_position`.
            block_position: None,
            // An exclusion from this producer is a fact about its own TERM
            // UNIVERSE — the matrix holds a row for this term, or it holds none —
            // which is what `Membership` means, and it is exact HOWEVER LOSSY THE
            // SEARCH IS. That is the whole reason the axis this declaration is
            // lossy on does not decide this one: a term the matrix holds no row
            // for is a term no beam reaches at any `ef`, so refusing the basis on
            // completeness would reject a provably exact answer, and the registry
            // does not.
            //
            // The basis is not the ranked question. `is this candidate among your
            // best n` has absences that are not exclusions: a candidate outside
            // the best n is one this producer may still name at rank n, and a
            // consumer that read that absence as an exclusion would refuse the
            // fused read as a contradiction the first time the stream named it.
            // Those two questions are two points of this relation's mode lattice,
            // and which one an invocation asks is decided by the count — see
            // `KNN_MEMBERSHIP_MODE` and [`Self::open`].
            //
            // What makes the basis deliverable is that a lookup arrives in the
            // membership mode. The consumer renders the lookup with the count left
            // FREE and declares the candidate a prepare parameter, so the
            // admission pass sees `KNN_MEMBERSHIP_MODE` rather than the general
            // one, and the invocation reaches this relation as the point lookup it
            // was admitted as. The promise that the candidate really will be
            // supplied is enforced where the execution begins; a plan that
            // declared it and did not supply it is refused rather than run with
            // the position free.
            //
            // The lookup costs one binary search over canonical term order, with
            // no graph node visited and no distance compared against any other
            // row. `HnswObservations` counts both halves, so that is measured
            // rather than argued.
            exclusion: ExclusionBasis::Membership,
            mandatory: false,
        }
    }

    /// The space this relation searches.
    #[must_use]
    pub fn space(&self) -> &HnswSpace<A> {
        &self.space
    }
}

impl<A: Arithmetic> PropertyFunction for HnswRelation<A> {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        KNN_ARITY
    }

    /// Two modes, answering two different questions.
    ///
    /// `fbbf` is the ranked capability: the seed and the count are the two positions
    /// this relation cannot enumerate, and it projects the other two. It subsumes every
    /// access pattern of this arity that binds both inputs, so a bound `?neighbour`
    /// beside a bound count — *is this term among the `k` this beam offers* — is
    /// feasible under it and means what it has always meant.
    ///
    /// `bbff` is the **membership** capability, and it is a genuinely new one rather
    /// than a narrowing: subsumption is `bound(declared) ⊆ bound(invocation)`, and
    /// `fbbf` binds the count that this pattern leaves free, so `fbbf` does not subsume
    /// it and an invocation of this shape was infeasible before it was declared. What
    /// it adds is the point lookup [`Self::open`] documents — *do you hold this term at
    /// all* — with the row bound [`Self::rows_per_invocation`] reports for it. That is
    /// the question an [`ExclusionBasis::Membership`] lookup asks, this relation answers
    /// it, and the ranked declaration offers the basis on that footing: a lookup is
    /// rendered with the count free and the candidate declared to the prepare, so it is
    /// admitted against THIS pattern rather than against the one beside it.
    ///
    /// Adding it takes nothing away from the pattern beside it. A count-bound call
    /// keeps its `k` cut, because the two shapes are two points of the lattice and the
    /// invocation's own pattern decides which question was asked.
    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// The declared upper bound on the number of rows one invocation may emit under `mode`.
    ///
    /// Mirrors the exact kNN relation's positional bound — `min(1, rows)` when
    /// `?neighbour` is bound — and otherwise takes the **smallest** of the three
    /// ceilings that really apply: the rows the space holds, the guard's
    /// `max_neighbours`, and the beam width `ef_search`.
    ///
    /// # Why `ef_search` belongs in this bound
    ///
    /// Because it is the ceiling that actually stops the read most often, and
    /// leaving it out makes a consumer's status wrong. `ef_search` is part of
    /// the declared artifact identity and is never widened to fit a request, so
    /// a `k` larger than the beam is answered with fewer than `k` rows
    /// ([`HnswIndex::search_rows`]). A consumer planning a read from this
    /// declaration would then ask for a depth the beam cannot reach, never
    /// see its limit met, and be told the stream was `Exhausted` — a
    /// completeness-flavoured ending for a read the *parameters* cut short.
    ///
    /// Declaring the beam does not refuse anything a caller could otherwise
    /// have had: no search ever returned those rows. It stops the plan asking
    /// for them, so the ending it gets back is the true one.
    ///
    /// Unlike the exact path this is still an upper bound **only**, never a
    /// promise that a call attains it: the beam may miss the query's exact
    /// neighbourhood even within its width, which is the approximation this
    /// relation exists to offer and which it now also declares through
    /// [`Self::ranked_declaration`].
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        let rows = self.space.row_count() as u64;
        if mode.is_bound(KNN_NEIGHBOUR) {
            rows.min(1)
        } else {
            rows.min(self.space.guard().max_neighbours())
                .min(self.space.beam_width())
        }
    }

    /// Begin one approximate nearest-neighbour invocation.
    ///
    /// # A free `k` asks a different question, and is answered by membership
    ///
    /// The **count** decides which of two questions an invocation is, and nothing else
    /// does.
    ///
    /// With `k` **bound** — every invocation `KNN_MODE` subsumes, which is every
    /// invocation that was feasible before `KNN_MEMBERSHIP_MODE` existed — the call
    /// means what it has always meant: traverse the graph and offer the `k` the beam
    /// reached, filtering `?neighbour` and `?distance` afterwards. A bound `?neighbour`
    /// is therefore still *is this term among the `k` this beam offers*, and a term the
    /// offer leaves out is still an empty answer that is no evidence of absence.
    /// Nothing on this path changed when the membership mode was declared, and nothing
    /// may: it is a public query surface, and a producer that quietly answered a wider
    /// question here would change the rows of a query nobody edited.
    ///
    /// With `k` **free** and `?neighbour` bound — `KNN_MEMBERSHIP_MODE`, a pattern
    /// `KNN_MODE` does not subsume and which was infeasible until it was declared —
    /// the call asks *do you hold this term at all*. That is the question
    /// [`ExclusionBasis::Membership`] declares, and `k` is no part of it: there is one
    /// candidate and no offer to size. It is answered by [`HnswSpace::row_of`] — a
    /// binary search over canonical term order, with no vector read, no node visited and
    /// no rank claimed — and the answer is one row where the space holds the term and
    /// none where it does not. [`HnswObservations`] counts both halves, so "the graph
    /// was not traversed" is an observation a test reads rather than an inference from a
    /// timing.
    ///
    /// The membership answer is **exact however lossy the beam is**, which is the whole
    /// reason this relation can declare a basis at all: a term the matrix holds no row
    /// for is a term no traversal can reach at any `ef`, in any layer, from any entry
    /// point.
    ///
    /// Splitting the two is what makes the exclusion lookup sound. A lookup is rendered
    /// as this call with the candidate bound, so a producer that answered it by
    /// traversing would report an absence for a term it is about to name at rank five,
    /// and a consumer that believed it would refuse the fused read as a contradiction. A
    /// membership answer cannot contradict a row, because every row a search can name is
    /// a row this lookup finds.
    ///
    /// The emitted `?distance` is the true one: a single pairwise evaluation through
    /// [`HnswIndex::row_distance`], which is the same [`Kernel`] over the same components
    /// and norms a traversal binds its query with, so it is bit-identical to the value
    /// the beam would have produced for that pair. What a point lookup cannot produce is
    /// a *rank* — one plus the number of rows nearer the seed is a fact about every other
    /// row — and this relation does not claim one.
    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.open_admitted(
            args,
            ceiling,
            purrdf_sparql_eval::WorkspaceCapability::resident(),
        )
    }

    fn open_admitted(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        workspace: purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        KnnInvocation::open_owned(
            "the HNSW relation",
            args,
            ceiling,
            &self.space.rows,
            self.space.guard(),
            &self.observations.membership_lookups,
            workspace,
        )?
        .cursor(HnswSource {
            space: Arc::clone(&self.space),
            observations: Arc::clone(&self.observations),
        })
    }
}

/// How an [`HnswRelation`] ranks: a beam search over the decoded graph, and one
/// pairwise distance for a membership answer, counted on the relation's observations.
#[derive(Debug)]
struct HnswSource<A: Arithmetic> {
    /// The space being searched.
    space: Arc<HnswSpace<A>>,
    /// The relation's counters, so the work a cursor does is observable from the
    /// relation a host still holds after moving it into a registry.
    observations: Arc<HnswObservations>,
}

impl<A: Arithmetic> RankSource for HnswSource<A> {
    /// The **only** call site of [`HnswIndex::search_rows_work`] in this relation, which
    /// is what makes [`HnswObservations::searches`] a measurement rather than an estimate.
    fn search(&self, query_row: usize, select_k: usize) -> Result<(Vec<Ranked>, u64), EvalError> {
        let (ranked, work) = self.search_admitted(
            query_row,
            select_k,
            &purrdf_sparql_eval::WorkspaceCapability::resident(),
        )?;
        let ranked = ranked.try_into_resident().map_err(|_| {
            EvalError::WorkspaceUnpriced("raw extraction of admitted HNSW source ranking")
        })?;
        Ok((ranked, work))
    }

    fn search_admitted(
        &self,
        query_row: usize,
        select_k: usize,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<(purrdf_sparql_eval::AdmittedVec<Ranked>, u64), EvalError> {
        self.observations.searches.fetch_add(1, Ordering::Relaxed);
        let (ranked, work) = self
            .space
            .index
            .search_rows_work_admitted(query_row, select_k, workspace)
            .map_err(|error| eval_error_admitted("the HNSW search failed", error, workspace))?;
        self.observations
            .graph_candidates
            .fetch_add(work, Ordering::Relaxed);
        Ok((ranked, work))
    }

    /// One pairwise evaluation: no node is visited and no layer is consulted.
    fn distance(&self, query_row: usize, row: usize) -> Result<f64, EvalError> {
        self.distance_admitted(
            query_row,
            row,
            &purrdf_sparql_eval::WorkspaceCapability::resident(),
        )
    }

    fn distance_admitted(
        &self,
        query_row: usize,
        row: usize,
        workspace: &purrdf_sparql_eval::WorkspaceCapability,
    ) -> Result<f64, EvalError> {
        self.observations
            .membership_distances
            .fetch_add(1, Ordering::Relaxed);
        self.space
            .index
            .row_distance(query_row, row)
            .map_err(|error| {
                eval_error_admitted("the HNSW membership lookup failed", error, workspace)
            })
    }

    fn term(&self, row: usize) -> Option<&TermValue> {
        self.space.term(row)
    }

    fn generation(&self) -> Arc<str> {
        Arc::clone(&self.space.generation)
    }
}

/// The order fidelity an index with this loss contract can promise.
///
/// A pure function of the contract, and a function rather than an inline
/// expression on purpose: written as a literal at its one call site the
/// [`OrderFidelity::Perturbed`] branch would never execute, and the branch that
/// never executes is the one carrying the claim that no finite score bound
/// exists. Here both branches are reachable and both are tested — directly, from
/// a contract a test writes, because this build's
/// [`profile::loss_contract`] is a constant with `transforms_vectors: false` and
/// reaches only the first of them.
///
/// **This is half of the answer, not the whole of it.** It describes what the
/// HNSW build did to the vectors it was given, and a host may have approximated
/// them before handing them over. [`composed_order_fidelity`] combines the two,
/// and it is that value — never this one alone — that reaches a declaration.
///
/// # Why `transforms_vectors` is the right discriminator
///
/// Because it is exactly the question "were the values this index compared the
/// values the caller meant?". A graph over untransformed vectors computes exact
/// distances for every candidate it visits, so the rows it returns are in true
/// relative order and it fails only to *visit* — lossy, but faithful, and a
/// named row's true rank is at least its emitted rank. Quantize the vectors and
/// that stops holding: a row can be ranked better than it was due, because the
/// distance that ranked it was an approximation of the real one, and no finite
/// bound on the error survives.
#[must_use]
pub fn order_fidelity(contract: &IndexLossContract, evidence: &Arc<str>) -> OrderFidelity {
    if contract.transforms_vectors {
        OrderFidelity::Perturbed {
            evidence: Arc::clone(evidence),
        }
    } else {
        OrderFidelity::Faithful
    }
}

/// The order fidelity of a search whose index may perturb the order **and**
/// whose vectors the host may already have perturbed before the index saw them.
///
/// The one composition every ranked producer in the workspace uses, defined
/// beside [`RankFidelity`] in `purrdf-sparql-eval` and re-exported here under the
/// path this crate has always published it at. It is the meet of the two on the
/// order axis: perturbed when either input is, faithful only when both are, and
/// when both are perturbed the host's evidence is the one carried.
///
/// # Why nothing is lost here when the host's evidence wins the axis
///
/// The derived string is not the profile's disclosure to a consumer — it is a
/// *second copy* of it. [`HnswRelation::fidelity`] derives both axes from one
/// string, the space's own [`HnswSpace::evidence`], and publishes it on the
/// completeness axis, which is [`Completeness::Lossy`] unconditionally and so
/// always carries it. So the profile's evidence reaches a declaration, a stream
/// contract and a fused trailer byte for byte on every path through this
/// function, and the host's words reach them beside it instead of displacing
/// them.
pub use purrdf_sparql_eval::composed_order_fidelity;

/// The content identity of a space over `index` bound to `terms`.
///
/// Framed part by part so no concatenation of one part can be read as another,
/// using the same `<tag><len><bytes>` discipline the registry fingerprints use.
/// Nothing here reads a clock, a counter or an RNG: two spaces built from the
/// same graph, parameters and terms digest identically on every target.
fn space_generation<A: Arithmetic>(index: &HnswIndex<A>, terms: &[TermValue]) -> Arc<str> {
    let mut bytes = Vec::new();
    frame_be_labelled(&mut bytes, "domain", SPACE_GENERATION_DOMAIN.as_bytes());
    frame_be_labelled(&mut bytes, "image", &index.canonical_image());
    frame_be_labelled(
        &mut bytes,
        "parameters",
        &profile::parameters(index.params()),
    );
    frame_be_labelled(&mut bytes, "row-count", &(terms.len() as u64).to_be_bytes());
    let mut term_bytes = Vec::new();
    for term in terms {
        term_bytes.clear();
        term.canonical_bytes(&mut term_bytes);
        frame_be_labelled(&mut bytes, "term", &term_bytes);
    }
    Arc::from(ContentDigest::of(&bytes).to_hex().as_str())
}

/// The domain separator every HNSW space generation opens with, so this digest
/// can never equal a digest of the same bytes taken for another purpose.
const SPACE_GENERATION_DOMAIN: Domain = Domain::new(b"purrdf-hnsw/space-generation-v1");

/// Everything [`HnswRelation::ranked_declaration`] cannot derive: the five facts
/// a host states about its own corpus and pipeline when it registers a space as
/// a ranked producer.
///
/// Carried as one value because that is what they are — one host's statement,
/// made at one moment, about one space — and because a registration function
/// taking them loose would be eight positional arguments of which five are the
/// same argument. Every field is public and none is optional: a bare-field
/// struct with no builder and no `Default` is what makes an omitted disclosure a
/// compile error rather than a silent top-of-lattice claim, which is the same
/// discipline [`RankedDeclaration`] itself follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedHnswRegistration {
    /// The caller's stratum IRI: the label rows from this space are ranked and
    /// fused under.
    pub stratum: Iri,
    /// Which RDF term kind the host's requests seed a search with.
    pub seed: TermKind,
    /// The caller's datatype IRI for the rendered neighbour count.
    pub depth_datatype: String,
    /// What the host did to its vectors **before** [`HnswIndex::build`] saw
    /// them. See [`HnswRelation::fidelity`].
    pub vector_order: OrderFidelity,
    /// Which blocks of the candidate universe this space may name.
    pub domains: CandidateDomains,
}

/// Register `space` as a relation under the caller's predicate `iri`.
///
/// PurRDF supplies no IRI: the caller's predicate is what a query text names the provider
/// by, which is the second of the three approximation channels the contract requires.
pub fn register_ranked_hnsw_relation<A: Arithmetic>(
    registry: &mut PropertyFunctionRegistry,
    iri: impl Into<String>,
    space: Arc<HnswSpace<A>>,
    declared: RankedHnswRegistration,
) {
    let RankedHnswRegistration {
        stratum,
        seed,
        depth_datatype,
        vector_order,
        domains,
    } = declared;
    let relation = HnswRelation::new(space);
    let declaration =
        relation.ranked_declaration(stratum, seed, depth_datatype, vector_order, domains);
    registry.register_ranked(iri, Arc::new(relation), declaration);
}

/// Register `space` as a plain relation under the caller's predicate `iri`.
///
/// PurRDF supplies no IRI: the caller's predicate is what a query text names the
/// provider by, which is the second of the approximation contract's governed
/// channels.
///
/// # This relation is invisible to the retrieval ladder, on purpose
///
/// A relation registered here declares no ranked contract, so it forms no
/// stratum, produces no trailer entry, and a retrieval request that wanted it
/// reports the term as unserved rather than fusing a stream that never
/// declared what it promises. That is a loud refusal and never a quiet false
/// claim — which is why this function can safely stay: the failure mode of
/// reaching for it by mistake is a planner that says so.
///
/// A host composing this space into a fused answer wants
/// [`register_ranked_hnsw_relation`] instead, which needs the stratum, seed
/// kind, depth datatype, vector-order disclosure and domain declaration a
/// plain-SPARQL host has no reason to invent.
pub fn register_hnsw_relation<A: Arithmetic>(
    registry: &mut PropertyFunctionRegistry,
    iri: impl Into<String>,
    space: Arc<HnswSpace<A>>,
) {
    registry.register(iri, Arc::new(HnswRelation::new(space)));
}

#[cfg(test)]
mod tests {
    use purrdf_core::DistanceMetric;

    use super::*;
    use crate::{Params, VectorMatrix};
    use purrdf_xsd::datatype::XSD_INTEGER;

    fn matrix(rows: usize, dims: usize) -> VectorMatrix {
        let mut state = 0x5151_5151_5151_5151_u64;
        let mut data = Vec::with_capacity(rows * dims);
        for _ in 0..rows * dims {
            data.push(purrdf_testkit::rng::signed_unit_step_nonzero(
                &mut state, 0.25,
            ));
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

    /// The space generation is frozen over a fixed index and fixed terms: it reaches
    /// retrieval evidence, so a moved value is a changed persisted identity.
    #[test]
    fn the_space_generation_is_frozen() {
        let index = HnswIndex::build(
            matrix(6, 4),
            &DistanceMetric::SquaredEuclidean,
            Params::new(4, 8, 16, 8).expect("valid"),
        )
        .expect("builds");
        assert_eq!(
            &*space_generation(&index, &terms(6)),
            "88d28fb13f55f3339a84a314e839558f471be39e9f715a371c8897cfa0dd7655"
        );
    }

    #[test]
    fn a_space_assembled_from_an_index_is_held_to_the_guards_candidate_bound() {
        let build = || {
            HnswIndex::build(
                matrix(16, 4),
                &DistanceMetric::SquaredEuclidean,
                Params::new(4, 8, 16, 8).expect("valid"),
            )
            .expect("builds")
        };
        let error =
            HnswSpace::from_index(build(), terms(16), KnnGuard::new(15, 16).expect("valid"))
                .expect_err("sixteen rows exceed a fifteen-candidate bound");
        assert!(matches!(error, EvalError::Config(_)), "got {error:?}");
        assert!(error.to_string().contains("16 row(s)"), "got {error}");
        assert!(error.to_string().contains("15 candidate(s)"), "got {error}");
        // The neighbouring valid case: a bound exactly at the row count admits the space.
        assert!(
            HnswSpace::from_index(build(), terms(16), KnnGuard::new(16, 16).expect("valid"))
                .is_ok()
        );
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

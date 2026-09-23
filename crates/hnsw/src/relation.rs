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

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{
    ContentDigest, EmbeddingView, IndexLossContract, Iri, TargetId, TargetSetId, TargetSetView,
    TermValue, VectorSpaceId, verify_embedding,
};
use purrdf_sparql_eval::{
    AcceptedTerm, CandidateDomains, Completeness, DepthPlacement, DuplicatePolicy, EvalError,
    ExclusionBasis, IndexGeneration, Kernel, KnnGuard, OrderFidelity, PfArgs, PfArity, PfCursor,
    PfRow, PropertyFunction, PropertyFunctionRegistry, RankFidelity, Ranked, RankedDeclaration,
    RequestFacet, TermKind, TermPattern, TermPlacement, Volatility,
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
/// The general access pattern [`HnswRelation`] declares: the seed and the neighbour
/// count are the two positions it cannot enumerate, and the two it projects are free.
const HNSW_MODE: &str = "fbbf";
/// The **membership** access pattern [`HnswRelation`] declares beside [`HNSW_MODE`]:
/// the neighbour and the seed bound, and the neighbour count **free**.
///
/// The free count is the whole of what distinguishes the two questions, and it is
/// deliberately a *different mode* rather than a second reading of [`HNSW_MODE`]:
///
/// * with the count **bound**, `?neighbour ex:knn (?query k ?distance)` asks *is this
///   term among the `k` this beam offers*. That invocation is subsumed by
///   [`HNSW_MODE`] and always was, and it means exactly what it has always meant.
/// * with the count **free** it asks *do you hold this term at all* — a question `k`
///   is no part of, answered by [`HnswSpace::row_of`] alone.
///
/// One binding pattern cannot mean both without silently changing the answer to
/// somebody's query, so the lattice separates them. [`HNSW_MODE`] does **not** subsume
/// this pattern — subsumption is `bound(declared) ⊆ bound(invocation)`, and
/// [`HNSW_MODE`] binds the count that this leaves free — so this is a genuinely new
/// declared capability, with its own point
/// [`PropertyFunction::rows_per_invocation`]. It is the question an
/// [`ExclusionBasis::Membership`] lookup asks, this relation answers it, and a lookup
/// really does arrive in it: the consumer renders the exclusion call with the count free
/// and declares the candidate to the prepare, so the admission pass matches THIS pattern
/// rather than [`HNSW_MODE`]. See [`PropertyFunction::open`] and the `exclusion` field of
/// the ranked declaration.
const HNSW_MEMBERSHIP_MODE: &str = "bbff";

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
    /// The content identity of everything that decides this space's answers.
    generation: Arc<str>,
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
        // Derived once, here, so BOTH constructors carry it: `from_artifact`
        // delegates to this function, and a field present on one construction
        // path and absent on the other is precisely the modal optionality this
        // workspace refuses.
        let generation = space_generation(&index, &terms);
        Ok(Self {
            index: Arc::new(index),
            terms,
            rows_by_term,
            guard,
            evidence: profile::LOSS_EVIDENCE.to_owned(),
            generation,
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
    /// An approximate nearest-neighbour relation over `space`.
    #[must_use]
    pub fn new(space: Arc<HnswSpace>) -> Self {
        Self {
            space,
            modes: [
                BindingPattern::from_code(HNSW_MODE),
                BindingPattern::from_code(HNSW_MEMBERSHIP_MODE),
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

    /// The `?neighbour` position: the retrieved term, and the ranked candidate.
    pub const NEIGHBOUR: usize = HNSW_NEIGHBOUR;
    /// The `?query` position: the term whose vector seeds the search.
    pub const QUERY: usize = HNSW_QUERY;
    /// The `k` position: how many neighbours to retrieve.
    pub const COUNT: usize = HNSW_COUNT;
    /// The `?distance` position: the retrieved term's distance from the query.
    pub const DISTANCE: usize = HNSW_DISTANCE;

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
    #[must_use]
    pub fn fidelity(&self, vector_order: OrderFidelity) -> RankFidelity {
        let evidence: Arc<str> = Arc::from(self.space.evidence());
        RankFidelity {
            completeness: Completeness::Lossy {
                evidence: Arc::clone(&evidence),
            },
            order: composed_order_fidelity(
                order_fidelity(&profile::loss_contract(), &evidence),
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
                    position: Self::QUERY,
                    datatype: None,
                }],
            }],
            depth_placement: Some(DepthPlacement {
                position: Self::COUNT,
                datatype: depth_datatype,
            }),
            candidate_position: Self::NEIGHBOUR,
            // The space enforces distinct terms at construction, and one search
            // visits a node at most once, so a row cannot repeat within an
            // invocation.
            duplicates: DuplicatePolicy::Unique,
            fidelity: self.fidelity(vector_order),
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
            // `HNSW_MEMBERSHIP_MODE` and [`Self::open`].
            //
            // What makes the basis deliverable is that a lookup arrives in the
            // membership mode. The consumer renders the lookup with the count left
            // FREE and declares the candidate a prepare parameter, so the
            // admission pass sees `HNSW_MEMBERSHIP_MODE` rather than the general
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
        if mode.is_bound(HNSW_NEIGHBOUR) {
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
    /// With `k` **bound** — every invocation `HNSW_MODE` subsumes, which is every
    /// invocation that was feasible before `HNSW_MEMBERSHIP_MODE` existed — the call
    /// means what it has always meant: traverse the graph and offer the `k` the beam
    /// reached, filtering `?neighbour` and `?distance` afterwards. A bound `?neighbour`
    /// is therefore still *is this term among the `k` this beam offers*, and a term the
    /// offer leaves out is still an empty answer that is no evidence of absence.
    /// Nothing on this path changed when the membership mode was declared, and nothing
    /// may: it is a public query surface, and a producer that quietly answered a wider
    /// question here would change the rows of a query nobody edited.
    ///
    /// With `k` **free** and `?neighbour` bound — `HNSW_MEMBERSHIP_MODE`, a pattern
    /// `HNSW_MODE` does not subsume and which was infeasible until it was declared —
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
                 the neighbours of a seed and cannot enumerate seeds, which is why both of \
                 its declared modes — `{HNSW_MODE}` and `{HNSW_MEMBERSHIP_MODE}` — demand it"
            )));
        };
        let query_row = self.space.row_of(query);

        // THE COUNT DECIDES WHICH QUESTION THIS IS, and nothing else does. A bound count
        // is the ranked read this relation has always performed, down to the `k` cut on a
        // bound `?neighbour`; a free count is the membership lookup. Branching on the
        // candidate instead would make one binding pattern mean two questions and would
        // silently widen the answer to `?n ex:knn (?q k ?d)` — a query nobody edited.
        let (answer, count_term) = match args.get(HNSW_COUNT) {
            Some(count) => {
                let k = neighbour_count(count, self.space.guard())?;
                // `?neighbour` and `?distance` are both filtered AFTER the ranking, so the
                // ceiling is withheld from the selection when either is bound: pushing it
                // down would rank a prefix, let the cursor filter it, and report fewer
                // rows than the engine asked for as an exhausted answer.
                let post_selection_filtered =
                    args.get(HNSW_NEIGHBOUR).is_some() || args.get(HNSW_DISTANCE).is_some();
                let select_k = if post_selection_filtered {
                    k
                } else {
                    ceiling.map_or(k, |ceiling| k.min(usize::try_from(ceiling).unwrap_or(k)))
                };
                (
                    Answer::Search {
                        query_row,
                        select_k,
                    },
                    count.clone(),
                )
            }
            None => {
                let Some(candidate) = args.get(HNSW_NEIGHBOUR) else {
                    return Err(EvalError::function(format!(
                        "the neighbour count at position {HNSW_COUNT} is free and so is the \
                         neighbour at position {HNSW_NEIGHBOUR}; how many neighbours to \
                         retrieve is a question this relation is asked, not one it answers, \
                         so the only call it serves without a count is the membership lookup \
                         `{HNSW_MEMBERSHIP_MODE}`, which names the one term it is about"
                    )));
                };
                self.observations
                    .membership_lookups
                    .fetch_add(1, Ordering::Relaxed);
                (
                    Answer::Membership(query_row.zip(self.space.row_of(candidate))),
                    universe_size(self.space.row_count()),
                )
            }
        };

        Ok(Box::new(HnswCursor {
            space: Arc::clone(&self.space),
            observations: Arc::clone(&self.observations),
            answer,
            query_term: query.clone(),
            count_term,
            bound: args.flattened().map(<Option<&TermValue>>::cloned).collect(),
            ranked: None,
            at: 0,
            remaining: ceiling,
            unreported_work: 0,
        }))
    }
}

/// The value the count position carries in a **membership** answer: the number of rows
/// the space holds, as an `xsd:integer`.
///
/// # Why the position needs a value at all, and why this is the one
///
/// A [`PfRow`] carries a value for every flattened position, so a mode that leaves the
/// count free must still fill it. In [`HNSW_MODE`] that position is an *input* — the
/// request — and is echoed back verbatim. In [`HNSW_MEMBERSHIP_MODE`] there is no request
/// to echo: the caller asked *do you hold this term*, a question no `k` is part of.
///
/// So under that mode the position is an **output**, and what it outputs is a fact about
/// the producer rather than a request it was never given: the size of its term universe.
/// That is exactly the quantity a membership answer is about — the lookup says *this term
/// is one of my rows*, and this says *how many rows there are* — it is single-valued, so
/// the mode's declared row bound of one is exact, and it costs a length read.
///
/// It is emphatically **not** a fabricated `k`. Inventing a request the caller did not
/// make would put a claim about rank into a row that traversed nothing, which is the one
/// thing a point lookup must never do.
fn universe_size(rows: usize) -> TermValue {
    TermValue::typed_literal(rows.to_string(), XSD_INTEGER)
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
/// What one invocation is going to do, decided in [`HnswRelation::open`] from the access
/// pattern it arrived in.
///
/// Two shapes rather than one with an optional field, because they are two different
/// questions: one traverses a graph for a neighbourhood and the other asks whether a
/// named term has a row at all. Naming them apart is what makes "the beam was not
/// entered" a branch a reader can see rather than a condition buried in a selection size.
#[derive(Debug)]
enum Answer {
    /// Rank the seed's neighbourhood: the offer of candidates.
    Search {
        /// The seed's row, or `None` when the space does not hold the seed term.
        query_row: Option<usize>,
        /// How many neighbours the ranking retains.
        select_k: usize,
    },
    /// Answer *do you hold this candidate*: the seed's row and the candidate's row, or
    /// `None` when the space holds no row for one of them — which is equally an
    /// exclusion, because a search from a seed with no row names nothing at all.
    Membership(Option<(usize, usize)>),
}

#[derive(Debug)]
struct HnswCursor {
    /// The space being searched.
    space: Arc<HnswSpace>,
    /// The relation's counters, so the work this cursor does is observable from the
    /// relation a host still holds after moving it into a registry.
    observations: Arc<HnswObservations>,
    /// What this invocation is going to do.
    answer: Answer,
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
    /// Produce this invocation's rows if they have not been produced yet, recording the
    /// candidates it examined.
    ///
    /// Laziness is what makes "no rows wanted" and "no work done" the same statement: the
    /// engine checks its ceiling before the first pull, so a call with an exhausted ceiling
    /// never reaches this function and is charged nothing.
    ///
    /// The **only** call site of [`HnswIndex::search_rows_work`] in this relation, which
    /// is what makes [`HnswObservations::searches`] a measurement rather than an estimate:
    /// an invocation that did not come through this branch traversed nothing, because
    /// there is no other way for it to have done so.
    fn ensure_ranked(&mut self) -> Result<(), EvalError> {
        if self.ranked.is_some() {
            return Ok(());
        }
        let (ranked, work) = match self.answer {
            Answer::Search {
                query_row: Some(row),
                select_k,
            } => {
                self.observations.searches.fetch_add(1, Ordering::Relaxed);
                let (ranked, work) = self
                    .space
                    .index
                    .search_rows_work(row, select_k)
                    .map_err(|e| EvalError::data(format!("the HNSW search failed: {e}")))?;
                self.observations
                    .graph_candidates
                    .fetch_add(work, Ordering::Relaxed);
                (ranked, work)
            }
            Answer::Search {
                query_row: None, ..
            }
            | Answer::Membership(None) => (Vec::new(), 0),
            // One pairwise evaluation, charged as the one candidate it examined. No node
            // is visited and no layer is consulted.
            Answer::Membership(Some((query_row, row))) => {
                self.observations
                    .membership_distances
                    .fetch_add(1, Ordering::Relaxed);
                let distance = self.space.index.row_distance(query_row, row).map_err(|e| {
                    EvalError::data(format!("the HNSW membership lookup failed: {e}"))
                })?;
                (vec![Ranked { distance, row }], 1)
            }
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
    /// The generation of the space that answered.
    ///
    /// The space is immutable once built, so the reading taken here is true for
    /// every row this cursor goes on to emit.
    ///
    /// The `Arc` is cloned rather than the string: the space already holds the
    /// one encoding of what it attested, and handing back a pointer to it is
    /// what keeps a per-invocation allocation off this path.
    fn generation(&self) -> IndexGeneration {
        IndexGeneration::Declared(Arc::clone(&self.space.generation))
    }

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
/// The meet of the two on a two-element lattice: the result is
/// [`OrderFidelity::Perturbed`] when either input is, and
/// [`OrderFidelity::Faithful`] only when both are. It degrades and never
/// upgrades, which is the property that matters — a host passing `Faithful`
/// says "I did nothing to the vectors", not "the index is faithful", and cannot
/// talk a transforming profile back up to the top of the axis.
///
/// # Which evidence survives, and why nothing is lost
///
/// When both inputs are perturbed the host's evidence is the one carried, and
/// the derived string is **not** concatenated onto it, re-worded, or summarized:
/// an axis holds one disclosure and a consumer reads those bytes rather than a
/// composition of them.
///
/// That costs the reader nothing, because the derived string is not the
/// profile's disclosure to a consumer — it is a *second copy* of it.
/// [`HnswRelation::fidelity`] derives both axes from one string, the space's
/// own [`HnswSpace::evidence`], and publishes it on the completeness axis, which
/// is [`Completeness::Lossy`] unconditionally and so always carries it. So the
/// profile's evidence reaches a declaration, a stream contract and a fused
/// trailer byte for byte on every path through this function, and the host's
/// words reach them beside it instead of displacing them.
///
/// The host's is the one that must win the axis: it is the only one of the two a
/// reader could otherwise never obtain, and A16 of the producer contract is
/// exactly the rule that a consumer either receives such a fact from the
/// producer or never has it.
#[must_use]
pub fn composed_order_fidelity(derived: OrderFidelity, host: OrderFidelity) -> OrderFidelity {
    match host {
        OrderFidelity::Perturbed { evidence } => OrderFidelity::Perturbed { evidence },
        OrderFidelity::Faithful => derived,
    }
}

/// The content identity of a space over `index` bound to `terms`.
///
/// Framed part by part so no concatenation of one part can be read as another,
/// using the same `<tag><len><bytes>` discipline the registry fingerprints use.
/// Nothing here reads a clock, a counter or an RNG: two spaces built from the
/// same graph, parameters and terms digest identically on every target.
fn space_generation(index: &HnswIndex, terms: &[TermValue]) -> Arc<str> {
    let mut bytes = Vec::new();
    append_framed(&mut bytes, b"domain", SPACE_GENERATION_DOMAIN.as_bytes());
    append_framed(&mut bytes, b"image", &index.canonical_image());
    append_framed(
        &mut bytes,
        b"parameters",
        &profile::parameters(index.params()),
    );
    append_framed(
        &mut bytes,
        b"row-count",
        &(terms.len() as u64).to_be_bytes(),
    );
    let mut term_bytes = Vec::new();
    for term in terms {
        term_bytes.clear();
        term.canonical_bytes(&mut term_bytes);
        append_framed(&mut bytes, b"term", &term_bytes);
    }
    Arc::from(ContentDigest::of(&bytes).to_hex().as_str())
}

/// Append `value` to `out` under `tag`, both length-framed.
fn append_framed(out: &mut Vec<u8>, tag: &[u8], value: &[u8]) {
    out.extend_from_slice(&(tag.len() as u64).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}

/// The domain separator every HNSW space generation opens with, so this digest
/// can never equal a digest of the same bytes taken for another purpose.
const SPACE_GENERATION_DOMAIN: &str = "purrdf-hnsw/space-generation-v1";

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
pub fn register_ranked_hnsw_relation(
    registry: &mut PropertyFunctionRegistry,
    iri: impl Into<String>,
    space: Arc<HnswSpace>,
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

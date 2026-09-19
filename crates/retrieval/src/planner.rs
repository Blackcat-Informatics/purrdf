// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pure planner: one request, one registry, one statistics input, one plan.
//!
//! [`plan`] is a pure function. It opens no store, reads no file, consults no
//! clock and spawns nothing; every input it can see is a borrow, and the value
//! it returns is the whole of its output. That is what makes a plan
//! golden-testable as text in, text out, and reproducible across processes and
//! targets for the same request, registry and statistics.
//!
//! # Matching is a lookup, never inference
//!
//! A producer declares, as serializable data, which request-term shapes it
//! accepts — the registry's side table holds that declaration
//! ([`RankedDeclaration::accepted_terms`](purrdf_sparql_eval::RankedDeclaration::accepted_terms)),
//! supplied where the producer was registered. The planner
//! matches a request term against those declarations by a fixed lookup over
//! term kind, language and predicate — it never infers capability from a
//! producer's name, from a statistical signal, or from what other producers
//! accepted.
//!
//! A request term maps to the RDF term kind its retrieval targets:
//!
//! * a lexical term, a spatial term, a query embedding, and the endpoints of a
//!   temporal or numeric interval all target a **literal**, because that is what
//!   each of them is written as;
//! * an entity seed targets the kind its canonical lexical names (`<…>` an IRI,
//!   `_:…` a blank node, `"…"` a literal).
//!
//! A pattern's `datatype` constraint can never match, because no request term
//! carries a datatype; its `language` constraint matches only a lexical term
//! with that tag; its `predicate` constraint matches only a lexical, spatial,
//! temporal or numeric-range term carrying that predicate.
//!
//! Those two constraints are how a producer narrows *within* a term kind, and
//! narrowing is sometimes the point. Several modalities are written as literals,
//! so an unconstrained `Literal` pattern accepts all of them — a needle, a
//! geometry, an interval endpoint, a query embedding. A producer that wants only
//! needles says so with a `language` or `predicate` constraint, neither of which
//! a geometry-less, predicate-less embedding can satisfy. A producer that
//! constrains neither has declared that it takes whatever arrives written as a
//! literal, and what it receives is then decided by whether its placement can
//! render that term at all.
//!
//! # Matching is not receiving: only a placed term is bound
//!
//! A declaration can accept a shape and declare no placement for it. That
//! producer is called for the request and receives none of the term — a
//! legitimate declaration, and the shape of a producer whose ranking is
//! request-independent. It is selected like any other, but the term is **not**
//! bound to it: a binding is the claim that the producer received the term, and
//! recording one here would make
//! [`Plan::unserved_terms`](crate::Plan::unserved_terms) report nothing while
//! the emitted query contains no trace of the request. So a
//! producer is bound to exactly the terms whose matching alternative places at
//! least one facet, and a term that reached only placement-free acceptors is
//! reported as
//! [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason::AcceptedWithoutPlacement).
//!
//! A producer that carries nothing at all is still selected, and its stratum
//! still emits: this is how a request-independent ranking — a quality prior
//! fused as its own stratum — takes part in an answer without claiming to have
//! read the request. What it cannot do is make the request look served.
//!
//! # Matching is not enough: a producer must also be *invocable*
//!
//! Accepting a term's shape and being able to render that term into an argument
//! position are two different claims, and a producer can make the first without
//! the second: a query embedding and a geometry have no constant form under a
//! datatype nobody declared, a blank-node seed is a non-distinguished variable
//! rather than a ground value, and a relation that can only run with its depth
//! bound cannot be called by one that leaves the depth free. So after matching,
//! the planner runs the compiler's own
//! [`place`](crate::matching::place) and records
//! [`RejectionReason::UnsatisfiedConstraint`] for a producer that cannot be
//! invoked, rather than binding it and emitting a call that silently drops the
//! facet it could not write. If that leaves nothing bound, the request reaches
//! nothing and [`PlanError::NoApplicableProducers`] is the answer.
//!
//! # A term that reaches nothing says so
//!
//! Both passes above decide per **producer**, and a caller asks per **term**. A
//! request can name a modality this registry has no producer for — the request
//! lattice deliberately carries shapes ahead of the producers that answer them —
//! and a plan that merely omitted such a term would be indistinguishable from
//! one that served it and found nothing. So every term no binding carries is
//! recorded in [`Plan::unserved_terms`](crate::Plan::unserved_terms) with a
//! typed [`UnservedReason`](crate::UnservedReason): nothing accepted its shape,
//! something accepted it and declared nowhere to put it, or something could have
//! carried it and every such acceptor was then rejected. The request is still
//! planned — the terms that *do* reach a producer are answered — because
//! refusing the whole request over one unserved term would discard every answer
//! the others can give.
//!
//! A producer the registry declares **mandatory** is not exempt. If placement
//! refuses one, the plan records the rejection and
//! [`compile`](crate::compile) refuses the plan with
//! [`MissingMandatoryProducer`](crate::AdmissionError::MissingMandatoryProducer):
//! the registry insists that producer serve every request, and this request
//! cannot be delivered to it. That is a visible, typed refusal at the admission
//! waist, which is the only place a coverage claim is enforced.
//!
//! # The registry is read, never duplicated
//!
//! Every declaration the planner consumes — a producer's IRI, its capability,
//! its access modes and their row bounds — is read from the registry's own
//! [`describe`](purrdf_sparql_eval::PropertyFunctionRegistry::describe)
//! machinery. The planner declares no parallel `volatility`, `arity` or
//! access-pattern fields of its own; the seam's declarations are the single
//! source, and [`Statistics`] supplies the cardinalities that bound them.
//!
//! # The request's own bound is the third narrowing, and it has to be proved
//!
//! A [`RetrievalRequest`] states how much of the answer it is for
//! ([`ReadBound`]), and a bound on the rows is sometimes a bound on the *read*.
//! When it is, the depth this planner records is that bound rather than the
//! declaration — because the depth a plan records is the depth that is actually
//! read, and narrowing only the emitted `LIMIT` would leave
//! [`Plan::stratum_depths`](crate::Plan::stratum_depths) describing a read nobody
//! took.
//!
//! Whether it is, is decided by what the producers declared about their own
//! candidates and by nothing else. There is no caller hint and no mode:
//!
//! * **Every surviving stratum declares
//!   [`CandidateDomains::Within`](purrdf_sparql_eval::CandidateDomains::Within),
//!   the declared block sets are pairwise disjoint, and every one of those strata
//!   declares
//!   [`DuplicatePolicy::Unique`](purrdf_sparql_eval::DuplicatePolicy::Unique).**
//!   Then depth `min(declared, statistics-narrowed, k)` is *exact* — the proof is
//!   below, and it runs on two premises rather than one.
//! * **Anything else** — two strata that share a block, any stratum declaring
//!   [`Unrestricted`](purrdf_sparql_eval::CandidateDomains::Unrestricted), or any
//!   stratum declaring
//!   [`DuplicatePolicy::Allowed`](purrdf_sparql_eval::DuplicatePolicy::Allowed) —
//!   and the declared-or-statistics bound stands exactly as it does for a
//!   [`ReadBound::Complete`] request.
//!
//! ## The proof, for the disjoint case
//!
//! Write the surviving strata `s = 1..m`, each with a declared block set `D_s`,
//! pairwise disjoint. A candidate lies in exactly **one** block — that is the
//! partition axiom
//! [`DomainTag`](purrdf_sparql_eval::DomainTag) is defined by — and a producer
//! that names a candidate outside its own declaration is refused by name
//! ([`ProtocolError::OutsideDeclaredDomain`](crate::ProtocolError::OutsideDeclaredDomain))
//! rather than merged. So for a candidate `x` there is at most one `s` with
//! `block(x) ∈ D_s`, and only that stratum can name `x`.
//!
//! Its fused score is therefore a **single** term, `weight_s × decay(rank_s(x))`,
//! not a sum across strata. `decay` is non-increasing in rank — the profile's own
//! curve never rises, which fusion re-verifies per row
//! ([`ProtocolError::ContributionMismatch`](crate::ProtocolError::ContributionMismatch))
//! — so within one stratum the fused score is non-increasing in rank.
//!
//! Now take any `x` that its naming stratum `s` ranks at `r > k`. The rows at
//! ranks `1..r-1` of `s` name `r-1 ≥ k` **distinct** candidates — the second
//! premise, and the one
//! [`DuplicatePolicy::Unique`](purrdf_sparql_eval::DuplicatePolicy::Unique)
//! supplies: a stream that names an item at most once puts a different candidate
//! at every rank, so a count of ranks is a count of candidates. Each of them
//! carries score
//! `weight_s × decay(rank) ≥ weight_s × decay(r) = score(x)`. At least `k`
//! candidates therefore score at or above `x`. Where a score is strictly greater,
//! `x` loses on the first tie-break key; where the decay has saturated and the
//! scores are equal, `x` loses on the second, which is best stratum rank
//! ascending, and every one of those candidates has a smaller rank in `s` than
//! `x` does. The third key is never reached. So `x` is beaten by at least `k`
//! candidates and cannot be in the global top `k`.
//!
//! Contrapositive: every member of the global top `k` sits at per-stratum rank
//! `≤ k` in its own naming stratum. Reading each stratum to depth `k` therefore
//! materializes a superset of the answer, and the fusion over those prefixes
//! yields the same rows, the same scores and the same order as the fusion over
//! the whole streams — every score in the prefix is already complete, because the
//! only stratum that could have added to it is the one the row came from.
//!
//! ## `k`, not `k + 1`
//!
//! The bound is tight at `k` and the boundary case is the tie. Rank `k + 1` of a
//! stratum is beaten by the `k` candidates above it in that same stratum even
//! when the decay has saturated and their scores are equal, because the tie-break
//! is total and its next key is the stratum rank they win on. Nothing at rank
//! `k + 1` can enter a top `k`, so nothing is gained by reading it *as a value*.
//! One row past the depth is nonetheless read, and always has been: the probe
//! slot [`compile`](crate::compile) emits at `min(depth, declared) + 1` is what
//! separates "the plan stopped me" from "this is all there is", and it matters
//! more at a tight depth than at a loose one. That row is a read and never a
//! value, so it is the probe that supplies it and not the depth.
//!
//! ## Why `Unrestricted` and `Allowed` are excluded, and why neither is an
//! over-refusal
//!
//! The first premise — each candidate has at most one naming stratum — is
//! supplied by the domain declarations and by nothing else. An `Unrestricted`
//! declaration supplies none: it says the producer may name anything, which is
//! exactly the promise that lets two strata name one candidate and sum into it.
//! Inferring the premise from the *shape of the plan* instead — one stratum, so
//! there is nobody to overlap with — would rest the depth on a count rather than
//! on a promise, and it would stop holding the moment a second producer is
//! registered, silently.
//!
//! The second premise — that the rows above rank `r` name `r-1` distinct
//! candidates — is supplied by `DuplicatePolicy::Unique` and by nothing else. An
//! `Allowed` declaration asks its consumer to de-duplicate, and de-duplicating is
//! exactly what [`FusionStream`](crate::FusionStream) does with it: a repeated row
//! is validated, charged to the producer — the rank counter advances and the row
//! counts as pulled — and then discarded before it can become a head. So a
//! depth-`k` prefix of an `Allowed` stream carries `k` *rows* and can carry fewer
//! than `k` candidates, and a prefix holding four candidates where the answer
//! wants five is no longer a superset of the top `k`: the fifth candidate sits
//! past the depth and is never read, so the answer is short by one row, or — with
//! a second stratum to fill the gap — the right length with the wrong row in it.
//! A count of ranks is a count of candidates only under `Unique`, and it is that
//! identity the depth is derived from.
//!
//! Excluding either costs nothing but reading: the fallback is the depth the
//! registry and the statistics already set, which is the depth every such plan has
//! always carried. No request is refused, and no answer the narrowing still
//! applies to changes — the narrowing applies exactly where both premises hold,
//! and where either fails the stratum keeps reading as deep as it did before and
//! answers exactly as it would for a request stating no bound at all.
//!
//! # Both statistics bound the depth, and neither raises it
//!
//! A stratum's depth starts at the registry's declared worst-case row count and
//! is lowered by whatever the provider measured: first by the stratum's
//! cardinality (there are only that many rows), then by the selectivity of the
//! request terms that reach it (only that fraction of them can match). Every
//! step is a `min`, so a statistic can only ever narrow a depth the registry
//! already declared — a provider cannot license reading deeper than a producer
//! promised to answer.
//!
//! The selectivity step is deliberately conservative in two ways, because a
//! depth that falls below the rows a stratum really holds truncates the ranked
//! list silently. Contributions are **summed** across the terms reaching one
//! stratum and saturated at unity, so a producer that takes its terms as a
//! disjunction is bounded as loosely as one that conjoins them; and the product
//! is rounded **up**, so a ratio never bounds below the count it describes.
//! Nothing here refuses a plan: the whole step is a `min` over a bound the
//! registry already set, and a stratum no provider spoke about keeps exactly
//! the depth it had.
//!
//! And nothing here narrows a depth to nothing. A bound on the read never
//! becomes a value: an estimate may narrow a read, and a declaration may bound
//! it, but only the producer can report that there was nothing to read, through
//! a receipt fusion checks against the rows it pulled. So the derived depth is
//! floored at one — see [`capped`] — and a stratum planned at depth one is a
//! stratum whose relation is still invoked and still asked.
//!
//! That floor covers the registry's own zero as well as the provider's. A
//! producer whose every declared access mode promises zero rows per invocation
//! is an ordinary state, not a contradiction: a text index built before its
//! documents land, or one over a predicate no triple carries yet, declares
//! exactly that and still answers. Such a producer is placed like any other and
//! its stratum is planned at the floored depth of one, so the relation runs,
//! reads, and reports its own emptiness. Refusing it here would have thrown the
//! producer's receipt away and reported "no registered producer accepts any term
//! of the request" about a producer that accepts the term.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use purrdf_sparql_eval::{
    CandidateDomains, DuplicatePolicy, PfDescriptor, PropertyFunctionRegistry, RankedDeclaration,
};

use crate::error::PlanError;
use crate::iri::Iri;
use crate::matching::{carries_content, pattern_matches, place};
use crate::plan::{
    Plan, PlanOrigin, ProducerBinding, ProducerDecision, RejectionReason, StatisticsEntry,
    StatisticsSnapshot, UnservedReason, UnservedTerm,
};
use crate::request::{ReadBound, RequestTerm, RetrievalRequest};
use crate::statistics::Statistics;

/// Parts per million of unity: the value "every row matches" is reported as.
///
/// It is the scale [`Statistics::selectivity_ppm`] declares and the scale
/// [`StatisticsEntry::selectivity_ppm`] records, so a reported selectivity
/// passes through the planner without a change of unit.
const PPM_UNIT: u64 = 1_000_000;

/// Plan `request` against `registry`, consulting `statistics`.
///
/// The returned [`Plan`] records, as pure data: the request terms, one binding
/// per selected producer (the stratum and which request-term indices it
/// receives), every considered producer's decision — selected or rejected with
/// a reason — every request term that reached no producer at all with the reason
/// it did not, the request's own [`ReadBound`], the per-stratum depth derived
/// from the registry's row-bound declarations capped by statistics **and by that
/// bound**, the statistics snapshot actually consulted, and both registry
/// identities (the ephemeral instance id and the durable content fingerprint).
///
/// When the bound narrows a depth is decided from the producers' own
/// candidate-domain declarations and from nothing else; the rule, and the proof
/// that the narrowed depth is exact rather than merely smaller, are in this
/// module's header.
///
/// It records no stratum weights. Planning happens before a fusion profile is
/// chosen — the profile is deliberately not a planning input — and the weights
/// that fuse an answer are that profile's, so there is no second weighting for
/// a plan to carry.
///
/// # Errors
///
/// * [`PlanError::InvalidRequestTerm`] when a term is malformed (empty text or
///   geometry, an empty embedding, a non-finite embedding value, an empty seed).
/// * [`PlanError::RegistryDeclaration`] when a registered relation's declaration
///   panics under the seam's containment.
/// * [`PlanError::NoApplicableProducers`] when no registered producer accepts any
///   term of the request, or when every producer that does accept one cannot be
///   invoked for it.
/// * [`PlanError::StatisticsUnavailable`] when a selected producer declares an
///   unbounded row count, statistics supply no cardinality to bound it, and the
///   request's own bound licenses no prefix either.
pub fn plan(
    request: &RetrievalRequest,
    registry: &PropertyFunctionRegistry,
    statistics: &impl Statistics,
) -> Result<Plan, PlanError> {
    // 1. A malformed term cannot be planned, and no later stage could repair
    //    it: refuse at the boundary, naming the term.
    for term in &request.terms {
        validate_term(term)?;
    }

    // 2. Read the registry's own declarations once. `describe` is IRI-sorted, so
    //    every derived vector below is a pure function of the registry's
    //    contents, not of registration order.
    let descriptors = registry
        .describe()
        .map_err(|error| PlanError::RegistryDeclaration {
            message: error.to_string(),
        })?;
    let content_fingerprint =
        registry
            .content_fingerprint()
            .map_err(|error| PlanError::RegistryDeclaration {
                message: error.to_string(),
            })?;

    // 3. Match request terms to producers by declared capability only. This pass
    //    settles term matching and nothing else; a producer that matches here is
    //    still only a candidate, because whether its declaration can actually
    //    *render* those terms into its argument positions is decided in step 5.
    let mut candidates: Vec<Candidate<'_>> = Vec::new();
    for descriptor in &descriptors {
        let producer = descriptor.iri.clone();
        // A relation registered without a ranked declaration declares nothing,
        // and nothing is what the planner reads back: it does not fuse.
        let Some(declaration) = descriptor.ranked.as_ref() else {
            candidates.push(Candidate::rejected(producer, RejectionReason::NotRanked));
            continue;
        };
        // The seam declares its stratum with the kernel IRI; a plan carries the
        // layer's validated, hashable, orderable wrapper.
        let stratum = Iri::from(declaration.stratum.clone());

        let accepted: Vec<u32> = request
            .terms
            .iter()
            .enumerate()
            .filter(|(_, term)| {
                declaration
                    .accepted_terms
                    .iter()
                    .any(|accepted| pattern_matches(&accepted.pattern, term))
            })
            .map(|(index, _)| u32::try_from(index).unwrap_or(u32::MAX))
            .collect();

        if accepted.is_empty() {
            candidates.push(Candidate::rejected(
                producer,
                RejectionReason::NoAcceptedTerm,
            ));
            continue;
        }

        // Of the terms it accepts, the ones it also declared somewhere to put.
        // Only these are bound: see this module's header.
        let carried: Vec<u32> = accepted
            .iter()
            .copied()
            .filter(|index| {
                request
                    .terms
                    .get(*index as usize)
                    .is_some_and(|term| carries_content(declaration, term))
            })
            .collect();

        candidates.push(Candidate {
            producer,
            outcome: Outcome::Matched {
                descriptor,
                declaration,
                stratum,
                accepted,
                carried,
            },
        });
    }

    // 4. The depth every candidate would be invoked at. `place` renders the
    //    per-stratum depth into an argument for a producer that declares a depth
    //    placement, so the bound has to exist before placement runs — over the
    //    term-matched set, which is the widest set placement can survive from.
    let provisional = depth_bounds(&candidates, &request.terms, statistics);

    // 5. Placement: can this producer's declaration actually render the terms it
    //    matched into its own argument positions, under a mode it declares? A
    //    producer that cannot is rejected here rather than emitted as a call that
    //    silently drops the facet it could not write.
    let mut bindings: Vec<ProducerBinding> = Vec::new();
    let mut decisions: Vec<ProducerDecision> = Vec::with_capacity(candidates.len());
    let mut selected: Vec<(Iri, u64)> = Vec::new();
    // What each surviving stratum's one producer declared about which blocks of
    // the candidate universe it may name, and whether its stream may name one
    // candidate twice. Both, because the merge argument the request's own bound is
    // derived under runs on both: the blocks make a candidate's naming stratum
    // unique, and the duplicate policy is what makes a count of ranks a count of
    // candidates. Collected here, over the set that actually survives placement,
    // because a rejected producer's declaration is not part of that premise.
    let mut surviving_declarations: BTreeMap<Iri, (&CandidateDomains, DuplicatePolicy)> =
        BTreeMap::new();
    for candidate in &candidates {
        let (descriptor, declaration, stratum, carried) = match &candidate.outcome {
            Outcome::Rejected(reason) => {
                decisions.push(ProducerDecision::Rejected {
                    producer: candidate.producer.clone(),
                    reason: *reason,
                });
                continue;
            }
            Outcome::Matched {
                descriptor,
                declaration,
                stratum,
                carried,
                ..
            } => (*descriptor, *declaration, stratum, carried),
        };
        // A producer whose every declared mode promises zero rows is *not*
        // filtered here. Its declaration says its data is empty right now, which
        // is an ordinary state and not a contradiction, and the honest answer to
        // it is to invoke the producer at the floored depth of one and let it
        // report `Exhausted { rows_emitted: 0 }` — a completeness claim it earned
        // by reading. Dropping it instead bound it nowhere, and where it was the
        // registry's only producer the plan then failed with
        // `NoApplicableProducers`, a message that denies the very acceptance the
        // matching pass had just recorded.
        let depth = provisional.get(stratum).copied().unwrap_or(u32::MAX);
        if place(
            &candidate.producer,
            descriptor,
            declaration,
            &request.terms,
            carried,
            depth,
        )
        .is_err()
        {
            decisions.push(ProducerDecision::Rejected {
                producer: candidate.producer.clone(),
                reason: RejectionReason::UnsatisfiedConstraint,
            });
            continue;
        }
        decisions.push(ProducerDecision::Selected {
            producer: candidate.producer.clone(),
            stratum: stratum.clone(),
        });
        bindings.push(ProducerBinding {
            producer: candidate.producer.clone(),
            stratum: stratum.clone(),
            request_terms: carried.clone(),
        });
        selected.push((stratum.clone(), declared_row_bound(descriptor)));
        surviving_declarations.insert(
            stratum.clone(),
            (&declaration.domains, declaration.duplicates),
        );
    }

    if bindings.is_empty() {
        return Err(PlanError::NoApplicableProducers);
    }

    // 5b. Per-TERM evidence, which the per-producer decisions above cannot
    //     carry. A request term the bindings do not reach went unserved, and the
    //     two ways that happens are different facts about the registry: nothing
    //     declared a shape that accepts it at all, or something did and every
    //     acceptor was then rejected. Recorded for every such term, ascending,
    //     so an armed-but-unserved modality is visible as exactly that rather
    //     than as a term that quietly fell out of the plan.
    let unserved_terms = unserved_terms(&request.terms, &candidates, &bindings);

    // 6. Per-stratum depth over the SURVIVING set: a producer dropped in step 5
    //    can lower its stratum's worst-case bound, and recording the wider bound
    //    would license a depth no remaining producer can fill. The bound is the
    //    declared row count, capped by a measured cardinality when statistics
    //    offer one; an unbounded declaration with no statistic to bound it has no
    //    finite depth to record.
    // One entry per stratum, never a worst case across several: the registry
    // refuses a stratum a second producer declares, so each surviving stratum
    // was placed by exactly one producer and the bound is that producer's.
    let mut declared_bounds: BTreeMap<Iri, u64> = BTreeMap::new();
    for (stratum, bound) in selected {
        declared_bounds.insert(stratum, bound);
    }
    // Which request terms actually reach each surviving stratum. A selectivity
    // is a statement about the rows a *term* matches, so only the terms a
    // stratum's producers were bound to can bound that stratum's depth.
    let mut reaching: BTreeMap<Iri, BTreeSet<u32>> = BTreeMap::new();
    for binding in &bindings {
        reaching
            .entry(binding.stratum.clone())
            .or_default()
            .extend(binding.request_terms.iter().copied());
    }
    // What the request's own bound licenses, over exactly the surviving strata's
    // declarations: `Some(k)` when the merge argument in this module's header
    // holds, `None` when it does not and the registry's own bound stands. Decided
    // once, from the shape of the declarations, with no caller hint in it.
    let prefix = licensed_prefix(request.bound, &surviving_declarations);
    let strata: BTreeSet<Iri> = declared_bounds.keys().cloned().collect();
    let mut stratum_depths: HashMap<Iri, u32> = HashMap::with_capacity(strata.len());
    for stratum in &strata {
        let declared = declared_bounds.get(stratum).copied().unwrap_or(0);
        let reached = terms_at(&request.terms, reaching.get(stratum));
        let bound = capped(declared, stratum, &reached, statistics, prefix);
        if bound == u64::MAX {
            return Err(PlanError::StatisticsUnavailable {
                predicate: Box::new(stratum.clone()),
            });
        }
        stratum_depths.insert(stratum.clone(), u32::try_from(bound).unwrap_or(u32::MAX));
    }

    // 5. Capture the statistics the planner actually consulted: the strata it
    //    placed and the predicates the request named.
    let statistics_snapshot = capture_statistics(request, &strata, &reaching, statistics);

    // 6. Record both registry identities.
    Ok(Plan {
        version: Plan::VERSION,
        request_terms: request.terms.clone(),
        read_bound: request.bound,
        producer_bindings: bindings,
        producer_decisions: decisions,
        unserved_terms,
        stratum_depths,
        statistics_snapshot,
        registry_instance_id: registry.instance_id(),
        registry_content_fingerprint: content_fingerprint,
        // Planned here, against a registry this process holds, so the instance
        // id above names something admission can still compare against.
        origin: PlanOrigin::SameProcess,
    })
}

/// One considered producer, between term matching and placement.
///
/// Planning is two passes over the registry's descriptions rather than one,
/// because placement needs a depth and the depth needs the set placement
/// survives. This value is what the first pass hands the second; it borrows the
/// registry's own description rather than copying any of it.
struct Candidate<'a> {
    /// The registered producer IRI, byte-exact.
    producer: String,
    /// What the first pass decided.
    outcome: Outcome<'a>,
}

impl Candidate<'_> {
    /// A producer the first pass already refused.
    const fn rejected(producer: String, reason: RejectionReason) -> Self {
        Self {
            producer,
            outcome: Outcome::Rejected(reason),
        }
    }
}

/// The first pass's decision for one producer.
enum Outcome<'a> {
    /// Refused on term matching alone; placement is never consulted.
    Rejected(RejectionReason),
    /// At least one request term matched a declared shape.
    Matched {
        /// The registry's own description of the relation.
        descriptor: &'a PfDescriptor,
        /// The ranked declaration supplied where it was registered.
        declaration: &'a RankedDeclaration,
        /// The stratum it ranks within.
        stratum: Iri,
        /// The request-term indices whose shape it accepts, ascending. This is
        /// the set a mandatory declaration's *presence* answers for.
        accepted: Vec<u32>,
        /// The subset of [`Self::Matched::accepted`] the matching alternative
        /// also declares a placement for, ascending — the terms this producer
        /// actually receives, and therefore the ones it is bound to.
        carried: Vec<u32>,
    },
}

/// The depth each stratum would carry over the term-matched candidates.
///
/// This is the provisional bound placement is run against, not the bound the
/// plan records: it is computed over the widest set (every producer whose terms
/// matched), so a producer can only ever be handed a depth at least as large as
/// the one its stratum finally records. An unbounded stratum with no statistic
/// has no finite depth here; it is carried as [`u32::MAX`] rather than refused,
/// because the refusal belongs to the surviving set and is raised there.
///
/// The terms read are the **carried** ones, matching what `plan` finally
/// records. Reading the accepted set instead would let a stratum's provisional
/// depth be narrowed by a selectivity its final depth is not, which is the one
/// direction the invariant above forbids.
///
/// The request's own bound ([`ReadBound`]) is deliberately **not** applied here,
/// for that same invariant. This depth exists only so [`place`] can be run, and
/// the one property it owes is that no producer is ever handed a depth smaller
/// than the one its stratum finally records; the request's bound only ever lowers
/// a depth, so applying it on this side could only push against that direction
/// while buying nothing — the number never reaches a plan, a unit or a row.
///
/// Every finite depth here carries [`capped`]'s floor of one, which matters
/// more at this step than at the final one: this is the depth [`place`] renders
/// into a producer's declared depth argument, so a zero would hand a relation a
/// literal instruction to return nothing before anything had read a row. The
/// floor applies to a registry bound of zero exactly as it does to a measured
/// zero, and for the same reason — the one row it asks for is the probe that lets
/// the producer say, in its own receipt, that it has none.
fn depth_bounds(
    candidates: &[Candidate<'_>],
    terms: &[RequestTerm],
    statistics: &impl Statistics,
) -> BTreeMap<Iri, u32> {
    let mut bounds: BTreeMap<Iri, u64> = BTreeMap::new();
    let mut reaching: BTreeMap<Iri, BTreeSet<u32>> = BTreeMap::new();
    for candidate in candidates {
        let Outcome::Matched {
            descriptor,
            stratum,
            carried,
            ..
        } = &candidate.outcome
        else {
            continue;
        };
        // One producer per stratum, so this key is fresh: see `plan`'s own
        // `declared_bounds` for why there is no worst case to take here.
        bounds.insert(stratum.clone(), declared_row_bound(descriptor));
        reaching
            .entry(stratum.clone())
            .or_default()
            .extend(carried.iter().copied());
    }
    bounds
        .into_iter()
        .map(|(stratum, declared)| {
            let reached = terms_at(terms, reaching.get(&stratum));
            let bound = capped(declared, &stratum, &reached, statistics, None);
            let depth = u32::try_from(bound).unwrap_or(u32::MAX);
            (stratum, depth)
        })
        .collect()
}

/// The request terms a recorded index set names, in ascending index order.
///
/// An index the request does not carry is skipped rather than refused: this is
/// a lookup used to narrow a bound, and a bound is narrowed by the terms that
/// exist. A plan whose recorded indices address no term of its own request is a
/// separate, typed refusal at the admission waist, where untrusted plans are
/// checked.
fn terms_at<'a>(terms: &'a [RequestTerm], indices: Option<&BTreeSet<u32>>) -> Vec<&'a RequestTerm> {
    indices.map_or_else(Vec::new, |indices| {
        indices
            .iter()
            .filter_map(|index| terms.get(*index as usize))
            .collect()
    })
}

/// Every request term no binding carries, ascending, with the reason it went
/// unserved.
///
/// The three reasons are read off the passes that produced them and are not
/// interchangeable:
///
/// * a term no [`Outcome::Matched`] candidate accepts was never accepted by any
///   declaration in the registry — nothing was a candidate for it, which is
///   [`UnservedReason::NoProducerAccepts`];
/// * a term some candidate would have **carried** — its matching alternative
///   declares a placement for it — that nonetheless reaches no binding was left
///   with nothing to answer it when every such acceptor was rejected at
///   placement, which is [`UnservedReason::EveryAcceptingProducerRejected`] and
///   sends the reader to that producer's own recorded dimension;
/// * a term every accepting candidate accepts *without* a placement reaches a
///   producer that was never going to write it down, which is
///   [`UnservedReason::AcceptedWithoutPlacement`].
///
/// The order of the last two is the order of what a caller can act on. "Someone
/// could have carried this and was rejected" names a producer whose refusal is
/// recorded and fixable; "everyone who takes this shape declares nowhere to put
/// it" names a registry with no such producer at all. A term that is both gets
/// the first, because the first is the narrower fact.
fn unserved_terms(
    terms: &[RequestTerm],
    candidates: &[Candidate<'_>],
    bindings: &[ProducerBinding],
) -> Vec<UnservedTerm> {
    let mut accepted = vec![false; terms.len()];
    let mut carried = vec![false; terms.len()];
    for candidate in candidates {
        let Outcome::Matched {
            accepted: candidate_accepted,
            carried: candidate_carried,
            ..
        } = &candidate.outcome
        else {
            continue;
        };
        for (indices, flags) in [
            (candidate_accepted, &mut accepted),
            (candidate_carried, &mut carried),
        ] {
            for index in indices {
                if let Some(slot) = flags.get_mut(*index as usize) {
                    *slot = true;
                }
            }
        }
    }
    let mut served = vec![false; terms.len()];
    for binding in bindings {
        for index in &binding.request_terms {
            if let Some(slot) = served.get_mut(*index as usize) {
                *slot = true;
            }
        }
    }
    served
        .iter()
        .enumerate()
        .filter(|(_, served)| !**served)
        .map(|(index, _)| UnservedTerm {
            request_term: u32::try_from(index).unwrap_or(u32::MAX),
            reason: if carried[index] {
                UnservedReason::EveryAcceptingProducerRejected
            } else if accepted[index] {
                UnservedReason::AcceptedWithoutPlacement
            } else {
                UnservedReason::NoProducerAccepts
            },
        })
        .collect()
}

/// The per-stratum read prefix `bound` licenses over strata declaring
/// `declarations`, or `None` when it licenses none.
///
/// This is the whole of the decision described in this module's header, and it is
/// shape-driven: the answer is a function of the request's bound and of the
/// surviving producers' own declarations. There is no caller hint, no mode and no
/// heuristic.
///
/// `Some(k)` requires **all three** of the conditions the proof's premises rest
/// on:
///
/// * every surviving stratum declares a block set — an `Unrestricted` stratum
///   promises nothing about which candidates it will not name, so it supplies no
///   premise at all;
/// * no two of those sets meet, which is what makes each candidate's naming
///   stratum unique and its fused score a single term rather than a sum;
/// * every one of those strata declares [`DuplicatePolicy::Unique`], which is what
///   makes its depth-`k` prefix `k` *candidates* rather than `k` rows. A repeat
///   under [`DuplicatePolicy::Allowed`] is validated, charged and then discarded
///   by [`FusionStream`](crate::FusionStream), so such a prefix can hold fewer
///   candidates than rows and is then not a superset of the top `k`.
///
/// [`CandidateDomains::intersects`] decides the second, and it is the right
/// question rather than a convenient one: it is true exactly when some candidate
/// could satisfy both declarations, which is exactly the case the merge argument
/// cannot survive.
///
/// The pairwise scan is quadratic in the number of strata, which is the number of
/// ranked producers one request reaches — a handful, fixed by the registry rather
/// than by the corpus. Nothing here touches a row.
fn licensed_prefix(
    bound: ReadBound,
    declarations: &BTreeMap<Iri, (&CandidateDomains, DuplicatePolicy)>,
) -> Option<u64> {
    let ReadBound::Bounded(top_k) = bound else {
        return None;
    };
    // The duplicate policies are read first and in one pass, and a single
    // `Allowed` stratum ends the question before any block set is compared: that
    // stream's rows are not its candidates, so no arrangement of the declared
    // blocks can make a depth of `k` hold `k` candidates.
    let declared: Vec<&CandidateDomains> = declarations
        .values()
        .map(|(domains, duplicates)| match duplicates {
            DuplicatePolicy::Unique => Some(*domains),
            DuplicatePolicy::Allowed => None,
        })
        .collect::<Option<Vec<&CandidateDomains>>>()?;
    if declared
        .iter()
        .any(|entry| matches!(entry, CandidateDomains::Unrestricted))
    {
        return None;
    }
    for (index, left) in declared.iter().enumerate() {
        for right in &declared[index + 1..] {
            if left.intersects(right) {
                return None;
            }
        }
    }
    Some(top_k.get() as u64)
}

/// A declared row bound, lowered (never raised) by what the provider measured,
/// and never lowered past the first row — nor read as zero when the declaration
/// itself was zero.
///
/// Two independent statistics narrow one number. A measured cardinality says
/// how many rows the stratum holds at all; a measured selectivity says what
/// fraction of them `terms` can match, and a term matching a tenth of a
/// stratum cannot be read a stratum-deep. Each is applied as a `min`, so the
/// result never exceeds the registry's own declaration and a provider that
/// measured nothing changes nothing.
///
/// # The finite bound is floored at one
///
/// A bound narrows a read; it never eliminates one. Emptiness is the
/// **producer's** to report, through a receipt fusion verifies against the rows
/// it actually pulled — so nothing upstream of the producer may be the thing
/// that decides a stratum is empty. A bound of zero would be exactly that: it
/// compiles to `LIMIT 0`, which hands back no row whatever the relation holds,
/// and the stratum is then reported exhausted with no rows — the strongest
/// completeness claim this layer has, minted from a number nobody checked against
/// the data and indistinguishable afterwards from an honest empty answer.
///
/// A tiny non-zero selectivity already lands on one row through `div_ceil`;
/// zero was the one input that escaped that protection, and it arrives by three
/// roads — a provider honestly reporting `selectivity_ppm` of zero, a measured
/// cardinality of zero, which lands in the bound before the ratio is applied,
/// and `declared` itself, when the registry's every access mode promises zero
/// rows per invocation. All three are floored here, so all three narrow the read
/// to a single probing row and leave the verdict to the producer.
///
/// The floor is unconditional, and the one row it can add past `declared` is the
/// point rather than an overreach. A producer declaring zero rows has described
/// its data, not forbidden its own invocation — an index built before its
/// documents land declares exactly that, and it answers a query against it
/// perfectly well by returning nothing. Reading one row from it asks the question
/// its declaration cannot answer on the registry's behalf: is that still true?
/// The producer answers with [`crate::ProducerStatus::Exhausted`] and a row count of
/// zero, which is a true completeness claim, because it really did read. So
/// [`crate::admission`] admits a depth of one against a declared zero, and the
/// compiler's `emitted_limit` never writes a `LIMIT 0`; the floor and those two
/// are one rule in three places.
///
/// A producer that declares no access mode at all is a different fact and is
/// still refused: it admits no invocation, so placement cannot render one, and it
/// never reaches this function.
///
/// The genuinely unbounded case returns before the floor and keeps returning
/// [`u64::MAX`]: the caller turns that into
/// [`PlanError::StatisticsUnavailable`], and flooring it would be flooring an
/// absent bound rather than a derived one.
fn capped(
    declared: u64,
    stratum: &Iri,
    terms: &[&RequestTerm],
    statistics: &impl Statistics,
    prefix: Option<u64>,
) -> u64 {
    let bound = match statistics.cardinality(stratum) {
        Some(cardinality) => declared.min(cardinality),
        None => declared,
    };
    // An unbounded stratum has no row count for a ratio to be a fraction of.
    // Scaling `u64::MAX` would manufacture a finite bound out of a missing one
    // and hide the condition `StatisticsUnavailable` exists to report.
    //
    // The request's own bound is the exception, and it is not a manufactured
    // bound: a stratum whose declaration promises unboundedly many rows, read for
    // an answer that provably cannot use more than `k` of them, is a read of `k`
    // rows and not an unbounded read. Reporting `StatisticsUnavailable` there
    // would refuse a plan whose depth is exactly known — the over-refusal mirror
    // of the silent truncation the rest of this function guards against.
    if bound == u64::MAX {
        return prefix.map_or(u64::MAX, |rows| rows.max(1));
    }
    let narrowed = match combined_selectivity_ppm(stratum, terms, statistics) {
        // Rounded up, in an intermediate wide enough that the product cannot
        // wrap: a bound derived from a ratio must never fall below the rows the
        // ratio describes, because a depth below a stratum's real answer
        // truncates its ranked list with nothing anywhere saying so.
        Some(ppm) => {
            let scaled = (u128::from(bound) * u128::from(ppm)).div_ceil(u128::from(PPM_UNIT));
            u64::try_from(scaled).unwrap_or(bound).min(bound)
        }
        None => bound,
    };
    // The request's bound is applied last, and the order is load-bearing. A
    // selectivity is a fraction of the rows the *stratum* holds, so scaling a
    // bound that has already been narrowed to `k` would ask for a fraction of `k`
    // — a depth below the rows the ratio describes, which is the silent truncation
    // this whole function is arranged to avoid. Narrowed here it is one more `min`
    // over a bound both the registry and the provider already set.
    let bounded = match prefix {
        Some(rows) => narrowed.min(rows),
        None => narrowed,
    };
    bounded.max(1)
}

/// The selectivity the provider reports for `subject` across `terms`, in parts
/// per million, or `None` when it reports none for any of them.
///
/// The terms are **summed** rather than minimised, and the sum saturates at
/// unity. Minimising would assume every producer conjoins the terms it is
/// handed; one that answers their union can return more rows than the most
/// selective term alone describes, and a bound below that is a silently
/// truncated ranked list. The sum bounds both readings, and a value a provider
/// reports above unity — a claim that a term matches more rows than exist — is
/// clamped there rather than allowed to widen anything.
///
/// No term is filtered out by the predicate it names. The provider decides
/// which `(subject, term)` pairs it can answer, and pre-filtering would make a
/// selectivity reported under a *stratum* unreachable — which is exactly the
/// one a stratum's depth is derived from. Asking about every term keeps the
/// lookup a lookup.
fn combined_selectivity_ppm(
    subject: &Iri,
    terms: &[&RequestTerm],
    statistics: &impl Statistics,
) -> Option<u64> {
    let mut total: Option<u64> = None;
    for term in terms {
        let Some(ppm) = statistics.selectivity_ppm(subject, term) else {
            continue;
        };
        total = Some(
            total
                .unwrap_or(0)
                .saturating_add(ppm.min(PPM_UNIT))
                .min(PPM_UNIT),
        );
    }
    total
}

/// Refuse a request term that cannot name anything.
///
/// The two interval arms are refused on the two grounds an interval can be
/// unusable: it constrains nothing (neither endpoint), or it can match nothing
/// (a lower endpoint above its upper). The second is checked for a numeric range
/// and not for a temporal one, and the asymmetry is the point rather than an
/// omission: a numeric endpoint is an exact [`Fixed`] this layer can order, while
/// a temporal endpoint is the caller's own lexical form, which this layer does
/// not parse and therefore cannot order — comparing two calendar lexicals as
/// strings would refuse legitimate intervals whose encoding is not
/// lexicographically ordered. A degenerate range whose endpoints are equal is a
/// single point and is admitted.
fn validate_term(term: &RequestTerm) -> Result<(), PlanError> {
    let reason = match term {
        RequestTerm::Lexical { text, .. } if text.trim().is_empty() => "lexical text is empty",
        RequestTerm::Vector { embedding, .. } if embedding.is_empty() => {
            "vector embedding is empty"
        }
        RequestTerm::Vector { embedding, .. }
            if embedding.iter().any(|value| !value.is_finite()) =>
        {
            "vector embedding contains a non-finite value"
        }
        RequestTerm::Spatial { geometry, .. } if geometry.trim().is_empty() => {
            "spatial geometry is empty"
        }
        RequestTerm::EntitySeed { entity } if entity.as_str().trim().is_empty() => {
            "entity seed is empty"
        }
        RequestTerm::Temporal {
            lower: None,
            upper: None,
            ..
        } => "temporal interval carries neither endpoint",
        RequestTerm::Temporal { lower, upper, .. }
            if lower
                .iter()
                .chain(upper.iter())
                .any(|end| end.trim().is_empty()) =>
        {
            "temporal interval endpoint is empty"
        }
        RequestTerm::NumericRange {
            lower: None,
            upper: None,
            ..
        } => "numeric range carries neither endpoint",
        RequestTerm::NumericRange {
            lower: Some(lower),
            upper: Some(upper),
            ..
        } if lower > upper => "numeric range lower endpoint exceeds its upper endpoint",
        _ => return Ok(()),
    };
    Err(PlanError::InvalidRequestTerm {
        term: Box::new(term.clone()),
        reason: reason.to_owned(),
    })
}

/// A producer's worst-case declared row count across its access modes.
///
/// This is the registry's own cost declaration (`rows_per_invocation` per
/// declared mode), read from the descriptor — never a parallel field.
fn declared_row_bound(descriptor: &PfDescriptor) -> u64 {
    descriptor
        .modes
        .iter()
        .map(|mode| mode.rows_per_invocation)
        .max()
        .unwrap_or(0)
}

/// The predicate a request term names, when it names one.
fn term_predicate(term: &RequestTerm) -> Option<&Iri> {
    match term {
        RequestTerm::Lexical { predicate, .. } => predicate.as_ref(),
        RequestTerm::Spatial { predicate, .. }
        | RequestTerm::Temporal { predicate, .. }
        | RequestTerm::NumericRange { predicate, .. } => Some(predicate),
        RequestTerm::Vector { .. } | RequestTerm::EntitySeed { .. } => None,
    }
}

/// Record the statistics the planner consulted, in a deterministic order.
///
/// Only facts the provider actually reports are recorded: a subject with no
/// cardinality is omitted rather than recorded as zero, because zero is a
/// measurement and absence is not.
///
/// The recorded selectivity is the same aggregate the depth was derived from,
/// over the same terms: for a stratum, the terms `reaching` says were bound to
/// it; for a request predicate, which no depth is derived for, the whole
/// request. So the snapshot explains the depth beside it rather than reporting
/// a second, differently-computed number that happens to sit next to it.
fn capture_statistics(
    request: &RetrievalRequest,
    strata: &BTreeSet<Iri>,
    reaching: &BTreeMap<Iri, BTreeSet<u32>>,
    statistics: &impl Statistics,
) -> StatisticsSnapshot {
    let mut subjects: BTreeSet<Iri> = strata.clone();
    for term in &request.terms {
        if let Some(predicate) = term_predicate(term) {
            subjects.insert(predicate.clone());
        }
    }
    let all_terms: Vec<&RequestTerm> = request.terms.iter().collect();

    let mut entries = Vec::new();
    for subject in subjects {
        let Some(cardinality) = statistics.cardinality(&subject) else {
            continue;
        };
        let consulted = match reaching.get(&subject) {
            Some(indices) => terms_at(&request.terms, Some(indices)),
            None => all_terms.clone(),
        };
        entries.push(StatisticsEntry {
            subject: subject.as_str().to_owned(),
            cardinality,
            selectivity_ppm: combined_selectivity_ppm(&subject, &consulted, statistics),
        });
    }

    StatisticsSnapshot {
        source: statistics.source().to_owned(),
        revision: statistics.revision().to_owned(),
        entries,
    }
}

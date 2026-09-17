// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The compiler: admitted plan in, independently executable SPARQL out.
//!
//! [`compile`] is the narrow admission waist. It runs every semantic check in
//! [`admission`](crate::admission) and, when the plan is admitted, emits one
//! SPARQL `SELECT` per stratum. Each unit names its stratum's producers by their
//! registered IRIs and is executable through `purrdf-sparql-eval` with no
//! composition layer in the path — the text is the whole executable content, so
//! nothing can live between planning and execution.
//!
//! # Why text, not an algebra node
//!
//! A caller can take an emitted unit and hand it to the evaluator directly; the
//! conformance corpus already insists the evaluator is the single source of query
//! truth, and a text seam lets a caller stand on that same surface without
//! adopting this crate's types. An AST carried between `compile` and `execute`
//! would create a second, non-textual artifact in exactly the gap the design
//! closes.
//!
//! # What a unit contains
//!
//! For each stratum the plan declares, the unit is a `SELECT ?candidate` over the
//! one branch of that stratum's one producer. The branch is a sub-`SELECT` that
//! projects the producer's own declared candidate position under the common name
//! `?candidate`, so strata whose producers name their candidate in different
//! argument positions still read back through one column.
//!
//! # One stratum, one branch — there is no union to emit
//!
//! A stratum carries exactly one producer, refused at registration by
//! [`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
//! and again at this waist for an edited plan. So there is never a second branch
//! to compose with, and the `UNION` this emitter once wrote could only ever have
//! run against a configuration the registry now refuses to build.
//!
//! It was also never a *merge*. A `UNION` concatenates: the second producer's
//! rank-1 row surfaced at stratum rank `n+1`, below the whole of the first
//! producer's output, and decayed as though it had lost to rows it never competed
//! with; a candidate both producers named arrived twice in one stratum's stream,
//! which an honest `Unique` declaration says cannot happen; and a first producer
//! that filled the depth left the second contributing nothing while the trailer
//! reported a clean exhaustion. Ranks are comparable only within the list that
//! assigned them, which is exactly why the merge belongs either inside one
//! producer (same scoring law, no weight between the two) or across strata in the
//! fusion sum (different scoring laws — or one law whose bounded score cannot
//! carry the weight the host means between two classes), and never in a
//! concatenation here.
//!
//! # The request is in the text
//!
//! Every argument position a producer's declaration places a request facet into
//! carries that facet as a **constant**; only the positions nothing was placed
//! into are free `?cN` variables. The placement rule is
//! [`matching::place`](crate::matching::place) — the same function the planner
//! ran, so admission re-derives the planner's decision rather than a second
//! approximation of it.
//!
//! So `search("quick brown fox")` and `search("")` compile to different queries
//! wherever a producer declared a placement for the needle, which is the whole
//! point of compiling a request rather than an arity. Where a producer declared
//! **none** — a declaration that accepts the shape and writes no part of it —
//! the two compile to the same query, because that producer asked to be called
//! with the request absent from its arguments. That is not a silent loss: such a
//! term is never bound to the producer, and the plan names it in
//! [`Plan::unserved_terms`] as
//! [`UnservedReason::AcceptedWithoutPlacement`](crate::UnservedReason::AcceptedWithoutPlacement).
//! An empty unserved list therefore does mean the text carries every term.
//!
//! # The subject argument list is always parenthesized
//!
//! Even at subject arity one, the subject side is written `( ?c0 )`. A bare
//! subject would make the call's parse depend on the grammar tolerating whatever
//! term landed there (a literal subject, say), whereas a parenthesized group
//! followed by a registered property-function IRI is routed unambiguously to the
//! argument-list production.
//!
//! # The branch carries the stratum's depth, and so does the unit
//!
//! A branch whose producer takes no depth argument is bounded by its own
//! `LIMIT <depth>`, and the unit repeats that `LIMIT` on the outside. The inner
//! one is the producer's licence to stop early — the evaluator offers it to the
//! relation as a row ceiling — and the outer one is the stratum's contract with
//! fusion, which holds whatever bounds the branch below it carries. A producer
//! that declares a [`DepthPlacement`](purrdf_sparql_eval::DepthPlacement) already
//! received the depth as an argument and bounds itself, so its branch carries no
//! `LIMIT`; the outer one still applies.
//!
//! A stratum whose depth is zero emits `LIMIT 0`. That is an honest empty
//! stratum — the statistics said there is nothing to rank — and not a failure.

use std::collections::BTreeMap;

use purrdf_sparql_eval::{PfDescriptor, RegistryId};

use crate::admission::{AdmissionEnvironment, AdmissionError, admit_plan};
use crate::id::PlanId;
use crate::iri::Iri;
use crate::matching::{PlacementError, place, render_slots};
use crate::plan::{Plan, ProducerBinding};
use crate::render::RenderError;

/// One stratum's independently executable query text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StratumUnit {
    /// The caller-supplied stratum the unit ranks within.
    pub stratum: Iri,
    /// A self-contained SPARQL `SELECT` over the stratum's registered relations.
    pub sparql: String,
}

/// The admitted, compiled plan: the query units plus the identities that pin them.
///
/// `plan_id` is the plan's canonical identity, so a stream or answer can name the
/// pinned plan it descends from; `registry_id` and `registry_fingerprint` are the
/// registry the units were compiled against, so [`execute`](crate::execute) can
/// refuse to run the same text against a different registry and silently obtain a
/// different meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRetrieval {
    /// Per-stratum query units, ordered by stratum IRI.
    pub units: Vec<StratumUnit>,
    /// The canonical identity of the admitted plan.
    pub plan_id: PlanId,
    /// The live registry instance the units were compiled against.
    pub registry_id: RegistryId,
    /// The durable content fingerprint of the registry the units were compiled
    /// against.
    pub registry_fingerprint: String,
}

/// The name of the variable every branch projects its candidate under, without
/// the `?` sigil.
///
/// Spelled once, because [`execute`](crate::execute) reads the emitted unit's
/// solutions back by exactly this name: two constants that could drift would let
/// the executor look for a column the compiler stopped writing.
pub(crate) const CANDIDATE_NAME: &str = "candidate";

/// Admit `plan` against `env` and emit its per-stratum SPARQL units.
///
/// The admission checks are documented on [`AdmissionError`]; this function adds
/// only the emission. A plan that passes admission yields exactly one unit per
/// stratum the plan declares that also has at least one bound producer, ordered by
/// stratum IRI, each independently executable through `purrdf-sparql-eval`.
///
/// # Errors
///
/// The distinct [`AdmissionError`] variant of the first violated admission
/// dimension; see [`AdmissionError::dimension`].
pub fn compile(
    plan: &Plan,
    env: &AdmissionEnvironment<'_>,
) -> Result<CompiledRetrieval, AdmissionError> {
    let admitted = admit_plan(plan, env)?;

    let mut units = Vec::new();
    for (stratum, depth) in &plan.stratum_depths {
        let Some(binding) = admitted.stratum_bindings.get(stratum) else {
            // A stratum the plan gives a depth but no producer has no relation
            // to run, so there is nothing to emit for it. That is the only case
            // this arm can reach: admission refuses a binding whose stratum the
            // plan records no depth for, so iterating the depth keys cannot skip
            // a bound producer. Without that check this `continue` would be the
            // silent narrowing — a producer dropped from the emitted text with
            // nothing reporting it.
            continue;
        };
        let sparql = emit_unit(plan, binding, &admitted.descriptors, *depth)?;
        units.push(StratumUnit {
            stratum: stratum.clone(),
            sparql,
        });
    }
    units.sort_by(|left, right| left.stratum.cmp(&right.stratum));

    Ok(CompiledRetrieval {
        units,
        plan_id: plan.id(),
        registry_id: admitted.instance_id,
        registry_fingerprint: admitted.fingerprint,
    })
}

/// Render one stratum's `SELECT` over its one `binding` at `depth`.
fn emit_unit(
    plan: &Plan,
    binding: &ProducerBinding,
    descriptors: &BTreeMap<String, PfDescriptor>,
    depth: u32,
) -> Result<String, AdmissionError> {
    let branch = emit_branch(plan, binding, descriptors, depth)?;
    Ok(format!(
        "SELECT ?{CANDIDATE_NAME} WHERE {{\n  {branch}\n}}\nLIMIT {depth}"
    ))
}

/// Render the stratum's producer as the unit's one branch.
fn emit_branch(
    plan: &Plan,
    binding: &ProducerBinding,
    descriptors: &BTreeMap<String, PfDescriptor>,
    depth: u32,
) -> Result<String, AdmissionError> {
    let descriptor = descriptors
        .get(&binding.producer)
        .ok_or_else(|| malformed(binding, "has no registry declaration to compile against"))?;
    let declaration = descriptor
        .ranked
        .as_ref()
        .ok_or_else(|| malformed(binding, "declares no ranked capability to compile against"))?;
    let subject = descriptor.subject_arity;
    let total = subject + descriptor.object_arity;
    if total == 0 {
        return Err(malformed(
            binding,
            "declares zero arguments, so a call cannot name a candidate",
        ));
    }

    // The plan is untrusted input, so the placement the planner proved is proved
    // again here — by the same function, never by a second approximation.
    let invocation = place(
        &binding.producer,
        descriptor,
        declaration,
        &plan.request_terms,
        &binding.request_terms,
        depth,
    )
    .map_err(|error| unsatisfiable(binding, &error))?;
    let candidate = declaration.candidate_position;
    if candidate >= total || invocation.mode.is_bound(candidate) {
        // The registry validates that the candidate position exists and is never
        // a placement target, so this is a registry that moved under the plan.
        return Err(malformed(
            binding,
            "projects a candidate from a position that is not a free argument",
        ));
    }

    let args = render_slots(&invocation).map_err(|error| unrenderable(binding, &error))?;
    let subject_text = args[..subject].join(" ");
    let object_text = args[subject..].join(" ");
    // A producer that took the depth as an argument bounds itself; one that did
    // not is bounded here, per this module's header.
    let limit = if declaration.depth_placement.is_none() {
        format!(" LIMIT {depth}")
    } else {
        String::new()
    };
    Ok(format!(
        "{{ SELECT (?c{candidate} AS ?{CANDIDATE_NAME}) WHERE {{ ( {subject_text} ) <{}> ( {object_text} ) }}{limit} }}",
        binding.producer
    ))
}

/// A structural defect in the plan-plus-registry pair, named by producer.
fn malformed(binding: &ProducerBinding, what: &str) -> AdmissionError {
    AdmissionError::MalformedPlan {
        reason: format!("producer {} {what}", binding.producer),
    }
}

/// A producer that cannot be invoked for the terms the plan gives it.
fn unsatisfiable(binding: &ProducerBinding, error: &PlacementError) -> AdmissionError {
    match Iri::parse(&binding.producer) {
        Ok(producer) => AdmissionError::UnsatisfiablePlacement {
            producer: Box::new(producer),
            rule: error.rule(),
            detail: error.to_string(),
            invocation: error.invocation(),
            declared: error.declared(),
        },
        Err(invalid) => AdmissionError::MalformedPlan {
            reason: format!("plan binds invalid producer IRI {invalid}"),
        },
    }
}

/// A placed value that has no SPARQL constant form.
///
/// [`place`] proves every slot it fills renders, so reaching this means the
/// registry moved between the two calls; it is reported on the same dimension
/// because it is the same claim.
fn unrenderable(binding: &ProducerBinding, error: &RenderError) -> AdmissionError {
    match Iri::parse(&binding.producer) {
        Ok(producer) => AdmissionError::UnsatisfiablePlacement {
            producer: Box::new(producer),
            rule: "unrenderable",
            detail: error.to_string(),
            invocation: None,
            declared: Vec::new(),
        },
        Err(invalid) => AdmissionError::MalformedPlan {
            reason: format!("plan binds invalid producer IRI {invalid}"),
        },
    }
}

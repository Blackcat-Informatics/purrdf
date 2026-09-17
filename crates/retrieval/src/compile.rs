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
//! For each stratum the plan declares, the unit is a `SELECT` over a `UNION` of
//! one property-function call per producer bound to that stratum, in IRI-sorted
//! order so the text is a pure function of the plan. A call uses a distinct
//! variable for every flattened argument position (`?c0`, `?c1`, …) and projects
//! the first, which is the candidate a ranked row names. Every producer's
//! declared arity comes from the registry's own
//! [`describe`](purrdf_sparql_eval::PropertyFunctionRegistry::describe) output —
//! never a parallel declaration.

use std::collections::BTreeMap;

use purrdf_sparql_eval::{PfDescriptor, RegistryId};

use crate::admission::{AdmissionEnvironment, AdmissionError, admit_plan};
use crate::id::PlanId;
use crate::iri::Iri;
use crate::plan::{Plan, ProducerBinding};

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

    // Group the plan's bindings by stratum once, preserving producer order by IRI
    // so the emitted text does not depend on the plan's binding order.
    let mut by_stratum: BTreeMap<Iri, Vec<&ProducerBinding>> = BTreeMap::new();
    for binding in &plan.producer_bindings {
        by_stratum
            .entry(binding.stratum.clone())
            .or_default()
            .push(binding);
    }
    for bindings in by_stratum.values_mut() {
        bindings.sort_by(|left, right| left.producer.cmp(&right.producer));
    }

    let mut units = Vec::new();
    for stratum in plan.stratum_depths.keys() {
        let Some(bindings) = by_stratum.get(stratum) else {
            // A declared stratum with no bound producer has no relation to run;
            // admission has already refused anything structurally inconsistent,
            // so this is simply nothing to emit.
            continue;
        };
        let sparql = emit_unit(bindings, &admitted.descriptors)?;
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

/// Render one stratum's `SELECT` over `bindings`.
fn emit_unit(
    bindings: &[&ProducerBinding],
    descriptors: &BTreeMap<String, PfDescriptor>,
) -> Result<String, AdmissionError> {
    let mut branches = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let descriptor =
            descriptors
                .get(&binding.producer)
                .ok_or_else(|| AdmissionError::MalformedPlan {
                    reason: format!(
                        "producer {} has no registry declaration to compile against",
                        binding.producer
                    ),
                })?;
        let subject = descriptor.subject_arity;
        let object = descriptor.object_arity;
        let total = subject + object;
        if total == 0 {
            return Err(AdmissionError::MalformedPlan {
                reason: format!(
                    "producer {} declares zero arguments, so a call cannot name a candidate",
                    binding.producer
                ),
            });
        }
        let mut object_args: Vec<String> = Vec::with_capacity(object.max(1));
        for position in subject..total {
            object_args.push(format!("?c{position}"));
        }
        // A one-argument subject may be written bare; a zero-argument or
        // multi-argument subject is a parenthesized argument group.
        let subject_text = if subject == 1 {
            "?c0".to_owned()
        } else {
            let subject_args: Vec<String> = (0..subject).map(|p| format!("?c{p}")).collect();
            format!("({})", subject_args.join(" "))
        };
        branches.push(format!(
            "{{ {subject_text} <{}> ({}) }}",
            binding.producer,
            object_args.join(" ")
        ));
    }
    Ok(format!(
        "SELECT ?c0 WHERE {{\n  {}\n}}",
        branches.join("\n  UNION\n  ")
    ))
}

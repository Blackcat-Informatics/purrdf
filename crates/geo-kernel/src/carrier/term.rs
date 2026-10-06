// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Move an admitted carrier string into one admitted RDF literal.

use crate::{GeoError, MetricContext, MetricWorkObserver};
use purrdf_core::TermValue;
use purrdf_xsd::integer::ExactArithmeticCost;

/// Admit an original borrowed RDF term's complete owned heap capacities and text.
/// The root's inline term and the bounded walk are covered by the entry allowance.
/// Descendants use the core's existing term walk and shallow ownership metadata.
/// The carrier's existing 64-byte allowance per source byte covers original text
/// ownership and bounded interpretation/error copies; spare capacity is additional.
///
/// # Errors
/// Refuses source work/storage exhaustion before a walk frontier can spill.
pub fn term_source_in_policy(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
) -> Result<super::ParseReceipt, GeoError> {
    term_source_observed(value, policy, None)
}

/// The same borrowed source admission with integer-only cancellation polls.
///
/// # Errors
/// Preserves the original source, policy and observer refusals.
pub fn term_source_metered(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: &mut dyn MetricWorkObserver,
) -> Result<super::ParseReceipt, GeoError> {
    term_source_observed(value, policy, Some(observer))
}

fn term_source_observed(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<super::ParseReceipt, GeoError> {
    let mut admission = super::ParseAdmission::new(0, policy, observer)?;
    admit_term_source(value, 0, &mut admission)?;
    Ok(admission.receipt())
}

pub(super) fn admit_term_source(
    value: &TermValue,
    already_admitted_bytes: u64,
    admission: &mut super::ParseAdmission<'_>,
) -> Result<(), GeoError> {
    let mut admitted_heap = already_admitted_bytes;
    let mut admitted_text = already_admitted_bytes;
    let result = value.visit_terms(|term| {
        let step = (|| {
            admission.admit_items(1)?;
            let bytes = term
                .owned_heap_bytes()
                .and_then(|bytes| u64::try_from(bytes).ok())
                .ok_or(GeoError::ArithmeticOverflow("original RDF source storage"))?;
            let covered_heap = admitted_heap.min(bytes);
            admitted_heap -= covered_heap;
            let text = term
                .text_payload_bytes()
                .and_then(|bytes| u64::try_from(bytes).ok())
                .ok_or(GeoError::ArithmeticOverflow("original RDF source text"))?;
            let covered_text = admitted_text.min(text);
            admitted_text -= covered_text;
            let extra_text = text - covered_text;
            let (work, storage) =
                super::ParseAdmission::text_allowance(extra_text, bytes - covered_heap)?;
            if work != 0 {
                admission.admit_items(work)?;
            }
            // Each visited triple increases the pending frontier by at most
            // two references. Four reference slots cover doubled Vec capacity
            // before its original walk pushes children; the inline first 16
            // slots and minimum spill capacity are covered by entry storage.
            let walk = if matches!(term, TermValue::Triple { .. }) {
                4 * size_of::<&TermValue>() as u64
            } else {
                0
            };
            let growth = storage
                .checked_add(walk)
                .ok_or(GeoError::ArithmeticOverflow(
                    "original RDF source allowance",
                ))?;
            admission.retain_source(growth)
        })();
        match step {
            Ok(()) => core::ops::ControlFlow::Continue(()),
            Err(error) => core::ops::ControlFlow::Break(error),
        }
    });
    match result {
        core::ops::ControlFlow::Continue(()) => Ok(()),
        core::ops::ControlFlow::Break(error) => Err(error),
    }
}

/// The immutable standard geometry carrier datatype.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarrierFormat {
    /// GeoSPARQL WKT literal.
    Wkt,
    /// GeoSPARQL GeoJSON literal.
    GeoJson,
}

impl CarrierFormat {
    const fn datatype(self) -> &'static str {
        match self {
            Self::Wkt => purrdf_iri::vocab::ogc::geo::WKT_LITERAL,
            Self::GeoJson => purrdf_iri::vocab::ogc::geo::GEO_JSON_LITERAL,
        }
    }
}

/// Move a string produced by an admitted carrier writer into an RDF literal.
/// Its complete String capacity must already be charged in `context`. The
/// datatype and term are admitted before their allocation; successful storage
/// remains charged alongside that string through the invocation. Refusal drops
/// the moved string and releases its original capacity allowance.
///
/// # Errors
/// Preserves typed work, memory, cancellation and environment refusals.
pub fn term_in_context(
    lexical: String,
    format: CarrierFormat,
    context: &mut MetricContext,
) -> Result<TermValue, GeoError> {
    term_observed(lexical, format, context, None)
}

/// The same move-only output with an aggregate observer that has already
/// charged the writer's entering work and workspace peak.
///
/// # Errors
/// Adds observer refusal before allocation and after the pure output body.
pub fn term_in_context_metered(
    lexical: String,
    format: CarrierFormat,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<TermValue, GeoError> {
    term_observed(lexical, format, context, Some(observer))
}

fn term_observed(
    lexical: String,
    format: CarrierFormat,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<TermValue, GeoError> {
    let datatype = format.datatype();
    let lexical_bytes = lexical.capacity() as u64;
    let output_bytes = (size_of::<TermValue>() + datatype.len()) as u64;
    let cost = ExactArithmeticCost {
        // One datatype byte copy, one bounded allocation and one move-only
        // term construction are declared logical codec units.
        work_items: datatype.len() as u64 + 2,
        workspace_bytes: 0,
        output_bits: 0,
    };
    let limit = context.policy().limits().max_workspace_bytes;
    let evaluate = || {
        let mut type_iri = String::new();
        type_iri
            .try_reserve_exact(datatype.len())
            .map_err(|_| GeoError::MemoryExhausted { limit })?;
        type_iri.push_str(datatype);
        Ok(TermValue::typed_literal(lexical, type_iri))
    };
    let result = if let Some(observer) = observer {
        context.produce_output_metered(cost, output_bytes, evaluate, observer)
    } else {
        context.produce_output(cost, output_bytes, evaluate)
    };
    if result.is_err() {
        context.release_workspace(lexical_bytes)?;
    }
    result
}

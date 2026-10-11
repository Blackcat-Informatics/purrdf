// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Move an admitted carrier string into one admitted RDF literal.

use crate::{GeoError, MetricContext, MetricWorkObserver};
use purrdf_core::TermValue;
use purrdf_xsd::integer::ExactArithmeticCost;

/// Construct a literal through the original native lexical destinations.
///
/// Both borrowed renderers must allocate no private heap. Native numeric
/// renderers prepare their separately admitted groups before entering here.
/// The returned strings retain the observer's original physical account;
/// resident calls use the same factory and own the returned payload themselves.
///
/// # Errors
/// Preserves checked layout, physical allocator, and original admission refusal.
pub fn literal_from_display_metered(
    lexical: impl core::fmt::Display,
    datatype: impl core::fmt::Display,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<TermValue, GeoError> {
    let mut progress = crate::context::WorkProgress::integer(observer);
    progress
        .with_storage(0, |memory| {
            memory.scope(|memory| {
                let lexical = memory.format(&lexical)?;
                let datatype = memory.format(&datatype)?;
                Ok(TermValue::typed_literal(lexical, datatype))
            })
        })
        .map_err(GeoError::Storage)
}

/// Render a native diagnostic under its producer's original physical account.
///
/// The renderer borrows its original fields and creates no private heap.
/// The caller keeps the original account alive through the returned error's
/// destruction or immutable diagnostic publication. Refusal returns its typed
/// storage cause instead of constructing an unpriced lexical fallback.
#[must_use]
pub fn error_from_display_metered(
    construct: fn(String) -> GeoError,
    message: impl core::fmt::Display,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> GeoError {
    let mut progress = crate::context::WorkProgress::integer(observer);
    match progress.with_storage(0, |memory| memory.format(&message)) {
        Ok(detail) => construct(detail),
        Err(error) => GeoError::Storage(error),
    }
}

/// Admit an original borrowed RDF term's complete owned heap capacities and text.
/// Descendants use the core's same native term walk and shallow ownership metadata.
/// The source receipt is the caller-selected parser policy's logical envelope;
/// fresh work-list storage is admitted separately at its actual native layout.
/// Borrowed prebuilt source terms are never treated as fresh query allocations.
///
/// # Errors
/// Refuses source work/storage exhaustion before a walk frontier can spill.
pub fn term_source_in_policy(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
) -> Result<super::ParseReceipt, GeoError> {
    term_source_observed(value, policy, None, None)
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
    term_source_observed(value, policy, Some(observer), None)
}

/// The original borrowed RDF source walk with native storage admitted before
/// work-list growth. The source payload remains caller-owned.
///
/// # Errors
/// Preserves original source, policy, observer and physical refusals.
pub fn term_source_with_storage(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: &mut dyn MetricWorkObserver,
    storage: &super::CarrierStorage<'_>,
) -> Result<super::ParseReceipt, GeoError> {
    term_source_observed(value, policy, Some(observer), Some(storage))
}

fn term_source_observed(
    value: &TermValue,
    policy: crate::ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
    storage: Option<&super::CarrierStorage<'_>>,
) -> Result<super::ParseReceipt, GeoError> {
    let mut admission = super::ParseAdmission::new(0, policy, observer)?;
    if let Some(storage) = storage {
        admission = admission.with_storage(storage);
    }
    admit_term_source(value, 0, &mut admission)?;
    Ok(admission.receipt())
}

pub(super) fn admit_term_source(
    value: &TermValue,
    already_admitted_bytes: u64,
    admission: &mut super::ParseAdmission<'_, '_, '_>,
) -> Result<(), GeoError> {
    let mut admitted_heap = already_admitted_bytes;
    let mut admitted_text = already_admitted_bytes;
    let storage = admission.storage();
    let mut original = None;
    let result = super::with_storage(storage, |memory| {
        value.visit_terms_with_memory(
            |term, _| {
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
                Ok(match step {
                    Ok(()) => core::ops::ControlFlow::Continue(()),
                    Err(error) => {
                        original = Some(error);
                        core::ops::ControlFlow::Break(())
                    }
                })
            },
            memory,
        )
    });
    match original {
        Some(error) => Err(error),
        None => result.map(|_| ()).map_err(GeoError::Storage),
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
    let evaluate = |memory: &mut purrdf_lex::allocation::Memory<
        '_,
        dyn purrdf_lex::allocation::Admission + '_,
    >| {
        let type_iri = memory.string(datatype).map_err(GeoError::Storage)?;
        Ok(TermValue::typed_literal(lexical, type_iri))
    };
    let result = if let Some(observer) = observer {
        context.produce_output_with_storage_metered(cost, output_bytes, evaluate, observer)
    } else {
        let mut resident = purrdf_lex::allocation::Resident;
        let storage: &mut dyn purrdf_lex::allocation::Admission = &mut resident;
        context.produce_output(cost, output_bytes, || {
            evaluate(&mut purrdf_lex::allocation::Memory::new(storage))
        })
    };
    if result.is_err() {
        let _ = context.release_workspace(lexical_bytes);
    }
    result
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prepared value keys over stationary caller-owned values.

use super::{order_permutation, project_admitted, total_order_admitted};
use crate::{AdmittedVec, WorkspaceCapability};
use purrdf_core::TermValue;

/// Return input ordinals in SPARQL value order, projecting each key once.
///
/// `None` is unbound and precedes every bound value. Sorting is stable in both
/// directions: equal keys retain input order. Values stay in their caller's
/// storage throughout preparation and sorting; all borrowed keys are dropped
/// before this function returns, so callers can then move their payloads.
///
/// This resident convenience entry uses the evaluator's native parsed-key and
/// permutation owners. It does not carry a bounded query's workspace or numeric
/// governor.
///
/// # Panics
/// Like [`super::order_values`], panics on physical allocation refusal.
#[must_use]
pub fn order_value_permutation<'a>(
    values: impl IntoIterator<Item = Option<&'a TermValue>>,
    descending: bool,
) -> Vec<usize> {
    let workspace = WorkspaceCapability::resident();
    #[cfg(test)]
    let preparation = phase_window();
    let mut keys = AdmittedVec::new(&workspace);
    for value in values {
        let key = project_admitted(value, &workspace).expect("resident value key projection");
        keys.push(key).expect("resident value key array");
    }
    #[cfg(test)]
    if let Some(preparation) = preparation {
        record_preparation(preparation.close());
    }
    #[cfg(test)]
    let sorting = phase_window();
    let order = order_permutation(keys.len(), &workspace, |left, right| {
        #[cfg(test)]
        note_comparison();
        let order = total_order_admitted(&keys[left].key, &keys[right].key, &workspace)?;
        Ok(if descending { order.reverse() } else { order })
    })
    .expect("resident value permutation");
    #[cfg(test)]
    if let Some(sorting) = sorting {
        record_sorting(sorting.close());
    }
    order.into_iter().collect()
}

#[cfg(test)]
#[path = "value_order/qualification.rs"]
mod qualification;
#[cfg(test)]
pub(super) use qualification::note_projection;
#[cfg(test)]
use qualification::{note_comparison, phase_window, record_preparation, record_sorting};

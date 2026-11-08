// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable original-model admission facts, shared by every worker.

use super::{RecursionSemantics, graph::Analysis};
use crate::error::ShapesError;
use crate::shapes::ModelOwner;
use std::sync::{Arc, Mutex, OnceLock};

/// Successful metadata only: a transient refusal must not poison a model's
/// future admission or retain an owning refusal inside that same model.
/// Analysis itself retains only weak model references.
#[derive(Default)]
pub(crate) struct Admission {
    open: OnceLock<Analysis>,
    closed: OnceLock<Analysis>,
    initializing: Mutex<()>,
}

fn initialize(owner: &Arc<ModelOwner>, closed: bool) -> Result<Analysis, ShapesError> {
    let admission = &owner.recursion_admission;
    let cell = if closed {
        &admission.closed
    } else {
        &admission.open
    };
    if let Some(analysis) = cell.get() {
        return Ok(analysis.clone());
    }
    // Only cold initialization takes the lock. The model walk reads original
    // declarations directly and never re-enters admission, so this cannot
    // recursively acquire its own initializer. It must not poll a signal,
    // call a resolver or suspend while holding this guard: WASM jobs can
    // interleave at those boundaries.
    let _guard = admission.initializing.lock().map_err(|_| {
        ShapesError::internal("original-model recursion admission initializer is poisoned")
    })?;
    if !closed && let Some(analysis) = admission.closed.get() {
        return Ok(analysis.clone());
    }
    if let Some(analysis) = cell.get() {
        return Ok(analysis.clone());
    }
    let analysis = if closed {
        super::model::analyze_closed(owner)?
    } else {
        super::model::analyze(owner)?
    };
    cell.set(analysis.clone()).map_err(|_| {
        ShapesError::internal("original-model recursion admission initialized twice")
    })?;
    Ok(analysis)
}

fn snapshot(owner: &Arc<ModelOwner>) -> Result<Analysis, ShapesError> {
    let admission = &owner.recursion_admission;
    // A successful closed proof also supplies the native dependency analysis.
    // Prefer it after it becomes available, preserving the stronger admission.
    if let Some(analysis) = admission.closed.get().or_else(|| admission.open.get()) {
        return Ok(analysis.clone());
    }
    initialize(owner, false)
}

pub(crate) fn admit(
    owner: &Arc<ModelOwner>,
    semantics: Option<RecursionSemantics>,
) -> Result<(), ShapesError> {
    if semantics.is_some() {
        prove(owner)?;
    }
    snapshot(owner)?.admit(semantics)
}

pub(crate) fn prove(owner: &Arc<ModelOwner>) -> Result<(), ShapesError> {
    initialize(owner, true).map(|_| ())
}

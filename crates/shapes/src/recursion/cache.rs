// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded immutable admission metadata; no data graph or conformance answers.

use super::{RecursionSemantics, graph::Analysis};
use crate::error::ShapesError;
use crate::shapes::ModelOwner;
use std::cell::RefCell;
use std::sync::{Arc, Weak};

const CAPACITY: usize = 32;
struct Entry {
    owner: Weak<ModelOwner>,
    analysis: Analysis,
    closed: bool,
}
thread_local! {
    // Inline slots make cold leaf admission and root export allocation-free.
    static CACHE: RefCell<[Option<Entry>; CAPACITY]> = const { RefCell::new([const { None }; CAPACITY]) };
}

fn snapshot(owner: &Arc<ModelOwner>) -> Result<Analysis, ShapesError> {
    let hit = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let slot = cache.iter().position(|entry| {
            entry.as_ref().is_some_and(|entry| {
                // The Weak keeps the control allocation alive, so an address cannot
                // be reused by another live model while this entry exists.
                entry.owner.as_ptr() == Arc::as_ptr(owner)
            })
        })?;
        cache.swap(0, slot);
        Some(cache[0].as_ref()?.analysis.clone())
    });
    if let Some(analysis) = hit {
        return Ok(analysis);
    }
    let analysis = super::model::analyze(owner)?;
    install(owner, analysis.clone());
    Ok(analysis)
}

pub(crate) fn admit(
    owner: &Arc<ModelOwner>,
    semantics: Option<RecursionSemantics>,
) -> Result<(), ShapesError> {
    if semantics.is_none() && super::model::flat_terminals(owner) {
        return Ok(());
    }
    if semantics.is_some() {
        prove(owner)?;
    }
    snapshot(owner)?.admit(semantics)
}

pub(crate) fn prove(owner: &Arc<ModelOwner>) -> Result<(), ShapesError> {
    if CACHE.with(|cache| {
        cache.borrow().iter().any(|entry| {
            entry
                .as_ref()
                .is_some_and(|entry| entry.closed && entry.owner.as_ptr() == Arc::as_ptr(owner))
        })
    }) {
        return Ok(());
    }
    let analysis = super::model::analyze_closed(owner)?;
    install(owner, analysis);
    CACHE.with(|cache| {
        if let Some(entry) = cache
            .borrow_mut()
            .iter_mut()
            .flatten()
            .find(|entry| entry.owner.as_ptr() == Arc::as_ptr(owner))
        {
            entry.closed = true;
        }
    });
    Ok(())
}

pub(crate) fn install(owner: &Arc<ModelOwner>, analysis: Analysis) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let slot = cache
            .iter()
            .position(|entry| {
                entry.as_ref().is_none_or(|entry| {
                    entry.owner.as_ptr() == Arc::as_ptr(owner) || entry.owner.strong_count() == 0
                })
            })
            .unwrap_or(CAPACITY - 1);
        cache[slot] = None;
        cache[..=slot].rotate_right(1);
        cache[0] = Some(Entry {
            owner: Arc::downgrade(owner),
            analysis,
            closed: false,
        });
    });
}

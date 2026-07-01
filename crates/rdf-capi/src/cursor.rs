// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_quads_for_pattern` + `purrdf_cursor_next`: pattern iteration over a
//! frozen dataset with zero-copy borrowed term views.

use std::sync::Arc;

use purrdf_core::{DatasetView, GraphMatch, QuadIds, RdfDataset, TermId};

use crate::error::PurrdfError;
use crate::handles::PurrdfDataset;
use crate::status::PurrdfStatus;
use crate::term::{
    render_term, view_to_value, PurrdfGraphMatch, PurrdfGraphMatchKind, PurrdfTermView,
};

/// A pattern-quad cursor. It holds an `Arc<RdfDataset>` clone (`_pin`) so the
/// term arena the views borrow into cannot dangle — the cursor stays valid even
/// after every `PurrdfDataset` handle is freed. The matching rows are snapshot
/// into an owned `Vec<QuadIds>` at creation (no live iterator, so no
/// self-referential borrow). Single-threaded.
pub struct PurrdfCursor {
    _pin: Arc<RdfDataset>,
    rows: Vec<QuadIds>,
    pos: usize,
}

/// A resolved subject/predicate/object slot.
enum Slot {
    /// No constraint (the input view pointer was null).
    Unbound,
    /// Constrained to an interned term.
    Bound(TermId),
    /// Constrained to a value that is interned nowhere — the whole pattern is empty.
    Absent,
}

impl Slot {
    fn term_id(&self) -> Option<TermId> {
        match self {
            Slot::Bound(id) => Some(*id),
            _ => None,
        }
    }

    fn is_absent(&self) -> bool {
        matches!(self, Slot::Absent)
    }
}

/// A resolved graph slot.
enum GraphSlot {
    Match(GraphMatch),
    /// A named graph that is interned nowhere — the whole pattern is empty.
    Absent,
}

/// Resolve an optional input term view to a [`Slot`] against `dataset`.
unsafe fn resolve_slot(
    dataset: &RdfDataset,
    view: *const PurrdfTermView,
) -> Result<Slot, PurrdfError> {
    if view.is_null() {
        return Ok(Slot::Unbound);
    }
    let value = view_to_value(&*view)?;
    Ok(match dataset.term_id_by_value(&value) {
        Some(id) => Slot::Bound(id),
        None => Slot::Absent,
    })
}

/// Resolve a `PurrdfGraphMatch` to a [`GraphSlot`] against `dataset`.
unsafe fn resolve_graph(
    dataset: &RdfDataset,
    graph: &PurrdfGraphMatch,
) -> Result<GraphSlot, PurrdfError> {
    let kind = PurrdfGraphMatchKind::try_from(graph.kind)?;
    Ok(match kind {
        PurrdfGraphMatchKind::Any => GraphSlot::Match(GraphMatch::Any),
        PurrdfGraphMatchKind::Default => GraphSlot::Match(GraphMatch::Default),
        PurrdfGraphMatchKind::Named => {
            let value = view_to_value(&graph.name)?;
            match dataset.term_id_by_value(&value) {
                Some(id) => GraphSlot::Match(GraphMatch::Named(id)),
                None => GraphSlot::Absent,
            }
        }
    })
}

/// Open a pattern cursor. Each of `s`/`p`/`o` is a nullable input term view
/// (null = unbound). `g` is a non-null [`PurrdfGraphMatch`] (use kind `Any` for
/// "any graph"). A bound term/graph value that is interned nowhere yields an
/// empty cursor (not an error). The cursor pins the dataset's `Arc`.
///
/// # Safety
/// `dataset` must be a live handle; the view/match pointers must be valid where
/// non-null; the out-params must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_quads_for_pattern(
    dataset: *const PurrdfDataset,
    s: *const PurrdfTermView,
    p: *const PurrdfTermView,
    o: *const PurrdfTermView,
    g: *const PurrdfGraphMatch,
    out_cursor: *mut *mut PurrdfCursor,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    ffi_try!(out_error, {
        if dataset.is_null() || g.is_null() || out_cursor.is_null() {
            return Err(PurrdfError::new(
                PurrdfStatus::NullPointer,
                "null pointer argument to purrdf_quads_for_pattern",
            ));
        }
        let pin = PurrdfDataset::arc(dataset).clone();
        let view = pin.as_ref();
        let subject = resolve_slot(view, s)?;
        let predicate = resolve_slot(view, p)?;
        let object = resolve_slot(view, o)?;
        let graph = resolve_graph(view, &*g)?;

        let rows: Vec<QuadIds> = match graph {
            GraphSlot::Absent => Vec::new(),
            GraphSlot::Match(_)
                if subject.is_absent() || predicate.is_absent() || object.is_absent() =>
            {
                Vec::new()
            }
            // `RdfDataset` overrides the `DatasetView::quads_for_pattern` default
            // with the indexed lookup, so this takes the fast path.
            GraphSlot::Match(graph_match) => view
                .quads_for_pattern(
                    subject.term_id(),
                    predicate.term_id(),
                    object.term_id(),
                    graph_match,
                )
                .collect(),
        };

        *out_cursor = Box::into_raw(Box::new(PurrdfCursor {
            _pin: pin,
            rows,
            pos: 0,
        }));
        Ok(PurrdfStatus::Ok)
    })
}

/// Advance to the next quad. On success fills the four term views; `out_has_graph`
/// is `0` for default-graph quads (the `out_g` view is then a zeroed placeholder).
/// The `PurrdfStr` pointers inside the views are valid until the next
/// `purrdf_cursor_next` on this cursor or `purrdf_cursor_free`. Returns
/// `PurrdfStatus::CursorExhausted` (a non-error terminal signal, `> 0`) when no
/// rows remain.
///
/// # Safety
/// `cursor` must be a live cursor; the out-params must be writable.
#[no_mangle]
pub unsafe extern "C" fn purrdf_cursor_next(
    cursor: *mut PurrdfCursor,
    out_s: *mut PurrdfTermView,
    out_p: *mut PurrdfTermView,
    out_o: *mut PurrdfTermView,
    out_g: *mut PurrdfTermView,
    out_has_graph: *mut u8,
) -> i32 {
    ffi_guard!(PurrdfStatus::Panic as i32, {
        if cursor.is_null()
            || out_s.is_null()
            || out_p.is_null()
            || out_o.is_null()
            || out_g.is_null()
            || out_has_graph.is_null()
        {
            return PurrdfStatus::NullPointer as i32;
        }
        let cursor = &mut *cursor;
        if cursor.pos >= cursor.rows.len() {
            return PurrdfStatus::CursorExhausted as i32;
        }
        let quad = cursor.rows[cursor.pos];
        cursor.pos += 1;
        let dataset = cursor._pin.as_ref();
        render_term(dataset, quad.s, &mut *out_s);
        render_term(dataset, quad.p, &mut *out_p);
        render_term(dataset, quad.o, &mut *out_o);
        match quad.g {
            Some(graph_id) => {
                render_term(dataset, graph_id, &mut *out_g);
                *out_has_graph = 1;
            }
            None => {
                *out_g = PurrdfTermView::empty();
                *out_has_graph = 0;
            }
        }
        PurrdfStatus::Ok as i32
    })
}

/// Release a cursor handle. No-op on null.
///
/// # Safety
/// `cursor` must be null or a live cursor not already freed.
#[no_mangle]
pub unsafe extern "C" fn purrdf_cursor_free(cursor: *mut PurrdfCursor) {
    ffi_guard!((), {
        if !cursor.is_null() {
            drop(Box::from_raw(cursor));
        }
    })
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed reached-term handles; no source rows or term-value dictionary.

use super::{UnionError, UnionTermId, check_stats};
use crate::{
    BlankScope, DatasetView, TermGuard, TermRef, TermValue, ViewLimits, ViewStats, ViewTermId,
    ViewWork,
};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};

pub(super) struct Index<I> {
    pub(super) ids: Vec<I>,
    pub(super) aliases: Vec<Option<UnionTermId>>,
    pub(super) states: Vec<u8>,
    pub(super) scopes: Vec<(BlankScope, BlankScope)>,
    pub(super) rebound: Vec<Option<String>>,
    pub(super) reverse: OrderedTable<UnionTermId, I>,
}

impl<I: ViewTermId> Index<I> {
    pub(super) fn position(&self, id: I) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    pub(super) fn alias(&self, id: I) -> Option<UnionTermId> {
        self.position(id).and_then(|at| self.aliases[at])
    }

    pub(super) fn scope(&self, original: BlankScope) -> Option<BlankScope> {
        self.scopes
            .binary_search_by_key(&original, |(old, _)| *old)
            .ok()
            .map(|at| self.scopes[at].1)
    }

    pub(super) fn local(&self, id: UnionTermId) -> Option<I> {
        self.reverse.get(&id).copied()
    }

    pub(super) fn published_work(&self) -> Result<ViewWork, StorageError> {
        let mut work = ViewWork::default();
        // Count published ID fields, excluding table slack, enum tags and hashes.
        for (count, width) in [
            (self.ids.len(), size_of::<I>()),
            (self.aliases.len(), size_of::<UnionTermId>()),
            (
                self.reverse.len(),
                size_of::<I>() + size_of::<UnionTermId>(),
            ),
            (self.scopes.len(), 2 * size_of::<BlankScope>()),
        ] {
            work.copied_index_bytes = count
                .checked_mul(width)
                .and_then(|bytes| work.copied_index_bytes.checked_add(bytes))
                .ok_or(StorageError::SizeOverflow)?;
        }
        for text in self.rebound.iter().flatten() {
            work.copied_terms = work
                .copied_terms
                .checked_add(1)
                .ok_or(StorageError::SizeOverflow)?;
            work.copied_text_bytes = work
                .copied_text_bytes
                .checked_add(text.len())
                .ok_or(StorageError::SizeOverflow)?;
        }
        Ok(work)
    }
}

fn reach<I: ViewTermId, L, R, S: Admission + ?Sized>(
    id: I,
    ids: &mut OrderedTable<I, ()>,
    pending: &mut Vec<I>,
    stats: &mut ViewStats,
    limits: ViewLimits,
    memory: &mut Memory<'_, S>,
) -> Result<(), UnionError<L, R>> {
    if ids.get(&id).is_some() {
        return Ok(());
    }
    stats.retained_terms = stats
        .retained_terms
        .checked_add(1)
        .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
    check_stats(limits, stats)?;
    ids.insert(id, (), memory)?;
    memory.push(pending, id)?;
    Ok(())
}

pub(super) fn capture<D: DatasetView, L, R, S: Admission + ?Sized>(
    view: &D,
    error: impl Fn(D::ReadError) -> UnionError<L, R> + Copy,
    stats: &mut ViewStats,
    limits: ViewLimits,
    memory: &mut Memory<'_, S>,
) -> Result<Index<D::Id>, UnionError<L, R>> {
    view.checked_read(|view| {
        let mut ids = OrderedTable::default();
        let mut pending = Vec::new();
        let mut scopes = OrderedTable::default();
        for row in view
            .quads()
            .chain(view.reifier_quads())
            .chain(view.annotation_quads())
        {
            stats.retained_rows = stats
                .retained_rows
                .checked_add(1)
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
            check_stats(limits, stats)?;
            for id in [Some(row.s), Some(row.p), Some(row.o), row.g]
                .into_iter()
                .flatten()
            {
                reach(id, &mut ids, &mut pending, stats, limits, memory)?;
            }
        }
        for graph in view.named_graphs() {
            reach(graph, &mut ids, &mut pending, stats, limits, memory)?;
        }
        while let Some(id) = pending.pop() {
            let guard = view.resolve(id).map_err(error)?;
            let bytes = match guard.term() {
                TermRef::Iri(iri) => iri.len(),
                TermRef::Blank { label, scope } => {
                    scopes.insert(scope, (), memory)?;
                    label.len()
                }
                TermRef::Triple { s, p, o } => {
                    for child in [s, p, o] {
                        reach(child, &mut ids, &mut pending, stats, limits, memory)?;
                    }
                    0
                }
                TermRef::Literal {
                    lexical,
                    datatype,
                    language,
                    ..
                } => {
                    reach(datatype, &mut ids, &mut pending, stats, limits, memory)?;
                    let datatype_guard = view.resolve(datatype).map_err(error)?;
                    let TermRef::Iri(datatype) = datatype_guard.term() else {
                        return Err(UnionError::Invariant("literal datatype is not an IRI"));
                    };
                    let mut blanks = crate::cdt_blank::cdt_embedded_blanks_with_memory(
                        lexical, datatype, memory,
                    )?;
                    // Keep the admitted Vec allocation alive until its exact release below.
                    while let Some((label, scope)) = blanks.pop() {
                        scopes.insert(scope, (), memory)?;
                        let blank = TermValue::Blank { label, scope };
                        let found = view.term_id_by_value(&blank).map_err(error)?.ok_or(
                            UnionError::Invariant("composite blank has no source identity"),
                        )?;
                        // The native scanner allocated this label under our memory.
                        if let TermValue::Blank { label, .. } = blank {
                            memory.release_string(label)?;
                        }
                        reach(found, &mut ids, &mut pending, stats, limits, memory)?;
                    }
                    memory.release_vec(blanks)?;
                    lexical
                        .len()
                        .checked_add(language.map_or(0, str::len))
                        .ok_or(UnionError::Storage(StorageError::SizeOverflow))?
                }
            };
            stats.retained_payload_bytes = stats
                .retained_payload_bytes
                .checked_add(bytes)
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
            check_stats(limits, stats)?;
        }
        memory.release_vec(pending)?;
        let mut ordered = Vec::new();
        memory.reserve(&mut ordered, ids.len())?;
        while let Some((id, ())) = ids.pop_first() {
            ordered.push(id);
        }
        ids.release(memory)?;
        let mut scope_map = Vec::new();
        memory.reserve(&mut scope_map, scopes.len())?;
        while let Some((scope, ())) = scopes.pop_first() {
            scope_map.push((scope, scope));
        }
        scopes.release(memory)?;
        let mut aliases = Vec::new();
        let mut states = Vec::new();
        let mut rebound = Vec::new();
        memory.reserve(&mut aliases, ordered.len())?;
        memory.reserve(&mut states, ordered.len())?;
        memory.reserve(&mut rebound, ordered.len())?;
        aliases.resize(ordered.len(), None);
        states.resize(ordered.len(), 0);
        rebound.resize_with(ordered.len(), || None);
        Ok(Index {
            ids: ordered,
            aliases,
            states,
            scopes: scope_map,
            rebound,
            reverse: OrderedTable::default(),
        })
    })
    .map_err(error)?
}

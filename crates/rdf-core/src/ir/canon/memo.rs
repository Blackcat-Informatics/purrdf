// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Issuance-local, source-admitted exact subsearch reuse.

use std::alloc::Layout;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};

use super::{CanonError, CanonState, Component, HashHex, IdIssuer};
use crate::{DatasetView, ViewTermId};

struct Footprint<Id> {
    component: Component<Id>,
    blanks: Vec<Id>,
}

/// Complete blank sets obtained by the existing component walk, not a second
/// source traversal. The catalog never shrinks after allocation.
pub(super) struct Footprints<Id> {
    components: Vec<Footprint<Id>>,
}

impl<Id: ViewTermId> Footprints<Id> {
    pub(super) fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub(super) fn push<S: Admission + ?Sized>(
        &mut self,
        component: Component<Id>,
        blanks: BTreeSet<Id>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        // No incident entry can refer to a component without a blank. In
        // particular, a zero-blank input acquires no extra provider reservation.
        if blanks.is_empty() {
            return Ok(());
        }
        let blanks = memory.collect(blanks)?;
        memory.push(&mut self.components, Footprint { component, blanks })
    }

    pub(super) fn finish(&mut self) {
        self.components
            .sort_unstable_by_key(|entry| entry.component);
    }

    fn get(&self, component: Component<Id>) -> &[Id] {
        let index = self
            .components
            .binary_search_by_key(&component, |entry| entry.component)
            .expect("every incident component retained its complete successful blank walk");
        &self.components[index].blanks
    }

    fn storage_bytes(&self) -> Result<usize, StorageError> {
        let mut bytes = Layout::array::<Footprint<Id>>(self.components.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size();
        for entry in &self.components {
            bytes = bytes
                .checked_add(
                    Layout::array::<Id>(entry.blanks.capacity())
                        .map_err(|_| StorageError::SizeOverflow)?
                        .size(),
                )
                .ok_or(StorageError::SizeOverflow)?;
        }
        Ok(bytes)
    }

    pub(super) fn release<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        for entry in &mut self.components {
            memory.release_vec(std::mem::take(&mut entry.blanks))?;
        }
        memory.release_vec(self.components)
    }
}

/// Prefixes and canonical state are fixed within one invocation. These ordinals
/// therefore describe the exact emitted labels, solely for key comparison.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Label {
    Canonical(usize),
    Temporary(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Prefix<Id> {
    focus: Id,
    focus_label: Label,
    next: usize,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key<Id> {
    prefix: Prefix<Id>,
    frontier: Vec<(Id, Label)>,
}

struct Value<Id> {
    hash: HashHex,
    suffix: Vec<Id>,
}

/// Each vector has exactly N capacity and is reused between keys. Marking an ID
/// before both frontier insertion and enqueue keeps every populated extent ≤ N.
struct Workspace<Id> {
    canonical: Vec<(Id, usize)>,
    prefix: Vec<(Id, usize)>,
    visited: Vec<bool>,
    pending: Vec<Id>,
    frontier: Vec<(Id, Label)>,
}

impl<Id: ViewTermId> Workspace<Id> {
    fn new<D: DatasetView<Id = Id>, S: Admission + ?Sized>(
        state: &CanonState<'_, D>,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, CanonError<D::ReadError>> {
        memory.try_scope(|memory| {
            state.check_stop().map_err(CanonError::with_read_error)?;
            let count = state.blanks.len();
            let mut canonical = Vec::new();
            let mut prefix = Vec::new();
            let mut visited = Vec::new();
            let mut pending = Vec::new();
            let mut frontier = Vec::new();
            memory.reserve(&mut canonical, count)?;
            memory.reserve(&mut prefix, count)?;
            memory.reserve(&mut visited, count)?;
            memory.reserve(&mut pending, count)?;
            memory.reserve(&mut frontier, count)?;
            for _ in 0..count {
                state.check_stop().map_err(CanonError::with_read_error)?;
                memory.push(&mut visited, false)?;
            }
            for (ordinal, &id) in state.canonical.order().iter().enumerate() {
                state.check_stop().map_err(CanonError::with_read_error)?;
                memory.push(&mut canonical, (id, ordinal))?;
            }
            state.check_stop().map_err(CanonError::with_read_error)?;
            canonical.sort_unstable_by_key(|&(id, _)| id);
            state.check_stop().map_err(CanonError::with_read_error)?;
            Ok(Self {
                canonical,
                prefix,
                visited,
                pending,
                frontier,
            })
        })
    }

    fn label(&self, id: Id) -> Option<Label> {
        self.canonical
            .binary_search_by_key(&id, |&(id, _)| id)
            .ok()
            .map(|index| Label::Canonical(self.canonical[index].1))
            .or_else(|| {
                self.prefix
                    .binary_search_by_key(&id, |&(id, _)| id)
                    .ok()
                    .map(|index| Label::Temporary(self.prefix[index].1))
            })
    }

    fn prepare<D: DatasetView<Id = Id>, S: Admission + ?Sized>(
        &mut self,
        state: &CanonState<'_, D>,
        footprints: &Footprints<Id>,
        focus: Id,
        issuer: &IdIssuer<Id>,
        entry_len: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<Prefix<Id>, CanonError<D::ReadError>> {
        state.check_stop().map_err(CanonError::with_read_error)?;
        self.prefix.clear();
        // Pending retains every traversed unknown, including the focus; frontier
        // retains every other marked ID. Reuse both as the sparse reset inventory.
        for (index, id) in self
            .pending
            .iter()
            .copied()
            .chain(self.frontier.iter().map(|&(id, _)| id))
            .enumerate()
        {
            if index.is_multiple_of(256) {
                state.check_stop().map_err(CanonError::with_read_error)?;
            }
            self.visited[state.blank_ordinal(id)] = false;
        }
        state.check_stop().map_err(CanonError::with_read_error)?;
        self.pending.clear();
        self.frontier.clear();
        // Successful issuers only append: the final prefix reconstructs the
        // original entry bindings without retaining a key in every frame.
        for (ordinal, &id) in issuer.order()[..entry_len].iter().enumerate() {
            state.check_stop().map_err(CanonError::with_read_error)?;
            memory.push(&mut self.prefix, (id, ordinal))?;
        }
        state.check_stop().map_err(CanonError::with_read_error)?;
        self.prefix.sort_unstable_by_key(|&(id, _)| id);
        state.check_stop().map_err(CanonError::with_read_error)?;
        let prefix = Prefix {
            focus,
            focus_label: self
                .label(focus)
                .expect("a root or recursive focus was issued before frame entry"),
            next: entry_len,
        };
        self.visited[state.blank_ordinal(focus)] = true;
        memory.push(&mut self.pending, focus)?;
        let mut cursor = 0;
        while cursor < self.pending.len() {
            let id = self.pending[cursor];
            cursor += 1;
            state.check_stop().map_err(CanonError::with_read_error)?;
            for &component in state.incident.get(&id).map_or_default(Vec::as_slice) {
                state.check_stop().map_err(CanonError::with_read_error)?;
                for &related in footprints.get(component) {
                    state.check_stop().map_err(CanonError::with_read_error)?;
                    let ordinal = state.blank_ordinal(related);
                    if std::mem::replace(&mut self.visited[ordinal], true) {
                        continue;
                    }
                    if let Some(label) = self.label(related) {
                        memory.push(&mut self.frontier, (related, label))?;
                    } else {
                        memory.push(&mut self.pending, related)?;
                    }
                }
            }
        }
        state.check_stop().map_err(CanonError::with_read_error)?;
        self.frontier.sort_unstable();
        state.check_stop().map_err(CanonError::with_read_error)?;
        Ok(prefix)
    }

    fn compare(&self, prefix: Prefix<Id>, stored: &Key<Id>) -> Ordering {
        prefix
            .cmp(&stored.prefix)
            .then_with(|| self.frontier.cmp(&stored.frontier))
    }

    /// Report the exact issued dependencies already obtained by preparation.
    /// The focus is expanded even though issued; every other issued ID is a frontier.
    fn observe_dependencies<D: DatasetView<Id = Id>>(
        &self,
        state: &CanonState<'_, D>,
        focus: Id,
        mut observe: impl FnMut(Id),
    ) -> Result<(), CanonError<D::ReadError>> {
        for id in std::iter::once(focus).chain(self.frontier.iter().map(|&(id, _)| id)) {
            state.check_stop().map_err(CanonError::with_read_error)?;
            observe(id);
        }
        Ok(())
    }

    fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_vec(self.canonical)?;
        memory.release_vec(self.prefix)?;
        memory.release_vec(self.visited)?;
        memory.release_vec(self.pending)?;
        memory.release_vec(self.frontier)
    }
}

/// A local retention limit precedes the original account. Its miss is distinct
/// from every real provider/allocator/arithmetic/refund error.
struct Quota<'a, 'account, S: Admission + ?Sized> {
    original: &'a mut Memory<'account, S>,
    live: usize,
    limit: usize,
    missed: bool,
    failure: Option<StorageError>,
}

impl<S: Admission + ?Sized> Admission for Quota<'_, '_, S> {
    fn resize(&mut self, next: usize) -> Result<(), StorageError> {
        if let Some(cause) = self.failure {
            return Err(cause);
        }
        if next > self.limit {
            self.missed = true;
            return Err(StorageError::AdmissionFailed);
        }
        let result = if next >= self.live {
            self.original.add_bytes(next - self.live)
        } else {
            self.original.release_bytes(self.live - next)
        };
        match result {
            Ok(()) => {
                self.live = next;
                Ok(())
            }
            Err(cause) => {
                self.failure = Some(cause);
                Err(cause)
            }
        }
    }
}

fn retain<T, S: Admission + ?Sized>(
    original: &mut Memory<'_, S>,
    live: &mut usize,
    limit: usize,
    operation: impl FnOnce(&mut Memory<'_, Quota<'_, '_, S>>) -> Result<T, StorageError>,
) -> Result<Option<T>, StorageError> {
    let mut quota = Quota {
        original,
        live: *live,
        limit,
        missed: false,
        failure: None,
    };
    let result = {
        let mut memory = Memory::resume(&mut quota, *live);
        // A surviving table is NOT a generic rollback scope. Every staged
        // allocation is explicitly released; retained arena growth stays live.
        let result = operation(&mut memory);
        *live = memory.admitted_bytes();
        result
    };
    match result {
        Ok(value) => Ok(Some(value)),
        Err(StorageError::AdmissionFailed) if quota.missed => match quota.failure {
            Some(cause) => Err(cause),
            None => Ok(None),
        },
        Err(cause) => Err(cause),
    }
}

/// One invocation fixes source, canonical issuer, prefixes and hash profile.
/// Only resident input-derived bytes determine its retention ceiling.
pub(super) struct Memo<'a, Id> {
    footprints: &'a Footprints<Id>,
    workspace: Option<Workspace<Id>>,
    table: OrderedTable<Key<Id>, Value<Id>>,
    live: usize,
    limit: usize,
}

impl<'a, Id: ViewTermId> Memo<'a, Id> {
    pub(super) fn new(footprints: &'a Footprints<Id>) -> Result<Self, StorageError> {
        Ok(Self {
            footprints,
            workspace: None,
            table: OrderedTable::default(),
            live: 0,
            limit: footprints.storage_bytes()?,
        })
    }

    fn workspace<D: DatasetView<Id = Id>, S: Admission + ?Sized>(
        &mut self,
        state: &CanonState<'_, D>,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), CanonError<D::ReadError>> {
        if self.workspace.is_none() {
            self.workspace = Some(Workspace::new(state, memory)?);
        }
        Ok(())
    }

    /// Called only after the original frame stop/charge/source initialization.
    pub(super) fn lookup<D: DatasetView<Id = Id>, S: Admission + ?Sized>(
        &mut self,
        state: &CanonState<'_, D>,
        focus: Id,
        issuer: &mut IdIssuer<Id>,
        memory: &mut Memory<'_, S>,
        observe: impl FnMut(Id),
    ) -> Result<Option<HashHex>, CanonError<D::ReadError>> {
        if self.table.is_empty() {
            return Ok(None);
        }
        self.workspace(state, memory)?;
        let workspace = self
            .workspace
            .as_mut()
            .expect("a memo lookup initialized its resident workspace");
        let prefix = workspace.prepare(
            state,
            self.footprints,
            focus,
            issuer,
            issuer.order().len(),
            memory,
        )?;
        let Some(value) = self.table.get_by(|key| workspace.compare(prefix, key)) else {
            return Ok(None);
        };
        workspace.observe_dependencies(state, focus, observe)?;
        // The complete frontier includes every initially issued ID the result
        // can encounter. Equal keys make each suffix ID fresh in this caller.
        for &id in &value.suffix {
            state.check_stop().map_err(CanonError::with_read_error)?;
            assert!(
                !issuer.has(id),
                "a cached suffix extends its original caller"
            );
            issuer.issue(id);
        }
        Ok(Some(value.hash))
    }

    /// The suspended parent candidate still owns the exact child-entry issuer.
    /// Successful children preserve that prefix, so no key is owned per depth.
    pub(super) fn completed<D: DatasetView<Id = Id>, S: Admission + ?Sized>(
        &mut self,
        state: &CanonState<'_, D>,
        (focus, hash): (Id, HashHex),
        issuer: &IdIssuer<Id>,
        entry: &IdIssuer<Id>,
        memory: &mut Memory<'_, S>,
        observe: impl FnMut(Id),
    ) -> Result<(), CanonError<D::ReadError>> {
        self.workspace(state, memory)?;
        let workspace = self
            .workspace
            .as_mut()
            .expect("a completed child initialized its resident workspace");
        let prefix = workspace.prepare(
            state,
            self.footprints,
            focus,
            issuer,
            entry.order().len(),
            memory,
        )?;
        workspace.observe_dependencies(state, focus, observe)?;
        if self
            .table
            .get_by(|key| workspace.compare(prefix, key))
            .is_some()
        {
            return Ok(());
        }
        let suffix = &issuer.order()[entry.order().len()..];
        loop {
            state.check_stop().map_err(CanonError::with_read_error)?;
            let table = &mut self.table;
            let published = retain(memory, &mut self.live, self.limit, |memory| {
                publish(table, prefix, &workspace.frontier, suffix, hash, memory)
            })?;
            if published.is_some() {
                return Ok(());
            }
            if let Some((key, value)) = self.table.pop_first() {
                retain(memory, &mut self.live, self.limit, |memory| {
                    release_entry(key, value, memory)
                })?
                .expect("destroying a retained entry cannot exceed its quota");
            } else if self.table.storage_bytes()? != 0 {
                let table = std::mem::take(&mut self.table);
                retain(memory, &mut self.live, self.limit, |memory| {
                    table.release(memory)
                })?
                .expect("destroying an empty arena cannot exceed its quota");
            } else {
                // This complete result does not fit the input-derived retention
                // ceiling. Its mandatory solver result remains unchanged.
                return Ok(());
            }
        }
    }

    pub(super) fn release<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        while let Some((key, value)) = self.table.pop_first() {
            retain(memory, &mut self.live, self.limit, |memory| {
                release_entry(key, value, memory)
            })?
            .expect("destroying retained entries cannot exceed their quota");
        }
        retain(memory, &mut self.live, self.limit, |memory| {
            self.table.release(memory)
        })?
        .expect("destroying the retained arena cannot exceed its quota");
        if let Some(workspace) = self.workspace {
            workspace.release(memory)?;
        }
        Ok(())
    }
}

fn release_entry<Id, S: Admission + ?Sized>(
    key: Key<Id>,
    value: Value<Id>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    memory.release_vec(key.frontier)?;
    memory.release_vec(value.suffix)
}

fn release_staged<Id, S: Admission + ?Sized>(
    frontier: Vec<(Id, Label)>,
    suffix: Vec<Id>,
    cause: StorageError,
    memory: &mut Memory<'_, Quota<'_, '_, S>>,
) -> StorageError {
    let quota_only = matches!(cause, StorageError::AdmissionFailed)
        && memory.admission().missed
        && memory.admission().failure.is_none();
    let suffix_cleanup = memory.release_vec(suffix);
    let frontier_cleanup = memory.release_vec(frontier);
    if quota_only {
        // A real cleanup error outranks the deliberate retention miss. An
        // original real operation error, conversely, remains authoritative.
        suffix_cleanup.and(frontier_cleanup).err().unwrap_or(cause)
    } else {
        cause
    }
}

fn publish<Id: ViewTermId, S: Admission + ?Sized>(
    table: &mut OrderedTable<Key<Id>, Value<Id>>,
    prefix: Prefix<Id>,
    frontier: &[(Id, Label)],
    suffix: &[Id],
    hash: HashHex,
    memory: &mut Memory<'_, Quota<'_, '_, S>>,
) -> Result<(), StorageError> {
    let frontier = memory.collect(frontier.iter().copied())?;
    let suffix = match memory.collect(suffix.iter().copied()) {
        Ok(suffix) => suffix,
        Err(cause) => {
            return Err(release_staged(frontier, Vec::new(), cause, memory));
        }
    };
    // reserve_insert can refuse while the caller still owns both staged
    // payloads. prepare_insert's owned key must never cross a quota failure.
    if let Err(cause) = table.reserve_insert(memory) {
        return Err(release_staged(frontier, suffix, cause, memory));
    }
    let entry = table.prepare_insert(Key { prefix, frontier }, memory, |a, b| {
        Ok::<_, StorageError>(a.cmp(b))
    })?;
    assert!(
        !entry.is_occupied(),
        "memo publication follows its exclusive vacant borrowed lookup"
    );
    let _ = entry.publish(Value { hash, suffix });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Provider {
        live: usize,
        calls: usize,
        refuse_growth: bool,
        refuse_refund_once: bool,
    }

    impl Admission for Provider {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            self.calls += 1;
            if bytes > self.live && self.refuse_growth {
                return Err(StorageError::AdmissionFailed);
            }
            if bytes < self.live && std::mem::take(&mut self.refuse_refund_once) {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = bytes;
            Ok(())
        }
    }

    /// A deliberate retention miss never calls the provider for its rejected
    /// layout, while real admission/refund failures remain terminal errors.
    #[test]
    fn retention_capacity_miss_cannot_mask_real_admission_or_refund_failure() {
        let mut provider = Provider::default();
        let mut memory = Memory::new(&mut provider);
        let mut live = 0;
        let attempt = |memory: &mut Memory<'_, Quota<'_, '_, Provider>>| {
            let staged = memory.collect([0_u8; 4])?;
            let cause = memory
                .collect([0_u8])
                .expect_err("the fifth byte exceeds retention");
            memory.release_vec(staged)?;
            Err::<(), _>(cause)
        };
        assert_eq!(retain(&mut memory, &mut live, 4, attempt), Ok(None));
        assert_eq!(
            memory.admission().calls,
            2,
            "only admitted growth and destruction reach the provider"
        );
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(live, 0);

        memory.admission_mut().refuse_growth = true;
        let failure = retain(&mut memory, &mut live, 8, |memory| {
            let bytes = memory.collect([0_u8])?;
            memory.release_vec(bytes)
        });
        assert_eq!(failure, Err(StorageError::AdmissionFailed));
        assert_eq!(memory.admitted_bytes(), 0);

        memory.admission_mut().refuse_growth = false;
        memory.admission_mut().refuse_refund_once = true;
        let failure = memory.try_scope(|memory| {
            let mut live = 0;
            retain(memory, &mut live, 4, |memory| {
                let staged = memory.collect([0_u8; 4])?;
                let cause = memory
                    .collect([0_u8])
                    .expect_err("the next byte is a quota miss");
                // The real refund error overrides the earlier deliberate miss.
                memory.release_vec(staged)?;
                Err::<(), _>(cause)
            })
        });
        assert_eq!(failure, Err(StorageError::AdmissionFailed));
        assert_eq!(
            memory.admitted_bytes(),
            0,
            "outer terminal cleanup still refunds its owners"
        );
        assert_eq!(memory.admission().live, 0);
    }
}

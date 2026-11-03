// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The immutable input contract shared by demand compilation and native projection.

use super::{BTreeMap, BTreeSet, ClauseTerm};
use crate::allocation::{construct_in, flush_retired_bytes, reserve_array_in, reserve_ordered_in};
use crate::dictionary::{DictionaryError, DictionaryTerm, TermDictionary, TermIdentity};
use crate::id::RowId;
use crate::store::{Arity, RelationStore, StoreShape};
use purrdf_lex::allocation::{Admission, Memory, OrderedTable, StorageError};
use purrdf_lex::walk::HeapSortFrame;
use std::sync::Arc;

mod update;
pub(crate) use update::Update;

/// No caller can attach arbitrary predicate families to an executable. The
/// native checker consumes this concrete mapping contract and the actual rules.
#[derive(Debug)]
pub(crate) struct Plan {
    map: Option<String>,
    map_surface: Option<String>,
    shadows: BTreeMap<String, String>,
    shadow_input: bool,
    internal: BTreeSet<String>,
    internal_order: Vec<String>,
    constants: Vec<ClauseTerm>,
    unit: String,
    unit_surface: String,
    authored_variables: BTreeSet<String>,
    generated_variables: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct Member<I> {
    references: usize,
    pinned: bool,
    target: Option<I>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Metadata,
    Collect,
    Constants,
    Sort,
    Internal,
    Members,
    PublishMap,
    Mirrors,
    Complete,
}

/// One resumable native source projection. Forward and inverse ownership are
/// keyed by actual dictionary identity, never by rendered text or Debug output.
pub(crate) struct Runtime<I: TermIdentity, const N: usize>
where
    Arity<N>: StoreShape<N>,
{
    plan: Arc<Plan>,
    metadata: crate::store::MetadataPreparation<I, N>,
    members: OrderedTable<I, Member<I>>,
    inverse: OrderedTable<I, I>,
    private_names: OrderedTable<String, ()>,
    internal_ids: OrderedTable<I, ()>,
    internal_cursor: usize,
    order: Vec<I>,
    sort: Option<HeapSortFrame>,
    phase: Phase,
    original_slots: u64,
    original_terms: u64,
    cursor: u64,
    coordinate: usize,
    constant: usize,
    member: usize,
    current: Option<(RowId, [I; N])>,
    candidate: usize,
    pending_name: Option<String>,
    pending_target: Option<I>,
    publishing: Option<I>,
    pending_map: Option<[I; N]>,
    unit_admitted: bool,
    added_rows: usize,
    added_terms: u64,
    retired_bytes: usize,
}

impl<I: TermIdentity, const N: usize> std::fmt::Debug for Runtime<I, N>
where
    Arity<N>: StoreShape<N>,
{
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("DemandInputProjection")
            .field("phase", &self.phase)
            .field("original_slots", &self.original_slots)
            .field("added_rows", &self.added_rows)
            .finish_non_exhaustive()
    }
}

/// A row publication is returned to the native state so its original height
/// representation can advance in lockstep before another factory runs.
pub(crate) enum Progress {
    Paused,
    Advanced,
    Inserted(RowId),
    Complete,
}

pub(crate) enum Error<E> {
    Native(DictionaryError<E>),
    Identity,
}
impl<E> From<DictionaryError<E>> for Error<E> {
    fn from(error: DictionaryError<E>) -> Self {
        Self::Native(error)
    }
}
impl<E> From<StorageError> for Error<E> {
    fn from(error: StorageError) -> Self {
        Self::Native(DictionaryError::Storage(error))
    }
}
impl<E> Error<E> {
    pub(crate) fn into_eval(self) -> crate::seminaive::EvalError<E> {
        match self {
            Self::Native(DictionaryError::Source(cause)) => {
                crate::seminaive::EvalError::Source(cause)
            }
            Self::Native(DictionaryError::Storage(cause)) => {
                crate::seminaive::EvalError::Storage(cause)
            }
            Self::Identity => crate::seminaive::EvalError::DemandInput {
                reason:
                    "a private predicate identity overlaps an original or another private predicate"
                        .to_owned(),
            },
        }
    }
}

impl<I: TermIdentity, const N: usize> Runtime<I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(crate) fn new<D: TermDictionary<Id = I>>(
        plan: Arc<Plan>,
        rel: &RelationStore<D, N>,
    ) -> Self {
        Self {
            plan,
            metadata: crate::store::MetadataPreparation::default(),
            members: OrderedTable::default(),
            inverse: OrderedTable::default(),
            private_names: OrderedTable::default(),
            order: Vec::new(),
            internal_ids: OrderedTable::default(),
            internal_cursor: 0,
            sort: None,
            phase: Phase::Metadata,
            original_slots: rel.row_slots(),
            original_terms: rel.interner().term_count(),
            cursor: 0,
            coordinate: 0,
            constant: 0,
            member: 0,
            current: None,
            candidate: 0,
            pending_name: None,
            pending_target: None,
            publishing: None,
            unit_admitted: false,
            pending_map: None,
            added_rows: 0,
            added_terms: 0,
            retired_bytes: 0,
        }
    }

    pub(crate) const fn seeded_rows(&self) -> usize {
        self.added_rows
    }
    pub(crate) const fn seeded_terms(&self) -> u64 {
        self.added_terms
    }
    pub(crate) fn original(&self, id: I) -> bool {
        self.members.get(&id).is_some()
    }
    pub(crate) fn shadow_source(&self, id: I) -> Option<I> {
        self.inverse.get(&id).copied()
    }
    pub(crate) fn shadow_target(&self, id: I) -> Option<I> {
        self.members.get(&id).and_then(|member| member.target)
    }
    pub(crate) fn private_id(&self, id: I) -> bool {
        self.inverse.get(&id).is_some() || self.internal_ids.get(&id).is_some()
    }
    pub(crate) fn uses_map(&self) -> bool {
        self.plan.map.is_some()
    }
    pub(crate) fn mirrors(&self) -> bool {
        self.plan.shadow_input
    }
    pub(crate) fn references(&self, id: I) -> usize {
        self.members.get(&id).map_or(0, |member| member.references)
    }
    pub(crate) fn active_with(&self, id: I, references: usize) -> bool {
        references != 0 || self.members.get(&id).is_some_and(|member| member.pinned)
    }
    pub(crate) fn set_references(&mut self, id: I, references: usize) {
        self.members
            .get_mut(&id)
            .expect("staged original member")
            .references = references;
    }
    pub(crate) fn private_surface(&self, surface: &str) -> bool {
        self.private_names
            .get_by(|stored| surface.cmp(stored.as_str()))
            .is_some()
    }
    pub(crate) fn input_scan_work(&self) -> Result<usize, StorageError> {
        N.checked_add(1).ok_or(StorageError::SizeOverflow)
    }
    fn fixed_original<D: TermDictionary<Id = I>>(
        &self,
        id: I,
        dictionary: &D,
    ) -> Result<bool, Error<D::Error>> {
        let lexical = dictionary
            .resolve_term(id)
            .map_err(DictionaryError::Source)?;
        let surface = lexical.as_ref();
        Ok(surface
            .strip_prefix('<')
            .and_then(|value| value.strip_suffix('>'))
            .is_some_and(|iri| self.plan.shadows.contains_key(iri))
            && dictionary
                .lookup_term(surface)
                .map_err(DictionaryError::Source)?
                == Some(id))
    }
    pub(crate) fn map_row<D: TermDictionary<Id = I>>(
        &self,
        original: I,
        rel: &mut RelationStore<D, N>,
    ) -> Result<Option<[I; N]>, Error<D::Error>> {
        let Some(map) = self.plan.map_surface.as_deref() else {
            return Ok(None);
        };
        let mut terms = [DictionaryTerm::Surface(&self.plan.unit_surface); N];
        terms[1] = DictionaryTerm::Surface(map);
        terms[2] = DictionaryTerm::Id(self.shadow_target(original).expect("staged target"));
        terms[Plan::original_column::<N>()] = DictionaryTerm::Id(original);
        let mut collision = false;
        let result = rel.intern_terms_checked(terms, |_, ids| {
            if self.members.get(&ids[1]).is_some()
                || self.inverse.get(&ids[1]).is_some()
                || (N == 4
                    && (self.members.get(&ids[0]).is_some() || self.inverse.get(&ids[0]).is_some()))
            {
                collision = true;
                return Err(DictionaryError::Storage(StorageError::AdmissionFailed));
            }
            Ok(ids)
        });
        if collision {
            return Err(Error::Identity);
        }
        Ok(Some(result?))
    }

    fn allow(&mut self, cost: usize, allow: &mut impl FnMut(usize) -> bool) -> bool {
        if self.unit_admitted {
            return allow(0);
        }
        if !allow(cost) {
            return false;
        }
        self.unit_admitted = true;
        true
    }
    fn complete_unit(&mut self) {
        self.unit_admitted = false;
    }

    fn member_in<A: Admission + ?Sized>(
        &mut self,
        id: I,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        if self.members.get(&id).is_none() {
            reserve_array_in(
                &mut self.order,
                memory,
                &mut self.retired_bytes,
                |order, memory| memory.reserve_for_push(order),
            )?;
            reserve_ordered_in(&mut self.members, memory, &mut self.retired_bytes)?;
            self.members.insert(
                id,
                Member {
                    references: 0,
                    pinned: false,
                    target: None,
                },
                memory,
            )?;
            self.order.push(id);
        }
        Ok(())
    }

    fn release_name<A: Admission + ?Sized>(
        &mut self,
        memory: &mut Memory<'_, A>,
    ) -> Result<(), StorageError> {
        if let Some(name) = self.pending_name.take() {
            self.retired_bytes = self
                .retired_bytes
                .checked_add(name.capacity())
                .ok_or(StorageError::SizeOverflow)?;
            drop(name);
        }
        flush_retired_bytes(&mut self.retired_bytes, memory)
    }

    pub(crate) fn mint_in<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        original: I,
        rel: &mut RelationStore<D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<bool, Error<D::Error>> {
        flush_retired_bytes(&mut self.retired_bytes, memory)?;
        if self.shadow_target(original).is_some() {
            return Ok(true);
        }
        self.member_in(original, memory)?;
        loop {
            if !self.allow(1, &mut allow) {
                return Ok(false);
            }
            reserve_ordered_in(&mut self.members, memory, &mut self.retired_bytes)?;
            reserve_ordered_in(&mut self.inverse, memory, &mut self.retired_bytes)?;
            reserve_ordered_in(&mut self.private_names, memory, &mut self.retired_bytes)?;
            if self.pending_name.is_none() {
                let lexical = rel
                    .interner()
                    .resolve_term(original)
                    .map_err(DictionaryError::Source)?;
                let surface = lexical.as_ref();
                let canonical = rel
                    .interner()
                    .lookup_term(surface)
                    .map_err(DictionaryError::Source)?;
                let fixed = surface
                    .strip_prefix('<')
                    .and_then(|value| value.strip_suffix('>'))
                    .and_then(|iri| self.plan.shadows.get(iri))
                    .filter(|_| canonical == Some(original));
                let name = construct_in(memory, &mut self.retired_bytes, |memory| {
                    if let (0, Some(fixed)) = (self.candidate, fixed) {
                        memory.format(&format_args!("<{fixed}>"))
                    } else {
                        let digest = purrdf_hash::blake3::hash(surface.as_bytes());
                        memory.format(&format_args!(
                            "<{}:shadow:{}:{}>",
                            self.plan.unit,
                            purrdf_hash::hex::Digest32::new(*digest.as_bytes()),
                            self.candidate
                        ))
                    }
                })?;
                self.pending_name = Some(name);
            }
            let name = self.pending_name.as_deref().expect("retained candidate");
            if self.private_surface(name) {
                self.candidate = self
                    .candidate
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                self.complete_unit();
                self.release_name(memory)?;
                continue;
            }
            if self.pending_target.is_none() {
                if let Some(existing) = rel
                    .interner()
                    .lookup_term(name)
                    .map_err(DictionaryError::Source)?
                {
                    if self.members.get(&existing).is_some()
                        || self.internal_ids.get(&existing).is_some()
                    {
                        return Err(Error::Identity);
                    }
                    if self.inverse.get(&existing) != Some(&original) {
                        self.candidate = self
                            .candidate
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        self.complete_unit();
                        self.release_name(memory)?;
                        continue;
                    }
                    self.pending_target = Some(existing);
                } else {
                    let mut identity_collision = false;
                    let result =
                        rel.intern_terms_checked([DictionaryTerm::Surface(name); N], |_, ids| {
                            let id = ids[0];
                            if self.members.get(&id).is_some()
                                || self.internal_ids.get(&id).is_some()
                                || self
                                    .inverse
                                    .get(&id)
                                    .is_some_and(|owner| *owner != original)
                            {
                                identity_collision = true;
                                // Refuse inside the dictionary transaction so all
                                // identities minted by this factory are rolled back.
                                return Err(DictionaryError::Storage(
                                    StorageError::AdmissionFailed,
                                ));
                            }
                            Ok(id)
                        });
                    if identity_collision {
                        return Err(Error::Identity);
                    }
                    self.pending_target = Some(result?);
                }
            }
            let target = self.pending_target.expect("retained target identity");
            if self
                .inverse
                .get(&target)
                .is_some_and(|owner| *owner != original)
            {
                self.pending_target = None;
                self.candidate = self
                    .candidate
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                self.complete_unit();
                self.release_name(memory)?;
                continue;
            }
            self.inverse.insert(target, original, memory)?;
            let previous = self.private_names.insert(
                self.pending_name.take().expect("minted private name"),
                (),
                memory,
            )?;
            assert!(previous.is_none(), "a minted private name has one owner");
            let member = self
                .members
                .get_mut(&original)
                .expect("original identity retained");
            member.target = Some(target);
            self.pending_target = None;
            self.candidate = 0;
            self.complete_unit();
            return Ok(true);
        }
    }

    fn next_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.cursor = 0;
        self.coordinate = 0;
        self.current = None;
        self.complete_unit();
    }

    /// Resume source capture and private-row publication through the same native
    /// factories used by original input and maintained transaction insertion.
    pub(crate) fn resume<D: TermDictionary<Id = I>, A: Admission + ?Sized>(
        &mut self,
        rel: &mut RelationStore<D, N>,
        memory: &mut Memory<'_, A>,
        mut allow: impl FnMut(usize) -> bool,
    ) -> Result<Progress, Error<D::Error>> {
        flush_retired_bytes(&mut self.retired_bytes, memory)?;
        rel.interner()
            .check_read()
            .map_err(DictionaryError::Source)?;
        loop {
            match self.phase {
                Phase::Metadata => {
                    if !self.plan.needs_projection() {
                        self.next_phase(Phase::Complete);
                    } else {
                        if !self.metadata.resume(rel, memory, &mut allow)? {
                            return Ok(Progress::Paused);
                        }
                        self.next_phase(Phase::Collect);
                    }
                }
                Phase::Collect => {
                    if self.current.is_none() {
                        self.current = rel
                            .rows_from(self.cursor)
                            .next()
                            .filter(|(row, _)| row.index() < self.original_slots);
                        if self.current.is_none() {
                            self.next_phase(Phase::Constants);
                            continue;
                        }
                    }
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let (row, terms) = self.current.expect("retained original source row");
                    let id = terms[self.coordinate];
                    if self.plan.map().is_some() {
                        self.member_in(id, memory)?;
                        let member = self.members.get_mut(&id).expect("admitted original member");
                        member.references = member
                            .references
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                    } else if self.coordinate == 1 {
                        let lexical = rel
                            .interner()
                            .resolve_term(id)
                            .map_err(DictionaryError::Source)?;
                        let surface = lexical.as_ref();
                        if surface
                            .strip_prefix('<')
                            .and_then(|value| value.strip_suffix('>'))
                            .is_some_and(|iri| self.plan.shadows.contains_key(iri))
                            && rel
                                .interner()
                                .lookup_term(surface)
                                .map_err(DictionaryError::Source)?
                                == Some(id)
                        {
                            self.member_in(id, memory)?;
                            self.members
                                .get_mut(&id)
                                .expect("admitted fixed member")
                                .references += 1;
                        }
                    }
                    self.coordinate += 1;
                    if self.coordinate == N {
                        self.cursor = row.index() + 1;
                        self.coordinate = 0;
                        self.current = None;
                    }
                    self.complete_unit();
                }
                Phase::Constants => {
                    if self.plan.map().is_none() || self.constant == self.plan.constants.len() {
                        self.sort = Some(HeapSortFrame::new(self.order.len()));
                        self.next_phase(Phase::Sort);
                        continue;
                    }
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    if self.pending_name.is_none() {
                        self.pending_name =
                            construct_in(memory, &mut self.retired_bytes, |memory| {
                                self.plan.constants[self.constant].surface_in(memory)
                            })?;
                    }
                    if self.pending_target.is_none() {
                        let surface = self
                            .pending_name
                            .as_deref()
                            .expect("authored constant surface");
                        self.pending_target =
                            Some(rel.intern_terms([DictionaryTerm::Surface(surface); N])?[0]);
                    }
                    let id = self
                        .pending_target
                        .expect("retained original constant identity");
                    self.member_in(id, memory)?;
                    self.members
                        .get_mut(&id)
                        .expect("admitted constant member")
                        .pinned = true;
                    self.pending_target = None;
                    self.constant += 1;
                    self.complete_unit();
                    self.release_name(memory)?;
                    return Ok(Progress::Advanced);
                }
                Phase::Sort => {
                    if !self
                        .sort
                        .as_mut()
                        .expect("canonical member ordering")
                        .resume(
                            &mut self.order,
                            |a, b| {
                                rel.interner()
                                    .compare_terms(DictionaryTerm::Id(*a), DictionaryTerm::Id(*b))
                                    .map(|order| order.then_with(|| a.cmp(b)))
                            },
                            || allow(1),
                        )
                        .map_err(DictionaryError::Source)?
                    {
                        return Ok(Progress::Paused);
                    }
                    self.sort = None;
                    self.next_phase(Phase::Internal);
                }
                Phase::Internal => {
                    if self.internal_cursor == self.plan.internal_order.len() {
                        self.next_phase(Phase::Members);
                        continue;
                    }
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let name = &self.plan.internal_order[self.internal_cursor];
                    reserve_ordered_in(&mut self.internal_ids, memory, &mut self.retired_bytes)?;
                    if self.pending_name.is_none() {
                        self.pending_name =
                            Some(construct_in(memory, &mut self.retired_bytes, |memory| {
                                memory.format(&format_args!("<{name}>"))
                            })?);
                    }
                    if self.pending_target.is_none() {
                        let mut collision = false;
                        let result = rel.intern_terms_checked(
                            [DictionaryTerm::Surface(
                                self.pending_name
                                    .as_deref()
                                    .expect("static predicate surface"),
                            ); N],
                            |_, ids| {
                                let id = ids[0];
                                if self.members.get(&id).is_some()
                                    || self.inverse.get(&id).is_some()
                                    || self.internal_ids.get(&id).is_some()
                                {
                                    collision = true;
                                    return Err(DictionaryError::Storage(
                                        StorageError::AdmissionFailed,
                                    ));
                                }
                                Ok(id)
                            },
                        );
                        if collision {
                            return Err(Error::Identity);
                        }
                        self.pending_target = Some(result?);
                    }
                    self.internal_ids.insert(
                        self.pending_target
                            .take()
                            .expect("static predicate identity"),
                        (),
                        memory,
                    )?;
                    self.internal_cursor += 1;
                    self.complete_unit();
                    self.release_name(memory)?;
                    return Ok(Progress::Advanced);
                }
                Phase::Members => {
                    let Some(&original) = self.order.get(self.member) else {
                        self.next_phase(Phase::Mirrors);
                        continue;
                    };
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    if !self.mint_in(original, rel, memory, &mut allow)? {
                        return Ok(Progress::Paused);
                    }
                    self.member += 1;
                    if self.plan.map().is_some() {
                        self.publishing = Some(original);
                        self.phase = Phase::PublishMap;
                    }
                    return Ok(Progress::Advanced);
                }
                Phase::PublishMap => {
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    let original = self.publishing.expect("retained map publication");
                    if self.pending_map.is_none() {
                        self.pending_map = self.map_row(original, rel)?;
                        return Ok(Progress::Advanced);
                    }
                    if let Some(terms) = self.pending_map {
                        let inserted = match rel.insert_terms_until_in(
                            terms.map(DictionaryTerm::Id),
                            memory,
                            &mut allow,
                        )? {
                            std::ops::ControlFlow::Break(()) => return Ok(Progress::Paused),
                            std::ops::ControlFlow::Continue(inserted) => inserted,
                        };
                        self.publishing = None;
                        self.pending_map = None;
                        self.phase = Phase::Members;
                        self.complete_unit();
                        if let Some((_, _, row)) = inserted {
                            self.added_rows += 1;
                            return Ok(Progress::Inserted(row));
                        }
                    }
                }
                Phase::Mirrors => {
                    if !self.plan.shadow_input {
                        self.next_phase(Phase::Complete);
                        continue;
                    }
                    let Some((row, mut terms)) = rel
                        .rows_from(self.cursor)
                        .next()
                        .filter(|(row, _)| row.index() < self.original_slots)
                    else {
                        self.next_phase(Phase::Complete);
                        continue;
                    };
                    if !self.allow(1, &mut allow) {
                        return Ok(Progress::Paused);
                    }
                    if let Some(target) = self.shadow_target(terms[1]) {
                        terms[1] = target;
                        let inserted = match rel.insert_terms_until_in(
                            terms.map(DictionaryTerm::Id),
                            memory,
                            &mut allow,
                        )? {
                            std::ops::ControlFlow::Break(()) => return Ok(Progress::Paused),
                            std::ops::ControlFlow::Continue(inserted) => inserted,
                        };
                        self.cursor = row.index() + 1;
                        self.complete_unit();
                        if let Some((_, _, row)) = inserted {
                            self.added_rows += 1;
                            return Ok(Progress::Inserted(row));
                        }
                    } else {
                        self.cursor = row.index() + 1;
                        self.complete_unit();
                    }
                }
                Phase::Complete => {
                    self.added_terms = rel.interner().term_count() - self.original_terms;
                    return Ok(Progress::Complete);
                }
            }
        }
    }
}

impl Plan {
    pub(super) fn new(
        map: Option<String>,
        shadows: BTreeMap<String, String>,
        shadow_input: bool,
        internal: BTreeSet<String>,
        constants: Vec<ClauseTerm>,
        unit: String,
        (authored_variables, generated_variables): (BTreeSet<String>, BTreeSet<String>),
    ) -> Self {
        Self {
            map_surface: map.as_ref().map(|map| format!("<{map}>")),
            unit_surface: format!("<{unit}>"),
            internal_order: internal.iter().cloned().collect(),
            map,
            shadows,
            shadow_input,
            internal,
            constants,
            unit,
            authored_variables,
            generated_variables,
        }
    }

    pub(crate) fn map(&self) -> Option<&str> {
        self.map.as_deref()
    }
    pub(crate) fn needs_projection(&self) -> bool {
        self.map.is_some() || self.shadow_input
    }
    pub(crate) fn unit(&self) -> &str {
        &self.unit
    }
    pub(crate) fn is_shadow(&self, iri: &str) -> bool {
        self.shadows.values().any(|name| name == iri)
    }
    pub(crate) fn is_internal(&self, iri: &str) -> bool {
        self.internal.contains(iri)
    }
    pub(crate) const fn original_column<const N: usize>() -> usize {
        if N == 4 { 3 } else { 0 }
    }
    pub(crate) fn variables_are_fresh(&self) -> bool {
        self.authored_variables
            .is_disjoint(&self.generated_variables)
    }
    pub(crate) fn generated_variable(&self, name: &str) -> bool {
        self.generated_variables.contains(name)
    }
    pub(crate) fn domains_are_disjoint(&self) -> bool {
        let shadows: BTreeSet<_> = self.shadows.values().collect();
        shadows.len() == self.shadows.len()
            && self
                .shadows
                .keys()
                .all(|name| !self.internal.contains(name) && !shadows.contains(name))
            && shadows.iter().all(|name| !self.internal.contains(*name))
            && self
                .map
                .as_ref()
                .is_none_or(|map| self.internal.contains(map))
            && self.internal.contains(&self.unit)
            && self
                .constants
                .iter()
                .filter_map(ClauseTerm::iri_value)
                .all(|name| {
                    !self.internal.contains(name)
                        && !shadows.iter().any(|shadow| shadow.as_str() == name)
                })
    }
    pub(crate) fn knows_variable(&self, name: &str) -> bool {
        self.authored_variables.contains(name) || self.generated_variables.contains(name)
    }
}

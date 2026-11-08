// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original lexical identities with explicit permanent or reclaiming ownership.

use super::{
    DictionaryError, DictionaryTerm, Memory, Resident, StorageError, TermDictionary, TermId,
    TermInterner, surface_hash, try_map_row,
};
use core::fmt;
use purrdf_lex::allocation::OrderedTable;

#[derive(Debug)]
pub(super) enum TermArena {
    Dense(Vec<String>),
    Sparse {
        values: OrderedTable<TermId, String>,
        next: u64,
    },
}

impl Default for TermArena {
    fn default() -> Self {
        Self::Dense(Vec::new())
    }
}

impl Clone for TermArena {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("cloning lexical ownership requires physical storage")
    }
}

impl TermArena {
    fn reclaiming() -> Self {
        Self::Sparse {
            values: OrderedTable::default(),
            next: 0,
        }
    }
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Dense(values) => values.len(),
            Self::Sparse { values, .. } => values.len(),
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(super) fn next(&self) -> u64 {
        match self {
            Self::Dense(values) => values.len() as u64,
            Self::Sparse { next, .. } => *next,
        }
    }
    pub(super) fn get(&self, id: TermId) -> Option<&String> {
        match self {
            Self::Dense(values) => values.get(id.address().ok()?),
            Self::Sparse { values, .. } => values.get(&id),
        }
    }
    pub(super) fn reserve(&mut self) -> Result<(), StorageError> {
        match self {
            Self::Dense(values) => values
                .try_reserve(1)
                .map_err(|_| StorageError::AllocationFailed),
            Self::Sparse { values, .. } => {
                let bytes = values.storage_bytes()?;
                values.reserve_insert(&mut Memory::resume(&mut Resident, bytes))
            }
        }
    }
    pub(super) fn publish(&mut self, id: TermId, value: String) {
        assert_eq!(
            id.index(),
            self.next(),
            "only the original checked mint publishes a term"
        );
        match self {
            Self::Dense(values) => values.push(value),
            Self::Sparse { values, next } => {
                let bytes = values
                    .storage_bytes()
                    .expect("admitted lexical arena layout");
                assert!(
                    values
                        .insert(id, value, &mut Memory::resume(&mut Resident, bytes))
                        .expect("lexical metadata was reserved before publication")
                        .is_none()
                );
                *next += 1;
            }
        }
    }
    pub(super) fn pop(&mut self, id: TermId) {
        match self {
            Self::Dense(values) => {
                values.pop().expect("rolling back a newly minted term");
            }
            Self::Sparse { values, next } => {
                values
                    .remove(&id)
                    .expect("only unpublished contiguous terms are rolled back");
                *next -= 1;
            }
        }
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = Result<(TermId, &String), StorageError>> {
        let dense = match self {
            Self::Dense(values) => Some(values.iter().enumerate()),
            Self::Sparse { .. } => None,
        };
        let sparse = match self {
            Self::Sparse { values, .. } => Some(values.ordered()),
            Self::Dense(_) => None,
        };
        dense
            .into_iter()
            .flatten()
            .map(|(index, value)| TermId::from_address(index).map(|id| (id, value)))
            .chain(
                sparse
                    .into_iter()
                    .flatten()
                    .map(|(&id, value)| Ok((id, value))),
            )
    }
    pub(super) fn into_reclaiming(self) -> Result<Self, StorageError> {
        let Self::Dense(values) = self else {
            return Ok(self);
        };
        let next = values.len() as u64;
        let mut owned = OrderedTable::default();
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for (index, value) in values.into_iter().enumerate() {
            let id = TermId::from_address(index)?;
            assert!(owned.insert(id, value, &mut memory)?.is_none());
        }
        Ok(Self::Sparse {
            values: owned,
            next,
        })
    }

    pub(super) fn try_clone(&self) -> Result<Self, StorageError> {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        match self {
            Self::Dense(values) => {
                let mut copy = Vec::new();
                memory.reserve(&mut copy, values.len())?;
                for value in values {
                    copy.push(memory.string(value)?);
                }
                Ok(Self::Dense(copy))
            }
            Self::Sparse { values, next } => {
                let mut copy = OrderedTable::default();
                for (&id, value) in values.ordered() {
                    let value = memory.string(value)?;
                    assert!(copy.insert(id, value, &mut memory)?.is_none());
                }
                Ok(Self::Sparse {
                    values: copy,
                    next: *next,
                })
            }
        }
    }
}

/// A reclaiming dictionary no longer owns the supplied original term identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermExpired {
    /// Original logical identity; it is never reassigned to another spelling.
    pub id: TermId,
}
impl fmt::Display for TermExpired {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "lexical term {:?} is not retained by this dictionary",
            self.id
        )
    }
}
impl std::error::Error for TermExpired {}
impl purrdf_hash::error::StructuredError for TermExpired {
    purrdf_hash::error_visit! {
        @record self { id } => "datalog-term-expired" { "id" => Unsigned(u128::from(self.id.index())) }
    }
}

/// Lexical ownership whose unused identities may be explicitly retired.
///
/// The permanent TermInterner keeps its original lifetime contract. This owner
/// shares the same interning implementation but stores only retained spellings.
/// Retirement is exclusive: its caller must first release every semantic use.
/// Raw IDs do not pin terms and an expired ID returns TermExpired on resolution.
#[derive(Debug, Clone)]
pub struct ReclaimingInterner {
    inner: TermInterner,
}
impl Default for ReclaimingInterner {
    fn default() -> Self {
        Self {
            inner: TermInterner {
                surfaces: TermArena::reclaiming(),
                ..TermInterner::default()
            },
        }
    }
}
impl ReclaimingInterner {
    /// Adopt an original permanent dictionary without copying spellings or IDs.
    ///
    /// # Errors
    /// Returns the original physical metadata allocation failure.
    pub fn from_permanent(mut inner: TermInterner) -> Result<Self, StorageError> {
        inner.surfaces = core::mem::take(&mut inner.surfaces).into_reclaiming()?;
        Ok(Self { inner })
    }

    /// Retire a term after its last semantic use has been released.
    ///
    /// # Errors
    /// Refuses an identity that this owner no longer retains.
    pub fn retire(&mut self, id: TermId) -> Result<(), TermExpired> {
        let value = self.inner.surfaces.get(id).ok_or(TermExpired { id })?;
        let bytes = value.len();
        self.inner
            .by_surface
            .find_entry(surface_hash(value), |candidate| *candidate == id)
            .expect("a retained lexical value has its original index entry")
            .remove();
        let TermArena::Sparse { values, .. } = &mut self.inner.surfaces else {
            unreachable!("reclaiming ownership always uses removable lexical metadata");
        };
        values.remove(&id).expect("the original retained value");
        self.inner.bytes -= bytes;
        Ok(())
    }
}

pub(super) trait InternOwner: TermDictionary<Id = TermId> {
    fn core(&self) -> &TermInterner;
    fn core_mut(&mut self) -> &mut TermInterner;
    fn validate_id(&self, id: TermId) -> Result<(), Self::Error>;
}

impl InternOwner for TermInterner {
    fn core(&self) -> &TermInterner {
        self
    }
    fn core_mut(&mut self) -> &mut TermInterner {
        self
    }
    fn validate_id(&self, id: TermId) -> Result<(), Self::Error> {
        let _ = self.resolve(id);
        Ok(())
    }
}
impl InternOwner for ReclaimingInterner {
    fn core(&self) -> &TermInterner {
        &self.inner
    }
    fn core_mut(&mut self) -> &mut TermInterner {
        &mut self.inner
    }
    fn validate_id(&self, id: TermId) -> Result<(), TermExpired> {
        self.inner
            .surfaces
            .get(id)
            .map(|_| ())
            .ok_or(TermExpired { id })
    }
}

pub(super) fn intern_transaction<O: InternOwner, T, const N: usize>(
    owner: &mut O,
    terms: [DictionaryTerm<'_, TermId>; N],
    validate: impl FnOnce(&O, [TermId; N]) -> Result<T, DictionaryError<O::Error>>,
) -> Result<T, DictionaryError<O::Error>> {
    let checkpoint = owner.core().surfaces.next();
    let result = (|| {
        let ids = try_map_row(terms, |term| match term {
            DictionaryTerm::Id(id) => {
                owner.validate_id(id).map_err(DictionaryError::Source)?;
                Ok(id)
            }
            DictionaryTerm::Surface(surface) => owner
                .core_mut()
                .intern(surface)
                .map_err(DictionaryError::Storage),
        })?;
        validate(owner, ids)
    })();
    if result.is_err() {
        owner
            .core_mut()
            .rollback_to(checkpoint)
            .map_err(DictionaryError::Storage)?;
    }
    result
}

impl TermDictionary for ReclaimingInterner {
    type Id = TermId;
    type Error = TermExpired;
    type Lexical<'a> = &'a str;
    fn lookup_term(&self, surface: &str) -> Result<Option<TermId>, TermExpired> {
        Ok(self.inner.lookup(surface))
    }
    fn resolve_term(&self, id: TermId) -> Result<&str, TermExpired> {
        self.inner
            .surfaces
            .get(id)
            .map(String::as_str)
            .ok_or(TermExpired { id })
    }
    fn compare_terms(
        &self,
        a: DictionaryTerm<'_, TermId>,
        b: DictionaryTerm<'_, TermId>,
    ) -> Result<core::cmp::Ordering, TermExpired> {
        crate::dictionary::compare_lexical_terms(self, a, b)
    }
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, TermId>; N],
        validate: impl FnOnce(&Self, [TermId; N]) -> Result<T, DictionaryError<TermExpired>>,
    ) -> Result<T, DictionaryError<TermExpired>> {
        intern_transaction(self, terms, validate)
    }
    fn check_read(&self) -> Result<(), TermExpired> {
        Ok(())
    }
    fn term_count(&self) -> u64 {
        self.inner.len() as u64
    }
    fn resident_bytes(&self) -> usize {
        self.inner.byte_len()
    }
}

impl crate::dictionary::ReclaimDictionary for ReclaimingInterner {
    fn owned_terms(&self) -> impl Iterator<Item = TermId> + '_ {
        let TermArena::Sparse { values, .. } = &self.inner.surfaces else {
            unreachable!("reclaiming ownership always uses removable lexical metadata");
        };
        values.ordered().map(|(&id, _)| id)
    }

    fn owned_term_scan_work(&self) -> Result<usize, StorageError> {
        let TermArena::Sparse { values, .. } = &self.inner.surfaces else {
            unreachable!("reclaiming ownership always uses removable lexical metadata");
        };
        values
            .storage_bytes()?
            .checked_add(1)
            .ok_or(StorageError::SizeOverflow)
    }

    fn release_term(&mut self, id: TermId) -> Result<(), DictionaryError<TermExpired>> {
        self.retire(id).map_err(DictionaryError::Source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_lexical_churn_reuses_storage_without_reusing_identities() {
        let mut dictionary = ReclaimingInterner::default();
        let [anchor] = dictionary
            .with_interned([DictionaryTerm::Surface("anchor")], |_, ids| Ok(ids))
            .unwrap();
        let mut previous = anchor;
        let mut capacity = None;
        for index in 0..4096 {
            let surface = format!("fresh-{index}");
            let [id] = dictionary
                .with_interned([DictionaryTerm::Surface(&surface)], |_, ids| Ok(ids))
                .unwrap();
            assert!(id > previous);
            previous = id;
            assert_eq!(dictionary.resolve_term(id).unwrap(), surface);
            dictionary.retire(id).unwrap();
            assert_eq!(dictionary.resolve_term(id), Err(TermExpired { id }));
            assert_eq!(dictionary.lookup_term(&surface).unwrap(), None);
            assert_eq!(dictionary.term_count(), 1);
            assert_eq!(dictionary.resident_bytes(), "anchor".len());
            assert_eq!(dictionary.resolve_term(anchor).unwrap(), "anchor");
            let TermArena::Sparse { values, .. } = &dictionary.inner.surfaces else {
                unreachable!()
            };
            let actual = values.storage_bytes().unwrap();
            assert_eq!(
                *capacity.get_or_insert(actual),
                actual,
                "physical metadata follows concurrent ownership"
            );
        }
        let mut copy = dictionary.clone();
        assert_eq!(copy.resolve_term(anchor).unwrap(), "anchor");
        assert_eq!(
            copy.resolve_term(previous),
            Err(TermExpired { id: previous })
        );
        let [next] = copy
            .with_interned([DictionaryTerm::Surface("after-clone")], |_, ids| Ok(ids))
            .unwrap();
        assert!(
            next > previous,
            "cloning preserves the original logical mint extent"
        );
    }

    #[test]
    fn expired_input_and_refused_publication_leave_no_new_lexical_payload() {
        let mut dictionary = ReclaimingInterner::default();
        let [old] = dictionary
            .with_interned([DictionaryTerm::Surface("old")], |_, ids| Ok(ids))
            .unwrap();
        dictionary.retire(old).unwrap();
        let result = dictionary.with_interned(
            [DictionaryTerm::Surface("new"), DictionaryTerm::Id(old)],
            |_, ids| Ok(ids),
        );
        assert!(matches!(result, Err(DictionaryError::Source(TermExpired { id })) if id == old));
        assert_eq!(dictionary.term_count(), 0);
        assert_eq!(dictionary.lookup_term("new").unwrap(), None);
        let result: Result<(), _> = dictionary
            .with_interned([DictionaryTerm::Surface("refused")], |_, _| {
                Err(DictionaryError::Storage(StorageError::AdmissionFailed))
            });
        assert!(matches!(
            result,
            Err(DictionaryError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(dictionary.lookup_term("refused").unwrap(), None);
        assert_eq!(dictionary.resident_bytes(), 0);
    }
}

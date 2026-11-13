// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Query-owned immutable parsed values and their admitted lookup table.

use purrdf_core::small::Shared;
use std::hash::Hash;
use std::ops::Deref;

use purrdf_xsd::{XsdDatatype, XsdValue};

use crate::{EvalError, WorkspaceAllocation, WorkspaceCapability};

#[derive(Debug)]
struct ParsedPayload {
    // Values die before their admission, including the last shared cache view.
    value: XsdValue,
    _allocation: WorkspaceAllocation,
}

#[derive(Debug)]
enum ParsedStorage {
    // Constructed only when the value owns no heap allocation.
    Inline(XsdValue),
    Shared(Shared<ParsedPayload>),
}

/// No raw extraction or independent lease clone is exposed. Numeric operators
/// borrow the value while this carrier keeps its original capacity admitted.
#[derive(Debug)]
pub(crate) struct ParsedValue {
    storage: ParsedStorage,
}

impl std::borrow::Borrow<XsdValue> for ParsedValue {
    fn borrow(&self) -> &XsdValue {
        self
    }
}

impl Clone for ParsedValue {
    fn clone(&self) -> Self {
        Self {
            storage: match &self.storage {
                ParsedStorage::Inline(value) => ParsedStorage::Inline(value.clone()),
                ParsedStorage::Shared(payload) => ParsedStorage::Shared(payload.clone()),
            },
        }
    }
}

impl Deref for ParsedValue {
    type Target = XsdValue;

    fn deref(&self) -> &Self::Target {
        match &self.storage {
            ParsedStorage::Inline(value) => value,
            ParsedStorage::Shared(payload) => &payload.value,
        }
    }
}

impl ParsedValue {
    /// Retain a native result whose actual payload was already admitted by its
    /// operation frame. This factory admits only its newly needed shared control.
    pub(crate) fn from_admitted(
        value: XsdValue,
        mut allocation: Option<WorkspaceAllocation>,
        admitted_bytes: usize,
    ) -> Result<Self, EvalError> {
        let Some(bytes) = value.owned_heap_bytes() else {
            drop(value);
            return Err(EvalError::WorkspaceBoundOverflow);
        };
        if bytes > admitted_bytes {
            drop(value);
            return Err(EvalError::WorkspaceBoundOverflow);
        }
        if bytes == 0 {
            drop(allocation);
            return Ok(Self {
                storage: ParsedStorage::Inline(value),
            });
        }
        let Some(mut allocation) = allocation.take() else {
            drop(value);
            return Err(EvalError::WorkspaceBoundOverflow);
        };
        let header = Shared::<ParsedPayload>::allocation_layout().size();
        let Some(retained) = bytes
            .checked_add(header)
            .and_then(|bytes| u64::try_from(bytes).ok())
        else {
            drop(value);
            return Err(EvalError::WorkspaceBoundOverflow);
        };
        if let Err(error) = allocation.resize(retained) {
            drop(value);
            return Err(error);
        }
        // The original frame grant now covers the retained payload plus control.
        // Neither raw extraction nor independent admission replacement is exposed.
        let payload = Shared::try_new(ParsedPayload {
            value,
            _allocation: allocation,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "computed numeric shared control",
        })?;
        Ok(Self {
            storage: ParsedStorage::Shared(payload),
        })
    }

    pub(crate) fn parse(
        lexical: &str,
        datatype: XsdDatatype,
        xsd10: bool,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self>, EvalError> {
        Ok(Self::parse_coded(lexical, datatype, xsd10, workspace)?.ok())
    }

    pub(crate) fn parse_coded(
        lexical: &str,
        datatype: XsdDatatype,
        xsd10: bool,
        workspace: &WorkspaceCapability,
    ) -> Result<Result<Self, Option<purrdf_xsd::ErrorCode>>, EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let (parsed, live) = {
            let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
            let parsed =
                purrdf_xsd::value::try_parse_with_memory(lexical, datatype, xsd10, &mut memory)
                    .map_err(|error| {
                        memory
                            .admission_mut()
                            .storage_error(error, "XSD parse destination")
                    })?;
            (parsed, memory.admitted_bytes())
        };
        match parsed {
            purrdf_xsd::value::ParsedValue::Invalid(code) => Ok(Err(code)),
            purrdf_xsd::value::ParsedValue::Value(value) => {
                Self::from_admitted(value, frame.into_allocation(), live).map(Ok)
            }
        }
    }
}

/// Table capacity belongs to the cache; individual value carriers own their
/// parsed payload. Old and replacement tables stay admitted during growth.
#[derive(Debug)]
pub(crate) struct ParsedCache<K> {
    entries: crate::workspace::AdmittedMap<K, Option<ParsedValue>>,
}

// Deriving Default would impose K: Default, though an empty table needs no key.
impl<K> Default for ParsedCache<K> {
    fn default() -> Self {
        Self {
            entries: crate::workspace::AdmittedMap::default(),
        }
    }
}

/// A present cache entry, preserving invalid/opaque values separately from a miss.
#[derive(Debug)]
pub(crate) enum ParsedCacheHit {
    Invalid,
    Value(ParsedValue),
}

impl ParsedCacheHit {
    pub(crate) fn into_value(self) -> Option<ParsedValue> {
        match self {
            Self::Invalid => None,
            Self::Value(value) => Some(value),
        }
    }
}

impl<K: Copy + Eq + Hash> ParsedCache<K> {
    pub(crate) fn get(&self, key: K) -> Option<ParsedCacheHit> {
        self.entries.get(&key).map(|entry| match entry {
            None => ParsedCacheHit::Invalid,
            Some(value) => ParsedCacheHit::Value(value.clone()),
        })
    }

    /// Whether the table already owns storage, so a further entry shares it instead of
    /// being the allocation that creates it.
    pub(crate) fn is_allocated(&self) -> bool {
        self.entries.capacity() > 0
    }

    pub(crate) fn insert_admitted(
        &mut self,
        key: K,
        value: Option<ParsedValue>,
        workspace: &WorkspaceCapability,
    ) -> Result<(), EvalError> {
        if self.entries.get(&key).is_some() {
            return Ok(());
        }
        let _ = self.entries.insert_admitted(key, value, workspace)?;
        Ok(())
    }
}

/// The key of the per-query parse memo: a dataset term, or a scratch constant a
/// program registered with [`ConstantParses`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ParseKey<I> {
    Dataset(I),
    Constant(crate::scratch::ScratchId),
}

/// Which scratch values are constant operands whose parse is worth remembering, on
/// the unbounded evaluation path only.
///
/// A constant operand is interned once per program, yet every row that compares
/// against it would parse the same lexical form again, because a computed scratch
/// value is not a dataset term. The scratch table is append-only within a query, so
/// a scratch id names one immutable value and its parse is a pure function of the
/// id. Only registered ids are remembered (in the same memo as dataset terms, under
/// [`ParseKey::Constant`]), so the memo never holds one entry per computed row value.
///
/// This is a few inline ids and nothing else, because it lives in the evaluation
/// context and that context sits in every nested level's stack frame: a larger one
/// makes a nested `EXISTS` refuse on a small stack. A program registers its
/// constants from its second row on, so a query that evaluates one row allocates
/// nothing for the memo.
///
/// A bounded evaluation registers nothing: its parse is admitted and released on
/// every use, and that accounting stays exactly as it was.
#[derive(Debug)]
pub(crate) struct ConstantParses {
    ids: [u32; CONSTANT_PARSE_SLOTS],
}

/// Constants one query keeps a parse for: the expression constants of its filters (the
/// two boolean terms a comparison yields are recognised without a slot). A constant past the last is
/// parsed per use, as it always was.
const CONSTANT_PARSE_SLOTS: usize = 2;

impl Default for ConstantParses {
    fn default() -> Self {
        Self {
            ids: [u32::MAX; CONSTANT_PARSE_SLOTS],
        }
    }
}

impl ConstantParses {
    fn raw(id: crate::scratch::ScratchId) -> u32 {
        u32::try_from(id.index()).unwrap_or(u32::MAX - 1)
    }

    /// Record `id` as a constant operand. Does nothing on a bounded path.
    pub(crate) fn register(
        &mut self,
        id: crate::scratch::ScratchId,
        workspace: &WorkspaceCapability,
    ) {
        let raw = Self::raw(id);
        if workspace.is_bounded() || self.ids.contains(&raw) {
            return;
        }
        if let Some(slot) = self.ids.iter_mut().find(|slot| **slot == u32::MAX) {
            *slot = raw;
        }
    }

    /// Whether `id` is a registered constant.
    pub(crate) fn contains(&self, id: crate::scratch::ScratchId) -> bool {
        self.ids.contains(&Self::raw(id))
    }
}

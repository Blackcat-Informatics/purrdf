// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable owned native composite keys, at the evaluator's parser adapter.

use crate::{EvalError, WorkspaceAllocation, WorkspaceCapability};
use purrdf_cdt::memory::ReadError;
use purrdf_cdt::{CdtDatatype, CdtValue};
use purrdf_core::small::Shared;
use std::cmp::Ordering;
use std::ops::Deref;

pub(crate) struct CompositePayload {
    value: CdtValue,
    // The original value dies before its grant, including on the last shared view.
    _allocation: WorkspaceAllocation,
}

/// Resident values remain explicit caller-owned allocations. Operational values
/// have one immutable payload and original lease; there is no raw extraction.
pub(crate) enum CompositeValue {
    Resident(CdtValue),
    Owned(Shared<CompositePayload>),
}

impl Deref for CompositeValue {
    type Target = CdtValue;
    fn deref(&self) -> &CdtValue {
        match self {
            Self::Resident(value) => value,
            Self::Owned(value) => &value.value,
        }
    }
}

impl std::fmt::Debug for CompositeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Engine diagnostics do not recursively format the composite payload.
        f.debug_struct("CompositeValue").finish_non_exhaustive()
    }
}

impl Clone for CompositeValue {
    fn clone(&self) -> Self {
        match self {
            Self::Resident(value) => Self::Resident(value.clone()),
            Self::Owned(value) => Self::Owned(value.clone()),
        }
    }
}

impl CompositeValue {
    pub(crate) fn resident(value: CdtValue) -> Self {
        Self::Resident(value)
    }

    /// Malformed lexical forms alone keep ORDER's original opaque-literal rule.
    /// Every admission, capacity and allocator failure remains operational.
    pub(crate) fn parse(
        lexical: &str,
        datatype: &str,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<Self>, EvalError> {
        let Some(datatype) = CdtDatatype::from_iri(datatype) else {
            return Ok(None);
        };
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let result = purrdf_cdt::parse::try_parse_cdt(
            lexical,
            datatype,
            &mut crate::cdt_fn::CdtStorage::new(&mut frame),
        );
        let (value, bytes) = match result {
            Ok(value) => value,
            Err(ReadError::Lexical(error)) => {
                drop(error);
                return frame.take_failure().map_or(Ok(None), Err);
            }
            Err(ReadError::Storage(error)) => {
                return Err(crate::cdt_fn::storage_error(
                    &mut frame,
                    error,
                    "native composite parse",
                ));
            }
        };
        let header = Shared::<CompositePayload>::allocation_layout().size();
        let bytes = bytes
            .checked_add(header)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        frame.resize_live(bytes)?;
        let allocation = frame
            .into_allocation()
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let value = Shared::try_new(CompositePayload {
            value,
            _allocation: allocation,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "parsed composite shared control",
        })?;
        Ok(Some(Self::Owned(value)))
    }

    pub(crate) fn total_cmp(
        &self,
        other: &Self,
        workspace: &WorkspaceCapability,
    ) -> Result<Ordering, EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let result = purrdf_cdt::ops::try_total_value_cmp(
            self,
            other,
            &mut crate::cdt_fn::CdtStorage::new(&mut frame),
        );
        result.map_err(|error| {
            crate::cdt_fn::storage_error(&mut frame, error, "native composite total order")
        })
    }
}

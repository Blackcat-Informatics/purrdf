// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Request-selected clock values retain their native coefficient admission.

use crate::workspace::LexicalFrame;
use crate::{EvalError, WorkspaceCapability};
use purrdf_lex::allocation::{Memory, SharedOwned};
use purrdf_xsd::{XsdValue, temporal::DateTime};

pub(super) fn now(
    supplied: Option<&DateTime>,
    workspace: &WorkspaceCapability,
) -> Result<XsdValue, EvalError> {
    let Some(supplied) = supplied else {
        return Ok(XsdValue::DateTime(crate::clock::wall_clock_now()));
    };
    if supplied.owned_heap_bytes() == 0 {
        return Ok(XsdValue::DateTime(supplied.clone()));
    }
    let mut frame = LexicalFrame::new(workspace);
    let prepared = supplied.detached_with_memory(&mut Memory::new(&mut frame));
    let mut value =
        prepared.map_err(|error| frame.storage_error(error, "query clock coordinate"))?;
    let admitted = frame.admitted_bytes();
    let replacement = LexicalFrame::new(workspace);
    let owner = match SharedOwned::retain_current(
        &mut frame,
        replacement,
        admitted,
        std::alloc::Layout::new::<()>(),
    ) {
        Ok((owner, _)) => owner,
        Err(error) => {
            drop(value);
            return Err(frame.storage_error(error, "query clock coordinate"));
        }
    };
    if let Err(error) = value.publish_detached(&owner) {
        drop(value);
        drop(owner);
        return Err(workspace.storage_error(error, "query clock coordinate"));
    }
    Ok(XsdValue::DateTime(value))
}

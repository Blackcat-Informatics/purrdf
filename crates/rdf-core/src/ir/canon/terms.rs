// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed result-value admission through the native RDF rules.

use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_lex::control::{ControlledError, Progress, with_memory_control};

use super::{CanonError, check_stop};
use crate::ir::builder::NativeBuildError;
use crate::{StopSignal, TermValue};

struct Poll<'a>(Option<&'a dyn StopSignal>);
impl Progress for Poll<'_> {
    type Error = CanonError;
    fn charge(&mut self, _: u64) -> Result<(), Self::Error> {
        check_stop(self.0)
    }
}

/// Admit borrowed solution values without constructing an RDF carrier.
///
/// Returns whether any value contains a scoped blank, including triple terms
/// and composite literals. Every value is structurally screened. An IRI is
/// admitted whatever its namespace: the canonical labelling reserves none. A
/// diagnostic keeps its original memory account; callers retain that admission
/// through its lifetime.
///
/// # Errors
/// Preserves native RDF, cancellation and storage refusals.
pub fn screen_comparison_terms_with_memory<'a, S: Admission + ?Sized>(
    values: impl IntoIterator<Item = &'a TermValue>,
    stop: Option<&dyn StopSignal>,
    memory: &mut Memory<'_, S>,
) -> Result<bool, CanonError> {
    let baseline = memory.admitted_bytes();
    let result = with_memory_control(memory, &mut Poll(stop), |memory| {
        let mut blanks = false;
        for value in values {
            memory.admission_mut().checkpoint(0)?;
            crate::ir::validate::validate_value_with_memory(value, memory, &mut |term, memory| {
                match term {
                    TermValue::Iri(_) | TermValue::Triple { .. } => {}
                    TermValue::Blank { .. } => blanks = true,
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        ..
                    } => {
                        let mut embedded = crate::cdt_blank::cdt_embedded_blanks_with_memory(
                            lexical_form,
                            datatype,
                            memory,
                        )?;
                        blanks |= !embedded.is_empty();
                        while let Some((label, _)) = embedded.pop() {
                            memory.release_string(label)?;
                        }
                        memory.release_vec(embedded)?;
                    }
                }
                Ok(())
            })
            .map_err(|cause| match cause {
                NativeBuildError::Diagnostic(cause) => CanonError::Invalid(cause),
                NativeBuildError::Storage(cause) => CanonError::Storage(cause),
            })?;
        }
        Ok::<_, CanonError>(blanks)
    });
    let result = result.map_err(|cause| match cause {
        ControlledError::Control(cause) => cause,
        ControlledError::Native(cause) => cause,
    });
    let retained = match &result {
        Err(CanonError::Invalid(cause)) => cause.retained_bytes(),
        _ => Some(0),
    };
    let Some(released) = retained
        .and_then(|bytes| baseline.checked_add(bytes))
        .and_then(|live| memory.admitted_bytes().checked_sub(live))
    else {
        drop(result);
        if let Some(bytes) = memory.admitted_bytes().checked_sub(baseline) {
            let _ = memory.release_bytes(bytes);
        }
        return Err(CanonError::Storage(StorageError::SizeOverflow));
    };
    let cleanup = memory.release_bytes(released);
    if result.is_ok() {
        cleanup.map_err(CanonError::Storage)?;
    }
    result
}

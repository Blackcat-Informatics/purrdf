// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Concrete query owners; all vector/string growth stays in the evaluator home.

use purrdf_sparql_eval::{EvalError, WorkspaceAllocation, WorkspaceCapability};

use crate::TextError;

/// The workspace capability's allocation-free stop classes, preserved by TEXT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapacityFailure {
    /// The shared account latched its first operational refusal.
    Stopped,
    /// An actual allocation layout cannot be represented.
    LayoutOverflow,
    /// The allocator refused a previously admitted native owner.
    Allocator {
        /// The native allocation that could not be constructed.
        construct: &'static str,
    },
    /// A diagnostic's two borrowed rendering passes differed.
    UnstableDiagnostic,
}

impl CapacityFailure {
    /// `charge`, `resize`, and native reserve helpers return only these classes.
    /// Adding another workspace failure requires extending this explicit mapping.
    fn from_workspace(error: &EvalError) -> Self {
        match error {
            EvalError::WorkspaceStopped => Self::Stopped,
            EvalError::WorkspaceBoundOverflow => Self::LayoutOverflow,
            EvalError::AllocationFailed { construct } => Self::Allocator { construct },
            EvalError::UnstableNativeDiagnostic => Self::UnstableDiagnostic,
            _ => unreachable!("native workspace helpers changed their failure contract"),
        }
    }

    pub(crate) fn into_eval(self) -> EvalError {
        match self {
            Self::Stopped => EvalError::WorkspaceStopped,
            Self::LayoutOverflow => EvalError::WorkspaceBoundOverflow,
            Self::Allocator { construct } => EvalError::AllocationFailed { construct },
            Self::UnstableDiagnostic => EvalError::UnstableNativeDiagnostic,
        }
    }
}

impl core::fmt::Display for CapacityFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.into_eval(), f)
    }
}

impl std::error::Error for CapacityFailure {}

impl purrdf_hash::error::StructuredError for CapacityFailure {
    purrdf_hash::error_visit! {
        Stopped => "text-capacity-stopped" {},
        LayoutOverflow => "text-capacity-layout-overflow" {},
        Allocator { construct } => "text-capacity-allocator" { "construct" => Text(construct) },
        UnstableDiagnostic => "text-capacity-unstable-diagnostic" {},
    }
}

pub(crate) fn admitted<T>(value: Result<T, EvalError>) -> Result<T, TextError> {
    value.map_err(|error| TextError::Capacity(CapacityFailure::from_workspace(&error)))
}

pub(crate) fn overflow() -> TextError {
    TextError::Capacity(CapacityFailure::LayoutOverflow)
}

/// The scorer's two error channels share computation and preserve its account.
pub(crate) trait TextErrorTarget: Sized {
    fn from_text(error: TextError, workspace: &WorkspaceCapability) -> Self;
}

impl TextErrorTarget for TextError {
    fn from_text(error: TextError, _workspace: &WorkspaceCapability) -> Self {
        error
    }
}

impl TextErrorTarget for EvalError {
    fn from_text(error: TextError, workspace: &WorkspaceCapability) -> Self {
        error.into_eval_in(workspace)
    }
}

/// One fresh token or generated lexical string, without a lease-dropping clone.
pub(crate) struct QueryString {
    text: String,
    allocation: Option<WorkspaceAllocation>,
}

impl QueryString {
    pub(crate) fn from_parts(text: String, allocation: Option<WorkspaceAllocation>) -> Self {
        Self { text, allocation }
    }
    pub(crate) fn copy(text: &str, workspace: &WorkspaceCapability) -> Result<Self, TextError> {
        let mut result = Self {
            text: String::new(),
            allocation: None,
        };
        admitted(workspace.reserve_string(&mut result.text, &mut result.allocation, text.len()))?;
        result.text.push_str(text);
        Ok(result)
    }

    pub(crate) fn chars(
        chars: impl Iterator<Item = char> + Clone,
        workspace: &WorkspaceCapability,
    ) -> Result<Self, TextError> {
        let bytes = chars
            .clone()
            .try_fold(0usize, |bytes, c| bytes.checked_add(c.len_utf8()))
            .ok_or_else(overflow)?;
        let mut result = Self {
            text: String::new(),
            allocation: None,
        };
        admitted(workspace.reserve_string(&mut result.text, &mut result.allocation, bytes))?;
        result.text.extend(chars);
        Ok(result)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

impl core::ops::Deref for QueryString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

impl core::fmt::Debug for QueryString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.text, f)
    }
}

pub(crate) fn query_error(
    workspace: &WorkspaceCapability,
    kind: purrdf_sparql_eval::NativeDiagnosticKind,
    message: impl core::fmt::Display,
) -> Result<TextError, TextError> {
    let diagnostic = admitted(purrdf_sparql_eval::NativeDiagnostic::render(
        kind, message, workspace,
    ))?;
    Ok(if workspace.is_bounded() {
        TextError::Diagnostic(diagnostic)
    } else {
        match kind {
            purrdf_sparql_eval::NativeDiagnosticKind::Data => TextError::Data(diagnostic),
            purrdf_sparql_eval::NativeDiagnosticKind::Config => TextError::Config(diagnostic),
            purrdf_sparql_eval::NativeDiagnosticKind::SourceRead => {
                TextError::SourceRead(diagnostic)
            }
            _ => TextError::Diagnostic(diagnostic),
        }
    })
}

pub(crate) fn data_error(
    cause: crate::data_refusal::DataRefusal,
    workspace: Option<&WorkspaceCapability>,
) -> TextError {
    let resident = WorkspaceCapability::resident();
    match admitted(purrdf_sparql_eval::NativeDiagnostic::render_cause(
        purrdf_sparql_eval::NativeDiagnosticKind::Data,
        cause,
        workspace.unwrap_or(&resident),
    )) {
        Ok(diagnostic) => {
            if workspace.is_some_and(WorkspaceCapability::is_bounded) {
                TextError::Diagnostic(diagnostic)
            } else {
                TextError::Data(diagnostic)
            }
        }
        Err(error) => error,
    }
}

pub(crate) fn arithmetic_error(
    cause: crate::arithmetic::ArithmeticRefusal,
    workspace: Option<&WorkspaceCapability>,
) -> TextError {
    let domain = cause.is_domain();
    let diagnostic_only = matches!(
        cause,
        crate::arithmetic::ArithmeticRefusal::PartitionRank { .. }
    );
    let resident = WorkspaceCapability::resident();
    match admitted(purrdf_sparql_eval::NativeDiagnostic::render_cause(
        purrdf_sparql_eval::NativeDiagnosticKind::Function,
        cause,
        workspace.unwrap_or(&resident),
    )) {
        Ok(diagnostic) => {
            if diagnostic_only || workspace.is_some_and(WorkspaceCapability::is_bounded) {
                TextError::Diagnostic(diagnostic)
            } else if domain {
                TextError::Domain(diagnostic)
            } else {
                TextError::Overflow(diagnostic)
            }
        }
        Err(error) => error,
    }
}

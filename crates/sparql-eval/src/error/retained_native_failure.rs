// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Classified native causes retained without eager presentation.

/// An original classified native cause with its original allocation admission.
/// Cloning shares the cause; inspection and construction never format it.
/// Equality compares that shared identity, independently of human presentation.
#[derive(Clone)]
pub struct RetainedNativeFailure {
    pub(super) kind: super::NativeDiagnosticKind,
    cause: purrdf_core::diagnostic::DiagnosticCause,
}

impl RetainedNativeFailure {
    pub(crate) fn new(
        kind: super::NativeDiagnosticKind,
        cause: impl purrdf_hash::error::NativeError + 'static,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, crate::EvalError> {
        crate::retained::PreparedDiagnosticCause::new(
            cause,
            crate::workspace::LexicalFrame::new(workspace),
        )?
        .publish()
        .map(|cause| Self { kind, cause })
    }

    pub(crate) fn error(
        kind: super::NativeDiagnosticKind,
        cause: impl purrdf_hash::error::NativeError + 'static,
        workspace: &crate::WorkspaceCapability,
    ) -> crate::EvalError {
        match Self::new(kind, cause, workspace) {
            Ok(failure) => crate::EvalError::RetainedNativeFailure(failure),
            Err(error) => error,
        }
    }

    /// The original classification, independent of presentation or inspection.
    #[must_use]
    pub const fn kind(&self) -> super::NativeDiagnosticKind {
        self.kind
    }

    /// Borrow the original typed cause, including its standard downcast identity.
    #[must_use]
    pub fn cause(&self) -> &(dyn purrdf_hash::error::NativeError + 'static) {
        &**self.cause
    }
}

purrdf_hash::debug_non_exhaustive!(RetainedNativeFailure {});

impl PartialEq for RetainedNativeFailure {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && core::ptr::eq(self.cause(), other.cause())
    }
}

impl Eq for RetainedNativeFailure {}

impl purrdf_hash::error::StructuredError for RetainedNativeFailure {
    purrdf_hash::error_visit! {
        @record self { kind, cause } => "eval-retained-native-failure" {
            "kind" => Record(&self.kind),
            "cause" => Cause(self.cause()),
        }
    }
}

impl core::fmt::Display for RetainedNativeFailure {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}: ", self.kind.prefix())?;
        if self.kind == super::NativeDiagnosticKind::SourceRead {
            super::write_source_message(self.cause(), formatter)
        } else {
            core::fmt::Display::fmt(self.cause(), formatter)
        }
    }
}

impl std::error::Error for RetainedNativeFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause())
    }
}

#[cfg(test)]
mod tests {
    use super::RetainedNativeFailure;
    use crate::workspace::owned_row_regressions::account;
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::{CallerRefusal, Capture};
    use std::sync::atomic::Ordering;

    #[test]
    fn retained_native_failure_preserves_function_promotion_without_formatting() {
        let workspace = crate::WorkspaceCapability::resident();
        let original = RetainedNativeFailure::error(
            crate::NativeDiagnosticKind::Function,
            CallerRefusal { limit: u128::MAX },
            &workspace,
        );
        let clone = original.clone();
        let promoted = original.preserve_function_failure();
        assert_eq!(
            promoted.diagnostic_code(),
            Some(crate::EvalError::FUNCTION_OPERATIONAL_CODE)
        );
        let crate::EvalError::RetainedNativeFailure(before) = clone else {
            unreachable!()
        };
        let crate::EvalError::RetainedNativeFailure(after) = promoted else {
            unreachable!()
        };
        assert_eq!(
            after.kind(),
            crate::NativeDiagnosticKind::FunctionOperational
        );
        assert!(core::ptr::eq(before.cause(), after.cause()));
        assert!(matches!(
            Capture::read(after.cause()).field("limit"),
            ErrorValue::Unsigned(u128::MAX)
        ));
    }

    #[test]
    fn retained_native_failure_bounds_source_presentation_without_truncating_its_cause() {
        let text = "漢字操作".repeat(5000);
        let original = purrdf_core::RdfDiagnostic::error("example-source", &text);
        let expected = crate::EvalError::source_read(&original).to_string();
        let failure = RetainedNativeFailure::new(
            crate::NativeDiagnosticKind::SourceRead,
            original,
            &crate::WorkspaceCapability::resident(),
        )
        .unwrap();
        assert_eq!(failure.to_string(), expected);
        let source = std::error::Error::source(&failure)
            .unwrap()
            .downcast_ref::<purrdf_core::RdfDiagnostic>()
            .unwrap();
        assert_eq!(source.message, text);
    }

    #[test]
    fn retained_native_failure_keeps_unformattable_cause_and_exact_original_grant() {
        let (workspace, ledger, base) = account(65_536);
        let capability = workspace.capability();
        let window = CurrentThreadWindow::open();
        let original = RetainedNativeFailure::new(
            crate::NativeDiagnosticKind::Internal,
            CallerRefusal { limit: u128::MAX },
            &capability,
        )
        .unwrap();
        let measured = window.close();
        let live = ledger.live.load(Ordering::Relaxed);
        assert_eq!(measured.retained_bytes, i64::try_from(live - base).unwrap());
        let retained = original.clone();
        assert_eq!(original, retained);
        assert!(core::ptr::eq(original.cause(), retained.cause()));
        drop(original);
        drop(capability);
        drop(workspace);
        assert_eq!(ledger.live.load(Ordering::Relaxed), live);
        let buffer = Vec::with_capacity(2);
        let window = CurrentThreadWindow::open();
        let captured = Capture::read_with_buffer(retained.cause(), buffer);
        assert_eq!(window.close().allocations, 0);
        assert!(matches!(
            captured.field("limit"),
            ErrorValue::Unsigned(u128::MAX)
        ));
        assert_eq!(
            std::error::Error::source(&retained)
                .unwrap()
                .downcast_ref::<CallerRefusal>()
                .unwrap()
                .limit,
            u128::MAX
        );
        drop(captured);
        drop(retained);
        assert_eq!(ledger.live.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn retained_native_failure_refuses_before_allocating_without_headroom() {
        let (initial, _, base) = account(65_536);
        drop(initial);
        let (workspace, ledger, _) = account(base);
        let capability = workspace.capability();
        let window = CurrentThreadWindow::open();
        let error = RetainedNativeFailure::new(
            crate::NativeDiagnosticKind::Internal,
            CallerRefusal { limit: u128::MAX },
            &capability,
        )
        .unwrap_err();
        assert_eq!(window.close().allocations, 0);
        assert!(matches!(error, crate::EvalError::WorkspaceStopped));
        assert_eq!(ledger.live.load(Ordering::Relaxed), base);
        assert_eq!(workspace.take_failure().unwrap().identity, 7);
        drop(capability);
        drop(workspace);
        assert_eq!(ledger.live.load(Ordering::Relaxed), 0);
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dynamic invariant causes retained without eager presentation.

/// An original native invariant cause with its original allocation admission.
/// Cloning shares the cause; inspection and construction never format it.
/// Equality compares that shared identity, independently of human presentation.
#[derive(Clone)]
pub struct RetainedInvariant {
    cause: purrdf_core::diagnostic::DiagnosticCause,
}

impl RetainedInvariant {
    pub(crate) fn new(
        cause: impl purrdf_hash::error::NativeError + 'static,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, crate::EvalError> {
        crate::retained::PreparedDiagnosticCause::new(
            cause,
            crate::workspace::LexicalFrame::new(workspace),
        )?
        .publish()
        .map(|cause| Self { cause })
    }

    /// Borrow the original typed cause, including its standard downcast identity.
    #[must_use]
    pub fn cause(&self) -> &(dyn purrdf_hash::error::NativeError + 'static) {
        &**self.cause
    }
}

purrdf_hash::debug_non_exhaustive!(RetainedInvariant {});

impl PartialEq for RetainedInvariant {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.cause(), other.cause())
    }
}

impl Eq for RetainedInvariant {}

impl purrdf_hash::error::StructuredError for RetainedInvariant {
    purrdf_hash::error_visit! {
        @record self { cause } => "eval-retained-invariant" {
            "cause" => Cause(self.cause()),
        }
    }
}

impl core::fmt::Display for RetainedInvariant {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.cause(), formatter)
    }
}

impl std::error::Error for RetainedInvariant {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.cause())
    }
}

#[cfg(test)]
mod tests {
    use super::RetainedInvariant;
    use crate::workspace::owned_row_regressions::account;
    use purrdf_alloc_probe::CurrentThreadWindow;
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::{CallerRefusal, Capture};
    use std::sync::atomic::Ordering;

    #[test]
    fn retained_invariant_keeps_unformattable_cause_and_exact_original_grant() {
        let (workspace, ledger, base) = account(65_536);
        let capability = workspace.capability();
        let window = CurrentThreadWindow::open();
        let original =
            RetainedInvariant::new(CallerRefusal { limit: u128::MAX }, &capability).unwrap();
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
    fn retained_invariant_refuses_before_allocating_without_headroom() {
        let (initial, _, base) = account(65_536);
        drop(initial);
        let (workspace, ledger, _) = account(base);
        let capability = workspace.capability();
        let window = CurrentThreadWindow::open();
        let error =
            RetainedInvariant::new(CallerRefusal { limit: u128::MAX }, &capability).unwrap_err();
        assert_eq!(window.close().allocations, 0);
        assert!(matches!(error, crate::EvalError::WorkspaceStopped));
        assert_eq!(ledger.live.load(Ordering::Relaxed), base);
        assert_eq!(workspace.take_failure().unwrap().identity, 7);
        drop(capability);
        drop(workspace);
        assert_eq!(ledger.live.load(Ordering::Relaxed), 0);
    }
}

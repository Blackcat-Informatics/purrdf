// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original geographic native causes, retained through native admission.

use crate::GeoError;
use crate::context::{Retained, WorkProgress};
use purrdf_hash::error::{NativeError, StructuredError};

/// An original cause and the native account owning its immutable box.
#[derive(Clone)]
pub struct RetainedGeoCause(Retained<Box<dyn NativeError>>);
purrdf_hash::debug_non_exhaustive!(RetainedGeoCause {});

impl RetainedGeoCause {
    pub(crate) fn capture(
        cause: impl NativeError + 'static,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        Retained::produce(
            progress,
            |memory| {
                let cause: Box<dyn NativeError> = memory.boxed(cause).map_err(GeoError::Storage)?;
                Ok(cause)
            },
            |_, _| Ok(()),
        )
        .map(Self)
    }

    /// Borrow the unmodified native cause without presentation or allocation.
    #[must_use]
    pub fn original(&self) -> &(dyn NativeError + 'static) {
        &**self.0
    }
}

impl StructuredError for RetainedGeoCause {
    purrdf_hash::error_visit! {
        @tuple self (_owner) => "geo-native-cause" { "cause" => Cause(self.original()) }
    }
}

impl core::fmt::Display for RetainedGeoCause {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.original(), f)
    }
}
impl std::error::Error for RetainedGeoCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.original())
    }
}
impl PartialEq for RetainedGeoCause {
    fn eq(&self, other: &Self) -> bool {
        Retained::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for RetainedGeoCause {}
impl core::hash::Hash for RetainedGeoCause {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // Equality of erased native causes follows the original shared owner.
        // This process-local identity is never an inspection field.
        core::ptr::hash(core::ptr::from_ref(&*self.0), state);
    }
}

/// An original typed geographic cause or an explicitly supplied caller message.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GeoFailureCause {
    /// The native read failure, without a rendered replacement.
    Native(RetainedGeoCause),
    /// Genuine caller message data, not a parsed native reason.
    Message(String),
}

impl StructuredError for GeoFailureCause {
    purrdf_hash::error_visit! {
        Native(cause) => "geo-cause-native" { "cause" => Cause(cause) },
        Message(message) => "geo-cause-message" { "message" => Message(message) },
    }
}

impl GeoFailureCause {
    pub(crate) fn native_text_bytes(&self) -> usize {
        match self {
            Self::Message(message) => message.capacity(),
            Self::Native(_) => 0,
        }
    }

    /// Explicit human detail. Native text is returned as a separate owned
    /// presentation; it is never cached inside the original allocation owner.
    #[must_use]
    pub fn detail(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Native(cause) => std::borrow::Cow::Owned(cause.original().to_string()),
            Self::Message(message) => std::borrow::Cow::Borrowed(message),
        }
    }
}

impl core::fmt::Display for GeoFailureCause {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Native(cause) => core::fmt::Display::fmt(cause, f),
            Self::Message(message) => f.write_str(message),
        }
    }
}
purrdf_lex::error_source!(GeoFailureCause => { Self::Native(cause) => cause });

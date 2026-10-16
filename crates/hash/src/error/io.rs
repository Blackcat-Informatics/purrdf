// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original standard I/O failures with borrowed native metadata.

use std::fmt;
use std::sync::{Arc, OnceLock};

use super::{ErrorValue, NativeError, StructuredError};

struct Payload {
    kind: std::io::ErrorKind,
    raw_os_error: Option<i32>,
    external_message: Option<String>,
    rendered: OnceLock<String>,
    original: std::io::Error,
}

struct NativeIoCause {
    cause: Box<dyn NativeError>,
}
crate::debug_non_exhaustive!(NativeIoCause {});

impl StructuredError for NativeIoCause {
    crate::error_visit! {
        @record self { cause } => "std-io-native-cause" { "cause" => Cause(&*self.cause) }
    }
}

impl fmt::Display for NativeIoCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.cause, f)
    }
}
impl std::error::Error for NativeIoCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.cause)
    }
}

/// Carry a known structured cause through the standard I/O error channel.
///
/// No formatting occurs here or when [`IoFailure`] captures this error. Native
/// fields remain inspectable after `Read`/`Write` has erased the concrete type.
/// Human presentation is rendered only when explicitly requested.
pub fn io_error(
    kind: std::io::ErrorKind,
    cause: impl std::error::Error + StructuredError + Send + Sync + 'static,
) -> std::io::Error {
    std::io::Error::new(
        kind,
        NativeIoCause {
            cause: Box::new(cause),
        },
    )
}

/// An original I/O failure, shared without cloning or replacing its opaque cause.
///
/// The native kind and OS code remain typed. A cause wrapped by [`io_error`]
/// remains structured and is never rendered by capture or inspection. An opaque
/// external I/O message is captured once for the existing string presentation API.
/// Original custom payloads remain available through [`original`](Self::original)
/// and the standard source chain, without inventing fields for an opaque type.
/// External captures compare by kind, OS code and message; typed cause owners
/// compare by identity because an erased error need not implement equality.
#[derive(Clone)]
pub struct IoFailure {
    payload: Arc<Payload>,
}

impl From<std::io::Error> for IoFailure {
    fn from(original: std::io::Error) -> Self {
        let external_message = (!original
            .get_ref()
            .is_some_and(<dyn std::error::Error + Send + Sync + 'static>::is::<NativeIoCause>))
        .then(|| original.to_string());
        Self {
            payload: Arc::new(Payload {
                kind: original.kind(),
                raw_os_error: original.raw_os_error(),
                external_message,
                rendered: OnceLock::new(),
                original,
            }),
        }
    }
}

impl IoFailure {
    /// The actual standard-library I/O classification.
    #[must_use]
    pub fn kind(&self) -> std::io::ErrorKind {
        self.payload.kind
    }

    /// The original platform-specific OS error code, when present.
    #[must_use]
    pub fn raw_os_error(&self) -> Option<i32> {
        self.payload.raw_os_error
    }

    /// Human-readable description, rendering a native cause lazily on request.
    /// Structured inspection never calls this presentation accessor.
    #[must_use]
    pub fn message(&self) -> &str {
        self.payload.external_message.as_deref().unwrap_or_else(|| {
            self.payload
                .rendered
                .get_or_init(|| self.original().to_string())
        })
    }

    /// The unmodified standard I/O error, including any opaque custom cause.
    #[must_use]
    pub fn original(&self) -> &std::io::Error {
        &self.payload.original
    }

    /// The typed bridge and original cause supplied through [`io_error`].
    #[must_use]
    pub fn native_cause(&self) -> Option<&dyn StructuredError> {
        self.original()
            .get_ref()?
            .downcast_ref::<NativeIoCause>()
            .map(|cause| cause as &dyn StructuredError)
    }
}

impl StructuredError for IoFailure {
    crate::error_visit! {
        @record self { payload } => "std-io" {
            "kind" => Record(&self.payload.kind),
            "raw_os_error" => Value(self.payload.raw_os_error.map_or(ErrorValue::Absent,
                |code| ErrorValue::Signed(i128::from(code)))),
            "message" => Value(self.payload.external_message.as_deref().map_or(ErrorValue::Absent, ErrorValue::Message)),
            "cause" => Value(self.native_cause().map_or(ErrorValue::Absent, ErrorValue::Cause)),
        }
    }
}

impl PartialEq for IoFailure {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.payload, &other.payload)
            || (self.payload.kind == other.payload.kind
                && self.payload.raw_os_error == other.payload.raw_os_error
                && self.payload.external_message.is_some()
                && self.payload.external_message == other.payload.external_message)
    }
}
impl Eq for IoFailure {}

impl fmt::Debug for IoFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IoFailure")
            .field("kind", &self.payload.kind)
            .field("raw_os_error", &self.payload.raw_os_error)
            .field("message", &self.payload.external_message)
            .finish_non_exhaustive()
    }
}

impl fmt::Display for IoFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for IoFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.original())
    }
}

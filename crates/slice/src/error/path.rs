// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original filesystem and IRI operands of slice path refusals.

use std::path::PathBuf;

/// A path rejected before slice discovery or retrieval.
#[derive(Debug)]
pub enum PathRefusal {
    /// Canonicalizing the original retrieval path failed.
    Canonicalize {
        /// Original caller path.
        path: PathBuf,
        /// Original filesystem cause.
        cause: purrdf_hash::error::IoFailure,
    },
    /// The absolute native path has no UTF-8 file-IRI spelling.
    NonUtf8 {
        /// Original caller path used in presentation.
        path: PathBuf,
        /// Canonical native path whose bytes were refused.
        absolute: PathBuf,
    },
    /// The derived file IRI failed its native parser.
    Iri {
        /// Original caller path.
        path: PathBuf,
        /// Original derived IRI.
        iri: String,
        /// Original IRI cause.
        cause: purrdf_iri::IriError,
    },
    /// An artifact is outside the discovery root.
    OutsideRoot {
        /// Original artifact path.
        path: PathBuf,
        /// Original discovery root.
        root: PathBuf,
    },
    /// A relative artifact path contains a forbidden component.
    Component {
        /// Original relative path.
        path: PathBuf,
        /// Original native component bytes.
        component: std::ffi::OsString,
    },
}

impl purrdf_hash::error::StructuredError for PathRefusal {
    purrdf_hash::error_visit! {
        Canonicalize { path, cause } => "slice-path-canonicalize" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "cause" => Cause(cause) },
        NonUtf8 { path, absolute } => "slice-path-non-utf8" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "absolute" => Bytes(absolute.as_os_str().as_encoded_bytes()) },
        Iri { path, iri, cause } => "slice-path-iri" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "iri" => Iri(iri), "cause" => Cause(cause) },
        OutsideRoot { path, root } => "slice-path-outside-root" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "root" => Bytes(root.as_os_str().as_encoded_bytes()) },
        Component { path, component } => "slice-path-component" { "path" => Bytes(path.as_os_str().as_encoded_bytes()), "component" => Bytes(component.as_encoded_bytes()) },
    }
}

impl core::fmt::Display for PathRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Canonicalize { path, cause } => write!(
                f,
                "{}: cannot derive the document's file:// retrieval IRI ({cause})",
                path.display()
            ),
            Self::NonUtf8 { path, .. } => write!(
                f,
                "{}: the path is not valid UTF-8, so it has no file:// IRI",
                path.display()
            ),
            Self::Iri { path, cause, .. } => write!(
                f,
                "{}: the path has no usable file:// IRI ({cause})",
                path.display()
            ),
            Self::OutsideRoot { path, .. } => write!(f, "path not under root: {}", path.display()),
            Self::Component { path, component } => {
                let component = std::path::Path::new(component)
                    .components()
                    .next()
                    .expect("the original native component is nonempty");
                write!(
                    f,
                    "unsafe path component {component:?} in {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for PathRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonicalize { cause, .. } => Some(cause),
            Self::Iri { cause, .. } => Some(cause),
            Self::NonUtf8 { .. } | Self::OutsideRoot { .. } | Self::Component { .. } => None,
        }
    }
}

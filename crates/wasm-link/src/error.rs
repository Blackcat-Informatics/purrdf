// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one error type: every refusal names what was found.

use std::fmt;

/// Why a module could not be linked or checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// The bytes are not a WebAssembly module this tool can read.
    Parse(String),
    /// The module (input or output) failed validation under the artifact's feature set.
    Invalid {
        /// Which module failed: `"input"` or `"output"`.
        stage: &'static str,
        /// The validator's diagnostic.
        message: String,
    },
    /// The module is a valid core module but not the artifact this tool links: the
    /// message names the finding (a missing export, a body of another shape, an import
    /// referenced other than by a direct call).
    Shape(String),
    /// The module already carries the linker's exports; linking twice is refused.
    AlreadyLinked(String),
    /// `--check` found a module that is not the linker's output.
    NotLinked(String),
    /// The wasm-encoder bridge could not translate a construct.
    Reencode(String),
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "the module could not be read: {message}"),
            Self::Invalid { stage, message } => {
                write!(f, "the {stage} module failed validation: {message}")
            }
            Self::Shape(message) => write!(
                f,
                "the module is not the artifact this tool links: {message}"
            ),
            Self::AlreadyLinked(message) => write!(f, "the module is already linked: {message}"),
            Self::NotLinked(message) => write!(f, "the module is not a linked artifact: {message}"),
            Self::Reencode(message) => write!(f, "the module could not be re-encoded: {message}"),
        }
    }
}

impl std::error::Error for LinkError {}

impl From<wasmparser::BinaryReaderError> for LinkError {
    fn from(error: wasmparser::BinaryReaderError) -> Self {
        Self::Parse(error.to_string())
    }
}

impl From<wasm_encoder::reencode::Error<Self>> for LinkError {
    fn from(error: wasm_encoder::reencode::Error<Self>) -> Self {
        match error {
            wasm_encoder::reencode::Error::UserError(inner) => inner,
            wasm_encoder::reencode::Error::ParseError(inner) => Self::Parse(inner.to_string()),
            other => Self::Reencode(other.to_string()),
        }
    }
}

impl From<wasm_encoder::reencode::Error<std::convert::Infallible>> for LinkError {
    fn from(error: wasm_encoder::reencode::Error<std::convert::Infallible>) -> Self {
        match error {
            wasm_encoder::reencode::Error::ParseError(inner) => Self::Parse(inner.to_string()),
            other => Self::Reencode(other.to_string()),
        }
    }
}

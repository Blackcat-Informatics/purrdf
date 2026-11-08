// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closed structural expansion over the shared native document request boundary.

use crate::document::{self, Operation};
use purrdf_core::RdfDiagnostic;
use purrdf_lex::json::Value;
use purrdf_rdf::native_codecs::NativeRdfFormat;

pub use document::OutputLimits;

/// Reusable decoded expansion request, preserving absent and explicit null contexts.
#[derive(Debug)]
pub struct Request(document::Request);
impl Request {
    /// The source format resolved through the native classification home.
    pub const fn format(&self) -> NativeRdfFormat {
        self.0.format()
    }
    /// The original effective bare context, or absence; null stays explicit.
    pub fn initial_context(&self) -> Option<&Value> {
        self.0.initial_context()
    }
    /// Existing source-decoder byte allowance for bounded host acquisition.
    pub fn max_input_bytes(&self) -> usize {
        self.0.max_input_bytes()
    }
}

/// Decode the closed version-one expansion request and compile its input context.
/// # Errors
/// Preserves original request, context, record, lexical and storage diagnostics.
pub fn request(bytes: &[u8]) -> Result<Request, RdfDiagnostic> {
    document::request(bytes, Operation::Expand).map(Request)
}

/// Expand source with a decoded request, preserving independent phase authorities.
/// # Errors
/// Preserves original source, expansion and bounded-output diagnostics and causes.
pub fn expand_decoded(source: &[u8], request: &Request) -> Result<String, RdfDiagnostic> {
    document::execute_decoded(source, &request.0)
}

/// Decode the request before source parsing and return compact expanded JSON text.
/// # Errors
/// Preserves original request, native expansion and bounded-output diagnostics.
pub fn expand(source: &[u8], request_json: &[u8]) -> Result<String, RdfDiagnostic> {
    expand_decoded(source, &request(request_json)?)
}

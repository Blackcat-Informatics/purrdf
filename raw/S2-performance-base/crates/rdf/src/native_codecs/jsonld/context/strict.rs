// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Duplicate-free, bounded JSON decoding shared by JSON-LD contexts and options.

use purrdf_lex::json::Value;

use super::super::ByteLimit;
use crate::RdfDiagnostic;

#[derive(Debug, Clone, Copy)]
pub(super) struct StrictJsonLimits {
    pub(super) bytes: ByteLimit,
    pub(super) depth: usize,
    pub(super) values: usize,
}

pub(super) fn parse_strict_json(
    bytes: &[u8],
    limits: StrictJsonLimits,
    description: &str,
) -> Result<Value, RdfDiagnostic> {
    if !limits.bytes.admits_usize(bytes.len()) {
        return Err(error(format!(
            "{description} is {} bytes; limit is {}",
            bytes.len(),
            limits.bytes
        )));
    }
    crate::json_number::parse_strict(bytes, limits.values, limits.depth)
        .map_err(|source| error(format!("parse {description}: {source}")))
}

fn error(message: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("jsonld-json-input", message)
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable conformance support over parsed RDF and result values.
//!
//! Acquisition, parsers, engines and language transports belong to callers. This
//! crate owns structural manifest reading and grading; graph equality delegates
//! to the kernel's fallible canonicalizer. An unavailable or malformed input is
//! an observed non-pass, never an expected semantic rejection.

#![forbid(unsafe_code)]

pub mod bag;
pub mod graph;
pub mod inventory;
pub mod manifest;
pub mod outcome;
pub mod report;
pub mod result_set;
pub mod reviews;

mod identity;

use std::fmt;

/// A grading refusal distinct from an unequal, well-formed pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradeError {
    /// The input does not satisfy its declared result or manifest shape.
    Malformed(String),
    /// Well-formed inputs disagree under the declared comparison.
    Mismatch(String),
    /// The kernel could not establish canonical equality within its limits.
    Canonicalization(String),
}

impl fmt::Display for GradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, message) = match self {
            Self::Malformed(message) => ("malformed input", message),
            Self::Mismatch(message) => ("mismatch", message),
            Self::Canonicalization(message) => ("canonicalization refused", message),
        };
        write!(f, "{kind}: {message}")
    }
}

impl std::error::Error for GradeError {}

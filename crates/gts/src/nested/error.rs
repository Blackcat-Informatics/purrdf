// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original nested-file admission and decode findings.

use crate::{codec::CodecError, model::Diagnostic};
use purrdf_hash::error::ErrorValue;

#[derive(Debug)]
pub(super) enum NestedError {
    Depth {
        digest: String,
        depth: usize,
        limit: usize,
    },
    Decode {
        digest: String,
        cause: CodecError,
    },
    DecodedSize {
        digest: String,
        actual: usize,
        remaining: usize,
    },
    Unreadable {
        digest: String,
        diagnostics: Vec<Diagnostic>,
    },
}

impl core::fmt::Display for NestedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Depth { digest, limit, .. } => {
                write!(f, "nested GTS blob {digest} exceeds max depth {limit}")
            }
            Self::Decode { digest, cause } => {
                write!(f, "nested GTS blob {digest} decode failed: {cause:?}")
            }
            Self::DecodedSize {
                digest,
                actual,
                remaining,
            } => write!(
                f,
                "nested GTS decoded-size budget exceeded at {digest}: {actual} > {remaining}"
            ),
            Self::Unreadable { digest, .. } => {
                write!(f, "nested GTS blob {digest} could not be parsed")
            }
        }
    }
}

purrdf_lex::error_source!(NestedError => { Self::Decode { cause, .. } => cause });

impl purrdf_hash::error::StructuredError for NestedError {
    purrdf_hash::error_visit! {
        Depth { digest, depth, limit } => "gts-nested-depth" { "digest" => Text(digest), "depth" => Unsigned(*depth as u128), "limit" => Unsigned(*limit as u128) },
        Decode { digest, cause } => "gts-nested-decode" { "digest" => Text(digest), "cause" => Cause(cause) },
        DecodedSize { digest, actual, remaining } => "gts-nested-decoded-size" { "digest" => Text(digest), "actual" => ByteCount(*actual as u128), "remaining" => ByteCount(*remaining as u128) },
        Unreadable { digest, diagnostics } => "gts-nested-unreadable" { "digest" => Text(digest), "diagnostics" => List(diagnostics.iter().map(|diagnostic| ErrorValue::Cause(diagnostic))) },
    }
}

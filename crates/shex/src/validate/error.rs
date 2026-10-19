// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A facet finding and a refused operation are distinct traversal outcomes.

use purrdf_core::xsd_regex::xpath;

#[derive(Debug, Clone)]
pub(super) enum CheckError {
    Violation(String),
    Operational(xpath::Error),
}

impl purrdf_hash::error::StructuredError for CheckError {
    purrdf_hash::error_visit! {
        Violation(reason) => "shex-check-violation" { "reason" => Text(reason) },
        Operational(cause) => "shex-check-operational" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(CheckError { Violation(String) });

impl CheckError {
    pub(super) fn pattern(error: xpath::Error, pattern: &str, flags: &str) -> Self {
        if error.is_operational() {
            Self::Operational(error)
        } else {
            Self::Violation(format!("invalid pattern /{pattern}/{flags}: {error}"))
        }
    }
}

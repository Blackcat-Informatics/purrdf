// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete original code differences for loss-contract verification.

/// The distinct missing and unexpected codes, each in lexical order.
/// This owned refusal remains inspectable after both input sets are dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LossCompletenessError {
    missing: Vec<String>,
    unexpected: Vec<String>,
}

impl LossCompletenessError {
    pub(super) fn new(missing: Vec<String>, unexpected: Vec<String>) -> Self {
        Self {
            missing,
            unexpected,
        }
    }

    /// Expected codes that the ledger did not record.
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// Recorded codes absent from the expected set.
    #[must_use]
    pub fn unexpected(&self) -> &[String] {
        &self.unexpected
    }
}

impl core::fmt::Display for LossCompletenessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("loss ledger incomplete: ")?;
        if !self.missing.is_empty() {
            write!(f, "expected code(s) {:?} were never recorded", self.missing)?;
        }
        if !self.unexpected.is_empty() {
            if !self.missing.is_empty() {
                f.write_str("; ")?;
            }
            write!(
                f,
                "ledger recorded unexpected code(s) {:?} not in expected_codes",
                self.unexpected
            )?;
        }
        Ok(())
    }
}

impl core::error::Error for LossCompletenessError {}

impl purrdf_hash::error::StructuredError for LossCompletenessError {
    purrdf_hash::error_visit! {
        @record self { missing, unexpected } => "loss-ledger-incomplete" {
            "missing" => List(self.missing.iter().map(|code| purrdf_hash::error::ErrorValue::Code(code))),
            "unexpected" => List(self.unexpected.iter().map(|code| purrdf_hash::error::ErrorValue::Code(code))),
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A refused explicit recursive request, including absent limit operands.

/// The exact mode and limits supplied by a caller whose request is incomplete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecursionRequestError(Box<Request>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    mode: Option<String>,
    max_pairs: Option<u64>,
    max_iterations: Option<u64>,
}

impl RecursionRequestError {
    /// Retain a request refused by the shared decoder, without interpreting text.
    #[must_use]
    pub fn new(mode: Option<&str>, max_pairs: Option<u64>, max_iterations: Option<u64>) -> Self {
        Self(Box::new(Request {
            mode: mode.map(str::to_owned),
            max_pairs,
            max_iterations,
        }))
    }
}

impl std::fmt::Display for RecursionRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.mode.as_deref() {
            None => f.write_str("recursion limits require an explicit lfp or gfp selection"),
            Some("lfp" | "gfp") => {
                f.write_str("recursion requires both max_pairs and max_iterations")
            }
            Some(mode) => write!(f, "unknown recursion mode {mode:?}; expected lfp or gfp"),
        }
    }
}
impl std::error::Error for RecursionRequestError {}
impl purrdf_hash::error::StructuredError for RecursionRequestError {
    fn visit<'e>(&'e self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'e>) {
        use purrdf_hash::error::ErrorValue;
        visitor.variant(match self.0.mode.as_deref() {
            None => "shacl-recursion-request-missing-mode",
            Some("lfp" | "gfp") => "shacl-recursion-request-missing-limit",
            Some(_) => "shacl-recursion-request-unknown-mode",
        });
        visitor.value(
            Some("mode"),
            self.0
                .mode
                .as_deref()
                .map_or(ErrorValue::Absent, ErrorValue::Text),
        );
        visitor.value(
            Some("max_pairs"),
            self.0.max_pairs.map_or(ErrorValue::Absent, |value| {
                ErrorValue::Unsigned(u128::from(value))
            }),
        );
        visitor.value(
            Some("max_iterations"),
            self.0.max_iterations.map_or(ErrorValue::Absent, |value| {
                ErrorValue::Unsigned(u128::from(value))
            }),
        );
    }
}

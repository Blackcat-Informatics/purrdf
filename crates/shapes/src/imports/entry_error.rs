// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Retained native causes of unusable import entries.

/// Why the original import-table entry cannot be used.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImportEntryError {
    /// The shared kernel key policy refused the original entry.
    Key(purrdf_core::imports::ImportKeyError),
    /// The imported document failed native Turtle ingress.
    Turtle(crate::text_ingest::TextErrors),
    /// The merged shapes graph failed native construction.
    Merge(purrdf_core::RdfDiagnostic),
}

impl purrdf_hash::error::StructuredError for ImportEntryError {
    purrdf_hash::error_visit! {
        Key(cause) => "shapes-import-key" { "cause" => Cause(cause) },
        Turtle(cause) => "shapes-import-turtle" { "cause" => Cause(cause) },
        Merge(cause) => "shapes-import-merge" { "cause" => Cause(cause) },
    }
}

impl std::fmt::Display for ImportEntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key(cause) => cause.fmt(f),
            Self::Turtle(cause) => {
                f.write_str("the document does not parse as Turtle: ")?;
                for (index, diagnostic) in cause.iter().enumerate() {
                    if index != 0 {
                        f.write_str("; ")?;
                    }
                    diagnostic.fmt(f)?;
                }
                Ok(())
            }
            Self::Merge(cause) => write!(f, "the merged shapes graph does not freeze: {cause}"),
        }
    }
}

purrdf_hash::error_sources!(ImportEntryError, Key, Turtle, Merge);

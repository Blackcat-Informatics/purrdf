// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Manifest and mapping invariants, retaining their original operands.

use crate::artifact::ArtifactRole;
use std::collections::BTreeSet;

/// A manifest or dependency-edit invariant that was refused.
#[derive(Debug)]
pub enum ManifestRefusal {
    /// A semantic compilation phase has no artifact digest.
    MissingSemanticDigest {
        /// Original compilation phase code.
        phase: &'static str,
        /// Logical artifact path.
        path: String,
        /// Original artifact role.
        role: ArtifactRole,
    },
    /// A cache key names no discovered slice.
    UnknownCacheSlice {
        /// Original slice IRI.
        slice: String,
    },
    /// A dependency names no discovered slice.
    UnknownDependencySlice {
        /// Original slice IRI.
        slice: String,
    },
    /// The manifest does not declare exactly one slice subject.
    SliceSubjects {
        /// Configured vocabulary prefix.
        prefix: String,
        /// All declared slice subjects, in lexical order.
        subjects: Vec<String>,
    },
    /// A dependency patch did not produce its prescribed set.
    PatchDependencies {
        /// Original manifest path.
        path: String,
        /// Actual reparsed dependency IRIs.
        actual: BTreeSet<String>,
        /// Required dependency IRIs.
        expected: BTreeSet<String>,
    },
    /// Patching removed the original slice declaration.
    PatchSubject {
        /// Original manifest path.
        path: String,
        /// Original slice IRI.
        slice: String,
    },
    /// The graph has dependencies but no editable textual predicate block.
    MissingPredicateBlock {
        /// Configured vocabulary prefix.
        prefix: String,
    },
    /// No preceding predicate can receive a terminal dot after block removal.
    MissingTerminator,
    /// A new dependency block has no slice declaration to anchor it.
    MissingAnchor {
        /// Configured vocabulary prefix.
        prefix: String,
    },
}

impl purrdf_hash::error::StructuredError for ManifestRefusal {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::{ErrorValue, visit_list};
        match self {
            Self::MissingSemanticDigest { phase, path, role } => {
                visitor.variant("slice-missing-semantic-digest");
                visitor.value(Some("phase"), ErrorValue::Code(phase));
                visitor.value(Some("path"), ErrorValue::Text(path));
                visitor.value(Some("role"), ErrorValue::Record(role));
            }
            Self::UnknownCacheSlice { slice } => {
                visitor.variant("slice-unknown-cache-slice");
                visitor.value(Some("slice"), ErrorValue::Iri(slice));
            }
            Self::UnknownDependencySlice { slice } => {
                visitor.variant("slice-unknown-dependency-slice");
                visitor.value(Some("slice"), ErrorValue::Iri(slice));
            }
            Self::SliceSubjects { prefix, subjects } => {
                visitor.variant("slice-subject-count");
                visitor.value(Some("prefix"), ErrorValue::Text(prefix));
                visitor.value(Some("required"), ErrorValue::Unsigned(1));
                visit_list(
                    visitor,
                    Some("subjects"),
                    subjects.iter().map(|value| ErrorValue::Iri(value)),
                );
            }
            Self::PatchDependencies {
                path,
                actual,
                expected,
            } => {
                visitor.variant("slice-patch-dependencies");
                visitor.value(Some("path"), ErrorValue::Text(path));
                visit_list(
                    visitor,
                    Some("actual"),
                    actual.iter().map(|value| ErrorValue::Iri(value)),
                );
                visit_list(
                    visitor,
                    Some("expected"),
                    expected.iter().map(|value| ErrorValue::Iri(value)),
                );
            }
            Self::PatchSubject { path, slice } => {
                visitor.variant("slice-patch-subject");
                visitor.value(Some("path"), ErrorValue::Text(path));
                visitor.value(Some("slice"), ErrorValue::Iri(slice));
            }
            Self::MissingPredicateBlock { prefix } => {
                visitor.variant("slice-missing-predicate-block");
                visitor.value(Some("prefix"), ErrorValue::Text(prefix));
            }
            Self::MissingTerminator => visitor.variant("slice-missing-terminator"),
            Self::MissingAnchor { prefix } => {
                visitor.variant("slice-missing-anchor");
                visitor.value(Some("prefix"), ErrorValue::Text(prefix));
            }
        }
    }
}

impl core::fmt::Display for ManifestRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingSemanticDigest { phase, path, role } => write!(f, "phase {phase} requires a semantic digest for artifact {path} (role {role:?}) but none was computed"),
            Self::UnknownCacheSlice { slice } => write!(f, "cache key references unknown slice IRI {slice}"),
            Self::UnknownDependencySlice { slice } => write!(f, "fix-deps: depending slice {slice} not found in catalog"),
            Self::SliceSubjects { prefix, subjects } => {
                if subjects.is_empty() {
                    write!(f, "no `a {prefix}:Slice` triple found in manifest.ttl")
                } else {
                    write!(f, "manifest.ttl declares {} `a {prefix}:Slice` subjects (must be exactly one): ", subjects.len())?;
                    for (index, subject) in subjects.iter().enumerate() {
                        if index != 0 { f.write_str(", ")?; }
                        f.write_str(subject)?;
                    }
                    Ok(())
                }
            }
            Self::PatchDependencies { path, actual, expected } => write!(f, "fix-deps: patched {path} did not yield the expected sliceDependsOn set (got {actual:?}, expected {expected:?})"),
            Self::PatchSubject { path, slice } => write!(f, "fix-deps: patched {path} no longer declares its slice subject {slice}"),
            Self::MissingPredicateBlock { prefix } => write!(f, "fix-deps: could not locate the {prefix}:sliceDependsOn predicate block for a surgical edit"),
            Self::MissingTerminator => f.write_str("fix-deps: removing the terminal sliceDependsOn block left no predicate to carry the statement terminator"),
            Self::MissingAnchor { prefix } => write!(f, "fix-deps: manifest has no `a {prefix}:Slice` line to anchor a new sliceDependsOn predicate"),
        }
    }
}

impl std::error::Error for ManifestRefusal {}

/// A required mapping-file binding is absent from its original subject.
#[derive(Debug)]
pub enum ParseRefusal {
    /// A term equivalence has no file binding.
    TermEquivalence {
        /// Original subject IRI.
        subject: String,
    },
    /// A mapping set has no file binding.
    MappingSet {
        /// Original subject IRI.
        subject: String,
    },
}

impl purrdf_hash::error::StructuredError for ParseRefusal {
    purrdf_hash::error_visit! {
        TermEquivalence { subject } => "slice-term-equivalence-file-missing" { "subject" => Iri(subject) },
        MappingSet { subject } => "slice-mapping-set-file-missing" { "subject" => Iri(subject) },
    }
}

impl core::fmt::Display for ParseRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TermEquivalence { subject } => {
                write!(f, "term equivalence {subject} missing sssomFile")
            }
            Self::MappingSet { subject } => write!(f, "mapping set {subject} missing sssomFile"),
        }
    }
}

impl std::error::Error for ParseRefusal {}

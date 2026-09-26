// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Why a schema could not be registered or compiled.

use std::fmt;

use crate::ecma::PatternError;

/// A schema could not be registered or compiled.
///
/// Every variant is a refusal to *process* the schema. None of them is a
/// validation verdict: an instance is never reported valid or invalid under a
/// schema this crate could not read as its author wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaError {
    /// A retrieval URI or identifier is not an absolute IRI, or carries a
    /// non-empty fragment.
    InvalidUri {
        /// The offending text.
        uri: String,
        /// Why it was refused.
        reason: String,
    },
    /// Two resources claim the same URI.
    DuplicateResource {
        /// The URI claimed twice.
        uri: String,
    },
    /// A schema declares a `$schema` dialect this crate does not implement:
    /// neither draft 2020-12, draft 2019-09 nor draft-07, nor a registered
    /// meta-schema built on one of them. Other drafts give keywords different
    /// meanings (draft-06 and earlier spell identifiers `id` and bounds
    /// differently), so they are refused rather than read as one of these.
    UnsupportedDialect {
        /// The resource whose dialect was refused.
        resource: String,
        /// The `$schema` it declares.
        dialect: String,
    },
    /// A meta-schema requires (`true` in `$vocabulary`) a vocabulary this
    /// crate does not implement.
    UnsupportedVocabulary {
        /// The meta-schema declaring it.
        metaschema: String,
        /// The vocabulary URI.
        vocabulary: String,
    },
    /// `format` is an assertion under the meta-schema, and names a format
    /// this crate cannot check completely.
    UnsupportedFormat {
        /// Where the keyword is.
        location: String,
        /// The format name.
        format: String,
    },
    /// A `$ref` or `$dynamicRef` does not resolve to a registered schema.
    UnresolvedReference {
        /// Where the reference is.
        location: String,
        /// The reference as written.
        reference: String,
        /// The absolute URI it resolved to.
        resolved: String,
    },
    /// Something that must be a schema is not an object or a boolean, or a
    /// keyword's value has the wrong shape.
    InvalidKeyword {
        /// Where the keyword is.
        location: String,
        /// What was expected.
        reason: String,
    },
    /// An `$anchor`, `$dynamicAnchor` or `$id` is malformed or repeated.
    InvalidIdentifier {
        /// Where it is.
        location: String,
        /// Why it was refused.
        reason: String,
    },
    /// A `pattern` or `patternProperties` key is not a runnable ECMA-262
    /// regular expression.
    Pattern {
        /// Where the pattern is.
        location: String,
        /// The pattern.
        pattern: String,
        /// Why it was refused.
        error: PatternError,
    },
    /// The schema document is not valid against its meta-schema.
    InvalidSchema {
        /// The document's retrieval URI.
        uri: String,
        /// `(keywordLocation, instanceLocation, message)` for each failure the
        /// meta-schema reported.
        errors: Vec<(String, String, String)>,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUri { uri, reason } => write!(f, "invalid URI {uri:?}: {reason}"),
            Self::DuplicateResource { uri } => {
                write!(f, "two schema resources claim the URI {uri:?}")
            }
            Self::UnsupportedDialect { resource, dialect } => write!(
                f,
                "{resource} declares the dialect {dialect:?}; the supported dialects are JSON \
                 Schema draft 2020-12, draft 2019-09 and draft-07, and registered meta-schemas \
                 built on them"
            ),
            Self::UnsupportedVocabulary {
                metaschema,
                vocabulary,
            } => write!(
                f,
                "meta-schema {metaschema} requires the vocabulary {vocabulary}, which is not \
                 implemented"
            ),
            Self::UnsupportedFormat { location, format } => write!(
                f,
                "{location}: format {format:?} is an assertion here and cannot be checked \
                 completely"
            ),
            Self::UnresolvedReference {
                location,
                reference,
                resolved,
            } => write!(
                f,
                "{location}: reference {reference:?} resolves to {resolved}, which is not a \
                 registered schema"
            ),
            Self::InvalidKeyword { location, reason } => write!(f, "{location}: {reason}"),
            Self::InvalidIdentifier { location, reason } => write!(f, "{location}: {reason}"),
            Self::Pattern {
                location,
                pattern,
                error,
            } => write!(f, "{location}: pattern {pattern:?}: {error}"),
            Self::InvalidSchema { uri, errors } => {
                write!(f, "{uri} is not valid against its meta-schema")?;
                for (keyword, instance, message) in errors {
                    write!(f, "; at {instance:?} ({keyword}): {message}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SchemaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pattern { error, .. } => Some(error),
            _ => None,
        }
    }
}

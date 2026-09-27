// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Errors raised while registering, resolving and compiling JSON Schemas.
//!
//! Every refusal names what was refused: the URI, the keyword's absolute
//! location (a URI whose fragment is a JSON Pointer into the schema
//! resource), and the offending value where there is one. Validation of an
//! *instance* never produces a [`SchemaError`]; that verdict is reported
//! through the evaluation output instead.

use std::error::Error;
use std::fmt;

use crate::ecma::PatternError;

/// Evaluation stopped because an expression exhausted its resource budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationError {
    /// Absolute keyword location responsible for the expression.
    pub keyword_location: String,
    /// Location of the instance being checked.
    pub instance_location: String,
    /// The matcher error.
    pub cause: PatternError,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            self.keyword_location, self.instance_location, self.cause
        )
    }
}
impl Error for EvaluationError {}

/// Why a schema, or a set of schemas, could not be registered or compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    /// A meta-schema evaluation exhausted its resource budget.
    Evaluation(EvaluationError),
    /// A URI could not be parsed, could not be resolved against its base, or
    /// has a shape the operation does not accept (relative where an absolute
    /// URI is needed, or carrying a fragment where none is allowed).
    InvalidUri {
        /// The URI or reference as given.
        uri: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A reference (or the URI asked to be compiled) resolves to a location
    /// that no registered resource provides.
    UnresolvedReference {
        /// The absolute location of the keyword holding the reference.
        location: String,
        /// The reference as written.
        reference: String,
        /// The reference resolved against its base URI.
        resolved: String,
    },
    /// A resource is written against a meta-schema that is not registered;
    /// meta-schemas are caller-supplied and never fetched or invented.
    MissingMetaschema {
        /// The URI of the meta-schema that is needed.
        metaschema: String,
        /// The resource that needs it.
        resource: String,
    },
    /// A schema document does not validate against its own meta-schema.
    InvalidSchema {
        /// The URI of the refused document.
        uri: String,
        /// Each failure the meta-schema reported, as
        /// `(keyword location, instance location, message)`: where in the
        /// meta-schema the failing keyword is, where in the refused document
        /// the failing value is, and what the meta-schema said about it.
        errors: Vec<(String, String, String)>,
    },
    /// A keyword's value is malformed in a way that prevents compiling it.
    InvalidKeyword {
        /// The absolute location of the keyword.
        location: String,
        /// What is wrong with its value.
        reason: String,
    },
    /// A `pattern` or `patternProperties` regular expression could not be
    /// compiled as an ECMA-262 regular expression.
    Pattern {
        /// The absolute location of the keyword holding the expression.
        location: String,
        /// The expression as written.
        pattern: String,
        /// Why the expression was refused.
        error: PatternError,
    },
    /// A `format` must assert here, but the named format has no check.
    UnsupportedFormat {
        /// The absolute location of the `format` keyword.
        location: String,
        /// The format name as written.
        format: String,
    },
    /// Two resources claim the same URI.
    DuplicateResource {
        /// The URI claimed twice.
        uri: String,
    },
    /// A resource declares, or inherits through a custom meta-schema, a
    /// dialect this crate does not implement.
    UnsupportedDialect {
        /// The resource written in that dialect.
        resource: String,
        /// The dialect's meta-schema URI, as declared.
        dialect: String,
    },
    /// A meta-schema's `$vocabulary` requires a vocabulary this crate does
    /// not implement.
    UnsupportedVocabulary {
        /// The meta-schema that requires it.
        metaschema: String,
        /// The URI of the required vocabulary.
        vocabulary: String,
    },
    /// An `$id`, `$anchor` or `$dynamicAnchor` is malformed or conflicts with
    /// another identifier in the same resource.
    InvalidIdentifier {
        /// The absolute location of the identifying keyword.
        location: String,
        /// What is wrong with it.
        reason: String,
    },
}

impl SchemaError {
    /// The schema location a located error points at, reported once after
    /// the description.
    fn location(&self) -> Option<&str> {
        match self {
            Self::Evaluation(error) => Some(&error.keyword_location),
            Self::UnresolvedReference { location, .. }
            | Self::InvalidKeyword { location, .. }
            | Self::Pattern { location, .. }
            | Self::UnsupportedFormat { location, .. }
            | Self::InvalidIdentifier { location, .. } => Some(location),
            Self::InvalidUri { .. }
            | Self::MissingMetaschema { .. }
            | Self::InvalidSchema { .. }
            | Self::DuplicateResource { .. }
            | Self::UnsupportedDialect { .. }
            | Self::UnsupportedVocabulary { .. } => None,
        }
    }

    /// The problem itself, without the location.
    fn describe(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluation(error) => write!(f, "meta-schema evaluation failed: {error}"),
            Self::InvalidUri { uri, reason } => {
                write!(f, "`{uri}` cannot be used as a URI here: {reason}")
            }
            Self::UnresolvedReference {
                location,
                reference,
                resolved,
            } => {
                if reference == resolved && resolved == location {
                    write!(f, "no registered resource provides `{resolved}`")
                } else if reference == resolved {
                    write!(
                        f,
                        "the reference `{reference}` points at nothing registered"
                    )
                } else {
                    write!(
                        f,
                        "the reference `{reference}` resolves to `{resolved}`, \
                         which no registered resource provides"
                    )
                }
            }
            Self::MissingMetaschema {
                metaschema,
                resource,
            } => write!(
                f,
                "`{resource}` needs the meta-schema `{metaschema}`, \
                 which has not been registered"
            ),
            Self::InvalidSchema { uri, errors } => {
                let count = errors.len();
                let noun = if count == 1 { "failure" } else { "failures" };
                write!(
                    f,
                    "the schema `{uri}` does not conform to its meta-schema \
                     ({count} {noun})"
                )?;
                for (keyword, instance, message) in errors {
                    let instance = if instance.is_empty() {
                        "the document root"
                    } else {
                        instance
                    };
                    write!(f, "\n  - at {instance} (meta-schema keyword {keyword})")?;
                    if !message.is_empty() {
                        write!(f, ": {message}")?;
                    }
                }
                Ok(())
            }
            Self::InvalidKeyword { reason, .. } => {
                write!(f, "malformed keyword: {reason}")
            }
            Self::Pattern { pattern, error, .. } => {
                write!(f, "the regular expression {pattern:?} is refused: {error}")
            }
            Self::UnsupportedFormat { format, .. } => {
                write!(f, "the format {format:?} must be asserted but has no check")
            }
            Self::DuplicateResource { uri } => {
                write!(f, "more than one resource claims the URI `{uri}`")
            }
            Self::UnsupportedDialect { resource, dialect } => write!(
                f,
                "`{resource}` is written in the dialect `{dialect}`, \
                 which is not supported"
            ),
            Self::UnsupportedVocabulary {
                metaschema,
                vocabulary,
            } => write!(
                f,
                "the meta-schema `{metaschema}` requires the vocabulary \
                 `{vocabulary}`, which is not implemented"
            ),
            Self::InvalidIdentifier { reason, .. } => {
                write!(f, "malformed identifier: {reason}")
            }
        }
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.describe(f)?;
        match self.location() {
            // An unresolved compile target already names its only URI.
            Some(location)
                if !matches!(
                    self,
                    Self::UnresolvedReference { resolved, .. } if resolved == location
                ) =>
            {
                write!(f, " (at `{location}`)")
            }
            _ => Ok(()),
        }
    }
}

impl Error for SchemaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pattern { error, .. } => Some(error),
            _ => None,
        }
    }
}

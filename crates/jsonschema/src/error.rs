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

mod keyword;
pub use keyword::{IdentifierRefusal, KeywordRefusal, KeywordRequirement};

/// Original URI grammar, resolution or schema-location refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriRefusal {
    /// The original URI grammar refusal.
    Parse(purrdf_iri::IriError),
    /// Resolving against the original base failed.
    Resolve {
        /// Base URI used by the resolver.
        base: String,
        /// Original resolver cause.
        cause: purrdf_iri::IriError,
    },
    /// A compiled schema must have an absolute location.
    CompilationRelative,
    /// A registered resource must have an absolute retrieval location.
    RetrievalRelative,
    /// A resource location cannot carry a nonempty fragment.
    Fragment,
}

impl purrdf_hash::error::StructuredError for UriRefusal {
    purrdf_hash::error_visit! {
        Parse(cause) => "schema-uri-parse" { "cause" => Cause(cause) },
        Resolve { base, cause } => "schema-uri-resolve" { "base" => Iri(base), "cause" => Cause(cause) },
        CompilationRelative => "schema-uri-compilation-relative" {},
        RetrievalRelative => "schema-uri-retrieval-relative" {},
        Fragment => "schema-uri-fragment" {},
    }
}

impl fmt::Display for UriRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(cause) | Self::Resolve { cause, .. } => cause.fmt(f),
            Self::CompilationRelative => f.write_str("a schema is compiled from an absolute URI"),
            Self::RetrievalRelative => f.write_str("a retrieval URI must be absolute"),
            Self::Fragment => f.write_str("a resource URI may not carry a non-empty fragment"),
        }
    }
}

impl Error for UriRefusal {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(cause) | Self::Resolve { cause, .. } => Some(cause),
            Self::CompilationRelative | Self::RetrievalRelative | Self::Fragment => None,
        }
    }
}

#[cfg(test)]
mod uri_tests {
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;

    #[test]
    fn schema_resolution_retains_original_uri_and_base_after_input_death() {
        let base = String::from("https://example.org/schema");
        let reference = String::from("https://[invalid");
        let error = crate::registry::resolve(&base, &reference).unwrap_err();
        let expected = purrdf_iri::parse(&reference).unwrap_err();
        drop(base);
        drop(reference);
        let retained = error.clone();
        drop(error);
        let fields = Capture::read(&retained);
        assert!(matches!(
            fields.field("uri"),
            ErrorValue::Iri("https://[invalid")
        ));
        let reason = fields.child("reason");
        assert_eq!(reason.code, "schema-uri-resolve");
        assert!(matches!(
            reason.field("base"),
            ErrorValue::Iri("https://example.org/schema")
        ));
        assert_eq!(reason.child("cause").code, Capture::read(&expected).code);
        assert!(std::error::Error::source(&retained).is_some());
    }
}

/// Why an evaluation stopped without reaching a verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationCause {
    /// A regular expression exhausted its matcher budget, or a string checked
    /// by the asserting `regex` format exceeds the ECMA-262 parser's limits.
    Pattern(PatternError),
    /// More than [`crate::MAX_REF_CHAIN`] references were followed in a row
    /// at one instance location.
    ReferenceChain,
}

impl purrdf_hash::error::StructuredError for EvaluationCause {
    purrdf_hash::error_visit! {
        Pattern(cause) => "schema-evaluation-pattern" { "cause" => Cause(cause) },
        ReferenceChain => "schema-evaluation-reference-chain" { "limit" => Unsigned(crate::MAX_REF_CHAIN as u128) },
    }
}

impl fmt::Display for EvaluationCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pattern(error) => error.fmt(f),
            Self::ReferenceChain => write!(
                f,
                "more than {} references are followed in a row at one instance location",
                crate::MAX_REF_CHAIN
            ),
        }
    }
}

/// Evaluation stopped because it exhausted a resource bound before reaching a
/// verdict; an instance is never reported valid or invalid on a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationError {
    /// Absolute location of the keyword that hit the bound.
    pub keyword_location: String,
    /// Location of the instance being checked.
    pub instance_location: String,
    /// The bound that was hit.
    pub cause: EvaluationCause,
}

impl purrdf_hash::error::StructuredError for EvaluationError {
    purrdf_hash::error_visit! {
        @record self { keyword_location, instance_location, cause } => "schema-evaluation-error" {
            "keyword_location" => Iri(&self.keyword_location), "instance_location" => Text(&self.instance_location), "cause" => Cause(&self.cause)
        }
    }
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
impl Error for EvaluationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.cause {
            EvaluationCause::Pattern(error) => Some(error),
            EvaluationCause::ReferenceChain => None,
        }
    }
}

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
        reason: UriRefusal,
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
        /// `(keyword location, instance location, native reason)`: where in the
        /// meta-schema the failing keyword is, where in the refused document
        /// the failing value is, and the original keyword's typed refusal.
        errors: Vec<(
            String,
            String,
            Option<std::sync::Arc<crate::ValidationRefusal>>,
        )>,
    },
    /// A keyword's value is malformed in a way that prevents compiling it.
    InvalidKeyword {
        /// The absolute location of the keyword.
        location: String,
        /// What is wrong with its value.
        reason: KeywordRefusal,
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
        reason: IdentifierRefusal,
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
                for (keyword, instance, reason) in errors {
                    let instance = if instance.is_empty() {
                        "the document root"
                    } else {
                        instance
                    };
                    write!(f, "\n  - at {instance} (meta-schema keyword {keyword})")?;
                    if let Some(reason) = reason {
                        write!(f, ": {reason}")?;
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
            Self::Evaluation(cause) => Some(cause),
            Self::Pattern { error, .. } => Some(error),
            Self::InvalidUri { reason, .. } => Some(reason),
            Self::InvalidKeyword { reason, .. } => Some(reason),
            Self::InvalidIdentifier { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for SchemaError {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        use purrdf_hash::error::ErrorValue;
        match self {
            Self::Evaluation(cause) => {
                visitor.variant("schema-evaluation");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Self::InvalidUri { uri, reason } => {
                visitor.variant("schema-invalid-uri");
                visitor.value(Some("uri"), ErrorValue::Iri(uri));
                visitor.value(Some("reason"), ErrorValue::Cause(reason));
            }
            Self::UnresolvedReference {
                location,
                reference,
                resolved,
            } => {
                visitor.variant("schema-unresolved-reference");
                visitor.value(Some("location"), ErrorValue::Iri(location));
                visitor.value(Some("reference"), ErrorValue::Iri(reference));
                visitor.value(Some("resolved"), ErrorValue::Iri(resolved));
            }
            Self::MissingMetaschema {
                metaschema,
                resource,
            } => {
                visitor.variant("schema-missing-metaschema");
                visitor.value(Some("metaschema"), ErrorValue::Iri(metaschema));
                visitor.value(Some("resource"), ErrorValue::Iri(resource));
            }
            Self::InvalidSchema { uri, errors } => {
                visitor.variant("schema-invalid-schema");
                visitor.value(Some("uri"), ErrorValue::Iri(uri));
                visitor.begin_list(Some("errors"), errors.len());
                for (keyword, instance, reason) in errors {
                    visitor.begin_record(None, "schema-validation-finding");
                    visitor.value(Some("keyword_location"), ErrorValue::Iri(keyword));
                    visitor.value(Some("instance_location"), ErrorValue::Text(instance));
                    visitor.value(
                        Some("reason"),
                        match reason {
                            Some(reason) => ErrorValue::Cause(&**reason),
                            None => ErrorValue::Absent,
                        },
                    );
                    visitor.end_record();
                }
                visitor.end_list();
            }
            Self::InvalidKeyword { location, reason } => {
                visitor.variant("schema-invalid-keyword");
                visitor.value(Some("location"), ErrorValue::Iri(location));
                visitor.value(Some("reason"), ErrorValue::Cause(reason));
            }
            Self::Pattern {
                location,
                pattern,
                error,
            } => {
                visitor.variant("schema-pattern");
                visitor.value(Some("location"), ErrorValue::Iri(location));
                visitor.value(Some("pattern"), ErrorValue::Text(pattern));
                visitor.value(Some("error"), ErrorValue::Cause(error));
            }
            Self::UnsupportedFormat { location, format } => {
                visitor.variant("schema-unsupported-format");
                visitor.value(Some("location"), ErrorValue::Iri(location));
                visitor.value(Some("format"), ErrorValue::Text(format));
            }
            Self::DuplicateResource { uri } => {
                visitor.variant("schema-duplicate-resource");
                visitor.value(Some("uri"), ErrorValue::Iri(uri));
            }
            Self::UnsupportedDialect { resource, dialect } => {
                visitor.variant("schema-unsupported-dialect");
                visitor.value(Some("resource"), ErrorValue::Iri(resource));
                visitor.value(Some("dialect"), ErrorValue::Iri(dialect));
            }
            Self::UnsupportedVocabulary {
                metaschema,
                vocabulary,
            } => {
                visitor.variant("schema-unsupported-vocabulary");
                visitor.value(Some("metaschema"), ErrorValue::Iri(metaschema));
                visitor.value(Some("vocabulary"), ErrorValue::Iri(vocabulary));
            }
            Self::InvalidIdentifier { location, reason } => {
                visitor.variant("schema-invalid-identifier");
                visitor.value(Some("location"), ErrorValue::Iri(location));
                visitor.value(Some("reason"), ErrorValue::Cause(reason));
            }
        }
    }
}

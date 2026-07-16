// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use purrdf_core::{BlankScope, DatasetView, RdfTextDirection, TermRef, TermValue};
use serde::{Deserialize, Serialize};

use super::util::canonical_json_bounded;
use super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Portable RDF 1.2 literal base direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectionDirection {
    /// Left-to-right.
    Ltr,
    /// Right-to-left.
    Rtl,
}

impl From<RdfTextDirection> for ProjectionDirection {
    fn from(value: RdfTextDirection) -> Self {
        match value {
            RdfTextDirection::Ltr => Self::Ltr,
            RdfTextDirection::Rtl => Self::Rtl,
        }
    }
}

impl From<ProjectionDirection> for RdfTextDirection {
    fn from(value: ProjectionDirection) -> Self {
        match value {
            ProjectionDirection::Ltr => Self::Ltr,
            ProjectionDirection::Rtl => Self::Rtl,
        }
    }
}

/// Dataset-independent, serialization-stable RDF 1.2 term identity.
///
/// Unlike `TermId`, this value is safe to persist in a projection artifact. It
/// preserves blank-node scope, literal lexical/datatype/language/direction identity,
/// and recursively nested triple terms. The tagged JSON representation is canonical
/// for a given value because variant and field order are fixed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProjectionTerm {
    /// Full absolute IRI.
    Iri {
        /// IRI string.
        value: String,
    },
    /// Blank node with explicit structural scope.
    Blank {
        /// Bare blank-node label.
        label: String,
        /// Scope ordinal.
        scope: u32,
    },
    /// RDF literal with its complete identity tuple.
    Literal {
        /// Authored lexical form.
        lexical: String,
        /// Expanded datatype IRI.
        datatype: String,
        /// Lowercase language tag, when present.
        language: Option<String>,
        /// RDF 1.2 base direction, when present.
        direction: Option<ProjectionDirection>,
    },
    /// RDF 1.2 quoted triple term.
    Triple {
        /// Quoted subject.
        subject: Box<Self>,
        /// Quoted predicate, which validation requires to be an IRI.
        predicate: Box<Self>,
        /// Quoted object.
        object: Box<Self>,
    },
}

impl ProjectionTerm {
    /// Resolve a dataset-local term id into a durable value under the configured
    /// recursion bound.
    ///
    /// # Errors
    ///
    /// Returns a term error for structurally invalid datatype/predicate positions or
    /// cycles, and a resource-limit error when triple nesting exceeds the bound.
    pub fn from_view<D: DatasetView>(
        view: &D,
        id: D::Id,
        limits: ProjectionLimits,
    ) -> Result<Self, ProjectionError> {
        let mut active = BTreeSet::new();
        let term = Self::from_view_inner(view, id, limits, 0, &mut active)?;
        term.validate(limits)?;
        Ok(term)
    }

    fn from_view_inner<D: DatasetView>(
        view: &D,
        id: D::Id,
        limits: ProjectionLimits,
        depth: usize,
        active: &mut BTreeSet<D::Id>,
    ) -> Result<Self, ProjectionError> {
        if !active.insert(id) {
            return Err(ProjectionError::term("cyclic triple-term component graph"));
        }
        let result = match view.resolve(id) {
            TermRef::Iri(value) => Ok(Self::Iri {
                value: value.to_owned(),
            }),
            TermRef::Blank { label, scope } => Ok(Self::Blank {
                label: label.to_owned(),
                scope: scope.ordinal(),
            }),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                let TermRef::Iri(datatype) = view.resolve(datatype) else {
                    active.remove(&id);
                    return Err(ProjectionError::term(
                        "literal datatype position does not resolve to an IRI",
                    ));
                };
                Ok(Self::Literal {
                    lexical: lexical.to_owned(),
                    datatype: datatype.to_owned(),
                    language: language.map(str::to_owned),
                    direction: direction.map(Into::into),
                })
            }
            TermRef::Triple { s, p, o } => {
                Self::validate_depth(limits, depth)?;
                let subject = Self::from_view_inner(view, s, limits, depth + 1, active)?;
                let predicate = Self::from_view_inner(view, p, limits, depth + 1, active)?;
                if !matches!(predicate, Self::Iri { .. }) {
                    active.remove(&id);
                    return Err(ProjectionError::term(
                        "triple-term predicate position does not resolve to an IRI",
                    ));
                }
                let object = Self::from_view_inner(view, o, limits, depth + 1, active)?;
                Ok(Self::Triple {
                    subject: Box::new(subject),
                    predicate: Box::new(predicate),
                    object: Box::new(object),
                })
            }
        };
        active.remove(&id);
        result
    }

    /// Convert a dataset-independent kernel value into its projection carrier form.
    ///
    /// # Errors
    ///
    /// Returns a typed term or resource-limit error when `value` is not a valid,
    /// bounded RDF 1.2 term.
    pub fn from_term_value(
        value: &TermValue,
        limits: ProjectionLimits,
    ) -> Result<Self, ProjectionError> {
        let term = Self::from_term_value_inner(value, limits, 0)?;
        term.validate(limits)?;
        Ok(term)
    }

    fn from_term_value_inner(
        value: &TermValue,
        limits: ProjectionLimits,
        depth: usize,
    ) -> Result<Self, ProjectionError> {
        Ok(match value {
            TermValue::Iri(value) => Self::Iri {
                value: value.clone(),
            },
            TermValue::Blank { label, scope } => Self::Blank {
                label: label.clone(),
                scope: scope.ordinal(),
            },
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => Self::Literal {
                lexical: lexical_form.clone(),
                datatype: datatype.clone(),
                language: language.clone(),
                direction: direction.map(Into::into),
            },
            TermValue::Triple { s, p, o } => {
                Self::validate_depth(limits, depth)?;
                Self::Triple {
                    subject: Box::new(Self::from_term_value_inner(s, limits, depth + 1)?),
                    predicate: Box::new(Self::from_term_value_inner(p, limits, depth + 1)?),
                    object: Box::new(Self::from_term_value_inner(o, limits, depth + 1)?),
                }
            }
        })
    }

    /// Convert this carrier term back to the kernel's dataset-independent value.
    ///
    /// # Errors
    ///
    /// Returns a typed term or resource-limit error when this value is not a valid,
    /// bounded RDF 1.2 term.
    pub fn to_term_value(&self, limits: ProjectionLimits) -> Result<TermValue, ProjectionError> {
        self.validate(limits)?;
        Ok(self.to_term_value_inner())
    }

    fn to_term_value_inner(&self) -> TermValue {
        match self {
            Self::Iri { value } => TermValue::Iri(value.clone()),
            Self::Blank { label, scope } => TermValue::Blank {
                label: label.clone(),
                scope: BlankScope(*scope),
            },
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => TermValue::Literal {
                lexical_form: lexical.clone(),
                datatype: datatype.clone(),
                language: language.clone(),
                direction: direction.map(Into::into),
            },
            Self::Triple {
                subject,
                predicate,
                object,
            } => TermValue::Triple {
                s: Box::new(subject.to_term_value_inner()),
                p: Box::new(predicate.to_term_value_inner()),
                o: Box::new(object.to_term_value_inner()),
            },
        }
    }

    /// Serialize this term to its canonical tagged JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns a typed term or resource-limit error for an invalid value, or a syntax
    /// error if the data-model serialization itself reports a failure.
    pub fn to_canonical_json(&self, limits: ProjectionLimits) -> Result<Vec<u8>, ProjectionError> {
        self.validate(limits)?;
        canonical_json_bounded(self, limits, "canonical term JSON")
    }

    /// Parse and validate canonical tagged JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns a syntax error for invalid/non-canonical JSON and a typed term or limit
    /// error for an invalid RDF value.
    pub fn from_canonical_json(
        bytes: &[u8],
        limits: ProjectionLimits,
    ) -> Result<Self, ProjectionError> {
        if bytes.len() > limits.max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "term JSON is {} bytes; limit is {}",
                bytes.len(),
                limits.max_artifact_bytes()
            )));
        }
        let term: Self = serde_json::from_slice(bytes)
            .map_err(|error| ProjectionError::syntax(format!("parse term JSON: {error}")))?;
        term.validate(limits)?;
        let canonical = term.to_canonical_json(limits)?;
        if canonical != bytes {
            return Err(ProjectionError::syntax(
                "term JSON is valid but not in canonical PurRDF form",
            ));
        }
        Ok(term)
    }

    /// Validate this RDF 1.2 term under an explicit recursion bound.
    ///
    /// # Errors
    ///
    /// Returns a typed term error for invalid RDF positions or identity fields, and a
    /// resource-limit error when nested triple terms exceed the configured depth.
    pub fn validate(&self, limits: ProjectionLimits) -> Result<(), ProjectionError> {
        self.validate_inner(limits, 0)
    }

    fn validate_depth(limits: ProjectionLimits, depth: usize) -> Result<(), ProjectionError> {
        if depth > limits.max_term_depth() {
            return Err(ProjectionError::limit(format!(
                "RDF triple term exceeds the configured depth limit of {}",
                limits.max_term_depth()
            )));
        }
        Ok(())
    }

    fn validate_inner(
        &self,
        limits: ProjectionLimits,
        depth: usize,
    ) -> Result<(), ProjectionError> {
        match self {
            Self::Iri { value } => validate_absolute_iri(value, "term IRI")
                .map_err(|error| ProjectionError::term(error.message())),
            Self::Blank { label, .. } => {
                if label.is_empty() || label.chars().any(char::is_control) {
                    Err(ProjectionError::term(
                        "blank-node label must be non-empty and control-free",
                    ))
                } else {
                    Ok(())
                }
            }
            Self::Literal {
                datatype,
                language,
                direction,
                ..
            } => {
                validate_absolute_iri(datatype, "literal datatype")
                    .map_err(|error| ProjectionError::term(error.message()))?;
                if direction.is_some() && language.is_none() {
                    return Err(ProjectionError::term(
                        "an RDF 1.2 literal direction requires a language tag",
                    ));
                }
                if let Some(language) = language {
                    validate_language_tag(language)?;
                    if language != &language.to_lowercase() {
                        return Err(ProjectionError::term(
                            "language tag must use lowercase canonical form",
                        ));
                    }
                    if datatype != RDF_LANG_STRING {
                        return Err(ProjectionError::term(format!(
                            "language-tagged literals must use datatype {RDF_LANG_STRING}"
                        )));
                    }
                } else if datatype == RDF_LANG_STRING {
                    return Err(ProjectionError::term(
                        "rdf:langString literals require a language tag",
                    ));
                }
                Ok(())
            }
            Self::Triple {
                subject,
                predicate,
                object,
            } => {
                Self::validate_depth(limits, depth)?;
                if matches!(subject.as_ref(), Self::Literal { .. }) {
                    return Err(ProjectionError::term(
                        "triple-term subject must not be a literal",
                    ));
                }
                subject.validate_inner(limits, depth + 1)?;
                let Self::Iri { value } = predicate.as_ref() else {
                    return Err(ProjectionError::term(
                        "triple-term predicate must be an IRI",
                    ));
                };
                validate_absolute_iri(value, "triple-term predicate")
                    .map_err(|error| ProjectionError::term(error.message()))?;
                object.validate_inner(limits, depth + 1)
            }
        }
    }
}

fn validate_language_tag(tag: &str) -> Result<(), ProjectionError> {
    let mut parts = tag.split('-');
    let primary = parts.next().unwrap_or_default();
    if primary.is_empty()
        || primary.len() > 8
        || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(ProjectionError::term(format!(
            "invalid language tag {tag:?}"
        )));
    }
    let mut private_use = primary.eq_ignore_ascii_case("x");
    let mut saw_subtag = false;
    let mut ends_with_private_marker = private_use;
    for subtag in parts {
        saw_subtag = true;
        let alphanumeric =
            !subtag.is_empty() && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric());
        if !alphanumeric || (!private_use && subtag.len() > 8) {
            return Err(ProjectionError::term(format!(
                "invalid language tag {tag:?}"
            )));
        }
        if subtag.eq_ignore_ascii_case("x") {
            private_use = true;
            ends_with_private_marker = true;
        } else {
            ends_with_private_marker = false;
        }
    }
    if (primary.eq_ignore_ascii_case("x") && !saw_subtag) || ends_with_private_marker {
        return Err(ProjectionError::term(format!(
            "invalid language tag {tag:?}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(8, 8_192, 32_768, 65_536, 8).expect("limits")
    }

    #[test]
    fn nested_directional_term_round_trips_view_json_and_value() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/s");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_literal(RdfLiteral {
            lexical_form: "marhaba".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        let triple = builder.intern_triple(s, p, o);
        let dataset = builder.freeze().expect("freeze");

        let term = ProjectionTerm::from_view(&dataset, triple, limits()).expect("project");
        let json = term.to_canonical_json(limits()).expect("JSON");
        let reparsed = ProjectionTerm::from_canonical_json(&json, limits()).expect("parse");
        assert_eq!(reparsed, term);
        let value = term.to_term_value(limits()).expect("kernel value");
        assert_eq!(
            ProjectionTerm::from_term_value(&value, limits()).expect("carrier value"),
            term
        );
    }

    #[test]
    fn canonical_json_rejects_whitespace_and_invalid_predicate() {
        let valid = ProjectionTerm::Iri {
            value: "http://example.org/a".to_owned(),
        };
        let mut padded = valid.to_canonical_json(limits()).expect("JSON");
        padded.push(b'\n');
        assert!(ProjectionTerm::from_canonical_json(&padded, limits()).is_err());

        let invalid = br#"{"kind":"triple","subject":{"kind":"iri","value":"http://example.org/s"},"predicate":{"kind":"blank","label":"p","scope":0},"object":{"kind":"iri","value":"http://example.org/o"}}"#;
        assert!(ProjectionTerm::from_canonical_json(invalid, limits()).is_err());
    }

    #[test]
    fn depth_limit_is_enforced() {
        let leaf = ProjectionTerm::Iri {
            value: "http://example.org/x".to_owned(),
        };
        let depth_one = ProjectionTerm::Triple {
            subject: Box::new(ProjectionTerm::Triple {
                subject: Box::new(leaf.clone()),
                predicate: Box::new(leaf.clone()),
                object: Box::new(leaf.clone()),
            }),
            predicate: Box::new(leaf.clone()),
            object: Box::new(leaf.clone()),
        };
        let depth_two = ProjectionTerm::Triple {
            subject: Box::new(depth_one.clone()),
            predicate: Box::new(leaf.clone()),
            object: Box::new(leaf),
        };
        let shallow = ProjectionLimits::new(8, 8_192, 32_768, 65_536, 1).expect("limits");
        assert!(depth_one.to_canonical_json(shallow).is_ok());
        assert!(depth_one.to_term_value(shallow).is_ok());
        assert!(depth_two.to_canonical_json(shallow).is_err());
        assert!(depth_two.to_term_value(shallow).is_err());
    }

    #[test]
    fn invalid_kernel_values_and_oversized_json_fail_closed() {
        let invalid_predicate = TermValue::Triple {
            s: Box::new(TermValue::iri("http://example.org/s")),
            p: Box::new(TermValue::blank("predicate")),
            o: Box::new(TermValue::iri("http://example.org/o")),
        };
        assert!(ProjectionTerm::from_term_value(&invalid_predicate, limits()).is_err());

        let invalid_language = ProjectionTerm::Literal {
            lexical: "hello".to_owned(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some("not a tag".to_owned()),
            direction: None,
        };
        assert!(invalid_language.validate(limits()).is_err());

        let tiny = ProjectionLimits::new(1, 16, 16, 1_536, 8).expect("limits");
        let iri = ProjectionTerm::Iri {
            value: "http://example.org/long".to_owned(),
        };
        assert!(iri.to_canonical_json(tiny).is_err());
        let oversized = vec![b'x'; 17];
        let error = ProjectionTerm::from_canonical_json(&oversized, tiny)
            .expect_err("size limit precedes parsing");
        assert_eq!(
            error.kind(),
            super::super::ProjectionErrorKind::ResourceLimit
        );

        let literal_subject = ProjectionTerm::Triple {
            subject: Box::new(ProjectionTerm::Literal {
                lexical: "bad".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                language: None,
                direction: None,
            }),
            predicate: Box::new(ProjectionTerm::Iri {
                value: "http://example.org/p".to_owned(),
            }),
            object: Box::new(ProjectionTerm::Iri {
                value: "http://example.org/o".to_owned(),
            }),
        };
        assert!(literal_subject.validate(limits()).is_err());
    }
}

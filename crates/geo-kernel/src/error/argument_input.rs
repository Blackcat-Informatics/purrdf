// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original scalar argument terms and native integer parser refusals.

use purrdf_core::TermValue;

/// The original scalar argument requirement that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgumentInputReason {
    /// Units require an IRI, an untagged anyURI or an untagged string.
    Unit,
    /// An integer argument requires an untagged literal.
    IntegerLiteral,
    /// The literal's datatype must belong to the XSD integer family.
    IntegerDatatype,
    /// The original integer lexical form cannot be represented as an index.
    IntegerLexical(core::num::ParseIntError),
    /// The argument requires an untagged XSD string.
    String,
}

impl core::hash::Hash for ArgumentInputReason {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        if let Self::IntegerLexical(error) = self {
            core::mem::discriminant(error.kind()).hash(state);
        }
    }
}

impl purrdf_hash::error::StructuredError for ArgumentInputReason {
    purrdf_hash::error_visit! {
        Unit => "argument-input-unit" {},
        IntegerLiteral => "argument-input-integer-literal" {},
        IntegerDatatype => "argument-input-integer-datatype" {},
        IntegerLexical(cause) => "argument-input-integer-lexical" { "cause" => Cause(cause) },
        String => "argument-input-string" {},
    }
}

/// An original refused argument copied under its immutable native owner.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArgumentInputError {
    reason: ArgumentInputReason,
    value: TermValue,
}

impl purrdf_hash::error::StructuredError for ArgumentInputError {
    purrdf_hash::error_visit! {
        @record self { reason, value } => "geo-argument-input" {
            "reason" => Record(&self.reason), "value" => Term(&self.value)
        }
    }
}

impl ArgumentInputError {
    /// Retain the original term and requirement through native admission.
    /// Native storage failure replaces no successfully published diagnostic.
    #[must_use]
    pub fn capture(
        value: &TermValue,
        reason: ArgumentInputReason,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> super::GeoError {
        let mut progress = crate::context::WorkProgress::integer(observer);
        let result = crate::context::Retained::produce(
            &mut progress,
            |memory| {
                let value = value
                    .try_clone_with_memory(memory, |_| {
                        Ok::<_, purrdf_lex::allocation::StorageError>(())
                    })
                    .map_err(|error| match error {
                        purrdf_core::OwnedTermFoldError::Visitor(error)
                        | purrdf_core::OwnedTermFoldError::Storage(error) => {
                            super::GeoError::Storage(error)
                        }
                    })?;
                Ok(Self { reason, value })
            },
            |_, _| Ok(()),
        );
        match result {
            Ok(cause) => super::GeoError::ArgumentInput(cause),
            Err(error) => error,
        }
    }
}

impl core::fmt::Display for ArgumentInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let original = crate::carrier::term_description_display(&self.value);
        match &self.reason {
            ArgumentInputReason::Unit => write!(
                f,
                "a units argument is a unit IRI (as an IRI term, an xsd:anyURI or an xsd:string) and {original} arrived"
            ),
            ArgumentInputReason::IntegerLiteral | ArgumentInputReason::IntegerDatatype => {
                write!(f, "an integer argument was expected and {original} arrived")
            }
            ArgumentInputReason::IntegerLexical(_) => match &self.value {
                TermValue::Literal { lexical_form, .. } => write!(
                    f,
                    "{lexical_form:?} is not an integer purrdf-geo can index with"
                ),
                _ => write!(f, "an integer argument was expected and {original} arrived"),
            },
            ArgumentInputReason::String => write!(
                f,
                "an xsd:string argument was expected and {original} arrived"
            ),
        }
    }
}
impl std::error::Error for ArgumentInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.reason {
            ArgumentInputReason::IntegerLexical(error) => Some(error),
            _ => None,
        }
    }
}

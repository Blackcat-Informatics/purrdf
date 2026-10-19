// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native schema keyword and identifier requirements.

use purrdf_lex::json::Value;

/// The semantic requirement a schema keyword must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordRequirement {
    /// A schema is an object or Boolean.
    Schema,
    /// A keyword is a string.
    String,
    /// A combinator has at least one schema.
    NonEmptySchemas,
    /// A map contains schemas.
    ObjectSchemas,
    /// A recursive reference is the sole 2019-09 spelling.
    RecursiveReference,
    /// A keyword is an array.
    Array,
    /// A divisor is strictly positive.
    Positive,
    /// A keyword is Boolean.
    Boolean,
    /// A keyword is an object.
    Object,
    /// A keyword is numeric.
    Number,
    /// A cardinality is an unsigned integer.
    NonNegativeInteger,
    /// A keyword is an array of strings.
    Strings,
    /// A dependency map contains arrays of strings.
    ObjectStringArrays,
    /// Every name is a declared JSON Schema type.
    TypeNames,
    /// A vocabulary declaration is an object.
    VocabularyObject,
}

impl KeywordRequirement {
    const fn description(self) -> &'static str {
        match self {
            Self::Schema => "a schema must be an object or a boolean",
            Self::String => "must be a string",
            Self::NonEmptySchemas => "must be a non-empty array of schemas",
            Self::ObjectSchemas => "must be an object of schemas",
            Self::RecursiveReference => {
                "the only value 2019-09 defines for `$recursiveRef` is \"#\""
            }
            Self::Array => "must be an array",
            Self::Positive => "must be greater than 0",
            Self::Boolean => "must be a boolean",
            Self::Object => "must be an object",
            Self::Number => "must be a number",
            Self::NonNegativeInteger => "must be a non-negative integer",
            Self::Strings => "must be an array of strings",
            Self::ObjectStringArrays => "must be an object of string arrays",
            Self::TypeNames => "must name JSON Schema types",
            Self::VocabularyObject => "`$vocabulary` must be an object",
        }
    }
}

impl purrdf_hash::error::StructuredError for KeywordRequirement {
    purrdf_hash::error_visit! {
        Schema => "schema-requirement-schema" {},
        String => "schema-requirement-string" {},
        NonEmptySchemas => "schema-requirement-nonempty-schemas" {},
        ObjectSchemas => "schema-requirement-object-schemas" {},
        RecursiveReference => "schema-requirement-recursive-reference" {},
        Array => "schema-requirement-array" {},
        Positive => "schema-requirement-positive" {},
        Boolean => "schema-requirement-boolean" {},
        Object => "schema-requirement-object" {},
        Number => "schema-requirement-number" {},
        NonNegativeInteger => "schema-requirement-nonnegative-integer" {},
        Strings => "schema-requirement-strings" {},
        ObjectStringArrays => "schema-requirement-object-string-arrays" {},
        TypeNames => "schema-requirement-type-names" {},
        VocabularyObject => "schema-requirement-vocabulary-object" {},
    }
}

/// A rejected native keyword value or duplicate object member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordRefusal {
    /// An original value violates the keyword requirement.
    Requirement {
        /// Required semantic shape or range.
        expected: KeywordRequirement,
        /// Original rejected value.
        actual: Value,
    },
    /// Object member names must be unique for schema evaluation.
    RepeatedMember {
        /// Original repeated member name.
        name: String,
    },
}

impl purrdf_hash::error::StructuredError for KeywordRefusal {
    purrdf_hash::error_visit! {
        Requirement { expected, actual } => "schema-keyword-requirement" { "expected" => Record(expected), "actual" => Record(actual) },
        RepeatedMember { name } => "schema-keyword-repeated-member" { "name" => Text(name) },
    }
}

impl core::fmt::Display for KeywordRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Requirement { expected, .. } => f.write_str(expected.description()),
            Self::RepeatedMember { name } => {
                write!(f, "the object repeats the member name {name:?}")
            }
        }
    }
}

impl std::error::Error for KeywordRefusal {}

/// The exact identifier declaration rejected during registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierRefusal {
    /// An anchor name was already declared in the same resource.
    RepeatedAnchor {
        /// Original anchor name.
        name: String,
    },
    /// The anchor value is not a valid anchor name.
    Anchor {
        /// Original rejected anchor value.
        value: Value,
    },
    /// An identifier must be a string.
    IdType {
        /// Original rejected identifier value.
        value: Value,
    },
    /// A draft-07 identifier fragment must be a plain name.
    PlainFragment {
        /// Original identifier spelling.
        id: String,
        /// Original percent-decoding cause, absent for an invalid decoded name.
        cause: Option<purrdf_iri::percent::PercentError>,
    },
    /// The selected dialect forbids a nonempty identifier fragment.
    NonEmptyFragment {
        /// Original identifier spelling.
        id: String,
    },
}

impl purrdf_hash::error::StructuredError for IdentifierRefusal {
    purrdf_hash::error_visit! {
        RepeatedAnchor { name } => "schema-identifier-repeated-anchor" { "name" => Text(name) },
        Anchor { value } => "schema-identifier-anchor" { "value" => Record(value) },
        IdType { value } => "schema-identifier-type" { "value" => Record(value) },
        PlainFragment { id, cause } => "schema-identifier-plain-fragment" {
            "id" => Iri(id),
            "cause" => Value(cause.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |value| purrdf_hash::error::ErrorValue::Cause(value)))
        },
        NonEmptyFragment { id } => "schema-identifier-nonempty-fragment" { "id" => Iri(id) },
    }
}

impl core::fmt::Display for IdentifierRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RepeatedAnchor { name } => {
                write!(f, "the anchor {name:?} is declared twice in one resource")
            }
            Self::Anchor { value } => write!(f, "{value} is not an anchor name"),
            Self::IdType { .. } => f.write_str("`$id` must be a string"),
            Self::PlainFragment { id, .. } => {
                write!(f, "`$id` {id:?} has a fragment that is not a plain name")
            }
            Self::NonEmptyFragment { id } => {
                write!(f, "`$id` {id:?} may not carry a non-empty fragment")
            }
        }
    }
}

impl std::error::Error for IdentifierRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::PlainFragment {
            cause: Some(cause), ..
        } = self
        {
            Some(cause)
        } else {
            None
        }
    }
}

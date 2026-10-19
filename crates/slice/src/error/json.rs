// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native slice-record shape refusals over the original JSON value.

use purrdf_lex::json::Value;

/// Required shape of a slice record member.
#[derive(Debug)]
pub enum JsonMemberKind {
    /// One string.
    String,
    /// A string or explicit null.
    OptionalString,
    /// An array containing only strings.
    Strings,
    /// An array of exact unsigned byte values.
    Bytes,
}

impl JsonMemberKind {
    const fn description(&self) -> &'static str {
        match self {
            Self::String => "a string",
            Self::OptionalString => "a string or null",
            Self::Strings => "an array of strings",
            Self::Bytes => "an array of bytes",
        }
    }
}

impl purrdf_hash::error::StructuredError for JsonMemberKind {
    purrdf_hash::error_visit! {
        String => "slice-json-string" {},
        OptionalString => "slice-json-optional-string" {},
        Strings => "slice-json-strings" {},
        Bytes => "slice-json-bytes" {},
    }
}

/// A rejected record, member, or variant with its original value.
#[derive(Debug)]
pub enum JsonRefusal {
    /// A record requires an object.
    Object {
        /// Record role requested by the decoder.
        what: String,
        /// Original nonobject value.
        value: Value,
    },
    /// A required member is absent.
    Missing {
        /// Original requested member name.
        name: String,
    },
    /// An existing member has an incompatible value.
    Member {
        /// Original member name.
        name: String,
        /// Required shape.
        expected: JsonMemberKind,
        /// Original member value.
        value: Value,
    },
    /// A newtype variant requires exactly one string member.
    Variant {
        /// Required variant name.
        variant: String,
        /// Original rejected variant value.
        value: Value,
    },
    /// An enum spelling is not a declared unit variant.
    Unknown {
        /// Enum role being decoded.
        category: &'static str,
        /// Original rejected spelling.
        name: String,
    },
}

impl purrdf_hash::error::StructuredError for JsonRefusal {
    purrdf_hash::error_visit! {
        Object { what, value } => "slice-json-object" { "what" => Text(what), "value" => Record(value) },
        Missing { name } => "slice-json-missing-member" { "name" => Text(name) },
        Member { name, expected, value } => "slice-json-member" { "name" => Text(name), "expected" => Record(expected), "value" => Record(value) },
        Variant { variant, value } => "slice-json-variant" { "variant" => Text(variant), "value" => Record(value) },
        Unknown { category, name } => "slice-json-unknown-variant" { "category" => Code(category), "name" => Text(name) },
    }
}

impl core::fmt::Display for JsonRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Object { what, .. } => write!(f, "{what} must be a JSON object"),
            Self::Missing { name } => write!(f, "missing member `{name}`"),
            Self::Member { name, expected, .. } => {
                write!(f, "member `{name}` must be {}", expected.description())
            }
            Self::Variant { variant, .. } => write!(
                f,
                "expected a variant name or a one-member `{variant}` object"
            ),
            Self::Unknown { category, name } => write!(f, "unknown {category} `{name}`"),
        }
    }
}

impl std::error::Error for JsonRefusal {}

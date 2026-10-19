// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native profile admission reasons, separate from their human presentation.

use crate::Params;
use purrdf_core::{EmbeddingError, IndexId, TlvWireType};

/// A coordinate's exact identity bytes or wire counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateValue {
    /// Content-addressed identity.
    Digest([u8; 32]),
    /// Dimension or use-role wire code.
    Counter(u32),
}

impl purrdf_hash::error::StructuredError for CoordinateValue {
    purrdf_hash::error_visit! {
        Digest(value) => "hnsw-coordinate-digest" { "value" => Bytes(value) },
        Counter(value) => "hnsw-coordinate-counter" { "value" => Unsigned(u128::from(*value)) },
    }
}

/// The native values at a rejected guard or parameter field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRefusal {
    /// Canonical parameter framing failed.
    ParameterBlock(EmbeddingError),
    /// A required parameter is not critical.
    ParameterCritical {
        /// Parameter tag.
        tag: u16,
        /// Original wire type.
        wire_type: TlvWireType,
        /// Original value bytes.
        value: Vec<u8>,
    },
    /// A parameter does not have the unsigned 64-bit wire type.
    ParameterType {
        /// Parameter tag.
        tag: u16,
        /// Actual wire type.
        actual: TlvWireType,
    },
    /// A parameter is not eight bytes long.
    ParameterLength {
        /// Parameter tag.
        tag: u16,
        /// Original value bytes.
        actual: Vec<u8>,
    },
    /// Unknown critical parameter.
    UnknownParameter {
        /// Original parameter tag.
        tag: u16,
        /// Original parameter value.
        value: u64,
    },
    /// Missing required parameter.
    MissingParameter {
        /// Native parameter name.
        name: &'static str,
    },
    /// Parameter exceeds this platform's index range.
    ParameterRange {
        /// Native parameter name.
        name: &'static str,
        /// Original supplied value.
        value: u64,
        /// Maximum platform index.
        maximum: usize,
    },
    /// Guard field has the wrong wire type.
    WireType {
        /// Native field name.
        field: &'static str,
        /// Original wire type.
        actual: TlvWireType,
        /// Required wire type.
        expected: TlvWireType,
        /// Existing human name for the required type.
        description: &'static str,
    },
    /// Unknown implementation identifier.
    Implementation {
        /// Original implementation identifier.
        actual: String,
    },
    /// A textual declaration disagrees with the profile.
    TextMismatch {
        /// Native field name.
        field: &'static str,
        /// Original declaration.
        actual: String,
        /// Required declaration.
        expected: &'static str,
    },
    /// The evidence revision does not belong to this implementation.
    Revision {
        /// Original implementation identifier.
        identifier: String,
        /// Original revision bytes, or their absence.
        revision: Option<Vec<u8>>,
        /// Other published implementation owning these bytes, if recognized.
        other_implementation: Option<&'static str>,
        /// That other implementation's arithmetic, if recognized.
        other_arithmetic: Option<&'static str>,
    },
    /// A use-role field is not four bytes long.
    RoleLength {
        /// Native field name.
        field: &'static str,
        /// Original value bytes.
        actual: Vec<u8>,
    },
    /// Unknown role from the profile validation path.
    UnknownIndexUseRole(u32),
    /// Unknown role from the coordinate validation path.
    UnknownUseRole(u32),
    /// Approximation flag disagrees with the profile.
    Approximation {
        /// Original flag bytes.
        actual: Vec<u8>,
    },
    /// Vector-transformation flag disagrees with the profile.
    Transforms {
        /// Original flag bytes.
        actual: Vec<u8>,
    },
    /// A guard coordinate disagrees with the searched matrix.
    Coordinate {
        /// Original guard identity.
        guard: IndexId,
        /// Coordinate name.
        field: &'static str,
        /// Original declared coordinate.
        actual: CoordinateValue,
        /// Searched coordinate.
        expected: CoordinateValue,
    },
    /// A guard names another arithmetic.
    Arithmetic {
        /// Original implementation identifier.
        implementation: &'static str,
        /// Declared arithmetic.
        actual: &'static str,
        /// Arithmetic used to load the index.
        expected: &'static str,
    },
    /// Guard revision and image dispatch codes disagree.
    Codes {
        /// Codes the revision publishes.
        expected: Vec<u32>,
        /// Recorded image code.
        actual: u32,
    },
    /// Embedded and declared parameters disagree.
    Parameters {
        /// Guard's declared parameters.
        expected: Params,
        /// Image's embedded parameters.
        actual: Params,
    },
    /// A required guard field is missing.
    MissingField {
        /// Required wire tag.
        tag: u16,
        /// Native field name.
        field: &'static str,
    },
    /// A declared UTF-8 field contains invalid bytes.
    Utf8 {
        /// Native field name.
        field: &'static str,
        /// Original value bytes.
        actual: Vec<u8>,
        /// Original standard decoding error.
        cause: core::str::Utf8Error,
    },
}

impl purrdf_hash::error::StructuredError for ProfileRefusal {
    purrdf_hash::error_visit! {
        ParameterBlock(cause) => "hnsw-profile-parameter-block" { "cause" => Cause(cause) },
        ParameterCritical { tag, wire_type, value } => "hnsw-profile-parameter-critical" {
            "tag" => Unsigned(u128::from(*tag)), "wire_type" => Unsigned(u128::from(*wire_type as u8)), "value" => Bytes(value), "critical" => Boolean(false),
        },
        ParameterType { tag, actual } => "hnsw-profile-parameter-type" {
            "tag" => Unsigned(u128::from(*tag)), "actual" => Unsigned(u128::from(*actual as u8)), "expected" => Unsigned(u128::from(TlvWireType::U64 as u8)),
        },
        ParameterLength { tag, actual } => "hnsw-profile-parameter-length" { "tag" => Unsigned(u128::from(*tag)), "actual" => Bytes(actual), "expected" => ByteCount(8) },
        UnknownParameter { tag, value } => "hnsw-profile-unknown-parameter" { "tag" => Unsigned(u128::from(*tag)), "value" => Unsigned(u128::from(*value)) },
        MissingParameter { name } => "hnsw-profile-missing-parameter" { "name" => Code(name) },
        ParameterRange { name, value, maximum } => "hnsw-profile-parameter-range" { "name" => Code(name), "value" => Unsigned(u128::from(*value)), "maximum" => Unsigned(*maximum as u128) },
        WireType { field, actual, expected, description } => "hnsw-profile-wire-type" {
            "field" => Code(field), "actual" => Unsigned(u128::from(*actual as u8)), "expected" => Unsigned(u128::from(*expected as u8)), "description" => Text(description),
        },
        Implementation { actual } => "hnsw-profile-implementation" { "actual" => Text(actual), "exact" => Code(crate::IMPLEMENTATION_ID), "reassociated" => Code(crate::IMPLEMENTATION_ID_REASSOCIATED) },
        TextMismatch { field, actual, expected } => "hnsw-profile-text-mismatch" { "field" => Code(field), "actual" => Text(actual), "expected" => Text(expected) },
        Revision { identifier, revision, other_implementation, other_arithmetic } => "hnsw-profile-revision" {
            "identifier" => Text(identifier),
            "revision" => Value(revision.as_deref().map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Bytes)),
            "other_implementation" => Value(other_implementation.map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Code)),
            "other_arithmetic" => Value(other_arithmetic.map_or(purrdf_hash::error::ErrorValue::Absent, purrdf_hash::error::ErrorValue::Code)),
        },
        RoleLength { field, actual } => "hnsw-profile-role-length" { "field" => Code(field), "actual" => Bytes(actual), "expected" => ByteCount(4) },
        UnknownIndexUseRole(value) => "hnsw-profile-unknown-index-use-role" { "value" => Unsigned(u128::from(*value)) },
        UnknownUseRole(value) => "hnsw-profile-unknown-use-role" { "value" => Unsigned(u128::from(*value)) },
        Approximation { actual } => "hnsw-profile-approximation" { "actual" => Bytes(actual), "expected" => Bytes(&[1]) },
        Transforms { actual } => "hnsw-profile-transforms" { "actual" => Bytes(actual), "expected" => Bytes(&[0]) },
        Coordinate { guard, field, actual, expected } => "hnsw-profile-coordinate" { "guard" => Record(guard), "field" => Code(field), "actual" => Record(actual), "expected" => Record(expected) },
        Arithmetic { implementation, actual, expected } => "hnsw-profile-arithmetic" { "implementation" => Code(implementation), "actual" => Code(actual), "expected" => Code(expected) },
        Codes { expected, actual } => "hnsw-profile-codes" { "expected" => List(expected.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(u128::from(*value)))), "actual" => Unsigned(u128::from(*actual)) },
        Parameters { expected, actual } => "hnsw-profile-parameters" { "expected" => Record(expected), "actual" => Record(actual) },
        MissingField { tag, field } => "hnsw-profile-missing-field" { "tag" => Unsigned(u128::from(*tag)), "field" => Code(field) },
        Utf8 { field, actual, cause } => "hnsw-profile-utf8" { "field" => Code(field), "actual" => Bytes(actual), "cause" => Cause(cause) },
    }
}

impl core::fmt::Display for ProfileRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ParameterBlock(cause) => write!(f, "the parameter block is not a canonical TLV block: {cause}"),
            Self::ParameterCritical { tag, wire_type: _, value: _ } => write!(f, "parameter {tag} is not critical"),
            Self::ParameterType { tag, actual } => write!(f, "parameter {tag} has wire type {actual:?}, not a u64"),
            Self::ParameterLength { tag, actual: _ } => write!(f, "parameter {tag} is not eight bytes"),
            Self::UnknownParameter { tag, value: _ } => write!(f, "unknown critical parameter tag {tag}"),
            Self::MissingParameter { name } => write!(f, "the parameter block is missing {name}"),
            Self::ParameterRange { name, value, maximum: _ } => write!(f, "parameter {name} = {value} does not fit usize"),
            Self::WireType { field, actual: _, expected: _, description } => write!(f, "{field} is not {description}"),
            Self::Implementation { actual } => write!(f, "the implementation identifier is `{actual}`, not `{}` or `{}`", crate::IMPLEMENTATION_ID, crate::IMPLEMENTATION_ID_REASSOCIATED),
            Self::TextMismatch { field, actual, expected } => write!(f, "{field} is `{actual}`, not `{expected}`"),
            Self::Revision { identifier, revision: _, other_implementation, other_arithmetic } => match (other_implementation, other_arithmetic) {
                (Some(implementation), Some(arithmetic)) => write!(f, "the implementation `{identifier}` carries the evidence revision `{implementation}` publishes for the {arithmetic} arithmetic; an implementation must publish its own arithmetic's evidence, so the pairing is refused"),
                _ => f.write_str("the implementation evidence revision is not the profile's approximation statement; an HNSW guard must publish what it does not promise"),
            },
            Self::RoleLength { field, actual: _ } => write!(f, "{field} is not four bytes"),
            Self::UnknownIndexUseRole(value) => write!(f, "unknown index use role {value}"),
            Self::UnknownUseRole(value) => write!(f, "the use role is {value}, which PURREMB does not define"),
            Self::Approximation { actual: _ } => f.write_str("the loss contract is not approximate"),
            Self::Transforms { actual: _ } => f.write_str("the loss contract claims an HNSW payload transforms vectors; this profile stores no vectors, so the claim is false"),
            Self::Coordinate { guard, field, actual: _, expected: _ } => write!(f, "guard {guard} is stale or substituted: its {field} does not name the searched effective matrix"),
            Self::Arithmetic { implementation, actual, expected } => write!(f, "the guard names the `{implementation}` implementation, whose distances are computed under the {actual} arithmetic, and it is being loaded as an index computed under {expected}"),
            Self::Codes { expected, actual } => write!(f, "the guard's evidence revision names the dispatch path of arithmetic code(s) {expected:?}, and the payload records code {actual} ({}); the guard and the payload describe two different compilations", crate::profile::path_label(*actual)),
            Self::Parameters { expected: _, actual: _ } => f.write_str("the guard's parameter block and the payload's embedded parameters disagree"),
            Self::MissingField { tag: _, field } => write!(f, "{field} is missing"),
            Self::Utf8 { field, actual: _, cause: _ } => write!(f, "{field} is not valid UTF-8"),
        }
    }
}

purrdf_lex::error_source!(ProfileRefusal => {
    Self::ParameterBlock(cause) => cause,
    Self::Utf8 { cause, .. } => cause,
});

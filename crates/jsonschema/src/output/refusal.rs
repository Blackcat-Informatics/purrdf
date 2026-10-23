// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native JSON Schema verdict reasons, rendered only at a presentation boundary.

use purrdf_lex::json::Value;

/// A failed JSON Schema keyword and its original operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationRefusal {
    /// A Boolean false schema.
    FalseSchema,
    /// A referenced schema failed.
    Reference {
        /// Original absolute target.
        target: String,
    },
    /// A conditional branch failed.
    Branch {
        /// Original branch name.
        name: &'static str,
    },
    /// Conjunctive subschemas failed.
    AllOf {
        /// Original subschema indices.
        failed: Vec<usize>,
    },
    /// No disjunct passed.
    NoSubschema,
    /// Multiple exclusive disjuncts passed.
    OneOf {
        /// Original passing subschema indices.
        passed: Vec<usize>,
    },
    /// A negated subschema passed.
    Not,
    /// Dependent schemas failed.
    DependentSchemas {
        /// Original property names.
        failed: Vec<String>,
    },
    /// Tuple item schemas failed.
    PrefixItems {
        /// Original item indices.
        failed: Vec<usize>,
    },
    /// Items failed their common schema.
    Items {
        /// Original item indices.
        failed: Vec<usize>,
    },
    /// Too few contains matches.
    ContainsMinimum {
        /// Actual match count.
        found: u64,
        /// Required count.
        minimum: u64,
    },
    /// Too many contains matches.
    ContainsMaximum {
        /// Actual match count.
        found: u64,
        /// Allowed count.
        maximum: u64,
    },
    /// Named properties failed.
    Properties {
        /// Original property names.
        failed: Vec<String>,
    },
    /// Pattern-selected properties failed.
    PatternProperties {
        /// Original property names.
        failed: Vec<String>,
    },
    /// Additional properties failed.
    AdditionalProperties {
        /// Original property names.
        failed: Vec<String>,
    },
    /// Property names failed their schema.
    PropertyNames {
        /// Original property names.
        failed: Vec<String>,
    },
    /// Unevaluated items failed.
    UnevaluatedItems {
        /// Original item indices.
        failed: Vec<usize>,
    },
    /// Unevaluated properties failed.
    UnevaluatedProperties {
        /// Original property names.
        failed: Vec<String>,
    },
    /// A reference re-entered the same instance without progress.
    ReferenceCycle {
        /// Original target location.
        target: String,
    },
    /// A reference-chain bound stopped evaluation.
    ReferenceChain {
        /// Configured maximum chain.
        maximum: usize,
    },
    /// The instance has no permitted type.
    Type {
        /// Permitted type names.
        expected: Vec<&'static str>,
        /// Original instance type.
        actual: &'static str,
    },
    /// The instance is absent from the enumerated values.
    Enum,
    /// The instance differs from a constant.
    Const {
        /// Original expected value.
        value: Value,
    },
    /// A number is not a multiple of the specified divisor.
    MultipleOf {
        /// Original divisor spelling/value.
        divisor: Value,
    },
    /// A number exceeds its maximum.
    Maximum {
        /// Original bound.
        bound: Value,
    },
    /// A number is not below its exclusive maximum.
    ExclusiveMaximum {
        /// Original bound.
        bound: Value,
    },
    /// A number is below its minimum.
    Minimum {
        /// Original bound.
        bound: Value,
    },
    /// A number is not above its exclusive minimum.
    ExclusiveMinimum {
        /// Original bound.
        bound: Value,
    },
    /// A string exceeds its character count bound.
    MaxLength {
        /// Actual character count.
        actual: usize,
        /// Original maximum.
        limit: u64,
    },
    /// A string is shorter than its character count bound.
    MinLength {
        /// Actual character count.
        actual: usize,
        /// Original minimum.
        limit: u64,
    },
    /// A string fails its pattern.
    Pattern {
        /// Original pattern spelling.
        pattern: String,
    },
    /// An array exceeds its item count bound.
    MaxItems {
        /// Actual count.
        actual: usize,
        /// Original maximum.
        limit: u64,
    },
    /// An array has fewer items than required.
    MinItems {
        /// Actual count.
        actual: usize,
        /// Original minimum.
        limit: u64,
    },
    /// Two array items are equal.
    UniqueItems {
        /// First original index.
        first: usize,
        /// Second original index.
        second: usize,
    },
    /// An object exceeds its property count bound.
    MaxProperties {
        /// Actual count.
        actual: usize,
        /// Original maximum.
        limit: u64,
    },
    /// An object has fewer properties than required.
    MinProperties {
        /// Actual count.
        actual: usize,
        /// Original minimum.
        limit: u64,
    },
    /// Required properties are absent.
    Required {
        /// Original missing names.
        missing: Vec<String>,
    },
    /// A present property requires an absent property.
    DependentRequired {
        /// Original triggering property.
        property: String,
        /// Original absent property.
        missing: String,
    },
    /// A string fails an asserted format.
    Format {
        /// Original format name.
        name: String,
    },
    /// A content assertion failed with its original native cause.
    Content(crate::content::ContentRefusal),
}

impl purrdf_hash::error::StructuredError for ValidationRefusal {
    purrdf_hash::error_visit! {
        FalseSchema => "schema-false" {},
        Reference { target } => "schema-reference-failed" { "target" => Iri(target) },
        Branch { name } => "schema-branch-failed" { "name" => Code(name) },
        AllOf { failed } => "schema-all-of-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        NoSubschema => "schema-no-subschema-passed" {},
        OneOf { passed } => "schema-one-of-failed" { "passed" => List(passed.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        Not => "schema-not-failed" {},
        DependentSchemas { failed } => "schema-dependent-schemas-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        PrefixItems { failed } => "schema-prefix-items-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        Items { failed } => "schema-items-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        ContainsMinimum { found, minimum } => "schema-contains-minimum" { "found" => Unsigned(u128::from(*found)), "minimum" => Unsigned(u128::from(*minimum)) },
        ContainsMaximum { found, maximum } => "schema-contains-maximum" { "found" => Unsigned(u128::from(*found)), "maximum" => Unsigned(u128::from(*maximum)) },
        Properties { failed } => "schema-properties-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        PatternProperties { failed } => "schema-pattern-properties-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        AdditionalProperties { failed } => "schema-additional-properties-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        PropertyNames { failed } => "schema-property-names-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        UnevaluatedItems { failed } => "schema-unevaluated-items-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Unsigned(*value as u128))) },
        UnevaluatedProperties { failed } => "schema-unevaluated-properties-failed" { "failed" => List(failed.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        ReferenceCycle { target } => "schema-reference-cycle" { "target" => Iri(target) },
        ReferenceChain { maximum } => "schema-reference-chain" { "maximum" => Unsigned(*maximum as u128) },
        Type { expected, actual } => "schema-type-mismatch" { "expected" => List(expected.iter().map(|value| purrdf_hash::error::ErrorValue::Code(value))), "actual" => Code(actual) },
        Enum => "schema-enum-mismatch" {},
        Const { value } => "schema-const-mismatch" { "value" => Record(value) },
        MultipleOf { divisor } => "schema-multiple-of-mismatch" { "divisor" => Record(divisor) },
        Maximum { bound } => "schema-maximum" { "bound" => Record(bound) },
        ExclusiveMaximum { bound } => "schema-exclusive-maximum" { "bound" => Record(bound) },
        Minimum { bound } => "schema-minimum" { "bound" => Record(bound) },
        ExclusiveMinimum { bound } => "schema-exclusive-minimum" { "bound" => Record(bound) },
        MaxLength { actual, limit } => "schema-max-length" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        MinLength { actual, limit } => "schema-min-length" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        Pattern { pattern } => "schema-pattern-mismatch" { "pattern" => Text(pattern) },
        MaxItems { actual, limit } => "schema-max-items" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        MinItems { actual, limit } => "schema-min-items" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        UniqueItems { first, second } => "schema-duplicate-items" { "first" => Unsigned(*first as u128), "second" => Unsigned(*second as u128) },
        MaxProperties { actual, limit } => "schema-max-properties" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        MinProperties { actual, limit } => "schema-min-properties" { "actual" => Unsigned(*actual as u128), "limit" => Unsigned(u128::from(*limit)) },
        Required { missing } => "schema-required-properties" { "missing" => List(missing.iter().map(|value| purrdf_hash::error::ErrorValue::Text(value))) },
        DependentRequired { property, missing } => "schema-dependent-required" { "property" => Text(property), "missing" => Text(missing) },
        Format { name } => "schema-format-mismatch" { "name" => Code(name) },
        Content(cause) => "schema-content-refused" { "cause" => Cause(cause) },
    }
}

impl std::fmt::Display for ValidationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FalseSchema => f.write_str("the false schema accepts nothing"),
            Self::Reference { target } => write!(f, "the referenced schema {target} failed"),
            Self::Branch { name } => write!(f, "the `{name}` subschema failed"),
            Self::AllOf { failed } => write!(f, "subschemas {failed:?} failed"),
            Self::NoSubschema => f.write_str("no subschema passed"),
            Self::OneOf { passed } => write!(f, "more than one subschema passed: {passed:?}"),
            Self::Not => f.write_str("the value must not be valid against the subschema"),
            Self::DependentSchemas { failed } => {
                write!(f, "the dependent schemas of {failed:?} failed")
            }
            Self::PrefixItems { failed } => {
                write!(f, "items {failed:?} failed their prefix schemas")
            }
            Self::Items { failed } => write!(f, "items {failed:?} failed the items schema"),
            Self::ContainsMinimum { found, minimum } => write!(
                f,
                "{found} items match the contains schema; at least {minimum} must"
            ),
            Self::ContainsMaximum { found, maximum } => write!(
                f,
                "{found} items match the contains schema; at most {maximum} may"
            ),
            Self::Properties { failed } => write!(f, "properties {failed:?} failed their schemas"),
            Self::PatternProperties { failed } => {
                write!(f, "properties {failed:?} failed their pattern schemas")
            }
            Self::AdditionalProperties { failed } => {
                write!(f, "additional properties {failed:?} are not allowed")
            }
            Self::PropertyNames { failed } => write!(f, "property names {failed:?} are not valid"),
            Self::UnevaluatedItems { failed } => {
                write!(f, "unevaluated items {failed:?} are not allowed")
            }
            Self::UnevaluatedProperties { failed } => {
                write!(f, "unevaluated properties {failed:?} are not allowed")
            }
            Self::ReferenceCycle { target } => write!(
                f,
                "reference cycle: {target} is re-entered at the same instance location without consuming any of it"
            ),
            Self::ReferenceChain { maximum } => write!(
                f,
                "more than {maximum} references are followed in a row at one instance location"
            ),
            Self::Type { expected, actual } => {
                write!(f, "expected {}, found {actual}", expected.join(" or "))
            }
            Self::Enum => f.write_str("the value is not one of the enumerated values"),
            Self::Const { value } => write!(f, "the value must be {value}"),
            Self::MultipleOf { divisor } => write!(f, "the number is not a multiple of {divisor}"),
            Self::Maximum { bound } => write!(f, "the number exceeds the maximum {bound}"),
            Self::ExclusiveMaximum { bound } => {
                write!(f, "the number is not below the exclusive maximum {bound}")
            }
            Self::Minimum { bound } => write!(f, "the number is below the minimum {bound}"),
            Self::ExclusiveMinimum { bound } => {
                write!(f, "the number is not above the exclusive minimum {bound}")
            }
            Self::MaxLength { limit, .. } => {
                write!(f, "the string is longer than {limit} characters")
            }
            Self::MinLength { limit, .. } => {
                write!(f, "the string is shorter than {limit} characters")
            }
            Self::Pattern { pattern } => {
                write!(f, "the string does not match the pattern {pattern:?}")
            }
            Self::MaxItems { limit, .. } => write!(f, "the array has more than {limit} items"),
            Self::MinItems { limit, .. } => write!(f, "the array has fewer than {limit} items"),
            Self::UniqueItems { first, second } => {
                write!(f, "items {first} and {second} are equal")
            }
            Self::MaxProperties { limit, .. } => {
                write!(f, "the object has more than {limit} properties")
            }
            Self::MinProperties { limit, .. } => {
                write!(f, "the object has fewer than {limit} properties")
            }
            Self::Required { missing } => write!(f, "missing required properties {missing:?}"),
            Self::DependentRequired { property, missing } => {
                write!(f, "property {property:?} requires property {missing:?}")
            }
            Self::Format { name } => write!(f, "the string is not a valid {name}"),
            Self::Content(cause) => cause.fmt(f),
        }
    }
}

purrdf_lex::error_source!(ValidationRefusal => { Self::Content(cause) => cause });

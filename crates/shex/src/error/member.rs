// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native ShExJ materialization requirements and original refused values.
use core::fmt;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_lex::json::{Value, record::DecodeError};

/// A ShExJ object discriminator recognized by the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonType {
    /// The `Schema` object.
    Schema,
    /// The `ShapeAnd` object.
    ShapeAnd,
    /// The `ShapeOr` object.
    ShapeOr,
    /// The `ShapeNot` object.
    ShapeNot,
    /// The `ShapeExternal` object.
    ShapeExternal,
    /// The `NodeConstraint` object.
    NodeConstraint,
    /// The `Shape` object.
    Shape,
    /// The `EachOf` object.
    EachOf,
    /// The `OneOf` object.
    OneOf,
    /// The `TripleConstraint` object.
    TripleConstraint,
    /// The `SemAct` object.
    SemAct,
    /// The `Annotation` object.
    Annotation,
    /// The `IriStem` object.
    IriStem,
    /// The `LiteralStem` object.
    LiteralStem,
    /// The `LanguageStem` object.
    LanguageStem,
    /// The `Language` object.
    Language,
    /// The `IriStemRange` object.
    IriStemRange,
    /// The `LiteralStemRange` object.
    LiteralStemRange,
    /// The `LanguageStemRange` object.
    LanguageStemRange,
    /// The `Wildcard` object.
    Wildcard,
}
impl JsonType {
    /// The discriminator spelling in the ShExJ grammar.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Schema => "Schema",
            Self::ShapeAnd => "ShapeAnd",
            Self::ShapeOr => "ShapeOr",
            Self::ShapeNot => "ShapeNot",
            Self::ShapeExternal => "ShapeExternal",
            Self::NodeConstraint => "NodeConstraint",
            Self::Shape => "Shape",
            Self::EachOf => "EachOf",
            Self::OneOf => "OneOf",
            Self::TripleConstraint => "TripleConstraint",
            Self::SemAct => "SemAct",
            Self::Annotation => "Annotation",
            Self::IriStem => "IriStem",
            Self::LiteralStem => "LiteralStem",
            Self::LanguageStem => "LanguageStem",
            Self::Language => "Language",
            Self::IriStemRange => "IriStemRange",
            Self::LiteralStemRange => "LiteralStemRange",
            Self::LanguageStemRange => "LanguageStemRange",
            Self::Wildcard => "Wildcard",
        }
    }
}
impl StructuredError for JsonType {
    purrdf_hash::error_visit! {
        Schema => "shexj-type-schema" {},
        ShapeAnd => "shexj-type-shape-and" {},
        ShapeOr => "shexj-type-shape-or" {},
        ShapeNot => "shexj-type-shape-not" {},
        ShapeExternal => "shexj-type-shape-external" {},
        NodeConstraint => "shexj-type-node-constraint" {},
        Shape => "shexj-type-shape" {},
        EachOf => "shexj-type-each-of" {},
        OneOf => "shexj-type-one-of" {},
        TripleConstraint => "shexj-type-triple-constraint" {},
        SemAct => "shexj-type-sem-act" {},
        Annotation => "shexj-type-annotation" {},
        IriStem => "shexj-type-iri-stem" {},
        LiteralStem => "shexj-type-literal-stem" {},
        LanguageStem => "shexj-type-language-stem" {},
        Language => "shexj-type-language" {},
        IriStemRange => "shexj-type-iri-stem-range" {},
        LiteralStemRange => "shexj-type-literal-stem-range" {},
        LanguageStemRange => "shexj-type-language-stem-range" {},
        Wildcard => "shexj-type-wildcard" {},
    }
}

/// The original grammar position of a ShExJ refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonContext {
    /// A specific ShExJ object type.
    Type(JsonType),
    /// A literal record whose type member is a datatype IRI.
    ObjectLiteral,
    /// The `ShapeAnd/ShapeOr` position.
    ShapeAndOr,
    /// The `EachOf/OneOf` position.
    EachOfOneOf,
    /// The `shapeExpr` position.
    ShapeExpr,
    /// The `tripleExpr` position.
    TripleExpr,
    /// The `shapes entry` position.
    ShapeEntry,
    /// The `IriStemRange exclusion` position.
    IriExclusion,
    /// The `LiteralStemRange exclusion` position.
    LiteralExclusion,
    /// The `LanguageStemRange exclusion` position.
    LanguageExclusion,
}
impl fmt::Display for JsonContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Type(value) => value.name(),
            Self::ObjectLiteral => "ObjectLiteral",
            Self::ShapeAndOr => "ShapeAnd/ShapeOr",
            Self::EachOfOneOf => "EachOf/OneOf",
            Self::ShapeExpr => "shapeExpr",
            Self::TripleExpr => "tripleExpr",
            Self::ShapeEntry => "shapes entry",
            Self::IriExclusion => "IriStemRange exclusion",
            Self::LiteralExclusion => "LiteralStemRange exclusion",
            Self::LanguageExclusion => "LanguageStemRange exclusion",
        })
    }
}
impl StructuredError for JsonContext {
    purrdf_hash::error_visit! {
        Type(value) => "shexj-context-type" { "type" => Record(value) },
        ObjectLiteral => "shexj-context-object-literal" {},
        ShapeAndOr => "shexj-context-shape-and-or" {},
        EachOfOneOf => "shexj-context-each-of-one-of" {},
        ShapeExpr => "shexj-context-shape-expr" {},
        TripleExpr => "shexj-context-triple-expr" {},
        ShapeEntry => "shexj-context-shape-entry" {},
        IriExclusion => "shexj-context-iri-exclusion" {},
        LiteralExclusion => "shexj-context-literal-exclusion" {},
        LanguageExclusion => "shexj-context-language-exclusion" {},
    }
}

/// A declared member of the ShExJ grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonMember {
    /// The `@context` member.
    Context,
    /// The `type` member.
    Type,
    /// The `imports` member.
    Imports,
    /// The `startActs` member.
    StartActs,
    /// The `start` member.
    Start,
    /// The `shapes` member.
    Shapes,
    /// The `id` member.
    Id,
    /// The `shapeExprs` member.
    ShapeExprs,
    /// The `shapeExpr` member.
    ShapeExpr,
    /// The `nodeKind` member.
    NodeKind,
    /// The `datatype` member.
    Datatype,
    /// The `length` member.
    Length,
    /// The `minlength` member.
    Minlength,
    /// The `maxlength` member.
    Maxlength,
    /// The `pattern` member.
    Pattern,
    /// The `flags` member.
    Flags,
    /// The `mininclusive` member.
    Mininclusive,
    /// The `minexclusive` member.
    Minexclusive,
    /// The `maxinclusive` member.
    Maxinclusive,
    /// The `maxexclusive` member.
    Maxexclusive,
    /// The `totaldigits` member.
    Totaldigits,
    /// The `fractiondigits` member.
    Fractiondigits,
    /// The `values` member.
    Values,
    /// The `stem` member.
    Stem,
    /// The `exclusions` member.
    Exclusions,
    /// The `languageTag` member.
    LanguageTag,
    /// The `language` member.
    Language,
    /// The `value` member.
    Value,
    /// The `closed` member.
    Closed,
    /// The `extra` member.
    Extra,
    /// The `expression` member.
    Expression,
    /// The `semActs` member.
    SemActs,
    /// The `annotations` member.
    Annotations,
    /// The `expressions` member.
    Expressions,
    /// The `inverse` member.
    Inverse,
    /// The `predicate` member.
    Predicate,
    /// The `min` member.
    Min,
    /// The `max` member.
    Max,
    /// The `valueExpr` member.
    ValueExpr,
    /// The `name` member.
    Name,
    /// The `code` member.
    Code,
    /// The `object` member.
    Object,
}
impl JsonMember {
    /// The original member name in the grammar.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Context => "@context",
            Self::Type => "type",
            Self::Imports => "imports",
            Self::StartActs => "startActs",
            Self::Start => "start",
            Self::Shapes => "shapes",
            Self::Id => "id",
            Self::ShapeExprs => "shapeExprs",
            Self::ShapeExpr => "shapeExpr",
            Self::NodeKind => "nodeKind",
            Self::Datatype => "datatype",
            Self::Length => "length",
            Self::Minlength => "minlength",
            Self::Maxlength => "maxlength",
            Self::Pattern => "pattern",
            Self::Flags => "flags",
            Self::Mininclusive => "mininclusive",
            Self::Minexclusive => "minexclusive",
            Self::Maxinclusive => "maxinclusive",
            Self::Maxexclusive => "maxexclusive",
            Self::Totaldigits => "totaldigits",
            Self::Fractiondigits => "fractiondigits",
            Self::Values => "values",
            Self::Stem => "stem",
            Self::Exclusions => "exclusions",
            Self::LanguageTag => "languageTag",
            Self::Language => "language",
            Self::Value => "value",
            Self::Closed => "closed",
            Self::Extra => "extra",
            Self::Expression => "expression",
            Self::SemActs => "semActs",
            Self::Annotations => "annotations",
            Self::Expressions => "expressions",
            Self::Inverse => "inverse",
            Self::Predicate => "predicate",
            Self::Min => "min",
            Self::Max => "max",
            Self::ValueExpr => "valueExpr",
            Self::Name => "name",
            Self::Code => "code",
            Self::Object => "object",
        }
    }
}
impl StructuredError for JsonMember {
    purrdf_hash::error_visit! {
        Context => "shexj-member-context" {},
        Type => "shexj-member-type" {},
        Imports => "shexj-member-imports" {},
        StartActs => "shexj-member-startacts" {},
        Start => "shexj-member-start" {},
        Shapes => "shexj-member-shapes" {},
        Id => "shexj-member-id" {},
        ShapeExprs => "shexj-member-shapeexprs" {},
        ShapeExpr => "shexj-member-shapeexpr" {},
        NodeKind => "shexj-member-nodekind" {},
        Datatype => "shexj-member-datatype" {},
        Length => "shexj-member-length" {},
        Minlength => "shexj-member-minlength" {},
        Maxlength => "shexj-member-maxlength" {},
        Pattern => "shexj-member-pattern" {},
        Flags => "shexj-member-flags" {},
        Mininclusive => "shexj-member-mininclusive" {},
        Minexclusive => "shexj-member-minexclusive" {},
        Maxinclusive => "shexj-member-maxinclusive" {},
        Maxexclusive => "shexj-member-maxexclusive" {},
        Totaldigits => "shexj-member-totaldigits" {},
        Fractiondigits => "shexj-member-fractiondigits" {},
        Values => "shexj-member-values" {},
        Stem => "shexj-member-stem" {},
        Exclusions => "shexj-member-exclusions" {},
        LanguageTag => "shexj-member-languagetag" {},
        Language => "shexj-member-language" {},
        Value => "shexj-member-value" {},
        Closed => "shexj-member-closed" {},
        Extra => "shexj-member-extra" {},
        Expression => "shexj-member-expression" {},
        SemActs => "shexj-member-semacts" {},
        Annotations => "shexj-member-annotations" {},
        Expressions => "shexj-member-expressions" {},
        Inverse => "shexj-member-inverse" {},
        Predicate => "shexj-member-predicate" {},
        Min => "shexj-member-min" {},
        Max => "shexj-member-max" {},
        ValueExpr => "shexj-member-valueexpr" {},
        Name => "shexj-member-name" {},
        Code => "shexj-member-code" {},
        Object => "shexj-member-object" {},
    }
}

/// The required native value domain, before schema construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonExpectation {
    /// A string, including an explicitly supplied optional string.
    String,
    /// A JSON number.
    Number,
    /// A boolean.
    Boolean,
    /// An array.
    Array,
    /// An integer lexeme in the unsigned 64-bit domain, excluding negative zero.
    UnsignedInteger,
    /// An integer lexeme in the signed 64-bit domain.
    SignedInteger,
}
impl JsonExpectation {
    fn description(self) -> &'static str {
        match self {
            Self::String => "a string",
            Self::Number => "a number",
            Self::Boolean => "a boolean",
            Self::Array => "an array",
            Self::UnsignedInteger => "a non-negative integer",
            Self::SignedInteger => "an integer",
        }
    }
}
impl StructuredError for JsonExpectation {
    purrdf_hash::error_visit! {
        String => "shexj-expect-string" {}, Number => "shexj-expect-number" {}, Boolean => "shexj-expect-boolean" {}, Array => "shexj-expect-array" {},
        UnsignedInteger => "shexj-expect-unsigned-integer" { "minimum" => Unsigned(0), "maximum" => Unsigned(u128::from(u64::MAX)), "negative_zero" => Boolean(false) },
        SignedInteger => "shexj-expect-signed-integer" { "minimum" => Signed(i128::from(i64::MIN)), "maximum" => Signed(i128::from(i64::MAX)) },
    }
}

/// An owned refused native JSON value. Equality retains spelling and member order.
/// Capture is cold-only; clone and drop use the JSON home's iterative operations.
#[derive(Debug, Clone)]
pub struct JsonOffered {
    value: Value,
}
impl JsonOffered {
    pub(crate) fn capture(value: &Value) -> Self {
        Self {
            value: value.clone(),
        }
    }
    /// The complete original native value, including number lexemes and order.
    pub fn value(&self) -> &Value {
        &self.value
    }
}
impl PartialEq for JsonOffered {
    fn eq(&self, other: &Self) -> bool {
        self.value.same_text(&other.value)
    }
}
impl Eq for JsonOffered {}
impl StructuredError for JsonOffered {
    purrdf_hash::error_visit! {
        @record self { value } => "shexj-offered" { "value" => Record(&self.value) }
    }
}

/// A complete native refusal from the ShExJ object/member or value reader.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonMemberReason {
    /// An object was required at this position.
    Object {
        /// The original grammar position.
        context: JsonContext,
        /// The actual offered value.
        offered: JsonOffered,
        /// The original native record kind refusal.
        cause: DecodeError,
    },
    /// A discriminator names a different type from the required one.
    Type {
        /// The original grammar position.
        context: JsonContext,
        /// The required object type.
        expected: JsonType,
        /// The original discriminator string.
        found: String,
    },
    /// A mandatory member is absent.
    Missing {
        /// The original grammar position.
        context: JsonContext,
        /// The missing member.
        member: JsonMember,
    },
    /// A present member has the wrong kind or numeric domain.
    Member {
        /// The original grammar position.
        context: JsonContext,
        /// The original member identity.
        member: JsonMember,
        /// The required value domain.
        expected: JsonExpectation,
        /// The complete original offered value.
        offered: JsonOffered,
    },
    /// A non-exponent numeric facet has no finite `f64` stand-in.
    NumericRange {
        /// The original grammar position.
        context: JsonContext,
        /// The numeric facet member.
        member: JsonMember,
        /// The original number, without reformatting its lexeme.
        offered: JsonOffered,
        /// The bits of the actual conversion that the range check refused.
        converted: u64,
    },
    /// An undeclared member remains after all modeled fields were read.
    Unknown {
        /// The original grammar position.
        context: JsonContext,
        /// The original unknown member name.
        name: String,
        /// The original member value.
        offered: JsonOffered,
    },
    /// A collection entry is not a string.
    Entry {
        /// The original grammar position.
        context: JsonContext,
        /// The imports or extra member.
        member: JsonMember,
        /// The original zero-based array position.
        index: usize,
        /// The actual entry value.
        offered: JsonOffered,
    },
    /// A shape declaration lacks a string identifier, absent or wrongly typed.
    Identifier {
        /// The present refused identifier, or an absent member.
        offered: Option<JsonOffered>,
    },
    /// A shape/triple expression lacks a string discriminator.
    Discriminator {
        /// Shape or triple expression grammar position.
        context: JsonContext,
        /// The original full value, preserving nonobjects and wrong-type tags.
        offered: JsonOffered,
    },
    /// A discriminator is not in the appropriate expression vocabulary.
    UnknownType {
        /// Shape or triple expression grammar position.
        context: JsonContext,
        /// The original unknown discriminator.
        found: String,
    },
    /// A stem has neither the string nor Wildcard form.
    Stem {
        /// The original stem-range grammar position.
        context: JsonContext,
        /// The actual offered value.
        offered: JsonOffered,
    },
    /// A node kind is not one of the four modeled names.
    NodeKind {
        /// The original unknown name.
        found: String,
    },
    /// The shared record law refused native metadata or a duplicate field.
    /// JSON ingress rejects duplicates first on the public ShExJ path.
    Record {
        /// The original grammar position.
        context: JsonContext,
        /// The original shared record cause, never rendered into a reason.
        cause: DecodeError,
    },
}
impl StructuredError for JsonMemberReason {
    purrdf_hash::error_visit! {
        Object { context, offered, cause } => "shexj-object" { "context" => Record(context), "offered" => Record(offered), "cause" => Cause(cause) },
        Type { context, expected, found } => "shexj-type" { "context" => Record(context), "expected" => Record(expected), "found" => Text(found) },
        Missing { context, member } => "shexj-missing" { "context" => Record(context), "member" => Record(member) },
        Member { context, member, expected, offered } => "shexj-member" { "context" => Record(context), "member" => Record(member), "expected" => Record(expected), "offered" => Record(offered) },
        NumericRange { context, member, offered, converted } => "shexj-numeric-range" { "context" => Record(context), "member" => Record(member), "offered" => Record(offered), "converted" => Float64Bits(*converted) },
        Unknown { context, name, offered } => "shexj-unknown-member" { "context" => Record(context), "name" => Text(name), "offered" => Record(offered) },
        Entry { context, member, index, offered } => "shexj-entry" { "context" => Record(context), "member" => Record(member), "index" => Unsigned(*index as u128), "offered" => Record(offered) },
        Identifier { offered } => "shexj-identifier" { "offered" => Value(offered.as_ref().map_or(ErrorValue::Absent, |value| ErrorValue::Record(value))) },
        Discriminator { context, offered } => "shexj-discriminator" { "context" => Record(context), "offered" => Record(offered) },
        UnknownType { context, found } => "shexj-unknown-type" { "context" => Record(context), "found" => Text(found) },
        Stem { context, offered } => "shexj-stem" { "context" => Record(context), "offered" => Record(offered) },
        NodeKind { found } => "shexj-node-kind" { "found" => Text(found) },
        Record { context, cause } => "shexj-record" { "context" => Record(context), "cause" => Cause(cause) },
    }
}
impl fmt::Display for JsonMemberReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Object { context, .. } => write!(f, "{context}: expected a JSON object"),
            Self::Type {
                context,
                expected,
                found,
            } => write!(
                f,
                "{context}: expected type {:?}, found {found:?}",
                expected.name()
            ),
            Self::Missing { context, member } => {
                write!(f, "{context}: missing {:?}", member.name())
            }
            Self::Member {
                context,
                member,
                expected,
                ..
            } => write!(
                f,
                "{context}: {:?} must be {}",
                member.name(),
                expected.description()
            ),
            Self::NumericRange {
                context, member, ..
            } => write!(f, "{context}: {:?} out of range", member.name()),
            Self::Unknown { context, name, .. } => write!(f, "{context}: unknown key {name:?}"),
            Self::Entry {
                context, member, ..
            } => write!(f, "{context}: {} entries must be strings", member.name()),
            Self::Identifier { .. } => f.write_str("shapes entry: missing string \"id\""),
            Self::Discriminator { context, .. } => {
                write!(f, "{context}: missing \"type\" discriminator")
            }
            Self::UnknownType { context, found } => write!(f, "{context}: unknown type {found:?}"),
            Self::Stem { context, .. } => write!(f, "{context}: stem must be a string or Wildcard"),
            Self::NodeKind { found } => write!(f, "NodeConstraint: unknown nodeKind {found:?}"),
            Self::Record { context, cause } => write!(f, "{context}: {cause}"),
        }
    }
}
purrdf_lex::error_source!(JsonMemberReason => { Self::Object { cause, .. } | Self::Record { cause, .. } => cause, });

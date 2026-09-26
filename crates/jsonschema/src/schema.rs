// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The compiled form of a schema: an arena of subschemas, each an ordered
//! list of keywords with every reference already resolved to an arena index.

use std::collections::BTreeMap;
use std::fmt;

use regex::Regex;
use serde_json::Value;

use crate::content::{Encoding, MediaType};
use crate::format::Format;
use crate::number::Decimal;

/// An index into [`Schema::nodes`].
pub(crate) type NodeId = usize;

/// A compiled schema, ready to validate instances.
///
/// Compiled by [`crate::Registry::compile`]. It owns everything it needs —
/// constants, patterns, the resolved reference graph — so it outlives the
/// registry it came from, and it is `Send + Sync`.
#[derive(Clone)]
pub struct Schema {
    pub(crate) nodes: Vec<Node>,
    /// Per compiled resource: what the dynamic scope can find in it.
    pub(crate) resources: Vec<ResourceScope>,
    pub(crate) root: NodeId,
}

impl fmt::Debug for Schema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Schema")
            .field("root", &self.nodes[self.root].location)
            .field("subschemas", &self.nodes.len())
            .finish_non_exhaustive()
    }
}

/// What a schema resource contributes to the dynamic scope.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResourceScope {
    /// Its `$dynamicAnchor`s (2020-12).
    pub(crate) dynamic_anchors: BTreeMap<String, NodeId>,
    /// Its root, when the root declares `"$recursiveAnchor": true` (2019-09).
    pub(crate) recursive_root: Option<NodeId>,
}

/// One compiled subschema.
#[derive(Debug, Clone)]
pub(crate) struct Node {
    /// `resource-uri#pointer`, the subschema's absolute location.
    pub(crate) location: String,
    /// The compiled resource it belongs to.
    pub(crate) resource: usize,
    pub(crate) body: Body,
}

#[derive(Debug, Clone)]
pub(crate) enum Body {
    Bool(bool),
    Keywords {
        keywords: Vec<Keyword>,
        /// Whether `unevaluatedItems`/`unevaluatedProperties` is present, so
        /// evaluation must track which items and properties were evaluated.
        tracks: bool,
    },
}

/// A keyword, with the token(s) it contributes to keyword locations.
#[derive(Debug, Clone)]
pub(crate) struct Keyword {
    /// The keyword as written; one location token.
    pub(crate) name: String,
    pub(crate) kind: Kind,
}

/// The JSON types `type` can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonType {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    String,
    Integer,
}

impl JsonType {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "null" => Self::Null,
            "boolean" => Self::Boolean,
            "object" => Self::Object,
            "array" => Self::Array,
            "number" => Self::Number,
            "string" => Self::String,
            "integer" => Self::Integer,
            _ => return None,
        })
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
            Self::Number => "number",
            Self::String => "string",
            Self::Integer => "integer",
        }
    }
}

/// A compiled `pattern` or `patternProperties` key.
#[derive(Debug, Clone)]
pub(crate) struct Pattern {
    pub(crate) source: String,
    pub(crate) regex: Regex,
}

#[derive(Debug, Clone)]
pub(crate) enum Kind {
    Ref(NodeId),
    /// `$dynamicRef`: the statically resolved target, and the anchor name to
    /// look up in the dynamic scope when the target is a `$dynamicAnchor`.
    DynamicRef {
        target: NodeId,
        anchor: Option<String>,
    },
    /// `$recursiveRef` (2019-09): the statically resolved target, and whether
    /// that target is a `$recursiveAnchor`, which makes the dynamic scope
    /// choose the resource evaluated.
    RecursiveRef {
        target: NodeId,
        dynamic: bool,
    },
    Type(Vec<JsonType>),
    Enum(Vec<Value>),
    Const(Value),
    MultipleOf(Decimal, Value),
    Maximum(Decimal, Value),
    ExclusiveMaximum(Decimal, Value),
    Minimum(Decimal, Value),
    ExclusiveMinimum(Decimal, Value),
    MaxLength(u64),
    MinLength(u64),
    Pattern(Box<Pattern>),
    MaxItems(u64),
    MinItems(u64),
    UniqueItems,
    MaxProperties(u64),
    MinProperties(u64),
    Required(Vec<String>),
    DependentRequired(Vec<(String, Vec<String>)>),
    /// `format`: its name, and the check when the Format-Assertion
    /// vocabulary is in force.
    Format(String, Option<Format>),
    /// Draft-07 `contentEncoding`: the string must decode.
    ContentEncoding(Encoding),
    /// Draft-07 `contentMediaType`: the string, decoded by the sibling
    /// `contentEncoding` when there is one, must be a document of the type.
    ContentMediaType {
        media: MediaType,
        encoding: Option<Encoding>,
    },
    AllOf(Vec<NodeId>),
    AnyOf(Vec<NodeId>),
    OneOf(Vec<NodeId>),
    Not(NodeId),
    If {
        condition: NodeId,
        then: Option<NodeId>,
        otherwise: Option<NodeId>,
    },
    DependentSchemas(Vec<(String, NodeId)>),
    /// `prefixItems`, or the array form of `items` in draft-07 and 2019-09.
    PrefixItems(Vec<NodeId>),
    /// `items`, and how many leading items `prefixItems` already covers; or
    /// `additionalItems` past an array of `items`.
    Items {
        schema: NodeId,
        skip: usize,
    },
    Contains {
        schema: NodeId,
        min: u64,
        max: Option<u64>,
        /// Whether the matched items are an annotation (2020-12).
        annotates: bool,
    },
    Properties(BTreeMap<String, NodeId>),
    PatternProperties(Vec<(Pattern, NodeId)>),
    AdditionalProperties {
        schema: NodeId,
        properties: Vec<String>,
        patterns: Vec<Regex>,
    },
    PropertyNames(NodeId),
    UnevaluatedItems(NodeId),
    UnevaluatedProperties(NodeId),
    /// A keyword with no assertion: its value is its annotation.
    Annotation(Value),
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON form of every visualization value.
//!
//! Each projection, scene, layout and export type is read and written through
//! [`purrdf_lex::json::record`]'s [`FromJson`] and [`ToJson`], so a
//! visualization document obeys the same record law as every other document a
//! PurRDF component reads. The form is fixed, because the export hashes
//! ([`VizExport::spec_hash`], [`VizExport::model_hash`], [`VizExport::scene_hash`]) are
//! taken over it:
//!
//! * a struct is an object of its fields, named as declared, in declaration order;
//! * an identifier newtype is its string;
//! * a unit variant is its camel-case name (`lowercase` for [`VizTextDirection`]), and a
//!   one-value variant of such an enum is an object of one member, the variant's name;
//! * a variant carrying named fields is an object whose `kind` member names the variant,
//!   followed by the fields; [`VizSemanticRef`] names its variant in `kind` and carries
//!   its identifier in `id`;
//! * `None` is `null`, and an absent member reads as `None`.
//!
//! Reading refuses a member the form does not declare, a repeated member, a missing
//! required member, a value of the wrong JSON type, and an unknown variant name.

use purrdf_lex::json::record::{DecodeError, FromJson, Record, ToJson};
use purrdf_lex::json::{Object, Value};

use super::{
    VizAccessibility, VizAssertion, VizAssertionId, VizBadge, VizBadgeKind, VizDiagnostic,
    VizDialect, VizEdgeAnchor, VizElementIndexEntry, VizElementKind, VizEndpoint, VizError,
    VizExport, VizGraph, VizGraphId, VizGraphPolicy, VizLabelPolicy, VizLayout, VizLayoutAnchor,
    VizLayoutBadge, VizLayoutEdge, VizLayoutLabel, VizLayoutLegendEntry, VizLayoutNode,
    VizLayoutOptions, VizLayoutPort, VizLayoutTable, VizLayoutTableCell, VizLegendEntry, VizMode,
    VizPoint, VizPort, VizPortKind, VizPosition, VizProjection, VizRect, VizReference,
    VizReferenceId, VizReferenceSite, VizRelation, VizRelationId, VizRenderOptions, VizRole,
    VizRoleRule, VizScene, VizSceneEdge, VizSceneEdgeKind, VizSceneGroup, VizSceneGroupKind,
    VizSceneLabel, VizSceneNode, VizSceneNodeKind, VizSceneTable, VizSceneTableCell,
    VizSceneTableRow, VizSemanticRef, VizSpec, VizStatement, VizStatementId, VizSvgDocument,
    VizSvgOptions, VizTable, VizTableField, VizTableRow, VizTerm, VizTermId, VizTermValue,
    VizTextDirection, VizValueRef, VizVocabularyMapping,
};

impl From<DecodeError> for VizError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error.to_string())
    }
}

/// An identifier newtype is its string.
macro_rules! viz_identifier_json {
    ($($type:ident),+ $(,)?) => {$(
        impl ToJson for $type {
            fn to_json(&self) -> Value {
                Value::from(self.0.as_str())
            }
        }

        impl FromJson for $type {
            fn from_json(value: &Value) -> Result<Self, DecodeError> {
                String::from_json(value).map(Self)
            }
        }
    )+};
}

viz_identifier_json!(
    VizTermId,
    VizStatementId,
    VizAssertionId,
    VizRelationId,
    VizReferenceId,
    VizGraphId,
);

/// A base direction as its lowercase token, `None` as `null`.
fn direction_to_json(direction: Option<VizTextDirection>) -> Value {
    direction.map_or(Value::Null, |direction| Value::from(direction.as_str()))
}

/// A base direction read from its lowercase token.
fn direction_from_json(value: &Value) -> Result<VizTextDirection, DecodeError> {
    let token = value
        .as_str()
        .ok_or_else(|| DecodeError::invalid_type(value, "enum VizTextDirection"))?;
    VizTextDirection::from_str_token(token).ok_or_else(|| {
        DecodeError::unknown_variant(
            token,
            &[
                VizTextDirection::Ltr.as_str(),
                VizTextDirection::Rtl.as_str(),
            ],
        )
    })
}

impl ToJson for VizValueRef {
    fn to_json(&self) -> Value {
        let (kind, id) = match self {
            Self::Term { id } => ("term", id.to_json()),
            Self::Statement { id } => ("statement", id.to_json()),
        };
        Object::new().with("kind", kind).with("id", id).into()
    }
}

impl FromJson for VizValueRef {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizValueRef")?;
        let reference = match record.tag("kind", &["term", "statement"])? {
            "term" => Self::Term {
                id: record.required("id")?,
            },
            _ => Self::Statement {
                id: record.required("id")?,
            },
        };
        record.deny_unknown()?;
        Ok(reference)
    }
}

impl ToJson for VizTermValue {
    fn to_json(&self) -> Value {
        match self {
            Self::Iri { value } => Object::new()
                .with("kind", "iri")
                .with("value", value.as_str())
                .into(),
            Self::Blank { label, scope } => Object::new()
                .with("kind", "blank")
                .with("label", label.as_str())
                .with("scope", scope.to_json())
                .into(),
            Self::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => Object::new()
                .with("kind", "literal")
                .with("lexical_form", lexical_form.as_str())
                .with("datatype", datatype.as_str())
                .with("language", language.to_json())
                .with("direction", direction_to_json(*direction))
                .into(),
        }
    }
}

impl FromJson for VizTermValue {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizTermValue")?;
        let term = match record.tag("kind", &["iri", "blank", "literal"])? {
            "iri" => Self::Iri {
                value: record.required("value")?,
            },
            "blank" => Self::Blank {
                label: record.required("label")?,
                scope: record.required("scope")?,
            },
            _ => Self::Literal {
                lexical_form: record.required("lexical_form")?,
                datatype: record.required("datatype")?,
                language: record.optional("language")?,
                direction: record.optional_with("direction", direction_from_json)?,
            },
        };
        record.deny_unknown()?;
        Ok(term)
    }
}

impl ToJson for VizRole {
    fn to_json(&self) -> Value {
        match self {
            Self::Focus => Value::from("focus"),
            Self::Reifier => Value::from("reifier"),
            Self::GraphName => Value::from("graphName"),
            Self::Predicate => Value::from("predicate"),
            Self::QuotedStatement => Value::from("quotedStatement"),
            Self::AssertedStatement => Value::from("assertedStatement"),
            Self::AnnotatedStatement => Value::from("annotatedStatement"),
            Self::Custom(inner) => Object::new().with("custom", inner.as_str()).into(),
        }
    }
}

impl FromJson for VizRole {
    /// A unit variant's name, or `{"custom": …}`.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        const UNIT: &[&str] = &[
            "focus",
            "reifier",
            "graphName",
            "predicate",
            "quotedStatement",
            "assertedStatement",
            "annotatedStatement",
        ];
        let Some(name) = value.as_str() else {
            let mut record = Record::new(value, "enum VizRole")?;
            let custom = record.required("custom")?;
            record.deny_unknown()?;
            return Ok(Self::Custom(custom));
        };
        match name {
            "focus" => Ok(Self::Focus),
            "reifier" => Ok(Self::Reifier),
            "graphName" => Ok(Self::GraphName),
            "predicate" => Ok(Self::Predicate),
            "quotedStatement" => Ok(Self::QuotedStatement),
            "assertedStatement" => Ok(Self::AssertedStatement),
            "annotatedStatement" => Ok(Self::AnnotatedStatement),
            other => Err(DecodeError::unknown_variant(other, UNIT)),
        }
    }
}

impl ToJson for VizGraphPolicy {
    fn to_json(&self) -> Value {
        match self {
            Self::All => Value::from("all"),
            Self::Include(inner) => Object::new().with("include", inner.to_json()).into(),
        }
    }
}

impl FromJson for VizGraphPolicy {
    /// `"all"`, or `{"include": [graph, …]}`.
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let Some(name) = value.as_str() else {
            let mut record = Record::new(value, "enum VizGraphPolicy")?;
            let include = record.required("include")?;
            record.deny_unknown()?;
            return Ok(Self::Include(include));
        };
        match name {
            "all" => Ok(Self::All),
            other => Err(DecodeError::unknown_variant(other, &["all"])),
        }
    }
}

impl ToJson for VizRelation {
    fn to_json(&self) -> Value {
        match self {
            Self::Reifies {
                id,
                reifier,
                statement,
                graph,
            } => Object::new()
                .with("kind", "reifies")
                .with("id", id.to_json())
                .with("reifier", reifier.to_json())
                .with("statement", statement.to_json())
                .with("graph", graph.to_json())
                .into(),
            Self::Annotation {
                id,
                reifier,
                predicate,
                object,
                graph,
            } => Object::new()
                .with("kind", "annotation")
                .with("id", id.to_json())
                .with("reifier", reifier.to_json())
                .with("predicate", predicate.to_json())
                .with("object", object.to_json())
                .with("graph", graph.to_json())
                .into(),
        }
    }
}

impl FromJson for VizRelation {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizRelation")?;
        let relation = match record.tag("kind", &["reifies", "annotation"])? {
            "reifies" => Self::Reifies {
                id: record.required("id")?,
                reifier: record.required("reifier")?,
                statement: record.required("statement")?,
                graph: record.required("graph")?,
            },
            _ => Self::Annotation {
                id: record.required("id")?,
                reifier: record.required("reifier")?,
                predicate: record.required("predicate")?,
                object: record.required("object")?,
                graph: record.required("graph")?,
            },
        };
        record.deny_unknown()?;
        Ok(relation)
    }
}

impl ToJson for VizReferenceSite {
    fn to_json(&self) -> Value {
        match self {
            Self::Statement {
                statement,
                position,
            } => Object::new()
                .with("kind", "statement")
                .with("statement", statement.to_json())
                .with("position", position.to_json())
                .into(),
            Self::Reification { relation } => Object::new()
                .with("kind", "reification")
                .with("relation", relation.to_json())
                .into(),
            Self::Annotation { relation } => Object::new()
                .with("kind", "annotation")
                .with("relation", relation.to_json())
                .into(),
            Self::GraphName { graph } => Object::new()
                .with("kind", "graphName")
                .with("graph", graph.to_json())
                .into(),
        }
    }
}

impl FromJson for VizReferenceSite {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizReferenceSite")?;
        let site = match record.tag(
            "kind",
            &["statement", "reification", "annotation", "graphName"],
        )? {
            "statement" => Self::Statement {
                statement: record.required("statement")?,
                position: record.required("position")?,
            },
            "reification" => Self::Reification {
                relation: record.required("relation")?,
            },
            "annotation" => Self::Annotation {
                relation: record.required("relation")?,
            },
            _ => Self::GraphName {
                graph: record.required("graph")?,
            },
        };
        record.deny_unknown()?;
        Ok(site)
    }
}

/// The `kind` spellings of [`VizSemanticRef`], in variant order.
const SEMANTIC_KINDS: &[&str] = &[
    "term",
    "statement",
    "assertion",
    "relation",
    "graph",
    "reference",
    "diagnostic",
];

impl ToJson for VizSemanticRef {
    fn to_json(&self) -> Value {
        let (kind, id) = match self {
            Self::Term(inner) => ("term", inner.to_json()),
            Self::Statement(inner) => ("statement", inner.to_json()),
            Self::Assertion(inner) => ("assertion", inner.to_json()),
            Self::Relation(inner) => ("relation", inner.to_json()),
            Self::Graph(inner) => ("graph", inner.to_json()),
            Self::Reference(inner) => ("reference", inner.to_json()),
            Self::Diagnostic(inner) => ("diagnostic", inner.to_json()),
        };
        Object::new().with("kind", kind).with("id", id).into()
    }
}

impl FromJson for VizSemanticRef {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizSemanticRef")?;
        let reference = match record.tag("kind", SEMANTIC_KINDS)? {
            "term" => Self::Term(record.required("id")?),
            "statement" => Self::Statement(record.required("id")?),
            "assertion" => Self::Assertion(record.required("id")?),
            "relation" => Self::Relation(record.required("id")?),
            "graph" => Self::Graph(record.required("id")?),
            "reference" => Self::Reference(record.required("id")?),
            _ => Self::Diagnostic(record.required("id")?),
        };
        record.deny_unknown()?;
        Ok(reference)
    }
}

impl ToJson for VizEndpoint {
    fn to_json(&self) -> Value {
        match self {
            Self::NodePort { node, port } => Object::new()
                .with("kind", "nodePort")
                .with("node", node.as_str())
                .with("port", port.as_str())
                .into(),
            Self::EdgeAnchor { edge, anchor } => Object::new()
                .with("kind", "edgeAnchor")
                .with("edge", edge.as_str())
                .with("anchor", anchor.as_str())
                .into(),
        }
    }
}

impl FromJson for VizEndpoint {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "internally tagged enum VizEndpoint")?;
        let endpoint = match record.tag("kind", &["nodePort", "edgeAnchor"])? {
            "nodePort" => Self::NodePort {
                node: record.required("node")?,
                port: record.required("port")?,
            },
            _ => Self::EdgeAnchor {
                edge: record.required("edge")?,
                anchor: record.required("anchor")?,
            },
        };
        record.deny_unknown()?;
        Ok(endpoint)
    }
}

impl ToJson for VizSceneLabel {
    fn to_json(&self) -> Value {
        Object::new()
            .with("text", self.text.as_str())
            .with("full_text", self.full_text.as_str())
            .with("language", self.language.to_json())
            .with("direction", direction_to_json(self.direction))
            .into()
    }
}

impl FromJson for VizSceneLabel {
    fn from_json(value: &Value) -> Result<Self, DecodeError> {
        let mut record = Record::new(value, "struct VizSceneLabel")?;
        let label = Self {
            text: record.required("text")?,
            full_text: record.required("full_text")?,
            language: record.optional("language")?,
            direction: record.optional_with("direction", direction_from_json)?,
        };
        record.deny_unknown()?;
        Ok(label)
    }
}

purrdf_lex::json_string_enum!(VizDialect {
    Rdf12 => "rdf12",
    SymmetricRdf12 => "symmetricRdf12",
    GeneralizedRdf => "generalizedRdf",
});

purrdf_lex::json_string_enum!(VizLabelPolicy { Compact => "compact", Full => "full" });

purrdf_lex::json_string_enum!(VizMode {
    Compact => "compact",
    Incidence => "incidence",
    Table => "table",
});

purrdf_lex::json_string_enum!(VizTableField {
    Statement => "statement",
    AssertedIn => "assertedIn",
    Reifiers => "reifiers",
    Annotations => "annotations",
    ReferencedBy => "referencedBy",
    Depth => "depth",
    Diagnostics => "diagnostics",
});

purrdf_lex::json_string_enum!(VizPosition { Subject => "subject", Object => "object" });

purrdf_lex::json_string_enum!(VizElementKind {
    NodeGroup => "nodeGroup",
    NodeShape => "nodeShape",
    NodeLabel => "nodeLabel",
    NodeBadge => "nodeBadge",
    NodeBadgeLabel => "nodeBadgeLabel",
    NodePort => "nodePort",
    EdgeGroup => "edgeGroup",
    EdgePath => "edgePath",
    EdgeLabel => "edgeLabel",
    EdgeBadge => "edgeBadge",
    EdgeBadgeLabel => "edgeBadgeLabel",
    EdgeAnchor => "edgeAnchor",
    EdgeAnchorLabel => "edgeAnchorLabel",
    EdgeAnchorBadge => "edgeAnchorBadge",
    EdgeAnchorBadgeLabel => "edgeAnchorBadgeLabel",
    Table => "table",
    TableCell => "tableCell",
    TableLabel => "tableLabel",
    Legend => "legend",
    LegendEntry => "legendEntry",
    LegendLabel => "legendLabel",
});

purrdf_lex::json_string_enum!(VizSceneNodeKind {
    Iri => "iri",
    Blank => "blank",
    Literal => "literal",
    Statement => "statement",
});

purrdf_lex::json_string_enum!(VizSceneEdgeKind {
    Assertion => "assertion",
    Subject => "subject",
    Predicate => "predicate",
    Object => "object",
    Reifies => "reifies",
    Annotation => "annotation",
    QuoteSubject => "quoteSubject",
    QuoteObject => "quoteObject",
});

purrdf_lex::json_string_enum!(VizPortKind {
    In => "in",
    Out => "out",
    Subject => "subject",
    Predicate => "predicate",
    Object => "object",
    Relation => "relation",
    Value => "value",
});

purrdf_lex::json_string_enum!(VizBadgeKind {
    Asserted => "asserted",
    Quoted => "quoted",
    Reifier => "reifier",
    Graph => "graph",
    AnnotationCount => "annotationCount",
    ReifierCount => "reifierCount",
    ReferenceCount => "referenceCount",
    NestingDepth => "nestingDepth",
    Dialect => "dialect",
    Direction => "direction",
    Focus => "focus",
    Role => "role",
});

purrdf_lex::json_string_enum!(VizSceneGroupKind { Legend => "legend" });

purrdf_lex::json_record!(VizDiagnostic as "struct VizDiagnostic" {
    "id" => id: required,
    "code" => code: required,
    "message" => message: required,
    "target" => target: optional,
    "dialect" => dialect: required,
});

purrdf_lex::json_record!(VizRoleRule as "struct VizRoleRule" {
    "predicate_iri" => predicate_iri: required,
    "role" => role: required,
});

purrdf_lex::json_record!(VizVocabularyMapping as "struct VizVocabularyMapping" {
    "prefix" => prefix: required,
    "namespace" => namespace: required,
});

purrdf_lex::json_record!(VizSpec as "struct VizSpec" {
    "mode" => mode: required,
    "focus" => focus: optional,
    "role_rules" => role_rules: required,
    "vocabulary" => vocabulary: required,
    "graph_policy" => graph_policy: required,
    "label_policy" => label_policy: required,
    "max_statements" => max_statements: required,
    "max_terms" => max_terms: required,
    "table_fields" => table_fields: required,
});

purrdf_lex::json_record!(VizTerm as "struct VizTerm" {
    "id" => id: required,
    "value" => value: required,
    "label" => label: required,
    "roles" => roles: required,
});

purrdf_lex::json_record!(VizStatement as "struct VizStatement" {
    "id" => id: required,
    "subject" => subject: required,
    "predicate" => predicate: required,
    "object" => object: required,
    "asserted_in" => asserted_in: required,
    "nesting_depth" => nesting_depth: required,
    "incoming_references" => incoming_references: required,
    "dialect" => dialect: required,
    "roles" => roles: required,
});

purrdf_lex::json_record!(VizAssertion as "struct VizAssertion" {
    "id" => id: required,
    "statement" => statement: required,
    "graph" => graph: required,
});

purrdf_lex::json_record!(VizReference as "struct VizReference" {
    "id" => id: required,
    "statement" => statement: required,
    "site" => site: required,
});

purrdf_lex::json_record!(VizGraph as "struct VizGraph" {
    "id" => id: required,
    "term" => term: optional,
    "label" => label: required,
});

purrdf_lex::json_record!(VizTableRow as "struct VizTableRow" {
    "statement" => statement: required,
    "asserted_in" => asserted_in: required,
    "reifier_count" => reifier_count: required,
    "annotation_count" => annotation_count: required,
    "referenced_by" => referenced_by: required,
    "depth" => depth: required,
});

purrdf_lex::json_record!(VizTable as "struct VizTable" {
    "fields" => fields: required,
    "rows" => rows: required,
});

purrdf_lex::json_record!(VizProjection as "struct VizProjection" {
    "terms" => terms: required,
    "statements" => statements: required,
    "assertions" => assertions: required,
    "relations" => relations: required,
    "graphs" => graphs: required,
    "references" => references: required,
    "table" => table: required,
    "diagnostics" => diagnostics: required,
});

purrdf_lex::json_record!(VizExport as "struct VizExport" {
    "schema_version" => schema_version: required,
    "spec" => spec: required,
    "spec_hash" => spec_hash: required,
    "model_hash" => model_hash: required,
    "scene_hash" => scene_hash: required,
    "model" => model: required,
    "scene" => scene: required,
    "layout" => layout: required,
    "element_index" => element_index: required,
    "diagnostics" => diagnostics: required,
});

purrdf_lex::json_record!(VizElementIndexEntry as "struct VizElementIndexEntry" {
    "element_id" => element_id: required,
    "scene_id" => scene_id: required,
    "bindings" => bindings: required,
    "kind" => kind: required,
});

purrdf_lex::json_record!(VizScene as "struct VizScene" {
    "schema_version" => schema_version: required,
    "mode" => mode: required,
    "nodes" => nodes: required,
    "edges" => edges: required,
    "groups" => groups: required,
    "legend" => legend: required,
    "table" => table: optional,
});

purrdf_lex::json_record!(VizSceneNode as "struct VizSceneNode" {
    "id" => id: required,
    "bindings" => bindings: required,
    "kind" => kind: required,
    "label" => label: required,
    "ports" => ports: required,
    "badges" => badges: required,
    "accessibility" => accessibility: required,
});

purrdf_lex::json_record!(VizSceneEdge as "struct VizSceneEdge" {
    "id" => id: required,
    "bindings" => bindings: required,
    "kind" => kind: required,
    "source" => source: required,
    "target" => target: required,
    "label" => label: required,
    "badges" => badges: required,
    "anchor" => anchor: optional,
    "accessibility" => accessibility: required,
});

purrdf_lex::json_record!(VizPort as "struct VizPort" {
    "id" => id: required,
    "kind" => kind: required,
});

purrdf_lex::json_record!(VizEdgeAnchor as "struct VizEdgeAnchor" {
    "id" => id: required,
    "bindings" => bindings: required,
    "label" => label: required,
    "badges" => badges: required,
    "accessibility" => accessibility: required,
});

purrdf_lex::json_record!(VizBadge as "struct VizBadge" {
    "kind" => kind: required,
    "label" => label: required,
    "binding" => binding: optional,
});

purrdf_lex::json_record!(VizAccessibility as "struct VizAccessibility" {
    "title" => title: required,
    "description" => description: required,
});

purrdf_lex::json_record!(VizSceneGroup as "struct VizSceneGroup" {
    "id" => id: required,
    "kind" => kind: required,
    "label" => label: required,
    "members" => members: required,
});

purrdf_lex::json_record!(VizLegendEntry as "struct VizLegendEntry" {
    "id" => id: required,
    "symbol" => symbol: required,
    "label" => label: required,
});

purrdf_lex::json_record!(VizSceneTable as "struct VizSceneTable" {
    "fields" => fields: required,
    "rows" => rows: required,
});

purrdf_lex::json_record!(VizSceneTableRow as "struct VizSceneTableRow" {
    "id" => id: required,
    "binding" => binding: required,
    "cells" => cells: required,
});

purrdf_lex::json_record!(VizSceneTableCell as "struct VizSceneTableCell" {
    "field" => field: required,
    "text" => text: required,
    "bindings" => bindings: required,
});

purrdf_lex::json_record!(VizLayoutOptions as "struct VizLayoutOptions" {
    "margin" => margin: required,
    "rank_spacing" => rank_spacing: required,
    "node_spacing" => node_spacing: required,
    "component_spacing" => component_spacing: required,
    "component_wrap_width" => component_wrap_width: required,
    "crossing_sweeps" => crossing_sweeps: required,
    "max_node_width" => max_node_width: required,
});

purrdf_lex::json_record!(VizPoint as "struct VizPoint" {
    "x" => x: required,
    "y" => y: required,
});

purrdf_lex::json_record!(VizRect as "struct VizRect" {
    "x" => x: required,
    "y" => y: required,
    "width" => width: required,
    "height" => height: required,
});

purrdf_lex::json_record!(VizLayoutLabel as "struct VizLayoutLabel" {
    "rect" => rect: required,
    "lines" => lines: required,
});

purrdf_lex::json_record!(VizLayoutBadge as "struct VizLayoutBadge" {
    "index" => index: required,
    "rect" => rect: required,
});

purrdf_lex::json_record!(VizLayoutPort as "struct VizLayoutPort" {
    "id" => id: required,
    "point" => point: required,
});

purrdf_lex::json_record!(VizLayoutNode as "struct VizLayoutNode" {
    "id" => id: required,
    "rect" => rect: required,
    "label" => label: required,
    "ports" => ports: required,
    "badges" => badges: required,
    "component" => component: required,
    "rank" => rank: required,
});

purrdf_lex::json_record!(VizLayoutAnchor as "struct VizLayoutAnchor" {
    "id" => id: required,
    "rect" => rect: required,
    "label" => label: required,
    "badges" => badges: required,
});

purrdf_lex::json_record!(VizLayoutEdge as "struct VizLayoutEdge" {
    "id" => id: required,
    "points" => points: required,
    "label" => label: required,
    "badges" => badges: required,
    "anchor" => anchor: optional,
});

purrdf_lex::json_record!(VizLayoutTableCell as "struct VizLayoutTableCell" {
    "row" => row: required,
    "column" => column: required,
    "rect" => rect: required,
    "label" => label: required,
});

purrdf_lex::json_record!(VizLayoutTable as "struct VizLayoutTable" {
    "rect" => rect: required,
    "cells" => cells: required,
});

purrdf_lex::json_record!(VizLayoutLegendEntry as "struct VizLayoutLegendEntry" {
    "id" => id: required,
    "rect" => rect: required,
});

purrdf_lex::json_record!(VizLayout as "struct VizLayout" {
    "schema_version" => schema_version: required,
    "mode" => mode: required,
    "width" => width: required,
    "height" => height: required,
    "nodes" => nodes: required,
    "edges" => edges: required,
    "table" => table: optional,
    "legend" => legend: required,
});

purrdf_lex::json_record!(VizSvgOptions as "struct VizSvgOptions" {
    "embed_metadata" => embed_metadata: required,
    "include_styles" => include_styles: required,
    "title" => title: required,
});

purrdf_lex::json_record!(VizRenderOptions as "struct VizRenderOptions" {
    "layout" => layout: required,
    "svg" => svg: required,
});

purrdf_lex::json_record!(VizSvgDocument as "struct VizSvgDocument" {
    "svg" => svg: required,
    "export" => export: required,
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variants_spell_their_camel_case_names() {
        assert_eq!(
            VizRole::QuotedStatement.to_json(),
            Value::from("quotedStatement")
        );
        assert_eq!(
            purrdf_lex::json::write_compact(&VizRole::Custom("x".to_owned()).to_json()),
            r#"{"custom":"x"}"#
        );
        assert_eq!(
            direction_to_json(Some(VizTextDirection::Rtl)),
            Value::from("rtl")
        );
        assert_eq!(
            purrdf_lex::json::write_compact(
                &VizSemanticRef::Term(VizTermId("t".to_owned())).to_json()
            ),
            r#"{"kind":"term","id":"t"}"#
        );
        let literal = VizTermValue::Literal {
            lexical_form: "a".to_owned(),
            datatype: "d".to_owned(),
            language: None,
            direction: Some(VizTextDirection::Ltr),
        };
        assert_eq!(
            purrdf_lex::json::write_compact(&literal.to_json()),
            r#"{"kind":"literal","lexical_form":"a","datatype":"d","language":null,"direction":"ltr"}"#
        );
        assert_eq!(VizTermValue::from_json(&literal.to_json()), Ok(literal));
    }

    #[test]
    fn reading_refuses_what_the_form_cannot_hold_and_accepts_its_neighbours() {
        let spec = VizSpec::default();
        let mut json = spec.to_json();
        assert_eq!(VizSpec::from_json(&json), Ok(spec.clone()));
        // An absent optional member is `None`.
        json.as_object_mut().expect("object").remove("focus");
        assert_eq!(VizSpec::from_json(&json), Ok(spec));
        // A member the form does not declare is refused, naming it.
        let mut extra = json.clone();
        extra.as_object_mut().expect("object").insert("extra", 1_u8);
        let error = VizSpec::from_json(&extra).expect_err("undeclared member");
        assert!(error.to_string().starts_with("unknown field `extra`"));
        // A repeated member is refused; the single spelling beside it reads.
        let text = purrdf_lex::json::write_compact(&json);
        let repeated = text.replacen('{', r#"{"mode":"table","#, 1);
        assert_eq!(
            VizSpec::from_json(&purrdf_lex::json::read(&repeated).expect("json"))
                .expect_err("repeated member")
                .to_string(),
            "duplicate field `mode` at /mode"
        );
        assert!(VizSpec::from_json(&purrdf_lex::json::read(&text).expect("json")).is_ok());
        // An absent required member, an unknown variant, and a wrong type are refused.
        json.as_object_mut().expect("object").remove("mode");
        assert_eq!(
            VizSpec::from_json(&json).expect_err("missing").to_string(),
            "missing field `mode`"
        );
        assert!(VizMode::from_json(&Value::from("sideways")).is_err());
        assert!(VizMode::from_json(&Value::from("table")).is_ok());
        assert!(u32::from_json(&Value::from(-1_i8)).is_err());
        assert!(u32::from_json(&Value::from(1_u8)).is_ok());
        assert!(VizGraphPolicy::from_json(&Value::from("all")).is_ok());
        assert!(
            VizGraphPolicy::from_json(
                &purrdf_lex::json::read(r#"{"include":["a"]}"#).expect("json")
            )
            .is_ok()
        );
        assert!(
            VizGraphPolicy::from_json(
                &purrdf_lex::json::read(r#"{"include":["a"],"all":null}"#).expect("json")
            )
            .is_err()
        );
        assert!(
            VizRole::from_json(&purrdf_lex::json::read(r#"{"custom":"x"}"#).expect("json")).is_ok()
        );
        assert!(
            VizRole::from_json(
                &purrdf_lex::json::read(r#"{"custom":"x","focus":1}"#).expect("json")
            )
            .is_err()
        );
        // A tagged variant refuses a member of another variant, and reads its own.
        let term = r#"{"kind":"iri","value":"https://example.org/a"}"#;
        assert!(VizTermValue::from_json(&purrdf_lex::json::read(term).expect("json")).is_ok());
        let crossed = r#"{"kind":"iri","value":"https://example.org/a","label":"b"}"#;
        assert!(VizTermValue::from_json(&purrdf_lex::json::read(crossed).expect("json")).is_err());
        // A base direction is exactly `ltr` or `rtl`.
        let label = r#"{"text":"a","full_text":"a","language":"ar","direction":"rtl"}"#;
        assert_eq!(
            VizSceneLabel::from_json(&purrdf_lex::json::read(label).expect("json"))
                .expect("label")
                .direction,
            Some(VizTextDirection::Rtl)
        );
        let sideways = label.replace("rtl", "ttb");
        assert!(
            VizSceneLabel::from_json(&purrdf_lex::json::read(&sideways).expect("json")).is_err()
        );
    }

    #[test]
    fn every_record_round_trips_through_its_one_member_list() {
        let label = VizSceneLabel {
            text: "a".to_owned(),
            full_text: "ab".to_owned(),
            language: Some("en".to_owned()),
            direction: Some(VizTextDirection::Ltr),
        };
        assert_eq!(
            purrdf_lex::json::write_compact(&label.to_json()),
            r#"{"text":"a","full_text":"ab","language":"en","direction":"ltr"}"#
        );
        assert_eq!(VizSceneLabel::from_json(&label.to_json()), Ok(label));
        let rect = VizRect {
            x: -1,
            y: 2,
            width: 3,
            height: 4,
        };
        assert_eq!(
            purrdf_lex::json::write_compact(&rect.to_json()),
            r#"{"x":-1,"y":2,"width":3,"height":4}"#
        );
        assert_eq!(VizRect::from_json(&rect.to_json()), Ok(rect));
        let id = VizTermId("t".to_owned());
        assert_eq!(id.to_json(), Value::from("t"));
        assert_eq!(VizTermId::from_json(&Value::from("t")), Ok(id));
        assert!(VizTermId::from_json(&Value::from(1_u8)).is_err());
    }
}

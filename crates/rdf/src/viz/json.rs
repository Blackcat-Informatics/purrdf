// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON form of every visualization value.
//!
//! [`VizJson`] maps each projection, scene, layout and export type to and from a
//! [`purrdf_lex::json::Value`]. The form is fixed, because the export hashes
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
//! Reading ignores members it does not know and refuses a missing required member, a
//! value of the wrong JSON type, and an unknown variant name.

use purrdf_lex::json::{Object, Value};

use super::*;

/// A visualization value's JSON form.
pub trait VizJson: Sized {
    /// The value as JSON.
    fn to_json(&self) -> Value;

    /// The value a JSON form denotes.
    ///
    /// # Errors
    ///
    /// [`VizError::Decode`] when `value` is not this type's JSON form.
    fn from_json(value: &Value) -> Result<Self, VizError>;

    /// The value of an absent object member: an error, except for an `Option`.
    ///
    /// # Errors
    ///
    /// [`VizError::Decode`] naming the member.
    fn from_absent(name: &str) -> Result<Self, VizError> {
        Err(VizError::Decode(format!("missing member `{name}`")))
    }
}

fn mismatch(expected: &str, value: &Value) -> VizError {
    VizError::Decode(format!(
        "expected {expected}, found {}",
        value.kind().name()
    ))
}

fn expect_object<'a>(value: &'a Value, what: &str) -> Result<&'a Object, VizError> {
    value
        .as_object()
        .ok_or_else(|| mismatch(&format!("a {what} object"), value))
}

fn expect_str<'a>(value: &'a Value, what: &str) -> Result<&'a str, VizError> {
    value
        .as_str()
        .ok_or_else(|| mismatch(&format!("a {what} string"), value))
}

/// The member `name` of `object`, read as `T`.
fn field<T: VizJson>(object: &Object, name: &str) -> Result<T, VizError> {
    object.get(name).map_or_else(
        || T::from_absent(name),
        |value| {
            T::from_json(value)
                .map_err(|error| VizError::Decode(format!("member `{name}`: {error}")))
        },
    )
}

/// The one member of an object naming a variant that carries a value.
fn single_member<'a>(value: &'a Value, what: &str) -> Result<(&'a str, &'a Value), VizError> {
    match expect_object(value, what)?.members() {
        [(name, inner)] => Ok((name.as_str(), inner)),
        _ => Err(mismatch(&format!("a {what} object of one member"), value)),
    }
}

fn unknown_variant(what: &str, name: &str) -> VizError {
    VizError::Decode(format!("unknown {what} variant `{name}`"))
}

impl VizJson for String {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        expect_str(value, "string").map(str::to_owned)
    }
}

impl VizJson for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        value.as_bool().ok_or_else(|| mismatch("a boolean", value))
    }
}

macro_rules! viz_json_integer {
    ($($t:ty),*) => {$(
        impl VizJson for $t {
            fn to_json(&self) -> Value {
                Value::from(*self)
            }

            fn from_json(value: &Value) -> Result<Self, VizError> {
                value
                    .as_number()
                    .and_then(purrdf_lex::json::Number::as_i128)
                    .and_then(|number| <$t>::try_from(number).ok())
                    .ok_or_else(|| mismatch(concat!("an integer in the ", stringify!($t), " range"), value))
            }
        }
    )*};
}

viz_json_integer!(i32, u32, usize);

impl<T: VizJson> VizJson for Option<T> {
    fn to_json(&self) -> Value {
        self.as_ref().map_or(Value::Null, T::to_json)
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        if value.is_null() {
            Ok(None)
        } else {
            T::from_json(value).map(Some)
        }
    }

    fn from_absent(_name: &str) -> Result<Self, VizError> {
        Ok(None)
    }
}

impl<T: VizJson> VizJson for Vec<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(T::to_json).collect())
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        value
            .as_array()
            .ok_or_else(|| mismatch("an array", value))?
            .iter()
            .enumerate()
            .map(|(index, item)| {
                T::from_json(item)
                    .map_err(|error| VizError::Decode(format!("item {index}: {error}")))
            })
            .collect()
    }
}

impl VizJson for VizTermId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizStatementId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizAssertionId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizRelationId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizReferenceId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizGraphId {
    fn to_json(&self) -> Value {
        self.0.to_json()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        String::from_json(value).map(Self)
    }
}

impl VizJson for VizValueRef {
    fn to_json(&self) -> Value {
        match self {
            Self::Term { id } => Object::new()
                .with("kind", "term")
                .with("id", id.to_json())
                .into(),
            Self::Statement { id } => Object::new()
                .with("kind", "statement")
                .with("id", id.to_json())
                .into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizValueRef")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "term" => Ok(Self::Term {
                id: field(object, "id")?,
            }),
            "statement" => Ok(Self::Statement {
                id: field(object, "id")?,
            }),
            other => Err(unknown_variant("VizValueRef", other)),
        }
    }
}

impl VizJson for VizTermValue {
    fn to_json(&self) -> Value {
        match self {
            Self::Iri { value } => Object::new()
                .with("kind", "iri")
                .with("value", value.to_json())
                .into(),
            Self::Blank { label, scope } => Object::new()
                .with("kind", "blank")
                .with("label", label.to_json())
                .with("scope", scope.to_json())
                .into(),
            Self::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => Object::new()
                .with("kind", "literal")
                .with("lexical_form", lexical_form.to_json())
                .with("datatype", datatype.to_json())
                .with("language", language.to_json())
                .with("direction", direction.to_json())
                .into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizTermValue")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "iri" => Ok(Self::Iri {
                value: field(object, "value")?,
            }),
            "blank" => Ok(Self::Blank {
                label: field(object, "label")?,
                scope: field(object, "scope")?,
            }),
            "literal" => Ok(Self::Literal {
                lexical_form: field(object, "lexical_form")?,
                datatype: field(object, "datatype")?,
                language: field(object, "language")?,
                direction: field(object, "direction")?,
            }),
            other => Err(unknown_variant("VizTermValue", other)),
        }
    }
}

impl VizJson for VizTextDirection {
    fn to_json(&self) -> Value {
        Value::from(self.as_str())
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let token = expect_str(value, "VizTextDirection")?;
        Self::from_str_token(token).ok_or_else(|| unknown_variant("VizTextDirection", token))
    }
}

impl VizJson for VizRole {
    fn to_json(&self) -> Value {
        match self {
            Self::Focus => Value::from("focus"),
            Self::Reifier => Value::from("reifier"),
            Self::GraphName => Value::from("graphName"),
            Self::Predicate => Value::from("predicate"),
            Self::QuotedStatement => Value::from("quotedStatement"),
            Self::AssertedStatement => Value::from("assertedStatement"),
            Self::AnnotatedStatement => Value::from("annotatedStatement"),
            Self::Custom(inner) => Object::new().with("custom", inner.to_json()).into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        if let Some(name) = value.as_str() {
            return match name {
                "focus" => Ok(Self::Focus),
                "reifier" => Ok(Self::Reifier),
                "graphName" => Ok(Self::GraphName),
                "predicate" => Ok(Self::Predicate),
                "quotedStatement" => Ok(Self::QuotedStatement),
                "assertedStatement" => Ok(Self::AssertedStatement),
                "annotatedStatement" => Ok(Self::AnnotatedStatement),
                other => Err(unknown_variant("VizRole", other)),
            };
        }
        let (name, inner) = single_member(value, "VizRole")?;
        match name {
            "custom" => Ok(Self::Custom(String::from_json(inner)?)),
            other => Err(unknown_variant("VizRole", other)),
        }
    }
}

impl VizJson for VizDialect {
    fn to_json(&self) -> Value {
        match self {
            Self::Rdf12 => Value::from("rdf12"),
            Self::SymmetricRdf12 => Value::from("symmetricRdf12"),
            Self::GeneralizedRdf => Value::from("generalizedRdf"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizDialect")? {
            "rdf12" => Ok(Self::Rdf12),
            "symmetricRdf12" => Ok(Self::SymmetricRdf12),
            "generalizedRdf" => Ok(Self::GeneralizedRdf),
            other => Err(unknown_variant("VizDialect", other)),
        }
    }
}

impl VizJson for VizDiagnostic {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("code", self.code.to_json())
            .with("message", self.message.to_json())
            .with("target", self.target.to_json())
            .with("dialect", self.dialect.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizDiagnostic")?;
        Ok(Self {
            id: field(object, "id")?,
            code: field(object, "code")?,
            message: field(object, "message")?,
            target: field(object, "target")?,
            dialect: field(object, "dialect")?,
        })
    }
}

impl VizJson for VizRoleRule {
    fn to_json(&self) -> Value {
        Object::new()
            .with("predicate_iri", self.predicate_iri.to_json())
            .with("role", self.role.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizRoleRule")?;
        Ok(Self {
            predicate_iri: field(object, "predicate_iri")?,
            role: field(object, "role")?,
        })
    }
}

impl VizJson for VizVocabularyMapping {
    fn to_json(&self) -> Value {
        Object::new()
            .with("prefix", self.prefix.to_json())
            .with("namespace", self.namespace.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizVocabularyMapping")?;
        Ok(Self {
            prefix: field(object, "prefix")?,
            namespace: field(object, "namespace")?,
        })
    }
}

impl VizJson for VizGraphPolicy {
    fn to_json(&self) -> Value {
        match self {
            Self::All => Value::from("all"),
            Self::Include(inner) => Object::new().with("include", inner.to_json()).into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        if let Some(name) = value.as_str() {
            return match name {
                "all" => Ok(Self::All),
                other => Err(unknown_variant("VizGraphPolicy", other)),
            };
        }
        let (name, inner) = single_member(value, "VizGraphPolicy")?;
        match name {
            "include" => Ok(Self::Include(Vec::<String>::from_json(inner)?)),
            other => Err(unknown_variant("VizGraphPolicy", other)),
        }
    }
}

impl VizJson for VizLabelPolicy {
    fn to_json(&self) -> Value {
        match self {
            Self::Compact => Value::from("compact"),
            Self::Full => Value::from("full"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizLabelPolicy")? {
            "compact" => Ok(Self::Compact),
            "full" => Ok(Self::Full),
            other => Err(unknown_variant("VizLabelPolicy", other)),
        }
    }
}

impl VizJson for VizMode {
    fn to_json(&self) -> Value {
        match self {
            Self::Compact => Value::from("compact"),
            Self::Incidence => Value::from("incidence"),
            Self::Table => Value::from("table"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizMode")? {
            "compact" => Ok(Self::Compact),
            "incidence" => Ok(Self::Incidence),
            "table" => Ok(Self::Table),
            other => Err(unknown_variant("VizMode", other)),
        }
    }
}

impl VizJson for VizTableField {
    fn to_json(&self) -> Value {
        match self {
            Self::Statement => Value::from("statement"),
            Self::AssertedIn => Value::from("assertedIn"),
            Self::Reifiers => Value::from("reifiers"),
            Self::Annotations => Value::from("annotations"),
            Self::ReferencedBy => Value::from("referencedBy"),
            Self::Depth => Value::from("depth"),
            Self::Diagnostics => Value::from("diagnostics"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizTableField")? {
            "statement" => Ok(Self::Statement),
            "assertedIn" => Ok(Self::AssertedIn),
            "reifiers" => Ok(Self::Reifiers),
            "annotations" => Ok(Self::Annotations),
            "referencedBy" => Ok(Self::ReferencedBy),
            "depth" => Ok(Self::Depth),
            "diagnostics" => Ok(Self::Diagnostics),
            other => Err(unknown_variant("VizTableField", other)),
        }
    }
}

impl VizJson for VizSpec {
    fn to_json(&self) -> Value {
        Object::new()
            .with("mode", self.mode.to_json())
            .with("focus", self.focus.to_json())
            .with("role_rules", self.role_rules.to_json())
            .with("vocabulary", self.vocabulary.to_json())
            .with("graph_policy", self.graph_policy.to_json())
            .with("label_policy", self.label_policy.to_json())
            .with("max_statements", self.max_statements.to_json())
            .with("max_terms", self.max_terms.to_json())
            .with("table_fields", self.table_fields.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSpec")?;
        Ok(Self {
            mode: field(object, "mode")?,
            focus: field(object, "focus")?,
            role_rules: field(object, "role_rules")?,
            vocabulary: field(object, "vocabulary")?,
            graph_policy: field(object, "graph_policy")?,
            label_policy: field(object, "label_policy")?,
            max_statements: field(object, "max_statements")?,
            max_terms: field(object, "max_terms")?,
            table_fields: field(object, "table_fields")?,
        })
    }
}

impl VizJson for VizTerm {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("value", self.value.to_json())
            .with("label", self.label.to_json())
            .with("roles", self.roles.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizTerm")?;
        Ok(Self {
            id: field(object, "id")?,
            value: field(object, "value")?,
            label: field(object, "label")?,
            roles: field(object, "roles")?,
        })
    }
}

impl VizJson for VizStatement {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("subject", self.subject.to_json())
            .with("predicate", self.predicate.to_json())
            .with("object", self.object.to_json())
            .with("asserted_in", self.asserted_in.to_json())
            .with("nesting_depth", self.nesting_depth.to_json())
            .with("incoming_references", self.incoming_references.to_json())
            .with("dialect", self.dialect.to_json())
            .with("roles", self.roles.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizStatement")?;
        Ok(Self {
            id: field(object, "id")?,
            subject: field(object, "subject")?,
            predicate: field(object, "predicate")?,
            object: field(object, "object")?,
            asserted_in: field(object, "asserted_in")?,
            nesting_depth: field(object, "nesting_depth")?,
            incoming_references: field(object, "incoming_references")?,
            dialect: field(object, "dialect")?,
            roles: field(object, "roles")?,
        })
    }
}

impl VizJson for VizAssertion {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("statement", self.statement.to_json())
            .with("graph", self.graph.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizAssertion")?;
        Ok(Self {
            id: field(object, "id")?,
            statement: field(object, "statement")?,
            graph: field(object, "graph")?,
        })
    }
}

impl VizJson for VizRelation {
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

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizRelation")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "reifies" => Ok(Self::Reifies {
                id: field(object, "id")?,
                reifier: field(object, "reifier")?,
                statement: field(object, "statement")?,
                graph: field(object, "graph")?,
            }),
            "annotation" => Ok(Self::Annotation {
                id: field(object, "id")?,
                reifier: field(object, "reifier")?,
                predicate: field(object, "predicate")?,
                object: field(object, "object")?,
                graph: field(object, "graph")?,
            }),
            other => Err(unknown_variant("VizRelation", other)),
        }
    }
}

impl VizJson for VizReference {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("statement", self.statement.to_json())
            .with("site", self.site.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizReference")?;
        Ok(Self {
            id: field(object, "id")?,
            statement: field(object, "statement")?,
            site: field(object, "site")?,
        })
    }
}

impl VizJson for VizPosition {
    fn to_json(&self) -> Value {
        match self {
            Self::Subject => Value::from("subject"),
            Self::Object => Value::from("object"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizPosition")? {
            "subject" => Ok(Self::Subject),
            "object" => Ok(Self::Object),
            other => Err(unknown_variant("VizPosition", other)),
        }
    }
}

impl VizJson for VizReferenceSite {
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

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizReferenceSite")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "statement" => Ok(Self::Statement {
                statement: field(object, "statement")?,
                position: field(object, "position")?,
            }),
            "reification" => Ok(Self::Reification {
                relation: field(object, "relation")?,
            }),
            "annotation" => Ok(Self::Annotation {
                relation: field(object, "relation")?,
            }),
            "graphName" => Ok(Self::GraphName {
                graph: field(object, "graph")?,
            }),
            other => Err(unknown_variant("VizReferenceSite", other)),
        }
    }
}

impl VizJson for VizGraph {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("term", self.term.to_json())
            .with("label", self.label.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizGraph")?;
        Ok(Self {
            id: field(object, "id")?,
            term: field(object, "term")?,
            label: field(object, "label")?,
        })
    }
}

impl VizJson for VizTableRow {
    fn to_json(&self) -> Value {
        Object::new()
            .with("statement", self.statement.to_json())
            .with("asserted_in", self.asserted_in.to_json())
            .with("reifier_count", self.reifier_count.to_json())
            .with("annotation_count", self.annotation_count.to_json())
            .with("referenced_by", self.referenced_by.to_json())
            .with("depth", self.depth.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizTableRow")?;
        Ok(Self {
            statement: field(object, "statement")?,
            asserted_in: field(object, "asserted_in")?,
            reifier_count: field(object, "reifier_count")?,
            annotation_count: field(object, "annotation_count")?,
            referenced_by: field(object, "referenced_by")?,
            depth: field(object, "depth")?,
        })
    }
}

impl VizJson for VizTable {
    fn to_json(&self) -> Value {
        Object::new()
            .with("fields", self.fields.to_json())
            .with("rows", self.rows.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizTable")?;
        Ok(Self {
            fields: field(object, "fields")?,
            rows: field(object, "rows")?,
        })
    }
}

impl VizJson for VizProjection {
    fn to_json(&self) -> Value {
        Object::new()
            .with("terms", self.terms.to_json())
            .with("statements", self.statements.to_json())
            .with("assertions", self.assertions.to_json())
            .with("relations", self.relations.to_json())
            .with("graphs", self.graphs.to_json())
            .with("references", self.references.to_json())
            .with("table", self.table.to_json())
            .with("diagnostics", self.diagnostics.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizProjection")?;
        Ok(Self {
            terms: field(object, "terms")?,
            statements: field(object, "statements")?,
            assertions: field(object, "assertions")?,
            relations: field(object, "relations")?,
            graphs: field(object, "graphs")?,
            references: field(object, "references")?,
            table: field(object, "table")?,
            diagnostics: field(object, "diagnostics")?,
        })
    }
}

impl VizJson for VizExport {
    fn to_json(&self) -> Value {
        Object::new()
            .with("schema_version", self.schema_version.to_json())
            .with("spec", self.spec.to_json())
            .with("spec_hash", self.spec_hash.to_json())
            .with("model_hash", self.model_hash.to_json())
            .with("scene_hash", self.scene_hash.to_json())
            .with("model", self.model.to_json())
            .with("scene", self.scene.to_json())
            .with("layout", self.layout.to_json())
            .with("element_index", self.element_index.to_json())
            .with("diagnostics", self.diagnostics.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizExport")?;
        Ok(Self {
            schema_version: field(object, "schema_version")?,
            spec: field(object, "spec")?,
            spec_hash: field(object, "spec_hash")?,
            model_hash: field(object, "model_hash")?,
            scene_hash: field(object, "scene_hash")?,
            model: field(object, "model")?,
            scene: field(object, "scene")?,
            layout: field(object, "layout")?,
            element_index: field(object, "element_index")?,
            diagnostics: field(object, "diagnostics")?,
        })
    }
}

impl VizJson for VizElementIndexEntry {
    fn to_json(&self) -> Value {
        Object::new()
            .with("element_id", self.element_id.to_json())
            .with("scene_id", self.scene_id.to_json())
            .with("bindings", self.bindings.to_json())
            .with("kind", self.kind.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizElementIndexEntry")?;
        Ok(Self {
            element_id: field(object, "element_id")?,
            scene_id: field(object, "scene_id")?,
            bindings: field(object, "bindings")?,
            kind: field(object, "kind")?,
        })
    }
}

impl VizJson for VizElementKind {
    fn to_json(&self) -> Value {
        match self {
            Self::NodeGroup => Value::from("nodeGroup"),
            Self::NodeShape => Value::from("nodeShape"),
            Self::NodeLabel => Value::from("nodeLabel"),
            Self::NodeBadge => Value::from("nodeBadge"),
            Self::NodeBadgeLabel => Value::from("nodeBadgeLabel"),
            Self::NodePort => Value::from("nodePort"),
            Self::EdgeGroup => Value::from("edgeGroup"),
            Self::EdgePath => Value::from("edgePath"),
            Self::EdgeLabel => Value::from("edgeLabel"),
            Self::EdgeBadge => Value::from("edgeBadge"),
            Self::EdgeBadgeLabel => Value::from("edgeBadgeLabel"),
            Self::EdgeAnchor => Value::from("edgeAnchor"),
            Self::EdgeAnchorLabel => Value::from("edgeAnchorLabel"),
            Self::EdgeAnchorBadge => Value::from("edgeAnchorBadge"),
            Self::EdgeAnchorBadgeLabel => Value::from("edgeAnchorBadgeLabel"),
            Self::Table => Value::from("table"),
            Self::TableCell => Value::from("tableCell"),
            Self::TableLabel => Value::from("tableLabel"),
            Self::Legend => Value::from("legend"),
            Self::LegendEntry => Value::from("legendEntry"),
            Self::LegendLabel => Value::from("legendLabel"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizElementKind")? {
            "nodeGroup" => Ok(Self::NodeGroup),
            "nodeShape" => Ok(Self::NodeShape),
            "nodeLabel" => Ok(Self::NodeLabel),
            "nodeBadge" => Ok(Self::NodeBadge),
            "nodeBadgeLabel" => Ok(Self::NodeBadgeLabel),
            "nodePort" => Ok(Self::NodePort),
            "edgeGroup" => Ok(Self::EdgeGroup),
            "edgePath" => Ok(Self::EdgePath),
            "edgeLabel" => Ok(Self::EdgeLabel),
            "edgeBadge" => Ok(Self::EdgeBadge),
            "edgeBadgeLabel" => Ok(Self::EdgeBadgeLabel),
            "edgeAnchor" => Ok(Self::EdgeAnchor),
            "edgeAnchorLabel" => Ok(Self::EdgeAnchorLabel),
            "edgeAnchorBadge" => Ok(Self::EdgeAnchorBadge),
            "edgeAnchorBadgeLabel" => Ok(Self::EdgeAnchorBadgeLabel),
            "table" => Ok(Self::Table),
            "tableCell" => Ok(Self::TableCell),
            "tableLabel" => Ok(Self::TableLabel),
            "legend" => Ok(Self::Legend),
            "legendEntry" => Ok(Self::LegendEntry),
            "legendLabel" => Ok(Self::LegendLabel),
            other => Err(unknown_variant("VizElementKind", other)),
        }
    }
}

impl VizJson for VizScene {
    fn to_json(&self) -> Value {
        Object::new()
            .with("schema_version", self.schema_version.to_json())
            .with("mode", self.mode.to_json())
            .with("nodes", self.nodes.to_json())
            .with("edges", self.edges.to_json())
            .with("groups", self.groups.to_json())
            .with("legend", self.legend.to_json())
            .with("table", self.table.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizScene")?;
        Ok(Self {
            schema_version: field(object, "schema_version")?,
            mode: field(object, "mode")?,
            nodes: field(object, "nodes")?,
            edges: field(object, "edges")?,
            groups: field(object, "groups")?,
            legend: field(object, "legend")?,
            table: field(object, "table")?,
        })
    }
}

impl VizJson for VizSemanticRef {
    fn to_json(&self) -> Value {
        match self {
            Self::Term(inner) => Object::new()
                .with("kind", "term")
                .with("id", inner.to_json())
                .into(),
            Self::Statement(inner) => Object::new()
                .with("kind", "statement")
                .with("id", inner.to_json())
                .into(),
            Self::Assertion(inner) => Object::new()
                .with("kind", "assertion")
                .with("id", inner.to_json())
                .into(),
            Self::Relation(inner) => Object::new()
                .with("kind", "relation")
                .with("id", inner.to_json())
                .into(),
            Self::Graph(inner) => Object::new()
                .with("kind", "graph")
                .with("id", inner.to_json())
                .into(),
            Self::Reference(inner) => Object::new()
                .with("kind", "reference")
                .with("id", inner.to_json())
                .into(),
            Self::Diagnostic(inner) => Object::new()
                .with("kind", "diagnostic")
                .with("id", inner.to_json())
                .into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSemanticRef")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "term" => Ok(Self::Term(field(object, "id")?)),
            "statement" => Ok(Self::Statement(field(object, "id")?)),
            "assertion" => Ok(Self::Assertion(field(object, "id")?)),
            "relation" => Ok(Self::Relation(field(object, "id")?)),
            "graph" => Ok(Self::Graph(field(object, "id")?)),
            "reference" => Ok(Self::Reference(field(object, "id")?)),
            "diagnostic" => Ok(Self::Diagnostic(field(object, "id")?)),
            other => Err(unknown_variant("VizSemanticRef", other)),
        }
    }
}

impl VizJson for VizSceneNode {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("bindings", self.bindings.to_json())
            .with("kind", self.kind.to_json())
            .with("label", self.label.to_json())
            .with("ports", self.ports.to_json())
            .with("badges", self.badges.to_json())
            .with("accessibility", self.accessibility.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneNode")?;
        Ok(Self {
            id: field(object, "id")?,
            bindings: field(object, "bindings")?,
            kind: field(object, "kind")?,
            label: field(object, "label")?,
            ports: field(object, "ports")?,
            badges: field(object, "badges")?,
            accessibility: field(object, "accessibility")?,
        })
    }
}

impl VizJson for VizSceneNodeKind {
    fn to_json(&self) -> Value {
        match self {
            Self::Iri => Value::from("iri"),
            Self::Blank => Value::from("blank"),
            Self::Literal => Value::from("literal"),
            Self::Statement => Value::from("statement"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizSceneNodeKind")? {
            "iri" => Ok(Self::Iri),
            "blank" => Ok(Self::Blank),
            "literal" => Ok(Self::Literal),
            "statement" => Ok(Self::Statement),
            other => Err(unknown_variant("VizSceneNodeKind", other)),
        }
    }
}

impl VizJson for VizSceneEdge {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("bindings", self.bindings.to_json())
            .with("kind", self.kind.to_json())
            .with("source", self.source.to_json())
            .with("target", self.target.to_json())
            .with("label", self.label.to_json())
            .with("badges", self.badges.to_json())
            .with("anchor", self.anchor.to_json())
            .with("accessibility", self.accessibility.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneEdge")?;
        Ok(Self {
            id: field(object, "id")?,
            bindings: field(object, "bindings")?,
            kind: field(object, "kind")?,
            source: field(object, "source")?,
            target: field(object, "target")?,
            label: field(object, "label")?,
            badges: field(object, "badges")?,
            anchor: field(object, "anchor")?,
            accessibility: field(object, "accessibility")?,
        })
    }
}

impl VizJson for VizSceneEdgeKind {
    fn to_json(&self) -> Value {
        match self {
            Self::Assertion => Value::from("assertion"),
            Self::Subject => Value::from("subject"),
            Self::Predicate => Value::from("predicate"),
            Self::Object => Value::from("object"),
            Self::Reifies => Value::from("reifies"),
            Self::Annotation => Value::from("annotation"),
            Self::QuoteSubject => Value::from("quoteSubject"),
            Self::QuoteObject => Value::from("quoteObject"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizSceneEdgeKind")? {
            "assertion" => Ok(Self::Assertion),
            "subject" => Ok(Self::Subject),
            "predicate" => Ok(Self::Predicate),
            "object" => Ok(Self::Object),
            "reifies" => Ok(Self::Reifies),
            "annotation" => Ok(Self::Annotation),
            "quoteSubject" => Ok(Self::QuoteSubject),
            "quoteObject" => Ok(Self::QuoteObject),
            other => Err(unknown_variant("VizSceneEdgeKind", other)),
        }
    }
}

impl VizJson for VizPort {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("kind", self.kind.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizPort")?;
        Ok(Self {
            id: field(object, "id")?,
            kind: field(object, "kind")?,
        })
    }
}

impl VizJson for VizPortKind {
    fn to_json(&self) -> Value {
        match self {
            Self::In => Value::from("in"),
            Self::Out => Value::from("out"),
            Self::Subject => Value::from("subject"),
            Self::Predicate => Value::from("predicate"),
            Self::Object => Value::from("object"),
            Self::Relation => Value::from("relation"),
            Self::Value => Value::from("value"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizPortKind")? {
            "in" => Ok(Self::In),
            "out" => Ok(Self::Out),
            "subject" => Ok(Self::Subject),
            "predicate" => Ok(Self::Predicate),
            "object" => Ok(Self::Object),
            "relation" => Ok(Self::Relation),
            "value" => Ok(Self::Value),
            other => Err(unknown_variant("VizPortKind", other)),
        }
    }
}

impl VizJson for VizEndpoint {
    fn to_json(&self) -> Value {
        match self {
            Self::NodePort { node, port } => Object::new()
                .with("kind", "nodePort")
                .with("node", node.to_json())
                .with("port", port.to_json())
                .into(),
            Self::EdgeAnchor { edge, anchor } => Object::new()
                .with("kind", "edgeAnchor")
                .with("edge", edge.to_json())
                .with("anchor", anchor.to_json())
                .into(),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizEndpoint")?;
        let kind: String = field(object, "kind")?;
        match kind.as_str() {
            "nodePort" => Ok(Self::NodePort {
                node: field(object, "node")?,
                port: field(object, "port")?,
            }),
            "edgeAnchor" => Ok(Self::EdgeAnchor {
                edge: field(object, "edge")?,
                anchor: field(object, "anchor")?,
            }),
            other => Err(unknown_variant("VizEndpoint", other)),
        }
    }
}

impl VizJson for VizEdgeAnchor {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("bindings", self.bindings.to_json())
            .with("label", self.label.to_json())
            .with("badges", self.badges.to_json())
            .with("accessibility", self.accessibility.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizEdgeAnchor")?;
        Ok(Self {
            id: field(object, "id")?,
            bindings: field(object, "bindings")?,
            label: field(object, "label")?,
            badges: field(object, "badges")?,
            accessibility: field(object, "accessibility")?,
        })
    }
}

impl VizJson for VizSceneLabel {
    fn to_json(&self) -> Value {
        Object::new()
            .with("text", self.text.to_json())
            .with("full_text", self.full_text.to_json())
            .with("language", self.language.to_json())
            .with("direction", self.direction.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneLabel")?;
        Ok(Self {
            text: field(object, "text")?,
            full_text: field(object, "full_text")?,
            language: field(object, "language")?,
            direction: field(object, "direction")?,
        })
    }
}

impl VizJson for VizBadge {
    fn to_json(&self) -> Value {
        Object::new()
            .with("kind", self.kind.to_json())
            .with("label", self.label.to_json())
            .with("binding", self.binding.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizBadge")?;
        Ok(Self {
            kind: field(object, "kind")?,
            label: field(object, "label")?,
            binding: field(object, "binding")?,
        })
    }
}

impl VizJson for VizBadgeKind {
    fn to_json(&self) -> Value {
        match self {
            Self::Asserted => Value::from("asserted"),
            Self::Quoted => Value::from("quoted"),
            Self::Reifier => Value::from("reifier"),
            Self::Graph => Value::from("graph"),
            Self::AnnotationCount => Value::from("annotationCount"),
            Self::ReifierCount => Value::from("reifierCount"),
            Self::ReferenceCount => Value::from("referenceCount"),
            Self::NestingDepth => Value::from("nestingDepth"),
            Self::Dialect => Value::from("dialect"),
            Self::Direction => Value::from("direction"),
            Self::Focus => Value::from("focus"),
            Self::Role => Value::from("role"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizBadgeKind")? {
            "asserted" => Ok(Self::Asserted),
            "quoted" => Ok(Self::Quoted),
            "reifier" => Ok(Self::Reifier),
            "graph" => Ok(Self::Graph),
            "annotationCount" => Ok(Self::AnnotationCount),
            "reifierCount" => Ok(Self::ReifierCount),
            "referenceCount" => Ok(Self::ReferenceCount),
            "nestingDepth" => Ok(Self::NestingDepth),
            "dialect" => Ok(Self::Dialect),
            "direction" => Ok(Self::Direction),
            "focus" => Ok(Self::Focus),
            "role" => Ok(Self::Role),
            other => Err(unknown_variant("VizBadgeKind", other)),
        }
    }
}

impl VizJson for VizAccessibility {
    fn to_json(&self) -> Value {
        Object::new()
            .with("title", self.title.to_json())
            .with("description", self.description.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizAccessibility")?;
        Ok(Self {
            title: field(object, "title")?,
            description: field(object, "description")?,
        })
    }
}

impl VizJson for VizSceneGroup {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("kind", self.kind.to_json())
            .with("label", self.label.to_json())
            .with("members", self.members.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneGroup")?;
        Ok(Self {
            id: field(object, "id")?,
            kind: field(object, "kind")?,
            label: field(object, "label")?,
            members: field(object, "members")?,
        })
    }
}

impl VizJson for VizSceneGroupKind {
    fn to_json(&self) -> Value {
        match self {
            Self::Legend => Value::from("legend"),
        }
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        match expect_str(value, "VizSceneGroupKind")? {
            "legend" => Ok(Self::Legend),
            other => Err(unknown_variant("VizSceneGroupKind", other)),
        }
    }
}

impl VizJson for VizLegendEntry {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("symbol", self.symbol.to_json())
            .with("label", self.label.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLegendEntry")?;
        Ok(Self {
            id: field(object, "id")?,
            symbol: field(object, "symbol")?,
            label: field(object, "label")?,
        })
    }
}

impl VizJson for VizSceneTable {
    fn to_json(&self) -> Value {
        Object::new()
            .with("fields", self.fields.to_json())
            .with("rows", self.rows.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneTable")?;
        Ok(Self {
            fields: field(object, "fields")?,
            rows: field(object, "rows")?,
        })
    }
}

impl VizJson for VizSceneTableRow {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("binding", self.binding.to_json())
            .with("cells", self.cells.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneTableRow")?;
        Ok(Self {
            id: field(object, "id")?,
            binding: field(object, "binding")?,
            cells: field(object, "cells")?,
        })
    }
}

impl VizJson for VizSceneTableCell {
    fn to_json(&self) -> Value {
        Object::new()
            .with("field", self.field.to_json())
            .with("text", self.text.to_json())
            .with("bindings", self.bindings.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSceneTableCell")?;
        Ok(Self {
            field: field(object, "field")?,
            text: field(object, "text")?,
            bindings: field(object, "bindings")?,
        })
    }
}

impl VizJson for VizLayoutOptions {
    fn to_json(&self) -> Value {
        Object::new()
            .with("margin", self.margin.to_json())
            .with("rank_spacing", self.rank_spacing.to_json())
            .with("node_spacing", self.node_spacing.to_json())
            .with("component_spacing", self.component_spacing.to_json())
            .with("component_wrap_width", self.component_wrap_width.to_json())
            .with("crossing_sweeps", self.crossing_sweeps.to_json())
            .with("max_node_width", self.max_node_width.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutOptions")?;
        Ok(Self {
            margin: field(object, "margin")?,
            rank_spacing: field(object, "rank_spacing")?,
            node_spacing: field(object, "node_spacing")?,
            component_spacing: field(object, "component_spacing")?,
            component_wrap_width: field(object, "component_wrap_width")?,
            crossing_sweeps: field(object, "crossing_sweeps")?,
            max_node_width: field(object, "max_node_width")?,
        })
    }
}

impl VizJson for VizPoint {
    fn to_json(&self) -> Value {
        Object::new()
            .with("x", self.x.to_json())
            .with("y", self.y.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizPoint")?;
        Ok(Self {
            x: field(object, "x")?,
            y: field(object, "y")?,
        })
    }
}

impl VizJson for VizRect {
    fn to_json(&self) -> Value {
        Object::new()
            .with("x", self.x.to_json())
            .with("y", self.y.to_json())
            .with("width", self.width.to_json())
            .with("height", self.height.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizRect")?;
        Ok(Self {
            x: field(object, "x")?,
            y: field(object, "y")?,
            width: field(object, "width")?,
            height: field(object, "height")?,
        })
    }
}

impl VizJson for VizLayoutLabel {
    fn to_json(&self) -> Value {
        Object::new()
            .with("rect", self.rect.to_json())
            .with("lines", self.lines.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutLabel")?;
        Ok(Self {
            rect: field(object, "rect")?,
            lines: field(object, "lines")?,
        })
    }
}

impl VizJson for VizLayoutBadge {
    fn to_json(&self) -> Value {
        Object::new()
            .with("index", self.index.to_json())
            .with("rect", self.rect.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutBadge")?;
        Ok(Self {
            index: field(object, "index")?,
            rect: field(object, "rect")?,
        })
    }
}

impl VizJson for VizLayoutPort {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("point", self.point.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutPort")?;
        Ok(Self {
            id: field(object, "id")?,
            point: field(object, "point")?,
        })
    }
}

impl VizJson for VizLayoutNode {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("rect", self.rect.to_json())
            .with("label", self.label.to_json())
            .with("ports", self.ports.to_json())
            .with("badges", self.badges.to_json())
            .with("component", self.component.to_json())
            .with("rank", self.rank.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutNode")?;
        Ok(Self {
            id: field(object, "id")?,
            rect: field(object, "rect")?,
            label: field(object, "label")?,
            ports: field(object, "ports")?,
            badges: field(object, "badges")?,
            component: field(object, "component")?,
            rank: field(object, "rank")?,
        })
    }
}

impl VizJson for VizLayoutAnchor {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("rect", self.rect.to_json())
            .with("label", self.label.to_json())
            .with("badges", self.badges.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutAnchor")?;
        Ok(Self {
            id: field(object, "id")?,
            rect: field(object, "rect")?,
            label: field(object, "label")?,
            badges: field(object, "badges")?,
        })
    }
}

impl VizJson for VizLayoutEdge {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("points", self.points.to_json())
            .with("label", self.label.to_json())
            .with("badges", self.badges.to_json())
            .with("anchor", self.anchor.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutEdge")?;
        Ok(Self {
            id: field(object, "id")?,
            points: field(object, "points")?,
            label: field(object, "label")?,
            badges: field(object, "badges")?,
            anchor: field(object, "anchor")?,
        })
    }
}

impl VizJson for VizLayoutTableCell {
    fn to_json(&self) -> Value {
        Object::new()
            .with("row", self.row.to_json())
            .with("column", self.column.to_json())
            .with("rect", self.rect.to_json())
            .with("label", self.label.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutTableCell")?;
        Ok(Self {
            row: field(object, "row")?,
            column: field(object, "column")?,
            rect: field(object, "rect")?,
            label: field(object, "label")?,
        })
    }
}

impl VizJson for VizLayoutTable {
    fn to_json(&self) -> Value {
        Object::new()
            .with("rect", self.rect.to_json())
            .with("cells", self.cells.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutTable")?;
        Ok(Self {
            rect: field(object, "rect")?,
            cells: field(object, "cells")?,
        })
    }
}

impl VizJson for VizLayoutLegendEntry {
    fn to_json(&self) -> Value {
        Object::new()
            .with("id", self.id.to_json())
            .with("rect", self.rect.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayoutLegendEntry")?;
        Ok(Self {
            id: field(object, "id")?,
            rect: field(object, "rect")?,
        })
    }
}

impl VizJson for VizLayout {
    fn to_json(&self) -> Value {
        Object::new()
            .with("schema_version", self.schema_version.to_json())
            .with("mode", self.mode.to_json())
            .with("width", self.width.to_json())
            .with("height", self.height.to_json())
            .with("nodes", self.nodes.to_json())
            .with("edges", self.edges.to_json())
            .with("table", self.table.to_json())
            .with("legend", self.legend.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizLayout")?;
        Ok(Self {
            schema_version: field(object, "schema_version")?,
            mode: field(object, "mode")?,
            width: field(object, "width")?,
            height: field(object, "height")?,
            nodes: field(object, "nodes")?,
            edges: field(object, "edges")?,
            table: field(object, "table")?,
            legend: field(object, "legend")?,
        })
    }
}

impl VizJson for VizSvgOptions {
    fn to_json(&self) -> Value {
        Object::new()
            .with("embed_metadata", self.embed_metadata.to_json())
            .with("include_styles", self.include_styles.to_json())
            .with("title", self.title.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSvgOptions")?;
        Ok(Self {
            embed_metadata: field(object, "embed_metadata")?,
            include_styles: field(object, "include_styles")?,
            title: field(object, "title")?,
        })
    }
}

impl VizJson for VizRenderOptions {
    fn to_json(&self) -> Value {
        Object::new()
            .with("layout", self.layout.to_json())
            .with("svg", self.svg.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizRenderOptions")?;
        Ok(Self {
            layout: field(object, "layout")?,
            svg: field(object, "svg")?,
        })
    }
}

impl VizJson for VizSvgDocument {
    fn to_json(&self) -> Value {
        Object::new()
            .with("svg", self.svg.to_json())
            .with("export", self.export.to_json())
            .into()
    }

    fn from_json(value: &Value) -> Result<Self, VizError> {
        let object = expect_object(value, "VizSvgDocument")?;
        Ok(Self {
            svg: field(object, "svg")?,
            export: field(object, "export")?,
        })
    }
}

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
        assert_eq!(VizTextDirection::Rtl.to_json(), Value::from("rtl"));
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
        // An absent optional member is `None`; an unknown member is ignored.
        json.as_object_mut().expect("object").remove("focus");
        json.as_object_mut().expect("object").insert("extra", 1_u8);
        assert_eq!(VizSpec::from_json(&json), Ok(spec));
        // An absent required member, an unknown variant, and a wrong type are refused.
        json.as_object_mut().expect("object").remove("mode");
        assert!(VizSpec::from_json(&json).is_err());
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
    }
}

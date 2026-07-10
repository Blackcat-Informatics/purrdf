// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Statement-centric RDF 1.2 visualization projection.
//!
//! The core contract is the [`VizProjection`]: a renderer-neutral Statement
//! Incidence Model that keeps structural statements separate from assertions,
//! reifiers, annotations, graph context, and dialect diagnostics. Renderers use
//! this model; they do not rediscover RDF 1.2 statement structure from flat quads.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};

use crate::{QuadRef, RdfDataset, RdfTextDirection, TermRef, TermValue};

mod scene;

pub use scene::*;

const DEFAULT_GRAPH_ID: &str = "graph:default";
const DEFAULT_MAX_STATEMENTS: usize = 500;
/// Visualization schema version embedded in structured exports.
pub const VIZ_EXPORT_SCHEMA_VERSION: &str = "purrdf-viz-export-1";

/// A typed term identifier within one deterministic visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizTermId(pub String);

/// A typed structural-statement identifier within one visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizStatementId(pub String);

/// A typed assertion identifier within one visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizAssertionId(pub String);

/// A typed relation identifier within one visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizRelationId(pub String);

/// A typed reference identifier within one visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizReferenceId(pub String);

/// A typed named-graph identifier within one visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VizGraphId(pub String);

/// A reference to either an ordinary RDF term or a structural statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VizValueRef {
    /// A non-triple RDF term.
    Term {
        /// The referenced term id.
        id: VizTermId,
    },
    /// An RDF 1.2 triple term represented by its structural statement id.
    Statement {
        /// The referenced structural statement id.
        id: VizStatementId,
    },
}

/// A JSON-friendly RDF term value used by visualization exports.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VizTermValue {
    /// An IRI term.
    Iri {
        /// The full IRI string.
        value: String,
    },
    /// A blank node term.
    Blank {
        /// The blank-node label.
        label: String,
        /// The blank-node scope ordinal.
        scope: u32,
    },
    /// A literal term, including RDF 1.2 base direction.
    Literal {
        /// The literal lexical form.
        lexical_form: String,
        /// The expanded datatype IRI.
        datatype: String,
        /// The language tag, when present.
        language: Option<String>,
        /// The RDF 1.2 base direction, when present.
        direction: Option<VizTextDirection>,
    },
}

/// RDF 1.2 base direction in exported visualization metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VizTextDirection {
    /// Left-to-right base direction.
    Ltr,
    /// Right-to-left base direction.
    Rtl,
}

/// Visualization role attached to a term or statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizRole {
    /// The current focus selected by the caller.
    Focus,
    /// A term used as a reifier.
    Reifier,
    /// A term used as a graph name.
    GraphName,
    /// A term used as a predicate.
    Predicate,
    /// A statement represented by a quoted triple term.
    QuotedStatement,
    /// A statement represented by an assertion.
    AssertedStatement,
    /// A statement with explicit annotations.
    AnnotatedStatement,
    /// A caller-supplied role.
    Custom(String),
}

/// RDF dialect/conformance state surfaced by the visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizDialect {
    /// Standard RDF 1.2 position.
    Rdf12,
    /// Symmetric RDF 1.2 position, such as a triple term in subject position.
    SymmetricRdf12,
    /// Generalized RDF position.
    GeneralizedRdf,
}

/// A typed visualization diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VizDiagnostic {
    /// Deterministic diagnostic id.
    pub id: String,
    /// Stable machine-readable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional projection id the diagnostic refers to.
    pub target: Option<String>,
    /// Dialect classification for the diagnostic.
    pub dialect: VizDialect,
}

/// A caller-supplied role rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VizRoleRule {
    /// Predicate IRI that activates this role.
    pub predicate_iri: String,
    /// Role to attach when the predicate is present.
    pub role: VizRole,
}

/// A caller-supplied vocabulary mapping used by specs and labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VizVocabularyMapping {
    /// Compact prefix.
    pub prefix: String,
    /// IRI namespace.
    pub namespace: String,
}

/// Graph-context filtering policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizGraphPolicy {
    /// Include every graph context.
    #[default]
    All,
    /// Include only graph selectors listed here. A selector is `default`, a
    /// full graph-name IRI, a compact blank-node label, a canonical term key,
    /// or a deterministic visualization graph id.
    Include(Vec<String>),
}

/// Label generation policy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizLabelPolicy {
    /// Generate compact labels from term values.
    #[default]
    Compact,
    /// Use full RDF term strings.
    Full,
}

/// Visualization mode requested by a spec.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizMode {
    /// Compact resource graph.
    #[default]
    Compact,
    /// Exact statement/incidence graph.
    Incidence,
    /// Statement table/matrix rows.
    Table,
}

/// Column available in the statement table projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizTableField {
    /// Structural statement text and identity.
    Statement,
    /// Graphs containing assertion occurrences.
    AssertedIn,
    /// Reifier count.
    Reifiers,
    /// Annotation count across the statement's reifiers.
    Annotations,
    /// Incoming triple-term reference count.
    ReferencedBy,
    /// Structural triple-term nesting depth.
    Depth,
    /// Dialect and conformance diagnostics.
    Diagnostics,
}

/// Caller-provided semantic lens for visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSpec {
    /// Requested visualization mode.
    pub mode: VizMode,
    /// Optional focus term or statement key.
    pub focus: Option<String>,
    /// Role rules keyed by caller vocabulary.
    pub role_rules: Vec<VizRoleRule>,
    /// Caller-provided vocabulary mappings.
    pub vocabulary: Vec<VizVocabularyMapping>,
    /// Graph filtering policy.
    pub graph_policy: VizGraphPolicy,
    /// Label policy.
    pub label_policy: VizLabelPolicy,
    /// Maximum structural statements accepted by this spec.
    pub max_statements: usize,
    /// Maximum visible terms accepted by this spec.
    pub max_terms: usize,
    /// Statement table fields requested by the caller.
    pub table_fields: Vec<VizTableField>,
}

impl Default for VizSpec {
    fn default() -> Self {
        Self {
            mode: VizMode::Compact,
            focus: None,
            role_rules: Vec::new(),
            vocabulary: Vec::new(),
            graph_policy: VizGraphPolicy::All,
            label_policy: VizLabelPolicy::Compact,
            max_statements: DEFAULT_MAX_STATEMENTS,
            max_terms: DEFAULT_MAX_STATEMENTS * 3,
            table_fields: vec![
                VizTableField::Statement,
                VizTableField::AssertedIn,
                VizTableField::Reifiers,
                VizTableField::Annotations,
                VizTableField::ReferencedBy,
                VizTableField::Depth,
                VizTableField::Diagnostics,
            ],
        }
    }
}

/// A graph-like input quad for callers that do not already have an [`RdfDataset`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInputQuad {
    /// Subject value.
    pub subject: TermValue,
    /// Predicate IRI.
    pub predicate: String,
    /// Object value.
    pub object: TermValue,
    /// Optional graph name.
    pub graph_name: Option<TermValue>,
}

/// A graph-like reification relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInputReifier {
    /// Reifier value.
    pub reifier: TermValue,
    /// Reified structural statement.
    pub statement: VizInputStatement,
    /// Optional graph name for the reification relation.
    pub graph_name: Option<TermValue>,
}

/// A graph-like annotation relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInputAnnotation {
    /// Reifier value.
    pub reifier: TermValue,
    /// Annotation predicate IRI.
    pub predicate: String,
    /// Annotation object.
    pub object: TermValue,
    /// Optional graph name for the annotation relation.
    pub graph_name: Option<TermValue>,
}

/// A graph-like structural statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VizInputStatement {
    /// Subject value.
    pub subject: TermValue,
    /// Predicate IRI.
    pub predicate: String,
    /// Object value.
    pub object: TermValue,
}

/// Graph-like visualization input for callers that do not hold an [`RdfDataset`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VizGraphInput {
    /// Asserted quads.
    pub quads: Vec<VizInputQuad>,
    /// Reification relations.
    pub reifiers: Vec<VizInputReifier>,
    /// Annotation relations.
    pub annotations: Vec<VizInputAnnotation>,
}

/// A projected ordinary RDF term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizTerm {
    /// Deterministic term id.
    pub id: VizTermId,
    /// Term value.
    pub value: VizTermValue,
    /// Display label.
    pub label: String,
    /// Roles this term plays in the projection.
    pub roles: Vec<VizRole>,
}

/// A projected RDF 1.2 structural statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizStatement {
    /// Deterministic statement id.
    pub id: VizStatementId,
    /// Statement subject.
    pub subject: VizValueRef,
    /// Statement predicate.
    pub predicate: VizTermId,
    /// Statement object.
    pub object: VizValueRef,
    /// Graphs where this statement is asserted.
    pub asserted_in: Vec<VizGraphId>,
    /// Structural nesting depth.
    pub nesting_depth: u32,
    /// Number of places where this statement is referenced as a triple term.
    pub incoming_references: u32,
    /// Dialect/conformance classification for this statement.
    pub dialect: VizDialect,
    /// Roles this statement plays in the projection.
    pub roles: Vec<VizRole>,
}

/// A concrete assertion occurrence for a structural statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizAssertion {
    /// Deterministic assertion id.
    pub id: VizAssertionId,
    /// Asserted statement id.
    pub statement: VizStatementId,
    /// Assertion graph.
    pub graph: VizGraphId,
}

/// A projected relation in the RDF 1.2 statement layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VizRelation {
    /// A reifier term reifies a structural statement.
    Reifies {
        /// Deterministic relation id.
        id: VizRelationId,
        /// Reifier term.
        reifier: VizTermId,
        /// Reified statement.
        statement: VizStatementId,
        /// Relation graph context.
        graph: VizGraphId,
    },
    /// An annotation is an ordinary predicate/object relation from a reifier.
    Annotation {
        /// Deterministic relation id.
        id: VizRelationId,
        /// Annotated reifier term.
        reifier: VizTermId,
        /// Annotation predicate term.
        predicate: VizTermId,
        /// Annotation object.
        object: VizValueRef,
        /// Relation graph context.
        graph: VizGraphId,
    },
}

/// A reference to a structural statement as a triple term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizReference {
    /// Deterministic reference id.
    pub id: VizReferenceId,
    /// Referenced structural statement.
    pub statement: VizStatementId,
    /// Exact place where the triple term occurs.
    pub site: VizReferenceSite,
}

/// Subject or object position occupied by a triple term.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizPosition {
    /// Subject position.
    Subject,
    /// Object position.
    Object,
}

/// Exact source site for a structural-statement reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VizReferenceSite {
    /// Triple term occurs in another structural statement.
    Statement {
        /// Containing structural statement.
        statement: VizStatementId,
        /// Subject or object position.
        position: VizPosition,
    },
    /// Triple term is the target of a reification relation.
    Reification {
        /// Containing reification relation.
        relation: VizRelationId,
    },
    /// Triple term occurs as an annotation object.
    Annotation {
        /// Containing annotation relation.
        relation: VizRelationId,
    },
    /// Triple term occurs as a graph name in generalized RDF.
    GraphName {
        /// Containing graph record.
        graph: VizGraphId,
    },
}

/// A graph context known to the visualization projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizGraph {
    /// Deterministic graph id.
    pub id: VizGraphId,
    /// Graph term. `None` is the default graph.
    pub term: Option<VizValueRef>,
    /// Display label.
    pub label: String,
}

/// A statement table row derived from the projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizTableRow {
    /// Statement id.
    pub statement: VizStatementId,
    /// Assertion graph ids.
    pub asserted_in: Vec<VizGraphId>,
    /// Reifier count.
    pub reifier_count: usize,
    /// Annotation count attached to all statement reifiers.
    pub annotation_count: usize,
    /// Reference count.
    pub referenced_by: u32,
    /// Nesting depth.
    pub depth: u32,
}

/// Statement table projection with caller-selected columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizTable {
    /// Columns in display order.
    pub fields: Vec<VizTableField>,
    /// Structural statement rows.
    pub rows: Vec<VizTableRow>,
}

/// The renderer-neutral Statement Incidence Model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizProjection {
    /// Terms in deterministic order.
    pub terms: Vec<VizTerm>,
    /// Structural statements in deterministic order.
    pub statements: Vec<VizStatement>,
    /// Assertions in deterministic order.
    pub assertions: Vec<VizAssertion>,
    /// Reification and annotation relations in deterministic order.
    pub relations: Vec<VizRelation>,
    /// Graph contexts in deterministic order.
    pub graphs: Vec<VizGraph>,
    /// Triple-term references in deterministic order.
    pub references: Vec<VizReference>,
    /// Statement table rows in deterministic order.
    pub table: VizTable,
    /// Diagnostics in deterministic order.
    pub diagnostics: Vec<VizDiagnostic>,
}

/// Backwards-friendly alias for the renderer-neutral visualization model.
pub type VizModel = VizProjection;

/// A versioned visualization export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizExport {
    /// Export schema version.
    pub schema_version: String,
    /// Deterministic spec hash.
    pub spec_hash: String,
    /// Projected model.
    pub model: VizProjection,
    /// Layout records.
    pub layout: Vec<VizLayoutRecord>,
    /// SVG element to model-id index.
    pub element_index: Vec<VizElementIndexEntry>,
    /// Export diagnostics.
    pub diagnostics: Vec<VizDiagnostic>,
}

/// Deterministic layout record for a projected entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLayoutRecord {
    /// Model id being positioned.
    pub id: String,
    /// Integer x coordinate in projection units.
    pub x: i32,
    /// Integer y coordinate in projection units.
    pub y: i32,
}

/// SVG element to projection-id mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizElementIndexEntry {
    /// SVG element id.
    pub element_id: String,
    /// Projection id.
    pub model_id: String,
    /// Element kind.
    pub kind: String,
}

/// Visualization projection errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VizError {
    /// The requested projection exceeds explicit size limits.
    TooLarge {
        /// Limit name.
        limit: &'static str,
        /// Actual count.
        actual: usize,
        /// Allowed count.
        allowed: usize,
    },
    /// A spec references an unknown role predicate.
    UnknownRolePredicate(String),
    /// A spec contains an invalid vocabulary mapping.
    InvalidVocabulary(String),
    /// A visualization specification is internally inconsistent.
    InvalidSpec(String),
    /// A graph-like input contains an invalid predicate.
    InvalidPredicate(String),
    /// A focus selector does not match any projected term or statement.
    UnknownFocus(String),
    /// A focus selector matches more than one projected entity.
    AmbiguousFocus(String),
    /// A deterministic identity hash collided with a different structural key.
    IdCollision(String),
    /// A renderer-neutral scene is structurally invalid.
    Scene(String),
    /// Serialization failed.
    Serialize(String),
}

impl fmt::Display for VizError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge {
                limit,
                actual,
                allowed,
            } => write!(
                f,
                "visualization {limit} limit exceeded: {actual} > {allowed}"
            ),
            Self::UnknownRolePredicate(iri) => {
                write!(f, "visualization role predicate {iri:?} is not present")
            }
            Self::InvalidVocabulary(message) => f.write_str(message),
            Self::InvalidSpec(message) => f.write_str(message),
            Self::InvalidPredicate(predicate) => {
                write!(f, "visualization predicate {predicate:?} is not an IRI")
            }
            Self::UnknownFocus(focus) => {
                write!(
                    f,
                    "visualization focus {focus:?} does not match the projection"
                )
            }
            Self::AmbiguousFocus(focus) => {
                write!(f, "visualization focus {focus:?} is ambiguous")
            }
            Self::IdCollision(id) => {
                write!(f, "visualization structural identity collision for {id}")
            }
            Self::Scene(message) => f.write_str(message),
            Self::Serialize(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for VizError {}

/// Project a dataset into the renderer-neutral Statement Incidence Model.
pub fn project_dataset(dataset: &RdfDataset, spec: &VizSpec) -> Result<VizProjection, VizError> {
    let mut builder = ProjectionBuilder::new(spec);
    builder.add_default_graph();
    for quad in dataset.quad_refs() {
        builder.add_dataset_quad(dataset, quad)?;
    }
    for (reifier, triple, graph) in dataset.reifiers_with_graph() {
        builder.add_reifier(
            dataset.term_value(reifier),
            dataset.term_value(triple),
            graph.map(|g| dataset.term_value(g)),
        )?;
    }
    for (reifier, predicate, object, graph) in dataset.annotations_with_graph() {
        let predicate_iri = match dataset.resolve(predicate) {
            TermRef::Iri(iri) => iri.to_owned(),
            _ => {
                return Err(VizError::InvalidPredicate(term_key(
                    &dataset.term_value(predicate),
                )));
            }
        };
        builder.add_annotation(
            dataset.term_value(reifier),
            &predicate_iri,
            dataset.term_value(object),
            graph.map(|g| dataset.term_value(g)),
        )?;
    }
    builder.finish()
}

/// Project graph-like caller input into the renderer-neutral Statement Incidence Model.
pub fn project_graph_input(
    input: &VizGraphInput,
    spec: &VizSpec,
) -> Result<VizProjection, VizError> {
    let mut builder = ProjectionBuilder::new(spec);
    builder.add_default_graph();
    for quad in &input.quads {
        builder.add_assertion(
            quad.subject.clone(),
            quad.predicate.clone(),
            quad.object.clone(),
            quad.graph_name.clone(),
        )?;
    }
    for reifier in &input.reifiers {
        builder.add_reifier(
            reifier.reifier.clone(),
            TermValue::Triple {
                s: Box::new(reifier.statement.subject.clone()),
                p: Box::new(TermValue::Iri(reifier.statement.predicate.clone())),
                o: Box::new(reifier.statement.object.clone()),
            },
            reifier.graph_name.clone(),
        )?;
    }
    for annotation in &input.annotations {
        builder.add_annotation(
            annotation.reifier.clone(),
            &annotation.predicate,
            annotation.object.clone(),
            annotation.graph_name.clone(),
        )?;
    }
    builder.finish()
}

/// Project a dataset and serialize the model to deterministic JSON.
pub fn project_dataset_json(dataset: &RdfDataset, spec: &VizSpec) -> Result<String, VizError> {
    let projection = project_dataset(dataset, spec)?;
    serde_json::to_string(&projection).map_err(|err| VizError::Serialize(err.to_string()))
}

#[derive(Debug, Clone)]
struct TermDraft {
    value: VizTermValue,
    label: String,
    roles: BTreeSet<VizRole>,
}

#[derive(Debug, Clone)]
struct StatementDraft {
    subject: VizValueRef,
    predicate: VizTermId,
    object: VizValueRef,
    asserted_in: BTreeSet<VizGraphId>,
    nesting_depth: u32,
    dialect: VizDialect,
    roles: BTreeSet<VizRole>,
}

#[derive(Debug, Clone)]
struct AssertionDraft {
    statement: VizStatementId,
    graph: VizGraphId,
}

#[derive(Debug, Clone)]
struct GraphDraft {
    term: Option<VizValueRef>,
    label: String,
}

#[derive(Debug)]
struct ProjectionBuilder<'a> {
    spec: &'a VizSpec,
    terms: BTreeMap<VizTermId, TermDraft>,
    term_by_key: BTreeMap<String, VizTermId>,
    statements: BTreeMap<VizStatementId, StatementDraft>,
    statement_by_key: BTreeMap<String, VizStatementId>,
    assertions: BTreeMap<VizAssertionId, AssertionDraft>,
    relations: BTreeMap<VizRelationId, VizRelation>,
    graphs: BTreeMap<VizGraphId, GraphDraft>,
    graph_by_key: BTreeMap<String, VizGraphId>,
    references: BTreeMap<VizReferenceId, VizReference>,
    diagnostics: BTreeMap<String, VizDiagnostic>,
    role_predicates_seen: BTreeSet<String>,
    identity_keys: BTreeMap<String, String>,
}

impl<'a> ProjectionBuilder<'a> {
    fn new(spec: &'a VizSpec) -> Self {
        Self {
            spec,
            terms: BTreeMap::new(),
            term_by_key: BTreeMap::new(),
            statements: BTreeMap::new(),
            statement_by_key: BTreeMap::new(),
            assertions: BTreeMap::new(),
            relations: BTreeMap::new(),
            graphs: BTreeMap::new(),
            graph_by_key: BTreeMap::new(),
            references: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
            role_predicates_seen: BTreeSet::new(),
            identity_keys: BTreeMap::new(),
        }
    }

    fn add_dataset_quad(
        &mut self,
        dataset: &RdfDataset,
        quad: QuadRef<'_>,
    ) -> Result<(), VizError> {
        let subject = term_ref_value(dataset, quad.s);
        let predicate = match quad.p {
            TermRef::Iri(iri) => iri.to_owned(),
            other => return Err(VizError::InvalidPredicate(format!("{other:?}"))),
        };
        let object = term_ref_value(dataset, quad.o);
        let graph = quad.g.map(|g| term_ref_value(dataset, g));
        self.add_assertion(subject, predicate, object, graph)
    }

    fn add_assertion(
        &mut self,
        subject: TermValue,
        predicate: String,
        object: TermValue,
        graph_name: Option<TermValue>,
    ) -> Result<(), VizError> {
        if !self.graph_selected(graph_name.as_ref()) {
            return Ok(());
        }
        let graph = self.graph_id(graph_name)?;
        let statement = self.statement_id(subject, predicate, object)?;
        self.apply_role_rules_to_value(&statement);
        let assertion_key = format!("{}|{}", statement.0, graph.0);
        let assertion = VizAssertionId(self.mint_id("assertion", &assertion_key)?);
        self.assertions.insert(
            assertion,
            AssertionDraft {
                statement: statement.clone(),
                graph: graph.clone(),
            },
        );
        let draft = self
            .statements
            .get_mut(&statement)
            .expect("statement inserted before assertion");
        draft.asserted_in.insert(graph);
        draft.roles.insert(VizRole::AssertedStatement);
        Ok(())
    }

    fn add_reifier(
        &mut self,
        reifier: TermValue,
        triple: TermValue,
        graph_name: Option<TermValue>,
    ) -> Result<(), VizError> {
        if !self.graph_selected(graph_name.as_ref()) {
            return Ok(());
        }
        let reifier_id = self.term_id(reifier)?;
        self.add_term_role(&reifier_id, VizRole::Reifier);
        let TermValue::Triple { s, p, o } = triple else {
            return Err(VizError::InvalidPredicate(
                "rdf:reifies object must be a triple term".to_owned(),
            ));
        };
        let predicate = predicate_iri(*p)?;
        let statement = self.statement_id(*s, predicate, *o)?;
        self.add_statement_role(&statement, VizRole::QuotedStatement);
        let graph = self.graph_id(graph_name)?;
        let relation_key = format!("{}|{}|{}", reifier_id.0, statement.0, graph.0);
        let relation = VizRelationId(self.mint_id("relation-reifies", &relation_key)?);
        self.relations.insert(
            relation.clone(),
            VizRelation::Reifies {
                id: relation.clone(),
                reifier: reifier_id,
                statement: statement.clone(),
                graph,
            },
        );
        self.add_reference(&statement, VizReferenceSite::Reification { relation })?;
        Ok(())
    }

    fn add_annotation(
        &mut self,
        reifier: TermValue,
        predicate: &str,
        object: TermValue,
        graph_name: Option<TermValue>,
    ) -> Result<(), VizError> {
        if !self.graph_selected(graph_name.as_ref()) {
            return Ok(());
        }
        let reifier_id = self.term_id(reifier)?;
        self.add_term_role(&reifier_id, VizRole::Reifier);
        let predicate_id = self.term_id(TermValue::Iri(predicate.to_owned()))?;
        self.add_term_role(&predicate_id, VizRole::Predicate);
        self.role_predicates_seen.insert(predicate.to_owned());
        self.apply_role_rules_to_term(&reifier_id, predicate);
        let object_ref = self.value_ref(object)?;
        let graph = self.graph_id(graph_name)?;
        let relation_key = format!(
            "{}|{}|{}|{}",
            reifier_id.0,
            predicate_id.0,
            value_ref_key(&object_ref),
            graph.0
        );
        let relation = VizRelationId(self.mint_id("relation-annotation", &relation_key)?);
        self.relations.insert(
            relation.clone(),
            VizRelation::Annotation {
                id: relation.clone(),
                reifier: reifier_id,
                predicate: predicate_id,
                object: object_ref.clone(),
                graph,
            },
        );
        if let VizValueRef::Statement { id } = object_ref {
            self.add_reference(&id, VizReferenceSite::Annotation { relation })?;
        }
        Ok(())
    }

    fn add_default_graph(&mut self) {
        if !self.graph_selected(None) {
            return;
        }
        self.graphs
            .entry(VizGraphId(DEFAULT_GRAPH_ID.to_owned()))
            .or_insert_with(|| GraphDraft {
                term: None,
                label: "default graph".to_owned(),
            });
        self.graph_by_key.insert(
            "default".to_owned(),
            VizGraphId(DEFAULT_GRAPH_ID.to_owned()),
        );
    }

    fn graph_id(&mut self, graph: Option<TermValue>) -> Result<VizGraphId, VizError> {
        match graph {
            None => Ok(VizGraphId(DEFAULT_GRAPH_ID.to_owned())),
            Some(value) => {
                let key = term_key(&value);
                if let Some(id) = self.graph_by_key.get(&key) {
                    return Ok(id.clone());
                }
                let id = VizGraphId(self.mint_id("graph", &key)?);
                let term = self.value_ref(value)?;
                let label = match &term {
                    VizValueRef::Term { id } => {
                        self.add_term_role(id, VizRole::GraphName);
                        self.terms
                            .get(id)
                            .map_or_else(|| id.0.clone(), |term| term.label.clone())
                    }
                    VizValueRef::Statement { id } => format!("quoted {}", short_id(&id.0)),
                };
                self.graphs.insert(
                    id.clone(),
                    GraphDraft {
                        term: Some(term.clone()),
                        label,
                    },
                );
                self.graph_by_key.insert(key, id.clone());
                if let VizValueRef::Statement { id: statement } = term {
                    self.add_reference(
                        &statement,
                        VizReferenceSite::GraphName { graph: id.clone() },
                    )?;
                    self.add_graph_diagnostic(&id, "triple term appears as a graph name")?;
                } else if self.graph_term_is_generalized(&id) {
                    self.add_graph_diagnostic(&id, "literal appears as a graph name")?;
                }
                Ok(id)
            }
        }
    }

    fn statement_id(
        &mut self,
        subject: TermValue,
        predicate: String,
        object: TermValue,
    ) -> Result<VizStatementId, VizError> {
        let subject_ref = self.value_ref(subject)?;
        let predicate_id = self.term_id(TermValue::Iri(predicate))?;
        self.add_term_role(&predicate_id, VizRole::Predicate);
        let object_ref = self.value_ref(object)?;
        let key = statement_key(&subject_ref, &predicate_id, &object_ref);
        if let Some(id) = self.statement_by_key.get(&key) {
            return Ok(id.clone());
        }
        let id = VizStatementId(self.mint_id("statement", &key)?);
        let dialect = self.statement_dialect(&subject_ref);
        let nesting_depth = value_ref_depth(&subject_ref, &self.statements)
            .max(value_ref_depth(&object_ref, &self.statements));
        self.statements.insert(
            id.clone(),
            StatementDraft {
                subject: subject_ref.clone(),
                predicate: predicate_id,
                object: object_ref.clone(),
                asserted_in: BTreeSet::new(),
                nesting_depth,
                dialect: dialect.clone(),
                roles: BTreeSet::new(),
            },
        );
        self.statement_by_key.insert(key, id.clone());
        if let VizValueRef::Statement { id: nested } = subject_ref {
            self.add_reference(
                &nested,
                VizReferenceSite::Statement {
                    statement: id.clone(),
                    position: VizPosition::Subject,
                },
            )?;
        }
        if let VizValueRef::Statement { id: nested } = object_ref {
            self.add_reference(
                &nested,
                VizReferenceSite::Statement {
                    statement: id.clone(),
                    position: VizPosition::Object,
                },
            )?;
        }
        match dialect {
            VizDialect::Rdf12 => {}
            VizDialect::SymmetricRdf12 => self.add_statement_diagnostic(
                &id,
                "viz-dialect-symmetric-subject",
                "triple term appears in subject position",
                dialect,
            )?,
            VizDialect::GeneralizedRdf => self.add_statement_diagnostic(
                &id,
                "viz-dialect-generalized-subject",
                "literal appears in subject position",
                dialect,
            )?,
        }
        Ok(id)
    }

    fn value_ref(&mut self, value: TermValue) -> Result<VizValueRef, VizError> {
        match value {
            TermValue::Triple { s, p, o } => {
                let predicate = predicate_iri(*p)?;
                let statement = self.statement_id(*s, predicate, *o)?;
                Ok(VizValueRef::Statement { id: statement })
            }
            other => Ok(VizValueRef::Term {
                id: self.term_id(other)?,
            }),
        }
    }

    fn term_id(&mut self, value: TermValue) -> Result<VizTermId, VizError> {
        let key = term_key(&value);
        if let Some(id) = self.term_by_key.get(&key) {
            return Ok(id.clone());
        }
        let id = VizTermId(self.mint_id("term", &key)?);
        let value = viz_term_value(value)?;
        let label = label_for_term(
            &value,
            self.spec.label_policy.clone(),
            &self.spec.vocabulary,
        );
        self.terms.insert(
            id.clone(),
            TermDraft {
                value,
                label,
                roles: BTreeSet::new(),
            },
        );
        self.term_by_key.insert(key, id.clone());
        Ok(id)
    }

    fn add_term_role(&mut self, id: &VizTermId, role: VizRole) {
        if let Some(term) = self.terms.get_mut(id) {
            term.roles.insert(role);
        }
    }

    fn add_statement_role(&mut self, id: &VizStatementId, role: VizRole) {
        if let Some(statement) = self.statements.get_mut(id) {
            statement.roles.insert(role);
        }
    }

    fn apply_role_rules_to_value(&mut self, statement: &VizStatementId) {
        let Some(draft) = self.statements.get(statement) else {
            return;
        };
        let subject = draft.subject.clone();
        let Some(predicate) = self
            .terms
            .get(&draft.predicate)
            .and_then(|term| match &term.value {
                VizTermValue::Iri { value } => Some(value.clone()),
                _ => None,
            })
        else {
            return;
        };
        let roles = self.roles_for_predicate(&predicate);
        for role in roles {
            self.add_role_to_value(&subject, role);
        }
    }

    fn apply_role_rules_to_term(&mut self, term: &VizTermId, predicate: &str) {
        for role in self.roles_for_predicate(predicate) {
            self.add_term_role(term, role);
        }
    }

    fn roles_for_predicate(&self, predicate: &str) -> Vec<VizRole> {
        self.spec
            .role_rules
            .iter()
            .filter(|rule| rule.predicate_iri == predicate)
            .map(|rule| rule.role.clone())
            .collect()
    }

    fn add_role_to_value(&mut self, value: &VizValueRef, role: VizRole) {
        match value {
            VizValueRef::Term { id } => self.add_term_role(id, role),
            VizValueRef::Statement { id } => self.add_statement_role(id, role),
        }
    }

    fn add_reference(
        &mut self,
        statement: &VizStatementId,
        site: VizReferenceSite,
    ) -> Result<(), VizError> {
        let site_key =
            serde_json::to_string(&site).map_err(|err| VizError::Serialize(err.to_string()))?;
        let id = VizReferenceId(self.mint_id("reference", &format!("{}|{site_key}", statement.0))?);
        self.references.insert(
            id.clone(),
            VizReference {
                id,
                statement: statement.clone(),
                site,
            },
        );
        self.add_statement_role(statement, VizRole::QuotedStatement);
        Ok(())
    }

    fn mint_id(&mut self, prefix: &str, key: &str) -> Result<String, VizError> {
        let id = format!("{prefix}:{}", stable_hash_hex(key));
        if let Some(existing) = self.identity_keys.get(&id) {
            if existing != key {
                return Err(VizError::IdCollision(id));
            }
        } else {
            self.identity_keys.insert(id.clone(), key.to_owned());
        }
        Ok(id)
    }

    fn graph_selected(&self, graph: Option<&TermValue>) -> bool {
        let VizGraphPolicy::Include(selectors) = &self.spec.graph_policy else {
            return true;
        };
        let candidates = match graph {
            None => vec!["default".to_owned(), DEFAULT_GRAPH_ID.to_owned()],
            Some(value) => {
                let key = term_key(value);
                let mut candidates = vec![key.clone(), format!("graph:{}", stable_hash_hex(&key))];
                match value {
                    TermValue::Iri(iri) => {
                        candidates.push(iri.clone());
                        candidates.push(compact_iri(iri));
                    }
                    TermValue::Blank { label, .. } => {
                        candidates.push(label.clone());
                        candidates.push(format!("_:{label}"));
                    }
                    TermValue::Literal { lexical_form, .. } => {
                        candidates.push(lexical_form.clone());
                    }
                    TermValue::Triple { .. } => {}
                }
                candidates
            }
        };
        selectors
            .iter()
            .any(|selector| candidates.iter().any(|candidate| candidate == selector))
    }

    fn statement_dialect(&self, subject: &VizValueRef) -> VizDialect {
        match subject {
            VizValueRef::Statement { .. } => VizDialect::SymmetricRdf12,
            VizValueRef::Term { id }
                if self
                    .terms
                    .get(id)
                    .is_some_and(|term| matches!(term.value, VizTermValue::Literal { .. })) =>
            {
                VizDialect::GeneralizedRdf
            }
            VizValueRef::Term { .. } => VizDialect::Rdf12,
        }
    }

    fn graph_term_is_generalized(&self, graph: &VizGraphId) -> bool {
        let Some(GraphDraft {
            term: Some(VizValueRef::Term { id }),
            ..
        }) = self.graphs.get(graph)
        else {
            return false;
        };
        self.terms
            .get(id)
            .is_some_and(|term| matches!(term.value, VizTermValue::Literal { .. }))
    }

    fn add_statement_diagnostic(
        &mut self,
        statement: &VizStatementId,
        code: &str,
        message: &str,
        dialect: VizDialect,
    ) -> Result<(), VizError> {
        let key = format!("{}|{code}", statement.0);
        let id = self.mint_id("diagnostic", &key)?;
        self.diagnostics.insert(
            id.clone(),
            VizDiagnostic {
                id,
                code: code.to_owned(),
                message: message.to_owned(),
                target: Some(statement.0.clone()),
                dialect,
            },
        );
        Ok(())
    }

    fn add_graph_diagnostic(&mut self, graph: &VizGraphId, message: &str) -> Result<(), VizError> {
        let code = "viz-dialect-generalized-graph-name";
        let key = format!("{}|{code}", graph.0);
        let id = self.mint_id("diagnostic", &key)?;
        self.diagnostics.insert(
            id.clone(),
            VizDiagnostic {
                id,
                code: code.to_owned(),
                message: message.to_owned(),
                target: Some(graph.0.clone()),
                dialect: VizDialect::GeneralizedRdf,
            },
        );
        Ok(())
    }

    fn apply_focus(&mut self) -> Result<(), VizError> {
        let Some(focus) = self.spec.focus.as_deref() else {
            return Ok(());
        };
        let mut term_matches = self
            .terms
            .iter()
            .filter_map(|(id, term)| term_matches_focus(id, term, focus).then_some(id.clone()))
            .collect::<Vec<_>>();
        let mut statement_matches = self
            .statement_by_key
            .iter()
            .filter_map(|(key, id)| {
                (id.0 == focus || key == focus || short_id(&id.0) == focus).then_some(id.clone())
            })
            .collect::<Vec<_>>();
        term_matches.sort();
        term_matches.dedup();
        statement_matches.sort();
        statement_matches.dedup();
        match term_matches.len() + statement_matches.len() {
            0 => Err(VizError::UnknownFocus(focus.to_owned())),
            1 => {
                if let Some(id) = term_matches.first() {
                    self.add_term_role(id, VizRole::Focus);
                } else if let Some(id) = statement_matches.first() {
                    self.add_statement_role(id, VizRole::Focus);
                }
                Ok(())
            }
            _ => Err(VizError::AmbiguousFocus(focus.to_owned())),
        }
    }

    fn validate_spec(&self) -> Result<(), VizError> {
        let mut prefixes = BTreeMap::new();
        let mut namespaces = BTreeMap::new();
        for mapping in &self.spec.vocabulary {
            if mapping.prefix.is_empty() || mapping.namespace.is_empty() {
                return Err(VizError::InvalidVocabulary(
                    "visualization vocabulary mappings require non-empty prefix and namespace"
                        .to_owned(),
                ));
            }
            if let Some(existing) = prefixes.insert(&mapping.prefix, &mapping.namespace)
                && existing != &mapping.namespace
            {
                return Err(VizError::InvalidVocabulary(format!(
                    "visualization prefix {:?} maps to multiple namespaces",
                    mapping.prefix
                )));
            }
            if let Some(existing) = namespaces.insert(&mapping.namespace, &mapping.prefix)
                && existing != &mapping.prefix
            {
                return Err(VizError::InvalidVocabulary(format!(
                    "visualization namespace {:?} maps to multiple prefixes",
                    mapping.namespace
                )));
            }
        }
        if let VizGraphPolicy::Include(selectors) = &self.spec.graph_policy
            && selectors.iter().any(String::is_empty)
        {
            return Err(VizError::InvalidSpec(
                "visualization graph selectors must not be empty".to_owned(),
            ));
        }
        let field_count = self.spec.table_fields.iter().collect::<BTreeSet<_>>().len();
        if field_count != self.spec.table_fields.len() {
            return Err(VizError::InvalidSpec(
                "visualization table fields must be unique".to_owned(),
            ));
        }
        for rule in &self.spec.role_rules {
            if !self.role_predicates_seen.contains(&rule.predicate_iri)
                && !self
                    .terms
                    .values()
                    .any(|term| matches!(&term.value, VizTermValue::Iri { value } if value == &rule.predicate_iri))
            {
                return Err(VizError::UnknownRolePredicate(rule.predicate_iri.clone()));
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Result<VizProjection, VizError> {
        self.validate_spec()?;
        self.apply_focus()?;
        if self.statements.len() > self.spec.max_statements {
            return Err(VizError::TooLarge {
                limit: "statements",
                actual: self.statements.len(),
                allowed: self.spec.max_statements,
            });
        }
        if self.terms.len() > self.spec.max_terms {
            return Err(VizError::TooLarge {
                limit: "terms",
                actual: self.terms.len(),
                allowed: self.spec.max_terms,
            });
        }

        let mut relation_by_reifier: BTreeMap<VizTermId, usize> = BTreeMap::new();
        let mut reifiers_by_statement: BTreeMap<VizStatementId, BTreeSet<VizTermId>> =
            BTreeMap::new();
        for relation in self.relations.values() {
            match relation {
                VizRelation::Reifies {
                    reifier, statement, ..
                } => {
                    reifiers_by_statement
                        .entry(statement.clone())
                        .or_default()
                        .insert(reifier.clone());
                }
                VizRelation::Annotation { reifier, .. } => {
                    *relation_by_reifier.entry(reifier.clone()).or_default() += 1;
                }
            }
        }

        for (statement, reifiers) in &reifiers_by_statement {
            if let Some(draft) = self.statements.get_mut(statement)
                && reifiers
                    .iter()
                    .any(|reifier| relation_by_reifier.contains_key(reifier))
            {
                draft.roles.insert(VizRole::AnnotatedStatement);
            }
        }

        let terms = self
            .terms
            .into_iter()
            .map(|(id, draft)| VizTerm {
                id,
                value: draft.value,
                label: draft.label,
                roles: draft.roles.into_iter().collect(),
            })
            .collect();

        let statements: Vec<VizStatement> = self
            .statements
            .iter()
            .map(|(id, draft)| VizStatement {
                id: id.clone(),
                subject: draft.subject.clone(),
                predicate: draft.predicate.clone(),
                object: draft.object.clone(),
                asserted_in: draft.asserted_in.iter().cloned().collect(),
                nesting_depth: draft.nesting_depth,
                incoming_references: u32::try_from(
                    self.references
                        .values()
                        .filter(|reference| reference.statement == *id)
                        .count(),
                )
                .unwrap_or(u32::MAX),
                dialect: draft.dialect.clone(),
                roles: draft.roles.iter().cloned().collect(),
            })
            .collect();

        let assertions = self
            .assertions
            .into_iter()
            .map(|(id, draft)| VizAssertion {
                id,
                statement: draft.statement,
                graph: draft.graph,
            })
            .collect();

        let relations = self.relations.into_values().collect();

        let graphs = self
            .graphs
            .into_iter()
            .map(|(id, draft)| VizGraph {
                id,
                term: draft.term,
                label: draft.label,
            })
            .collect();

        let references = self.references.into_values().collect();

        let table_rows = statements
            .iter()
            .map(|statement| {
                let reifiers = reifiers_by_statement
                    .get(&statement.id)
                    .cloned()
                    .unwrap_or_default();
                let annotation_count = reifiers
                    .iter()
                    .map(|reifier| {
                        relation_by_reifier
                            .get(reifier)
                            .copied()
                            .unwrap_or_default()
                    })
                    .sum();
                VizTableRow {
                    statement: statement.id.clone(),
                    asserted_in: statement.asserted_in.clone(),
                    reifier_count: reifiers.len(),
                    annotation_count,
                    referenced_by: statement.incoming_references,
                    depth: statement.nesting_depth,
                }
            })
            .collect();

        let table = VizTable {
            fields: self.spec.table_fields.clone(),
            rows: table_rows,
        };

        Ok(VizProjection {
            terms,
            statements,
            assertions,
            relations,
            graphs,
            references,
            table,
            diagnostics: self.diagnostics.into_values().collect(),
        })
    }
}

fn term_ref_value(dataset: &RdfDataset, term: TermRef<'_>) -> TermValue {
    match term {
        TermRef::Iri(iri) => TermValue::Iri(iri.to_owned()),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            let datatype = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("literal datatype must be an IRI, got {other:?}"),
            };
            TermValue::Literal {
                lexical_form: lexical.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }
        }
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(dataset.term_value(s)),
            p: Box::new(dataset.term_value(p)),
            o: Box::new(dataset.term_value(o)),
        },
    }
}

fn predicate_iri(value: TermValue) -> Result<String, VizError> {
    match value {
        TermValue::Iri(iri) => Ok(iri),
        other => Err(VizError::InvalidPredicate(term_key(&other))),
    }
}

fn viz_term_value(value: TermValue) -> Result<VizTermValue, VizError> {
    match value {
        TermValue::Iri(value) => Ok(VizTermValue::Iri { value }),
        TermValue::Blank { label, scope } => Ok(VizTermValue::Blank {
            label,
            scope: scope.ordinal(),
        }),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => Ok(VizTermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction: direction.map(viz_direction),
        }),
        TermValue::Triple { .. } => Err(VizError::InvalidPredicate(
            "triple terms are represented as statements, not ordinary terms".to_owned(),
        )),
    }
}

fn viz_direction(direction: RdfTextDirection) -> VizTextDirection {
    match direction {
        RdfTextDirection::Ltr => VizTextDirection::Ltr,
        RdfTextDirection::Rtl => VizTextDirection::Rtl,
    }
}

fn statement_key(subject: &VizValueRef, predicate: &VizTermId, object: &VizValueRef) -> String {
    format!(
        "s={} p={} o={}",
        value_ref_key(subject),
        predicate.0,
        value_ref_key(object)
    )
}

fn value_ref_key(value: &VizValueRef) -> &str {
    match value {
        VizValueRef::Term { id } => &id.0,
        VizValueRef::Statement { id } => &id.0,
    }
}

fn value_ref_depth(
    value: &VizValueRef,
    statements: &BTreeMap<VizStatementId, StatementDraft>,
) -> u32 {
    match value {
        VizValueRef::Term { .. } => 0,
        VizValueRef::Statement { id } => statements
            .get(id)
            .map_or(1, |statement| statement.nesting_depth + 1),
    }
}

fn term_key(value: &TermValue) -> String {
    let mut out = String::new();
    write_term_key(value, &mut out).expect("writing to String cannot fail");
    out
}

fn write_term_key(value: &TermValue, out: &mut String) -> fmt::Result {
    match value {
        TermValue::Iri(iri) => {
            out.write_str("iri:")?;
            write_json_string(iri, out)
        }
        TermValue::Blank { label, scope } => {
            write!(out, "blank:{}:", scope.ordinal())?;
            write_json_string(label, out)
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.write_str("literal:")?;
            write_json_string(lexical_form, out)?;
            out.write_char(':')?;
            write_json_string(datatype, out)?;
            out.write_char(':')?;
            if let Some(language) = language {
                write_json_string(language, out)?;
            }
            out.write_char(':')?;
            if let Some(direction) = direction {
                out.write_str(direction.as_str())?;
            }
            Ok(())
        }
        TermValue::Triple { s, p, o } => {
            out.write_str("triple(")?;
            write_term_key(s, out)?;
            out.write_char('|')?;
            write_term_key(p, out)?;
            out.write_char('|')?;
            write_term_key(o, out)?;
            out.write_char(')')
        }
    }
}

fn write_json_string(value: &str, out: &mut String) -> fmt::Result {
    let encoded = serde_json::to_string(value).expect("string serialization cannot fail");
    out.write_str(&encoded)
}

fn label_for_term(
    value: &VizTermValue,
    policy: VizLabelPolicy,
    vocabulary: &[VizVocabularyMapping],
) -> String {
    match (policy, value) {
        (VizLabelPolicy::Full, _) => {
            serde_json::to_string(value).unwrap_or_else(|_| "?".to_owned())
        }
        (_, VizTermValue::Iri { value }) => vocabulary
            .iter()
            .filter(|mapping| value.starts_with(&mapping.namespace))
            .max_by(|left, right| {
                left.namespace
                    .len()
                    .cmp(&right.namespace.len())
                    .then_with(|| right.prefix.cmp(&left.prefix))
            })
            .map_or_else(
                || compact_iri(value),
                |mapping| format!("{}:{}", mapping.prefix, &value[mapping.namespace.len()..]),
            ),
        (_, VizTermValue::Blank { label, scope }) if *scope == 0 => format!("_:{label}"),
        (_, VizTermValue::Blank { label, scope }) => format!("_:{label}.s{scope}"),
        (
            _,
            VizTermValue::Literal {
                lexical_form,
                language,
                direction,
                ..
            },
        ) => {
            let mut label = format!("\"{lexical_form}\"");
            if let Some(language) = language {
                label.push('@');
                label.push_str(language);
            }
            if let Some(direction) = direction {
                label.push(' ');
                label.push_str(match direction {
                    VizTextDirection::Ltr => "ltr",
                    VizTextDirection::Rtl => "rtl",
                });
            }
            label
        }
    }
}

fn term_matches_focus(id: &VizTermId, term: &TermDraft, focus: &str) -> bool {
    if id.0 == focus || term.label == focus {
        return true;
    }
    match &term.value {
        VizTermValue::Iri { value } => value == focus,
        VizTermValue::Blank { label, .. } => label == focus || format!("_:{label}") == focus,
        VizTermValue::Literal { lexical_form, .. } => lexical_form == focus,
    }
}

fn short_id(id: &str) -> String {
    id.rsplit(':')
        .next()
        .map_or_else(|| id.to_owned(), |suffix| suffix.chars().take(8).collect())
}

fn compact_iri(iri: &str) -> String {
    iri.rsplit(['#', '/'])
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(iri)
        .to_owned()
}

/// Compute a deterministic non-cryptographic hash over text.
pub fn stable_hash_hex(input: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfDatasetBuilder, RdfLiteral};

    const EX: &str = "https://example.org/";
    const KNOWS: &str = "https://example.org/knows";
    const CLAIM: &str = "https://example.org/claim";
    const CAROL: &str = "https://example.org/carol";
    const ATTRIBUTED_TO: &str = "https://example.org/attributedTo";
    const CONFIDENCE: &str = "https://example.org/confidence";

    fn iri(value: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{value}"))
    }

    fn example_dataset() -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let alice = b.intern_iri(&format!("{EX}alice"));
        let bob = b.intern_iri(&format!("{EX}bob"));
        let knows = b.intern_iri(KNOWS);
        let claim = b.intern_iri(CLAIM);
        let carol = b.intern_iri(CAROL);
        let attributed_to = b.intern_iri(ATTRIBUTED_TO);
        let confidence = b.intern_iri(CONFIDENCE);
        let confidence_value = b.intern_literal(RdfLiteral::typed(
            "0.8",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        let statement = b.intern_triple(alice, knows, bob);
        b.push_quad(alice, knows, bob, None);
        b.push_reifier(claim, statement);
        b.push_annotation(claim, attributed_to, carol);
        b.push_annotation(claim, confidence, confidence_value);
        b.freeze().expect("valid dataset")
    }

    #[test]
    fn projection_separates_assertion_statement_reifier_and_annotations() {
        let ds = example_dataset();
        let projection = project_dataset(&ds, &VizSpec::default()).expect("project");
        assert_eq!(projection.statements.len(), 1);
        assert_eq!(projection.assertions.len(), 1);
        assert_eq!(projection.relations.len(), 3);
        let statement = &projection.statements[0];
        assert_eq!(statement.asserted_in.len(), 1);
        assert_eq!(statement.incoming_references, 1);
        assert!(statement.roles.contains(&VizRole::AssertedStatement));
        assert!(statement.roles.contains(&VizRole::AnnotatedStatement));
        let row = &projection.table.rows[0];
        assert_eq!(row.reifier_count, 1);
        assert_eq!(row.annotation_count, 2);
    }

    #[test]
    fn quoted_only_triple_is_not_asserted() {
        let input = VizGraphInput {
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                },
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert_eq!(projection.statements.len(), 1);
        assert!(projection.assertions.is_empty());
        assert!(projection.statements[0].asserted_in.is_empty());
        assert!(
            !projection.statements[0]
                .roles
                .contains(&VizRole::AnnotatedStatement)
        );
    }

    #[test]
    fn one_reifier_can_cover_multiple_statements() {
        let input = VizGraphInput {
            reifiers: vec![
                VizInputReifier {
                    reifier: iri("claim"),
                    statement: VizInputStatement {
                        subject: iri("alice"),
                        predicate: KNOWS.to_owned(),
                        object: iri("bob"),
                    },
                    graph_name: None,
                },
                VizInputReifier {
                    reifier: iri("claim"),
                    statement: VizInputStatement {
                        subject: iri("bob"),
                        predicate: KNOWS.to_owned(),
                        object: iri("carol"),
                    },
                    graph_name: None,
                },
            ],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert_eq!(projection.statements.len(), 2);
        assert_eq!(
            projection
                .table
                .rows
                .iter()
                .map(|row| row.reifier_count)
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn directional_literals_keep_direction() {
        let mut lit = RdfLiteral::language_tagged("مرحبا", "ar");
        lit.direction = Some(RdfTextDirection::Rtl);
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: iri("alice"),
                predicate: format!("{EX}says"),
                object: TermValue::Literal {
                    lexical_form: lit.lexical_form,
                    datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
                    language: Some("ar".to_owned()),
                    direction: Some(RdfTextDirection::Rtl),
                },
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert!(projection.terms.iter().any(|term| {
            matches!(
                &term.value,
                VizTermValue::Literal {
                    direction: Some(VizTextDirection::Rtl),
                    ..
                }
            )
        }));
    }

    #[test]
    fn triple_term_subject_gets_symmetric_dialect_diagnostic() {
        let nested = TermValue::Triple {
            s: Box::new(iri("alice")),
            p: Box::new(TermValue::Iri(KNOWS.to_owned())),
            o: Box::new(iri("bob")),
        };
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: nested,
                predicate: format!("{EX}reportedBy"),
                object: iri("carol"),
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert!(
            projection
                .diagnostics
                .iter()
                .any(|diag| diag.code == "viz-dialect-symmetric-subject")
        );
        assert!(
            projection
                .statements
                .iter()
                .any(|statement| statement.dialect == VizDialect::SymmetricRdf12)
        );
    }

    #[test]
    fn invalid_role_rule_hard_errors() {
        let spec = VizSpec {
            role_rules: vec![VizRoleRule {
                predicate_iri: format!("{EX}missing"),
                role: VizRole::Custom("important".to_owned()),
            }],
            ..VizSpec::default()
        };
        let err = project_graph_input(&VizGraphInput::default(), &spec).expect_err("bad spec");
        assert!(matches!(err, VizError::UnknownRolePredicate(_)));
    }

    #[test]
    fn role_rules_apply_to_assertion_subjects_and_annotation_reifiers() {
        let important = VizRole::Custom("important".to_owned());
        let reviewed = VizRole::Custom("reviewed".to_owned());
        let spec = VizSpec {
            role_rules: vec![
                VizRoleRule {
                    predicate_iri: KNOWS.to_owned(),
                    role: important.clone(),
                },
                VizRoleRule {
                    predicate_iri: ATTRIBUTED_TO.to_owned(),
                    role: reviewed.clone(),
                },
            ],
            ..VizSpec::default()
        };
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: iri("alice"),
                predicate: KNOWS.to_owned(),
                object: iri("bob"),
                graph_name: None,
            }],
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                },
                graph_name: None,
            }],
            annotations: vec![VizInputAnnotation {
                reifier: iri("claim"),
                predicate: ATTRIBUTED_TO.to_owned(),
                object: iri("carol"),
                graph_name: None,
            }],
        };
        let projection = project_graph_input(&input, &spec).expect("project");
        let alice = projection
            .terms
            .iter()
            .find(|term| term.label == "alice")
            .expect("alice term");
        assert!(alice.roles.contains(&important));
        let claim = projection
            .terms
            .iter()
            .find(|term| term.label == "claim")
            .expect("claim term");
        assert!(claim.roles.contains(&reviewed));
    }

    #[test]
    fn graph_like_inputs_are_deterministic() {
        let input = VizGraphInput {
            quads: vec![
                VizInputQuad {
                    subject: iri("bob"),
                    predicate: KNOWS.to_owned(),
                    object: iri("carol"),
                    graph_name: None,
                },
                VizInputQuad {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                    graph_name: None,
                },
            ],
            ..VizGraphInput::default()
        };
        let a = project_graph_input(&input, &VizSpec::default()).expect("project");
        let b = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert_eq!(a, b);
    }

    #[test]
    fn asserted_only_statement_is_not_quoted() {
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: iri("alice"),
                predicate: KNOWS.to_owned(),
                object: iri("bob"),
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        let statement = &projection.statements[0];
        assert!(statement.roles.contains(&VizRole::AssertedStatement));
        assert!(!statement.roles.contains(&VizRole::QuotedStatement));
        assert_eq!(statement.incoming_references, 0);
    }

    #[test]
    fn structural_ids_and_projection_are_input_order_independent() {
        let first = rich_input();
        let mut reversed = first.clone();
        reversed.quads.reverse();
        reversed.reifiers.reverse();
        reversed.annotations.reverse();
        let a = project_graph_input(&first, &VizSpec::default()).expect("project first");
        let b = project_graph_input(&reversed, &VizSpec::default()).expect("project reversed");
        assert_eq!(a, b);
        assert!(
            a.terms
                .iter()
                .all(|term| term.id.0.starts_with("term:") && term.id.0.len() == 21)
        );
        assert!(a.statements.iter().all(|statement| {
            statement.id.0.starts_with("statement:") && statement.id.0.len() == 26
        }));
    }

    #[test]
    fn references_record_exact_containing_sites() {
        let nested = TermValue::Triple {
            s: Box::new(iri("alice")),
            p: Box::new(TermValue::Iri(KNOWS.to_owned())),
            o: Box::new(iri("bob")),
        };
        let input = VizGraphInput {
            quads: vec![
                VizInputQuad {
                    subject: nested.clone(),
                    predicate: format!("{EX}reportedBy"),
                    object: iri("carol"),
                    graph_name: None,
                },
                VizInputQuad {
                    subject: iri("carol"),
                    predicate: format!("{EX}disputes"),
                    object: nested,
                    graph_name: None,
                },
            ],
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                },
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        let inner = projection
            .statements
            .iter()
            .find(|statement| {
                projection
                    .terms
                    .iter()
                    .find(|term| term.id == statement.predicate)
                    .is_some_and(|term| term.label == "knows")
            })
            .expect("inner statement");
        let sites = projection
            .references
            .iter()
            .filter(|reference| reference.statement == inner.id)
            .map(|reference| &reference.site)
            .collect::<Vec<_>>();
        assert_eq!(sites.len(), 3);
        assert!(sites.iter().any(|site| matches!(
            site,
            VizReferenceSite::Statement {
                position: VizPosition::Subject,
                ..
            }
        )));
        assert!(sites.iter().any(|site| matches!(
            site,
            VizReferenceSite::Statement {
                position: VizPosition::Object,
                ..
            }
        )));
        assert!(
            sites
                .iter()
                .any(|site| matches!(site, VizReferenceSite::Reification { .. }))
        );
        assert_eq!(inner.incoming_references, 3);
    }

    #[test]
    fn graph_filter_focus_vocabulary_and_table_fields_are_operational() {
        let input = rich_input();
        let spec = VizSpec {
            focus: Some(format!("{EX}alice")),
            vocabulary: vec![VizVocabularyMapping {
                prefix: "ex".to_owned(),
                namespace: EX.to_owned(),
            }],
            graph_policy: VizGraphPolicy::Include(vec![format!("{EX}facts")]),
            table_fields: vec![VizTableField::Statement, VizTableField::AssertedIn],
            ..VizSpec::default()
        };
        let projection = project_graph_input(&input, &spec).expect("project");
        assert_eq!(projection.assertions.len(), 2);
        assert!(projection.relations.is_empty());
        assert!(
            projection
                .graphs
                .iter()
                .all(|graph| graph.label == "ex:facts")
        );
        let alice = projection
            .terms
            .iter()
            .find(|term| term.label == "ex:alice")
            .expect("focused alice");
        assert!(alice.roles.contains(&VizRole::Focus));
        assert_eq!(
            projection.table.fields,
            vec![VizTableField::Statement, VizTableField::AssertedIn]
        );
    }

    #[test]
    fn generalized_literal_subject_is_explicit() {
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: TermValue::Literal {
                    lexical_form: "subject".to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#string".to_owned(),
                    language: None,
                    direction: None,
                },
                predicate: KNOWS.to_owned(),
                object: iri("bob"),
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let projection = project_graph_input(&input, &VizSpec::default()).expect("project");
        assert_eq!(projection.statements[0].dialect, VizDialect::GeneralizedRdf);
        assert!(
            projection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "viz-dialect-generalized-subject")
        );
    }

    #[test]
    fn invalid_focus_and_duplicate_table_fields_hard_error() {
        let missing_focus = VizSpec {
            focus: Some(format!("{EX}missing")),
            ..VizSpec::default()
        };
        let err = project_graph_input(&rich_input(), &missing_focus).expect_err("missing focus");
        assert!(matches!(err, VizError::UnknownFocus(_)));

        let duplicate_fields = VizSpec {
            table_fields: vec![VizTableField::Statement, VizTableField::Statement],
            ..VizSpec::default()
        };
        let err =
            project_graph_input(&rich_input(), &duplicate_fields).expect_err("duplicate fields");
        assert!(matches!(err, VizError::InvalidSpec(_)));
    }

    #[test]
    fn size_limits_hard_error() {
        let spec = VizSpec {
            max_statements: 0,
            ..VizSpec::default()
        };
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: iri("alice"),
                predicate: KNOWS.to_owned(),
                object: iri("bob"),
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let err = project_graph_input(&input, &spec).expect_err("too large");
        assert!(matches!(
            err,
            VizError::TooLarge {
                limit: "statements",
                ..
            }
        ));
    }

    fn rich_input() -> VizGraphInput {
        VizGraphInput {
            quads: vec![
                VizInputQuad {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                    graph_name: Some(iri("facts")),
                },
                VizInputQuad {
                    subject: iri("bob"),
                    predicate: KNOWS.to_owned(),
                    object: iri("carol"),
                    graph_name: Some(iri("facts")),
                },
            ],
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement: VizInputStatement {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                },
                graph_name: Some(iri("claims")),
            }],
            annotations: vec![VizInputAnnotation {
                reifier: iri("claim"),
                predicate: ATTRIBUTED_TO.to_owned(),
                object: iri("carol"),
                graph_name: Some(iri("provenance")),
            }],
        }
    }
}

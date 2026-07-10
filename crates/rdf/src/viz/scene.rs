// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Renderer-neutral semantic scenes for RDF 1.2 visualization.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::*;

/// Version of the renderer-neutral semantic scene contract.
pub const VIZ_SCENE_SCHEMA_VERSION: &str = "purrdf-viz-scene-1";

/// Renderer-neutral semantic scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizScene {
    /// Scene schema version.
    pub schema_version: String,
    /// Visual projection mode.
    pub mode: VizMode,
    /// Semantic nodes.
    pub nodes: Vec<VizSceneNode>,
    /// Semantic edges.
    pub edges: Vec<VizSceneEdge>,
    /// Non-geometric scene groups.
    pub groups: Vec<VizSceneGroup>,
    /// Legend entries required to decode the visual grammar.
    pub legend: Vec<VizLegendEntry>,
    /// Statement table when `mode` is [`VizMode::Table`].
    pub table: Option<VizSceneTable>,
}

/// Typed semantic identity bound to scene elements.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "camelCase")]
pub enum VizSemanticRef {
    /// RDF term.
    Term(VizTermId),
    /// Structural RDF statement.
    Statement(VizStatementId),
    /// Assertion occurrence.
    Assertion(VizAssertionId),
    /// Reification or annotation relation.
    Relation(VizRelationId),
    /// Graph context.
    Graph(VizGraphId),
    /// Triple-term reference occurrence.
    Reference(VizReferenceId),
    /// Dialect/conformance diagnostic.
    Diagnostic(String),
}

/// Renderer-neutral node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneNode {
    /// Stable scene id.
    pub id: String,
    /// Semantic identities represented by this node.
    pub bindings: Vec<VizSemanticRef>,
    /// Node visual grammar.
    pub kind: VizSceneNodeKind,
    /// Human-facing label.
    pub label: VizSceneLabel,
    /// Typed ports used by edges.
    pub ports: Vec<VizPort>,
    /// Semantic status badges.
    pub badges: Vec<VizBadge>,
    /// Accessible title and description.
    pub accessibility: VizAccessibility,
}

/// Node grammar independent of renderer styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizSceneNodeKind {
    /// IRI resource.
    Iri,
    /// Blank node.
    Blank,
    /// Literal value.
    Literal,
    /// Structural statement capsule or incidence node.
    Statement,
}

/// Renderer-neutral edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneEdge {
    /// Stable scene id.
    pub id: String,
    /// Semantic identities represented by this edge.
    pub bindings: Vec<VizSemanticRef>,
    /// Edge visual grammar.
    pub kind: VizSceneEdgeKind,
    /// Source endpoint.
    pub source: VizEndpoint,
    /// Target endpoint.
    pub target: VizEndpoint,
    /// Human-facing edge label.
    pub label: VizSceneLabel,
    /// Relation-level semantic badges.
    pub badges: Vec<VizBadge>,
    /// Optional addressable statement anchor carried by this edge.
    pub anchor: Option<VizEdgeAnchor>,
    /// Accessible title and description.
    pub accessibility: VizAccessibility,
}

/// Edge grammar independent of renderer styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizSceneEdgeKind {
    /// Asserted predicate edge.
    Assertion,
    /// Subject incidence relation.
    Subject,
    /// Predicate incidence relation.
    Predicate,
    /// Object incidence relation.
    Object,
    /// Reifier-to-statement relation.
    Reifies,
    /// Annotation relation from a reifier.
    Annotation,
    /// Compact quoted-statement subject tether.
    QuoteSubject,
    /// Compact quoted-statement object tether.
    QuoteObject,
}

/// Typed port on a scene node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizPort {
    /// Stable port id, local to the node.
    pub id: String,
    /// Port role.
    pub kind: VizPortKind,
}

/// Port role used by layout and emitters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizPortKind {
    /// General incoming edge port.
    In,
    /// General outgoing edge port.
    Out,
    /// Subject role port on a statement.
    Subject,
    /// Predicate role port on a statement.
    Predicate,
    /// Object role port on a statement.
    Object,
    /// Reification/annotation attachment port on a statement.
    Relation,
}

/// Endpoint on a node port or on an addressable assertion edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VizEndpoint {
    /// Endpoint on a node port.
    NodePort {
        /// Scene node id.
        node: String,
        /// Node-local port id.
        port: String,
    },
    /// Endpoint on a statement anchor carried by an assertion edge.
    EdgeAnchor {
        /// Scene assertion edge id.
        edge: String,
        /// Edge-local anchor id.
        anchor: String,
    },
}

/// Addressable structural-statement identity carried by an assertion edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizEdgeAnchor {
    /// Stable edge-local anchor id.
    pub id: String,
    /// Structural statement and related identities.
    pub bindings: Vec<VizSemanticRef>,
    /// Compact anchor label.
    pub label: VizSceneLabel,
    /// Anchor status badges.
    pub badges: Vec<VizBadge>,
    /// Accessible title and description.
    pub accessibility: VizAccessibility,
}

/// Display label with full RDF text and language direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneLabel {
    /// Compact display text.
    pub text: String,
    /// Full non-truncated text.
    pub full_text: String,
    /// Language tag when applicable.
    pub language: Option<String>,
    /// RDF 1.2 base direction when applicable.
    pub direction: Option<VizTextDirection>,
}

/// Semantic badge attached to a node, edge, or edge anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizBadge {
    /// Badge grammar.
    pub kind: VizBadgeKind,
    /// Human-facing badge label.
    pub label: String,
    /// Semantic identity represented by the badge.
    pub binding: Option<VizSemanticRef>,
}

/// Badge grammar independent of renderer styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizBadgeKind {
    /// Assertion state.
    Asserted,
    /// Triple-term quotation/reference state.
    Quoted,
    /// Reifier role.
    Reifier,
    /// Named/default graph context.
    Graph,
    /// Annotation count.
    AnnotationCount,
    /// Reifier count.
    ReifierCount,
    /// Incoming reference count.
    ReferenceCount,
    /// Structural nesting depth.
    NestingDepth,
    /// Dialect/conformance status.
    Dialect,
    /// Directional literal direction.
    Direction,
    /// Caller-selected focus.
    Focus,
    /// Caller-supplied visual role.
    Role,
}

/// Accessible text carried independently of concrete SVG elements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizAccessibility {
    /// Concise accessible name.
    pub title: String,
    /// Complete semantic description.
    pub description: String,
}

/// Scene group that does not impose geometry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneGroup {
    /// Stable group id.
    pub id: String,
    /// Group grammar.
    pub kind: VizSceneGroupKind,
    /// Human-facing group label.
    pub label: String,
    /// Scene or legend entry ids in the group.
    pub members: Vec<String>,
}

/// Scene group grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VizSceneGroupKind {
    /// Visual grammar legend.
    Legend,
}

/// One legend entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizLegendEntry {
    /// Stable legend entry id.
    pub id: String,
    /// Node, edge, or badge grammar name.
    pub symbol: String,
    /// Human-facing explanation.
    pub label: String,
}

/// Renderer-neutral statement table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneTable {
    /// Ordered columns.
    pub fields: Vec<VizTableField>,
    /// Ordered rows.
    pub rows: Vec<VizSceneTableRow>,
}

/// Renderer-neutral statement table row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneTableRow {
    /// Stable row id.
    pub id: String,
    /// Structural statement represented by the row.
    pub binding: VizSemanticRef,
    /// Cells in column order.
    pub cells: Vec<VizSceneTableCell>,
}

/// Renderer-neutral statement table cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VizSceneTableCell {
    /// Column represented by the cell.
    pub field: VizTableField,
    /// Human-facing cell text.
    pub text: String,
    /// Semantic identities represented by the cell.
    pub bindings: Vec<VizSemanticRef>,
}

/// Build a renderer-neutral scene from an existing semantic projection.
pub fn build_scene(projection: &VizProjection, spec: &VizSpec) -> Result<VizScene, VizError> {
    let mut scene = match spec.mode {
        VizMode::Compact => build_compact_scene(projection),
        VizMode::Incidence => build_incidence_scene(projection),
        VizMode::Table => build_table_scene(projection),
    };
    add_legend_group(&mut scene);
    validate_scene(&scene)?;
    Ok(scene)
}

/// Project a dataset and build its renderer-neutral scene.
pub fn project_dataset_scene(
    dataset: &RdfDataset,
    spec: &VizSpec,
) -> Result<(VizProjection, VizScene), VizError> {
    let projection = project_dataset(dataset, spec)?;
    let scene = build_scene(&projection, spec)?;
    Ok((projection, scene))
}

/// Project graph-like input and build its renderer-neutral scene.
pub fn project_graph_input_scene(
    input: &VizGraphInput,
    spec: &VizSpec,
) -> Result<(VizProjection, VizScene), VizError> {
    let projection = project_graph_input(input, spec)?;
    let scene = build_scene(&projection, spec)?;
    Ok((projection, scene))
}

fn build_compact_scene(projection: &VizProjection) -> VizScene {
    let terms = term_map(projection);
    let statements = statement_map(projection);
    let relation_summary = relation_summary(projection);
    let visible_terms = compact_visible_terms(projection);
    let mut nodes = visible_terms
        .iter()
        .filter_map(|id| terms.get(id).map(|term| term_node(term)))
        .collect::<Vec<_>>();
    for statement in projection
        .statements
        .iter()
        .filter(|statement| statement.asserted_in.is_empty())
    {
        nodes.push(statement_node(
            statement,
            projection,
            &terms,
            &relation_summary,
            true,
        ));
    }
    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    let mut edges = Vec::new();
    for statement in &projection.statements {
        if statement.asserted_in.is_empty() {
            let statement_node_id = scene_statement_node_id(&statement.id);
            edges.push(VizSceneEdge {
                id: format!("edge-quote-subject-{}", id_suffix(&statement.id.0)),
                bindings: statement_reference_bindings(projection, &statement.id),
                kind: VizSceneEdgeKind::QuoteSubject,
                source: compact_value_endpoint(&statement.subject, &statements, true),
                target: node_endpoint(&statement_node_id, "subject"),
                label: plain_label("subject"),
                badges: Vec::new(),
                anchor: None,
                accessibility: VizAccessibility {
                    title: "quoted statement subject".to_owned(),
                    description: format!(
                        "{} is the subject of quoted statement {}.",
                        value_display(&statement.subject, &terms),
                        statement_display(statement, &terms)
                    ),
                },
            });
            edges.push(VizSceneEdge {
                id: format!("edge-quote-object-{}", id_suffix(&statement.id.0)),
                bindings: statement_reference_bindings(projection, &statement.id),
                kind: VizSceneEdgeKind::QuoteObject,
                source: node_endpoint(&statement_node_id, "object"),
                target: compact_value_endpoint(&statement.object, &statements, false),
                label: plain_label("object"),
                badges: Vec::new(),
                anchor: None,
                accessibility: VizAccessibility {
                    title: "quoted statement object".to_owned(),
                    description: format!(
                        "{} is the object of quoted statement {}.",
                        value_display(&statement.object, &terms),
                        statement_display(statement, &terms)
                    ),
                },
            });
        } else {
            edges.push(assertion_edge(
                statement,
                projection,
                &terms,
                &statements,
                &relation_summary,
            ));
        }
    }
    add_relation_edges(
        &mut edges,
        projection,
        &terms,
        &statements,
        VizMode::Compact,
    );
    edges.sort_by(|left, right| left.id.cmp(&right.id));

    VizScene {
        schema_version: VIZ_SCENE_SCHEMA_VERSION.to_owned(),
        mode: VizMode::Compact,
        nodes,
        edges,
        groups: Vec::new(),
        legend: compact_legend(),
        table: None,
    }
}

fn build_incidence_scene(projection: &VizProjection) -> VizScene {
    let terms = term_map(projection);
    let statements = statement_map(projection);
    let relation_summary = relation_summary(projection);
    let mut nodes = projection.terms.iter().map(term_node).collect::<Vec<_>>();
    nodes.extend(
        projection.statements.iter().map(|statement| {
            statement_node(statement, projection, &terms, &relation_summary, false)
        }),
    );
    nodes.sort_by(|left, right| left.id.cmp(&right.id));

    let mut edges = Vec::new();
    for statement in &projection.statements {
        let target_node = scene_statement_node_id(&statement.id);
        let roles = [
            (
                VizSceneEdgeKind::Subject,
                "subject",
                &statement.subject,
                "subject",
            ),
            (
                VizSceneEdgeKind::Predicate,
                "predicate",
                &VizValueRef::Term {
                    id: statement.predicate.clone(),
                },
                "predicate",
            ),
            (
                VizSceneEdgeKind::Object,
                "object",
                &statement.object,
                "object",
            ),
        ];
        for (kind, role, value, port) in roles {
            edges.push(VizSceneEdge {
                id: format!("edge-{role}-{}", id_suffix(&statement.id.0)),
                bindings: vec![VizSemanticRef::Statement(statement.id.clone())],
                kind,
                source: incidence_value_endpoint(value),
                target: node_endpoint(&target_node, port),
                label: plain_label(role),
                badges: Vec::new(),
                anchor: None,
                accessibility: VizAccessibility {
                    title: format!("statement {role}"),
                    description: format!(
                        "{} fills the {role} port of {}.",
                        value_display(value, &terms),
                        statement_display(statement, &terms)
                    ),
                },
            });
        }
    }
    add_relation_edges(
        &mut edges,
        projection,
        &terms,
        &statements,
        VizMode::Incidence,
    );
    edges.sort_by(|left, right| left.id.cmp(&right.id));

    VizScene {
        schema_version: VIZ_SCENE_SCHEMA_VERSION.to_owned(),
        mode: VizMode::Incidence,
        nodes,
        edges,
        groups: Vec::new(),
        legend: incidence_legend(),
        table: None,
    }
}

fn build_table_scene(projection: &VizProjection) -> VizScene {
    let terms = term_map(projection);
    let graphs = graph_map(projection);
    let relation_summary = relation_summary(projection);
    let rows = projection
        .table
        .rows
        .iter()
        .filter_map(|row| {
            let statement = projection
                .statements
                .iter()
                .find(|statement| statement.id == row.statement)?;
            let summary = relation_summary
                .get(&row.statement)
                .cloned()
                .unwrap_or_default();
            let diagnostics = projection
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.target.as_deref() == Some(&row.statement.0))
                .collect::<Vec<_>>();
            let cells = projection
                .table
                .fields
                .iter()
                .map(|field| match field {
                    VizTableField::Statement => VizSceneTableCell {
                        field: *field,
                        text: statement_display(statement, &terms),
                        bindings: vec![VizSemanticRef::Statement(statement.id.clone())],
                    },
                    VizTableField::AssertedIn => VizSceneTableCell {
                        field: *field,
                        text: row
                            .asserted_in
                            .iter()
                            .map(|id| {
                                graphs
                                    .get(id)
                                    .map_or(id.0.as_str(), |graph| graph.label.as_str())
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                        bindings: row
                            .asserted_in
                            .iter()
                            .cloned()
                            .map(VizSemanticRef::Graph)
                            .collect(),
                    },
                    VizTableField::Reifiers => VizSceneTableCell {
                        field: *field,
                        text: row.reifier_count.to_string(),
                        bindings: summary
                            .reifiers
                            .iter()
                            .cloned()
                            .map(VizSemanticRef::Term)
                            .collect(),
                    },
                    VizTableField::Annotations => VizSceneTableCell {
                        field: *field,
                        text: row.annotation_count.to_string(),
                        bindings: summary
                            .annotation_relations
                            .iter()
                            .cloned()
                            .map(VizSemanticRef::Relation)
                            .collect(),
                    },
                    VizTableField::ReferencedBy => VizSceneTableCell {
                        field: *field,
                        text: row.referenced_by.to_string(),
                        bindings: projection
                            .references
                            .iter()
                            .filter(|reference| reference.statement == row.statement)
                            .map(|reference| VizSemanticRef::Reference(reference.id.clone()))
                            .collect(),
                    },
                    VizTableField::Depth => VizSceneTableCell {
                        field: *field,
                        text: row.depth.to_string(),
                        bindings: vec![VizSemanticRef::Statement(statement.id.clone())],
                    },
                    VizTableField::Diagnostics => VizSceneTableCell {
                        field: *field,
                        text: diagnostics
                            .iter()
                            .map(|diagnostic| diagnostic.code.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                        bindings: diagnostics
                            .iter()
                            .map(|diagnostic| VizSemanticRef::Diagnostic(diagnostic.id.clone()))
                            .collect(),
                    },
                })
                .collect();
            Some(VizSceneTableRow {
                id: format!("row-{}", id_suffix(&row.statement.0)),
                binding: VizSemanticRef::Statement(row.statement.clone()),
                cells,
            })
        })
        .collect();
    VizScene {
        schema_version: VIZ_SCENE_SCHEMA_VERSION.to_owned(),
        mode: VizMode::Table,
        nodes: Vec::new(),
        edges: Vec::new(),
        groups: Vec::new(),
        legend: table_legend(),
        table: Some(VizSceneTable {
            fields: projection.table.fields.clone(),
            rows,
        }),
    }
}

#[derive(Debug, Clone, Default)]
struct StatementRelationSummary {
    reifiers: BTreeSet<VizTermId>,
    reification_relations: BTreeSet<VizRelationId>,
    annotation_relations: BTreeSet<VizRelationId>,
}

fn relation_summary(
    projection: &VizProjection,
) -> BTreeMap<VizStatementId, StatementRelationSummary> {
    let mut summaries = BTreeMap::new();
    let mut statements_by_reifier: BTreeMap<VizTermId, BTreeSet<VizStatementId>> = BTreeMap::new();
    for relation in &projection.relations {
        if let VizRelation::Reifies {
            id,
            reifier,
            statement,
            ..
        } = relation
        {
            let summary = summaries
                .entry(statement.clone())
                .or_insert_with(StatementRelationSummary::default);
            summary.reifiers.insert(reifier.clone());
            summary.reification_relations.insert(id.clone());
            statements_by_reifier
                .entry(reifier.clone())
                .or_default()
                .insert(statement.clone());
        }
    }
    for relation in &projection.relations {
        if let VizRelation::Annotation { id, reifier, .. } = relation
            && let Some(statements) = statements_by_reifier.get(reifier)
        {
            for statement in statements {
                summaries
                    .entry(statement.clone())
                    .or_default()
                    .annotation_relations
                    .insert(id.clone());
            }
        }
    }
    summaries
}

fn assertion_edge(
    statement: &VizStatement,
    projection: &VizProjection,
    terms: &BTreeMap<VizTermId, &VizTerm>,
    statements: &BTreeMap<VizStatementId, &VizStatement>,
    summaries: &BTreeMap<VizStatementId, StatementRelationSummary>,
) -> VizSceneEdge {
    let edge_id = scene_assertion_edge_id(&statement.id);
    let assertions = projection
        .assertions
        .iter()
        .filter(|assertion| assertion.statement == statement.id)
        .collect::<Vec<_>>();
    let mut bindings = vec![VizSemanticRef::Statement(statement.id.clone())];
    bindings.extend(
        assertions
            .iter()
            .map(|assertion| VizSemanticRef::Assertion(assertion.id.clone())),
    );
    let graph_ids = assertions
        .iter()
        .map(|assertion| assertion.graph.clone())
        .collect::<BTreeSet<_>>();
    let anchor = statement_needs_anchor(statement, summaries)
        .then(|| statement_anchor(statement, projection, summaries.get(&statement.id)));
    let predicate = terms
        .get(&statement.predicate)
        .map_or(statement.predicate.0.as_str(), |term| term.label.as_str());
    VizSceneEdge {
        id: edge_id,
        bindings,
        kind: VizSceneEdgeKind::Assertion,
        source: compact_value_endpoint(&statement.subject, statements, true),
        target: compact_value_endpoint(&statement.object, statements, false),
        label: plain_label(predicate),
        badges: std::iter::once(VizBadge {
            kind: VizBadgeKind::Asserted,
            label: "asserted".to_owned(),
            binding: None,
        })
        .chain(
            graph_ids
                .into_iter()
                .map(|graph| graph_badge(graph, projection)),
        )
        .collect(),
        anchor,
        accessibility: VizAccessibility {
            title: format!("asserted {predicate}"),
            description: format!(
                "{}. Asserted in {}.",
                statement_display(statement, terms),
                statement
                    .asserted_in
                    .iter()
                    .map(|graph| graph.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
    }
}

fn statement_anchor(
    statement: &VizStatement,
    projection: &VizProjection,
    summary: Option<&StatementRelationSummary>,
) -> VizEdgeAnchor {
    let summary = summary.cloned().unwrap_or_default();
    let references = projection
        .references
        .iter()
        .filter(|reference| reference.statement == statement.id)
        .collect::<Vec<_>>();
    let mut bindings = vec![VizSemanticRef::Statement(statement.id.clone())];
    bindings.extend(
        references
            .iter()
            .map(|reference| VizSemanticRef::Reference(reference.id.clone())),
    );
    let mut badges = vec![VizBadge {
        kind: VizBadgeKind::Asserted,
        label: "asserted".to_owned(),
        binding: None,
    }];
    if !summary.reifiers.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::ReifierCount,
            label: format!("{} reifier(s)", summary.reifiers.len()),
            binding: None,
        });
    }
    if !summary.annotation_relations.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::AnnotationCount,
            label: format!("{} annotation(s)", summary.annotation_relations.len()),
            binding: None,
        });
    }
    if !references.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::ReferenceCount,
            label: format!("{} reference(s)", references.len()),
            binding: None,
        });
    }
    VizEdgeAnchor {
        id: scene_statement_anchor_id(&statement.id),
        bindings,
        label: plain_label(&format!("S:{}", short_suffix(&statement.id.0))),
        badges,
        accessibility: VizAccessibility {
            title: "addressable asserted statement".to_owned(),
            description: format!(
                "Asserted statement with {} reifier(s), {} annotation(s), and {} incoming reference(s).",
                summary.reifiers.len(),
                summary.annotation_relations.len(),
                references.len()
            ),
        },
    }
}

fn statement_node(
    statement: &VizStatement,
    projection: &VizProjection,
    terms: &BTreeMap<VizTermId, &VizTerm>,
    summaries: &BTreeMap<VizStatementId, StatementRelationSummary>,
    compact: bool,
) -> VizSceneNode {
    let summary = summaries.get(&statement.id).cloned().unwrap_or_default();
    let mut badges = statement_badges(statement, projection, &summary);
    if statement.asserted_in.is_empty() {
        badges.insert(
            0,
            VizBadge {
                kind: VizBadgeKind::Quoted,
                label: "quoted only".to_owned(),
                binding: Some(VizSemanticRef::Statement(statement.id.clone())),
            },
        );
    }
    let display = statement_display(statement, terms);
    let display_label = if compact {
        format!("⟪ {display} ⟫")
    } else {
        format!("S:{}", short_suffix(&statement.id.0))
    };
    VizSceneNode {
        id: scene_statement_node_id(&statement.id),
        bindings: vec![VizSemanticRef::Statement(statement.id.clone())],
        kind: VizSceneNodeKind::Statement,
        label: plain_label(&display_label),
        ports: vec![
            port("subject", VizPortKind::Subject),
            port("predicate", VizPortKind::Predicate),
            port("object", VizPortKind::Object),
            port("relation", VizPortKind::Relation),
        ],
        badges,
        accessibility: VizAccessibility {
            title: if statement.asserted_in.is_empty() {
                "quoted structural statement".to_owned()
            } else {
                "structural statement".to_owned()
            },
            description: statement_accessible_description(statement, terms, &summary),
        },
    }
}

fn term_node(term: &VizTerm) -> VizSceneNode {
    let (kind, full_text, language, direction, title) = match &term.value {
        VizTermValue::Iri { value } => (
            VizSceneNodeKind::Iri,
            value.clone(),
            None,
            None,
            "IRI resource",
        ),
        VizTermValue::Blank { label, scope } => (
            VizSceneNodeKind::Blank,
            format!("_:{label} in blank-node scope {scope}"),
            None,
            None,
            "blank node",
        ),
        VizTermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => (
            VizSceneNodeKind::Literal,
            format!("{lexical_form} datatype {datatype}"),
            language.clone(),
            *direction,
            "literal",
        ),
    };
    let mut badges = term
        .roles
        .iter()
        .filter_map(|role| role_badge(role, &term.id))
        .collect::<Vec<_>>();
    if let Some(direction) = direction {
        badges.push(VizBadge {
            kind: VizBadgeKind::Direction,
            label: match direction {
                VizTextDirection::Ltr => "left-to-right",
                VizTextDirection::Rtl => "right-to-left",
            }
            .to_owned(),
            binding: Some(VizSemanticRef::Term(term.id.clone())),
        });
    }
    VizSceneNode {
        id: scene_term_node_id(&term.id),
        bindings: vec![VizSemanticRef::Term(term.id.clone())],
        kind,
        label: VizSceneLabel {
            text: term.label.clone(),
            full_text,
            language,
            direction,
        },
        ports: vec![port("in", VizPortKind::In), port("out", VizPortKind::Out)],
        badges,
        accessibility: VizAccessibility {
            title: title.to_owned(),
            description: format!("{title}: {}.", term.label),
        },
    }
}

fn add_relation_edges(
    edges: &mut Vec<VizSceneEdge>,
    projection: &VizProjection,
    terms: &BTreeMap<VizTermId, &VizTerm>,
    statements: &BTreeMap<VizStatementId, &VizStatement>,
    mode: VizMode,
) {
    for relation in &projection.relations {
        match relation {
            VizRelation::Reifies {
                id,
                reifier,
                statement,
                graph,
            } => {
                let target = match mode {
                    VizMode::Compact => compact_statement_endpoint(statement, statements),
                    VizMode::Incidence => {
                        node_endpoint(&scene_statement_node_id(statement), "relation")
                    }
                    VizMode::Table => continue,
                };
                edges.push(VizSceneEdge {
                    id: format!("edge-reifies-{}", id_suffix(&id.0)),
                    bindings: vec![VizSemanticRef::Relation(id.clone())],
                    kind: VizSceneEdgeKind::Reifies,
                    source: node_endpoint(&scene_term_node_id(reifier), "out"),
                    target,
                    label: plain_label("reifies"),
                    badges: vec![graph_badge(graph.clone(), projection)],
                    anchor: None,
                    accessibility: VizAccessibility {
                        title: "reifies".to_owned(),
                        description: format!(
                            "{} reifies statement {} in graph {}.",
                            terms
                                .get(reifier)
                                .map_or(reifier.0.as_str(), |term| term.label.as_str()),
                            id_suffix(&statement.0),
                            graph.0
                        ),
                    },
                });
            }
            VizRelation::Annotation {
                id,
                reifier,
                predicate,
                object,
                graph,
            } => {
                let predicate_label = terms
                    .get(predicate)
                    .map_or(predicate.0.as_str(), |term| term.label.as_str());
                let target = match mode {
                    VizMode::Compact => compact_value_endpoint(object, statements, false),
                    VizMode::Incidence => incidence_value_endpoint(object),
                    VizMode::Table => continue,
                };
                edges.push(VizSceneEdge {
                    id: format!("edge-annotation-{}", id_suffix(&id.0)),
                    bindings: vec![VizSemanticRef::Relation(id.clone())],
                    kind: VizSceneEdgeKind::Annotation,
                    source: node_endpoint(&scene_term_node_id(reifier), "out"),
                    target,
                    label: plain_label(predicate_label),
                    badges: vec![graph_badge(graph.clone(), projection)],
                    anchor: None,
                    accessibility: VizAccessibility {
                        title: format!("annotation {predicate_label}"),
                        description: format!(
                            "{} has annotation {predicate_label} {} in graph {}.",
                            terms
                                .get(reifier)
                                .map_or(reifier.0.as_str(), |term| term.label.as_str()),
                            value_display(object, terms),
                            graph.0
                        ),
                    },
                });
            }
        }
    }
}

fn statement_badges(
    statement: &VizStatement,
    projection: &VizProjection,
    summary: &StatementRelationSummary,
) -> Vec<VizBadge> {
    let mut badges = Vec::new();
    if !statement.asserted_in.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::Asserted,
            label: "asserted".to_owned(),
            binding: Some(VizSemanticRef::Statement(statement.id.clone())),
        });
        badges.extend(
            statement
                .asserted_in
                .iter()
                .cloned()
                .map(|graph| graph_badge(graph, projection)),
        );
    }
    if !summary.reifiers.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::ReifierCount,
            label: format!("{} reifier(s)", summary.reifiers.len()),
            binding: None,
        });
    }
    if !summary.annotation_relations.is_empty() {
        badges.push(VizBadge {
            kind: VizBadgeKind::AnnotationCount,
            label: format!("{} annotation(s)", summary.annotation_relations.len()),
            binding: None,
        });
    }
    if statement.incoming_references > 0 {
        badges.push(VizBadge {
            kind: VizBadgeKind::ReferenceCount,
            label: format!("{} reference(s)", statement.incoming_references),
            binding: None,
        });
    }
    if statement.nesting_depth > 0 {
        badges.push(VizBadge {
            kind: VizBadgeKind::NestingDepth,
            label: format!("depth {}", statement.nesting_depth),
            binding: None,
        });
    }
    if statement.dialect != VizDialect::Rdf12 {
        badges.push(VizBadge {
            kind: VizBadgeKind::Dialect,
            label: dialect_label(&statement.dialect).to_owned(),
            binding: projection
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.target.as_deref() == Some(&statement.id.0))
                .map(|diagnostic| VizSemanticRef::Diagnostic(diagnostic.id.clone())),
        });
    }
    if statement.roles.contains(&VizRole::Focus) {
        badges.push(VizBadge {
            kind: VizBadgeKind::Focus,
            label: "focus".to_owned(),
            binding: Some(VizSemanticRef::Statement(statement.id.clone())),
        });
    }
    badges
}

fn role_badge(role: &VizRole, term: &VizTermId) -> Option<VizBadge> {
    let (kind, label) = match role {
        VizRole::Focus => (VizBadgeKind::Focus, "focus".to_owned()),
        VizRole::Reifier => (VizBadgeKind::Reifier, "reifier".to_owned()),
        VizRole::Custom(label) => (VizBadgeKind::Role, label.clone()),
        VizRole::GraphName
        | VizRole::Predicate
        | VizRole::QuotedStatement
        | VizRole::AssertedStatement
        | VizRole::AnnotatedStatement => return None,
    };
    Some(VizBadge {
        kind,
        label,
        binding: Some(VizSemanticRef::Term(term.clone())),
    })
}

fn compact_visible_terms(projection: &VizProjection) -> BTreeSet<VizTermId> {
    let mut visible = BTreeSet::new();
    for statement in &projection.statements {
        add_value_term(&statement.subject, &mut visible);
        add_value_term(&statement.object, &mut visible);
    }
    for relation in &projection.relations {
        match relation {
            VizRelation::Reifies { reifier, .. } => {
                visible.insert(reifier.clone());
            }
            VizRelation::Annotation {
                reifier, object, ..
            } => {
                visible.insert(reifier.clone());
                add_value_term(object, &mut visible);
            }
        }
    }
    visible
}

fn add_value_term(value: &VizValueRef, terms: &mut BTreeSet<VizTermId>) {
    if let VizValueRef::Term { id } = value {
        terms.insert(id.clone());
    }
}

fn compact_value_endpoint(
    value: &VizValueRef,
    statements: &BTreeMap<VizStatementId, &VizStatement>,
    source: bool,
) -> VizEndpoint {
    match value {
        VizValueRef::Term { id } => {
            node_endpoint(&scene_term_node_id(id), if source { "out" } else { "in" })
        }
        VizValueRef::Statement { id } => compact_statement_endpoint(id, statements),
    }
}

fn compact_statement_endpoint(
    id: &VizStatementId,
    statements: &BTreeMap<VizStatementId, &VizStatement>,
) -> VizEndpoint {
    if statements
        .get(id)
        .is_some_and(|statement| !statement.asserted_in.is_empty())
    {
        VizEndpoint::EdgeAnchor {
            edge: scene_assertion_edge_id(id),
            anchor: scene_statement_anchor_id(id),
        }
    } else {
        node_endpoint(&scene_statement_node_id(id), "relation")
    }
}

fn incidence_value_endpoint(value: &VizValueRef) -> VizEndpoint {
    match value {
        VizValueRef::Term { id } => node_endpoint(&scene_term_node_id(id), "out"),
        VizValueRef::Statement { id } => node_endpoint(&scene_statement_node_id(id), "relation"),
    }
}

fn statement_needs_anchor(
    statement: &VizStatement,
    summaries: &BTreeMap<VizStatementId, StatementRelationSummary>,
) -> bool {
    statement.incoming_references > 0
        || statement.roles.contains(&VizRole::Focus)
        || summaries.get(&statement.id).is_some_and(|summary| {
            !summary.reifiers.is_empty() || !summary.annotation_relations.is_empty()
        })
}

fn statement_reference_bindings(
    projection: &VizProjection,
    statement: &VizStatementId,
) -> Vec<VizSemanticRef> {
    std::iter::once(VizSemanticRef::Statement(statement.clone()))
        .chain(
            projection
                .references
                .iter()
                .filter(|reference| reference.statement == *statement)
                .map(|reference| VizSemanticRef::Reference(reference.id.clone())),
        )
        .collect()
}

fn statement_accessible_description(
    statement: &VizStatement,
    terms: &BTreeMap<VizTermId, &VizTerm>,
    summary: &StatementRelationSummary,
) -> String {
    let status = if statement.asserted_in.is_empty() {
        "Not asserted"
    } else {
        "Asserted"
    };
    format!(
        "{}. {status}. {} reifier(s), {} annotation(s), {} incoming reference(s). Dialect: {}.",
        statement_display(statement, terms),
        summary.reifiers.len(),
        summary.annotation_relations.len(),
        statement.incoming_references,
        dialect_label(&statement.dialect)
    )
}

fn statement_display(statement: &VizStatement, terms: &BTreeMap<VizTermId, &VizTerm>) -> String {
    format!(
        "{} —{}→ {}",
        value_display(&statement.subject, terms),
        terms
            .get(&statement.predicate)
            .map_or(statement.predicate.0.as_str(), |term| term.label.as_str()),
        value_display(&statement.object, terms)
    )
}

fn value_display(value: &VizValueRef, terms: &BTreeMap<VizTermId, &VizTerm>) -> String {
    match value {
        VizValueRef::Term { id } => terms
            .get(id)
            .map_or_else(|| id.0.clone(), |term| term.label.clone()),
        VizValueRef::Statement { id } => format!("S:{}", short_suffix(&id.0)),
    }
}

fn graph_badge(graph: VizGraphId, projection: &VizProjection) -> VizBadge {
    let graph_label = projection
        .graphs
        .iter()
        .find(|candidate| candidate.id == graph)
        .map_or_else(
            || {
                if graph.0 == DEFAULT_GRAPH_ID {
                    "default graph".to_owned()
                } else {
                    format!("graph {}", short_suffix(&graph.0))
                }
            },
            |candidate| candidate.label.clone(),
        );
    VizBadge {
        kind: VizBadgeKind::Graph,
        label: graph_label,
        binding: Some(VizSemanticRef::Graph(graph)),
    }
}

fn dialect_label(dialect: &VizDialect) -> &'static str {
    match dialect {
        VizDialect::Rdf12 => "RDF 1.2",
        VizDialect::SymmetricRdf12 => "symmetric RDF 1.2",
        VizDialect::GeneralizedRdf => "generalized RDF",
    }
}

fn plain_label(value: &str) -> VizSceneLabel {
    VizSceneLabel {
        text: value.to_owned(),
        full_text: value.to_owned(),
        language: None,
        direction: None,
    }
}

fn port(id: &str, kind: VizPortKind) -> VizPort {
    VizPort {
        id: id.to_owned(),
        kind,
    }
}

fn node_endpoint(node: &str, port: &str) -> VizEndpoint {
    VizEndpoint::NodePort {
        node: node.to_owned(),
        port: port.to_owned(),
    }
}

fn term_map(projection: &VizProjection) -> BTreeMap<VizTermId, &VizTerm> {
    projection
        .terms
        .iter()
        .map(|term| (term.id.clone(), term))
        .collect()
}

fn statement_map(projection: &VizProjection) -> BTreeMap<VizStatementId, &VizStatement> {
    projection
        .statements
        .iter()
        .map(|statement| (statement.id.clone(), statement))
        .collect()
}

fn graph_map(projection: &VizProjection) -> BTreeMap<VizGraphId, &VizGraph> {
    projection
        .graphs
        .iter()
        .map(|graph| (graph.id.clone(), graph))
        .collect()
}

fn scene_term_node_id(id: &VizTermId) -> String {
    format!("node-term-{}", id_suffix(&id.0))
}

fn scene_statement_node_id(id: &VizStatementId) -> String {
    format!("node-statement-{}", id_suffix(&id.0))
}

fn scene_assertion_edge_id(id: &VizStatementId) -> String {
    format!("edge-assertion-{}", id_suffix(&id.0))
}

fn scene_statement_anchor_id(id: &VizStatementId) -> String {
    format!("anchor-statement-{}", id_suffix(&id.0))
}

fn id_suffix(id: &str) -> &str {
    id.rsplit(':').next().unwrap_or(id)
}

fn short_suffix(id: &str) -> String {
    id_suffix(id).chars().take(8).collect()
}

fn compact_legend() -> Vec<VizLegendEntry> {
    vec![
        legend("assertion", "solid arrow", "asserted RDF statement"),
        legend(
            "quoted",
            "hollow statement capsule",
            "triple term that is not asserted",
        ),
        legend(
            "anchor",
            "statement anchor",
            "addressable asserted statement",
        ),
        legend("reifies", "reifies arrow", "reifier-to-statement relation"),
        legend(
            "annotation",
            "annotation arrow",
            "ordinary relation from a reifier",
        ),
    ]
}

fn incidence_legend() -> Vec<VizLegendEntry> {
    vec![
        legend(
            "statement",
            "three-port statement node",
            "exact structural RDF statement",
        ),
        legend("subject", "subject port", "statement subject role"),
        legend("predicate", "predicate port", "statement predicate role"),
        legend("object", "object port", "statement object role"),
        legend("reifies", "reifies arrow", "reifier-to-statement relation"),
    ]
}

fn table_legend() -> Vec<VizLegendEntry> {
    vec![legend(
        "table",
        "statement matrix",
        "one row per structural statement",
    )]
}

fn legend(id: &str, symbol: &str, label: &str) -> VizLegendEntry {
    VizLegendEntry {
        id: format!("legend-{id}"),
        symbol: symbol.to_owned(),
        label: label.to_owned(),
    }
}

fn add_legend_group(scene: &mut VizScene) {
    scene.groups.push(VizSceneGroup {
        id: "group-legend".to_owned(),
        kind: VizSceneGroupKind::Legend,
        label: "Visual grammar".to_owned(),
        members: scene.legend.iter().map(|entry| entry.id.clone()).collect(),
    });
}

fn validate_scene(scene: &VizScene) -> Result<(), VizError> {
    let nodes = scene
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let edges = scene
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    if nodes.len() != scene.nodes.len() || edges.len() != scene.edges.len() {
        return Err(VizError::Scene(
            "visualization scene contains duplicate node or edge ids".to_owned(),
        ));
    }
    for edge in &scene.edges {
        validate_endpoint(&edge.source, &nodes, &edges)?;
        validate_endpoint(&edge.target, &nodes, &edges)?;
        if edge.bindings.is_empty() {
            return Err(VizError::Scene(format!(
                "visualization scene edge {} has no semantic binding",
                edge.id
            )));
        }
    }
    for node in &scene.nodes {
        if node.bindings.is_empty() {
            return Err(VizError::Scene(format!(
                "visualization scene node {} has no semantic binding",
                node.id
            )));
        }
    }
    Ok(())
}

fn validate_endpoint(
    endpoint: &VizEndpoint,
    nodes: &BTreeMap<&str, &VizSceneNode>,
    edges: &BTreeMap<&str, &VizSceneEdge>,
) -> Result<(), VizError> {
    match endpoint {
        VizEndpoint::NodePort { node, port } => {
            let Some(target) = nodes.get(node.as_str()) else {
                return Err(VizError::Scene(format!(
                    "visualization endpoint references missing node {node}"
                )));
            };
            if !target.ports.iter().any(|candidate| candidate.id == *port) {
                return Err(VizError::Scene(format!(
                    "visualization endpoint references missing port {node}.{port}"
                )));
            }
        }
        VizEndpoint::EdgeAnchor { edge, anchor } => {
            let Some(target) = edges.get(edge.as_str()) else {
                return Err(VizError::Scene(format!(
                    "visualization endpoint references missing edge {edge}"
                )));
            };
            if target.anchor.as_ref().map(|value| &value.id) != Some(anchor) {
                return Err(VizError::Scene(format!(
                    "visualization endpoint references missing anchor {edge}.{anchor}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EX: &str = "https://example.org/";
    const KNOWS: &str = "https://example.org/knows";

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{local}"))
    }

    fn semantic_input(asserted: bool) -> VizGraphInput {
        let statement = VizInputStatement {
            subject: iri("alice"),
            predicate: KNOWS.to_owned(),
            object: iri("bob"),
        };
        VizGraphInput {
            quads: asserted
                .then(|| VizInputQuad {
                    subject: iri("alice"),
                    predicate: KNOWS.to_owned(),
                    object: iri("bob"),
                    graph_name: Some(iri("facts")),
                })
                .into_iter()
                .collect(),
            reifiers: vec![VizInputReifier {
                reifier: iri("claim"),
                statement,
                graph_name: Some(iri("claims")),
            }],
            annotations: vec![VizInputAnnotation {
                reifier: iri("claim"),
                predicate: format!("{EX}confidence"),
                object: TermValue::Literal {
                    lexical_form: "0.8".to_owned(),
                    datatype: "http://www.w3.org/2001/XMLSchema#decimal".to_owned(),
                    language: None,
                    direction: None,
                },
                graph_name: Some(iri("provenance")),
            }],
        }
    }

    #[test]
    fn compact_reifier_targets_assertion_edge_anchor() {
        let (projection, scene) =
            project_graph_input_scene(&semantic_input(true), &VizSpec::default()).expect("scene");
        assert_eq!(scene.mode, VizMode::Compact);
        let assertion = scene
            .edges
            .iter()
            .find(|edge| edge.kind == VizSceneEdgeKind::Assertion)
            .expect("assertion edge");
        assert!(assertion.anchor.is_some());
        let reifies = scene
            .edges
            .iter()
            .find(|edge| edge.kind == VizSceneEdgeKind::Reifies)
            .expect("reifies edge");
        assert!(matches!(reifies.target, VizEndpoint::EdgeAnchor { .. }));
        assert_eq!(
            scene
                .nodes
                .iter()
                .filter(|node| matches!(node.kind, VizSceneNodeKind::Iri))
                .filter(|node| node.label.text == "alice")
                .count(),
            1
        );
        assert_eq!(projection.statements.len(), 1);
    }

    #[test]
    fn compact_quoted_only_statement_is_connected_capsule() {
        let (_, scene) =
            project_graph_input_scene(&semantic_input(false), &VizSpec::default()).expect("scene");
        assert!(
            !scene
                .edges
                .iter()
                .any(|edge| edge.kind == VizSceneEdgeKind::Assertion)
        );
        let statement = scene
            .nodes
            .iter()
            .find(|node| node.kind == VizSceneNodeKind::Statement)
            .expect("statement capsule");
        assert!(
            statement
                .badges
                .iter()
                .any(|badge| badge.kind == VizBadgeKind::Quoted)
        );
        assert!(
            scene
                .edges
                .iter()
                .any(|edge| edge.kind == VizSceneEdgeKind::QuoteSubject)
        );
        assert!(
            scene
                .edges
                .iter()
                .any(|edge| edge.kind == VizSceneEdgeKind::QuoteObject)
        );
    }

    #[test]
    fn incidence_scene_has_three_explicit_statement_ports() {
        let spec = VizSpec {
            mode: VizMode::Incidence,
            ..VizSpec::default()
        };
        let (_, scene) = project_graph_input_scene(&semantic_input(true), &spec).expect("scene");
        let kinds = scene
            .edges
            .iter()
            .map(|edge| edge.kind)
            .collect::<BTreeSet<_>>();
        assert!(kinds.contains(&VizSceneEdgeKind::Subject));
        assert!(kinds.contains(&VizSceneEdgeKind::Predicate));
        assert!(kinds.contains(&VizSceneEdgeKind::Object));
    }

    #[test]
    fn table_scene_uses_selected_columns() {
        let spec = VizSpec {
            mode: VizMode::Table,
            table_fields: vec![VizTableField::Statement, VizTableField::Reifiers],
            ..VizSpec::default()
        };
        let (_, scene) = project_graph_input_scene(&semantic_input(true), &spec).expect("scene");
        let table = scene.table.expect("table");
        assert_eq!(table.fields, spec.table_fields);
        assert!(table.rows.iter().all(|row| row.cells.len() == 2));
        assert!(scene.nodes.is_empty());
        assert!(scene.edges.is_empty());
    }

    #[test]
    fn scene_is_deterministic_under_input_permutation() {
        let mut first = semantic_input(true);
        first.quads.push(VizInputQuad {
            subject: iri("bob"),
            predicate: KNOWS.to_owned(),
            object: iri("carol"),
            graph_name: Some(iri("facts")),
        });
        let mut second = first.clone();
        second.quads.reverse();
        second.reifiers.reverse();
        second.annotations.reverse();
        let (_, a) = project_graph_input_scene(&first, &VizSpec::default()).expect("scene a");
        let (_, b) = project_graph_input_scene(&second, &VizSpec::default()).expect("scene b");
        assert_eq!(a, b);
    }

    #[test]
    fn directional_literal_scene_label_preserves_rdf_direction() {
        let input = VizGraphInput {
            quads: vec![VizInputQuad {
                subject: iri("notice"),
                predicate: format!("{EX}message"),
                object: TermValue::Literal {
                    lexical_form: "مرحبا".to_owned(),
                    datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".to_owned(),
                    language: Some("ar".to_owned()),
                    direction: Some(RdfTextDirection::Rtl),
                },
                graph_name: None,
            }],
            ..VizGraphInput::default()
        };
        let (_, scene) = project_graph_input_scene(&input, &VizSpec::default()).expect("scene");
        let literal = scene
            .nodes
            .iter()
            .find(|node| node.kind == VizSceneNodeKind::Literal)
            .expect("literal node");
        assert_eq!(literal.label.language.as_deref(), Some("ar"));
        assert_eq!(literal.label.direction, Some(VizTextDirection::Rtl));
        assert!(
            literal
                .badges
                .iter()
                .any(|badge| badge.kind == VizBadgeKind::Direction)
        );
    }
}

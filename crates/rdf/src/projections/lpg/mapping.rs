// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::loss::{
    LOSS_LPG_ANNOTATION_SIDEBAND, LOSS_LPG_BLANK_SCOPE_SIDEBAND, LOSS_LPG_EDGE_ID_DROPPED,
    LOSS_LPG_EDGE_SEMANTICS_LOWERED, LOSS_LPG_EDGE_TYPE_INTERPRETED, LOSS_LPG_LABEL_INTERPRETED,
    LOSS_LPG_LITERAL_SEMANTICS_LOWERED, LOSS_LPG_NAMED_GRAPH_SIDEBAND,
    LOSS_LPG_NODE_ID_INTERPRETED, LOSS_LPG_PROPERTY_KEY_INTERPRETED, LOSS_LPG_REIFIER_SIDEBAND,
    LOSS_LPG_TRIPLE_TERM_SIDEBAND, LOSS_LPG_TYPE_SEMANTICS_LOWERED, LOSS_LPG_VALUE_INTERPRETED,
};
use purrdf_core::{
    BlankScope, DatasetView, LossEntry, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfLocation, TermId, check_ledger_sound, lpg_to_rdf_loss_ledger, rdf_to_lpg_loss_ledger,
};

use super::super::{ProjectionError, ProjectionLimits, ProjectionTerm};

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
use super::model::{
    LpgAnnotation, LpgConfig, LpgEdge, LpgGraph, LpgGraphContext, LpgLabel, LpgNode, LpgProperty,
    LpgRdfQuad, LpgReifier, annotation_identifier, collect_node_terms, edge_identifier,
    node_identifier, property_atom, reifier_identifier, statement_identifier,
};

/// Result of RDF→LPG projection.
#[derive(Debug, Clone)]
pub struct LpgProjection {
    /// Canonical property graph with complete RDF 1.2 reversal sideband.
    pub graph: LpgGraph,
    /// Always-computed, located semantic-lowering ledger.
    pub loss_ledger: LossLedger,
}

/// Result of canonical LPG→RDF lifting.
#[derive(Debug, Clone)]
pub struct LpgLiftOutcome {
    /// Reconstructed validated RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Always-computed, located native-LPG interpretation ledger.
    pub loss_ledger: LossLedger,
}

/// Project any static RDF dataset view into the canonical LPG model.
///
/// Output order and identifiers depend only on RDF values, never backend-local ids or
/// iteration order. Every RDF-origin row retains exact term and graph identity, while
/// its native label/property/edge semantics are recorded in the closed loss profile.
///
/// # Errors
///
/// Returns a typed failure for malformed view data, inconsistent RDF positions,
/// resource-limit breaches, identifier collisions, or an unsound loss ledger.
pub fn project_lpg<D: DatasetView>(
    view: &D,
    config: &LpgConfig,
) -> Result<LpgProjection, ProjectionError> {
    let mut budget = RecordBudget::new(config.max_records());
    let mut cache = BTreeMap::new();
    let mut named_graphs = BTreeSet::new();

    for graph in view.named_graphs() {
        let term = resolve_term(view, graph, config.limits(), &mut cache)?;
        if !named_graphs.insert(term) {
            return Err(ProjectionError::integrity(
                "dataset view exposed a duplicate named graph declaration",
            ));
        }
        budget.consume("named graph declaration")?;
    }

    let mut quads = Vec::new();
    for quad in view.quads() {
        budget.consume("RDF statement")?;
        let subject = resolve_term(view, quad.s, config.limits(), &mut cache)?;
        let predicate = resolve_term(view, quad.p, config.limits(), &mut cache)?;
        let ProjectionTerm::Iri { value: predicate } = predicate else {
            return Err(ProjectionError::integrity(
                "RDF view exposed a non-IRI predicate",
            ));
        };
        let object = resolve_term(view, quad.o, config.limits(), &mut cache)?;
        let graph = context_from_id(
            view,
            quad.g,
            config.limits(),
            &mut cache,
            &mut named_graphs,
            &mut budget,
        )?;
        quads.push(LpgRdfQuad {
            subject,
            predicate,
            object,
            graph,
        });
    }
    quads.sort();
    reject_duplicates(&quads, "RDF statements")?;

    let mut reifiers = Vec::new();
    for quad in view.reifier_quads() {
        budget.consume("RDF reifier")?;
        let predicate = resolve_term(view, quad.p, config.limits(), &mut cache)?;
        if predicate
            != (ProjectionTerm::Iri {
                value: RDF_REIFIES.to_owned(),
            })
        {
            return Err(ProjectionError::integrity(
                "RDF view exposed a reifier row without the rdf:reifies predicate",
            ));
        }
        let reifier = resolve_term(view, quad.s, config.limits(), &mut cache)?;
        let statement = resolve_term(view, quad.o, config.limits(), &mut cache)?;
        let graph = context_from_id(
            view,
            quad.g,
            config.limits(),
            &mut cache,
            &mut named_graphs,
            &mut budget,
        )?;
        let mut row = LpgReifier {
            id: String::new(),
            reifier,
            statement,
            graph,
        };
        row.id = reifier_identifier(&row, config.limits())?;
        reifiers.push(row);
    }
    reifiers.sort_by(|left, right| left.id.cmp(&right.id));
    reject_duplicate_ids(&reifiers, |row| &row.id, "RDF reifiers")?;

    let mut annotations = Vec::new();
    for quad in view.annotation_quads() {
        budget.consume("RDF annotation")?;
        let reifier = resolve_term(view, quad.s, config.limits(), &mut cache)?;
        let predicate = resolve_term(view, quad.p, config.limits(), &mut cache)?;
        let ProjectionTerm::Iri { value: predicate } = predicate else {
            return Err(ProjectionError::integrity(
                "RDF view exposed a non-IRI annotation predicate",
            ));
        };
        let object = resolve_term(view, quad.o, config.limits(), &mut cache)?;
        let graph = context_from_id(
            view,
            quad.g,
            config.limits(),
            &mut cache,
            &mut named_graphs,
            &mut budget,
        )?;
        let mut row = LpgAnnotation {
            id: String::new(),
            reifier,
            predicate,
            object,
            graph,
        };
        row.id = annotation_identifier(&row, config.limits())?;
        annotations.push(row);
    }
    annotations.sort_by(|left, right| left.id.cmp(&right.id));
    reject_duplicate_ids(&annotations, |row| &row.id, "RDF annotations")?;

    let mut node_terms = BTreeSet::new();
    for graph in &named_graphs {
        collect_node_terms(graph, &mut node_terms);
    }
    for quad in &quads {
        collect_node_terms(&quad.subject, &mut node_terms);
        collect_node_terms(&quad.object, &mut node_terms);
    }
    for row in &reifiers {
        collect_node_terms(&row.reifier, &mut node_terms);
        collect_node_terms(&row.statement, &mut node_terms);
    }
    for row in &annotations {
        collect_node_terms(&row.reifier, &mut node_terms);
        collect_node_terms(&row.object, &mut node_terms);
    }

    let mut term_to_node = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    for term in node_terms {
        budget.consume("LPG node")?;
        let id = node_identifier(&term, config.limits())?;
        if let Some(existing) = nodes.get(&id) {
            let existing: &LpgNode = existing;
            if existing.identity != term {
                return Err(ProjectionError::integrity(
                    "SHA-256 collision between distinct LPG node identities",
                ));
            }
            return Err(ProjectionError::integrity("duplicate LPG node identity"));
        }
        term_to_node.insert(term.clone(), id.clone());
        nodes.insert(
            id.clone(),
            LpgNode {
                id,
                identity: term,
                labels: Vec::new(),
                properties: Vec::new(),
            },
        );
    }

    let mut edges = BTreeMap::new();
    for quad in &quads {
        let source = term_to_node.get(&quad.subject).ok_or_else(|| {
            ProjectionError::integrity("RDF statement subject has no canonical LPG node")
        })?;
        if quad.predicate == config.rdf_type()
            && let ProjectionTerm::Iri { value } = &quad.object
        {
            let statement_id = statement_identifier(quad, config.limits())?;
            nodes
                .get_mut(source)
                .expect("source id was read from nodes")
                .labels
                .push(LpgLabel {
                    statement_id,
                    value: value.clone(),
                    rdf: quad.clone(),
                });
        } else if matches!(quad.object, ProjectionTerm::Literal { .. }) {
            let statement_id = statement_identifier(quad, config.limits())?;
            nodes
                .get_mut(source)
                .expect("source id was read from nodes")
                .properties
                .push(LpgProperty {
                    statement_id,
                    key: quad.predicate.clone(),
                    value: property_atom(&quad.object)?,
                    rdf: quad.clone(),
                });
        } else {
            let target = term_to_node.get(&quad.object).ok_or_else(|| {
                ProjectionError::integrity("RDF statement object has no canonical LPG node")
            })?;
            let id = edge_identifier(quad, config.limits())?;
            let edge = LpgEdge {
                id: id.clone(),
                source: source.clone(),
                target: target.clone(),
                edge_type: quad.predicate.clone(),
                rdf: quad.clone(),
            };
            if edges.insert(id, edge).is_some() {
                return Err(ProjectionError::integrity(
                    "duplicate or colliding LPG edge identifier",
                ));
            }
        }
    }

    for node in nodes.values_mut() {
        node.labels.sort();
        node.properties.sort();
    }
    let graph = LpgGraph::new(
        nodes.into_values().collect(),
        edges.into_values().collect(),
        reifiers,
        annotations,
        named_graphs.into_iter().collect(),
    );
    graph.validate(config)?;

    let mut ledger = LossLedger::new();
    let contract = rdf_to_lpg_loss_ledger();
    for graph_name in &graph.named_graphs {
        let id = node_identifier(graph_name, config.limits())?;
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_NAMED_GRAPH_SIDEBAND,
            "lpg:named-graph",
            &id,
        );
        if contains_blank(graph_name) {
            record_loss(
                &mut ledger,
                &contract,
                LOSS_LPG_BLANK_SCOPE_SIDEBAND,
                "lpg:named-graph",
                &id,
            );
        }
    }
    for quad in &quads {
        let id = statement_identifier(quad, config.limits())?;
        let primary = if quad.predicate == config.rdf_type()
            && matches!(quad.object, ProjectionTerm::Iri { .. })
        {
            LOSS_LPG_TYPE_SEMANTICS_LOWERED
        } else if matches!(quad.object, ProjectionTerm::Literal { .. }) {
            LOSS_LPG_LITERAL_SEMANTICS_LOWERED
        } else {
            LOSS_LPG_EDGE_SEMANTICS_LOWERED
        };
        record_loss(&mut ledger, &contract, primary, "lpg:statement", &id);
        record_term_sideband_losses(
            &mut ledger,
            &contract,
            [&quad.subject, &quad.object],
            &quad.graph,
            "lpg:statement",
            &id,
        );
    }
    for row in &graph.reifiers {
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_REIFIER_SIDEBAND,
            "lpg:reifier",
            &row.id,
        );
        record_term_sideband_losses(
            &mut ledger,
            &contract,
            [&row.reifier, &row.statement],
            &row.graph,
            "lpg:reifier",
            &row.id,
        );
    }
    for row in &graph.annotations {
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_ANNOTATION_SIDEBAND,
            "lpg:annotation",
            &row.id,
        );
        record_term_sideband_losses(
            &mut ledger,
            &contract,
            [&row.reifier, &row.object],
            &row.graph,
            "lpg:annotation",
            &row.id,
        );
    }
    ensure_sound(&ledger, "rdf-1.2-dataset", "lpg")?;

    Ok(LpgProjection {
        graph,
        loss_ledger: ledger,
    })
}

/// Lift a validated canonical LPG into a concrete RDF 1.2 dataset.
///
/// The exact sideband determines RDF identity; native node ids, labels, edge types,
/// and property values are still checked and ledgered as interpretation steps. No
/// fallback inference or fabricated vocabulary is permitted.
///
/// # Errors
///
/// Returns a typed failure for malformed ordering, ambiguous/colliding records,
/// unknown or inconsistent predicate mappings, dangling nodes, limit breaches, or a
/// reconstructed dataset that fails kernel validation.
pub fn lift_lpg(graph: &LpgGraph, config: &LpgConfig) -> Result<LpgLiftOutcome, ProjectionError> {
    graph.validate(config)?;
    let mut builder = RdfDatasetBuilder::new();

    for graph_name in &graph.named_graphs {
        let id = intern_term(&mut builder, graph_name)?;
        builder.declare_named_graph(id);
    }
    for node in &graph.nodes {
        for label in &node.labels {
            push_quad(&mut builder, &label.rdf)?;
        }
        for property in &node.properties {
            push_quad(&mut builder, &property.rdf)?;
        }
    }
    for edge in &graph.edges {
        push_quad(&mut builder, &edge.rdf)?;
    }
    for row in &graph.reifiers {
        let reifier = intern_term(&mut builder, &row.reifier)?;
        let statement = intern_term(&mut builder, &row.statement)?;
        let graph_name = intern_context(&mut builder, &row.graph)?;
        builder.push_reifier_in_graph(reifier, statement, graph_name);
    }
    for row in &graph.annotations {
        let reifier = intern_term(&mut builder, &row.reifier)?;
        let predicate = builder.intern_iri(&row.predicate);
        let object = intern_term(&mut builder, &row.object)?;
        let graph_name = intern_context(&mut builder, &row.graph)?;
        builder.push_annotation_in_graph(reifier, predicate, object, graph_name);
    }
    let dataset = builder.freeze().map_err(|error| {
        ProjectionError::integrity(format!("lifted LPG produced invalid RDF: {error}"))
    })?;

    let mut ledger = LossLedger::new();
    let contract = lpg_to_rdf_loss_ledger();
    for node in &graph.nodes {
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_NODE_ID_INTERPRETED,
            "lpg:node",
            &node.id,
        );
        for label in &node.labels {
            record_loss(
                &mut ledger,
                &contract,
                LOSS_LPG_LABEL_INTERPRETED,
                "lpg:label",
                &label.statement_id,
            );
        }
        for property in &node.properties {
            record_loss(
                &mut ledger,
                &contract,
                LOSS_LPG_PROPERTY_KEY_INTERPRETED,
                "lpg:property",
                &property.statement_id,
            );
            record_loss(
                &mut ledger,
                &contract,
                LOSS_LPG_VALUE_INTERPRETED,
                "lpg:property",
                &property.statement_id,
            );
        }
    }
    for edge in &graph.edges {
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_EDGE_TYPE_INTERPRETED,
            "lpg:edge",
            &edge.id,
        );
        record_loss(
            &mut ledger,
            &contract,
            LOSS_LPG_EDGE_ID_DROPPED,
            "lpg:edge",
            &edge.id,
        );
    }
    ensure_sound(&ledger, "lpg", "rdf-1.2-dataset")?;

    Ok(LpgLiftOutcome {
        dataset,
        loss_ledger: ledger,
    })
}

struct RecordBudget {
    used: usize,
    maximum: usize,
}

impl RecordBudget {
    const fn new(maximum: usize) -> Self {
        Self { used: 0, maximum }
    }

    fn consume(&mut self, description: &str) -> Result<(), ProjectionError> {
        self.used = self
            .used
            .checked_add(1)
            .ok_or_else(|| ProjectionError::limit("LPG record count overflow"))?;
        if self.used > self.maximum {
            return Err(ProjectionError::limit(format!(
                "{description} exceeds the {}-record LPG limit",
                self.maximum
            )));
        }
        Ok(())
    }
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    limits: ProjectionLimits,
    cache: &mut BTreeMap<D::Id, ProjectionTerm>,
) -> Result<ProjectionTerm, ProjectionError> {
    if let Some(term) = cache.get(&id) {
        return Ok(term.clone());
    }
    let term = ProjectionTerm::from_view(view, id, limits)?;
    let _ = term.to_canonical_json(limits)?;
    cache.insert(id, term.clone());
    Ok(term)
}

fn context_from_id<D: DatasetView>(
    view: &D,
    id: Option<D::Id>,
    limits: ProjectionLimits,
    cache: &mut BTreeMap<D::Id, ProjectionTerm>,
    named_graphs: &mut BTreeSet<ProjectionTerm>,
    budget: &mut RecordBudget,
) -> Result<LpgGraphContext, ProjectionError> {
    let Some(id) = id else {
        return Ok(LpgGraphContext::Default);
    };
    let name = resolve_term(view, id, limits, cache)?;
    insert_named_graph(named_graphs, name.clone(), budget)?;
    Ok(LpgGraphContext::named(name))
}

fn insert_named_graph(
    graphs: &mut BTreeSet<ProjectionTerm>,
    graph: ProjectionTerm,
    budget: &mut RecordBudget,
) -> Result<(), ProjectionError> {
    if graphs.insert(graph) {
        budget.consume("named graph declaration")?;
    }
    Ok(())
}

fn reject_duplicates<T: Ord>(rows: &[T], description: &str) -> Result<(), ProjectionError> {
    if rows.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProjectionError::integrity(format!(
            "dataset view exposed duplicate {description}"
        )));
    }
    Ok(())
}

fn reject_duplicate_ids<T>(
    rows: &[T],
    id: impl Fn(&T) -> &str,
    description: &str,
) -> Result<(), ProjectionError> {
    if rows.windows(2).any(|pair| id(&pair[0]) == id(&pair[1])) {
        return Err(ProjectionError::integrity(format!(
            "dataset view exposed duplicate or colliding {description}"
        )));
    }
    Ok(())
}

fn record_loss(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    code: &'static str,
    logical: &str,
    subject: &str,
) {
    let template = contract
        .entries()
        .iter()
        .find(|entry| entry.code == code)
        .expect("runtime LPG code must exist in its closed contract");
    ledger.record(LossEntry {
        code: Cow::Borrowed(code),
        from: template.from.clone(),
        to: template.to.clone(),
        note: template.note.clone(),
        location: Some(Box::new(
            RdfLocation::logical(logical).with_subject(subject),
        )),
    });
}

fn record_term_sideband_losses<'a>(
    ledger: &mut LossLedger,
    contract: &LossLedger,
    terms: impl IntoIterator<Item = &'a ProjectionTerm>,
    graph: &LpgGraphContext,
    logical: &str,
    subject: &str,
) {
    let terms: Vec<&ProjectionTerm> = terms.into_iter().collect();
    if terms.iter().any(|term| contains_blank(term)) || graph.name().is_some_and(contains_blank) {
        record_loss(
            ledger,
            contract,
            LOSS_LPG_BLANK_SCOPE_SIDEBAND,
            logical,
            subject,
        );
    }
    if terms.iter().any(|term| contains_triple(term)) || graph.name().is_some_and(contains_triple) {
        record_loss(
            ledger,
            contract,
            LOSS_LPG_TRIPLE_TERM_SIDEBAND,
            logical,
            subject,
        );
    }
    if !matches!(graph, LpgGraphContext::Default) {
        record_loss(
            ledger,
            contract,
            LOSS_LPG_NAMED_GRAPH_SIDEBAND,
            logical,
            subject,
        );
    }
}

fn contains_blank(term: &ProjectionTerm) -> bool {
    match term {
        ProjectionTerm::Blank { .. } => true,
        ProjectionTerm::Triple {
            subject,
            predicate,
            object,
        } => contains_blank(subject) || contains_blank(predicate) || contains_blank(object),
        ProjectionTerm::Iri { .. } | ProjectionTerm::Literal { .. } => false,
    }
}

fn contains_triple(term: &ProjectionTerm) -> bool {
    matches!(term, ProjectionTerm::Triple { .. })
}

fn ensure_sound(ledger: &LossLedger, from: &str, to: &str) -> Result<(), ProjectionError> {
    check_ledger_sound(ledger, from, to).map_err(ProjectionError::integrity)
}

fn push_quad(builder: &mut RdfDatasetBuilder, quad: &LpgRdfQuad) -> Result<(), ProjectionError> {
    let subject = intern_term(builder, &quad.subject)?;
    let predicate = builder.intern_iri(&quad.predicate);
    let object = intern_term(builder, &quad.object)?;
    let graph = intern_context(builder, &quad.graph)?;
    builder.push_quad(subject, predicate, object, graph);
    Ok(())
}

fn intern_context(
    builder: &mut RdfDatasetBuilder,
    context: &LpgGraphContext,
) -> Result<Option<TermId>, ProjectionError> {
    context
        .name()
        .map(|name| intern_term(builder, name))
        .transpose()
}

fn intern_term(
    builder: &mut RdfDatasetBuilder,
    term: &ProjectionTerm,
) -> Result<TermId, ProjectionError> {
    Ok(match term {
        ProjectionTerm::Iri { value } => builder.intern_iri(value),
        ProjectionTerm::Blank { label, scope } => builder.intern_blank(label, BlankScope(*scope)),
        ProjectionTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => builder.intern_literal(RdfLiteral {
            lexical_form: lexical.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: direction.map(Into::into),
        }),
        ProjectionTerm::Triple {
            subject,
            predicate,
            object,
        } => {
            let subject = intern_term(builder, subject)?;
            let ProjectionTerm::Iri { value: predicate } = predicate.as_ref() else {
                return Err(ProjectionError::integrity(
                    "triple-term predicate is not an IRI",
                ));
            };
            let predicate = builder.intern_iri(predicate);
            let object = intern_term(builder, object)?;
            builder.intern_triple(subject, predicate, object)
        }
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use purrdf_core::loss::{
        LOSS_LPG_ANNOTATION_SIDEBAND, LOSS_LPG_BLANK_SCOPE_SIDEBAND, LOSS_LPG_EDGE_ID_DROPPED,
        LOSS_LPG_EDGE_SEMANTICS_LOWERED, LOSS_LPG_EDGE_TYPE_INTERPRETED,
        LOSS_LPG_LABEL_INTERPRETED, LOSS_LPG_LITERAL_SEMANTICS_LOWERED,
        LOSS_LPG_NAMED_GRAPH_SIDEBAND, LOSS_LPG_NODE_ID_INTERPRETED,
        LOSS_LPG_PROPERTY_KEY_INTERPRETED, LOSS_LPG_REIFIER_SIDEBAND,
        LOSS_LPG_TRIPLE_TERM_SIDEBAND, LOSS_LPG_TYPE_SEMANTICS_LOWERED, LOSS_LPG_VALUE_INTERPRETED,
    };
    use purrdf_core::{
        PackBuilder, PackView, RdfTextDirection, assert_ledger_complete, datasets_isomorphic,
    };

    use super::*;

    const TYPE: &str = "http://example.org/type";

    fn config(max_records: usize) -> LpgConfig {
        LpgConfig::new(
            TYPE,
            ProjectionLimits::new(64, 4_000_000, 8_000_000, 9_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn fixture(reverse_interning: bool) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        if reverse_interning {
            let _ = builder.intern_iri("http://example.org/z-unused");
            let _ = builder.intern_iri("http://example.org/a-unused");
        }
        let subject = builder.intern_iri("http://example.org/s");
        let predicate = builder.intern_iri("http://example.org/p");
        let object = builder.intern_iri("http://example.org/o");
        let quoted = builder.intern_triple(subject, predicate, object);
        let graph = builder.intern_iri("http://example.org/g");
        let class = builder.intern_iri("http://example.org/Class");
        let rdf_type = builder.intern_iri(TYPE);
        builder.push_quad(subject, rdf_type, class, Some(graph));

        let blank = builder.intern_blank("b", BlankScope(7));
        let label = builder.intern_iri("http://example.org/label");
        let literal = builder.intern_literal(RdfLiteral {
            lexical_form: "marhaba".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        builder.push_quad(blank, label, literal, Some(graph));
        let relates = builder.intern_iri("http://example.org/relates");
        builder.push_quad(subject, relates, quoted, None);

        let reifier = builder.intern_iri("http://example.org/reifier");
        builder.push_reifier_in_graph(reifier, quoted, Some(graph));
        let annotation_predicate = builder.intern_iri("http://example.org/confidence");
        let annotation_object = builder.intern_iri("http://example.org/high");
        builder.push_annotation_in_graph(
            reifier,
            annotation_predicate,
            annotation_object,
            Some(graph),
        );
        builder.freeze().expect("fixture")
    }

    #[test]
    fn rdf_lpg_round_trip_is_exact_and_both_ledgers_are_complete() {
        let dataset = fixture(false);
        let projected = project_lpg(dataset.as_ref(), &config(1_000)).expect("project");
        assert_ledger_complete(
            &projected.loss_ledger,
            &[
                LOSS_LPG_ANNOTATION_SIDEBAND,
                LOSS_LPG_BLANK_SCOPE_SIDEBAND,
                LOSS_LPG_EDGE_SEMANTICS_LOWERED,
                LOSS_LPG_LITERAL_SEMANTICS_LOWERED,
                LOSS_LPG_NAMED_GRAPH_SIDEBAND,
                LOSS_LPG_REIFIER_SIDEBAND,
                LOSS_LPG_TRIPLE_TERM_SIDEBAND,
                LOSS_LPG_TYPE_SEMANTICS_LOWERED,
            ],
        );
        assert!(
            projected
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );

        let lifted = lift_lpg(&projected.graph, &config(1_000)).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));
        assert_ledger_complete(
            &lifted.loss_ledger,
            &[
                LOSS_LPG_EDGE_ID_DROPPED,
                LOSS_LPG_EDGE_TYPE_INTERPRETED,
                LOSS_LPG_LABEL_INTERPRETED,
                LOSS_LPG_NODE_ID_INTERPRETED,
                LOSS_LPG_PROPERTY_KEY_INTERPRETED,
                LOSS_LPG_VALUE_INTERPRETED,
            ],
        );
        assert!(
            lifted
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );

        let projected_again =
            project_lpg(lifted.dataset.as_ref(), &config(1_000)).expect("project lifted dataset");
        assert_eq!(projected.graph, projected_again.graph);
        assert_eq!(
            projected.loss_ledger.render_json(),
            projected_again.loss_ledger.render_json()
        );
    }

    #[test]
    fn backend_and_interning_order_do_not_change_graph_or_bytes() {
        let first = fixture(false);
        let second = fixture(true);
        let config = config(1_000);
        let first_projection = project_lpg(first.as_ref(), &config).expect("first");
        let second_projection = project_lpg(second.as_ref(), &config).expect("second");
        assert_eq!(first_projection.graph, second_projection.graph);
        assert_eq!(
            first_projection
                .graph
                .to_canonical_json(&config)
                .expect("JSON"),
            second_projection
                .graph
                .to_canonical_json(&config)
                .expect("JSON")
        );

        let pack = PackBuilder::build_bytes(&first).expect("pack");
        let view = PackView::from_bytes(&pack).expect("pack view");
        let packed_projection = project_lpg(&view, &config).expect("pack projection");
        assert_eq!(first_projection.graph, packed_projection.graph);
        assert_eq!(
            first_projection.loss_ledger.render_json(),
            packed_projection.loss_ledger.render_json()
        );
    }

    #[test]
    fn strict_validation_rejects_dangling_ambiguous_and_inconsistent_models() {
        let cfg = config(1_000);
        let graph = project_lpg(fixture(false).as_ref(), &cfg)
            .expect("projection")
            .graph;

        let mut dangling = graph.clone();
        dangling.edges[0].source = "node_missing".to_owned();
        assert!(lift_lpg(&dangling, &cfg).is_err());

        let mut unknown_predicate = graph.clone();
        let property = unknown_predicate
            .nodes
            .iter_mut()
            .find_map(|node| node.properties.first_mut())
            .expect("property");
        property.key = "http://example.org/other".to_owned();
        assert!(lift_lpg(&unknown_predicate, &cfg).is_err());

        let mut duplicate_label = graph.clone();
        let labels = &mut duplicate_label
            .nodes
            .iter_mut()
            .find(|node| !node.labels.is_empty())
            .expect("label node")
            .labels;
        labels.push(labels[0].clone());
        assert!(lift_lpg(&duplicate_label, &cfg).is_err());

        let mut wrong_id = graph.clone();
        wrong_id.nodes[0].id = "node_wrong".to_owned();
        assert!(lift_lpg(&wrong_id, &cfg).is_err());

        assert!(lift_lpg(&graph, &config(1)).is_err());
        assert!(project_lpg(fixture(false).as_ref(), &config(1)).is_err());
    }

    #[test]
    fn canonical_json_and_empty_named_graph_round_trip_strictly() {
        let mut builder = RdfDatasetBuilder::new();
        let empty_iri = builder.intern_iri("http://example.org/empty");
        let empty_blank = builder.intern_blank("empty", BlankScope(11));
        builder.declare_named_graph(empty_iri);
        builder.declare_named_graph(empty_blank);
        let dataset = builder.freeze().expect("empty named graph");
        let config = config(100);
        let graph = project_lpg(dataset.as_ref(), &config).expect("project");
        assert_ledger_complete(
            &graph.loss_ledger,
            &[LOSS_LPG_BLANK_SCOPE_SIDEBAND, LOSS_LPG_NAMED_GRAPH_SIDEBAND],
        );
        assert_eq!(graph.loss_ledger.entries().len(), 3);
        assert!(
            graph
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );
        let graph = graph.graph;
        let bytes = graph.to_canonical_json(&config).expect("JSON");
        assert_eq!(
            LpgGraph::from_canonical_json(&bytes, &config).expect("parse"),
            graph
        );
        let mut padded = bytes;
        padded.push(b'\n');
        assert!(LpgGraph::from_canonical_json(&padded, &config).is_err());
        let lifted = lift_lpg(&graph, &config).expect("lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));
        assert_eq!(lifted.dataset.named_graphs().count(), 2);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        #[test]
        fn arbitrary_literal_and_blank_identity_round_trip_stably(
            lexical in "[ -~]{0,32}",
            integer in any::<i64>(),
            scope in any::<u16>(),
            named in any::<bool>(),
        ) {
            let mut builder = RdfDatasetBuilder::new();
            let subject = builder.intern_blank("generated", BlankScope(u32::from(scope)));
            let text_predicate = builder.intern_iri("http://example.org/text");
            let text = builder.intern_literal(RdfLiteral::simple(lexical));
            let number_predicate = builder.intern_iri("http://example.org/number");
            let number = builder.intern_literal(RdfLiteral::typed(
                integer.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            ));
            let edge_predicate = builder.intern_iri("http://example.org/edge");
            let target = builder.intern_iri("http://example.org/target");
            let graph = named.then(|| builder.intern_iri("http://example.org/graph"));
            builder.push_quad(subject, text_predicate, text, graph);
            builder.push_quad(subject, number_predicate, number, graph);
            builder.push_quad(subject, edge_predicate, target, graph);
            let dataset = builder.freeze().expect("generated dataset");

            let config = config(1_000);
            let first = project_lpg(dataset.as_ref(), &config).expect("project");
            let lifted = lift_lpg(&first.graph, &config).expect("lift");
            prop_assert!(datasets_isomorphic(&dataset, &lifted.dataset));
            let second = project_lpg(lifted.dataset.as_ref(), &config).expect("reproject");
            prop_assert_eq!(first.graph, second.graph);
        }
    }
}

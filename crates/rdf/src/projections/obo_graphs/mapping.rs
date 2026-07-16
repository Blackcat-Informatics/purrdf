// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_OBO_ANNOTATION_DROPPED, LOSS_OBO_BLANK_IDENTITY_DROPPED,
    LOSS_OBO_LITERAL_FIDELITY_WIDENED, LOSS_OBO_NAMED_GRAPH_DROPPED,
    LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED, LOSS_OBO_REIFIER_DROPPED, LOSS_OBO_TRIPLE_TERM_DROPPED,
};
use purrdf_core::{
    DatasetView, LossEntry, LossLedger, RdfLocation, check_ledger_sound,
    rdf_to_obo_graphs_loss_ledger,
};
use serde::Serialize;

use super::super::{ProjectionError, ProjectionTerm, stable_identifier};
use super::{
    OboDomainRangeAxiom, OboEdge, OboEquivalentNodesSet, OboExistentialRestriction, OboGraph,
    OboGraphDocument, OboGraphsConfig, OboLogicalDefinitionAxiom, OboMeta, OboNode, OboNodeType,
    OboPropertyChainAxiom, OboPropertyType, OboPropertyValue, OboSynonym, OboXref,
};

/// Result of projecting an RDF 1.2 dataset into the OBO Graphs 0.3.2 view.
#[derive(Debug, Clone)]
pub struct OboGraphsProjection {
    /// Deterministic full-IRI graph document.
    pub document: OboGraphDocument,
    /// Located, always-computed runtime loss ledger.
    pub loss_ledger: LossLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct SourceQuad {
    subject: ProjectionTerm,
    predicate: String,
    object: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct SourceReifier {
    reifier: ProjectionTerm,
    statement: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct SourceAnnotation {
    reifier: ProjectionTerm,
    predicate: String,
    object: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

#[derive(Default)]
struct NodeBuilder {
    label: Option<String>,
    node_type: Option<OboNodeType>,
    property_type: Option<OboPropertyType>,
    meta: OboMeta,
}

#[derive(Default)]
struct DomainRangeBuilder {
    domains: BTreeSet<String>,
    ranges: BTreeSet<String>,
    all_values_from_edges: Vec<OboEdge>,
    meta: OboMeta,
}

#[derive(Default)]
struct AnnotationBundle {
    xrefs: Vec<String>,
    synonym_type: Option<String>,
    meta: OboMeta,
}

impl AnnotationBundle {
    fn into_parts(mut self) -> (Vec<String>, Option<String>, Option<Box<OboMeta>>) {
        self.meta.normalize();
        let meta = (!self.meta.is_empty()).then(|| Box::new(self.meta));
        (self.xrefs, self.synonym_type, meta)
    }
}

enum RestrictionKind {
    Some(OboExistentialRestriction),
    All(OboExistentialRestriction),
}

struct ParsedRestriction {
    kind: RestrictionKind,
    structural_quads: Vec<usize>,
}

struct Projector<'a> {
    config: &'a OboGraphsConfig,
    quads: Vec<SourceQuad>,
    consumed: Vec<bool>,
    named_graphs: Vec<ProjectionTerm>,
    reifiers: Vec<SourceReifier>,
    annotations: Vec<SourceAnnotation>,
    annotation_target: Vec<Option<usize>>,
    annotation_handled: Vec<bool>,
    by_subject_predicate: BTreeMap<(ProjectionTerm, String), Vec<usize>>,
    nodes: BTreeMap<String, NodeBuilder>,
    graph_label: Option<String>,
    graph_meta: OboMeta,
    edges: Vec<OboEdge>,
    equivalence_pairs: Vec<(String, String, OboMeta)>,
    logical_definitions: Vec<OboLogicalDefinitionAxiom>,
    domain_ranges: BTreeMap<String, DomainRangeBuilder>,
    property_chains: Vec<OboPropertyChainAxiom>,
    ledger: LossLedger,
    contract: LossLedger,
}

/// Project any static RDF dataset backend into OBO Graphs 0.3.2.
///
/// All vocabulary and graph identity come from `config`. The implementation
/// resolves values before ordering them, so backend-local ids and interning order
/// cannot affect the result.
///
/// # Errors
///
/// Returns a typed configuration, term, integrity, or resource-limit failure for
/// an incomplete policy, malformed backend data, ambiguous OWL structure, or a
/// projection that exceeds the caller's explicit bounds.
pub fn project_obo_graphs<D: DatasetView>(
    view: &D,
    config: &OboGraphsConfig,
) -> Result<OboGraphsProjection, ProjectionError> {
    let mut projector = Projector::load(view, config)?;
    projector.project()
}

impl<'a> Projector<'a> {
    fn load<D: DatasetView>(
        view: &D,
        config: &'a OboGraphsConfig,
    ) -> Result<Self, ProjectionError> {
        let mut cache = BTreeMap::new();
        let mut quads = Vec::new();
        for quad in view.quads() {
            let subject = resolve_term(view, quad.s, config, &mut cache)?;
            let ProjectionTerm::Iri { value: predicate } =
                resolve_term(view, quad.p, config, &mut cache)?
            else {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a non-IRI predicate",
                ));
            };
            let object = resolve_term(view, quad.o, config, &mut cache)?;
            let graph = quad
                .g
                .map(|id| resolve_term(view, id, config, &mut cache))
                .transpose()?;
            quads.push(SourceQuad {
                subject,
                predicate,
                object,
                graph,
            });
        }
        quads.sort();
        reject_duplicates(&quads, "RDF quads")?;

        let mut reifiers = Vec::new();
        for quad in view.reifier_quads() {
            let reifier = resolve_term(view, quad.s, config, &mut cache)?;
            let predicate = resolve_term(view, quad.p, config, &mut cache)?;
            if predicate != iri(config.vocabulary().rdf().rdf_reifies()) {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a reifier row whose predicate differs from the caller-supplied rdf:reifies role",
                ));
            }
            let statement = resolve_term(view, quad.o, config, &mut cache)?;
            if !matches!(statement, ProjectionTerm::Triple { .. }) {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a reifier binding to a non-triple term",
                ));
            }
            let graph = quad
                .g
                .map(|id| resolve_term(view, id, config, &mut cache))
                .transpose()?;
            reifiers.push(SourceReifier {
                reifier,
                statement,
                graph,
            });
        }
        reifiers.sort();
        reject_duplicates(&reifiers, "RDF reifier bindings")?;

        let mut annotations = Vec::new();
        for quad in view.annotation_quads() {
            let reifier = resolve_term(view, quad.s, config, &mut cache)?;
            let ProjectionTerm::Iri { value: predicate } =
                resolve_term(view, quad.p, config, &mut cache)?
            else {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a non-IRI annotation predicate",
                ));
            };
            let object = resolve_term(view, quad.o, config, &mut cache)?;
            let graph = quad
                .g
                .map(|id| resolve_term(view, id, config, &mut cache))
                .transpose()?;
            annotations.push(SourceAnnotation {
                reifier,
                predicate,
                object,
                graph,
            });
        }
        annotations.sort();
        reject_duplicates(&annotations, "RDF statement annotations")?;

        let mut named_graphs = Vec::new();
        for graph in view.named_graphs() {
            named_graphs.push(resolve_term(view, graph, config, &mut cache)?);
        }
        named_graphs.sort();
        reject_duplicates(&named_graphs, "named graph declarations")?;

        let input_records = quads
            .len()
            .checked_add(named_graphs.len())
            .and_then(|count| count.checked_add(reifiers.len()))
            .and_then(|count| count.checked_add(annotations.len()))
            .ok_or_else(|| ProjectionError::limit("OBO Graphs input record count overflow"))?;
        if input_records > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "OBO Graphs input has {input_records} records; limit is {}",
                config.max_records()
            )));
        }

        let mut by_subject_predicate = BTreeMap::new();
        for (index, quad) in quads.iter().enumerate() {
            by_subject_predicate
                .entry((quad.subject.clone(), quad.predicate.clone()))
                .or_insert_with(Vec::new)
                .push(index);
        }

        let mut projector = Self {
            config,
            consumed: vec![false; quads.len()],
            named_graphs,
            annotation_target: vec![None; annotations.len()],
            annotation_handled: vec![false; annotations.len()],
            quads,
            reifiers,
            annotations,
            by_subject_predicate,
            nodes: BTreeMap::new(),
            graph_label: None,
            graph_meta: OboMeta::default(),
            edges: Vec::new(),
            equivalence_pairs: Vec::new(),
            logical_definitions: Vec::new(),
            domain_ranges: BTreeMap::new(),
            property_chains: Vec::new(),
            ledger: LossLedger::new(),
            contract: rdf_to_obo_graphs_loss_ledger(),
        };
        projector.index_statement_annotations()?;
        projector.record_placement_and_reifier_losses()?;
        Ok(projector)
    }

    fn project(&mut self) -> Result<OboGraphsProjection, ProjectionError> {
        self.project_declarations()?;
        self.project_equivalence_axioms()?;
        self.project_subclass_restrictions()?;
        self.project_domain_ranges()?;
        self.project_property_chains()?;
        self.project_metadata()?;
        self.project_remaining_basic_statements()?;
        self.record_unrepresented_input()?;

        let equivalent_nodes_sets = self.build_equivalence_sets()?;
        let nodes = std::mem::take(&mut self.nodes)
            .into_iter()
            .map(|(id, mut builder)| {
                builder.meta.normalize();
                OboNode {
                    id,
                    lbl: builder.label,
                    node_type: builder.node_type,
                    property_type: builder.property_type,
                    meta: (!builder.meta.is_empty()).then_some(builder.meta),
                }
            })
            .collect();

        self.graph_meta.normalize();
        let graph = OboGraph {
            id: self.config.graph_id().to_owned(),
            lbl: self.graph_label.take(),
            meta: (!self.graph_meta.is_empty()).then(|| std::mem::take(&mut self.graph_meta)),
            nodes,
            edges: std::mem::take(&mut self.edges),
            equivalent_nodes_sets,
            logical_definition_axioms: std::mem::take(&mut self.logical_definitions),
            domain_range_axioms: std::mem::take(&mut self.domain_ranges)
                .into_iter()
                .map(|(predicate_id, mut builder)| {
                    builder.meta.normalize();
                    OboDomainRangeAxiom {
                        predicate_id,
                        domain_class_ids: builder.domains.into_iter().collect(),
                        range_class_ids: builder.ranges.into_iter().collect(),
                        all_values_from_edges: builder.all_values_from_edges,
                        meta: (!builder.meta.is_empty()).then_some(builder.meta),
                    }
                })
                .collect(),
            property_chain_axioms: std::mem::take(&mut self.property_chains),
        };
        let mut document = OboGraphDocument {
            graphs: vec![graph],
        };
        document.normalize();
        document.validate(self.config)?;
        let _ = document.to_canonical_json(self.config)?;
        self.enforce_output_budget(&document)?;
        check_ledger_sound(&self.ledger, "rdf-1.2-dataset", "obo-graphs-0.3.2")
            .map_err(ProjectionError::integrity)?;

        Ok(OboGraphsProjection {
            document,
            loss_ledger: std::mem::take(&mut self.ledger),
        })
    }

    fn index_statement_annotations(&mut self) -> Result<(), ProjectionError> {
        let mut binding_targets: BTreeMap<(ProjectionTerm, Option<ProjectionTerm>), usize> =
            BTreeMap::new();
        for binding in &self.reifiers {
            let ProjectionTerm::Triple {
                subject,
                predicate,
                object,
            } = &binding.statement
            else {
                unreachable!("validated while loading")
            };
            let ProjectionTerm::Iri { value: predicate } = predicate.as_ref() else {
                return Err(ProjectionError::integrity(
                    "RDF triple-term predicate is not an IRI",
                ));
            };
            let matches: Vec<usize> = self
                .quads
                .iter()
                .enumerate()
                .filter_map(|(index, quad)| {
                    (quad.subject == **subject
                        && quad.predicate == *predicate
                        && quad.object == **object
                        && quad.graph == binding.graph)
                        .then_some(index)
                })
                .collect();
            if matches.len() > 1 {
                return Err(ProjectionError::integrity(
                    "one RDF reifier binding resolves to multiple identical source statements",
                ));
            }
            if let Some(&target) = matches.first() {
                let key = (binding.reifier.clone(), binding.graph.clone());
                if binding_targets
                    .insert(key, target)
                    .is_some_and(|prior| prior != target)
                {
                    return Err(ProjectionError::integrity(
                        "one RDF reifier identity binds multiple statements in the same graph",
                    ));
                }
            }
        }
        for (index, annotation) in self.annotations.iter().enumerate() {
            self.annotation_target[index] = binding_targets
                .get(&(annotation.reifier.clone(), annotation.graph.clone()))
                .copied();
        }
        Ok(())
    }

    fn record_placement_and_reifier_losses(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.named_graphs.len() {
            self.record_named_graph_loss(index, LOSS_OBO_NAMED_GRAPH_DROPPED)?;
            if contains_blank(&self.named_graphs[index]) {
                self.record_named_graph_loss(index, LOSS_OBO_BLANK_IDENTITY_DROPPED)?;
            }
            if contains_triple(&self.named_graphs[index]) {
                self.record_named_graph_loss(index, LOSS_OBO_TRIPLE_TERM_DROPPED)?;
            }
        }
        for index in 0..self.quads.len() {
            if self.quads[index].graph.is_some() {
                self.record_quad_loss(index, LOSS_OBO_NAMED_GRAPH_DROPPED)?;
            }
        }
        for index in 0..self.reifiers.len() {
            if self.reifiers[index].graph.is_some() {
                self.record_reifier_loss(index, LOSS_OBO_NAMED_GRAPH_DROPPED)?;
            }
            self.record_reifier_loss(index, LOSS_OBO_REIFIER_DROPPED)?;
            self.record_reifier_loss(index, LOSS_OBO_TRIPLE_TERM_DROPPED)?;
            if contains_blank(&self.reifiers[index].reifier) {
                self.record_reifier_loss(index, LOSS_OBO_BLANK_IDENTITY_DROPPED)?;
            }
        }
        for index in 0..self.annotations.len() {
            if self.annotations[index].graph.is_some() {
                self.record_annotation_loss(index, LOSS_OBO_NAMED_GRAPH_DROPPED)?;
            }
        }
        Ok(())
    }

    fn project_declarations(&mut self) -> Result<(), ProjectionError> {
        let rdf_type = self.config.vocabulary().rdf().rdf_type().to_owned();
        for index in 0..self.quads.len() {
            if self.consumed[index] || self.quads[index].predicate != rdf_type {
                continue;
            }
            let quad = self.quads[index].clone();
            let (ProjectionTerm::Iri { value: subject }, ProjectionTerm::Iri { value: object }) =
                (&quad.subject, &quad.object)
            else {
                continue;
            };
            if object == self.config.vocabulary().owl().owl_restriction() {
                continue;
            }
            if subject == self.config.graph_id()
                && object == self.config.vocabulary().owl().owl_ontology()
            {
                let meta = self.statement_meta(index)?;
                merge_meta_option(&mut self.graph_meta, meta)?;
                self.consumed[index] = true;
                continue;
            }
            let declaration = if object == self.config.vocabulary().owl().owl_class() {
                Some((OboNodeType::Class, None))
            } else if object == self.config.vocabulary().owl().owl_named_individual() {
                Some((OboNodeType::Individual, None))
            } else if object == self.config.vocabulary().owl().owl_object_property() {
                Some((OboNodeType::Property, Some(OboPropertyType::Object)))
            } else if object == self.config.vocabulary().owl().owl_annotation_property() {
                Some((OboNodeType::Property, Some(OboPropertyType::Annotation)))
            } else if object == self.config.vocabulary().owl().owl_datatype_property() {
                Some((OboNodeType::Property, Some(OboPropertyType::Data)))
            } else {
                None
            };
            if let Some((node_type, property_type)) = declaration {
                let meta = self.statement_meta(index)?;
                let node = self.node_mut(subject);
                if node.node_type.is_some_and(|existing| existing != node_type)
                    || node
                        .property_type
                        .is_some_and(|existing| Some(existing) != property_type)
                {
                    return Err(ProjectionError::integrity(format!(
                        "OBO node `{subject}` has contradictory configured declaration types"
                    )));
                }
                node.node_type = Some(node_type);
                node.property_type = property_type;
                merge_meta_option(&mut node.meta, meta)?;
                self.consumed[index] = true;
            }
        }
        Ok(())
    }

    fn project_equivalence_axioms(&mut self) -> Result<(), ProjectionError> {
        let predicate = self
            .config
            .vocabulary()
            .owl()
            .owl_equivalent_class()
            .to_owned();
        for index in 0..self.quads.len() {
            if self.consumed[index] || self.quads[index].predicate != predicate {
                continue;
            }
            let quad = self.quads[index].clone();
            match (&quad.subject, &quad.object) {
                (ProjectionTerm::Iri { value: left }, ProjectionTerm::Iri { value: right }) => {
                    let meta = self.statement_meta(index)?.unwrap_or_default();
                    self.node_mut(left);
                    self.node_mut(right);
                    self.equivalence_pairs
                        .push((left.clone(), right.clone(), meta));
                    self.consumed[index] = true;
                }
                (
                    ProjectionTerm::Iri { value: defined },
                    expression @ ProjectionTerm::Blank { .. },
                )
                | (
                    expression @ ProjectionTerm::Blank { .. },
                    ProjectionTerm::Iri { value: defined },
                ) => {
                    let (mut axiom, structural) =
                        self.parse_logical_definition(defined, expression, quad.graph.as_ref())?;
                    let mut meta = self.statement_meta(index)?.unwrap_or_default();
                    for structural_index in structural {
                        self.consumed[structural_index] = true;
                    }
                    meta.normalize();
                    axiom.meta = (!meta.is_empty()).then_some(meta);
                    self.node_mut(defined);
                    for genus in &axiom.genus_ids {
                        self.node_mut(genus);
                    }
                    for restriction in &axiom.restrictions {
                        self.node_mut(&restriction.property_id);
                        self.node_mut(&restriction.filler_id);
                    }
                    self.logical_definitions.push(axiom);
                    self.consumed[index] = true;
                }
                _ => {
                    return Err(ProjectionError::integrity(format!(
                        "ambiguous owl:equivalentClass pattern at {}: expected two named classes or one named class and one intersection blank node",
                        self.quad_location(index)?
                    )));
                }
            }
        }
        Ok(())
    }

    fn project_subclass_restrictions(&mut self) -> Result<(), ProjectionError> {
        let predicate = self
            .config
            .vocabulary()
            .owl()
            .rdfs_sub_class_of()
            .to_owned();
        for index in 0..self.quads.len() {
            if self.consumed[index] || self.quads[index].predicate != predicate {
                continue;
            }
            let quad = self.quads[index].clone();
            let (
                ProjectionTerm::Iri { value: subject },
                restriction @ ProjectionTerm::Blank { .. },
            ) = (&quad.subject, &quad.object)
            else {
                continue;
            };
            let parsed = self.parse_restriction(restriction, quad.graph.as_ref())?;
            let mut meta = self.statement_meta(index)?.unwrap_or_default();
            for structural_index in parsed.structural_quads {
                self.consumed[structural_index] = true;
            }
            meta.normalize();
            match parsed.kind {
                RestrictionKind::Some(restriction) => {
                    self.node_mut(subject);
                    self.node_mut(&restriction.property_id);
                    self.node_mut(&restriction.filler_id);
                    self.edges.push(OboEdge {
                        sub: subject.clone(),
                        pred: restriction.property_id,
                        obj: restriction.filler_id,
                        meta: (!meta.is_empty()).then_some(meta),
                    });
                }
                RestrictionKind::All(restriction) => {
                    self.node_mut(subject);
                    self.node_mut(&restriction.property_id);
                    self.node_mut(&restriction.filler_id);
                    let builder = self
                        .domain_ranges
                        .entry(restriction.property_id.clone())
                        .or_default();
                    builder.all_values_from_edges.push(OboEdge {
                        sub: subject.clone(),
                        pred: restriction.property_id,
                        obj: restriction.filler_id,
                        meta: (!meta.is_empty()).then_some(meta),
                    });
                }
            }
            self.consumed[index] = true;
        }
        Ok(())
    }

    fn project_domain_ranges(&mut self) -> Result<(), ProjectionError> {
        let domain = self.config.vocabulary().owl().rdfs_domain().to_owned();
        let range = self.config.vocabulary().owl().rdfs_range().to_owned();
        for index in 0..self.quads.len() {
            if self.consumed[index]
                || (self.quads[index].predicate != domain && self.quads[index].predicate != range)
            {
                continue;
            }
            let quad = self.quads[index].clone();
            let (ProjectionTerm::Iri { value: property }, ProjectionTerm::Iri { value: class }) =
                (&quad.subject, &quad.object)
            else {
                return Err(ProjectionError::integrity(format!(
                    "ambiguous configured domain/range axiom at {}: property and class must be named IRIs",
                    self.quad_location(index)?
                )));
            };
            let meta = self.statement_meta(index)?;
            let builder = self.domain_ranges.entry(property.clone()).or_default();
            if quad.predicate == domain {
                builder.domains.insert(class.clone());
            } else {
                builder.ranges.insert(class.clone());
            }
            merge_meta_option(&mut builder.meta, meta)?;
            self.node_mut(property);
            self.node_mut(class);
            self.consumed[index] = true;
        }
        Ok(())
    }

    fn project_property_chains(&mut self) -> Result<(), ProjectionError> {
        let predicate = self
            .config
            .vocabulary()
            .owl()
            .owl_property_chain_axiom()
            .to_owned();
        for index in 0..self.quads.len() {
            if self.consumed[index] || self.quads[index].predicate != predicate {
                continue;
            }
            let quad = self.quads[index].clone();
            let ProjectionTerm::Iri { value: property } = &quad.subject else {
                return Err(ProjectionError::integrity(format!(
                    "ambiguous property-chain axiom at {}: super-property must be a named IRI",
                    self.quad_location(index)?
                )));
            };
            let (members, structural) = self.parse_list(&quad.object, quad.graph.as_ref())?;
            if members.len() < 2 {
                return Err(ProjectionError::integrity(
                    "an OBO Graphs property chain must contain at least two predicates",
                ));
            }
            let mut chain = Vec::with_capacity(members.len());
            for member in members {
                let ProjectionTerm::Iri { value } = member else {
                    return Err(ProjectionError::integrity(
                        "an OBO Graphs property chain contains a non-IRI member",
                    ));
                };
                self.node_mut(&value);
                chain.push(value);
            }
            let mut meta = self.statement_meta(index)?.unwrap_or_default();
            for structural_index in structural {
                self.consumed[structural_index] = true;
            }
            meta.normalize();
            self.node_mut(property);
            self.property_chains.push(OboPropertyChainAxiom {
                predicate_id: property.clone(),
                chain_predicate_ids: chain,
                meta: (!meta.is_empty()).then_some(meta),
            });
            self.consumed[index] = true;
        }
        Ok(())
    }

    fn project_metadata(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.quads.len() {
            if self.consumed[index] || !self.is_metadata_predicate(&self.quads[index].predicate) {
                continue;
            }
            let quad = self.quads[index].clone();
            let ProjectionTerm::Iri { value: subject } = &quad.subject else {
                continue;
            };
            if quad.predicate == self.config.vocabulary().owl().rdfs_label() {
                self.project_label(index, subject)?;
            } else {
                let mut target = if subject == self.config.graph_id() {
                    std::mem::take(&mut self.graph_meta)
                } else {
                    std::mem::take(&mut self.node_mut(subject).meta)
                };
                self.add_metadata_statement(&mut target, index)?;
                if subject == self.config.graph_id() {
                    self.graph_meta = target;
                } else {
                    self.node_mut(subject).meta = target;
                }
            }
            self.consumed[index] = true;
        }
        Ok(())
    }

    fn project_label(&mut self, index: usize, subject: &str) -> Result<(), ProjectionError> {
        let value = self.scalar_from_quad(index)?;
        let Some(value) = value else {
            return Ok(());
        };
        let bundle = self.property_annotations(index, false)?;
        let annotations_present =
            !bundle.xrefs.is_empty() || bundle.synonym_type.is_some() || !bundle.meta.is_empty();
        let (xrefs, _synonym_type, nested) = bundle.into_parts();
        let predicate = self.quads[index].predicate.clone();
        let property = OboPropertyValue {
            pred: predicate,
            val: value.clone(),
            xrefs,
            meta: nested,
        };
        if subject == self.config.graph_id() {
            if self.graph_label.is_none() {
                self.graph_label = Some(value);
            }
            if annotations_present || self.graph_label.as_deref() != Some(property.val.as_str()) {
                self.graph_meta.basic_property_values.push(property);
            }
        } else {
            let node = self.node_mut(subject);
            if node.label.is_none() {
                node.label = Some(value);
            }
            if annotations_present || node.label.as_deref() != Some(property.val.as_str()) {
                node.meta.basic_property_values.push(property);
            }
        }
        Ok(())
    }

    fn project_remaining_basic_statements(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.quads.len() {
            if self.consumed[index] {
                continue;
            }
            let quad = self.quads[index].clone();
            if self.is_structural_predicate(&quad.predicate) {
                continue;
            }
            match (&quad.subject, &quad.object) {
                (ProjectionTerm::Iri { value: subject }, ProjectionTerm::Iri { value: object }) => {
                    if subject == self.config.graph_id() {
                        let mut meta = std::mem::take(&mut self.graph_meta);
                        self.add_basic_property(&mut meta, index)?;
                        self.graph_meta = meta;
                    } else {
                        let statement_meta = self.statement_meta(index)?;
                        self.node_mut(subject);
                        self.node_mut(object);
                        self.edges.push(OboEdge {
                            sub: subject.clone(),
                            pred: quad.predicate,
                            obj: object.clone(),
                            meta: statement_meta,
                        });
                    }
                    self.consumed[index] = true;
                }
                (ProjectionTerm::Iri { value: subject }, ProjectionTerm::Literal { .. }) => {
                    let mut meta = if subject == self.config.graph_id() {
                        std::mem::take(&mut self.graph_meta)
                    } else {
                        std::mem::take(&mut self.node_mut(subject).meta)
                    };
                    self.add_basic_property(&mut meta, index)?;
                    if subject == self.config.graph_id() {
                        self.graph_meta = meta;
                    } else {
                        self.node_mut(subject).meta = meta;
                    }
                    self.consumed[index] = true;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn add_metadata_statement(
        &mut self,
        meta: &mut OboMeta,
        index: usize,
    ) -> Result<(), ProjectionError> {
        let predicate = self.quads[index].predicate.clone();
        let Some(value) = self.scalar_from_quad(index)? else {
            return Ok(());
        };
        let synonym = self.synonym_scope(&predicate).map(str::to_owned);
        let mut bundle = self.property_annotations(index, synonym.is_some())?;
        bundle.xrefs.sort();
        bundle.xrefs.dedup();
        let (xrefs, synonym_type, nested) = bundle.into_parts();
        if predicate == self.config.vocabulary().metadata().definition() {
            let property = OboPropertyValue {
                pred: predicate,
                val: value,
                xrefs,
                meta: nested,
            };
            if meta.definition.is_none() {
                meta.definition = Some(property);
            } else {
                meta.basic_property_values.push(property);
            }
        } else if let Some(scope) = synonym {
            meta.synonyms.push(OboSynonym {
                synonym_type,
                pred: scope,
                val: value,
                xrefs,
                meta: nested,
            });
        } else if predicate == self.config.vocabulary().metadata().xref() {
            meta.xrefs.push(OboXref {
                lbl: None,
                pred: predicate,
                val: value,
                xrefs,
                meta: nested,
            });
        } else if predicate == self.config.vocabulary().owl().rdfs_comment() {
            meta.comments.push(value.clone());
            if nested.is_some() || !xrefs.is_empty() {
                meta.basic_property_values.push(OboPropertyValue {
                    pred: predicate,
                    val: value,
                    xrefs,
                    meta: nested,
                });
            }
        } else if predicate == self.config.vocabulary().metadata().subset() {
            meta.subsets.push(value.clone());
            if nested.is_some() || !xrefs.is_empty() {
                meta.basic_property_values.push(OboPropertyValue {
                    pred: predicate,
                    val: value,
                    xrefs,
                    meta: nested,
                });
            }
        } else if predicate == self.config.vocabulary().metadata().version() {
            if meta
                .version
                .as_ref()
                .is_none_or(|existing| existing == &value)
            {
                meta.version = Some(value);
            } else {
                meta.basic_property_values.push(OboPropertyValue {
                    pred: predicate,
                    val: value,
                    xrefs,
                    meta: nested,
                });
            }
        } else if predicate == self.config.vocabulary().owl().owl_deprecated() {
            let deprecated = self.boolean_from_quad(index)?;
            if deprecated {
                meta.deprecated = true;
            }
            if !deprecated || nested.is_some() || !xrefs.is_empty() {
                meta.basic_property_values.push(OboPropertyValue {
                    pred: predicate,
                    val: value,
                    xrefs,
                    meta: nested,
                });
            }
        } else {
            meta.basic_property_values.push(OboPropertyValue {
                pred: predicate,
                val: value,
                xrefs,
                meta: nested,
            });
        }
        Ok(())
    }

    fn add_basic_property(
        &mut self,
        meta: &mut OboMeta,
        index: usize,
    ) -> Result<(), ProjectionError> {
        let Some(value) = self.scalar_from_quad(index)? else {
            return Ok(());
        };
        let bundle = self.property_annotations(index, false)?;
        let (xrefs, _synonym_type, nested) = bundle.into_parts();
        meta.basic_property_values.push(OboPropertyValue {
            pred: self.quads[index].predicate.clone(),
            val: value,
            xrefs,
            meta: nested,
        });
        Ok(())
    }

    fn property_annotations(
        &mut self,
        quad_index: usize,
        extract_synonym_type: bool,
    ) -> Result<AnnotationBundle, ProjectionError> {
        let annotation_indices: Vec<usize> = self
            .annotation_target
            .iter()
            .enumerate()
            .filter_map(|(index, target)| (*target == Some(quad_index)).then_some(index))
            .collect();
        let mut bundle = AnnotationBundle::default();
        for index in annotation_indices {
            if self.annotation_handled[index] {
                continue;
            }
            let annotation = self.annotations[index].clone();
            let Some(value) = self.scalar_from_annotation(index)? else {
                self.record_annotation_loss(index, LOSS_OBO_ANNOTATION_DROPPED)?;
                self.annotation_handled[index] = true;
                continue;
            };
            if annotation.predicate == self.config.vocabulary().metadata().xref() {
                bundle.xrefs.push(value);
            } else if extract_synonym_type
                && annotation.predicate == self.config.vocabulary().metadata().synonym_type()
            {
                if bundle
                    .synonym_type
                    .as_ref()
                    .is_some_and(|existing| existing != &value)
                {
                    return Err(ProjectionError::integrity(
                        "one synonym statement has multiple distinct configured synonym types",
                    ));
                }
                bundle.synonym_type = Some(value);
            } else {
                self.add_annotation_meta(&mut bundle.meta, &annotation.predicate, value, index)?;
            }
            self.annotation_handled[index] = true;
        }
        Ok(bundle)
    }

    fn statement_meta(&mut self, quad_index: usize) -> Result<Option<OboMeta>, ProjectionError> {
        let bundle = self.property_annotations(quad_index, false)?;
        let mut meta = bundle.meta;
        for value in bundle.xrefs {
            meta.xrefs.push(OboXref {
                lbl: None,
                pred: self.config.vocabulary().metadata().xref().to_owned(),
                val: value,
                xrefs: Vec::new(),
                meta: None,
            });
        }
        if let Some(value) = bundle.synonym_type {
            meta.basic_property_values.push(OboPropertyValue {
                pred: self
                    .config
                    .vocabulary()
                    .metadata()
                    .synonym_type()
                    .to_owned(),
                val: value,
                xrefs: Vec::new(),
                meta: None,
            });
        }
        meta.normalize();
        Ok((!meta.is_empty()).then_some(meta))
    }

    fn add_annotation_meta(
        &self,
        meta: &mut OboMeta,
        predicate: &str,
        value: String,
        annotation_index: usize,
    ) -> Result<(), ProjectionError> {
        if predicate == self.config.vocabulary().metadata().subset() {
            meta.subsets.push(value);
        } else if predicate == self.config.vocabulary().owl().rdfs_comment() {
            meta.comments.push(value);
        } else if predicate == self.config.vocabulary().metadata().xref() {
            meta.xrefs.push(OboXref {
                lbl: None,
                pred: predicate.to_owned(),
                val: value,
                xrefs: Vec::new(),
                meta: None,
            });
        } else if predicate == self.config.vocabulary().owl().owl_deprecated() {
            if self.boolean_from_annotation(annotation_index)? {
                meta.deprecated = true;
            } else {
                meta.basic_property_values.push(OboPropertyValue {
                    pred: predicate.to_owned(),
                    val: value,
                    xrefs: Vec::new(),
                    meta: None,
                });
            }
        } else {
            meta.basic_property_values.push(OboPropertyValue {
                pred: predicate.to_owned(),
                val: value,
                xrefs: Vec::new(),
                meta: None,
            });
        }
        Ok(())
    }

    fn parse_logical_definition(
        &self,
        defined: &str,
        expression: &ProjectionTerm,
        graph: Option<&ProjectionTerm>,
    ) -> Result<(OboLogicalDefinitionAxiom, Vec<usize>), ProjectionError> {
        let intersection = self.required_object(
            expression,
            self.config.vocabulary().owl().owl_intersection_of(),
            graph,
            "logical-definition intersection",
        )?;
        let (members, mut structural) = self.parse_list(&intersection.1, graph)?;
        structural.push(intersection.0);
        let mut genera = Vec::new();
        let mut restrictions = Vec::new();
        for member in members {
            match member {
                ProjectionTerm::Iri { value } => genera.push(value),
                blank @ ProjectionTerm::Blank { .. } => {
                    let parsed = self.parse_restriction(&blank, graph)?;
                    let RestrictionKind::Some(restriction) = parsed.kind else {
                        return Err(ProjectionError::integrity(
                            "logical definitions admit named existential restrictions, not all-values-from restrictions",
                        ));
                    };
                    restrictions.push(restriction);
                    structural.extend(parsed.structural_quads);
                }
                _ => {
                    return Err(ProjectionError::integrity(
                        "logical-definition intersection contains a literal or triple-term member",
                    ));
                }
            }
        }
        if genera.is_empty() && restrictions.is_empty() {
            return Err(ProjectionError::integrity(
                "logical-definition intersection is empty",
            ));
        }
        genera.sort();
        genera.dedup();
        restrictions.sort();
        restrictions.dedup();
        structural.sort_unstable();
        structural.dedup();
        Ok((
            OboLogicalDefinitionAxiom {
                defined_class_id: defined.to_owned(),
                genus_ids: genera,
                restrictions,
                meta: None,
            },
            structural,
        ))
    }

    fn parse_restriction(
        &self,
        restriction: &ProjectionTerm,
        graph: Option<&ProjectionTerm>,
    ) -> Result<ParsedRestriction, ProjectionError> {
        if !matches!(restriction, ProjectionTerm::Blank { .. }) {
            return Err(ProjectionError::integrity(
                "OWL restriction expression must be a blank node",
            ));
        }
        let declaration = self.required_object(
            restriction,
            self.config.vocabulary().rdf().rdf_type(),
            graph,
            "restriction declaration",
        )?;
        if declaration.1 != iri(self.config.vocabulary().owl().owl_restriction()) {
            return Err(ProjectionError::integrity(
                "restriction blank node is not declared with the configured owl:Restriction IRI",
            ));
        }
        let on_property = self.required_object(
            restriction,
            self.config.vocabulary().owl().owl_on_property(),
            graph,
            "restriction property",
        )?;
        let ProjectionTerm::Iri { value: property_id } = on_property.1 else {
            return Err(ProjectionError::integrity(
                "OWL restriction property must be a named IRI",
            ));
        };
        let some = self.optional_object(
            restriction,
            self.config.vocabulary().owl().owl_some_values_from(),
            graph,
            "existential restriction filler",
        )?;
        let all = self.optional_object(
            restriction,
            self.config.vocabulary().owl().owl_all_values_from(),
            graph,
            "all-values-from restriction filler",
        )?;
        let (filler_index, filler, is_some) = match (some, all) {
            (Some((index, filler)), None) => (index, filler, true),
            (None, Some((index, filler))) => (index, filler, false),
            (None, None) => {
                return Err(ProjectionError::integrity(
                    "OWL restriction has neither a configured someValuesFrom nor allValuesFrom filler",
                ));
            }
            (Some(_), Some(_)) => {
                return Err(ProjectionError::integrity(
                    "ambiguous OWL restriction has both someValuesFrom and allValuesFrom fillers",
                ));
            }
        };
        let ProjectionTerm::Iri { value: filler_id } = filler else {
            return Err(ProjectionError::integrity(
                "OWL restriction filler must be a named class IRI",
            ));
        };
        let restriction = OboExistentialRestriction {
            property_id,
            filler_id,
        };
        Ok(ParsedRestriction {
            kind: if is_some {
                RestrictionKind::Some(restriction)
            } else {
                RestrictionKind::All(restriction)
            },
            structural_quads: vec![declaration.0, on_property.0, filler_index],
        })
    }

    fn parse_list(
        &self,
        head: &ProjectionTerm,
        graph: Option<&ProjectionTerm>,
    ) -> Result<(Vec<ProjectionTerm>, Vec<usize>), ProjectionError> {
        let nil = iri(self.config.vocabulary().rdf().rdf_nil());
        if *head == nil {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut cursor = head.clone();
        let mut visited = BTreeSet::new();
        let mut members = Vec::new();
        let mut structural = Vec::new();
        loop {
            if cursor == nil {
                break;
            }
            if !matches!(cursor, ProjectionTerm::Blank { .. }) {
                return Err(ProjectionError::integrity(
                    "RDF collection tail must be the configured rdf:nil IRI or a blank list node",
                ));
            }
            if !visited.insert(cursor.clone()) {
                return Err(ProjectionError::integrity(
                    "cyclic RDF collection in OWL axiom",
                ));
            }
            if visited.len() > self.config.max_records() {
                return Err(ProjectionError::limit(
                    "RDF collection exceeds the configured OBO Graphs record limit",
                ));
            }
            let first = self.required_object(
                &cursor,
                self.config.vocabulary().rdf().rdf_first(),
                graph,
                "RDF collection first",
            )?;
            let rest = self.required_object(
                &cursor,
                self.config.vocabulary().rdf().rdf_rest(),
                graph,
                "RDF collection rest",
            )?;
            structural.push(first.0);
            structural.push(rest.0);
            members.push(first.1);
            cursor = rest.1;
        }
        Ok((members, structural))
    }

    fn required_object(
        &self,
        subject: &ProjectionTerm,
        predicate: &str,
        graph: Option<&ProjectionTerm>,
        description: &str,
    ) -> Result<(usize, ProjectionTerm), ProjectionError> {
        self.optional_object(subject, predicate, graph, description)?
            .ok_or_else(|| ProjectionError::integrity(format!("missing {description}")))
    }

    fn optional_object(
        &self,
        subject: &ProjectionTerm,
        predicate: &str,
        graph: Option<&ProjectionTerm>,
        description: &str,
    ) -> Result<Option<(usize, ProjectionTerm)>, ProjectionError> {
        let matches: Vec<usize> = self
            .by_subject_predicate
            .get(&(subject.clone(), predicate.to_owned()))
            .into_iter()
            .flatten()
            .copied()
            .filter(|&index| self.quads[index].graph.as_ref() == graph)
            .collect();
        if matches.len() > 1 {
            return Err(ProjectionError::integrity(format!(
                "ambiguous {description}: expected exactly one statement in the source graph"
            )));
        }
        Ok(matches
            .first()
            .map(|&index| (index, self.quads[index].object.clone())))
    }

    fn build_equivalence_sets(&self) -> Result<Vec<OboEquivalentNodesSet>, ProjectionError> {
        let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (left, right, _) in &self.equivalence_pairs {
            adjacency
                .entry(left.clone())
                .or_default()
                .insert(right.clone());
            adjacency
                .entry(right.clone())
                .or_default()
                .insert(left.clone());
        }
        let mut visited = BTreeSet::new();
        let mut sets = Vec::new();
        for root in adjacency.keys() {
            if !visited.insert(root.clone()) {
                continue;
            }
            let mut pending = vec![root.clone()];
            let mut members = BTreeSet::new();
            while let Some(node) = pending.pop() {
                members.insert(node.clone());
                if let Some(neighbors) = adjacency.get(&node) {
                    for neighbor in neighbors.iter().rev() {
                        if visited.insert(neighbor.clone()) {
                            pending.push(neighbor.clone());
                        }
                    }
                }
            }
            let mut meta = OboMeta::default();
            for (left, right, pair_meta) in &self.equivalence_pairs {
                if members.contains(left) && members.contains(right) {
                    merge_meta(&mut meta, pair_meta.clone())?;
                }
            }
            meta.normalize();
            let node_ids: Vec<String> = members.into_iter().collect();
            sets.push(OboEquivalentNodesSet {
                representative_node_id: node_ids[0].clone(),
                node_ids,
                meta: (!meta.is_empty()).then_some(meta),
            });
        }
        Ok(sets)
    }

    fn record_unrepresented_input(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.quads.len() {
            if self.consumed[index] {
                continue;
            }
            let quad = self.quads[index].clone();
            if contains_triple(&quad.subject) || contains_triple(&quad.object) {
                self.record_quad_loss(index, LOSS_OBO_TRIPLE_TERM_DROPPED)?;
            }
            if contains_blank(&quad.subject) || contains_blank(&quad.object) {
                self.record_quad_loss(index, LOSS_OBO_BLANK_IDENTITY_DROPPED)?;
            }
            self.record_quad_loss(index, LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED)?;
        }
        for index in 0..self.annotations.len() {
            if self.annotation_handled[index] {
                continue;
            }
            if contains_triple(&self.annotations[index].object) {
                self.record_annotation_loss(index, LOSS_OBO_TRIPLE_TERM_DROPPED)?;
            }
            if contains_blank(&self.annotations[index].reifier)
                || contains_blank(&self.annotations[index].object)
            {
                self.record_annotation_loss(index, LOSS_OBO_BLANK_IDENTITY_DROPPED)?;
            }
            self.record_annotation_loss(index, LOSS_OBO_ANNOTATION_DROPPED)?;
            self.annotation_handled[index] = true;
        }
        Ok(())
    }

    fn scalar_from_quad(&mut self, index: usize) -> Result<Option<String>, ProjectionError> {
        let term = self.quads[index].object.clone();
        match term {
            ProjectionTerm::Iri { value } => Ok(Some(value)),
            ProjectionTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                if datatype != self.config.vocabulary().rdf().xsd_string()
                    || language.is_some()
                    || direction.is_some()
                {
                    self.record_quad_loss(index, LOSS_OBO_LITERAL_FIDELITY_WIDENED)?;
                }
                Ok(Some(lexical))
            }
            ProjectionTerm::Blank { .. } => Ok(None),
            ProjectionTerm::Triple { .. } => Ok(None),
        }
    }

    fn scalar_from_annotation(&mut self, index: usize) -> Result<Option<String>, ProjectionError> {
        let term = self.annotations[index].object.clone();
        match term {
            ProjectionTerm::Iri { value } => Ok(Some(value)),
            ProjectionTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                if datatype != self.config.vocabulary().rdf().xsd_string()
                    || language.is_some()
                    || direction.is_some()
                {
                    self.record_annotation_loss(index, LOSS_OBO_LITERAL_FIDELITY_WIDENED)?;
                }
                Ok(Some(lexical))
            }
            ProjectionTerm::Blank { .. } | ProjectionTerm::Triple { .. } => Ok(None),
        }
    }

    fn boolean_from_quad(&self, index: usize) -> Result<bool, ProjectionError> {
        let ProjectionTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = &self.quads[index].object
        else {
            return Err(ProjectionError::integrity(
                "configured owl:deprecated value must be an xsd:boolean literal",
            ));
        };
        if datatype != self.config.vocabulary().rdf().xsd_boolean()
            || language.is_some()
            || direction.is_some()
        {
            return Err(ProjectionError::integrity(
                "configured owl:deprecated value must use the caller-supplied xsd:boolean IRI without language or direction",
            ));
        }
        match lexical.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(ProjectionError::integrity(
                "configured owl:deprecated literal has an invalid xsd:boolean lexical form",
            )),
        }
    }

    fn boolean_from_annotation(&self, index: usize) -> Result<bool, ProjectionError> {
        let ProjectionTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = &self.annotations[index].object
        else {
            return Err(ProjectionError::integrity(
                "configured owl:deprecated annotation must be an xsd:boolean literal",
            ));
        };
        if datatype != self.config.vocabulary().rdf().xsd_boolean()
            || language.is_some()
            || direction.is_some()
        {
            return Err(ProjectionError::integrity(
                "configured owl:deprecated annotation must use the caller-supplied xsd:boolean IRI without language or direction",
            ));
        }
        match lexical.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(ProjectionError::integrity(
                "configured owl:deprecated annotation has an invalid xsd:boolean lexical form",
            )),
        }
    }

    fn is_metadata_predicate(&self, predicate: &str) -> bool {
        let owl = self.config.vocabulary().owl();
        let metadata = self.config.vocabulary().metadata();
        predicate == owl.rdfs_label()
            || predicate == owl.rdfs_comment()
            || predicate == owl.owl_deprecated()
            || predicate == metadata.definition()
            || predicate == metadata.exact_synonym()
            || predicate == metadata.broad_synonym()
            || predicate == metadata.narrow_synonym()
            || predicate == metadata.related_synonym()
            || predicate == metadata.xref()
            || predicate == metadata.subset()
            || predicate == metadata.version()
    }

    fn synonym_scope<'b>(&'b self, predicate: &'b str) -> Option<&'b str> {
        let metadata = self.config.vocabulary().metadata();
        [
            metadata.exact_synonym(),
            metadata.broad_synonym(),
            metadata.narrow_synonym(),
            metadata.related_synonym(),
        ]
        .into_iter()
        .find(|candidate| *candidate == predicate)
    }

    fn is_structural_predicate(&self, predicate: &str) -> bool {
        let rdf = self.config.vocabulary().rdf();
        let owl = self.config.vocabulary().owl();
        predicate == rdf.rdf_first()
            || predicate == rdf.rdf_rest()
            || predicate == owl.owl_intersection_of()
            || predicate == owl.owl_on_property()
            || predicate == owl.owl_some_values_from()
            || predicate == owl.owl_all_values_from()
            || predicate == owl.owl_equivalent_class()
            || predicate == owl.owl_property_chain_axiom()
            || predicate == owl.rdfs_domain()
            || predicate == owl.rdfs_range()
    }

    fn node_mut(&mut self, id: &str) -> &mut NodeBuilder {
        self.nodes.entry(id.to_owned()).or_default()
    }

    fn enforce_output_budget(&self, document: &OboGraphDocument) -> Result<(), ProjectionError> {
        let graph = &document.graphs[0];
        let output = graph
            .nodes
            .len()
            .checked_add(graph.edges.len())
            .and_then(|count| count.checked_add(graph.equivalent_nodes_sets.len()))
            .and_then(|count| count.checked_add(graph.logical_definition_axioms.len()))
            .and_then(|count| count.checked_add(graph.domain_range_axioms.len()))
            .and_then(|count| count.checked_add(graph.property_chain_axioms.len()))
            .ok_or_else(|| ProjectionError::limit("OBO Graphs output record count overflow"))?;
        let total = self
            .quads
            .len()
            .checked_add(self.named_graphs.len())
            .and_then(|count| count.checked_add(self.reifiers.len()))
            .and_then(|count| count.checked_add(self.annotations.len()))
            .and_then(|count| count.checked_add(output))
            .ok_or_else(|| ProjectionError::limit("OBO Graphs total record count overflow"))?;
        if total > self.config.max_records() {
            return Err(ProjectionError::limit(format!(
                "OBO Graphs projection uses {total} input/output records; limit is {}",
                self.config.max_records()
            )));
        }
        Ok(())
    }

    fn record_quad_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = self.quad_location(index)?;
        self.record_loss(code, "obo-graphs:quad", subject);
        Ok(())
    }

    fn record_named_graph_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("graph", &self.named_graphs[index])?;
        self.record_loss(code, "obo-graphs:named-graph", subject);
        Ok(())
    }

    fn record_reifier_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("reifier", &self.reifiers[index])?;
        self.record_loss(code, "obo-graphs:reifier", subject);
        Ok(())
    }

    fn record_annotation_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("annotation", &self.annotations[index])?;
        self.record_loss(code, "obo-graphs:annotation", subject);
        Ok(())
    }

    fn quad_location(&self, index: usize) -> Result<String, ProjectionError> {
        source_identifier("quad", &self.quads[index])
    }

    fn record_loss(&mut self, code: &'static str, logical: &str, subject: String) {
        let template = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .expect("runtime OBO Graphs code must exist in the closed contract");
        self.ledger.record(LossEntry {
            code: Cow::Borrowed(code),
            from: template.from.clone(),
            to: template.to.clone(),
            note: template.note.clone(),
            location: Some(Box::new(
                RdfLocation::logical(logical).with_subject(subject),
            )),
        });
    }
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    config: &OboGraphsConfig,
    cache: &mut BTreeMap<D::Id, ProjectionTerm>,
) -> Result<ProjectionTerm, ProjectionError> {
    if let Some(term) = cache.get(&id) {
        return Ok(term.clone());
    }
    let term = ProjectionTerm::from_view(view, id, config.limits())?;
    let _ = term.to_canonical_json(config.limits())?;
    cache.insert(id, term.clone());
    Ok(term)
}

fn iri(value: &str) -> ProjectionTerm {
    ProjectionTerm::Iri {
        value: value.to_owned(),
    }
}

fn source_identifier(prefix: &str, value: &impl Serialize) -> Result<String, ProjectionError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ProjectionError::integrity(format!("serialize source location: {error}"))
    })?;
    stable_identifier(prefix, &bytes)
}

fn reject_duplicates<T: Ord>(values: &[T], description: &str) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProjectionError::integrity(format!(
            "dataset view exposed duplicate {description}"
        )));
    }
    Ok(())
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

fn merge_meta_option(target: &mut OboMeta, source: Option<OboMeta>) -> Result<(), ProjectionError> {
    if let Some(source) = source {
        merge_meta(target, source)?;
    }
    Ok(())
}

fn merge_meta(target: &mut OboMeta, mut source: OboMeta) -> Result<(), ProjectionError> {
    if let Some(definition) = source.definition.take() {
        if target.definition.is_none() {
            target.definition = Some(definition);
        } else {
            target.basic_property_values.push(definition);
        }
    }
    target.comments.append(&mut source.comments);
    target.subsets.append(&mut source.subsets);
    target.synonyms.append(&mut source.synonyms);
    target.xrefs.append(&mut source.xrefs);
    target
        .basic_property_values
        .append(&mut source.basic_property_values);
    match (&target.version, source.version) {
        (None, version) => target.version = version,
        (Some(left), Some(right)) if left != &right => {
            return Err(ProjectionError::integrity(
                "multiple distinct OBO Graphs versions collapse into one metadata object",
            ));
        }
        _ => {}
    }
    target.deprecated |= source.deprecated;
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::loss::{
        LOSS_OBO_ANNOTATION_DROPPED, LOSS_OBO_BLANK_IDENTITY_DROPPED,
        LOSS_OBO_LITERAL_FIDELITY_WIDENED, LOSS_OBO_NAMED_GRAPH_DROPPED,
        LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED, LOSS_OBO_REIFIER_DROPPED,
        LOSS_OBO_TRIPLE_TERM_DROPPED,
    };
    use purrdf_core::{
        BlankScope, PackBuilder, PackView, RdfDataset, RdfDatasetBuilder, RdfLiteral,
        RdfTextDirection, TermId, assert_ledger_complete,
    };

    use super::*;
    use crate::projections::{
        OboGraphsVocabulary, OboMetadataRoles, OboOwlRoles, OboRdfRoles, ProjectionLimits,
    };

    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
    const OWL: &str = "http://www.w3.org/2002/07/owl#";
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    const OBO: &str = "http://www.geneontology.org/formats/oboInOwl#";
    const EX: &str = "http://example.org/";

    fn config(max_records: usize) -> OboGraphsConfig {
        let rdf = OboRdfRoles::new(
            format!("{RDF}type"),
            format!("{RDF}reifies"),
            format!("{RDF}first"),
            format!("{RDF}rest"),
            format!("{RDF}nil"),
            format!("{XSD}string"),
            format!("{XSD}boolean"),
        )
        .expect("RDF roles");
        let owl = OboOwlRoles::new(
            format!("{RDFS}label"),
            format!("{RDFS}comment"),
            format!("{RDFS}subClassOf"),
            format!("{RDFS}subPropertyOf"),
            format!("{RDFS}domain"),
            format!("{RDFS}range"),
            format!("{OWL}Ontology"),
            format!("{OWL}Class"),
            format!("{OWL}NamedIndividual"),
            format!("{OWL}ObjectProperty"),
            format!("{OWL}AnnotationProperty"),
            format!("{OWL}DatatypeProperty"),
            format!("{OWL}equivalentClass"),
            format!("{OWL}intersectionOf"),
            format!("{OWL}Restriction"),
            format!("{OWL}onProperty"),
            format!("{OWL}someValuesFrom"),
            format!("{OWL}allValuesFrom"),
            format!("{OWL}propertyChainAxiom"),
            format!("{OWL}deprecated"),
        )
        .expect("OWL roles");
        let metadata = OboMetadataRoles::new(
            format!("{EX}definition"),
            format!("{OBO}hasExactSynonym"),
            format!("{OBO}hasBroadSynonym"),
            format!("{OBO}hasNarrowSynonym"),
            format!("{OBO}hasRelatedSynonym"),
            format!("{OBO}hasSynonymType"),
            format!("{OBO}hasDbXref"),
            format!("{OBO}inSubset"),
            format!("{OWL}versionInfo"),
        )
        .expect("metadata roles");
        let vocabulary = OboGraphsVocabulary::new(rdf, owl, metadata).expect("vocabulary");
        OboGraphsConfig::new(
            format!("{EX}ontology"),
            vocabulary,
            ProjectionLimits::new(16, 1_000_000, 2_000_000, 3_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("config")
    }

    fn iri(builder: &mut RdfDatasetBuilder, value: &str) -> TermId {
        builder.intern_iri(value)
    }

    fn push_iri_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: Option<TermId>,
    ) -> (TermId, TermId, TermId) {
        let subject = iri(builder, subject);
        let predicate = iri(builder, predicate);
        let object = iri(builder, object);
        builder.push_quad(subject, predicate, object, graph);
        (subject, predicate, object)
    }

    fn push_literal_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        literal: RdfLiteral,
        graph: Option<TermId>,
    ) -> (TermId, TermId, TermId) {
        let subject = iri(builder, subject);
        let predicate = iri(builder, predicate);
        let object = builder.intern_literal(literal);
        builder.push_quad(subject, predicate, object, graph);
        (subject, predicate, object)
    }

    fn fixture(reverse_interning: bool) -> std::sync::Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        if reverse_interning {
            let _ = iri(&mut builder, &format!("{EX}z-unused"));
            let _ = iri(&mut builder, &format!("{EX}a-unused"));
        }
        let rdf_type = format!("{RDF}type");
        let rdf_first = format!("{RDF}first");
        let rdf_rest = format!("{RDF}rest");
        let rdf_nil = format!("{RDF}nil");
        let subclass = format!("{RDFS}subClassOf");
        let label = format!("{RDFS}label");
        let domain = format!("{RDFS}domain");
        let range = format!("{RDFS}range");
        let equivalent = format!("{OWL}equivalentClass");
        let intersection = format!("{OWL}intersectionOf");
        let restriction_type = format!("{OWL}Restriction");
        let on_property = format!("{OWL}onProperty");
        let some_values = format!("{OWL}someValuesFrom");
        let all_values = format!("{OWL}allValuesFrom");
        let chain = format!("{OWL}propertyChainAxiom");
        let graph_id = format!("{EX}ontology");
        let class_a = format!("{EX}A");
        let class_b = format!("{EX}B");
        let class_c = format!("{EX}C");
        let class_d = format!("{EX}D");
        let property_p = format!("{EX}p");
        let property_q = format!("{EX}q");
        let property_r = format!("{EX}r");

        push_iri_quad(
            &mut builder,
            &graph_id,
            &rdf_type,
            &format!("{OWL}Ontology"),
            None,
        );
        for class in [&class_a, &class_b, &class_c, &class_d] {
            push_iri_quad(&mut builder, class, &rdf_type, &format!("{OWL}Class"), None);
        }
        for property in [&property_p, &property_q, &property_r] {
            push_iri_quad(
                &mut builder,
                property,
                &rdf_type,
                &format!("{OWL}ObjectProperty"),
                None,
            );
        }

        push_literal_quad(
            &mut builder,
            &class_a,
            &label,
            RdfLiteral {
                lexical_form: "Alpha".to_owned(),
                datatype: None,
                language: None,
                direction: None,
            },
            None,
        );
        push_literal_quad(
            &mut builder,
            &class_d,
            &label,
            RdfLiteral {
                lexical_form: "دلتا".to_owned(),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            },
            None,
        );
        let definition = push_literal_quad(
            &mut builder,
            &class_a,
            &format!("{EX}definition"),
            RdfLiteral {
                lexical_form: "The alpha class".to_owned(),
                datatype: None,
                language: None,
                direction: None,
            },
            None,
        );
        push_literal_quad(
            &mut builder,
            &class_a,
            &format!("{OBO}hasExactSynonym"),
            RdfLiteral {
                lexical_form: "A class".to_owned(),
                datatype: None,
                language: None,
                direction: None,
            },
            None,
        );
        push_literal_quad(
            &mut builder,
            &class_a,
            &format!("{OBO}hasDbXref"),
            RdfLiteral {
                lexical_form: "DB:1".to_owned(),
                datatype: None,
                language: None,
                direction: None,
            },
            None,
        );
        push_iri_quad(
            &mut builder,
            &class_a,
            &format!("{OBO}inSubset"),
            &format!("{EX}subset"),
            None,
        );
        push_literal_quad(
            &mut builder,
            &class_a,
            &format!("{OWL}deprecated"),
            RdfLiteral {
                lexical_form: "true".to_owned(),
                datatype: Some(format!("{XSD}boolean")),
                language: None,
                direction: None,
            },
            None,
        );

        let simple_edge = push_iri_quad(&mut builder, &class_a, &subclass, &class_b, None);
        push_iri_quad(&mut builder, &class_a, &equivalent, &class_b, None);

        let expression = builder.intern_blank("expression", BlankScope::DEFAULT);
        let list_one = builder.intern_blank("intersection-1", BlankScope::DEFAULT);
        let list_two = builder.intern_blank("intersection-2", BlankScope::DEFAULT);
        let logical_restriction = builder.intern_blank("logical-r", BlankScope::DEFAULT);
        let c = iri(&mut builder, &class_c);
        let equivalent_id = iri(&mut builder, &equivalent);
        builder.push_quad(c, equivalent_id, expression, None);
        let intersection_id = iri(&mut builder, &intersection);
        builder.push_quad(expression, intersection_id, list_one, None);
        let first_id = iri(&mut builder, &rdf_first);
        let rest_id = iri(&mut builder, &rdf_rest);
        let a = iri(&mut builder, &class_a);
        builder.push_quad(list_one, first_id, a, None);
        builder.push_quad(list_one, rest_id, list_two, None);
        builder.push_quad(list_two, first_id, logical_restriction, None);
        let nil = iri(&mut builder, &rdf_nil);
        builder.push_quad(list_two, rest_id, nil, None);
        let rdf_type_id = iri(&mut builder, &rdf_type);
        let restriction_type_id = iri(&mut builder, &restriction_type);
        let on_property_id = iri(&mut builder, &on_property);
        let p = iri(&mut builder, &property_p);
        let some_values_id = iri(&mut builder, &some_values);
        let d = iri(&mut builder, &class_d);
        builder.push_quad(logical_restriction, rdf_type_id, restriction_type_id, None);
        builder.push_quad(logical_restriction, on_property_id, p, None);
        builder.push_quad(logical_restriction, some_values_id, d, None);

        let subclass_restriction = builder.intern_blank("subclass-r", BlankScope::DEFAULT);
        let subclass_id = iri(&mut builder, &subclass);
        builder.push_quad(a, subclass_id, subclass_restriction, None);
        builder.push_quad(subclass_restriction, rdf_type_id, restriction_type_id, None);
        let q = iri(&mut builder, &property_q);
        builder.push_quad(subclass_restriction, on_property_id, q, None);
        builder.push_quad(subclass_restriction, some_values_id, d, None);

        let universal = builder.intern_blank("universal-r", BlankScope::DEFAULT);
        builder.push_quad(a, subclass_id, universal, None);
        builder.push_quad(universal, rdf_type_id, restriction_type_id, None);
        builder.push_quad(universal, on_property_id, p, None);
        let all_values_id = iri(&mut builder, &all_values);
        builder.push_quad(universal, all_values_id, d, None);

        push_iri_quad(&mut builder, &property_p, &domain, &class_a, None);
        push_iri_quad(&mut builder, &property_p, &range, &class_d, None);

        let chain_head = builder.intern_blank("chain-1", BlankScope::DEFAULT);
        let chain_tail = builder.intern_blank("chain-2", BlankScope::DEFAULT);
        let r = iri(&mut builder, &property_r);
        let chain_id = iri(&mut builder, &chain);
        builder.push_quad(r, chain_id, chain_head, None);
        builder.push_quad(chain_head, first_id, p, None);
        builder.push_quad(chain_head, rest_id, chain_tail, None);
        builder.push_quad(chain_tail, first_id, q, None);
        builder.push_quad(chain_tail, rest_id, nil, None);

        let named_graph = iri(&mut builder, &format!("{EX}source-graph"));
        push_iri_quad(
            &mut builder,
            &class_b,
            &format!("{EX}relatedTo"),
            &class_c,
            Some(named_graph),
        );

        let edge_triple = builder.intern_triple(simple_edge.0, simple_edge.1, simple_edge.2);
        let edge_reifier = iri(&mut builder, &format!("{EX}edge-axiom"));
        builder.push_reifier(edge_reifier, edge_triple);
        let confidence = iri(&mut builder, &format!("{EX}confidence"));
        let high = iri(&mut builder, &format!("{EX}high"));
        builder.push_annotation(edge_reifier, confidence, high);
        let unsupported_annotation = builder.intern_blank("annotation-object", BlankScope::DEFAULT);
        builder.push_annotation(edge_reifier, confidence, unsupported_annotation);

        let definition_triple = builder.intern_triple(definition.0, definition.1, definition.2);
        let definition_reifier = iri(&mut builder, &format!("{EX}definition-axiom"));
        builder.push_reifier(definition_reifier, definition_triple);
        let xref = iri(&mut builder, &format!("{OBO}hasDbXref"));
        let citation = builder.intern_literal(RdfLiteral {
            lexical_form: "PMID:1".to_owned(),
            datatype: None,
            language: None,
            direction: None,
        });
        builder.push_annotation(definition_reifier, xref, citation);

        let unsupported = builder.intern_blank("unsupported", BlankScope::DEFAULT);
        let unrelated = iri(&mut builder, &format!("{EX}unrelated"));
        builder.push_quad(unsupported, unrelated, a, None);
        let quoted = builder.intern_triple(a, unrelated, d);
        builder.push_quad(a, unrelated, quoted, None);

        builder.freeze().expect("valid OBO fixture")
    }

    #[test]
    fn projects_complete_basic_advanced_and_metadata_surface_deterministically() {
        let configuration = config(2_000);
        let first = fixture(false);
        let second = fixture(true);
        let projected = project_obo_graphs(first.as_ref(), &configuration).expect("project");
        let projected_again =
            project_obo_graphs(second.as_ref(), &configuration).expect("project reordered");
        assert_eq!(projected.document, projected_again.document);
        assert_eq!(
            projected
                .document
                .to_canonical_json(&configuration)
                .expect("canonical JSON"),
            projected_again
                .document
                .to_canonical_json(&configuration)
                .expect("canonical reordered JSON")
        );
        assert_eq!(
            projected.loss_ledger.render_json(),
            projected_again.loss_ledger.render_json()
        );

        let graph = &projected.document.graphs[0];
        assert_eq!(graph.id, format!("{EX}ontology"));
        assert_eq!(graph.equivalent_nodes_sets.len(), 1);
        assert_eq!(graph.logical_definition_axioms.len(), 1);
        assert_eq!(graph.domain_range_axioms.len(), 1);
        assert_eq!(graph.property_chain_axioms.len(), 1);
        assert!(graph.edges.iter().any(|edge| {
            edge.sub == format!("{EX}A")
                && edge.pred == format!("{RDFS}subClassOf")
                && edge.obj == format!("{EX}B")
                && edge.meta.is_some()
        }));
        let alpha = graph
            .nodes
            .iter()
            .find(|node| node.id == format!("{EX}A"))
            .expect("alpha node");
        assert_eq!(alpha.lbl.as_deref(), Some("Alpha"));
        let meta = alpha.meta.as_ref().expect("alpha metadata");
        assert_eq!(
            meta.definition.as_ref().expect("definition").xrefs,
            vec!["PMID:1"]
        );
        assert!(meta.deprecated);
        assert_eq!(meta.synonyms.len(), 1);
        assert_eq!(meta.xrefs.len(), 1);
        assert_eq!(meta.subsets, vec![format!("{EX}subset")]);

        let bytes = projected
            .document
            .to_canonical_json(&configuration)
            .expect("bytes");
        let json = std::str::from_utf8(&bytes).expect("UTF-8");
        assert!(json.contains(&format!("{RDFS}subClassOf")));
        assert!(!json.contains("\"is_a\""));
        assert!(
            projected
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );
        assert_ledger_complete(
            &projected.loss_ledger,
            &[
                LOSS_OBO_ANNOTATION_DROPPED,
                LOSS_OBO_BLANK_IDENTITY_DROPPED,
                LOSS_OBO_LITERAL_FIDELITY_WIDENED,
                LOSS_OBO_NAMED_GRAPH_DROPPED,
                LOSS_OBO_NON_PROFILE_STATEMENT_DROPPED,
                LOSS_OBO_REIFIER_DROPPED,
                LOSS_OBO_TRIPLE_TERM_DROPPED,
            ],
        );
    }

    #[test]
    fn pack_backend_matches_resident_backend() {
        let dataset = fixture(false);
        let configuration = config(2_000);
        let resident = project_obo_graphs(dataset.as_ref(), &configuration).expect("resident");
        let pack = PackBuilder::build_bytes(&dataset).expect("pack");
        let view = PackView::from_bytes(&pack).expect("view");
        let packed = project_obo_graphs(&view, &configuration).expect("packed");
        assert_eq!(resident.document, packed.document);
        assert_eq!(
            resident.loss_ledger.render_json(),
            packed.loss_ledger.render_json()
        );
    }

    #[test]
    fn explicitly_empty_named_graph_is_not_silently_discarded() {
        let mut builder = RdfDatasetBuilder::new();
        let graph = iri(&mut builder, &format!("{EX}empty-source-graph"));
        builder.declare_named_graph(graph);
        let dataset = builder.freeze().expect("empty named graph dataset");
        let projection = project_obo_graphs(dataset.as_ref(), &config(100)).expect("projection");
        assert!(projection.document.graphs[0].nodes.is_empty());
        assert_ledger_complete(&projection.loss_ledger, &[LOSS_OBO_NAMED_GRAPH_DROPPED]);
        assert_eq!(projection.loss_ledger.entries().len(), 1);
        assert_eq!(
            projection.loss_ledger.entries()[0]
                .location
                .as_ref()
                .and_then(|location| location.logical.as_deref()),
            Some("obo-graphs:named-graph")
        );
    }

    #[test]
    fn rejects_incomplete_vocabulary_and_ambiguous_owl() {
        let configuration = config(2_000);
        let mut value = serde_json::to_value(&configuration).expect("config JSON");
        value["vocabulary"]["rdf"]
            .as_object_mut()
            .expect("RDF roles")
            .remove("rdf_type");
        assert!(serde_json::from_value::<OboGraphsConfig>(value).is_err());

        let duplicate_metadata = OboMetadataRoles::new(
            format!("{RDFS}label"),
            format!("{EX}exact"),
            format!("{EX}broad"),
            format!("{EX}narrow"),
            format!("{EX}related"),
            format!("{EX}synonym-type"),
            format!("{EX}xref"),
            format!("{EX}subset"),
            format!("{EX}version"),
        )
        .expect("locally valid metadata");
        assert!(
            OboGraphsVocabulary::new(
                configuration.vocabulary().rdf().clone(),
                configuration.vocabulary().owl().clone(),
                duplicate_metadata,
            )
            .is_err()
        );

        let mut builder = RdfDatasetBuilder::new();
        let subject = iri(&mut builder, &format!("{EX}A"));
        let subclass = iri(&mut builder, &format!("{RDFS}subClassOf"));
        let restriction = builder.intern_blank("ambiguous", BlankScope::DEFAULT);
        builder.push_quad(subject, subclass, restriction, None);
        let rdf_type = iri(&mut builder, &format!("{RDF}type"));
        let restriction_type = iri(&mut builder, &format!("{OWL}Restriction"));
        builder.push_quad(restriction, rdf_type, restriction_type, None);
        let on_property = iri(&mut builder, &format!("{OWL}onProperty"));
        let first_property = iri(&mut builder, &format!("{EX}p"));
        let second_property = iri(&mut builder, &format!("{EX}q"));
        builder.push_quad(restriction, on_property, first_property, None);
        builder.push_quad(restriction, on_property, second_property, None);
        let some_values = iri(&mut builder, &format!("{OWL}someValuesFrom"));
        let filler = iri(&mut builder, &format!("{EX}B"));
        builder.push_quad(restriction, some_values, filler, None);
        let dataset = builder.freeze().expect("structurally valid ambiguous OWL");
        let error = project_obo_graphs(dataset.as_ref(), &configuration).expect_err("ambiguous");
        assert_eq!(
            error.kind(),
            super::super::super::ProjectionErrorKind::Integrity
        );
        assert!(error.message().contains("ambiguous restriction property"));
    }

    #[test]
    fn enforces_total_record_and_artifact_bounds() {
        let dataset = fixture(false);
        assert!(project_obo_graphs(dataset.as_ref(), &config(1)).is_err());
        let tiny = OboGraphsConfig::new(
            format!("{EX}ontology"),
            config(2_000).vocabulary().clone(),
            ProjectionLimits::new(2, 64, 64, 1_536, 16).expect("tiny limits"),
            2_000,
        )
        .expect("tiny config");
        assert!(project_obo_graphs(dataset.as_ref(), &tiny).is_err());
    }
}

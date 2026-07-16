// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::loss::{
    LOSS_RESEARCH_ANNOTATION_DROPPED, LOSS_RESEARCH_BLANK_IDENTITY_RESOLVED,
    LOSS_RESEARCH_EMPTY_GRAPH_DROPPED, LOSS_RESEARCH_NAMED_GRAPH_DROPPED,
    LOSS_RESEARCH_REIFIER_DROPPED, LOSS_RESEARCH_TRIPLE_TERM_DROPPED,
    LOSS_RESEARCH_UNMAPPED_STATEMENT_DROPPED,
};
use purrdf_core::{
    DatasetView, LossEntry, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfLocation,
    check_ledger_sound, rdf_to_research_object_loss_ledger,
};

use super::super::{ProjectionError, ProjectionTerm, stable_identifier};
use super::{
    ResearchActivity, ResearchAgent, ResearchChecksum, ResearchDataset, ResearchField,
    ResearchObjectConfig, ResearchObjectModel, ResearchRecordSet, ResearchResource, ResearchRole,
    ResearchText, ResearchValue,
};

/// Common semantic projection and its always-computed runtime losses.
#[derive(Debug, Clone)]
pub struct ResearchObjectProjection {
    /// Validated, normalized format-neutral research-object model.
    pub model: ResearchObjectModel,
    /// Located runtime losses incurred interpreting the source RDF view.
    pub loss_ledger: LossLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceQuad {
    subject: ProjectionTerm,
    predicate: String,
    object: ProjectionTerm,
    graph: Option<ProjectionTerm>,
}

struct Projector<'a> {
    config: &'a ResearchObjectConfig,
    target_profile: String,
    quads: Vec<SourceQuad>,
    consumed: Vec<bool>,
    named_graphs: Vec<ProjectionTerm>,
    reifiers: Vec<(ProjectionTerm, ProjectionTerm, Option<ProjectionTerm>)>,
    annotations: Vec<(
        ProjectionTerm,
        String,
        ProjectionTerm,
        Option<ProjectionTerm>,
    )>,
    ledger: LossLedger,
    contract: LossLedger,
}

/// Interpret one RDF 1.2 dataset view as the shared research-object model.
///
/// `target_profile` is a closed versioned name from
/// [`purrdf_core::RESEARCH_OBJECT_CODECS`]. It selects the loss contract only;
/// native format encoding happens in the profile adapter.
///
/// # Errors
///
/// Returns a typed configuration, term, integrity, or resource-limit failure
/// when caller roles are ambiguous, required data is absent, the input view is
/// invalid, or a mandatory bound is exceeded.
pub fn project_research_object<D: DatasetView>(
    view: &D,
    target_profile: &str,
    config: &ResearchObjectConfig,
) -> Result<ResearchObjectProjection, ProjectionError> {
    Projector::load(view, target_profile, config)?.project()
}

impl<'a> Projector<'a> {
    fn load<D: DatasetView>(
        view: &D,
        target_profile: &str,
        config: &'a ResearchObjectConfig,
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

        let mut named_graphs = Vec::new();
        for graph in view.named_graphs() {
            named_graphs.push(resolve_term(view, graph, config, &mut cache)?);
        }
        named_graphs.sort();
        reject_duplicates(&named_graphs, "named graph declarations")?;

        let mut reifiers = Vec::new();
        for row in view.reifier_quads() {
            reifiers.push((
                resolve_term(view, row.s, config, &mut cache)?,
                resolve_term(view, row.o, config, &mut cache)?,
                row.g
                    .map(|id| resolve_term(view, id, config, &mut cache))
                    .transpose()?,
            ));
        }
        reifiers.sort();
        reject_duplicates(&reifiers, "RDF reifier bindings")?;

        let mut annotations = Vec::new();
        for row in view.annotation_quads() {
            let ProjectionTerm::Iri { value: predicate } =
                resolve_term(view, row.p, config, &mut cache)?
            else {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a non-IRI annotation predicate",
                ));
            };
            annotations.push((
                resolve_term(view, row.s, config, &mut cache)?,
                predicate,
                resolve_term(view, row.o, config, &mut cache)?,
                row.g
                    .map(|id| resolve_term(view, id, config, &mut cache))
                    .transpose()?,
            ));
        }
        annotations.sort();
        reject_duplicates(&annotations, "RDF annotations")?;

        let count = quads
            .len()
            .checked_add(named_graphs.len())
            .and_then(|value| value.checked_add(reifiers.len()))
            .and_then(|value| value.checked_add(annotations.len()))
            .ok_or_else(|| ProjectionError::limit("research-object input record count overflow"))?;
        if count > config.policy().max_records() {
            return Err(ProjectionError::limit(format!(
                "research-object input has {count} records; limit is {}",
                config.policy().max_records()
            )));
        }

        let consumed = vec![false; quads.len()];
        let contract = rdf_to_research_object_loss_ledger(target_profile);
        let mut projector = Self {
            config,
            target_profile: target_profile.to_owned(),
            quads,
            consumed,
            named_graphs,
            reifiers,
            annotations,
            ledger: LossLedger::new(),
            contract,
        };
        projector.record_structural_losses();
        Ok(projector)
    }

    fn project(mut self) -> Result<ResearchObjectProjection, ProjectionError> {
        let root = iri(self.config.identity().dataset_iri());
        self.require_type(&root, ResearchRole::DatasetClass, "dataset")?;

        let creators = self.take_entity_ids(&root, ResearchRole::Creator)?;
        let publishers = self.take_entity_ids(&root, ResearchRole::Publisher)?;
        let resource_nodes = self.take_entity_nodes(&root, ResearchRole::HasResource)?;
        let activity_nodes = self.take_entity_nodes(&root, ResearchRole::HasActivity)?;
        let record_set_nodes = self.take_entity_nodes(&root, ResearchRole::HasRecordSet)?;

        let dataset = ResearchDataset {
            id: self.config.identity().dataset_iri().to_owned(),
            titles: self.take_texts(&root, ResearchRole::Title)?,
            descriptions: self.take_texts(&root, ResearchRole::Description)?,
            identifiers: self.take_values(&root, ResearchRole::Identifier)?,
            versions: self.take_texts(&root, ResearchRole::Version)?,
            issued: self.take_texts(&root, ResearchRole::Issued)?,
            modified: self.take_texts(&root, ResearchRole::Modified)?,
            landing_pages: self.take_values(&root, ResearchRole::LandingPage)?,
            keywords: self.take_texts(&root, ResearchRole::Keyword)?,
            licenses: self.take_values(&root, ResearchRole::License)?,
            creators: creators.iter().map(|(_, id)| id.clone()).collect(),
            publishers: publishers.iter().map(|(_, id)| id.clone()).collect(),
            resources: resource_nodes.iter().map(|(_, id)| id.clone()).collect(),
            activities: activity_nodes.iter().map(|(_, id)| id.clone()).collect(),
            record_sets: record_set_nodes.iter().map(|(_, id)| id.clone()).collect(),
        };

        let mut agent_nodes: BTreeMap<ProjectionTerm, String> =
            creators.into_iter().chain(publishers).collect();
        let mut activities = Vec::new();
        for (node, id) in &activity_nodes {
            let (activity, actors) = self.extract_activity(node, id)?;
            agent_nodes.extend(actors);
            activities.push(activity);
        }

        let mut agents = Vec::new();
        for (node, id) in agent_nodes {
            self.require_type(&node, ResearchRole::AgentClass, "agent")?;
            agents.push(ResearchAgent {
                id,
                names: self.take_texts(&node, ResearchRole::AgentName)?,
            });
        }

        let mut resources = Vec::new();
        for (node, id) in resource_nodes {
            resources.push(self.extract_resource(&node, id)?);
        }

        let mut record_sets = Vec::new();
        for (node, id) in record_set_nodes {
            record_sets.push(self.extract_record_set(&node, id)?);
        }

        self.record_unmapped_losses();
        check_ledger_sound(&self.ledger, "rdf-1.2-dataset", &self.target_profile)
            .map_err(ProjectionError::integrity)?;

        let model = ResearchObjectModel {
            dataset,
            agents,
            resources,
            activities,
            record_sets,
        }
        .normalize(self.config.policy())?;

        Ok(ResearchObjectProjection {
            model,
            loss_ledger: self.ledger,
        })
    }

    fn extract_resource(
        &mut self,
        node: &ProjectionTerm,
        id: String,
    ) -> Result<ResearchResource, ProjectionError> {
        self.require_type(node, ResearchRole::ResourceClass, "resource")?;
        let byte_sizes = self.take_texts(node, ResearchRole::ByteSize)?;
        let byte_size = match byte_sizes.as_slice() {
            [] => None,
            [value] => Some(value.value.parse::<u64>().map_err(|error| {
                ProjectionError::integrity(format!(
                    "resource `{id}` byte size `{}` is invalid: {error}",
                    value.value
                ))
            })?),
            _ => {
                return Err(ProjectionError::integrity(format!(
                    "resource `{id}` has conflicting byte-size values"
                )));
            }
        };

        let checksum_nodes = self.take_entity_nodes(node, ResearchRole::Checksum)?;
        let mut checksums = Vec::new();
        for (checksum_node, checksum_id) in checksum_nodes {
            self.require_type(&checksum_node, ResearchRole::ChecksumClass, "checksum")?;
            let algorithms = self.take_values(&checksum_node, ResearchRole::ChecksumAlgorithm)?;
            let values = self.take_texts(&checksum_node, ResearchRole::ChecksumValue)?;
            let (algorithm, value) = match (algorithms.as_slice(), values.as_slice()) {
                ([algorithm], [value]) => (algorithm.clone(), value.clone()),
                _ => {
                    return Err(ProjectionError::integrity(format!(
                        "checksum `{checksum_id}` requires exactly one algorithm and value"
                    )));
                }
            };
            checksums.push(ResearchChecksum { algorithm, value });
        }

        Ok(ResearchResource {
            id,
            names: self.take_texts(node, ResearchRole::ResourceName)?,
            descriptions: self.take_texts(node, ResearchRole::ResourceDescription)?,
            paths: self
                .take_texts(node, ResearchRole::ResourcePath)?
                .into_iter()
                .map(|value| value.value)
                .collect(),
            urls: self.take_values(node, ResearchRole::ResourceUrl)?,
            media_types: self.take_texts(node, ResearchRole::MediaType)?,
            formats: self.take_values(node, ResearchRole::Format)?,
            byte_size,
            checksums,
        })
    }

    fn extract_activity(
        &mut self,
        node: &ProjectionTerm,
        id: &str,
    ) -> Result<(ResearchActivity, BTreeMap<ProjectionTerm, String>), ProjectionError> {
        self.require_type(node, ResearchRole::ActivityClass, "activity")?;
        let actors = self.take_entity_nodes(node, ResearchRole::Actor)?;
        let actor_map: BTreeMap<ProjectionTerm, String> = actors.iter().cloned().collect();
        Ok((
            ResearchActivity {
                id: id.to_owned(),
                names: self.take_texts(node, ResearchRole::ActivityName)?,
                instruments: self.take_values(node, ResearchRole::Instrument)?,
                actors: actors.into_iter().map(|(_, id)| id).collect(),
                objects: self
                    .take_entity_nodes(node, ResearchRole::Object)?
                    .into_iter()
                    .map(|(_, id)| id)
                    .collect(),
                results: self
                    .take_entity_nodes(node, ResearchRole::Result)?
                    .into_iter()
                    .map(|(_, id)| id)
                    .collect(),
                end_times: self.take_texts(node, ResearchRole::EndTime)?,
                workflows: self.take_values(node, ResearchRole::Workflow)?,
            },
            actor_map,
        ))
    }

    fn extract_record_set(
        &mut self,
        node: &ProjectionTerm,
        id: String,
    ) -> Result<ResearchRecordSet, ProjectionError> {
        self.require_type(node, ResearchRole::RecordSetClass, "record set")?;
        let field_nodes = self.take_entity_nodes(node, ResearchRole::HasField)?;
        let mut fields = Vec::new();
        for (field_node, field_id) in field_nodes {
            self.require_type(&field_node, ResearchRole::FieldClass, "field")?;
            fields.push(ResearchField {
                id: field_id,
                names: self.take_texts(&field_node, ResearchRole::FieldName)?,
                data_types: self.take_values(&field_node, ResearchRole::FieldDataType)?,
            });
        }
        let row_terms = self.take_terms(node, ResearchRole::HasRow);
        let mut rows = Vec::new();
        for term in row_terms {
            let ProjectionTerm::Literal {
                lexical, datatype, ..
            } = term
            else {
                return Err(ProjectionError::integrity(format!(
                    "record set `{id}` row is not a JSON literal"
                )));
            };
            if datatype != self.config.roles().iri(ResearchRole::JsonDatatype) {
                return Err(ProjectionError::integrity(format!(
                    "record set `{id}` row datatype `{datatype}` does not match caller JSON datatype"
                )));
            }
            rows.push(serde_json::from_str(&lexical).map_err(|error| {
                ProjectionError::syntax(format!(
                    "parse inline row JSON for record set `{id}`: {error}"
                ))
            })?);
        }
        Ok(ResearchRecordSet {
            id,
            names: self.take_texts(node, ResearchRole::RecordSetName)?,
            descriptions: self.take_texts(node, ResearchRole::RecordSetDescription)?,
            fields,
            rows,
        })
    }

    fn require_type(
        &mut self,
        subject: &ProjectionTerm,
        class_role: ResearchRole,
        description: &str,
    ) -> Result<(), ProjectionError> {
        let predicate = self.config.roles().iri(ResearchRole::RdfType);
        let class = self.config.roles().iri(class_role);
        let mut found = false;
        for index in 0..self.quads.len() {
            let quad = &self.quads[index];
            if &quad.subject == subject && quad.predicate == predicate && quad.object == iri(class)
            {
                self.consumed[index] = true;
                found = true;
            }
        }
        if !found {
            return Err(ProjectionError::integrity(format!(
                "configured research-object {description} `{}` lacks required type `{class}`",
                term_label(subject)
            )));
        }
        Ok(())
    }

    fn take_terms(&mut self, subject: &ProjectionTerm, role: ResearchRole) -> Vec<ProjectionTerm> {
        let predicate = self.config.roles().iri(role);
        let mut values = Vec::new();
        for index in 0..self.quads.len() {
            let quad = &self.quads[index];
            if &quad.subject == subject && quad.predicate == predicate {
                self.consumed[index] = true;
                values.push(quad.object.clone());
            }
        }
        values.sort();
        values.dedup();
        values
    }

    fn take_texts(
        &mut self,
        subject: &ProjectionTerm,
        role: ResearchRole,
    ) -> Result<Vec<ResearchText>, ProjectionError> {
        self.take_terms(subject, role)
            .into_iter()
            .map(|term| match term {
                ProjectionTerm::Literal {
                    lexical,
                    datatype,
                    language,
                    direction,
                } => ResearchText::new(lexical, datatype, language, direction),
                other => Err(ProjectionError::integrity(format!(
                    "research-object role `{role:?}` requires a literal, got `{}`",
                    term_label(&other)
                ))),
            })
            .collect()
    }

    fn take_values(
        &mut self,
        subject: &ProjectionTerm,
        role: ResearchRole,
    ) -> Result<Vec<ResearchValue>, ProjectionError> {
        let mut values = Vec::new();
        for term in self.take_terms(subject, role) {
            if let Some(value) = self.value_from_term(&term)? {
                values.push(value);
            }
        }
        values.sort();
        values.dedup();
        Ok(values)
    }

    fn take_entity_nodes(
        &mut self,
        subject: &ProjectionTerm,
        role: ResearchRole,
    ) -> Result<Vec<(ProjectionTerm, String)>, ProjectionError> {
        let mut values = Vec::new();
        for term in self.take_terms(subject, role) {
            if let Some(id) = self.entity_id(&term)? {
                values.push((term, id));
            }
        }
        values.sort_by(|left, right| left.1.cmp(&right.1));
        values.dedup_by(|left, right| left.1 == right.1);
        Ok(values)
    }

    fn take_entity_ids(
        &mut self,
        subject: &ProjectionTerm,
        role: ResearchRole,
    ) -> Result<Vec<(ProjectionTerm, String)>, ProjectionError> {
        self.take_entity_nodes(subject, role)
    }

    fn value_from_term(
        &mut self,
        term: &ProjectionTerm,
    ) -> Result<Option<ResearchValue>, ProjectionError> {
        Ok(match term {
            ProjectionTerm::Iri { value } => Some(ResearchValue::iri(value.clone())?),
            ProjectionTerm::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => Some(ResearchValue::Text(ResearchText::new(
                lexical.clone(),
                datatype.clone(),
                language.clone(),
                *direction,
            )?)),
            ProjectionTerm::Blank { .. } => self
                .entity_id(term)?
                .map(|value| ResearchValue::Iri { value }),
            ProjectionTerm::Triple { .. } => {
                self.record_loss(
                    LOSS_RESEARCH_TRIPLE_TERM_DROPPED,
                    "research-object:rdf-term",
                    &term_label(term),
                );
                None
            }
        })
    }

    fn entity_id(&mut self, term: &ProjectionTerm) -> Result<Option<String>, ProjectionError> {
        match term {
            ProjectionTerm::Iri { value } => Ok(Some(value.clone())),
            ProjectionTerm::Blank { .. } => {
                let key = term.to_canonical_json(self.config.limits())?;
                let local = stable_identifier("entity", &key)?;
                let iri = self.config.identity().resolve_relative(&local)?;
                self.record_loss(
                    LOSS_RESEARCH_BLANK_IDENTITY_RESOLVED,
                    "research-object:rdf-identity",
                    &term_label(term),
                );
                Ok(Some(iri))
            }
            ProjectionTerm::Triple { .. } => {
                self.record_loss(
                    LOSS_RESEARCH_TRIPLE_TERM_DROPPED,
                    "research-object:rdf-identity",
                    &term_label(term),
                );
                Ok(None)
            }
            ProjectionTerm::Literal { .. } => Err(ProjectionError::integrity(format!(
                "research-object entity relation points to literal `{}`",
                term_label(term)
            ))),
        }
    }

    fn record_structural_losses(&mut self) {
        let populated: BTreeSet<&ProjectionTerm> = self
            .quads
            .iter()
            .filter_map(|quad| quad.graph.as_ref())
            .collect();
        let empty_graphs: Vec<String> = self
            .named_graphs
            .iter()
            .filter(|graph| !populated.contains(graph))
            .map(term_label)
            .collect();
        for graph in empty_graphs {
            self.record_loss(
                LOSS_RESEARCH_EMPTY_GRAPH_DROPPED,
                "research-object:named-graph",
                &graph,
            );
        }
        let named_quads: Vec<(String, String)> = self
            .quads
            .iter()
            .filter_map(|quad| {
                quad.graph
                    .as_ref()
                    .map(|graph| (term_label(graph), term_label(&quad.subject)))
            })
            .collect();
        for (graph, subject) in named_quads {
            self.record_loss(
                LOSS_RESEARCH_NAMED_GRAPH_DROPPED,
                &format!("research-object:named-graph:{graph}"),
                &subject,
            );
        }
        let reifiers: Vec<String> = self
            .reifiers
            .iter()
            .map(|(reifier, _, _)| term_label(reifier))
            .collect();
        for reifier in reifiers {
            self.record_loss(
                LOSS_RESEARCH_REIFIER_DROPPED,
                "research-object:reifier",
                &reifier,
            );
        }
        let annotations: Vec<String> = self
            .annotations
            .iter()
            .map(|(reifier, predicate, _, _)| format!("{} {predicate}", term_label(reifier)))
            .collect();
        for annotation in annotations {
            self.record_loss(
                LOSS_RESEARCH_ANNOTATION_DROPPED,
                "research-object:annotation",
                &annotation,
            );
        }
    }

    fn record_unmapped_losses(&mut self) {
        let rows: Vec<SourceQuad> = self
            .quads
            .iter()
            .zip(&self.consumed)
            .filter(|(_, consumed)| !**consumed)
            .map(|(quad, _)| quad.clone())
            .collect();
        for quad in rows {
            if contains_triple(&quad.subject)
                || contains_triple(&quad.object)
                || quad.graph.as_ref().is_some_and(contains_triple)
            {
                self.record_loss(
                    LOSS_RESEARCH_TRIPLE_TERM_DROPPED,
                    "research-object:unmapped-rdf",
                    &term_label(&quad.subject),
                );
            }
            self.record_loss(
                LOSS_RESEARCH_UNMAPPED_STATEMENT_DROPPED,
                "research-object:unmapped-rdf",
                &format!("{} {}", term_label(&quad.subject), quad.predicate),
            );
        }
    }

    fn record_loss(&mut self, code: &'static str, logical: &str, subject: &str) {
        let template = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .expect("runtime research-object code must exist in closed contract");
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

/// Lift one normalized research-object model into caller-vocabulary RDF 1.2.
///
/// This semantic step is lossless: native-reader losses are computed before the
/// model reaches this function. All emitted vocabulary and data bases come from
/// `config`.
///
/// # Errors
///
/// Returns a typed model, term, or dataset-integrity failure.
pub fn lift_research_object(
    model: ResearchObjectModel,
    config: &ResearchObjectConfig,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let model = model.normalize(config.policy())?;
    let roles = config.roles();
    let mut builder = RdfDatasetBuilder::new();
    push_type(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::RdfType),
        roles.iri(ResearchRole::DatasetClass),
    );
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Title),
        &model.dataset.titles,
    )?;
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Description),
        &model.dataset.descriptions,
    )?;
    push_values(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Identifier),
        &model.dataset.identifiers,
    )?;
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Version),
        &model.dataset.versions,
    )?;
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Issued),
        &model.dataset.issued,
    )?;
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Modified),
        &model.dataset.modified,
    )?;
    push_values(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::LandingPage),
        &model.dataset.landing_pages,
    )?;
    push_texts(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Keyword),
        &model.dataset.keywords,
    )?;
    push_values(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::License),
        &model.dataset.licenses,
    )?;
    push_relations(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Creator),
        &model.dataset.creators,
    );
    push_relations(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::Publisher),
        &model.dataset.publishers,
    );
    push_relations(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::HasResource),
        &model.dataset.resources,
    );
    push_relations(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::HasActivity),
        &model.dataset.activities,
    );
    push_relations(
        &mut builder,
        &model.dataset.id,
        roles.iri(ResearchRole::HasRecordSet),
        &model.dataset.record_sets,
    );

    for agent in &model.agents {
        push_type(
            &mut builder,
            &agent.id,
            roles.iri(ResearchRole::RdfType),
            roles.iri(ResearchRole::AgentClass),
        );
        push_texts(
            &mut builder,
            &agent.id,
            roles.iri(ResearchRole::AgentName),
            &agent.names,
        )?;
    }
    for resource in &model.resources {
        push_type(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::RdfType),
            roles.iri(ResearchRole::ResourceClass),
        );
        push_texts(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::ResourceName),
            &resource.names,
        )?;
        push_texts(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::ResourceDescription),
            &resource.descriptions,
        )?;
        let paths: Vec<ResearchText> = resource
            .paths
            .iter()
            .map(|path| ResearchText::plain(path, roles.iri(ResearchRole::XsdString)))
            .collect::<Result<_, _>>()?;
        push_texts(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::ResourcePath),
            &paths,
        )?;
        push_values(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::ResourceUrl),
            &resource.urls,
        )?;
        push_texts(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::MediaType),
            &resource.media_types,
        )?;
        push_values(
            &mut builder,
            &resource.id,
            roles.iri(ResearchRole::Format),
            &resource.formats,
        )?;
        if let Some(byte_size) = resource.byte_size {
            let value = ResearchText::new(
                byte_size.to_string(),
                roles.iri(ResearchRole::XsdNonNegativeInteger),
                None,
                None,
            )?;
            push_texts(
                &mut builder,
                &resource.id,
                roles.iri(ResearchRole::ByteSize),
                &[value],
            )?;
        }
        for checksum in &resource.checksums {
            let key = serde_json::to_vec(&(resource.id.as_str(), checksum)).map_err(|error| {
                ProjectionError::integrity(format!("serialize checksum identity: {error}"))
            })?;
            let local = stable_identifier("checksum", &key)?;
            let checksum_id = config.identity().resolve_relative(&local)?;
            push_relation(
                &mut builder,
                &resource.id,
                roles.iri(ResearchRole::Checksum),
                &checksum_id,
            );
            push_type(
                &mut builder,
                &checksum_id,
                roles.iri(ResearchRole::RdfType),
                roles.iri(ResearchRole::ChecksumClass),
            );
            push_values(
                &mut builder,
                &checksum_id,
                roles.iri(ResearchRole::ChecksumAlgorithm),
                std::slice::from_ref(&checksum.algorithm),
            )?;
            push_texts(
                &mut builder,
                &checksum_id,
                roles.iri(ResearchRole::ChecksumValue),
                std::slice::from_ref(&checksum.value),
            )?;
        }
    }
    for activity in &model.activities {
        push_type(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::RdfType),
            roles.iri(ResearchRole::ActivityClass),
        );
        push_texts(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::ActivityName),
            &activity.names,
        )?;
        push_values(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::Instrument),
            &activity.instruments,
        )?;
        push_relations(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::Actor),
            &activity.actors,
        );
        push_relations(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::Object),
            &activity.objects,
        );
        push_relations(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::Result),
            &activity.results,
        );
        push_texts(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::EndTime),
            &activity.end_times,
        )?;
        push_values(
            &mut builder,
            &activity.id,
            roles.iri(ResearchRole::Workflow),
            &activity.workflows,
        )?;
    }
    for record_set in &model.record_sets {
        push_type(
            &mut builder,
            &record_set.id,
            roles.iri(ResearchRole::RdfType),
            roles.iri(ResearchRole::RecordSetClass),
        );
        push_texts(
            &mut builder,
            &record_set.id,
            roles.iri(ResearchRole::RecordSetName),
            &record_set.names,
        )?;
        push_texts(
            &mut builder,
            &record_set.id,
            roles.iri(ResearchRole::RecordSetDescription),
            &record_set.descriptions,
        )?;
        for field in &record_set.fields {
            push_relation(
                &mut builder,
                &record_set.id,
                roles.iri(ResearchRole::HasField),
                &field.id,
            );
            push_type(
                &mut builder,
                &field.id,
                roles.iri(ResearchRole::RdfType),
                roles.iri(ResearchRole::FieldClass),
            );
            push_texts(
                &mut builder,
                &field.id,
                roles.iri(ResearchRole::FieldName),
                &field.names,
            )?;
            push_values(
                &mut builder,
                &field.id,
                roles.iri(ResearchRole::FieldDataType),
                &field.data_types,
            )?;
        }
        for row in &record_set.rows {
            let lexical = serde_json::to_string(row).map_err(|error| {
                ProjectionError::syntax(format!("serialize inline row JSON: {error}"))
            })?;
            let value =
                ResearchText::new(lexical, roles.iri(ResearchRole::JsonDatatype), None, None)?;
            push_texts(
                &mut builder,
                &record_set.id,
                roles.iri(ResearchRole::HasRow),
                &[value],
            )?;
        }
    }

    builder
        .freeze()
        .map_err(|error| ProjectionError::integrity(format!("freeze research-object RDF: {error}")))
}

fn push_type(builder: &mut RdfDatasetBuilder, subject: &str, predicate: &str, class: &str) {
    push_relation(builder, subject, predicate, class);
}

fn push_relations(
    builder: &mut RdfDatasetBuilder,
    subject: &str,
    predicate: &str,
    values: &[String],
) {
    for value in values {
        push_relation(builder, subject, predicate, value);
    }
}

fn push_relation(builder: &mut RdfDatasetBuilder, subject: &str, predicate: &str, object: &str) {
    let subject = builder.intern_iri(subject);
    let predicate = builder.intern_iri(predicate);
    let object = builder.intern_iri(object);
    builder.push_quad(subject, predicate, object, None);
}

fn push_values(
    builder: &mut RdfDatasetBuilder,
    subject: &str,
    predicate: &str,
    values: &[ResearchValue],
) -> Result<(), ProjectionError> {
    for value in values {
        match value {
            ResearchValue::Iri { value } => push_relation(builder, subject, predicate, value),
            ResearchValue::Text(value) => {
                push_texts(builder, subject, predicate, std::slice::from_ref(value))?;
            }
        }
    }
    Ok(())
}

fn push_texts(
    builder: &mut RdfDatasetBuilder,
    subject: &str,
    predicate: &str,
    values: &[ResearchText],
) -> Result<(), ProjectionError> {
    for value in values {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_literal(RdfLiteral {
            lexical_form: value.value.clone(),
            datatype: Some(value.datatype.clone()),
            language: value.language.clone(),
            direction: value.direction.map(Into::into),
        });
        builder.push_quad(subject, predicate, object, None);
    }
    Ok(())
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    config: &ResearchObjectConfig,
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

fn reject_duplicates<T: PartialEq>(values: &[T], description: &str) -> Result<(), ProjectionError> {
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProjectionError::integrity(format!(
            "dataset view exposed duplicate {description}"
        )));
    }
    Ok(())
}

fn iri(value: &str) -> ProjectionTerm {
    ProjectionTerm::Iri {
        value: value.to_owned(),
    }
}

fn contains_triple(term: &ProjectionTerm) -> bool {
    match term {
        ProjectionTerm::Triple { .. } => true,
        ProjectionTerm::Iri { .. }
        | ProjectionTerm::Blank { .. }
        | ProjectionTerm::Literal { .. } => false,
    }
}

fn term_label(term: &ProjectionTerm) -> String {
    match term {
        ProjectionTerm::Iri { value } => value.clone(),
        ProjectionTerm::Blank { label, scope } => format!("_:{scope}:{label}"),
        ProjectionTerm::Literal { lexical, .. } => format!("literal:{lexical}"),
        ProjectionTerm::Triple { .. } => {
            serde_json::to_string(term).unwrap_or_else(|_| "triple-term".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projections::{
        ProjectionLimits, RESEARCH_ROLES, ResearchObjectIdentity, ResearchObjectPolicy,
        ResearchObjectRoles,
    };

    const EX: &str = "https://example.org/roles/";

    fn config() -> ResearchObjectConfig {
        let roles = RESEARCH_ROLES
            .iter()
            .copied()
            .enumerate()
            .map(|(index, role)| (role, format!("{EX}{index}")))
            .collect();
        ResearchObjectConfig::new(
            ResearchObjectRoles::new(roles).expect("roles"),
            ResearchObjectIdentity::new(
                "https://example.org/dataset",
                "https://example.org/entities/",
            )
            .expect("identity"),
            ResearchObjectPolicy::new(
                ProjectionLimits::new(32, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits"),
                1_000,
                500,
                1_000,
                16,
            )
            .expect("policy"),
        )
    }

    fn push_iri(builder: &mut RdfDatasetBuilder, subject: &str, predicate: &str, object: &str) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_iri(object);
        builder.push_quad(subject, predicate, object, None);
    }

    fn push_text(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        value: &str,
        datatype: &str,
    ) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_literal(RdfLiteral::typed(value, datatype));
        builder.push_quad(subject, predicate, object, None);
    }

    #[test]
    fn common_model_round_trips_and_every_unmapped_statement_is_ledgered() {
        let config = config();
        let roles = config.roles();
        let mut builder = RdfDatasetBuilder::new();
        push_iri(
            &mut builder,
            config.identity().dataset_iri(),
            roles.iri(ResearchRole::RdfType),
            roles.iri(ResearchRole::DatasetClass),
        );
        push_text(
            &mut builder,
            config.identity().dataset_iri(),
            roles.iri(ResearchRole::Title),
            "Dataset",
            roles.iri(ResearchRole::XsdString),
        );
        push_iri(
            &mut builder,
            config.identity().dataset_iri(),
            "https://example.org/unmapped",
            "https://example.org/value",
        );
        let dataset = builder.freeze().expect("dataset");

        let first =
            project_research_object(dataset.as_ref(), "croissant-1.1", &config).expect("project");
        assert_eq!(first.model.dataset.titles[0].value, "Dataset");
        assert!(first.loss_ledger.entries().iter().any(|entry| {
            entry.code == LOSS_RESEARCH_UNMAPPED_STATEMENT_DROPPED && entry.location.is_some()
        }));

        let lifted = lift_research_object(first.model.clone(), &config).expect("lift");
        let second =
            project_research_object(lifted.as_ref(), "croissant-1.1", &config).expect("reproject");
        assert_eq!(second.model, first.model);
        assert!(second.loss_ledger.is_empty());
    }

    #[test]
    fn model_lift_uses_only_caller_roles() {
        let config = config();
        let text = ResearchText::plain("Dataset", config.roles().iri(ResearchRole::XsdString))
            .expect("text");
        let model = ResearchObjectModel {
            dataset: ResearchDataset {
                id: config.identity().dataset_iri().to_owned(),
                titles: vec![text],
                descriptions: vec![],
                identifiers: vec![],
                versions: vec![],
                issued: vec![],
                modified: vec![],
                landing_pages: vec![],
                keywords: vec![],
                licenses: vec![],
                creators: vec![],
                publishers: vec![],
                resources: vec![],
                activities: vec![],
                record_sets: vec![],
            },
            agents: vec![],
            resources: vec![],
            activities: vec![],
            record_sets: vec![],
        };
        let lifted = lift_research_object(model, &config).expect("lift");
        for quad in lifted.quads() {
            let predicate = lifted.resolve(quad.p);
            let purrdf_core::TermRef::Iri(predicate) = predicate else {
                panic!("predicate must be IRI");
            };
            assert!(predicate.starts_with(EX));
        }
    }
}

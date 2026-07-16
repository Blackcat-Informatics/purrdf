// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::loss::{
    LOSS_SKOS_ANNOTATION_DROPPED, LOSS_SKOS_BLANK_IDENTITY_DROPPED, LOSS_SKOS_NAMED_GRAPH_DROPPED,
    LOSS_SKOS_NON_PROFILE_STATEMENT_DROPPED, LOSS_SKOS_REIFIER_DROPPED,
    LOSS_SKOS_TRIPLE_TERM_DROPPED,
};
use purrdf_core::{
    BlankScope, DatasetView, LossEntry, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfLocation, TermId, check_ledger_sound, rdf_to_skos_loss_ledger,
};
use serde::Serialize;

use crate::native_codecs::{NativeRdfFormat, serialize_dataset_to_format};

use super::super::{ProjectionError, ProjectionTerm, stable_identifier};
use super::{SkosConfig, SkosGraphSelection, SkosRelationRoles};

/// Result of projecting an RDF 1.2 dataset into one SKOS concept-scheme view.
#[derive(Debug, Clone)]
pub struct SkosProjection {
    /// Frozen, deterministic default-graph SKOS dataset.
    pub dataset: Arc<RdfDataset>,
    /// Native deterministic Turtle serialization of [`Self::dataset`].
    pub turtle: Vec<u8>,
    /// Located, always-computed loss ledger for the source dataset.
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OutputQuad {
    subject: ProjectionTerm,
    predicate: String,
    object: ProjectionTerm,
}

#[derive(Default)]
struct ConceptLabels {
    preferred: BTreeSet<ProjectionTerm>,
    alternate: BTreeSet<ProjectionTerm>,
    hidden: BTreeSet<ProjectionTerm>,
}

#[derive(Clone, Copy)]
enum LabelKind {
    Preferred,
    Alternate,
    Hidden,
    Notation,
}

#[derive(Clone, Copy)]
enum RelationKind {
    Broader,
    Narrower,
    Related,
    CloseMatch,
    ExactMatch,
    BroadMatch,
    NarrowMatch,
    RelatedMatch,
}

struct Projector<'a> {
    config: &'a SkosConfig,
    quads: Vec<SourceQuad>,
    consumed: Vec<bool>,
    named_graphs: Vec<ProjectionTerm>,
    reifiers: Vec<SourceReifier>,
    annotations: Vec<SourceAnnotation>,
    output: BTreeSet<OutputQuad>,
    concepts: BTreeSet<String>,
    top_concepts: BTreeSet<String>,
    labels: BTreeMap<String, ConceptLabels>,
    broader: BTreeSet<(String, String)>,
    related: BTreeSet<(String, String)>,
    exact_matches: BTreeSet<(String, String)>,
    other_matches: BTreeSet<(String, String)>,
    ledger: LossLedger,
    contract: LossLedger,
}

/// Project any static RDF dataset backend into a deterministic SKOS view.
///
/// Vocabulary, concept-scheme identity, and source graph selection are mandatory
/// caller configuration. Backend-local ids are resolved before sorting, so neither
/// interning order nor backend representation can affect the result.
///
/// # Errors
///
/// Returns a typed configuration, term, integrity, codec, or resource-limit
/// failure for malformed role data, a violated SKOS integrity condition, an
/// invalid backend view, or an exceeded caller bound.
pub fn project_skos<D: DatasetView>(
    view: &D,
    config: &SkosConfig,
) -> Result<SkosProjection, ProjectionError> {
    Projector::load(view, config)?.project()
}

impl<'a> Projector<'a> {
    fn load<D: DatasetView>(view: &D, config: &'a SkosConfig) -> Result<Self, ProjectionError> {
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
            let reifier = resolve_term(view, row.s, config, &mut cache)?;
            let statement = resolve_term(view, row.o, config, &mut cache)?;
            if !matches!(statement, ProjectionTerm::Triple { .. }) {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a reifier binding to a non-triple term",
                ));
            }
            let graph = row
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
        for row in view.annotation_quads() {
            let reifier = resolve_term(view, row.s, config, &mut cache)?;
            let ProjectionTerm::Iri { value: predicate } =
                resolve_term(view, row.p, config, &mut cache)?
            else {
                return Err(ProjectionError::integrity(
                    "RDF dataset view exposed a non-IRI annotation predicate",
                ));
            };
            let object = resolve_term(view, row.o, config, &mut cache)?;
            let graph = row
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

        let input_records = quads
            .len()
            .checked_add(named_graphs.len())
            .and_then(|count| count.checked_add(reifiers.len()))
            .and_then(|count| count.checked_add(annotations.len()))
            .ok_or_else(|| ProjectionError::limit("SKOS input record count overflow"))?;
        if input_records > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "SKOS input has {input_records} records; limit is {}",
                config.max_records()
            )));
        }

        let mut projector = Self {
            config,
            consumed: vec![false; quads.len()],
            quads,
            named_graphs,
            reifiers,
            annotations,
            output: BTreeSet::new(),
            concepts: BTreeSet::new(),
            top_concepts: BTreeSet::new(),
            labels: BTreeMap::new(),
            broader: BTreeSet::new(),
            related: BTreeSet::new(),
            exact_matches: BTreeSet::new(),
            other_matches: BTreeSet::new(),
            ledger: LossLedger::new(),
            contract: rdf_to_skos_loss_ledger(),
        };
        projector.record_structural_losses()?;
        Ok(projector)
    }

    fn project(mut self) -> Result<SkosProjection, ProjectionError> {
        for index in 0..self.quads.len() {
            if self.selected(self.quads[index].graph.as_ref()) {
                self.project_quad(index)?;
            }
        }
        self.validate_integrity()?;
        self.emit_derived_scheme_surface();
        self.record_unrepresented_input()?;
        self.enforce_record_budget()?;

        let dataset = self.freeze_output()?;
        let serialized = serialize_dataset_to_format(&*dataset, NativeRdfFormat::Turtle, None)
            .map_err(|error| {
                ProjectionError::integrity(format!(
                    "native Turtle serialization of SKOS view failed: {error}"
                ))
            })?;
        if serialized.statement_rows_dropped != 0 || serialized.directional_literals_dropped != 0 {
            return Err(ProjectionError::integrity(
                "native Turtle serialization unexpectedly dropped SKOS dataset content",
            ));
        }
        if serialized.bytes.len() > self.config.limits().max_artifact_bytes() {
            return Err(ProjectionError::limit(format!(
                "SKOS Turtle is {} bytes; per-artifact limit is {}",
                serialized.bytes.len(),
                self.config.limits().max_artifact_bytes()
            )));
        }
        check_ledger_sound(&self.ledger, "rdf-1.2-dataset", "skos")
            .map_err(ProjectionError::integrity)?;

        Ok(SkosProjection {
            dataset,
            turtle: serialized.bytes,
            loss_ledger: self.ledger,
        })
    }

    fn project_quad(&mut self, index: usize) -> Result<(), ProjectionError> {
        let quad = self.quads[index].clone();
        let source = self.config.source();
        if quad.predicate == source.classes().rdf_type() {
            if quad.object == iri(source.classes().concept()) {
                let Some(concept) = iri_value(&quad.subject) else {
                    return Ok(());
                };
                self.add_concept(concept)?;
                self.consumed[index] = true;
            } else if quad.object == iri(source.classes().concept_scheme())
                && quad.subject == iri(self.config.scheme_iri())
            {
                self.consumed[index] = true;
            }
            return Ok(());
        }

        if let Some((kind, target_predicate)) = self.label_role(&quad.predicate) {
            let Some(concept) = iri_value(&quad.subject) else {
                return Ok(());
            };
            if matches!(
                quad.object,
                ProjectionTerm::Blank { .. } | ProjectionTerm::Triple { .. }
            ) {
                return Ok(());
            }
            if !matches!(quad.object, ProjectionTerm::Literal { .. }) {
                return Err(ProjectionError::integrity(format!(
                    "configured SKOS lexical role `{}` has a non-literal object",
                    quad.predicate
                )));
            }
            self.add_concept(concept)?;
            self.record_label(concept, kind, &quad.object);
            self.emit_iri(concept, target_predicate, quad.object);
            self.consumed[index] = true;
            return Ok(());
        }

        if let Some(target_predicate) = self.documentation_role(&quad.predicate) {
            let Some(concept) = iri_value(&quad.subject) else {
                return Ok(());
            };
            if matches!(quad.object, ProjectionTerm::Triple { .. }) {
                return Ok(());
            }
            self.add_concept(concept)?;
            self.emit_iri(concept, target_predicate, quad.object);
            self.consumed[index] = true;
            return Ok(());
        }

        if let Some((kind, target_predicate)) = self.relation_role(&quad.predicate) {
            let (Some(subject), Some(object)) = (iri_value(&quad.subject), iri_value(&quad.object))
            else {
                if matches!(
                    (&quad.subject, &quad.object),
                    (ProjectionTerm::Literal { .. }, _) | (_, ProjectionTerm::Literal { .. })
                ) {
                    return Err(ProjectionError::integrity(format!(
                        "configured SKOS relation role `{}` has a literal endpoint",
                        quad.predicate
                    )));
                }
                return Ok(());
            };
            self.record_relation(kind, subject, object)?;
            self.emit_iri(subject, target_predicate, iri(object));
            self.consumed[index] = true;
            return Ok(());
        }

        let source_relations = source.relations();
        if quad.predicate == source_relations.in_scheme() {
            if quad.object == iri(self.config.scheme_iri()) {
                let Some(concept) = iri_value(&quad.subject) else {
                    return Ok(());
                };
                self.add_concept(concept)?;
                self.consumed[index] = true;
            }
            return Ok(());
        }
        if quad.predicate == source_relations.has_top_concept() {
            if quad.subject == iri(self.config.scheme_iri()) {
                let Some(concept) = iri_value(&quad.object) else {
                    return Ok(());
                };
                self.add_top_concept(concept)?;
                self.consumed[index] = true;
            }
            return Ok(());
        }
        if quad.predicate == source_relations.top_concept_of()
            && quad.object == iri(self.config.scheme_iri())
        {
            let Some(concept) = iri_value(&quad.subject) else {
                return Ok(());
            };
            self.add_top_concept(concept)?;
            self.consumed[index] = true;
        }
        Ok(())
    }

    fn label_role(&self, predicate: &str) -> Option<(LabelKind, String)> {
        let source = self.config.source().labels();
        let target = self.config.target().labels();
        [
            (
                source.pref_label(),
                LabelKind::Preferred,
                target.pref_label(),
            ),
            (source.alt_label(), LabelKind::Alternate, target.alt_label()),
            (
                source.hidden_label(),
                LabelKind::Hidden,
                target.hidden_label(),
            ),
            (source.notation(), LabelKind::Notation, target.notation()),
        ]
        .into_iter()
        .find_map(|(candidate, kind, target)| {
            (predicate == candidate).then(|| (kind, target.to_owned()))
        })
    }

    fn documentation_role(&self, predicate: &str) -> Option<String> {
        let source = self.config.source().documentation();
        let target = self.config.target().documentation();
        [
            (source.note(), target.note()),
            (source.change_note(), target.change_note()),
            (source.definition(), target.definition()),
            (source.editorial_note(), target.editorial_note()),
            (source.example(), target.example()),
            (source.history_note(), target.history_note()),
            (source.scope_note(), target.scope_note()),
        ]
        .into_iter()
        .find_map(|(candidate, target)| (predicate == candidate).then(|| target.to_owned()))
    }

    fn relation_role(&self, predicate: &str) -> Option<(RelationKind, String)> {
        let source = self.config.source().relations();
        let target = self.config.target().relations();
        relation_pairs(source, target)
            .into_iter()
            .find_map(|(candidate, kind, target)| {
                (predicate == candidate).then(|| (kind, target.to_owned()))
            })
    }

    fn record_label(&mut self, concept: &str, kind: LabelKind, value: &ProjectionTerm) {
        let labels = self.labels.entry(concept.to_owned()).or_default();
        match kind {
            LabelKind::Preferred => {
                labels.preferred.insert(value.clone());
            }
            LabelKind::Alternate => {
                labels.alternate.insert(value.clone());
            }
            LabelKind::Hidden => {
                labels.hidden.insert(value.clone());
            }
            LabelKind::Notation => {}
        }
    }

    fn record_relation(
        &mut self,
        kind: RelationKind,
        subject: &str,
        object: &str,
    ) -> Result<(), ProjectionError> {
        if object == self.config.scheme_iri() {
            return Err(ProjectionError::integrity(
                "SKOS integrity S9 violated: a Concept relation targets the configured ConceptScheme",
            ));
        }
        match kind {
            RelationKind::Broader => {
                self.add_concept(subject)?;
                self.add_concept(object)?;
                self.broader.insert((subject.to_owned(), object.to_owned()));
            }
            RelationKind::Narrower => {
                self.add_concept(subject)?;
                self.add_concept(object)?;
                self.broader.insert((object.to_owned(), subject.to_owned()));
            }
            RelationKind::Related => {
                self.add_concept(subject)?;
                self.add_concept(object)?;
                self.related.insert(unordered_pair(subject, object));
            }
            RelationKind::ExactMatch => {
                self.add_concept(subject)?;
                self.exact_matches.insert(unordered_pair(subject, object));
            }
            RelationKind::BroadMatch | RelationKind::NarrowMatch | RelationKind::RelatedMatch => {
                self.add_concept(subject)?;
                self.other_matches.insert(unordered_pair(subject, object));
            }
            RelationKind::CloseMatch => {
                self.add_concept(subject)?;
            }
        }
        Ok(())
    }

    fn add_concept(&mut self, concept: &str) -> Result<(), ProjectionError> {
        if concept == self.config.scheme_iri() {
            return Err(ProjectionError::integrity(
                "SKOS integrity S9 violated: the configured ConceptScheme is also a Concept",
            ));
        }
        self.concepts.insert(concept.to_owned());
        Ok(())
    }

    fn add_top_concept(&mut self, concept: &str) -> Result<(), ProjectionError> {
        self.add_concept(concept)?;
        self.top_concepts.insert(concept.to_owned());
        Ok(())
    }

    fn validate_integrity(&self) -> Result<(), ProjectionError> {
        for (concept, labels) in &self.labels {
            for value in &labels.preferred {
                if labels.alternate.contains(value) || labels.hidden.contains(value) {
                    return Err(ProjectionError::integrity(format!(
                        "SKOS integrity S13 violated for `{concept}`: one literal is both preferred and alternate/hidden"
                    )));
                }
            }
            for value in &labels.alternate {
                if labels.hidden.contains(value) {
                    return Err(ProjectionError::integrity(format!(
                        "SKOS integrity S13 violated for `{concept}`: one literal is both alternate and hidden"
                    )));
                }
            }
            let mut by_language: BTreeMap<Option<&str>, &ProjectionTerm> = BTreeMap::new();
            for value in &labels.preferred {
                let ProjectionTerm::Literal { language, .. } = value else {
                    unreachable!("lexical roles were validated as literals")
                };
                if by_language.insert(language.as_deref(), value).is_some() {
                    return Err(ProjectionError::integrity(format!(
                        "SKOS integrity S14 violated for `{concept}`: more than one preferred label has the same language"
                    )));
                }
            }
        }

        for (left, right) in &self.related {
            if self.reachable(left, right)? || self.reachable(right, left)? {
                return Err(ProjectionError::integrity(format!(
                    "SKOS integrity S27 violated: `{left}` and `{right}` are both related and connected by broaderTransitive"
                )));
            }
        }
        if let Some(pair) = self.exact_matches.intersection(&self.other_matches).next() {
            return Err(ProjectionError::integrity(format!(
                "SKOS integrity S46 violated: exactMatch overlaps broadMatch, narrowMatch, or relatedMatch for `{}` and `{}`",
                pair.0, pair.1
            )));
        }
        Ok(())
    }

    fn reachable(&self, start: &str, target: &str) -> Result<bool, ProjectionError> {
        let mut pending = vec![start];
        let mut visited = BTreeSet::new();
        while let Some(node) = pending.pop() {
            if !visited.insert(node) {
                continue;
            }
            if visited.len() > self.config.max_records() {
                return Err(ProjectionError::limit(
                    "SKOS hierarchy traversal exceeded max_records",
                ));
            }
            for (_, broader) in self.broader.iter().filter(|(narrower, _)| narrower == node) {
                if broader == target {
                    return Ok(true);
                }
                pending.push(broader);
            }
        }
        Ok(false)
    }

    fn emit_derived_scheme_surface(&mut self) {
        let scheme = self.config.scheme_iri().to_owned();
        let target_classes = self.config.target().classes();
        let target_relations = self.config.target().relations();
        self.output.insert(OutputQuad {
            subject: iri(&scheme),
            predicate: target_classes.rdf_type().to_owned(),
            object: iri(target_classes.concept_scheme()),
        });
        for concept in &self.concepts {
            self.output.insert(OutputQuad {
                subject: iri(concept),
                predicate: target_classes.rdf_type().to_owned(),
                object: iri(target_classes.concept()),
            });
            self.output.insert(OutputQuad {
                subject: iri(concept),
                predicate: target_relations.in_scheme().to_owned(),
                object: iri(&scheme),
            });
        }
        for concept in &self.top_concepts {
            self.output.insert(OutputQuad {
                subject: iri(&scheme),
                predicate: target_relations.has_top_concept().to_owned(),
                object: iri(concept),
            });
            self.output.insert(OutputQuad {
                subject: iri(concept),
                predicate: target_relations.top_concept_of().to_owned(),
                object: iri(&scheme),
            });
        }
    }

    fn emit_iri(&mut self, subject: &str, predicate: String, object: ProjectionTerm) {
        self.output.insert(OutputQuad {
            subject: iri(subject),
            predicate,
            object,
        });
    }

    fn selected(&self, graph: Option<&ProjectionTerm>) -> bool {
        match self.config.graph_selection() {
            SkosGraphSelection::DefaultGraph => graph.is_none(),
            SkosGraphSelection::NamedGraph { graph_iri } => {
                matches!(graph, Some(ProjectionTerm::Iri { value }) if value == graph_iri)
            }
            SkosGraphSelection::Union => true,
        }
    }

    fn record_structural_losses(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.named_graphs.len() {
            self.record_named_graph_loss(index, LOSS_SKOS_NAMED_GRAPH_DROPPED)?;
            if contains_blank(&self.named_graphs[index]) {
                self.record_named_graph_loss(index, LOSS_SKOS_BLANK_IDENTITY_DROPPED)?;
            }
            if contains_triple(&self.named_graphs[index]) {
                self.record_named_graph_loss(index, LOSS_SKOS_TRIPLE_TERM_DROPPED)?;
            }
        }
        for index in 0..self.quads.len() {
            if self.quads[index].graph.is_some() {
                self.record_quad_loss(index, LOSS_SKOS_NAMED_GRAPH_DROPPED)?;
            }
        }
        for index in 0..self.reifiers.len() {
            if self.reifiers[index].graph.is_some() {
                self.record_reifier_loss(index, LOSS_SKOS_NAMED_GRAPH_DROPPED)?;
            }
            self.record_reifier_loss(index, LOSS_SKOS_REIFIER_DROPPED)?;
            self.record_reifier_loss(index, LOSS_SKOS_TRIPLE_TERM_DROPPED)?;
            if source_reifier_contains_blank(&self.reifiers[index]) {
                self.record_reifier_loss(index, LOSS_SKOS_BLANK_IDENTITY_DROPPED)?;
            }
        }
        for index in 0..self.annotations.len() {
            if self.annotations[index].graph.is_some() {
                self.record_annotation_loss(index, LOSS_SKOS_NAMED_GRAPH_DROPPED)?;
            }
            self.record_annotation_loss(index, LOSS_SKOS_ANNOTATION_DROPPED)?;
            if source_annotation_contains_blank(&self.annotations[index]) {
                self.record_annotation_loss(index, LOSS_SKOS_BLANK_IDENTITY_DROPPED)?;
            }
            if source_annotation_contains_triple(&self.annotations[index]) {
                self.record_annotation_loss(index, LOSS_SKOS_TRIPLE_TERM_DROPPED)?;
            }
        }
        Ok(())
    }

    fn record_unrepresented_input(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.quads.len() {
            if self.consumed[index] {
                continue;
            }
            self.record_quad_loss(index, LOSS_SKOS_NON_PROFILE_STATEMENT_DROPPED)?;
            if source_quad_contains_blank(&self.quads[index]) {
                self.record_quad_loss(index, LOSS_SKOS_BLANK_IDENTITY_DROPPED)?;
            }
            if source_quad_contains_triple(&self.quads[index]) {
                self.record_quad_loss(index, LOSS_SKOS_TRIPLE_TERM_DROPPED)?;
            }
        }
        Ok(())
    }

    fn enforce_record_budget(&self) -> Result<(), ProjectionError> {
        let total = self
            .quads
            .len()
            .checked_add(self.named_graphs.len())
            .and_then(|count| count.checked_add(self.reifiers.len()))
            .and_then(|count| count.checked_add(self.annotations.len()))
            .and_then(|count| count.checked_add(self.output.len()))
            .ok_or_else(|| ProjectionError::limit("SKOS total record count overflow"))?;
        if total > self.config.max_records() {
            return Err(ProjectionError::limit(format!(
                "SKOS projection uses {total} input/output records; limit is {}",
                self.config.max_records()
            )));
        }
        Ok(())
    }

    fn freeze_output(&self) -> Result<Arc<RdfDataset>, ProjectionError> {
        let mut builder = RdfDatasetBuilder::new();
        for quad in &self.output {
            let subject = intern_term(&mut builder, &quad.subject)?;
            let predicate = builder.intern_iri(&quad.predicate);
            let object = intern_term(&mut builder, &quad.object)?;
            builder.push_quad(subject, predicate, object, None);
        }
        builder.freeze().map_err(|error| {
            ProjectionError::integrity(format!("SKOS output is not a valid RDF dataset: {error}"))
        })
    }

    fn record_quad_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("quad", &self.quads[index])?;
        self.record_loss(code, "skos:quad", subject);
        Ok(())
    }

    fn record_named_graph_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("graph", &self.named_graphs[index])?;
        self.record_loss(code, "skos:named-graph", subject);
        Ok(())
    }

    fn record_reifier_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("reifier", &self.reifiers[index])?;
        self.record_loss(code, "skos:reifier", subject);
        Ok(())
    }

    fn record_annotation_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("annotation", &self.annotations[index])?;
        self.record_loss(code, "skos:annotation", subject);
        Ok(())
    }

    fn record_loss(&mut self, code: &'static str, logical: &str, subject: String) {
        let template = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .expect("runtime SKOS code must exist in the closed contract");
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

fn relation_pairs<'a>(
    source: &'a SkosRelationRoles,
    target: &'a SkosRelationRoles,
) -> [(&'a str, RelationKind, &'a str); 8] {
    [
        (source.broader(), RelationKind::Broader, target.broader()),
        (source.narrower(), RelationKind::Narrower, target.narrower()),
        (source.related(), RelationKind::Related, target.related()),
        (
            source.close_match(),
            RelationKind::CloseMatch,
            target.close_match(),
        ),
        (
            source.exact_match(),
            RelationKind::ExactMatch,
            target.exact_match(),
        ),
        (
            source.broad_match(),
            RelationKind::BroadMatch,
            target.broad_match(),
        ),
        (
            source.narrow_match(),
            RelationKind::NarrowMatch,
            target.narrow_match(),
        ),
        (
            source.related_match(),
            RelationKind::RelatedMatch,
            target.related_match(),
        ),
    ]
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    config: &SkosConfig,
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
        ProjectionTerm::Triple { .. } => {
            return Err(ProjectionError::integrity(
                "SKOS output unexpectedly contains an RDF triple term",
            ));
        }
    })
}

fn iri(value: &str) -> ProjectionTerm {
    ProjectionTerm::Iri {
        value: value.to_owned(),
    }
}

fn iri_value(term: &ProjectionTerm) -> Option<&str> {
    let ProjectionTerm::Iri { value } = term else {
        return None;
    };
    Some(value)
}

fn unordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_owned(), right.to_owned())
    } else {
        (right.to_owned(), left.to_owned())
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

fn source_quad_contains_blank(quad: &SourceQuad) -> bool {
    contains_blank(&quad.subject)
        || contains_blank(&quad.object)
        || quad.graph.as_ref().is_some_and(contains_blank)
}

fn source_quad_contains_triple(quad: &SourceQuad) -> bool {
    contains_triple(&quad.subject)
        || contains_triple(&quad.object)
        || quad.graph.as_ref().is_some_and(contains_triple)
}

fn source_reifier_contains_blank(row: &SourceReifier) -> bool {
    contains_blank(&row.reifier)
        || contains_blank(&row.statement)
        || row.graph.as_ref().is_some_and(contains_blank)
}

fn source_annotation_contains_blank(row: &SourceAnnotation) -> bool {
    contains_blank(&row.reifier)
        || contains_blank(&row.object)
        || row.graph.as_ref().is_some_and(contains_blank)
}

fn source_annotation_contains_triple(row: &SourceAnnotation) -> bool {
    contains_triple(&row.reifier)
        || contains_triple(&row.object)
        || row.graph.as_ref().is_some_and(contains_triple)
}

#[cfg(test)]
mod tests {
    use purrdf_core::{
        PackBuilder, PackView, RdfTextDirection, TermRef, assert_ledger_complete,
        datasets_isomorphic,
    };

    use super::*;
    use crate::native_codecs::parse_dataset;
    use crate::projections::{
        ProjectionErrorKind, ProjectionLimits, SkosClassRoles, SkosDocumentationRoles,
        SkosLabelRoles, SkosSourceRoles, SkosTargetRoles,
    };

    const SOURCE: &str = "https://source.example/";
    const TARGET: &str = "https://target.example/";
    const DATA: &str = "https://data.example/";

    fn class_roles(prefix: &str) -> SkosClassRoles {
        SkosClassRoles::new(
            format!("{prefix}type"),
            format!("{prefix}Concept"),
            format!("{prefix}ConceptScheme"),
        )
        .expect("class roles")
    }

    fn label_roles(prefix: &str) -> SkosLabelRoles {
        SkosLabelRoles::new(
            format!("{prefix}prefLabel"),
            format!("{prefix}altLabel"),
            format!("{prefix}hiddenLabel"),
            format!("{prefix}notation"),
        )
        .expect("label roles")
    }

    fn documentation_roles(prefix: &str) -> SkosDocumentationRoles {
        SkosDocumentationRoles::new(
            format!("{prefix}note"),
            format!("{prefix}changeNote"),
            format!("{prefix}definition"),
            format!("{prefix}editorialNote"),
            format!("{prefix}example"),
            format!("{prefix}historyNote"),
            format!("{prefix}scopeNote"),
        )
        .expect("documentation roles")
    }

    fn relation_roles(prefix: &str) -> SkosRelationRoles {
        SkosRelationRoles::new(
            format!("{prefix}broader"),
            format!("{prefix}narrower"),
            format!("{prefix}related"),
            format!("{prefix}closeMatch"),
            format!("{prefix}exactMatch"),
            format!("{prefix}broadMatch"),
            format!("{prefix}narrowMatch"),
            format!("{prefix}relatedMatch"),
            format!("{prefix}inScheme"),
            format!("{prefix}hasTopConcept"),
            format!("{prefix}topConceptOf"),
        )
        .expect("relation roles")
    }

    fn source_roles() -> SkosSourceRoles {
        SkosSourceRoles::new(
            class_roles(SOURCE),
            label_roles(SOURCE),
            documentation_roles(SOURCE),
            relation_roles(SOURCE),
        )
        .expect("source roles")
    }

    fn target_roles() -> SkosTargetRoles {
        SkosTargetRoles::new(
            class_roles(TARGET),
            label_roles(TARGET),
            documentation_roles(TARGET),
            relation_roles(TARGET),
        )
        .expect("target roles")
    }

    fn config(selection: SkosGraphSelection, max_records: usize) -> SkosConfig {
        SkosConfig::new(
            source_roles(),
            target_roles(),
            format!("{DATA}scheme"),
            selection,
            ProjectionLimits::new(8, 2_000_000, 4_000_000, 5_000_000, 16).expect("limits"),
            max_records,
        )
        .expect("SKOS config")
    }

    fn push_iri_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: Option<TermId>,
    ) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_iri(object);
        builder.push_quad(subject, predicate, object, graph);
    }

    fn push_literal_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        literal: RdfLiteral,
        graph: Option<TermId>,
    ) -> (TermId, TermId, TermId) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_literal(literal);
        builder.push_quad(subject, predicate, object, graph);
        (subject, predicate, object)
    }

    fn literal(value: &str) -> RdfLiteral {
        RdfLiteral {
            lexical_form: value.to_owned(),
            datatype: None,
            language: None,
            direction: None,
        }
    }

    fn fixture(reverse_interning: bool) -> Arc<RdfDataset> {
        let configuration = config(SkosGraphSelection::DefaultGraph, 1_000);
        let source = configuration.source();
        let classes = source.classes();
        let labels = source.labels();
        let docs = source.documentation();
        let relations = source.relations();
        let scheme = configuration.scheme_iri();
        let c1 = format!("{DATA}c1");
        let c2 = format!("{DATA}c2");
        let c3 = format!("{DATA}c3");
        let c4 = format!("{DATA}c4");
        let mut builder = RdfDatasetBuilder::new();
        if reverse_interning {
            let _ = builder.intern_iri(&format!("{DATA}z-unused"));
            let _ = builder.intern_iri(&format!("{DATA}a-unused"));
        }

        push_iri_quad(
            &mut builder,
            scheme,
            classes.rdf_type(),
            classes.concept_scheme(),
            None,
        );
        push_iri_quad(
            &mut builder,
            &c1,
            classes.rdf_type(),
            classes.concept(),
            None,
        );
        push_literal_quad(
            &mut builder,
            &c1,
            labels.pref_label(),
            RdfLiteral {
                lexical_form: "مرحبا".to_owned(),
                datatype: None,
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            },
            None,
        );
        push_literal_quad(
            &mut builder,
            &c1,
            labels.alt_label(),
            literal("alternate"),
            None,
        );
        push_literal_quad(
            &mut builder,
            &c1,
            labels.hidden_label(),
            literal("hidden"),
            None,
        );
        push_literal_quad(
            &mut builder,
            &c1,
            labels.notation(),
            RdfLiteral {
                lexical_form: "C-1".to_owned(),
                datatype: Some(format!("{DATA}Notation")),
                language: None,
                direction: None,
            },
            None,
        );
        for (predicate, value) in [
            (docs.note(), "note"),
            (docs.change_note(), "change"),
            (docs.definition(), "definition"),
            (docs.editorial_note(), "editorial"),
            (docs.example(), "example"),
            (docs.history_note(), "history"),
            (docs.scope_note(), "scope"),
        ] {
            push_literal_quad(&mut builder, &c1, predicate, literal(value), None);
        }
        let concept = builder.intern_iri(&c1);
        let note = builder.intern_iri(docs.note());
        let structured_note = builder.intern_blank("structured-note", BlankScope(3));
        builder.push_quad(concept, note, structured_note, None);
        push_iri_quad(&mut builder, &c1, relations.broader(), &c2, None);
        push_iri_quad(&mut builder, &c2, relations.narrower(), &c3, None);
        push_iri_quad(&mut builder, &c1, relations.related(), &c4, None);
        for (predicate, object) in [
            (relations.close_match(), format!("{DATA}external-close")),
            (relations.exact_match(), format!("{DATA}external-exact")),
            (relations.broad_match(), format!("{DATA}external-broad")),
            (relations.narrow_match(), format!("{DATA}external-narrow")),
            (relations.related_match(), format!("{DATA}external-related")),
        ] {
            push_iri_quad(&mut builder, &c1, predicate, &object, None);
        }
        push_iri_quad(&mut builder, &c1, relations.in_scheme(), scheme, None);
        push_iri_quad(&mut builder, scheme, relations.has_top_concept(), &c1, None);
        push_iri_quad(
            &mut builder,
            &c1,
            &format!("{SOURCE}unmapped"),
            &format!("{DATA}ignored"),
            None,
        );

        let graph = builder.intern_iri(&format!("{DATA}source-graph"));
        builder.declare_named_graph(graph);
        let blank = builder.intern_blank("unstable", BlankScope(7));
        let junk = builder.intern_iri(&format!("{SOURCE}junk"));
        let quoted_subject = builder.intern_iri(&c1);
        let quoted_object = builder.intern_iri(&c2);
        let quoted = builder.intern_triple(quoted_subject, junk, quoted_object);
        builder.push_quad(blank, junk, quoted, Some(graph));
        let reifier = builder.intern_blank("reifier", BlankScope(9));
        builder.push_reifier_in_graph(reifier, quoted, Some(graph));
        let annotation_predicate = builder.intern_iri(&format!("{SOURCE}confidence"));
        builder.push_annotation_in_graph(reifier, annotation_predicate, quoted, Some(graph));
        builder.freeze().expect("SKOS fixture")
    }

    #[test]
    fn projects_complete_surface_with_literal_fidelity_and_closed_ledger() {
        let dataset = fixture(false);
        let configuration = config(SkosGraphSelection::DefaultGraph, 1_000);
        let projected = project_skos(dataset.as_ref(), &configuration).expect("project");
        let reparsed = parse_dataset(&projected.turtle, "text/turtle", None).expect("parse Turtle");
        assert!(datasets_isomorphic(&projected.dataset, &reparsed));
        assert!(
            projected.dataset.quads().all(|quad| quad.g.is_none()),
            "SKOS view must be one default graph"
        );
        assert!(projected.dataset.quads().any(|quad| {
            matches!(
                projected.dataset.resolve(quad.o),
                TermRef::Literal {
                    lexical: "مرحبا",
                    language: Some("ar"),
                    direction: Some(RdfTextDirection::Rtl),
                    ..
                }
            )
        }));
        let turtle = std::str::from_utf8(&projected.turtle).expect("UTF-8 Turtle");
        let target_labels = configuration.target().labels();
        let target_docs = configuration.target().documentation();
        let target_relations = configuration.target().relations();
        for role in [
            target_labels.pref_label(),
            target_labels.alt_label(),
            target_labels.hidden_label(),
            target_labels.notation(),
            target_docs.note(),
            target_docs.change_note(),
            target_docs.definition(),
            target_docs.editorial_note(),
            target_docs.example(),
            target_docs.history_note(),
            target_docs.scope_note(),
            target_relations.broader(),
            target_relations.narrower(),
            target_relations.related(),
            target_relations.close_match(),
            target_relations.exact_match(),
            target_relations.broad_match(),
            target_relations.narrow_match(),
            target_relations.related_match(),
            target_relations.in_scheme(),
            target_relations.has_top_concept(),
            target_relations.top_concept_of(),
        ] {
            assert!(turtle.contains(role), "missing target role {role}");
        }
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
                LOSS_SKOS_ANNOTATION_DROPPED,
                LOSS_SKOS_BLANK_IDENTITY_DROPPED,
                LOSS_SKOS_NAMED_GRAPH_DROPPED,
                LOSS_SKOS_NON_PROFILE_STATEMENT_DROPPED,
                LOSS_SKOS_REIFIER_DROPPED,
                LOSS_SKOS_TRIPLE_TERM_DROPPED,
            ],
        );
    }

    #[test]
    fn output_is_backend_and_interning_order_independent() {
        let configuration = config(SkosGraphSelection::DefaultGraph, 1_000);
        let first = fixture(false);
        let reversed = fixture(true);
        let resident = project_skos(first.as_ref(), &configuration).expect("resident");
        let reordered = project_skos(reversed.as_ref(), &configuration).expect("reordered");
        assert_eq!(resident.turtle, reordered.turtle);
        assert_eq!(
            resident.loss_ledger.render_json(),
            reordered.loss_ledger.render_json()
        );

        let bytes = PackBuilder::build_bytes(&first).expect("pack");
        let view = PackView::from_bytes(&bytes).expect("pack view");
        let packed = project_skos(&view, &configuration).expect("packed");
        assert_eq!(resident.turtle, packed.turtle);
        assert!(datasets_isomorphic(&resident.dataset, &packed.dataset));
        assert_eq!(
            resident.loss_ledger.render_json(),
            packed.loss_ledger.render_json()
        );
    }

    #[test]
    fn graph_selection_is_explicit_and_named_placement_is_ledgered() {
        let source = source_roles();
        let mut builder = RdfDatasetBuilder::new();
        let graph_iri = format!("{DATA}selected");
        let graph = builder.intern_iri(&graph_iri);
        push_literal_quad(
            &mut builder,
            &format!("{DATA}default-concept"),
            source.labels().pref_label(),
            literal("default"),
            None,
        );
        push_literal_quad(
            &mut builder,
            &format!("{DATA}named-concept"),
            source.labels().pref_label(),
            literal("named"),
            Some(graph),
        );
        let dataset = builder.freeze().expect("dataset");
        let default = project_skos(
            dataset.as_ref(),
            &config(SkosGraphSelection::DefaultGraph, 100),
        )
        .expect("default graph");
        let named = project_skos(
            dataset.as_ref(),
            &config(
                SkosGraphSelection::NamedGraph {
                    graph_iri: graph_iri.clone(),
                },
                100,
            ),
        )
        .expect("named graph");
        let union =
            project_skos(dataset.as_ref(), &config(SkosGraphSelection::Union, 100)).expect("union");
        let default_text = std::str::from_utf8(&default.turtle).expect("default UTF-8");
        let named_text = std::str::from_utf8(&named.turtle).expect("named UTF-8");
        let union_text = std::str::from_utf8(&union.turtle).expect("union UTF-8");
        assert!(default_text.contains("default-concept"));
        assert!(!default_text.contains("named-concept"));
        assert!(named_text.contains("named-concept"));
        assert!(!named_text.contains("default-concept"));
        assert!(union_text.contains("default-concept"));
        assert!(union_text.contains("named-concept"));
        for result in [&default, &named, &union] {
            assert!(result.loss_ledger.entries().iter().any(|entry| {
                entry.code == LOSS_SKOS_NAMED_GRAPH_DROPPED && entry.location.is_some()
            }));
        }
    }

    fn dataset_with_iri_rows(rows: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        for (subject, predicate, object) in rows {
            push_iri_quad(&mut builder, subject, predicate, object, None);
        }
        builder.freeze().expect("dataset")
    }

    #[test]
    fn enforces_s9_s27_and_s46_relation_integrity() {
        let configuration = config(SkosGraphSelection::DefaultGraph, 100);
        let source = configuration.source();
        let scheme = configuration.scheme_iri();
        let s9 = dataset_with_iri_rows(&[(
            scheme,
            source.classes().rdf_type(),
            source.classes().concept(),
        )]);
        let error = project_skos(s9.as_ref(), &configuration).expect_err("S9");
        assert!(error.message().contains("S9"));

        let a = format!("{DATA}a");
        let b = format!("{DATA}b");
        let s27 = dataset_with_iri_rows(&[
            (&a, source.relations().broader(), &b),
            (&a, source.relations().related(), &b),
        ]);
        let error = project_skos(s27.as_ref(), &configuration).expect_err("S27");
        assert!(error.message().contains("S27"));

        let s46 = dataset_with_iri_rows(&[
            (&a, source.relations().exact_match(), &b),
            (&b, source.relations().broad_match(), &a),
        ]);
        let error = project_skos(s46.as_ref(), &configuration).expect_err("S46");
        assert!(error.message().contains("S46"));
    }

    #[test]
    fn enforces_s13_s14_and_lexical_term_shape() {
        let configuration = config(SkosGraphSelection::DefaultGraph, 100);
        let labels = configuration.source().labels();
        let concept = format!("{DATA}concept");

        let mut s13 = RdfDatasetBuilder::new();
        push_literal_quad(
            &mut s13,
            &concept,
            labels.pref_label(),
            literal("same"),
            None,
        );
        push_literal_quad(
            &mut s13,
            &concept,
            labels.alt_label(),
            literal("same"),
            None,
        );
        let s13 = s13.freeze().expect("S13 dataset");
        let error = project_skos(s13.as_ref(), &configuration).expect_err("S13");
        assert!(error.message().contains("S13"));

        let mut s14 = RdfDatasetBuilder::new();
        for lexical in ["first", "second"] {
            push_literal_quad(
                &mut s14,
                &concept,
                labels.pref_label(),
                RdfLiteral {
                    lexical_form: lexical.to_owned(),
                    datatype: None,
                    language: Some("en".to_owned()),
                    direction: None,
                },
                None,
            );
        }
        let s14 = s14.freeze().expect("S14 dataset");
        let error = project_skos(s14.as_ref(), &configuration).expect_err("S14");
        assert!(error.message().contains("S14"));

        let malformed = dataset_with_iri_rows(&[(
            &concept,
            labels.pref_label(),
            &format!("{DATA}not-a-literal"),
        )]);
        let error = project_skos(malformed.as_ref(), &configuration).expect_err("malformed");
        assert_eq!(error.kind(), ProjectionErrorKind::Integrity);
        assert!(error.message().contains("non-literal"));
    }

    #[test]
    fn rejects_incomplete_or_ambiguous_configuration_and_enforces_bounds() {
        let configuration = config(SkosGraphSelection::DefaultGraph, 1_000);
        let mut value = serde_json::to_value(&configuration).expect("config JSON");
        value["source"]["classes"]
            .as_object_mut()
            .expect("class roles")
            .remove("rdf_type");
        assert!(serde_json::from_value::<SkosConfig>(value).is_err());

        let ambiguous = SkosLabelRoles::new(
            format!("{SOURCE}same"),
            format!("{SOURCE}same"),
            format!("{SOURCE}hidden"),
            format!("{SOURCE}notation"),
        )
        .expect("locally valid IRIs");
        assert!(
            SkosSourceRoles::new(
                class_roles(SOURCE),
                ambiguous,
                documentation_roles(SOURCE),
                relation_roles(SOURCE),
            )
            .is_err()
        );

        let dataset = fixture(false);
        assert!(
            project_skos(
                dataset.as_ref(),
                &config(SkosGraphSelection::DefaultGraph, 1)
            )
            .is_err()
        );
        let tiny = SkosConfig::new(
            source_roles(),
            target_roles(),
            format!("{DATA}scheme"),
            SkosGraphSelection::DefaultGraph,
            ProjectionLimits::new(2, 32, 32, 1_536, 16).expect("tiny limits"),
            1_000,
        )
        .expect("tiny config");
        assert!(project_skos(dataset.as_ref(), &tiny).is_err());
    }

    #[test]
    fn explicitly_empty_named_graph_has_a_located_loss() {
        let mut builder = RdfDatasetBuilder::new();
        let graph = builder.intern_iri(&format!("{DATA}empty"));
        builder.declare_named_graph(graph);
        let dataset = builder.freeze().expect("empty graph dataset");
        let projected = project_skos(
            dataset.as_ref(),
            &config(SkosGraphSelection::DefaultGraph, 100),
        )
        .expect("project");
        assert_eq!(projected.loss_ledger.entries().len(), 1);
        assert_eq!(
            projected.loss_ledger.entries()[0].code,
            LOSS_SKOS_NAMED_GRAPH_DROPPED
        );
        assert_eq!(
            projected.loss_ledger.entries()[0]
                .location
                .as_ref()
                .and_then(|location| location.logical.as_deref()),
            Some("skos:named-graph")
        );
    }
}

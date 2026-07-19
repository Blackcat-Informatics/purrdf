// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use purrdf_core::{
    DatasetView, LossEntry, LossLedger, RdfLocation, check_ledger_sound, rdf_to_okf_loss_ledger,
};
use purrdf_xsd::XsdValue;
use serde::Serialize;

use super::config::validate_path_stem;
use super::{
    OkfBodyStyle, OkfBodyValueMode, OkfCardinality, OkfConceptSelector, OkfFieldMapping,
    OkfGenerationConfig, OkfGraphSelection, OkfLinkPathStyle, OkfLinkStyle, OkfLinkTargetMode,
    OkfPathStrategy, OkfResourceMapping, OkfTermRendering, OkfValueMode,
};
use crate::native_codecs::okf::OkfBundle;
use crate::projections::{ProjectionError, ProjectionPackage, ProjectionTerm, stable_identifier};

const LOSS_NAMED_GRAPH_DROPPED: &str = "named-graph-dropped";
const LOSS_NON_PROFILE_QUAD_DROPPED: &str = "okf-non-profile-quad-dropped";
const LOSS_REIFIER_DROPPED: &str = "okf-reifier-dropped";
const LOSS_ANNOTATION_DROPPED: &str = "okf-annotation-dropped";

const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// Deterministic execution counts for one OKF terms projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OkfGenerationReport {
    /// Source named-graph declarations, quads, reifiers, and annotations examined.
    pub source_records: usize,
    /// Source quads inside the caller-selected graph scope.
    pub scoped_quads: usize,
    /// Selected concept documents.
    pub concepts: usize,
    /// Configured category indexes.
    pub categories: usize,
    /// Distinct mapped frontmatter values emitted.
    pub frontmatter_values: usize,
    /// Distinct body values emitted.
    pub body_values: usize,
    /// Distinct Markdown links emitted.
    pub links: usize,
}

/// Caller-curated OKF bundle, filesystem-free package, counts, and located losses.
#[derive(Debug, Clone)]
pub struct OkfProjection {
    /// Validated deterministic Markdown knowledge bundle.
    pub bundle: OkfBundle,
    /// The same documents as a bounded package suitable for canonical USTAR.
    pub package: ProjectionPackage,
    /// Deterministic execution counts.
    pub report: OkfGenerationReport,
    /// Located losses for every source row not carried exactly by the bundle.
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
enum YamlScalar {
    String(String),
    Boolean(bool),
    Number(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum YamlField {
    Scalar(YamlScalar),
    Sequence(Vec<YamlScalar>),
}

#[derive(Debug, Clone)]
struct Frontmatter {
    document_type: String,
    title: Option<YamlField>,
    description: Option<YamlField>,
    resource: Option<YamlField>,
    tags: Option<YamlField>,
    timestamp: Option<YamlField>,
    extensions: BTreeMap<String, YamlField>,
}

impl Frontmatter {
    fn title_text(&self) -> Option<&str> {
        match &self.title {
            Some(YamlField::Scalar(YamlScalar::String(value))) => Some(value),
            _ => None,
        }
    }

    fn description_text(&self) -> Option<&str> {
        match &self.description {
            Some(YamlField::Scalar(YamlScalar::String(value))) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct ConceptDocument {
    subject: ProjectionTerm,
    category_key: String,
    path: String,
    frontmatter: Frontmatter,
    body: String,
}

struct Projector<'a> {
    config: &'a OkfGenerationConfig,
    named_graphs: Vec<ProjectionTerm>,
    quads: Vec<SourceQuad>,
    reifiers: Vec<SourceReifier>,
    annotations: Vec<SourceAnnotation>,
    by_subject: BTreeMap<ProjectionTerm, Vec<usize>>,
    consumed_quads: Vec<bool>,
    ledger: LossLedger,
    contract: LossLedger,
    frontmatter_values: usize,
    body_values: usize,
    links: usize,
}

/// Project any RDF 1.2 dataset backend into a deterministic caller-curated OKF bundle.
///
/// Category classification, graph scope, document paths, frontmatter, body/link
/// layout, index prose, and every resource bound are mandatory caller policy.
/// Backend-local ids are resolved before sorting, so interning order and storage
/// representation cannot affect output bytes.
///
/// # Errors
///
/// Returns a typed configuration, term, integrity, cardinality, syntax, package,
/// or resource-limit failure.
pub fn project_okf_terms<D: DatasetView>(
    view: &D,
    config: &OkfGenerationConfig,
) -> Result<OkfProjection, ProjectionError> {
    Projector::load(view, config)?.project()
}

impl<'a> Projector<'a> {
    fn load<D: DatasetView>(
        view: &D,
        config: &'a OkfGenerationConfig,
    ) -> Result<Self, ProjectionError> {
        config.validate()?;
        let mut cache = BTreeMap::new();

        let mut named_graphs = Vec::new();
        for graph in view.named_graphs() {
            named_graphs.push(resolve_term(view, graph, config, &mut cache)?);
        }
        named_graphs.sort();
        reject_duplicates(&named_graphs, "named graph declarations")?;

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
        reject_duplicates(&annotations, "RDF annotations")?;

        let source_records = named_graphs
            .len()
            .checked_add(quads.len())
            .and_then(|count| count.checked_add(reifiers.len()))
            .and_then(|count| count.checked_add(annotations.len()))
            .ok_or_else(|| ProjectionError::limit("OKF source record count overflow"))?;
        if source_records > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "OKF input has {source_records} source records; limit is {}",
                config.max_records()
            )));
        }

        let mut by_subject = BTreeMap::<ProjectionTerm, Vec<usize>>::new();
        for (index, quad) in quads.iter().enumerate() {
            by_subject
                .entry(quad.subject.clone())
                .or_default()
                .push(index);
        }
        let consumed_quads = vec![false; quads.len()];

        Ok(Self {
            config,
            named_graphs,
            quads,
            reifiers,
            annotations,
            by_subject,
            consumed_quads,
            ledger: LossLedger::new(),
            contract: rdf_to_okf_loss_ledger(),
            frontmatter_values: 0,
            body_values: 0,
            links: 0,
        })
    }

    fn project(mut self) -> Result<OkfProjection, ProjectionError> {
        let scoped_quads = self
            .quads
            .iter()
            .filter(|quad| self.graph_selected(quad.graph.as_ref()))
            .count();
        let mut documents = self.classify_concepts()?;
        self.assign_paths(&mut documents)?;
        self.populate_frontmatter(&mut documents)?;
        self.populate_bodies(&mut documents)?;
        let bundle = self.render_bundle(&documents)?;
        let package = ProjectionPackage::from_artifacts(
            self.config.limits(),
            bundle
                .documents()
                .map(|(path, document)| (path.to_owned(), document.as_bytes().to_vec())),
        )?;

        self.record_source_losses()?;
        check_ledger_sound(&self.ledger, "rdf-1.2-dataset", "okf")
            .map_err(ProjectionError::integrity)?;

        let report = OkfGenerationReport {
            source_records: self.named_graphs.len()
                + self.quads.len()
                + self.reifiers.len()
                + self.annotations.len(),
            scoped_quads,
            concepts: documents.len(),
            categories: self.config.categories().len(),
            frontmatter_values: self.frontmatter_values,
            body_values: self.body_values,
            links: self.links,
        };
        Ok(OkfProjection {
            bundle,
            package,
            report,
            loss_ledger: self.ledger,
        })
    }

    fn classify_concepts(&mut self) -> Result<Vec<ConceptDocument>, ProjectionError> {
        let candidates: BTreeSet<ProjectionTerm> = self
            .quads
            .iter()
            .filter(|quad| self.graph_selected(quad.graph.as_ref()))
            .map(|quad| quad.subject.clone())
            .collect();
        let mut documents = Vec::new();
        for subject in candidates {
            let mut matching = self
                .config
                .categories()
                .iter()
                .filter(|(_, category)| self.selector_matches(category.selector(), &subject))
                .map(|(key, _)| key.as_str());
            let Some(category_key) = matching.next() else {
                continue;
            };
            if let Some(second_key) = matching.next() {
                let mut matching_keys = format!("{category_key}, {second_key}");
                for key in matching {
                    write!(matching_keys, ", {key}")
                        .expect("writing category keys to a String cannot fail");
                }
                return Err(ProjectionError::integrity(format!(
                    "OKF subject {} matches multiple categories: {}",
                    term_text(&subject, OkfTermRendering::Canonical, self.config)?,
                    matching_keys
                )));
            }
            if documents.len() >= self.config.max_concepts() {
                return Err(ProjectionError::limit(format!(
                    "OKF projection exceeds its {}-concept limit",
                    self.config.max_concepts()
                )));
            }
            self.consume_classifier_evidence(&subject, category_key);
            let category = &self.config.categories()[category_key];
            documents.push(ConceptDocument {
                subject,
                category_key: category_key.to_owned(),
                path: String::new(),
                frontmatter: Frontmatter {
                    document_type: category.document_type().to_owned(),
                    title: None,
                    description: None,
                    resource: None,
                    tags: None,
                    timestamp: None,
                    extensions: BTreeMap::new(),
                },
                body: String::new(),
            });
        }
        Ok(documents)
    }

    fn selector_matches(&self, selector: &OkfConceptSelector, subject: &ProjectionTerm) -> bool {
        if !selector.iri_prefixes().is_empty() {
            let ProjectionTerm::Iri { value } = subject else {
                return false;
            };
            if !selector
                .iri_prefixes()
                .iter()
                .any(|prefix| value.starts_with(prefix))
            {
                return false;
            }
        }
        let Some(type_predicate) = selector.type_predicate() else {
            return true;
        };
        (selector.any_types().is_empty()
            || selector
                .any_types()
                .iter()
                .any(|value| self.subject_has_type(subject, type_predicate, value)))
            && selector
                .all_types()
                .iter()
                .all(|value| self.subject_has_type(subject, type_predicate, value))
            && selector
                .none_types()
                .iter()
                .all(|value| !self.subject_has_type(subject, type_predicate, value))
    }

    fn subject_has_type(&self, subject: &ProjectionTerm, predicate: &str, expected: &str) -> bool {
        self.subject_indices(subject).iter().any(|&index| {
            let quad = &self.quads[index];
            self.graph_selected(quad.graph.as_ref())
                && quad.predicate == predicate
                && matches!(&quad.object, ProjectionTerm::Iri { value } if value == expected)
        })
    }

    fn consume_classifier_evidence(&mut self, subject: &ProjectionTerm, category_key: &str) {
        let selector = self.config.categories()[category_key].selector();
        let Some(predicate) = selector.type_predicate() else {
            return;
        };
        let selection = self.config.graph_selection();
        let indices = self.by_subject.get(subject).map_or(&[][..], Vec::as_slice);
        let quads = &self.quads;
        let consumed = &mut self.consumed_quads;
        for &index in indices {
            let quad = &quads[index];
            if Self::graph_is_selected(selection, quad.graph.as_ref())
                && quad.predicate == predicate
                && matches!(
                    &quad.object,
                    ProjectionTerm::Iri { value }
                        if selector.any_types().contains(value)
                            || selector.all_types().contains(value)
                )
            {
                consumed[index] = true;
            }
        }
    }

    fn assign_paths(&mut self, documents: &mut [ConceptDocument]) -> Result<(), ProjectionError> {
        let mut owners = BTreeMap::<String, ProjectionTerm>::new();
        for document in documents.iter_mut() {
            let stem = match self.config.path_strategy() {
                OkfPathStrategy::SubjectLocalName => subject_local_name(&document.subject)?,
                OkfPathStrategy::Predicate {
                    predicate,
                    rendering,
                } => self.mapped_path_stem(&document.subject, predicate, *rendering)?,
                OkfPathStrategy::StableHash { prefix } => stable_identifier(
                    prefix,
                    &document.subject.to_canonical_json(self.config.limits())?,
                )?,
            };
            validate_path_stem(&stem, "OKF concept path stem")?;
            let directory = self.config.categories()[&document.category_key].directory();
            let path = format!("{directory}/{stem}.md");
            if let Some(first) = owners.insert(path.clone(), document.subject.clone()) {
                return Err(ProjectionError::integrity(format!(
                    "OKF concept path collision `{path}` between {} and {}",
                    term_text(&first, OkfTermRendering::Canonical, self.config)?,
                    term_text(&document.subject, OkfTermRendering::Canonical, self.config)?
                )));
            }
            document.path = path;
        }
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(())
    }

    fn mapped_path_stem(
        &mut self,
        subject: &ProjectionTerm,
        predicate: &str,
        rendering: OkfTermRendering,
    ) -> Result<String, ProjectionError> {
        let selection = self.config.graph_selection();
        let indices = self.by_subject.get(subject).map_or(&[][..], Vec::as_slice);
        let quads = &self.quads;
        let consumed = &mut self.consumed_quads;
        let mut values = BTreeSet::new();
        for &index in indices {
            let quad = &quads[index];
            if Self::graph_is_selected(selection, quad.graph.as_ref())
                && quad.predicate == predicate
            {
                values.insert(quad.object.clone());
                consumed[index] = true;
            }
        }
        if values.len() != 1 {
            return Err(ProjectionError::integrity(format!(
                "OKF path predicate `{predicate}` for {} must have exactly one distinct value; found {}",
                term_text(subject, OkfTermRendering::Canonical, self.config)?,
                values.len()
            )));
        }
        let value = term_text(
            values.first().expect("one mapped path value"),
            rendering,
            self.config,
        )?;
        Ok(value)
    }

    fn populate_frontmatter(
        &mut self,
        documents: &mut [ConceptDocument],
    ) -> Result<(), ProjectionError> {
        let mappings = self.config.frontmatter().clone();
        for document in documents {
            document.frontmatter.title =
                self.map_optional_field(&document.subject, "title", mappings.title())?;
            document.frontmatter.description =
                self.map_optional_field(&document.subject, "description", mappings.description())?;
            document.frontmatter.resource = match mappings.resource() {
                OkfResourceMapping::Omit => None,
                OkfResourceMapping::Subject => {
                    let ProjectionTerm::Iri { value } = &document.subject else {
                        return Err(ProjectionError::integrity(format!(
                            "OKF resource policy requires an IRI subject for `{}`",
                            document.path
                        )));
                    };
                    self.frontmatter_values += 1;
                    Some(YamlField::Scalar(YamlScalar::String(value.clone())))
                }
                OkfResourceMapping::Predicate { mapping } => {
                    self.map_field(&document.subject, "resource", mapping)?
                }
            };
            document.frontmatter.tags =
                self.map_optional_field(&document.subject, "tags", mappings.tags())?;
            document.frontmatter.timestamp =
                self.map_optional_field(&document.subject, "timestamp", mappings.timestamp())?;
            for (key, mapping) in mappings.extensions() {
                if let Some(value) = self.map_field(&document.subject, key, mapping)? {
                    document.frontmatter.extensions.insert(key.clone(), value);
                }
            }
        }
        Ok(())
    }

    fn map_optional_field(
        &mut self,
        subject: &ProjectionTerm,
        field: &str,
        mapping: Option<&OkfFieldMapping>,
    ) -> Result<Option<YamlField>, ProjectionError> {
        mapping.map_or(Ok(None), |mapping| self.map_field(subject, field, mapping))
    }

    fn map_field(
        &mut self,
        subject: &ProjectionTerm,
        field: &str,
        mapping: &OkfFieldMapping,
    ) -> Result<Option<YamlField>, ProjectionError> {
        let indices = self.matching_indices(subject, mapping.predicates());
        let mut values = BTreeSet::new();
        for &index in &indices {
            values.insert(yaml_scalar(
                &self.quads[index].object,
                mapping.value_mode(),
                self.config,
                field,
            )?);
        }
        self.enforce_value_limit(values.len(), field, subject)?;
        let value = match mapping.cardinality() {
            OkfCardinality::ZeroOrOne => match values.len() {
                0 => None,
                1 => Some(YamlField::Scalar(
                    values.into_iter().next().expect("one field value"),
                )),
                count => {
                    return Err(self.cardinality_error(field, subject, "at most one", count)?);
                }
            },
            OkfCardinality::One => match values.len() {
                1 => Some(YamlField::Scalar(
                    values.into_iter().next().expect("one field value"),
                )),
                count => {
                    return Err(self.cardinality_error(field, subject, "exactly one", count)?);
                }
            },
            OkfCardinality::Many => {
                (!values.is_empty()).then(|| YamlField::Sequence(values.into_iter().collect()))
            }
        };
        if let Some(value) = &value {
            self.frontmatter_values = self
                .frontmatter_values
                .checked_add(field_value_len(value))
                .ok_or_else(|| ProjectionError::limit("OKF frontmatter value count overflow"))?;
            for index in indices {
                self.consumed_quads[index] = true;
            }
        }
        Ok(value)
    }

    fn populate_bodies(
        &mut self,
        documents: &mut [ConceptDocument],
    ) -> Result<(), ProjectionError> {
        let targets: BTreeMap<ProjectionTerm, (String, String)> = documents
            .iter()
            .map(|document| {
                let title = document
                    .frontmatter
                    .title_text()
                    .map_or_else(|| lexical_term_text(&document.subject), str::to_owned);
                (document.subject.clone(), (document.path.clone(), title))
            })
            .collect();
        let body_sections = self.config.body_sections().to_vec();
        let link_sections = self.config.link_sections().to_vec();

        for document in documents {
            let mut chunks = Vec::new();
            for section in &body_sections {
                let indices = self.matching_indices(&document.subject, section.predicates());
                let mut values = BTreeSet::new();
                for &index in &indices {
                    let value =
                        match section.value_mode() {
                            OkfBodyValueMode::Text { rendering } => escape_markdown_text(
                                &term_text(&self.quads[index].object, rendering, self.config)?,
                            ),
                            OkfBodyValueMode::MarkdownLiteral => {
                                markdown_literal(&self.quads[index].object, &document.path)?
                            }
                        };
                    values.insert(value);
                }
                self.enforce_value_limit(values.len(), "body section", &document.subject)?;
                if values.is_empty() {
                    continue;
                }
                for index in indices {
                    self.consumed_quads[index] = true;
                }
                self.body_values = self
                    .body_values
                    .checked_add(values.len())
                    .ok_or_else(|| ProjectionError::limit("OKF body value count overflow"))?;
                chunks.push(render_body_section(
                    section.heading(),
                    section.style(),
                    values,
                ));
            }
            for section in &link_sections {
                let indices = self.matching_indices(&document.subject, section.predicates());
                let mut rendered = BTreeSet::new();
                let mut represented = Vec::new();
                for &index in &indices {
                    let target = &self.quads[index].object;
                    let link = if let Some((target_path, title)) = targets.get(target) {
                        let destination = match section.path_style() {
                            OkfLinkPathStyle::Relative => {
                                relative_link(&document.path, target_path)
                            }
                            OkfLinkPathStyle::BundleAbsolute => format!("/{target_path}"),
                        };
                        Some((title.clone(), destination, false))
                    } else if section.targets() == OkfLinkTargetMode::IncludeExternalIris {
                        match target {
                            ProjectionTerm::Iri { value } => {
                                Some((value.clone(), value.clone(), true))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    if let Some((title, destination, external)) = link {
                        rendered.insert(render_markdown_link(&title, &destination, external));
                        represented.push(index);
                    }
                }
                self.enforce_value_limit(rendered.len(), "link section", &document.subject)?;
                if rendered.is_empty() {
                    continue;
                }
                for index in represented {
                    self.consumed_quads[index] = true;
                }
                self.links = self
                    .links
                    .checked_add(rendered.len())
                    .ok_or_else(|| ProjectionError::limit("OKF link count overflow"))?;
                chunks.push(render_link_section(
                    section.heading(),
                    section.relation_label(),
                    section.style(),
                    rendered,
                ));
            }
            document.body = join_markdown_chunks(&chunks);
        }
        Ok(())
    }

    fn render_bundle(&self, documents: &[ConceptDocument]) -> Result<OkfBundle, ProjectionError> {
        let mut bundle = OkfBundle::new();
        for document in documents {
            let markdown = render_document(&document.frontmatter, &document.body)?;
            bundle
                .insert(document.path.clone(), markdown)
                .map_err(|error| ProjectionError::package(error.to_string()))?;
        }

        for (category_key, category) in self.config.categories() {
            let members: Vec<&ConceptDocument> = documents
                .iter()
                .filter(|document| document.category_key == *category_key)
                .collect();
            let index = render_category_index(category.index_heading(), &members);
            bundle
                .insert(format!("{}/index.md", category.directory()), index)
                .map_err(|error| ProjectionError::package(error.to_string()))?;
        }
        let root = render_root_index(self.config, documents);
        bundle
            .insert("index.md", root)
            .map_err(|error| ProjectionError::package(error.to_string()))?;
        Ok(bundle)
    }

    fn record_source_losses(&mut self) -> Result<(), ProjectionError> {
        for index in 0..self.named_graphs.len() {
            let subject = source_identifier("OkfTermsGraph", &self.named_graphs[index])?;
            self.record_loss(LOSS_NAMED_GRAPH_DROPPED, "okf-terms:named-graph", subject);
        }
        for index in 0..self.quads.len() {
            if self.quads[index].graph.is_some() {
                let subject = source_identifier("OkfTermsQuad", &self.quads[index])?;
                self.record_loss(LOSS_NAMED_GRAPH_DROPPED, "okf-terms:quad", subject);
            }
            if !self.consumed_quads[index] {
                let subject = source_identifier("OkfTermsQuad", &self.quads[index])?;
                self.record_loss(LOSS_NON_PROFILE_QUAD_DROPPED, "okf-terms:quad", subject);
            }
        }
        for index in 0..self.reifiers.len() {
            let subject = source_identifier("OkfTermsReifier", &self.reifiers[index])?;
            self.record_loss(LOSS_REIFIER_DROPPED, "okf-terms:reifier", subject);
        }
        for index in 0..self.annotations.len() {
            let subject = source_identifier("OkfTermsAnnotation", &self.annotations[index])?;
            self.record_loss(LOSS_ANNOTATION_DROPPED, "okf-terms:annotation", subject);
        }
        Ok(())
    }

    fn record_loss(&mut self, code: &'static str, logical: &str, subject: String) {
        let template = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .expect("runtime OKF terms code must exist in the closed contract");
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

    fn graph_selected(&self, graph: Option<&ProjectionTerm>) -> bool {
        Self::graph_is_selected(self.config.graph_selection(), graph)
    }

    fn graph_is_selected(selection: &OkfGraphSelection, graph: Option<&ProjectionTerm>) -> bool {
        match selection {
            OkfGraphSelection::All => true,
            selection @ OkfGraphSelection::Include { .. } => match graph {
                None => selection.includes_default_graph(),
                Some(ProjectionTerm::Iri { value }) => selection.includes_named_graph(value),
                Some(
                    ProjectionTerm::Blank { .. }
                    | ProjectionTerm::Literal { .. }
                    | ProjectionTerm::Triple { .. },
                ) => false,
            },
        }
    }

    fn subject_indices(&self, subject: &ProjectionTerm) -> &[usize] {
        self.by_subject.get(subject).map_or(&[], Vec::as_slice)
    }

    fn matching_indices(
        &self,
        subject: &ProjectionTerm,
        predicates: &BTreeSet<String>,
    ) -> Vec<usize> {
        self.subject_indices(subject)
            .iter()
            .copied()
            .filter(|&index| {
                let quad = &self.quads[index];
                self.graph_selected(quad.graph.as_ref()) && predicates.contains(&quad.predicate)
            })
            .collect()
    }

    fn enforce_value_limit(
        &self,
        count: usize,
        field: &str,
        subject: &ProjectionTerm,
    ) -> Result<(), ProjectionError> {
        if count > self.config.max_values_per_field() {
            return Err(ProjectionError::limit(format!(
                "OKF `{field}` for {} has {count} distinct values; limit is {}",
                term_text(subject, OkfTermRendering::Canonical, self.config)?,
                self.config.max_values_per_field()
            )));
        }
        Ok(())
    }

    fn cardinality_error(
        &self,
        field: &str,
        subject: &ProjectionTerm,
        expected: &str,
        actual: usize,
    ) -> Result<ProjectionError, ProjectionError> {
        Ok(ProjectionError::integrity(format!(
            "OKF `{field}` for {} requires {expected} distinct value(s); found {actual}",
            term_text(subject, OkfTermRendering::Canonical, self.config)?
        )))
    }
}

fn yaml_scalar(
    term: &ProjectionTerm,
    mode: OkfValueMode,
    config: &OkfGenerationConfig,
    field: &str,
) -> Result<YamlScalar, ProjectionError> {
    match mode {
        OkfValueMode::Text { rendering } => {
            Ok(YamlScalar::String(term_text(term, rendering, config)?))
        }
        OkfValueMode::Iri => match term {
            ProjectionTerm::Iri { value } => Ok(YamlScalar::String(value.clone())),
            _ => Err(ProjectionError::term(format!(
                "OKF `{field}` requires an IRI object"
            ))),
        },
        OkfValueMode::Boolean => {
            let value = typed_xsd_value(term, field)?;
            let XsdValue::Boolean(value) = value else {
                return Err(ProjectionError::term(format!(
                    "OKF `{field}` requires an `{XSD_BOOLEAN}` literal"
                )));
            };
            Ok(YamlScalar::Boolean(value))
        }
        OkfValueMode::Integer => {
            let value = typed_xsd_value(term, field)?;
            let XsdValue::Integer { .. } = value else {
                return Err(ProjectionError::term(format!(
                    "OKF `{field}` requires an XSD integer-family literal"
                )));
            };
            Ok(YamlScalar::Number(value.canonical_lexical()))
        }
        OkfValueMode::Decimal => {
            let value = typed_xsd_value(term, field)?;
            let XsdValue::Decimal(_) = value else {
                return Err(ProjectionError::term(format!(
                    "OKF `{field}` requires an `{XSD_DECIMAL}` literal"
                )));
            };
            Ok(YamlScalar::Number(value.canonical_lexical()))
        }
        OkfValueMode::DateTime => {
            let value = typed_xsd_value(term, field)?;
            let XsdValue::DateTime(_) = value else {
                return Err(ProjectionError::term(format!(
                    "OKF `{field}` requires an `{XSD_DATETIME}` literal"
                )));
            };
            Ok(YamlScalar::String(value.canonical_lexical()))
        }
    }
}

fn typed_xsd_value(term: &ProjectionTerm, field: &str) -> Result<XsdValue, ProjectionError> {
    let ProjectionTerm::Literal {
        lexical,
        datatype,
        language,
        direction,
    } = term
    else {
        return Err(ProjectionError::term(format!(
            "OKF `{field}` typed value requires a literal object"
        )));
    };
    if language.is_some() || direction.is_some() {
        return Err(ProjectionError::term(format!(
            "OKF `{field}` typed value cannot carry language or base direction"
        )));
    }
    purrdf_xsd::parse_by_iri(lexical, datatype)
        .map_err(|error| {
            ProjectionError::term(format!(
                "invalid OKF `{field}` literal `{lexical}`^^`{datatype}`: {error}"
            ))
        })?
        .ok_or_else(|| {
            ProjectionError::term(format!(
                "OKF `{field}` datatype `{datatype}` is not a recognized XSD value type"
            ))
        })
}

fn term_text(
    term: &ProjectionTerm,
    rendering: OkfTermRendering,
    config: &OkfGenerationConfig,
) -> Result<String, ProjectionError> {
    match rendering {
        OkfTermRendering::Lexical => Ok(lexical_term_text(term)),
        OkfTermRendering::Canonical => String::from_utf8(term.to_canonical_json(config.limits())?)
            .map_err(|error| {
                ProjectionError::integrity(format!(
                    "canonical OKF RDF term JSON is not UTF-8: {error}"
                ))
            }),
    }
}

fn lexical_term_text(term: &ProjectionTerm) -> String {
    match term {
        ProjectionTerm::Iri { value } => value.clone(),
        ProjectionTerm::Blank { label, scope } => format!("_:{scope}:{label}"),
        ProjectionTerm::Literal { lexical, .. } => lexical.clone(),
        ProjectionTerm::Triple {
            subject,
            predicate,
            object,
        } => format!(
            "<< {} {} {} >>",
            lexical_term_text(subject),
            lexical_term_text(predicate),
            lexical_term_text(object)
        ),
    }
}

fn subject_local_name(subject: &ProjectionTerm) -> Result<String, ProjectionError> {
    let ProjectionTerm::Iri { value } = subject else {
        return Err(ProjectionError::term(
            "OKF subject-local-name path strategy requires IRI concept subjects",
        ));
    };
    let stem = value
        .rsplit(['#', '/', ':'])
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ProjectionError::term(format!(
                "OKF concept IRI `{value}` has no non-empty local name"
            ))
        })?;
    Ok(stem.to_owned())
}

fn markdown_literal(term: &ProjectionTerm, path: &str) -> Result<String, ProjectionError> {
    let ProjectionTerm::Literal { lexical, .. } = term else {
        return Err(ProjectionError::term(format!(
            "OKF Markdown-literal body mapping in `{path}` requires literal objects"
        )));
    };
    if lexical.contains('\0') {
        return Err(ProjectionError::term(format!(
            "OKF Markdown-literal body mapping in `{path}` contains NUL"
        )));
    }
    Ok(lexical.clone())
}

fn render_document(frontmatter: &Frontmatter, body: &str) -> Result<String, ProjectionError> {
    let mut output = String::from("---\n");
    render_yaml_entry(
        &mut output,
        "type",
        &YamlField::Scalar(YamlScalar::String(frontmatter.document_type.clone())),
    )?;
    for (key, value) in [
        ("title", frontmatter.title.as_ref()),
        ("description", frontmatter.description.as_ref()),
        ("resource", frontmatter.resource.as_ref()),
        ("tags", frontmatter.tags.as_ref()),
        ("timestamp", frontmatter.timestamp.as_ref()),
    ] {
        if let Some(value) = value {
            render_yaml_entry(&mut output, key, value)?;
        }
    }
    for (key, value) in &frontmatter.extensions {
        render_yaml_entry(&mut output, key, value)?;
    }
    output.push_str("---\n");
    output.push_str(body);
    Ok(output)
}

fn render_yaml_entry(
    output: &mut String,
    key: &str,
    value: &YamlField,
) -> Result<(), ProjectionError> {
    output.push_str(key);
    match value {
        YamlField::Scalar(value) => {
            output.push_str(": ");
            output.push_str(&render_yaml_scalar(value)?);
            output.push('\n');
        }
        YamlField::Sequence(values) => {
            output.push_str(":\n");
            for value in values {
                output.push_str("  - ");
                output.push_str(&render_yaml_scalar(value)?);
                output.push('\n');
            }
        }
    }
    Ok(())
}

fn render_yaml_scalar(value: &YamlScalar) -> Result<String, ProjectionError> {
    match value {
        YamlScalar::String(value) => serde_json::to_string(value).map_err(|error| {
            ProjectionError::integrity(format!("serialize OKF YAML string scalar: {error}"))
        }),
        YamlScalar::Boolean(value) => Ok(value.to_string()),
        YamlScalar::Number(value) => Ok(value.clone()),
    }
}

fn render_body_section(
    heading: Option<&str>,
    style: OkfBodyStyle,
    values: BTreeSet<String>,
) -> String {
    let mut lines = Vec::new();
    if let Some(heading) = heading {
        lines.push(format!("## {heading}"));
        lines.push(String::new());
    }
    match style {
        OkfBodyStyle::Paragraphs => {
            for value in values {
                lines.push(value);
                lines.push(String::new());
            }
        }
        OkfBodyStyle::Bullets => {
            lines.extend(values.into_iter().map(|value| format!("- {value}")));
        }
    }
    trim_blank_lines(lines).join("\n")
}

fn render_link_section(
    heading: Option<&str>,
    relation_label: Option<&str>,
    style: OkfLinkStyle,
    links: BTreeSet<String>,
) -> String {
    let mut lines = Vec::new();
    if let Some(heading) = heading {
        lines.push(format!("## {heading}"));
        lines.push(String::new());
    }
    let prefix = relation_label.map_or(String::new(), |label| format!("{label}: "));
    for link in links {
        match style {
            OkfLinkStyle::Bullets => lines.push(format!("- {prefix}{link}")),
            OkfLinkStyle::Paragraphs => {
                lines.push(format!("{prefix}{link}"));
                lines.push(String::new());
            }
        }
    }
    trim_blank_lines(lines).join("\n")
}

fn join_markdown_chunks(chunks: &[String]) -> String {
    let chunks: Vec<&str> = chunks
        .iter()
        .map(String::as_str)
        .filter(|chunk| !chunk.is_empty())
        .collect();
    if chunks.is_empty() {
        String::new()
    } else {
        format!("{}\n", chunks.join("\n\n"))
    }
}

fn trim_blank_lines(mut lines: Vec<String>) -> Vec<String> {
    while lines.last().is_some_and(String::is_empty) {
        let _ = lines.pop();
    }
    lines
}

fn render_category_index(heading: &str, members: &[&ConceptDocument]) -> String {
    let mut output = format!("# {heading}\n");
    for member in members {
        let filename = member
            .path
            .rsplit_once('/')
            .map_or(member.path.as_str(), |(_, filename)| filename);
        let title = member
            .frontmatter
            .title_text()
            .map_or_else(|| filename.trim_end_matches(".md"), str::trim);
        output.push_str("\n* [");
        output.push_str(&escape_link_text(title));
        output.push_str("](");
        output.push_str(filename);
        output.push(')');
        if let Some(description) = member.frontmatter.description_text() {
            output.push_str(" - ");
            output.push_str(&escape_markdown_text(description));
        }
        output.push('\n');
    }
    output
}

fn render_root_index(config: &OkfGenerationConfig, documents: &[ConceptDocument]) -> String {
    let index = config.index();
    let mut output = format!(
        "# {}\n\n## {}\n\n{}",
        index.root_heading(),
        index.fidelity_heading(),
        index.loss_declaration()
    );
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("\n## ");
    output.push_str(index.categories_heading());
    output.push('\n');
    for (key, category) in config.categories() {
        let count = documents
            .iter()
            .filter(|document| document.category_key == *key)
            .count();
        output.push_str("\n* [");
        output.push_str(&escape_link_text(category.index_heading()));
        output.push_str("](");
        output.push_str(category.directory());
        output.push_str("/index.md) - ");
        output.push_str(&escape_markdown_text(category.index_description()));
        output.push_str(" (");
        output.push_str(&count.to_string());
        output.push_str(")\n");
    }
    output
}

fn render_markdown_link(title: &str, destination: &str, external: bool) -> String {
    let destination = if external {
        format!("<{}>", destination.replace('\\', "%5C").replace('>', "%3E"))
    } else {
        destination.to_owned()
    };
    format!("[{}]({destination})", escape_link_text(title))
}

fn escape_link_text(value: &str) -> String {
    flatten_controls(value)
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_text(value: &str) -> String {
    let flattened = flatten_controls(value);
    let mut output = String::with_capacity(flattened.len());
    for character in flattened.chars() {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '('
                | ')'
                | '#'
                | '+'
                | '-'
                | '!'
                | '|'
        ) {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

fn flatten_controls(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\n' | '\r' | '\t' => output.push(' '),
            control if control.is_control() => {
                let _ = write!(output, "\\u{{{:x}}}", control as u32);
            }
            other => output.push(other),
        }
    }
    output
}

fn relative_link(from_path: &str, to_path: &str) -> String {
    let from_parent: Vec<&str> = from_path
        .rsplit_once('/')
        .map_or(Vec::new(), |(parent, _)| parent.split('/').collect());
    let target: Vec<&str> = to_path.split('/').collect();
    let target_parent_len = target.len().saturating_sub(1);
    let common = from_parent
        .iter()
        .zip(target.iter().take(target_parent_len))
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; from_parent.len().saturating_sub(common)];
    parts.extend_from_slice(&target[common..]);
    parts.join("/")
}

fn field_value_len(value: &YamlField) -> usize {
    match value {
        YamlField::Scalar(_) => 1,
        YamlField::Sequence(values) => values.len(),
    }
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    config: &OkfGenerationConfig,
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

fn source_identifier(prefix: &str, value: &impl Serialize) -> Result<String, ProjectionError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ProjectionError::integrity(format!("serialize OKF terms source location: {error}"))
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, assert_ledger_sound};

    use super::*;
    use crate::projections::{
        OkfBodySection, OkfCategory, OkfFrontmatterMappings, OkfIndexConfig, OkfLinkSection,
        ProjectionLimits,
    };

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
    const CLASS: &str = "https://example.org/Class";
    const PROPERTY: &str = "https://example.org/Property";
    const LABEL: &str = "https://example.org/label";
    const DESCRIPTION: &str = "https://example.org/description";
    const TAG: &str = "https://example.org/tag";
    const RELATED: &str = "https://example.org/related";

    fn mapping(
        predicate: &str,
        cardinality: OkfCardinality,
        mode: OkfValueMode,
    ) -> OkfFieldMapping {
        OkfFieldMapping::new(BTreeSet::from([predicate.to_owned()]), cardinality, mode)
            .expect("mapping")
    }

    fn selector(class: &str) -> OkfConceptSelector {
        OkfConceptSelector::new(
            Some(RDF_TYPE.to_owned()),
            BTreeSet::from([class.to_owned()]),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::from(["https://example.org/".to_owned()]),
        )
        .expect("selector")
    }

    fn config() -> OkfGenerationConfig {
        config_with(
            BTreeMap::from([
                (
                    "class".to_owned(),
                    OkfCategory::new(
                        "classes",
                        "Class",
                        "Classes",
                        "Ontology classes.",
                        selector(CLASS),
                    )
                    .expect("class category"),
                ),
                (
                    "property".to_owned(),
                    OkfCategory::new(
                        "properties",
                        "Property",
                        "Properties",
                        "Ontology properties.",
                        selector(PROPERTY),
                    )
                    .expect("property category"),
                ),
            ]),
            OkfPathStrategy::SubjectLocalName,
        )
    }

    fn config_with(
        categories: BTreeMap<String, OkfCategory>,
        path_strategy: OkfPathStrategy,
    ) -> OkfGenerationConfig {
        let text = OkfValueMode::Text {
            rendering: OkfTermRendering::Lexical,
        };
        OkfGenerationConfig::new(
            OkfGraphSelection::All,
            categories,
            path_strategy,
            OkfFrontmatterMappings::new(
                Some(mapping(LABEL, OkfCardinality::ZeroOrOne, text)),
                Some(mapping(DESCRIPTION, OkfCardinality::ZeroOrOne, text)),
                OkfResourceMapping::Subject,
                Some(mapping(TAG, OkfCardinality::Many, text)),
                None,
                BTreeMap::from([(
                    "identity".to_owned(),
                    mapping(RDF_TYPE, OkfCardinality::Many, OkfValueMode::Iri),
                )]),
            )
            .expect("frontmatter"),
            vec![
                OkfBodySection::new(
                    None,
                    BTreeSet::from([DESCRIPTION.to_owned()]),
                    OkfBodyStyle::Paragraphs,
                    OkfBodyValueMode::Text {
                        rendering: OkfTermRendering::Lexical,
                    },
                )
                .expect("body"),
            ],
            vec![
                OkfLinkSection::new(
                    Some("Relations".to_owned()),
                    BTreeSet::from([RELATED.to_owned()]),
                    Some("related".to_owned()),
                    OkfLinkStyle::Bullets,
                    OkfLinkPathStyle::Relative,
                    OkfLinkTargetMode::InternalOnly,
                )
                .expect("links"),
            ],
            OkfIndexConfig::new(
                "Example ontology",
                "Categories",
                "Projection fidelity",
                "Only the configured term view is represented.",
            )
            .expect("index"),
            ProjectionLimits::new(32, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits"),
            1_000,
            20,
            100,
        )
        .expect("configuration")
    }

    fn dataset(reverse: bool) -> Arc<RdfDataset> {
        let mut statements = vec![
            ("A", RDF_TYPE, CLASS, true),
            ("A", LABEL, "Alpha", false),
            ("A", DESCRIPTION, "An *important* class.", false),
            ("A", TAG, "zeta", false),
            ("A", TAG, "alpha", false),
            ("A", RELATED, "B", true),
            ("B", RDF_TYPE, PROPERTY, true),
            ("B", LABEL, "Beta", false),
        ];
        let mut builder = RdfDatasetBuilder::new();
        if reverse {
            statements.reverse();
        }
        for (subject, predicate, object, object_is_iri) in statements {
            let subject = builder.intern_iri(&format!("https://example.org/{subject}"));
            let predicate = builder.intern_iri(predicate);
            let object = if object_is_iri {
                let iri = if object == "B" {
                    format!("https://example.org/{object}")
                } else {
                    object.to_owned()
                };
                builder.intern_iri(&iri)
            } else {
                builder.intern_literal(RdfLiteral::typed(object, XSD_STRING))
            };
            builder.push_quad(subject, predicate, object, None);
        }
        builder.freeze().expect("dataset")
    }

    #[test]
    fn projection_is_exact_and_independent_of_interning_order() {
        let config = config();
        let first = project_okf_terms(dataset(false).as_ref(), &config).expect("project");
        let reversed = project_okf_terms(dataset(true).as_ref(), &config).expect("reproject");
        assert_eq!(first.bundle, reversed.bundle);
        assert_eq!(
            first.package.to_ustar().expect("archive"),
            reversed.package.to_ustar().expect("archive")
        );
        assert_eq!(first.report.concepts, 2);
        assert_eq!(first.report.categories, 2);
        assert_eq!(first.report.links, 1);
        assert_ledger_sound(&first.loss_ledger, "rdf-1.2-dataset", "okf");

        assert_eq!(
            first.bundle.get("classes/A.md"),
            Some(
                "---\ntype: \"Class\"\ntitle: \"Alpha\"\ndescription: \"An *important* class.\"\nresource: \"https://example.org/A\"\ntags:\n  - \"alpha\"\n  - \"zeta\"\nidentity:\n  - \"https://example.org/Class\"\n---\nAn \\*important\\* class.\n\n## Relations\n\n- related: [Beta](../properties/B.md)\n"
            )
        );
        assert_eq!(
            first.bundle.get("classes/index.md"),
            Some("# Classes\n\n* [Alpha](A.md) - An \\*important\\* class.\n")
        );
        assert_eq!(
            first.bundle.get("index.md"),
            Some(
                "# Example ontology\n\n## Projection fidelity\n\nOnly the configured term view is represented.\n\n## Categories\n\n* [Classes](classes/index.md) - Ontology classes. (1)\n\n* [Properties](properties/index.md) - Ontology properties. (1)\n"
            )
        );
    }

    #[test]
    fn total_term_rendering_covers_nested_rdf12_terms() {
        let term = ProjectionTerm::Triple {
            subject: Box::new(ProjectionTerm::Blank {
                label: "b".to_owned(),
                scope: 7,
            }),
            predicate: Box::new(ProjectionTerm::Iri {
                value: "https://example.org/p".to_owned(),
            }),
            object: Box::new(ProjectionTerm::Literal {
                lexical: "hello".to_owned(),
                datatype: RDF_LANG_STRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(crate::projections::ProjectionDirection::Ltr),
            }),
        };
        let config = config();
        assert_eq!(
            term_text(&term, OkfTermRendering::Lexical, &config).expect("lexical"),
            "<< _:7:b https://example.org/p hello >>"
        );
        let canonical = term_text(&term, OkfTermRendering::Canonical, &config).expect("canonical");
        assert!(canonical.contains("\"kind\":\"triple\""));
    }

    #[test]
    fn loss_ledger_locates_every_dropped_rdf12_record() {
        let mut builder = RdfDatasetBuilder::new();
        let concept = builder.intern_iri("https://example.org/A");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let class = builder.intern_iri(CLASS);
        let label = builder.intern_iri(LABEL);
        let title = builder.intern_literal(RdfLiteral::typed("Alpha", XSD_STRING));
        let graph = builder.intern_iri("https://example.org/source-graph");
        builder.declare_named_graph(graph);
        builder.push_quad(concept, rdf_type, class, Some(graph));
        builder.push_quad(concept, label, title, None);

        let unmapped = builder.intern_iri("https://example.org/unmapped");
        let ignored = builder.intern_iri("https://example.org/ignored");
        builder.push_quad(concept, unmapped, ignored, None);

        let quoted = builder.intern_triple(concept, unmapped, ignored);
        let reifier = builder.intern_blank("r", BlankScope(7));
        builder.push_reifier_in_graph(reifier, quoted, Some(graph));
        let confidence = builder.intern_iri("https://example.org/confidence");
        builder.push_annotation_in_graph(reifier, confidence, quoted, Some(graph));

        let dataset = builder.freeze().expect("dataset");
        let projected = project_okf_terms(&dataset, &config()).expect("project");
        assert_eq!(projected.report.source_records, 6);
        assert_eq!(projected.report.scoped_quads, 3);
        assert_eq!(projected.report.concepts, 1);
        assert_ledger_sound(&projected.loss_ledger, "rdf-1.2-dataset", "okf");
        assert!(
            projected
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );
        let counts = projected.loss_ledger.entries().iter().fold(
            BTreeMap::<&str, usize>::new(),
            |mut counts, entry| {
                *counts.entry(entry.code.as_ref()).or_default() += 1;
                counts
            },
        );
        assert_eq!(counts.get(LOSS_NAMED_GRAPH_DROPPED), Some(&2));
        assert_eq!(counts.get(LOSS_NON_PROFILE_QUAD_DROPPED), Some(&1));
        assert_eq!(counts.get(LOSS_REIFIER_DROPPED), Some(&1));
        assert_eq!(counts.get(LOSS_ANNOTATION_DROPPED), Some(&1));
    }

    #[test]
    fn hard_fails_on_ambiguous_categories_path_collisions_and_cardinality() {
        let ambiguous = config_with(
            BTreeMap::from([
                (
                    "first".to_owned(),
                    OkfCategory::new("first", "First", "First", "First.", selector(CLASS))
                        .expect("first category"),
                ),
                (
                    "second".to_owned(),
                    OkfCategory::new("second", "Second", "Second", "Second.", selector(CLASS))
                        .expect("second category"),
                ),
            ]),
            OkfPathStrategy::SubjectLocalName,
        );
        let error = project_okf_terms(dataset(false).as_ref(), &ambiguous)
            .expect_err("ambiguous category must fail");
        assert!(error.to_string().contains("matches multiple categories"));

        let collision = config_with(
            BTreeMap::from([(
                "class".to_owned(),
                OkfCategory::new("classes", "Class", "Classes", "Classes.", selector(CLASS))
                    .expect("class category"),
            )]),
            OkfPathStrategy::SubjectLocalName,
        );
        let mut builder = RdfDatasetBuilder::new();
        for subject in [
            "https://example.org/one/Thing",
            "https://example.org/two/Thing",
        ] {
            let subject = builder.intern_iri(subject);
            let predicate = builder.intern_iri(RDF_TYPE);
            let object = builder.intern_iri(CLASS);
            builder.push_quad(subject, predicate, object, None);
        }
        let collision_dataset = builder.freeze().expect("collision dataset");
        let error = project_okf_terms(&collision_dataset, &collision)
            .expect_err("path collision must fail");
        assert!(error.to_string().contains("path collision"));

        let mut builder = RdfDatasetBuilder::new();
        let concept = builder.intern_iri("https://example.org/A");
        let rdf_type = builder.intern_iri(RDF_TYPE);
        let class = builder.intern_iri(CLASS);
        builder.push_quad(concept, rdf_type, class, None);
        for value in ["Alpha", "Alternate"] {
            let label = builder.intern_iri(LABEL);
            let value = builder.intern_literal(RdfLiteral::typed(value, XSD_STRING));
            builder.push_quad(concept, label, value, None);
        }
        let cardinality_dataset = builder.freeze().expect("cardinality dataset");
        let error = project_okf_terms(&cardinality_dataset, &config())
            .expect_err("scalar cardinality must fail");
        assert!(error.to_string().contains("requires at most one"));
    }
}

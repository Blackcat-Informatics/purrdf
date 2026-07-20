// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{
    DatasetView, LossLedger, QuadIds, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermRef,
};

use super::super::dataset_description::serialize_description;
use super::super::util::canonical_json_bounded;
use super::super::{
    ProjectionError, ProjectionTerm, RdfDescriptionProjection, stable_identifier,
    validate_absolute_iri,
};
use super::config::{VoidConfig, VoidDatasetPrefix, VoidGraphSelector, VoidRole, VoidStaticValue};

/// Generate one deterministic, blank-free VoID dataset description.
///
/// The selected source rows are normalized once from any [`DatasetView`]. Header
/// literals, dataset statistics, class/property partitions, metadata links, and
/// alignment linksets are then emitted directly into caller-vocabulary RDF IR.
///
/// # Errors
///
/// Returns typed configuration, term, integrity, serialization, package, or
/// resource-limit errors for ambiguous/malformed inputs or exceeded bounds.
pub fn project_void<D: DatasetView>(
    view: &D,
    config: &VoidConfig,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    let records = selected_records(view, config)?;
    let header_version = required_header_literal(
        &records,
        config,
        config.source_roles().header_version(),
        "version",
    )?;
    let header_abstract = required_header_literal(
        &records,
        config,
        config.source_roles().header_abstract(),
        "abstract",
    )?;
    let data = analyze_data(&records, config)?;
    let links = analyze_linksets(&records, config)?;
    let external_links = collect_external_links(&records, config)?;
    let dataset = emit_void(
        config,
        &header_version,
        &header_abstract,
        &data,
        &links,
        &external_links,
    )?;
    serialize_description(
        dataset,
        LossLedger::new(),
        config.format(),
        "void",
        config.limits(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceRecord {
    graph: VoidGraphSelector,
    subject: ProjectionTerm,
    predicate: String,
    object: ProjectionTerm,
}

fn selected_records<D: DatasetView>(
    view: &D,
    config: &VoidConfig,
) -> Result<BTreeSet<SourceRecord>, ProjectionError> {
    let input_records = view
        .quads()
        .count()
        .checked_add(view.named_graphs().count())
        .and_then(|count| count.checked_add(view.reifier_quads().count()))
        .and_then(|count| count.checked_add(view.annotation_quads().count()))
        .ok_or_else(|| ProjectionError::limit("VoID input record count overflow"))?;
    if input_records > config.execution_limits().max_input_records() {
        return Err(ProjectionError::limit(format!(
            "VoID input has {input_records} records; limit is {}",
            config.execution_limits().max_input_records()
        )));
    }

    let mut records = BTreeSet::new();
    for row in view
        .quads()
        .chain(view.reifier_quads())
        .chain(view.annotation_quads())
    {
        let Some(graph) = selected_graph(view, row.g, config)? else {
            continue;
        };
        records.insert(resolve_record(view, row, graph, config)?);
    }
    Ok(records)
}

fn selected_graph<D: DatasetView>(
    view: &D,
    graph: Option<D::Id>,
    config: &VoidConfig,
) -> Result<Option<VoidGraphSelector>, ProjectionError> {
    let graph_iri = match graph {
        None => None,
        Some(graph) => match view.resolve(graph) {
            TermRef::Iri(iri) => Some(iri),
            TermRef::Blank { .. } => return Ok(None),
            TermRef::Literal { .. } | TermRef::Triple { .. } => {
                return Err(ProjectionError::integrity(
                    "VoID source graph name is neither an IRI nor a blank node",
                ));
            }
        },
    };
    Ok(config
        .data_graphs()
        .iter()
        .chain([
            config.header_graph(),
            config.alignment_graph(),
            config.metadata_graph(),
        ])
        .find(|selector| match (selector, graph_iri) {
            (VoidGraphSelector::DefaultGraph, None) => true,
            (VoidGraphSelector::NamedGraph { graph_iri }, Some(actual)) => graph_iri == actual,
            (VoidGraphSelector::DefaultGraph | VoidGraphSelector::NamedGraph { .. }, _) => false,
        })
        .cloned())
}

fn resolve_record<D: DatasetView>(
    view: &D,
    row: QuadIds<D::Id>,
    graph: VoidGraphSelector,
    config: &VoidConfig,
) -> Result<SourceRecord, ProjectionError> {
    let TermRef::Iri(predicate) = view.resolve(row.p) else {
        return Err(ProjectionError::integrity(
            "VoID source predicate does not resolve to an IRI",
        ));
    };
    Ok(SourceRecord {
        graph,
        subject: ProjectionTerm::from_view(view, row.s, config.limits())?,
        predicate: predicate.to_owned(),
        object: ProjectionTerm::from_view(view, row.o, config.limits())?,
    })
}

fn required_header_literal(
    records: &BTreeSet<SourceRecord>,
    config: &VoidConfig,
    predicate: &str,
    label: &str,
) -> Result<ProjectionTerm, ProjectionError> {
    let mut values = BTreeSet::new();
    for record in records {
        if &record.graph != config.header_graph()
            || iri(&record.subject) != Some(config.header_subject_iri())
            || record.predicate != predicate
        {
            continue;
        }
        if !matches!(record.object, ProjectionTerm::Literal { .. }) {
            return Err(ProjectionError::integrity(format!(
                "VoID header {label} object must be an RDF literal"
            )));
        }
        values.insert(record.object.clone());
    }
    match values.len() {
        1 => Ok(values.into_iter().next().expect("one header value")),
        0 => Err(ProjectionError::integrity(format!(
            "VoID source header lacks required {label} literal"
        ))),
        count => Err(ProjectionError::integrity(format!(
            "VoID source header has {count} distinct {label} literals"
        ))),
    }
}

#[derive(Debug)]
struct DataAnalysis {
    aggregate: Aggregate,
    class_partitions: BTreeMap<String, Aggregate>,
    property_partitions: BTreeMap<String, Aggregate>,
}

#[derive(Debug, Default)]
struct Aggregate {
    triples: u64,
    subjects: BTreeSet<ProjectionTerm>,
    objects: BTreeSet<ProjectionTerm>,
}

impl Aggregate {
    fn record(&mut self, row: &SourceRecord, label: &str) -> Result<(), ProjectionError> {
        self.triples = self
            .triples
            .checked_add(1)
            .ok_or_else(|| ProjectionError::limit(format!("{label} triple count overflow")))?;
        self.subjects.insert(row.subject.clone());
        self.objects.insert(row.object.clone());
        Ok(())
    }
}

fn analyze_data(
    records: &BTreeSet<SourceRecord>,
    config: &VoidConfig,
) -> Result<DataAnalysis, ProjectionError> {
    let data_graphs: BTreeSet<&VoidGraphSelector> = config.data_graphs().iter().collect();
    let data: Vec<&SourceRecord> = records
        .iter()
        .filter(|record| data_graphs.contains(&record.graph))
        .collect();
    let mut aggregate = Aggregate::default();
    let mut property_partitions = BTreeMap::<String, Aggregate>::new();
    let mut class_members = BTreeMap::<String, BTreeSet<ProjectionTerm>>::new();

    for record in &data {
        aggregate.record(record, "VoID dataset")?;
        property_partitions
            .entry(record.predicate.clone())
            .or_default()
            .record(record, "VoID property partition")?;
        if record.predicate == config.source_roles().rdf_type() {
            let Some(class) = iri(&record.object) else {
                return Err(ProjectionError::integrity(
                    "VoID rdf:type object in a selected data graph must be an IRI",
                ));
            };
            class_members
                .entry(class.to_owned())
                .or_default()
                .insert(record.subject.clone());
        }
    }

    let partition_count = class_members
        .len()
        .checked_add(property_partitions.len())
        .ok_or_else(|| ProjectionError::limit("VoID partition count overflow"))?;
    if partition_count > config.execution_limits().max_partitions() {
        return Err(ProjectionError::limit(format!(
            "VoID analysis has {partition_count} partitions; limit is {}",
            config.execution_limits().max_partitions()
        )));
    }

    let mut classes_by_subject = BTreeMap::<&ProjectionTerm, Vec<&str>>::new();
    for (class, members) in &class_members {
        for member in members {
            classes_by_subject.entry(member).or_default().push(class);
        }
    }
    let mut class_partitions: BTreeMap<String, Aggregate> = class_members
        .keys()
        .cloned()
        .map(|class| (class, Aggregate::default()))
        .collect();
    let mut membership_work = 0_usize;
    for record in data {
        let Some(classes) = classes_by_subject.get(&record.subject) else {
            continue;
        };
        for class in classes {
            membership_work = membership_work.checked_add(1).ok_or_else(|| {
                ProjectionError::limit("VoID partition membership count overflow")
            })?;
            if membership_work > config.execution_limits().max_partition_memberships() {
                return Err(ProjectionError::limit(format!(
                    "VoID partition expansion exceeds the {}-membership limit",
                    config.execution_limits().max_partition_memberships()
                )));
            }
            class_partitions
                .get_mut(*class)
                .expect("class aggregate initialized")
                .record(record, "VoID class partition")?;
        }
    }

    Ok(DataAnalysis {
        aggregate,
        class_partitions,
        property_partitions,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LinksetKey {
    subjects_target: String,
    objects_target: String,
    predicate: String,
}

#[derive(Debug)]
struct Linkset {
    iri: String,
    key: LinksetKey,
    triples: u64,
}

fn analyze_linksets(
    records: &BTreeSet<SourceRecord>,
    config: &VoidConfig,
) -> Result<Vec<Linkset>, ProjectionError> {
    let mut buckets = BTreeMap::<(&str, &str, &str), u64>::new();
    for record in records
        .iter()
        .filter(|record| &record.graph == config.alignment_graph())
    {
        let Some(subject) = iri(&record.subject) else {
            return Err(ProjectionError::integrity(
                "VoID alignment subject must be an IRI",
            ));
        };
        let Some(object) = iri(&record.object) else {
            return Err(ProjectionError::integrity(
                "VoID alignment object must be an IRI",
            ));
        };
        let subject_dataset = classify_dataset(subject, config)?;
        let object_dataset = classify_dataset(object, config)?;
        if !subject_dataset.local && !object_dataset.local {
            return Err(ProjectionError::integrity(format!(
                "VoID alignment `<{subject}> <{}> <{object}>` has no local endpoint",
                record.predicate
            )));
        }
        let key = (
            subject_dataset.dataset_iri,
            object_dataset.dataset_iri,
            record.predicate.as_str(),
        );
        let count = buckets.entry(key).or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| ProjectionError::limit("VoID linkset triple count overflow"))?;
    }
    if buckets.len() > config.execution_limits().max_linksets() {
        return Err(ProjectionError::limit(format!(
            "VoID analysis has {} linksets; limit is {}",
            buckets.len(),
            config.execution_limits().max_linksets()
        )));
    }

    let mut linksets = Vec::with_capacity(buckets.len());
    for ((subjects_target, objects_target, predicate), triples) in buckets {
        let identity = canonical_json_bounded(
            &(subjects_target, objects_target, predicate),
            config.limits(),
            "VoID linkset identity key",
        )?;
        let local = stable_identifier("void_linkset", &identity)?;
        let iri = generated_iri(config, &local, "VoID linkset")?;
        linksets.push(Linkset {
            iri,
            key: LinksetKey {
                subjects_target: subjects_target.to_owned(),
                objects_target: objects_target.to_owned(),
                predicate: predicate.to_owned(),
            },
            triples,
        });
    }
    Ok(linksets)
}

struct ClassifiedDataset<'a> {
    dataset_iri: &'a str,
    local: bool,
}

fn classify_dataset<'a>(
    iri: &str,
    config: &'a VoidConfig,
) -> Result<ClassifiedDataset<'a>, ProjectionError> {
    let candidates = config
        .local_datasets()
        .iter()
        .map(|binding| (binding, true))
        .chain(
            config
                .external_datasets()
                .iter()
                .map(|binding| (binding, false)),
        );
    let mut best = None::<(&VoidDatasetPrefix, bool)>;
    let mut ambiguous = false;
    for (binding, local) in candidates {
        let prefix = binding.iri_prefix();
        if !iri.starts_with(prefix) {
            continue;
        }
        let Some((best_binding, _)) = best else {
            best = Some((binding, local));
            continue;
        };
        match prefix.len().cmp(&best_binding.iri_prefix().len()) {
            std::cmp::Ordering::Greater => {
                best = Some((binding, local));
                ambiguous = false;
            }
            std::cmp::Ordering::Equal => ambiguous = true,
            std::cmp::Ordering::Less => {}
        }
    }
    let Some((binding, local)) = best else {
        return Err(ProjectionError::integrity(format!(
            "VoID alignment endpoint `{iri}` matches no dataset prefix"
        )));
    };
    if ambiguous {
        return Err(ProjectionError::integrity(format!(
            "VoID alignment endpoint `{iri}` has an ambiguous longest-prefix classification"
        )));
    }
    Ok(ClassifiedDataset {
        dataset_iri: binding.dataset_iri(),
        local,
    })
}

fn collect_external_links<'records, 'config>(
    records: &'records BTreeSet<SourceRecord>,
    config: &'config VoidConfig,
) -> Result<BTreeMap<&'config str, BTreeSet<&'records str>>, ProjectionError> {
    let mut output = BTreeMap::<&str, BTreeSet<&str>>::new();
    for record in records {
        if &record.graph != config.metadata_graph()
            || iri(&record.subject) != Some(config.header_subject_iri())
        {
            continue;
        }
        let Some(target_predicate) = config.external_links().iter().find_map(|mapping| {
            (mapping.source_predicate() == record.predicate).then_some(mapping.target_predicate())
        }) else {
            continue;
        };
        let Some(object) = iri(&record.object) else {
            return Err(ProjectionError::integrity(format!(
                "VoID external-link source predicate `{}` requires an IRI object",
                record.predicate
            )));
        };
        output.entry(target_predicate).or_default().insert(object);
    }
    Ok(output)
}

fn emit_void(
    config: &VoidConfig,
    version: &ProjectionTerm,
    abstract_value: &ProjectionTerm,
    data: &DataAnalysis,
    linksets: &[Linkset],
    external_links: &BTreeMap<&str, BTreeSet<&str>>,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let mut emitter = VoidEmitter {
        builder: RdfDatasetBuilder::new(),
        config,
    };
    let dataset = config.dataset_iri();
    emitter.push_type(dataset, VoidRole::DatasetClass);
    emitter.push_term(dataset, VoidRole::Version, version)?;
    emitter.push_term(dataset, VoidRole::Abstract, abstract_value)?;
    for statement in config.static_statements() {
        emitter.push_static(dataset, statement.predicate(), statement.object());
    }
    emitter.push_count(dataset, VoidRole::Triples, data.aggregate.triples);
    emitter.push_count(
        dataset,
        VoidRole::Entities,
        count_u64(data.aggregate.subjects.len(), "VoID entity")?,
    );
    emitter.push_count(
        dataset,
        VoidRole::Classes,
        count_u64(data.class_partitions.len(), "VoID class")?,
    );
    emitter.push_count(
        dataset,
        VoidRole::Properties,
        count_u64(data.property_partitions.len(), "VoID property")?,
    );
    emitter.push_count(
        dataset,
        VoidRole::DistinctSubjects,
        count_u64(data.aggregate.subjects.len(), "VoID distinct-subject")?,
    );
    emitter.push_count(
        dataset,
        VoidRole::DistinctObjects,
        count_u64(data.aggregate.objects.len(), "VoID distinct-object")?,
    );
    for (predicate, objects) in external_links {
        for object in objects {
            emitter.push_iri_predicate(dataset, predicate, object);
        }
    }

    for (class, aggregate) in &data.class_partitions {
        let local = stable_identifier("void_class_partition", class.as_bytes())?;
        let partition = generated_iri(config, &local, "VoID class partition")?;
        emitter.push_type(&partition, VoidRole::ClassPartitionClass);
        emitter.push_iri(dataset, VoidRole::ClassPartition, &partition);
        emitter.push_iri(dataset, VoidRole::Subset, &partition);
        emitter.push_iri(&partition, VoidRole::Class, class);
        emitter.push_partition_counts(&partition, aggregate)?;
    }
    for (property, aggregate) in &data.property_partitions {
        let local = stable_identifier("void_property_partition", property.as_bytes())?;
        let partition = generated_iri(config, &local, "VoID property partition")?;
        emitter.push_type(&partition, VoidRole::PropertyPartitionClass);
        emitter.push_iri(dataset, VoidRole::PropertyPartition, &partition);
        emitter.push_iri(dataset, VoidRole::Subset, &partition);
        emitter.push_iri(&partition, VoidRole::Property, property);
        emitter.push_partition_counts(&partition, aggregate)?;
    }
    for linkset in linksets {
        emitter.push_type(&linkset.iri, VoidRole::LinksetClass);
        emitter.push_iri(
            &linkset.iri,
            VoidRole::SubjectsTarget,
            &linkset.key.subjects_target,
        );
        emitter.push_iri(
            &linkset.iri,
            VoidRole::ObjectsTarget,
            &linkset.key.objects_target,
        );
        emitter.push_iri(
            &linkset.iri,
            VoidRole::LinkPredicate,
            &linkset.key.predicate,
        );
        emitter.push_count(&linkset.iri, VoidRole::Triples, linkset.triples);
        emitter.push_iri(dataset, VoidRole::Subset, &linkset.iri);
    }

    let dataset = emitter
        .builder
        .freeze()
        .map_err(|error| ProjectionError::integrity(format!("freeze VoID dataset: {error}")))?;
    let output_records = dataset
        .quads()
        .count()
        .checked_add(dataset.reifier_quads().count())
        .and_then(|count| count.checked_add(dataset.annotation_quads().count()))
        .ok_or_else(|| ProjectionError::limit("VoID output record count overflow"))?;
    if output_records > config.execution_limits().max_output_records() {
        return Err(ProjectionError::limit(format!(
            "VoID output has {output_records} records; limit is {}",
            config.execution_limits().max_output_records()
        )));
    }
    Ok(dataset)
}

struct VoidEmitter<'a> {
    builder: RdfDatasetBuilder,
    config: &'a VoidConfig,
}

impl VoidEmitter<'_> {
    fn push_type(&mut self, subject: &str, class: VoidRole) {
        let class = self.config.vocabulary().iri(class).to_owned();
        let predicate = self.config.vocabulary().iri(VoidRole::RdfType).to_owned();
        self.push_iri_predicate(subject, &predicate, &class);
    }

    fn push_iri(&mut self, subject: &str, predicate: VoidRole, object: &str) {
        let predicate = self.config.vocabulary().iri(predicate).to_owned();
        self.push_iri_predicate(subject, &predicate, object);
    }

    fn push_iri_predicate(&mut self, subject: &str, predicate: &str, object: &str) {
        let subject = self.builder.intern_iri(subject);
        let predicate = self.builder.intern_iri(predicate);
        let object = self.builder.intern_iri(object);
        self.builder.push_quad(subject, predicate, object, None);
    }

    fn push_count(&mut self, subject: &str, predicate: VoidRole, value: u64) {
        let datatype = self
            .config
            .vocabulary()
            .iri(VoidRole::XsdNonNegativeInteger)
            .to_owned();
        self.push_literal(
            subject,
            self.config.vocabulary().iri(predicate),
            RdfLiteral::typed(value.to_string(), datatype),
        );
    }

    fn push_term(
        &mut self,
        subject: &str,
        predicate: VoidRole,
        value: &ProjectionTerm,
    ) -> Result<(), ProjectionError> {
        let ProjectionTerm::Literal {
            lexical,
            datatype,
            language,
            direction,
        } = value
        else {
            return Err(ProjectionError::integrity(
                "VoID header target value is not a literal",
            ));
        };
        let literal = RdfLiteral {
            lexical_form: lexical.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: direction.map(Into::into),
        };
        self.push_literal(subject, self.config.vocabulary().iri(predicate), literal);
        Ok(())
    }

    fn push_static(&mut self, subject: &str, predicate: &str, value: &VoidStaticValue) {
        match value {
            VoidStaticValue::Iri { value } => {
                self.push_iri_predicate(subject, predicate, value);
            }
            VoidStaticValue::TypedLiteral { lexical, datatype } => {
                self.push_literal(subject, predicate, RdfLiteral::typed(lexical, datatype));
            }
            VoidStaticValue::LanguageLiteral {
                lexical,
                language,
                direction,
            } => self.push_literal(
                subject,
                predicate,
                RdfLiteral {
                    lexical_form: lexical.clone(),
                    datatype: None,
                    language: Some(language.clone()),
                    direction: direction.map(Into::into),
                },
            ),
        }
    }

    fn push_partition_counts(
        &mut self,
        partition: &str,
        aggregate: &Aggregate,
    ) -> Result<(), ProjectionError> {
        self.push_count(partition, VoidRole::Triples, aggregate.triples);
        let subjects = count_u64(aggregate.subjects.len(), "VoID partition subject")?;
        let objects = count_u64(aggregate.objects.len(), "VoID partition object")?;
        self.push_count(partition, VoidRole::Entities, subjects);
        self.push_count(partition, VoidRole::DistinctSubjects, subjects);
        self.push_count(partition, VoidRole::DistinctObjects, objects);
        Ok(())
    }

    fn push_literal(&mut self, subject: &str, predicate: &str, literal: RdfLiteral) {
        let subject = self.builder.intern_iri(subject);
        let predicate = self.builder.intern_iri(predicate);
        let object = self.builder.intern_literal(literal);
        self.builder.push_quad(subject, predicate, object, None);
    }
}

fn generated_iri(config: &VoidConfig, local: &str, label: &str) -> Result<String, ProjectionError> {
    let iri = format!("{}{local}", config.generated_resource_base_iri());
    validate_absolute_iri(&iri, label)?;
    if iri == config.dataset_iri() {
        return Err(ProjectionError::integrity(format!(
            "{label} identity collides with the described dataset"
        )));
    }
    Ok(iri)
}

fn count_u64(value: usize, label: &str) -> Result<u64, ProjectionError> {
    u64::try_from(value).map_err(|_| ProjectionError::limit(format!("{label} count exceeds u64")))
}

fn iri(term: &ProjectionTerm) -> Option<&str> {
    let ProjectionTerm::Iri { value } = term else {
        return None;
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use purrdf_core::{PackBuilder, PackView, datasets_isomorphic};
    use serde_json::Value;

    use super::*;
    use crate::native_codecs::{NativeRdfFormat, parse_dataset};
    use crate::projections::ProjectionErrorKind;
    use crate::{RdfTerm, dataset_from_quads};

    const SOURCE: &[u8] =
        include_bytes!("../../../tests/fixtures/dataset-description/void-source.trig");
    const CONFIG: &[u8] = include_bytes!("../../../tests/fixtures/dataset-description/void.json");

    fn source() -> Arc<RdfDataset> {
        parse_dataset(SOURCE, "application/trig", None).expect("VoID source")
    }

    fn config_for(format: NativeRdfFormat) -> VoidConfig {
        let mut value: Value = serde_json::from_slice(CONFIG).expect("VoID fixture JSON");
        value["config"]["format"] = Value::String(format.id().to_owned());
        serde_json::from_value(value["config"].clone()).expect("VoID config")
    }

    fn count_value(dataset: &RdfDataset, predicate: &str) -> u64 {
        dataset
            .owned_quads()
            .find_map(|quad| {
                if quad.subject != RdfTerm::iri("https://example.org/void#dataset")
                    || quad.predicate != predicate
                {
                    return None;
                }
                let RdfTerm::Literal(literal) = quad.object else {
                    return None;
                };
                literal.lexical_form.parse().ok()
            })
            .unwrap_or_else(|| panic!("missing count predicate {predicate}"))
    }

    fn typed_subjects(dataset: &RdfDataset, class: &str, config: &VoidConfig) -> usize {
        dataset
            .owned_quads()
            .filter(|quad| {
                quad.predicate == config.vocabulary().iri(VoidRole::RdfType)
                    && quad.object == RdfTerm::iri(class)
            })
            .count()
    }

    #[test]
    fn void_fixture_emits_complete_statistics_partitions_links_and_metadata() {
        let config = config_for(NativeRdfFormat::Turtle);
        let projected = project_void(source().as_ref(), &config).expect("project VoID");
        assert_eq!(projected.artifact_path, "void.ttl");
        assert_eq!(projected.package.len(), 1);
        assert_eq!(
            projected.package.get("void.ttl").expect("Turtle artifact"),
            include_bytes!("../../../tests/fixtures/dataset-description/void.ttl")
        );
        assert_eq!(
            count_value(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::Triples)
            ),
            5
        );
        assert_eq!(
            count_value(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::Entities)
            ),
            2
        );
        assert_eq!(
            count_value(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::Classes)
            ),
            2
        );
        assert_eq!(
            count_value(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::Properties)
            ),
            3
        );
        assert_eq!(
            count_value(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::DistinctObjects)
            ),
            4
        );
        assert_eq!(
            typed_subjects(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::ClassPartitionClass),
                &config,
            ),
            2
        );
        assert_eq!(
            typed_subjects(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::PropertyPartitionClass),
                &config,
            ),
            3
        );
        assert_eq!(
            typed_subjects(
                &projected.dataset,
                config.vocabulary().iri(VoidRole::LinksetClass),
                &config,
            ),
            2
        );

        let quads: Vec<_> = projected.dataset.owned_quads().collect();
        assert!(quads.iter().any(|quad| {
            quad.predicate == "https://example.org/target/authority"
                && quad.object == RdfTerm::iri("https://external.example/catalogue")
        }));
        assert!(quads.iter().any(|quad| {
            quad.predicate == config.vocabulary().iri(VoidRole::ObjectsTarget)
                && quad.object == RdfTerm::iri("https://external.example/dataset/specific")
        }));
        assert!(quads.iter().any(|quad| {
            quad.predicate == config.vocabulary().iri(VoidRole::SubjectsTarget)
                && quad.object == RdfTerm::iri("https://external.example/dataset/general")
        }));
        assert!(quads.iter().all(|quad| {
            !matches!(quad.subject, RdfTerm::BlankNode(_) | RdfTerm::Triple { .. })
                && !matches!(quad.object, RdfTerm::BlankNode(_) | RdfTerm::Triple { .. })
                && quad.graph_name.is_none()
        }));
    }

    #[test]
    fn void_is_byte_stable_cross_syntax_order_and_backend() {
        let source = source();
        let mut reversed_quads: Vec<_> = source.owned_quads().collect();
        reversed_quads.reverse();
        let reversed = dataset_from_quads(&reversed_quads).expect("reversed source");
        let pack = PackBuilder::build_bytes(&source).expect("pack");
        let packed = PackView::from_bytes(&pack).expect("pack view");

        for format in NativeRdfFormat::all() {
            let config = config_for(format);
            let resident = project_void(source.as_ref(), &config).expect("resident");
            let repeat = project_void(source.as_ref(), &config).expect("repeat");
            let reordered = project_void(reversed.as_ref(), &config).expect("reordered");
            let packed = project_void(&packed, &config).expect("packed");
            assert_eq!(
                resident.package.to_ustar().expect("resident archive"),
                repeat.package.to_ustar().expect("repeat archive")
            );
            assert_eq!(
                resident.package.to_ustar().expect("resident archive"),
                reordered.package.to_ustar().expect("reordered archive")
            );
            assert_eq!(
                resident.package.to_ustar().expect("resident archive"),
                packed.package.to_ustar().expect("packed archive")
            );
            let bytes = resident
                .package
                .get(&resident.artifact_path)
                .expect("artifact");
            let reparsed = parse_dataset(bytes, format.media_type(), None).expect("reparse VoID");
            assert!(datasets_isomorphic(&resident.dataset, &reparsed));
        }
    }

    #[test]
    fn void_config_and_source_ambiguity_fail_closed() {
        let mut duplicate: Value = serde_json::from_slice(CONFIG).expect("fixture JSON");
        let repeated = duplicate["config"]["local_datasets"][0].clone();
        duplicate["config"]["external_datasets"]
            .as_array_mut()
            .expect("external array")
            .push(repeated);
        assert!(serde_json::from_value::<VoidConfig>(duplicate["config"].clone()).is_err());

        let mut overlapping_graphs: Value = serde_json::from_slice(CONFIG).expect("fixture JSON");
        overlapping_graphs["config"]["metadata_graph"] =
            overlapping_graphs["config"]["alignment_graph"].clone();
        assert!(
            serde_json::from_value::<VoidConfig>(overlapping_graphs["config"].clone()).is_err()
        );

        let mut tiny: Value = serde_json::from_slice(CONFIG).expect("fixture JSON");
        tiny["config"]["execution_limits"]["max_output_records"] = Value::from(1);
        let tiny: VoidConfig = serde_json::from_value(tiny["config"].clone()).expect("tiny config");
        assert!(project_void(source().as_ref(), &tiny).is_err());

        let mut tiny_input: Value = serde_json::from_slice(CONFIG).expect("fixture JSON");
        tiny_input["config"]["execution_limits"]["max_input_records"] = Value::from(1);
        let tiny_input: VoidConfig =
            serde_json::from_value(tiny_input["config"].clone()).expect("tiny input config");
        let input_error =
            project_void(source().as_ref(), &tiny_input).expect_err("input bound must fail");
        assert_eq!(input_error.kind(), ProjectionErrorKind::ResourceLimit);

        let invalid_alignment = String::from_utf8(SOURCE.to_vec())
            .expect("UTF-8 source")
            .replace("specific:a .", "\"not an IRI\" .");
        let invalid_alignment =
            parse_dataset(invalid_alignment.as_bytes(), "application/trig", None)
                .expect("parse invalid alignment shape");
        assert!(
            project_void(
                invalid_alignment.as_ref(),
                &config_for(NativeRdfFormat::Turtle)
            )
            .is_err()
        );

        let unmapped_alignment = String::from_utf8(SOURCE.to_vec())
            .expect("UTF-8 source")
            .replace("specific:a .", "<https://unmapped.example/a> .");
        let unmapped_alignment =
            parse_dataset(unmapped_alignment.as_bytes(), "application/trig", None)
                .expect("parse unmapped alignment");
        assert!(
            project_void(
                unmapped_alignment.as_ref(),
                &config_for(NativeRdfFormat::Turtle)
            )
            .is_err()
        );

        let missing_header = String::from_utf8(SOURCE.to_vec())
            .expect("UTF-8 source")
            .replace("ex:version", "ex:notVersion");
        let missing_header = parse_dataset(missing_header.as_bytes(), "application/trig", None)
            .expect("parse missing header");
        assert!(
            project_void(
                missing_header.as_ref(),
                &config_for(NativeRdfFormat::Turtle)
            )
            .is_err()
        );
    }

    #[test]
    fn dataset_classification_uses_the_longest_matching_prefix() {
        let config = config_for(NativeRdfFormat::Turtle);
        let specific = classify_dataset("https://external.example/resource/item", &config)
            .expect("specific external prefix");
        assert_eq!(
            specific.dataset_iri,
            "https://external.example/dataset/specific"
        );
        assert!(!specific.local);

        let general = classify_dataset("https://external.example/other", &config)
            .expect("general external prefix");
        assert_eq!(
            general.dataset_iri,
            "https://external.example/dataset/general"
        );
        assert!(!general.local);
    }
}

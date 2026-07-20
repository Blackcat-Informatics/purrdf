// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF serialization of mapped or caller-CONSTRUCTed DCAT descriptions.

use std::sync::Arc;

use purrdf_core::{
    DatasetView, LossLedger, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection,
    check_ledger_sound,
};
use serde::{Deserialize, Deserializer, Serialize};

use crate::native_codecs::NativeRdfFormat;

use super::dataset_description::serialize_description;
use super::research_object::{
    DCAT_PROFILE, DcatConfig, DcatRole, ResearchActivity, ResearchAgent, ResearchChecksum,
    ResearchField, ResearchObjectModel, ResearchRecordSet, ResearchResource, ResearchRole,
    ResearchText, ResearchValue, project_research_object,
};
use super::util::canonical_json_bounded;
use super::{
    ConstructViewConfig, ProjectionDirection, ProjectionError, ProjectionLimits,
    RdfDescriptionProjection, project_construct_view, stable_identifier, validate_absolute_iri,
};

/// Mandatory target-core vocabulary and output bound for mapped DCAT RDF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DcatRdfMappingConfig {
    dcat: DcatConfig,
    rdf_type: String,
    xsd_string: String,
    max_output_records: usize,
}

impl DcatRdfMappingConfig {
    /// Construct a mapped DCAT RDF policy.
    ///
    /// # Errors
    ///
    /// Rejects relative/colliding target-core vocabulary or a zero/non-portable
    /// output record bound.
    pub fn new(
        dcat: DcatConfig,
        rdf_type: impl Into<String>,
        xsd_string: impl Into<String>,
        max_output_records: usize,
    ) -> Result<Self, ProjectionError> {
        let rdf_type = rdf_type.into();
        let xsd_string = xsd_string.into();
        validate_absolute_iri(&rdf_type, "DCAT RDF type predicate")?;
        validate_absolute_iri(&xsd_string, "DCAT RDF XSD string datatype")?;
        if rdf_type == xsd_string {
            return Err(ProjectionError::configuration(
                "DCAT RDF type predicate and XSD string datatype must be distinct",
            ));
        }
        for role in super::research_object::DCAT_ROLES {
            let term = dcat.vocabulary().term(*role);
            let iri = dcat
                .context()
                .expand(term)
                .expect("validated DCAT configuration has every role expansion");
            if iri == rdf_type || iri == xsd_string {
                return Err(ProjectionError::configuration(format!(
                    "DCAT RDF core vocabulary collides with role `{role:?}` at `{iri}`"
                )));
            }
        }
        validate_record_bound(max_output_records, "DCAT RDF max_output_records")?;
        Ok(Self {
            dcat,
            rdf_type,
            xsd_string,
            max_output_records,
        })
    }

    /// Existing caller-owned DCAT model/context policy.
    pub const fn dcat(&self) -> &DcatConfig {
        &self.dcat
    }

    /// Caller-owned target RDF type predicate.
    pub fn rdf_type(&self) -> &str {
        &self.rdf_type
    }

    /// Caller-owned target XSD string datatype.
    pub fn xsd_string(&self) -> &str {
        &self.xsd_string
    }

    /// Maximum emitted RDF records.
    pub const fn max_output_records(&self) -> usize {
        self.max_output_records
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDcatRdfMappingConfig {
    dcat: DcatConfig,
    rdf_type: String,
    xsd_string: String,
    max_output_records: usize,
}

impl<'de> Deserialize<'de> for DcatRdfMappingConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDcatRdfMappingConfig::deserialize(deserializer)?;
        Self::new(
            raw.dcat,
            raw.rdf_type,
            raw.xsd_string,
            raw.max_output_records,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Complete source policy for native DCAT RDF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    content = "config",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum DcatRdfSource {
    /// Interpret the existing shared research-object model and emit direct RDF IR.
    Mapped(Box<DcatRdfMappingConfig>),
    /// Treat a caller-supplied whole-dataset CONSTRUCT as the complete DCAT mapping.
    Construct(Box<ConstructViewConfig>),
}

/// Mandatory output syntax and source policy for the `dcat-rdf` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DcatRdfConfig {
    format: NativeRdfFormat,
    source: DcatRdfSource,
}

impl DcatRdfConfig {
    /// Construct a native DCAT RDF policy with no inferred syntax or mapping.
    pub const fn new(format: NativeRdfFormat, source: DcatRdfSource) -> Self {
        Self { format, source }
    }

    /// Selected registered RDF syntax.
    pub const fn format(&self) -> NativeRdfFormat {
        self.format
    }

    /// Caller-selected mapped or CONSTRUCT source policy.
    pub const fn source(&self) -> &DcatRdfSource {
        &self.source
    }

    /// Package limits supplied by the active source policy.
    pub const fn limits(&self) -> ProjectionLimits {
        match &self.source {
            DcatRdfSource::Mapped(config) => config.dcat().common().limits(),
            DcatRdfSource::Construct(config) => config.limits(),
        }
    }
}

/// Project caller-vocabulary RDF 1.2 into deterministic native DCAT RDF.
///
/// Mapped mode shares the established normalized research-object interpretation;
/// CONSTRUCT mode evaluates the caller mapping over the complete dataset view. Both
/// routes emit one blank-free default graph and use the same lossless syntax packager.
///
/// # Errors
///
/// Returns typed mapping, query, vocabulary, model, codec, integrity, package, or
/// resource-limit failures.
pub fn project_dcat_rdf<D: DatasetView + Sync>(
    view: &D,
    config: &DcatRdfConfig,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    match config.source() {
        DcatRdfSource::Mapped(mapping) => {
            let projection = project_research_object(view, DCAT_PROFILE, mapping.dcat().common())?;
            check_ledger_sound(&projection.loss_ledger, "rdf-1.2-dataset", DCAT_PROFILE)
                .map_err(ProjectionError::integrity)?;
            let dataset = emit_mapped_dcat(&projection.model, mapping)?;
            serialize_description(
                dataset,
                projection.loss_ledger,
                config.format(),
                "dcat",
                config.limits(),
            )
        }
        DcatRdfSource::Construct(construct) => {
            let projection = project_construct_view(view, construct)?;
            serialize_description(
                projection.dataset,
                LossLedger::new(),
                config.format(),
                "dcat",
                config.limits(),
            )
        }
    }
}

fn emit_mapped_dcat(
    model: &ResearchObjectModel,
    config: &DcatRdfMappingConfig,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let mut emitter = DcatEmitter {
        builder: RdfDatasetBuilder::new(),
        config,
    };
    emitter.emit_dataset(model)?;
    for agent in &model.agents {
        emitter.emit_agent(agent);
    }
    for resource in &model.resources {
        emitter.emit_resource(resource)?;
    }
    for activity in &model.activities {
        emitter.emit_activity(activity);
    }
    for record_set in &model.record_sets {
        emitter.emit_record_set(record_set)?;
        for field in &record_set.fields {
            emitter.emit_field(field);
        }
    }
    let dataset = emitter.builder.freeze().map_err(|error| {
        ProjectionError::integrity(format!("freeze mapped DCAT RDF dataset: {error}"))
    })?;
    let records = dataset
        .quads()
        .count()
        .checked_add(dataset.reifier_quads().count())
        .and_then(|count| count.checked_add(dataset.annotation_quads().count()))
        .ok_or_else(|| ProjectionError::limit("mapped DCAT RDF record count overflow"))?;
    if records > config.max_output_records() {
        return Err(ProjectionError::limit(format!(
            "mapped DCAT RDF has {records} records; limit is {}",
            config.max_output_records()
        )));
    }
    Ok(dataset)
}

struct DcatEmitter<'a> {
    builder: RdfDatasetBuilder,
    config: &'a DcatRdfMappingConfig,
}

impl DcatEmitter<'_> {
    fn emit_dataset(&mut self, model: &ResearchObjectModel) -> Result<(), ProjectionError> {
        let dataset = &model.dataset;
        self.push_type(&dataset.id, DcatRole::DatasetClass);
        self.push_iri(
            &dataset.id,
            DcatRole::ConformsTo,
            self.config.dcat().profile_iri(),
        );
        self.push_texts(&dataset.id, DcatRole::Title, &dataset.titles);
        self.push_texts(&dataset.id, DcatRole::Description, &dataset.descriptions);
        self.push_values(&dataset.id, DcatRole::Identifier, &dataset.identifiers);
        self.push_texts(&dataset.id, DcatRole::Version, &dataset.versions);
        self.push_texts(&dataset.id, DcatRole::Issued, &dataset.issued);
        self.push_texts(&dataset.id, DcatRole::Modified, &dataset.modified);
        self.push_values(&dataset.id, DcatRole::LandingPage, &dataset.landing_pages);
        self.push_texts(&dataset.id, DcatRole::Keyword, &dataset.keywords);
        self.push_values(&dataset.id, DcatRole::License, &dataset.licenses);
        for (role, values) in [
            (DcatRole::Creator, &dataset.creators),
            (DcatRole::Publisher, &dataset.publishers),
            (DcatRole::Distribution, &dataset.resources),
            (DcatRole::Activity, &dataset.activities),
            (DcatRole::RecordSet, &dataset.record_sets),
        ] {
            for value in values {
                self.push_iri(&dataset.id, role, value);
            }
        }
        Ok(())
    }

    fn emit_agent(&mut self, agent: &ResearchAgent) {
        self.push_type(&agent.id, DcatRole::AgentClass);
        self.push_texts(&agent.id, DcatRole::AgentName, &agent.names);
    }

    fn emit_resource(&mut self, resource: &ResearchResource) -> Result<(), ProjectionError> {
        self.push_type(&resource.id, DcatRole::DistributionClass);
        self.push_texts(&resource.id, DcatRole::Title, &resource.names);
        self.push_texts(&resource.id, DcatRole::Description, &resource.descriptions);
        for path in &resource.paths {
            self.push_literal(
                &resource.id,
                DcatRole::Path,
                RdfLiteral::typed(path, self.config.xsd_string()),
            );
        }
        self.push_values(&resource.id, DcatRole::DownloadUrl, &resource.urls);
        self.push_texts(&resource.id, DcatRole::MediaType, &resource.media_types);
        self.push_values(&resource.id, DcatRole::Format, &resource.formats);
        if let Some(byte_size) = resource.byte_size {
            self.push_literal(
                &resource.id,
                DcatRole::ByteSize,
                RdfLiteral::typed(
                    byte_size.to_string(),
                    self.config
                        .dcat()
                        .common()
                        .roles()
                        .iri(ResearchRole::XsdNonNegativeInteger),
                ),
            );
        }
        for (index, checksum) in resource.checksums.iter().enumerate() {
            let checksum_iri = self.checksum_iri(resource, index, checksum)?;
            self.push_iri(&resource.id, DcatRole::Checksum, &checksum_iri);
            self.emit_checksum(&checksum_iri, checksum);
        }
        Ok(())
    }

    fn checksum_iri(
        &self,
        resource: &ResearchResource,
        index: usize,
        checksum: &ResearchChecksum,
    ) -> Result<String, ProjectionError> {
        let key = canonical_json_bounded(
            &(resource.id.as_str(), index, checksum),
            self.config.dcat().common().limits(),
            "DCAT RDF checksum identity key",
        )?;
        let local = stable_identifier("dcat_checksum", &key)?;
        let iri = format!(
            "{}{}",
            self.config.dcat().common().identity().entity_base_iri(),
            local
        );
        validate_absolute_iri(&iri, "DCAT RDF checksum identity")?;
        Ok(iri)
    }

    fn emit_checksum(&mut self, id: &str, checksum: &ResearchChecksum) {
        self.push_type(id, DcatRole::ChecksumClass);
        self.push_value(id, DcatRole::ChecksumAlgorithm, &checksum.algorithm);
        self.push_text(id, DcatRole::ChecksumValue, &checksum.value);
    }

    fn emit_activity(&mut self, activity: &ResearchActivity) {
        self.push_type(&activity.id, DcatRole::ActivityClass);
        self.push_texts(&activity.id, DcatRole::Title, &activity.names);
        self.push_values(&activity.id, DcatRole::Instrument, &activity.instruments);
        for (role, values) in [
            (DcatRole::Agent, &activity.actors),
            (DcatRole::Object, &activity.objects),
            (DcatRole::Result, &activity.results),
        ] {
            for value in values {
                self.push_iri(&activity.id, role, value);
            }
        }
        self.push_texts(&activity.id, DcatRole::EndTime, &activity.end_times);
        self.push_values(&activity.id, DcatRole::Workflow, &activity.workflows);
    }

    fn emit_record_set(&mut self, record_set: &ResearchRecordSet) -> Result<(), ProjectionError> {
        self.push_type(&record_set.id, DcatRole::RecordSetClass);
        self.push_texts(&record_set.id, DcatRole::Title, &record_set.names);
        self.push_texts(
            &record_set.id,
            DcatRole::Description,
            &record_set.descriptions,
        );
        for field in &record_set.fields {
            self.push_iri(&record_set.id, DcatRole::Field, &field.id);
        }
        for row in &record_set.rows {
            let bytes = canonical_json_bounded(
                row,
                self.config.dcat().common().limits(),
                "DCAT RDF inline row",
            )?;
            let lexical = std::str::from_utf8(&bytes)
                .expect("canonical JSON is UTF-8")
                .to_owned();
            self.push_literal(
                &record_set.id,
                DcatRole::Records,
                RdfLiteral::typed(
                    lexical,
                    self.config
                        .dcat()
                        .common()
                        .roles()
                        .iri(ResearchRole::JsonDatatype),
                ),
            );
        }
        Ok(())
    }

    fn emit_field(&mut self, field: &ResearchField) {
        self.push_type(&field.id, DcatRole::FieldClass);
        self.push_texts(&field.id, DcatRole::Title, &field.names);
        self.push_values(&field.id, DcatRole::DataType, &field.data_types);
    }

    fn push_type(&mut self, subject: &str, class: DcatRole) {
        let class = self.role_iri(class).to_owned();
        self.push_iri_predicate(subject, self.config.rdf_type(), &class);
    }

    fn push_iri(&mut self, subject: &str, predicate: DcatRole, object: &str) {
        let predicate = self.role_iri(predicate).to_owned();
        self.push_iri_predicate(subject, &predicate, object);
    }

    fn push_iri_predicate(&mut self, subject: &str, predicate: &str, object: &str) {
        let subject = self.builder.intern_iri(subject);
        let predicate = self.builder.intern_iri(predicate);
        let object = self.builder.intern_iri(object);
        self.builder.push_quad(subject, predicate, object, None);
    }

    fn push_texts(&mut self, subject: &str, predicate: DcatRole, values: &[ResearchText]) {
        for value in values {
            self.push_text(subject, predicate, value);
        }
    }

    fn push_text(&mut self, subject: &str, predicate: DcatRole, value: &ResearchText) {
        self.push_literal(subject, predicate, rdf_literal(value));
    }

    fn push_values(&mut self, subject: &str, predicate: DcatRole, values: &[ResearchValue]) {
        for value in values {
            self.push_value(subject, predicate, value);
        }
    }

    fn push_value(&mut self, subject: &str, predicate: DcatRole, value: &ResearchValue) {
        match value {
            ResearchValue::Iri { value } => self.push_iri(subject, predicate, value),
            ResearchValue::Text(value) => self.push_text(subject, predicate, value),
        }
    }

    fn push_literal(&mut self, subject: &str, predicate: DcatRole, value: RdfLiteral) {
        let predicate = self.role_iri(predicate).to_owned();
        let subject = self.builder.intern_iri(subject);
        let predicate = self.builder.intern_iri(&predicate);
        let object = self.builder.intern_literal(value);
        self.builder.push_quad(subject, predicate, object, None);
    }

    fn role_iri(&self, role: DcatRole) -> &str {
        let term = self.config.dcat().vocabulary().term(role);
        self.config
            .dcat()
            .context()
            .expand(term)
            .expect("validated DCAT configuration has every role expansion")
    }
}

fn rdf_literal(value: &ResearchText) -> RdfLiteral {
    RdfLiteral {
        lexical_form: value.value.clone(),
        datatype: Some(value.datatype.clone()),
        language: value.language.clone(),
        direction: value.direction.map(|direction| match direction {
            ProjectionDirection::Ltr => RdfTextDirection::Ltr,
            ProjectionDirection::Rtl => RdfTextDirection::Rtl,
        }),
    }
}

fn validate_record_bound(value: usize, field: &str) -> Result<(), ProjectionError> {
    if value == 0 {
        return Err(ProjectionError::configuration(format!(
            "{field} must be greater than zero"
        )));
    }
    if u32::try_from(value).is_err() {
        return Err(ProjectionError::configuration(format!(
            "{field} exceeds the portable u32 ceiling"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::{RdfDatasetBuilder, datasets_isomorphic};
    use serde_json::Value;

    use super::*;
    use crate::native_codecs::parse_dataset;
    use crate::projections::{DCAT_ARTIFACT, project_dcat};

    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

    fn dcat_config() -> DcatConfig {
        let value: Value = serde_json::from_slice(include_bytes!(
            "../../tests/fixtures/research-objects/carrier/dcat-3.json"
        ))
        .expect("fixture JSON");
        serde_json::from_value(value["config"].clone()).expect("DCAT config")
    }

    fn mapping() -> DcatRdfMappingConfig {
        DcatRdfMappingConfig::new(dcat_config(), RDF_TYPE, XSD_STRING, 10_000).expect("mapping")
    }

    fn minimal_source(config: &DcatConfig) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri(config.common().identity().dataset_iri());
        let rdf_type = builder.intern_iri(config.common().roles().iri(ResearchRole::RdfType));
        let dataset_class =
            builder.intern_iri(config.common().roles().iri(ResearchRole::DatasetClass));
        builder.push_quad(subject, rdf_type, dataset_class, None);
        let title = builder.intern_iri(config.common().roles().iri(ResearchRole::Title));
        let value = builder.intern_literal(RdfLiteral::typed(
            "Minimal dataset",
            config.common().roles().iri(ResearchRole::XsdString),
        ));
        builder.push_quad(subject, title, value, None);
        builder.freeze().expect("source")
    }

    #[test]
    fn mapped_rdf_matches_jsonld_semantics_without_checksum_skolemization() {
        let mapping = mapping();
        let source = minimal_source(mapping.dcat());
        let jsonld = project_dcat(source.as_ref(), mapping.dcat()).expect("JSON-LD");
        let semantic = parse_dataset(
            jsonld.package.get(DCAT_ARTIFACT).expect("artifact"),
            NativeRdfFormat::JsonLd.media_type(),
            None,
        )
        .expect("parse JSON-LD");
        let projected = project_dcat_rdf(
            source.as_ref(),
            &DcatRdfConfig::new(
                NativeRdfFormat::Turtle,
                DcatRdfSource::Mapped(Box::new(mapping)),
            ),
        )
        .expect("mapped RDF");
        assert!(datasets_isomorphic(&semantic, &projected.dataset));
    }

    #[test]
    fn mapped_and_construct_modes_cover_every_registered_syntax() {
        let mapping = mapping();
        let source = minimal_source(mapping.dcat());
        for format in NativeRdfFormat::all() {
            let mapped = project_dcat_rdf(
                source.as_ref(),
                &DcatRdfConfig::new(format, DcatRdfSource::Mapped(Box::new(mapping.clone()))),
            )
            .expect("mapped");
            assert_eq!(
                mapped.artifact_path,
                format!("dcat.{}", format.file_extension())
            );
            let bytes = mapped.package.get(&mapped.artifact_path).expect("artifact");
            let reparsed = parse_dataset(bytes, format.media_type(), None).expect("parse mapped");
            assert!(datasets_isomorphic(&mapped.dataset, &reparsed));

            let construct = ConstructViewConfig::new(
                "CONSTRUCT { <https://example.org/dataset> <https://example.org/title> \"DCAT\" } WHERE {}",
                None,
                mapping.dcat().common().limits(),
                1_000,
                10,
                10,
            )
            .expect("CONSTRUCT");
            let constructed = project_dcat_rdf(
                source.as_ref(),
                &DcatRdfConfig::new(format, DcatRdfSource::Construct(Box::new(construct))),
            )
            .expect("constructed");
            let bytes = constructed
                .package
                .get(&constructed.artifact_path)
                .expect("artifact");
            let reparsed =
                parse_dataset(bytes, format.media_type(), None).expect("parse construct");
            assert!(datasets_isomorphic(&constructed.dataset, &reparsed));
        }
    }

    #[test]
    fn mapped_configuration_revalidates_and_output_bound_hard_fails() {
        let mapping = mapping();
        let json = serde_json::to_vec(&mapping).expect("serialize");
        assert!(serde_json::from_slice::<DcatRdfMappingConfig>(&json).is_ok());
        assert!(DcatRdfMappingConfig::new(dcat_config(), RDF_TYPE, RDF_TYPE, 10).is_err());

        let source = minimal_source(mapping.dcat());
        let tiny = DcatRdfMappingConfig::new(dcat_config(), RDF_TYPE, XSD_STRING, 1)
            .expect("tiny mapping");
        assert!(
            project_dcat_rdf(
                source.as_ref(),
                &DcatRdfConfig::new(
                    NativeRdfFormat::Turtle,
                    DcatRdfSource::Mapped(Box::new(tiny)),
                ),
            )
            .is_err()
        );
    }
}

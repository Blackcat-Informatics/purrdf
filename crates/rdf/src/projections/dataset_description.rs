// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared deterministic RDF dataset-description serialization.

use std::sync::Arc;

use purrdf_core::{LossLedger, RdfDataset};

use crate::native_codecs::{NativeRdfFormat, serialize_dataset_to_format};

use super::{ProjectionError, ProjectionLimits, ProjectionPackage};

/// One frozen RDF description graph and its deterministic packaged serialization.
#[derive(Debug, Clone)]
pub struct RdfDescriptionProjection {
    /// Frozen caller-vocabulary RDF 1.2 description graph.
    pub dataset: Arc<RdfDataset>,
    /// Canonical package member path derived from the selected syntax registry row.
    pub artifact_path: String,
    /// One-member deterministic projection package.
    pub package: ProjectionPackage,
    /// Always-computed semantic projection loss ledger.
    pub loss_ledger: LossLedger,
}

/// Package an already-materialized RDF 1.2 description graph in any registered syntax.
///
/// # Errors
///
/// Returns a typed configuration, integrity, codec, package, or resource-limit error
/// when the artifact stem is unsafe, the dataset contains named graphs, the selected
/// syntax would lower RDF 1.2 content, serialization fails, or package bounds are
/// exceeded.
pub fn serialize_rdf_description(
    dataset: Arc<RdfDataset>,
    format: NativeRdfFormat,
    artifact_stem: &str,
    limits: ProjectionLimits,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    serialize_description(dataset, LossLedger::new(), format, artifact_stem, limits)
}

/// Serialize a default-graph RDF description without lowering any RDF 1.2 content.
///
/// The description engines deliberately emit one default graph, so every registered
/// syntax carries the same graph. Syntaxes unable to carry a produced RDF 1.2
/// statement row or directional literal fail instead of silently lowering it.
pub(crate) fn serialize_description(
    dataset: Arc<RdfDataset>,
    loss_ledger: LossLedger,
    format: NativeRdfFormat,
    artifact_stem: &str,
    limits: ProjectionLimits,
) -> Result<RdfDescriptionProjection, ProjectionError> {
    validate_artifact_stem(artifact_stem)?;
    if dataset.named_graphs().next().is_some() {
        return Err(ProjectionError::integrity(
            "an RDF dataset description must contain only the default graph",
        ));
    }
    let serialized =
        serialize_dataset_to_format(dataset.as_ref(), format, None).map_err(|error| {
            ProjectionError::integrity(format!(
                "native {} serialization of RDF dataset description failed: {error}",
                format.id()
            ))
        })?;
    if serialized.statement_rows_dropped != 0 || serialized.directional_literals_dropped != 0 {
        return Err(ProjectionError::integrity(format!(
            "native {} serialization would drop {} RDF 1.2 statement rows and {} directional literals",
            format.id(),
            serialized.statement_rows_dropped,
            serialized.directional_literals_dropped
        )));
    }
    let artifact_path = format!("{artifact_stem}.{}", format.file_extension());
    let package =
        ProjectionPackage::from_artifacts(limits, [(artifact_path.clone(), serialized.bytes)])?;
    Ok(RdfDescriptionProjection {
        dataset,
        artifact_path,
        package,
        loss_ledger,
    })
}

fn validate_artifact_stem(value: &str) -> Result<(), ProjectionError> {
    let mut chars = value.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(ProjectionError::configuration(
            "RDF description artifact stem must start with an ASCII letter and contain only ASCII alphanumerics, `-`, or `_`",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_core::RdfDatasetBuilder;

    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(1, 1_000_000, 1_000_000, 1_002_000, 16).expect("limits")
    }

    fn default_graph() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        builder.push_quad(subject, predicate, object, None);
        builder.freeze().expect("dataset")
    }

    #[test]
    fn every_registered_syntax_gets_one_stable_artifact() {
        for format in NativeRdfFormat::all() {
            let first = serialize_description(
                default_graph(),
                LossLedger::new(),
                format,
                "description",
                limits(),
            )
            .expect("serialize description");
            let second = serialize_description(
                default_graph(),
                LossLedger::new(),
                format,
                "description",
                limits(),
            )
            .expect("serialize description again");
            assert_eq!(
                first.artifact_path,
                format!("description.{}", format.file_extension())
            );
            assert_eq!(first.package.len(), 1);
            assert_eq!(
                first.package.to_ustar().expect("first archive"),
                second.package.to_ustar().expect("second archive")
            );
        }
    }

    #[test]
    fn named_graphs_and_unsafe_stems_fail_closed() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        let graph = builder.intern_iri("https://example.org/graph");
        builder.push_quad(subject, predicate, object, Some(graph));
        let dataset = builder.freeze().expect("dataset");
        assert!(
            serialize_description(
                dataset,
                LossLedger::new(),
                NativeRdfFormat::TriG,
                "description",
                limits(),
            )
            .is_err()
        );
        assert!(
            serialize_description(
                default_graph(),
                LossLedger::new(),
                NativeRdfFormat::Turtle,
                "../unsafe",
                limits(),
            )
            .is_err()
        );
    }
}

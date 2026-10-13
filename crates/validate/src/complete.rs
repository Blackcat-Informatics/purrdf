// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete contextual SHACL reports at the shared Rust boundary.
//!
//! The graph, its report root and original source correspondence travel together.
//! Existing host payloads and SARIF mappings remain their compatibility projection.

use std::sync::Arc;

use purrdf_core::RdfDataset;
use purrdf_lex::json::{self, Object, Value};
use purrdf_shapes::engine::{PreparedShapes, ValidationOptions};
use purrdf_shapes::report::{CompleteValidationError, CompleteValidationReport};
use purrdf_shapes::{ShaclProfile, ShapesImports, UnsupportedProfile};

use crate::{SarifOptions, ShapesImportList};

/// A verdict or refusal category. Refusals never imply conformance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompleteValidationStatus {
    /// A complete validation found no disallowed result.
    Conforms,
    /// A complete validation found a disallowed result.
    Nonconforms,
    /// The selected query or profile contract refused admission.
    AdmissionRefused,
    /// Original source or occurrence correspondence could not be established.
    SourceRefused,
    /// A query solution explicitly declared failure.
    SemanticFailure,
    /// The actual governor or native XPath resource bound stopped execution.
    ResourceRefused,
    /// The selected native executor returned another typed refusal.
    NativeXPathRefused,
    /// An authenticated preparation could not be restored.
    ProductRefused,
    /// The requested profile identifier is unsupported.
    UnsupportedProfile,
    /// Parsing or another execution failed without a conformance verdict.
    ExecutionFailure,
}

impl CompleteValidationStatus {
    /// A stable machine-readable label independent of diagnostic wording.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Conforms => "conforms",
            Self::Nonconforms => "nonconforms",
            Self::AdmissionRefused => "admission-refused",
            Self::SourceRefused => "source-refused",
            Self::SemanticFailure => "semantic-failure",
            Self::ResourceRefused => "resource-refused",
            Self::NativeXPathRefused => "native-xpath-refused",
            Self::ProductRefused => "product-refused",
            Self::UnsupportedProfile => "unsupported-profile",
            Self::ExecutionFailure => "execution-failure",
        }
    }
}

/// Classify an actual typed outcome without matching diagnostic text.
#[must_use]
pub fn complete_validation_status(
    outcome: &Result<CompleteValidationReport, CompleteValidationError>,
) -> CompleteValidationStatus {
    use CompleteValidationStatus as Status;
    match outcome {
        Ok(report) if report.legacy().conforms => Status::Conforms,
        Ok(_) => Status::Nonconforms,
        Err(CompleteValidationError::Admission(_) | CompleteValidationError::XPathProfile(_)) => {
            Status::AdmissionRefused
        }
        Err(
            CompleteValidationError::SourceConstraint(_)
            | CompleteValidationError::QuerySource(_)
            | CompleteValidationError::SourceContext(_),
        ) => Status::SourceRefused,
        Err(CompleteValidationError::Semantic(_)) => Status::SemanticFailure,
        Err(CompleteValidationError::Resource(_)) => Status::ResourceRefused,
        Err(CompleteValidationError::Preparation(_)) => Status::ProductRefused,
        Err(CompleteValidationError::XPath(error)) => match error.as_ref() {
            purrdf_shapes::xpath::XPathValidationError::Pattern(
                purrdf_core::xsd_regex::xpath::Error::Resource(_)
                | purrdf_core::xsd_regex::xpath::Error::Allocation { .. },
            ) => Status::ResourceRefused,
            purrdf_shapes::xpath::XPathValidationError::Query(diagnostic)
                if [
                    purrdf_core::xsd_regex::xpath::Resource::PatternBytes,
                    purrdf_core::xsd_regex::xpath::Resource::CompileSteps,
                    purrdf_core::xsd_regex::xpath::Resource::ProgramNodes,
                    purrdf_core::xsd_regex::xpath::Resource::CompileSlots,
                    purrdf_core::xsd_regex::xpath::Resource::MatchSteps,
                    purrdf_core::xsd_regex::xpath::Resource::MatchStates,
                    purrdf_core::xsd_regex::xpath::Resource::MatchSlots,
                    purrdf_core::xsd_regex::xpath::Resource::OutputBytes,
                ]
                .into_iter()
                .any(|resource| diagnostic.code == resource.code()) =>
            {
                Status::ResourceRefused
            }
            _ => Status::NativeXPathRefused,
        },
        Err(_) => Status::ExecutionFailure,
    }
}

/// A decoded supported profile or its explicit unsupported status.
///
/// No guessed date, fabricated default or fallback is used.
/// # Errors
/// Returns the original unsupported identifier beside its non-pass status.
pub fn complete_profile(
    id: &str,
) -> Result<ShaclProfile, (CompleteValidationStatus, UnsupportedProfile)> {
    ShaclProfile::from_id(id).map_err(|error| (CompleteValidationStatus::UnsupportedProfile, error))
}

/// Validate native immutable acquisitions, retaining deliberate source aliasing.
/// # Errors
/// Returns the engine's actual profile, source, semantic or operational refusal.
pub fn validate_complete_sources(
    data: Arc<RdfDataset>,
    shapes: Arc<purrdf_shapes::shapes::Shapes>,
    options: &ValidationOptions,
) -> Result<CompleteValidationReport, CompleteValidationError> {
    PreparedShapes::new(shapes)
        .with_validation_options(options.clone())
        .bind_complete_shared_dataset(data)?
        .validate()
}

/// Parse the caller's two documents and validate under the current dated bundle.
/// Imports and data-graph shapes links use the existing import resolver.
/// # Errors
/// Refusals remain typed and no partial report is returned.
pub fn validate_complete_documents(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
    data_nt: &str,
    options: &ValidationOptions,
    imports: &ShapesImportList<'_>,
) -> Result<CompleteValidationReport, CompleteValidationError> {
    options
        .shacl_profile
        .resolve_xpath(options.xpath_regex)
        .map_err(CompleteValidationError::XPathProfile)?;
    let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| CompleteValidationError::Execution(errors.join("\n")))?;
    let mut table = ShapesImports::from_turtle(imports)
        .map_err(|error| CompleteValidationError::Shapes(error.into()))?;
    table
        .link_data_graph(data.as_ref(), &[])
        .map_err(|error| CompleteValidationError::Shapes(error.into()))?;
    let shapes = purrdf_shapes::engine::parse_shapes_with_options(
        shapes_ttl,
        shapes_base,
        None,
        None,
        &table,
        options,
    )?;
    validate_complete_sources(data, Arc::new(shapes), options)
}

/// Restore an authenticated product under the current request before validating.
/// `rebuild` selects the existing forward-compatible reconstruction door;
/// `expected_identity` preserves the existing artifact expectation check.
/// # Errors
/// Returns actual product dimensions, admission or execution refusals.
pub fn validate_complete_product(
    product: &[u8],
    data_nt: &str,
    options: &ValidationOptions,
    expected_identity: Option<&[u8; 32]>,
    rebuild: bool,
) -> Result<CompleteValidationReport, CompleteValidationError> {
    let prepared = crate::product::restore_shapes_product_with_options(
        product,
        options,
        expected_identity,
        rebuild,
    )?;
    let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| CompleteValidationError::Execution(errors.join("\n")))?;
    prepared.bind_complete_shared_dataset(data)?.validate()
}

/// A deterministic full report payload, including a dedicated RDF graph/root and
/// acquisition-local source correspondence. Source indices identify the two
/// retained roles within this payload, never a process-global identity.
/// # Errors
/// Returns an actual native codec refusal rather than dropping a graph or source.
pub fn complete_report_payload(
    report: &CompleteValidationReport,
) -> Result<Value, purrdf_core::RdfDiagnostic> {
    let graph = report.to_graph();
    let context = graph.source_context();
    let shared = context.sources_share_identity();
    let encode = |dataset: &RdfDataset| {
        purrdf_rdf::serialize_dataset(
            dataset,
            "application/n-quads",
            purrdf_rdf::SerializeGraph::Dataset,
        )
        .map(|bytes| String::from_utf8(bytes).expect("native RDF text is UTF-8"))
    };
    let mut blanks: Vec<_> = graph.blank_labels().sources().collect();
    blanks.sort_unstable_by_key(|(label, _)| *label);
    let correspondence = Value::array(blanks.into_iter().map(|(label, source)| {
        let source_index = u32::from(source.source() != context.data());
        Object::new()
            .with("reportLabel", label)
            .with("source", source_index)
            .with("scope", source.scope().0)
            .with("label", source.label())
    }));
    let mut sources = vec![Value::from(
        Object::new().with("graph", encode(context.data().dataset())?),
    )];
    if !shared {
        sources.push(
            Object::new()
                .with("graph", encode(context.shapes().dataset())?)
                .into(),
        );
    }
    Ok(Object::new()
        .with("profile", report.profile().id())
        .with(
            "status",
            if report.legacy().conforms {
                "conforms"
            } else {
                "nonconforms"
            },
        )
        .with("graph", encode(graph.dataset())?)
        .with("root", graph.root().to_string())
        .with("sources", Value::Array(sources))
        .with("dataSource", 0_u32)
        .with("shapesSource", u32::from(!shared))
        .with("correspondence", correspondence)
        .into())
}

/// Render the complete payload through the existing lexical JSON writer.
/// # Errors
/// Returns the same codec refusal as [`complete_report_payload`].
pub fn complete_report_to_json(
    report: &CompleteValidationReport,
) -> Result<String, purrdf_core::RdfDiagnostic> {
    complete_report_payload(report).map(|value| json::write_pretty(&value))
}

/// Reuse the compatibility SARIF mapper and attach the complete contextual RDF
/// payload at run level. Results, source constraints and details remain paired
/// in that graph; sorted SARIF result rows are never used to reconstruct evidence.
/// # Errors
/// Returns an actual complete-graph codec refusal.
pub fn complete_report_to_sarif_string(
    report: &CompleteValidationReport,
    options: &SarifOptions,
) -> Result<String, purrdf_core::RdfDiagnostic> {
    let mut log = crate::build_report_sarif(report.legacy(), options);
    log.runs[0]
        .properties
        .insert("shaclCompleteReport", complete_report_payload(report)?);
    Ok(crate::model::to_json_pretty(&log))
}

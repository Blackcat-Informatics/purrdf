// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public CSVW read/validate pipeline.

use std::sync::Arc;

use purrdf_core::{LossLedger, RdfDataset};

use super::super::ProjectionError;
use super::config::CsvwConfig;
use super::input::{CsvwInput, CsvwWarning, CsvwWarningKind};
use super::metadata::load_metadata;
use super::model::CsvwTableGroup;
use super::rdf::table_group_to_rdf;
use super::table::annotate_tables;

/// Result of processing a complete CSVW resource package.
#[derive(Debug, Clone)]
pub struct CsvwReadOutcome {
    /// Fully normalized annotated-table model retained for deterministic writing.
    pub group: CsvwTableGroup,
    /// Normative RDF conversion in the configured standard/minimal mode.
    pub dataset: Arc<RdfDataset>,
    /// Deterministically ordered metadata and cell diagnostics.
    pub warnings: Vec<CsvwWarning>,
    /// Always-computed conversion ledger; normative CSVW conversion is lossless.
    pub loss_ledger: LossLedger,
}

impl CsvwReadOutcome {
    /// Whether no cell/schema validation error was observed.
    ///
    /// Metadata fallback warnings do not by themselves make a table invalid.
    pub fn is_valid(&self) -> bool {
        !self
            .warnings
            .iter()
            .any(|warning| warning.kind == CsvwWarningKind::Validation)
    }
}

/// Parse metadata and tables, validate them, and run the normative RDF mapping.
///
/// # Errors
///
/// Returns a typed hard failure for structurally invalid metadata, missing or
/// malformed resources, ambiguous references, unsafe IRIs/templates, configured
/// resource-limit breaches, or an invalid RDF result. Recommendation-defined
/// recoverable conditions are returned as [`CsvwWarning`] values.
pub fn read_csvw(
    input: &CsvwInput,
    config: &CsvwConfig,
) -> Result<CsvwReadOutcome, ProjectionError> {
    let mut metadata = load_metadata(input, config)?;
    annotate_tables(&mut metadata.group, input, config, &mut metadata.warnings)?;
    metadata.warnings.sort();
    metadata.warnings.dedup();
    let dataset = table_group_to_rdf(&metadata.group, config)?;
    Ok(CsvwReadOutcome {
        group: metadata.group,
        dataset,
        warnings: metadata.warnings,
        loss_ledger: LossLedger::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use purrdf_core::datasets_isomorphic;

    use super::*;
    use crate::{
        CsvwAction, CsvwContext, CsvwMode, CsvwVocabulary, ProjectionLimits, parse_dataset,
    };

    fn config(mode: CsvwMode) -> CsvwConfig {
        CsvwConfig::new(
            "http://example.org/",
            CsvwContext::new("http://www.w3.org/ns/csvw", BTreeMap::new()).expect("context"),
            "http://example.org/package",
            CsvwVocabulary::new(
                "http://www.w3.org/ns/csvw#",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "http://www.w3.org/2000/01/rdf-schema#",
                "http://www.w3.org/2001/XMLSchema#",
            )
            .expect("vocabulary"),
            mode,
            ProjectionLimits::new(64, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits"),
            10_000,
        )
        .expect("config")
    }

    #[test]
    fn embedded_table_matches_the_normative_standard_mapping() {
        let table = "http://example.org/test.csv";
        let input = CsvwInput::new(
            CsvwAction::Table {
                table_iri: table.to_owned(),
                metadata_iri: None,
            },
            BTreeMap::from([(table.to_owned(), b"name,age\nAlice,42\n".to_vec())]),
            config(CsvwMode::Standard).limits(),
        )
        .expect("input");
        let outcome = read_csvw(&input, &config(CsvwMode::Standard)).expect("CSVW");
        assert!(outcome.warnings.is_empty());
        assert!(outcome.loss_ledger.is_empty());
        let expected = parse_dataset(
            br#"@prefix csvw: <http://www.w3.org/ns/csvw#> .
                @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
                @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
                [ a csvw:TableGroup; csvw:table [ a csvw:Table;
                    csvw:url <http://example.org/test.csv>;
                    csvw:row [ a csvw:Row; csvw:rownum 1;
                        csvw:url <http://example.org/test.csv#row=2>;
                        csvw:describes [
                            <http://example.org/test.csv#name> "Alice";
                            <http://example.org/test.csv#age> "42"
                        ]
                    ]
                ] ] ."#,
            "text/turtle",
            Some("http://example.org/"),
        )
        .expect("expected RDF");
        assert!(datasets_isomorphic(&outcome.dataset, &expected));
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! In-memory graph, tabular, and research-object projection carrier bindings.

use purrdf::ir::MutableDataset;
use purrdf::{LiftProfile, ProjectionConfig, ProjectionProfile, lift_archive, project_archive};
use wasm_bindgen::prelude::*;

use crate::dataset::{Dataset, diag_to_err};

fn parse_projection_config(config_json: &str) -> Result<ProjectionConfig, String> {
    ProjectionConfig::from_json(config_json.as_bytes()).map_err(|error| error.to_string())
}

/// A deterministic USTAR projection package and its canonical runtime ledger.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ProjectionPackage {
    profile: String,
    archive: Vec<u8>,
    loss_ledger_json: String,
}

#[wasm_bindgen]
impl ProjectionPackage {
    /// Stable carrier profile name.
    #[wasm_bindgen(getter)]
    pub fn profile(&self) -> String {
        self.profile.clone()
    }

    /// Canonical deterministic USTAR bytes.
    #[wasm_bindgen(getter)]
    pub fn archive(&self) -> Vec<u8> {
        self.archive.clone()
    }

    /// Canonical, versioned runtime loss-ledger JSON.
    #[wasm_bindgen(getter, js_name = lossLedgerJson)]
    pub fn loss_ledger_json(&self) -> String {
        self.loss_ledger_json.clone()
    }
}

/// Result of lifting a strict carrier package into an in-memory RDF dataset.
#[wasm_bindgen]
#[derive(Debug)]
pub struct ProjectionLift {
    dataset: Option<Dataset>,
    loss_ledger_json: String,
}

#[wasm_bindgen]
impl ProjectionLift {
    /// Move the lifted dataset out of this result. The dataset can be taken once.
    #[wasm_bindgen(js_name = takeDataset)]
    pub fn take_dataset(&mut self) -> Option<Dataset> {
        self.dataset.take()
    }

    /// Canonical, versioned runtime loss-ledger JSON.
    #[wasm_bindgen(getter, js_name = lossLedgerJson)]
    pub fn loss_ledger_json(&self) -> String {
        self.loss_ledger_json.clone()
    }
}

#[wasm_bindgen]
impl Dataset {
    /// Project this dataset into a deterministic graph, tabular, or research-object USTAR package.
    #[wasm_bindgen(js_name = project)]
    pub fn project(&self, profile: &str, config_json: &str) -> Result<ProjectionPackage, JsError> {
        let profile = profile
            .parse::<ProjectionProfile>()
            .map_err(|error| JsError::new(&error.to_string()))?;
        let config = parse_projection_config(config_json).map_err(|error| JsError::new(&error))?;
        let frozen = self.inner.freeze().map_err(|error| diag_to_err(&error))?;
        let outcome = project_archive(frozen.as_ref(), profile, &config)
            .map_err(|error| JsError::new(&error.to_string()))?;
        Ok(ProjectionPackage {
            profile: outcome.profile.as_str().to_owned(),
            archive: outcome.archive,
            loss_ledger_json: outcome.loss_ledger.render_json(),
        })
    }
}

/// Lift a strict bidirectional USTAR package into an in-memory RDF dataset.
#[wasm_bindgen(js_name = liftProjection)]
pub fn lift_projection(
    archive: &[u8],
    profile: &str,
    config_json: &str,
) -> Result<ProjectionLift, JsError> {
    let profile = profile
        .parse::<LiftProfile>()
        .map_err(|error| JsError::new(&error.to_string()))?;
    let config = ProjectionConfig::from_json(config_json.as_bytes())
        .map_err(|error| JsError::new(&error.to_string()))?;
    let outcome = lift_archive(archive, profile, &config)
        .map_err(|error| JsError::new(&error.to_string()))?;
    Ok(ProjectionLift {
        dataset: Some(Dataset {
            inner: MutableDataset::new(outcome.dataset),
        }),
        loss_ledger_json: outcome.loss_ledger.render_json(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"{
      "profile": "lpg-csv",
      "config": {
        "rdf_type": "https://example.org/type",
        "scope": {"mode": "all"},
        "limits": {
          "max_artifacts": 16,
          "max_artifact_bytes": 1000000,
          "max_total_bytes": 4000000,
          "max_archive_bytes": 5000000,
          "max_term_depth": 16
        },
        "execution_limits": {
          "max_input_records": 1000,
          "max_model_records": 1000,
          "max_nodes": 1000,
          "max_edges": 1000
        }
      }
    }"#;
    const MISSING_SCOPE_CONFIG: &str = r#"{
      "profile": "lpg-csv",
      "config": {
        "rdf_type": "https://example.org/type",
        "limits": {
          "max_artifacts": 16,
          "max_artifact_bytes": 1000000,
          "max_total_bytes": 4000000,
          "max_archive_bytes": 5000000,
          "max_term_depth": 16
        },
        "execution_limits": {
          "max_input_records": 1000,
          "max_model_records": 1000,
          "max_nodes": 1000,
          "max_edges": 1000
        }
      }
    }"#;
    const RESEARCH_SOURCE: &str =
        include_str!("../../rdf/tests/fixtures/research-objects/carrier/shared.ttl");
    const CSVW_TERMS_CONFIG: &str = include_str!("../../rdf/tests/fixtures/csvw-terms.json");
    const OKF_TERMS_CONFIG: &str = include_str!("../../rdf/tests/fixtures/okf-terms.json");
    const OKF_TERMS_SOURCE: &str = include_str!("../../rdf/tests/fixtures/okf-terms.trig");
    const RESEARCH_CONFIGS: &[(&str, &str)] = &[
        (
            "croissant-1.1",
            include_str!("../../rdf/tests/fixtures/research-objects/carrier/croissant-1.1.json"),
        ),
        (
            "ro-crate-1.3",
            include_str!("../../rdf/tests/fixtures/research-objects/carrier/ro-crate-1.3.json"),
        ),
        (
            "datacite-4.6",
            include_str!("../../rdf/tests/fixtures/research-objects/carrier/datacite-4.6.json"),
        ),
        (
            "dcat-3",
            include_str!("../../rdf/tests/fixtures/research-objects/carrier/dcat-3.json"),
        ),
        (
            "frictionless-data-package-1",
            include_str!(
                "../../rdf/tests/fixtures/research-objects/carrier/frictionless-data-package-1.json"
            ),
        ),
    ];

    #[test]
    fn wasm_projection_shim_is_deterministic_and_round_trips() {
        let dataset = Dataset::parse(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
            "ntriples",
            None,
        )
        .expect("parse");
        let first = dataset.project("lpg-csv", CONFIG).expect("project");
        let second = dataset.project("lpg-csv", CONFIG).expect("project again");
        assert_eq!(first.archive, second.archive);
        assert_eq!(first.profile, "lpg-csv");
        assert!(first.loss_ledger_json.contains("\"schema_version\": 1"));

        let mut lifted = lift_projection(&first.archive, "lpg-csv", CONFIG).expect("lift");
        let lifted_dataset = lifted.take_dataset().expect("dataset");
        assert_eq!(lifted_dataset.size(), 1);
        assert!(lifted.take_dataset().is_none());

        let error = parse_projection_config(MISSING_SCOPE_CONFIG).expect_err("missing scope");
        assert!(error.contains("scope"));
    }

    #[test]
    fn wasm_projection_shim_executes_every_research_object_profile() {
        for &(profile, config) in RESEARCH_CONFIGS {
            let dataset = Dataset::parse(RESEARCH_SOURCE, "turtle", None).expect("parse source");
            let first = dataset.project(profile, config).expect("project profile");
            let second = dataset.project(profile, config).expect("repeat profile");
            assert_eq!(first.profile, profile);
            assert_eq!(first.archive, second.archive);
            let mut lifted =
                lift_projection(&first.archive, profile, config).expect("lift profile");
            assert!(lifted.take_dataset().is_some());
        }
    }

    #[test]
    fn wasm_projection_shim_exposes_write_only_curated_csvw_terms() {
        let dataset = Dataset::parse(
            "<https://example.org/term> <https://example.org/label> \"Term\" .\n",
            "ntriples",
            None,
        )
        .expect("parse terms source");
        let first = dataset
            .project("csvw-terms", CSVW_TERMS_CONFIG)
            .expect("project terms");
        let second = dataset
            .project("csvw-terms", CSVW_TERMS_CONFIG)
            .expect("repeat terms");
        assert_eq!(first.profile, "csvw-terms");
        assert_eq!(first.archive, second.archive);
        assert!("csvw-terms".parse::<LiftProfile>().is_err());
    }

    #[test]
    fn wasm_projection_shim_exposes_write_only_curated_okf_terms() {
        let dataset = Dataset::parse(OKF_TERMS_SOURCE, "trig", None).expect("parse terms source");
        let first = dataset
            .project("okf-terms", OKF_TERMS_CONFIG)
            .expect("project OKF terms");
        let second = dataset
            .project("okf-terms", OKF_TERMS_CONFIG)
            .expect("repeat OKF terms");
        assert_eq!(first.profile, "okf-terms");
        assert_eq!(first.archive, second.archive);
        assert!(first.loss_ledger_json.contains("named-graph-dropped"));
        assert!("okf-terms".parse::<LiftProfile>().is_err());
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unified deterministic archive surface for graph, tabular, and research-object projections.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use purrdf_core::{DatasetView, LossLedger, RdfDataset};
use serde::{Deserialize, Serialize};

use super::{
    CroissantConfig, CsvwConfig, DataCiteConfig, DcatConfig, FrictionlessConfig, LpgConfig,
    OboGraphsConfig, ProjectionError, ProjectionLimits, ProjectionPackage, RoCrateConfig,
    SkosConfig, lift_lpg, project_croissant, project_csvw_exact, project_datacite, project_dcat,
    project_frictionless, project_lpg_csv, project_lpg_cypher, project_lpg_graphml,
    project_neo4j_csv, project_obo_graphs, project_ro_crate, project_skos, read_croissant,
    read_csvw_exact, read_datacite, read_dcat, read_frictionless, read_lpg_csv, read_lpg_cypher,
    read_lpg_graphml, read_neo4j_csv, read_ro_crate,
};

const OBO_GRAPHS_PATH: &str = "obo-graphs.json";
const SKOS_PATH: &str = "skos.ttl";

/// Closed set of RDF projection archive profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionProfile {
    /// Generic deterministic LPG CSV package.
    LpgCsv,
    /// Neo4j Admin Import CSV package.
    Neo4jCsv,
    /// Closed deterministic openCypher package.
    OpenCypher,
    /// GraphML 1.0 package.
    Graphml,
    /// Exact, lossless RDF 1.2 CSVW package.
    CsvwExact,
    /// OBO Graphs 0.3.2 JSON view (write-only).
    OboGraphs,
    /// SKOS Turtle concept-scheme view (write-only).
    Skos,
    /// Croissant 1.1 research-object package.
    #[serde(rename = "croissant-1.1")]
    Croissant11,
    /// RO-Crate 1.3 research-object package.
    #[serde(rename = "ro-crate-1.3")]
    RoCrate13,
    /// DataCite Metadata Schema 4.6 package.
    #[serde(rename = "datacite-4.6")]
    DataCite46,
    /// DCAT 3 research-object package.
    #[serde(rename = "dcat-3")]
    Dcat3,
    /// Frictionless Data Package v1.
    #[serde(rename = "frictionless-data-package-1")]
    FrictionlessDataPackage1,
}

impl ProjectionProfile {
    /// Stable command/config spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LpgCsv => "lpg-csv",
            Self::Neo4jCsv => "neo4j-csv",
            Self::OpenCypher => "open-cypher",
            Self::Graphml => "graphml",
            Self::CsvwExact => "csvw-exact",
            Self::OboGraphs => "obo-graphs",
            Self::Skos => "skos",
            Self::Croissant11 => "croissant-1.1",
            Self::RoCrate13 => "ro-crate-1.3",
            Self::DataCite46 => "datacite-4.6",
            Self::Dcat3 => "dcat-3",
            Self::FrictionlessDataPackage1 => "frictionless-data-package-1",
        }
    }

    /// Whether this carrier has a strict package reader and RDF lift path.
    pub const fn is_bidirectional(self) -> bool {
        matches!(
            self,
            Self::LpgCsv
                | Self::Neo4jCsv
                | Self::OpenCypher
                | Self::Graphml
                | Self::CsvwExact
                | Self::Croissant11
                | Self::RoCrate13
                | Self::DataCite46
                | Self::Dcat3
                | Self::FrictionlessDataPackage1
        )
    }
}

impl fmt::Display for ProjectionProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProjectionProfile {
    type Err = ProjectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "lpg-csv" => Ok(Self::LpgCsv),
            "neo4j-csv" => Ok(Self::Neo4jCsv),
            "open-cypher" => Ok(Self::OpenCypher),
            "graphml" => Ok(Self::Graphml),
            "csvw-exact" => Ok(Self::CsvwExact),
            "obo-graphs" => Ok(Self::OboGraphs),
            "skos" => Ok(Self::Skos),
            "croissant-1.1" => Ok(Self::Croissant11),
            "ro-crate-1.3" => Ok(Self::RoCrate13),
            "datacite-4.6" => Ok(Self::DataCite46),
            "dcat-3" => Ok(Self::Dcat3),
            "frictionless-data-package-1" => Ok(Self::FrictionlessDataPackage1),
            other => Err(ProjectionError::configuration(format!(
                "unknown projection profile `{other}`"
            ))),
        }
    }
}

/// Closed set of profiles accepted by the lift operation.
///
/// OBO Graphs and SKOS cannot be constructed as this type: they are deliberately
/// write-only views rather than pretend round-trip carriers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LiftProfile {
    /// Generic deterministic LPG CSV package.
    LpgCsv,
    /// Neo4j Admin Import CSV package.
    Neo4jCsv,
    /// Closed deterministic openCypher package.
    OpenCypher,
    /// GraphML 1.0 package.
    Graphml,
    /// Exact, lossless RDF 1.2 CSVW package.
    CsvwExact,
    /// Croissant 1.1 research-object package.
    #[serde(rename = "croissant-1.1")]
    Croissant11,
    /// RO-Crate 1.3 research-object package.
    #[serde(rename = "ro-crate-1.3")]
    RoCrate13,
    /// DataCite Metadata Schema 4.6 package.
    #[serde(rename = "datacite-4.6")]
    DataCite46,
    /// DCAT 3 research-object package.
    #[serde(rename = "dcat-3")]
    Dcat3,
    /// Frictionless Data Package v1.
    #[serde(rename = "frictionless-data-package-1")]
    FrictionlessDataPackage1,
}

impl LiftProfile {
    /// Stable command/config spelling.
    pub const fn as_str(self) -> &'static str {
        self.projection_profile().as_str()
    }

    /// Corresponding projection profile.
    pub const fn projection_profile(self) -> ProjectionProfile {
        match self {
            Self::LpgCsv => ProjectionProfile::LpgCsv,
            Self::Neo4jCsv => ProjectionProfile::Neo4jCsv,
            Self::OpenCypher => ProjectionProfile::OpenCypher,
            Self::Graphml => ProjectionProfile::Graphml,
            Self::CsvwExact => ProjectionProfile::CsvwExact,
            Self::Croissant11 => ProjectionProfile::Croissant11,
            Self::RoCrate13 => ProjectionProfile::RoCrate13,
            Self::DataCite46 => ProjectionProfile::DataCite46,
            Self::Dcat3 => ProjectionProfile::Dcat3,
            Self::FrictionlessDataPackage1 => ProjectionProfile::FrictionlessDataPackage1,
        }
    }
}

impl fmt::Display for LiftProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LiftProfile {
    type Err = ProjectionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "lpg-csv" => Ok(Self::LpgCsv),
            "neo4j-csv" => Ok(Self::Neo4jCsv),
            "open-cypher" => Ok(Self::OpenCypher),
            "graphml" => Ok(Self::Graphml),
            "csvw-exact" => Ok(Self::CsvwExact),
            "croissant-1.1" => Ok(Self::Croissant11),
            "ro-crate-1.3" => Ok(Self::RoCrate13),
            "datacite-4.6" => Ok(Self::DataCite46),
            "dcat-3" => Ok(Self::Dcat3),
            "frictionless-data-package-1" => Ok(Self::FrictionlessDataPackage1),
            other => Err(ProjectionError::configuration(format!(
                "profile `{other}` is not a bidirectional projection carrier"
            ))),
        }
    }
}

/// Profile-tagged, caller-owned projection configuration.
///
/// The JSON representation is `{ "profile": "…", "config": { … } }` and
/// rejects unknown fields at both layers. Each profile variant carries the exact
/// mandatory configuration its engine requires; no library vocabulary or limits
/// are synthesized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "profile",
    content = "config",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum ProjectionConfig {
    /// Generic deterministic LPG CSV configuration.
    LpgCsv(LpgConfig),
    /// Neo4j Admin Import CSV configuration.
    Neo4jCsv(LpgConfig),
    /// Closed deterministic openCypher configuration.
    OpenCypher(LpgConfig),
    /// GraphML 1.0 configuration.
    Graphml(LpgConfig),
    /// Exact RDF 1.2 CSVW configuration.
    CsvwExact(CsvwConfig),
    /// OBO Graphs 0.3.2 configuration.
    OboGraphs(Box<OboGraphsConfig>),
    /// SKOS concept-scheme configuration.
    Skos(Box<SkosConfig>),
    /// Croissant 1.1 configuration.
    #[serde(rename = "croissant-1.1")]
    Croissant11(Box<CroissantConfig>),
    /// RO-Crate 1.3 configuration.
    #[serde(rename = "ro-crate-1.3")]
    RoCrate13(Box<RoCrateConfig>),
    /// DataCite Metadata Schema 4.6 configuration.
    #[serde(rename = "datacite-4.6")]
    DataCite46(Box<DataCiteConfig>),
    /// DCAT 3 configuration.
    #[serde(rename = "dcat-3")]
    Dcat3(Box<DcatConfig>),
    /// Frictionless Data Package v1 configuration.
    #[serde(rename = "frictionless-data-package-1")]
    FrictionlessDataPackage1(Box<FrictionlessConfig>),
}

impl ProjectionConfig {
    /// Parse a strict profile-tagged JSON configuration.
    ///
    /// # Errors
    ///
    /// Returns a typed syntax/configuration error for malformed JSON, an unknown
    /// profile, an unknown field, or an invalid nested mandatory policy.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ProjectionError> {
        serde_json::from_slice(bytes).map_err(|error| {
            ProjectionError::syntax(format!("parse projection configuration JSON: {error}"))
        })
    }

    /// Deterministic compact JSON representation.
    ///
    /// # Errors
    ///
    /// Returns a typed integrity error if the validated configuration cannot be
    /// serialized.
    pub fn to_json(&self) -> Result<Vec<u8>, ProjectionError> {
        serde_json::to_vec(self).map_err(|error| {
            ProjectionError::integrity(format!("serialize projection configuration JSON: {error}"))
        })
    }

    /// Profile carried by this configuration.
    pub const fn profile(&self) -> ProjectionProfile {
        match self {
            Self::LpgCsv(_) => ProjectionProfile::LpgCsv,
            Self::Neo4jCsv(_) => ProjectionProfile::Neo4jCsv,
            Self::OpenCypher(_) => ProjectionProfile::OpenCypher,
            Self::Graphml(_) => ProjectionProfile::Graphml,
            Self::CsvwExact(_) => ProjectionProfile::CsvwExact,
            Self::OboGraphs(_) => ProjectionProfile::OboGraphs,
            Self::Skos(_) => ProjectionProfile::Skos,
            Self::Croissant11(_) => ProjectionProfile::Croissant11,
            Self::RoCrate13(_) => ProjectionProfile::RoCrate13,
            Self::DataCite46(_) => ProjectionProfile::DataCite46,
            Self::Dcat3(_) => ProjectionProfile::Dcat3,
            Self::FrictionlessDataPackage1(_) => ProjectionProfile::FrictionlessDataPackage1,
        }
    }

    /// Resource limits governing this projection and its archive.
    pub const fn limits(&self) -> ProjectionLimits {
        match self {
            Self::LpgCsv(config)
            | Self::Neo4jCsv(config)
            | Self::OpenCypher(config)
            | Self::Graphml(config) => config.limits(),
            Self::CsvwExact(config) => config.limits(),
            Self::OboGraphs(config) => config.limits(),
            Self::Skos(config) => config.limits(),
            Self::Croissant11(config) => config.common().limits(),
            Self::RoCrate13(config) => config.common().limits(),
            Self::DataCite46(config) => config.common().limits(),
            Self::Dcat3(config) => config.common().limits(),
            Self::FrictionlessDataPackage1(config) => config.common().limits(),
        }
    }

    fn require_profile(&self, profile: ProjectionProfile) -> Result<(), ProjectionError> {
        if self.profile() != profile {
            return Err(ProjectionError::configuration(format!(
                "projection profile `{profile}` does not match tagged configuration profile `{}`",
                self.profile()
            )));
        }
        Ok(())
    }
}

/// Deterministic USTAR projection plus its always-computed runtime loss ledger.
#[derive(Debug, Clone)]
pub struct ProjectionArchive {
    /// Exact carrier profile used to construct the archive.
    pub profile: ProjectionProfile,
    /// Canonical deterministic USTAR bytes.
    pub archive: Vec<u8>,
    /// Located runtime loss ledger; display policy remains a caller concern.
    pub loss_ledger: LossLedger,
}

/// Dataset reconstructed from a bidirectional projection carrier.
#[derive(Debug, Clone)]
pub struct ProjectionLift {
    /// Validated RDF 1.2 dataset.
    pub dataset: Arc<RdfDataset>,
    /// Always-computed carrier→RDF loss ledger.
    pub loss_ledger: LossLedger,
}

/// Project a dataset view into one deterministic USTAR carrier archive.
///
/// # Errors
///
/// Returns a typed configuration, model, package, serialization, integrity, or
/// resource-limit failure. `profile` must exactly match the tagged configuration.
pub fn project_archive<D: DatasetView>(
    view: &D,
    profile: ProjectionProfile,
    config: &ProjectionConfig,
) -> Result<ProjectionArchive, ProjectionError> {
    config.require_profile(profile)?;
    let (package, loss_ledger) = match config {
        ProjectionConfig::LpgCsv(config) => {
            let outcome = project_lpg_csv(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::Neo4jCsv(config) => {
            let outcome = project_neo4j_csv(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::OpenCypher(config) => {
            let outcome = project_lpg_cypher(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::Graphml(config) => {
            let outcome = project_lpg_graphml(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::CsvwExact(config) => {
            let outcome = project_csvw_exact(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::OboGraphs(config) => {
            let outcome = project_obo_graphs(view, config)?;
            let bytes = outcome.document.to_canonical_json(config)?;
            let package =
                ProjectionPackage::from_artifacts(config.limits(), [(OBO_GRAPHS_PATH, bytes)])?;
            (package, outcome.loss_ledger)
        }
        ProjectionConfig::Skos(config) => {
            let outcome = project_skos(view, config)?;
            let package =
                ProjectionPackage::from_artifacts(config.limits(), [(SKOS_PATH, outcome.turtle)])?;
            (package, outcome.loss_ledger)
        }
        ProjectionConfig::Croissant11(config) => {
            let outcome = project_croissant(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::RoCrate13(config) => {
            let outcome = project_ro_crate(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::DataCite46(config) => {
            let outcome = project_datacite(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::Dcat3(config) => {
            let outcome = project_dcat(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
        ProjectionConfig::FrictionlessDataPackage1(config) => {
            let outcome = project_frictionless(view, config)?;
            (outcome.package, outcome.loss_ledger)
        }
    };
    Ok(ProjectionArchive {
        profile,
        archive: package.to_ustar()?,
        loss_ledger,
    })
}

/// Lift one strict bidirectional carrier archive into RDF 1.2.
///
/// # Errors
///
/// Rejects malformed/non-canonical archives, a profile/config mismatch, a carrier
/// outside its closed grammar, inconsistent sideband data, or an invalid lifted
/// dataset. OBO Graphs and SKOS are unrepresentable as [`LiftProfile`].
pub fn lift_archive(
    archive: &[u8],
    profile: LiftProfile,
    config: &ProjectionConfig,
) -> Result<ProjectionLift, ProjectionError> {
    let projection_profile = profile.projection_profile();
    config.require_profile(projection_profile)?;
    let package = ProjectionPackage::from_ustar(archive, config.limits())?;
    match (profile, config) {
        (LiftProfile::LpgCsv, ProjectionConfig::LpgCsv(config)) => {
            lift_lpg_package(&read_lpg_csv(&package, config)?, config)
        }
        (LiftProfile::Neo4jCsv, ProjectionConfig::Neo4jCsv(config)) => {
            lift_lpg_package(&read_neo4j_csv(&package, config)?, config)
        }
        (LiftProfile::OpenCypher, ProjectionConfig::OpenCypher(config)) => {
            lift_lpg_package(&read_lpg_cypher(&package, config)?, config)
        }
        (LiftProfile::Graphml, ProjectionConfig::Graphml(config)) => {
            lift_lpg_package(&read_lpg_graphml(&package, config)?, config)
        }
        (LiftProfile::CsvwExact, ProjectionConfig::CsvwExact(config)) => {
            let outcome = read_csvw_exact(&package, config)?;
            Ok(ProjectionLift {
                dataset: outcome.dataset,
                loss_ledger: outcome.loss_ledger,
            })
        }
        (LiftProfile::Croissant11, ProjectionConfig::Croissant11(config)) => {
            lift_research_object_package(read_croissant(&package, config)?)
        }
        (LiftProfile::RoCrate13, ProjectionConfig::RoCrate13(config)) => {
            lift_research_object_package(read_ro_crate(&package, config)?)
        }
        (LiftProfile::DataCite46, ProjectionConfig::DataCite46(config)) => {
            lift_research_object_package(read_datacite(&package, config)?)
        }
        (LiftProfile::Dcat3, ProjectionConfig::Dcat3(config)) => {
            lift_research_object_package(read_dcat(&package, config)?)
        }
        (
            LiftProfile::FrictionlessDataPackage1,
            ProjectionConfig::FrictionlessDataPackage1(config),
        ) => lift_research_object_package(read_frictionless(&package, config)?),
        _ => unreachable!("profile/config equality was checked before dispatch"),
    }
}

fn lift_research_object_package(
    outcome: super::ResearchObjectReadOutcome,
) -> Result<ProjectionLift, ProjectionError> {
    Ok(ProjectionLift {
        dataset: outcome.dataset,
        loss_ledger: outcome.loss_ledger,
    })
}

fn lift_lpg_package(
    graph: &super::LpgGraph,
    config: &LpgConfig,
) -> Result<ProjectionLift, ProjectionError> {
    let outcome = lift_lpg(graph, config)?;
    Ok(ProjectionLift {
        dataset: outcome.dataset,
        loss_ledger: outcome.loss_ledger,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use purrdf_core::{RdfDatasetBuilder, datasets_isomorphic};

    use super::*;
    use crate::{CsvwContext, CsvwMode, CsvwVocabulary};

    const RESEARCH_SOURCE: &[u8] =
        include_bytes!("../../tests/fixtures/research-objects/carrier/shared.ttl");
    const CROISSANT_CONFIG: &[u8] =
        include_bytes!("../../tests/fixtures/research-objects/carrier/croissant-1.1.json");
    const RO_CRATE_CONFIG: &[u8] =
        include_bytes!("../../tests/fixtures/research-objects/carrier/ro-crate-1.3.json");
    const DATACITE_CONFIG: &[u8] =
        include_bytes!("../../tests/fixtures/research-objects/carrier/datacite-4.6.json");
    const DCAT_CONFIG: &[u8] =
        include_bytes!("../../tests/fixtures/research-objects/carrier/dcat-3.json");
    const FRICTIONLESS_CONFIG: &[u8] = include_bytes!(
        "../../tests/fixtures/research-objects/carrier/frictionless-data-package-1.json"
    );
    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits")
    }

    fn lpg_config(profile: ProjectionProfile) -> ProjectionConfig {
        let config = LpgConfig::new("https://example.org/type", limits(), 1_000).expect("LPG");
        match profile {
            ProjectionProfile::LpgCsv => ProjectionConfig::LpgCsv(config),
            ProjectionProfile::Neo4jCsv => ProjectionConfig::Neo4jCsv(config),
            ProjectionProfile::OpenCypher => ProjectionConfig::OpenCypher(config),
            ProjectionProfile::Graphml => ProjectionConfig::Graphml(config),
            _ => panic!("not LPG"),
        }
    }

    fn csvw_config() -> ProjectionConfig {
        ProjectionConfig::CsvwExact(
            CsvwConfig::new(
                "https://example.org/metadata",
                CsvwContext::new("https://example.org/context", BTreeMap::default())
                    .expect("context"),
                "https://example.org/group",
                CsvwVocabulary::new(
                    "https://example.org/csvw#",
                    "https://example.org/rdf#",
                    "https://example.org/rdfs#",
                    "https://example.org/xsd#",
                )
                .expect("vocabulary"),
                CsvwMode::Standard,
                limits(),
                1_000,
            )
            .expect("CSVW"),
        )
    }

    fn dataset() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/subject");
        let predicate = builder.intern_iri("https://example.org/predicate");
        let object = builder.intern_iri("https://example.org/object");
        builder.push_quad(subject, predicate, object, None);
        builder.freeze().expect("dataset")
    }

    #[test]
    fn every_bidirectional_profile_round_trips_and_is_deterministic() {
        let dataset = dataset();
        let cases = [
            (ProjectionProfile::LpgCsv, LiftProfile::LpgCsv),
            (ProjectionProfile::Neo4jCsv, LiftProfile::Neo4jCsv),
            (ProjectionProfile::OpenCypher, LiftProfile::OpenCypher),
            (ProjectionProfile::Graphml, LiftProfile::Graphml),
        ];
        for (project_profile, lift_profile) in cases {
            let config = lpg_config(project_profile);
            let first = project_archive(dataset.as_ref(), project_profile, &config).expect("first");
            let second =
                project_archive(dataset.as_ref(), project_profile, &config).expect("second");
            assert_eq!(first.archive, second.archive);
            let lifted = lift_archive(&first.archive, lift_profile, &config).expect("lift");
            assert!(datasets_isomorphic(&dataset, &lifted.dataset));
        }

        let config = csvw_config();
        let projected = project_archive(dataset.as_ref(), ProjectionProfile::CsvwExact, &config)
            .expect("CSVW project");
        let lifted =
            lift_archive(&projected.archive, LiftProfile::CsvwExact, &config).expect("CSVW lift");
        assert!(datasets_isomorphic(&dataset, &lifted.dataset));
        assert!(projected.loss_ledger.is_empty());
        assert!(lifted.loss_ledger.is_empty());
    }

    #[test]
    fn tagged_json_is_strict_and_profile_mismatch_hard_fails() {
        let config = lpg_config(ProjectionProfile::LpgCsv);
        let bytes = config.to_json().expect("JSON");
        assert_eq!(ProjectionConfig::from_json(&bytes).expect("parse"), config);
        let dataset = dataset();
        let error = project_archive(dataset.as_ref(), ProjectionProfile::Graphml, &config)
            .expect_err("mismatch");
        assert!(error.message().contains("does not match"));
        assert!(ProjectionConfig::from_json(br#"{"profile":"skos","config":{}}"#).is_err());
        assert!(
            ProjectionConfig::from_json(
                br#"{"profile":"lpg-csv","config":{"rdf_type":"https://example.org/type","limits":{"max_artifacts":16,"max_artifact_bytes":1000000,"max_total_bytes":4000000,"max_archive_bytes":5000000,"max_term_depth":16},"max_records":1000},"extra":true}"#,
            )
            .is_err()
        );
        assert!("skos".parse::<LiftProfile>().is_err());
        assert!("obo-graphs".parse::<LiftProfile>().is_err());
    }

    #[test]
    fn every_research_object_profile_uses_the_unified_stable_carrier() {
        let dataset = crate::parse_dataset(RESEARCH_SOURCE, "text/turtle", None)
            .expect("shared research-object source");
        let cases = [
            (
                ProjectionProfile::Croissant11,
                LiftProfile::Croissant11,
                CROISSANT_CONFIG,
            ),
            (
                ProjectionProfile::RoCrate13,
                LiftProfile::RoCrate13,
                RO_CRATE_CONFIG,
            ),
            (
                ProjectionProfile::DataCite46,
                LiftProfile::DataCite46,
                DATACITE_CONFIG,
            ),
            (ProjectionProfile::Dcat3, LiftProfile::Dcat3, DCAT_CONFIG),
            (
                ProjectionProfile::FrictionlessDataPackage1,
                LiftProfile::FrictionlessDataPackage1,
                FRICTIONLESS_CONFIG,
            ),
        ];

        for (project_profile, lift_profile, bytes) in cases {
            assert!(project_profile.is_bidirectional());
            assert_eq!(
                project_profile.as_str().parse::<ProjectionProfile>(),
                Ok(project_profile)
            );
            assert_eq!(
                lift_profile.as_str().parse::<LiftProfile>(),
                Ok(lift_profile)
            );
            assert_eq!(
                serde_json::to_string(&project_profile).expect("serialize project profile"),
                format!("\"{}\"", project_profile.as_str())
            );
            assert_eq!(
                serde_json::to_string(&lift_profile).expect("serialize lift profile"),
                format!("\"{}\"", lift_profile.as_str())
            );
            let config = ProjectionConfig::from_json(bytes).expect("tagged profile config");
            assert_eq!(config.profile(), project_profile);
            let encoded = config.to_json().expect("serialize config");
            assert_eq!(
                ProjectionConfig::from_json(&encoded).expect("reparse config"),
                config
            );

            let first = project_archive(dataset.as_ref(), project_profile, &config)
                .expect("project shared intersection");
            let second = project_archive(dataset.as_ref(), project_profile, &config)
                .expect("repeat shared intersection");
            assert_eq!(first.archive, second.archive, "{project_profile}");

            let lifted = lift_archive(&first.archive, lift_profile, &config).expect("lift profile");
            let rewritten = project_archive(lifted.dataset.as_ref(), project_profile, &config)
                .expect("rewrite lifted profile");
            assert_eq!(first.archive, rewritten.archive, "{project_profile}");

            for (target_profile, target_lift, target_config_bytes) in cases {
                let target_config = ProjectionConfig::from_json(target_config_bytes)
                    .expect("target tagged profile config");
                let transcoded =
                    project_archive(lifted.dataset.as_ref(), target_profile, &target_config)
                        .unwrap_or_else(|error| {
                            panic!(
                                "{project_profile} -> {target_profile} project failed: {error:?}"
                            )
                        });
                let transcoded_lift =
                    lift_archive(&transcoded.archive, target_lift, &target_config)
                        .expect("cross-profile lift");
                let stable = project_archive(
                    transcoded_lift.dataset.as_ref(),
                    target_profile,
                    &target_config,
                )
                .expect("cross-profile stable rewrite");
                assert_eq!(
                    transcoded.archive, stable.archive,
                    "{project_profile} -> {target_profile} must stabilize"
                );
            }
        }
    }
}

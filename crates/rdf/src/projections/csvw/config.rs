// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

use crate::native_codecs::jsonld::CompiledJsonLdContext;

use super::super::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// RDF conversion mode defined by the CSVW Recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CsvwMode {
    /// Emit the CSVW table-group, table, row, and source-location structure.
    Standard,
    /// Emit only the resources described by table cells.
    Minimal,
}

/// Caller-supplied RDF namespaces used by the CSVW conversion algorithm.
///
/// The W3C Recommendations define the semantic roles, but PurRDF deliberately
/// does not choose concrete vocabulary IRIs on behalf of a caller. A standards
/// profile supplies the W3C namespace IRIs explicitly; another closed deployment
/// can supply equivalent role vocabularies without changing engine behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvwVocabulary {
    #[serde(rename = "csvw_namespace")]
    csvw: String,
    #[serde(rename = "rdf_namespace")]
    rdf: String,
    #[serde(rename = "rdfs_namespace")]
    rdfs: String,
    #[serde(rename = "xsd_namespace")]
    xsd: String,
}

impl CsvwVocabulary {
    /// Construct and validate every namespace required by CSVW-to-RDF.
    ///
    /// # Errors
    ///
    /// Returns a configuration failure unless every value is an absolute IRI
    /// ending in `/` or `#`, so appending a local role name is unambiguous.
    pub fn new(
        csvw_namespace: impl Into<String>,
        rdf_namespace: impl Into<String>,
        rdfs_namespace: impl Into<String>,
        xsd_namespace: impl Into<String>,
    ) -> Result<Self, ProjectionError> {
        let csvw_namespace = validate_namespace(csvw_namespace.into(), "CSVW")?;
        let rdf_namespace = validate_namespace(rdf_namespace.into(), "RDF")?;
        let rdfs_namespace = validate_namespace(rdfs_namespace.into(), "RDFS")?;
        let xsd_namespace = validate_namespace(xsd_namespace.into(), "XSD")?;
        Ok(Self {
            csvw: csvw_namespace,
            rdf: rdf_namespace,
            rdfs: rdfs_namespace,
            xsd: xsd_namespace,
        })
    }

    /// CSVW vocabulary namespace.
    pub fn csvw_namespace(&self) -> &str {
        &self.csvw
    }

    /// RDF vocabulary namespace.
    pub fn rdf_namespace(&self) -> &str {
        &self.rdf
    }

    /// RDFS vocabulary namespace.
    pub fn rdfs_namespace(&self) -> &str {
        &self.rdfs
    }

    /// XML Schema datatype namespace.
    pub fn xsd_namespace(&self) -> &str {
        &self.xsd
    }

    pub(crate) fn csvw(&self, local: &str) -> String {
        format!("{}{local}", self.csvw)
    }

    pub(crate) fn rdf(&self, local: &str) -> String {
        format!("{}{local}", self.rdf)
    }

    pub(crate) fn rdfs(&self, local: &str) -> String {
        format!("{}{local}", self.rdfs)
    }

    pub(crate) fn xsd(&self, local: &str) -> String {
        format!("{}{local}", self.xsd)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCsvwVocabulary {
    #[serde(rename = "csvw_namespace")]
    csvw: String,
    #[serde(rename = "rdf_namespace")]
    rdf: String,
    #[serde(rename = "rdfs_namespace")]
    rdfs: String,
    #[serde(rename = "xsd_namespace")]
    xsd: String,
}

impl<'de> Deserialize<'de> for CsvwVocabulary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCsvwVocabulary::deserialize(deserializer)?;
        Self::new(raw.csvw, raw.rdf, raw.rdfs, raw.xsd).map_err(serde::de::Error::custom)
    }
}

/// Caller-owned JSON-LD context identity and compact-IRI prefix map.
///
/// PurRDF does not fetch a remote context and does not embed a prefix registry.
/// The host resolves that policy once and passes the exact expansion map used for
/// metadata annotations, URL templates, and datatype identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvwContext {
    iri: String,
    prefixes: BTreeMap<String, String>,
    #[serde(skip)]
    compiled: Arc<CompiledJsonLdContext>,
}

impl CsvwContext {
    /// Construct a validated context.
    ///
    /// # Errors
    ///
    /// Returns a configuration failure for a relative context IRI, malformed
    /// prefix, non-absolute namespace, or namespace without an explicit boundary.
    pub fn new(
        iri: impl Into<String>,
        prefixes: BTreeMap<String, String>,
    ) -> Result<Self, ProjectionError> {
        let iri = iri.into();
        validate_absolute_iri(&iri, "CSVW context")?;
        for (prefix, namespace) in &prefixes {
            validate_prefix(prefix)?;
            validate_namespace(namespace.clone(), "CSVW context prefix")?;
        }
        let compiled = CompiledJsonLdContext::from_prefixes(
            prefixes
                .iter()
                .map(|(prefix, namespace)| (prefix.clone(), namespace.clone())),
        )
        .map_err(|error| {
            ProjectionError::configuration(format!("compile CSVW JSON-LD context: {error}"))
        })?;
        Ok(Self {
            iri,
            prefixes,
            compiled: Arc::new(compiled),
        })
    }

    /// Context identity serialized into generated metadata.
    pub fn iri(&self) -> &str {
        &self.iri
    }

    /// Deterministically ordered compact-IRI namespace map.
    pub const fn prefixes(&self) -> &BTreeMap<String, String> {
        &self.prefixes
    }

    /// Expand an absolute or compact IRI using only caller-supplied context data.
    ///
    /// # Errors
    ///
    /// Returns an input failure for an unknown prefix or malformed expanded IRI.
    pub fn expand_iri(&self, value: &str) -> Result<String, ProjectionError> {
        if let Some((prefix, _)) = value.split_once(':')
            && self.prefixes.contains_key(prefix)
        {
            let expanded = self
                .compiled
                .expand_iri(value, true, false)
                .map_err(|error| {
                    ProjectionError::syntax(format!("expand CSVW compact IRI `{value}`: {error}"))
                })?
                .ok_or_else(|| {
                    ProjectionError::syntax(format!(
                        "CSVW compact IRI `{value}` has a null mapping"
                    ))
                })?;
            validate_absolute_iri(&expanded, "expanded CSVW IRI")?;
            return Ok(expanded);
        }
        if validate_absolute_iri(value, "CSVW IRI").is_ok() {
            return Ok(value.to_owned());
        }
        let (prefix, _) = value.split_once(':').ok_or_else(|| {
            ProjectionError::syntax(format!(
                "CSVW compact IRI `{value}` has no caller-supplied prefix"
            ))
        })?;
        Err(ProjectionError::syntax(format!(
            "CSVW compact IRI `{value}` uses unknown prefix `{prefix}`"
        )))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCsvwContext {
    iri: String,
    prefixes: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for CsvwContext {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCsvwContext::deserialize(deserializer)?;
        Self::new(raw.iri, raw.prefixes).map_err(serde::de::Error::custom)
    }
}

/// Mandatory identity and resource policy for CSVW processing.
///
/// There is deliberately no `Default`. The caller supplies the metadata base, the
/// CSVW context identity, and the table-group identity. PurRDF never invents an
/// application namespace or a package identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvwConfig {
    metadata_base_iri: String,
    context: CsvwContext,
    table_group_iri: String,
    vocabulary: CsvwVocabulary,
    mode: CsvwMode,
    limits: ProjectionLimits,
    max_records: usize,
}

impl CsvwConfig {
    /// Construct a validated mandatory CSVW configuration.
    ///
    /// # Errors
    ///
    /// Returns a configuration failure when an identity is not an absolute IRI or
    /// the record ceiling is zero or exceeds the portable `u32` range.
    pub fn new(
        metadata_base_iri: impl Into<String>,
        context: CsvwContext,
        table_group_iri: impl Into<String>,
        vocabulary: CsvwVocabulary,
        mode: CsvwMode,
        limits: ProjectionLimits,
        max_records: usize,
    ) -> Result<Self, ProjectionError> {
        let metadata_base_iri = metadata_base_iri.into();
        let table_group_iri = table_group_iri.into();
        validate_absolute_iri(&metadata_base_iri, "CSVW metadata base")?;
        validate_absolute_iri(&table_group_iri, "CSVW table-group identity")?;
        if max_records == 0 {
            return Err(ProjectionError::configuration(
                "CSVW max_records must be greater than zero",
            ));
        }
        if u32::try_from(max_records).is_err() {
            return Err(ProjectionError::configuration(
                "CSVW max_records exceeds the portable u32 record ceiling",
            ));
        }
        Ok(Self {
            metadata_base_iri,
            context,
            table_group_iri,
            vocabulary,
            mode,
            limits,
            max_records,
        })
    }

    /// Absolute base used to resolve metadata and table references.
    pub fn metadata_base_iri(&self) -> &str {
        &self.metadata_base_iri
    }

    /// Caller-selected CSVW context IRI required in metadata documents.
    pub fn context_iri(&self) -> &str {
        self.context.iri()
    }

    /// Caller-supplied context identity and prefix expansion policy.
    pub const fn context(&self) -> &CsvwContext {
        &self.context
    }

    /// Caller-owned identity of the table group.
    pub fn table_group_iri(&self) -> &str {
        &self.table_group_iri
    }

    /// Caller-supplied vocabulary roles used by normative RDF conversion.
    pub const fn vocabulary(&self) -> &CsvwVocabulary {
        &self.vocabulary
    }

    /// RDF conversion mode.
    pub const fn mode(&self) -> CsvwMode {
        self.mode
    }

    /// Shared artifact and recursive-term limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.limits
    }

    /// Maximum total model/table records accepted by one operation.
    pub const fn max_records(&self) -> usize {
        self.max_records
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCsvwConfig {
    metadata_base_iri: String,
    context: CsvwContext,
    table_group_iri: String,
    vocabulary: CsvwVocabulary,
    mode: CsvwMode,
    limits: ProjectionLimits,
    max_records: usize,
}

impl<'de> Deserialize<'de> for CsvwConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCsvwConfig::deserialize(deserializer)?;
        Self::new(
            raw.metadata_base_iri,
            raw.context,
            raw.table_group_iri,
            raw.vocabulary,
            raw.mode,
            raw.limits,
            raw.max_records,
        )
        .map_err(serde::de::Error::custom)
    }
}

fn validate_namespace(value: String, role: &str) -> Result<String, ProjectionError> {
    validate_absolute_iri(&value, &format!("{role} namespace"))?;
    if !value.ends_with(['/', '#']) {
        return Err(ProjectionError::configuration(format!(
            "{role} namespace must end in `/` or `#`"
        )));
    }
    Ok(value)
}

fn validate_prefix(prefix: &str) -> Result<(), ProjectionError> {
    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return Err(ProjectionError::configuration(
            "CSVW context prefixes must not be empty",
        ));
    };
    if !(first == '_' || first.is_alphabetic())
        || !chars
            .all(|character| character == '_' || character == '-' || character.is_alphanumeric())
    {
        return Err(ProjectionError::configuration(format!(
            "invalid CSVW context prefix `{prefix}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProjectionLimits {
        ProjectionLimits::new(16, 1_000_000, 4_000_000, 5_000_000, 16).expect("limits")
    }

    fn context() -> CsvwContext {
        CsvwContext::new(
            "http://www.w3.org/ns/csvw",
            BTreeMap::from([
                ("schema".to_owned(), "http://schema.org/".to_owned()),
                (
                    "xsd".to_owned(),
                    "http://www.w3.org/2001/XMLSchema#".to_owned(),
                ),
            ]),
        )
        .expect("context")
    }

    fn vocabulary() -> CsvwVocabulary {
        CsvwVocabulary::new(
            "http://www.w3.org/ns/csvw#",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
            "http://www.w3.org/2000/01/rdf-schema#",
            "http://www.w3.org/2001/XMLSchema#",
        )
        .expect("vocabulary")
    }

    #[test]
    fn config_requires_every_caller_identity_and_revalidates_json() {
        let config = CsvwConfig::new(
            "http://example.org/package/",
            context(),
            "http://example.org/table-group",
            vocabulary(),
            CsvwMode::Standard,
            limits(),
            100,
        )
        .expect("config");
        let json = serde_json::to_vec(&config).expect("JSON");
        assert_eq!(
            serde_json::from_slice::<CsvwConfig>(&json).expect("reparse"),
            config
        );
        assert!(
            CsvwConfig::new(
                "relative/",
                context(),
                "http://example.org/group",
                vocabulary(),
                CsvwMode::Minimal,
                limits(),
                1,
            )
            .is_err()
        );
        assert_eq!(
            config
                .context()
                .expand_iri("schema:name")
                .expect("expansion"),
            "http://schema.org/name"
        );
    }
}

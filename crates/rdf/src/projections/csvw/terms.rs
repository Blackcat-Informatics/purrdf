// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Caller-declared, deterministic RDF term inventories in CSVW.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::config::CsvwConfig;
use super::model::{CsvwDatatype, CsvwNaturalLanguage, CsvwTextDirection};
use crate::projections::{ProjectionError, ProjectionLimits, validate_absolute_iri};

/// Stable loss-contract target for the curated terms profile.
pub const CSVW_TERMS_PROFILE: &str = "csvw-terms";

/// Explicit RDF graph scope used to discover rows and column values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum CsvwTermsGraphSelection {
    /// Read the default graph and every declared named graph.
    All,
    /// Read exactly the named graph identities and default-graph flag supplied here.
    Include {
        /// Whether default-graph quads are in scope.
        default_graph: bool,
        /// Exact named-graph IRIs in scope.
        named_graphs: BTreeSet<String>,
    },
}

impl CsvwTermsGraphSelection {
    /// Construct and validate an exact graph selection.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for a relative named-graph IRI or an empty scope.
    pub fn include(
        default_graph: bool,
        named_graphs: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selection = Self::Include {
            default_graph,
            named_graphs,
        };
        selection.validate()?;
        Ok(selection)
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        if let Self::Include {
            default_graph,
            named_graphs,
        } = self
        {
            if !default_graph && named_graphs.is_empty() {
                return Err(ProjectionError::configuration(
                    "CSVW terms graph selection must include at least one graph",
                ));
            }
            for graph in named_graphs {
                validate_absolute_iri(graph, "CSVW terms named graph")?;
            }
        }
        Ok(())
    }
}

/// Caller-supplied RDF-type and subject-namespace membership test for one table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsSelector {
    type_predicate: Option<String>,
    any_types: BTreeSet<String>,
    all_types: BTreeSet<String>,
    none_types: BTreeSet<String>,
    iri_prefixes: BTreeSet<String>,
}

impl CsvwTermsSelector {
    /// Construct a validated table selector.
    ///
    /// Empty type sets mean that type membership does not constrain the table. Empty
    /// IRI prefixes mean that every IRI subject is eligible. A type predicate is
    /// required exactly when at least one type constraint is present.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for incomplete, contradictory, or relative IRI
    /// policy.
    pub fn new(
        type_predicate: Option<String>,
        any_types: BTreeSet<String>,
        all_types: BTreeSet<String>,
        none_types: BTreeSet<String>,
        iri_prefixes: BTreeSet<String>,
    ) -> Result<Self, ProjectionError> {
        let selector = Self {
            type_predicate,
            any_types,
            all_types,
            none_types,
            iri_prefixes,
        };
        selector.validate()?;
        Ok(selector)
    }

    /// RDF type predicate used by the membership test.
    pub fn type_predicate(&self) -> Option<&str> {
        self.type_predicate.as_deref()
    }

    /// Types of which at least one must be present, when non-empty.
    pub const fn any_types(&self) -> &BTreeSet<String> {
        &self.any_types
    }

    /// Types all of which must be present.
    pub const fn all_types(&self) -> &BTreeSet<String> {
        &self.all_types
    }

    /// Types none of which may be present.
    pub const fn none_types(&self) -> &BTreeSet<String> {
        &self.none_types
    }

    /// Allowed subject-IRI prefixes; empty means every IRI.
    pub const fn iri_prefixes(&self) -> &BTreeSet<String> {
        &self.iri_prefixes
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        let has_type_constraints =
            !(self.any_types.is_empty() && self.all_types.is_empty() && self.none_types.is_empty());
        if has_type_constraints != self.type_predicate.is_some() {
            return Err(ProjectionError::configuration(
                "CSVW terms selector requires type_predicate exactly when type constraints are present",
            ));
        }
        if let Some(predicate) = &self.type_predicate {
            validate_absolute_iri(predicate, "CSVW terms type predicate")?;
        }
        for (role, values) in [
            ("any type", &self.any_types),
            ("all type", &self.all_types),
            ("excluded type", &self.none_types),
        ] {
            for value in values {
                validate_absolute_iri(value, &format!("CSVW terms {role}"))?;
            }
        }
        if self
            .none_types
            .iter()
            .any(|value| self.any_types.contains(value) || self.all_types.contains(value))
        {
            return Err(ProjectionError::configuration(
                "CSVW terms selector cannot both require and exclude the same type",
            ));
        }
        for prefix in &self.iri_prefixes {
            validate_absolute_iri(prefix, "CSVW terms subject IRI prefix")?;
        }
        Ok(())
    }
}

/// Visible subject-identity column shared by every row in one table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsIdentityColumn {
    name: String,
    titles: CsvwNaturalLanguage,
    datatype: CsvwDatatype,
}

impl CsvwTermsIdentityColumn {
    /// Construct a validated identity column.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an unsafe name or invalid datatype IRI.
    pub fn new(
        name: impl Into<String>,
        titles: CsvwNaturalLanguage,
        datatype: CsvwDatatype,
    ) -> Result<Self, ProjectionError> {
        let column = Self {
            name: name.into(),
            titles,
            datatype,
        };
        column.validate()?;
        Ok(column)
    }

    /// URI-template variable and physical CSV column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Caller-authored localized column titles.
    pub const fn titles(&self) -> &CsvwNaturalLanguage {
        &self.titles
    }

    /// Caller-selected CSVW datatype for the visible IRI text.
    pub const fn datatype(&self) -> &CsvwDatatype {
        &self.datatype
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_column_name(&self.name)?;
        validate_titles(&self.titles)?;
        validate_datatype(&self.datatype)
    }
}

/// Exact RDF object kind accepted by a curated column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum CsvwTermsValueMode {
    /// Accept IRI objects and preserve them through CSVW `valueUrl`.
    Iri {
        /// Datatype assigned to the visible IRI cell text.
        datatype: CsvwDatatype,
    },
    /// Accept literals whose identity facets exactly match this declaration.
    Literal {
        /// Exact expanded RDF datatype IRI and CSVW value-space policy.
        datatype: CsvwDatatype,
        /// Exact lowercase language tag, when required.
        language: Option<String>,
        /// Exact RDF 1.2 base direction, when required.
        direction: Option<CsvwTextDirection>,
    },
}

impl CsvwTermsValueMode {
    /// Construct an IRI-object column mode.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an invalid datatype declaration.
    pub fn iri(datatype: CsvwDatatype) -> Result<Self, ProjectionError> {
        let mode = Self::Iri { datatype };
        mode.validate()?;
        Ok(mode)
    }

    /// Construct an exact literal-object column mode.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for invalid datatype/language/direction facets.
    pub fn literal(
        datatype: CsvwDatatype,
        language: Option<String>,
        direction: Option<CsvwTextDirection>,
    ) -> Result<Self, ProjectionError> {
        let mode = Self::Literal {
            datatype,
            language,
            direction,
        };
        mode.validate()?;
        Ok(mode)
    }

    /// CSVW datatype carried by this column.
    pub const fn datatype(&self) -> &CsvwDatatype {
        match self {
            Self::Iri { datatype } | Self::Literal { datatype, .. } => datatype,
        }
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_datatype(self.datatype())?;
        if let Self::Literal {
            language,
            direction,
            ..
        } = self
        {
            if direction.is_some() && language.is_none() {
                return Err(ProjectionError::configuration(
                    "CSVW terms literal direction requires a language tag",
                ));
            }
            if let Some(language) = language {
                validate_language(language)?;
            }
        }
        Ok(())
    }
}

/// Cardinality and deterministic multi-value encoding for one column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum CsvwTermsCardinality {
    /// Zero or one matching value; more than one is a hard error.
    One,
    /// Zero or more values joined with an explicit collision-free separator.
    Many {
        /// Separator placed between sorted cell values.
        separator: String,
        /// Whether CSVW-to-RDF interprets the values as an RDF list.
        ordered: bool,
    },
}

impl CsvwTermsCardinality {
    /// Construct a validated multi-value policy.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty or line-breaking separator.
    pub fn many(separator: impl Into<String>, ordered: bool) -> Result<Self, ProjectionError> {
        let cardinality = Self::Many {
            separator: separator.into(),
            ordered,
        };
        cardinality.validate()?;
        Ok(cardinality)
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        if let Self::Many { separator, .. } = self
            && (separator.is_empty() || separator.contains(['\r', '\n']))
        {
            return Err(ProjectionError::configuration(
                "CSVW terms multi-value separator must be non-empty and single-line",
            ));
        }
        Ok(())
    }
}

/// One caller-owned RDF predicate mapped to one ordered CSVW column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsColumn {
    name: String,
    titles: CsvwNaturalLanguage,
    predicate: String,
    value_mode: CsvwTermsValueMode,
    cardinality: CsvwTermsCardinality,
    required: bool,
}

impl CsvwTermsColumn {
    /// Construct a validated predicate column.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an unsafe name, relative predicate, invalid
    /// title map, value mode, or cardinality.
    pub fn new(
        name: impl Into<String>,
        titles: CsvwNaturalLanguage,
        predicate: impl Into<String>,
        value_mode: CsvwTermsValueMode,
        cardinality: CsvwTermsCardinality,
        required: bool,
    ) -> Result<Self, ProjectionError> {
        let column = Self {
            name: name.into(),
            titles,
            predicate: predicate.into(),
            value_mode,
            cardinality,
            required,
        };
        column.validate()?;
        Ok(column)
    }

    /// Physical column and URI-template variable name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Caller-authored localized titles.
    pub const fn titles(&self) -> &CsvwNaturalLanguage {
        &self.titles
    }

    /// Exact RDF predicate mapped to this column.
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    /// Exact accepted object kind/facets.
    pub const fn value_mode(&self) -> &CsvwTermsValueMode {
        &self.value_mode
    }

    /// Declared one/many policy.
    pub const fn cardinality(&self) -> &CsvwTermsCardinality {
        &self.cardinality
    }

    /// Whether every selected row must have at least one representable value.
    pub const fn required(&self) -> bool {
        self.required
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_column_name(&self.name)?;
        validate_titles(&self.titles)?;
        validate_absolute_iri(&self.predicate, "CSVW terms column predicate")?;
        self.value_mode.validate()?;
        self.cardinality.validate()
    }
}

/// One curated entity table and its complete mapping policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsTable {
    name: String,
    table_url: String,
    artifact_path: String,
    selector: CsvwTermsSelector,
    identity: CsvwTermsIdentityColumn,
    columns: Vec<CsvwTermsColumn>,
}

impl CsvwTermsTable {
    /// Construct and validate one table declaration.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for invalid identity/path policy, no predicate
    /// columns, or duplicate column names/predicates.
    pub fn new(
        name: impl Into<String>,
        table_url: impl Into<String>,
        artifact_path: impl Into<String>,
        selector: CsvwTermsSelector,
        identity: CsvwTermsIdentityColumn,
        columns: Vec<CsvwTermsColumn>,
    ) -> Result<Self, ProjectionError> {
        let table = Self {
            name: name.into(),
            table_url: table_url.into(),
            artifact_path: artifact_path.into(),
            selector,
            identity,
            columns,
        };
        table.validate()?;
        Ok(table)
    }

    /// Stable caller-facing table name used in diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Absolute CSV resource URL carried by CSVW metadata.
    pub fn table_url(&self) -> &str {
        &self.table_url
    }

    /// Safe package-relative CSV artifact path.
    pub fn artifact_path(&self) -> &str {
        &self.artifact_path
    }

    /// Row-membership selector.
    pub const fn selector(&self) -> &CsvwTermsSelector {
        &self.selector
    }

    /// Subject identity column.
    pub const fn identity(&self) -> &CsvwTermsIdentityColumn {
        &self.identity
    }

    /// Ordered predicate columns.
    pub fn columns(&self) -> &[CsvwTermsColumn] {
        &self.columns
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_table_name(&self.name)?;
        validate_absolute_iri(&self.table_url, "CSVW terms table URL")?;
        validate_artifact_path(&self.artifact_path)?;
        self.selector.validate()?;
        self.identity.validate()?;
        if self.columns.is_empty() {
            return Err(ProjectionError::configuration(
                "CSVW terms table requires at least one predicate column",
            ));
        }
        let mut names = BTreeSet::from([self.identity.name.clone()]);
        let mut predicates = BTreeSet::new();
        for column in &self.columns {
            column.validate()?;
            if !names.insert(column.name.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW terms column name `{}` in table `{}`",
                    column.name, self.name
                )));
            }
            if !predicates.insert(column.predicate.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW terms predicate `{}` in table `{}`",
                    column.predicate, self.name
                )));
            }
        }
        Ok(())
    }
}

/// Portable execution ceilings specific to curated wide tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsLimits {
    #[serde(rename = "max_rows")]
    rows: usize,
    #[serde(rename = "max_values")]
    values: usize,
    #[serde(rename = "max_values_per_cell")]
    values_per_cell: usize,
}

impl CsvwTermsLimits {
    /// Construct non-zero portable ceilings.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for zero or values beyond `u32`.
    pub fn new(
        max_rows: usize,
        max_values: usize,
        max_values_per_cell: usize,
    ) -> Result<Self, ProjectionError> {
        for (name, value) in [
            ("max_rows", max_rows),
            ("max_values", max_values),
            ("max_values_per_cell", max_values_per_cell),
        ] {
            if value == 0 || u32::try_from(value).is_err() {
                return Err(ProjectionError::configuration(format!(
                    "CSVW terms {name} must be in 1..=u32::MAX"
                )));
            }
        }
        Ok(Self {
            rows: max_rows,
            values: max_values,
            values_per_cell: max_values_per_cell,
        })
    }

    /// Maximum rows across all output tables.
    pub const fn max_rows(self) -> usize {
        self.rows
    }

    /// Maximum represented values across all predicate cells.
    pub const fn max_values(self) -> usize {
        self.values
    }

    /// Maximum represented values in one cell.
    pub const fn max_values_per_cell(self) -> usize {
        self.values_per_cell
    }
}

/// Complete mandatory configuration for the write-only `csvw-terms` profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvwTermsConfig {
    csvw: CsvwConfig,
    metadata_path: String,
    graph_selection: CsvwTermsGraphSelection,
    tables: Vec<CsvwTermsTable>,
    execution_limits: CsvwTermsLimits,
}

impl CsvwTermsConfig {
    /// Construct and validate a complete curated terms profile.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for unsafe/duplicate artifacts, duplicate table
    /// identities/names, invalid nested policies, or an empty table list.
    pub fn new(
        csvw: CsvwConfig,
        metadata_path: impl Into<String>,
        graph_selection: CsvwTermsGraphSelection,
        tables: Vec<CsvwTermsTable>,
        execution_limits: CsvwTermsLimits,
    ) -> Result<Self, ProjectionError> {
        let config = Self {
            csvw,
            metadata_path: metadata_path.into(),
            graph_selection,
            tables,
            execution_limits,
        };
        config.validate()?;
        Ok(config)
    }

    /// Shared normative CSVW configuration.
    pub const fn csvw(&self) -> &CsvwConfig {
        &self.csvw
    }

    /// Package-relative metadata artifact path.
    pub fn metadata_path(&self) -> &str {
        &self.metadata_path
    }

    /// Explicit source graph scope.
    pub const fn graph_selection(&self) -> &CsvwTermsGraphSelection {
        &self.graph_selection
    }

    /// Ordered entity-table declarations.
    pub fn tables(&self) -> &[CsvwTermsTable] {
        &self.tables
    }

    /// Curated-table execution ceilings.
    pub const fn execution_limits(&self) -> CsvwTermsLimits {
        self.execution_limits
    }

    /// Shared package/term recursion limits.
    pub const fn limits(&self) -> ProjectionLimits {
        self.csvw.limits()
    }

    fn validate(&self) -> Result<(), ProjectionError> {
        validate_artifact_path(&self.metadata_path)?;
        self.graph_selection.validate()?;
        if self.tables.is_empty() {
            return Err(ProjectionError::configuration(
                "CSVW terms configuration requires at least one table",
            ));
        }
        let mut names = BTreeSet::new();
        let mut urls = BTreeSet::new();
        let mut paths = BTreeSet::from([self.metadata_path.clone()]);
        for table in &self.tables {
            table.validate()?;
            if !names.insert(table.name.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW terms table name `{}`",
                    table.name
                )));
            }
            if !urls.insert(table.table_url.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW terms table URL `{}`",
                    table.table_url
                )));
            }
            if !paths.insert(table.artifact_path.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW terms artifact path `{}`",
                    table.artifact_path
                )));
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCsvwTermsConfig {
    csvw: CsvwConfig,
    metadata_path: String,
    graph_selection: CsvwTermsGraphSelection,
    tables: Vec<CsvwTermsTable>,
    execution_limits: CsvwTermsLimits,
}

impl<'de> Deserialize<'de> for CsvwTermsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCsvwTermsConfig::deserialize(deserializer)?;
        Self::new(
            raw.csvw,
            raw.metadata_path,
            raw.graph_selection,
            raw.tables,
            raw.execution_limits,
        )
        .map_err(serde::de::Error::custom)
    }
}

fn validate_datatype(datatype: &CsvwDatatype) -> Result<(), ProjectionError> {
    validate_absolute_iri(&datatype.base, "CSVW terms datatype base")?;
    if let Some(id) = &datatype.id {
        validate_absolute_iri(id, "CSVW terms datatype identity")?;
    }
    Ok(())
}

fn validate_language(language: &str) -> Result<(), ProjectionError> {
    let valid = !language.is_empty()
        && language == language.to_lowercase()
        && language.split('-').all(|part| {
            !part.is_empty()
                && part.len() <= 8
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        });
    if valid {
        Ok(())
    } else {
        Err(ProjectionError::configuration(format!(
            "invalid lowercase CSVW terms language tag `{language}`"
        )))
    }
}

fn validate_titles(titles: &CsvwNaturalLanguage) -> Result<(), ProjectionError> {
    for (language, values) in titles {
        if !language.is_empty() {
            validate_language(language)?;
        }
        if values.is_empty() || values.iter().any(String::is_empty) {
            return Err(ProjectionError::configuration(
                "CSVW terms title maps require non-empty value lists and strings",
            ));
        }
    }
    Ok(())
}

fn validate_table_name(name: &str) -> Result<(), ProjectionError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(ProjectionError::configuration(format!(
            "invalid CSVW terms table name `{name}`"
        )));
    }
    Ok(())
}

fn validate_column_name(name: &str) -> Result<(), ProjectionError> {
    if name.is_empty()
        || name.starts_with('_')
        || name.contains(['{', '}', '\r', '\n'])
        || name.chars().any(char::is_whitespace)
    {
        return Err(ProjectionError::configuration(format!(
            "invalid CSVW terms column name `{name}`"
        )));
    }
    Ok(())
}

fn validate_artifact_path(path: &str) -> Result<(), ProjectionError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.split('/').any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || segment.contains(['\\', '\0', '\r', '\n'])
        })
    {
        return Err(ProjectionError::configuration(format!(
            "unsafe CSVW terms artifact path `{path}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::projections::{CsvwContext, CsvwMode, CsvwVocabulary};

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

    fn datatype(base: &str) -> CsvwDatatype {
        CsvwDatatype {
            id: None,
            base: base.to_owned(),
            format: None,
            length: None,
            min_length: None,
            max_length: None,
            minimum: None,
            maximum: None,
            min_inclusive: None,
            max_inclusive: None,
            min_exclusive: None,
            max_exclusive: None,
        }
    }

    fn csvw() -> CsvwConfig {
        CsvwConfig::new(
            "https://example.org/catalog/metadata.json",
            CsvwContext::new(
                "http://www.w3.org/ns/csvw",
                BTreeMap::from([("xsd".to_owned(), XSD.to_owned())]),
            )
            .expect("context"),
            "https://example.org/catalog",
            CsvwVocabulary::new(
                "http://www.w3.org/ns/csvw#",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "http://www.w3.org/2000/01/rdf-schema#",
                XSD,
            )
            .expect("vocabulary"),
            CsvwMode::Minimal,
            ProjectionLimits::new(16, 1_000_000, 8_000_000, 16_000_000, 16).expect("limits"),
            10_000,
        )
        .expect("CSVW")
    }

    fn table() -> CsvwTermsTable {
        CsvwTermsTable::new(
            "classes",
            "https://example.org/catalog/classes.csv",
            "classes.csv",
            CsvwTermsSelector::new(
                Some("https://example.org/type".to_owned()),
                BTreeSet::from(["https://example.org/Class".to_owned()]),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::from(["https://example.org/vocab/".to_owned()]),
            )
            .expect("selector"),
            CsvwTermsIdentityColumn::new(
                "iri",
                BTreeMap::from([("".to_owned(), vec!["IRI".to_owned()])]),
                datatype(&format!("{XSD}anyURI")),
            )
            .expect("identity"),
            vec![
                CsvwTermsColumn::new(
                    "label",
                    BTreeMap::new(),
                    "https://example.org/label",
                    CsvwTermsValueMode::literal(datatype(&format!("{XSD}string")), None, None)
                        .expect("literal mode"),
                    CsvwTermsCardinality::One,
                    false,
                )
                .expect("column"),
            ],
        )
        .expect("table")
    }

    #[test]
    fn complete_config_round_trips_through_strict_json() {
        let config = CsvwTermsConfig::new(
            csvw(),
            "csvw-metadata.json",
            CsvwTermsGraphSelection::include(true, BTreeSet::new()).expect("scope"),
            vec![table()],
            CsvwTermsLimits::new(1_000, 10_000, 100).expect("execution limits"),
        )
        .expect("config");
        let bytes = serde_json::to_vec(&config).expect("serialize");
        let reparsed: CsvwTermsConfig = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(config, reparsed);
    }

    #[test]
    fn rejects_ambiguous_and_incomplete_policy() {
        assert!(CsvwTermsGraphSelection::include(false, BTreeSet::new()).is_err());
        assert!(
            CsvwTermsSelector::new(
                None,
                BTreeSet::from(["https://example.org/Class".to_owned()]),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
            )
            .is_err()
        );
        assert!(CsvwTermsLimits::new(0, 1, 1).is_err());

        let mut duplicate = table();
        duplicate.artifact_path = "csvw-metadata.json".to_owned();
        assert!(
            CsvwTermsConfig::new(
                csvw(),
                "csvw-metadata.json",
                CsvwTermsGraphSelection::All,
                vec![duplicate],
                CsvwTermsLimits::new(1, 1, 1).expect("limits"),
            )
            .is_err()
        );
    }

    #[test]
    fn strict_config_rejects_unknown_nested_fields() {
        let config = CsvwTermsConfig::new(
            csvw(),
            "csvw-metadata.json",
            CsvwTermsGraphSelection::All,
            vec![table()],
            CsvwTermsLimits::new(10, 20, 5).expect("limits"),
        )
        .expect("config");
        let mut value = serde_json::to_value(config).expect("value");
        value["tables"][0]["mystery"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CsvwTermsConfig>(value).is_err());
    }
}

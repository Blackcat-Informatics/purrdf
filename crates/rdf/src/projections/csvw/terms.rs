// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Caller-declared, deterministic RDF term inventories in CSVW.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use purrdf_core::loss::{
    LOSS_CSVW_TERMS_ANNOTATION_DROPPED, LOSS_CSVW_TERMS_EMPTY_GRAPH_DROPPED,
    LOSS_CSVW_TERMS_GRAPH_UNSELECTED, LOSS_CSVW_TERMS_NAMED_GRAPH_PLACEMENT_DROPPED,
    LOSS_CSVW_TERMS_OBJECT_UNREPRESENTABLE, LOSS_CSVW_TERMS_PREDICATE_UNMAPPED,
    LOSS_CSVW_TERMS_REIFIER_DROPPED, LOSS_CSVW_TERMS_SUBJECT_UNREPRESENTABLE,
    LOSS_CSVW_TERMS_SUBJECT_UNSELECTED,
};
use purrdf_core::{
    DatasetView, LossEntry, LossLedger, RdfLocation, check_ledger_sound,
    rdf_to_csvw_terms_loss_ledger,
};
use serde::{Deserialize, Deserializer, Serialize};

use super::config::CsvwConfig;
use super::input::CsvwInput;
use super::model::{
    CsvwAnnotations, CsvwCell, CsvwColumn, CsvwDatatype, CsvwDialect, CsvwInheritedProperties,
    CsvwNaturalLanguage, CsvwRow, CsvwSchema, CsvwTable, CsvwTableDirection, CsvwTableGroup,
    CsvwTextDirection, CsvwTrim, CsvwValue,
};
use super::writer::{CsvwWritePlan, write_csvw};
use crate::projections::{
    ProjectionDirection, ProjectionError, ProjectionLimits, ProjectionPackage, ProjectionTerm,
    stable_identifier, validate_absolute_iri,
};

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
        direction: Option<ProjectionDirection>,
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
        direction: Option<ProjectionDirection>,
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
    },
}

impl CsvwTermsCardinality {
    /// Construct a validated multi-value policy.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an empty or line-breaking separator.
    pub fn many(separator: impl Into<String>) -> Result<Self, ProjectionError> {
        let cardinality = Self::Many {
            separator: separator.into(),
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
        self.cardinality.validate()?;
        Ok(())
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
            for column in &table.columns {
                if let CsvwTermsValueMode::Literal {
                    datatype, language, ..
                } = &column.value_mode
                {
                    let csvw_string = self.csvw.vocabulary().xsd("string");
                    if language.is_some() && datatype.base != csvw_string {
                        return Err(ProjectionError::configuration(format!(
                            "CSVW terms literal column `{}` must use the caller XSD string datatype when it declares a language",
                            column.name
                        )));
                    }
                    if datatype.base == self.csvw.vocabulary().rdf("langString") {
                        return Err(ProjectionError::configuration(format!(
                            "CSVW terms literal column `{}` must express language through the CSVW lang property, not an RDF langString datatype",
                            column.name
                        )));
                    }
                }
            }
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

/// Deterministic execution counts for one curated terms projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTermsReport {
    /// Source named-graph, quad, reifier, and annotation rows examined.
    pub source_records: usize,
    /// Source quads inside the explicit graph scope.
    pub scoped_quads: usize,
    /// Number of tables emitted.
    pub tables: usize,
    /// Total rows emitted across all tables.
    pub rows: usize,
    /// Total non-null predicate values emitted across all tables.
    pub values: usize,
}

/// Curated CSVW package, normalized table model, and complete runtime ledger.
#[derive(Debug, Clone)]
pub struct CsvwTermsProjection {
    /// Filesystem-free deterministic artifact package.
    pub package: ProjectionPackage,
    /// IRI-keyed resources accepted directly by [`super::read_csvw`].
    pub input: CsvwInput,
    /// Fully normalized table group used to write the package.
    pub table_group: CsvwTableGroup,
    /// Deterministic execution counts.
    pub report: CsvwTermsReport,
    /// Located losses for every source row not carried exactly by the tables.
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

struct TermsProjector<'a> {
    config: &'a CsvwTermsConfig,
    named_graphs: Vec<ProjectionTerm>,
    quads: Vec<SourceQuad>,
    reifiers: Vec<SourceReifier>,
    annotations: Vec<SourceAnnotation>,
    ledger: LossLedger,
    contract: LossLedger,
}

/// Project any RDF 1.2 dataset backend into caller-declared curated CSVW tables.
///
/// The exact CSVW profile remains the archival carrier. This write-only profile
/// creates a deterministic wide-table view and records every omitted source row in
/// its closed loss ledger.
///
/// # Errors
///
/// Returns a typed configuration, term, integrity, cardinality, separator,
/// serialization, package, or resource-limit failure.
pub fn project_csvw_terms<D: DatasetView>(
    view: &D,
    config: &CsvwTermsConfig,
) -> Result<CsvwTermsProjection, ProjectionError> {
    TermsProjector::load(view, config)?.project()
}

impl<'a> TermsProjector<'a> {
    fn load<D: DatasetView>(
        view: &D,
        config: &'a CsvwTermsConfig,
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
        reject_duplicates(&annotations, "RDF statement annotations")?;

        let source_records = named_graphs
            .len()
            .checked_add(quads.len())
            .and_then(|count| count.checked_add(reifiers.len()))
            .and_then(|count| count.checked_add(annotations.len()))
            .ok_or_else(|| ProjectionError::limit("CSVW terms input record count overflow"))?;
        if source_records > config.csvw.max_records() {
            return Err(ProjectionError::limit(format!(
                "CSVW terms input has {source_records} records; limit is {}",
                config.csvw.max_records()
            )));
        }

        Ok(Self {
            config,
            named_graphs,
            quads,
            reifiers,
            annotations,
            ledger: LossLedger::new(),
            contract: rdf_to_csvw_terms_loss_ledger(),
        })
    }

    fn project(mut self) -> Result<CsvwTermsProjection, ProjectionError> {
        let scoped_quads = self
            .quads
            .iter()
            .filter(|quad| self.graph_selected(quad.graph.as_ref()))
            .count();
        let index = self.build_subject_index();
        let memberships = self.table_memberships(&index);
        let (table_group, rows, values) = self.build_table_group(&index, &memberships)?;
        self.record_quad_losses(&memberships)?;
        self.record_structural_losses()?;
        check_ledger_sound(&self.ledger, "rdf-1.2-dataset", CSVW_TERMS_PROFILE)
            .map_err(ProjectionError::integrity)?;

        let plan = CsvwWritePlan::new(
            self.config.metadata_path.clone(),
            self.config
                .tables
                .iter()
                .map(|table| (table.table_url.clone(), table.artifact_path.clone()))
                .collect(),
        )?;
        let written = write_csvw(&table_group, &plan, &self.config.csvw)?;
        let source_records = self
            .named_graphs
            .len()
            .checked_add(self.quads.len())
            .and_then(|count| count.checked_add(self.reifiers.len()))
            .and_then(|count| count.checked_add(self.annotations.len()))
            .ok_or_else(|| ProjectionError::limit("CSVW terms report count overflow"))?;
        Ok(CsvwTermsProjection {
            package: written.package,
            input: written.input,
            table_group,
            report: CsvwTermsReport {
                source_records,
                scoped_quads,
                tables: self.config.tables.len(),
                rows,
                values,
            },
            loss_ledger: self.ledger,
        })
    }

    fn build_subject_index(&self) -> BTreeMap<String, BTreeMap<String, BTreeSet<ProjectionTerm>>> {
        let mut index: BTreeMap<String, BTreeMap<String, BTreeSet<ProjectionTerm>>> =
            BTreeMap::new();
        for quad in &self.quads {
            if !self.graph_selected(quad.graph.as_ref()) {
                continue;
            }
            let ProjectionTerm::Iri { value: subject } = &quad.subject else {
                continue;
            };
            index
                .entry(subject.clone())
                .or_default()
                .entry(quad.predicate.clone())
                .or_default()
                .insert(quad.object.clone());
        }
        index
    }

    fn table_memberships<'index>(
        &self,
        index: &'index BTreeMap<String, BTreeMap<String, BTreeSet<ProjectionTerm>>>,
    ) -> Vec<Vec<&'index str>> {
        self.config
            .tables
            .iter()
            .map(|table| {
                index
                    .iter()
                    .filter(|(subject, predicates)| {
                        selector_matches(&table.selector, subject, predicates)
                    })
                    .map(|(subject, _)| subject.as_str())
                    .collect()
            })
            .collect()
    }

    fn build_table_group(
        &self,
        index: &BTreeMap<String, BTreeMap<String, BTreeSet<ProjectionTerm>>>,
        memberships: &[Vec<&str>],
    ) -> Result<(CsvwTableGroup, usize, usize), ProjectionError> {
        let mut tables = Vec::with_capacity(self.config.tables.len());
        let mut row_count = 0usize;
        let mut value_count = 0usize;
        for (declaration, subjects) in self.config.tables.iter().zip(memberships) {
            let mut rows = Vec::with_capacity(subjects.len());
            for &subject in subjects {
                row_count = row_count
                    .checked_add(1)
                    .ok_or_else(|| ProjectionError::limit("CSVW terms row count overflow"))?;
                if row_count > self.config.execution_limits.max_rows() {
                    return Err(ProjectionError::limit(format!(
                        "CSVW terms output exceeds the {}-row limit",
                        self.config.execution_limits.max_rows()
                    )));
                }
                let predicates = index.get(subject).ok_or_else(|| {
                    ProjectionError::integrity("selected CSVW terms subject is absent from index")
                })?;
                let mut cells = Vec::with_capacity(declaration.columns.len() + 1);
                cells.push(identity_cell(subject, &declaration.identity));
                for column in &declaration.columns {
                    let objects =
                        predicates
                            .get(&column.predicate)
                            .map_or_else(Vec::new, |objects| {
                                objects
                                    .iter()
                                    .filter_map(|object| {
                                        cell_value(object, &column.value_mode, &self.config.csvw)
                                    })
                                    .collect::<Vec<_>>()
                            });
                    if objects.len() > self.config.execution_limits.max_values_per_cell() {
                        return Err(ProjectionError::limit(format!(
                            "CSVW terms table `{}` column `{}` exceeds the {}-value cell limit",
                            declaration.name,
                            column.name,
                            self.config.execution_limits.max_values_per_cell()
                        )));
                    }
                    if column.required && objects.is_empty() {
                        return Err(ProjectionError::integrity(format!(
                            "CSVW terms table `{}` required column `{}` is empty for `{subject}`",
                            declaration.name, column.name
                        )));
                    }
                    if matches!(column.cardinality, CsvwTermsCardinality::One) && objects.len() > 1
                    {
                        return Err(ProjectionError::integrity(format!(
                            "CSVW terms table `{}` single-valued column `{}` has {} values for `{subject}`",
                            declaration.name,
                            column.name,
                            objects.len()
                        )));
                    }
                    value_count = value_count
                        .checked_add(objects.len())
                        .ok_or_else(|| ProjectionError::limit("CSVW terms value count overflow"))?;
                    if value_count > self.config.execution_limits.max_values() {
                        return Err(ProjectionError::limit(format!(
                            "CSVW terms output exceeds the {}-value limit",
                            self.config.execution_limits.max_values()
                        )));
                    }
                    cells.push(predicate_cell(column, objects)?);
                }
                let number = rows.len() + 1;
                rows.push(CsvwRow {
                    number,
                    source_number: number,
                    url: row_url(&declaration.table_url, number)?,
                    titles: vec![subject.to_owned()],
                    cells,
                });
            }
            tables.push(materialize_table(declaration, rows, &self.config.csvw));
        }
        Ok((
            CsvwTableGroup {
                id: self.config.csvw.table_group_iri().to_owned(),
                rdf_id: Some(self.config.csvw.table_group_iri().to_owned()),
                tables,
                annotations: CsvwAnnotations::new(),
            },
            row_count,
            value_count,
        ))
    }

    fn record_quad_losses(&mut self, memberships: &[Vec<&str>]) -> Result<(), ProjectionError> {
        for index in 0..self.quads.len() {
            let quad = &self.quads[index];
            if !self.graph_selected(quad.graph.as_ref()) {
                self.record_quad_loss(index, LOSS_CSVW_TERMS_GRAPH_UNSELECTED)?;
                continue;
            }
            let ProjectionTerm::Iri { value: subject } = &quad.subject else {
                self.record_quad_loss(index, LOSS_CSVW_TERMS_SUBJECT_UNREPRESENTABLE)?;
                continue;
            };
            let mut has_matching_table = false;
            let mut mapped_predicate = false;
            let mut represented = false;
            for (table_index, subjects) in memberships.iter().enumerate() {
                if subjects.binary_search(&subject.as_str()).is_err() {
                    continue;
                }
                has_matching_table = true;
                if let Some(column) = self.config.tables[table_index]
                    .columns
                    .iter()
                    .find(|column| column.predicate == quad.predicate)
                {
                    mapped_predicate = true;
                    if cell_value(&quad.object, &column.value_mode, &self.config.csvw).is_some() {
                        represented = true;
                        break;
                    }
                }
            }
            if !has_matching_table {
                self.record_quad_loss(index, LOSS_CSVW_TERMS_SUBJECT_UNSELECTED)?;
                continue;
            }
            if represented {
                if quad.graph.is_some() {
                    self.record_quad_loss(index, LOSS_CSVW_TERMS_NAMED_GRAPH_PLACEMENT_DROPPED)?;
                }
            } else if mapped_predicate {
                self.record_quad_loss(index, LOSS_CSVW_TERMS_OBJECT_UNREPRESENTABLE)?;
            } else {
                self.record_quad_loss(index, LOSS_CSVW_TERMS_PREDICATE_UNMAPPED)?;
            }
        }
        Ok(())
    }

    fn record_structural_losses(&mut self) -> Result<(), ProjectionError> {
        let non_empty_graphs = self
            .quads
            .iter()
            .filter_map(|quad| quad.graph.clone())
            .collect::<BTreeSet<_>>();
        for index in 0..self.named_graphs.len() {
            if !self.graph_selected(Some(&self.named_graphs[index])) {
                self.record_named_graph_loss(index, LOSS_CSVW_TERMS_GRAPH_UNSELECTED)?;
            } else if !non_empty_graphs.contains(&self.named_graphs[index]) {
                self.record_named_graph_loss(index, LOSS_CSVW_TERMS_EMPTY_GRAPH_DROPPED)?;
            }
        }
        for index in 0..self.reifiers.len() {
            if !self.graph_selected(self.reifiers[index].graph.as_ref()) {
                self.record_reifier_loss(index, LOSS_CSVW_TERMS_GRAPH_UNSELECTED)?;
            }
            self.record_reifier_loss(index, LOSS_CSVW_TERMS_REIFIER_DROPPED)?;
        }
        for index in 0..self.annotations.len() {
            if !self.graph_selected(self.annotations[index].graph.as_ref()) {
                self.record_annotation_loss(index, LOSS_CSVW_TERMS_GRAPH_UNSELECTED)?;
            }
            self.record_annotation_loss(index, LOSS_CSVW_TERMS_ANNOTATION_DROPPED)?;
        }
        Ok(())
    }

    fn graph_selected(&self, graph: Option<&ProjectionTerm>) -> bool {
        match self.config.graph_selection() {
            CsvwTermsGraphSelection::All => true,
            CsvwTermsGraphSelection::Include {
                default_graph,
                named_graphs,
            } => match graph {
                None => *default_graph,
                Some(ProjectionTerm::Iri { value }) => named_graphs.contains(value),
                Some(
                    ProjectionTerm::Blank { .. }
                    | ProjectionTerm::Literal { .. }
                    | ProjectionTerm::Triple { .. },
                ) => false,
            },
        }
    }

    fn record_quad_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("CsvwTermsQuad", &self.quads[index])?;
        self.record_loss(code, "csvw-terms:quad", subject);
        Ok(())
    }

    fn record_named_graph_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("CsvwTermsGraph", &self.named_graphs[index])?;
        self.record_loss(code, "csvw-terms:named-graph", subject);
        Ok(())
    }

    fn record_reifier_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("CsvwTermsReifier", &self.reifiers[index])?;
        self.record_loss(code, "csvw-terms:reifier", subject);
        Ok(())
    }

    fn record_annotation_loss(
        &mut self,
        index: usize,
        code: &'static str,
    ) -> Result<(), ProjectionError> {
        let subject = source_identifier("CsvwTermsAnnotation", &self.annotations[index])?;
        self.record_loss(code, "csvw-terms:annotation", subject);
        Ok(())
    }

    fn record_loss(&mut self, code: &'static str, logical: &str, subject: String) {
        let template = self
            .contract
            .entries()
            .iter()
            .find(|entry| entry.code == code)
            .expect("runtime CSVW terms code must exist in the closed contract");
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
}

fn selector_matches(
    selector: &CsvwTermsSelector,
    subject: &str,
    predicates: &BTreeMap<String, BTreeSet<ProjectionTerm>>,
) -> bool {
    if !selector.iri_prefixes.is_empty()
        && !selector
            .iri_prefixes
            .iter()
            .any(|prefix| subject.starts_with(prefix))
    {
        return false;
    }
    let Some(type_predicate) = selector.type_predicate.as_ref() else {
        return true;
    };
    let Some(types) = predicates.get(type_predicate) else {
        return selector.any_types.is_empty() && selector.all_types.is_empty();
    };
    let mut any_matched = selector.any_types.is_empty();
    let mut all_remaining = selector.all_types.len();
    for term in types {
        let ProjectionTerm::Iri { value } = term else {
            continue;
        };
        if selector.none_types.contains(value.as_str()) {
            return false;
        }
        if !any_matched && selector.any_types.contains(value.as_str()) {
            any_matched = true;
        }
        if selector.all_types.contains(value.as_str()) {
            all_remaining -= 1;
        }
    }
    any_matched && all_remaining == 0
}

fn identity_cell(subject: &str, identity: &CsvwTermsIdentityColumn) -> CsvwCell {
    CsvwCell {
        column: 0,
        string_value: subject.to_owned(),
        values: vec![CsvwValue {
            source: subject.to_owned(),
            lexical: subject.to_owned(),
            datatype: identity.datatype.base.clone(),
            language: None,
            direction: None,
        }],
        is_null: false,
    }
}

fn cell_value(
    object: &ProjectionTerm,
    mode: &CsvwTermsValueMode,
    config: &CsvwConfig,
) -> Option<CsvwValue> {
    match (object, mode) {
        (ProjectionTerm::Iri { value }, CsvwTermsValueMode::Iri { datatype }) => Some(CsvwValue {
            source: value.clone(),
            lexical: value.clone(),
            datatype: datatype.base.clone(),
            language: None,
            direction: None,
        }),
        (
            ProjectionTerm::Literal {
                lexical,
                datatype: actual_datatype,
                language: actual_language,
                direction: actual_direction,
            },
            CsvwTermsValueMode::Literal {
                datatype,
                language,
                direction,
            },
        ) if actual_datatype
            == &language.as_ref().map_or_else(
                || datatype.base.clone(),
                |_| config.vocabulary().rdf("langString"),
            )
            && actual_language == language
            && actual_direction == direction =>
        {
            Some(CsvwValue {
                source: lexical.clone(),
                lexical: lexical.clone(),
                datatype: datatype.base.clone(),
                language: actual_language.clone(),
                direction: actual_direction.map(direction_to_csvw),
            })
        }
        _ => None,
    }
}

fn predicate_cell(
    column: &CsvwTermsColumn,
    values: Vec<CsvwValue>,
) -> Result<CsvwCell, ProjectionError> {
    let string_value = match &column.cardinality {
        CsvwTermsCardinality::One => values
            .first()
            .map_or_else(String::new, |value| value.source.clone()),
        CsvwTermsCardinality::Many { separator } => {
            if values.iter().any(|value| value.source.contains(separator)) {
                return Err(ProjectionError::integrity(format!(
                    "CSVW terms column `{}` separator occurs inside a value",
                    column.name
                )));
            }
            values
                .iter()
                .map(|value| value.source.as_str())
                .collect::<Vec<_>>()
                .join(separator)
        }
    };
    Ok(CsvwCell {
        column: 0,
        string_value,
        is_null: values.is_empty(),
        values,
    })
}

fn materialize_table(
    declaration: &CsvwTermsTable,
    mut rows: Vec<CsvwRow>,
    config: &CsvwConfig,
) -> CsvwTable {
    let mut columns = Vec::with_capacity(declaration.columns.len() + 1);
    columns.push(CsvwColumn {
        id: None,
        number: 0,
        name: declaration.identity.name.clone(),
        name_explicit: true,
        titles: declaration.identity.titles.clone(),
        virtual_column: false,
        suppress_output: true,
        inherited: inherited(
            declaration.identity.datatype.clone(),
            None,
            None,
            None,
            None,
            None,
            false,
        ),
        annotations: CsvwAnnotations::new(),
    });
    for (offset, declaration_column) in declaration.columns.iter().enumerate() {
        let (language, direction, value_url) = match &declaration_column.value_mode {
            CsvwTermsValueMode::Iri { .. } => (
                None,
                None,
                Some(format!("{{+{}}}", declaration_column.name)),
            ),
            CsvwTermsValueMode::Literal {
                language,
                direction,
                ..
            } => (language.clone(), direction.map(direction_to_csvw), None),
        };
        let separator = match &declaration_column.cardinality {
            CsvwTermsCardinality::One => None,
            CsvwTermsCardinality::Many { separator } => Some(separator.clone()),
        };
        columns.push(CsvwColumn {
            id: None,
            number: offset + 1,
            name: declaration_column.name.clone(),
            name_explicit: true,
            titles: declaration_column.titles.clone(),
            virtual_column: false,
            suppress_output: false,
            inherited: inherited(
                declaration_column.value_mode.datatype().clone(),
                Some(format!("{{+{}}}", declaration.identity.name)),
                Some(declaration_column.predicate.clone()),
                value_url,
                language,
                direction,
                declaration_column.required,
            )
            .with_separator(separator),
            annotations: CsvwAnnotations::new(),
        });
    }
    for row in &mut rows {
        for (index, cell) in row.cells.iter_mut().enumerate() {
            cell.column = index;
        }
    }
    CsvwTable {
        id: None,
        url: declaration.table_url.clone(),
        dialect: canonical_dialect(),
        schema: CsvwSchema {
            id: None,
            columns,
            metadata_explicit: true,
            primary_key: vec![declaration.identity.name.clone()],
            foreign_keys: Vec::new(),
            row_titles: vec![declaration.identity.name.clone()],
            inherited: inherited(
                CsvwDatatype {
                    id: None,
                    base: config.vocabulary().xsd("string"),
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
                },
                None,
                None,
                None,
                None,
                None,
                false,
            ),
            annotations: CsvwAnnotations::new(),
        },
        suppress_output: false,
        table_direction: CsvwTableDirection::Auto,
        rows,
        comments: Vec::new(),
        transformations: Vec::new(),
        annotations: CsvwAnnotations::new(),
    }
}

trait InheritedSeparator {
    fn with_separator(self, separator: Option<String>) -> Self;
}

impl InheritedSeparator for CsvwInheritedProperties {
    fn with_separator(mut self, separator: Option<String>) -> Self {
        self.separator = separator;
        self
    }
}

fn inherited(
    datatype: CsvwDatatype,
    about_url: Option<String>,
    property_url: Option<String>,
    value_url: Option<String>,
    language: Option<String>,
    text_direction: Option<CsvwTextDirection>,
    required: bool,
) -> CsvwInheritedProperties {
    CsvwInheritedProperties {
        about_url,
        datatype,
        default: String::new(),
        language,
        nulls: vec![String::new()],
        ordered: false,
        property_url,
        required,
        separator: None,
        text_direction,
        value_url,
    }
}

fn canonical_dialect() -> CsvwDialect {
    CsvwDialect {
        encoding: "utf-8".to_owned(),
        line_terminators: vec!["\n".to_owned()],
        delimiter: ',',
        quote_char: Some('"'),
        double_quote: true,
        comment_prefix: Some("#".to_owned()),
        skip_rows: 0,
        skip_columns: 0,
        header_row_count: 0,
        skip_blank_rows: true,
        skip_initial_space: false,
        trim: CsvwTrim::None,
    }
}

fn row_url(table_url: &str, source_number: usize) -> Result<String, ProjectionError> {
    let base = purrdf_iri::parse(table_url)
        .map_err(|error| ProjectionError::term(format!("invalid CSVW terms table URL: {error}")))?;
    base.resolve(&format!("#row={source_number}"))
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| ProjectionError::term(format!("invalid CSVW terms row URL: {error}")))
}

fn direction_to_csvw(direction: ProjectionDirection) -> CsvwTextDirection {
    match direction {
        ProjectionDirection::Ltr => CsvwTextDirection::Ltr,
        ProjectionDirection::Rtl => CsvwTextDirection::Rtl,
    }
}

fn resolve_term<D: DatasetView>(
    view: &D,
    id: D::Id,
    config: &CsvwTermsConfig,
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
        ProjectionError::integrity(format!("serialize CSVW terms source location: {error}"))
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

    use purrdf_core::{
        BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermRef,
    };

    use crate::{
        parse_dataset,
        projections::{CsvwContext, CsvwMode, CsvwVocabulary, read_csvw},
    };

    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    const EX: &str = "https://example.org/";
    const VOCAB: &str = "https://example.org/vocab/";
    const TYPE: &str = "https://example.org/type";
    const CLASS: &str = "https://example.org/Class";
    const PROPERTY: &str = "https://example.org/Property";
    const PERSON: &str = "https://example.org/Person";
    const LABEL: &str = "https://example.org/label";
    const PARENT: &str = "https://example.org/parent";
    const NOTE: &str = "https://example.org/note";

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
                BTreeMap::from([(String::new(), vec!["IRI".to_owned()])]),
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

    fn identity() -> CsvwTermsIdentityColumn {
        CsvwTermsIdentityColumn::new(
            "iri",
            BTreeMap::from([(String::new(), vec!["IRI".to_owned()])]),
            datatype(&format!("{XSD}anyURI")),
        )
        .expect("identity")
    }

    fn label_column() -> CsvwTermsColumn {
        CsvwTermsColumn::new(
            "label",
            BTreeMap::new(),
            LABEL,
            CsvwTermsValueMode::literal(datatype(&format!("{XSD}string")), None, None)
                .expect("literal mode"),
            CsvwTermsCardinality::One,
            false,
        )
        .expect("label column")
    }

    fn typed_table(name: &str, kind: &str, path: &str) -> CsvwTermsTable {
        CsvwTermsTable::new(
            name,
            format!("{EX}catalog/{path}"),
            path,
            CsvwTermsSelector::new(
                Some(TYPE.to_owned()),
                BTreeSet::from([kind.to_owned()]),
                BTreeSet::new(),
                if kind == PERSON {
                    BTreeSet::from([CLASS.to_owned(), PROPERTY.to_owned()])
                } else {
                    BTreeSet::new()
                },
                BTreeSet::from([VOCAB.to_owned()]),
            )
            .expect("selector"),
            identity(),
            vec![label_column()],
        )
        .expect("typed table")
    }

    fn projection_config() -> CsvwTermsConfig {
        let mut classes = typed_table("classes", CLASS, "classes.csv");
        classes.columns.push(
            CsvwTermsColumn::new(
                "parents",
                BTreeMap::new(),
                PARENT,
                CsvwTermsValueMode::iri(datatype(&format!("{XSD}anyURI"))).expect("IRI mode"),
                CsvwTermsCardinality::many("\u{1f}").expect("many"),
                false,
            )
            .expect("parent column"),
        );
        classes.columns.push(
            CsvwTermsColumn::new(
                "note",
                BTreeMap::new(),
                NOTE,
                CsvwTermsValueMode::literal(
                    datatype(&format!("{XSD}string")),
                    Some("en".to_owned()),
                    Some(ProjectionDirection::Ltr),
                )
                .expect("directional mode"),
                CsvwTermsCardinality::One,
                false,
            )
            .expect("note column"),
        );
        CsvwTermsConfig::new(
            csvw(),
            "csvw-metadata.json",
            CsvwTermsGraphSelection::include(
                true,
                BTreeSet::from([format!("{EX}graph/business"), format!("{EX}graph/empty")]),
            )
            .expect("scope"),
            vec![
                classes,
                typed_table("properties", PROPERTY, "properties.csv"),
                typed_table("individuals", PERSON, "individuals.csv"),
            ],
            CsvwTermsLimits::new(100, 1_000, 20).expect("execution limits"),
        )
        .expect("terms config")
    }

    fn fixture() -> std::sync::Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let class = format!("{VOCAB}ClassA");
        let property = format!("{VOCAB}propertyA");
        let individual = format!("{VOCAB}individualA");
        let other = format!("{VOCAB}other");
        for (subject, kind) in [
            (&class, CLASS),
            (&property, PROPERTY),
            (&individual, PERSON),
        ] {
            push_iri_quad(&mut builder, subject, TYPE, kind, None);
        }
        for (subject, label) in [
            (&class, "Class A"),
            (&property, "Property A"),
            (&individual, "Individual A"),
        ] {
            push_literal_quad(
                &mut builder,
                subject,
                LABEL,
                RdfLiteral::simple(label),
                None,
            );
        }
        push_iri_quad(
            &mut builder,
            &class,
            PARENT,
            &format!("{VOCAB}ParentA"),
            None,
        );
        push_iri_quad(
            &mut builder,
            &class,
            PARENT,
            &format!("{VOCAB}ParentB"),
            None,
        );
        push_iri_quad(
            &mut builder,
            &class,
            LABEL,
            &format!("{VOCAB}not-a-literal"),
            None,
        );
        push_iri_quad(
            &mut builder,
            &other,
            LABEL,
            &format!("{VOCAB}ignored"),
            None,
        );

        let business = builder.intern_iri(&format!("{EX}graph/business"));
        builder.declare_named_graph(business);
        push_literal_quad(
            &mut builder,
            &class,
            NOTE,
            RdfLiteral {
                lexical_form: "Curated note".to_owned(),
                datatype: None,
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            },
            Some(business),
        );
        let empty = builder.intern_iri(&format!("{EX}graph/empty"));
        builder.declare_named_graph(empty);
        let excluded = builder.intern_iri(&format!("{EX}graph/excluded"));
        builder.declare_named_graph(excluded);
        push_iri_quad(
            &mut builder,
            &format!("{VOCAB}excluded"),
            TYPE,
            CLASS,
            Some(excluded),
        );

        let blank = builder.intern_blank("row", BlankScope(7));
        let label = builder.intern_iri(LABEL);
        let blank_value = builder.intern_literal(RdfLiteral::simple("Blank"));
        builder.push_quad(blank, label, blank_value, None);

        let quoted_subject = builder.intern_iri(&class);
        let quoted_predicate = builder.intern_iri(PARENT);
        let quoted_object = builder.intern_iri(&format!("{VOCAB}ParentA"));
        let statement = builder.intern_triple(quoted_subject, quoted_predicate, quoted_object);
        let reifier = builder.intern_blank("statement", BlankScope(9));
        builder.push_reifier_in_graph(reifier, statement, Some(business));
        let confidence = builder.intern_iri(&format!("{EX}confidence"));
        let confidence_value =
            builder.intern_literal(RdfLiteral::typed("0.9", format!("{XSD}decimal")));
        builder.push_annotation_in_graph(reifier, confidence, confidence_value, Some(business));
        builder.freeze().expect("fixture")
    }

    fn push_iri_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: Option<purrdf_core::TermId>,
    ) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_iri(object);
        builder.push_quad(subject, predicate, object, graph);
    }

    fn push_literal_quad(
        builder: &mut RdfDatasetBuilder,
        subject: &str,
        predicate: &str,
        literal: RdfLiteral,
        graph: Option<purrdf_core::TermId>,
    ) {
        let subject = builder.intern_iri(subject);
        let predicate = builder.intern_iri(predicate);
        let object = builder.intern_literal(literal);
        builder.push_quad(subject, predicate, object, graph);
    }

    fn has_iri_quad(dataset: &RdfDataset, subject: &str, predicate: &str, object: &str) -> bool {
        dataset.quads().any(|quad| {
            matches!(dataset.resolve(quad.s), TermRef::Iri(value) if value == subject)
                && matches!(dataset.resolve(quad.p), TermRef::Iri(value) if value == predicate)
                && matches!(dataset.resolve(quad.o), TermRef::Iri(value) if value == object)
        })
    }

    #[test]
    fn projects_scoped_entity_tables_deterministically_and_reads_semantics_back() {
        let dataset = fixture();
        let config = projection_config();
        let first = project_csvw_terms(dataset.as_ref(), &config).expect("first projection");
        let second = project_csvw_terms(dataset.as_ref(), &config).expect("second projection");
        assert_eq!(
            first.package.to_ustar().expect("archive"),
            second.package.to_ustar().expect("archive"),
        );
        assert_eq!(first.package.artifacts().len(), 4);
        assert_eq!(first.report.tables, 3);
        assert_eq!(first.report.rows, 3);
        assert_eq!(first.report.values, 6);
        let classes =
            std::str::from_utf8(first.package.get("classes.csv").expect("classes artifact"))
                .expect("UTF-8");
        assert!(classes.contains(&format!("{VOCAB}ClassA")));
        assert!(classes.contains("Class A"));
        assert!(classes.contains(&format!("{VOCAB}ParentA")));
        assert!(classes.contains(&format!("{VOCAB}ParentB")));
        assert!(!classes.contains(&format!("{VOCAB}excluded")));

        let read = read_csvw(&first.input, config.csvw()).expect("read curated CSVW");
        assert!(read.is_valid(), "warnings: {:?}", read.warnings);
        assert!(has_iri_quad(
            read.dataset.as_ref(),
            &format!("{VOCAB}ClassA"),
            PARENT,
            &format!("{VOCAB}ParentA"),
        ));
        assert!(has_iri_quad(
            read.dataset.as_ref(),
            &format!("{VOCAB}ClassA"),
            PARENT,
            &format!("{VOCAB}ParentB"),
        ));
        let note_column = &first.table_group.tables[0].schema.columns[3];
        assert_eq!(
            note_column.inherited.text_direction,
            Some(CsvwTextDirection::Ltr)
        );
        assert_eq!(
            first.table_group.tables[0].rows[0].cells[3].values[0].direction,
            Some(CsvwTextDirection::Ltr)
        );
        assert!(read.dataset.quads().any(|quad| {
            matches!(read.dataset.resolve(quad.s), TermRef::Iri(value) if value == format!("{VOCAB}ClassA"))
                && matches!(read.dataset.resolve(quad.p), TermRef::Iri(value) if value == NOTE)
                && matches!(
                    read.dataset.resolve(quad.o),
                    TermRef::Literal {
                        lexical: "Curated note",
                        language: Some("en"),
                        direction: None,
                        ..
                    }
                )
        }));

        let codes = first
            .loss_ledger
            .entries()
            .iter()
            .map(|entry| entry.code.as_ref())
            .collect::<BTreeSet<_>>();
        for code in [
            LOSS_CSVW_TERMS_ANNOTATION_DROPPED,
            LOSS_CSVW_TERMS_EMPTY_GRAPH_DROPPED,
            LOSS_CSVW_TERMS_GRAPH_UNSELECTED,
            LOSS_CSVW_TERMS_NAMED_GRAPH_PLACEMENT_DROPPED,
            LOSS_CSVW_TERMS_OBJECT_UNREPRESENTABLE,
            LOSS_CSVW_TERMS_PREDICATE_UNMAPPED,
            LOSS_CSVW_TERMS_REIFIER_DROPPED,
            LOSS_CSVW_TERMS_SUBJECT_UNREPRESENTABLE,
            LOSS_CSVW_TERMS_SUBJECT_UNSELECTED,
        ] {
            assert!(codes.contains(code), "missing loss code {code}");
        }
        assert!(
            first
                .loss_ledger
                .entries()
                .iter()
                .all(|entry| entry.location.is_some())
        );
    }

    #[test]
    fn bytes_ignore_source_statement_and_term_interning_order() {
        let first = parse_dataset(
            format!(
                "<{VOCAB}ClassA> <{TYPE}> <{CLASS}> .\n<{VOCAB}ClassA> <{LABEL}> \"Class A\" .\n"
            )
            .as_bytes(),
            "application/n-quads",
            None,
        )
        .expect("first dataset");
        let second = parse_dataset(
            format!(
                "<{VOCAB}ClassA> <{LABEL}> \"Class A\" .\n<{VOCAB}ClassA> <{TYPE}> <{CLASS}> .\n"
            )
            .as_bytes(),
            "application/n-quads",
            None,
        )
        .expect("second dataset");
        let config = projection_config();
        let first = project_csvw_terms(first.as_ref(), &config).expect("first projection");
        let second = project_csvw_terms(second.as_ref(), &config).expect("second projection");
        assert_eq!(
            first.package.to_ustar().expect("first archive"),
            second.package.to_ustar().expect("second archive"),
        );
        assert_eq!(
            first.loss_ledger.render_json(),
            second.loss_ledger.render_json(),
        );
    }

    #[test]
    fn hard_fails_cardinality_separator_and_execution_limits() {
        let dataset = fixture();
        let mut config = projection_config();
        config.tables[0].columns[1].cardinality = CsvwTermsCardinality::One;
        assert!(project_csvw_terms(dataset.as_ref(), &config).is_err());

        let mut config = projection_config();
        config.tables[0].columns[1].cardinality =
            CsvwTermsCardinality::many("/").expect("separator");
        assert!(project_csvw_terms(dataset.as_ref(), &config).is_err());

        let mut config = projection_config();
        config.execution_limits = CsvwTermsLimits::new(2, 1_000, 20).expect("limits");
        assert!(project_csvw_terms(dataset.as_ref(), &config).is_err());

        let mut config = projection_config();
        config.execution_limits = CsvwTermsLimits::new(100, 1_000, 1).expect("limits");
        assert!(project_csvw_terms(dataset.as_ref(), &config).is_err());
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

    #[test]
    fn selector_type_sets_preserve_any_all_and_none_semantics() {
        let subject = format!("{VOCAB}resource");
        let selector = CsvwTermsSelector::new(
            Some(TYPE.to_owned()),
            BTreeSet::from([CLASS.to_owned()]),
            BTreeSet::from([CLASS.to_owned(), PROPERTY.to_owned()]),
            BTreeSet::from([PERSON.to_owned()]),
            BTreeSet::from([VOCAB.to_owned()]),
        )
        .expect("selector");
        let mut types = BTreeSet::from([
            ProjectionTerm::Iri {
                value: CLASS.to_owned(),
            },
            ProjectionTerm::Iri {
                value: PROPERTY.to_owned(),
            },
        ]);
        let predicates = BTreeMap::from([(TYPE.to_owned(), types.clone())]);
        assert!(selector_matches(&selector, &subject, &predicates));

        types.insert(ProjectionTerm::Iri {
            value: PERSON.to_owned(),
        });
        let predicates = BTreeMap::from([(TYPE.to_owned(), types)]);
        assert!(!selector_matches(&selector, &subject, &predicates));

        let none_only = CsvwTermsSelector::new(
            Some(TYPE.to_owned()),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::from([PERSON.to_owned()]),
            BTreeSet::new(),
        )
        .expect("none-only selector");
        assert!(selector_matches(&none_only, &subject, &BTreeMap::new()));
        assert!(!selector_matches(&selector, &subject, &BTreeMap::new()));
    }
}

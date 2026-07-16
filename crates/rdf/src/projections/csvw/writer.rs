// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic CSVW table-group writing and caller-owned RDF mappings.

use std::collections::{BTreeMap, BTreeSet};

use csv::{QuoteStyle, Terminator, WriterBuilder};
use purrdf_core::{DatasetView, LossLedger};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::super::util::canonical_json_bounded;
use super::super::{ProjectionError, ProjectionPackage, validate_absolute_iri};
use super::config::CsvwConfig;
use super::input::{CsvwAction, CsvwInput};
use super::model::{
    CsvwAnnotations, CsvwDatatype, CsvwDatatypeFormat, CsvwForeignKey, CsvwInheritedProperties,
    CsvwNaturalLanguage, CsvwReference, CsvwSchema, CsvwTable, CsvwTableDirection, CsvwTableGroup,
    CsvwTextDirection, CsvwTransformation,
};

/// Mandatory mapping from resource identities to safe package paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CsvwWritePlan {
    metadata_path: String,
    table_paths: BTreeMap<String, String>,
}

impl CsvwWritePlan {
    /// Construct and validate a complete table-path mapping.
    ///
    /// # Errors
    ///
    /// Returns a configuration error for an unsafe path, malformed table IRI,
    /// duplicate artifact path, or an empty mapping.
    pub fn new(
        metadata_path: impl Into<String>,
        table_paths: BTreeMap<String, String>,
    ) -> Result<Self, ProjectionError> {
        let metadata_path = metadata_path.into();
        validate_package_path(&metadata_path)?;
        if table_paths.is_empty() {
            return Err(ProjectionError::configuration(
                "CSVW write plan requires at least one table path",
            ));
        }
        let mut paths = BTreeSet::from([metadata_path.clone()]);
        for (table, path) in &table_paths {
            validate_absolute_iri(table, "CSVW write-plan table")?;
            validate_package_path(path)?;
            if !paths.insert(path.clone()) {
                return Err(ProjectionError::configuration(format!(
                    "duplicate CSVW artifact path `{path}`"
                )));
            }
        }
        Ok(Self {
            metadata_path,
            table_paths,
        })
    }

    /// Metadata artifact path.
    pub fn metadata_path(&self) -> &str {
        &self.metadata_path
    }

    /// Deterministically ordered table-IRI to artifact-path mappings.
    pub fn table_paths(&self) -> &BTreeMap<String, String> {
        &self.table_paths
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCsvwWritePlan {
    metadata_path: String,
    table_paths: BTreeMap<String, String>,
}

impl<'de> Deserialize<'de> for CsvwWritePlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCsvwWritePlan::deserialize(deserializer)?;
        Self::new(raw.metadata_path, raw.table_paths).map_err(serde::de::Error::custom)
    }
}

/// Result of deterministically writing one normative CSVW table group.
#[derive(Debug, Clone)]
pub struct CsvwWriteOutcome {
    /// Filesystem-free artifact package using caller-selected safe paths.
    pub package: ProjectionPackage,
    /// Equivalent IRI-keyed input that can be passed directly to [`super::read_csvw`].
    pub input: CsvwInput,
    /// Always-computed mapping ledger.
    pub loss_ledger: LossLedger,
}

/// Typed result that a caller-owned RDF-to-table mapping must produce.
#[derive(Debug, Clone)]
pub struct CsvwMappedTableGroup {
    /// Fully materialized annotated-table model.
    pub group: CsvwTableGroup,
    /// Complete ledger for semantics the mapping did not carry into tables.
    pub loss_ledger: LossLedger,
}

/// Caller-owned semantics for mapping an RDF backend into a CSVW table group.
///
/// CSVW specifies table-to-RDF conversion, not a unique arbitrary RDF-to-table
/// inverse. PurRDF therefore owns the deterministic carrier while callers own
/// predicate/row selection and must return the corresponding loss ledger.
pub trait CsvwRdfTableMapping<D: DatasetView> {
    /// Map `view` into a validated table group and a complete loss ledger.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the caller mapping is ambiguous or cannot be
    /// represented under its declared contract.
    fn map(&self, view: &D, config: &CsvwConfig) -> Result<CsvwMappedTableGroup, ProjectionError>;
}

/// Apply a caller-owned RDF mapping and write its deterministic CSVW package.
///
/// # Errors
///
/// Returns any mapping, model-integrity, serialization, path, or resource-limit
/// failure without producing a partial package.
pub fn project_csvw<D, M>(
    view: &D,
    mapping: &M,
    plan: &CsvwWritePlan,
    config: &CsvwConfig,
) -> Result<CsvwWriteOutcome, ProjectionError>
where
    D: DatasetView,
    M: CsvwRdfTableMapping<D>,
{
    let mapped = mapping.map(view, config)?;
    write_csvw_with_ledger(&mapped.group, plan, config, mapped.loss_ledger)
}

/// Write a normalized table group to canonical metadata and CSV resources.
///
/// The writer uses a fixed UTF-8, comma-delimited, always-quoted physical form.
/// Metadata carries every effective column property explicitly, so a successful
/// read followed by repeated writes reaches byte-identical canonical output.
///
/// # Errors
///
/// Returns a model-integrity, incomplete path mapping, CSV/JSON serialization,
/// unsafe comment, or configured resource-limit failure.
pub fn write_csvw(
    group: &CsvwTableGroup,
    plan: &CsvwWritePlan,
    config: &CsvwConfig,
) -> Result<CsvwWriteOutcome, ProjectionError> {
    write_csvw_with_ledger(group, plan, config, LossLedger::new())
}

fn write_csvw_with_ledger(
    group: &CsvwTableGroup,
    plan: &CsvwWritePlan,
    config: &CsvwConfig,
    loss_ledger: LossLedger,
) -> Result<CsvwWriteOutcome, ProjectionError> {
    group.validate()?;
    let group_urls = group
        .tables
        .iter()
        .map(|table| table.url.as_str())
        .collect::<BTreeSet<_>>();
    let plan_urls = plan
        .table_paths
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if group_urls != plan_urls {
        return Err(ProjectionError::configuration(
            "CSVW write plan must map every table URL exactly once",
        ));
    }

    let metadata = metadata_bytes(group, config)?;
    let mut artifacts = BTreeMap::from([(plan.metadata_path.clone(), metadata.clone())]);
    let mut resources = BTreeMap::from([(config.metadata_base_iri().to_owned(), metadata)]);
    for table in &group.tables {
        let bytes = table_bytes(table)?;
        let path = plan.table_paths.get(&table.url).ok_or_else(|| {
            ProjectionError::configuration(format!(
                "CSVW write plan has no path for `{}`",
                table.url
            ))
        })?;
        artifacts.insert(path.clone(), bytes.clone());
        resources.insert(table.url.clone(), bytes);
    }
    let package = ProjectionPackage::from_artifacts(config.limits(), artifacts)?;
    let input = CsvwInput::new(
        CsvwAction::Metadata {
            metadata_iri: config.metadata_base_iri().to_owned(),
        },
        resources,
        config.limits(),
    )?;
    Ok(CsvwWriteOutcome {
        package,
        input,
        loss_ledger,
    })
}

fn metadata_bytes(group: &CsvwTableGroup, config: &CsvwConfig) -> Result<Vec<u8>, ProjectionError> {
    let mut root = annotation_map(&group.annotations);
    root.insert(
        "@context".to_owned(),
        Value::String(config.context_iri().to_owned()),
    );
    root.insert("@type".to_owned(), Value::String("TableGroup".to_owned()));
    if let Some(id) = &group.rdf_id {
        root.insert("@id".to_owned(), Value::String(id.clone()));
    }
    root.insert(
        "tables".to_owned(),
        Value::Array(
            group
                .tables
                .iter()
                .map(|table| table_metadata(table, config))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    canonical_json_bounded(&Value::Object(root), config.limits(), "CSVW metadata")
}

fn table_metadata(table: &CsvwTable, config: &CsvwConfig) -> Result<Value, ProjectionError> {
    let mut object = annotation_map(&table.annotations);
    object.insert("@type".to_owned(), Value::String("Table".to_owned()));
    if let Some(id) = &table.id {
        object.insert("@id".to_owned(), Value::String(id.clone()));
    }
    object.insert("url".to_owned(), Value::String(table.url.clone()));
    object.insert("dialect".to_owned(), canonical_dialect());
    object.insert(
        "tableSchema".to_owned(),
        schema_metadata(&table.schema, config)?,
    );
    object.insert(
        "tableDirection".to_owned(),
        Value::String(table_direction(table.table_direction).to_owned()),
    );
    if table.suppress_output {
        object.insert("suppressOutput".to_owned(), Value::Bool(true));
    }
    if !table.transformations.is_empty() {
        object.insert(
            "transformations".to_owned(),
            Value::Array(
                table
                    .transformations
                    .iter()
                    .map(transformation_metadata)
                    .collect(),
            ),
        );
    }
    Ok(Value::Object(object))
}

fn canonical_dialect() -> Value {
    serde_json::json!({
        "@type": "Dialect",
        "commentPrefix": "#",
        "delimiter": ",",
        "doubleQuote": true,
        "encoding": "utf-8",
        "headerRowCount": 0,
        "lineTerminators": ["\n"],
        "quoteChar": "\"",
        "skipBlankRows": true,
        "skipColumns": 0,
        "skipInitialSpace": false,
        "skipRows": 0,
        "trim": false
    })
}

fn schema_metadata(schema: &CsvwSchema, config: &CsvwConfig) -> Result<Value, ProjectionError> {
    let mut object = annotation_map(&schema.annotations);
    object.insert("@type".to_owned(), Value::String("Schema".to_owned()));
    if let Some(id) = &schema.id {
        object.insert("@id".to_owned(), Value::String(id.clone()));
    }
    insert_inherited(&mut object, &schema.inherited, config)?;
    object.insert(
        "columns".to_owned(),
        Value::Array(
            schema
                .columns
                .iter()
                .map(|column| {
                    let mut column_object = annotation_map(&column.annotations);
                    column_object.insert("@type".to_owned(), Value::String("Column".to_owned()));
                    if let Some(id) = &column.id {
                        column_object.insert("@id".to_owned(), Value::String(id.clone()));
                    }
                    column_object.insert("name".to_owned(), Value::String(column.name.clone()));
                    if !column.titles.is_empty() {
                        column_object.insert("titles".to_owned(), natural_language(&column.titles));
                    }
                    if column.virtual_column {
                        column_object.insert("virtual".to_owned(), Value::Bool(true));
                    }
                    if column.suppress_output {
                        column_object.insert("suppressOutput".to_owned(), Value::Bool(true));
                    }
                    insert_inherited(&mut column_object, &column.inherited, config)?;
                    Ok(Value::Object(column_object))
                })
                .collect::<Result<Vec<_>, ProjectionError>>()?,
        ),
    );
    insert_string_or_array(&mut object, "primaryKey", &schema.primary_key);
    insert_string_or_array(&mut object, "rowTitles", &schema.row_titles);
    if !schema.foreign_keys.is_empty() {
        object.insert(
            "foreignKeys".to_owned(),
            Value::Array(
                schema
                    .foreign_keys
                    .iter()
                    .map(foreign_key_metadata)
                    .collect(),
            ),
        );
    }
    Ok(Value::Object(object))
}

fn insert_inherited(
    object: &mut Map<String, Value>,
    inherited: &CsvwInheritedProperties,
    config: &CsvwConfig,
) -> Result<(), ProjectionError> {
    insert_optional_string(object, "aboutUrl", inherited.about_url.as_deref());
    object.insert(
        "datatype".to_owned(),
        datatype_metadata(&inherited.datatype, config)?,
    );
    object.insert(
        "default".to_owned(),
        Value::String(inherited.default.clone()),
    );
    insert_optional_string(object, "lang", inherited.language.as_deref());
    object.insert("null".to_owned(), strings_value(&inherited.nulls));
    object.insert("ordered".to_owned(), Value::Bool(inherited.ordered));
    insert_optional_string(object, "propertyUrl", inherited.property_url.as_deref());
    object.insert("required".to_owned(), Value::Bool(inherited.required));
    if let Some(separator) = &inherited.separator {
        object.insert("separator".to_owned(), Value::String(separator.clone()));
    }
    if let Some(direction) = inherited.text_direction {
        object.insert(
            "textDirection".to_owned(),
            Value::String(text_direction(direction).to_owned()),
        );
    }
    insert_optional_string(object, "valueUrl", inherited.value_url.as_deref());
    Ok(())
}

fn datatype_metadata(
    datatype: &CsvwDatatype,
    config: &CsvwConfig,
) -> Result<Value, ProjectionError> {
    let mut object = Map::new();
    object.insert("@type".to_owned(), Value::String("Datatype".to_owned()));
    if let Some(id) = &datatype.id {
        object.insert("@id".to_owned(), Value::String(id.clone()));
    }
    object.insert(
        "base".to_owned(),
        Value::String(datatype_base(&datatype.base, config)?),
    );
    if let Some(format) = &datatype.format {
        object.insert(
            "format".to_owned(),
            match format {
                CsvwDatatypeFormat::Pattern(pattern) => Value::String(pattern.clone()),
                CsvwDatatypeFormat::Numeric(numeric) => {
                    let mut value = Map::new();
                    if let Some(pattern) = &numeric.pattern {
                        value.insert("pattern".to_owned(), Value::String(pattern.clone()));
                    }
                    value.insert(
                        "decimalChar".to_owned(),
                        Value::String(numeric.decimal_char.to_string()),
                    );
                    if let Some(group) = numeric.group_char {
                        value.insert("groupChar".to_owned(), Value::String(group.to_string()));
                    }
                    Value::Object(value)
                }
            },
        );
    }
    insert_usize(&mut object, "length", datatype.length);
    insert_usize(&mut object, "minLength", datatype.min_length);
    insert_usize(&mut object, "maxLength", datatype.max_length);
    insert_value(&mut object, "minimum", datatype.minimum.as_ref());
    insert_value(&mut object, "maximum", datatype.maximum.as_ref());
    insert_value(&mut object, "minInclusive", datatype.min_inclusive.as_ref());
    insert_value(&mut object, "maxInclusive", datatype.max_inclusive.as_ref());
    insert_value(&mut object, "minExclusive", datatype.min_exclusive.as_ref());
    insert_value(&mut object, "maxExclusive", datatype.max_exclusive.as_ref());
    Ok(Value::Object(object))
}

fn datatype_base(base: &str, config: &CsvwConfig) -> Result<String, ProjectionError> {
    if let Some(local) = base.strip_prefix(config.vocabulary().xsd_namespace()) {
        return Ok(local.to_owned());
    }
    if base == config.vocabulary().rdf("HTML") {
        return Ok("html".to_owned());
    }
    if base == config.vocabulary().rdf("JSON") {
        return Ok("json".to_owned());
    }
    if base == config.vocabulary().rdf("XMLLiteral") {
        return Ok("xml".to_owned());
    }
    Err(ProjectionError::configuration(format!(
        "CSVW datatype base `{base}` is not a configured built-in datatype"
    )))
}

fn transformation_metadata(transformation: &CsvwTransformation) -> Value {
    let mut object = Map::new();
    object.insert("@type".to_owned(), Value::String("Template".to_owned()));
    if let Some(id) = &transformation.id {
        object.insert("@id".to_owned(), Value::String(id.clone()));
    }
    object.insert("url".to_owned(), Value::String(transformation.url.clone()));
    object.insert(
        "titles".to_owned(),
        natural_language(&transformation.titles),
    );
    object.insert(
        "targetFormat".to_owned(),
        Value::String(transformation.target_format.clone()),
    );
    object.insert(
        "scriptFormat".to_owned(),
        Value::String(transformation.script_format.clone()),
    );
    if let Some(source) = &transformation.source {
        object.insert("source".to_owned(), Value::String(source.clone()));
    }
    Value::Object(object)
}

fn foreign_key_metadata(foreign_key: &CsvwForeignKey) -> Value {
    let mut object = Map::new();
    object.insert(
        "columnReference".to_owned(),
        strings_value(&foreign_key.column_reference),
    );
    object.insert(
        "reference".to_owned(),
        reference_metadata(&foreign_key.reference),
    );
    Value::Object(object)
}

fn reference_metadata(reference: &CsvwReference) -> Value {
    let mut object = Map::new();
    if let Some(resource) = &reference.resource {
        object.insert("resource".to_owned(), Value::String(resource.clone()));
    }
    if let Some(schema) = &reference.schema_reference {
        object.insert("schemaReference".to_owned(), Value::String(schema.clone()));
    }
    object.insert(
        "columnReference".to_owned(),
        strings_value(&reference.column_reference),
    );
    Value::Object(object)
}

fn table_bytes(table: &CsvwTable) -> Result<Vec<u8>, ProjectionError> {
    let mut rows = BTreeMap::new();
    for row in &table.rows {
        if row.source_number == 0 || rows.insert(row.source_number, row).is_some() {
            return Err(ProjectionError::integrity(
                "CSVW source row numbers must be unique and one-based",
            )
            .at_path(&table.url));
        }
        let expected = purrdf_iri::parse(&table.url)
            .map_err(|error| ProjectionError::term(format!("invalid CSVW table URL: {error}")))?
            .resolve(&format!("#row={}", row.source_number))
            .map_err(|error| ProjectionError::term(format!("invalid CSVW row URL: {error}")))?;
        if expected.as_str() != row.url {
            return Err(ProjectionError::integrity(format!(
                "CSVW row URL `{}` does not match source row {}",
                row.url, row.source_number
            ))
            .at_path(&table.url));
        }
    }
    let mut comments = table.comments.iter();
    let mut output = Vec::new();
    let maximum = rows.keys().next_back().copied().unwrap_or(0);
    for source_number in 1..=maximum {
        if let Some(row) = rows.get(&source_number) {
            let fields = table
                .schema
                .columns
                .iter()
                .zip(&row.cells)
                .filter(|(column, _)| !column.virtual_column)
                .map(|(_, cell)| cell.string_value.as_str())
                .collect::<Vec<_>>();
            append_csv_record(&mut output, &fields)?;
        } else if let Some(comment) = comments.next() {
            append_comment(&mut output, comment)?;
        } else {
            output.push(b'\n');
        }
    }
    for comment in comments {
        append_comment(&mut output, comment)?;
    }
    Ok(output)
}

fn append_csv_record(output: &mut Vec<u8>, fields: &[&str]) -> Result<(), ProjectionError> {
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::Always)
        .terminator(Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer
        .write_record(fields)
        .map_err(|error| ProjectionError::syntax(format!("write CSVW row: {error}")))?;
    let bytes = writer
        .into_inner()
        .map_err(|error| ProjectionError::syntax(format!("finish CSVW row: {error}")))?;
    output.extend_from_slice(&bytes);
    Ok(())
}

fn append_comment(output: &mut Vec<u8>, comment: &str) -> Result<(), ProjectionError> {
    if comment.contains(['\r', '\n']) {
        return Err(ProjectionError::integrity(
            "CSVW comments cannot contain a line terminator",
        ));
    }
    output.push(b'#');
    output.extend_from_slice(comment.as_bytes());
    output.push(b'\n');
    Ok(())
}

fn annotation_map(annotations: &CsvwAnnotations) -> Map<String, Value> {
    annotations
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn natural_language(values: &CsvwNaturalLanguage) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(language, strings)| (language.clone(), strings_value(strings)))
            .collect(),
    )
}

fn strings_value(values: &[String]) -> Value {
    if let [value] = values {
        Value::String(value.clone())
    } else {
        Value::Array(values.iter().cloned().map(Value::String).collect())
    }
}

fn insert_string_or_array(object: &mut Map<String, Value>, key: &str, values: &[String]) {
    if !values.is_empty() {
        object.insert(key.to_owned(), strings_value(values));
    }
}

fn insert_optional_string(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), Value::String(value.to_owned()));
    }
}

fn insert_usize(object: &mut Map<String, Value>, key: &str, value: Option<usize>) {
    if let Some(value) = value.and_then(|value| u64::try_from(value).ok()) {
        object.insert(key.to_owned(), Value::Number(value.into()));
    }
}

fn insert_value(object: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), value.clone());
    }
}

const fn table_direction(direction: CsvwTableDirection) -> &'static str {
    match direction {
        CsvwTableDirection::Auto => "auto",
        CsvwTableDirection::Ltr => "ltr",
        CsvwTableDirection::Rtl => "rtl",
    }
}

const fn text_direction(direction: CsvwTextDirection) -> &'static str {
    match direction {
        CsvwTextDirection::Ltr => "ltr",
        CsvwTextDirection::Rtl => "rtl",
        CsvwTextDirection::Auto => "auto",
        CsvwTextDirection::Inherit => "inherit",
    }
}

fn validate_package_path(path: &str) -> Result<(), ProjectionError> {
    if path.is_empty()
        || path.len() > 4_096
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains(['\\', '\0'])
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(ProjectionError::configuration(format!(
            "unsafe CSVW artifact path `{path}`"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, datasets_isomorphic};

    use super::*;
    use crate::{CsvwContext, CsvwMode, CsvwVocabulary, ProjectionLimits, parse_dataset};

    fn config() -> CsvwConfig {
        CsvwConfig::new(
            "http://example.org/csvw-metadata.json",
            CsvwContext::new("http://www.w3.org/ns/csvw", BTreeMap::new()).expect("context"),
            "http://example.org/group",
            CsvwVocabulary::new(
                "http://www.w3.org/ns/csvw#",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
                "http://www.w3.org/2000/01/rdf-schema#",
                "http://www.w3.org/2001/XMLSchema#",
            )
            .expect("vocabulary"),
            CsvwMode::Standard,
            ProjectionLimits::new(16, 1_000_000, 2_000_000, 3_000_000, 16).expect("limits"),
            10_000,
        )
        .expect("config")
    }

    fn source_outcome(config: &CsvwConfig) -> super::super::CsvwReadOutcome {
        let metadata = br##"{
            "@context":"http://www.w3.org/ns/csvw",
            "url":"http://example.org/data.csv",
            "dialect":{"commentPrefix":"#"},
            "tableSchema":{"columns":[
                {"name":"name","titles":"Name","required":true},
                {"name":"tags","titles":"Tags","separator":";"},
                {"name":"kind","virtual":true,"propertyUrl":"http://example.org/kind","valueUrl":"http://example.org/Thing"}
            ]}
        }"##;
        let input = CsvwInput::new(
            CsvwAction::Metadata {
                metadata_iri: config.metadata_base_iri().to_owned(),
            },
            BTreeMap::from([
                (config.metadata_base_iri().to_owned(), metadata.to_vec()),
                (
                    "http://example.org/data.csv".to_owned(),
                    b"#source comment\nName,Tags\nAlice,a; b\n".to_vec(),
                ),
            ]),
            config.limits(),
        )
        .expect("input");
        super::super::read_csvw(&input, config).expect("read")
    }

    fn plan() -> CsvwWritePlan {
        CsvwWritePlan::new(
            "metadata.json",
            BTreeMap::from([(
                "http://example.org/data.csv".to_owned(),
                "tables/data.csv".to_owned(),
            )]),
        )
        .expect("plan")
    }

    #[test]
    fn canonical_write_read_write_is_byte_identical() {
        let config = config();
        let original = source_outcome(&config);
        let first = write_csvw(&original.group, &plan(), &config).expect("first write");
        let decoded = super::super::read_csvw(&first.input, &config).expect("canonical read");
        let second = write_csvw(&decoded.group, &plan(), &config).expect("second write");

        assert_eq!(first.package, second.package);
        assert_eq!(
            first.package.to_ustar().expect("archive"),
            second.package.to_ustar().expect("archive")
        );
        assert!(datasets_isomorphic(&original.dataset, &decoded.dataset));
        assert!(first.loss_ledger.is_empty());
        assert_eq!(
            first.package.get("tables/data.csv"),
            Some(&b"#source comment\n\n\"Alice\",\"a; b\"\n"[..])
        );
    }

    #[derive(Clone)]
    struct Mapping {
        group: CsvwTableGroup,
    }

    impl CsvwRdfTableMapping<RdfDataset> for Mapping {
        fn map(
            &self,
            view: &RdfDataset,
            _config: &CsvwConfig,
        ) -> Result<CsvwMappedTableGroup, ProjectionError> {
            if view.quads().next().is_none() {
                return Err(ProjectionError::integrity(
                    "fixture mapping requires a non-empty dataset",
                ));
            }
            Ok(CsvwMappedTableGroup {
                group: self.group.clone(),
                loss_ledger: LossLedger::new(),
            })
        }
    }

    #[test]
    fn caller_mapping_uses_the_generic_dataset_view_route() {
        let config = config();
        let source = source_outcome(&config);
        let dataset: Arc<RdfDataset> = parse_dataset(
            b"<http://example.org/s> <http://example.org/p> <http://example.org/o> .",
            "application/n-triples",
            None,
        )
        .expect("dataset");
        let projected = project_csvw(
            dataset.as_ref(),
            &Mapping {
                group: source.group,
            },
            &plan(),
            &config,
        )
        .expect("mapped projection");
        assert!(projected.loss_ledger.is_empty());
        assert!(projected.package.get("metadata.json").is_some());
    }

    #[test]
    fn write_plan_and_model_mismatch_fail_closed() {
        assert!(CsvwWritePlan::new("../metadata.json", BTreeMap::new()).is_err());
        let config = config();
        let source = source_outcome(&config);
        let wrong = CsvwWritePlan::new(
            "metadata.json",
            BTreeMap::from([(
                "http://example.org/other.csv".to_owned(),
                "other.csv".to_owned(),
            )]),
        )
        .expect("syntactically valid plan");
        assert!(write_csvw(&source.group, &wrong, &config).is_err());
    }
}

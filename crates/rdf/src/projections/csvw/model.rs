// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Typed CSVW annotated-table model.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{ProjectionError, validate_absolute_iri};

/// Natural-language values keyed by a BCP47 language tag.
///
/// The empty key denotes an untagged value. Value order is significant and is
/// retained; language-key order is deterministic.
pub type CsvwNaturalLanguage = BTreeMap<String, Vec<String>>;

/// Common JSON-LD annotations after inherited-property processing.
///
/// Keys are expanded absolute IRIs. Values retain their normalized JSON form so
/// the RDF conversion can preserve language maps, value objects, lists, and node
/// objects without reducing them to strings.
pub type CsvwAnnotations = BTreeMap<String, Value>;

/// CSVW text direction for a column value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvwTextDirection {
    /// Direction is inferred from the value.
    Auto,
    /// Left-to-right text.
    Ltr,
    /// Right-to-left text.
    Rtl,
    /// Inherit direction from the enclosing description.
    Inherit,
}

/// Direction in which a table is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvwTableDirection {
    /// Direction is inferred by the host.
    Auto,
    /// Left-to-right columns.
    Ltr,
    /// Right-to-left columns.
    Rtl,
}

/// Whitespace trimming policy from a CSVW dialect description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CsvwTrim {
    /// Retain leading and trailing whitespace.
    None,
    /// Trim leading whitespace.
    Start,
    /// Trim trailing whitespace.
    End,
    /// Trim both ends.
    Both,
}

/// A normalized CSVW dialect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwDialect {
    /// Character encoding label. The engine currently accepts UTF-8 labels only.
    pub encoding: String,
    /// Accepted line terminators in declared priority order.
    pub line_terminators: Vec<String>,
    /// Single field delimiter.
    pub delimiter: char,
    /// Quote character, or `None` when quoting is disabled.
    pub quote_char: Option<char>,
    /// Whether a doubled quote escapes a quote inside a quoted field.
    pub double_quote: bool,
    /// Comment prefix, or `None` when comment rows are disabled.
    pub comment_prefix: Option<String>,
    /// Physical rows skipped before header processing.
    pub skip_rows: usize,
    /// Leading physical columns skipped from every record.
    pub skip_columns: usize,
    /// Number of header records.
    pub header_row_count: usize,
    /// Whether blank records are omitted.
    pub skip_blank_rows: bool,
    /// Whether whitespace following a delimiter is ignored.
    pub skip_initial_space: bool,
    /// Cell whitespace policy.
    pub trim: CsvwTrim,
}

impl Default for CsvwDialect {
    fn default() -> Self {
        Self {
            encoding: "utf-8".to_owned(),
            line_terminators: vec!["\r\n".to_owned(), "\n".to_owned()],
            delimiter: ',',
            quote_char: Some('"'),
            double_quote: true,
            comment_prefix: None,
            skip_rows: 0,
            skip_columns: 0,
            header_row_count: 1,
            skip_blank_rows: false,
            skip_initial_space: false,
            trim: CsvwTrim::None,
        }
    }
}

/// A CSVW numeric-format object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwNumericFormat {
    /// UTS 35 number pattern.
    pub pattern: Option<String>,
    /// Decimal separator.
    pub decimal_char: char,
    /// Optional digit-grouping separator.
    pub group_char: Option<char>,
}

/// String or numeric-object datatype format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum CsvwDatatypeFormat {
    /// UTS 35 date/time, boolean, or regular-expression-like format string.
    Pattern(String),
    /// UTS 35 numeric pattern and separators.
    Numeric(CsvwNumericFormat),
}

/// CSVW datatype and its value-space facets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwDatatype {
    /// Optional custom datatype identity used for emitted literals.
    pub id: Option<String>,
    /// Expanded datatype IRI.
    pub base: String,
    /// Optional lexical format.
    pub format: Option<CsvwDatatypeFormat>,
    /// Exact string length facet.
    pub length: Option<usize>,
    /// Minimum string length facet.
    pub min_length: Option<usize>,
    /// Maximum string length facet.
    pub max_length: Option<usize>,
    /// Inclusive lower value-space bound in metadata JSON form.
    pub minimum: Option<Value>,
    /// Inclusive upper value-space bound in metadata JSON form.
    pub maximum: Option<Value>,
    /// Inclusive lower value-space bound.
    pub min_inclusive: Option<Value>,
    /// Inclusive upper value-space bound.
    pub max_inclusive: Option<Value>,
    /// Exclusive lower value-space bound.
    pub min_exclusive: Option<Value>,
    /// Exclusive upper value-space bound.
    pub max_exclusive: Option<Value>,
}

/// Properties inherited by table, schema, and column descriptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwInheritedProperties {
    /// URI template selecting each described subject.
    pub about_url: Option<String>,
    /// Cell datatype.
    pub datatype: CsvwDatatype,
    /// Replacement value used for an otherwise empty cell.
    pub default: String,
    /// Default language tag.
    pub language: Option<String>,
    /// Values treated as null.
    pub nulls: Vec<String>,
    /// Whether separator values become an RDF list.
    pub ordered: bool,
    /// URI template selecting the predicate.
    pub property_url: Option<String>,
    /// Whether a non-null value is mandatory.
    pub required: bool,
    /// Separator used to split one cell into multiple values.
    pub separator: Option<String>,
    /// Base direction of string values.
    pub text_direction: Option<CsvwTextDirection>,
    /// URI template selecting an IRI object.
    pub value_url: Option<String>,
}

/// A normalized value produced by parsing one CSV cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwValue {
    /// Source component after trim/default/separator handling.
    pub source: String,
    /// Lexical form emitted in RDF after datatype-format normalization.
    pub lexical: String,
    /// Expanded datatype IRI.
    pub datatype: String,
    /// Language tag, when applicable.
    pub language: Option<String>,
    /// Text direction, when applicable.
    pub direction: Option<CsvwTextDirection>,
}

/// One annotated table cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwCell {
    /// Zero-based column index in the normalized schema.
    pub column: usize,
    /// Raw field after dialect-level preprocessing.
    pub string_value: String,
    /// Parsed non-null values.
    pub values: Vec<CsvwValue>,
    /// Whether this cell matched a declared null marker.
    pub is_null: bool,
}

/// One annotated table row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwRow {
    /// One-based row number among data records.
    pub number: usize,
    /// One-based physical source record number.
    pub source_number: usize,
    /// Absolute row URL.
    pub url: String,
    /// Expanded row-title strings.
    pub titles: Vec<String>,
    /// Cells in normalized schema order.
    pub cells: Vec<CsvwCell>,
}

/// One column description in a CSVW table schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwColumn {
    /// Absolute column-description identity, when supplied.
    pub id: Option<String>,
    /// Zero-based index in the normalized schema.
    pub number: usize,
    /// Column name used by templates and key references.
    pub name: String,
    /// Whether `name` was explicitly supplied by metadata.
    pub name_explicit: bool,
    /// Natural-language titles.
    pub titles: CsvwNaturalLanguage,
    /// Whether the column has no source field.
    pub virtual_column: bool,
    /// Whether RDF output for this column is suppressed.
    pub suppress_output: bool,
    /// Effective inherited properties.
    pub inherited: CsvwInheritedProperties,
    /// Expanded common annotations.
    pub annotations: CsvwAnnotations,
}

/// A foreign-key reference target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwReference {
    /// Referenced table URL, when targeting another table.
    pub resource: Option<String>,
    /// Referenced schema identity, when targeting a reusable schema.
    pub schema_reference: Option<String>,
    /// Referenced column names in tuple order.
    pub column_reference: Vec<String>,
}

/// A table-schema foreign-key constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwForeignKey {
    /// Local column names in tuple order.
    pub column_reference: Vec<String>,
    /// Reference target.
    pub reference: CsvwReference,
}

/// A CSVW transformation description retained by the annotated model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTransformation {
    /// Absolute transformation identity, when supplied.
    pub id: Option<String>,
    /// Absolute script URL.
    pub url: String,
    /// Natural-language titles.
    pub titles: CsvwNaturalLanguage,
    /// Result media type.
    pub target_format: String,
    /// Script media type.
    pub script_format: String,
    /// Optional inline script source.
    pub source: Option<String>,
}

/// A normalized CSVW table schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwSchema {
    /// Absolute schema identity, when supplied.
    pub id: Option<String>,
    /// Ordered columns, including virtual columns.
    pub columns: Vec<CsvwColumn>,
    /// Whether a `tableSchema` value was explicitly supplied.
    pub metadata_explicit: bool,
    /// Primary-key column names in tuple order.
    pub primary_key: Vec<String>,
    /// Foreign-key constraints.
    pub foreign_keys: Vec<CsvwForeignKey>,
    /// Column names used as row titles.
    pub row_titles: Vec<String>,
    /// Effective inherited properties for child columns.
    pub inherited: CsvwInheritedProperties,
    /// Expanded common annotations.
    pub annotations: CsvwAnnotations,
}

/// One annotated CSVW table and its parsed rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTable {
    /// Absolute table-description identity, when supplied.
    pub id: Option<String>,
    /// Absolute source URL.
    pub url: String,
    /// Effective table dialect.
    pub dialect: CsvwDialect,
    /// Effective table schema.
    pub schema: CsvwSchema,
    /// Whether all table RDF output is suppressed.
    pub suppress_output: bool,
    /// Presentation direction.
    pub table_direction: CsvwTableDirection,
    /// Parsed rows.
    pub rows: Vec<CsvwRow>,
    /// Comment lines associated with the table.
    pub comments: Vec<String>,
    /// Retained transformation descriptions.
    pub transformations: Vec<CsvwTransformation>,
    /// Expanded common annotations.
    pub annotations: CsvwAnnotations,
}

/// A normalized CSVW table group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CsvwTableGroup {
    /// Caller-owned absolute group identity.
    pub id: String,
    /// Metadata-authored RDF identifier, or `None` for the normative blank node.
    pub rdf_id: Option<String>,
    /// Tables in metadata order.
    pub tables: Vec<CsvwTable>,
    /// Expanded common annotations.
    pub annotations: CsvwAnnotations,
}

impl CsvwTableGroup {
    /// Validate cross-record identities and references.
    ///
    /// # Errors
    ///
    /// Returns a semantic failure for duplicate table URLs, duplicate column
    /// names, dangling row cells, malformed identities, or invalid key references.
    pub fn validate(&self) -> Result<(), ProjectionError> {
        validate_absolute_iri(&self.id, "CSVW table-group identity")?;
        if let Some(id) = &self.rdf_id {
            validate_absolute_iri(id, "CSVW table-group RDF identity")?;
        }
        let mut table_urls = BTreeMap::new();
        for (table_index, table) in self.tables.iter().enumerate() {
            validate_absolute_iri(&table.url, "CSVW table URL")?;
            if table_urls.insert(table.url.as_str(), table_index).is_some() {
                return Err(ProjectionError::integrity(format!(
                    "duplicate CSVW table URL `{}`",
                    table.url
                )));
            }
            let mut columns = BTreeMap::new();
            for (column_index, column) in table.schema.columns.iter().enumerate() {
                if column.number != column_index {
                    return Err(ProjectionError::integrity(format!(
                        "CSVW column `{}` has non-canonical number {}",
                        column.name, column.number
                    )));
                }
                if column.name.is_empty() {
                    return Err(ProjectionError::integrity(
                        "CSVW column names must not be empty",
                    ));
                }
                if columns.insert(column.name.as_str(), column_index).is_some() {
                    return Err(ProjectionError::integrity(format!(
                        "duplicate CSVW column name `{}`",
                        column.name
                    )));
                }
                validate_absolute_iri(&column.inherited.datatype.base, "CSVW datatype")?;
            }
            validate_key_columns(&table.schema.primary_key, &columns, "primary key")?;
            validate_key_columns(&table.schema.row_titles, &columns, "row titles")?;
            for foreign_key in &table.schema.foreign_keys {
                validate_key_columns(&foreign_key.column_reference, &columns, "foreign key")?;
                if foreign_key.column_reference.len()
                    != foreign_key.reference.column_reference.len()
                {
                    return Err(ProjectionError::integrity(
                        "CSVW foreign-key source and target arities differ",
                    ));
                }
            }
            for (row_index, row) in table.rows.iter().enumerate() {
                if row.number != row_index + 1 {
                    return Err(ProjectionError::integrity(
                        "CSVW row numbers must be contiguous and one-based",
                    ));
                }
                if row.cells.len() != table.schema.columns.len()
                    || row
                        .cells
                        .iter()
                        .enumerate()
                        .any(|(index, cell)| cell.column != index)
                {
                    return Err(ProjectionError::integrity(
                        "CSVW row cells do not match the normalized schema",
                    ));
                }
            }
        }
        for table in &self.tables {
            for foreign_key in &table.schema.foreign_keys {
                if let Some(resource) = &foreign_key.reference.resource {
                    let target_index = table_urls.get(resource.as_str()).ok_or_else(|| {
                        ProjectionError::integrity(format!(
                            "CSVW foreign key references unknown table `{resource}`"
                        ))
                    })?;
                    let target = &self.tables[*target_index];
                    let target_columns = target
                        .schema
                        .columns
                        .iter()
                        .map(|column| column.name.as_str())
                        .collect::<std::collections::BTreeSet<_>>();
                    for column in &foreign_key.reference.column_reference {
                        if !target_columns.contains(column.as_str()) {
                            return Err(ProjectionError::integrity(format!(
                                "CSVW foreign key references unknown target column `{column}`"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_key_columns(
    names: &[String],
    columns: &BTreeMap<&str, usize>,
    role: &str,
) -> Result<(), ProjectionError> {
    let mut seen = std::collections::BTreeSet::new();
    for name in names {
        if !columns.contains_key(name.as_str()) {
            return Err(ProjectionError::integrity(format!(
                "CSVW {role} references unknown column `{name}`"
            )));
        }
        if !seen.insert(name) {
            return Err(ProjectionError::integrity(format!(
                "CSVW {role} repeats column `{name}`"
            )));
        }
    }
    Ok(())
}

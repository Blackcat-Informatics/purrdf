// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CSVW metadata normalization and inherited-property processing.

use std::collections::{BTreeMap, BTreeSet};

use oxilangtag::LanguageTag;
use purrdf_xsd::{XsdDatatype, parse as parse_xsd, value_cmp};
use regex::Regex;
use serde_json::{Map, Value};

use super::super::{ProjectionError, validate_absolute_iri};
use super::config::{CsvwConfig, CsvwContext};
use super::input::{CsvwAction, CsvwInput, CsvwWarning, CsvwWarningKind};
use super::model::{
    CsvwAnnotations, CsvwColumn, CsvwDatatype, CsvwDatatypeFormat, CsvwDialect, CsvwForeignKey,
    CsvwInheritedProperties, CsvwNaturalLanguage, CsvwNumericFormat, CsvwReference, CsvwSchema,
    CsvwTable, CsvwTableDirection, CsvwTableGroup, CsvwTextDirection, CsvwTransformation, CsvwTrim,
};

pub(crate) struct MetadataOutcome {
    pub group: CsvwTableGroup,
    pub warnings: Vec<CsvwWarning>,
}

pub(crate) fn load_metadata(
    input: &CsvwInput,
    config: &CsvwConfig,
) -> Result<MetadataOutcome, ProjectionError> {
    let mut loader = MetadataLoader {
        input,
        config,
        warnings: Vec::new(),
        active_resources: BTreeSet::new(),
    };
    let group = match input.action() {
        CsvwAction::Table {
            table_iri,
            metadata_iri: Some(metadata_iri),
        } => {
            let mut group = loader.load_document(metadata_iri)?;
            let matching = group
                .tables
                .iter()
                .position(|table| table.url == *table_iri)
                .ok_or_else(|| {
                    ProjectionError::integrity(format!(
                        "CSVW metadata `{metadata_iri}` does not describe `{table_iri}`"
                    ))
                })?;
            let table = group.tables.remove(matching);
            group.tables = vec![table];
            group
        }
        CsvwAction::Table {
            table_iri,
            metadata_iri: None,
        } => loader.embedded_table(table_iri),
        CsvwAction::Metadata { metadata_iri } => loader.load_document(metadata_iri)?,
    };
    if group.tables.len() > config.max_records() {
        return Err(ProjectionError::limit(format!(
            "CSVW table group exceeds the {}-record limit",
            config.max_records()
        )));
    }
    Ok(MetadataOutcome {
        group,
        warnings: loader.warnings,
    })
}

struct MetadataLoader<'a> {
    input: &'a CsvwInput,
    config: &'a CsvwConfig,
    warnings: Vec<CsvwWarning>,
    active_resources: BTreeSet<String>,
}

impl MetadataLoader<'_> {
    fn parse_root_context(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
    ) -> Result<DocumentContext, ProjectionError> {
        let context_value = object.get("@context").ok_or_else(|| {
            ProjectionError::integrity("CSVW metadata requires an @context").at_path(resource)
        })?;
        let mut base_iri = resource.to_owned();
        let mut language = None;
        let mut prefixes = self.config.context().prefixes().clone();
        match context_value {
            Value::String(iri) => {
                if iri != self.config.context_iri() {
                    return Err(ProjectionError::integrity(format!(
                        "CSVW metadata context `{iri}` does not match caller context `{}`",
                        self.config.context_iri()
                    ))
                    .at_path(resource));
                }
            }
            Value::Array(values) => {
                if values.is_empty() || values.len() > 2 {
                    return Err(ProjectionError::integrity(
                        "CSVW @context array must contain the context IRI and at most one object",
                    )
                    .at_path(resource));
                }
                if values[0].as_str() != Some(self.config.context_iri()) {
                    return Err(ProjectionError::integrity(
                        "CSVW @context array must begin with the caller context IRI",
                    )
                    .at_path(resource));
                }
                if let Some(value) = values.get(1) {
                    let map = value.as_object().ok_or_else(|| {
                        ProjectionError::integrity(
                            "second CSVW @context array entry must be an object",
                        )
                        .at_path(resource)
                    })?;
                    parse_context_object(
                        map,
                        resource,
                        &mut base_iri,
                        &mut language,
                        &mut prefixes,
                        self.config,
                        &mut self.warnings,
                    )?;
                }
            }
            Value::Object(map) => {
                parse_context_object(
                    map,
                    resource,
                    &mut base_iri,
                    &mut language,
                    &mut prefixes,
                    self.config,
                    &mut self.warnings,
                )?;
            }
            _ => {
                return Err(ProjectionError::integrity(
                    "CSVW @context must be a string, object, or two-entry array",
                )
                .at_path(resource));
            }
        }
        let expansion = CsvwContext::new(self.config.context_iri(), prefixes)?;
        Ok(DocumentContext {
            base_iri,
            language,
            expansion,
        })
    }

    fn parse_inherited(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        location: &str,
        parent: &CsvwInheritedProperties,
        context: &DocumentContext,
    ) -> Result<CsvwInheritedProperties, ProjectionError> {
        let mut inherited = parent.clone();
        inherited.about_url = template_property(
            object.get("aboutUrl"),
            inherited.about_url,
            resource,
            &format!("{location}.aboutUrl"),
            &mut self.warnings,
        );
        if let Some(value) = object.get("datatype") {
            inherited.datatype = self.parse_datatype(
                value,
                resource,
                &format!("{location}.datatype"),
                context,
                &parent.datatype,
            )?;
        }
        inherited.default = atomic_string(
            object.get("default"),
            &inherited.default,
            resource,
            &format!("{location}.default"),
            &mut self.warnings,
        );
        inherited.language = language_property(
            object.get("lang"),
            inherited.language,
            resource,
            &format!("{location}.lang"),
            &mut self.warnings,
        );
        inherited.nulls = string_or_array(
            object.get("null"),
            &inherited.nulls,
            resource,
            &format!("{location}.null"),
            &mut self.warnings,
        );
        inherited.ordered = atomic_bool(
            object.get("ordered"),
            inherited.ordered,
            resource,
            &format!("{location}.ordered"),
            &mut self.warnings,
        );
        inherited.property_url = template_property(
            object.get("propertyUrl"),
            inherited.property_url,
            resource,
            &format!("{location}.propertyUrl"),
            &mut self.warnings,
        );
        inherited.required = atomic_bool(
            object.get("required"),
            inherited.required,
            resource,
            &format!("{location}.required"),
            &mut self.warnings,
        );
        inherited.separator = optional_string(
            object.get("separator"),
            inherited.separator,
            resource,
            &format!("{location}.separator"),
            &mut self.warnings,
        );
        inherited.text_direction = parse_text_direction(
            object.get("textDirection"),
            inherited.text_direction,
            resource,
            &format!("{location}.textDirection"),
            &mut self.warnings,
        );
        inherited.value_url = template_property(
            object.get("valueUrl"),
            inherited.value_url,
            resource,
            &format!("{location}.valueUrl"),
            &mut self.warnings,
        );
        Ok(inherited)
    }

    fn parse_schema_property(
        &mut self,
        value: Option<&Value>,
        resource: &str,
        context: &DocumentContext,
        inherited: &CsvwInheritedProperties,
    ) -> Result<CsvwSchema, ProjectionError> {
        let Some(value) = value else {
            return Ok(empty_schema(inherited.clone(), false));
        };
        match value {
            Value::Object(object) => {
                self.parse_schema(object, resource, context, inherited, "$.table.tableSchema")
            }
            Value::String(reference) => {
                let iri = resolve_iri(&context.base_iri, reference, "CSVW schema reference")?;
                if !self.active_resources.insert(iri.clone()) {
                    return Err(ProjectionError::integrity(format!(
                        "cyclic CSVW schema reference through `{iri}`"
                    )));
                }
                let bytes = self.input.get(&iri).ok_or_else(|| {
                    ProjectionError::package(format!("CSVW schema resource `{iri}` is absent"))
                })?;
                let value: Value = serde_json::from_slice(bytes).map_err(|error| {
                    ProjectionError::syntax(format!("invalid CSVW schema JSON: {error}"))
                        .at_path(&iri)
                })?;
                let object = value.as_object().ok_or_else(|| {
                    ProjectionError::syntax("CSVW schema resource must be a JSON object")
                        .at_path(&iri)
                })?;
                let schema_context = DocumentContext {
                    base_iri: iri.clone(),
                    language: context.language.clone(),
                    expansion: context.expansion.clone(),
                };
                let schema = self.parse_schema(object, &iri, &schema_context, inherited, "$");
                self.active_resources.remove(&iri);
                schema
            }
            _ => {
                self.warn(
                    resource,
                    "$.table.tableSchema",
                    "invalid tableSchema value was ignored",
                );
                Ok(empty_schema(inherited.clone(), true))
            }
        }
    }

    fn parse_schema(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        context: &DocumentContext,
        inherited: &CsvwInheritedProperties,
        location: &str,
    ) -> Result<CsvwSchema, ProjectionError> {
        check_type(object, "Schema", resource)?;
        let id = parse_optional_id(
            object,
            resource,
            context,
            &format!("{location}.@id"),
            &mut self.warnings,
        )?;
        let inherited = self.parse_inherited(object, resource, location, inherited, context)?;
        let values =
            optional_array_property(object, "columns", resource, location, &mut self.warnings);
        let mut columns = Vec::with_capacity(values.len());
        let mut virtual_seen = false;
        for (index, value) in values.iter().enumerate() {
            let Some(column) = value.as_object() else {
                self.warn(
                    resource,
                    format!("{location}.columns[{index}]"),
                    "non-object column description was ignored",
                );
                continue;
            };
            let parsed = self.parse_column(
                column,
                resource,
                context,
                &inherited,
                columns.len(),
                &format!("{location}.columns[{index}]"),
            )?;
            if virtual_seen && !parsed.virtual_column {
                return Err(ProjectionError::integrity(
                    "CSVW virtual columns must follow all non-virtual columns",
                )
                .at_path(resource));
            }
            virtual_seen |= parsed.virtual_column;
            columns.push(parsed);
        }
        ensure_unique_column_names(&columns, resource)?;
        let mut primary_key = column_reference_property(
            object.get("primaryKey"),
            resource,
            &format!("{location}.primaryKey"),
            &mut self.warnings,
        );
        if primary_key.iter().any(|name| {
            columns
                .iter()
                .find(|column| column.name == *name)
                .is_none_or(|column| !column.name_explicit)
        }) {
            self.warn(
                resource,
                format!("{location}.primaryKey"),
                "primaryKey referenced a column without an explicit name and was ignored",
            );
            primary_key.clear();
        }
        let row_titles = column_reference_property(
            object.get("rowTitles"),
            resource,
            &format!("{location}.rowTitles"),
            &mut self.warnings,
        );
        let foreign_keys =
            self.parse_foreign_keys(object.get("foreignKeys"), resource, context, location)?;
        for foreign_key in &foreign_keys {
            for name in &foreign_key.column_reference {
                if columns
                    .iter()
                    .find(|column| column.name == *name)
                    .is_none_or(|column| !column.name_explicit)
                {
                    return Err(ProjectionError::integrity(format!(
                        "CSVW foreign key references column `{name}` without an explicit name"
                    ))
                    .at_path(resource));
                }
            }
        }
        let annotations =
            self.parse_annotations(object, SCHEMA_PROPERTIES, resource, location, context)?;
        self.warn_unknown(object, SCHEMA_PROPERTIES, resource, location, context);
        Ok(CsvwSchema {
            id,
            columns,
            metadata_explicit: true,
            primary_key,
            foreign_keys,
            row_titles,
            inherited,
            annotations,
        })
    }

    fn parse_column(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        context: &DocumentContext,
        inherited: &CsvwInheritedProperties,
        number: usize,
        location: &str,
    ) -> Result<CsvwColumn, ProjectionError> {
        check_type(object, "Column", resource)?;
        let id = parse_optional_id(
            object,
            resource,
            context,
            &format!("{location}.@id"),
            &mut self.warnings,
        )?;
        let titles = natural_language_property(
            object.get("titles"),
            context.language.as_deref(),
            resource,
            &format!("{location}.titles"),
            &mut self.warnings,
        );
        let (name, name_explicit) = match object.get("name") {
            Some(Value::String(value)) if valid_column_name(value) => (value.clone(), true),
            Some(Value::String(value)) => {
                self.warn(
                    resource,
                    format!("{location}.name"),
                    format!("invalid CSVW column name `{value}` was ignored"),
                );
                (
                    name_from_titles(&titles, context.language.as_deref(), number),
                    false,
                )
            }
            Some(_) => {
                self.warn(
                    resource,
                    format!("{location}.name"),
                    "non-string CSVW column name was ignored",
                );
                (
                    name_from_titles(&titles, context.language.as_deref(), number),
                    false,
                )
            }
            None => (
                name_from_titles(&titles, context.language.as_deref(), number),
                false,
            ),
        };
        let virtual_column = atomic_bool(
            object.get("virtual"),
            false,
            resource,
            &format!("{location}.virtual"),
            &mut self.warnings,
        );
        let suppress_output = atomic_bool(
            object.get("suppressOutput"),
            false,
            resource,
            &format!("{location}.suppressOutput"),
            &mut self.warnings,
        );
        let inherited = self.parse_inherited(object, resource, location, inherited, context)?;
        let annotations =
            self.parse_annotations(object, COLUMN_PROPERTIES, resource, location, context)?;
        self.warn_unknown(object, COLUMN_PROPERTIES, resource, location, context);
        Ok(CsvwColumn {
            id,
            number,
            name,
            name_explicit,
            titles,
            virtual_column,
            suppress_output,
            inherited,
            annotations,
        })
    }

    fn parse_datatype(
        &mut self,
        value: &Value,
        resource: &str,
        location: &str,
        context: &DocumentContext,
        fallback: &CsvwDatatype,
    ) -> Result<CsvwDatatype, ProjectionError> {
        match value {
            Value::String(name) => {
                let Some(base) = expand_datatype(name, context, self.config, false) else {
                    self.warn(
                        resource,
                        location,
                        format!("invalid CSVW datatype `{name}` was ignored"),
                    );
                    return Ok(fallback.clone());
                };
                Ok(CsvwDatatype {
                    id: None,
                    base,
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
                })
            }
            Value::Object(object) => {
                check_type(object, "Datatype", resource)?;
                let id = match object.get("@id") {
                    None => None,
                    Some(Value::String(value)) => {
                        if value.starts_with("_:") {
                            return Err(ProjectionError::integrity(
                                "CSVW datatype @id must not be a blank node",
                            )
                            .at_path(resource));
                        }
                        let iri = resolve_iri(&context.base_iri, value, "CSVW datatype @id")?;
                        if builtin_datatype_iri(&iri, self.config) {
                            return Err(ProjectionError::integrity(
                                "CSVW datatype @id must not identify a built-in datatype",
                            )
                            .at_path(resource));
                        }
                        Some(iri)
                    }
                    Some(_) => {
                        self.warn(
                            resource,
                            format!("{location}.@id"),
                            "invalid datatype @id was ignored",
                        );
                        None
                    }
                };
                let base_name = object
                    .get("base")
                    .and_then(Value::as_str)
                    .unwrap_or("string");
                let Some(base) = expand_datatype(base_name, context, self.config, false) else {
                    self.warn(
                        resource,
                        format!("{location}.base"),
                        format!("invalid CSVW datatype base `{base_name}` was ignored"),
                    );
                    return Ok(fallback.clone());
                };
                let format_location = format!("{location}.format");
                let format = validate_datatype_format(
                    &base,
                    parse_datatype_format(
                        object.get("format"),
                        resource,
                        &format_location,
                        &mut self.warnings,
                    ),
                    self.config,
                    resource,
                    &format_location,
                    &mut self.warnings,
                );
                let length = nonnegative_integer(
                    object.get("length"),
                    resource,
                    &format!("{location}.length"),
                    &mut self.warnings,
                );
                let min_length = nonnegative_integer(
                    object.get("minLength"),
                    resource,
                    &format!("{location}.minLength"),
                    &mut self.warnings,
                );
                let max_length = nonnegative_integer(
                    object.get("maxLength"),
                    resource,
                    &format!("{location}.maxLength"),
                    &mut self.warnings,
                );
                if min_length
                    .zip(max_length)
                    .is_some_and(|(min, max)| min > max)
                    || length
                        .zip(min_length)
                        .is_some_and(|(length, min)| length < min)
                    || length
                        .zip(max_length)
                        .is_some_and(|(length, max)| length > max)
                {
                    return Err(ProjectionError::integrity(
                        "CSVW datatype length facets are contradictory",
                    )
                    .at_path(resource));
                }
                if (length.is_some() || min_length.is_some() || max_length.is_some())
                    && !length_datatype(&base, self.config)
                {
                    return Err(ProjectionError::integrity(
                        "CSVW length facets require a string or binary datatype",
                    )
                    .at_path(resource));
                }
                let datatype = CsvwDatatype {
                    id,
                    base,
                    format,
                    length,
                    min_length,
                    max_length,
                    minimum: object.get("minimum").cloned(),
                    maximum: object.get("maximum").cloned(),
                    min_inclusive: object.get("minInclusive").cloned(),
                    max_inclusive: object.get("maxInclusive").cloned(),
                    min_exclusive: object.get("minExclusive").cloned(),
                    max_exclusive: object.get("maxExclusive").cloned(),
                };
                validate_facet_combinations(&datatype, resource, self.config)?;
                Ok(datatype)
            }
            _ => {
                self.warn(resource, location, "invalid CSVW datatype was ignored");
                Ok(fallback.clone())
            }
        }
    }

    fn parse_dialect_property(
        &mut self,
        value: Option<&Value>,
        resource: &str,
        context: &DocumentContext,
    ) -> Result<CsvwDialect, ProjectionError> {
        let Some(value) = value else {
            return Ok(CsvwDialect::default());
        };
        match value {
            Value::Object(object) => self.parse_dialect(object, resource, "$.table.dialect"),
            Value::String(reference) => {
                let iri = resolve_iri(&context.base_iri, reference, "CSVW dialect reference")?;
                let bytes = self.input.get(&iri).ok_or_else(|| {
                    ProjectionError::package(format!("CSVW dialect resource `{iri}` is absent"))
                })?;
                let value: Value = serde_json::from_slice(bytes).map_err(|error| {
                    ProjectionError::syntax(format!("invalid CSVW dialect JSON: {error}"))
                        .at_path(&iri)
                })?;
                let object = value.as_object().ok_or_else(|| {
                    ProjectionError::syntax("CSVW dialect resource must be a JSON object")
                        .at_path(&iri)
                })?;
                self.parse_dialect(object, &iri, "$")
            }
            _ => {
                self.warn(
                    resource,
                    "$.table.dialect",
                    "invalid CSVW dialect was ignored",
                );
                Ok(CsvwDialect::default())
            }
        }
    }

    fn parse_dialect(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        location: &str,
    ) -> Result<CsvwDialect, ProjectionError> {
        check_type(object, "Dialect", resource)?;
        if let Some(value) = object.get("@id") {
            parse_id_value(value, resource, resource)?;
        }
        let mut dialect = CsvwDialect::default();
        dialect.comment_prefix = optional_string(
            object.get("commentPrefix"),
            dialect.comment_prefix,
            resource,
            &format!("{location}.commentPrefix"),
            &mut self.warnings,
        );
        dialect.delimiter = single_character(
            object.get("delimiter"),
            dialect.delimiter,
            resource,
            &format!("{location}.delimiter"),
            &mut self.warnings,
        );
        dialect.double_quote = atomic_bool(
            object.get("doubleQuote"),
            dialect.double_quote,
            resource,
            &format!("{location}.doubleQuote"),
            &mut self.warnings,
        );
        dialect.encoding = atomic_string(
            object.get("encoding"),
            &dialect.encoding,
            resource,
            &format!("{location}.encoding"),
            &mut self.warnings,
        )
        .to_ascii_lowercase();
        if !matches!(dialect.encoding.as_str(), "utf-8" | "utf8") {
            self.warn(
                resource,
                format!("{location}.encoding"),
                format!(
                    "unsupported encoding `{}` was replaced by UTF-8",
                    dialect.encoding
                ),
            );
            "utf-8".clone_into(&mut dialect.encoding);
        }
        let explicit_header_count = object.contains_key("headerRowCount");
        if !explicit_header_count && let Some(value) = object.get("header") {
            dialect.header_row_count = usize::from(atomic_bool(
                Some(value),
                true,
                resource,
                &format!("{location}.header"),
                &mut self.warnings,
            ));
        }
        if explicit_header_count {
            dialect.header_row_count = nonnegative_integer(
                object.get("headerRowCount"),
                resource,
                &format!("{location}.headerRowCount"),
                &mut self.warnings,
            )
            .unwrap_or(1);
        }
        dialect.line_terminators = string_or_array(
            object.get("lineTerminators"),
            &dialect.line_terminators,
            resource,
            &format!("{location}.lineTerminators"),
            &mut self.warnings,
        );
        dialect.quote_char = optional_character(
            object.get("quoteChar"),
            dialect.quote_char,
            resource,
            &format!("{location}.quoteChar"),
            &mut self.warnings,
        );
        dialect.skip_blank_rows = atomic_bool(
            object.get("skipBlankRows"),
            dialect.skip_blank_rows,
            resource,
            &format!("{location}.skipBlankRows"),
            &mut self.warnings,
        );
        dialect.skip_columns = nonnegative_integer(
            object.get("skipColumns"),
            resource,
            &format!("{location}.skipColumns"),
            &mut self.warnings,
        )
        .unwrap_or(dialect.skip_columns);
        dialect.skip_initial_space = atomic_bool(
            object.get("skipInitialSpace"),
            dialect.skip_initial_space,
            resource,
            &format!("{location}.skipInitialSpace"),
            &mut self.warnings,
        );
        dialect.skip_rows = nonnegative_integer(
            object.get("skipRows"),
            resource,
            &format!("{location}.skipRows"),
            &mut self.warnings,
        )
        .unwrap_or(dialect.skip_rows);
        dialect.trim = parse_trim(
            object.get("trim"),
            dialect.trim,
            resource,
            &format!("{location}.trim"),
            &mut self.warnings,
        );
        self.warn_unknown(
            object,
            DIALECT_PROPERTIES,
            resource,
            location,
            &DocumentContext {
                base_iri: resource.to_owned(),
                language: None,
                expansion: self.config.context().clone(),
            },
        );
        Ok(dialect)
    }

    fn parse_foreign_keys(
        &mut self,
        value: Option<&Value>,
        resource: &str,
        context: &DocumentContext,
        location: &str,
    ) -> Result<Vec<CsvwForeignKey>, ProjectionError> {
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let Some(values) = value.as_array() else {
            self.warn(
                resource,
                format!("{location}.foreignKeys"),
                "non-array foreignKeys value was ignored",
            );
            return Ok(Vec::new());
        };
        let mut foreign_keys = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let Some(object) = value.as_object() else {
                self.warn(
                    resource,
                    format!("{location}.foreignKeys[{index}]"),
                    "non-object foreign key was ignored",
                );
                continue;
            };
            ensure_only_properties(
                object,
                &["columnReference", "reference"],
                resource,
                "CSVW foreign key",
            )?;
            let columns = column_reference_property(
                object.get("columnReference"),
                resource,
                &format!("{location}.foreignKeys[{index}].columnReference"),
                &mut self.warnings,
            );
            if columns.is_empty() {
                return Err(ProjectionError::integrity(
                    "CSVW foreign key requires a valid columnReference",
                )
                .at_path(resource));
            }
            let Some(reference) = object.get("reference").and_then(Value::as_object) else {
                return Err(ProjectionError::integrity(
                    "CSVW foreign key requires a reference object",
                )
                .at_path(resource));
            };
            ensure_only_properties(
                reference,
                &["columnReference", "resource", "schemaReference"],
                resource,
                "CSVW foreign-key reference",
            )?;
            let target_columns = column_reference_property(
                reference.get("columnReference"),
                resource,
                &format!("{location}.foreignKeys[{index}].reference.columnReference"),
                &mut self.warnings,
            );
            let target_resource = link_property(
                reference.get("resource"),
                &context.base_iri,
                resource,
                &format!("{location}.foreignKeys[{index}].reference.resource"),
                &mut self.warnings,
            )?;
            let schema_reference = link_property(
                reference.get("schemaReference"),
                &context.base_iri,
                resource,
                &format!("{location}.foreignKeys[{index}].reference.schemaReference"),
                &mut self.warnings,
            )?;
            if target_resource.is_some() == schema_reference.is_some()
                || target_columns.is_empty()
                || columns.len() != target_columns.len()
            {
                return Err(ProjectionError::integrity(
                    "CSVW foreign key has an invalid reference",
                )
                .at_path(resource));
            }
            foreign_keys.push(CsvwForeignKey {
                column_reference: columns,
                reference: CsvwReference {
                    resource: target_resource,
                    schema_reference,
                    column_reference: target_columns,
                },
            });
        }
        Ok(foreign_keys)
    }

    fn parse_transformations(
        &mut self,
        value: Option<&Value>,
        resource: &str,
        context: &DocumentContext,
    ) -> Result<Vec<CsvwTransformation>, ProjectionError> {
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let Some(values) = value.as_array() else {
            self.warn(
                resource,
                "$.table.transformations",
                "non-array transformations value was ignored",
            );
            return Ok(Vec::new());
        };
        let mut transformations = Vec::new();
        for (index, value) in values.iter().enumerate() {
            let Some(object) = value.as_object() else {
                self.warn(
                    resource,
                    format!("$.table.transformations[{index}]"),
                    "non-object transformation was ignored",
                );
                continue;
            };
            check_type(object, "Template", resource)?;
            let transformation_location = format!("$.table.transformations[{index}]");
            for key in object.keys() {
                if !TRANSFORMATION_PROPERTIES.contains(&key.as_str()) {
                    self.warn(
                        resource,
                        format!("{transformation_location}.{key}"),
                        format!("unknown CSVW transformation property `{key}` was ignored"),
                    );
                }
            }
            let id = parse_optional_id(
                object,
                resource,
                context,
                &format!("$.table.transformations[{index}].@id"),
                &mut self.warnings,
            )?;
            let Some(url) = object.get("url").and_then(Value::as_str) else {
                self.warn(
                    resource,
                    format!("$.table.transformations[{index}].url"),
                    "transformation without a string URL was ignored",
                );
                continue;
            };
            let target_format = object
                .get("targetFormat")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let script_format = object
                .get("scriptFormat")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if target_format.is_empty() || script_format.is_empty() {
                self.warn(
                    resource,
                    format!("$.table.transformations[{index}]"),
                    "transformation without targetFormat/scriptFormat was ignored",
                );
                continue;
            }
            transformations.push(CsvwTransformation {
                id,
                url: resolve_iri(&context.base_iri, url, "CSVW transformation URL")?,
                titles: natural_language_property(
                    object.get("titles"),
                    context.language.as_deref(),
                    resource,
                    &format!("$.table.transformations[{index}].titles"),
                    &mut self.warnings,
                ),
                target_format,
                script_format,
                source: object
                    .get("source")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
        }
        Ok(transformations)
    }

    fn parse_annotations(
        &self,
        object: &Map<String, Value>,
        known: &[&str],
        resource: &str,
        location: &str,
        context: &DocumentContext,
    ) -> Result<CsvwAnnotations, ProjectionError> {
        let mut annotations = CsvwAnnotations::new();
        for (key, value) in object {
            if known.contains(&key.as_str()) || key.starts_with('@') {
                continue;
            }
            let Ok(expanded) = (if key == "notes" {
                Ok(self.config.vocabulary().csvw("note"))
            } else {
                expand_annotation_key(key, &context.expansion)
            }) else {
                continue;
            };
            validate_annotation_value(value, resource, &format!("{location}.{key}"), context)?;
            annotations.insert(expanded, normalize_annotation_value(value, context));
        }
        Ok(annotations)
    }

    fn warn_unknown(
        &mut self,
        object: &Map<String, Value>,
        known: &[&str],
        resource: &str,
        location: &str,
        context: &DocumentContext,
    ) {
        for key in object.keys() {
            if known.contains(&key.as_str()) || key.starts_with('@') {
                continue;
            }
            if key != "notes" && expand_annotation_key(key, &context.expansion).is_err() {
                self.warnings.push(CsvwWarning::new(
                    CsvwWarningKind::UnknownProperty,
                    resource,
                    format!("{location}.{key}"),
                    format!("unknown CSVW property `{key}` was ignored"),
                ));
            }
        }
    }

    fn load_document(&mut self, iri: &str) -> Result<CsvwTableGroup, ProjectionError> {
        if !self.active_resources.insert(iri.to_owned()) {
            return Err(ProjectionError::integrity(format!(
                "cyclic CSVW metadata reference through `{iri}`"
            )));
        }
        let bytes = self.input.get(iri).ok_or_else(|| {
            ProjectionError::package(format!("CSVW metadata resource `{iri}` is absent"))
        })?;
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|error| {
                ProjectionError::syntax(format!("invalid CSVW metadata JSON: {error}"))
            })
            .map_err(|error| error.at_path(iri))?;
        let object = value.as_object().ok_or_else(|| {
            ProjectionError::syntax("CSVW metadata root must be a JSON object").at_path(iri)
        })?;
        let context = self.parse_root_context(object, iri)?;
        let inherited = self.default_inherited(None);
        let result = if object.contains_key("tables") || type_is(object, "TableGroup") {
            self.parse_group(object, iri, &context, &inherited)
        } else {
            let table = self.parse_table(object, iri, &context, &inherited)?;
            Ok(CsvwTableGroup {
                id: self.config.table_group_iri().to_owned(),
                rdf_id: None,
                tables: vec![table],
                annotations: CsvwAnnotations::new(),
            })
        };
        self.active_resources.remove(iri);
        result
    }

    fn embedded_table(&self, table_iri: &str) -> CsvwTableGroup {
        CsvwTableGroup {
            id: self.config.table_group_iri().to_owned(),
            rdf_id: None,
            tables: vec![CsvwTable {
                id: None,
                url: table_iri.to_owned(),
                dialect: CsvwDialect::default(),
                schema: CsvwSchema {
                    id: None,
                    columns: Vec::new(),
                    metadata_explicit: false,
                    primary_key: Vec::new(),
                    foreign_keys: Vec::new(),
                    row_titles: Vec::new(),
                    inherited: self.default_inherited(None),
                    annotations: CsvwAnnotations::new(),
                },
                suppress_output: false,
                table_direction: CsvwTableDirection::Auto,
                rows: Vec::new(),
                comments: Vec::new(),
                transformations: Vec::new(),
                annotations: CsvwAnnotations::new(),
            }],
            annotations: CsvwAnnotations::new(),
        }
    }

    fn parse_group(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        context: &DocumentContext,
        inherited: &CsvwInheritedProperties,
    ) -> Result<CsvwTableGroup, ProjectionError> {
        check_type(object, "TableGroup", resource)?;
        let rdf_id = parse_optional_id(object, resource, context, "$.@id", &mut self.warnings)?;
        let id = rdf_id
            .clone()
            .unwrap_or_else(|| self.config.table_group_iri().to_owned());
        let inherited = self.parse_inherited(object, resource, "$", inherited, context)?;
        let dialect = self.parse_dialect_property(object.get("dialect"), resource, context)?;
        let table_direction = parse_table_direction(
            object.get("tableDirection"),
            CsvwTableDirection::Auto,
            resource,
            "$.tableDirection",
            &mut self.warnings,
        );
        let values = array_property(object, "tables", resource, "$", &mut self.warnings)?;
        if values.is_empty() {
            return Err(ProjectionError::integrity(
                "CSVW table group must contain at least one table",
            )
            .at_path(resource));
        }
        let mut tables = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            let Some(table) = value.as_object() else {
                self.warn(
                    resource,
                    format!("$.tables[{index}]"),
                    "non-object table description was ignored",
                );
                continue;
            };
            let mut parsed = self.parse_table(table, resource, context, &inherited)?;
            if !table.contains_key("dialect") {
                parsed.dialect = dialect.clone();
            }
            if !table.contains_key("tableDirection") {
                parsed.table_direction = table_direction;
            }
            tables.push(parsed);
        }
        if tables.is_empty() {
            return Err(ProjectionError::integrity(
                "CSVW table group has no valid table descriptions",
            )
            .at_path(resource));
        }
        ensure_unique_table_urls(&tables, resource)?;
        let annotations =
            self.parse_annotations(object, GROUP_PROPERTIES, resource, "$", context)?;
        self.warn_unknown(object, GROUP_PROPERTIES, resource, "$", context);
        Ok(CsvwTableGroup {
            id,
            rdf_id,
            tables,
            annotations,
        })
    }

    fn parse_table(
        &mut self,
        object: &Map<String, Value>,
        resource: &str,
        context: &DocumentContext,
        inherited: &CsvwInheritedProperties,
    ) -> Result<CsvwTable, ProjectionError> {
        check_type(object, "Table", resource)?;
        let id = parse_optional_id(object, resource, context, "$.table.@id", &mut self.warnings)?;
        let url_value = object.get("url").and_then(Value::as_str).ok_or_else(|| {
            ProjectionError::integrity("CSVW table requires a string `url`").at_path(resource)
        })?;
        let url = resolve_iri(&context.base_iri, url_value, "CSVW table URL")?;
        let inherited = self.parse_inherited(object, resource, "$.table", inherited, context)?;
        let dialect = self.parse_dialect_property(object.get("dialect"), resource, context)?;
        let schema =
            self.parse_schema_property(object.get("tableSchema"), resource, context, &inherited)?;
        let suppress_output = atomic_bool(
            object.get("suppressOutput"),
            false,
            resource,
            "$.table.suppressOutput",
            &mut self.warnings,
        );
        let table_direction = parse_table_direction(
            object.get("tableDirection"),
            CsvwTableDirection::Auto,
            resource,
            "$.table.tableDirection",
            &mut self.warnings,
        );
        let transformations =
            self.parse_transformations(object.get("transformations"), resource, context)?;
        let annotations =
            self.parse_annotations(object, TABLE_PROPERTIES, resource, "$.table", context)?;
        self.warn_unknown(object, TABLE_PROPERTIES, resource, "$.table", context);
        Ok(CsvwTable {
            id,
            url,
            dialect,
            schema,
            suppress_output,
            table_direction,
            rows: Vec::new(),
            comments: Vec::new(),
            transformations,
            annotations,
        })
    }

    fn default_inherited(&self, language: Option<String>) -> CsvwInheritedProperties {
        CsvwInheritedProperties {
            about_url: None,
            datatype: CsvwDatatype {
                id: None,
                base: self.config.vocabulary().xsd("string"),
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
            default: String::new(),
            language,
            nulls: vec![String::new()],
            ordered: false,
            property_url: None,
            required: false,
            separator: None,
            text_direction: None,
            value_url: None,
        }
    }

    fn warn(&mut self, resource: &str, location: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(CsvwWarning::new(
            CsvwWarningKind::InvalidValue,
            resource,
            location,
            message,
        ));
    }
}

#[derive(Clone)]
struct DocumentContext {
    base_iri: String,
    language: Option<String>,
    expansion: CsvwContext,
}

const GROUP_PROPERTIES: &[&str] = &[
    "@context",
    "@id",
    "@type",
    "aboutUrl",
    "datatype",
    "default",
    "dialect",
    "lang",
    "null",
    "ordered",
    "propertyUrl",
    "required",
    "separator",
    "tableDirection",
    "tables",
    "textDirection",
    "valueUrl",
];

const TABLE_PROPERTIES: &[&str] = &[
    "@context",
    "@id",
    "@type",
    "aboutUrl",
    "datatype",
    "default",
    "dialect",
    "lang",
    "null",
    "ordered",
    "propertyUrl",
    "required",
    "separator",
    "suppressOutput",
    "tableDirection",
    "tableSchema",
    "textDirection",
    "transformations",
    "url",
    "valueUrl",
];

const SCHEMA_PROPERTIES: &[&str] = &[
    "@context",
    "@id",
    "@type",
    "aboutUrl",
    "columns",
    "datatype",
    "default",
    "foreignKeys",
    "lang",
    "null",
    "ordered",
    "primaryKey",
    "propertyUrl",
    "required",
    "rowTitles",
    "separator",
    "textDirection",
    "valueUrl",
];

const COLUMN_PROPERTIES: &[&str] = &[
    "@context",
    "@id",
    "@type",
    "aboutUrl",
    "datatype",
    "default",
    "lang",
    "name",
    "null",
    "ordered",
    "propertyUrl",
    "required",
    "separator",
    "suppressOutput",
    "textDirection",
    "titles",
    "valueUrl",
    "virtual",
];

const DIALECT_PROPERTIES: &[&str] = &[
    "@id",
    "@type",
    "commentPrefix",
    "delimiter",
    "doubleQuote",
    "encoding",
    "header",
    "headerRowCount",
    "lineTerminators",
    "quoteChar",
    "skipBlankRows",
    "skipColumns",
    "skipInitialSpace",
    "skipRows",
    "trim",
];

const TRANSFORMATION_PROPERTIES: &[&str] = &[
    "@id",
    "@type",
    "scriptFormat",
    "source",
    "targetFormat",
    "titles",
    "url",
];

fn type_is(object: &Map<String, Value>, expected: &str) -> bool {
    object
        .get("@type")
        .and_then(Value::as_str)
        .is_some_and(|value| value == expected || value.ends_with(&format!("#{expected}")))
}

fn check_type(
    object: &Map<String, Value>,
    expected: &str,
    resource: &str,
) -> Result<(), ProjectionError> {
    if let Some(value) = object.get("@type") {
        let Some(actual) = value.as_str() else {
            return Err(ProjectionError::integrity(format!(
                "CSVW {expected} @type must be a string"
            ))
            .at_path(resource));
        };
        if actual.starts_with("_:")
            || !(actual == expected || actual.ends_with(&format!("#{expected}")))
        {
            return Err(ProjectionError::integrity(format!(
                "CSVW {expected} has incompatible @type `{actual}`"
            ))
            .at_path(resource));
        }
    }
    Ok(())
}

fn ensure_only_properties(
    object: &Map<String, Value>,
    allowed: &[&str],
    resource: &str,
    role: &str,
) -> Result<(), ProjectionError> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(ProjectionError::integrity(format!(
            "{role} contains forbidden property `{key}`"
        ))
        .at_path(resource));
    }
    Ok(())
}

fn ensure_unique_table_urls(tables: &[CsvwTable], resource: &str) -> Result<(), ProjectionError> {
    let mut seen = BTreeSet::new();
    for table in tables {
        if !seen.insert(&table.url) {
            return Err(ProjectionError::integrity(format!(
                "duplicate CSVW table URL `{}`",
                table.url
            ))
            .at_path(resource));
        }
    }
    Ok(())
}

fn parse_context_object(
    map: &Map<String, Value>,
    resource: &str,
    base_iri: &mut String,
    language: &mut Option<String>,
    prefixes: &mut BTreeMap<String, String>,
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<(), ProjectionError> {
    for (key, value) in map {
        match key.as_str() {
            "@base" => {
                let Some(reference) = value.as_str() else {
                    return Err(
                        ProjectionError::integrity("CSVW context @base must be a string")
                            .at_path(resource),
                    );
                };
                *base_iri = resolve_iri(base_iri, reference, "CSVW context base")?;
            }
            "@language" => {
                if let Some(tag) = value.as_str().filter(|tag| valid_language_tag(tag)) {
                    *language = Some(tag.to_ascii_lowercase());
                } else {
                    invalid_warning(
                        resource,
                        "$['@context']['@language']",
                        "BCP47 language tag",
                        warnings,
                    );
                    *language = None;
                }
            }
            "@vocab" => {
                let Some(value) = value.as_str() else {
                    return Err(
                        ProjectionError::integrity("CSVW context @vocab must be a string")
                            .at_path(resource),
                    );
                };
                if value != config.vocabulary().csvw_namespace() {
                    return Err(ProjectionError::integrity(
                        "CSVW context @vocab differs from the caller vocabulary",
                    )
                    .at_path(resource));
                }
            }
            key if key.starts_with('@') => {
                return Err(ProjectionError::integrity(format!(
                    "unsupported CSVW context keyword `{key}`"
                ))
                .at_path(resource));
            }
            prefix => {
                let Some(namespace) = value.as_str() else {
                    return Err(ProjectionError::integrity(format!(
                        "CSVW context prefix `{prefix}` must map to a string"
                    ))
                    .at_path(resource));
                };
                let resolved = resolve_iri(base_iri, namespace, "CSVW context namespace")?;
                match config.context().prefixes().get(prefix) {
                    Some(expected) if expected == &resolved => {
                        prefixes.insert(prefix.to_owned(), resolved);
                    }
                    _ => {
                        return Err(ProjectionError::integrity(format!(
                            "CSVW context prefix `{prefix}` was not authorized by caller configuration"
                        ))
                        .at_path(resource));
                    }
                }
            }
        }
    }
    Ok(())
}

fn resolve_iri(base: &str, reference: &str, role: &str) -> Result<String, ProjectionError> {
    let base = purrdf_iri::parse(base)
        .map_err(|error| ProjectionError::configuration(format!("invalid {role} base: {error}")))?;
    let resolved = base
        .resolve(reference)
        .map_err(|error| ProjectionError::syntax(format!("invalid {role}: {error}")))?;
    validate_absolute_iri(resolved.as_str(), role)?;
    Ok(resolved.as_str().to_owned())
}

fn parse_optional_id(
    object: &Map<String, Value>,
    resource: &str,
    context: &DocumentContext,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<Option<String>, ProjectionError> {
    match object.get("@id") {
        None => Ok(None),
        Some(Value::String(_)) => object
            .get("@id")
            .map(|value| parse_id_value(value, resource, &context.base_iri))
            .transpose(),
        Some(_) => {
            invalid_warning(resource, location, "link string", warnings);
            Ok(Some(context.base_iri.clone()))
        }
    }
}

fn parse_id_value(value: &Value, resource: &str, base: &str) -> Result<String, ProjectionError> {
    let value = value
        .as_str()
        .ok_or_else(|| ProjectionError::integrity("CSVW @id must be a string").at_path(resource))?;
    if value.starts_with("_:") {
        return Err(
            ProjectionError::integrity("CSVW @id must not be a blank node").at_path(resource),
        );
    }
    resolve_iri(base, value, "CSVW @id")
}

fn array_property<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<&'a [Value], ProjectionError> {
    let Some(value) = object.get(key) else {
        return Err(
            ProjectionError::integrity(format!("CSVW metadata requires `{key}`")).at_path(resource),
        );
    };
    if let Some(values) = value.as_array() {
        Ok(values)
    } else {
        warnings.push(CsvwWarning::new(
            CsvwWarningKind::InvalidValue,
            resource,
            format!("{location}.{key}"),
            format!("non-array `{key}` was treated as an empty array"),
        ));
        Ok(&[])
    }
}

fn optional_array_property<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> &'a [Value] {
    let Some(value) = object.get(key) else {
        return &[];
    };
    if let Some(values) = value.as_array() {
        values
    } else {
        warnings.push(CsvwWarning::new(
            CsvwWarningKind::InvalidValue,
            resource,
            format!("{location}.{key}"),
            format!("non-array `{key}` was treated as an empty array"),
        ));
        &[]
    }
}

fn empty_schema(inherited: CsvwInheritedProperties, metadata_explicit: bool) -> CsvwSchema {
    CsvwSchema {
        id: None,
        columns: Vec::new(),
        metadata_explicit,
        primary_key: Vec::new(),
        foreign_keys: Vec::new(),
        row_titles: Vec::new(),
        inherited,
        annotations: CsvwAnnotations::new(),
    }
}

fn ensure_unique_column_names(
    columns: &[CsvwColumn],
    resource: &str,
) -> Result<(), ProjectionError> {
    let mut names = BTreeSet::new();
    for column in columns {
        if !names.insert(&column.name) {
            return Err(ProjectionError::integrity(format!(
                "duplicate CSVW column name `{}`",
                column.name
            ))
            .at_path(resource));
        }
    }
    Ok(())
}

fn atomic_bool(
    value: Option<&Value>,
    fallback: bool,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> bool {
    match value {
        None => fallback,
        Some(Value::Bool(value)) => *value,
        Some(_) => {
            invalid_warning(resource, location, "boolean", warnings);
            fallback
        }
    }
}

fn atomic_string(
    value: Option<&Value>,
    fallback: &str,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> String {
    match value {
        None => fallback.to_owned(),
        Some(Value::String(value)) => value.clone(),
        Some(_) => {
            invalid_warning(resource, location, "string", warnings);
            fallback.to_owned()
        }
    }
}

fn optional_string(
    value: Option<&Value>,
    fallback: Option<String>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<String> {
    match value {
        None => fallback,
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) => None,
        Some(_) => {
            invalid_warning(resource, location, "string or null", warnings);
            fallback
        }
    }
}

fn template_property(
    value: Option<&Value>,
    fallback: Option<String>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<String> {
    match value {
        None => fallback,
        Some(Value::Null) => None,
        Some(Value::String(template)) if valid_uri_template(template) => Some(template.clone()),
        Some(_) => {
            invalid_warning(resource, location, "URI template", warnings);
            Some(String::new())
        }
    }
}

fn language_property(
    value: Option<&Value>,
    fallback: Option<String>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<String> {
    match value {
        None => fallback,
        Some(Value::String(value)) if valid_language_tag(value) => Some(value.to_ascii_lowercase()),
        Some(Value::Null) => None,
        Some(_) => {
            invalid_warning(resource, location, "BCP47 language tag", warnings);
            fallback
        }
    }
}

fn string_or_array(
    value: Option<&Value>,
    fallback: &[String],
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Vec<String> {
    match value {
        None => fallback.to_vec(),
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values))
            if values.iter().all(|value| matches!(value, Value::String(_))) =>
        {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        }
        Some(_) => {
            invalid_warning(resource, location, "string or string array", warnings);
            fallback.to_vec()
        }
    }
}

fn nonnegative_integer(
    value: Option<&Value>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<usize> {
    let value = value?;
    if let Some(value) = value.as_u64().and_then(|value| usize::try_from(value).ok()) {
        Some(value)
    } else {
        invalid_warning(resource, location, "non-negative integer", warnings);
        None
    }
}

fn single_character(
    value: Option<&Value>,
    fallback: char,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> char {
    let Some(value) = value else {
        return fallback;
    };
    if let Some(value) = value.as_str()
        && let Some(character) = exactly_one_character(value)
    {
        return character;
    }
    invalid_warning(resource, location, "single character", warnings);
    fallback
}

fn optional_character(
    value: Option<&Value>,
    fallback: Option<char>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<char> {
    match value {
        None => fallback,
        Some(Value::Null) => None,
        Some(Value::String(value)) => exactly_one_character(value).or_else(|| {
            invalid_warning(resource, location, "single character or null", warnings);
            fallback
        }),
        Some(_) => {
            invalid_warning(resource, location, "single character or null", warnings);
            fallback
        }
    }
}

fn exactly_one_character(value: &str) -> Option<char> {
    let mut characters = value.chars();
    let first = characters.next()?;
    characters.next().is_none().then_some(first)
}

fn parse_trim(
    value: Option<&Value>,
    fallback: CsvwTrim,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> CsvwTrim {
    match value {
        None => fallback,
        Some(Value::Bool(false)) => CsvwTrim::None,
        Some(Value::Bool(true)) => CsvwTrim::Both,
        Some(Value::String(value)) if value == "start" => CsvwTrim::Start,
        Some(Value::String(value)) if value == "end" => CsvwTrim::End,
        Some(_) => {
            invalid_warning(resource, location, "trim policy", warnings);
            fallback
        }
    }
}

fn parse_text_direction(
    value: Option<&Value>,
    fallback: Option<CsvwTextDirection>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<CsvwTextDirection> {
    match value {
        None => fallback,
        Some(Value::String(value)) if value == "auto" => Some(CsvwTextDirection::Auto),
        Some(Value::String(value)) if value == "ltr" => Some(CsvwTextDirection::Ltr),
        Some(Value::String(value)) if value == "rtl" => Some(CsvwTextDirection::Rtl),
        Some(Value::String(value)) if value == "inherit" => Some(CsvwTextDirection::Inherit),
        Some(Value::Null) => None,
        Some(_) => {
            invalid_warning(resource, location, "CSVW text direction", warnings);
            fallback
        }
    }
}

fn parse_table_direction(
    value: Option<&Value>,
    fallback: CsvwTableDirection,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> CsvwTableDirection {
    match value {
        None => fallback,
        Some(Value::String(value)) if value == "auto" => CsvwTableDirection::Auto,
        Some(Value::String(value)) if value == "ltr" => CsvwTableDirection::Ltr,
        Some(Value::String(value)) if value == "rtl" => CsvwTableDirection::Rtl,
        Some(_) => {
            invalid_warning(resource, location, "CSVW table direction", warnings);
            fallback
        }
    }
}

fn invalid_warning(
    resource: &str,
    location: &str,
    expected: &str,
    warnings: &mut Vec<CsvwWarning>,
) {
    warnings.push(CsvwWarning::new(
        CsvwWarningKind::InvalidValue,
        resource,
        location,
        format!("invalid value ignored; expected {expected}"),
    ));
}

fn natural_language_property(
    value: Option<&Value>,
    default_language: Option<&str>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> CsvwNaturalLanguage {
    let mut result = CsvwNaturalLanguage::new();
    let default_key = default_language.unwrap_or("und").to_ascii_lowercase();
    match value {
        None => {}
        Some(Value::String(value)) => {
            result.insert(default_key, vec![value.clone()]);
        }
        Some(Value::Array(values)) => {
            let strings = values
                .iter()
                .filter_map(|value| {
                    if let Some(value) = value.as_str() {
                        Some(value.to_owned())
                    } else {
                        invalid_warning(resource, location, "string title", warnings);
                        None
                    }
                })
                .collect::<Vec<_>>();
            if !strings.is_empty() {
                result.insert(default_key, strings);
            }
        }
        Some(Value::Object(languages)) => {
            for (language, values) in languages {
                if language != "und" && !valid_language_tag(language) {
                    invalid_warning(resource, location, "language-map title", warnings);
                    continue;
                }
                let strings = match values {
                    Value::String(value) => vec![value.clone()],
                    Value::Array(values) => values
                        .iter()
                        .filter_map(|value| {
                            if let Some(value) = value.as_str() {
                                Some(value.to_owned())
                            } else {
                                invalid_warning(resource, location, "string title", warnings);
                                None
                            }
                        })
                        .collect(),
                    _ => {
                        invalid_warning(resource, location, "string title", warnings);
                        Vec::new()
                    }
                };
                if !strings.is_empty() {
                    result.insert(language.to_ascii_lowercase(), strings);
                }
            }
        }
        Some(_) => invalid_warning(resource, location, "natural-language value", warnings),
    }
    result
}

fn name_from_titles(
    titles: &CsvwNaturalLanguage,
    default_language: Option<&str>,
    number: usize,
) -> String {
    let title = default_language
        .and_then(|language| titles.get(&language.to_ascii_lowercase()))
        .or_else(|| titles.get("und"))
        .and_then(|values| values.first());
    title.map_or_else(
        || format!("_col.{}", number + 1),
        |value| percent_encode_variable(value),
    )
}

fn valid_column_name(value: &str) -> bool {
    if value.is_empty() || value.starts_with('_') {
        return false;
    }
    value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b'_' | b'.' | b'-' | b'%' | b'~')
            || byte >= 0x80
    })
}

fn percent_encode_variable(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    if output.starts_with('_') {
        output.replace_range(..1, "%5F");
    }
    output
}

fn valid_uri_template(value: &str) -> bool {
    let mut depth = 0usize;
    for character in value.chars() {
        match character {
            '{' if depth == 0 => depth = 1,
            '}' if depth == 1 => depth = 0,
            '{' | '}' => return false,
            _ => {}
        }
    }
    depth == 0
}

fn column_reference_property(
    value: Option<&Value>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(Value::String(value)) if valid_column_name(value) => vec![value.clone()],
        Some(Value::Array(values)) => {
            let mut result = Vec::with_capacity(values.len());
            let mut seen = BTreeSet::new();
            for value in values {
                let Some(value) = value.as_str() else {
                    invalid_warning(resource, location, "column reference", warnings);
                    return Vec::new();
                };
                if !valid_column_name(value) || !seen.insert(value) {
                    invalid_warning(resource, location, "unique column reference", warnings);
                    return Vec::new();
                }
                result.push(value.to_owned());
            }
            result
        }
        Some(_) => {
            invalid_warning(resource, location, "column reference", warnings);
            Vec::new()
        }
    }
}

fn link_property(
    value: Option<&Value>,
    base: &str,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<Option<String>, ProjectionError> {
    match value {
        None => Ok(None),
        Some(Value::String(value)) => resolve_iri(base, value, "CSVW link").map(Some),
        Some(_) => {
            invalid_warning(resource, location, "link string", warnings);
            Ok(None)
        }
    }
}

fn valid_language_tag(value: &str) -> bool {
    LanguageTag::parse(value).is_ok()
}

fn expand_datatype(
    name: &str,
    context: &DocumentContext,
    config: &CsvwConfig,
    allow_absolute: bool,
) -> Option<String> {
    match name {
        "html" => return Some(config.vocabulary().rdf("HTML")),
        "json" => return Some(config.vocabulary().rdf("JSON")),
        "xml" => return Some(config.vocabulary().rdf("XMLLiteral")),
        _ => {}
    }
    let local = match name {
        "any" => "anyType",
        "anyAtomicType" => "anyAtomicType",
        "anyURI" | "anyUri" => "anyURI",
        "base64Binary" => "base64Binary",
        "boolean" => "boolean",
        "byte" => "byte",
        "date" => "date",
        "dateTime" | "datetime" => "dateTime",
        "dateTimeStamp" => "dateTimeStamp",
        "dayTimeDuration" => "dayTimeDuration",
        "decimal" => "decimal",
        "double" | "number" => "double",
        "duration" => "duration",
        "float" => "float",
        "gDay" => "gDay",
        "gMonth" => "gMonth",
        "gMonthDay" => "gMonthDay",
        "gYear" => "gYear",
        "gYearMonth" => "gYearMonth",
        "hexBinary" => "hexBinary",
        "int" => "int",
        "integer" => "integer",
        "language" => "language",
        "long" => "long",
        "Name" => "Name",
        "NCName" => "NCName",
        "negativeInteger" => "negativeInteger",
        "NMTOKEN" => "NMTOKEN",
        "nonNegativeInteger" => "nonNegativeInteger",
        "nonPositiveInteger" => "nonPositiveInteger",
        "normalizedString" => "normalizedString",
        "positiveInteger" => "positiveInteger",
        "QName" => "QName",
        "short" => "short",
        "string" => "string",
        "time" => "time",
        "token" => "token",
        "unsignedByte" => "unsignedByte",
        "unsignedInt" => "unsignedInt",
        "unsignedLong" => "unsignedLong",
        "unsignedShort" => "unsignedShort",
        "yearMonthDuration" => "yearMonthDuration",
        _ => {
            if allow_absolute {
                return context.expansion.expand_iri(name).ok();
            }
            return None;
        }
    };
    Some(config.vocabulary().xsd(local))
}

fn builtin_datatype_iri(iri: &str, config: &CsvwConfig) -> bool {
    iri.strip_prefix(config.vocabulary().xsd_namespace())
        .is_some_and(|local| {
            matches!(
                local,
                "anyType"
                    | "anyAtomicType"
                    | "anyURI"
                    | "base64Binary"
                    | "boolean"
                    | "byte"
                    | "date"
                    | "dateTime"
                    | "dateTimeStamp"
                    | "dayTimeDuration"
                    | "decimal"
                    | "double"
                    | "duration"
                    | "float"
                    | "gDay"
                    | "gMonth"
                    | "gMonthDay"
                    | "gYear"
                    | "gYearMonth"
                    | "hexBinary"
                    | "int"
                    | "integer"
                    | "language"
                    | "long"
                    | "Name"
                    | "NCName"
                    | "negativeInteger"
                    | "NMTOKEN"
                    | "nonNegativeInteger"
                    | "nonPositiveInteger"
                    | "normalizedString"
                    | "positiveInteger"
                    | "QName"
                    | "short"
                    | "string"
                    | "time"
                    | "token"
                    | "unsignedByte"
                    | "unsignedInt"
                    | "unsignedLong"
                    | "unsignedShort"
                    | "yearMonthDuration"
            )
        })
        || matches!(
            iri.strip_prefix(config.vocabulary().rdf_namespace()),
            Some("HTML" | "JSON" | "XMLLiteral")
        )
}

fn parse_datatype_format(
    value: Option<&Value>,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<CsvwDatatypeFormat> {
    match value {
        None => None,
        Some(Value::String(value)) => Some(CsvwDatatypeFormat::Pattern(value.clone())),
        Some(Value::Object(object)) => {
            let pattern = object
                .get("pattern")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let decimal_char = match object.get("decimalChar") {
                None => '.',
                Some(Value::String(value)) => match exactly_one_character(value) {
                    Some(character) => character,
                    None => {
                        invalid_warning(resource, location, "decimal character", warnings);
                        return None;
                    }
                },
                Some(_) => {
                    invalid_warning(resource, location, "decimal character", warnings);
                    return None;
                }
            };
            let group_char = match object.get("groupChar") {
                None | Some(Value::Null) => None,
                Some(Value::String(value)) => match exactly_one_character(value) {
                    Some(character) => Some(character),
                    None => {
                        invalid_warning(resource, location, "group character", warnings);
                        return None;
                    }
                },
                Some(_) => {
                    invalid_warning(resource, location, "group character", warnings);
                    return None;
                }
            };
            if group_char == Some(decimal_char) {
                invalid_warning(resource, location, "distinct numeric separators", warnings);
                return None;
            }
            Some(CsvwDatatypeFormat::Numeric(CsvwNumericFormat {
                pattern,
                decimal_char,
                group_char,
            }))
        }
        Some(_) => {
            invalid_warning(resource, location, "datatype format", warnings);
            None
        }
    }
}

fn validate_datatype_format(
    base: &str,
    format: Option<CsvwDatatypeFormat>,
    config: &CsvwConfig,
    resource: &str,
    location: &str,
    warnings: &mut Vec<CsvwWarning>,
) -> Option<CsvwDatatypeFormat> {
    let local = base.strip_prefix(config.vocabulary().xsd_namespace());
    match (local, format) {
        (Some(local), Some(CsvwDatatypeFormat::Pattern(pattern)))
            if numeric_datatype_name(local) =>
        {
            if valid_numeric_pattern(&pattern) {
                Some(CsvwDatatypeFormat::Pattern(pattern))
            } else {
                invalid_warning(resource, location, "numeric pattern", warnings);
                None
            }
        }
        (Some(local), Some(CsvwDatatypeFormat::Numeric(mut numeric)))
            if numeric_datatype_name(local) =>
        {
            if numeric
                .pattern
                .as_deref()
                .is_some_and(|pattern| !valid_numeric_pattern(pattern))
            {
                invalid_warning(resource, location, "numeric pattern", warnings);
                numeric.pattern = None;
            }
            Some(CsvwDatatypeFormat::Numeric(numeric))
        }
        (Some("boolean"), Some(CsvwDatatypeFormat::Pattern(pattern))) => {
            if pattern
                .split_once('|')
                .is_some_and(|(truth, falsity)| !truth.is_empty() && !falsity.contains('|'))
            {
                Some(CsvwDatatypeFormat::Pattern(pattern))
            } else {
                invalid_warning(resource, location, "boolean pattern", warnings);
                None
            }
        }
        (Some("boolean"), Some(CsvwDatatypeFormat::Numeric(_))) => {
            invalid_warning(resource, location, "boolean format string", warnings);
            None
        }
        (Some(local), Some(CsvwDatatypeFormat::Pattern(pattern)))
            if !temporal_datatype_name(local) =>
        {
            if Regex::new(&format!("^(?:{pattern})$")).is_ok() {
                Some(CsvwDatatypeFormat::Pattern(pattern))
            } else {
                invalid_warning(resource, location, "regular-expression pattern", warnings);
                None
            }
        }
        (_, format) => format,
    }
}

fn valid_numeric_pattern(pattern: &str) -> bool {
    pattern.matches(';').count() <= 1
        && pattern.contains('0')
        && pattern.chars().all(|character| {
            matches!(
                character,
                '#' | '0' | '.' | ',' | ';' | '%' | '‰' | 'E' | '-' | '+'
            )
        })
}

fn numeric_datatype_name(local: &str) -> bool {
    matches!(
        local,
        "integer"
            | "long"
            | "int"
            | "short"
            | "byte"
            | "unsignedLong"
            | "unsignedInt"
            | "unsignedShort"
            | "unsignedByte"
            | "nonNegativeInteger"
            | "positiveInteger"
            | "nonPositiveInteger"
            | "negativeInteger"
            | "decimal"
            | "float"
            | "double"
    )
}

fn temporal_datatype_name(local: &str) -> bool {
    matches!(local, "date" | "time" | "dateTime" | "dateTimeStamp")
}

fn length_datatype(base: &str, config: &CsvwConfig) -> bool {
    const LOCALS: &[&str] = &[
        "string",
        "normalizedString",
        "token",
        "language",
        "Name",
        "NCName",
        "NMTOKEN",
        "hexBinary",
        "base64Binary",
    ];
    LOCALS
        .iter()
        .any(|local| base == config.vocabulary().xsd(local))
}

fn validate_facet_combinations(
    datatype: &CsvwDatatype,
    resource: &str,
    config: &CsvwConfig,
) -> Result<(), ProjectionError> {
    if datatype.minimum.is_some()
        && (datatype.min_inclusive.is_some() || datatype.min_exclusive.is_some())
    {
        return Err(ProjectionError::integrity(
            "CSVW datatype minimum conflicts with an explicit lower bound",
        )
        .at_path(resource));
    }
    if datatype.maximum.is_some()
        && (datatype.max_inclusive.is_some() || datatype.max_exclusive.is_some())
    {
        return Err(ProjectionError::integrity(
            "CSVW datatype maximum conflicts with an explicit upper bound",
        )
        .at_path(resource));
    }
    if datatype.min_inclusive.is_some() && datatype.min_exclusive.is_some() {
        return Err(
            ProjectionError::integrity("CSVW datatype has two lower-bound facets")
                .at_path(resource),
        );
    }
    if datatype.max_inclusive.is_some() && datatype.max_exclusive.is_some() {
        return Err(
            ProjectionError::integrity("CSVW datatype has two upper-bound facets")
                .at_path(resource),
        );
    }
    let has_value_facet = datatype.minimum.is_some()
        || datatype.maximum.is_some()
        || datatype.min_inclusive.is_some()
        || datatype.max_inclusive.is_some()
        || datatype.min_exclusive.is_some()
        || datatype.max_exclusive.is_some();
    if !has_value_facet {
        return Ok(());
    }
    let xsd = facet_datatype(&datatype.base, config).ok_or_else(|| {
        ProjectionError::integrity(
            "CSVW value facets require a numeric, date/time, or duration datatype",
        )
        .at_path(resource)
    })?;
    let lower = datatype
        .minimum
        .as_ref()
        .or(datatype.min_inclusive.as_ref())
        .map(|value| (value, true))
        .or_else(|| datatype.min_exclusive.as_ref().map(|value| (value, false)));
    let upper = datatype
        .maximum
        .as_ref()
        .or(datatype.max_inclusive.as_ref())
        .map(|value| (value, true))
        .or_else(|| datatype.max_exclusive.as_ref().map(|value| (value, false)));
    if let (Some((lower, lower_inclusive)), Some((upper, upper_inclusive))) = (lower, upper) {
        let lower = parse_facet_value(lower, xsd, resource)?;
        let upper = parse_facet_value(upper, xsd, resource)?;
        let ordering = value_cmp(&upper, &lower).ok_or_else(|| {
            ProjectionError::integrity("CSVW datatype bounds are incomparable").at_path(resource)
        })?;
        if ordering == std::cmp::Ordering::Less
            || (ordering == std::cmp::Ordering::Equal && !(lower_inclusive && upper_inclusive))
        {
            return Err(
                ProjectionError::integrity("CSVW datatype bounds are contradictory")
                    .at_path(resource),
            );
        }
    }
    Ok(())
}

fn facet_datatype(base: &str, config: &CsvwConfig) -> Option<XsdDatatype> {
    Some(
        match base.strip_prefix(config.vocabulary().xsd_namespace())? {
            "integer" => XsdDatatype::Integer,
            "long" => XsdDatatype::Long,
            "int" => XsdDatatype::Int,
            "short" => XsdDatatype::Short,
            "byte" => XsdDatatype::Byte,
            "unsignedLong" => XsdDatatype::UnsignedLong,
            "unsignedInt" => XsdDatatype::UnsignedInt,
            "unsignedShort" => XsdDatatype::UnsignedShort,
            "unsignedByte" => XsdDatatype::UnsignedByte,
            "nonNegativeInteger" => XsdDatatype::NonNegativeInteger,
            "positiveInteger" => XsdDatatype::PositiveInteger,
            "nonPositiveInteger" => XsdDatatype::NonPositiveInteger,
            "negativeInteger" => XsdDatatype::NegativeInteger,
            "decimal" => XsdDatatype::Decimal,
            "float" => XsdDatatype::Float,
            "double" => XsdDatatype::Double,
            "date" => XsdDatatype::Date,
            "time" => XsdDatatype::Time,
            "dateTime" | "dateTimeStamp" => XsdDatatype::DateTime,
            "duration" => XsdDatatype::Duration,
            "dayTimeDuration" => XsdDatatype::DayTimeDuration,
            "yearMonthDuration" => XsdDatatype::YearMonthDuration,
            _ => return None,
        },
    )
}

fn parse_facet_value(
    value: &Value,
    datatype: XsdDatatype,
    resource: &str,
) -> Result<purrdf_xsd::XsdValue, ProjectionError> {
    let lexical = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => {
            return Err(
                ProjectionError::integrity("CSVW datatype facet must be atomic").at_path(resource),
            );
        }
    };
    parse_xsd(&lexical, datatype).map_err(|error| {
        ProjectionError::integrity(format!("invalid CSVW datatype facet: {error}"))
            .at_path(resource)
    })
}

fn expand_annotation_key(key: &str, context: &CsvwContext) -> Result<String, ()> {
    if !key.contains(':') {
        return Err(());
    }
    context.expand_iri(key).map_err(|_| ())
}

fn validate_annotation_value(
    value: &Value,
    resource: &str,
    location: &str,
    context: &DocumentContext,
) -> Result<(), ProjectionError> {
    match value {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_annotation_value(
                    value,
                    resource,
                    &format!("{location}[{index}]"),
                    context,
                )?;
            }
        }
        Value::Object(object) => {
            if object.contains_key("@context") {
                return Err(ProjectionError::integrity(
                    "CSVW common-property values must not introduce a context",
                )
                .at_path(resource));
            }
            if object.contains_key("@list") || object.contains_key("@set") {
                return Err(ProjectionError::integrity(
                    "CSVW common-property values must not use @list or @set objects",
                )
                .at_path(resource));
            }
            if let Some(value) = object.get("@value") {
                if !(value.is_string() || value.is_number() || value.is_boolean()) {
                    return Err(ProjectionError::integrity(
                        "CSVW @value must be a string, number, or boolean",
                    )
                    .at_path(resource));
                }
                let allowed = object
                    .keys()
                    .all(|key| matches!(key.as_str(), "@value" | "@type" | "@language"));
                if !allowed || (object.contains_key("@type") && object.contains_key("@language")) {
                    return Err(ProjectionError::integrity(
                        "CSVW @value object has incompatible properties",
                    )
                    .at_path(resource));
                }
                if let Some(datatype) = object.get("@type") {
                    validate_jsonld_iri(datatype, resource, context, "@type")?;
                }
                if let Some(language) = object.get("@language") {
                    let Some(language) = language.as_str() else {
                        return Err(
                            ProjectionError::integrity("CSVW @language must be a string")
                                .at_path(resource),
                        );
                    };
                    if !valid_language_tag(language) {
                        return Err(ProjectionError::integrity(format!(
                            "invalid CSVW @language `{language}`"
                        ))
                        .at_path(resource));
                    }
                }
                return Ok(());
            }
            if object.contains_key("@language") {
                return Err(ProjectionError::integrity(
                    "CSVW @language may appear only in an @value object",
                )
                .at_path(resource));
            }
            if let Some(id) = object.get("@id") {
                validate_jsonld_iri(id, resource, context, "@id")?;
            }
            if let Some(types) = object.get("@type") {
                match types {
                    Value::Array(values) => {
                        for value in values {
                            validate_jsonld_iri(value, resource, context, "@type")?;
                        }
                    }
                    value => validate_jsonld_iri(value, resource, context, "@type")?,
                }
            }
            for (key, value) in object {
                if key.starts_with('@') {
                    if !matches!(key.as_str(), "@id" | "@type") {
                        return Err(ProjectionError::integrity(format!(
                            "unsupported CSVW JSON-LD keyword `{key}`"
                        ))
                        .at_path(resource));
                    }
                } else {
                    expand_annotation_key(key, &context.expansion).map_err(|()| {
                        ProjectionError::integrity(format!(
                            "CSVW common-property node uses unknown property `{key}`"
                        ))
                        .at_path(resource)
                    })?;
                    validate_annotation_value(
                        value,
                        resource,
                        &format!("{location}.{key}"),
                        context,
                    )?;
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

fn validate_jsonld_iri(
    value: &Value,
    resource: &str,
    context: &DocumentContext,
    role: &str,
) -> Result<(), ProjectionError> {
    let value = value.as_str().ok_or_else(|| {
        ProjectionError::integrity(format!("CSVW {role} must be a string")).at_path(resource)
    })?;
    if value.starts_with("_:") {
        return Err(
            ProjectionError::integrity(format!("CSVW {role} must not be a blank node"))
                .at_path(resource),
        );
    }
    if matches!(
        value,
        "TableGroup" | "Table" | "Schema" | "Column" | "Dialect" | "Template"
    ) {
        return Ok(());
    }
    if context.expansion.expand_iri(value).is_ok()
        || resolve_iri(&context.base_iri, value, "CSVW JSON-LD IRI").is_ok()
    {
        Ok(())
    } else {
        Err(ProjectionError::integrity(format!("invalid CSVW {role} `{value}`")).at_path(resource))
    }
}

fn normalize_annotation_value(value: &Value, context: &DocumentContext) -> Value {
    match value {
        Value::String(value) if context.language.is_some() => serde_json::json!({
            "@value": value,
            "@language": context.language,
        }),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| normalize_annotation_value(value, context))
                .collect(),
        ),
        Value::Object(object) if !object.contains_key("@value") => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if key.starts_with('@') {
                            value.clone()
                        } else {
                            normalize_annotation_value(value, context)
                        },
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_tags_follow_the_rfc_5646_grammar() {
        for valid in [
            "sl-1996",
            "x-private",
            "i-klingon",
            "zh-cmn-Hans-CN",
            "de-DE-u-co-phonebk",
            "en-a-bbb-x-a-ccc",
        ] {
            assert!(valid_language_tag(valid), "rejected valid tag {valid:?}");
        }

        for invalid in [
            "",
            "x",
            "x-",
            "en-",
            "en--US",
            "en-u",
            "en-US-abc",
            "en-abcdefghi",
        ] {
            assert!(
                !valid_language_tag(invalid),
                "accepted invalid tag {invalid:?}"
            );
        }
    }
}

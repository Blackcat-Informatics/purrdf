// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! CSV dialect parsing, annotated rows, and schema validation.

use std::collections::{BTreeMap, BTreeSet};

use csv::ReaderBuilder;
use purrdf_xsd::{XsdDatatype, parse as parse_xsd, value_cmp};
use regex::Regex;

use super::super::ProjectionError;
use super::config::CsvwConfig;
use super::input::{CsvwInput, CsvwWarning, CsvwWarningKind};
use super::model::{
    CsvwAnnotations, CsvwCell, CsvwColumn, CsvwDatatype, CsvwDatatypeFormat,
    CsvwInheritedProperties, CsvwNaturalLanguage, CsvwRow, CsvwTable, CsvwTableGroup,
    CsvwTextDirection, CsvwTrim, CsvwValue,
};

pub(crate) fn annotate_tables(
    group: &mut CsvwTableGroup,
    input: &CsvwInput,
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<(), ProjectionError> {
    let mut budget = 0usize;
    for table in &mut group.tables {
        let bytes = input.get(&table.url).ok_or_else(|| {
            ProjectionError::package(format!("CSVW table resource `{}` is absent", table.url))
        })?;
        parse_table(table, bytes, config, warnings, &mut budget)?;
    }
    validate_primary_and_foreign_keys(group, warnings)?;
    group.validate()?;
    Ok(())
}

fn parse_table(
    table: &mut CsvwTable,
    bytes: &[u8],
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
    budget: &mut usize,
) -> Result<(), ProjectionError> {
    if !matches!(table.dialect.encoding.as_str(), "utf-8" | "utf8") {
        return Err(ProjectionError::configuration(format!(
            "CSVW encoding `{}` is not available in the portable UTF-8 engine",
            table.dialect.encoding
        ))
        .at_path(&table.url));
    }
    let source = std::str::from_utf8(bytes).map_err(|error| {
        ProjectionError::syntax(format!("CSVW table is not UTF-8: {error}")).at_path(&table.url)
    })?;
    let records = logical_records(source, table.dialect.quote_char, table.dialect.double_quote)?;
    let mut headers: Vec<(usize, Vec<String>)> = Vec::new();
    let mut data = Vec::new();
    let mut eligible_index = 0usize;
    for record in records {
        if record.source_number <= table.dialect.skip_rows {
            continue;
        }
        if let Some(prefix) = &table.dialect.comment_prefix
            && record.text.starts_with(prefix)
        {
            table.comments.push(record.text[prefix.len()..].to_owned());
            continue;
        }
        let values = parse_record(record.text, table)?;
        if table.dialect.skip_blank_rows && values.iter().all(String::is_empty) {
            continue;
        }
        let values = values
            .into_iter()
            .skip(table.dialect.skip_columns)
            .collect::<Vec<_>>();
        if eligible_index < table.dialect.header_row_count {
            headers.push((record.source_number, values));
        } else {
            data.push((record.source_number, values));
        }
        eligible_index += 1;
    }
    reconcile_schema(
        table,
        &headers,
        data.first().map_or(0, |(_, row)| row.len()),
        config,
        warnings,
    )?;
    let source_columns = table
        .schema
        .columns
        .iter()
        .filter(|column| !column.virtual_column)
        .count();
    for (row_index, (source_number, mut fields)) in data.into_iter().enumerate() {
        if fields.len() > source_columns {
            warnings.push(CsvwWarning::new(
                CsvwWarningKind::Validation,
                &table.url,
                format!("row={source_number}"),
                format!(
                    "row has {} fields but schema has {source_columns} source columns",
                    fields.len()
                ),
            ));
            fields.truncate(source_columns);
        }
        fields.resize(source_columns, String::new());
        let mut cells = Vec::with_capacity(table.schema.columns.len());
        let mut source_index = 0usize;
        for column in &table.schema.columns {
            let raw = if column.virtual_column {
                String::new()
            } else {
                let value = fields[source_index].clone();
                source_index += 1;
                value
            };
            cells.push(parse_cell(
                raw,
                column,
                &table.url,
                source_number,
                config,
                warnings,
            ));
        }
        let row_url = row_url(&table.url, source_number)?;
        let titles = row_titles(&table.schema.row_titles, &table.schema.columns, &cells);
        table.rows.push(CsvwRow {
            number: row_index + 1,
            source_number,
            url: row_url,
            titles,
            cells,
        });
        *budget = budget
            .checked_add(1 + table.schema.columns.len())
            .ok_or_else(|| ProjectionError::limit("CSVW record count overflow"))?;
        if *budget > config.max_records() {
            return Err(ProjectionError::limit(format!(
                "CSVW annotated model exceeds the {}-record limit",
                config.max_records()
            ))
            .at_path(&table.url));
        }
    }
    Ok(())
}

struct LogicalRecord<'a> {
    source_number: usize,
    text: &'a str,
}

fn logical_records(
    source: &str,
    quote: Option<char>,
    double_quote: bool,
) -> Result<Vec<LogicalRecord<'_>>, ProjectionError> {
    let mut records = Vec::new();
    let mut in_quotes = false;
    let mut start = 0usize;
    let mut source_number = 1usize;
    let mut record_source = 1usize;
    let mut chars = source.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        if Some(character) == quote {
            if in_quotes && double_quote && chars.peek().is_some_and(|(_, next)| *next == character)
            {
                let _ = chars.next();
            } else {
                in_quotes = !in_quotes;
            }
            continue;
        }
        if character == '\n' {
            source_number += 1;
            if !in_quotes {
                let end = if index > start && source.as_bytes()[index - 1] == b'\r' {
                    index - 1
                } else {
                    index
                };
                records.push(LogicalRecord {
                    source_number: record_source,
                    text: &source[start..end],
                });
                start = index + 1;
                record_source = source_number;
            }
        }
    }
    if in_quotes {
        return Err(ProjectionError::syntax(
            "CSVW table ends inside a quoted field",
        ));
    }
    if start < source.len() || source.is_empty() {
        records.push(LogicalRecord {
            source_number: record_source,
            text: &source[start..],
        });
    }
    Ok(records)
}

fn parse_record(record: &str, table: &CsvwTable) -> Result<Vec<String>, ProjectionError> {
    let delimiter = u8::try_from(u32::from(table.dialect.delimiter))
        .map_err(|_| ProjectionError::configuration("CSVW delimiter must be an ASCII byte"))?;
    let mut builder = ReaderBuilder::new();
    builder
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .double_quote(table.dialect.double_quote)
        .trim(csv::Trim::None);
    if let Some(quote) = table.dialect.quote_char {
        builder.quote(u8::try_from(u32::from(quote)).map_err(|_| {
            ProjectionError::configuration("CSVW quote character must be an ASCII byte")
        })?);
    } else {
        builder.quoting(false);
    }
    let mut reader = builder.from_reader(record.as_bytes());
    let mut records = reader.records();
    let parsed = records
        .next()
        .transpose()
        .map_err(|error| ProjectionError::syntax(format!("invalid CSVW row: {error}")))?
        .unwrap_or_default();
    if records.next().is_some() {
        return Err(ProjectionError::syntax(
            "CSVW logical record decoded as more than one row",
        ));
    }
    Ok(parsed
        .iter()
        .map(|field| normalize_field(field, table.dialect.trim, table.dialect.skip_initial_space))
        .collect())
}

fn normalize_field(value: &str, trim: CsvwTrim, skip_initial_space: bool) -> String {
    let value = if skip_initial_space {
        value.trim_start_matches([' ', '\t'])
    } else {
        value
    };
    match trim {
        CsvwTrim::None => value.to_owned(),
        CsvwTrim::Start => value.trim_start().to_owned(),
        CsvwTrim::End => value.trim_end().to_owned(),
        CsvwTrim::Both => value.trim().to_owned(),
    }
}

fn reconcile_schema(
    table: &mut CsvwTable,
    headers: &[(usize, Vec<String>)],
    first_data_width: usize,
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<(), ProjectionError> {
    let header_width = headers.iter().map(|(_, row)| row.len()).max().unwrap_or(0);
    let source_width = header_width.max(first_data_width);
    let existing_nonvirtual = table
        .schema
        .columns
        .iter()
        .filter(|column| !column.virtual_column)
        .count();
    if existing_nonvirtual == 0 {
        if table.schema.metadata_explicit && source_width != 0 {
            warnings.push(CsvwWarning::new(
                CsvwWarningKind::Validation,
                &table.url,
                "header",
                "explicit metadata schema has no columns compatible with the embedded schema",
            ));
        }
        let virtual_columns = table
            .schema
            .columns
            .iter()
            .filter(|column| column.virtual_column)
            .cloned()
            .collect::<Vec<_>>();
        table.schema.columns.clear();
        for index in 0..source_width {
            let titles = header_titles(headers, index, table.schema.inherited.language.as_deref());
            let name = if table.schema.metadata_explicit {
                format!("_col.{}", index + 1)
            } else {
                titles
                    .values()
                    .next()
                    .and_then(|values| values.first())
                    .map_or_else(
                        || format!("_col.{}", index + 1),
                        |title| percent_encode(title),
                    )
            };
            table.schema.columns.push(default_column(
                index,
                name,
                titles,
                table.schema.inherited.clone(),
            ));
        }
        for mut column in virtual_columns {
            column.number = table.schema.columns.len();
            table.schema.columns.push(column);
        }
    } else {
        if source_width != 0 && source_width != existing_nonvirtual {
            warnings.push(CsvwWarning::new(
                CsvwWarningKind::Validation,
                &table.url,
                "header",
                format!(
                    "embedded table has {source_width} columns but metadata has {existing_nonvirtual}"
                ),
            ));
        }
        if source_width > existing_nonvirtual {
            let insertion = table
                .schema
                .columns
                .iter()
                .position(|column| column.virtual_column)
                .unwrap_or(table.schema.columns.len());
            for index in existing_nonvirtual..source_width {
                let titles =
                    header_titles(headers, index, table.schema.inherited.language.as_deref());
                table.schema.columns.insert(
                    insertion + index - existing_nonvirtual,
                    default_column(
                        index,
                        format!("_col.{}", index + 1),
                        titles,
                        table.schema.inherited.clone(),
                    ),
                );
            }
        }
        let mut source_index = 0usize;
        for column in &mut table.schema.columns {
            if column.virtual_column {
                continue;
            }
            let embedded = header_titles(
                headers,
                source_index,
                table.schema.inherited.language.as_deref(),
            );
            if !embedded.is_empty() {
                if column.titles.is_empty() {
                    if table.schema.metadata_explicit {
                        warnings.push(CsvwWarning::new(
                            CsvwWarningKind::Validation,
                            &table.url,
                            format!("header-column={}", source_index + 1),
                            format!(
                                "metadata column `{}` has no title compatible with the embedded title",
                                column.name
                            ),
                        ));
                    }
                    column.titles = embedded.clone();
                } else if !titles_compatible(&column.titles, &embedded) {
                    warnings.push(CsvwWarning::new(
                        CsvwWarningKind::Validation,
                        &table.url,
                        format!("header-column={}", source_index + 1),
                        format!(
                            "metadata for column `{}` is incompatible with the embedded title",
                            column.name
                        ),
                    ));
                }
            }
            source_index += 1;
        }
    }
    if table.schema.columns.len() > config.max_records() {
        return Err(
            ProjectionError::limit("CSVW schema exceeds the configured record limit")
                .at_path(&table.url),
        );
    }
    let mut names = BTreeSet::new();
    for (index, column) in table.schema.columns.iter_mut().enumerate() {
        column.number = index;
        if !names.insert(column.name.clone()) {
            return Err(ProjectionError::integrity(format!(
                "duplicate CSVW column name `{}`",
                column.name
            ))
            .at_path(&table.url));
        }
    }
    Ok(())
}

fn header_titles(
    headers: &[(usize, Vec<String>)],
    index: usize,
    language: Option<&str>,
) -> CsvwNaturalLanguage {
    let values = headers
        .iter()
        .filter_map(|(_, row)| row.get(index))
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if values.is_empty() {
        CsvwNaturalLanguage::new()
    } else {
        BTreeMap::from([(language.unwrap_or("und").to_ascii_lowercase(), values)])
    }
}

fn titles_compatible(left: &CsvwNaturalLanguage, right: &CsvwNaturalLanguage) -> bool {
    left.iter().any(|(left_language, left_values)| {
        right.iter().any(|(right_language, right_values)| {
            languages_compatible(left_language, right_language)
                && left_values
                    .iter()
                    .any(|left| right_values.iter().any(|right| left == right))
        })
    })
}

fn languages_compatible(left: &str, right: &str) -> bool {
    left == "und"
        || right == "und"
        || left.eq_ignore_ascii_case(right)
        || left
            .get(..right.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(right))
        || right
            .get(..left.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(left))
}

fn default_column(
    number: usize,
    name: String,
    titles: CsvwNaturalLanguage,
    inherited: CsvwInheritedProperties,
) -> CsvwColumn {
    CsvwColumn {
        id: None,
        number,
        name,
        name_explicit: false,
        titles,
        virtual_column: false,
        suppress_output: false,
        inherited,
        annotations: CsvwAnnotations::new(),
    }
}

fn percent_encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

fn row_url(table_url: &str, source_number: usize) -> Result<String, ProjectionError> {
    let base = purrdf_iri::parse(table_url)
        .map_err(|error| ProjectionError::term(format!("invalid CSVW table URL: {error}")))?;
    base.resolve(&format!("#row={source_number}"))
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| ProjectionError::term(format!("invalid CSVW row URL: {error}")))
}

fn row_titles(names: &[String], columns: &[CsvwColumn], cells: &[CsvwCell]) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| {
            let index = columns.iter().position(|column| &column.name == name)?;
            Some(cells[index].string_value.clone())
        })
        .collect()
}

fn parse_cell(
    raw: String,
    column: &CsvwColumn,
    table_url: &str,
    source_number: usize,
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
) -> CsvwCell {
    let inherited = &column.inherited;
    let effective = if raw.is_empty() {
        inherited.default.clone()
    } else {
        raw.clone()
    };
    let location = format!("row={source_number},column={}", column.number + 1);
    let components = if let Some(separator) = &inherited.separator {
        if effective.is_empty() || inherited.nulls.contains(&effective) {
            Vec::new()
        } else {
            effective
                .split(separator)
                .map(|value| {
                    let value = value.trim();
                    if value.is_empty() {
                        inherited.default.as_str()
                    } else {
                        value
                    }
                })
                .filter(|value| !inherited.nulls.iter().any(|null| null == *value))
                .map(str::to_owned)
                .collect()
        }
    } else if inherited.nulls.contains(&effective) {
        Vec::new()
    } else {
        vec![effective]
    };
    let is_null = components.is_empty();
    if is_null && inherited.required {
        warnings.push(CsvwWarning::new(
            CsvwWarningKind::Validation,
            table_url,
            &location,
            format!("required CSVW column `{}` has a null value", column.name),
        ));
    }
    let values = components
        .into_iter()
        .map(|component| {
            parse_component(
                component,
                &inherited.datatype,
                inherited.language.as_deref(),
                inherited.text_direction,
                table_url,
                &location,
                config,
                warnings,
            )
        })
        .collect();
    CsvwCell {
        column: column.number,
        string_value: raw,
        values,
        is_null,
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_component(
    source: String,
    datatype: &CsvwDatatype,
    language: Option<&str>,
    direction: Option<CsvwTextDirection>,
    table_url: &str,
    location: &str,
    config: &CsvwConfig,
    warnings: &mut Vec<CsvwWarning>,
) -> CsvwValue {
    let normalized = normalize_lexical(&source, datatype, config);
    let lexical = match normalized
        .and_then(|lexical| validate_lexical(&lexical, datatype, config).map(|()| lexical))
    {
        Ok(lexical) => lexical,
        Err(message) => {
            warnings.push(CsvwWarning::new(
                CsvwWarningKind::Validation,
                table_url,
                location,
                message,
            ));
            return CsvwValue {
                source: source.clone(),
                lexical: source,
                datatype: config.vocabulary().xsd("string"),
                language: language.map(str::to_ascii_lowercase),
                direction,
            };
        }
    };
    let language_datatype = datatype.base == config.vocabulary().xsd("string");
    CsvwValue {
        source,
        lexical,
        datatype: datatype.id.clone().unwrap_or_else(|| datatype.base.clone()),
        language: language_datatype
            .then(|| language.map(str::to_ascii_lowercase))
            .flatten(),
        direction: language_datatype.then_some(direction).flatten(),
    }
}

fn normalize_lexical(
    source: &str,
    datatype: &CsvwDatatype,
    config: &CsvwConfig,
) -> Result<String, String> {
    let local = datatype
        .base
        .strip_prefix(config.vocabulary().xsd_namespace());
    match (local, &datatype.format) {
        (Some("boolean"), Some(CsvwDatatypeFormat::Pattern(pattern))) => {
            let Some((truth, falsity)) = pattern.split_once('|') else {
                return Err("invalid CSVW boolean format".to_owned());
            };
            if truth.contains('|') || falsity.contains('|') {
                return Err("invalid CSVW boolean format".to_owned());
            }
            if source == truth {
                Ok("true".to_owned())
            } else if source == falsity {
                Ok("false".to_owned())
            } else {
                Err("cell does not match the CSVW boolean format".to_owned())
            }
        }
        (Some(local), Some(CsvwDatatypeFormat::Pattern(pattern))) if temporal_datatype(local) => {
            parse_temporal_pattern(source, pattern, local)
        }
        (Some(local), Some(CsvwDatatypeFormat::Pattern(pattern))) if numeric_datatype(local) => {
            normalize_number(source, Some(pattern), '.', None, local)
        }
        (Some(local), Some(CsvwDatatypeFormat::Numeric(format))) if numeric_datatype(local) => {
            normalize_number(
                source,
                format.pattern.as_deref(),
                format.decimal_char,
                format.group_char,
                local,
            )
        }
        (Some(local), Some(CsvwDatatypeFormat::Pattern(pattern)))
            if !numeric_datatype(local) && !temporal_datatype(local) =>
        {
            let regex = Regex::new(&format!("^(?:{pattern})$"))
                .map_err(|_| "invalid CSVW regular-expression format".to_owned())?;
            if regex.is_match(source) {
                Ok(source.to_owned())
            } else {
                Err("cell does not match the CSVW datatype format".to_owned())
            }
        }
        (_, Some(CsvwDatatypeFormat::Numeric(_))) => {
            Err("numeric CSVW format used with a non-numeric datatype".to_owned())
        }
        (Some("boolean"), None) => match source {
            "true" | "1" => Ok("true".to_owned()),
            "false" | "0" => Ok("false".to_owned()),
            _ => Ok(source.to_owned()),
        },
        _ => Ok(source.to_owned()),
    }
}

fn validate_lexical(
    lexical: &str,
    datatype: &CsvwDatatype,
    config: &CsvwConfig,
) -> Result<(), String> {
    if let Some(xsd) = xsd_datatype(&datatype.base, config) {
        parse_xsd(lexical, xsd)
            .map_err(|error| format!("invalid CSVW {} value: {error}", datatype.base))?;
    } else if datatype.base == config.vocabulary().xsd("dateTimeStamp") {
        parse_xsd(lexical, XsdDatatype::DateTime)
            .map_err(|error| format!("invalid CSVW dateTimeStamp value: {error}"))?;
        if !has_timezone(lexical) {
            return Err("CSVW dateTimeStamp requires a timezone".to_owned());
        }
    } else if datatype.base == config.vocabulary().xsd("anyURI") {
        purrdf_iri::parse(lexical)
            .map_err(|error| format!("invalid CSVW anyURI value: {error}"))?;
    } else if let Some(local) = datatype
        .base
        .strip_prefix(config.vocabulary().xsd_namespace())
    {
        validate_derived_string(lexical, local)?;
    }
    let length = value_length(lexical, &datatype.base, config)?;
    if datatype.length.is_some_and(|expected| length != expected) {
        return Err(format!(
            "CSVW value length {length} does not equal the required length"
        ));
    }
    if datatype.min_length.is_some_and(|minimum| length < minimum) {
        return Err(format!("CSVW value length {length} is below minLength"));
    }
    if datatype.max_length.is_some_and(|maximum| length > maximum) {
        return Err(format!("CSVW value length {length} exceeds maxLength"));
    }
    validate_value_facets(lexical, datatype, config)
}

fn value_length(lexical: &str, base: &str, config: &CsvwConfig) -> Result<usize, String> {
    match base.strip_prefix(config.vocabulary().xsd_namespace()) {
        Some("hexBinary") => Ok(lexical.len() / 2),
        Some("base64Binary") => {
            let compact = lexical
                .bytes()
                .filter(|byte| !byte.is_ascii_whitespace())
                .collect::<Vec<_>>();
            let padding = compact
                .iter()
                .rev()
                .take_while(|byte| **byte == b'=')
                .count();
            compact
                .len()
                .checked_div(4)
                .and_then(|groups| groups.checked_mul(3))
                .and_then(|bytes| bytes.checked_sub(padding))
                .ok_or_else(|| "invalid CSVW base64Binary length".to_owned())
        }
        _ => Ok(lexical.chars().count()),
    }
}

fn xsd_datatype(base: &str, config: &CsvwConfig) -> Option<XsdDatatype> {
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
            "boolean" => XsdDatatype::Boolean,
            "string" => XsdDatatype::String,
            "date" => XsdDatatype::Date,
            "time" => XsdDatatype::Time,
            "dateTime" => XsdDatatype::DateTime,
            "duration" => XsdDatatype::Duration,
            "dayTimeDuration" => XsdDatatype::DayTimeDuration,
            "yearMonthDuration" => XsdDatatype::YearMonthDuration,
            "gYear" => XsdDatatype::GYear,
            "gMonth" => XsdDatatype::GMonth,
            "gDay" => XsdDatatype::GDay,
            "gYearMonth" => XsdDatatype::GYearMonth,
            "gMonthDay" => XsdDatatype::GMonthDay,
            "hexBinary" => XsdDatatype::HexBinary,
            "base64Binary" => XsdDatatype::Base64Binary,
            _ => return None,
        },
    )
}

fn validate_derived_string(value: &str, local: &str) -> Result<(), String> {
    match local {
        "normalizedString" if value.contains(['\r', '\n', '\t']) => {
            Err("normalizedString contains forbidden whitespace".to_owned())
        }
        "token" if value.trim() != value || value.contains("  ") => {
            Err("token contains uncollapsed whitespace".to_owned())
        }
        "language" if !valid_language(value) => Err("invalid language value".to_owned()),
        "Name" if !valid_xml_name(value, true) => Err("invalid XML Name value".to_owned()),
        "NCName" if !valid_xml_name(value, false) => Err("invalid XML NCName value".to_owned()),
        "NMTOKEN" if value.is_empty() || value.chars().any(char::is_whitespace) => {
            Err("invalid XML NMTOKEN value".to_owned())
        }
        _ => Ok(()),
    }
}

fn valid_language(value: &str) -> bool {
    let mut parts = value.split('-');
    parts.next().is_some_and(|part| {
        part.len() >= 2 && part.len() <= 8 && part.bytes().all(|byte| byte.is_ascii_alphabetic())
    }) && parts.all(|part| {
        !part.is_empty() && part.len() <= 8 && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}

fn valid_xml_name(value: &str, colon: bool) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|character| {
        character == '_' || character.is_alphabetic() || (colon && character == ':')
    }) && chars.all(|character| {
        character == '_'
            || character == '-'
            || character == '.'
            || character.is_alphanumeric()
            || (colon && character == ':')
    })
}

fn numeric_datatype(local: &str) -> bool {
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

fn temporal_datatype(local: &str) -> bool {
    matches!(local, "date" | "time" | "dateTime" | "dateTimeStamp")
}

fn normalize_number(
    source: &str,
    pattern: Option<&str>,
    decimal_char: char,
    group_char: Option<char>,
    local: &str,
) -> Result<String, String> {
    if matches!(source, "NaN" | "INF" | "-INF") {
        return if matches!(local, "float" | "double") {
            Ok(source.to_owned())
        } else {
            Err("special numeric value is not valid for this CSVW datatype".to_owned())
        };
    }
    let effective_group =
        group_char.or_else(|| pattern.filter(|pattern| pattern.contains(',')).map(|_| ','));
    if let Some(pattern) = pattern {
        validate_number_pattern(pattern)?;
        let standardized = source
            .chars()
            .map(|character| {
                if Some(character) == effective_group {
                    ','
                } else if character == decimal_char {
                    '.'
                } else {
                    character
                }
            })
            .collect::<String>();
        validate_number_against_pattern(&standardized, pattern)?;
    }
    if let Some(group) = effective_group {
        let doubled = format!("{group}{group}");
        if source.contains(&doubled) {
            return Err("CSVW number contains consecutive group characters".to_owned());
        }
    }
    let mut normalized = String::with_capacity(source.len());
    for character in source.chars() {
        if Some(character) == effective_group {
            continue;
        }
        if character == decimal_char {
            normalized.push('.');
        } else {
            normalized.push(character);
        }
    }
    let scale = if normalized.contains('%') {
        normalized = normalized.replace('%', "");
        2
    } else if normalized.contains('‰') {
        normalized = normalized.replace('‰', "");
        3
    } else {
        0
    };
    if scale != 0 {
        normalized = shift_decimal_left(&normalized, scale)?;
    }
    if integer_datatype(local) && normalized.contains(['.', 'e', 'E']) {
        return Err("CSVW integer contains a decimal point or exponent".to_owned());
    }
    if local == "decimal"
        && (normalized.contains(['e', 'E'])
            || matches!(normalized.as_str(), "NaN" | "INF" | "-INF"))
    {
        return Err("CSVW decimal contains an exponent or special value".to_owned());
    }
    if matches!(local, "float" | "double") {
        normalized = normalized.replace('E', "e");
    }
    Ok(normalized)
}

fn integer_datatype(local: &str) -> bool {
    numeric_datatype(local) && !matches!(local, "decimal" | "float" | "double")
}

fn validate_number_pattern(pattern: &str) -> Result<(), String> {
    if !pattern.contains('0')
        || pattern.matches(';').count() > 1
        || pattern.chars().any(|character| {
            !matches!(
                character,
                '0' | '#' | '.' | ',' | ';' | 'E' | '+' | '-' | '%' | '‰'
            )
        })
    {
        return Err("invalid CSVW numeric pattern".to_owned());
    }
    Ok(())
}

fn validate_number_against_pattern(value: &str, pattern: &str) -> Result<(), String> {
    let positive = pattern
        .split_once(';')
        .map_or(pattern, |(positive, _)| positive);
    let integral_pattern = positive
        .split_once('.')
        .map_or(positive, |(integral, _)| integral)
        .split_once('E')
        .map_or_else(
            || {
                positive
                    .split_once('.')
                    .map_or(positive, |(integral, _)| integral)
            },
            |(integral, _)| integral,
        );
    let pattern_groups = integral_pattern.split(',').collect::<Vec<_>>();
    let primary_grouping = pattern_groups
        .get(1..)
        .and_then(|groups| groups.last())
        .map_or(0, |group| placeholder_count(group));
    let secondary_grouping = if pattern_groups.len() > 2 {
        placeholder_count(pattern_groups[1])
    } else {
        primary_grouping
    };
    let min_integral = integral_pattern
        .chars()
        .rev()
        .take_while(|character| *character == '0')
        .count();
    let exponent_digits = positive.split_once('E').map_or(0, |(_, exponent)| {
        exponent
            .chars()
            .skip_while(|character| *character == ',')
            .take_while(|character| matches!(character, '0' | '#'))
            .count()
    });
    let decimal_pattern = positive.split_once('.').map_or("", |(_, decimal)| {
        decimal
            .split_once('E')
            .map_or(decimal, |(decimal, _)| decimal)
    });
    let decimal_digits = placeholder_count(decimal_pattern);
    let significant_decimal_digits = decimal_pattern
        .chars()
        .take_while(|character| !matches!(character, 'E' | '#'))
        .filter(|character| *character == '0')
        .count();

    let (integral, decimal) = value.split_once('.').unwrap_or((value, ""));
    let decimal = decimal
        .split_once(['e', 'E'])
        .map_or(decimal, |(decimal, _)| decimal);
    let groups = integral.split(',').collect::<Vec<_>>();
    let significant = significant_integral_digits(&groups);
    if (min_integral != 0 && significant < min_integral)
        || (primary_grouping != 0
            && digit_count(groups.last().copied().unwrap_or_default()) > primary_grouping)
        || (primary_grouping != 0
            && groups.len() > 1
            && digit_count(groups.last().copied().unwrap_or_default()) < primary_grouping)
        || (!decimal.is_empty() && digit_count(decimal) > decimal_digits)
        || (significant_decimal_digits != 0
            && (decimal.is_empty() || digit_count(decimal) < significant_decimal_digits))
    {
        return Err("CSVW number does not match the declared pattern".to_owned());
    }
    if exponent_digits != 0
        && value.contains(['e', 'E'])
        && value
            .split(['e', 'E'])
            .next_back()
            .is_some_and(|exponent| digit_count(exponent) > exponent_digits)
    {
        return Err("CSVW number exponent does not match the declared pattern".to_owned());
    }
    if secondary_grouping != 0 && groups.len() > 1 {
        for (index, group) in groups[..groups.len() - 1].iter().enumerate() {
            let digits = digit_count(group);
            if (index == 0 && digits > secondary_grouping)
                || (index != 0 && digits != secondary_grouping)
            {
                return Err("CSVW number grouping does not match the declared pattern".to_owned());
            }
        }
    }
    Ok(())
}

fn placeholder_count(value: &str) -> usize {
    value
        .chars()
        .filter(|character| matches!(character, '0' | '#'))
        .count()
}

fn digit_count(value: &str) -> usize {
    value.bytes().filter(u8::is_ascii_digit).count()
}

fn significant_integral_digits(groups: &[&str]) -> usize {
    let mut leading_zero = false;
    let mut skipping = true;
    let mut significant = 0usize;
    for character in groups.iter().flat_map(|group| group.chars()) {
        if matches!(character, '+' | '-' | '%' | '‰') {
            continue;
        }
        if character == '0' && skipping {
            leading_zero = true;
            continue;
        }
        if character != '0' {
            skipping = false;
        }
        significant += 1;
    }
    if significant == 0 && leading_zero {
        1
    } else {
        significant
    }
}

fn shift_decimal_left(value: &str, places: usize) -> Result<String, String> {
    if value.contains(['e', 'E']) {
        let parsed = value
            .parse::<f64>()
            .map_err(|_| "invalid CSVW percentage number".to_owned())?;
        return Ok(format!(
            "{}",
            parsed / 10_f64.powi(i32::try_from(places).unwrap_or(0))
        ));
    }
    let (sign, value) = value.strip_prefix('-').map_or_else(
        || {
            value
                .strip_prefix('+')
                .map_or(("", value), |value| ("", value))
        },
        |value| ("-", value),
    );
    if !value
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
        || value.matches('.').count() > 1
    {
        return Err("invalid CSVW percentage number".to_owned());
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let mut digits = format!("{whole}{fraction}");
    let decimal_position = isize::try_from(whole.len()).unwrap_or(isize::MAX)
        - isize::try_from(places).unwrap_or(isize::MAX);
    let output = if decimal_position <= 0 {
        let zeros = usize::try_from(-decimal_position).unwrap_or(0);
        format!("0.{}{digits}", "0".repeat(zeros))
    } else {
        let position = usize::try_from(decimal_position).unwrap_or(digits.len());
        if position >= digits.len() {
            digits.push_str(&"0".repeat(position - digits.len()));
        } else {
            digits.insert(position, '.');
        }
        digits
    };
    Ok(format!("{sign}{output}"))
}

fn validate_value_facets(
    lexical: &str,
    datatype: &CsvwDatatype,
    config: &CsvwConfig,
) -> Result<(), String> {
    let Some(xsd) = xsd_datatype(&datatype.base, config) else {
        return Ok(());
    };
    let value = parse_xsd(lexical, xsd)
        .map_err(|error| format!("invalid CSVW value for facets: {error}"))?;
    let lower = datatype
        .minimum
        .as_ref()
        .or(datatype.min_inclusive.as_ref());
    if let Some(bound) = lower {
        let bound = parse_bound(bound, xsd)?;
        if !matches!(
            value_cmp(&value, &bound),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ) {
            return Err("CSVW value is below its inclusive lower bound".to_owned());
        }
    }
    if let Some(bound) = &datatype.min_exclusive {
        let bound = parse_bound(bound, xsd)?;
        if value_cmp(&value, &bound) != Some(std::cmp::Ordering::Greater) {
            return Err("CSVW value is not above its exclusive lower bound".to_owned());
        }
    }
    let upper = datatype
        .maximum
        .as_ref()
        .or(datatype.max_inclusive.as_ref());
    if let Some(bound) = upper {
        let bound = parse_bound(bound, xsd)?;
        if !matches!(
            value_cmp(&value, &bound),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ) {
            return Err("CSVW value exceeds its inclusive upper bound".to_owned());
        }
    }
    if let Some(bound) = &datatype.max_exclusive {
        let bound = parse_bound(bound, xsd)?;
        if value_cmp(&value, &bound) != Some(std::cmp::Ordering::Less) {
            return Err("CSVW value is not below its exclusive upper bound".to_owned());
        }
    }
    Ok(())
}

fn parse_bound(
    value: &serde_json::Value,
    datatype: XsdDatatype,
) -> Result<purrdf_xsd::XsdValue, String> {
    let lexical = match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        _ => return Err("CSVW datatype facet is not atomic".to_owned()),
    };
    parse_xsd(&lexical, datatype).map_err(|error| format!("invalid CSVW datatype facet: {error}"))
}

fn has_timezone(value: &str) -> bool {
    value.ends_with('Z')
        || value
            .rfind(['+', '-'])
            .is_some_and(|index| index > value.find('T').unwrap_or(value.len()))
}

fn parse_temporal_pattern(source: &str, pattern: &str, local: &str) -> Result<String, String> {
    let mut input = 0usize;
    let mut pattern_index = 0usize;
    let mut year = None;
    let mut month = None;
    let mut day = None;
    let mut hour = None;
    let mut minute = None;
    let mut second = None;
    let mut fraction = None;
    let mut timezone = None;
    while pattern_index < pattern.len() {
        let character = pattern[pattern_index..]
            .chars()
            .next()
            .ok_or_else(|| "invalid CSVW temporal pattern".to_owned())?;
        let width = pattern[pattern_index..]
            .chars()
            .take_while(|candidate| *candidate == character)
            .count();
        match character {
            'y' => {
                if width != 4 {
                    return Err("CSVW temporal pattern requires four-digit years".to_owned());
                }
                year = Some(read_digits(source, &mut input, 4, 4)?);
            }
            'M' => month = Some(read_digits(source, &mut input, width.min(2), 2)?),
            'd' => day = Some(read_digits(source, &mut input, width.min(2), 2)?),
            'H' => hour = Some(read_digits(source, &mut input, width.min(2), 2)?),
            'm' => minute = Some(read_digits(source, &mut input, width.min(2), 2)?),
            's' => second = Some(read_digits(source, &mut input, width.min(2), 2)?),
            'S' => {
                let start = input;
                let _ = read_digits(source, &mut input, 1, width)?;
                fraction = Some(source[start..input].to_owned());
            }
            'X' => timezone = Some(read_timezone(source, &mut input, width)?),
            'T' => {
                if source.as_bytes().get(input) != Some(&b'T') {
                    return Err("cell does not match the CSVW temporal pattern".to_owned());
                }
                input += 1;
            }
            _ if character.is_ascii_alphabetic() => {
                return Err(format!("unsupported CSVW temporal field `{character}`"));
            }
            _ => {
                let literal_len = character.len_utf8();
                if source.get(input..input + literal_len)
                    != Some(&pattern[pattern_index..pattern_index + literal_len])
                {
                    return Err("cell does not match the CSVW temporal pattern".to_owned());
                }
                input += literal_len;
            }
        }
        pattern_index += character.len_utf8() * width;
    }
    if input != source.len() {
        return Err("cell has trailing content after the CSVW temporal pattern".to_owned());
    }
    let timezone = timezone.unwrap_or_default();
    let fraction = fraction.map_or_else(String::new, |value| format!(".{value}"));
    match local {
        "date" => Ok(format!(
            "{:04}-{:02}-{:02}{timezone}",
            year.ok_or_else(|| "CSVW date format lacks a year".to_owned())?,
            month.ok_or_else(|| "CSVW date format lacks a month".to_owned())?,
            day.ok_or_else(|| "CSVW date format lacks a day".to_owned())?,
        )),
        "time" => Ok(format!(
            "{:02}:{:02}:{:02}{fraction}{timezone}",
            hour.ok_or_else(|| "CSVW time format lacks an hour".to_owned())?,
            minute.ok_or_else(|| "CSVW time format lacks a minute".to_owned())?,
            second.unwrap_or(0),
        )),
        "dateTime" | "dateTimeStamp" => Ok(format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{fraction}{timezone}",
            year.ok_or_else(|| "CSVW dateTime format lacks a year".to_owned())?,
            month.ok_or_else(|| "CSVW dateTime format lacks a month".to_owned())?,
            day.ok_or_else(|| "CSVW dateTime format lacks a day".to_owned())?,
            hour.ok_or_else(|| "CSVW dateTime format lacks an hour".to_owned())?,
            minute.ok_or_else(|| "CSVW dateTime format lacks a minute".to_owned())?,
            second.unwrap_or(0),
        )),
        _ => Err("unsupported CSVW temporal datatype".to_owned()),
    }
}

fn read_digits(
    source: &str,
    cursor: &mut usize,
    minimum: usize,
    maximum: usize,
) -> Result<u32, String> {
    let bytes = source.as_bytes();
    let start = *cursor;
    while *cursor < bytes.len() && *cursor - start < maximum && bytes[*cursor].is_ascii_digit() {
        *cursor += 1;
    }
    if *cursor - start < minimum {
        return Err("cell does not match the CSVW temporal pattern".to_owned());
    }
    source[start..*cursor]
        .parse()
        .map_err(|_| "invalid CSVW temporal field".to_owned())
}

fn read_timezone(source: &str, cursor: &mut usize, width: usize) -> Result<String, String> {
    if source.as_bytes().get(*cursor) == Some(&b'Z') {
        *cursor += 1;
        return Ok("Z".to_owned());
    }
    let sign = *source
        .as_bytes()
        .get(*cursor)
        .filter(|byte| matches!(byte, b'+' | b'-'))
        .ok_or_else(|| "cell lacks the timezone required by its CSVW pattern".to_owned())?;
    *cursor += 1;
    let hour = read_digits(source, cursor, 2, 2)?;
    let minute = match width {
        1 => {
            if source
                .as_bytes()
                .get(*cursor..*cursor + 2)
                .is_some_and(|bytes| bytes.iter().all(u8::is_ascii_digit))
            {
                read_digits(source, cursor, 2, 2)?
            } else {
                0
            }
        }
        2 => read_digits(source, cursor, 2, 2)?,
        3 => {
            if source.as_bytes().get(*cursor) != Some(&b':') {
                return Err("CSVW timezone requires a colon".to_owned());
            }
            *cursor += 1;
            read_digits(source, cursor, 2, 2)?
        }
        _ => return Err("unsupported CSVW timezone pattern width".to_owned()),
    };
    if hour > 14 || minute > 59 || (hour == 14 && minute != 0) {
        return Err("CSVW timezone is outside the XSD range".to_owned());
    }
    Ok(format!("{}{:02}:{:02}", char::from(sign), hour, minute))
}

fn validate_primary_and_foreign_keys(
    group: &CsvwTableGroup,
    warnings: &mut Vec<CsvwWarning>,
) -> Result<(), ProjectionError> {
    for table in &group.tables {
        let column_indices = table
            .schema
            .columns
            .iter()
            .enumerate()
            .map(|(index, column)| (column.name.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let primary_indices = table
            .schema
            .primary_key
            .iter()
            .map(|name| {
                column_indices.get(name.as_str()).copied().ok_or_else(|| {
                    ProjectionError::integrity(format!(
                        "CSVW primary key references unknown column `{name}`"
                    ))
                    .at_path(&table.url)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut primary_values = BTreeSet::new();
        for row in &table.rows {
            let tuple = key_tuple(row, &primary_indices);
            if !primary_indices.is_empty() {
                if tuple.is_none() {
                    warnings.push(CsvwWarning::new(
                        CsvwWarningKind::Validation,
                        &table.url,
                        format!("row={}", row.source_number),
                        "CSVW primary-key cell is null",
                    ));
                } else if !primary_values.insert(tuple.unwrap_or_default()) {
                    warnings.push(CsvwWarning::new(
                        CsvwWarningKind::Validation,
                        &table.url,
                        format!("row={}", row.source_number),
                        "duplicate CSVW primary-key value",
                    ));
                }
            }
        }
        for foreign_key in &table.schema.foreign_keys {
            let local_indices = foreign_key
                .column_reference
                .iter()
                .map(|name| {
                    column_indices.get(name.as_str()).copied().ok_or_else(|| {
                        ProjectionError::integrity(format!(
                            "CSVW foreign key references unknown local column `{name}`"
                        ))
                        .at_path(&table.url)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let target = if let Some(resource) = &foreign_key.reference.resource {
                group
                    .tables
                    .iter()
                    .find(|candidate| &candidate.url == resource)
            } else if let Some(schema) = &foreign_key.reference.schema_reference {
                group
                    .tables
                    .iter()
                    .find(|candidate| candidate.schema.id.as_ref() == Some(schema))
            } else {
                None
            }
            .ok_or_else(|| {
                ProjectionError::integrity("CSVW foreign key has no matching target table")
                    .at_path(&table.url)
            })?;
            let target_columns = target
                .schema
                .columns
                .iter()
                .enumerate()
                .map(|(index, column)| (column.name.as_str(), index))
                .collect::<BTreeMap<_, _>>();
            let target_indices = foreign_key
                .reference
                .column_reference
                .iter()
                .map(|name| {
                    let index = target_columns.get(name.as_str()).copied().ok_or_else(|| {
                        ProjectionError::integrity(format!(
                            "CSVW foreign key references unknown target column `{name}`"
                        ))
                        .at_path(&table.url)
                    })?;
                    if !target.schema.columns[index].name_explicit {
                        return Err(ProjectionError::integrity(format!(
                            "CSVW foreign key target column `{name}` lacks an explicit name"
                        ))
                        .at_path(&table.url));
                    }
                    Ok(index)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut target_values = BTreeMap::<Vec<String>, usize>::new();
            for tuple in target
                .rows
                .iter()
                .filter_map(|row| key_tuple(row, &target_indices))
            {
                *target_values.entry(tuple).or_default() += 1;
            }
            for row in &table.rows {
                let valid_reference = key_tuple(row, &local_indices)
                    .is_some_and(|tuple| target_values.get(&tuple) == Some(&1));
                if !valid_reference {
                    warnings.push(CsvwWarning::new(
                        CsvwWarningKind::Validation,
                        &table.url,
                        format!("row={}", row.source_number),
                        "CSVW foreign-key value must identify exactly one referenced row",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn key_tuple(row: &CsvwRow, indices: &[usize]) -> Option<Vec<String>> {
    indices
        .iter()
        .map(|index| {
            let cell = row.cells.get(*index)?;
            (!cell.is_null).then(|| cell.string_value.clone())
        })
        .collect()
}

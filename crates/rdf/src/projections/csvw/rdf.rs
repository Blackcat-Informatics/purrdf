// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Normative CSVW-to-RDF conversion.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};
use purrdf_iri::terminals::{ByteClass, byte_run_count};
use serde_json::Value;

use super::super::ProjectionError;
use super::config::{CsvwConfig, CsvwMode};
use super::model::{CsvwAnnotations, CsvwCell, CsvwRow, CsvwTable, CsvwTableGroup};

pub(crate) fn table_group_to_rdf(
    group: &CsvwTableGroup,
    config: &CsvwConfig,
) -> Result<Arc<RdfDataset>, ProjectionError> {
    let mut converter = Converter {
        builder: RdfDatasetBuilder::new(),
        config,
        blank_counter: 0,
    };
    converter.convert(group)?;
    converter
        .builder
        .freeze()
        .map_err(|error| ProjectionError::integrity(format!("invalid CSVW RDF output: {error}")))
}

struct Converter<'a> {
    builder: RdfDatasetBuilder,
    config: &'a CsvwConfig,
    blank_counter: u64,
}

impl Converter<'_> {
    fn convert(&mut self, group: &CsvwTableGroup) -> Result<(), ProjectionError> {
        let standard = self.config.mode() == CsvwMode::Standard;
        let group_node = if standard {
            Some(match &group.rdf_id {
                Some(id) => self.builder.intern_iri(id),
                None => self.blank("group"),
            })
        } else {
            None
        };
        if let Some(group_node) = group_node {
            self.type_quad(group_node, "TableGroup");
            self.emit_annotations(group_node, &group.annotations)?;
        }
        for (table_index, table) in group.tables.iter().enumerate() {
            if table.suppress_output {
                continue;
            }
            let table_node = if standard {
                Some(match &table.id {
                    Some(id) => self.builder.intern_iri(id),
                    None => self.blank(&format!("table-{table_index}")),
                })
            } else {
                None
            };
            if let (Some(group_node), Some(table_node)) = (group_node, table_node) {
                let table_predicate = self.csvw("table");
                self.quad(group_node, table_predicate, table_node);
                self.type_quad(table_node, "Table");
                let url = self.builder.intern_iri(&table.url);
                let url_predicate = self.csvw("url");
                self.quad(table_node, url_predicate, url);
                for comment in &table.comments {
                    let literal = self.simple_literal(comment);
                    let comment_predicate = self.rdfs("comment");
                    self.quad(table_node, comment_predicate, literal);
                }
                self.emit_annotations(table_node, &table.annotations)?;
            }
            for row in &table.rows {
                self.convert_row(table, row, table_node, table_index)?;
            }
        }
        Ok(())
    }

    fn convert_row(
        &mut self,
        table: &CsvwTable,
        row: &CsvwRow,
        table_node: Option<TermId>,
        table_index: usize,
    ) -> Result<(), ProjectionError> {
        let standard = self.config.mode() == CsvwMode::Standard;
        let row_node = standard.then(|| self.blank(&format!("row-{table_index}-{}", row.number)));
        if let (Some(table_node), Some(row_node)) = (table_node, row_node) {
            let row_predicate = self.csvw("row");
            self.quad(table_node, row_predicate, row_node);
            self.type_quad(row_node, "Row");
            let rownum = self.typed_literal(
                &row.number.to_string(),
                self.config.vocabulary().xsd("integer"),
            );
            let rownum_predicate = self.csvw("rownum");
            self.quad(row_node, rownum_predicate, rownum);
            let url = self.builder.intern_iri(&row.url);
            let url_predicate = self.csvw("url");
            self.quad(row_node, url_predicate, url);
            for title in &row.titles {
                let title = self.simple_literal(title);
                let title_predicate = self.csvw("title");
                self.quad(row_node, title_predicate, title);
            }
        }
        let variables = row_variables(table, row);
        let default_subject = self.blank(&format!("subject-{table_index}-{}", row.number));
        let mut described = BTreeSet::new();
        for (column, cell) in table.schema.columns.iter().zip(&row.cells) {
            if column.suppress_output {
                continue;
            }
            let mut cell_variables = variables.clone();
            cell_variables.insert("_name".to_owned(), column.name.clone());
            cell_variables.insert("_column".to_owned(), (column.number + 1).to_string());
            cell_variables.insert(
                "_sourceColumn".to_owned(),
                (column.number + 1 + table.dialect.skip_columns).to_string(),
            );
            let subject = match &column.inherited.about_url {
                Some(template) => {
                    let iri = expand_url(template, &cell_variables, &table.url, self.config)?;
                    self.builder.intern_iri(&iri)
                }
                None => default_subject,
            };
            if let Some(row_node) = row_node
                && described.insert(subject)
            {
                let describes_predicate = self.csvw("describes");
                self.quad(row_node, describes_predicate, subject);
            }
            let predicate_iri = match &column.inherited.property_url {
                Some(template) => expand_url(template, &cell_variables, &table.url, self.config)?,
                None => fragment_iri(&table.url, &column.name)?,
            };
            let predicate = self.builder.intern_iri(&predicate_iri);
            if cell.is_null && !(column.virtual_column && column.inherited.value_url.is_some()) {
                continue;
            }
            if let Some(template) = &column.inherited.value_url {
                if cell.values.is_empty() {
                    let value = expand_url(template, &cell_variables, &table.url, self.config)?;
                    let object = self.builder.intern_iri(&value);
                    self.quad(subject, predicate, object);
                } else {
                    let mut component_variables = cell_variables.clone();
                    for component in &cell.values {
                        let component_variable =
                            component_variables.get_mut(&column.name).ok_or_else(|| {
                                ProjectionError::integrity(
                                    "CSVW component variable is absent from its row",
                                )
                            })?;
                        component_variable.clone_from(&component.source);
                        let value =
                            expand_url(template, &component_variables, &table.url, self.config)?;
                        let object = self.builder.intern_iri(&value);
                        self.quad(subject, predicate, object);
                    }
                }
            } else if column.inherited.separator.is_some() && column.inherited.ordered {
                if !cell.values.is_empty() {
                    let head = self.emit_list(cell, table_index, row.number, column.number);
                    self.quad(subject, predicate, head);
                }
            } else {
                for value in &cell.values {
                    let object = self.value_literal(value);
                    self.quad(subject, predicate, object);
                }
            }
        }
        Ok(())
    }

    fn emit_list(&mut self, cell: &CsvwCell, table: usize, row: usize, column: usize) -> TermId {
        let nil = self
            .builder
            .intern_iri(&self.config.vocabulary().rdf("nil"));
        let first_predicate = self.rdf("first");
        let rest_predicate = self.rdf("rest");
        let mut nodes = Vec::with_capacity(cell.values.len());
        for index in 0..cell.values.len() {
            nodes.push(self.blank(&format!("list-{table}-{row}-{column}-{index}")));
        }
        for (index, value) in cell.values.iter().enumerate() {
            let object = self.value_literal(value);
            self.quad(nodes[index], first_predicate, object);
            self.quad(
                nodes[index],
                rest_predicate,
                nodes.get(index + 1).copied().unwrap_or(nil),
            );
        }
        nodes[0]
    }

    fn emit_annotations(
        &mut self,
        subject: TermId,
        annotations: &CsvwAnnotations,
    ) -> Result<(), ProjectionError> {
        for (property, value) in annotations {
            let predicate = self.builder.intern_iri(property);
            let objects = self.annotation_objects(value)?;
            for object in objects {
                self.quad(subject, predicate, object);
            }
        }
        Ok(())
    }

    fn annotation_objects(&mut self, value: &Value) -> Result<Vec<TermId>, ProjectionError> {
        match value {
            Value::Array(values) => {
                let mut objects = Vec::new();
                for value in values {
                    objects.extend(self.annotation_objects(value)?);
                }
                Ok(objects)
            }
            Value::Object(object) if object.contains_key("@value") => {
                Ok(vec![self.annotation_literal(object)?])
            }
            Value::Object(object) => {
                let node = match object.get("@id") {
                    Some(Value::String(id)) => {
                        let iri = expand_jsonld_iri(id, self.config)?;
                        self.builder.intern_iri(&iri)
                    }
                    Some(_) => {
                        return Err(ProjectionError::integrity(
                            "CSVW annotation @id must be a string",
                        ));
                    }
                    None => self.blank("annotation"),
                };
                if let Some(types) = object.get("@type") {
                    let values: Vec<&Value> = match types {
                        Value::Array(values) => values.iter().collect(),
                        value => vec![value],
                    };
                    for value in values {
                        let Value::String(value) = value else {
                            return Err(ProjectionError::integrity(
                                "CSVW annotation @type must contain strings",
                            ));
                        };
                        let object = self
                            .builder
                            .intern_iri(&expand_jsonld_iri(value, self.config)?);
                        let type_predicate = self.rdf("type");
                        self.quad(node, type_predicate, object);
                    }
                }
                for (property, value) in object {
                    if property.starts_with('@') {
                        continue;
                    }
                    let property = if property == "notes" {
                        self.config.vocabulary().csvw("notes")
                    } else {
                        expand_jsonld_iri(property, self.config)?
                    };
                    let predicate = self.builder.intern_iri(&property);
                    for object in self.annotation_objects(value)? {
                        self.quad(node, predicate, object);
                    }
                }
                Ok(vec![node])
            }
            Value::String(value) => Ok(vec![self.simple_literal(value)]),
            Value::Bool(value) => Ok(vec![self.typed_literal(
                if *value { "true" } else { "false" },
                self.config.vocabulary().xsd("boolean"),
            )]),
            Value::Number(value) => {
                let datatype = if value.is_i64() || value.is_u64() {
                    self.config.vocabulary().xsd("integer")
                } else {
                    self.config.vocabulary().xsd("double")
                };
                Ok(vec![self.typed_literal(&value.to_string(), datatype)])
            }
            Value::Null => Ok(Vec::new()),
        }
    }

    fn annotation_literal(
        &mut self,
        object: &serde_json::Map<String, Value>,
    ) -> Result<TermId, ProjectionError> {
        let value = object.get("@value").ok_or_else(|| {
            ProjectionError::integrity("CSVW annotation value object lacks @value")
        })?;
        let lexical = match value {
            Value::String(value) => value.clone(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            _ => {
                return Err(ProjectionError::integrity(
                    "CSVW annotation @value is not atomic",
                ));
            }
        };
        let language = object
            .get("@language")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let datatype = if language.is_some() {
            None
        } else if let Some(Value::String(datatype)) = object.get("@type") {
            Some(expand_jsonld_iri(datatype, self.config)?)
        } else if value.is_boolean() {
            Some(self.config.vocabulary().xsd("boolean"))
        } else if value.is_number() {
            Some(if value.as_i64().is_some() || value.as_u64().is_some() {
                self.config.vocabulary().xsd("integer")
            } else {
                self.config.vocabulary().xsd("double")
            })
        } else {
            None
        };
        Ok(self.builder.intern_literal(RdfLiteral {
            lexical_form: lexical,
            datatype,
            language,
            direction: None,
        }))
    }

    fn value_literal(&mut self, value: &super::model::CsvwValue) -> TermId {
        self.builder.intern_literal(RdfLiteral {
            lexical_form: value.lexical.clone(),
            datatype: value.language.is_none().then(|| value.datatype.clone()),
            language: value.language.clone(),
            // The CSVW Recommendation's RDF conversion targets RDF 1.1 and has no
            // directional-literal mapping. Direction remains exact in CsvwValue and
            // the annotated table model instead of silently changing W3C output.
            direction: None,
        })
    }

    fn simple_literal(&mut self, value: &str) -> TermId {
        self.builder.intern_literal(RdfLiteral::simple(value))
    }

    fn typed_literal(&mut self, lexical: &str, datatype: String) -> TermId {
        self.builder
            .intern_literal(RdfLiteral::typed(lexical, datatype))
    }

    fn type_quad(&mut self, subject: TermId, csvw_type: &str) {
        let predicate = self.rdf("type");
        let object = self.csvw(csvw_type);
        self.quad(subject, predicate, object);
    }

    fn csvw(&mut self, local: &str) -> TermId {
        self.builder
            .intern_iri(&self.config.vocabulary().csvw(local))
    }

    fn rdf(&mut self, local: &str) -> TermId {
        self.builder
            .intern_iri(&self.config.vocabulary().rdf(local))
    }

    fn rdfs(&mut self, local: &str) -> TermId {
        self.builder
            .intern_iri(&self.config.vocabulary().rdfs(local))
    }

    fn blank(&mut self, role: &str) -> TermId {
        let label = format!("csvw-{role}-{}", self.blank_counter);
        self.blank_counter += 1;
        self.builder.intern_blank(&label, BlankScope(0))
    }

    fn quad(&mut self, subject: TermId, predicate: TermId, object: TermId) {
        self.builder.push_quad(subject, predicate, object, None);
    }
}

fn row_variables(table: &CsvwTable, row: &CsvwRow) -> BTreeMap<String, String> {
    let mut variables = BTreeMap::from([
        ("_row".to_owned(), row.number.to_string()),
        ("_sourceRow".to_owned(), row.source_number.to_string()),
    ]);
    for (column, cell) in table.schema.columns.iter().zip(&row.cells) {
        variables.insert(column.name.clone(), cell.string_value.clone());
    }
    variables
}

fn expand_url(
    template: &str,
    variables: &BTreeMap<String, String>,
    table_url: &str,
    config: &CsvwConfig,
) -> Result<String, ProjectionError> {
    let mut output = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        output.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| ProjectionError::syntax("unterminated CSVW URI-template expression"))?;
        let expression = &after[..close];
        let (operator, name) = expression.strip_prefix('#').map_or_else(
            || {
                expression
                    .strip_prefix('+')
                    .map_or((None, expression), |name| (Some('+'), name))
            },
            |name| (Some('#'), name),
        );
        let value = variables.get(name).map_or("", String::as_str);
        if operator == Some('#') && !value.is_empty() {
            output.push('#');
        }
        if name == "_name" {
            output.push_str(value);
        } else if operator == Some('+') {
            output.push_str(&percent_encode_reserved(value));
        } else {
            output.push_str(&percent_encode(value));
        }
        rest = &after[close + 1..];
    }
    output.push_str(rest);
    if output.contains(['{', '}']) {
        return Err(ProjectionError::syntax(
            "unsupported nested CSVW URI-template expression",
        ));
    }
    if let Ok(iri) = config.context().expand_iri(&output) {
        return Ok(iri);
    }
    let base = purrdf_iri::parse(table_url)
        .map_err(|error| ProjectionError::term(format!("invalid CSVW table URL: {error}")))?;
    base.resolve(&output)
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| ProjectionError::term(format!("invalid expanded CSVW URL: {error}")))
}

/// The `{+name}` (reserved) expansion: every byte [`reserved_byte`] admits is
/// written as itself and every other byte as `%XX`, borrowing `value` when
/// nothing needs encoding.
fn percent_encode_reserved(value: &str) -> Cow<'_, str> {
    let Some(first) = find_first_reserved_escape(value.as_bytes()) else {
        return Cow::Borrowed(value);
    };
    let mut output = String::with_capacity(value.len() + 2);
    push_percent_encoded(value, first, find_first_reserved_escape, &mut output);
    Cow::Owned(output)
}

/// The bytes the reserved expansion percent-encodes: every byte
/// [`reserved_byte`] does not admit (the C0 controls, space, `"`, `<`, `>`,
/// `\`, `^`, `` ` ``, `{`, `|`, `}`, DEL and every non-ASCII byte).
const RESERVED_ESCAPE_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b: u8 = 0;
    loop {
        if !reserved_byte(b) {
            table[b as usize] = 1;
        }
        if b == u8::MAX {
            break;
        }
        b += 1;
    }
    table
};

const RESERVED_ESCAPES: ByteClass<{ byte_run_count(&RESERVED_ESCAPE_TABLE) }> =
    ByteClass::from_table(RESERVED_ESCAPE_TABLE);

/// The offset of the first byte of `bytes` the reserved expansion encodes.
///
/// Out of line, so the class compiles to one kernel with its runs folded in as
/// constants.
#[inline(never)]
fn find_first_reserved_escape(bytes: &[u8]) -> Option<usize> {
    RESERVED_ESCAPES.find_first(bytes)
}

/// Append `value` to `output` with every byte `find` stops at written as
/// `%XX` (uppercase hex) and every other byte as itself; `first` is the offset
/// of the first stop.
///
/// Both encoded classes hold every non-ASCII byte, so a run between two stops
/// is ASCII and begins and ends on a `char` boundary. The position after a stop
/// may fall inside a multi-byte scalar, but then the next byte is a stop too
/// and the run there is empty, so it is never sliced.
fn push_percent_encoded(
    value: &str,
    first: usize,
    find: impl Fn(&[u8]) -> Option<usize>,
    output: &mut String,
) {
    const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";
    let bytes = value.as_bytes();
    let mut run_start = 0;
    let mut hit = first;
    loop {
        if hit > run_start {
            output.push_str(&value[run_start..hit]);
        }
        let byte = bytes[hit];
        output.push('%');
        output.push(char::from(HEX_UPPER[usize::from(byte >> 4)]));
        output.push(char::from(HEX_UPPER[usize::from(byte & 0xF)]));
        run_start = hit + 1;
        match find(&bytes[run_start..]) {
            Some(offset) => hit = run_start + offset,
            None => break,
        }
    }
    output.push_str(&value[run_start..]);
}

const fn reserved_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b':'
                | b'/'
                | b'?'
                | b'#'
                | b'['
                | b']'
                | b'@'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'%'
        )
}

fn fragment_iri(table_url: &str, name: &str) -> Result<String, ProjectionError> {
    let base = purrdf_iri::parse(table_url)
        .map_err(|error| ProjectionError::term(format!("invalid CSVW table URL: {error}")))?;
    base.resolve(&format!("#{name}"))
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| ProjectionError::term(format!("invalid CSVW property URL: {error}")))
}

/// The `{name}` (simple) expansion: every byte [`unreserved_byte`] admits is
/// written as itself and every other byte as `%XX`.
fn percent_encode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    match find_first_unreserved_escape(value.as_bytes()) {
        Some(first) => {
            push_percent_encoded(value, first, find_first_unreserved_escape, &mut output);
        }
        None => output.push_str(value),
    }
    output
}

/// RFC 3986's `unreserved`: ASCII letters and digits, `-`, `.`, `_` and `~`.
const fn unreserved_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// The bytes the simple expansion percent-encodes: every byte
/// [`unreserved_byte`] does not admit.
const UNRESERVED_ESCAPE_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b: u8 = 0;
    loop {
        if !unreserved_byte(b) {
            table[b as usize] = 1;
        }
        if b == u8::MAX {
            break;
        }
        b += 1;
    }
    table
};

const UNRESERVED_ESCAPES: ByteClass<{ byte_run_count(&UNRESERVED_ESCAPE_TABLE) }> =
    ByteClass::from_table(UNRESERVED_ESCAPE_TABLE);

/// The offset of the first byte of `bytes` the simple expansion encodes.
///
/// Out of line, so the class compiles to one kernel with its runs folded in as
/// constants.
#[inline(never)]
fn find_first_unreserved_escape(bytes: &[u8]) -> Option<usize> {
    UNRESERVED_ESCAPES.find_first(bytes)
}

fn expand_jsonld_iri(value: &str, config: &CsvwConfig) -> Result<String, ProjectionError> {
    // CSVW's fixed class names and metadata-base fallback are profile rules layered
    // above the caller-supplied context; compact-IRI processing stays in CsvwContext.
    if matches!(
        value,
        "TableGroup" | "Table" | "Schema" | "Column" | "Dialect" | "Template"
    ) {
        return Ok(config.vocabulary().csvw(value));
    }
    if let Ok(expanded) = config.context().expand_iri(value) {
        return Ok(expanded);
    }
    let base = purrdf_iri::parse(config.metadata_base_iri())
        .map_err(|error| ProjectionError::configuration(format!("invalid CSVW base: {error}")))?;
    base.resolve(value)
        .map(|iri| iri.as_str().to_owned())
        .map_err(|error| ProjectionError::term(format!("invalid CSVW JSON-LD IRI: {error}")))
}

#[cfg(test)]
mod tests {
    use super::{
        RESERVED_ESCAPES, UNRESERVED_ESCAPES, percent_encode, percent_encode_reserved,
        reserved_byte, unreserved_byte,
    };
    use std::borrow::Cow;
    use std::fmt::Write as _;

    /// The per-byte reserved expansion the scan replaced, kept as the oracle.
    fn percent_encode_reserved_reference(value: &str) -> Cow<'_, str> {
        let Some(first_escape) = value.bytes().position(|byte| !reserved_byte(byte)) else {
            return Cow::Borrowed(value);
        };
        let mut output = String::with_capacity(value.len() + 2);
        output.push_str(&value[..first_escape]);
        for &byte in &value.as_bytes()[first_escape..] {
            if reserved_byte(byte) {
                output.push(char::from(byte));
            } else {
                let _ = write!(output, "%{byte:02X}");
            }
        }
        Cow::Owned(output)
    }

    /// The per-byte simple expansion the scan replaced, kept as the oracle.
    fn percent_encode_reference(value: &str) -> String {
        let mut output = String::with_capacity(value.len());
        for byte in value.bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                output.push(char::from(byte));
            } else {
                let _ = write!(output, "%{byte:02X}");
            }
        }
        output
    }

    /// Every ASCII scalar and non-ASCII scalars of each UTF-8 width, each
    /// placed after every prefix length 0 to 40 of a plain run (so across the
    /// sixteen-byte chunks and into the tail), alone, doubled, and followed by
    /// plain text, other stops and a second scalar.
    fn samples() -> Vec<String> {
        let mut scalars: Vec<char> = (0_u8..=0x7F).map(char::from).collect();
        scalars.extend([
            '\u{85}',
            '\u{a0}',
            '\u{e9}',
            '\u{2028}',
            '\u{feff}',
            '\u{1f600}',
        ]);
        let mut out = vec![String::new()];
        for &c in &scalars {
            for prefix in 0..=40 {
                let plain = "a/b".repeat(prefix / 3 + 1);
                let head = &plain[..prefix.min(plain.len())];
                out.push(format!("{head}{c}"));
                out.push(format!("{head}{c}{c}"));
                out.push(format!("{head}{c}tail-0123456789-ABCDEFGHIJ"));
                out.push(format!("{head}{c} %{{}}\u{e9}{head}\u{1f600}"));
            }
        }
        out
    }

    #[test]
    fn reserved_expansion_scan_equals_the_per_byte_encoder() {
        for byte in 0_u8..=u8::MAX {
            assert_eq!(
                RESERVED_ESCAPES.contains(byte),
                !reserved_byte(byte),
                "{byte:#04x}"
            );
        }
        for value in samples() {
            let expected = percent_encode_reserved_reference(&value);
            let actual = percent_encode_reserved(&value);
            assert_eq!(actual, expected, "{value:?}");
            assert_eq!(
                matches!(actual, Cow::Borrowed(_)),
                matches!(expected, Cow::Borrowed(_)),
                "{value:?}"
            );
        }
    }

    #[test]
    fn simple_expansion_scan_equals_the_per_byte_encoder() {
        for byte in 0_u8..=u8::MAX {
            assert_eq!(
                UNRESERVED_ESCAPES.contains(byte),
                !unreserved_byte(byte),
                "{byte:#04x}"
            );
        }
        for value in samples() {
            assert_eq!(
                percent_encode(&value),
                percent_encode_reference(&value),
                "{value:?}"
            );
        }
    }

    #[test]
    fn reserved_expansion_borrows_unchanged_iris_and_encodes_only_on_demand() {
        let unchanged = "https://example.org/a/b?x=y#fragment";
        assert!(matches!(
            percent_encode_reserved(unchanged),
            Cow::Borrowed(value) if value == unchanged
        ));

        assert_eq!(
            percent_encode_reserved("https://example.org/na\u{ef}ve path"),
            Cow::<str>::Owned("https://example.org/na%C3%AFve%20path".to_owned())
        );
    }
}

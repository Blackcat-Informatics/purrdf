// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL Results **JSON** (SRJ) reader — the inverse of [`crate::json`].
//!
//! Parses a W3C SPARQL 1.1 Query Results JSON document
//! (<https://www.w3.org/TR/sparql11-results-json/>) into a [`ParsedSolutions`]
//! (for `SELECT`) or a boolean (for `ASK`). This is what SPARQL `SERVICE`
//! federation uses to ingest a remote endpoint's response, and what
//! the W3C conformance harness uses to read expected `.srj` results.
//!
//! # Wasm discipline
//!
//! A hand-rolled recursive-descent parser over `&[u8]` — **no `serde`, no
//! `std::io`** — symmetric with the hand-rolled writers and keeping the crate
//! wasm-clean and oxigraph-free.

use purrdf_core::{BlankScope, RdfTextDirection, TermValue};

use crate::error::Error;
use crate::model::{ProvenanceNamespace, ResultProvenance, SolutionProvenance};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// Dense decoded row and bounded row-prefix result aliases keep the streaming reader's
/// signatures readable without changing the public model.
type BindingRow = Vec<Option<TermValue>>;
type BoundedRows = (Vec<BindingRow>, bool);
type BoundedRowsResult = Result<BoundedRows, Error>;

/// A decoded `SELECT` result set: ordered variable names plus dense rows. A
/// `None` cell is an unbound (absent) binding for that variable in that row.
///
/// This is the structural inverse of [`purrdf_core::SparqlResult::Solutions`]
/// and the shape the SERVICE evaluator interns into a solution sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSolutions {
    /// The result variables, in `head.vars` order.
    pub variables: Vec<String>,
    /// One row per binding; `rows[i][j]` is the value of `variables[j]`.
    pub rows: Vec<Vec<Option<TermValue>>>,
}

/// A bounded decode result. `truncated` means the document contained at least one binding
/// beyond the supplied intermediate-cell ceiling; `solutions.rows` is the ordered prefix
/// that fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedParsedSolutions {
    /// The decoded, cell-bounded solution prefix.
    pub solutions: ParsedSolutions,
    /// Whether a further binding was present and deliberately not materialized.
    pub truncated: bool,
}

/// Parse a SPARQL Results JSON `SELECT` document into [`ParsedSolutions`].
///
/// # Examples
///
/// ```
/// use purrdf_core::TermValue;
/// use purrdf_sparql_results::from_json;
///
/// let doc = br#"{
///   "head": { "vars": ["s"] },
///   "results": { "bindings": [
///     { "s": { "type": "uri", "value": "http://example.org/alice" } }
///   ] }
/// }"#;
///
/// let parsed = from_json(doc).expect("well-formed SRJ");
/// assert_eq!(parsed.variables, ["s"]);
/// assert_eq!(
///     parsed.rows,
///     [vec![Some(TermValue::Iri("http://example.org/alice".to_string()))]],
/// );
/// ```
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed JSON, a non-object document, an `ASK`
/// (`boolean`) document (use [`from_json_boolean`]), or a binding object whose
/// `type`/`value` shape is invalid.
pub fn from_json(bytes: &[u8]) -> Result<ParsedSolutions, Error> {
    let doc = JsonParser::new(bytes).parse_document()?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    if obj_get(obj, "boolean").is_some() {
        return Err(fmt(
            "expected SELECT results, got an ASK (boolean) document",
        ));
    }
    let head = obj_get(obj, "head")
        .and_then(Json::as_object)
        .ok_or_else(|| fmt("missing `head` object"))?;
    let variables = match obj_get(head, "vars") {
        Some(Json::Array(items)) => items
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| fmt("`head.vars` entry is not a string"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        // A results doc with no `vars` is degenerate but valid (zero columns).
        _ => Vec::new(),
    };

    let results = obj_get(obj, "results")
        .and_then(Json::as_object)
        .ok_or_else(|| fmt("missing `results` object"))?;
    let bindings = match obj_get(results, "bindings") {
        Some(Json::Array(items)) => items.as_slice(),
        _ => return Err(fmt("missing `results.bindings` array")),
    };

    let mut rows = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let row_obj = binding
            .as_object()
            .ok_or_else(|| fmt("`results.bindings` entry is not an object"))?;
        let mut row = vec![None; variables.len()];
        for (j, var) in variables.iter().enumerate() {
            if let Some(cell) = obj_get(row_obj, var) {
                row[j] = Some(decode_binding(cell)?);
            }
        }
        rows.push(row);
    }
    Ok(ParsedSolutions { variables, rows })
}

/// Parse a SPARQL Results JSON `SELECT` document without materializing more than
/// `max_cells` cells (`rows * head.vars.len()`).
///
/// The document is scanned twice. The first pass reads only `head.vars`; the second decodes
/// bindings in source order until the inclusive ceiling is full, then structurally skips
/// the suffix without constructing its JSON tree or owned RDF terms. Two passes over bytes
/// are cheaper than building an attacker-sized response tree, and retain JSON's
/// order-independence (`results` may precede `head`).
///
/// A zero-column result has zero cells regardless of row count and is therefore decoded in
/// full. Callers that need to bound unit rows must use a row/answer governor.
///
/// # Errors
///
/// Returns [`Error::Format`] under the same structural conditions as [`from_json`]. JSON in
/// the deliberately skipped over-limit suffix is still syntax-validated, but its SPARQL
/// binding shape is not decoded because the cell governor has already refused that work.
pub fn from_json_bounded(bytes: &[u8], max_cells: u64) -> Result<BoundedParsedSolutions, Error> {
    let variables = scan_variables(bytes)?;
    let row_limit = if variables.is_empty() {
        None
    } else {
        let width = u64::try_from(variables.len()).unwrap_or(u64::MAX);
        Some(usize::try_from(max_cells / width).unwrap_or(usize::MAX))
    };
    let (rows, truncated) = scan_bindings(bytes, &variables, row_limit)?;
    Ok(BoundedParsedSolutions {
        solutions: ParsedSolutions { variables, rows },
        truncated,
    })
}

/// First bounded-decoder pass: locate and decode `head.vars`, skipping every other value
/// without building a JSON tree.
fn scan_variables(bytes: &[u8]) -> Result<Vec<String>, Error> {
    let mut parser = JsonParser::new(bytes);
    parser.skip_ws();
    parser.expect(b'{', "top level is not an object")?;
    let mut variables = None;
    parser.skip_ws();
    if parser.peek() != Some(b'}') {
        loop {
            let key = parser.parse_object_key()?;
            if key == "head" && variables.is_none() {
                variables = Some(parser.parse_head_variables()?);
            } else {
                parser.skip_value()?;
            }
            if parser.finish_entry(b'}', "expected `,` or `}` in object")? {
                break;
            }
        }
    } else {
        parser.pos += 1;
    }
    parser.finish_document()?;
    variables.ok_or_else(|| fmt("missing `head` object"))
}

/// Second bounded-decoder pass: locate `results.bindings`, materializing only the prefix
/// admitted by `row_limit`.
fn scan_bindings(
    bytes: &[u8],
    variables: &[String],
    row_limit: Option<usize>,
) -> BoundedRowsResult {
    let mut parser = JsonParser::new(bytes);
    parser.skip_ws();
    parser.expect(b'{', "top level is not an object")?;
    let mut result = None;
    let mut saw_boolean = false;
    parser.skip_ws();
    if parser.peek() != Some(b'}') {
        loop {
            let key = parser.parse_object_key()?;
            match key.as_str() {
                "boolean" => {
                    saw_boolean = true;
                    parser.skip_value()?;
                }
                "results" if result.is_none() => {
                    result = Some(parser.parse_bounded_results(variables, row_limit)?);
                }
                _ => parser.skip_value()?,
            }
            if parser.finish_entry(b'}', "expected `,` or `}` in object")? {
                break;
            }
        }
    } else {
        parser.pos += 1;
    }
    parser.finish_document()?;
    if saw_boolean {
        return Err(fmt(
            "expected SELECT results, got an ASK (boolean) document",
        ));
    }
    result.ok_or_else(|| fmt("missing `results` object"))
}

/// Parse a SPARQL Results JSON `ASK` document into its boolean.
///
/// # Examples
///
/// ```
/// use purrdf_sparql_results::from_json_boolean;
///
/// let verdict = from_json_boolean(br#"{ "head": {}, "boolean": true }"#)
///     .expect("well-formed ASK document");
/// assert!(verdict);
/// ```
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed JSON or a document without a boolean
/// `boolean` field.
pub fn from_json_boolean(bytes: &[u8]) -> Result<bool, Error> {
    let doc = JsonParser::new(bytes).parse_document()?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    match obj_get(obj, "boolean") {
        Some(Json::Bool(b)) => Ok(*b),
        _ => Err(fmt("missing boolean `boolean` field")),
    }
}

/// Decode the additive `purrdf` provenance extension a
/// [`ProvenanceNamespace`]-keyed SRJ document carries, or
/// [`ResultProvenance::default`] when no such top-level member is present.
///
/// This is the inverse of [`crate::json::to_json`]'s additive extension (see
/// [`crate::model::ProvenanceNamespace`]'s module docs): the writer emits the
/// member keyed under the caller-supplied namespace's `prefix`, but the KEY
/// SPELLING is not what identifies the member as this caller's own — a bare
/// string like `"prov"` has no uniqueness guarantee (the actual W3C PROV
/// namespace, `http://www.w3.org/ns/prov#`, is commonly bound to exactly that
/// prefix). Identity is the `"namespace"` field the writer records INSIDE the
/// member (the JSON twin of the XML writer's `xmlns:{prefix}="{iri}"`
/// declaration): every top-level object-valued member is scanned, and the
/// first one whose own `"namespace"` field equals `namespace.iri()` is
/// decoded — regardless of which key it happens to be spelled under. This
/// mirrors [`crate::xml_read::provenance_from_xml`]'s namespace-URI match
/// exactly: a document that writes this caller's IRI under a DIFFERENT
/// top-level key still decodes correctly, and a document that reuses this
/// caller's PREFIX spelling for an unrelated namespace is correctly not read
/// as this caller's extension. A document with no member recording
/// `namespace.iri()` (never written, or written under a different namespace)
/// decodes to the empty provenance, exactly like a document nothing ever
/// populated.
///
/// The `queryForm` field is read to VALIDATE the member's shape but is not
/// itself carried in [`ResultProvenance`] (it is derived from the result kind
/// on write, not caller data).
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed JSON, a non-object document, or a
/// member recording `namespace.iri()` whose shape does not match the
/// writer's (`queryForm`/`queryHash`/`engine`/`solutions[].sources[]`, all
/// strings).
pub fn provenance_from_json(
    bytes: &[u8],
    namespace: &ProvenanceNamespace,
) -> Result<ResultProvenance, Error> {
    let doc = JsonParser::new(bytes).parse_document()?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    let iri = namespace.iri();
    let Some(member_obj) = obj.iter().find_map(|(_, value)| {
        let candidate = value.as_object()?;
        let recorded = obj_get(candidate, "namespace").and_then(Json::as_str)?;
        (recorded == iri).then_some(candidate)
    }) else {
        return Ok(ResultProvenance::default());
    };
    let query_hash = obj_get(member_obj, "queryHash")
        .and_then(Json::as_str)
        .map(str::to_owned);
    let engine = obj_get(member_obj, "engine")
        .and_then(Json::as_str)
        .map(str::to_owned);
    let solutions = match obj_get(member_obj, "solutions") {
        Some(Json::Array(items)) => items
            .iter()
            .map(|item| {
                let item_obj = item
                    .as_object()
                    .ok_or_else(|| fmt("provenance solution is not an object"))?;
                let sources = match obj_get(item_obj, "sources") {
                    Some(Json::Array(values)) => values
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(str::to_owned)
                                .ok_or_else(|| fmt("provenance source is not a string"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => Vec::new(),
                };
                Ok(SolutionProvenance { sources })
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    Ok(ResultProvenance {
        query_hash,
        engine,
        solutions,
    })
}

/// Decode one SPARQL-JSON binding object into a [`TermValue`] (recursive for
/// RDF 1.2 triple terms).
fn decode_binding(value: &Json) -> Result<TermValue, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| fmt("binding is not an object"))?;
    let ty = obj_get(obj, "type")
        .and_then(Json::as_str)
        .ok_or_else(|| fmt("binding has no string `type`"))?;
    match ty {
        "uri" => {
            let v = binding_value(obj)?;
            Ok(TermValue::Iri(v.to_owned()))
        }
        "bnode" => {
            let v = binding_value(obj)?;
            Ok(TermValue::Blank {
                label: v.to_owned(),
                scope: BlankScope::DEFAULT,
            })
        }
        "literal" | "typed-literal" => {
            let v = binding_value(obj)?;
            let language = obj_get(obj, "xml:lang").and_then(Json::as_str);
            // `its:dir` (the ITS — Internationalization Tag Set — namespace
            // convention) is the spelling the SPARQL 1.2 Query Results
            // specification uses for RDF 1.2 base direction — see
            // [`crate::json`]'s module docs for the fixture evidence. The bare
            // `dir` spelling is tolerated too, for interop with producers that
            // predate the SPARQL 1.2 spelling.
            let direction = match obj_get(obj, "its:dir")
                .or_else(|| obj_get(obj, "dir"))
                .and_then(Json::as_str)
            {
                Some("ltr") => Some(RdfTextDirection::Ltr),
                Some("rtl") => Some(RdfTextDirection::Rtl),
                Some(other) => return Err(fmt(&format!("unknown base direction `{other}`"))),
                None => None,
            };
            let datatype = obj_get(obj, "datatype").and_then(Json::as_str);
            let datatype = resolve_datatype(datatype, language.is_some(), direction.is_some());
            Ok(TermValue::Literal {
                lexical_form: v.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            })
        }
        "triple" => {
            let inner = obj_get(obj, "value")
                .and_then(Json::as_object)
                .ok_or_else(|| fmt("triple binding has no object `value`"))?;
            let s = decode_binding(
                obj_get(inner, "subject").ok_or_else(|| fmt("triple has no subject"))?,
            )?;
            let p = decode_binding(
                obj_get(inner, "predicate").ok_or_else(|| fmt("triple has no predicate"))?,
            )?;
            let o = decode_binding(
                obj_get(inner, "object").ok_or_else(|| fmt("triple has no object"))?,
            )?;
            if !matches!(p, TermValue::Iri(_)) {
                return Err(fmt("triple-term predicate is not an IRI"));
            }
            Ok(TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        }
        other => Err(fmt(&format!("unknown binding type `{other}`"))),
    }
}

/// Read the required string `value` field of a binding object.
fn binding_value(obj: &[(String, Json)]) -> Result<&str, Error> {
    obj_get(obj, "value")
        .and_then(Json::as_str)
        .ok_or_else(|| fmt("binding has no string `value`"))
}

/// Resolve a literal's datatype: an explicit `datatype` wins; otherwise a
/// language-tagged literal is `rdf:langString` (or `rdf:dirLangString` with a
/// base direction), and a plain literal is `xsd:string`.
fn resolve_datatype(datatype: Option<&str>, has_lang: bool, has_dir: bool) -> String {
    match datatype {
        Some(dt) => dt.to_owned(),
        None if has_lang && has_dir => RDF_DIR_LANGSTRING.to_owned(),
        None if has_lang => RDF_LANGSTRING.to_owned(),
        None => XSD_STRING.to_owned(),
    }
}

/// Build a `Format` error.
fn fmt(msg: &str) -> Error {
    Error::Format(format!("SPARQL-JSON: {msg}"))
}

/// Look up a key in an object's `(key, value)` pairs (first match).
fn obj_get<'a>(obj: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// A minimal JSON value (numbers retained as their lexical form — SPARQL-JSON
/// never needs them numerically).
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl Json {
    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }
}

/// A hand-rolled recursive-descent JSON parser over `&[u8]`.
struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Parse the whole document; trailing non-whitespace is an error.
    fn parse_document(&mut self) -> Result<Json, Error> {
        self.skip_ws();
        let value = self.parse_value()?;
        self.skip_ws();
        if self.pos != self.bytes.len() {
            return Err(fmt("trailing data after JSON value"));
        }
        Ok(value)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Consume `byte` after whitespace, or return a format error with `message`.
    fn expect(&mut self, byte: u8, message: &str) -> Result<(), Error> {
        self.skip_ws();
        if self.peek() != Some(byte) {
            return Err(fmt(message));
        }
        self.pos += 1;
        Ok(())
    }

    /// Finish a streaming object/array entry. Returns `true` when `closing` was consumed.
    fn finish_entry(&mut self, closing: u8, message: &str) -> Result<bool, Error> {
        self.skip_ws();
        match self.peek() {
            Some(b',') => {
                self.pos += 1;
                Ok(false)
            }
            Some(found) if found == closing => {
                self.pos += 1;
                Ok(true)
            }
            _ => Err(fmt(message)),
        }
    }

    /// Validate that the streaming pass consumed the whole document.
    fn finish_document(&mut self) -> Result<(), Error> {
        self.skip_ws();
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(fmt("trailing data after JSON value"))
        }
    }

    /// Parse one object key and its following colon.
    fn parse_object_key(&mut self) -> Result<String, Error> {
        self.skip_ws();
        if self.peek() != Some(b'"') {
            return Err(fmt("expected string key in object"));
        }
        let key = self.parse_string()?;
        self.expect(b':', "expected `:` after object key")?;
        Ok(key)
    }

    /// Decode the first `vars` array in a `head` object, skipping other fields.
    fn parse_head_variables(&mut self) -> Result<Vec<String>, Error> {
        self.expect(b'{', "missing `head` object")?;
        let mut variables = None;
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Vec::new());
        }
        loop {
            let key = self.parse_object_key()?;
            if key == "vars" && variables.is_none() && self.peek_after_ws() == Some(b'[') {
                variables = Some(self.parse_string_array("`head.vars` entry is not a string")?);
            } else {
                self.skip_value()?;
            }
            if self.finish_entry(b'}', "expected `,` or `}` in object")? {
                break;
            }
        }
        Ok(variables.unwrap_or_default())
    }

    /// Parse a string-only JSON array.
    fn parse_string_array(&mut self, item_error: &str) -> Result<Vec<String>, Error> {
        self.expect(b'[', "expected array")?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(items);
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(fmt(item_error));
            }
            items.push(self.parse_string()?);
            if self.finish_entry(b']', "expected `,` or `]` in array")? {
                break;
            }
        }
        Ok(items)
    }

    /// Parse the first `bindings` array in a `results` object through a bounded sink.
    fn parse_bounded_results(
        &mut self,
        variables: &[String],
        row_limit: Option<usize>,
    ) -> BoundedRowsResult {
        self.expect(b'{', "missing `results` object")?;
        let mut result = None;
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Err(fmt("missing `results.bindings` array"));
        }
        loop {
            let key = self.parse_object_key()?;
            if key == "bindings" && result.is_none() {
                result = Some(self.parse_bounded_binding_array(variables, row_limit)?);
            } else {
                self.skip_value()?;
            }
            if self.finish_entry(b'}', "expected `,` or `}` in object")? {
                break;
            }
        }
        result.ok_or_else(|| fmt("missing `results.bindings` array"))
    }

    /// Decode the prefix of a bindings array that fits `row_limit`, then syntax-scan the
    /// suffix without constructing its tree.
    fn parse_bounded_binding_array(
        &mut self,
        variables: &[String],
        row_limit: Option<usize>,
    ) -> BoundedRowsResult {
        self.expect(b'[', "missing `results.bindings` array")?;
        let mut rows = Vec::new();
        let mut truncated = false;
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok((rows, false));
        }
        loop {
            if row_limit.is_none_or(|limit| rows.len() < limit) {
                let binding = self.parse_value()?;
                let row_obj = binding
                    .as_object()
                    .ok_or_else(|| fmt("`results.bindings` entry is not an object"))?;
                let mut row = vec![None; variables.len()];
                for (index, variable) in variables.iter().enumerate() {
                    if let Some(cell) = obj_get(row_obj, variable) {
                        row[index] = Some(decode_binding(cell)?);
                    }
                }
                rows.push(row);
            } else {
                truncated = true;
                self.skip_value()?;
            }
            if self.finish_entry(b']', "expected `,` or `]` in array")? {
                break;
            }
        }
        Ok((rows, truncated))
    }

    /// Peek after insignificant whitespace without changing the parser's final position.
    fn peek_after_ws(&mut self) -> Option<u8> {
        self.skip_ws();
        self.peek()
    }

    /// Syntax-validate and discard one JSON value without building its recursive tree.
    fn skip_value(&mut self) -> Result<(), Error> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(());
                }
                loop {
                    drop(self.parse_object_key()?);
                    self.skip_value()?;
                    if self.finish_entry(b'}', "expected `,` or `}` in object")? {
                        return Ok(());
                    }
                }
            }
            Some(b'[') => {
                self.pos += 1;
                self.skip_ws();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    return Ok(());
                }
                loop {
                    self.skip_value()?;
                    if self.finish_entry(b']', "expected `,` or `]` in array")? {
                        return Ok(());
                    }
                }
            }
            Some(b'"') => {
                drop(self.parse_string()?);
                Ok(())
            }
            Some(b't') => self.parse_lit("true", Json::Bool(true)).map(drop),
            Some(b'f') => self.parse_lit("false", Json::Bool(false)).map(drop),
            Some(b'n') => self.parse_lit("null", Json::Null).map(drop),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number().map(drop),
            _ => Err(fmt("unexpected token while parsing a value")),
        }
    }

    fn parse_value(&mut self) -> Result<Json, Error> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::String(self.parse_string()?)),
            Some(b't') => self.parse_lit("true", Json::Bool(true)),
            Some(b'f') => self.parse_lit("false", Json::Bool(false)),
            Some(b'n') => self.parse_lit("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            _ => Err(fmt("unexpected token while parsing a value")),
        }
    }

    fn parse_lit(&mut self, word: &str, value: Json) -> Result<Json, Error> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(fmt(&format!("expected `{word}`")))
        }
    }

    fn parse_number(&mut self) -> Result<Json, Error> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw = core::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| fmt("non-UTF-8 number"))?;
        Ok(Json::Number(raw.to_owned()))
    }

    fn parse_array(&mut self) -> Result<Json, Error> {
        self.pos += 1; // consume '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(fmt("expected `,` or `]` in array")),
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_object(&mut self) -> Result<Json, Error> {
        self.pos += 1; // consume '{'
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(fmt("expected string key in object"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(fmt("expected `:` after object key"));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(fmt("expected `,` or `}` in object")),
            }
        }
        Ok(Json::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, Error> {
        self.pos += 1; // consume opening '"'
        let mut s = String::new();
        loop {
            // Fast path: a run of plain ASCII (no quote, no backslash) is one
            // `push_str` instead of one `next_utf8_char` + `push` per byte.
            // The escape / terminator / non-ASCII cases below are untouched.
            let start = self.pos;
            while self
                .bytes
                .get(self.pos)
                .is_some_and(|&b| b < 0x80 && b != b'"' && b != b'\\')
            {
                self.pos += 1;
            }
            if self.pos > start {
                // A run of bytes all `< 0x80` is ASCII, hence valid UTF-8; the
                // `expect` documents that invariant rather than trusting it.
                s.push_str(
                    std::str::from_utf8(&self.bytes[start..self.pos])
                        .expect("a pure-ASCII byte run is valid UTF-8"),
                );
            }
            let Some(c) = self.peek() else {
                return Err(fmt("unterminated string"));
            };
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(s);
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(esc) = self.peek() else {
                        return Err(fmt("unterminated escape"));
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\u{0008}'),
                        b'f' => s.push('\u{000C}'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => s.push(self.parse_unicode_escape()?),
                        other => {
                            return Err(fmt(&format!("bad escape \\{}", other as char)));
                        }
                    }
                }
                // A raw multibyte UTF-8 sequence: copy the whole code point.
                _ => {
                    let ch = self.next_utf8_char()?;
                    s.push(ch);
                }
            }
        }
    }

    /// Decode a `\uXXXX` escape (with surrogate-pair handling), positioned just
    /// after the `u`.
    fn parse_unicode_escape(&mut self) -> Result<char, Error> {
        let hi = self.read_hex4()?;
        if (0xD800..=0xDBFF).contains(&hi) {
            // High surrogate: expect a following `\uXXXX` low surrogate.
            if self.peek() != Some(b'\\') {
                return Err(fmt("lone high surrogate"));
            }
            self.pos += 1;
            if self.peek() != Some(b'u') {
                return Err(fmt("lone high surrogate"));
            }
            self.pos += 1;
            let lo = self.read_hex4()?;
            if !(0xDC00..=0xDFFF).contains(&lo) {
                return Err(fmt("invalid low surrogate"));
            }
            let c = 0x1_0000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
            char::from_u32(c).ok_or_else(|| fmt("invalid surrogate pair"))
        } else {
            char::from_u32(hi).ok_or_else(|| fmt("invalid \\u escape"))
        }
    }

    fn read_hex4(&mut self) -> Result<u32, Error> {
        if self.pos + 4 > self.bytes.len() {
            return Err(fmt("truncated \\u escape"));
        }
        let mut value: u32 = 0;
        for _ in 0..4 {
            let c = self.bytes[self.pos];
            let digit = match c {
                b'0'..=b'9' => u32::from(c - b'0'),
                b'a'..=b'f' => u32::from(c - b'a' + 10),
                b'A'..=b'F' => u32::from(c - b'A' + 10),
                _ => return Err(fmt("non-hex digit in \\u escape")),
            };
            value = value * 16 + digit;
            self.pos += 1;
        }
        Ok(value)
    }

    /// Consume one UTF-8 code point starting at `pos` in O(1).
    ///
    /// Determines the code-point width from the lead byte's bit pattern, slices
    /// exactly those 1–4 bytes, and validates only that small slice — avoiding
    /// the O(N²) cost of validating the entire remaining buffer on every call.
    fn next_utf8_char(&mut self) -> Result<char, Error> {
        let lead = self
            .bytes
            .get(self.pos)
            .copied()
            .ok_or_else(|| fmt("unterminated string"))?;
        // Determine the encoded width from the lead byte.
        let width = if lead < 0x80 {
            1usize
        } else if lead & 0xE0 == 0xC0 {
            2
        } else if lead & 0xF0 == 0xE0 {
            3
        } else if lead & 0xF8 == 0xF0 {
            4
        } else {
            return Err(fmt("invalid UTF-8 lead byte in string"));
        };
        let end = self.pos + width;
        if end > self.bytes.len() {
            return Err(fmt("truncated UTF-8 sequence in string"));
        }
        // Validate and decode only the exact code-point slice.
        let slice = &self.bytes[self.pos..end];
        let s = core::str::from_utf8(slice).map_err(|_| fmt("invalid UTF-8 sequence in string"))?;
        let ch = s.chars().next().ok_or_else(|| fmt("empty UTF-8 slice"))?;
        self.pos = end;
        Ok(ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::to_json;
    use purrdf_core::SparqlResult;

    fn parse_string_at(input: &[u8]) -> Result<(String, usize), Error> {
        let mut parser = JsonParser::new(input);
        let value = parser.parse_string()?;
        Ok((value, parser.pos))
    }

    /// The ASCII bulk-copy fast path must splice correctly around escapes,
    /// raw multibyte UTF-8, and the terminator, leaving `pos` just past the
    /// closing quote so the enclosing parser resumes at the right byte.
    #[test]
    fn parse_string_mixed_ascii_escape_multibyte() {
        let cases: [(&[u8], &str); 9] = [
            (b"\"\"", ""),
            (b"\"plain ascii\"", "plain ascii"),
            (b"\"a\\\"b\"", "a\"b"),
            (
                b"\"tab\\tnl\\ncr\\r\\\\ \\/ \\b\\f\"",
                "tab\tnl\ncr\r\\ / \u{8}\u{c}",
            ),
            (b"\"\\u00e9\\ud83d\\udc31\"", "\u{e9}\u{1f431}"),
            (
                "\"caf\u{e9} \u{4e2d}\u{6587} \u{1f431}\"".as_bytes(),
                "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
            ),
            (
                "\"ascii \u{e9} \\\" more \\u0041 \u{1f431}\\n tail\"".as_bytes(),
                "ascii \u{e9} \" more A \u{1f431}\n tail",
            ),
            (b"\"raw\x01control\"", "raw\u{1}control"),
            (b"\"end\" trailing", "end"),
        ];
        for (input, expected) in cases {
            let (value, pos) = parse_string_at(input).expect("string parses");
            assert_eq!(value, expected, "{input:?}");
            let close = input
                .iter()
                .rposition(|&b| b == b'"')
                .expect("closing quote");
            assert!(
                pos == close + 1 || input.ends_with(b" trailing"),
                "pos {pos} for {input:?}"
            );
        }
        assert_eq!(parse_string_at(b"\"end\" trailing").expect("parses").1, 5);
    }

    #[test]
    fn parse_string_errors_unchanged_after_ascii_run() {
        assert!(parse_string_at(b"\"no terminator").is_err());
        assert!(parse_string_at(b"\"ascii then bad escape \\q\"").is_err());
        assert!(parse_string_at(b"\"ascii then \\").is_err());
        assert!(parse_string_at(b"\"ascii then \xff\"").is_err());
        assert!(parse_string_at(b"\"ascii then truncated \xe4\xb8").is_err());
    }

    /// Provenance round-trip: what [`crate::json::to_json`] writes under a namespace,
    /// [`provenance_from_json`] reads back — the writer no longer emits
    /// something nothing can decode.
    #[test]
    fn provenance_round_trips_through_json() {
        let result = SparqlResult::Boolean(true);
        let provenance = ResultProvenance {
            query_hash: Some("deadbeef".to_owned()),
            engine: Some("purrdf-sparql-eval".to_owned()),
            solutions: vec![
                SolutionProvenance {
                    sources: vec!["http://example.org/g1".to_owned()],
                },
                SolutionProvenance { sources: vec![] },
            ],
        };
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("valid namespace");
        let outcome = to_json(&result, &provenance, Some(&namespace)).expect("serializes");

        let decoded =
            provenance_from_json(&outcome.bytes, &namespace).expect("provenance decodes back");
        assert_eq!(decoded, provenance);
    }

    /// A document with no member under the caller's namespace prefix — because
    /// the writer never populated one, or a different namespace was used —
    /// decodes to the empty provenance rather than erroring.
    #[test]
    fn absent_provenance_member_decodes_to_default() {
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes");
        assert!(decoded.is_empty());
    }

    /// Namespace-by-IRI pin (false NEGATIVE, mirroring
    /// [`crate::xml_read::tests::provenance_reads_correctly_under_an_alternate_prefix_for_the_same_namespace`]):
    /// a document that records the caller's OWN namespace IRI under a top-level
    /// key spelled with a DIFFERENT prefix than this crate's own writer happens
    /// to use (`p` instead of `prov`) must still decode — JSON provenance
    /// identity is IRI-based (the `"namespace"` field), not top-level-key-based.
    #[test]
    fn provenance_reads_correctly_under_an_alternate_prefix_for_the_same_namespace() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true,"p":{
            "namespace":"https://example.org/ns/prov#",
            "queryForm":"ask",
            "queryHash":"deadbeef",
            "engine":"purrdf-sparql-eval"
        }}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes");
        assert_eq!(
            decoded,
            ResultProvenance {
                query_hash: Some("deadbeef".to_owned()),
                engine: Some("purrdf-sparql-eval".to_owned()),
                solutions: Vec::new(),
            }
        );
    }

    /// Namespace-by-IRI pin (false POSITIVE, mirroring
    /// [`crate::xml_read::tests::foreign_namespace_under_the_writers_own_prefix_is_not_read_as_provenance`]):
    /// a document that reuses this crate's writer's OWN prefix spelling
    /// (`prov`) as its top-level key, but records an UNRELATED namespace IRI —
    /// here the actual W3C PROV namespace (`http://www.w3.org/ns/prov#`), the
    /// single most common real-world `prov:` binding — must NOT be read as the
    /// caller's provenance extension just because the top-level key spelling
    /// matches; only the recorded `"namespace"` IRI identifies it. This is the
    /// direction the old, tautological `provenance_under_a_different_namespace_is_not_found`
    /// test (which varied prefix AND iri together) could never actually observe.
    #[test]
    fn foreign_namespace_under_the_writers_own_prefix_is_not_read_as_provenance() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"{"head":{},"boolean":true,"prov":{
            "namespace":"http://www.w3.org/ns/prov#",
            "queryForm":"ask",
            "queryHash":"deadbeef"
        }}"#;
        let decoded = provenance_from_json(doc, &namespace).expect("decodes without error");
        assert!(
            decoded.is_empty(),
            "a `prov`-keyed member recording the W3C PROV namespace must not be mistaken \
             for this caller's own `prov` namespace extension"
        );
    }

    #[test]
    fn reads_select_with_mixed_terms() {
        let srj = r#"{
          "head": { "vars": [ "s", "name", "label", "age" ] },
          "results": { "bindings": [
            {
              "s": { "type": "uri", "value": "http://example.org/s" },
              "name": { "type": "literal", "value": "Ada" },
              "label": { "type": "literal", "value": "bonjour", "xml:lang": "fr" },
              "age": { "type": "literal", "value": "42",
                       "datatype": "http://www.w3.org/2001/XMLSchema#integer" }
            },
            {
              "s": { "type": "bnode", "value": "b0" }
            }
          ] }
        }"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(parsed.variables, vec!["s", "name", "label", "age"]);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Iri("http://example.org/s".to_owned()))
        );
        assert_eq!(
            parsed.rows[0][1],
            Some(TermValue::Literal {
                lexical_form: "Ada".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            })
        );
        assert_eq!(
            parsed.rows[0][2],
            Some(TermValue::Literal {
                lexical_form: "bonjour".to_owned(),
                datatype: RDF_LANGSTRING.to_owned(),
                language: Some("fr".to_owned()),
                direction: None,
            })
        );
        assert_eq!(
            parsed.rows[0][3],
            Some(TermValue::Literal {
                lexical_form: "42".to_owned(),
                datatype: "http://www.w3.org/2001/XMLSchema#integer".to_owned(),
                language: None,
                direction: None,
            })
        );
        // Second row: only `s` bound (a bnode), the rest unbound.
        assert_eq!(
            parsed.rows[1][0],
            Some(TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            })
        );
        assert_eq!(parsed.rows[1][1], None);
        assert_eq!(parsed.rows[1][3], None);
    }

    #[test]
    fn bounded_reader_stops_before_the_limit_plus_one_binding() {
        // `results` deliberately precedes `head`: the two-pass reader may not rely on the
        // conventional field order when deriving the two-column row ceiling.
        let srj = br#"{
          "results":{"bindings":[
            {"x":{"type":"uri","value":"http://example.org/0"}},
            {"x":{"type":"uri","value":"http://example.org/1"}},
            {"x":{"type":"uri","value":"http://example.org/2"}}
          ]},
          "head":{"vars":["x","y"]}
        }"#;

        let bounded = from_json_bounded(srj, 4).expect("two two-cell rows fit");
        assert_eq!(bounded.solutions.variables, ["x", "y"]);
        assert_eq!(bounded.solutions.rows.len(), 2);
        assert!(bounded.truncated, "the third binding is the overflow proof");

        let exact = from_json_bounded(srj, 6).expect("the exact boundary is inclusive");
        assert_eq!(exact.solutions.rows.len(), 3);
        assert!(!exact.truncated);
        assert_eq!(exact.solutions, from_json(srj).expect("ordinary decode"));
    }

    #[test]
    fn bounded_reader_does_not_invent_a_row_bound_for_zero_columns() {
        let srj = br#"{"head":{"vars":[]},"results":{"bindings":[{},{},{}]}}"#;
        let bounded = from_json_bounded(srj, 0).expect("unit rows consume zero cells");
        assert_eq!(bounded.solutions.rows.len(), 3);
        assert!(!bounded.truncated);
    }

    #[test]
    fn reads_directional_literal() {
        // `its:dir` is the SPARQL 1.2 Query Results spec spelling (see
        // `crate::json`'s module docs for the fixture evidence).
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"שלום","xml:lang":"he","its:dir":"rtl"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "שלום".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("he".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// The bare `dir` spelling is tolerated for interop with producers that
    /// predate the SPARQL 1.2 `its:dir` spelling.
    #[test]
    fn tolerates_legacy_bare_dir_spelling() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"hello","xml:lang":"en","dir":"ltr"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
        );
    }

    /// When both spellings are present, `its:dir` — the spec spelling — takes
    /// priority over the legacy bare `dir`.
    #[test]
    fn its_dir_takes_priority_over_bare_dir() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"hello","xml:lang":"en","its:dir":"ltr","dir":"rtl"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
        );
    }

    #[test]
    fn reads_triple_term() {
        let srj = r#"{"head":{"vars":["t"]},"results":{"bindings":[
          {"t":{"type":"triple","value":{
            "subject":{"type":"uri","value":"http://ex/s"},
            "predicate":{"type":"uri","value":"http://ex/p"},
            "object":{"type":"uri","value":"http://ex/o"}}}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Triple {
                s: Box::new(TermValue::Iri("http://ex/s".to_owned())),
                p: Box::new(TermValue::Iri("http://ex/p".to_owned())),
                o: Box::new(TermValue::Iri("http://ex/o".to_owned())),
            })
        );
    }

    #[test]
    fn reads_ask_boolean() {
        assert!(from_json_boolean(br#"{"head":{},"boolean":true}"#).expect("ask"));
        assert!(!from_json_boolean(br#"{"head":{},"boolean":false}"#).expect("ask"));
    }

    #[test]
    fn select_reader_rejects_ask_document() {
        let err = from_json(br#"{"head":{},"boolean":true}"#).unwrap_err();
        assert!(matches!(err, Error::Format(_)));
    }

    #[test]
    fn handles_escapes_and_unicode() {
        let srj = r#"{"head":{"vars":["x"]},"results":{"bindings":[
          {"x":{"type":"literal","value":"a\"b\\c\nA😀"}}]}}"#;
        let parsed = from_json(srj.as_bytes()).expect("parse");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, "a\"b\\c\nA😀");
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(from_json(br#"{"head":{"vars":[]},"results":{"bindings":[]}} oops"#).is_err());
    }

    /// Guard against the O(N²) regression: a long multibyte string must parse
    /// correctly and the decoded value must round-trip.
    #[test]
    fn long_multibyte_string_parses_correctly() {
        // Build a large string of multibyte chars: mix of 2-byte (é, U+00E9)
        // and 3-byte (你, U+4F60) code points so all width branches are hit.
        let repeated_2byte = "é".repeat(1_500); // 3 000 bytes
        let repeated_3byte = "你".repeat(1_000); // 3 000 bytes
        let long_value = format!("{repeated_2byte}{repeated_3byte}");

        let srj = format!(
            r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"literal","value":"{long_value}"}}}}]}}}}"#
        );
        let parsed = from_json(srj.as_bytes()).expect("parse long multibyte string");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, long_value, "decoded value must round-trip");
    }

    /// A 4-byte UTF-8 sequence (emoji, U+1F600) must decode correctly through
    /// the lead-byte-width path.
    #[test]
    fn four_byte_utf8_sequence_parses() {
        // U+1F600 GRINNING FACE encodes as 4 UTF-8 bytes.
        let val = "😀".repeat(500);
        let srj = format!(
            r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"literal","value":"{val}"}}}}]}}}}"#
        );
        let parsed = from_json(srj.as_bytes()).expect("parse 4-byte sequences");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, val);
    }

    /// Malformed UTF-8 bytes inside a JSON string must yield a parse Error, not
    /// a panic.  We inject a raw invalid continuation byte (0x80) that is not
    /// preceded by a valid lead byte.
    #[test]
    fn malformed_utf8_yields_error_not_panic() {
        // Construct bytes: valid JSON prefix, then a bare 0x80 continuation byte
        // (invalid as a lead byte), then closing JSON.
        let prefix =
            br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"literal","value":""#;
        let suffix = br#""}}]}}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(prefix);
        // Insert the invalid lead byte just before the closing quote.
        bytes.push(0x80); // bare continuation — not a valid UTF-8 lead
        bytes.extend_from_slice(suffix);
        let result = from_json(&bytes);
        assert!(result.is_err(), "expected Err for invalid UTF-8, got Ok");
        assert!(
            matches!(result.unwrap_err(), Error::Format(_)),
            "error must be Error::Format"
        );
    }
}

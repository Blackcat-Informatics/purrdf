// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! The JSON grammar is the workspace's one reader, [`purrdf_lex::json`]: the
//! tree readers build its [`Value`], and the bounded reader drives its pull
//! [`Reader`] so a skipped member is syntax-checked without being built. No
//! `std::io`, so the crate stays wasm-clean.
//!
//! # Nesting depth
//!
//! Every walk over input nesting — reading a JSON value, skipping one, dropping a
//! read value, and decoding an RDF 1.2 triple-term binding — runs over an
//! explicit heap stack rather than the machine stack, and the reader sets no
//! container-depth cap. A document nested to any
//! depth is therefore parsed (or refused for a syntax or shape error) with a
//! machine-stack footprint independent of that depth, which is what keeps a remote
//! `SERVICE` endpoint's deeply nested answer from overflowing the stack.

use std::borrow::Cow;

use purrdf_core::TermBox;
use purrdf_core::{BlankScope, RdfTextDirection, TermValue};
use purrdf_lex::json::{self, Event, Limits, Object, Reader, Value};
use purrdf_lex::terminals::skip_ws;

use crate::error::Error;
use crate::model::{ProvenanceNamespace, ResultProvenance, SolutionProvenance};

use purrdf_core::datatype::XSD_STRING;
use purrdf_core::vocab::language_datatype_iri;

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
    let doc = read_document(bytes)?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    if obj.get("boolean").is_some() {
        return Err(fmt(
            "expected SELECT results, got an ASK (boolean) document",
        ));
    }
    let head = obj
        .get("head")
        .and_then(Value::as_object)
        .ok_or_else(|| fmt("missing `head` object"))?;
    let variables = match head.get("vars") {
        Some(Value::Array(items)) => items
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

    let results = obj
        .get("results")
        .and_then(Value::as_object)
        .ok_or_else(|| fmt("missing `results` object"))?;
    let bindings = match results.get("bindings") {
        Some(Value::Array(items)) => items.as_slice(),
        _ => return Err(fmt("missing `results.bindings` array")),
    };

    let rows = bindings
        .iter()
        .map(|binding| decode_row(binding, &variables))
        .collect::<Result<Vec<_>, _>>()?;
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
    let mut reader = Reader::from_slice(bytes, LIMITS).map_err(syntax)?;
    open_object(&mut reader, "top level is not an object")?;
    let mut variables = None;
    while let Some(key) = next_key(&mut reader)? {
        if key == "head" && variables.is_none() {
            variables = Some(head_variables(&mut reader)?);
        } else {
            reader.skip_value().map_err(syntax)?;
        }
    }
    reader.finish().map_err(syntax)?;
    variables.ok_or_else(|| fmt("missing `head` object"))
}

/// Second bounded-decoder pass: locate `results.bindings`, materializing only the prefix
/// admitted by `row_limit`.
fn scan_bindings(
    bytes: &[u8],
    variables: &[String],
    row_limit: Option<usize>,
) -> BoundedRowsResult {
    let mut reader = Reader::from_slice(bytes, LIMITS).map_err(syntax)?;
    open_object(&mut reader, "top level is not an object")?;
    let mut result = None;
    let mut saw_boolean = false;
    while let Some(key) = next_key(&mut reader)? {
        match &*key {
            "boolean" => {
                saw_boolean = true;
                reader.skip_value().map_err(syntax)?;
            }
            "results" if result.is_none() => {
                result = Some(bounded_results(&mut reader, variables, row_limit)?);
            }
            _ => {
                reader.skip_value().map_err(syntax)?;
            }
        }
    }
    reader.finish().map_err(syntax)?;
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
    let doc = read_document(bytes)?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    match obj.get("boolean") {
        Some(Value::Bool(b)) => Ok(*b),
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
    let doc = read_document(bytes)?;
    let obj = doc
        .as_object()
        .ok_or_else(|| fmt("top level is not an object"))?;
    let iri = namespace.iri();
    let Some(member_obj) = obj.iter().find_map(|(_, value)| {
        let candidate = value.as_object()?;
        let recorded = candidate.get("namespace").and_then(Value::as_str)?;
        (recorded == iri).then_some(candidate)
    }) else {
        return Ok(ResultProvenance::default());
    };
    let query_hash = member_obj
        .get("queryHash")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let engine = member_obj
        .get("engine")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let solutions = match member_obj.get("solutions") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                let item_obj = item
                    .as_object()
                    .ok_or_else(|| fmt("provenance solution is not an object"))?;
                let sources = match item_obj.get("sources") {
                    Some(Value::Array(values)) => values
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

/// Decode one SPARQL-JSON binding object into a [`TermValue`].
///
/// An RDF 1.2 triple term nests further binding objects under its `subject`,
/// `predicate` and `object` members to any depth. They are decoded over an explicit
/// heap stack of partly decoded triple terms, so the machine stack used does not
/// grow with the nesting depth. Components are decoded in the order `subject`,
/// `predicate`, `object`, and a triple term's predicate is checked to be an IRI only
/// after its object has been decoded, so the first error reported for a malformed
/// document is the one a depth-first descent in that order meets first.
fn decode_binding(value: &Value) -> Result<TermValue, Error> {
    let mut open: Vec<OpenTriple<'_>> = Vec::new();
    let mut next = value;
    loop {
        let mut term = match decode_binding_node(next)? {
            BindingNode::Term(term) => term,
            BindingNode::Triple(inner) => {
                next = inner
                    .get("subject")
                    .ok_or_else(|| fmt("triple has no subject"))?;
                open.push(OpenTriple {
                    inner,
                    subject: None,
                    predicate: None,
                });
                continue;
            }
        };
        // Hand the finished term to the innermost open triple term; every triple term
        // it completes is handed on to the one enclosing it in turn.
        loop {
            let Some(triple) = open.last_mut() else {
                return Ok(term);
            };
            if triple.subject.is_none() {
                triple.subject = Some(term);
                next = triple
                    .inner
                    .get("predicate")
                    .ok_or_else(|| fmt("triple has no predicate"))?;
                break;
            }
            if triple.predicate.is_none() {
                triple.predicate = Some(term);
                next = triple
                    .inner
                    .get("object")
                    .ok_or_else(|| fmt("triple has no object"))?;
                break;
            }
            let innermost = open.pop();
            let OpenTriple {
                subject, predicate, ..
            } = innermost.expect("the innermost open triple term was just read");
            let (Some(s), Some(p)) = (subject, predicate) else {
                unreachable!(
                    "an open triple term holding its object holds its subject and predicate"
                );
            };
            if !matches!(p, TermValue::Iri(_)) {
                return Err(fmt("triple-term predicate is not an IRI"));
            }
            term = TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(term),
            };
        }
    }
}

/// A triple-term binding whose components are still being decoded: its `value`
/// object and the components decoded so far, in `subject`, `predicate` order.
struct OpenTriple<'a> {
    inner: &'a Object,
    subject: Option<TermValue>,
    predicate: Option<TermValue>,
}

/// One binding object read at a single level: a finished term, or the `value`
/// object of a triple term whose components remain to be decoded.
enum BindingNode<'a> {
    Term(TermValue),
    Triple(&'a Object),
}

/// Decode the binding object `value` down to, but not into, the components of a
/// triple term.
fn decode_binding_node(value: &Value) -> Result<BindingNode<'_>, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| fmt("binding is not an object"))?;
    let ty = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| fmt("binding has no string `type`"))?;
    match ty {
        "uri" => {
            let v = binding_value(obj)?;
            Ok(BindingNode::Term(TermValue::Iri(v.to_owned())))
        }
        "bnode" => {
            let v = binding_value(obj)?;
            Ok(BindingNode::Term(TermValue::Blank {
                label: v.to_owned(),
                scope: BlankScope::DEFAULT,
            }))
        }
        "literal" | "typed-literal" => {
            let v = binding_value(obj)?;
            let language = obj.get("xml:lang").and_then(Value::as_str);
            // A tag arriving in a DOCUMENT is parsed input, held to the same
            // grammar as every other parsed tag in the workspace. Refusing here
            // is what stops a federated `SERVICE` response from laundering an
            // unserializable tag through this reader and back out a writer.
            if let Some(detail) = language.and_then(crate::error::language_tag_refusal) {
                return Err(fmt(&detail));
            }
            // `its:dir` (the ITS — Internationalization Tag Set — namespace
            // convention) is the spelling the SPARQL 1.2 Query Results
            // specification uses for RDF 1.2 base direction — see
            // [`crate::json`]'s module docs for the fixture evidence. The bare
            // `dir` spelling is tolerated too, for interop with producers that
            // predate the SPARQL 1.2 spelling.
            let direction = match obj
                .get("its:dir")
                .or_else(|| obj.get("dir"))
                .and_then(Value::as_str)
            {
                Some(token) => Some(
                    RdfTextDirection::from_str_token(token)
                        .ok_or_else(|| fmt(&format!("unknown base direction `{token}`")))?,
                ),
                None => None,
            };
            let datatype = obj.get("datatype").and_then(Value::as_str);
            let datatype = resolve_datatype(datatype, language.is_some(), direction.is_some());
            Ok(BindingNode::Term(TermValue::Literal {
                lexical_form: v.to_owned(),
                datatype,
                language: language.map(str::to_owned),
                direction,
            }))
        }
        "triple" => obj
            .get("value")
            .and_then(Value::as_object)
            .map(BindingNode::Triple)
            .ok_or_else(|| fmt("triple binding has no object `value`")),
        other => Err(fmt(&format!("unknown binding type `{other}`"))),
    }
}

/// Read the required string `value` field of a binding object.
fn binding_value(obj: &Object) -> Result<&str, Error> {
    obj.get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| fmt("binding has no string `value`"))
}

/// Resolve a literal's datatype: an explicit `datatype` wins; otherwise a
/// language-tagged literal is `rdf:langString` (or `rdf:dirLangString` with a
/// base direction), and a plain literal is `xsd:string`.
fn resolve_datatype(datatype: Option<&str>, has_lang: bool, has_dir: bool) -> String {
    match datatype {
        Some(dt) => dt.to_owned(),
        None if has_lang => language_datatype_iri(has_dir).to_owned(),
        None => XSD_STRING.to_owned(),
    }
}

/// Build a `Format` error.
/// Build a `Format` error.
fn fmt(msg: &str) -> Error {
    Error::Format(format!("SPARQL-JSON: {msg}"))
}

/// A refusal from the JSON reader, named with its byte offset.
fn syntax(error: json::Error) -> Error {
    fmt(&error.to_string())
}

/// The reader's bounds: no container-depth cap. Every walk over the document —
/// the reader's own, the tree's drop, [`decode_binding`] — runs over a heap
/// stack, so nesting costs memory proportional to the input and never machine
/// stack, and a triple term may nest as deep as the endpoint wrote it.
const LIMITS: Limits = Limits::with_depth(usize::MAX);

/// The whole document as a JSON tree.
fn read_document(bytes: &[u8]) -> Result<Value, Error> {
    json::read_slice(bytes, LIMITS).map_err(syntax)
}

/// Decode one `results.bindings` entry into a dense row over `variables`.
fn decode_row(binding: &Value, variables: &[String]) -> Result<BindingRow, Error> {
    let row_obj = binding
        .as_object()
        .ok_or_else(|| fmt("`results.bindings` entry is not an object"))?;
    let mut row = vec![None; variables.len()];
    for (index, variable) in variables.iter().enumerate() {
        if let Some(cell) = row_obj.get(variable) {
            row[index] = Some(decode_binding(cell)?);
        }
    }
    Ok(row)
}

/// Read the next value, requiring it to open an object; `message` otherwise.
fn open_object(reader: &mut Reader<'_>, message: &str) -> Result<(), Error> {
    match reader.next_event().map_err(syntax)? {
        Event::BeginObject { .. } => Ok(()),
        _ => Err(fmt(message)),
    }
}

/// Read the next value, requiring it to open an array; `message` otherwise.
fn open_array(reader: &mut Reader<'_>, message: &str) -> Result<(), Error> {
    match reader.next_event().map_err(syntax)? {
        Event::BeginArray { .. } => Ok(()),
        _ => Err(fmt(message)),
    }
}

/// The next member name of the open object, decoded, or `None` at its `}`.
fn next_key<'a>(reader: &mut Reader<'a>) -> Result<Option<Cow<'a, str>>, Error> {
    reader
        .next_key()
        .map_err(syntax)?
        .map(|name| name.decode().map_err(syntax))
        .transpose()
}

/// Whether the next value, past insignificant whitespace, starts with `byte`.
fn next_starts_with(reader: &Reader<'_>, byte: u8) -> bool {
    let text = reader.text().as_bytes();
    text.get(skip_ws(text, reader.offset())) == Some(&byte)
}

/// Decode the first `vars` array in a `head` object, skipping other fields.
fn head_variables(reader: &mut Reader<'_>) -> Result<Vec<String>, Error> {
    open_object(reader, "missing `head` object")?;
    let mut variables = None;
    while let Some(key) = next_key(reader)? {
        if key == "vars" && variables.is_none() && next_starts_with(reader, b'[') {
            variables = Some(string_array(reader, "`head.vars` entry is not a string")?);
        } else {
            reader.skip_value().map_err(syntax)?;
        }
    }
    Ok(variables.unwrap_or_default())
}

/// Read a string-only JSON array.
fn string_array(reader: &mut Reader<'_>, item_error: &str) -> Result<Vec<String>, Error> {
    open_array(reader, "expected array")?;
    let mut items = Vec::new();
    while reader.next_item().map_err(syntax)? {
        match reader.next_event().map_err(syntax)? {
            Event::String(item) => items.push(item.decode().map_err(syntax)?.into_owned()),
            _ => return Err(fmt(item_error)),
        }
    }
    Ok(items)
}

/// Read the first `bindings` array in a `results` object through a bounded sink.
fn bounded_results(
    reader: &mut Reader<'_>,
    variables: &[String],
    row_limit: Option<usize>,
) -> BoundedRowsResult {
    open_object(reader, "missing `results` object")?;
    let mut result = None;
    while let Some(key) = next_key(reader)? {
        if key == "bindings" && result.is_none() {
            result = Some(bounded_binding_array(reader, variables, row_limit)?);
        } else {
            reader.skip_value().map_err(syntax)?;
        }
    }
    result.ok_or_else(|| fmt("missing `results.bindings` array"))
}

/// Decode the prefix of a bindings array that fits `row_limit`, then syntax-scan the
/// suffix without constructing its tree.
fn bounded_binding_array(
    reader: &mut Reader<'_>,
    variables: &[String],
    row_limit: Option<usize>,
) -> BoundedRowsResult {
    open_array(reader, "missing `results.bindings` array")?;
    let mut rows = Vec::new();
    let mut truncated = false;
    while reader.next_item().map_err(syntax)? {
        if row_limit.is_none_or(|limit| rows.len() < limit) {
            let binding = reader.read_value().map_err(syntax)?;
            rows.push(decode_row(&binding, variables)?);
        } else {
            truncated = true;
            reader.skip_value().map_err(syntax)?;
        }
    }
    Ok((rows, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::to_json;
    use purrdf_core::SparqlResult;
    use purrdf_core::TermBox;
    use purrdf_core::vocab::rdf::{
        DIR_LANG_STRING as RDF_DIR_LANGSTRING, LANG_STRING as RDF_LANGSTRING,
    };

    /// A `SELECT` document binding `?x` to a literal whose `value` member is
    /// the JSON string `string` (quotes included, escapes as written).
    fn literal_document(string: &[u8]) -> Vec<u8> {
        let mut document =
            br#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":{"type":"literal","value":"#
                .to_vec();
        document.extend_from_slice(string);
        document.extend_from_slice(b"}}]}}");
        document
    }

    /// The lexical form `from_json` reads for [`literal_document`].
    fn literal_value(string: &[u8]) -> Result<String, Error> {
        let parsed = from_json(&literal_document(string))?;
        let Some(TermValue::Literal { lexical_form, .. }) = parsed.rows[0][0].clone() else {
            panic!("expected a literal");
        };
        Ok(lexical_form)
    }

    /// A raw C0 control inside a JSON string is refused (RFC 8259 §7), both in a
    /// value that is decoded and in one that is only skipped.
    #[test]
    fn a_raw_control_character_in_a_string_is_refused() {
        for raw in [0x01_u8, b'\t', b'\n', 0x1F] {
            let mut string = b"\"a".to_vec();
            string.push(raw);
            string.extend_from_slice(b"b\"");
            let error = literal_value(&string).expect_err("a raw control is refused");
            assert!(
                matches!(&error, Error::Format(message)
                    if message.contains("a control character in a string must be escaped")),
                "{raw:#04X}: {error:?}"
            );
            assert!(
                from_json_bounded(&literal_document(&string), u64::MAX).is_err(),
                "{raw:#04X}"
            );
            // And in a member the reader only skips.
            let mut skipped = br#"{"head":{"vars":[],"note":"a"#.to_vec();
            skipped.push(raw);
            skipped.extend_from_slice(br#"b"},"boolean":true}"#);
            assert!(from_json_boolean(&skipped).is_err(), "{raw:#04X}");
            assert!(from_json_bounded(&skipped, u64::MAX).is_err(), "{raw:#04X}");
        }
    }

    /// The valid neighbours: the same scalars written as escapes parse, and DEL
    /// (not a C0 control) is lawful raw.
    #[test]
    fn an_escaped_control_character_still_parses() {
        assert_eq!(
            literal_value(br#""a\u0001b""#).expect("escaped U+0001"),
            "a\u{1}b"
        );
        assert_eq!(
            literal_value(br#""a\tb\nc\u001f""#).expect("escaped controls"),
            "a\tb\nc\u{1f}"
        );
        assert_eq!(literal_value(b"\"a\x7fb\"").expect("raw DEL"), "a\u{7f}b");
        let skipped = br#"{"head":{"vars":[],"note":"a\u0001b"},"boolean":true}"#;
        assert!(from_json_boolean(skipped).expect("an escaped control in a skipped member"));
    }

    /// Strings decode across escapes, raw multibyte UTF-8 and surrogate pairs,
    /// in the tree reader and the bounded reader alike.
    #[test]
    fn strings_decode_escapes_and_multibyte_text() {
        let cases: [(&[u8], &str); 8] = [
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
            (b"\"raw\\u0001control\"", "raw\u{1}control"),
        ];
        for (string, expected) in cases {
            assert_eq!(
                literal_value(string).expect("parses"),
                expected,
                "{string:?}"
            );
            let bounded = from_json_bounded(&literal_document(string), u64::MAX).expect("parses");
            assert_eq!(
                bounded.solutions,
                from_json(&literal_document(string)).expect("parses"),
                "{string:?}"
            );
        }
    }

    /// A malformed string is refused: unterminated, a bad escape, a lone
    /// surrogate, invalid UTF-8.
    #[test]
    fn malformed_strings_are_refused() {
        for string in [
            &b"\"no terminator"[..],
            b"\"ascii then bad escape \\q\"",
            b"\"ascii then \\",
            b"\"lone high \\ud83d\"",
            b"\"ascii then \xff\"",
            b"\"ascii then truncated \xe4\xb8\"",
        ] {
            assert!(literal_value(string).is_err(), "{string:?}");
            assert!(
                from_json_bounded(&literal_document(string), u64::MAX).is_err(),
                "{string:?}"
            );
        }
    }

    /// RFC 8259 §6: a number is `[-] int [frac] [exp]`. A run of number
    /// characters outside that grammar is refused wherever it stands, in a
    /// skipped member included.
    #[test]
    fn a_number_outside_the_grammar_is_refused() {
        let document = br#"{"head":{"vars":[]},"extra":1-2+e,"boolean":true}"#;
        assert!(from_json_boolean(document).is_err());
        let select = br#"{"head":{"vars":[]},"extra":1-2+e,"results":{"bindings":[]}}"#;
        assert!(from_json(select).is_err());
        assert!(from_json_bounded(select, u64::MAX).is_err());
    }

    /// The valid neighbour: a signed fraction with a signed exponent is a number.
    #[test]
    fn a_number_inside_the_grammar_is_accepted() {
        let document = br#"{"head":{"vars":[]},"extra":-1.2e+3,"boolean":true}"#;
        assert!(from_json_boolean(document).expect("a lawful number"));
        let select = br#"{"head":{"vars":[]},"extra":-1.2e+3,"results":{"bindings":[]}}"#;
        let empty = ParsedSolutions {
            variables: Vec::new(),
            rows: Vec::new(),
        };
        assert_eq!(from_json(select), Ok(empty.clone()));
        assert_eq!(
            from_json_bounded(select, u64::MAX).map(|bounded| bounded.solutions),
            Ok(empty)
        );
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
                s: TermBox::new(TermValue::Iri("http://ex/s".to_owned())),
                p: TermBox::new(TermValue::Iri("http://ex/p".to_owned())),
                o: TermBox::new(TermValue::Iri("http://ex/o".to_owned())),
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

    /// Tags the `LANGTAG` grammar refuses, and the neighbouring ones it must
    /// still take. Shared with the XML reader's twin and with the
    /// `STRLANG`/`STRLANGDIR` gate in `purrdf-sparql-eval` — one accept set,
    /// whichever door a tag arrives through.
    const REFUSED_TAGS: &[&str] = &["en us", "1", "9-9", "123-456", "en-", "-", "!!!"];
    const ACCEPTED_TAGS: &[&str] = &[
        "en",
        "en-US",
        "zh-Hans-CN",
        "de-CH-x-phonebk",
        "i-enochian",
        "x-purrdf-afrikaans",
        "x-gmeow-english",
        "en-fr-jura",
        "fr-be-fbcl",
    ];

    fn srj_with_lang(tag: &str) -> Vec<u8> {
        format!(
            "{{\"head\":{{\"vars\":[\"l\"]}},\"results\":{{\"bindings\":\
             [{{\"l\":{{\"type\":\"literal\",\"value\":\"x\",\"xml:lang\":\"{tag}\"}}}}]}}}}"
        )
        .into_bytes()
    }

    /// A results DOCUMENT is parsed input. A federated `SERVICE` response is
    /// how a hostile or sloppy endpoint's tag gets in, and before this gate it
    /// was copied verbatim into a `TermValue` and written straight back out.
    #[test]
    fn a_refused_language_tag_in_a_document_is_refused_with_its_diagnostic_code() {
        for tag in REFUSED_TAGS {
            let error = from_json(&srj_with_lang(tag))
                .expect_err("a tag the grammar refuses must not decode");
            let Error::Format(message) = &error else {
                panic!("expected Error::Format for {tag:?}, got {error:?}");
            };
            assert!(
                message.starts_with("SPARQL-JSON: invalid language tag "),
                "the refusal must name the format and the tag: {message}"
            );
            assert!(
                message.contains(&format!("`{tag}`")),
                "the refusal must quote the offending tag verbatim, since this \
                 crate's error carries no position: {message}"
            );
            assert!(
                message.contains("(langtag-"),
                "the refusal must surface the grammar's own diagnostic code, not \
                 collapse to a generic sentence: {message}"
            );
        }
    }

    /// The over-refusal half. `i-enochian`, the two `x-` private-use tags and
    /// the two terminal-only shapes are exactly what a profile chosen one notch
    /// too strict would start silently rejecting on ingress.
    #[test]
    fn every_well_formed_language_tag_still_reads_back() {
        for tag in ACCEPTED_TAGS {
            let parsed = from_json(&srj_with_lang(tag))
                .unwrap_or_else(|e| panic!("{tag:?} must still parse: {e}"));
            assert_eq!(
                parsed.rows[0][0],
                Some(TermValue::Literal {
                    lexical_form: "x".to_owned(),
                    datatype: RDF_LANGSTRING.to_owned(),
                    language: Some((*tag).to_owned()),
                    direction: None,
                }),
                "the gate must not alter the tag it lets through ({tag:?})"
            );
        }
    }

    /// The nesting depth every deep-input test uses.
    const DEEP: usize = 1_000_000;

    /// Run `body` on a thread with a 128 KiB machine stack: a walk that recursed once
    /// per nesting level would overflow it after a few hundred levels.
    fn on_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
        purrdf_stack::on_stack(128 * 1024, body).expect("spawn a small-stack thread")
    }

    /// `depth` nested JSON arrays around nothing: `[[…[]…]]`.
    fn nested_arrays(depth: usize) -> String {
        let mut text = "[".repeat(depth);
        text.push_str(&"]".repeat(depth));
        text
    }

    fn select_with_binding(cell: &str) -> Vec<u8> {
        format!(r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{cell}}}]}}}}"#)
            .into_bytes()
    }

    fn format_error(message: &str) -> Error {
        Error::Format(format!("SPARQL-JSON: {message}"))
    }

    #[test]
    fn a_deeply_nested_value_in_a_binding_position_is_refused_without_overflow() {
        let (full, bounded, truncated) = on_small_stack(|| {
            let doc = select_with_binding(&nested_arrays(DEEP));
            // The same nesting with its innermost array left unclosed.
            let unclosed = select_with_binding(&"[".repeat(DEEP));
            (
                from_json(&doc),
                from_json_bounded(&doc, u64::MAX).map(|bounded| bounded.solutions),
                from_json(&unclosed),
            )
        });
        let not_an_object = format_error("binding is not an object");
        assert_eq!(full, Err(not_an_object.clone()));
        assert_eq!(bounded, Err(not_an_object));
        assert_eq!(
            truncated,
            Err(format_error(&format!(
                "JSON byte {}: expected a JSON value",
                r#"{"head":{"vars":["x"]},"results":{"bindings":[{"x":"#.len() + DEEP
            )))
        );
    }

    #[test]
    fn a_deeply_nested_ignored_member_parses_without_overflow() {
        let (full, bounded, skipped) = on_small_stack(|| {
            let deep = nested_arrays(DEEP);
            let doc = format!(
                r#"{{"head":{{"vars":["x"],"link":{deep}}},"results":{{"bindings":[{{"x":{{"type":"uri","value":"http://example.org/a"}},"y":{deep}}}]}},"extra":{deep}}}"#
            )
            .into_bytes();
            // A second binding past a one-cell ceiling is skipped, not decoded.
            let over_limit = format!(
                r#"{{"head":{{"vars":["x"]}},"results":{{"bindings":[{{"x":{{"type":"uri","value":"http://example.org/a"}}}},{deep}]}}}}"#
            )
            .into_bytes();
            (
                from_json(&doc),
                from_json_bounded(&doc, u64::MAX),
                from_json_bounded(&over_limit, 1),
            )
        });
        let expected = ParsedSolutions {
            variables: vec!["x".to_owned()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/a".to_owned(),
            ))]],
        };
        assert_eq!(full.as_ref(), Ok(&expected));
        assert_eq!(
            bounded,
            Ok(BoundedParsedSolutions {
                solutions: expected.clone(),
                truncated: false,
            })
        );
        assert_eq!(
            skipped,
            Ok(BoundedParsedSolutions {
                solutions: expected,
                truncated: true,
            })
        );
    }

    /// A triple term nesting `depth` triple terms in object position, around an IRI.
    fn nested_triple_binding(depth: usize) -> String {
        let open = r#"{"type":"triple","value":{"subject":{"type":"bnode","value":"b"},"predicate":{"type":"uri","value":"http://example.org/p"},"object":"#;
        let mut text = open.repeat(depth);
        text.push_str(r#"{"type":"uri","value":"http://example.org/o"}"#);
        text.push_str(&"}}".repeat(depth));
        text
    }

    /// The triple-term nesting depth of `term`, counted along object positions.
    fn object_chain_depth(mut term: &TermValue) -> usize {
        let mut depth = 0;
        while let TermValue::Triple { s, p, o } = term {
            assert!(matches!(&**s, TermValue::Blank { label, .. } if label == "b"));
            assert_eq!(&**p, &TermValue::Iri("http://example.org/p".to_owned()));
            depth += 1;
            term = o;
        }
        assert_eq!(term, &TermValue::Iri("http://example.org/o".to_owned()));
        depth
    }

    #[test]
    fn a_deeply_nested_triple_term_binding_decodes_without_overflow() {
        let (full, bounded) = on_small_stack(|| {
            let doc = select_with_binding(&nested_triple_binding(DEEP));
            let full = from_json(&doc).map(|parsed| {
                let [row] = parsed.rows.as_slice() else {
                    panic!("one row");
                };
                object_chain_depth(row[0].as_ref().expect("bound"))
            });
            let bounded = from_json_bounded(&doc, u64::MAX).map(|bounded| {
                object_chain_depth(bounded.solutions.rows[0][0].as_ref().expect("bound"))
            });
            (full, bounded)
        });
        assert_eq!(full, Ok(DEEP));
        assert_eq!(bounded, Ok(DEEP));
    }

    #[test]
    fn a_shallow_nested_triple_term_decodes_to_the_same_term() {
        let parsed = from_json(&select_with_binding(&nested_triple_binding(2))).expect("parse");
        let triple = |o: TermValue| TermValue::Triple {
            s: TermBox::new(TermValue::Blank {
                label: "b".to_owned(),
                scope: BlankScope::DEFAULT,
            }),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_owned())),
            o: TermBox::new(o),
        };
        let expected = triple(triple(TermValue::Iri("http://example.org/o".to_owned())));
        assert_eq!(parsed.rows, vec![vec![Some(expected)]]);
    }

    #[test]
    fn a_nested_triple_term_reports_its_first_error_in_component_order() {
        // The predicate is not an IRI, and the object is malformed: the object is
        // decoded before the predicate is checked, so the object's error is reported.
        let both = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"literal","value":"p"},"object":{"type":"nonsense"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(both)),
            Err(format_error("unknown binding type `nonsense`"))
        );
        let predicate_only = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"literal","value":"p"},"object":{"type":"uri","value":"http://example.org/o"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(predicate_only)),
            Err(format_error("triple-term predicate is not an IRI"))
        );
        let no_object = r#"{"type":"triple","value":{"subject":{"type":"uri","value":"http://example.org/s"},"predicate":{"type":"uri","value":"http://example.org/p"}}}"#;
        assert_eq!(
            from_json(&select_with_binding(no_object)),
            Err(format_error("triple has no object"))
        );
    }

    #[test]
    fn malformed_nesting_reports_the_same_errors() {
        for (doc, message) in [
            (
                r#"{"a":[1,2}"#,
                "JSON byte 9: expected `,` or `]` in an array",
            ),
            (
                r#"{"a":{"b":1]}"#,
                "JSON byte 11: expected `,` or `}` in an object",
            ),
            (
                r#"{"a":{"b" 1}}"#,
                "JSON byte 10: expected `:` after a member name",
            ),
            (
                r#"{"a":[{ 7 }]}"#,
                "JSON byte 8: expected a `\"`-quoted member name",
            ),
            (r#"{"a":[1,]}"#, "JSON byte 8: expected a JSON value"),
            (
                r#"{"a":[[[]]]} x"#,
                "JSON byte 13: trailing data after the top-level value",
            ),
        ] {
            assert_eq!(
                from_json(doc.as_bytes()),
                Err(format_error(message)),
                "tree parser on {doc}"
            );
            assert_eq!(
                from_json_bounded(doc.as_bytes(), u64::MAX),
                Err(format_error(message)),
                "skipping parser on {doc}"
            );
        }
    }
}

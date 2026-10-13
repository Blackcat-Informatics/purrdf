// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical SPARQL Results JSON (SRJ) serializer plus the additive, provenance
//! carrying extension.
//!
//! The default (empty-provenance) `SELECT`/`ASK` output matches the SPARQL 1.2
//! Query Results JSON Format specification's own binding-encoding table
//! (<https://www.w3.org/TR/sparql12-results-json/>, §3.2.2 "Encoding RDF
//! terms"). In particular: **a simple literal (no language tag, `xsd:string`
//! datatype) serializes BARE** — the spec's table gives "Literal *S*" as
//! `{"type": "literal","value": "S"}`, with NO `"datatype"` member at all;
//! only "Literal *S* with datatype IRI *D*" carries one, as `{ "type":
//! "literal", "value": "S", "datatype": "D"}`. The vendored W3C corpus
//! confirms this spelling directly: `suite/w3c-sparql12/lang-basedir/
//! langdir-literal.srj`'s `"langdir"` bindings are bare
//! (`{ "type": "literal" , "value": "" }`, no `"datatype"` key). This writer
//! previously emitted an explicit
//! `"datatype":"http://www.w3.org/2001/XMLSchema#string"` on every simple
//! literal — a genuine spelling divergence from the spec, now fixed (see
//! `simple_literal_serializes_bare_per_spec` below for the pin). The
//! CONSTRUCT (`Graph`) branch, meanwhile, uses the wasm-clean [`crate::graph`]
//! N-QUADS writer and therefore additionally carries
//! RDF-1.2-star reifier/annotation lines AND every row's named graph (maximal
//! information flow). `{"graph": …}` is PurRDF's own envelope member, not a W3C
//! SRJ one, and a quad-template `CONSTRUCT { GRAPH ?g { … } }` result rendered
//! through a triple-only writer would have dropped the graph names the query
//! spelled out — see [`crate::graph`] for why this member widens rather than
//! refusing. A default-graph-only result is byte-identical either way.
//!
//! When the supplied [`ResultProvenance`] is non-empty AND the caller supplies a
//! [`ProvenanceNamespace`], an additive top-level member (keyed by
//! [`ProvenanceNamespace::prefix`]) is appended to the result object. W3C SRJ
//! parsers ignore the unknown key, so populated output stays a valid superset of
//! the standard form. PurRDF mints no vocabulary IRIs of its own, so without a
//! caller-supplied namespace no such member is ever emitted, however populated
//! `provenance` is — see [`ProvenanceNamespace`].
//!
//! Likewise the per-literal SPARQL 1.2 `"its:dir"` key is emitted only for
//! directional literals, so non-directional output is unchanged. `its:dir` is
//! the spelling the SPARQL 1.2 Query Results specification uses for RDF 1.2
//! base direction (the ITS — Internationalization Tag Set — namespace
//! `http://www.w3.org/2005/11/its`, the same convention the SPARQL 1.2 Query
//! Results XML Format spec uses on `<literal its:dir="…">`; see [`crate::xml`]),
//! and it is fixture-evidenced: the vendored W3C SPARQL 1.2 test suite's
//! `lang-basedir/langdir-literal.srj`, `strlangdir.srj`, and `concat.srj` all use
//! `"its:dir"` on directional-literal bindings, and [`crate::json_read`] prefers
//! that spelling on read.

use crate::SerializeOutcome;
use crate::error::Error;
use crate::graph::write_dataset_nquads;
use crate::model::{ProvenanceNamespace, ResultProvenance};
use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
use purrdf_core::sink::TextOut;
use purrdf_core::{SparqlResult, TermValue};
use purrdf_lex::json_escape::{JsonEscapes, escape_body};

/// The `xsd:string` IRI; a literal carrying it (with no language) serializes
/// BARE — no `"datatype"` member — per the SPARQL 1.2 Query Results JSON
/// Format spec's own encoding table (see the module docs).
use purrdf_core::datatype::XSD_STRING;

/// Serialize a [`SparqlResult`] to SPARQL Results JSON, appending the additive
/// provenance extension — keyed under `namespace.prefix` — when `provenance` is
/// non-empty and `namespace` is supplied.
///
/// JSON carries everything that is requested PROVIDED a namespace is supplied
/// to anchor it under (PurRDF mints no vocabulary IRIs of its own — see
/// [`ProvenanceNamespace`]). [`SerializeOutcome::provenance_dropped`] is `true`
/// only when `provenance` is non-empty but no `namespace` was given, so a caller
/// that populates provenance without configuring a namespace can still detect
/// the drop, exactly as CSV/TSV signal their lack of an extension point.
///
/// # Examples
///
/// An `ASK` boolean round-trips through the SRJ document form:
///
/// ```
/// use purrdf_sparql_results::{ResultProvenance, SparqlResult, from_json_boolean, to_json};
///
/// let outcome = to_json(&SparqlResult::Boolean(true), &ResultProvenance::default(), None)
///     .expect("ASK serializes to JSON");
/// assert!(!outcome.provenance_dropped);
/// assert!(from_json_boolean(&outcome.bytes).expect("emitted SRJ parses back"));
/// ```
///
/// # Errors
///
/// Returns [`Error::MalformedTerm`] when a [`purrdf_core::TermValue::Triple`]
/// carries a non-IRI predicate (e.g. a blank node or literal). RDF predicates
/// must be IRIs; emitting such input would produce structurally invalid SRJ.
pub fn to_json(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
) -> Result<SerializeOutcome, Error> {
    // Cheap lower-bound pre-size (capacity is unobservable in the output):
    // the fixed skeleton plus a modest per-cell estimate saves the early
    // doubling reallocations on large result sets.
    let mut out = String::with_capacity(output_size_hint(result));
    write_srj(result, provenance, namespace, &mut out)?;
    Ok(SerializeOutcome {
        bytes: out.into_bytes(),
        provenance_dropped: !provenance.is_empty() && namespace.is_none(),
    })
}

/// Write the full SRJ document (base object + optional provenance extension).
///
/// The base object's members are written first WITHOUT their enclosing `}`, then —
/// when `provenance` is non-empty AND `namespace` is supplied — the
/// `namespace.prefix`-keyed member, and finally the one closing brace. Either
/// condition failing means no extension is written at all (PurRDF mints no
/// vocabulary IRIs of its own).
///
/// The brace is emitted here rather than by the base writers and retracted with
/// `pop` when an extension follows. The bytes are identical either way — every
/// base branch ended in exactly the root `}`, so withholding it and appending it
/// here reproduces the same sequence from the same producers — but a rewind is not
/// available to an incremental sink that may already have drained the brace
/// downstream. The former `Error::Internal` guard checked that the branch just
/// called had ended with `}`; that is now a property of the split rather than a
/// runtime assertion, so the arm is gone.
pub(crate) fn write_srj<W: TextOut + ?Sized>(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
    out: &mut W,
) -> Result<(), Error> {
    write_base_body(result, out)?;

    if !provenance.is_empty()
        && let Some(namespace) = namespace
    {
        out.push(',');
        json_string(namespace.prefix(), out);
        out.push(':');
        write_provenance_body(result, provenance, namespace, out);
    }

    out.push('}');
    Ok(())
}

/// Write the pure-W3C SRJ object (no provenance extension at the top level). This
/// is the byte-identity contract with the legacy rdf-capi emitter, save for the
/// `Graph` branch and the additive per-literal SPARQL 1.2 `"its:dir"` key.
fn write_base_body<W: TextOut + ?Sized>(result: &SparqlResult, out: &mut W) -> Result<(), Error> {
    match result {
        SparqlResult::Boolean(value) => {
            out.push_str("{\"head\":{},\"boolean\":");
            out.push_str(if *value { "true" } else { "false" });
        }
        SparqlResult::Solutions {
            variables, rows, ..
        } => {
            out.push_str("{\"head\":{\"vars\":[");
            for (i, var) in variables.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_string(var, out);
            }
            out.push_str("]},\"results\":{\"bindings\":[");
            for (row_index, row) in rows.iter().enumerate() {
                if row_index > 0 {
                    out.push(',');
                }
                out.push('{');
                let mut first = true;
                for (column, cell) in row.iter().enumerate() {
                    if let Some(value) = cell {
                        // Rows are dense over `vars`; a cell with no matching
                        // variable header is a structural violation rather than a
                        // panic site.
                        let var = variables.get(column).ok_or_else(|| {
                            Error::MalformedTerm(format!(
                                "binding column {column} has no variable header (row has {} vars)",
                                variables.len()
                            ))
                        })?;
                        if !first {
                            out.push(',');
                        }
                        first = false;
                        json_string(var, out);
                        out.push(':');
                        json_binding(value, out)?;
                    }
                }
                out.push('}');
            }
            // `]}` closes `bindings` and `results`; the root `}` is `write_srj`'s.
            out.push_str("]}");
        }
        SparqlResult::Graph(graph) => {
            // Wasm-clean deviation from rdf-capi: render N-Quads directly from
            // the rdf-core kernel, additionally carrying
            // reifier/annotation lines and every row's graph slot (see
            // [`crate::graph`] for why this envelope widens rather than refuses).
            //
            // Rendered STRAIGHT THROUGH the escaper rather than rendered whole and
            // escaped afterwards. A CONSTRUCT answer is the one SPARQL result that is
            // dataset-sized, so building the N-Quads document first held the entire
            // graph twice over — once as the dataset, once as its rendering — before a
            // byte could reach the sink, and the sink's window bounded neither.
            //
            // The quotes are written here because the adapter escapes fragments and
            // cannot know where the string begins or ends.
            out.push_str("{\"graph\":\"");
            write_dataset_nquads(graph.as_ref(), &mut JsonEscaping(out));
            out.push('"');
        }
    }
    Ok(())
}

/// Write the additive provenance extension object (the value of the caller's
/// namespaced top-level member). Only present fields are emitted, in a fixed
/// order.
///
/// The `"namespace"` member records `namespace.iri()` — JSON has no `xmlns`-style
/// binding mechanism, so this is the JSON twin of the XML writer's
/// `xmlns:{prefix}="{iri}"` declaration on `<{prefix}:provenance>`. Writing the
/// caller's IRI INTO the document is what lets [`crate::json_read::provenance_from_json`]
/// resolve this member by namespace identity instead of trusting that the
/// top-level key it happens to be spelled under (`namespace.prefix()`, a bare
/// string with no uniqueness guarantee) was never reused by an unrelated caller.
fn write_provenance_body<W: TextOut + ?Sized>(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: &ProvenanceNamespace,
    out: &mut W,
) {
    out.push_str("{\"namespace\":");
    json_string(namespace.iri(), out);
    out.push_str(",\"queryForm\":");
    json_string(result.query_form(), out);
    if let Some(query_hash) = &provenance.query_hash {
        out.push_str(",\"queryHash\":");
        json_string(query_hash, out);
    }
    if let Some(engine) = &provenance.engine {
        out.push_str(",\"engine\":");
        json_string(engine, out);
    }
    if !provenance.solutions.is_empty() {
        out.push_str(",\"solutions\":[");
        for (i, solution) in provenance.solutions.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"sources\":[");
            for (j, source) in solution.sources.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                json_string(source, out);
            }
            out.push_str("]}");
        }
        out.push(']');
    }
    out.push('}');
}

/// A cheap lower-bound estimate of the serialized size, used only to pre-size
/// the output buffer.
fn output_size_hint(result: &SparqlResult) -> usize {
    const SKELETON: usize = 64;
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => SKELETON.saturating_add(
            rows.len()
                .saturating_mul(variables.len().saturating_add(1))
                .saturating_mul(32),
        ),
        SparqlResult::Graph(dataset) => {
            SKELETON.saturating_add(dataset.quad_count().saturating_mul(64))
        }
        SparqlResult::Boolean(_) => SKELETON,
    }
}

/// Append a JSON-escaped string literal (including the surrounding quotes).
fn json_string<W: TextOut + ?Sized>(value: &str, out: &mut W) {
    out.push('"');
    json_escape_body(value, out);
    out.push('"');
}

/// A [`TextOut`] that JSON-escapes every fragment pushed through it.
///
/// This is what lets a dataset-sized value become a JSON string without ever being
/// one: the kernel's N-Quads writers push term by term, each push is escaped on its
/// way past, and nothing between the producer and the drain holds the document.
///
/// It emits NO quotes — see [`json_escape_body`] for why escaping fragment-wise is
/// exact, and the graph arm of [`write_srj_body`] for the caller that writes them.
struct JsonEscaping<'a, W: TextOut + ?Sized>(&'a mut W);

impl<W: TextOut + ?Sized> std::fmt::Write for JsonEscaping<'_, W> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        json_escape_body(text, self.0);
        Ok(())
    }
}

impl<W: TextOut + ?Sized> TextOut for JsonEscaping<'_, W> {
    fn push_str(&mut self, text: &str) {
        json_escape_body(text, self.0);
    }

    fn push(&mut self, ch: char) {
        let mut buffer = [0u8; 4];
        json_escape_body(ch.encode_utf8(&mut buffer), self.0);
    }

    /// Forwarded, so an emitter polling a dead drain through this adapter sees the
    /// same answer it would see through the sink itself. Reporting `false` here
    /// would cost a whole document of formatting per failed write.
    fn failed(&self) -> bool {
        self.0.failed()
    }
}

/// A JSON string's BODY — everything that goes between the quotes.
///
/// Split out from [`json_string`] so a caller that is assembling one JSON string
/// from many fragments can write the quotes itself and escape each fragment as it
/// is produced. Fragment-wise escaping is exact here because every rule of the law maps
/// one `char` independently: a `&str` fragment can never split a `char`, so
/// escaping the pieces and escaping the concatenation give the same bytes.
///
/// The spelling is the workspace's one JSON escape law,
/// [`purrdf_lex::json_escape`], in its [`JsonEscapes::Minimal`] form: clean runs
/// end at the first `"`, `\` or C0 control, found by the chunked scan of that
/// exact class, and are handed to the sink whole.
fn json_escape_body<W: TextOut + ?Sized>(value: &str, out: &mut W) {
    escape_body(value, JsonEscapes::Minimal, |piece| out.push_str(piece));
}

/// Append a SPARQL-JSON binding object for a term value (recursive for triples).
///
/// Byte-identical to the rdf-capi emitter except for the SPARQL 1.2 `"its:dir"`
/// key, emitted only when the literal carries a base direction.
///
/// # Errors
///
/// Returns [`crate::error::Error::MalformedTerm`] if a [`TermValue::Triple`]
/// arm's predicate is not an IRI. RDF predicates must be IRIs; emitting a
/// non-IRI predicate would produce structurally invalid SRJ output.
///
/// A triple term is written over a work list: its predicate is checked and its opening
/// written at once, then its subject next, with the predicate key, the predicate, the
/// object key, the object and the closing held back in that order until the subject's
/// whole nesting is written.
fn json_binding<W: TextOut + ?Sized>(value: &TermValue, out: &mut W) -> Result<(), Error> {
    enum Piece<'t> {
        Term(&'t TermValue),
        Text(&'static str),
    }
    let mut held: Vec<Piece<'_>> = Vec::new();
    let mut next = Some(Piece::Term(value));
    while let Some(piece) = next.take().or_else(|| held.pop()) {
        let value = match piece {
            Piece::Text(text) => {
                out.push_str(text);
                continue;
            }
            Piece::Term(value) => value,
        };
        match value {
            TermValue::Iri(iri) => {
                out.push_str("{\"type\":\"uri\",\"value\":");
                json_string(iri, out);
                out.push('}');
            }
            TermValue::Blank { label, scope } => {
                // A SPARQL-results JSON bnode `value` is a blank-node LABEL, not
                // free text, so the `(label, scope)` pair is encoded into the W3C
                // BLANK_NODE_LABEL alphabet — the same alphabet the CSV/TSV writers
                // emit, so one result never disagrees with itself across formats.
                out.push_str("{\"type\":\"bnode\",\"value\":");
                json_string(
                    &encode_blank_label(label, *scope, LabelAlphabet::BlankNodeLabel),
                    out,
                );
                out.push('}');
            }
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                out.push_str("{\"type\":\"literal\",\"value\":");
                json_string(lexical_form, out);
                if let Some(language) = language {
                    out.push_str(",\"xml:lang\":");
                    json_string(language, out);
                } else if datatype != XSD_STRING {
                    // A simple literal (no language, `xsd:string` datatype)
                    // serializes BARE per the spec's own encoding table — see the
                    // module docs.
                    out.push_str(",\"datatype\":");
                    json_string(datatype, out);
                }
                if let Some(direction) = direction {
                    out.push_str(",\"its:dir\":\"");
                    out.push_str(direction.as_str());
                    out.push('"');
                }
                out.push('}');
            }
            TermValue::Triple { s, p, o } => {
                // RDF predicates must be IRIs; a non-IRI predicate has no valid SRJ
                // "predicate" form → hard-fail per the serializer contract.
                if !matches!(p.as_ref(), TermValue::Iri(_)) {
                    return Err(Error::MalformedTerm(
                        "triple-term predicate is not an IRI".to_string(),
                    ));
                }
                out.push_str("{\"type\":\"triple\",\"value\":{\"subject\":");
                held.extend([
                    Piece::Text("}}"),
                    Piece::Term(o),
                    Piece::Text(",\"object\":"),
                    Piece::Term(p),
                    Piece::Text(",\"predicate\":"),
                ]);
                next = Some(Piece::Term(s));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SolutionProvenance;
    use purrdf_core::TermBox;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm, RdfTextDirection};

    use purrdf_core::datatype::XSD_INTEGER;
    use purrdf_core::vocab::rdf::LANG_STRING as RDF_LANGSTRING;

    fn json_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        let outcome = to_json(result, prov, None).expect("serialization succeeds");
        String::from_utf8(outcome.bytes).expect("UTF-8 output")
    }

    /// The SRJ string spelling, byte for byte: `"` and `\` escaped, LF, CR
    /// and TAB as their short forms, every other C0 control as a lowercase
    /// `\u00XX`, and everything else (DEL, C1, U+2028, non-ASCII) verbatim.
    #[test]
    fn json_string_spells_the_minimal_escape_law() {
        let cases: [(&str, &str); 15] = [
            ("", r#""""#),
            (
                "plain ascii text 0123456789 ~!@#$%^&*()_+-=[]{};':,./<>?",
                r#""plain ascii text 0123456789 ~!@#$%^&*()_+-=[]{};':,./<>?""#,
            ),
            ("\"", r#""\"""#),
            ("\\", r#""\\""#),
            ("\n", r#""\n""#),
            ("\r", r#""\r""#),
            ("\t", r#""\t""#),
            (
                "\u{0}\u{1}\u{8}\u{c}\u{1f}",
                r#""\u0000\u0001\u0008\u000c\u001f""#,
            ),
            ("\u{7f}", "\"\u{7f}\""),
            ("\u{80}\u{85}\u{9f}", "\"\u{80}\u{85}\u{9f}\""),
            (
                "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
                "\"caf\u{e9} \u{4e2d}\u{6587} \u{1f431}\"",
            ),
            (
                "mixed \"quoted\" \\ back\\slash\n\ttab \u{e9}\u{1}end",
                "\"mixed \\\"quoted\\\" \\\\ back\\\\slash\\n\\ttab \u{e9}\\u0001end\"",
            ),
            ("\u{2028}\u{feff}\u{e9}\"", "\"\u{2028}\u{feff}\u{e9}\\\"\""),
            ("trailing quote\"", r#""trailing quote\"""#),
            ("\"leading quote", r#""\"leading quote""#),
        ];
        for (value, expected) in cases {
            let mut out = String::from("prefix");
            json_string(value, &mut out);
            assert_eq!(out, format!("prefix{expected}"), "{value:?}");
            // And it reads back as the value it spells.
            assert_eq!(
                purrdf_lex::json::read(expected).expect("a JSON string"),
                value,
                "{value:?}"
            );
        }
    }

    /// Escaping fragment by fragment through [`JsonEscaping`] gives the bytes
    /// escaping the whole value gives.
    #[test]
    fn fragment_wise_escaping_matches_whole_value_escaping() {
        let value = "a\"b\\c\nd\u{1}e\u{e9}f\u{1f431}";
        let mut whole = String::new();
        json_escape_body(value, &mut whole);
        let mut pieces = String::new();
        {
            let mut escaping = JsonEscaping(&mut pieces);
            for ch in value.chars() {
                escaping.push(ch);
            }
        }
        assert_eq!(pieces, whole);
    }

    fn json_text_ns(
        result: &SparqlResult,
        prov: &ResultProvenance,
        namespace: &ProvenanceNamespace,
    ) -> String {
        let outcome = to_json(result, prov, Some(namespace)).expect("serialization succeeds");
        String::from_utf8(outcome.bytes).expect("UTF-8 output")
    }

    fn lit(lex: &str, datatype: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: lex.to_string(),
            datatype: datatype.to_string(),
            language: None,
            direction: None,
        }
    }

    // 1. BYTE PIN — SELECT covering IRI, bnode, xsd:string, typed,
    //    lang, an unbound cell, and a multi-row case.
    #[test]
    fn select_empty_provenance_matches_spec_encoding() {
        let variables = vec![
            "s".to_string(),
            "b".to_string(),
            "name".to_string(),
            "age".to_string(),
            "label".to_string(),
        ];
        let rows = vec![
            // Row 0: every column bound.
            vec![
                Some(TermValue::Iri("http://example.org/s".to_string())),
                Some(TermValue::Blank {
                    label: "b0".to_string(),
                    scope: BlankScope(0),
                }),
                Some(lit("Ada", XSD_STRING)),
                Some(lit("42", XSD_INTEGER)),
                Some(TermValue::Literal {
                    lexical_form: "bonjour".to_string(),
                    datatype: RDF_LANGSTRING.to_string(),
                    language: Some("fr".to_string()),
                    direction: None,
                }),
            ],
            // Row 1: `b` and `age` unbound (None) → omitted from the row object.
            vec![
                Some(TermValue::Iri("http://example.org/s2".to_string())),
                None,
                Some(lit("Bob", XSD_STRING)),
                None,
                Some(lit("Grace", XSD_STRING)),
            ],
        ];
        let result = SparqlResult::Solutions {
            variables,
            rows,
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };

        let expected = concat!(
            "{\"head\":{\"vars\":[\"s\",\"b\",\"name\",\"age\",\"label\"]},",
            "\"results\":{\"bindings\":[",
            "{\"s\":{\"type\":\"uri\",\"value\":\"http://example.org/s\"},",
            "\"b\":{\"type\":\"bnode\",\"value\":\"b0\"},",
            "\"name\":{\"type\":\"literal\",\"value\":\"Ada\"},",
            "\"age\":{\"type\":\"literal\",\"value\":\"42\",\"datatype\":\"http://www.w3.org/2001/XMLSchema#integer\"},",
            "\"label\":{\"type\":\"literal\",\"value\":\"bonjour\",\"xml:lang\":\"fr\"}},",
            "{\"s\":{\"type\":\"uri\",\"value\":\"http://example.org/s2\"},",
            "\"name\":{\"type\":\"literal\",\"value\":\"Bob\"},",
            "\"label\":{\"type\":\"literal\",\"value\":\"Grace\"}}",
            "]}}"
        );
        assert_eq!(json_text(&result, &ResultProvenance::default()), expected);
    }

    /// BYTE PIN — a simple (`xsd:string`, no language) literal serializes
    /// BARE: `{"type":"literal","value":"S"}`, no `"datatype"` member — the
    /// SPARQL 1.2 Query Results JSON Format spec's own encoding table (see
    /// the module docs), matching the vendored W3C fixture
    /// `suite/w3c-sparql12/lang-basedir/langdir-literal.srj`'s `"langdir"`
    /// bindings, which are bare in exactly the same way.
    #[test]
    fn simple_literal_serializes_bare_per_spec() {
        let mut out = String::new();
        json_binding(&lit("hello", XSD_STRING), &mut out).expect("literal binding serializes");
        assert_eq!(out, "{\"type\":\"literal\",\"value\":\"hello\"}");
    }

    /// A literal with a NON-`xsd:string` datatype still carries an explicit
    /// `"datatype"` member — only the `xsd:string` abbreviation is bare.
    #[test]
    fn non_string_datatype_still_carries_explicit_datatype_member() {
        let mut out = String::new();
        json_binding(&lit("42", XSD_INTEGER), &mut out).expect("literal binding serializes");
        assert_eq!(
            out,
            "{\"type\":\"literal\",\"value\":\"42\",\"datatype\":\"http://www.w3.org/2001/XMLSchema#integer\"}"
        );
    }

    #[test]
    fn ask_true_empty_provenance_is_byte_identical() {
        let result = SparqlResult::Boolean(true);
        assert_eq!(
            json_text(&result, &ResultProvenance::default()),
            "{\"head\":{},\"boolean\":true}"
        );
    }

    #[test]
    fn ask_false_empty_provenance_is_byte_identical() {
        let result = SparqlResult::Boolean(false);
        assert_eq!(
            json_text(&result, &ResultProvenance::default()),
            "{\"head\":{},\"boolean\":false}"
        );
    }

    // 2. TRIPLE TERM nested binding shape.
    #[test]
    fn triple_term_binding_shape() {
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let text = json_text(&result, &ResultProvenance::default());
        assert!(
            text.contains(concat!(
                "\"t\":{\"type\":\"triple\",\"value\":{",
                "\"subject\":{\"type\":\"uri\",\"value\":\"http://example.org/s\"},",
                "\"predicate\":{\"type\":\"uri\",\"value\":\"http://example.org/p\"},",
                "\"object\":{\"type\":\"uri\",\"value\":\"http://example.org/o\"}}}"
            )),
            "unexpected triple shape: {text}"
        );
    }

    /// BYTE PIN — the FULL document for a single-row, single-variable triple-term
    /// SELECT result, pinned exactly (not just a substring), so any drift in the
    /// nested `{"type":"triple","value":{...}}` encoding is caught at the byte
    /// level.
    #[test]
    fn triple_term_document_exact_bytes() {
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Iri("http://example.org/p".to_string())),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let text = json_text(&result, &ResultProvenance::default());
        let expected = concat!(
            "{\"head\":{\"vars\":[\"t\"]},\"results\":{\"bindings\":[",
            "{\"t\":{\"type\":\"triple\",\"value\":{",
            "\"subject\":{\"type\":\"uri\",\"value\":\"http://example.org/s\"},",
            "\"predicate\":{\"type\":\"uri\",\"value\":\"http://example.org/p\"},",
            "\"object\":{\"type\":\"uri\",\"value\":\"http://example.org/o\"}}}}",
            "]}}"
        );
        assert_eq!(text, expected);
    }

    // 3. NON-IRI PREDICATE — triple-term with a non-IRI predicate must hard-fail.
    #[test]
    fn non_iri_triple_predicate_is_malformed_error() {
        // A triple-term whose predicate is a plain literal (not an IRI) must
        // hard-fail with MalformedTerm rather than emitting structurally invalid
        // SRJ output.
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(lit("not-an-iri", XSD_STRING)),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let err = to_json(&result, &ResultProvenance::default(), None)
            .expect_err("non-IRI predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm, got: {err:?}"
        );
    }

    #[test]
    fn non_iri_bnode_triple_predicate_is_malformed_error() {
        // A blank-node predicate is equally invalid.
        let triple = TermValue::Triple {
            s: TermBox::new(TermValue::Iri("http://example.org/s".to_string())),
            p: TermBox::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: TermBox::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let err = to_json(&result, &ResultProvenance::default(), None)
            .expect_err("bnode predicate must be rejected");
        assert!(
            matches!(err, Error::MalformedTerm(_)),
            "expected MalformedTerm, got: {err:?}"
        );
    }

    // 5. DIRECTION ADDITIVE — directional literal carries the SPARQL 1.2
    //    "its:dir" key (see the module docs for the fixture evidence); plain
    //    literals must not.
    #[test]
    fn directional_literal_carries_its_dir_key() {
        let result = SparqlResult::Solutions {
            variables: vec!["d".to_string()],
            rows: vec![vec![Some(TermValue::Literal {
                lexical_form: "hello".to_string(),
                datatype: RDF_LANGSTRING.to_string(),
                language: Some("en".to_string()),
                direction: Some(RdfTextDirection::Ltr),
            })]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let text = json_text(&result, &ResultProvenance::default());
        assert!(
            text.contains("\"its:dir\":\"ltr\""),
            "expected SPARQL 1.2 its:dir key: {text}"
        );
    }

    #[test]
    fn non_directional_literal_omits_dir_key() {
        let result = SparqlResult::Solutions {
            variables: vec!["v".to_string()],
            rows: vec![vec![Some(lit("x", XSD_STRING))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let text = json_text(&result, &ResultProvenance::default());
        assert!(
            !text.contains("dir"),
            "plain literal must not carry an its:dir key: {text}"
        );
    }

    /// BYTE PIN — the exact per-binding shape of a directional literal must
    /// match the vendored W3C SPARQL 1.2 fixture
    /// `suite/w3c-sparql12/lang-basedir/langdir-literal.srj`'s `"l"`/`en`/`ltr`
    /// row: `{"type":"literal","value":"l","xml:lang":"en","its:dir":"ltr"}`.
    #[test]
    fn directional_literal_binding_matches_vendored_fixture_shape() {
        let mut out = String::new();
        json_binding(
            &TermValue::Literal {
                lexical_form: "l".to_string(),
                datatype: RDF_LANGSTRING.to_string(),
                language: Some("en".to_string()),
                direction: Some(RdfTextDirection::Ltr),
            },
            &mut out,
        )
        .expect("literal binding serializes");
        assert_eq!(
            out,
            "{\"type\":\"literal\",\"value\":\"l\",\"xml:lang\":\"en\",\"its:dir\":\"ltr\"}"
        );
    }

    // 6. MAXIMAL PATH — populated provenance, WITH a caller-supplied namespace,
    //    appears as a valid top-level member keyed by that namespace's prefix.
    #[test]
    fn populated_provenance_with_namespace_appends_valid_member() {
        let result = SparqlResult::Solutions {
            variables: vec!["s".to_string()],
            rows: vec![vec![Some(TermValue::Iri(
                "http://example.org/s".to_string(),
            ))]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux").into(),
        };
        let provenance = ResultProvenance {
            query_hash: Some("deadbeef".to_string()),
            engine: Some("purrdf-sparql-eval".to_string()),
            solutions: vec![SolutionProvenance {
                sources: vec!["http://example.org/g1".to_string()],
            }],
        };
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("test namespace is a valid NCName prefix + absolute IRI");
        let outcome = to_json(&result, &provenance, Some(&namespace)).expect("serializes");
        assert!(!outcome.provenance_dropped, "namespace was supplied");
        let text = String::from_utf8(outcome.bytes).expect("UTF-8");

        assert!(text.contains("\"prov\":{"), "missing prov member: {text}");
        assert!(
            text.contains("\"queryForm\":\"select\""),
            "missing queryForm: {text}"
        );
        assert!(
            text.contains("\"queryHash\":\"deadbeef\""),
            "missing queryHash: {text}"
        );
        assert!(
            text.contains("\"engine\":\"purrdf-sparql-eval\""),
            "missing engine: {text}"
        );
        assert!(
            text.contains("\"solutions\":[{\"sources\":[\"http://example.org/g1\"]}]"),
            "missing solutions: {text}"
        );
        // Still a single valid JSON object: starts/ends with braces, balanced.
        assert!(
            text.starts_with('{') && text.ends_with('}'),
            "envelope: {text}"
        );
        assert!(well_formed(&text), "malformed JSON: {text}");
    }

    #[test]
    fn populated_provenance_on_ask_stays_valid() {
        let result = SparqlResult::Boolean(true);
        let provenance = ResultProvenance {
            engine: Some("e".to_string()),
            ..Default::default()
        };
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("test namespace is a valid NCName prefix + absolute IRI");
        let text = json_text_ns(&result, &provenance, &namespace);
        assert!(
            text.contains("\"queryForm\":\"ask\""),
            "ask queryForm: {text}"
        );
        // No queryHash/solutions present (absent fields omitted).
        assert!(!text.contains("queryHash"), "no queryHash expected: {text}");
        assert!(!text.contains("solutions"), "no solutions expected: {text}");
        assert!(well_formed(&text), "malformed JSON: {text}");
    }

    /// DE-MINTING — populated provenance with NO namespace supplied emits no
    /// extension member at all (PurRDF mints no vocabulary IRIs of its own) and
    /// the drop is signalled, exactly like the CSV/TSV exit gate.
    #[test]
    fn populated_provenance_without_namespace_is_dropped_and_signalled() {
        let result = SparqlResult::Boolean(true);
        let provenance = ResultProvenance {
            engine: Some("e".to_string()),
            ..Default::default()
        };
        let outcome = to_json(&result, &provenance, None).expect("serializes");
        assert!(
            outcome.provenance_dropped,
            "non-empty provenance with no namespace must be signalled as dropped"
        );
        let text = String::from_utf8(outcome.bytes).expect("UTF-8");
        assert_eq!(
            text, "{\"head\":{},\"boolean\":true}",
            "base document must stay pure W3C with no fabricated member: {text}"
        );
    }

    // 7. GRAPH — CONSTRUCT result renders `{"graph":"<nq>"}` carrying the triple.
    #[test]
    fn graph_result_wraps_nquads() {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_owned_quad(&RdfQuad {
            subject: RdfTerm::iri("http://example.org/s"),
            predicate: "http://example.org/p".to_string(),
            object: RdfTerm::iri("http://example.org/o"),
            graph_name: None,
            location: None,
        });
        let dataset = builder.freeze().expect("dataset freezes");
        let result = SparqlResult::Graph(dataset.into());

        let text = json_text(&result, &ResultProvenance::default());
        assert!(text.starts_with("{\"graph\":\""), "graph envelope: {text}");
        assert!(text.ends_with("\"}"), "graph envelope close: {text}");
        // The embedded N-Quads (JSON-escaped) contains the expected line, with no
        // fourth term — a default-graph row renders exactly as it always did.
        assert_eq!(
            text,
            "{\"graph\":\"<http://example.org/s> <http://example.org/p> \
             <http://example.org/o> .\\n\"}",
        );
    }

    /// A quad-template `CONSTRUCT` result carries its graph name INTO the envelope.
    ///
    /// This member used to be rendered by a triple-only writer, so the graph name
    /// the query spelled out disappeared with no error and no loss count — the same
    /// silent-drop the CLI, Python, wasm and `purrdf_serialize` all refuse or
    /// count. `{"graph": …}` is PurRDF's own envelope rather than a caller-chosen
    /// RDF syntax, so it widens to N-Quads instead of refusing.
    #[test]
    fn graph_result_carries_the_named_graph() {
        let mut builder = RdfDatasetBuilder::new();
        builder.push_owned_quad(&RdfQuad {
            subject: RdfTerm::iri("http://example.org/s"),
            predicate: "http://example.org/p".to_string(),
            object: RdfTerm::iri("http://example.org/o"),
            graph_name: Some(RdfTerm::iri("http://example.org/g1")),
            location: None,
        });
        let dataset = builder.freeze().expect("dataset freezes");
        let result = SparqlResult::Graph(dataset.into());

        let text = json_text(&result, &ResultProvenance::default());
        assert_eq!(
            text,
            "{\"graph\":\"<http://example.org/s> <http://example.org/p> \
             <http://example.org/o> <http://example.org/g1> .\\n\"}",
            "the named graph must survive the envelope",
        );
    }

    /// Whether `text` is one well-formed JSON document.
    fn well_formed(text: &str) -> bool {
        purrdf_lex::json::read(text).is_ok()
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The binding writer against its recursive reference, and at a hundred thousand
    //! levels on a 128 KiB thread.

    use purrdf_core::TermValue;

    use super::json_binding;
    use crate::error::Error;

    fn reference(value: &TermValue, out: &mut String) -> Result<(), Error> {
        let TermValue::Triple { s, p, o } = value else {
            return json_binding(value, out);
        };
        if !matches!(p.as_ref(), TermValue::Iri(_)) {
            return Err(Error::MalformedTerm(
                "triple-term predicate is not an IRI".to_string(),
            ));
        }
        out.push_str("{\"type\":\"triple\",\"value\":{\"subject\":");
        reference(s, out)?;
        out.push_str(",\"predicate\":");
        reference(p, out)?;
        out.push_str(",\"object\":");
        reference(o, out)?;
        out.push_str("}}");
        Ok(())
    }

    /// Every generated term writes exactly the text — and, on a non-IRI predicate, the
    /// partial text and the refusal — the recursive reference writes.
    #[test]
    fn the_writer_agrees_with_its_recursive_reference_on_generated_terms() {
        let mut refused = 0;
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            let (mut written, mut expected) = (String::new(), String::new());
            let result = json_binding(&value, &mut written);
            assert_eq!(result, reference(&value, &mut expected), "seed {seed}");
            assert_eq!(written, expected, "seed {seed}");
            refused += usize::from(result.is_err());
        }
        assert!(refused > 0, "some generated term has a non-IRI predicate");
    }

    /// A triple term a hundred thousand levels deep is written on a thread whose whole
    /// stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_written_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let mut written = String::new();
            json_binding(
                &purrdf_core::term_fixture::triple_chain(LEVELS),
                &mut written,
            )
            .expect("every predicate is an IRI");
            assert_eq!(written.matches("\"type\":\"triple\"").count(), LEVELS);
            assert!(written.ends_with(&"}}".repeat(LEVELS)));
        })
        .expect("the thread starts");
    }
}

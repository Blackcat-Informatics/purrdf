// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! N-QUADS writer (no oxigraph) and therefore additionally carries
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
use crate::graph::dataset_to_nquads;
use crate::model::{ProvenanceNamespace, ResultProvenance};
use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
use purrdf_core::{SparqlResult, TermValue};

/// The `xsd:string` IRI; a literal carrying it (with no language) serializes
/// BARE — no `"datatype"` member — per the SPARQL 1.2 Query Results JSON
/// Format spec's own encoding table (see the module docs).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

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
/// The base object is written first, then — when `provenance` is non-empty AND
/// `namespace` is supplied — the `namespace.prefix`-keyed member is inserted
/// just before the document's final closing `}` so the resulting object stays
/// valid for all three result kinds. Either condition failing means no
/// extension is written at all (PurRDF mints no vocabulary IRIs of its own).
fn write_srj(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: Option<&ProvenanceNamespace>,
    out: &mut String,
) -> Result<(), Error> {
    write_base(result, out)?;

    if provenance.is_empty() {
        return Ok(());
    }
    let Some(namespace) = namespace else {
        return Ok(());
    };

    // Remove the trailing `}` of the base object, append the additive member,
    // then re-close. The base writers always end the object with `}`.
    if out.pop() != Some('}') {
        return Err(Error::Internal(
            "SRJ base object did not end with a closing brace".to_string(),
        ));
    }
    out.push(',');
    json_string(namespace.prefix(), out);
    out.push(':');
    write_provenance_body(result, provenance, namespace, out);
    out.push('}');
    Ok(())
}

/// Write the pure-W3C SRJ object (no provenance extension at the top level). This
/// is the byte-identity contract with the legacy rdf-capi emitter, save for the
/// `Graph` branch and the additive per-literal SPARQL 1.2 `"its:dir"` key.
fn write_base(result: &SparqlResult, out: &mut String) -> Result<(), Error> {
    match result {
        SparqlResult::Boolean(value) => {
            out.push_str("{\"head\":{},\"boolean\":");
            out.push_str(if *value { "true" } else { "false" });
            out.push('}');
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
            out.push_str("]}}");
        }
        SparqlResult::Graph(graph) => {
            // Wasm-clean deviation from rdf-capi: render N-Quads directly from
            // the rdf-core kernel (no oxigraph), additionally carrying
            // reifier/annotation lines and every row's graph slot (see
            // [`crate::graph`] for why this envelope widens rather than refuses).
            let nq = dataset_to_nquads(graph.as_ref());
            out.push_str("{\"graph\":");
            json_string(&nq, out);
            out.push('}');
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
fn write_provenance_body(
    result: &SparqlResult,
    provenance: &ResultProvenance,
    namespace: &ProvenanceNamespace,
    out: &mut String,
) {
    out.push_str("{\"namespace\":");
    json_string(namespace.iri(), out);
    out.push_str(",\"queryForm\":");
    json_string(query_form(result), out);
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

/// The `queryForm` discriminator for a result kind.
fn query_form(result: &SparqlResult) -> &'static str {
    match result {
        SparqlResult::Solutions { .. } => "select",
        SparqlResult::Boolean(_) => "ask",
        SparqlResult::Graph(_) => "construct",
    }
}

/// A byte the JSON escaper must act on: the two metacharacters and the C0
/// controls. All triggers are ASCII, so non-ASCII bytes (`>= 0x80`) never match
/// and a run split here always lands on a char boundary.
const fn json_trigger_byte(b: u8) -> bool {
    b < 0x20 || b == b'"' || b == b'\\'
}

/// Append a JSON-escaped string literal (including the surrounding quotes).
fn json_string(value: &str, out: &mut String) {
    use core::fmt::Write as _;

    out.push('"');
    let mut rest = value;
    while !rest.is_empty() {
        // Bulk-copy the clean run in one `push_str` rather than one `push`
        // per `char`; only the trigger byte goes through the match below.
        let run = rest
            .bytes()
            .position(json_trigger_byte)
            .unwrap_or(rest.len());
        out.push_str(&rest[..run]);
        rest = &rest[run..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                // Writing to a `String` is infallible; the escape bytes are unchanged.
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
        rest = &rest[ch.len_utf8()..];
    }
    out.push('"');
}

/// The original per-`char` escaper, kept as the oracle for [`json_string`].
#[cfg(test)]
fn json_string_reference(value: &str, out: &mut String) {
    use core::fmt::Write as _;

    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
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
fn json_binding(value: &TermValue, out: &mut String) -> Result<(), Error> {
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
            json_binding(s, out)?;
            out.push_str(",\"predicate\":");
            json_binding(p, out)?;
            out.push_str(",\"object\":");
            json_binding(o, out)?;
            out.push_str("}}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SolutionProvenance;
    use pretty_assertions::assert_eq;
    use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfQuad, RdfTerm, RdfTextDirection};

    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

    fn json_text(result: &SparqlResult, prov: &ResultProvenance) -> String {
        let outcome = to_json(result, prov, None).expect("serialization succeeds");
        String::from_utf8(outcome.bytes).expect("UTF-8 output")
    }

    #[test]
    fn json_string_matches_reference_escaper() {
        let cases: [&str; 15] = [
            "",
            "plain ascii text 0123456789 ~!@#$%^&*()_+-=[]{};':,./<>?",
            "\"",
            "\\",
            "\n",
            "\r",
            "\t",
            "\u{0}\u{1}\u{8}\u{c}\u{1f}",
            "\u{7f}",
            "\u{80}\u{85}\u{9f}",
            "caf\u{e9} \u{4e2d}\u{6587} \u{1f431}",
            "mixed \"quoted\" \\ back\\slash\n\ttab \u{e9}\u{1}end",
            "\u{2028}\u{feff}\u{e9}\"",
            "trailing quote\"",
            "\"leading quote",
        ];
        for case in cases {
            let mut fast = String::from("prefix");
            let mut reference = String::from("prefix");
            json_string(case, &mut fast);
            json_string_reference(case, &mut reference);
            assert_eq!(fast, reference, "{case:?}");
        }
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
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Iri("http://example.org/p".to_string())),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(lit("not-an-iri", XSD_STRING)),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            s: Box::new(TermValue::Iri("http://example.org/s".to_string())),
            p: Box::new(TermValue::Blank {
                label: "b0".to_string(),
                scope: BlankScope(0),
            }),
            o: Box::new(TermValue::Iri("http://example.org/o".to_string())),
        };
        let result = SparqlResult::Solutions {
            variables: vec!["t".to_string()],
            rows: vec![vec![Some(triple)]],
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
            aux: RdfDatasetBuilder::new().freeze().expect("empty aux"),
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
        assert!(braces_balanced(&text), "unbalanced braces: {text}");
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
        assert!(braces_balanced(&text), "unbalanced braces: {text}");
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
        let result = SparqlResult::Graph(dataset);

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
        let result = SparqlResult::Graph(dataset);

        let text = json_text(&result, &ResultProvenance::default());
        assert_eq!(
            text,
            "{\"graph\":\"<http://example.org/s> <http://example.org/p> \
             <http://example.org/o> <http://example.org/g1> .\\n\"}",
            "the named graph must survive the envelope",
        );
    }

    /// Tiny brace-balance check (no serde dep): every `{`/`}` outside a JSON
    /// string literal must nest to zero, never going negative.
    fn braces_balanced(text: &str) -> bool {
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escaped = false;
        for ch in text.chars() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        depth == 0 && !in_string
    }
}

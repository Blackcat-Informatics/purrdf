// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party in-memory serialization model + RDF text serializers.
//!
//! [`SerGraph`] is the first-party term/quad/reifier/annotation shape the frozen
//! [`RdfDataset`](crate::RdfDataset) IR is lowered into before egress. The Turtle /
//! TriG / N-Triples / N-Quads serializers walk exactly this shape, emitting literal
//! lexical forms VERBATIM — they never canonicalize a literal's value-space nor narrow
//! its datatype (the whole point of the native codec: byte-for-byte lexical fidelity).

use crate::RdfDiagnostic;

/// The kind of a serialization term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SerTermKind {
    Iri,
    Bnode,
    Literal,
    Triple,
}

/// A single RDF term in the serialization model, carried by integer id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SerTerm {
    /// Term kind.
    pub kind: SerTermKind,
    /// IRI string, literal lexical form, or blank-node label (scope-local).
    pub value: Option<String>,
    /// Term-id of the literal's datatype IRI, when explicit.
    pub datatype: Option<usize>,
    /// Literal language tag (BCP 47).
    pub lang: Option<String>,
    /// RDF 1.2 literal base direction (`"ltr"` or `"rtl"`).
    pub direction: Option<String>,
    /// Term-id of the reifier of a quoted triple (`kind == Triple`).
    pub reifier: Option<usize>,
}

/// A quad of term-ids; the graph slot is `None` for the default graph.
pub(crate) type SerQuad = (usize, usize, usize, Option<usize>);
/// A subject/predicate/object triple of term-ids.
pub(crate) type SerTriple3 = (usize, usize, usize);
/// A reifier row: `(reifier, (s, p, o), graph?)`.
pub(crate) type SerReifierRow = (usize, SerTriple3, Option<usize>);
/// An annotation row: `(reifier, predicate, value, graph?)`.
pub(crate) type SerAnnotationRow = (usize, usize, usize, Option<usize>);

/// The serialization graph: terms plus the base quads and the RDF 1.2 statement layer
/// (reifier bindings + annotations). Each row carries an `Option<usize>` graph slot
/// (`None` = default graph).
#[derive(Debug, Default)]
pub(crate) struct SerGraph {
    pub terms: Vec<SerTerm>,
    pub quads: Vec<SerQuad>,
    pub reifiers: Vec<SerReifierRow>,
    pub annotations: Vec<SerAnnotationRow>,
}

impl SerGraph {
    /// Look up a reifier binding: the `(s, p, o)` of the FIRST `reifiers` row whose id
    /// equals `rid`.
    pub(crate) fn reifier(&self, rid: usize) -> Option<SerTriple3> {
        self.reifiers
            .iter()
            .find(|(r, _, _)| *r == rid)
            .map(|(_, spo, _)| *spo)
    }
}

/// Crockford Base32 alphabet (the ULID rendering alphabet).
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// A rendered ULID is 26 Crockford Base32 digits.
const ULID_LEN: usize = 26;

/// A deterministic blank-node label with the given `prefix`, byte-identical to the
/// prior purrdf-gts `deterministic_label(prefix, counter)`: `prefix` plus the 26-digit
/// Crockford Base32 rendering of a zero-timestamp ULID built from `counter`.
///
/// With a zero timestamp the rendered ULID value equals `counter` for any
/// `counter < 2^80`, so this renders the 128-bit big-endian value `counter as u128`
/// as 26 Crockford Base32 digits, digit `i` being `(value >> (125 - i*5)) & 0x1f`.
pub(crate) fn deterministic_blank_label_with_prefix(prefix: &str, counter: usize) -> String {
    let value = counter as u128;
    let mut buffer = [0u8; ULID_LEN];
    for (index, byte) in buffer.iter_mut().enumerate() {
        let shift = 125 - index * 5;
        let digit = ((value >> shift) & 0x1f) as usize;
        *byte = CROCKFORD[digit];
    }
    // The buffer is ASCII (every byte comes from the Crockford alphabet), so the
    // UTF-8 conversion never fails.
    let rendered = std::str::from_utf8(&buffer).expect("Crockford digits are ASCII");
    format!("{prefix}{rendered}")
}

/// A deterministic blank-node label, byte-identical to the prior purrdf-gts
/// `deterministic_label("gts_", counter)`. See
/// [`deterministic_blank_label_with_prefix`].
pub(crate) fn deterministic_blank_label(counter: usize) -> String {
    deterministic_blank_label_with_prefix("gts_", counter)
}

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Return whether `direction` is a valid RDF 1.2 base direction token.
fn is_literal_direction(direction: &str) -> bool {
    matches!(direction, "ltr" | "rtl")
}

/// Escape an IRI body for an N-Triples / Turtle / TriG `<…>` `IRIREF`. The W3C grammar
/// forbids `<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\`, the space character, and every
/// control code point (C0 `0x00-0x1F`, DEL `0x7F`, and the C1 block `0x80-0x9F`) appearing
/// raw; each rides as a `\uXXXX` `UCHAR` (the text parser decodes them back). A clean ASCII
/// IRI (every production IRI) passes through byte-for-byte unchanged.
fn escape_iri(iri: &str) -> String {
    let mut out = String::with_capacity(iri.len());
    for ch in iri.chars() {
        match ch {
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                out.push_str(&format!("\\u{:04X}", ch as u32));
            }
            c if c.is_control() || c == ' ' => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Escape a literal lexical form for N-Triples. Escapes `\` and `"`, emits the readable ECHAR
/// forms for `\n`/`\r`/`\t`, and rides EVERY other control character (C0, DEL, and the C1 block
/// `0x80-0x9F`) as `\uXXXX`. This deliberately escapes MORE than the W3C-pinned canonical form
/// (`purrdf_core::ir::canon::write_literal_escaped`, which keeps C1 raw): this serializer's
/// output is embedded verbatim inside an XML text node by the CL-dialect carrier, and an XML
/// parser normalizes/replaces raw C1 code points on read — so the payload only survives an XML
/// round-trip if the full control range rides as ASCII `\uXXXX`. The canonical form answers to
/// RDFC-1.0 byte-conformance; this one answers to XML transport.
fn escape_literal(lex: &str) -> String {
    let mut out = String::with_capacity(lex.len());
    for ch in lex.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Render a term-id as an N-Triples token.
fn render_term(g: &SerGraph, tid: usize) -> String {
    let t = &g.terms[tid];
    match t.kind {
        SerTermKind::Iri => format!("<{}>", escape_iri(t.value.as_deref().unwrap_or(""))),
        SerTermKind::Bnode => match &t.value {
            Some(v) => format!("_:{v}"),
            None => format!("_:b{tid}"),
        },
        SerTermKind::Literal => {
            let lit = format!("\"{}\"", escape_literal(t.value.as_deref().unwrap_or("")));
            if let Some(lang) = &t.lang {
                match t.direction.as_deref().filter(|d| is_literal_direction(d)) {
                    Some(direction) => format!("{lit}@{lang}--{direction}"),
                    None => format!("{lit}@{lang}"),
                }
            } else if let Some(dt) = t.datatype {
                format!("{lit}^^{}", render_term(g, dt))
            } else {
                lit // plain literal == xsd:string
            }
        }
        // quoted triple (RDF 1.2 triple term), resolved through its reifier
        SerTermKind::Triple => match t.reifier.and_then(|rf| g.reifier(rf)) {
            Some((s, p, o)) => {
                format!(
                    "<<( {} {} {} )>>",
                    render_term(g, s),
                    render_term(g, p),
                    render_term(g, o)
                )
            }
            // degraded but syntactically valid: an unbound reifier becomes a blank node
            None => format!("_:unbound_triple_{tid}"),
        },
    }
}

/// Serialise a [`SerGraph`] to N-Quads text.
pub(crate) fn to_nquads(g: &SerGraph) -> String {
    let mut lines: Vec<String> = Vec::new();
    for &(s, p, o, gname) in &g.quads {
        let triple = format!(
            "{} {} {}",
            render_term(g, s),
            render_term(g, p),
            render_term(g, o)
        );
        match gname {
            Some(gv) => lines.push(format!("{triple} {} .", render_term(g, gv))),
            None => lines.push(format!("{triple} .")),
        }
    }
    for &(rid, (s, p, o), gname) in &g.reifiers {
        if g.terms
            .get(rid)
            .is_some_and(|term| term.kind == SerTermKind::Triple && term.reifier == Some(rid))
        {
            continue;
        }
        let quoted = format!(
            "<<( {} {} {} )>>",
            render_term(g, s),
            render_term(g, p),
            render_term(g, o)
        );
        let triple = format!("{} <{RDF_REIFIES}> {quoted}", render_term(g, rid));
        match gname {
            Some(gv) => lines.push(format!("{triple} {} .", render_term(g, gv))),
            None => lines.push(format!("{triple} .")),
        }
    }
    for &(r, p, v, gname) in &g.annotations {
        let triple = format!(
            "{} {} {}",
            render_term(g, r),
            render_term(g, p),
            render_term(g, v)
        );
        match gname {
            Some(gv) => lines.push(format!("{triple} {} .", render_term(g, gv))),
            None => lines.push(format!("{triple} .")),
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// Assert that no row of `g` carries a named-graph slot — the single-graph syntaxes
/// (N-Triples, Turtle) cannot serialize named-graph quads. Mirrors the upstream
/// `ensure_default_graph_projection` rejection.
fn ensure_default_graph_projection(g: &SerGraph, format: &str) -> Result<(), RdfDiagnostic> {
    let named = g.quads.iter().any(|(_, _, _, gname)| gname.is_some())
        || g.reifiers.iter().any(|(_, _, gname)| gname.is_some())
        || g.annotations.iter().any(|(_, _, _, gname)| gname.is_some());
    if named {
        return Err(RdfDiagnostic::error(
            "native-codec-serialize",
            format!("{format} cannot serialize a named graph"),
        ));
    }
    Ok(())
}

/// Serialise a [`SerGraph`] to N-Triples text (default graph only).
pub(crate) fn to_ntriples(g: &SerGraph) -> Result<String, RdfDiagnostic> {
    ensure_default_graph_projection(g, "N-Triples")?;
    Ok(to_nquads(g))
}

/// Serialise a [`SerGraph`] to Turtle text (default graph only); the N-Quads body is
/// prefixed with the `rdf:`/`xsd:` `@prefix` header. IRIs in the body stay full
/// `<...>` — they are NOT abbreviated against the declared prefixes.
pub(crate) fn to_turtle(g: &SerGraph) -> Result<String, RdfDiagnostic> {
    ensure_default_graph_projection(g, "Turtle")?;
    let body = to_nquads(g);
    if body.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!(
            "@prefix rdf: <{RDF_NS}> .\n@prefix xsd: <{XSD_NS}> .\n\n{body}"
        ))
    }
}

// ── TriG ──────────────────────────────────────────────────────────────────────────

fn render_trig_term(g: &SerGraph, tid: usize) -> String {
    let t = &g.terms[tid];
    match t.kind {
        SerTermKind::Iri if t.value.as_deref() == Some(RDF_REIFIES) => "rdf:reifies".to_string(),
        SerTermKind::Iri => format!("<{}>", escape_iri(t.value.as_deref().unwrap_or(""))),
        SerTermKind::Bnode => match &t.value {
            Some(v) => format!("_:{v}"),
            None => format!("_:b{tid}"),
        },
        SerTermKind::Literal => {
            let lit = format!("\"{}\"", escape_literal(t.value.as_deref().unwrap_or("")));
            if let Some(lang) = &t.lang {
                match t.direction.as_deref().filter(|d| is_literal_direction(d)) {
                    Some(direction) => format!("{lit}@{lang}--{direction}"),
                    None => format!("{lit}@{lang}"),
                }
            } else if let Some(dt) = t.datatype {
                format!("{lit}^^{}", render_trig_term(g, dt))
            } else {
                lit
            }
        }
        SerTermKind::Triple => match t.reifier.and_then(|rf| g.reifier(rf)) {
            Some((s, p, o)) => format!(
                "<<( {} {} {} )>>",
                render_trig_term(g, s),
                render_trig_term(g, p),
                render_trig_term(g, o)
            ),
            None => render_term(g, tid),
        },
    }
}

fn close_graph(out: &mut Vec<String>, open_graph: &mut Option<String>) {
    if open_graph.take().is_some() {
        out.push("}".to_string());
    }
}

fn push_statement(
    out: &mut Vec<String>,
    open_graph: &mut Option<String>,
    graph: &SerGraph,
    graph_name: Option<usize>,
    statement: String,
) {
    if let Some(gid) = graph_name {
        let rendered_graph = render_trig_term(graph, gid);
        if open_graph.as_deref() != Some(rendered_graph.as_str()) {
            close_graph(out, open_graph);
            out.push(format!("{rendered_graph} {{"));
            *open_graph = Some(rendered_graph);
        }
        out.push(format!("  {statement}"));
    } else {
        close_graph(out, open_graph);
        out.push(statement);
    }
}

/// Serialise a [`SerGraph`] to TriG text.
pub(crate) fn to_trig(g: &SerGraph) -> String {
    if g.quads.is_empty() && g.reifiers.is_empty() && g.annotations.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("@prefix rdf: <{RDF_NS}> ."), String::new()];
    let mut open_graph: Option<String> = None;

    for &(s, p, o, gname) in &g.quads {
        let triple = format!(
            "{} {} {} .",
            render_trig_term(g, s),
            render_trig_term(g, p),
            render_trig_term(g, o)
        );
        push_statement(&mut lines, &mut open_graph, g, gname, triple);
    }

    for &(rid, (s, p, o), gname) in &g.reifiers {
        // A triple TERM keys its own components under its own id (a self-reference, not
        // a reifier relationship); rendering it as `<<( … )>> rdf:reifies <<( … )>>`
        // would assert a triple term in subject position. Its components are already
        // carried inline wherever the term appears, so skip the entry.
        if g.terms
            .get(rid)
            .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
        {
            continue;
        }
        let quoted = format!(
            "<<( {} {} {} )>>",
            render_trig_term(g, s),
            render_trig_term(g, p),
            render_trig_term(g, o)
        );
        let statement = format!("{} rdf:reifies {quoted} .", render_trig_term(g, rid));
        push_statement(&mut lines, &mut open_graph, g, gname, statement);
    }
    for &(r, p, v, gname) in &g.annotations {
        let statement = format!(
            "{} {} {} .",
            render_trig_term(g, r),
            render_trig_term(g, p),
            render_trig_term(g, v)
        );
        push_statement(&mut lines, &mut open_graph, g, gname, statement);
    }

    close_graph(&mut lines, &mut open_graph);
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_blank_label_matches_zero_timestamp_ulid() {
        // The raw blank-label shape is byte-identity critical: the W3C canonical
        // comparison relabels blanks and will NOT catch a label-shape regression, so
        // these exact strings are checked directly. Each is the 26-digit Crockford
        // Base32 rendering of the zero-timestamp ULID built from the counter.
        assert_eq!(
            deterministic_blank_label(0),
            "gts_00000000000000000000000000"
        );
        assert_eq!(
            deterministic_blank_label(1),
            "gts_00000000000000000000000001"
        );
        assert_eq!(
            deterministic_blank_label(31),
            "gts_0000000000000000000000000Z"
        );
        assert_eq!(
            deterministic_blank_label(32),
            "gts_00000000000000000000000010"
        );
        assert_eq!(
            deterministic_blank_label(1000),
            "gts_000000000000000000000000Z8"
        );
    }

    /// A single-quad graph `<s> <p> "<lit>"` over default-graph terms.
    fn lit_graph(lexical: &str, datatype_iri: &str) -> SerGraph {
        let mut g = SerGraph::default();
        // 0: s, 1: p, 2: datatype IRI, 3: literal
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some(datatype_iri.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some(lexical.to_owned()),
            datatype: Some(2),
            lang: None,
            direction: None,
            reifier: None,
        });
        g.quads.push((0, 1, 3, None));
        g
    }

    #[test]
    fn decimal_lexical_preserved_verbatim_in_ntriples() {
        // The trailing zero of "0.90"^^xsd:decimal MUST survive verbatim: no
        // value-space canonicalization, no datatype narrowing.
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(
            nt.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "raw N-Triples output must carry the verbatim lexical form, got: {nt}"
        );
    }

    #[test]
    fn turtle_begins_with_prefix_header() {
        let g = lit_graph("0.90", "http://www.w3.org/2001/XMLSchema#decimal");
        let ttl = to_turtle(&g).expect("turtle");
        let expected = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n";
        assert!(
            ttl.starts_with(expected),
            "Turtle must begin with the two @prefix lines, got: {ttl}"
        );
        // The IRI body stays full <...>, NOT abbreviated against the declared prefixes.
        assert!(
            ttl.contains("\"0.90\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "Turtle body keeps the verbatim lexical form + full datatype IRI"
        );
    }

    #[test]
    fn empty_turtle_is_empty_string() {
        let g = SerGraph::default();
        assert_eq!(to_turtle(&g).expect("turtle"), "");
    }

    #[test]
    fn ntriples_rejects_named_graph() {
        let mut g = lit_graph("x", "http://www.w3.org/2001/XMLSchema#string");
        // Re-point the literal as a plain literal and add a named-graph quad.
        g.quads.push((0, 1, 0, Some(0)));
        assert!(
            to_ntriples(&g).is_err(),
            "N-Triples must reject a named-graph quad"
        );
    }

    #[test]
    fn language_tag_with_direction_renders() {
        let mut g = SerGraph::default();
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/s".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Iri,
            value: Some("https://e/p".to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        });
        g.terms.push(SerTerm {
            kind: SerTermKind::Literal,
            value: Some("hi".to_owned()),
            datatype: None,
            lang: Some("en".to_owned()),
            direction: Some("ltr".to_owned()),
            reifier: None,
        });
        g.quads.push((0, 1, 2, None));
        let nt = to_ntriples(&g).expect("ntriples");
        assert!(nt.contains("\"hi\"@en--ltr"), "got: {nt}");
    }
}

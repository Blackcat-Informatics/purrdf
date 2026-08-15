// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL Results **XML** (SRX) reader — the inverse of [`crate::xml`].
//!
//! Parses a W3C SPARQL Query Results XML document
//! (<https://www.w3.org/TR/rdf-sparql-XMLres/>) into a
//! [`crate::json_read::ParsedSolutions`] (`SELECT`) or a boolean (`ASK`). The W3C
//! conformance harness reads expected `.srx` results with it.
//!
//! # Wasm discipline
//!
//! A hand-rolled XML scanner over `&[u8]` building a minimal DOM tree — **no
//! external XML crate, no `std::io`** — keeping the crate wasm-clean and
//! oxigraph-free. The SRX grammar is shallow and fixed, so a tree-walk is enough.

use purrdf_core::{BlankScope, RdfTextDirection, TermValue};

use crate::error::Error;
use crate::json_read::ParsedSolutions;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANGSTRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
/// The ITS (Internationalization Tag Set) namespace IRI the SPARQL 1.2 Query
/// Results specification uses for the base-direction attribute — see
/// [`crate::xml`]'s module docs for the spec quote. Matched by namespace URI,
/// not by the literal `its:` prefix spelling — see [`Element::attr_ns`].
const ITS_NS: &str = "http://www.w3.org/2005/11/its";

/// Parse a SPARQL Results XML `SELECT` document into [`ParsedSolutions`].
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed XML, a non-`<sparql>` root, an `ASK`
/// (`<boolean>`) document (use [`from_xml_boolean`]), or a malformed binding.
pub fn from_xml(bytes: &[u8]) -> Result<ParsedSolutions, Error> {
    let root = parse_root(bytes)?;
    if root.child("boolean").is_some() {
        return Err(fmt(
            "expected SELECT results, got an ASK (boolean) document",
        ));
    }
    let variables = read_head_vars(&root)?;
    let results = root
        .child("results")
        .ok_or_else(|| fmt("missing <results>"))?;

    let mut rows = Vec::new();
    for result in results.children_named("result") {
        let mut row = vec![None; variables.len()];
        for binding in result.children_named("binding") {
            let name = binding
                .attr("name")
                .ok_or_else(|| fmt("<binding> without name"))?;
            let idx = variables
                .iter()
                .position(|v| v == name)
                .ok_or_else(|| fmt("<binding> names an undeclared variable"))?;
            // A `<binding>` with no child term element means the variable is
            // unbound in this solution — an older convention (conformant
            // SPARQL-XML simply omits the `<binding>`).  Treat it as absent.
            let Some(term_elem) = binding.child_elements().next() else {
                continue;
            };
            row[idx] = Some(decode_term(term_elem)?);
        }
        rows.push(row);
    }
    Ok(ParsedSolutions { variables, rows })
}

/// Parse a SPARQL Results XML `ASK` document into its boolean.
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed XML or a document without a
/// `<boolean>` element.
pub fn from_xml_boolean(bytes: &[u8]) -> Result<bool, Error> {
    let root = parse_root(bytes)?;
    let boolean = root
        .child("boolean")
        .ok_or_else(|| fmt("missing <boolean>"))?;
    match boolean.text().trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(fmt(&format!("invalid <boolean> value `{other}`"))),
    }
}

/// Parse the document and return its `<sparql>` root element.
fn parse_root(bytes: &[u8]) -> Result<Element, Error> {
    let text = core::str::from_utf8(bytes).map_err(|_| fmt("non-UTF-8 document"))?;
    let root = XmlParser::new(text).parse_document()?;
    if root.name != "sparql" {
        return Err(fmt("root element is not <sparql>"));
    }
    Ok(root)
}

/// Read the `<head>`'s `<variable name="…"/>` declarations, in order.
fn read_head_vars(root: &Element) -> Result<Vec<String>, Error> {
    let Some(head) = root.child("head") else {
        return Ok(Vec::new());
    };
    Ok(head
        .children_named("variable")
        .filter_map(|v| v.attr("name").map(str::to_owned))
        .collect())
}

/// Decode a single bound-term element (`<uri>`/`<bnode>`/`<literal>`/`<triple>`).
fn decode_term(elem: &Element) -> Result<TermValue, Error> {
    match elem.name.as_str() {
        "uri" => Ok(TermValue::Iri(elem.text())),
        "bnode" => Ok(TermValue::Blank {
            label: elem.text(),
            scope: BlankScope::DEFAULT,
        }),
        "literal" => {
            let language = elem.attr("xml:lang").map(str::to_owned);
            // `its:dir` — resolved by NAMESPACE URI (`ITS_NS`) plus local name
            // `dir`, not by the literal QName spelling — is the spelling the
            // SPARQL 1.2 Query Results specification uses for RDF 1.2 base
            // direction (see [`crate::xml`]'s module docs for the spec
            // quote), so it is preferred. Matching by namespace URI means a
            // document that binds the ITS namespace to a non-`its` prefix (or
            // declares it on an ancestor element, e.g. the document root —
            // this crate's own writer's default style) is read correctly:
            // XML namespace semantics are defined by URI, not prefix spelling.
            // The bare `dir` and `purrdf:dir` spellings are tolerated ONLY
            // because this crate's own earlier writer emitted them before
            // this de-minting pass — never because they are spec-sanctioned.
            let dir_str = elem
                .attr_ns(ITS_NS, "dir")
                .or_else(|| elem.attr("dir"))
                .or_else(|| elem.attr("purrdf:dir"));
            let direction = match dir_str {
                Some("ltr") => Some(RdfTextDirection::Ltr),
                Some("rtl") => Some(RdfTextDirection::Rtl),
                Some(other) => return Err(fmt(&format!("unknown base direction `{other}`"))),
                None => None,
            };
            let datatype = match elem.attr("datatype") {
                Some(dt) => dt.to_owned(),
                None if language.is_some() && direction.is_some() => RDF_DIR_LANGSTRING.to_owned(),
                None if language.is_some() => RDF_LANGSTRING.to_owned(),
                None => XSD_STRING.to_owned(),
            };
            Ok(TermValue::Literal {
                lexical_form: elem.text(),
                datatype,
                language,
                direction,
            })
        }
        "triple" => {
            let s = decode_term(component(elem, "subject")?)?;
            let p = decode_term(component(elem, "predicate")?)?;
            let o = decode_term(component(elem, "object")?)?;
            if !matches!(p, TermValue::Iri(_)) {
                return Err(fmt("triple-term predicate is not an IRI"));
            }
            Ok(TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        }
        other => Err(fmt(&format!("unexpected term element <{other}>"))),
    }
}

/// Read the single child term element of a `<triple>` component wrapper.
fn component<'a>(triple: &'a Element, role: &str) -> Result<&'a Element, Error> {
    triple
        .child(role)
        .ok_or_else(|| fmt(&format!("<triple> missing <{role}>")))?
        .child_elements()
        .next()
        .ok_or_else(|| fmt(&format!("<{role}> has no term")))
}

/// Build a `Format` error.
fn fmt(msg: &str) -> Error {
    Error::Format(format!("SPARQL-XML: {msg}"))
}

/// A minimal parsed XML element: name, attributes (raw QName spelling plus a
/// namespace-resolved view), and ordered child nodes.
#[derive(Debug)]
struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    /// Every attribute resolved to `(namespace URI, local name, value)` using
    /// the `xmlns:*` bindings in scope at this element (its own declarations
    /// plus every ancestor's — XML namespace scope is inherited downward).
    /// `namespace` is `None` for an unprefixed attribute (XML Namespaces never
    /// puts an unprefixed ATTRIBUTE in a namespace, unlike elements, which
    /// inherit a default `xmlns`) or a prefix with no in-scope binding. See
    /// [`Self::attr_ns`].
    ns_attrs: Vec<(Option<String>, String, String)>,
    children: Vec<Node>,
}

#[derive(Debug)]
enum Node {
    Element(Element),
    Text(String),
}

impl Element {
    /// The value of an attribute, if present, matched by its RAW QName
    /// (literal prefix spelling). Used for attributes this reader matches
    /// positionally (`name`, `xml:lang`, `datatype`) or for legacy fallback
    /// spellings that were never namespace-qualified in the first place. For
    /// an attribute whose correctness depends on XML namespace semantics
    /// (defined by URI, not prefix), use [`Self::attr_ns`] instead.
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The value of the attribute whose namespace URI is `ns` and local name
    /// is `local`, resolved via the `xmlns:*` bindings in scope at this
    /// element — matching by namespace URI rather than by the literal QName
    /// prefix, exactly as XML Namespaces (<https://www.w3.org/TR/xml-names/>)
    /// defines attribute identity. A document that binds `ns` to a different
    /// prefix than the writer's own convention, or declares the binding on an
    /// ancestor element rather than this one, still resolves correctly.
    fn attr_ns(&self, ns: &str, local: &str) -> Option<&str> {
        self.ns_attrs.iter().find_map(|(uri, name, value)| {
            (uri.as_deref() == Some(ns) && name == local).then_some(value.as_str())
        })
    }

    /// The first direct child element named `name`.
    fn child(&self, name: &str) -> Option<&Self> {
        self.child_elements().find(|e| e.name == name)
    }

    /// All direct child elements named `name`.
    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Self> {
        self.child_elements().filter(move |e| e.name == name)
    }

    /// All direct child elements (text nodes skipped).
    fn child_elements(&self) -> impl Iterator<Item = &Self> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    /// The concatenated direct text content (entity-unescaped during parse).
    fn text(&self) -> String {
        let mut s = String::new();
        for n in &self.children {
            if let Node::Text(t) = n {
                s.push_str(t);
            }
        }
        s
    }
}

/// A hand-rolled XML tree parser over `&str`, covering the shallow SRX grammar
/// plus the standard prolog/comment/entity surface.
struct XmlParser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> XmlParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    /// Skip the prolog (`<?xml?>`, comments, doctype, whitespace) and parse the
    /// single root element.
    fn parse_document(&mut self) -> Result<Element, Error> {
        self.skip_misc()?;
        if self.peek() != Some(b'<') {
            return Err(fmt("expected root element"));
        }
        self.parse_element(&[])
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.bytes[self.pos..].starts_with(s.as_bytes())
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

    /// Skip XML declarations, comments, processing instructions, and doctype.
    fn skip_misc(&mut self) -> Result<(), Error> {
        loop {
            self.skip_ws();
            if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("<!--") {
                self.skip_until("-->")?;
            } else if self.starts_with("<!") {
                self.skip_until(">")?;
            } else {
                return Ok(());
            }
        }
    }

    fn skip_until(&mut self, close: &str) -> Result<(), Error> {
        if let Some(rel) = self.src[self.pos..].find(close) {
            self.pos += rel + close.len();
            Ok(())
        } else {
            Err(fmt(&format!("unterminated `{close}`")))
        }
    }

    /// Parse an element starting at `<`. `parent_scope` is the `(prefix,
    /// namespace URI)` bindings in scope from ancestor elements; this
    /// element's own `xmlns:*` declarations extend it (see
    /// [`extend_ns_scope`]) both for resolving its own attributes and for
    /// every descendant this call parses.
    fn parse_element(&mut self, parent_scope: &[(String, String)]) -> Result<Element, Error> {
        self.pos += 1; // consume '<'
        let name = self.parse_name()?;
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'/') => {
                    // Self-closing element.
                    self.pos += 1;
                    self.expect(b'>')?;
                    let ns_attrs = resolve_ns_attrs(&attrs, parent_scope);
                    return Ok(Element {
                        name,
                        attrs,
                        ns_attrs,
                        children: Vec::new(),
                    });
                }
                Some(b'>') => {
                    self.pos += 1;
                    break;
                }
                Some(_) => {
                    let (k, v) = self.parse_attr()?;
                    attrs.push((k, v));
                }
                None => return Err(fmt("unterminated start tag")),
            }
        }
        let scope = extend_ns_scope(parent_scope, &attrs);
        let ns_attrs = resolve_ns_attrs(&attrs, &scope);
        // Parse children until the matching end tag.
        let mut children = Vec::new();
        loop {
            match self.peek() {
                None => return Err(fmt("unterminated element")),
                Some(b'<') => {
                    if self.starts_with("</") {
                        self.pos += 2;
                        let close = self.parse_name()?;
                        self.skip_ws();
                        self.expect(b'>')?;
                        if close != name {
                            return Err(fmt(&format!(
                                "mismatched end tag </{close}> for <{name}>"
                            )));
                        }
                        break;
                    } else if self.starts_with("<!--") {
                        self.skip_until("-->")?;
                    } else if self.starts_with("<![CDATA[") {
                        let start = self.pos + "<![CDATA[".len();
                        let rel = self.src[start..]
                            .find("]]>")
                            .ok_or_else(|| fmt("unterminated CDATA"))?;
                        children.push(Node::Text(self.src[start..start + rel].to_owned()));
                        self.pos = start + rel + "]]>".len();
                    } else {
                        children.push(Node::Element(self.parse_element(&scope)?));
                    }
                }
                Some(_) => {
                    let text = self.parse_text()?;
                    if !text.is_empty() {
                        children.push(Node::Text(text));
                    }
                }
            }
        }
        Ok(Element {
            name,
            attrs,
            ns_attrs,
            children,
        })
    }

    /// Parse character data up to the next `<`, unescaping entities.
    fn parse_text(&mut self) -> Result<String, Error> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == b'<' {
                break;
            }
            self.pos += 1;
        }
        unescape(&self.src[start..self.pos])
    }

    fn parse_name(&mut self) -> Result<String, Error> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>' | b'=') {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(fmt("empty element/attribute name"));
        }
        Ok(self.src[start..self.pos].to_owned())
    }

    fn parse_attr(&mut self) -> Result<(String, String), Error> {
        let key = self.parse_name()?;
        self.skip_ws();
        self.expect(b'=')?;
        self.skip_ws();
        let Some(quote @ (b'"' | b'\'')) = self.peek() else {
            return Err(fmt("attribute value must be quoted"));
        };
        self.pos += 1;
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                break;
            }
            self.pos += 1;
        }
        if self.peek() != Some(quote) {
            return Err(fmt("unterminated attribute value"));
        }
        let value = unescape(&self.src[start..self.pos])?;
        self.pos += 1; // consume closing quote
        Ok((key, value))
    }

    fn expect(&mut self, c: u8) -> Result<(), Error> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(fmt(&format!("expected `{}`", c as char)))
        }
    }
}

/// Extend `parent_scope` with any `xmlns:prefix="uri"` declarations found
/// among `attrs` (this element's own start tag) — the namespace scope every
/// descendant, and this element's own attributes, resolve against.
fn extend_ns_scope(
    parent_scope: &[(String, String)],
    attrs: &[(String, String)],
) -> Vec<(String, String)> {
    let mut scope = parent_scope.to_vec();
    for (k, v) in attrs {
        if let Some(prefix) = k.strip_prefix("xmlns:") {
            scope.push((prefix.to_owned(), v.clone()));
        }
    }
    scope
}

/// Resolve every attribute in `attrs` to `(namespace URI, local name, value)`
/// using `scope` (the nearest — i.e. last-pushed — binding for a prefix
/// wins, matching XML's "nearest declaration in scope" resolution rule).
/// `xmlns`/`xmlns:*` declaration attributes themselves are namespace
/// machinery, not ordinary content attributes, so they are excluded here.
fn resolve_ns_attrs(
    attrs: &[(String, String)],
    scope: &[(String, String)],
) -> Vec<(Option<String>, String, String)> {
    attrs
        .iter()
        .filter(|(k, _)| k != "xmlns" && !k.starts_with("xmlns:"))
        .map(|(k, v)| match k.split_once(':') {
            Some((prefix, local)) => {
                let ns = scope
                    .iter()
                    .rev()
                    .find(|(p, _)| p == prefix)
                    .map(|(_, uri)| uri.clone());
                (ns, local.to_owned(), v.clone())
            }
            None => (None, k.clone(), v.clone()),
        })
        .collect()
}

/// Unescape the five predefined XML entities plus numeric character references.
fn unescape(s: &str) -> Result<String, Error> {
    if !s.contains('&') {
        return Ok(s.to_owned());
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        let semi = after
            .find(';')
            .ok_or_else(|| fmt("unterminated entity reference"))?;
        let entity = &after[1..semi];
        match entity {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                let code = u32::from_str_radix(&entity[2..], 16)
                    .map_err(|_| fmt("bad hex character reference"))?;
                out.push(char::from_u32(code).ok_or_else(|| fmt("invalid character reference"))?);
            }
            _ if entity.starts_with('#') => {
                let code = entity[1..]
                    .parse::<u32>()
                    .map_err(|_| fmt("bad character reference"))?;
                out.push(char::from_u32(code).ok_or_else(|| fmt("invalid character reference"))?);
            }
            other => return Err(fmt(&format!("unknown entity &{other};"))),
        }
        rest = &after[semi + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_select_with_mixed_terms() {
        let srx = r#"<?xml version="1.0"?>
        <sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head>
            <variable name="s"/>
            <variable name="name"/>
            <variable name="label"/>
            <variable name="age"/>
          </head>
          <results>
            <result>
              <binding name="s"><uri>http://example.org/s</uri></binding>
              <binding name="name"><literal>Ada</literal></binding>
              <binding name="label"><literal xml:lang="fr">bonjour</literal></binding>
              <binding name="age"><literal datatype="http://www.w3.org/2001/XMLSchema#integer">42</literal></binding>
            </result>
            <result>
              <binding name="s"><bnode>b0</bnode></binding>
            </result>
          </results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(parsed.variables, vec!["s", "name", "label", "age"]);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Iri("http://example.org/s".to_owned()))
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
            parsed.rows[1][0],
            Some(TermValue::Blank {
                label: "b0".to_owned(),
                scope: BlankScope::DEFAULT,
            })
        );
        assert_eq!(parsed.rows[1][1], None);
    }

    #[test]
    fn reads_triple_term() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="t"/></head>
          <results><result><binding name="t"><triple>
            <subject><uri>http://ex/s</uri></subject>
            <predicate><uri>http://ex/p</uri></predicate>
            <object><literal>o</literal></object>
          </triple></binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Triple {
                s: Box::new(TermValue::Iri("http://ex/s".to_owned())),
                p: Box::new(TermValue::Iri("http://ex/p".to_owned())),
                o: Box::new(TermValue::Literal {
                    lexical_form: "o".to_owned(),
                    datatype: XSD_STRING.to_owned(),
                    language: None,
                    direction: None,
                }),
            })
        );
    }

    /// `its:dir` (the SPARQL 1.2 Query Results spec spelling) is preferred and
    /// yields `rdf:dirLangString` off the datatype ladder.
    #[test]
    fn reads_its_dir_directional_literal() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="ar" xmlns:its="http://www.w3.org/2005/11/its" its:dir="rtl">قطة</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "قطة".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// The legacy `dir`/`purrdf:dir` spellings this crate's own earlier writer
    /// emitted are tolerated on read, `its:dir` taking priority when both are
    /// present.
    #[test]
    fn tolerates_legacy_dir_spellings() {
        let bare_dir = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="en" dir="ltr">hello</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(bare_dir.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Ltr),
            })
        );

        let purrdf_dir = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="en" purrdf:dir="ltr" xmlns:purrdf="https://purrdf.dev/ns/results#">hello</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(purrdf_dir.as_bytes()).expect("parse");
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

    /// When `its:dir` and a legacy spelling disagree on one literal, the spec
    /// spelling wins.
    #[test]
    fn its_dir_takes_priority_over_legacy_spellings() {
        let both = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="en" xmlns:its="http://www.w3.org/2005/11/its" its:dir="rtl" dir="ltr" purrdf:dir="ltr" xmlns:purrdf="https://purrdf.dev/ns/results#">hello</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(both.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "hello".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("en".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// H9 fix pin: the ITS namespace declared on the ROOT `<sparql>` element
    /// — this crate's own writer's default style (see [`crate::xml`]'s module
    /// docs) — must still resolve for a `<literal its:dir="…">` several
    /// elements deeper, since XML namespace scope is inherited downward.
    #[test]
    fn reads_its_dir_declared_on_root() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#" xmlns:its="http://www.w3.org/2005/11/its" its:version="2.0">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="ar" its:dir="rtl">قطة</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "قطة".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// H9 fix pin: XML namespace semantics are defined by URI, not prefix
    /// spelling (<https://www.w3.org/TR/xml-names/>). A document that binds
    /// the ITS namespace to a NON-`its` prefix (here `i`) must still be read
    /// correctly — matching by `(namespace URI, local name)` rather than the
    /// raw QName `its:dir`.
    #[test]
    fn reads_its_dir_bound_to_non_its_prefix() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="ar" xmlns:i="http://www.w3.org/2005/11/its" i:dir="rtl">قطة</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Literal {
                lexical_form: "قطة".to_owned(),
                datatype: RDF_DIR_LANGSTRING.to_owned(),
                language: Some("ar".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            })
        );
    }

    /// H9 fix pin, combined: the ITS namespace bound to a non-`its` prefix
    /// AND declared on the root element rather than inline on the literal —
    /// the fully general case a raw-QName match cannot handle at all.
    #[test]
    fn reads_its_dir_non_its_prefix_declared_on_root() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#" xmlns:ns7="http://www.w3.org/2005/11/its" ns7:version="2.0">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal xml:lang="en" ns7:dir="ltr">hello</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
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
    fn unescapes_entities() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head><variable name="x"/></head>
          <results><result><binding name="x">
            <literal>a &lt; b &amp; c &#65;</literal>
          </binding></result></results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        let TermValue::Literal { lexical_form, .. } = parsed.rows[0][0].clone().unwrap() else {
            panic!("expected literal");
        };
        assert_eq!(lexical_form, "a < b & c A");
    }

    #[test]
    fn reads_ask_boolean() {
        let yes = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>true</boolean></sparql>"#;
        assert!(from_xml_boolean(yes.as_bytes()).expect("ask"));
        let no = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>false</boolean></sparql>"#;
        assert!(!from_xml_boolean(no.as_bytes()).expect("ask"));
    }

    #[test]
    fn select_reader_rejects_ask_document() {
        let srx = r#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>true</boolean></sparql>"#;
        assert!(matches!(from_xml(srx.as_bytes()), Err(Error::Format(_))));
    }

    /// An empty `<binding name="x"></binding>` element means the variable is
    /// unbound in that solution — the older producer convention where unbound
    /// variables are emitted as an empty element rather than being omitted.
    /// The reader must treat it identically to an absent binding: no value for
    /// that variable in the row.
    #[test]
    fn empty_binding_treated_as_unbound() {
        let srx = r#"<?xml version="1.0"?>
        <sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head>
            <variable name="s"/>
            <variable name="o1"/>
            <variable name="o2"/>
          </head>
          <results>
            <result>
              <binding name="s"><uri>http://example.org/s1</uri></binding>
              <binding name="o1"><literal>present</literal></binding>
              <binding name="o2"></binding>
            </result>
            <result>
              <binding name="s"><uri>http://example.org/s2</uri></binding>
              <binding name="o1"><literal>also-present</literal></binding>
            </result>
          </results>
        </sparql>"#;
        let parsed = from_xml(srx.as_bytes()).expect("parse");
        assert_eq!(parsed.variables, vec!["s", "o1", "o2"]);
        assert_eq!(parsed.rows.len(), 2);
        // Row 0: s and o1 bound, o2 explicitly empty → must be unbound (None).
        assert_eq!(
            parsed.rows[0][0],
            Some(TermValue::Iri("http://example.org/s1".to_owned()))
        );
        assert_eq!(
            parsed.rows[0][1],
            Some(TermValue::Literal {
                lexical_form: "present".to_owned(),
                datatype: XSD_STRING.to_owned(),
                language: None,
                direction: None,
            })
        );
        assert_eq!(parsed.rows[0][2], None, "empty <binding> must be unbound");
        // Row 1: o2 absent entirely → also unbound (regression check).
        assert_eq!(
            parsed.rows[1][0],
            Some(TermValue::Iri("http://example.org/s2".to_owned()))
        );
        assert_eq!(parsed.rows[1][2], None, "absent binding must be unbound");
    }
}

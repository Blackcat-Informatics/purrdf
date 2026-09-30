// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL Results **XML** (SRX) reader — the inverse of [`crate::xml`].
//!
//! Parses a W3C SPARQL Query Results XML document
//! (<https://www.w3.org/TR/rdf-sparql-XMLres/>) into a
//! [`crate::json_read::ParsedSolutions`] (`SELECT`) or a boolean (`ASK`). The W3C
//! conformance harness reads expected `.srx` results with it.
//!
//! # The XML layer
//!
//! The document is read by the workspace's one XML reader,
//! [`purrdf_lex::xml`]: XML 1.0 and Namespaces in XML 1.0 well-formedness,
//! character and entity references expanded, line ends and attribute values
//! normalized, and every element and attribute resolved to its namespace name.
//! A document type declaration is refused ([`purrdf_lex::xml::Dtd::Refuse`]):
//! a results document has no use for one, and refusing it means no entity is
//! ever expanded. The SRX grammar is shallow and fixed, so a walk of the tree
//! is enough; its elements are matched by local name in the SRX namespace
//! (or in no namespace, as some producers write them).

use purrdf_core::TermBox;
use purrdf_core::terminals;
use purrdf_core::{BlankScope, RdfTextDirection, TermValue};
use purrdf_lex::xml::{Document, Node, XML_NAMESPACE};

use crate::error::Error;
use crate::json_read::ParsedSolutions;
use crate::model::{ProvenanceNamespace, ResultProvenance, SolutionProvenance};

use purrdf_core::datatype::XSD_STRING;
/// The ITS (Internationalization Tag Set) namespace IRI the SPARQL 1.2 Query
/// Results specification uses for the base-direction attribute — see
/// [`crate::xml`]'s module docs for the spec quote. Matched by namespace URI,
/// not by the literal `its:` prefix spelling.
use purrdf_core::vocab::its::NS as ITS_NS;
use purrdf_core::vocab::language_datatype_iri;

/// The namespace of every SPARQL Query Results XML element (SPARQL 1.1 Query
/// Results XML Format §2).
const SRX_NS: &str = "http://www.w3.org/2005/sparql-results#";

/// Parse a SPARQL Results XML `SELECT` document into [`ParsedSolutions`].
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed XML, a non-`<sparql>` root, an `ASK`
/// (`<boolean>`) document (use [`from_xml_boolean`]), or a malformed binding.
pub fn from_xml(bytes: &[u8]) -> Result<ParsedSolutions, Error> {
    let document = parse(bytes)?;
    let root = sparql_root(&document)?;
    if child(root, "boolean").is_some() {
        return Err(fmt(
            "expected SELECT results, got an ASK (boolean) document",
        ));
    }
    let variables = read_head_vars(root);
    let results = child(root, "results").ok_or_else(|| fmt("missing <results>"))?;

    let mut rows = Vec::new();
    for result in children_named(results, "result") {
        let mut row = vec![None; variables.len()];
        for binding in children_named(result, "binding") {
            let name = binding
                .attribute("name")
                .ok_or_else(|| fmt("<binding> without name"))?;
            let idx = variables
                .iter()
                .position(|v| v == name)
                .ok_or_else(|| fmt("<binding> names an undeclared variable"))?;
            // A `<binding>` with no child term element means the variable is
            // unbound in this solution — an older convention (conformant
            // SPARQL-XML simply omits the `<binding>`).  Treat it as absent.
            let Some(term_elem) = binding.element_children().next() else {
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
/// # Surrounding whitespace, and exactly which four code points that is
///
/// `<boolean>` carries an `xsd:boolean`, whose `whiteSpace` facet is `collapse`,
/// and both the collapsing and the XML that delivers it are defined over the
/// same enumerated terminal — XML 1.0 §2.3, production 3:
///
/// > `S ::= (#x20 | #x9 | #xD | #xA)+`
///
/// So `<boolean> true </boolean>` and `<boolean>\n  false\n</boolean>` are the
/// ordinary pretty-printed spellings and are read as written, while a value
/// padded with any OTHER whitespace-looking scalar — U+00A0 NO-BREAK SPACE,
/// U+2028 LINE SEPARATOR, U+3000 IDEOGRAPHIC SPACE, U+000C FORM FEED — is not
/// an `xsd:boolean` lexical form at all and is refused with the offending text
/// quoted back.
///
/// The trim is therefore [`purrdf_iri::terminals::is_ws`] and not [`str::trim`],
/// which is defined over [`char::is_whitespace`] and so strips twenty-six code
/// points. That is not merely a wider accepted language: this trim decides where
/// the *value* starts and stops, so the wide class silently re-reads
/// `\u{2028}true` as the boolean `true`, in a reader whose own scanner never
/// treated U+2028 as markup separation. The narrow class is the one the rest of
/// this file already uses.
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed XML, a document without a
/// `<boolean>` element, or a `<boolean>` whose text is not `true` or `false`
/// once XML `S` is stripped from both ends.
pub fn from_xml_boolean(bytes: &[u8]) -> Result<bool, Error> {
    let document = parse(bytes)?;
    let boolean =
        child(sparql_root(&document)?, "boolean").ok_or_else(|| fmt("missing <boolean>"))?;
    let text = text(boolean);
    match terminals::trim_ws(&text) {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(fmt(&format!("invalid <boolean> value `{other}`"))),
    }
}

/// Decode the additive `<provenance>` element (under `namespace.iri()`) a
/// [`ProvenanceNamespace`]-keyed SRX document carries, or
/// [`ResultProvenance::default`] when no such element is present.
///
/// The inverse of [`crate::xml::to_xml`]'s additive extension: the writer emits
/// `<{prefix}:provenance xmlns:{prefix}="{iri}">` (and matching QNames for its
/// members) only when both the source `provenance` was non-empty and a
/// namespace was supplied, so a reader must be told the SAME namespace to know
/// which element to decode. Matched by NAMESPACE URI plus local name — exactly
/// as `its:dir` is resolved — not by the literal QName
/// prefix spelling, because XML namespace identity is URI-based
/// (<https://www.w3.org/TR/xml-names/>): a document that binds
/// `namespace.iri()` to a DIFFERENT prefix than this crate's own writer uses
/// still decodes correctly, and a document that reuses `namespace.prefix()`'s
/// spelling for an UNRELATED namespace (e.g. the actual W3C PROV namespace,
/// `http://www.w3.org/ns/prov#`, bound to `prov:` — the single most common
/// real-world `prov:` binding) is correctly NOT read as this caller's
/// extension. A document with no `provenance` element under `namespace.iri()`
/// (never written, written under a different namespace, or written by
/// something other than this crate's own writer) decodes to the empty
/// provenance rather than guessing.
///
/// # Errors
///
/// Returns [`Error::Format`] on malformed XML or a non-`<sparql>` root.
pub fn provenance_from_xml(
    bytes: &[u8],
    namespace: &ProvenanceNamespace,
) -> Result<ResultProvenance, Error> {
    let document = parse(bytes)?;
    let root = sparql_root(&document)?;
    let iri = namespace.iri();
    let named = |node, local| children_in(node, iri, local);
    let Some(provenance_elem) = named(root, "provenance").next() else {
        return Ok(ResultProvenance::default());
    };
    let query_hash = named(provenance_elem, "queryHash").next().map(text);
    let engine = named(provenance_elem, "engine").next().map(text);
    let solutions = named(provenance_elem, "solution")
        .map(|solution_elem| SolutionProvenance {
            sources: named(solution_elem, "source").map(text).collect(),
        })
        .collect();
    Ok(ResultProvenance {
        query_hash,
        engine,
        solutions,
    })
}

/// Read `bytes` as an XML document (a document type declaration refused).
fn parse(bytes: &[u8]) -> Result<Document<'_>, Error> {
    let text = core::str::from_utf8(bytes).map_err(|_| fmt("non-UTF-8 document"))?;
    Document::parse(text).map_err(|error| {
        let (line, column) = error.line_column(text);
        fmt(&format!("{error} (line {line}, column {column})"))
    })
}

/// The document's `<sparql>` root element.
fn sparql_root<'d, 'a>(document: &'d Document<'a>) -> Result<Node<'d, 'a>, Error> {
    let root = document.root_element();
    if !is_srx(root, "sparql") {
        return Err(fmt("root element is not <sparql>"));
    }
    Ok(root)
}

/// Read the `<head>`'s `<variable name="…"/>` declarations, in order.
fn read_head_vars(root: Node<'_, '_>) -> Vec<String> {
    child(root, "head").map_or_else(Vec::new, |head| {
        children_named(head, "variable")
            .filter_map(|v| v.attribute("name").map(str::to_owned))
            .collect()
    })
}

/// Whether `node` is the SRX element `local`: that local name, in the SRX
/// namespace or in none.
fn is_srx(node: Node<'_, '_>, local: &str) -> bool {
    let name = node.tag_name();
    name.name() == local && matches!(name.namespace(), None | Some(SRX_NS))
}

/// The direct child SRX elements `local`.
fn children_named<'d, 'a>(
    node: Node<'d, 'a>,
    local: &'static str,
) -> impl Iterator<Item = Node<'d, 'a>> {
    node.element_children()
        .filter(move |child| is_srx(*child, local))
}

/// The direct child elements `local` in the namespace `namespace`.
fn children_in<'d, 'a, 'q>(
    node: Node<'d, 'a>,
    namespace: &'q str,
    local: &'q str,
) -> impl Iterator<Item = Node<'d, 'a>> + use<'d, 'a, 'q> {
    node.element_children()
        .filter(move |child| child.has_tag_name((namespace, local)))
}

/// The first direct child SRX element `local`.
fn child<'d, 'a>(node: Node<'d, 'a>, local: &'static str) -> Option<Node<'d, 'a>> {
    children_named(node, local).next()
}

/// The concatenated direct text content, every reference already expanded.
fn text(node: Node<'_, '_>) -> String {
    node.children().filter_map(|child| child.text()).collect()
}

/// Decode a single bound-term element (`<uri>`/`<bnode>`/`<literal>`/`<triple>`).
///
/// A `<triple>` is decoded over a work list of the triples being decoded: each finds
/// its `<subject>` term element and decodes it — a nested `<triple>` there fully —
/// then does the same for its `<predicate>` and its `<object>`, and is checked and
/// assembled once all three are decoded. The first error ends the decoding.
fn decode_term(elem: Node<'_, '_>) -> Result<TermValue, Error> {
    /// A `<triple>` being decoded, and the components decoded so far.
    struct Frame<'d, 'a> {
        elem: Node<'d, 'a>,
        decoded: Vec<TermValue>,
    }
    let mut frames: Vec<Frame<'_, '_>> = Vec::new();
    let mut next = elem;
    loop {
        let Some(mut term) = decode_leaf(next)? else {
            let first = component(next, "subject")?;
            frames.push(Frame {
                elem: next,
                decoded: Vec::with_capacity(3),
            });
            next = first;
            continue;
        };
        loop {
            let Some(frame) = frames.last_mut() else {
                return Ok(term);
            };
            frame.decoded.push(term);
            match frame.decoded.len() {
                1 => {
                    next = component(frame.elem, "predicate")?;
                    break;
                }
                2 => {
                    next = component(frame.elem, "object")?;
                    break;
                }
                _ => {}
            }
            let innermost = frames.pop().expect("the innermost triple is being decoded");
            let [s, p, o] = <[TermValue; 3]>::try_from(innermost.decoded)
                .unwrap_or_else(|_| unreachable!("a triple is assembled from three components"));
            if !matches!(p, TermValue::Iri(_)) {
                return Err(fmt("triple-term predicate is not an IRI"));
            }
            term = TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            };
        }
    }
}

/// Decode a bound-term element that is not a `<triple>`, or `None` for a `<triple>`.
fn decode_leaf(elem: Node<'_, '_>) -> Result<Option<TermValue>, Error> {
    let name = elem.tag_name();
    if !matches!(name.namespace(), None | Some(SRX_NS)) {
        return Err(fmt(&format!("unexpected term element <{}>", name.name())));
    }
    Ok(Some(match name.name() {
        "uri" => TermValue::Iri(text(elem)),
        "bnode" => TermValue::Blank {
            label: text(elem),
            scope: BlankScope::DEFAULT,
        },
        "literal" => {
            let language = elem.attribute((XML_NAMESPACE, "lang")).map(str::to_owned);
            // A tag arriving in a DOCUMENT is parsed input, held to the same
            // grammar as every other parsed tag in the workspace — see
            // [`crate::error::language_tag_refusal`] for why a results reader
            // is on that list.
            if let Some(detail) = language
                .as_deref()
                .and_then(crate::error::language_tag_refusal)
            {
                return Err(fmt(&detail));
            }
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
                .attribute((ITS_NS, "dir"))
                .or_else(|| elem.attribute("dir"))
                .or_else(|| {
                    elem.attributes()
                        .iter()
                        .find(|attribute| attribute.qname() == "purrdf:dir")
                        .map(purrdf_lex::xml::Attribute::value)
                });
            let direction = match dir_str {
                Some(token) => Some(
                    RdfTextDirection::from_str_token(token)
                        .ok_or_else(|| fmt(&format!("unknown base direction `{token}`")))?,
                ),
                None => None,
            };
            let datatype = match elem.attribute("datatype") {
                Some(dt) => dt.to_owned(),
                None if language.is_some() => language_datatype_iri(direction.is_some()).to_owned(),
                None => XSD_STRING.to_owned(),
            };
            TermValue::Literal {
                lexical_form: text(elem),
                datatype,
                language,
                direction,
            }
        }
        "triple" => return Ok(None),
        other => return Err(fmt(&format!("unexpected term element <{other}>"))),
    }))
}

/// Read the single child term element of a `<triple>` component wrapper.
fn component<'d, 'a>(triple: Node<'d, 'a>, role: &'static str) -> Result<Node<'d, 'a>, Error> {
    let wrapper = child(triple, role).ok_or_else(|| fmt(&format!("<triple> missing <{role}>")))?;
    wrapper
        .element_children()
        .next()
        .ok_or_else(|| fmt(&format!("<{role}> has no term")))
}

/// Build a `Format` error.
fn fmt(msg: &str) -> Error {
    Error::Format(format!("SPARQL-XML: {msg}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::to_xml;
    use purrdf_core::SparqlResult;
    use purrdf_core::TermBox;
    use purrdf_core::vocab::rdf::{
        DIR_LANG_STRING as RDF_DIR_LANGSTRING, LANG_STRING as RDF_LANGSTRING,
    };

    /// A one-literal SELECT document whose literal content is `content`.
    fn srx_literal(content: &str) -> Vec<u8> {
        format!(
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\
             <head><variable name=\"l\"/></head><results><result>\
             <binding name=\"l\"><literal>{content}</literal></binding>\
             </result></results></sparql>"
        )
        .into_bytes()
    }

    fn literal_text(document: &[u8]) -> Result<String, Error> {
        let parsed = from_xml(document)?;
        match parsed.rows[0][0].clone() {
            Some(TermValue::Literal { lexical_form, .. }) => Ok(lexical_form),
            other => panic!("expected a literal, got {other:?}"),
        }
    }

    #[test]
    fn a_character_reference_to_a_non_char_or_a_misspelled_one_is_refused() {
        let decimal_zero = format!("&#{};", 0);
        for text in [
            "&#x0;",
            &decimal_zero,
            "&#x1;",
            "&#xFFFE;",
            "&#xFFFF;",
            "&#xD800;",
            "&#X41;",
            "&#x+41;",
            "&#+65;",
            "&#x-41;",
            "&#-65;",
        ] {
            assert!(
                matches!(literal_text(&srx_literal(text)), Err(Error::Format(_))),
                "{text}"
            );
        }
    }

    #[test]
    fn a_character_reference_to_a_char_still_decodes() {
        for (text, expected) in [
            ("&#x9;", "\t"),
            ("&#xA;", "\n"),
            ("&#xD;", "\r"),
            ("&#x20;", " "),
            ("&#xFFFD;", "\u{fffd}"),
            ("&#x10000;", "\u{10000}"),
            ("&#x10FFFF;", "\u{10ffff}"),
            ("&#x0010FFFF;", "\u{10ffff}"),
            ("&#x41;", "A"),
        ] {
            assert_eq!(
                literal_text(&srx_literal(text)).expect("a lawful reference"),
                expected,
                "{text}"
            );
        }
        // The decimal form is spelled through `format!` so the source holds no `#`
        // followed by digits.
        assert_eq!(
            literal_text(&srx_literal(&format!("&#{};", 65))).expect("a lawful reference"),
            "A"
        );
    }

    /// A results document has no use for a document type declaration, so one is
    /// refused (no entity it declares is ever expanded); the same document without
    /// it still reads.
    #[test]
    fn a_doctype_is_refused_and_the_document_without_it_reads() {
        let plain = srx_literal("x");
        assert_eq!(literal_text(&plain).expect("no DOCTYPE"), "x");
        for doctype in [
            "<!DOCTYPE sparql>",
            "<!DOCTYPE sparql [<!ENTITY x \"expanded\">]>",
        ] {
            let mut document = doctype.as_bytes().to_vec();
            document.extend_from_slice(&plain);
            assert!(
                matches!(from_xml(&document), Err(Error::Format(_))),
                "{doctype}"
            );
        }
    }

    /// The SRX elements are matched in the SRX namespace or in none; the same
    /// document under a foreign default namespace is not a results document.
    #[test]
    fn srx_elements_are_read_in_their_namespace_or_none() {
        let document = |namespace: &str| {
            format!(
                "<sparql{namespace}><head><variable name=\"x\"/></head><results><result>\
                 <binding name=\"x\"><uri>http://example.org/a</uri></binding>\
                 </result></results></sparql>"
            )
            .into_bytes()
        };
        let expected = Some(TermValue::Iri("http://example.org/a".to_owned()));
        for namespace in ["", " xmlns=\"http://www.w3.org/2005/sparql-results#\""] {
            assert_eq!(
                from_xml(&document(namespace)).expect("reads").rows[0][0],
                expected
            );
        }
        let prefixed = b"<r:sparql xmlns:r=\"http://www.w3.org/2005/sparql-results#\">\
            <r:head/><r:boolean>true</r:boolean></r:sparql>";
        assert!(from_xml_boolean(prefixed).expect("a prefixed SRX document reads"));
        assert!(matches!(
            from_xml(&document(" xmlns=\"http://example.org/other\"")),
            Err(Error::Format(_))
        ));
    }

    /// Provenance round-trip: what [`crate::xml::to_xml`] writes under a namespace,
    /// [`provenance_from_xml`] reads back — the writer no longer emits
    /// something nothing can decode.
    #[test]
    fn provenance_round_trips_through_xml() {
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
        let outcome = to_xml(&result, &provenance, Some(&namespace)).expect("serializes");

        let decoded =
            provenance_from_xml(&outcome.bytes, &namespace).expect("provenance decodes back");
        assert_eq!(decoded, provenance);
    }

    /// A document with no `<{prefix}:provenance>` element decodes to the empty
    /// provenance rather than erroring.
    #[test]
    fn absent_provenance_element_decodes_to_default() {
        let namespace = ProvenanceNamespace::new("prov", "http://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>true</boolean></sparql>"#;
        let decoded = provenance_from_xml(doc, &namespace).expect("decodes");
        assert!(decoded.is_empty());
    }

    /// Namespace-by-URI pin (false NEGATIVE, mirroring
    /// [`reads_its_dir_bound_to_non_its_prefix`]): a document that binds the
    /// caller's OWN namespace IRI to a prefix OTHER than the one this crate's
    /// writer happens to use (`p:` instead of `prov:`) must still decode —
    /// XML namespace identity is URI-based, not QName-spelling-based.
    #[test]
    fn provenance_reads_correctly_under_an_alternate_prefix_for_the_same_namespace() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>true</boolean>
          <p:provenance xmlns:p="https://example.org/ns/prov#">
            <p:queryForm>ask</p:queryForm>
            <p:queryHash>deadbeef</p:queryHash>
            <p:engine>purrdf-sparql-eval</p:engine>
          </p:provenance>
        </sparql>"#;
        let decoded = provenance_from_xml(doc, &namespace).expect("decodes");
        assert_eq!(
            decoded,
            ResultProvenance {
                query_hash: Some("deadbeef".to_owned()),
                engine: Some("purrdf-sparql-eval".to_owned()),
                solutions: Vec::new(),
            }
        );
    }

    /// Namespace-by-URI pin (false POSITIVE): a document that reuses this
    /// crate's writer's OWN prefix spelling (`prov:`) but binds it to an
    /// UNRELATED namespace — here the actual W3C PROV namespace
    /// (`http://www.w3.org/ns/prov#`), the single most common real-world
    /// `prov:` binding — must NOT be read as the caller's provenance
    /// extension just because the QName spelling matches; only the bound
    /// namespace IRI identifies it.
    #[test]
    fn foreign_namespace_under_the_writers_own_prefix_is_not_read_as_provenance() {
        let namespace = ProvenanceNamespace::new("prov", "https://example.org/ns/prov#")
            .expect("valid namespace");
        let doc = br#"<sparql xmlns="http://www.w3.org/2005/sparql-results#">
          <head></head><boolean>true</boolean>
          <prov:provenance xmlns:prov="http://www.w3.org/ns/prov#">
            <prov:queryHash>deadbeef</prov:queryHash>
          </prov:provenance>
        </sparql>"#;
        let decoded = provenance_from_xml(doc, &namespace).expect("decodes without error");
        assert!(
            decoded.is_empty(),
            "a `prov:provenance` element bound to the W3C PROV namespace must not be \
             mistaken for this caller's own `prov` namespace extension"
        );
    }

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
                s: TermBox::new(TermValue::Iri("http://ex/s".to_owned())),
                p: TermBox::new(TermValue::Iri("http://ex/p".to_owned())),
                o: TermBox::new(TermValue::Literal {
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

    /// Namespace-inheritance pin: the ITS namespace declared on the ROOT `<sparql>` element
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

    /// Namespace-by-URI pin: XML namespace semantics are defined by URI, not prefix
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

    /// Combined namespace pin: the ITS namespace bound to a non-`its` prefix
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

    /// `<boolean>`'s surrounding whitespace is XML `S`, and only XML `S`.
    ///
    /// > `S ::= (#x20 | #x9 | #xD | #xA)+` (XML 1.0 §2.3, production 3)
    ///
    /// An `xsd:boolean`'s `whiteSpace` facet is `collapse`, which collapses over
    /// that production — not over the Unicode `White_Space` property [`str::trim`]
    /// implements. So a `<boolean>` padded with U+00A0 or U+2028 is not an
    /// `xsd:boolean` lexical form and must be refused rather than silently read.
    ///
    /// Pinned in both directions, because this is a trim and a trim is where
    /// over-refusal hides: every pretty-printed spelling a real producer emits
    /// still parses.
    #[test]
    fn only_xml_s_pads_an_ask_boolean() {
        let doc = |inner: &str| {
            format!(
                "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\
                 <head></head><boolean>{inner}</boolean></sparql>"
            )
        };
        // The refusal vectors: whitespace-looking scalars XML `S` does not name.
        for padded in [
            "\u{a0}true",     // NO-BREAK SPACE
            "\u{2028}true",   // LINE SEPARATOR
            "true\u{2029}",   // PARAGRAPH SEPARATOR
            "\u{3000}false",  // IDEOGRAPHIC SPACE
            "\u{c}true",      // FORM FEED — ASCII whitespace, not XML `S`
            "\u{b}true",      // VERTICAL TAB — Unicode whitespace, not XML `S`
            " \u{a0} false ", // mixed with the real thing
        ] {
            assert!(
                matches!(
                    from_xml_boolean(doc(padded).as_bytes()),
                    Err(Error::Format(_))
                ),
                "{padded:?} is not an xsd:boolean lexical form and must be refused"
            );
        }
        // The VALID neighbours: every run of the four still reads, in both
        // polarities, including the multi-line form a pretty-printer emits.
        for (inner, expected) in [
            ("true", true),
            (" true ", true),
            ("\ttrue\t", true),
            ("\r\ntrue\r\n", true),
            ("\n  false\n", false),
            ("false", false),
            (" \t\r\n false \t\r\n ", false),
        ] {
            assert_eq!(
                from_xml_boolean(doc(inner).as_bytes()).expect("a padded boolean still reads"),
                expected,
                "{inner:?} must still read as {expected}"
            );
        }
        // And the refusal still names a non-boolean for what it is.
        assert!(matches!(
            from_xml_boolean(doc("maybe").as_bytes()),
            Err(Error::Format(_))
        ));
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

    /// Tags the `LANGTAG` grammar refuses, and the neighbouring ones it must
    /// still take — the same two corpora the SRJ reader and the
    /// `STRLANG`/`STRLANGDIR` gate use, because there is one accept set.
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

    fn srx_with_lang(tag: &str) -> Vec<u8> {
        format!(
            "<sparql xmlns=\"http://www.w3.org/2005/sparql-results#\">\
             <head><variable name=\"l\"/></head><results><result>\
             <binding name=\"l\"><literal xml:lang=\"{tag}\">x</literal></binding>\
             </result></results></sparql>"
        )
        .into_bytes()
    }

    /// A results DOCUMENT is parsed input, whatever syntax it arrives in.
    #[test]
    fn a_refused_language_tag_in_a_document_is_refused_with_its_diagnostic_code() {
        for tag in REFUSED_TAGS {
            let error = from_xml(&srx_with_lang(tag))
                .expect_err("a tag the grammar refuses must not decode");
            let Error::Format(message) = &error else {
                panic!("expected Error::Format for {tag:?}, got {error:?}");
            };
            assert!(
                message.starts_with("SPARQL-XML: invalid language tag "),
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

    /// The over-refusal half: the private-use and terminal-only shapes a
    /// too-strict profile would silently start dropping on ingress.
    #[test]
    fn every_well_formed_language_tag_still_reads_back() {
        for tag in ACCEPTED_TAGS {
            let parsed = from_xml(&srx_with_lang(tag))
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
}

#[cfg(test)]
mod term_walk_tests {
    //! The term-element decoder against its recursive reference.

    use purrdf_core::{TermBox, TermValue};

    use purrdf_lex::xml::{Document, Node};

    use super::{Error, component, decode_leaf, decode_term, fmt};

    fn reference(elem: Node<'_, '_>) -> Result<TermValue, Error> {
        if let Some(term) = decode_leaf(elem)? {
            return Ok(term);
        }
        let s = reference(component(elem, "subject")?)?;
        let p = reference(component(elem, "predicate")?)?;
        let o = reference(component(elem, "object")?)?;
        if !matches!(p, TermValue::Iri(_)) {
            return Err(fmt("triple-term predicate is not an IRI"));
        }
        Ok(TermValue::Triple {
            s: TermBox::new(s),
            p: TermBox::new(p),
            o: TermBox::new(o),
        })
    }

    /// A term's result-document element, with every `<object>` wrapper dropped when
    /// `lose_objects` holds, so the decoder meets a missing component.
    fn element_text(value: &TermValue, lose_objects: bool) -> String {
        let escape = |text: &str| {
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        match value {
            TermValue::Iri(iri) => format!("<uri>{}</uri>", escape(iri)),
            TermValue::Blank { label, .. } => format!("<bnode>{}</bnode>", escape(label)),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                ..
            } => match language {
                Some(tag) => format!(
                    "<literal xml:lang=\"{}\">{}</literal>",
                    escape(tag),
                    escape(lexical_form)
                ),
                None => format!(
                    "<literal datatype=\"{}\">{}</literal>",
                    escape(datatype),
                    escape(lexical_form)
                ),
            },
            TermValue::Triple { s, p, o } => format!(
                "<triple><subject>{}</subject><predicate>{}</predicate>{}</triple>",
                element_text(s, lose_objects),
                element_text(p, lose_objects),
                if lose_objects {
                    String::new()
                } else {
                    format!("<object>{}</object>", element_text(o, lose_objects))
                }
            ),
        }
    }

    /// Every generated term's element — whole, and with its objects lost — decodes to
    /// exactly the term or the refusal the recursive reference reaches.
    #[test]
    fn the_decoder_agrees_with_its_recursive_reference_on_generated_elements() {
        let (mut decoded, mut refused) = (0, 0);
        for seed in 0..400_u64 {
            let mut state = seed;
            let mut budget = 6;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::Any,
            );
            for lose_objects in [false, true] {
                let text = element_text(&value, lose_objects);
                let document =
                    Document::parse(&text).expect("a generated element is well-formed XML");
                let elem = document.root_element();
                let found = decode_term(elem);
                assert_eq!(
                    format!("{found:?}"),
                    format!("{:?}", reference(elem)),
                    "seed {seed}: {text}"
                );
                decoded += usize::from(found.is_ok());
                refused += usize::from(found.is_err());
            }
        }
        assert!(decoded > 0, "some generated element decodes");
        assert!(refused > 0, "some generated element is refused");
    }
}

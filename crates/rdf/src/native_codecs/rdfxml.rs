// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party RDF/XML codec (W3C RDF 1.1 / RDF-XML grammar) — .
//!
//! This module REPLACES the external purrdf-gts `rdf_codecs::{from_rdf_xml,
//! to_rdf_xml}` codecs (the first-party mandate: RDF/XML must NOT be parsed or
//! serialized via the external crate). It implements the RDF/XML production rules
//! in-repo on top of a pure-Rust XML DOM (`roxmltree`), parsing straight into the
//! frozen [`RdfDataset`](crate::RdfDataset) IR (via the shared
//! [`fold_statement_layer`](super::parse::fold_statement_layer)) and serializing from
//! the first-party [`SerGraph`](super::ser_model::SerGraph). It is fully purrdf-gts
//! free; byte-identity with the purrdf-gts path that produced the conformance and
//! regenerate corpora is held by the W3C RDF/XML suite + RDFC-1.0 + round-trip tests.
//!
//! ## Parse: XML → frozen IR through the shared statement-layer fold
//!
//! The parser walks the grammar accumulating first-party `(subject, predicate, object)`
//! rows ([`XmlRow`] over [`XmlTerm`]): base triples, classic RDF-1.0 reification as
//! PLAIN quads (`rdf:type rdf:Statement` / `rdf:subject` / `rdf:predicate` /
//! `rdf:object`), and RDF-1.2 reifiers as `<reifier> rdf:reifies <<( s p o )>>` rows
//! (object = [`XmlTerm::Triple`]). A final pass interns each row into a
//! [`RdfDatasetBuilder`] and hands the resulting [`FoldRow`]s to
//! [`fold_statement_layer`](super::parse::fold_statement_layer) — pass 1 binds the
//! `rdf:reifies` rows as reifiers, pass 2 keeps everything else (incl. classic
//! reification) as base quads — then freezes. No intermediate GTS graph.
//!
//! ## Grammar coverage
//!
//! `rdf:RDF` root, `rdf:Description`, typed-node elements, property elements,
//! `rdf:about`/`rdf:resource`/`rdf:ID`/`rdf:nodeID`, property attributes,
//! `rdf:datatype`, `xml:lang`, `its:dir` (RDF 1.2 base direction),
//! `rdf:parseType="Resource"|"Literal"|"Collection"|"Triple"`, RDF 1.0 `rdf:ID`
//! reification, RDF 1.2 `rdf:annotation`/`rdf:annotationNodeID` reifiers, list
//! expansion, node/property striping, base-IRI resolution, and `xmlns` prefix
//! scoping.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;

use roxmltree::{Document, Node};

use super::codec::RdfCodec;
use super::parse::{FoldNode, FoldRow, RDF_REIFIES as RDF_REIFIES_IRI, fold_statement_layer};
use super::ser_model::{SerGraph, SerTerm, SerTermKind, deterministic_blank_label_with_prefix};
use super::text_parse::LineParseMode;
use crate::nesting::guard_xml_nesting;
use crate::{RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfLiteral, RdfTextDirection, TermId};
use purrdf_core::blank_label::{LabelAlphabet, is_valid_label};
use purrdf_core::cdt_blank::BlankBinding;

/// The RDF/XML codec: a standalone (non-line-family) [`RdfCodec`] over the in-repo W3C
/// RDF/XML grammar. RDF/XML is treated as star-INcapable under the transcode loss
/// contract (`*→rdfxml` is `rdf12-star-unrepresentable`), and its parser carries no
/// span-recording tokenizer.
pub(super) struct RdfXmlCodec;

impl RdfCodec for RdfXmlCodec {
    fn parse(
        &self,
        text: &str,
        base: &mut purrdf_iri::BaseScope,
        _mode: LineParseMode,
    ) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        super::parse::parse_rdfxml_without_panicking(text, base)
    }

    fn serialize_into(&self, graph: &SerGraph, out: &mut String) -> Result<(), RdfDiagnostic> {
        // Built whole, then appended. Unlike the four text formats, this one's document
        // is assembled as a TREE — XML nesting, or a `serde_json` value — so its writer
        // cannot emit a prefix before it knows what follows, and appending would mean
        // rebuilding the construction itself rather than redirecting its output. The
        // sink still earns its place here: the caller's buffer is the only one that
        // outlives the call, and this is the seam a streaming writer replaces.
        out.push_str(&serialize_ser_graph_to_rdfxml(graph)?);
        Ok(())
    }
}

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const ITS_NS: &str = "http://www.w3.org/2005/11/its";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

const RDF_DESCRIPTION: &str = "Description";
const RDF_ABOUT: &str = "about";
const RDF_ID: &str = "ID";
const RDF_NODE_ID: &str = "nodeID";
const RDF_RESOURCE: &str = "resource";
const RDF_DATATYPE: &str = "datatype";
const RDF_PARSE_TYPE: &str = "parseType";
const RDF_TYPE: &str = "type";
const RDF_VERSION: &str = "version";
const RDF_ANNOTATION: &str = "annotation";
const RDF_ANNOTATION_NODE_ID: &str = "annotationNodeID";
const RDF_FIRST: &str = "first";
const RDF_REST: &str = "rest";
const RDF_NIL: &str = "nil";
const RDF_STATEMENT: &str = "Statement";
const RDF_SUBJECT: &str = "subject";
const RDF_PREDICATE: &str = "predicate";
const RDF_OBJECT: &str = "object";
const RDF_XML_LITERAL: &str = "XMLLiteral";
const XML_BASE: &str = "base";
const XML_LANG: &str = "lang";
const ITS_DIR: &str = "dir";
const ITS_VERSION: &str = "version";

fn parse_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-parse", format!("RDF/XML: {}", detail.into()))
}

fn serialize_err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "native-codec-serialize",
        format!("RDF/XML: {}", detail.into()),
    )
}

// ───────────────────────────────────────────────────────────────────────────────
// First-party RDF/XML term + row model (the parser's in-memory accumulation)
// ───────────────────────────────────────────────────────────────────────────────

/// RDF 1.2 base direction, parsed off `its:dir`. Mapped to the IR's
/// [`RdfTextDirection`] when a row interns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BaseDirection {
    Ltr,
    Rtl,
}

/// A first-party RDF term the parser accumulates before interning into the IR.
#[derive(Clone, Debug)]
enum XmlTerm {
    Iri(String),
    Blank(String),
    Literal(RdfLiteral),
    Triple(Box<(Self, Self, Self)>),
}

/// A subject / object node: an IRI or a blank node (never a literal / triple in
/// subject position).
#[derive(Clone, Debug)]
enum XmlNode {
    Iri(String),
    Blank(String),
}

impl From<XmlNode> for XmlTerm {
    fn from(node: XmlNode) -> Self {
        match node {
            XmlNode::Iri(iri) => Self::Iri(iri),
            XmlNode::Blank(label) => Self::Blank(label),
        }
    }
}

/// One asserted `(subject, predicate, object)` triple, default-graph only (RDF/XML is a
/// single-graph syntax). The `predicate` is its IRI string.
struct XmlRow {
    subject: XmlTerm,
    predicate: String,
    object: XmlTerm,
}

// ───────────────────────────────────────────────────────────────────────────────
// Parse: RDF/XML text → frozen RdfDataset IR (via the shared statement-layer fold)
// ───────────────────────────────────────────────────────────────────────────────

/// Parse RDF/XML `text` into a frozen [`RdfDataset`] applying the W3C RDF/XML grammar.
/// `base_iri` is the document base for relative-IRI / `rdf:ID` resolution.
///
/// The parser accumulates first-party [`XmlRow`]s — base triples, classic RDF-1.0
/// reification as plain quads, and RDF-1.2 reifiers as `rdf:reifies` rows — then interns
/// each into a [`RdfDatasetBuilder`] and folds them through the shared
/// [`fold_statement_layer`], identically to the line/Turtle-family path.
///
/// `base` is `&mut`: `xml:base` on the ROOT element establishes the document base, so on
/// return the scope holds the base in force at the end of the document. `xml:base` deeper
/// in the tree is subtree-scoped and has popped by then, which is exactly right — a base
/// that governed one element never governed the document.
pub(super) fn parse_rdfxml_to_dataset(
    text: &str,
    base: &mut purrdf_iri::BaseScope,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    // `roxmltree`'s tokenizer recurses once per element and aborts the process on a deeply
    // nested document, so the nesting is measured on the SOURCE and refused here. This is
    // also what bounds the walk below: the tree it descends came from this very call, so its
    // element nesting is bounded and it needs no depth counter of its own.
    guard_xml_nesting(text).map_err(|depth| {
        parse_err(format!(
            "element nesting reaches {depth} levels, past the parser limit"
        ))
    })?;
    let document = Document::parse(text).map_err(|e| parse_err(e.to_string()))?;
    let mut parser = RdfXmlParser {
        rows: Vec::new(),
        bnode_counter: 0,
        collection_counter: 0,
    };
    let context = ParseContext {
        base: base.clone(),
        ..Default::default()
    };
    *base = parser.parse_document(document.root_element(), &context)?;
    parser.freeze()
}

#[derive(Clone, Debug, Default)]
struct ParseContext {
    /// The base IRIs in scope for this element. `xml:base` on an element rebinds it
    /// for that element's subtree only, which falls out of the per-child clone.
    base: purrdf_iri::BaseScope,
    language: Option<String>,
    direction: Option<BaseDirection>,
    /// `rdf:version="1.2"` declared on this element or an ancestor: gates the RDF 1.2
    /// features (triple terms via `parseType="Triple"`, ITS base direction).
    rdf_version_12: bool,
    /// `its:version` declared (ITS 2.0 processing mode).
    its_version: bool,
}

impl ParseContext {
    fn for_child(&self, element: Node<'_, '_>) -> Result<Self, RdfDiagnostic> {
        let mut next = self.clone();
        // Version flags are sticky once declared on any ancestor.
        if attr_rdf(element, RDF_VERSION) == Some("1.2") {
            next.rdf_version_12 = true;
        }
        if attr_its(element, ITS_VERSION).is_some() {
            next.its_version = true;
        }
        if let Some(base) = attr_xml(element, XML_BASE) {
            // XML Base: the attribute may itself be relative, in which case it resolves
            // against the base in force on the parent element (RFC-3986 5.1.1).
            next.base
                .rebind(base, purrdf_iri::BaseOrigin::Enclosing)
                .map_err(|e| {
                    RdfDiagnostic::error(e.diagnostic_code(), format!("invalid xml:base: {e}"))
                })?;
        }
        if let Some(language) = attr_xml(element, XML_LANG) {
            next.language = (!language.is_empty()).then(|| language.to_string());
        }
        if let Some(direction) = attr_its(element, ITS_DIR) {
            let parsed = match direction {
                "ltr" => BaseDirection::Ltr,
                "rtl" => BaseDirection::Rtl,
                other => return Err(parse_err(format!("invalid ITS direction {other:?}"))),
            };
            // RDF 1.2 base direction is suppressed in ITS 2.0 mode (`its:version`)
            // unless the document explicitly opts into RDF 1.2 via `rdf:version="1.2"`.
            next.direction = if next.its_version && !next.rdf_version_12 {
                None
            } else {
                Some(parsed)
            };
        }
        Ok(next)
    }
}

struct RdfXmlParser {
    rows: Vec<XmlRow>,
    bnode_counter: usize,
    collection_counter: usize,
}

impl RdfXmlParser {
    /// Intern the accumulated rows into a fresh [`RdfDatasetBuilder`] and fold them
    /// through the shared [`fold_statement_layer`], then freeze. Term interning is
    /// shared across all rows, so identical terms collapse to one id (the same fold the
    /// line/Turtle-family path applies).
    fn freeze(self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        let mut builder = RdfDatasetBuilder::new();
        let mut fold_rows: Vec<FoldRow> = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let subject = intern_term(&mut builder, &row.subject)?;
            let predicate = builder.intern_iri(&row.predicate);
            let object = intern_node(&mut builder, &row.object)?;
            fold_rows.push(FoldRow {
                subject,
                is_reifies: row.predicate == RDF_REIFIES_IRI,
                predicate,
                object,
                graph: None,
            });
        }
        fold_statement_layer(&mut builder, fold_rows)?;
        builder.freeze()
    }

    /// Returns the base scope the ROOT element left in force — the document base, after
    /// any `xml:base` the root itself carries. Child elements get their own cloned
    /// context, so a subtree's `xml:base` never reaches this answer.
    fn parse_document(
        &mut self,
        root: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<purrdf_iri::BaseScope, RdfDiagnostic> {
        let context = context.for_child(root)?;
        if is_rdf(root, "RDF") {
            for child in element_children(root) {
                self.parse_node_element(child, &context)?;
            }
        } else {
            self.parse_node_element(root, &context)?;
        }
        Ok(context.base)
    }

    fn parse_node_element(
        &mut self,
        element: Node<'_, '_>,
        parent_context: &ParseContext,
    ) -> Result<XmlNode, RdfDiagnostic> {
        let context = parent_context.for_child(element)?;
        let subject = self.subject_for_node(element, &context)?;

        if !is_rdf(element, RDF_DESCRIPTION) {
            self.insert_statement(
                subject.clone().into(),
                rdf_iri(RDF_TYPE)?,
                XmlTerm::Iri(element_iri(element, &context.base)?),
                None,
                None,
            )?;
        }
        if let Some(type_iri) = attr_rdf(element, RDF_TYPE) {
            self.insert_statement(
                subject.clone().into(),
                rdf_iri(RDF_TYPE)?,
                XmlTerm::Iri(self.iri_ref(type_iri, &context)?),
                None,
                None,
            )?;
        }

        for attr in property_attrs(element) {
            let predicate = name_iri(attr.namespace(), attr.name(), &context.base)?;
            let literal = self.context_literal(attr.value(), None, &context)?;
            self.insert_statement(
                subject.clone().into(),
                predicate,
                XmlTerm::Literal(literal),
                None,
                None,
            )?;
        }

        for child in element_children(element) {
            self.parse_property_element(&subject, child, &context)?;
        }
        Ok(subject)
    }

    fn parse_property_element(
        &mut self,
        subject: &XmlNode,
        element: Node<'_, '_>,
        parent_context: &ParseContext,
    ) -> Result<(), RdfDiagnostic> {
        let context = parent_context.for_child(element)?;
        let predicate = element_iri(element, &context.base)?;
        let reifier = attr_rdf(element, RDF_ID)
            .map(|id| self.rdf_id_iri(id, &context).map(XmlNode::Iri))
            .transpose()?;
        // `rdf:annotation="IRI"` and `rdf:annotationNodeID="id"` both name the reifier
        // of the asserted triple; the former is an IRI, the latter a blank node.
        let annotation = match attr_rdf(element, RDF_ANNOTATION) {
            Some(annotation) => Some(XmlNode::Iri(self.iri_ref(annotation, &context)?)),
            None => match attr_rdf(element, RDF_ANNOTATION_NODE_ID) {
                Some(node_id) => Some(XmlNode::Blank(blank_label(node_id)?)),
                None => None,
            },
        };

        if let Some(resource) = attr_rdf(element, RDF_RESOURCE) {
            let object = XmlNode::Iri(self.iri_ref(resource, &context)?);
            self.insert_statement(
                subject.clone().into(),
                predicate,
                object.clone().into(),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }
        if let Some(node_id) = attr_rdf(element, RDF_NODE_ID) {
            let object = XmlNode::Blank(blank_label(node_id)?);
            self.insert_statement(
                subject.clone().into(),
                predicate,
                object.clone().into(),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }

        match attr_rdf(element, RDF_PARSE_TYPE) {
            Some("Resource") => {
                let object = self.fresh_bnode()?;
                self.insert_statement(
                    subject.clone().into(),
                    predicate,
                    object.clone().into(),
                    reifier,
                    annotation,
                )?;
                self.insert_property_attribute_statements(&object, element, &context)?;
                for child in element_children(element) {
                    self.parse_property_element(&object, child, &context)?;
                }
                return Ok(());
            }
            Some("Collection") => {
                let head = self.parse_collection(element, &context)?;
                return self.insert_statement(
                    subject.clone().into(),
                    predicate,
                    head,
                    reifier,
                    annotation,
                );
            }
            Some("Literal") => {
                let xml_literal = serialize_children_as_xml(element);
                let literal = RdfLiteral::typed(xml_literal, rdf_iri(RDF_XML_LITERAL)?);
                return self.insert_statement(
                    subject.clone().into(),
                    predicate,
                    XmlTerm::Literal(literal),
                    reifier,
                    annotation,
                );
            }
            Some("Triple") => {
                // A triple term is an RDF 1.2 feature: without `rdf:version="1.2"` the
                // whole property is ignored (W3C `rdf12-xml-tt-01`, "Ignored triple term").
                if !context.rdf_version_12 {
                    return Ok(());
                }
                let triple = self.parse_triple_element(element, &context)?;
                return self.insert_statement(
                    subject.clone().into(),
                    predicate,
                    XmlTerm::Triple(Box::new(triple)),
                    reifier,
                    annotation,
                );
            }
            Some(other) => {
                return Err(parse_err(format!("unsupported rdf:parseType {other:?}")));
            }
            None => {}
        }

        let element_children: Vec<Node<'_, '_>> = element_children(element).collect();
        if let Some(datatype) = attr_rdf(element, RDF_DATATYPE) {
            if !element_children.is_empty() {
                return Err(parse_err(
                    "rdf:datatype property cannot contain node elements",
                ));
            }
            let literal =
                RdfLiteral::typed(element_text(element), self.iri_ref(datatype, &context)?);
            return self.insert_statement(
                subject.clone().into(),
                predicate,
                XmlTerm::Literal(literal),
                reifier,
                annotation,
            );
        }

        if element_children.len() == 1 {
            let object = self.parse_node_element(element_children[0], &context)?;
            return self.insert_statement(
                subject.clone().into(),
                predicate,
                object.into(),
                reifier,
                annotation,
            );
        }
        if element_children.len() > 1 {
            return Err(parse_err(
                "property element contains more than one node element",
            ));
        }

        if property_attrs(element).next().is_some() {
            let object = self.fresh_bnode()?;
            self.insert_statement(
                subject.clone().into(),
                predicate,
                object.clone().into(),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }

        let literal = self.context_literal(&element_text(element), None, &context)?;
        self.insert_statement(
            subject.clone().into(),
            predicate,
            XmlTerm::Literal(literal),
            reifier,
            annotation,
        )
    }

    fn insert_property_attribute_statements(
        &mut self,
        subject: &XmlNode,
        element: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<(), RdfDiagnostic> {
        for attr in property_attrs(element) {
            let literal = self.context_literal(attr.value(), None, context)?;
            self.insert_statement(
                subject.clone().into(),
                name_iri(attr.namespace(), attr.name(), &context.base)?,
                XmlTerm::Literal(literal),
                None,
                None,
            )?;
        }
        Ok(())
    }

    fn parse_collection(
        &mut self,
        element: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<XmlTerm, RdfDiagnostic> {
        let items: Vec<Node<'_, '_>> = element_children(element).collect();
        if items.is_empty() {
            return Ok(XmlTerm::Iri(rdf_iri(RDF_NIL)?));
        }
        let nodes = (0..items.len())
            .map(|_| self.fresh_collection_bnode())
            .collect::<Result<Vec<_>, _>>()?;
        for (index, item) in items.iter().enumerate() {
            let object = self.parse_node_element(*item, context)?;
            self.insert_statement(
                nodes[index].clone().into(),
                rdf_iri(RDF_FIRST)?,
                object.into(),
                None,
                None,
            )?;
            let rest: XmlTerm = if let Some(next) = nodes.get(index + 1) {
                next.clone().into()
            } else {
                XmlTerm::Iri(rdf_iri(RDF_NIL)?)
            };
            self.insert_statement(
                nodes[index].clone().into(),
                rdf_iri(RDF_REST)?,
                rest,
                None,
                None,
            )?;
        }
        Ok(nodes
            .first()
            .expect("non-empty collection has a head node")
            .clone()
            .into())
    }

    fn parse_triple_element(
        &mut self,
        element: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<(XmlTerm, XmlTerm, XmlTerm), RdfDiagnostic> {
        let nodes: Vec<Node<'_, '_>> = element_children(element).collect();
        if nodes.len() != 1 {
            return Err(parse_err(
                "rdf:parseType=\"Triple\" requires one node element",
            ));
        }
        let node = nodes[0];
        let triple_subject = self.subject_for_node(node, context)?;
        let node_ctx = context.for_child(node)?;

        // The single predicate/object may come from a child property element, a
        // `rdf:type` attribute, or another property attribute (literal-valued).
        let type_attr = attr_rdf(node, RDF_TYPE);
        let prop_attrs: Vec<roxmltree::Attribute<'_, '_>> = property_attrs(node).collect();
        let child_props: Vec<Node<'_, '_>> = element_children(node).collect();
        if usize::from(type_attr.is_some()) + prop_attrs.len() + child_props.len() != 1 {
            return Err(parse_err(
                "rdf:parseType=\"Triple\" requires exactly one predicate/object",
            ));
        }
        let (predicate, object): (String, XmlTerm) = if let Some(type_iri) = type_attr {
            (
                rdf_iri(RDF_TYPE)?,
                XmlTerm::Iri(self.iri_ref(type_iri, &node_ctx)?),
            )
        } else if let Some(attr) = prop_attrs.first() {
            (
                name_iri(attr.namespace(), attr.name(), &node_ctx.base)?,
                XmlTerm::Literal(self.context_literal(attr.value(), None, &node_ctx)?),
            )
        } else {
            (
                element_iri(child_props[0], &node_ctx.base)?,
                self.triple_object(child_props[0], context)?,
            )
        };
        Ok((triple_subject.into(), XmlTerm::Iri(predicate), object))
    }

    fn triple_object(
        &mut self,
        property: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<XmlTerm, RdfDiagnostic> {
        let context = context.for_child(property)?;
        if let Some(resource) = attr_rdf(property, RDF_RESOURCE) {
            return Ok(XmlTerm::Iri(self.iri_ref(resource, &context)?));
        }
        if let Some(node_id) = attr_rdf(property, RDF_NODE_ID) {
            return Ok(XmlTerm::Blank(blank_label(node_id)?));
        }
        if attr_rdf(property, RDF_PARSE_TYPE) == Some("Triple") {
            return Ok(XmlTerm::Triple(Box::new(
                self.parse_triple_element(property, &context)?,
            )));
        }
        let nodes: Vec<Node<'_, '_>> = element_children(property).collect();
        if nodes.len() == 1 {
            let object = self.subject_for_node(nodes[0], &context)?;
            return Ok(object.into());
        }
        if nodes.len() > 1 {
            return Err(parse_err(
                "rdf:parseType=\"Triple\" object has multiple node elements",
            ));
        }
        Ok(XmlTerm::Literal(self.context_literal(
            &element_text(property),
            attr_rdf(property, RDF_DATATYPE),
            &context,
        )?))
    }

    fn subject_for_node(
        &mut self,
        element: Node<'_, '_>,
        context: &ParseContext,
    ) -> Result<XmlNode, RdfDiagnostic> {
        if let Some(about) = attr_rdf(element, RDF_ABOUT) {
            return Ok(XmlNode::Iri(self.iri_ref(about, context)?));
        }
        if let Some(id) = attr_rdf(element, RDF_ID) {
            return Ok(XmlNode::Iri(self.rdf_id_iri(id, context)?));
        }
        if let Some(node_id) = attr_rdf(element, RDF_NODE_ID) {
            return Ok(XmlNode::Blank(blank_label(node_id)?));
        }
        self.fresh_bnode()
    }

    fn insert_statement(
        &mut self,
        subject: XmlTerm,
        predicate: String,
        object: XmlTerm,
        reifier: Option<XmlNode>,
        annotation: Option<XmlNode>,
    ) -> Result<(), RdfDiagnostic> {
        self.rows.push(XmlRow {
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
        });
        // `rdf:ID` on a property element is RDF 1.0 reification (the classic
        // rdf:Statement/subject/predicate/object quads); `rdf:annotation` /
        // `rdf:annotationNodeID` is the RDF 1.2 reifier (rdf:reifies a triple term).
        if let Some(reifier) = reifier {
            self.insert_classic_reification(
                reifier,
                subject.clone(),
                predicate.clone(),
                object.clone(),
            )?;
        }
        if let Some(annotation) = annotation {
            self.insert_reifier(annotation, subject, predicate, object);
        }
        Ok(())
    }

    /// Emit the RDF 1.0 reification quads for a property element carrying `rdf:ID`.
    fn insert_classic_reification(
        &mut self,
        reifier: XmlNode,
        subject: XmlTerm,
        predicate: String,
        object: XmlTerm,
    ) -> Result<(), RdfDiagnostic> {
        let reifier: XmlTerm = reifier.into();
        self.rows.push(XmlRow {
            subject: reifier.clone(),
            predicate: rdf_iri(RDF_TYPE)?,
            object: XmlTerm::Iri(rdf_iri(RDF_STATEMENT)?),
        });
        self.rows.push(XmlRow {
            subject: reifier.clone(),
            predicate: rdf_iri(RDF_SUBJECT)?,
            object: subject,
        });
        self.rows.push(XmlRow {
            subject: reifier.clone(),
            predicate: rdf_iri(RDF_PREDICATE)?,
            object: XmlTerm::Iri(predicate),
        });
        self.rows.push(XmlRow {
            subject: reifier,
            predicate: rdf_iri(RDF_OBJECT)?,
            object,
        });
        Ok(())
    }

    fn insert_reifier(
        &mut self,
        reifier: XmlNode,
        subject: XmlTerm,
        predicate: String,
        object: XmlTerm,
    ) {
        let quoted = XmlTerm::Triple(Box::new((subject, XmlTerm::Iri(predicate), object)));
        self.rows.push(XmlRow {
            subject: reifier.into(),
            predicate: RDF_REIFIES_IRI.to_owned(),
            object: quoted,
        });
    }

    fn context_literal(
        &self,
        lexical: &str,
        datatype: Option<&str>,
        context: &ParseContext,
    ) -> Result<RdfLiteral, RdfDiagnostic> {
        if let Some(datatype) = datatype {
            return Ok(RdfLiteral::typed(lexical, self.iri_ref(datatype, context)?));
        }
        if let Some(language) = &context.language {
            validate_language_tag(language)?;
            // A directional language-tagged literal carries the RDF 1.2 base direction;
            // the IR expands the datatype to rdf:langString on intern (C0.1).
            let direction = context.direction.map(|d| match d {
                BaseDirection::Ltr => RdfTextDirection::Ltr,
                BaseDirection::Rtl => RdfTextDirection::Rtl,
            });
            return Ok(RdfLiteral {
                lexical_form: lexical.to_owned(),
                datatype: None,
                language: Some(language.clone()),
                direction,
            });
        }
        Ok(RdfLiteral::simple(lexical))
    }

    /// Resolve an `rdf:about` / `rdf:resource` reference against the base in force.
    ///
    /// RDF/XML admits relative references, so this is `BaseScope::resolve`. There is no
    /// "no base, keep the raw text" fallthrough: that is what let an unresolved relative
    /// IRI reach the frozen IR.
    fn iri_ref(&self, value: &str, context: &ParseContext) -> Result<String, RdfDiagnostic> {
        context
            .base
            .resolve(value)
            .map(|iri| iri.as_str().to_owned())
            .map_err(|e| RdfDiagnostic::error(e.diagnostic_code(), e.to_string()))
    }

    fn rdf_id_iri(&self, value: &str, context: &ParseContext) -> Result<String, RdfDiagnostic> {
        if value.is_empty() {
            return Err(parse_err("empty rdf:ID"));
        }
        // `rdf:ID="x"` denotes the same-document reference `#x`, which RFC-3986 5.2
        // resolves against the base (dropping the base's own fragment). With no base in
        // scope there is nothing to resolve against, so this hard-fails rather than
        // interning the bare `#x` as if it were an IRI.
        context
            .base
            .resolve(&format!("#{value}"))
            .map(|iri| iri.as_str().to_owned())
            .map_err(|e| RdfDiagnostic::error(e.diagnostic_code(), e.to_string()))
    }

    fn fresh_bnode(&mut self) -> Result<XmlNode, RdfDiagnostic> {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        let label = deterministic_blank_label_with_prefix("rdfxml_", id);
        validate_blank_label(&label)?;
        Ok(XmlNode::Blank(label))
    }

    fn fresh_collection_bnode(&mut self) -> Result<XmlNode, RdfDiagnostic> {
        let id = self.collection_counter;
        self.collection_counter += 1;
        let label = deterministic_blank_label_with_prefix("rdfxml_list_", id);
        validate_blank_label(&label)?;
        Ok(XmlNode::Blank(label))
    }
}

/// Intern an [`XmlTerm`] subject position into the builder, returning its [`TermId`].
/// A triple term resolves its `(s, p, o)` components and interns as a triple.
fn intern_term(builder: &mut RdfDatasetBuilder, term: &XmlTerm) -> Result<TermId, RdfDiagnostic> {
    match intern_node(builder, term)? {
        FoldNode::Term(id) => Ok(id),
        FoldNode::Triple { s, p, o } => Ok(builder.intern_triple(s, p, o)),
    }
}

/// Intern an [`XmlTerm`] object into the builder, returning a [`FoldNode`]: a leaf
/// becomes `Term`, a triple term becomes `Triple` (its components already interned) so
/// the shared fold can bind it as a reifier when it is the object of an `rdf:reifies`
/// row, or re-intern it as a quoted-triple object otherwise.
fn intern_node(builder: &mut RdfDatasetBuilder, term: &XmlTerm) -> Result<FoldNode, RdfDiagnostic> {
    match term {
        XmlTerm::Iri(iri) => Ok(FoldNode::Term(builder.intern_iri(iri))),
        // Text ingress: decode the `(label, scope)` encoding this codec's serializer
        // applied at egress, so a document it wrote re-parses to the very
        // `(label, scope)` pair it was written from. `rdf:nodeID` carries an XML
        // `NCName`, which is the alphabet the image test re-encodes against.
        XmlTerm::Blank(label) => Ok(FoldNode::Term(
            builder.intern_text_blank(label, LabelAlphabet::NcName),
        )),
        // A composite literal's embedded blank labels bind through the SAME rule
        // the `rdf:nodeID` tokens above use, so both spellings of one node agree.
        XmlTerm::Literal(literal) => Ok(FoldNode::Term(builder.intern_literal_bound(
            literal.clone(),
            BlankBinding::Decoded(LabelAlphabet::NcName),
        )?)),
        XmlTerm::Triple(components) => {
            let (subject, predicate, object) = components.as_ref();
            let s = intern_term(builder, subject)?;
            let p = intern_term(builder, predicate)?;
            let o = intern_term(builder, object)?;
            Ok(FoldNode::Triple { s, p, o })
        }
    }
}

// ── roxmltree element/attribute helpers (RDF/XML name matching) ─────────────────

fn is_rdf(element: Node<'_, '_>, local: &str) -> bool {
    element.tag_name().namespace() == Some(RDF_NS) && element.tag_name().name() == local
}

/// The IRI of a node element / property element: `namespace + local`.
fn element_iri(
    element: Node<'_, '_>,
    base: &purrdf_iri::BaseScope,
) -> Result<String, RdfDiagnostic> {
    name_iri(
        element.tag_name().namespace(),
        element.tag_name().name(),
        base,
    )
}

fn name_iri(
    namespace: Option<&str>,
    local: &str,
    base: &purrdf_iri::BaseScope,
) -> Result<String, RdfDiagnostic> {
    let iri = format!("{}{local}", namespace.unwrap_or_default());
    validate_iri(&iri, base)?;
    Ok(iri)
}

fn attr_rdf<'a>(element: Node<'a, '_>, local: &str) -> Option<&'a str> {
    attr_in_ns(element, RDF_NS, local)
}

fn attr_xml<'a>(element: Node<'a, '_>, local: &str) -> Option<&'a str> {
    attr_in_ns(element, XML_NS, local)
}

fn attr_its<'a>(element: Node<'a, '_>, local: &str) -> Option<&'a str> {
    attr_in_ns(element, ITS_NS, local)
}

fn attr_in_ns<'a>(element: Node<'a, '_>, namespace: &str, local: &str) -> Option<&'a str> {
    element
        .attributes()
        .find(|attr| attr.namespace() == Some(namespace) && attr.name() == local)
        .map(|attr| attr.value())
}

/// Property attributes: every attribute that is NOT an `xml:`/`its:` attribute or one
/// of the reserved `rdf:` mapping attributes — exactly the purrdf-gts `property_attrs`
/// filter.
fn property_attrs<'a, 'input>(
    element: Node<'a, 'input>,
) -> impl Iterator<Item = roxmltree::Attribute<'a, 'input>> {
    element
        .attributes()
        .filter(|attr| attr.namespace() != Some(XML_NS))
        .filter(|attr| attr.namespace() != Some(ITS_NS))
        .filter(|attr| {
            !(attr.namespace() == Some(RDF_NS)
                && matches!(
                    attr.name(),
                    RDF_ABOUT
                        | RDF_ID
                        | RDF_NODE_ID
                        | RDF_RESOURCE
                        | RDF_DATATYPE
                        | RDF_PARSE_TYPE
                        | RDF_TYPE
                        | RDF_VERSION
                        | RDF_ANNOTATION
                        | RDF_ANNOTATION_NODE_ID
                ))
        })
}

/// Element children of `node`, in document order (skipping text / comment nodes).
fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(Node::is_element)
}

/// Concatenate the direct text-node children of `element` (the literal text content).
fn element_text(element: Node<'_, '_>) -> String {
    element
        .children()
        .filter(Node::is_text)
        .filter_map(|n| n.text())
        .collect()
}

fn rdf_iri(local: &str) -> Result<String, RdfDiagnostic> {
    let iri = format!("{RDF_NS}{local}");
    // The one IRI position with no document text in it at all: both halves are crate
    // constants, so no caller base could bear on it and the empty scope is the literal
    // truth rather than a stand-in for a scope that was thrown away. The check stays
    // because a constant can be mistyped; it cannot fire on user input.
    validate_iri(&iri, &purrdf_iri::BaseScope::empty())?;
    Ok(iri)
}

/// Validate + normalize a blank-node label off `rdf:nodeID` / `rdf:annotationNodeID`.
fn blank_label(label: &str) -> Result<String, RdfDiagnostic> {
    validate_blank_label(label)?;
    Ok(label.to_owned())
}

/// Validate an IRI built from an XML qualified NAME (`namespace + local`), which is the
/// one IRI position in this codec that is not a reference and so never resolves.
///
/// A `rdf:about` / `rdf:resource` / `rdf:ID` value goes through the base scope instead
/// (see [`iri_ref`](RdfXmlParser::iri_ref)); this is only for element and attribute
/// names, whose namespace comes from an `xmlns:` declaration.
///
/// The check is the shared RFC-3987 grammar plus the absoluteness requirement, NOT the
/// bare "contains a `:`" test this used to apply. That test admitted a relative
/// reference whose first segment merely contained a colon — a `urn`-shaped namespace
/// like `xmlns:ex="urn"` composed to `urnLocal`, which has no colon and was rejected for
/// the right reason by accident, while `xmlns:ex="a:b"` composed to `a:bLocal` and was
/// accepted as "absolute" when it is a `path-noscheme` relative reference.
///
/// The scope handed in is the CALLER'S, not a locally minted empty one. It is still never
/// applied — an `xmlns:` namespace is not a reference and resolves against nothing — but
/// the refusal can now name the base in scope and say it does not apply here. The empty
/// stand-in could only say "no base IRI is in scope", which was false for anyone who had
/// passed `--base` and sent them hunting for a dropped parameter rather than at the
/// relative `xmlns:` declaration that actually caused it.
fn validate_iri(value: &str, base: &purrdf_iri::BaseScope) -> Result<(), RdfDiagnostic> {
    base.resolve_absolute_only(value)
        .map(|_| ())
        .map_err(|error| {
            RdfDiagnostic::error(
                error.diagnostic_code(),
                format!("invalid IRI from an XML qualified name: {error}"),
            )
        })
}

/// The blank-node label contract for an `rdf:nodeID` / `rdf:annotationNodeID`
/// attribute value.
///
/// Ingress is deliberately the LIBERAL union of the two alphabets a producer can
/// legitimately have written: the XML `NCName` this codec itself EMITS (the
/// RDF/XML grammar's own constraint on `rdf:nodeID`), and the Turtle/SPARQL
/// `BLANK_NODE_LABEL`, which admits the digit-led labels the text codecs carry.
/// Accepting the union means every document this workspace writes re-parses,
/// while a genuinely malformed identifier (whitespace, delimiters, control
/// characters) is still a hard parse failure.
fn validate_blank_label(label: &str) -> Result<(), RdfDiagnostic> {
    if !is_valid_label(label, LabelAlphabet::NcName)
        && !is_valid_label(label, LabelAlphabet::BlankNodeLabel)
    {
        return Err(parse_err(format!(
            "invalid blank-node identifier {label:?}"
        )));
    }
    Ok(())
}

/// The language-tag contract the prior purrdf-gts `validate_language_tag` enforced:
/// non-empty, hyphen-separated subtags that are each non-empty and ASCII alphanumeric.
fn validate_language_tag(language: &str) -> Result<(), RdfDiagnostic> {
    let valid = !language.is_empty()
        && language.split('-').all(|subtag| {
            !subtag.is_empty() && subtag.chars().all(|ch| ch.is_ascii_alphanumeric())
        });
    if !valid {
        return Err(parse_err(format!("invalid language tag {language:?}")));
    }
    Ok(())
}

// ── XML-literal (`rdf:parseType="Literal"`) inclusive canonicalization ──────────

/// Serialize an element's children as the canonical XML-literal lexical form, the
/// `rdf:parseType="Literal"` object value. The literal's apex elements carry the
/// in-scope namespace declarations (inclusive canonicalization); descendants inherit
/// them and add none — matching the prior purrdf-gts XML-literal canonicalization.
fn serialize_children_as_xml(element: Node<'_, '_>) -> String {
    // In-scope namespace declarations on the literal apex, in declaration order
    // (excluding the implicit `xml` prefix, which is never rendered).
    let apex_ns: Vec<(String, String)> = element
        .namespaces()
        .filter(|ns| ns.name() != Some("xml"))
        .map(|ns| {
            (
                ns.name().unwrap_or_default().to_string(),
                ns.uri().to_string(),
            )
        })
        .collect();
    let mut out = String::new();
    for child in element.children() {
        if child.is_element() || child.is_text() {
            serialize_xml_node(child, Some(&apex_ns), &mut out);
        }
    }
    out
}

/// One step of the XML-literal walk: an element/text node still to be written, or the end
/// tag owed to an element whose start tag is already out.
enum XmlLiteralStep<'a, 'input> {
    /// Write this node. The flag is set only for the literal's apex, which is the one
    /// element that renders the in-scope namespace declarations.
    Open(Node<'a, 'input>, bool),
    /// Close the element whose raw (prefixed) name this is.
    Close(String),
}

/// Append one node of an XML literal to `out` in canonical form.
///
/// # An explicit stack, because the content is arbitrary
///
/// The value of an `rdf:parseType="Literal"` property is OPAQUE: it is whatever XML the
/// author put there, and it lowers to a single literal STRING — no part of it reaches the IR
/// as structure, so nothing downstream bounds its depth. Walking it recursively made the
/// document's nesting the process's stack depth, which aborted on deep input. The work list
/// below makes the depth a heap cost instead, so THIS WALK adds no bound of its own.
///
/// It is not the only thing between the input and here, and the distinction matters to
/// anyone reading this for the codec's real limit. [`guard_xml_nesting`] measures element
/// nesting on the SOURCE TEXT, in front of the XML tokenizer that would otherwise overflow
/// on its own, and text is all it has — it cannot tell an element that is literal content
/// from one that is structure. So a deep literal IS refused, by that guard rather than by
/// this function.
///
/// Pre-order with an owed end tag reproduces the recursive walk's bytes exactly: children are
/// pushed in reverse so they pop in document order, and the [`XmlLiteralStep::Close`] pushed
/// before them pops after the whole subtree.
fn serialize_xml_node(node: Node<'_, '_>, apex_ns: Option<&[(String, String)]>, out: &mut String) {
    let mut stack = vec![XmlLiteralStep::Open(node, true)];
    while let Some(step) = stack.pop() {
        let (node, is_apex) = match step {
            XmlLiteralStep::Close(raw) => {
                out.push_str("</");
                out.push_str(&raw);
                out.push('>');
                continue;
            }
            XmlLiteralStep::Open(node, is_apex) => (node, is_apex),
        };
        if node.is_text() {
            if let Some(text) = node.text() {
                out.push_str(&escape_xml_text(text));
            }
            continue;
        }
        if !node.is_element() {
            continue;
        }
        let raw = raw_name(node);
        out.push('<');
        out.push_str(&raw);
        if let Some(namespaces) = apex_ns.filter(|_| is_apex) {
            for (prefix, iri) in namespaces {
                if prefix.is_empty() {
                    let _ = write!(out, " xmlns=\"{}\"", escape_xml_attr(iri));
                } else {
                    let _ = write!(out, " xmlns:{prefix}=\"{}\"", escape_xml_attr(iri));
                }
            }
        }
        for attr in node.attributes() {
            out.push(' ');
            out.push_str(&raw_attr_name(node, attr));
            out.push_str("=\"");
            out.push_str(&escape_xml_attr(attr.value()));
            out.push('"');
        }
        // Canonical XML has no self-closing form: always emit a start/end pair.
        out.push('>');
        stack.push(XmlLiteralStep::Close(raw));
        stack.extend(
            node.children()
                .rev()
                .filter(|child| child.is_element() || child.is_text())
                .map(|child| XmlLiteralStep::Open(child, false)),
        );
    }
}

/// The raw (prefixed) element name as it would be written: `prefix:local` when the
/// element's namespace has a bound prefix, else its local name.
fn raw_name(node: Node<'_, '_>) -> String {
    let name = node.tag_name();
    qualify(node, name.namespace(), name.name())
}

/// The raw (prefixed) attribute name. An unprefixed attribute carries no namespace.
fn raw_attr_name(node: Node<'_, '_>, attr: roxmltree::Attribute<'_, '_>) -> String {
    match attr.namespace() {
        Some(ns) => qualify(node, Some(ns), attr.name()),
        None => attr.name().to_string(),
    }
}

fn qualify(node: Node<'_, '_>, namespace: Option<&str>, local: &str) -> String {
    match namespace {
        Some(ns) => match node.lookup_prefix(ns) {
            Some(prefix) if !prefix.is_empty() => format!("{prefix}:{local}"),
            _ => local.to_string(),
        },
        None => local.to_string(),
    }
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value).replace('"', "&quot;")
}

// ───────────────────────────────────────────────────────────────────────────────
// Serialize: SerGraph → RDF/XML text
// ───────────────────────────────────────────────────────────────────────────────

/// One property to render under a subject: a base `(predicate, object)` pair, or an RDF
/// 1.2 reifier binding `<rid> rdf:reifies <<( s p o )>>` rendered with a
/// `parseType="Triple"` object.
enum PropertyItem {
    /// A base quad property: `(predicate-id, object-id)`.
    Pair(usize, usize),
    /// An `rdf:reifies` binding to the quoted triple `(s, p, o)` term-ids.
    Reifies(usize, usize, usize),
}

/// Serialize a default-graph [`SerGraph`] to RDF/XML text, grouping by subject.
///
/// Lowers the graph exactly as the prior `to_rdf_quads` path did: the base quads, then
/// each NON-internal reifier binding as a `<rid> rdf:reifies <<( s p o )>>` triple (a
/// `parseType="Triple"` object), then each annotation as a plain triple. When the caller
/// drops the statement layer (star-incapable RDF/XML egress), `graph.reifiers` carries
/// only self-reifier sentinels (skipped) and `graph.annotations` is empty, so only the
/// base quads render. Named graphs are rejected (RDF/XML is a single-graph syntax).
///
/// # The base
///
/// RDF/XML's row in the format registry sets `emits_base`, so a base reaching this graph
/// is declared as `xml:base` on the `rdf:RDF` root and every `rdf:about` / `rdf:resource`
/// reference is spelled against it. Those are precisely the two attributes the parser
/// resolves through the base scope, so the document reads back to the same terms. Element
/// and attribute NAMES are built from namespace IRIs, which are not references and never
/// resolve against a base, so they stay absolute — as does the `rdf:nodeID` label, which
/// is not an IRI at all.
///
/// Subject GROUPING keys ([`subject_key`]) stay on the absolute IRI, so the emitted
/// element order is the same whether or not a base is in force.
pub(super) fn serialize_ser_graph_to_rdfxml(graph: &SerGraph) -> Result<String, RdfDiagnostic> {
    let named = graph.quads.iter().any(|(_, _, _, g)| g.is_some())
        || graph.reifiers.iter().any(|(_, _, g)| g.is_some())
        || graph.annotations.iter().any(|(_, _, _, g)| g.is_some());
    if named {
        return Err(serialize_err("cannot serialize a named graph"));
    }

    let mut subjects: BTreeMap<String, Vec<PropertyItem>> = BTreeMap::new();
    let mut subject_terms: BTreeMap<String, usize> = BTreeMap::new();
    for &(s, p, o, _) in &graph.quads {
        let key = subject_key(graph, s)?;
        subject_terms.entry(key.clone()).or_insert(s);
        subjects
            .entry(key)
            .or_default()
            .push(PropertyItem::Pair(p, o));
    }
    for &(rid, (s, p, o), _) in &graph.reifiers {
        // A triple TERM keys its own components under its own id (a self-reference, the
        // inline quoted-triple binding) — its components render inline wherever it
        // appears, so it carries no `rdf:reifies` row, exactly as `to_rdf_quads` skips it.
        if graph
            .terms
            .get(rid)
            .is_some_and(|t| t.kind == SerTermKind::Triple && t.reifier == Some(rid))
        {
            continue;
        }
        let key = subject_key(graph, rid)?;
        subject_terms.entry(key.clone()).or_insert(rid);
        subjects
            .entry(key)
            .or_default()
            .push(PropertyItem::Reifies(s, p, o));
    }
    for &(r, p, v, _) in &graph.annotations {
        let key = subject_key(graph, r)?;
        subject_terms.entry(key.clone()).or_insert(r);
        subjects
            .entry(key)
            .or_default()
            .push(PropertyItem::Pair(p, v));
    }

    let namespaces = serializer_namespaces(graph, &subjects)?;
    let mut out = String::from(
        "<?xml version=\"1.0\"?>\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"",
    );
    for (namespace, prefix) in &namespaces {
        if prefix != "rdf" && prefix != "xsd" {
            let _ = write!(out, " xmlns:{prefix}=\"{}\"", escape_xml_attr(namespace));
        }
    }
    // The document base, when one is in force: `xml:base` on the root scopes it to the
    // whole document, which is what every relativized `rdf:about` / `rdf:resource` below
    // resolves against on re-read.
    if let Some(base) = graph.base() {
        let _ = write!(out, " xml:base=\"{}\"", escape_xml_attr(base.as_str()));
    }
    // Declare RDF 1.2 so a round-trip preserves triple terms and base direction (their
    // parse is gated on `rdf:version="1.2"`).
    out.push_str(" rdf:version=\"1.2\">\n");

    for (key, properties) in subjects {
        let subject = *subject_terms
            .get(&key)
            .expect("subject term exists for every grouped subject");
        out.push_str("  <rdf:Description");
        write_node_attribute(&mut out, graph, subject)?;
        out.push_str(">\n");
        for property in properties {
            match property {
                PropertyItem::Pair(predicate, object) => {
                    write_property(&mut out, "    ", graph, predicate, object, &namespaces)?;
                }
                PropertyItem::Reifies(s, p, o) => {
                    write_reifies(&mut out, "    ", graph, (s, p, o), &namespaces)?;
                }
            }
        }
        out.push_str("  </rdf:Description>\n");
    }

    out.push_str("</rdf:RDF>\n");
    Ok(out)
}

/// Render an `rdf:reifies` binding to the quoted triple `(s, p, o)` as a
/// `parseType="Triple"` property, matching the prior path's
/// `<rid> rdf:reifies <<( s p o )>>` rendering.
fn write_reifies(
    out: &mut String,
    indent: &str,
    graph: &SerGraph,
    (s, p, o): (usize, usize, usize),
    namespaces: &BTreeMap<String, String>,
) -> Result<(), RdfDiagnostic> {
    let name = serializer_qname(RDF_REIFIES_IRI, namespaces);
    let _ = writeln!(out, "{indent}<{name} rdf:parseType=\"Triple\">");
    write_triple_node(out, &format!("{indent}  "), graph, (s, p, o), namespaces)?;
    let _ = writeln!(out, "{indent}</{name}>");
    Ok(())
}

/// The grouping key for a subject term: `I<iri>` for an IRI, `B<label>` for a blank node.
fn subject_key(graph: &SerGraph, tid: usize) -> Result<String, RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => Ok(format!("I{}", ser_value(term)?)),
        SerTermKind::Bnode => Ok(format!("B{}", ser_value(term)?)),
        other => Err(serialize_err(format!(
            "a subject must be an IRI or blank node, got {other:?}"
        ))),
    }
}

/// Write the `rdf:about` / `rdf:nodeID` attribute for a subject node term.
///
/// `rdf:about` is an IRI REFERENCE, so it is spelled against the document base declared
/// as `xml:base` on the root; `rdf:nodeID` is a blank-node label and is not.
fn write_node_attribute(
    out: &mut String,
    graph: &SerGraph,
    tid: usize,
) -> Result<(), RdfDiagnostic> {
    let term = ser_term(graph, tid)?;
    match term.kind {
        SerTermKind::Iri => {
            let _ = write!(
                out,
                " rdf:about=\"{}\"",
                escape_xml_attr(&iri_reference(graph, ser_value(term)?))
            );
        }
        SerTermKind::Bnode => {
            let _ = write!(out, " rdf:nodeID=\"{}\"", escape_xml_attr(ser_value(term)?));
        }
        other => {
            return Err(serialize_err(format!(
                "a subject must be an IRI or blank node, got {other:?}"
            )));
        }
    }
    Ok(())
}

fn serializer_namespaces(
    graph: &SerGraph,
    subjects: &BTreeMap<String, Vec<PropertyItem>>,
) -> Result<BTreeMap<String, String>, RdfDiagnostic> {
    let mut namespaces = BTreeMap::from([
        (RDF_NS.to_string(), "rdf".to_string()),
        (XSD_NS.to_string(), "xsd".to_string()),
    ]);
    let mut next = 0usize;
    // Mirror the prior `to_rdf_quads`-fed serializer: a namespace is registered ONLY for
    // each top-level quad's predicate (for an `rdf:reifies` binding that predicate is
    // `rdf:reifies`, already in the RDF namespace). Inner triple-term predicates are not
    // pre-registered — they qualify lazily in `write_property`.
    for properties in subjects.values() {
        for property in properties {
            let predicate_iri = match property {
                PropertyItem::Pair(predicate, _) => ser_value(ser_term(graph, *predicate)?)?,
                PropertyItem::Reifies(_, _, _) => RDF_REIFIES_IRI,
            };
            let namespace = split_property_iri(predicate_iri).0;
            if namespaces.contains_key(namespace) {
                continue;
            }
            namespaces.insert(namespace.to_string(), format!("ns{next}"));
            next += 1;
        }
    }
    Ok(namespaces)
}

/// Write one `<predicate>object</predicate>` property, resolving a quoted-triple object
/// through `rdf:parseType="Triple"`.
///
/// # Termination
///
/// The `write_property` → `write_triple_node` cycle carries no depth bound and no
/// visited set. It terminates because the term table it walks does: every producer of a
/// [`SerGraph`] guarantees that, and the one that takes a caller-supplied graph
/// (`crate::gts::gts_to_ser`) proves it, refusing a self-reaching table with
/// `gts-self-reaching-term`. See `ser_model::write_term`.
fn write_property(
    out: &mut String,
    indent: &str,
    graph: &SerGraph,
    predicate: usize,
    object: usize,
    namespaces: &BTreeMap<String, String>,
) -> Result<(), RdfDiagnostic> {
    let name = serializer_qname(ser_value(ser_term(graph, predicate)?)?, namespaces);
    let term = ser_term(graph, object)?;
    match term.kind {
        SerTermKind::Iri => {
            // `rdf:resource` is an IRI reference and resolves against `xml:base`, so it
            // is spelled against the document base exactly as `rdf:about` is.
            let _ = writeln!(
                out,
                "{indent}<{name} rdf:resource=\"{}\"/>",
                escape_xml_attr(&iri_reference(graph, ser_value(term)?))
            );
        }
        SerTermKind::Bnode => {
            let _ = writeln!(
                out,
                "{indent}<{name} rdf:nodeID=\"{}\"/>",
                escape_xml_attr(ser_value(term)?)
            );
        }
        SerTermKind::Literal => {
            let _ = write!(out, "{indent}<{name}");
            if let Some(language) = &term.lang {
                let _ = write!(out, " xml:lang=\"{}\"", escape_xml_attr(language));
            }
            if let Some(direction) = &term.direction {
                let _ = write!(out, " xmlns:its=\"{ITS_NS}\" its:dir=\"{direction}\"");
            }
            if let Some(datatype) = term.datatype {
                let _ = write!(
                    out,
                    " rdf:datatype=\"{}\"",
                    escape_xml_attr(ser_value(ser_term(graph, datatype)?)?)
                );
            }
            let _ = writeln!(out, ">{}</{name}>", escape_xml_text(ser_value(term)?));
        }
        SerTermKind::Triple => {
            let (s, p, o) = term
                .reifier
                .and_then(|rf| graph.reifier(rf))
                .ok_or_else(|| serialize_err("a triple term has no reifier binding"))?;
            let _ = writeln!(out, "{indent}<{name} rdf:parseType=\"Triple\">");
            write_triple_node(out, &format!("{indent}  "), graph, (s, p, o), namespaces)?;
            let _ = writeln!(out, "{indent}</{name}>");
        }
    }
    Ok(())
}

fn write_triple_node(
    out: &mut String,
    indent: &str,
    graph: &SerGraph,
    (s, p, o): (usize, usize, usize),
    namespaces: &BTreeMap<String, String>,
) -> Result<(), RdfDiagnostic> {
    let _ = write!(out, "{indent}<rdf:Description");
    write_node_attribute(out, graph, s)?;
    out.push_str(">\n");
    write_property(out, &format!("{indent}  "), graph, p, o, namespaces)?;
    let _ = writeln!(out, "{indent}</rdf:Description>");
    Ok(())
}

/// Spell an IRI that will be written into an attribute the RDF/XML grammar treats as a
/// REFERENCE (`rdf:about`, `rdf:resource`), relative to the document base when one is in
/// force.
///
/// Delegates to [`ser_model::spell_iri`], which is the one relativization seam in this
/// crate; this codec performs no reference arithmetic of its own.
fn iri_reference<'a>(graph: &SerGraph, iri: &'a str) -> std::borrow::Cow<'a, str> {
    super::ser_model::spell_iri(iri, graph.base())
}

/// Borrow a [`SerTerm`] by id, hard-failing on an out-of-range id.
fn ser_term(graph: &SerGraph, tid: usize) -> Result<&SerTerm, RdfDiagnostic> {
    graph.terms.get(tid).ok_or_else(|| {
        serialize_err(format!(
            "term id {tid} is out of range for the serialization graph"
        ))
    })
}

/// Borrow a term's string value (IRI / literal lexical / blank label), hard-failing when
/// absent.
fn ser_value(term: &SerTerm) -> Result<&str, RdfDiagnostic> {
    term.value
        .as_deref()
        .ok_or_else(|| serialize_err("term is missing its value"))
}

fn serializer_qname(iri: &str, namespaces: &BTreeMap<String, String>) -> String {
    let (namespace, local) = split_property_iri(iri);
    let prefix = namespaces.get(namespace).map_or("ns", String::as_str);
    format!("{prefix}:{local}")
}

fn split_property_iri(iri: &str) -> (&str, &str) {
    let split = iri.rfind(['#', '/', ':']).map_or(0, |index| index + 1);
    let (namespace, local) = iri.split_at(split);
    if local.is_empty() || !is_xml_name(local) {
        (iri, "property")
    } else {
        (namespace, local)
    }
}

fn is_xml_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_xml_name_start(first) {
        return false;
    }
    chars.all(is_xml_name_char)
}

fn is_xml_name_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_xml_name_char(ch: char) -> bool {
    is_xml_name_start(ch) || ch.is_numeric() || matches!(ch, '-' | '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The in-scope base a caller-supplied base string produces, matching what the
    /// public `parse_dataset` entry point builds.
    fn scope(base: Option<&str>) -> purrdf_iri::BaseScope {
        super::super::parse::base_scope_for(base).expect("fixture base is absolute")
    }

    /// Parse RDF/XML straight into a frozen dataset, for assertions over quads.
    fn parse(text: &str, base: Option<&str>) -> Arc<RdfDataset> {
        parse_rdfxml_to_dataset(text, &mut scope(base)).expect("parse rdf/xml")
    }

    /// A DEEP DOCUMENT IS A DIAGNOSTIC, NOT A DEAD PROCESS.
    ///
    /// Twenty thousand nested `rdf:Description` elements aborted the `purrdf` binary with
    /// `SIGABRT`, and the overflow was inside `roxmltree`'s own `parse_content` ⇄
    /// `parse_element` recursion — before any first-party code held a tree, and past what
    /// `catch_unwind` can see. The guard therefore sits in front of `Document::parse`, and
    /// this is the end-to-end proof that it does.
    ///
    /// An `rdf:parseType="Literal"` whose CONTENT is deeply nested is the second shape: its
    /// canonicalization walked the subtree recursively, so it overflowed on XML the RDF
    /// grammar never descends into. That walk is iterative now, and the same door guards the
    /// tokenizer underneath it.
    #[test]
    fn a_deeply_nested_document_is_refused_rather_than_overflowing_the_stack() {
        const DEPTH: usize = 20_000;
        const HEAD: &str = "<?xml version=\"1.0\"?>\n<rdf:RDF \
            xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
            xmlns:ex=\"http://example.org/\">";

        let striped = format!(
            "{HEAD}<rdf:Description rdf:about=\"http://example.org/s\">{}{}</rdf:Description></rdf:RDF>",
            "<ex:p><rdf:Description>".repeat(DEPTH),
            "</rdf:Description></ex:p>".repeat(DEPTH),
        );
        let xml_literal = format!(
            "{HEAD}<rdf:Description rdf:about=\"http://example.org/s\">\
             <ex:p rdf:parseType=\"Literal\">{}x{}</ex:p></rdf:Description></rdf:RDF>",
            "<a>".repeat(DEPTH),
            "</a>".repeat(DEPTH),
        );

        for (name, text) in [("node striping", &striped), ("XML literal", &xml_literal)] {
            let error = parse_rdfxml_to_dataset(text, &mut scope(None))
                .err()
                .unwrap_or_else(|| panic!("{name}: a 20 000-deep document must be refused"));
            assert!(
                error.message.contains("element nesting reaches"),
                "{name}: the refusal must name the nesting, got: {}",
                error.message
            );
        }
    }

    /// …AND ORDINARY RDF/XML IS UNTOUCHED. Node/property striping spends TWO elements per
    /// RDF nesting level, so a document nested far past anything an author writes still has
    /// to parse — a bound that cost real documents would be worse than the crash.
    #[test]
    fn ordinary_element_nesting_is_untouched_by_the_bound() {
        const DEPTH: usize = 30;
        let text = format!(
            "<?xml version=\"1.0\"?>\n<rdf:RDF \
             xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
             xmlns:ex=\"http://example.org/\">\
             <rdf:Description rdf:about=\"http://example.org/s\">{}\
             <ex:leaf>x</ex:leaf>{}</rdf:Description></rdf:RDF>",
            "<ex:p><rdf:Description>".repeat(DEPTH),
            "</rdf:Description></ex:p>".repeat(DEPTH),
        );
        let dataset = parse(&text, None);
        assert_eq!(
            dataset.quads().count(),
            DEPTH + 1,
            "one triple per nesting level, plus the leaf"
        );
    }

    /// Serialize a frozen dataset to RDF/XML with the RDF 1.2 statement layer PROJECTED
    /// away (declared loss for RDF/XML under the transcode contract), matching the
    /// production arm `serialize_dataset_to_format` takes for this format.
    fn serialize(dataset: &RdfDataset) -> String {
        crate::native_codecs::serialize_dataset_with(
            dataset,
            crate::NativeRdfFormat::RdfXml,
            None,
            &crate::native_codecs::SerializeOptions {
                selection: crate::SerializeGraph::Dataset,
                statement_layer: crate::native_codecs::StatementLayer::Project,
                jsonld_options: None,
            },
        )
        .map(|outcome| String::from_utf8(outcome.bytes).expect("utf8"))
        .expect("serialize rdf/xml")
    }

    #[test]
    fn description_with_property_round_trips() {
        let text = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <eg:p rdf:resource="http://example.org/o"/>
  </rdf:Description>
</rdf:RDF>"#;
        let ds = parse(text, None);
        assert_eq!(ds.quad_count(), 1);
        // Serialize → re-parse must be isomorphic.
        let xml = serialize(&ds);
        let reparsed = parse(&xml, None);
        assert!(
            crate::datasets_isomorphic(&ds, &reparsed),
            "rdf/xml round-trip must be isomorphic"
        );
    }

    #[test]
    fn typed_node_emits_rdf_type() {
        let text = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <eg:Thing rdf:about="http://example.org/s"/>
</rdf:RDF>"#;
        let ds = parse(text, None);
        assert_eq!(ds.quad_count(), 1, "typed node element emits rdf:type quad");
    }

    #[test]
    fn literal_with_lang_and_datatype() {
        let text = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <eg:label xml:lang="en">hello</eg:label>
    <eg:count rdf:datatype="http://www.w3.org/2001/XMLSchema#integer">42</eg:count>
  </rdf:Description>
</rdf:RDF>"#;
        let ds = parse(text, None);
        assert_eq!(ds.quad_count(), 2);
    }

    #[test]
    fn collection_expands_to_list() {
        let text = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <eg:items rdf:parseType="Collection">
      <rdf:Description rdf:about="http://example.org/a"/>
      <rdf:Description rdf:about="http://example.org/b"/>
    </eg:items>
  </rdf:Description>
</rdf:RDF>"#;
        let ds = parse(text, None);
        // head quad + 2*(first,rest) = 1 + 4.
        assert_eq!(ds.quad_count(), 5, "two-item collection expands to a list");
    }

    #[test]
    fn rdf_id_resolves_against_base() {
        let text = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:ID="x">
    <eg:p rdf:resource="http://example.org/o"/>
  </rdf:Description>
</rdf:RDF>"#;
        let ds = parse(text, Some("http://base.example/doc"));
        assert!(
            ds.term_id_by_value(&crate::TermValue::Iri(
                "http://base.example/doc#x".to_owned()
            ))
            .is_some(),
            "rdf:ID resolves to base#x"
        );
    }
}

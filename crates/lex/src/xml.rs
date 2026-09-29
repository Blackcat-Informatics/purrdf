// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one XML reader: XML 1.0 (Fifth Edition) with Namespaces in
//! XML 1.0, as a pull [`Reader`] and a read-only [`Document`] tree built from
//! it.
//!
//! # Contract
//!
//! * **Well-formedness** — every XML 1.0 well-formedness constraint a
//!   non-validating processor checks, and every Namespaces in XML 1.0
//!   constraint: names are `Name`s and `QName`s, every scalar is a `Char`
//!   (U+0000-U+0008, U+000B, U+000C, U+000E-U+001F, U+FFFE and U+FFFF are
//!   refused wherever they appear, literally or as a character reference),
//!   tags balance, an attribute appears once by name and once by expanded
//!   name, `]]>` never appears in character data, comments hold no `--`, the
//!   XML declaration comes first or not at all, there is exactly one root
//!   element and nothing but markup and whitespace around it, prefixes are
//!   declared before use, and the `xml`/`xmlns` prefixes and namespace names
//!   are used only as §3 of Namespaces allows.
//! * **Normalization** — line ends are normalized (§2.11), attribute values
//!   are normalized (§3.3.3), and character and entity references are
//!   expanded, so a caller sees the document's information set rather than
//!   its spelling. Text is delivered in document order; the [`Document`]
//!   merges adjacent character data, CDATA sections and references into one
//!   text node.
//! * **Namespaces** — every element and attribute arrives resolved to its
//!   namespace name, with its prefix as written kept beside it; the tree
//!   answers in-scope bindings in both directions.
//! * **No fetching, no bombs** — a document type declaration is refused
//!   ([`XmlErrorKind::Doctype`]) unless the caller asks for its internal
//!   subset ([`Dtd::InternalSubset`]), and even then an external subset, an
//!   external or unparsed entity and every parameter entity are refused
//!   ([`XmlErrorKind::ExternalEntity`], [`XmlErrorKind::ParameterEntity`]),
//!   and the total replacement text expanded is bounded.
//! * **No machine-stack recursion, and an explicit depth cap** — the reader
//!   keeps its open elements and its entity expansions on heap stacks, and
//!   refuses an element deeper than [`Options::max_depth`]
//!   ([`XmlErrorKind::DepthLimit`]). The tree is built from the event stream
//!   in the same single pass, so no input shape can exhaust the call stack.
//! * **Located errors** — every [`XmlError`] carries the byte offset where the
//!   document went wrong, with [`XmlError::line_column`] to render it.
//!
//! Text is borrowed from the source wherever the source spells it exactly
//! (no reference, no line end to normalize), so reading a document copies
//! only what normalization changes.
//!
//! # Why one reader
//!
//! RDF/XML, TriX, the SPARQL results XML format, RIF-XML, GraphML, DataCite
//! and SVG all read XML, and a reader decides what a document *means*: which
//! scalars are text, what a prefix resolves to, whether an entity expands.
//! Two readers disagree on exactly the documents that matter — a namespace
//! declared on an ancestor, a CR LF inside an attribute, a character
//! reference with a sign — so each of those decisions is made once, here, in
//! the lexical layer every one of those readers reaches.
//!
//! # Examples
//!
//! ```
//! use purrdf_lex::xml::Document;
//!
//! let text = r#"<r xmlns="http://example.org/ns" xmlns:x="http://example.org/x">
//!   <item x:id="1">A &amp; B<![CDATA[ <raw> ]]>&#x41;</item>
//! </r>"#;
//! let document = Document::parse(text).unwrap();
//! let root = document.root_element();
//! assert_eq!(root.tag_name().namespace(), Some("http://example.org/ns"));
//! let item = root.first_element_child().unwrap();
//! assert_eq!(item.attribute(("http://example.org/x", "id")), Some("1"));
//! assert_eq!(item.text(), Some("A & B <raw> A"));
//! ```

mod dom;
mod error;
mod reader;

pub use dom::{Document, ExpandedName, NameQuery, Namespace, Node, NodeId, NodeType};
pub use error::{XmlError, XmlErrorKind};
pub use reader::{
    Attribute, DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTITY_EXPANSION, Declaration, Dtd, EndTag, Event,
    NamespaceDecl, Options, QName, Reader, StartTag, XML_NAMESPACE, XMLNS_NAMESPACE,
};

#[cfg(test)]
mod tests;

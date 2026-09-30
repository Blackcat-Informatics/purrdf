// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The read-only document tree, built from the pull reader's events in one
//! pass with no recursion.

use std::borrow::Cow;
use std::ops::Range;

use super::error::XmlError;
use super::reader::{Attribute, Event, NamespaceDecl, Options, QName, Reader, XML_NAMESPACE};

/// The index of a node in its [`Document`], in document order: the root is
/// `0`, and a node's descendants are exactly the ids after it up to the end of
/// its subtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
    /// The index.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// What a node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    /// The document node, parent of the root element and of the prolog's and
    /// epilog's comments and processing instructions.
    Root,
    /// An element.
    Element,
    /// Character data: adjacent text, CDATA sections and references merged.
    Text,
    /// A comment.
    Comment,
    /// A processing instruction.
    Pi,
}

#[derive(Debug)]
enum Kind<'a> {
    Root,
    Element {
        name: QName<'a>,
        namespace: Option<Cow<'a, str>>,
        attributes: Range<usize>,
        namespaces: Range<usize>,
    },
    Text(Cow<'a, str>),
    Comment(Cow<'a, str>),
    Pi {
        target: Cow<'a, str>,
        data: Option<Cow<'a, str>>,
    },
}

#[derive(Debug)]
struct NodeData<'a> {
    kind: Kind<'a>,
    parent: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
    first_child: Option<usize>,
    last_child: Option<usize>,
    /// One past the last node of this node's subtree.
    end: usize,
    offset: usize,
}

/// A parsed document: namespace-resolved elements, attributes, text,
/// comments and processing instructions, in document order.
#[derive(Debug)]
pub struct Document<'a> {
    text: &'a str,
    nodes: Vec<NodeData<'a>>,
    attributes: Vec<Attribute<'a>>,
    namespaces: Vec<NamespaceDecl<'a>>,
}

/// An expanded name, `(namespace, local)`: the identity XML Namespaces gives
/// an element or attribute name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpandedName<'n> {
    namespace: Option<&'n str>,
    name: &'n str,
}

impl<'n> ExpandedName<'n> {
    /// The namespace name, if any.
    #[must_use]
    pub const fn namespace(&self) -> Option<&'n str> {
        self.namespace
    }

    /// The local name.
    #[must_use]
    pub const fn name(&self) -> &'n str {
        self.name
    }
}

/// A name to match: a bare local name, or a `(namespace, local)` pair.
///
/// A bare `&str` matches an element by local name alone, and an attribute
/// only when it has no namespace; a pair matches the expanded name exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameQuery<'q> {
    /// A local name.
    Local(&'q str),
    /// A namespace name and a local name.
    Expanded(&'q str, &'q str),
}

impl<'q> From<&'q str> for NameQuery<'q> {
    fn from(local: &'q str) -> Self {
        Self::Local(local)
    }
}

impl<'q> From<(&'q str, &'q str)> for NameQuery<'q> {
    fn from((namespace, local): (&'q str, &'q str)) -> Self {
        Self::Expanded(namespace, local)
    }
}

/// An in-scope namespace binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Namespace<'d> {
    name: Option<&'d str>,
    uri: &'d str,
}

impl<'d> Namespace<'d> {
    /// The prefix, or `None` for the default namespace.
    #[must_use]
    pub const fn name(&self) -> Option<&'d str> {
        self.name
    }

    /// The namespace name; empty for a default namespace undeclared with
    /// `xmlns=""`.
    #[must_use]
    pub const fn uri(&self) -> &'d str {
        self.uri
    }
}

impl<'a> Document<'a> {
    /// Parse `text` with [`Options::default`].
    ///
    /// # Errors
    ///
    /// [`XmlError`] at the first violation.
    pub fn parse(text: &'a str) -> Result<Self, XmlError> {
        Self::parse_with_options(text, Options::default())
    }

    /// Parse `text` with `options`.
    ///
    /// # Errors
    ///
    /// [`XmlError`] at the first violation.
    pub fn parse_with_options(text: &'a str, options: Options) -> Result<Self, XmlError> {
        let mut document = Self {
            text,
            nodes: vec![NodeData {
                kind: Kind::Root,
                parent: None,
                previous: None,
                next: None,
                first_child: None,
                last_child: None,
                end: 1,
                offset: 0,
            }],
            attributes: Vec::new(),
            namespaces: Vec::new(),
        };
        let mut reader = Reader::with_options(text, options);
        let mut parent = 0;
        loop {
            match reader.next()? {
                Event::Declaration(_) => {}
                Event::Start(tag) => {
                    let attributes_start = document.attributes.len();
                    document.attributes.extend_from_slice(tag.attributes);
                    let namespaces_start = document.namespaces.len();
                    document
                        .namespaces
                        .extend_from_slice(tag.namespace_declarations);
                    let kind = Kind::Element {
                        name: tag.name.clone(),
                        namespace: tag.namespace.cloned(),
                        attributes: attributes_start..document.attributes.len(),
                        namespaces: namespaces_start..document.namespaces.len(),
                    };
                    parent = document.push(parent, kind, tag.offset);
                }
                Event::End(_) => {
                    document.nodes[parent].end = document.nodes.len();
                    parent = document.nodes[parent]
                        .parent
                        .expect("an element has a parent");
                }
                Event::Text { text, offset } | Event::CData { text, offset } => {
                    document.push_text(parent, text, offset);
                }
                Event::Comment { text, offset } => {
                    document.push(parent, Kind::Comment(text), offset);
                }
                Event::Pi {
                    target,
                    data,
                    offset,
                } => {
                    document.push(parent, Kind::Pi { target, data }, offset);
                }
                Event::Eof => break,
            }
        }
        document.nodes[0].end = document.nodes.len();
        Ok(document)
    }

    /// Append a node as `parent`'s last child, returning its id.
    fn push(&mut self, parent: usize, kind: Kind<'a>, offset: usize) -> usize {
        let id = self.nodes.len();
        let previous = self.nodes[parent].last_child;
        self.nodes.push(NodeData {
            kind,
            parent: Some(parent),
            previous,
            next: None,
            first_child: None,
            last_child: None,
            end: id + 1,
            offset,
        });
        if let Some(previous) = previous {
            self.nodes[previous].next = Some(id);
        } else {
            self.nodes[parent].first_child = Some(id);
        }
        self.nodes[parent].last_child = Some(id);
        id
    }

    /// Append character data, merged into `parent`'s last child when that is
    /// already text.
    fn push_text(&mut self, parent: usize, text: Cow<'a, str>, offset: usize) {
        if let Some(last) = self.nodes[parent].last_child
            && let Kind::Text(existing) = &mut self.nodes[last].kind
        {
            existing.to_mut().push_str(&text);
            return;
        }
        self.push(parent, Kind::Text(text), offset);
    }

    /// The source text.
    #[must_use]
    pub const fn input_text(&self) -> &'a str {
        self.text
    }

    /// The document node.
    #[must_use]
    pub const fn root(&self) -> Node<'_, 'a> {
        Node {
            document: self,
            id: 0,
        }
    }

    /// The root element.
    ///
    /// # Panics
    ///
    /// Never: a parsed document always has exactly one root element.
    #[must_use]
    pub fn root_element(&self) -> Node<'_, 'a> {
        self.root()
            .children()
            .find(Node::is_element)
            .expect("a parsed document has a root element")
    }

    /// Every node in document order, the document node first.
    pub fn descendants(&self) -> impl DoubleEndedIterator<Item = Node<'_, 'a>> + '_ {
        self.root().descendants()
    }

    /// The node with id `id`, if the document has one.
    #[must_use]
    pub fn get_node(&self, id: NodeId) -> Option<Node<'_, 'a>> {
        (id.0 < self.nodes.len()).then_some(Node {
            document: self,
            id: id.0,
        })
    }

    /// The number of nodes, the document node included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Always `false`: a document holds at least the document node and its
    /// root element.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// A node of a [`Document`]: a cheap copyable handle.
#[derive(Clone, Copy)]
pub struct Node<'d, 'a> {
    document: &'d Document<'a>,
    id: usize,
}

impl PartialEq for Node<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.document, other.document) && self.id == other.id
    }
}

impl Eq for Node<'_, '_> {}

impl core::fmt::Debug for Node<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.data().kind {
            Kind::Root => write!(f, "Root"),
            Kind::Element { name, .. } => write!(f, "Element(<{}>)", name.as_str()),
            Kind::Text(text) => write!(f, "Text({text:?})"),
            Kind::Comment(text) => write!(f, "Comment({text:?})"),
            Kind::Pi { target, .. } => write!(f, "Pi({target})"),
        }
    }
}

impl<'d, 'a> Node<'d, 'a> {
    fn data(&self) -> &'d NodeData<'a> {
        &self.document.nodes[self.id]
    }

    const fn at(&self, id: usize) -> Self {
        Self {
            document: self.document,
            id,
        }
    }

    fn link(&self, id: Option<usize>) -> Option<Self> {
        id.map(|id| self.at(id))
    }

    /// This node's id.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        NodeId(self.id)
    }

    /// The document this node belongs to.
    #[must_use]
    pub const fn document(&self) -> &'d Document<'a> {
        self.document
    }

    /// What this node is.
    #[must_use]
    pub fn node_type(&self) -> NodeType {
        match self.data().kind {
            Kind::Root => NodeType::Root,
            Kind::Element { .. } => NodeType::Element,
            Kind::Text(_) => NodeType::Text,
            Kind::Comment(_) => NodeType::Comment,
            Kind::Pi { .. } => NodeType::Pi,
        }
    }

    /// Whether this is the document node.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.node_type() == NodeType::Root
    }

    /// Whether this is an element.
    #[must_use]
    pub fn is_element(&self) -> bool {
        self.node_type() == NodeType::Element
    }

    /// Whether this is character data.
    #[must_use]
    pub fn is_text(&self) -> bool {
        self.node_type() == NodeType::Text
    }

    /// Whether this is a comment.
    #[must_use]
    pub fn is_comment(&self) -> bool {
        self.node_type() == NodeType::Comment
    }

    /// Whether this is a processing instruction.
    #[must_use]
    pub fn is_pi(&self) -> bool {
        self.node_type() == NodeType::Pi
    }

    /// The byte offset of the node in the source: an element's `<`, a text
    /// run's first byte, or the reference that brought it in from an entity.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.data().offset
    }

    /// An element's expanded name; empty for any other node.
    #[must_use]
    pub fn tag_name(&self) -> ExpandedName<'d> {
        match &self.data().kind {
            Kind::Element {
                name, namespace, ..
            } => ExpandedName {
                namespace: namespace.as_deref(),
                name: name.local(),
            },
            _ => ExpandedName {
                namespace: None,
                name: "",
            },
        }
    }

    /// Whether this is an element named `name`.
    #[must_use]
    pub fn has_tag_name<'q>(&self, name: impl Into<NameQuery<'q>>) -> bool {
        if !self.is_element() {
            return false;
        }
        let tag = self.tag_name();
        match name.into() {
            NameQuery::Local(local) => tag.name == local,
            NameQuery::Expanded(namespace, local) => {
                tag.namespace == Some(namespace) && tag.name == local
            }
        }
    }

    /// An element's name as written, `prefix:local` or `local`.
    #[must_use]
    pub fn qname(&self) -> Option<&'d str> {
        match &self.data().kind {
            Kind::Element { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    /// An element's prefix as written, if it has one.
    #[must_use]
    pub fn prefix(&self) -> Option<&'d str> {
        match &self.data().kind {
            Kind::Element { name, .. } => name.prefix(),
            _ => None,
        }
    }

    /// An element's attributes, in document order and then any `ATTLIST`
    /// defaults; namespace declarations are not attributes. Empty for any
    /// other node.
    #[must_use]
    pub fn attributes(&self) -> &'d [Attribute<'a>] {
        &self.document.attributes[self.element_ranges().0]
    }

    /// The value of the attribute `name` names (see [`NameQuery`]).
    #[must_use]
    pub fn attribute<'q>(&self, name: impl Into<NameQuery<'q>>) -> Option<&'d str> {
        let query = name.into();
        self.attributes()
            .iter()
            .find(|attribute| match query {
                NameQuery::Local(local) => {
                    attribute.namespace().is_none() && attribute.name() == local
                }
                NameQuery::Expanded(namespace, local) => {
                    attribute.namespace() == Some(namespace) && attribute.name() == local
                }
            })
            .map(Attribute::value)
    }

    /// Whether the element has the attribute `name` names.
    #[must_use]
    pub fn has_attribute<'q>(&self, name: impl Into<NameQuery<'q>>) -> bool {
        self.attribute(name).is_some()
    }

    /// The namespace declarations this element's start tag makes.
    #[must_use]
    pub fn namespace_declarations(&self) -> &'d [NamespaceDecl<'a>] {
        &self.document.namespaces[self.element_ranges().1]
    }

    /// The element's attribute and namespace-declaration ranges into the
    /// document's tables, or two empty ranges for any other node: where both
    /// per-element slices are read from.
    fn element_ranges(&self) -> (Range<usize>, Range<usize>) {
        match &self.data().kind {
            Kind::Element {
                attributes,
                namespaces,
                ..
            } => (attributes.clone(), namespaces.clone()),
            _ => (0..0, 0..0),
        }
    }

    /// Every namespace binding in scope at this element, nearest first: its
    /// own declarations in document order, then each ancestor's that no
    /// nearer element redeclares, then the `xml` prefix. A default namespace
    /// undeclared with `xmlns=""` appears with an empty URI.
    #[must_use]
    pub fn namespaces(&self) -> std::vec::IntoIter<Namespace<'d>> {
        let mut out: Vec<Namespace<'d>> = Vec::new();
        let mut element = Some(*self).filter(Node::is_element);
        while let Some(node) = element {
            for decl in node.namespace_declarations() {
                if !out.iter().any(|seen| seen.name == decl.name()) {
                    out.push(Namespace {
                        name: decl.name(),
                        uri: decl.uri(),
                    });
                }
            }
            element = node.parent_element();
        }
        if self.is_element() && !out.iter().any(|seen| seen.name == Some("xml")) {
            out.push(Namespace {
                name: Some("xml"),
                uri: XML_NAMESPACE,
            });
        }
        out.into_iter()
    }

    /// The namespace name `prefix` is bound to here (`None` asks for the
    /// default namespace), or `None` when it is unbound or undeclared.
    #[must_use]
    pub fn lookup_namespace_uri(&self, prefix: Option<&str>) -> Option<&'d str> {
        let mut element = Some(*self).filter(Node::is_element);
        while let Some(node) = element {
            if let Some(decl) = node
                .namespace_declarations()
                .iter()
                .rev()
                .find(|decl| decl.name() == prefix)
            {
                return (!decl.uri().is_empty()).then_some(decl.uri());
            }
            element = node.parent_element();
        }
        (prefix == Some("xml")).then_some(XML_NAMESPACE)
    }

    /// The nearest prefix bound to `uri` here, `""` for the default
    /// namespace, or `None` when no in-scope binding names it. A binding
    /// shadowed by a nearer declaration of the same prefix does not count.
    #[must_use]
    pub fn lookup_prefix(&self, uri: &str) -> Option<&'d str> {
        if uri == XML_NAMESPACE {
            return Some("xml");
        }
        self.namespaces()
            .find(|namespace| namespace.uri == uri && !uri.is_empty())
            .map(|namespace| namespace.name.unwrap_or(""))
    }

    /// A text node's text, a comment's text, or an element's first child's
    /// text when that child is character data.
    #[must_use]
    pub fn text(&self) -> Option<&'d str> {
        match &self.data().kind {
            Kind::Text(text) | Kind::Comment(text) => Some(text),
            Kind::Element { .. } => self
                .first_child()
                .and_then(|child| match &child.data().kind {
                    Kind::Text(text) => Some(&**text),
                    _ => None,
                }),
            _ => None,
        }
    }

    /// A processing instruction's target and data.
    #[must_use]
    pub fn pi(&self) -> Option<(&'d str, Option<&'d str>)> {
        match &self.data().kind {
            Kind::Pi { target, data } => Some((target, data.as_deref())),
            _ => None,
        }
    }

    /// The parent node.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.link(self.data().parent)
    }

    /// The nearest ancestor that is an element.
    #[must_use]
    pub fn parent_element(&self) -> Option<Self> {
        self.parent().filter(Node::is_element)
    }

    /// This node and its ancestors, nearest first.
    pub fn ancestors(&self) -> impl Iterator<Item = Self> + use<'d, 'a> {
        core::iter::successors(Some(*self), Node::parent)
    }

    /// The first child.
    #[must_use]
    pub fn first_child(&self) -> Option<Self> {
        self.link(self.data().first_child)
    }

    /// The last child.
    #[must_use]
    pub fn last_child(&self) -> Option<Self> {
        self.link(self.data().last_child)
    }

    /// The next sibling.
    #[must_use]
    pub fn next_sibling(&self) -> Option<Self> {
        self.link(self.data().next)
    }

    /// The previous sibling.
    #[must_use]
    pub fn prev_sibling(&self) -> Option<Self> {
        self.link(self.data().previous)
    }

    /// The first child that is an element.
    #[must_use]
    pub fn first_element_child(&self) -> Option<Self> {
        self.children().find(Node::is_element)
    }

    /// The next sibling that is an element.
    #[must_use]
    pub fn next_sibling_element(&self) -> Option<Self> {
        core::iter::successors(self.next_sibling(), Node::next_sibling).find(Node::is_element)
    }

    /// Whether the node has children.
    #[must_use]
    pub fn has_children(&self) -> bool {
        self.data().first_child.is_some()
    }

    /// The children, in document order.
    pub fn children(&self) -> impl DoubleEndedIterator<Item = Self> + use<'d, 'a> {
        Children {
            front: self.first_child(),
            back: self.last_child(),
        }
    }

    /// This node and every node below it, in document order.
    pub fn descendants(&self) -> impl DoubleEndedIterator<Item = Self> + use<'d, 'a> {
        let node = *self;
        (self.id..self.data().end).map(move |id| node.at(id))
    }
}

/// The children of a node, walked from both ends.
struct Children<'d, 'a> {
    front: Option<Node<'d, 'a>>,
    back: Option<Node<'d, 'a>>,
}

impl<'d, 'a> Iterator for Children<'d, 'a> {
    type Item = Node<'d, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.front?;
        if self.back == Some(node) {
            self.front = None;
            self.back = None;
        } else {
            self.front = node.next_sibling();
        }
        Some(node)
    }
}

impl DoubleEndedIterator for Children<'_, '_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.back?;
        if self.front == Some(node) {
            self.front = None;
            self.back = None;
        } else {
            self.back = node.prev_sibling();
        }
        Some(node)
    }
}

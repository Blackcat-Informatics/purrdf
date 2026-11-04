// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source occurrences produced by the shared YAML grammar before schema resolution.

use super::admission::Allocations;
use super::build::{Builder, PropertySpans, Refusal};
use super::read::{self, Error, ErrorKind, Limits, Tag};
use crate::allocation::OrderedTable;
use core::cmp::Ordering;
use purrdf_hash::hex::Digest32;

/// Bounds for an eager Core projection through the shared grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectionLimits {
    /// Existing semantic node, depth, alias and scalar-key controls.
    pub core: Limits,
    /// Maximum original UTF-8 input bytes.
    pub max_input_bytes: usize,
    /// Maximum additional live native bytes during projection, including
    /// parser buffers, anchor copies, expanded aliases and the output tree.
    /// The already admitted borrowed syntax document is outside this account.
    pub max_allocation_bytes: usize,
}
impl ProjectionLimits {
    /// Legacy Core controls plus 16 MiB input and 128 MiB projection storage.
    pub const DEFAULT: Self = Self {
        core: Limits::DEFAULT,
        max_input_bytes: 16 << 20,
        max_allocation_bytes: 128 << 20,
    };
}
impl Default for ProjectionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Explicit admission bounds for a borrowed syntax document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxLimits {
    /// Maximum UTF-8 input bytes, checked before parsing or hashing.
    pub max_input_bytes: usize,
    /// Maximum native admitted bytes, including transient parser buffers.
    pub max_allocation_bytes: usize,
    /// Maximum stored node occurrences; aliases cost one occurrence.
    pub max_occurrences: u64,
    /// Maximum simultaneously open collections.
    pub max_depth: usize,
    /// Whether anchors and aliases are admitted.
    pub aliases: bool,
}
impl SyntaxLimits {
    /// A 16 MiB input, 128 MiB allocation account, 1,048,576 occurrences,
    /// 128 collections and aliases enabled.
    pub const DEFAULT: Self = Self {
        max_input_bytes: 16 << 20,
        max_allocation_bytes: 128 << 20,
        max_occurrences: 1 << 20,
        max_depth: 128,
        aliases: true,
    };
}
impl Default for SyntaxLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Aggregate lexical stream bounds; depth remains local to each document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxStreamLimits {
    /// Input, occurrence, allocation and alias controls for the complete stream.
    pub syntax: SyntaxLimits,
    /// Maximum retained document roots, including explicit empty documents.
    pub max_documents: usize,
}
/// When mapping-key identity is checked during lexical ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxKeyPolicy {
    /// Preserve the existing decoded-text/raw-tag lexical uniqueness law.
    Lexical,
    /// Retain every entry for a later semantic key check.
    Deferred,
}

/// A UTF-8 byte range in its owning document's original source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Inclusive start byte.
    pub start: usize,
    /// Exclusive end byte.
    pub end: usize,
}
impl Span {
    pub(super) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// An occurrence handle, local to the document which returned it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub(super) usize);
/// Original source coordinates; columns count Unicode characters, not bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    /// UTF-8 byte offset.
    pub offset: usize,
    /// Line from one; CRLF is one break.
    pub line: usize,
    /// Character column from one.
    pub column: usize,
}
impl Position {
    /// Locate a byte boundary in original UTF-8 source, including its EOF.
    pub fn at(source: &str, offset: usize) -> Option<Self> {
        if !source.is_char_boundary(offset) {
            return None;
        }
        let (line, column) = read::position(source, offset);
        Some(Self {
            offset,
            line,
            column,
        })
    }
}

/// The spelling of a scalar before schema resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarStyle {
    /// An omitted scalar.
    Empty,
    /// Unquoted text.
    Plain,
    /// Single quotes and doubled apostrophes.
    SingleQuoted,
    /// Double quotes and YAML escapes.
    DoubleQuoted,
    /// A literal block.
    Literal,
    /// A folded block.
    Folded,
}
/// Collection presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionStyle {
    /// Indentation and block indicators.
    Block,
    /// Bracketed or braced flow collection.
    Flow,
}
/// Block-scalar trailing-break policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chomping {
    /// Keep one final break.
    Clip,
    /// Strip final breaks.
    Strip,
    /// Preserve all final breaks.
    Keep,
}
/// Native block-header details.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    /// Header bytes, excluding its line ending.
    pub span: Span,
    /// Chomping policy.
    pub chomping: Chomping,
    /// Explicit indentation increment, if present.
    pub indent_indicator: Option<usize>,
    /// Detected content indentation.
    pub content_indent: usize,
}
/// One decoded scalar and its unmodified spelling.
#[derive(Debug)]
pub struct SyntaxScalar {
    /// Decoded lexical text; no core-schema conversion occurs.
    pub text: String,
    /// Exact scalar spelling, including block header/body.
    pub raw: Span,
    /// Authored style.
    pub style: ScalarStyle,
    /// Native block-header details.
    pub block: Option<BlockHeader>,
}
/// One mapping entry in authored order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxEntry {
    /// Key occurrence.
    pub key: NodeId,
    /// Value occurrence, including omitted values.
    pub value: NodeId,
}
/// A node's syntactic product, independent of JSON's value space.
#[derive(Debug)]
pub enum NodeKind {
    /// Lexical scalar, including .inf and other non-JSON spellings.
    Scalar(SyntaxScalar),
    /// Sequence items in source order.
    Sequence {
        /// Collection style.
        style: CollectionStyle,
        /// Ordered occurrences.
        items: Vec<NodeId>,
    },
    /// Mapping entries in source order.
    Mapping {
        /// Collection style.
        style: CollectionStyle,
        /// Ordered key/value occurrences.
        entries: Vec<SyntaxEntry>,
    },
    /// An alias occurrence, never a subtree copy.
    Alias {
        /// Alias spelling, including the star.
        raw: Span,
        /// Exact prior anchor target.
        target: NodeId,
    },
}
/// An ordered source occurrence and its properties.
#[derive(Debug)]
pub struct SyntaxNode {
    /// Node extent, including its properties.
    pub span: Span,
    /// Syntactic product.
    pub kind: NodeKind,
    /// Authored anchor property, including ampersand.
    pub anchor: Option<Span>,
    /// Authored tag property, with its original handle/spelling.
    pub tag: Option<Span>,
    /// Explicit lexical tag identity after handle expansion; absent for an
    /// untagged node. This does not apply implicit Core-schema resolution.
    pub explicit_tag: Option<ExplicitTag>,
    /// Explicit-key indicator, when this node is a key.
    pub explicit_key: Option<Span>,
}
/// Explicit tag identity, independent of its authored property spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplicitTag {
    /// The bare `!` property.
    NonSpecific,
    /// A specific tag name with its document-local handle expanded.
    Specific(String),
}
/// One native escape occurrence inside a quoted scalar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxEscape {
    /// Original escape bytes.
    pub span: Span,
    /// Decoded character; None denotes escaped line joining.
    pub decoded: Option<char>,
}
/// Presentation-only source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    /// Spaces or tabs.
    Whitespace,
    /// A source line ending.
    LineBreak,
    /// A whitespace-only line, including its ending.
    BlankLine,
    /// A grammar-recognized comment.
    Comment,
    /// A comment on a block-scalar header.
    BlockHeaderComment,
}
/// A trivia association; document-end material has no adjacent node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attachment {
    /// Before the next occurrence.
    Leading(NodeId),
    /// On the preceding occurrence's line.
    Trailing(NodeId),
    /// Document prologue or epilogue.
    Document,
}
/// One ordered trivia occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivia {
    /// Original bytes.
    pub span: Span,
    /// Trivia form.
    pub kind: TriviaKind,
    /// Deterministic presentation association.
    pub attachment: Attachment,
}

/// A bounded source-preserving YAML document. Node IDs and spans belong here.
///
/// Scalar spelling is never inferred from JSON. Tags and merge candidates are
/// retained and never applied. Scalar-key uniqueness compares decoded text and
/// explicit tag spelling, ignoring quote style; Core projection retains its own
/// existing key law. Complex keys remain ordered syntactic nodes.
#[derive(Debug)]
pub struct SyntaxDocument<'a> {
    root: NodeId,
    arena: SyntaxArena<'a>,
}
impl<'a> SyntaxDocument<'a> {
    /// Original UTF-8, including line endings, comments, BOM and markers.
    pub const fn source(&self) -> &'a str {
        self.arena.source()
    }
    /// BLAKE3 identity of the exact original bytes, including line endings.
    pub const fn source_id(&self) -> Digest32 {
        self.arena.source_id()
    }
    /// The root occurrence, including an omitted empty-document scalar.
    pub const fn root(&self) -> NodeId {
        self.root
    }
    /// Borrow an occurrence belonging to this document.
    pub fn node(&self, id: NodeId) -> Option<&SyntaxNode> {
        self.arena.node(id)
    }
    /// Occurrences in grammar-completion order, with stable local handles.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &SyntaxNode)> {
        self.arena.nodes()
    }
    /// Ordered trivia with native leading/trailing associations.
    pub fn trivia(&self) -> &[Trivia] {
        self.arena.trivia()
    }
    /// Original directive spans in source order.
    pub fn directives(&self) -> &[Span] {
        self.arena.directives()
    }
    /// Decoded quoted escape occurrences in source order.
    pub fn escapes(&self) -> &[SyntaxEscape] {
        self.arena.escapes()
    }
    /// Slice an original UTF-8 range; invalid boundaries are refused.
    pub fn raw(&self, span: Span) -> Option<&'a str> {
        self.arena.raw(span)
    }
    /// Locate a source byte, with CRLF/bare-CR and Unicode character columns.
    pub fn position(&self, offset: usize) -> Option<Position> {
        self.arena.position(offset)
    }
    /// Project through the same eager Core builder used by legacy readers.
    /// # Errors
    /// Returns the positioned eager Core refusal, including semantic alias,
    /// input and live native allocation bounds. No partial tree is returned.
    pub fn to_core(&self, limits: ProjectionLimits) -> Result<crate::json::Value, Error> {
        read::project(self.source(), limits)
    }
    /// Whether a key has the plain merge spelling (or an explicit merge tag).
    pub fn is_merge_key(&self, id: NodeId) -> bool {
        self.arena.is_merge_key(id)
    }
}
impl<'a> core::ops::Deref for SyntaxDocument<'a> {
    type Target = SyntaxArena<'a>;
    fn deref(&self) -> &Self::Target {
        &self.arena
    }
}

/// One document's root and exact absolute source envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxDocumentRoot {
    /// Root occurrence in the stream-wide arena.
    pub root: NodeId,
    /// Complete consumed document range, including its prologue and end marker.
    pub span: Span,
    /// Original bytes preceding the explicit start marker or implicit content.
    pub prologue: Span,
    /// Explicit `---`, excluding subsequent separation.
    pub start_marker: Option<Span>,
    /// Explicit `...`, excluding subsequent separation.
    pub end_marker: Option<Span>,
}

/// Ordered documents sharing one original source and occurrence/trivia arena.
/// Empty streams have no roots. Only acyclic aliases are admitted.
#[derive(Debug)]
pub struct SyntaxStream<'a> {
    pub(super) documents: Vec<SyntaxDocumentRoot>,
    pub(super) arena: SyntaxArena<'a>,
}
impl SyntaxStream<'_> {
    /// Ordered document envelopes; an explicit empty document has a scalar root.
    pub fn documents(&self) -> &[SyntaxDocumentRoot] {
        &self.documents
    }
}
impl<'a> core::ops::Deref for SyntaxStream<'a> {
    type Target = SyntaxArena<'a>;
    fn deref(&self) -> &Self::Target {
        &self.arena
    }
}

/// Shared source, occurrences and trivia for a document or complete stream.
#[derive(Debug)]
pub struct SyntaxArena<'a> {
    source: &'a str,
    identity: Digest32,
    pub(super) nodes: Vec<SyntaxNode>,
    trivia: Vec<Trivia>,
    directives: Vec<Span>,
    escapes: Vec<SyntaxEscape>,
    pub(super) retained_bytes: usize,
}
impl<'a> SyntaxArena<'a> {
    /// Original UTF-8, including line endings, comments, BOM and markers.
    pub const fn source(&self) -> &'a str {
        self.source
    }
    /// BLAKE3 identity of the exact original bytes, including line endings.
    pub const fn source_id(&self) -> Digest32 {
        self.identity
    }
    /// Borrow an occurrence belonging to this document.
    pub fn node(&self, id: NodeId) -> Option<&SyntaxNode> {
        self.nodes.get(id.0)
    }
    /// Occurrences in grammar-completion order, with stable local handles.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &SyntaxNode)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(at, node)| (NodeId(at), node))
    }
    /// Ordered trivia with native leading/trailing associations.
    pub fn trivia(&self) -> &[Trivia] {
        &self.trivia
    }
    /// Original directive spans in source order.
    pub fn directives(&self) -> &[Span] {
        &self.directives
    }
    /// Decoded quoted escape occurrences in source order.
    pub fn escapes(&self) -> &[SyntaxEscape] {
        &self.escapes
    }
    /// Slice an original UTF-8 range; invalid boundaries are refused.
    pub fn raw(&self, span: Span) -> Option<&'a str> {
        self.source.get(span.start..span.end)
    }
    /// Locate a source byte, with CRLF/bare-CR and Unicode character columns.
    pub fn position(&self, offset: usize) -> Option<Position> {
        Position::at(self.source, offset)
    }
    /// Whether a key has the plain merge spelling (or an explicit merge tag).
    pub fn is_merge_key(&self, id: NodeId) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        matches!(&node.kind, NodeKind::Scalar(scalar) if scalar.style == ScalarStyle::Plain && scalar.text == "<<")
            || matches!(&node.explicit_tag, Some(ExplicitTag::Specific(tag)) if tag == "tag:yaml.org,2002:merge")
    }
}

pub(super) struct SyntaxMap {
    entries: Vec<SyntaxEntry>,
    keys: OrderedTable<NodeId, ()>,
}
pub(super) struct SyntaxBuilder<'a> {
    source: &'a str,
    limits: SyntaxLimits,
    allocations: Allocations,
    nodes: Vec<SyntaxNode>,
    properties: OrderedTable<(usize, bool), PropertySpans>,
    tags: OrderedTable<Span, ExplicitTag>,
    blocks: OrderedTable<usize, BlockHeader>,
    comments: OrderedTable<Span, bool>,
    directives: Vec<Span>,
    escapes: Vec<SyntaxEscape>,
    questions: Vec<Span>,
    key_policy: SyntaxKeyPolicy,
}

impl SyntaxBuilder<'_> {
    fn refuse(&self, cause: crate::allocation::StorageError, at: usize) -> Refusal {
        (
            if cause == crate::allocation::StorageError::AdmissionFailed {
                ErrorKind::Allocation {
                    limit: self.limits.max_allocation_bytes,
                }
            } else {
                ErrorKind::Storage(cause)
            },
            at,
        )
    }
    fn node(
        &mut self,
        kind: NodeKind,
        at: usize,
        end: usize,
        collection: bool,
    ) -> Result<NodeId, Refusal> {
        if self.nodes.len() as u64 >= self.limits.max_occurrences {
            return Err((
                ErrorKind::Nodes {
                    limit: self.limits.max_occurrences,
                },
                at,
            ));
        }
        let properties = self
            .properties
            .get(&(at, collection))
            .copied()
            .unwrap_or_default();
        let start = properties
            .anchor
            .into_iter()
            .chain(properties.tag)
            .map(|span| span.start)
            .fold(at, usize::min);
        let id = NodeId(self.nodes.len());
        let explicit_tag = properties.tag.and_then(|span| self.tags.remove(&span));
        self.allocations
            .push(
                &mut self.nodes,
                SyntaxNode {
                    span: Span::new(start, end.max(at)),
                    kind,
                    anchor: properties.anchor,
                    tag: properties.tag,
                    explicit_tag,
                    explicit_key: None,
                },
            )
            .map_err(|cause| self.refuse(cause, at))?;
        Ok(id)
    }
    fn scalar_node(
        &mut self,
        text: String,
        plain: bool,
        at: usize,
        end: usize,
    ) -> Result<NodeId, Refusal> {
        let style = if plain {
            if at == end {
                ScalarStyle::Empty
            } else {
                ScalarStyle::Plain
            }
        } else {
            match self.source.as_bytes().get(at) {
                Some(b'\'') => ScalarStyle::SingleQuoted,
                Some(b'"') => ScalarStyle::DoubleQuoted,
                Some(b'|') => ScalarStyle::Literal,
                Some(b'>') => ScalarStyle::Folded,
                _ => ScalarStyle::Empty,
            }
        };
        self.node(
            NodeKind::Scalar(SyntaxScalar {
                text,
                raw: Span::new(at, end),
                style,
                block: self.blocks.get(&at).copied(),
            }),
            at,
            end,
            false,
        )
    }
    fn key_cmp(source: &str, nodes: &[SyntaxNode], left: NodeId, right: NodeId) -> Ordering {
        let target = |id: NodeId| match nodes[id.0].kind {
            NodeKind::Alias { target, .. } => target,
            _ => id,
        };
        let (left, right) = (target(left), target(right));
        match (&nodes[left.0].kind, &nodes[right.0].kind) {
            (NodeKind::Scalar(a), NodeKind::Scalar(b)) => a.text.cmp(&b.text).then_with(|| {
                nodes[left.0]
                    .tag
                    .map(|s| &source[s.start..s.end])
                    .cmp(&nodes[right.0].tag.map(|s| &source[s.start..s.end]))
            }),
            _ => left.cmp(&right),
        }
    }
}

impl Builder for SyntaxBuilder<'_> {
    const LEXICAL: bool = true;
    type Node = NodeId;
    type Key = NodeId;
    type Mapping = SyntaxMap;
    fn tag(&mut self, span: Span, tag: ExplicitTag) -> Result<(), Refusal> {
        self.allocations
            .memory(|memory| self.tags.insert(span, tag, memory))
            .map_err(|cause| self.refuse(cause, span.start))?;
        Ok(())
    }
    fn decorate(&mut self, id: &NodeId, spans: PropertySpans) -> Result<(), Refusal> {
        let node = &mut self.nodes[id.0];
        node.anchor = spans.anchor;
        node.tag = spans.tag;
        if let Some(tag) = spans.tag.and_then(|span| self.tags.remove(&span)) {
            node.explicit_tag = Some(tag);
        }
        node.span.start = spans
            .anchor
            .into_iter()
            .chain(spans.tag)
            .map(|span| span.start)
            .fold(node.span.start, usize::min);
        Ok(())
    }
    fn properties(
        &mut self,
        at: usize,
        collection: bool,
        spans: PropertySpans,
    ) -> Result<(), Refusal> {
        self.allocations
            .memory(|memory| self.properties.insert((at, collection), spans, memory))
            .map_err(|cause| self.refuse(cause, at))?;
        Ok(())
    }
    fn block(&mut self, at: usize, header: BlockHeader) -> Result<(), Refusal> {
        self.allocations
            .memory(|memory| self.blocks.insert(at, header, memory))
            .map_err(|cause| self.refuse(cause, at))?;
        Ok(())
    }
    fn comment(&mut self, span: Span, header: bool) -> Result<(), Refusal> {
        self.allocations
            .memory(|memory| self.comments.insert(span, header, memory))
            .map_err(|cause| self.refuse(cause, span.start))?;
        Ok(())
    }
    fn directive(&mut self, span: Span) -> Result<(), Refusal> {
        self.allocations
            .push(&mut self.directives, span)
            .map_err(|cause| self.refuse(cause, span.start))
    }
    fn escape(&mut self, escape: SyntaxEscape) -> Result<(), Refusal> {
        self.allocations
            .push(&mut self.escapes, escape)
            .map_err(|cause| self.refuse(cause, escape.span.start))
    }
    fn question(&mut self, span: Span) -> Result<(), Refusal> {
        self.allocations
            .push(&mut self.questions, span)
            .map_err(|cause| self.refuse(cause, span.start))
    }
    fn allocations(&mut self) -> &mut Allocations {
        &mut self.allocations
    }
    fn allocation_limit(&self) -> Option<usize> {
        Some(self.limits.max_allocation_bytes)
    }
    fn discard_text(&mut self, text: String) -> Result<(), Refusal> {
        self.allocations
            .discard_string(text)
            .map_err(|cause| self.refuse(cause, 0))
    }
    fn scalar(
        &mut self,
        text: String,
        plain: bool,
        _: Option<(Tag, usize)>,
        at: usize,
        end: usize,
    ) -> Result<NodeId, Refusal> {
        self.scalar_node(text, plain, at, end)
    }
    fn key_text(
        &mut self,
        text: String,
        plain: bool,
        _: Option<(Tag, usize)>,
        at: usize,
        end: usize,
    ) -> Result<NodeId, Refusal> {
        self.scalar_node(text, plain, at, end)
    }
    fn key_node(
        &mut self,
        key: &NodeId,
        text: String,
        _: bool,
        _: Option<(Tag, usize)>,
        _: usize,
    ) -> Result<NodeId, Refusal> {
        self.discard_text(text)?;
        Ok(*key)
    }
    fn key_value(&mut self, key: NodeId) -> NodeId {
        key
    }
    fn copy_node(&mut self, node: &NodeId, _: usize) -> Result<NodeId, Refusal> {
        Ok(*node)
    }
    fn copy_key(&mut self, key: &NodeId, _: usize) -> Result<NodeId, Refusal> {
        Ok(*key)
    }
    fn discard_node(&mut self, _: NodeId) -> Result<(), Refusal> {
        Ok(())
    }
    fn key_of(&mut self, value: NodeId, _: bool, _: usize) -> Result<Option<NodeId>, Refusal> {
        Ok(Some(value))
    }
    fn null(&mut self, at: usize) -> Result<NodeId, Refusal> {
        self.scalar_node(String::new(), true, at, at)
    }
    fn new_map(&mut self) -> SyntaxMap {
        SyntaxMap {
            entries: Vec::new(),
            keys: OrderedTable::default(),
        }
    }
    fn insert(
        &mut self,
        map: &mut SyntaxMap,
        key: NodeId,
        value: NodeId,
        at: usize,
    ) -> Result<(), Refusal> {
        if self.key_policy == SyntaxKeyPolicy::Deferred {
            return self
                .allocations
                .push(&mut map.entries, SyntaxEntry { key, value })
                .map_err(|cause| self.refuse(cause, at));
        }
        self.allocations
            .reserve_index(&mut map.keys)
            .map_err(|cause| self.refuse(cause, at))?;
        let source = self.source;
        let nodes = &self.nodes;
        self.allocations
            .memory(|memory| {
                map.keys
                    .prepare_insert(key, memory, |a, b| {
                        Ok::<_, crate::allocation::StorageError>(Self::key_cmp(
                            source, nodes, *a, *b,
                        ))
                    })
                    .map(|entry| entry.publish(()).1)
            })
            .map_err(|cause| self.refuse(cause, at))?
            .map_or(Ok(()), |()| Err((ErrorKind::DuplicateKey, at)))?;
        self.allocations
            .push(&mut map.entries, SyntaxEntry { key, value })
            .map_err(|cause| self.refuse(cause, at))
    }
    fn sequence(
        &mut self,
        items: Vec<NodeId>,
        at: usize,
        end: usize,
        flow: bool,
    ) -> Result<NodeId, Refusal> {
        let end = if flow {
            end
        } else {
            items.last().map_or(end, |id| self.nodes[id.0].span.end)
        };
        self.node(
            NodeKind::Sequence {
                style: if flow {
                    CollectionStyle::Flow
                } else {
                    CollectionStyle::Block
                },
                items,
            },
            at,
            end,
            true,
        )
    }
    fn mapping(
        &mut self,
        map: SyntaxMap,
        at: usize,
        end: usize,
        flow: bool,
    ) -> Result<NodeId, Refusal> {
        self.allocations
            .memory(|memory| map.keys.release(memory))
            .map_err(|cause| self.refuse(cause, at))?;
        let start = map
            .entries
            .first()
            .map_or(at, |entry| self.nodes[entry.key.0].span.start.min(at));
        let end = if flow {
            end
        } else {
            map.entries
                .last()
                .map_or(end, |entry| self.nodes[entry.value.0].span.end)
        };
        let node = self.node(
            NodeKind::Mapping {
                style: if flow {
                    CollectionStyle::Flow
                } else {
                    CollectionStyle::Block
                },
                entries: map.entries,
            },
            at,
            end,
            true,
        )?;
        self.nodes[node.0].span.start = self.nodes[node.0].span.start.min(start);
        Ok(node)
    }
    fn collection_tag(&self, _: Option<(Tag, usize)>, _: Tag) -> Result<(), Refusal> {
        Ok(())
    }
    fn size(&self, _: &NodeId) -> u64 {
        1
    }
    fn alias_cost(&self, _: u64) -> u64 {
        1
    }
    fn alias(&mut self, node: &NodeId, _: &str, at: usize, end: usize) -> Result<NodeId, Refusal> {
        self.node(
            NodeKind::Alias {
                raw: Span::new(at, end),
                target: *node,
            },
            at,
            end,
            false,
        )
    }
}

/// Read lexical YAML using the native grammar without Core-schema conversion.
///
/// # Errors
/// Refuses malformed syntax, repeated scalar keys, disabled aliases, and any
/// exhausted input, occurrence, depth or native allocation limit.
pub fn read_syntax(source: &str, limits: SyntaxLimits) -> Result<SyntaxDocument<'_>, Error> {
    let (parser_limits, builder) = prepare(source, limits)?;
    let (root, builder) = read::parse(source, parser_limits, builder)?;
    let arena = builder
        .finish(&[])
        .map_err(|(kind, at)| Error::new(source, kind, at))?;
    Ok(SyntaxDocument { root, arena })
}

/// Read a complete ordered stream into one admitted lexical arena.
///
/// All source coordinates are absolute. Empty/comment-only streams have no
/// roots; explicit empty documents have an empty plain scalar. Anchors and tag
/// directives reset between documents. In-progress aliases are refused: this
/// source model supports an acyclic subset, without applying implicit tags or
/// claiming complete Core representation-key equality or YAML-LD composition.
///
/// # Errors
/// Returns the original positioned syntax, alias or aggregate admission refusal.
/// No partial stream is returned if any later document fails.
pub fn read_syntax_stream(
    source: &str,
    limits: SyntaxStreamLimits,
) -> Result<SyntaxStream<'_>, Error> {
    read_syntax_stream_with(source, limits, SyntaxKeyPolicy::Lexical)
}

/// Read through the same lexical grammar with an explicit mapping-key policy.
/// # Errors
/// Returns the same positioned syntax and admission refusals as `read_syntax_stream`.
pub fn read_syntax_stream_with(
    source: &str,
    limits: SyntaxStreamLimits,
    policy: SyntaxKeyPolicy,
) -> Result<SyntaxStream<'_>, Error> {
    let (parser_limits, mut builder) = prepare(source, limits.syntax)?;
    builder.key_policy = policy;
    let (documents, builder) = read::parse_stream(
        source,
        parser_limits,
        builder,
        limits.max_documents,
        |root, boundary| SyntaxDocumentRoot {
            root,
            span: boundary.span,
            prologue: boundary.prologue,
            start_marker: boundary.start_marker,
            end_marker: boundary.end_marker,
        },
    )?;
    let arena = builder
        .finish(&documents)
        .map_err(|(kind, at)| Error::new(source, kind, at))?;
    Ok(SyntaxStream { documents, arena })
}

fn prepare(source: &str, limits: SyntaxLimits) -> Result<(Limits, SyntaxBuilder<'_>), Error> {
    if source.len() > limits.max_input_bytes {
        return Err(Error::new(
            source,
            ErrorKind::Input {
                limit: limits.max_input_bytes,
            },
            0,
        ));
    }
    let builder = SyntaxBuilder {
        source,
        limits,
        allocations: Allocations::bounded(limits.max_allocation_bytes),
        nodes: Vec::new(),
        properties: OrderedTable::default(),
        tags: OrderedTable::default(),
        blocks: OrderedTable::default(),
        comments: OrderedTable::default(),
        directives: Vec::new(),
        escapes: Vec::new(),
        questions: Vec::new(),
        key_policy: SyntaxKeyPolicy::Lexical,
    };
    let parser_limits = Limits {
        max_depth: limits.max_depth,
        max_nodes: u64::MAX,
        aliases: limits.aliases,
        scalar_keys: true,
        ..Limits::DEFAULT
    };
    Ok((parser_limits, builder))
}

impl<'a> SyntaxBuilder<'a> {
    fn finish(mut self, documents: &[SyntaxDocumentRoot]) -> Result<SyntaxArena<'a>, Refusal> {
        // Only committed leaves delimit trivia gaps. No scalar grammar is
        // repeated here, and a hash inside a committed scalar remains content.
        let properties = core::mem::take(&mut self.properties);
        let blocks = core::mem::take(&mut self.blocks);
        let tags = core::mem::take(&mut self.tags);
        self.allocations
            .memory(|memory| {
                properties.release(memory)?;
                tags.release(memory)?;
                blocks.release(memory)
            })
            .map_err(|cause| self.refuse(cause, 0))?;
        let mut leaves = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut covered = Vec::new();
        let mut keys = Vec::new();
        for (index, node) in self.nodes.iter().enumerate() {
            self.allocations
                .push(&mut starts, (node.span, NodeId(index)))
                .map_err(|cause| self.refuse(cause, node.span.start))?;
            self.allocations
                .push(&mut ends, (node.span, NodeId(index)))
                .map_err(|cause| self.refuse(cause, node.span.start))?;
            let raw = match &node.kind {
                NodeKind::Scalar(s) => Some(s.raw),
                NodeKind::Alias { raw, .. } => Some(*raw),
                _ => None,
            };
            if let Some(span) = raw {
                self.allocations
                    .push(&mut leaves, (span, NodeId(index)))
                    .map_err(|cause| self.refuse(cause, span.start))?;
                self.allocations
                    .push(&mut covered, span)
                    .map_err(|cause| self.refuse(cause, span.start))?;
            }
            for span in node.anchor.into_iter().chain(node.tag) {
                self.allocations
                    .push(&mut covered, span)
                    .map_err(|cause| self.refuse(cause, span.start))?;
            }
        }
        for node in &self.nodes {
            if let NodeKind::Mapping { entries, .. } = &node.kind {
                for entry in entries {
                    self.allocations
                        .push(&mut keys, (self.nodes[entry.key.0].span, entry.key))
                        .map_err(|cause| self.refuse(cause, node.span.start))?;
                }
            }
        }
        keys.sort_unstable_by_key(|(span, id)| (span.start, span.end, *id));
        leaves.sort_unstable_by_key(|(span, id)| (span.start, span.end, *id));
        for question in &self.questions {
            if let Some((_, id)) =
                keys.get(keys.partition_point(|(span, _)| span.start < question.end))
            {
                self.nodes[id.0].explicit_key = Some(*question);
            }
        }
        let mut trivia = Vec::new();
        for (&span, &header) in self.comments.iter() {
            let candidate = leaves.partition_point(|(leaf, _)| leaf.start <= span.start);
            let inside = candidate
                .checked_sub(1)
                .and_then(|index| leaves.get(index))
                .is_some_and(|(leaf, _)| span.end <= leaf.end);
            if header || !inside {
                self.allocations
                    .push(
                        &mut trivia,
                        Trivia {
                            span,
                            kind: if header {
                                TriviaKind::BlockHeaderComment
                            } else {
                                TriviaKind::Comment
                            },
                            attachment: Attachment::Document,
                        },
                    )
                    .map_err(|cause| self.refuse(cause, span.start))?;
                self.allocations
                    .push(&mut covered, span)
                    .map_err(|cause| self.refuse(cause, span.start))?;
            }
        }
        for &span in &self.directives {
            self.allocations
                .push(&mut covered, span)
                .map_err(|cause| self.refuse(cause, span.start))?;
        }
        covered.sort_unstable();
        let bytes = self.source.as_bytes();
        let (mut cursor, mut interval) = (0, 0);
        while cursor < bytes.len() {
            while interval < covered.len() && covered[interval].end <= cursor {
                interval += 1;
            }
            if interval < covered.len() && covered[interval].start <= cursor {
                cursor = covered[interval].end.max(cursor + 1);
                continue;
            }
            let ceiling = covered.get(interval).map_or(bytes.len(), |span| span.start);
            let start = cursor;
            let kind = match bytes[cursor] {
                b' ' | b'\t' => {
                    while cursor < ceiling && matches!(bytes[cursor], b' ' | b'\t') {
                        cursor += 1;
                    }
                    TriviaKind::Whitespace
                }
                b'\r' | b'\n' => {
                    cursor += 1;
                    if bytes[start] == b'\r' && cursor < ceiling && bytes[cursor] == b'\n' {
                        cursor += 1;
                    }
                    let line_start = self.source[..start]
                        .rfind(['\r', '\n'])
                        .map_or(0, |p| p + 1);
                    if bytes[line_start..start]
                        .iter()
                        .all(|b| matches!(b, b' ' | b'\t'))
                    {
                        TriviaKind::BlankLine
                    } else {
                        TriviaKind::LineBreak
                    }
                }
                _ => {
                    cursor += 1;
                    continue;
                }
            };
            self.allocations
                .push(
                    &mut trivia,
                    Trivia {
                        span: Span::new(start, cursor),
                        kind,
                        attachment: Attachment::Document,
                    },
                )
                .map_err(|cause| self.refuse(cause, start))?;
        }
        trivia.sort_unstable_by_key(|entry| entry.span);
        starts.sort_unstable_by_key(|(span, id)| (span.start, span.end, *id));
        ends.sort_unstable_by_key(|(span, id)| (span.end, span.start, *id));
        for entry in &mut trivia {
            let document = documents
                .get(documents.partition_point(|doc| doc.span.end <= entry.span.start))
                .filter(|doc| doc.span.start <= entry.span.start);
            if entry.kind == TriviaKind::BlockHeaderComment {
                let end = leaves.partition_point(|(span, _)| span.start <= entry.span.start);
                if let Some((_, id)) = end
                    .checked_sub(1)
                    .and_then(|index| leaves.get(index))
                    .filter(|(span, _)| document.is_none_or(|doc| span.start >= doc.span.start))
                {
                    entry.attachment = Attachment::Trailing(*id);
                    continue;
                }
            }
            let previous = ends.partition_point(|(span, _)| span.end <= entry.span.start);
            let prior = previous
                .checked_sub(1)
                .and_then(|index| ends.get(index))
                .filter(|(span, _)| document.is_none_or(|doc| span.start >= doc.span.start));
            let next = starts
                .get(starts.partition_point(|(span, _)| span.start < entry.span.end))
                .filter(|(span, _)| document.is_none_or(|doc| span.start < doc.span.end));
            entry.attachment = if entry.kind == TriviaKind::Comment {
                let line_start = self.source[..entry.span.start]
                    .rfind(['\r', '\n'])
                    .map_or(0, |byte| byte + 1);
                prior.filter(|(span, _)| span.end > line_start).map_or_else(
                    || next.map_or(Attachment::Document, |(_, id)| Attachment::Leading(*id)),
                    |(_, id)| Attachment::Trailing(*id),
                )
            } else {
                next.map_or(Attachment::Document, |(_, id)| Attachment::Leading(*id))
            };
        }

        let comments = core::mem::take(&mut self.comments);
        let questions = core::mem::take(&mut self.questions);
        self.allocations
            .memory(|memory| {
                comments.release(memory)?;
                memory.release_vec(questions)?;
                memory.release_vec(covered)?;
                memory.release_vec(leaves)?;
                memory.release_vec(keys)?;
                memory.release_vec(starts)?;
                memory.release_vec(ends)
            })
            .map_err(|cause| self.refuse(cause, 0))?;
        self.escapes.sort_unstable_by_key(|escape| escape.span);
        Ok(SyntaxArena {
            source: self.source,
            identity: Digest32::new(*purrdf_hash::blake3::hash(self.source.as_bytes()).as_bytes()),
            nodes: self.nodes,
            trivia,
            directives: self.directives,
            escapes: self.escapes,
            retained_bytes: self.allocations.live(),
        })
    }
}

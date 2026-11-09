// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The YAML 1.2 reader: one document into the JSON data model, over explicit
//! stacks of open block and flow collections.

use crate::allocation::OrderedTable;
use core::cell::Cell;
use core::fmt;
use core::mem;

use super::build::{Builder, CoreBuilder};
use super::syntax::Span;
use crate::allocation::StorageError;
use crate::json::Value;

/// The resource bounds of one read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most block and flow collections open at once.
    pub max_depth: usize,
    /// The most nodes the document may denote, every node an alias expands to
    /// included, so an alias bomb is refused before it is built.
    pub max_nodes: u64,
    /// Aggregate scalar-classification and numeric-conversion work units.
    /// One input byte, packed radix digit, divided word or output/validation
    /// byte costs one unit. The account is shared by every stream document.
    pub max_numeric_work: u64,
    /// Aggregate bytes of newly materialized numeric spellings. Alias copies
    /// remain governed by the node and allocation bounds.
    pub max_numeric_bytes: usize,
    /// Whether anchors and aliases are accepted. YAML-LD's JSON profile
    /// refuses them.
    pub aliases: bool,
    /// Whether a mapping key written as a number, a boolean or `null` (or left
    /// empty) is accepted, keeping its source text as the key (`1.50: a` is the
    /// key `1.50`, `True: a` the key `True`). JSON object names are strings, so
    /// a reader that feeds JSON refuses them by default; YAML-LD requires the
    /// refusal (`mapping-key-error`, "Mapping Key Types"). A collection key is
    /// refused either way.
    pub scalar_keys: bool,
}

impl Limits {
    /// 128 open collections, 1,048,576 nodes, 67,108,864 numeric work units,
    /// 32 MiB of numeric spellings, aliases accepted, scalar keys refused.
    pub const DEFAULT: Self = Self {
        max_depth: 128,
        max_nodes: 1 << 20,
        max_numeric_work: 1 << 26,
        max_numeric_bytes: 32 << 20,
        aliases: true,
        scalar_keys: false,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Explicit bounds for construction of a complete ordered YAML stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamLimits {
    /// Semantic rules and bounds. `max_nodes` covers the entire stream,
    /// including every expanded alias; collection depth is local to a document.
    pub core: Limits,
    /// Maximum number of documents, including explicitly empty documents.
    pub max_documents: usize,
    /// Maximum original UTF-8 bytes, checked before allocation.
    pub max_input_bytes: usize,
    /// Maximum additional live native bytes, including all retained documents,
    /// the result vector, parser scratch and anchor/alias copies.
    pub max_allocation_bytes: usize,
}

/// The completed construct followed by unexpected source content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrailingContext {
    /// A complete top-level document node.
    Document,
    /// A complete node on the same line.
    Node,
    /// An explicit `...` document end marker.
    EndMarker,
}

/// What is wrong with a YAML document.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The grammar refuses the text here; the text names what was expected.
    Syntax(&'static str),
    /// A tab where YAML requires indentation spaces (§6.1).
    Tab,
    /// A tag other than the core schema's (`!!str`, `!!int`, `!!float`,
    /// `!!bool`, `!!null`, `!!seq`, `!!map`) and the non-specific `!`, or a
    /// core tag its node does not match (`!!int x`).
    Tag,
    /// An anchor or alias while [`Limits::aliases`] is off.
    Alias {
        /// Whether the refused occurrence declares an anchor (`&`), rather than uses an alias (`*`).
        anchor: bool,
        /// Original name bytes, excluding its indicator; no failure-path allocation.
        name: Span,
    },
    /// An alias names no anchor defined before it.
    UnknownAlias {
        /// Original name bytes, excluding `*`.
        name: Span,
    },
    /// An alias names a node whose anchor is still being constructed.
    /// The native reader supports acyclic aliases only.
    CyclicAlias {
        /// Original alias name bytes, excluding `*`.
        name: Span,
    },
    /// A shorthand tag uses a handle not declared in this document.
    UnknownTagHandle,
    /// A semantic projection requires a collection or null document root.
    InvalidRoot,
    /// A mapping key that is not a string: a number, a boolean, `null`, an
    /// empty key (all accepted under [`Limits::scalar_keys`]), or a collection.
    NonStringKey,
    /// A mapping repeats a key (§3.2.1.1).
    DuplicateKey {
        /// Already admitted decoded key text, moved from the refused document.
        key: String,
        /// Original first key occurrence byte offset.
        first: usize,
        /// Original repeated key occurrence byte offset, including an alias indicator.
        repeated: usize,
    },
    /// A lexical collection key is repeated through an alias to the same node.
    DuplicateCollectionKey {
        /// Original first key occurrence.
        first: Span,
        /// Original repeated alias occurrence.
        repeated: Span,
    },
    /// `.inf`, `-.inf` or `.nan`, which JSON cannot hold.
    NonFinite,
    /// Scalar classification or numeric preparation exhausted its work account.
    NumericWork {
        /// Original construction-wide work allowance.
        limit: u64,
    },
    /// Newly materialized numeric spellings exceed their aggregate byte bound.
    NumericOutput {
        /// Original construction-wide spelling-byte allowance.
        limit: usize,
    },
    /// A malformed double-quoted escape, or one that names no character.
    Escape {
        /// Original refused escape bytes, including its backslash.
        span: Span,
    },
    /// EOF before a quoted scalar closes.
    UnfinishedQuote {
        /// Original opening quote.
        quote: char,
        /// Original scalar bytes, from its opening quote through EOF.
        span: Span,
    },
    /// EOF before a flow collection closes.
    UnfinishedFlow {
        /// Original opening delimiter.
        opener: Span,
        /// Required closing delimiter.
        expected: char,
    },
    /// Another token follows a completed document or node.
    TrailingContent {
        /// Original remaining source bytes.
        span: Span,
        /// The completed construct preceding the refused content.
        context: TrailingContext,
    },
    /// A second document in the stream.
    MultipleDocuments,
    /// More documents than [`StreamLimits::max_documents`] allows.
    Documents {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// More collections open at once than [`Limits::max_depth`] allows.
    Depth {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// More nodes than [`Limits::max_nodes`] allows.
    Nodes {
        /// The cap that was exceeded.
        limit: u64,
    },
    /// Input exceeds the explicit syntax admission bound.
    Input {
        /// Maximum input bytes.
        limit: usize,
    },
    /// Native allocation admission exceeds the syntax bound.
    Allocation {
        /// Maximum admitted bytes.
        limit: usize,
    },
    /// A physical native allocation could not complete.
    Storage(StorageError),
}

/// A refused YAML document: what is wrong, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: usize,
    line: usize,
    column: usize,
}

crate::variant_from!(ErrorKind { Storage(StorageError) });

impl purrdf_hash::error::StructuredError for ErrorKind {
    purrdf_hash::error_visit! {
        Syntax(expected) => "yaml-syntax" { "expected" => Text(expected) },
        Tab => "yaml-tab" {},
        Tag => "yaml-tag" {},
        Alias { anchor, name } => "yaml-alias-disabled" { "anchor" => Boolean(*anchor), "name" => Cause(name) },
        UnknownAlias { name } => "yaml-unknown-alias" { "name" => Cause(name) },
        CyclicAlias { name } => "yaml-cyclic-alias" { "name" => Cause(name) },
        UnknownTagHandle => "yaml-unknown-tag-handle" {},
        InvalidRoot => "yaml-invalid-root" {},
        NonStringKey => "yaml-non-string-key" {},
        DuplicateKey { key, first, repeated } => "yaml-duplicate-key" { "key" => Text(key), "first" => ByteCount(*first as u128), "repeated" => ByteCount(*repeated as u128) },
        DuplicateCollectionKey { first, repeated } => "yaml-duplicate-key" { "first" => Cause(first), "repeated" => Cause(repeated) },
        NonFinite => "yaml-non-finite" {},
        NumericWork { limit } => "yaml-numeric-work-limit" { "limit" => Unsigned(u128::from(*limit)) },
        NumericOutput { limit } => "yaml-numeric-output-limit" { "limit" => ByteCount(*limit as u128) },
        Escape { span } => "yaml-escape" { "span" => Cause(span) },
        UnfinishedQuote { quote, span } => "yaml-unfinished-quote" { "quote" => Unsigned(u128::from(u32::from(*quote))), "span" => Cause(span) },
        UnfinishedFlow { opener, expected } => "yaml-unfinished-flow" { "opener" => Cause(opener), "expected" => Unsigned(u128::from(u32::from(*expected))) },
        TrailingContent { span, context } => "yaml-trailing-content" { "span" => Cause(span), "context" => Code(match context {
            TrailingContext::Document => "document",
            TrailingContext::Node => "node",
            TrailingContext::EndMarker => "end-marker",
        }) },
        MultipleDocuments => "yaml-multiple-documents" {},
        Documents { limit } => "yaml-document-limit" { "limit" => Unsigned(*limit as u128) },
        Depth { limit } => "yaml-depth-limit" { "limit" => Unsigned(*limit as u128) },
        Nodes { limit } => "yaml-node-limit" { "limit" => Unsigned(u128::from(*limit)) },
        Input { limit } => "yaml-input-limit" { "limit" => ByteCount(*limit as u128) },
        Allocation { limit } => "yaml-allocation-limit" { "limit" => ByteCount(*limit as u128) },
        Storage(cause) => "yaml-storage" { "cause" => Cause(cause) },
    }
}

impl purrdf_hash::error::StructuredError for Error {
    purrdf_hash::error_visit! {
        @record self { kind, offset, line, column } => "yaml-document" {
            "kind" => Cause(&self.kind),
            "offset" => ByteCount(self.offset as u128),
            "line" => Unsigned(self.line as u128),
            "column" => Unsigned(self.column as u128),
        }
    }
}

impl Error {
    pub(super) fn new(text: &str, kind: ErrorKind, at: usize) -> Self {
        let at = at.min(text.len());
        let (line, column) = position(text, at);
        Self {
            kind,
            offset: at,
            line,
            column,
        }
    }

    pub(super) fn storage(
        text: &str,
        limit: Option<usize>,
        cause: StorageError,
        at: usize,
    ) -> Self {
        let kind = match (cause, limit) {
            (StorageError::AdmissionFailed, Some(limit)) => ErrorKind::Allocation { limit },
            _ => ErrorKind::Storage(cause),
        };
        Self::new(text, kind, at)
    }
    /// What is wrong.
    pub const fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Consume this refusal, retaining its owned decoded context.
    pub fn into_kind(self) -> ErrorKind {
        self.kind
    }

    /// The byte offset of the defect.
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// The defect's line, from 1.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The defect's column in characters, from 1.
    pub const fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "YAML line {} column {}: ", self.line, self.column)?;
        match &self.kind {
            ErrorKind::Syntax(what) => write!(f, "expected {what}"),
            ErrorKind::Tab => f.write_str("a tab cannot indent a block"),
            ErrorKind::Tag => {
                f.write_str("a tag other than a core-schema tag, or one its node does not match")
            }
            ErrorKind::Alias { .. } => f.write_str("anchors and aliases are not accepted here"),
            ErrorKind::UnknownAlias { .. } => f.write_str("the alias names no anchor defined before it"),
            ErrorKind::CyclicAlias { .. } => f.write_str("the alias names an anchor still being constructed; only acyclic aliases are supported"),
            ErrorKind::UnknownTagHandle => f.write_str("the tag handle is not declared in this document"),
            ErrorKind::InvalidRoot => f.write_str("a document root must be a mapping, sequence or null"),
            ErrorKind::NonStringKey => f.write_str("a mapping key must be a string"),
            ErrorKind::DuplicateKey { .. } | ErrorKind::DuplicateCollectionKey { .. } => f.write_str("a mapping repeats a key"),
            ErrorKind::NonFinite => f.write_str("JSON cannot hold an infinite or NaN number"),
            ErrorKind::NumericWork { limit } => write!(f, "numeric work exceeds {limit} units"),
            ErrorKind::NumericOutput { limit } => {
                write!(f, "numeric spellings exceed {limit} bytes")
            }
            ErrorKind::Escape { .. } => f.write_str("a malformed escape"),
            ErrorKind::UnfinishedQuote { .. } => f.write_str("expected the quote that closes the scalar"),
            ErrorKind::UnfinishedFlow { .. } => f.write_str("expected the end of a flow collection"),
            ErrorKind::TrailingContent { context: TrailingContext::Document, .. } => f.write_str("expected the end of the document after its top-level node"),
            ErrorKind::TrailingContent { context: TrailingContext::Node, .. } => f.write_str("expected a line break after a complete node"),
            ErrorKind::TrailingContent { context: TrailingContext::EndMarker, .. } => f.write_str("expected a line ending after `...`"),
            ErrorKind::MultipleDocuments => f.write_str("the stream holds more than one document"),
            ErrorKind::Documents { limit } => write!(f, "more than {limit} documents"),
            ErrorKind::Depth { limit } => write!(f, "more than {limit} nested collections"),
            ErrorKind::Nodes { limit } => write!(f, "more than {limit} nodes"),
            ErrorKind::Input { limit } => write!(f, "input exceeds {limit} bytes"),
            ErrorKind::Allocation { limit } => write!(f, "syntax allocation exceeds {limit} bytes"),
            ErrorKind::Storage(cause) => cause.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Storage(cause) => Some(cause),
            _ => None,
        }
    }
}

/// A block mapping's pending entry.
enum KeyInput<K> {
    Scalar(String, bool),
    Ready(K),
}

enum Slot<K> {
    /// Between entries.
    Idle,
    /// After `?`: the key node is next.
    ExplicitKey,
    /// An explicit key is complete; `:` and its value may follow.
    KeyDone(K, usize),
    /// After `key:`: the value node is next.
    Value(K, usize),
}

/// An open block collection.
enum Frame<B: Builder> {
    /// The document: its one node.
    Root(Option<B::Node>),
    Sequence {
        indent: usize,
        start: usize,
        items: Vec<B::Node>,
        /// After `-`: the item node is next.
        open: bool,
        anchor: Option<Anchor>,
    },
    Mapping {
        indent: usize,
        start: usize,
        object: B::Mapping,
        slot: Slot<B::Key>,
        anchor: Option<Anchor>,
    },
}

/// Where the cursor is on its line, for what may start there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// Past a line break: any node.
    Fresh,
    /// After `-` or `?`: a compact block collection may start.
    Compact,
    /// After a mapping's `:` or the document's `---`: no block collection may
    /// start on this line.
    Inline,
}

/// A declaration token travels with its node until completion.
struct Anchor {
    name: String,
    declaration: usize,
}
struct Binding<N> {
    declaration: usize,
    ready: Option<(N, u64)>,
}

/// A YAML 1.2 core-schema tag (§10.3), or the non-specific `!`: the tags the
/// JSON data model can honour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tag {
    Str,
    Int,
    Float,
    Bool,
    Null,
    Seq,
    Map,
    NonSpecific,
    Other,
}
impl Tag {
    pub(super) fn specific(name: &str) -> Self {
        match name.strip_prefix("tag:yaml.org,2002:") {
            Some("str") => Self::Str,
            Some("int") => Self::Int,
            Some("float") => Self::Float,
            Some("bool") => Self::Bool,
            Some("null") => Self::Null,
            Some("seq") => Self::Seq,
            Some("map") => Self::Map,
            _ => Self::Other,
        }
    }
}

/// A node's properties (§6.9), waiting for their node.
struct Properties {
    anchor: Option<Anchor>,
    tag: Option<(Tag, usize)>,
    /// Where the first property starts.
    at: usize,
    column: usize,
    spans: super::build::PropertySpans,
}

type PropertyParts = (Option<Anchor>, Option<(Tag, usize)>);

/// The anchor and tag of `properties`.
fn split(properties: Option<Properties>) -> PropertyParts {
    properties.map_or((None, None), |properties| {
        (properties.anchor, properties.tag)
    })
}

/// An open flow collection.
enum Flow<B: Builder> {
    Sequence {
        items: Vec<B::Node>,
        anchor: Option<Anchor>,
        start: usize,
        /// Whether an entry is complete and `,` or `]` is next.
        after_item: bool,
    },
    Mapping {
        object: B::Mapping,
        anchor: Option<Anchor>,
        start: usize,
        key: Option<(B::Key, usize)>,
        state: FlowMap,
    },
    /// A single-pair mapping inside a flow sequence (`[a: b]`).
    Pair { key: B::Key },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlowMap {
    Key,
    Colon,
    Value,
    Separator,
}

pub(super) struct Parser<'a, B: Builder> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    limits: Limits,
    nodes: u64,
    depth: usize,
    /// Each anchor's node and how many nodes it holds.
    anchors: OrderedTable<String, Binding<B::Node>>,
    handles: OrderedTable<String, String>,
    stack: Vec<Frame<B>>,
    builder: B,
    last_scalar_end: usize,
    pending: Option<Properties>,
    /// Properties that start a later line than `pending`: on an implicit key's
    /// line they are the key's, and `pending` the mapping's.
    later: Option<Properties>,
    /// The last position located: its offset, line start and column, so
    /// locating the cursor as it advances costs the bytes it crossed.
    located: Cell<(usize, usize, usize)>,
    /// A consumed document-prefix BOM is not an indentation column. Keep its
    /// original line origin for backwards location lookups within that document.
    bom_line: Option<usize>,
    document_start: Option<Span>,
    prologue_end: usize,
}

const fn is_break(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'\n' | b'\r'))
}

use super::scalar::flow_indicator as is_flow_indicator;

macro_rules! admit {
    ($parser:ident, $operation:expr) => {{
        let limit = $parser.builder.allocation_limit();
        $operation.map_err(|cause| Error::storage($parser.text, limit, cause, $parser.pos))?
    }};
}

impl<'a, B: Builder> Parser<'a, B> {
    fn split(
        &mut self,
        properties: Option<Properties>,
        at: usize,
        collection: bool,
    ) -> Result<PropertyParts, Error> {
        if let Some(properties) = &properties {
            self.builder
                .properties(at, collection, properties.spans)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        Ok(split(properties))
    }
    fn error(&self, kind: ErrorKind, at: usize) -> Error {
        Error::new(self.text, kind, at)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    /// Whether the byte at `at` is a space, tab, line break or the end.
    fn blank_at(&self, at: usize) -> bool {
        super::scalar::blank(self.peek_at(at))
    }

    /// Whether the cursor is on the indicator `c` followed by a blank.
    fn at_indicator(&self, c: u8) -> bool {
        self.peek() == Some(c) && self.blank_at(self.pos + 1)
    }

    /// The start of the line holding `at`, and `at`'s column in characters.
    fn locate(&self, at: usize) -> (usize, usize) {
        let (last, last_start, last_column) = self.located.get();
        let located = if at >= last {
            let crossed = &self.text[last..at];
            match crossed.rfind(['\n', '\r']) {
                Some(index) => {
                    let start = last + index + 1;
                    (start, self.text[start..at].chars().count())
                }
                None => (last_start, last_column + crossed.chars().count()),
            }
        } else {
            let start = self.text[..at]
                .rfind(['\n', '\r'])
                .map_or(0, |index| index + 1);
            (start, self.text[start..at].chars().count())
        };
        let located = if self.bom_line == Some(located.0) && at >= located.0 + '\u{feff}'.len_utf8()
        {
            (located.0 + '\u{feff}'.len_utf8(), located.1 - 1)
        } else {
            located
        };
        self.located.set((at, located.0, located.1));
        located
    }

    fn line_start(&self, at: usize) -> usize {
        self.locate(at).0
    }

    fn column(&self, at: usize) -> usize {
        self.locate(at).1
    }

    /// Whether the cursor is on a document marker (`---` or `...` at the start
    /// of a line, followed by a blank).
    fn at_marker(&self) -> bool {
        self.pos == self.line_start(self.pos)
            && super::scalar::document_marker(&self.bytes[self.pos..])
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn skip_to_line_end(&mut self) {
        while !matches!(self.peek(), None | Some(b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn take_break(&mut self) -> bool {
        match self.peek() {
            Some(b'\n') => {
                self.pos += 1;
                true
            }
            Some(b'\r') => {
                self.pos += 1;
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                true
            }
            _ => false,
        }
    }

    /// Whether `#` at the cursor starts a comment (it is preceded by a blank
    /// or starts a line).
    fn at_comment(&self) -> bool {
        self.peek() == Some(b'#')
            && (self.pos == self.line_start(self.pos)
                || matches!(self.bytes[self.pos - 1], b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Skip spaces, tabs, comments and line breaks: whether a line break was
    /// crossed.
    fn skip_separation(&mut self) -> Result<bool, Error> {
        let mut crossed = false;
        loop {
            self.skip_inline_space();
            if self.at_comment() {
                let start = self.pos;
                self.skip_to_line_end();
                self.builder
                    .comment(Span::new(start, self.pos), false)
                    .map_err(|(kind, at)| self.error(kind, at))?;
            }
            if !self.take_break() {
                return Ok(crossed);
            }
            crossed = true;
        }
    }

    /// Refuse a tab in the indentation of the line holding `at`. Only block
    /// structure (an entry, an indicator, an implicit key) is indented by
    /// spaces alone (§6.1); a tab before a flow node or a scalar is
    /// separation white space, which is why this is asked only where block
    /// structure starts.
    fn check_indentation(&self, at: usize) -> Result<(), Error> {
        let start = self.line_start(at);
        let indent = self.bytes[start..]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
        if self.bytes[start..start + indent].contains(&b'\t') {
            return Err(self.error(ErrorKind::Tab, start));
        }
        Ok(())
    }

    /// Whether the white space between the block indicator (`-`, `?`, `:`)
    /// before `at` and `at` holds a tab: a block collection or implicit key
    /// that starts there is indented by spaces only.
    fn tab_after_indicator(&self, at: usize) -> bool {
        let mut start = at;
        let mut tab = false;
        while start > 0 && matches!(self.bytes[start - 1], b' ' | b'\t') {
            tab |= self.bytes[start - 1] == b'\t';
            start -= 1;
        }
        tab && start > 0 && matches!(self.bytes[start - 1], b'-' | b'?' | b':')
    }

    fn count(&mut self, nodes: u64, at: usize) -> Result<(), Error> {
        self.nodes = self.nodes.saturating_add(nodes);
        if self.nodes > self.limits.max_nodes {
            return Err(self.error(
                ErrorKind::Nodes {
                    limit: self.limits.max_nodes,
                },
                at,
            ));
        }
        Ok(())
    }

    fn enter(&mut self, at: usize) -> Result<(), Error> {
        if self.depth >= self.limits.max_depth {
            return Err(self.error(
                ErrorKind::Depth {
                    limit: self.limits.max_depth,
                },
                at,
            ));
        }
        self.depth += 1;
        self.count(1, at)
    }

    fn scalar(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<B::Node, Error> {
        let end = if plain && text.is_empty() {
            at
        } else {
            self.last_scalar_end.max(at)
        };
        self.builder
            .scalar(text, plain, tag, at, end)
            .map_err(|(kind, at)| self.error(kind, at))
    }

    fn key_text(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<B::Key, Error> {
        let end = if plain && text.is_empty() {
            at
        } else {
            self.last_scalar_end.max(at)
        };
        self.builder
            .key_text(text, plain, tag, at, end)
            .map_err(|(kind, at)| self.error(kind, at))
    }

    fn key_node(
        &mut self,
        key: &B::Key,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<B::Node, Error> {
        self.builder
            .key_node(key, text, plain, tag, at)
            .map_err(|(kind, at)| self.error(kind, at))
    }

    fn collection_tag(&self, tag: Option<(Tag, usize)>, kind: Tag) -> Result<(), Error> {
        self.builder
            .collection_tag(tag, kind)
            .map_err(|(kind, at)| self.error(kind, at))
    }

    fn null(&mut self, at: usize) -> Result<B::Node, Error> {
        self.builder
            .null(at)
            .map_err(|(kind, at)| self.error(kind, at))
    }

    /// Recognize one tag (§6.9.1) before the builder judges its semantics.
    fn tag(&mut self) -> Result<Tag, Error> {
        let at = self.pos;
        let (expanded, decode_from) = if self.bytes[at..].starts_with(b"!<") {
            let close = self.text[at + 2..].find('>').ok_or_else(|| {
                self.error(ErrorKind::Syntax("the `>` that closes a verbatim tag"), at)
            })?;
            self.pos = at + 3 + close;
            let body = &self.text[at + 2..self.pos - 1];
            if !tag_uri(body, false) {
                return Err(self.error(ErrorKind::Tag, at));
            }
            (admit!(self, self.builder.allocations().string(body)), 0)
        } else {
            self.pos += 1;
            while !self.blank_at(self.pos) && !is_flow_indicator(self.peek()) {
                self.pos += self.text[self.pos..]
                    .chars()
                    .next()
                    .expect("a tag character")
                    .len_utf8();
            }
            let spelling = &self.text[at..self.pos];
            if spelling == "!" {
                if !self.blank_at(self.pos) && !matches!(self.peek(), Some(b',' | b']' | b'}')) {
                    return Err(self.error(
                        ErrorKind::Syntax("separation after a tag property"),
                        self.pos,
                    ));
                }
                self.builder
                    .tag(
                        Span::new(at, self.pos),
                        super::syntax::ExplicitTag::NonSpecific,
                    )
                    .map_err(|(kind, at)| self.error(kind, at))?;
                return Ok(Tag::NonSpecific);
            }
            let (handle, suffix) = if let Some(suffix) = spelling.strip_prefix("!!") {
                ("!!", suffix)
            } else if let Some(end) = spelling[1..].find('!') {
                let end = end + 2;
                (&spelling[..end], &spelling[end..])
            } else {
                ("!", &spelling[1..])
            };
            if !tag_handle(handle) || !tag_uri(suffix, true) {
                return Err(self.error(ErrorKind::Tag, at));
            }
            let prefix = match self
                .handles
                .get_by(|candidate| handle.cmp(candidate.as_str()))
            {
                Some(prefix) => prefix.as_str(),
                None if handle == "!!" => "tag:yaml.org,2002:",
                None if handle == "!" => "!",
                None => return Err(self.error(ErrorKind::UnknownTagHandle, at)),
            };
            let mut expanded = admit!(self, self.builder.allocations().string(prefix));
            let decode_from = expanded.len();
            admit!(
                self,
                self.builder.allocations().push_str(&mut expanded, suffix)
            );
            (expanded, decode_from)
        };
        if !self.blank_at(self.pos) && !matches!(self.peek(), Some(b',' | b']' | b'}')) {
            return Err(self.error(
                ErrorKind::Syntax("separation after a tag property"),
                self.pos,
            ));
        }
        let expanded = self.decode_tag(expanded, decode_from, at)?;
        let tag = Tag::specific(&expanded);
        if tag == Tag::Other && !B::LEXICAL {
            return Err(self.error(ErrorKind::Tag, at));
        }
        self.builder
            .tag(
                Span::new(at, self.pos),
                super::syntax::ExplicitTag::Specific(expanded),
            )
            .map_err(|(kind, at)| self.error(kind, at))?;
        Ok(tag)
    }

    /// Prefixes are decoded at declaration; decode only the newly read suffix.
    fn decode_tag(&self, text: String, from: usize, at: usize) -> Result<String, Error> {
        let mut bytes = text.into_bytes();
        let length = crate::percent::decode_in_place(&mut bytes[from..])
            .map_err(|_| self.error(ErrorKind::Tag, at))?;
        bytes.truncate(from + length);
        String::from_utf8(bytes).map_err(|_| self.error(ErrorKind::Tag, at))
    }

    /// The properties waiting for their node, those of several lines joined
    /// (§6.9: the separation between properties may cross lines).
    fn take_props(&mut self) -> Result<Option<Properties>, Error> {
        match (self.pending.take(), self.later.take()) {
            (properties, None) => Ok(properties),
            (None, later) => Ok(later),
            (Some(mut first), Some(later)) => {
                if later.anchor.is_some() {
                    if first.anchor.is_some() {
                        return Err(self.error(ErrorKind::Syntax("one anchor per node"), later.at));
                    }
                    first.anchor = later.anchor;
                    first.spans.anchor = later.spans.anchor;
                }
                if later.tag.is_some() {
                    if first.tag.is_some() {
                        return Err(self.error(ErrorKind::Syntax("one tag per node"), later.at));
                    }
                    first.tag = later.tag;
                    first.spans.tag = later.spans.tag;
                }
                Ok(Some(first))
            }
        }
    }

    /// Add the block-context property at the cursor to the properties waiting
    /// for their node: those on the line of an earlier property extend its set,
    /// and those on a new line start the set an implicit key on that line owns.
    fn add_property(&mut self) -> Result<(), Error> {
        let line = self.line_start(self.pos);
        if let Some(later) = &self.later
            && self.line_start(later.at) != line
        {
            // Three lines of properties: the first two are one set.
            let joined = self.take_props()?;
            self.pending = joined;
        }
        match (&self.pending, &self.later) {
            (Some(first), None) if self.line_start(first.at) != line => {
                let mut fresh = None;
                self.property(&mut fresh)?;
                self.later = fresh;
            }
            (_, Some(_)) => {
                let mut later = self.later.take();
                self.property(&mut later)?;
                self.later = later;
            }
            _ => {
                let mut pending = self.pending.take();
                self.property(&mut pending)?;
                self.pending = pending;
            }
        }
        Ok(())
    }

    /// Add the property at the cursor (an `&anchor` or a `!tag`) to
    /// `properties`.
    fn property(&mut self, properties: &mut Option<Properties>) -> Result<(), Error> {
        let at = self.pos;
        let column = self.column(at);
        let mut entry = properties.take().unwrap_or_else(|| Properties {
            anchor: None,
            tag: None,
            at,
            column,
            spans: super::build::PropertySpans::default(),
        });
        if self.peek() == Some(b'&') {
            if entry.anchor.is_some() {
                return Err(self.error(ErrorKind::Syntax("one anchor per node"), at));
            }
            let name = self.name()?;
            self.declare_anchor(&name, at)?;
            entry.anchor = Some(Anchor {
                name,
                declaration: at,
            });
            entry.spans.anchor = Some(Span::new(at, self.pos));
        } else {
            if entry.tag.is_some() {
                return Err(self.error(ErrorKind::Syntax("one tag per node"), at));
            }
            entry.tag = Some((self.tag()?, at));
            entry.spans.tag = Some(Span::new(at, self.pos));
        }
        *properties = Some(entry);
        Ok(())
    }

    /// An anchor's or alias's name, the cursor on its `&` or `*`.
    fn name(&mut self) -> Result<String, Error> {
        let at = self.pos;
        if !self.limits.aliases {
            let mut end = at + 1;
            while !self.blank_at(end) && !is_flow_indicator(self.peek_at(end)) {
                end += self.text[end..].chars().next().map_or(1, char::len_utf8);
            }
            return Err(self.error(
                ErrorKind::Alias {
                    anchor: self.peek() == Some(b'&'),
                    name: Span::new(at + 1, end),
                },
                at,
            ));
        }
        self.pos += 1;
        let start = self.pos;
        while !self.blank_at(self.pos) && !is_flow_indicator(self.peek()) {
            self.pos += self.text[self.pos..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
        }
        if self.pos == start {
            return Err(self.error(ErrorKind::Syntax("an anchor name"), start));
        }
        Ok(admit!(
            self,
            self.builder
                .allocations()
                .string(&self.text[start..self.pos])
        ))
    }

    /// The value an alias names, counted against the node bound before it is
    /// copied, so an alias bomb is refused before it is built.
    fn alias(&mut self) -> Result<B::Node, Error> {
        let at = self.pos;
        let name = self.name()?;
        let size = self
            .anchors
            .get(&name)
            .ok_or_else(|| {
                self.error(
                    ErrorKind::UnknownAlias {
                        name: Span::new(at + 1, self.pos),
                    },
                    at,
                )
            })?
            .ready
            .as_ref()
            .map(|(_, size)| *size)
            .ok_or_else(|| {
                self.error(
                    ErrorKind::CyclicAlias {
                        name: Span::new(at + 1, self.pos),
                    },
                    at,
                )
            })?;
        self.count(self.builder.alias_cost(size), at)?;
        let value = self
            .builder
            .alias(
                &self
                    .anchors
                    .get(&name)
                    .expect("a checked anchor")
                    .ready
                    .as_ref()
                    .expect("a completed anchor")
                    .0,
                &name,
                at,
                self.pos,
            )
            .map_err(|(kind, at)| self.error(kind, at))?;
        self.builder
            .discard_text(name)
            .map_err(|(kind, at)| self.error(kind, at))?;
        Ok(value)
    }

    fn register(&mut self, anchor: Option<Anchor>, value: &B::Node) -> Result<(), Error> {
        if let Some(name) = anchor {
            if self
                .anchors
                .get(&name.name)
                .is_some_and(|binding| binding.declaration != name.declaration)
            {
                return self
                    .builder
                    .discard_text(name.name)
                    .map_err(|(kind, at)| self.error(kind, at));
            }
            let named = self
                .builder
                .copy_node(value, self.pos)
                .map_err(|(kind, at)| self.error(kind, at))?;
            self.anchor(name, named)?;
        }
        Ok(())
    }

    /// Shadow immediately, before any part of the declared node is parsed.
    fn declare_anchor(&mut self, name: &str, declaration: usize) -> Result<(), Error> {
        if let Some(binding) = self
            .anchors
            .get_mut_by(|candidate| name.cmp(candidate.as_str()))
        {
            let old = mem::replace(
                binding,
                Binding {
                    declaration,
                    ready: None,
                },
            );
            if let Some((old, _)) = old.ready {
                self.builder
                    .discard_node(old)
                    .map_err(|(kind, at)| self.error(kind, at))?;
            }
            return Ok(());
        }
        let key = admit!(self, self.builder.allocations().string(name));
        admit!(
            self,
            self.builder.allocations().reserve_index(&mut self.anchors)
        );
        admit!(
            self,
            self.builder
                .allocations()
                .memory(|memory| self.anchors.insert(
                    key,
                    Binding {
                        declaration,
                        ready: None
                    },
                    memory
                ))
        );
        Ok(())
    }

    /// Complete only the declaration that is still current under this name.
    fn anchor(&mut self, anchor: Anchor, value: B::Node) -> Result<(), Error> {
        let size = self.builder.size(&value);
        let binding = self
            .anchors
            .get_mut(&anchor.name)
            .expect("a declared anchor");
        if binding.declaration == anchor.declaration {
            debug_assert!(binding.ready.is_none());
            binding.ready = Some((value, size));
        } else {
            self.builder
                .discard_node(value)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        self.builder
            .discard_text(anchor.name)
            .map_err(|(kind, at)| self.error(kind, at))?;
        Ok(())
    }

    /// After a scalar or alias: skip spaces and take a `:` value indicator
    /// (followed by a blank) if one is there.
    fn take_value_indicator(&mut self) -> bool {
        let save = self.pos;
        self.skip_inline_space();
        if self.at_indicator(b':') {
            self.pos += 1;
            return true;
        }
        self.pos = save;
        false
    }

    // ── scalars ───────────────────────────────────────────────────────────

    fn take_unquoted_character(&mut self) -> Result<(), Error> {
        let character = self.text[self.pos..]
            .chars()
            .next()
            .filter(|&character| super::scalar::non_break(character))
            .ok_or_else(|| {
                self.error(
                    ErrorKind::Syntax("an unquoted non-break character"),
                    self.pos,
                )
            })?;
        self.pos += character.len_utf8();
        Ok(())
    }

    /// One line of a plain scalar in block context: its text without trailing
    /// white space. It stops at a line break, at a comment, and at `:` followed
    /// by a blank; the cursor is left after the text.
    fn plain_line(&mut self) -> Result<&'a str, Error> {
        let start = self.pos;
        let mut end = self.pos;
        loop {
            if super::scalar::plain_end(self.bytes, self.pos, false) {
                break;
            }
            match self.peek() {
                Some(b' ' | b'\t') => self.pos += 1,
                _ => {
                    self.take_unquoted_character()?;
                    end = self.pos;
                }
            }
        }
        self.pos = end;
        Ok(&self.text[start..end])
    }

    /// Continue a block plain scalar over the lines indented more than
    /// `owner`, folding line breaks (§7.3.3).
    fn plain_rest(&mut self, mut text: String, owner: isize) -> Result<String, Error> {
        loop {
            let save = self.pos;
            self.skip_inline_space();
            if !is_break(self.peek()) {
                self.pos = save;
                return Ok(text);
            }
            let mut breaks = 0;
            while self.take_break() {
                breaks += 1;
                self.skip_inline_space();
            }
            // The line continues the scalar only when its indentation SPACES pass
            // the block's (a tab is separation, not indentation).
            let spaces = self.bytes[self.line_start(self.pos)..]
                .iter()
                .take_while(|&&b| b == b' ')
                .count();
            if self.pos >= self.bytes.len()
                || self.at_marker()
                || spaces as isize <= owner
                || self.at_comment()
            {
                self.pos = save;
                return Ok(text);
            }
            let at = self.pos;
            let line = self.owned_plain_line()?;
            if self.peek() == Some(b':') {
                return Err(self.error(
                    ErrorKind::Syntax("no mapping key inside a multi-line plain scalar"),
                    at,
                ));
            }
            if breaks == 1 {
                admit!(self, self.builder.allocations().push_char(&mut text, ' '));
            } else {
                admit!(
                    self,
                    self.builder
                        .allocations()
                        .repeat(&mut text, '\n', breaks - 1)
                );
            }
            admit!(self, self.builder.allocations().push_str(&mut text, &line));
            self.builder
                .discard_text(line)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
    }

    fn owned_plain_line(&mut self) -> Result<String, Error> {
        let start = self.pos;
        let line = self.plain_line()?;
        self.last_scalar_end = start + line.len();
        Ok(admit!(self, self.builder.allocations().string(line)))
    }

    /// One line of a plain scalar in flow context, stopping also at the flow
    /// indicators and at `:` followed by one.
    fn plain_flow_line(&mut self) -> Result<(usize, usize), Error> {
        let start = self.pos;
        let mut end = self.pos;
        loop {
            if super::scalar::plain_end(self.bytes, self.pos, true) {
                break;
            }
            match self.peek() {
                Some(b' ' | b'\t') => self.pos += 1,
                _ => {
                    self.take_unquoted_character()?;
                    end = self.pos;
                }
            }
        }
        self.pos = end;
        self.last_scalar_end = end;
        Ok((start, end))
    }

    /// A plain scalar in flow context, possibly over several lines.
    fn plain_flow(&mut self, owner: isize) -> Result<String, Error> {
        let (start, end) = self.plain_flow_line()?;
        let mut text = admit!(
            self,
            self.builder.allocations().string(&self.text[start..end])
        );
        loop {
            let save = self.pos;
            self.skip_inline_space();
            let mut breaks = 0;
            while self.take_break() {
                breaks += 1;
                self.skip_inline_space();
            }
            let stop = breaks == 0
                || self.pos >= self.bytes.len()
                || is_flow_indicator(self.peek())
                || self.at_comment()
                || (self.peek() == Some(b':')
                    && (self.blank_at(self.pos + 1)
                        || is_flow_indicator(self.peek_at(self.pos + 1))));
            if stop {
                self.pos = save;
                return Ok(text);
            }
            self.check_flow_line(self.pos, owner)?;
            let (start, end) = self.plain_flow_line()?;
            if breaks == 1 {
                admit!(self, self.builder.allocations().push_char(&mut text, ' '));
            } else {
                admit!(
                    self,
                    self.builder
                        .allocations()
                        .repeat(&mut text, '\n', breaks - 1)
                );
            }
            admit!(
                self,
                self.builder
                    .allocations()
                    .push_str(&mut text, &self.text[start..end])
            );
        }
    }

    /// Fold the line break at the cursor inside a quoted scalar (§7.3.1):
    /// one break is a space, `n` breaks with only white space between them are
    /// `n - 1` line feeds; the next line's leading white space is dropped.
    fn fold_quoted_break(&mut self, out: &mut String) -> Result<(), Error> {
        let mut breaks = 0;
        while self.take_break() {
            breaks += 1;
            self.skip_inline_space();
        }
        if breaks == 1 {
            admit!(self, self.builder.allocations().push_char(out, ' '));
        } else {
            admit!(
                self,
                self.builder.allocations().repeat(out, '\n', breaks - 1)
            );
        }
        Ok(())
    }

    /// Refuse the line a multi-line quoted scalar continues on when it is a
    /// document marker or is indented no more than the block `owner` that holds
    /// the scalar (§7.3.1: each line is prefixed by the block's indentation).
    fn check_continuation(&self, owner: isize) -> Result<(), Error> {
        let start = self.line_start(self.pos);
        if self.pos == start && self.at_marker() {
            return Err(self.error(
                ErrorKind::Syntax("no document marker inside a quoted scalar"),
                self.pos,
            ));
        }
        let spaces = self.bytes[start..self.pos]
            .iter()
            .take_while(|&&b| b == b' ')
            .count();
        if (spaces as isize) <= owner {
            return Err(self.error(
                ErrorKind::Syntax(
                    "a quoted scalar line indented more than the block that holds it",
                ),
                self.pos,
            ));
        }
        Ok(())
    }

    /// A single- or double-quoted scalar, the cursor on its opening quote: its
    /// text, and whether it spans lines. `owner` is the indentation of the
    /// block that holds it, or -1 at the top of a document.
    fn quoted(&mut self, owner: isize) -> Result<(String, bool), Error> {
        let start = self.pos;
        let quote = self.bytes[start];
        self.pos += 1;
        let mut out = String::new();
        let mut multiline = false;
        // Where the current run of source white space began in `out`: white
        // space before a line break is not content.
        let mut blank_from: Option<usize> = None;
        loop {
            match self.peek() {
                None => {
                    return Err(self.error(
                        ErrorKind::UnfinishedQuote {
                            quote: char::from(quote),
                            span: Span::new(start, self.text.len()),
                        },
                        start,
                    ));
                }
                Some(b'\'') if quote == b'\'' => {
                    if self.peek_at(self.pos + 1) == Some(b'\'') {
                        admit!(self, self.builder.allocations().push_char(&mut out, '\''));
                        self.builder
                            .escape(super::syntax::SyntaxEscape {
                                span: Span::new(self.pos, self.pos + 2),
                                decoded: Some('\''),
                            })
                            .map_err(|(kind, at)| self.error(kind, at))?;
                        self.pos += 2;
                        blank_from = None;
                        continue;
                    }
                    self.pos += 1;
                    self.last_scalar_end = self.pos;
                    return Ok((out, multiline));
                }
                Some(b'"') if quote == b'"' => {
                    self.pos += 1;
                    self.last_scalar_end = self.pos;
                    return Ok((out, multiline));
                }
                Some(b'\n' | b'\r') => {
                    multiline = true;
                    if let Some(from) = blank_from.take() {
                        out.truncate(from);
                    }
                    self.fold_quoted_break(&mut out)?;
                    self.check_continuation(owner)?;
                }
                Some(b'\\') if quote == b'"' => {
                    blank_from = None;
                    if is_break(self.peek_at(self.pos + 1)) {
                        // An escaped line break joins the lines with nothing.
                        multiline = true;
                        let escape_start = self.pos;
                        self.pos += 1;
                        self.take_break();
                        self.skip_inline_space();
                        while self.take_break() {
                            admit!(self, self.builder.allocations().push_char(&mut out, '\n'));
                            self.skip_inline_space();
                        }
                        self.check_continuation(owner)?;
                        self.builder
                            .escape(super::syntax::SyntaxEscape {
                                span: Span::new(escape_start, self.pos),
                                decoded: None,
                            })
                            .map_err(|(kind, at)| self.error(kind, at))?;
                        continue;
                    }
                    let (c, width) = self.escape()?;
                    self.builder
                        .escape(super::syntax::SyntaxEscape {
                            span: Span::new(self.pos, self.pos + width),
                            decoded: Some(c),
                        })
                        .map_err(|(kind, at)| self.error(kind, at))?;
                    admit!(self, self.builder.allocations().push_char(&mut out, c));
                    self.pos += width;
                }
                Some(b' ' | b'\t') => {
                    blank_from.get_or_insert(out.len());
                    admit!(
                        self,
                        self.builder
                            .allocations()
                            .push_char(&mut out, char::from(self.bytes[self.pos]))
                    );
                    self.pos += 1;
                }
                Some(_) => {
                    let c = self.text[self.pos..].chars().next().expect("a character");
                    admit!(self, self.builder.allocations().push_char(&mut out, c));
                    self.pos += c.len_utf8();
                    blank_from = None;
                }
            }
        }
    }

    /// The double-quoted escape at the cursor (§5.7): the character and the
    /// bytes the escape spans.
    fn escape(&self) -> Result<(char, usize), Error> {
        let at = self.pos;
        let width = self
            .peek_at(at + 1)
            .and_then(super::scalar::hex_width)
            .map_or_else(
                || {
                    self.text[at + 1..]
                        .chars()
                        .next()
                        .map_or(1, |c| 1 + c.len_utf8())
                },
                |width| 2 + width,
            );
        let mut end = (at + width).min(self.text.len());
        while !self.text.is_char_boundary(end) {
            end += 1;
        }
        let refused = || {
            self.error(
                ErrorKind::Escape {
                    span: Span::new(at, end),
                },
                at,
            )
        };
        let hex = |width: usize| -> Result<(char, usize), Error> {
            let digits = self.text.get(at + 2..at + 2 + width).ok_or_else(refused)?;
            let mut code = 0_u32;
            for byte in digits.bytes() {
                let digit = purrdf_hash::hex::nibble(byte).ok_or_else(refused)?;
                code = (code << 4) | u32::from(digit);
            }
            char::from_u32(code)
                .map(|c| (c, 2 + width))
                .ok_or_else(refused)
        };
        let name = self.peek_at(at + 1).ok_or_else(refused)?;
        if let Some(width) = super::scalar::hex_width(name) {
            return hex(width);
        }
        let short = super::scalar::short_escape(name).ok_or_else(refused)?;
        Ok((short, 2))
    }

    /// A literal (`|`) or folded (`>`) block scalar, the cursor on its
    /// indicator, inside a block indented `owner` (§8.1).
    fn block_scalar(&mut self, owner: isize) -> Result<String, Error> {
        let block_start = self.pos;
        let literal = self.peek() == Some(b'|');
        self.pos += 1;
        let (mut chomping, mut increment) = (0_i8, 0_usize);
        for _ in 0..2 {
            match self.peek() {
                Some(sign @ (b'+' | b'-')) if chomping == 0 => {
                    chomping = if sign == b'+' { 1 } else { -1 };
                    self.pos += 1;
                }
                Some(digit @ b'1'..=b'9') if increment == 0 => {
                    increment = usize::from(digit - b'0');
                    self.pos += 1;
                }
                Some(b'0') => {
                    return Err(self.error(
                        ErrorKind::Syntax("an indentation indicator from 1 to 9"),
                        self.pos,
                    ));
                }
                _ => break,
            }
        }
        self.skip_inline_space();
        if self.at_comment() {
            let start = self.pos;
            self.skip_to_line_end();
            self.builder
                .comment(Span::new(start, self.pos), true)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        let header_end = self.pos;
        if self.pos >= self.bytes.len() {
            self.builder
                .block(
                    block_start,
                    super::syntax::BlockHeader {
                        span: Span::new(block_start, header_end),
                        chomping: match chomping {
                            -1 => super::syntax::Chomping::Strip,
                            1 => super::syntax::Chomping::Keep,
                            _ => super::syntax::Chomping::Clip,
                        },
                        indent_indicator: (increment != 0).then_some(increment),
                        content_indent: 0,
                    },
                )
                .map_err(|(kind, at)| self.error(kind, at))?;
            self.last_scalar_end = self.pos;
            return Ok(String::new());
        }
        if !self.take_break() {
            return Err(self.error(
                ErrorKind::Syntax("a line break after a block scalar header"),
                self.pos,
            ));
        }
        let mut indent = match (increment, usize::try_from(owner)) {
            (0, _) => None,
            (increment, Ok(owner)) => Some(owner + increment),
            (increment, Err(_)) => Some(increment),
        };
        let (mut out, mut leading_break, mut trailing_breaks) =
            (String::new(), String::new(), String::new());
        let mut leading_blank = false;
        self.block_breaks(&mut indent, &mut trailing_breaks, owner)?;
        while let Some(content) = indent {
            // A zero-indented scalar (a document's only node) ends at a
            // document marker.
            if self.pos >= self.bytes.len()
                || self.column(self.pos) != content
                || (content == 0 && self.at_marker())
            {
                break;
            }
            let trailing_blank = matches!(self.peek(), Some(b' ' | b'\t'));
            if !literal && leading_break == "\n" && !leading_blank && !trailing_blank {
                if trailing_breaks.is_empty() {
                    admit!(self, self.builder.allocations().push_char(&mut out, ' '));
                }
            } else {
                admit!(
                    self,
                    self.builder
                        .allocations()
                        .push_str(&mut out, &leading_break)
                );
            }
            leading_break.clear();
            admit!(
                self,
                self.builder
                    .allocations()
                    .push_str(&mut out, &trailing_breaks)
            );
            trailing_breaks.clear();
            leading_blank = trailing_blank;
            let start = self.pos;
            while !matches!(self.peek(), None | Some(b'\n' | b'\r')) {
                self.take_unquoted_character()?;
            }
            admit!(
                self,
                self.builder
                    .allocations()
                    .push_str(&mut out, &self.text[start..self.pos])
            );
            // A last line of white space alone is closed by the end of the stream
            // as by a line break (the suite's `L24T`); a last line with text is
            // not, so `k: |\n  x` at the end of a stream stays `x`, as `serde_yaml`
            // read it.
            let blank_last_line = self.pos >= self.bytes.len()
                && self.text[start..self.pos]
                    .bytes()
                    .all(|b| b == b' ' || b == b'\t');
            if self.take_break() || blank_last_line {
                admit!(
                    self,
                    self.builder
                        .allocations()
                        .push_char(&mut leading_break, '\n')
                );
            }
            self.block_breaks(&mut indent, &mut trailing_breaks, owner)?;
        }
        if chomping != -1 {
            admit!(
                self,
                self.builder
                    .allocations()
                    .push_str(&mut out, &leading_break)
            );
        }
        if chomping == 1 {
            admit!(
                self,
                self.builder
                    .allocations()
                    .push_str(&mut out, &trailing_breaks)
            );
        }
        self.builder
            .block(
                block_start,
                super::syntax::BlockHeader {
                    span: Span::new(block_start, header_end),
                    chomping: match chomping {
                        -1 => super::syntax::Chomping::Strip,
                        1 => super::syntax::Chomping::Keep,
                        _ => super::syntax::Chomping::Clip,
                    },
                    indent_indicator: (increment != 0).then_some(increment),
                    content_indent: indent.unwrap_or(0),
                },
            )
            .map_err(|(kind, at)| self.error(kind, at))?;
        self.builder
            .discard_text(leading_break)
            .map_err(|(kind, at)| self.error(kind, at))?;
        self.builder
            .discard_text(trailing_breaks)
            .map_err(|(kind, at)| self.error(kind, at))?;
        self.last_scalar_end = if self.pos == self.text.len() {
            self.pos
        } else {
            self.line_start(self.pos)
        };
        Ok(out)
    }

    /// Consume the indentation and empty lines before a block scalar's next
    /// content line, fixing the content indentation when it is still to be
    /// detected (`indent` is `None`): the first content line's own spaces, and
    /// no leading empty line may hold more (§8.1.1.1).
    fn block_breaks(
        &mut self,
        indent: &mut Option<usize>,
        breaks: &mut String,
        owner: isize,
    ) -> Result<(), Error> {
        let floor = usize::try_from(owner + 1).unwrap_or(0);
        let mut widest_empty = 0;
        loop {
            while indent.is_none_or(|width| self.column(self.pos) < width)
                && self.peek() == Some(b' ')
            {
                self.pos += 1;
            }
            let column = self.column(self.pos);
            // A tab is content once the indentation spaces are all there
            // (§6.1); short of them it stands where a space must.
            if self.peek() == Some(b'\t') && column < indent.unwrap_or(floor) {
                return Err(self.error(ErrorKind::Tab, self.pos));
            }
            if self.take_break() {
                widest_empty = widest_empty.max(column);
                admit!(self, self.builder.allocations().push_char(breaks, '\n'));
                continue;
            }
            if self.pos >= self.bytes.len() {
                // White space alone on the last line is an empty line.
                if column > 0 {
                    admit!(self, self.builder.allocations().push_char(breaks, '\n'));
                }
            } else if indent.is_none() && column >= floor && widest_empty > column {
                return Err(self.error(
                    ErrorKind::Syntax(
                        "no empty line before the first content line with more spaces than it",
                    ),
                    self.pos,
                ));
            }
            break;
        }
        if indent.is_none() {
            let column = self.column(self.pos);
            *indent = Some(if self.pos >= self.bytes.len() {
                floor
            } else {
                column.max(floor)
            });
        }
        Ok(())
    }

    // ── flow collections ──────────────────────────────────────────────────

    /// Skip white space, line breaks and comments inside a flow collection.
    fn skip_flow_space(&mut self) -> Result<(), Error> {
        loop {
            self.skip_inline_space();
            if self.at_comment() {
                let start = self.pos;
                self.skip_to_line_end();
                self.builder
                    .comment(Span::new(start, self.pos), false)
                    .map_err(|(kind, at)| self.error(kind, at))?;
            }
            if !self.take_break() {
                return Ok(());
            }
        }
    }

    /// Refuse a document marker, or a line indented no more than its block,
    /// where a token of a flow collection starts a line: `owner` is the
    /// indentation of the block that holds the collection, or -1 at the top.
    fn check_flow_line(&self, at: usize, owner: isize) -> Result<(), Error> {
        let start = self.line_start(at);
        if !self.bytes[start..at]
            .iter()
            .all(|&b| b == b' ' || b == b'\t')
        {
            return Ok(());
        }
        if at == start && self.bytes[at..].starts_with(b"---") && self.blank_at(at + 3)
            || at == start && self.bytes[at..].starts_with(b"...") && self.blank_at(at + 3)
        {
            return Err(self.error(
                ErrorKind::Syntax("no document marker inside a flow collection"),
                at,
            ));
        }
        let spaces = self.bytes[start..at]
            .iter()
            .take_while(|&&b| b == b' ')
            .count();
        if (spaces as isize) <= owner {
            return Err(self.error(
                ErrorKind::Syntax("a flow line indented more than the block that holds it"),
                at,
            ));
        }
        Ok(())
    }

    /// A scalar node of a flow collection: the value to hand the collection.
    /// In a mapping's key position it is the key's string; either way its
    /// anchor names what it denotes.
    fn flow_scalar(
        &mut self,
        text: String,
        plain: bool,
        properties: Option<Properties>,
        key_position: bool,
        at: usize,
    ) -> Result<B::Node, Error> {
        let (anchor, tag) = self.split(properties, at, false)?;
        if key_position {
            let copied = admit!(self, self.builder.allocations().string(&text));
            let key = self.key_text(copied, plain, tag, at)?;
            if let Some(name) = anchor {
                let named = self.key_node(&key, text, plain, tag, at)?;
                self.count(1, at)?;
                self.anchor(name, named)?;
            } else {
                self.builder
                    .discard_text(text)
                    .map_err(|(kind, at)| self.error(kind, at))?;
                self.count(1, at)?;
            }
            Ok(self.builder.key_value(key))
        } else {
            let value = self.scalar(text, plain, tag, at)?;
            self.count(1, at)?;
            self.register(anchor, &value)?;
            Ok(value)
        }
    }

    /// A flow collection, the cursor on its `[` or `{`, however deep, over a
    /// heap stack. `owner` is the indentation of the block that holds it, or
    /// -1 at the top of a document.
    fn flow(&mut self, properties: Option<Properties>, owner: isize) -> Result<B::Node, Error> {
        let scalar_keys = self.limits.scalar_keys;
        let mut stack: Vec<Flow<B>> = Vec::new();
        let mut pending = properties;
        // An entry that began with `?`, and whether its node is still to come.
        let (mut explicit, mut fresh) = (false, false);
        loop {
            let before = self.pos;
            self.skip_flow_space()?;
            let at = self.pos;
            let crossed_break = self.text[before..at].contains(['\n', '\r']);
            let Some(byte) = self.peek() else {
                let (start, expected) = stack
                    .iter()
                    .rev()
                    .find_map(|frame| match frame {
                        Flow::Sequence { start, .. } => Some((*start, ']')),
                        Flow::Mapping { start, .. } => Some((*start, '}')),
                        Flow::Pair { .. } => None,
                    })
                    .expect("an open flow collection");
                return Err(self.error(
                    ErrorKind::UnfinishedFlow {
                        opener: Span::new(start, start + 1),
                        expected,
                    },
                    at,
                ));
            };
            self.check_flow_line(at, owner)?;
            // A node with properties and no content (`!!str ,`), or the empty
            // node after `?`, where the collection expects a node.
            let terminator = matches!(byte, b',' | b']' | b'}')
                || (byte == b':'
                    && (self.blank_at(at + 1) || is_flow_indicator(self.peek_at(at + 1))));
            let expects_node = match stack.last() {
                Some(Flow::Sequence { after_item, .. }) => !after_item,
                Some(Flow::Mapping { state, .. }) => matches!(state, FlowMap::Key | FlowMap::Value),
                Some(Flow::Pair { .. }) => true,
                None => false,
            };
            if terminator && expects_node && (pending.is_some() || (explicit && fresh)) {
                let key_position = matches!(
                    stack.last(),
                    Some(Flow::Mapping {
                        state: FlowMap::Key,
                        ..
                    })
                );
                let value =
                    self.flow_scalar(String::new(), true, pending.take(), key_position, at)?;
                fresh = false;
                if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                    admit!(self, self.builder.allocations().discard_vec(stack));
                    return Ok(done);
                }
                continue;
            }
            // Separators and closers, where the innermost collection expects one.
            match stack.last_mut() {
                Some(Flow::Sequence { after_item, .. }) => match byte {
                    b']' => {
                        self.pos += 1;
                        let value = self.close_flow(&mut stack)?;
                        if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                            admit!(self, self.builder.allocations().discard_vec(stack));
                            return Ok(done);
                        }
                        continue;
                    }
                    b',' if *after_item => {
                        self.pos += 1;
                        *after_item = false;
                        explicit = false;
                        continue;
                    }
                    b':' if *after_item => {
                        if crossed_break && !explicit {
                            return Err(
                                self.error(ErrorKind::Syntax("an implicit key on one line"), at)
                            );
                        }
                        // `[key: value]`: the last item is the key of a
                        // single-pair mapping.
                        self.pos += 1;
                        explicit = false;
                        let Some(Flow::Sequence {
                            items, after_item, ..
                        }) = stack.last_mut()
                        else {
                            unreachable!("the innermost collection is a sequence");
                        };
                        let key = match items.pop() {
                            Some(item) => self
                                .builder
                                .key_of(item, scalar_keys, at)
                                .map_err(|(kind, at)| Error::new(self.text, kind, at))?,
                            None => None,
                        };
                        let Some(key) = key else {
                            return Err(self.error(ErrorKind::NonStringKey, at));
                        };
                        *after_item = false;
                        admit!(
                            self,
                            self.builder
                                .allocations()
                                .push(&mut stack, Flow::Pair { key })
                        );
                        continue;
                    }
                    _ if *after_item => {
                        return Err(
                            self.error(ErrorKind::Syntax("`,` or `]` in a flow sequence"), at)
                        );
                    }
                    b',' => {
                        return Err(self.error(ErrorKind::Syntax("an entry before `,`"), at));
                    }
                    _ => {}
                },
                Some(Flow::Mapping {
                    state, key, object, ..
                }) => match (*state, byte) {
                    (_, b'}') => {
                        if let Some((name, key_at)) = key.take() {
                            let null = self
                                .builder
                                .null(at)
                                .map_err(|(kind, at)| self.error(kind, at))?;
                            self.builder
                                .insert(object, name, null, key_at)
                                .map_err(|(kind, at)| self.error(kind, at))?;
                        }
                        self.pos += 1;
                        let value = self.close_flow(&mut stack)?;
                        if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                            admit!(self, self.builder.allocations().discard_vec(stack));
                            return Ok(done);
                        }
                        continue;
                    }
                    (FlowMap::Separator, b',') => {
                        self.pos += 1;
                        *state = FlowMap::Key;
                        explicit = false;
                        continue;
                    }
                    (FlowMap::Colon | FlowMap::Value, b',') => {
                        let (name, key_at) = key.take().expect("a key precedes its value");
                        let null = self
                            .builder
                            .null(at)
                            .map_err(|(kind, at)| self.error(kind, at))?;
                        self.builder
                            .insert(object, name, null, key_at)
                            .map_err(|(kind, at)| self.error(kind, at))?;
                        self.pos += 1;
                        *state = FlowMap::Key;
                        explicit = false;
                        continue;
                    }
                    (FlowMap::Colon, b':') => {
                        self.pos += 1;
                        *state = FlowMap::Value;
                        explicit = false;
                        continue;
                    }
                    (FlowMap::Colon | FlowMap::Separator, _) => {
                        return Err(
                            self.error(ErrorKind::Syntax("`,` or `}` in a flow mapping"), at)
                        );
                    }
                    (FlowMap::Key, b',') => {
                        return Err(self.error(ErrorKind::Syntax("an entry before `,`"), at));
                    }
                    (FlowMap::Key, b':')
                        if self.blank_at(at + 1) || is_flow_indicator(self.peek_at(at + 1)) =>
                    {
                        if !B::LEXICAL {
                            return Err(self.error(ErrorKind::NonStringKey, at));
                        }
                        let empty = self
                            .builder
                            .null(at)
                            .map_err(|(kind, at)| Error::new(self.text, kind, at))?;
                        *key = Some((
                            self.builder
                                .key_of(empty, true, at)
                                .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                                .expect("a lexical key"),
                            at,
                        ));
                        *state = FlowMap::Value;
                        self.pos += 1;
                        continue;
                    }
                    _ => {}
                },
                Some(Flow::Pair { .. }) => {
                    if matches!(byte, b',' | b']') {
                        let null = self.null(at)?;
                        if let Some(done) = self.attach_flow(&mut stack, null, at)? {
                            admit!(self, self.builder.allocations().discard_vec(stack));
                            return Ok(done);
                        }
                        continue;
                    }
                }
                None => {}
            }
            // A node.
            match byte {
                b'[' | b'{' => {
                    let (anchor, tag) = self.split(pending.take(), at, true)?;
                    self.collection_tag(tag, if byte == b'[' { Tag::Seq } else { Tag::Map })?;
                    self.enter(at)?;
                    self.pos += 1;
                    fresh = false;
                    let frame = if byte == b'[' {
                        Flow::Sequence {
                            items: Vec::new(),
                            anchor,
                            start: at,
                            after_item: false,
                        }
                    } else {
                        Flow::Mapping {
                            object: self.builder.new_map(),
                            anchor,
                            start: at,
                            key: None,
                            state: FlowMap::Key,
                        }
                    };
                    admit!(self, self.builder.allocations().push(&mut stack, frame));
                }
                b'&' | b'!' => self.property(&mut pending)?,
                b'*' => {
                    if pending.is_some() {
                        return Err(
                            self.error(ErrorKind::Syntax("an alias without properties"), at)
                        );
                    }
                    let value = self.alias()?;
                    fresh = false;
                    if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                        admit!(self, self.builder.allocations().discard_vec(stack));
                        return Ok(done);
                    }
                }
                b'?' if self.blank_at(at + 1) => {
                    self.builder
                        .question(Span::new(at, at + 1))
                        .map_err(|(kind, at)| self.error(kind, at))?;
                    self.pos += 1;
                    explicit = true;
                    fresh = true;
                }
                b'|' | b'>' | b'%' | b'@' | b'`' | b'#' => {
                    return Err(self.error(ErrorKind::Syntax("a flow node"), at));
                }
                b'-' | b'?' if self.blank_at(at + 1) || is_flow_indicator(self.peek_at(at + 1)) => {
                    return Err(self.error(
                        ErrorKind::Syntax(
                            "a flow node; `-` before a blank or a flow indicator is not a plain scalar",
                        ),
                        at,
                    ));
                }
                _ => {
                    // A quoted key may be followed by `:` directly (§7.4.1);
                    // a plain one stops before `:` and a blank or indicator.
                    let (text, plain) = if matches!(byte, b'"' | b'\'') {
                        (self.quoted(owner)?.0, false)
                    } else {
                        (self.plain_flow(owner)?, true)
                    };
                    let key_position = matches!(
                        stack.last(),
                        Some(Flow::Mapping {
                            state: FlowMap::Key,
                            ..
                        })
                    );
                    let value = self.flow_scalar(text, plain, pending.take(), key_position, at)?;
                    fresh = false;
                    if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                        admit!(self, self.builder.allocations().discard_vec(stack));
                        return Ok(done);
                    }
                }
            }
        }
    }

    /// Close the innermost flow collection: its value, its anchor registered.
    fn close_flow(&mut self, stack: &mut Vec<Flow<B>>) -> Result<B::Node, Error> {
        self.depth -= 1;
        let (value, anchor) = match stack.pop() {
            Some(Flow::Sequence {
                items,
                anchor,
                start,
                ..
            }) => (self.builder.sequence(items, start, self.pos, true), anchor),
            Some(Flow::Mapping {
                object,
                anchor,
                start,
                ..
            }) => (self.builder.mapping(object, start, self.pos, true), anchor),
            _ => unreachable!("only an open collection closes"),
        };
        let value = value.map_err(|(kind, at)| self.error(kind, at))?;
        self.register(anchor, &value)?;
        Ok(value)
    }

    /// Hand a complete flow node to the innermost collection; the whole value
    /// once the outermost collection closes.
    fn attach_flow(
        &mut self,
        stack: &mut Vec<Flow<B>>,
        value: B::Node,
        at: usize,
    ) -> Result<Option<B::Node>, Error> {
        let mut value = value;
        loop {
            match stack.last_mut() {
                None => return Ok(Some(value)),
                Some(Flow::Sequence {
                    items, after_item, ..
                }) => {
                    admit!(self, self.builder.allocations().push(items, value));
                    *after_item = true;
                    return Ok(None);
                }
                Some(Flow::Mapping {
                    state, key, object, ..
                }) => {
                    match *state {
                        FlowMap::Key => {
                            let Some(name) = self
                                .builder
                                .key_of(value, self.limits.scalar_keys, at)
                                .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                            else {
                                return Err(self.error(ErrorKind::NonStringKey, at));
                            };
                            *key = Some((name, at));
                            *state = FlowMap::Colon;
                        }
                        FlowMap::Value => {
                            let (name, key_at) = key.take().expect("a key precedes its value");
                            self.builder
                                .insert(object, name, value, key_at)
                                .map_err(|(kind, at)| self.error(kind, at))?;
                            *state = FlowMap::Separator;
                        }
                        FlowMap::Colon | FlowMap::Separator => {
                            return Err(
                                self.error(ErrorKind::Syntax("`,` or `}` in a flow mapping"), at)
                            );
                        }
                    }
                    return Ok(None);
                }
                Some(Flow::Pair { .. }) => {
                    let Some(Flow::Pair { key }) = stack.pop() else {
                        unreachable!("the innermost frame is a pair");
                    };
                    let mut pair = self.builder.new_map();
                    self.builder
                        .insert(&mut pair, key, value, at)
                        .map_err(|(kind, at)| self.error(kind, at))?;
                    value = self
                        .builder
                        .mapping(pair, at, self.pos, true)
                        .map_err(|(kind, at)| self.error(kind, at))?;
                }
            }
        }
    }

    // ── block structure ───────────────────────────────────────────────────

    /// The open slot of the innermost block collection: the indentation of
    /// its owner, and whether it is a mapping value (which an indentless
    /// sequence may fill).
    fn open_slot(&self) -> Option<(isize, bool)> {
        match self.stack.last()? {
            Frame::Root(None) => Some((-1, false)),
            Frame::Sequence {
                indent, open: true, ..
            } => Some((*indent as isize, false)),
            Frame::Mapping {
                indent,
                slot: Slot::ExplicitKey,
                ..
            } => Some((*indent as isize, false)),
            Frame::Mapping {
                indent,
                slot: Slot::Value(..),
                ..
            } => Some((*indent as isize, true)),
            _ => None,
        }
    }

    /// Fill the innermost open slot with a complete node.
    fn fill(&mut self, value: B::Node, at: usize) -> Result<(), Error> {
        let scalar_keys = self.limits.scalar_keys;
        let refused = match self.stack.last_mut().expect("the root frame stays") {
            Frame::Root(slot) => {
                *slot = Some(value);
                None
            }
            Frame::Sequence { items, open, .. } => {
                admit!(self, self.builder.allocations().push(items, value));
                *open = false;
                None
            }
            Frame::Mapping { object, slot, .. } => match mem::replace(slot, Slot::Idle) {
                Slot::Value(key, key_at) => self.builder.insert(object, key, value, key_at).err(),
                Slot::ExplicitKey => match self.builder.key_of(value, scalar_keys, at) {
                    Ok(Some(key)) => {
                        *slot = Slot::KeyDone(key, at);
                        None
                    }
                    Ok(None) => Some((ErrorKind::NonStringKey, at)),
                    Err(refusal) => Some(refusal),
                },
                Slot::Idle | Slot::KeyDone(..) => unreachable!("only an open slot is filled"),
            },
        };
        refused.map_or(Ok(()), |(kind, at)| Err(self.error(kind, at)))
    }

    /// Fill the open slot with a scalar carrying `properties`.
    fn fill_scalar(
        &mut self,
        text: String,
        plain: bool,
        properties: Option<Properties>,
        at: usize,
    ) -> Result<(), Error> {
        let (anchor, tag) = self.split(properties, at, false)?;
        let explicit_key = matches!(
            self.stack.last(),
            Some(Frame::Mapping {
                slot: Slot::ExplicitKey,
                ..
            })
        );
        if explicit_key && self.limits.scalar_keys {
            // A scalar after `?` is a key; a number, boolean or null one keeps
            // its source text, as an implicit key does.
            let copied = admit!(self, self.builder.allocations().string(&text));
            let key = self.key_text(copied, plain, tag, at)?;
            let value = self.key_node(&key, text, plain, tag, at)?;
            self.count(1, at)?;
            if let Some(name) = anchor {
                self.anchor(name, value)?;
            } else {
                self.builder
                    .discard_node(value)
                    .map_err(|(kind, at)| self.error(kind, at))?;
            }
            return self.fill_key(key, at);
        }
        let value = self.scalar(text, plain, tag, at)?;
        self.count(1, at)?;
        self.register(anchor, &value)?;
        self.fill(value, at)
    }

    /// Complete the explicit key the open slot waits for.
    fn fill_key(&mut self, key: B::Key, at: usize) -> Result<(), Error> {
        let Some(Frame::Mapping { slot, .. }) = self.stack.last_mut() else {
            unreachable!("an explicit key fills a mapping's slot");
        };
        *slot = Slot::KeyDone(key, at);
        Ok(())
    }

    /// Close the innermost block collection and fill its parent's slot.
    fn close_top(&mut self) -> Result<(), Error> {
        let (value, anchor, start) = match self.stack.pop() {
            Some(Frame::Sequence {
                mut items,
                open,
                anchor,
                start,
                ..
            }) => {
                if open {
                    let null = self.null(self.pos)?;
                    admit!(self, self.builder.allocations().push(&mut items, null));
                }
                (
                    self.builder.sequence(items, start, self.pos, false),
                    anchor,
                    start,
                )
            }
            Some(Frame::Mapping {
                mut object,
                slot,
                anchor,
                start,
                ..
            }) => {
                match slot {
                    Slot::Value(key, key_at) | Slot::KeyDone(key, key_at) => {
                        let null = self.null(self.pos)?;
                        self.builder
                            .insert(&mut object, key, null, key_at)
                            .map_err(|(kind, at)| self.error(kind, at))?;
                    }
                    Slot::ExplicitKey => return Err(self.error(ErrorKind::NonStringKey, start)),
                    Slot::Idle => {}
                }
                (
                    self.builder.mapping(object, start, self.pos, false),
                    anchor,
                    start,
                )
            }
            _ => unreachable!("the root frame is never closed here"),
        };
        let value = value.map_err(|(kind, at)| self.error(kind, at))?;
        self.depth -= 1;
        self.register(anchor, &value)?;
        self.fill(value, start)
    }

    /// Open a block mapping at `column` whose first key is the scalar `text`
    /// (`plain` when it resolves by the core schema).
    fn open_mapping(
        &mut self,
        column: usize,
        input: KeyInput<B::Key>,
        key_at: usize,
        line: Line,
    ) -> Result<(), Error> {
        if line == Line::Inline {
            return Err(self.error(
                ErrorKind::Syntax(
                    "a line break before a block mapping (none may start on this line)",
                ),
                key_at,
            ));
        }
        // Properties on the key's line belong to the key, and the mapping
        // starts where they do; properties on an earlier line belong to the
        // mapping.
        self.check_indentation(key_at)?;
        if line == Line::Compact && self.tab_after_indicator(key_at) {
            return Err(self.error(ErrorKind::Tab, key_at));
        }
        let key_line = self.line_start(key_at);
        let (mapping, key_properties, indent) = match (self.pending.take(), self.later.take()) {
            (Some(mapping), Some(key)) if self.line_start(key.at) == key_line => {
                let indent = key.column;
                (Some(mapping), Some(key), indent)
            }
            (Some(first), Some(later)) => {
                self.pending = Some(first);
                self.later = Some(later);
                (self.take_props()?, None, column)
            }
            (Some(properties), None) if self.line_start(properties.at) == key_line => {
                let indent = properties.column;
                (None, Some(properties), indent)
            }
            (other, None) => (other, None, column),
            (None, Some(_)) => unreachable!("a later property follows an earlier one"),
        };
        let key_spans = key_properties.as_ref().map(|properties| properties.spans);
        let (key_anchor, key_tag) = self.split(key_properties, key_at, false)?;
        let key = match input {
            KeyInput::Scalar(text, plain) => {
                let copied = admit!(self, self.builder.allocations().string(&text));
                let key = self.key_text(copied, plain, key_tag, key_at)?;
                if let Some(name) = key_anchor {
                    let node = self.key_node(&key, text, plain, key_tag, key_at)?;
                    self.anchor(name, node)?;
                } else {
                    self.builder
                        .discard_text(text)
                        .map_err(|(kind, at)| self.error(kind, at))?;
                }
                key
            }
            KeyInput::Ready(key) => {
                if let Some(spans) = key_spans {
                    let copied = self
                        .builder
                        .copy_key(&key, key_at)
                        .map_err(|(kind, at)| self.error(kind, at))?;
                    let node = self.builder.key_value(copied);
                    self.builder
                        .decorate(&node, spans)
                        .map_err(|(kind, at)| self.error(kind, at))?;
                    self.register(key_anchor, &node)?;
                }
                key
            }
        };
        let (anchor, tag) = self.split(mapping, key_at, true)?;
        self.collection_tag(tag, Tag::Map)?;
        self.enter(key_at)?;
        self.count(1, key_at)?;
        let frame = Frame::Mapping {
            indent,
            start: key_at,
            object: self.builder.new_map(),
            slot: Slot::Value(key, key_at),
            anchor,
        };
        admit!(
            self,
            self.builder.allocations().push(&mut self.stack, frame)
        );
        Ok(())
    }

    /// A block mapping's implicit key, the cursor on it, through its `:`.
    fn block_key(&mut self) -> Result<(B::Key, usize), Error> {
        let mut properties = None;
        while matches!(self.peek(), Some(b'&' | b'!')) {
            self.property(&mut properties)?;
            self.skip_inline_space();
        }
        let at = self.pos;
        let (text, plain) = match self.peek() {
            Some(b'*') => {
                if properties.is_some() {
                    return Err(self.error(ErrorKind::Syntax("an alias without properties"), at));
                }
                let value = self.alias()?;
                let Some(key) = self
                    .builder
                    .key_of(value, self.limits.scalar_keys, at)
                    .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                else {
                    return Err(self.error(ErrorKind::NonStringKey, at));
                };
                if !self.take_value_indicator() {
                    return Err(self.error(ErrorKind::Syntax("`:` after a mapping key"), self.pos));
                }
                self.count(1, at)?;
                return Ok((key, at));
            }
            Some(b'"' | b'\'') => {
                let (key, multiline) = self.quoted(-1)?;
                if multiline {
                    return Err(self.error(ErrorKind::Syntax("an implicit key on one line"), at));
                }
                (key, false)
            }
            Some(b'[' | b'{') if B::LEXICAL => {
                let value = self.flow(properties, -1)?;
                if !self.take_value_indicator() {
                    return Err(self.error(ErrorKind::Syntax("`:` after a mapping key"), self.pos));
                }
                return Ok((
                    self.builder
                        .key_of(value, true, at)
                        .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                        .expect("a lexical key"),
                    at,
                ));
            }
            Some(b'[' | b'{') => return Err(self.error(ErrorKind::NonStringKey, at)),
            Some(b'|' | b'>') => {
                return Err(self.error(
                    ErrorKind::Syntax("a mapping key; a block scalar is not one"),
                    at,
                ));
            }
            _ if self.at_indicator(b'-') => {
                return Err(self.error(
                    ErrorKind::Syntax("a mapping key at a mapping's indentation"),
                    at,
                ));
            }
            _ => (self.owned_plain_line()?, true),
        };
        let (anchor, tag) = self.split(properties, at, false)?;
        let copied = admit!(self, self.builder.allocations().string(&text));
        let key = self.key_text(copied, plain, tag, at)?;
        if !self.take_value_indicator() {
            return Err(self.error(ErrorKind::Syntax("`:` after a mapping key"), self.pos));
        }
        self.count(1, at)?;
        if let Some(name) = anchor {
            let node = self.key_node(&key, text, plain, tag, at)?;
            self.anchor(name, node)?;
        } else {
            self.builder
                .discard_text(text)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        Ok((key, at))
    }

    /// The node at the cursor, which fills the innermost open slot (owned by
    /// a block indented `owner`).
    fn node(&mut self, owner: isize, line: &mut Line) -> Result<(), Error> {
        let at = self.pos;
        let column = self.column(at);
        match self.bytes[at] {
            b'&' | b'!' => self.add_property()?,
            b'*' => {
                // Properties of an earlier line belong to the mapping this
                // alias keys; properties on its own line would be the alias's.
                let own_line =
                    self.pending
                        .as_ref()
                        .or(self.later.as_ref())
                        .is_some_and(|properties| {
                            self.line_start(properties.at) == self.line_start(at)
                        });
                if own_line || self.later.is_some() {
                    return Err(self.error(ErrorKind::Syntax("an alias without properties"), at));
                }
                let value = self.alias()?;
                if self.take_value_indicator() {
                    let Some(key) = self
                        .builder
                        .key_of(value, self.limits.scalar_keys, at)
                        .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                    else {
                        return Err(self.error(ErrorKind::NonStringKey, at));
                    };
                    self.open_mapping(column, KeyInput::Ready(key), at, *line)?;
                    *line = Line::Inline;
                } else if self.pending.is_some() {
                    return Err(self.error(ErrorKind::Syntax("an alias without properties"), at));
                } else {
                    self.fill(value, at)?;
                }
            }
            b'-' | b'?' if self.blank_at(at + 1) => {
                if *line == Line::Inline {
                    return Err(self.error(
                        ErrorKind::Syntax(
                            "a line break before a block collection (none may start on this line)",
                        ),
                        at,
                    ));
                }
                self.check_indentation(at)?;
                if self.tab_after_indicator(at) {
                    return Err(self.error(ErrorKind::Tab, at));
                }
                let sequence = self.bytes[at] == b'-';
                if !sequence {
                    self.builder
                        .question(Span::new(at, at + 1))
                        .map_err(|(kind, at)| self.error(kind, at))?;
                }
                let properties = self.take_props()?;
                if properties
                    .as_ref()
                    .is_some_and(|p| self.line_start(p.at) == self.line_start(at))
                {
                    return Err(self.error(
                        ErrorKind::Syntax("a line break between properties and a block collection"),
                        at,
                    ));
                }
                let (anchor, tag) = self.split(properties, at, true)?;
                self.collection_tag(tag, if sequence { Tag::Seq } else { Tag::Map })?;
                self.enter(at)?;
                let frame = if sequence {
                    Frame::Sequence {
                        indent: column,
                        start: at,
                        items: Vec::new(),
                        open: true,
                        anchor,
                    }
                } else {
                    Frame::Mapping {
                        indent: column,
                        start: at,
                        object: self.builder.new_map(),
                        slot: Slot::ExplicitKey,
                        anchor,
                    }
                };
                admit!(
                    self,
                    self.builder.allocations().push(&mut self.stack, frame)
                );
                self.pos += 1;
                *line = Line::Compact;
            }
            b':' if self.blank_at(at + 1) => {
                // An empty key is a key only with properties on its line (`!!null : a`)
                // and only where scalar keys are accepted.
                let line_start = self.line_start(at);
                let propertied = self
                    .pending
                    .iter()
                    .chain(self.later.iter())
                    .any(|properties| self.line_start(properties.at) == line_start);
                if !(B::LEXICAL || self.limits.scalar_keys && propertied) {
                    return Err(self.error(ErrorKind::NonStringKey, at));
                }
                self.pos += 1;
                self.open_mapping(column, KeyInput::Scalar(String::new(), true), at, *line)?;
                *line = Line::Inline;
            }
            b'[' | b'{' if B::LEXICAL => {
                // The flow grammar produces the actual collection first.
                // A following colon decides whether preceding properties
                // belong to this key or to its enclosing block mapping.
                let value = self.flow(None, owner)?;
                if self.take_value_indicator() {
                    let key = self
                        .builder
                        .key_of(value, true, at)
                        .map_err(|(kind, at)| Error::new(self.text, kind, at))?
                        .expect("a lexical key");
                    self.open_mapping(column, KeyInput::Ready(key), at, *line)?;
                    *line = Line::Inline;
                } else {
                    let properties = self.take_props()?;
                    let spans = properties.as_ref().map(|properties| properties.spans);
                    let (anchor, _) = self.split(properties, at, true)?;
                    if let Some(spans) = spans {
                        self.builder
                            .decorate(&value, spans)
                            .map_err(|(kind, at)| self.error(kind, at))?;
                    }
                    self.register(anchor, &value)?;
                    self.fill(value, at)?;
                }
            }
            b'[' | b'{' => {
                // Two sets of properties before a flow collection are a
                // mapping's and an implicit key's; JSON holds no such key.
                let properties = match self.take_props() {
                    Err(error) if matches!(error.kind, ErrorKind::Syntax(what) if what.starts_with("one ")) =>
                    {
                        return Err(self.error(ErrorKind::NonStringKey, at));
                    }
                    other => other?,
                };
                let value = self.flow(properties, owner)?;
                if self.take_value_indicator() {
                    return Err(self.error(ErrorKind::NonStringKey, at));
                }
                self.fill(value, at)?;
            }
            b'|' | b'>' => {
                let text = self.block_scalar(owner)?;
                let properties = self.take_props()?;
                self.fill_scalar(text, false, properties, at)?;
                *line = Line::Fresh;
            }
            b',' | b']' | b'}' | b'%' | b'@' | b'`' => {
                return Err(self.error(
                    ErrorKind::Syntax("a node; a plain scalar cannot start with this indicator"),
                    at,
                ));
            }
            quote => {
                let (text, plain, multiline) = if matches!(quote, b'"' | b'\'') {
                    let (text, multiline) = self.quoted(owner)?;
                    (text, false, multiline)
                } else {
                    (self.owned_plain_line()?, true, false)
                };
                if self.take_value_indicator() {
                    if multiline {
                        return Err(
                            self.error(ErrorKind::Syntax("an implicit key on one line"), at)
                        );
                    }
                    self.open_mapping(column, KeyInput::Scalar(text, plain), at, *line)?;
                    *line = Line::Inline;
                } else {
                    let text = if plain {
                        self.plain_rest(text, owner)?
                    } else {
                        text
                    };
                    let properties = self.take_props()?;
                    self.fill_scalar(text, plain, properties, at)?;
                }
            }
        }
        Ok(())
    }

    /// The next entry of the innermost block collection that continues at
    /// the cursor's column: `-` of a sequence, or a mapping's key, `?` or `:`.
    fn entry(&mut self, line: &mut Line) -> Result<(), Error> {
        let at = self.pos;
        self.check_indentation(at)?;
        let column = self.column(at);
        loop {
            match self.stack.last() {
                Some(Frame::Root(_)) | None => {
                    return Err(self.error(
                        ErrorKind::TrailingContent {
                            span: Span::new(at, self.text.len()),
                            context: TrailingContext::Document,
                        },
                        at,
                    ));
                }
                Some(Frame::Sequence { indent, .. }) => {
                    if column == *indent && self.at_indicator(b'-') {
                        break;
                    }
                    if column <= *indent {
                        self.close_top()?;
                        continue;
                    }
                    return Err(self.error(
                        ErrorKind::Syntax("a sequence entry at its sequence's indentation"),
                        at,
                    ));
                }
                Some(Frame::Mapping { indent, .. }) => {
                    if column == *indent {
                        break;
                    }
                    if column < *indent {
                        self.close_top()?;
                        continue;
                    }
                    return Err(self.error(
                        ErrorKind::Syntax("a mapping entry at its mapping's indentation"),
                        at,
                    ));
                }
            }
        }
        let explicit = self.at_indicator(b'?');
        let value = self.at_indicator(b':');
        let pending_key = match self.stack.last_mut() {
            Some(Frame::Sequence { open, .. }) => {
                self.pos += 1;
                *open = true;
                *line = Line::Compact;
                return Ok(());
            }
            Some(Frame::Mapping { slot, .. }) => match mem::replace(slot, Slot::Idle) {
                Slot::KeyDone(key, key_at) if value => {
                    *slot = Slot::Value(key, key_at);
                    self.pos += 1;
                    *line = Line::Compact;
                    return Ok(());
                }
                Slot::KeyDone(key, key_at) => Some((key, key_at)),
                _ if value && !B::LEXICAL => return Err(self.error(ErrorKind::NonStringKey, at)),
                _ => None,
            },
            _ => unreachable!("a collection continues here"),
        };
        if let Some((key, key_at)) = pending_key {
            // An explicit key with no `:` line has a null value.
            let null = self
                .builder
                .null(at)
                .map_err(|(kind, at)| self.error(kind, at))?;
            let Some(Frame::Mapping { object, .. }) = self.stack.last_mut() else {
                unreachable!("the innermost collection is a mapping");
            };
            self.builder
                .insert(object, key, null, key_at)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        if explicit {
            let Some(Frame::Mapping { slot, .. }) = self.stack.last_mut() else {
                unreachable!("the innermost collection is a mapping");
            };
            *slot = Slot::ExplicitKey;
            self.builder
                .question(Span::new(self.pos, self.pos + 1))
                .map_err(|(kind, at)| Error::new(self.text, kind, at))?;
            self.pos += 1;
            *line = Line::Compact;
            return Ok(());
        }
        let (key, key_at) = self.block_key()?;
        let Some(Frame::Mapping { slot, .. }) = self.stack.last_mut() else {
            unreachable!("the innermost collection is a mapping");
        };
        *slot = Slot::Value(key, key_at);
        *line = Line::Inline;
        Ok(())
    }

    /// A directive line (§6.8), the cursor on its `%`: `%YAML` names one
    /// `1.x` version and appears once, `%TAG` names a handle (once) and a
    /// prefix, and any other directive is reserved and ignored.
    fn directive(
        &mut self,
        yaml_seen: &mut bool,
        handles: &mut OrderedTable<String, String>,
    ) -> Result<(), Error> {
        let at = self.pos;
        self.skip_to_line_end();
        self.builder
            .directive(Span::new(at, self.pos))
            .map_err(|(kind, at)| self.error(kind, at))?;
        let line = &self.text[at + 1..self.pos];
        // A comment starts at a `#` that white space precedes.
        let body = line
            .char_indices()
            .find(|&(index, c)| c == '#' && line[..index].ends_with([' ', '\t']))
            .map_or(line, |(index, _)| &line[..index]);
        if body.len() < line.len() {
            let comment = Span::new(at + 1 + body.len(), self.pos);
            self.builder
                .comment(comment, false)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        let mut words = body.split([' ', '\t']).filter(|word| !word.is_empty());
        let text = self.text;
        let refuse = |what: &'static str| Error::new(text, ErrorKind::Syntax(what), at);
        match words.next() {
            None => Err(refuse("a directive name after `%`")),
            Some("YAML") => {
                let version = words.next().ok_or_else(|| refuse("a `%YAML` version"))?;
                let digits =
                    |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
                let supported = version
                    .split_once('.')
                    .is_some_and(|(major, minor)| digits(major) && digits(minor) && major == "1");
                if !supported || words.next().is_some() {
                    return Err(refuse("one `%YAML` version, `1.` and a minor number"));
                }
                if mem::replace(yaml_seen, true) {
                    return Err(refuse("one `%YAML` directive per document"));
                }
                Ok(())
            }
            Some("TAG") => {
                let (Some(handle), Some(prefix), None) = (words.next(), words.next(), words.next())
                else {
                    return Err(refuse("a `%TAG` handle and a prefix"));
                };
                if !tag_handle(handle) {
                    return Err(refuse("a tag handle: `!`, `!!` or `!name!`"));
                }
                if !tag_uri(prefix, false) || is_flow_indicator(prefix.as_bytes().first().copied())
                {
                    return Err(refuse("a URI tag prefix with valid percent escapes"));
                }
                let handle = admit!(self, self.builder.allocations().string(handle));
                if handles.get(&handle).is_some() {
                    return Err(refuse("one `%TAG` directive per handle"));
                }
                let prefix = admit!(self, self.builder.allocations().string(prefix));
                let prefix = self.decode_tag(prefix, 0, at)?;
                admit!(self, self.builder.allocations().reserve_index(handles));
                if admit!(
                    self,
                    self.builder
                        .allocations()
                        .memory(|memory| handles.insert(handle, prefix, memory))
                )
                .is_some()
                {
                    return Err(refuse("one `%TAG` directive per handle"));
                }
                Ok(())
            }
            Some(_) => Ok(()),
        }
    }

    fn take_bom(&mut self) {
        self.bom_line = Some(self.pos);
        self.pos += '\u{feff}'.len_utf8();
    }

    fn document(&mut self, stream: bool) -> Result<B::Node, Error> {
        if self.pos == 0 && self.text.starts_with('\u{feff}') {
            self.take_bom();
        }
        // The prologue: comments, directives, and the document start marker.
        let mut directives = false;
        let mut yaml_directive = false;
        let mut tag_handles: OrderedTable<String, String> = OrderedTable::default();
        let mut line = Line::Fresh;
        loop {
            self.skip_separation()?;
            if self.pos < self.bytes.len()
                && self.peek() == Some(b'%')
                && self.pos == self.line_start(self.pos)
            {
                directives = true;
                self.directive(&mut yaml_directive, &mut tag_handles)?;
                continue;
            }
            if self.at_marker() && self.bytes[self.pos] == b'-' {
                self.document_start = Some(Span::new(self.pos, self.pos + 3));
                self.prologue_end = self.pos;
                self.pos += 3;
                line = Line::Inline;
            } else if directives {
                return Err(self.error(ErrorKind::Syntax("`---` after the directives"), self.pos));
            } else {
                self.prologue_end = self.pos;
            }
            break;
        }
        self.handles = tag_handles;
        loop {
            if self.skip_separation()? {
                line = Line::Fresh;
            }
            if self.pos >= self.bytes.len() || self.at_marker() {
                break;
            }
            if let Some((owner, mapping_value)) = self.open_slot() {
                let column = self.column(self.pos) as isize;
                let here = line != Line::Fresh
                    || column > owner
                    || (mapping_value && column == owner && self.at_indicator(b'-'));
                if here {
                    self.node(owner, &mut line)?;
                } else {
                    let properties = self.take_props()?;
                    self.fill_scalar(String::new(), true, properties, self.pos)?;
                }
            } else {
                if line != Line::Fresh {
                    return Err(self.error(
                        ErrorKind::TrailingContent {
                            span: Span::new(self.pos, self.text.len()),
                            context: TrailingContext::Node,
                        },
                        self.pos,
                    ));
                }
                self.entry(&mut line)?;
            }
        }
        // Close every open collection; an open slot holds null.
        let end = self.pos;
        loop {
            if self.open_slot().is_some() {
                let properties = self.take_props()?;
                self.fill_scalar(String::new(), true, properties, end)?;
            }
            if self.stack.len() == 1 {
                break;
            }
            self.close_top()?;
        }
        if !stream {
            // Preserve the single-document refusal before publishing its root.
            if self.pos < self.bytes.len()
                && self.bytes[self.pos..].starts_with(b"...")
                && self.at_marker()
            {
                self.pos += 3;
                self.skip_separation()?;
            }
            if self.pos < self.bytes.len() {
                return Err(self.error(ErrorKind::MultipleDocuments, self.pos));
            }
        }
        match self.stack.pop() {
            Some(Frame::Root(Some(value))) => Ok(value),
            Some(Frame::Root(None)) => self.null(self.pos),
            _ => unreachable!("only the root frame remains"),
        }
    }

    fn end_marker(&mut self) -> Result<(), Error> {
        self.pos += 3;
        self.skip_inline_space();
        if !matches!(self.peek(), None | Some(b'\n' | b'\r' | b'#')) {
            return Err(self.error(
                ErrorKind::TrailingContent {
                    span: Span::new(self.pos, self.text.len()),
                    context: TrailingContext::EndMarker,
                },
                self.pos,
            ));
        }
        self.skip_separation()?;
        Ok(())
    }

    // Stream material is not itself an empty document. Explicit `---` is.
    fn stream_prefix(&mut self) -> Result<bool, Error> {
        loop {
            self.skip_separation()?;
            if (self.pos == 0 || matches!(self.bytes.get(self.pos - 1), Some(b'\r' | b'\n')))
                && self.text[self.pos..].starts_with('\u{feff}')
            {
                self.take_bom();
            } else if self.at_marker() && self.peek() == Some(b'.') {
                self.end_marker()?;
            } else {
                return Ok(self.pos < self.bytes.len());
            }
        }
    }

    fn clear_document(&mut self) -> Result<(), Error> {
        while let Some((name, binding)) = self.anchors.pop_first() {
            if let Some((value, _)) = binding.ready {
                self.builder
                    .discard_node(value)
                    .map_err(|(kind, at)| self.error(kind, at))?;
            }
            self.builder
                .discard_text(name)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        while let Some((name, prefix)) = self.handles.pop_first() {
            self.builder
                .discard_text(name)
                .map_err(|(kind, at)| self.error(kind, at))?;
            self.builder
                .discard_text(prefix)
                .map_err(|(kind, at)| self.error(kind, at))?;
        }
        let anchors = mem::take(&mut self.anchors);
        let handles = mem::take(&mut self.handles);
        admit!(
            self,
            self.builder.allocations().memory(|memory| {
                anchors.release(memory)?;
                handles.release(memory)
            })
        );
        self.last_scalar_end = self.pos;
        Ok(())
    }
}

/// Read `text` as one YAML 1.2 document under [`Limits::DEFAULT`].
///
/// ```rust
/// use purrdf_lex::yaml::{self, ErrorKind};
///
/// let value = yaml::read("name: x  # a comment\nlist:\n- 1.50\n- 'true'\n- {a: [b, c]}\ntext: |\n  one\n  two\n").unwrap();
/// assert_eq!(value["list"][0].as_number().unwrap().lexeme(), "1.50");
/// assert_eq!(value["list"][1], "true");
/// assert_eq!(value["list"][2]["a"][1], "c");
/// assert_eq!(value["text"], "one\ntwo\n");
/// assert!(matches!(yaml::read("a: 1\na: 2").unwrap_err().kind(), ErrorKind::DuplicateKey { .. }));
/// ```
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read(text: &str) -> Result<Value, Error> {
    read_with(text, Limits::DEFAULT)
}

/// Read `text` as one YAML 1.2 document under `limits`.
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read_with(text: &str, limits: Limits) -> Result<Value, Error> {
    parse(
        text,
        limits,
        CoreBuilder::new(limits, super::admission::Allocations::legacy()),
    )
    .map(|(node, _)| node.value)
}

/// Construct every document using the existing reader's scalar, tag, mapping
/// and alias rules, in source order, under one allocation and node account.
///
/// Empty/comment-only streams and standalone end markers contain no documents;
/// `---` with an omitted root constructs a null document. Anchor bindings and
/// directives are local to each document. Original offsets remain stream-wide.
/// No documents are returned unless the entire stream succeeds.
///
/// # Errors
///
/// [`Error`] at the original absolute position of a syntax, semantic or resource
/// refusal, including refusals in later documents. Single-document [`read_with`]
/// retains its existing `MultipleDocuments` policy and error precedence.
pub fn read_stream(text: &str, limits: StreamLimits) -> Result<Vec<Value>, Error> {
    input_limit(text, limits.max_input_bytes)?;
    let builder = CoreBuilder::new(
        limits.core,
        super::admission::Allocations::bounded(limits.max_allocation_bytes),
    );
    parse_stream(
        text,
        limits.core,
        builder,
        limits.max_documents,
        |node, _| node.value,
    )
    .map(|(documents, _)| documents)
}

/// Shared stream driver; each retained record is admitted beside its builder's
/// occurrences or Core products, without publishing a partial vector.
pub(super) struct DocumentBoundary {
    pub(super) span: Span,
    pub(super) prologue: Span,
    pub(super) start_marker: Option<Span>,
    pub(super) end_marker: Option<Span>,
}

pub(super) fn parse_stream<B: Builder, R>(
    text: &str,
    limits: Limits,
    builder: B,
    max_documents: usize,
    mut record: impl FnMut(B::Node, DocumentBoundary) -> R,
) -> Result<(Vec<R>, B), Error> {
    let mut parser = parser(text, limits, builder);
    let mut documents = Vec::new();
    loop {
        let prefix = parser.pos;
        if !parser.stream_prefix()? {
            break;
        }
        if documents.len() >= max_documents {
            return Err(parser.error(
                ErrorKind::Documents {
                    limit: max_documents,
                },
                parser.pos,
            ));
        }
        admit!(
            parser,
            parser
                .builder
                .allocations()
                .push(&mut parser.stack, Frame::Root(None))
        );
        parser.document_start = None;
        let node = parser.document(true)?;
        // Preserve Core stream refusal order: admit the retained result slot
        // and release document-local state before examining its end marker.
        admit!(
            parser,
            parser
                .builder
                .allocations()
                .memory(|memory| memory.reserve_for_push(&mut documents))
        );
        parser.clear_document()?;
        let end_marker = if parser.at_marker() && parser.peek() == Some(b'.') {
            let marker = Span::new(parser.pos, parser.pos + 3);
            parser.end_marker()?;
            Some(marker)
        } else {
            None
        };
        let boundaries = DocumentBoundary {
            span: Span::new(prefix, parser.pos),
            prologue: Span::new(prefix, parser.prologue_end),
            start_marker: parser.document_start,
            end_marker,
        };
        documents.push(record(node, boundaries));
    }
    admit!(
        parser,
        parser.builder.allocations().discard_vec(parser.stack)
    );
    Ok((documents, parser.builder))
}

fn input_limit(text: &str, limit: usize) -> Result<(), Error> {
    if text.len() > limit {
        return Err(Error::new(text, ErrorKind::Input { limit }, 0));
    }
    Ok(())
}

/// Run the one grammar with an admitted product builder.
pub(super) fn project(text: &str, limits: super::syntax::ProjectionLimits) -> Result<Value, Error> {
    input_limit(text, limits.max_input_bytes)?;
    parse(
        text,
        limits.core,
        CoreBuilder::new(
            limits.core,
            super::admission::Allocations::bounded(limits.max_allocation_bytes),
        ),
    )
    .map(|(node, _)| node.value)
}

/// Run the one grammar with an admitted product builder.
pub(super) fn parse<B: Builder>(
    text: &str,
    limits: Limits,
    builder: B,
) -> Result<(B::Node, B), Error> {
    let mut parser = parser(text, limits, builder);
    admit!(
        parser,
        parser
            .builder
            .allocations()
            .push(&mut parser.stack, Frame::Root(None))
    );
    let node = parser.document(false)?;
    parser.clear_document()?;
    admit!(
        parser,
        parser.builder.allocations().discard_vec(parser.stack)
    );
    Ok((node, parser.builder))
}

fn parser<B: Builder>(text: &str, limits: Limits, builder: B) -> Parser<'_, B> {
    Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        limits,
        nodes: 0,
        depth: 0,
        anchors: OrderedTable::default(),
        handles: OrderedTable::default(),
        stack: Vec::new(),
        pending: None,
        later: None,
        located: Cell::new((0, 0, 0)),
        bom_line: None,
        document_start: None,
        prologue_end: 0,
        builder,
        last_scalar_end: 0,
    }
}

pub(super) fn position(text: &str, at: usize) -> (usize, usize) {
    let (mut line, mut column, mut cr) = (1, 1, false);
    for ch in text[..at].chars() {
        match ch {
            '\r' => {
                line += 1;
                column = 1;
                cr = true;
            }
            '\n' => {
                if !cr {
                    line += 1;
                }
                column = 1;
                cr = false;
            }
            _ => {
                column += 1;
                cr = false;
            }
        }
    }
    (line, column)
}

/// YAML 1.2 §6.8.2.1 c-tag-handle; the primary and secondary handles are
/// predefined, and a named handle has an ASCII word between its bangs.
fn tag_handle(handle: &str) -> bool {
    handle == "!"
        || handle == "!!"
        || (handle.len() > 2
            && handle.starts_with('!')
            && handle.ends_with('!')
            && handle[1..handle.len() - 1]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
}

/// YAML 1.2 §5.6 ns-uri-char and ns-tag-char. Percent escapes consume exactly
/// two hexadecimal digits; shorthand suffixes exclude bangs and flow indicators.
fn tag_uri(text: &str, shorthand: bool) -> bool {
    if text.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'%' {
            if bytes
                .get(cursor + 1)
                .and_then(|&byte| purrdf_hash::hex::nibble(byte))
                .is_none()
                || bytes
                    .get(cursor + 2)
                    .and_then(|&byte| purrdf_hash::hex::nibble(byte))
                    .is_none()
            {
                return false;
            }
            cursor += 3;
        } else {
            let uri = byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'-' | b'#'
                        | b';'
                        | b'/'
                        | b'?'
                        | b':'
                        | b'@'
                        | b'&'
                        | b'='
                        | b'+'
                        | b'$'
                        | b','
                        | b'_'
                        | b'.'
                        | b'!'
                        | b'~'
                        | b'*'
                        | b'\''
                        | b'('
                        | b')'
                        | b'['
                        | b']'
                );
            if !uri || (shorthand && (byte == b'!' || is_flow_indicator(Some(byte)))) {
                return false;
            }
            cursor += 1;
        }
    }
    true
}

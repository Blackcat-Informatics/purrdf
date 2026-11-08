// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Bounded source syntax shared by authoring consumers and the RDF slicer.

mod authoring;
mod block;
pub use authoring::{
    AuthoringComment, AuthoringDocument, AuthoringHeading, AuthoringLine, AuthoringLink,
    AuthoringLinkForm, authoring,
};
mod budget;
mod code_span;
pub use code_span::{CodeSpanLimits, replace_code_spans};
mod inline;
use crate::Span;
use budget::Budget;
use std::borrow::Cow;
use std::ops::Range;

/// The implementation-owned grammar revision, bound into the slicing identity.
pub const DIALECT_REVISION: u32 = 2;
/// Caller-selected inclusive ceilings; these are admission policy, not grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxLimits {
    /// Original document UTF-8 bytes.
    pub source_bytes: usize,
    /// Live native storage, including construction scratch and decoded text.
    pub auxiliary_bytes: usize,
    /// Deterministic native source-scan, comparison and copy work units.
    pub work: u64,
    /// Open quote/list containers.
    pub containers: usize,
    /// Open inline brackets and destination parentheses.
    pub nesting: usize,
    /// Raw and decoded UTF-8 bytes of each destination or title, including HTML href/src values.
    pub link_part_bytes: usize,
}
impl SyntaxLimits {
    /// Preserve the original allocator-only admission policy for existing slicers.
    pub const fn unbounded() -> Self {
        Self {
            source_bytes: usize::MAX,
            auxiliary_bytes: usize::MAX,
            work: u64::MAX,
            containers: usize::MAX,
            nesting: usize::MAX,
            link_part_bytes: usize::MAX,
        }
    }
}
/// A zero-based byte position and one-based line and Unicode-scalar column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    /// Byte offset.
    pub byte: usize,
    /// Physical line.
    pub line: usize,
    /// Unicode scalar column; tabs count as one source scalar.
    pub column: usize,
}
/// Original typed syntax/resource refusal; no partial model is published.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyntaxError {
    /// An inclusive caller resource ceiling was exceeded.
    Limit {
        /// Original source position.
        position: SourcePosition,
        /// Resource name.
        resource: &'static str,
        /// Required amount.
        actual: u64,
        /// Inclusive caller ceiling.
        limit: u64,
    },
    /// Original shared HTML reference layout/capacity refusal.
    Reference {
        /// Original source position.
        position: SourcePosition,
        /// Unchanged native reference-reader cause.
        cause: purrdf_lex::html::ResolveIntoError,
    },
    /// Original physical allocation/admission/layout refusal.
    Storage {
        /// Original source position.
        position: SourcePosition,
        /// Unchanged native cause.
        cause: purrdf_lex::allocation::StorageError,
    },
}
impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limit {
                position,
                resource,
                actual,
                limit,
            } => write!(
                f,
                "Markdown {}:{} (byte {}): {resource} requires {actual}, limit {limit}",
                position.line, position.column, position.byte
            ),
            Self::Reference { position, cause } => write!(
                f,
                "Markdown {}:{} (byte {}): {cause}",
                position.line, position.column, position.byte
            ),
            Self::Storage { position, cause } => write!(
                f,
                "Markdown {}:{} (byte {}): {cause}",
                position.line, position.column, position.byte
            ),
        }
    }
}
impl std::error::Error for SyntaxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage { cause, .. } => Some(cause),
            Self::Reference { cause, .. } => Some(cause),
            _ => None,
        }
    }
}
impl purrdf_hash::error::StructuredError for SyntaxError {
    purrdf_hash::error_visit! {
        Limit { position, resource, actual, limit } => "markdown-syntax-limit" {
            "byte" => ByteCount(position.byte as u128), "line" => Unsigned(position.line as u128), "column" => Unsigned(position.column as u128),
            "resource" => Code(*resource), "actual" => Unsigned(u128::from(*actual)), "limit" => Unsigned(u128::from(*limit)),
        },
        Reference { position, cause } => "markdown-syntax-reference" {
            "byte" => ByteCount(position.byte as u128), "line" => Unsigned(position.line as u128), "column" => Unsigned(position.column as u128), "cause" => Cause(cause),
        },
        Storage { position, cause } => "markdown-syntax-storage" {
            "byte" => ByteCount(position.byte as u128), "line" => Unsigned(position.line as u128), "column" => Unsigned(position.column as u128), "cause" => Cause(cause),
        },
    }
}
/// One original physical line; terminators are never normalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentLine {
    /// Entire physical line payload.
    pub source: Span,
    /// Content after container prefixes and code indentation.
    pub content: Span,
    /// Residual virtual spaces from a source tab partly consumed by a container
    /// or code indentation. Prepend these columns when rendering content; source
    /// bytes and ranges remain unchanged.
    pub virtual_spaces: usize,
    /// Original LF, CRLF, CR, or empty EOF terminator.
    pub newline: Span,
}
/// Native block classification. Syntax spelling stays in source spans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// Block quote container.
    Quote,
    /// List item container; marker and optional ordered number are preserved.
    ListItem {
        /// Original marker.
        marker: Span,
        /// Native ordered value, when ordered.
        number: Option<u64>,
    },
    /// ATX or Setext heading.
    Heading {
        /// Heading depth.
        level: u32,
        /// Raw title content.
        text: Span,
    },
    /// Paragraph with inline syntax.
    Paragraph,
    /// Fenced code, including the original trimmed info-string span.
    FencedCode {
        /// Opening fence run.
        marker: Span,
        /// Original info string.
        info: Span,
        /// Whether a matching closer was present.
        closed: bool,
    },
    /// Indented code.
    IndentedCode,
    /// HTML block, including comments; unfinished blocks retain their extent.
    Html {
        /// Comment block rather than a tag/declaration block.
        comment: bool,
        /// Raw-text/comment/declaration block; its body is opaque to tag recognition.
        opaque: bool,
        /// Explicit closer found, or blank-line block terminated.
        closed: bool,
    },
    /// Thematic break.
    Rule,
    /// Link reference definition.
    ReferenceDefinitions {
        /// Consecutive definition indexes covering this block.
        definitions: Range<usize>,
    },
}
/// A flat arena block; parents always precede children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// Original physical-line envelope, including the final original line ending.
    pub source: Span,
    /// Parent container ID.
    pub parent: Option<usize>,
    /// Classification.
    pub kind: BlockKind,
    /// Content-line indexes in the document arena.
    pub lines: Range<usize>,
}
impl Block {
    fn content_range(&self) -> Range<usize> {
        match self.kind {
            BlockKind::FencedCode { closed, .. } => {
                self.lines.start + 1..self.lines.end - usize::from(closed)
            }
            BlockKind::Heading { .. } if self.lines.len() > 1 => {
                self.lines.start..self.lines.end - 1
            }
            _ => self.lines.clone(),
        }
    }
}
/// A link component retains spelling alongside native-decoded text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkPart<'a> {
    /// Original component envelope, without its surrounding delimiters.
    /// Authoring comment removal and code replacement can leave holes inside it.
    pub source: Span,
    /// Interpreted value. Standard syntax decodes punctuation escapes and
    /// character references; authoring preserves references and raw HTML values.
    pub decoded: Cow<'a, str>,
}
/// A link/image occurrence, resolved natively when reference syntax is used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link<'a> {
    /// Whole original occurrence.
    pub source: Span,
    /// Visible label source.
    pub label: Span,
    /// Image occurrence.
    pub image: bool,
    /// Resolved destination.
    pub destination: LinkPart<'a>,
    /// Resolved title.
    pub title: Option<LinkPart<'a>>,
    /// Definition index, for reference uses.
    pub definition: Option<usize>,
}
/// Reference definition, including duplicates, with an exact original source span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferenceDefinition<'a> {
    /// Whole original definition.
    pub source: Span,
    /// Original label spelling and decoded label.
    pub label: LinkPart<'a>,
    /// Full Unicode fold of raw spelling, collapsing only space, tab and line endings.
    pub key: String,
    /// Destination.
    pub destination: LinkPart<'a>,
    /// Optional title.
    pub title: Option<LinkPart<'a>>,
}
/// Syntax excluded from prose, including inline HTML tag attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineKind {
    /// Backtick code span.
    Code,
    /// HTML comment.
    Comment,
    /// Native tag or opaque processing instruction, declaration or CDATA form.
    /// Only tags carry source-exact href/src attributes.
    Html {
        /// Native-decoded href/src occurrences in attribute order.
        attributes: Range<usize>,
    },
}
/// One source-exact inline opaque occurrence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inline {
    /// Source spelling.
    pub source: Span,
    /// Native classification.
    pub kind: InlineKind,
}
/// An HTML href/src occurrence; name spelling is preserved by its span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlAttribute<'a> {
    /// Original attribute name.
    pub name: Span,
    /// Raw and decoded attribute value.
    pub value: LinkPart<'a>,
}
/// Fully admitted borrowed source syntax; construction publishes atomically.
#[derive(Debug)]
pub struct SyntaxDocument<'a> {
    policy: Policy,
    source: &'a str,
    pub(crate) blocks: Vec<Block>,
    pub(crate) lines: Vec<ContentLine>,
    pub(crate) links: Vec<Link<'a>>,
    pub(crate) definitions: Vec<ReferenceDefinition<'a>>,
    pub(crate) inlines: Vec<Inline>,
    pub(crate) attributes: Vec<HtmlAttribute<'a>>,
    pub(crate) prose: Vec<Span>,
    pub(crate) sliced: Vec<SliceLine>,
    /// Retained native storage bytes, excluding the caller's source.
    pub auxiliary_bytes: usize,
    /// Native work consumed while constructing this model.
    pub work: u64,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct SliceLine {
    pub source: Span,
    pub content: Span,
    pub heading: Option<(u32, Span)>,
    pub eligible: bool,
}
impl<'a> SyntaxDocument<'a> {
    /// Original borrowed UTF-8 source.
    pub const fn source(&self) -> &'a str {
        self.source
    }
    /// Exact spelling of a source span produced by this model.
    pub fn spelling(&self, span: Span) -> &'a str {
        &self.source[span.range()]
    }
    /// Blocks in native opening order, with container parents.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
    /// Original content lines; code-block line indexes refer here.
    pub fn lines(&self) -> &[ContentLine] {
        &self.lines
    }
    /// Decontainerized content lines of a block. Fenced-code delimiters are
    /// excluded, including their newlines. Setext underlines are excluded;
    /// all actual title lines and blank code content lines remain present.
    pub fn content_lines(&self, block: &Block) -> &[ContentLine] {
        let range = block.content_range();
        &self.lines[range]
    }
    /// Links and images, including native-resolved reference uses.
    pub fn links(&self) -> &[Link<'a>] {
        &self.links
    }
    /// Reference definitions in deterministic arena order, with stable indexes.
    /// Sort by `source.start` when source order is needed. The matching definition
    /// with the earliest original source position wins, regardless of arena order.
    pub fn definitions(&self) -> &[ReferenceDefinition<'a>] {
        &self.definitions
    }
    /// Inline code, comments, tags and opaque HTML forms.
    pub fn inlines(&self) -> &[Inline] {
        &self.inlines
    }
    /// href/src attributes over native HTML tags.
    pub fn html_attributes(&self) -> &[HtmlAttribute<'a>] {
        &self.attributes
    }
    /// Prose runs excluding block code, inline code, HTML forms and comments.
    pub fn prose(&self) -> &[Span] {
        &self.prose
    }
    /// Source position at an exact UTF-8 boundary, or None outside the source.
    pub fn position(&self, byte: usize) -> Option<SourcePosition> {
        self.source
            .is_char_boundary(byte)
            .then(|| position(self.source, byte, self.policy))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    Standard,
    Authoring,
}
// An initial encoding marker affects recognition, never original coordinates.
fn payload_start(source: &str, start: usize) -> usize {
    if start == 0 && source.starts_with('\u{feff}') {
        3
    } else {
        start
    }
}
fn span(start: usize, end: usize) -> Span {
    Span::new(start as u64, end as u64)
}
fn position(text: &str, byte: usize, policy: Policy) -> SourcePosition {
    let mut line = 1;
    let mut from = 0;
    let mut at = 0;
    let b = text.as_bytes();
    while at < byte {
        if policy == Policy::Standard && b[at] == b'\r' {
            line += 1;
            at += 1;
            if at < byte && b[at] == b'\n' {
                at += 1;
            }
            from = at;
        } else if b[at] == b'\n' {
            line += 1;
            at += 1;
            from = at;
        } else {
            at += 1;
        }
    }
    let mut boundary = byte;
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    SourcePosition {
        byte,
        line,
        column: text[from..boundary].chars().count() + 1,
    }
}

/// Parse once into the shared syntax model, refusing all output on resource failure.
/// No file access, normalization or downstream policy is performed.
pub fn syntax(source: &str, limits: SyntaxLimits) -> Result<SyntaxDocument<'_>, SyntaxError> {
    let mut budget = Budget::new(source, limits, Policy::Standard);
    budget.limit(0, "source bytes", source.len(), limits.source_bytes)?;
    budget.tick(0, source.len())?;
    let mut document = SyntaxDocument {
        policy: Policy::Standard,
        source,
        blocks: Vec::new(),
        lines: Vec::new(),
        links: Vec::new(),
        definitions: Vec::new(),
        inlines: Vec::new(),
        attributes: Vec::new(),
        prose: Vec::new(),
        sliced: Vec::new(),
        auxiliary_bytes: 0,
        work: 0,
    };
    block::read(&mut document, &mut budget)?;
    inline::read(&mut document, &mut budget)?;
    document.auxiliary_bytes = budget.live;
    document.work = budget.work;
    Ok(document)
}

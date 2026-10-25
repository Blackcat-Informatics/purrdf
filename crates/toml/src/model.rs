// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use purrdf_core::ContentDigest;
use std::ops::Range;

/// Explicitly named original UTF-8 source.
#[derive(Clone, Copy, Debug)]
pub struct SourceDocument<'a> {
    /// Absolute source IRI.
    pub id: &'a str,
    /// Original bytes, including comments and line endings.
    pub bytes: &'a [u8],
}

/// A decoded key and the exact quoted/bare token that authored it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    /// Unicode key identity, without quoting or escapes.
    pub decoded: String,
    /// Exact source token byte range.
    pub span: Range<usize>,
}

/// TOML floating-point values preserve infinity and signed NaN distinctly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Float {
    /// IEEE 754 double precision, including signed zero.
    Finite(f64),
    /// Infinity with its authored sign.
    Infinity {
        /// Whether the sign is negative.
        negative: bool,
    },
    /// Not-a-number with its authored sign.
    Nan {
        /// Whether the sign is negative.
        negative: bool,
    },
}

/// Typed scalar semantics. Temporal values retain their complete precision and
/// local/offset distinction; normalized spelling supplies omitted seconds.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    /// Decoded Unicode text.
    String(String),
    /// Signed 64-bit integer.
    Integer(i64),
    /// Floating-point value, never coerced to JSON or text.
    Float(Float),
    /// Boolean.
    Boolean(bool),
    /// Validated offset date and time.
    OffsetDateTime(String),
    /// Validated local date and time, without a fabricated timezone.
    LocalDateTime(String),
    /// Validated local date.
    LocalDate(String),
    /// Validated local time, without a fabricated date or timezone.
    LocalTime(String),
}

/// How a table entered the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableOrigin {
    /// Unnamed document root.
    Root,
    /// Parent implied by a table header.
    Implicit,
    /// An explicit single-bracket declaration.
    Explicit,
    /// A table defined by a dotted assignment.
    Dotted,
    /// Curly-brace table, closed against later external extension.
    Inline,
    /// One element introduced by a double-bracket declaration.
    ArrayElement,
}

/// Semantic node kind. Containers hold only arena indices, so drop is flat.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// Scalar semantics with separate source spelling.
    Scalar(Scalar),
    /// Table declaration category.
    Table(TableOrigin),
    /// Ordinary array.
    Array,
    /// Array whose elements are declared by double-bracket headers.
    ArrayOfTables,
}

/// One stable occurrence in the flat document arena.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Kind and decoded scalar value.
    pub kind: Kind,
    /// First authored token/value range; implicit nodes cover their key token.
    pub span: Range<usize>,
    /// Key identity/token, absent for root and array elements.
    pub key: Option<Key>,
    /// Parent arena index; absent only for the root.
    pub parent: Option<usize>,
    /// Zero-based position among this parent's immediate children.
    pub ordinal: usize,
    /// Child indices in insertion order.
    pub children: Vec<usize>,
    /// Absolute escaped address; array elements include their ordinal.
    pub path: String,
    pub(crate) depth: u32,
    pub(crate) sealed: bool,
}

/// An authored declaration or assignment in source order. Promoting an implicit
/// table never invents a second semantic table or loses its later declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    /// Exact statement bytes, excluding trailing comment/newline.
    pub span: Range<usize>,
    /// Original key/header path tokens in spelling order.
    pub keys: Vec<Key>,
    /// Arena index of the assigned value or declared table element.
    pub node: usize,
    /// Whether this statement is a table header.
    pub header: bool,
}

/// Validated model; constructors are private so projection cannot trust forged
/// source ranges. Source text is borrowed and never rewritten.
#[derive(Clone, Debug)]
pub struct Document<'a> {
    pub(crate) source: SourceDocument<'a>,
    pub(crate) text: &'a str,
    pub(crate) nodes: Vec<Node>,
    pub(crate) statements: Vec<Statement>,
    pub(crate) keys: Vec<Key>,
    pub(crate) digest: ContentDigest,
    pub(crate) id: String,
    pub(crate) profile: ContentDigest,
}

impl<'a> Document<'a> {
    /// Content-addressed document IRI.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Original source descriptor.
    pub const fn source(&self) -> SourceDocument<'a> {
        self.source
    }
    /// Original bytes interpreted as UTF-8.
    pub const fn text(&self) -> &'a str {
        self.text
    }
    /// Semantic arena, rooted at index zero.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    /// Authored statements in exact source order.
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }
    /// Every authored key token in source order, including inline dotted keys.
    pub fn keys(&self) -> &[Key] {
        &self.keys
    }
    /// Maximal source runs between scalar verbatim spans, in byte order. Together
    /// with the scalar spans these form the complete, nonoverlapping byte cover.
    pub fn structure(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        let mut end = 0;
        self.nodes
            .iter()
            .filter_map(|node| matches!(node.kind, Kind::Scalar(_)).then_some(node.span.clone()))
            .chain(std::iter::once(self.text.len()..self.text.len()))
            .filter_map(move |span| {
                let gap = end..span.start;
                end = span.end;
                (!gap.is_empty()).then_some(gap)
            })
    }
    /// Shared native digest of the original bytes.
    pub const fn digest(&self) -> ContentDigest {
        self.digest
    }
}

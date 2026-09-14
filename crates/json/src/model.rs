// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::Range;

use purrdf_core::ContentDigest;

/// A caller-named UTF-8 JSON source. Its IRI is validated by [`crate::analyze`].
#[derive(Clone, Copy, Debug)]
pub struct SourceDocument<'a> {
    /// Absolute source IRI, independent of the derived document identity.
    pub id: &'a str,
    /// Original bytes, including leading and trailing whitespace.
    pub bytes: &'a [u8],
}

/// A JSON value's syntactic kind. No numeric conversion changes its spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A string; its covering span excludes the quote delimiters.
    String,
    /// A JSON number, preserved as a lexical string.
    Number,
    /// The literal `true`.
    True,
    /// The literal `false`.
    False,
    /// The literal `null`.
    Null,
    /// An ordered array, including an empty array.
    Array,
    /// An object retaining every member occurrence, including duplicate names.
    Object,
}

impl Kind {
    /// The stable name written in RDF metadata.
    pub const fn name(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::True => "true",
            Self::False => "false",
            Self::Null => "null",
            Self::Array => "array",
            Self::Object => "object",
        }
    }

    /// Whether this value owns lexical bytes in the cover.
    pub const fn is_scalar(self) -> bool {
        !matches!(self, Self::Array | Self::Object)
    }
}

/// One value occurrence, in preorder. Equal paths never collapse occurrences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    pub(crate) span: Range<usize>,
    pub(crate) kind: Kind,
    pub(crate) path: String,
    pub(crate) parent: Option<usize>,
    pub(crate) ordinal: usize,
    // The profile admits at most 1,048,576 values. A 32-bit count fits the
    // occurrence model without requiring a pointer-width count.
    pub(crate) size: u32,
}

impl Value {
    /// Lexical byte span; containers span their complete syntax but do not cover it.
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
    /// Syntactic JSON kind.
    pub const fn kind(&self) -> Kind {
        self.kind
    }
    /// RFC 6901 pointer with decoded member names. Duplicates may share a pointer.
    pub fn path(&self) -> &str {
        &self.path
    }
    /// Parent occurrence index, absent for the root value.
    pub const fn parent(&self) -> Option<usize> {
        self.parent
    }
    /// Immediate element/member count for a container, including duplicate keys.
    /// Scalars have no container size.
    pub const fn size(&self) -> Option<usize> {
        if self.kind.is_scalar() {
            None
        } else {
            Some(self.size as usize)
        }
    }
    /// Zero-based member or element position within the parent; zero for the root.
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }
}

/// A validated, immutable model borrowing its source. Construction is only through
/// [`crate::analyze`], so projection cannot accept fabricated offsets or paths.
#[derive(Clone, Debug)]
pub struct Document<'a> {
    pub(crate) source: SourceDocument<'a>,
    pub(crate) text: &'a str,
    pub(crate) values: Vec<Value>,
    pub(crate) runs: Vec<Range<usize>>,
    pub(crate) digest: ContentDigest,
    pub(crate) id: String,
    pub(crate) profile_id: ContentDigest,
}

impl<'a> Document<'a> {
    /// Content-addressed RDF document IRI.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The caller's original source descriptor.
    pub const fn source(&self) -> SourceDocument<'a> {
        self.source
    }
    /// Original JSON, without normalization or a second owned copy.
    pub const fn text(&self) -> &'a str {
        self.text
    }
    /// Every value occurrence, in document preorder.
    pub fn values(&self) -> &[Value] {
        &self.values
    }
    /// Maximal byte runs outside scalar lexical spans, in ascending order.
    pub fn structure(&self) -> &[Range<usize>] {
        &self.runs
    }
    /// SHA-256 digest of the original bytes.
    pub const fn digest(&self) -> ContentDigest {
        self.digest
    }
}

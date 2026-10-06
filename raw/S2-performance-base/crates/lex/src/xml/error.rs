// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The reader's typed refusals.

use core::fmt;

/// Why a document was refused. Every variant is a well-formedness or
/// namespace-well-formedness violation of the document, or a limit the caller
/// set; none is recoverable by reading on.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlErrorKind {
    /// A document type declaration under [`Dtd::Refuse`](super::Dtd::Refuse).
    Doctype,
    /// A reference to an external general entity: the reader never fetches, so
    /// it cannot include one (XML 1.0 §4.4.3 permits a non-validating
    /// processor to decline, and the reader has no way to report a skipped
    /// entity, so it refuses rather than drop the reference). An external
    /// entity may be *declared*, and a document may name an external subset:
    /// neither is read, and neither is refused.
    ExternalEntity,
    /// A parameter-entity reference inside a markup declaration of the internal
    /// subset (XML 1.0 well-formedness constraint "PEs in Internal Subset":
    /// they may appear only between declarations).
    ParameterEntity,
    /// A reference to a parameter entity no declaration in the internal subset
    /// binds, when no external declaration could have.
    UndeclaredParameterEntity(String),
    /// A reference to an unparsed (`NDATA`) entity, which is a well-formedness
    /// error wherever it appears (constraint "Parsed Entity").
    UnparsedEntity(String),
    /// A reference to an entity that no declaration the reader has read binds,
    /// in a document whose other declarations it has not read completely: an
    /// external subset or external parameter entity (never fetched), or an
    /// internal parameter-entity reference. XML 1.0 §4.1 makes that a mere
    /// validity error, and §4.4.3 has the processor report the skipped entity
    /// to the application; the reader has no such report, so it refuses
    /// rather than drop the reference.
    UnexpandedEntity(String),
    /// A declaration of `lt`, `gt`, `amp`, `apos` or `quot` that is not, as
    /// XML 1.0 §4.6 requires, an internal entity whose replacement text is the
    /// character (or, for `lt` and `amp`, a character reference to it).
    PredefinedEntity(String),
    /// An element nested deeper than [`Options::max_depth`](super::Options::max_depth).
    DepthLimit {
        /// The limit that was exceeded.
        limit: usize,
    },
    /// Expanding internal entities would produce more than
    /// [`Dtd::InternalSubset::max_expansion`](super::Dtd::InternalSubset) bytes.
    EntityExpansionLimit {
        /// The limit that was exceeded.
        limit: usize,
    },
    /// An entity whose replacement text refers back to itself.
    RecursiveEntity(String),
    /// A reference to an entity that is neither predefined nor declared.
    UndeclaredEntity(String),
    /// An entity's replacement text is not balanced content where it is
    /// referenced: it opens an element it does not close, closes one it did
    /// not open, or carries `<` into an attribute value.
    UnbalancedEntity(String),
    /// A character reference that names no XML `Char` or is not spelled
    /// `&#` digits `;` / `&#x` hex digits `;`.
    InvalidCharRef,
    /// A scalar outside the XML 1.0 `Char` production.
    IllegalChar(char),
    /// The input ended inside a construct.
    UnexpectedEof,
    /// A token other than the one the grammar requires here.
    Expected(&'static str),
    /// A name that is not an XML `Name`, or not a namespace `QName`.
    InvalidName,
    /// An end tag that does not close the open element.
    MismatchedEndTag {
        /// The open element's name.
        expected: String,
        /// The end tag's name.
        found: String,
    },
    /// An attribute repeated on one start tag, by name or by expanded name.
    DuplicateAttribute(String),
    /// A prefix used with no namespace declaration in scope.
    UndeclaredPrefix(String),
    /// A declaration or use of the reserved `xml`/`xmlns` prefixes or their
    /// namespace names that Namespaces in XML 1.0 §3 forbids.
    ReservedNamespace(String),
    /// `xmlns:p=""`: Namespaces in XML 1.0 cannot undeclare a prefix.
    EmptyNamespacePrefix(String),
    /// A `<` inside an attribute value.
    LessThanInAttributeValue,
    /// `]]>` in character data outside a CDATA section.
    CdataEndInText,
    /// `--` inside a comment, or a comment ending in `-`.
    InvalidComment,
    /// A processing instruction whose target is `xml` in any letter case, or
    /// carries a colon.
    ReservedPiTarget,
    /// An XML declaration that is not the very first thing in the document.
    MisplacedXmlDeclaration,
    /// A malformed XML declaration.
    InvalidXmlDeclaration,
    /// An XML declaration naming a version other than `1.0`: the reader is an
    /// XML 1.0 processor and never reads an XML 1.1 (or later) document as
    /// though it were 1.0.
    UnsupportedVersion(String),
    /// A document whose bytes are in an encoding the reader does not decode
    /// (see [`decode`](super::decode)): named by its declaration or by the
    /// byte pattern.
    UnsupportedEncoding(String),
    /// A document whose bytes contradict its declared encoding, or are not
    /// well-formed in it.
    Encoding(&'static str),
    /// A second document type declaration, or one after the root element.
    MisplacedDoctype,
    /// The document has no root element.
    NoRootElement,
    /// Character data or a second element outside the root element.
    ContentOutsideRoot,
}

impl fmt::Display for XmlErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Doctype => f.write_str("document type declarations are refused"),
            Self::ExternalEntity => {
                f.write_str("external entities are never fetched, so a reference to one is refused")
            }
            Self::ParameterEntity => f.write_str(
                "a parameter-entity reference inside a markup declaration of the internal subset",
            ),
            Self::UndeclaredParameterEntity(name) => {
                write!(f, "undeclared parameter entity `%{name};`")
            }
            Self::UnparsedEntity(name) => {
                write!(f, "reference to the unparsed entity `&{name};`")
            }
            Self::UnexpandedEntity(name) => write!(
                f,
                "`&{name};` may be declared where the reader does not read (an external subset, an external parameter entity or an internal parameter entity)"
            ),
            Self::PredefinedEntity(name) => write!(
                f,
                "`{name}` may be declared only as the character it escapes"
            ),
            Self::DepthLimit { limit } => {
                write!(f, "element nesting exceeds the limit of {limit}")
            }
            Self::EntityExpansionLimit { limit } => {
                write!(f, "entity expansion exceeds the limit of {limit} bytes")
            }
            Self::RecursiveEntity(name) => write!(f, "entity `{name}` refers to itself"),
            Self::UndeclaredEntity(name) => write!(f, "undeclared entity `&{name};`"),
            Self::UnbalancedEntity(name) => {
                write!(f, "entity `{name}` is not balanced content here")
            }
            Self::InvalidCharRef => {
                f.write_str("character reference does not name an XML character")
            }
            Self::IllegalChar(c) => write!(f, "U+{:04X} is not an XML character", u32::from(*c)),
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::Expected(what) => write!(f, "expected {what}"),
            Self::InvalidName => f.write_str("invalid XML name"),
            Self::MismatchedEndTag { expected, found } => {
                write!(f, "end tag `</{found}>` does not close `<{expected}>`")
            }
            Self::DuplicateAttribute(name) => write!(f, "duplicate attribute `{name}`"),
            Self::UndeclaredPrefix(prefix) => write!(f, "undeclared namespace prefix `{prefix}`"),
            Self::ReservedNamespace(what) => write!(f, "reserved namespace misuse: {what}"),
            Self::EmptyNamespacePrefix(prefix) => {
                write!(
                    f,
                    "prefix `{prefix}` cannot be bound to the empty namespace"
                )
            }
            Self::LessThanInAttributeValue => f.write_str("`<` in an attribute value"),
            Self::CdataEndInText => f.write_str("`]]>` in character data"),
            Self::InvalidComment => f.write_str("`--` inside a comment"),
            Self::ReservedPiTarget => f.write_str("reserved processing-instruction target"),
            Self::MisplacedXmlDeclaration => {
                f.write_str("the XML declaration must open the document")
            }
            Self::InvalidXmlDeclaration => f.write_str("malformed XML declaration"),
            Self::UnsupportedVersion(version) => {
                write!(f, "XML version `{version}` is not supported (XML 1.0 only)")
            }
            Self::UnsupportedEncoding(name) => {
                write!(
                    f,
                    "encoding `{name}` is not supported (UTF-8, UTF-16, US-ASCII and ISO-8859-1 only)"
                )
            }
            Self::Encoding(what) => write!(f, "encoding error: {what}"),
            Self::MisplacedDoctype => {
                f.write_str("the document type declaration must precede the root element")
            }
            Self::NoRootElement => f.write_str("the document has no root element"),
            Self::ContentOutsideRoot => f.write_str("content outside the root element"),
        }
    }
}

/// A refused document: what was wrong and the byte offset in the source text
/// where it was found. Inside an entity's replacement text the offset is the
/// document's reference to the outermost entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XmlError {
    kind: XmlErrorKind,
    offset: usize,
}

impl XmlError {
    pub(super) const fn new(kind: XmlErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// What was wrong.
    #[must_use]
    pub const fn kind(&self) -> &XmlErrorKind {
        &self.kind
    }

    /// The byte offset in the source text.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// The 1-based line and column (in scalars) of the offset within `text`,
    /// the source the error came from. A line ends at LINE FEED, at CARRIAGE
    /// RETURN, and at a CR LF pair counted once (XML 1.0 §2.11).
    #[must_use]
    pub fn line_column(&self, text: &str) -> (usize, usize) {
        let end = self.offset.min(text.len());
        let before = text.get(..end).unwrap_or(text);
        let mut line = 1;
        let mut line_start = 0;
        let bytes = before.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => {
                    line += 1;
                    line_start = i + 1;
                }
                b'\r' => {
                    line += 1;
                    if bytes.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    line_start = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        (line, before[line_start..].chars().count() + 1)
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.kind, self.offset)
    }
}

impl std::error::Error for XmlError {}

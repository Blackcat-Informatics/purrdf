// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical document: the canonical line sequence spelled as N-Quads 1.2.
//!
//! The definition ends at the canonical line sequence and its digest (7.6); a
//! text document is a presentation a format layers on top (7.14), and its line
//! order is the format's. PurRDF's document keeps the order of the sequence:
//! one line of text per statement line, in ascending bytewise order of the line
//! encoding (3.7), not in the order of the spelled text. Every blank node is
//! `_:` and its position in decimal.
//!
//! What each kind becomes:
//!
//! * a `quad` line is its statement;
//! * a `reifier-binding` line is the statement `reifier rdf:reifies triple`;
//! * an `annotation` line is the statement `reifier predicate object`, once,
//!   whatever its count;
//! * an `empty-graph` line and a `record` line have no N-Quads spelling and
//!   are not written.
//!
//! The text is therefore not injective on the sequence: two lines of two kinds
//! may spell one statement, and both are written. The digest, not the text,
//! carries the identity.
//!
//! A composite literal (`cdt:List` or `cdt:Map`) that names blank nodes is
//! lowered as the blank leaf of a composite node and a `record` line that
//! holds its text and the blank nodes it names ([`composite_payload`]). The
//! canonical document spells a composite node as its literal, each embedded
//! blank node `_:` and its position ([`Composites`]); no label spells the
//! composite node itself.
//!
//! Terms are spelled as the canonical form of RDF 1.2 N-Quads (section 3): an
//! IRI escapes exactly what `IRIREF` forbids, and U+FFFE and U+FFFF in a
//! literal are `\uFFFE` and `\uFFFF`, since neither matches the XML 1.1 `Char`
//! production. DEL and the C1 controls ride raw. The digest is over the binary
//! line encodings, so these spellings do not move it.
//!
//! The encoding is read by the rules of 3.7 and Lemma E; nothing here parses a
//! label.

use std::collections::BTreeMap;
use std::fmt;

use purrdf_iri::vocab::rdf::REIFIES;
use purrdf_lex::term_syntax::{TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN, write_iri, write_literal};
use purrdf_lex::text_out::TextOut;

use super::lines::push_bytes;
use super::write_label;
use crate::RdfTextDirection;

const TAG_IRI: u8 = 0x01;
const TAG_LITERAL: u8 = 0x02;
const TAG_TRIPLE: u8 = 0x03;
const TAG_FIXED: u8 = 0x05;
const TAG_ABSENT: u8 = 0x20;
const TAG_PRESENT: u8 = 0x21;
const TAG_LTR: u8 = 0x30;
const TAG_RTL: u8 = 0x31;
const TAG_QUAD: u8 = 0x40;
const TAG_REIFIER_BINDING: u8 = 0x41;
const TAG_ANNOTATION: u8 = 0x42;
const TAG_EMPTY_GRAPH: u8 = 0x43;
const TAG_RECORD: u8 = 0x44;

/// Why a line encoding could not be spelled. A sequence the engine produced is
/// never in this state; it is the typed refusal for bytes that are not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DocumentError {
    /// The bytes end inside a field, or leave bytes after the line.
    Truncated,
    /// A tag byte the encoding does not use at that place, or a `free` leaf.
    Tag(u8),
    /// A text field is not UTF-8.
    NotText,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("a line encoding ends inside a field or runs past it"),
            Self::Tag(tag) => write!(f, "a line encoding holds the unexpected tag {tag:#04x}"),
            Self::NotText => f.write_str("a line encoding holds a text field that is not UTF-8"),
        }
    }
}

impl std::error::Error for DocumentError {}

/// The marker byte after the four scope bytes of a composite node's source
/// identifier. A blank node of a view has its label there, which is UTF-8, and
/// `0xFF` never begins UTF-8, so no blank node of a view has such a source.
const COMPOSITE_MARK: u8 = 0xFF;

/// The source identifier of the composite node of the literal leaf whose
/// encoding is `literal`: four zero bytes, [`COMPOSITE_MARK`], then the
/// encoding. One literal value is one node.
pub(crate) fn composite_source(literal: &[u8]) -> Vec<u8> {
    let mut source = Vec::with_capacity(5 + literal.len());
    source.extend_from_slice(&[0, 0, 0, 0, COMPOSITE_MARK]);
    source.extend_from_slice(literal);
    source
}

/// Whether `source` is the source identifier of a composite node.
pub(crate) fn is_composite_source(source: &[u8]) -> bool {
    source.get(4) == Some(&COMPOSITE_MARK)
}

/// The payload of a composite node's `record` line: `bytes(datatype)`, then
/// `bytes(segment)` for each segment of the lexical form around its embedded
/// blank-node tokens, in order. The record's terms are the composite node and
/// then the blank node of each token, so the segments are one more than the
/// embedded tokens and one fewer than the terms.
pub(crate) fn composite_payload(datatype: &str, segments: &[&str]) -> Vec<u8> {
    let mut payload = Vec::new();
    push_bytes(&mut payload, datatype.as_bytes());
    for segment in segments {
        push_bytes(&mut payload, segment.as_bytes());
    }
    payload
}

/// The literal each composite node of a canonical line sequence spells, by the
/// node's position: its lexical form with each embedded blank node written
/// `_:` and its position, and its datatype.
#[derive(Clone, Debug, Default)]
pub(crate) struct Composites(BTreeMap<u64, (String, String)>);

impl Composites {
    /// Read the composite spellings of `lines`, a canonical line sequence, in
    /// which `is_composite` names the positions of the composite nodes.
    ///
    /// # Errors
    /// [`DocumentError`] when a composite node's record is not one this
    /// lowering writes.
    pub(crate) fn of(
        lines: &[Box<[u8]>],
        is_composite: impl Fn(u64) -> bool,
    ) -> Result<Self, DocumentError> {
        let mut spelled = BTreeMap::new();
        for line in lines {
            let Some((&TAG_RECORD, rest)) = line.split_first() else {
                continue;
            };
            let mut reader = Reader::new(rest, None);
            let payload = reader.field()?;
            let terms = reader.u64()?;
            if terms == 0 || reader.tag()? != TAG_FIXED {
                continue;
            }
            let node = reader.u64()?;
            if !is_composite(node) {
                continue;
            }
            let mut fields = Reader::new(payload, None);
            let datatype = fields.text()?;
            let mut lexical = String::from(fields.text()?);
            for _ in 1..terms {
                let tag = reader.tag()?;
                if tag != TAG_FIXED {
                    return Err(DocumentError::Tag(tag));
                }
                write_label(reader.u64()?, &mut lexical);
                lexical.push_str(fields.text()?);
            }
            if !fields.bytes.is_empty() || !reader.bytes.is_empty() {
                return Err(DocumentError::Truncated);
            }
            spelled.insert(node, (lexical, datatype.to_owned()));
        }
        Ok(Self(spelled))
    }
}

/// A cursor over one line encoding.
struct Reader<'a, 'c> {
    bytes: &'a [u8],
    /// How composite nodes are spelled; none spells every leaf as a label.
    composites: Option<&'c Composites>,
}

impl<'a, 'c> Reader<'a, 'c> {
    const fn new(bytes: &'a [u8], composites: Option<&'c Composites>) -> Self {
        Self { bytes, composites }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DocumentError> {
        let (head, rest) = self
            .bytes
            .split_at_checked(n)
            .ok_or(DocumentError::Truncated)?;
        self.bytes = rest;
        Ok(head)
    }

    fn tag(&mut self) -> Result<u8, DocumentError> {
        Ok(self.take(1)?[0])
    }

    fn u64(&mut self) -> Result<u64, DocumentError> {
        let word: [u8; 8] = self
            .take(8)?
            .try_into()
            .map_err(|_| DocumentError::Truncated)?;
        Ok(u64::from_be_bytes(word))
    }

    /// A `bytes(b)` field.
    fn field(&mut self) -> Result<&'a [u8], DocumentError> {
        let length = usize::try_from(self.u64()?).map_err(|_| DocumentError::Truncated)?;
        self.take(length)
    }

    fn text(&mut self) -> Result<&'a str, DocumentError> {
        std::str::from_utf8(self.field()?).map_err(|_| DocumentError::NotText)
    }

    /// The direction option: `absent`, or `present` with `ltr` or `rtl`.
    fn direction(&mut self) -> Result<Option<&'static str>, DocumentError> {
        match self.tag()? {
            TAG_ABSENT => Ok(None),
            TAG_PRESENT => match self.tag()? {
                TAG_LTR => Ok(Some(RdfTextDirection::Ltr.as_str())),
                TAG_RTL => Ok(Some(RdfTextDirection::Rtl.as_str())),
                other => Err(DocumentError::Tag(other)),
            },
            other => Err(DocumentError::Tag(other)),
        }
    }

    /// Spell the next term into `out`. Triple terms nest to the depth of the
    /// input, so the walk keeps its own work list instead of the machine stack.
    fn term<W: TextOut + ?Sized>(&mut self, out: &mut W) -> Result<(), DocumentError> {
        // Each entry is a piece still to write: a term to read, or a literal
        // piece of text between the terms of a triple.
        enum Step {
            Term,
            Text(&'static str),
        }
        let mut pending = vec![Step::Term];
        while let Some(step) = pending.pop() {
            match step {
                Step::Text(text) => out.push_str(text),
                Step::Term => match self.tag()? {
                    TAG_IRI => write_iri(self.text()?, out),
                    TAG_LITERAL => {
                        let lexical = self.text()?;
                        let datatype = self.text()?;
                        let language = match self.tag()? {
                            TAG_ABSENT => None,
                            TAG_PRESENT => Some(self.text()?),
                            other => return Err(DocumentError::Tag(other)),
                        };
                        let direction = self.direction()?;
                        write_literal(lexical, datatype, language, direction, out);
                    }
                    TAG_FIXED => {
                        let position = self.u64()?;
                        match self.composites.and_then(|c| c.0.get(&position)) {
                            Some((lexical, datatype)) => {
                                write_literal(lexical, datatype, None, None, out);
                            }
                            None => write_label(position, out),
                        }
                    }
                    TAG_TRIPLE => {
                        out.push_str(TRIPLE_TERM_OPEN);
                        out.push(' ');
                        // The work list pops from the end: subject first, the
                        // close last.
                        pending.extend([
                            Step::Text(TRIPLE_TERM_CLOSE),
                            Step::Text(" "),
                            Step::Term,
                            Step::Text(" "),
                            Step::Term,
                            Step::Text(" "),
                            Step::Term,
                        ]);
                    }
                    other => return Err(DocumentError::Tag(other)),
                },
            }
        }
        Ok(())
    }

    /// The graph option and the closing of the statement.
    fn graph_and_end<W: TextOut + ?Sized>(&mut self, out: &mut W) -> Result<(), DocumentError> {
        match self.tag()? {
            TAG_ABSENT => {}
            TAG_PRESENT => {
                out.push(' ');
                self.term(out)?;
            }
            other => return Err(DocumentError::Tag(other)),
        }
        out.push_str(" .\n");
        Ok(())
    }
}

/// Spell one line encoding as an N-Quads statement, or write nothing for a
/// kind with no N-Quads spelling. Every `fixed` leaf is spelled as its label;
/// the canonical document of a dataset view, which spells a composite node as
/// its literal, is [`try_canonicalize_view`](crate::try_canonicalize_view).
///
/// # Errors
/// [`DocumentError`] when `encoding` is not one line encoding of 3.7.
pub fn write_line<W: TextOut + ?Sized>(encoding: &[u8], out: &mut W) -> Result<(), DocumentError> {
    spell_line(Reader::new(encoding, None), out)
}

/// [`write_line`] with each composite node of `composites` spelled as its
/// literal.
///
/// # Errors
/// As [`write_line`].
pub(crate) fn write_line_with<W: TextOut + ?Sized>(
    encoding: &[u8],
    composites: &Composites,
    out: &mut W,
) -> Result<(), DocumentError> {
    spell_line(Reader::new(encoding, Some(composites)), out)
}

fn spell_line<W: TextOut + ?Sized>(
    mut reader: Reader<'_, '_>,
    out: &mut W,
) -> Result<(), DocumentError> {
    match reader.tag()? {
        TAG_QUAD => {
            reader.term(out)?;
            out.push(' ');
            reader.term(out)?;
            out.push(' ');
            reader.term(out)?;
            reader.graph_and_end(out)?;
        }
        TAG_REIFIER_BINDING => {
            reader.term(out)?;
            out.push(' ');
            write_iri(REIFIES, out);
            out.push(' ');
            reader.term(out)?;
            reader.graph_and_end(out)?;
        }
        TAG_ANNOTATION => {
            reader.u64()?;
            reader.term(out)?;
            out.push(' ');
            reader.term(out)?;
            out.push(' ');
            reader.term(out)?;
            reader.graph_and_end(out)?;
        }
        TAG_EMPTY_GRAPH | TAG_RECORD => return Ok(()),
        other => return Err(DocumentError::Tag(other)),
    }
    if reader.bytes.is_empty() {
        Ok(())
    } else {
        Err(DocumentError::Truncated)
    }
}

/// Write the document of `lines`, the canonical line sequence of a labelling
/// ([`Labelled::lines`](super::Labelled::lines)), into `out`, every `fixed`
/// leaf spelled as its label as [`write_line`] does.
///
/// Statements are written in the order of `lines`; see the module
/// documentation for what each kind becomes.
///
/// # Errors
/// [`DocumentError`] when an element is not a line encoding of 3.7. Nothing is
/// guaranteed about the text already written when it is returned.
pub fn write_document<W: TextOut + ?Sized>(
    lines: &[Box<[u8]>],
    out: &mut W,
) -> Result<(), DocumentError> {
    for line in lines {
        write_line(line, out)?;
    }
    Ok(())
}

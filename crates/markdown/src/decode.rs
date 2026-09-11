// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The decode law: the graph back to the document, byte for byte.
//!
//! The emission covers every byte of the source — a unit's plain text
//! over its byte span, a section's typed verbatim heading line over its
//! own span, a structure node's typed verbatim run over the bytes
//! nothing owns — so a consumer holding only the claims rebuilds the
//! document and proves the result against the document node's
//! `sourceDigest`.
//!
//! Both halves of that law are shipped, and neither is restated
//! anywhere. The span half — [`reconstruct`], [`VerbatimSpan`],
//! [`ReconstructError`] — is the **kernel's** cover law, re-exported
//! here exactly as [`verify_unit`](crate::verify_unit) reaches the
//! kernel's verification law. The graph half is [`decode_document`]:
//! the specification's two extraction rules applied to a parsed
//! dataset under the caller's own [`Vocabulary`], then the kernel law
//! over what they extracted. A consumer that also states these rules
//! in its own vocabulary states them by calling here, so every reader
//! of the graph refuses the same graphs for the same reasons.
//!
//! # The two extraction rules
//!
//! One [`VerbatimSpan`] per text-carrying fact, and no class dispatch:
//!
//! * a node stating the vocabulary's `verbatim` — as a literal typed
//!   with the verbatim datatype — contributes its `verbatimStart`,
//!   `verbatimEnd` and the literal's exact bytes;
//! * a node stating the vocabulary's `text` as a **plain** `xsd:string`
//!   — a unit — contributes its `byteStart`, `byteEnd` and that
//!   literal, with `continues`, where the node states the edge,
//!   resolved to the named piece's own byte span.
//!
//! A literal under those predicates in any *other* shape — a typed
//! text, an untyped verbatim, a language-tagged anything — contributes
//! nothing: the law reads exactly what the emission law writes, and a
//! graph whose cover leans on such a literal is refused downstream as
//! the uncovered range it leaves.

use purrdf_core::ir::{RdfDataset, TermRef};

pub use purrdf_core::cover::{ReconstructError, VerbatimSpan, reconstruct};

use crate::model::Document;
use crate::profile::Vocabulary;

/// The one datatype the unit rule admits: a unit's text is a plain
/// literal, and a plain literal's datatype is `xsd:string`.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Why a graph could not be read back into a cover: the refusals of
/// the **graph** half of the decode law. The span half's refusals
/// arrive wrapped, in [`DecodeError::Reconstruct`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// The document node states no readable value under a fact the
    /// decode law needs — its byte length, or its source digest.
    MissingDocumentFact {
        /// The local name of the predicate whose object is missing or
        /// unreadable.
        predicate: &'static str,
    },
    /// The document's source digest is stated but is not
    /// `sha256:<64 hex digits>`.
    MalformedSourceDigest {
        /// The literal as stated.
        stated: String,
    },
    /// A node carries a text-bearing literal and not the span it
    /// quotes: a `verbatim` without `verbatimStart`/`verbatimEnd`, or
    /// a unit text without `byteStart`/`byteEnd`.
    IncompleteSpan {
        /// The node's IRI.
        subject: String,
        /// The local name of the offset predicate that is missing.
        missing: &'static str,
    },
    /// A `continues` edge names a piece the graph does not carry a
    /// byte span for.
    UnknownContinuedPiece {
        /// The continuation's IRI.
        subject: String,
        /// The IRI the edge names.
        piece: String,
    },
    /// The extracted cover refused under the kernel's span law.
    Reconstruct(ReconstructError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDocumentFact { predicate } => {
                write!(f, "the document node states no readable {predicate}")
            }
            Self::MalformedSourceDigest { stated } => {
                write!(f, "source digest {stated:?} is not sha256:<hex>")
            }
            Self::IncompleteSpan { subject, missing } => {
                write!(f, "<{subject}> carries text and no {missing} for it")
            }
            Self::UnknownContinuedPiece { subject, piece } => {
                write!(
                    f,
                    "<{subject}> continues <{piece}>, which states no byte span"
                )
            }
            Self::Reconstruct(cause) => write!(f, "{cause}"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Reconstruct(cause) => Some(cause),
            _ => None,
        }
    }
}

impl From<ReconstructError> for DecodeError {
    fn from(cause: ReconstructError) -> Self {
        Self::Reconstruct(cause)
    }
}

/// One node's text-bearing facts, gathered before the rules read them.
#[derive(Default)]
struct NodeFacts {
    text: Option<String>,
    verbatim: Option<String>,
    byte_start: Option<u64>,
    byte_end: Option<u64>,
    verbatim_start: Option<u64>,
    verbatim_end: Option<u64>,
    continues: Option<String>,
}

/// The document back from its graph: the specification's two
/// extraction rules over a parsed dataset, then the kernel's
/// [`reconstruct`] over what they extracted.
///
/// `dataset` is any RDF 1.2 dataset holding the document's claims —
/// parsed back from Turtle, merged from several sources, or built by
/// hand; the rules read predicates and literal shapes and nothing
/// else, so extra triples beside the claims are ignored rather than
/// refused. `vocabulary` is the caller's own — the one the graph was
/// emitted under — and `document_id` names the document node the byte
/// length and source digest are read from.
///
/// # Errors
///
/// The graph half first: [`DecodeError::MissingDocumentFact`] and
/// [`DecodeError::MalformedSourceDigest`] for a document node that
/// cannot anchor a decode; [`DecodeError::IncompleteSpan`] for a
/// text-bearing node without its offsets;
/// [`DecodeError::UnknownContinuedPiece`] for a `continues` edge
/// naming a piece with no span. Then the span half, wrapped in
/// [`DecodeError::Reconstruct`]: exactly [`reconstruct`]'s refusals,
/// in exactly its order.
pub fn decode_document(
    dataset: &RdfDataset,
    vocabulary: &Vocabulary,
    document_id: &str,
) -> Result<String, DecodeError> {
    let mut byte_length: Option<u64> = None;
    let mut source_digest: Option<String> = None;
    let mut nodes: std::collections::BTreeMap<&str, NodeFacts> = std::collections::BTreeMap::new();

    for quad in dataset.iter() {
        let TermRef::Iri(subject) = quad.s else {
            continue;
        };
        let TermRef::Iri(predicate) = quad.p else {
            continue;
        };
        let literal = |datatype_iri: &str| match quad.o {
            TermRef::Literal {
                lexical,
                datatype,
                language: None,
                ..
            } if matches!(dataset.resolve(datatype), TermRef::Iri(iri) if iri == datatype_iri) => {
                Some(lexical.to_owned())
            }
            _ => None,
        };
        let integer = || {
            literal("http://www.w3.org/2001/XMLSchema#integer").and_then(|s| s.parse::<u64>().ok())
        };
        if subject == document_id {
            if predicate == vocabulary.byte_length {
                byte_length = integer();
            } else if predicate == vocabulary.source_digest {
                source_digest = literal(&vocabulary.dt_digest);
            }
        }
        let node = nodes.entry(subject).or_default();
        if predicate == vocabulary.text {
            if let Some(text) = literal(XSD_STRING) {
                node.text = Some(text);
            }
        } else if predicate == vocabulary.verbatim {
            if let Some(text) = literal(&vocabulary.dt_verbatim) {
                node.verbatim = Some(text);
            }
        } else if predicate == vocabulary.byte_start {
            node.byte_start = integer();
        } else if predicate == vocabulary.byte_end {
            node.byte_end = integer();
        } else if predicate == vocabulary.verbatim_start {
            node.verbatim_start = integer();
        } else if predicate == vocabulary.verbatim_end {
            node.verbatim_end = integer();
        } else if predicate == vocabulary.continues
            && let TermRef::Iri(piece) = quad.o
        {
            node.continues = Some(piece.to_owned());
        }
    }

    let byte_length = byte_length.ok_or(DecodeError::MissingDocumentFact {
        predicate: "byteLength",
    })?;
    let stated = source_digest.ok_or(DecodeError::MissingDocumentFact {
        predicate: "sourceDigest",
    })?;
    let source_digest = stated
        .strip_prefix("sha256:")
        .and_then(purrdf_core::ContentDigest::from_hex)
        .ok_or(DecodeError::MalformedSourceDigest { stated })?;

    let mut spans: Vec<VerbatimSpan<'_>> = Vec::new();
    for (subject, node) in &nodes {
        if let Some(text) = &node.verbatim {
            spans.push(VerbatimSpan {
                byte_start: require(node.verbatim_start, subject, "verbatimStart")?,
                byte_end: require(node.verbatim_end, subject, "verbatimEnd")?,
                text,
                continues: None,
            });
        }
        if let Some(text) = &node.text {
            let continues = match &node.continues {
                None => None,
                Some(piece) => {
                    let it = nodes
                        .get(piece.as_str())
                        .filter(|p| p.byte_start.is_some() && p.byte_end.is_some())
                        .ok_or_else(|| DecodeError::UnknownContinuedPiece {
                            subject: (*subject).to_owned(),
                            piece: piece.clone(),
                        })?;
                    Some((
                        it.byte_start.expect("filtered present"),
                        it.byte_end.expect("filtered present"),
                    ))
                }
            };
            spans.push(VerbatimSpan {
                byte_start: require(node.byte_start, subject, "byteStart")?,
                byte_end: require(node.byte_end, subject, "byteEnd")?,
                text,
                continues,
            });
        }
    }
    Ok(reconstruct(byte_length, &source_digest, &spans)?)
}

/// An offset a text-bearing node must state, or the refusal that names
/// what it left out.
fn require(offset: Option<u64>, subject: &str, missing: &'static str) -> Result<u64, DecodeError> {
    offset.ok_or_else(|| DecodeError::IncompleteSpan {
        subject: subject.to_owned(),
        missing,
    })
}

/// The model's own spans, as [`reconstruct`] takes them: what the
/// projection is about to emit, read off the model rather than back out
/// of the graph — the write-side half of the codec's verification, and
/// the tests' independent read of the same law.
pub(crate) fn model_spans<'d>(document: &Document<'d>) -> Vec<VerbatimSpan<'d>> {
    let mut spans = Vec::with_capacity(
        document.sections().len() + document.units().len() + document.structures().len(),
    );
    for section in document.sections() {
        let heading = section.heading_span();
        spans.push(VerbatimSpan {
            byte_start: heading.start,
            byte_end: heading.end,
            text: document.structure_text(heading),
            continues: None,
        });
    }
    for unit in document.units() {
        let span = unit.span();
        spans.push(VerbatimSpan {
            byte_start: span.start,
            byte_end: span.end,
            text: unit.quote(),
            continues: unit.continues().map(|i| {
                let piece = document.units()[i].span();
                (piece.start, piece.end)
            }),
        });
    }
    for span in document.structures() {
        spans.push(VerbatimSpan {
            byte_start: span.start,
            byte_end: span.end,
            text: document.structure_text(*span),
            continues: None,
        });
    }
    spans
}

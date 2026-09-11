// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The decode law: the graph back to the document, byte for byte.
//!
//! The emission covers every byte of the source — a unit's plain text
//! over its byte span, a section's typed verbatim heading line over its
//! own span, a structure node's typed verbatim run over the bytes
//! nothing else owns — so a consumer holding only the claims rebuilds
//! the document by writing each covered span's bytes at its offset and
//! proving the result against the document node's `sourceDigest`.
//! [`reconstruct`] is that law, stated once, so every consumer refuses
//! the same graphs for the same reasons.
//!
//! # What the caller extracts, and by what rule
//!
//! One [`VerbatimSpan`] per text-carrying fact of the graph, read off
//! the claims by two rules and no class dispatch:
//!
//! * a node stating the vocabulary's `verbatim` contributes its
//!   `verbatimStart`, `verbatimEnd` and the literal's lexical form;
//! * a node stating the vocabulary's `text` — a unit — contributes its
//!   `byteStart`, `byteEnd` and that plain literal, with `continues`
//!   resolved to the named piece's own byte span when the edge is
//!   present.
//!
//! # The overlap law
//!
//! The spans of a lawful graph are pairwise disjoint with one declared
//! exception: a split's continuation reaches back into the piece it
//! [`continues`](VerbatimSpan::continues), and there the overlapping
//! bytes must agree. An **undeclared** overlap is refused even when its
//! bytes agree, because agreement is no evidence of structure — two
//! spans sliced from the same file agree wherever they overlap, so a
//! byte-agreement test would pass on exactly the graphs that are
//! defective. The declaration is the graph's own `continues` edge:
//! the exception is stated by the data, never inferred by the reader.
//!
//! # Bounds
//!
//! A graph is attacker-influenced bytes, so the decoder's allocation is
//! bounded by its **input**, never by what the graph claims: a
//! `byteLength` no span set could cover is refused before a buffer
//! exists, and the buffer allocated is exactly the declared length,
//! which the coverage walk has by then proven to be at most the total
//! of the spans handed in.

use purrdf_core::ContentDigest;

use crate::model::Document;

/// One text-carrying span of the graph: the bytes at
/// `byte_start..byte_end`, quoted verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerbatimSpan<'a> {
    /// The first byte of the span.
    pub byte_start: u64,
    /// One past the last byte of the span.
    pub byte_end: u64,
    /// The span's bytes, exactly as the graph carries them — a unit's
    /// plain `text` literal, or a `verbatim` literal's lexical form.
    pub text: &'a str,
    /// The byte span of the piece this one continues, when the graph
    /// states a `continues` edge: the one declaration under which a
    /// partial overlap is lawful.
    pub continues: Option<(u64, u64)>,
}

/// Why a graph could not be decoded back into a document.
///
/// Every refusal names the bytes it is about, because a decode failure
/// is a finding about a graph somebody has to go and look at.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReconstructError {
    /// A span ends before it starts.
    InvertedSpan {
        /// The span's declared first byte.
        byte_start: u64,
        /// The span's declared end, before its start.
        byte_end: u64,
    },
    /// A span reaches past the declared byte length.
    SpanOutOfBounds {
        /// The span's first byte.
        byte_start: u64,
        /// One past the span's last byte.
        byte_end: u64,
        /// The document's declared byte length.
        byte_length: u64,
    },
    /// A span's text is not as many bytes as the span it claims to
    /// quote.
    SpanLengthMismatch {
        /// The span's first byte.
        byte_start: u64,
        /// One past the span's last byte.
        byte_end: u64,
        /// How many bytes the text actually is.
        text_bytes: usize,
    },
    /// A run of bytes no span covers. The graph is an index, not a
    /// codec's output, or a claim of it is missing.
    UncoveredRange {
        /// The first uncovered byte.
        byte_start: u64,
        /// One past the last uncovered byte of the run.
        byte_end: u64,
    },
    /// Two spans share bytes and the graph declares no `continues`
    /// edge that answers for them — no edge at all, an edge naming a
    /// span the graph does not carry, or an edge naming one that does
    /// not cover the shared bytes. Refused **even when the shared bytes
    /// agree**: two spans sliced from one file agree wherever they
    /// overlap, so agreement is evidence they came from the same file
    /// and none at all that the structure is lawful.
    UndeclaredOverlap {
        /// The later span's first byte.
        byte_start: u64,
        /// One past the later span's last byte.
        byte_end: u64,
        /// The first byte of the span already covering the shared run.
        covered_start: u64,
        /// One past the last byte of the span already covering it.
        covered_end: u64,
    },
    /// A declared continuation disagrees with the piece it continues
    /// about a byte they both cover.
    OverlapDisagreement {
        /// The first byte offset at which the two texts disagree.
        at: u64,
    },
    /// The reconstructed bytes are not UTF-8, which no lawful set of
    /// spans can produce: every span's text is UTF-8 whole, and every
    /// span boundary is one of some text.
    InvalidUtf8 {
        /// The byte offset where decoding stopped.
        valid_up_to: usize,
    },
    /// The rebuilt document does not digest to the digest the graph
    /// states of its source.
    DigestMismatch {
        /// The digest the graph states, as lowercase hex.
        expected: String,
        /// The digest of what the spans rebuilt, as lowercase hex.
        reconstructed: String,
    },
}

impl std::fmt::Display for ReconstructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvertedSpan {
                byte_start,
                byte_end,
            } => {
                write!(f, "span {byte_start}..{byte_end} ends before it starts")
            }
            Self::SpanOutOfBounds {
                byte_start,
                byte_end,
                byte_length,
            } => {
                write!(
                    f,
                    "span {byte_start}..{byte_end} reaches past the declared length {byte_length}"
                )
            }
            Self::SpanLengthMismatch {
                byte_start,
                byte_end,
                text_bytes,
            } => {
                write!(
                    f,
                    "span {byte_start}..{byte_end} quotes {text_bytes} bytes of text"
                )
            }
            Self::UncoveredRange {
                byte_start,
                byte_end,
            } => {
                write!(f, "no span covers bytes {byte_start}..{byte_end}")
            }
            Self::UndeclaredOverlap {
                byte_start,
                byte_end,
                covered_start,
                covered_end,
            } => {
                write!(
                    f,
                    "span {byte_start}..{byte_end} overlaps {covered_start}..{covered_end} \
                     and declares no continues edge to it"
                )
            }
            Self::OverlapDisagreement { at } => {
                write!(
                    f,
                    "a continuation disagrees with the piece it continues at byte {at}"
                )
            }
            Self::InvalidUtf8 { valid_up_to } => {
                write!(
                    f,
                    "reconstructed bytes are not UTF-8 after byte {valid_up_to}"
                )
            }
            Self::DigestMismatch {
                expected,
                reconstructed,
            } => {
                write!(
                    f,
                    "reconstruction digests to sha256:{reconstructed}, \
                     and the graph states sha256:{expected}"
                )
            }
        }
    }
}

impl std::error::Error for ReconstructError {}

/// The document back from its graph: writes each span's bytes at its
/// offset, requires every byte covered, admits an overlap only where a
/// `continues` edge declares it and its bytes agree, and proves the
/// result against the stated source digest.
///
/// The spans may come in any order — a graph states no order — and the
/// answer does not depend on theirs. Empty spans cover nothing and are
/// ignored; the graph never emits one, and refusing one would refuse a
/// claim that asserts nothing.
///
/// # Errors
///
/// Each span is answered for first, in the caller's order:
/// [`ReconstructError::InvertedSpan`],
/// [`ReconstructError::SpanOutOfBounds`], and
/// [`ReconstructError::SpanLengthMismatch`]. Then the cover, in byte
/// order: [`ReconstructError::UncoveredRange`] for the first run no
/// span covers, [`ReconstructError::UndeclaredOverlap`] for the first
/// overlap no `continues` edge declares, and
/// [`ReconstructError::OverlapDisagreement`] for the first declared
/// overlap whose bytes disagree. Last, the whole:
/// [`ReconstructError::DigestMismatch`] and
/// [`ReconstructError::InvalidUtf8`].
pub fn reconstruct(
    byte_length: u64,
    source_digest: &ContentDigest,
    spans: &[VerbatimSpan<'_>],
) -> Result<String, ReconstructError> {
    for span in spans {
        if span.byte_end < span.byte_start {
            return Err(ReconstructError::InvertedSpan {
                byte_start: span.byte_start,
                byte_end: span.byte_end,
            });
        }
        if span.byte_end > byte_length {
            return Err(ReconstructError::SpanOutOfBounds {
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                byte_length,
            });
        }
        if span.text.len() as u64 != span.byte_end - span.byte_start {
            return Err(ReconstructError::SpanLengthMismatch {
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                text_bytes: span.text.len(),
            });
        }
    }
    let mut ordered: Vec<&VerbatimSpan<'_>> =
        spans.iter().filter(|s| s.byte_start < s.byte_end).collect();
    ordered.sort_unstable_by_key(|s| (s.byte_start, s.byte_end));

    // The coverage walk, before a buffer exists: `frontier` is one past
    // the last covered byte, and `owner` the span that put it there.
    // Every refusal about the cover is found here, so the allocation
    // below is bounded by the spans in hand — a `byteLength` past their
    // total is refused as the uncovered run it is.
    //
    // An overlap is answered against the span the `continues` edge
    // **declares**, never against whichever span happens to hold the
    // frontier. The two part company on a real chain: two pieces of a
    // split can end at the same cut, and the continuation of the later
    // one then overlaps bytes the frontier credits to the earlier.
    // The split law guarantees the declared piece covers the whole
    // overlap — a continuation starts strictly inside the piece it
    // continues and ends at or past that piece's end — so the declared
    // piece is the one span the shared bytes can always be held
    // against, and a declaration that does not cover them is no answer
    // for them and refuses exactly as no declaration does.
    let by_span: std::collections::BTreeMap<(u64, u64), &VerbatimSpan<'_>> = ordered
        .iter()
        .map(|s| ((s.byte_start, s.byte_end), *s))
        .collect();
    let mut frontier = 0u64;
    let mut owner: Option<&VerbatimSpan<'_>> = None;
    for span in &ordered {
        if span.byte_start > frontier {
            return Err(ReconstructError::UncoveredRange {
                byte_start: frontier,
                byte_end: span.byte_start,
            });
        }
        if span.byte_start < frontier {
            let covering = owner.expect("bytes below the frontier have an owner");
            let shared_end = frontier.min(span.byte_end);
            let declared = span
                .continues
                .and_then(|key| by_span.get(&key))
                .filter(|p| p.byte_start <= span.byte_start && shared_end <= p.byte_end);
            let Some(piece) = declared else {
                return Err(ReconstructError::UndeclaredOverlap {
                    byte_start: span.byte_start,
                    byte_end: span.byte_end,
                    covered_start: covering.byte_start,
                    covered_end: covering.byte_end,
                });
            };
            let shared = (shared_end - span.byte_start) as usize;
            let of_continuation = &span.text.as_bytes()[..shared];
            let from = (span.byte_start - piece.byte_start) as usize;
            let of_piece = &piece.text.as_bytes()[from..from + shared];
            if let Some(differs) = of_continuation
                .iter()
                .zip(of_piece)
                .position(|(a, b)| a != b)
            {
                return Err(ReconstructError::OverlapDisagreement {
                    at: span.byte_start + differs as u64,
                });
            }
        }
        if span.byte_end > frontier {
            frontier = span.byte_end;
            owner = Some(span);
        }
    }
    if frontier < byte_length {
        return Err(ReconstructError::UncoveredRange {
            byte_start: frontier,
            byte_end: byte_length,
        });
    }

    // The cover is proven, so the length fits the spans in hand and the
    // writes below can only restate agreed bytes.
    let mut bytes = vec![0u8; byte_length as usize];
    for span in &ordered {
        bytes[span.byte_start as usize..span.byte_end as usize]
            .copy_from_slice(span.text.as_bytes());
    }
    let digest = ContentDigest::of(&bytes);
    if digest != *source_digest {
        return Err(ReconstructError::DigestMismatch {
            expected: source_digest.to_hex(),
            reconstructed: digest.to_hex(),
        });
    }
    String::from_utf8(bytes).map_err(|e| ReconstructError::InvalidUtf8 {
        valid_up_to: e.utf8_error().valid_up_to(),
    })
}

/// The model's own spans, as [`reconstruct`] takes them: what the
/// projection is about to emit, read off the model rather than back out
/// of the graph. It is the write-side half of the codec's verification
/// — [`render`](crate::render) holds the emission against it before
/// handing a claim out — and the tests' independent read of the same
/// law.
pub(crate) fn model_spans<'d>(document: &Document<'d>) -> Vec<VerbatimSpan<'d>> {
    let mut spans = Vec::new();
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

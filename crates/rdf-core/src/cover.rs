// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The cover law: a set of verbatim byte spans back into the document
//! they cover, byte for byte — the decode half of every ordered codec.
//!
//! A codec in this workspace emits a graph whose text-carrying spans
//! cover **every byte** of its source, each span quoting its bytes
//! verbatim at a declared offset. This module is the one law that
//! turns such a cover back into the document: write each span's bytes
//! at its offset, require the cover whole, admit an overlap only where
//! the data itself declares it, and prove the result against the
//! source digest the graph states. It is format-neutral on purpose —
//! nothing here knows a heading from a mail header — so every codec
//! decodes under one law and two formats cannot drift apart on it.
//! `purrdf-markdown` (its specification's §11.3) is the first emitter;
//! the next structured format reuses this module unchanged.
//!
//! # The overlap law
//!
//! The spans of a lawful cover are pairwise disjoint with one declared
//! exception: a continuation reaches back into the piece it
//! [`continues`](VerbatimSpan::continues), and there the shared bytes
//! must agree with **that piece's**. An **undeclared** overlap is
//! refused even when its bytes agree, because agreement is no evidence
//! of structure — two spans sliced from the same file agree wherever
//! they overlap, so a byte-agreement test would pass on exactly the
//! covers that are defective. The declaration is the data's own edge:
//! the exception is stated by the producer, never inferred by the
//! reader.
//!
//! # Bounds
//!
//! A cover arrives as attacker-influenced claims, so the decoder's
//! allocation is bounded by its **input**, never by what the cover
//! declares: a `byte_length` no span set could cover is refused before
//! a buffer exists, and the buffer then built is append-only — every
//! output byte is written exactly once, from a slice already resident
//! in memory.

use crate::ContentDigest;

/// One text-carrying span of a cover: the bytes at
/// `byte_start..byte_end`, quoted verbatim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerbatimSpan<'a> {
    /// The first byte of the span.
    pub byte_start: u64,
    /// One past the last byte of the span.
    pub byte_end: u64,
    /// The span's bytes, exactly as the cover carries them.
    pub text: &'a str,
    /// The byte span of the piece this one continues, when the cover
    /// declares one: the one declaration under which a partial overlap
    /// is lawful.
    pub continues: Option<(u64, u64)>,
}

/// Why a cover could not be decoded back into a document.
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
    /// A run of bytes no span covers. The cover is an index, not a
    /// codec's output, or a claim of it is missing.
    UncoveredRange {
        /// The first uncovered byte.
        byte_start: u64,
        /// One past the last uncovered byte of the run.
        byte_end: u64,
    },
    /// Two spans share bytes and the cover declares no `continues`
    /// edge that answers for them — no edge at all, an edge naming a
    /// span the cover does not carry, or an edge naming one that does
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
    /// The rebuilt document does not digest to the digest the cover
    /// states of its source.
    DigestMismatch {
        /// The digest the cover states, as lowercase hex.
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

/// The document back from its cover: writes each span's bytes at its
/// offset, requires every byte covered, admits an overlap only where a
/// `continues` edge declares it and its bytes agree, and proves the
/// result against the stated source digest.
///
/// The spans may come in any order — a graph states no order — and the
/// answer does not depend on theirs. Empty spans cover nothing and are
/// ignored; a codec never emits one, and refusing one would refuse a
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
    // below is bounded by the spans in hand — a `byte_length` past
    // their total is refused as the uncovered run it is.
    //
    // An overlap is answered against the span the `continues` edge
    // **declares**, never against whichever span happens to hold the
    // frontier. The two part company on a real chain: two pieces of a
    // split can end at the same cut, and the continuation of the later
    // one then overlaps bytes the frontier credits to the earlier. A
    // lawful producer's declared piece covers the whole overlap — a
    // continuation starts inside the piece it continues — so the
    // declared piece is the one span the shared bytes can always be
    // held against, and a declaration that does not cover them is no
    // answer for them and refuses exactly as no declaration does.
    //
    // The sorted vector is its own index: a declared piece is found by
    // binary search over the very ordering the walk reads, so the
    // lookup costs no second structure and nothing is built for the
    // common cover — the one with no overlap at all.
    let find = |key: (u64, u64)| -> Option<&VerbatimSpan<'_>> {
        ordered
            .binary_search_by_key(&key, |s| (s.byte_start, s.byte_end))
            .ok()
            .map(|i| ordered[i])
    };
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
                .and_then(find)
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
            // One slice comparison decides — the compiler lowers it to
            // a memcmp — and only a refusal pays to locate the byte.
            if of_continuation != of_piece {
                let differs = of_continuation
                    .iter()
                    .zip(of_piece)
                    .position(|(a, b)| a != b)
                    .expect("the slices differ, so some byte does");
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

    // The cover is proven, so the walk below appends every byte exactly
    // once: each span contributes the suffix past the bytes already
    // written — its whole text at a meet, the unshared tail at a
    // declared overlap, nothing when it lies inside what is written —
    // so there is no zero-fill pass and no byte is copied twice. The
    // length fits an allocation because the cover equals it: every
    // covered byte is a byte of some span's text, and those texts are
    // slices already resident in memory.
    let capacity =
        usize::try_from(byte_length).expect("a proven cover is at most the resident spans' bytes");
    let mut bytes = Vec::with_capacity(capacity);
    for span in &ordered {
        let written = bytes.len() as u64;
        if span.byte_end > written {
            let skip = (written - span.byte_start) as usize;
            bytes.extend_from_slice(&span.text.as_bytes()[skip..]);
        }
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

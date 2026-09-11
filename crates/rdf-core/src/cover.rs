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
    /// spans can produce: every span's text is UTF-8 whole, every
    /// admitted overlap agrees with a strictly earlier piece, and so
    /// every appended tail begins on a scalar boundary. It is kept as
    /// the typed last line — never an `expect` — so a defect in that
    /// argument would fail loudly and by name rather than by panic.
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
    // The text is inside the sort key so the order is total: two spans
    // over one range with different texts — no lawful cover has them,
    // but a hostile one may — sort the same way on every run and every
    // host, and the walk below therefore accepts or refuses such a
    // cover deterministically rather than by allocation order.
    ordered.sort_unstable_by_key(|s| (s.byte_start, s.byte_end, s.text));

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
    // The first span of the equal-range group, so a duplicated range is
    // answered by the same representative on every run.
    let find = |key: (u64, u64)| -> Option<&VerbatimSpan<'_>> {
        let at = ordered.partition_point(|s| (s.byte_start, s.byte_end) < key);
        ordered
            .get(at)
            .filter(|s| (s.byte_start, s.byte_end) == key)
            .copied()
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
            // The declared piece must start STRICTLY before the
            // continuation — the split law's own geometry, and the
            // clause that closes self-reference: a span naming its own
            // range would be found, cover its own overlap, and agree
            // with itself byte for byte, laundering any overlap into
            // legality. Strictly-earlier makes self-declaration and
            // same-start declaration answer for nothing, and a
            // duplicated range is then always refused at its second
            // twin, deterministically.
            let declared = span
                .continues
                .and_then(find)
                .filter(|p| p.byte_start < span.byte_start && shared_end <= p.byte_end);
            let Some(piece) = declared else {
                return Err(ReconstructError::UndeclaredOverlap {
                    byte_start: span.byte_start,
                    byte_end: span.byte_end,
                    covered_start: covering.byte_start,
                    covered_end: covering.byte_end,
                });
            };
            // Both narrowings are bounded by a resident `&str`'s
            // length — `shared` by this span's text, `from + shared`
            // by the declared piece's, both via the length check
            // above — so neither can truncate on a 32-bit target.
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
            // Bounded by this span's own text: the proven cover keeps
            // `written` between `byte_start` and `byte_end`, whose
            // distance is the text's length, checked above.
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

#[cfg(test)]
mod tests {
    //! The refusal vectors of the cover law, each executed beside the
    //! neighbouring input that still succeeds — the workspace refusal
    //! discipline, applied where the law lives so a future codec crate
    //! and a kernel-only refactor both get the signal from this crate's
    //! own suite. The graph-level vectors — spans extracted from parsed
    //! triples — stay with the Markdown codec that emits them.

    use super::*;

    /// A lawful cover of `"ab\ncd"`: two spans meeting at byte 2.
    fn covered() -> (u64, ContentDigest, [VerbatimSpan<'static>; 2]) {
        let text = "ab\ncd";
        (
            text.len() as u64,
            ContentDigest::of(text.as_bytes()),
            [
                VerbatimSpan {
                    byte_start: 0,
                    byte_end: 2,
                    text: "ab",
                    continues: None,
                },
                VerbatimSpan {
                    byte_start: 2,
                    byte_end: 5,
                    text: "\ncd",
                    continues: None,
                },
            ],
        )
    }

    #[test]
    fn a_full_cover_decodes_and_a_missing_span_names_the_uncovered_run() {
        let (len, digest, spans) = covered();
        assert_eq!(reconstruct(len, &digest, &spans).as_deref(), Ok("ab\ncd"));
        assert_eq!(
            reconstruct(len, &digest, &spans[..1]),
            Err(ReconstructError::UncoveredRange {
                byte_start: 2,
                byte_end: 5,
            })
        );
        assert_eq!(
            reconstruct(len, &digest, &spans[1..]),
            Err(ReconstructError::UncoveredRange {
                byte_start: 0,
                byte_end: 2,
            })
        );
        // A declared length past the cover is the uncovered run it is —
        // found before any buffer exists, so a graph cannot buy an
        // allocation with a number.
        assert_eq!(
            reconstruct(u64::MAX, &digest, &spans),
            Err(ReconstructError::UncoveredRange {
                byte_start: 5,
                byte_end: u64::MAX,
            })
        );
    }

    #[test]
    fn an_overlap_is_refused_undeclared_and_admitted_declared_even_though_the_bytes_agree_either_way()
     {
        let text = "abcdef";
        let len = text.len() as u64;
        let digest = ContentDigest::of(text.as_bytes());
        let piece = VerbatimSpan {
            byte_start: 0,
            byte_end: 4,
            text: "abcd",
            continues: None,
        };
        let agreeing = |continues| VerbatimSpan {
            byte_start: 2,
            byte_end: 6,
            text: "cdef",
            continues,
        };
        // Undeclared: the shared bytes agree — they came from the same
        // file — and that is exactly why agreement alone must admit
        // nothing.
        assert_eq!(
            reconstruct(len, &digest, &[piece, agreeing(None)]),
            Err(ReconstructError::UndeclaredOverlap {
                byte_start: 2,
                byte_end: 6,
                covered_start: 0,
                covered_end: 4,
            })
        );
        // A declaration naming a span the graph does not carry answers for
        // nothing.
        assert_eq!(
            reconstruct(len, &digest, &[piece, agreeing(Some((0, 3)))]),
            Err(ReconstructError::UndeclaredOverlap {
                byte_start: 2,
                byte_end: 6,
                covered_start: 0,
                covered_end: 4,
            })
        );
        // The neighbour: the same overlap, declared to the piece that
        // covers it, decodes.
        assert_eq!(
            reconstruct(len, &digest, &[piece, agreeing(Some((0, 4)))]).as_deref(),
            Ok("abcdef")
        );
        // And a declared continuation that disagrees about a shared byte
        // is named at that byte.
        let disagreeing = VerbatimSpan {
            byte_start: 2,
            byte_end: 6,
            text: "cXef",
            continues: Some((0, 4)),
        };
        assert_eq!(
            reconstruct(len, &digest, &[piece, disagreeing]),
            Err(ReconstructError::OverlapDisagreement { at: 3 })
        );
    }

    #[test]
    fn a_chain_whose_pieces_share_a_cut_decodes_by_its_declarations() {
        // The geometry the write-side check caught on a real split: two
        // pieces end at one cut, and the continuation of the later one
        // overlaps bytes the sorted walk credits to the earlier. The
        // declaration — not the walk's frontier — is what answers.
        let text = "ab\ncd\nef";
        let len = text.len() as u64;
        let digest = ContentDigest::of(text.as_bytes());
        let spans = [
            VerbatimSpan {
                byte_start: 0,
                byte_end: 2,
                text: "ab",
                continues: None,
            },
            VerbatimSpan {
                byte_start: 1,
                byte_end: 5,
                text: "b\ncd",
                continues: Some((0, 2)),
            },
            VerbatimSpan {
                byte_start: 3,
                byte_end: 5,
                text: "cd",
                continues: Some((1, 5)),
            },
            VerbatimSpan {
                byte_start: 4,
                byte_end: 8,
                text: "d\nef",
                continues: Some((3, 5)),
            },
        ];
        assert_eq!(reconstruct(len, &digest, &spans).as_deref(), Ok(text));
    }

    #[test]
    fn a_span_answers_for_its_own_shape_before_the_cover_is_asked() {
        let (len, digest, [a, b]) = covered();
        let mut inverted = a;
        inverted.byte_start = 2;
        inverted.byte_end = 0;
        assert_eq!(
            reconstruct(len, &digest, &[inverted, b]),
            Err(ReconstructError::InvertedSpan {
                byte_start: 2,
                byte_end: 0,
            })
        );
        let mut past = b;
        past.byte_end = 6;
        assert_eq!(
            reconstruct(len, &digest, &[a, past]),
            Err(ReconstructError::SpanOutOfBounds {
                byte_start: 2,
                byte_end: 6,
                byte_length: 5,
            })
        );
        let mut short = b;
        short.text = "\ncd extra";
        assert_eq!(
            reconstruct(len, &digest, &[a, short]),
            Err(ReconstructError::SpanLengthMismatch {
                byte_start: 2,
                byte_end: 5,
                text_bytes: 9,
            })
        );
        // The neighbour of all three: the unmutated spans still decode.
        assert_eq!(reconstruct(len, &digest, &[a, b]).as_deref(), Ok("ab\ncd"));
    }

    #[test]
    fn a_cover_that_rebuilds_the_wrong_bytes_is_named_by_its_digest() {
        let (len, _, [a, b]) = covered();
        let other = ContentDigest::of(b"ab\ncD");
        assert_eq!(
            reconstruct(len, &other, &[a, b]),
            Err(ReconstructError::DigestMismatch {
                expected: other.to_hex(),
                reconstructed: ContentDigest::of(b"ab\ncd").to_hex(),
            })
        );
        // The neighbour: the digest the graph would actually state.
        let stated = ContentDigest::of(b"ab\ncd");
        assert!(reconstruct(len, &stated, &[a, b]).is_ok());
    }

    #[test]
    fn an_empty_document_is_a_lawful_graph_of_nothing() {
        assert_eq!(
            reconstruct(0, &ContentDigest::of(b""), &[]).as_deref(),
            Ok("")
        );
        // The neighbour in the other direction: a declared length with no
        // spans at all is one uncovered run.
        assert_eq!(
            reconstruct(1, &ContentDigest::of(b"x"), &[]),
            Err(ReconstructError::UncoveredRange {
                byte_start: 0,
                byte_end: 1,
            })
        );
    }

    /// The overlap law cannot be laundered by self-reference. A span
    /// naming its own range as the piece it continues would be found
    /// by the lookup, cover its own overlap, and agree with itself
    /// byte for byte — so the declared piece must start strictly
    /// earlier, and a self-declaration, a same-start declaration, and
    /// a duplicated range all answer for nothing. The smuggling this
    /// closes is real: through the self-gate, a continuation could
    /// split a scalar and rebuild bytes no lawful cover states.
    #[test]
    fn a_continuation_cannot_declare_itself_and_a_split_scalar_cannot_be_smuggled() {
        let digest = ContentDigest::of(b"abcd");
        let piece = VerbatimSpan {
            byte_start: 0,
            byte_end: 2,
            text: "ab",
            continues: None,
        };
        // The laundering shape: the later span contradicts the earlier
        // about byte 1 and "declares" the overlap against itself.
        let selfish = VerbatimSpan {
            byte_start: 1,
            byte_end: 4,
            text: "Xcd",
            continues: Some((1, 4)),
        };
        assert_eq!(
            reconstruct(4, &digest, &[piece, selfish]),
            Err(ReconstructError::UndeclaredOverlap {
                byte_start: 1,
                byte_end: 4,
                covered_start: 0,
                covered_end: 2,
            })
        );
        // A same-start declaration is no answer either.
        let same_start = VerbatimSpan {
            byte_start: 0,
            byte_end: 4,
            text: "abcd",
            continues: Some((0, 2)),
        };
        assert_eq!(
            reconstruct(4, &digest, &[piece, same_start]),
            Err(ReconstructError::UndeclaredOverlap {
                byte_start: 0,
                byte_end: 4,
                covered_start: 0,
                covered_end: 2,
            })
        );
        // The scalar smuggle the self-gate would have admitted: a
        // continuation whose tail begins mid-scalar, its stated digest
        // matching the invalid bytes it would rebuild. Refused as the
        // undeclared overlap it is, before a byte exists.
        let smuggle = VerbatimSpan {
            byte_start: 1,
            byte_end: 4,
            text: "\u{e9}!",
            continues: Some((1, 4)),
        };
        let smuggled = ContentDigest::of(&[0x61, 0x62, 0xA9, 0x21]);
        assert_eq!(
            reconstruct(4, &smuggled, &[piece, smuggle]),
            Err(ReconstructError::UndeclaredOverlap {
                byte_start: 1,
                byte_end: 4,
                covered_start: 0,
                covered_end: 2,
            })
        );
        // The honest neighbour: the same overlap declared against the
        // strictly earlier piece it agrees with, decoding whole.
        let lawful = VerbatimSpan {
            byte_start: 1,
            byte_end: 4,
            text: "bcd",
            continues: Some((0, 2)),
        };
        assert_eq!(
            reconstruct(4, &digest, &[piece, lawful]).as_deref(),
            Ok("abcd")
        );
        // With every route refused earlier, the UTF-8 refusal is the
        // typed last line rather than a panic, and its finding renders.
        assert_eq!(
            ReconstructError::InvalidUtf8 { valid_up_to: 2 }.to_string(),
            "reconstructed bytes are not UTF-8 after byte 2"
        );
    }
}

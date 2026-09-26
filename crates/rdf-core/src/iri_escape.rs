// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE transcription of which scalars an `IRIREF` writer must escape — the
//! egress mirror of [`purrdf_iri::terminals::is_iriref_forbidden`].
//!
//! # Why egress needs its own name for the same law
//!
//! Ingress and egress ask two different questions about one production:
//!
//! ```text
//! IRIREF ::= '<' ( [^#x00-#x20<>"{}|^`\] | UCHAR )* '>'
//! ```
//!
//! (SPARQL 1.2 §19.8 / Turtle 1.2 §6.5 `[18t]`.) A *parser* asks **may this
//! scalar stand here raw?** — that is [`purrdf_iri::terminals::is_iriref_forbidden`],
//! and it is exact in both directions because it decides where the body ends.
//! A *writer* asks the weaker question **must this scalar ride as a `UCHAR`?**,
//! and the production answers only half of it: every forbidden scalar must ride
//! escaped, but a permitted scalar MAY ride escaped too, because `UCHAR` is an
//! alternative of the body at every position. Over-escaping is therefore
//! lossless — any conforming parser decodes `A` back to `A` — which is why
//! a writer is free to escape a wider set, and why a divergence between two
//! writers is invisible until someone diffs their bytes.
//!
//! That freedom is exactly how this law came to be transcribed FIVE times in
//! this workspace. The predicate below is the one spelling. The emission is
//! here too: every escaped scalar is at most U+009F, so every writer's `UCHAR`
//! is the same `\u00XX` in upper-case hex, and [`push_escaped`] (for a writer
//! appending to a text sink) and [`escape`] (for one that borrows a clean
//! value) are the one implementation of it. Both are one pass of
//! [`find_first_candidate`], the chunked scan for the bytes that can begin an
//! escaped scalar, copying every run between two escapes whole.
//!
//! # The one scalar range egress adds, and why
//!
//! [`is_iriref_escape_required`] is [`purrdf_iri::terminals::is_iriref_forbidden`]
//! widened by exactly `[#x7F-#x9F]` — DEL and the C1 control block. Those are
//! **lawful raw** in an `IRIREF` body and the ingress predicate rightly permits
//! them; they are escaped on the way out because this workspace's serialized
//! output is embedded verbatim inside an XML text node by the CL-dialect
//! carrier, and an XML processor normalizes or replaces raw C1 code points on
//! read. Escaping them is what makes serialize-then-parse a round trip across
//! that carrier. Nothing else is added: U+00A0 NO-BREAK SPACE, U+2000-U+200A,
//! U+2028, U+2029 and U+3000 all ride VERBATIM, because they are `ucschar`
//! under RFC 3987 §2.2 and an `IRIREF` body is content rather than a token
//! boundary.
//!
//! The widening is written as the enumerated range it is, not as
//! [`char::is_control`]. The two happen to agree here only because
//! `char::is_control` is `[#x00-#x1F] ∪ [#x7F-#x9F]` and the first half is
//! already inside `[#x00-#x20]`; saying so with a Unicode property would leave
//! a reader unable to tell which half of the answer came from the grammar and
//! which from the carrier.

use crate::sink::TextOut;
use purrdf_iri::terminals::{ByteClass, byte_run_count};
use std::borrow::Cow;

/// DEL and the C1 control block, `[#x7F-#x9F]` — the single range this egress
/// law adds to the ingress production. Lawful raw inside an `IRIREF`, escaped
/// anyway so the bytes survive an XML text node.
const C1_AND_DEL: (char, char) = ('\u{7F}', '\u{9F}');

/// Whether an `IRIREF` writer must emit `c` as a `UCHAR` escape rather than raw.
///
/// This is the union of two enumerated sets and nothing else:
///
/// * [`purrdf_iri::terminals::is_iriref_forbidden`] — `[#x00-#x20]` (every C0
///   control **plus the SPACE**) and the nine reserved delimiters
///   ``< > " { } | ^ ` \``, which the grammar forbids raw;
/// * `C1_AND_DEL` — `[#x7F-#x9F]`, DEL and the C1 control block, which the
///   grammar PERMITS raw and which this workspace escapes anyway for XML
///   transport (see the [module docs](self)).
///
/// Every scalar at or above U+00A0 answers `false`: that includes the whole
/// non-ASCII Unicode whitespace block, which is content inside an `IRIREF` body
/// and must ride verbatim or the writers stop round-tripping.
///
/// ```
/// use purrdf_core::iri_escape::is_iriref_escape_required;
///
/// // The grammar's own exclusions.
/// assert!(is_iriref_escape_required(' '));
/// assert!(is_iriref_escape_required('<'));
/// assert!(is_iriref_escape_required('\u{0}'));
/// // The carrier's widening: lawful raw, escaped anyway.
/// assert!(is_iriref_escape_required('\u{7F}'));
/// assert!(is_iriref_escape_required('\u{9F}'));
/// // And the neighbour that must still ride verbatim.
/// assert!(!is_iriref_escape_required('\u{A0}'));
/// assert!(!is_iriref_escape_required('\u{3000}'));
/// assert!(!is_iriref_escape_required('a'));
/// ```
#[inline]
#[must_use]
pub const fn is_iriref_escape_required(c: char) -> bool {
    purrdf_iri::terminals::is_iriref_forbidden(c) || (C1_AND_DEL.0 <= c && c <= C1_AND_DEL.1)
}

/// The bytes that can begin a scalar [`is_iriref_escape_required`] answers
/// `true` for, as a class table: every ASCII byte whose scalar is escaped
/// (`[#x00-#x20]`, the nine delimiters and DEL), plus `0xC2`, the UTF-8 lead
/// byte of U+0080-U+00BF, the block that holds the C1 controls.
///
/// Derived from the predicate itself, so the scan and the law cannot drift: the
/// ASCII half is the predicate over every ASCII scalar, and the one lead byte is
/// the lead of the non-ASCII half (`C1_AND_DEL` lies wholly inside the two-byte
/// sequences led by `0xC2`, which a const assertion below pins).
const CANDIDATE_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0_u8;
    while b < 0x80 {
        if is_iriref_escape_required(b as char) {
            table[b as usize] = 1;
        }
        b += 1;
    }
    table[C1_LEAD as usize] = 1;
    table
};

/// The UTF-8 lead byte of every scalar in U+0080-U+00BF.
const C1_LEAD: u8 = 0xC2;

// Every non-ASCII scalar the law escapes is encoded with the lead byte
// `C1_LEAD`, so stopping at that byte (and at the ASCII members) stops at every
// scalar that can need an escape.
const _: () = {
    let (lo, hi) = C1_AND_DEL;
    assert!(lo as u32 == 0x7F && hi as u32 <= 0xBF);
};

const CANDIDATES: ByteClass<{ byte_run_count(&CANDIDATE_TABLE) }> =
    ByteClass::from_table(CANDIDATE_TABLE);

/// The offset of the first byte of `bytes` that can begin a scalar an `IRIREF`
/// writer must escape, or `None` when no scalar of `bytes` needs one.
///
/// A byte *candidate*, not a verdict: an ASCII member is always escaped, and at
/// the lead byte `0xC2` the scalar it begins is escaped only when it is a C1
/// control (U+0080-U+009F), so a caller decodes the scalar there and asks
/// [`is_iriref_escape_required`]. Every byte before the offset belongs to a
/// scalar that rides verbatim, and the offset is always a char boundary of a
/// `str`'s bytes (the class holds ASCII bytes and one lead byte).
///
/// The chunked kernel of [`purrdf_iri::terminals::ByteClass`], so the scan is
/// sixteen-byte packed compares on every target with a vector unit.
///
/// ```
/// use purrdf_core::iri_escape::find_first_candidate;
///
/// assert_eq!(find_first_candidate(b"https://example.org/a b"), Some(21));
/// assert_eq!(find_first_candidate(b"https://example.org/a"), None);
/// // `é` is U+00E9 (lead byte 0xC3): verbatim, not a candidate.
/// assert_eq!(find_first_candidate("caf\u{e9}".as_bytes()), None);
/// // U+0085 NEXT LINE is a C1 control; its lead byte is the candidate.
/// assert_eq!(find_first_candidate("a\u{85}".as_bytes()), Some(1));
/// ```
#[inline(never)]
#[must_use]
pub fn find_first_candidate(bytes: &[u8]) -> Option<usize> {
    CANDIDATES.find_first(bytes)
}

/// Upper-case hex digits, for the `\u00XX` spelling.
const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Append the `UCHAR` of `ch`, a scalar [`is_iriref_escape_required`] answers
/// `true` for and hence at most U+009F: `\u00XX` in upper-case hex, the bytes
/// `write!(out, "\\u{:04X}", ch as u32)` produces.
fn push_uchar<W: TextOut + ?Sized>(ch: char, out: &mut W) {
    let v = u32::from(ch);
    debug_assert!(v <= 0x9F, "only scalars up to U+009F are escaped");
    out.push_str("\\u00");
    out.push(char::from(HEX_UPPER[((v >> 4) & 0xF) as usize]));
    out.push(char::from(HEX_UPPER[(v & 0xF) as usize]));
}

/// Append `iri[at..]` escaped, copying each run between two escapes whole.
///
/// `at` is a char boundary. A candidate that needs no escape (a non-C1 scalar
/// led by `0xC2`, such as U+00A0) stays inside the run it interrupts.
fn push_escaped_from<W: TextOut + ?Sized>(iri: &str, mut at: usize, out: &mut W) {
    let bytes = iri.as_bytes();
    let mut run_start = at;
    while let Some(offset) = find_first_candidate(&bytes[at..]) {
        let hit = at + offset;
        let ch = iri[hit..]
            .chars()
            .next()
            .expect("a candidate offset is a char boundary");
        at = hit + ch.len_utf8();
        if is_iriref_escape_required(ch) {
            out.push_str(&iri[run_start..hit]);
            push_uchar(ch, out);
            run_start = at;
        }
    }
    out.push_str(&iri[run_start..]);
}

/// Append `iri` to `out` as an `IRIREF` body: every scalar
/// [`is_iriref_escape_required`] answers `true` for rides as its `\u00XX`
/// `UCHAR`, and every other scalar rides verbatim.
///
/// The emission of the canonical N-Quads writer and the entailment surface
/// writer; one scan with bulk copies of the clean runs.
///
/// ```
/// use purrdf_core::iri_escape::push_escaped;
///
/// let mut out = String::new();
/// push_escaped("urn:ex:a b<\u{85}>\u{a0}", &mut out);
/// assert_eq!(out, "urn:ex:a\\u0020b\\u003C\\u0085\\u003E\u{a0}");
/// ```
pub fn push_escaped<W: TextOut + ?Sized>(iri: &str, out: &mut W) {
    push_escaped_from(iri, 0, out);
}

/// `iri` as an `IRIREF` body, borrowed when no scalar needs an escape.
///
/// The same bytes as [`push_escaped`]. The scan that finds the first escaped
/// scalar is the scan that emits, so a value that needs escaping is still read
/// once: its clean prefix is copied whole and the emission continues from the
/// first escape.
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_core::iri_escape::escape;
///
/// assert!(matches!(escape("https://example.org/caf\u{e9}"), Cow::Borrowed(_)));
/// assert_eq!(escape("urn:ex:a b"), "urn:ex:a\\u0020b");
/// ```
#[must_use]
pub fn escape(iri: &str) -> Cow<'_, str> {
    let bytes = iri.as_bytes();
    let mut at = 0;
    while let Some(offset) = find_first_candidate(&bytes[at..]) {
        let hit = at + offset;
        let ch = iri[hit..]
            .chars()
            .next()
            .expect("a candidate offset is a char boundary");
        if is_iriref_escape_required(ch) {
            let mut out = String::with_capacity(iri.len() + 8);
            out.push_str(&iri[..hit]);
            push_escaped_from(iri, hit, &mut out);
            return Cow::Owned(out);
        }
        at = hit + ch.len_utf8();
    }
    Cow::Borrowed(iri)
}

#[cfg(test)]
mod tests {
    use super::{
        CANDIDATE_TABLE, escape, find_first_candidate, is_iriref_escape_required, push_escaped,
    };
    use std::borrow::Cow;
    use std::fmt::Write as _;

    /// Every Unicode scalar value, in order.
    fn all_scalars() -> impl Iterator<Item = char> {
        (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
    }

    /// The spelling the five writers carried before they were collapsed onto
    /// one predicate, transcribed verbatim from their match arms.
    ///
    /// This is the byte-determinism proof the collapse rests on: the goldens
    /// and frozen vectors were produced by this function, so if the new
    /// predicate disagrees with it at a single scalar, a golden changes.
    fn the_spelling_the_writers_carried(c: char) -> bool {
        matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\')
            || c.is_control()
            || c == ' '
    }

    #[test]
    fn the_collapse_changes_no_emitted_byte() {
        for c in all_scalars() {
            assert_eq!(
                is_iriref_escape_required(c),
                the_spelling_the_writers_carried(c),
                "{c:?}"
            );
        }
    }

    #[test]
    fn egress_is_ingress_widened_by_exactly_del_and_c1() {
        for c in all_scalars() {
            // Everything the grammar forbids must be escaped: a writer that
            // emitted one raw would mint a document no parser reads back.
            if purrdf_iri::terminals::is_iriref_forbidden(c) {
                assert!(is_iriref_escape_required(c), "{c:?}");
            }
            // And the difference is one enumerated range, stated as a total
            // function so neither side can drift.
            assert_eq!(
                is_iriref_escape_required(c) && !purrdf_iri::terminals::is_iriref_forbidden(c),
                matches!(c, '\u{7F}'..='\u{9F}'),
                "{c:?}"
            );
        }
    }

    #[test]
    fn non_ascii_whitespace_rides_verbatim() {
        // The refusal side would be a silent round-trip break rather than a
        // parse error, so it is pinned scalar by scalar.
        for c in [
            '\u{A0}', '\u{1680}', '\u{2000}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}',
            '\u{205F}', '\u{3000}',
        ] {
            assert!(c.is_whitespace(), "{c:?}");
            assert!(!is_iriref_escape_required(c), "{c:?}");
        }
        // The valid neighbour on the other side of the boundary: U+009F is the
        // last scalar escaped and U+00A0 the first that is not, so this is an
        // exact edge and not a blanket refusal of the Latin-1 supplement.
        assert!(is_iriref_escape_required('\u{9F}'));
        assert!(!is_iriref_escape_required('\u{A0}'));
        assert!(!is_iriref_escape_required('\u{E9}'));
    }

    #[test]
    fn a_clean_production_iri_needs_no_escape_at_all() {
        // The case that matters most for the writers' fast paths: every scalar
        // of an ordinary IRI answers `false`, so the borrow-preserving scan in
        // the native serializer keeps borrowing.
        for c in "https://example.org/vocab/Thing#frag?q=1&r=2".chars() {
            assert!(!is_iriref_escape_required(c), "{c:?}");
        }
    }

    /// The per-scalar writer `push_escaped` replaced: the loop the canonical
    /// N-Quads and entailment writers carried, kept as the oracle.
    fn reference(iri: &str) -> String {
        let mut out = String::new();
        for ch in iri.chars() {
            if is_iriref_escape_required(ch) {
                let _ = write!(out, "\\u{:04X}", ch as u32);
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            crate::test_rng::splitmix64_next(&mut self.0)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

    /// Every ASCII scalar, every scalar with a `0xC2` lead, and non-ASCII
    /// neighbours in every UTF-8 width.
    fn alphabet() -> Vec<char> {
        let mut out: Vec<char> = (0_u8..0x80).map(char::from).collect();
        out.extend(('\u{80}'..='\u{BF}').chain([
            '\u{C0}',
            '\u{E9}',
            '\u{FF}',
            '\u{100}',
            '\u{2028}',
            '\u{3000}',
            '\u{FFFD}',
            '\u{1F408}',
            '\u{10FFFF}',
        ]));
        out
    }

    #[test]
    fn candidate_table_is_the_ascii_law_and_one_lead_byte() {
        for b in 0..=u8::MAX {
            let expected = if b < 0x80 {
                is_iriref_escape_required(char::from(b))
            } else {
                b == 0xC2
            };
            assert_eq!(CANDIDATE_TABLE[usize::from(b)] != 0, expected, "{b:#04X}");
        }
        // Every escaped scalar begins with a candidate byte.
        for c in all_scalars().filter(|&c| is_iriref_escape_required(c)) {
            let mut buf = [0_u8; 4];
            let lead = c.encode_utf8(&mut buf).as_bytes()[0];
            assert!(CANDIDATE_TABLE[usize::from(lead)] != 0, "{c:?}");
        }
    }

    #[test]
    fn push_escaped_and_escape_agree_with_the_per_scalar_writer() {
        let alphabet = alphabet();
        let mut rng = SplitMix(0x001E_1E5C_A9E0_0001);
        let mut borrowed = 0_usize;
        let mut owned = 0_usize;
        for len in (0..=70).chain([127, 128, 129, 255, 1000]) {
            for _ in 0..40 {
                let iri: String = (0..len)
                    .map(|_| {
                        if rng.below(6) == 0 {
                            alphabet[rng.below(alphabet.len())]
                        } else {
                            'a'
                        }
                    })
                    .collect();
                let expected = reference(&iri);
                let mut pushed = String::from("prefix:");
                push_escaped(&iri, &mut pushed);
                assert_eq!(&pushed["prefix:".len()..], expected, "{iri:?}");
                let escaped = escape(&iri);
                assert_eq!(escaped, expected, "{iri:?}");
                match escaped {
                    Cow::Borrowed(_) => {
                        borrowed += 1;
                        assert_eq!(expected, iri, "borrowed only when unchanged");
                    }
                    Cow::Owned(_) => {
                        owned += 1;
                        assert_ne!(expected, iri, "owned only when changed");
                    }
                }
                if let Some(i) = find_first_candidate(iri.as_bytes()) {
                    assert!(iri.is_char_boundary(i), "{iri:?} at {i}");
                }
            }
        }
        // Non-vacuity: both arms were taken.
        assert!(borrowed > 0 && owned > 0, "{borrowed} {owned}");
    }

    #[test]
    fn a_candidate_that_needs_no_escape_does_not_split_the_run() {
        // U+00A0 is led by 0xC2 and rides verbatim, so the value is borrowed.
        assert!(matches!(escape("urn:ex:\u{a0}x"), Cow::Borrowed(_)));
        // Its neighbour U+009F is escaped.
        assert_eq!(escape("urn:ex:\u{9f}x"), "urn:ex:\\u009Fx");
    }

    #[test]
    fn the_predicate_is_usable_in_const_context() {
        const { assert!(is_iriref_escape_required(' ')) }
        const { assert!(is_iriref_escape_required('\\')) }
        const { assert!(is_iriref_escape_required('\u{7F}')) }
        const { assert!(!is_iriref_escape_required('\u{A0}')) }
    }
}

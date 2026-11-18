// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE transcription of which scalars an `IRIREF` writer must escape — the
//! egress mirror of [`crate::terminals::is_iriref_forbidden`].
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
//! scalar stand here raw?** — that is [`crate::terminals::is_iriref_forbidden`],
//! and it is exact in both directions because it decides where the body ends.
//! A *writer* asks the weaker question **must this scalar ride as a `UCHAR`?**,
//! and the production answers only half of it: every forbidden scalar must ride
//! escaped, but a permitted scalar MAY ride escaped too, because `UCHAR` is an
//! alternative of the body at every position. Over-escaping is therefore
//! lossless — any conforming parser decodes `A` back to `A` — which is why
//! a writer is free to escape a wider set, and why a divergence between two
//! writers is invisible until someone diffs their bytes.
//!
//! That freedom is why the law needs one spelling: two writers that each chose
//! a defensible escape set would emit different bytes for the same IRI, and a
//! canonical form or a golden file would notice before any parser did. The
//! predicate below is that spelling, in the lexical layer every writer
//! reaches. The emission is here too: every escaped scalar is at most U+009F, so every writer's `UCHAR`
//! is the same `\u00XX` in upper-case hex, and [`push_escaped`] (for a writer
//! appending to a text sink) and [`escape`] (for one that borrows a clean
//! value) are the one implementation of it. Both are one pass of
//! [`find_first_candidate`], the chunked scan for the bytes that can begin an
//! escaped scalar, copying every run between two escapes whole.
//!
//! # Two escape sets, one per carrier
//!
//! An `IRIREF` writer is free to escape more than the production forbids, so
//! which set a writer uses is a choice of **carrier**, and each carrier names
//! the rule it follows:
//!
//! * [`Carrier::Canonical`] is exactly the production's exclusions: `[#x00-#x20]`
//!   and the nine delimiters ``< > " { } | ^ ` \``, the set
//!   [`crate::terminals::is_iriref_forbidden`] answers. Its rule is the `IRIREF`
//!   production of RDF 1.2 N-Quads and N-Triples (section 2, grammar). The canonical
//!   forms of those two languages (section 3 of each) say nothing about escaping in
//!   an IRI, so the canonical document escapes what the grammar requires and nothing
//!   else: DEL and the C1 block, which `IRIREF` admits and which match the XML 1.1
//!   `Char` production, ride raw, exactly as U+00A0 does.
//! * [`Carrier::Xml`] is the same set widened by exactly `[#x7F-#x9F]`, DEL and the
//!   C1 control block. Those are lawful raw in an `IRIREF` body and are escaped
//!   because serialized output of this carrier is embedded verbatim in an XML text
//!   node, and an XML processor normalizes or replaces raw C1 code points on read.
//!   Escaping them is what makes serialize-then-parse a round trip across that
//!   carrier. Its rule is XML 1.0 section 2.2 (`Char`, with the C1 controls
//!   discouraged) and XML 1.1 section 2.11 (NEL rewriting).
//!
//! Nothing else is added to either: U+00A0 NO-BREAK SPACE, U+2000-U+200A, U+2028,
//! U+2029 and U+3000 ride VERBATIM in both, because they are `ucschar` under RFC
//! 3987 section 2.2 and an `IRIREF` body is content rather than a token boundary.
//! The widening is written as the enumerated range it is, not as
//! [`char::is_control`], so a reader can tell which half of the answer came from
//! the grammar and which from the carrier.
//!
//! The `_for` forms ([`is_iriref_escape_required_for`], [`push_escaped_for`],
//! [`escape_for`]) name the carrier. [`crate::term_syntax::write_iri`] and every
//! canonical or hashed form use [`Carrier::Canonical`]; the Turtle emitter and
//! renderer of `purrdf-core`, the N-Triples writer of the native codecs and the
//! SHACL path renderer feed the CL-dialect carrier and name [`Carrier::Xml`]. The
//! unqualified functions are the [`Carrier::Xml`] set, kept for existing callers;
//! new code names its carrier. Both carriers share one candidate scan,
//! [`find_first_candidate`], whose stops are a superset of either set, so a
//! canonical writer decides the scalar at the `0xC2` lead byte and keeps the C1
//! block raw.

use crate::terminals::{ByteClass, byte_run_count};
use crate::text_out::TextOut;
use std::borrow::Cow;

/// DEL and the C1 control block, `[#x7F-#x9F]` — the single range the
/// [`Carrier::Xml`] set adds to the production's exclusions. Lawful raw inside
/// an `IRIREF`, escaped anyway so the bytes survive an XML text node.
const C1_AND_DEL: (char, char) = ('\u{7F}', '\u{9F}');

/// The carrier an `IRIREF` body is escaped for. Each is a specification's
/// constraint, not a style; see the [module docs](self).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Carrier {
    /// The canonical form of RDF 1.2 N-Quads and N-Triples: exactly the scalars
    /// the `IRIREF` production excludes, DEL and the C1 block raw.
    Canonical,
    /// Text embedded in an XML document: the production's exclusions and the
    /// range `[#x7F-#x9F]`.
    Xml,
}

/// Whether an `IRIREF` writer for `carrier` must emit `c` as a `UCHAR` escape
/// rather than raw.
///
/// [`Carrier::Canonical`] answers [`crate::terminals::is_iriref_forbidden`] and
/// nothing else; [`Carrier::Xml`] adds DEL and the C1 control block.
///
/// ```
/// use purrdf_lex::iri_escape::{Carrier, is_iriref_escape_required_for};
///
/// assert!(is_iriref_escape_required_for(' ', Carrier::Canonical));
/// assert!(is_iriref_escape_required_for('<', Carrier::Canonical));
/// // DEL and the C1 block: raw in the canonical form, escaped for XML.
/// assert!(!is_iriref_escape_required_for('\u{7F}', Carrier::Canonical));
/// assert!(!is_iriref_escape_required_for('\u{85}', Carrier::Canonical));
/// assert!(is_iriref_escape_required_for('\u{85}', Carrier::Xml));
/// // The neighbour past the C1 block is raw in both.
/// assert!(!is_iriref_escape_required_for('\u{A0}', Carrier::Canonical));
/// assert!(!is_iriref_escape_required_for('\u{A0}', Carrier::Xml));
/// ```
#[inline]
#[must_use]
pub const fn is_iriref_escape_required_for(c: char, carrier: Carrier) -> bool {
    crate::terminals::is_iriref_forbidden(c)
        || matches!(carrier, Carrier::Xml) && C1_AND_DEL.0 <= c && c <= C1_AND_DEL.1
}

/// Whether an `IRIREF` writer for [`Carrier::Xml`] must emit `c` as a `UCHAR`
/// escape rather than raw: [`is_iriref_escape_required_for`] with that carrier.
///
/// This is the union of two enumerated sets and nothing else:
///
/// * [`crate::terminals::is_iriref_forbidden`] — `[#x00-#x20]` (every C0
///   control **plus the SPACE**) and the nine reserved delimiters
///   ``< > " { } | ^ ` \``, which the grammar forbids raw;
/// * `C1_AND_DEL` — `[#x7F-#x9F]`, DEL and the C1 control block, which the
///   grammar PERMITS raw and which this carrier escapes for XML transport (see
///   the [module docs](self)).
///
/// Every scalar at or above U+00A0 answers `false`: that includes the whole
/// non-ASCII Unicode whitespace block, which is content inside an `IRIREF` body
/// and must ride verbatim or the writers stop round-tripping.
///
/// ```
/// use purrdf_lex::iri_escape::is_iriref_escape_required;
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
    is_iriref_escape_required_for(c, Carrier::Xml)
}

/// The bytes that can begin a scalar [`is_iriref_escape_required`] answers
/// `true` for, as a class table: every ASCII byte whose scalar is escaped
/// (`[#x00-#x20]`, the nine delimiters and DEL), plus `0xC2`, the UTF-8 lead
/// byte of U+0080-U+00BF, the block that holds the C1 controls.
///
/// Derived from the predicate itself, so the scan and the law cannot drift: the
/// ASCII half is the predicate over every ASCII scalar, and the one lead byte is
/// the lead of the non-ASCII half (`C1_AND_DEL` lies wholly inside the two-byte
/// sequences led by `0xC2`, which a const assertion below pins). This is the
/// [`Carrier::Xml`] set, a superset of [`Carrier::Canonical`]'s, so one scan
/// serves both: a canonical writer meets `0xC2` as a candidate that needs no
/// escape and keeps the run whole.
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
/// The chunked kernel of [`crate::terminals::ByteClass`], so the scan is
/// sixteen-byte packed compares on every target with a vector unit.
///
/// ```
/// use purrdf_lex::iri_escape::find_first_candidate;
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

/// Whether `b` is a byte [`find_first_candidate`] stops at: one that can begin
/// a scalar an `IRIREF` writer must escape.
///
/// The one-byte form of the same class, for a caller that has already found
/// the one position that matters (the first byte at which two IRIs differ) and
/// asks only about the bytes of its scalar: a table read per byte, with no
/// scan around it.
///
/// ```
/// use purrdf_lex::iri_escape::is_candidate;
///
/// assert!(is_candidate(b' ') && is_candidate(b'>') && is_candidate(0xC2));
/// assert!(!is_candidate(b'a') && !is_candidate(0xC3));
/// ```
#[inline]
#[must_use]
pub const fn is_candidate(b: u8) -> bool {
    CANDIDATE_TABLE[b as usize] != 0
}

/// Append the `UCHAR` of `ch`, a scalar [`is_iriref_escape_required`] answers
/// `true` for and hence at most U+009F: `\u00XX` in upper-case hex, the bytes
/// `write!(out, "\\u{:04X}", ch as u32)` produces.
fn push_uchar<W: TextOut + ?Sized>(ch: char, out: &mut W) {
    let v = u32::from(ch);
    debug_assert!(v <= 0x9F, "only scalars up to U+009F are escaped");
    let mut digits = [0u8; 2];
    out.push_str("\\u00");
    out.push_str(
        purrdf_hash::hex::encode_upper_to_slice(&[v as u8], &mut digits)
            .expect("one byte renders in two digits"),
    );
}

/// Append `iri[at..]` escaped, copying each run between two escapes whole.
///
/// `at` is a char boundary. A candidate that needs no escape (a non-C1 scalar
/// led by `0xC2`, such as U+00A0) stays inside the run it interrupts.
fn push_escaped_from<W: TextOut + ?Sized>(iri: &str, mut at: usize, carrier: Carrier, out: &mut W) {
    let bytes = iri.as_bytes();
    let mut run_start = at;
    while at < bytes.len() {
        let end = at.saturating_add(out.scan_limit().max(1)).min(bytes.len());
        if !out.checkpoint(end - at) {
            return;
        }
        let Some(offset) = find_first_candidate(&bytes[at..end]) else {
            at = end;
            continue;
        };
        let hit = at + offset;
        let ch = iri[hit..]
            .chars()
            .next()
            .expect("a candidate offset is a char boundary");
        at = hit + ch.len_utf8();
        if is_iriref_escape_required_for(ch, carrier) {
            out.push_str(&iri[run_start..hit]);
            push_uchar(ch, out);
            if out.failed() {
                return;
            }
            run_start = at;
        }
    }
    out.push_str(&iri[run_start..]);
}

/// Append `iri` to `out` as an `IRIREF` body for [`Carrier::Xml`]: every scalar
/// [`is_iriref_escape_required`] answers `true` for rides as its `\u00XX`
/// `UCHAR`, and every other scalar rides verbatim. [`push_escaped_for`] is the
/// form that names the carrier.
///
/// The emission of the Turtle writer and of the XML carriers; one scan with
/// bulk copies of the clean runs.
///
/// ```
/// use purrdf_lex::iri_escape::push_escaped;
///
/// let mut out = String::new();
/// push_escaped("urn:ex:a b<\u{85}>\u{a0}", &mut out);
/// assert_eq!(out, "urn:ex:a\\u0020b\\u003C\\u0085\\u003E\u{a0}");
/// ```
pub fn push_escaped<W: TextOut + ?Sized>(iri: &str, out: &mut W) {
    push_escaped_for(iri, Carrier::Xml, out);
}

/// Append `iri` to `out` as an `IRIREF` body for `carrier`: every scalar
/// [`is_iriref_escape_required_for`] answers `true` for rides as its `\u00XX`
/// `UCHAR` in upper-case hex, and every other scalar rides verbatim. The
/// emission of the canonical form uses [`Carrier::Canonical`].
///
/// ```
/// use purrdf_lex::iri_escape::{Carrier, push_escaped_for};
///
/// let mut out = String::new();
/// push_escaped_for("urn:ex:a b<\u{85}>\u{7f}\u{a0}", Carrier::Canonical, &mut out);
/// assert_eq!(out, "urn:ex:a\\u0020b\\u003C\u{85}\\u003E\u{7f}\u{a0}");
/// ```
pub fn push_escaped_for<W: TextOut + ?Sized>(iri: &str, carrier: Carrier, out: &mut W) {
    push_escaped_from(iri, 0, carrier, out);
}

/// `iri` as an `IRIREF` body for [`Carrier::Xml`], borrowed when no scalar needs
/// an escape. [`escape_for`] is the form that names the carrier.
///
/// The same bytes as [`push_escaped`]. The scan that finds the first escaped
/// scalar is the scan that emits, so a value that needs escaping is still read
/// once: its clean prefix is copied whole and the emission continues from the
/// first escape.
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_lex::iri_escape::escape;
///
/// assert!(matches!(escape("https://example.org/caf\u{e9}"), Cow::Borrowed(_)));
/// assert_eq!(escape("urn:ex:a b"), "urn:ex:a\\u0020b");
/// ```
#[must_use]
pub fn escape(iri: &str) -> Cow<'_, str> {
    escape_for(iri, Carrier::Xml)
}

/// `iri` as an `IRIREF` body for `carrier`, borrowed when no scalar needs an
/// escape. The same bytes as [`push_escaped_for`].
///
/// ```
/// use std::borrow::Cow;
/// use purrdf_lex::iri_escape::{Carrier, escape_for};
///
/// // DEL is raw for the canonical form and escaped for XML.
/// assert!(matches!(escape_for("urn:ex:\u{7f}", Carrier::Canonical), Cow::Borrowed(_)));
/// assert_eq!(escape_for("urn:ex:\u{7f}", Carrier::Xml), "urn:ex:\\u007F");
/// ```
#[must_use]
pub fn escape_for(iri: &str, carrier: Carrier) -> Cow<'_, str> {
    let bytes = iri.as_bytes();
    let mut at = 0;
    while let Some(offset) = find_first_candidate(&bytes[at..]) {
        let hit = at + offset;
        let ch = iri[hit..]
            .chars()
            .next()
            .expect("a candidate offset is a char boundary");
        if is_iriref_escape_required_for(ch, carrier) {
            let mut out = String::with_capacity(iri.len() + 8);
            out.push_str(&iri[..hit]);
            push_escaped_from(iri, hit, carrier, &mut out);
            return Cow::Owned(out);
        }
        at = hit + ch.len_utf8();
    }
    Cow::Borrowed(iri)
}

#[cfg(test)]
mod tests {
    use super::{
        CANDIDATE_TABLE, Carrier, escape, escape_for, find_first_candidate,
        is_iriref_escape_required, is_iriref_escape_required_for, push_escaped, push_escaped_for,
    };
    use purrdf_testkit::rng::SplitMix64;
    use std::borrow::Cow;
    use std::fmt::Write as _;

    /// Every Unicode scalar value, in order.
    fn all_scalars() -> impl Iterator<Item = char> {
        (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
    }

    /// The escape set as a flat match: the grammar's nine delimiters, every
    /// control scalar and the SPACE.
    ///
    /// The goldens and frozen vectors were produced by this spelling, so if
    /// the predicate disagrees with it at a single scalar, a golden changes.
    fn the_flat_escape_set(c: char) -> bool {
        matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\')
            || c.is_control()
            || c == ' '
    }

    #[test]
    fn the_predicate_is_the_flat_escape_set_at_every_scalar() {
        for c in all_scalars() {
            assert_eq!(
                is_iriref_escape_required(c),
                the_flat_escape_set(c),
                "{c:?}"
            );
        }
    }

    #[test]
    fn egress_is_ingress_widened_by_exactly_del_and_c1() {
        for c in all_scalars() {
            // Everything the grammar forbids must be escaped: a writer that
            // emitted one raw would mint a document no parser reads back.
            if crate::terminals::is_iriref_forbidden(c) {
                assert!(is_iriref_escape_required(c), "{c:?}");
            }
            // And the difference is one enumerated range, stated as a total
            // function so neither side can drift.
            assert_eq!(
                is_iriref_escape_required(c) && !crate::terminals::is_iriref_forbidden(c),
                matches!(c, '\u{7F}'..='\u{9F}'),
                "{c:?}"
            );
        }
    }

    #[test]
    fn the_canonical_set_is_exactly_the_production_exclusions_at_every_scalar() {
        // N-Quads 1.2 `IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\\] | UCHAR)* '>'`: the
        // canonical form escapes what the grammar forbids and nothing more.
        for c in all_scalars() {
            assert_eq!(
                is_iriref_escape_required_for(c, Carrier::Canonical),
                crate::terminals::is_iriref_forbidden(c),
                "{c:?}"
            );
        }
    }

    #[test]
    fn the_carriers_differ_at_del_and_the_c1_block_and_nowhere_else() {
        for c in all_scalars() {
            assert_eq!(
                is_iriref_escape_required_for(c, Carrier::Xml)
                    != is_iriref_escape_required_for(c, Carrier::Canonical),
                matches!(c, '\u{7F}'..='\u{9F}'),
                "{c:?}"
            );
        }
        // Refused-side case and its valid neighbour, executed on the writer: DEL
        // and NEL are raw canonically and escaped for XML, and U+00A0 is raw in both.
        for (raw, xml) in [
            ("\u{7f}", "\\u007F"),
            ("\u{85}", "\\u0085"),
            ("\u{9f}", "\\u009F"),
        ] {
            let iri = format!("urn:ex:{raw}x");
            assert!(matches!(
                escape_for(&iri, Carrier::Canonical),
                Cow::Borrowed(_)
            ));
            assert_eq!(escape_for(&iri, Carrier::Xml), format!("urn:ex:{xml}x"));
        }
        let neighbour = "urn:ex:\u{a0}x";
        assert!(matches!(
            escape_for(neighbour, Carrier::Canonical),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            escape_for(neighbour, Carrier::Xml),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn the_canonical_writer_agrees_with_the_per_scalar_oracle() {
        let alphabet = alphabet();
        let mut rng = SplitMix64::new(0x001E_1E5C_A9E0_0002);
        for len in (0..=70).chain([127, 128, 129, 255]) {
            for _ in 0..20 {
                let iri: String = (0..len)
                    .map(|_| {
                        if rng.below_usize(6) == 0 {
                            alphabet[rng.below_usize(alphabet.len())]
                        } else {
                            'a'
                        }
                    })
                    .collect();
                let mut expected = String::new();
                for ch in iri.chars() {
                    if crate::terminals::is_iriref_forbidden(ch) {
                        let _ = write!(expected, "\\u{:04X}", ch as u32);
                    } else {
                        expected.push(ch);
                    }
                }
                let mut pushed = String::new();
                push_escaped_for(&iri, Carrier::Canonical, &mut pushed);
                assert_eq!(pushed, expected, "{iri:?}");
                assert_eq!(escape_for(&iri, Carrier::Canonical), expected, "{iri:?}");
            }
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

    /// The per-scalar writer, kept as the oracle for the clean-run emitter.
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
        let mut rng = SplitMix64::new(0x001E_1E5C_A9E0_0001);
        let mut borrowed = 0_usize;
        let mut owned = 0_usize;
        for len in (0..=70).chain([127, 128, 129, 255, 1000]) {
            for _ in 0..40 {
                let iri: String = (0..len)
                    .map(|_| {
                        if rng.below_usize(6) == 0 {
                            alphabet[rng.below_usize(alphabet.len())]
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

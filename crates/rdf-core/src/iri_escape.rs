// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! this workspace. The predicate below is the one spelling; the escape's
//! *emission* stays with each writer, because the three writers that reach here
//! emit through three different machines (a `write!`, a nibble-table push, and a
//! borrow-preserving `Cow` scan) and collapsing those would cost the fast paths
//! without making the law any more shared than naming it once does.
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

#[cfg(test)]
mod tests {
    use super::is_iriref_escape_required;
    use pretty_assertions::assert_eq;

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

    #[test]
    fn the_predicate_is_usable_in_const_context() {
        const { assert!(is_iriref_escape_required(' ')) }
        const { assert!(is_iriref_escape_required('\\')) }
        const { assert!(is_iriref_escape_required('\u{7F}')) }
        const { assert!(!is_iriref_escape_required('\u{A0}')) }
    }
}

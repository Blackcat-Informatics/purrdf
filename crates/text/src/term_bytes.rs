// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An injective, self-delimiting byte encoding of a [`TermValue`].
//!
//! # What it is for
//!
//! The index is fingerprinted by hashing the terms it was built over, so that a
//! caller can tell one index apart from another without comparing them term by
//! term. A fingerprint is only worth anything if distinct inputs cannot collide
//! *before* the hash sees them, so the encoding below is **injective**: two
//! [`TermValue`]s produce the same bytes if and only if they are equal.
//!
//! # Why length prefixes rather than separators
//!
//! A separator byte is not injective over arbitrary RDF, because a literal's
//! lexical form is an arbitrary Unicode string and may contain any byte sequence
//! the separator is spelled with — including the separator. Concatenating
//! `"a\0b"` with `"c"` under a `\0` separator is byte-identical to concatenating
//! `"a"` with `"b\0c"`, and an escape scheme merely moves the same problem into
//! the escape alphabet. A `u64` little-endian length prefix has no such
//! ambiguity: the decoder is told exactly how many bytes to consume before it
//! reads them, so no payload byte can ever be mistaken for structure. Every
//! variable-length field here is written that way, every optional field is
//! preceded by a presence byte, and every enumerated field is a fixed-width tag.
//!
//! # Layout
//!
//! | field | bytes |
//! | --- | --- |
//! | variant tag | 1 (`TAG_IRI` / `TAG_BLANK` / `TAG_LITERAL` / `TAG_TRIPLE`) |
//! | string | 8 (length, little-endian `u64`) + that many UTF-8 bytes |
//! | `Option<T>` | 1 (`ABSENT` / `PRESENT`), then `T` when present |
//! | [`BlankScope`] | 4 (ordinal, little-endian `u32`) |
//! | [`RdfTextDirection`] | 1 (`DIRECTION_LTR` / `DIRECTION_RTL`) |
//!
//! `Iri` is a string; `Blank` is a string then a scope; `Literal` is a lexical
//! form, a datatype IRI, an optional language tag and an optional direction, in
//! that order; `Triple` is its subject, predicate and object encoded in turn.
//! Because the four tags are distinct and each variant's own fields are
//! self-delimiting, the whole encoding is prefix-free and injective.

use purrdf_core::TermValue;

use crate::error::TextError;

/// The width of a term fingerprint, in bytes — `blake3`'s default digest length.
pub const FINGERPRINT_BYTES: usize = 32;

/// Tag byte for [`TermValue::Iri`].
const TAG_IRI: u8 = 0x01;
/// Tag byte for [`TermValue::Blank`].
const TAG_BLANK: u8 = 0x02;
/// Tag byte for [`TermValue::Literal`].
const TAG_LITERAL: u8 = 0x03;
/// Tag byte for [`TermValue::Triple`].
const TAG_TRIPLE: u8 = 0x04;

/// Presence byte for an absent [`Option`] field.
const ABSENT: u8 = 0x00;
/// Presence byte for a present [`Option`] field.
const PRESENT: u8 = 0x01;

/// Tag byte for a left-to-right base direction.
const DIRECTION_LTR: u8 = 0x01;
/// Tag byte for a right-to-left base direction.
const DIRECTION_RTL: u8 = 0x02;

/// How deeply a [`TermValue::Triple`] may nest before the encoder refuses it.
///
/// `TermValue` is a heap-linked tree and this encoder walks it with ordinary
/// recursion, so an adversarial or merely pathological term — a triple term
/// whose subject is a triple term, some thousands deep — would exhaust the
/// stack, and a stack overflow aborts the process rather than raising anything a
/// caller can handle. The bound turns that abort into a
/// [`TextError::Data`]: the input is refused, by name, and the host stays up.
///
/// The value is far above anything RDF 1.2 syntax produces in practice (a
/// hand-written triple term nests once or twice) and far below the depth at
/// which the frames here threaten a default stack, so it separates real data
/// from a resource attack without arbitrating between two plausible datasets.
const MAX_TRIPLE_DEPTH: u32 = 64;

/// Append the canonical, injective encoding of `value` to `out`.
///
/// Distinct terms always append distinct byte strings, and equal terms always
/// append identical ones — see the module documentation for why. `out` is
/// appended to rather than replaced, so a sequence of terms can be encoded into
/// one buffer and stays unambiguous when it is (each term's encoding is
/// self-delimiting).
///
/// Fails with [`TextError::Data`] if `value` nests triple terms more than
/// [`MAX_TRIPLE_DEPTH`] deep.
pub(crate) fn encode_term(value: &TermValue, out: &mut Vec<u8>) -> Result<(), TextError> {
    encode_at_depth(value, out, 0)
}

/// A [`FINGERPRINT_BYTES`]-wide digest of `terms`, in the order given.
///
/// This is what the encoding above exists for. Because the encoding is
/// injective and self-delimiting, two sequences of terms digest identically if
/// and only if they are the same sequence — so a caller can compare two indexes,
/// or check one against a recorded value, without holding either's terms.
///
/// The digest is a pure function of the terms: `blake3` is keyless and
/// unseeded, and the encoding depends on no ambient state, so the same sequence
/// yields the same bytes on every target and in every process.
///
/// # Errors
///
/// [`TextError::Data`] if any term nests triple terms past the encoder's depth
/// bound. The bound exists because the encoder walks a heap-linked tree with
/// ordinary recursion, and a stack overflow would abort the process rather than
/// raise anything a caller can handle.
pub fn fingerprint_terms<'a, I>(terms: I) -> Result<[u8; FINGERPRINT_BYTES], TextError>
where
    I: IntoIterator<Item = &'a TermValue>,
{
    let mut hasher = blake3::Hasher::new();
    let mut buffer = Vec::new();
    for term in terms {
        buffer.clear();
        encode_term(term, &mut buffer)?;
        hasher.update(&buffer);
    }
    Ok(*hasher.finalize().as_bytes())
}

/// [`encode_term`]'s recursive body, carrying the current triple-term nesting
/// depth so the bound can be enforced without exposing it in the public
/// signature.
fn encode_at_depth(value: &TermValue, out: &mut Vec<u8>, depth: u32) -> Result<(), TextError> {
    if depth > MAX_TRIPLE_DEPTH {
        return Err(TextError::data(format!(
            "triple term nests deeper than the encoder's bound of {MAX_TRIPLE_DEPTH}"
        )));
    }
    match value {
        TermValue::Iri(iri) => {
            out.push(TAG_IRI);
            push_str(iri, out);
        }
        TermValue::Blank { label, scope } => {
            out.push(TAG_BLANK);
            push_str(label, out);
            out.extend_from_slice(&scope.ordinal().to_le_bytes());
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            out.push(TAG_LITERAL);
            push_str(lexical_form, out);
            push_str(datatype, out);
            match language {
                Some(tag) => {
                    out.push(PRESENT);
                    push_str(tag, out);
                }
                None => out.push(ABSENT),
            }
            match direction {
                Some(dir) => {
                    out.push(PRESENT);
                    out.push(match dir {
                        purrdf_core::RdfTextDirection::Ltr => DIRECTION_LTR,
                        purrdf_core::RdfTextDirection::Rtl => DIRECTION_RTL,
                    });
                }
                None => out.push(ABSENT),
            }
        }
        TermValue::Triple { s, p, o } => {
            out.push(TAG_TRIPLE);
            encode_at_depth(s, out, depth + 1)?;
            encode_at_depth(p, out, depth + 1)?;
            encode_at_depth(o, out, depth + 1)?;
        }
    }
    Ok(())
}

/// Append `text` as a little-endian `u64` byte length followed by its UTF-8
/// bytes — the one self-delimiting string form this encoding uses.
fn push_str(text: &str, out: &mut Vec<u8>) {
    let bytes = text.as_bytes();
    // `usize` is at most 64 bits on every target this workspace builds for
    // (x86-64 and wasm32, where it is 32), so the length always fits.
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use purrdf_core::{BlankScope, RdfTextDirection, TermValue};

    use super::{FINGERPRINT_BYTES, MAX_TRIPLE_DEPTH, encode_term, fingerprint_terms};
    use crate::error::TextError;

    /// Encode one term into a fresh buffer, for tests that only care about the
    /// bytes of a single value.
    fn encode(value: &TermValue) -> Vec<u8> {
        let mut out = Vec::new();
        encode_term(value, &mut out).expect("shallow test terms are within the depth bound");
        out
    }

    /// Wrap `inner` in `depth` nested triple terms, always in the subject slot,
    /// so the nesting is exactly `depth` levels.
    fn nest(inner: TermValue, depth: u32) -> TermValue {
        let mut value = inner;
        for _ in 0..depth {
            value = TermValue::Triple {
                s: Box::new(value),
                p: Box::new(TermValue::iri("https://example.org/p")),
                o: Box::new(TermValue::iri("https://example.org/o")),
            };
        }
        value
    }

    /// An arbitrary non-recursive term: an IRI, a blank node in one of a few
    /// scopes, or a literal with every combination of optional field.
    ///
    /// The alphabets are deliberately tiny and overlapping — the same short
    /// strings appear as IRIs, as blank labels, as lexical forms and as language
    /// tags — because that is what makes the injectivity property below a real
    /// test: a shared alphabet is where a separator-based encoding would
    /// collide, so proptest is given every chance to find such a collision.
    fn leaf_term() -> impl Strategy<Value = TermValue> {
        prop_oneof![
            "[a-c:/#]{0,4}".prop_map(TermValue::Iri),
            ("[a-c0-9]{0,4}", 0_u32..3).prop_map(|(label, scope)| TermValue::Blank {
                label,
                scope: BlankScope(scope),
            }),
            (
                "[a-c:/# ]{0,4}",
                "[a-c:/#]{0,4}",
                proptest::option::of("[a-c:/#]{0,4}"),
                proptest::option::of(prop_oneof![
                    Just(RdfTextDirection::Ltr),
                    Just(RdfTextDirection::Rtl)
                ]),
            )
                .prop_map(|(lexical_form, datatype, language, direction)| {
                    TermValue::Literal {
                        lexical_form,
                        datatype,
                        language,
                        direction,
                    }
                }),
        ]
    }

    /// An arbitrary term, including triple terms nested up to three deep.
    fn any_term() -> impl Strategy<Value = TermValue> {
        leaf_term().prop_recursive(3, 24, 3, |inner| {
            (inner.clone(), inner.clone(), inner).prop_map(|(s, p, o)| TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            })
        })
    }

    proptest! {
        /// The whole contract, in both directions: equal terms encode
        /// identically, and distinct terms never encode identically.
        #[test]
        fn encoding_is_injective(a in any_term(), b in any_term()) {
            let encoded_a = encode(&a);
            let encoded_b = encode(&b);
            if a == b {
                prop_assert_eq!(
                    &encoded_a,
                    &encoded_b,
                    "equal terms encoded differently: {:?}",
                    a
                );
            } else {
                prop_assert_ne!(
                    &encoded_a,
                    &encoded_b,
                    "distinct terms collided: {:?} vs {:?}",
                    a,
                    b
                );
            }
        }

        /// Encoding is a pure function of the term: the same input yields the
        /// same bytes however many times it is asked.
        #[test]
        fn encoding_is_deterministic(term in any_term()) {
            prop_assert_eq!(encode(&term), encode(&term));
        }

        /// Encoding into a non-empty buffer appends rather than disturbing what
        /// is already there, so a run of terms can share one buffer.
        #[test]
        fn encoding_appends_to_the_buffer(a in any_term(), b in any_term()) {
            let mut both = encode(&a);
            let prefix_len = both.len();
            encode_term(&b, &mut both).expect("within the depth bound");
            prop_assert_eq!(&both[..prefix_len], &encode(&a)[..]);
            prop_assert_eq!(&both[prefix_len..], &encode(&b)[..]);
        }
    }

    /// The four variant tags are distinct, so no variant's encoding can be
    /// another's — the base case injectivity rests on.
    #[test]
    fn each_variant_has_its_own_tag() {
        let iri = encode(&TermValue::iri("https://example.org/a"));
        let blank = encode(&TermValue::blank("a"));
        let literal = encode(&TermValue::simple_literal("a"));
        let triple = encode(&nest(TermValue::iri("https://example.org/a"), 1));
        let tags = [iri[0], blank[0], literal[0], triple[0]];
        for (i, left) in tags.iter().enumerate() {
            for right in &tags[i + 1..] {
                assert_ne!(left, right, "two variants share a tag byte");
            }
        }
    }

    /// The case a separator byte gets wrong. Both literals hold the same total
    /// text with the split in a different place; only the length prefixes tell
    /// them apart.
    #[test]
    fn a_shifted_split_does_not_collide() {
        let left = TermValue::Literal {
            lexical_form: "ab".to_owned(),
            datatype: "c".to_owned(),
            language: None,
            direction: None,
        };
        let right = TermValue::Literal {
            lexical_form: "a".to_owned(),
            datatype: "bc".to_owned(),
            language: None,
            direction: None,
        };
        assert_ne!(encode(&left), encode(&right));
    }

    /// A blank node's scope participates in identity, so it must participate in
    /// the encoding: same label, different scope, different bytes.
    #[test]
    fn blank_scope_is_encoded() {
        let default = TermValue::Blank {
            label: "b".to_owned(),
            scope: BlankScope::DEFAULT,
        };
        let scoped = TermValue::Blank {
            label: "b".to_owned(),
            scope: BlankScope(7),
        };
        assert_ne!(encode(&default), encode(&scoped));
    }

    /// An absent optional field is distinguishable from a present one holding
    /// the empty string, which a bare "write the payload" encoding would not be.
    #[test]
    fn an_absent_language_differs_from_an_empty_one() {
        let absent = TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: "d".to_owned(),
            language: None,
            direction: None,
        };
        let empty = TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: "d".to_owned(),
            language: Some(String::new()),
            direction: None,
        };
        assert_ne!(encode(&absent), encode(&empty));
    }

    /// The two base directions encode differently, and both differ from an
    /// absent direction.
    #[test]
    fn both_directions_are_distinguished() {
        let of = |direction| TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: "d".to_owned(),
            language: Some("en".to_owned()),
            direction,
        };
        let none = encode(&of(None));
        let ltr = encode(&of(Some(RdfTextDirection::Ltr)));
        let rtl = encode(&of(Some(RdfTextDirection::Rtl)));
        assert_ne!(none, ltr);
        assert_ne!(none, rtl);
        assert_ne!(ltr, rtl);
    }

    /// The digest is a pure function of the term sequence, and order is part of
    /// that sequence: reordering two distinct terms changes the answer.
    #[test]
    fn the_fingerprint_is_deterministic_and_order_sensitive() {
        let a = TermValue::iri("https://example.org/a");
        let b = TermValue::simple_literal("a");

        let forward = fingerprint_terms([&a, &b]).expect("within the depth bound");
        assert_eq!(
            forward,
            fingerprint_terms([&a, &b]).expect("within the depth bound"),
            "the same sequence must digest identically"
        );
        assert_ne!(
            forward,
            fingerprint_terms([&b, &a]).expect("within the depth bound"),
            "a reordered sequence is a different sequence"
        );
        assert_eq!(forward.len(), FINGERPRINT_BYTES);
    }

    /// A term the encoder refuses is refused by the fingerprint too, rather
    /// than silently contributing nothing to the digest.
    #[test]
    fn the_fingerprint_propagates_the_depth_refusal() {
        let deep = nest(
            TermValue::iri("https://example.org/x"),
            MAX_TRIPLE_DEPTH + 1,
        );
        assert!(matches!(
            fingerprint_terms([&deep]),
            Err(TextError::Data(_))
        ));
    }

    /// A term exactly at the bound encodes; one past it is refused as data
    /// rather than overflowing the stack.
    #[test]
    fn the_depth_bound_is_enforced_not_overflowed() {
        let at_bound = nest(TermValue::iri("https://example.org/x"), MAX_TRIPLE_DEPTH);
        let mut out = Vec::new();
        assert_eq!(
            encode_term(&at_bound, &mut out),
            Ok(()),
            "a term at the bound must encode"
        );

        let past_bound = nest(
            TermValue::iri("https://example.org/x"),
            MAX_TRIPLE_DEPTH + 1,
        );
        let mut out = Vec::new();
        assert!(
            matches!(encode_term(&past_bound, &mut out), Err(TextError::Data(_))),
            "a term past the bound must be refused as data"
        );
    }
}

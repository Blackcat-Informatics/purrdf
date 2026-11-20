// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Term nodes: the lowering of a triple term that is a component of another.
//!
//! A NESTED TERM is a triple term that stands as a component of another
//! triple term handed to the labelling. Each distinct nested term becomes one
//! TERM NODE, a blank node of the labelled set that is no blank node of the
//! view, and one `record` line, `record(P, [node, N(s), N(p), N(o)])`, whose
//! payload `P` is [`NESTED_TERM_PAYLOAD`] and whose components are lowered in
//! turn: `N(x)` is the term node of `x` when `x` is a triple term and `x`
//! otherwise. Every triple term of the labelled set then has depth one, so the
//! lines that name a chain of triple terms grow with its length and not with
//! its square.
//!
//! The source identifier of a term node is `E([d(x)])`, the deterministic CBOR
//! encoding of an array of one 32-byte digest, with
//! `d(x) = H(nested_term, E([K(s), K(p), K(o)]))` and `H` BLAKE3 in key
//! derivation mode under [`NESTED_TERM_CONTEXT`]. The source `K` of a
//! component is `E([0, h(b)])` for a blank node, `h(b)` the digest of its
//! source identifier `E([ordinal, label])` under the same context;
//! `E([d(c)])` for a triple term; and the byte string of `E(v)` for an IRI
//! (`v = [1, text]`) or a literal (`v = [2, lexical, form, x]`, `form` the
//! literal form code: 1 typed, 2 language, 3 language with `ltr`, 4 language
//! with `rtl`; `x` the datatype IRI of a typed literal and the language of a
//! language literal). These are the bytes of the second-generation GTS
//! format's section "Nested terms, lowering and term nodes". The digest only
//! orders nodes; it enters no line, payload or other digest.

use purrdf_hash::Domain;
use purrdf_lex::cbor::head::{
    ARRAY, UNSIGNED, push_head, read_argument, read_bytes, write_bytes, write_text,
};

use super::digest_of;
use crate::governor::is_stopped;
use crate::{FastMap, RdfTextDirection, StopSignal, TermValue};

/// The hash context of the digests `d` and `h` of a term node's source
/// identifier.
pub(crate) const NESTED_TERM_CONTEXT: Domain = Domain::new(b"gts2 nested term");

/// The payload of a nested term's `record` line: `E([5, [[1], [1], [1], [1]]])`,
/// the record kind `nested_term` and four term slots.
pub(crate) const NESTED_TERM_PAYLOAD: [u8; 11] = [
    0x82, 0x05, 0x84, 0x81, 0x01, 0x81, 0x01, 0x81, 0x01, 0x81, 0x01,
];

/// The literal form codes of a literal's source: typed, then language with no
/// direction, with `ltr` and with `rtl`.
const FORM_TYPED: u64 = 1;
const FORM_LANGUAGE: u64 = 2;
const FORM_LANGUAGE_LTR: u64 = 3;
const FORM_LANGUAGE_RTL: u64 = 4;

/// The first byte of a term node's source identifier, `E([d])`: an array of one.
const TERM_NODE_HEAD: u8 = 0x81;
/// The first byte of a blank node's source identifier in the text form,
/// `E([ordinal, label])`: an array of two.
const TEXT_BLANK_HEAD: u8 = 0x82;

/// The digest function of the term nodes. The lowering takes it as a value so
/// that a test can give two nested terms one digest; every entry passes
/// [`nested_term_digest`].
pub(crate) type NestedHash = fn(&[u8]) -> [u8; 32];

/// `H(nested_term, message)`.
pub(crate) fn nested_term_digest(message: &[u8]) -> [u8; 32] {
    digest_of(NESTED_TERM_CONTEXT, message)
}

/// A component of a nested term as the labelled set holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Part {
    /// An interned constant leaf, by index into the constant table.
    Const(usize),
    /// A blank node of the view or a composite node, by node index.
    Blank(usize),
    /// Another nested term, by its index among the nested terms.
    Nested(usize),
}

/// The distinct nested terms of one input, by their lowered components. A
/// nested term is interned after its components, so index order puts every
/// component before the term that holds it.
#[derive(Debug, Default)]
pub(crate) struct NestedTerms {
    parts: Vec<[Part; 3]>,
    index: FastMap<[Part; 3], usize>,
    /// `K` of each constant and composite node that is a component of a
    /// nested term, formed from its term value when it is first reached as
    /// one.
    sources: FastMap<Part, Box<[u8]>>,
}

impl NestedTerms {
    /// Whether the input holds no nested term.
    pub(crate) fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The index of the nested term of `parts`, if it is interned.
    pub(crate) fn get(&self, parts: &[Part; 3]) -> Option<usize> {
        self.index.get(parts).copied()
    }

    /// Intern the nested term of `parts`, which is not yet interned.
    pub(crate) fn push(&mut self, parts: [Part; 3]) -> usize {
        let n = self.parts.len();
        self.parts.push(parts);
        self.index.insert(parts, n);
        n
    }

    /// Whether `K` of the component `part` is not yet kept.
    pub(crate) fn needs_source(&self, part: Part) -> bool {
        !self.sources.contains_key(&part)
    }

    /// Keep `source`, `K` of the component `part`, which is not yet kept.
    pub(crate) fn keep_source(&mut self, part: Part, source: Box<[u8]>) {
        self.sources.insert(part, source);
    }

    /// An upper bound of the bytes [`Self::digests`] keeps until the nested
    /// records are pushed: the digest of each nested term, in the list and as
    /// the key of the collision check, and the digest `h` of each blank node
    /// that is a component, at most three for each nested term.
    pub(crate) fn digest_bytes(&self) -> usize {
        self.parts
            .len()
            .saturating_mul(2 * 32 + 3 * (32 + size_of::<usize>()))
    }

    /// The components of nested term `n`.
    pub(crate) fn parts(&self, n: usize) -> [Part; 3] {
        self.parts[n]
    }

    /// The sum of the weights of the `record` lines of the nested terms: for
    /// each, `37` (its tag, its payload with its length, the length of its
    /// term list and its term node) plus the weight of each component, a blank
    /// node or a term node weighing `9`, saturating at `2^64 - 1`.
    pub(crate) fn record_weight(&self, consts: &[Box<[u8]>]) -> u64 {
        self.parts.iter().fold(0u64, |total, parts| {
            parts
                .iter()
                .map(|part| match *part {
                    Part::Const(c) => consts[c].len() as u64,
                    Part::Blank(_) | Part::Nested(_) => 9,
                })
                .fold(total.saturating_add(37), u64::saturating_add)
        })
    }

    /// The digest `d` of every nested term, components first, computing `h`
    /// once for each blank node that is a component. `nodes` holds the source
    /// identifiers of the blank nodes in the text form.
    ///
    /// Every constant and composite component has its source kept by
    /// [`Self::keep_source`]: the lowering keeps it as it forms the component.
    ///
    /// The caller's `stop` signal is polled before each nested term.
    ///
    /// # Errors
    /// [`DigestRefusal::Stopped`] when `stop` is set, and
    /// [`DigestRefusal::Collision`] with the digest two different nested terms
    /// share: the refusal `term_node`.
    pub(crate) fn digests(
        &self,
        nodes: &[Box<[u8]>],
        hash: NestedHash,
        stop: Option<&dyn StopSignal>,
    ) -> Result<Vec<[u8; 32]>, DigestRefusal> {
        let mut digests: Vec<[u8; 32]> = Vec::with_capacity(self.parts.len());
        let mut seen: FastMap<[u8; 32], ()> = FastMap::default();
        let mut blank_digests: FastMap<usize, [u8; 32]> = FastMap::default();
        let mut message = Vec::new();
        let mut source = Vec::new();
        for parts in &self.parts {
            if is_stopped(stop) {
                return Err(DigestRefusal::Stopped);
            }
            message.clear();
            push_head(&mut message, ARRAY, 3);
            for part in parts {
                source.clear();
                match *part {
                    Part::Const(_) => source.extend_from_slice(kept(&self.sources, part)),
                    Part::Blank(b) => match self.sources.get(part) {
                        // A composite node stands for its literal value, one
                        // node per value, so its source is the literal's.
                        Some(kept) => source.extend_from_slice(kept),
                        None => {
                            let h = *blank_digests.entry(b).or_insert_with(|| hash(&nodes[b]));
                            push_head(&mut source, ARRAY, 2);
                            push_head(&mut source, UNSIGNED, 0);
                            push_bytes(&mut source, &h);
                        }
                    },
                    Part::Nested(n) => source.extend_from_slice(&term_node_source(&digests[n])),
                }
                push_bytes(&mut message, &source);
            }
            let digest = hash(&message);
            if seen.insert(digest, ()).is_some() {
                return Err(DigestRefusal::Collision(digest));
            }
            digests.push(digest);
        }
        Ok(digests)
    }
}

/// Why [`NestedTerms::digests`] stopped before it returned every digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DigestRefusal {
    /// The caller's stop signal was set.
    Stopped,
    /// Two different nested terms share this digest.
    Collision([u8; 32]),
}

/// The kept source of a constant component.
fn kept<'a>(sources: &'a FastMap<Part, Box<[u8]>>, part: &Part) -> &'a [u8] {
    let Some(source) = sources.get(part) else {
        // INVARIANT: the lowering forms a constant component in one place,
        // which keeps its source from the term value before it returns it.
        unreachable!("a constant component of a nested term has no kept source")
    };
    source
}

/// The source identifier of a term node of digest `digest`: `E([d])`.
pub(crate) fn term_node_source(digest: &[u8; 32]) -> [u8; 35] {
    let mut source = [0u8; 35];
    source[0] = TERM_NODE_HEAD;
    source[1] = 0x58;
    source[2] = 0x20;
    source[3..].copy_from_slice(digest);
    source
}

/// Whether `source` is a term node's source identifier among the sources of a
/// labelled set in the text form.
pub(crate) fn is_term_node_source(source: &[u8]) -> bool {
    source.first() == Some(&TERM_NODE_HEAD)
}

/// The text form of the source identifier of the blank node `(label, scope)`
/// of scope ordinal `ordinal`: `E([ordinal, label])`, the label a byte string.
pub(crate) fn text_blank_source(ordinal: u32, label: &[u8]) -> Vec<u8> {
    let mut source = Vec::with_capacity(16 + label.len());
    push_head(&mut source, ARRAY, 2);
    push_head(&mut source, UNSIGNED, u64::from(ordinal));
    push_bytes(&mut source, label);
    source
}

/// The scope ordinal and the label of a source identifier in the text form,
/// or `None` for any other source.
pub(crate) fn parse_text_blank_source(source: &[u8]) -> Option<(u32, &[u8])> {
    let mut rest = source.strip_prefix(&[TEXT_BLANK_HEAD])?;
    let ordinal = u32::try_from(read_argument(&mut rest, UNSIGNED)?).ok()?;
    let label = read_bytes(&mut rest)?;
    rest.is_empty().then_some((ordinal, label))
}

/// `K` of the IRI or literal `value`, the byte string of `E(v)`, or `None`
/// for a blank node or a triple term, whose sources are formed from digests.
/// Every text is taken whole, as the view holds it: no case is folded.
pub(crate) fn leaf_source(value: &TermValue) -> Option<Box<[u8]>> {
    let mut v = Vec::new();
    match value {
        TermValue::Iri(iri) => {
            push_head(&mut v, ARRAY, 2);
            push_head(&mut v, UNSIGNED, 1);
            push_text(&mut v, iri);
        }
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            let (form, x) = match (language, direction) {
                (None, _) => (FORM_TYPED, datatype),
                (Some(language), None) => (FORM_LANGUAGE, language),
                (Some(language), Some(RdfTextDirection::Ltr)) => (FORM_LANGUAGE_LTR, language),
                (Some(language), Some(RdfTextDirection::Rtl)) => (FORM_LANGUAGE_RTL, language),
            };
            push_head(&mut v, ARRAY, 4);
            push_head(&mut v, UNSIGNED, 2);
            push_text(&mut v, lexical_form);
            push_head(&mut v, UNSIGNED, form);
            push_text(&mut v, x);
        }
        TermValue::Blank { .. } | TermValue::Triple { .. } => return None,
    }
    let mut source = Vec::with_capacity(9 + v.len());
    push_bytes(&mut source, &v);
    Some(source.into_boxed_slice())
}

/// Append `bytes` as a CBOR byte string.
fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let Ok(()) = write_bytes(out, bytes) else {
        // INVARIANT: writing to a `Vec` grows it or aborts; it never errs.
        unreachable!("writing to a Vec cannot fail")
    };
}

/// Append `text` as a CBOR text string.
fn push_text(out: &mut Vec<u8>, text: &str) {
    let Ok(()) = write_text(out, text) else {
        // INVARIANT: writing to a `Vec` grows it or aborts; it never errs.
        unreachable!("writing to a Vec cannot fail")
    };
}

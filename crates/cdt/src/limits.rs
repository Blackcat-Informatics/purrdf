// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The two resource bounds **every** [`crate::CdtValue`] satisfies, and the
//! machinery that establishes them.
//!
//! # The bounds are an invariant of the type, not a property of one code path
//!
//! There is no way to obtain a [`crate::CdtValue`] except through a constructor in
//! this crate, and every one of them either checks both bounds here or is reached
//! only from a caller that just did. Concretely:
//!
//! * [`crate::CdtValue::list`] and [`crate::CdtValue::map`] measure the prospective
//!   value and refuse before returning it;
//! * [`crate::parse_cdt`] enforces the element bound *as it scans* — before the
//!   offending element is allocated — and the byte bound both on the input it is
//!   offered and on the canonical form the result would have;
//! * [`crate::functions`] measures each prospective result from borrowed inputs and
//!   refuses before cloning anything;
//! * [`crate::CdtValue::empty_list`] and [`crate::CdtValue::empty_map`] are within
//!   both bounds by inspection.
//!
//! That is why the crate's public constructors return `Result` rather than the value:
//! a bound is not advice a caller may decline. A consumer that only ever *reads* a
//! [`crate::CdtValue`] can rely on the invariant without re-checking it.
//!
//! # Every value carries its own measure
//!
//! A [`crate::CdtValue`] stores the [`Extent`] it was built with — its element count
//! at every level and the exact byte length of its canonical form — so measuring a
//! composite that holds it costs one read rather than a walk. That is what keeps
//! building a value one level at a time linear: each constructor measures its
//! direct children, and a nested composite answers from the extent it already
//! carries. The cached measure is exact by construction (every constructor computes
//! it from the same walker the renderer uses), and it is an invariant for the same
//! reason the bounds are — the contents are private, so nothing can change them
//! underneath it.
//!
//! # Every check here is iterative
//!
//! Measuring a composite — one that exists, or one that does not exist yet — walks
//! with an explicit heap worklist and never recurses. A bound check that could itself
//! overflow the stack would be checking for the very thing it caused, and in Rust a
//! stack overflow is an `abort` no caller can catch.
//!
//! # Why this crate owns its own bounds
//!
//! A composite-datatype lexical form is **attacker-controlled data inside a
//! literal**: it arrives as the lexical form of an RDF term, so a hostile dataset
//! (or a hostile SPARQL literal) chooses its shape entirely. In Rust, exhausting
//! the stack is an `abort`, not a catchable panic, so a scanner that recursed on
//! this input could be turned into an uncatchable process kill by a payload as
//! small as two megabytes of `[[[[…`. The whole crate is therefore iterative — the
//! scanner, the renderer, equality, ordering, the canonical mapping, and the value
//! tree's own `Drop`, `Clone` and `Debug` all carry an explicit heap worklist — and
//! these two bounds cap the *heap* the input can command.
//!
//! # Why there is no nesting bound
//!
//! Depth is not a resource of its own here. Every nesting level is one element of
//! the composite that holds it, so a value's depth is at most one more than its
//! element count and [`MAX_ELEMENTS`] bounds it as a consequence; every walk over a
//! value is iterative, so a level costs heap and never stack; and a fixed depth cap
//! would have refused values the two real bounds admit, identically on every host,
//! for no resource reason. A million-deep `[[[[…` is two megabytes of input, a
//! million minus one elements, and a value this crate parses, renders, compares and
//! drops like any other.

use alloc::vec::Vec;

use crate::error::CdtError;
use crate::render::{key_lexical_len, term_lexical_len};
use crate::term::{CdtKey, CdtTerm};

/// Maximum number of elements (list items plus map entries, at every level) in one
/// CDT lexical form.
///
/// A form that would exceed this is [`crate::CdtError::TooManyElements`].
///
/// **Governor justification.** A [`crate::CdtTerm`] is tens of bytes on its own
/// before the `String`s it owns, and every element additionally costs a slot in its
/// parent's `Vec`, so 2²⁰ elements is already a tens-of-megabytes resident value
/// produced from a *single* literal — and a solution sequence holds many literals at
/// once. This is the bound that stops a small lexical form (`[1,1,1,…]` costs two
/// input bytes per element) from amplifying into an unbounded parsed value. It is
/// also what bounds nesting: a level is an element of its container, so a value's
/// depth never exceeds its element count by more than one.
pub const MAX_ELEMENTS: usize = 1 << 20;

/// Maximum length, in bytes, of a CDT lexical form offered to the scanner.
///
/// A longer input is refused up front as [`crate::CdtError::InputTooLarge`], before
/// a single byte is scanned or a single allocation is made.
///
/// **Governor justification.** This is the outer of the two bounds: it bounds the
/// work as well as the result. Scanning is linear in the input, but every accepted
/// element also allocates, so the peak resident set while parsing is a multiple of
/// the input length; 64 MiB caps that multiple at a size a wasm module's 32-bit
/// address space can survive. It also makes the refusal *cheap* — an oversized
/// payload costs one length comparison rather than a scan that eventually trips
/// [`MAX_ELEMENTS`] after allocating its way there.
///
/// The bound applies to the canonical form as well as to the input. A lexical form
/// PurRDF *accepts* may be shorter than the one it would *write* — `[1]` is three
/// bytes in and forty-eight out, because the canonical form spells every shorthand —
/// so checking only the input would let a value into the type whose own lexical form
/// no host could hold. [`crate::parse_cdt`] therefore checks both, and the
/// [`crate::functions`] that mint check the canonical length they will create.
pub const MAX_LEXICAL_BYTES: usize = 64 * 1024 * 1024;

// ── Measuring a value, or a value that does not exist yet ───────────────────────

/// The shape a composite has, or would have if it were built.
///
/// Measured from **borrowed** parts, so a value that will be refused is never
/// allocated: `cdt:put(?m, ?k, ?m)` roughly doubles a map's element count on every
/// application, and a query of twenty-one lines could otherwise ask for a value no
/// host can hold. Every [`crate::CdtValue`] keeps the extent it was built with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Extent {
    /// Elements at every level, counted the way
    /// [`crate::CdtValue::element_count`] counts them.
    pub(crate) elements: usize,
    /// The exact byte length of the canonical lexical form the composite would have.
    pub(crate) bytes: usize,
}

impl Extent {
    /// The extent of an empty composite: no elements, and the two delimiter bytes.
    pub(crate) const EMPTY: Self = Self {
        elements: 0,
        bytes: 2,
    };
}

/// The elements one element contributes to its container: none for a leaf, the
/// value's whole element count for a nested composite, and the sum over its
/// components for a triple term. Iterative, with an explicit worklist.
pub(crate) fn term_elements(term: &CdtTerm) -> usize {
    let mut elements = 0usize;
    let mut work: Vec<&CdtTerm> = alloc::vec![term];
    while let Some(current) = work.pop() {
        match current {
            CdtTerm::Composite(inner) => {
                elements = elements.saturating_add(inner.extent().elements);
            }
            CdtTerm::TripleTerm(triple) => {
                work.push(&triple.subject);
                work.push(&triple.predicate);
                work.push(&triple.object);
            }
            CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::Literal(_) | CdtTerm::Null => {}
        }
    }
    elements
}

/// The extent of the list these elements would form.
pub(crate) fn list_extent<'a>(items: impl IntoIterator<Item = &'a CdtTerm>) -> Extent {
    let mut count = 0usize;
    let mut elements = 0usize;
    let mut bytes = 0usize;
    for term in items {
        count = count.saturating_add(1);
        elements = elements.saturating_add(term_elements(term));
        bytes = bytes.saturating_add(term_lexical_len(term));
    }
    Extent {
        elements: elements.saturating_add(count),
        // `[`, `]`, and one `,` between each adjacent pair.
        bytes: bytes
            .saturating_add(2)
            .saturating_add(count.saturating_sub(1)),
    }
}

/// The extent of the map these key/value pairs would form. The pairs must already be
/// deduplicated by key.
pub(crate) fn map_extent<'a>(pairs: impl IntoIterator<Item = (&'a CdtKey, &'a CdtTerm)>) -> Extent {
    let mut count = 0usize;
    let mut elements = 0usize;
    let mut bytes = 0usize;
    for (key, value) in pairs {
        count = count.saturating_add(1);
        elements = elements.saturating_add(term_elements(value));
        // `key` `:` `value`.
        bytes = bytes
            .saturating_add(key_lexical_len(key))
            .saturating_add(1)
            .saturating_add(term_lexical_len(value));
    }
    Extent {
        elements: elements.saturating_add(count),
        // `{`, `}`, and one `,` between each adjacent pair.
        bytes: bytes
            .saturating_add(2)
            .saturating_add(count.saturating_sub(1)),
    }
}

/// Check a composite, built or prospective, against both bounds.
///
/// The offsets these errors carry are positions in a lexical form, and a value with no
/// scanned input has only the canonical form it would have; each offset therefore names
/// the position the offending construct would occupy there, which for the opening
/// delimiter is byte 0.
pub(crate) fn check_extent(extent: &Extent) -> Result<(), CdtError> {
    if extent.elements > MAX_ELEMENTS {
        return Err(CdtError::TooManyElements {
            offset: 0,
            limit: MAX_ELEMENTS,
        });
    }
    if extent.bytes > MAX_LEXICAL_BYTES {
        return Err(CdtError::InputTooLarge {
            offset: MAX_LEXICAL_BYTES,
            length: extent.bytes,
        });
    }
    Ok(())
}

/// Check a prospective **element** against both bounds.
///
/// An element is not itself a composite value, so it carries no invariant of its own;
/// what this answers is whether the element could ever appear in one. The measure is
/// therefore the **smallest composite that could hold it** — the one-element list
/// `[term]`, which is one element larger and two bytes longer than the element. An
/// element that fails here can never be placed anywhere, so
/// [`CdtTerm::composite`](crate::CdtTerm::composite) and
/// [`CdtTerm::triple`](crate::CdtTerm::triple) refuse it where it is made rather than
/// leaving the caller to discover it when it is finally placed. A triple term is the
/// case that most needs this: it combines three separately admissible elements' element
/// counts and canonical lengths into one that need not be.
pub(crate) fn check_term(term: &CdtTerm) -> Result<(), CdtError> {
    check_extent(&Extent {
        elements: term_elements(term).saturating_add(1),
        bytes: term_lexical_len(term).saturating_add(2),
    })
}

#[cfg(test)]
mod tests {
    use super::{Extent, list_extent, map_extent};
    use alloc::vec;

    use crate::{CdtTerm, CdtValue, canonical_lexical, parse_list, parse_map};

    /// The extent a value carries is the extent a walk over it measures, and the byte
    /// count is the length of the form the renderer writes.
    #[test]
    fn a_carried_extent_agrees_with_a_fresh_walk_and_with_the_rendering() {
        for lexical in [
            "[]",
            "[1]",
            "[1,[2,[3]],{'k':[null]}]",
            "[<<(<http://example.org/s> [<http://example.org/p>] {'a':<<(1 2 3)>>})>>]",
            "[\"a\\u0001b\"@en--rtl, _:b, 1.5, true]",
        ] {
            let value = parse_list(lexical).expect("the fixture parses");
            let walked = match value.contents() {
                crate::CdtContents::List(items) => list_extent(items.iter()),
                crate::CdtContents::Map(_) => unreachable!("parse_list yields a list"),
            };
            assert_eq!(value.extent(), walked, "{lexical}");
            assert_eq!(value.extent().elements, value.element_count(), "{lexical}");
            assert_eq!(
                value.extent().bytes,
                canonical_lexical(&value).len(),
                "{lexical}"
            );
        }
        let map = parse_map("{'b':[1,{'c':null}],'a':<<(1 2 [3])>>}").expect("the fixture parses");
        let walked = match map.contents() {
            crate::CdtContents::Map(entries) => {
                map_extent(entries.iter().map(|entry| (&entry.key, &entry.value)))
            }
            crate::CdtContents::List(_) => unreachable!("parse_map yields a map"),
        };
        assert_eq!(map.extent(), walked);
        assert_eq!(map.extent().elements, map.element_count());
        assert_eq!(map.extent().bytes, canonical_lexical(&map).len());
    }

    /// An empty composite carries the empty extent, and a value built one level at a
    /// time carries the extent of the whole tree at every level.
    #[test]
    fn the_extent_accumulates_through_programmatic_nesting() {
        assert_eq!(CdtValue::empty_list().extent(), Extent::EMPTY);
        assert_eq!(CdtValue::empty_map().extent(), Extent::EMPTY);
        let mut value = CdtValue::empty_list();
        for level in 1..64usize {
            value = CdtValue::list(vec![CdtTerm::composite(value).expect("within both bounds")])
                .expect("within both bounds");
            assert_eq!(value.extent().elements, level);
            assert_eq!(value.extent().bytes, 2 * (level + 1));
            assert_eq!(value.element_count(), level);
            assert_eq!(value.depth(), level + 1);
        }
    }
}

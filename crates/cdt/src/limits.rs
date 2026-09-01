// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The three resource bounds **every** [`crate::CdtValue`] satisfies, and the machinery that
//! establishes them.
//!
//! # The bounds are an invariant of the type, not a property of one code path
//!
//! There is no way to obtain a [`crate::CdtValue`] except through a constructor in this
//! crate, and every one of them either checks all three bounds here or is reached only
//! from a caller that just did. Concretely:
//!
//! * [`crate::CdtValue::list`] and [`crate::CdtValue::map`] measure the prospective value and refuse
//!   before returning it;
//! * [`crate::parse_cdt`] enforces the depth and element bounds *as it scans* — before
//!   the offending element is allocated — and the byte bound both on the input it is
//!   offered and on the canonical form the result would have;
//! * [`crate::functions`] measures each prospective result from borrowed inputs and
//!   refuses before cloning anything;
//! * [`crate::CdtValue::empty_list`] and [`crate::CdtValue::empty_map`] are within every bound by
//!   inspection.
//!
//! That is why the crate's public constructors return `Result` rather than the value:
//! a bound is not advice a caller may decline. A consumer that only ever *reads* a
//! [`crate::CdtValue`] can rely on the invariant without re-checking it, which is what makes
//! the recursive `Drop`, `Clone` and `Debug` glue on this owning tree safe.
//!
//! # Every check here is iterative
//!
//! [`term_extent`], [`list_extent`] and [`map_extent`] walk with an
//! explicit heap worklist and never recurse. A bound check that could itself overflow
//! the stack would be checking for the very thing it caused, and in Rust a stack
//! overflow is an `abort` no caller can catch.
//!
//! # Why this crate owns its own bounds
//!
//! A composite-datatype lexical form is **attacker-controlled data inside a
//! literal**: it arrives as the lexical form of an RDF term, so a hostile dataset
//! (or a hostile SPARQL literal) chooses its shape entirely. In Rust, exhausting
//! the stack is an `abort`, not a catchable panic, so a scanner that recursed on
//! this input could be turned into an uncatchable process kill by a payload as
//! small as two megabytes of `[[[[…`. The whole crate is therefore iterative — the
//! scanner, the renderer, equality, ordering and the canonical mapping all carry an
//! explicit heap worklist — and these three bounds cap the *heap* the input can
//! command.
//!
//! # Why NOT the RDF parser's nesting bound
//!
//! `purrdf-rdf` has a nesting bound of its own for Turtle/TriG collections. It is
//! deliberately not reused here, and not only because it is `pub(crate)` in a crate
//! this closed leaf must not depend on. It counts a **different quantity**: it
//! bounds the number of live *stack frames* in a recursive-descent parser, so its
//! value is calibrated to the native thread stack (frames × frame size < stack).
//! This crate's scanner never recurses, so it has no per-level stack cost at all;
//! what [`MAX_NESTING_DEPTH`] bounds is the depth of the composite **value tree**,
//! which is a data-shape question about heap residency and teardown. Importing a
//! stack-frame budget to answer a heap-shape question would pin the wrong number to
//! the wrong reason, and would silently drift the moment either side was retuned.

use alloc::vec::Vec;

use crate::error::CdtError;
use crate::render::{key_lexical_len, term_lexical_len};
use crate::term::{CdtKey, CdtTerm};
use crate::value::CdtContents;

/// Maximum nesting depth of composite values in one CDT lexical form.
///
/// The top-level `cdt:List` / `cdt:Map` is depth 1; a list inside a list is depth 2.
/// A form that would exceed this is [`crate::CdtError::DepthExceeded`].
///
/// **Governor justification.** Each level is a `Vec` header plus one heap
/// allocation, so the direct heap cost per level is small; the reason a *bound*
/// exists at all is teardown. A parsed [`crate::CdtValue`] is an owning tree whose
/// `Drop` glue is the compiler-generated recursive one, so dropping a value of depth
/// *d* uses *d* stack frames — the one place in this crate where input depth reaches
/// the stack, and it is unavoidable for a tree-shaped owning value. 64 keeps that
/// teardown to a few kilobytes of stack on any host, including the 1 MiB main stack
/// wasm engines give a module, while still exceeding by orders of magnitude any
/// composite a query or dataset plausibly authors (SEP-0009's own conformance corpus
/// nests two deep). It is a *data-shape* budget, not a parser-stack budget.
pub const MAX_NESTING_DEPTH: usize = 64;

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
/// input bytes per element) from amplifying into an unbounded parsed value, which is
/// a different attack from a deep one and needs its own cap.
pub const MAX_ELEMENTS: usize = 1 << 20;

/// Maximum length, in bytes, of a CDT lexical form offered to the scanner.
///
/// A longer input is refused up front as [`crate::CdtError::InputTooLarge`], before
/// a single byte is scanned or a single allocation is made.
///
/// **Governor justification.** This is the outermost of the three: it bounds the
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
/// [`crate::functions`] that mint check the canonical length they are about to create.
pub const MAX_LEXICAL_BYTES: usize = 64 * 1024 * 1024;

// ── Measuring a value, or a value that does not exist yet ───────────────────────

/// The shape a composite has, or would have if it were built.
///
/// Measured from **borrowed** parts, so a value that will be refused is never
/// allocated: `cdt:put(?m, ?k, ?m)` roughly doubles a map's element count on every
/// application, and a query of twenty-one lines could otherwise ask for a value no
/// host can hold.
pub(crate) struct Extent {
    /// Elements at every level, counted the way [`crate::CdtValue::element_count`] counts
    /// them.
    pub(crate) elements: usize,
    /// Nesting depth, counted the way [`crate::CdtValue::depth`] counts it: the composite
    /// itself is 1.
    pub(crate) depth: usize,
    /// The exact byte length of the canonical lexical form the composite would have.
    pub(crate) bytes: usize,
}

/// The elements and nesting depth one element contributes to its container.
///
/// Returns `(elements, depth)` where `depth` is 0 for a leaf, so a container adds 1 to
/// the maximum over its own elements. Iterative, with an explicit worklist.
pub(crate) fn term_extent(term: &CdtTerm) -> (usize, usize) {
    let mut elements = 0usize;
    let mut depth = 0usize;
    let mut work: Vec<(&CdtTerm, usize)> = alloc::vec![(term, 0usize)];
    while let Some((current, level)) = work.pop() {
        match current {
            CdtTerm::Composite(inner) => {
                let value = inner.as_ref();
                let here = level.saturating_add(1);
                if here > depth {
                    depth = here;
                }
                elements = elements.saturating_add(value.len());
                match value.contents() {
                    CdtContents::List(items) => {
                        work.extend(items.iter().map(|item| (item, here)));
                    }
                    CdtContents::Map(entries) => {
                        work.extend(entries.iter().map(|entry| (&entry.value, here)));
                    }
                }
            }
            CdtTerm::TripleTerm(triple) => {
                work.push((&triple.subject, level));
                work.push((&triple.predicate, level));
                work.push((&triple.object, level));
            }
            CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::Literal(_) | CdtTerm::Null => {}
        }
    }
    (elements, depth)
}

/// The extent of the list these elements would form.
pub(crate) fn list_extent<'a>(items: impl IntoIterator<Item = &'a CdtTerm>) -> Extent {
    let mut count = 0usize;
    let mut elements = 0usize;
    let mut depth = 0usize;
    let mut bytes = 0usize;
    for term in items {
        count = count.saturating_add(1);
        let (inner_elements, inner_depth) = term_extent(term);
        elements = elements.saturating_add(inner_elements);
        if inner_depth > depth {
            depth = inner_depth;
        }
        bytes = bytes.saturating_add(term_lexical_len(term));
    }
    Extent {
        elements: elements.saturating_add(count),
        depth: depth.saturating_add(1),
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
    let mut depth = 0usize;
    let mut bytes = 0usize;
    for (key, value) in pairs {
        count = count.saturating_add(1);
        let (inner_elements, inner_depth) = term_extent(value);
        elements = elements.saturating_add(inner_elements);
        if inner_depth > depth {
            depth = inner_depth;
        }
        // `key` `:` `value`.
        bytes = bytes
            .saturating_add(key_lexical_len(key))
            .saturating_add(1)
            .saturating_add(term_lexical_len(value));
    }
    Extent {
        elements: elements.saturating_add(count),
        depth: depth.saturating_add(1),
        // `{`, `}`, and one `,` between each adjacent pair.
        bytes: bytes
            .saturating_add(2)
            .saturating_add(count.saturating_sub(1)),
    }
}

/// Check a composite, built or prospective, against all three bounds.
///
/// The offsets these errors carry are positions in a lexical form, and a value with no
/// scanned input has only the canonical form it would have; each offset therefore names
/// the position the offending construct would occupy there, which for the opening
/// delimiter is byte 0.
pub(crate) fn check_extent(extent: &Extent) -> Result<(), CdtError> {
    if extent.depth > MAX_NESTING_DEPTH {
        return Err(CdtError::DepthExceeded {
            offset: 0,
            limit: MAX_NESTING_DEPTH,
        });
    }
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

/// Check a prospective **element** against all three bounds.
///
/// An element is not itself a composite value, so it carries no invariant of its own;
/// what this answers is whether the element could ever appear in one. The measure is
/// therefore the **smallest composite that could hold it** — the one-element list
/// `[term]`, which is one level deeper, one element larger and two bytes longer than
/// the element. An element that fails here can never be placed anywhere, so
/// [`CdtTerm::composite`](crate::CdtTerm::composite) and
/// [`CdtTerm::triple`](crate::CdtTerm::triple) refuse it where it is made rather than
/// leaving the caller to discover it when it is finally placed. A triple term is the
/// case that most needs this: it combines three separately admissible elements' element
/// counts and canonical lengths into one that need not be.
pub(crate) fn check_term(term: &CdtTerm) -> Result<(), CdtError> {
    let (elements, depth) = term_extent(term);
    check_extent(&Extent {
        elements: elements.saturating_add(1),
        depth: depth.saturating_add(1),
        bytes: term_lexical_len(term).saturating_add(2),
    })
}

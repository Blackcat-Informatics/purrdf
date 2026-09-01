// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The three resource bounds every CDT lexical form is scanned under.
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
pub const MAX_LEXICAL_BYTES: usize = 64 * 1024 * 1024;

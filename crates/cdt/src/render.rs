// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The canonical lexical mapping: [`canonical_lexical`].
//!
//! SEP-0009 defines **no** canonical form for `cdt:List` / `cdt:Map`. This module
//! therefore *chooses* one, for exactly the same reason and with exactly the same
//! standing as `purrdf_xsd::XsdValue::canonical_lexical` choosing the XSD canonical
//! mapping: whenever PurRDF **computes** a composite value it must write some
//! lexical form, and a serializer that writes different bytes for the same value on
//! two runs breaks the workspace's byte-determinism rule. Values PurRDF merely
//! *carries* are never rewritten — the kernel keeps literals lexical-verbatim.
//!
//! # The form
//!
//! * A list is `[`, its elements separated by a single `,`, `]`. No whitespace
//!   anywhere. An empty list is `[]`.
//! * A map is `{`, its entries separated by a single `,`, `}`, each entry spelled
//!   `key` `:` `value` with no whitespace. Entries are written in the crate's
//!   syntactic key order ([`crate::total_key_cmp`]), which is what makes the output
//!   independent of authoring order. An empty map is `{}`.
//! * An IRI is `<`, the IRI text with the `IRIREF`-forbidden code points escaped as
//!   `\u00XX`, `>`.
//! * A blank node is `_:` followed by its label.
//! * A literal is **always explicit**: `"lexical"^^<datatype>`, with no shorthand —
//!   never a bare `1`, `1.5`, `1e0`, `true` or `"abc"`. A language-tagged literal is
//!   `"lexical"@tag`, and a directional one `"lexical"@tag--ltr` / `--rtl`. The
//!   lexical form is written verbatim under the SPARQL string escape set.
//! * A triple term is `<<(`, the three components separated by a single space,
//!   `)>>`.
//! * The null element is `null`.
//!
//! # Total by construction, and iterative
//!
//! [`canonical_lexical`] takes `&CdtValue` and returns `String`: there is no failure
//! mode, because every inhabitant of the type has a spelling. It walks the value
//! with an explicit heap job stack and never recurses, so it is safe on a value of
//! any admissible depth.
//!
//! # Measuring without materialising
//!
//! [`canonical_lexical_len`] answers "how many bytes would that be?" without
//! allocating them. It is not a second, parallel spelling of the form — it drives
//! the **same** walker through a different [`Sink`], so the two can never drift.
//! [`crate::functions`] needs it: a minted composite must be refused *before* it is
//! built when its canonical form would exceed [`crate::MAX_LEXICAL_BYTES`], and
//! rendering the very thing you are trying not to allocate is not a bound check.

use alloc::string::String;
use alloc::vec::Vec;

use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm};
use crate::value::{CdtContents, CdtValue};

/// Uppercase hex digits, for the `\u00XX` escape forms.
const HEX_UPPER: [u8; 16] = *b"0123456789ABCDEF";

/// One step of the iterative renderer.
enum Job<'a> {
    /// Render a term (which may open a nested composite).
    Term(&'a CdtTerm),
    /// Render a map key (always a leaf).
    Key(&'a CdtKey),
    /// Emit a fixed piece of punctuation.
    Punct(&'static str),
}

/// Where the canonical renderer puts its output.
///
/// There are exactly two implementations — [`String`], which materialises the form,
/// and [`Measure`], which keeps only its byte length — and **one** walker drives
/// both. Measuring is therefore guaranteed to agree with rendering, byte for byte,
/// with no second description of the form to fall out of step.
trait Sink {
    /// Append a string slice.
    fn put_str(&mut self, text: &str);
    /// Append one character.
    fn put_char(&mut self, ch: char);
}

impl Sink for String {
    fn put_str(&mut self, text: &str) {
        self.push_str(text);
    }

    fn put_char(&mut self, ch: char) {
        self.push(ch);
    }
}

/// A [`Sink`] that materialises nothing and accumulates only the byte length.
///
/// Saturating rather than wrapping: a length that overflows `usize` is astronomically
/// over every bound this crate enforces, and saturating keeps the comparison against
/// [`crate::MAX_LEXICAL_BYTES`] correct instead of wrapping it to a small number.
struct Measure(usize);

impl Sink for Measure {
    fn put_str(&mut self, text: &str) {
        self.0 = self.0.saturating_add(text.len());
    }

    fn put_char(&mut self, ch: char) {
        self.0 = self.0.saturating_add(ch.len_utf8());
    }
}

/// The canonical lexical form of a composite value.
///
/// Byte-identical across runs, processes and hosts: the walk order is the value's
/// own order, map entries are written in the crate's total key order, and no hash
/// iteration, clock or RNG is consulted.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtDatatype, canonical_lexical, parse_cdt};
///
/// // Shorthands become explicit; whitespace disappears.
/// let v = parse_cdt("[ 1 , null ]", CdtDatatype::List)?;
/// assert_eq!(
///     canonical_lexical(&v),
///     "[\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>,null]"
/// );
///
/// // Re-parsing the canonical form yields the same value (a fixpoint).
/// let again = parse_cdt(&canonical_lexical(&v), CdtDatatype::List)?;
/// assert_eq!(canonical_lexical(&again), canonical_lexical(&v));
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn canonical_lexical(value: &CdtValue) -> String {
    let mut out = String::new();
    let mut jobs: Vec<Job<'_>> = Vec::new();
    push_value(&mut jobs, value);
    run(&mut out, jobs);
    out
}

/// The **byte length** of [`canonical_lexical`], computed without allocating it.
///
/// Exactly equal to `canonical_lexical(value).len()` for every value, because both
/// drive the same walker (see the [`Sink`] trait); this one just never keeps the
/// bytes. That is what lets [`crate::functions`] check a prospective composite
/// against [`crate::MAX_LEXICAL_BYTES`] *before* deciding to build it.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{canonical_lexical, canonical_lexical_len, parse_list};
///
/// let value = parse_list("[1, 'two'@en, [ true ]]")?;
/// assert_eq!(canonical_lexical_len(&value), canonical_lexical(&value).len());
/// # Ok::<(), purrdf_cdt::CdtError>(())
/// ```
#[must_use]
pub fn canonical_lexical_len(value: &CdtValue) -> usize {
    let mut out = Measure(0);
    let mut jobs: Vec<Job<'_>> = Vec::new();
    push_value(&mut jobs, value);
    run(&mut out, jobs);
    out.0
}

/// The canonical lexical form of a single map key, used by the duplicate-key
/// diagnostic.
#[must_use]
pub fn canonical_key_lexical(key: &CdtKey) -> String {
    let mut out = String::new();
    write_key(&mut out, key);
    out
}

/// The byte length one element occupies in a canonical form, without allocating it.
pub(crate) fn term_lexical_len(term: &CdtTerm) -> usize {
    let mut out = Measure(0);
    run(&mut out, alloc::vec![Job::Term(term)]);
    out.0
}

/// The byte length one map key occupies in a canonical form, without allocating it.
pub(crate) fn key_lexical_len(key: &CdtKey) -> usize {
    let mut out = Measure(0);
    write_key(&mut out, key);
    out.0
}

/// Drive the renderer's job stack into a sink. Iterative: nesting costs heap, never
/// stack.
fn run<S: Sink>(out: &mut S, mut jobs: Vec<Job<'_>>) {
    while let Some(job) = jobs.pop() {
        match job {
            Job::Punct(text) => out.put_str(text),
            Job::Key(key) => write_key(out, key),
            Job::Term(term) => match term {
                CdtTerm::Composite(inner) => push_value(&mut jobs, inner.as_ref()),
                CdtTerm::TripleTerm(triple) => push_triple(&mut jobs, triple.as_ref()),
                CdtTerm::Iri(iri) => write_iri(out, iri),
                CdtTerm::Blank(label) => {
                    out.put_str("_:");
                    out.put_str(label);
                }
                CdtTerm::Literal(literal) => write_literal(out, literal),
                CdtTerm::Null => out.put_str("null"),
            },
        }
    }
}

/// Push the jobs for a composite, in reverse emission order (the stack pops LIFO).
fn push_value<'a>(jobs: &mut Vec<Job<'a>>, value: &'a CdtValue) {
    match value.contents() {
        CdtContents::List(items) => {
            jobs.push(Job::Punct("]"));
            for (index, item) in items.iter().enumerate().rev() {
                jobs.push(Job::Term(item));
                if index > 0 {
                    jobs.push(Job::Punct(","));
                }
            }
            jobs.push(Job::Punct("["));
        }
        CdtContents::Map(entries) => {
            jobs.push(Job::Punct("}"));
            for (index, CdtEntry { key, value: item }) in entries.iter().enumerate().rev() {
                jobs.push(Job::Term(item));
                jobs.push(Job::Punct(":"));
                jobs.push(Job::Key(key));
                if index > 0 {
                    jobs.push(Job::Punct(","));
                }
            }
            jobs.push(Job::Punct("{"));
        }
    }
}

/// Push the jobs for a triple term, in reverse emission order.
fn push_triple<'a>(jobs: &mut Vec<Job<'a>>, triple: &'a CdtTripleTerm) {
    jobs.push(Job::Punct(")>>"));
    jobs.push(Job::Term(&triple.object));
    jobs.push(Job::Punct(" "));
    jobs.push(Job::Term(&triple.predicate));
    jobs.push(Job::Punct(" "));
    jobs.push(Job::Term(&triple.subject));
    jobs.push(Job::Punct("<<("));
}

fn write_key<S: Sink>(out: &mut S, key: &CdtKey) {
    match key {
        CdtKey::Iri(iri) => write_iri(out, iri),
        CdtKey::Literal(literal) => write_literal(out, literal),
    }
}

fn write_iri<S: Sink>(out: &mut S, iri: &str) {
    out.put_char('<');
    for ch in iri.chars() {
        if is_iri_forbidden(ch) {
            push_uchar(out, ch);
        } else {
            out.put_char(ch);
        }
    }
    out.put_char('>');
}

fn write_literal<S: Sink>(out: &mut S, literal: &CdtLiteral) {
    out.put_char('"');
    for ch in literal.lexical.chars() {
        match ch {
            '\\' => out.put_str("\\\\"),
            '"' => out.put_str("\\\""),
            '\n' => out.put_str("\\n"),
            '\r' => out.put_str("\\r"),
            '\t' => out.put_str("\\t"),
            '\u{8}' => out.put_str("\\b"),
            '\u{c}' => out.put_str("\\f"),
            c if c.is_control() => push_uchar(out, c),
            c => out.put_char(c),
        }
    }
    out.put_char('"');
    match &literal.language {
        Some(language) => {
            out.put_char('@');
            out.put_str(language);
            if let Some(direction) = literal.direction {
                out.put_str("--");
                out.put_str(direction.as_str());
            }
        }
        None => {
            out.put_str("^^");
            write_iri(out, &literal.datatype);
        }
    }
}

/// The code points an `IRIREF` may not carry raw: the grammar's own delimiters, the
/// space, and every control code point (C0, DEL and the C1 block).
fn is_iri_forbidden(ch: char) -> bool {
    matches!(
        ch,
        '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' | ' '
    ) || ch.is_control()
}

/// Push a `UCHAR` escape, in the narrow `\uXXXX` form for a code point in the Basic
/// Multilingual Plane and the wide `\UXXXXXXXX` form above it. Both escape sets this
/// module applies — the control code points and the `IRIREF` delimiters — lie in the
/// BMP, so the narrow form is the one an emitted canonical form actually carries; the
/// wide branch is what keeps the function total over `char`.
fn push_uchar<S: Sink>(out: &mut S, ch: char) {
    let value = ch as u32;
    if value <= 0xFFFF {
        out.put_str("\\u");
        push_hex(out, value, 4);
    } else {
        out.put_str("\\U");
        push_hex(out, value, 8);
    }
}

fn push_hex<S: Sink>(out: &mut S, value: u32, digits: u32) {
    for shift in (0..digits).rev() {
        let nibble = (value >> (shift * 4)) & 0xF;
        out.put_char(HEX_UPPER[nibble as usize] as char);
    }
}

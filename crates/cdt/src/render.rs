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

use alloc::string::String;
use alloc::vec::Vec;

use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm};
use crate::value::CdtValue;

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
    while let Some(job) = jobs.pop() {
        match job {
            Job::Punct(text) => out.push_str(text),
            Job::Key(key) => write_key(&mut out, key),
            Job::Term(term) => match term {
                CdtTerm::Composite(inner) => push_value(&mut jobs, inner.as_ref()),
                CdtTerm::TripleTerm(triple) => push_triple(&mut jobs, triple.as_ref()),
                CdtTerm::Iri(iri) => write_iri(&mut out, iri),
                CdtTerm::Blank(label) => {
                    out.push_str("_:");
                    out.push_str(label);
                }
                CdtTerm::Literal(literal) => write_literal(&mut out, literal),
                CdtTerm::Null => out.push_str("null"),
            },
        }
    }
    out
}

/// The canonical lexical form of a single map key, used by the duplicate-key
/// diagnostic.
#[must_use]
pub fn canonical_key_lexical(key: &CdtKey) -> String {
    let mut out = String::new();
    write_key(&mut out, key);
    out
}

/// Push the jobs for a composite, in reverse emission order (the stack pops LIFO).
fn push_value<'a>(jobs: &mut Vec<Job<'a>>, value: &'a CdtValue) {
    match value {
        CdtValue::List(items) => {
            jobs.push(Job::Punct("]"));
            for (index, item) in items.iter().enumerate().rev() {
                jobs.push(Job::Term(item));
                if index > 0 {
                    jobs.push(Job::Punct(","));
                }
            }
            jobs.push(Job::Punct("["));
        }
        CdtValue::Map(entries) => {
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

fn write_key(out: &mut String, key: &CdtKey) {
    match key {
        CdtKey::Iri(iri) => write_iri(out, iri),
        CdtKey::Literal(literal) => write_literal(out, literal),
    }
}

fn write_iri(out: &mut String, iri: &str) {
    out.push('<');
    for ch in iri.chars() {
        if is_iri_forbidden(ch) {
            push_uchar(out, ch);
        } else {
            out.push(ch);
        }
    }
    out.push('>');
}

fn write_literal(out: &mut String, literal: &CdtLiteral) {
    out.push('"');
    for ch in literal.lexical.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if c.is_control() => push_uchar(out, c),
            c => out.push(c),
        }
    }
    out.push('"');
    match &literal.language {
        Some(language) => {
            out.push('@');
            out.push_str(language);
            if let Some(direction) = literal.direction {
                out.push_str("--");
                out.push_str(direction.as_str());
            }
        }
        None => {
            out.push_str("^^");
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

/// Push a `UCHAR` escape. Every code point this crate escapes is a control code
/// point or an ASCII delimiter, so the short `\u00XX` form always suffices; the
/// wider `\UXXXXXXXX` form is emitted for anything else a future caller escapes.
fn push_uchar(out: &mut String, ch: char) {
    let value = ch as u32;
    if value <= 0xFFFF {
        out.push_str("\\u");
        push_hex(out, value, 4);
    } else {
        out.push_str("\\U");
        push_hex(out, value, 8);
    }
}

fn push_hex(out: &mut String, value: u32, digits: u32) {
    for shift in (0..digits).rev() {
        let nibble = (value >> (shift * 4)) & 0xF;
        out.push(HEX_UPPER[nibble as usize] as char);
    }
}

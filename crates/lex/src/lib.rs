// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-lex` — the **lexical foundations** shared by every grammar in the
//! PurRDF workspace.
//!
//! A pure-Rust, wasm-clean crate whose one runtime dependency is the
//! zero-dependency [`purrdf_hash`] root (its hex-digit reader). Every reader and
//! writer above it — the IRI parser, the Turtle/TriG/N-Triples/N-Quads codecs,
//! the SPARQL lexer, the SPARQL results writers, JSON-LD, JSON Schema and the
//! GTS container — decides token boundaries and escapes with the code here, so
//! each lexical law is written down once and every grammar agrees with every
//! other.
//!
//! # Scope
//!
//! This crate is the home of the workspace's lexical layer: byte-class
//! scanning, grammar terminals, term syntax, literal and IRI escaping, percent
//! encoding, JSON strings, JSON pointers, a JSON reader and writer, an XML
//! reader and Unicode normalisation. Each is a law a grammar states; none is a
//! vocabulary, and nothing here mints an IRI.
//!
//! * **Grammar terminals** — [`terminals`], the exact Turtle/SPARQL character
//!   classes (`WS`, `PN_CHARS_BASE`, `PN_CHARS_U`, `PN_CHARS`, `VARNAME`), the
//!   XML 1.0 `Char`, `NameStartChar` and `NameChar` classes, and the Unicode
//!   `White_Space` property, each a range table proved sorted and disjoint at
//!   compile time and answered below U+0100 by one class-table load. A
//!   scanner's character class decides token BOUNDARIES under maximal munch,
//!   so an approximation misparses documents rather than merely widening the
//!   accepted language; one transcription is the only way to keep the scanners
//!   agreeing with each other.
//! * **Byte-class scanning** — [`scan`], the chunked scanners built from those
//!   tables ([`terminals::find_first_trivia`],
//!   [`terminals::find_first_iri_body_special`],
//!   [`terminals::find_first_json_string_special`],
//!   [`terminals::find_first_xml_special`]) and [`terminals::ByteClass`], the
//!   same kernel over a caller's own class. Each finds the first byte of a
//!   class sixteen bytes at a time, as run compares with no data-dependent
//!   branch: the shape LLVM lowers to packed byte compares and a mask
//!   extraction on x86_64, aarch64 and wasm `simd128`, and to straight-line
//!   scalar code where the target has no vector unit.
//!   [`scan::find_byte`] and [`scan::find_byte2`] are the same kernel for a
//!   needle known only at run time.
//! * **Escape decoding** — [`terminals::decode_uchar`], [`terminals::echar_value`]
//!   and [`terminals::decode_char_ref`]: the `UCHAR`, `ECHAR` and XML `CharRef`
//!   decoders every RDF, SPARQL, ShEx and XML reader shares, with every digit
//!   read by [`purrdf_hash::hex::nibble`], so a sign is never a digit.
//! * **Grammar whitespace** — [`terminals::skip_ws`] and [`terminals::trim_ws`]:
//!   the `WS` / XML `S` / JSON `ws` skip and trim, four scalars and never FORM
//!   FEED.
//! * **JSON strings** — [`json_escape`], the one RFC 8259 §7 string body
//!   escaper every PurRDF JSON writer shares, over the JSON string-body
//!   scanner ([`json_escape::JsonEscapes`] names the spellings those writers
//!   pin), and the one decoder every reader shares ([`json_escape::unescape`]),
//!   which refuses an unpaired surrogate.
//! * **JSON Pointer** — [`json_pointer`], RFC 6901 reference tokens.
//! * **Percent-encoding** — [`percent`], the RFC 3986 encoder over the sets
//!   the specifications define, the strict and form-urlencoded decoders, and
//!   RFC 3986 §6.2.2 normalization.
//!
//! # Examples
//!
//! Find where a run of Turtle/SPARQL whitespace ends, test a terminal, and
//! escape a JSON string body:
//!
//! ```rust
//! use purrdf_lex::json_escape::{JsonEscapes, push_string};
//! use purrdf_lex::terminals::{find_first_trivia, is_pn_chars_base};
//!
//! assert_eq!(find_first_trivia(b" \t\r\n?s"), Some(4));
//! assert!(is_pn_chars_base('é'));
//! assert!(!is_pn_chars_base('_'));
//!
//! let mut out = String::new();
//! push_string(&mut out, "say \"hi\"\n", JsonEscapes::Minimal);
//! assert_eq!(out, r#""say \"hi\"\n""#);
//! ```
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![forbid(unsafe_code)]

pub mod json_escape;
pub mod json_pointer;
pub mod percent;
pub mod scan;
pub mod terminals;

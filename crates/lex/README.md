<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-lex` — Lexical Foundations for PurRDF's Grammars

[![crates.io](https://img.shields.io/crates/v/purrdf-lex.svg)](https://crates.io/crates/purrdf-lex)
[![docs.rs](https://docs.rs/purrdf-lex/badge.svg)](https://docs.rs/purrdf-lex)
[![License](https://img.shields.io/badge/license-(MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0)%20AND%20Unicode--3.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-lex` is the lexical layer every grammar in the PurRDF toolkit shares:
the IRI parser, the Turtle/TriG/N-Triples/N-Quads codecs, the SPARQL lexer,
the SPARQL results writers, JSON-LD, JSON Schema and the GTS container all
decide token boundaries and escapes with it, so each lexical law is written
down once. Its one runtime dependency is `purrdf-hash`, the workspace's
zero-dependency root (for its hex-digit reader); it contains no `unsafe` code
and builds for `wasm32-unknown-unknown`.

Its scope is the workspace's lexical foundations: byte-class scanning,
grammar terminals, term syntax, literal and IRI escaping, percent encoding,
JSON strings, JSON pointers, a JSON reader and writer, an XML reader and
Unicode normalisation.

| Module | What it holds | Specification |
|---|---|---|
| `terminals` | The exact character classes `WS`, `PN_CHARS_BASE`, `PN_CHARS_U`, `PN_CHARS`, `VARNAME`, `IRIREF`'s forbidden set, XML `Char`, `NameStartChar`, `NameChar`, Unicode `White_Space`, and ECMA-262 `LineTerminator` and `\s`, each a range table proved sorted and disjoint at compile time | SPARQL 1.2 Query §19.8, RDF 1.2 Turtle §6.5, XML 1.0 (Fifth Edition) §2.2–2.3, RFC 8259 §7, ECMA-262 §12.2–12.3 and §22.2.2.9 |
| `scan` | Chunked byte-class scanners over those tables (`find_first_trivia`, `find_first_iri_body_special`, `find_first_json_string_special`, `find_first_xml_special`) and `ByteClass`, the same kernel over a caller's own class | — |
| `terminals` (escapes and `WS`) | `decode_uchar`, `echar_value`, `expand_uchars` and `decode_char_ref`, strict decoders of `UCHAR`, `ECHAR` and the XML `CharRef`; `skip_ws` and `trim_ws`, the four-scalar `WS` skip and trim; `is_ncname`; `in_ranges`, the one range-table search | RDF 1.2 Turtle §6.5, SPARQL 1.2 §19.8, XML 1.0 §4.1 `[66]`, Namespaces in XML 1.0 §3 |
| `scan` (needles) | `find_byte` and `find_byte2`, the first occurrence of a byte known only at run time | — |
| `json_escape` | The one JSON string-body escaper every PurRDF JSON writer shares, in the four spellings those writers pin (`JsonEscapes`), and the one decoder every reader shares (`unescape`, `decode_escape`, `decode_u_escape`), which refuses an unpaired surrogate | RFC 8259 §7 |
| `json_pointer` | Reference-token escaping and unescaping, pointer parsing, the array-index token | RFC 6901 |
| `percent` | `encode` over the specification-defined sets (`UNRESERVED`, `REG_NAME`, `PATH`, `FRAGMENT`, `URI_TEMPLATE_RESERVED`, `NON_ASCII`, …), strict `decode`, `decode_form`, `normalize` | RFC 3986 §2, §3, §6.2.2; RFC 3987 §3.1; RFC 6570 §3.2.3 |
| `unicode` | The workspace's one normalization pipeline: `nfc`, `nfd`, `nfkc`, `nfkd`, `is_nfc` and `ccc`, and the streaming stages (`Decompose`, `Compose`, `drive`) a caller composes with its own stage, over tables generated from the vendored Unicode Character Database at `UNICODE_VERSION`, the version every Unicode table in the workspace is generated from | UAX 15; Unicode core specification §3.11–3.12 |

## Why a scanner may not approximate

A character class inside a scanner is a **boundary** test, not a membership
test: under maximal munch, widening the class moves the token boundary in
documents both the liberal and the exact scanner accept. A `PN_CHARS` that
admits every scalar above `0x7F` absorbs U+00A0 NO-BREAK SPACE into a variable
name, and a join silently becomes a cross product. Every terminal is therefore
spelled once, here, with its production cited.

## Usage

```rust
use purrdf_lex::json_escape::{JsonEscapes, push_string};
use purrdf_lex::terminals::{ByteClass, byte_run_count, find_first_trivia, is_pn_chars_base};

// The end of a run of Turtle/SPARQL whitespace.
assert_eq!(find_first_trivia(b" \t\r\n?s"), Some(4));

// A grammar terminal.
assert!(is_pn_chars_base('é'));
assert!(!is_pn_chars_base('_'));

// A writer's own class, scanned by the same kernel.
const QUOTES: [u8; 256] = {
    let mut table = [0_u8; 256];
    table[b'"' as usize] = 1;
    table
};
const QUOTE: ByteClass<{ byte_run_count(&QUOTES) }> = ByteClass::from_table(QUOTES);
assert_eq!(QUOTE.find_first(b"say \"hi\""), Some(4));

// An RFC 8259 JSON string.
let mut out = String::new();
push_string(&mut out, "say \"hi\"\n", JsonEscapes::Minimal);
assert_eq!(out, r#""say \"hi\"\n""#);
```

## The scan kernel

Every scanner walks its input in sixteen-byte chunks. A class is projected at
compile time onto its maximal runs of member bytes, and each byte is tested
against a run by one wrapping subtraction and one unsigned comparison, with no
data-dependent branch. The sixteen answers of a chunk are OR-folded for a
branch-free clean-chunk test; in the one chunk that holds a hit, the lanes read
as a little-endian `u128` give the first hit's offset as their trailing-zero
count over eight. That is the shape LLVM lowers to packed byte compares and a
mask extraction — `pcmpeqb`/`pmovmskb` on x86_64, `cmeq`/`cmhi` and `addp` on
aarch64, `i8x16.eq`/`i8x16.lt_u` under wasm `simd128` — and to straight-line
scalar code where the target has no vector unit. `scripts/check-simd-asm.py`
holds each scanner to that shape in the emitted assembly on seven target
configurations, and the equivalence tests compare every scanner with the
per-byte search it replaces.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate; `purrdf-iri` re-exports this
crate's modules as `purrdf_iri::terminals`, `purrdf_iri::scan` and
`purrdf_iri::json_escape`.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)

The normalization tables in `src/unicode_tables.rs` are generated from the
Unicode Character Database and ship Unicode, Inc. data under the
[Unicode License v3](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSES/Unicode-3.0.txt)
in addition; see [LICENSING.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md).

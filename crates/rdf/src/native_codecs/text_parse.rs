// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party RDF text → in-memory [`SerGraph`] front-end for the line / Turtle
//! family (N-Triples, N-Quads, Turtle, TriG).
//!
//! This module REPLACES the `purrdf-gts` `from_ntriples` / `from_nquads` /
//! `from_turtle` / `from_trig` text codecs (which delegated all RDF text parsing
//! to the EXTERNAL crate, FORBIDDEN here) with an in-repo parser that lowers
//! directly to the first-party in-memory [`SerGraph`] the purrdf-gts roundtrip used to
//! produce — WITHOUT the text→GTS-bytes→reader indirection.
//!
//! ## Byte-identity discipline
//!
//! The downstream fold ([`super::parse::dataset_from_ser_graph`]) re-interns its
//! [`RdfDatasetBuilder`] from `graph.reifiers` THEN `graph.quads`, in order, so the
//! frozen IR's term table is the first-seen interning order over those rows. To stay
//! BYTE-IDENTICAL to the prior purrdf-gts path this parser reproduces, exactly, the
//! `from_nquads` `build_gts` structure: terms in first-seen order, quads in statement
//! order, reifiers in encounter order, the `rdf:reifies` statement-layer shorthand, and
//! the self-reifier sentinel for inline quoted-triple TERMS. The prior purrdf-gts
//! `Writer` / `read` roundtrip was append-order-preserving (it did NOT sort
//! terms/quads/reifiers), so the in-memory graph the reader produced was already exactly
//! this structure — only the serialize / deserialize hop, and the `\uXXXX` UCHAR-in-IRI
//! gap, are removed.
//!
//! ## The UCHAR fix (W3C `test060`)
//!
//! The purrdf-gts N-Quads/Turtle IRIREF readers took the raw bytes between `<` and
//! `>` and REJECTED a backslash as a forbidden IRI character, so `\uXXXX` UCHAR
//! escapes inside an IRIREF (`<urn:ex:s:000:s⁰1>`) failed to parse. This
//! front-end decodes `\u`/`\U` UCHAR escapes inside IRIREFs (via the proven
//! sparql-algebra lexer, which decodes them in `IRIREF` position), so `test060`
//! now parses.

use std::collections::HashMap;

use purrdf_iri::terminals::is_ws;
use purrdf_iri::{BaseOrigin, BaseScope, Iri, IriError, Position};
use purrdf_sparql_algebra::lexer::{Spanned, Token, tokenize, tokenize_turtle};
use rayon::prelude::*;

use super::media_type::NativeRdfFormat;
use super::ser_model::{SerGraph, SerTerm, SerTermKind, SerTriple3};
use super::span::{NoSpans, SpanCollector};
use crate::nesting::{MAX_PARSE_NESTING_DEPTH, nesting_too_deep};
use crate::{RdfDiagnostic, RdfLocation};

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

fn err(detail: impl Into<String>) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-parse", detail.into())
}

/// Build a located parse diagnostic (1-based line/column).
fn err_at(detail: impl Into<String>, line: u32, column: u32) -> RdfDiagnostic {
    RdfDiagnostic::error("native-codec-parse", detail.into()).with_location(RdfLocation {
        line: Some(line),
        column: Some(column),
        ..RdfLocation::default()
    })
}

/// Surface a `purrdf-iri` failure as a located codec diagnostic.
///
/// The code comes from [`IriError::diagnostic_code`] — the single owner of those
/// strings — rather than being re-spelled here, so `iri-relative-no-base` and
/// `iri-not-absolute-by-grammar` cannot drift between this codec and any other. The
/// message is the error's own `Display`, which already names the offending reference
/// verbatim and tells the user to add `@base`/`BASE` or pass a base.
fn iri_err_at(error: &IriError, line: u32, column: u32) -> RdfDiagnostic {
    RdfDiagnostic::error(error.diagnostic_code(), error.to_string()).with_location(RdfLocation {
        line: Some(line),
        column: Some(column),
        ..RdfLocation::default()
    })
}

/// U+FEFF ZERO WIDTH NO-BREAK SPACE, the scalar a UTF-8 byte order mark spells
/// (`EF BB BF`). Named here so the one place that refuses it and the one place that
/// documents the refusal cannot spell it differently.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// `raw` with its leading run of `WS` removed.
///
/// > `WS ::= #x20 | #x9 | #xD | #xA`
///
/// — Turtle 1.2 §6.5 / SPARQL 1.2 §19.8, transcribed once in
/// [`purrdf_iri::terminals::is_ws`] and scanned through it here.
///
/// Deliberately NOT [`str::trim_start`], which is defined over
/// [`char::is_whitespace`] — the Unicode `White_Space` property, twenty-six scalars —
/// where the line grammar names four. The difference is not cosmetic: `str::trim_start`
/// eats U+00A0 NO-BREAK SPACE, U+000B, U+000C, U+2028 and the rest, so a line the
/// grammar has no reading for is silently re-shaped into one that parses.
///
/// The scan is byte-wise, which is EXACT over UTF-8 rather than an approximation of it:
/// every `WS` member is ASCII and no byte of a multi-byte UTF-8 sequence is below
/// `0x80`, so a raw-byte test can neither miss a member nor alias one — and the byte
/// index it stops at is therefore always a scalar boundary.
fn trim_ws_start(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    let start = bytes
        .iter()
        .position(|&byte| !is_ws(byte))
        .unwrap_or(bytes.len());
    &raw[start..]
}

/// `raw` with its leading AND trailing runs of `WS` removed — the line-grammar-exact
/// replacement for [`str::trim`].
///
/// > `WS ::= #x20 | #x9 | #xD | #xA`
///
/// — Turtle 1.2 §6.5 / SPARQL 1.2 §19.8. See [`trim_ws_start`] for why the four-member
/// production and the twenty-six-member Unicode property are not interchangeable and
/// why the byte-wise scan is exact.
fn trim_ws(raw: &str) -> &str {
    let trimmed = trim_ws_start(raw);
    let bytes = trimmed.as_bytes();
    let end = bytes
        .iter()
        .rposition(|&byte| !is_ws(byte))
        .map_or(0, |last| last + 1);
    &trimmed[..end]
}

/// 1-based column (counted in Unicode scalar values) of a byte offset that lies
/// within the TRIMMED content of `raw`. `trimmed_off` is a byte offset into
/// [`trim_ws(raw)`](trim_ws) (i.e. token spans from tokenizing the trimmed line); it is
/// rebased onto `raw` by adding the leading-`WS` width.
///
/// > `WS ::= #x20 | #x9 | #xD | #xA`
///
/// — Turtle 1.2 §6.5 / SPARQL 1.2 §19.8.
///
/// The leading run measured here MUST be the run [`parse_one_line`] actually removed,
/// scalar for scalar, because this function's whole job is to undo that removal. The two
/// therefore scan the SAME predicate through the SAME [`trim_ws_start`], and neither
/// spells a whitespace test of its own. Letting them drift is a pure diagnostic
/// regression and an invisible one: the parse still succeeds or fails exactly as it did,
/// and only the column moves — so every error on a line with leading whitespace would
/// quietly point at the wrong scalar with no test failing. Concretely, if this measured
/// [`str::trim_start`]'s wider run while the parser removed `WS`, then on
/// `"␣␣<NBSP><urn:ex:s> …"` the parser would fail AT the NO-BREAK SPACE while this
/// reported the column of the `<` after it.
fn column_in_raw(raw: &str, trimmed_off: usize) -> u32 {
    let lead = raw.len() - trim_ws_start(raw).len();
    let mut byte = (lead + trimmed_off).min(raw.len());
    while byte > 0 && !raw.is_char_boundary(byte) {
        byte -= 1;
    }
    u32::try_from(raw[..byte].chars().count() + 1).unwrap_or(u32::MAX)
}

/// A parsed RDF term node, mirroring the `from_nquads` `Node` so the
/// `build_gts` lowering is structurally identical.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    /// A RESOLVED, validated IRI.
    ///
    /// The payload is a [`purrdf_iri::Iri`] rather than a `String` on purpose: an
    /// unresolved relative reference cannot be spelled in this position, so no parse
    /// path can silently intern one. Every value here has come through
    /// [`BaseScope::resolve`] or [`BaseScope::resolve_absolute_only`].
    Iri(Iri),
    Bnode(String),
    Literal {
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    },
    Triple(Box<Self>, Box<Self>, Box<Self>),
}

/// Line-family execution mode: `Auto` routes N-Triples/N-Quads inputs at or above
/// [`PARALLEL_MIN_BYTES`] through the chunk-parallel phase-1 tokenizer;
/// `ForceSequential` pins the single-pass pipeline (the bench baseline and the
/// determinism-proof tests compare the two — the outputs are byte-identical).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LineParseMode {
    /// Pick parallel above the size threshold, sequential below it.
    Auto,
    /// Always take the single-threaded pipeline, whatever the input size.
    ForceSequential,
}

/// Parse RDF text of one of the four line/Turtle-family `format`s into the first-party
/// in-memory [`SerGraph`] that the downstream statement-layer fold consumes. Mirrors the
/// `from_*` structure exactly (see the module note) so the resulting IR is byte-identical
/// to the prior purrdf-gts path, with the UCHAR-in-IRI gap fixed.
///
/// The mode applies ONLY to N-Triples / N-Quads: those grammars are newline-delimited
/// with no cross-line state, so line-aligned chunks can be tokenized+parsed in
/// parallel and re-joined in document order. Turtle / TriG stay sequential BY DESIGN —
/// `@prefix` / `@base` directives rebind mid-document (a later line's meaning depends
/// on every earlier directive) and anonymous blank nodes / reifiers mint labels from a
/// document-ordered counter, so a chunk cannot be parsed without the full prefix and
/// counter state of everything before it.
/// `base` is `&mut` because Turtle and TriG can MOVE it: `@base` / `BASE` rebinds the
/// base for the rest of the document, so on return the scope holds the base actually in
/// force at the END of the document — which is what the parse leg reports to its caller.
/// N-Triples / N-Quads have no base directive and leave it untouched, so the answer there
/// is the caller's base without those grammars having to say anything.
pub(super) fn parse_to_gts_graph_mode<S: SpanCollector>(
    format: NativeRdfFormat,
    text: &str,
    base: &mut BaseScope,
    mode: LineParseMode,
    collector: &mut S,
) -> Result<SerGraph, RdfDiagnostic> {
    let statements = match format {
        // N-Triples / N-Quads admit no relative reference by grammar, so they never
        // consult `base` — they route every IRI through `resolve_absolute_only`. This
        // arm matches the `admits_relative_iri = false` column for those two rows.
        NativeRdfFormat::NTriples => parse_lines(text, false, mode, base, collector)?,
        NativeRdfFormat::NQuads => parse_lines(text, true, mode, base, collector)?,
        NativeRdfFormat::Turtle => document_statements(text, base, false, collector)?,
        NativeRdfFormat::TriG => document_statements(text, base, true, collector)?,
        NativeRdfFormat::RdfXml => {
            return Err(err("RDF/XML is not a line/Turtle-family format"));
        }
        NativeRdfFormat::TriX
        | NativeRdfFormat::HexTuples
        | NativeRdfFormat::JsonLd
        | NativeRdfFormat::YamlLd => {
            return Err(err(
                "TriX / HexTuples / JSON-LD / YAML-LD are not line/Turtle-family formats",
            ));
        }
    };
    build_gts_graph(&statements)
}

/// Run the Turtle/TriG document parser over `text`, leaving `base` holding the base in
/// force at the end of the document.
///
/// The write-back is the whole point: `@base` rebinding is document state, and a parser
/// that swallowed it would leave the caller unable to re-serialize under the base the
/// document itself declared without re-reading the text by hand. On an ERROR the scope is
/// deliberately left untouched — a document that failed to parse declared nothing.
fn document_statements<S: SpanCollector>(
    text: &str,
    base: &mut BaseScope,
    allow_named_graphs: bool,
    collector: &mut S,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let mut parser = DocParser::new(text, base.clone(), allow_named_graphs, collector);
    let statements = parser.parse()?;
    *base = parser.base;
    Ok(statements)
}

// ───────────────────────────────────────────────────────────────────────────────
// N-Triples / N-Quads (line-oriented; absolute IRIs only)
// ───────────────────────────────────────────────────────────────────────────────

/// One statement: subject, predicate, object, and (N-Quads) an optional graph name.
type Statement = Vec<Node>;

/// The lexical span-table key for a statement subject, or `None` for a subject with no
/// single lexical key (a quoted-triple subject). A named node keys by its BARE IRI
/// string (no angle brackets, so a SHACL focus node joins directly); a blank node keys
/// as `_:label`. See [`SpanTable`](super::span::SpanTable) for the convention.
fn subject_key(node: &Node) -> Option<String> {
    match node {
        Node::Iri(iri) => Some(iri.as_str().to_owned()),
        Node::Bnode(label) => Some(format!("_:{label}")),
        // A literal is never a legal subject (validation rejects it) and a quoted-triple
        // subject has no single lexical key — neither is recorded.
        Node::Literal { .. } | Node::Triple(..) => None,
    }
}

/// Inputs at or above this many bytes take the chunk-parallel phase-1 pipeline.
///
/// Rationale: below ~1 MiB the whole parse completes in single-digit milliseconds,
/// so rayon's fork/join dispatch plus the per-chunk `Vec` staging would cost a
/// larger fraction of the runtime than the parallelism recovers — and 1 MiB also
/// guarantees at least four [`PARALLEL_MIN_CHUNK_BYTES`] chunks, so the parallel
/// path never degenerates into "one chunk plus overhead". Small documents (the
/// overwhelmingly common conformance / fixture case) keep the sequential path with
/// ZERO added overhead.
const PARALLEL_MIN_BYTES: usize = 1 << 20;

/// Smallest line-aligned chunk phase 1 hands a rayon worker (256 KiB). Chunks are
/// sized `len / (threads * 4)` — enough splits for work-stealing to balance ragged
/// lines — clamped to [256 KiB, 4 MiB] so tiny chunks never drown in dispatch
/// overhead and huge ones never serialize the tail. Chunk geometry affects ONLY
/// scheduling, never output: phase 2 re-joins chunks in document order.
const PARALLEL_MIN_CHUNK_BYTES: usize = 256 << 10;

/// Largest phase-1 chunk (see [`PARALLEL_MIN_CHUNK_BYTES`]).
const PARALLEL_MAX_CHUNK_BYTES: usize = 4 << 20;

/// Parse N-Triples (`allow_graph == false`) / N-Quads (`allow_graph == true`),
/// dispatching on size (see [`PARALLEL_MIN_BYTES`]) unless `mode` pins sequential.
///
/// Both paths produce the IDENTICAL statement list: each line is parsed with no
/// cross-line state, and the parallel path re-joins its chunks in document order, so
/// the downstream [`build_gts_graph`] interner sees the same statements in the same
/// order and assigns the same term ids (interning stays the sequential serialization
/// point). The determinism-proof tests below assert this end to end.
fn parse_lines<S: SpanCollector>(
    text: &str,
    allow_graph: bool,
    mode: LineParseMode,
    base: &BaseScope,
    collector: &mut S,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    if mode == LineParseMode::Auto && text.len() >= PARALLEL_MIN_BYTES {
        // The parallel path is `NoSpans`-only (each chunk gets its own ZST collector);
        // span tracking forces sequential (see `parse_dataset_with`), so `S::ENABLED`
        // is always false here. The parallel branch stays non-generic in the collector.
        return parse_lines_parallel(text, allow_graph, base);
    }
    parse_lines_sequential(text, allow_graph, 1, base, collector)
}

/// One physical line, plus the width in BYTES of the `EOL` terminator that ended it
/// (`0` for a final line that ends at end-of-input instead).
///
/// The width is CARRIED rather than recomputed by the consumer because the sequential
/// path maintains a document-global byte offset for span recording. An offset that
/// stepped `#xD#xA` as one byte — or a lone `#xD` as two, which is what the code here
/// used to do — moves every recorded [`Position`] onto the wrong scalar, and nothing
/// fails: the parse succeeds exactly as before and only the reported location lies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhysicalLine<'a> {
    /// The line's text, WITHOUT its terminator.
    text: &'a str,
    /// The terminator's width in bytes: 2 for `#xD#xA`, 1 for a lone `#xD` or `#xA`,
    /// 0 at end of input.
    terminator: usize,
}

/// The byte width of the `EOL` terminator starting at `at`, where `bytes[at]` is already
/// known to be `#xD` or `#xA`: 2 for the `#xD#xA` PAIR, 1 for anything else.
///
/// The pair rule is spelled ONCE, here, and every line path reaches it: the splitter, the
/// chunker and the per-chunk line counter. Spelling it twice is how a CRLF document comes
/// to parse as a different document depending on which path read it — the splitter
/// yielding one line where the counter counted two shifts every subsequent diagnostic's
/// line number by one, silently.
fn eol_width(bytes: &[u8], at: usize) -> usize {
    if bytes[at] == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
        2
    } else {
        1
    }
}

/// Split `text` into physical lines at the line grammar's OWN terminator.
///
/// > `EOL ::= [#xD#xA]+`
///
/// — RDF 1.2 N-Triples §2.2 / N-Quads §2.3 (`ntriplesDoc ::= triple? (EOL triple?)* EOL?`),
/// the production this family's whole line structure rests on.
///
/// This is deliberately NOT [`str::lines`], and the difference is a SILENT DROP rather
/// than a misparse. `str::lines` splits at `#xA` and strips a `#xD` that immediately
/// precedes one; a LONE `#xD` is ordinary text to it. So `<s> <p> <o> .#xD<s2> <p2> <o2>
/// .#xD` arrived as ONE line, was tokenized as one, and — because [`parse_one_line`] took
/// the tokens up to the first `.` and threw the rest away (see its own note on leftover
/// tokens) — the second statement vanished with exit zero and no diagnostic.
///
/// Each cut is ONE terminator: `#xD#xA` is a single terminator and not two, so a CRLF
/// document yields no spurious empty line between consecutive statements. A LONGER run of
/// `[#xD#xA]` yields empty lines between the cuts, and `EOL`'s `+` is honoured one layer
/// up rather than here: an empty line opens no production, [`parse_one_line`] returns
/// `None` for it, and a run of terminators therefore separates two statements exactly as a
/// single terminator does. Collapsing the run inside the splitter would be observably
/// WORSE, because the 1-based line number every diagnostic carries counts PHYSICAL lines:
/// `"a .\n\n\nb ."` would report `b .`'s errors at line 2 of a file in which every editor
/// shows it at line 4.
///
/// Every terminator byte is ASCII and no byte of a multi-byte UTF-8 sequence is below
/// `0x80`, so the byte-wise scan can neither miss a terminator nor cut inside a scalar.
/// Nothing here reaches INSIDE a token: N-Triples/N-Quads exclude a RAW `#xD` from both
/// token bodies that could otherwise hold one —
/// `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'` names `#xD` in
/// its exclusion set, and ``IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`` excludes
/// all of `#x00-#x20` — so a carriage return between `<`…`>` or between two `"` is not
/// content this splitter is stealing; it is a document the grammar already has no reading
/// for (and which the shared lexer already refused, as
/// `a_carriage_return_inside_a_token_is_not_content_of_that_token` executes). The escaped
/// spellings — `ECHAR`'s `\r`, and the `UCHAR` form of the same scalar, which ARE how a
/// carriage return is written into a literal — carry no `#xD` BYTE in the source at all
/// and are untouched.
fn physical_lines(text: &str) -> PhysicalLines<'_> {
    PhysicalLines { rest: text }
}

/// The iterator [`physical_lines`] returns.
struct PhysicalLines<'a> {
    /// The not-yet-yielded remainder of the document.
    rest: &'a str,
}

impl<'a> Iterator for PhysicalLines<'a> {
    type Item = PhysicalLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let bytes = self.rest.as_bytes();
        let Some(offset) = memchr::memchr2(b'\r', b'\n', bytes) else {
            // A final line that ends at end-of-input: `EOL?` is optional, so this is a
            // line, and (like `str::lines`) a document that DOES end in a terminator
            // yields no extra empty line after it — `rest` is empty and the iterator ends.
            let text = self.rest;
            self.rest = "";
            return Some(PhysicalLine {
                text,
                terminator: 0,
            });
        };
        let terminator = eol_width(bytes, offset);
        let text = &self.rest[..offset];
        self.rest = &self.rest[offset + terminator..];
        Some(PhysicalLine { text, terminator })
    }
}

/// How many `EOL` terminators `text` holds — i.e. how many physical lines it ENDS.
///
/// This is the chunk-parallel path's line-number arithmetic, and it must agree with
/// [`physical_lines`] exactly: it counts through the same [`eol_width`], so a `#xD#xA`
/// pair counts ONCE on both sides and neither can drift into counting it twice.
fn count_line_terminators(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let mut count = 0u32;
    let mut at = 0usize;
    while let Some(offset) = memchr::memchr2(b'\r', b'\n', &bytes[at..]) {
        let start = at + offset;
        at = start + eol_width(bytes, start);
        count = count.saturating_add(1);
    }
    count
}

/// Split `text` into line-aligned chunks of roughly `target_bytes` each: every chunk
/// (except possibly the last) ends immediately after a COMPLETE `EOL` terminator, so
/// concatenating the chunks' [`physical_lines`] streams reproduces `physical_lines(text)`
/// exactly.
///
/// "Complete" is the load-bearing word and it is the third trap of the `EOL` production.
/// A boundary that fell BETWEEN `#xD` and `#xA` would hand one chunk a line ending in a
/// lone `#xD` and the next a leading lone `#xA`, i.e. two terminators where the document
/// has one — so a CRLF file would parse into a different number of lines depending on the
/// chunk size, which is the machine's memory geometry deciding what a document means. The
/// cut is therefore taken at `offset + eol_width(..)`, past the whole terminator, through
/// the same [`eol_width`] the splitter and the counter use.
///
/// Every terminator byte is ASCII, so every boundary is a valid UTF-8 char boundary.
fn split_line_chunks(text: &str, target_bytes: usize) -> Vec<&str> {
    let bytes = text.as_bytes();
    let target = target_bytes.max(1);
    let mut chunks = Vec::with_capacity(text.len() / target + 1);
    let mut start = 0;
    while start < text.len() {
        let mut end = start.saturating_add(target).min(text.len());
        if end < text.len() {
            end = match memchr::memchr2(b'\r', b'\n', &bytes[end..]) {
                Some(offset) => {
                    let at = end + offset;
                    at + eol_width(bytes, at)
                }
                None => text.len(),
            };
        }
        chunks.push(&text[start..end]);
        start = end;
    }
    chunks
}

/// Phase 1 + 2 of the chunk-parallel line parse.
///
/// Phase 1 (parallel): rayon maps [`parse_lines_sequential`] over line-aligned chunks
/// — each chunk's lines are tokenized/parsed independently (the grammar has no
/// cross-line state) into a per-chunk statement buffer.
///
/// Phase 2 (sequential, document order): chunk results are visited IN DOCUMENT ORDER.
/// The FIRST error in document order wins — chunk results are fully collected before
/// any is inspected, so a fast-failing late chunk can never race ahead of an earlier
/// chunk's diagnostic, and each per-line diagnostic is built from the line text alone,
/// so the message is byte-identical to the sequential path's. Successful chunks
/// concatenate in order into the exact statement list the sequential pass yields.
fn parse_lines_parallel(
    text: &str,
    allow_graph: bool,
    base: &BaseScope,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let threads = rayon::current_num_threads().max(1);
    let target = (text.len() / (threads * 4).max(1))
        .clamp(PARALLEL_MIN_CHUNK_BYTES, PARALLEL_MAX_CHUNK_BYTES);
    parse_lines_parallel_with_chunk_size(text, allow_graph, target, base)
}

/// [`parse_lines_parallel`] with an explicit chunk size (tests use a tiny size to
/// force many chunks over small fixtures; chunk geometry never changes the output).
fn parse_lines_parallel_with_chunk_size(
    text: &str,
    allow_graph: bool,
    target_bytes: usize,
    base: &BaseScope,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let chunks = split_line_chunks(text, target_bytes);
    // Each chunk is a contiguous line-aligned slice; chunk 0 begins at document
    // line 1 and chunk k begins at `1 + (total EOL terminators in chunks[0..k])`.
    // Precompute those 1-based base lines (a sequential prefix sum) so every per-chunk
    // diagnostic reports the SAME document-global line the sequential path would,
    // keeping the parallel path byte-identical (line numbers included). The count runs
    // through `count_line_terminators`, i.e. through the same `eol_width` the splitter
    // uses, so a `#xD#xA` pair advances the line number by ONE here exactly as it ends
    // one line there.
    let mut base_lines = Vec::with_capacity(chunks.len());
    let mut next_line = 1u32;
    for chunk in &chunks {
        base_lines.push(next_line);
        next_line = next_line.saturating_add(count_line_terminators(chunk));
    }
    // Phase 1: parallel per-chunk tokenize+parse (on wasm32 rayon runs this inline).
    let per_chunk: Vec<Result<Vec<Statement>, RdfDiagnostic>> = chunks
        .par_iter()
        .enumerate()
        .map(|(i, chunk)| {
            parse_lines_sequential(chunk, allow_graph, base_lines[i], base, &mut NoSpans)
        })
        .collect();
    // Phase 2: document order — first error wins, then in-order concatenation.
    let mut statements = Vec::with_capacity(
        per_chunk
            .iter()
            .map(|r| r.as_ref().map_or(0, Vec::len))
            .sum(),
    );
    for chunk_result in per_chunk {
        statements.extend(chunk_result?);
    }
    Ok(statements)
}

/// The single-threaded line pipeline (also phase 1's per-chunk worker).
///
/// Line-oriented like the purrdf-gts parser: blank lines and `#`-comment lines are
/// skipped, every other line is one statement of 3 (NT) or 3-or-4 (NQ) terms. The
/// `<<( s p o )>>` quoted-triple TERM is admitted in subject (NQ only) and object
/// position; IRIREFs are UCHAR-decoded (the test060 fix).
fn parse_lines_sequential<S: SpanCollector>(
    text: &str,
    allow_graph: bool,
    base_line: u32,
    base: &BaseScope,
    collector: &mut S,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let mut statements = Vec::new();
    let mut lineno = base_line;
    // Running document-global byte offset of the current line's first byte. Only
    // maintained when span tracking is on (`S::ENABLED`); span tracking forces the
    // sequential path (see `parse_dataset_with`), so `text` here is the whole
    // document and this offset is document-global. For `NoSpans` the compiler proves
    // `S::ENABLED == false` and deletes every touch of `line_offset`, leaving the hot
    // path byte-identical. The step is the line's own bytes plus the width of the `EOL`
    // terminator that ended it, which `physical_lines` measured while it was cutting —
    // so a lone `#xD` advances by one and a `#xD#xA` pair by two, and neither is guessed
    // at from the byte that happens to follow.
    let mut line_offset = 0usize;
    for line in physical_lines(text) {
        let raw = line.text;
        let Some(nodes) = parse_one_line(raw, allow_graph, lineno, base)? else {
            if S::ENABLED {
                line_offset += raw.len() + line.terminator;
            }
            lineno = lineno.saturating_add(1);
            continue;
        };
        // Record the subject's source position when tracking is on. `S::ENABLED` is a
        // const, so for `NoSpans` this whole block is dead code (no key is built).
        if S::ENABLED {
            if let Some(key) = subject_key(&nodes[0]) {
                collector.record(
                    &key,
                    Position {
                        line: lineno,
                        column: column_in_raw(raw, 0),
                        byte_offset: line_offset,
                    },
                );
            }
            line_offset += raw.len() + line.terminator;
        }
        statements.push(nodes);
        lineno = lineno.saturating_add(1);
    }
    Ok(statements)
}

/// Parse ONE physical line of N-Triples / N-Quads into its statement, or `None` when the
/// line is blank or a `#` comment (which carry no statement and are skipped).
///
/// This is the WHOLE of the grammar's per-line work, and it is deliberately the only
/// copy of it: [`parse_lines_sequential`] (and therefore the chunk-parallel phase-1
/// workers, which are that same function over line-aligned slices) and the streaming
/// [`LineStreamParser`] both call THIS function on the same line text with the same
/// document-global `lineno`. The three paths cannot drift in what a line means or in
/// what diagnostic a malformed one produces, because there is nothing to drift: they
/// differ only in where the line came from and where the statement goes.
///
/// `raw` is the UNTRIMMED line as [`physical_lines`] yields it (no `#xD`, `#xA` or
/// `#xD#xA` terminator); columns in diagnostics are rebased onto it.
///
/// # One line is at most one statement, and nothing may follow it
///
/// > `ntriplesDoc ::= triple? (EOL triple?)* EOL?`
///
/// > `triple ::= subject predicate object '.'`
///
/// — RDF 1.2 N-Triples §2.2 (N-Quads §2.3 differs only in admitting the graph label). A
/// line holds AT MOST ONE `triple`, and the only thing the production allows after that
/// `'.'` is the `EOL` that ends the line — or a comment, which is not a token and never
/// reaches here. So anything still in the token stream once the terminator has been
/// consumed belongs to no production, and is refused by [`expect_exhausted`](
/// TokenCursor::expect_exhausted) naming what was found and the column it sits at.
///
/// Before that check this function took the tokens up to the first `.`, consumed the `.`,
/// and returned — DISCARDING everything after it, with exit zero and no diagnostic. That
/// is the silent-drop half of this file's `EOL` defect and it needed no exotic input at
/// all: `<s> <p> <o> . <s2> <p2> <o2> .` is a line a user can type, and one of its two
/// statements never reached the graph.
///
/// # What separates the tokens, and what does not
///
/// The run stripped off each end is `WS` and only `WS`:
///
/// > `WS ::= #x20 | #x9 | #xD | #xA`
///
/// — Turtle 1.2 §6.5 / SPARQL 1.2 §19.8, four code points, scanned through the single
/// [`purrdf_iri::terminals::is_ws`] transcription. [`str::trim`] is NOT that: it is
/// defined over [`char::is_whitespace`], the Unicode `White_Space` property, and admits
/// twenty-six scalars including U+00A0 NO-BREAK SPACE, U+000B, U+000C, U+2028 and
/// U+3000.
///
/// Using the wider test here did not merely widen the accepted language, it CHANGED WHAT
/// TWO KINDS OF LINE MEAN — and both changes were silent, because the affected lines
/// still parsed:
///
/// * **A line holding nothing but U+00A0** trimmed to the empty string and was dropped
///   as BLANK. It is not blank: U+00A0 is not `WS`, so nothing about it is skippable, and
///   it is not a comment either. It is now what the grammar says it is — a line whose
///   first scalar opens no terminal — and it is refused, with the column pointing at the
///   NO-BREAK SPACE itself.
/// * **A NO-BREAK SPACE before `#`** trimmed away, leaving the line starting with `#`, so
///   the line was read as a COMMENT and everything on it was discarded. `#` opens a
///   comment only where a comment may begin, and a comment may begin only after `WS` or
///   at the line's start; a U+00A0 before it is neither. Such a line is now refused
///   rather than silently thrown away. (Turtle 1.2 / N-Triples 1.2, *Comments*:
///   "Comments in N-Triples take the form of `#`, outside an `IRIREF` or
///   `STRING_LITERAL_QUOTE`, and continue to the end of line (marked by characters #xD or
///   #xA) or end of file.")
///
/// None of this reaches INSIDE a token. A U+00A0 in a quoted literal or in an `IRIREF`
/// body is content, is lawful, and is untouched — `IRIREF` excludes only `#x00-#x20` and
/// nine reserved delimiters, and U+00A0 is `ucschar` (see
/// [`absolute_iri_by_grammar`]). This is a rule about the LINE grammar, not about string
/// content.
///
/// # U+FEFF: a lawful NAME character, refused in the ONE position no name may occupy
///
/// U+FEFF ZERO WIDTH NO-BREAK SPACE — the scalar a UTF-8 byte order mark spells — is
/// **not** whitespace of any kind, and it is **not** a character this grammar excludes.
/// It sits inside `PN_CHARS_BASE`:
///
/// > `PN_CHARS_BASE ::= … | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]`
///
/// `#xFEFF` falls in `[#xFDF0-#xFFFD]`, so it is a lawful name-start and name-continue
/// scalar, a lawful `ucschar` inside an `IRIREF` body, and lawful content inside a
/// literal. Every one of `_:a<FEFF>b`, `ex:a<FEFF>b`, `<urn:ex:a<FEFF>b>` and
/// `"x<FEFF>y"` parses, and MUST keep parsing: refusing a scalar the name production
/// explicitly admits would be over-refusal, which this repository treats as exactly as
/// severe as a silent drop. Nothing below touches any of them.
///
/// What IS refused is one position: the first non-`WS` scalar of a line. That refusal is
/// exact rather than cautious, and it is exact by exhaustion over the grammar's own
/// alternatives. A line of this family is a `triple`, a comment, or empty, with
///
/// > `WS ::= #x20 | #x9 | #xD | #xA`
///
/// > `EOL ::= [#xD#xA]+`
///
/// and a `triple` opens at
///
/// > `subject ::= IRIREF | BLANK_NODE_LABEL`
///
/// (RDF 1.2 adds the triple term `<<( … )>>`, which also opens at `<`). Spelled out,
/// the first scalar of a statement is `<` or `_`, a comment's is `#`, and there is no
/// fourth alternative — N-Triples and N-Quads have NO bare-word position at all: no
/// keywords, no prefixed names, no `a`. So a name-start scalar at the head of a line
/// belongs to no production, and no document is lost by saying so.
///
/// This is therefore not a new refusal, only a new *diagnostic*. Such a line was already
/// refused, and refused unhelpfully: U+FEFF being `PN_CHARS_BASE`, the scanner had
/// already dispatched it as a name before any arm could look at it, so the line came back
/// as "unexpected token `Word("\u{feff}")`" — a message naming nothing the user could act
/// on. The verdict is unchanged; only the explanation is.
///
/// The rejected alternative was to STRIP exactly one leading U+FEFF at document start. It
/// loses on the grammar: no production of this family names a byte order mark, so
/// stripping would be this parser inventing a production the specification does not have
/// — the same class of act as widening `WS` to the Unicode property, and the very class
/// of act the rest of this module exists to undo. It is also the weaker reading of what a
/// mark IS: a byte order mark is a claim about the ENCODING of a byte stream, and by the
/// time a line reaches here the bytes have been decoded as UTF-8 and the claim has been
/// honoured, so what remains is only a scalar the line grammar does not place. And it is
/// unspellable without inventing state: this function is the single copy of the line
/// grammar shared by all three line paths and it is handed a LINE, never a document
/// offset, while a per-line strip would accept a mark at the head of EVERY line, which no
/// reading of any specification supports.
///
/// The two neighbours of the decision, both refused, neither by this arm:
///
/// * A **second consecutive** U+FEFF is a leading U+FEFF in its own right and meets the
///   same refusal — which it also would under the strip-exactly-one alternative.
/// * A **mid-line** U+FEFF standing alone between two tokens is refused one layer down,
///   as the `Word` it lawfully lexes into, because no position of an N-Triples/N-Quads
///   statement admits a bare word. That has nothing to do with byte order marks and the
///   diagnostic does not pretend otherwise. A mid-line U+FEFF *adjacent to or inside* a
///   token is a different thing entirely — it is part of that name, IRI or literal, and
///   it parses.
fn parse_one_line(
    raw: &str,
    allow_graph: bool,
    lineno: u32,
    base: &BaseScope,
) -> Result<Option<Statement>, RdfDiagnostic> {
    let line = trim_ws(raw);
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    if line.starts_with(BYTE_ORDER_MARK) {
        return Err(err_at(
            "line begins with U+FEFF ZERO WIDTH NO-BREAK SPACE (the UTF-8 byte order \
             mark, bytes EF BB BF). It is a lawful PN_CHARS_BASE scalar INSIDE a token, \
             but no production of the N-Triples / N-Quads line grammar names a byte order \
             mark: a statement begins at an IRIREF `<` or a blank node label `_:`, a \
             comment at `#`, and WS is #x20 | #x9 | #xD | #xA. Remove the mark",
            lineno,
            column_in_raw(raw, 0),
        ));
    }
    let tokens = tokenize(line).map_err(|e| {
        let col = e.byte_offset().map_or(1, |at| column_in_raw(raw, at));
        err_at(e.to_string(), lineno, col)
    })?;
    let mut cursor = TokenCursor::new(tokens, raw, lineno, base);
    // A well-formed line holds three or four terms: reserve once, no regrowth.
    let mut nodes = Vec::with_capacity(4);
    while !cursor.at_statement_end() {
        nodes.push(cursor.term(0)?);
    }
    cursor.expect_dot()?;
    cursor.expect_exhausted()?;
    let valid_len = if allow_graph {
        nodes.len() == 3 || nodes.len() == 4
    } else {
        nodes.len() == 3
    };
    if !valid_len {
        return Err(err_at(
            format!(
                "expected {} terms, got {}",
                if allow_graph { "3 or 4" } else { "3" },
                nodes.len(),
            ),
            lineno,
            column_in_raw(raw, 0),
        ));
    }
    validate_statement(&nodes, lineno, column_in_raw(raw, 0))?;
    Ok(Some(nodes))
}

/// A cursor over one line's lexer tokens, parsing N-Triples/N-Quads terms.
///
/// The cursor OWNS its token buffer (discarded after the line is parsed), so
/// [`bump`](Self::bump) can MOVE each consumed token out instead of deep-cloning
/// its `String` payload.
struct TokenCursor<'a> {
    tokens: Vec<Spanned<'a>>,
    pos: usize,
    raw: &'a str,
    lineno: u32,
    /// The base in scope, carried for the DIAGNOSTIC only. This grammar admits no
    /// relative reference, so the base is never applied — but a refusal that cannot see
    /// it can only say "no base IRI is in scope", which is false whenever the caller
    /// supplied one.
    base: &'a BaseScope,
}

impl<'a> TokenCursor<'a> {
    fn new(tokens: Vec<Spanned<'a>>, raw: &'a str, lineno: u32, base: &'a BaseScope) -> Self {
        Self {
            tokens,
            pos: 0,
            raw,
            lineno,
            base,
        }
    }

    /// 1-based column of the current token (or, past the end, just after the last
    /// token), rebased onto the untrimmed source line.
    fn col(&self) -> u32 {
        let off = self
            .tokens
            .get(self.pos)
            .map(|s| s.start)
            .or_else(|| self.tokens.last().map(|s| s.end))
            .unwrap_or(0);
        column_in_raw(self.raw, off)
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    /// Consume the current token, MOVING it out of the owned buffer (a cheap
    /// `Token::Dot` placeholder is left behind; the cursor never re-reads a
    /// consumed position — `peek` looks only at `pos`, which has advanced).
    fn bump(&mut self) -> Option<Token<'a>> {
        let t = self
            .tokens
            .get_mut(self.pos)
            .map(|s| std::mem::replace(&mut s.token, Token::Dot));
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// True at the statement terminator `.` or the end of the token stream.
    fn at_statement_end(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Dot))
    }

    /// Refuse anything left in the token stream after the statement terminator.
    ///
    /// > `ntriplesDoc ::= triple? (EOL triple?)* EOL?`
    ///
    /// — RDF 1.2 N-Triples §2.2: a `triple` is followed by `EOL`, never by another
    /// `triple` on the same line. The tokenizer has already eaten any trailing comment
    /// (a `#` outside a token runs to end of line and produces no token) and all `WS`, so
    /// a leftover token here is real content the grammar cannot place.
    ///
    /// The diagnostic names the token and its column because the two ways to arrive here
    /// are both typos a user must SEE to fix: a second statement crammed onto the line,
    /// and a stray `.` that ended the statement early (`<s> <p> <o>. <g> .`). Returning
    /// `Ok` instead — which is what this function replaced — threw the remainder away.
    fn expect_exhausted(&self) -> Result<(), RdfDiagnostic> {
        match self.peek() {
            None => Ok(()),
            Some(token) => Err(err_at(
                format!(
                    "unexpected token {token:?} after the statement terminator '.'; a \
                     line of this grammar holds at most one statement (`ntriplesDoc ::= \
                     triple? (EOL triple?)* EOL?`), so a second statement must begin on \
                     its own line"
                ),
                self.lineno,
                self.col(),
            )),
        }
    }

    /// Consume the statement terminator, which is not optional.
    ///
    /// > `ntriplesDoc ::= triple? (EOL triple?)* EOL?`
    /// >
    /// > `triple ::= subject predicate object '.'`
    ///
    /// — RDF 1.2 N-Triples §2.2 (N-Quads §2.2 spells `quad ::= subject predicate object`
    /// `graphLabel? '.'`). The `'.'` is a literal inside `triple`, not an optional
    /// trailing element of `ntriplesDoc`: what `ntriplesDoc` makes optional is the
    /// `triple` itself and the final `EOL`, so a line that HAS a subject, predicate and
    /// object has a `'.'` too. There is no production in this family under which
    /// `<s> <p> <o>` alone is a statement.
    ///
    /// # This used to accept the terminator's ABSENCE
    ///
    /// The match arm read `Some(Token::Dot) | None => Ok(())`, so running off the end of
    /// the token stream was as good as finding the dot. Nothing was dropped and nothing
    /// was misparsed — the three terms still became the triple the author meant — which
    /// is exactly why it survived: the defect is pure over-acceptance, a document this
    /// grammar does not define being read as though it did, and it only ever shows up
    /// when the file moves to a conforming parser that refuses it.
    ///
    /// # What is NOT refused
    ///
    /// [`parse_one_line`] returns before a cursor is built for a line that is empty or
    /// comment-only after `WS` trimming, and `trim_ws` removes a trailing `#xD`, so none
    /// of a blank final line, a comment-only final line, a file with no final newline, or
    /// a file whose last line ends in a bare `\r` reaches this function at all. A quad's
    /// `graphLabel` is consumed by the term loop before it, so `<s> <p> <o> <g> .`
    /// arrives here at the dot like any triple.
    fn expect_dot(&mut self) -> Result<(), RdfDiagnostic> {
        let col = self.col();
        match self.bump() {
            Some(Token::Dot) => Ok(()),
            None => Err(err_at(
                "the statement terminator '.' is missing: `triple ::= subject predicate \
                 object '.'` (RDF 1.2 N-Triples §2.2) ends every statement with a '.', \
                 and the line ended without one"
                    .to_owned(),
                self.lineno,
                col,
            )),
            other => Err(err_at(
                format!("expected '.' terminator, found {other:?}"),
                self.lineno,
                col,
            )),
        }
    }

    /// Parse one term in N-Triples/N-Quads syntax. The lexer admits `<<( … )>>` in every
    /// position; positional validity is checked later by [`validate_statement`], exactly as
    /// the purrdf-gts parser does.
    ///
    /// `depth` is how many quoted-triple terms this one is already nested inside — 0 at a
    /// statement's top level — and is what [`MAX_PARSE_NESTING_DEPTH`] is checked against.
    fn term(&mut self, depth: usize) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::TripleOpen) => self.quoted_triple(depth),
            Some(Token::Iri(_)) => {
                let col = self.col();
                let Some(Token::Iri(value)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Iri(absolute_iri_by_grammar(
                    &value,
                    self.base,
                    self.lineno,
                    col,
                )?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label.to_owned()))
            }
            Some(Token::StringLit(_) | Token::LongStringLit(_)) => self.literal(),
            other => Err(err_at(
                format!("unexpected token {other:?}"),
                self.lineno,
                self.col(),
            )),
        }
    }

    /// `<<( s p o )>>` quoted-triple term (the only triple form N-Triples/N-Quads
    /// admit). The purrdf-gts N-Quads parser requires the parenthesized form.
    ///
    /// The ONE recursion in the line grammar, so this is where the nesting bound is
    /// decided. It is refused BEFORE the `<<(` is consumed and before the level's `Node` is
    /// built, so the diagnostic points at the offending token and the partially built term
    /// that is dropped on the way out is itself no deeper than the bound.
    fn quoted_triple(&mut self, depth: usize) -> Result<Node, RdfDiagnostic> {
        if depth >= MAX_PARSE_NESTING_DEPTH {
            return Err(nesting_too_deep(self.lineno, self.col()));
        }
        let depth = depth + 1;
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.term(depth)?;
        let p = self.term(depth)?;
        let o = self.term(depth)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// A string literal with an optional `@lang[--dir]` tag or `^^<datatype>`.
    fn literal(&mut self) -> Result<Node, RdfDiagnostic> {
        let Some(Token::StringLit(value) | Token::LongStringLit(value)) = self.bump() else {
            unreachable!()
        };
        let mut lang = None;
        let mut direction = None;
        let mut datatype = None;
        match self.peek() {
            Some(Token::LangTag(_)) => {
                let col = self.col();
                let Some(Token::LangTag(raw)) = self.bump() else {
                    unreachable!()
                };
                let (base, dir) = split_lang_direction(raw, self.lineno, col)?;
                validate_language_tag(&base, self.lineno, col)?;
                lang = Some(base);
                direction = dir;
            }
            Some(Token::HatHat) => {
                self.bump();
                let col = self.col();
                let Some(Token::Iri(iri)) = self.bump() else {
                    return Err(err_at("datatype must be an IRI", self.lineno, col));
                };
                absolute_iri_by_grammar(&iri, self.base, self.lineno, col)?;
                if matches!(iri.as_ref(), RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                    return Err(err_at(
                        "literal cannot explicitly use the RDF language-string datatype",
                        self.lineno,
                        col,
                    ));
                }
                datatype = Some(iri.into_owned());
            }
            _ => {}
        }
        Ok(Node::Literal {
            value: value.into_owned(),
            lang,
            direction,
            datatype,
        })
    }

    fn expect(&mut self, token: &Token<'a>) -> Result<(), RdfDiagnostic> {
        if self.peek() == Some(token) {
            self.pos += 1;
            Ok(())
        } else {
            Err(err_at(
                format!("expected {token:?}, found {:?}", self.peek()),
                self.lineno,
                self.col(),
            ))
        }
    }
}

/// Split an N-Quads language tag into `(language, direction)`: `ar--rtl` →
/// `("ar", Some("rtl"))`; a plain `en` → `("en", None)`. A `--ltr`/`--rtl` suffix is
/// the RDF 1.2 base-direction marker; any other `--`-suffix is rejected, mirroring
/// purrdf-gts.
fn split_lang_direction(
    raw: &str,
    line_no: u32,
    column: u32,
) -> Result<(String, Option<String>), RdfDiagnostic> {
    if let Some((base, dir)) = raw.rsplit_once("--") {
        if matches!(dir, "ltr" | "rtl") && !base.is_empty() {
            Ok((base.to_owned(), Some(dir.to_owned())))
        } else {
            Err(err_at("invalid literal base direction", line_no, column))
        }
    } else {
        Ok((raw.to_owned(), None))
    }
}

// ───────────────────────────────────────────────────────────────────────────────
// Term validation (positional + IRI/lang shape), mirroring the prior purrdf-gts parser
// ───────────────────────────────────────────────────────────────────────────────

/// Resolve an IRI for a grammar that admits NO relative reference, after
/// UCHAR-decoding.
///
/// The N-Triples/N-Quads IRIREF grammar is `'<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`:
/// a character is forbidden as a RAW byte but PERMITTED by the production when spelled
/// as a `UCHAR` escape. The lexer ([`tokenize`]) enforces the raw-byte half — its IRIREF
/// scan STOPS at a raw `#x00-#x20` (every C0 control and the SPACE) or one of the nine
/// reserved delimiters ``< > " { } | ^ ` \`` — and decodes every `\u`/`\U` escape, so by
/// the time a value reaches here every otherwise-forbidden character it carries came from
/// a UCHAR. That exclusion set is exactly the production's and no larger: a raw U+00A0 or
/// U+3000 is `ucschar`, is lawful in an `IRIREF` body, and passes through to this check.
///
/// A UCHAR lifts the LEXER's restriction, not the IRI's. RDF Concepts §3.2 requires the
/// term to be an RFC-3987 IRI, and an escape is a SPELLING of a code point rather than a
/// licence for one, so validation runs the RFC-3987 grammar over the DECODED value. That
/// splits the escapes in two, and the split is the whole point of doing it here:
///
/// * A `ucschar` code point is legal in an IRI and must SURVIVE, however it was spelled.
///   W3C RDFC-1.0 `test060` (`tests/fixtures/rdfc/test060-in.nq`) carries `<urn:ex:\u00a0>`
///   (NO-BREAK SPACE) and `<urn:ex:\u221e>` (INFINITY), and its canonical form keeps the
///   decoded character — so this check must NOT reject them. Note `\u00a0` is NOT U+0020;
///   reading it as one is what makes this pair look like a contradiction.
/// * A code point OUTSIDE that grammar stays illegal however it was spelled. `\u0020`
///   (SPACE) and `\u003c` (`<`) decode to characters the IRI grammar does not admit, and
///   are refused with `iri-disallowed-char` exactly as their raw forms are — the prior
///   scheme-only check let both through and interned an un-serializable term.
///
/// Both directions are pinned by `uchar_escapes_are_validated_against_the_decoded_iri`
/// in `crates/rdf/tests/relative_iri_base.rs`; neither may drift without reddening it.
///
/// This routes through [`BaseScope::resolve_absolute_only`], so a relative reference
/// reports `iri-not-absolute-by-grammar` — the code that says "supplying a base will
/// not help" — instead of the generic parse error the hand-rolled scheme check gave.
/// The base is never APPLIED, because none may be applied to these grammars.
///
/// It is nonetheless the caller's REAL scope, not a locally minted empty one. The two
/// resolve identically — `resolve_absolute_only` returns `Err` for a relative reference
/// whatever is in scope — but they DIAGNOSE differently, and the empty stand-in
/// diagnosed a lie: a user who ran `purrdf convert --from ntriples --base http://…/`
/// was told "no base IRI is in scope" about a base they had just supplied and which was
/// threaded all the way down to this frame. The message now names the base and says it
/// is deliberately not applied here, which is the fact the user needs.
fn absolute_iri_by_grammar(
    value: &str,
    base: &BaseScope,
    line_no: u32,
    column: u32,
) -> Result<Iri, RdfDiagnostic> {
    base.resolve_absolute_only(value)
        .map_err(|e| iri_err_at(&e, line_no, column))
}

/// Validate a BCP-47 language tag, including the long private-use subtag relaxation
/// (`x-purrdf-…`) purrdf-gts applies.
fn validate_language_tag(tag: &str, line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    let mut parts = tag.split('-');
    let Some(primary) = parts.next() else {
        return Err(err_at("empty language tag", line_no, column));
    };
    if primary.is_empty()
        || primary.len() > 8
        || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return Err(err_at(
            format!("invalid language tag {tag:?}"),
            line_no,
            column,
        ));
    }
    let mut private_use = primary.eq_ignore_ascii_case("x");
    for subtag in parts {
        let alnum = !subtag.is_empty() && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric());
        let acceptable = if private_use {
            alnum
        } else {
            alnum && subtag.len() <= 8
        };
        if !acceptable {
            return Err(err_at(
                format!("invalid language tag {tag:?}"),
                line_no,
                column,
            ));
        }
        if subtag.eq_ignore_ascii_case("x") {
            private_use = true;
        }
    }
    Ok(())
}

fn node_is(node: &Node, kinds: &[fn(&Node) -> bool]) -> bool {
    kinds.iter().any(|p| p(node))
}

fn is_iri(node: &Node) -> bool {
    matches!(node, Node::Iri(_))
}
fn is_bnode(node: &Node) -> bool {
    matches!(node, Node::Bnode(_))
}
fn is_literal(node: &Node) -> bool {
    matches!(node, Node::Literal { .. })
}

/// A subject position — asserted, or nested inside a triple term — is an IRI or a
/// blank node in the RDF 1.2 term model, with no per-syntax variation: N-Triples and
/// N-Quads read the same terms, one of them merely carries a fourth (graph) slot.
fn validate_subject(node: &Node, line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode]) {
        return Ok(());
    }
    Err(err_at("invalid subject term", line_no, column))
}

fn validate_predicate(node: &Node, line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    if is_iri(node) {
        Ok(())
    } else {
        Err(err_at("predicate must be IRI", line_no, column))
    }
}

/// An object position — asserted, or nested inside a triple term — admits every term
/// kind, and a triple term there carries the term model down into its own components.
/// This is the ONLY position RDF 1.2 nests a triple term in.
fn validate_object(node: &Node, line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode, is_literal]) {
        return Ok(());
    }
    if let Node::Triple(s, p, o) = node {
        return validate_triple(s, p, o, line_no, column);
    }
    Err(err_at("invalid object term", line_no, column))
}

fn validate_triple(
    s: &Node,
    p: &Node,
    o: &Node,
    line_no: u32,
    column: u32,
) -> Result<(), RdfDiagnostic> {
    validate_subject(s, line_no, column)?;
    validate_predicate(p, line_no, column)?;
    validate_object(o, line_no, column)
}

fn validate_statement(nodes: &[Node], line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    validate_subject(&nodes[0], line_no, column)?;
    validate_predicate(&nodes[1], line_no, column)?;
    validate_object(&nodes[2], line_no, column)?;
    if let Some(graph_name) = nodes.get(3)
        && !node_is(graph_name, &[is_iri, is_bnode])
    {
        return Err(err_at("invalid graph name term", line_no, column));
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────────
// Turtle / TriG (prefixes, base, collections, BNPL, quoted/reifying triples)
// ───────────────────────────────────────────────────────────────────────────────

/// A recursive-descent Turtle/TriG parser over the sparql-algebra token stream. It
/// emits the SAME flat statement list (subject/predicate/object[/graph] `Node`s) the
/// purrdf-gts Turtle/TriG parser produced before lowering through `from_nquads`'s
/// `build_gts`, so the resulting [`SerGraph`] is byte-identical.
struct DocParser<'a, 'c, S: SpanCollector> {
    tokens: Vec<Spanned<'a>>,
    pos: usize,
    /// Declared prefix → its namespace, ALREADY RESOLVED against the base that was in
    /// force when the `@prefix` was read (Turtle §4.4). Resolving at declaration time
    /// rather than at use time is what makes `p:x` denote one IRI for the whole
    /// document even if a later `@base` rebinds.
    prefixes: HashMap<String, String>,
    /// The stack of base IRIs in scope. Empty means no base at all, which is a
    /// first-class state: it is not an error until a relative reference needs one.
    base: BaseScope,
    /// The vocabulary IRIs the Turtle grammar itself mints (`a`, collection cells,
    /// the reification predicate). Parsed ONCE here rather than at each use, so the
    /// per-collection-item and per-annotation emit paths pay a clone, exactly as they
    /// paid a `to_owned()` before.
    rdf_type: Iri,
    rdf_first: Iri,
    rdf_rest: Iri,
    rdf_nil: Iri,
    rdf_reifies: Iri,
    bnode_counter: usize,
    allow_named_graphs: bool,
    statements: Vec<Statement>,
    src: &'a str,
    /// Opt-in subject-position sink. For `NoSpans` this is a ZST and every use is
    /// dead code under monomorphization.
    collector: &'c mut S,
    /// Document byte offset of the current top-level statement subject's first token,
    /// captured when the subject term is parsed and resolved at emit time. Only read
    /// when `S::ENABLED`.
    subject_off: usize,
    /// Newline table over `src`, built lazily on the FIRST position lookup (a recorded
    /// subject under `S::ENABLED`, or any `loc()` diagnostic) and reused for the rest of
    /// the parse. A `OnceCell` so the `&self` `loc()` path can memoize it too — otherwise
    /// a document with many diagnostics-adjacent lookups rebuilds the whole-buffer scan
    /// per call, which is quadratic in the source length.
    line_index: std::cell::OnceCell<purrdf_iri::LineIndex>,
}

impl<'a, 'c, S: SpanCollector> DocParser<'a, 'c, S> {
    fn new(text: &'a str, base: BaseScope, allow_named_graphs: bool, collector: &'c mut S) -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert("rdf".to_owned(), RDF_NS.to_owned());
        // These are compile-time constants known to be well-formed absolute IRIs, so a
        // parse failure here is a programming error in this file, not bad input.
        let vocab =
            |iri: &str| purrdf_iri::parse(iri).expect("grammar vocabulary IRI is well-formed");
        Self {
            tokens: Vec::new(),
            pos: 0,
            prefixes,
            base,
            rdf_type: vocab(RDF_TYPE),
            rdf_first: vocab(RDF_FIRST),
            rdf_rest: vocab(RDF_REST),
            rdf_nil: vocab(RDF_NIL),
            rdf_reifies: vocab(RDF_REIFIES),
            bnode_counter: 0,
            allow_named_graphs,
            statements: Vec::new(),
            src: text,
            collector,
            subject_off: 0,
            line_index: std::cell::OnceCell::new(),
        }
    }

    /// Document byte offset of the current token (or the end of the last token past
    /// end-of-stream). Only called on the span-tracking path.
    fn cur_off(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|s| s.start)
            .or_else(|| self.tokens.last().map(|s| s.end))
            .unwrap_or(0)
    }

    /// Takes `&mut self` rather than `self` so the caller keeps the parser afterwards and
    /// can read the [`base`](Self::base) the document's `@base` directives left in force.
    ///
    /// # A leading U+FEFF is refused BY NAME — the same verdict, a better diagnostic
    ///
    /// U+FEFF ZERO WIDTH NO-BREAK SPACE is a lawful `PN_CHARS_BASE` scalar (it sits in
    /// `[#xFDF0-#xFFFD]`), so `ex:a<FEFF>b`, `_:a<FEFF>b`, `<urn:ex:a<FEFF>b>` and
    /// `"x<FEFF>y"` all parse and MUST keep parsing. Being a name character is exactly
    /// why the mark used to produce an unusable refusal at the head of a document: the
    /// scanner dispatched it as a name before any arm could look at it, and the document
    /// came back as `unexpected token Some(Word("\u{feff}"))` — a message naming a token
    /// the author never typed.
    ///
    /// This arm changes no verdict. Such a document was already refused, because U+FEFF
    /// is not one of the bare words the Turtle/TriG grammar has (`PREFIX`, `BASE`,
    /// `VERSION`, `GRAPH`, `a`, `true`, `false`) and no other production opens there. It
    /// is refused the same way here, with the mark named and the remedy stated — the same
    /// treatment [`parse_one_line`] gives the N-Triples/N-Quads line paths, so all five
    /// text codecs now answer a byte order mark alike.
    ///
    /// **Stripping the mark was rejected**, for the reason spelled out at
    /// [`parse_one_line`]: no production of this family names a byte order mark, so
    /// stripping would be this parser inventing a production the specification does not
    /// have — and a mark is a claim about the ENCODING of a byte stream, already honoured
    /// by the time these scalars were decoded.
    ///
    /// The position is byte 0 of the document and nothing else. A byte order mark is only
    /// a byte order mark at the start of the stream; a U+FEFF anywhere else is either part
    /// of the name, IRI or literal it sits in (lawful, untouched) or a stray `Word`
    /// refused one layer down as the token it lawfully lexes into, which is not a byte
    /// order mark and is not called one.
    fn parse(&mut self) -> Result<Vec<Statement>, RdfDiagnostic> {
        if self.src.starts_with(BYTE_ORDER_MARK) {
            return Err(err_at(
                "document begins with U+FEFF ZERO WIDTH NO-BREAK SPACE (the UTF-8 byte \
                 order mark, bytes EF BB BF). It is a lawful PN_CHARS_BASE scalar INSIDE \
                 a token, but no production of the Turtle/TriG grammar names a byte order \
                 mark: a statement begins at a directive (`@prefix`, `@base`, `PREFIX`, \
                 `BASE`), at a subject term (`<`, a prefixed name, `_:`, `[`, `(`) or — in \
                 TriG — at `GRAPH` or `{`, and WS is #x20 | #x9 | #xD | #xA. Remove the \
                 mark",
                1,
                1,
            ));
        }
        // Turtle/TriG admit a bare `/` in a prefixed-name local part (e.g.
        // `purrdf:report/shacl/sarif`), matching oxigraph/purrdf-gts leniency.
        // Turtle has no `/` operator, so this is unambiguous in term position;
        // the SPARQL `tokenize` keeps `/` as the property-path operator.
        self.tokens = tokenize_turtle(self.src).map_err(|e| {
            let off = e.byte_offset().unwrap_or(0);
            let p = purrdf_iri::LineIndex::new(self.src).locate(self.src, off);
            err_at(e.to_string(), p.line, p.column)
        })?;
        while self.peek().is_some() {
            if self.try_directive()? {
                continue;
            }
            if self.eat_kw("GRAPH") {
                if !self.allow_named_graphs {
                    let (l, c) = self.loc();
                    return Err(err_at("Turtle input cannot contain GRAPH blocks", l, c));
                }
                let graph = self.term(None, 0)?;
                self.expect(&Token::LBrace)?;
                self.graph_block(Some(&graph))?;
                continue;
            }
            // TriG rule [3] admits a bare `wrappedGraph` — `{ … }` with no label — as a
            // block naming the default graph. It is not a term, so it must be taken
            // before the subject position below.
            if self.at(&Token::LBrace) {
                if !self.allow_named_graphs {
                    let (l, c) = self.loc();
                    return Err(err_at("Turtle input cannot contain graph blocks", l, c));
                }
                self.pos += 1;
                self.graph_block(None)?;
                continue;
            }
            if S::ENABLED {
                self.subject_off = self.cur_off();
            }
            let first = self.term(None, 0)?;
            if self.eat(&Token::LBrace) {
                if !self.allow_named_graphs {
                    let (l, c) = self.loc();
                    return Err(err_at("Turtle input cannot contain graph blocks", l, c));
                }
                self.graph_block(Some(&first))?;
            } else {
                self.statement_after_subject(&first, None)?;
            }
        }
        Ok(std::mem::take(&mut self.statements))
    }

    /// Consume a `@prefix`/`@base`/`@version` or `PREFIX`/`BASE`/`VERSION` directive
    /// when present. Returns whether one was consumed.
    fn try_directive(&mut self) -> Result<bool, RdfDiagnostic> {
        // `@prefix` / `@base` / `@version` lex as a `LangTag` (the `@` form).
        if let Some(Token::LangTag(tag)) = self.peek() {
            match *tag {
                "prefix" => {
                    self.pos += 1;
                    self.prefix_directive(true)?;
                    return Ok(true);
                }
                "base" => {
                    self.pos += 1;
                    self.base_directive(true)?;
                    return Ok(true);
                }
                "version" => {
                    self.pos += 1;
                    self.version_string()?;
                    self.expect(&Token::Dot)?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        if self.eat_kw("PREFIX") {
            self.prefix_directive(false)?;
            return Ok(true);
        }
        if self.eat_kw("BASE") {
            self.base_directive(false)?;
            return Ok(true);
        }
        if self.eat_kw("VERSION") {
            self.version_string()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// `@prefix p: <ns> .` / `PREFIX p: <ns>`.
    ///
    /// Turtle §4.4 resolves the namespace against the base in force **when the
    /// directive is read**, and the binding is that resolved IRI from then on. Storing
    /// the raw text and resolving at use time instead — which this parser used to do —
    /// makes `p:x` denote two different IRIs on either side of a later `@base`, so the
    /// resolution happens here, once.
    fn prefix_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let (prefix, _) = self.expect_prefix_ns()?;
        let (l, c) = self.loc();
        let raw = self.expect_iri_raw()?;
        let namespace = self
            .base
            .resolve(&raw)
            .map_err(|e| iri_err_at(&e, l, c))?
            .as_str()
            .to_owned();
        self.prefixes.insert(prefix, namespace);
        if require_dot {
            self.expect(&Token::Dot)?;
        } else {
            self.eat(&Token::Dot);
        }
        Ok(())
    }

    /// `@base <iri> .` / `BASE <iri>`.
    ///
    /// The directive itself MAY be relative, in which case Turtle §6.1 (chaining to
    /// RFC-3986 §5.1.1) resolves it against the base already in force, so a chain of
    /// directives composes. It is an error only when no base is in scope AND the
    /// directive is relative — there is then nothing to resolve against.
    fn base_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let (l, c) = self.loc();
        let raw = self.expect_iri_raw()?;
        self.base
            .rebind(&raw, BaseOrigin::Directive { line: l, column: c })
            .map_err(|e| iri_err_at(&e, l, c))?;
        if require_dot {
            self.expect(&Token::Dot)?;
        } else {
            self.eat(&Token::Dot);
        }
        Ok(())
    }

    /// A `VERSION`/`@version` argument: a **single-line** string literal, recorded only
    /// to be accepted and skipped. RDF 1.2 forbids a triple-quoted (`'''`/`"""`) long
    /// string here, so the raw span is checked and a long form is rejected (the lexer
    /// collapses both quote styles into one `StringLit`, so the source span is the only
    /// place the distinction survives).
    fn version_string(&mut self) -> Result<(), RdfDiagnostic> {
        let span = self.tokens.get(self.pos).map(|s| (s.start, s.end));
        let (l, c) = self.loc();
        match self.bump() {
            Some(Token::StringLit(_)) => {
                if let Some((start, _)) = span {
                    let raw = &self.src[start..];
                    if raw.starts_with("\"\"\"") || raw.starts_with("'''") {
                        return Err(err_at(
                            "version directive needs a single-line string, found a triple-quoted string",
                            l,
                            c,
                        ));
                    }
                }
                Ok(())
            }
            other => Err(err_at(
                format!("version directive needs a string, found {other:?}"),
                l,
                c,
            )),
        }
    }

    /// A bare `prefix:` namespace (PNAME_NS); the local part must be empty.
    fn expect_prefix_ns(&mut self) -> Result<(String, String), RdfDiagnostic> {
        let (line, col) = self.loc();
        match self.bump() {
            Some(Token::PrefixedName(p, l)) if l.is_empty() => Ok((p.to_owned(), l.into_owned())),
            other => Err(err_at(
                format!("expected a prefix namespace, found {other:?}"),
                line,
                col,
            )),
        }
    }

    /// An IRIREF, returned UNRESOLVED (for `@prefix`/`@base` targets). The lexer has
    /// already UCHAR-decoded it.
    fn expect_iri_raw(&mut self) -> Result<String, RdfDiagnostic> {
        let (l, c) = self.loc();
        match self.bump() {
            Some(Token::Iri(s)) => Ok(s.into_owned()),
            other => Err(err_at(format!("expected an IRIREF, found {other:?}"), l, c)),
        }
    }

    /// Enter one level of syntactic nesting, or refuse the document.
    ///
    /// Turtle/TriG nest through many constructs — quoted and reifying triples, blank-node
    /// property lists, collections, annotation blocks — and each one is a recursive-descent
    /// frame, so an input's nesting depth is an instruction about how much stack to burn.
    /// [`MAX_PARSE_NESTING_DEPTH`] is the ceiling, and this is the single place it is
    /// enforced: EVERY cycle in this parser's call graph passes through
    /// [`term`](Self::term) or [`predicate_object_list`](Self::predicate_object_list), and
    /// both of them descend here on entry, so no nesting construct can be reached without
    /// paying a level. The location is the token that would have opened the level too many.
    fn descend(&self, depth: usize) -> Result<usize, RdfDiagnostic> {
        if depth >= MAX_PARSE_NESTING_DEPTH {
            let (line, column) = self.loc();
            return Err(nesting_too_deep(line, column));
        }
        Ok(depth + 1)
    }

    fn term(&mut self, graph: Option<&Node>, depth: usize) -> Result<Node, RdfDiagnostic> {
        let depth = self.descend(depth)?;
        match self.peek() {
            Some(Token::TripleOpen) => {
                // Distinguish the value form `<<( s p o )>>` from the reifying form
                // `<< s p o [~r] >>` by the immediately-following `(`.
                if self.peek2() == Some(&Token::LParen) {
                    self.parenthesized_quoted_triple(graph, depth)
                } else {
                    self.reifying_triple(graph, depth)
                }
            }
            Some(Token::Iri(_)) => {
                let (l, c) = self.loc();
                let Some(Token::Iri(raw)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Iri(self.resolve_iri(&raw, l, c)?))
            }
            Some(Token::PrefixedName(_, _)) => {
                let (l, c) = self.loc();
                let Some(Token::PrefixedName(prefix, local)) = self.bump() else {
                    unreachable!()
                };
                self.resolve_prefixed(prefix, &local, l, c)
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label.to_owned()))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(self.next_bnode())
            }
            Some(Token::LBracket) => self.blank_node_property_list(graph, depth),
            Some(Token::LParen) => self.collection(graph, depth),
            Some(Token::StringLit(_) | Token::LongStringLit(_)) => self.literal(),
            Some(Token::Integer(_) | Token::Decimal(_) | Token::Double(_)) => {
                self.numeric_literal("")
            }
            // A signed numeric literal `+N` / `-N`: the lexer emits the sign as a
            // separate `Plus`/`Minus` token, so consume it and fold it back into the
            // lexical form (kept verbatim, e.g. `-200.0`), matching purrdf-gts.
            Some(Token::Plus | Token::Minus)
                if matches!(
                    self.peek2(),
                    Some(Token::Integer(_) | Token::Decimal(_) | Token::Double(_))
                ) =>
            {
                let sign = if self.eat(&Token::Minus) {
                    "-"
                } else {
                    self.expect(&Token::Plus)?;
                    "+"
                };
                self.numeric_literal(sign)
            }
            Some(Token::Word(w)) if *w == "true" || *w == "false" => {
                let Some(Token::Word(value)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Literal {
                    value: value.to_owned(),
                    lang: None,
                    direction: None,
                    datatype: Some(XSD_BOOLEAN.to_owned()),
                })
            }
            _ => {
                let (l, c) = self.loc();
                Err(err_at(
                    format!("unexpected token {:?} in Turtle/TriG term", self.peek()),
                    l,
                    c,
                ))
            }
        }
    }

    /// A subject/object inside a triple term. Non-empty `[ … ]` / `( … )` would emit
    /// extra triples that cannot live inside a triple term, so they are rejected
    /// (W3C-conformant); an empty `[]` / `()` is a plain term and is allowed.
    fn quoted_component(
        &mut self,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::LBracket) => {
                let (l, c) = self.loc();
                Err(err_at(
                    "blank-node property list is not allowed inside a quoted triple",
                    l,
                    c,
                ))
            }
            Some(Token::LParen) => {
                if self.peek2() == Some(&Token::RParen) {
                    self.term(graph, depth)
                } else {
                    let (l, c) = self.loc();
                    Err(err_at(
                        "RDF collection is not allowed inside a quoted triple",
                        l,
                        c,
                    ))
                }
            }
            _ => self.term(graph, depth),
        }
    }

    fn predicate(&mut self, depth: usize) -> Result<Node, RdfDiagnostic> {
        if matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            return Ok(Node::Iri(self.rdf_type.clone()));
        }
        self.term(None, depth)
    }

    fn parenthesized_quoted_triple(
        &mut self,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.quoted_component(graph, depth)?;
        let p = self.predicate(depth)?;
        let o = self.quoted_component(graph, depth)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// A triple TERM in `rdf:reifies` object position: `<<( s p o )>>` (canonical) or
    /// the legacy non-parenthesized `<< s p o >>` (purrdf pre-0.9.11 triple-term
    /// serialization). Always a [`Node::Triple`] — never a minted reifier — because the
    /// object of `rdf:reifies` denotes the reified triple itself.
    fn reifies_object_triple_term(
        &mut self,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let parenthesized = self.eat(&Token::LParen);
        let s = self.quoted_component(graph, depth)?;
        let p = self.predicate(depth)?;
        let o = self.quoted_component(graph, depth)?;
        if parenthesized {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// RDF 1.2 reifying triple `<< s p o ~r? >>` in subject/object position: emits
    /// `r rdf:reifies <<( s p o )>>` and returns the reifier `r`. With an explicit
    /// `~ id`, `r` is that id; otherwise (`~` alone, or no reifier at all) a fresh
    /// blank node is minted. The inner triple is NOT independently asserted here — the
    /// reifiedTriple denotes its reifier, so only the `rdf:reifies` statement is emitted.
    fn reifying_triple(
        &mut self,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let s = self.quoted_component(graph, depth)?;
        let p = self.predicate(depth)?;
        let o = self.quoted_component(graph, depth)?;
        let reifier = if self.eat(&Token::Tilde) {
            if self.at_reifier_id() {
                self.term(graph, depth)?
            } else {
                self.next_bnode()
            }
        } else {
            self.next_bnode()
        };
        self.expect(&Token::TripleClose)?;
        self.emit_reifies(&reifier, &s, &p, &o, graph);
        Ok(reifier)
    }

    /// `blankNodePropertyList ::= '[' predicateObjectList ']'` (Turtle 1.2 §6.5),
    /// or the anonymous blank node `ANON ::= '[' WS* ']'`.
    ///
    /// The two are distinguished lexically: `[]`, `[ ]` and `[` tab/CR/LF `]` all
    /// lex to a single [`Token::Anon`], so an `LBracket` here always opens a
    /// property list, whose `predicateObjectList` is **not** optional.
    ///
    /// An `LBracket` immediately followed by an `RBracket` is therefore not a
    /// production at all. It has exactly one source: a comment between the
    /// brackets, `[ #` … newline … `]`. The tokenizer's `ANON` scan cannot refuse
    /// that, because it is comment-blind by construction — `[ #` … newline …
    /// `:p :o ]` is a lawful *populated* list and nothing lexical separates the
    /// two cases — so the refusal belongs here, where the following tokens are
    /// known.
    fn blank_node_property_list(
        &mut self,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<Node, RdfDiagnostic> {
        // `[]` lexes as a single `Anon`; `[ … ]` opens with `LBracket`.
        if self.eat(&Token::Anon) {
            return Ok(self.next_bnode());
        }
        self.expect(&Token::LBracket)?;
        if self.at(&Token::RBracket) {
            let (line, column) = self.loc();
            return Err(err_at(
                "`[` `]` with nothing between them is not a blank-node property \
                 list (which requires at least one predicate-object pair) and not \
                 an anonymous blank node (`ANON` admits only whitespace between \
                 its brackets, and a comment is not whitespace)",
                line,
                column,
            ));
        }
        let subject = self.next_bnode();
        self.predicate_object_list(&subject, graph, depth)?;
        self.expect(&Token::RBracket)?;
        Ok(subject)
    }

    fn collection(&mut self, graph: Option<&Node>, depth: usize) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::LParen)?;
        let mut items = Vec::new();
        while !self.eat(&Token::RParen) {
            if self.peek().is_none() {
                let (l, c) = self.loc();
                return Err(err_at("unterminated RDF collection", l, c));
            }
            items.push(self.term(graph, depth)?);
        }
        if items.is_empty() {
            return Ok(Node::Iri(self.rdf_nil.clone()));
        }
        let cells: Vec<Node> = (0..items.len()).map(|_| self.next_bnode()).collect();
        // The three vocabulary nodes are built once per collection, not once per item,
        // and the cells are borrowed from the local `cells` rather than cloned — `emit`
        // takes references and copies what it keeps.
        let rdf_first = Node::Iri(self.rdf_first.clone());
        let rdf_rest = Node::Iri(self.rdf_rest.clone());
        let rdf_nil = Node::Iri(self.rdf_nil.clone());
        for (index, item) in items.into_iter().enumerate() {
            let current = &cells[index];
            let rest = if index + 1 == cells.len() {
                &rdf_nil
            } else {
                &cells[index + 1]
            };
            self.emit(current, &rdf_first, &item, graph);
            self.emit(current, &rdf_rest, rest, graph);
        }
        Ok(cells.into_iter().next().expect("non-empty collection"))
    }

    fn literal(&mut self) -> Result<Node, RdfDiagnostic> {
        let Some(Token::StringLit(value) | Token::LongStringLit(value)) = self.bump() else {
            unreachable!()
        };
        let mut lang = None;
        let mut direction = None;
        let mut datatype = None;
        match self.peek() {
            Some(Token::LangTag(_)) => {
                let (l, c) = self.loc();
                let Some(Token::LangTag(raw)) = self.bump() else {
                    unreachable!()
                };
                // purrdf-gts's Turtle parser keeps the raw `@lang` text (including any
                // `--dir`) on the literal `lang` field and lowers it to an N-Quads
                // `@lang` token, so the direction is re-parsed at the `from_nquads`
                // stage. To match that exactly, split here into lang + direction.
                let (base, dir) = split_lang_direction(raw, l, c)?;
                lang = Some(base);
                direction = dir;
            }
            Some(Token::HatHat) => {
                self.bump();
                datatype = Some(self.datatype_iri()?);
            }
            _ => {}
        }
        Ok(Node::Literal {
            value: value.into_owned(),
            lang,
            direction,
            datatype,
        })
    }

    fn datatype_iri(&mut self) -> Result<String, RdfDiagnostic> {
        let (l, c) = self.loc();
        match self.bump() {
            Some(Token::Iri(raw)) => Ok(self.resolve_iri(&raw, l, c)?.as_str().to_owned()),
            Some(Token::PrefixedName(prefix, local)) => {
                match self.resolve_prefixed(prefix, &local, l, c)? {
                    Node::Iri(iri) => Ok(iri.as_str().to_owned()),
                    _ => unreachable!("resolve_prefixed yields an IRI node"),
                }
            }
            other => Err(err_at(
                format!("expected a datatype IRI, found {other:?}"),
                l,
                c,
            )),
        }
    }

    fn numeric_literal(&mut self, sign: &str) -> Result<Node, RdfDiagnostic> {
        let (l, c) = self.loc();
        match self.bump() {
            Some(Token::Integer(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_INTEGER)),
            Some(Token::Decimal(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_DECIMAL)),
            Some(Token::Double(lexical)) => Ok(numeric(format!("{sign}{lexical}"), XSD_DOUBLE)),
            other => Err(err_at(
                format!("expected a numeric literal, found {other:?}"),
                l,
                c,
            )),
        }
    }

    /// A TriG `wrappedGraph` body, the opening `{` already consumed. `graph` is the
    /// block's label, or `None` for the **unlabelled** `{ … }` block, which TriG rule
    /// \[3\] `block ::= triplesOrGraph | wrappedGraph | triples2 | 'GRAPH' …` admits and
    /// which names the *default* graph.
    fn graph_block(&mut self, graph: Option<&Node>) -> Result<(), RdfDiagnostic> {
        if let Some(name) = graph
            && !matches!(name, Node::Iri(_) | Node::Bnode(_))
        {
            let (l, c) = self.loc();
            return Err(err_at(
                "graph block name must be an IRI or blank node",
                l,
                c,
            ));
        }
        while !self.eat(&Token::RBrace) {
            if self.peek().is_none() {
                let (l, c) = self.loc();
                return Err(err_at("unterminated graph block", l, c));
            }
            if S::ENABLED {
                self.subject_off = self.cur_off();
            }
            let subject = self.term(graph, 0)?;
            self.statement_after_subject_in_graph(&subject, graph)?;
        }
        Ok(())
    }

    fn statement_after_subject(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        // A self-asserting subject (reifying triple or blank-node property list) may
        // end immediately at `.`; a plain subject still needs a predicate-object list.
        if !self.at(&Token::Dot) {
            self.predicate_object_list(subject, graph, 0)?;
        }
        self.expect(&Token::Dot)
    }

    fn statement_after_subject_in_graph(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        if !(self.at(&Token::Dot) || self.at(&Token::RBrace)) {
            self.predicate_object_list(subject, graph, 0)?;
        }
        // The trailing `.` is optional for the final statement before `}`.
        if self.eat(&Token::Dot) || self.at(&Token::RBrace) {
            Ok(())
        } else {
            let (l, c) = self.loc();
            Err(err_at(
                "expected '.' to terminate statement in graph block",
                l,
                c,
            ))
        }
    }

    /// `depth` is the syntactic nesting this list is already inside. It descends a level of
    /// its own — the `{| … |}` annotation cycle
    /// (`predicate_object_list` → [`maybe_reify_and_annotate`](Self::maybe_reify_and_annotate)
    /// → `predicate_object_list`) is the one recursion in this parser that need not pass
    /// through a nested [`term`](Self::term), so counting only terms would leave it
    /// unbounded.
    fn predicate_object_list(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        let depth = self.descend(depth)?;
        loop {
            let predicate = self.predicate(depth)?;
            loop {
                // The object of `rdf:reifies` is a triple TERM. Parse `<<` here as a
                // triple term whether or not it carries parens, tolerating purrdf's
                // legacy non-parenthesized `<< s p o >>` triple-term serialization in
                // addition to the canonical `<<( s p o )>>` — in EVERY other position a
                // bare `<< … >>` keeps its W3C reifying-triple meaning (`reifying_triple`).
                let object = if matches!(&predicate, Node::Iri(p) if p.as_str() == RDF_REIFIES)
                    && self.at(&Token::TripleOpen)
                {
                    self.reifies_object_triple_term(graph, depth)?
                } else {
                    self.term(graph, depth)?
                };
                self.emit(subject, &predicate, &object, graph);
                self.maybe_reify_and_annotate(subject, &predicate, &object, graph, depth)?;
                if self.eat(&Token::Comma) {
                    continue;
                }
                break;
            }
            if self.eat(&Token::Semicolon) {
                // A predicateObjectList item after `;` is optional, so a run of `;`
                // (the doubled/trailing form `; ;`) denotes empty items and emits no
                // triples. Consume the run, then decide on the first non-`;` token.
                while self.eat(&Token::Semicolon) {}
                // `AnnotationClose` (`|}`) terminates a trailing `;` inside a
                // `{| … |}` annotation block.
                if self.at(&Token::Dot)
                    || self.at(&Token::RBracket)
                    || self.at(&Token::RBrace)
                    || self.at(&Token::AnnotationClose)
                {
                    break;
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    /// The RDF 1.2 reifier (`~ id`) / annotation (`{| pol |}`) suffix on a just-emitted
    /// `s p o` triple, matching the W3C RDF 1.2 Turtle/TriG reification expansion:
    ///
    /// - `~ id?` mints (or names) a reifier `r` and emits `r rdf:reifies <<( s p o )>>`.
    /// - `{| pol |}` reuses the immediately-preceding `~`-reifier if one is pending,
    ///   else mints a fresh reifier (with its own `rdf:reifies` triple), then evaluates
    ///   `pol` with that reifier as subject.
    ///
    /// Multiple suffixes chain (`~r1 ~r2`, `{| a |} {| b |}`); each annotation block
    /// consumes at most the one pending reifier, so a second block mints fresh.
    /// `~` is `Token::Tilde`; `{|`/`|}` are the `LBrace Pipe` / `Pipe RBrace` pairs.
    fn maybe_reify_and_annotate(
        &mut self,
        s: &Node,
        p: &Node,
        o: &Node,
        graph: Option<&Node>,
        depth: usize,
    ) -> Result<(), RdfDiagnostic> {
        let mut pending: Option<Node> = None;
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = if self.at_reifier_id() {
                    self.term(graph, depth)?
                } else {
                    self.next_bnode()
                };
                self.emit_reifies(&reifier, s, p, o, graph);
                pending = Some(reifier);
            } else if self.at(&Token::AnnotationOpen) {
                self.bump(); // `{|`
                let reifier = match pending.take() {
                    Some(reifier) => reifier,
                    None => {
                        let reifier = self.next_bnode();
                        self.emit_reifies(&reifier, s, p, o, graph);
                        reifier
                    }
                };
                self.predicate_object_list(&reifier, graph, depth)?;
                self.expect(&Token::AnnotationClose)?; // `|}`
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Emit `reifier rdf:reifies <<( s p o )>>` (the triple term is self-reifying via
    /// [`Node::Triple`]), the canonical RDF 1.2 reification triple.
    fn emit_reifies(&mut self, reifier: &Node, s: &Node, p: &Node, o: &Node, graph: Option<&Node>) {
        let triple_term = Node::Triple(
            Box::new(s.clone()),
            Box::new(p.clone()),
            Box::new(o.clone()),
        );
        self.emit(
            reifier,
            &Node::Iri(self.rdf_reifies.clone()),
            &triple_term,
            graph,
        );
    }

    /// Whether the next token can begin a reifier identifier (`iri | BlankNode`).
    fn at_reifier_id(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Token::Iri(_) | Token::PrefixedName(_, _) | Token::BlankNodeLabel(_) | Token::Anon
            )
        )
    }

    fn emit(&mut self, subject: &Node, predicate: &Node, object: &Node, graph: Option<&Node>) {
        // Exact-size reservation: three terms plus the optional graph, never regrown.
        let mut nodes = Vec::with_capacity(3 + usize::from(graph.is_some()));
        nodes.push(subject.clone());
        nodes.push(predicate.clone());
        nodes.push(object.clone());
        if let Some(graph) = graph {
            nodes.push(graph.clone());
        }
        // Record the subject's source position when tracking is on. `S::ENABLED` is a
        // const, so for `NoSpans` this block (and the lazy `LineIndex`) is dead code.
        if S::ENABLED
            && let Some(key) = subject_key(&nodes[0])
        {
            let src = self.src;
            let index = self
                .line_index
                .get_or_init(|| purrdf_iri::LineIndex::new(src));
            let position = index.locate(src, self.subject_off);
            self.collector.record(&key, position);
        }
        self.statements.push(nodes);
    }

    fn next_bnode(&mut self) -> Node {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        Node::Bnode(deterministic_label(id))
    }

    /// Resolve an IRIREF against the base in force — the ONLY constructor of
    /// [`Node::Iri`] on the Turtle/TriG path.
    ///
    /// Turtle admits relative references, so this is [`BaseScope::resolve`]. There is
    /// deliberately no "no base in scope, keep the raw text" fallthrough: that is what
    /// let an unresolved relative IRI reach the frozen IR and be emitted as invalid
    /// N-Triples. With no base and a relative reference this is a hard
    /// `iri-relative-no-base`, located at the offending token.
    fn resolve_iri(&self, raw: &str, line: u32, column: u32) -> Result<Iri, RdfDiagnostic> {
        self.base
            .resolve(raw)
            .map_err(|e| iri_err_at(&e, line, column))
    }

    /// Resolve a `PrefixedName` against the declared prefixes. The `(line, col)`
    /// is the position of the prefixed-name token itself, captured by the caller
    /// BEFORE it consumed the token (the token cursor has already advanced by the
    /// time we get here, so `self.loc()` would report the following token).
    fn resolve_prefixed(
        &self,
        prefix: &str,
        local: &str,
        line: u32,
        col: u32,
    ) -> Result<Node, RdfDiagnostic> {
        match self.prefixes.get(prefix) {
            Some(namespace) => {
                // The namespace was resolved to an absolute IRI when its `@prefix` was
                // read, so expansion is concatenation — NOT another base resolution,
                // which would re-resolve against whatever base is in force HERE and make
                // the same prefixed name mean different things in one document. The
                // concatenation is still validated: a local part can make it malformed.
                let expanded = format!("{namespace}{local}");
                purrdf_iri::parse(&expanded)
                    .map(Node::Iri)
                    .map_err(|e| iri_err_at(&e, line, col))
            }
            None => Err(err_at(format!("unknown prefix {prefix:?}"), line, col)),
        }
    }

    // token cursor helpers

    /// Resolve the current token's document-global 1-based `(line, column)` by
    /// scanning the full source with the shared [`purrdf_iri::LineIndex`]. The index is
    /// memoized in `self.line_index` (a `OnceCell`), so the FIRST lookup pays one
    /// whole-buffer scan and every subsequent lookup is an `O(log lines)` locate — a
    /// document with many lookups is linear, not quadratic, in the source length.
    fn loc(&self) -> (u32, u32) {
        let off = self
            .tokens
            .get(self.pos)
            .map(|s| s.start)
            .or_else(|| self.tokens.last().map(|s| s.end))
            .unwrap_or(0);
        let p = self
            .line_index
            .get_or_init(|| purrdf_iri::LineIndex::new(self.src))
            .locate(self.src, off);
        (p.line, p.column)
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    /// Consume the current token, MOVING it out of the owned buffer (a cheap
    /// `Token::Dot` placeholder is left behind; nothing re-reads a consumed
    /// position — `peek`/`peek2` look only at `pos` and beyond, which advance
    /// monotonically).
    fn bump(&mut self) -> Option<Token<'a>> {
        let t = self
            .tokens
            .get_mut(self.pos)
            .map(|s| std::mem::replace(&mut s.token, Token::Dot));
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn at(&self, token: &Token<'a>) -> bool {
        self.peek() == Some(token)
    }

    fn eat(&mut self, token: &Token<'a>) -> bool {
        if self.at(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw)) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &Token<'a>) -> Result<(), RdfDiagnostic> {
        if self.eat(token) {
            Ok(())
        } else {
            let (l, c) = self.loc();
            Err(err_at(
                format!("expected {token:?}, found {:?}", self.peek()),
                l,
                c,
            ))
        }
    }
}

fn numeric(lexical: String, datatype: &str) -> Node {
    Node::Literal {
        value: lexical,
        lang: None,
        direction: None,
        datatype: Some(datatype.to_owned()),
    }
}

/// A fresh blank-node label, delegating to the first-party
/// [`deterministic_blank_label`](super::ser_model::deterministic_blank_label): the
/// `gts_` prefix plus the Crockford Base32 ULID rendering of the zero-timestamp counter,
/// byte-identical to the prior purrdf-gts `deterministic_label("gts_", id)`.
fn deterministic_label(id: usize) -> String {
    super::ser_model::deterministic_blank_label(id)
}

// ───────────────────────────────────────────────────────────────────────────────
// build_gts: lower the flat statement list to an in-memory SerGraph
// ───────────────────────────────────────────────────────────────────────────────

/// Fixed-key hash of an atom's identity components. Any hash would do for
/// byte-determinism — term ids come from `terms` push order, never from
/// hash-iteration order — so a fixed-key `AHasher` just keeps SipHash off the hot
/// interning path (same pattern as `purrdf-core`'s `ir::builder::hash_of`).
fn hash_atom(
    kind: SerTermKind,
    value: &str,
    lang: Option<&str>,
    direction: Option<&str>,
    datatype: Option<usize>,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = ahash::AHasher::default();
    let tag: u8 = match kind {
        SerTermKind::Iri => 0,
        SerTermKind::Bnode => 1,
        SerTermKind::Literal => 2,
        SerTermKind::Triple => unreachable!("triple terms are keyed structurally, not as atoms"),
    };
    tag.hash(&mut hasher);
    value.hash(&mut hasher);
    lang.hash(&mut hasher);
    direction.hash(&mut hasher);
    datatype.hash(&mut hasher);
    hasher.finish()
}

/// Re-hash a STORED atom row byte-identically to [`hash_atom`] over the same
/// components (needed when the hash table resizes). Only atom rows (never
/// `SerTermKind::Triple`) live in the atoms table, so `value` is always present.
fn hash_stored_atom(term: &SerTerm) -> u64 {
    hash_atom(
        term.kind,
        term.value
            .as_deref()
            .expect("atom rows always carry a value"),
        term.lang.as_deref(),
        term.direction.as_deref(),
        term.datatype,
    )
}

/// Whether a stored atom row equals the borrowed lookup components. Keyed on the
/// datatype's interned ID rather than its string: equal datatype strings always
/// intern to the same id (and vice versa), so equality is unchanged.
fn atom_matches(
    term: &SerTerm,
    kind: SerTermKind,
    value: &str,
    lang: Option<&str>,
    direction: Option<&str>,
    datatype: Option<usize>,
) -> bool {
    term.kind == kind
        && term.value.as_deref() == Some(value)
        && term.lang.as_deref() == lang
        && term.direction.as_deref() == direction
        && term.datatype == datatype
}

/// The first-seen-order term interner, reproducing `from_nquads`'s `Interner`
/// so `dataset_from_ser_graph` re-interns its builder in the identical order.
///
/// **Store-once** (the `ir::builder::store_once` pattern): `terms` is the sole
/// owner of every atom's strings; `atoms` holds only `u32` indices into it, with
/// hash/eq that look INTO `terms`. First-seen order is untouched — ids still come
/// exclusively from `terms` push order.
struct Interner {
    /// Index table over the atom rows of `terms` (store-once dedup).
    atoms: hashbrown::HashTable<u32>,
    /// Structural dedup for triple terms; the key is three term ids (no strings).
    triples: HashMap<SerTriple3, usize>,
    terms: Vec<SerTerm>,
}

impl Interner {
    fn new() -> Self {
        Self {
            atoms: hashbrown::HashTable::new(),
            triples: HashMap::new(),
            terms: Vec::new(),
        }
    }

    /// Insert-or-find an atom row by its borrowed components, storing the strings
    /// exactly once (on first sight) in `terms`.
    fn intern_atom(
        &mut self,
        kind: SerTermKind,
        value: &str,
        lang: Option<&str>,
        direction: Option<&str>,
        datatype: Option<usize>,
    ) -> usize {
        let Self { atoms, terms, .. } = self;
        let hash = hash_atom(kind, value, lang, direction, datatype);
        if let Some(&id) = atoms.find(hash, |&i| {
            atom_matches(&terms[i as usize], kind, value, lang, direction, datatype)
        }) {
            return id as usize;
        }
        let id = terms.len();
        terms.push(SerTerm {
            kind,
            value: Some(value.to_owned()),
            datatype,
            lang: lang.map(str::to_owned),
            direction: direction.map(str::to_owned),
            reifier: None,
        });
        let id32 = u32::try_from(id).expect("term table exceeds u32::MAX entries");
        atoms.insert_unique(hash, id32, |&i| hash_stored_atom(&terms[i as usize]));
        id
    }

    fn atom(&mut self, node: &Node) -> usize {
        match node {
            Node::Iri(value) => {
                self.intern_atom(SerTermKind::Iri, value.as_str(), None, None, None)
            }
            Node::Bnode(value) => self.intern_atom(SerTermKind::Bnode, value, None, None, None),
            Node::Literal {
                value,
                lang,
                direction,
                datatype,
            } => {
                // A literal's datatype IRI is interned as its own IRI term (first-seen),
                // just as purrdf-gts does, so the term table matches. Interning it BEFORE
                // the literal lookup preserves first-seen order exactly: on a literal
                // cache hit the datatype was already interned at the literal's first
                // sighting, so this is a pure lookup; on a miss the prior code interned
                // it before pushing the literal too.
                let datatype_id = datatype
                    .as_deref()
                    .map(|iri| self.intern_atom(SerTermKind::Iri, iri, None, None, None));
                self.intern_atom(
                    SerTermKind::Literal,
                    value,
                    lang.as_deref(),
                    direction.as_deref(),
                    datatype_id,
                )
            }
            Node::Triple(..) => unreachable!("atom() is never called on a triple node"),
        }
    }

    fn node(&mut self, node: &Node, reifiers: &mut Vec<(usize, SerTriple3)>) -> usize {
        match node {
            Node::Triple(s, p, o) => {
                let s = self.node(s, reifiers);
                let p = self.node(p, reifiers);
                let o = self.node(o, reifiers);
                if let Some(id) = self.triples.get(&(s, p, o)) {
                    return *id;
                }
                let id = self.terms.len();
                // A triple TERM is self-reifying: its reifier is its own id, matching
                // the purrdf-gts shape so `dataset_from_ser_graph` recognizes the
                // self-reifier sentinel (an inline quoted-triple object, NOT a statement
                // reifier).
                self.terms.push(SerTerm {
                    kind: SerTermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(id),
                });
                self.triples.insert((s, p, o), id);
                reifiers.push((id, (s, p, o)));
                id
            }
            _ => self.atom(node),
        }
    }
}

/// The in-progress [`SerGraph`] lowering, fed ONE statement at a time.
///
/// This is `from_nquads`'s `build_gts` turned inside out: the loop body became
/// [`push`](Self::push) and the tail became [`finish`](Self::finish), with the state it
/// carried between iterations (the interner, the reifier table, the quad table) becoming
/// the struct's fields. Nothing about the lowering changed — the same first-seen
/// interning, the same statement-order quads, the same encounter-order reifiers, the same
/// `rdf:reifies` statement-layer shorthand — so [`build_gts_graph`] over a statement
/// slice and [`LineStreamParser`] over a stream of lines produce the IDENTICAL graph for
/// the identical statement sequence, by construction rather than by parallel maintenance.
struct GraphAccumulator {
    /// The first-seen-order term interner (the sequential serialization point that
    /// fixes every term id).
    interner: Interner,
    /// Reifier bindings in encounter order.
    reifiers: Vec<(usize, SerTriple3)>,
    /// Base quads in statement order.
    quads: Vec<(usize, usize, usize, Option<usize>)>,
}

impl GraphAccumulator {
    fn new() -> Self {
        Self {
            interner: Interner::new(),
            reifiers: Vec::new(),
            quads: Vec::new(),
        }
    }

    /// Lower ONE statement, in document order.
    fn push(&mut self, nodes: &Statement) -> Result<(), RdfDiagnostic> {
        let s = &nodes[0];
        let p = &nodes[1];
        let o = &nodes[2];
        let gname = nodes.get(3);

        // `<subject> rdf:reifies <<( s p o )>> .` in the DEFAULT graph is the
        // statement-layer reifier shorthand: bind the reifier, do NOT emit a base quad.
        if let (Node::Iri(_) | Node::Bnode(_), Node::Iri(pred_iri), Node::Triple(ts, tp, to), None) =
            (s, p, o, gname)
            && pred_iri.as_str() == RDF_REIFIES
        {
            let rid = self.interner.atom(s);
            let ss = self.interner.node(ts, &mut self.reifiers);
            let pp = self.interner.node(tp, &mut self.reifiers);
            let oo = self.interner.node(to, &mut self.reifiers);
            set_reifier(&mut self.reifiers, rid, (ss, pp, oo));
            return Ok(());
        }

        let sid = self.interner.node(s, &mut self.reifiers);
        let pid = self.interner.node(p, &mut self.reifiers);
        let oid = self.interner.node(o, &mut self.reifiers);
        let gid = gname.map(|node| self.interner.node(node, &mut self.reifiers));
        self.quads.push((sid, pid, oid, gid));
        Ok(())
    }

    /// Freeze the accumulated tables into the in-memory [`SerGraph`].
    fn finish(self) -> SerGraph {
        SerGraph {
            terms: self.interner.terms,
            quads: self.quads,
            // The reifier row carries an optional graph slot; this first-party text
            // parser binds reifiers only in the DEFAULT graph (the `rdf:reifies`
            // shorthand is gated on `None` graph in `push`), so the slot is always
            // `None`. Annotations are left in `quads` here and reclassified by
            // `fold_statement_layer`'s pass 2 (the `annotations` table stays empty).
            reifiers: self
                .reifiers
                .into_iter()
                .map(|(rid, spo)| (rid, spo, None))
                .collect(),
            ..Default::default()
        }
    }
}

/// Lower the flat statement list into the in-memory [`SerGraph`], reproducing
/// `from_nquads`'s `build_gts` (the `rdf:reifies` statement-layer shorthand,
/// first-seen interning, statement-order quads, encounter-order reifiers).
fn build_gts_graph(statements: &[Statement]) -> Result<SerGraph, RdfDiagnostic> {
    let mut accumulator = GraphAccumulator::new();
    for nodes in statements {
        accumulator.push(nodes)?;
    }
    Ok(accumulator.finish())
}

// ───────────────────────────────────────────────────────────────────────────────
// Streaming line front-end
// ───────────────────────────────────────────────────────────────────────────────

/// The N-Triples / N-Quads parser driven ONE LINE AT A TIME, for a caller reading from
/// a `Read` rather than holding the document.
///
/// It is the SAME parser: [`push_line`](Self::push_line) calls [`parse_one_line`] —
/// the one copy of the grammar, which the buffered sequential path and the
/// chunk-parallel workers also call — and hands the statement straight to a
/// [`GraphAccumulator`], which is `build_gts` with its loop turned inside out. Because
/// the statement sequence and the lowering are identical, the frozen graph is identical:
/// the equivalence is structural, not a property re-established by testing (though
/// [`stream`](super::stream) tests it over a non-trivial corpus anyway).
///
/// What is genuinely different is RESIDENCY. The buffered path holds the whole source
/// text AND the whole `Vec<Statement>` alive while it lowers; this holds exactly one
/// line's text and one statement, dropping each before reading the next. The
/// accumulated graph is unavoidable — it is the output — but the two source-sized
/// buffers are gone.
///
/// Only line-oriented formats reach here. Turtle / TriG cannot: their `@prefix` /
/// `@base` directives rebind mid-document and their anonymous blank nodes mint labels
/// from a document-ordered counter, so a line has no meaning independent of the lines
/// before it. [`new`](Self::new) rejects them by name rather than silently mis-parsing.
pub(super) struct LineStreamParser {
    accumulator: GraphAccumulator,
    /// N-Quads admits a fourth graph term; N-Triples does not.
    allow_graph: bool,
    /// The 1-based document line number of the NEXT line to be pushed.
    lineno: u32,
    /// The base in scope, for the diagnostic only — see [`TokenCursor::base`]. The
    /// streaming lane received the caller's base and dropped it on the floor, so it
    /// reported "no base IRI is in scope" to a caller who had supplied one, exactly as
    /// the buffered lane did.
    base: BaseScope,
}

impl LineStreamParser {
    /// Start a streaming parse of `format`, which MUST be N-Triples or N-Quads.
    pub(super) fn new(format: NativeRdfFormat, base: BaseScope) -> Result<Self, RdfDiagnostic> {
        let allow_graph = match format {
            NativeRdfFormat::NTriples => false,
            NativeRdfFormat::NQuads => true,
            other => {
                return Err(err(format!(
                    "{} is not a line-oriented format and cannot be parsed from a stream",
                    other.media_type()
                )));
            }
        };
        Ok(Self {
            accumulator: GraphAccumulator::new(),
            allow_graph,
            lineno: 1,
            base,
        })
    }

    /// Feed the next physical line, in document order.
    ///
    /// `raw` is the line WITHOUT its `EOL` terminator — exactly what [`physical_lines`]
    /// yields for the buffered path, so the diagnostics (line, column, message) are the
    /// buffered path's diagnostics. The reader that feeds this (the `LineReader` in
    /// [`super::stream`]) cuts at the same `EOL ::= [#xD#xA]+`, including
    /// a lone `#xD`, which is the whole point: a line path that split differently would
    /// make a document's meaning depend on how it arrived.
    pub(super) fn push_line(&mut self, raw: &str) -> Result<(), RdfDiagnostic> {
        if let Some(nodes) = parse_one_line(raw, self.allow_graph, self.lineno, &self.base)? {
            self.accumulator.push(&nodes)?;
        }
        self.lineno = self.lineno.saturating_add(1);
        Ok(())
    }

    /// Freeze the accumulated graph once the stream is exhausted.
    pub(super) fn finish(self) -> SerGraph {
        self.accumulator.finish()
    }
}

/// Record a reifier binding, idempotent on an identical rebind.
///
/// `rdf:reifies` is NOT a functional property: `<r> rdf:reifies <<( s p o1 )>>`
/// and `<r> rdf:reifies <<( s p o2 )>>` are both assertable, so a second,
/// different binding for one reifier is ordinary RDF 1.2 and every one of them
/// is kept. This does not make a quoted-triple TERM ambiguous: a triple term in
/// this model is SELF-reifying (its binding is keyed by its own term id), so it
/// never shares a key with a statement reifier.
fn set_reifier(reifiers: &mut Vec<(usize, SerTriple3)>, rid: usize, spo: SerTriple3) {
    if !reifiers
        .iter()
        .any(|&(r, existing)| r == rid && existing == spo)
    {
        reifiers.push((rid, spo));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_iri::BaseIri;

    /// A base scope rooted at a caller-supplied absolute base, as the library entry
    /// point builds for `parse_dataset(.., Some(base))`.
    fn rooted_scope(base: &str) -> BaseScope {
        BaseScope::rooted(
            BaseIri::parse(base).expect("fixture base is absolute"),
            BaseOrigin::Caller,
        )
    }

    /// Deterministic synthetic N-Quads with terms REPEATED across chunk boundaries
    /// (subjects/predicates/graphs cycle through small moduli), blank nodes, plain /
    /// language-tagged / directional / typed literals, quoted-triple object terms,
    /// and `rdf:reifies` reifier bindings with annotations — every term shape the
    /// N-Quads grammar admits, so the parallel-vs-sequential comparison exercises the
    /// whole interner.
    fn synthetic_nquads(rows: usize) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(rows * 160);
        out.push_str("# synthetic determinism fixture\n\n");
        for i in 0..rows {
            let g = i % 7;
            let s = i % 997;
            let p = i % 13;
            match i % 6 {
                0 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/p{p}> \
                     <https://example.org/o{}> <https://example.org/g{g}> .",
                    i % 991
                ),
                1 => writeln!(
                    out,
                    "_:b{} <https://example.org/knows> _:b{} .",
                    i % 499,
                    (i + 1) % 499
                ),
                2 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/label> \"row {i}\"@en ."
                ),
                3 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/title> \
                     \"\\u0645 {i}\"@ar--rtl <https://example.org/g{g}> ."
                ),
                4 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/count> \
                     \"{i}\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
                ),
                _ => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/asserts> \
                     <<( <https://example.org/a{}> <https://example.org/p{p}> \
                     <https://example.org/c{}> )>> .",
                    i % 89,
                    i % 83
                ),
            }
            .expect("write row");
            if i % 100 == 0 {
                writeln!(
                    out,
                    "<https://example.org/r{i}> \
                     <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                     <<( <https://example.org/a{}> <https://example.org/p{p}> \
                     <https://example.org/c{}> )>> .",
                    i % 89,
                    i % 83
                )
                .expect("write reifier");
                writeln!(
                    out,
                    "<https://example.org/r{i}> <https://example.org/confidence> \
                     \"0.9\"^^<http://www.w3.org/2001/XMLSchema#decimal> ."
                )
                .expect("write annotation");
            }
        }
        out
    }

    /// The determinism proof: a document ABOVE the parallel threshold parsed through
    /// the sequential and the (auto-selected) parallel path must be identical at every
    /// stage — statement list, `SerGraph` term table (interning order = term ids),
    /// quad/reifier/annotation rows, frozen dataset rows, and the canonical N-Quads
    /// bytes serialized back out.
    #[test]
    fn parallel_line_parse_is_byte_identical_to_sequential() {
        let text = synthetic_nquads(12_000);
        assert!(
            text.len() >= PARALLEL_MIN_BYTES,
            "fixture ({} bytes) must cross the {PARALLEL_MIN_BYTES}-byte parallel threshold",
            text.len()
        );

        let seq = parse_lines_sequential(&text, true, 1, &BaseScope::empty(), &mut NoSpans)
            .expect("sequential parse");
        let par = parse_lines(
            &text,
            true,
            LineParseMode::Auto,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect("parallel parse");
        assert!(seq == par, "statement lists must be identical");

        let graph_seq = build_gts_graph(&seq).expect("sequential graph");
        let graph_par = build_gts_graph(&par).expect("parallel graph");
        assert!(
            graph_seq.terms == graph_par.terms,
            "term tables (first-seen interning order = ids) must be identical"
        );
        assert!(graph_seq.quads == graph_par.quads, "quad rows must match");
        assert!(
            graph_seq.reifiers == graph_par.reifiers,
            "reifier rows must match"
        );
        assert!(
            graph_seq.annotations == graph_par.annotations,
            "annotation rows must match"
        );

        let ds_seq = super::super::parse::dataset_from_ser_graph(&graph_seq).expect("freeze seq");
        let ds_par = super::super::parse::dataset_from_ser_graph(&graph_par).expect("freeze par");
        assert_eq!(ds_seq.term_count(), ds_par.term_count());
        assert!(
            ds_seq.quads().collect::<Vec<_>>() == ds_par.quads().collect::<Vec<_>>(),
            "frozen quad rows (term ids + order) must be identical"
        );
        assert!(
            ds_seq.reifiers().collect::<Vec<_>>() == ds_par.reifiers().collect::<Vec<_>>(),
            "frozen reifier rows must be identical"
        );
        assert!(
            ds_seq.annotations().collect::<Vec<_>>() == ds_par.annotations().collect::<Vec<_>>(),
            "frozen annotation rows must be identical"
        );

        let bytes_seq = crate::native_codecs::serialize_dataset(
            &ds_seq,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize seq");
        let bytes_par = crate::native_codecs::serialize_dataset(
            &ds_par,
            "application/n-quads",
            crate::SerializeGraph::Dataset,
        )
        .expect("serialize par");
        assert!(
            bytes_seq == bytes_par,
            "canonical N-Quads bytes must be identical"
        );
    }

    /// Chunk geometry (down to a 1-byte target, i.e. one line per chunk) never changes
    /// the parsed statement list — including across comments, blank lines, CRLF line
    /// ends, and quoted-triple terms.
    #[test]
    fn chunk_geometry_never_changes_output() {
        let text = "# comment\n\n<https://e/s> <https://e/p> \"a\" .\r\n\
                    <https://e/s> <https://e/p> \"b\"@en <https://e/g> .\n\
                    _:b0 <https://e/p> <<( <https://e/x> <https://e/y> <https://e/z> )>> .\n";
        let expected = parse_lines_sequential(text, true, 1, &BaseScope::empty(), &mut NoSpans)
            .expect("sequential");
        for chunk_bytes in [1usize, 7, 16, 64, 4096] {
            let actual =
                parse_lines_parallel_with_chunk_size(text, true, chunk_bytes, &BaseScope::empty())
                    .expect("parallel parse");
            assert!(
                actual == expected,
                "chunk size {chunk_bytes} must not change the parse"
            );
        }
    }

    /// Line-aligned chunks partition the input exactly: concatenating the chunks
    /// reproduces the text, and every non-final chunk ends immediately after a `\n`.
    #[test]
    fn split_line_chunks_partitions_at_line_boundaries() {
        let text = "aaa\nbb\n\nccccc\nno-trailing-newline";
        for target in 1..=text.len() + 1 {
            let chunks = split_line_chunks(text, target);
            assert_eq!(chunks.concat(), text, "target {target} must partition");
            for chunk in &chunks[..chunks.len().saturating_sub(1)] {
                assert!(
                    chunk.ends_with('\n'),
                    "non-final chunk {chunk:?} (target {target}) must end at a line boundary"
                );
            }
        }
        assert_eq!(split_line_chunks("", 8), [] as [&str; 0]);
    }

    /// Error semantics: with an invalid line in an EARLY chunk and a different invalid
    /// line in a LATE chunk, the parallel path must report the earliest document-order
    /// diagnostic, byte-identical to the sequential path's (no chunk race).
    #[test]
    fn first_error_in_document_order_wins_across_chunks() {
        use std::fmt::Write as _;
        let mut text = String::new();
        for i in 0..40 {
            writeln!(text, "<https://e/s{i}> <https://e/p> <https://e/o{i}> .").expect("write");
        }
        text.push_str("<https://e/early-error> <https://e/p> .\n");
        for i in 40..400 {
            writeln!(text, "<https://e/s{i}> <https://e/p> <https://e/o{i}> .").expect("write");
        }
        text.push_str("this is not rdf\n");

        let seq_err = parse_lines_sequential(&text, true, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("sequential must fail");
        // A tiny chunk target guarantees the two bad lines land in different chunks.
        let par_err = parse_lines_parallel_with_chunk_size(&text, true, 256, &BaseScope::empty())
            .expect_err("parallel must fail");
        assert_eq!(
            par_err, seq_err,
            "parallel must report the sequential (earliest) diagnostic byte-identically"
        );
        // The diagnostic no longer embeds the raw line text; instead it carries a
        // 1-based location. Resolve that line back into the source to prove the
        // EARLIER (early-error) line's diagnostic won, not the late garbage line.
        let line = par_err
            .location
            .as_ref()
            .and_then(|l| l.line)
            .expect("located diagnostic");
        let offending = text
            .lines()
            .nth((line - 1) as usize)
            .expect("line in source");
        assert!(
            offending.contains("early-error"),
            "the EARLIER line's diagnostic must win, got line {line}: {offending}"
        );
    }

    /// The same earliest-error-wins guarantee through the real `Auto` threshold path
    /// (input above [`PARALLEL_MIN_BYTES`], errors in far-apart chunks).
    #[test]
    fn first_error_wins_on_auto_threshold_path() {
        let mut text = synthetic_nquads(600);
        text.push_str("<https://e/early-error> <https://e/p> .\n");
        text.push_str(&synthetic_nquads(12_000));
        text.push_str("late garbage line\n");
        text.push_str(&synthetic_nquads(600));
        assert!(
            text.len() >= PARALLEL_MIN_BYTES,
            "fixture must cross the parallel threshold"
        );

        let seq_err = parse_lines_sequential(&text, true, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("sequential must fail");
        let par_err = parse_lines(
            &text,
            true,
            LineParseMode::Auto,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect_err("parallel must fail");
        assert_eq!(par_err, seq_err, "diagnostics must be byte-identical");
        // Resolve the located line back into the source to prove the earlier chunk's
        // error (early-error line) won, not the late garbage line.
        let line = par_err
            .location
            .as_ref()
            .and_then(|l| l.line)
            .expect("located diagnostic");
        let offending = text
            .lines()
            .nth((line - 1) as usize)
            .expect("line in source");
        assert!(
            offending.contains("early-error"),
            "the earlier chunk's error must win, got line {line}: {offending}"
        );
    }

    /// The parallel line-number prefix sum must be correct for a parse error on the
    /// FINAL line when that line lacks a trailing newline — the arithmetic edge the
    /// determinism tests never hit (their fixtures all end in `\n`, and the error line
    /// is never the last). A large all-valid body crosses [`PARALLEL_MIN_BYTES`] so the
    /// `Auto` path takes the chunk-parallel branch; the deliberately invalid final line
    /// (a blank-node predicate) carries no `\n`, so `str::lines()` yields it as the last
    /// item and the parallel per-chunk base-line prefix sum must still report its true
    /// 1-based document line. The forced-sequential path parses the SAME input and must
    /// agree — the parallel-vs-sequential equivalence for a newline-less final line.
    #[test]
    fn parallel_final_line_without_newline_reports_correct_line() {
        use std::fmt::Write as _;
        // Enough valid rows to comfortably exceed the 1 MiB parallel threshold; each row
        // is ~72 bytes, so 20_000 rows is ~1.4 MiB.
        const VALID_ROWS: usize = 20_000;
        let mut text = String::with_capacity(VALID_ROWS * 80);
        for i in 0..VALID_ROWS {
            writeln!(
                text,
                "<http://example.org/s> <http://example.org/p> <http://example.org/o{i}> ."
            )
            .expect("write valid row");
        }
        // The final line is INVALID (a blank-node predicate) and has NO trailing newline.
        // It is document line `VALID_ROWS + 1` (rows 1..=VALID_ROWS ended in `\n`).
        text.push_str("<http://example.org/s> _:bad <http://example.org/o> .");
        assert!(
            !text.ends_with('\n'),
            "the final line must lack a trailing newline"
        );
        let expected_line = u32::try_from(VALID_ROWS + 1).expect("line fits u32");

        assert!(
            text.len() >= PARALLEL_MIN_BYTES,
            "fixture ({} bytes) must cross the {PARALLEL_MIN_BYTES}-byte parallel threshold",
            text.len()
        );

        // Auto path over a >1 MiB input takes the chunk-parallel branch.
        let par_err = parse_lines(
            &text,
            false,
            LineParseMode::Auto,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect_err("parallel must reject the final line");
        let par_line = par_err
            .location
            .as_ref()
            .and_then(|l| l.line)
            .expect("parallel diagnostic is located");
        assert_eq!(
            par_line, expected_line,
            "parallel path must report the newline-less final line's true 1-based number"
        );

        // Forced-sequential path over the identical input must agree.
        let seq_err = parse_lines_sequential(&text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("sequential must reject the final line");
        let seq_line = seq_err
            .location
            .as_ref()
            .and_then(|l| l.line)
            .expect("sequential diagnostic is located");
        assert_eq!(
            seq_line, expected_line,
            "sequential path must report the same final-line number"
        );
        assert_eq!(
            par_err, seq_err,
            "parallel and sequential diagnostics must be byte-identical for the final \
             newline-less line"
        );
    }

    /// A rejected N-Quads line carries a 1-based `(line, column)` location and no
    /// longer embeds the raw line text in the message.
    /// A DEEP INPUT IS A DIAGNOSTIC, NOT A DEAD PROCESS.
    ///
    /// Twenty thousand nested quoted triple terms in one N-Triples line used to abort the
    /// `purrdf` binary with `SIGABRT`: the recursive descent ran out of stack, so nothing
    /// unwound, no `Result` came back and an embedding host died with the library. Every
    /// nesting construct in both grammars is checked, because each one is its own recursion
    /// and a bound on only the reported one would have left the rest crashing.
    ///
    /// The depth is far past the limit on purpose — the guard has to fire on the way DOWN,
    /// before the frames exist, and a guard that only noticed afterwards would still abort.
    #[test]
    fn a_deeply_nested_input_is_refused_rather_than_overflowing_the_stack() {
        const DEPTH: usize = 20_000;
        let s = "<http://example.org/s>";
        let p = "<http://example.org/p>";
        let o = "<http://example.org/o>";
        let triple = format!("{s} {p} {o}");

        // Every construct that nests, in the grammar that admits it.
        let quoted = format!(
            "{s} {p} {}{triple}{} .",
            "<<( ".repeat(DEPTH),
            " )>>".repeat(DEPTH)
        );
        let reifying = format!(
            "{s} {p} {}{triple}{} .",
            "<< ".repeat(DEPTH),
            " >>".repeat(DEPTH)
        );
        let bnpl = format!(
            "{s} {p} {}{o}{} .",
            format!("[ {p} ").repeat(DEPTH),
            " ]".repeat(DEPTH)
        );
        let collection = format!("{s} {p} {}{o}{} .", "( ".repeat(DEPTH), " )".repeat(DEPTH));
        let annotation = format!(
            "{s} {p} {o} {}{o}{} .",
            format!("{{| {p} {o} ").repeat(DEPTH - 1) + &format!("{{| {p} "),
            " |}".repeat(DEPTH)
        );

        // The line family: N-Triples and N-Quads share one cursor, and the same line is a
        // legal shape for both, so both are driven.
        for allow_graph in [false, true] {
            let error =
                parse_lines_sequential(&quoted, allow_graph, 1, &BaseScope::empty(), &mut NoSpans)
                    .expect_err("a 20 000-deep quoted triple term must be refused");
            assert!(
                error.message.contains("nesting exceeds the parser limit"),
                "the refusal must name the limit, got: {}",
                error.message
            );
            assert!(
                error.location.as_ref().is_some_and(|l| l.column.is_some()),
                "the refusal is located at the offending token"
            );
        }

        // Turtle/TriG: five constructs, each its own recursion.
        for (name, text) in [
            ("quoted triple term", &quoted),
            ("reifying triple", &reifying),
            ("blank-node property list", &bnpl),
            ("collection", &collection),
            ("annotation block", &annotation),
        ] {
            let error = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .expect_err(&format!("{name}: a 20 000-deep document must be refused"));
            assert!(
                error.message.contains("nesting exceeds the parser limit"),
                "{name}: the refusal must name the limit, got: {}",
                error.message
            );
        }
    }

    /// …AND THE BOUND DOES NOT COST ORDINARY RDF 1.2.
    ///
    /// The other half of the contract: a bound that refused the nesting real documents use
    /// would be a worse defect than the crash it replaced. The IR itself refuses a
    /// triple-term nesting past 16, so the deepest triple term that can exist round-trips
    /// with the parser limit an order of magnitude away — and each syntactic construct is
    /// exercised well past anything an author writes by hand.
    #[test]
    fn ordinary_nesting_is_untouched_by_the_bound() {
        let s = "<http://example.org/s>";
        let p = "<http://example.org/p>";
        let o = "<http://example.org/o>";

        // A 16-deep quoted triple term in object position — the deepest the IR will hold.
        let mut term = format!("<<( {s} {p} {o} )>>");
        for _ in 1..16 {
            term = format!("<<( {s} {p} {term} )>>");
        }
        let line = format!("{s} {p} {term} .");
        parse_lines_sequential(&line, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect("16 nested triple terms");
        DocParser::new(&line, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("16 nested triple terms in Turtle");

        // Turtle's flat-lowering constructs reach the IR as ordinary statements, so nothing
        // downstream bounds them; 32 deep is far past hand-written RDF and must parse.
        const DEEP: usize = 32;
        for text in [
            format!(
                "{s} {p} {}{o}{} .",
                format!("[ {p} ").repeat(DEEP),
                " ]".repeat(DEEP)
            ),
            format!("{s} {p} {}{o}{} .", "( ".repeat(DEEP), " )".repeat(DEEP)),
            format!(
                "{s} {p} {o} {}{o}{} .",
                format!("{{| {p} {o} ").repeat(DEEP - 1) + &format!("{{| {p} "),
                " |}".repeat(DEEP)
            ),
        ] {
            DocParser::new(&text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .unwrap_or_else(|error| {
                    panic!("{DEEP}-deep nesting must parse: {}", error.message)
                });
        }
    }

    #[test]
    fn nquads_error_carries_line_and_column() {
        // The third line has a blank-node predicate, which is invalid.
        let text = "<http://ex/s> <http://ex/p> <http://ex/o> .\n\
                    <http://ex/s> <http://ex/p> <http://ex/o> .\n\
                    <http://ex/s> _:bad <http://ex/o> .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(3));
        assert!(loc.column.is_some(), "column must be attached");
        // The message no longer embeds the offending raw line text.
        assert!(
            !e.message.contains("_:bad <http://ex/o>"),
            "message must not embed the raw line text, got: {}",
            e.message
        );
    }

    /// `expect_dot` must report the column of the OFFENDING token, not the token
    /// after it. Constructed directly on the cursor because the sequential driver's
    /// term loop otherwise consumes every parseable token before `expect_dot` runs.
    #[test]
    fn expect_dot_column_points_at_offending_token() {
        // `<http://ex/b>` (the token where a `.` was expected) begins at column 15.
        let raw = "<http://ex/a> <http://ex/b>";
        let tokens = tokenize(raw).expect("tokenizes");
        let scope = BaseScope::empty();
        let mut cursor = TokenCursor::new(tokens, raw, 7, &scope);
        cursor.bump().expect("consume subject IRI");
        let e = cursor.expect_dot().expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(7));
        assert_eq!(loc.column, Some(15));
    }

    /// `term()`'s IRI branch reports the column of the invalid IRI itself, not the
    /// following token.
    #[test]
    fn term_iri_validation_column_points_at_iri() {
        // The object `<relative>` (no scheme) begins at column 29.
        let text = "<http://ex/s> <http://ex/p> <relative> .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(29));
        // N-Triples admits no relative reference at ALL, so this is the code that
        // says a base would not help — not `iri-relative-no-base`.
        assert_eq!(e.code, "iri-not-absolute-by-grammar");
        assert!(
            e.message.contains("\"relative\""),
            "message names the reference"
        );
    }

    /// A bad literal base direction reports the column of the language tag, not the
    /// following token.
    #[test]
    fn langtag_direction_column_points_at_langtag() {
        // The `@en--bad` tag begins at column 32.
        let text = "<http://ex/s> <http://ex/p> \"x\"@en--bad .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(32));
        assert!(e.message.contains("invalid literal base direction"));
    }

    /// A malformed language tag reports the column of the language tag itself.
    #[test]
    fn langtag_validation_column_points_at_langtag() {
        // The `@toolongprimary` tag begins at column 32 (primary subtag > 8 chars).
        let text = "<http://ex/s> <http://ex/p> \"x\"@toolongprimary .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(32));
        assert!(e.message.contains("invalid language tag"));
    }

    /// A non-IRI datatype after `^^` reports the column of the datatype token, not
    /// the token after it.
    #[test]
    fn datatype_non_iri_column_points_at_datatype() {
        // The datatype string `"y"` begins at column 34 (right after `^^`).
        let text = "<http://ex/s> <http://ex/p> \"x\"^^\"y\" .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(34));
        assert!(e.message.contains("datatype must be an IRI"));
    }

    /// A relative datatype IRI after `^^` reports the column of the datatype IRI.
    #[test]
    fn datatype_iri_validation_column_points_at_datatype() {
        // The datatype `<relative>` begins at column 34.
        let text = "<http://ex/s> <http://ex/p> \"x\"^^<relative> .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(34));
        assert_eq!(e.code, "iri-not-absolute-by-grammar");
        assert!(
            e.message.contains("\"relative\""),
            "message names the reference"
        );
    }

    /// An explicit `rdf:langString` datatype after `^^` reports the column of the
    /// datatype IRI, not the token after it.
    #[test]
    fn datatype_rdf_lang_string_column_points_at_datatype() {
        // The datatype IRI begins at column 34.
        let text = "<http://ex/s> <http://ex/p> \"x\"^^\
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .\n";
        let e = parse_lines_sequential(text, false, 1, &BaseScope::empty(), &mut NoSpans)
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(34));
        assert!(e.message.contains("RDF language-string datatype"));
    }

    /// A rejected Turtle document (DocParser path) carries a 1-based `(line, column)`
    /// resolved via the shared `LineIndex` over the full source.
    #[test]
    fn turtle_error_carries_line_and_column() {
        // The unknown prefix `nope:` on the third line must fail with a located error.
        let text = "@prefix ex: <https://example.org/> .\n\
                    ex:s ex:p ex:o .\n\
                    ex:s ex:p nope:o .\n";
        let e = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(3));
        assert!(loc.column.is_some(), "column must be attached");
        assert!(
            e.message.contains("unknown prefix"),
            "message keeps the informative reason, got: {}",
            e.message
        );
    }

    /// A `@base` with a non-absolute IRI (DocParser path) reports the column of the
    /// base-IRI token, not the token consumed after it.
    #[test]
    fn base_directive_column_points_at_relative_iri() {
        // The relative IRI `<relative>` begins at column 7 (right after `@base `).
        let text = "@base <relative> .\n";
        let e = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(7));
        // Turtle 6.1 lets a `@base` directive be relative, but only when a base is
        // already in force. With an EMPTY scope there is nothing to resolve against,
        // so the directive itself must be absolute.
        assert_eq!(e.code, "iri-non-absolute-base");
    }

    /// A malformed language tag on a Turtle literal (DocParser path) reports the
    /// column of the language tag, not the following token.
    #[test]
    fn doc_langtag_column_points_at_langtag() {
        // The `@bad--bad` tag begins at column 32.
        let text = "<http://ex/s> <http://ex/p> \"x\"@bad--bad .\n";
        let e = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(32));
        assert!(e.message.contains("invalid literal base direction"));
    }

    /// An undeclared prefix in the object position (DocParser path) reports the
    /// column of the prefixed name itself, not the following token.
    #[test]
    fn doc_unknown_prefix_column_points_at_prefixed_name() {
        // The undeclared `ex:o` begins at column 29.
        let text = "<http://ex/s> <http://ex/p> ex:o .\n";
        let e = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(29));
        assert!(e.message.contains("unknown prefix"));
    }

    // ── `'[' ']'` with nothing between the brackets is not a production ───────────
    //
    // Turtle's `blankNodePropertyList ::= '[' predicateObjectList ']'` has no empty
    // alternative, and `ANON ::= '[' WS* ']'` is a terminal, so `[ #` comment
    // newline `]` is not a Turtle document. This reader accepted it, and — unlike
    // the SPARQL side — it accepted it BOTH before and after the `ANON` scan was
    // tightened, because the comment is eaten as trivia either way. The lexer
    // change alone would have left this half silently wrong, which is why the
    // parser refusal is what closes it.

    /// Parse a Turtle document with no base and no span collection.
    fn turtle(text: &str) -> Result<Vec<Statement>, RdfDiagnostic> {
        DocParser::new(text, BaseScope::empty(), false, &mut NoSpans).parse()
    }

    /// The refusal: a bracket pair emptied only by a comment, in subject and in
    /// object position.
    #[test]
    fn a_comment_emptied_bracket_pair_is_not_a_blank_node_property_list() {
        for text in [
            "<http://ex/s> <http://ex/p> [ # c\n ] .\n",
            "[ # c\n ] <http://ex/p> <http://ex/o> .\n",
            "<http://ex/s> <http://ex/p> ( [ # c\n ] ) .\n",
        ] {
            let e = turtle(text).expect_err("`[` `]` with only a comment is not a production");
            assert!(
                e.message.contains("with nothing between them"),
                "{text:?} must be refused by the empty-bracket-pair arm, got: {}",
                e.message
            );
        }
    }

    /// Every neighbour still parses: the same comment in the same place once the
    /// list is populated, the anonymous blank node in each `WS` spelling, and the
    /// empty collection.
    #[test]
    fn the_neighbours_of_the_comment_emptied_bracket_pair_still_parse() {
        for text in [
            "<http://ex/s> <http://ex/p> [ <http://ex/q> <http://ex/o> ] .\n",
            "<http://ex/s> <http://ex/p> [ # c\n <http://ex/q> <http://ex/o> ] .\n",
            "[ <http://ex/q> <http://ex/o> ] <http://ex/p> <http://ex/o> .\n",
            "<http://ex/s> <http://ex/p> [] .\n",
            "<http://ex/s> <http://ex/p> [ ] .\n",
            "<http://ex/s> <http://ex/p> [\t\r\n] .\n",
            "<http://ex/s> <http://ex/p> () .\n",
            "<http://ex/s> <http://ex/p> ( ) .\n",
            "<http://ex/s> <http://ex/p> ( # c\n ) .\n",
        ] {
            turtle(text).unwrap_or_else(|e| panic!("must still parse {text:?}: {}", e.message));
        }
    }

    /// The tightened `WS` class reaches this reader too — it shares the SPARQL
    /// tokenizer — so a NO-BREAK SPACE neither closes an `ANON` nor separates two
    /// terms, while its ASCII neighbour does both.
    #[test]
    fn a_non_ws_space_is_refused_in_turtle_delimiter_position() {
        assert!(
            turtle("<http://ex/s> <http://ex/p> [\u{a0}] .\n").is_err(),
            "U+00A0 is not `WS`, so `[<NBSP>]` is not an ANON"
        );
        turtle("<http://ex/s> <http://ex/p> [ ] .\n").expect("`[ ]` is an ANON");
        // And the mirror: the very same scalar is LAWFUL raw inside `<...>`,
        // because `IRIREF` excludes only `#x00-#x20` and the nine reserved
        // delimiters. The two productions disagree about U+00A0 and both are
        // transcribed exactly, so tightening one must not move the other.
        turtle("<http://ex/s> <http://ex/p> <urn:ex:a\u{a0}b> .\n")
            .expect("U+00A0 is a lawful IRIREF body character");
    }

    /// A bare `/` in a prefixed-name local part (e.g. `ex:report/shacl/sarif`)
    /// must parse as ONE prefixed name and expand to the prefix namespace plus the
    /// slash-bearing local, matching oxigraph/purrdf-gts (strict Turtle would need
    /// `\/`, but real-world ontologies and fixtures use the bare form).
    #[test]
    fn turtle_prefixed_name_allows_bare_slash_in_local() {
        let text = "@prefix ex: <https://example.org/vocab/> .\n\
                    ex:report/shacl/sarif ex:projection/okf ex:report/shacl/sarif .";
        let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 1);
        let nodes = &statements[0];
        assert_eq!(
            nodes[0],
            Node::Iri(
                purrdf_iri::parse("https://example.org/vocab/report/shacl/sarif")
                    .expect("fixture IRI parses")
            )
        );
        assert_eq!(
            nodes[1],
            Node::Iri(
                purrdf_iri::parse("https://example.org/vocab/projection/okf")
                    .expect("fixture IRI parses")
            )
        );
        assert_eq!(
            nodes[2],
            Node::Iri(
                purrdf_iri::parse("https://example.org/vocab/report/shacl/sarif")
                    .expect("fixture IRI parses")
            )
        );
    }

    /// A prefixed name whose namespace is empty (`@prefix : <> .`) expands to a
    /// relative IRI reference that must be resolved against the document base, just like
    /// a bare IRIREF. This ensures `@prefix : <> . :knows` and `PREFIX : <base> :knows`
    /// name the same absolute IRI.
    #[test]
    fn turtle_empty_namespace_prefixed_name_resolves_against_base() {
        let text = "@prefix : <> .\n\
                    <#a> :knows <#b> .";
        let statements = DocParser::new(
            text,
            rooted_scope("http://example.org/"),
            false,
            &mut NoSpans,
        )
        .parse()
        .expect("parses");
        assert_eq!(statements.len(), 1);
        let nodes = &statements[0];
        assert_eq!(
            nodes[0],
            Node::Iri(purrdf_iri::parse("http://example.org/#a").expect("fixture IRI parses"))
        );
        assert_eq!(
            nodes[1],
            Node::Iri(purrdf_iri::parse("http://example.org/knows").expect("fixture IRI parses"))
        );
        assert_eq!(
            nodes[2],
            Node::Iri(purrdf_iri::parse("http://example.org/#b").expect("fixture IRI parses"))
        );
    }

    /// Regression for the lexer trailing-dot bug: `_:y.` at end of statement must
    /// tokenize as blank-node label `y` followed by a `Dot` terminator (not label
    /// `y.` with no terminator). Proves the fix end-to-end by parsing a document
    /// where the same blank node appears once immediately followed by `.` and once
    /// followed by whitespace, and asserting both statements resolve to the SAME
    /// blank-node identity.
    #[test]
    fn blank_node_immediately_followed_by_dot_is_same_node_as_later_reference() {
        let text = "@prefix : <https://example.org/> .\n\
                    :x :p _:y.\n\
                    _:y :q :z .\n";
        let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2, "must yield exactly two triples");

        let first = &statements[0];
        assert_eq!(
            first[0],
            Node::Iri(purrdf_iri::parse("https://example.org/x").expect("fixture IRI parses"))
        );
        assert_eq!(
            first[1],
            Node::Iri(purrdf_iri::parse("https://example.org/p").expect("fixture IRI parses"))
        );
        let Node::Bnode(label_as_object) = &first[2] else {
            panic!("expected blank-node object, got {:?}", first[2]);
        };

        let second = &statements[1];
        let Node::Bnode(label_as_subject) = &second[0] else {
            panic!("expected blank-node subject, got {:?}", second[0]);
        };
        assert_eq!(
            second[1],
            Node::Iri(purrdf_iri::parse("https://example.org/q").expect("fixture IRI parses"))
        );
        assert_eq!(
            second[2],
            Node::Iri(purrdf_iri::parse("https://example.org/z").expect("fixture IRI parses"))
        );

        assert_eq!(
            label_as_object, label_as_subject,
            "the trailing-dot blank node in statement 1 must be the SAME node as \
             the blank node referenced in statement 2"
        );
    }

    /// The Turtle/TriG `predicateObjectList` grammar makes the item after `;`
    /// OPTIONAL (`';' predicateObjectListItem?` in effect), so an interior doubled
    /// `;` denotes an empty item between two real ones and must emit no extra triple.
    #[test]
    fn turtle_doubled_semicolon_interior_emits_no_extra_triple() {
        let text = "<https://example.org/s> a <https://example.org/C> ; ; \
                     <https://example.org/p> <https://example.org/o> .";
        let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2);
    }

    /// A longer run of consecutive `;` (three in a row) collapses the same way: each
    /// extra `;` past the first is just another empty item, never an extra triple.
    #[test]
    fn turtle_semicolon_run_of_three_emits_no_extra_triples() {
        let text = "<https://example.org/s> <https://example.org/p1> <https://example.org/o1> ; ; ; \
                     <https://example.org/p2> <https://example.org/o2> .";
        let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2);
    }

    /// A trailing `; ;` before the terminating `.` is the doubled/trailing empty-item
    /// form: it must not require (or produce) a following predicate-object pair.
    #[test]
    fn turtle_trailing_doubled_semicolon_emits_no_extra_triple() {
        let text = "<https://example.org/s> <https://example.org/p> <https://example.org/o> ; ; .";
        let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 1);
    }

    /// The same empty-item rule applies inside a blank-node property list `[ … ]`:
    /// a doubled `;` there must parse and yield the same statements as the collapsed
    /// (single `;`) form.
    #[test]
    fn turtle_doubled_semicolon_inside_blank_node_property_list() {
        let collapsed = "<https://example.org/s> <https://example.org/p> \
                          [ <https://example.org/a> <https://example.org/b> ; \
                            <https://example.org/c> <https://example.org/d> ] .";
        let doubled = "<https://example.org/s> <https://example.org/p> \
                        [ <https://example.org/a> <https://example.org/b> ; ; \
                          <https://example.org/c> <https://example.org/d> ] .";
        let expected = DocParser::new(collapsed, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("collapsed parses");
        let actual = DocParser::new(doubled, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("doubled parses");
        assert_eq!(actual, expected);
    }

    /// The empty-item rule applies inside an RDF 1.2 annotation block `{| … |}` too: a
    /// doubled `;` separating two annotation predicate-object pairs must parse and
    /// yield IDENTICAL statements (including the minted reifier and its `rdf:reifies`
    /// triple) to the same document written with a single `;`.
    #[test]
    fn turtle_doubled_semicolon_inside_annotation_block() {
        let collapsed = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                          {| <https://example.org/a> <https://example.org/b> ; \
                             <https://example.org/c> <https://example.org/d> |} .";
        let doubled = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                        {| <https://example.org/a> <https://example.org/b> ; ; \
                           <https://example.org/c> <https://example.org/d> |} .";
        let expected = DocParser::new(collapsed, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("collapsed parses");
        let actual = DocParser::new(doubled, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("doubled parses");
        assert_eq!(actual, expected);
    }

    /// A trailing `;` inside an annotation block (`{| … ; |}`) is the empty-item form
    /// terminated by `Pipe` rather than `Dot`/`RBracket`/`RBrace` — this specifically
    /// exercises the `Pipe` branch of the terminator check after the `;` run is drained.
    #[test]
    fn turtle_trailing_semicolon_inside_annotation_block_before_pipe() {
        let no_trailing = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                            {| <https://example.org/a> <https://example.org/b> |} .";
        let trailing = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                         {| <https://example.org/a> <https://example.org/b> ; |} .";
        let expected = DocParser::new(no_trailing, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("no-trailing parses");
        let actual = DocParser::new(trailing, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("trailing parses");
        assert_eq!(actual, expected);
    }

    /// A LEADING empty item is still illegal: `predicate()` runs at the top of the
    /// `predicateObjectList` loop before any `;` handling, so a `;` with no preceding
    /// predicate-object pair for this subject has no predicate to parse and must error.
    #[test]
    fn turtle_leading_semicolon_before_any_predicate_is_an_error() {
        let text = "<https://example.org/s> ; <https://example.org/p> <https://example.org/o> .";
        assert!(
            DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .is_err()
        );
    }

    /// A subject followed immediately by `;` and then `.` (no predicate-object pair at
    /// all) is also illegal for the same reason: `predicate()` has nothing to consume.
    #[test]
    fn turtle_leading_semicolon_with_no_predicate_object_is_an_error() {
        let text = "<https://example.org/s> ; .";
        assert!(
            DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .is_err()
        );
    }

    /// A LEADING `;` inside a blank-node property list `[ … ]` is also illegal:
    /// `predicate()` runs at the top of the `predicateObjectList` loop before any `;`
    /// handling, so a `;` with no preceding predicate-object pair inside the blank
    /// node has no predicate to parse and must error.
    #[test]
    fn turtle_leading_semicolon_inside_blank_node_property_list_is_an_error() {
        let text = "<https://example.org/s> <https://example.org/p> \
                     [ ; <https://example.org/a> <https://example.org/b> ] .";
        assert!(
            DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .is_err()
        );
    }

    /// A LEADING `;` inside an RDF 1.2 annotation block `{| … |}` is also illegal for
    /// the same reason: `predicate()` has nothing to consume before the first `;`.
    #[test]
    fn turtle_leading_semicolon_inside_annotation_block_is_an_error() {
        let text = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                     {| ; <https://example.org/a> <https://example.org/b> |} .";
        assert!(
            DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .is_err()
        );
    }

    /// A DOUBLED trailing `;` before the annotation-block `Pipe` (`{| a b ; ; |}`)
    /// must drain the whole run of semicolons and then break on `Pipe`, parsing
    /// IDENTICALLY to the no-trailing form — this pairs the `while self.eat(&Token::Semicolon) {}`
    /// drain with the `Pipe` terminator, distinct from the single-`;` trailing case.
    #[test]
    fn turtle_doubled_trailing_semicolon_inside_annotation_block_before_pipe() {
        let no_trailing = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                            {| <https://example.org/a> <https://example.org/b> |} .";
        let doubled = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                        {| <https://example.org/a> <https://example.org/b> ; ; |} .";
        let expected = DocParser::new(no_trailing, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("no-trailing parses");
        let actual = DocParser::new(doubled, BaseScope::empty(), false, &mut NoSpans)
            .parse()
            .expect("doubled parses");
        assert_eq!(actual, expected);
    }

    // ───────────────────────────────────────────────────────────────────────────
    // The line grammar's `WS`, on ALL THREE line paths
    //
    // `WS ::= #x20 | #x9 | #xD | #xA` decides where a statement starts and whether a
    // line is blank or a comment, so a document can change MEANING (not merely
    // validity) when the test widens. Every vector below is therefore executed on the
    // sequential path, the chunk-parallel path and the streaming `LineStreamParser`:
    // they are separate call sites, and a fix to one is not a fix to the others.
    // ───────────────────────────────────────────────────────────────────────────

    /// One path's reading of a document: how many statements it found, or why it
    /// refused.
    type PathAnswer = Result<usize, RdfDiagnostic>;

    /// The three line paths, in the order [`all_line_paths`] reports them.
    const LINE_PATHS: [&str; 3] = ["sequential", "chunk-parallel", "line-stream"];

    /// Feed `text` to the streaming [`LineStreamParser`] one physical line at a time —
    /// exactly as `stream::stream_line_format` drives it — and report the quad count of
    /// the graph it froze.
    ///
    /// The lines come from [`physical_lines`], which is the splitter the streaming
    /// `LineReader` reproduces over a `Read`; `stream`'s own tests drive the real reader
    /// at read granularities down to one byte, so the two halves of that claim are each
    /// executed rather than assumed.
    fn line_stream_answer(text: &str) -> PathAnswer {
        let mut parser = LineStreamParser::new(NativeRdfFormat::NTriples, BaseScope::empty())?;
        for line in physical_lines(text) {
            parser.push_line(line.text)?;
        }
        Ok(parser.finish().quads.len())
    }

    /// `text` as read by every N-Triples line path.
    ///
    /// The chunk-parallel path is driven with a 1-byte chunk target, so EVERY line is
    /// its own chunk and every line boundary is also a chunk boundary — the geometry
    /// most likely to expose a rule that was only ever applied in the buffered lane.
    fn all_line_paths(text: &str) -> [PathAnswer; 3] {
        let base = BaseScope::empty();
        [
            parse_lines_sequential(text, false, 1, &base, &mut NoSpans)
                .map(|statements| statements.len()),
            parse_lines_parallel_with_chunk_size(text, false, 1, &base)
                .map(|statements| statements.len()),
            line_stream_answer(text),
        ]
    }

    /// Every line path reads `text` as exactly `statements` statements.
    #[track_caller]
    fn every_line_path_accepts(text: &str, statements: usize) {
        for (path, answer) in LINE_PATHS.iter().zip(all_line_paths(text)) {
            let found =
                answer.unwrap_or_else(|e| panic!("{path} must accept {text:?}, refused it: {e:?}"));
            assert_eq!(found, statements, "{path} statement count for {text:?}");
        }
    }

    /// Every line path refuses `text` with the IDENTICAL diagnostic; that diagnostic is
    /// handed back so the caller can pin its message and its column.
    #[track_caller]
    fn every_line_path_refuses(text: &str) -> RdfDiagnostic {
        let mut answers = all_line_paths(text)
            .into_iter()
            .zip(LINE_PATHS)
            .map(|(answer, path)| match answer {
                Ok(count) => panic!("{path} must refuse {text:?}, read {count} statements"),
                Err(diagnostic) => diagnostic,
            });
        let first = answers.next().expect("three paths");
        for (diagnostic, path) in answers.zip(&LINE_PATHS[1..]) {
            assert_eq!(
                diagnostic, first,
                "{path} must report the sequential diagnostic byte-identically"
            );
        }
        first
    }

    /// The 1-based (line, column) a located diagnostic names.
    #[track_caller]
    fn located(diagnostic: &RdfDiagnostic) -> (u32, u32) {
        let location = diagnostic.location.as_ref().expect("located diagnostic");
        (
            location.line.expect("line"),
            location.column.expect("column"),
        )
    }

    /// The over-refusal side, executed: everything the line grammar DOES admit around
    /// whitespace still parses, on all three paths.
    ///
    /// This is the neighbour set for every refusal tightened below. `WS` has four
    /// members and all four of them still separate, terminate and indent.
    #[test]
    fn the_valid_neighbours_of_the_exact_ws_class_still_parse() {
        let plain = "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n";
        every_line_path_accepts(plain, 1);
        // Indented with SPACEs, and with TABs.
        every_line_path_accepts("    <urn:ex:s> <urn:ex:p> <urn:ex:o> .\n", 1);
        every_line_path_accepts("\t\t<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n", 1);
        // Trailing `WS` after the terminator.
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> . \t\n", 1);
        // CRLF line endings (`str::lines` leaves nothing, but a lone trailing `#xD`
        // on an un-terminated final line is `WS` and must still be trimmed).
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\n", 1);
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r", 1);
        // A file with NO trailing newline.
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> .", 1);
        // A comment line, an INDENTED comment line, a blank line, and a line of
        // nothing but `WS` — four lines, no statements, no error.
        every_line_path_accepts("# a comment\n\t  # an indented comment\n\n \t \n", 0);
        // And all of it at once, still exactly the two statements.
        every_line_path_accepts(
            "# leading comment\r\n\
             \t<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\n\
             \n\
             \x20\x20# indented comment\n\
             <urn:ex:s2> <urn:ex:p> \"v\" .",
            2,
        );
    }

    /// U+00A0 INSIDE a token is content, is lawful, and is untouched: this is a rule
    /// about the LINE grammar, not about what a literal or an `IRIREF` may hold.
    #[test]
    fn a_no_break_space_inside_a_token_still_parses() {
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> \"a\u{a0}b\" .\n", 1);
        every_line_path_accepts("<urn:ex:a\u{a0}b> <urn:ex:p> <urn:ex:o> .\n", 1);
        // And the scalar SURVIVES into the term rather than being trimmed out of it.
        let statements = parse_lines_sequential(
            "<urn:ex:a\u{a0}b> <urn:ex:p> <urn:ex:o> .\n",
            false,
            1,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect("a NO-BREAK SPACE in an IRIREF body is `ucschar` and parses");
        assert_eq!(
            subject_key(&statements[0][0]).as_deref(),
            Some("urn:ex:a\u{a0}b"),
            "the NO-BREAK SPACE must survive verbatim in the resolved IRI"
        );
    }

    /// A line holding nothing but U+00A0 used to trim to the empty string and be
    /// dropped as BLANK. It is not blank — U+00A0 is not `WS` — and it is not a
    /// comment, so it is now refused, pointing at the NO-BREAK SPACE itself.
    #[test]
    fn a_line_of_only_a_no_break_space_is_not_a_blank_line() {
        let diagnostic = every_line_path_refuses(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n\u{a0}\n<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
        );
        assert_eq!(located(&diagnostic), (2, 1));
        assert!(
            diagnostic.message.contains("U+00A0 NO-BREAK SPACE"),
            "the diagnostic must name the offending scalar: {}",
            diagnostic.message
        );
        // The neighbours: the SPACE-only line and the TAB-only line the author almost
        // certainly meant are still blank lines, and the document still parses.
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n \n<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
            2,
        );
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n\t\n<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
            2,
        );
    }

    /// A U+00A0 before `#` used to trim away, so the line was read as a COMMENT and
    /// everything on it was silently discarded. A comment may open at the line start or
    /// after `WS`, and a NO-BREAK SPACE is neither.
    #[test]
    fn a_no_break_space_before_a_hash_does_not_open_a_comment() {
        let diagnostic = every_line_path_refuses("\u{a0}# this does not open a comment\n");
        assert_eq!(located(&diagnostic), (1, 1));
        assert!(
            diagnostic.message.contains("U+00A0 NO-BREAK SPACE"),
            "the diagnostic must name the offending scalar: {}",
            diagnostic.message
        );
        // The neighbours: a comment at the line start and a comment after real `WS`
        // are both still comments, carrying no statement and raising no error.
        every_line_path_accepts("# this opens a comment\n", 0);
        every_line_path_accepts(" \t# this opens a comment too\n", 0);
        // And a `#` INSIDE a token is still not a comment either way.
        every_line_path_accepts("<urn:ex:s#f> <urn:ex:p> \"a # b\" .\n", 1);
    }

    /// A LEADING U+FEFF is refused, by name, on every line path — the decision
    /// documented at [`parse_one_line`].
    #[test]
    fn a_leading_byte_order_mark_is_refused_by_name() {
        let diagnostic = every_line_path_refuses("\u{feff}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n");
        assert_eq!(located(&diagnostic), (1, 1));
        assert!(
            diagnostic
                .message
                .contains("U+FEFF ZERO WIDTH NO-BREAK SPACE"),
            "the diagnostic must name the character: {}",
            diagnostic.message
        );
        // The neighbour: the identical document without the mark parses.
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n", 1);
    }

    /// A SECOND consecutive U+FEFF is refused under this decision AND would be refused
    /// under the strip-exactly-one alternative, so it pins the boundary of the decision
    /// rather than restating it.
    #[test]
    fn a_second_consecutive_byte_order_mark_is_refused() {
        let diagnostic =
            every_line_path_refuses("\u{feff}\u{feff}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n");
        assert_eq!(located(&diagnostic), (1, 1));
        assert!(
            diagnostic
                .message
                .contains("U+FEFF ZERO WIDTH NO-BREAK SPACE"),
            "the diagnostic must name the character: {}",
            diagnostic.message
        );
    }

    /// A MID-LINE U+FEFF outside a token is refused as the stray `Word` it lexes into —
    /// again under either decision, since no strip rule reaches past a line's head.
    #[test]
    fn a_mid_line_byte_order_mark_outside_a_token_is_refused() {
        let diagnostic = every_line_path_refuses("<urn:ex:s> \u{feff}<urn:ex:p> <urn:ex:o> .\n");
        assert_eq!(
            located(&diagnostic),
            (1, 12),
            "the column must name the mark, scalar 12 of the line"
        );
        assert!(
            !diagnostic.message.contains("byte order mark"),
            "a mid-line mark is not a byte order mark and must not be called one: {}",
            diagnostic.message
        );
        // The neighbour: U+FEFF INSIDE a token is `ucschar`/literal content, lawful,
        // and still parses — the refusal reaches the line grammar only.
        every_line_path_accepts("<urn:ex:a\u{feff}b> <urn:ex:p> <urn:ex:o> .\n", 1);
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> \"a\u{feff}b\" .\n", 1);
    }

    /// The neighbour that decides the whole U+FEFF question: it is a LAWFUL name
    /// character, and the refusal above must not reach it.
    ///
    /// `PN_CHARS_BASE` includes `[#xFDF0-#xFFFD]`, and `#xFEFF` is inside that range, so
    /// U+FEFF is a legal `BLANK_NODE_LABEL` scalar. A blanket ban on the character —
    /// the obvious wrong turn when writing the refusal above — would reject this line,
    /// which is exactly the over-refusal mirror of the silent re-read. U+00A0, which is
    /// in NO name class, is the contrast that shows the two are not interchangeable.
    #[test]
    fn a_byte_order_mark_inside_a_blank_node_label_is_a_name_character_and_parses() {
        every_line_path_accepts("_:a\u{feff}b <urn:ex:p> <urn:ex:o> .\n", 1);
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> _:a\u{feff}b .\n", 1);
        // A U+FEFF that STARTS the label, right after `_:` — `PN_CHARS_U` admits it in
        // the label's first position too, so the line's first scalar being `_` is what
        // matters, not what follows it.
        every_line_path_accepts("_:\u{feff}b <urn:ex:p> <urn:ex:o> .\n", 1);
        // The contrast: U+00A0 is in no name class, so the same shape is refused.
        let diagnostic = every_line_path_refuses("_:a\u{a0}b <urn:ex:p> <urn:ex:o> .\n");
        assert!(
            diagnostic.message.contains("U+00A0 NO-BREAK SPACE"),
            "the diagnostic must name the offending scalar: {}",
            diagnostic.message
        );
    }

    /// The Turtle/TriG path answers a leading U+FEFF the same way the line paths
    /// do — by name, at (1, 1), with the same verdict it always had.
    ///
    /// Before this, the mark reached [`DocParser::term`] as the `Word` it
    /// lawfully lexes into and came back as `unexpected token
    /// Some(Word("\u{feff}"))`, which names a token no author typed.
    #[test]
    fn a_turtle_document_opening_with_a_byte_order_mark_is_refused_by_name() {
        for allow_named_graphs in [false, true] {
            let text = "\u{feff}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n";
            let diagnostic =
                DocParser::new(text, BaseScope::empty(), allow_named_graphs, &mut NoSpans)
                    .parse()
                    .expect_err("a byte order mark opens no Turtle/TriG production");
            assert_eq!(located(&diagnostic), (1, 1));
            assert!(
                diagnostic
                    .message
                    .contains("U+FEFF ZERO WIDTH NO-BREAK SPACE"),
                "the diagnostic must name the character: {}",
                diagnostic.message
            );
            // The neighbour: the identical document without the mark parses, so
            // the arm refuses the mark and not the statement after it.
            let statements = DocParser::new(
                &text[3..],
                BaseScope::empty(),
                allow_named_graphs,
                &mut NoSpans,
            )
            .parse()
            .expect("the same document without the mark is well formed");
            assert_eq!(statements.len(), 1);
        }
    }

    /// The over-refusal neighbour that decides the whole U+FEFF question on this
    /// path too: `#xFEFF` is inside `PN_CHARS_BASE`'s `[#xFDF0-#xFFFD]`, so it is
    /// a lawful scalar INSIDE a name, an IRI or a literal — and a `@prefix`
    /// directive before a marked statement does not make the mark a mark either.
    #[test]
    fn a_byte_order_mark_inside_a_turtle_token_is_a_name_character_and_parses() {
        for text in [
            "@prefix ex: <urn:ex:> .\nex:a\u{feff}b ex:p ex:o .\n",
            "_:a\u{feff}b <urn:ex:p> <urn:ex:o> .\n",
            "_:\u{feff}b <urn:ex:p> <urn:ex:o> .\n",
            "<urn:ex:a\u{feff}b> <urn:ex:p> <urn:ex:o> .\n",
            "<urn:ex:s> <urn:ex:p> \"a\u{feff}b\" .\n",
        ] {
            let statements = DocParser::new(text, BaseScope::empty(), false, &mut NoSpans)
                .parse()
                .unwrap_or_else(|e| panic!("U+FEFF is a lawful name scalar: {}", e.message));
            assert_eq!(statements.len(), 1, "{text:?}");
        }
    }

    /// The Turtle/TriG codec reads its names through the shared scanner, so the
    /// `BLANK_NODE_LABEL` and `PN_LOCAL` HEAD classes reach this path too.
    ///
    /// The blank-node half is the one that had teeth: this codec's own writer
    /// validates labels with `purrdf_rdf_core::blank_label::is_valid_blank_node_label`,
    /// which implements the same production, so a label accepted here and refused
    /// there could be read in and never written back out.
    #[test]
    fn the_name_head_classes_reach_the_turtle_path() {
        let parse =
            |text: &str| DocParser::new(text, BaseScope::empty(), false, &mut NoSpans).parse();
        for label in ["-a", ".a", "\u{300}a", "\u{b7}a"] {
            let text = format!("_:{label} <urn:ex:p> <urn:ex:o> .\n");
            assert!(
                parse(&text).is_err(),
                "_:{label} opens no BLANK_NODE_LABEL, and the writer would refuse it"
            );
            assert!(
                !purrdf_core::blank_label::is_valid_blank_node_label(label),
                "egress refuses _:{label}, so ingress must too"
            );
        }
        for local in ["-a", ".a", "\u{300}a"] {
            let text = format!("@prefix ex: <urn:ex:> .\n<urn:ex:s> ex:{local} <urn:ex:o> .\n");
            assert!(parse(&text).is_err(), "ex:{local} opens no PN_LOCAL");
        }
        // The lawful neighbours, each of which the egress validator also accepts.
        for label in ["0a", "_a", "a-b", "a.b", "cafe\u{301}", "a\u{feff}b"] {
            let text = format!("_:{label} <urn:ex:p> <urn:ex:o> .\n");
            assert_eq!(
                parse(&text).expect("a lawful label").len(),
                1,
                "_:{label} is a lawful BLANK_NODE_LABEL"
            );
            assert!(
                purrdf_core::blank_label::is_valid_blank_node_label(label),
                "ingress accepts _:{label}, so egress must too"
            );
        }
        for local in ["", "0a", "_a", ":a", "a-b", "a.b", "%20a", "report/x"] {
            let text = format!("@prefix ex: <urn:ex:> .\n<urn:ex:s> ex:{local} <urn:ex:o> .\n");
            assert_eq!(
                parse(&text).expect("a lawful local name").len(),
                1,
                "ex:{local} is a lawful prefixed name here"
            );
        }
    }

    /// [`column_in_raw`] and [`parse_one_line`] measure the SAME leading run.
    ///
    /// Nothing else in the crate would notice if they drifted: the parse outcome is
    /// unchanged and only the reported column moves, so this is the one place that
    /// pins it. The two halves are asserted separately and then against each other.
    #[test]
    fn the_diagnostic_column_and_the_trim_move_in_lockstep() {
        // Half one — the column's own view of the leading run. `WS` members are part
        // of it; Unicode whitespace that is not `WS` is the first CONTENT scalar.
        assert_eq!(column_in_raw("  <urn:ex:s>", 0), 3);
        assert_eq!(column_in_raw("\t<urn:ex:s>", 0), 2);
        assert_eq!(column_in_raw("\r\n<urn:ex:s>", 0), 3);
        assert_eq!(column_in_raw("\u{a0}<urn:ex:s>", 0), 1);
        assert_eq!(column_in_raw("  \u{a0}<urn:ex:s>", 0), 3);
        assert_eq!(column_in_raw("\u{b}<urn:ex:s>", 0), 1);
        assert_eq!(column_in_raw("\u{c}<urn:ex:s>", 0), 1);
        assert_eq!(column_in_raw("\u{3000}<urn:ex:s>", 0), 1);

        // Half two — the parser's. Two SPACEs then a NO-BREAK SPACE: the refusal must
        // land on scalar 3, the NO-BREAK SPACE, not on the `<` after it.
        let diagnostic = every_line_path_refuses("  \u{a0}<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n");
        assert_eq!(located(&diagnostic), (1, 3));

        // The neighbour: the same indentation without the NO-BREAK SPACE parses, and a
        // refusal further along that line is reported at the right scalar too.
        every_line_path_accepts("  <urn:ex:s> <urn:ex:p> <urn:ex:o> .\n", 1);
        let short = every_line_path_refuses("  <urn:ex:s> <urn:ex:p> .\n");
        assert_eq!(located(&short), (1, 3));
    }

    // ───────────────────────────────────────────────────────────────────────────
    // The line grammar's `EOL`, on ALL THREE line paths
    //
    // `EOL ::= [#xD#xA]+` decides where one statement ends and the next begins, so a
    // splitter that answers it with `str::lines` — `#xA` only, with a `#xD` before it
    // absorbed — does not misparse a document, it DROPS statements out of one. Every
    // vector below runs on the sequential path, the chunk-parallel path (at a 1-byte
    // chunk target, so every line boundary is also a chunk boundary) and the streaming
    // parser, and asserts they answer identically.
    // ───────────────────────────────────────────────────────────────────────────

    /// The defect itself: two statements separated by a LONE `#xD` are two statements.
    ///
    /// `str::lines` treats a lone `#xD` as ordinary text, so the pair arrived as ONE
    /// line; the parser then took the tokens up to the first `.` and threw the rest
    /// away, so the second statement was lost with exit zero and no diagnostic.
    #[test]
    fn a_lone_carriage_return_terminates_a_line_on_every_path() {
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r<urn:ex:s2> <urn:ex:p> <urn:ex:o> .\r",
            2,
        );
        // The same pair with no terminator after the last statement.
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r<urn:ex:s2> <urn:ex:p> <urn:ex:o> .",
            2,
        );
        // Three statements, one terminator of each shape.
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\
             <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n\
             <urn:ex:s3> <urn:ex:p> <urn:ex:o> .\r\n",
            3,
        );
        // A lone `#xD` also ends a COMMENT, which otherwise swallows the statement
        // after it — the silent-drop shape one layer up from the statement case.
        every_line_path_accepts("# a comment\r<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r", 1);
    }

    /// `#xD#xA` is ONE terminator, not two: a CRLF document holds no empty line between
    /// consecutive statements, and its line NUMBERS are the numbers an editor shows.
    ///
    /// Splitting the pair into two terminators is silent in the statement count (an
    /// empty line carries no statement) and loud only here, in the diagnostic: every
    /// line after the first would be reported at twice its true number.
    #[test]
    fn a_crlf_pair_is_one_terminator_not_two() {
        every_line_path_accepts(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\n\
             <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\r\n\
             <urn:ex:s3> <urn:ex:p> <urn:ex:o> .\r\n",
            3,
        );
        // An error on the THIRD line of a CRLF document is reported at line 3.
        let diagnostic = every_line_path_refuses(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\n\
             <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\r\n\
             <urn:ex:s3> <urn:ex:p> .\r\n",
        );
        assert_eq!(located(&diagnostic), (3, 1));
        // And the same document with LF endings answers identically, which is the
        // property that makes a file portable between editors.
        let lf = every_line_path_refuses(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\n\
             <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n\
             <urn:ex:s3> <urn:ex:p> .\n",
        );
        assert_eq!(lf, diagnostic, "CRLF and LF must read the SAME document");
    }

    /// A longer run of `[#xD#xA]` separates two statements exactly as a single
    /// terminator does — the `+` in the production — while the 1-based line numbers keep
    /// counting PHYSICAL lines, which is what every editor and every diagnostic means.
    #[test]
    fn a_run_of_terminators_separates_statements_and_still_counts_physical_lines() {
        for separator in ["\r\r", "\n\n", "\r\n\r\n", "\n\r", "\r\n\n\r"] {
            let text = format!(
                "<urn:ex:s> <urn:ex:p> <urn:ex:o> .{separator}<urn:ex:s2> <urn:ex:p> \
                 <urn:ex:o> .\n"
            );
            every_line_path_accepts(&text, 2);
        }
        // Two blank lines between the statements: the bad line is document line 4.
        let diagnostic = every_line_path_refuses(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> .\r\n\r\n\r\n<urn:ex:s2> <urn:ex:p> .\r\n",
        );
        assert_eq!(located(&diagnostic), (4, 1));
    }

    /// A trailing terminator at end of file yields no phantom final line, whatever shape
    /// it took, and a document with NO trailing terminator still yields its last line.
    #[test]
    fn a_trailing_terminator_adds_no_phantom_line() {
        for tail in ["", "\n", "\r", "\r\n"] {
            let text = format!("<urn:ex:s> <urn:ex:p> <urn:ex:o> .{tail}");
            every_line_path_accepts(&text, 1);
        }
        // The empty document, and a document of nothing but terminators.
        for text in ["", "\n", "\r", "\r\n", "\r\n\r\n", "\n\r\n\r"] {
            every_line_path_accepts(text, 0);
        }
    }

    /// [`physical_lines`] IS `str::lines` on every document that holds no lone `#xD` —
    /// which is the whole of the change, stated as a property rather than as a list of
    /// examples, so a future edit to the splitter cannot quietly move the LF case too.
    #[test]
    fn the_splitter_reproduces_str_lines_wherever_no_lone_carriage_return_appears() {
        for text in [
            "",
            "a",
            "a\n",
            "a\nb",
            "a\nb\n",
            "a\r\nb\r\n",
            "a\r\n\r\nb",
            "\n\n\n",
            "a\n\nb\n",
            "no trailing newline",
        ] {
            let split: Vec<&str> = physical_lines(text).map(|line| line.text).collect();
            let expected: Vec<&str> = text.lines().collect();
            assert_eq!(split, expected, "{text:?}");
            // And the partition is exact: every byte is either line content or one of
            // the terminators the splitter measured.
            let total: usize = physical_lines(text)
                .map(|line| line.text.len() + line.terminator)
                .sum();
            assert_eq!(total, text.len(), "{text:?} must partition exactly");
        }
        // The one document where the two DIFFER, which is the defect this splitter
        // exists to fix.
        assert_eq!(
            physical_lines("a\rb").map(|l| l.text).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!("a\rb".lines().collect::<Vec<_>>(), vec!["a\rb"]);
    }

    /// A chunk boundary must never fall BETWEEN `#xD` and `#xA`, or a CRLF document
    /// would parse into a different number of lines depending on the chunk size — the
    /// machine's memory geometry deciding what a document means.
    #[test]
    fn split_line_chunks_never_splits_a_crlf_pair() {
        let text = "aaa\r\nbb\r\n\r\nccccc\r\nno-trailing-terminator";
        for target in 1..=text.len() + 1 {
            let chunks = split_line_chunks(text, target);
            assert_eq!(chunks.concat(), text, "target {target} must partition");
            for (index, chunk) in chunks.iter().enumerate() {
                if index + 1 == chunks.len() {
                    continue;
                }
                assert!(
                    chunk.ends_with('\n') || chunk.ends_with('\r'),
                    "non-final chunk {chunk:?} (target {target}) must end at a terminator"
                );
                assert!(
                    !(chunk.ends_with('\r') && chunks[index + 1].starts_with('\n')),
                    "target {target} split a `#xD#xA` pair across chunks {chunk:?}"
                );
            }
            // The invariant that makes the parallel path equal the sequential one:
            // concatenating the chunks' line streams reproduces the document's.
            let per_chunk: Vec<&str> = chunks
                .iter()
                .flat_map(|chunk| physical_lines(chunk).map(|line| line.text))
                .collect();
            let whole: Vec<&str> = physical_lines(text).map(|line| line.text).collect();
            assert_eq!(per_chunk, whole, "target {target} must re-join exactly");
        }
        // A CR-only document has no `#xA` at all: the chunker must still find a
        // boundary, or the parallel path would degenerate to a single chunk.
        let cr_only = "aaa\rbb\r\rccccc\r";
        assert!(
            split_line_chunks(cr_only, 4).len() > 1,
            "a `#xD`-terminated document must still chunk"
        );
    }

    /// Chunk geometry never changes a CRLF document's parse — asserted at EVERY chunk
    /// size, including one byte, so every boundary in the fixture is exercised.
    #[test]
    fn crlf_chunk_geometry_never_changes_output() {
        let text = "# comment\r\n\r\n<https://e/s> <https://e/p> \"a\" .\r\n\
                    <https://e/s> <https://e/p> \"b\"@en <https://e/g> .\r\
                    _:b0 <https://e/p> <<( <https://e/x> <https://e/y> <https://e/z> )>> .\r\n";
        let expected = parse_lines_sequential(text, true, 1, &BaseScope::empty(), &mut NoSpans)
            .expect("sequential");
        assert_eq!(expected.len(), 3, "the fixture holds three statements");
        for target in 1..=text.len() + 1 {
            let actual =
                parse_lines_parallel_with_chunk_size(text, true, target, &BaseScope::empty())
                    .expect("parallel parse");
            assert!(
                actual == expected,
                "chunk size {target} must not change the parse"
            );
        }
    }

    /// Tokens after the statement terminator are REFUSED, naming what was found and
    /// where. They used to be discarded silently — a statement a user typed, dropped
    /// with exit zero.
    #[test]
    fn tokens_after_the_terminator_are_refused_not_discarded() {
        let diagnostic = every_line_path_refuses(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> . <urn:ex:s2> <urn:ex:p> <urn:ex:o> .\n",
        );
        assert_eq!(
            located(&diagnostic),
            (1, 36),
            "the column must name the first leftover token"
        );
        assert!(
            diagnostic.message.contains("urn:ex:s2"),
            "the diagnostic must name what was found: {}",
            diagnostic.message
        );
        // A stray `.` mid-statement ends it early; the remainder is leftover, not lost.
        let early = every_line_path_refuses("<urn:ex:s> <urn:ex:p>. <urn:ex:o> .\n");
        assert!(
            early.message.contains("after the statement terminator"),
            "the diagnostic must say what it is refusing: {}",
            early.message
        );
        // The over-refusal side, executed: everything the grammar DOES allow after the
        // `.` still parses. A comment, `WS`, a `#xD`-terminated line, and (N-Quads) the
        // graph label, which comes BEFORE the terminator and is not leftover at all.
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> . # a comment\n", 1);
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> .\t \n", 1);
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> <urn:ex:o> . # a comment\r", 1);
        let quads = parse_lines_sequential(
            "<urn:ex:s> <urn:ex:p> <urn:ex:o> <urn:ex:g> . # a comment\n",
            true,
            1,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect("a graph label precedes the terminator and is not leftover");
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].len(), 4);
    }

    /// The over-refusal direction of the `EOL` split: a `#xD` INSIDE a token.
    ///
    /// The productions make this unrepresentable rather than merely unusual —
    /// `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'` names
    /// `#xD` in its exclusion set, and ``IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)*
    /// '>'`` excludes all of `#x00-#x20` — so a RAW `#xD` between quotes or between
    /// angle brackets was ALREADY refused, by the shared lexer, before this splitter
    /// existed. Splitting there therefore takes no lawful document away; this test pins
    /// that by refusing both shapes, and then executes the spellings that ARE lawful.
    #[test]
    fn a_carriage_return_inside_a_token_is_not_content_of_that_token() {
        // Refused, on every path, in both token bodies.
        every_line_path_refuses("<urn:ex:s> <urn:ex:p> \"a\rb\" .\n");
        every_line_path_refuses("<urn:ex:a\rb> <urn:ex:p> <urn:ex:o> .\n");
        // The lawful neighbours: the ESCAPED carriage return, which is how the scalar is
        // written into a literal, parses AND survives into the term verbatim.
        every_line_path_accepts("<urn:ex:s> <urn:ex:p> \"a\\rb\" .\n", 1);
        let statements = parse_lines_sequential(
            "<urn:ex:s> <urn:ex:p> \"a\\rb\" .\n",
            false,
            1,
            &BaseScope::empty(),
            &mut NoSpans,
        )
        .expect("`\\r` is ECHAR and parses");
        assert_eq!(
            statements[0][2],
            Node::Literal {
                value: "a\rb".to_owned(),
                lang: None,
                direction: None,
                datatype: None,
            },
            "the escape must decode to the scalar, which the splitter never sees"
        );
    }
}

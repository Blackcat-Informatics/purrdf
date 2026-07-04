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

use purrdf_iri::Position;
use purrdf_sparql_algebra::lexer::{tokenize, tokenize_turtle, Spanned, Token};
use rayon::prelude::*;

use super::media_type::NativeRdfFormat;
use super::ser_model::{SerGraph, SerTerm, SerTermKind, SerTriple3};
use super::span::{NoSpans, SpanCollector};
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

/// 1-based column (counted in Unicode scalar values) of a byte offset that lies
/// within the TRIMMED content of `raw`. `trimmed_off` is a byte offset into
/// `raw.trim()` (i.e. token spans from tokenizing the trimmed line); it is
/// rebased onto `raw` by adding the leading-whitespace width.
fn column_in_raw(raw: &str, trimmed_off: usize) -> u32 {
    let lead = raw.len() - raw.trim_start().len();
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
    Iri(String),
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
pub(super) fn parse_to_gts_graph_mode<S: SpanCollector>(
    format: NativeRdfFormat,
    text: &str,
    base_iri: Option<&str>,
    mode: LineParseMode,
    collector: &mut S,
) -> Result<SerGraph, RdfDiagnostic> {
    let statements = match format {
        NativeRdfFormat::NTriples => parse_lines(text, false, mode, collector)?,
        NativeRdfFormat::NQuads => parse_lines(text, true, mode, collector)?,
        NativeRdfFormat::Turtle => DocParser::new(text, base_iri, false, collector).parse()?,
        NativeRdfFormat::TriG => DocParser::new(text, base_iri, true, collector).parse()?,
        NativeRdfFormat::RdfXml => {
            return Err(err("RDF/XML is not a line/Turtle-family format"));
        }
        NativeRdfFormat::TriX | NativeRdfFormat::HexTuples => {
            return Err(err("TriX / HexTuples are not line/Turtle-family formats"));
        }
    };
    build_gts_graph(&statements)
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
        Node::Iri(iri) => Some(iri.clone()),
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
    collector: &mut S,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    if mode == LineParseMode::Auto && text.len() >= PARALLEL_MIN_BYTES {
        // The parallel path is `NoSpans`-only (each chunk gets its own ZST collector);
        // span tracking forces sequential (see `parse_dataset_with`), so `S::ENABLED`
        // is always false here. The parallel branch stays non-generic in the collector.
        return parse_lines_parallel(text, allow_graph);
    }
    parse_lines_sequential(text, allow_graph, 1, collector)
}

/// Split `text` into line-aligned chunks of roughly `target_bytes` each: every chunk
/// (except possibly the last) ends immediately after a `'\n'`, so concatenating the
/// chunks' [`str::lines`] streams reproduces `text.lines()` exactly. `'\n'` is ASCII,
/// so every boundary is a valid UTF-8 char boundary.
fn split_line_chunks(text: &str, target_bytes: usize) -> Vec<&str> {
    let bytes = text.as_bytes();
    let target = target_bytes.max(1);
    let mut chunks = Vec::with_capacity(text.len() / target + 1);
    let mut start = 0;
    while start < text.len() {
        let mut end = start.saturating_add(target).min(text.len());
        if end < text.len() {
            end = match memchr::memchr(b'\n', &bytes[end..]) {
                Some(offset) => end + offset + 1,
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
fn parse_lines_parallel(text: &str, allow_graph: bool) -> Result<Vec<Statement>, RdfDiagnostic> {
    let threads = rayon::current_num_threads().max(1);
    let target = (text.len() / (threads * 4).max(1))
        .clamp(PARALLEL_MIN_CHUNK_BYTES, PARALLEL_MAX_CHUNK_BYTES);
    parse_lines_parallel_with_chunk_size(text, allow_graph, target)
}

/// [`parse_lines_parallel`] with an explicit chunk size (tests use a tiny size to
/// force many chunks over small fixtures; chunk geometry never changes the output).
fn parse_lines_parallel_with_chunk_size(
    text: &str,
    allow_graph: bool,
    target_bytes: usize,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let chunks = split_line_chunks(text, target_bytes);
    // Each chunk is a contiguous line-aligned slice; chunk 0 begins at document
    // line 1 and chunk k begins at `1 + (total '\n' in chunks[0..k])`. Precompute
    // those 1-based base lines (a sequential prefix sum) so every per-chunk
    // diagnostic reports the SAME document-global line the sequential path would,
    // keeping the parallel path byte-identical (line numbers included).
    let mut base_lines = Vec::with_capacity(chunks.len());
    let mut base = 1u32;
    for chunk in &chunks {
        base_lines.push(base);
        let newlines =
            u32::try_from(chunk.bytes().filter(|&b| b == b'\n').count()).unwrap_or(u32::MAX);
        base = base.saturating_add(newlines);
    }
    // Phase 1: parallel per-chunk tokenize+parse (on wasm32 rayon runs this inline).
    let per_chunk: Vec<Result<Vec<Statement>, RdfDiagnostic>> = chunks
        .par_iter()
        .enumerate()
        .map(|(i, chunk)| parse_lines_sequential(chunk, allow_graph, base_lines[i], &mut NoSpans))
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
    collector: &mut S,
) -> Result<Vec<Statement>, RdfDiagnostic> {
    let mut statements = Vec::new();
    let mut lineno = base_line;
    // Running document-global byte offset of the current line's first byte. Only
    // maintained when span tracking is on (`S::ENABLED`); span tracking forces the
    // sequential path (see `parse_dataset_with`), so `text` here is the whole
    // document and this offset is document-global. For `NoSpans` the compiler proves
    // `S::ENABLED == false` and deletes every touch of `line_offset`, leaving the hot
    // path byte-identical. `advance_line_offset` steps it past a line plus its `\n`
    // or `\r\n` terminator (`str::lines` strips both).
    let mut line_offset = 0usize;
    let advance_line_offset = |offset: &mut usize, raw: &str| {
        *offset += raw.len();
        match text.as_bytes().get(*offset) {
            Some(b'\r') => *offset += 2,
            Some(b'\n') => *offset += 1,
            _ => {}
        }
    };
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            if S::ENABLED {
                advance_line_offset(&mut line_offset, raw);
            }
            lineno = lineno.saturating_add(1);
            continue;
        }
        let tokens = tokenize(line).map_err(|e| {
            let col = e.byte_offset().map_or(1, |at| column_in_raw(raw, at));
            err_at(e.to_string(), lineno, col)
        })?;
        let mut cursor = TokenCursor::new(tokens, raw, lineno);
        let mut nodes = Vec::new();
        while !cursor.at_statement_end() {
            nodes.push(cursor.term(allow_graph)?);
        }
        cursor.expect_dot()?;
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
        validate_statement(&nodes, lineno, column_in_raw(raw, 0), allow_graph)?;
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
            advance_line_offset(&mut line_offset, raw);
        }
        statements.push(nodes);
        lineno = lineno.saturating_add(1);
    }
    Ok(statements)
}

/// A cursor over one line's lexer tokens, parsing N-Triples/N-Quads terms.
///
/// The cursor OWNS its token buffer (discarded after the line is parsed), so
/// [`bump`](Self::bump) can MOVE each consumed token out instead of deep-cloning
/// its `String` payload.
struct TokenCursor<'a> {
    tokens: Vec<Spanned>,
    pos: usize,
    raw: &'a str,
    lineno: u32,
}

impl<'a> TokenCursor<'a> {
    fn new(tokens: Vec<Spanned>, raw: &'a str, lineno: u32) -> Self {
        Self {
            tokens,
            pos: 0,
            raw,
            lineno,
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

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    /// Consume the current token, MOVING it out of the owned buffer (a cheap
    /// `Token::Dot` placeholder is left behind; the cursor never re-reads a
    /// consumed position — `peek` looks only at `pos`, which has advanced).
    fn bump(&mut self) -> Option<Token> {
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

    fn expect_dot(&mut self) -> Result<(), RdfDiagnostic> {
        let col = self.col();
        match self.bump() {
            Some(Token::Dot) | None => Ok(()),
            other => Err(err_at(
                format!("expected '.' terminator, found {other:?}"),
                self.lineno,
                col,
            )),
        }
    }

    /// Parse one term in N-Triples/N-Quads syntax. `allow_triple_subject` is unused
    /// here (the lexer admits `<<( … )>>` everywhere); positional validity is checked
    /// later by [`validate_statement`], exactly as the purrdf-gts parser does.
    fn term(&mut self, _allow_triple_subject: bool) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::TripleOpen) => self.quoted_triple(),
            Some(Token::Iri(_)) => {
                let col = self.col();
                let Some(Token::Iri(value)) = self.bump() else {
                    unreachable!()
                };
                validate_iri(&value, self.lineno, col)?;
                Ok(Node::Iri(value))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label))
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
    fn quoted_triple(&mut self) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.term(true)?;
        let p = self.term(true)?;
        let o = self.term(true)?;
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
                let (base, dir) = split_lang_direction(&raw, self.lineno, col)?;
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
                validate_iri(&iri, self.lineno, col)?;
                if matches!(iri.as_str(), RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                    return Err(err_at(
                        "literal cannot explicitly use the RDF language-string datatype",
                        self.lineno,
                        col,
                    ));
                }
                datatype = Some(iri);
            }
            _ => {}
        }
        Ok(Node::Literal {
            value,
            lang,
            direction,
            datatype,
        })
    }

    fn expect(&mut self, token: &Token) -> Result<(), RdfDiagnostic> {
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

/// Whether `value` carries an absolute-IRI scheme (`scheme:`), matching the
/// purrdf-gts `has_iri_scheme`.
fn has_iri_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    for ch in chars {
        if ch == ':' {
            return true;
        }
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

/// Validate an absolute IRI's shape after UCHAR-decoding.
///
/// The N-Triples/N-Quads IRIREF grammar is `'<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`:
/// a character is forbidden as a RAW byte but PERMITTED when introduced by a `UCHAR`
/// escape. The lexer ([`tokenize`]) already enforces the raw-byte restriction — its
/// IRIREF scan STOPS at a raw whitespace / `< " { } | ^ \`` — and decodes every
/// `\u`/`\U` escape, so by the time the value reaches here any otherwise-forbidden
/// character can ONLY have come from a (legal) UCHAR. So this checks ONLY the
/// absolute-IRI requirement (N-Triples/N-Quads admit no relative IRIs); rejecting the
/// decoded special characters would wrongly fail legal UCHAR IRIs such as
/// `<urn:ex: >` (W3C test060), whose canonical form keeps the decoded character.
fn validate_iri(value: &str, line_no: u32, column: u32) -> Result<(), RdfDiagnostic> {
    if value.is_empty() || value.starts_with("//") || !has_iri_scheme(value) {
        return Err(err_at(
            format!("IRI must be absolute (needs a scheme), found <{value}>"),
            line_no,
            column,
        ));
    }
    Ok(())
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

fn validate_subject(
    node: &Node,
    line_no: u32,
    column: u32,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode]) {
        return Ok(());
    }
    if allow_triple_subject {
        if let Node::Triple(s, p, o) = node {
            return validate_triple(s, p, o, line_no, column, allow_triple_subject);
        }
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

fn validate_object(
    node: &Node,
    line_no: u32,
    column: u32,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    if node_is(node, &[is_iri, is_bnode, is_literal]) {
        return Ok(());
    }
    if let Node::Triple(s, p, o) = node {
        return validate_triple(s, p, o, line_no, column, allow_triple_subject);
    }
    Err(err_at("invalid object term", line_no, column))
}

fn validate_triple(
    s: &Node,
    p: &Node,
    o: &Node,
    line_no: u32,
    column: u32,
    allow_triple_subject: bool,
) -> Result<(), RdfDiagnostic> {
    validate_subject(s, line_no, column, allow_triple_subject)?;
    validate_predicate(p, line_no, column)?;
    validate_object(o, line_no, column, allow_triple_subject)
}

fn validate_statement(
    nodes: &[Node],
    line_no: u32,
    column: u32,
    allow_graph: bool,
) -> Result<(), RdfDiagnostic> {
    validate_subject(&nodes[0], line_no, column, allow_graph)?;
    validate_predicate(&nodes[1], line_no, column)?;
    validate_object(&nodes[2], line_no, column, allow_graph)?;
    if let Some(graph_name) = nodes.get(3) {
        if !node_is(graph_name, &[is_iri, is_bnode]) {
            return Err(err_at("invalid graph name term", line_no, column));
        }
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
    tokens: Vec<Spanned>,
    pos: usize,
    prefixes: HashMap<String, String>,
    base_iri: Option<String>,
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
    /// Newline table over `src`, built lazily on the FIRST recorded subject and reused
    /// for the rest of the parse. Never built when `S::ENABLED` is false.
    line_index: Option<purrdf_iri::LineIndex>,
}

impl<'a, 'c, S: SpanCollector> DocParser<'a, 'c, S> {
    fn new(
        text: &'a str,
        base_iri: Option<&str>,
        allow_named_graphs: bool,
        collector: &'c mut S,
    ) -> Self {
        let mut prefixes = HashMap::new();
        prefixes.insert("rdf".to_owned(), RDF_NS.to_owned());
        Self {
            tokens: Vec::new(),
            pos: 0,
            prefixes,
            base_iri: base_iri.map(str::to_owned),
            bnode_counter: 0,
            allow_named_graphs,
            statements: Vec::new(),
            src: text,
            collector,
            subject_off: 0,
            line_index: None,
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

    fn parse(mut self) -> Result<Vec<Statement>, RdfDiagnostic> {
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
                let graph = self.term(None)?;
                self.expect(&Token::LBrace)?;
                self.graph_block(&graph)?;
                continue;
            }
            if S::ENABLED {
                self.subject_off = self.cur_off();
            }
            let first = self.term(None)?;
            if self.eat(&Token::LBrace) {
                if !self.allow_named_graphs {
                    let (l, c) = self.loc();
                    return Err(err_at("Turtle input cannot contain graph blocks", l, c));
                }
                self.graph_block(&first)?;
            } else {
                self.statement_after_subject(&first, None)?;
            }
        }
        Ok(self.statements)
    }

    /// Consume a `@prefix`/`@base`/`@version` or `PREFIX`/`BASE`/`VERSION` directive
    /// when present. Returns whether one was consumed.
    fn try_directive(&mut self) -> Result<bool, RdfDiagnostic> {
        // `@prefix` / `@base` / `@version` lex as a `LangTag` (the `@` form).
        if let Some(Token::LangTag(tag)) = self.peek() {
            match tag.as_str() {
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

    fn prefix_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let (prefix, _) = self.expect_prefix_ns()?;
        let iri = self.expect_iri_raw()?;
        self.prefixes.insert(prefix, iri);
        if require_dot {
            self.expect(&Token::Dot)?;
        } else {
            self.eat(&Token::Dot);
        }
        Ok(())
    }

    fn base_directive(&mut self, require_dot: bool) -> Result<(), RdfDiagnostic> {
        let (l, c) = self.loc();
        let iri = self.expect_iri_raw()?;
        if !has_iri_scheme(&iri) {
            return Err(err_at(format!("base IRI must be absolute: {iri:?}"), l, c));
        }
        self.base_iri = Some(iri);
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
            Some(Token::PrefixedName(p, l)) if l.is_empty() => Ok((p, l)),
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
            Some(Token::Iri(s)) => Ok(s),
            other => Err(err_at(format!("expected an IRIREF, found {other:?}"), l, c)),
        }
    }

    fn term(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        match self.peek() {
            Some(Token::TripleOpen) => {
                // Distinguish the value form `<<( s p o )>>` from the reifying form
                // `<< s p o [~r] >>` by the immediately-following `(`.
                if self.peek2() == Some(&Token::LParen) {
                    self.parenthesized_quoted_triple(graph)
                } else {
                    self.reifying_triple(graph)
                }
            }
            Some(Token::Iri(_)) => {
                let Some(Token::Iri(raw)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Iri(self.resolve_iri(&raw)))
            }
            Some(Token::PrefixedName(_, _)) => {
                let (l, c) = self.loc();
                let Some(Token::PrefixedName(prefix, local)) = self.bump() else {
                    unreachable!()
                };
                self.resolve_prefixed(&prefix, &local, l, c)
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(label)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Bnode(label))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(self.next_bnode())
            }
            Some(Token::LBracket) => self.blank_node_property_list(graph),
            Some(Token::LParen) => self.collection(graph),
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
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                let Some(Token::Word(value)) = self.bump() else {
                    unreachable!()
                };
                Ok(Node::Literal {
                    value,
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
    fn quoted_component(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
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
                    self.term(graph)
                } else {
                    let (l, c) = self.loc();
                    Err(err_at(
                        "RDF collection is not allowed inside a quoted triple",
                        l,
                        c,
                    ))
                }
            }
            _ => self.term(graph),
        }
    }

    fn predicate(&mut self) -> Result<Node, RdfDiagnostic> {
        if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(Node::Iri(RDF_TYPE.to_owned()));
        }
        self.term(None)
    }

    fn parenthesized_quoted_triple(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        self.expect(&Token::LParen)?;
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Node::Triple(Box::new(s), Box::new(p), Box::new(o)))
    }

    /// A triple TERM in `rdf:reifies` object position: `<<( s p o )>>` (canonical) or
    /// the legacy non-parenthesized `<< s p o >>` (purrdf pre-0.9.11 triple-term
    /// serialization). Always a [`Node::Triple`] — never a minted reifier — because the
    /// object of `rdf:reifies` denotes the reified triple itself.
    fn reifies_object_triple_term(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let parenthesized = self.eat(&Token::LParen);
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
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
    fn reifying_triple(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::TripleOpen)?;
        let s = self.quoted_component(graph)?;
        let p = self.predicate()?;
        let o = self.quoted_component(graph)?;
        let reifier = if self.eat(&Token::Tilde) {
            if self.at_reifier_id() {
                self.term(graph)?
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

    fn blank_node_property_list(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        // `[]` lexes as a single `Anon`; `[ … ]` opens with `LBracket`.
        if self.eat(&Token::Anon) {
            return Ok(self.next_bnode());
        }
        self.expect(&Token::LBracket)?;
        let subject = self.next_bnode();
        if !self.eat(&Token::RBracket) {
            self.predicate_object_list(&subject, graph)?;
            self.expect(&Token::RBracket)?;
        }
        Ok(subject)
    }

    fn collection(&mut self, graph: Option<&Node>) -> Result<Node, RdfDiagnostic> {
        self.expect(&Token::LParen)?;
        let mut items = Vec::new();
        while !self.eat(&Token::RParen) {
            if self.peek().is_none() {
                let (l, c) = self.loc();
                return Err(err_at("unterminated RDF collection", l, c));
            }
            items.push(self.term(graph)?);
        }
        if items.is_empty() {
            return Ok(Node::Iri(RDF_NIL.to_owned()));
        }
        let cells: Vec<Node> = (0..items.len()).map(|_| self.next_bnode()).collect();
        for (index, item) in items.into_iter().enumerate() {
            let current = cells[index].clone();
            let rest = if index + 1 == cells.len() {
                Node::Iri(RDF_NIL.to_owned())
            } else {
                cells[index + 1].clone()
            };
            self.emit(&current, &Node::Iri(RDF_FIRST.to_owned()), &item, graph);
            self.emit(&current, &Node::Iri(RDF_REST.to_owned()), &rest, graph);
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
                let (base, dir) = split_lang_direction(&raw, l, c)?;
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
            value,
            lang,
            direction,
            datatype,
        })
    }

    fn datatype_iri(&mut self) -> Result<String, RdfDiagnostic> {
        let (l, c) = self.loc();
        match self.bump() {
            Some(Token::Iri(raw)) => Ok(self.resolve_iri(&raw)),
            Some(Token::PrefixedName(prefix, local)) => {
                match self.resolve_prefixed(&prefix, &local, l, c)? {
                    Node::Iri(iri) => Ok(iri),
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

    fn graph_block(&mut self, graph: &Node) -> Result<(), RdfDiagnostic> {
        if !matches!(graph, Node::Iri(_) | Node::Bnode(_)) {
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
            let subject = self.term(Some(graph))?;
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
            self.predicate_object_list(subject, graph)?;
        }
        self.expect(&Token::Dot)
    }

    fn statement_after_subject_in_graph(
        &mut self,
        subject: &Node,
        graph: &Node,
    ) -> Result<(), RdfDiagnostic> {
        if !(self.at(&Token::Dot) || self.at(&Token::RBrace)) {
            self.predicate_object_list(subject, Some(graph))?;
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

    fn predicate_object_list(
        &mut self,
        subject: &Node,
        graph: Option<&Node>,
    ) -> Result<(), RdfDiagnostic> {
        loop {
            let predicate = self.predicate()?;
            loop {
                // The object of `rdf:reifies` is a triple TERM. Parse `<<` here as a
                // triple term whether or not it carries parens, tolerating purrdf's
                // legacy non-parenthesized `<< s p o >>` triple-term serialization in
                // addition to the canonical `<<( s p o )>>` — in EVERY other position a
                // bare `<< … >>` keeps its W3C reifying-triple meaning (`reifying_triple`).
                let object = if matches!(&predicate, Node::Iri(p) if p == RDF_REIFIES)
                    && self.at(&Token::TripleOpen)
                {
                    self.reifies_object_triple_term(graph)?
                } else {
                    self.term(graph)?
                };
                self.emit(subject, &predicate, &object, graph);
                self.maybe_reify_and_annotate(subject, &predicate, &object, graph)?;
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
    ) -> Result<(), RdfDiagnostic> {
        let mut pending: Option<Node> = None;
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = if self.at_reifier_id() {
                    self.term(graph)?
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
                self.predicate_object_list(&reifier, graph)?;
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
            &Node::Iri(RDF_REIFIES.to_owned()),
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
        let mut nodes = vec![subject.clone(), predicate.clone(), object.clone()];
        if let Some(graph) = graph {
            nodes.push(graph.clone());
        }
        // Record the subject's source position when tracking is on. `S::ENABLED` is a
        // const, so for `NoSpans` this block (and the lazy `LineIndex`) is dead code.
        if S::ENABLED {
            if let Some(key) = subject_key(&nodes[0]) {
                let src = self.src;
                let index = self
                    .line_index
                    .get_or_insert_with(|| purrdf_iri::LineIndex::new(src));
                let position = index.locate(src, self.subject_off);
                self.collector.record(&key, position);
            }
        }
        self.statements.push(nodes);
    }

    fn next_bnode(&mut self) -> Node {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        Node::Bnode(deterministic_label(id))
    }

    fn resolve_iri(&self, raw: &str) -> String {
        if has_iri_scheme(raw) {
            raw.to_owned()
        } else if let Some(base) = &self.base_iri {
            resolve_relative_iri(base, raw)
        } else {
            raw.to_owned()
        }
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
            Some(base) => Ok(Node::Iri(format!("{base}{local}"))),
            None => Err(err_at(format!("unknown prefix {prefix:?}"), line, col)),
        }
    }

    // token cursor helpers

    /// Resolve the current token's document-global 1-based `(line, column)` by
    /// scanning the full source with the shared [`purrdf_iri::LineIndex`]. Built
    /// lazily on the error path only (the tokens carry document-global byte spans),
    /// so the happy path never pays for it.
    fn loc(&self) -> (u32, u32) {
        let off = self
            .tokens
            .get(self.pos)
            .map(|s| s.start)
            .or_else(|| self.tokens.last().map(|s| s.end))
            .unwrap_or(0);
        let p = purrdf_iri::LineIndex::new(self.src).locate(self.src, off);
        (p.line, p.column)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    /// Consume the current token, MOVING it out of the owned buffer (a cheap
    /// `Token::Dot` placeholder is left behind; nothing re-reads a consumed
    /// position — `peek`/`peek2` look only at `pos` and beyond, which advance
    /// monotonically).
    fn bump(&mut self) -> Option<Token> {
        let t = self
            .tokens
            .get_mut(self.pos)
            .map(|s| std::mem::replace(&mut s.token, Token::Dot));
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn at(&self, token: &Token) -> bool {
        self.peek() == Some(token)
    }

    fn eat(&mut self, token: &Token) -> bool {
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

    fn expect(&mut self, token: &Token) -> Result<(), RdfDiagnostic> {
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
// Relative-IRI resolution (mirrors the prior from_trig `resolve_relative_iri`)
// ───────────────────────────────────────────────────────────────────────────────

fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let keep_trailing_slash = path.ends_with('/')
        || path.ends_with("/.")
        || path.ends_with("/..")
        || path == "."
        || path == "..";
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }
    let mut normalized = String::new();
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&segments.join("/"));
    if keep_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    if normalized.is_empty() && absolute {
        normalized.push('/');
    }
    normalized
}

fn split_raw_path_suffix(raw: &str) -> (&str, &str) {
    let split = raw.find(['?', '#']).unwrap_or(raw.len());
    (&raw[..split], &raw[split..])
}

fn split_base_for_path(base: &str) -> (String, &str) {
    let Some(scheme_end) = base.find(':') else {
        return (String::new(), base);
    };
    let scheme_prefix = &base[..=scheme_end];
    let rest = &base[scheme_end + 1..];
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let authority_end = after_slashes.find('/').unwrap_or(after_slashes.len());
        let authority = &after_slashes[..authority_end];
        let path = &after_slashes[authority_end..];
        (format!("{scheme_prefix}//{authority}"), path)
    } else {
        (scheme_prefix.to_string(), rest)
    }
}

fn resolve_relative_iri(base: &str, raw: &str) -> String {
    if has_iri_scheme(raw) {
        return raw.to_string();
    }
    let base_without_fragment = base.split_once('#').map_or(base, |(before, _)| before);
    if raw.is_empty() {
        return base_without_fragment.to_string();
    }
    if raw.starts_with('#') {
        return format!("{base_without_fragment}{raw}");
    }
    let base_without_query = base_without_fragment
        .split_once('?')
        .map_or(base_without_fragment, |(before, _)| before);
    if raw.starts_with('?') {
        return format!("{base_without_query}{raw}");
    }
    if raw.starts_with("//") {
        if let Some(scheme_end) = base.find(':') {
            return format!("{}:{raw}", &base[..scheme_end]);
        }
        return raw.to_string();
    }
    let (prefix, base_path) = split_base_for_path(base_without_query);
    let (raw_path, suffix) = split_raw_path_suffix(raw);
    let merged_path = if raw_path.starts_with('/') {
        raw_path.to_string()
    } else {
        let base_dir = if base_path.is_empty() {
            "/"
        } else {
            base_path
                .rfind('/')
                .map_or("", |index| &base_path[..=index])
        };
        format!("{base_dir}{raw_path}")
    };
    format!("{prefix}{}{}", remove_dot_segments(&merged_path), suffix)
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
            Node::Iri(value) => self.intern_atom(SerTermKind::Iri, value, None, None, None),
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

/// Lower the flat statement list into the in-memory [`SerGraph`], reproducing
/// `from_nquads`'s `build_gts` (the `rdf:reifies` statement-layer shorthand,
/// first-seen interning, statement-order quads, encounter-order reifiers).
fn build_gts_graph(statements: &[Statement]) -> Result<SerGraph, RdfDiagnostic> {
    let mut interner = Interner::new();
    let mut reifiers: Vec<(usize, SerTriple3)> = Vec::new();
    let mut quads: Vec<(usize, usize, usize, Option<usize>)> = Vec::new();

    for nodes in statements {
        let s = &nodes[0];
        let p = &nodes[1];
        let o = &nodes[2];
        let gname = nodes.get(3);

        // `<subject> rdf:reifies <<( s p o )>> .` in the DEFAULT graph is the
        // statement-layer reifier shorthand: bind the reifier, do NOT emit a base quad.
        if let (
            Node::Iri(_) | Node::Bnode(_),
            Node::Iri(pred_iri),
            Node::Triple(ts, tp, to),
            None,
        ) = (s, p, o, gname)
        {
            if pred_iri == RDF_REIFIES {
                let rid = interner.atom(s);
                let ss = interner.node(ts, &mut reifiers);
                let pp = interner.node(tp, &mut reifiers);
                let oo = interner.node(to, &mut reifiers);
                set_reifier(&mut reifiers, rid, (ss, pp, oo))?;
                continue;
            }
        }

        let sid = interner.node(s, &mut reifiers);
        let pid = interner.node(p, &mut reifiers);
        let oid = interner.node(o, &mut reifiers);
        let gid = gname.map(|node| interner.node(node, &mut reifiers));
        quads.push((sid, pid, oid, gid));
    }

    Ok(SerGraph {
        terms: interner.terms,
        quads,
        // The reifier row carries an optional graph slot; this first-party text parser
        // binds reifiers only in the DEFAULT graph (the `rdf:reifies` shorthand is gated
        // on `None` graph above), so the slot is always `None`. Annotations are left in
        // `quads` here and reclassified by `fold_statement_layer`'s pass 2 (the
        // `annotations` table stays empty).
        reifiers: reifiers
            .into_iter()
            .map(|(rid, spo)| (rid, spo, None))
            .collect(),
        ..Default::default()
    })
}

/// Bind a reifier, hard-failing on a conflicting rebinding (CONSTITUTION P7: never
/// silently last-write-win), idempotent on an identical rebind. Mirrors
/// `from_nquads`'s `set_reifier`.
fn set_reifier(
    reifiers: &mut Vec<(usize, SerTriple3)>,
    rid: usize,
    spo: SerTriple3,
) -> Result<(), RdfDiagnostic> {
    if let Some((_, existing)) = reifiers.iter().find(|(r, _)| *r == rid) {
        if *existing != spo {
            return Err(err(format!(
                "conflicting rdf:reifies binding for reifier term {rid}"
            )));
        }
    } else {
        reifiers.push((rid, spo));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let seq = parse_lines_sequential(&text, true, 1, &mut NoSpans).expect("sequential parse");
        let par =
            parse_lines(&text, true, LineParseMode::Auto, &mut NoSpans).expect("parallel parse");
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
        let expected = parse_lines_sequential(text, true, 1, &mut NoSpans).expect("sequential");
        for chunk_bytes in [1usize, 7, 16, 64, 4096] {
            let actual = parse_lines_parallel_with_chunk_size(text, true, chunk_bytes)
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
        assert!(split_line_chunks("", 8).is_empty());
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

        let seq_err =
            parse_lines_sequential(&text, true, 1, &mut NoSpans).expect_err("sequential must fail");
        // A tiny chunk target guarantees the two bad lines land in different chunks.
        let par_err =
            parse_lines_parallel_with_chunk_size(&text, true, 256).expect_err("parallel must fail");
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

        let seq_err =
            parse_lines_sequential(&text, true, 1, &mut NoSpans).expect_err("sequential must fail");
        let par_err = parse_lines(&text, true, LineParseMode::Auto, &mut NoSpans)
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
        let par_err = parse_lines(&text, false, LineParseMode::Auto, &mut NoSpans)
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
        let seq_err = parse_lines_sequential(&text, false, 1, &mut NoSpans)
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
    #[test]
    fn nquads_error_carries_line_and_column() {
        // The third line has a blank-node predicate, which is invalid.
        let text = "<http://ex/s> <http://ex/p> <http://ex/o> .\n\
                    <http://ex/s> <http://ex/p> <http://ex/o> .\n\
                    <http://ex/s> _:bad <http://ex/o> .\n";
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
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
        let mut cursor = TokenCursor::new(tokens, raw, 7);
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
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(29));
        assert!(e.message.contains("IRI must be absolute"));
    }

    /// A bad literal base direction reports the column of the language tag, not the
    /// following token.
    #[test]
    fn langtag_direction_column_points_at_langtag() {
        // The `@en--bad` tag begins at column 32.
        let text = "<http://ex/s> <http://ex/p> \"x\"@en--bad .\n";
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
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
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
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
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
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
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(34));
        assert!(e.message.contains("IRI must be absolute"));
    }

    /// An explicit `rdf:langString` datatype after `^^` reports the column of the
    /// datatype IRI, not the token after it.
    #[test]
    fn datatype_rdf_lang_string_column_points_at_datatype() {
        // The datatype IRI begins at column 34.
        let text = "<http://ex/s> <http://ex/p> \"x\"^^\
                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .\n";
        let e = parse_lines_sequential(text, false, 1, &mut NoSpans).expect_err("must fail");
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
        let e = DocParser::new(text, None, false, &mut NoSpans)
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
        let e = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(7));
        assert!(e.message.contains("base IRI must be absolute"));
    }

    /// A malformed language tag on a Turtle literal (DocParser path) reports the
    /// column of the language tag, not the following token.
    #[test]
    fn doc_langtag_column_points_at_langtag() {
        // The `@bad--bad` tag begins at column 32.
        let text = "<http://ex/s> <http://ex/p> \"x\"@bad--bad .\n";
        let e = DocParser::new(text, None, false, &mut NoSpans)
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
        let e = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect_err("must fail");
        let loc = e.location.as_ref().expect("has location");
        assert_eq!(loc.line, Some(1));
        assert_eq!(loc.column, Some(29));
        assert!(e.message.contains("unknown prefix"));
    }

    /// A bare `/` in a prefixed-name local part (e.g. `ex:report/shacl/sarif`)
    /// must parse as ONE prefixed name and expand to the prefix namespace plus the
    /// slash-bearing local, matching oxigraph/purrdf-gts (strict Turtle would need
    /// `\/`, but real-world ontologies and fixtures use the bare form).
    #[test]
    fn turtle_prefixed_name_allows_bare_slash_in_local() {
        let text = "@prefix ex: <https://example.org/vocab/> .\n\
                    ex:report/shacl/sarif ex:projection/okf ex:report/shacl/sarif .";
        let statements = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 1);
        let nodes = &statements[0];
        assert_eq!(
            nodes[0],
            Node::Iri("https://example.org/vocab/report/shacl/sarif".to_owned())
        );
        assert_eq!(
            nodes[1],
            Node::Iri("https://example.org/vocab/projection/okf".to_owned())
        );
        assert_eq!(
            nodes[2],
            Node::Iri("https://example.org/vocab/report/shacl/sarif".to_owned())
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
        let statements = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2, "must yield exactly two triples");

        let first = &statements[0];
        assert_eq!(first[0], Node::Iri("https://example.org/x".to_owned()));
        assert_eq!(first[1], Node::Iri("https://example.org/p".to_owned()));
        let Node::Bnode(label_as_object) = &first[2] else {
            panic!("expected blank-node object, got {:?}", first[2]);
        };

        let second = &statements[1];
        let Node::Bnode(label_as_subject) = &second[0] else {
            panic!("expected blank-node subject, got {:?}", second[0]);
        };
        assert_eq!(second[1], Node::Iri("https://example.org/q".to_owned()));
        assert_eq!(second[2], Node::Iri("https://example.org/z".to_owned()));

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
        let statements = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2);
    }

    /// A longer run of consecutive `;` (three in a row) collapses the same way: each
    /// extra `;` past the first is just another empty item, never an extra triple.
    #[test]
    fn turtle_semicolon_run_of_three_emits_no_extra_triples() {
        let text =
            "<https://example.org/s> <https://example.org/p1> <https://example.org/o1> ; ; ; \
                     <https://example.org/p2> <https://example.org/o2> .";
        let statements = DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .expect("parses");
        assert_eq!(statements.len(), 2);
    }

    /// A trailing `; ;` before the terminating `.` is the doubled/trailing empty-item
    /// form: it must not require (or produce) a following predicate-object pair.
    #[test]
    fn turtle_trailing_doubled_semicolon_emits_no_extra_triple() {
        let text = "<https://example.org/s> <https://example.org/p> <https://example.org/o> ; ; .";
        let statements = DocParser::new(text, None, false, &mut NoSpans)
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
        let expected = DocParser::new(collapsed, None, false, &mut NoSpans)
            .parse()
            .expect("collapsed parses");
        let actual = DocParser::new(doubled, None, false, &mut NoSpans)
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
        let expected = DocParser::new(collapsed, None, false, &mut NoSpans)
            .parse()
            .expect("collapsed parses");
        let actual = DocParser::new(doubled, None, false, &mut NoSpans)
            .parse()
            .expect("doubled parses");
        assert_eq!(actual, expected);
    }

    /// A trailing `;` inside an annotation block (`{| … ; |}`) is the empty-item form
    /// terminated by `Pipe` rather than `Dot`/`RBracket`/`RBrace` — this specifically
    /// exercises the `Pipe` branch of the terminator check after the `;` run is drained.
    #[test]
    fn turtle_trailing_semicolon_inside_annotation_block_before_pipe() {
        let no_trailing =
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                            {| <https://example.org/a> <https://example.org/b> |} .";
        let trailing = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                         {| <https://example.org/a> <https://example.org/b> ; |} .";
        let expected = DocParser::new(no_trailing, None, false, &mut NoSpans)
            .parse()
            .expect("no-trailing parses");
        let actual = DocParser::new(trailing, None, false, &mut NoSpans)
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
        assert!(DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .is_err());
    }

    /// A subject followed immediately by `;` and then `.` (no predicate-object pair at
    /// all) is also illegal for the same reason: `predicate()` has nothing to consume.
    #[test]
    fn turtle_leading_semicolon_with_no_predicate_object_is_an_error() {
        let text = "<https://example.org/s> ; .";
        assert!(DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .is_err());
    }

    /// A LEADING `;` inside a blank-node property list `[ … ]` is also illegal:
    /// `predicate()` runs at the top of the `predicateObjectList` loop before any `;`
    /// handling, so a `;` with no preceding predicate-object pair inside the blank
    /// node has no predicate to parse and must error.
    #[test]
    fn turtle_leading_semicolon_inside_blank_node_property_list_is_an_error() {
        let text = "<https://example.org/s> <https://example.org/p> \
                     [ ; <https://example.org/a> <https://example.org/b> ] .";
        assert!(DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .is_err());
    }

    /// A LEADING `;` inside an RDF 1.2 annotation block `{| … |}` is also illegal for
    /// the same reason: `predicate()` has nothing to consume before the first `;`.
    #[test]
    fn turtle_leading_semicolon_inside_annotation_block_is_an_error() {
        let text = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                     {| ; <https://example.org/a> <https://example.org/b> |} .";
        assert!(DocParser::new(text, None, false, &mut NoSpans)
            .parse()
            .is_err());
    }

    /// A DOUBLED trailing `;` before the annotation-block `Pipe` (`{| a b ; ; |}`)
    /// must drain the whole run of semicolons and then break on `Pipe`, parsing
    /// IDENTICALLY to the no-trailing form — this pairs the `while self.eat(&Token::Semicolon) {}`
    /// drain with the `Pipe` terminator, distinct from the single-`;` trailing case.
    #[test]
    fn turtle_doubled_trailing_semicolon_inside_annotation_block_before_pipe() {
        let no_trailing =
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                            {| <https://example.org/a> <https://example.org/b> |} .";
        let doubled = "<https://example.org/s> <https://example.org/p> <https://example.org/o> \
                        {| <https://example.org/a> <https://example.org/b> ; ; |} .";
        let expected = DocParser::new(no_trailing, None, false, &mut NoSpans)
            .parse()
            .expect("no-trailing parses");
        let actual = DocParser::new(doubled, None, false, &mut NoSpans)
            .parse()
            .expect("doubled parses");
        assert_eq!(actual, expected);
    }
}

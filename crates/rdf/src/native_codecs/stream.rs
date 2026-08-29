// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing RDF from a `Read` WITHOUT buffering the source document.
//!
//! Every other parse entry point in this crate takes `&[u8]`: the caller must already
//! hold the whole document. [`parse_dataset_from_reader`] does not. For a LINE-ORIENTED
//! syntax (N-Triples, N-Quads, HexTuples — see
//! [`NativeRdfFormat::is_line_oriented`]) it pulls one physical line at a time out of
//! the reader, parses it, folds it into the accumulating graph, and drops it before
//! reading the next. The source text is never resident.
//!
//! # What this does and does not bound
//!
//! Be precise about the claim. Streaming removes the two SOURCE-SIZED buffers:
//!
//! * the document bytes (the `Vec<u8>` a caller used to `read_to_end` into), and
//! * the intermediate `Vec<Statement>` the buffered line pipeline builds before it
//!   lowers (each statement owning its terms' `String`s, so it is typically larger
//!   than the source text it came from).
//!
//! It does NOT make the parse constant-memory, and nothing here should be read as
//! claiming that. The product is a frozen [`RdfDataset`], which is proportional to the
//! document's content by definition, and the RDF 1.2 statement-layer fold
//! ([`fold_statement_layer`](super::parse::fold_statement_layer)) is genuinely
//! two-pass: whether `<r> <p> <v>` is a base quad or an annotation depends on whether
//! some — possibly LATER — line binds `r` with `rdf:reifies`. A row therefore cannot be
//! classified when it is read, so the row table is resident until the document ends.
//! That is a property of the format, not a shortcut taken here.
//!
//! The honest statement of the bound is: **peak residency drops from
//! `source_text + statements + graph + dataset` to `one line + graph + dataset`.**
//!
//! # Non-line-oriented formats
//!
//! Turtle and TriG are NOT line-oriented and are not streamed: `@prefix` / `@base`
//! rebind mid-document and anonymous blank nodes mint labels from a document-ordered
//! counter, so a line has no meaning independent of every line before it. RDF/XML,
//! TriX, JSON-LD and YAML-LD are tree syntaxes with no line structure at all.
//! [`parse_dataset_from_reader`] accepts all of them for interface uniformity, and
//! reads them to a buffer first — the same buffer the caller would otherwise have
//! built. This is stated rather than papered over: those formats gain nothing here.
//!
//! # Equivalence with the buffered paths
//!
//! The streaming pipeline is not a second parser. It calls the SAME per-line grammar
//! function the buffered sequential path calls (and therefore the same one the
//! chunk-parallel path's per-chunk workers call), and hands each statement to the SAME
//! lowering, one statement at a time instead of in a slice. So for one input the three
//! paths — chunk-parallel, buffered-sequential, streaming — produce the identical
//! frozen dataset by construction. [`tests`] proves it anyway, over a corpus with
//! blank nodes and reifiers that forward-reference across every chunk and buffer
//! boundary, multi-byte UTF-8 straddling the read buffer, CRLF endings, and a final
//! line with no terminator.

use std::io::{BufRead, BufReader, Read};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::str::Utf8Error;
use std::sync::Arc;

use super::hextuples::HexTuplesStreamParser;
use super::media_type::{NativeRdfFormat, classify};
use super::text_parse::LineStreamParser;
use crate::{RdfDataset, RdfDiagnostic};

/// The read buffer the line reader pulls through. Sized to amortize syscalls without
/// making the "bounded" claim depend on a large constant: a 64 KiB window is the same
/// order as `BufReader`'s own default and is dwarfed by any document worth streaming.
const READ_BUFFER_BYTES: usize = 64 << 10;

/// Parse an RDF document out of a `Read` into a frozen [`RdfDataset`].
///
/// For a line-oriented syntax the reader is consumed INCREMENTALLY — the source text is
/// never held — and for every other syntax it is read to a buffer and handed to
/// [`parse_dataset`](super::parse_dataset), which is what that grammar requires. Either
/// way the resulting dataset is byte-for-byte what [`parse_dataset`](super::parse_dataset)
/// returns for the same bytes; see the module documentation for the equivalence
/// argument and for the exact memory bound this does and does not achieve.
///
/// `base_iri` behaves exactly as it does on the buffered path: N-Triples / N-Quads /
/// HexTuples require absolute IRIs and ignore it (N/A by syntax), the others resolve
/// against it.
///
/// # Errors
///
/// * `native-codec-unsupported-format` — `media_type` names no known syntax.
/// * `native-codec-utf8` — the document is not valid UTF-8. The reported byte index is
///   document-global, as on the buffered path.
/// * `native-codec-read` — the underlying reader failed (including a transport decoder
///   reporting a truncated or corrupt frame mid-stream).
/// * `native-codec-parse` — a line is not well-formed, reported with its
///   document-global line and column.
/// * `native-codec-panic` — a codec unwound; converted, never propagated.
///
/// A streaming parse reports the FIRST failure in document order. The buffered path
/// validates the whole document's UTF-8 before parsing any of it, so on input that is
/// BOTH malformed UTF-8 late and syntactically invalid early, the two report different
/// (both correct) failures. Reproducing the buffered precedence would require reading
/// to the end first, which is the thing streaming exists to avoid.
pub fn parse_dataset_from_reader<R: Read>(
    reader: R,
    media_type: &str,
    base_iri: Option<&str>,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let format = classify(media_type)?;
    if !format.is_line_oriented() {
        // Not streamable by its grammar: read it whole, exactly as the caller would
        // have, and take the ordinary buffered path.
        let mut bytes = Vec::new();
        let mut reader = reader;
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| read_error(&error))?;
        return super::parse_dataset(&bytes, media_type, base_iri);
    }
    // Panic-guarded like every other codec entry point, so a codec unwind becomes a
    // structured diagnostic rather than tearing down the caller.
    catch_unwind(AssertUnwindSafe(|| stream_line_format(reader, format))).unwrap_or_else(
        |payload| {
            Err(RdfDiagnostic::error(
                "native-codec-panic",
                format!(
                    "native RDF text parser panicked while streaming {}: {}",
                    format.media_type(),
                    super::parse::panic_payload_message(payload.as_ref()),
                ),
            ))
        },
    )
}

/// Drive one line-oriented format's streaming parser to exhaustion.
fn stream_line_format<R: Read>(
    reader: R,
    format: NativeRdfFormat,
) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut lines = LineReader::new(reader);
    match format {
        NativeRdfFormat::HexTuples => {
            let mut parser = HexTuplesStreamParser::new();
            while let Some(line) = lines.next_line()? {
                parser.push_line(line)?;
            }
            parser.finish()
        }
        // N-Triples / N-Quads: `LineStreamParser::new` re-checks the format and rejects
        // anything else, so the line family cannot be entered by the wrong door.
        other => {
            let mut parser = LineStreamParser::new(other)?;
            while let Some(line) = lines.next_line()? {
                parser.push_line(line)?;
            }
            super::parse::dataset_from_text_ser_graph(&parser.finish())
        }
    }
}

/// A reader that yields logical lines EXACTLY as `str::lines` would, one at a time,
/// reusing a single buffer.
///
/// Matching `str::lines` is the whole point, because the buffered parser is defined in
/// terms of it. That means, precisely:
///
/// * lines are split at `\n`, and a `\r` immediately before that `\n` is part of the
///   terminator (CRLF);
/// * a trailing `\r` on the FINAL line, when the document does not end in `\n`, is
///   NOT a terminator and stays in the line — `str::lines` keeps it, so this keeps it;
/// * a document ending in `\n` yields no extra empty line, and an empty document
///   yields no lines at all.
///
/// A multi-byte UTF-8 sequence straddling a read-buffer boundary is a non-issue by
/// construction: `read_until` accumulates until it sees `\n`, and `\n` cannot occur
/// inside a UTF-8 sequence, so a line is always assembled whole before it is validated.
struct LineReader<R> {
    inner: BufReader<R>,
    /// The current line's raw bytes INCLUDING its terminator, reused across lines.
    raw: Vec<u8>,
    /// Document-global byte offset of the current line's first byte, so a UTF-8
    /// diagnostic names the same index the buffered whole-document validation would.
    offset: usize,
}

impl<R: Read> LineReader<R> {
    fn new(reader: R) -> Self {
        Self {
            inner: BufReader::with_capacity(READ_BUFFER_BYTES, reader),
            raw: Vec::new(),
            offset: 0,
        }
    }

    /// The next logical line without its terminator, or `None` at end of input.
    fn next_line(&mut self) -> Result<Option<&str>, RdfDiagnostic> {
        self.offset += self.raw.len();
        self.raw.clear();
        if self
            .inner
            .read_until(b'\n', &mut self.raw)
            .map_err(|error| read_error(&error))?
            == 0
        {
            return Ok(None);
        }
        // Validate the raw bytes INCLUDING the terminator, which is what the buffered
        // path's whole-document `from_utf8` sees at this position — so a truncated
        // sequence followed by a newline is classified identically (`error_len` is
        // `Some`, not "incomplete"), and the diagnostic matches.
        let raw = std::str::from_utf8(&self.raw).map_err(|e| utf8_error(&e, self.offset))?;
        let line = match raw.strip_suffix('\n') {
            Some(body) => body.strip_suffix('\r').unwrap_or(body),
            // No `\n`: this is the final line of a document with no trailing newline,
            // and a `\r` at its end is content, exactly as `str::lines` treats it.
            None => raw,
        };
        Ok(Some(line))
    }
}

/// A reader failure, as a structured diagnostic.
///
/// This is where a transport decoder's mid-stream "truncated frame" surfaces: it is an
/// ERROR that fails the whole parse, never a short read that would hand a downstream
/// consumer a silently-shortened document.
fn read_error(error: &std::io::Error) -> RdfDiagnostic {
    RdfDiagnostic::error(
        "native-codec-read",
        format!("failed to read the RDF source: {error}"),
    )
}

/// A UTF-8 failure rebased onto the whole document.
///
/// The message reproduces `Utf8Error`'s own `Display` with the index shifted from
/// line-local to document-global, so a streamed parse and a buffered parse of the same
/// bytes report the SAME diagnostic — code and text — for a document that is not UTF-8.
fn utf8_error(error: &Utf8Error, line_offset: usize) -> RdfDiagnostic {
    let index = line_offset + error.valid_up_to();
    let detail = match error.error_len() {
        Some(len) => format!("invalid utf-8 sequence of {len} bytes from index {index}"),
        None => format!("incomplete utf-8 byte sequence from index {index}"),
    };
    RdfDiagnostic::error("native-codec-utf8", detail)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::io::Cursor;

    use super::*;
    use crate::SerializeGraph;
    use crate::native_codecs::{parse_dataset, parse_dataset_forced_sequential, serialize_dataset};

    /// A reader that hands back at most `chunk` bytes per `read`, so every buffer
    /// boundary in the test lands where the test chooses — including in the middle of a
    /// multi-byte UTF-8 sequence and in the middle of a line.
    struct DribbleReader<'a> {
        data: &'a [u8],
        chunk: usize,
    }

    impl Read for DribbleReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let take = self.chunk.min(buf.len()).min(self.data.len());
            buf[..take].copy_from_slice(&self.data[..take]);
            self.data = &self.data[take..];
            Ok(take)
        }
    }

    /// The canonical N-Quads bytes of a dataset — the byte-identity yardstick.
    fn canonical(dataset: &RdfDataset) -> Vec<u8> {
        serialize_dataset(dataset, "application/n-quads", SerializeGraph::Dataset)
            .expect("serialize")
    }

    /// Assert chunk-parallel == buffered-sequential == streaming, for every read
    /// granularity that could split the input differently.
    fn assert_three_way_equivalence(text: &str, media_type: &str) {
        let parallel = parse_dataset(text.as_bytes(), media_type, None).expect("auto/parallel");
        let sequential = parse_dataset_forced_sequential(text.as_bytes(), media_type, None)
            .expect("forced sequential");
        assert_eq!(
            canonical(&parallel),
            canonical(&sequential),
            "parallel and sequential must already agree"
        );
        // 1 byte at a time is the adversarial case: every line, and every multi-byte
        // character, is split across reads.
        for chunk in [1usize, 3, 7, 64, 4096, usize::MAX] {
            let streamed = parse_dataset_from_reader(
                DribbleReader {
                    data: text.as_bytes(),
                    chunk,
                },
                media_type,
                None,
            )
            .unwrap_or_else(|e| panic!("streaming parse at chunk {chunk}: {e}"));
            assert_eq!(
                canonical(&streamed),
                canonical(&sequential),
                "streamed dataset must be identical at read granularity {chunk}"
            );
            assert_eq!(
                streamed.term_count(),
                sequential.term_count(),
                "term table must be identical at read granularity {chunk}"
            );
            assert!(
                streamed.quads().collect::<Vec<_>>() == sequential.quads().collect::<Vec<_>>(),
                "frozen quad rows (term ids AND order) must be identical at granularity {chunk}"
            );
        }
    }

    /// A non-trivial N-Quads corpus: blank nodes and reifiers that forward-reference
    /// across every plausible chunk boundary, multi-byte UTF-8, every literal shape,
    /// quoted-triple terms, comments and blank lines.
    fn corpus(rows: usize) -> String {
        let mut out = String::with_capacity(rows * 200);
        out.push_str("# streaming equivalence corpus\n\n");
        for i in 0..rows {
            let (g, s, p) = (i % 7, i % 997, i % 13);
            match i % 7 {
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
                // Multi-byte UTF-8 in a language-tagged literal.
                2 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/label> \
                     \"\u{6f22}\u{5b57} \u{1f408} {i}\"@ja ."
                ),
                3 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/title> \
                     \"\u{645}\u{631}\u{62d}\u{628}\u{627} {i}\"@ar--rtl \
                     <https://example.org/g{g}> ."
                ),
                4 => writeln!(
                    out,
                    "<https://example.org/s{s}> <https://example.org/count> \
                     \"{i}\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
                ),
                // A reifier bound HERE and annotated far away (below), so the
                // statement-layer fold's two passes straddle chunk boundaries. The
                // reified triple is a function of the reifier label alone, so a label
                // that recurs re-binds it IDENTICALLY (`set_reifier` is idempotent on
                // an identical rebind and hard-fails a conflicting one).
                5 => {
                    let r = i % 211;
                    writeln!(
                        out,
                        "_:r{r} <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                         <<( <https://example.org/a{r}> <https://example.org/p{}> \
                         <https://example.org/c{r}> )>> .",
                        r % 13
                    )
                }
                _ => writeln!(
                    out,
                    "_:r{} <https://example.org/confidence> \"0.{}\" .",
                    // Annotates a reifier declared MANY lines earlier — often in a
                    // different chunk and certainly in a different read buffer.
                    (i + 100) % 211,
                    i % 100
                ),
            }
            .expect("write row");
        }
        out
    }

    #[test]
    fn streaming_matches_parallel_and_sequential_over_a_large_corpus() {
        // Large enough to cross `text_parse::PARALLEL_MIN_BYTES`, so the `parallel`
        // arm really is the chunk-parallel pipeline and this is a genuine three-way
        // comparison rather than sequential-vs-sequential.
        let text = corpus(14_000);
        assert!(
            text.len() >= 1 << 20,
            "corpus must cross the parallel threshold, got {} bytes",
            text.len()
        );
        assert_three_way_equivalence(&text, "application/n-quads");
    }

    #[test]
    fn streaming_matches_the_buffered_paths_on_edge_shaped_documents() {
        // A blank-node subject forward-referenced by a reifier declared later, CRLF
        // endings, a comment, a blank line, and a final line with NO trailing newline.
        let crlf = concat!(
            "# leading comment\r\n",
            "\r\n",
            "_:r1 <https://example.org/confidence> \"0.9\" .\r\n",
            "<https://example.org/s> <https://example.org/p> \"caf\u{e9} \u{1f408}\"@fr .\r\n",
            "_:r1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
            "<<( <https://example.org/a> <https://example.org/b> <https://example.org/c> )>> .",
        );
        assert!(!crlf.ends_with('\n'), "fixture must lack a final newline");
        assert_three_way_equivalence(crlf, "application/n-quads");

        // The same document LF-terminated, and the empty document.
        assert_three_way_equivalence(&crlf.replace("\r\n", "\n"), "application/n-quads");
        assert_three_way_equivalence("", "application/n-quads");
        assert_three_way_equivalence("\n\n\n", "application/n-quads");
        assert_three_way_equivalence(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .",
            "application/n-triples",
        );
    }

    #[test]
    fn a_line_longer_than_the_read_buffer_is_assembled_whole() {
        // The line reader must not be bounded by its read buffer: a single statement
        // larger than `READ_BUFFER_BYTES` has to survive.
        let long = "x".repeat(READ_BUFFER_BYTES * 3);
        let text = format!("<https://example.org/s> <https://example.org/p> \"{long}\" .\n");
        assert_three_way_equivalence(&text, "application/n-triples");
    }

    #[test]
    fn hextuples_streams_identically() {
        let mut text = String::new();
        for i in 0..500 {
            writeln!(
                text,
                "[\"https://example.org/s{}\",\"https://example.org/p\",\"\u{6f22} {i}\",\
                 \"http://www.w3.org/1999/02/22-rdf-syntax-ns#langString\",\"ja\",\"\"]",
                i % 37
            )
            .expect("write row");
        }
        let buffered =
            parse_dataset(text.as_bytes(), "application/x-hextuples", None).expect("buffered");
        for chunk in [1usize, 5, 4096] {
            let streamed = parse_dataset_from_reader(
                DribbleReader {
                    data: text.as_bytes(),
                    chunk,
                },
                "application/x-hextuples",
                None,
            )
            .expect("streamed");
            assert_eq!(canonical(&streamed), canonical(&buffered));
        }
    }

    #[test]
    fn non_line_oriented_formats_are_read_whole_and_parse_identically() {
        // Turtle / TriG / RDF-XML / JSON-LD are NOT streamed — stated, not hidden —
        // but the entry point still accepts them and must agree with `parse_dataset`.
        for (text, media_type) in [
            (
                "@prefix ex: <https://example.org/> .\nex:s ex:p \"o\" .\n",
                "text/turtle",
            ),
            (
                "@prefix ex: <https://example.org/> .\nGRAPH ex:g { ex:s ex:p \"o\" . }\n",
                "application/trig",
            ),
            (
                "{\"@context\":{},\"@graph\":[{\"@id\":\"https://example.org/s\",\
                 \"https://example.org/p\":{\"@value\":\"o\"}}]}",
                "application/ld+json",
            ),
        ] {
            let buffered = parse_dataset(text.as_bytes(), media_type, None).expect("buffered");
            let streamed = parse_dataset_from_reader(Cursor::new(text), media_type, None)
                .expect("read-whole path");
            assert_eq!(
                canonical(&streamed),
                canonical(&buffered),
                "{media_type} must parse identically through the reader entry point"
            );
        }
    }

    #[test]
    fn is_line_oriented_names_exactly_the_streamed_formats() {
        // The capability column and the streaming dispatch must not drift apart.
        let streamed: Vec<&str> = NativeRdfFormat::all()
            .filter(|f| f.is_line_oriented())
            .map(NativeRdfFormat::id)
            .collect();
        assert_eq!(streamed, vec!["ntriples", "nquads", "hextuples"]);
    }

    #[test]
    fn a_malformed_line_reports_the_buffered_paths_diagnostic() {
        // Same code, same document-global line, same message: the streamed parse runs
        // the same per-line grammar function.
        let text = concat!(
            "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
            "\n",
            "# comment\n",
            "<https://example.org/s> <https://example.org/p> .\n",
        );
        let buffered = parse_dataset(text.as_bytes(), "application/n-triples", None)
            .expect_err("buffered must reject");
        let streamed = parse_dataset_from_reader(Cursor::new(text), "application/n-triples", None)
            .expect_err("streaming must reject");
        assert_eq!(streamed.code, buffered.code);
        assert_eq!(streamed.message, buffered.message);
        assert_eq!(
            streamed.location.as_ref().and_then(|l| l.line),
            Some(4),
            "the document-global line number survives streaming"
        );
        assert_eq!(streamed.location, buffered.location);
    }

    #[test]
    fn invalid_utf8_reports_the_buffered_paths_diagnostic() {
        // The index is document-global and the wording is `Utf8Error`'s own, so the
        // streamed diagnostic is the buffered diagnostic verbatim.
        let mut bytes = b"<https://example.org/s> <https://example.org/p> \"a\" .\n".to_vec();
        let bad_line_offset = bytes.len();
        bytes.extend_from_slice(b"<https://example.org/s> <https://example.org/p> \"");
        let bad_index = bytes.len();
        bytes.extend_from_slice(&[0xff, 0xfe]);
        bytes.extend_from_slice(b"\" .\n");

        let buffered = parse_dataset(&bytes, "application/n-triples", None)
            .expect_err("buffered must reject invalid utf-8");
        let streamed =
            parse_dataset_from_reader(Cursor::new(bytes.clone()), "application/n-triples", None)
                .expect_err("streaming must reject invalid utf-8");
        assert_eq!(streamed.code, "native-codec-utf8");
        assert_eq!(streamed.code, buffered.code);
        assert_eq!(
            streamed.message, buffered.message,
            "the byte index must be document-global, not line-local"
        );
        assert!(
            streamed.message.contains(&bad_index.to_string()),
            "expected the document-global index {bad_index} in {:?} (line began at \
             {bad_line_offset})",
            streamed.message
        );
    }

    #[test]
    fn a_reader_failure_is_an_error_not_a_short_read() {
        // A source that fails mid-stream — what a truncated gzip frame looks like from
        // here — must fail the parse. Nothing partial escapes.
        struct FailsAfterOneLine {
            remaining: Vec<u8>,
        }

        impl Read for FailsAfterOneLine {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.remaining.is_empty() {
                    return Err(std::io::Error::other("simulated truncated stream"));
                }
                let take = self.remaining.len().min(buf.len());
                buf[..take].copy_from_slice(&self.remaining[..take]);
                self.remaining.drain(..take);
                Ok(take)
            }
        }

        let error = parse_dataset_from_reader(
            FailsAfterOneLine {
                remaining: b"<https://example.org/s> <https://example.org/p> \"o\" .\n".to_vec(),
            },
            "application/n-triples",
            None,
        )
        .expect_err("a mid-stream read failure must fail the parse");
        assert_eq!(error.code, "native-codec-read");
        assert!(error.message.contains("simulated truncated stream"));
    }

    #[test]
    fn an_unknown_media_type_fails_before_a_byte_is_read() {
        let error = parse_dataset_from_reader(Cursor::new("x"), "application/json", None)
            .expect_err("unknown media type must fail");
        assert_eq!(error.code, "native-codec-unsupported-format");
    }
}

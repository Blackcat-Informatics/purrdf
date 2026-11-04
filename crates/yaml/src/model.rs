// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::{Profile, YamlError, admit_count, profile::validate_iri};
use purrdf_core::{
    ContentDigest,
    cover::{VerbatimSpan, reconstruct_with_memory},
};
use purrdf_lex::{
    allocation::{Ceiling, Memory},
    yaml::{self, NodeKind, Span, SyntaxKeyPolicy, SyntaxStream},
};

/// Caller-owned original bytes under an absolute source IRI.
#[derive(Clone, Copy, Debug)]
pub struct SourceDocument<'a> {
    /// Absolute identity of the original byte source.
    pub id: &'a str,
    /// Original UTF-8, including BOM, comments, markers and line endings.
    pub bytes: &'a [u8],
}

/// One byte source owning its native lexical stream, without another syntax arena.
#[derive(Debug)]
pub struct Document<'a> {
    pub(crate) source: SourceDocument<'a>,
    pub(crate) stream: SyntaxStream<'a>,
    pub(crate) cover: Vec<Span>,
    pub(crate) digest: ContentDigest,
    pub(crate) id: String,
    pub(crate) profile: ContentDigest,
}
impl<'a> Document<'a> {
    /// Original source identity and bytes.
    pub const fn source(&self) -> SourceDocument<'a> {
        self.source
    }
    /// The one typed native stream, including all occurrence and trivia fields.
    pub const fn syntax(&self) -> &SyntaxStream<'a> {
        &self.stream
    }
    /// SHA-256 under the shared cover ContentDigest law.
    pub const fn source_digest(&self) -> ContentDigest {
        self.digest
    }
    /// Lex's separately named BLAKE3 identity of the original bytes.
    pub fn lex_source_id(&self) -> purrdf_hash::hex::Digest32 {
        self.stream.source_id()
    }
    /// Framed source/profile/content identity of this carrier root.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Separate complete byte partition; structural syntax spans may overlap it.
    pub fn cover_spans(&self) -> &[Span] {
        &self.cover
    }
}

/// Analyze presentation with deferred keys; no Core or JSON-LD semantics execute.
/// # Errors
/// Preserves positioned syntax errors, physical refusal and the shared cover law.
pub fn analyze<'a>(
    source: SourceDocument<'a>,
    profile: &Profile,
) -> Result<Document<'a>, YamlError> {
    let bounds = profile.bounds();
    admit_count(
        source.bytes.len(),
        bounds.syntax.syntax.max_input_bytes,
        "source bytes",
    )?;
    validate_iri(source.id)?;
    let text = core::str::from_utf8(source.bytes).map_err(|error| YamlError::Utf8 {
        at: error.valid_up_to(),
    })?;
    let stream = yaml::read_syntax_stream_with(text, bounds.syntax, SyntaxKeyPolicy::Deferred)?;
    let mut ceiling = Ceiling(bounds.max_partition_bytes);
    let mut memory = Memory::new(&mut ceiling);
    let mut raw = Vec::new();
    for (_, node) in stream.nodes() {
        let span = match &node.kind {
            NodeKind::Scalar(scalar) => scalar.raw,
            NodeKind::Alias { raw, .. } => *raw,
            _ => continue,
        };
        if span.start != span.end {
            admit_count(
                raw.len().saturating_add(1),
                bounds.max_cover_spans,
                "cover spans",
            )?;
            memory.push(&mut raw, span)?;
        }
    }
    raw.sort_unstable();
    let mut cover = Vec::new();
    let mut cursor = 0;
    for span in &raw {
        if cursor < span.start {
            append(
                &mut cover,
                Span {
                    start: cursor,
                    end: span.start,
                },
                bounds.max_cover_spans,
                &mut memory,
            )?;
        }
        append(&mut cover, *span, bounds.max_cover_spans, &mut memory)?;
        cursor = cursor.max(span.end);
    }
    if cursor < text.len() {
        append(
            &mut cover,
            Span {
                start: cursor,
                end: text.len(),
            },
            bounds.max_cover_spans,
            &mut memory,
        )?;
    }
    let digest = ContentDigest::of(source.bytes);
    let mut spans = Vec::new();
    for span in &cover {
        let text = stream.raw(*span).ok_or(YamlError::Metadata(
            "native raw span is outside source UTF-8",
        ))?;
        memory.push(
            &mut spans,
            VerbatimSpan {
                byte_start: span.start as u64,
                byte_end: span.end as u64,
                text,
                continues: None,
            },
        )?;
    }
    let verified =
        reconstruct_with_memory(source.bytes.len() as u64, &digest, &spans, &mut memory)?;
    memory.release_string(verified)?;
    memory.release_vec(spans)?;
    memory.release_vec(raw)?;
    let id = profile.stream_id(source.id, digest, &mut memory)?;
    Ok(Document {
        source,
        stream,
        cover,
        digest,
        id,
        profile: profile.identity(),
    })
}

fn append<S: purrdf_lex::allocation::Admission + ?Sized>(
    cover: &mut Vec<Span>,
    span: Span,
    maximum: usize,
    memory: &mut Memory<'_, S>,
) -> Result<(), YamlError> {
    admit_count(cover.len().saturating_add(1), maximum, "cover spans")?;
    memory.push(cover, span)?;
    Ok(())
}

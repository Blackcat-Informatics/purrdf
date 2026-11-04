// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Physical-line authoring interpretation, separate from the RDF dialect.
use super::{
    Block, BlockKind, Inline, InlineKind, LinkPart, Policy, SourcePosition, Span, SyntaxDocument,
    SyntaxError, SyntaxLimits, block,
    budget::Budget,
    inline::{self, Input},
    span,
};
use purrdf_core::hash::table::HashTable;
use std::borrow::Cow;

/// One original physical line and its admitted authoring views.
#[derive(Debug)]
pub struct AuthoringLine<'a> {
    /// Original payload, excluding its exact terminator.
    pub source: Span,
    /// Original LF, CRLF, trailing EOF CR, or empty EOF terminator.
    pub newline: Span,
    /// Fence delimiter or content, excluded from authoring interpretation.
    pub fenced: bool,
    /// Comment-free text; matched line-local code remains verbatim.
    pub visible: Cow<'a, str>,
    /// Prose override replacing each matched code span with one space.
    /// None means prose is identical to `visible`.
    pub prose: Option<Cow<'a, str>>,
}
impl AuthoringLine<'_> {
    /// Original physical prefixes and ordinary HTML remain in prose.
    pub fn prose(&self) -> &str {
        self.prose.as_deref().unwrap_or(&self.visible)
    }
}
/// ATX heading over the physical comment-free line, before code replacement.
#[derive(Debug)]
pub struct AuthoringHeading<'a> {
    /// Whole original physical payload.
    pub source: Span,
    /// Original title envelope; removed comments can leave holes inside it.
    pub title: Span,
    /// Heading level, one through six.
    pub level: u32,
    /// Interpreted title, retaining code spelling and character references.
    pub text: Cow<'a, str>,
}
/// Authoring link interpretation; reference uses are deliberately unresolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringLinkForm {
    /// Markdown inline link.
    Inline,
    /// Markdown image.
    Image,
    /// Destination on a reference definition, including duplicate definitions.
    ReferenceDefinition,
    /// Raw HTML href attribute.
    Href,
    /// Raw HTML src attribute.
    Src,
}
/// A source-exact authoring destination and its interpreted value.
#[derive(Debug)]
pub struct AuthoringLink<'a> {
    /// Whole original occurrence or attribute.
    pub source: Span,
    /// Authoring spelling.
    pub form: AuthoringLinkForm,
    /// Markdown punctuation escapes removed; HTML and entities remain literal.
    pub destination: LinkPart<'a>,
    /// Optional Markdown title with the same escape policy.
    pub title: Option<LinkPart<'a>>,
}
/// Comment extent, including an unfinished comment through original EOF.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoringComment {
    /// Original envelope, including excluded fenced lines inside an open comment.
    pub source: Span,
    /// An actual `-->` terminator was encountered outside fences.
    pub closed: bool,
}
/// Bounded physical-line interpretation. This type cannot enter RDF slicing.
#[derive(Debug)]
pub struct AuthoringDocument<'a> {
    syntax: SyntaxDocument<'a>,
    lines: Vec<AuthoringLine<'a>>,
    headings: Vec<AuthoringHeading<'a>>,
    comments: Vec<AuthoringComment>,
    links: Vec<AuthoringLink<'a>>,
    split_links: Vec<AuthoringLink<'a>>,
    /// Retained admitted storage, excluding the borrowed original source.
    pub auxiliary_bytes: usize,
    /// Deterministic construction work, including all scratch views.
    pub work: u64,
}
impl<'a> AuthoringDocument<'a> {
    /// Original source, without normalization.
    pub const fn source(&self) -> &'a str {
        self.syntax.source
    }
    /// Original source spelling of a model span.
    pub fn spelling(&self, source: Span) -> &'a str {
        self.syntax.spelling(source)
    }
    /// Physical lines in source order.
    pub fn lines(&self) -> &[AuthoringLine<'a>] {
        &self.lines
    }
    /// Physical ATX headings in source order; Setext is excluded.
    pub fn headings(&self) -> &[AuthoringHeading<'a>] {
        &self.headings
    }
    /// Complete and unfinished comments in source order.
    pub fn comments(&self) -> &[AuthoringComment] {
        &self.comments
    }
    /// Links, definition destinations and HTML attributes in source order.
    pub fn links(&self) -> &[AuthoringLink<'a>] {
        &self.links
    }
    /// Multiline inline links/images, excluded from ordinary link occurrences.
    pub fn split_links(&self) -> &[AuthoringLink<'a>] {
        &self.split_links
    }
    /// Matched line-local code spans in original coordinates.
    pub fn code_spans(&self) -> &[Inline] {
        &self.syntax.inlines
    }
    /// Shared raw fence/container arena; authoring heading interpretation is
    /// supplied separately by `headings`, never by this structural arena.
    pub fn blocks(&self) -> &[Block] {
        &self.syntax.blocks
    }
    /// Source position at an original UTF-8 boundary.
    pub fn position(&self, byte: usize) -> Option<SourcePosition> {
        self.syntax.position(byte)
    }
}
fn empty() -> Input {
    Input {
        text: String::new(),
        map: Vec::new(),
        ends: Vec::new(),
    }
}
fn append(
    out: &mut Input,
    text: &str,
    map: impl IntoIterator<Item = (usize, usize)>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    budget.memory(|m| m.push_str(&mut out.text, text))?;
    for (at, end) in map {
        budget.push(&mut out.map, at)?;
        budget.push(&mut out.ends, end)?;
    }
    Ok(())
}
fn sentinel(input: &mut Input, end: usize, budget: &mut Budget<'_>) -> Result<(), SyntaxError> {
    budget.push(&mut input.map, end)
}
fn text<'a>(
    original: &'a str,
    raw: Span,
    cooked: &str,
    budget: &mut Budget<'_>,
) -> Result<Cow<'a, str>, SyntaxError> {
    budget.tick(raw.start as usize, cooked.len())?;
    if &original[raw.range()] == cooked {
        Ok(Cow::Borrowed(&original[raw.range()]))
    } else {
        Ok(Cow::Owned(budget.memory(|m| m.string(cooked))?))
    }
}
fn into_text<'a>(
    original: &'a str,
    raw: Span,
    input: Input,
    budget: &mut Budget<'_>,
) -> Result<Cow<'a, str>, SyntaxError> {
    budget.release(input.map)?;
    budget.release(input.ends)?;
    budget.tick(raw.start as usize, input.text.len())?;
    if original[raw.range()] == input.text {
        budget.memory(|m| m.release_string(input.text))?;
        Ok(Cow::Borrowed(&original[raw.range()]))
    } else {
        Ok(Cow::Owned(input.text))
    }
}
fn code_end(
    input: &Input,
    at: usize,
    budget: &mut Budget<'_>,
) -> Result<Option<usize>, SyntaxError> {
    if input.text.as_bytes()[at] != b'`' {
        return Ok(None);
    }
    let run = input.text.as_bytes()[at..]
        .iter()
        .take_while(|b| **b == b'`')
        .count();
    let slashes = input.text.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|b| **b == b'\\')
        .count();
    budget.tick(input.map[at], run + slashes)?;
    if slashes % 2 != 0 {
        return Ok(None);
    }
    super::code_span_end(&input.text, at + run, run, |at| input.map[at], budget)
}
fn literal_end(input: &Input, at: usize) -> usize {
    if input.text.as_bytes()[at] == b'`' {
        at + input.text.as_bytes()[at..]
            .iter()
            .take_while(|b| **b == b'`')
            .count()
    } else {
        // INVARIANT: the physical-line loop admits a nonempty UTF-8 suffix;
        // every prior advance ends at a scalar or ASCII delimiter boundary.
        at + input.text[at..]
            .chars()
            .next()
            .expect("physical scalar")
            .len_utf8()
    }
}
fn comments(
    input: &Input,
    open: &mut Option<usize>,
    found: &mut Vec<AuthoringComment>,
    budget: &mut Budget<'_>,
) -> Result<Input, SyntaxError> {
    let mut out = empty();
    let mut at = 0;
    while at < input.text.len() {
        budget.tick(input.map[at], 1)?;
        if let Some(start) = *open {
            let (count, closed) = budget
                .memory(|m| purrdf_lex::html::comment_body_with_memory(&input.text[at..], m))?;
            at += count;
            if closed {
                budget.push(
                    found,
                    AuthoringComment {
                        source: span(start, input.map[at - 1] + 1),
                        closed: true,
                    },
                )?;
                *open = None;
            }
        } else if input.text[at..].starts_with("<!--") {
            *open = Some(input.map[at]);
            at += 4;
        } else {
            let end = code_end(input, at, budget)?.unwrap_or_else(|| literal_end(input, at));
            append(
                &mut out,
                &input.text[at..end],
                (at..end).map(|i| (input.map[i], input.ends[i])),
                budget,
            )?;
            at = end;
        }
    }
    // INVARIANT: every physical input appends its original end sentinel,
    // including an empty visible view after comment removal.
    sentinel(
        &mut out,
        *input.map.last().expect("physical sentinel"),
        budget,
    )?;
    Ok(out)
}
fn prose(
    input: &Input,
    doc: &mut SyntaxDocument<'_>,
    budget: &mut Budget<'_>,
) -> Result<Option<Input>, SyntaxError> {
    let mut out = empty();
    let mut at = 0;
    let mut replaced = false;
    while at < input.text.len() {
        budget.tick(input.map[at], 1)?;
        if let Some(end) = code_end(input, at, budget)? {
            budget.push(
                &mut doc.inlines,
                Inline {
                    source: input.source(at..end),
                    kind: InlineKind::Code,
                },
            )?;
            append(
                &mut out,
                " ",
                [(input.map[at], input.ends[end - 1])],
                budget,
            )?;
            replaced = true;
            at = end;
        } else {
            let end = literal_end(input, at);
            append(
                &mut out,
                &input.text[at..end],
                (at..end).map(|i| (input.map[i], input.ends[i])),
                budget,
            )?;
            at = end;
        }
    }
    // INVARIANT: every physical input appends its original end sentinel,
    // including an empty visible view after comment removal.
    sentinel(
        &mut out,
        *input.map.last().expect("physical sentinel"),
        budget,
    )?;
    if replaced {
        Ok(Some(out))
    } else {
        inline::release(out, budget)?;
        Ok(None)
    }
}
fn flush<'a>(
    doc: &mut SyntaxDocument<'a>,
    chunk: &mut Input,
    links: &mut Vec<AuthoringLink<'a>>,
    split: &mut Vec<AuthoringLink<'a>>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    if chunk.text.is_empty() {
        return Ok(());
    }
    sentinel(chunk, chunk.ends.last().copied().unwrap_or(0), budget)?;
    inline::occurrences(doc, chunk, &HashTable::new(), budget)?;
    // Transfer decoded strings rather than duplicating retained link storage.
    let found = std::mem::take(&mut doc.links);
    for link in &found {
        budget.tick(link.source.start as usize, link.source.len() as usize)?;
    }
    // Retain the backing-array charge while the consuming iterator is alive.
    let bytes = budget.memory(|_| {
        std::alloc::Layout::array::<super::Link<'a>>(found.capacity())
            .map(|layout| layout.size())
            .map_err(|_| purrdf_lex::allocation::StorageError::SizeOverflow)
    })?;
    for link in found {
        let multiline = doc.spelling(link.source).contains('\n');
        let link = AuthoringLink {
            source: link.source,
            form: if link.image {
                AuthoringLinkForm::Image
            } else {
                AuthoringLinkForm::Inline
            },
            destination: link.destination,
            title: link.title,
        };
        budget.push(if multiline { &mut *split } else { &mut *links }, link)?;
    }
    budget.memory(|m| m.release_bytes(bytes))?;
    let old = std::mem::replace(chunk, empty());
    inline::release(old, budget)
}
fn flush_html<'a>(
    doc: &SyntaxDocument<'a>,
    chunk: &mut Input,
    links: &mut Vec<AuthoringLink<'a>>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    if chunk.text.is_empty() {
        return Ok(());
    }
    sentinel(chunk, chunk.ends.last().copied().unwrap_or(0), budget)?;
    inline::authoring_attributes(doc, chunk, links, budget)?;
    inline::release(std::mem::replace(chunk, empty()), budget)
}
/// Read the bounded authoring policy from original UTF-8. Resource limits are
/// explicit; callers should set their source/container/link contract directly.
/// Comments concatenate surrounding text; physical code spans become one prose
/// space. Fences stay opaque, ordinary HTML and indentation stay authoring text,
/// ATX headings use physical prefixes, and entities/reference uses stay literal.
pub fn authoring(source: &str, limits: SyntaxLimits) -> Result<AuthoringDocument<'_>, SyntaxError> {
    let mut budget = Budget::new(source, limits, Policy::Authoring);
    budget.limit(0, "source bytes", source.len(), limits.source_bytes)?;
    budget.tick(0, source.len())?;
    let mut doc = SyntaxDocument {
        policy: Policy::Authoring,
        source,
        blocks: Vec::new(),
        lines: Vec::new(),
        links: Vec::new(),
        definitions: Vec::new(),
        inlines: Vec::new(),
        attributes: Vec::new(),
        prose: Vec::new(),
        sliced: Vec::new(),
        auxiliary_bytes: 0,
        work: 0,
    };
    block::read(&mut doc, &mut budget)?;
    budget.release(std::mem::take(&mut doc.sliced))?;
    let mut fenced = Vec::new();
    for _ in &doc.lines {
        budget.push(&mut fenced, false)?;
    }
    for block in &doc.blocks {
        if matches!(block.kind, BlockKind::FencedCode { .. }) {
            for line in block.lines.clone() {
                budget.tick(doc.lines[line].source.start as usize, 1)?;
                fenced[line] = true;
            }
        }
    }
    let mut result = AuthoringDocument {
        syntax: doc,
        lines: Vec::new(),
        headings: Vec::new(),
        comments: Vec::new(),
        links: Vec::new(),
        split_links: Vec::new(),
        auxiliary_bytes: 0,
        work: 0,
    };
    let mut open = None;
    let mut chunk = empty();
    let mut html = empty();
    for (index, fenced) in fenced.iter().copied().enumerate() {
        let line = result.syntax.lines[index];
        if fenced {
            flush(
                &mut result.syntax,
                &mut chunk,
                &mut result.links,
                &mut result.split_links,
                &mut budget,
            )?;
            flush_html(&result.syntax, &mut html, &mut result.links, &mut budget)?;
            budget.push(
                &mut result.lines,
                AuthoringLine {
                    source: line.source,
                    newline: line.newline,
                    fenced,
                    visible: Cow::Borrowed(""),
                    prose: None,
                },
            )?;
            continue;
        }
        let mut raw = empty();
        append(
            &mut raw,
            &source[line.source.range()],
            line.source.range().map(|at| (at, at + 1)),
            &mut budget,
        )?;
        sentinel(&mut raw, line.source.end as usize, &mut budget)?;
        let visible = comments(&raw, &mut open, &mut result.comments, &mut budget)?;
        inline::release(raw, &mut budget)?;
        // Admit the bounded trimming/heading passes before inspecting the view.
        budget.tick(
            line.source.start as usize,
            visible.text.len().saturating_mul(6),
        )?;
        // Comment removal preserves the initial mark as a literal scalar;
        // the recognition offset never rebases this original physical line.
        let payload =
            super::payload_start(source, line.source.start as usize) - line.source.start as usize;
        let indent =
            visible.text[payload..].len() - visible.text[payload..].trim_start_matches(' ').len();
        let heading = if indent <= 3 {
            block::authoring_heading(
                &visible.text,
                payload + indent,
                visible.text.len(),
                Policy::Authoring,
            )
        } else {
            None
        };
        if let Some((level, title)) = heading {
            let range = title.range();
            let raw = visible.source(range.clone());
            let text = text(source, raw, &visible.text[range], &mut budget)?;
            budget.push(
                &mut result.headings,
                AuthoringHeading {
                    source: line.source,
                    title: raw,
                    level,
                    text,
                },
            )?;
        }
        let prose = prose(&visible, &mut result.syntax, &mut budget)?;
        let view = prose.as_ref().unwrap_or(&visible);
        budget.tick(
            line.source.start as usize,
            line.source.len() as usize + view.text.len(),
        )?;
        let blank = source[line.source.range()]
            .trim_matches(purrdf_lex::terminals::is_unicode_white_space)
            .is_empty();
        if blank || heading.is_some() {
            flush(
                &mut result.syntax,
                &mut chunk,
                &mut result.links,
                &mut result.split_links,
                &mut budget,
            )?;
        }
        if blank {
            flush_html(&result.syntax, &mut html, &mut result.links, &mut budget)?;
        } else {
            append(
                &mut html,
                &view.text,
                (0..view.text.len()).map(|i| (view.map[i], view.ends[i])),
                &mut budget,
            )?;
            append(
                &mut html,
                "\n",
                [(line.newline.start as usize, line.newline.end as usize)],
                &mut budget,
            )?;
            inline::authoring_definition(&result.syntax, view, &mut result.links, &mut budget)?;
            let mut from = 0;
            loop {
                let spaces =
                    view.text[from..].len() - view.text[from..].trim_start_matches(' ').len();
                if spaces > 3 || view.text.as_bytes().get(from + spaces) != Some(&b'>') {
                    break;
                }
                from += spaces + 1;
                if view.text.as_bytes().get(from) == Some(&b' ') {
                    from += 1;
                }
            }
            append(
                &mut chunk,
                &view.text[from..],
                (from..view.text.len()).map(|i| (view.map[i], view.ends[i])),
                &mut budget,
            )?;
            append(
                &mut chunk,
                "\n",
                [(line.newline.start as usize, line.newline.end as usize)],
                &mut budget,
            )?;
            if heading.is_some() {
                flush(
                    &mut result.syntax,
                    &mut chunk,
                    &mut result.links,
                    &mut result.split_links,
                    &mut budget,
                )?;
            }
        }
        let prose = prose
            .map(|p| into_text(source, line.source, p, &mut budget))
            .transpose()?;
        let visible = into_text(source, line.source, visible, &mut budget)?;
        budget.push(
            &mut result.lines,
            AuthoringLine {
                source: line.source,
                newline: line.newline,
                fenced,
                visible,
                prose,
            },
        )?;
    }
    flush(
        &mut result.syntax,
        &mut chunk,
        &mut result.links,
        &mut result.split_links,
        &mut budget,
    )?;
    flush_html(&result.syntax, &mut html, &mut result.links, &mut budget)?;
    inline::release(html, &mut budget)?;
    inline::release(chunk, &mut budget)?;
    budget.release(fenced)?;
    if let Some(start) = open {
        budget.push(
            &mut result.comments,
            AuthoringComment {
                source: span(start, source.len()),
                closed: false,
            },
        )?;
    }
    budget.tick(0, result.links.len().saturating_mul(usize::BITS as usize))?;
    result
        .links
        .sort_unstable_by_key(|link| (link.source.start, link.source.end));
    result.auxiliary_bytes = budget.live;
    result.work = budget.work;
    Ok(result)
}

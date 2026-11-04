// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use super::{
    BlockKind, HtmlAttribute, Inline, InlineKind, Link, LinkPart, ReferenceDefinition, Span,
    SyntaxDocument, SyntaxError, budget::Budget, span,
};
use purrdf_core::hash::{hash_table_retained_bound, reserve_table_with_memory, table::HashTable};
use purrdf_hash::fixed::hash_one as hash_of;
use purrdf_lex::html::{Mode, resolution_layout, resolve_preallocated};
use std::borrow::Cow;
use std::ops::Range;

pub(super) struct Input {
    pub(super) text: String,
    pub(super) map: Vec<usize>,
    pub(super) ends: Vec<usize>,
}
impl Input {
    pub(super) fn source(&self, range: Range<usize>) -> Span {
        let start = self.map[range.start];
        let end = if range.is_empty() {
            start
        } else if self.ends.is_empty() {
            self.map[range.end - 1] + 1
        } else {
            self.ends[range.end - 1]
        };
        span(start, end)
    }
}
fn input(
    doc: &SyntaxDocument<'_>,
    lines: Range<usize>,
    budget: &mut Budget<'_>,
    eligible_only: bool,
) -> Result<Input, SyntaxError> {
    let mut result = Input {
        text: String::new(),
        map: Vec::new(),
        ends: Vec::new(),
    };
    for index in lines {
        if eligible_only && !doc.sliced[index].eligible {
            continue;
        }
        let line = &doc.lines[index];
        for _ in 0..line.virtual_spaces {
            budget.push(
                &mut result.map,
                (line.content.start as usize).saturating_sub(1),
            )?;
            budget.memory(|m| m.push_char(&mut result.text, ' '))?;
        }
        let parts = [line.content, line.newline];
        for part in parts {
            for at in part.start as usize..part.end as usize {
                budget.push(&mut result.map, at)?;
            }
            budget.memory(|m| m.push_str(&mut result.text, doc.spelling(part)))?;
        }
    }
    let end = result.map.last().map_or(0, |at| at + 1);
    budget.push(&mut result.map, end)?;
    Ok(result)
}
pub(super) fn release(input: Input, budget: &mut Budget<'_>) -> Result<(), SyntaxError> {
    budget.release(input.map)?;
    budget.release(input.ends)?;
    budget.memory(|m| m.release_string(input.text))
}
#[derive(Clone, Copy)]
enum ValueSyntax {
    Markdown,
    ReferenceLabel,
    HtmlAttribute,
    EscapesOnly,
    Raw,
}
impl ValueSyntax {
    fn mode(self) -> Mode {
        if matches!(self, Self::EscapesOnly | Self::Raw) {
            Mode::Plain
        } else if matches!(self, Self::HtmlAttribute) {
            Mode::Attribute
        } else {
            Mode::Markdown
        }
    }
    fn escapes(self) -> bool {
        !matches!(self, Self::HtmlAttribute | Self::Raw)
    }
    fn limit(self, budget: &Budget<'_>) -> usize {
        if matches!(self, Self::ReferenceLabel) {
            usize::MAX
        } else {
            budget.limits.link_part_bytes
        }
    }
}
fn append_references(
    text: &str,
    mode: Mode,
    out: &mut String,
    at: usize,
    budget: &mut Budget<'_>,
    limit: usize,
) -> Result<(), SyntaxError> {
    budget.tick(
        at,
        text.len()
            .checked_mul(3)
            .ok_or_else(|| SyntaxError::Storage {
                position: budget.position(at),
                cause: purrdf_lex::allocation::StorageError::SizeOverflow,
            })?,
    )?;
    let layout = resolution_layout(text, mode).map_err(|cause| SyntaxError::Reference {
        position: budget.position(at),
        cause,
    })?;
    let bytes = if layout.replaced {
        layout.text_bytes
    } else {
        text.len()
    };
    let next = out
        .len()
        .checked_add(bytes)
        .ok_or_else(|| SyntaxError::Storage {
            position: budget.position(at),
            cause: purrdf_lex::allocation::StorageError::SizeOverflow,
        })?;
    budget.limit(at, "decoded link part bytes", next, limit)?;
    if !layout.replaced {
        return budget.memory(|m| m.push_str(out, text));
    }
    let mut decoded = String::new();
    let mut sources = Vec::new();
    let mut diagnostics = Vec::new();
    budget.memory(|m| {
        m.reserve_string(&mut decoded, layout.text_bytes)?;
        m.reserve(&mut sources, layout.sources)?;
        m.reserve(&mut diagnostics, layout.diagnostics)
    })?;
    let resolution = resolve_preallocated(text, mode, layout, decoded, sources, diagnostics)
        .map_err(|cause| SyntaxError::Reference {
            position: budget.position(at),
            cause,
        })?;
    budget.release(resolution.decoded.sources)?;
    budget.release(resolution.diagnostics)?;
    budget.memory(|m| m.push_str(out, &resolution.decoded.text))?;
    if let Cow::Owned(text) = resolution.decoded.text {
        budget.memory(|m| m.release_string(text))?;
    }
    Ok(())
}
fn decode<'a>(
    source: &'a str,
    raw: Span,
    text: &str,
    syntax: ValueSyntax,
    budget: &mut Budget<'_>,
) -> Result<LinkPart<'a>, SyntaxError> {
    let limit = syntax.limit(budget);
    let mode = syntax.mode();
    let escapes = syntax.escapes();
    budget.limit(raw.start as usize, "link part bytes", text.len(), limit)?;
    budget.tick(raw.start as usize, text.len())?;
    let original = &source[raw.range()];
    if !(text.contains('&') || escapes && text.contains('\\')) && text == original {
        return Ok(LinkPart {
            source: raw,
            decoded: Cow::Borrowed(original),
        });
    }
    let mut decoded = String::new();
    let mut at = 0;
    let mut run = 0;
    while at < text.len() {
        let c = text[at..].chars().next().expect("decode scalar");
        if escapes
            && c == '\\'
            && text
                .as_bytes()
                .get(at + 1)
                .is_some_and(u8::is_ascii_punctuation)
        {
            append_references(
                &text[run..at],
                mode,
                &mut decoded,
                raw.start as usize,
                budget,
                limit,
            )?;
            budget.limit(
                raw.start as usize,
                "decoded link part bytes",
                decoded.len() + 1,
                limit,
            )?;
            budget.memory(|m| m.push_char(&mut decoded, char::from(text.as_bytes()[at + 1])))?;
            at += 2;
            run = at;
        } else {
            at += c.len_utf8();
        }
    }
    append_references(
        &text[run..],
        mode,
        &mut decoded,
        raw.start as usize,
        budget,
        limit,
    )?;
    budget.limit(
        raw.start as usize,
        "decoded link part bytes",
        decoded.len(),
        limit,
    )?;
    let decoded = if decoded == original {
        budget.memory(|m| m.release_string(decoded))?;
        Cow::Borrowed(original)
    } else {
        Cow::Owned(decoded)
    };
    Ok(LinkPart {
        source: raw,
        decoded,
    })
}

fn part<'a>(
    doc: &SyntaxDocument<'a>,
    input: &Input,
    range: Range<usize>,
    budget: &mut Budget<'_>,
) -> Result<LinkPart<'a>, SyntaxError> {
    decode(
        doc.source,
        input.source(range.clone()),
        &input.text[range],
        if doc.policy == super::Policy::Authoring {
            ValueSyntax::EscapesOnly
        } else {
            ValueSyntax::Markdown
        },
        budget,
    )
}
fn label_part<'a>(
    doc: &SyntaxDocument<'a>,
    input: &Input,
    range: Range<usize>,
    budget: &mut Budget<'_>,
) -> Result<LinkPart<'a>, SyntaxError> {
    decode(
        doc.source,
        input.source(range.clone()),
        &input.text[range],
        if doc.policy == super::Policy::Authoring {
            ValueSyntax::EscapesOnly
        } else {
            ValueSyntax::ReferenceLabel
        },
        budget,
    )
}
fn valid_label(raw: &str) -> bool {
    let mut escaped = false;
    let mut count = 0;
    let mut nonblank = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        count += 1;
        if count > 999 || !escaped && matches!(c, '[' | ']') {
            return false;
        }
        nonblank |= !purrdf_lex::terminals::is_ws_char(c);
        if c == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
        }
        escaped = c == '\\' && !escaped;
    }
    nonblank
}
fn key(text: &str, budget: &mut Budget<'_>) -> Result<String, SyntaxError> {
    let mut result = String::new();
    let mut pending_space = false;
    for c in text.chars() {
        // CommonMark §6.3 matches raw labels after Unicode folding and collapse
        // of space, tab and line endings; NBSP remains a real label character.
        if purrdf_lex::terminals::is_ws_char(c) {
            pending_space = !result.is_empty();
            continue;
        }
        if pending_space {
            budget.memory(|m| m.push_char(&mut result, ' '))?;
            pending_space = false;
        }
        let mut failure = None;
        purrdf_lex::unicode::fold_scalar(c, |folded| {
            if failure.is_none()
                && let Err(error) = budget.memory(|m| m.push_char(&mut result, folded))
            {
                failure = Some(error);
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
    }
    Ok(result)
}
fn space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
fn skip(text: &str, mut at: usize) -> usize {
    while text.as_bytes().get(at).is_some_and(|b| space(*b)) {
        at += 1;
    }
    at
}
fn escaped(text: &str, at: usize) -> bool {
    text.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|b| **b == b'\\')
        .count()
        % 2
        != 0
}
fn destination(
    text: &str,
    start: usize,
    budget: &mut Budget<'_>,
    origin: usize,
    policy: super::Policy,
) -> Result<Option<(Range<usize>, usize)>, SyntaxError> {
    let b = text.as_bytes();
    let mut at = start;
    let mut depth = 0;
    let angle = b.get(at) == Some(&b'<');
    if angle {
        at += 1;
    }
    let from = at;
    while at < b.len() {
        budget.tick(origin, 1)?;
        budget.limit(
            origin,
            "link part bytes",
            at - from,
            budget.limits.link_part_bytes,
        )?;
        match b[at] {
            b'\\'
                if b.get(at + 1).is_some_and(|b| {
                    policy == super::Policy::Authoring || b.is_ascii_punctuation()
                }) =>
            {
                at += 2;
                continue;
            }
            b'>' if angle => return Ok(Some((from..at, at + 1))),
            b'<' | b'\n' if angle => return Ok(None),
            b'\r' if angle && policy == super::Policy::Standard => return Ok(None),
            c if !angle
                && (space(c) || c < 32 || policy == super::Policy::Authoring && c == 127) =>
            {
                break;
            }
            b'(' if !angle => {
                depth += 1;
                budget.limit(origin, "inline nesting", depth, budget.limits.nesting)?;
            }
            b')' if !angle => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        at += 1;
    }
    if angle || depth != 0 {
        Ok(None)
    } else {
        Ok(Some((from..at, at)))
    }
}
fn title(
    text: &str,
    start: usize,
    budget: &mut Budget<'_>,
    origin: usize,
    policy: super::Policy,
) -> Result<Option<(Range<usize>, usize)>, SyntaxError> {
    let b = text.as_bytes();
    let Some(&quote @ (b'"' | b'\'' | b'(')) = b.get(start) else {
        return Ok(None);
    };
    let close = if quote == b'(' { b')' } else { quote };
    let mut at = start + 1;
    while at < b.len() {
        budget.tick(origin, 1)?;
        budget.limit(
            origin,
            "link part bytes",
            at - start - 1,
            budget.limits.link_part_bytes,
        )?;
        if b[at] == b'\\'
            && b.get(at + 1)
                .is_some_and(|b| policy == super::Policy::Authoring || b.is_ascii_punctuation())
        {
            at += 2;
            continue;
        }
        if b[at] == close {
            return Ok(Some((start + 1..at, at + 1)));
        }
        if policy == super::Policy::Standard && b[at] == b'(' && quote == b'(' {
            return Ok(None);
        }
        at += 1;
    }
    Ok(None)
}
fn lookup(
    table: &HashTable<(u64, usize)>,
    definitions: &[ReferenceDefinition<'_>],
    key: &str,
    budget: &mut Budget<'_>,
    at: usize,
) -> Result<Option<usize>, SyntaxError> {
    budget.tick(at, key.len())?;
    let hash = hash_of(&key);
    budget.memory(|memory| {
        table
            .find_with_control(
                hash,
                |(_, id), memory| {
                    use purrdf_lex::allocation::Admission as _;
                    memory
                        .admission_mut()
                        .checkpoint(definitions[*id].key.len().max(key.len()) as u64)?;
                    Ok(definitions[*id].key == key)
                },
                memory,
            )
            .map(|found| found.map(|(_, id)| *id))
    })
}
fn duplicate_part<'a>(
    value: &LinkPart<'a>,
    budget: &mut Budget<'_>,
) -> Result<LinkPart<'a>, SyntaxError> {
    let decoded = match &value.decoded {
        Cow::Borrowed(s) => Cow::Borrowed(*s),
        Cow::Owned(s) => Cow::Owned(budget.memory(|m| m.string(s))?),
    };
    Ok(LinkPart {
        source: value.source,
        decoded,
    })
}
fn definition(
    doc: &mut SyntaxDocument<'_>,
    input: &Input,
    block: usize,
    offset: usize,
    consumed_line: &mut usize,
    budget: &mut Budget<'_>,
) -> Result<Option<usize>, SyntaxError> {
    let text = &input.text;
    let from = text.len() - text[offset..].trim_start_matches([' ', '\t']).len();
    if text.as_bytes().get(from) != Some(&b'[') {
        return Ok(None);
    }
    let mut end = from + 1;
    while end < text.len() && (text.as_bytes()[end] != b']' || escaped(text, end)) {
        budget.tick(input.map[end], 1)?;
        end += 1;
    }
    if end == text.len() || text.as_bytes().get(end + 1) != Some(&b':') {
        return Ok(None);
    }
    let begin = skip(text, end + 2);
    let origin = input.map[begin.min(input.map.len() - 1)];
    let Some((dest, next)) = destination(text, begin, budget, origin, doc.policy)? else {
        return Ok(None);
    };
    if dest.is_empty() && text.as_bytes().get(begin) != Some(&b'<') {
        return Ok(None);
    }
    let after = skip(text, next);
    let parsed_title = if after > next {
        title(text, after, budget, origin, doc.policy)?
    } else {
        None
    };
    let finish = parsed_title.as_ref().map_or(next, |(_, at)| *at);
    let line_end = text[finish..]
        .find(['\n', '\r'])
        .map_or(text.len(), |n| finish + n);
    if !text[finish..line_end].trim_matches([' ', '\t']).is_empty() {
        return Ok(None);
    }
    if !valid_label(&input.text[from + 1..end]) {
        return Ok(None);
    }
    let label = label_part(doc, input, from + 1..end, budget)?;
    // A reference definition consumes its paragraph prefix only. Later prose
    // remains a separate eligible line, preserving multiline target/title spans.
    let end_source = input.source(0..finish).end;
    let definition = ReferenceDefinition {
        source: input.source(from..finish),
        key: key(&input.text[from + 1..end], budget)?,
        label,
        destination: part(doc, input, dest, budget)?,
        title: parsed_title
            .map(|(range, _)| part(doc, input, range, budget))
            .transpose()?,
    };
    budget.push(&mut doc.definitions, definition)?;
    while *consumed_line < doc.blocks[block].lines.end {
        let line = *consumed_line;
        budget.tick(doc.lines[line].source.start as usize, 1)?;
        if doc.lines[line].source.start >= end_source {
            break;
        }
        doc.sliced[line].eligible = false;
        *consumed_line += 1;
    }
    Ok(Some(skip(text, line_end)))
}
fn html(
    doc: &mut SyntaxDocument<'_>,
    input: &Input,
    at: usize,
    budget: &mut Budget<'_>,
) -> Result<Option<usize>, SyntaxError> {
    let rest = &input.text[at..];
    budget.tick(input.map[at], 0)?;
    let Some(tag) = budget.memory(|m| purrdf_lex::html::tag_with_memory(rest, m))? else {
        return Ok(None);
    };
    let first = doc.attributes.len();
    let mut cursor = 0;
    while let Some(attr) = budget.memory(|m| tag.next_attribute_with_memory(&mut cursor, m))? {
        if !attr.name.eq_ignore_ascii_case("href") && !attr.name.eq_ignore_ascii_case("src") {
            continue;
        }
        if let Some(value) = attr.value_source {
            let raw = input.source(at + value.start..at + value.end);
            let value = decode(
                doc.source,
                raw,
                &input.text[at + value.start..at + value.end],
                ValueSyntax::HtmlAttribute,
                budget,
            )?;
            let name =
                input.source(at + attr.source.start..at + attr.source.start + attr.name.len());
            budget.push(&mut doc.attributes, HtmlAttribute { name, value })?;
        }
    }
    let end = at + tag.source.end;
    budget.push(
        &mut doc.inlines,
        Inline {
            source: input.source(at..end),
            kind: InlineKind::Html {
                attributes: first..doc.attributes.len(),
            },
        },
    )?;
    Ok(Some(end))
}
fn opaque_html(
    doc: &mut SyntaxDocument<'_>,
    input: &Input,
    at: usize,
    require_closed: bool,
    budget: &mut Budget<'_>,
) -> Result<Option<usize>, SyntaxError> {
    budget.tick(input.map[at], 0)?;
    let Some(token) =
        budget.memory(|m| purrdf_lex::html::opaque_with_memory(&input.text[at..], m))?
    else {
        return Ok(None);
    };
    if require_closed && !token.closed {
        return Ok(None);
    }
    let end = at + token.source.end;
    let kind = if token.kind == purrdf_lex::html::OpaqueKind::Comment {
        InlineKind::Comment
    } else {
        InlineKind::Html {
            attributes: doc.attributes.len()..doc.attributes.len(),
        }
    };
    budget.push(
        &mut doc.inlines,
        Inline {
            source: input.source(at..end),
            kind,
        },
    )?;
    Ok(Some(end))
}
fn prose(
    doc: &mut SyntaxDocument<'_>,
    input: &Input,
    from: usize,
    end: usize,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    if from == end {
        return Ok(());
    }
    let mut begin = from;
    for at in from + 1..end {
        if input.map[at] != input.map[at - 1] + 1 {
            budget.push(&mut doc.prose, input.source(begin..at))?;
            begin = at;
        }
    }
    budget.push(&mut doc.prose, input.source(begin..end))
}
struct Bracket {
    at: usize,
    image: bool,
    active: bool,
}
pub(super) fn occurrences(
    doc: &mut SyntaxDocument<'_>,
    input: &Input,
    table: &HashTable<(u64, usize)>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    let text = &input.text;
    let b = text.as_bytes();
    let mut at = 0;
    let mut brackets = Vec::new();
    let mut prose = 0;
    while at < text.len() {
        budget.tick(input.map[at], 1)?;
        if b[at] == b'\\' && b.get(at + 1).is_some_and(u8::is_ascii_punctuation) {
            at += 2;
            continue;
        }
        let mut opaque = None;
        let mut recorded = false;
        if doc.policy == super::Policy::Standard && b[at] == b'`' {
            let run = b[at..].iter().take_while(|c| **c == b'`').count();
            if let Some(end) =
                super::code_span_end(text, at + run, run, |at| input.map[at], budget)?
            {
                opaque = Some((end, InlineKind::Code));
            }
            if opaque.is_none() {
                at += run;
                continue;
            }
        } else if doc.policy == super::Policy::Standard && b[at] == b'<' {
            let end = if let Some(end) = opaque_html(doc, input, at, true, budget)? {
                Some(end)
            } else {
                html(doc, input, at, budget)?
            };
            if let Some(end) = end {
                opaque = Some((end, InlineKind::Html { attributes: 0..0 }));
                recorded = true;
            }
        }
        if let Some((end, kind)) = opaque {
            if prose < at {
                self::prose(doc, input, prose, at, budget)?;
            }
            if !recorded {
                budget.push(
                    &mut doc.inlines,
                    Inline {
                        source: input.source(at..end),
                        kind,
                    },
                )?;
            }
            at = end;
            prose = end;
            continue;
        }
        if b[at] == b'[' {
            budget.limit(
                input.map[at],
                "inline nesting",
                brackets.len() + 1,
                budget.limits.nesting,
            )?;
            let image = at > 0 && b[at - 1] == b'!' && !escaped(text, at - 1);
            budget.push(
                &mut brackets,
                Bracket {
                    at,
                    image,
                    active: true,
                },
            )?;
            at += 1;
            continue;
        }
        if b[at] == b']'
            && let Some(bracket) = brackets.pop()
            && bracket.active
        {
            let begin = bracket.at;
            let image = bracket.image;
            let mut end = at + 1;
            let mut target = None;
            if b.get(end) == Some(&b'(') {
                let start = skip(text, end + 1);
                if let Some((dest, next)) = destination(
                    text,
                    start,
                    budget,
                    input.map[start.min(text.len())],
                    doc.policy,
                )? {
                    let after = skip(text, next);
                    let title = if after > next || doc.policy == super::Policy::Authoring {
                        title(
                            text,
                            after,
                            budget,
                            input.map[after.min(text.len())],
                            doc.policy,
                        )?
                    } else {
                        None
                    };
                    let close = skip(text, title.as_ref().map_or(next, |(_, at)| *at));
                    if b.get(close) == Some(&b')') {
                        end = close + 1;
                        target = Some((
                            part(doc, input, dest, budget)?,
                            title
                                .map(|(r, _)| part(doc, input, r, budget))
                                .transpose()?,
                            None,
                        ));
                    }
                }
            }
            if target.is_none() && doc.policy == super::Policy::Standard {
                let mut label = begin + 1..at;
                if b.get(end) == Some(&b'[') {
                    let mut close = end + 1;
                    while close < text.len() && (b[close] != b']' || escaped(text, close)) {
                        budget.tick(input.map[close], 1)?;
                        close += 1;
                    }
                    if close < text.len() {
                        if close > end + 1 {
                            label = end + 1..close;
                        }
                        end = close + 1;
                    }
                }
                let raw_label = &input.text[label];
                if !valid_label(raw_label) {
                    at += 1;
                    continue;
                }
                let normalized = key(raw_label, budget)?;
                let found = lookup(
                    table,
                    &doc.definitions,
                    &normalized,
                    budget,
                    input.map[begin],
                )?;
                budget.memory(|m| m.release_string(normalized))?;
                if let Some(id) = found {
                    let definition = &doc.definitions[id];
                    target = Some((
                        duplicate_part(&definition.destination, budget)?,
                        definition
                            .title
                            .as_ref()
                            .map(|p| duplicate_part(p, budget))
                            .transpose()?,
                        Some(id),
                    ));
                }
            }
            if let Some((destination, title, definition)) = target {
                if !image {
                    budget.tick(input.map[at], brackets.len())?;
                    for bracket in &mut brackets {
                        if !bracket.image {
                            bracket.active = false;
                        }
                    }
                }
                budget.push(
                    &mut doc.links,
                    Link {
                        source: input.source(if image { begin - 1 } else { begin }..end),
                        label: input.source(begin + 1..at),
                        image,
                        destination,
                        title,
                        definition,
                    },
                )?;
                at = end;
                continue;
            }
        }
        at += text[at..].chars().next().expect("inline scalar").len_utf8();
    }
    if doc.policy == super::Policy::Standard && prose < text.len() {
        self::prose(doc, input, prose, text.len(), budget)?;
    }
    budget.release(brackets)
}
pub(super) fn definitions(
    doc: &mut SyntaxDocument<'_>,
    block: usize,
    budget: &mut Budget<'_>,
) -> Result<usize, SyntaxError> {
    let input = input(doc, doc.blocks[block].lines.clone(), budget, true)?;
    let first = doc.definitions.len();
    let mut offset = 0;
    let mut consumed_line = doc.blocks[block].lines.start;
    while let Some(next) = definition(doc, &input, block, offset, &mut consumed_line, budget)? {
        if next <= offset || next == input.text.len() {
            break;
        }
        offset = next;
    }
    if doc.definitions.len() > first && consumed_line == doc.blocks[block].lines.end {
        doc.blocks[block].kind = BlockKind::ReferenceDefinitions {
            definitions: first..doc.definitions.len(),
        };
    }
    release(input, budget)?;
    Ok(consumed_line)
}
pub(super) fn read(
    doc: &mut SyntaxDocument<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    for block in 0..doc.blocks.len() {
        if !matches!(doc.blocks[block].kind, BlockKind::Paragraph) {
            continue;
        }
        definitions(doc, block, budget)?;
    }
    let mut table = HashTable::new();
    for (id, definition) in doc.definitions.iter().enumerate() {
        if let Some(existing) = lookup(
            &table,
            &doc.definitions,
            &definition.key,
            budget,
            definition.source.start as usize,
        )? {
            if definition.source.start < doc.definitions[existing].source.start {
                budget.tick(definition.source.start as usize, definition.key.len())?;
                let hash = hash_of(&definition.key.as_str());
                budget.memory(|memory| {
                    let entry = table.find_mut_with_control(
                        hash,
                        |(_, candidate), _| Ok(*candidate == existing),
                        memory,
                    )?;
                    entry.expect("existing reference index entry").1 = id;
                    Ok(())
                })?;
            }
            continue;
        }
        let table_entries = table.len() + 1;
        budget.memory(|m| {
            reserve_table_with_memory(&mut table, table_entries, |(hash, _)| *hash, m)
        })?;
        let hash = hash_of(&definition.key.as_str());
        budget.memory(|memory| {
            table
                .try_insert_unique_with_control(hash, (hash, id), |(h, _), _| Ok(*h), memory)
                .map(|_| ())
                .map_err(|refusal| purrdf_core::hash::table_storage_error(&refusal.cause))
        })?;
    }
    for block in 0..doc.blocks.len() {
        match doc.blocks[block].kind {
            BlockKind::Paragraph | BlockKind::Heading { .. } => {
                let heading = matches!(doc.blocks[block].kind, BlockKind::Heading { .. });
                let input = input(doc, doc.blocks[block].content_range(), budget, !heading)?;
                occurrences(doc, &input, &table, budget)?;
                release(input, budget)?;
            }
            BlockKind::Html { opaque, .. } => {
                let input = input(doc, doc.blocks[block].lines.clone(), budget, false)?;
                let mut at = 0;
                if opaque {
                    let marker =
                        input.text.len() - input.text.trim_start_matches([' ', '\t']).len();
                    if opaque_html(doc, &input, marker, false, budget)?.is_none() {
                        let _ = html(doc, &input, marker, budget)?;
                    }
                    release(input, budget)?;
                    continue;
                }
                while at < input.text.len() {
                    budget.tick(input.map[at], 1)?;
                    if input.text.as_bytes()[at] == b'<' {
                        let end = if let Some(end) = opaque_html(doc, &input, at, false, budget)? {
                            Some(end)
                        } else {
                            html(doc, &input, at, budget)?
                        };
                        if let Some(end) = end {
                            at = end;
                            continue;
                        }
                    }
                    at += input.text[at..]
                        .chars()
                        .next()
                        .expect("HTML scalar")
                        .len_utf8();
                }
                release(input, budget)?;
            }
            _ => {}
        }
    }
    let bytes = hash_table_retained_bound(&table).expect("admitted native table layout");
    drop(table);
    budget.memory(|m| m.release_bytes(bytes))
}

pub(super) fn authoring_attributes<'a>(
    doc: &SyntaxDocument<'a>,
    input: &Input,
    links: &mut Vec<super::authoring::AuthoringLink<'a>>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    use super::authoring::{AuthoringLink, AuthoringLinkForm};
    let mut state = purrdf_lex::html::AuthoringTagState::default();
    loop {
        budget.tick(input.map[0], 0)?;
        let Some(attr) = budget.memory(|m| {
            purrdf_lex::html::authoring_attribute_with_memory(&input.text, &mut state, m)
        })?
        else {
            break;
        };
        let range = attr.value_source.expect("authoring target value");
        let raw = input.source(range);
        let destination = decode(
            doc.source,
            raw,
            attr.value.expect("authoring target spelling"),
            ValueSyntax::Raw,
            budget,
        )?;
        budget.push(
            links,
            AuthoringLink {
                source: input.source(attr.source),
                form: if attr.name.eq_ignore_ascii_case("href") {
                    AuthoringLinkForm::Href
                } else {
                    AuthoringLinkForm::Src
                },
                destination,
                title: None,
            },
        )?;
    }
    Ok(())
}

pub(super) fn authoring_definition<'a>(
    doc: &SyntaxDocument<'a>,
    input: &Input,
    links: &mut Vec<super::authoring::AuthoringLink<'a>>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    use super::authoring::{AuthoringLink, AuthoringLinkForm};
    let text = &input.text;
    budget.tick(input.map[0], text.len().saturating_mul(4))?;
    let payload = super::payload_start(doc.source, input.map[0]) - input.map[0];
    let from = text.len() - text[payload..].trim_start_matches(' ').len();
    if from - payload > 3 || text.as_bytes().get(from) != Some(&b'[') {
        return Ok(());
    }
    let Some(label_end) = text[from + 1..].find("]:").map(|at| at + from + 1) else {
        return Ok(());
    };
    let label = &text[from + 1..label_end];
    if label
        .trim_matches(purrdf_lex::terminals::is_unicode_white_space)
        .is_empty()
        || label.starts_with('^')
    {
        return Ok(());
    }
    let after = label_end + 2;
    let start = text.len()
        - text[after..]
            .trim_start_matches(purrdf_lex::terminals::is_unicode_white_space)
            .len();
    let angle = text.as_bytes().get(start) == Some(&b'<');
    let begin = start + usize::from(angle);
    let end = text[begin..]
        .find(|c: char| {
            if angle {
                c == '>'
            } else {
                purrdf_lex::terminals::is_unicode_white_space(c)
            }
        })
        .map_or(text.len(), |at| begin + at);
    if begin == end {
        return Ok(());
    }
    let destination = part(doc, input, begin..end, budget)?;
    let after = if angle && text.as_bytes().get(end) == Some(&b'>') {
        end + 1
    } else {
        end
    };
    let title_start = skip(text, after);
    let parsed_title = if title_start > after {
        title(
            text,
            title_start,
            budget,
            input.map[title_start],
            doc.policy,
        )?
    } else {
        None
    };
    let finish = parsed_title.as_ref().map_or(after, |(_, end)| *end);
    let title = parsed_title
        .map(|(range, _)| part(doc, input, range, budget))
        .transpose()?;
    budget.push(
        links,
        AuthoringLink {
            source: input.source(from..finish),
            form: AuthoringLinkForm::ReferenceDefinition,
            destination,
            title,
        },
    )
}

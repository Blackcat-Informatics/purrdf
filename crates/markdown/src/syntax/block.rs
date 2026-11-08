// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use super::{
    Block, BlockKind, ContentLine, SliceLine, SyntaxDocument, SyntaxError, budget::Budget, span,
};

struct Container {
    id: usize,
    quote: bool,
    indent: usize,
}
enum Open {
    Fence {
        id: usize,
        byte: u8,
        count: usize,
        indent: usize,
        depth: usize,
    },
    Html {
        id: usize,
        close: Option<&'static str>,
        blank: bool,
        depth: usize,
    },
    Indent {
        id: usize,
        depth: usize,
    },
}
#[derive(Clone, Copy)]
struct Prefix {
    at: usize,
    column: usize,
    pending: usize,
}
impl Prefix {
    fn logical_column(self) -> usize {
        self.column - self.pending
    }
    fn ascii(mut self, count: usize) -> Self {
        self.at += count;
        self.column += count;
        self
    }
    fn space(
        &mut self,
        text: &str,
        end: usize,
        budget: &mut Budget<'_>,
    ) -> Result<bool, SyntaxError> {
        budget.tick(self.at, 1)?;
        if self.pending > 0 {
            self.pending -= 1;
            return Ok(true);
        }
        if self.at >= end {
            return Ok(false);
        }
        Ok(match text.as_bytes()[self.at] {
            b' ' => {
                self.at += 1;
                self.column += 1;
                true
            }
            b'\t' => {
                let width = 4 - self.column % 4;
                self.at += 1;
                self.column += width;
                self.pending = width - 1;
                true
            }
            _ => false,
        })
    }
}
fn indent(
    text: &str,
    mut from: Prefix,
    end: usize,
    maximum: usize,
    budget: &mut Budget<'_>,
) -> Result<(Prefix, usize), SyntaxError> {
    let mut columns = 0;
    while columns < maximum && from.space(text, end, budget)? {
        columns += 1;
    }
    Ok((from, columns))
}
fn interrupt(
    text: &str,
    from: Prefix,
    end: usize,
    budget: &mut Budget<'_>,
) -> Result<bool, SyntaxError> {
    let (probe, columns) = indent(text, from, end, 4, budget)?;
    let at = probe.at;
    Ok(columns <= 3
        && (heading(text, at, end).is_some()
            || list(text, at, end).is_some()
            || rule(&text[at..end], at, budget)?
            || ["```", "~~~", ">"]
                .iter()
                .any(|s| text[at..end].starts_with(s))
            || html_close(&text[at..end], false, at, budget)?.is_some()))
}
fn blank(text: &str) -> bool {
    text.bytes().all(|b| matches!(b, b' ' | b'\t'))
}
fn heading(text: &str, from: usize, end: usize) -> Option<(u32, super::Span)> {
    authoring_heading(text, from, end, super::Policy::Standard)
}
pub(super) fn authoring_heading(
    text: &str,
    from: usize,
    end: usize,
    policy: super::Policy,
) -> Option<(u32, super::Span)> {
    let b = text.as_bytes();
    let mut at = from;
    while at < end && b[at] == b'#' {
        at += 1;
    }
    let count = at - from;
    if count == 0
        || count > 6
        || (policy == super::Policy::Authoring && at == end)
        || at < end && !matches!(b[at], b' ' | b'\t')
    {
        return None;
    }
    while at < end && matches!(b[at], b' ' | b'\t') {
        at += 1;
    }
    let mut last = if policy == super::Policy::Authoring {
        let trimmed = text[at..end].trim_matches(purrdf_lex::terminals::is_unicode_white_space);
        if trimmed.is_empty() {
            return None;
        }
        at = end
            - text[at..end]
                .trim_start_matches(purrdf_lex::terminals::is_unicode_white_space)
                .len();
        at + trimmed.len()
    } else {
        end
    };
    while last > at && matches!(b[last - 1], b' ' | b'\t') {
        last -= 1;
    }
    let hashes = last;
    while last > at && b[last - 1] == b'#' {
        last -= 1;
    }
    if last < hashes && (last == at || matches!(b[last - 1], b' ' | b'\t')) {
        while last > at && matches!(b[last - 1], b' ' | b'\t') {
            last -= 1;
        }
    } else {
        last = hashes;
    }
    if policy == super::Policy::Authoring {
        let title = text[at..last].trim_matches(purrdf_lex::terminals::is_unicode_white_space);
        let begin = last
            - text[at..last]
                .trim_start_matches(purrdf_lex::terminals::is_unicode_white_space)
                .len();
        return Some((count as u32, span(begin, begin + title.len())));
    }
    Some((count as u32, span(at, last)))
}
fn list(text: &str, at: usize, end: usize) -> Option<(usize, Option<u64>)> {
    let b = text.as_bytes();
    let mut next = at;
    let number = if matches!(b.get(at), Some(b'-' | b'+' | b'*')) {
        next += 1;
        None
    } else {
        while next < end && b[next].is_ascii_digit() {
            next += 1;
        }
        if next == at || next - at > 9 || !matches!(b.get(next), Some(b'.' | b')')) {
            return None;
        }
        let value = text[at..next].parse().ok()?;
        next += 1;
        Some(value)
    };
    if next < end && !matches!(b[next], b' ' | b'\t') {
        return None;
    }
    Some((next, number))
}
fn rule(text: &str, origin: usize, budget: &mut Budget<'_>) -> Result<bool, SyntaxError> {
    rule_policy(text, origin, super::Policy::Standard, budget)
}
fn rule_policy(
    text: &str,
    origin: usize,
    policy: super::Policy,
    budget: &mut Budget<'_>,
) -> Result<bool, SyntaxError> {
    let mut marker = None;
    let mut count = 0;
    for (offset, b) in text.bytes().enumerate() {
        budget.tick(origin + offset, 1)?;
        if b == b' ' || policy == super::Policy::Standard && b == b'\t' {
            continue;
        }
        if !matches!(b, b'-' | b'*' | b'_') || marker.is_some_and(|m| m != b) {
            return Ok(false);
        }
        marker = Some(b);
        count += 1;
    }
    Ok(count >= 3)
}

struct HtmlStart {
    close: Option<&'static str>,
    blank: bool,
    comment: bool,
}
fn html_close(
    text: &str,
    type_seven: bool,
    origin: usize,
    budget: &mut Budget<'_>,
) -> Result<Option<HtmlStart>, SyntaxError> {
    budget.tick(origin, text.len().min(12))?;
    if text.starts_with("<!--") {
        return Ok(Some(HtmlStart {
            close: Some("-->"),
            blank: false,
            comment: true,
        }));
    }
    if text.starts_with("<?") {
        return Ok(Some(HtmlStart {
            close: Some("?>"),
            blank: false,
            comment: false,
        }));
    }
    if text.starts_with("<![CDATA[") {
        return Ok(Some(HtmlStart {
            close: Some("]]>"),
            blank: false,
            comment: false,
        }));
    }
    if text.as_bytes().get(0..2) == Some(b"<!")
        && text.as_bytes().get(2).is_some_and(u8::is_ascii_uppercase)
    {
        return Ok(Some(HtmlStart {
            close: Some(">"),
            blank: false,
            comment: false,
        }));
    }
    for (name, close) in [
        ("script", "</script>"),
        ("pre", "</pre>"),
        ("style", "</style>"),
        ("textarea", "</textarea>"),
    ] {
        if let Some(rest) = text.get(1..)
            && rest
                .get(..name.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(name))
            && rest
                .as_bytes()
                .get(name.len())
                .is_none_or(|b| matches!(b, b' ' | b'\t' | b'>'))
        {
            return Ok(Some(HtmlStart {
                close: Some(close),
                blank: false,
                comment: false,
            }));
        }
    }
    let Some(after_angle) = text.strip_prefix('<') else {
        return Ok(None);
    };
    let name_source = after_angle.strip_prefix('/').unwrap_or(after_angle);
    // Every named type-six tag below has at most ten characters; a longer
    // name can still be a type-seven complete tag, using the shared reader.
    let length = name_source
        .bytes()
        .take(11)
        .take_while(|c| c.is_ascii_alphanumeric() || *c == b'-')
        .count();
    let name = &name_source[..length];
    const BLOCK: &[&str] = &[
        "address",
        "article",
        "aside",
        "base",
        "basefont",
        "blockquote",
        "body",
        "caption",
        "center",
        "col",
        "colgroup",
        "dd",
        "details",
        "dialog",
        "dir",
        "div",
        "dl",
        "dt",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "frame",
        "frameset",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hr",
        "html",
        "iframe",
        "legend",
        "li",
        "link",
        "main",
        "menu",
        "menuitem",
        "nav",
        "noframes",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "search",
        "section",
        "summary",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "title",
        "tr",
        "track",
        "ul",
    ];
    let name_end = text.len() - name_source.len() + name.len();
    if BLOCK.iter().any(|n| name.eq_ignore_ascii_case(n))
        && text
            .as_bytes()
            .get(name_end)
            .is_none_or(|b| matches!(b, b' ' | b'\t' | b'>' | b'/'))
    {
        return Ok(Some(HtmlStart {
            close: None,
            blank: true,
            comment: false,
        }));
    }
    if type_seven {
        let tag = budget.memory(|m| purrdf_lex::html::tag_with_memory(text, m))?;
        if let Some(tag) = tag {
            budget.tick(origin + tag.source.end, text.len() - tag.source.end)?;
            if blank(&text[tag.source.end..]) {
                return Ok(Some(HtmlStart {
                    close: None,
                    blank: true,
                    comment: false,
                }));
            }
        }
    }
    Ok(None)
}
fn contains_ascii(text: &str, needle: &str) -> bool {
    text.as_bytes()
        .windows(needle.len())
        .any(|s| s.eq_ignore_ascii_case(needle.as_bytes()))
}
fn after_line_ending(text: &str, end: usize) -> usize {
    match text.as_bytes().get(end) {
        Some(b'\r') if text.as_bytes().get(end + 1) == Some(&b'\n') => end + 2,
        Some(b'\r' | b'\n') => end + 1,
        _ => end,
    }
}
fn add(
    doc: &mut SyntaxDocument<'_>,
    budget: &mut Budget<'_>,
    start: usize,
    end: usize,
    parent: Option<usize>,
    kind: BlockKind,
    line: usize,
) -> Result<usize, SyntaxError> {
    let id = doc.blocks.len();
    budget.push(
        &mut doc.blocks,
        Block {
            source: span(start, after_line_ending(doc.source, end)),
            parent,
            kind,
            lines: line..line + 1,
        },
    )?;
    Ok(id)
}
fn extend(doc: &mut SyntaxDocument<'_>, id: usize, end: usize, line: usize) {
    doc.blocks[id].source.end = after_line_ending(doc.source, end) as u64;
    doc.blocks[id].lines.end = line + 1;
}
pub(super) fn read(
    doc: &mut SyntaxDocument<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    if doc.policy == super::Policy::Authoring {
        return read_authoring(doc, budget);
    }
    let text = doc.source;
    let b = text.as_bytes();
    let mut at = 0;
    let mut containers: Vec<Container> = Vec::new();
    let mut open = None;
    let mut paragraph: Option<usize> = None;
    while at < text.len() {
        let start = at;
        let (end, next) = physical_line(text, start, doc.policy);
        at = next;
        budget.tick(start, at - start + 1)?;
        let line = doc.lines.len();
        let payload_start = super::payload_start(text, start);
        let slicing_source = span(
            payload_start,
            end + usize::from(at == end + 2 && b.get(end) == Some(&b'\r')),
        );
        let mut from = Prefix {
            at: payload_start,
            column: 0,
            pending: 0,
        };
        let mut kept = 0;
        for container in &containers {
            let (indented, columns) = indent(
                text,
                from,
                end,
                if container.quote { 4 } else { container.indent },
                budget,
            )?;
            if container.quote {
                if columns <= 3 && b.get(indented.at) == Some(&b'>') {
                    from = indented.ascii(1);
                    let _ = from.space(text, end, budget)?;
                } else if paragraph.is_none()
                    || blank(&text[from.at..end])
                    || interrupt(text, from, end, budget)?
                {
                    break;
                }
            } else if columns >= container.indent {
                from = indented;
            } else if !blank(&text[from.at..end])
                && (paragraph.is_none() || interrupt(text, from, end, budget)?)
            {
                break;
            }
            kept += 1;
        }
        if kept != containers.len() {
            containers.truncate(kept);
            paragraph = None;
        }
        let depth = containers.len();
        if open.as_ref().is_some_and(|o| match o {
            Open::Fence { depth: d, .. }
            | Open::Html { depth: d, .. }
            | Open::Indent { depth: d, .. } => *d != depth,
        }) {
            open = None;
        }
        let mut handled = false;
        if let Some(state) = &mut open {
            match *state {
                Open::Fence {
                    id,
                    byte,
                    count,
                    indent: leading,
                    ..
                } => {
                    let (probe, columns) = indent(text, from, end, 4, budget)?;
                    let marker = probe.at;
                    let closing = columns <= 3
                        && fence_closes(text, marker, end, byte, count, super::Policy::Standard);
                    let stripped = indent(text, from, end, leading, budget)?.0;
                    let content = if closing {
                        span(end, end)
                    } else {
                        span(stripped.at, end)
                    };
                    let virtual_spaces = if closing { 0 } else { stripped.pending };
                    budget.push(
                        &mut doc.lines,
                        ContentLine {
                            source: span(start, end),
                            content,
                            virtual_spaces,
                            newline: span(end, at),
                        },
                    )?;
                    budget.push(
                        &mut doc.sliced,
                        SliceLine {
                            source: slicing_source,
                            content,
                            heading: None,
                            eligible: false,
                        },
                    )?;
                    extend(doc, id, end, line);
                    if closing {
                        if let BlockKind::FencedCode { closed, .. } = &mut doc.blocks[id].kind {
                            *closed = true;
                        }
                        open = None;
                    }
                    handled = true;
                }
                Open::Html {
                    id,
                    close,
                    blank: blank_end,
                    ..
                } => {
                    if blank_end && blank(&text[from.at..end]) {
                        if let BlockKind::Html { closed, .. } = &mut doc.blocks[id].kind {
                            *closed = true;
                        }
                        open = None;
                    } else {
                        budget.tick(from.at, end - from.at)?;
                        let done = close.is_some_and(|s| contains_ascii(&text[from.at..end], s));
                        budget.push(
                            &mut doc.lines,
                            ContentLine {
                                source: span(start, end),
                                content: span(from.at, end),
                                virtual_spaces: from.pending,
                                newline: span(end, at),
                            },
                        )?;
                        budget.push(
                            &mut doc.sliced,
                            SliceLine {
                                source: slicing_source,
                                content: span(from.at, end),
                                heading: None,
                                eligible: false,
                            },
                        )?;
                        extend(doc, id, end, line);
                        if done {
                            if let BlockKind::Html { closed, .. } = &mut doc.blocks[id].kind {
                                *closed = true;
                            }
                            open = None;
                        }
                        handled = true;
                    }
                }
                Open::Indent { id, .. } => {
                    let (content, columns) = indent(text, from, end, 4, budget)?;
                    if columns >= 4 || blank(&text[from.at..end]) {
                        budget.push(
                            &mut doc.lines,
                            ContentLine {
                                source: span(start, end),
                                content: span(content.at, end),
                                virtual_spaces: content.pending,
                                newline: span(end, at),
                            },
                        )?;
                        budget.push(
                            &mut doc.sliced,
                            SliceLine {
                                source: slicing_source,
                                content: span(content.at, end),
                                heading: None,
                                eligible: false,
                            },
                        )?;
                        if !blank(&text[from.at..end]) {
                            extend(doc, id, end, line);
                        }
                        handled = true;
                    } else {
                        open = None;
                    }
                }
            }
        }
        if handled {
            for c in &containers {
                extend(doc, c.id, end, line);
            }
            continue;
        }
        let mut verse_start = None;
        loop {
            let (probe, columns) = indent(text, from, end, 4, budget)?;
            let marker = probe.at;
            if columns > 3 {
                break;
            }
            let kind;
            let next;
            let quote;
            if b.get(marker) == Some(&b'>') {
                let mut quote_content = probe.ascii(1);
                let _ = quote_content.space(text, end, budget)?;
                next = quote_content;
                kind = BlockKind::Quote;
                quote = true;
            } else if !rule(&text[marker..end], marker, budget)?
                && let Some((marker_end, number)) = list(text, marker, end)
                && (paragraph.is_none() || !blank(&text[marker_end..end]))
            {
                let after_marker = probe.ascii(marker_end - marker);
                let (padding_end, padding) = indent(text, after_marker, end, 5, budget)?;
                next = if padding <= 4 {
                    padding_end
                } else {
                    indent(text, after_marker, end, 1, budget)?.0
                };
                if number.is_some() {
                    verse_start = Some(marker);
                }
                kind = BlockKind::ListItem {
                    marker: span(marker, marker_end),
                    number,
                };
                quote = false;
            } else {
                break;
            }
            budget.limit(
                marker,
                "containers",
                containers.len() + 1,
                budget.limits.containers,
            )?;
            let parent = containers.last().map(|c| c.id);
            let content_indent = match &kind {
                BlockKind::ListItem { marker: mark, .. } if next.at == end => {
                    columns + mark.len() as usize + 1
                }
                _ => next.logical_column() - from.logical_column(),
            };
            let id = add(doc, budget, start, end, parent, kind, line)?;
            budget.push(
                &mut containers,
                Container {
                    id,
                    quote,
                    indent: content_indent,
                },
            )?;
            from = next;
            paragraph = None;
        }
        let parent = containers.last().map(|c| c.id);
        let (probe, columns) = indent(text, from, end, 4, budget)?;
        let marker = probe.at;
        let mut eligible = true;
        let mut head = None;
        let mut content = span(from.at, end);
        let mut virtual_spaces = from.pending;
        let mut new_kind = None;
        if blank(&text[from.at..end]) {
            paragraph = None;
        } else if columns >= 4 && paragraph.is_none() {
            content.start = marker as u64;
            virtual_spaces = probe.pending;
            eligible = false;
            new_kind = Some(BlockKind::IndentedCode);
        } else if columns <= 3
            && let Some((fence, count, info)) =
                fence_start(text, marker, end, super::Policy::Standard)
        {
            let id = add(
                doc,
                budget,
                start,
                end,
                parent,
                BlockKind::FencedCode {
                    marker: span(marker, marker + count),
                    info,
                    closed: false,
                },
                line,
            )?;
            open = Some(Open::Fence {
                id,
                byte: fence,
                count,
                indent: columns,
                depth: containers.len(),
            });
            content = span(end, end);
            virtual_spaces = 0;
            eligible = false;
            paragraph = None;
        } else if columns <= 3
            && let Some(HtmlStart {
                close,
                blank: blank_end,
                comment,
            }) = html_close(&text[marker..end], paragraph.is_none(), marker, budget)?
        {
            let done = close.is_some_and(|s| contains_ascii(&text[marker..end], s));
            let id = add(
                doc,
                budget,
                start,
                end,
                parent,
                BlockKind::Html {
                    comment,
                    opaque: close.is_some(),
                    closed: done,
                },
                line,
            )?;
            if !done {
                open = Some(Open::Html {
                    id,
                    close,
                    blank: blank_end,
                    depth: containers.len(),
                });
            }
            eligible = false;
            paragraph = None;
        } else if columns <= 3
            && let Some(h) = heading(text, marker, end)
        {
            head = Some(h);
            new_kind = Some(BlockKind::Heading {
                level: h.0,
                text: h.1,
            });
            paragraph = None;
        } else if columns <= 3
            && paragraph.is_some()
            && text[marker..end]
                .trim_end_matches([' ', '\t'])
                .bytes()
                .all(|c| c == b[marker])
            && matches!(b.get(marker), Some(b'=' | b'-'))
            && !text[marker..end].is_empty()
        {
            let mut id = paragraph.take().expect("open paragraph");
            let level = if b[marker] == b'=' { 1 } else { 2 };
            let definitions = doc.definitions.len();
            let first = super::inline::definitions(doc, id, budget)?;
            if first > doc.blocks[id].lines.start {
                doc.blocks[id].kind = BlockKind::ReferenceDefinitions {
                    definitions: definitions..doc.definitions.len(),
                };
                doc.blocks[id].lines.end = first;
                doc.blocks[id].source.end = doc.lines[first - 1].newline.end;
            }
            if first == line {
                // Definitions alone cannot provide a Setext heading title.
                if rule(&text[marker..end], marker, budget)? {
                    new_kind = Some(BlockKind::Rule);
                    eligible = false;
                } else {
                    paragraph = Some(add(
                        doc,
                        budget,
                        start,
                        end,
                        parent,
                        BlockKind::Paragraph,
                        line,
                    )?);
                }
            } else {
                let title = span(
                    doc.lines[first].content.start as usize,
                    doc.lines[line - 1].content.end as usize,
                );
                let kind = BlockKind::Heading { level, text: title };
                if first > doc.blocks[id].lines.start {
                    id = add(
                        doc,
                        budget,
                        doc.lines[first].source.start as usize,
                        end,
                        parent,
                        kind,
                        first,
                    )?;
                } else {
                    doc.blocks[id].kind = kind;
                }
                doc.sliced[first].heading = Some((level, title));
                doc.sliced[first].source.end =
                    (end + usize::from(at == end + 2 && b.get(end) == Some(&b'\r'))) as u64;
                doc.sliced[first].eligible = true;
                for continuation in &mut doc.sliced[first + 1..line] {
                    budget.tick(continuation.source.start as usize, 1)?;
                    continuation.eligible = false;
                }
                extend(doc, id, end, line);
                eligible = false;
            }
        } else if columns <= 3 && rule(&text[marker..end], marker, budget)? {
            new_kind = Some(BlockKind::Rule);
            eligible = false;
            paragraph = None;
        } else if let Some(id) = paragraph {
            extend(doc, id, end, line);
        } else {
            paragraph = Some(add(
                doc,
                budget,
                start,
                end,
                parent,
                BlockKind::Paragraph,
                line,
            )?);
        }
        if let Some((_, title)) = head {
            content = title;
            virtual_spaces = 0;
        }
        if let Some(kind) = new_kind {
            let indented = matches!(kind, BlockKind::IndentedCode);
            let id = add(doc, budget, start, end, parent, kind, line)?;
            if indented {
                open = Some(Open::Indent {
                    id,
                    depth: containers.len(),
                });
            }
        }
        budget.push(
            &mut doc.lines,
            ContentLine {
                source: span(start, end),
                content,
                virtual_spaces,
                newline: span(end, at),
            },
        )?;
        budget.push(
            &mut doc.sliced,
            SliceLine {
                source: slicing_source,
                content: span(
                    verse_start.unwrap_or_else(|| {
                        if columns <= 3 {
                            probe.at
                        } else {
                            from.at - usize::from(from.pending > 0)
                        }
                    }),
                    content.end as usize,
                ),
                heading: head,
                eligible,
            },
        )?;
        for c in &containers {
            extend(doc, c.id, end, line);
        }
    }
    if let Some(Open::Html {
        id, blank: true, ..
    }) = open
        && let BlockKind::Html { closed, .. } = &mut doc.blocks[id].kind
    {
        *closed = true;
    }
    budget.release(containers)
}

fn physical_line(text: &str, start: usize, policy: super::Policy) -> (usize, usize) {
    if policy == super::Policy::Authoring {
        let raw_end = purrdf_lex::scan::find_byte(&text.as_bytes()[start..], b'\n')
            .map_or(text.len(), |i| start + i);
        let end = raw_end - usize::from(text[start..raw_end].ends_with('\r'));
        return (end, raw_end + usize::from(raw_end < text.len()));
    }
    let end = purrdf_lex::scan::find_byte2(&text.as_bytes()[start..], b'\n', b'\r')
        .map_or(text.len(), |i| start + i);
    (end, after_line_ending(text, end))
}
fn fence_start(
    text: &str,
    marker: usize,
    end: usize,
    policy: super::Policy,
) -> Option<(u8, usize, super::Span)> {
    let byte = *text
        .as_bytes()
        .get(marker)
        .filter(|b| matches!(b, b'`' | b'~'))?;
    let count = text.as_bytes()[marker..end]
        .iter()
        .take_while(|b| **b == byte)
        .count();
    if count < 3 || byte == b'`' && text[marker + count..end].contains('`') {
        return None;
    }
    let raw = &text[marker + count..end];
    let info = if policy == super::Policy::Authoring {
        raw.trim_matches(purrdf_lex::terminals::is_unicode_white_space)
    } else {
        raw.trim_matches([' ', '\t'])
    };
    let leading = if policy == super::Policy::Authoring {
        raw.len()
            - raw
                .trim_start_matches(purrdf_lex::terminals::is_unicode_white_space)
                .len()
    } else {
        raw.len() - raw.trim_start_matches([' ', '\t']).len()
    };
    let offset = marker + count + leading;
    Some((byte, count, span(offset, offset + info.len())))
}
fn fence_closes(
    text: &str,
    marker: usize,
    end: usize,
    byte: u8,
    count: usize,
    policy: super::Policy,
) -> bool {
    let run = text.as_bytes()[marker..end]
        .iter()
        .take_while(|b| **b == byte)
        .count();
    run >= count
        && if policy == super::Policy::Authoring {
            text[marker + run..end]
                .trim_matches(purrdf_lex::terminals::is_unicode_white_space)
                .is_empty()
        } else {
            blank(&text[marker + run..end])
        }
}
struct AuthoringMarker {
    at: usize,
    end: usize,
    quote: bool,
    number: Option<u64>,
}
// Physical authoring prefixes use literal spaces, without Standard's lazy
// continuation or tab-column interpretation. Marker and fence grammar is shared.
fn extend_authoring(
    doc: &mut SyntaxDocument<'_>,
    mut id: usize,
    end: usize,
    line: usize,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    loop {
        budget.tick(doc.blocks[id].source.start as usize, 1)?;
        doc.blocks[id].source.end = end as u64;
        doc.blocks[id].lines.end = line + 1;
        let Some(parent) = doc.blocks[id].parent else {
            return Ok(());
        };
        id = parent;
    }
}
fn read_authoring(
    doc: &mut SyntaxDocument<'_>,
    budget: &mut Budget<'_>,
) -> Result<(), SyntaxError> {
    let text = doc.source;
    let mut at = 0;
    let mut lists: Vec<(usize, usize)> = Vec::new();
    let mut open: Option<(usize, u8, usize, usize)> = None;
    let mut previous_blank = true;
    while at < text.len() {
        let start = at;
        let (end, next) = physical_line(text, start, doc.policy);
        at = next;
        budget.tick(start, (at - start).saturating_mul(4).saturating_add(1))?;
        let line = doc.lines.len();
        let payload = super::payload_start(text, start);
        let raw = &text[payload..end];
        let is_blank = raw
            .trim_matches(purrdf_lex::terminals::is_unicode_white_space)
            .is_empty();
        let mut markers = Vec::new();
        let mut content = payload;
        if !is_blank {
            let mut prefixes = 0;
            loop {
                // The authoring grammar inspects 64 prefixes plus one overflow
                // probe in fenced text. Keep that fixed lookahead independent
                // of caller admission: a later marker must not expose code.
                if open.is_some() && prefixes == 65 {
                    break;
                }
                let probe = end - text[content..end].trim_start_matches(' ').len();
                if rule_policy(&text[probe..end], probe, super::Policy::Authoring, budget)? {
                    break;
                }
                let (mark_end, quote, number) = if text.as_bytes().get(probe) == Some(&b'>') {
                    (probe + 1, true, None)
                } else if let Some((finish, number)) = list(text, probe, end)
                    && (finish == end || text.as_bytes()[finish] == b' ')
                {
                    (finish, false, number)
                } else {
                    break;
                };
                // Fence content ignores prefix depth. Ordinary prefixes refuse
                // at the first source marker crossing the caller's ceiling.
                if open.is_none() {
                    budget.limit(
                        probe,
                        "containers",
                        markers.len() + 1,
                        budget.limits.containers,
                    )?;
                }
                prefixes += 1;
                if open.is_none() {
                    budget.push(
                        &mut markers,
                        AuthoringMarker {
                            at: probe,
                            end: mark_end,
                            quote,
                            number,
                        },
                    )?;
                }
                content = mark_end;
                if quote {
                    if text.as_bytes().get(content) == Some(&b' ') {
                        content += 1;
                    }
                } else {
                    content = end - text[content..end].trim_start_matches(' ').len();
                }
            }
        }
        if let Some((id, byte, count, allowance)) = open {
            let marker = end - text[content..end].trim_start_matches(' ').len();
            let closing = marker - content <= allowance
                && fence_closes(text, marker, end, byte, count, super::Policy::Authoring);
            extend_authoring(doc, id, at, line, budget)?;
            if closing {
                if let BlockKind::FencedCode { closed, .. } = &mut doc.blocks[id].kind {
                    *closed = true;
                }
                open = None;
            }
        } else if is_blank {
            previous_blank = true;
        } else {
            let first_list = markers
                .iter()
                .find(|marker| !marker.quote)
                .map(|marker| marker.at - payload);
            if let Some(first) = first_list {
                while lists.last().is_some_and(|(column, _)| *column >= first) {
                    lists.pop();
                }
            } else if previous_blank {
                let indentation = raw.len() - raw.trim_start_matches(' ').len();
                while lists
                    .last()
                    .is_some_and(|(column, _)| indentation < column + 2)
                {
                    lists.pop();
                }
            }
            let depth = lists.len() + markers.len();
            let over = depth.saturating_sub(budget.limits.containers);
            let crossing = markers.len().saturating_sub(over.min(markers.len()));
            let origin = markers.get(crossing).map_or(start, |marker| marker.at);
            budget.limit(origin, "containers", depth, budget.limits.containers)?;
            let mut parent = lists.last().map(|(_, id)| *id);
            for marker in &markers {
                let kind = if marker.quote {
                    BlockKind::Quote
                } else {
                    BlockKind::ListItem {
                        marker: span(marker.at, marker.end),
                        number: marker.number,
                    }
                };
                let id = add(doc, budget, start, end, parent, kind, line)?;
                parent = Some(id);
                if !marker.quote {
                    budget.push(&mut lists, (marker.at - payload, id))?;
                }
            }
            previous_blank = false;
            let allowance = lists
                .last()
                .map_or(3, |(column, _)| column.saturating_add(5));
            let marker = end - text[content..end].trim_start_matches(' ').len();
            if marker - content <= allowance
                && let Some((byte, count, info)) =
                    fence_start(text, marker, end, super::Policy::Authoring)
            {
                let id = add(
                    doc,
                    budget,
                    start,
                    end,
                    parent,
                    BlockKind::FencedCode {
                        marker: span(marker, marker + count),
                        info,
                        closed: false,
                    },
                    line,
                )?;
                open = Some((id, byte, count, allowance));
            }
        }
        for (_, id) in &lists {
            extend_authoring(doc, *id, at, line, budget)?;
        }
        budget.push(
            &mut doc.lines,
            ContentLine {
                source: span(start, end),
                content: span(content, end),
                virtual_spaces: 0,
                newline: span(end, at),
            },
        )?;
        budget.release(markers)?;
    }
    budget.release(lists)
}

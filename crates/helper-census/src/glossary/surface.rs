// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A bounded Markdown surface: balanced inline links/images, reference labels
//! and definitions, angle autolinks, exact backtick runs and fenced blocks.
//! This is not a CommonMark renderer. Removed bytes become spaces, preserving
//! newlines/source offsets and preventing accidental token concatenation.

#[derive(Debug)]
pub(super) struct Surface {
    pub(super) visible: String,
    pub(super) prose: String,
}

fn mask(bytes: &mut [u8], start: usize, end: usize) {
    for byte in &mut bytes[start..end] {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
}

fn run(bytes: &[u8], at: usize, needle: u8) -> usize {
    bytes[at..]
        .iter()
        .take_while(|byte| **byte == needle)
        .count()
}

fn inline_end(bytes: &[u8], at: usize) -> Option<usize> {
    let width = run(bytes, at, b'`');
    let mut pos = at + width;
    while pos < bytes.len() {
        if bytes[pos] == b'`' {
            let next = run(bytes, pos, b'`');
            if next == width {
                return Some(pos + next);
            }
            pos += next;
        } else {
            pos += 1;
        }
    }
    None
}

fn balanced(bytes: &[u8], at: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth = 1;
    let mut pos = at + 1;
    let mut quote = None;
    while pos < bytes.len() {
        let ch = bytes[pos];
        if ch == b'\\' {
            pos += 2;
            continue;
        }
        if open == b'['
            && ch == b'`'
            && let Some(end) = inline_end(bytes, pos)
        {
            pos = end;
            continue;
        }
        if open == b'(' && matches!(ch, b'\'' | b'"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() && depth == 1 && bytes[pos - 1].is_ascii_whitespace() {
                quote = Some(ch);
            }
        }
        if quote.is_none() {
            if ch == open {
                depth += 1;
            }
            if ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(pos + 1);
                }
            }
        }
        pos += 1;
    }
    None
}

fn finish_segment(segments: &mut Vec<(usize, usize)>, start: &mut Option<usize>, end: usize) {
    if let Some(begin) = start.take() {
        segments.push((begin, end));
    }
}

pub(super) fn project(text: &str) -> Surface {
    let source = text.as_bytes();
    let mut visible = source.to_vec();
    let mut prose = source.to_vec();
    let mut offset = 0;
    let mut fence: Option<(u8, usize)> = None;
    let mut segments = Vec::new();
    let mut segment_start = None;
    for line in text.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let indent = bytes
            .iter()
            .take_while(|byte| **byte == b' ' || **byte == b'\t')
            .count();
        let trimmed = &bytes[indent..];
        let marker = trimmed.first().copied();
        let width = marker.map_or(0, |ch| run(trimmed, 0, ch));
        let delimiter = matches!(marker, Some(b'`' | b'~')) && width >= 3;
        if let Some((ch, min)) = fence {
            mask(&mut prose, offset, offset + bytes.len());
            if marker == Some(ch)
                && width >= min
                && trimmed[width..].iter().all(u8::is_ascii_whitespace)
            {
                fence = None;
                mask(&mut visible, offset, offset + bytes.len());
            }
            offset += bytes.len();
            continue;
        }
        if delimiter {
            finish_segment(&mut segments, &mut segment_start, offset);
            fence = marker.map(|ch| (ch, width));
            mask(&mut visible, offset, offset + bytes.len());
            mask(&mut prose, offset, offset + bytes.len());
            offset += bytes.len();
            continue;
        }
        // Definition syntax is hidden, irrespective of whether a reference is
        // used in this unit. This does not require cross-unit label resolution.
        if indent <= 3
            && trimmed.first() == Some(&b'[')
            && let Some(end) = balanced(trimmed, 0, b'[', b']')
            && trimmed.get(end) == Some(&b':')
        {
            finish_segment(&mut segments, &mut segment_start, offset);
            mask(&mut visible, offset, offset + bytes.len());
            mask(&mut prose, offset, offset + bytes.len());
            offset += bytes.len();
            continue;
        }
        // CommonMark blank lines contain space/tab; CR/LF here are the
        // physical line terminator, not a wider character-class shorthand.
        if trimmed
            .iter()
            .all(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            finish_segment(&mut segments, &mut segment_start, offset);
        } else {
            segment_start.get_or_insert(offset);
        }
        offset += bytes.len();
    }
    finish_segment(&mut segments, &mut segment_start, source.len());
    // Inline syntax belongs to a paragraph, not an individual physical line.
    // Fences, reference definitions and blank lines delimit these segments.
    for (start, end) in segments {
        let mut pos = start;
        while pos < end {
            // A prior link may already have hidden this destination/title.
            // Its brackets or backticks must never start a second parse.
            if visible[pos] != source[pos] {
                pos += 1;
                continue;
            }
            if source[pos] == b'\\' {
                pos += 2;
                continue;
            }
            if source[pos] == b'`'
                && let Some(close) = inline_end(&source[..end], pos)
            {
                mask(&mut prose, pos, close);
                pos = close;
                continue;
            }
            if source[pos] == b'['
                && let Some(label_end) = balanced(&source[..end], pos, b'[', b']')
            {
                let mut next = label_end;
                if source.get(next) == Some(&b'(')
                    && let Some(close) = balanced(&source[..end], next, b'(', b')')
                {
                    mask(&mut visible, next, close);
                    mask(&mut prose, next, close);
                    next = close;
                } else if source.get(next) == Some(&b'[')
                    && let Some(close) = balanced(&source[..end], next, b'[', b']')
                {
                    mask(&mut visible, next, close);
                    mask(&mut prose, next, close);
                    next = close;
                }
                // Continue inside the label: code and nested labels still need
                // their own projections. The destination is already masked.
                if next > label_end {
                    pos += 1;
                    continue;
                }
            }
            pos += 1;
        }
    }
    Surface {
        visible: String::from_utf8(visible).expect("only whole spans masked"),
        prose: String::from_utf8(prose).expect("only whole spans masked"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_destinations_visible_labels_and_offsets() {
        let text = "[label](https://a/provenance(foo\\)bar) \"IRI title\") ![RDF](hidden)\n[id]: https://a/GMEOW \"provenance\"\n[Rust][id] <https://a/SPARQL> `outside purrdf-core`";
        let result = project(text);
        assert!(!result.visible.contains("provenance"));
        assert!(!result.visible.contains("IRI"));
        assert!(!result.visible.contains("GMEOW"));
        for keep in ["label", "RDF", "Rust", "SPARQL", "outside purrdf-core"] {
            assert!(result.visible.contains(keep));
        }
        assert_eq!(result.visible.len(), text.len());
        assert_eq!(result.visible.lines().count(), text.lines().count());
        assert!(!result.prose.contains("outside purrdf-core"));
        assert!(
            !project("pro[xx](url)venance")
                .visible
                .contains("provenance")
        );
    }

    #[test]
    fn fences_and_exact_runs() {
        let text = "before\n````rust\n资料类型 RDF\n```\n资料类型\n````\nafter `资料类型` ``x `资料类型` x`` unmatched `资料类型\n~~~\n资料类型\n~~~\nend";
        let result = project(text);
        assert_eq!(result.prose.matches("资料类型").count(), 1);
        assert!(result.visible.contains("RDF"));
        assert_eq!(result.prose.len(), text.len());
        assert_eq!(result.prose.lines().count(), text.lines().count());
    }

    #[test]
    fn multiline_inline_links_preserve_labels_offsets_and_block_boundaries() {
        let text = "[visible\nRDF](\nhttps://example.invalid/provenance(foo\\)bar)\n\"IRI\nGMEOW title\"\n)\n![alt\nRust](\nhttps://example.invalid/SPARQL\n)\n<https://example.invalid/OWL>\n\n````\n[code](\nhttps://example.invalid/SHACL\n)\n````\n[broken](\n\nprovenance)\nend";
        let result = project(text);
        for kept in [
            "visible\nRDF",
            "alt\nRust",
            "OWL",
            "SHACL",
            "[code](",
            "provenance)",
        ] {
            assert!(result.visible.contains(kept), "{kept}");
        }
        for hidden in ["provenance(foo", "IRI", "GMEOW", "SPARQL"] {
            assert!(!result.visible.contains(hidden), "{hidden}");
        }
        assert!(!result.prose.contains("SHACL"));
        assert_eq!(result.visible.len(), text.len());
        assert_eq!(result.prose.len(), text.len());
        assert_eq!(
            result
                .visible
                .match_indices('\n')
                .map(|(at, _)| at)
                .collect::<Vec<_>>(),
            text.match_indices('\n')
                .map(|(at, _)| at)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            result
                .prose
                .match_indices('\n')
                .map(|(at, _)| at)
                .collect::<Vec<_>>(),
            text.match_indices('\n')
                .map(|(at, _)| at)
                .collect::<Vec<_>>()
        );
        // A hidden title's unmatched backtick cannot consume real prose after
        // the link. Destination contents are skipped after the first masking.
        let nested = project(
            "[label](\nhttps://example.invalid/RDF\n\"` [hidden](provenance)\"\n)\n真实资料类型 `safe` end",
        );
        assert!(!nested.visible.contains("RDF"));
        assert!(!nested.visible.contains("provenance"));
        assert!(nested.prose.contains("真实资料类型"));
        assert!(!nested.prose.contains("safe"));
        let code = project("before `资料类型\n具名图` after");
        assert!(code.visible.contains("资料类型"));
        assert!(!code.prose.contains("资料类型"));
        assert!(!code.prose.contains("具名图"));
        let labelled_code =
            project("[visible `RDF]`\nlabel](\nhttps://example.invalid/provenance\n)");
        assert!(labelled_code.visible.contains("RDF]"));
        assert!(!labelled_code.prose.contains("RDF]"));
        assert!(!labelled_code.visible.contains("provenance"));
    }
}

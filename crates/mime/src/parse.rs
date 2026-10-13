// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Iterative, original-byte MIME structure. Invalid input becomes typed data;
//! only invalid source/profile or the caller's declared resource policy refuses.

use std::ops::Range;

use purrdf_core::{BaseIri, cover::CoverBuilder};

use crate::{
    Document, Header, Limits, MediaType, MimeError, Part, Problem, ProblemKind, Profile,
    SourceDocument, Structure, StructureKind, TransferEncoding, transfer,
};

#[derive(Clone, Copy)]
struct Line {
    start: usize,
    content_end: usize,
    end: usize,
}

fn line(source: &[u8], start: usize, end: usize) -> Line {
    let next = purrdf_lex::scan::find_byte(&source[start..end], b'\n');
    let end = next.map_or(end, |offset| start + offset + 1);
    let mut content_end = end;
    if source.get(content_end.wrapping_sub(1)) == Some(&b'\n') {
        content_end -= 1;
        if content_end > start && source[content_end - 1] == b'\r' {
            content_end -= 1;
        }
    }
    Line {
        start,
        content_end,
        end,
    }
}

#[derive(Clone, Copy)]
enum DefaultType {
    Text,
    Message,
}

struct Job {
    range: Range<usize>,
    parent: Option<usize>,
    ordinal: usize,
    depth: usize,
    default_type: DefaultType,
}

/// Analyze exact bytes under an explicit profile. Flat work queues and indices
/// make part depth independent of native stack size.
///
/// # Errors
/// Refuses an invalid source IRI, caller-declared resource excess, or a producer
/// bug rejected by the shared cover emitter. Malformed message bytes are retained.
pub fn analyze<'a>(
    source: SourceDocument<'a>,
    profile: &Profile,
) -> Result<Document<'a>, MimeError> {
    BaseIri::parse(source.id)?;
    let limits = profile.limits();
    Limits::check(
        limits.source_bytes,
        source.bytes.len() as u64,
        "source bytes",
    )?;
    let mut emitter = CoverBuilder::new(source.bytes);
    let mut start = 0;
    while start < source.bytes.len() {
        let physical = line(source.bytes, start, source.bytes.len());
        Limits::check(
            limits.line_bytes,
            (physical.content_end - start) as u64,
            "line bytes",
        )?;
        emitter.emit(start..physical.end, None)?;
        start = physical.end;
    }
    let cover = emitter.finish()?;
    let id = profile.document_id(source.id, cover.source_digest());
    let mut document = Document {
        source,
        cover,
        id,
        profile_id: profile.identity(),
        profile: profile.clone(),
        parts: Vec::new(),
        headers: Vec::new(),
        structures: Vec::new(),
        problems: Vec::new(),
    };
    let mut jobs = vec![Job {
        range: 0..source.bytes.len(),
        parent: None,
        ordinal: 0,
        depth: 0,
        default_type: DefaultType::Text,
    }];
    while let Some(job) = jobs.pop() {
        Limits::check(limits.parts, document.parts.len() as u64 + 1, "parts")?;
        Limits::check(limits.depth, job.depth as u64, "part depth")?;
        let index = document.parts.len();
        let (headers, body) = parse_headers(&mut document, index, job.range.clone(), limits)?;
        let media_type = declared_type(&mut document, index, &headers, job.default_type);
        let encoding = declared_encoding(&mut document, index, &headers);
        let part = Part {
            parent: job.parent,
            ordinal: job.ordinal,
            depth: job.depth,
            span: job.range,
            body,
            headers,
            children: Vec::new(),
            media_type,
            transfer_encoding: encoding,
        };
        document.parts.push(part);
        if let Some(parent) = job.parent {
            document.parts[parent].children.push(index);
        }
        let children = child_ranges(&mut document, index);
        for (ordinal, (range, default_type)) in children.into_iter().enumerate().rev() {
            jobs.push(Job {
                range,
                parent: Some(index),
                ordinal,
                depth: job.depth + 1,
                default_type,
            });
        }
    }
    Ok(document)
}

fn problem(document: &mut Document<'_>, part: usize, span: Range<usize>, kind: ProblemKind) {
    document.problems.push(Problem { part, span, kind });
}

fn parse_headers(
    document: &mut Document<'_>,
    part: usize,
    range: Range<usize>,
    limits: Limits,
) -> Result<(Vec<usize>, Range<usize>), MimeError> {
    let source = document.source.bytes;
    let mut headers: Vec<usize> = Vec::new();
    let mut at = range.start;
    while at < range.end {
        let physical = line(source, at, range.end);
        if physical.end - physical.content_end == 1 && source[physical.content_end] == b'\n' {
            problem(
                document,
                part,
                physical.content_end..physical.end,
                ProblemKind::BareLf,
            );
        }
        if physical.content_end == at {
            return Ok((headers, physical.end..range.end));
        }
        let payload = &source[at..physical.content_end];
        for (offset, byte) in payload.iter().enumerate() {
            if *byte == b'\r' {
                problem(
                    document,
                    part,
                    at + offset..at + offset + 1,
                    ProblemKind::BareCr,
                );
            }
        }
        if payload.iter().any(|byte| *byte >= 128) {
            problem(
                document,
                part,
                at..physical.content_end,
                ProblemKind::NonAsciiHeader,
            );
        }
        if matches!(payload.first(), Some(b' ' | b'\t')) {
            if let Some(previous) = headers
                .last()
                .copied()
                .filter(|previous| document.headers[*previous].name.is_some())
            {
                let header = &mut document.headers[previous];
                header.span.end = physical.end;
                header.segments.push(at..physical.content_end);
                at = physical.end;
                continue;
            }
            problem(document, part, at..physical.end, ProblemKind::OrphanFold);
        }
        Limits::check(limits.headers, document.headers.len() as u64 + 1, "headers")?;
        let colon = purrdf_lex::scan::find_byte(payload, b':');
        let name = colon
            .filter(|length| {
                *length > 0
                    && payload[..*length]
                        .iter()
                        .all(|byte| (33..=126).contains(byte) && *byte != b':')
            })
            .map(|length| at..at + length);
        if name.is_none() {
            problem(document, part, at..physical.end, ProblemKind::InvalidHeader);
        }
        let value_start = name.as_ref().map_or(at, |name| name.end + 1);
        let index = document.headers.len();
        document.headers.push(Header {
            part,
            ordinal: headers.len(),
            span: at..physical.end,
            name,
            segments: std::iter::once(value_start..physical.content_end).collect(),
        });
        headers.push(index);
        at = physical.end;
    }
    problem(
        document,
        part,
        range.end..range.end,
        ProblemKind::MissingHeaderSeparator,
    );
    Ok((headers, range.end..range.end))
}

fn header_values(
    document: &Document<'_>,
    headers: &[usize],
    name: &[u8],
) -> Vec<(Range<usize>, Vec<u8>)> {
    headers
        .iter()
        .filter_map(|index| {
            let header = &document.headers[*index];
            header
                .named(document.source.bytes, name)
                .then(|| (header.span.clone(), header.unfolded(document.source.bytes)))
        })
        .collect()
}

fn declared_type(
    document: &mut Document<'_>,
    part: usize,
    headers: &[usize],
    default_type: DefaultType,
) -> Option<MediaType> {
    let values = header_values(document, headers, b"content-type");
    if values.is_empty() {
        return Some(match default_type {
            DefaultType::Text => MediaType {
                main: b"text".to_vec(),
                sub: b"plain".to_vec(),
                parameters: Vec::new(),
            },
            DefaultType::Message => MediaType {
                main: b"message".to_vec(),
                sub: b"rfc822".to_vec(),
                parameters: Vec::new(),
            },
        });
    }
    let mut parsed = None;
    let mut conflict = false;
    for (span, value) in &values {
        let current = parse_media_type(value);
        if current.is_none() {
            problem(
                document,
                part,
                span.clone(),
                ProblemKind::InvalidContentType,
            );
        }
        if let Some(previous) = &parsed {
            if *previous != current {
                conflict = true;
            }
        } else {
            parsed = Some(current);
        }
    }
    if values.len() > 1 {
        let extent = values[0].0.start..values.last().expect("nonempty declarations").0.end;
        problem(
            document,
            part,
            extent.clone(),
            ProblemKind::DuplicateStructuralHeader,
        );
        if conflict {
            problem(
                document,
                part,
                extent,
                ProblemKind::ConflictingStructuralHeaders,
            );
            return None;
        }
    }
    parsed.flatten()
}

fn declared_encoding(
    document: &mut Document<'_>,
    part: usize,
    headers: &[usize],
) -> TransferEncoding {
    let values = header_values(document, headers, b"content-transfer-encoding");
    let mut encoding = None;
    let mut conflict = false;
    for (span, value) in &values {
        let mut parser = FieldParser {
            bytes: value,
            at: 0,
        };
        let token = parser.token().filter(|_| parser.done());
        let current = match token.as_deref().map(<[u8]>::to_ascii_lowercase).as_deref() {
            Some(b"7bit") => TransferEncoding::SevenBit,
            Some(b"8bit") => TransferEncoding::EightBit,
            Some(b"binary") => TransferEncoding::Binary,
            Some(b"base64") => TransferEncoding::Base64,
            Some(b"quoted-printable") => TransferEncoding::QuotedPrintable,
            _ => {
                problem(
                    document,
                    part,
                    span.clone(),
                    ProblemKind::InvalidTransferEncoding,
                );
                TransferEncoding::Unknown(value.clone())
            }
        };
        if let Some(previous) = &encoding {
            if *previous != current {
                conflict = true;
            }
        } else {
            encoding = Some(current);
        }
    }
    if values.len() > 1 {
        let extent = values[0].0.start..values.last().expect("nonempty declarations").0.end;
        problem(
            document,
            part,
            extent.clone(),
            ProblemKind::DuplicateStructuralHeader,
        );
        if conflict {
            problem(
                document,
                part,
                extent,
                ProblemKind::ConflictingStructuralHeaders,
            );
            return TransferEncoding::Ambiguous;
        }
    }
    encoding.unwrap_or(TransferEncoding::SevenBit)
}

fn child_ranges(document: &mut Document<'_>, part: usize) -> Vec<(Range<usize>, DefaultType)> {
    let entry = &document.parts[part];
    let media = entry.media_type.clone();
    let body = entry.body.clone();
    let encoding = entry.transfer_encoding.clone();
    // Header encoded words are validated after all continuations have arrived.
    for index in entry.headers.clone() {
        let header = &document.headers[index];
        let name = header
            .name
            .as_ref()
            .map(|range| &document.source.bytes[range.clone()]);
        let words = transfer::encoded_words_valid(&header.unfolded(document.source.bytes), name);
        let long_word_line = words == Ok(true)
            && header
                .raw(document.source.bytes)
                .split(|byte| *byte == b'\n')
                .any(|line| line.strip_suffix(b"\r").unwrap_or(line).len() > 76);
        if words.is_err() || long_word_line {
            problem(
                document,
                part,
                header.span.clone(),
                ProblemKind::InvalidEncodedWord,
            );
        }
    }
    let multipart = media
        .as_ref()
        .is_some_and(|value| value.main == b"multipart");
    let message = media
        .as_ref()
        .is_some_and(|value| value.main == b"message" && value.sub == b"rfc822");
    if matches!(
        encoding,
        TransferEncoding::SevenBit | TransferEncoding::EightBit
    ) {
        let bytes = &document.source.bytes[body.clone()];
        if transfer::validate_identity(bytes, encoding == TransferEncoding::SevenBit).is_err() {
            problem(
                document,
                part,
                body.clone(),
                ProblemKind::InvalidTransferEncoding,
            );
        }
        for (offset, byte) in bytes.iter().enumerate() {
            let kind = match *byte {
                b'\n' if offset == 0 || bytes[offset - 1] != b'\r' => Some(ProblemKind::BareLf),
                b'\r' if bytes.get(offset + 1) != Some(&b'\n') => Some(ProblemKind::BareCr),
                _ => None,
            };
            if let Some(kind) = kind {
                problem(
                    document,
                    part,
                    body.start + offset..body.start + offset + 1,
                    kind,
                );
            }
        }
    }
    if multipart || message {
        if !matches!(
            encoding,
            TransferEncoding::SevenBit | TransferEncoding::EightBit | TransferEncoding::Binary
        ) {
            problem(
                document,
                part,
                body,
                ProblemKind::UnsupportedContainerEncoding,
            );
            return Vec::new();
        }
    } else if !matches!(
        encoding,
        TransferEncoding::SevenBit | TransferEncoding::EightBit
    ) && transfer::decode(&document.source.bytes[body.clone()], &encoding).is_err()
    {
        problem(
            document,
            part,
            body.clone(),
            ProblemKind::InvalidTransferEncoding,
        );
    }
    if message {
        return vec![(body, DefaultType::Text)];
    }
    if !multipart {
        return Vec::new();
    }
    let media = media.expect("multipart type is present");
    let mut boundaries = media
        .parameters
        .iter()
        .filter(|(name, _)| name == b"boundary");
    let Some((_, boundary)) = boundaries.next() else {
        problem(document, part, body, ProblemKind::MissingBoundary);
        return Vec::new();
    };
    // RFC2046 §5.1.1's 1..70 bchars syntax is a typed syntax observation,
    // never a resource ceiling or a reason to discard the original cover.
    if boundary.is_empty()
        || boundary.len() > 70
        || boundary.last() == Some(&b' ')
        || boundary.iter().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(
                    byte,
                    b'\''
                        | b'('
                        | b')'
                        | b'+'
                        | b'_'
                        | b','
                        | b'-'
                        | b'.'
                        | b'/'
                        | b':'
                        | b'='
                        | b'?'
                        | b' '
                )
        })
        || boundaries.any(|(_, other)| other != boundary)
    {
        problem(document, part, body, ProblemKind::InvalidContentType);
        return Vec::new();
    }
    let default_type = if media.sub == b"digest" {
        DefaultType::Message
    } else {
        DefaultType::Text
    };
    multipart_children(document, part, body, boundary, default_type)
}

fn multipart_children(
    document: &mut Document<'_>,
    part: usize,
    body: Range<usize>,
    boundary: &[u8],
    default_type: DefaultType,
) -> Vec<(Range<usize>, DefaultType)> {
    let source = document.source.bytes;
    let binary = document.parts[part].transfer_encoding == TransferEncoding::Binary;
    let mut children = Vec::new();
    let mut at = body.start;
    let mut open: Option<usize> = None;
    let mut found = false;
    while at < body.end {
        let physical = line(source, at, body.end);
        let payload = &source[at..physical.content_end];
        let marker = payload
            .strip_prefix(b"--")
            .and_then(|tail| tail.strip_prefix(boundary));
        let closing = marker.and_then(|tail| tail.strip_prefix(b"--"));
        let recognized = marker
            .is_some_and(|tail| tail.iter().all(|byte| matches!(byte, b' ' | b'\t')))
            || closing.is_some_and(|tail| tail.iter().all(|byte| matches!(byte, b' ' | b'\t')));
        if recognized {
            if binary
                && physical.end - physical.content_end == 1
                && source[physical.content_end] == b'\n'
            {
                problem(
                    document,
                    part,
                    physical.content_end..physical.end,
                    ProblemKind::BareLf,
                );
            }
            if closing.is_some() && !found {
                problem(
                    document,
                    part,
                    physical.start..physical.end,
                    ProblemKind::MissingOpeningBoundary,
                );
            }
            let mut boundary_start = physical.start;
            if boundary_start >= body.start + 2
                && &source[boundary_start - 2..boundary_start] == b"\r\n"
            {
                boundary_start -= 2;
            } else if boundary_start > body.start && source[boundary_start - 1] == b'\n' {
                boundary_start -= 1;
                if binary {
                    problem(
                        document,
                        part,
                        boundary_start..physical.start,
                        ProblemKind::BareLf,
                    );
                }
            }
            // Adjacent delimiter lines do not invent a backwards empty child
            // by charging the previous delimiter's own ending to this one.
            boundary_start = boundary_start.max(open.unwrap_or(body.start));
            if let Some(start) = open.take() {
                // The parent owns the delimiter CRLF, not the preceding child.
                children.push((start..boundary_start, default_type));
            } else if !found {
                document.structures.push(Structure {
                    part,
                    span: body.start..boundary_start,
                    kind: StructureKind::Preamble,
                });
            }
            found = true;
            let kind = if closing.is_some() {
                StructureKind::ClosingBoundary
            } else {
                StructureKind::Boundary
            };
            document.structures.push(Structure {
                part,
                span: boundary_start..physical.end,
                kind,
            });
            if closing.is_some() {
                document.structures.push(Structure {
                    part,
                    span: physical.end..body.end,
                    kind: StructureKind::Epilogue,
                });
                return children;
            }
            open = Some(physical.end);
        } else if marker.is_some() {
            problem(
                document,
                part,
                physical.start..physical.end,
                ProblemKind::UnexpectedBoundary,
            );
        }
        at = physical.end;
    }
    if let Some(start) = open {
        children.push((start..body.end, default_type));
    }
    problem(
        document,
        part,
        body.clone(),
        if found {
            ProblemKind::MissingClosingBoundary
        } else {
            ProblemKind::MissingBoundary
        },
    );
    if !found {
        document.structures.push(Structure {
            part,
            span: body,
            kind: StructureKind::Preamble,
        });
    }
    children
}

struct FieldParser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl FieldParser<'_> {
    fn cfws(&mut self) -> bool {
        loop {
            while self
                .bytes
                .get(self.at)
                .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
            {
                self.at += 1;
            }
            if self.bytes.get(self.at) != Some(&b'(') {
                return true;
            }
            self.at += 1;
            let mut depth = 1usize;
            while depth > 0 {
                let Some(byte) = self.bytes.get(self.at).copied() else {
                    return false;
                };
                self.at += 1;
                match byte {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    b'\\' => {
                        if self.at == self.bytes.len() {
                            return false;
                        }
                        self.at += 1;
                    }
                    b'\r' | b'\n' => return false,
                    _ => {}
                }
            }
        }
    }

    fn token(&mut self) -> Option<Vec<u8>> {
        if !self.cfws() {
            return None;
        }
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(|byte| {
            byte.is_ascii_graphic()
                && !matches!(
                    byte,
                    b'(' | b')'
                        | b'<'
                        | b'>'
                        | b'@'
                        | b','
                        | b';'
                        | b':'
                        | b'\\'
                        | b'"'
                        | b'/'
                        | b'['
                        | b']'
                        | b'?'
                        | b'='
                )
        }) {
            self.at += 1;
        }
        (self.at > start).then(|| self.bytes[start..self.at].to_vec())
    }

    fn expect(&mut self, byte: u8) -> bool {
        if !self.cfws() || self.bytes.get(self.at) != Some(&byte) {
            return false;
        }
        self.at += 1;
        true
    }

    fn value(&mut self) -> Option<Vec<u8>> {
        if !self.cfws() {
            return None;
        }
        if self.bytes.get(self.at) != Some(&b'"') {
            return self.token();
        }
        self.at += 1;
        let mut value = Vec::new();
        loop {
            let mut byte = *self.bytes.get(self.at)?;
            self.at += 1;
            match byte {
                b'"' => return Some(value),
                b'\\' => {
                    byte = *self.bytes.get(self.at)?;
                    self.at += 1;
                }
                b'\r' | b'\n' => return None,
                _ => {}
            }
            value.push(byte);
        }
    }

    fn done(&mut self) -> bool {
        self.cfws() && self.at == self.bytes.len()
    }
}

fn parse_media_type(bytes: &[u8]) -> Option<MediaType> {
    let mut parser = FieldParser { bytes, at: 0 };
    let main = parser.token()?.to_ascii_lowercase();
    if !parser.expect(b'/') {
        return None;
    }
    let sub = parser.token()?.to_ascii_lowercase();
    let mut parameters = Vec::new();
    while !parser.done() {
        if !parser.expect(b';') {
            return None;
        }
        let name = parser.token()?.to_ascii_lowercase();
        if !parser.expect(b'=') {
            return None;
        }
        let value = parser.value()?;
        parameters.push((name, value));
    }
    Some(MediaType {
        main,
        sub,
        parameters,
    })
}

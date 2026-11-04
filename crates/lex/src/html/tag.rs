// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! CommonMark §6.6 HTML tag and attribute grammar over original UTF-8 bytes.
//! This shared lexical subset performs no WHATWG tree building or recovery.
use crate::allocation::{Admission, Memory, Resident, StorageError};
use std::ops::Range;

/// A tag's syntactic direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagKind {
    /// Opening tag.
    Start,
    /// Closing tag.
    End,
    /// Explicitly self-closing tag.
    SelfClosing,
}
/// A complete CommonMark HTML tag over original source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag<'a> {
    /// Entire spelling, including angle brackets.
    pub source: Range<usize>,
    /// Original case-preserving name.
    pub name: &'a str,
    /// Tag direction.
    pub kind: TagKind,
    attributes: &'a str,
    offset: usize,
}
/// An attribute occurrence; duplicate names remain ordered occurrences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute<'a> {
    /// Whole spelling, excluding surrounding whitespace.
    pub source: Range<usize>,
    /// Original ASCII name; HTML name comparison is ASCII case insensitive.
    pub name: &'a str,
    /// Unquoted value spelling, or None for an attribute without a value.
    pub value: Option<&'a str>,
    /// Value bytes excluding quote delimiters.
    pub value_source: Option<Range<usize>>,
}
struct Scan<'a, 's, S: Admission + ?Sized> {
    input: &'a str,
    admission: &'s mut S,
    failure: Option<StorageError>,
}
impl<S: Admission + ?Sized> Scan<'_, '_, S> {
    fn matches(&mut self, at: usize, expected: &[u8]) -> bool {
        expected
            .iter()
            .copied()
            .enumerate()
            .all(|(offset, byte)| self.byte(at + offset) == Some(byte))
    }
    fn byte(&mut self, at: usize) -> Option<u8> {
        if self.failure.is_some() {
            return None;
        }
        if let Err(error) = self.admission.checkpoint(1) {
            self.failure = Some(error);
            return None;
        }
        self.input.as_bytes().get(at).copied()
    }
}
fn whitespace<S: Admission + ?Sized>(scan: &mut Scan<'_, '_, S>, at: &mut usize) -> Option<()> {
    let mut ending = false;
    loop {
        match scan.byte(*at) {
            Some(b' ' | b'\t') => *at += 1,
            Some(c @ (b'\n' | b'\r')) => {
                if ending {
                    return None;
                }
                ending = true;
                *at += 1;
                if c == b'\r' && scan.byte(*at) == Some(b'\n') {
                    *at += 1;
                }
            }
            _ => return Some(()),
        }
    }
}
fn name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || matches!(c, b'_' | b':')
}
fn name_continue(c: u8) -> bool {
    name_start(c) || c.is_ascii_digit() || matches!(c, b'.' | b'-')
}
fn attribute<'a, S: Admission + ?Sized>(
    scan: &mut Scan<'a, '_, S>,
    start: usize,
) -> Option<(Attribute<'a>, usize)> {
    let input = scan.input;
    let mut at = start;
    whitespace(scan, &mut at)?;
    let begin = at;
    if !scan.byte(at).is_some_and(name_start) {
        return None;
    }
    name_end(scan, &mut at, TagPolicy::Standard);
    let name_end = at;
    whitespace(scan, &mut at)?;
    let mut value_source = None;
    if scan.byte(at) == Some(b'=') {
        at += 1;
        whitespace(scan, &mut at)?;
        value_source = Some(value(scan, &mut at, TagPolicy::Standard)?.0);
    } else {
        at = name_end;
    }
    Some((
        Attribute {
            source: begin..at,
            name: &input[begin..name_end],
            value: value_source.as_ref().map(|r| &input[r.clone()]),
            value_source,
        },
        at,
    ))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum TagPolicy {
    Standard,
    Authoring,
}
fn name_end<S: Admission + ?Sized>(scan: &mut Scan<'_, '_, S>, at: &mut usize, policy: TagPolicy) {
    if policy == TagPolicy::Authoring {
        scan.through(at, crate::terminals::is_authoring_html_name);
    } else {
        *at += 1;
        while scan.byte(*at).is_some_and(name_continue) {
            *at += 1;
        }
    }
}
// One admitted value reader, with the deliberately different unquoted sets
// selected by the closed policy. Quoted delimiter scanning is shared verbatim.
fn value<S: Admission + ?Sized>(
    scan: &mut Scan<'_, '_, S>,
    at: &mut usize,
    policy: TagPolicy,
) -> Option<(Range<usize>, bool)> {
    let quote = scan.byte(*at)?;
    let quoted = matches!(quote, b'\'' | b'"');
    if quoted {
        *at += 1;
    }
    let from = *at;
    if quoted {
        while scan.byte(*at)? != quote {
            *at += 1;
        }
    } else if policy == TagPolicy::Authoring {
        scan.through(at, |c| {
            !crate::terminals::is_unicode_white_space(c) && c != '>'
        });
    } else {
        while scan.byte(*at).is_some_and(|c| {
            !crate::terminals::is_ws(c) && !matches!(c, b'>' | b'<' | b'=' | b'`' | b'"' | b'\'')
        }) {
            *at += 1;
        }
        if *at == from {
            return None;
        }
    }
    let end = *at;
    if quoted {
        *at += 1;
    }
    Some((from..end, quoted))
}
impl Tag<'_> {
    /// Inspect the next validated attribute under original native work control.
    /// Start `cursor` at zero; successful inspection advances it. Returned ranges
    /// use the tag input's original byte coordinates.
    pub fn next_attribute_with_memory<S: Admission + ?Sized>(
        &self,
        cursor: &mut usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<Option<Attribute<'_>>, StorageError> {
        let mut scan = Scan {
            input: self.attributes,
            admission: memory.admission_mut(),
            failure: None,
        };
        let next = attribute(&mut scan, *cursor);
        if let Some(error) = scan.failure {
            return Err(error);
        }
        Ok(next.map(|(mut attr, at)| {
            *cursor = at;
            attr.source.start += self.offset;
            attr.source.end += self.offset;
            if let Some(range) = &mut attr.value_source {
                range.start += self.offset;
                range.end += self.offset;
            }
            attr
        }))
    }
    /// Attributes in source order, with byte ranges relative to the original tag
    /// input. This allocation-free iterator visits only the already validated tag.
    pub fn attributes(&self) -> impl Iterator<Item = Attribute<'_>> {
        let mut at = 0;
        std::iter::from_fn(move || {
            let mut resident = Resident;
            let mut scan = Scan {
                input: self.attributes,
                admission: &mut resident,
                failure: None,
            };
            let (mut attr, next) = attribute(&mut scan, at)?;
            at = next;
            attr.source.start += self.offset;
            attr.source.end += self.offset;
            if let Some(range) = &mut attr.value_source {
                range.start += self.offset;
                range.end += self.offset;
            }
            Some(attr)
        })
    }
}
fn read<'a, S: Admission + ?Sized>(
    input: &'a str,
    admission: &mut S,
) -> Result<Option<Tag<'a>>, StorageError> {
    let mut scan = Scan {
        input,
        admission,
        failure: None,
    };
    let found = (|| {
        if scan.byte(0) != Some(b'<') {
            return None;
        }
        let end_tag = scan.byte(1) == Some(b'/');
        let start = if end_tag { 2 } else { 1 };
        if !scan.byte(start)?.is_ascii_alphabetic() {
            return None;
        }
        let mut at = start + 1;
        while scan
            .byte(at)
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            at += 1;
        }
        let name = &input[start..at];
        let attributes_start = at;
        loop {
            let before = at;
            whitespace(&mut scan, &mut at)?;
            if scan.byte(at) == Some(b'>') {
                return Some(Tag {
                    source: 0..at + 1,
                    name,
                    kind: if end_tag {
                        TagKind::End
                    } else {
                        TagKind::Start
                    },
                    attributes: &input[attributes_start..at],
                    offset: attributes_start,
                });
            }
            if !end_tag && scan.byte(at) == Some(b'/') && scan.byte(at + 1) == Some(b'>') {
                return Some(Tag {
                    source: 0..at + 2,
                    name,
                    kind: TagKind::SelfClosing,
                    attributes: &input[attributes_start..at],
                    offset: attributes_start,
                });
            }
            if end_tag || at == before {
                return None;
            }
            let (_, next) = attribute(&mut scan, at)?;
            at = next;
        }
    })();
    match scan.failure {
        Some(error) => Err(error),
        None => Ok(found),
    }
}
/// Read one complete CommonMark §6.6 HTML tag without allocation.
/// Malformed or unfinished spellings return None and remain ordinary text.
pub fn tag(input: &str) -> Option<Tag<'_>> {
    read(input, &mut Resident).expect("resident tag inspection has no fallible admission")
}
/// The same tag reader under the caller's original native work admission.
/// Each byte inspection checkpoints before it occurs; no suffix is precharged.
/// Original admission failures return before any tag is published.
pub fn tag_with_memory<'a, S: Admission + ?Sized>(
    input: &'a str,
    memory: &mut Memory<'_, S>,
) -> Result<Option<Tag<'a>>, StorageError> {
    read(input, memory.admission_mut())
}

/// A CommonMark opaque HTML form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpaqueKind {
    /// `<!-- ... -->`, including its two immediate closing forms.
    Comment,
    /// `<? ... ?>`.
    ProcessingInstruction,
    /// `<!` followed by an ASCII letter and text through `>`.
    Declaration,
    /// `<![CDATA[ ... ]]>`.
    Cdata,
}
/// An opaque CommonMark HTML form, including unfinished original spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opaque {
    /// Original input bytes occupied by the form.
    pub source: Range<usize>,
    /// Native lexical classification.
    pub kind: OpaqueKind,
    /// A native closing form was present rather than end of input.
    pub closed: bool,
}
/// Inspect one CommonMark opaque HTML form under original native work control.
/// The special `<!-->` and `<!--->` forms close immediately. An unfinished
/// form is retained through input end with `closed = false`; inline HTML
/// recognition requires `closed = true`.
pub fn opaque_with_memory<S: Admission + ?Sized>(
    input: &str,
    memory: &mut Memory<'_, S>,
) -> Result<Option<Opaque>, StorageError> {
    let mut scan = Scan {
        input,
        admission: memory.admission_mut(),
        failure: None,
    };
    let result = (|| {
        let (kind, at, closer): (_, _, &[u8]) = if scan.matches(0, b"<!--") {
            (OpaqueKind::Comment, 4, b"-->")
        } else if scan.matches(0, b"<?") {
            (OpaqueKind::ProcessingInstruction, 2, b"?>")
        } else if scan.matches(0, b"<![CDATA[") {
            (OpaqueKind::Cdata, 9, b"]]>")
        } else if scan.matches(0, b"<!") && scan.byte(2).is_some_and(|c| c.is_ascii_alphabetic()) {
            (OpaqueKind::Declaration, 3, b">")
        } else {
            return None;
        };
        if kind == OpaqueKind::Comment {
            for ending in [b">".as_slice(), b"->"] {
                if scan.matches(at, ending) {
                    return Some(Opaque {
                        source: 0..at + ending.len(),
                        kind,
                        closed: true,
                    });
                }
            }
        }
        let (end, closed) = close(&mut scan, at, closer)?;
        Some(Opaque {
            source: 0..end,
            kind,
            closed,
        })
    })();
    match scan.failure {
        Some(error) => Err(error),
        None => Ok(result),
    }
}

// Shared admitted closing-delimiter scan. The caller owns opening policy.
fn close<S: Admission + ?Sized>(
    scan: &mut Scan<'_, '_, S>,
    mut at: usize,
    closer: &[u8],
) -> Option<(usize, bool)> {
    while at < scan.input.len() {
        if scan.matches(at, closer) {
            return Some((at + closer.len(), true));
        }
        if scan.failure.is_some() {
            return None;
        }
        at += 1;
    }
    Some((scan.input.len(), false))
}

/// Continue an HTML comment body through `-->`, under original admission.
/// The input begins after the opening marker, or at a later physical line.
/// Unlike CommonMark's complete opaque form, this accepts no immediate closer.
/// Returns the consumed byte count (including the closer) and whether it closed.
pub fn comment_body_with_memory<S: Admission + ?Sized>(
    input: &str,
    memory: &mut Memory<'_, S>,
) -> Result<(usize, bool), StorageError> {
    let mut scan = Scan {
        input,
        admission: memory.admission_mut(),
        failure: None,
    };
    let result = close(&mut scan, 0, b"-->");
    match scan.failure {
        Some(error) => Err(error),
        // INVARIANT: close returns None only after recording admission failure.
        None => Ok(result.expect("admitted comment continuation")),
    }
}

/// Cursor for the physical-line authoring tag policy. Reset it at a blank or
/// fenced boundary; supply the same admitted text on each subsequent call.
#[derive(Default, Debug)]
pub struct AuthoringTagState {
    at: usize,
    in_tag: bool,
}
impl<S: Admission + ?Sized> Scan<'_, '_, S> {
    fn scalar(&mut self, at: usize) -> Option<char> {
        let c = self.input.get(at..)?.chars().next()?;
        for offset in 0..c.len_utf8() {
            self.byte(at + offset)?;
        }
        Some(c)
    }
    fn through(&mut self, at: &mut usize, keep: impl Fn(char) -> bool) {
        while let Some(c) = self.scalar(*at) {
            if !keep(c) {
                break;
            }
            *at += c.len_utf8();
        }
    }
}
/// Inspect authoring attributes without requiring a complete tag. Ordinary
/// Unicode names and whitespace follow the physical-line authoring policy.
/// Quoted values crossing a physical line are skipped; a value beginning on a
/// later line is admitted. Unquoted trailing slashes stay in the source range
/// and are removed from the interpreted value. No entities are decoded.
pub fn authoring_attribute_with_memory<'a, S: Admission + ?Sized>(
    input: &'a str,
    state: &mut AuthoringTagState,
    memory: &mut Memory<'_, S>,
) -> Result<Option<Attribute<'a>>, StorageError> {
    let mut scan = Scan {
        input,
        admission: memory.admission_mut(),
        failure: None,
    };
    let found = (|| {
        let mut at = state.at;
        while at < input.len() {
            let c = scan.scalar(at)?;
            if !state.in_tag {
                at += c.len_utf8();
                if c == '<'
                    && scan.scalar(at).is_some_and(|c| {
                        crate::terminals::is_authoring_html_tag_start(c) || c == '/'
                    })
                {
                    let start = at;
                    scan.through(&mut at, |c| {
                        crate::terminals::is_authoring_html_name(c) || c == '/'
                    });
                    scan.admission
                        .checkpoint((at - start) as u64)
                        .map_err(|error| scan.failure = Some(error))
                        .ok()?;
                    state.in_tag = !input[start..at].contains(':');
                }
            } else if c == '>' {
                state.in_tag = false;
                at += 1;
            } else if matches!(c, '\'' | '"') {
                at += 1;
                scan.through(&mut at, |next| next != c);
                if scan.scalar(at) == Some(c) {
                    at += 1;
                }
            } else if crate::terminals::is_authoring_html_name(c) {
                let start = at;
                name_end(&mut scan, &mut at, TagPolicy::Authoring);
                let name_end = at;
                scan.through(&mut at, |c| {
                    crate::terminals::is_unicode_white_space(c) && c != '\n'
                });
                if scan.scalar(at) != Some('=') {
                    at = name_end;
                    continue;
                }
                at += 1;
                scan.through(&mut at, |c| {
                    crate::terminals::is_unicode_white_space(c) && c != '\n'
                });
                let wanted = input[start..name_end].eq_ignore_ascii_case("href")
                    || input[start..name_end].eq_ignore_ascii_case("src");
                if scan.scalar(at).is_some_and(|c| c == '\n') {
                    if !wanted {
                        continue;
                    }
                    scan.through(&mut at, crate::terminals::is_unicode_white_space);
                }
                let Some((range, quoted)) = value(&mut scan, &mut at, TagPolicy::Authoring) else {
                    break;
                };
                let from = range.start;
                let end = range.end;
                let name = &input[start..name_end];
                scan.admission
                    .checkpoint((end - from) as u64)
                    .map_err(|error| scan.failure = Some(error))
                    .ok()?;
                if (name.eq_ignore_ascii_case("href") || name.eq_ignore_ascii_case("src"))
                    && !(quoted && input[from..end].contains('\n'))
                {
                    state.at = at;
                    return Some(Attribute {
                        source: start..at,
                        name,
                        value: Some(if quoted {
                            &input[from..end]
                        } else {
                            input[from..end].trim_end_matches('/')
                        }),
                        value_source: Some(from..end),
                    });
                }
            } else {
                at += c.len_utf8();
            }
            if scan.failure.is_some() {
                return None;
            }
        }
        state.at = at;
        None
    })();
    match scan.failure {
        Some(error) => Err(error),
        None => Ok(found),
    }
}

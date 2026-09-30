// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The document type declaration: the grammar of its markup declarations and
//! the internal subset's parameter entities.
//!
//! A non-validating processor still checks every markup declaration for
//! well-formedness (XML 1.0 §2.8, §3.2, §3.3, §4.2, §4.7); this module holds
//! that grammar, `ELEMENT` content models included, and applies the
//! declarations that matter to a reader: general entities and `ATTLIST`
//! defaults.
//!
//! # Parameter entities
//!
//! A parameter-entity reference (`%name;`) is read between the internal
//! subset's markup declarations (`DeclSep`), where the entity's replacement
//! text is read as further declarations, comments, processing instructions and
//! references, exactly as if it stood in the subset (§4.4.8). Three
//! constraints keep that well-formed:
//!
//! * *PEs in Internal Subset*: a reference inside a markup declaration is
//!   refused ([`XmlErrorKind::ParameterEntity`]);
//! * *Proper Declaration/PE Nesting*: a declaration, comment or processing
//!   instruction that opens in an entity's replacement text closes in it
//!   ([`XmlErrorKind::UnbalancedEntity`]);
//! * *No Recursion*: an entity does not contain itself
//!   ([`XmlErrorKind::RecursiveEntity`]).
//!
//! The reader keeps its open entities on a heap stack, and every inclusion is
//! charged to the same replacement-text budget general entities are
//! ([`XmlErrorKind::EntityExpansionLimit`]), so a nest of parameter entities
//! is bounded like a nest of general ones.
//!
//! An *external* parameter entity is declared but never read (§5.1). The
//! reader then stops applying declarations: the ones after the reference are
//! still checked for well-formedness but bind nothing, because the unread
//! entity may have bound the same names first. [`Reader::declarations_unread`]
//! says so.

use std::ops::Range;
use std::sync::Arc;

use super::{
    AttDefault, AttlistDecl, Declarations, EntityDecl, EntityKind, ParameterDecl, Reader, is_space,
    normalize_line_ends, predefined,
};
use crate::terminals::{decode_char_ref, is_xml_name_char, is_xml_name_start_char};
use crate::xml::error::{XmlError, XmlErrorKind};
use crate::xml::reader::Dtd;

/// A parameter entity being read in the internal subset.
struct PeFrame {
    entity: usize,
    text: Arc<str>,
    pos: usize,
    /// The document offset of the reference that brought it in.
    offset: usize,
}

/// One item of the internal subset.
enum Step {
    /// A declaration, comment, or processing instruction, ending here.
    Next(usize),
    /// The end of the current text: the entity's replacement text, or (an
    /// error) the document.
    End,
    /// The `]` that closes the subset, and the offset past it.
    Close(usize),
    /// A parameter-entity reference: its name and the offset past the `;`.
    Reference { name: Range<usize>, next: usize },
}

/// Whether `b` is a `PubidChar`.
const fn pubid_char(b: u8) -> bool {
    matches!(b, b' ' | b'\r' | b'\n' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9')
        || matches!(
            b,
            b'-' | b'\''
                | b'('
                | b')'
                | b'+'
                | b','
                | b'.'
                | b'/'
                | b':'
                | b'='
                | b'?'
                | b';'
                | b'!'
                | b'*'
                | b'#'
                | b'@'
                | b'$'
                | b'_'
                | b'%'
        )
}

/// A group of a content model being read, and the separator it settled on.
struct Group {
    separator: Option<u8>,
}

impl Reader<'_> {
    /// Whether declarations no longer take effect (§5.1).
    fn stopped(&self) -> bool {
        self.dtd_read == Declarations::Stopped
    }

    /// The document type declaration at `pos`.
    pub(super) fn doctype(&mut self, pos: usize) -> Result<(), XmlError> {
        if self.options.dtd == Dtd::Refuse {
            return Err(XmlError::new(XmlErrorKind::Doctype, pos));
        }
        let src = self.src;
        let bytes = src.as_bytes();
        let mut at = self.require_space(bytes, pos + "<!DOCTYPE".len())?;
        at = self.name_end(src, at)?;
        let spaced = crate::terminals::skip_ws(bytes, at);
        if bytes[spaced..].starts_with(b"SYSTEM") || bytes[spaced..].starts_with(b"PUBLIC") {
            if spaced == at {
                return Err(XmlError::new(XmlErrorKind::Expected("whitespace"), at));
            }
            // An external subset is never fetched (§5.1).
            at = self.external_id(src, spaced, false)?;
            self.dtd_read = Declarations::Unread;
        }
        at = crate::terminals::skip_ws(bytes, at);
        if bytes.get(at) == Some(&b'[') {
            at = self.internal_subset(at + 1)?;
            at = crate::terminals::skip_ws(bytes, at);
        }
        if bytes.get(at) != Some(&b'>') {
            return Err(XmlError::new(XmlErrorKind::Expected("`>`"), at));
        }
        self.pos = at + 1;
        Ok(())
    }

    /// The internal subset whose text starts at `start`, returning the offset
    /// past its `]`.
    fn internal_subset(&mut self, start: usize) -> Result<usize, XmlError> {
        let src = self.src;
        let mut frames: Vec<PeFrame> = Vec::new();
        let mut doc_at = start;
        loop {
            let entity_text = frames.last().map(|f| Arc::clone(&f.text));
            let (text, in_doc): (&str, bool) = match &entity_text {
                Some(text) => (text, false),
                None => (src, true),
            };
            let from = frames.last().map_or(doc_at, |f| f.pos);
            let pending = self.dtd_pis.len();
            let step = match self.dtd_item(text, in_doc, from) {
                Ok(step) => step,
                Err(error) => return Err(self.dtd_error(&error, text, &frames)),
            };
            if let Some(first) = frames.first() {
                for pi in self.dtd_pis.iter_mut().skip(pending) {
                    pi.2 = first.offset;
                }
            }
            match step {
                Step::Next(next) => match frames.last_mut() {
                    Some(frame) => frame.pos = next,
                    None => doc_at = next,
                },
                Step::Close(next) => return Ok(next),
                Step::End => {
                    if frames.pop().is_none() {
                        return Err(XmlError::new(XmlErrorKind::UnexpectedEof, src.len()));
                    }
                }
                Step::Reference { name, next } => {
                    let offset = frames.first().map_or(name.start - 1, |f| f.offset);
                    match frames.last_mut() {
                        Some(frame) => frame.pos = next,
                        None => doc_at = next,
                    }
                    if self.dtd_read == Declarations::Complete {
                        self.dtd_read = Declarations::Referenced;
                    }
                    let name = &text[name];
                    match self
                        .parameter_entities
                        .iter()
                        .position(|e| &*e.name == name)
                    {
                        Some(entity) => {
                            let Some(replacement) = self.parameter_entities[entity].text.clone()
                            else {
                                // An external parameter entity: never read;
                                // what follows binds nothing (§5.1).
                                self.dtd_read = Declarations::Stopped;
                                continue;
                            };
                            if frames.iter().any(|f| f.entity == entity) {
                                let name = self.parameter_entities[entity].name.to_string();
                                return Err(XmlError::new(
                                    XmlErrorKind::RecursiveEntity(format!("%{name}")),
                                    offset,
                                ));
                            }
                            self.charge(replacement.len(), offset)?;
                            frames.push(PeFrame {
                                entity,
                                text: replacement,
                                pos: 0,
                                offset,
                            });
                        }
                        None if self.stopped() => {}
                        None => {
                            return Err(XmlError::new(
                                XmlErrorKind::UndeclaredParameterEntity(name.to_owned()),
                                offset,
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Turn an error from one item of the subset into what the document's
    /// reader reports: a `%` where a token belongs is the constraint "PEs in
    /// Internal Subset", an entity's end inside a declaration is "Proper
    /// Declaration/PE Nesting", and inside an entity the offset is that of the
    /// reference to the outermost one.
    fn dtd_error(&self, error: &XmlError, text: &str, frames: &[PeFrame]) -> XmlError {
        let mut kind = error.kind().clone();
        if matches!(kind, XmlErrorKind::Expected(_) | XmlErrorKind::InvalidName)
            && text.as_bytes().get(error.offset()) == Some(&b'%')
        {
            kind = XmlErrorKind::ParameterEntity;
        }
        let Some(first) = frames.first() else {
            return XmlError::new(kind, error.offset());
        };
        if error.offset() >= text.len()
            && let Some(last) = frames.last()
        {
            let name = self.parameter_entities[last.entity].name.to_string();
            kind = XmlErrorKind::UnbalancedEntity(format!("%{name}"));
        }
        XmlError::new(kind, first.offset)
    }

    /// The next item of the internal subset in `text`, from `from`.
    fn dtd_item(&mut self, text: &str, in_doc: bool, from: usize) -> Result<Step, XmlError> {
        let bytes = text.as_bytes();
        let at = crate::terminals::skip_ws(bytes, from);
        let Some(&first) = bytes.get(at) else {
            return Ok(Step::End);
        };
        let rest = &bytes[at..];
        match first {
            b']' if in_doc => Ok(Step::Close(at + 1)),
            b'%' => {
                let end = self.name_end(text, at + 1)?;
                if bytes.get(end) != Some(&b';') {
                    return Err(XmlError::new(XmlErrorKind::Expected("`;`"), end));
                }
                Ok(Step::Reference {
                    name: at + 1..end,
                    next: end + 1,
                })
            }
            b'<' if rest.starts_with(b"<!--") => {
                let (start, end) = self.comment_span(text, at)?;
                self.checked(text, in_doc, start, end)?;
                Ok(Step::Next(end + 3))
            }
            b'<' if rest.starts_with(b"<?") => {
                // Processing instructions must reach the application (§2.6),
                // the subset's included.
                let (target, data, end) = self.pi_parts(text, in_doc, at)?;
                self.dtd_pis.push_back((target, data, at));
                Ok(Step::Next(end))
            }
            b'<' if rest.starts_with(b"<!ENTITY") => {
                self.entity_decl(text, in_doc, at).map(Step::Next)
            }
            b'<' if rest.starts_with(b"<!ATTLIST") => {
                self.attlist_decl(text, in_doc, at).map(Step::Next)
            }
            b'<' if rest.starts_with(b"<!ELEMENT") => self.element_decl(text, at).map(Step::Next),
            b'<' if rest.starts_with(b"<!NOTATION") => self.notation_decl(text, at).map(Step::Next),
            b'<' if rest.starts_with(b"<![") => Err(XmlError::new(
                XmlErrorKind::Expected(
                    "a markup declaration (conditional sections belong to the external subset)",
                ),
                at,
            )),
            _ => Err(XmlError::new(
                XmlErrorKind::Expected("a markup declaration"),
                at,
            )),
        }
    }

    /// A quoted literal opening at `at`: the offsets of its body.
    fn quoted(&self, text: &str, at: usize) -> Result<(usize, usize), XmlError> {
        let bytes = text.as_bytes();
        let Some(&quote @ (b'"' | b'\'')) = bytes.get(at) else {
            return Err(XmlError::new(
                XmlErrorKind::Expected("a quoted literal"),
                at,
            ));
        };
        let close = bytes[at + 1..]
            .iter()
            .position(|&b| b == quote)
            .map(|end| at + 1 + end)
            .ok_or_else(|| XmlError::new(XmlErrorKind::UnexpectedEof, at))?;
        Ok((at + 1, close))
    }

    /// An `ExternalID` at `at` (`SYSTEM` or `PUBLIC`), returning its end. The
    /// system literal of a `PUBLIC` identifier is optional in a notation
    /// declaration (`PublicID`) and required everywhere else.
    fn external_id(&self, text: &str, at: usize, system_optional: bool) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let system = |this: &Self, at: usize| -> Result<usize, XmlError> {
            let (start, close) = this.quoted(text, at)?;
            this.checked(text, true, start, close)?;
            Ok(close + 1)
        };
        if bytes[at..].starts_with(b"SYSTEM") {
            let literal = self.require_space(bytes, at + "SYSTEM".len())?;
            return system(self, literal);
        }
        if !bytes[at..].starts_with(b"PUBLIC") {
            return Err(XmlError::new(
                XmlErrorKind::Expected("`SYSTEM` or `PUBLIC`"),
                at,
            ));
        }
        let literal = self.require_space(bytes, at + "PUBLIC".len())?;
        let (start, close) = self.quoted(text, literal)?;
        let quote = bytes[literal];
        if let Some(bad) = bytes[start..close]
            .iter()
            .position(|&b| !pubid_char(b) || b == quote)
        {
            return Err(XmlError::new(
                XmlErrorKind::Expected("a public identifier of `PubidChar`s"),
                start + bad,
            ));
        }
        let end = close + 1;
        if system_optional {
            let spaced = crate::terminals::skip_ws(bytes, end);
            if spaced > end && matches!(bytes.get(spaced), Some(b'"' | b'\'')) {
                return system(self, spaced);
            }
            return Ok(end);
        }
        let literal = self.require_space(bytes, end)?;
        system(self, literal)
    }

    /// A `NOTATION` declaration at `pos`, returning its end.
    fn notation_decl(&self, text: &str, pos: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut at = self.require_space(bytes, pos + "<!NOTATION".len())?;
        let name_end = self.name_end(text, at)?;
        if text[at..name_end].contains(':') {
            return Err(XmlError::new(XmlErrorKind::InvalidName, at));
        }
        at = self.require_space(bytes, name_end)?;
        at = self.external_id(text, at, true)?;
        at = crate::terminals::skip_ws(bytes, at);
        if bytes.get(at) != Some(&b'>') {
            return Err(XmlError::new(XmlErrorKind::Expected("`>`"), at));
        }
        Ok(at + 1)
    }

    /// An `ELEMENT` declaration at `pos`, returning its end.
    fn element_decl(&self, text: &str, pos: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut at = self.require_space(bytes, pos + "<!ELEMENT".len())?;
        at = self.name_end(text, at)?;
        at = self.require_space(bytes, at)?;
        at = if bytes[at..].starts_with(b"EMPTY") {
            at + "EMPTY".len()
        } else if bytes[at..].starts_with(b"ANY") {
            at + "ANY".len()
        } else if bytes.get(at) == Some(&b'(') {
            self.content_model(text, at)?
        } else {
            return Err(XmlError::new(
                XmlErrorKind::Expected("`EMPTY`, `ANY` or a content model"),
                at,
            ));
        };
        at = crate::terminals::skip_ws(bytes, at);
        if bytes.get(at) != Some(&b'>') {
            return Err(XmlError::new(XmlErrorKind::Expected("`>`"), at));
        }
        Ok(at + 1)
    }

    /// A `Mixed` or `children` content model whose `(` is at `at`, returning
    /// its end. Groups nest on a heap stack, not the call stack.
    fn content_model(&self, text: &str, at: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let inner = crate::terminals::skip_ws(bytes, at + 1);
        if bytes[inner..].starts_with(b"#PCDATA") {
            return self.mixed(text, inner + "#PCDATA".len());
        }
        let mut groups: Vec<Group> = Vec::new();
        let mut at = at;
        loop {
            // A `cp` begins here: a group, or a name.
            at = crate::terminals::skip_ws(bytes, at);
            if bytes.get(at) == Some(&b'(') {
                groups.push(Group { separator: None });
                at += 1;
                continue;
            }
            if groups.is_empty() {
                return Err(XmlError::new(
                    XmlErrorKind::Expected("a parenthesized content model"),
                    at,
                ));
            }
            at = self.name_end(text, at)?;
            // The `cp` (and every group it closes) is complete.
            loop {
                if matches!(bytes.get(at), Some(b'?' | b'*' | b'+')) {
                    at += 1;
                }
                at = crate::terminals::skip_ws(bytes, at);
                match bytes.get(at) {
                    Some(&separator @ (b',' | b'|')) => {
                        let group = groups.last_mut().expect("a group is open");
                        if *group.separator.get_or_insert(separator) != separator {
                            return Err(XmlError::new(
                                XmlErrorKind::Expected("one separator (`,` or `|`) in a group"),
                                at,
                            ));
                        }
                        at += 1;
                        break;
                    }
                    Some(b')') => {
                        groups.pop();
                        at += 1;
                        if groups.is_empty() {
                            if matches!(bytes.get(at), Some(b'?' | b'*' | b'+')) {
                                at += 1;
                            }
                            return Ok(at);
                        }
                    }
                    _ => {
                        return Err(XmlError::new(
                            XmlErrorKind::Expected("`,`, `|` or `)` in a content model"),
                            at,
                        ));
                    }
                }
            }
        }
    }

    /// The rest of a `Mixed` content model, after `#PCDATA`.
    fn mixed(&self, text: &str, mut at: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut names = 0;
        loop {
            at = crate::terminals::skip_ws(bytes, at);
            match bytes.get(at) {
                Some(b'|') => {
                    at = crate::terminals::skip_ws(bytes, at + 1);
                    at = self.name_end(text, at)?;
                    names += 1;
                }
                Some(b')') => {
                    at += 1;
                    if bytes.get(at) == Some(&b'*') {
                        return Ok(at + 1);
                    }
                    if names > 0 {
                        return Err(XmlError::new(
                            XmlErrorKind::Expected("`)*` after mixed content names"),
                            at,
                        ));
                    }
                    return Ok(at);
                }
                _ => {
                    return Err(XmlError::new(
                        XmlErrorKind::Expected("`|` or `)` in mixed content"),
                        at,
                    ));
                }
            }
        }
    }

    /// An `EntityValue` whose quote is at `at`: its replacement text (§4.5:
    /// character references expanded, general entity references kept for
    /// where the entity is used) and its end.
    fn entity_value(
        &self,
        text: &str,
        in_doc: bool,
        at: usize,
    ) -> Result<(String, usize), XmlError> {
        let (start, close) = self.quoted(text, at)?;
        let literal = self.checked(text, in_doc, start, close)?;
        let mut replacement = String::with_capacity(literal.len());
        let mut rest: &str = &literal;
        while let Some(amp) = rest.find(['&', '%']) {
            replacement.push_str(&rest[..amp]);
            if rest.as_bytes()[amp] == b'%' {
                // A parameter-entity reference in an entity value, in the
                // internal subset (constraint "PEs in Internal Subset").
                return Err(XmlError::new(XmlErrorKind::ParameterEntity, start));
            }
            let semi = rest[amp..]
                .find(';')
                .map(|s| amp + s)
                .ok_or_else(|| XmlError::new(XmlErrorKind::Expected("`;`"), start))?;
            let body = &rest[amp + 1..semi];
            if let Some(digits) = body.strip_prefix('#') {
                let c = decode_char_ref(digits.as_bytes())
                    .ok_or_else(|| XmlError::new(XmlErrorKind::InvalidCharRef, start))?;
                replacement.push(c);
            } else {
                let valid = body.chars().next().is_some_and(is_xml_name_start_char)
                    && body.chars().all(is_xml_name_char);
                if !valid {
                    return Err(XmlError::new(XmlErrorKind::InvalidName, start));
                }
                replacement.push_str(&rest[amp..=semi]);
            }
            rest = &rest[semi + 1..];
        }
        replacement.push_str(rest);
        Ok((replacement, close + 1))
    }

    /// An `ENTITY` declaration at `pos`, returning its end.
    fn entity_decl(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut at = self.require_space(bytes, pos + "<!ENTITY".len())?;
        let parameter = bytes.get(at) == Some(&b'%');
        if parameter {
            let after = at + 1;
            if !bytes.get(after).is_some_and(|&b| is_space(b)) {
                // `%name;` where the declared name belongs is a reference
                // (constraint "PEs in Internal Subset"), not a declaration.
                let is_reference = self
                    .name_end(text, after)
                    .is_ok_and(|end| bytes.get(end) == Some(&b';'));
                let kind = if is_reference {
                    XmlErrorKind::ParameterEntity
                } else {
                    XmlErrorKind::Expected("whitespace")
                };
                return Err(XmlError::new(kind, at));
            }
            at = self.require_space(bytes, after)?;
        }
        let name_end = self.name_end(text, at)?;
        let name = &text[at..name_end];
        if name.contains(':') {
            return Err(XmlError::new(XmlErrorKind::InvalidName, at));
        }
        at = self.require_space(bytes, name_end)?;
        let (kind, value) = if matches!(bytes.get(at), Some(b'"' | b'\'')) {
            let (value, end) = self.entity_value(text, in_doc, at)?;
            at = end;
            (EntityKind::Internal, Some(value))
        } else if bytes[at..].starts_with(b"SYSTEM") || bytes[at..].starts_with(b"PUBLIC") {
            at = self.external_id(text, at, false)?;
            let mut kind = EntityKind::External;
            let spaced = crate::terminals::skip_ws(bytes, at);
            if !parameter && spaced > at && bytes[spaced..].starts_with(b"NDATA") {
                let notation = self.require_space(bytes, spaced + "NDATA".len())?;
                at = self.name_end(text, notation)?;
                kind = EntityKind::Unparsed;
            }
            (kind, None)
        } else {
            return Err(XmlError::new(
                XmlErrorKind::Expected("an entity value or an external identifier"),
                at,
            ));
        };
        at = crate::terminals::skip_ws(bytes, at);
        if bytes.get(at) != Some(&b'>') {
            return Err(XmlError::new(XmlErrorKind::Expected("`>`"), at));
        }
        if self.stopped() {
            // §5.1: declarations after an unread external parameter entity
            // are checked, but an earlier, unread declaration may bind first.
            return Ok(at + 1);
        }
        if parameter {
            if !self.parameter_entities.iter().any(|e| &*e.name == name) {
                self.parameter_entities.push(ParameterDecl {
                    name: name.into(),
                    text: value.map(Arc::from),
                });
            }
            return Ok(at + 1);
        }
        if let Some(character) = predefined(name) {
            // §4.6: the predefined five may be declared, but only as the
            // character they escape (`lt` and `amp` doubly escaped).
            let one = |replacement: &str| {
                let mut chars = replacement.chars();
                matches!((chars.next(), chars.next()), (Some(c), None) if character.starts_with(c))
            };
            let reference = |replacement: &str| {
                replacement
                    .strip_prefix("&#")
                    .and_then(|r| r.strip_suffix(';'))
                    .and_then(|digits| decode_char_ref(digits.as_bytes()))
                    .is_some_and(|c| character.starts_with(c))
            };
            let lawful = value.as_deref().is_some_and(|replacement| {
                reference(replacement) || (name != "lt" && name != "amp" && one(replacement))
            });
            if !lawful {
                return Err(XmlError::new(
                    XmlErrorKind::PredefinedEntity(name.to_owned()),
                    pos,
                ));
            }
            return Ok(at + 1);
        }
        // The first declaration binds (§4.2).
        if !self.entities.iter().any(|e| &*e.name == name) {
            self.entities.push(EntityDecl {
                name: name.into(),
                text: Arc::from(value.unwrap_or_default()),
                kind,
            });
        }
        Ok(at + 1)
    }

    /// An `ATTLIST` declaration at `pos`, returning its end.
    fn attlist_decl(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut at = self.require_space(bytes, pos + "<!ATTLIST".len())?;
        let element_end = self.name_end(text, at)?;
        let element = &text[at..element_end];
        at = element_end;
        loop {
            let spaced = crate::terminals::skip_ws(bytes, at);
            match bytes.get(spaced) {
                Some(b'>') => return Ok(spaced + 1),
                None => return Err(XmlError::new(XmlErrorKind::UnexpectedEof, pos)),
                Some(_) if spaced == at => {
                    return Err(XmlError::new(XmlErrorKind::Expected("whitespace"), at));
                }
                Some(_) => {}
            }
            let name_end = self.name_end(text, spaced)?;
            let attribute = &text[spaced..name_end];
            at = self.require_space(bytes, name_end)?;
            let cdata = self.attribute_type(text, &mut at)?;
            at = self.require_space(bytes, at)?;
            let default = if bytes[at..].starts_with(b"#REQUIRED") {
                at += "#REQUIRED".len();
                None
            } else if bytes[at..].starts_with(b"#IMPLIED") {
                at += "#IMPLIED".len();
                None
            } else {
                if bytes[at..].starts_with(b"#FIXED") {
                    at = self.require_space(bytes, at + "#FIXED".len())?;
                }
                let (default, end) = self.attribute_default(text, in_doc, at)?;
                at = end;
                Some(default)
            };
            let known = self
                .attlists
                .iter()
                .any(|d| &*d.element == element && &*d.attribute == attribute);
            if !known && !self.stopped() {
                self.attlists.push(AttlistDecl {
                    element: element.into(),
                    attribute: attribute.into(),
                    cdata,
                    default,
                });
            }
        }
    }

    /// An `AttType` at `*at`, advancing it; whether the type is `CDATA`.
    fn attribute_type(&self, text: &str, at: &mut usize) -> Result<bool, XmlError> {
        let bytes = text.as_bytes();
        if bytes.get(*at) == Some(&b'(') {
            *at = self.enumeration(text, *at, false)?;
            return Ok(false);
        }
        let end = self.name_end(text, *at)?;
        let keyword = &text[*at..end];
        match keyword {
            "CDATA" => {
                *at = end;
                Ok(true)
            }
            "ID" | "IDREF" | "IDREFS" | "ENTITY" | "ENTITIES" | "NMTOKEN" | "NMTOKENS" => {
                *at = end;
                Ok(false)
            }
            "NOTATION" => {
                let open = self.require_space(bytes, end)?;
                if bytes.get(open) != Some(&b'(') {
                    return Err(XmlError::new(XmlErrorKind::Expected("`(`"), open));
                }
                *at = self.enumeration(text, open, true)?;
                Ok(false)
            }
            _ => Err(XmlError::new(
                XmlErrorKind::Expected("an attribute type"),
                *at,
            )),
        }
    }

    /// An enumeration whose `(` is at `at`: `Nmtoken`s, or `Name`s for a
    /// `NOTATION` type. Returns its end.
    fn enumeration(&self, text: &str, at: usize, names: bool) -> Result<usize, XmlError> {
        let bytes = text.as_bytes();
        let mut at = at + 1;
        loop {
            at = crate::terminals::skip_ws(bytes, at);
            at = if names {
                self.name_end(text, at)?
            } else {
                let rest = &text[at..];
                let len = rest
                    .find(|c: char| !is_xml_name_char(c))
                    .unwrap_or(rest.len());
                if len == 0 {
                    return Err(XmlError::new(XmlErrorKind::InvalidName, at));
                }
                at + len
            };
            at = crate::terminals::skip_ws(bytes, at);
            match bytes.get(at) {
                Some(b'|') => at += 1,
                Some(b')') => return Ok(at + 1),
                _ => {
                    return Err(XmlError::new(
                        XmlErrorKind::Expected("`|` or `)` in an enumeration"),
                        at,
                    ));
                }
            }
        }
    }

    /// An `AttValue` default whose quote is at `at`, checked here and kept as
    /// an [`AttDefault`], expanded now unless it refers to an entity only an
    /// unread declaration could bind.
    fn attribute_default(
        &mut self,
        text: &str,
        in_doc: bool,
        at: usize,
    ) -> Result<(AttDefault, usize), XmlError> {
        let (start, close) = self.quoted(text, at)?;
        let bytes = text.as_bytes();
        let mut entity_reference = false;
        let mut scan = start;
        while let Some(offset) = bytes[scan..close]
            .iter()
            .position(|&b| b == b'<' || b == b'&')
        {
            let hit = scan + offset;
            if bytes[hit] == b'<' {
                return Err(XmlError::new(XmlErrorKind::LessThanInAttributeValue, hit));
            }
            if bytes.get(hit + 1) == Some(&b'#') {
                let semi = bytes[hit..close]
                    .iter()
                    .position(|&b| b == b';')
                    .ok_or_else(|| XmlError::new(XmlErrorKind::InvalidCharRef, hit))?;
                decode_char_ref(&bytes[hit + 2..hit + semi])
                    .ok_or_else(|| XmlError::new(XmlErrorKind::InvalidCharRef, hit))?;
                scan = hit + semi + 1;
            } else {
                let end = self.name_end(text, hit + 1)?;
                if end >= close || bytes[end] != b';' {
                    return Err(XmlError::new(XmlErrorKind::Expected("`;`"), end));
                }
                entity_reference |= predefined(&text[hit + 1..end]).is_none();
                scan = end + 1;
            }
        }
        self.checked(text, in_doc, start, close)?;
        if entity_reference {
            // The entities a default refers to are declared before it (§4.1:
            // "Entity Declared"), and expanded now; a reference only an unread
            // declaration could bind is kept for where the default applies.
            if !self.stopped() {
                match self.attribute_value(text, in_doc, at) {
                    Ok((value, end)) => {
                        return Ok((AttDefault::Ready(value.into_owned().into_boxed_str()), end));
                    }
                    Err(error) if !matches!(error.kind(), XmlErrorKind::UnexpandedEntity(_)) => {
                        return Err(error);
                    }
                    Err(_) => {}
                }
            }
            let literal = &text[at..=close];
            let literal = if literal.contains('\r') {
                normalize_line_ends(literal)
            } else {
                literal.to_owned()
            };
            return Ok((AttDefault::Deferred(literal.into_boxed_str()), close + 1));
        }
        let (value, end) = self.attribute_value(text, in_doc, at)?;
        Ok((AttDefault::Ready(value.into_owned().into_boxed_str()), end))
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pull reader: one event per call, in document order, over an explicit
//! element stack and an explicit entity-expansion stack.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::sync::Arc;

mod dtd;

use super::error::{XmlError, XmlErrorKind};
use crate::terminals::{
    ByteClass, byte_run_count, decode_char_ref, is_ncname, is_xml_char, is_xml_name_char,
    is_xml_name_start_char,
};

/// The namespace name the `xml` prefix is bound to by definition
/// (Namespaces in XML 1.0 §3). Spelled here because the XML reader sits below
/// `purrdf-iri`, whose vocabulary constants every crate above uses.
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// The namespace name of the `xmlns` prefix, which no declaration may bind
/// (Namespaces in XML 1.0 §3).
pub const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

/// The element nesting [`Options::default`] allows.
///
/// The reader itself holds its nesting on the heap and is indifferent to
/// depth; the cap exists for the callers, which walk the tree it produces. 128
/// is the nesting envelope the workspace's document parsers publish for every
/// syntax (the text codecs and the JSON-LD reader refuse deeper input too),
/// and it sits well below the depth at which a recursive walk over the tree
/// exhausts the smallest supported stack (wasm32's 1 MiB).
pub const DEFAULT_MAX_DEPTH: usize = 128;

/// The replacement-text budget [`Dtd::internal_subset`] allows: 1 MiB of text
/// produced by expanding internal entities (general and parameter), across the
/// whole document.
pub const DEFAULT_MAX_ENTITY_EXPANSION: usize = 1 << 20;

/// What the reader does with a document type declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dtd {
    /// Refuse any `<!DOCTYPE`, with [`XmlErrorKind::Doctype`].
    Refuse,
    /// Read the internal subset as XML 1.0 §5.1 requires of a non-validating
    /// processor: every markup declaration is checked for well-formedness;
    /// internal general entities are declared and expanded (in content,
    /// markup included, and in attribute values); internal parameter entities
    /// are declared and expanded between the subset's declarations (a
    /// reference inside a declaration is refused, as XML 1.0 requires); and
    /// `ATTLIST` defaults and non-`CDATA` normalization are applied. Nothing
    /// is ever fetched: a document may name an external subset, and may
    /// declare and reference external parameter entities, but the reader does
    /// not read them ([`Reader::declarations_unread`]; after such a reference
    /// the remaining declarations are checked but bind nothing, §5.1), and a
    /// reference to an external or unparsed general entity is refused
    /// ([`XmlErrorKind::ExternalEntity`], [`XmlErrorKind::UnparsedEntity`]).
    /// `max_expansion` bounds the total replacement text the document may
    /// expand, general and parameter entities alike, so a nested-entity bomb
    /// is refused rather than allocated.
    InternalSubset {
        /// The most replacement-text bytes the whole document may expand.
        max_expansion: usize,
    },
}

impl Dtd {
    /// [`Dtd::InternalSubset`] with the [`DEFAULT_MAX_ENTITY_EXPANSION`] budget.
    #[must_use]
    pub const fn internal_subset() -> Self {
        Self::InternalSubset {
            max_expansion: DEFAULT_MAX_ENTITY_EXPANSION,
        }
    }
}

/// How a document is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The deepest element nesting accepted: the root element is depth 1, and
    /// an element at depth `max_depth + 1` is refused with
    /// [`XmlErrorKind::DepthLimit`].
    pub max_depth: usize,
    /// What to do with a document type declaration.
    pub dtd: Dtd,
}

impl Default for Options {
    /// [`DEFAULT_MAX_DEPTH`], and [`Dtd::Refuse`].
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            dtd: Dtd::Refuse,
        }
    }
}

/// A namespace-well-formed qualified name, `prefix:local` or `local`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QName<'a> {
    text: Cow<'a, str>,
    colon: Option<usize>,
}

impl<'a> QName<'a> {
    /// The name as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The prefix, when the name has one.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.colon.map(|colon| &self.text[..colon])
    }

    /// The local part: the whole name when it has no prefix.
    #[must_use]
    pub fn local(&self) -> &str {
        self.colon
            .map_or(&*self.text, |colon| &self.text[colon + 1..])
    }

    /// Validate `text` as a `QName` (Namespaces in XML 1.0 §4): at most one
    /// colon, with an `NCName` on each side.
    fn new(text: Cow<'a, str>) -> Option<Self> {
        let colon = text.find(':');
        let valid = match colon {
            None => is_ncname(&text),
            Some(at) => is_ncname(&text[..at]) && is_ncname(&text[at + 1..]),
        };
        valid.then_some(Self { text, colon })
    }
}

/// One attribute of a start tag, its value normalized per XML 1.0 §3.3.3 and
/// its name resolved per Namespaces in XML 1.0 §6.3: a prefixed name takes
/// its prefix's namespace, an unprefixed name has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute<'a> {
    name: QName<'a>,
    namespace: Option<Cow<'a, str>>,
    value: Cow<'a, str>,
    offset: usize,
}

impl Attribute<'_> {
    /// The name as written.
    #[must_use]
    pub fn qname(&self) -> &str {
        self.name.as_str()
    }

    /// The prefix, when the name has one.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.name.prefix()
    }

    /// The local name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.local()
    }

    /// The namespace name, when the attribute is prefixed.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// The normalized value, every reference expanded.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The byte offset of the attribute's name in the source, or of the
    /// reference that brought it in from an entity or a default.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }
}

/// A namespace declaration: `xmlns="uri"` (no prefix) or `xmlns:p="uri"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamespaceDecl<'a> {
    prefix: Option<Cow<'a, str>>,
    uri: Cow<'a, str>,
}

impl NamespaceDecl<'_> {
    /// The declared prefix, or `None` for the default namespace.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// The namespace name; empty for `xmlns=""`, which undeclares the default
    /// namespace.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// The XML declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration<'a> {
    /// `VersionNum`: always `1.0`, the one version the reader reads.
    pub version: Cow<'a, str>,
    /// The `EncName`, when declared.
    pub encoding: Option<Cow<'a, str>>,
    /// The standalone document declaration, when present.
    pub standalone: Option<bool>,
}

/// A start tag, borrowed from the reader until the next call.
#[derive(Debug)]
pub struct StartTag<'r, 'a> {
    /// The element name.
    pub name: &'r QName<'a>,
    /// The element's namespace name: its prefix's, or the default namespace
    /// when unprefixed.
    pub namespace: Option<&'r Cow<'a, str>>,
    /// The attributes, in document order, then any `ATTLIST` defaults;
    /// namespace declarations are not among them.
    pub attributes: &'r [Attribute<'a>],
    /// The namespace declarations this start tag makes, in document order.
    pub namespace_declarations: &'r [NamespaceDecl<'a>],
    /// Whether the tag is an empty-element tag. An [`Event::End`] follows it
    /// either way.
    pub self_closing: bool,
    /// The element's depth: 1 for the root element.
    pub depth: usize,
    /// The byte offset of the `<`.
    pub offset: usize,
}

/// An end tag (or the end of an empty-element tag).
#[derive(Debug)]
pub struct EndTag<'r, 'a> {
    /// The element name.
    pub name: &'r QName<'a>,
    /// The element's namespace name.
    pub namespace: Option<&'r Cow<'a, str>>,
    /// The byte offset of the `</`, or of the empty-element tag's `<`.
    pub offset: usize,
}

/// One reader event.
#[derive(Debug)]
pub enum Event<'r, 'a> {
    /// The XML declaration, when the document opens with one.
    Declaration(Declaration<'a>),
    /// A start tag.
    Start(StartTag<'r, 'a>),
    /// An end tag.
    End(EndTag<'r, 'a>),
    /// Character data, every reference expanded and line ends normalized.
    /// Adjacent runs are not merged: a reference or an entity boundary may
    /// split one run of text into several events.
    Text {
        /// The text.
        text: Cow<'a, str>,
        /// The byte offset of the run.
        offset: usize,
    },
    /// The content of a CDATA section.
    CData {
        /// The text.
        text: Cow<'a, str>,
        /// The byte offset of `<![CDATA[`.
        offset: usize,
    },
    /// A comment's text.
    Comment {
        /// The text between `<!--` and `-->`.
        text: Cow<'a, str>,
        /// The byte offset of `<!--`.
        offset: usize,
    },
    /// A processing instruction.
    Pi {
        /// The target.
        target: Cow<'a, str>,
        /// The data after the target's whitespace, when present.
        data: Option<Cow<'a, str>>,
        /// The byte offset of `<?`.
        offset: usize,
    },
    /// The end of the document; every later call returns it again.
    Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Start,
    Prolog,
    Content,
    Epilog,
    Done,
}

/// How much of a document's declarations the reader has read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Declarations {
    /// No document type declaration, or an internal subset without
    /// parameter-entity references: every declaration is known, so an
    /// undeclared entity is a well-formedness error.
    Complete,
    /// The internal subset references an internal parameter entity: all
    /// declarations were read, but XML 1.0 §4.1 makes an undeclared entity
    /// reference a mere validity error.
    Referenced,
    /// The document type declaration names an external subset, which is never
    /// read.
    Unread,
    /// An external parameter entity was referenced in the internal subset
    /// (§5.1): what follows is checked but binds nothing.
    Stopped,
}

/// What an entity declaration declares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EntityKind {
    /// An internal entity: replacement text the reader expands.
    Internal,
    /// An external parsed entity: never fetched, so a reference is refused.
    External,
    /// An unparsed entity (`NDATA`): a reference is a well-formedness error.
    Unparsed,
}

struct EntityDecl {
    name: Box<str>,
    text: Arc<str>,
    kind: EntityKind,
}

/// A declared parameter entity; `text` is `None` for an external one.
struct ParameterDecl {
    name: Box<str>,
    text: Option<Arc<str>>,
}

/// An `ATTLIST` default.
enum AttDefault {
    /// Expanded already.
    Ready(Box<str>),
    /// The literal, quotes included, line ends normalized, holding a
    /// reference to an entity only a declaration the reader has not read could
    /// bind: expanded (and so refused) if the default is ever applied.
    Deferred(Box<str>),
}

struct AttlistDecl {
    element: Box<str>,
    attribute: Box<str>,
    cdata: bool,
    default: Option<AttDefault>,
}

/// An entity being expanded in content.
struct Frame {
    entity: usize,
    text: Arc<str>,
    pos: usize,
    /// The element depth when the reference was read: the replacement text
    /// must close exactly what it opens.
    depth: usize,
    /// The offset of the `&` in the document (or in the enclosing frame's
    /// document reference).
    offset: usize,
}

#[derive(Debug)]
struct Open<'a> {
    name: QName<'a>,
    namespace: Option<Cow<'a, str>>,
    /// `bindings.len()` before this element's declarations.
    mark: usize,
    offset: usize,
}

/// Bytes that end or interrupt a run of character data: markup, a reference,
/// `]` (a possible `]]>`), CARRIAGE RETURN (a line end to normalize), the C0
/// controls that are not `Char`, and `0xEF`, the lead of U+FFFE and U+FFFF.
const TEXT_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'\t' as usize] = 0;
    table[b'\n' as usize] = 0;
    table[b'<' as usize] = 1;
    table[b'&' as usize] = 1;
    table[b']' as usize] = 1;
    table[0xEF] = 1;
    table
};

/// Bytes that end or interrupt an attribute value: both quotes, `<`, `&`, the
/// three whitespace characters normalized to SPACE, the other C0 controls,
/// and `0xEF`.
const ATTR_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'"' as usize] = 1;
    table[b'\'' as usize] = 1;
    table[b'<' as usize] = 1;
    table[b'&' as usize] = 1;
    table[0xEF] = 1;
    table
};

/// Bytes a comment, PI, CDATA section or entity value must check: CARRIAGE
/// RETURN, the C0 controls that are not `Char`, and `0xEF`.
const CHAR_TABLE: [u8; 256] = {
    let mut table = [0_u8; 256];
    let mut b = 0;
    while b < 0x20 {
        table[b] = 1;
        b += 1;
    }
    table[b'\t' as usize] = 0;
    table[b'\n' as usize] = 0;
    table[0xEF] = 1;
    table
};

const TEXT: ByteClass<{ byte_run_count(&TEXT_TABLE) }> = ByteClass::from_table(TEXT_TABLE);
const ATTR: ByteClass<{ byte_run_count(&ATTR_TABLE) }> = ByteClass::from_table(ATTR_TABLE);
const CHARS: ByteClass<{ byte_run_count(&CHAR_TABLE) }> = ByteClass::from_table(CHAR_TABLE);

/// XML 1.0 `S`.
const fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// The scalar outside `Char` that begins at `bytes[at]`, a stop of one of the
/// classes above that is neither markup nor whitespace, or `None` when the
/// scalar there is lawful.
fn illegal_at(text: &str, at: usize) -> Option<char> {
    let c = text[at..].chars().next()?;
    (!is_xml_char(c)).then_some(c)
}

/// XML 1.0 §2.11 line-end normalization of a run that holds a CARRIAGE RETURN.
fn normalize_line_ends(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('\r') {
        out.push_str(&rest[..at]);
        out.push('\n');
        rest = &rest[at + 1..];
        if let Some(tail) = rest.strip_prefix('\n') {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// A slice of the current input as a `Cow` of the document: borrowed when the
/// input is the document itself, owned when it is an entity's replacement
/// text.
fn slice<'a>(doc: &'a str, text: &str, in_doc: bool, start: usize, end: usize) -> Cow<'a, str> {
    if in_doc {
        Cow::Borrowed(&doc[start..end])
    } else {
        Cow::Owned(text[start..end].to_owned())
    }
}

/// A processing instruction's target, data and end offset.
type PiParts<'a> = (Cow<'a, str>, Option<Cow<'a, str>>, usize);

/// Where a reference in text or an attribute value leads.
enum Reference {
    Char(char),
    Predefined(&'static str),
    Entity(usize),
}

/// The five entities XML 1.0 §4.6 predefines.
fn predefined(name: &str) -> Option<&'static str> {
    Some(match name {
        "lt" => "<",
        "gt" => ">",
        "amp" => "&",
        "apos" => "'",
        "quot" => "\"",
        _ => return None,
    })
}

/// The pull reader over one document. See the [module docs](super).
pub struct Reader<'a> {
    src: &'a str,
    pos: usize,
    options: Options,
    phase: Phase,
    doctype_seen: bool,
    entities: Vec<EntityDecl>,
    parameter_entities: Vec<ParameterDecl>,
    attlists: Vec<AttlistDecl>,
    /// Processing instructions of the internal subset, read but not yet
    /// delivered: target, data and offset.
    dtd_pis: VecDeque<PiParts<'a>>,
    /// The document's `standalone="yes"`.
    standalone: bool,
    /// What the reader has read of the declarations.
    dtd_read: Declarations,
    frames: Vec<Frame>,
    expanded: usize,
    open: Vec<Open<'a>>,
    bindings: Vec<NamespaceDecl<'a>>,
    attributes: Vec<Attribute<'a>>,
    declarations: Vec<NamespaceDecl<'a>>,
    closed: Option<Open<'a>>,
    pending_end: bool,
}

impl core::fmt::Debug for Reader<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Reader")
            .field("pos", &self.pos)
            .field("phase", &self.phase)
            .field("depth", &self.open.len())
            .field("entity_depth", &self.frames.len())
            .finish_non_exhaustive()
    }
}

impl<'a> Reader<'a> {
    /// A reader over `text` with [`Options::default`].
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self::with_options(text, Options::default())
    }

    /// A reader over `text`.
    #[must_use]
    pub const fn with_options(text: &'a str, options: Options) -> Self {
        Self {
            src: text,
            pos: 0,
            options,
            phase: Phase::Start,
            doctype_seen: false,
            entities: Vec::new(),
            parameter_entities: Vec::new(),
            attlists: Vec::new(),
            dtd_pis: VecDeque::new(),
            standalone: false,
            dtd_read: Declarations::Complete,
            frames: Vec::new(),
            expanded: 0,
            open: Vec::new(),
            bindings: Vec::new(),
            attributes: Vec::new(),
            declarations: Vec::new(),
            closed: None,
            pending_end: false,
        }
    }

    /// The element depth: the number of open elements.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.open.len()
    }

    /// Whether declarations the reader never read may exist: the document type
    /// declaration names an external subset, or its internal subset references
    /// an external parameter entity (XML 1.0 §5.1). Entity declarations and
    /// attribute defaults there do not take effect, and a reference to an
    /// entity only they could declare is refused
    /// ([`XmlErrorKind::UnexpandedEntity`]). A caller that needs the complete
    /// information set refuses such a document.
    #[must_use]
    pub const fn declarations_unread(&self) -> bool {
        matches!(self.dtd_read, Declarations::Unread | Declarations::Stopped)
    }

    /// The namespace name `prefix` resolves to at the current position
    /// (`None` for the default namespace), or `None` when it is unbound.
    #[must_use]
    pub fn lookup_namespace(&self, prefix: Option<&str>) -> Option<&str> {
        lookup(&self.bindings, prefix)
    }

    /// The error for `kind` at `pos` in the current input: inside an entity,
    /// the offset of the document's reference to the outermost entity.
    fn error(&self, kind: XmlErrorKind, pos: usize) -> XmlError {
        XmlError::new(kind, self.frames.first().map_or(pos, |frame| frame.offset))
    }

    /// The offset an event at `pos` in the current input reports.
    fn offset(&self, pos: usize) -> usize {
        self.frames.first().map_or(pos, |frame| frame.offset)
    }

    /// The current input and whether it is the document.
    fn input(&self) -> (Option<Arc<str>>, usize) {
        match self.frames.last() {
            None => (None, self.pos),
            Some(frame) => (Some(Arc::clone(&frame.text)), frame.pos),
        }
    }

    fn set_pos(&mut self, pos: usize) {
        match self.frames.last_mut() {
            None => self.pos = pos,
            Some(frame) => frame.pos = pos,
        }
    }

    /// The next event.
    ///
    /// # Errors
    ///
    /// [`XmlError`] at the first well-formedness or namespace violation, or
    /// the first limit exceeded. The reader is not usable after an error.
    #[allow(
        clippy::should_implement_trait,
        reason = "each event borrows the reader until the next call, which `Iterator` cannot express"
    )]
    pub fn next(&mut self) -> Result<Event<'_, 'a>, XmlError> {
        if self.pending_end {
            self.pending_end = false;
            return Ok(self.close_element());
        }
        if self.phase == Phase::Start {
            self.phase = Phase::Prolog;
            if self.src.starts_with('\u{FEFF}') {
                self.pos = '\u{FEFF}'.len_utf8();
            }
            let rest = &self.src.as_bytes()[self.pos..];
            if rest.starts_with(b"<?xml") && rest.get(5).is_some_and(|&b| is_space(b)) {
                return self.declaration().map(Event::Declaration);
            }
        }
        loop {
            if let Some((target, data, offset)) = self.dtd_pis.pop_front() {
                return Ok(Event::Pi {
                    target,
                    data,
                    offset,
                });
            }
            if self.phase == Phase::Done {
                return Ok(Event::Eof);
            }
            let (entity_text, pos) = self.input();
            let in_doc = entity_text.is_none();
            let text: &str = entity_text.as_deref().unwrap_or(self.src);
            let bytes = text.as_bytes();
            if pos >= bytes.len() {
                if let Some(frame) = self.frames.last() {
                    if self.open.len() != frame.depth {
                        let name = self.entities[frame.entity].name.to_string();
                        return Err(self.error(XmlErrorKind::UnbalancedEntity(name), pos));
                    }
                    self.frames.pop();
                    continue;
                }
                return match self.phase {
                    Phase::Prolog => Err(self.error(XmlErrorKind::NoRootElement, pos)),
                    Phase::Content => Err(self.error(XmlErrorKind::UnexpectedEof, pos)),
                    _ => {
                        self.phase = Phase::Done;
                        Ok(Event::Eof)
                    }
                };
            }
            let rest = &bytes[pos..];
            if self.phase != Phase::Content {
                // Prolog and epilog: `Misc*` — whitespace, comments and PIs —
                // plus the doctype and the one root element.
                if is_space(rest[0]) {
                    let end = crate::terminals::skip_ws(bytes, pos);
                    self.set_pos(end);
                    continue;
                }
                if rest.starts_with(b"<!--") {
                    return self.comment(text, in_doc, pos);
                }
                if rest.starts_with(b"<?") {
                    return self.pi(text, in_doc, pos);
                }
                if rest.starts_with(b"<!DOCTYPE") {
                    if self.phase != Phase::Prolog || self.doctype_seen {
                        return Err(self.error(XmlErrorKind::MisplacedDoctype, pos));
                    }
                    self.doctype_seen = true;
                    self.doctype(pos)?;
                    continue;
                }
                if rest[0] != b'<'
                    || rest.starts_with(b"<!")
                    || rest.starts_with(b"</")
                    || self.phase == Phase::Epilog
                {
                    return Err(self.error(XmlErrorKind::ContentOutsideRoot, pos));
                }
                self.phase = Phase::Content;
                return self.start_tag(text, in_doc, pos);
            }
            match rest[0] {
                b'<' => {
                    if rest.starts_with(b"</") {
                        return self.end_tag(text, pos);
                    }
                    if rest.starts_with(b"<!--") {
                        return self.comment(text, in_doc, pos);
                    }
                    if rest.starts_with(b"<![CDATA[") {
                        return self.cdata(text, in_doc, pos);
                    }
                    if rest.starts_with(b"<?") {
                        return self.pi(text, in_doc, pos);
                    }
                    if rest.starts_with(b"<!") {
                        return Err(self.error(XmlErrorKind::Expected("an element"), pos));
                    }
                    return self.start_tag(text, in_doc, pos);
                }
                b'&' => {
                    let (reference, end) = self.reference(text, pos)?;
                    self.set_pos(end);
                    let offset = self.offset(pos);
                    match reference {
                        Reference::Char(c) => {
                            return Ok(Event::Text {
                                text: Cow::Owned(c.to_string()),
                                offset,
                            });
                        }
                        Reference::Predefined(value) => {
                            return Ok(Event::Text {
                                text: Cow::Borrowed(value),
                                offset,
                            });
                        }
                        Reference::Entity(entity) => self.enter_entity(entity, pos)?,
                    }
                }
                _ => return self.text(text, in_doc, pos),
            }
        }
    }

    /// Pop the element whose end was just read and lend it as the end event.
    fn close_element(&mut self) -> Event<'_, 'a> {
        let open = self.open.pop().expect("an element is open");
        self.bindings.truncate(open.mark);
        if self.open.is_empty() && self.frames.is_empty() {
            self.phase = Phase::Epilog;
        }
        let offset = open.offset;
        let closed = self.closed.insert(open);
        Event::End(EndTag {
            name: &closed.name,
            namespace: closed.namespace.as_ref(),
            offset,
        })
    }

    /// Begin expanding `entity`, referenced at `pos`, in content.
    fn enter_entity(&mut self, entity: usize, pos: usize) -> Result<(), XmlError> {
        if self.frames.iter().any(|frame| frame.entity == entity) {
            let name = self.entities[entity].name.to_string();
            return Err(self.error(XmlErrorKind::RecursiveEntity(name), pos));
        }
        let text = Arc::clone(&self.entities[entity].text);
        self.charge(text.len(), pos)?;
        let offset = self.offset(pos);
        self.frames.push(Frame {
            entity,
            text,
            pos: 0,
            depth: self.open.len(),
            offset,
        });
        Ok(())
    }

    /// Count `len` bytes of replacement text against the expansion budget.
    fn charge(&mut self, len: usize, pos: usize) -> Result<(), XmlError> {
        let Dtd::InternalSubset { max_expansion } = self.options.dtd else {
            unreachable!("entities are declared only under an internal subset");
        };
        self.expanded = self.expanded.saturating_add(len);
        if self.expanded > max_expansion {
            return Err(self.error(
                XmlErrorKind::EntityExpansionLimit {
                    limit: max_expansion,
                },
                pos,
            ));
        }
        Ok(())
    }

    /// Read a `Name` at `pos`, returning its end.
    fn name_end(&self, text: &str, pos: usize) -> Result<usize, XmlError> {
        let mut chars = text[pos..].char_indices();
        match chars.next() {
            Some((_, c)) if is_xml_name_start_char(c) => {}
            Some(_) => return Err(self.error(XmlErrorKind::InvalidName, pos)),
            None => return Err(self.error(XmlErrorKind::UnexpectedEof, pos)),
        }
        for (at, c) in chars {
            if !is_xml_name_char(c) {
                return Ok(pos + at);
            }
        }
        Ok(text.len())
    }

    /// Read a `QName` at `pos`, returning it and its end.
    fn qname(&self, text: &str, in_doc: bool, pos: usize) -> Result<(QName<'a>, usize), XmlError> {
        let end = self.name_end(text, pos)?;
        let name = QName::new(slice(self.src, text, in_doc, pos, end))
            .ok_or_else(|| self.error(XmlErrorKind::InvalidName, pos))?;
        Ok((name, end))
    }

    /// Read the reference at `pos` (an `&`), returning where it leads and its
    /// end.
    fn reference(&self, text: &str, pos: usize) -> Result<(Reference, usize), XmlError> {
        let bytes = text.as_bytes();
        if bytes.get(pos + 1) == Some(&b'#') {
            let body_start = pos + 2;
            let semi = bytes[body_start..]
                .iter()
                .position(|&b| b == b';')
                .ok_or_else(|| self.error(XmlErrorKind::InvalidCharRef, pos))?;
            let c = decode_char_ref(&bytes[body_start..body_start + semi])
                .ok_or_else(|| self.error(XmlErrorKind::InvalidCharRef, pos))?;
            return Ok((Reference::Char(c), body_start + semi + 1));
        }
        let end = self.name_end(text, pos + 1)?;
        if bytes.get(end) != Some(&b';') {
            return Err(self.error(XmlErrorKind::Expected("`;`"), end));
        }
        let name = &text[pos + 1..end];
        if let Some(value) = predefined(name) {
            return Ok((Reference::Predefined(value), end + 1));
        }
        match self.entities.iter().position(|e| &*e.name == name) {
            Some(entity) => match self.entities[entity].kind {
                EntityKind::Internal => Ok((Reference::Entity(entity), end + 1)),
                EntityKind::External => Err(self.error(XmlErrorKind::ExternalEntity, pos)),
                EntityKind::Unparsed => {
                    Err(self.error(XmlErrorKind::UnparsedEntity(name.to_owned()), pos))
                }
            },
            None => {
                // XML 1.0 §4.1: an entity reference must match a declaration
                // unless declarations the reader has not read could hold it.
                let kind = if self.dtd_read != Declarations::Complete && !self.standalone {
                    XmlErrorKind::UnexpandedEntity(name.to_owned())
                } else {
                    XmlErrorKind::UndeclaredEntity(name.to_owned())
                };
                Err(self.error(kind, pos))
            }
        }
    }

    /// Read a run of character data at `pos`.
    fn text(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<Event<'_, 'a>, XmlError> {
        let bytes = text.as_bytes();
        let mut at = pos;
        let mut carriage_return = false;
        let end = loop {
            let Some(offset) = TEXT.find_first(&bytes[at..]) else {
                break bytes.len();
            };
            let hit = at + offset;
            match bytes[hit] {
                b'<' | b'&' => break hit,
                b']' => {
                    if bytes[hit..].starts_with(b"]]>") {
                        return Err(self.error(XmlErrorKind::CdataEndInText, hit));
                    }
                }
                b'\r' => carriage_return = true,
                _ => {
                    if let Some(c) = illegal_at(text, hit) {
                        return Err(self.error(XmlErrorKind::IllegalChar(c), hit));
                    }
                }
            }
            at = hit + 1;
        };
        self.set_pos(end);
        let run = if carriage_return && in_doc {
            Cow::Owned(normalize_line_ends(&text[pos..end]))
        } else {
            slice(self.src, text, in_doc, pos, end)
        };
        Ok(Event::Text {
            text: run,
            offset: self.offset(pos),
        })
    }

    /// Check `text[start..end]` for scalars outside `Char` and normalize its
    /// line ends when it comes from the document.
    fn checked(
        &self,
        text: &str,
        in_doc: bool,
        start: usize,
        end: usize,
    ) -> Result<Cow<'a, str>, XmlError> {
        let bytes = &text.as_bytes()[..end];
        let mut at = start;
        let mut carriage_return = false;
        while let Some(offset) = CHARS.find_first(&bytes[at..]) {
            let hit = at + offset;
            if bytes[hit] == b'\r' {
                carriage_return = true;
            } else if let Some(c) = illegal_at(text, hit) {
                return Err(self.error(XmlErrorKind::IllegalChar(c), hit));
            }
            at = hit + 1;
        }
        Ok(if carriage_return && in_doc {
            Cow::Owned(normalize_line_ends(&text[start..end]))
        } else {
            slice(self.src, text, in_doc, start, end)
        })
    }

    /// Find `needle` at or after `from`, or refuse the unterminated construct.
    fn find(&self, text: &str, from: usize, needle: &str, pos: usize) -> Result<usize, XmlError> {
        text[from..]
            .find(needle)
            .map(|at| from + at)
            .ok_or_else(|| self.error(XmlErrorKind::UnexpectedEof, pos))
    }

    fn comment(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<Event<'_, 'a>, XmlError> {
        let (start, end) = self.comment_span(text, pos)?;
        let body = self.checked(text, in_doc, start, end)?;
        self.set_pos(end + 3);
        Ok(Event::Comment {
            text: body,
            offset: self.offset(pos),
        })
    }

    /// The body of the comment at `pos`, checked for `--`.
    fn comment_span(&self, text: &str, pos: usize) -> Result<(usize, usize), XmlError> {
        let start = pos + 4;
        let dashes = self.find(text, start, "--", pos)?;
        if !text[dashes..].starts_with("-->") {
            return Err(self.error(XmlErrorKind::InvalidComment, dashes));
        }
        if dashes > start && text.as_bytes()[dashes - 1] == b'-' {
            return Err(self.error(XmlErrorKind::InvalidComment, dashes - 1));
        }
        Ok((start, dashes))
    }

    fn pi(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<Event<'_, 'a>, XmlError> {
        let (target, data, end) = self.pi_parts(text, in_doc, pos)?;
        self.set_pos(end);
        Ok(Event::Pi {
            target,
            data,
            offset: self.offset(pos),
        })
    }

    /// The processing instruction at `pos`: target, data and end.
    fn pi_parts(&self, text: &str, in_doc: bool, pos: usize) -> Result<PiParts<'a>, XmlError> {
        let start = pos + 2;
        let target_end = self.name_end(text, start)?;
        let target = &text[start..target_end];
        if target.eq_ignore_ascii_case("xml") {
            let kind = if target == "xml" {
                XmlErrorKind::MisplacedXmlDeclaration
            } else {
                XmlErrorKind::ReservedPiTarget
            };
            return Err(self.error(kind, pos));
        }
        if target.contains(':') {
            return Err(self.error(XmlErrorKind::ReservedPiTarget, pos));
        }
        let close = self.find(text, target_end, "?>", pos)?;
        let data = if close == target_end {
            None
        } else {
            if !is_space(text.as_bytes()[target_end]) {
                return Err(self.error(XmlErrorKind::Expected("whitespace"), target_end));
            }
            let data_start = crate::terminals::skip_ws(text.as_bytes(), target_end).min(close);
            Some(self.checked(text, in_doc, data_start, close)?)
        };
        Ok((
            slice(self.src, text, in_doc, start, target_end),
            data,
            close + 2,
        ))
    }

    fn cdata(&mut self, text: &str, in_doc: bool, pos: usize) -> Result<Event<'_, 'a>, XmlError> {
        let start = pos + "<![CDATA[".len();
        let end = self.find(text, start, "]]>", pos)?;
        let body = self.checked(text, in_doc, start, end)?;
        self.set_pos(end + 3);
        Ok(Event::CData {
            text: body,
            offset: self.offset(pos),
        })
    }

    /// The XML declaration, at the start of the document.
    fn declaration(&mut self) -> Result<Declaration<'a>, XmlError> {
        let src = self.src;
        let bytes = src.as_bytes();
        let start = self.pos;
        let bad = |at: usize| XmlError::new(XmlErrorKind::InvalidXmlDeclaration, at);
        let close = src[start..]
            .find("?>")
            .map(|at| start + at)
            .ok_or_else(|| bad(start))?;
        let mut at = start + "<?xml".len();
        let pseudo = |name: &str, at: &mut usize| -> Result<Option<&'a str>, XmlError> {
            let spaced = crate::terminals::skip_ws(bytes, *at);
            if spaced == *at || !src[spaced..close].starts_with(name) {
                return Ok(None);
            }
            let mut cursor = crate::terminals::skip_ws(bytes, spaced + name.len());
            if bytes.get(cursor) != Some(&b'=') {
                return Err(bad(cursor));
            }
            cursor = crate::terminals::skip_ws(bytes, cursor + 1);
            let quote = *bytes.get(cursor).ok_or_else(|| bad(cursor))?;
            if quote != b'"' && quote != b'\'' {
                return Err(bad(cursor));
            }
            let value_end = src[cursor + 1..close]
                .find(char::from(quote))
                .map(|end| cursor + 1 + end)
                .ok_or_else(|| bad(cursor))?;
            *at = value_end + 1;
            Ok(Some(&src[cursor + 1..value_end]))
        };
        let version = pseudo("version", &mut at)?.ok_or_else(|| bad(at))?;
        let valid_version = version
            .strip_prefix("1.")
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        if !valid_version {
            return Err(bad(start));
        }
        // `1.1` is a defined version with its own rules (XML 1.1); any other
        // `1.x` is read as 1.0 (XML 1.0 Fifth Edition §2.8).
        if version == "1.1" {
            return Err(XmlError::new(
                XmlErrorKind::UnsupportedVersion(version.to_owned()),
                start,
            ));
        }
        let encoding = pseudo("encoding", &mut at)?;
        if let Some(name) = encoding {
            let mut chars = name.bytes();
            let valid = chars.next().is_some_and(|b| b.is_ascii_alphabetic())
                && chars.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
            if !valid {
                return Err(bad(start));
            }
        }
        let standalone = match pseudo("standalone", &mut at)? {
            None => None,
            Some("yes") => Some(true),
            Some("no") => Some(false),
            Some(_) => return Err(bad(start)),
        };
        if crate::terminals::skip_ws(bytes, at) != close {
            return Err(bad(at));
        }
        self.standalone = standalone == Some(true);
        self.pos = close + 2;
        Ok(Declaration {
            version: Cow::Borrowed(version),
            encoding: encoding.map(Cow::Borrowed),
            standalone,
        })
    }

    /// Require whitespace at `pos` and skip it.
    fn require_space(&self, bytes: &[u8], pos: usize) -> Result<usize, XmlError> {
        let end = crate::terminals::skip_ws(bytes, pos);
        if end == pos {
            return Err(self.error(XmlErrorKind::Expected("whitespace"), pos));
        }
        Ok(end)
    }

    /// The start tag at `pos`.
    fn start_tag(
        &mut self,
        text: &str,
        in_doc: bool,
        pos: usize,
    ) -> Result<Event<'_, 'a>, XmlError> {
        let depth = self.open.len() + 1;
        if depth > self.options.max_depth {
            return Err(self.error(
                XmlErrorKind::DepthLimit {
                    limit: self.options.max_depth,
                },
                pos,
            ));
        }
        let bytes = text.as_bytes();
        let (name, mut at) = self.qname(text, in_doc, pos + 1)?;
        self.attributes.clear();
        self.declarations.clear();
        let self_closing = loop {
            let spaced = crate::terminals::skip_ws(bytes, at);
            match bytes.get(spaced) {
                None => return Err(self.error(XmlErrorKind::UnexpectedEof, spaced)),
                Some(b'>') => {
                    at = spaced + 1;
                    break false;
                }
                Some(b'/') => {
                    if bytes.get(spaced + 1) != Some(&b'>') {
                        return Err(self.error(XmlErrorKind::Expected("`/>`"), spaced));
                    }
                    at = spaced + 2;
                    break true;
                }
                Some(_) if spaced == at => {
                    return Err(self.error(XmlErrorKind::Expected("whitespace"), at));
                }
                Some(_) => {}
            }
            let (attribute, name_end) = self.qname(text, in_doc, spaced)?;
            let mut cursor = crate::terminals::skip_ws(bytes, name_end);
            if bytes.get(cursor) != Some(&b'=') {
                return Err(self.error(XmlErrorKind::Expected("`=`"), cursor));
            }
            cursor = crate::terminals::skip_ws(bytes, cursor + 1);
            let (value, value_end) = self.attribute_value(text, in_doc, cursor)?;
            let offset = self.offset(spaced);
            self.attributes.push(Attribute {
                name: attribute,
                namespace: None,
                value,
                offset,
            });
            at = value_end;
        };
        self.set_pos(at);
        let offset = self.offset(pos);
        self.resolve_start(name, offset)?;
        if self_closing {
            self.pending_end = true;
        }
        let open = self.open.last().expect("the element was just opened");
        Ok(Event::Start(StartTag {
            name: &open.name,
            namespace: open.namespace.as_ref(),
            attributes: &self.attributes,
            namespace_declarations: &self.declarations,
            self_closing,
            depth,
            offset,
        }))
    }

    /// Apply the internal subset's attribute declarations, split off the
    /// namespace declarations, resolve every name, and open the element.
    fn resolve_start(&mut self, name: QName<'a>, offset: usize) -> Result<(), XmlError> {
        if let Some(duplicate) = duplicate(&self.attributes, |a| a.name.as_str()) {
            let name = self.attributes[duplicate].name.as_str().to_owned();
            return Err(XmlError::new(
                XmlErrorKind::DuplicateAttribute(name),
                offset,
            ));
        }
        self.apply_attlists(name.as_str(), offset)?;
        // Namespace declarations (Namespaces in XML 1.0 §3).
        let mut index = 0;
        while index < self.attributes.len() {
            let attribute = &self.attributes[index];
            let prefix = match (attribute.name.prefix(), attribute.name.local()) {
                (None, "xmlns") => None,
                (Some("xmlns"), local) => Some(local),
                _ => {
                    index += 1;
                    continue;
                }
            };
            let uri = attribute.value.as_ref();
            let reserved = |what: &str| {
                XmlError::new(XmlErrorKind::ReservedNamespace(what.to_owned()), offset)
            };
            match prefix {
                Some("xmlns") => return Err(reserved("the `xmlns` prefix cannot be declared")),
                Some("xml") if uri != XML_NAMESPACE => {
                    return Err(reserved("the `xml` prefix is bound to the XML namespace"));
                }
                Some(p) if p != "xml" && uri == XML_NAMESPACE => {
                    return Err(reserved("only `xml` may be bound to the XML namespace"));
                }
                None if uri == XML_NAMESPACE => {
                    return Err(reserved("only `xml` may be bound to the XML namespace"));
                }
                _ if uri == XMLNS_NAMESPACE => {
                    return Err(reserved("the xmlns namespace cannot be declared"));
                }
                Some(p) if uri.is_empty() => {
                    return Err(XmlError::new(
                        XmlErrorKind::EmptyNamespacePrefix(p.to_owned()),
                        offset,
                    ));
                }
                _ => {}
            }
            let attribute = self.attributes.remove(index);
            let prefix = match attribute.name.text {
                Cow::Borrowed(text) => attribute.name.colon.map(|c| Cow::Borrowed(&text[c + 1..])),
                Cow::Owned(ref text) => attribute
                    .name
                    .colon
                    .map(|c| Cow::Owned(text[c + 1..].to_owned())),
            };
            self.declarations.push(NamespaceDecl {
                prefix,
                uri: attribute.value,
            });
        }
        let mark = self.bindings.len();
        self.bindings.extend(self.declarations.iter().cloned());
        let namespace = match name.prefix() {
            Some("xmlns") => {
                return Err(XmlError::new(
                    XmlErrorKind::ReservedNamespace(
                        "an element cannot have the `xmlns` prefix".to_owned(),
                    ),
                    offset,
                ));
            }
            prefix => match resolve(&self.bindings, prefix) {
                Resolution::Bound(uri) => Some(uri),
                Resolution::None => None,
                Resolution::Unbound => {
                    let prefix = prefix.unwrap_or_default().to_owned();
                    return Err(XmlError::new(
                        XmlErrorKind::UndeclaredPrefix(prefix),
                        offset,
                    ));
                }
            },
        };
        for index in 0..self.attributes.len() {
            let Some(prefix) = self.attributes[index].name.prefix() else {
                continue;
            };
            match resolve(&self.bindings, Some(prefix)) {
                Resolution::Bound(uri) => self.attributes[index].namespace = Some(uri),
                Resolution::None | Resolution::Unbound => {
                    let prefix = prefix.to_owned();
                    return Err(XmlError::new(
                        XmlErrorKind::UndeclaredPrefix(prefix),
                        offset,
                    ));
                }
            }
        }
        // Namespaces in XML 1.0 §6.3: no two attributes with one expanded name.
        if self.attributes.iter().any(|a| a.namespace.is_some()) {
            let mut keys: Vec<(&str, &str, usize)> = self
                .attributes
                .iter()
                .enumerate()
                .map(|(i, a)| (a.namespace.as_deref().unwrap_or(""), a.name.local(), i))
                .collect();
            keys.sort_unstable();
            if let Some(pair) = keys
                .windows(2)
                .find(|pair| pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1)
            {
                let name = self.attributes[pair[1].2].name.as_str().to_owned();
                return Err(XmlError::new(
                    XmlErrorKind::DuplicateAttribute(name),
                    offset,
                ));
            }
        }
        self.open.push(Open {
            name,
            namespace,
            mark,
            offset,
        });
        Ok(())
    }

    /// Add the declared defaults this start tag omits, and normalize the
    /// values of attributes declared with a non-`CDATA` type (XML 1.0 §3.3.2,
    /// §3.3.3).
    fn apply_attlists(&mut self, element: &str, offset: usize) -> Result<(), XmlError> {
        for index in 0..self.attlists.len() {
            let decl = &self.attlists[index];
            if &*decl.element != element {
                continue;
            }
            let cdata = decl.cdata;
            match self
                .attributes
                .iter_mut()
                .find(|a| a.name.as_str() == &*decl.attribute)
            {
                Some(present) => {
                    if !cdata {
                        let collapsed = collapse_spaces(&present.value);
                        if collapsed != present.value {
                            present.value = Cow::Owned(collapsed);
                        }
                    }
                }
                None => {
                    let value = match &decl.default {
                        None => continue,
                        Some(AttDefault::Ready(value)) => value.to_string(),
                        Some(AttDefault::Deferred(literal)) => {
                            let literal = literal.clone();
                            self.attribute_value(&literal, false, 0)?.0.into_owned()
                        }
                    };
                    let value = if cdata {
                        value
                    } else {
                        collapse_spaces(&value)
                    };
                    let name = self.attlists[index].attribute.to_string();
                    let name = QName::new(Cow::Owned(name))
                        .ok_or_else(|| XmlError::new(XmlErrorKind::InvalidName, offset))?;
                    self.attributes.push(Attribute {
                        name,
                        namespace: None,
                        value: Cow::Owned(value),
                        offset,
                    });
                }
            }
        }
        Ok(())
    }

    /// The end tag at `pos`.
    fn end_tag(&mut self, text: &str, pos: usize) -> Result<Event<'_, 'a>, XmlError> {
        let bytes = text.as_bytes();
        let name_end = self.name_end(text, pos + 2)?;
        let close = crate::terminals::skip_ws(bytes, name_end);
        if bytes.get(close) != Some(&b'>') {
            return Err(self.error(XmlErrorKind::Expected("`>`"), close));
        }
        let found = &text[pos + 2..name_end];
        let Some(open) = self.open.last() else {
            return Err(self.error(XmlErrorKind::ContentOutsideRoot, pos));
        };
        if open.name.as_str() != found {
            let expected = open.name.as_str().to_owned();
            return Err(self.error(
                XmlErrorKind::MismatchedEndTag {
                    expected,
                    found: found.to_owned(),
                },
                pos,
            ));
        }
        if let Some(frame) = self.frames.last()
            && self.open.len() <= frame.depth
        {
            let name = self.entities[frame.entity].name.to_string();
            return Err(self.error(XmlErrorKind::UnbalancedEntity(name), pos));
        }
        self.set_pos(close + 1);
        let offset = self.offset(pos);
        let event = self.close_element();
        Ok(match event {
            Event::End(end) => Event::End(EndTag { offset, ..end }),
            other => other,
        })
    }

    /// The attribute value whose opening quote is at `pos`, and its end.
    fn attribute_value(
        &mut self,
        text: &str,
        in_doc: bool,
        pos: usize,
    ) -> Result<(Cow<'a, str>, usize), XmlError> {
        let bytes = text.as_bytes();
        let quote = match bytes.get(pos) {
            Some(&q @ (b'"' | b'\'')) => q,
            Some(_) => return Err(self.error(XmlErrorKind::Expected("a quoted value"), pos)),
            None => return Err(self.error(XmlErrorKind::UnexpectedEof, pos)),
        };
        let start = pos + 1;
        let mut at = start;
        let mut value: Option<String> = None;
        let mut run = start;
        loop {
            let Some(offset) = ATTR.find_first(&bytes[at..]) else {
                return Err(self.error(XmlErrorKind::UnexpectedEof, pos));
            };
            let hit = at + offset;
            let b = bytes[hit];
            if b == quote {
                let end = hit + 1;
                return Ok(match value {
                    None => (slice(self.src, text, in_doc, start, hit), end),
                    Some(mut owned) => {
                        owned.push_str(&text[run..hit]);
                        (Cow::Owned(owned), end)
                    }
                });
            }
            match b {
                b'"' | b'\'' => {
                    at = hit + 1;
                    continue;
                }
                b'<' => return Err(self.error(XmlErrorKind::LessThanInAttributeValue, hit)),
                0xEF => {
                    if let Some(c) = illegal_at(text, hit) {
                        return Err(self.error(XmlErrorKind::IllegalChar(c), hit));
                    }
                    at = hit + 1;
                    continue;
                }
                b'\t' | b'\n' | b'\r' | b'&' => {}
                _ => {
                    let c = char::from(b);
                    return Err(self.error(XmlErrorKind::IllegalChar(c), hit));
                }
            }
            let owned = value.get_or_insert_with(String::new);
            owned.push_str(&text[run..hit]);
            if b == b'&' {
                let (reference, end) = self.reference(text, hit)?;
                match reference {
                    Reference::Char(c) => owned.push(c),
                    Reference::Predefined(v) => owned.push_str(v),
                    Reference::Entity(entity) => {
                        let mut expanded = String::new();
                        self.expand_in_attribute(entity, hit, &mut expanded)?;
                        value.get_or_insert_with(String::new).push_str(&expanded);
                    }
                }
                at = end;
            } else {
                owned.push(' ');
                at = hit + 1;
                if b == b'\r' && in_doc && bytes.get(at) == Some(&b'\n') {
                    at += 1;
                }
            }
            run = at;
        }
    }

    /// Append the normalized replacement text of `entity`, referenced at
    /// `pos` inside an attribute value, to `out` (XML 1.0 §3.3.3: nested
    /// references expanded, each whitespace character a SPACE, no `<`).
    fn expand_in_attribute(
        &mut self,
        entity: usize,
        pos: usize,
        out: &mut String,
    ) -> Result<(), XmlError> {
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let push = |this: &mut Self, stack: &mut Vec<(usize, usize)>, entity: usize| {
            if this.frames.iter().any(|f| f.entity == entity)
                || stack.iter().any(|&(e, _)| e == entity)
            {
                let name = this.entities[entity].name.to_string();
                return Err(this.error(XmlErrorKind::RecursiveEntity(name), pos));
            }
            let len = this.entities[entity].text.len();
            this.charge(len, pos)?;
            stack.push((entity, 0));
            Ok(())
        };
        push(self, &mut stack, entity)?;
        while let Some(&mut (current, ref mut at)) = stack.last_mut() {
            let text = Arc::clone(&self.entities[current].text);
            let Some(c) = text[*at..].chars().next() else {
                stack.pop();
                continue;
            };
            let here = *at;
            *at += c.len_utf8();
            match c {
                '<' => return Err(self.error(XmlErrorKind::LessThanInAttributeValue, pos)),
                ' ' | '\t' | '\n' | '\r' => out.push(' '),
                '&' => {
                    let (reference, end) = self.reference(&text, here)?;
                    if let Some(top) = stack.last_mut() {
                        top.1 = end;
                    }
                    match reference {
                        Reference::Char(c) => out.push(c),
                        Reference::Predefined(v) => out.push_str(v),
                        Reference::Entity(nested) => push(self, &mut stack, nested)?,
                    }
                }
                c => out.push(c),
            }
        }
        Ok(())
    }
}

/// How a prefix resolves against a binding stack.
enum Resolution<'a> {
    Bound(Cow<'a, str>),
    /// The default namespace is undeclared or was never declared.
    None,
    Unbound,
}

fn resolve<'a>(bindings: &[NamespaceDecl<'a>], prefix: Option<&str>) -> Resolution<'a> {
    match bindings
        .iter()
        .rev()
        .find(|b| b.prefix.as_deref() == prefix)
    {
        Some(binding) if binding.uri.is_empty() => Resolution::None,
        Some(binding) => Resolution::Bound(binding.uri.clone()),
        None => match prefix {
            None => Resolution::None,
            Some("xml") => Resolution::Bound(Cow::Borrowed(XML_NAMESPACE)),
            Some(_) => Resolution::Unbound,
        },
    }
}

fn lookup<'s>(bindings: &'s [NamespaceDecl<'_>], prefix: Option<&str>) -> Option<&'s str> {
    match bindings
        .iter()
        .rev()
        .find(|b| b.prefix.as_deref() == prefix)
    {
        Some(binding) => (!binding.uri.is_empty()).then_some(&*binding.uri),
        None => (prefix == Some("xml")).then_some(XML_NAMESPACE),
    }
}

/// The index of an item whose key repeats an earlier item's.
fn duplicate<T>(items: &[T], key: impl Fn(&T) -> &str) -> Option<usize> {
    if items.len() <= 16 {
        return (1..items.len()).find(|&i| items[..i].iter().any(|e| key(e) == key(&items[i])));
    }
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| key(&items[a]).cmp(key(&items[b])).then(a.cmp(&b)));
    order
        .windows(2)
        .find(|pair| key(&items[pair[0]]) == key(&items[pair[1]]))
        .map(|pair| pair[1])
}

/// Non-`CDATA` attribute normalization (XML 1.0 §3.3.3): leading and trailing
/// SPACEs dropped, every run of SPACEs one SPACE.
fn collapse_spaces(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for token in value.split(' ').filter(|t| !t.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(token);
    }
    out
}

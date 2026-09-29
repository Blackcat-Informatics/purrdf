// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The YAML 1.2 reader: one document into the JSON data model, over explicit
//! stacks of open block and flow collections.

use core::cell::Cell;
use core::fmt;
use core::mem;
use std::collections::{HashMap, HashSet};

use purrdf_hash::fixed::FixedState;

use super::resolve::{Plain, resolve};
use crate::json::{Number, Object, Value};

/// The resource bounds of one read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most block and flow collections open at once.
    pub max_depth: usize,
    /// The most nodes the document may denote, every node an alias expands to
    /// included, so an alias bomb is refused before it is built.
    pub max_nodes: u64,
    /// Whether anchors and aliases are accepted. YAML-LD's JSON profile
    /// refuses them.
    pub aliases: bool,
}

impl Limits {
    /// 128 open collections, 1,048,576 nodes, aliases accepted.
    pub const DEFAULT: Self = Self {
        max_depth: 128,
        max_nodes: 1 << 20,
        aliases: true,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What is wrong with a YAML document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// The grammar refuses the text here; the text names what was expected.
    Syntax(&'static str),
    /// A tab where YAML requires indentation spaces (§6.1).
    Tab,
    /// A tag other than the core schema's (`!!str`, `!!int`, `!!float`,
    /// `!!bool`, `!!null`, `!!seq`, `!!map`) and the non-specific `!`, or a
    /// core tag its node does not match (`!!int x`).
    Tag,
    /// An anchor or alias while [`Limits::aliases`] is off.
    Alias,
    /// An alias names no anchor defined before it.
    UnknownAlias,
    /// A mapping key that is not a string: a number, a boolean, `null`, an
    /// empty key, or a collection.
    NonStringKey,
    /// A mapping repeats a key (§3.2.1.1).
    DuplicateKey,
    /// `.inf`, `-.inf` or `.nan`, which JSON cannot hold.
    NonFinite,
    /// A malformed double-quoted escape, or one that names no character.
    Escape,
    /// A second document in the stream.
    MultipleDocuments,
    /// More collections open at once than [`Limits::max_depth`] allows.
    Depth {
        /// The cap that was exceeded.
        limit: usize,
    },
    /// More nodes than [`Limits::max_nodes`] allows.
    Nodes {
        /// The cap that was exceeded.
        limit: u64,
    },
}

/// A refused YAML document: what is wrong, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    offset: usize,
    line: usize,
    column: usize,
}

impl Error {
    /// What is wrong.
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The byte offset of the defect.
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// The defect's line, from 1.
    pub const fn line(&self) -> usize {
        self.line
    }

    /// The defect's column in characters, from 1.
    pub const fn column(&self) -> usize {
        self.column
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "YAML line {} column {}: ", self.line, self.column)?;
        match self.kind {
            ErrorKind::Syntax(what) => write!(f, "expected {what}"),
            ErrorKind::Tab => f.write_str("a tab cannot indent a block"),
            ErrorKind::Tag => {
                f.write_str("a tag other than a core-schema tag, or one its node does not match")
            }
            ErrorKind::Alias => f.write_str("anchors and aliases are not accepted here"),
            ErrorKind::UnknownAlias => f.write_str("the alias names no anchor defined before it"),
            ErrorKind::NonStringKey => f.write_str("a mapping key must be a string"),
            ErrorKind::DuplicateKey => f.write_str("a mapping repeats a key"),
            ErrorKind::NonFinite => f.write_str("JSON cannot hold an infinite or NaN number"),
            ErrorKind::Escape => f.write_str("a malformed escape"),
            ErrorKind::MultipleDocuments => f.write_str("the stream holds more than one document"),
            ErrorKind::Depth { limit } => write!(f, "more than {limit} nested collections"),
            ErrorKind::Nodes { limit } => write!(f, "more than {limit} nodes"),
        }
    }
}

impl std::error::Error for Error {}

/// A block mapping's pending entry.
enum Slot {
    /// Between entries.
    Idle,
    /// After `?`: the key node is next.
    ExplicitKey,
    /// An explicit key is complete; `:` and its value may follow.
    KeyDone(String, usize),
    /// After `key:`: the value node is next.
    Value(String, usize),
}

/// An open block collection.
enum Frame {
    /// The document: its one node.
    Root(Option<Value>),
    Sequence {
        indent: usize,
        start: usize,
        items: Vec<Value>,
        /// After `-`: the item node is next.
        open: bool,
        anchor: Option<String>,
    },
    Mapping {
        indent: usize,
        start: usize,
        object: Object,
        keys: HashSet<String, FixedState>,
        slot: Slot,
        anchor: Option<String>,
    },
}

/// Where the cursor is on its line, for what may start there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// Past a line break: any node.
    Fresh,
    /// After `-` or `?`: a compact block collection may start.
    Compact,
    /// After a mapping's `:` or the document's `---`: no block collection may
    /// start on this line.
    Inline,
}

/// An anchor waiting for its node.
/// A YAML 1.2 core-schema tag (§10.3), or the non-specific `!`: the tags the
/// JSON data model can honour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    Str,
    Int,
    Float,
    Bool,
    Null,
    Seq,
    Map,
    NonSpecific,
}

/// A node's properties (§6.9), waiting for their node.
struct Properties {
    anchor: Option<String>,
    tag: Option<(Tag, usize)>,
    /// Where the first property starts.
    at: usize,
    column: usize,
}

/// The anchor and tag of `properties`.
fn split(properties: Option<Properties>) -> (Option<String>, Option<(Tag, usize)>) {
    properties.map_or((None, None), |properties| {
        (properties.anchor, properties.tag)
    })
}

/// An open flow collection.
enum Flow {
    Sequence {
        items: Vec<Value>,
        anchor: Option<String>,
        /// Whether an entry is complete and `,` or `]` is next.
        after_item: bool,
    },
    Mapping {
        object: Object,
        keys: HashSet<String, FixedState>,
        anchor: Option<String>,
        key: Option<(String, usize)>,
        state: FlowMap,
    },
    /// A single-pair mapping inside a flow sequence (`[a: b]`).
    Pair { key: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlowMap {
    Key,
    Colon,
    Value,
    Separator,
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    limits: Limits,
    nodes: u64,
    depth: usize,
    /// Each anchor's node and how many nodes it holds.
    anchors: HashMap<String, (Value, u64), FixedState>,
    stack: Vec<Frame>,
    pending: Option<Properties>,
    /// The last position located: its offset, line start and column, so
    /// locating the cursor as it advances costs the bytes it crossed.
    located: Cell<(usize, usize, usize)>,
}

const fn is_break(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'\n' | b'\r'))
}

const fn is_flow_indicator(byte: Option<u8>) -> bool {
    matches!(byte, Some(b',' | b'[' | b']' | b'{' | b'}'))
}

impl Parser<'_> {
    fn error(&self, kind: ErrorKind, at: usize) -> Error {
        let at = at.min(self.text.len());
        let line_start = self.line_start(at);
        Error {
            kind,
            offset: at,
            line: 1 + self.text[..at].bytes().filter(|&b| b == b'\n').count(),
            column: 1 + self.text[line_start..at].chars().count(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, at: usize) -> Option<u8> {
        self.bytes.get(at).copied()
    }

    /// Whether the byte at `at` is a space, tab, line break or the end.
    fn blank_at(&self, at: usize) -> bool {
        matches!(self.peek_at(at), None | Some(b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Whether the cursor is on the indicator `c` followed by a blank.
    fn at_indicator(&self, c: u8) -> bool {
        self.peek() == Some(c) && self.blank_at(self.pos + 1)
    }

    /// The start of the line holding `at`, and `at`'s column in characters.
    fn locate(&self, at: usize) -> (usize, usize) {
        let (last, last_start, last_column) = self.located.get();
        let located = if at >= last {
            let crossed = &self.text[last..at];
            match crossed.rfind(['\n', '\r']) {
                Some(index) => {
                    let start = last + index + 1;
                    (start, self.text[start..at].chars().count())
                }
                None => (last_start, last_column + crossed.chars().count()),
            }
        } else {
            let start = self.text[..at]
                .rfind(['\n', '\r'])
                .map_or(0, |index| index + 1);
            (start, self.text[start..at].chars().count())
        };
        self.located.set((at, located.0, located.1));
        located
    }

    fn line_start(&self, at: usize) -> usize {
        self.locate(at).0
    }

    fn column(&self, at: usize) -> usize {
        self.locate(at).1
    }

    /// Whether the cursor is on a document marker (`---` or `...` at the start
    /// of a line, followed by a blank).
    fn at_marker(&self) -> bool {
        self.pos == self.line_start(self.pos)
            && (self.bytes[self.pos..].starts_with(b"---")
                || self.bytes[self.pos..].starts_with(b"..."))
            && self.blank_at(self.pos + 3)
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn skip_to_line_end(&mut self) {
        while !matches!(self.peek(), None | Some(b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn take_break(&mut self) -> bool {
        match self.peek() {
            Some(b'\n') => {
                self.pos += 1;
                true
            }
            Some(b'\r') => {
                self.pos += 1;
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                true
            }
            _ => false,
        }
    }

    /// Whether `#` at the cursor starts a comment (it is preceded by a blank
    /// or starts a line).
    fn at_comment(&self) -> bool {
        self.peek() == Some(b'#')
            && (self.pos == 0 || matches!(self.bytes[self.pos - 1], b' ' | b'\t' | b'\n' | b'\r'))
    }

    /// Skip spaces, tabs, comments and line breaks: whether a line break was
    /// crossed.
    fn skip_separation(&mut self) -> bool {
        let mut crossed = false;
        loop {
            self.skip_inline_space();
            if self.at_comment() {
                self.skip_to_line_end();
            }
            if !self.take_break() {
                return crossed;
            }
            crossed = true;
        }
    }

    /// Refuse a tab in the indentation of the line the cursor's node starts.
    fn check_indentation(&self) -> Result<(), Error> {
        let start = self.line_start(self.pos);
        let prefix = &self.bytes[start..self.pos];
        if prefix.iter().all(|&b| b == b' ' || b == b'\t') && prefix.contains(&b'\t') {
            return Err(self.error(ErrorKind::Tab, start));
        }
        Ok(())
    }

    fn count(&mut self, nodes: u64, at: usize) -> Result<(), Error> {
        self.nodes = self.nodes.saturating_add(nodes);
        if self.nodes > self.limits.max_nodes {
            return Err(self.error(
                ErrorKind::Nodes {
                    limit: self.limits.max_nodes,
                },
                at,
            ));
        }
        Ok(())
    }

    fn enter(&mut self, at: usize) -> Result<(), Error> {
        if self.depth >= self.limits.max_depth {
            return Err(self.error(
                ErrorKind::Depth {
                    limit: self.limits.max_depth,
                },
                at,
            ));
        }
        self.depth += 1;
        self.count(1, at)
    }

    /// The value a plain scalar denotes.
    fn plain_value(&self, text: String, at: usize) -> Result<Value, Error> {
        Ok(match resolve(&text) {
            Plain::Null => Value::Null,
            Plain::Bool(value) => Value::Bool(value),
            Plain::Number(lexeme) => Value::Number(
                Number::from_lexeme(lexeme)
                    .map_err(|_| self.error(ErrorKind::Syntax("a JSON-compatible number"), at))?,
            ),
            Plain::NonFinite => return Err(self.error(ErrorKind::NonFinite, at)),
            Plain::String => Value::String(text),
        })
    }

    /// A plain scalar key: its text, refused unless it denotes a string.
    fn plain_key(&self, text: &str, at: usize) -> Result<String, Error> {
        match resolve(text) {
            Plain::String => Ok(text.to_owned()),
            _ => Err(self.error(ErrorKind::NonStringKey, at)),
        }
    }

    /// A key's text, refused unless it denotes a string: a plain key resolves
    /// by the core schema unless a tag says `!!str` or `!`.
    fn key_text(
        &self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<String, Error> {
        match tag {
            None if plain => self.plain_key(&text, at),
            None | Some((Tag::Str | Tag::NonSpecific, _)) => Ok(text),
            Some(_) => Err(self.error(ErrorKind::NonStringKey, at)),
        }
    }

    /// The value of a scalar: a plain one resolves by the core schema, a
    /// quoted or block one is a string, and a tag decides both (§10.3).
    fn scalar(
        &self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<Value, Error> {
        let Some((tag, tag_at)) = tag else {
            return if plain {
                self.plain_value(text, at)
            } else {
                Ok(Value::String(text))
            };
        };
        let value = match tag {
            Tag::Str | Tag::NonSpecific => return Ok(Value::String(text)),
            Tag::Seq | Tag::Map => return Err(self.error(ErrorKind::Tag, tag_at)),
            Tag::Int | Tag::Float | Tag::Bool | Tag::Null => self.plain_value(text, at)?,
        };
        let matches = match (tag, &value) {
            (Tag::Int, Value::Number(number)) => number.is_integer(),
            (Tag::Float, Value::Number(_))
            | (Tag::Bool, Value::Bool(_))
            | (Tag::Null, Value::Null) => true,
            _ => false,
        };
        if matches {
            Ok(value)
        } else {
            Err(self.error(ErrorKind::Tag, tag_at))
        }
    }

    /// Refuse a tag a collection does not match.
    fn collection_tag(&self, tag: Option<(Tag, usize)>, kind: Tag) -> Result<(), Error> {
        match tag {
            Some((tag, at)) if tag != kind && tag != Tag::NonSpecific => {
                Err(self.error(ErrorKind::Tag, at))
            }
            _ => Ok(()),
        }
    }

    /// The tag at the cursor (on its `!`).
    fn tag(&mut self) -> Result<Tag, Error> {
        let at = self.pos;
        let name = if self.bytes[at..].starts_with(b"!<") {
            let close = self.text[at..].find('>').ok_or_else(|| {
                self.error(ErrorKind::Syntax("the `>` that closes a verbatim tag"), at)
            })?;
            self.pos = at + close + 1;
            self.text[at + 2..at + close]
                .strip_prefix("tag:yaml.org,2002:")
                .ok_or_else(|| self.error(ErrorKind::Tag, at))?
        } else {
            self.pos += 1;
            while !self.blank_at(self.pos) && !is_flow_indicator(self.peek()) {
                self.pos += self.text[self.pos..]
                    .chars()
                    .next()
                    .map_or(1, char::len_utf8);
            }
            if self.pos == at + 1 {
                return Ok(Tag::NonSpecific);
            }
            self.text[at..self.pos]
                .strip_prefix("!!")
                .ok_or_else(|| self.error(ErrorKind::Tag, at))?
        };
        match name {
            "str" => Ok(Tag::Str),
            "int" => Ok(Tag::Int),
            "float" => Ok(Tag::Float),
            "bool" => Ok(Tag::Bool),
            "null" => Ok(Tag::Null),
            "seq" => Ok(Tag::Seq),
            "map" => Ok(Tag::Map),
            _ => Err(self.error(ErrorKind::Tag, at)),
        }
    }

    /// Add the property at the cursor (an `&anchor` or a `!tag`) to
    /// `properties`.
    fn property(&mut self, properties: &mut Option<Properties>) -> Result<(), Error> {
        let at = self.pos;
        let column = self.column(at);
        let mut entry = properties.take().unwrap_or(Properties {
            anchor: None,
            tag: None,
            at,
            column,
        });
        if self.peek() == Some(b'&') {
            if entry.anchor.is_some() {
                return Err(self.error(ErrorKind::Syntax("one anchor per node"), at));
            }
            entry.anchor = Some(self.name()?);
        } else {
            if entry.tag.is_some() {
                return Err(self.error(ErrorKind::Syntax("one tag per node"), at));
            }
            entry.tag = Some((self.tag()?, at));
        }
        *properties = Some(entry);
        Ok(())
    }

    /// An anchor's or alias's name, the cursor on its `&` or `*`.
    fn name(&mut self) -> Result<String, Error> {
        let at = self.pos;
        if !self.limits.aliases {
            return Err(self.error(ErrorKind::Alias, at));
        }
        self.pos += 1;
        let start = self.pos;
        while !self.blank_at(self.pos) && !is_flow_indicator(self.peek()) {
            self.pos += self.text[self.pos..]
                .chars()
                .next()
                .map_or(1, char::len_utf8);
        }
        if self.pos == start {
            return Err(self.error(ErrorKind::Syntax("an anchor name"), start));
        }
        Ok(self.text[start..self.pos].to_owned())
    }

    /// The value an alias names, counted against the node bound before it is
    /// copied, so an alias bomb is refused before it is built.
    fn alias(&mut self) -> Result<Value, Error> {
        let at = self.pos;
        let name = self.name()?;
        let size = self
            .anchors
            .get(&name)
            .map(|(_, size)| *size)
            .ok_or_else(|| self.error(ErrorKind::UnknownAlias, at))?;
        self.count(size, at)?;
        Ok(self.anchors[&name].0.clone())
    }

    fn register(&mut self, anchor: Option<String>, value: &Value) {
        if let Some(name) = anchor {
            self.anchor(name, value.clone());
        }
    }

    /// Define the anchor `name` as `value`.
    fn anchor(&mut self, name: String, value: Value) {
        let mut size = 0_u64;
        let mut work = vec![&value];
        while let Some(node) = work.pop() {
            size += 1;
            match node {
                Value::Array(items) => work.extend(items),
                Value::Object(object) => work.extend(object.values()),
                _ => {}
            }
        }
        self.anchors.insert(name, (value, size));
    }

    /// After a scalar or alias: skip spaces and take a `:` value indicator
    /// (followed by a blank) if one is there.
    fn take_value_indicator(&mut self) -> bool {
        let save = self.pos;
        self.skip_inline_space();
        if self.at_indicator(b':') {
            self.pos += 1;
            return true;
        }
        self.pos = save;
        false
    }

    // ── scalars ───────────────────────────────────────────────────────────

    /// One line of a plain scalar in block context: its text without trailing
    /// white space. It stops at a line break, at a comment, and at `:` followed
    /// by a blank; the cursor is left after the text.
    fn plain_line(&mut self) -> &str {
        let start = self.pos;
        let mut end = self.pos;
        loop {
            match self.peek() {
                None | Some(b'\n' | b'\r') => break,
                Some(b' ' | b'\t') => self.pos += 1,
                Some(b':') if self.blank_at(self.pos + 1) => break,
                Some(b'#') if matches!(self.bytes[self.pos - 1], b' ' | b'\t') => break,
                Some(_) => {
                    self.pos += self.text[self.pos..]
                        .chars()
                        .next()
                        .map_or(1, char::len_utf8);
                    end = self.pos;
                }
            }
        }
        self.pos = end;
        &self.text[start..end]
    }

    /// Continue a block plain scalar over the lines indented more than
    /// `owner`, folding line breaks (§7.3.3).
    fn plain_rest(&mut self, mut text: String, owner: isize) -> Result<String, Error> {
        loop {
            let save = self.pos;
            self.skip_inline_space();
            if !is_break(self.peek()) {
                self.pos = save;
                return Ok(text);
            }
            let mut breaks = 0;
            while self.take_break() {
                breaks += 1;
                self.skip_inline_space();
            }
            if self.pos >= self.bytes.len()
                || self.at_marker()
                || self.column(self.pos) as isize <= owner
                || self.at_comment()
            {
                self.pos = save;
                return Ok(text);
            }
            let at = self.pos;
            let line = self.plain_line().to_owned();
            if self.peek() == Some(b':') {
                return Err(self.error(
                    ErrorKind::Syntax("no mapping key inside a multi-line plain scalar"),
                    at,
                ));
            }
            if breaks == 1 {
                text.push(' ');
            } else {
                text.extend(core::iter::repeat_n('\n', breaks - 1));
            }
            text.push_str(&line);
        }
    }

    /// One line of a plain scalar in flow context, stopping also at the flow
    /// indicators and at `:` followed by one.
    fn plain_flow_line(&mut self) -> (usize, usize) {
        let start = self.pos;
        let mut end = self.pos;
        loop {
            match self.peek() {
                None | Some(b'\n' | b'\r' | b',' | b'[' | b']' | b'{' | b'}') => break,
                Some(b' ' | b'\t') => self.pos += 1,
                Some(b':')
                    if self.blank_at(self.pos + 1)
                        || is_flow_indicator(self.peek_at(self.pos + 1)) =>
                {
                    break;
                }
                Some(b'#') if matches!(self.bytes[self.pos - 1], b' ' | b'\t') => break,
                Some(_) => {
                    self.pos += self.text[self.pos..]
                        .chars()
                        .next()
                        .map_or(1, char::len_utf8);
                    end = self.pos;
                }
            }
        }
        self.pos = end;
        (start, end)
    }

    /// A plain scalar in flow context, possibly over several lines.
    fn plain_flow(&mut self) -> String {
        let (start, end) = self.plain_flow_line();
        let mut text = self.text[start..end].to_owned();
        loop {
            let save = self.pos;
            self.skip_inline_space();
            let mut breaks = 0;
            while self.take_break() {
                breaks += 1;
                self.skip_inline_space();
            }
            let stop = breaks == 0
                || self.pos >= self.bytes.len()
                || is_flow_indicator(self.peek())
                || self.at_comment()
                || (self.peek() == Some(b':')
                    && (self.blank_at(self.pos + 1)
                        || is_flow_indicator(self.peek_at(self.pos + 1))));
            if stop {
                self.pos = save;
                return text;
            }
            let (start, end) = self.plain_flow_line();
            if breaks == 1 {
                text.push(' ');
            } else {
                text.extend(core::iter::repeat_n('\n', breaks - 1));
            }
            text.push_str(&self.text[start..end]);
        }
    }

    /// Fold the line break at the cursor inside a quoted scalar (§7.3.1):
    /// one break is a space, `n` breaks with only white space between them are
    /// `n - 1` line feeds; the next line's leading white space is dropped.
    fn fold_quoted_break(&mut self, out: &mut String) {
        let mut breaks = 0;
        while self.take_break() {
            breaks += 1;
            self.skip_inline_space();
        }
        if breaks == 1 {
            out.push(' ');
        } else {
            out.extend(core::iter::repeat_n('\n', breaks - 1));
        }
    }

    /// A single- or double-quoted scalar, the cursor on its opening quote: its
    /// text, and whether it spans lines.
    fn quoted(&mut self) -> Result<(String, bool), Error> {
        let start = self.pos;
        let quote = self.bytes[start];
        self.pos += 1;
        let mut out = String::new();
        let mut multiline = false;
        // Where the current run of source white space began in `out`: white
        // space before a line break is not content.
        let mut blank_from: Option<usize> = None;
        loop {
            match self.peek() {
                None => {
                    return Err(
                        self.error(ErrorKind::Syntax("the quote that closes the scalar"), start)
                    );
                }
                Some(b'\'') if quote == b'\'' => {
                    if self.peek_at(self.pos + 1) == Some(b'\'') {
                        out.push('\'');
                        self.pos += 2;
                        blank_from = None;
                        continue;
                    }
                    self.pos += 1;
                    return Ok((out, multiline));
                }
                Some(b'"') if quote == b'"' => {
                    self.pos += 1;
                    return Ok((out, multiline));
                }
                Some(b'\n' | b'\r') => {
                    multiline = true;
                    if let Some(from) = blank_from.take() {
                        out.truncate(from);
                    }
                    self.fold_quoted_break(&mut out);
                }
                Some(b'\\') if quote == b'"' => {
                    blank_from = None;
                    if is_break(self.peek_at(self.pos + 1)) {
                        // An escaped line break joins the lines with nothing.
                        multiline = true;
                        self.pos += 1;
                        self.take_break();
                        self.skip_inline_space();
                        while self.take_break() {
                            out.push('\n');
                            self.skip_inline_space();
                        }
                        continue;
                    }
                    let (c, width) = self.escape()?;
                    out.push(c);
                    self.pos += width;
                }
                Some(b' ' | b'\t') => {
                    blank_from.get_or_insert(out.len());
                    out.push(char::from(self.bytes[self.pos]));
                    self.pos += 1;
                }
                Some(_) => {
                    let c = self.text[self.pos..].chars().next().expect("a character");
                    out.push(c);
                    self.pos += c.len_utf8();
                    blank_from = None;
                }
            }
        }
    }

    /// The double-quoted escape at the cursor (§5.7): the character and the
    /// bytes the escape spans.
    fn escape(&self) -> Result<(char, usize), Error> {
        let at = self.pos;
        let hex = |width: usize| -> Result<(char, usize), Error> {
            let digits = self
                .text
                .get(at + 2..at + 2 + width)
                .ok_or_else(|| self.error(ErrorKind::Escape, at))?;
            let mut code = 0_u32;
            for byte in digits.bytes() {
                let digit = purrdf_hash::hex::nibble(byte)
                    .ok_or_else(|| self.error(ErrorKind::Escape, at))?;
                code = (code << 4) | u32::from(digit);
            }
            char::from_u32(code)
                .map(|c| (c, 2 + width))
                .ok_or_else(|| self.error(ErrorKind::Escape, at))
        };
        let short = match self.peek_at(at + 1) {
            Some(b'0') => '\0',
            Some(b'a') => '\u{7}',
            Some(b'b') => '\u{8}',
            Some(b't' | b'\t') => '\t',
            Some(b'n') => '\n',
            Some(b'v') => '\u{b}',
            Some(b'f') => '\u{c}',
            Some(b'r') => '\r',
            Some(b'e') => '\u{1b}',
            Some(b' ') => ' ',
            Some(b'"') => '"',
            Some(b'/') => '/',
            Some(b'\\') => '\\',
            Some(b'N') => '\u{85}',
            Some(b'_') => '\u{a0}',
            Some(b'L') => '\u{2028}',
            Some(b'P') => '\u{2029}',
            Some(b'x') => return hex(2),
            Some(b'u') => return hex(4),
            Some(b'U') => return hex(8),
            _ => return Err(self.error(ErrorKind::Escape, at)),
        };
        Ok((short, 2))
    }

    /// A literal (`|`) or folded (`>`) block scalar, the cursor on its
    /// indicator, inside a block indented `owner` (§8.1).
    fn block_scalar(&mut self, owner: isize) -> Result<String, Error> {
        let literal = self.peek() == Some(b'|');
        self.pos += 1;
        let (mut chomping, mut increment) = (0_i8, 0_usize);
        for _ in 0..2 {
            match self.peek() {
                Some(sign @ (b'+' | b'-')) if chomping == 0 => {
                    chomping = if sign == b'+' { 1 } else { -1 };
                    self.pos += 1;
                }
                Some(digit @ b'1'..=b'9') if increment == 0 => {
                    increment = usize::from(digit - b'0');
                    self.pos += 1;
                }
                Some(b'0') => {
                    return Err(self.error(
                        ErrorKind::Syntax("an indentation indicator from 1 to 9"),
                        self.pos,
                    ));
                }
                _ => break,
            }
        }
        self.skip_inline_space();
        if self.at_comment() {
            self.skip_to_line_end();
        }
        if self.pos >= self.bytes.len() {
            return Ok(String::new());
        }
        if !self.take_break() {
            return Err(self.error(
                ErrorKind::Syntax("a line break after a block scalar header"),
                self.pos,
            ));
        }
        let mut indent = match (increment, usize::try_from(owner)) {
            (0, _) => 0,
            (increment, Ok(owner)) => owner + increment,
            (increment, Err(_)) => increment,
        };
        let (mut out, mut leading_break, mut trailing_breaks) =
            (String::new(), String::new(), String::new());
        let mut leading_blank = false;
        self.block_breaks(&mut indent, &mut trailing_breaks, owner)?;
        while self.pos < self.bytes.len() && self.column(self.pos) == indent {
            let trailing_blank = matches!(self.peek(), Some(b' ' | b'\t'));
            if !literal && leading_break == "\n" && !leading_blank && !trailing_blank {
                if trailing_breaks.is_empty() {
                    out.push(' ');
                }
            } else {
                out.push_str(&leading_break);
            }
            leading_break.clear();
            out.push_str(&trailing_breaks);
            trailing_breaks.clear();
            leading_blank = trailing_blank;
            let start = self.pos;
            self.skip_to_line_end();
            out.push_str(&self.text[start..self.pos]);
            if self.take_break() {
                leading_break.push('\n');
            }
            self.block_breaks(&mut indent, &mut trailing_breaks, owner)?;
        }
        if chomping != -1 {
            out.push_str(&leading_break);
        }
        if chomping == 1 {
            out.push_str(&trailing_breaks);
        }
        Ok(out)
    }

    /// Consume the indentation and empty lines before a block scalar's next
    /// content line, fixing the content indentation when it is still to be
    /// detected (`indent == 0`).
    fn block_breaks(
        &mut self,
        indent: &mut usize,
        breaks: &mut String,
        owner: isize,
    ) -> Result<(), Error> {
        let mut max_indent = 0;
        loop {
            while (*indent == 0 || self.column(self.pos) < *indent) && self.peek() == Some(b' ') {
                self.pos += 1;
            }
            max_indent = max_indent.max(self.column(self.pos));
            if (*indent == 0 || self.column(self.pos) < *indent) && self.peek() == Some(b'\t') {
                return Err(self.error(ErrorKind::Tab, self.pos));
            }
            if !self.take_break() {
                break;
            }
            breaks.push('\n');
        }
        if *indent == 0 {
            *indent = max_indent
                .max(usize::try_from(owner + 1).unwrap_or(0))
                .max(1);
        }
        Ok(())
    }

    // ── flow collections ──────────────────────────────────────────────────

    /// Skip white space, line breaks and comments inside a flow collection.
    fn skip_flow_space(&mut self) {
        loop {
            self.skip_inline_space();
            if self.at_comment() {
                self.skip_to_line_end();
            }
            if !self.take_break() {
                return;
            }
        }
    }

    /// A flow collection, the cursor on its `[` or `{`, however deep, over a
    /// heap stack.
    fn flow(&mut self, properties: Option<Properties>) -> Result<Value, Error> {
        let mut stack: Vec<Flow> = Vec::new();
        let mut pending = properties;
        loop {
            self.skip_flow_space();
            let at = self.pos;
            let Some(byte) = self.peek() else {
                return Err(self.error(ErrorKind::Syntax("the end of a flow collection"), at));
            };
            // Separators and closers, where the innermost collection expects one.
            match stack.last_mut() {
                Some(Flow::Sequence { after_item, .. }) => match byte {
                    b']' => {
                        self.pos += 1;
                        let value = self.close_flow(&mut stack);
                        if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                            return Ok(done);
                        }
                        continue;
                    }
                    b',' if *after_item => {
                        self.pos += 1;
                        *after_item = false;
                        continue;
                    }
                    b':' if *after_item => {
                        // `[key: value]`: the last item is the key of a
                        // single-pair mapping.
                        self.pos += 1;
                        let Some(Flow::Sequence {
                            items, after_item, ..
                        }) = stack.last_mut()
                        else {
                            unreachable!("the innermost collection is a sequence");
                        };
                        let Some(key) = items.pop().and_then(string) else {
                            return Err(self.error(ErrorKind::NonStringKey, at));
                        };
                        *after_item = false;
                        stack.push(Flow::Pair { key });
                        continue;
                    }
                    _ if *after_item => {
                        return Err(
                            self.error(ErrorKind::Syntax("`,` or `]` in a flow sequence"), at)
                        );
                    }
                    b',' => {
                        return Err(self.error(ErrorKind::Syntax("an entry before `,`"), at));
                    }
                    _ => {}
                },
                Some(Flow::Mapping {
                    state,
                    key,
                    object,
                    keys,
                    ..
                }) => match (*state, byte) {
                    (_, b'}') => {
                        if let Some((name, key_at)) = key.take() {
                            if !keys.insert(name.clone()) {
                                return Err(self.error(ErrorKind::DuplicateKey, key_at));
                            }
                            object.push(name, Value::Null);
                        }
                        self.pos += 1;
                        let value = self.close_flow(&mut stack);
                        if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                            return Ok(done);
                        }
                        continue;
                    }
                    (FlowMap::Separator, b',') => {
                        self.pos += 1;
                        *state = FlowMap::Key;
                        continue;
                    }
                    (FlowMap::Colon | FlowMap::Value, b',') => {
                        let (name, key_at) = key.take().expect("a key precedes its value");
                        if !keys.insert(name.clone()) {
                            return Err(self.error(ErrorKind::DuplicateKey, key_at));
                        }
                        object.push(name, Value::Null);
                        self.pos += 1;
                        *state = FlowMap::Key;
                        continue;
                    }
                    (FlowMap::Colon, b':') => {
                        self.pos += 1;
                        *state = FlowMap::Value;
                        continue;
                    }
                    (FlowMap::Colon | FlowMap::Separator, _) => {
                        return Err(
                            self.error(ErrorKind::Syntax("`,` or `}` in a flow mapping"), at)
                        );
                    }
                    (FlowMap::Key, b',') => {
                        return Err(self.error(ErrorKind::Syntax("an entry before `,`"), at));
                    }
                    (FlowMap::Key, b':')
                        if self.blank_at(at + 1) || is_flow_indicator(self.peek_at(at + 1)) =>
                    {
                        return Err(self.error(ErrorKind::NonStringKey, at));
                    }
                    _ => {}
                },
                Some(Flow::Pair { .. }) => {
                    if matches!(byte, b',' | b']') {
                        if let Some(done) = self.attach_flow(&mut stack, Value::Null, at)? {
                            return Ok(done);
                        }
                        continue;
                    }
                }
                None => {}
            }
            // A node.
            match byte {
                b'[' | b'{' => {
                    let (anchor, tag) = split(pending.take());
                    self.collection_tag(tag, if byte == b'[' { Tag::Seq } else { Tag::Map })?;
                    self.enter(at)?;
                    self.pos += 1;
                    stack.push(if byte == b'[' {
                        Flow::Sequence {
                            items: Vec::new(),
                            anchor,
                            after_item: false,
                        }
                    } else {
                        Flow::Mapping {
                            object: Object::new(),
                            keys: HashSet::default(),
                            anchor,
                            key: None,
                            state: FlowMap::Key,
                        }
                    });
                }
                b'&' | b'!' => self.property(&mut pending)?,
                b'*' => {
                    if pending.is_some() {
                        return Err(
                            self.error(ErrorKind::Syntax("an alias without properties"), at)
                        );
                    }
                    let value = self.alias()?;
                    if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                        return Ok(done);
                    }
                }
                b'?' if self.blank_at(at + 1) => self.pos += 1,
                b'|' | b'>' | b'%' | b'@' | b'`' | b'#' => {
                    return Err(self.error(ErrorKind::Syntax("a flow node"), at));
                }
                b'-' if self.blank_at(at + 1) => {
                    return Err(self.error(
                        ErrorKind::Syntax("a flow node; `- ` is a block sequence entry"),
                        at,
                    ));
                }
                _ => {
                    // A quoted key may be followed by `:` directly (§7.4.1);
                    // a plain one stops before `:` and a blank or indicator.
                    let (text, plain) = if matches!(byte, b'"' | b'\'') {
                        (self.quoted()?.0, false)
                    } else {
                        (self.plain_flow(), true)
                    };
                    let key_position = matches!(
                        stack.last(),
                        Some(Flow::Mapping {
                            state: FlowMap::Key,
                            ..
                        })
                    );
                    let (anchor, tag) = split(pending.take());
                    let value = if key_position {
                        Value::String(self.key_text(text, plain, tag, at)?)
                    } else {
                        self.scalar(text, plain, tag, at)?
                    };
                    self.count(1, at)?;
                    self.register(anchor, &value);
                    if let Some(done) = self.attach_flow(&mut stack, value, at)? {
                        return Ok(done);
                    }
                }
            }
        }
    }

    /// Close the innermost flow collection: its value, its anchor registered.
    fn close_flow(&mut self, stack: &mut Vec<Flow>) -> Value {
        self.depth -= 1;
        let (value, anchor) = match stack.pop() {
            Some(Flow::Sequence { items, anchor, .. }) => (Value::Array(items), anchor),
            Some(Flow::Mapping { object, anchor, .. }) => (Value::Object(object), anchor),
            _ => unreachable!("only an open collection closes"),
        };
        self.register(anchor, &value);
        value
    }

    /// Hand a complete flow node to the innermost collection; the whole value
    /// once the outermost collection closes.
    fn attach_flow(
        &self,
        stack: &mut Vec<Flow>,
        value: Value,
        at: usize,
    ) -> Result<Option<Value>, Error> {
        let mut value = value;
        loop {
            match stack.last_mut() {
                None => return Ok(Some(value)),
                Some(Flow::Sequence {
                    items, after_item, ..
                }) => {
                    items.push(value);
                    *after_item = true;
                    return Ok(None);
                }
                Some(Flow::Mapping {
                    state,
                    key,
                    object,
                    keys,
                    ..
                }) => {
                    match *state {
                        FlowMap::Key => {
                            let Some(name) = string(value) else {
                                return Err(self.error(ErrorKind::NonStringKey, at));
                            };
                            *key = Some((name, at));
                            *state = FlowMap::Colon;
                        }
                        FlowMap::Value => {
                            let (name, key_at) = key.take().expect("a key precedes its value");
                            if !keys.insert(name.clone()) {
                                return Err(self.error(ErrorKind::DuplicateKey, key_at));
                            }
                            object.push(name, value);
                            *state = FlowMap::Separator;
                        }
                        FlowMap::Colon | FlowMap::Separator => {
                            return Err(
                                self.error(ErrorKind::Syntax("`,` or `}` in a flow mapping"), at)
                            );
                        }
                    }
                    return Ok(None);
                }
                Some(Flow::Pair { .. }) => {
                    let Some(Flow::Pair { key }) = stack.pop() else {
                        unreachable!("the innermost frame is a pair");
                    };
                    let mut pair = Object::new();
                    pair.push(key, value);
                    value = Value::Object(pair);
                }
            }
        }
    }

    // ── block structure ───────────────────────────────────────────────────

    /// The open slot of the innermost block collection: the indentation of
    /// its owner, and whether it is a mapping value (which an indentless
    /// sequence may fill).
    fn open_slot(&self) -> Option<(isize, bool)> {
        match self.stack.last()? {
            Frame::Root(None) => Some((-1, false)),
            Frame::Sequence {
                indent, open: true, ..
            } => Some((*indent as isize, false)),
            Frame::Mapping {
                indent,
                slot: Slot::ExplicitKey,
                ..
            } => Some((*indent as isize, false)),
            Frame::Mapping {
                indent,
                slot: Slot::Value(..),
                ..
            } => Some((*indent as isize, true)),
            _ => None,
        }
    }

    /// Fill the innermost open slot with a complete node.
    fn fill(&mut self, value: Value, at: usize) -> Result<(), Error> {
        let refused = match self.stack.last_mut().expect("the root frame stays") {
            Frame::Root(slot) => {
                *slot = Some(value);
                None
            }
            Frame::Sequence { items, open, .. } => {
                items.push(value);
                *open = false;
                None
            }
            Frame::Mapping {
                object, keys, slot, ..
            } => match mem::replace(slot, Slot::Idle) {
                Slot::Value(key, key_at) => {
                    if keys.insert(key.clone()) {
                        object.push(key, value);
                        None
                    } else {
                        Some((ErrorKind::DuplicateKey, key_at))
                    }
                }
                Slot::ExplicitKey => match string(value) {
                    Some(key) => {
                        *slot = Slot::KeyDone(key, at);
                        None
                    }
                    None => Some((ErrorKind::NonStringKey, at)),
                },
                Slot::Idle | Slot::KeyDone(..) => unreachable!("only an open slot is filled"),
            },
        };
        refused.map_or(Ok(()), |(kind, at)| Err(self.error(kind, at)))
    }

    /// Fill the open slot with a scalar carrying `properties`.
    fn fill_scalar(
        &mut self,
        text: String,
        plain: bool,
        properties: Option<Properties>,
        at: usize,
    ) -> Result<(), Error> {
        let (anchor, tag) = split(properties);
        let value = self.scalar(text, plain, tag, at)?;
        self.count(1, at)?;
        self.register(anchor, &value);
        self.fill(value, at)
    }

    /// Close the innermost block collection and fill its parent's slot.
    fn close_top(&mut self) -> Result<(), Error> {
        let (value, anchor, start) = match self.stack.pop() {
            Some(Frame::Sequence {
                mut items,
                open,
                anchor,
                start,
                ..
            }) => {
                if open {
                    items.push(Value::Null);
                }
                (Value::Array(items), anchor, start)
            }
            Some(Frame::Mapping {
                mut object,
                mut keys,
                slot,
                anchor,
                start,
                ..
            }) => {
                match slot {
                    Slot::Value(key, key_at) | Slot::KeyDone(key, key_at) => {
                        if !keys.insert(key.clone()) {
                            return Err(self.error(ErrorKind::DuplicateKey, key_at));
                        }
                        object.push(key, Value::Null);
                    }
                    Slot::ExplicitKey => return Err(self.error(ErrorKind::NonStringKey, start)),
                    Slot::Idle => {}
                }
                (Value::Object(object), anchor, start)
            }
            _ => unreachable!("the root frame is never closed here"),
        };
        self.depth -= 1;
        self.register(anchor, &value);
        self.fill(value, start)
    }

    /// Open a block mapping at `column` whose first key is the scalar `text`
    /// (`plain` when it resolves by the core schema).
    fn open_mapping(
        &mut self,
        column: usize,
        text: String,
        plain: bool,
        key_at: usize,
        line: Line,
    ) -> Result<(), Error> {
        if line == Line::Inline {
            return Err(self.error(
                ErrorKind::Syntax(
                    "a line break before a block mapping (none may start on this line)",
                ),
                key_at,
            ));
        }
        // Properties on the key's line belong to the key, and the mapping
        // starts where they do; properties on an earlier line belong to the
        // mapping.
        let (mapping, key_properties, indent) = match self.pending.take() {
            Some(properties) if self.line_start(properties.at) == self.line_start(key_at) => {
                let indent = properties.column;
                (None, Some(properties), indent)
            }
            other => (other, None, column),
        };
        let (key_anchor, key_tag) = split(key_properties);
        let key = self.key_text(text, plain, key_tag, key_at)?;
        if let Some(name) = key_anchor {
            self.anchor(name, Value::String(key.clone()));
        }
        let (anchor, tag) = split(mapping);
        self.collection_tag(tag, Tag::Map)?;
        self.enter(key_at)?;
        self.count(1, key_at)?;
        self.stack.push(Frame::Mapping {
            indent,
            start: key_at,
            object: Object::new(),
            keys: HashSet::default(),
            slot: Slot::Value(key, key_at),
            anchor,
        });
        Ok(())
    }

    /// A block mapping's implicit key, the cursor on it, through its `:`.
    fn block_key(&mut self) -> Result<(String, usize), Error> {
        let mut properties = None;
        while matches!(self.peek(), Some(b'&' | b'!')) {
            self.property(&mut properties)?;
            self.skip_inline_space();
        }
        let at = self.pos;
        let (text, plain) = match self.peek() {
            Some(b'*') => {
                if properties.is_some() {
                    return Err(self.error(ErrorKind::Syntax("an alias without properties"), at));
                }
                let Some(key) = string(self.alias()?) else {
                    return Err(self.error(ErrorKind::NonStringKey, at));
                };
                (key, false)
            }
            Some(b'"' | b'\'') => {
                let (key, multiline) = self.quoted()?;
                if multiline {
                    return Err(self.error(ErrorKind::Syntax("an implicit key on one line"), at));
                }
                (key, false)
            }
            Some(b'[' | b'{') => return Err(self.error(ErrorKind::NonStringKey, at)),
            Some(b'|' | b'>') => {
                return Err(self.error(
                    ErrorKind::Syntax("a mapping key; a block scalar is not one"),
                    at,
                ));
            }
            _ if self.at_indicator(b'-') => {
                return Err(self.error(
                    ErrorKind::Syntax("a mapping key at a mapping's indentation"),
                    at,
                ));
            }
            _ => (self.plain_line().to_owned(), true),
        };
        let (anchor, tag) = split(properties);
        let key = self.key_text(text, plain, tag, at)?;
        if !self.take_value_indicator() {
            return Err(self.error(ErrorKind::Syntax("`:` after a mapping key"), self.pos));
        }
        self.count(1, at)?;
        if let Some(name) = anchor {
            self.anchor(name, Value::String(key.clone()));
        }
        Ok((key, at))
    }

    /// The node at the cursor, which fills the innermost open slot (owned by
    /// a block indented `owner`).
    fn node(&mut self, owner: isize, line: &mut Line) -> Result<(), Error> {
        let at = self.pos;
        let column = self.column(at);
        match self.bytes[at] {
            b'&' | b'!' => {
                let mut properties = self.pending.take();
                self.property(&mut properties)?;
                self.pending = properties;
            }
            b'*' => {
                if self.pending.is_some() {
                    return Err(self.error(ErrorKind::Syntax("an alias without properties"), at));
                }
                let value = self.alias()?;
                if self.take_value_indicator() {
                    let Some(key) = string(value) else {
                        return Err(self.error(ErrorKind::NonStringKey, at));
                    };
                    self.open_mapping(column, key, false, at, *line)?;
                    *line = Line::Inline;
                } else {
                    self.fill(value, at)?;
                }
            }
            b'-' | b'?' if self.blank_at(at + 1) => {
                if *line == Line::Inline {
                    return Err(self.error(
                        ErrorKind::Syntax(
                            "a line break before a block collection (none may start on this line)",
                        ),
                        at,
                    ));
                }
                let sequence = self.bytes[at] == b'-';
                let (anchor, tag) = split(self.pending.take());
                self.collection_tag(tag, if sequence { Tag::Seq } else { Tag::Map })?;
                self.enter(at)?;
                self.stack.push(if sequence {
                    Frame::Sequence {
                        indent: column,
                        start: at,
                        items: Vec::new(),
                        open: true,
                        anchor,
                    }
                } else {
                    Frame::Mapping {
                        indent: column,
                        start: at,
                        object: Object::new(),
                        keys: HashSet::default(),
                        slot: Slot::ExplicitKey,
                        anchor,
                    }
                });
                self.pos += 1;
                *line = Line::Compact;
            }
            b':' if self.blank_at(at + 1) => return Err(self.error(ErrorKind::NonStringKey, at)),
            b'[' | b'{' => {
                let properties = self.pending.take();
                let value = self.flow(properties)?;
                if self.take_value_indicator() {
                    return Err(self.error(ErrorKind::NonStringKey, at));
                }
                self.fill(value, at)?;
            }
            b'|' | b'>' => {
                let text = self.block_scalar(owner)?;
                let properties = self.pending.take();
                self.fill_scalar(text, false, properties, at)?;
                *line = Line::Fresh;
            }
            b',' | b']' | b'}' | b'%' | b'@' | b'`' => {
                return Err(self.error(
                    ErrorKind::Syntax("a node; a plain scalar cannot start with this indicator"),
                    at,
                ));
            }
            quote => {
                let (text, plain, multiline) = if matches!(quote, b'"' | b'\'') {
                    let (text, multiline) = self.quoted()?;
                    (text, false, multiline)
                } else {
                    (self.plain_line().to_owned(), true, false)
                };
                if self.take_value_indicator() {
                    if multiline {
                        return Err(
                            self.error(ErrorKind::Syntax("an implicit key on one line"), at)
                        );
                    }
                    self.open_mapping(column, text, plain, at, *line)?;
                    *line = Line::Inline;
                } else {
                    let text = if plain {
                        self.plain_rest(text, owner)?
                    } else {
                        text
                    };
                    let properties = self.pending.take();
                    self.fill_scalar(text, plain, properties, at)?;
                }
            }
        }
        Ok(())
    }

    /// The next entry of the innermost block collection that continues at
    /// the cursor's column: `-` of a sequence, or a mapping's key, `?` or `:`.
    fn entry(&mut self, line: &mut Line) -> Result<(), Error> {
        let at = self.pos;
        let column = self.column(at);
        loop {
            match self.stack.last() {
                Some(Frame::Root(_)) | None => {
                    return Err(self.error(
                        ErrorKind::Syntax("the end of the document after its top-level node"),
                        at,
                    ));
                }
                Some(Frame::Sequence { indent, .. }) => {
                    if column == *indent && self.at_indicator(b'-') {
                        break;
                    }
                    if column <= *indent {
                        self.close_top()?;
                        continue;
                    }
                    return Err(self.error(
                        ErrorKind::Syntax("a sequence entry at its sequence's indentation"),
                        at,
                    ));
                }
                Some(Frame::Mapping { indent, .. }) => {
                    if column == *indent {
                        break;
                    }
                    if column < *indent {
                        self.close_top()?;
                        continue;
                    }
                    return Err(self.error(
                        ErrorKind::Syntax("a mapping entry at its mapping's indentation"),
                        at,
                    ));
                }
            }
        }
        let explicit = self.at_indicator(b'?');
        let value = self.at_indicator(b':');
        let pending_key = match self.stack.last_mut() {
            Some(Frame::Sequence { open, .. }) => {
                self.pos += 1;
                *open = true;
                *line = Line::Compact;
                return Ok(());
            }
            Some(Frame::Mapping { slot, .. }) => match mem::replace(slot, Slot::Idle) {
                Slot::KeyDone(key, key_at) if value => {
                    *slot = Slot::Value(key, key_at);
                    self.pos += 1;
                    *line = Line::Compact;
                    return Ok(());
                }
                Slot::KeyDone(key, key_at) => Some((key, key_at)),
                _ if value => return Err(self.error(ErrorKind::NonStringKey, at)),
                _ => None,
            },
            _ => unreachable!("a collection continues here"),
        };
        if let Some((key, key_at)) = pending_key {
            // An explicit key with no `:` line has a null value.
            let Some(Frame::Mapping { object, keys, .. }) = self.stack.last_mut() else {
                unreachable!("the innermost collection is a mapping");
            };
            if !keys.insert(key.clone()) {
                return Err(self.error(ErrorKind::DuplicateKey, key_at));
            }
            object.push(key, Value::Null);
        }
        if explicit {
            let Some(Frame::Mapping { slot, .. }) = self.stack.last_mut() else {
                unreachable!("the innermost collection is a mapping");
            };
            *slot = Slot::ExplicitKey;
            self.pos += 1;
            *line = Line::Compact;
            return Ok(());
        }
        let (key, key_at) = self.block_key()?;
        let Some(Frame::Mapping { slot, .. }) = self.stack.last_mut() else {
            unreachable!("the innermost collection is a mapping");
        };
        *slot = Slot::Value(key, key_at);
        *line = Line::Inline;
        Ok(())
    }

    fn document(&mut self) -> Result<Value, Error> {
        if self.text.starts_with('\u{feff}') {
            self.pos = 3;
        }
        // The prologue: comments, directives, and the document start marker.
        let mut directives = false;
        let mut line = Line::Fresh;
        loop {
            self.skip_separation();
            if self.pos < self.bytes.len()
                && self.peek() == Some(b'%')
                && self.pos == self.line_start(self.pos)
            {
                directives = true;
                self.skip_to_line_end();
                continue;
            }
            if self.at_marker() && self.bytes[self.pos] == b'-' {
                self.pos += 3;
                line = Line::Inline;
            } else if directives {
                return Err(self.error(ErrorKind::Syntax("`---` after the directives"), self.pos));
            }
            break;
        }
        loop {
            if self.skip_separation() {
                line = Line::Fresh;
            }
            if self.pos >= self.bytes.len() || self.at_marker() {
                break;
            }
            self.check_indentation()?;
            if let Some((owner, mapping_value)) = self.open_slot() {
                let column = self.column(self.pos) as isize;
                let here = line != Line::Fresh
                    || column > owner
                    || (mapping_value && column == owner && self.at_indicator(b'-'));
                if here {
                    self.node(owner, &mut line)?;
                } else {
                    let properties = self.pending.take();
                    self.fill_scalar(String::new(), true, properties, self.pos)?;
                }
            } else {
                if line != Line::Fresh {
                    return Err(self.error(
                        ErrorKind::Syntax("a line break after a complete node"),
                        self.pos,
                    ));
                }
                self.entry(&mut line)?;
            }
        }
        // Close every open collection; an open slot holds null.
        let end = self.pos;
        loop {
            if self.open_slot().is_some() {
                let properties = self.pending.take();
                self.fill_scalar(String::new(), true, properties, end)?;
            }
            if self.stack.len() == 1 {
                break;
            }
            self.close_top()?;
        }
        // Only comments may follow the document (after an optional `...`).
        if self.pos < self.bytes.len()
            && self.bytes[self.pos..].starts_with(b"...")
            && self.at_marker()
        {
            self.pos += 3;
            self.skip_separation();
        }
        if self.pos < self.bytes.len() {
            return Err(self.error(ErrorKind::MultipleDocuments, self.pos));
        }
        match self.stack.pop() {
            Some(Frame::Root(value)) => Ok(value.unwrap_or(Value::Null)),
            _ => unreachable!("only the root frame remains"),
        }
    }
}

/// The text of a string value.
fn string(mut value: Value) -> Option<String> {
    match &mut value {
        Value::String(text) => Some(mem::take(text)),
        _ => None,
    }
}

/// Read `text` as one YAML 1.2 document under [`Limits::DEFAULT`].
///
/// ```rust
/// use purrdf_lex::yaml::{self, ErrorKind};
///
/// let value = yaml::read("name: x  # a comment\nlist:\n- 1.50\n- 'true'\n- {a: [b, c]}\ntext: |\n  one\n  two\n").unwrap();
/// assert_eq!(value["list"][0].as_number().unwrap().lexeme(), "1.50");
/// assert_eq!(value["list"][1], "true");
/// assert_eq!(value["list"][2]["a"][1], "c");
/// assert_eq!(value["text"], "one\ntwo\n");
/// assert_eq!(yaml::read("a: 1\na: 2").unwrap_err().kind(), ErrorKind::DuplicateKey);
/// ```
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read(text: &str) -> Result<Value, Error> {
    read_with(text, Limits::DEFAULT)
}

/// Read `text` as one YAML 1.2 document under `limits`.
///
/// # Errors
///
/// [`Error`] at the first refusal.
pub fn read_with(text: &str, limits: Limits) -> Result<Value, Error> {
    let mut parser = Parser {
        text,
        bytes: text.as_bytes(),
        pos: 0,
        limits,
        nodes: 0,
        depth: 0,
        anchors: HashMap::default(),
        stack: vec![Frame::Root(None)],
        pending: None,
        located: Cell::new((0, 0, 0)),
    };
    parser.document()
}

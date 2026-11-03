// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The deterministic YAML emitter.
//!
//! A [`Value`] is flattened into the event stream a YAML serializer produces
//! (mapping and sequence starts and ends, and scalars with a requested style),
//! and the stream drives a block-style emitter whose layout decisions — where a
//! line breaks, how far a node is indented, when a scalar is quoted and how —
//! follow the reference emitter (libyaml) that `serde_yaml` drives, with no
//! line-width folding. Both the flattening and the emitter are loops over heap
//! stacks.

use crate::json::Value;

use super::resolve::needs_quotes;

/// The indentation unit.
const INDENT: i32 = 2;

/// The scalar style a serializer requests; the emitter may choose a more
/// quoted one when the text does not allow it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Style {
    Any,
    Plain,
    Single,
    Double,
    Literal,
}

enum Event<'v> {
    SequenceStart,
    SequenceEnd,
    MappingStart,
    MappingEnd,
    Scalar(&'v str, Style),
}

/// The style a string requests: literal for multi-line text, single-quoted for
/// text that would otherwise read back as another type, and double-quoted for
/// text holding U+2028 or U+2029 (which a YAML 1.2 reader keeps as content, so
/// no other style can place them safely).
fn string_style(text: &str) -> Style {
    if text.contains(['\u{2028}', '\u{2029}']) {
        Style::Double
    } else if text.contains('\n') {
        Style::Literal
    } else if needs_quotes(text) {
        Style::Single
    } else {
        Style::Any
    }
}

/// The event stream of `root`, over a heap work list.
fn events(root: &Value) -> Vec<Event<'_>> {
    enum Job<'v> {
        Node(&'v Value),
        Event(Event<'v>),
    }
    let mut out = Vec::new();
    let mut jobs = vec![Job::Node(root)];
    while let Some(job) = jobs.pop() {
        let value = match job {
            Job::Event(event) => {
                out.push(event);
                continue;
            }
            Job::Node(value) => value,
        };
        match value {
            Value::Null => out.push(Event::Scalar("null", Style::Plain)),
            Value::Bool(true) => out.push(Event::Scalar("true", Style::Plain)),
            Value::Bool(false) => out.push(Event::Scalar("false", Style::Plain)),
            Value::Number(number) => out.push(Event::Scalar(number.lexeme(), Style::Plain)),
            Value::String(text) => out.push(Event::Scalar(text, string_style(text))),
            Value::Array(items) => {
                out.push(Event::SequenceStart);
                jobs.push(Job::Event(Event::SequenceEnd));
                jobs.extend(items.iter().rev().map(Job::Node));
            }
            Value::Object(object) => {
                out.push(Event::MappingStart);
                jobs.push(Job::Event(Event::MappingEnd));
                for (name, member) in object.iter().rev() {
                    jobs.push(Job::Node(member));
                    jobs.push(Job::Event(Event::Scalar(name, string_style(name))));
                }
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    DocumentContent,
    DocumentEnd,
    BlockSequenceFirstItem,
    BlockSequenceItem,
    BlockMappingFirstKey,
    BlockMappingKey,
    BlockMappingSimpleValue,
    BlockMappingValue,
    FlowSequenceFirstItem,
    FlowMappingFirstKey,
}

/// What the analysis of a scalar allows.
#[allow(
    clippy::struct_excessive_bools,
    reason = "the reference emitter's per-scalar verdicts, one flag each"
)]
#[derive(Clone, Copy, Debug, Default)]
struct Analysis {
    length: usize,
    multiline: bool,
    flow_plain_allowed: bool,
    block_plain_allowed: bool,
    single_quoted_allowed: bool,
    block_allowed: bool,
}

const fn is_break(c: char) -> bool {
    matches!(c, '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

const fn is_blankz(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(c) => matches!(c, ' ' | '\t' | '\0') || is_break(c),
    }
}

/// The characters the emitter writes raw (every other one is escaped in a
/// double-quoted scalar).
const fn is_printable(c: char) -> bool {
    matches!(c, '\n' | ' '..='~' | '\u{a0}'..='\u{d7ff}' | '\u{10000}'..)
        || (matches!(c, '\u{e000}'..='\u{fffd}') && c != '\u{feff}')
}

/// Which styles `text` allows.
fn analyze(text: &str) -> Analysis {
    let mut analysis = Analysis {
        length: text.len(),
        ..Analysis::default()
    };
    if text.is_empty() {
        analysis.block_plain_allowed = true;
        analysis.single_quoted_allowed = true;
        return analysis;
    }
    let mut block_indicators = text.starts_with("---") || text.starts_with("...");
    let mut flow_indicators = block_indicators;
    let (mut line_breaks, mut special) = (false, false);
    let (mut leading_space, mut leading_break) = (false, false);
    let (mut trailing_space, mut trailing_break) = (false, false);
    let (mut break_space, mut space_break) = (false, false);
    let (mut previous_space, mut previous_break) = (false, false);
    let mut chars = text.chars().peekable();
    let mut preceded_by_whitespace = true;
    let mut index = 0;
    while let Some(c) = chars.next() {
        let first = index == 0;
        let last = chars.peek().is_none();
        let followed_by_whitespace = is_blankz(chars.peek().copied());
        if first {
            if matches!(
                c,
                '#' | ','
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '&'
                    | '*'
                    | '!'
                    | '|'
                    | '>'
                    | '\''
                    | '"'
                    | '%'
                    | '@'
                    | '`'
            ) {
                flow_indicators = true;
                block_indicators = true;
            }
            if matches!(c, '?' | ':') {
                flow_indicators = true;
                if followed_by_whitespace {
                    block_indicators = true;
                }
            }
            if c == '-' && followed_by_whitespace {
                flow_indicators = true;
                block_indicators = true;
            }
        } else {
            if matches!(c, ',' | '?' | '[' | ']' | '{' | '}') {
                flow_indicators = true;
            }
            if c == ':' {
                flow_indicators = true;
                if followed_by_whitespace {
                    block_indicators = true;
                }
            }
            if c == '#' && preceded_by_whitespace {
                flow_indicators = true;
                block_indicators = true;
            }
        }
        if !is_printable(c) {
            special = true;
        }
        if is_break(c) {
            line_breaks = true;
        }
        if c == ' ' {
            leading_space |= first;
            trailing_space |= last;
            break_space |= previous_break;
            previous_space = true;
            previous_break = false;
        } else if is_break(c) {
            leading_break |= first;
            trailing_break |= last;
            space_break |= previous_space;
            previous_space = false;
            previous_break = true;
        } else {
            previous_space = false;
            previous_break = false;
        }
        preceded_by_whitespace = is_blankz(Some(c));
        index += 1;
    }
    analysis.multiline = line_breaks;
    analysis.flow_plain_allowed = true;
    analysis.block_plain_allowed = true;
    analysis.single_quoted_allowed = true;
    analysis.block_allowed = true;
    if leading_space || leading_break || trailing_space || trailing_break {
        analysis.flow_plain_allowed = false;
        analysis.block_plain_allowed = false;
    }
    if trailing_space {
        analysis.block_allowed = false;
    }
    if break_space {
        analysis.flow_plain_allowed = false;
        analysis.block_plain_allowed = false;
        analysis.single_quoted_allowed = false;
    }
    if space_break || special {
        analysis.flow_plain_allowed = false;
        analysis.block_plain_allowed = false;
        analysis.single_quoted_allowed = false;
        analysis.block_allowed = false;
    }
    if line_breaks {
        analysis.flow_plain_allowed = false;
        analysis.block_plain_allowed = false;
    }
    if flow_indicators {
        analysis.flow_plain_allowed = false;
    }
    if block_indicators {
        analysis.block_plain_allowed = false;
    }
    analysis
}

#[allow(
    clippy::struct_excessive_bools,
    reason = "the reference emitter's cursor state: independent flags it tests and sets one by one"
)]
struct Emitter<'v> {
    events: Vec<Event<'v>>,
    at: usize,
    out: String,
    measure: Option<usize>,
    overflow: bool,
    literal_hint: u32,
    state: State,
    states: Vec<State>,
    indent: i32,
    indents: Vec<i32>,
    flow_level: i32,
    mapping_context: bool,
    simple_key_context: bool,
    column: i32,
    whitespace: bool,
    indention: bool,
    analysis: Analysis,
}

impl Emitter<'_> {
    fn output(&mut self, c: char) {
        if let Some(bytes) = &mut self.measure {
            if let Some(next) = bytes.checked_add(c.len_utf8()) {
                *bytes = next;
            } else {
                self.overflow = true;
            }
        } else {
            self.out.push(c);
        }
    }
    fn put(&mut self, c: char) {
        self.output(c);
        self.column = self.column.saturating_add(1);
    }

    fn put_break(&mut self) {
        self.output('\n');
        self.column = 0;
    }

    fn write_indent(&mut self) {
        let indent = self.indent.max(0);
        if !self.indention || self.column > indent || (self.column == indent && !self.whitespace) {
            self.put_break();
        }
        if self.column < indent
            && let Some(bytes) = &mut self.measure
        {
            let padding =
                usize::try_from(indent - self.column).expect("positive native indentation");
            if let Some(next) = bytes.checked_add(padding) {
                *bytes = next;
            } else {
                self.overflow = true;
            }
            self.column = indent;
        } else {
            while self.column < indent {
                self.put(' ');
            }
        }
        self.whitespace = true;
        self.indention = true;
    }

    fn write_indicator(
        &mut self,
        indicator: &str,
        need_whitespace: bool,
        is_whitespace: bool,
        is_indention: bool,
    ) {
        if need_whitespace && !self.whitespace {
            self.put(' ');
        }
        for c in indicator.chars() {
            self.put(c);
        }
        self.whitespace = is_whitespace;
        self.indention = self.indention && is_indention;
    }

    fn increase_indent(&mut self, flow: bool, indentless: bool) {
        self.indents.push(self.indent);
        if self.indent < 0 {
            self.indent = if flow { INDENT } else { 0 };
        } else if !indentless {
            self.indent += INDENT;
        }
    }

    fn pop_indent(&mut self) {
        self.indent = self.indents.pop().expect("an indent was pushed");
    }

    fn pop_state(&mut self) {
        self.state = self.states.pop().expect("a state was pushed");
    }

    /// Whether the event at `at` starts an empty sequence or mapping.
    fn empty_collection(&self) -> bool {
        matches!(
            (&self.events[self.at], self.events.get(self.at + 1)),
            (Event::SequenceStart, Some(Event::SequenceEnd))
                | (Event::MappingStart, Some(Event::MappingEnd))
        )
    }

    fn check_simple_key(&self) -> bool {
        match &self.events[self.at] {
            Event::Scalar(..) => !self.analysis.multiline && self.analysis.length <= 128,
            Event::SequenceStart | Event::MappingStart => self.empty_collection(),
            _ => false,
        }
    }

    fn run(&mut self) {
        while self.at < self.events.len() {
            if let Event::Scalar(text, _) = self.events[self.at] {
                self.analysis = analyze(text);
            }
            match self.state {
                State::DocumentContent => {
                    self.states.push(State::DocumentEnd);
                    self.node(false, false);
                }
                State::BlockSequenceFirstItem => self.block_sequence_item(true),
                State::BlockSequenceItem => self.block_sequence_item(false),
                State::BlockMappingFirstKey => self.block_mapping_key(true),
                State::BlockMappingKey => self.block_mapping_key(false),
                State::BlockMappingSimpleValue => self.block_mapping_value(true),
                State::BlockMappingValue => self.block_mapping_value(false),
                State::FlowSequenceFirstItem => self.empty_flow("[", "]"),
                State::FlowMappingFirstKey => self.empty_flow("{", "}"),
                State::DocumentEnd => unreachable!("the document holds one node"),
            }
            self.at += 1;
        }
        // The implicit document end: a final line break. `serde_yaml` takes
        // its output before the stream end, so no `...` follows a final
        // keep-chomped block scalar.
        self.write_indent();
    }

    fn node(&mut self, mapping: bool, simple_key: bool) {
        self.mapping_context = mapping;
        self.simple_key_context = simple_key;
        match self.events[self.at] {
            Event::Scalar(text, style) => self.scalar(text, style),
            Event::SequenceStart => {
                self.state = if self.flow_level != 0 || self.empty_collection() {
                    State::FlowSequenceFirstItem
                } else {
                    State::BlockSequenceFirstItem
                };
            }
            Event::MappingStart => {
                self.state = if self.flow_level != 0 || self.empty_collection() {
                    State::FlowMappingFirstKey
                } else {
                    State::BlockMappingFirstKey
                };
            }
            Event::SequenceEnd | Event::MappingEnd => unreachable!("a node starts here"),
        }
    }

    /// An empty collection, `[]` or `{}`: the only flow collections written.
    fn empty_flow(&mut self, open: &str, close: &str) {
        self.write_indicator(open, true, true, false);
        self.increase_indent(true, false);
        self.pop_indent();
        self.write_indicator(close, false, false, false);
        self.pop_state();
    }

    fn block_sequence_item(&mut self, first: bool) {
        if first {
            self.increase_indent(false, self.mapping_context && !self.indention);
        }
        if matches!(self.events[self.at], Event::SequenceEnd) {
            self.pop_indent();
            self.pop_state();
            return;
        }
        self.write_indent();
        self.write_indicator("-", true, false, true);
        self.states.push(State::BlockSequenceItem);
        self.node(false, false);
    }

    fn block_mapping_key(&mut self, first: bool) {
        if first {
            self.increase_indent(false, false);
        }
        if matches!(self.events[self.at], Event::MappingEnd) {
            self.pop_indent();
            self.pop_state();
            return;
        }
        self.write_indent();
        if self.check_simple_key() {
            self.states.push(State::BlockMappingSimpleValue);
            self.node(true, true);
        } else {
            self.write_indicator("?", true, false, true);
            self.states.push(State::BlockMappingValue);
            self.node(true, false);
        }
    }

    fn block_mapping_value(&mut self, simple: bool) {
        if simple {
            self.write_indicator(":", false, false, false);
        } else {
            self.write_indent();
            self.write_indicator(":", true, false, true);
        }
        self.states.push(State::BlockMappingKey);
        self.node(true, false);
    }

    fn select_style(&self, requested: Style) -> Style {
        let analysis = self.analysis;
        let mut style = if self.simple_key_context && analysis.multiline {
            Style::Double
        } else if requested == Style::Any {
            Style::Plain
        } else {
            requested
        };
        if style == Style::Plain {
            if (self.flow_level != 0 && !analysis.flow_plain_allowed)
                || (self.flow_level == 0 && !analysis.block_plain_allowed)
            {
                style = Style::Single;
            }
            if analysis.length == 0 && (self.flow_level != 0 || self.simple_key_context) {
                style = Style::Single;
            }
        }
        if style == Style::Single && !analysis.single_quoted_allowed {
            style = Style::Double;
        }
        if style == Style::Literal
            && (!analysis.block_allowed || self.flow_level != 0 || self.simple_key_context)
        {
            style = Style::Double;
        }
        style
    }

    fn scalar(&mut self, text: &str, requested: Style) {
        let style = self.select_style(requested);
        self.increase_indent(true, false);
        match style {
            Style::Plain | Style::Any => self.write_plain(text),
            Style::Single => self.write_single_quoted(text),
            Style::Double => self.write_double_quoted(text),
            Style::Literal => self.write_literal(text),
        }
        self.pop_indent();
        self.pop_state();
    }

    fn write_plain(&mut self, text: &str) {
        if !self.whitespace && (!text.is_empty() || self.flow_level != 0) {
            self.put(' ');
        }
        for c in text.chars() {
            self.put(c);
        }
        self.whitespace = false;
        self.indention = false;
    }

    fn write_single_quoted(&mut self, text: &str) {
        // Only text without line breaks is single-quoted here: a multi-line
        // string requests a literal block, and one holding a line separator a
        // double-quoted scalar.
        self.write_indicator("'", true, false, false);
        for c in text.chars() {
            if c == '\'' {
                self.put('\'');
            }
            self.put(c);
        }
        self.write_indicator("'", false, false, false);
        self.whitespace = false;
        self.indention = false;
    }

    fn write_double_quoted(&mut self, text: &str) {
        self.write_indicator("\"", true, false, false);
        for c in text.chars() {
            if !is_printable(c) || c == '\u{feff}' || is_break(c) || c == '"' || c == '\\' {
                self.put('\\');
                let short = match c {
                    '\0' => Some('0'),
                    '\u{7}' => Some('a'),
                    '\u{8}' => Some('b'),
                    '\t' => Some('t'),
                    '\n' => Some('n'),
                    '\u{b}' => Some('v'),
                    '\u{c}' => Some('f'),
                    '\r' => Some('r'),
                    '\u{1b}' => Some('e'),
                    '"' => Some('"'),
                    '\\' => Some('\\'),
                    '\u{85}' => Some('N'),
                    '\u{a0}' => Some('_'),
                    '\u{2028}' => Some('L'),
                    '\u{2029}' => Some('P'),
                    _ => None,
                };
                if let Some(short) = short {
                    self.put(short);
                } else {
                    let code = u32::from(c);
                    let (letter, width) = if code <= 0xff {
                        ('x', 2)
                    } else if code <= 0xffff {
                        ('u', 4)
                    } else {
                        ('U', 8)
                    };
                    self.put(letter);
                    for shift in (0..width).rev() {
                        let digit = (code >> (shift * 4)) & 0xf;
                        self.put(
                            char::from_digit(digit, 16)
                                .expect("a nibble")
                                .to_ascii_uppercase(),
                        );
                    }
                }
            } else {
                self.put(c);
            }
        }
        self.write_indicator("\"", false, false, false);
        self.whitespace = false;
        self.indention = false;
    }

    fn write_literal(&mut self, text: &str) {
        self.write_indicator("|", true, false, false);
        // Block scalar hints: an indentation indicator when the text starts
        // with a space or a break, and a chomping indicator.
        if text.chars().next().is_some_and(|c| c == ' ' || is_break(c)) {
            self.put(char::from_digit(self.literal_hint, 10).expect("a digit"));
        }
        let mut tail = text.chars().rev();
        let last = tail.next();
        let before = tail.next();
        let chomp = match (last, before) {
            (None, _) => Some("-"),
            (Some(last), _) if !is_break(last) => Some("-"),
            (Some(_), None) => Some("+"),
            (_, Some(before)) if is_break(before) => Some("+"),
            _ => None,
        };
        if let Some(chomp) = chomp {
            self.write_indicator(chomp, false, false, false);
        }
        self.put_break();
        self.indention = true;
        self.whitespace = true;
        let mut breaks = true;
        for c in text.chars() {
            if is_break(c) {
                if c == '\n' {
                    self.put_break();
                } else {
                    self.output(c);
                    self.column = 0;
                }
                self.indention = true;
                breaks = true;
            } else {
                if breaks {
                    self.write_indent();
                }
                self.put(c);
                self.indention = false;
                breaks = false;
            }
        }
    }
}

/// `value` as a YAML 1.2 document in block style.
///
/// The output is a pure function of the value, and a YAML reader reads it back
/// to the same value (strings stay strings, numbers keep their lexemes). The
/// layout is `serde_yaml`'s: mappings one `key: value` per line, nested
/// collections indented two spaces, a sequence inside a mapping at the
/// mapping's own indentation, empty collections as `[]` and `{}`, a key longer
/// than 128 bytes or holding a line break as an explicit `? key` entry, a
/// multi-line string as a literal block scalar, and a final line break.
///
/// ```rust
/// use purrdf_lex::{json, yaml};
///
/// let value = json::read(r#"{"id": "x", "tags": ["a", "true"], "n": 1.50, "doc": "one\ntwo\n", "e": {}}"#).unwrap();
/// assert_eq!(
///     yaml::write(&value),
///     "id: x\ntags:\n- a\n- 'true'\nn: 1.50\ndoc: |\n  one\n  two\ne: {}\n"
/// );
/// ```
pub fn write(value: &Value) -> String {
    let mut emitter = Emitter {
        events: events(value),
        at: 0,
        out: String::new(),
        measure: None,
        overflow: false,
        literal_hint: INDENT as u32,
        state: State::DocumentContent,
        states: Vec::new(),
        indent: -1,
        indents: Vec::new(),
        flow_level: 0,
        mapping_context: false,
        simple_key_context: false,
        column: 0,
        whitespace: true,
        indention: true,
        analysis: Analysis::default(),
    };
    emitter.run();
    emitter.out
}

/// A scalar spelling requested by a layout-policy caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitStyle {
    /// Choose a spelling that retains the scalar's string value.
    Auto,
    /// Prefer an unquoted spelling; unsafe spellings become quoted.
    Plain,
    /// Prefer single quotes; multiline or special text becomes double quoted.
    SingleQuoted,
    /// Use the native YAML double-quoted escape table.
    DoubleQuoted,
    /// Prefer a literal block; flow and key contexts become double quoted.
    Literal,
}
/// The surrounding native layout constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarContext {
    /// Block value with absolute parent and content indentation.
    Block {
        /// Indentation of the owning block (zero for a top-level scalar).
        parent_indentation: usize,
        /// Absolute content indentation, greater than its parent's.
        indentation: usize,
    },
    /// A value inside brackets or braces.
    Flow,
    /// A simple mapping key on one source line, at most 1,024 source characters.
    Key,
}
/// Explicit scalar output and allocation bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmitLimits {
    /// Maximum generated UTF-8 bytes.
    pub max_output_bytes: usize,
    /// Maximum admitted output buffer capacity.
    pub max_allocation_bytes: usize,
}
/// An emitted scalar and the style actually selected.
#[derive(Debug, PartialEq, Eq)]
pub struct EmittedScalar {
    /// Native spelling; literal blocks include header and content indentation.
    pub text: String,
    /// The chosen presentation, including a safe fallback when needed.
    pub style: EmitStyle,
}
/// A position-independent emission refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitError {
    /// The requested indentation cannot describe a block scalar.
    Indentation,
    /// The complete emitted simple key exceeds YAML's source-character bound.
    KeyLength {
        /// Maximum source characters, including quotes and escape spellings.
        limit: usize,
    },
    /// Native storage arithmetic or allocation failed.
    Storage(crate::allocation::StorageError),
    /// Generated bytes exceed the caller's bound.
    Output {
        /// Caller bound.
        limit: usize,
    },
    /// Output buffer capacity exceeds the caller's allocation bound.
    Allocation {
        /// Caller bound.
        limit: usize,
    },
}
impl core::fmt::Display for EmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Indentation => f.write_str(
                "block content indentation must exceed its parent and fit a native column",
            ),
            Self::KeyLength { limit } => {
                write!(f, "YAML simple key exceeds {limit} source characters")
            }
            Self::Storage(cause) => cause.fmt(f),
            Self::Output { limit } => write!(f, "YAML scalar output exceeds {limit} bytes"),
            Self::Allocation { limit } => write!(f, "YAML scalar allocation exceeds {limit} bytes"),
        }
    }
}
impl purrdf_hash::error::StructuredError for EmitError {
    purrdf_hash::error_visit! {
        Indentation => "yaml-emission-indentation" {},
        KeyLength { limit } => "yaml-emission-key-length" { "limit" => Unsigned(*limit as u128) },
        Storage(cause) => "yaml-emission-storage" { "cause" => Cause(cause) },
        Output { limit } => "yaml-emission-output-limit" { "limit" => ByteCount(*limit as u128) },
        Allocation { limit } => "yaml-emission-allocation-limit" { "limit" => ByteCount(*limit as u128) },
    }
}
purrdf_hash::error_sources!(EmitError, Storage);

/// Emit one string scalar through the same quoting/escaping bodies as
/// [`write`](fn@write), reporting the actual safe style. Output is measured without
/// allocation, then its complete buffer is admitted before writing.
///
/// # Errors
/// Refuses invalid block indentation, an overlong simple key, output/allocation
/// exhaustion and native storage failure. No partial spelling is returned.
pub fn emit_scalar(
    text: &str,
    requested: EmitStyle,
    context: ScalarContext,
    limits: EmitLimits,
) -> Result<EmittedScalar, EmitError> {
    let (indent, hint) = match context {
        ScalarContext::Block {
            parent_indentation,
            indentation,
        } => {
            let increment = indentation
                .checked_sub(parent_indentation)
                .filter(|&increment| increment > 0)
                .ok_or(EmitError::Indentation)?;
            let indent = i32::try_from(indentation).map_err(|_| EmitError::Indentation)?;
            (indent, increment)
        }
        ScalarContext::Flow | ScalarContext::Key => (0, 0),
    };
    let analysis = analyze(text);
    let plain = !text.is_empty()
        && !needs_quotes(text)
        && match context {
            ScalarContext::Block { .. } => analysis.block_plain_allowed,
            ScalarContext::Flow | ScalarContext::Key => analysis.flow_plain_allowed,
        };
    let single = analysis.single_quoted_allowed && !text.chars().any(is_break);
    let needs_hint = text.starts_with(' ') || text.chars().next().is_some_and(is_break);
    let literal = matches!(context, ScalarContext::Block { .. })
        && (!needs_hint || hint < 10)
        && analysis.block_allowed
        && !text.contains(['\r', '\u{85}', '\u{2028}', '\u{2029}']);
    let style = match requested {
        EmitStyle::DoubleQuoted => EmitStyle::DoubleQuoted,
        EmitStyle::Literal if literal => EmitStyle::Literal,
        EmitStyle::SingleQuoted if single => EmitStyle::SingleQuoted,
        EmitStyle::Plain | EmitStyle::Auto if plain => EmitStyle::Plain,
        EmitStyle::Auto if literal && text.contains('\n') => EmitStyle::Literal,
        EmitStyle::Auto | EmitStyle::Plain if single => EmitStyle::SingleQuoted,
        _ => EmitStyle::DoubleQuoted,
    };
    let mut emitter = Emitter {
        events: Vec::new(),
        at: 0,
        out: String::new(),
        measure: Some(0),
        overflow: false,
        literal_hint: u32::try_from(hint).unwrap_or(2),
        state: State::DocumentContent,
        states: Vec::new(),
        indent,
        indents: Vec::new(),
        flow_level: 0,
        mapping_context: false,
        simple_key_context: false,
        column: 0,
        whitespace: true,
        indention: true,
        analysis,
    };
    let emit = |emitter: &mut Emitter<'_>| match style {
        EmitStyle::Plain => {
            for c in text.chars() {
                emitter.put(c);
            }
        }
        EmitStyle::SingleQuoted => emitter.write_single_quoted(text),
        EmitStyle::Literal => emitter.write_literal(text),
        EmitStyle::Auto | EmitStyle::DoubleQuoted => emitter.write_double_quoted(text),
    };
    emit(&mut emitter);
    if emitter.overflow {
        return Err(EmitError::Storage(
            crate::allocation::StorageError::SizeOverflow,
        ));
    }
    if matches!(context, ScalarContext::Key) && emitter.column > 1024 {
        return Err(EmitError::KeyLength { limit: 1024 });
    }
    let bytes = emitter.measure.expect("measurement mode");
    if bytes > limits.max_output_bytes {
        return Err(EmitError::Output {
            limit: limits.max_output_bytes,
        });
    }
    let mut account = super::admission::Allocations::bounded(limits.max_allocation_bytes);
    account
        .memory(|memory| memory.reserve_string(&mut emitter.out, bytes))
        .map_err(|cause| {
            if cause == crate::allocation::StorageError::AdmissionFailed {
                EmitError::Allocation {
                    limit: limits.max_allocation_bytes,
                }
            } else {
                EmitError::Storage(cause)
            }
        })?;
    emitter.measure = None;
    emitter.column = 0;
    emitter.whitespace = true;
    emitter.indention = true;
    emit(&mut emitter);
    Ok(EmittedScalar {
        text: emitter.out,
        style,
    })
}

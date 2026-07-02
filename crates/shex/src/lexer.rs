// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A hand-rolled ShExC (ShEx 2.1 §6) tokenizer.
//!
//! The token set covers the full ShExC terminal grammar: IRI references
//! (UCHAR-decoded, strictly validated), prefixed names with `%HH` and
//! `PN_LOCAL_ESC` escapes, blank-node labels, language tags, the four string
//! literal forms, signed numeric literals, `/regex/flags` patterns, the
//! composite `%name{ code %}` semantic-action token, the composite
//! `{m,n}` repeat range, and the punctuation set.
//!
//! Context-sensitive corners handled here so the parser stays purely
//! token-driven:
//!
//! * `@` is a language tag (`@en-UK`), the empty language stem sigil (`@~`),
//!   or the shape-reference sigil (`@<iri>` / `@pfx:local` / `@_:b`); the
//!   lexer resolves this with bounded lookahead ([`Token::LangTag`] vs
//!   [`Token::At`]).
//! * `/` opens a regex unless doubled (`//`, the annotation marker).
//! * `{` opens a shape body unless immediately followed by a digit, in which
//!   case it must complete a `REPEAT_RANGE` terminal.
//! * `%` opens a semantic action (`%iri{ … %}` / `%iri%`), lexed atomically
//!   because the code block is raw text up to an unescaped `%}`.
//!
//! Every token carries its source byte span so the parser can report
//! [`ShexError::Syntax`] at a precise offset. The lexer never panics: all
//! malformed input is a typed [`ShexError::Lex`].

use std::sync::OnceLock;

use regex::Regex;

use crate::error::{Result, ShexError};

/// A lexical token. Payload-bearing variants keep the *lexical* form where the
/// grammar cares about it (numeric literals in value sets keep their spelling);
/// keyword recognition is left to the parser, which matches [`Token::Word`]
/// case-insensitively (except `a`, `true` and `false`, which ShExC treats
/// case-sensitively).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// An `IRIREF`: the content between `<` and `>`, UCHAR-unescaped.
    Iri(String),
    /// A prefixed name `prefix:local` (`local` has `PN_LOCAL_ESC` escapes
    /// removed; `%HH` sequences are kept verbatim).
    PName(String, String),
    /// A `_:label` blank node (label without the `_:`).
    BNode(String),
    /// A `@langtag`.
    LangTag(String),
    /// A string literal's unescaped content (quote style is not retained).
    StringLit(String),
    /// An `INTEGER` (lexical form, sign included).
    Integer(String),
    /// A `DECIMAL` (lexical form).
    Decimal(String),
    /// A `DOUBLE` (lexical form).
    Double(String),
    /// A `/pattern/flags` regex with `\/` unescaped and UCHARs decoded.
    Regex {
        /// The XPath regex source.
        pattern: String,
        /// The `[smix]*` flags.
        flags: String,
    },
    /// A whole semantic action `%name{ code %}` or `%name%`.
    Code {
        /// The extension name (IRI or prefixed name, expanded by the parser).
        name: CodeName,
        /// The unescaped code text; `None` for the `%name%` form.
        code: Option<String>,
    },
    /// A `REPEAT_RANGE` `{m}` / `{m,}` / `{m,n}` / `{m,*}`; `max == -1` means
    /// unbounded.
    Repeat {
        /// Minimum occurrence count.
        min: i64,
        /// Maximum occurrence count (`-1` = unbounded).
        max: i64,
    },
    /// An alphabetic word: a keyword, `a`, or a boolean literal.
    Word(String),

    /// `@` when not part of a language tag (shape refs, `@~`).
    At,
    /// `=`
    Eq,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `.`
    Dot,
    /// `;`
    Semi,
    /// `|`
    Pipe,
    /// `*`
    Star,
    /// `+`
    Plus,
    /// `?`
    Question,
    /// `^` (inverse sense flag)
    Caret,
    /// `^^` (datatype marker)
    HatHat,
    /// `~`
    Tilde,
    /// `-`
    Minus,
    /// `$` (triple-expression label sigil)
    Dollar,
    /// `&` (triple-expression inclusion sigil)
    Amp,
    /// `//` (annotation marker)
    AnnotMarker,
}

/// The name half of a [`Token::Code`] semantic action, prior to prefix
/// expansion (which needs the parser's prefix map).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeName {
    /// `%<iri>{ … %}`
    Iri(String),
    /// `%pfx:local{ … %}`
    PName(String, String),
}

/// A token plus its half-open source byte span `[start, end)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned {
    /// The token.
    pub token: Token,
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

/// Tokenize a full ShExC document into a flat token stream. Whitespace and
/// `#`-comments are dropped. Returns [`ShexError::Lex`] on the first
/// malformed token.
pub fn tokenize(input: &str) -> Result<Vec<Spanned>> {
    Lexer::new(input).run()
}

/// `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*` (body, without the `@`).
fn langtag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("^[a-zA-Z]+(-[a-zA-Z0-9]+)*$").unwrap_or_else(|_| unreachable!()))
}

struct Lexer<'a> {
    src: &'a str,
    chars: Vec<(usize, char)>,
    /// Index into `chars`.
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.char_indices().collect(),
            pos: 0,
        }
    }

    /// Byte offset of the char at `chars[i]`, or `src.len()` at end.
    fn byte_at(&self, i: usize) -> usize {
        self.chars.get(i).map_or(self.src.len(), |&(b, _)| b)
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).map(|&(_, c)| c)
    }

    fn cur(&self) -> Option<char> {
        self.peek(0)
    }

    fn run(mut self) -> Result<Vec<Spanned>> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia();
            let start = self.byte_at(self.pos);
            let Some(c) = self.cur() else { break };
            let token = self.lex_one(c, start)?;
            let end = self.byte_at(self.pos);
            out.push(Spanned { token, start, end });
        }
        Ok(out)
    }

    /// Skip whitespace, `#` line comments and `/* … */` block comments.
    ///
    /// `/*` always opens a comment: a REGEXP beginning with a literal `*`
    /// would be ambiguous, and the reference grammar resolves it the same
    /// way (`*` at the start of a pattern is meaningless in XPath regexes).
    fn skip_trivia(&mut self) {
        loop {
            match self.cur() {
                Some(c) if c.is_whitespace() => self.pos += 1,
                Some('#') => {
                    while let Some(c) = self.cur() {
                        self.pos += 1;
                        if c == '\n' {
                            break;
                        }
                    }
                }
                Some('/') if self.peek(1) == Some('*') => {
                    self.pos += 2;
                    while let Some(c) = self.cur() {
                        if c == '*' && self.peek(1) == Some('/') {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_one(&mut self, c: char, start: usize) -> Result<Token> {
        match c {
            '<' => self.lex_iriref(start),
            '"' | '\'' => self.lex_string(c, start),
            '@' => Ok(self.lex_at(start)?),
            '_' if self.peek(1) == Some(':') => self.lex_bnode(start),
            ':' => {
                self.pos += 1;
                let local = self.lex_pn_local();
                Ok(Token::PName(String::new(), local))
            }
            '{' => self.lex_brace_or_repeat(start),
            '%' => self.lex_code(start),
            '/' => self.lex_annot_or_regex(start),
            '^' => {
                self.pos += 1;
                if self.cur() == Some('^') {
                    self.pos += 1;
                    Ok(Token::HatHat)
                } else {
                    Ok(Token::Caret)
                }
            }
            '0'..='9' => Ok(self.lex_number()),
            '+' | '-' if self.starts_signed_number() => Ok(self.lex_number()),
            '.' if matches!(self.peek(1), Some('0'..='9')) => Ok(self.lex_number()),
            '=' => self.single(Token::Eq),
            '}' => self.single(Token::RBrace),
            '(' => self.single(Token::LParen),
            ')' => self.single(Token::RParen),
            '[' => self.single(Token::LBracket),
            ']' => self.single(Token::RBracket),
            '.' => self.single(Token::Dot),
            ';' => self.single(Token::Semi),
            '|' => self.single(Token::Pipe),
            '*' => self.single(Token::Star),
            '+' => self.single(Token::Plus),
            '?' => self.single(Token::Question),
            '~' => self.single(Token::Tilde),
            '-' => self.single(Token::Minus),
            '$' => self.single(Token::Dollar),
            '&' => self.single(Token::Amp),
            _ if is_pn_chars_base(c) => Ok(self.lex_word_or_pname()),
            _ => Err(ShexError::lex(format!("unexpected character {c:?}"), start)),
        }
    }

    fn single(&mut self, t: Token) -> Result<Token> {
        self.pos += 1;
        Ok(t)
    }

    /// Does the `+`/`-` at the cursor start a numeric literal (`[+-]` directly
    /// followed by a digit, or by `.` and a digit)?
    fn starts_signed_number(&self) -> bool {
        match self.peek(1) {
            Some('0'..='9') => true,
            Some('.') => matches!(self.peek(2), Some('0'..='9')),
            _ => false,
        }
    }

    /// `IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'` — strict, no
    /// operator fallback (ShExC has no `<` comparison operator).
    fn lex_iriref(&mut self, start: usize) -> Result<Token> {
        self.pos += 1; // `<`
        let mut content = String::new();
        loop {
            let Some(c) = self.cur() else {
                return Err(ShexError::lex("unterminated IRI reference", start));
            };
            match c {
                '>' => {
                    self.pos += 1;
                    return Ok(Token::Iri(content));
                }
                '\\' => {
                    let decoded = self.read_uchar(start)?;
                    content.push(decoded);
                }
                '\u{0}'..='\u{20}' | '<' | '"' | '{' | '}' | '|' | '^' | '`' => {
                    return Err(ShexError::lex(
                        format!("character {c:?} is not allowed in an IRI reference"),
                        self.byte_at(self.pos),
                    ));
                }
                _ => {
                    content.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    /// Read a `\uXXXX` / `\UXXXXXXXX` escape at the cursor (which sits on the
    /// backslash), advancing past it. Anything else after the backslash is a
    /// hard error.
    fn read_uchar(&mut self, err_at: usize) -> Result<char> {
        let width = match self.peek(1) {
            Some('u') => 4,
            Some('U') => 8,
            _ => {
                return Err(ShexError::lex(
                    "backslash must start a \\u/\\U escape here",
                    self.byte_at(self.pos),
                ));
            }
        };
        let mut value: u32 = 0;
        for k in 0..width {
            let d = self
                .peek(2 + k)
                .and_then(|c| c.to_digit(16))
                .ok_or_else(|| {
                    ShexError::lex("bad \\u/\\U escape (expected hex digits)", err_at)
                })?;
            value = value * 16 + d;
        }
        let decoded = char::from_u32(value)
            .ok_or_else(|| ShexError::lex("\\u/\\U escape is not a Unicode scalar", err_at))?;
        self.pos += 2 + width;
        Ok(decoded)
    }

    fn lex_string(&mut self, quote: char, start: usize) -> Result<Token> {
        let long = self.peek(1) == Some(quote) && self.peek(2) == Some(quote);
        self.pos += if long { 3 } else { 1 };
        let mut value = String::new();
        loop {
            let Some(c) = self.cur() else {
                return Err(ShexError::lex("unterminated string literal", start));
            };
            if c == '\\' {
                match self.peek(1) {
                    Some('t') => value.push('\t'),
                    Some('b') => value.push('\u{0008}'),
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('f') => value.push('\u{000C}'),
                    Some('"') => value.push('"'),
                    Some('\'') => value.push('\''),
                    Some('\\') => value.push('\\'),
                    Some('u' | 'U') => {
                        let decoded = self.read_uchar(start)?;
                        value.push(decoded);
                        continue;
                    }
                    other => {
                        return Err(ShexError::lex(
                            format!(
                                "bad string escape \\{}",
                                other.map_or_else(String::new, String::from)
                            ),
                            self.byte_at(self.pos),
                        ));
                    }
                }
                self.pos += 2;
                continue;
            }
            if c == quote {
                if long {
                    if self.peek(1) == Some(quote) && self.peek(2) == Some(quote) {
                        self.pos += 3;
                        return Ok(Token::StringLit(value));
                    }
                    value.push(c);
                    self.pos += 1;
                    continue;
                }
                self.pos += 1;
                return Ok(Token::StringLit(value));
            }
            if !long && matches!(c, '\n' | '\r') {
                return Err(ShexError::lex("raw newline in short string literal", start));
            }
            value.push(c);
            self.pos += 1;
        }
    }

    /// `@` — a `LANGTAG`, or the bare shape-reference / language-stem sigil.
    fn lex_at(&mut self, start: usize) -> Result<Token> {
        // Try a LANGTAG body: scan the maximal [a-zA-Z0-9-] run after `@`.
        let mut i = self.pos + 1;
        let mut body = String::new();
        while let Some(&(_, c)) = self.chars.get(i) {
            if c.is_ascii_alphanumeric() || c == '-' {
                body.push(c);
                i += 1;
            } else {
                break;
            }
        }
        let next = self.chars.get(i).map(|&(_, c)| c);
        if body.is_empty() || next == Some(':') {
            // `@<iri>`, `@_:b`, `@pfx:local`, `@~`: a bare sigil; the label
            // is lexed as its own following token.
            self.pos += 1;
            return Ok(Token::At);
        }
        if !langtag_re().is_match(&body) {
            return Err(ShexError::lex(
                format!("malformed language tag @{body}"),
                start,
            ));
        }
        self.pos = i;
        Ok(Token::LangTag(body))
    }

    /// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
    fn lex_bnode(&mut self, start: usize) -> Result<Token> {
        self.pos += 2; // `_:`
        let first = self
            .cur()
            .filter(|&c| is_pn_chars_u(c) || c.is_ascii_digit());
        let Some(first) = first else {
            return Err(ShexError::lex("malformed blank node label", start));
        };
        let mut label = String::new();
        label.push(first);
        self.pos += 1;
        let mut trailing_dots = 0usize;
        while let Some(c) = self.cur() {
            if c == '.' {
                label.push(c);
                trailing_dots += 1;
                self.pos += 1;
            } else if is_pn_chars(c) {
                label.push(c);
                trailing_dots = 0;
                self.pos += 1;
            } else {
                break;
            }
        }
        if trailing_dots > 0 {
            label.truncate(label.len() - trailing_dots);
            self.pos -= trailing_dots;
        }
        Ok(Token::BNode(label))
    }

    /// `{` opens a shape body, unless a digit follows immediately — then it
    /// must complete a `REPEAT_RANGE` terminal (no interior whitespace).
    fn lex_brace_or_repeat(&mut self, start: usize) -> Result<Token> {
        if !matches!(self.peek(1), Some('0'..='9')) {
            return self.single(Token::LBrace);
        }
        self.pos += 1; // `{`
        let min = self.lex_repeat_int(start)?;
        match self.cur() {
            Some('}') => {
                self.pos += 1;
                Ok(Token::Repeat { min, max: min })
            }
            Some(',') => {
                self.pos += 1;
                let max = match self.cur() {
                    Some('}') => -1,
                    Some('*') => {
                        self.pos += 1;
                        -1
                    }
                    Some('0'..='9') => self.lex_repeat_int(start)?,
                    _ => {
                        return Err(ShexError::lex("malformed repeat range", start));
                    }
                };
                if self.cur() == Some('}') {
                    self.pos += 1;
                    Ok(Token::Repeat { min, max })
                } else {
                    Err(ShexError::lex("unterminated repeat range", start))
                }
            }
            _ => Err(ShexError::lex("malformed repeat range", start)),
        }
    }

    fn lex_repeat_int(&mut self, start: usize) -> Result<i64> {
        let mut value: i64 = 0;
        let mut any = false;
        while let Some(c) = self.cur() {
            let Some(d) = c.to_digit(10) else { break };
            value = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(i64::from(d)))
                .ok_or_else(|| ShexError::lex("repeat range bound overflows", start))?;
            any = true;
            self.pos += 1;
        }
        if any {
            Ok(value)
        } else {
            Err(ShexError::lex("expected integer in repeat range", start))
        }
    }

    /// `%name{ code %}` / `%name%` — the whole semantic action as one token.
    fn lex_code(&mut self, start: usize) -> Result<Token> {
        self.pos += 1; // `%`
        self.skip_trivia();
        let name = match self.cur() {
            Some('<') => {
                let Token::Iri(iri) = self.lex_iriref(start)? else {
                    unreachable!()
                };
                CodeName::Iri(iri)
            }
            Some(':') => {
                self.pos += 1;
                CodeName::PName(String::new(), self.lex_pn_local())
            }
            Some(c) if is_pn_chars_base(c) => match self.lex_word_or_pname() {
                Token::PName(p, l) => CodeName::PName(p, l),
                _ => {
                    return Err(ShexError::lex("expected IRI after '%'", start));
                }
            },
            _ => {
                return Err(ShexError::lex("expected IRI after '%'", start));
            }
        };
        self.skip_trivia();
        match self.cur() {
            Some('%') => {
                self.pos += 1;
                Ok(Token::Code { name, code: None })
            }
            Some('{') => {
                self.pos += 1;
                let code = self.lex_code_body(start)?;
                Ok(Token::Code {
                    name,
                    code: Some(code),
                })
            }
            _ => Err(ShexError::lex(
                "expected '{' or '%' after semantic action name",
                start,
            )),
        }
    }

    /// `CODE ::= '{' ([^%\] | '\' [%\] | UCHAR)* '%' '}'` — raw text with
    /// `\%`, `\\` and UCHAR escapes decoded.
    fn lex_code_body(&mut self, start: usize) -> Result<String> {
        let mut code = String::new();
        loop {
            let Some(c) = self.cur() else {
                return Err(ShexError::lex("unterminated semantic action code", start));
            };
            match c {
                '%' => {
                    if self.peek(1) == Some('}') {
                        self.pos += 2;
                        return Ok(code);
                    }
                    return Err(ShexError::lex(
                        "'%' in semantic action code must be escaped as \\% or close the block",
                        self.byte_at(self.pos),
                    ));
                }
                '\\' => match self.peek(1) {
                    Some('%') => {
                        code.push('%');
                        self.pos += 2;
                    }
                    Some('\\') => {
                        code.push('\\');
                        self.pos += 2;
                    }
                    Some('u' | 'U') => {
                        let decoded = self.read_uchar(start)?;
                        code.push(decoded);
                    }
                    _ => {
                        return Err(ShexError::lex(
                            "bad escape in semantic action code",
                            self.byte_at(self.pos),
                        ));
                    }
                },
                _ => {
                    code.push(c);
                    self.pos += 1;
                }
            }
        }
    }

    /// `//` annotation marker, or a `/pattern/flags` regex.
    fn lex_annot_or_regex(&mut self, start: usize) -> Result<Token> {
        if self.peek(1) == Some('/') {
            self.pos += 2;
            return Ok(Token::AnnotMarker);
        }
        self.pos += 1; // `/`
        let mut pattern = String::new();
        loop {
            let Some(c) = self.cur() else {
                return Err(ShexError::lex("unterminated regex", start));
            };
            match c {
                '/' => {
                    self.pos += 1;
                    break;
                }
                '\n' | '\r' => {
                    return Err(ShexError::lex("raw newline in regex", start));
                }
                '\\' => match self.peek(1) {
                    // `\/` denotes a literal `/`; every other permitted escape
                    // is preserved verbatim for the XPath regex engine.
                    Some('/') => {
                        pattern.push('/');
                        self.pos += 2;
                    }
                    Some(
                        e @ ('n' | 'r' | 't' | '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{'
                        | '}' | '$' | '-' | '[' | ']' | '^'),
                    ) => {
                        pattern.push('\\');
                        pattern.push(e);
                        self.pos += 2;
                    }
                    Some('u' | 'U') => {
                        let decoded = self.read_uchar(start)?;
                        pattern.push(decoded);
                    }
                    other => {
                        return Err(ShexError::lex(
                            format!(
                                "bad regex escape \\{}",
                                other.map_or_else(String::new, String::from)
                            ),
                            self.byte_at(self.pos),
                        ));
                    }
                },
                _ => {
                    pattern.push(c);
                    self.pos += 1;
                }
            }
        }
        if pattern.is_empty() {
            return Err(ShexError::lex("empty regex", start));
        }
        let mut flags = String::new();
        while let Some(c @ ('s' | 'm' | 'i' | 'x')) = self.cur() {
            flags.push(c);
            self.pos += 1;
        }
        Ok(Token::Regex { pattern, flags })
    }

    /// `INTEGER | DECIMAL | DOUBLE` with optional sign; the trailing-dot
    /// statement terminator is never absorbed (`5.` lexes as `5` `.` unless an
    /// exponent follows).
    fn lex_number(&mut self) -> Token {
        let begin = self.pos;
        if matches!(self.cur(), Some('+' | '-')) {
            self.pos += 1;
        }
        while matches!(self.cur(), Some('0'..='9')) {
            self.pos += 1;
        }
        let mut has_frac = false;
        if self.cur() == Some('.') {
            let digits_after = matches!(self.peek(1), Some('0'..='9'));
            // `5.e3` is a valid DOUBLE; a bare `5.` leaves the dot alone.
            let exponent_after = matches!(self.peek(1), Some('e' | 'E')) && self.exp_has_digits(2);
            if digits_after || (exponent_after && self.pos > begin) {
                has_frac = true;
                self.pos += 1;
                while matches!(self.cur(), Some('0'..='9')) {
                    self.pos += 1;
                }
            }
        }
        let mut has_exp = false;
        if matches!(self.cur(), Some('e' | 'E')) && self.exp_has_digits(1) {
            has_exp = true;
            self.pos += 1;
            if matches!(self.cur(), Some('+' | '-')) {
                self.pos += 1;
            }
            while matches!(self.cur(), Some('0'..='9')) {
                self.pos += 1;
            }
        }
        let lexical: String = self.chars[begin..self.pos]
            .iter()
            .map(|&(_, c)| c)
            .collect();
        if has_exp {
            Token::Double(lexical)
        } else if has_frac {
            Token::Decimal(lexical)
        } else {
            Token::Integer(lexical)
        }
    }

    /// Do the chars at `self.pos + offset` form a valid exponent body
    /// (`[0-9]+` or `[+-][0-9]+`)? Called while the char at
    /// `self.pos + offset - 1` is `e`/`E`.
    fn exp_has_digits(&self, offset: usize) -> bool {
        match self.peek(offset) {
            Some('+' | '-') => matches!(self.peek(offset + 1), Some('0'..='9')),
            Some('0'..='9') => true,
            _ => false,
        }
    }

    /// A word that is a keyword (`PREFIX`, `AND`, `a`, `true`, …) or the
    /// prefix half of a prefixed name.
    fn lex_word_or_pname(&mut self) -> Token {
        let word = self.take_pn_prefix();
        if self.cur() == Some(':') {
            self.pos += 1;
            let local = self.lex_pn_local();
            Token::PName(word, local)
        } else {
            Token::Word(word)
        }
    }

    /// `PN_PREFIX ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?` — trailing
    /// dots are pushed back (they terminate statements / are lex errors in
    /// context).
    fn take_pn_prefix(&mut self) -> String {
        let mut out = String::new();
        let mut trailing_dots = 0usize;
        while let Some(c) = self.cur() {
            if c == '.' {
                out.push(c);
                trailing_dots += 1;
                self.pos += 1;
            } else if is_pn_chars(c) {
                out.push(c);
                trailing_dots = 0;
                self.pos += 1;
            } else {
                break;
            }
        }
        if trailing_dots > 0 {
            out.truncate(out.len() - trailing_dots);
            self.pos -= trailing_dots;
        }
        out
    }

    /// `PN_LOCAL ::= (PN_CHARS_U | ':' | [0-9] | PLX) ((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?`
    ///
    /// `PN_LOCAL_ESC` escapes are decoded to their raw character; `%HH`
    /// percent escapes are kept verbatim (they are part of the IRI). A `%`
    /// not followed by two hex digits, or a `\` not followed by an escapable
    /// character, terminates the local name (and becomes an error in the
    /// caller's context). May legitimately be empty (`ex:`).
    fn lex_pn_local(&mut self) -> String {
        let mut out = String::new();
        let mut trailing_dots = 0usize;
        let mut first = true;
        while let Some(c) = self.cur() {
            match c {
                '\\' => {
                    let Some(esc) = self.peek(1).filter(|&e| is_pn_local_esc(e)) else {
                        break;
                    };
                    out.push(esc);
                    self.pos += 2;
                    trailing_dots = 0;
                }
                '%' => {
                    let two_hex = self.peek(1).is_some_and(|c| c.is_ascii_hexdigit())
                        && self.peek(2).is_some_and(|c| c.is_ascii_hexdigit());
                    if !two_hex {
                        break;
                    }
                    out.push('%');
                    out.extend(self.peek(1));
                    out.extend(self.peek(2));
                    self.pos += 3;
                    trailing_dots = 0;
                }
                '.' if !first => {
                    out.push(c);
                    trailing_dots += 1;
                    self.pos += 1;
                }
                ':' => {
                    out.push(c);
                    trailing_dots = 0;
                    self.pos += 1;
                }
                _ if (first && (is_pn_chars_u(c) || c.is_ascii_digit()))
                    || (!first && is_pn_chars(c)) =>
                {
                    out.push(c);
                    trailing_dots = 0;
                    self.pos += 1;
                }
                _ => break,
            }
            first = false;
        }
        if trailing_dots > 0 {
            out.truncate(out.len() - trailing_dots);
            self.pos -= trailing_dots;
        }
        out
    }
}

/// The set of characters a `PN_LOCAL_ESC` (`\X`) may escape.
const fn is_pn_local_esc(c: char) -> bool {
    matches!(
        c,
        '_' | '~'
            | '.'
            | '-'
            | '!'
            | '$'
            | '&'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | ';'
            | '='
            | '/'
            | '?'
            | '#'
            | '@'
            | '%'
    )
}

/// `PN_CHARS_BASE` per the ShExC terminal grammar (shared with Turtle/SPARQL).
const fn is_pn_chars_base(c: char) -> bool {
    matches!(c,
        'A'..='Z'
        | 'a'..='z'
        | '\u{C0}'..='\u{D6}'
        | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}'
        | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}')
}

/// `PN_CHARS_U ::= PN_CHARS_BASE | '_'`
const fn is_pn_chars_u(c: char) -> bool {
    is_pn_chars_base(c) || c == '_'
}

/// `PN_CHARS ::= PN_CHARS_U | '-' | [0-9] | #xB7 | [#x300-#x36F] | [#x203F-#x2040]`
const fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c)
        || c.is_ascii_digit()
        || matches!(c, '-' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn toks(s: &str) -> Vec<Token> {
        tokenize(s)
            .unwrap_or_else(|e| panic!("lex {s:?}: {e}"))
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn iriref_strict() {
        assert_eq!(toks("<http://x/y>"), vec![Token::Iri("http://x/y".into())]);
        assert!(tokenize("<http://x/ y>").is_err());
        assert!(tokenize("<http://x/\\n>").is_err());
        assert!(tokenize("<http://x/\\u00zz>").is_err());
        assert_eq!(toks("<a\\u0062c>"), vec![Token::Iri("abc".into())]);
    }

    #[test]
    fn at_disambiguation() {
        assert_eq!(
            toks("\"x\"@en-UK"),
            vec![Token::StringLit("x".into()), Token::LangTag("en-UK".into())]
        );
        assert_eq!(
            toks("@ex:S"),
            vec![Token::At, Token::PName("ex".into(), "S".into())]
        );
        assert_eq!(toks("@~"), vec![Token::At, Token::Tilde]);
        assert_eq!(
            toks("@<http://x/>"),
            vec![Token::At, Token::Iri("http://x/".into())]
        );
    }

    #[test]
    fn repeat_ranges() {
        assert_eq!(toks("{2}"), vec![Token::Repeat { min: 2, max: 2 }]);
        assert_eq!(toks("{2,}"), vec![Token::Repeat { min: 2, max: -1 }]);
        assert_eq!(toks("{2,5}"), vec![Token::Repeat { min: 2, max: 5 }]);
        assert_eq!(toks("{2,*}"), vec![Token::Repeat { min: 2, max: -1 }]);
        assert_eq!(toks("{ <p>"), vec![Token::LBrace, Token::Iri("p".into())]);
    }

    #[test]
    fn regex_vs_annotation() {
        assert_eq!(toks("//"), vec![Token::AnnotMarker]);
        assert_eq!(
            toks(r"/^\/x\t/i"),
            vec![Token::Regex {
                pattern: "^/x\\t".into(),
                flags: "i".into(),
            }]
        );
        assert!(tokenize(r"/\b/").is_err());
    }

    #[test]
    fn code_decls() {
        assert_eq!(
            toks("%<u:a>{ x \\% y %}"),
            vec![Token::Code {
                name: CodeName::Iri("u:a".into()),
                code: Some(" x % y ".into()),
            }]
        );
        assert_eq!(
            toks("%ex:f%"),
            vec![Token::Code {
                name: CodeName::PName("ex".into(), "f".into()),
                code: None,
            }]
        );
        assert!(tokenize("%{ x %}").is_err());
    }

    #[test]
    fn numbers() {
        assert_eq!(toks("5 ."), vec![Token::Integer("5".into()), Token::Dot]);
        assert_eq!(toks("-1"), vec![Token::Integer("-1".into())]);
        assert_eq!(toks("04.50"), vec![Token::Decimal("04.50".into())]);
        assert_eq!(toks("4.5E0"), vec![Token::Double("4.5E0".into())]);
        assert_eq!(toks(".5"), vec![Token::Decimal(".5".into())]);
        assert_eq!(
            toks("123e"),
            vec![Token::Integer("123".into()), Token::Word("e".into())]
        );
        assert_eq!(toks("+-1"), vec![Token::Plus, Token::Integer("-1".into())]);
    }

    #[test]
    fn pnames_and_bnodes() {
        assert_eq!(
            toks("ex:p1- ."),
            vec![Token::PName("ex".into(), "p1-".into()), Token::Dot]
        );
        assert_eq!(
            toks("_:b1. x"),
            vec![
                Token::BNode("b1".into()),
                Token::Dot,
                Token::Word("x".into())
            ]
        );
        assert_eq!(
            toks(r"ex:a\(b\)"),
            vec![Token::PName("ex".into(), "a(b)".into())]
        );
        assert_eq!(
            toks("ex:%41B"),
            vec![Token::PName("ex".into(), "%41B".into())]
        );
    }
}

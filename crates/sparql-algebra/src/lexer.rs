// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A hand-rolled SPARQL 1.1/1.2 tokenizer.
//!
//! Scope is corpus-driven (purrdf S5): the token set covers every construct used
//! across the project's `queries/**/*.rq` and the 51 DSL-generated CONSTRUCT
//! projections — IRIs, prefixed names, variables, blank nodes, RDF literals
//! (plain/typed/`@lang`), the operator/punctuation set, and the RDF 1.2
//! triple-term delimiters `<<` / `>>`.
//!
//! Tokenizing `<` is context-sensitive: it may open an `IRIREF` (`<...>`), be the
//! triple-term open `<<`, the comparison `<=`, or the comparison `<`. The lexer
//! resolves this by *first* attempting a greedy `IRIREF` body scan to a clean
//! `>`; only on failure does it fall back to the two-or-one-char operators.
//!
//! Every token carries its source byte span so the parser can report
//! [`crate::error::ParseError::Syntax`] at a precise offset.

use crate::error::{ParseError, Result};

/// A lexical token. Payload-bearing variants keep the *lexical* form (the AST
/// owns value-space concerns); keyword recognition is left to the parser, which
/// matches [`Token::Word`] case-insensitively (except the rdf:type `a` and the
/// boolean literals, which SPARQL treats case-sensitively).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// An `IRIREF`: the resolved content between `<` and `>` (UCHAR-unescaped).
    Iri(String),
    /// A prefixed name `prefix:local`. `local` is empty for a bare `prefix:`.
    PrefixedName(String, String),
    /// A `?var` / `$var` query variable (name without the sigil).
    Variable(String),
    /// A `_:label` blank node (label without `_:`).
    BlankNodeLabel(String),
    /// An anonymous blank node `[]` (with only whitespace inside).
    Anon,
    /// A short string literal's unescaped content (`'...'` / `"..."`; quote
    /// style is not retained).
    StringLit(String),
    /// A long (triple-quoted) string literal's unescaped content
    /// (`'''...'''` / `"""..."""`). Kept distinct from [`Token::StringLit`] so
    /// grammar productions that admit only short strings — e.g. the SPARQL 1.2
    /// `VersionSpecifier` — can reject the long form.
    LongStringLit(String),
    /// An integer literal (lexical form).
    Integer(String),
    /// A decimal literal (lexical form).
    Decimal(String),
    /// A double literal (lexical form).
    Double(String),
    /// A `@langtag` (raw text after `@`, e.g. `en` or `en--ltr`).
    LangTag(String),
    /// An alphabetic word: a keyword, the rdf:type `a`, or a boolean literal.
    Word(String),

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
    Semicolon,
    /// `,`
    Comma,
    /// `/`
    Slash,
    /// `|`
    Pipe,
    /// `{|` — RDF 1.2 annotation-block open.
    AnnotationOpen,
    /// `|}` — RDF 1.2 annotation-block close.
    AnnotationClose,
    /// `^`
    Caret,
    /// `*`
    Star,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `!`
    Bang,
    /// `?` (path "zero-or-one"; the variable sigil never reaches here)
    Question,
    /// `=`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `^^` (datatype marker)
    HatHat,
    /// `<<` (RDF 1.2 triple-term open)
    TripleOpen,
    /// `>>` (RDF 1.2 triple-term close)
    TripleClose,
    /// `~` (RDF 1.2 reifier marker, e.g. `:s :p :o ~ :r`)
    Tilde,
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

/// Lexer leniency options. These default OFF so [`tokenize`] (the SPARQL entry)
/// stays byte-for-byte unchanged; only an explicitly opted-in caller (the Turtle
/// text codec via [`tokenize_turtle`]) flips them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LexerOptions {
    /// When `true`, a bare `/` is admitted as a `PN_LOCAL` character (so
    /// `purrdf:report/shacl/sarif` tokenizes as ONE prefixed name). This is only
    /// safe in Turtle/TriG term position, where `/` is NOT an operator. In SPARQL
    /// `/` is the property-path sequence operator, so this MUST stay `false` there.
    pub pn_local_allows_slash: bool,
}

/// Tokenize a full SPARQL query string into a flat token stream.
///
/// Whitespace and `#`-comments are dropped. Returns
/// [`ParseError::Lex`] on the first malformed token.
pub fn tokenize(input: &str) -> Result<Vec<Spanned>> {
    Lexer::new(input).run()
}

/// Tokenize Turtle/TriG text, admitting a bare `/` inside `PN_LOCAL` (e.g.
/// `purrdf:report/shacl/sarif`). Turtle has no `/` operator, so this is
/// unambiguous in term position; it differs from [`tokenize`] (SPARQL) ONLY by
/// the [`LexerOptions::pn_local_allows_slash`] flag.
pub fn tokenize_turtle(input: &str) -> Result<Vec<Spanned>> {
    tokenize_with(
        input,
        LexerOptions {
            pn_local_allows_slash: true,
        },
    )
}

/// Tokenize with explicit [`LexerOptions`]. [`tokenize`] is exactly
/// `tokenize_with(input, LexerOptions::default())`.
pub fn tokenize_with(input: &str, options: LexerOptions) -> Result<Vec<Spanned>> {
    Lexer::with_options(input, options).run()
}

/// A byte-cursor tokenizer over the source `str`.
///
/// `pos` is a **byte** offset (always on a UTF-8 char boundary — the lexer only ever
/// advances by whole chars). Working directly on the source bytes avoids the
/// `char_indices().collect()` full-input materialization the prior cursor paid up
/// front, and lets the hot scans (string body, `IRIREF` end, comment tails) run
/// through `memchr`. Token spans are byte offsets, so `pos` *is* the span cursor.
struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    /// Byte offset into `src` (char boundary).
    pos: usize,
    options: LexerOptions,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self::with_options(src, LexerOptions::default())
    }

    fn with_options(src: &'a str, options: LexerOptions) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            options,
        }
    }

    /// The `ahead`-th char from the cursor without consuming (`ahead == 0` is the
    /// current char). Small `ahead` only (0/1/2), so the per-call decode is cheap.
    fn peek(&self, ahead: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(ahead)
    }

    fn cur(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn run(mut self) -> Result<Vec<Spanned>> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia();
            let start = self.pos;
            let Some(c) = self.cur() else { break };
            let token = self.lex_one(c, start)?;
            let end = self.pos;
            out.push(Spanned { token, start, end });
        }
        Ok(out)
    }

    /// Skip whitespace and `#` line comments. The comment tail is skipped with a
    /// single `memchr` to the newline rather than a per-char walk.
    fn skip_trivia(&mut self) {
        loop {
            match self.cur() {
                Some(c) if c.is_whitespace() => self.pos += c.len_utf8(),
                Some('#') => match memchr::memchr(b'\n', &self.bytes[self.pos..]) {
                    // Consume through the newline (byte-identical to the prior
                    // per-char loop, which broke AFTER pushing past '\n').
                    Some(rel) => self.pos += rel + 1,
                    None => self.pos = self.bytes.len(),
                },
                _ => break,
            }
        }
    }

    fn lex_one(&mut self, c: char, start: usize) -> Result<Token> {
        match c {
            '<' => self.lex_lt_or_iri(),
            '>' => Ok(self.two_or_one('>', Token::TripleClose, '=', Token::GtEq, Token::Gt)),
            '"' | '\'' => self.lex_string(c, start),
            // `?` is the variable sigil when a name follows, else the path
            // zero-or-one operator. `$` is only ever a variable sigil.
            '?' if !matches!(self.peek(1), Some(c) if is_varname_char(c)) => {
                self.single(Token::Question)
            }
            '?' | '$' => self.lex_variable(c, start),
            '_' if self.peek(1) == Some(':') => self.lex_blank_label(start),
            ':' => self.lex_prefixed_name(start),
            '@' => self.lex_lang_tag(start),
            '{' if self.peek(1) == Some('|') => {
                self.pos += 2;
                Ok(Token::AnnotationOpen)
            }
            '{' => self.single(Token::LBrace),
            '}' => self.single(Token::RBrace),
            '(' => self.single(Token::LParen),
            ')' => self.single(Token::RParen),
            '[' => self.lex_bracket_or_anon(),
            ']' => self.single(Token::RBracket),
            '.' if !self.next_is_digit() => self.single(Token::Dot),
            ';' => self.single(Token::Semicolon),
            ',' => self.single(Token::Comma),
            '/' => self.single(Token::Slash),
            '|' if self.peek(1) == Some('}') => {
                self.pos += 2;
                Ok(Token::AnnotationClose)
            }
            '|' => Ok(self.two_or_one('|', Token::Or, '\0', Token::Or, Token::Pipe)),
            '^' => Ok(self.two_or_one('^', Token::HatHat, '\0', Token::HatHat, Token::Caret)),
            '*' => self.single(Token::Star),
            '+' => self.single(Token::Plus),
            '-' => self.single(Token::Minus),
            '!' => Ok(self.two_or_one('=', Token::NotEq, '\0', Token::NotEq, Token::Bang)),
            '=' => self.single(Token::Eq),
            '~' => self.single(Token::Tilde),
            '&' => self.lex_and(start),
            '0'..='9' => Ok(self.lex_number()),
            '.' => Ok(self.lex_number()), // a leading-dot decimal like `.5`
            _ if is_pn_chars_base(c) => self.lex_word_or_prefixed(start),
            _ => Err(ParseError::lex(
                format!("unexpected character {c:?}"),
                start,
            )),
        }
    }

    fn single(&mut self, t: Token) -> Result<Token> {
        self.pos += 1;
        Ok(t)
    }

    fn next_is_digit(&self) -> bool {
        matches!(self.peek(1), Some('0'..='9'))
    }

    /// Consume `lead`; if the next char is `two_ch` emit `two`, else if it is
    /// `alt_ch` emit `alt`, else emit `one`.
    fn two_or_one(
        &mut self,
        two_ch: char,
        two: Token,
        alt_ch: char,
        alt: Token,
        one: Token,
    ) -> Token {
        self.pos += 1; // consume the lead char
        match self.cur() {
            Some(c) if c == two_ch => {
                self.pos += 1;
                two
            }
            Some(c) if c == alt_ch => {
                self.pos += 1;
                alt
            }
            _ => one,
        }
    }

    fn lex_and(&mut self, start: usize) -> Result<Token> {
        self.pos += 1;
        if self.cur() == Some('&') {
            self.pos += 1;
            Ok(Token::And)
        } else {
            Err(ParseError::lex("expected '&&'", start))
        }
    }

    /// `<` is `IRIREF` / `<<` / `<=` / `<`. Try a greedy IRIREF body first.
    ///
    /// Fast path: `memchr` the closing `>`. A UCHAR escape (`\uXXXX`) never contains a
    /// literal `>` byte, so the first `>` is always the true `IRIREF` end. When the
    /// body has no backslash (every ordinary IRI), it is emitted VERBATIM as a single
    /// slice after a delimiter-free check — no per-char `String` build. Only a body
    /// carrying a `\` UCHAR escape (or no closing `>`) falls to the decoding scan.
    fn lex_lt_or_iri(&mut self) -> Result<Token> {
        let body_start = self.pos + 1;
        if let Some(rel) = memchr::memchr(b'>', &self.bytes[body_start..]) {
            let end = body_start + rel;
            let body = &self.src[body_start..end];
            if !body.as_bytes().contains(&b'\\') {
                // No escapes: an IRIREF iff no disallowed char appears in the body.
                if body.chars().all(|c| {
                    !c.is_whitespace() && !matches!(c, '<' | '"' | '{' | '}' | '|' | '^' | '`')
                }) {
                    self.pos = end + 1; // consume through '>'
                    return Ok(Token::Iri(body.to_owned()));
                }
                // A disallowed char precedes the '>' → not an IRIREF.
                return Ok(self.two_or_one('<', Token::TripleOpen, '=', Token::LtEq, Token::Lt));
            }
        }
        // Backslash in the body (UCHAR), or no closing '>': decode char by char.
        self.lex_iri_escaped()
    }

    /// The `IRIREF` slow path: a byte-cursor scan that decodes `\uXXXX`/`\UXXXXXXXX`
    /// UCHAR escapes into the resolved content, mirroring the prior char-cursor scan.
    fn lex_iri_escaped(&mut self) -> Result<Token> {
        let mut i = self.pos + 1; // byte offset just past '<'
        let mut content = String::new();
        let mut ok = false;
        while let Some(c) = self.src[i..].chars().next() {
            if c == '>' {
                ok = true;
                i += 1;
                break;
            }
            if c == '\\' {
                // UCHAR escape inside an IRIREF.
                if let Some((consumed, decoded)) = self.read_uchar(i) {
                    content.push(decoded);
                    i += consumed;
                    continue;
                }
                break; // a non-UCHAR backslash is not valid in an IRIREF
            }
            if c.is_whitespace() || matches!(c, '<' | '"' | '{' | '}' | '|' | '^' | '`') {
                break; // disallowed in IRIREF → not an IRIREF
            }
            content.push(c);
            i += c.len_utf8();
        }
        if ok {
            self.pos = i;
            return Ok(Token::Iri(content));
        }
        // Not an IRIREF: fall back to `<<` / `<=` / `<`.
        Ok(self.two_or_one('<', Token::TripleOpen, '=', Token::LtEq, Token::Lt))
    }

    /// Read a `\uXXXX` / `\UXXXXXXXX` escape starting at byte offset `i` (the `\`).
    /// Returns `(bytes_consumed, decoded_char)`. The escape is all-ASCII, so the
    /// byte count equals the char count.
    fn read_uchar(&self, i: usize) -> Option<(usize, char)> {
        let width = match *self.bytes.get(i + 1)? {
            b'u' => 4,
            b'U' => 8,
            _ => return None,
        };
        let mut value: u32 = 0;
        for k in 0..width {
            value = value * 16 + char::from(*self.bytes.get(i + 2 + k)?).to_digit(16)?;
        }
        let decoded = char::from_u32(value)?;
        Some((2 + width, decoded))
    }

    fn lex_string(&mut self, quote: char, start: usize) -> Result<Token> {
        // `quote` is `"` or `'` — ASCII, so its byte is the delimiter to scan for.
        let quote_byte = quote as u8;
        // Long form `"""` / `'''` vs short form.
        let long = self.peek(1) == Some(quote) && self.peek(2) == Some(quote);
        self.pos += if long { 3 } else { 1 };
        let mut value = String::new();
        loop {
            // memchr-forward over the clean run to the next interesting byte: the
            // quote or a `\` escape (both forms), plus a raw CR/LF for the short form
            // (which forbids them). The skipped bytes are literal content, copied
            // wholesale in one `push_str` instead of char by char.
            let tail = &self.bytes[self.pos..];
            let stop = if long {
                memchr::memchr2(quote_byte, b'\\', tail)
            } else {
                min_opt(
                    memchr::memchr2(quote_byte, b'\\', tail),
                    memchr::memchr2(b'\n', b'\r', tail),
                )
            };
            let Some(stop) = stop else {
                return Err(ParseError::lex("unterminated string literal", start));
            };
            if stop > 0 {
                value.push_str(&self.src[self.pos..self.pos + stop]);
                self.pos += stop;
            }
            let c = self.cur().expect("memchr stop is a byte within the source");
            if c == '\\' {
                self.pos += 1;
                let Some(esc) = self.cur() else {
                    return Err(ParseError::lex("unterminated escape", start));
                };
                match esc {
                    't' => value.push('\t'),
                    'n' => value.push('\n'),
                    'r' => value.push('\r'),
                    'b' => value.push('\u{0008}'),
                    'f' => value.push('\u{000C}'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    '\\' => value.push('\\'),
                    'u' | 'U' => {
                        // Re-decode via read_uchar starting at the backslash.
                        let bs = self.pos - 1;
                        if let Some((consumed, decoded)) = self.read_uchar(bs) {
                            value.push(decoded);
                            self.pos = bs + consumed;
                            continue;
                        }
                        return Err(ParseError::lex("bad unicode escape", start));
                    }
                    other => {
                        return Err(ParseError::lex(format!("bad escape \\{other}"), start));
                    }
                }
                self.pos += 1; // the escape char is ASCII
                continue;
            }
            if c == quote {
                if long {
                    if self.peek(1) == Some(quote) && self.peek(2) == Some(quote) {
                        self.pos += 3;
                        return Ok(Token::LongStringLit(value));
                    }
                    // a lone quote inside a long string is literal
                    value.push(c);
                    self.pos += 1;
                    continue;
                }
                self.pos += 1;
                return Ok(Token::StringLit(value));
            }
            // Short form only: `stop` landed on a raw CR/LF. SPARQL STRING_LITERAL1/2
            // forbid raw line breaks (only `'''`/`"""` admit them) — reject.
            return Err(ParseError::lex(
                "raw newline in short string literal",
                start,
            ));
        }
    }

    fn lex_variable(&mut self, _sigil: char, start: usize) -> Result<Token> {
        self.pos += 1; // sigil
        let name = self.take_while(is_varname_char);
        if name.is_empty() {
            return Err(ParseError::lex("empty variable name after sigil", start));
        }
        Ok(Token::Variable(name))
    }

    fn lex_blank_label(&mut self, start: usize) -> Result<Token> {
        self.pos += 2; // `_:`
        let raw = self.take_while(|c| is_pn_chars(c) || c == '.');
        let label = raw.trim_end_matches('.');
        // Push `pos` back over the over-consumed trailing dots: a trailing `.` is
        // the statement terminator, not part of the label. `.` is ASCII (1 byte),
        // so the trimmed byte-length delta equals the dot run.
        self.pos -= raw.len() - label.len();
        if label.is_empty() {
            return Err(ParseError::lex("empty blank node label after `_:`", start));
        }
        Ok(Token::BlankNodeLabel(label.to_string()))
    }

    fn lex_bracket_or_anon(&mut self) -> Result<Token> {
        // `[` optionally `]` (with only whitespace between) → anonymous blank.
        let mut j = self.pos + 1; // byte offset past '['
        while let Some(c) = self.src[j..].chars().next() {
            if c.is_whitespace() {
                j += c.len_utf8();
            } else {
                break;
            }
        }
        if self.src[j..].starts_with(']') {
            self.pos = j + 1; // ']' is ASCII
            Ok(Token::Anon)
        } else {
            self.pos += 1;
            Ok(Token::LBracket)
        }
    }

    fn lex_lang_tag(&mut self, start: usize) -> Result<Token> {
        self.pos += 1; // `@`
        let tag = self.take_while(|c| c.is_ascii_alphanumeric() || c == '-');
        if tag.is_empty() {
            return Err(ParseError::lex("empty language tag", start));
        }
        Ok(Token::LangTag(tag))
    }

    /// A bare `:local` or `:` prefixed name (empty prefix).
    fn lex_prefixed_name(&mut self, _start: usize) -> Result<Token> {
        self.pos += 1; // `:`
        let local = self.take_local();
        Ok(Token::PrefixedName(String::new(), local))
    }

    /// A word that may be a keyword (`SELECT`, `a`, `true`) or the prefix part of
    /// a prefixed name (`purrdf:` / `rdf:type`).
    fn lex_word_or_prefixed(&mut self, _start: usize) -> Result<Token> {
        let word = self.take_pn_prefix();
        if self.cur() == Some(':') {
            self.pos += 1; // `:`
            let local = self.take_local();
            Ok(Token::PrefixedName(word, local))
        } else {
            Ok(Token::Word(word))
        }
    }

    /// Returns `true` when the chars at `self.pos+1` (and optionally `+2`)
    /// constitute a valid SPARQL exponent body, i.e. `[0-9]+` or `[+-][0-9]+`.
    /// Called while `self.cur()` is `e`/`E`.
    fn exp_has_digits(&self) -> bool {
        match self.peek(1) {
            Some('+' | '-') => matches!(self.peek(2), Some('0'..='9')),
            Some('0'..='9') => true,
            _ => false,
        }
    }

    fn lex_number(&mut self) -> Token {
        let begin = self.pos;
        let mut seen_dot = false;
        let mut seen_exp = false;
        while let Some(c) = self.cur() {
            match c {
                '0'..='9' => self.pos += 1,
                '.' if !seen_dot && !seen_exp && self.next_is_digit() => {
                    seen_dot = true;
                    self.pos += 1;
                }
                'e' | 'E' if !seen_exp && self.exp_has_digits() => {
                    seen_exp = true;
                    self.pos += 1;
                    if matches!(self.cur(), Some('+' | '-')) {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        // Numbers are ASCII, so the byte span is the lexical form verbatim.
        let lexical = self.src[begin..self.pos].to_owned();
        if seen_exp {
            Token::Double(lexical)
        } else if seen_dot {
            Token::Decimal(lexical)
        } else {
            Token::Integer(lexical)
        }
    }

    fn take_while(&mut self, pred: impl Fn(char) -> bool) -> String {
        let begin = self.pos;
        while let Some(c) = self.cur() {
            if pred(c) {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        self.src[begin..self.pos].to_owned()
    }

    /// `PN_PREFIX`: starts with a base char, may contain `.`/`-`/digits, must not
    /// end with `.`.
    fn take_pn_prefix(&mut self) -> String {
        let raw = self.take_while(|c| is_pn_chars(c) || c == '.');
        let trimmed = raw.trim_end_matches('.');
        // Push `pos` (a byte offset) back over the over-consumed trailing dots. `.`
        // is ASCII (1 byte), so the trimmed byte-length delta equals that dot run.
        self.pos -= raw.len() - trimmed.len();
        trimmed.to_owned()
    }

    /// `PN_LOCAL`: like a prefix but may also start with a digit or `_`/`:`; must
    /// not end with `.`.
    ///
    /// A backslash starts a `PN_LOCAL_ESC` (SPARQL 1.1 §19.8 / Turtle): `\` followed
    /// by one of `_~.-!$&'()*+,;=/?#@%` denotes that literal character in the local
    /// name (so `dbr:Semantic_analysis_\(linguistics\)` is one prefixed name whose
    /// local part is `Semantic_analysis_(linguistics)`). The escaped character is
    /// emitted UNESCAPED into the returned local — the value-space form the IRI
    /// expansion uses — and never terminates the scan even when it is a delimiter.
    /// A trailing UNescaped `.` is the statement terminator and is pushed back; an
    /// escaped `\.` is a literal dot in PN_LOCAL and is kept.
    fn take_local(&mut self) -> String {
        let mut out = String::new();
        let mut trailing_dots = 0usize;
        while let Some(c) = self.cur() {
            if c == '\\' {
                // PN_LOCAL_ESC: consume the backslash and emit the next char verbatim.
                if let Some(escaped) = self.peek(1).filter(|e| is_pn_local_esc(*e)) {
                    out.push(escaped);
                    self.pos += 2;
                    trailing_dots = 0;
                    continue;
                }
                break; // a non-PN_LOCAL_ESC backslash does not belong to the local name
            }
            if c == '.' {
                // A dot may be internal, but a RUN of trailing dots is the terminator;
                // track the run and trim it after the scan.
                out.push(c);
                trailing_dots += 1;
                self.pos += 1;
                continue;
            }
            if c == '/' && self.options.pn_local_allows_slash {
                // Turtle-only leniency: a bare `/` is a PN_LOCAL char (strict
                // grammar requires `\/`, but oxigraph/purrdf-gts accept the bare
                // form, e.g. `purrdf:report/shacl/sarif`). Turtle has no `/`
                // operator, so this is unambiguous in term position.
                out.push(c);
                trailing_dots = 0;
                self.pos += 1;
                continue;
            }
            if is_pn_chars(c) || c == ':' || c == '%' {
                out.push(c);
                trailing_dots = 0;
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if trailing_dots > 0 {
            // Push back the trailing-dot run: it is the statement terminator.
            out.truncate(out.len() - trailing_dots);
            self.pos -= trailing_dots;
        }
        out
    }
}

/// The smaller of two optional offsets — the earliest of two `memchr` hits, or
/// whichever is present, or `None` when neither matched.
fn min_opt(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (a, b) => a.or(b),
    }
}

/// The set of characters a Turtle/SPARQL `PN_LOCAL_ESC` (`\X`) may escape (§19.8).
fn is_pn_local_esc(c: char) -> bool {
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

fn is_pn_chars_base(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || (c as u32) > 0x7F
}

fn is_pn_chars(c: char) -> bool {
    is_pn_chars_base(c) || c.is_ascii_digit() || c == '-'
}

fn is_varname_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || (c as u32) > 0x7F
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn toks(s: &str) -> Vec<Token> {
        tokenize(s).unwrap().into_iter().map(|s| s.token).collect()
    }

    fn toks_turtle(s: &str) -> Vec<Token> {
        tokenize_turtle(s)
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn iri_vs_operators() {
        assert_eq!(toks("<http://x/y>"), vec![Token::Iri("http://x/y".into())]);
        assert_eq!(toks("?a < ?b"), vec![var("a"), Token::Lt, var("b")]);
        assert_eq!(toks("?a <= ?b"), vec![var("a"), Token::LtEq, var("b")]);
        assert_eq!(toks("?a >= ?b"), vec![var("a"), Token::GtEq, var("b")]);
    }

    #[test]
    fn triple_term_delimiters() {
        assert_eq!(
            toks("<<( ?s ?p ?o )>>"),
            vec![
                Token::TripleOpen,
                Token::LParen,
                var("s"),
                var("p"),
                var("o"),
                Token::RParen,
                Token::TripleClose,
            ]
        );
    }

    #[test]
    fn prefixed_names_and_a() {
        assert_eq!(
            toks("?x a rdf:type ."),
            vec![
                var("x"),
                Token::Word("a".into()),
                Token::PrefixedName("rdf".into(), "type".into()),
                Token::Dot,
            ]
        );
        assert_eq!(
            toks("PREFIX purrdf: <u:>"),
            vec![
                Token::Word("PREFIX".into()),
                Token::PrefixedName("purrdf".into(), String::new()),
                Token::Iri("u:".into()),
            ]
        );
    }

    #[test]
    fn property_path_operators() {
        assert_eq!(
            toks("owl:members/rdf:rest*/rdf:first"),
            vec![
                Token::PrefixedName("owl".into(), "members".into()),
                Token::Slash,
                Token::PrefixedName("rdf".into(), "rest".into()),
                Token::Star,
                Token::Slash,
                Token::PrefixedName("rdf".into(), "first".into()),
            ]
        );
    }

    #[test]
    fn literals_and_lang() {
        assert_eq!(
            toks("\"hi\"@en"),
            vec![Token::StringLit("hi".into()), Token::LangTag("en".into())]
        );
        assert_eq!(
            toks("\"x\"^^xsd:string"),
            vec![
                Token::StringLit("x".into()),
                Token::HatHat,
                Token::PrefixedName("xsd".into(), "string".into()),
            ]
        );
        assert_eq!(toks("3"), vec![Token::Integer("3".into())]);
        assert_eq!(toks("3.5"), vec![Token::Decimal("3.5".into())]);
        assert_eq!(toks("1e9"), vec![Token::Double("1e9".into())]);
    }

    #[test]
    fn string_escapes() {
        assert_eq!(toks(r#""a\tb\n""#), vec![Token::StringLit("a\tb\n".into())]);
        assert_eq!(toks(r#""A""#), vec![Token::StringLit("A".into())]);
    }

    #[test]
    fn comments_skipped() {
        assert_eq!(
            toks("# a comment\nSELECT ?x"),
            vec![Token::Word("SELECT".into()), var("x")]
        );
    }

    #[test]
    fn anon_and_blank() {
        assert_eq!(toks("[]"), vec![Token::Anon]);
        assert_eq!(toks("_:b1"), vec![Token::BlankNodeLabel("b1".into())]);
    }

    #[test]
    fn blank_label_trailing_dot_is_statement_terminator() {
        // A trailing `.` after a blank-node label is the Turtle statement
        // terminator, not part of the label: it must surface as its own `Dot`
        // token and must not be swallowed into the label string.
        assert_eq!(
            toks_turtle(":x :p _:y."),
            vec![
                Token::PrefixedName(String::new(), "x".into()),
                Token::PrefixedName(String::new(), "p".into()),
                Token::BlankNodeLabel("y".into()),
                Token::Dot,
            ]
        );
        // An internal `.` is kept as part of the label; only the final,
        // over-consumed trailing dot is pushed back as the terminator.
        assert_eq!(
            toks_turtle("_:a.b."),
            vec![Token::BlankNodeLabel("a.b".into()), Token::Dot]
        );
    }

    #[test]
    fn not_in_and_neq() {
        assert_eq!(
            toks("?x != ?y && ?z"),
            vec![var("x"), Token::NotEq, var("y"), Token::And, var("z")]
        );
    }

    #[test]
    fn question_is_path_op_or_var_by_lookahead() {
        // `?` before a name char is a variable; otherwise the zero-or-one path op.
        assert_eq!(
            toks("purrdf:p? ?y"),
            vec![
                Token::PrefixedName("purrdf".into(), "p".into()),
                Token::Question,
                var("y"),
            ]
        );
        assert_eq!(toks("?x"), vec![var("x")]);
    }

    #[test]
    fn dot_terminator_not_decimal() {
        assert_eq!(
            toks("?x ?y ?z ."),
            vec![var("x"), var("y"), var("z"), Token::Dot]
        );
    }

    // ── G1 regression: trailing dot must NOT be absorbed into a number ──────

    #[test]
    fn trailing_dot_is_separator_not_decimal() {
        // `3 .` — the dot is a statement separator, not part of the literal.
        assert_eq!(toks("3 ."), vec![Token::Integer("3".into()), Token::Dot]);
    }

    #[test]
    fn number_in_triple_pattern_dot_separator() {
        // Simulates `?o 3 .` — `3` must come out as Integer, not Decimal("3.").
        assert_eq!(
            toks("?o 3 ."),
            vec![var("o"), Token::Integer("3".into()), Token::Dot]
        );
    }

    #[test]
    fn decimal_with_digit_after_dot_still_works() {
        // Smoke-test: `1.5` must remain Decimal.
        assert_eq!(toks("1.5"), vec![Token::Decimal("1.5".into())]);
    }

    // ── G2 regression: exponent requires at least one digit after e/E ────────

    #[test]
    fn double_exponent_no_digit_yields_integer_then_word() {
        // `1e` — no digit follows `e`, so `1` is Integer and `e` is a Word.
        assert_eq!(
            toks("1e"),
            vec![Token::Integer("1".into()), Token::Word("e".into())]
        );
    }

    #[test]
    fn exponent_followed_by_non_digit_word() {
        // `1err` — `e` has no digit after it, so `1` is Integer; `err` is a Word.
        assert_eq!(
            toks("1err"),
            vec![Token::Integer("1".into()), Token::Word("err".into())]
        );
    }

    #[test]
    fn double_exponent_still_works() {
        // Smoke-test: `1e9` must still be Double.
        assert_eq!(toks("1e9"), vec![Token::Double("1e9".into())]);
    }

    #[test]
    fn double_exponent_with_sign_still_works() {
        // Smoke-test: `1.5e-3` must still be Double.
        assert_eq!(toks("1.5e-3"), vec![Token::Double("1.5e-3".into())]);
    }

    #[test]
    fn double_exponent_with_plus_sign_still_works() {
        // `2E+10` must still be Double.
        assert_eq!(toks("2E+10"), vec![Token::Double("2E+10".into())]);
    }

    fn var(n: &str) -> Token {
        Token::Variable(n.into())
    }

    // ── Turtle-only PN_LOCAL slash leniency (default OFF for SPARQL) ─────────

    fn turtle_toks(s: &str) -> Vec<Token> {
        tokenize_turtle(s)
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn turtle_mode_admits_bare_slash_in_pn_local() {
        // `purrdf:report/shacl/sarif` is ONE prefixed name in Turtle mode.
        assert_eq!(
            turtle_toks("purrdf:report/shacl/sarif"),
            vec![Token::PrefixedName(
                "purrdf".into(),
                "report/shacl/sarif".into()
            )]
        );
        assert_eq!(
            turtle_toks("purrdf:projection/okf"),
            vec![Token::PrefixedName(
                "purrdf".into(),
                "projection/okf".into()
            )]
        );
    }

    #[test]
    fn sparql_default_keeps_slash_as_path_operator() {
        // The SPARQL entry (default options) MUST still split on `/` so property
        // paths like `foaf:knows/foaf:name` keep the sequence operator.
        assert_eq!(
            toks("purrdf:report/shacl/sarif"),
            vec![
                Token::PrefixedName("purrdf".into(), "report".into()),
                Token::Slash,
                Token::Word("shacl".into()),
                Token::Slash,
                Token::Word("sarif".into()),
            ]
        );
    }
}

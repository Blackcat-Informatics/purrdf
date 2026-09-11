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

use std::borrow::Cow;

use purrdf_iri::terminals;

use crate::error::{ParseError, Result};

/// A lexical token. Payload-bearing variants keep the *lexical* form (the AST
/// owns value-space concerns); keyword recognition is left to the parser, which
/// matches [`Token::Word`] case-insensitively (except the rdf:type `a` and the
/// boolean literals, which SPARQL treats case-sensitively).
///
/// Tokens are **zero-copy** over the source `str`: the verbatim variants borrow a
/// sub-slice of the input directly (`&'a str`), and the variants that may rewrite
/// bytes (unescaping) carry a [`Cow<'a, str>`] — `Cow::Borrowed` on the common
/// no-escape path and `Cow::Owned` only when an escape actually forced a rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    /// An `IRIREF`: the resolved content between `<` and `>`. `Cow::Borrowed` when
    /// the body has no `UCHAR` escape; `Cow::Owned` when one was decoded.
    Iri(Cow<'a, str>),
    /// A prefixed name `prefix:local`. `local` is empty for a bare `prefix:`. The
    /// prefix is a verbatim slice; the local part is a [`Cow`] because a
    /// `PN_LOCAL_ESC` (`\X`) rewrites it (`Cow::Borrowed` otherwise).
    PrefixedName(&'a str, Cow<'a, str>),
    /// A `?var` / `$var` query variable (name without the sigil).
    Variable(&'a str),
    /// A `_:label` blank node (label without `_:`).
    BlankNodeLabel(&'a str),
    /// An anonymous blank node `[]` (with only whitespace inside).
    Anon,
    /// A short string literal's unescaped content (`'...'` / `"..."`; quote
    /// style is not retained). `Cow::Borrowed` when the body has no escape.
    StringLit(Cow<'a, str>),
    /// A long (triple-quoted) string literal's unescaped content
    /// (`'''...'''` / `"""..."""`). Kept distinct from [`Token::StringLit`] so
    /// grammar productions that admit only short strings — e.g. the SPARQL 1.2
    /// `VersionSpecifier` — can reject the long form.
    LongStringLit(Cow<'a, str>),
    /// An integer literal (lexical form).
    Integer(&'a str),
    /// A decimal literal (lexical form).
    Decimal(&'a str),
    /// A double literal (lexical form).
    Double(&'a str),
    /// A `@langtag` (raw text after `@`, e.g. `en` or `en--ltr`).
    LangTag(&'a str),
    /// An alphabetic word: a keyword, the rdf:type `a`, or a boolean literal.
    Word(&'a str),

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
pub struct Spanned<'a> {
    /// The token.
    pub token: Token<'a>,
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
pub fn tokenize(input: &str) -> Result<Vec<Spanned<'_>>> {
    Lexer::new(input).run()
}

/// Tokenize Turtle/TriG text, admitting a bare `/` inside `PN_LOCAL` (e.g.
/// `purrdf:report/shacl/sarif`). Turtle has no `/` operator, so this is
/// unambiguous in term position; it differs from [`tokenize`] (SPARQL) ONLY by
/// the [`LexerOptions::pn_local_allows_slash`] flag.
pub fn tokenize_turtle(input: &str) -> Result<Vec<Spanned<'_>>> {
    tokenize_with(
        input,
        LexerOptions {
            pn_local_allows_slash: true,
        },
    )
}

/// Tokenize with explicit [`LexerOptions`]. [`tokenize`] is exactly
/// `tokenize_with(input, LexerOptions::default())`.
pub fn tokenize_with(input: &str, options: LexerOptions) -> Result<Vec<Spanned<'_>>> {
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

    /// The byte `ahead` bytes past the cursor, if in bounds.
    ///
    /// Comparing this against an ASCII byte is exactly `peek(ahead) == Some(c)`
    /// for ASCII `c` WHEN every char between the cursor and that offset is ASCII
    /// (so `pos + ahead` is a char start): an ASCII char is its single byte, and a
    /// non-ASCII char's lead/continuation bytes are all `>= 0x80`, so neither side
    /// can match the other. Callers only use it where the cursor char is a known
    /// ASCII token lead and any intermediate byte was itself just matched as
    /// ASCII — no UTF-8 decode per lookahead.
    #[inline]
    fn byte_at(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.pos + ahead).copied()
    }

    fn run(mut self) -> Result<Vec<Spanned<'a>>> {
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

    /// Skip `WS` and `#` line comments. The comment tail is skipped with a
    /// single `memchr` to the newline rather than a per-char walk.
    ///
    /// The whitespace test is [`terminals::is_ws`] — the grammar's
    /// `WS ::= #x20 | #x9 | #xD | #xA` — applied to the raw byte, NOT
    /// [`char::is_whitespace`]. Two separate reasons, and the first is a
    /// correctness one:
    ///
    /// * `char::is_whitespace` answers the Unicode `White_Space` property, which
    ///   names some twenty-odd scalars the grammar does not: U+00A0 NO-BREAK
    ///   SPACE, U+1680, U+2000-U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 and
    ///   U+000B/U+000C. Skipping them between tokens accepts documents no
    ///   conforming processor accepts, and — because none of them is a `PN_CHARS`
    ///   member either — it lets a misplaced NO-BREAK SPACE *silently separate*
    ///   two tokens the grammar would have refused to separate.
    /// * The byte test is exact over UTF-8 for the same reason
    ///   [`terminals::is_iriref_forbidden_byte`] is: every `WS` member is ASCII,
    ///   and no byte of a multi-byte UTF-8 sequence is below `0x80`, so a
    ///   raw-byte comparison can neither miss a member nor alias one. That
    ///   removes a UTF-8 decode and a Unicode-property lookup from the loop that
    ///   runs between *every* pair of tokens in SPARQL, Turtle, TriG, N-Triples
    ///   and N-Quads.
    fn skip_trivia(&mut self) {
        loop {
            match self.byte_at(0) {
                Some(b) if terminals::is_ws(b) => self.pos += 1,
                Some(b'#') => match memchr::memchr(b'\n', &self.bytes[self.pos..]) {
                    // Consume through the newline (byte-identical to the prior
                    // per-char loop, which broke AFTER pushing past '\n').
                    Some(rel) => self.pos += rel + 1,
                    None => self.pos = self.bytes.len(),
                },
                _ => break,
            }
        }
    }

    fn lex_one(&mut self, c: char, start: usize) -> Result<Token<'a>> {
        match c {
            '<' => self.lex_lt_or_iri(),
            '>' => Ok(self.two_or_one('>', Token::TripleClose, '=', Token::GtEq, Token::Gt)),
            '"' | '\'' => self.lex_string(c, start),
            // `?` is the variable sigil when a name follows, else the path
            // zero-or-one operator. `$` is only ever a variable sigil.
            // The disambiguation peek is at the FIRST scalar of `VARNAME`, so it
            // is `is_varname_start` and not the continue class: `?` followed by a
            // combining mark or a MIDDLE DOT begins no variable, and is the path
            // operator.
            '?' if !matches!(self.peek(1), Some(c) if terminals::is_varname_start(c)) => {
                self.single(Token::Question)
            }
            '?' | '$' => self.lex_variable(c, start),
            // Cursor is on the ASCII lead char in each guarded arm below, so the
            // next byte is the next char start (see `byte_at`).
            '_' if self.byte_at(1) == Some(b':') => self.lex_blank_label(start),
            ':' => self.lex_prefixed_name(start),
            '@' => self.lex_lang_tag(start),
            '{' if self.byte_at(1) == Some(b'|') => {
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
            '|' if self.byte_at(1) == Some(b'}') => {
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
            // A keyword and a `PN_PREFIX` both begin at `PN_CHARS_BASE`; the one
            // `PN_CHARS_U` member that is not in it, `'_'`, opens only a
            // `BLANK_NODE_LABEL` and is handled above.
            _ if terminals::is_pn_chars_base(c) => self.lex_word_or_prefixed(start),
            _ => Err(ParseError::lex(unexpected_character(c), start)),
        }
    }

    fn single(&mut self, t: Token<'a>) -> Result<Token<'a>> {
        self.pos += 1;
        Ok(t)
    }

    /// Every caller sits on an ASCII `.`, so the next byte is the next char.
    fn next_is_digit(&self) -> bool {
        matches!(self.byte_at(1), Some(b'0'..=b'9'))
    }

    /// Consume `lead`; if the next char is `two_ch` emit `two`, else if it is
    /// `alt_ch` emit `alt`, else emit `one`.
    fn two_or_one(
        &mut self,
        two_ch: char,
        two: Token<'a>,
        alt_ch: char,
        alt: Token<'a>,
        one: Token<'a>,
    ) -> Token<'a> {
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

    fn lex_and(&mut self, start: usize) -> Result<Token<'a>> {
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
    fn lex_lt_or_iri(&mut self) -> Result<Token<'a>> {
        let body_start = self.pos + 1;
        if let Some(rel) = memchr::memchr(b'>', &self.bytes[body_start..]) {
            let end = body_start + rel;
            let body = &self.src[body_start..end];
            if !body.as_bytes().contains(&b'\\') {
                // No escapes: an IRIREF iff no forbidden char appears in the body. Every
                // character the production forbids raw is ASCII, so a BYTE scan is exact —
                // a UTF-8 lead/continuation byte is always `>= 0x80` and can never alias one.
                if !body
                    .as_bytes()
                    .iter()
                    .copied()
                    .any(terminals::is_iriref_forbidden_byte)
                {
                    self.pos = end + 1; // consume through '>'
                    return Ok(Token::Iri(Cow::Borrowed(body)));
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
    fn lex_iri_escaped(&mut self) -> Result<Token<'a>> {
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
            if terminals::is_iriref_forbidden(c) {
                break; // forbidden raw in IRIREF → not an IRIREF
            }
            content.push(c);
            i += c.len_utf8();
        }
        if ok {
            self.pos = i;
            return Ok(Token::Iri(Cow::Owned(content)));
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

    fn lex_string(&mut self, quote: char, start: usize) -> Result<Token<'a>> {
        // `quote` is `"` or `'` — ASCII, so its byte is the delimiter to scan for.
        let quote_byte = quote as u8;
        // Long form `"""` / `'''` vs short form. The cursor is on the ASCII
        // quote and `pos + 2` is only read once `pos + 1` matched the same ASCII
        // quote, so both lookaheads are byte compares (see `byte_at`).
        let long = self.byte_at(1) == Some(quote_byte) && self.byte_at(2) == Some(quote_byte);
        self.pos += if long { 3 } else { 1 };
        // Fast path: when the body carries no escape and closes cleanly, borrow the
        // literal slice VERBATIM (no per-char `String` build). Anything else — an
        // escape, a raw newline, or an unterminated body — falls to the owned scan
        // below, which builds the `Cow::Owned` value and produces the error messages.
        if let Some((slice, end)) = self.try_borrow_string(quote_byte, long) {
            self.pos = end;
            return Ok(if long {
                Token::LongStringLit(Cow::Borrowed(slice))
            } else {
                Token::StringLit(Cow::Borrowed(slice))
            });
        }
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
                    // `c` is the ASCII quote under the cursor: byte lookahead is
                    // exact here (see `byte_at`).
                    if self.byte_at(1) == Some(quote_byte) && self.byte_at(2) == Some(quote_byte) {
                        self.pos += 3;
                        return Ok(Token::LongStringLit(Cow::Owned(value)));
                    }
                    // a lone quote inside a long string is literal
                    value.push(c);
                    self.pos += 1;
                    continue;
                }
                self.pos += 1;
                return Ok(Token::StringLit(Cow::Owned(value)));
            }
            // Short form only: `stop` landed on a raw CR/LF. SPARQL STRING_LITERAL1/2
            // forbid raw line breaks (only `'''`/`"""` admit them) — reject.
            return Err(ParseError::lex(
                "raw newline in short string literal",
                start,
            ));
        }
    }

    /// The string-literal borrow fast path: from the already-past-open cursor
    /// (`self.pos` at the body start), return `Some((body_slice, end_pos))` when the
    /// body has NO `\` escape and closes cleanly, so the token can borrow the source
    /// slice verbatim. Returns `None` — leaving `self.pos` untouched — when an escape
    /// is present, a short literal hits a raw CR/LF, or the body is unterminated; the
    /// owned scan in [`lex_string`](Self::lex_string) then handles those cases (and
    /// their error messages) identically to before.
    fn try_borrow_string(&self, quote_byte: u8, long: bool) -> Option<(&'a str, usize)> {
        let body_start = self.pos;
        let mut p = body_start;
        loop {
            let tail = &self.bytes[p..];
            if long {
                // Long form: scan to the next quote or `\`. A `\` forces the owned
                // path; a quote closes only when it is a `"""` / `'''` triple —
                // a lone quote is literal content, so scanning continues past it.
                let rel = memchr::memchr2(quote_byte, b'\\', tail)?;
                let at = p + rel;
                if self.bytes[at] == b'\\' {
                    return None;
                }
                if self.bytes.get(at + 1) == Some(&quote_byte)
                    && self.bytes.get(at + 2) == Some(&quote_byte)
                {
                    return Some((&self.src[body_start..at], at + 3));
                }
                p = at + 1;
            } else {
                // Short form: stop at the quote, a `\`, or a raw CR/LF (forbidden).
                let stop = min_opt(
                    memchr::memchr2(quote_byte, b'\\', tail),
                    memchr::memchr2(b'\n', b'\r', tail),
                )?;
                let at = p + stop;
                if self.bytes[at] == quote_byte {
                    return Some((&self.src[body_start..at], at + 1));
                }
                // A `\` escape or a raw newline — defer to the owned scan.
                return None;
            }
        }
    }

    /// `Var ::= VAR1 | VAR2`, i.e. `'?' VARNAME` or `'$' VARNAME`.
    ///
    /// `VARNAME` is position-dependent, so the scan is too: the first scalar must
    /// satisfy [`terminals::is_varname_start`] (`PN_CHARS_U | [0-9]`) and every
    /// later one [`terminals::is_varname_continue`], which adds U+00B7, the
    /// combining diacriticals and the two ties. Scanning the tail with
    /// `is_pn_chars` instead would admit `'-'` and turn `?a-?b` into one variable;
    /// scanning it with the START class would refuse the lawful `?a\u{300}`.
    fn lex_variable(&mut self, _sigil: char, start: usize) -> Result<Token<'a>> {
        self.pos += 1; // sigil
        let begin = self.pos;
        match self.cur() {
            Some(c) if terminals::is_varname_start(c) => self.pos += c.len_utf8(),
            _ => return Err(ParseError::lex("empty variable name after sigil", start)),
        }
        self.take_while(terminals::is_varname_continue);
        Ok(Token::Variable(&self.src[begin..self.pos]))
    }

    /// `BLANK_NODE_LABEL ::= '_:' ( PN_CHARS_U | [0-9] ) ((PN_CHARS|'.')* PN_CHARS)?`.
    ///
    /// A blank node label is *data identity*, so the name class here is held to
    /// the same exactness as a variable's: [`terminals::is_pn_chars`] and nothing
    /// wider. An approximation that swallowed U+00A0 would fuse `_:a<NBSP>b` into
    /// a single node and silently merge two subjects.
    ///
    /// # The production is position-dependent, so the scan is too
    ///
    /// The head is `( PN_CHARS_U | [0-9] )` —
    /// [`terminals::is_blank_node_label_start`] — and it is strictly NARROWER
    /// than the `PN_CHARS` tail. Five scalars may continue a label and may not
    /// begin one (`'-'`, U+00B7, the combining marks `[#x300-#x36F]`, the ties
    /// `[#x203F-#x2040]`), and `'.'` may appear only between name characters.
    /// Scanning the head with the tail class accepted `_:-a`, `_:.a` and
    /// `_:\u{300}a` as labels — and that asymmetry had a sharp edge, because
    /// `purrdf_rdf_core::blank_label::is_valid_blank_node_label` implements the
    /// same production on EGRESS and refuses all three: this parser was reading
    /// labels its own writer would not write, so a round trip through the
    /// workspace's own codecs could not be closed.
    ///
    /// The refusal is a refusal and not a re-split, because `'_'` opens a
    /// `BLANK_NODE_LABEL` and no other terminal (see [`Self::lex_one`]): there is
    /// no shorter token for the scanner to fall back to. The lawful neighbours
    /// that must keep parsing are `_:0a` and `_:_a` (digit and underscore are the
    /// head), `_:a-b` and `_:a.b` (hyphen and internal dot in the tail), and
    /// `_:café` in either normalization (a combining mark is a lawful tail).
    fn lex_blank_label(&mut self, start: usize) -> Result<Token<'a>> {
        self.pos += 2; // `_:`
        let begin = self.pos;
        match self.cur() {
            Some(c) if terminals::is_blank_node_label_start(c) => self.pos += c.len_utf8(),
            _ => {
                return Err(ParseError::lex(
                    "a blank node label must begin with PN_CHARS_U or [0-9]: \
                     BLANK_NODE_LABEL ::= '_:' ( PN_CHARS_U | [0-9] ) \
                     ((PN_CHARS | '.')* PN_CHARS)?",
                    start,
                ));
            }
        }
        self.take_while(|c| terminals::is_pn_chars(c) || c == '.');
        let raw = &self.src[begin..self.pos];
        let label = raw.trim_end_matches('.');
        // Push `pos` back over the over-consumed trailing dots: a trailing `.` is
        // the statement terminator, not part of the label. `.` is ASCII (1 byte),
        // so the trimmed byte-length delta equals the dot run. The head is never
        // a `'.'`, so this can never trim the label away to nothing.
        self.pos -= raw.len() - label.len();
        Ok(Token::BlankNodeLabel(label))
    }

    /// `ANON ::= '[' WS* ']'` — one token — or else the bare `'['` that opens a
    /// blank-node property list.
    ///
    /// The inter-bracket run is scanned with [`terminals::is_ws`] over raw bytes,
    /// the same exact-over-UTF-8 byte test [`Self::skip_trivia`] uses and for the
    /// same two reasons. Here the exactness is load-bearing in a second way: a
    /// scalar that `char::is_whitespace` admits but `WS` does not — U+00A0 the
    /// leading example — must not *close* the `ANON`, because it is not a member
    /// of any production that may appear between the brackets, so `[<NBSP>]` is
    /// simply not a SPARQL token and must be refused rather than silently read as
    /// a fresh blank node.
    ///
    /// This scan is deliberately **comment-blind**, and must stay so. `ANON` is a
    /// *terminal*, so the grammar admits no comment inside it; and the lexer could
    /// not skip one soundly even if it wanted to, because `[ # c\n ?p ?o ]` is a
    /// lawful *populated* `BlankNodePropertyListPath` that must emit `LBracket`.
    /// Nothing at this level can tell the empty case from the populated one, so
    /// teaching this scan about `#` would only *widen* acceptance. The empty pair
    /// `'[' ']'` reachable only via a comment is refused by the parser, which does
    /// know which case it is in — see `parse_blank_node_property_list` in
    /// [`crate::parser`].
    fn lex_bracket_or_anon(&mut self) -> Result<Token<'a>> {
        let mut j = self.pos + 1; // byte offset past '['
        while self.bytes.get(j).copied().is_some_and(terminals::is_ws) {
            j += 1; // every `WS` member is one ASCII byte
        }
        if self.bytes.get(j) == Some(&b']') {
            self.pos = j + 1; // ']' is ASCII
            Ok(Token::Anon)
        } else {
            self.pos += 1;
            Ok(Token::LBracket)
        }
    }

    fn lex_lang_tag(&mut self, start: usize) -> Result<Token<'a>> {
        self.pos += 1; // `@`
        let tag = self.take_while(|c| c.is_ascii_alphanumeric() || c == '-');
        if tag.is_empty() {
            return Err(ParseError::lex("empty language tag", start));
        }
        Ok(Token::LangTag(tag))
    }

    /// A bare `:local` or `:` prefixed name (empty prefix).
    fn lex_prefixed_name(&mut self, _start: usize) -> Result<Token<'a>> {
        self.pos += 1; // `:`
        let local = self.take_local();
        Ok(Token::PrefixedName("", local))
    }

    /// A word that may be a keyword (`SELECT`, `a`, `true`) or the prefix part of
    /// a prefixed name (`purrdf:` / `rdf:type`).
    fn lex_word_or_prefixed(&mut self, _start: usize) -> Result<Token<'a>> {
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
    /// The cursor is on an ASCII `e`/`E`; `pos + 2` is only read after `pos + 1`
    /// matched an ASCII sign, so both lookaheads are byte compares (see `byte_at`).
    fn exp_has_digits(&self) -> bool {
        match self.byte_at(1) {
            Some(b'+' | b'-') => matches!(self.byte_at(2), Some(b'0'..=b'9')),
            Some(b'0'..=b'9') => true,
            _ => false,
        }
    }

    fn lex_number(&mut self) -> Token<'a> {
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
        let lexical = &self.src[begin..self.pos];
        if seen_exp {
            Token::Double(lexical)
        } else if seen_dot {
            Token::Decimal(lexical)
        } else {
            Token::Integer(lexical)
        }
    }

    fn take_while(&mut self, pred: impl Fn(char) -> bool) -> &'a str {
        let begin = self.pos;
        while let Some(c) = self.cur() {
            if pred(c) {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        &self.src[begin..self.pos]
    }

    /// `PN_PREFIX`: starts with a base char, may contain `.`/`-`/digits, must not
    /// end with `.`.
    ///
    /// The continue class is [`terminals::is_pn_chars`], which is `PN_CHARS_U`
    /// plus `'-'`, `[0-9]`, U+00B7, `[#x300-#x36F]` and `[#x203F-#x2040]` — the
    /// combining marks matter, because an NFD-decomposed name (`e` + U+0301) is
    /// exactly as lawful as its NFC spelling and must scan as one token.
    fn take_pn_prefix(&mut self) -> &'a str {
        let raw = self.take_while(|c| terminals::is_pn_chars(c) || c == '.');
        let trimmed = raw.trim_end_matches('.');
        // Push `pos` (a byte offset) back over the over-consumed trailing dots. `.`
        // is ASCII (1 byte), so the trimmed byte-length delta equals that dot run.
        self.pos -= raw.len() - trimmed.len();
        trimmed
    }

    /// Whether the scalar at the cursor opens a `PN_LOCAL`.
    ///
    /// `PN_LOCAL`'s head is `(PN_CHARS_U | ':' | [0-9] | PLX)`. The first three
    /// alternatives are a character class and are answered by
    /// [`terminals::is_pn_local_start`]; `PLX ::= PERCENT | PN_LOCAL_ESC` is not
    /// one — `PERCENT ::= '%' HEX HEX` and `PN_LOCAL_ESC ::= '\' [_~.-!$&…]` are
    /// decisions about the scalars that FOLLOW — so those two are decided here,
    /// with the cursor, exactly as the body of the scan decides them.
    ///
    /// The `'/'` arm is the [`LexerOptions::pn_local_allows_slash`] dialect and
    /// not the W3C production. That option's contract is that a bare `/` is *a
    /// `PN_LOCAL` character*, so it is one in both positions, and
    /// `ex:/a` keeps the single-token reading it has always had under Turtle
    /// while SPARQL keeps reading the `/` as the property-path operator.
    fn at_pn_local_start(&self) -> bool {
        match self.cur() {
            // `PN_LOCAL_ESC` — a `\` that escapes nothing belongs to no name.
            Some('\\') => self.peek(1).is_some_and(terminals::is_pn_local_esc),
            Some('%') => self.at_percent(),
            Some('/') => self.options.pn_local_allows_slash,
            Some(c) => terminals::is_pn_local_start(c),
            None => false,
        }
    }

    /// Whether the cursor sits on a whole `PERCENT ::= '%' HEX HEX`.
    ///
    /// `HEX ::= [0-9] | [A-F] | [a-f]`, which is exactly
    /// [`char::is_ascii_hexdigit`] — an enumerated production answered by an
    /// exact predicate rather than a convenient property, so nothing here
    /// approximates. Case is not significant: `%2F`, `%2f` and `%aB` are all
    /// lawful `PERCENT`s.
    ///
    /// **Both digits are required, and that is a refusal this scanner did not
    /// use to make.** A bare `'%'` was admitted on its lead scalar alone, at the
    /// head of a local name and in its tail, so `ex:%zz` lexed here as one
    /// prefixed name while the workspace's ShExC scanner — reading the same
    /// production — read it as `ex:` followed by junk. Two surfaces of one
    /// product gave opposite answers to one grammar, and the grammar is not
    /// ambiguous.
    ///
    /// Like every other `PLX` decision this refusal moves a token BOUNDARY
    /// rather than rejecting a document outright: the local name simply stops
    /// before the `'%'`, and what the `'%'` then means is the caller's question.
    /// `ex:%` truncated at end of input is the reason both lookaheads are
    /// `Option`-shaped — [`Lexer::peek`] returns `None` past the end, so a
    /// truncated escape ends the name instead of reading past the buffer.
    fn at_percent(&self) -> bool {
        self.peek(1).is_some_and(|h| h.is_ascii_hexdigit())
            && self.peek(2).is_some_and(|h| h.is_ascii_hexdigit())
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
    ///
    /// # The head is its own class, and the EMPTY local name is lawful
    ///
    /// ```text
    /// PN_LOCAL  ::= (PN_CHARS_U | ':' | [0-9] | PLX) ((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?
    /// PLX       ::= PERCENT | PN_LOCAL_ESC
    /// PNAME_LN  ::= PNAME_NS PN_LOCAL
    /// PNAME_NS  ::= PN_PREFIX? ':'
    /// ```
    ///
    /// Two things follow, and they pull in opposite directions.
    ///
    /// First, the head is NARROWER than the tail: `'-'`, `'.'`, U+00B7, the
    /// combining marks and the two ties are all `PN_CHARS`, so they continue a
    /// local name and none of them starts one. `ex:a-b` is one prefixed name and
    /// `ex:-b` is not; scanning the head with the tail class read both the same
    /// way. [`terminals::is_pn_local_start`] answers the three single-scalar
    /// alternatives; `PLX` is a shape rather than a class and is decided here —
    /// a `'%'` opens `PERCENT` when two `HEX` digits follow it and a `'\'` opens
    /// `PN_LOCAL_ESC` when what follows it is escapable — so `ex:%20a` and
    /// `ex:\~a` keep their leading `PLX`, and `ex:%zz` does not have one.
    ///
    /// Second, a head check may not become a *requirement* that a head exist.
    /// `PNAME_NS` is a whole terminal — `ex:` and `:` are complete prefixed names
    /// with an EMPTY local part, and `:a` has an empty PREFIX — so when the
    /// scalar at the cursor cannot open a local name this returns the empty
    /// string WITHOUT consuming it, and the scanner reads the next token from
    /// there. `ex:.` is therefore the prefixed name `ex:` followed by `Dot`,
    /// exactly as it already was, and `ex:-a` becomes `ex:`, `Minus`, `a`. The
    /// tightening moves a token BOUNDARY; it refuses no document that has a
    /// reading.
    fn take_local(&mut self) -> Cow<'a, str> {
        // Fast path: scan the local name assuming no `PN_LOCAL_ESC`. When no `\`
        // escape is present the local part is a contiguous source slice (borrowed);
        // hitting a valid escape rewinds and defers to the owned builder below.
        let begin = self.pos;
        if !self.at_pn_local_start() {
            return Cow::Borrowed(&self.src[begin..begin]);
        }
        let mut trailing_dots = 0usize;
        while let Some(c) = self.cur() {
            if c == '\\' {
                if self.peek(1).is_some_and(terminals::is_pn_local_esc) {
                    // An escape rewrites bytes — restart with the owned builder.
                    self.pos = begin;
                    return Cow::Owned(self.take_local_owned());
                }
                break; // a non-PN_LOCAL_ESC backslash does not belong to the local name
            }
            if c == '.' {
                trailing_dots += 1;
                self.pos += 1;
                continue;
            }
            if c == '/' && self.options.pn_local_allows_slash {
                trailing_dots = 0;
                self.pos += 1;
                continue;
            }
            if c == '%' {
                // `PLX` → `PERCENT ::= '%' HEX HEX`: all three scalars or none.
                if !self.at_percent() {
                    break;
                }
                trailing_dots = 0;
                // `'%'` and both `HEX` digits are ASCII, so the escape is
                // exactly three bytes wide.
                self.pos += 3;
                continue;
            }
            if terminals::is_pn_chars(c) || c == ':' {
                trailing_dots = 0;
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if trailing_dots > 0 {
            // Push back the trailing-dot run: it is the statement terminator.
            self.pos -= trailing_dots;
        }
        Cow::Borrowed(&self.src[begin..self.pos])
    }

    /// The `take_local` owned path: identical scan but decoding `PN_LOCAL_ESC`
    /// (`\X`) into the returned local name. Only reached when the fast path saw a
    /// `\` escape, so building a fresh `String` here is the rare case.
    fn take_local_owned(&mut self) -> String {
        let mut out = String::new();
        let mut trailing_dots = 0usize;
        while let Some(c) = self.cur() {
            if c == '\\' {
                // PN_LOCAL_ESC: consume the backslash and emit the next char verbatim.
                if let Some(escaped) = self.peek(1).filter(|e| terminals::is_pn_local_esc(*e)) {
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
            if c == '%' {
                // `PLX` → `PERCENT ::= '%' HEX HEX`, kept VERBATIM (unlike a
                // `PN_LOCAL_ESC`, the escape is part of the IRI's value).
                if !self.at_percent() {
                    break;
                }
                out.push(c);
                out.extend(self.peek(1));
                out.extend(self.peek(2));
                trailing_dots = 0;
                self.pos += 3;
                continue;
            }
            if terminals::is_pn_chars(c) || c == ':' {
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

// ── Naming the scalar a reader cannot see ────────────────────────────────────
//
// The scanner's character classes are the grammar's, so `?s<NBSP>?p` is refused
// rather than silently fused into one variable. That refusal is only half the
// deliverable. The offending scalar is INVISIBLE — that is why the defect
// survived in the first place — so an error that renders it as its own glyph
// tells the author nothing: the text on screen reads `?s ?p` and looks correct.
// A reader handed `unexpected character '\u{a0}'` has no path from the message
// to the fix.
//
// So the offending scalar is named, and so is the production that refuses it.
// Membership below is a property of the CHARACTER; whether a character is an
// offender at all is a property of the POSITION, which is why these tables are
// consulted at exactly one place — [`Lexer::lex_one`]'s final arm, reached only
// once every production that could have claimed the scalar has declined it.
// Nothing here can fire inside an `IRIREF` body (U+00A0 and its neighbours are
// LAWFUL there), inside a string literal, or inside a name.

/// The `WS` production spelled the way the grammar spells it, so every message
/// quotes the same four members rather than paraphrasing them.
const WS_MEMBERS: &str = "#x20 | #x9 | #xD | #xA";

/// Every scalar carrying the Unicode `White_Space` property that
/// `WS ::= #x20 | #x9 | #xD | #xA` does NOT name, paired with its Unicode name.
///
/// This is the whole property minus the four `WS` members — 21 entries — and it
/// is pinned as exactly that by
/// `the_unicode_space_table_is_the_whole_property_minus_ws`, which derives the
/// set from [`char::is_whitespace`] and compares. A hand-picked list of "the
/// Unicode spaces" loses members silently, and the ones it loses are the ones
/// nobody pictures: an earlier draft of this table omitted U+205F MEDIUM
/// MATHEMATICAL SPACE and U+2029 PARAGRAPH SEPARATOR. The literal lives here
/// because the names do — spelling them is the point — but the MEMBERSHIP is
/// checked against the property, so a gap is a test failure and not a hole.
///
/// U+1680 OGHAM SPACE MARK is in the table and can never be reported from it:
/// it carries `White_Space` AND lies inside `PN_CHARS_BASE`'s `[#x37F-#x1FFF]`,
/// so the grammar makes it a NAME character and [`Lexer::lex_one`]'s
/// `is_pn_chars_base` arm claims it before the diagnostic arm is reached. It is
/// listed anyway so this table is the property and not a subset of it; the
/// unreachability is pinned by
/// `the_ogham_space_mark_is_never_reported_as_a_separator`.
const NON_WS_UNICODE_SPACES: &[(char, &str)] = &[
    ('\u{b}', "LINE TABULATION"),
    ('\u{c}', "FORM FEED"),
    ('\u{85}', "NEXT LINE"),
    ('\u{a0}', "NO-BREAK SPACE"),
    ('\u{1680}', "OGHAM SPACE MARK"),
    ('\u{2000}', "EN QUAD"),
    ('\u{2001}', "EM QUAD"),
    ('\u{2002}', "EN SPACE"),
    ('\u{2003}', "EM SPACE"),
    ('\u{2004}', "THREE-PER-EM SPACE"),
    ('\u{2005}', "FOUR-PER-EM SPACE"),
    ('\u{2006}', "SIX-PER-EM SPACE"),
    ('\u{2007}', "FIGURE SPACE"),
    ('\u{2008}', "PUNCTUATION SPACE"),
    ('\u{2009}', "THIN SPACE"),
    ('\u{200a}', "HAIR SPACE"),
    ('\u{2028}', "LINE SEPARATOR"),
    ('\u{2029}', "PARAGRAPH SEPARATOR"),
    ('\u{202f}', "NARROW NO-BREAK SPACE"),
    ('\u{205f}', "MEDIUM MATHEMATICAL SPACE"),
    ('\u{3000}', "IDEOGRAPHIC SPACE"),
];

/// The invisible scalars that do NOT carry `White_Space`, paired with their
/// Unicode names.
///
/// These would otherwise get the blankest message of all: they carry no
/// property a reader would think to ask about, so nothing in the trivia-side
/// story reaches them, and they are not name characters either — U+200B ZERO
/// WIDTH SPACE sits in the hole immediately below `PN_CHARS_BASE`'s
/// `[#x200C-#x200D]`. A user who pastes one from a word processor, a web page
/// or a bidi-marked document gets a query that looks right, lexes wrong, and
/// (before the name classes were made exact) parsed to the wrong answer.
///
/// **U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH JOINER are deliberately
/// NOT here**, and no future sweep of "the invisible format characters" may add
/// them: `PN_CHARS_BASE` admits `[#x200C-#x200D]` outright, so both are LAWFUL
/// name characters — `ex:a\u{200c}b` is one prefixed name and `?a\u{200d}b` is
/// one variable. Listing them would hand a user a message telling them their
/// valid query is wrong. Their absence is pinned by
/// `the_two_joiners_are_lawful_names_and_are_not_offenders`.
///
/// U+FEFF ZERO WIDTH NO-BREAK SPACE is listed but, like U+1680, is unreachable
/// from here: `PN_CHARS_BASE` runs to `[#xFDF0-#xFFFD]`, which contains it, so a
/// byte-order mark at the head of a query is a lawful NAME character and the
/// `is_pn_chars_base` arm claims it first (`\u{feff}SELECT` lexes as one word,
/// and the *parser* is what refuses it). It is named here so a reader auditing
/// the invisible scalars finds it with its verdict attached rather than assuming
/// the table forgot it; `the_byte_order_mark_is_a_name_character_too` pins the
/// verdict.
const INVISIBLE_NON_SPACES: &[(char, &str)] = &[
    ('\u{ad}', "SOFT HYPHEN"),
    ('\u{200b}', "ZERO WIDTH SPACE"),
    ('\u{200e}', "LEFT-TO-RIGHT MARK"),
    ('\u{200f}', "RIGHT-TO-LEFT MARK"),
    ('\u{2060}', "WORD JOINER"),
    ('\u{feff}', "ZERO WIDTH NO-BREAK SPACE"),
];

/// The Unicode name `table` gives `c`, or `None` when `c` is not listed.
///
/// A linear scan over a 21- or 6-entry table, on an error path that has already
/// decided the parse is over. Nothing here indexes, unwraps or slices.
fn lookup_scalar_name(table: &[(char, &'static str)], c: char) -> Option<&'static str> {
    table
        .iter()
        .find_map(|&(scalar, name)| (scalar == c).then_some(name))
}

/// The message for a scalar a reader cannot see: its Unicode name, the
/// production that refuses it, and what to do instead. `None` for a scalar that
/// renders as its own visible glyph, which [`unexpected_character`] then reports
/// the ordinary way.
///
/// Total by construction — a lookup miss returns `None` and a hit formats a
/// `&'static str` — so it cannot panic on input that is already an error.
fn invisible_scalar_diagnostic(c: char) -> Option<String> {
    if let Some(name) = lookup_scalar_name(NON_WS_UNICODE_SPACES, c) {
        return Some(format!(
            "U+{:04X} {name} cannot separate tokens: WS is {WS_MEMBERS}. \
             Replace it with a space.",
            u32::from(c)
        ));
    }
    let name = lookup_scalar_name(INVISIBLE_NON_SPACES, c)?;
    Some(format!(
        "U+{:04X} {name} is invisible and starts no token: PN_CHARS does not name it, \
         and WS is {WS_MEMBERS}. Delete it.",
        u32::from(c)
    ))
}

/// The reason text for a scalar that begins no SPARQL token.
///
/// An invisible offender is named and told which production refuses it; a
/// visible one keeps the ordinary `unexpected character '%'` form, which already
/// shows the reader exactly what they typed.
fn unexpected_character(c: char) -> String {
    invisible_scalar_diagnostic(c).unwrap_or_else(|| format!("unexpected character {c:?}"))
}

/// Whether `name` is a complete `VARNAME`, sigil already stripped.
///
/// `VARNAME ::= ( PN_CHARS_U | [0-9] ) ( PN_CHARS_U | [0-9] | #xB7 |`
/// `[#x300-#x36F] | [#x203F-#x2040] )*` is **position-dependent**, so membership
/// cannot be answered by one character class applied to every scalar: a
/// combining mark, a MIDDLE DOT or an UNDERTIE may continue a variable name but
/// may not begin one. The two halves are [`terminals::is_varname_start`] and
/// [`terminals::is_varname_continue`]; this is the whole-string form the
/// algebra validator needs, and the empty name is not a `VARNAME`.
///
/// Note that neither half admits `'-'`, which `PN_CHARS` does — `?a-b` is three
/// tokens, not one variable.
pub(crate) fn is_varname(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    terminals::is_varname_start(first) && chars.all(terminals::is_varname_continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn toks(s: &str) -> Vec<Token<'_>> {
        tokenize(s).unwrap().into_iter().map(|s| s.token).collect()
    }

    fn toks_turtle(s: &str) -> Vec<Token<'_>> {
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

    // ── IRIREF body membership: `[^#x00-#x20<>"{}|^`\]`, and nothing else ──────────
    //
    // The defect these pin: the body scan terminated at any `char::is_whitespace`, which
    // is a strictly LARGER set than the production excludes. U+00A0 and the other
    // non-ASCII Unicode whitespace are lawful `IRIREF` characters — and RFC-3987
    // `ucschar`, so they are lawful IRIs too — but the lexer refused them. The
    // workspace's own writers emit those code points raw, so an IRI carrying a U+00A0
    // NO-BREAK SPACE serialized fine and then failed to re-parse: the `<` fell back to the
    // comparison operator and the parser reported an unexpected `Lt`. (The code points are
    // written here as escapes on purpose — spelling them literally would put an invisible
    // character in a comment about invisible characters.)
    //
    // This is an over-refusal fix, so the neighbouring refusals are executed alongside
    // it: ASCII SPACE and the C0 controls are `#x00-#x20` and must STILL terminate the
    // body, as must each of the nine reserved delimiters.

    /// Non-ASCII Unicode whitespace is lawful raw inside an `IRIREF` (fast path — the
    /// body carries no escape, so it is borrowed verbatim).
    #[test]
    fn non_ascii_whitespace_is_lawful_raw_inside_an_iriref() {
        for c in ['\u{a0}', '\u{2000}', '\u{2028}', '\u{3000}'] {
            let src = format!("<urn:ex:a{c}b>");
            assert_eq!(
                toks(&src),
                vec![Token::Iri(format!("urn:ex:a{c}b").into())],
                "U+{:04X} is above #x20 and is not a reserved delimiter",
                c as u32
            );
        }
    }

    /// The same holds on the escape-decoding slow path: one `UCHAR` in the body must not
    /// change the verdict on a raw `ucschar` elsewhere in it.
    #[test]
    fn non_ascii_whitespace_survives_the_uchar_decoding_path() {
        assert_eq!(
            toks("<urn:ex:a\u{a0}b\\u221E>"),
            vec![Token::Iri("urn:ex:a\u{a0}b\u{221e}".into())]
        );
    }

    /// Whether the LEADING `<` of `src` failed to open an `IRIREF` — the stream either
    /// refused outright or backed that `<` out to the comparison operator. Both are "this
    /// is not an `IRIREF`"; which one occurs depends on how the tail happens to lex. Only
    /// the first token is inspected, because backing out re-lexes the tail, where a later
    /// `<` may legitimately open an IRIREF of its own (`<urn:ex:a<b>` ends in `<b>`).
    fn no_leading_iriref(src: &str) -> bool {
        match tokenize(src) {
            Ok(spanned) => !matches!(spanned.first().map(|s| &s.token), Some(Token::Iri(_))),
            Err(_) => true,
        }
    }

    /// The valid neighbour's mirror: `#x00-#x20` — ASCII SPACE and the C0 controls —
    /// still ends the body, so an unescaped space inside `<...>` is still not an IRIREF.
    #[test]
    fn ascii_space_and_c0_controls_still_end_an_iriref_body() {
        // The headline case lexes cleanly all the way through, so it is pinned exactly:
        // `<` backs out to the comparison operator.
        assert_eq!(
            toks("<urn:ex:a b>").first().cloned(),
            Some(Token::Lt),
            "an unescaped SPACE inside <...> is not an IRIREF"
        );
        for c in [' ', '\t', '\n', '\r', '\u{0}', '\u{1f}'] {
            let src = format!("<urn:ex:a{c}b>");
            assert!(
                no_leading_iriref(&src),
                "U+{:04X} is inside #x00-#x20 and must not be admitted raw",
                c as u32
            );
        }
    }

    /// Each reserved delimiter still ends the body, on both the verbatim and the
    /// escape-decoding path.
    #[test]
    fn reserved_delimiters_still_end_an_iriref_body() {
        for c in ['<', '"', '{', '}', '|', '^', '`', '\\'] {
            let src = format!("<urn:ex:a{c}b>");
            assert!(
                no_leading_iriref(&src),
                "{c:?} is a reserved IRIREF delimiter"
            );
            let escaped = format!("<urn:ex:\\u0041a{c}b>");
            assert!(
                no_leading_iriref(&escaped),
                "{c:?} is a reserved IRIREF delimiter on the UCHAR path too"
            );
        }
    }

    // ── `WS ::= #x20 | #x9 | #xD | #xA`, and nothing else, at BOTH trivia sites ──
    //
    // The defect these pin: `skip_trivia` and the `ANON` scan both tested
    // `char::is_whitespace`, i.e. the Unicode `White_Space` property, which names
    // roughly two dozen scalars the grammar does not. That is not a harmless
    // liberality, because a scanner's character class is a BOUNDARY test: none of
    // those extra scalars is a `PN_CHARS` member either, so skipping one lets it
    // silently separate two tokens the grammar would have refused to separate,
    // and the document parses — differently — instead of failing.
    //
    // The two sites must move together. Tightening the `ANON` scan alone flips
    // `{ [<NBSP>] }` from refused to ACCEPTED: the NO-BREAK SPACE stops closing
    // the `ANON`, an `LBracket` is emitted instead, a still-liberal `skip_trivia`
    // then eats the NO-BREAK SPACE, the `]` arrives, and the parser reads an empty
    // blank-node property list. So the vector below exercises the pair.
    //
    // The refusal set is GENERATED from the Unicode property rather than written
    // out by hand, because a hand-written list of "the Unicode spaces" silently
    // loses members: U+205F MEDIUM MATHEMATICAL SPACE and U+2029 PARAGRAPH
    // SEPARATOR are the two that go first.

    /// Every scalar carrying the Unicode `White_Space` property that `WS` does
    /// NOT name — the exact set both trivia sites must refuse to skip.
    ///
    /// `char::is_whitespace` *is* the `White_Space` property, so deriving the set
    /// from it makes the vector self-maintaining: a future Unicode revision that
    /// adds a space character adds it here too, rather than leaving a stale
    /// literal list that silently stops covering it.
    fn non_ws_unicode_whitespace() -> Vec<char> {
        (0..=0x0010_FFFF_u32)
            .filter_map(char::from_u32)
            .filter(|&c| c.is_whitespace() && !matches!(c, ' ' | '\t' | '\r' | '\n'))
            .collect()
    }

    /// The generated set really does hold the members a hand-list drops, and
    /// really does exclude the four `WS` members — otherwise every vector built on
    /// it would pass vacuously.
    #[test]
    fn the_generated_refusal_set_covers_what_a_hand_list_loses() {
        let set = non_ws_unicode_whitespace();
        for c in [
            '\u{b}', '\u{c}', '\u{85}', '\u{a0}', '\u{1680}', '\u{2000}', '\u{200a}', '\u{2028}',
            '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        ] {
            assert!(
                set.contains(&c),
                "U+{:04X} carries White_Space and is not `WS`",
                c as u32
            );
        }
        for c in [' ', '\t', '\r', '\n'] {
            assert!(
                !set.contains(&c),
                "U+{:04X} is one of the four `WS` members",
                c as u32
            );
        }
        // A floor, not an equality: the point is that the set is the property's,
        // so a Unicode revision may grow it without invalidating the vector.
        assert!(
            set.len() >= 21,
            "White_Space minus `WS` has {} members",
            set.len()
        );
    }

    /// Between two tokens, exactly the four `WS` members are trivia. Each refusal
    /// is executed next to its ASCII neighbour in the same vector, so this pins
    /// exactness in both directions rather than blanket non-ASCII refusal.
    ///
    /// The asserted property is "the scalar was not DROPPED", not a particular
    /// error arm. Which arm reports it depends on the scalar: most are no token's
    /// start and reach the lex error, while U+1680 OGHAM SPACE MARK is a lawful
    /// `PN_CHARS_BASE` member and surfaces as an extra `Word` token. Either
    /// outcome is a refusal to treat it as trivia; which one occurs is not this
    /// vector's claim to make.
    #[test]
    fn only_ws_is_skipped_between_two_tokens() {
        for c in [' ', '\t', '\r', '\n'] {
            let src = format!("{{{c}}}");
            assert_eq!(
                toks(&src),
                vec![Token::LBrace, Token::RBrace],
                "U+{:04X} is `WS` and must still be skipped",
                c as u32
            );
        }
        for c in non_ws_unicode_whitespace() {
            let src = format!("{{{c}}}");
            let skipped = tokenize(&src).is_ok_and(|spanned| {
                spanned
                    .iter()
                    .map(|s| &s.token)
                    .eq([Token::LBrace, Token::RBrace].iter())
            });
            assert!(
                !skipped,
                "U+{:04X} is not `WS` and must not be skipped as trivia",
                c as u32
            );
        }
    }

    /// `u8::is_ascii_whitespace` is a two-sided trap, so both of its sides are
    /// executed: it ADMITS U+000C, which `WS` does not name, and EXCLUDES U+000B,
    /// which Unicode does. Both are ASCII, so both reach the lexer's error arm
    /// and the refusal can be pinned exactly.
    #[test]
    fn form_feed_and_vertical_tab_are_not_trivia() {
        assert!(0x0c_u8.is_ascii_whitespace());
        assert!(tokenize("{\u{c}}").is_err(), "U+000C FORM FEED is not `WS`");
        assert!(!0x0b_u8.is_ascii_whitespace());
        assert!('\u{b}'.is_whitespace());
        assert!(
            tokenize("{\u{b}}").is_err(),
            "U+000B VERTICAL TAB is not `WS`"
        );
        // The neighbours: the two ASCII controls `WS` DOES name still separate.
        assert_eq!(toks("{\t}"), vec![Token::LBrace, Token::RBrace]);
        assert_eq!(toks("{\r\n}"), vec![Token::LBrace, Token::RBrace]);
    }

    /// `ANON ::= '[' WS* ']'` is closed by the four `WS` members and by nothing
    /// else, and the sibling `NIL ::= '(' WS* ')'` bracket pair is untouched.
    #[test]
    fn anon_admits_exactly_ws_between_its_brackets() {
        assert_eq!(toks("[]"), vec![Token::Anon]);
        assert_eq!(toks("[ \t\r\n]"), vec![Token::Anon]);
        assert_eq!(toks_turtle("[ \t\r\n]"), vec![Token::Anon]);
        for c in non_ws_unicode_whitespace() {
            let src = format!("[{c}]");
            let closes_anon = tokenize(&src).is_ok_and(
                |spanned| matches!(spanned.as_slice(), [one] if one.token == Token::Anon),
            );
            assert!(
                !closes_anon,
                "U+{:04X} is not `WS` and must not close an ANON",
                c as u32
            );
        }
        // `( WS* )` still lexes to the pair the parser folds into `rdf:nil`.
        assert_eq!(toks("( )"), vec![Token::LParen, Token::RParen]);
        assert_eq!(toks("(\t\r\n)"), vec![Token::LParen, Token::RParen]);
    }

    /// The same scalar is refused as trivia and admitted inside an `IRIREF` — the
    /// two productions disagree about U+00A0 and both are now transcribed
    /// exactly, so tightening `WS` must not have re-broken the `IRIREF` body.
    #[test]
    fn no_break_space_is_refused_as_trivia_yet_lawful_inside_an_iriref() {
        assert!(
            !tokenize("{\u{a0}}").is_ok_and(|spanned| spanned.len() == 2),
            "U+00A0 is not `WS`"
        );
        assert_eq!(
            toks("<urn:ex:a\u{a0}b>"),
            vec![Token::Iri("urn:ex:a\u{a0}b".into())],
            "U+00A0 is above #x20 and is not a reserved IRIREF delimiter"
        );
    }

    // ── `PN_CHARS` and `VARNAME`: a name class is a boundary, not a set ──────
    //
    // The defect these pin: the lexer approximated `PN_CHARS_BASE` as "an ASCII
    // letter, `_`, or ANY scalar above 0x7F", and `VARNAME` as "ASCII
    // alphanumeric, `_`, or any scalar above 0x7F". Tightening `WS` alone does
    // not fix that — it makes it WORSE-hidden — because the greedy name scan runs
    // first and never consults the trivia skip: in `?s<NBSP>?p` the NO-BREAK
    // SPACE was absorbed into the variable name, so the input lexed cleanly as
    // the two variables `s\u{a0}` and `p` instead of being refused.
    //
    // End to end that is a WRONG ANSWER, not an error. In
    // `SELECT ?s WHERE { ?s<NBSP><urn:ex:p> ?o . ?s <urn:ex:q> ?z }` the author
    // wrote three variables; the liberal class produced FOUR, `s\u{a0}` and `s`
    // being distinct, so the join on `?s` silently became a cross product. The
    // whole-query proof lives in `tests/name_class_boundaries.rs`; what follows
    // is the token-level boundary each of those answers rests on.
    //
    // The asserted property is "not one token spanning the intruder" — either a
    // refusal or a different split — and never a particular error arm: where the
    // intruder lands decides which, and that is not this vector's claim to make.

    /// A scalar that is neither `WS` nor a name character can only be refused: it
    /// may not extend a variable, a prefixed name or a blank-node label.
    ///
    /// Blank-node labels are in here for the same reason variables are, not as a
    /// tidiness matter: a label IS data identity, so fusing `_:a<NBSP>b` into one
    /// label merges two distinct nodes exactly as the variable case merges two
    /// distinct bindings.
    #[test]
    fn non_ascii_whitespace_cannot_extend_a_name() {
        for src in [
            "?s\u{a0}?p",
            "$s\u{a0}$p",
            "ex:a\u{a0}ex:b",
            "_:a\u{a0}b",
            "ex\u{a0}:b",
        ] {
            assert!(
                tokenize(src).is_err(),
                "U+00A0 is neither `WS` nor `PN_CHARS`, so {src:?} is not SPARQL"
            );
        }
        // The neighbours, spelled with the SPACE their author meant, still lex —
        // this is exactness, not a blanket refusal of what follows a name.
        assert_eq!(toks("?s ?p"), vec![var("s"), var("p")]);
        assert_eq!(toks("$s $p"), vec![var("s"), var("p")]);
        assert_eq!(
            toks("ex:a ex:b"),
            vec![
                Token::PrefixedName("ex", "a".into()),
                Token::PrefixedName("ex", "b".into()),
            ]
        );
        assert_eq!(
            toks("_:a _:b"),
            vec![Token::BlankNodeLabel("a"), Token::BlankNodeLabel("b")]
        );
    }

    /// U+200B ZERO WIDTH SPACE is the case a `char::is_whitespace` audit misses
    /// entirely: it does NOT carry `White_Space`, so no trivia-side tightening
    /// ever looked at it — and it sits in the hole between `PN_CHARS_BASE`'s
    /// `[#x200C-#x200D]` and everything below, so it is not a name character
    /// either. The liberal class swallowed it into names invisibly.
    #[test]
    fn zero_width_space_is_not_a_name_character() {
        assert!(!'\u{200b}'.is_whitespace());
        for src in ["?s\u{200b}?p", "ex:a\u{200b}b", "_:a\u{200b}b"] {
            assert!(
                tokenize(src).is_err(),
                "U+200B is in the hole below [#x200C-#x200D] and names no token"
            );
        }
        // Its two admitted neighbours, ZWNJ and ZWJ, ARE `PN_CHARS_BASE` — the
        // production names `[#x200C-#x200D]` — so the hole is a hole and not a
        // truncated table.
        for c in ['\u{200c}', '\u{200d}'] {
            let name = format!("a{c}b");
            assert_eq!(toks(&format!("?{name}")), vec![var(&name)]);
        }
    }

    /// U+1680 OGHAM SPACE MARK carries the Unicode `White_Space` property AND
    /// lies inside `PN_CHARS_BASE`'s `[#x37F-#x1FFF]`, so the grammar makes it a
    /// NAME character. It therefore does NOT separate two variables: `?s<U+1680>?p`
    /// is the variable `s\u{1680}` followed by `?p`, and the intruder belongs to
    /// the first NAME rather than ending it.
    ///
    /// Pinned positively and on purpose. It reads like the U+00A0 bug and is not
    /// one; a future reader "fixing" it by excluding Unicode spaces from the name
    /// classes would manufacture an over-refusal of a lawful SPARQL name.
    #[test]
    fn ogham_space_mark_is_a_name_character_because_the_production_says_so() {
        assert!('\u{1680}'.is_whitespace());
        assert_eq!(
            toks("?s\u{1680}?p"),
            vec![var("s\u{1680}"), var("p")],
            "U+1680 is inside PN_CHARS_BASE's [#x37F-#x1FFF]"
        );
        assert_eq!(toks("?a\u{1680}"), vec![var("a\u{1680}")]);
        assert_eq!(
            toks("ex:a\u{1680}b"),
            vec![Token::PrefixedName("ex", "a\u{1680}b".into())]
        );
    }

    /// `PN_CHARS` is NOT `PN_CHARS_BASE` plus digits and `-`: it also names
    /// `#xB7`, `[#x300-#x36F]` and `[#x203F-#x2040]`. Dropping those refuses
    /// `ex:col·lecció` and, far worse, EVERY NFD-decomposed name — so the same
    /// word would parse spelled in NFC and fail spelled in NFD.
    #[test]
    fn pn_chars_keeps_the_middle_dot_the_combining_marks_and_the_ties() {
        assert_eq!(
            toks("ex:col\u{b7}lecci\u{f3}"),
            vec![Token::PrefixedName("ex", "col\u{b7}lecci\u{f3}".into())]
        );
        // The SAME word in NFD: `e` + U+0301 COMBINING ACUTE ACCENT.
        assert_eq!(
            toks("ex:cafe\u{301}"),
            vec![Token::PrefixedName("ex", "cafe\u{301}".into())]
        );
        // ... and in NFC, so the two normalizations agree.
        assert_eq!(
            toks("ex:caf\u{e9}"),
            vec![Token::PrefixedName("ex", "caf\u{e9}".into())]
        );
        assert_eq!(
            toks("ex:a\u{203f}b"),
            vec![Token::PrefixedName("ex", "a\u{203f}b".into())]
        );
        // Prefix, local part and blank-node label all scan with the same class.
        assert_eq!(
            toks("pre\u{b7}fix:local"),
            vec![Token::PrefixedName("pre\u{b7}fix", "local".into())]
        );
        assert_eq!(
            toks("_:cafe\u{301}"),
            vec![Token::BlankNodeLabel("cafe\u{301}")]
        );
        // A CJK name and an astral-plane one are `PN_CHARS_BASE` members too.
        assert_eq!(
            toks("ex:\u{4e2d}\u{6587}"),
            vec![Token::PrefixedName("ex", "\u{4e2d}\u{6587}".into())]
        );
        // U+1F600 is inside `[#x10000-#xEFFFF]`, so the production really does
        // admit this emoji as a name character.
        assert_eq!(
            toks("ex:\u{1f600}"),
            vec![Token::PrefixedName("ex", "\u{1f600}".into())]
        );
    }

    /// `VARNAME ::= ( PN_CHARS_U | [0-9] ) ( PN_CHARS_U | [0-9] | #xB7 |`
    /// `[#x300-#x36F] | [#x203F-#x2040] )*` is POSITION-DEPENDENT, so one
    /// predicate cannot express it. The tail class admits a combining mark, a
    /// MIDDLE DOT and the two ties; scanning the tail with the START class
    /// over-refuses every one of these lawful variables.
    #[test]
    fn varname_continues_with_what_it_may_not_start_with() {
        for name in [
            "a\u{b7}",
            "a\u{300}",
            "a\u{203f}b",
            "a\u{2040}",
            "cafe\u{301}",
            "_1",
            "0",
            "9lives",
        ] {
            assert_eq!(toks(&format!("?{name}")), vec![var(name)], "?{name}");
            assert_eq!(toks(&format!("${name}")), vec![var(name)], "${name}");
        }
        // The mirror: a scalar the CONTINUE class admits may not START a name.
        // The `?` takes the path-operator branch, and the scalar then has to
        // stand on its own — which, being no token's start either, it cannot. The
        // claim is only that no VARIABLE was read, not which arm reports it.
        for c in ['\u{b7}', '\u{300}', '\u{203f}', '\u{2040}'] {
            let src = format!("?{c}");
            assert!(
                no_leading_variable(&src),
                "U+{:04X} may continue a VARNAME but may not begin one",
                c as u32
            );
        }
    }

    /// Whether the leading `?`/`$` of `src` failed to open a variable — the
    /// stream either refused outright or read that `?` as the path zero-or-one
    /// operator. Both are "this is not a variable"; which one occurs depends on
    /// how the tail happens to lex, and is not a vector's claim to make. Shaped
    /// after [`no_leading_iriref`] for the same reason.
    fn no_leading_variable(src: &str) -> bool {
        match tokenize(src) {
            Ok(spanned) => !matches!(spanned.first().map(|s| &s.token), Some(Token::Variable(_))),
            Err(_) => true,
        }
    }

    /// `VARNAME` excludes `'-'` in BOTH positions even though `PN_CHARS`
    /// includes it, so a variable's name scan must not reach for `is_pn_chars`:
    /// `?a-?b` is three tokens and `?x-1` is a subtraction.
    #[test]
    fn varname_never_admits_the_hyphen_pn_chars_does() {
        assert_eq!(toks("?a-?b"), vec![var("a"), Token::Minus, var("b")]);
        assert_eq!(
            toks("?x-1"),
            vec![var("x"), Token::Minus, Token::Integer("1")]
        );
        // And the counterpart that proves `PN_CHARS` still keeps it: a prefixed
        // name's local part scans the hyphen straight through.
        assert_eq!(
            toks("ex:a-b"),
            vec![Token::PrefixedName("ex", "a-b".into())]
        );
    }

    /// Only `PN_CHARS_BASE` opens a keyword or a `PN_PREFIX`. `'_'` is the one
    /// `PN_CHARS_U` member outside it, and in SPARQL it opens exactly one thing —
    /// a `BLANK_NODE_LABEL` — so a bare `_` is not the start of any token.
    #[test]
    fn underscore_opens_a_blank_node_label_and_nothing_else() {
        assert!(tokenize("_x").is_err(), "`_x` starts no SPARQL token");
        // The neighbour: with its colon it is still a blank node, and `_` still
        // scans freely INSIDE a name.
        assert_eq!(toks("_:x"), vec![Token::BlankNodeLabel("x")]);
        assert_eq!(toks("?_x"), vec![var("_x")]);
        assert_eq!(
            toks("ex:a_b"),
            vec![Token::PrefixedName("ex", "a_b".into())]
        );
    }

    /// The three name productions in this grammar open at three DIFFERENT
    /// classes, and each is narrower than the tail it continues into.
    ///
    /// ```text
    /// PN_PREFIX        ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?
    /// PN_LOCAL         ::= (PN_CHARS_U | ':' | [0-9] | PLX) ((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?
    /// BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?
    /// ```
    ///
    /// A prefix may not open at `'_'`; a label may not open at `':'`; neither
    /// may open at a `'-'`, a `'.'`, a MIDDLE DOT, a combining mark or a tie,
    /// though all five continue both. Read as a property over every ASCII scalar
    /// — where every one of the separating scalars lives — rather than as a
    /// hand-picked list, because a hand-picked list is how one of them is missed.
    #[test]
    fn each_name_production_opens_at_its_own_head_class() {
        for c in (0..=0x7F_u8).map(char::from) {
            let probe = format!("_:{c}z");
            let label = tokenize(&probe).ok().and_then(|t| match t.as_slice() {
                [one] => match one.token {
                    Token::BlankNodeLabel(l) => Some(l.to_owned()),
                    _ => None,
                },
                _ => None,
            });
            assert_eq!(
                label.is_some(),
                terminals::is_blank_node_label_start(c),
                "{probe:?} reads as one BLANK_NODE_LABEL exactly when the \
                 production's head admits U+{:04X}",
                u32::from(c)
            );
            if let Some(label) = label {
                assert_eq!(label, format!("{c}z"), "{probe:?}");
            }
        }
    }

    /// The same property for `PN_LOCAL`, whose head carries the one alternative
    /// no character class can answer.
    ///
    /// `PLX ::= PERCENT | PN_LOCAL_ESC` is a SHAPE — `'%' HEX HEX` and `'\' [_~.…]`
    /// — so an escaping `'\'` and a `'%'` with two `HEX` digits behind it open a
    /// local name although [`terminals::is_pn_local_start`] refuses both leads.
    /// Reading them out of the head class is the over-refusal this sweep exists
    /// to catch: `ex:%20a` is a lawful prefixed name.
    ///
    /// Being a shape cuts the other way too, which is why the single-scalar
    /// sweep below can say nothing about either lead: the probe `ex:{c}z` puts a
    /// `'z'` behind the candidate, and `z` is neither a `HEX` digit nor
    /// escapable, so `ex:%z` and `ex:\z` open NO local name. Both shapes are
    /// therefore pinned by their own vectors after the loop, in both directions.
    ///
    /// A head the production does not name does not fail — it ENDS the prefixed
    /// name at the colon, because `PNAME_NS ::= PN_PREFIX? ':'` is itself a
    /// complete terminal. So the assertion is about the token SPLIT, never about
    /// an error arm.
    #[test]
    fn a_local_name_opens_at_its_head_class_plus_the_shapes_of_plx() {
        for c in (0..=0x7F_u8).map(char::from) {
            // Neither `PLX` shape is complete in this probe: `ex:\z` is not a
            // `PN_LOCAL_ESC` (`z` is not escapable) and `ex:%z` is not a
            // `PERCENT` (`z` is not a `HEX` digit), so both leads open nothing
            // here. The completed shapes are pinned below.
            let opens = terminals::is_pn_local_start(c);
            let probe = format!("ex:{c}z");
            for (reading, name) in [
                (tokenize(&probe), "SPARQL"),
                (tokenize_turtle(&probe), "Turtle"),
            ] {
                // The dialect flag makes `/` a `PN_LOCAL` character in Turtle,
                // in every position, which is the one difference between the
                // two entry points.
                let opens = opens || (c == '/' && name == "Turtle");
                // A scalar that opens no local name leaves the `PNAME_NS` whole
                // and is read as whatever token IT starts — which, for a scalar
                // that starts none (a control character, a `"`), is no token at
                // all. Both outcomes say the same thing: the name stopped at the
                // colon.
                let local = reading.ok().map(|spanned| match spanned.first() {
                    Some(Spanned {
                        token: Token::PrefixedName("ex", local),
                        ..
                    }) => local.clone().into_owned(),
                    other => panic!("{name}: {probe:?} must open a prefixed name, got {other:?}"),
                });
                let at = u32::from(c);
                match (opens, local) {
                    (true, Some(local)) => assert_eq!(local, format!("{c}z"), "{name}: {probe:?}"),
                    (true, None) => {
                        panic!("{name}: U+{at:04X} opens a local name, so {probe:?} must lex")
                    }
                    (false, Some(local)) => assert_eq!(
                        local, "",
                        "{name}: U+{at:04X} opens no local name, so {probe:?} \
                         must stop the name at the colon"
                    ),
                    // The scalar opens no local name AND no token of its own, so
                    // the document has no reading. The name still stopped at the
                    // colon, which is what this sweep is about.
                    (false, None) => {}
                }
            }
        }
        // `PLX`'s two shapes, which the character class cannot see, both open a
        // local name — and the escape decodes, exactly as it does mid-name.
        assert_eq!(
            toks("ex:%20a"),
            vec![Token::PrefixedName("ex", "%20a".into())]
        );
        assert_eq!(
            toks("ex:\\~a"),
            vec![Token::PrefixedName("ex", "~a".into())]
        );
        assert_eq!(
            toks("ex:\\.a"),
            vec![Token::PrefixedName("ex", ".a".into())]
        );
    }

    /// `PERCENT ::= '%' HEX HEX` — all three scalars, at the head and in the
    /// tail, with `HEX ::= [0-9] | [A-F] | [a-f]` case-insensitive.
    ///
    /// The scanner used to admit the `'%'` on its own and never look at the two
    /// digits, so `ex:%zz` was one prefixed name here and two tokens in the
    /// workspace's ShExC scanner, which reads the same production. Requiring the
    /// digits is a REFUSAL, so the lawful neighbours are executed beside it:
    /// every case below that the production admits is asserted to still lex as
    /// one name.
    #[test]
    fn percent_needs_both_hex_digits_and_takes_them_in_either_case() {
        // Admitted: the head position, every `HEX` case, and the tail.
        for local in [
            "%20a", // the canonical escaped space
            "%2F",  // upper-case `HEX`
            "%2f",  // lower-case, the same escape
            "%ab",
            "%AB",
            "%aB", // `[a-f]`, `[A-F]`, and mixed within one escape
            "%00",
            "%ff",       // the ends of the byte range
            "a%20b",     // a `PERCENT` in the TAIL, between name characters
            "a%20",      // and at the very end of a name
            "%20%2F%ab", // three in a row
            "%20:a",     // a `PERCENT` beside the `':'` a local name may carry
        ] {
            let probe = format!("ex:{local}");
            assert_eq!(
                toks(&probe),
                vec![Token::PrefixedName("ex", local.into())],
                "{probe:?} is one prefixed name: PERCENT admits it"
            );
        }
        // Refused, at the head: the local name is EMPTY and the `'%'` is left
        // for the next token, which in SPARQL is no token at all.
        for probe in ["ex:%zz", "ex:%2", "ex:%2z", "ex:%z2", "ex:%%20", "ex:%"] {
            assert!(
                tokenize(probe).is_err(),
                "{probe:?} has no reading: `%` opens no PERCENT and starts no token"
            );
            assert!(tokenize_turtle(probe).is_err(), "{probe:?}, as Turtle");
        }
        // Refused in the TAIL, where the name before it is real: the boundary
        // moves, the document does not become unreadable at the name.
        for (probe, local) in [("ex:a%zz", "a"), ("ex:a%2", "a"), ("ex:a%", "a")] {
            let reading = tokenize(probe).ok();
            assert!(
                reading.is_none(),
                "{probe:?}: the `%` is left over and begins no token"
            );
            // The name itself stopped where the production says it stops, which
            // a prefix of the probe shows directly.
            let truncated = format!("{probe} ");
            assert!(tokenize(&truncated).is_err(), "{truncated:?}");
            assert_eq!(
                toks(&format!("ex:{local}")),
                vec![Token::PrefixedName("ex", local.into())]
            );
        }
        // End of input, the case that must not read past the buffer: a `'%'`
        // with nothing behind it, and one with a single digit behind it.
        for probe in ["ex:%", "ex:%2", "ex:a%", "ex:a%2", "%", "_:a%"] {
            let _ = tokenize(probe);
            let _ = tokenize_turtle(probe);
        }
    }

    /// The empty local name and the empty prefix are LAWFUL, and a head-class
    /// check is exactly the change that would refuse them.
    ///
    /// `PNAME_NS ::= PN_PREFIX? ':'` is a terminal in its own right, so `ex:`,
    /// `:` and `:a` are complete prefixed names. `ex:.` is that terminal followed
    /// by the statement-terminating `Dot` — the same reading it had before the
    /// head class existed.
    #[test]
    fn an_empty_local_name_and_an_empty_prefix_are_both_whole_names() {
        assert_eq!(toks("ex:"), vec![Token::PrefixedName("ex", "".into())]);
        assert_eq!(toks(":"), vec![Token::PrefixedName("", "".into())]);
        assert_eq!(toks(":a"), vec![Token::PrefixedName("", "a".into())]);
        assert_eq!(
            toks("ex:."),
            vec![Token::PrefixedName("ex", "".into()), Token::Dot]
        );
        assert_eq!(
            toks("ex: ex:"),
            vec![
                Token::PrefixedName("ex", "".into()),
                Token::PrefixedName("ex", "".into()),
            ]
        );
    }

    /// The non-ASCII half of the head classes, where the interesting scalars are
    /// the ones that continue a name without opening one.
    ///
    /// Each appears TWICE — once at the head, where it must not join, and once
    /// in the tail, where it must — so neither half can be read as blanket
    /// strictness or blanket leniency. `ex:cafe\u{301}` and `_:cafe\u{301}` are
    /// the ones that matter in practice: an NFD-spelled name is exactly as
    /// lawful as its NFC spelling.
    #[test]
    fn a_continue_only_scalar_joins_a_name_and_never_opens_one() {
        for c in ['\u{b7}', '\u{300}', '\u{36f}', '\u{203f}', '\u{2040}'] {
            assert!(terminals::is_pn_chars(c), "U+{:04X}", u32::from(c));
            // Head: the local name is empty and the scalar opens no token of its
            // own, so the document has no reading at all.
            assert!(
                tokenize(&format!("ex:{c}z")).is_err(),
                "`ex:{c}z` opens no local name and `{c}` opens no token"
            );
            assert!(
                tokenize(&format!("_:{c}z")).is_err(),
                "`_:{c}z` opens no blank node label"
            );
            // Tail: the same scalar is part of the name.
            assert_eq!(
                toks(&format!("ex:a{c}z")),
                vec![Token::PrefixedName("ex", format!("a{c}z").into())]
            );
            assert_eq!(
                toks(&format!("_:a{c}z")),
                vec![Token::BlankNodeLabel(&format!("_:a{c}z")[2..])]
            );
        }
        // The lawful non-ASCII heads, so none of the above is an alphabet
        // narrowed to ASCII: a CJK ideograph, an accented Latin letter in NFC,
        // and U+FEFF — which is `[#xFDF0-#xFFFD]`, hence a lawful name start
        // however much it looks like the byte order mark it also spells.
        for c in ['\u{4e2d}', '\u{e9}', '\u{feff}', '\u{1f600}'] {
            assert_eq!(
                toks(&format!("ex:{c}z")),
                vec![Token::PrefixedName("ex", format!("{c}z").into())]
            );
            assert_eq!(
                toks(&format!("_:{c}z")),
                vec![Token::BlankNodeLabel(&format!("_:{c}z")[2..])]
            );
        }
    }

    /// A blank node label's trailing-dot pushback survives the head check, and
    /// an INTERNAL dot is still part of the label.
    ///
    /// The head is never a `'.'` now, so the pushback can no longer trim a label
    /// away to nothing — `_:.` is refused at the head rather than after the trim,
    /// and the trim itself only ever hands back a terminator.
    #[test]
    fn the_label_dot_pushback_still_keeps_internal_dots() {
        assert_eq!(
            toks("_:a.b."),
            vec![Token::BlankNodeLabel("a.b"), Token::Dot]
        );
        assert_eq!(toks("_:a."), vec![Token::BlankNodeLabel("a"), Token::Dot]);
        assert!(tokenize("_:.").is_err(), "`_:.` opens no label");
        assert!(tokenize("_:.a").is_err(), "`_:.a` opens no label");
    }

    /// Tightening the name classes must not have re-broken the `IRIREF` body,
    /// which lawfully carries U+00A0: the two productions disagree about that
    /// scalar and both are now transcribed exactly. (The `IRIREF` block above
    /// owns this claim; it is re-executed here because this change is the one
    /// that could silently reverse it.)
    #[test]
    fn the_name_classes_did_not_re_break_the_iriref_body() {
        assert_eq!(
            toks("<urn:ex:a\u{a0}b>"),
            vec![Token::Iri("urn:ex:a\u{a0}b".into())]
        );
        assert_eq!(
            toks("<urn:ex:a\u{200b}b>"),
            vec![Token::Iri("urn:ex:a\u{200b}b".into())]
        );
    }

    /// A whole-string `VARNAME` test is what the algebra validator needs, and it
    /// is position-dependent in exactly the way the scanner is.
    #[test]
    fn is_varname_answers_the_whole_production() {
        for name in ["s", "_1", "0", "a\u{b7}", "cafe\u{301}", "__purrdf_agg_0"] {
            assert!(is_varname(name), "{name:?}");
        }
        for name in ["", "-a", "a-b", "a\u{a0}b", "\u{300}a", "a b", "a."] {
            assert!(!is_varname(name), "{name:?}");
        }
    }

    // ── The refusal must also NAME what it refused ───────────────────────────
    //
    // Tightening the classes above made `?s<NBSP>?p` an error instead of a wrong
    // answer. That is half the fix. The other half is that the character is
    // INVISIBLE: the author's screen reads `?s ?p` and looks perfectly correct,
    // so `unexpected character '\u{a0}'` leaves them with a refusal they cannot
    // act on — the same invisibility that hid the defect now hides the remedy.
    //
    // So the message names the scalar and the production that refuses it. The
    // vectors below pin the message TEXT (a message nobody reads is not a
    // deliverable), the derivation of the whitespace half of the table, and both
    // directions of the trap: scalars that look like offenders and are lawful
    // (U+200C, U+200D, U+1680) and a visible offender whose ordinary message
    // must not have been swallowed by the new path.

    /// The reason text of the lex failure `src` produces.
    fn lex_reason(src: &str) -> String {
        match tokenize(src) {
            Err(ParseError::Lex { reason, .. }) => reason,
            other => panic!("expected a lex error for {src:?}, got {other:?}"),
        }
    }

    /// The whitespace half of the diagnostic table is EXACTLY the Unicode
    /// `White_Space` property minus the four `WS` members.
    ///
    /// This is the guard the prompt's own history earned: a hand-picked list of
    /// a 26-member class loses the members nobody pictures, and an earlier draft
    /// of this work lost U+205F and U+2029 precisely that way. The literal in
    /// `lexer.rs` carries the NAMES, which cannot be derived without a Unicode
    /// database; its MEMBERSHIP is derived here from `char::is_whitespace` —
    /// which *is* the `White_Space` property — so a future Unicode revision that
    /// adds a space character turns a silent gap into a red test.
    #[test]
    fn the_unicode_space_table_is_the_whole_property_minus_ws() {
        let listed: Vec<char> = NON_WS_UNICODE_SPACES.iter().map(|&(c, _)| c).collect();
        assert_eq!(
            listed,
            non_ws_unicode_whitespace(),
            "the table must be the property, not a hand-picked subset of it"
        );
        // The four `WS` members are absent: they are lawful trivia, so a message
        // saying they "cannot separate tokens" would be a lie.
        for c in [' ', '\t', '\r', '\n'] {
            assert!(
                lookup_scalar_name(NON_WS_UNICODE_SPACES, c).is_none(),
                "U+{:04X} is `WS`",
                c as u32
            );
        }
    }

    /// The two tables are disjoint, duplicate-free and ordered by scalar, and
    /// every entry carries a name — a blank or duplicated entry would produce a
    /// message no better than the one this work replaces.
    #[test]
    fn both_tables_are_ordered_disjoint_and_fully_named() {
        for table in [NON_WS_UNICODE_SPACES, INVISIBLE_NON_SPACES] {
            for window in table.windows(2) {
                let [(a, _), (b, _)] = window else {
                    unreachable!("windows(2) yields pairs")
                };
                assert!(a < b, "U+{:04X} is out of order", *b as u32);
            }
            for &(c, name) in table {
                assert!(!name.is_empty(), "U+{:04X} has no name", c as u32);
            }
        }
        for &(c, _) in INVISIBLE_NON_SPACES {
            assert!(
                !c.is_whitespace(),
                "U+{:04X} carries White_Space and belongs in the other table",
                c as u32
            );
            assert!(lookup_scalar_name(NON_WS_UNICODE_SPACES, c).is_none());
        }
    }

    /// The headline message, pinned verbatim for the two scalars the report
    /// names: the offender is identified by code point AND by Unicode name, the
    /// violated production is quoted as the grammar spells it, and the reader is
    /// told what to type instead.
    #[test]
    fn a_unicode_space_is_named_along_with_the_production_that_refuses_it() {
        // Spelled on one line each, with no `\` continuation: the golden text is
        // greppable, and it cannot agree with the builder merely because both
        // were assembled the same way.
        assert_eq!(
            lex_reason("?s\u{a0}?p"),
            "U+00A0 NO-BREAK SPACE cannot separate tokens: WS is #x20 | #x9 | #xD | #xA. Replace it with a space."
        );
        assert_eq!(
            lex_reason("?s\u{3000}?p"),
            "U+3000 IDEOGRAPHIC SPACE cannot separate tokens: WS is #x20 | #x9 | #xD | #xA. Replace it with a space."
        );
        // The same message wherever the scalar lands — it is a property of the
        // character and of the production, not of the surrounding tokens.
        assert_eq!(lex_reason("{\u{a0}}"), lex_reason("_:a\u{a0}b"));
    }

    /// A scalar that is invisible WITHOUT carrying `White_Space` gets its own
    /// message: no trivia-side story reaches it, so being told which production
    /// declines it is the only way a reader can act.
    #[test]
    fn a_zero_width_space_is_named_even_though_it_is_not_whitespace() {
        assert!(!'\u{200b}'.is_whitespace());
        assert_eq!(
            lex_reason("?s\u{200b}?p"),
            "U+200B ZERO WIDTH SPACE is invisible and starts no token: PN_CHARS does not name it, and WS is #x20 | #x9 | #xD | #xA. Delete it."
        );
        // A soft hyphen is the same story with a different disguise: it renders
        // as nothing at all (or as a hyphen only at a line break).
        assert_eq!(
            lex_reason("ex:a\u{ad}b"),
            "U+00AD SOFT HYPHEN is invisible and starts no token: PN_CHARS does not name it, and WS is #x20 | #x9 | #xD | #xA. Delete it."
        );
    }

    /// **A third instance of the trap, found by this work.** U+FEFF ZERO WIDTH
    /// NO-BREAK SPACE — a byte-order mark at the head of a `.rq` file — is
    /// invisible, is not `White_Space`, and is nevertheless a lawful
    /// `PN_CHARS_BASE` character, because that production runs to
    /// `[#xFDF0-#xFFFD]`. So `\u{feff}SELECT` is ONE word and the lexer is right
    /// not to refuse it; the refusal belongs to the parser, which knows that no
    /// keyword is spelled that way.
    ///
    /// Pinned positively for the same reason U+1680 is: it reads exactly like
    /// the U+00A0 defect and is not one, and "fixing" it inside the lexer would
    /// manufacture an over-refusal of a scalar the grammar admits.
    #[test]
    fn the_byte_order_mark_is_a_name_character_too() {
        assert!(!'\u{feff}'.is_whitespace());
        assert!(terminals::is_pn_chars_base('\u{feff}'));
        assert_eq!(
            toks("\u{feff}SELECT"),
            vec![Token::Word("\u{feff}SELECT")],
            "U+FEFF is inside PN_CHARS_BASE's [#xFDF0-#xFFFD]"
        );
        assert_eq!(
            toks("ex:a\u{feff}b"),
            vec![Token::PrefixedName("ex", "a\u{feff}b".into())]
        );
    }

    /// Every listed offender that can actually reach the arm is named there —
    /// the table is wired in, not merely present — and no message smuggles the
    /// invisible scalar itself in as its own description.
    ///
    /// Two listed scalars never reach it, and that is the point of the whole
    /// design rather than an exception to it: U+1680 and U+FEFF are
    /// `PN_CHARS_BASE` members, so [`Lexer::lex_one`] hands them to the name
    /// scan before the diagnostic arm exists. Being an offender is a property of
    /// the POSITION, not of the character. Each has its own vector below.
    #[test]
    fn every_listed_offender_reaches_the_arm_with_its_name() {
        for &(c, name) in NON_WS_UNICODE_SPACES {
            if terminals::is_pn_chars_base(c) {
                continue; // a NAME character; see the U+1680 vector below
            }
            let reason = lex_reason(&format!("{{{c}}}"));
            assert!(
                reason.starts_with(&format!(
                    "U+{:04X} {name} cannot separate tokens:",
                    c as u32
                )),
                "U+{:04X} reported as {reason:?}",
                c as u32
            );
            assert!(!reason.contains(c), "the message must not be invisible too");
        }
        for &(c, name) in INVISIBLE_NON_SPACES {
            if terminals::is_pn_chars_base(c) {
                continue; // a NAME character; see the U+FEFF vector above
            }
            let reason = lex_reason(&format!("{{{c}}}"));
            assert!(
                reason.starts_with(&format!(
                    "U+{:04X} {name} is invisible and starts no token:",
                    c as u32
                )),
                "U+{:04X} reported as {reason:?}",
                c as u32
            );
            assert!(!reason.contains(c), "the message must not be invisible too");
        }
    }

    /// **The trap.** U+200C ZERO WIDTH NON-JOINER and U+200D ZERO WIDTH JOINER
    /// are invisible format characters AND lawful name characters:
    /// `PN_CHARS_BASE` names `[#x200C-#x200D]` outright. A diagnostic table built
    /// by sweeping "the invisible characters" hoovers them up, and then tells a
    /// user their VALID query is wrong. Asserted in both directions: absent from
    /// the tables, and still lexing as part of a single name.
    #[test]
    fn the_two_joiners_are_lawful_names_and_are_not_offenders() {
        for c in ['\u{200c}', '\u{200d}'] {
            assert!(
                lookup_scalar_name(NON_WS_UNICODE_SPACES, c).is_none()
                    && lookup_scalar_name(INVISIBLE_NON_SPACES, c).is_none(),
                "U+{:04X} is inside PN_CHARS_BASE's [#x200C-#x200D] and is not an offender",
                c as u32
            );
            assert!(
                invisible_scalar_diagnostic(c).is_none(),
                "U+{:04X} must have no offender message at all",
                c as u32
            );
            assert_eq!(
                toks(&format!("ex:a{c}b")),
                vec![Token::PrefixedName("ex", format!("a{c}b").into())]
            );
            assert_eq!(toks(&format!("?a{c}b")), vec![var(&format!("a{c}b"))]);
        }
    }

    /// **The other direction.** U+1680 OGHAM SPACE MARK carries `White_Space`
    /// AND lies inside `PN_CHARS_BASE`'s `[#x37F-#x1FFF]`, so the grammar makes
    /// it a NAME character: `?s\u{1680}?p` is ONE variable followed by another,
    /// and it must never be reported as a failed token separator.
    ///
    /// It is listed in the table because the table is the `White_Space`
    /// property, and it is unreachable from there because [`Lexer::lex_one`]
    /// dispatches `is_pn_chars_base` BEFORE the diagnostic arm — membership is a
    /// property of the character, but being an offender is a property of the
    /// position. That is the whole reason the table cannot be "every invisible
    /// character".
    #[test]
    fn the_ogham_space_mark_is_never_reported_as_a_separator() {
        assert!('\u{1680}'.is_whitespace());
        assert!(terminals::is_pn_chars_base('\u{1680}'));
        assert!(lookup_scalar_name(NON_WS_UNICODE_SPACES, '\u{1680}').is_some());
        // ... and yet nothing reports it, because nothing refuses it.
        assert_eq!(
            toks("?s\u{1680}?p"),
            vec![var("s\u{1680}"), var("p")],
            "U+1680 joins the first name rather than ending it"
        );
        assert_eq!(
            toks("ex:a\u{1680}b"),
            vec![Token::PrefixedName("ex", "a\u{1680}b".into())]
        );
        assert!(tokenize("{\u{1680}}").is_ok_and(|s| s.len() == 3), "a Word");
    }

    /// A VISIBLE offender keeps the ordinary message: the new path names what a
    /// reader cannot see, and swallows nothing else. `'%'` shows the author
    /// exactly what they typed, so naming its code point would be noise.
    #[test]
    fn a_visible_offender_keeps_the_general_message() {
        assert_eq!(lex_reason("%"), "unexpected character '%'");
        assert_eq!(lex_reason("?s % ?p"), "unexpected character '%'");
        assert_eq!(lex_reason("\u{a7}"), "unexpected character '\u{a7}'");
        for c in ['%', '\u{a7}'] {
            assert!(
                invisible_scalar_diagnostic(c).is_none(),
                "{c:?} renders as its own glyph"
            );
        }
    }

    /// The diagnostic must never fire INSIDE an `IRIREF`. That body excludes
    /// only `#x00-#x20` and nine delimiters, so all but two of the listed
    /// scalars are LAWFUL there — and the workspace's own writers emit them raw,
    /// so a diagnostic firing inside `<...>` would break a round trip the crate
    /// already guarantees.
    ///
    /// The two exceptions are the reason this is a per-production question and
    /// not a per-character one: U+000B LINE TABULATION and U+000C FORM FEED are
    /// inside `#x00-#x20`, so the `IRIREF` body must still END at them. Both
    /// directions run here.
    #[test]
    fn the_diagnostic_never_fires_inside_an_iriref() {
        for &(c, _) in NON_WS_UNICODE_SPACES.iter().chain(INVISIBLE_NON_SPACES) {
            let src = format!("<urn:ex:a{c}b>");
            if terminals::is_iriref_forbidden(c) {
                assert!(
                    no_leading_iriref(&src),
                    "U+{:04X} is inside #x00-#x20 and still ends the body",
                    c as u32
                );
                continue;
            }
            assert_eq!(
                toks(&src),
                vec![Token::Iri(format!("urn:ex:a{c}b").into())],
                "U+{:04X} is lawful raw inside an IRIREF",
                c as u32
            );
        }
        // Exactly two of them are the forbidden ones, so the branch above is not
        // quietly excusing the whole table.
        let forbidden: Vec<char> = NON_WS_UNICODE_SPACES
            .iter()
            .chain(INVISIBLE_NON_SPACES)
            .filter(|&&(c, _)| terminals::is_iriref_forbidden(c))
            .map(|&(c, _)| c)
            .collect();
        assert_eq!(forbidden, vec!['\u{b}', '\u{c}']);
        // The neighbouring refusal still holds: an ASCII SPACE ends the body.
        assert!(no_leading_iriref("<urn:ex:a b>"));
    }

    /// The message builder is TOTAL: every scalar gets exactly one of the two
    /// messages, and which one is decided solely by table membership. A builder
    /// that panicked would turn a reported error into a crash on input that is
    /// already known to be invalid, so it holds no index, unwrap or slice.
    #[test]
    fn the_message_builder_answers_for_every_scalar() {
        for c in (0..=0xFFFF_u32)
            .chain([0x1_0000, 0x1_F600, 0x10_FFFF])
            .filter_map(char::from_u32)
        {
            let listed = lookup_scalar_name(NON_WS_UNICODE_SPACES, c).is_some()
                || lookup_scalar_name(INVISIBLE_NON_SPACES, c).is_some();
            let message = unexpected_character(c);
            assert_eq!(
                listed,
                message.starts_with("U+"),
                "U+{:04X} got {message:?}",
                c as u32
            );
            assert_eq!(
                listed,
                !message.starts_with("unexpected character "),
                "U+{:04X} got {message:?}",
                c as u32
            );
        }
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
                Token::Word("a"),
                Token::PrefixedName("rdf", "type".into()),
                Token::Dot,
            ]
        );
        assert_eq!(
            toks("PREFIX purrdf: <u:>"),
            vec![
                Token::Word("PREFIX"),
                Token::PrefixedName("purrdf", "".into()),
                Token::Iri("u:".into()),
            ]
        );
    }

    #[test]
    fn property_path_operators() {
        assert_eq!(
            toks("owl:members/rdf:rest*/rdf:first"),
            vec![
                Token::PrefixedName("owl", "members".into()),
                Token::Slash,
                Token::PrefixedName("rdf", "rest".into()),
                Token::Star,
                Token::Slash,
                Token::PrefixedName("rdf", "first".into()),
            ]
        );
    }

    #[test]
    fn literals_and_lang() {
        assert_eq!(
            toks("\"hi\"@en"),
            vec![Token::StringLit("hi".into()), Token::LangTag("en")]
        );
        assert_eq!(
            toks("\"x\"^^xsd:string"),
            vec![
                Token::StringLit("x".into()),
                Token::HatHat,
                Token::PrefixedName("xsd", "string".into()),
            ]
        );
        assert_eq!(toks("3"), vec![Token::Integer("3")]);
        assert_eq!(toks("3.5"), vec![Token::Decimal("3.5")]);
        assert_eq!(toks("1e9"), vec![Token::Double("1e9")]);
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
            vec![Token::Word("SELECT"), var("x")]
        );
    }

    #[test]
    fn anon_and_blank() {
        assert_eq!(toks("[]"), vec![Token::Anon]);
        assert_eq!(toks("_:b1"), vec![Token::BlankNodeLabel("b1")]);
    }

    #[test]
    fn blank_label_trailing_dot_is_statement_terminator() {
        // A trailing `.` after a blank-node label is the Turtle statement
        // terminator, not part of the label: it must surface as its own `Dot`
        // token and must not be swallowed into the label string.
        assert_eq!(
            toks_turtle(":x :p _:y."),
            vec![
                Token::PrefixedName("", "x".into()),
                Token::PrefixedName("", "p".into()),
                Token::BlankNodeLabel("y"),
                Token::Dot,
            ]
        );
        // An internal `.` is kept as part of the label; only the final,
        // over-consumed trailing dot is pushed back as the terminator.
        assert_eq!(
            toks_turtle("_:a.b."),
            vec![Token::BlankNodeLabel("a.b"), Token::Dot]
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
                Token::PrefixedName("purrdf", "p".into()),
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

    // ── Trailing dot must NOT be absorbed into a number ──────────────────────

    #[test]
    fn trailing_dot_is_separator_not_decimal() {
        // `3 .` — the dot is a statement separator, not part of the literal.
        assert_eq!(toks("3 ."), vec![Token::Integer("3"), Token::Dot]);
    }

    #[test]
    fn number_in_triple_pattern_dot_separator() {
        // Simulates `?o 3 .` — `3` must come out as Integer, not Decimal("3.").
        assert_eq!(
            toks("?o 3 ."),
            vec![var("o"), Token::Integer("3"), Token::Dot]
        );
    }

    #[test]
    fn decimal_with_digit_after_dot_still_works() {
        // Smoke-test: `1.5` must remain Decimal.
        assert_eq!(toks("1.5"), vec![Token::Decimal("1.5")]);
    }

    // ── Exponent requires at least one digit after e/E ────────────────────────

    #[test]
    fn double_exponent_no_digit_yields_integer_then_word() {
        // `1e` — no digit follows `e`, so `1` is Integer and `e` is a Word.
        assert_eq!(toks("1e"), vec![Token::Integer("1"), Token::Word("e")]);
    }

    #[test]
    fn exponent_followed_by_non_digit_word() {
        // `1err` — `e` has no digit after it, so `1` is Integer; `err` is a Word.
        assert_eq!(toks("1err"), vec![Token::Integer("1"), Token::Word("err")]);
    }

    #[test]
    fn double_exponent_still_works() {
        // Smoke-test: `1e9` must still be Double.
        assert_eq!(toks("1e9"), vec![Token::Double("1e9")]);
    }

    #[test]
    fn double_exponent_with_sign_still_works() {
        // Smoke-test: `1.5e-3` must still be Double.
        assert_eq!(toks("1.5e-3"), vec![Token::Double("1.5e-3")]);
    }

    #[test]
    fn double_exponent_with_plus_sign_still_works() {
        // `2E+10` must still be Double.
        assert_eq!(toks("2E+10"), vec![Token::Double("2E+10")]);
    }

    fn var(n: &str) -> Token<'_> {
        Token::Variable(n)
    }

    // ── SEP-0008 SHA-3: the only built-in names containing a `-` ─────────────

    /// `SHA3-224` and friends must arrive at the parser as ONE [`Token::Word`].
    ///
    /// `is_pn_chars` admits `-`, and `take_pn_prefix` scans with it, so the
    /// hyphenated names need no lexer special case — but nothing pinned that
    /// before SEP-0008 added the first hyphen-bearing keyword. If the scan ever
    /// stopped at `-`, `builtin_function("SHA3-224")` could never fire and the
    /// call would silently degrade into `Word("SHA3")` followed by a
    /// subtraction.
    #[test]
    fn hyphenated_sha3_names_are_single_words() {
        for name in ["SHA3-224", "SHA3-256", "SHA3-384", "SHA3-512"] {
            assert_eq!(toks(name), vec![Token::Word(name)]);
        }
        assert_eq!(
            toks("SHA3-256(\"abc\")"),
            vec![
                Token::Word("SHA3-256"),
                Token::LParen,
                Token::StringLit("abc".into()),
                Token::RParen,
            ]
        );
    }

    /// SEP-0008 spells the same four functions with an UNDERSCORE, and that
    /// spelling must also arrive as ONE word — `_` is `PN_CHARS_U`, so the same
    /// scan covers it, and no `SHA3` / `_224` split can occur (there is no
    /// underscore operator to split on in the first place).
    #[test]
    fn underscored_sha3_names_are_single_words() {
        for name in ["SHA3_224", "SHA3_256", "SHA3_384", "SHA3_512"] {
            assert_eq!(toks(name), vec![Token::Word(name)]);
        }
        assert_eq!(
            toks("sha3_256(\"abc\")"),
            vec![
                Token::Word("sha3_256"),
                Token::LParen,
                Token::StringLit("abc".into()),
                Token::RParen,
            ]
        );
    }

    /// The counterpart: WHITESPACE around the hyphen makes it the subtraction
    /// operator again, so `SHA3 - 224` is three tokens and cannot be confused
    /// with the built-in. This is what makes the single-word rule above a
    /// *lexical* fact rather than an accident of the input string.
    #[test]
    fn spaced_hyphen_stays_the_subtraction_operator() {
        assert_eq!(
            toks("SHA3 - 224"),
            vec![Token::Word("SHA3"), Token::Minus, Token::Integer("224")]
        );
        // And a genuine subtraction whose left operand ends a word-like token is
        // unaffected: a variable's name scan (`VARNAME`) excludes `-`.
        assert_eq!(
            toks("?sha3-224"),
            vec![var("sha3"), Token::Minus, Token::Integer("224")]
        );
    }

    // ── Turtle-only PN_LOCAL slash leniency (default OFF for SPARQL) ─────────

    fn turtle_toks(s: &str) -> Vec<Token<'_>> {
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
            vec![Token::PrefixedName("purrdf", "report/shacl/sarif".into())]
        );
        assert_eq!(
            turtle_toks("purrdf:projection/okf"),
            vec![Token::PrefixedName("purrdf", "projection/okf".into())]
        );
    }

    #[test]
    fn sparql_default_keeps_slash_as_path_operator() {
        // The SPARQL entry (default options) MUST still split on `/` so property
        // paths like `foaf:knows/foaf:name` keep the sequence operator.
        assert_eq!(
            toks("purrdf:report/shacl/sarif"),
            vec![
                Token::PrefixedName("purrdf", "report".into()),
                Token::Slash,
                Token::Word("shacl"),
                Token::Slash,
                Token::Word("sarif"),
            ]
        );
    }
}

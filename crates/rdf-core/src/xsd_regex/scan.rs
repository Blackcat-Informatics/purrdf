// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The one left-to-right tokenizer over the XSD/XPath `regExp` grammar.
//!
//! Every consumer in this module walks the same raw pattern text: the `x`-flag
//! stripper, the translator, and the ECMA-262 divergence scan. Each used to
//! carry its own character cursor and re-derive character-class nesting from
//! scratch, which is one recognition decision written three times. [`Scanner`]
//! is the single cursor: it owns every character index and lookahead, the
//! class-frame stack, and the capturing-group bookkeeping, and yields
//! [`Token`]s whose **source spans** let a consumer that must reproduce the raw
//! text (the `x` stripper) do so without a second cursor.
//!
//! [`Scanner`] is a resumable, fallible iterator: a rejected construct yields
//! `Some(Err(..))` **after** consuming its own spelling, so the next call
//! continues with the rest of the pattern. Translation stops at the first
//! error; the `x` stripper does not, because its output is a pure textual
//! rewrite of characters it has already been given a span for.

use std::ops::Range;

use super::error::XsdRegexError;

/// One lexical unit of the XSD/XPath `regExp` grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Token {
    /// An ordinary character, literal in this context (`.`, `-`, `(`, `)`,
    /// `]` inside a class are [`Token::Literal`]s, not metacharacters).
    Literal(char),
    /// An ordinary character-class member that `regex-syntax` would read as
    /// part of a set operator: an unescaped `&&` is set INTERSECTION and `~~`
    /// is SYMMETRIC DIFFERENCE there, but XML Schema Part 2 Appendix G's
    /// `XmlCharIncDash ::= [^\#x5B#x5D]` makes both `&` and `~` ordinary
    /// members. The emitter escapes it (`\&`, `\~`) so the XSD language is
    /// preserved exactly.
    ClassMember(char),
    /// Any escape with no grammar-specific rewrite (`\d`, `\D`, `\n`, `\\`,
    /// …), passed through. Only escapes that are actually in the governing
    /// grammar reach this variant: `scan_escape` refuses everything else by
    /// name, so `regex-syntax`'s own escape table never decides acceptance.
    Escape(char),
    /// `\i`/`\I` (`chars == false`) or `\c`/`\C` (`chars == true`).
    NameEscape { negated: bool, chars: bool },
    /// `\s`/`\S`.
    SpaceEscape { negated: bool },
    /// `\w`/`\W`.
    WordEscape { negated: bool },
    /// `\p{…}`/`\P{…}` with its name scanned from between the braces.
    UnicodeProperty { negated: bool, name: String },
    /// An unescaped `.` outside a class.
    Dot,
    /// An unescaped `[`, consuming a following `^` when negated.
    ClassOpen { negated: bool },
    /// An unescaped `]` closing the innermost open class.
    ClassClose,
    /// The `-` of `-[`; the following `[` is its own [`Token::ClassOpen`].
    Subtract,
    /// An unescaped `(` outside a class.
    GroupOpen { capturing: bool },
    /// An unescaped `)` outside a class.
    GroupClose,
    /// A well-formed back-reference to the `n`th capturing group.
    Backreference(u32),
}

impl Token {
    /// Whether this token is an ordinary member of the character class it was
    /// scanned inside. A `]` seen while the innermost open frame has recorded
    /// no member is the Rust-ism of a literal `]` at the class head (`[]a]`),
    /// which XSD's `charGroup` forbids; this predicate lets the accumulator in
    /// [`Scanner::scan_next`] recognize the members that make a later `]` a
    /// legitimate close.
    fn is_class_member(&self) -> bool {
        matches!(
            self,
            Self::Literal(_)
                | Self::Escape(_)
                | Self::NameEscape { .. }
                | Self::SpaceEscape { .. }
                | Self::WordEscape { .. }
                | Self::UnicodeProperty { .. }
                | Self::ClassMember(_)
        )
    }
}

/// One open character-class level, tracked because XSD's `charClassSub`
/// subtracts from a `negCharGroup` as readily as from a `posCharGroup`
/// (`charClassSub ::= ( posCharGroup | negCharGroup ) '-' charClassExpr`)
/// while the `regex` crate's `^` binds *outside* its set operators.
struct ClassFrame {
    /// Whether this level opened as `[^…` and therefore had its negation
    /// emitted on an inner, wrapped group.
    negated: bool,
    /// Whether a `-[` subtraction has already been emitted at this level
    /// (which also closed the inner negated group, if there was one).
    subtracted: bool,
    /// Whether any member of this level has been consumed yet. XSD's
    /// `posCharGroup ::= ( charRange | charClassEsc )+` is non-empty, so a `]`
    /// while this is clear is the literal-first-member Rust-ism, not a close.
    member_seen: bool,
}

/// Capturing-group bookkeeping, kept solely so a back-reference can be
/// **tokenized** the way the specification says rather than greedily.
///
/// XPath F&O 3.1 §5.6.1.4 makes the scan context-dependent, which is unusual
/// enough to quote:
///
/// > The construct `\N` where N is a single digit is always recognized as a
/// > back-reference; if this is followed by further digits, these digits are
/// > taken to be part of the back-reference **if and only if** the resulting
/// > number NN is such that the back-reference is preceded by the opening
/// > parenthesis of the NNth capturing left parenthesis.
///
/// So `(a)\12` is `\1` followed by a literal `2`, while the same text after
/// twelve capturing groups is `\12`. A greedy longest-digit-run scan gets the
/// first case wrong, and since this module's whole contract for a
/// back-reference is an error message that names the construct it found, a
/// wrong tokenization is a message that lies about the pattern.
///
/// The same clause supplies the validity rule: the expression is invalid if a
/// back-reference "refers to a capturing sub-expression that does not exist or
/// whose closing right parenthesis occurs after the back-reference" — which is
/// a MALFORMED pattern, refused by any engine, as distinct from a well-formed
/// back-reference this implementation declines to execute.
#[derive(Default)]
struct Groups {
    /// Capturing `(` seen so far, in source order.
    opened: u32,
    /// The numbers of the capturing groups still open at this point; a group
    /// not on this stack has had its `)` already, and may be referred to.
    open_stack: Vec<u32>,
}

impl Groups {
    fn open(&mut self) {
        self.opened += 1;
        self.open_stack.push(self.opened);
    }

    fn close(&mut self) {
        self.open_stack.pop();
    }

    /// Whether the `n`th capturing group's `(` precedes this point — the test
    /// that decides whether one more digit joins a back-reference.
    fn opened_before(&self, n: u32) -> bool {
        n >= 1 && n <= self.opened
    }

    /// Whether the `n`th capturing group's `)` also precedes this point, which
    /// is what makes a reference to it well-formed rather than merely
    /// well-tokenized.
    fn closed_before(&self, n: u32) -> bool {
        self.opened_before(n) && !self.open_stack.contains(&n)
    }
}

/// The single cursor over an XSD/XPath `regExp` pattern.
pub(super) struct Scanner<'a> {
    pattern: &'a str,
    chars: Vec<char>,
    /// Byte offset of each character, with a trailing sentinel equal to
    /// `pattern.len()`, so character index `i` spans `offsets[i]..offsets[i + 1]`.
    offsets: Vec<usize>,
    /// Character index of the next unscanned character.
    pos: usize,
    /// One frame per open `[`.
    classes: Vec<ClassFrame>,
    /// Capturing-group bookkeeping (see [`Groups`]).
    groups: Groups,
    /// Byte span of the most recently yielded token or error.
    span: Range<usize>,
    /// The `x`-flag stripper's own class depth, maintained from the raw
    /// characters as they are consumed. It is NOT the grammar's class depth:
    /// `x`'s removal is textual and a removed whitespace character re-binds a
    /// pending `\` to whatever follows, so `\ [` delimits no class even though
    /// the grammar reads `\ ` as an escape and `[` as an opener. See
    /// [`Scanner::in_class`].
    x_depth: usize,
    /// The `x`-flag stripper's pending-escape flag (see [`Scanner::x_feed`]).
    x_escaped: bool,
    /// Per character, whether the `x`-flag state machine had it inside a class
    /// at that character. A single token can span a depth change (a
    /// `\p{…}` name may contain an unprotected `[`), so the stripper cannot
    /// answer from one flag per token.
    x_inside: Vec<bool>,
    /// Character range of the most recently yielded token or error.
    span_chars: Range<usize>,
    /// Emission hint for the class token just yielded: whether a wrapped
    /// negated inner class is still open and must be closed before the token's
    /// own bracket work. See [`Scanner::closes_negated_wrap`].
    negated_wrap_close: bool,
    /// Whether the `-` of a `-[` was just consumed, so the `[` the cursor now
    /// points at is that subtraction's `charClassExpr` rather than the
    /// malformed nested class the Rust dialect would accept. Set by the `-`
    /// arm and consumed by the very next `[`; the two are adjacent by
    /// construction, since the `-` only becomes [`Token::Subtract`] when its
    /// lookahead is already `[`.
    subtraction_operand: bool,
}

impl<'a> Scanner<'a> {
    /// Begin a scan of `pattern`.
    pub(super) fn new(pattern: &'a str) -> Self {
        let mut chars = Vec::with_capacity(pattern.len());
        let mut offsets = Vec::with_capacity(pattern.len() + 1);
        for (i, c) in pattern.char_indices() {
            offsets.push(i);
            chars.push(c);
        }
        offsets.push(pattern.len());
        Self {
            pattern,
            chars,
            offsets,
            pos: 0,
            classes: Vec::new(),
            groups: Groups::default(),
            span: 0..0,
            x_depth: 0,
            x_escaped: false,
            x_inside: Vec::with_capacity(pattern.len()),
            span_chars: 0..0,
            negated_wrap_close: false,
            subtraction_operand: false,
        }
    }

    /// The raw characters of the most recently yielded token or error, each
    /// paired with whether the `x`-flag state machine was inside a character
    /// class at that character (i.e. whether its whitespace is exempt from
    /// removal). This is the token-span round-trip the `x` stripper folds over.
    pub(super) fn source_chars(&self) -> impl Iterator<Item = (char, bool)> + '_ {
        let span = self.span_chars.clone();
        self.source_text()
            .chars()
            .zip(self.x_inside[span].iter().copied())
    }

    /// Whether the cursor is currently inside a character class according to
    /// the `x`-flag stripper's textual state machine — the state that decides
    /// whether a whitespace character is exempt from removal.
    ///
    /// This deliberately differs from the grammar's [`ClassFrame`] stack: it
    /// folds a removed whitespace character back into a pending `\` (the doc
    /// comment of [`super::xflag::strip_x_flag_whitespace`] explains why), so
    /// the `[` of `\ [` opens nothing here while the grammar still reads it as
    /// a [`Token::ClassOpen`]. Both states are owned by this one cursor.
    #[allow(
        dead_code,
        reason = "exercised by the scanner's unit tests and kept for later consumers that \
                  need only a token-boundary answer; the x stripper requires the per-character \
                  `source_chars` because one token can span a depth change"
    )]
    pub(super) fn in_class(&self) -> bool {
        self.x_depth > 0
    }

    /// Whether the class token just yielded must close a wrapped negated inner
    /// group *before* its own bracket work.
    ///
    /// `regex` binds `^` outside its set operators, so XSD's `[^g-[e]]` (the
    /// complement of `g`, minus `e`) is emitted as `[[^g]--e]` with the
    /// negation pushed onto an inner group. That inner group closes early on
    /// the first `-[` subtraction at a negated level, so `--` subtracts from
    /// the complement rather than around it; a negated level that saw no
    /// subtraction closes it at its own `]` instead. True for exactly those two
    /// tokens — the `Subtract` and `ClassClose` arms of [`super::emit`] read
    /// this, and the [`ClassFrame`] stack that decides it stays here, not in
    /// the emitter.
    pub(super) fn closes_negated_wrap(&self) -> bool {
        self.negated_wrap_close
    }

    /// Advance the `x`-flag textual state machine over one raw character. This
    /// is the same left-to-right `escaped`/`class_depth` fold the stripper
    /// used before there was a tokenizer: a whitespace character while
    /// `x_escaped` and at depth 0 is REMOVED, so it does not clear the pending
    /// escape (that re-binding is what turns `\ s` into `\s`).
    fn x_feed(&mut self, c: char) {
        self.x_inside.push(self.x_depth > 0);
        if self.x_escaped {
            if self.x_depth != 0 || !purrdf_iri::terminals::is_ws_char(c) {
                self.x_escaped = false;
            }
            return;
        }
        match c {
            '\\' => self.x_escaped = true,
            '[' => self.x_depth += 1,
            ']' if self.x_depth > 0 => self.x_depth -= 1,
            _ => {}
        }
    }

    /// The exact source text the most recently yielded token or error was
    /// scanned from. Empty before the first item and once the iterator ends.
    pub(super) fn source_text(&self) -> &'a str {
        let pattern: &'a str = self.pattern;
        &pattern[self.span.clone()]
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.pos + ahead).copied()
    }

    fn scan_next(&mut self) -> Option<Result<Token, XsdRegexError>> {
        if self.pos >= self.chars.len() {
            self.span = 0..0;
            self.span_chars = 0..0;
            if !self.classes.is_empty() {
                self.classes.clear();
                return Some(Err(XsdRegexError::UnterminatedCharacterClass));
            }
            return None;
        }
        let start = self.pos;
        let result = self.scan_one();
        // Record class content for the frame the token was scanned into. A
        // `]` reads this to tell a legitimate close from the literal-first-
        // member Rust-ism; a `ClassOpen` is not content and pushes its own
        // frame in `scan_one`, so the already-open frame is unaffected.
        if let Ok(token) = &result
            && token.is_class_member()
            && let Some(frame) = self.classes.last_mut()
        {
            frame.member_seen = true;
        }
        self.span = self.offsets[start]..self.offsets[self.pos];
        self.span_chars = start..self.pos;
        for i in start..self.pos {
            self.x_feed(self.chars[i]);
        }
        Some(result)
    }

    fn scan_one(&mut self) -> Result<Token, XsdRegexError> {
        self.negated_wrap_close = false;
        match self.chars[self.pos] {
            '\\' => self.scan_escape(),
            '[' => {
                // A `[` legitimately appears inside an open class in exactly
                // one place: as the operand `charClassExpr` of a `-[`
                // subtraction. The `-` arm below records that with
                // `subtraction_operand`, and this arm consumes the record.
                // Any other `[` inside a class is the Rust nested-class
                // union (`[a[b]]`, `[[:alpha:]]`), which XSD's
                // `charGroup` has no production for; a literal `[` there is
                // the `SingleCharEsc` spelling `\[`.
                let subtraction_operand = std::mem::take(&mut self.subtraction_operand);
                if !self.classes.is_empty() && !subtraction_operand {
                    self.pos += 1;
                    return Err(XsdRegexError::UnescapedClassOpen);
                }
                // XSD `[^g-e]` is (complement of `g`) minus `e`, but
                // Rust's `[^g--e]` applies `^` to the WHOLE class
                // expression — i.e. the complement of (`g` minus `e`),
                // a different set. Opening an extra bracket and putting
                // the negation on the inner one restores XSD's binding:
                // `[[^g]--e]`. Emitted for every negated group, not only
                // the ones a subtraction follows, because the wrap is
                // semantically free when it does not (`[[^g]]` == `[^g]`)
                // and deciding otherwise would need a lookahead over the
                // whole group. The scanner records the `negated` bit on the
                // frame; the emitter emits the wrap.
                let negated = self.peek(1) == Some('^');
                self.pos += if negated { 2 } else { 1 };
                self.classes.push(ClassFrame {
                    negated,
                    subtracted: false,
                    member_seen: false,
                });
                Ok(Token::ClassOpen { negated })
            }
            // A `]` only ever CLOSES a character class. Outside one it is not
            // a `Char` of the grammar at all (`Char ::= [^.\?*+()|#x5B#x5D]`
            // excludes it), so the bare `]` of `[a]]` is malformed, not a
            // literal.
            ']' if self.classes.is_empty() => {
                self.pos += 1;
                Err(XsdRegexError::UnescapedClassClose)
            }
            ']' => {
                if !self
                    .classes
                    .last()
                    .expect("non-empty checked by the previous arm")
                    .member_seen
                {
                    // XSD's `charGroup` is non-empty, so `]` cannot close a
                    // class before any member. Rust reads `[]a]` as a class
                    // holding `]` and `a`; XSD reads an (illegal) empty class
                    // then a bare `]`, so the correct spelling is `\]a]`.
                    self.pos += 1;
                    self.classes.pop();
                    return Err(XsdRegexError::LiteralClassCloseAtHead);
                }
                let frame = self.classes.pop().expect("non-empty checked above");
                self.negated_wrap_close = frame.negated && !frame.subtracted;
                self.pos += 1;
                Ok(Token::ClassClose)
            }
            // A `.` is the XPath wildcard only outside a character class —
            // inside one it is always a literal dot, never rewritten.
            '.' if self.classes.is_empty() => {
                self.pos += 1;
                Ok(Token::Dot)
            }
            // XPath class subtraction `[...-[...]]` -> regex's own `--`
            // difference operator. Frame-tracked (not a one-shot flag) so it
            // fires correctly for a subtraction nested inside another
            // subtraction's right-hand `charClassExpr`. The emitter closes
            // the inner negated group before the difference operator, so `--`
            // subtracts from the complement; the scanner records that a
            // subtraction happened on the frame.
            '-' if !self.classes.is_empty() && self.peek(1) == Some('[') => {
                let frame = self
                    .classes
                    .last_mut()
                    .expect("non-empty checked by the guard");
                self.negated_wrap_close = frame.negated && !frame.subtracted;
                frame.subtracted = true;
                self.subtraction_operand = true;
                self.pos += 1;
                Ok(Token::Subtract)
            }
            // Inside a class `&` and `~` are ordinary members per
            // `XmlCharIncDash ::= [^\#x5B#x5D]`, but `regex-syntax` reads an
            // adjacent `&&` as set intersection and `~~` as symmetric
            // difference. Yield them as [`Token::ClassMember`] so the emitter
            // escapes each one; outside a class they are plain literals.
            '&' | '~' if !self.classes.is_empty() => {
                let member = self.chars[self.pos];
                self.pos += 1;
                Ok(Token::ClassMember(member))
            }
            // Capturing-parenthesis bookkeeping. XML Schema Part 2 Appendix G
            // (as amended by XPath F&O 3.1 §5.6.1.3): "a left parenthesis is
            // recognized as a capturing left parenthesis provided it is not
            // immediately followed by `?:`, is not within a character group
            // (square brackets), and is not escaped with a backslash".
            //
            // The grammar defines NO inline-flag, lookaround, comment, or
            // named-group syntax at all, so `(?:` is the ONLY `(?` spelling it
            // has. Anything else used to fall through to the literal path and
            // be compiled by `regex-syntax` with ITS semantics -- `(?i)`
            // applied Rust's `i` flag, `(?x)` smuggled `ignore_whitespace`
            // back in through the pattern body, `(?P<name>…)` became a named
            // capture -- and counted as a capturing group for the F&O
            // §5.6.1.4 back-reference tokenizer. It is refused here by name.
            '(' if self.classes.is_empty() => {
                if self.peek(1) == Some('?') {
                    if self.peek(2) == Some(':') {
                        self.pos += 3;
                        Ok(Token::GroupOpen { capturing: false })
                    } else {
                        let found: String = match self.peek(2) {
                            Some(c) => format!("(?{c}"),
                            None => "(?".to_owned(),
                        };
                        self.pos += if self.peek(2).is_some() { 3 } else { 2 };
                        Err(XsdRegexError::UnsupportedGroupConstruct { found })
                    }
                } else {
                    self.groups.open();
                    self.pos += 1;
                    Ok(Token::GroupOpen { capturing: true })
                }
            }
            ')' if self.classes.is_empty() => {
                self.groups.close();
                self.pos += 1;
                Ok(Token::GroupClose)
            }
            other => {
                self.pos += 1;
                Ok(Token::Literal(other))
            }
        }
    }

    fn scan_escape(&mut self) -> Result<Token, XsdRegexError> {
        let Some(esc) = self.peek(1) else {
            self.pos += 1;
            return Err(XsdRegexError::DanglingBackslash);
        };
        let token = match esc {
            'b' => Err(XsdRegexError::UnsupportedConstruct("\\b")),
            'B' => Err(XsdRegexError::UnsupportedConstruct("\\B")),
            'i' => Ok(Token::NameEscape {
                negated: false,
                chars: false,
            }),
            'I' => Ok(Token::NameEscape {
                negated: true,
                chars: false,
            }),
            'c' => Ok(Token::NameEscape {
                negated: false,
                chars: true,
            }),
            'C' => Ok(Token::NameEscape {
                negated: true,
                chars: true,
            }),
            's' => Ok(Token::SpaceEscape { negated: false }),
            'S' => Ok(Token::SpaceEscape { negated: true }),
            'w' => Ok(Token::WordEscape { negated: false }),
            'W' => Ok(Token::WordEscape { negated: true }),
            'p' | 'P' if self.peek(2) == Some('{') => return self.scan_unicode_property(),
            // `\d`/`\D` already default to `\p{Nd}`/its complement in
            // `regex-syntax`, exactly XSD's definition, so no rewrite is
            // needed.
            'd' | 'D' => Ok(Token::Escape(esc)),
            '1'..='9' => return self.scan_backreference(esc),
            // `\0` is NOT a back-reference: XPath F&O 3.1 §5.6.1.4's production
            // is `backReference ::= "\" [1-9][0-9]*`, which starts at 1. Nor is
            // it a `SingleCharEsc` — XML Schema Part 2 Appendix G enumerates
            // those, and `\0` is not among them. So it is simply not a
            // construct this dialect has. Rejected here by name, because letting
            // it fall through to the engine produced "backreferences are not
            // supported" — a message that is wrong about what the pattern
            // contains, and would send a reader looking for a capture group
            // that was never there.
            '0' => Err(XsdRegexError::UnsupportedNulEscape),
            // XML Schema Part 2 Appendix G's `SingleCharEsc` is a CLOSED
            // enumeration -- `n r t \ | . ? * + ( ) { } - [ ] ^` -- and
            // `regex-syntax`'s escape table is not the governing grammar.
            // `\p`/`\P` with a `{…}` name is handled above; a bare `\p`/`\P`
            // is not a construct and falls through to the named refusal.
            //
            // `\&`, `\~` and `\$` are NOT in `SingleCharEsc`, but they are
            // kept accepted deliberately: the first-party corpus pins the
            // escaped class members (`[a\&b]`, `[\~]`), and the vendored
            // ShExTest corpus pins `\$` as its literal-dollar spelling in
            // `1literalPattern_with_all_punctuation.shex` (the ShExC lexer
            // preserves `\$` verbatim). Refusing them would be over-refusal
            // hidden as grammar strictness.
            'n' | 'r' | 't' | '\\' | '|' | '.' | '?' | '*' | '+' | '(' | ')' | '{' | '}' | '-'
            | '[' | ']' | '^' | '&' | '~' | '$' => Ok(Token::Escape(esc)),
            // Everything else is decided by the GOVERNING grammar, never by
            // `regex-syntax`'s own escape table: `\A`/`\z`/`\Z`, `\x41`,
            // `\u{41}`, `\a`/`\f`/`\v`/`\e`, `\Q`, `\k<name>`, `\G`, `\h`,
            // `\N`, `\R`, `\X`, and a bare `\p`/`\P` are all constructs this
            // dialect does not have. Each is refused by name so the
            // diagnostic is the same kind as `\b`/`\B`/`\0`, rather than an
            // opaque engine error or a silently different language.
            _ => {
                self.pos += 2;
                return Err(XsdRegexError::UnsupportedEscape { escape: esc });
            }
        };
        self.pos += 2;
        token
    }

    /// Handle `\p{...}`/`\P{...}` with the cursor on the backslash and
    /// `self.peek(2) == Some('{')`.
    fn scan_unicode_property(&mut self) -> Result<Token, XsdRegexError> {
        let esc = self.chars[self.pos + 1];
        let name_start = self.pos + 3;
        let mut j = name_start;
        while self.chars.get(j).is_some_and(|&ch| ch != '}') {
            j += 1;
        }
        if self.chars.get(j) != Some(&'}') {
            self.pos = self.chars.len();
            return Err(XsdRegexError::UnterminatedBlockName { escape: esc });
        }
        let name: String = self.chars[name_start..j].iter().collect();
        self.pos = j + 1;
        Ok(Token::UnicodeProperty {
            negated: esc == 'P',
            name,
        })
    }

    /// Tokenize a back-reference per F&O §5.6.1.4 (see [`Groups`]): the first
    /// digit is always part of the reference, and each further digit joins it
    /// only while the resulting number still names a capturing group whose `(`
    /// precedes this point. NOT a greedy digit run — after one group, `\12` is
    /// `\1` then a literal `2`.
    fn scan_backreference(&mut self, first: char) -> Result<Token, XsdRegexError> {
        let start_digit = self.pos + 1;
        let mut number = first.to_digit(10).expect("matched 1..=9");
        let mut j = start_digit + 1;
        while let Some(digit) = self.chars.get(j).and_then(|c| c.to_digit(10)) {
            let Some(extended) = number.checked_mul(10).and_then(|n| n.checked_add(digit)) else {
                break;
            };
            if !self.groups.opened_before(extended) {
                break;
            }
            number = extended;
            j += 1;
        }
        let reference: String = std::iter::once('\\')
            .chain(self.chars[start_digit..j].iter().copied())
            .collect();
        self.pos = j;
        if !self.groups.closed_before(number) {
            // Invalid for ANY engine, backtracking or not: the reference
            // names a group that does not exist, or one whose `)` has not
            // been seen yet. Reported as malformed rather than as an
            // unsupported construct, because "this implementation cannot run
            // it" would be a misleading excuse for a pattern nothing can run.
            return Err(XsdRegexError::BadBackreference {
                reference,
                opened_groups: self.groups.opened,
            });
        }
        Ok(Token::Backreference(number))
    }
}

impl Iterator for Scanner<'_> {
    type Item = Result<Token, XsdRegexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.scan_next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Token::{
        Backreference, ClassClose, ClassMember, Dot, Escape, GroupClose, Literal, SpaceEscape,
        Subtract, WordEscape,
    };

    fn tokens(pattern: &str) -> Vec<Token> {
        Scanner::new(pattern)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("scan {pattern:?}: {e}"))
    }

    fn first_error(pattern: &str) -> XsdRegexError {
        Scanner::new(pattern)
            .find_map(Result::err)
            .unwrap_or_else(|| panic!("expected an error for {pattern:?}"))
    }

    fn class(negated: bool) -> Token {
        Token::ClassOpen { negated }
    }

    fn group(capturing: bool) -> Token {
        Token::GroupOpen { capturing }
    }

    fn name(negated: bool, chars: bool) -> Token {
        Token::NameEscape { negated, chars }
    }

    fn unicode(negated: bool, name: &str) -> Token {
        Token::UnicodeProperty {
            negated,
            name: name.to_owned(),
        }
    }

    #[test]
    fn literals_and_classes() {
        assert_eq!(
            tokens("abc"),
            vec![Literal('a'), Literal('b'), Literal('c')]
        );
        assert_eq!(
            tokens("[abc]"),
            vec![
                class(false),
                Literal('a'),
                Literal('b'),
                Literal('c'),
                ClassClose,
            ]
        );
        assert_eq!(tokens("[^a]"), vec![class(true), Literal('a'), ClassClose]);
        // `.` is the wildcard only outside a class; `-` is ordinary.
        assert_eq!(tokens("[.]"), vec![class(false), Literal('.'), ClassClose]);
        assert_eq!(tokens("."), vec![Dot]);
        assert_eq!(
            tokens("a-z"),
            vec![Literal('a'), Literal('-'), Literal('z')]
        );
    }

    #[test]
    fn class_subtraction_is_tokenized_frame_by_frame() {
        assert_eq!(
            tokens("[a-z-[aeiou]]"),
            vec![
                class(false),
                Literal('a'),
                Literal('-'),
                Literal('z'),
                Subtract,
                class(false),
                Literal('a'),
                Literal('e'),
                Literal('i'),
                Literal('o'),
                Literal('u'),
                ClassClose,
                ClassClose,
            ]
        );
        // A negated `posCharGroup` is a subtraction's left side just as readily.
        assert_eq!(
            tokens("[^0-9-[a-z]]"),
            vec![
                class(true),
                Literal('0'),
                Literal('-'),
                Literal('9'),
                Subtract,
                class(false),
                Literal('a'),
                Literal('-'),
                Literal('z'),
                ClassClose,
                ClassClose,
            ]
        );
    }

    /// The class interior is a recognizer of the XSD grammar, not a pass
    /// through to `regex-syntax`: `&`/`~` are ordinary members, a `[` that is
    /// not a subtraction operand and a `]` in a member position are malformed,
    /// and only the escaped spellings are literal brackets.
    #[test]
    fn class_interior_follows_the_xsd_grammar() {
        assert_eq!(
            tokens("[a&&b]"),
            vec![
                class(false),
                Literal('a'),
                ClassMember('&'),
                ClassMember('&'),
                Literal('b'),
                ClassClose,
            ]
        );
        assert_eq!(
            tokens("[a~~b]"),
            vec![
                class(false),
                Literal('a'),
                ClassMember('~'),
                ClassMember('~'),
                Literal('b'),
                ClassClose,
            ]
        );
        // Outside a class neither is special: both stay ordinary literals.
        assert_eq!(
            tokens("a&b"),
            vec![Literal('a'), Literal('&'), Literal('b')]
        );
        assert_eq!(
            tokens("a~b"),
            vec![Literal('a'), Literal('~'), Literal('b')]
        );

        // The subtraction operand `[a-z-[aeiou]]` is the one legitimate `[`
        // inside a class, and it still opens one.
        assert!(tokens("[a-z-[aeiou]]").contains(&Subtract));
        // ...but any other nested `[` is malformed XSD, not a class union.
        for pattern in ["[a[b]]", "[[:alpha:]]"] {
            let message = first_error(pattern).to_string();
            assert!(
                message.contains("unescaped '['"),
                "{pattern:?} must be rejected naming the '[': {message}"
            );
        }
        // A `]` cannot open a class's member list...
        for pattern in ["[]a]", "[^]a]"] {
            let message = first_error(pattern).to_string();
            assert!(
                message.contains("literal ']'"),
                "{pattern:?} must be rejected naming the ']': {message}"
            );
        }
        // ...and a bare `]` outside any class is not a `Char` either.
        let message = first_error("[a]]").to_string();
        assert!(
            message.contains("unescaped ']' outside"),
            "a bare ']' outside a class must be rejected naming it: {message}"
        );

        // The correctly escaped forms stay literal members.
        assert_eq!(
            tokens(r"[a\[b]"),
            vec![
                class(false),
                Literal('a'),
                Escape('['),
                Literal('b'),
                ClassClose,
            ]
        );
        assert_eq!(
            tokens(r"a\]b"),
            vec![Literal('a'), Escape(']'), Literal('b')]
        );
        assert_eq!(
            tokens(r"[a\&b]"),
            vec![
                class(false),
                Literal('a'),
                Escape('&'),
                Literal('b'),
                ClassClose,
            ]
        );
        assert_eq!(tokens(r"[\~]"), vec![class(false), Escape('~'), ClassClose]);
    }

    #[test]
    fn multi_character_escapes() {
        assert_eq!(
            tokens(r"\p{IsBasicLatin}"),
            vec![unicode(false, "IsBasicLatin")]
        );
        assert_eq!(
            tokens(r"\P{IsBasicLatin}"),
            vec![unicode(true, "IsBasicLatin")]
        );
        assert_eq!(
            tokens(r"\i\I\c\C"),
            vec![
                name(false, false),
                name(true, false),
                name(false, true),
                name(true, true),
            ]
        );
        assert_eq!(
            tokens(r"\s\S\w\W"),
            vec![
                SpaceEscape { negated: false },
                SpaceEscape { negated: true },
                WordEscape { negated: false },
                WordEscape { negated: true },
            ]
        );
        assert_eq!(
            tokens(r"\d\D\.\n"),
            vec![Escape('d'), Escape('D'), Escape('.'), Escape('n')]
        );
    }

    /// The scanner recognizes the SYNTACTIC form of every `\p{…}`/`\P{…}` name
    /// and leaves RESOLUTION to `emit`: whether a name is one of Appendix G's
    /// `IsCategory` names or an `Is` block is decided where the block table
    /// lives. So a script name still tokenizes here rather than failing, which
    /// is what keeps `ecma_262_divergences` -- another fold over this scanner
    /// -- able to report the construct instead of dying on it.
    #[test]
    fn non_category_property_names_are_tokenized_not_resolved() {
        assert_eq!(tokens(r"\p{Greek}"), vec![unicode(false, "Greek")]);
        assert_eq!(tokens(r"\P{sc=Greek}"), vec![unicode(true, "sc=Greek")]);
        // `concat!` keeps the `{Age:6.0}` property key out of a literal clippy
        // would read as a formatting argument.
        assert_eq!(
            tokens(concat!(r"\p{Age", ":6.0}")),
            vec![unicode(false, "Age:6.0")]
        );
        assert_eq!(tokens(r"\p{any}"), vec![unicode(false, "any")]);
    }

    /// The `(?` group arm accepts ONLY `(?:`; every other parenthesized
    /// construct is refused by name. XML Schema Part 2 Appendix G defines no
    /// inline-flag, lookaround, comment or named-group syntax, so before this
    /// `(?i)` applied Rust's `i`, `(?x)` smuggled `ignore_whitespace` in, and
    /// `(?<name>…)` became a named capture -- each announced only by the
    /// engine's own, different escape table.
    #[test]
    fn parenthesized_dialect_constructs_are_refused_by_name() {
        for (pattern, needle) in [
            (r"(?i)abc", "(?i"),
            (r"(?i:abc)", "(?i"),
            (r"(?x)a b", "(?x"),
            (r"(?s)a.b", "(?s"),
            (r"(?m)a", "(?m"),
            (r"(?U)a+", "(?U"),
            (r"(?u)a", "(?u"),
            (r"(?-i:a)", "(?-"),
            (r"(?=a)", "(?="),
            (r"(?!a)", "(?!"),
            (r"(?<=a)b", "(?<"),
            (r"(?<n>a)", "(?<"),
            (r"(?P<n>a)", "(?P"),
            (r"(?#c)a", "(?#"),
        ] {
            let message = first_error(pattern).to_string();
            assert!(
                message.contains(needle),
                "{pattern:?} must be refused naming {needle:?}: {message}"
            );
        }
        // The only `(?` spelling the grammar has still tokenizes as a group.
        assert_eq!(
            tokens("(?:ab)+"),
            vec![
                group(false),
                Literal('a'),
                Literal('b'),
                GroupClose,
                Literal('+'),
            ]
        );
        assert_eq!(tokens("(a)"), vec![group(true), Literal('a'), GroupClose]);
    }

    /// The `(` arm used to count `(?i)`, `(?=` and `(?<name>` as capturing
    /// groups, so the F&O §5.6.1.4 back-reference tokenizer saw two captures
    /// in `(?i)(a)\2` and reported a WELL-FORMED back-reference to group 2,
    /// when only one capturing group exists. Now the inline-flag construct is
    /// refused by name and opens no capture, so the `\2` is exposed as
    /// malformed. The scanner is resumable, so the late reference is reached
    /// after the earlier refusal.
    #[test]
    fn refused_parenthesized_constructs_do_not_corrupt_the_group_count() {
        let errors: Vec<XsdRegexError> =
            Scanner::new(r"(?i)(a)\2").filter_map(Result::err).collect();
        assert!(
            errors
                .iter()
                .all(|e| !matches!(e, XsdRegexError::Backreference(_))),
            "no well-formed back-reference may be reported: {errors:?}"
        );
        assert!(
            errors.iter().any(|e| e.to_string().contains("(?i")),
            "the inline-flag construct must be refused by name: {errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.to_string().contains("back-reference")),
            "the stray \\2 must be malformed, not a well-formed back-reference: {errors:?}"
        );
    }

    /// Every escape outside the closed `SingleCharEsc` enumeration (plus the
    /// module's multi-character escapes) is refused by name instead of being
    /// forwarded to `regex-syntax`'s own escape table.
    #[test]
    fn non_xsd_escapes_are_refused_by_name() {
        for escape in [
            r"\A", r"\z", r"\Z", r"\x", r"\u", r"\a", r"\f", r"\v", r"\e", r"\Q", r"\k", r"\G",
            r"\h", r"\N", r"\R", r"\X", r"\p", r"\P",
        ] {
            let pattern = format!("a{escape}b");
            let message = first_error(&pattern).to_string();
            assert!(
                message.contains(escape),
                "{pattern:?} must be refused naming {escape:?}: {message}"
            );
        }
        // The full spellings a reader actually writes are still named by
        // their prefix.
        for pattern in [r"\x41", r"\u{41}", r"\k<n>"] {
            let needle: String = pattern.chars().take(2).collect();
            let message = first_error(pattern).to_string();
            assert!(
                message.contains(&needle),
                "{pattern:?} must be refused naming {needle:?}: {message}"
            );
        }
        // ...but the whole `SingleCharEsc` enumeration still passes through,
        // and so do the corpus-pinned `\&`/`\~`/`\$`.
        assert_eq!(
            tokens(r"\n\r\t\\\|\.\?\*\+\{\}\(\)\-\[\]\^\&\~\$"),
            vec![
                Escape('n'),
                Escape('r'),
                Escape('t'),
                Escape('\\'),
                Escape('|'),
                Escape('.'),
                Escape('?'),
                Escape('*'),
                Escape('+'),
                Escape('{'),
                Escape('}'),
                Escape('('),
                Escape(')'),
                Escape('-'),
                Escape('['),
                Escape(']'),
                Escape('^'),
                Escape('&'),
                Escape('~'),
                Escape('$'),
            ]
        );
    }

    #[test]
    fn groups_and_backreferences() {
        assert_eq!(
            tokens("(?:a)"),
            vec![group(false), Literal('a'), GroupClose]
        );
        assert_eq!(tokens("(a)"), vec![group(true), Literal('a'), GroupClose]);
        // A parenthesis inside a class is a literal, never a group.
        assert_eq!(tokens("[(]"), vec![class(false), Literal('('), ClassClose]);
        assert_eq!(
            tokens("(a)\\1"),
            vec![group(true), Literal('a'), GroupClose, Backreference(1)]
        );
        // F&O §5.6.1.4: after one group `\12` is `\1` then a literal `2`.
        assert_eq!(
            tokens("(a)\\12"),
            vec![
                group(true),
                Literal('a'),
                GroupClose,
                Backreference(1),
                Literal('2'),
            ]
        );
    }

    #[test]
    fn errors_match_todays_rejections() {
        assert_eq!(
            first_error(r"a\bc"),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            first_error(r"[\B]"),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
        assert!(matches!(
            first_error(r"a\0b"),
            XsdRegexError::UnsupportedNulEscape
        ));
        assert!(matches!(
            first_error(r"\1"),
            XsdRegexError::BadBackreference { .. }
        ));
        assert!(matches!(
            first_error(r"(a\1)"),
            XsdRegexError::BadBackreference { .. }
        ));
        assert_eq!(first_error("a\\"), XsdRegexError::DanglingBackslash);
        assert!(matches!(
            first_error("[abc"),
            XsdRegexError::UnterminatedCharacterClass
        ));
        assert!(matches!(
            first_error(r"\p{IsBasicLatin"),
            XsdRegexError::UnterminatedBlockName { .. }
        ));
    }

    #[test]
    fn class_state_and_source_spans_survive_a_walk() {
        let mut scanner = Scanner::new("[abc]");
        assert!(!scanner.in_class());
        assert_eq!(scanner.next(), Some(Ok(class(false))));
        assert!(scanner.in_class());
        assert_eq!(scanner.source_text(), "[");
        assert_eq!(scanner.next(), Some(Ok(Literal('a'))));
        assert_eq!(scanner.source_text(), "a");
        for expected in [Literal('b'), Literal('c'), ClassClose] {
            assert_eq!(scanner.next(), Some(Ok(expected)));
        }
        assert!(!scanner.in_class());
        assert_eq!(scanner.next(), None);

        let mut scanner = Scanner::new(r"[a-z]+\p{IsBasicLatin}");
        let mut spans = Vec::new();
        loop {
            if scanner.next().is_none() {
                break;
            }
            spans.push(scanner.source_text());
        }
        assert_eq!(
            spans,
            vec!["[", "a", "-", "z", "]", "+", r"\p{IsBasicLatin}"]
        );
    }
}

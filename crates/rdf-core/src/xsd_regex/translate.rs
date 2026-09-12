// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The construct-by-construct XSD/XPath `regExp` -> `regex`-crate translator,
//! plus the `x`-flag whitespace stripper that runs ahead of it (see
//! [`super`]'s module doc for the full construct table this implements).
//!
//! [`translate`] is a single left-to-right scan over the pattern's `char`s
//! that tracks character-class nesting depth (`[`...`]`, which nests through
//! XSD's `charClassSub` subtraction, e.g. `[a-z-[aeiou]]`) so every
//! class-aware rewrite fires correctly whether the construct appears inside
//! or outside a class. Every multi-character-escape rewrite below (`\i \I \c
//! \C \s \S \w \W`, and block escapes `\p{Is…}`/`\P{Is…}`) is emitted as a
//! **self-contained bracket expression** (`[...]`/`[^...]`) rather than a
//! bare, un-bracketed splice: `regex-syntax` supports nested character-class
//! union (`[a[b-c]d]` is valid and means the same as `[abcd]` restricted to
//! `[b-c]`'s member codepoints), so a self-contained bracket expression
//! composes correctly by ordinary set union whether it stands alone as its
//! own atom or is nested inside an already-open class -- one rewrite rule
//! for both contexts, rather than two (verified directly against `regex`
//! 1.13: `Regex::new("[a[b-c]d]")`, `Regex::new("[[^\\u{370}-\\u{3ff}]a]")`
//! and similar nested-negation forms all compile and match as expected).

use std::fmt::Write as _;

use crate::blank_label::{PN_CHARS_BASE_RANGES, PN_CHARS_EXTRA_RANGES};

use super::blocks;
use super::error::XsdRegexError;

/// Apply the XPath `x` flag to a `sh:pattern`/`REGEX`/`PATTERN` source,
/// textually, before the pattern is translated or parsed.
///
/// SHACL §4.5.3 defines `sh:pattern` by the SPARQL `REGEX` function, and
/// SPARQL 1.1 §17.4.3.14 defines `REGEX` as an invocation of XPath
/// `fn:matches`, so the `x` flag is XPath's. *XPath and XQuery Functions and
/// Operators 3.1* §5.6.2 defines it in one sentence:
///
/// > `x`: If present, whitespace characters (`#x9`, `#xA`, `#xD` and `#x20`)
/// > in the regular expression are removed prior to matching with one
/// > exception: whitespace characters within character class expressions
/// > (`charClassExpr`) are not removed. This flag can be used, for example,
/// > to break up long regular expressions into readable lines.
///
/// and pins it with four examples, every one of which is tested against in
/// [`tests::xpath_x_flag_matches_the_specifications_examples`]:
///
/// > `fn:matches("helloworld", "hello world", "x")` returns `true()`
/// >
/// > `fn:matches("helloworld", "hello[ ]world", "x")` returns `false()`
/// >
/// > `fn:matches("hello world", "hello\ sworld", "x")` returns `true()`
/// >
/// > `fn:matches("hello world", "hello world", "x")` returns `false()`
///
/// # Why the removal is done here and not by `RegexBuilder::ignore_whitespace`
///
/// Rust's verbose mode is a different production wearing the same letter,
/// and it is wrong in three ways at once:
///
/// * it removes every code point with the Unicode `White_Space` property —
///   twenty-six, including U+00A0 NO-BREAK SPACE and U+3000 — where XPath
///   names exactly four, so a pattern matching a literal IDEOGRAPHIC SPACE
///   silently stops matching it;
/// * it treats `#` as a comment introducer running to end of line, so
///   `"a#b c"` compiles to `a` where XPath compiles it to `a#bc`. XPath regex
///   has no comment syntax at all;
/// * it removes whitespace **inside** character classes, which is the one
///   case XPath explicitly exempts — the specification's second example
///   exists for exactly this, and `ignore_whitespace` gets it backwards.
///
/// # The backslash does not protect whitespace, and the third example is why
///
/// Removal is textual and happens *prior to parsing*, so a backslash does
/// not escape a following space out of it: the source `hello\ sworld` has
/// its space removed, yielding `hello\sworld`, and the specification's third
/// example requires that to match `"hello world"`. The backslash is still
/// tracked here, because it decides whether a `[` or `]` **delimits** a
/// character class — `\[` opens nothing and `\]` closes nothing — and
/// getting that wrong would move the exempt region. Note that a removed
/// whitespace character does not consume the pending escape: it re-binds to
/// whatever follows, which is what turns `\ s` into `\s`.
///
/// `charClassExpr` nests, through `charClassSub` (`[a-z-[aeiou]]`), so the
/// exempt region is tracked by depth rather than by a single flag. An
/// unterminated `[` leaves the rest of the pattern exempt and then fails to
/// compile, which is a named error rather than a silently different
/// pattern.
pub(super) fn strip_x_flag_whitespace(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut class_depth = 0_usize;
    let mut escaped = false;
    for c in pattern.chars() {
        if escaped {
            // Inside a character class nothing is removed; outside one, even
            // an escaped whitespace character goes, and the escape carries
            // over to the next character (`\ s` becomes `\s`).
            if class_depth == 0 && purrdf_iri::terminals::is_ws_char(c) {
                continue;
            }
            out.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' => {
                out.push(c);
                escaped = true;
            }
            '[' => {
                class_depth += 1;
                out.push(c);
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                out.push(c);
            }
            _ if class_depth == 0 && purrdf_iri::terminals::is_ws_char(c) => {}
            _ => out.push(c),
        }
    }
    out
}

/// Push a single inclusive codepoint range as a `regex`-crate bracket-class
/// member: `\u{lo}` for a one-codepoint range, `\u{lo}-\u{hi}` otherwise.
fn push_hex_range(out: &mut String, lo: u32, hi: u32) {
    if lo == hi {
        write!(out, "\\u{{{lo:x}}}").expect("writing to a String cannot fail");
    } else {
        write!(out, "\\u{{{lo:x}}}-\\u{{{hi:x}}}").expect("writing to a String cannot fail");
    }
}

/// Push every range in `ranges` as bracket-class members (see
/// [`push_hex_range`]).
fn push_ranges(out: &mut String, ranges: &[(u32, u32)]) {
    for &(lo, hi) in ranges {
        push_hex_range(out, lo, hi);
    }
}

/// The body (without enclosing `[`/`]`) of the `\i`/`\I` bracket expression:
/// XML 1.0 `NameStartChar`, minus `':'` and `'_'` per
/// [`PN_CHARS_BASE_RANGES`]'s own definition, with both of those two
/// characters added back explicitly (XSD's `\i` is `NameStartChar` in full,
/// unlike Turtle's `PN_CHARS_BASE`).
fn name_start_class_body() -> String {
    let mut body = String::new();
    push_ranges(&mut body, PN_CHARS_BASE_RANGES);
    body.push_str(":_");
    body
}

/// The body (without enclosing `[`/`]`) of the `\c`/`\C` bracket expression:
/// [`name_start_class_body`]'s set plus XML 1.0 `NameChar`'s extra ranges
/// (`PN_CHARS_EXTRA_RANGES`) plus `'-'`, `'.'`, and `[0-9]`.
fn name_char_class_body() -> String {
    let mut body = name_start_class_body();
    push_ranges(&mut body, PN_CHARS_EXTRA_RANGES);
    // `-` must be escaped inside a class (it would otherwise open a range
    // with whatever precedes it); `.` is always literal inside a class.
    body.push_str("\\-.0-9");
    body
}

/// Translate an XSD/XPath `regExp` pattern body into `regex`-crate syntax.
///
/// `dot_all` is the caller's `s` flag: when set, an unescaped `.` outside a
/// character class is left untouched (the caller applies
/// `RegexBuilder::dot_matches_new_line(true)` instead); when clear, `.` is
/// rewritten per XPath F&O 3.1 §5.6.2 (excludes both `#xA` and `#xD`, not
/// only `#xA` as Rust's own default does).
///
/// Must be called AFTER [`strip_x_flag_whitespace`] when the `x` flag is
/// set — the two operate on the same raw pattern text, and `x`'s removal
/// has to happen first (its whitespace-exemption tracking is textual, not
/// aware of any of the rewrites below).
pub(super) fn translate(pattern: &str, dot_all: bool) -> Result<String, XsdRegexError> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut out = String::with_capacity(pattern.len() + 16);
    // One frame per open `[`. A plain depth counter is not enough: a negated
    // group has to be wrapped so a following subtraction binds to the
    // negation rather than around it (see `ClassFrame`).
    let mut classes: Vec<ClassFrame> = Vec::new();
    // Capturing-group bookkeeping, needed only to TOKENIZE a back-reference
    // correctly — see `Groups` and `translate_escape`'s digit arm.
    let mut groups = Groups::default();
    let mut i = 0_usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                i += 1;
                let Some(&esc) = chars.get(i) else {
                    return Err(XsdRegexError::Malformed(
                        "pattern ends with a dangling backslash".to_owned(),
                    ));
                };
                translate_escape(&chars, &mut i, &mut out, esc, &groups)?;
            }
            '[' => {
                i += 1;
                let negated = chars.get(i) == Some(&'^');
                if negated {
                    // XSD `[^g-e]` is (complement of `g`) minus `e`, but
                    // Rust's `[^g--e]` applies `^` to the WHOLE class
                    // expression — i.e. the complement of (`g` minus `e`),
                    // a different set. Opening an extra bracket and putting
                    // the negation on the inner one restores XSD's binding:
                    // `[[^g]--e]`. Emitted for every negated group, not only
                    // the ones a subtraction follows, because the wrap is
                    // semantically free when it does not (`[[^g]]` == `[^g]`)
                    // and deciding otherwise would need a lookahead over the
                    // whole group.
                    out.push_str("[[^");
                    i += 1;
                } else {
                    out.push('[');
                }
                classes.push(ClassFrame {
                    negated,
                    subtracted: false,
                });
            }
            ']' if !classes.is_empty() => {
                let frame = classes.pop().expect("non-empty checked by the guard");
                // The inner negated group's own `]` was already emitted by
                // the subtraction arm below; only an unsubtracted negated
                // group still owes one here.
                if frame.negated && !frame.subtracted {
                    out.push(']');
                }
                out.push(']');
                i += 1;
            }
            // XPath class subtraction `[...-[...]]` -> regex's own `--`
            // difference operator. Frame-tracked (not a one-shot flag) so it
            // fires correctly for a subtraction nested inside another
            // subtraction's right-hand `charClassExpr`.
            '-' if !classes.is_empty() && chars.get(i + 1) == Some(&'[') => {
                let frame = classes.last_mut().expect("non-empty checked by the guard");
                if frame.negated && !frame.subtracted {
                    // Close the inner negated group before the difference
                    // operator, so `--` subtracts from the complement.
                    out.push(']');
                }
                frame.subtracted = true;
                out.push_str("--");
                i += 1;
            }
            // A `.` is the XPath wildcard only outside a character class —
            // inside one it is always a literal dot, never rewritten.
            '.' if classes.is_empty() && !dot_all => {
                out.push_str("[^\\u{a}\\u{d}]");
                i += 1;
            }
            // Capturing-parenthesis bookkeeping. XML Schema Part 2 Appendix G
            // (as amended by XPath F&O 3.1 §5.6.1.3): "a left parenthesis is
            // recognized as a capturing left parenthesis provided it is not
            // immediately followed by `?:`, is not within a character group
            // (square brackets), and is not escaped with a backslash" — the
            // escaped case never reaches here, having been consumed above.
            '(' if classes.is_empty() => {
                if chars.get(i + 1) == Some(&'?') && chars.get(i + 2) == Some(&':') {
                    out.push_str("(?:");
                    i += 3;
                } else {
                    groups.open();
                    out.push('(');
                    i += 1;
                }
            }
            ')' if classes.is_empty() => {
                groups.close();
                out.push(')');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    if !classes.is_empty() {
        return Err(XsdRegexError::Malformed(
            "unterminated character class (missing closing ']')".to_owned(),
        ));
    }
    Ok(out)
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
}

/// Handle one `\`-escaped construct, starting at `chars[*i] == esc`.
/// Advances `*i` past everything it consumes and appends the translated
/// text to `out`. Called for a `\`-escape in ANY context (in or out of a
/// character class) — `\b`/`\B` and backreferences are rejected in both
/// contexts per the XSD/XPath grammar (neither is defined by it at all), and
/// every other rewrite composes correctly via nested bracket-class union
/// (see the module doc) regardless of context, so there is no class-depth
/// branch here.
fn translate_escape(
    chars: &[char],
    i: &mut usize,
    out: &mut String,
    esc: char,
    groups: &Groups,
) -> Result<(), XsdRegexError> {
    match esc {
        'b' => return Err(XsdRegexError::UnsupportedConstruct("\\b")),
        'B' => return Err(XsdRegexError::UnsupportedConstruct("\\B")),
        'i' => {
            out.push('[');
            out.push_str(&name_start_class_body());
            out.push(']');
            *i += 1;
        }
        'I' => {
            out.push_str("[^");
            out.push_str(&name_start_class_body());
            out.push(']');
            *i += 1;
        }
        'c' => {
            out.push('[');
            out.push_str(&name_char_class_body());
            out.push(']');
            *i += 1;
        }
        'C' => {
            out.push_str("[^");
            out.push_str(&name_char_class_body());
            out.push(']');
            *i += 1;
        }
        // XSD `\s ::= [#x20\t\n\r]`, exactly 4 codepoints — Rust's own `\s`
        // is the full Unicode `White_Space` property (26 codepoints
        // including U+00A0/U+3000), which is too wide (§3 of the governing
        // plan).
        's' => {
            out.push_str("[\\u{9}\\u{a}\\u{d}\\u{20}]");
            *i += 1;
        }
        'S' => {
            out.push_str("[^\\u{9}\\u{a}\\u{d}\\u{20}]");
            *i += 1;
        }
        // XSD `\w ::= [^\p{P}\p{Z}\p{C}]` — Rust's own `\w` is Perl's
        // alphanumeric-plus-underscore-plus-combining-marks class, a
        // different (and differently-shaped) set.
        'w' => {
            out.push_str("[^\\p{P}\\p{Z}\\p{C}]");
            *i += 1;
        }
        'W' => {
            out.push_str("[\\p{P}\\p{Z}\\p{C}]");
            *i += 1;
        }
        // `\d`/`\D` already default to `\p{Nd}`/its complement in
        // `regex-syntax`, exactly XSD's definition — confirmed by direct
        // testing (`Regex::new(r"^\d$").unwrap().is_match("\u{0660}")`,
        // U+0660 ARABIC-INDIC DIGIT ZERO, is `true`). No rewrite needed.
        'd' | 'D' => {
            out.push('\\');
            out.push(esc);
            *i += 1;
        }
        'p' | 'P' => {
            *i += 1;
            translate_unicode_property(chars, i, out, esc)?;
        }
        '1'..='9' => {
            // Tokenize per F&O §5.6.1.4 (see `Groups`): the first digit is
            // always part of the reference, and each further digit joins it
            // only while the resulting number still names a capturing group
            // whose `(` precedes this point. NOT a greedy digit run — after
            // one group, `\12` is `\1` then a literal `2`.
            let start = *i;
            let mut number = esc.to_digit(10).expect("matched 1..=9");
            let mut j = *i + 1;
            while let Some(digit) = chars.get(j).and_then(|c| c.to_digit(10)) {
                let Some(extended) = number.checked_mul(10).and_then(|n| n.checked_add(digit))
                else {
                    break;
                };
                if !groups.opened_before(extended) {
                    break;
                }
                number = extended;
                j += 1;
            }
            let mut reference = String::from("\\");
            reference.extend(&chars[start..j]);
            *i = j;
            if !groups.closed_before(number) {
                // Invalid for ANY engine, backtracking or not: the reference
                // names a group that does not exist, or one whose `)` has not
                // been seen yet. Reported as malformed rather than as an
                // unsupported construct, because "this implementation cannot
                // run it" would be a misleading excuse for a pattern nothing
                // can run.
                return Err(XsdRegexError::Malformed(format!(
                    "back-reference {reference} refers to a capturing group that does not \
                     exist, or whose closing ')' comes after it ({} capturing group(s) are \
                     complete at that point)",
                    groups.opened
                )));
            }
            return Err(XsdRegexError::Backreference(reference));
        }
        // `\0` is NOT a back-reference: XPath F&O 3.1 §5.6.1.4's production is
        // `backReference ::= "\" [1-9][0-9]*`, which starts at 1. Nor is it a
        // `SingleCharEsc` — XML Schema Part 2 Appendix G enumerates those, and
        // `\0` is not among them. So it is simply not a construct this dialect
        // has. Rejected here by name, because letting it fall through to the
        // engine produced "backreferences are not supported" — a message that
        // is wrong about what the pattern contains, and would send a reader
        // looking for a capture group that was never there.
        '0' => {
            return Err(XsdRegexError::Malformed(
                "\\0 is not a construct of the XSD/XPath regular-expression grammar (a \
                 back-reference starts at \\1, and \\0 is not a single-character escape)"
                    .to_owned(),
            ));
        }
        _ => {
            out.push('\\');
            out.push(esc);
            *i += 1;
        }
    }
    Ok(())
}

/// Whether `lo..=hi` lies wholly inside the UTF-16 surrogate range
/// `U+D800..=U+DFFF`, whose code points are not Unicode scalar values.
///
/// Tested as a range rather than by block name so it stays correct if a future
/// Unicode revision renames or re-partitions the three surrogate blocks: the
/// property that matters is where the code points are, not what they are
/// called.
fn is_surrogate_range(lo: u32, hi: u32) -> bool {
    lo >= 0xD800 && hi <= 0xDFFF
}

/// Handle `\p{...}`/`\P{...}` starting just past the `p`/`P`
/// (`chars[*i]` is either `{` or the construct is not brace form at all).
fn translate_unicode_property(
    chars: &[char],
    i: &mut usize,
    out: &mut String,
    esc: char,
) -> Result<(), XsdRegexError> {
    if chars.get(*i) != Some(&'{') {
        // Not the `\p{...}` brace form at all (malformed, or a single-letter
        // shorthand this dialect does not define) — pass through unchanged
        // and let the underlying engine accept or reject it; this module
        // only owns the `Is`-prefixed block-escape form.
        out.push('\\');
        out.push(esc);
        return Ok(());
    }
    *i += 1; // consume '{'
    let name_start = *i;
    while chars.get(*i).is_some_and(|&ch| ch != '}') {
        *i += 1;
    }
    if chars.get(*i) != Some(&'}') {
        return Err(XsdRegexError::Malformed(format!(
            "unterminated \\{esc}{{ block-escape name (no matching '}}')"
        )));
    }
    let name: String = chars[name_start..*i].iter().collect();
    *i += 1; // consume '}'

    if name.starts_with("Is") {
        let (lo, hi) =
            blocks::lookup(&name).ok_or_else(|| XsdRegexError::UnknownBlock(name.clone()))?;
        if is_surrogate_range(lo, hi) {
            // Three of Unicode's blocks — High Surrogates, High Private Use
            // Surrogates and Low Surrogates — consist entirely of code points
            // that are NOT Unicode scalar values. A Rust `&str` is UTF-8 over
            // scalar values, so no subject string this engine can be handed
            // will ever contain one.
            //
            // "Matches nothing" is therefore the EXACT answer over the domain,
            // not a degraded approximation of one, and `\P{Is…}` of such a
            // block matches every representable character for the same reason.
            // Emitting the range literally is what does not work: `regex`
            // refuses `\u{d800}` outright, so the caller got an opaque
            // "hexadecimal literal is not a Unicode scalar value" parse error
            // naming neither the block nor the reason.
            //
            // Refusing the pattern instead would be over-refusal: the escape
            // is well-formed XSD naming a block that genuinely exists, and
            // over-refusal is the direction this dialect work is least willing
            // to move in. Both forms are self-contained bracket expressions, so
            // they compose by union inside an enclosing class exactly as every
            // other block escape does.
            out.push_str(if esc == 'P' { "[\\s\\S]" } else { "[^\\s\\S]" });
        } else {
            out.push('[');
            if esc == 'P' {
                out.push('^');
            }
            push_hex_range(out, lo, hi);
            out.push(']');
        }
    } else {
        // A general-category escape (`\p{L}`, `\p{Nd}`, ...) is legitimately
        // shared syntax between XSD and `regex-syntax` — never intercepted.
        out.push('\\');
        out.push(esc);
        out.push('{');
        out.push_str(&name);
        out.push('}');
    }
    Ok(())
}

/// Scan a pattern for constructs that do **not** carry the same meaning in
/// ECMA-262 — the dialect JSON Schema's `pattern`, JavaScript, and most
/// `pattern` consumers use.
///
/// Class-awareness mirrors [`translate`]'s own decisions exactly (a `.` or a
/// `-[` inside a character class is not the metacharacter), so a construct is
/// reported here if and only if translation actually rewrote it. Names are
/// returned in first-appearance order with duplicates collapsed, so the
/// caller's message is deterministic.
///
/// This is emphatically **not** a validity check: every construct named here
/// is a perfectly well-formed `sh:pattern`. It answers the narrower question
/// an emitter has to ask before it copies the source text into a different
/// dialect's slot.
pub(super) fn ecma_262_divergences(pattern: &str) -> Vec<&'static str> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut found: Vec<&'static str> = Vec::new();
    let mut note = |name: &'static str| {
        if !found.contains(&name) {
            found.push(name);
        }
    };
    let mut class_depth: usize = 0;
    let mut i = 0_usize;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                let Some(&esc) = chars.get(i) else { break };
                match esc {
                    // ECMA-262 has no such escape. In its non-Unicode mode
                    // `\i` is even an *identity escape* — a literal `i` —
                    // so copying the text across is silently wrong, not an
                    // error the consumer would report.
                    'i' | 'I' | 'c' | 'C' => note("\\i/\\I/\\c/\\C"),
                    // XSD `\s` is four code points; ECMA-262's adds vertical
                    // tab, form feed, NBSP, BOM and the Unicode space
                    // separators.
                    's' | 'S' => note("\\s/\\S"),
                    // XSD `\w` is `[^\p{P}\p{Z}\p{C}]`; ECMA-262's is
                    // `[A-Za-z0-9_]`.
                    'w' | 'W' => note("\\w/\\W"),
                    // XSD `\d` is `\p{Nd}` (every Unicode decimal digit);
                    // ECMA-262's is `[0-9]`.
                    'd' | 'D' => note("\\d/\\D"),
                    'p' | 'P' => {
                        // Only the `Is`-prefixed BLOCK form diverges; a
                        // general-category escape is shared syntax (though
                        // ECMA-262 needs its `u` flag for it, which JSON
                        // Schema's flagless `pattern` cannot set — recorded
                        // through the flag path, not here).
                        let mut j = i + 1;
                        if chars.get(j) == Some(&'{') {
                            j += 1;
                            let start = j;
                            while chars.get(j).is_some_and(|&ch| ch != '}') {
                                j += 1;
                            }
                            let name: String = chars[start..j].iter().collect();
                            if name.starts_with("Is") {
                                note("\\p{Is…} block escape");
                            }
                            i = j;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            '[' => {
                class_depth += 1;
                i += 1;
                if chars.get(i) == Some(&'^') {
                    i += 1;
                }
            }
            ']' if class_depth > 0 => {
                class_depth -= 1;
                i += 1;
            }
            '-' if class_depth > 0 && chars.get(i + 1) == Some(&'[') => {
                // ECMA-262 has no class subtraction: `[a-z-[aeiou]]` parses
                // there as the class `a-z`, `-`, `[`, `aeiou` followed by a
                // stray `]` — a different language, accepted without complaint.
                note("[…-[…]] class subtraction");
                i += 1;
            }
            '.' if class_depth == 0 => {
                // XSD's `.` excludes #x0A and #x0D; ECMA-262's also excludes
                // U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR. A
                // narrow divergence, but a real one.
                note(". wildcard");
                i += 1;
            }
            _ => i += 1,
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blank_label::{is_pn_chars, is_pn_chars_u};

    fn translated_regex(pattern: &str, dot_all: bool) -> regex::Regex {
        let source = translate(pattern, dot_all).expect("translate");
        regex::Regex::new(&source).unwrap_or_else(|e| panic!("compile {source:?}: {e}"))
    }

    #[test]
    fn class_subtraction_is_rewritten() {
        let re = translated_regex("^[a-z-[aeiou]]+$", false);
        assert!(re.is_match("bcd"));
        assert!(!re.is_match("bad"));
    }

    /// XSD's `charClassSub` takes a `negCharGroup` on its left as readily as
    /// a `posCharGroup`, and the two dialects bind the negation on opposite
    /// sides of the difference: `[^0-9-[a-z]]` is XSD's (¬digits) ∖ (a–z),
    /// while Rust's `[^0-9--[a-z]]` is ¬(digits ∖ a–z) — which is just
    /// ¬digits, and therefore matches every letter. Asserted on the emitted
    /// source as well as the behaviour, because the bug was one bracket.
    #[test]
    fn negated_class_subtraction_binds_to_the_negation() {
        assert_eq!(
            translate("[^0-9-[a-z]]", false).expect("translate"),
            "[[^0-9]--[a-z]]"
        );
        let re = translated_regex("^[^0-9-[a-z]]+$", false);
        assert!(re.is_match("ABC"), "upper case is in neither subtrahend");
        assert!(!re.is_match("a"), "a-z is subtracted from the complement");
        assert!(!re.is_match("5"), "digits are excluded by the negation");

        // The wrap is emitted for every negated group, subtraction or not,
        // and `[[^x]]` means exactly what `[^x]` means.
        assert_eq!(translate("[^abc]", false).expect("translate"), "[[^abc]]");
        let plain = translated_regex("^[^abc]$", false);
        assert!(plain.is_match("d"));
        assert!(!plain.is_match("a"));

        // A negated subtrahend nests the same way.
        let nested = translated_regex("^[a-z-[^aeiou]]+$", false);
        assert!(nested.is_match("aei"), "a-z intersected with the vowels");
        assert!(!nested.is_match("b"));
    }

    /// A back-reference's digits are tokenized against the capturing-group
    /// count that precedes it (F&O §5.6.1.4), not by a greedy digit run. The
    /// whole contract for a back-reference here is an error that names the
    /// construct, so a wrong tokenization is a message that lies about the
    /// pattern.
    #[test]
    fn backreference_tokenization_follows_the_preceding_group_count() {
        let named = |pattern: &str| match translate(pattern, false) {
            Err(XsdRegexError::Backreference(reference)) => reference,
            other => panic!("expected a back-reference for {pattern:?}, got {other:?}"),
        };
        // One group: `\12` is `\1` followed by a literal `2`.
        assert_eq!(named(r"(a)\12"), r"\1");
        // Ten groups: the second digit joins.
        assert_eq!(named(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\10"), r"\10");
        // Twelve groups: so does a `\12`.
        assert_eq!(named(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)\12"), r"\12");
        // A non-capturing group is not counted...
        assert_eq!(named(r"(?:a)(b)\1"), r"\1");
        // ...nor is a parenthesis inside a character class, or an escaped one.
        assert_eq!(named(r"[()](a)\1"), r"\1");
        assert_eq!(named(r"\((a)\1"), r"\1");

        // A reference to a group that does not exist, or whose `)` has not
        // been seen, is MALFORMED — no engine can run it — which is a
        // different answer from one this engine declines to execute.
        for pattern in [r"a\9b", r"(a\1)", r"(?:a)\1"] {
            match translate(pattern, false) {
                Err(XsdRegexError::Malformed(message)) => {
                    assert!(
                        message.contains("does not exist") || message.contains("closing"),
                        "{pattern:?}: {message}"
                    );
                }
                other => panic!("expected malformed for {pattern:?}, got {other:?}"),
            }
        }
    }

    /// The three surrogate blocks hold no Unicode scalar values, so no `&str`
    /// subject can contain one. Emitting the range literally made the engine
    /// refuse `\u{d800}` with a message naming neither the block nor the
    /// reason; the exact answer is the empty set (and its complement).
    #[test]
    fn surrogate_blocks_are_the_empty_set_not_a_parse_error() {
        assert_eq!(
            translate(r"\p{IsHighSurrogates}", false).expect("translate"),
            "[^\\s\\S]"
        );
        assert_eq!(
            translate(r"\P{IsLowSurrogates}", false).expect("translate"),
            "[\\s\\S]"
        );
        let empty = translated_regex(r"^\p{IsHighSurrogates}$", false);
        assert!(!empty.is_match("a"));
        assert!(!empty.is_match("\u{10000}"));
        let all = translated_regex(r"^\P{IsHighSurrogates}$", false);
        assert!(all.is_match("a"));
        assert!(all.is_match("\n"), "the complement is every scalar value");

        // Composes by union inside an enclosing class like any other block.
        let union = translated_regex(r"^[\p{IsHighSurrogates}\p{IsBasicLatin}]+$", false);
        assert!(union.is_match("abc"));
        assert!(!union.is_match("\u{3b1}"));

        // A neighbouring non-surrogate block is unaffected by the carve-out.
        let latin = translated_regex(r"^\p{IsBasicLatin}$", false);
        assert!(latin.is_match("A"));
        assert!(!latin.is_match("\u{e9}"));
    }

    /// `\0` is neither a back-reference (XPath F&O §5.6.1.4's production
    /// starts at `\1`) nor a `SingleCharEsc` (XML Schema Appendix G
    /// enumerates those). Letting it reach the engine produced
    /// "backreferences are not supported", which is wrong about what the
    /// pattern contains.
    #[test]
    fn nul_escape_is_refused_as_itself_not_as_a_backreference() {
        let err = translate("^a\\0b$", false).expect_err("\\0 is not a construct");
        let message = err.to_string();
        assert!(
            message.contains("\\0"),
            "the message must name the construct: {message}"
        );
        assert!(
            !message.contains("backreference"),
            "\\0 is not a back-reference, and the message must not say it is: {message}"
        );
        // A real back-reference still reports as one.
        let back = translate("(a)\\1", false).expect_err("no backreferences");
        assert!(back.to_string().contains("backreference"));
    }

    #[test]
    fn nested_class_subtraction_is_rewritten() {
        // A subtraction inside a subtraction's own right-hand side.
        let re = translated_regex("^[a-z-[a-c-[b]]]+$", false);
        // Right-hand side `[a-c-[b]]` = {a, c}; overall = a-z minus {a, c}.
        assert!(re.is_match("d"));
        assert!(!re.is_match("a"));
        assert!(!re.is_match("c"));
    }

    #[test]
    fn dot_excludes_lf_and_cr_outside_class() {
        let re = translated_regex("^a.b$", false);
        assert!(re.is_match("axb"));
        assert!(!re.is_match("a\nb"));
        assert!(!re.is_match("a\rb"));
    }

    #[test]
    fn dot_is_literal_inside_class() {
        let re = translated_regex("^[.]$", false);
        assert!(re.is_match("."));
        assert!(!re.is_match("a"));
    }

    #[test]
    fn dot_all_flag_leaves_dot_untouched_for_builder() {
        // `dot_all = true` means the translator must NOT rewrite `.`; the
        // caller applies `dot_matches_new_line` at the builder instead.
        let source = translate("^a.b$", true).expect("translate");
        assert_eq!(source, "^a.b$");
    }

    #[test]
    fn i_and_ii_escapes_standalone() {
        let re = translated_regex(r"^\i+$", false);
        assert!(re.is_match("abc"));
        assert!(re.is_match(":a_b"));
        assert!(!re.is_match("1abc"));
        let re_neg = translated_regex(r"^\I$", false);
        assert!(re_neg.is_match("1"));
        assert!(!re_neg.is_match("a"));
    }

    #[test]
    fn i_escape_splices_inside_an_open_class() {
        let re = translated_regex(r"^[\i0-9]+$", false);
        assert!(re.is_match("a1:_9"));
        assert!(!re.is_match("a!b"));
    }

    #[test]
    fn c_and_cc_escapes() {
        let re = translated_regex(r"^\c+$", false);
        assert!(re.is_match("a-1._:"));
        let re_neg = translated_regex(r"^\C$", false);
        assert!(re_neg.is_match("!"));
        assert!(!re_neg.is_match("a"));
    }

    #[test]
    fn s_escape_is_the_four_codepoint_class_not_unicode_whitespace() {
        let re = translated_regex(r"^\s$", false);
        assert!(re.is_match(" "));
        assert!(re.is_match("\t"));
        assert!(re.is_match("\n"));
        assert!(re.is_match("\r"));
        assert!(!re.is_match("\u{A0}")); // NBSP: Unicode whitespace, not XSD's
        assert!(!re.is_match("\u{3000}")); // IDEOGRAPHIC SPACE, ditto
        let re_neg = translated_regex(r"^\S$", false);
        assert!(re_neg.is_match("a"));
        assert!(!re_neg.is_match(" "));
    }

    #[test]
    fn w_escape_excludes_punctuation_separator_other() {
        let re = translated_regex(r"^\w$", false);
        assert!(re.is_match("a"));
        assert!(re.is_match("1"));
        // `_` is Unicode category Pc (Connector Punctuation) -- XSD's
        // `[^\p{P}\p{Z}\p{C}]` formula excludes it, unlike Perl's `\w`.
        assert!(!re.is_match("_"));
        assert!(!re.is_match(" "));
        assert!(!re.is_match("."));
        let re_neg = translated_regex(r"^\W$", false);
        assert!(re_neg.is_match("_"));
        assert!(re_neg.is_match("."));
        assert!(!re_neg.is_match("a"));
    }

    #[test]
    fn d_escape_is_unchanged_and_matches_non_ascii_decimal_digits() {
        let re = translated_regex(r"^\d+$", false);
        assert!(re.is_match("42"));
        assert!(!re.is_match("4a"));
        assert!(re.is_match("\u{0660}")); // ARABIC-INDIC DIGIT ZERO
        let re_neg = translated_regex(r"^\D$", false);
        assert!(re_neg.is_match("a"));
        assert!(!re_neg.is_match("4"));
    }

    #[test]
    fn general_category_escape_passes_through_unchanged() {
        let re = translated_regex(r"^\p{L}+$", false);
        assert!(re.is_match("abc"));
        assert!(!re.is_match("123"));
    }

    #[test]
    fn is_block_escape_is_spliced_from_the_table() {
        let re = translated_regex(r"^\p{IsBasicLatin}$", false);
        assert!(re.is_match("A"));
        assert!(!re.is_match("\u{0100}")); // Latin Extended-A: outside the block
        let re_neg = translated_regex(r"^\P{IsBasicLatin}$", false);
        assert!(re_neg.is_match("\u{0100}"));
        assert!(!re_neg.is_match("A"));
    }

    #[test]
    fn unknown_block_name_is_rejected() {
        let err = translate(r"\p{IsNotARealBlock}", false).unwrap_err();
        assert_eq!(
            err,
            XsdRegexError::UnknownBlock("IsNotARealBlock".to_owned())
        );
    }

    #[test]
    fn script_vs_block_confusion_is_closed() {
        // `\p{IsGreek}` is NOT a recognized block name (the real UCD block
        // is "Greek and Coptic", normalizing to `IsGreekandCoptic` per XML
        // Schema Part 2 §G.4.2.3) -- Rust's own un-intercepted resolution
        // silently accepts "IsGreek" anyway by falling back to the *Script*
        // `Greek` (confirmed live: `Regex::new(r"\p{IsGreek}")` compiles and
        // matches U+1F00 GREEK SMALL LETTER ALPHA WITH PSILI, which is in
        // the Greek Extended block, not Greek and Coptic). This translator
        // must reject that spelling outright rather than let it fall
        // through to Rust's resolution.
        let err = translate(r"\p{IsGreek}", false).unwrap_err();
        assert_eq!(err, XsdRegexError::UnknownBlock("IsGreek".to_owned()));

        // The correctly-spelled block escape must resolve to the BLOCK
        // (U+0370-03FF), not the wider Script: it matches a codepoint in
        // Greek and Coptic but not one in Greek Extended.
        let re = translated_regex(r"^\p{IsGreekandCoptic}$", false);
        assert!(re.is_match("\u{0391}")); // GREEK CAPITAL LETTER ALPHA
        assert!(!re.is_match("\u{1F00}")); // Greek Extended, not this block
    }

    #[test]
    fn backreference_is_rejected_single_digit() {
        let err = translate(r"(a)\1", false).unwrap_err();
        assert_eq!(err, XsdRegexError::Backreference("\\1".to_owned()));
    }

    #[test]
    fn backreference_is_rejected_multi_digit() {
        let err = translate(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\10", false).unwrap_err();
        assert_eq!(err, XsdRegexError::Backreference("\\10".to_owned()));
    }

    #[test]
    fn bare_b_and_bb_are_rejected_outside_a_class() {
        assert_eq!(
            translate(r"a\bc", false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            translate(r"\B", false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
    }

    #[test]
    fn bare_b_and_bb_are_rejected_inside_a_class() {
        assert_eq!(
            translate(r"[\b]", false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            translate(r"[\B]", false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
    }

    #[test]
    fn dangling_backslash_is_rejected() {
        assert!(matches!(
            translate("a\\", false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn unterminated_class_is_rejected() {
        assert!(matches!(
            translate("[abc", false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn unterminated_block_escape_name_is_rejected() {
        assert!(matches!(
            translate(r"\p{IsBasicLatin", false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn translation_introduces_zero_new_capturing_groups() {
        let re = translated_regex(r"^(\i+)\s(\c*)\p{IsBasicLatin}$", false);
        // 2 explicit groups in the source + the implicit whole-match group.
        assert_eq!(re.captures_len(), 3);
    }

    /// `x`'s exact four specification examples (XPath F&O 3.1 §5.6.2).
    fn compiles(source: &str) -> regex::Regex {
        regex::Regex::new(source).unwrap_or_else(|e| panic!("compile {source:?}: {e}"))
    }

    #[test]
    fn xpath_x_flag_matches_the_specifications_examples() {
        let no_space = compiles(&strip_x_flag_whitespace("hello world"));
        assert!(no_space.is_match("helloworld"));
        assert!(!no_space.is_match("hello world"));

        let bracket_space = compiles(&strip_x_flag_whitespace("hello[ ]world"));
        assert!(!bracket_space.is_match("helloworld"));

        let escaped_space = compiles(&strip_x_flag_whitespace("hello\\ sworld"));
        assert!(escaped_space.is_match("hello world"));
    }

    /// `\i` must match EXACTLY the set [`is_pn_chars_u`] accepts, plus `':'`
    /// (XSD's `\i` is XML 1.0 `NameStartChar` in full; `PN_CHARS_U` is
    /// `NameStartChar` minus `':'`, which this test folds back in) and `\c`
    /// must match exactly the set [`is_pn_chars`] accepts, plus `':'` and
    /// `'.'` (XSD's `\c` is XML 1.0 `NameChar` in full; `PN_CHARS` omits
    /// both `':'` -- for the same reason as `PN_CHARS_U` -- and `'.'`,
    /// which Turtle handles separately in `PN_LOCAL` because an unescaped
    /// `.` cannot end a Turtle name but XML's `NameChar` places no such
    /// restriction, see [`crate::blank_label::is_valid_ncname`]'s own
    /// `ch == '.'` fold-in), for every scalar value in the Unicode
    /// codespace. A full sweep over all ~1.1M scalar values is cheap for a
    /// compiled DFA; boundary codepoints of every range in
    /// `PN_CHARS_BASE_RANGES`/`PN_CHARS_EXTRA_RANGES` are exercised by
    /// construction since the sweep is exhaustive, not sampled.
    #[test]
    fn i_and_c_escapes_match_blank_label_tables_exactly() {
        let i_re = translated_regex(r"^\i$", false);
        let c_re = translated_regex(r"^\c$", false);
        for cp in 0_u32..=0x0010_FFFF {
            if (0xD800..=0xDFFF).contains(&cp) {
                continue; // surrogates: not representable as a `char`
            }
            let c = char::from_u32(cp).expect("valid scalar value");
            let s = c.to_string();
            let expected_i = is_pn_chars_u(c) || c == ':';
            assert_eq!(
                i_re.is_match(&s),
                expected_i,
                "\\i mismatch at U+{cp:04X} {c:?}"
            );
            let expected_c = is_pn_chars(c) || c == ':' || c == '.';
            assert_eq!(
                c_re.is_match(&s),
                expected_c,
                "\\c mismatch at U+{cp:04X} {c:?}"
            );
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `x`-flag whitespace stripper that runs ahead of [`super::translate`]
//! (see [`super`]'s module doc for where it sits in the pipeline).

use super::scan::Scanner;

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
    let mut scanner = Scanner::new(pattern);
    loop {
        // The `Result` is deliberately ignored: every rejected construct is
        // still consumed and its spelling is still available through
        // `source_text()`, because `x`'s removal is a textual rewrite that must
        // preserve the exact source a later translation will reject.
        if scanner.next().is_none() {
            break;
        }
        for (c, inside_class) in scanner.source_chars() {
            // Inside a character class nothing is removed; outside one, even
            // an escaped whitespace character goes, and the escape carries
            // over to the next character (`\ s` becomes `\s`). The class bit is
            // per character, not per token: a `\p{…}` name may contain an
            // unprotected `[`, so one span can cross a depth change.
            if inside_class || !purrdf_iri::terminals::is_ws_char(c) {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A removed whitespace character re-binds a pending `\` to whatever
    /// follows, which moves the class-exemption boundary with it. In `\ [ ` the
    /// `\` escapes the `[`, so no class opens and the trailing space goes; in
    /// `\ \[ ` the first `\` escapes the second, so the `[` DOES open and its
    /// space is exempt. The second case is where a naive per-token strip loses
    /// the thread, so it is pinned here.
    #[test]
    fn a_removed_whitespace_rebinds_the_pending_escape() {
        assert_eq!(strip_x_flag_whitespace("\\ [ "), "\\[");
        assert_eq!(strip_x_flag_whitespace("\\ \\[ "), "\\\\[ ");
        assert_eq!(strip_x_flag_whitespace("\\ s"), "\\s");
    }
}

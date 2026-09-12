// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ECMA-262 dialect-divergence scan that backs
//! [`super::ecma_262_divergences`].
//!
//! The translation this module used to perform now lives in [`super::emit`],
//! folded over the one cursor in [`super::scan`]; this module keeps only the
//! question an emitter must ask *before* it copies a `sh:pattern`'s source text
//! into an ECMA-262 slot: would that copy change the accepted language?

/// Scan a pattern for constructs that do **not** carry the same meaning in
/// ECMA-262 — the dialect JSON Schema's `pattern`, JavaScript, and most
/// `pattern` consumers use.
///
/// Class-awareness mirrors [`super::emit::translate`]'s own decisions exactly
/// (a `.` or a `-[` inside a character class is not the metacharacter), so a
/// construct is reported here if and only if translation actually rewrote it.
/// Names are returned in first-appearance order with duplicates collapsed, so
/// the caller's message is deterministic.
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

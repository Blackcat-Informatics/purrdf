// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! First-party conformance harness for `purrdf_core::xsd_regex` — the one
//! shared translation from the XSD/XPath `regExp` dialect onto the `regex`
//! crate that `sh:pattern`, SPARQL `REGEX`/`REPLACE` and ShEx `PATTERN` all
//! route through.
//!
//! The corpus lives in `crates/rdf-core/corpus/xsd-regex/`, one `.cases` file
//! per construct group; its `README.md` is the format's specification and
//! cites the clause behind each group. Cases are hand-derived from *XML Schema
//! Part 2* Appendix G and *XQuery and XPath Functions and Operators 3.1* §5.6,
//! because no redistributable W3C suite exercises this language in isolation.
//!
//! Being first-party, the corpus carries **no xfail ledger**: a corpus that
//! ledgers its own cases grades the engine against whatever the engine happens
//! to do. Every case must pass, and the harness asserts the **exact** case
//! count so a deleted file or a mistyped line reduces coverage loudly.
//!
//! The escape decoder is graded too ([`decoder_self_test`]) — a corpus read
//! through a decoder nobody tested is a corpus whose subjects are unknown.

#![cfg(not(target_arch = "wasm32"))]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use purrdf_core::xsd_regex;

const CORPUS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/corpus/xsd-regex");

/// The exact number of executable cases across every `.cases` file.
///
/// Asserted, not merely printed: a renamed or deleted corpus file would
/// otherwise silently shrink the suite while the harness still reported green.
/// Bump this when adding or removing a case.
const EXPECTED_CASES: usize = 247;

/// What a case says must happen.
#[derive(Debug, PartialEq, Eq)]
enum Expectation {
    /// The pattern compiles and matches the subject.
    Match,
    /// The pattern compiles and does not match the subject.
    NoMatch,
    /// The pattern does not compile, and the message contains this substring.
    Error(String),
}

/// One executable corpus line, with enough provenance to name it in a failure.
#[derive(Debug)]
struct Case {
    file: String,
    line: usize,
    pattern: String,
    flags: String,
    expectation: Expectation,
    /// The match subject (unused, and empty, for [`Expectation::Error`]).
    subject: String,
}

impl Case {
    /// A stable one-line identity for failure messages.
    fn describe(&self) -> String {
        format!(
            "{}:{} /{}/{}",
            self.file,
            self.line,
            self.pattern,
            if self.flags.is_empty() {
                "-"
            } else {
                &self.flags
            }
        )
    }
}

/// Decode the corpus escape alphabet: `\t`, `\n`, `\r` and `\uXXXX`.
///
/// Every other `\`-sequence passes through **unchanged, both characters** —
/// which is what lets a pattern field read exactly as a shape author would
/// write it (`\i`, `\p{IsGreek}`, `\\` for a literal backslash) while still
/// being able to name a control character. It is also what makes `\\uXXXX`
/// decode correctly: the scanner sees `\` followed by `\`, which is not one of
/// the four, so it emits both and resumes at the `u`.
///
/// # Errors
///
/// A `\uXXXX` whose four characters are not hex, or which names a value that
/// is not a Unicode scalar (a surrogate), is an error rather than a silent
/// literal: a typo in a corpus file must fail the harness, not quietly test a
/// different pattern.
fn decode(field: &str) -> Result<String, String> {
    let chars: Vec<char> = field.chars().collect();
    let mut out = String::with_capacity(field.len());
    let mut i = 0_usize;
    while i < chars.len() {
        if chars[i] != '\\' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('t') => {
                out.push('\t');
                i += 2;
            }
            Some('n') => {
                out.push('\n');
                i += 2;
            }
            Some('r') => {
                out.push('\r');
                i += 2;
            }
            Some('u') => {
                let digits: String = chars
                    .get(i + 2..i + 6)
                    .map_or_else(String::new, |slice| slice.iter().collect::<String>());
                if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!(
                        "malformed \\u escape at char {i}: expected four hex digits, found \
                         {digits:?}"
                    ));
                }
                let value =
                    u32::from_str_radix(&digits, 16).map_err(|e| format!("\\u{digits}: {e}"))?;
                let decoded = char::from_u32(value)
                    .ok_or_else(|| format!("\\u{digits} is not a Unicode scalar value"))?;
                out.push(decoded);
                i += 6;
            }
            // Not one of the four: emit the backslash and whatever follows it,
            // verbatim, and resume past both.
            Some(&other) => {
                out.push('\\');
                out.push(other);
                i += 2;
            }
            None => {
                out.push('\\');
                i += 1;
            }
        }
    }
    Ok(out)
}

/// Parse one `.cases` file into executable cases, rejecting anything
/// malformed rather than skipping it.
fn parse_file(path: &Path) -> Result<Vec<Case>, String> {
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut cases = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = raw.split('\t').collect();
        if fields.len() != 4 {
            return Err(format!(
                "{name}:{line}: expected 4 tab-separated fields, found {} in {raw:?}",
                fields.len()
            ));
        }
        let pattern =
            decode(fields[0]).map_err(|e| format!("{name}:{line}: pattern field: {e}"))?;
        let flags = if fields[1] == "-" {
            String::new()
        } else {
            fields[1].to_owned()
        };
        let subject =
            decode(fields[3]).map_err(|e| format!("{name}:{line}: subject field: {e}"))?;
        let (expectation, subject) = match fields[2] {
            "match" => (Expectation::Match, subject),
            "nomatch" => (Expectation::NoMatch, subject),
            "error" => (Expectation::Error(subject), String::new()),
            other => {
                return Err(format!(
                    "{name}:{line}: unknown expectation {other:?} (expected match, nomatch or \
                     error)"
                ));
            }
        };
        if !matches!(expectation, Expectation::Error(_)) && subject.is_empty() {
            return Err(format!(
                "{name}:{line}: the subject field must be non-empty for a match/nomatch case \
                 (see the corpus README)"
            ));
        }
        cases.push(Case {
            file: name.clone(),
            line,
            pattern,
            flags,
            expectation,
            subject,
        });
    }
    Ok(cases)
}

/// Every `.cases` file in the corpus, sorted so the run order is
/// deterministic.
fn corpus_files() -> Vec<PathBuf> {
    let dir = Path::new(CORPUS_DIR);
    assert!(dir.is_dir(), "corpus directory not found at {CORPUS_DIR}");
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {CORPUS_DIR}: {e}"))
        .map(|entry| entry.expect("corpus dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cases"))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no .cases files found in {CORPUS_DIR} — the corpus cannot have been read"
    );
    files
}

/// Run one case, returning `Err` with the full story when it does not hold.
fn run(case: &Case) -> Result<(), String> {
    let compiled = xsd_regex::compile(&case.pattern, &case.flags);
    match (&case.expectation, compiled) {
        (Expectation::Match, Ok(re)) => {
            if re.is_match(&case.subject) {
                Ok(())
            } else {
                Err(format!(
                    "{} expected to match {:?}, did not",
                    case.describe(),
                    case.subject
                ))
            }
        }
        (Expectation::NoMatch, Ok(re)) => {
            if re.is_match(&case.subject) {
                Err(format!(
                    "{} expected NOT to match {:?}, matched",
                    case.describe(),
                    case.subject
                ))
            } else {
                Ok(())
            }
        }
        (Expectation::Match | Expectation::NoMatch, Err(e)) => Err(format!(
            "{} expected to compile, failed: {e}",
            case.describe()
        )),
        (Expectation::Error(needle), Err(e)) => {
            let message = e.to_string();
            if message.contains(needle.as_str()) {
                Ok(())
            } else {
                Err(format!(
                    "{} failed to compile as expected, but the message {message:?} does not \
                     contain {needle:?} — an error that does not name its construct is not an \
                     actionable one",
                    case.describe()
                ))
            }
        }
        (Expectation::Error(needle), Ok(_)) => Err(format!(
            "{} expected to FAIL with a message containing {needle:?}, but it compiled",
            case.describe()
        )),
    }
}

#[test]
fn xsd_regex_corpus_is_green() {
    let files = corpus_files();
    let mut cases = Vec::new();
    for path in &files {
        cases.extend(parse_file(path).unwrap_or_else(|e| panic!("{e}")));
    }

    let total = cases.len();
    let mut failures: Vec<String> = Vec::new();
    for case in &cases {
        if let Err(message) = run(case) {
            failures.push(message);
        }
    }
    let passed = total - failures.len();

    // Printed BEFORE the assertions so a red run still reports a scoreboard
    // for `scripts/conformance-matrix.py` to scrape.
    println!("XSD-REGEX-CORPUS: passed {passed} total {total}");

    if !failures.is_empty() {
        let mut report = format!("{} of {total} corpus case(s) failed:\n", failures.len());
        for failure in &failures {
            let _ = writeln!(report, "  - {failure}");
        }
        panic!("{report}");
    }
    assert_eq!(
        passed, total,
        "every first-party corpus case must pass — this corpus carries no xfail ledger"
    );
}

/// The corpus's size and shape, asserted separately from its greenness so a
/// shrinking corpus is a distinct failure from a failing one.
#[test]
fn xsd_regex_corpus_case_count_is_exact() {
    let files = corpus_files();
    assert_eq!(
        files.len(),
        6,
        "unexpected .cases file count — update this when adding or removing a corpus file"
    );
    let total: usize = files
        .iter()
        .map(|path| parse_file(path).unwrap_or_else(|e| panic!("{e}")).len())
        .sum();
    assert_eq!(
        total, EXPECTED_CASES,
        "unexpected corpus case count — update EXPECTED_CASES when adding or removing a case"
    );
}

/// The escape decoder, graded against hand-written expectations. A corpus read
/// through an untested decoder is a corpus whose subjects are unknown — in
/// particular the pass-through rule, which is what keeps `\i` and `\\uXXXX`
/// meaning what they look like.
#[test]
fn decoder_self_test() {
    // The four decoded sequences.
    assert_eq!(decode(r"a\tb").expect("decode"), "a\tb");
    assert_eq!(decode(r"a\nb").expect("decode"), "a\nb");
    assert_eq!(decode(r"a\rb").expect("decode"), "a\rb");
    assert_eq!(decode(r"a\u0041b").expect("decode"), "aAb");
    assert_eq!(decode(r"\u00a0").expect("decode"), "\u{a0}");
    assert_eq!(decode(r"\u3000").expect("decode"), "\u{3000}");

    // Pass-through: a regex escape survives as its two characters.
    assert_eq!(decode(r"^\i\c*$").expect("decode"), r"^\i\c*$");
    assert_eq!(decode(r"\p{IsGreek}").expect("decode"), r"\p{IsGreek}");
    assert_eq!(decode(r"\s\d\w").expect("decode"), r"\s\d\w");
    assert_eq!(decode(r"a\.b").expect("decode"), r"a\.b");

    // A doubled backslash is two pass-through characters, so a following
    // `uXXXX` stays literal — the case that decides whether the alphabet is
    // usable inside a pattern at all.
    assert_eq!(decode(r"a\\b").expect("decode"), r"a\\b");
    assert_eq!(decode(r"\\u0041").expect("decode"), r"\\u0041");
    assert_eq!(decode(r"\\t").expect("decode"), r"\\t");

    // A trailing backslash is the field's own last character, not a decode.
    assert_eq!(decode(r"abc\").expect("decode"), r"abc\");

    // Nothing to decode.
    assert_eq!(decode("plain text").expect("decode"), "plain text");
    assert_eq!(decode("").expect("decode"), "");

    // Malformed `\u` is an error, never a silent literal.
    assert!(decode(r"\u00").is_err(), "too few hex digits");
    assert!(decode(r"\uZZZZ").is_err(), "not hex");
    assert!(decode(r"\ud800").is_err(), "a surrogate is not a scalar");
}

/// The harness's own failure detection, graded: a case that should fail must
/// actually be reported as failing. Without this the suite could pass by
/// never checking anything.
#[test]
fn harness_detects_a_wrong_expectation() {
    let wrong = Case {
        file: "synthetic".to_owned(),
        line: 0,
        pattern: "^a$".to_owned(),
        flags: String::new(),
        expectation: Expectation::Match,
        subject: "b".to_owned(),
    };
    assert!(
        run(&wrong).is_err(),
        "a pattern that does not match its subject must be reported as a failure"
    );

    let right = Case {
        expectation: Expectation::NoMatch,
        ..wrong
    };
    assert!(run(&right).is_ok());

    let wrong_error = Case {
        file: "synthetic".to_owned(),
        line: 0,
        pattern: "^a$".to_owned(),
        flags: String::new(),
        expectation: Expectation::Error("some message".to_owned()),
        subject: String::new(),
    };
    assert!(
        run(&wrong_error).is_err(),
        "a pattern expected to fail but which compiles must be reported as a failure"
    );

    let wrong_needle = Case {
        pattern: r"\bx".to_owned(),
        expectation: Expectation::Error("not in the message".to_owned()),
        ..wrong_error
    };
    assert!(
        run(&wrong_needle).is_err(),
        "an error whose message lacks the expected substring must be reported as a failure"
    );
}

/// A malformed corpus line is a harness failure, not a skipped case.
#[test]
fn parser_rejects_a_malformed_line() {
    // `TempDir`, not a fixed path under `temp_dir()`: a fixed one is shared by
    // concurrent runs of this binary and survives a panic between writing a
    // fixture and removing it, so the NEXT run fails on a leftover file for a
    // reason that has nothing to do with the parser. `TempDir` gives a unique
    // directory and removes the whole tree on drop, panic included.
    let dir = tempfile::Builder::new()
        .prefix("purrdf-xsd-regex-corpus-parse-")
        .tempdir()
        .expect("create temp dir");

    let cases = [
        ("three-fields.cases", "^a$\t-\tmatch\n"),
        ("bad-expectation.cases", "^a$\t-\tmaybe\ta\n"),
        ("empty-subject.cases", "^a$\t-\tmatch\t\n"),
        ("bad-escape.cases", "^a$\t-\tmatch\t\\u00\n"),
    ];
    for (name, body) in cases {
        let path = dir.path().join(name);
        fs::write(&path, body).expect("write temp corpus");
        assert!(
            parse_file(&path).is_err(),
            "{name} must be rejected, not skipped"
        );
    }

    // ...and a well-formed one is accepted, so the rejections above are not
    // the parser refusing everything.
    let ok = dir.path().join("ok.cases");
    fs::write(&ok, "# a comment\n\n^a$\t-\tmatch\ta\n^a$\ti\tnomatch\tb\n")
        .expect("write temp corpus");
    let parsed = parse_file(&ok).expect("a well-formed file parses");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].flags, "");
    assert_eq!(parsed[1].flags, "i");
}

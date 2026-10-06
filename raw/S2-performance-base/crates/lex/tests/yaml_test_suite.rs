// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `purrdf_lex::yaml` reader graded against the official YAML test suite
//! (`vectors/yaml-test-suite`, vendored by `scripts/vendor-yaml-test-suite.py`).
//!
//! Every case is run. Each lands in exactly one class:
//!
//! * **pass**: the suite marks the input valid and the reader's value equals
//!   the suite's `in.json` (compared as JSON values: numbers by value, object
//!   members by name). Where the suite records no `in.json` (its generator
//!   could not write one) the value is compared with the one its `test.event`
//!   stream denotes, and a stream whose events have no JSON value (a null or
//!   collection key) must be refused;
//! * **empty stream**: the suite's stream holds no document (its `in.json` is
//!   empty) and the reader reads it as `null`, as `serde_yaml` did;
//! * **expected refusal**: the suite marks the input invalid (an `error` file)
//!   and the reader refuses it;
//! * **unsupported, refused**: the input is valid YAML the reader hard-fails on
//!   with an error kind the reader documents as "JSON cannot hold this"
//!   ([`ErrorKind::Tag`], [`ErrorKind::NonStringKey`],
//!   [`ErrorKind::NonFinite`], [`ErrorKind::MultipleDocuments`]); aliases and
//!   explicit keys are read, so only those kinds qualify;
//! * **FAIL**: anything else: a wrong value, an invalid input accepted, a valid
//!   input refused with a grammar error (over-refusal), or a panic.
//!
//! The unsupported class is derived from the reader's own error kinds; there is
//! no list of skipped cases. The totals are pinned, so any drift in either
//! direction (a fix that turns a refusal into a pass, or a regression) fails
//! here and is re-pinned deliberately.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use purrdf_lex::json::{self, Value};
use purrdf_lex::yaml::{self, ErrorKind};

/// Whether `kind` says "valid YAML, but JSON (or this reader's JSON-shaped
/// output) cannot hold it", as opposed to a grammar refusal.
const fn unsupported(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::Tag
            | ErrorKind::NonStringKey
            | ErrorKind::NonFinite
            | ErrorKind::MultipleDocuments
    )
}

/// JSON equality: numbers by value, object members by name (order-free),
/// everything else structurally.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            x == y || (x.is_integer() == y.is_integer() && x.as_f64() == y.as_f64())
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(name, p)| y.get(name).is_some_and(|q| same(p, q)))
        }
        _ => false,
    }
}

fn cases(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if dir.join("in.yaml").is_file() {
            found.push(dir);
            continue;
        }
        for entry in fs::read_dir(&dir).expect("a readable suite directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The JSON value a `test.event` stream denotes, when it has one: a single
/// document whose keys are all strings and whose scalars are unambiguous
/// (quoted, block, or a plain scalar that cannot be a number, boolean or
/// null word). `None` means the stream has no JSON value this test can state.
fn events_value(events: &str) -> Option<Value> {
    enum Open {
        Seq(Vec<Value>),
        Map(Vec<(String, Value)>, Option<String>),
    }
    fn attach(stack: &mut [Open], root: &mut Option<Value>, value: Value) -> Option<()> {
        match stack.last_mut() {
            None => {
                if root.is_some() {
                    return None;
                }
                *root = Some(value);
            }
            Some(Open::Seq(items)) => items.push(value),
            Some(Open::Map(members, key)) => match key.take() {
                Some(name) => members.push((name, value)),
                None => {
                    *key = Some(value.as_str()?.to_owned());
                }
            },
        }
        Some(())
    }
    let mut stack: Vec<Open> = Vec::new();
    let mut root: Option<Value> = None;
    let mut docs = 0;
    for line in events.lines() {
        let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
        match kind {
            "+STR" | "-STR" | "-DOC" => {}
            "+DOC" => docs += 1,
            "+SEQ" => stack.push(Open::Seq(Vec::new())),
            "+MAP" => stack.push(Open::Map(Vec::new(), None)),
            "-SEQ" => {
                let Some(Open::Seq(items)) = stack.pop() else {
                    return None;
                };
                attach(&mut stack, &mut root, Value::Array(items))?;
            }
            "-MAP" => {
                let Some(Open::Map(members, None)) = stack.pop() else {
                    return None;
                };
                let mut object = json::Object::new();
                for (name, value) in members {
                    object.push(name, value);
                }
                attach(&mut stack, &mut root, Value::Object(object))?;
            }
            "=VAL" => {
                // Properties (`&anchor`, `<tag>`) precede the style character.
                let mut rest = rest;
                let mut string_tag = false;
                while let Some(word) = rest.split(' ').next() {
                    if word.starts_with('&') {
                        rest = rest[word.len()..].trim_start_matches(' ');
                    } else if word.starts_with('<') {
                        if word != "<tag:yaml.org,2002:str>" && word != "<!>" {
                            return None;
                        }
                        string_tag = true;
                        rest = rest[word.len()..].trim_start_matches(' ');
                    } else {
                        break;
                    }
                }
                let style = rest.chars().next()?;
                let text = &rest[style.len_utf8()..];
                let mut decoded = String::new();
                let mut chars = text.chars();
                while let Some(c) = chars.next() {
                    if c != '\\' {
                        decoded.push(c);
                        continue;
                    }
                    decoded.push(match chars.next()? {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        'b' => '\u{8}',
                        'e' => '\u{1b}',
                        '\\' => '\\',
                        _ => return None,
                    });
                }
                let value = match style {
                    ':' if string_tag => Value::String(decoded),
                    ':' if decoded.is_empty() => Value::Null,
                    ':' if decoded
                        .starts_with(|c: char| c.is_ascii_digit() || "+-.~".contains(c))
                        || [
                            "null", "Null", "NULL", "true", "True", "TRUE", "false", "False",
                            "FALSE",
                        ]
                        .contains(&decoded.as_str()) =>
                    {
                        return None;
                    }
                    ':' | '\'' | '"' | '|' | '>' => Value::String(decoded),
                    _ => return None,
                };
                attach(&mut stack, &mut root, value)?;
            }
            _ => return None,
        }
    }
    (docs == 1 && stack.is_empty()).then_some(root).flatten()
}

#[derive(Default)]
struct Tally {
    pass: usize,
    empty_stream: usize,
    expected_refusal: usize,
    unsupported_refused: usize,
    fail: Vec<String>,
    /// The unsupported refusals, by error kind and case, for the report.
    unsupported: Vec<(String, String)>,
}

fn grade(root: &Path, case: &Path, tally: &mut Tally) {
    let id = case
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let input = fs::read(case.join("in.yaml")).expect("in.yaml");
    let input = String::from_utf8(input).expect("suite inputs are UTF-8");
    let invalid = case.join("error").is_file();
    let outcome = catch_unwind(AssertUnwindSafe(|| yaml::read(&input)));
    let Ok(outcome) = outcome else {
        tally.fail.push(format!("{id}: the reader panicked"));
        return;
    };
    match (invalid, outcome) {
        (true, Err(_)) => tally.expected_refusal += 1,
        (true, Ok(value)) => tally
            .fail
            .push(format!("{id}: invalid YAML accepted as {value}")),
        (false, Err(error)) if unsupported(error.kind()) => {
            // An "unsupported" refusal is a claim too: prove each kind's own
            // precondition against the suite, so a valid, JSON-representable
            // input cannot hide behind it.
            let events = fs::read_to_string(case.join("test.event")).expect("test.event");
            let proven = match error.kind() {
                // The suite gives the input no JSON value at all.
                ErrorKind::NonStringKey => !case.join("in.json").is_file(),
                ErrorKind::MultipleDocuments => events.matches("+DOC").count() > 1,
                ErrorKind::Tag => input.contains('!'),
                ErrorKind::NonFinite => input.contains(".inf") || input.contains(".nan"),
                _ => false,
            };
            if proven {
                tally.unsupported_refused += 1;
                tally.unsupported.push((format!("{:?}", error.kind()), id));
            } else {
                tally.fail.push(format!(
                    "{id}: refused as {:?}, but the suite does not bear that out: {error}",
                    error.kind()
                ));
            }
        }
        (false, Err(error)) => tally
            .fail
            .push(format!("{id}: valid YAML refused: {error}")),
        (false, Ok(value)) => {
            let expected = fs::read_to_string(case.join("in.json")).ok();
            let want = match expected.as_deref() {
                Some(text) if text.trim().is_empty() => {
                    // A stream with no document.
                    if value.is_null() {
                        tally.empty_stream += 1;
                    } else {
                        tally
                            .fail
                            .push(format!("{id}: an empty stream read as {value}"));
                    }
                    return;
                }
                Some(text) => match json::read(text) {
                    Ok(want) => Some(want),
                    Err(_) => {
                        tally.fail.push(format!(
                            "{id}: accepted as {value}, but the suite's JSON is several documents"
                        ));
                        return;
                    }
                },
                None => {
                    let events = fs::read_to_string(case.join("test.event")).expect("test.event");
                    events_value(&events)
                }
            };
            match want {
                Some(want) if same(&want, &value) => tally.pass += 1,
                Some(want) => tally
                    .fail
                    .push(format!("{id}: wrong value {value}, the suite says {want}")),
                None => tally.fail.push(format!(
                    "{id}: accepted as {value}, but the suite's events have no JSON value"
                )),
            }
        }
    }
}

#[test]
fn the_reader_is_graded_against_the_yaml_test_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/yaml-test-suite");
    let all = cases(&root);
    let mut tally = Tally::default();
    for case in &all {
        grade(&root, case, &mut tally);
    }
    let Tally {
        pass,
        empty_stream,
        expected_refusal,
        unsupported_refused,
        fail,
        unsupported,
    } = tally;
    eprintln!(
        "yaml-test-suite: {} cases: {pass} pass, {empty_stream} empty-stream, {expected_refusal} expected-refusal, {unsupported_refused} unsupported-refused, {} FAIL",
        all.len(),
        fail.len()
    );
    let mut by_kind = std::collections::BTreeMap::<&str, Vec<&str>>::new();
    for (kind, id) in &unsupported {
        by_kind.entry(kind).or_default().push(id);
    }
    for (kind, ids) in &by_kind {
        eprintln!("unsupported {kind}: {} cases: {}", ids.len(), ids.join(" "));
    }
    assert!(fail.is_empty(), "FAIL:\n{}", fail.join("\n"));
    assert_eq!(all.len(), 402, "the vendored suite's case count");
    assert_eq!(
        (pass, empty_stream, expected_refusal, unsupported_refused),
        (244, 5, 94, 59),
        "the pinned totals"
    );
}

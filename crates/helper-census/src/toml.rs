// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A strict reader for the TOML subset `helpers-ledger.toml` is written in.
//!
//! The subset is: `#` comments; `[[a]]`, `[[a.b]]` array-of-table headers and
//! `[a.b]` table headers over bare keys; `key = value` lines with bare keys;
//! values that are basic strings (`"…"` with the TOML escapes), literal strings
//! (`'…'`), booleans, decimal integers, and arrays of those values that may span
//! lines, carry comments and end with a trailing comma. Anything else — inline
//! tables, dotted keys on the left of `=`, multi-line strings, dates, floats — is
//! refused with its line number rather than guessed at, so the ledger can never
//! mean one thing to this reader and another to Python's `tomllib`. The engine
//! (`scripts/check-shared-helpers.py`) compares this reader's result with
//! `tomllib`'s on every run.

use std::collections::BTreeMap;
use std::fmt;

/// One TOML value of the subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    /// A basic or literal string.
    Str(String),
    /// `true` or `false`.
    Bool(bool),
    /// A decimal integer.
    Int(i64),
    /// An array of values.
    Array(Vec<Self>),
    /// A table (the root, a `[table]`, or one element of an `[[array]]`).
    Table(BTreeMap<String, Self>),
}

impl Value {
    /// The value as JSON, for the engine's cross-check against `tomllib`.
    pub(crate) fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Str(text) => serde_json::Value::String(text.clone()),
            Self::Bool(flag) => serde_json::Value::Bool(*flag),
            Self::Int(number) => serde_json::Value::from(*number),
            Self::Array(items) => {
                serde_json::Value::Array(items.iter().map(Self::to_json).collect())
            }
            Self::Table(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json()))
                    .collect(),
            ),
        }
    }
}

/// A refusal, with the 1-based line it was found on.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Error {
    /// 1-based line number.
    pub(crate) line: usize,
    /// What is wrong there.
    pub(crate) message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

/// Parse `text` into its root table.
pub(crate) fn parse(text: &str) -> Result<BTreeMap<String, Value>, Error> {
    let mut reader = Reader {
        chars: text.chars().collect(),
        at: 0,
        line: 1,
    };
    let mut root: BTreeMap<String, Value> = BTreeMap::new();
    // The path of the table `key = value` lines currently land in; each segment
    // names a key, and an array segment always means its last element.
    let mut current: Vec<String> = Vec::new();
    loop {
        reader.skip_blank_lines();
        let Some(next) = reader.peek() else {
            return Ok(root);
        };
        let line = reader.line;
        if next == '[' {
            let (path, is_array) = reader.header()?;
            open_header(&mut root, &path, is_array).map_err(|message| Error { line, message })?;
            current = path;
        } else {
            let key = reader.bare_key()?;
            reader.skip_inline_space();
            reader.expect('=')?;
            reader.skip_inline_space();
            let value = reader.value()?;
            reader.end_of_line()?;
            let table = table_at(&mut root, &current).map_err(|message| Error { line, message })?;
            if table.insert(key.clone(), value).is_some() {
                return Err(Error {
                    line,
                    message: format!("key `{key}` is defined twice in one table"),
                });
            }
        }
    }
}

/// Create the table (or append the array element) a header names.
fn open_header(
    root: &mut BTreeMap<String, Value>,
    path: &[String],
    is_array: bool,
) -> Result<(), String> {
    let (last, parents) = path.split_last().ok_or("an empty table header")?;
    let parent = table_at(root, parents)?;
    if is_array {
        match parent
            .entry(last.clone())
            .or_insert_with(|| Value::Array(Vec::new()))
        {
            Value::Array(items) => {
                items.push(Value::Table(BTreeMap::new()));
                Ok(())
            }
            _ => Err(format!(
                "`[[{}]]` names a key that is not an array of tables",
                path.join(".")
            )),
        }
    } else if parent.contains_key(last) {
        Err(format!("table `[{}]` is defined twice", path.join(".")))
    } else {
        parent.insert(last.clone(), Value::Table(BTreeMap::new()));
        Ok(())
    }
}

/// The table at `path`, descending into the last element of every array.
fn table_at<'a>(
    root: &'a mut BTreeMap<String, Value>,
    path: &[String],
) -> Result<&'a mut BTreeMap<String, Value>, String> {
    let mut table = root;
    for segment in path {
        let next = table
            .get_mut(segment)
            .ok_or_else(|| format!("`{segment}` is used before its header"))?;
        table = match next {
            Value::Table(inner) => inner,
            Value::Array(items) => match items.last_mut() {
                Some(Value::Table(inner)) => inner,
                _ => return Err(format!("`{segment}` is not an array of tables")),
            },
            _ => return Err(format!("`{segment}` is a value, not a table")),
        };
    }
    Ok(table)
}

/// A character cursor that counts lines.
struct Reader {
    chars: Vec<char>,
    at: usize,
    line: usize,
}

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.at += 1;
        if next == '\n' {
            self.line += 1;
        }
        Some(next)
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, Error> {
        Err(Error {
            line: self.line,
            message: message.into(),
        })
    }

    fn expect(&mut self, wanted: char) -> Result<(), Error> {
        match self.peek() {
            Some(found) if found == wanted => {
                self.bump();
                Ok(())
            }
            Some(found) => self.fail(format!("expected `{wanted}`, found `{found}`")),
            None => self.fail(format!("expected `{wanted}`, found the end of the file")),
        }
    }

    fn skip_inline_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while !matches!(self.peek(), None | Some('\n')) {
                self.bump();
            }
        }
    }

    /// Skip whitespace, newlines and comments between statements.
    fn skip_blank_lines(&mut self) {
        loop {
            self.skip_inline_space();
            self.skip_comment();
            match self.peek() {
                Some('\n') => {
                    self.bump();
                }
                Some('\r') if self.chars.get(self.at + 1) == Some(&'\n') => {
                    self.bump();
                    self.bump();
                }
                _ => return,
            }
        }
    }

    /// The rest of a statement line must be space and an optional comment.
    fn end_of_line(&mut self) -> Result<(), Error> {
        self.skip_inline_space();
        self.skip_comment();
        match self.peek() {
            None | Some('\n') => Ok(()),
            Some('\r') if self.chars.get(self.at + 1) == Some(&'\n') => Ok(()),
            Some(found) => self.fail(format!("unexpected `{found}` after a complete statement")),
        }
    }

    fn bare_key(&mut self) -> Result<String, Error> {
        let mut key = String::new();
        while let Some(next) = self.peek() {
            if next.is_ascii_alphanumeric() || next == '_' || next == '-' {
                key.push(next);
                self.bump();
            } else {
                break;
            }
        }
        if key.is_empty() {
            return self
                .fail("expected a bare key (quoted and dotted keys are outside the subset)");
        }
        Ok(key)
    }

    /// `[a.b]` or `[[a.b]]`.
    fn header(&mut self) -> Result<(Vec<String>, bool), Error> {
        self.expect('[')?;
        let is_array = self.peek() == Some('[');
        if is_array {
            self.bump();
        }
        let mut path = Vec::new();
        loop {
            self.skip_inline_space();
            path.push(self.bare_key()?);
            self.skip_inline_space();
            if self.peek() == Some('.') {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(']')?;
        if is_array {
            self.expect(']')?;
        }
        self.end_of_line()?;
        Ok((path, is_array))
    }

    fn value(&mut self) -> Result<Value, Error> {
        match self.peek() {
            Some('"') => self.basic_string().map(Value::Str),
            Some('\'') => self.literal_string().map(Value::Str),
            Some('[') => self.array(),
            Some('t' | 'f') => {
                let word = self.bare_key()?;
                match word.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    _ => self.fail(format!("`{word}` is not a value")),
                }
            }
            Some(first) if first.is_ascii_digit() || first == '-' || first == '+' => self.integer(),
            Some('{') => self.fail("inline tables are outside the subset; use a [table] header"),
            Some(found) => self.fail(format!("`{found}` does not start a value")),
            None => self.fail("expected a value, found the end of the file"),
        }
    }

    fn integer(&mut self) -> Result<Value, Error> {
        let mut digits = String::new();
        if let Some(sign @ ('-' | '+')) = self.peek() {
            digits.push(sign);
            self.bump();
        }
        let mut previous_underscore = true;
        while let Some(next) = self.peek() {
            if next.is_ascii_digit() {
                digits.push(next);
                previous_underscore = false;
            } else if next == '_' && !previous_underscore {
                previous_underscore = true;
            } else {
                break;
            }
            self.bump();
        }
        if previous_underscore {
            return self.fail("an integer must start and end with a digit");
        }
        if matches!(
            self.peek(),
            Some('.' | 'e' | 'E' | 'x' | 'o' | 'b' | ':' | '-')
        ) {
            return self.fail(
                "only decimal integers are in the subset (no floats, dates or radix prefixes)",
            );
        }
        let unsigned = digits.trim_start_matches(['-', '+']);
        if unsigned.len() > 1 && unsigned.starts_with('0') {
            return self.fail("a TOML integer has no leading zeros");
        }
        digits
            .parse()
            .map(Value::Int)
            .or_else(|_| self.fail(format!("`{digits}` does not fit a 64-bit integer")))
    }

    fn basic_string(&mut self) -> Result<String, Error> {
        self.expect('"')?;
        if self.peek() == Some('"') && self.chars.get(self.at + 1) == Some(&'"') {
            return self.fail("multi-line strings are outside the subset");
        }
        let mut text = String::new();
        loop {
            if matches!(self.peek(), None | Some('\n')) {
                return self.fail("a basic string is not closed on its line");
            }
            match self.bump() {
                None => return self.fail("a basic string is not closed on its line"),
                Some('"') => return Ok(text),
                Some('\\') => text.push(self.escape()?),
                Some(control) if control.is_control() && control != '\t' => {
                    return self.fail("a control character must be escaped in a basic string");
                }
                Some(other) => text.push(other),
            }
        }
    }

    fn escape(&mut self) -> Result<char, Error> {
        let escaped = match self.bump() {
            Some('b') => '\u{8}',
            Some('t') => '\t',
            Some('n') => '\n',
            Some('f') => '\u{c}',
            Some('r') => '\r',
            Some('"') => '"',
            Some('\\') => '\\',
            Some('u') => self.unicode_escape(4)?,
            Some('U') => self.unicode_escape(8)?,
            Some(other) => return self.fail(format!("`\\{other}` is not a TOML escape")),
            None => return self.fail("an escape is cut off by the end of the file"),
        };
        Ok(escaped)
    }

    fn unicode_escape(&mut self, width: usize) -> Result<char, Error> {
        let mut scalar = 0_u32;
        for _ in 0..width {
            let digit = self
                .bump()
                .and_then(|next| u8::try_from(next).ok())
                .and_then(purrdf_hash::hex::nibble)
                .map(u32::from)
                .map_or_else(
                    || self.fail("a unicode escape needs hexadecimal digits"),
                    Ok,
                )?;
            scalar = scalar * 16 + digit;
        }
        char::from_u32(scalar).map_or_else(
            || self.fail(format!("U+{scalar:X} is not a Unicode scalar value")),
            Ok,
        )
    }

    fn literal_string(&mut self) -> Result<String, Error> {
        self.expect('\'')?;
        if self.peek() == Some('\'') && self.chars.get(self.at + 1) == Some(&'\'') {
            return self.fail("multi-line literal strings are outside the subset");
        }
        let mut text = String::new();
        loop {
            if matches!(self.peek(), None | Some('\n')) {
                return self.fail("a literal string is not closed on its line");
            }
            match self.bump() {
                None => return self.fail("a literal string is not closed on its line"),
                Some('\'') => return Ok(text),
                Some(other) => text.push(other),
            }
        }
    }

    /// Whitespace, newlines and comments inside an array.
    fn skip_array_space(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\n' | '\r') => {
                    self.bump();
                }
                Some('#') => self.skip_comment(),
                _ => return,
            }
        }
    }

    fn array(&mut self) -> Result<Value, Error> {
        self.expect('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_array_space();
            if self.peek() == Some(']') {
                self.bump();
                return Ok(Value::Array(items));
            }
            items.push(self.value()?);
            self.skip_array_space();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {}
                _ => return self.fail("array items are separated by `,`"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, parse};

    #[test]
    fn arrays_of_tables_nest_under_their_last_element() {
        let root = parse(
            "# comment\n[[job]]\nid = \"a\"\n[[job.variant]]\nsymbol = 'x'\n[job.forbidden]\nnames = [\n  '^a$', # why\n  \"b\\u0041\",\n]\n[[job]]\nid = \"b\"\nenforced = true\ncount = -3\n",
        )
        .expect("the subset parses");
        let Some(Value::Array(jobs)) = root.get("job") else {
            panic!("job is an array");
        };
        assert_eq!(jobs.len(), 2);
        let Value::Table(first) = &jobs[0] else {
            panic!("a job is a table");
        };
        let Some(Value::Table(forbidden)) = first.get("forbidden") else {
            panic!("forbidden is a table");
        };
        assert_eq!(
            forbidden.get("names"),
            Some(&Value::Array(vec![
                Value::Str("^a$".to_owned()),
                Value::Str("bA".to_owned())
            ]))
        );
        let Value::Table(second) = &jobs[1] else {
            panic!("a job is a table");
        };
        assert_eq!(second.get("enforced"), Some(&Value::Bool(true)));
        assert_eq!(second.get("count"), Some(&Value::Int(-3)));
    }

    #[test]
    fn constructs_outside_the_subset_are_refused_with_their_line() {
        for (source, line) in [
            ("a = { b = 1 }\n", 1),
            ("\n\na = \"\"\"x\"\"\"\n", 3),
            ("a.b = 1\n", 1),
            ("a = 1.5\n", 1),
            ("a = 0x10\n", 1),
            ("a = 1\na = 2\n", 2),
            ("[t]\n[t]\n", 2),
            ("a = \"open\n", 1),
            ("a = 1 b\n", 1),
        ] {
            let error = parse(source).expect_err(source);
            assert_eq!(error.line, line, "{source:?}: {error}");
        }
    }

    #[test]
    fn the_neighbouring_valid_spellings_are_accepted() {
        for source in [
            "a = []\n",
            "a = \"x\" # trailing comment\n",
            "a = 1_000\n",
            "a = +7\n",
            "[t]\nb = 1\n",
            "a = 'C:\\path'\n",
        ] {
            parse(source).unwrap_or_else(|error| panic!("{source:?}: {error}"));
        }
    }
}

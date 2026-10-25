// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! TOML 1.1 grammar (https://toml.io/en/v1.1.0), with explicit container frames.
//! Table declaration identity is resolved against arena parents, never a flattened
//! dotted string: quoted dots and distinct array elements cannot alias.

use crate::{Bounds, Float, Key, Kind, Location, Node, Scalar, Statement, TableOrigin, TomlError};
use std::collections::BTreeMap;
use std::ops::Range;

type Parsed = (Vec<Node>, Vec<Statement>, Vec<Key>);
pub(crate) fn parse(text: &str, bounds: Bounds) -> Result<Parsed, TomlError> {
    let mut reader = Reader {
        text,
        at: 0,
        bounds,
        nodes: Vec::new(),
        statements: Vec::new(),
        keys: Vec::new(),
        names: BTreeMap::new(),
        retained: 0,
    };
    if text.len() as u64 > bounds.max_source_bytes {
        return reader.limit("source bytes", bounds.max_source_bytes);
    }
    reader.add(None, None, Kind::Table(TableOrigin::Root), 0..text.len())?;
    let mut current = 0;
    loop {
        reader.space(true)?;
        if reader.peek().is_none() {
            return Ok((reader.nodes, reader.statements, reader.keys));
        }
        let start = reader.at;
        let (keys, node, header) = if reader.peek() == Some(b'[') {
            reader.at += 1;
            let array = reader.eat(b'[');
            let keys = reader.keys()?;
            reader.expect(b']')?;
            if array {
                reader.expect(b']')?;
            }
            let node = reader.header(&keys, array, start..reader.at)?;
            current = node;
            (keys, node, true)
        } else {
            let keys = reader.keys()?;
            reader.expect(b'=')?;
            reader.horizontal();
            let (parent, key) = reader.assignment(current, &keys)?;
            let node = reader.value(parent, Some(key))?;
            (keys, node, false)
        };
        reader.charge(
            (size_of::<Statement>()
                + keys
                    .iter()
                    .map(|k| size_of::<Key>() + k.decoded.len())
                    .sum::<usize>()) as u64,
        )?;
        if reader.statements.len() as u64 >= u64::from(bounds.max_nodes) {
            return reader.limit("statements", u64::from(bounds.max_nodes));
        }
        reader.statements.push(Statement {
            span: start..reader.at,
            keys,
            node,
            header,
        });
        reader.horizontal();
        if reader.peek() == Some(b'#') {
            reader.comment()?;
        }
        if reader.peek().is_some() {
            reader.newline()?;
        }
    }
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
    bounds: Bounds,
    nodes: Vec<Node>,
    statements: Vec<Statement>,
    keys: Vec<Key>,
    names: BTreeMap<(usize, String), usize>,
    retained: u64,
}

impl Reader<'_> {
    fn location(&self) -> Location {
        let prefix = &self.text[..self.at];
        Location {
            byte: self.at,
            line: prefix.bytes().filter(|&b| b == b'\n').count() + 1,
            column: prefix
                .rsplit('\n')
                .next()
                .map_or(1, |line| line.chars().count() + 1),
        }
    }
    fn fail<T>(&self, expected: &'static str) -> Result<T, TomlError> {
        Err(TomlError::Syntax {
            location: self.location(),
            expected,
        })
    }
    fn limit<T>(&self, resource: &'static str, limit: u64) -> Result<T, TomlError> {
        Err(TomlError::Limit {
            location: self.location(),
            resource,
            limit,
        })
    }
    fn charge(&mut self, bytes: u64) -> Result<(), TomlError> {
        self.retained = self.retained.checked_add(bytes).ok_or(TomlError::Limit {
            location: self.location(),
            resource: "model bytes",
            limit: self.bounds.max_model_bytes,
        })?;
        if self.retained > self.bounds.max_model_bytes {
            return self.limit("model bytes", self.bounds.max_model_bytes);
        }
        Ok(())
    }
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }
    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, byte: u8) -> Result<(), TomlError> {
        if self.eat(byte) {
            Ok(())
        } else {
            self.fail("required TOML delimiter")
        }
    }
    fn horizontal(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.at += 1;
        }
    }
    fn newline(&mut self) -> Result<(), TomlError> {
        if self.eat(b'\n') || (self.eat(b'\r') && self.eat(b'\n')) {
            Ok(())
        } else {
            self.fail("LF or CRLF after a statement")
        }
    }
    fn comment(&mut self) -> Result<(), TomlError> {
        while let Some(byte) = self.peek() {
            if byte == b'\n'
                || (byte == b'\r' && self.text.as_bytes().get(self.at + 1) == Some(&b'\n'))
            {
                break;
            }
            if byte < 0x20 && byte != b'\t' || byte == 0x7f {
                return self.fail("no control characters in comments");
            }
            let character = self.text[self.at..]
                .chars()
                .next()
                .expect("peek found a UTF-8 scalar");
            self.at += character.len_utf8();
        }
        Ok(())
    }
    fn space(&mut self, multiline: bool) -> Result<(), TomlError> {
        loop {
            self.horizontal();
            if !multiline {
                return Ok(());
            }
            match self.peek() {
                Some(b'#') => self.comment()?,
                Some(b'\n' | b'\r') => self.newline()?,
                _ => return Ok(()),
            }
        }
    }
    fn keys(&mut self) -> Result<Vec<Key>, TomlError> {
        let mut keys = Vec::new();
        loop {
            self.horizontal();
            let start = self.at;
            let decoded = match self.peek() {
                Some(b'"' | b'\'') => self.string(false)?,
                _ => {
                    while self
                        .peek()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
                    {
                        self.at += 1;
                    }
                    if self.at == start {
                        return self.fail("a bare or quoted key");
                    }
                    self.text[start..self.at].to_owned()
                }
            };
            if keys.len() as u64 >= u64::from(self.bounds.max_depth) {
                return self.limit("key depth", u64::from(self.bounds.max_depth));
            }
            self.charge((size_of::<Key>() * 2 + decoded.len() * 2) as u64)?;
            let key = Key {
                decoded,
                span: start..self.at,
            };
            self.keys.push(key.clone());
            keys.push(key);
            self.horizontal();
            if !self.eat(b'.') {
                return Ok(keys);
            }
        }
    }
    fn string(&mut self, allow_multiline: bool) -> Result<String, TomlError> {
        let quote = self.peek().expect("string called at a quote");
        self.at += 1;
        let multiline = self.text.as_bytes().get(self.at..self.at + 2) == Some(&[quote, quote]);
        if multiline {
            if !allow_multiline {
                return self.fail("quoted keys cannot use multiline strings");
            }
            self.at += 2;
            if matches!(self.peek(), Some(b'\n' | b'\r')) {
                self.newline()?;
            }
        }
        let mut result = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return self.fail("a closing string delimiter");
            };
            if byte == quote {
                let count = self.text.as_bytes()[self.at..]
                    .iter()
                    .take_while(|&&b| b == quote)
                    .count();
                if !multiline {
                    self.at += 1;
                    return Ok(result);
                }
                if count >= 3 {
                    if count > 5 {
                        return self.fail("at most two quotes before a multiline delimiter");
                    }
                    for _ in 3..count {
                        result.push(char::from(quote));
                    }
                    self.at += count;
                    return Ok(result);
                }
                for _ in 0..count {
                    result.push(char::from(quote));
                }
                self.at += count;
            } else if byte == b'\\' && quote == b'"' {
                let escape_at = self.at;
                self.at += 1;
                if multiline && matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                    self.horizontal();
                    self.newline()?;
                    loop {
                        self.horizontal();
                        if matches!(self.peek(), Some(b'\n' | b'\r')) {
                            self.newline()?;
                        } else {
                            break;
                        }
                    }
                    continue;
                }
                let scalar = match self.peek() {
                    Some(b'u' | b'U') => {
                        let (scalar, used) =
                            purrdf_lex::terminals::decode_uchar(&self.text.as_bytes()[escape_at..])
                                .map_err(|_| TomlError::Syntax {
                                    location: self.location(),
                                    expected: "a Unicode scalar escape",
                                })?;
                        self.at = escape_at + used;
                        scalar
                    }
                    Some(b'x') => {
                        let digits = self.text.as_bytes().get(self.at + 1..self.at + 3).ok_or(
                            TomlError::Syntax {
                                location: self.location(),
                                expected: "two hexadecimal digits",
                            },
                        )?;
                        let encoded = [b'\\', b'u', b'0', b'0', digits[0], digits[1]];
                        let (scalar, _) =
                            purrdf_lex::terminals::decode_uchar(&encoded).map_err(|_| {
                                TomlError::Syntax {
                                    location: self.location(),
                                    expected: "two hexadecimal digits",
                                }
                            })?;
                        self.at += 3;
                        scalar
                    }
                    Some(b'e') => {
                        self.at += 1;
                        '\u{1b}'
                    }
                    Some(b'\'') | None => return self.fail("a TOML basic-string escape"),
                    Some(letter) => {
                        let scalar = purrdf_lex::terminals::echar_value(letter).ok_or(
                            TomlError::Syntax {
                                location: self.location(),
                                expected: "a TOML basic-string escape",
                            },
                        )?;
                        self.at += 1;
                        scalar
                    }
                };
                result.push(scalar);
            } else if matches!(byte, b'\n' | b'\r') {
                if !multiline {
                    return self.fail("single-line string without a newline");
                }
                self.newline()?;
                result.push('\n');
            } else {
                if byte < 0x20 && byte != b'\t' || byte == 0x7f {
                    return self.fail("an escaped string control character");
                }
                let scalar = self.text[self.at..]
                    .chars()
                    .next()
                    .expect("peek found a scalar");
                self.at += scalar.len_utf8();
                result.push(scalar);
            }
            if result.len() as u64 > self.bounds.max_model_bytes.saturating_sub(self.retained) {
                return self.limit("model bytes", self.bounds.max_model_bytes);
            }
        }
    }
    fn add(
        &mut self,
        parent: Option<usize>,
        key: Option<Key>,
        kind: Kind,
        span: Range<usize>,
    ) -> Result<usize, TomlError> {
        if self.nodes.len() as u64 >= u64::from(self.bounds.max_nodes) {
            return self.limit("nodes", u64::from(self.bounds.max_nodes));
        }
        let depth = parent.map_or(0, |p| self.nodes[p].depth + 1);
        if depth > self.bounds.max_depth {
            return self.limit("depth", u64::from(self.bounds.max_depth));
        }
        let mut path = parent.map_or_else(String::new, |p| self.nodes[p].path.clone());
        if let Some(parent) = parent {
            match &key {
                Some(key) => purrdf_lex::json_pointer::push_token(&mut path, &key.decoded),
                None => {
                    use std::fmt::Write as _;
                    write!(path, "/{}", self.nodes[parent].children.len())
                        .expect("writing into String");
                }
            }
        }
        let scalar_bytes = match &kind {
            Kind::Scalar(
                Scalar::String(s)
                | Scalar::OffsetDateTime(s)
                | Scalar::LocalDateTime(s)
                | Scalar::LocalDate(s)
                | Scalar::LocalTime(s),
            ) => s.len(),
            _ => 0,
        };
        self.charge(
            (size_of::<Node>() * 3
                + path.len()
                + scalar_bytes
                + key.as_ref().map_or(0, |key| key.decoded.len() * 2)) as u64,
        )?;
        let index = self.nodes.len();
        let ordinal = parent.map_or(0, |parent| self.nodes[parent].children.len());
        if let Some(parent) = parent {
            if let Some(key) = &key {
                if self.names.contains_key(&(parent, key.decoded.clone())) {
                    return self.fail("a key or table may only be defined once");
                }
                self.names.insert((parent, key.decoded.clone()), index);
            }
            self.nodes[parent].children.push(index);
        }
        self.nodes.push(Node {
            kind,
            span,
            key,
            parent,
            ordinal,
            children: Vec::new(),
            path,
            depth,
            sealed: false,
        });
        Ok(index)
    }
    fn child(&self, parent: usize, key: &Key) -> Option<usize> {
        self.names.get(&(parent, key.decoded.clone())).copied()
    }
    fn descend(
        &mut self,
        mut parent: usize,
        keys: &[Key],
        dotted: bool,
    ) -> Result<usize, TomlError> {
        for key in keys {
            if self.nodes[parent].sealed {
                return self.fail("inline tables cannot be extended outside their braces");
            }
            let node = match self.child(parent, key) {
                Some(node) => node,
                None => self.add(
                    Some(parent),
                    Some(key.clone()),
                    Kind::Table(if dotted {
                        TableOrigin::Dotted
                    } else {
                        TableOrigin::Implicit
                    }),
                    key.span.clone(),
                )?,
            };
            match self.nodes[node].kind {
                Kind::ArrayOfTables if !dotted => {
                    parent = *self.nodes[node]
                        .children
                        .last()
                        .expect("an array-of-tables has its first element");
                }
                Kind::Table(origin)
                    if !self.nodes[node].sealed && (!dotted || origin != TableOrigin::Explicit) =>
                {
                    parent = node;
                }
                _ => {
                    return self.fail("a traversable table that has not been explicitly redefined");
                }
            }
        }
        Ok(parent)
    }
    fn assignment(&mut self, parent: usize, keys: &[Key]) -> Result<(usize, Key), TomlError> {
        let (key, prefix) = keys.split_last().expect("keys are nonempty");
        let parent = self.descend(parent, prefix, true)?;
        if self.nodes[parent].sealed || self.child(parent, key).is_some() {
            return self.fail("an unused key in an open table");
        }
        Ok((parent, key.clone()))
    }
    fn header(
        &mut self,
        keys: &[Key],
        array: bool,
        span: Range<usize>,
    ) -> Result<usize, TomlError> {
        let (key, prefix) = keys.split_last().expect("header keys are nonempty");
        let parent = self.descend(0, prefix, false)?;
        if self.nodes[parent].sealed {
            return self.fail("an open table for the declaration");
        }
        let existing = self.child(parent, key);
        if array {
            let array = match existing {
                Some(node) if self.nodes[node].kind == Kind::ArrayOfTables => node,
                Some(_) => {
                    return self
                        .fail("array-of-table declaration cannot replace a table or static array");
                }
                None => self.add(
                    Some(parent),
                    Some(key.clone()),
                    Kind::ArrayOfTables,
                    span.clone(),
                )?,
            };
            self.add(
                Some(array),
                None,
                Kind::Table(TableOrigin::ArrayElement),
                span,
            )
        } else {
            match existing {
                Some(node) if self.nodes[node].kind == Kind::Table(TableOrigin::Implicit) => {
                    self.nodes[node].kind = Kind::Table(TableOrigin::Explicit);
                    Ok(node)
                }
                Some(_) => self.fail("a table must not be declared twice or replace a value"),
                None => self.add(
                    Some(parent),
                    Some(key.clone()),
                    Kind::Table(TableOrigin::Explicit),
                    span,
                ),
            }
        }
    }
    fn start_value(&mut self, parent: usize, key: Option<Key>) -> Result<(usize, bool), TomlError> {
        let start = self.at;
        let kind = match self.peek() {
            Some(b'[') => {
                self.at += 1;
                Kind::Array
            }
            Some(b'{') => {
                self.at += 1;
                Kind::Table(TableOrigin::Inline)
            }
            Some(b'"' | b'\'') => Kind::Scalar(Scalar::String(self.string(true)?)),
            _ => {
                while let Some(byte) = self.peek() {
                    if matches!(byte, b'\t' | b'\n' | b'\r' | b',' | b']' | b'}' | b'#') {
                        break;
                    }
                    if byte == b' ' {
                        let date_separator = self.at == start + 10
                            && self.text.as_bytes().get(start + 4) == Some(&b'-')
                            && self.text.as_bytes().get(self.at + 3) == Some(&b':');
                        if !date_separator {
                            break;
                        }
                    }
                    let scalar = self.text[self.at..]
                        .chars()
                        .next()
                        .expect("peek found a scalar");
                    self.at += scalar.len_utf8();
                }
                let scalar = atom(&self.text[start..self.at]).ok_or(TomlError::Syntax {
                    location: self.location(),
                    expected: "a TOML scalar value",
                })?;
                Kind::Scalar(scalar)
            }
        };
        let container = !matches!(kind, Kind::Scalar(_));
        self.add(Some(parent), key, kind, start..self.at)
            .map(|node| (node, container))
    }
    fn value(&mut self, parent: usize, key: Option<Key>) -> Result<usize, TomlError> {
        let (root, container) = self.start_value(parent, key)?;
        if !container {
            return Ok(root);
        }
        // The boolean records whether a value was completed and needs a comma.
        let mut frames = vec![(root, false)];
        while let Some(&(node, after)) = frames.last() {
            self.space(true)?;
            let inline = matches!(self.nodes[node].kind, Kind::Table(_));
            let close = if inline { b'}' } else { b']' };
            if self.eat(close) {
                self.nodes[node].span.end = self.at;
                if inline {
                    for nested in &mut self.nodes[node..] {
                        nested.sealed = true;
                    }
                }
                frames.pop();
                continue;
            }
            if after {
                self.expect(b',')?;
                frames.last_mut().expect("the current frame exists").1 = false;
                continue;
            }
            let (parent, key) = if inline {
                let keys = self.keys()?;
                self.expect(b'=')?;
                self.horizontal();
                let (parent, key) = self.assignment(node, &keys)?;
                (parent, Some(key))
            } else {
                (node, None)
            };
            let (child, container) = self.start_value(parent, key)?;
            frames.last_mut().expect("the current frame exists").1 = true;
            if container {
                frames.push((child, false));
            }
        }
        Ok(root)
    }
}

fn digit_group(text: &str, radix: u32) -> bool {
    !text.is_empty()
        && text
            .split('_')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii() && c.is_digit(radix)))
}

fn atom(raw: &str) -> Option<Scalar> {
    match raw {
        "true" => return Some(Scalar::Boolean(true)),
        "false" => return Some(Scalar::Boolean(false)),
        "inf" | "+inf" | "-inf" => {
            return Some(Scalar::Float(Float::Infinity {
                negative: raw.starts_with('-'),
            }));
        }
        "nan" | "+nan" | "-nan" => {
            return Some(Scalar::Float(Float::Nan {
                negative: raw.starts_with('-'),
            }));
        }
        _ => {}
    }
    if raw.as_bytes().get(4) == Some(&b'-') || raw.as_bytes().get(2) == Some(&b':') {
        return temporal(raw);
    }
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = raw.strip_prefix(prefix) {
            if !digit_group(digits, radix) {
                return None;
            }
            return i64::from_str_radix(&digits.replace('_', ""), radix)
                .ok()
                .map(Scalar::Integer);
        }
    }
    let unsigned = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(at) => (&unsigned[..at], Some(&unsigned[at + 1..])),
        None => (unsigned, None),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (mantissa, None),
    };
    if !digit_group(integer, 10) || integer.starts_with('0') && integer.len() > 1 {
        return None;
    }
    if fraction.is_some_and(|part| !digit_group(part, 10))
        || exponent
            .is_some_and(|part| !digit_group(part.strip_prefix(['+', '-']).unwrap_or(part), 10))
    {
        return None;
    }
    let normalized = raw.replace('_', "");
    if fraction.is_none() && exponent.is_none() {
        return normalized.parse().ok().map(Scalar::Integer);
    }
    let value: f64 = normalized.parse().ok()?;
    Some(Scalar::Float(if value.is_finite() {
        Float::Finite(value)
    } else {
        Float::Infinity {
            negative: value.is_sign_negative(),
        }
    }))
}

fn temporal(raw: &str) -> Option<Scalar> {
    use purrdf_xsd::rfc3339::{self, LeapSecond, Separator};
    if raw.len() == 10 && raw.as_bytes().get(4) == Some(&b'-') {
        rfc3339::parse_date(raw).ok()?;
        return Some(Scalar::LocalDate(raw.to_owned()));
    }
    let time_at = if raw.as_bytes().get(4) == Some(&b'-') {
        11
    } else {
        0
    };
    let time = raw.get(time_at..)?;
    if time.len() < 5 {
        return None;
    }
    let mut normalized = raw.to_owned();
    // TOML 1.1 permits omitted seconds; preserve that choice in the source span.
    if time.len() == 5 || matches!(time.as_bytes().get(5), Some(b'Z' | b'z' | b'+' | b'-')) {
        normalized.insert_str(time_at + 5, ":00");
    }
    let offset = normalized[time_at..].contains(['Z', 'z', '+', '-']);
    if time_at == 0 {
        if offset {
            return None;
        }
        rfc3339::parse_time(&format!("{normalized}Z")).ok()?;
        Some(Scalar::LocalTime(normalized))
    } else if offset {
        rfc3339::parse(&normalized, Separator::GtsSpaceAllowed, LeapSecond::AnyDay).ok()?;
        Some(Scalar::OffsetDateTime(normalized))
    } else {
        rfc3339::parse(
            &format!("{normalized}Z"),
            Separator::GtsSpaceAllowed,
            LeapSecond::AnyDay,
        )
        .ok()?;
        Some(Scalar::LocalDateTime(normalized))
    }
}

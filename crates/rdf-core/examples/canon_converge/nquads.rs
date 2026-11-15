// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A reader of canonical line text into masked canonical lines, for the
//! invariance checks and the shared vectors. Lines are plain N-Quads, or the
//! canonical line forms (`@r`, `@a`, `@a*k`, `@g`, `@s`) with triple terms at any
//! depth. Everything is kept as written except a blank label, which becomes the
//! mask and joins the line's tuple, and the count of an `@a*k` line, which
//! becomes the line's count.

use purrdf_core::FastMap;

use crate::colour::{MARK, RawLine};

/// The masked lines of a document of lines.
pub(crate) fn read(text: &str) -> Vec<RawLine> {
    let mut ids: FastMap<String, u32> = FastMap::default();
    let mut lines = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (count, body) = if let Some(rest) = line.strip_prefix("@a*") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (
                digits.parse::<u32>().expect("a count"),
                format!("@a{}", &rest[digits.len()..]),
            )
        } else {
            (1, line.to_owned())
        };
        let mut t = String::with_capacity(body.len());
        let mut tuple = Vec::new();
        let b = body.as_bytes();
        let mut i = 0;
        while i < b.len() {
            let at_token = i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'(');
            if body[i..].starts_with("<<(") {
                t.push_str("<<(");
                i += 3;
            } else if b[i] == b'<' {
                let end = i + body[i..].find('>').expect("a closed IRI") + 1;
                t.push_str(&body[i..end]);
                i = end;
            } else if b[i] == b'"' {
                let mut j = i + 1;
                while b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                t.push_str(&body[i..=j]);
                i = j + 1;
            } else if at_token && body[i..].starts_with("_:") {
                let mut j = i + 2;
                while j < b.len() && !matches!(b[j], b' ' | b'\t' | b')') {
                    j += 1;
                }
                let next = ids.len() as u32;
                tuple.push(*ids.entry(body[i + 2..j].to_owned()).or_insert(next));
                t.push(MARK);
                i = j;
            } else {
                let ch = body[i..].chars().next().expect("a character");
                t.push(ch);
                i += ch.len_utf8();
            }
        }
        lines.push(RawLine {
            text: t,
            tuple,
            count,
        });
    }
    lines
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The gate's gettext text boundary. Unknown escapes retain their spelling.

use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) struct Entry {
    pub(super) line: usize,
    pub(super) id: String,
    pub(super) translated: String,
    pub(super) active: bool,
}

fn quoted(text: &str) -> Result<String, String> {
    let body = text
        .trim()
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| format!("not a quoted PO string: {text:?}"))?;
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            return Err("unescaped quote in PO string".to_owned());
        }
        if ch == '\\' {
            match chars.peek() {
                Some('\\' | '"' | 'n' | 't') => {
                    out.push(match chars.next().expect("peeked") {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    continue;
                }
                None => return Err("trailing backslash in PO string".to_owned()),
                _ => {}
            }
        }
        out.push(ch);
    }
    Ok(out)
}

#[derive(Default)]
struct Pending {
    fields: BTreeMap<String, String>,
    current: Option<String>,
    fuzzy: bool,
    obsolete: bool,
    line: usize,
}

impl Pending {
    fn flush(&mut self, entries: &mut Vec<Entry>) -> Result<(), String> {
        if !self.fields.is_empty() {
            let id = self
                .fields
                .get("msgid")
                .ok_or("PO entry has fields but no msgid")?;
            let translated = self
                .fields
                .get("msgstr")
                .or_else(|| {
                    self.fields
                        .iter()
                        .find(|(key, _)| key.starts_with("msgstr["))
                        .map(|(_, value)| value)
                })
                .ok_or("PO entry has no msgstr")?;
            if !id.is_empty() || self.fields.contains_key("msgctxt") {
                entries.push(Entry {
                    line: self.line,
                    id: id.clone(),
                    translated: translated.clone(),
                    active: !translated.is_empty() && !self.fuzzy && !self.obsolete,
                });
            }
        }
        *self = Self::default();
        Ok(())
    }
}

pub(super) fn parse(text: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut pending = Pending::default();
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let mut line = raw.trim();
        if line.is_empty() {
            pending.flush(&mut entries)?;
            continue;
        }
        let obsolete = line.starts_with("#~");
        if obsolete {
            line = line[2..].trim();
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            if pending.current.is_some() {
                pending.flush(&mut entries)?;
            }
            if let Some(flags) = line.strip_prefix("#,") {
                pending.fuzzy |= flags.split(',').any(|flag| flag.trim() == "fuzzy");
            }
            continue;
        }
        if line.starts_with('"') {
            let current = pending
                .current
                .as_ref()
                .ok_or_else(|| format!("line {number}: continuation without keyword"))?;
            pending
                .fields
                .get_mut(current)
                .expect("current exists")
                .push_str(&quoted(line).map_err(|error| format!("line {number}: {error}"))?);
            continue;
        }
        let (keyword, value) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("line {number}: missing PO value"))?;
        let plural = keyword
            .strip_prefix("msgstr[")
            .and_then(|s| s.strip_suffix(']'))
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
        if !matches!(keyword, "msgctxt" | "msgid" | "msgid_plural" | "msgstr") && !plural {
            return Err(format!("line {number}: unknown PO keyword {keyword:?}"));
        }
        if matches!(keyword, "msgctxt" | "msgid")
            && (pending.fields.contains_key(keyword)
                || pending.fields.keys().any(|key| key.starts_with("msgstr")))
        {
            pending.flush(&mut entries)?;
        }
        if pending.fields.is_empty() {
            pending.line = number;
            pending.obsolete = obsolete;
        }
        if pending.fields.contains_key(keyword) {
            return Err(format!("line {number}: duplicate PO keyword {keyword}"));
        }
        pending.current = Some(keyword.to_owned());
        pending.fields.insert(
            keyword.to_owned(),
            quoted(value).map_err(|error| format!("line {number}: {error}"))?,
        );
    }
    pending.flush(&mut entries)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuations_context_plural_and_suppression() {
        let entries = parse("msgid \"\"\nmsgstr \"header\"\n\nmsgctxt \"context\"\nmsgid \"a\"\n\"b\"\nmsgid_plural \"abs\"\nmsgstr[1] \"second\"\nmsgstr[0] \"first\\n\\t\\q\\\\\\\"\"\n\n#, fuzzy\nmsgid \"f\"\nmsgstr \"bad\"\n\n#~ msgid \"o\"\n#~ msgstr \"bad\"\n\nmsgid \"e\"\nmsgstr \"\"").unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].line, 4);
        assert_eq!(entries[0].id, "ab");
        assert_eq!(entries[0].translated, "first\n\t\\q\\\"");
        assert!(entries[0].active);
        assert!(entries[1..].iter().all(|entry| !entry.active));
    }

    #[test]
    fn malformed_inputs_fail() {
        for text in [
            "\"orphan\"",
            "msgid x",
            "msgid \"x\"",
            "wrong \"x\"",
            "msgid \"x\"\nmsgstr[wat] \"x\"",
            "msgid \"x\"\nmsgstr \"x\" trailing",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
}

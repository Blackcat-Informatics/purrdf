// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared COSE header label, duplicate and common-value judgments.

use purrdf_lex::cbor::{Integer, Value};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum HeaderLabel<'a> {
    Integer(Integer),
    Text(&'a str),
}
impl<'a> HeaderLabel<'a> {
    pub(super) fn from_value(value: &'a Value) -> Option<Self> {
        match value {
            Value::Integer(integer) => Some(Self::Integer(*integer)),
            Value::Text(text) => Some(Self::Text(text)),
            _ => None,
        }
    }
}

pub(super) fn header_labels(headers: &[(Value, Value)]) -> Option<BTreeSet<HeaderLabel<'_>>> {
    let mut labels = BTreeSet::new();
    for (label, value) in headers {
        if !labels.insert(HeaderLabel::from_value(label)?) {
            return None;
        }
        // RFC 9052 common-header value types, even for ignored metadata.
        match label.as_integer().map(Integer::get) {
            Some(1) if value.as_integer().is_none() => return None,
            Some(2) if value.as_array().is_none_or(Vec::is_empty) => {
                return None;
            }
            Some(3) if !valid_content_type(value) => {
                return None;
            }
            Some(4..=6) if value.as_bytes().is_none() => return None,
            // These GTS envelopes implement no counter-signature layer.
            Some(7) => return None,
            _ => {}
        }
    }
    Some(labels)
}

/// RFC 9052 §3.1: a nonnegative CoAP content format, or exactly two
/// RFC 6838 §4.2 restricted names. Parameter syntax is not part of this
/// COSE textual pair; no trimming, case normalization or slash repair occurs.
fn valid_content_type(value: &Value) -> bool {
    match value {
        Value::Integer(integer) => integer.get() >= 0,
        Value::Text(text) => text
            .split_once('/')
            .is_some_and(|(kind, subtype)| restricted_name(kind) && restricted_name(subtype)),
        _ => false,
    }
}

/// The exact ABNF `ALPHA / DIGIT` followed by up to 126 of those characters
/// or `! # $ & - ^ _ . +`. Length is bytes; every admitted byte is ASCII.
fn restricted_name(name: &str) -> bool {
    let Some((first, rest)) = name.as_bytes().split_first() else {
        return false;
    };
    name.len() <= 127
        && first.is_ascii_alphanumeric()
        && rest.iter().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'-' | b'^' | b'_' | b'.' | b'+'
                )
        })
}

pub(super) fn header_value(headers: &[(Value, Value)], label: i64) -> Option<&Value> {
    headers
        .iter()
        .find(|(key, _)| key.as_integer() == Some(label.into()))
        .map(|(_, value)| value)
}

pub(super) fn critical_labels<'a>(
    protected: &'a [(Value, Value)],
    labels: &BTreeSet<HeaderLabel<'a>>,
) -> Option<BTreeSet<HeaderLabel<'a>>> {
    let mut critical = BTreeSet::new();
    if let Some(value) = header_value(protected, 2) {
        let values = value.as_array()?;
        if values.is_empty() {
            return None;
        }
        for value in values {
            let label = HeaderLabel::from_value(value)?;
            if !critical.insert(label) || !labels.contains(&label) {
                return None;
            }
        }
    }
    Some(critical)
}

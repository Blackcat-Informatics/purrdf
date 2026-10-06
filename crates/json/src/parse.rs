// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ordered occurrence model over the workspace JSON reader
//! ([`purrdf_lex::json::occurrences`], RFC 8259 §§2, 4–7), with Unicode scalar
//! member names for the RFC 6901 §3 paths this crate builds.

use std::{borrow::Cow, fmt::Write as _, ops::Range};

use purrdf_lex::json::{self as lex, ErrorKind, Limits};
use purrdf_lex::json_escape::{self, JsonEscapeErrorKind};
use purrdf_lex::json_pointer;

use crate::{Bounds, JsonError, Kind, Value};

pub(crate) fn parse(
    text: &str,
    bounds: Bounds,
) -> Result<(Vec<Value>, Vec<Range<usize>>), JsonError> {
    let limits = Limits {
        max_depth: usize::from(bounds.max_depth),
        max_values: u64::from(bounds.max_values),
        ..Limits::DEFAULT
    };
    let occurrences = lex::occurrences(text, limits).map_err(|error| refusal(error, bounds))?;
    let mut values: Vec<Value> = Vec::with_capacity(occurrences.len());
    let mut pointer_bytes = 0_u64;
    for occurrence in occurrences {
        let mut path = match occurrence.parent {
            Some(parent) => values[parent].path.clone(),
            None => String::new(),
        };
        if occurrence.parent.is_some() {
            match &occurrence.key {
                Some(key) => {
                    let name = unescape(&text[key.clone()], key.start)?;
                    json_pointer::push_token(&mut path, &name);
                }
                // Writing to String is infallible and avoids a temporary index string.
                None => write!(path, "/{}", occurrence.ordinal).expect("writing into a String"),
            }
        }
        pointer_bytes += path.len() as u64;
        if pointer_bytes > bounds.max_pointer_bytes {
            return Err(JsonError::Limit {
                resource: "pointer bytes",
                limit: bounds.max_pointer_bytes,
            });
        }
        values.push(Value {
            span: occurrence.span,
            kind: kind(occurrence.kind),
            path,
            parent: occurrence.parent,
            ordinal: occurrence.ordinal,
            size: u32::try_from(occurrence.size)
                .expect("the profile bounds the total value count below u32::MAX"),
        });
    }
    let mut runs = Vec::new();
    let mut at = 0;
    for value in values.iter().filter(|value| value.kind.is_scalar()) {
        debug_assert!(value.span.start >= at, "scalar spans follow source order");
        if value.span.start > at {
            runs.push(at..value.span.start);
        }
        at = value.span.end;
    }
    if at < text.len() {
        runs.push(at..text.len());
    }
    Ok((values, runs))
}

const fn kind(kind: lex::Kind) -> Kind {
    match kind {
        lex::Kind::Null => Kind::Null,
        lex::Kind::True => Kind::True,
        lex::Kind::False => Kind::False,
        lex::Kind::Number => Kind::Number,
        lex::Kind::String => Kind::String,
        lex::Kind::Array => Kind::Array,
        lex::Kind::Object => Kind::Object,
    }
}

/// The reader's refusal as this crate's typed error.
fn refusal(error: lex::Error, bounds: Bounds) -> JsonError {
    let at = error.offset();
    match error.kind() {
        ErrorKind::Interrupted => JsonError::Interrupted { at },
        ErrorKind::Expected(expected) => JsonError::Syntax { at, expected },
        ErrorKind::RawControl => JsonError::Syntax {
            at,
            expected: "an escaped control character",
        },
        ErrorKind::Escape(JsonEscapeErrorKind::BadHex) => JsonError::Syntax {
            at,
            expected: "four hexadecimal digits",
        },
        ErrorKind::Escape(_) => JsonError::Syntax {
            at,
            expected: "a JSON escape",
        },
        // The caller hands the reader a `&str`.
        ErrorKind::InvalidUtf8 => JsonError::InvalidUtf8 { valid_up_to: at },
        ErrorKind::Trailing => JsonError::Trailing { at },
        ErrorKind::Depth { .. } => JsonError::Limit {
            resource: "container depth",
            limit: u64::from(bounds.max_depth),
        },
        ErrorKind::Values { .. } => JsonError::Limit {
            resource: "value occurrences",
            limit: u64::from(bounds.max_values),
        },
        // Neither bound is set: occurrences decode no string and keep repeats.
        ErrorKind::StringBytes { .. } | ErrorKind::DuplicateMember => JsonError::Syntax {
            at,
            expected: "a JSON value",
        },
    }
}

// The reader validated each escape's syntax. Member names additionally require
// paired surrogate escapes for a representable RFC 6901 pointer; a value keeps
// its text raw, and RFC 8259's grammar admits an unpaired surrogate there.
fn unescape(raw: &str, at: usize) -> Result<Cow<'_, str>, JsonError> {
    json_escape::unescape(raw).map_err(|error| match error.kind {
        JsonEscapeErrorKind::UnpairedHigh | JsonEscapeErrorKind::UnpairedLow => {
            JsonError::LoneSurrogate { at }
        }
        JsonEscapeErrorKind::Truncated
        | JsonEscapeErrorKind::BadEscape
        | JsonEscapeErrorKind::BadHex => JsonError::Syntax {
            at,
            expected: "a JSON escape",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::{Bounds, JsonError, Kind};

    #[test]
    fn a_lone_surrogate_member_name_is_refused_and_a_lone_surrogate_value_is_kept() {
        assert!(matches!(
            parse(r#"{"\ud800":1}"#, Bounds::standard()),
            Err(JsonError::LoneSurrogate { at: 2 })
        ));
        let (values, _) = parse(r#"{"😀":"\ud800"}"#, Bounds::standard()).unwrap();
        assert_eq!(values[1].path, "/\u{1f600}");
        assert_eq!(values[1].kind, Kind::String);
    }

    #[test]
    fn paths_escape_member_names_and_index_elements() {
        let (values, runs) = parse(r#" {"a/b":[true,{"~":null}]} "#, Bounds::standard()).unwrap();
        let paths: Vec<&str> = values.iter().map(|value| value.path.as_str()).collect();
        assert_eq!(paths, ["", "/a~1b", "/a~1b/0", "/a~1b/1", "/a~1b/1/~0"]);
        assert_eq!(values[2].span, 9..13);
        assert_eq!(values[3].size, 1);
        assert_eq!(runs.first(), Some(&(0..9)));
    }

    #[test]
    fn the_pointer_budget_refuses_past_its_bound_and_accepts_at_it() {
        let at_bound = Bounds {
            max_pointer_bytes: 4,
            ..Bounds::standard()
        };
        assert!(parse(r#"{"abc":1}"#, at_bound).is_ok());
        let below = Bounds {
            max_pointer_bytes: 3,
            ..Bounds::standard()
        };
        assert!(matches!(
            parse(r#"{"abc":1}"#, below),
            Err(JsonError::Limit {
                resource: "pointer bytes",
                ..
            })
        ));
    }
}

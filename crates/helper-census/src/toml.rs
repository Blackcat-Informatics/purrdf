// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Ledger data adaptation over the public TOML parser. Grammar, escaping and
//! table declarations have one home; the census owns its supported field types.

use purrdf_toml::{Kind, Profile, Scalar, SourceDocument, TomlError};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Str(String),
    Bool(bool),
    Int(i64),
    Array(Vec<Self>),
    Table(BTreeMap<String, Self>),
}

impl Value {
    pub(crate) fn to_json(&self) -> purrdf_lex::json::Value {
        use purrdf_lex::json::Value as Json;
        match self {
            Self::Str(text) => Json::from(text),
            Self::Bool(flag) => Json::from(*flag),
            Self::Int(number) => Json::from(*number),
            Self::Array(items) => items.iter().map(Self::to_json).collect(),
            Self::Table(entries) => Json::object(
                entries
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.to_json())),
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Error {
    pub(crate) line: usize,
    pub(crate) message: String,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for Error {}

pub(crate) fn parse(text: &str) -> Result<BTreeMap<String, Value>, Error> {
    let refusal = |error: TomlError| {
        let line = match &error {
            TomlError::Syntax { location, .. } | TomlError::Limit { location, .. } => location.line,
            _ => 1,
        };
        Error {
            line,
            message: error.to_string(),
        }
    };
    let profile = Profile::standard().map_err(refusal)?;
    let document = purrdf_toml::analyze(
        SourceDocument {
            id: "urn:example:helper-ledger",
            bytes: text.as_bytes(),
        },
        &profile,
    )
    .map_err(refusal)?;
    let mut values: Vec<Option<Value>> = (0..document.nodes().len()).map(|_| None).collect();
    for (index, node) in document.nodes().iter().enumerate().rev() {
        let value =
            match &node.kind {
                Kind::Scalar(Scalar::String(text)) => Value::Str(text.clone()),
                Kind::Scalar(Scalar::Integer(number)) => Value::Int(*number),
                Kind::Scalar(Scalar::Boolean(flag)) => Value::Bool(*flag),
                Kind::Scalar(_) => return Err(Error {
                    line: text[..node.span.start]
                        .bytes()
                        .filter(|&byte| byte == b'\n')
                        .count()
                        + 1,
                    message:
                        "the census ledger schema does not admit floating-point or date/time fields"
                            .to_owned(),
                }),
                Kind::Array | Kind::ArrayOfTables => Value::Array(
                    node.children
                        .iter()
                        .map(|&child| {
                            values[child]
                                .take()
                                .expect("reverse arena traversal resolves children first")
                        })
                        .collect(),
                ),
                Kind::Table(_) => Value::Table(
                    node.children
                        .iter()
                        .map(|&child| {
                            (
                                document.nodes()[child]
                                    .key
                                    .as_ref()
                                    .expect("table children have keys")
                                    .decoded
                                    .clone(),
                                values[child]
                                    .take()
                                    .expect("reverse arena traversal resolves children first"),
                            )
                        })
                        .collect(),
                ),
            };
        values[index] = Some(value);
    }
    match values[0]
        .take()
        .expect("the parser always provides its root")
    {
        Value::Table(root) => Ok(root),
        _ => unreachable!("TOML has a table root"),
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
    fn malformed_grammar_and_unsupported_ledger_values_keep_their_line() {
        for (source, line) in [
            ("a = 1.5\n", 1),
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
            "a = { b = 1 }\n",
            "\n\na = \"\"\"x\"\"\"\n",
            "a.b = 1\n",
            "a = 0x10\n",
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

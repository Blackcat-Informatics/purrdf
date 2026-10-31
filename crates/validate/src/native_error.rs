// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact JSON rendering of the native error-inspection protocol for host errors.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::json::Value;

/// A native producer violated the closed inspection protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionError(&'static str);

impl std::fmt::Display for InspectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native error inspection protocol failed: {}", self.0)
    }
}

impl std::error::Error for InspectionError {}

enum Cell {
    Value(Value),
    Child { kind: &'static str, index: usize },
}

enum Node {
    Record {
        code: Option<&'static str>,
        fields: Vec<(&'static str, Cell)>,
        items: Vec<Cell>,
    },
    List {
        length: usize,
        items: Vec<Cell>,
    },
}

impl Node {
    fn record(code: Option<&'static str>) -> Self {
        Self::Record {
            code,
            fields: Vec::new(),
            items: Vec::new(),
        }
    }
}

struct Inspector<'a, 'error> {
    arena: &'a mut Vec<Node>,
    jobs: &'a mut Vec<(&'error dyn StructuredError, usize)>,
    stack: Vec<usize>,
    error: Option<InspectionError>,
}

impl<'error> Inspector<'_, 'error> {
    fn append(&mut self, name: Option<&'static str>, cell: Cell) {
        if self.error.is_some() {
            return;
        }
        let Some(&index) = self.stack.last() else {
            self.error = Some(InspectionError("a field has no enclosing container"));
            return;
        };
        match &mut self.arena[index] {
            Node::Record { fields, items, .. } => match name {
                Some(name) if fields.iter().any(|(previous, _)| *previous == name) => {
                    self.error = Some(InspectionError("a record repeats a field name"));
                }
                Some(name) => fields.push((name, cell)),
                None => items.push(cell),
            },
            Node::List { items, .. } if name.is_none() => items.push(cell),
            Node::List { .. } => {
                self.error = Some(InspectionError("a list entry has a field name"));
            }
        }
    }

    fn child(
        &mut self,
        name: Option<&'static str>,
        kind: &'static str,
        source: &'error dyn StructuredError,
    ) {
        let index = self.arena.len();
        self.arena.push(Node::record(None));
        self.jobs.push((source, index));
        self.append(name, Cell::Child { kind, index });
    }

    fn close(&mut self, list: bool) {
        if self.error.is_some() {
            return;
        }
        if self.stack.len() <= 1 {
            self.error = Some(InspectionError("an end event has no matching begin event"));
            return;
        }
        let index = self.stack.pop().expect("the stack has a nested container");
        if matches!(self.arena[index], Node::List { .. }) != list {
            self.error = Some(InspectionError("a container has the wrong end event"));
        }
    }
}

impl<'error> ErrorVisitor<'error> for Inspector<'_, 'error> {
    fn variant(&mut self, code: &'static str) {
        if self.error.is_some() {
            return;
        }
        match self.stack.last().map(|&index| &mut self.arena[index]) {
            Some(Node::Record { code: previous, .. }) if previous.is_none() => {
                *previous = Some(code);
            }
            _ => {
                self.error = Some(InspectionError(
                    "a variant is outside a record or repeats its code",
                ));
            }
        }
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        if self.error.is_some() {
            return;
        }
        let (kind, value) = match value {
            ErrorValue::Cause(source) => {
                self.child(name, "cause", source);
                return;
            }
            ErrorValue::Record(source) => {
                self.child(name, "record", source);
                return;
            }
            ErrorValue::Term(source) => {
                self.child(name, "term", source);
                return;
            }
            ErrorValue::Absent => ("absent", Value::Null),
            ErrorValue::Unsigned(value) => ("unsigned", Value::from(value.to_string())),
            ErrorValue::Signed(value) => ("signed", Value::from(value.to_string())),
            ErrorValue::ByteCount(value) => ("byte-count", Value::from(value.to_string())),
            ErrorValue::Float32Bits(value) => ("float32-bits", Value::from(value.to_string())),
            ErrorValue::Float64Bits(value) => ("float64-bits", Value::from(value.to_string())),
            ErrorValue::Boolean(value) => ("boolean", Value::from(value)),
            ErrorValue::Character(value) => ("character", Value::from(value.to_string())),
            ErrorValue::Code(value) => ("code", Value::from(value)),
            ErrorValue::Text(value) => ("text", Value::from(value)),
            ErrorValue::Message(value) => ("message", Value::from(value)),
            ErrorValue::Iri(value) => ("iri", Value::from(value)),
            ErrorValue::Bytes(value) => ("bytes", Value::from(purrdf_hash::hex::encode(value))),
        };
        self.append(name, Cell::Value(tagged(kind, value)));
    }

    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        if self.error.is_some() {
            return;
        }
        let index = self.arena.len();
        self.arena.push(Node::List {
            length,
            items: Vec::new(),
        });
        self.append(
            name,
            Cell::Child {
                kind: "list",
                index,
            },
        );
        self.stack.push(index);
    }

    fn end_list(&mut self) {
        self.close(true);
    }

    fn begin_record(&mut self, name: Option<&'static str>, code: &'static str) {
        if self.error.is_some() {
            return;
        }
        let index = self.arena.len();
        self.arena.push(Node::record(Some(code)));
        self.append(
            name,
            Cell::Child {
                kind: "record",
                index,
            },
        );
        self.stack.push(index);
    }

    fn end_record(&mut self) {
        self.close(false);
    }
}

fn tagged(kind: &str, value: Value) -> Value {
    Value::object([("kind", Value::from(kind)), ("value", value)])
}

fn finish_cell(cell: Cell, completed: &mut [Option<Value>]) -> Result<Value, InspectionError> {
    match cell {
        Cell::Value(value) => Ok(value),
        Cell::Child { kind, index } => completed
            .get_mut(index)
            .and_then(Option::take)
            .map(|value| tagged(kind, value))
            .ok_or(InspectionError(
                "a child record is absent or was consumed twice",
            )),
    }
}

/// Render the original closed native fields without parsing `Display` text.
/// Integer and float-bit payloads are decimal strings; byte payloads are
/// lowercase hexadecimal. Compound visits and assembly use heap work lists.
///
/// # Errors
/// Returns an actionable protocol failure rather than publishing partial fields.
pub fn to_json(source: &dyn StructuredError) -> Result<Value, InspectionError> {
    let mut arena = vec![Node::record(None)];
    let mut jobs = vec![(source, 0)];
    while let Some((source, root)) = jobs.pop() {
        let mut inspector = Inspector {
            arena: &mut arena,
            jobs: &mut jobs,
            stack: vec![root],
            error: None,
        };
        source.visit(&mut inspector);
        if let Some(error) = inspector.error {
            return Err(error);
        }
        if inspector.stack.len() != 1 {
            return Err(InspectionError("a visit left a container open"));
        }
    }
    let mut completed: Vec<Option<Value>> = (0..arena.len()).map(|_| None).collect();
    for (index, node) in arena.into_iter().enumerate().rev() {
        let value = match node {
            Node::Record {
                code,
                fields,
                items,
            } => {
                let code = code.ok_or(InspectionError("a record has no variant code"))?;
                let fields = fields
                    .into_iter()
                    .map(|(name, cell)| Ok((name, finish_cell(cell, &mut completed)?)))
                    .collect::<Result<Vec<_>, InspectionError>>()?;
                let items = items
                    .into_iter()
                    .map(|cell| finish_cell(cell, &mut completed))
                    .collect::<Result<Vec<_>, _>>()?;
                Value::object([
                    ("code", Value::from(code)),
                    ("fields", Value::object(fields)),
                    ("items", Value::Array(items)),
                ])
            }
            Node::List { length, items } => {
                if length != items.len() {
                    return Err(InspectionError("a list length disagrees with its entries"));
                }
                Value::Array(
                    items
                        .into_iter()
                        .map(|cell| finish_cell(cell, &mut completed))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
        };
        completed[index] = Some(value);
    }
    completed[0]
        .take()
        .ok_or(InspectionError("the root record is absent"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ListRefusal(usize);

    impl StructuredError for ListRefusal {
        fn visit<'error>(&'error self, visitor: &mut dyn ErrorVisitor<'error>) {
            visitor.variant("native-list-refusal");
            visitor.begin_list(Some("coordinates"), self.0);
            visitor.value(None, ErrorValue::Unsigned(u128::MAX));
            visitor.end_list();
        }
    }

    #[test]
    fn malformed_native_list_refuses_instead_of_publishing_partial_fields() {
        assert!(to_json(&ListRefusal(2)).is_err());
        let admitted = to_json(&ListRefusal(1)).unwrap().to_string();
        assert!(admitted.contains("native-list-refusal"));
        assert!(
            admitted.contains(&format!("\"{}\"", u128::MAX)),
            "{admitted}"
        );
    }
}

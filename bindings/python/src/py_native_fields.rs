// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lossless Python snapshots of the native closed error-inspection algebra.

use purrdf_lex::error::{ErrorValue, ErrorVisitor, StructuredError};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyTuple};

enum Frame<'py> {
    Record {
        name: Option<&'static str>,
        node: Bound<'py, PyDict>,
        fields: Bound<'py, PyDict>,
        items: Vec<Py<PyAny>>,
    },
    List {
        name: Option<&'static str>,
        length: usize,
        items: Vec<Py<PyAny>>,
    },
}

fn record(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let node = PyDict::new(py);
    node.set_item("fields", PyDict::new(py))?;
    Ok(node)
}

fn value_node<'py>(py: Python<'py>, kind: &str, value: Py<PyAny>) -> PyResult<Bound<'py, PyDict>> {
    let node = PyDict::new(py);
    node.set_item("kind", kind)?;
    node.set_item("value", value)?;
    Ok(node)
}

struct Inspector<'a, 'error, 'py> {
    py: Python<'py>,
    stack: Vec<Frame<'py>>,
    jobs: &'a mut Vec<(&'error dyn StructuredError, Bound<'py, PyDict>)>,
    error: Option<PyErr>,
}

impl<'error, 'py> Inspector<'_, 'error, 'py> {
    fn append(&mut self, name: Option<&'static str>, value: Bound<'py, PyDict>) -> PyResult<()> {
        match self.stack.last_mut() {
            Some(Frame::Record { fields, items, .. }) => match name {
                Some(name) => fields.set_item(name, value),
                None => {
                    items.push(value.into_any().unbind());
                    Ok(())
                }
            },
            Some(Frame::List { items, .. }) if name.is_none() => {
                items.push(value.into_any().unbind());
                Ok(())
            }
            _ => Err(PyRuntimeError::new_err(
                "native error protocol has a misplaced field",
            )),
        }
    }

    fn convert(&mut self, value: ErrorValue<'error>) -> PyResult<Bound<'py, PyDict>> {
        let (kind, value) = match value {
            ErrorValue::Absent => ("absent", self.py.None()),
            ErrorValue::Unsigned(value) => (
                "unsigned",
                value.into_pyobject(self.py)?.into_any().unbind(),
            ),
            ErrorValue::Signed(value) => {
                ("signed", value.into_pyobject(self.py)?.into_any().unbind())
            }
            ErrorValue::ByteCount(value) => (
                "byte-count",
                value.into_pyobject(self.py)?.into_any().unbind(),
            ),
            ErrorValue::Boolean(value) => (
                "boolean",
                value.into_pyobject(self.py)?.to_owned().into_any().unbind(),
            ),
            ErrorValue::Float32Bits(value) => (
                "float32-bits",
                value.into_pyobject(self.py)?.into_any().unbind(),
            ),
            ErrorValue::Float64Bits(value) => (
                "float64-bits",
                value.into_pyobject(self.py)?.into_any().unbind(),
            ),
            ErrorValue::Character(value) => (
                "character",
                value
                    .to_string()
                    .into_pyobject(self.py)?
                    .into_any()
                    .unbind(),
            ),
            ErrorValue::Code(value) => ("code", value.into_pyobject(self.py)?.into_any().unbind()),
            ErrorValue::Text(value) => ("text", value.into_pyobject(self.py)?.into_any().unbind()),
            ErrorValue::Message(value) => {
                ("message", value.into_pyobject(self.py)?.into_any().unbind())
            }
            ErrorValue::Iri(value) => ("iri", value.into_pyobject(self.py)?.into_any().unbind()),
            ErrorValue::Bytes(value) => ("bytes", PyBytes::new(self.py, value).into_any().unbind()),
            ErrorValue::Cause(value) => return self.compound("cause", value),
            ErrorValue::Record(value) => return self.compound("record", value),
            ErrorValue::Term(value) => return self.compound("term", value),
        };
        value_node(self.py, kind, value)
    }

    fn compound(
        &mut self,
        kind: &str,
        source: &'error dyn StructuredError,
    ) -> PyResult<Bound<'py, PyDict>> {
        let node = record(self.py)?;
        self.jobs.push((source, node.clone()));
        value_node(self.py, kind, node.into_any().unbind())
    }

    fn close(&mut self, is_list: bool) -> PyResult<()> {
        let (name, value) = match self.stack.pop() {
            Some(Frame::List {
                name,
                length,
                items,
            }) if is_list => {
                if items.len() != length {
                    return Err(PyRuntimeError::new_err(
                        "native error list count disagrees with its items",
                    ));
                }
                (
                    name,
                    value_node(
                        self.py,
                        "list",
                        PyTuple::new(self.py, items)?.into_any().unbind(),
                    )?,
                )
            }
            Some(Frame::Record {
                name, node, items, ..
            }) if !is_list => {
                node.set_item("items", PyTuple::new(self.py, items)?)?;
                (
                    name,
                    value_node(self.py, "record", node.into_any().unbind())?,
                )
            }
            _ => {
                return Err(PyRuntimeError::new_err(
                    "native error protocol has an unmatched container",
                ));
            }
        };
        self.append(name, value)
    }
}

impl<'error> ErrorVisitor<'error> for Inspector<'_, 'error, '_> {
    fn variant(&mut self, code: &'static str) {
        if self.error.is_some() {
            return;
        }
        match self.stack.last() {
            Some(Frame::Record { node, .. }) => {
                if let Err(error) = node.set_item("code", code) {
                    self.error = Some(error);
                }
            }
            _ => {
                self.error = Some(PyRuntimeError::new_err(
                    "native error variant is outside a record",
                ));
            }
        }
    }
    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self
            .convert(value)
            .and_then(|value| self.append(name, value))
        {
            self.error = Some(error);
        }
    }
    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        if self.error.is_none() {
            self.stack.push(Frame::List {
                name,
                length,
                items: Vec::new(),
            });
        }
    }
    fn end_list(&mut self) {
        if self.error.is_none()
            && let Err(error) = self.close(true)
        {
            self.error = Some(error);
        }
    }
    fn begin_record(&mut self, name: Option<&'static str>, code: &'static str) {
        if self.error.is_some() {
            return;
        }
        let make = || -> PyResult<_> {
            let node = record(self.py)?;
            node.set_item("code", code)?;
            let fields = node
                .get_item("fields")?
                .ok_or_else(|| PyRuntimeError::new_err("native error record has no field map"))?
                .cast_into::<PyDict>()?;
            Ok(Frame::Record {
                name,
                node,
                fields,
                items: Vec::new(),
            })
        };
        match make() {
            Ok(frame) => self.stack.push(frame),
            Err(error) => self.error = Some(error),
        }
    }
    fn end_record(&mut self) {
        if self.error.is_none()
            && let Err(error) = self.close(false)
        {
            self.error = Some(error);
        }
    }
}

/// Inspect nested causes iteratively; no native cause is rendered or mirrored.
pub(crate) fn inspect<'py>(
    py: Python<'py>,
    source: &dyn StructuredError,
) -> PyResult<Bound<'py, PyDict>> {
    let root = record(py)?;
    let mut jobs = vec![(source, root.clone())];
    while let Some((source, node)) = jobs.pop() {
        let fields = node
            .get_item("fields")?
            .ok_or_else(|| PyRuntimeError::new_err("native error record has no field map"))?
            .cast_into::<PyDict>()?;
        let mut visitor = Inspector {
            py,
            stack: vec![Frame::Record {
                name: None,
                node,
                fields,
                items: Vec::new(),
            }],
            jobs: &mut jobs,
            error: None,
        };
        source.visit(&mut visitor);
        if let Some(error) = visitor.error {
            return Err(error);
        }
        match visitor.stack.pop() {
            Some(Frame::Record { node, items, .. }) if visitor.stack.is_empty() => {
                node.set_item("items", PyTuple::new(py, items)?)?;
            }
            _ => {
                return Err(PyRuntimeError::new_err(
                    "native error protocol did not close its containers",
                ));
            }
        }
    }
    Ok(root)
}

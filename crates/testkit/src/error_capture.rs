// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A generic test consumer of borrowed error events, without native enum mirrors.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// One original event, with list boundaries and unnamed entries preserved.
#[derive(Clone, Copy, Debug)]
pub enum Event<'error> {
    /// A named field or unnamed list entry.
    Value(Option<&'static str>, ErrorValue<'error>),
    /// A list's original name and exact element count.
    BeginList(Option<&'static str>, usize),
    /// End of the most recent list.
    EndList,
}

/// One producer's complete event stream. Nested payloads remain borrowed.
#[derive(Debug)]
#[must_use]
pub struct Capture<'error> {
    /// The producer's stable variant identity.
    pub code: &'static str,
    /// Every field and list event in original order.
    pub events: Vec<Event<'error>>,
}

impl<'error> Capture<'error> {
    /// Inspect one value; no native Debug/Display or cloning is needed.
    ///
    /// # Panics
    /// Fails a producer that omits or repeats its variant identity.
    pub fn read(error: &'error dyn StructuredError) -> Self {
        let mut sink = Sink::default();
        error.visit(&mut sink);
        Self {
            code: sink.code.expect("the producer must report its variant"),
            events: sink.events,
        }
    }

    /// Read an expected named scalar or compound field.
    ///
    /// # Panics
    /// Fails if the producer did not report the expected value field.
    pub fn field(&self, name: &str) -> ErrorValue<'error> {
        self.events
            .iter()
            .find_map(|event| match event {
                Event::Value(Some(field), value) if *field == name => Some(*value),
                _ => None,
            })
            .expect("the expected structured field is missing")
    }

    /// Inspect one borrowed compound field, without recursively visiting others.
    ///
    /// # Panics
    /// Fails for a missing or noncompound field.
    pub fn child(&self, name: &str) -> Self {
        match self.field(name) {
            ErrorValue::Cause(value) | ErrorValue::Record(value) | ErrorValue::Term(value) => {
                Self::read(value)
            }
            _ => panic!("the expected structured field is not compound"),
        }
    }
}

#[derive(Default)]
struct Sink<'error> {
    code: Option<&'static str>,
    events: Vec<Event<'error>>,
}

impl<'error> ErrorVisitor<'error> for Sink<'error> {
    fn variant(&mut self, code: &'static str) {
        assert!(
            self.code.replace(code).is_none(),
            "a producer reported its variant twice"
        );
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        self.events.push(Event::Value(name, value));
    }

    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        self.events.push(Event::BeginList(name, length));
    }

    fn end_list(&mut self) {
        self.events.push(Event::EndList);
    }
}

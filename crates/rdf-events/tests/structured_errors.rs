// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact ingestion capacity payloads through the shared inspection protocol.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

#[derive(Debug, Default)]
struct CapacityFields<'error> {
    code: &'static str,
    space: Option<&'error str>,
    attempted: u128,
    limit: u128,
    fields: usize,
}

impl<'error> ErrorVisitor<'error> for CapacityFields<'error> {
    fn variant(&mut self, code: &'static str) {
        self.code = code;
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        match (name, value) {
            (Some("space"), ErrorValue::Code(space)) => self.space = Some(space),
            (Some("attempted"), ErrorValue::Unsigned(value)) => self.attempted = value,
            (Some("limit"), ErrorValue::Unsigned(value)) => self.limit = value,
            _ => panic!("unexpected ingestion capacity field"),
        }
        self.fields += 1;
    }

    fn begin_list(&mut self, _: Option<&'static str>, _: usize) {
        panic!("ingestion capacity is not a list");
    }

    fn end_list(&mut self) {
        panic!("ingestion capacity has no list terminator");
    }
}

#[test]
fn capacity_inspection_keeps_original_wide_counts_and_space() {
    let error = purrdf_events::EventError::Capacity {
        space: "caller-space",
        attempted: u128::MAX,
        limit: u128::MAX - 1,
    };
    let mut captured = CapacityFields::default();
    error.visit(&mut captured);
    assert_eq!(captured.code, "event-capacity");
    assert_eq!(captured.space, Some("caller-space"));
    assert_eq!(captured.attempted, u128::MAX);
    assert_eq!(captured.limit, u128::MAX - 1);
    assert_eq!(captured.fields, 3);
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Inspect real logical refusals and parser failures without matching their enums.

use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};
use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

#[derive(Default)]
struct Fields<'error> {
    entries: Vec<(&'static str, ErrorValue<'error>)>,
    lists: Vec<(Option<&'static str>, usize)>,
    list_ends: usize,
}

impl<'error> ErrorVisitor<'error> for Fields<'error> {
    fn variant(&mut self, code: &'static str) {
        self.entries.push(("variant", ErrorValue::Code(code)));
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        self.entries.push((name.unwrap_or("entry"), value));
    }

    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        self.lists.push((name, length));
    }

    fn end_list(&mut self) {
        self.list_ends += 1;
    }
}

#[test]
fn original_capacity_counts_survive_storage_and_json_transport() {
    let widest = CapacityError::check(CapacitySpace::Quads, u128::MAX, u128::MAX - 1)
        .expect_err("the original producer retains all 128 bits");
    let mut wide_fields = Fields::default();
    widest.visit(&mut wide_fields);
    assert!(matches!(
        wide_fields.entries[2],
        ("attempted", ErrorValue::Unsigned(u128::MAX))
    ));
    assert!(
        matches!(wide_fields.entries[3], ("limit", ErrorValue::Unsigned(n)) if n == u128::MAX - 1)
    );
    let capacity = CapacityError::slot(CapacitySpace::GlobalTerms, u64::MAX, u64::MAX)
        .expect_err("the exclusive count exceeds the full-width address space");
    let storage = StorageError::from(capacity);
    let json = purrdf_lex::json::Error::new(purrdf_lex::json::ErrorKind::Storage(storage), 17);
    let mut pending: Vec<&dyn StructuredError> = vec![&json];
    let mut observed = Vec::new();
    while let Some(error) = pending.pop() {
        let mut fields = Fields::default();
        error.visit(&mut fields);
        for (name, value) in fields.entries {
            if let ErrorValue::Cause(cause) = value {
                pending.push(cause);
            } else {
                observed.push((name, value));
            }
        }
    }
    assert_eq!(observed.len(), 8);
    assert!(matches!(
        observed[0],
        ("variant", ErrorValue::Code("json-document"))
    ));
    assert!(matches!(observed[1], ("offset", ErrorValue::ByteCount(17))));
    assert!(matches!(
        observed[2],
        ("variant", ErrorValue::Code("json-storage"))
    ));
    assert!(matches!(
        observed[3],
        ("variant", ErrorValue::Code("storage-capacity"))
    ));
    assert!(matches!(
        observed[4],
        ("variant", ErrorValue::Code("capacity-exceeded"))
    ));
    assert!(matches!(
        observed[5],
        ("space", ErrorValue::Code("rdf-global-terms"))
    ));
    assert!(
        matches!(observed[6], ("attempted", ErrorValue::Unsigned(n)) if n == u128::from(u64::MAX) + 1)
    );
    assert!(matches!(observed[7], ("limit", ErrorValue::Unsigned(n)) if n == u128::from(u64::MAX)));
}

#[test]
fn parser_reports_original_escape_reason_and_byte_offset() {
    let error = purrdf_lex::json::read(r#""\uDC00""#).expect_err("a lone low surrogate is invalid");
    let mut fields = Fields::default();
    error.visit(&mut fields);
    assert_eq!(fields.entries.len(), 3);
    assert!(
        matches!(fields.entries[2], ("offset", ErrorValue::ByteCount(n)) if n == error.offset() as u128)
    );
    let ErrorValue::Cause(kind) = fields.entries[1].1 else {
        panic!("JSON must preserve its typed cause");
    };
    let mut reason = Fields::default();
    kind.visit(&mut reason);
    assert!(matches!(
        reason.entries[0],
        ("variant", ErrorValue::Code("json-invalid-escape"))
    ));
    let ErrorValue::Cause(escape) = reason.entries[1].1 else {
        panic!("the escape reason must remain typed");
    };
    let mut detail = Fields::default();
    escape.visit(&mut detail);
    assert_eq!(detail.entries.len(), 1);
    assert!(matches!(
        detail.entries[0],
        ("variant", ErrorValue::Code("json-escape-unpaired-low"))
    ));
}

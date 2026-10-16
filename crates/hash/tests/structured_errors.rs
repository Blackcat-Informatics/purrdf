// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed native errors, complete field capture and allocation-free inspection.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Debug, Default)]
struct Capture<'error> {
    code: &'static str,
    fields: [Option<(Option<&'static str>, ErrorValue<'error>)>; 16],
    count: usize,
    lists: [Option<(Option<&'static str>, usize)>; 4],
    list_count: usize,
    list_ends: usize,
}

impl<'error> ErrorVisitor<'error> for Capture<'error> {
    fn variant(&mut self, code: &'static str) {
        self.code = code;
    }

    fn value(&mut self, name: Option<&'static str>, value: ErrorValue<'error>) {
        self.fields[self.count] = Some((name, value));
        self.count += 1;
    }

    fn begin_list(&mut self, name: Option<&'static str>, length: usize) {
        self.lists[self.list_count] = Some((name, length));
        self.list_count += 1;
    }

    fn end_list(&mut self) {
        self.list_ends += 1;
    }
}

enum NoFormat {
    Cause,
}

impl core::fmt::Debug for NoFormat {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("inspection called Debug");
    }
}

impl core::fmt::Display for NoFormat {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        panic!("inspection called Display");
    }
}

impl std::error::Error for NoFormat {}

impl StructuredError for NoFormat {
    purrdf_hash::error_visit! {
        Cause => "test-cause" {},
    }
}

enum Producer<'error> {
    Parent {
        cause: &'error dyn StructuredError,
        attempted: u128,
        message: &'error str,
    },
    List(&'error [&'error dyn StructuredError]),
    Optional {
        cause: Option<&'error dyn StructuredError>,
    },
    Later {
        signed: i128,
    },
}

impl StructuredError for Producer<'_> {
    purrdf_hash::error_visit! {
        Parent { cause, attempted, message } => "test-parent" {
            "cause" => Cause(*cause),
            "attempted" => Unsigned(*attempted),
            "message" => Message(message),
        },
        List(items) => "test-list" {
            "items" => List(items.iter().map(|item| ErrorValue::Cause(*item))),
        },
        Optional { cause } => "test-optional" {
            "cause" => Value(cause.map_or(ErrorValue::Absent, ErrorValue::Cause)),
        },
        Later { signed } => "test-later" {
            "signed" => Signed(*signed),
            "optional" => Absent,
        },
    }
}

#[test]
fn native_hex_failures_expose_original_fields_without_allocating() {
    let errors = [
        purrdf_hash::hex::decode("x").unwrap_err(),
        purrdf_hash::hex::decode("g0").unwrap_err(),
        purrdf_hash::hex::encode_to_slice(&[1], &mut [0; 1]).unwrap_err(),
    ];
    let mut captures: [Capture<'_>; 3] = core::array::from_fn(|_| Capture::default());
    let window = CurrentThreadWindow::open();
    for (error, capture) in errors.iter().zip(&mut captures) {
        error.visit(capture);
    }
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(captures[0].code, "hex-odd-length");
    assert!(matches!(
        captures[0].fields[0],
        Some((Some("len"), ErrorValue::ByteCount(1)))
    ));
    assert_eq!(captures[1].code, "hex-invalid-digit");
    assert!(matches!(
        captures[1].fields[0],
        Some((Some("offset"), ErrorValue::ByteCount(0)))
    ));
    assert!(matches!(
        captures[1].fields[1],
        Some((Some("byte"), ErrorValue::Unsigned(103)))
    ));
    assert_eq!(captures[2].code, "hex-output-too-short");
    assert!(matches!(
        captures[2].fields[0],
        Some((Some("needed"), ErrorValue::ByteCount(2)))
    ));
    assert!(matches!(
        captures[2].fields[1],
        Some((Some("available"), ErrorValue::ByteCount(1)))
    ));
}

#[test]
fn generic_capture_preserves_wide_values_lists_and_unformatted_causes() {
    let cause = NoFormat::Cause;
    let parent = Producer::Parent {
        cause: &cause,
        attempted: u128::MAX,
        message: "remote message, not a formatted native cause",
    };
    let mut capture = Capture::default();
    parent.visit(&mut capture);
    assert!(matches!(
        capture.fields[1],
        Some((Some("attempted"), ErrorValue::Unsigned(u128::MAX)))
    ));
    assert!(matches!(
        capture.fields[2],
        Some((
            Some("message"),
            ErrorValue::Message("remote message, not a formatted native cause")
        ))
    ));
    let Some((Some("cause"), ErrorValue::Cause(nested))) = capture.fields[0] else {
        panic!("the original structured cause is missing");
    };
    let mut child = Capture::default();
    nested.visit(&mut child);
    assert_eq!(child.code, "test-cause");

    let items: [&dyn StructuredError; 2] = [&cause, &parent];
    let list = Producer::List(&items);
    let mut capture = Capture::default();
    list.visit(&mut capture);
    assert_eq!(capture.lists[0], Some((Some("items"), 2)));
    assert_eq!(capture.list_ends, 1);
    assert_eq!(capture.count, 2);
    assert!(matches!(
        capture.fields[0],
        Some((None, ErrorValue::Cause(_)))
    ));

    let empty = Producer::List(&[]);
    let mut capture = Capture::default();
    empty.visit(&mut capture);
    assert_eq!(capture.lists[0], Some((Some("items"), 0)));
    assert_eq!(capture.list_ends, 1);
    assert_eq!(capture.count, 0);

    let later = Producer::Later { signed: i128::MIN };
    let mut capture = Capture::default();
    later.visit(&mut capture);
    assert_eq!(capture.code, "test-later");
    assert!(matches!(
        capture.fields[0],
        Some((Some("signed"), ErrorValue::Signed(i128::MIN)))
    ));
    assert!(matches!(
        capture.fields[1],
        Some((Some("optional"), ErrorValue::Absent))
    ));

    for cause in [None, Some(&cause as &dyn StructuredError)] {
        let optional = Producer::Optional { cause };
        let mut capture = Capture::default();
        optional.visit(&mut capture);
        assert_eq!(capture.code, "test-optional");
        match capture.fields[0] {
            Some((Some("cause"), ErrorValue::Absent)) => assert!(cause.is_none()),
            Some((Some("cause"), ErrorValue::Cause(_))) => assert!(cause.is_some()),
            _ => panic!("optional cause lost its field or value kind"),
        }
    }
}

#[test]
fn standard_io_categories_retain_native_identity_without_formatting_or_allocation() {
    let kinds = [
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::Other,
        std::io::Error::from_raw_os_error(i32::MAX).kind(),
    ];
    let mut captures: [Capture<'_>; 3] = core::array::from_fn(|_| Capture::default());
    let window = CurrentThreadWindow::open();
    for (kind, capture) in kinds.iter().zip(&mut captures) {
        kind.visit(capture);
    }
    assert_eq!(window.close().allocations, 0);
    for (kind, capture) in kinds.iter().zip(&captures) {
        assert_eq!(capture.code, "std-io-kind");
        assert_eq!(capture.count, 2);
        assert!(
            matches!(capture.fields[1], Some((Some("native_discriminant"), ErrorValue::Unsigned(n))) if n == *kind as u128)
        );
    }
    assert!(matches!(
        captures[0].fields[0],
        Some((Some("name"), ErrorValue::Code("permission-denied")))
    ));
    assert!(matches!(
        captures[1].fields[0],
        Some((Some("name"), ErrorValue::Code("other")))
    ));
    assert_eq!(
        matches!(
            captures[2].fields[0],
            Some((Some("name"), ErrorValue::Code("other")))
        ),
        kinds[2] == std::io::ErrorKind::Other,
        "an unnamed standard category must not be relabeled Other",
    );
}

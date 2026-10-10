// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native range allocation/refusal controls over independently known value sets.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_xsd::range::{
    self, Cardinality, DataRange, Facet, Satisfiability, Storage, StorageError,
};
use purrdf_xsd::{XsdDatatype, XsdValue};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Default)]
struct Owner {
    ceiling: Option<usize>,
    work_ceiling: Option<u64>,
    work: u64,
    live: usize,
    peak: usize,
    allocations: u64,
    refused: bool,
}
impl Storage for Owner {
    fn work(&mut self, operations: u64) -> Result<(), StorageError> {
        let next = self
            .work
            .checked_add(operations)
            .ok_or(StorageError::Allocation)?;
        if self.refused || self.work_ceiling.is_some_and(|ceiling| next > ceiling) {
            self.refused = true;
            return Err(StorageError::Refused);
        }
        self.work = next;
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.live
            && (self.refused || self.ceiling.is_some_and(|ceiling| bytes > ceiling))
        {
            self.refused = true;
            return Err(StorageError::Refused);
        }
        self.live = bytes;
        self.peak = self.peak.max(bytes);
        Ok(())
    }
    fn allocated(&mut self) {
        self.allocations += 1;
    }
}

fn integer_interval(lower: &str, upper: &str) -> DataRange {
    DataRange::Restriction {
        base: XsdDatatype::Integer,
        facets: vec![
            Facet::MinInclusive(purrdf_xsd::parse(lower, XsdDatatype::Decimal).unwrap()),
            Facet::MaxInclusive(purrdf_xsd::parse(upper, XsdDatatype::Decimal).unwrap()),
        ],
    }
}

#[test]
fn long_exact_endpoint_products_are_admitted_and_all_scratch_is_released() {
    // A huge coefficient and a fractional endpoint force the actual limb,
    // comparison, ceil/floor and finite-extent operations, rather than just Vecs.
    let whole = "9".repeat(1024);
    let lower = format!("{whole}.1");
    let upper = format!("1{}0.9", "0".repeat(1023));
    let range = integer_interval(&lower, &upper);
    // Both endpoints lie between the integers 10^1024-1 and 10^1024+1.
    // Exactly 10^1024 belongs to this integer range.
    let mut owner = Owner::default();
    let window = CurrentThreadWindow::open();
    assert_eq!(
        range::try_cardinality(&range, &mut owner).unwrap(),
        Cardinality::Exactly(1)
    );
    let measured = window.close();
    assert_eq!(owner.live, 0);
    assert_eq!(measured.retained_bytes, 0);
    assert!(owner.allocations > 0 && owner.peak > 0 && owner.work > 0);
    assert!(
        measured.peak_working_bytes <= i64::try_from(owner.peak).unwrap(),
        "actual native allocator peak {measured:?} exceeds original admission {}",
        owner.peak
    );
    let mut refused = Owner {
        ceiling: Some(0),
        ..Owner::default()
    };
    let window = CurrentThreadWindow::open();
    assert_eq!(
        range::try_cardinality(&range, &mut refused),
        Err(StorageError::Refused)
    );
    let measured = window.close();
    assert_eq!(
        measured.allocations, 0,
        "refusal occurs before the native destination exists"
    );
    assert_eq!(refused.live, 0);
}

#[test]
fn exact_boolean_decimal_binary_and_string_set_laws_survive_the_native_owner() {
    let lower = integer_interval(
        "-10000000000000000000000000000000000000000000000000000000000000000.9",
        "-10000000000000000000000000000000000000000000000000000000000000000.1",
    );
    // There is no integer strictly between k-0.9 and k-0.1.
    let mut owner = Owner::default();
    assert_eq!(
        range::try_cardinality(&lower, &mut owner).unwrap(),
        Cardinality::Exactly(0)
    );
    let single = DataRange::OneOf(vec![XsdValue::String("é".repeat(128))]);
    let whole = DataRange::Datatype(XsdDatatype::String);
    assert_eq!(
        range::try_containment(&single, &whole, &mut owner).unwrap(),
        Satisfiability::Empty
    );
    assert_eq!(
        range::try_cardinality(&DataRange::Datatype(XsdDatatype::Boolean), &mut owner).unwrap(),
        Cardinality::Exactly(2)
    );
    let octets = DataRange::Restriction {
        base: XsdDatatype::HexBinary,
        facets: vec![Facet::Length(2)],
    };
    assert_eq!(
        range::try_cardinality(&octets, &mut owner).unwrap(),
        Cardinality::Exactly(65_536)
    );
    let window = integer_interval("-1.5", "2.5");
    assert_eq!(
        range::try_cardinality(&window, &mut owner).unwrap(),
        Cardinality::Exactly(4)
    );
    assert_eq!(owner.live, 0);
}

#[test]
fn refusing_work_never_becomes_an_empty_or_complete_containment_answer() {
    let range = DataRange::And(vec![
        DataRange::Datatype(XsdDatatype::Integer),
        integer_interval("0", "2"),
    ]);
    let mut owner = Owner {
        work_ceiling: Some(3),
        ..Owner::default()
    };
    assert_eq!(
        range::try_containment(
            &range,
            &DataRange::Datatype(XsdDatatype::Decimal),
            &mut owner
        ),
        Err(StorageError::Refused)
    );
    assert!(owner.refused);
    assert_eq!(owner.live, 0);
    owner.refused = false;
    owner.work_ceiling = None;
    assert_eq!(
        range::try_containment(
            &range,
            &DataRange::Datatype(XsdDatatype::Decimal),
            &mut owner
        )
        .unwrap(),
        Satisfiability::Empty
    );
    assert_eq!(owner.live, 0);
}

#[test]
fn original_allocation_failure_outranks_a_failed_cleanup_callback() {
    struct Refuse {
        birth: bool,
        cleanup: bool,
    }
    impl Storage for Refuse {
        fn work(&mut self, _: u64) -> Result<(), StorageError> {
            Ok(())
        }
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes != 0 {
                self.birth = true;
                Err(StorageError::Allocation)
            } else {
                self.cleanup = true;
                Err(StorageError::Refused)
            }
        }
    }
    let mut owner = Refuse {
        birth: false,
        cleanup: false,
    };
    assert_eq!(
        range::try_cardinality(&DataRange::Datatype(XsdDatatype::Integer), &mut owner),
        Err(StorageError::Allocation)
    );
    assert!(owner.birth && owner.cleanup);
}

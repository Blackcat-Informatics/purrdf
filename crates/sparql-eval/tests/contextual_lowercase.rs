// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual allocation/refusal boundaries of the contextual lowercase producer.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

#[derive(Default)]
struct Budget {
    limit: usize,
    live: usize,
    peak: usize,
}
impl Admission for Budget {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.limit {
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        self.peak = self.peak.max(bytes);
        Ok(())
    }
}

#[test]
fn refusal_precedes_the_actual_output_allocation() {
    let mut budget = Budget::default();
    let mut memory = Memory::new(&mut budget);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let output = purrdf_lex::unicode::lowercase_with_memory("İAΣ", &mut memory);
    let measured = window.close();
    assert!(matches!(output, Err(StorageError::AdmissionFailed)));
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
    assert_eq!(memory.admitted_bytes(), 0);
    assert_eq!(budget.live, 0);
}

#[test]
fn healthy_expansion_and_final_sigma_have_one_original_output_owner() {
    let mut budget = Budget {
        limit: 4096,
        ..Budget::default()
    };
    let mut memory = Memory::new(&mut budget);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let output = purrdf_lex::unicode::lowercase_with_memory("İA.Σ\u{345}", &mut memory).unwrap();
    let measured = window.close();
    assert_eq!(output, "i\u{307}a.ς\u{345}");
    let retained = memory.admitted_bytes();
    assert_eq!(output.capacity(), retained);
    assert_eq!(measured.allocations, 1);
    assert_eq!(
        usize::try_from(measured.peak_working_bytes).unwrap(),
        retained,
        "the native contextual iterator allocates no private transformation buffer"
    );
    assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), retained);
    memory.release_string(output).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
    assert_eq!(budget.live, 0);
    assert_eq!(budget.peak, retained);
}

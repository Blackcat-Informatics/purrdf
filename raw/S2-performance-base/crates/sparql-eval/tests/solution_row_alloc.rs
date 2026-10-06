// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Allocation shape of the native solution-row clone at both storage widths.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::TermId;
use purrdf_sparql_eval::{Solution, SolutionTerm};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn row(width: usize) -> Solution {
    (0..width)
        .map(|index| {
            Some(SolutionTerm::Existing(TermId::from_index(
                u32::try_from(index).expect("fixture index"),
            )))
        })
        .collect()
}

#[test]
fn clone_has_inline_zero_and_spilled_one_allocation() {
    for (width, expected_allocations) in [(4, 0), (9, 1)] {
        let source = row(width);
        let warm = source.clone();
        assert_eq!(&warm[..], &source[..]);
        drop(warm);

        let window = CurrentThreadWindow::open();
        let cloned = std::hint::black_box(&source).clone();
        let measured = window.close();
        assert_eq!(&cloned[..], &source[..]);
        assert_eq!(measured.allocations, expected_allocations, "width {width}");
        assert_eq!(
            measured.requested_bytes,
            u64::try_from(width * size_of::<Option<SolutionTerm>>())
                .expect("fixture size fits u64")
                * expected_allocations,
            "width {width}"
        );
        assert_eq!(cloned.capacity(), width.max(4), "width {width}");
        assert_eq!(cloned.spilled(), width > 4);
    }
}

#[test]
fn copy_from_slice_has_exact_allocation_budget() {
    for width in [4, 9] {
        let source = row(width);
        let window = CurrentThreadWindow::open();
        let copied = Solution::from_slice(std::hint::black_box(&source));
        let measured = window.close();
        assert_eq!(&copied[..], &source[..]);
        assert_eq!(measured.allocations, u64::from(width > 4), "width {width}");
        assert_eq!(copied.capacity(), width.max(4), "width {width}");
        assert_eq!(
            measured.requested_bytes,
            u64::try_from(if width > 4 {
                width * size_of::<Option<SolutionTerm>>()
            } else {
                0
            })
            .expect("fixture size fits u64"),
            "width {width}"
        );
    }
}

#[test]
fn clone_then_push_allocation_count_is_explicit() {
    for (width, expected_allocations) in [(4, 1), (9, 2)] {
        let source = row(width);
        let cell = Some(SolutionTerm::Existing(TermId::from_index(100)));
        let window = CurrentThreadWindow::open();
        let mut cloned = std::hint::black_box(&source).clone();
        cloned.push(cell);
        let measured = window.close();
        assert_eq!(cloned.len(), width + 1);
        assert_eq!(cloned[width], cell);
        assert_eq!(measured.allocations, expected_allocations, "width {width}");
        assert_eq!(
            measured.requested_bytes,
            u64::try_from(
                (cloned.capacity() + usize::from(width > 4) * width)
                    * size_of::<Option<SolutionTerm>>()
            )
            .expect("fixture size fits u64"),
            "width {width}"
        );
    }
}

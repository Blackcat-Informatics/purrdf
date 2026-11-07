// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Work and allocation observations through the resident production entry.

use super::*;
use purrdf_alloc_probe::{CurrentThreadWindow, Measurement};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default)]
struct Work {
    measure_phases: bool,
    projections: usize,
    comparisons: usize,
    preparation: Measurement,
    sorting: Measurement,
}

thread_local! {
    static VALUE_ORDER_WORK: Cell<Work> = const { Cell::new(Work {
        measure_phases: false,
        projections: 0, comparisons: 0,
        preparation: Measurement { allocations: 0, requested_bytes: 0, retained_bytes: 0, peak_working_bytes: 0 },
        sorting: Measurement { allocations: 0, requested_bytes: 0, retained_bytes: 0, peak_working_bytes: 0 },
    }) };
}

fn record(update: impl FnOnce(&mut Work)) {
    VALUE_ORDER_WORK.with(|cell| {
        let mut work = cell.get();
        update(&mut work);
        cell.set(work);
    });
}
pub(crate) fn phase_window() -> Option<CurrentThreadWindow> {
    VALUE_ORDER_WORK
        .with(|cell| cell.get().measure_phases)
        .then(CurrentThreadWindow::open)
}
pub(crate) fn note_projection() {
    record(|work| work.projections += 1);
}
pub(crate) fn note_comparison() {
    record(|work| work.comparisons += 1);
}
pub(crate) fn record_preparation(measurement: Measurement) {
    record(|work| work.preparation = measurement);
}
pub(crate) fn record_sorting(measurement: Measurement) {
    record(|work| work.sorting = measurement);
}

fn triple(object: TermValue) -> TermValue {
    TermValue::Triple {
        s: purrdf_core::TermBox::new(TermValue::iri("https://example.org/s")),
        p: purrdf_core::TermBox::new(TermValue::iri("https://example.org/p")),
        o: purrdf_core::TermBox::new(object),
    }
}

#[test]
fn prepared_value_order_preserves_pairwise_permutation_and_reports_phase_costs() {
    let samples = [
        None,
        Some(TermValue::Blank {
            label: "b".into(),
            scope: purrdf_core::BlankScope(3),
        }),
        Some(TermValue::Blank {
            label: "b".into(),
            scope: purrdf_core::BlankScope(1),
        }),
        Some(TermValue::iri("https://example.org/i")),
        Some(TermValue::typed_literal(
            "02",
            purrdf_xsd::datatype::XSD_INTEGER,
        )),
        Some(TermValue::typed_literal(
            "2",
            purrdf_xsd::datatype::XSD_INTEGER,
        )),
        Some(TermValue::typed_literal(
            "invalid",
            purrdf_xsd::datatype::XSD_INTEGER,
        )),
        Some(TermValue::typed_literal("[1, 2]", purrdf_cdt::CDT_LIST)),
        Some(TermValue::typed_literal("[2]", purrdf_cdt::CDT_LIST)),
        Some(triple(triple(TermValue::typed_literal(
            "10",
            purrdf_xsd::datatype::XSD_INTEGER,
        )))),
        Some(triple(triple(TermValue::typed_literal(
            "2",
            purrdf_xsd::datatype::XSD_INTEGER,
        )))),
    ];
    for length in [0, 1, 32, 256] {
        let values: Vec<_> = samples.iter().cycle().take(length).cloned().collect();
        for descending in [false, true] {
            VALUE_ORDER_WORK.with(|cell| cell.set(Work::default()));
            let baseline_window = CurrentThreadWindow::open();
            let mut expected: Vec<_> = (0..length).collect();
            expected.sort_by(|&left, &right| {
                note_comparison();
                let order = crate::modifier::total_order(
                    &crate::modifier::project(values[left].as_ref()),
                    &crate::modifier::project(values[right].as_ref()),
                );
                if descending { order.reverse() } else { order }
            });
            let baseline_allocation = baseline_window.close();
            let baseline_work = VALUE_ORDER_WORK.with(Cell::get);
            VALUE_ORDER_WORK.with(|cell| {
                cell.set(Work {
                    measure_phases: true,
                    ..Work::default()
                })
            });
            let actual = order_value_permutation(values.iter().map(Option::as_ref), descending);
            let optimized_work = VALUE_ORDER_WORK.with(Cell::get);
            assert_eq!(actual, expected);
            assert_eq!(optimized_work.projections, length);
            // Whole-call and phase windows are separate invocations: the
            // allocator deliberately refuses nested measurement windows.
            VALUE_ORDER_WORK.with(|cell| cell.set(Work::default()));
            let optimized_window = CurrentThreadWindow::open();
            let repeated = order_value_permutation(values.iter().map(Option::as_ref), descending);
            let optimized_allocation = optimized_window.close();
            assert_eq!(repeated, expected);
            eprintln!(
                "length={length} descending={descending} baseline={baseline_work:?} allocation={baseline_allocation:?}"
            );
            eprintln!(
                "length={length} descending={descending} prepared={optimized_work:?} allocation={optimized_allocation:?}"
            );
        }
    }
}

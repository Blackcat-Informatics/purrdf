// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native differential witness and resource qualification of assignment pruning.

use super::*;
use crate::ast::TripleExprGroup;
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

type Kernel = fn(
    &Compiled<'_>,
    &[ArcOptions],
    &mut [(u64, u64)],
    &mut [Option<usize>],
    &mut u64,
    u64,
) -> Result<bool, crate::ValidationError>;

fn run(kernel: Kernel, compiled: &Compiled<'_>, arcs: &[ArcOptions]) -> Option<Assignment> {
    let mut counts = vec![(0, 0); compiled.slots.len()];
    let mut assignment = vec![None; arcs.len()];
    let mut budget = 2_000_000;
    kernel(
        compiled,
        arcs,
        &mut counts,
        &mut assignment,
        &mut budget,
        2_000_000,
    )
    .unwrap()
    .then_some((counts, assignment))
}

fn tc(min: i64, max: i64, inverse: bool) -> TripleExpr {
    TripleExpr::TripleConstraint(TripleConstraint {
        predicate: "p".into(),
        min: Some(min),
        max: Some(max),
        inverse: Some(inverse),
        sem_acts: vec![SemAct {
            name: format!("slot-{min}-{max}"),
            code: None,
        }],
        ..TripleConstraint::default()
    })
}

fn group(children: Vec<TripleExpr>, one: bool, min: i64, max: i64) -> TripleExpr {
    let group = TripleExprGroup {
        expressions: children,
        min: Some(min),
        max: Some(max),
        sem_acts: vec![SemAct {
            name: format!("group-{one}-{min}-{max}"),
            code: None,
        }],
        ..TripleExprGroup::default()
    };
    if one {
        TripleExpr::OneOf(group)
    } else {
        TripleExpr::EachOf(group)
    }
}

#[test]
fn pruned_search_preserves_exhaustive_first_witness() {
    let expressions = [
        group(vec![tc(0, 0, false), tc(2, 2, false)], false, 1, 1),
        group(vec![tc(0, 2, false), tc(1, 1, true)], true, 2, 3),
        group(vec![tc(2, 2, false), tc(2, 2, true)], false, 0, -1),
        group(
            vec![
                group(vec![tc(0, 1, false), tc(1, 2, true)], true, 1, 2),
                tc(0, 2, false),
            ],
            false,
            0,
            2,
        ),
    ];
    for expr in &expressions {
        let compiled = compile(expr, &FastMap::default()).unwrap();
        let kinds = 1usize << (compiled.slots.len() + 1);
        // Enumerate every candidate/EXTRA combination for up to three arcs.
        for length in 0..=3 {
            for mut code in 0..kinds.pow(length) {
                let arcs: Vec<_> = (0..length)
                    .map(|_| {
                        let mask = code % kinds;
                        code /= kinds;
                        ArcOptions {
                            candidates: (0..compiled.slots.len())
                                .filter(|slot| mask & (1 << slot) != 0)
                                .collect(),
                            extra_allowed: mask & (1 << compiled.slots.len()) != 0,
                        }
                    })
                    .collect();
                let expected = run(exhaustive_search, &compiled, &arcs);
                for kernel in [search::<0> as Kernel, search::<128>, search::<MEMO_BYTES>] {
                    let actual = run(kernel, &compiled, &arcs);
                    assert_eq!(actual, expected, "{expr:?} {arcs:?}");
                }
            }
        }
    }
}

#[test]
fn compare_ambiguous_and_forced_search_work_and_storage() {
    let cases = [
        (
            "zero-slot",
            group(vec![tc(0, 0, false), tc(18, 18, false)], false, 1, 1),
            18,
            false,
        ),
        // An odd number of repetitions cannot split into equal even counts.
        (
            "residual-reuse",
            group(vec![tc(2, 2, false), tc(2, 2, false)], false, 0, -1),
            18,
            false,
        ),
        (
            "forced",
            group(vec![tc(9, 9, false), tc(9, 9, true)], false, 1, 1),
            18,
            true,
        ),
    ];
    for (name, expr, length, forced) in cases {
        let compiled = compile(&expr, &FastMap::default()).unwrap();
        let arcs: Vec<_> = (0..length)
            .map(|index| ArcOptions {
                candidates: if forced { vec![index % 2] } else { vec![0, 1] },
                extra_allowed: false,
            })
            .collect();
        let mut baseline = None;
        let mut no_cache = SearchWork::default();
        for (method, kernel) in [
            ("exhaustive", exhaustive_search as Kernel),
            ("interval", search::<0> as Kernel),
            ("memo", search::<MEMO_BYTES> as Kernel),
        ] {
            SEARCH_WORK.with(|work| work.set(SearchWork::default()));
            let window = CurrentThreadWindow::open();
            let result = run(kernel, &compiled, &arcs);
            let measurement = window.close();
            let work = SEARCH_WORK.with(std::cell::Cell::get);
            eprintln!("case={name} method={method} work={work:?} allocation={measurement:?}");
            if method == "exhaustive" {
                baseline = Some(result.clone());
            }
            assert_eq!(Some(&result), baseline.as_ref());
            assert!(work.memo_bytes <= MEMO_BYTES);
            if method == "interval" {
                no_cache = work;
            }
            if method == "memo" && name == "residual-reuse" {
                assert!(work.memo_entries > 0);
                assert!(work.states < no_cache.states / 2);
            }
            if method != "exhaustive" && name == "zero-slot" {
                assert!(work.states <= 2 * length as u64 + 1);
            }
        }
    }
}

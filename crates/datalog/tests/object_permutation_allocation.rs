// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The lazy object index admits one resident address per consolidated row.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::cursor::LendingIterator;
use purrdf_datalog::seminaive::{compile, evaluate};
use purrdf_datalog::store::{Bound, RelationStore};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn consolidated_object_order_serves_the_actual_cycle_join() {
    let atom = |s, p, o| ClauseAtom::positive(ClauseTerm::var(s), p, ClauseTerm::var(o));
    let program = compile(vec![DlClause::datalog(
        atom("?s", "https://example.org/result", "?t"),
        vec![
            atom("?s", "https://example.org/p", "?o"),
            atom("?t", "https://example.org/q", "?o"),
            atom("?s", "https://example.org/r", "?t"),
        ],
    )])
    .unwrap();
    let term = |prefix, index| format!("<https://example.org/{prefix}{index}>");
    let mut input = RelationStore::default();
    for index in 0..517 {
        input
            .insert(
                &term("s", index),
                "<https://example.org/p>",
                &term("o", index),
                "",
            )
            .unwrap();
        input
            .insert(
                &term("s", index),
                "<https://example.org/r>",
                &term("t", index),
                "",
            )
            .unwrap();
    }
    for index in (0..517).rev() {
        input
            .insert(
                &term("t", index),
                "<https://example.org/q>",
                &term("o", index),
                "",
            )
            .unwrap();
    }
    let output = evaluate(&program, input).unwrap();
    assert_eq!(
        output.facts().len_for("<https://example.org/result>", ""),
        517
    );
    for index in 0..517 {
        assert!(output.facts().contains(
            &term("s", index),
            "<https://example.org/result>",
            &term("t", index),
            ""
        ));
    }
}

#[test]
fn object_probe_allocates_one_address_per_row_once() {
    let mut store = RelationStore::default();
    let count = 512;
    for index in 0..count {
        store.insert(&format!("s{index}"), "p", "o", "").unwrap();
    }
    let object = store.term_id("o").unwrap();
    for first in [true, false] {
        let window = CurrentThreadWindow::open();
        let mut observed = 0;
        {
            let mut cursor = store.select("p", "", Bound::Object(object));
            while cursor.next().is_some() {
                observed += 1;
            }
        }
        let allocation = window.close();
        assert_eq!(observed, count);
        let expected = if first { count * size_of::<usize>() } else { 0 };
        assert_eq!(allocation.requested_bytes, expected as u64);
        assert_eq!(allocation.retained_bytes, expected as i64);
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual borrowed iteration and clone storage, measured at the public seam.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_text::unicode::{sentence_bounds, sentence_indices};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn sentence_iteration_and_partial_cursor_clones_allocate_nothing() {
    let input = String::from("。！？\r\nÉté. 東京。\u{2029}👩‍💻!");
    let window = CurrentThreadWindow::open();
    let mut original = sentence_bounds(&input);
    assert_eq!(original.next(), Some("。！？\r\n"));
    let mut cloned = original.clone();
    for expected in ["Été. ", "東京。\u{2029}", "👩‍💻!"] {
        let left = original.next().expect("original segment");
        let right = cloned.next().expect("cloned segment");
        assert_eq!(left, expected);
        assert_eq!(right, expected);
        assert!(std::ptr::eq(left.as_ptr(), right.as_ptr()));
    }
    assert_eq!(original.next(), None);
    assert_eq!(cloned.next(), None);
    let mut indices = sentence_indices(&input);
    assert_eq!(indices.next(), Some((11, "Été. ")));
    let mut cloned_indices = indices.clone();
    assert_eq!(indices.next(), Some((18, "東京。\u{2029}")));
    assert_eq!(cloned_indices.next(), Some((18, "東京。\u{2029}")));
    assert_eq!(indices.next(), None);
    assert_eq!(cloned_indices.next(), None);
    let measured = window.close();
    assert_eq!(measured.allocations, 0, "all cursors and clones are inline");
    assert_eq!(measured.retained_bytes, 0, "segments are original borrows");
    assert_eq!(measured.peak_working_bytes, 0);
}

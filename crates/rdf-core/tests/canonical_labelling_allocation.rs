// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The bytes the canonical labelling requests grow linearly with the size of
//! the input. Refining a path peels two members off one large cell per step, so
//! storage sized by the cell and requested per split makes the total quadratic
//! in the length of the path while the allocation count, the retained bytes and
//! the walk units stay linear.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::canonical_labelling::{Line, Term, label};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn quad(subject: &str, object: &str) -> Line {
    Line::Quad {
        subject: Term::blank(subject.as_bytes()),
        predicate: Term::iri(b"p"),
        object: Term::blank(object.as_bytes()),
        graph: None,
    }
}

fn path(n: usize) -> Vec<Line> {
    (0..n - 1)
        .map(|i| quad(&format!("n{i}"), &format!("n{}", i + 1)))
        .collect()
}

fn hub(n: usize) -> Vec<Line> {
    (0..n).map(|i| quad("h", &format!("l{i}"))).collect()
}

fn requested_bytes(lines: &[Line]) -> u64 {
    let window = CurrentThreadWindow::open();
    let labelled = label(lines).expect("the shape is labelled");
    let measured = window.close();
    drop(labelled);
    measured.requested_bytes
}

/// Quadrupling the input at most about quadruples the requested bytes: the
/// path of the refinement ripple, and a hub whose one large cell is never
/// peeled, both stay within a factor of six where quadratic growth gives
/// sixteen.
#[test]
fn requested_bytes_grow_linearly_with_the_size_of_the_input() {
    for (name, shape) in [("path", path as fn(usize) -> Vec<Line>), ("hub", hub)] {
        let small = requested_bytes(&shape(500));
        let large = requested_bytes(&shape(2_000));
        assert!(
            large < 6 * small,
            "{name}: 500 requested {small} bytes, 2000 requested {large}"
        );
    }
}

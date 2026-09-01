// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hostile lexical forms produce a **typed error**, and the test process survives.
//!
//! # What "the process exits 0" proves
//!
//! A Rust stack overflow is an `abort`: the process dies with SIGSEGV/SIGABRT and no
//! `catch_unwind`, `Result` or test harness can intercept it. A recursive-descent
//! scanner handed a million-deep `[[[[…` therefore does not *fail* the assertion
//! below — it kills the test binary before the assertion runs, and the harness
//! reports a crashed test rather than a failed one. So the fact that this file's
//! tests reach their assertions **at all** is the evidence that no walk in this
//! crate recurses; the assertions themselves then check that the refusal is the
//! right typed error rather than, say, a silent truncation.
//!
//! `RUST_MIN_STACK` is deliberately left unset (and asserted unset), so the run uses
//! the platform's ordinary thread stack and nothing is being propped up. The deep
//! case additionally runs on a thread with a **256 KiB** stack — far smaller than
//! any default — to make the claim independent of the host's default at all.

use std::thread;

use purrdf_cdt::{
    CdtError, MAX_ELEMENTS, MAX_LEXICAL_BYTES, MAX_NESTING_DEPTH, parse_list, parse_map,
};

/// Roughly 200 MB — comfortably past [`MAX_LEXICAL_BYTES`], and past what a 32-bit
/// wasm address space would tolerate holding parsed.
const OVERSIZED_BYTES: usize = 200 * 1024 * 1024;

#[test]
fn rust_min_stack_is_unset_so_the_evidence_is_honest() {
    assert!(
        std::env::var_os("RUST_MIN_STACK").is_none(),
        "RUST_MIN_STACK is set, so this file's stack-exhaustion evidence would be \
         about an enlarged stack rather than about the code never recursing"
    );
}

#[test]
fn a_million_deep_list_is_a_typed_error_on_a_256_kib_stack() {
    // 1,000,000 nested lists: two megabytes of input, and a depth that would blow
    // any thread stack in the world if a single level cost a stack frame.
    let depth = 1_000_000usize;
    let mut lexical = String::with_capacity(depth * 2);
    for _ in 0..depth {
        lexical.push('[');
    }
    for _ in 0..depth {
        lexical.push(']');
    }
    assert_eq!(lexical.len(), depth * 2);

    let handle = thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || parse_list(&lexical))
        .expect("the scanner thread starts");
    let error = handle
        .join()
        .expect("the scanner thread did not abort")
        .expect_err("a million-deep list is refused");

    let CdtError::DepthExceeded { offset, limit } = error else {
        panic!("expected a depth error, got {error:?}");
    };
    assert_eq!(limit, MAX_NESTING_DEPTH);
    // Refused at the delimiter that would have crossed the bound, not after
    // building a million frames.
    assert_eq!(offset, MAX_NESTING_DEPTH);
}

#[test]
fn a_million_deep_map_is_a_typed_error_too() {
    let depth = 1_000_000usize;
    let mut lexical = String::with_capacity(depth * 6);
    for _ in 0..depth {
        lexical.push_str("{\"k\":");
    }
    lexical.push('1');
    for _ in 0..depth {
        lexical.push('}');
    }
    let handle = thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || parse_map(&lexical))
        .expect("the scanner thread starts");
    let error = handle
        .join()
        .expect("the scanner thread did not abort")
        .expect_err("a million-deep map is refused");
    assert!(
        matches!(error, CdtError::DepthExceeded { .. }),
        "expected a depth error, got {error:?}"
    );
}

/// The two ~200 MB cases share one test so the peak resident set stays at one
/// oversized string rather than several running concurrently; each string is dropped
/// before the next is built.
#[test]
fn oversized_lexical_forms_are_typed_errors() {
    // A ~200 MB single-level list: two input bytes per element, so the shape that
    // amplifies hardest into a parsed value.
    {
        let mut lexical = String::with_capacity(OVERSIZED_BYTES + 2);
        lexical.push('[');
        while lexical.len() < OVERSIZED_BYTES {
            lexical.push_str("1,");
        }
        lexical.push_str("1]");
        assert!(lexical.len() > OVERSIZED_BYTES);
        let error = parse_list(&lexical).expect_err("a 200 MB list is refused");
        let CdtError::InputTooLarge { offset, length } = error else {
            panic!("expected an input-size error, got {error:?}");
        };
        assert_eq!(offset, MAX_LEXICAL_BYTES);
        assert_eq!(length, lexical.len());
    }

    // A ~200 MB single literal: one enormous element rather than many small ones.
    {
        let mut lexical = String::with_capacity(OVERSIZED_BYTES + 4);
        lexical.push_str("[\"");
        lexical.push_str(&"a".repeat(OVERSIZED_BYTES));
        lexical.push_str("\"]");
        let error = parse_list(&lexical).expect_err("a 200 MB literal is refused");
        assert!(
            matches!(error, CdtError::InputTooLarge { .. }),
            "expected an input-size error, got {error:?}"
        );
    }
}

#[test]
fn the_element_bound_refuses_a_small_input_that_would_build_a_huge_value() {
    // Under the byte bound (about 2 MB) but over the element bound: the two bounds
    // answer different attacks, so neither alone would catch this.
    let mut lexical = String::with_capacity(MAX_ELEMENTS * 2 + 4);
    lexical.push('[');
    for _ in 0..=MAX_ELEMENTS {
        lexical.push_str("1,");
    }
    lexical.push_str("1]");
    assert!(
        lexical.len() < MAX_LEXICAL_BYTES,
        "this case must pass the byte bound so it can exercise the element bound"
    );
    let error = parse_list(&lexical).expect_err("too many elements is refused");
    let CdtError::TooManyElements { limit, .. } = error else {
        panic!("expected an element-count error, got {error:?}");
    };
    assert_eq!(limit, MAX_ELEMENTS);
}

#[test]
fn a_deeply_nested_but_admissible_value_parses_renders_and_compares() {
    // Exactly at the bound: accepted, and every walk over it (render, equality,
    // ordering, depth) completes on the small stack too.
    let depth = MAX_NESTING_DEPTH;
    let mut lexical = String::new();
    for _ in 0..depth {
        lexical.push('[');
    }
    for _ in 0..depth {
        lexical.push(']');
    }
    let handle = thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let value = parse_list(&lexical).expect("depth at the bound is admissible");
            let canonical = value.canonical_lexical();
            let again = parse_list(&canonical).expect("the canonical form re-parses");
            (
                value.depth(),
                value.element_count(),
                again == value,
                canonical,
            )
        })
        .expect("the scanner thread starts");
    let (measured_depth, elements, equal, canonical) =
        handle.join().expect("the walks did not abort");
    assert_eq!(measured_depth, depth);
    assert_eq!(elements, depth - 1);
    assert!(equal);
    assert_eq!(canonical.len(), depth * 2);

    // One deeper is refused.
    let mut too_deep = String::new();
    for _ in 0..=depth {
        too_deep.push('[');
    }
    for _ in 0..=depth {
        too_deep.push(']');
    }
    assert!(matches!(
        parse_list(&too_deep),
        Err(CdtError::DepthExceeded { .. })
    ));
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Hostile lexical forms: a shape that crosses a bound is a **typed error**, a shape
//! that crosses none — however deep — is an ordinary value, and the test process
//! survives both.
//!
//! # What "the process exits 0" proves
//!
//! A Rust stack overflow is an `abort`: the process dies with SIGSEGV/SIGABRT and no
//! `catch_unwind`, `Result` or test harness can intercept it. A recursive-descent
//! scanner handed a million-deep `[[[[…` therefore does not *fail* the assertion
//! below — it kills the test binary before the assertion runs, and the harness
//! reports a crashed test rather than a failed one. The same holds for a recursive
//! `Drop`, `Clone`, `Debug`, renderer or comparator over the value that scanner
//! built. So the fact that this file's tests reach their assertions **at all** is the
//! evidence that no walk in this crate recurses; the assertions themselves then check
//! that a deep value is the value its lexical form denotes, and that a refusal is the
//! right typed error rather than, say, a silent truncation.
//!
//! `RUST_MIN_STACK` is deliberately left unset (and asserted unset), so the run uses
//! the platform's ordinary thread stack and nothing is being propped up. Every deep
//! case additionally runs on a thread with a **256 KiB** stack — far smaller than any
//! default — to make the claim independent of the host's default at all.
//!
//! # Depth is not a bound
//!
//! The crate has two bounds, [`MAX_ELEMENTS`] and [`MAX_LEXICAL_BYTES`], and no depth
//! bound: a level of nesting is one element of its container and a few bytes of its
//! form, so the two real bounds cap it, and every walk is iterative so it costs no
//! stack. The first half of this file proves that at a million levels through the
//! scanner and at a hundred thousand through the programmatic constructors, and the
//! second half proves the two real bounds still refuse — the same shapes one element
//! or one byte over the line.
//!
//! # The scanner is not the only way in
//!
//! A bound the *scanner* enforces is a property of one code path; a bound the *type*
//! enforces is a property of every value. The programmatic constructors —
//! `CdtValue::list`, `CdtValue::map`, `CdtTerm::composite`, `CdtTerm::triple` — are
//! therefore attacked with the same shapes, since a consumer assembling a value from
//! bindings never goes near a lexical form.

use std::thread;

use purrdf_cdt::{
    CDT_LIST, CdtEntry, CdtError, CdtKey, CdtLiteral, CdtTerm, CdtValue, MAX_ELEMENTS,
    MAX_LEXICAL_BYTES, canonical_lexical_len, list_equal, list_less_than, parse_list, parse_map,
    total_value_cmp,
};

/// Roughly 200 MB — comfortably past [`MAX_LEXICAL_BYTES`], and past what a 32-bit
/// wasm address space would tolerate holding parsed.
const OVERSIZED_BYTES: usize = 200 * 1024 * 1024;

/// The stack every deep case runs on: far smaller than any platform default, so a
/// walk that cost even a few bytes of stack per level would abort at these depths.
const SMALL_STACK: usize = 256 * 1024;

#[test]
fn rust_min_stack_is_unset_so_the_evidence_is_honest() {
    assert!(
        std::env::var_os("RUST_MIN_STACK").is_none(),
        "RUST_MIN_STACK is set, so this file's stack-exhaustion evidence would be \
         about an enlarged stack rather than about the code never recursing"
    );
}

/// Run `body` to completion on a [`SMALL_STACK`] thread and hand its result back. A
/// stack overflow on that thread aborts the whole process, which is the failure this
/// harness exists to make visible.
fn on_a_small_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(body)
        .expect("the thread starts")
        .join()
        .expect("the thread did not abort")
}

/// `[` × `depth`, then `]` × `depth`: a list nested `depth` deep with nothing in it.
fn nested_lists(depth: usize) -> String {
    let mut lexical = String::with_capacity(depth * 2);
    for _ in 0..depth {
        lexical.push('[');
    }
    for _ in 0..depth {
        lexical.push(']');
    }
    lexical
}

// ── Depth has no bound ─────────────────────────────────────────────────────────

/// A million nested lists — two megabytes of `[[[[…` — is a value, not a refusal: it
/// parses, renders back to itself, re-parses to an equal value, reports its depth
/// and element count, and drops, all on a 256 KiB stack.
///
/// The map twin follows in the same test rather than beside it so that the two
/// million-deep values, each hundreds of megabytes resident, are never live at once.
///
/// Expected values, derived from the shape and not from the code: `depth` opening
/// brackets are `depth` levels; the root is not an element of anything and every
/// other level is one element of the level above it, so there are `depth - 1`
/// elements; and the canonical form of a nest of empty lists is the input itself,
/// since there is no shorthand to spell out, so it is `depth * 2` bytes long.
#[test]
fn a_million_deep_list_and_map_parse_render_compare_and_drop_on_a_256_kib_stack() {
    let depth = 1_000_000usize;

    let lexical = nested_lists(depth);
    assert_eq!(lexical.len(), depth * 2);
    let (canonical_len, measured_depth, elements, again_depth, equal) =
        on_a_small_stack(move || {
            let value = parse_list(&lexical).expect("a million-deep list is a value");
            let canonical = value.canonical_lexical();
            assert_eq!(
                canonical, lexical,
                "nested empty lists have no shorthand to spell out"
            );
            let again = parse_list(&canonical).expect("the canonical form re-parses");
            let equal = again == value;
            let out = (
                canonical.len(),
                value.depth(),
                value.element_count(),
                again.depth(),
                equal,
            );
            drop(again);
            drop(value);
            out
        });
    assert_eq!(canonical_len, depth * 2);
    assert_eq!(measured_depth, depth);
    assert_eq!(elements, depth - 1);
    assert_eq!(again_depth, depth);
    assert!(equal, "the re-parsed canonical form is the same value");

    // The map twin: `{"k":` × depth, the integer `1`, `}` × depth. Every level has one
    // entry, so there are `depth` elements, the innermost being the `1`.
    let mut lexical = String::with_capacity(depth * 6 + 1);
    for _ in 0..depth {
        lexical.push_str("{\"k\":");
    }
    lexical.push('1');
    for _ in 0..depth {
        lexical.push('}');
    }
    // The canonical form spells the key and the leaf out. Its length is derived from
    // the one-level form, asserted here byte for byte: each level contributes `{`,
    // the key `"k"^^<…#string>`, `:` and `}`, and the innermost leaf is
    // `"1"^^<…#integer>`.
    const LEVEL: usize = 1 + 46 + 1 + 1;
    const LEAF: usize = 47;
    let one_level = parse_map("{\"k\":1}")
        .expect("the one-level map parses")
        .canonical_lexical();
    assert_eq!(
        one_level,
        "{\"k\"^^<http://www.w3.org/2001/XMLSchema#string>:\
         \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>}"
    );
    assert_eq!(one_level.len(), LEVEL + LEAF);
    let expected_canonical_len = depth * LEVEL + LEAF;
    assert!(
        expected_canonical_len < MAX_LEXICAL_BYTES,
        "the case must sit inside the byte bound to be about depth"
    );
    let (canonical_len, measured_depth, elements, again_depth, again_elements, same) =
        on_a_small_stack(move || {
            let value = parse_map(&lexical).expect("a million-deep map is a value");
            let canonical = value.canonical_lexical();
            let measured = (value.depth(), value.element_count());
            // The first value is released before the second is built, so the two
            // are never resident together; the canonical form is injective over
            // values, so byte-equal canonical forms are the same value.
            drop(value);
            let again = parse_map(&canonical).expect("the canonical form re-parses");
            let same = again.canonical_lexical() == canonical;
            let out = (
                canonical.len(),
                measured.0,
                measured.1,
                again.depth(),
                again.element_count(),
                same,
            );
            drop(again);
            out
        });
    assert_eq!(canonical_len, expected_canonical_len);
    assert_eq!(measured_depth, depth);
    assert_eq!(elements, depth);
    assert_eq!(again_depth, depth);
    assert_eq!(again_elements, depth);
    assert!(same, "the re-parsed canonical form is the same value");
}

/// Every walk over a deep value completes on a small stack: the value relations, the
/// total order, and equality, over two hundred-thousand-deep lists that differ only in
/// their innermost leaf.
///
/// Expected values from the shape: the two lists agree at every position down to the
/// leaf, where `1 < 2`, so the first is `<` the second, not equal to it, and equal to
/// itself; the total order agrees with `<` on this pair because it, too, walks to the
/// leaf and compares two `xsd:integer` literals' lexical forms.
#[test]
fn every_walk_over_a_hundred_thousand_deep_value_completes_on_a_256_kib_stack() {
    let depth = 100_000usize;
    let wrapped = |leaf: &str| {
        let mut lexical = String::with_capacity(depth * 2 + leaf.len());
        for _ in 0..depth {
            lexical.push('[');
        }
        lexical.push_str(leaf);
        for _ in 0..depth {
            lexical.push(']');
        }
        lexical
    };
    let (one, two) = (wrapped("1"), wrapped("2"));
    on_a_small_stack(move || {
        let one = parse_list(&one).expect("a deep list around 1 parses");
        let two = parse_list(&two).expect("a deep list around 2 parses");
        assert_eq!(one.depth(), depth);
        assert_eq!(one.element_count(), depth);
        let (items_one, items_two) = (
            one.as_list().expect("a list"),
            two.as_list().expect("a list"),
        );
        assert_eq!(list_less_than(items_one, items_two), Ok(true));
        assert_eq!(list_less_than(items_two, items_one), Ok(false));
        assert_eq!(list_equal(items_one, items_two), Ok(false));
        assert_eq!(list_equal(items_one, items_one), Ok(true));
        assert_eq!(total_value_cmp(&one, &two), std::cmp::Ordering::Less);
        assert_eq!(total_value_cmp(&one, &one), std::cmp::Ordering::Equal);
        assert_ne!(one, two);
        assert_eq!(one.clone(), one);
        assert_eq!(canonical_lexical_len(&one), one.canonical_lexical().len());
    });
}

/// A composite spelled as a `cdt:`-typed literal is compared as the value it denotes,
/// and a literal inside a literal inside a literal keeps being seen through: the walk
/// owns what it parses, so the nesting costs it heap and not stack.
///
/// The depth is what the escaping allows rather than what the walk allows: each
/// level quotes the level below it, so the lexical form doubles in size per level and
/// sixteen levels is a few megabytes. Expected values from the shape: the two chains
/// wrap `[1]` and `[2]`, so the first is `<` the second and not equal to it, and each
/// is equal to a second chain built the same way.
#[test]
fn composite_literals_nested_in_literals_are_seen_through_at_every_level() {
    fn chain(leaf: &str, levels: usize) -> CdtValue {
        let mut value = parse_list(leaf).expect("the leaf parses");
        for _ in 0..levels {
            let literal = CdtTerm::Literal(CdtLiteral::typed(value.canonical_lexical(), CDT_LIST));
            value = CdtValue::list(vec![literal]).expect("within both bounds");
        }
        value
    }
    on_a_small_stack(|| {
        let levels = 16;
        let (one, two, one_again) = (
            chain("[1]", levels),
            chain("[2]", levels),
            chain("[1]", levels),
        );
        let (items_one, items_two, items_again) = (
            one.as_list().expect("a list"),
            two.as_list().expect("a list"),
            one_again.as_list().expect("a list"),
        );
        assert_eq!(list_equal(items_one, items_again), Ok(true));
        assert_eq!(list_equal(items_one, items_two), Ok(false));
        assert_eq!(list_less_than(items_one, items_two), Ok(true));
        assert_eq!(list_less_than(items_two, items_one), Ok(false));
        // The literal spelling and the bracket spelling of one value are equal by
        // value at every level of the chain.
        let bracketed = {
            let mut value = parse_list("[1]").expect("the leaf parses");
            for _ in 0..levels {
                value =
                    CdtValue::list(vec![CdtTerm::composite(value).expect("within both bounds")])
                        .expect("within both bounds");
            }
            value
        };
        assert_eq!(
            list_equal(items_one, bracketed.as_list().expect("a list")),
            Ok(true)
        );
    });
}

// ── The programmatic constructors have no depth bound either ─────────────────────

/// A composite element the fixture is known to be within both bounds for.
fn composite(value: CdtValue) -> CdtTerm {
    CdtTerm::composite(value).expect("the fixture is within both bounds")
}

/// A hundred thousand levels built one at a time with no lexical form in sight —
/// through `CdtTerm::composite` and `CdtValue::list`, which measure every level — then
/// cloned, compared equal to the clone, `Debug`-formatted, and dropped, on a 256 KiB
/// stack. The map twin is built through `CdtValue::map`.
///
/// Expected values from the shape: `depth` levels, `depth - 1` elements, a canonical
/// form of `depth * 2` bytes. The `Debug` length is a formula whose two constants are
/// pinned against the derive's own spelling at depths one and two: the empty list
/// prints as `CdtValue { parts: List([]) }` (28 bytes), and each further level wraps
/// the one below in `CdtValue { parts: List([Composite(` and `)]) }` (39 bytes).
#[test]
fn programmatic_nesting_a_hundred_thousand_deep_builds_clones_prints_and_drops() {
    let depth = 100_000usize;
    const EMPTY_DEBUG: &str = "CdtValue { parts: List([]) }";
    const ONE_LEVEL_DEBUG: &str =
        "CdtValue { parts: List([Composite(CdtValue { parts: List([]) })]) }";
    assert_eq!(format!("{:?}", CdtValue::empty_list()), EMPTY_DEBUG);
    assert_eq!(
        format!(
            "{:?}",
            CdtValue::list(vec![composite(CdtValue::empty_list())]).expect("one level")
        ),
        ONE_LEVEL_DEBUG
    );
    let per_level = ONE_LEVEL_DEBUG.len() - EMPTY_DEBUG.len();
    assert_eq!((EMPTY_DEBUG.len(), per_level), (28, 39));
    let expected_debug_len = EMPTY_DEBUG.len() + per_level * (depth - 1);

    on_a_small_stack(move || {
        let mut value = CdtValue::empty_list();
        for _ in 1..depth {
            value = CdtValue::list(vec![composite(value)]).expect("no depth is refused");
        }
        assert_eq!(value.depth(), depth);
        assert_eq!(value.element_count(), depth - 1);
        assert_eq!(canonical_lexical_len(&value), depth * 2);
        let copy = value.clone();
        assert_eq!(copy, value);
        assert_eq!(copy.depth(), depth);
        assert_eq!(format!("{value:?}").len(), expected_debug_len);
        // An element that bypassed `CdtTerm::composite` is measured by the value
        // constructor instead, and admitted just the same.
        let smuggled = CdtValue::list(vec![CdtTerm::Composite(Box::new(copy))])
            .expect("a raw element one level deeper is within both bounds");
        assert_eq!(smuggled.depth(), depth + 1);
        drop(smuggled);
        drop(value);

        let mut map = CdtValue::empty_map();
        for _ in 1..depth {
            map = CdtValue::map(vec![CdtEntry {
                key: CdtKey::Literal(CdtLiteral::plain("k")),
                value: composite(map),
            }])
            .expect("no depth is refused");
        }
        assert_eq!(map.depth(), depth);
        assert_eq!(map.element_count(), depth - 1);
        assert_eq!(map.clone(), map);
        drop(map);
    });
}

// ── The two bounds still refuse ──────────────────────────────────────────────────

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

/// Depth reaches the element bound like any other shape: a nest one level deeper than
/// the bound admits is one element over it, and is refused as such — by the scanner
/// at the delimiter that would open the element past the bound, and by the
/// programmatic constructors on the prospective value. The neighbour one level
/// shallower is admitted.
///
/// Expected values from the shape: a nest of `d` empty lists has `d - 1` elements, so
/// `MAX_ELEMENTS + 1` levels is admitted and `MAX_ELEMENTS + 2` is not. The scanner's
/// offset is the position of the `[` that would have been element `MAX_ELEMENTS + 1`,
/// which is byte `MAX_ELEMENTS + 1` (the root's bracket is byte 0).
#[test]
fn nesting_meets_the_element_bound_and_nothing_else() {
    let admitted = nested_lists(MAX_ELEMENTS + 1);
    let refused = nested_lists(MAX_ELEMENTS + 2);
    let (depth, elements, error) = on_a_small_stack(move || {
        let value = parse_list(&admitted).expect("a nest inside the element bound is a value");
        let measured = (value.depth(), value.element_count());
        drop(value);
        let error = parse_list(&refused).expect_err("a nest past the element bound is refused");
        (measured.0, measured.1, error)
    });
    assert_eq!(depth, MAX_ELEMENTS + 1);
    assert_eq!(elements, MAX_ELEMENTS);
    let CdtError::TooManyElements { offset, limit } = error else {
        panic!("expected an element-count error, got {error:?}");
    };
    assert_eq!(limit, MAX_ELEMENTS);
    assert_eq!(offset, MAX_ELEMENTS + 1);
}

#[test]
fn programmatic_element_counts_are_refused_at_the_same_bound() {
    let too_many = vec![CdtTerm::Null; MAX_ELEMENTS + 1];
    assert!(matches!(
        CdtValue::list(too_many),
        Err(CdtError::TooManyElements { limit, .. }) if limit == MAX_ELEMENTS
    ));

    // A triple term is where three separately admissible elements combine into one
    // that is not: each third is inside the bound, the union is not.
    let third =
        || composite(CdtValue::list(vec![CdtTerm::Null; MAX_ELEMENTS / 2]).expect("a half"));
    assert!(matches!(
        CdtTerm::triple(third(), third(), third()),
        Err(CdtError::TooManyElements { .. })
    ));
}

#[test]
fn a_map_built_programmatically_is_bounded_too() {
    let entries: Vec<CdtEntry> = (0..=MAX_ELEMENTS)
        .map(|index| CdtEntry {
            key: CdtKey::Literal(CdtLiteral::plain(index.to_string())),
            value: CdtTerm::Null,
        })
        .collect();
    assert!(matches!(
        CdtValue::map(entries),
        Err(CdtError::TooManyElements { limit, .. }) if limit == MAX_ELEMENTS
    ));
}

/// The canonical form can be far longer than the input that produced it, so the byte
/// bound is checked on both.
///
/// A raw control code point costs one input byte and six canonical bytes (`\u0001`),
/// which is a sixfold amplification well inside the element bound. Checking only the
/// input would therefore admit a value whose own lexical form is larger than any host
/// is asked to hold.
#[test]
fn a_lexical_form_whose_canonical_form_is_oversized_is_refused() {
    let payload = MAX_LEXICAL_BYTES / 6 + 64;
    let mut lexical = String::with_capacity(payload + 4);
    lexical.push_str("[\"");
    for _ in 0..payload {
        lexical.push('\u{1}');
    }
    lexical.push_str("\"]");
    assert!(
        lexical.len() < MAX_LEXICAL_BYTES,
        "the input must pass the input-length check so this exercises the canonical one"
    );
    let error = parse_list(&lexical).expect_err("an oversized canonical form is refused");
    let CdtError::InputTooLarge { offset, length } = error else {
        panic!("expected an input-size error, got {error:?}");
    };
    assert_eq!(offset, MAX_LEXICAL_BYTES);
    assert!(length > MAX_LEXICAL_BYTES);

    // The same payload one order of magnitude smaller is admitted, and the length the
    // refusal reported is the length the renderer would actually have produced.
    let mut smaller = String::from("[\"");
    for _ in 0..1024 {
        smaller.push('\u{1}');
    }
    smaller.push_str("\"]");
    let value = parse_list(&smaller).expect("a small control-character payload is admissible");
    assert_eq!(
        canonical_lexical_len(&value),
        value.canonical_lexical().len()
    );
}

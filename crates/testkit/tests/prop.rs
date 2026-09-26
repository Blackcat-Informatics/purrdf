// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The property harness, checked from outside.
//!
//! * Every generator respects its constraints — range bounds, collection
//!   sizes, and for strings a full match of the pattern under the `regex`
//!   crate, over 10 000 samples of every pattern the workspace's properties
//!   use.
//! * Shrinking lands on the exact minimal counterexample of planted failures,
//!   and a shrunk value still satisfies its generator's constraints.
//! * A filter that rejects everything fails with the reject-limit error, and
//!   its neighbour that rejects half its candidates passes.
//! * A passing property runs exactly its configured number of cases.
//! * The printed choice sequence replays the shrunk input.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::process::Command;

use purrdf_testkit::prop::collection::SizeRange;
use purrdf_testkit::prop::prelude::*;
use purrdf_testkit::prop::state_machine::{self, ReferenceStateMachine, SystemUnderTest};
use purrdf_testkit::prop::{
    CASES_VARIABLE, Choices, FailedCase, Failure, HexError, Invalid, RunSummary, Runner,
    SEED_VARIABLE, parse_seed, replay, seed_for,
};

/// Every regex pattern a property in this workspace generates strings from,
/// plus `\PC*`, which `any::<String>()` generates from.
const WORKSPACE_PATTERNS: &[&str] = &[
    "\\PC*",
    "-?[0-9]{1,6}",
    ".{0,10}",
    ".{0,200}",
    ".{0,24}",
    ".{0,30}",
    ".{0,64}",
    ".{0,8}",
    "[ -~]{0,32}",
    "[0-9][a-z0-9]{0,6}",
    "[0-9][a-zA-Z0-9]{3}",
    "[0-9]{3}",
    "[0-9a-wyzA-WYZ]",
    "[A-Za-z0-9 ]{0,32}",
    "[A-Za-z0-9._-]{0,12}",
    "[A-Za-z0-9:@/_-]{1,24}",
    "[A-Za-z0-9_ /?.-]{1,24}",
    "[a-c0-9]{0,4}",
    "[a-c:/# ]{0,4}",
    "[a-c:/#]{0,4}",
    "[a-z0-9._~-]{1,6}",
    "[a-z0-9]{0,4}",
    "[a-z0-9]{0,6}",
    "[a-z0-9]{1,5}(/[a-z0-9]{1,5}){0,3}",
    "[a-z:#/\\t]{0,12}",
    "[a-zA-Z0-9 ?<>{}().*+/^!|:_\"@-]{0,80}",
    "[a-zA-Z0-9 ]{0,8}",
    "[a-zA-Z0-9-]{100,600}",
    "[a-zA-Z0-9._~!$&'()*+,;=-]{0,8}",
    "[a-zA-Z0-9._~!$&'()*+,;=:@/?-]{0,10}",
    "[a-zA-Z0-9]{1,8}",
    "[a-zA-Z0-9]{2,8}",
    "[a-zA-Z0-9]{5,8}",
    "[a-zA-Z]{2,3}",
    "[a-zA-Z]{2}",
    "[a-zA-Z]{3}",
    "[a-zA-Z]{4}",
    "[a-zA-Z]{5,8}",
    "[a-z][a-z0-9-]{0,10}(\\.[a-z][a-z0-9-]{0,10}){0,2}",
    "[a-z][a-z0-9]{0,2}-[a-z0-9]{1,3}",
    "[a-z][a-z0-9]{0,2}\\.[a-z0-9]{1,3}",
    "[a-z][a-z0-9]{0,5}",
    "[a-z][a-z0-9]{0,6}",
    "[a-z][a-z0-9]{0,8}(\\.[a-z][a-z0-9]{0,8}){0,3}",
    "[a-z]{1,4}",
    "[a-z]{1,6}",
    "[a-z]{1,8}",
    "[a-z]{2}",
    "[xX]",
    "_[a-z0-9]{0,6}",
];

const SAMPLES: usize = 10_000;

fn samples<S: Strategy>(strategy: &S, name: &str, count: usize) -> Vec<S::Value> {
    let mut choices = Choices::random(seed_for(name));
    (0..count)
        .map(|_| strategy.generate(&mut choices).expect("generates"))
        .collect()
}

/// The failure a run must end in, shrunk.
fn failure<S, F>(strategy: &S, name: &str, test: F) -> FailedCase
where
    S: Strategy,
    F: Fn(S::Value) -> Result<(), TestCaseError>,
{
    match Runner::with_seed(Config::default(), name, seed_for(name)).run(strategy, test) {
        Err(Failure::Failed(case)) => *case,
        other => panic!("expected a failed case, got {other:?}"),
    }
}

// ── Generator constraints ──────────────────────────────────────────────────────

#[test]
fn ranges_stay_inside_their_bounds_and_reach_both_ends() {
    let values = samples(&(3u8..9), "ranges_u8", SAMPLES);
    assert!(values.iter().all(|value| (3..9).contains(value)));
    assert_eq!(values.iter().min(), Some(&3));
    assert_eq!(values.iter().max(), Some(&8));

    let values = samples(&(-5i32..=5), "ranges_i32", SAMPLES);
    assert!(values.iter().all(|value| (-5..=5).contains(value)));
    assert_eq!(values.iter().min(), Some(&-5));
    assert_eq!(values.iter().max(), Some(&5));

    let values = samples(&(i64::MIN..=i64::MAX), "ranges_i64_full", 1_000);
    assert!(values.iter().any(|value| *value < 0) && values.iter().any(|value| *value > 0));

    let values = samples(&(2usize..=4), "ranges_usize", SAMPLES);
    assert_eq!(
        values.iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from([2, 3, 4])
    );

    let values = samples(&(u64::MAX - 1..=u64::MAX), "ranges_u64_top", 1_000);
    assert!(values.iter().all(|value| *value >= u64::MAX - 1));
}

#[test]
fn collections_respect_their_size_ranges() {
    let values = samples(
        &prop::collection::vec(any::<u8>(), 2..6),
        "vec_sizes",
        SAMPLES,
    );
    let lengths: BTreeSet<usize> = values.iter().map(Vec::len).collect();
    assert_eq!(lengths, BTreeSet::from([2, 3, 4, 5]));

    let values = samples(&prop::collection::vec(any::<u8>(), 3), "vec_exact", 1_000);
    assert!(values.iter().all(|value| value.len() == 3));

    let values = samples(
        &prop::collection::btree_set(0u8..10, 1..=4),
        "set_sizes",
        SAMPLES,
    );
    assert!(values.iter().all(|set| (1..=4).contains(&set.len())));
    assert_eq!(values.iter().map(BTreeSet::len).max(), Some(4));

    let values = samples(
        &prop::collection::btree_map(0u8..4, any::<bool>(), 0..=4),
        "map_sizes",
        SAMPLES,
    );
    assert!(values.iter().all(|map| map.len() <= 4));
    assert!(values.iter().any(|map| map.len() == 4));

    let values = samples(&prop::option::of(0u8..3), "option", SAMPLES);
    let nones = values.iter().filter(|value| value.is_none()).count();
    assert!(
        (4_000..6_000).contains(&nones),
        "{nones} of {SAMPLES} were None"
    );
    assert!(values.iter().flatten().all(|value| *value < 3));
}

#[test]
fn selections_unions_and_indexes_draw_only_what_they_offer() {
    let values = samples(
        &prop::sample::select(vec!['x', 'y', 'z']),
        "select",
        SAMPLES,
    );
    assert_eq!(
        values.iter().copied().collect::<BTreeSet<_>>(),
        BTreeSet::from(['x', 'y', 'z'])
    );

    let weighted = prop_oneof![9 => Just(0u16), 1 => Just(1u16)];
    let ones = samples(&weighted, "weighted", SAMPLES)
        .iter()
        .filter(|value| **value == 1)
        .count();
    assert!(
        (700..1_300).contains(&ones),
        "{ones} of {SAMPLES} took the 1-weight arm"
    );

    let indexes = samples(&any::<prop::sample::Index>(), "index", SAMPLES);
    assert!(indexes.iter().all(|index| index.index(7) < 7));
    assert_eq!(
        indexes
            .iter()
            .map(|index| index.index(7))
            .collect::<BTreeSet<_>>()
            .len(),
        7
    );

    let chars = samples(&any::<char>(), "chars", SAMPLES);
    assert!(chars.iter().any(|c| u32::from(*c) > 0xFFFF));
}

#[test]
fn recursion_respects_its_depth() {
    #[derive(Debug, Clone)]
    enum Tree {
        Leaf,
        Node(Vec<Self>),
    }
    fn depth(tree: &Tree) -> u32 {
        match tree {
            Tree::Leaf => 0,
            Tree::Node(children) => 1 + children.iter().map(depth).max().unwrap_or(0),
        }
    }
    let strategy = Just(Tree::Leaf).prop_recursive(3, 16, 3, |inner| {
        prop::collection::vec(inner, 0..4).prop_map(Tree::Node)
    });
    let trees = samples(&strategy, "recursion", SAMPLES);
    assert!(trees.iter().all(|tree| depth(tree) <= 3));
    assert!(trees.iter().any(|tree| depth(tree) == 3));
    assert!(trees.iter().any(|tree| depth(tree) == 0));
}

#[test]
fn every_workspace_pattern_generates_only_full_matches() {
    for pattern in WORKSPACE_PATTERNS {
        let oracle = regex::Regex::new(&format!("^(?:{pattern})$")).expect("the oracle compiles");
        let strategy = prop::string::regex(pattern);
        for text in samples(&strategy, pattern, SAMPLES) {
            assert!(oracle.is_match(&text), "/{pattern}/ generated {text:?}");
        }
    }
}

/// The Rust string literal body `text`, unescaped (the escapes patterns use).
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some(other @ ('\\' | '"' | '\'')) => out.push(other),
            other => panic!("an escape this scan does not read: \\{other:?} in {text:?}"),
        }
    }
    out
}

/// Every pattern handed to `prop::string::regex` in the workspace's sources, so
/// [`WORKSPACE_PATTERNS`] can be checked against it in both directions.
fn patterns_in_the_workspace() -> BTreeSet<String> {
    let call =
        regex::Regex::new(r#"prop::string::regex\("((?:\\.|[^"\\])*)"\)"#).expect("compiles");
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .to_path_buf();
    let mut found = BTreeSet::new();
    let mut pending = vec![crates.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("readable") {
            let path = entry.expect("readable").path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if name != "target" && path != crates.join("testkit") {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).expect("UTF-8 source");
                for capture in call.captures_iter(&text) {
                    found.insert(unescape(&capture[1]));
                }
            }
        }
    }
    found
}

#[test]
fn the_pattern_list_is_exactly_the_workspace_patterns() {
    let used = patterns_in_the_workspace();
    assert!(
        used.len() > 40,
        "the scan found the workspace's patterns: {used:?}"
    );
    let listed: BTreeSet<String> = WORKSPACE_PATTERNS
        .iter()
        .filter(|pattern| **pattern != "\\PC*")
        .map(|pattern| (*pattern).to_owned())
        .collect();
    let unlisted: Vec<&String> = used.difference(&listed).collect();
    let stale: Vec<&String> = listed.difference(&used).collect();
    assert!(
        unlisted.is_empty(),
        "patterns used but not oracle-checked: {unlisted:?}"
    );
    assert!(
        stale.is_empty(),
        "patterns listed but no longer used: {stale:?}"
    );
}

#[test]
fn strings_cover_their_classes_and_honour_unicode() {
    let texts = samples(&any::<String>(), "any_string", SAMPLES);
    assert!(
        texts.iter().any(|text| !text.is_ascii()),
        "\\PC reaches past ASCII"
    );
    assert!(texts.iter().all(|text| !text.chars().any(char::is_control)));

    let letters: BTreeSet<char> = samples(&prop::string::regex("[a-e]"), "class", SAMPLES)
        .iter()
        .flat_map(|text| text.chars())
        .collect();
    assert_eq!(letters, "abcde".chars().collect());

    let alternation = samples(
        &prop::string::regex("cat|dog|(?i:ox)"),
        "alternation",
        SAMPLES,
    );
    let spelled: BTreeSet<&str> = alternation.iter().map(String::as_str).collect();
    assert!(spelled.is_superset(&BTreeSet::from(["cat", "dog", "ox", "OX", "Ox", "oX"])));
    assert_eq!(spelled.len(), 6);

    let repeated = samples(&prop::string::regex("a{2,4}"), "repetition", SAMPLES);
    assert_eq!(
        repeated.iter().map(String::len).collect::<BTreeSet<_>>(),
        BTreeSet::from([2, 3, 4])
    );
    let unbounded = samples(&prop::string::regex("b+"), "unbounded", SAMPLES);
    assert!(unbounded.iter().all(|text| (1..=32).contains(&text.len())));
    let anchored = samples(&prop::string::regex("^[0-9]+$"), "anchored", 1_000);
    assert!(
        anchored
            .iter()
            .all(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
    );
}

#[test]
fn a_pattern_that_cannot_be_generated_is_refused_and_its_neighbour_is_not() {
    let refused = prop::string::string_regex("\\bword\\b").expect_err("a word boundary");
    assert!(
        refused.to_string().contains("cannot be generated"),
        "{refused}"
    );
    assert!(prop::string::string_regex("(unclosed").is_err());
    prop::string::string_regex("word").expect("the valid neighbour parses");
    prop::string::string_regex("^word$").expect("anchors are generated as nothing");
}

#[test]
fn any_byte_string_drives_a_generator_within_its_constraints() {
    let strategy = (
        prop::collection::vec(10u16..20, 1..5),
        prop::string::regex("[x-z]{2,3}"),
        prop::option::of(any::<bool>()),
    );
    let oracle = regex::Regex::new("^[x-z]{2,3}$").expect("compiles");
    let mut source = Choices::random(seed_for("byte_strings"));
    for _ in 0..2_000 {
        let length = source.draw(64).expect("draws") as usize;
        let bytes: Vec<u8> = (0..length)
            .map(|_| source.draw(255).expect("draws") as u8)
            .collect();
        let (items, text, _) = strategy
            .generate(&mut Choices::from_bytes(&bytes))
            .expect("any bytes generate");
        assert!((1..5).contains(&items.len()) && items.iter().all(|item| (10..20).contains(item)));
        assert!(oracle.is_match(&text), "{text:?}");
    }
    // The empty string is the all-zero sequence: every generator's simplest value.
    assert_eq!(
        strategy.generate(&mut Choices::from_bytes(&[])),
        Ok((vec![10], "xx".to_owned(), None))
    );
}

// ── Shrinking ──────────────────────────────────────────────────────────────────

#[test]
fn a_planted_threshold_shrinks_to_exactly_the_threshold() {
    let case = failure(&(0u32..10_000), "threshold", |x| {
        prop_assert!(x < 1000);
        Ok(())
    });
    assert_eq!(case.minimal, "1000");
    assert_eq!(replay(&(0u32..10_000), &case.choices_hex), 1000);
}

#[test]
fn a_planted_length_bound_shrinks_to_three_minimal_elements() {
    let strategy = prop::collection::vec(0u8..100, 0..20);
    let case = failure(&strategy, "length", |items| {
        prop_assert!(items.len() < 3);
        Ok(())
    });
    assert_eq!(case.minimal, "[0, 0, 0]");
    assert_eq!(replay(&strategy, &case.choices_hex), vec![0, 0, 0]);

    // The minimal elements are the generator's own minimum, not zero.
    let strategy = prop::collection::vec(7u8..100, 0..20);
    let case = failure(&strategy, "length_offset", |items| {
        prop_assert!(items.len() < 3);
        Ok(())
    });
    assert_eq!(case.minimal, "[7, 7, 7]");
}

#[test]
fn shrunk_values_still_satisfy_their_generators() {
    // Every value fails; the shrink target is each generator's minimum.
    let case = failure(&(1_500u32..10_000), "floor", |_| {
        Err(TestCaseError::fail("always"))
    });
    assert_eq!(case.minimal, "1500");

    let pattern = "[b-d]{3,5}-[0-9]";
    let case = failure(&prop::string::regex(pattern), "pattern", |_| {
        Err(TestCaseError::fail("always"))
    });
    assert_eq!(case.minimal, "\"bbb-0\"");

    let set = prop::collection::btree_set(5u8..50, 2..6);
    let case = failure(&set, "set", |_| Err(TestCaseError::fail("always")));
    let shrunk = replay(&set, &case.choices_hex);
    assert!((2..6).contains(&shrunk.len()) && shrunk.iter().all(|x| (5..50).contains(x)));
    assert_eq!(shrunk, BTreeSet::from([5, 6]));

    // Through a map and a filter: the smallest odd multiple of three above 100.
    let mapped = (0u32..1_000)
        .prop_map(|x| x * 3)
        .prop_filter("odd", |x| x % 2 == 1);
    let case = failure(&mapped, "mapped", |x| {
        prop_assert!(x <= 100);
        Ok(())
    });
    assert_eq!(case.minimal, "105");

    // Through a flat map: a length first, then a vector of exactly that length.
    let dependent = (1usize..8).prop_flat_map(|len| prop::collection::vec(0u8..10, len));
    let case = failure(&dependent, "dependent", |items| {
        prop_assert!(items.iter().map(|&x| u32::from(x)).sum::<u32>() < 12);
        Ok(())
    });
    let shrunk = replay(&dependent, &case.choices_hex);
    assert!((1..8).contains(&shrunk.len()) && shrunk.iter().all(|&x| x < 10));
    assert_eq!(
        shrunk.iter().map(|&x| u32::from(x)).sum::<u32>(),
        12,
        "{shrunk:?}"
    );
    assert_eq!(
        shrunk.len(),
        2,
        "the length drawn up front shrinks too: {shrunk:?}"
    );
}

#[test]
fn a_panicking_property_is_shrunk_like_a_returned_failure() {
    let case = failure(&(0i64..=1_000_000), "panics", |x| {
        assert!(x < 4_242, "too big: {x}");
        Ok(())
    });
    assert_eq!(case.minimal, "4242");
    assert!(case.message.contains("too big: 4242"), "{}", case.message);
}

#[test]
fn the_failure_report_prints_the_input_and_a_replayable_sequence() {
    let strategy = (0u8..=255, prop::string::regex("[a-z]{0,4}"));
    let case = failure(&strategy, "report", |(number, text)| {
        prop_assert!(
            number < 200 || text.is_empty(),
            "number {number} text {text:?}"
        );
        Ok(())
    });
    assert_eq!(case.minimal, "(200, \"a\")");
    assert_eq!(replay(&strategy, &case.choices_hex), (200, "a".to_owned()));
    let report = Failure::Failed(Box::new(case.clone())).to_string();
    for needle in [
        "property `report` failed after",
        "minimal failing input: (200, \"a\")",
        &format!(
            "choice sequence (hex, for Choices::from_hex): {}",
            case.choices_hex
        ),
        "PURRDF_PROP_SEED=0x",
        "number 200 text \"a\"",
    ] {
        assert!(
            report.contains(needle),
            "{needle:?} missing from:\n{report}"
        );
    }
}

// ── Rejection ──────────────────────────────────────────────────────────────────

#[test]
fn a_filter_that_rejects_everything_exhausts_the_reject_budget() {
    let strategy = (0u32..100).prop_filter("nothing passes", |_| false);
    let result = Runner::with_seed(Config::default(), "reject_all", 1).run(&strategy, |_| Ok(()));
    match result {
        Err(Failure::TooManyRejects { reason, passed, .. }) => {
            assert_eq!(reason, "nothing passes");
            assert_eq!(passed, 0);
        }
        other => panic!("expected the reject-limit error, got {other:?}"),
    }
}

#[test]
fn a_filter_that_rejects_half_passes() {
    let strategy = (0u32..100).prop_filter("even only", |x| x % 2 == 0);
    let seen = Cell::new(0u32);
    let summary = Runner::with_seed(Config::default(), "reject_half", 1)
        .run(&strategy, |x| {
            seen.set(seen.get() + 1);
            prop_assert_eq!(x % 2, 0);
            Ok(())
        })
        .expect("a half-rejecting filter passes");
    assert_eq!(summary.cases, 256);
    assert_eq!(seen.get(), 256);
    assert!(
        summary.rejects > 50,
        "about half the candidates were rejected: {summary:?}"
    );
}

#[test]
fn an_assumption_rejects_the_case_and_the_run_still_reaches_its_count() {
    let seen = Cell::new(0u32);
    let summary = Runner::with_seed(Config::with_cases(40), "assume", 3)
        .run(&(0u32..10), |x| {
            prop_assume!(x != 3);
            seen.set(seen.get() + 1);
            Ok(())
        })
        .expect("passes");
    assert_eq!(seen.get(), 40);
    assert_eq!(summary.cases, 40);
    assert!(summary.rejects > 0);

    let refused =
        Runner::with_seed(Config::with_cases(40), "assume_never", 3).run(&(0u32..10), |_| {
            prop_assume!(false);
            Ok(())
        });
    assert!(
        matches!(refused, Err(Failure::TooManyRejects { .. })),
        "{refused:?}"
    );
}

// ── Case counts and seeds ──────────────────────────────────────────────────────

#[test]
fn a_passing_property_runs_exactly_its_configured_cases() {
    for cases in [1, 7, 64, 256, 1_000] {
        let seen = Cell::new(0u32);
        let summary = Runner::with_seed(Config::with_cases(cases), "count", 5)
            .run(&any::<u64>(), |_| {
                seen.set(seen.get() + 1);
                Ok(())
            })
            .expect("passes");
        assert_eq!(seen.get(), cases);
        assert_eq!(summary, RunSummary { cases, rejects: 0 });
    }
    assert_eq!(Config::default().cases, 256);
}

thread_local! {
    static MACRO_CASES: Cell<u32> = const { Cell::new(0) };
}

prop_test! {
    #![prop_config(Config::with_cases(37))]

    fn counted_by_the_macro(x in 0u8..10, (a, b) in (any::<bool>(), Just(4u8))) {
        MACRO_CASES.with(|cases| cases.set(cases.get() + 1));
        prop_assert!(x < 10);
        prop_assert_eq!(b, 4);
        let _ = a;
    }
}

#[test]
fn the_macro_runs_its_configured_cases() {
    MACRO_CASES.with(|cases| cases.set(0));
    counted_by_the_macro();
    assert_eq!(MACRO_CASES.with(Cell::get), 37);
}

prop_test! {
    /// The default configuration: 256 cases.
    fn counted_by_default(_x in any::<u8>()) {
        MACRO_CASES.with(|cases| cases.set(cases.get() + 1));
    }

    fn fails_above_nine(x in 0u8..=255) {
        prop_assert!(x <= 9, "x was {}", x);
    }
}

#[test]
fn the_macro_defaults_to_256_cases_and_reports_a_shrunk_failure() {
    MACRO_CASES.with(|cases| cases.set(0));
    counted_by_default();
    assert_eq!(MACRO_CASES.with(Cell::get), 256);

    let payload = std::panic::catch_unwind(fails_above_nine).expect_err("the property fails");
    let message = payload
        .downcast_ref::<String>()
        .expect("the report is a String");
    assert!(
        message.contains("minimal failing input: (10,)"),
        "{message}"
    );
    assert!(message.contains("x was 10"), "{message}");
    // The default seed is the module path and name's, unless the seed variable
    // replaces it for this run.
    let seed = std::env::var(SEED_VARIABLE).map_or_else(
        |_| seed_for("prop::fails_above_nine"),
        |text| parse_seed(&text).expect("the run's seed variable parses"),
    );
    assert!(message.contains(&format!("{seed:#018x}")), "{message}");
}

#[test]
fn the_default_seed_is_a_function_of_the_name() {
    assert_eq!(seed_for("a::b"), seed_for("a::b"));
    assert_ne!(seed_for("a::b"), seed_for("a::c"));
    let first = samples(&any::<u64>(), "same", 16);
    assert_eq!(first, samples(&any::<u64>(), "same", 16));
    assert_ne!(first, samples(&any::<u64>(), "other", 16));
}

#[test]
fn seeds_parse_in_decimal_and_hex_and_refuse_anything_else() {
    assert_eq!(parse_seed("42"), Ok(42));
    assert_eq!(parse_seed("0x2a"), Ok(42));
    assert_eq!(parse_seed(" 0X2A "), Ok(42));
    assert!(parse_seed("forty-two").is_err());
    assert!(parse_seed("0x").is_err());
    assert!(parse_seed("-1").is_err());
}

/// Run by [`the_seed_variable_overrides_the_default_seed`] as a child process:
/// prints the seed a runner picks for a fixed name.
#[test]
fn print_the_runner_seed() {
    println!(
        "runner seed = {:#x}",
        Runner::new(Config::default(), "seed_probe").seed()
    );
}

#[test]
fn the_seed_variable_overrides_the_default_seed() {
    let run = |seed: Option<&str>| {
        let mut command = Command::new(std::env::current_exe().expect("the test binary"));
        command.args([
            "--exact",
            "print_the_runner_seed",
            "--nocapture",
            "--test-threads=1",
        ]);
        match seed {
            Some(seed) => command.env(SEED_VARIABLE, seed),
            None => command.env_remove(SEED_VARIABLE),
        };
        let output = command.output().expect("run the test binary");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )
    };
    let (ok, stdout) = run(None);
    assert!(ok, "{stdout}");
    assert!(
        stdout.contains(&format!("runner seed = {:#x}", seed_for("seed_probe"))),
        "{stdout}"
    );
    let (ok, stdout) = run(Some("0x1234"));
    assert!(ok, "{stdout}");
    assert!(stdout.contains("runner seed = 0x1234"), "{stdout}");
    let (ok, _) = run(Some("not-a-seed"));
    assert!(!ok, "an unparseable seed is refused rather than ignored");
}

/// Run by [`the_cases_variable_sets_the_count_and_refuses_garbage`] as a child
/// process: prints the case count a property offering a deeper search picks.
#[test]
fn print_the_case_count() {
    println!("case count = {}", prop::cases_from_env(7));
}

#[test]
fn the_cases_variable_sets_the_count_and_refuses_garbage() {
    let run = |cases: Option<&str>| {
        let mut command = Command::new(std::env::current_exe().expect("the test binary"));
        command.args([
            "--exact",
            "print_the_case_count",
            "--nocapture",
            "--test-threads=1",
        ]);
        match cases {
            Some(cases) => command.env(CASES_VARIABLE, cases),
            None => command.env_remove(CASES_VARIABLE),
        };
        let output = command.output().expect("run the test binary");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )
    };
    let (ok, stdout) = run(None);
    assert!(ok && stdout.contains("case count = 7"), "{stdout}");
    let (ok, stdout) = run(Some("12"));
    assert!(ok && stdout.contains("case count = 12"), "{stdout}");
    for refused in ["0", "many", "-3"] {
        let (ok, _) = run(Some(refused));
        assert!(!ok, "`{refused}` is refused rather than ignored");
    }
}

// ── The rest of the public surface ─────────────────────────────────────────────

#[test]
fn choices_report_their_mode_and_length() {
    let mut random = Choices::random(1);
    assert!(random.is_random() && random.is_empty());
    assert_eq!(random.len(), 0);
    random.draw(9).expect("draws");
    random.weighted_bool(0.5).expect("draws");
    assert_eq!(random.len(), 2);
    assert!(!random.is_empty());
    let replayed = Choices::from_bytes(&random.to_bytes());
    assert!(!replayed.is_random() && replayed.is_empty());
    assert_eq!(random.to_hex().len(), 4, "two one-byte draws");

    let error: HexError = Choices::from_hex("abc").expect_err("odd length");
    assert!(error.to_string().contains("even number"), "{error}");
    Choices::from_hex("abcd").expect("the valid neighbour decodes");
}

#[test]
fn an_exhausted_budget_and_a_runaway_generator_are_invalid() {
    let mut choices = Choices::from_bytes(&[]);
    let never = (0u8..10).prop_filter("never", |_| false);
    assert_eq!(never.generate(&mut choices), Err(Invalid::RejectLimit));
    assert!(Invalid::RejectLimit.to_string().contains("reject budget"));
    assert!(
        Invalid::Overrun
            .to_string()
            .contains(&Choices::MAX_DRAWS.to_string())
    );

    let weights = [0, 3, 0];
    let mut random = Choices::random(4);
    for _ in 0..200 {
        assert_eq!(
            random.weighted_index(&weights),
            Ok(1),
            "zero weights are never drawn"
        );
    }
    let mut empty = Choices::from_bytes(&[]);
    assert_eq!(
        empty.weighted_index(&weights),
        Ok(0),
        "a replay reads the recorded index"
    );
}

#[test]
fn size_ranges_convert_from_every_range_form() {
    let exact = SizeRange::from(3);
    assert_eq!((exact.min(), exact.max()), (3, 3));
    let half_open = SizeRange::from(2..5);
    assert_eq!((half_open.min(), half_open.max()), (2, 4));
    assert!(half_open.contains(2) && half_open.contains(4) && !half_open.contains(5));
    assert_eq!(SizeRange::from(1..=6), SizeRange::new(1, 6));
    assert_eq!(SizeRange::from(..4), SizeRange::new(0, 3));
    assert_eq!(SizeRange::from(..=4), SizeRange::new(0, 4));
    assert_eq!(SizeRange::new(1, 6).to_string(), "1..=6");
    assert!(
        std::panic::catch_unwind(|| SizeRange::from(3..3)).is_err(),
        "an empty range"
    );
}

#[test]
fn weighted_options_and_booleans_follow_their_probabilities() {
    let options = samples(
        &prop::option::weighted(0.9, Just(1u8)),
        "weighted_option",
        SAMPLES,
    );
    let somes = options.iter().filter(|value| value.is_some()).count();
    assert!(
        (8_500..9_500).contains(&somes),
        "{somes} of {SAMPLES} were Some"
    );
    let flags = samples(&prop::bool::weighted(0.1), "weighted_bool", SAMPLES);
    let trues = flags.iter().filter(|flag| **flag).count();
    assert!(
        (500..1_500).contains(&trues),
        "{trues} of {SAMPLES} were true"
    );
    let fair = samples(&prop::bool::ANY, "fair_bool", SAMPLES);
    let trues = fair.iter().filter(|flag| **flag).count();
    assert!(
        (4_500..5_500).contains(&trues),
        "{trues} of {SAMPLES} were true"
    );
    assert!(std::panic::catch_unwind(|| prop::bool::weighted(1.5)).is_err());
}

#[test]
fn an_index_resolves_against_the_collection_it_meets() {
    let index = replay(&any::<prop::sample::Index>(), "8000000000000000");
    assert_eq!(index.index(10), 5, "one half of the way along");
    assert_eq!(*index.get(&['a', 'b', 'c', 'd']), 'c');
    assert_eq!(replay(&any::<prop::sample::Index>(), "").index(3), 0);
}

#[test]
fn a_regex_strategy_names_its_pattern_and_any_strings_are_unicode_text() {
    assert_eq!(prop::string::regex("[a-z]+").pattern(), "[a-z]+");
    assert_eq!(any::<String>().pattern(), "\\PC*");
    let signed = samples(&any::<i8>(), "signed", SAMPLES);
    assert!(signed.contains(&i8::MIN) && signed.contains(&i8::MAX));
    assert_eq!(
        replay(&any::<i64>(), ""),
        0,
        "a signed integer shrinks to zero"
    );
    assert_eq!(replay(&any::<i64>(), "0000000000000001"), -1);
    assert_eq!(replay(&any::<i64>(), "0000000000000002"), 1);
    assert_eq!(replay(&any::<char>(), ""), '\0');
}

// ── Stateful testing ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Op {
    Push(u8),
    Pop,
    Clear,
}

#[derive(Debug)]
struct StackModel;

impl ReferenceStateMachine for StackModel {
    type State = Vec<u8>;
    type Transition = Op;

    fn init_state(&self) -> BoxedStrategy<Vec<u8>> {
        Just(Vec::new()).boxed()
    }

    fn transitions(&self, _state: &Vec<u8>) -> BoxedStrategy<Op> {
        prop_oneof![
            any::<u8>().prop_map(Op::Push),
            Just(Op::Pop),
            Just(Op::Clear),
        ]
        .boxed()
    }

    fn preconditions(&self, state: &Vec<u8>, op: &Op) -> bool {
        !matches!(op, Op::Pop) || !state.is_empty()
    }

    fn apply(&self, mut state: Vec<u8>, op: &Op) -> Vec<u8> {
        match op {
            Op::Push(value) => state.push(*value),
            Op::Pop => {
                state.pop();
            }
            Op::Clear => state.clear(),
        }
        state
    }
}

/// A stack with a planted defect: pushing a value of 100 or more while it
/// already holds two values drops the push.
struct BuggyStack(Vec<u8>);

impl SystemUnderTest<StackModel> for BuggyStack {
    fn init(state: &Vec<u8>) -> Self {
        Self(state.clone())
    }

    fn apply(&mut self, op: &Op, _after: &Vec<u8>) -> Result<(), TestCaseError> {
        match op {
            Op::Push(value) if *value >= 100 && self.0.len() >= 2 => {}
            Op::Push(value) => self.0.push(*value),
            Op::Pop => {
                prop_assert!(self.0.pop().is_some(), "popped an empty stack");
            }
            Op::Clear => self.0.clear(),
        }
        Ok(())
    }

    fn check_invariants(&self, state: &Vec<u8>) -> Result<(), TestCaseError> {
        prop_assert_eq!(&self.0, state);
        Ok(())
    }
}

/// The same stack without the defect.
struct GoodStack(Vec<u8>);

impl SystemUnderTest<StackModel> for GoodStack {
    fn init(state: &Vec<u8>) -> Self {
        Self(state.clone())
    }

    fn apply(&mut self, op: &Op, _after: &Vec<u8>) -> Result<(), TestCaseError> {
        match op {
            Op::Push(value) => self.0.push(*value),
            Op::Pop => {
                prop_assert!(self.0.pop().is_some(), "popped an empty stack");
            }
            Op::Clear => self.0.clear(),
        }
        Ok(())
    }

    fn check_invariants(&self, state: &Vec<u8>) -> Result<(), TestCaseError> {
        prop_assert_eq!(&self.0, state);
        Ok(())
    }
}

#[test]
fn a_state_machine_failure_shrinks_to_the_shortest_failing_run() {
    let strategy = state_machine::sequences(StackModel, 0..40);
    let case = failure(&strategy, "stack", |run| run.execute::<BuggyStack>());
    let shrunk = replay(&strategy, &case.choices_hex);
    assert_eq!(
        format!("{:?}", shrunk.transitions),
        "[Push(0), Push(0), Push(100)]",
        "{case:?}"
    );
    assert!(
        case.message.starts_with("at step 2 (Push(100))"),
        "{}",
        case.message
    );
}

#[test]
fn a_correct_system_passes_its_state_machine() {
    let strategy = state_machine::sequences(StackModel, 0..40);
    let seen = Cell::new(0u32);
    Runner::with_seed(Config::default(), "good_stack", 9)
        .run(&strategy, |run| {
            seen.set(seen.get() + 1);
            run.execute::<GoodStack>()
        })
        .expect("the correct stack matches its model");
    assert_eq!(seen.get(), 256);
    // Preconditions hold in every generated run: no pop from an empty model.
    for run in samples(&strategy, "stack_preconditions", 1_000) {
        let mut depth = 0usize;
        for op in &run.transitions {
            match op {
                Op::Push(_) => depth += 1,
                Op::Pop => {
                    assert!(depth > 0, "{run:?}");
                    depth -= 1;
                }
                Op::Clear => depth = 0,
            }
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native grammar, denotation, work and physical ownership of compiled globs.

use purrdf_glob::{
    Admission, CompileError, CompiledGlob, IriComponent, MatchError, Mode, StorageError,
    SyntaxReason,
};
use std::cell::Cell;
use std::rc::Rc;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn grammar_and_path_boundaries() {
    for (pattern, input, mode, expected) in [
        ("", "", Mode::Text, true),
        ("", "a", Mode::Text, false),
        ("a?", "a猫", Mode::Text, true),
        ("?", "e\u{301}", Mode::Text, false),
        ("é", "e\u{301}", Mode::Text, false),
        ("a", "A", Mode::Text, false),
        ("a*", "a/猫", Mode::Text, true),
        ("a*", "a/猫", Mode::Path, false),
        ("[α-ω][!0-9][^a]", "β猫z", Mode::Text, true),
        ("[α-ω]", "A", Mode::Text, false),
        ("[!a]", "a", Mode::Text, false),
        ("[-a][a-][\\]]", "-a]", Mode::Text, true),
        ("[--0]", ".", Mode::Text, false),
        (r"\*\?\[\{\}\\", "*?[{}\\", Mode::Text, true),
        ("[{]", "{", Mode::Text, true),
        ("[/-0]", "/", Mode::Text, true),
        ("[/-0]", "/", Mode::Path, false),
        ("[!a]", "/", Mode::Path, false),
        ("?", "/", Mode::Path, false),
        ("a/**/b", "a/b", Mode::Path, true),
        ("a/**/b", "a//b", Mode::Path, true),
        ("a/**/b", "a/猫/d/b", Mode::Path, true),
        ("a/**", "a/", Mode::Path, true),
        ("a/**", "a", Mode::Path, false),
        ("**/", "", Mode::Path, true),
        ("**/", "a/", Mode::Path, true),
        ("**/", "a", Mode::Path, false),
        ("**", "a//猫/", Mode::Path, true),
        ("a/", "a", Mode::Path, false),
        ("a/b", "a//b", Mode::Path, false),
        ("a/*/b", "a//b", Mode::Path, true),
        ("**/a*b/c", "axxx/y/ab/c", Mode::Path, true),
        (r"**\/a*b/c", "axxx/y/ab/c", Mode::Path, true),
        ("**/a*/**/b?", "z/ax/y/b猫", Mode::Path, true),
        ("**/a*/**/b?", "z/ax/y/b", Mode::Path, false),
        ("a***b", "a/b", Mode::Text, true),
    ] {
        let glob = CompiledGlob::compile(pattern, mode).unwrap();
        assert_eq!(
            glob.match_literal(input, u64::MAX).unwrap().matched,
            expected,
            "{mode:?} {pattern:?} {input:?}"
        );
    }
    for (pattern, reason, offset) in [
        ("é\\", SyntaxReason::TrailingEscape, 2),
        ("[]", SyntaxReason::EmptyClass, 0),
        ("[!]", SyntaxReason::EmptyClass, 0),
        ("[a", SyntaxReason::UnclosedClass, 0),
        ("[a-", SyntaxReason::UnclosedClass, 3),
        ("[z-a]", SyntaxReason::ReversedRange, 1),
        ("[[:alpha:]]", SyntaxReason::NamedClass, 1),
        ("a{b,c}", SyntaxReason::BraceAlternative, 1),
    ] {
        assert!(
            matches!(CompiledGlob::compile(pattern, Mode::Text), Err(CompileError::Syntax { reason: actual, offset: at }) if actual == reason && at == offset),
            "{pattern}"
        );
    }
    for (pattern, input, expected) in [
        ("***", "abc", true),
        ("***", "a/b", false),
        ("a**b", "axxb", true),
        ("**a", "ba", true),
        ("a/***/b", "a/x/b", true),
        ("a/***/b", "a/x/y/b", false),
        ("a/**b", "a/xb", true),
        ("[a]**", "abc", true),
        ("[za-cb-d]", "b", true),
    ] {
        assert_eq!(
            CompiledGlob::compile(pattern, Mode::Path)
                .unwrap()
                .match_literal(input, 1000)
                .unwrap()
                .matched,
            expected,
            "{pattern}"
        );
    }
}

fn explicit_iri_surfaces_and_zero_execution_allocations() {
    let iri = purrdf_iri::parse("https://example.org/a/%2F?x=#").unwrap();
    for (component, pattern) in [
        (IriComponent::Lexical, "https://example.org/a/%2F?x=#"),
        (IriComponent::Scheme, "https"),
        (IriComponent::Authority, "example.org"),
        (IriComponent::Path, "/a/%2F"),
        (IriComponent::Query, "x="),
        (IriComponent::Fragment, ""),
    ] {
        let glob = CompiledGlob::compile(pattern, Mode::Path).unwrap();
        assert!(glob.match_iri(&iri, component, 1000).unwrap().matched);
    }
    let path = CompiledGlob::compile("/a/*", Mode::Path).unwrap();
    let exact_slash = CompiledGlob::compile("/a//", Mode::Path).unwrap();
    assert!(
        !exact_slash
            .match_iri(&iri, IriComponent::Path, 1000)
            .unwrap()
            .matched
    );
    let no_query = purrdf_iri::parse("relative/path").unwrap();
    let empty = CompiledGlob::compile("*", Mode::Text).unwrap();
    let absent = empty.match_iri(&no_query, IriComponent::Query, 1).unwrap();
    assert!(!absent.matched);
    assert_eq!(absent.work, 1);
    assert!(matches!(
        empty.match_iri(&no_query, IriComponent::Query, 0),
        Err(MatchError::WorkExceeded { consumed: 0, .. })
    ));
    let empty_query = purrdf_iri::parse("relative/path?").unwrap();
    assert!(
        empty
            .match_iri(&empty_query, IriComponent::Query, 1000)
            .unwrap()
            .matched
    );
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let mut matched = 0;
    for _ in 0..1000 {
        matched += usize::from(
            path.match_iri(&iri, IriComponent::Path, 1000)
                .unwrap()
                .matched,
        );
        matched += usize::from(path.match_literal("/a/猫", 1000).unwrap().matched);
        assert!(path.match_literal("/a/z", 0).is_err());
    }
    let measured = window.close();
    assert_eq!(matched, 2000);
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.requested_bytes, 0);
}

#[derive(Debug)]
struct Budget {
    state: Rc<Cell<(usize, usize)>>,
    limit: usize,
}
impl Admission for Budget {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        let (_, peak) = self.state.get();
        if live > self.limit {
            return Err(StorageError::AdmissionFailed);
        }
        self.state.set((live, peak.max(live)));
        Ok(())
    }
}

fn compiled_storage_is_admitted_retained_and_refunded() {
    let state = Rc::new(Cell::new((0, 0)));
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let compiled = CompiledGlob::compile_with_storage(
        "[a-z]?/**/猫*",
        Mode::Path,
        Budget {
            state: state.clone(),
            limit: usize::MAX,
        },
    )
    .unwrap();
    let bytes = state.get().0;
    let measured = window.close();
    assert!(bytes > 0);
    assert_eq!(measured.allocations, 2);
    assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), bytes);
    assert_eq!(usize::try_from(measured.requested_bytes).unwrap(), bytes);
    assert!(compiled.match_literal("ab/z/猫", 1000).unwrap().matched);
    assert_eq!(state.get().0, bytes);
    drop(compiled);
    assert_eq!(state.get().0, 0);
    for limit in [bytes, bytes - 1, 0] {
        state.set((0, 0));
        let result = CompiledGlob::compile_with_storage(
            "[a-z]?/**/猫*",
            Mode::Path,
            Budget {
                state: state.clone(),
                limit,
            },
        );
        if limit == bytes {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(CompileError::Storage(StorageError::AdmissionFailed))
            ));
        }
        drop(result);
        assert_eq!(state.get().0, 0);
    }
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let invalid = CompiledGlob::compile("[z-a]", Mode::Text);
    let measured = window.close();
    assert!(matches!(invalid, Err(CompileError::Syntax { .. })));
    assert_eq!(
        measured.allocations, 0,
        "syntax validation precedes physical allocation"
    );
    #[derive(Debug)]
    struct Refuse(StorageError);
    impl Admission for Refuse {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            if live == 0 { Ok(()) } else { Err(self.0) }
        }
    }
    for cause in [
        StorageError::SizeOverflow,
        StorageError::AllocationFailed,
        StorageError::AdmissionFailed,
    ] {
        assert!(
            matches!(CompiledGlob::compile_with_storage("a", Mode::Text, Refuse(cause)), Err(CompileError::Storage(actual)) if actual == cause)
        );
    }
}

fn deterministic_work_refuses_at_the_exact_boundary() {
    let glob = CompiledGlob::compile("**/a*[b-d]x/c", Mode::Path).unwrap();
    for (input, expected) in [
        ("q/abx/c", true),
        ("q/abx/no", false),
        ("axxx/y/abx/c", true),
    ] {
        let report = glob.match_literal(input, u64::MAX).unwrap();
        assert_eq!(report.matched, expected);
        assert_eq!(glob.match_literal(input, report.work).unwrap(), report);
        assert_eq!(glob.match_literal(input, u64::MAX).unwrap(), report);
        assert!(
            matches!(glob.match_literal(input, report.work - 1), Err(MatchError::WorkExceeded { consumed, limit, .. }) if consumed == report.work - 1 && limit == consumed)
        );
    }
    let glob = CompiledGlob::compile("*aaaaaaaaab", Mode::Text).unwrap();
    let mut previous = 0;
    for count in [32, 64, 128, 256] {
        let input = "a".repeat(count);
        let report = glob.match_literal(&input, u64::MAX).unwrap();
        assert!(!report.matched);
        assert!(report.work > previous);
        previous = report.work;
        assert!(matches!(
            glob.match_literal(&input, 100),
            Err(MatchError::WorkExceeded {
                consumed: 100,
                limit: 100,
                ..
            })
        ));
    }
}

// Independent finite denotation: enumerate each wildcard's possible consumption,
// rather than reusing the production greedy checkpoints. Inputs stay tiny.
fn denotes(pattern: &str, input: &str, path: bool, component_start: bool) -> bool {
    if pattern.is_empty() {
        return input.is_empty();
    }
    if path
        && component_start
        && let Some(rest) = pattern.strip_prefix("**/")
    {
        return denotes(rest, input, path, true)
            || input
                .char_indices()
                .filter(|(_, ch)| *ch == '/')
                .any(|(at, _)| denotes(rest, &input[at + 1..], path, true));
    }
    if path && component_start && pattern == "**" {
        return true;
    }
    let ch = pattern.chars().next().unwrap();
    let rest = &pattern[ch.len_utf8()..];
    if ch == '*' {
        let rest = rest.trim_start_matches('*');
        if denotes(rest, input, path, false) {
            return true;
        }
        for (at, ch) in input.char_indices() {
            if path && ch == '/' {
                break;
            }
            if denotes(rest, &input[at + ch.len_utf8()..], path, false) {
                return true;
            }
        }
        false
    } else if let Some(first) = input.chars().next() {
        (ch == first || ch == '?' && (!path || first != '/'))
            && denotes(rest, &input[first.len_utf8()..], path, ch == '/')
    } else {
        false
    }
}

fn greedy_checkpoints_agree_with_finite_denotation() {
    let mut inputs = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..5 {
        layer = layer
            .iter()
            .flat_map(|prefix| ['a', 'b', '/'].map(|ch| format!("{prefix}{ch}")))
            .collect();
        inputs.extend(layer.iter().cloned());
    }
    for mode in [Mode::Text, Mode::Path] {
        let mut patterns = vec![String::new()];
        let mut layer = vec![String::new()];
        for _ in 0..3 {
            layer = layer
                .iter()
                .flat_map(|prefix| {
                    ["a", "b", "*", "?", "/", "**/", "**"].map(|atom| format!("{prefix}{atom}"))
                })
                .collect();
            patterns.extend(layer.iter().cloned());
        }
        for pattern in patterns {
            let Ok(glob) = CompiledGlob::compile(&pattern, mode) else {
                continue;
            };
            for input in &inputs {
                assert_eq!(
                    glob.match_literal(input, 100_000).unwrap().matched,
                    denotes(&pattern, input, mode == Mode::Path, true),
                    "{mode:?} {pattern:?} {input:?}"
                );
            }
        }
    }
}

fn ordered_sets_agree_with_individual_patterns_and_iri_surfaces() {
    use purrdf_glob::CompiledGlobSet;
    let patterns = ["", "**/a*b/c", "**", "[α-ω]*", "**/a*b/c", "?"];
    for mode in [Mode::Text, Mode::Path] {
        let set = CompiledGlobSet::compile(&patterns, mode).unwrap();
        let singles: Vec<_> = patterns
            .iter()
            .map(|pattern| CompiledGlob::compile(pattern, mode).unwrap())
            .collect();
        for input in ["", "猫", "β猫", "axxx/y/ab/c", "a/b", "/a/%2F"] {
            let reports: Vec<_> = singles
                .iter()
                .map(|glob| glob.match_literal(input, u64::MAX).unwrap())
                .collect();
            let expected: Vec<_> = reports
                .iter()
                .enumerate()
                .filter_map(|(index, report)| report.matched.then_some(index))
                .collect();
            let mut workspace = [usize::MAX; 6];
            let report = set.match_literal(input, u64::MAX, &mut workspace).unwrap();
            assert_eq!(report.indices, expected, "{mode:?} {input:?}");
            assert_eq!(
                report.work,
                reports.iter().map(|report| report.work).sum::<u64>()
            );
        }
        for text in [
            "https://example.org/a/%2F?x=#",
            "relative/path",
            "relative/path?",
        ] {
            let iri = purrdf_iri::parse(text).unwrap();
            for component in [
                IriComponent::Lexical,
                IriComponent::Scheme,
                IriComponent::Authority,
                IriComponent::Path,
                IriComponent::Query,
                IriComponent::Fragment,
            ] {
                let reports: Vec<_> = singles
                    .iter()
                    .map(|glob| glob.match_iri(&iri, component, u64::MAX).unwrap())
                    .collect();
                let expected: Vec<_> = reports
                    .iter()
                    .enumerate()
                    .filter_map(|(index, report)| report.matched.then_some(index))
                    .collect();
                let mut workspace = [0; 6];
                let report = set
                    .match_iri(&iri, component, u64::MAX, &mut workspace)
                    .unwrap();
                assert_eq!(report.indices, expected, "{mode:?} {text} {component:?}");
                assert_eq!(
                    report.work,
                    reports.iter().map(|report| report.work).sum::<u64>()
                );
            }
        }
    }
    let empty = CompiledGlobSet::compile(&[] as &[&str], Mode::Text).unwrap();
    let report = empty.match_literal("anything", 0, &mut []).unwrap();
    assert_eq!(report.indices, []);
    assert_eq!(report.work, 0);
}

fn set_refusals_preserve_pattern_index_original_cause_and_exact_limits() {
    use purrdf_glob::{CompiledGlobSet, SetCompileError, SetMatchError};
    use purrdf_hash::error::ErrorValue;
    use purrdf_testkit::error_capture::Capture;
    let invalid = CompiledGlobSet::compile(&["*", "é[z-a]", "[]"], Mode::Text).unwrap_err();
    let native = CompileError::Syntax {
        offset: 3,
        reason: SyntaxReason::ReversedRange,
    };
    assert_eq!(
        invalid,
        SetCompileError::Pattern {
            index: 1,
            cause: native
        }
    );
    assert_eq!(
        std::error::Error::source(&invalid)
            .unwrap()
            .downcast_ref::<CompileError>(),
        Some(&native)
    );
    let inspected = Capture::read(&invalid);
    assert!(matches!(inspected.field("index"), ErrorValue::Unsigned(1)));
    assert_eq!(inspected.child("cause").code, "glob-syntax");
    assert!(CompiledGlobSet::compile(&["*", "é[a-z]", "[x]"], Mode::Text).is_ok());

    let set = CompiledGlobSet::compile(&["**", "a"], Mode::Path).unwrap();
    assert_eq!(
        set.match_literal("a", 0, &mut [0]).unwrap_err(),
        SetMatchError::Workspace {
            required: 2,
            provided: 1
        }
    );
    let mut workspace = [0; 2];
    let report = set.match_literal("a", 3, &mut workspace).unwrap();
    assert_eq!(report.indices, [0, 1]);
    assert_eq!(report.work, 3);
    let events = Vec::with_capacity(16);
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let error = set.match_literal("a", 2, &mut workspace).unwrap_err();
    let inspected = Capture::read_with_buffer(&error, events);
    let measured = window.close();
    assert_eq!(measured.allocations, 0);
    let cause = MatchError::WorkExceeded {
        consumed: 2,
        limit: 2,
        offset: 1,
    };
    assert_eq!(error, SetMatchError::Pattern { index: 1, cause });
    assert_eq!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<MatchError>(),
        Some(&cause)
    );
    assert!(matches!(
        inspected.child("cause").field("consumed"),
        ErrorValue::Unsigned(2)
    ));
    assert!(matches!(
        inspected.child("cause").field("limit"),
        ErrorValue::Unsigned(2)
    ));

    let absent = purrdf_iri::parse("a").unwrap();
    let set = CompiledGlobSet::compile(&["*", "?", ""], Mode::Text).unwrap();
    let mut workspace = [0; 3];
    let report = set
        .match_iri(&absent, IriComponent::Query, 3, &mut workspace)
        .unwrap();
    assert_eq!(report.indices, []);
    assert_eq!(report.work, 3);
    assert!(matches!(
        set.match_iri(&absent, IriComponent::Query, 2, &mut workspace),
        Err(SetMatchError::Pattern {
            index: 2,
            cause: MatchError::WorkExceeded {
                consumed: 2,
                limit: 2,
                offset: 0
            }
        })
    ));
    let present = purrdf_iri::parse("a?").unwrap();
    assert_eq!(
        set.match_iri(&present, IriComponent::Query, 100, &mut workspace)
            .unwrap()
            .indices,
        [0, 2]
    );
}

fn set_admission_refunds_and_measured_growth_keep_matching_allocation_free() {
    use purrdf_glob::{CompiledGlobSet, SetCompileError};
    let state = Rc::new(Cell::new((0, 0)));
    let patterns = ["**/a[bc]*", "**/a[bc]*", "[z-a]"];
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    assert!(CompiledGlobSet::compile(&patterns, Mode::Path).is_err());
    assert_eq!(
        window.close().allocations,
        0,
        "all validation precedes allocation"
    );
    let patterns = &patterns[..2];
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let set = CompiledGlobSet::compile_with_storage(
        patterns,
        Mode::Path,
        Budget {
            state: state.clone(),
            limit: usize::MAX,
        },
    )
    .unwrap();
    let measured = window.close();
    let bytes = state.get().0;
    assert_eq!(measured.allocations, 3);
    assert_eq!(usize::try_from(measured.retained_bytes).unwrap(), bytes);
    assert_eq!(usize::try_from(measured.requested_bytes).unwrap(), bytes);
    let mut workspace = [0; 2];
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    for _ in 0..1000 {
        assert_eq!(
            set.match_literal("z/abcd", u64::MAX, &mut workspace)
                .unwrap()
                .indices,
            [0, 1]
        );
        assert!(set.match_literal("z/abcd", 0, &mut workspace).is_err());
    }
    assert_eq!(window.close().allocations, 0);
    assert_eq!(
        state.get().0,
        bytes,
        "execution retains the compilation owner"
    );
    drop(set);
    assert_eq!(state.get().0, 0);
    for limit in [bytes, bytes - 1, 0] {
        let result = CompiledGlobSet::compile_with_storage(
            patterns,
            Mode::Path,
            Budget {
                state: state.clone(),
                limit,
            },
        );
        if limit == bytes {
            assert!(result.is_ok());
        } else {
            assert!(matches!(
                result,
                Err(SetCompileError::Storage(StorageError::AdmissionFailed))
            ));
        }
        drop(result);
        assert_eq!(state.get().0, 0);
    }
    let one = CompiledGlob::compile(patterns[0], Mode::Path)
        .unwrap()
        .match_literal("z/abcd", u64::MAX)
        .unwrap()
        .work;
    for count in [1, 8, 64] {
        let patterns = vec![patterns[0]; count];
        let mut workspace = vec![0; count];
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let set = CompiledGlobSet::compile(&patterns, Mode::Path).unwrap();
        let allocation = window.close();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let report = set
            .match_literal("z/abcd", u64::MAX, &mut workspace)
            .unwrap();
        let matching = window.close();
        assert_eq!(matching.allocations, 0);
        assert_eq!(report.indices.len(), count);
        assert!(report.indices.iter().copied().eq(0..count));
        assert_eq!(report.work, one * u64::try_from(count).unwrap());
        println!(
            "glob set growth: patterns={count} work={} compilation_allocations={} compilation_bytes={} matching_allocations={}",
            report.work, allocation.allocations, allocation.requested_bytes, matching.allocations
        );
    }
}

purrdf_testkit::harness_main!(
    grammar_and_path_boundaries,
    explicit_iri_surfaces_and_zero_execution_allocations,
    compiled_storage_is_admitted_retained_and_refunded,
    deterministic_work_refuses_at_the_exact_boundary,
    greedy_checkpoints_agree_with_finite_denotation,
    ordered_sets_agree_with_individual_patterns_and_iri_surfaces,
    set_refusals_preserve_pattern_index_original_cause_and_exact_limits,
    set_admission_refunds_and_measured_growth_keep_matching_allocation_free
);

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

purrdf_testkit::harness_main!(
    grammar_and_path_boundaries,
    explicit_iri_surfaces_and_zero_execution_allocations,
    compiled_storage_is_admitted_retained_and_refunded,
    deterministic_work_refuses_at_the_exact_boundary,
    greedy_checkpoints_agree_with_finite_denotation
);

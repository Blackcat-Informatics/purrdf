// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual runner regressions and same-artifact allocation evidence for ground bags.

use super::tests::{blank, case_with, lit};
use super::*;
use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_core::{
    CanonError, CanonicalizationControl, SearchConfiguration, SearchPolicy, TermBox,
};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn control() -> CanonicalizationControl<'static> {
    CanonicalizationControl::new(purrdf_core::RDFC_V2_CORPUS_POLICY, None)
}

fn triple(s: TermValue, p: TermValue, o: TermValue) -> TermValue {
    TermValue::Triple {
        s: TermBox::new(s),
        p: TermBox::new(p),
        o: TermBox::new(o),
    }
}

fn parsed(variables: &[String], rows: Vec<Vec<Option<TermValue>>>) -> ParsedSolutions {
    ParsedSolutions {
        variables: variables.to_vec(),
        rows,
    }
}

#[test]
fn ground_verdicts_match_the_original_numeric_carrier() {
    let variables = vec!["x".into(), "y".into()];
    let iri = || TermValue::iri("https://example.org/p");
    let numeric = |text, name| lit(text, &format!("http://www.w3.org/2001/XMLSchema#{name}"));
    let rows = vec![
        vec![None, Some(numeric("02", "integer"))],
        vec![
            Some(numeric("NaN", "double")),
            Some(numeric("INF", "float")),
        ],
        vec![
            Some(numeric("bad", "decimal")),
            Some(numeric("2.0", "decimal")),
        ],
        vec![
            Some(triple(iri(), iri(), numeric("1.05E3", "double"))),
            None,
        ],
    ];
    let mut equivalent = rows.clone();
    equivalent[0][1] = Some(numeric("+2", "integer"));
    equivalent[2][1] = Some(numeric("2", "decimal"));
    equivalent[3][0] = Some(triple(iri(), iri(), numeric("1050", "double")));
    let mut duplicated = equivalent.clone();
    duplicated.push(equivalent[0].clone());
    let mut illtyped = equivalent.clone();
    illtyped[2][0] = Some(numeric("different", "decimal"));
    let mut reversed = equivalent.clone();
    reversed.reverse();
    for ordered in [false, true] {
        for (wanted, equal) in [
            (&rows, true),
            (&equivalent, true),
            (&duplicated, false),
            (&illtyped, false),
            (&reversed, !ordered),
        ] {
            let old_left = encode_solution_set(
                &variables,
                &rows,
                ordered,
                Literals::ByValue,
                &mut control(),
            )
            .unwrap();
            let old_right = encode_solution_set(
                &variables,
                wanted,
                ordered,
                Literals::ByValue,
                &mut control(),
            )
            .unwrap();
            let expected = parsed(&variables, wanted.clone());
            assert_eq!(old_left.nquads() == old_right.nquads(), equal);
            assert_eq!(
                compare_solutions(&variables, &rows, &expected, ordered, &mut control()).is_ok(),
                old_left.nquads() == old_right.nquads(),
            );
        }
    }
    // A zero-width row is still a solution, and each occurrence counts.
    for wanted in [vec![], vec![vec![]], vec![vec![], vec![]]] {
        assert_eq!(
            compare_solutions(
                &[],
                &[vec![]],
                &parsed(&[], wanted.clone()),
                false,
                &mut control()
            )
            .is_ok(),
            wanted.len() == 1
        );
    }
}

#[test]
fn headers_align_by_name_but_result_vectors_keep_their_order() {
    let variables = vec!["x".into(), "y".into()];
    let rows = vec![vec![Some(TermValue::iri("https://example.org/a")), None]];
    let expected = ParsedSolutions {
        variables: vec!["y".into(), "x".into()],
        rows: vec![vec![None, rows[0][0].clone()]],
    };
    let result = SparqlResult::Solutions {
        variables: variables.clone(),
        rows: rows.clone(),
        aux: RdfDatasetBuilder::new().freeze().unwrap().into(),
    };
    let reversed = SparqlResult::Solutions {
        variables: expected.variables.clone(),
        rows: expected.rows.clone(),
        aux: RdfDatasetBuilder::new().freeze().unwrap().into(),
    };
    for ordered in [false, true] {
        assert!(compare_solutions(&variables, &rows, &expected, ordered, &mut control()).is_ok());
        assert!(matches!(
            compare_results(&result, &reversed, ordered, &mut control()),
            Err(CompareError::Mismatch(_))
        ));
    }
}

#[test]
fn language_identity_and_direction_remain_part_of_ground_keys() {
    let variables = vec!["x".into()];
    let language =
        |tag: &str, direction: Option<purrdf_core::RdfTextDirection>| TermValue::Literal {
            lexical_form: "text".into(),
            datatype: if direction.is_some() {
                purrdf_iri::vocab::rdf::DIR_LANG_STRING
            } else {
                purrdf_iri::vocab::rdf::LANG_STRING
            }
            .into(),
            language: Some(tag.into()),
            direction,
        };
    let rows = vec![vec![Some(language("EN-us", None))]];
    let folded = parsed(&variables, vec![vec![Some(language("en-US", None))]]);
    assert!(compare_solutions(&variables, &rows, &folded, false, &mut control()).is_ok());
    let directional = vec![vec![Some(language(
        "en-US",
        Some(purrdf_core::RdfTextDirection::Ltr),
    ))]];
    let reversed = parsed(
        &variables,
        vec![vec![Some(language(
            "en-US",
            Some(purrdf_core::RdfTextDirection::Rtl),
        ))]],
    );
    assert!(matches!(
        compare_solutions(&variables, &directional, &reversed, false, &mut control()),
        Err(CompareError::Mismatch(_))
    ));
}

#[test]
fn sorting_polls_cancellation_after_keys_are_complete() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    #[derive(Debug, Default)]
    struct SortingStop {
        armed: AtomicBool,
        polls: AtomicUsize,
    }
    impl purrdf_core::StopSignal for SortingStop {
        fn stopped(&self) -> bool {
            self.armed.load(Ordering::Relaxed) && self.polls.fetch_add(1, Ordering::Relaxed) >= 8
        }
    }
    let stop = SortingStop::default();
    let variables = vec!["x".into()];
    let rows: Vec<_> = (0..32)
        .rev()
        .map(|index| vec![Some(TermValue::simple_literal(index.to_string()))])
        .collect();
    let view = SolutionRows {
        variables: &variables,
        rows: &rows,
    };
    let mut normalized = 0;
    let mut normalize = |_: &str, _: &str, _: &mut Memory<'_, Resident>| {
        normalized += 1;
        if normalized == 2 * rows.len() {
            stop.armed.store(true, Ordering::Relaxed);
        }
        Ok(None)
    };
    let mut controlled = CanonicalizationControl::new(SearchPolicy::Unlimited, Some(&stop));
    let error = bag::compare_ground_with_memory(
        view,
        view,
        ComparisonLaw {
            headers: HeaderLaw::ByName,
            rows: RowLaw::Multiset,
            literals: LiteralLaw::ByValue,
        },
        &mut normalize,
        &mut Memory::new(&mut Resident),
        &mut controlled,
    )
    .unwrap_err();
    assert_eq!(normalized, 2 * rows.len());
    assert!(matches!(
        error,
        purrdf_conformance_kit::GradeError::Canonicalization(CanonError::Stopped)
    ));
}

#[test]
fn malformed_shapes_refuse_at_case_reduced_and_trace_boundaries() {
    for (variables, rows) in [
        (vec!["x".into(), "x".into()], vec![vec![None, None]]),
        (vec!["x".into()], vec![vec![]]),
        (vec!["x".into()], vec![vec![None, None]]),
    ] {
        let expected = parsed(&variables, rows.clone());
        for reduced in [false, true] {
            let mut case = case_with(ExpectedResult::None);
            case.lax_cardinality = reduced;
            assert!(matches!(
                compare_case_solutions(&case, &variables, &rows, &expected, false, &mut control()),
                Err(CompareError::Preparation(_))
            ));
        }
        assert!(matches!(
            canonical_solutions(&variables, &rows, false, &mut control()),
            Err(CompareError::Preparation(_))
        ));
    }
}

#[test]
fn reserved_refusal_preserves_the_original_borrowed_admission() {
    use purrdf_lex::allocation::Ceiling;
    let iri = format!("{}{}", purrdf_core::RESERVED_NAMESPACE, "z".repeat(4096));
    let value = TermValue::iri(&iri);
    for limit in [0, iri.len()] {
        let mut ceiling = Ceiling(limit);
        let mut memory = Memory::new(&mut ceiling);
        let refusal = purrdf_core::ir::canon::screen_comparison_terms_with_memory(
            [&value],
            None,
            &mut memory,
        )
        .unwrap_err();
        assert_eq!(refusal, CanonError::Storage(StorageError::AdmissionFailed));
        assert_eq!(memory.admitted_bytes(), 0);
    }
    let variables = vec!["x".into()];
    let least = format!("{}a", purrdf_core::RESERVED_NAMESPACE);
    let rows = vec![vec![Some(value)], vec![Some(TermValue::iri(&least))]];
    let view = SolutionRows {
        variables: &variables,
        rows: &rows,
    };
    let mut ceiling = Ceiling(usize::MAX);
    let mut memory = Memory::new(&mut ceiling);
    let error = bag::compare_ground_with_memory(
        view,
        view,
        ComparisonLaw {
            headers: HeaderLaw::ByName,
            rows: RowLaw::Multiset,
            literals: LiteralLaw::ByValue,
        },
        &mut comparison_lexical_with_memory,
        &mut memory,
        &mut control(),
    )
    .unwrap_err();
    let purrdf_conformance_kit::GradeError::Canonicalization(CanonError::ReservedVocabulary(
        refusal,
    )) = error
    else {
        panic!("expected reserved vocabulary refusal, received {error:?}");
    };
    assert_eq!(refusal.iri(), least);
    let retained = refusal.retained_bytes().unwrap();
    assert_eq!(memory.admitted_bytes(), retained);
    assert!(retained > least.len(), "the shared header is admitted too");
    assert!(
        retained < iri.len(),
        "the superseded witness scratch is released"
    );
    let clone = refusal.clone();
    assert_eq!(refusal.iri().as_ptr(), clone.iri().as_ptr());
    drop(refusal);
    assert_eq!(memory.admitted_bytes(), retained);
    assert_eq!(clone.iri(), least);
    drop(clone);
    memory.release_bytes(retained).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
}

#[test]
fn nested_reserved_witness_keeps_native_first_depth_first_candidate() {
    let reserved = purrdf_core::RESERVED_NAMESPACE;
    let value = triple(
        TermValue::iri(format!("{reserved}z")),
        TermValue::iri("https://example.org/p"),
        lit("value", &format!("{reserved}a")),
    );
    let variables = vec!["x".into()];
    let rows = vec![vec![Some(value)]];
    let original = canonical_solutions(&variables, &rows, false, &mut control()).unwrap_err();
    assert!(matches!(
        original,
        CompareError::Canonicalization(CanonError::ReservedVocabulary(_))
    ));
    let borrowed = compare_solutions(
        &variables,
        &rows,
        &parsed(&variables, rows.clone()),
        false,
        &mut control(),
    )
    .unwrap_err();
    assert_eq!(borrowed, original);
    assert_eq!(borrowed.to_string(), original.to_string());
    assert_eq!(format!("{borrowed:?}"), format!("{original:?}"));
}

#[test]
fn malformed_terms_refuse_before_identical_or_reduced_shortcuts() {
    let variables = vec!["x".into()];
    let values = [
        triple(
            TermValue::simple_literal("subject"),
            TermValue::iri("https://example.org/p"),
            TermValue::iri("https://example.org/o"),
        ),
        lit("[", purrdf_cdt::CDT_LIST),
        TermValue::Literal {
            lexical_form: "text".into(),
            datatype: purrdf_iri::vocab::rdf::LANG_STRING.into(),
            language: Some("bad_tag".into()),
            direction: None,
        },
    ];
    for value in values {
        let rows = vec![vec![Some(value)]];
        for reduced in [false, true] {
            let mut case = case_with(ExpectedResult::None);
            case.lax_cardinality = reduced;
            let error = compare_case_solutions(
                &case,
                &variables,
                &rows,
                &parsed(&variables, rows.clone()),
                false,
                &mut control(),
            )
            .unwrap_err();
            assert!(!matches!(error, CompareError::Mismatch(_)), "{error:?}");
        }
    }
}

#[test]
fn composite_and_bare_blanks_use_one_global_bijection_in_the_runner() {
    use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
    let row = |label: &str, other: &str| {
        let encoded = encode_blank_label(label, BlankScope::DEFAULT, LabelAlphabet::BlankNodeLabel);
        vec![
            Some(blank(other)),
            Some(lit(
                &format!(
                    "[_:{encoded}, <<( _:{encoded} <https://example.org/p> <https://example.org/o> )>>]"
                ),
                purrdf_cdt::CDT_LIST,
            )),
        ]
    };
    let variables = vec!["direct".into(), "list".into()];
    let rows = vec![row("a", "a"), row("a", "a")];
    let expected = parsed(
        &variables,
        vec![row("renamed", "renamed"), row("renamed", "renamed")],
    );
    assert!(compare_solutions(&variables, &rows, &expected, false, &mut control()).is_ok());
    let split = parsed(
        &variables,
        vec![row("renamed", "different"), row("renamed", "different")],
    );
    assert!(matches!(
        compare_solutions(&variables, &rows, &split, false, &mut control()),
        Err(CompareError::Mismatch(_))
    ));
    let a = vec![vec![Some(lit("[2]", purrdf_cdt::CDT_LIST))]];
    let b = parsed(
        &["x".into()],
        vec![vec![Some(lit("[02]", purrdf_cdt::CDT_LIST))]],
    );
    assert!(matches!(
        compare_solutions(&b.variables, &a, &b, false, &mut control()),
        Err(CompareError::Mismatch(_))
    ));
}

#[test]
fn ground_comparison_has_no_rdfc_search_and_preserves_physical_and_stop_refusals() {
    let variables = vec!["x".into()];
    let rows = vec![vec![Some(TermValue::iri("https://example.org/a"))]];
    let view = SolutionRows {
        variables: &variables,
        rows: &rows,
    };
    let mut zero = CanonicalizationControl::new(SearchPolicy::Finite(0), None);
    assert!(
        compare_solutions(
            &variables,
            &rows,
            &parsed(&variables, rows.clone()),
            false,
            &mut zero
        )
        .is_ok()
    );
    assert_eq!(zero.receipts().len(), 1);
    assert_eq!(
        zero.receipts()[0].configuration,
        SearchConfiguration::ComparisonPreparation
    );
    assert_eq!(zero.receipts()[0].work, purrdf_core::SearchWork::default());
    let mut ceiling = purrdf_lex::allocation::Ceiling(0);
    let error = bag::compare_ground_with_memory(
        view,
        view,
        ComparisonLaw {
            headers: HeaderLaw::ByName,
            rows: RowLaw::Multiset,
            literals: LiteralLaw::ByValue,
        },
        &mut comparison_lexical_with_memory,
        &mut Memory::new(&mut ceiling),
        &mut control(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        purrdf_conformance_kit::GradeError::Storage(_)
    ));
    #[derive(Debug)]
    struct Stopped;
    impl purrdf_core::StopSignal for Stopped {
        fn stopped(&self) -> bool {
            true
        }
    }
    let stop = Stopped;
    let mut stopped = CanonicalizationControl::new(SearchPolicy::Unlimited, Some(&stop));
    assert!(matches!(
        compare_solutions(
            &variables,
            &rows,
            &parsed(&variables, rows.clone()),
            false,
            &mut stopped
        ),
        Err(CompareError::Canonicalization(CanonError::Stopped))
    ));
}

#[test]
fn actual_ground_runner_allocations_and_work_scale_without_rdf_carriers() {
    let variables = vec!["x".into()];
    for count in [16, 64, 256] {
        let rows: Vec<_> = (0..count)
            .map(|index| {
                vec![Some(TermValue::iri(format!(
                    "https://example.org/item/{index:04}"
                )))]
            })
            .collect();
        let expected = parsed(&variables, rows.iter().rev().cloned().collect());
        let mut new_control = control();
        let window = CurrentThreadWindow::open();
        compare_solutions(&variables, &rows, &expected, false, &mut new_control).unwrap();
        let ground = window.close();
        let mut old_control = control();
        let window = CurrentThreadWindow::open();
        let left = encode_solution_set(
            &variables,
            &rows,
            false,
            Literals::ByValue,
            &mut old_control,
        )
        .unwrap();
        let right = encode_solution_set(
            &variables,
            &expected.rows,
            false,
            Literals::ByValue,
            &mut old_control,
        )
        .unwrap();
        assert_eq!(left.nquads(), right.nquads());
        drop((left, right));
        let carrier = window.close();
        let work = ground_comparison(
            SolutionRows {
                variables: &variables,
                rows: &rows,
            },
            SolutionRows {
                variables: &variables,
                rows: &expected.rows,
            },
            RowLaw::Multiset,
            HeaderLaw::ByName,
            &mut control(),
        )
        .unwrap()
        .work;
        assert_eq!(work.row_keys, 2 * count);
        assert_eq!(work.key_parts, 2 * count);
        assert!(work.row_comparisons < 32 * count as u64 * (count.ilog2() as u64 + 1));
        assert!(ground.allocations < carrier.allocations);
        assert!(ground.requested_bytes < carrier.requested_bytes);
        assert!(ground.peak_working_bytes < carrier.peak_working_bytes);
        eprintln!(
            "ground rows={count} allocation={ground:?} work={work:?}; original-carrier allocation={carrier:?}"
        );
    }
}

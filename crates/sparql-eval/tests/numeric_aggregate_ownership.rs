// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact aggregate ownership through the real persistent-query boundary.

mod support;
use purrdf_core::DatasetView;

use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_core::{SegmentedError, TermValue};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine, QueryGovernors, QueryOptions};
use purrdf_xsd::datatype::{XSD_DECIMAL, XSD_DOUBLE, XSD_DURATION, XSD_INTEGER};
use purrdf_xsd::exact::DivisionPolicy;
use support::segmented::{CEILING, fixture, open};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn pair_query(integer: &str) -> String {
    format!(
        "SELECT (SUM(?x) AS ?sum) (AVG(?x) AS ?avg) WHERE {{ VALUES ?x {{ {integer} {integer} }} }}"
    )
}

#[test]
fn exact_sum_and_avg_keep_output_admitted_through_clone_and_extraction() {
    let (image, _) = fixture();
    let zeros = "0".repeat(4096);
    let input = format!("1{zeros}");
    let expected = vec![vec![
        Some(TermValue::typed_literal(format!("2{zeros}"), XSD_INTEGER)),
        Some(TermValue::typed_literal(input.clone(), XSD_DECIMAL)),
    ]];
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(&pair_query(&input), None).unwrap();
    let source = open(&image, CEILING);
    let options = QueryOptions::EMPTY.with_division(DivisionPolicy::Exact);

    let window = CurrentThreadWindow::open();
    let complete = engine
        .query_prepared_fallible_view(&source, &prepared, &[], options)
        .expect("the exact integer/decimal fold is supported");
    let measured = window.close();
    assert_eq!(complete.result.solutions().unwrap().1, expected);
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= CEILING);

    // Keep only the extracted result so its sibling receipt cannot mask a
    // missing result owner. Shallow cloning must not duplicate either magnitude.
    let (result, evidence) = complete.into_parts();
    drop(evidence);
    let live = source.evidence().live_bytes();
    let window = CurrentThreadWindow::open();
    let clone = result.clone();
    assert_eq!(window.close().allocations, 0);
    assert_eq!(source.evidence().live_bytes(), live);
    drop(result);
    assert_eq!(clone.solutions().unwrap().1, expected);
    assert_eq!(source.evidence().live_bytes(), live);
    drop(clone);
    assert!(
        source.evidence().live_bytes() < live,
        "the last payload owner releases its original grant"
    );
}

#[test]
fn native_ieee_and_raw_duration_fold_laws_survive_owned_state() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let query = format!(
        "SELECT (SUM(?x) AS ?sum) WHERE {{ VALUES ?x {{ \"-3.0e0\"^^<{XSD_DOUBLE}> \"-9007199254740992.0e0\"^^<{XSD_DOUBLE}> \"-1.0e0\"^^<{XSD_DOUBLE}> \"-0.7e0\"^^<{XSD_DOUBLE}> }} }}"
    );
    let prepared = engine.prepare_query(&query, None).unwrap();
    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    let (_, rows) = complete.result.solutions().unwrap();
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = rows[0][0].as_ref().unwrap()
    else {
        panic!("numeric aggregate literal")
    };
    assert_eq!(datatype, XSD_DOUBLE);
    let purrdf_xsd::XsdValue::Double(value) =
        purrdf_xsd::parse(lexical_form, purrdf_xsd::XsdDatatype::Double).unwrap()
    else {
        panic!("double aggregate")
    };
    assert_eq!(value.to_bits(), (-9_007_199_254_740_996.0_f64).to_bits());

    // The second prefix is mixed-sign; only the final (12 months, 0 seconds)
    // pair is validated. Validating each intermediate would wrongly poison it.
    let query = format!(
        "SELECT (SUM(?x) AS ?sum) (AVG(?x) AS ?avg) WHERE {{ VALUES ?x {{ \"P1Y\"^^<{XSD_DURATION}> \"-P1D\"^^<{XSD_DURATION}> \"P1D\"^^<{XSD_DURATION}> }} }}"
    );
    let prepared = engine.prepare_query(&query, None).unwrap();
    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_fallible_view(&source, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(
        complete.result.solutions().unwrap().1,
        vec![vec![
            Some(TermValue::typed_literal("P1Y", XSD_DURATION)),
            Some(TermValue::typed_literal("P4M", XSD_DURATION)),
        ]]
    );
}

#[test]
fn exact_mean_refusal_stays_an_aggregate_code_in_an_admitted_receipt() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(
            "SELECT (SUM(?x) AS ?sum) (AVG(?x) AS ?avg) WHERE { VALUES ?x { 1 0 0 } }",
            None,
        )
        .unwrap();
    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_governed_fallible_view(
            &source,
            &prepared,
            &[],
            QueryOptions::EMPTY.with_division(DivisionPolicy::Exact),
            &QueryGovernors::METERED,
        )
        .unwrap();
    assert_eq!(
        complete.result.solutions().unwrap().1,
        vec![vec![Some(TermValue::typed_literal("1", XSD_INTEGER)), None,]]
    );
    assert_eq!(
        complete.evidence.governors.expression_errors(),
        &[(purrdf_xsd::ErrorCode::Foar0002, 1)]
    );
    assert!(
        source.read_error().is_none(),
        "non-terminating AVG is not a storage refusal"
    );
}

#[test]
fn capacity_refusal_never_becomes_a_poisoned_or_partial_aggregate() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_division(DivisionPolicy::Exact);
    let small = engine.prepare_query(&pair_query("1"), None).unwrap();
    let calibration = open(&image, CEILING);
    let small_result = engine
        .query_prepared_fallible_view(&calibration, &small, &[], options)
        .unwrap();
    let ceiling = calibration.evidence().peak_bytes();
    drop(small_result);
    drop(calibration);

    // Same two-row operator shape. Each final lexical payload alone is larger
    // than the healthy small execution's measured storage/workspace ceiling.
    let digits = usize::try_from(ceiling).unwrap().checked_add(1).unwrap();
    let input = format!("1{}", "0".repeat(digits));
    let large = engine.prepare_query(&pair_query(&input), None).unwrap();
    let source = open(&image, ceiling);
    let window = CurrentThreadWindow::open();
    let refused = engine.query_prepared_fallible_view(&source, &large, &[], options);
    let measured = window.close();
    assert!(
        matches!(
            refused,
            Err(FallibleSparqlError::Operational {
                error: SegmentedError::Residency { .. },
                ..
            })
        ),
        "workspace refusal must propagate before aggregate publication"
    );
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= ceiling);

    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_fallible_view(&source, &large, &[], options)
        .expect("the same magnitude succeeds with sufficient capacity");
    let (_, rows) = complete.result.solutions().unwrap();
    assert_eq!(
        rows[0][0],
        Some(TermValue::typed_literal(
            format!("2{}", "0".repeat(digits)),
            XSD_INTEGER
        ))
    );
    assert_eq!(
        rows[0][1],
        Some(TermValue::typed_literal(input, XSD_DECIMAL))
    );
}
fn statistical_env() -> purrdf_sparql_eval::extension_env::ExtensionEnv {
    let mut aggregates = purrdf_sparql_eval::AggregateRegistry::default();
    aggregates.register_statistical_aggregates("http://example.org/stat#");
    purrdf_sparql_eval::extension_env::ExtensionEnv::new(
        purrdf_sparql_algebra::ParserOptions::default(),
        purrdf_sparql_eval::PropertyFunctionRegistry::default(),
        aggregates,
    )
    .unwrap()
}

#[test]
fn all_ten_native_statistics_keep_original_owners_through_every_query_door() {
    use purrdf_core::SparqlRequest;
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    let env = statistical_env();
    let options = QueryOptions::EMPTY.with_env(&env);
    let query = "SELECT (AGG(<http://example.org/stat#MEDIAN>, ?x) AS ?median) (AGG(<http://example.org/stat#PERCENTILE>, ?x; P=0.5) AS ?percentile) (AGG(<http://example.org/stat#STDDEV>, ?x) AS ?stddev) (AGG(<http://example.org/stat#STDDEV_POP>, ?x) AS ?stddevpop) (AGG(<http://example.org/stat#VARIANCE>, ?x) AS ?variance) (AGG(<http://example.org/stat#VAR_POP>, ?x) AS ?varpop) (AGG(<http://example.org/stat#MODE>, ?x) AS ?mode) (AGG(<http://example.org/stat#FIRST>, ?x) AS ?first) (AGG(<http://example.org/stat#LAST>, ?x) AS ?last) (AGG(<http://example.org/stat#TOPK>, ?x; K=2) AS ?topk) WHERE { VALUES ?x { 3 1 2 } }";
    let prepared = engine
        .prepare_query_with_options(query, None, options)
        .unwrap();
    let expected = engine
        .query_prepared_view(&*resident, &prepared, &[], options)
        .unwrap();
    let expected_rows = expected.solutions().unwrap().1;
    assert_eq!(
        expected_rows[0][0],
        Some(TermValue::typed_literal("2", XSD_DECIMAL))
    );
    assert_eq!(expected_rows[0][1], expected_rows[0][0]);
    assert_eq!(
        expected_rows[0][6],
        Some(TermValue::typed_literal("1", XSD_INTEGER))
    );
    assert_eq!(
        expected_rows[0][7],
        Some(TermValue::typed_literal("3", XSD_INTEGER))
    );
    assert_eq!(
        expected_rows[0][8],
        Some(TermValue::typed_literal("2", XSD_INTEGER))
    );
    assert_eq!(expected_rows[0][9], Some(TermValue::simple_literal("3 2")));
    for door in 0..4 {
        let source = open(&image, CEILING);
        let request = SparqlRequest {
            query,
            base_iri: None,
            substitutions: &[],
        };
        let window = CurrentThreadWindow::open();
        let result = match door {
            0 => engine
                .query_fallible_view(&source, request, options)
                .map(|complete| complete.result)
                .map_err(|failure| format!("{failure:?}")),
            1 => engine
                .query_prepared_fallible_view(&source, &prepared, &[], options)
                .map(|complete| complete.result)
                .map_err(|failure| format!("{failure:?}")),
            2 => engine
                .query_governed_fallible_view(&source, request, options, &QueryGovernors::METERED)
                .map(|complete| complete.result)
                .map_err(|failure| format!("{failure:?}")),
            _ => engine
                .query_prepared_governed_fallible_view(
                    &source,
                    &prepared,
                    &[],
                    options,
                    &QueryGovernors::METERED,
                )
                .map(|complete| complete.result)
                .map_err(|failure| format!("{failure:?}")),
        }
        .expect("every native statistical door is supported");
        let measured = window.close();
        assert_eq!(result.solutions().unwrap().1, expected_rows);
        assert!(
            u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes()
        );
        assert!(source.evidence().peak_bytes() <= CEILING);
        let live = source.evidence().live_bytes();
        let window = CurrentThreadWindow::open();
        let clone = result.clone();
        assert_eq!(window.close().allocations, 0);
        drop(result);
        assert_eq!(source.evidence().live_bytes(), live);
        assert_eq!(clone.solutions().unwrap().1, expected_rows);
        drop(clone);
        assert!(source.evidence().live_bytes() < live);
    }
}

#[test]
fn statistical_duration_interpolation_and_exact_variance_codes_use_native_kernels() {
    let (image, resident) = fixture();
    let engine = NativeSparqlEngine::new();
    let env = statistical_env();
    let options = QueryOptions::EMPTY
        .with_env(&env)
        .with_division(DivisionPolicy::Exact);
    let query = format!(
        "SELECT (AGG(<http://example.org/stat#MEDIAN>, ?x) AS ?median) (AGG(<http://example.org/stat#PERCENTILE>, ?x; P=0.25) AS ?percentile) WHERE {{ VALUES ?x {{ \"P1D\"^^<{XSD_DURATION}> \"P3D\"^^<{XSD_DURATION}> }} }}"
    );
    let prepared = engine
        .prepare_query_with_options(&query, None, options)
        .unwrap();
    let expected = engine
        .query_prepared_view(&*resident, &prepared, &[], options)
        .unwrap();
    let source = open(&image, CEILING);
    let window = CurrentThreadWindow::open();
    let complete = engine
        .query_prepared_governed_fallible_view(
            &source,
            &prepared,
            &[],
            options,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let measured = window.close();
    assert_eq!(
        complete.result.solutions().unwrap().1,
        expected.solutions().unwrap().1
    );
    assert_eq!(
        complete.result.solutions().unwrap().1[0][0],
        Some(TermValue::typed_literal("P2D", XSD_DURATION))
    );
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    drop(complete);
    let query = "SELECT (AGG(<http://example.org/stat#VAR_POP>, ?x) AS ?variance) WHERE { VALUES ?x { 1 0 0 } }";
    let prepared = engine
        .prepare_query_with_options(query, None, options)
        .unwrap();
    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_governed_fallible_view(
            &source,
            &prepared,
            &[],
            options,
            &QueryGovernors::METERED,
        )
        .unwrap();
    assert_eq!(complete.result.solutions().unwrap().1, vec![vec![None]]);
    assert_eq!(
        complete.evidence.governors.expression_errors(),
        &[(purrdf_xsd::ErrorCode::Foar0002, 1)]
    );
    assert!(source.read_error().is_none());
}

#[test]
fn statistical_large_state_refusal_never_publishes_poison_or_partial_success() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let env = statistical_env();
    let options = QueryOptions::EMPTY.with_env(&env);
    let query_for = |integer: &str| {
        format!(
            "SELECT (AGG(<http://example.org/stat#MEDIAN>, ?x) AS ?median) (AGG(<http://example.org/stat#TOPK>, ?x; K=2) AS ?topk) WHERE {{ VALUES ?x {{ {integer} {integer} }} }}"
        )
    };
    let small = engine
        .prepare_query_with_options(&query_for("1"), None, options)
        .unwrap();
    let calibration = open(&image, CEILING);
    let small_result = engine
        .query_prepared_fallible_view(&calibration, &small, &[], options)
        .unwrap();
    let ceiling = calibration.evidence().peak_bytes();
    drop(small_result);
    drop(calibration);
    let large = query_for(&format!(
        "1{}",
        "0".repeat(usize::try_from(ceiling).unwrap() + 1)
    ));
    let prepared = engine
        .prepare_query_with_options(&large, None, options)
        .unwrap();
    let source = open(&image, ceiling);
    let window = CurrentThreadWindow::open();
    let result = engine.query_prepared_fallible_view(&source, &prepared, &[], options);
    let measured = window.close();
    assert!(matches!(
        result,
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Residency { .. },
            ..
        })
    ));
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= ceiling);
}

#[test]
fn statistical_big_integer_states_and_outputs_are_physically_admitted() {
    let (image, _) = fixture();
    let engine = NativeSparqlEngine::new();
    let env = statistical_env();
    let options = QueryOptions::EMPTY
        .with_env(&env)
        .with_division(DivisionPolicy::Exact);
    let zeros = "0".repeat(4096);
    let first = format!("1{zeros}");
    let last = format!("3{zeros}");
    let expected = vec![vec![
        Some(TermValue::typed_literal(format!("2{zeros}"), XSD_DECIMAL)),
        Some(TermValue::typed_literal(
            format!("1{}", "0".repeat(8192)),
            XSD_DECIMAL,
        )),
        Some(TermValue::typed_literal(first.clone(), XSD_INTEGER)),
        Some(TermValue::simple_literal(format!("{last} {first}"))),
    ]];
    let query = format!(
        "SELECT (AGG(<http://example.org/stat#MEDIAN>, ?x) AS ?median) (AGG(<http://example.org/stat#VAR_POP>, ?x) AS ?variance) (AGG(<http://example.org/stat#MODE>, ?x) AS ?mode) (AGG(<http://example.org/stat#TOPK>, ?x; K=2) AS ?topk) WHERE {{ VALUES ?x {{ {first} {last} }} }}"
    );
    let prepared = engine
        .prepare_query_with_options(&query, None, options)
        .unwrap();
    let source = open(&image, CEILING);
    let window = CurrentThreadWindow::open();
    let complete = engine
        .query_prepared_governed_fallible_view(
            &source,
            &prepared,
            &[],
            options,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let measured = window.close();
    assert_eq!(complete.result.solutions().unwrap().1, expected);
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= CEILING);
    let (result, evidence) = complete.into_parts();
    drop(evidence);
    let live = source.evidence().live_bytes();
    let window = CurrentThreadWindow::open();
    let clone = result.clone();
    assert_eq!(window.close().allocations, 0);
    drop(result);
    assert_eq!(source.evidence().live_bytes(), live);
    assert_eq!(clone.solutions().unwrap().1, expected);
    drop(clone);
    assert!(source.evidence().live_bytes() < live);
}

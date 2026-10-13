// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Custom aggregate tuple/metadata/output owners through the complete query door.

mod support;

use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_core::{SegmentedError, TermValue};
use purrdf_sparql_eval::agg_fn::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, CustomAggregate, ScalarvalKind,
    ScalarvalSpec, WorkspaceAccumulator,
};
use purrdf_sparql_eval::user_fn::{Arity, Volatility};
use purrdf_sparql_eval::{
    EvalError, ExtensionEnv, FallibleSparqlError, NativeSparqlEngine, QueryOptions,
    WorkspaceCapability, WorkspaceTerm,
};
use purrdf_xsd::{
    datatype::XSD_STRING,
    exact::{Cost, DivisionPolicy},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use support::segmented::{CEILING, fixture, open};

#[global_allocator]
static GLOBAL: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

const IRI: &str = "http://example.org/aggregate/last-pair";

struct LastPair {
    prefix: WorkspaceTerm,
    last: Option<WorkspaceTerm>,
    workspace: WorkspaceCapability,
    steps: Arc<AtomicUsize>,
}

fn lexical(value: &TermValue) -> &str {
    match value {
        TermValue::Literal { lexical_form, .. } => lexical_form,
        _ => panic!("the fixture admits literal tuples"),
    }
}

impl AggregateAccumulator for LastPair {
    fn native_workspace(&self) -> Option<&WorkspaceCapability> {
        Some(&self.workspace)
    }
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let value = self.workspace.literal(
            format_args!(
                "{}{}|{}",
                lexical(&self.prefix),
                lexical(&args[0]),
                lexical(&args[1])
            ),
            XSD_STRING,
        )?;
        self.last = Some(value);
        self.steps.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        let other = other
            .into_any()
            .downcast::<Self>()
            .map_err(|_| EvalError::internal("last-pair fixture received a different partial"))?;
        if other.last.is_some() {
            self.last = other.last;
        }
        Ok(())
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }
    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(self.last.map(|value| (*value).clone()))
    }
    fn finish_admitted(
        self: Box<Self>,
        _: &WorkspaceCapability,
    ) -> Result<Option<WorkspaceTerm>, EvalError> {
        Ok(self.last)
    }
}

struct LastPairAggregate {
    steps: Arc<AtomicUsize>,
}

impl CustomAggregate for LastPairAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(2)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::OrderDependent
    }
    fn state_bound(&self) -> u64 {
        // The fixture accepts arbitrary caller lexical lengths. Physical growth
        // is admitted natively; the legacy governor declaration has no smaller
        // universal state bound for this input-dependent string producer.
        u64::MAX
    }
    fn scalarvals(&self) -> &[ScalarvalSpec] {
        const SPEC: [ScalarvalSpec; 1] = [ScalarvalSpec::new("PREFIX", ScalarvalKind::String)];
        &SPEC
    }
    fn init(&self, scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        let workspace = WorkspaceCapability::resident();
        Box::new(LastPair {
            prefix: workspace.clone_term(&scalarvals[0].1).unwrap(),
            last: None,
            workspace,
            steps: self.steps.clone(),
        })
    }
    fn init_admitted(
        &self,
        scalarvals: &[(String, TermValue)],
        _: DivisionPolicy,
        workspace: &WorkspaceCapability,
    ) -> Result<WorkspaceAccumulator, EvalError> {
        WorkspaceAccumulator::new(
            LastPair {
                prefix: workspace.clone_term(&scalarvals[0].1)?,
                last: None,
                workspace: workspace.clone(),
                steps: self.steps.clone(),
            },
            workspace,
        )
    }
    fn exact_numeric_cost_admitted(
        &self,
        _: &[Vec<TermValue>],
        _: &[(String, TermValue)],
        _: DivisionPolicy,
        _: &WorkspaceCapability,
    ) -> Result<Cost, EvalError> {
        Ok(Cost::ZERO)
    }
}

fn environment() -> (ExtensionEnv, Arc<AtomicUsize>) {
    let steps = Arc::new(AtomicUsize::new(0));
    let mut registry = AggregateRegistry::default();
    registry.register(
        IRI,
        Arc::new(LastPairAggregate {
            steps: steps.clone(),
        }),
    );
    (ExtensionEnv::over_aggregates(registry).unwrap(), steps)
}

fn query(prefix: &str) -> String {
    format!(
        "SELECT (AGG(<{IRI}>, DISTINCT ?a, ?b; PREFIX='{prefix}') AS ?answer) WHERE {{ VALUES (?a ?b) {{ ('a' '1') ('a' '1') ('a' '2') ('b' UNDEF) ('a' '1') }} }}"
    )
}

#[test]
fn full_tuple_distinct_scalarvals_and_output_original_grants_survive_extraction() {
    let (image, _) = fixture();
    let (environment, steps) = environment();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_env(&environment);
    let prepared = engine
        .prepare_query_with_options(&query("pre:"), None, options)
        .unwrap();
    let source = open(&image, CEILING);
    let window = CurrentThreadWindow::open();
    let complete = engine
        .query_prepared_fallible_view(&source, &prepared, &[], options)
        .unwrap();
    let measured = window.close();
    assert_eq!(
        steps.load(Ordering::Relaxed),
        2,
        "DISTINCT compares the full tuple, skips the unbound row, and keeps first occurrence order"
    );
    assert_eq!(
        complete.result.solutions().unwrap().1,
        [vec![Some(TermValue::simple_literal("pre:a|2"))]]
    );
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= CEILING);

    let (result, evidence) = complete.into_parts();
    drop(evidence);
    let live = source.evidence().live_bytes();
    let window = CurrentThreadWindow::open();
    let cloned = result.clone();
    assert_eq!(window.close().allocations, 0);
    assert_eq!(source.evidence().live_bytes(), live);
    drop(result);
    assert_eq!(
        cloned.solutions().unwrap().1,
        [vec![Some(TermValue::simple_literal("pre:a|2"))]]
    );
    assert_eq!(source.evidence().live_bytes(), live);
    drop(cloned);
    assert!(source.evidence().live_bytes() < live);
}

#[test]
fn scalarval_capacity_refusal_keeps_original_source_error_and_does_not_start_the_fold() {
    let (image, _) = fixture();
    let (environment, steps) = environment();
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_env(&environment);
    let small = engine
        .prepare_query_with_options(&query("p:"), None, options)
        .unwrap();
    let calibration = open(&image, CEILING);
    let healthy = engine
        .query_prepared_fallible_view(&calibration, &small, &[], options)
        .unwrap();
    let ceiling = calibration.evidence().peak_bytes();
    drop(healthy);
    drop(calibration);
    steps.store(0, Ordering::Relaxed);

    let prefix = "p".repeat(usize::try_from(ceiling).unwrap().checked_add(1).unwrap());
    let large = engine
        .prepare_query_with_options(&query(&prefix), None, options)
        .unwrap();
    let source = open(&image, ceiling);
    let window = CurrentThreadWindow::open();
    let refused = engine.query_prepared_fallible_view(&source, &large, &[], options);
    let measured = window.close();
    assert!(matches!(
        refused,
        Err(FallibleSparqlError::Operational {
            error: SegmentedError::Residency { .. },
            ..
        })
    ));
    assert_eq!(steps.load(Ordering::Relaxed), 0);
    assert!(u64::try_from(measured.peak_working_bytes).unwrap() <= source.evidence().peak_bytes());
    assert!(source.evidence().peak_bytes() <= ceiling);

    let source = open(&image, CEILING);
    let complete = engine
        .query_prepared_fallible_view(&source, &large, &[], options)
        .unwrap();
    assert_eq!(
        complete.result.solutions().unwrap().1,
        [vec![Some(TermValue::simple_literal(format!(
            "{prefix}a|2"
        )))]]
    );
}

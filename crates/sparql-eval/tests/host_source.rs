// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The host clock and entropy at the public query boundary, read through the
//! installed source and from nowhere else.
//!
//! A source is installed once per process, so the whole sequence is one case: what
//! a request gets before any source exists, the installation, and what it gets
//! after. On `wasm32-unknown-unknown` the first part is the refusal this crate
//! raises in place of a host import; natively it is the operating system's answer.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, StopCause, TermValue};
use purrdf_sparql_eval::eval::EvalCtx;
use purrdf_sparql_eval::host_source::{self, HostInput, HostSource, HostSourceInstalled};
use purrdf_sparql_eval::{
    EvalError, EvaluationFailure, InProcessServiceResolver, NativeSparqlEngine, QueryCoordinates,
    QueryGovernors, QueryOptions, StopSignal, WallDeadline, WorkspaceCapability,
};

/// A host whose clock the test moves and whose reads it counts.
#[derive(Debug)]
struct Scripted {
    millis: AtomicU32,
    clock_reads: AtomicU32,
    entropy_reads: AtomicU32,
}

const SEED: u64 = 0x5eed_0123_4567_89ab;

impl HostSource for Scripted {
    fn unix_millis(&self) -> f64 {
        self.clock_reads.fetch_add(1, Ordering::Relaxed);
        f64::from(self.millis.load(Ordering::Relaxed))
    }

    fn entropy_seed(&self) -> u64 {
        self.entropy_reads.fetch_add(1, Ordering::Relaxed);
        SEED
    }
}

static SOURCE: Scripted = Scripted {
    millis: AtomicU32::new(1_500),
    clock_reads: AtomicU32::new(0),
    entropy_reads: AtomicU32::new(0),
};

/// A second source, which must never be read.
static REFUSED: Scripted = Scripted {
    millis: AtomicU32::new(9_000_000),
    clock_reads: AtomicU32::new(0),
    entropy_reads: AtomicU32::new(0),
};

const QUERY: &str = "SELECT (NOW() AS ?time) (RAND() AS ?random) WHERE {}";
const UPDATE: &str = "INSERT { <http://example.org/s> <http://example.org/time> ?t ; \
     <http://example.org/random> ?r } WHERE { BIND(NOW() AS ?t) BIND(RAND() AS ?r) }";

const ENDPOINT: &str = "http://example.org/endpoint";
/// The request's own time and random draw beside those of a clause answered in
/// process.
const SERVICE_QUERY: &str = "SELECT ?outer ?inner ?mine ?theirs WHERE { \
     BIND(NOW() AS ?outer) BIND(RAND() AS ?mine) \
     SERVICE <http://example.org/endpoint> { BIND(NOW() AS ?inner) BIND(RAND() AS ?theirs) } }";

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn service_resolver(dataset: &Arc<RdfDataset>) -> InProcessServiceResolver {
    InProcessServiceResolver::new().with_endpoint(ENDPOINT, Arc::clone(dataset))
}

/// The rows of [`SERVICE_QUERY`] under `options`: exactly one, with every cell bound.
fn service_rows(
    engine: &NativeSparqlEngine,
    dataset: &Arc<RdfDataset>,
    options: &QueryOptions<'_, '_>,
) -> Vec<Vec<Option<TermValue>>> {
    let result = engine
        .query_with_options_view(dataset, request(SERVICE_QUERY), options)
        .unwrap();
    let rows = result.solutions().unwrap().1.to_vec();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].iter().all(Option::is_some), "{rows:?}");
    rows
}

/// What a build with no operating system and no source answers: each entry point
/// refuses with the typed cause naming the missing input, and the same request with
/// that input supplied runs.
fn without_a_source_a_host_read_is_refused_and_supplied_inputs_run(
    engine: &NativeSparqlEngine,
    now: &purrdf_xsd::temporal::DateTime,
) {
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let supplied = QueryOptions::new().with_now(now).with_entropy_seed(7);
    for (options, missing) in [
        (QueryOptions::EMPTY, Some("clock")),
        (QueryOptions::new().with_entropy_seed(7), Some("clock")),
        (QueryOptions::new().with_now(now), Some("entropy")),
        (supplied, None),
    ] {
        let query = engine.query_with_options_view(&dataset, request(QUERY), &options);
        let governed = engine.query_governed_view(
            &dataset,
            request(QUERY),
            &options,
            &QueryGovernors::METERED,
        );
        let mut target = Arc::clone(&dataset);
        let update = engine.update_with_options(&mut target, request(UPDATE), &options);
        let Some(input) = missing else {
            assert_eq!(query.unwrap().solutions().unwrap().1.len(), 1);
            assert!(governed.unwrap().into_complete().is_ok());
            update.unwrap();
            assert_ne!(target.quad_count(), dataset.quad_count());
            continue;
        };
        for diagnostic in [
            query.unwrap_err(),
            governed.map(|_| ()).unwrap_err(),
            update.unwrap_err(),
        ] {
            assert_eq!(diagnostic.code, EvalError::HOST_SOURCE_MISSING_CODE);
            assert!(
                diagnostic
                    .message
                    .starts_with(&format!("no host {input} source is installed: supply the ")),
                "{}",
                diagnostic.message
            );
            assert!(
                diagnostic.message.ends_with(
                    "or install a source with `purrdf_sparql_eval::host_source::install`"
                ),
                "{}",
                diagnostic.message
            );
        }
        assert_eq!(target.quad_count(), dataset.quad_count());
    }

    // The typed lane keeps the cause as a value, not as text.
    let capability = WorkspaceCapability::resident();
    let mut prepared = engine
        .prepare_execution_admitted(QUERY, None, &[], &QueryOptions::EMPTY, &capability)
        .unwrap();
    let mut typed = |options: &QueryOptions<'_, '_>| {
        engine
            .execute_governed_admitted(
                &mut prepared,
                &dataset,
                options,
                &QueryGovernors::METERED,
                &capability,
                |_| (),
            )
            .map(|_| ())
    };
    let Err(EvaluationFailure::Evaluation(EvalError::HostSourceMissing(cause))) =
        typed(&QueryOptions::EMPTY)
    else {
        panic!("the typed lane refuses a default read with its cause")
    };
    assert_eq!(cause.input(), HostInput::Clock);
    assert!(typed(&supplied).is_ok());

    // An in-process SERVICE is part of the request that reaches it: with the inputs
    // supplied it answers with no source, and with neither it is the same refusal.
    let resolver = service_resolver(&dataset);
    let rows = service_rows(engine, &dataset, &supplied.with_remote(Some(&resolver)));
    assert_eq!(rows[0][0], rows[0][1], "one request has one NOW()");
    let refused = engine
        .query_with_options_view(
            &dataset,
            request(SERVICE_QUERY),
            &QueryOptions::new().with_remote(Some(&resolver)),
        )
        .unwrap_err();
    assert_eq!(refused.code, EvalError::HOST_SOURCE_MISSING_CODE);
    assert!(
        refused.message.starts_with("no host clock source"),
        "{}",
        refused.message
    );

    assert_eq!(
        WallDeadline::try_after(Duration::from_millis(5))
            .err()
            .map(|missing| missing.input()),
        Some(HostInput::Clock)
    );
    assert!(matches!(
        EvalCtx::try_new(&*dataset).err(),
        Some(EvalError::HostSourceMissing(_))
    ));
}

/// The one ordered case: before a source, the installation, and after it.
fn the_installed_source_is_the_only_host_read_and_its_absence_is_a_typed_refusal() {
    let engine = NativeSparqlEngine::new();
    let dataset = RdfDatasetBuilder::new().freeze().unwrap();
    let now = purrdf_xsd::temporal::parse_datetime("2024-02-29T12:34:56.125+05:30").unwrap();

    assert!(host_source::installed().is_none());
    if cfg!(target_arch = "wasm32") {
        without_a_source_a_host_read_is_refused_and_supplied_inputs_run(&engine, &now);
    } else {
        // A native build reads the operating system, as it did before a source existed.
        let sampled = engine
            .query_governed_view(
                &dataset,
                request(QUERY),
                &QueryOptions::EMPTY,
                &QueryGovernors::METERED,
            )
            .unwrap();
        assert!(matches!(
            sampled.coordinates(),
            QueryCoordinates::Started(_)
        ));
        assert!(
            WallDeadline::try_after(Duration::from_secs(3600))
                .unwrap()
                .poll()
                .is_none()
        );
    }
    assert_eq!(SOURCE.clock_reads.load(Ordering::Relaxed), 0);

    host_source::install(&SOURCE).unwrap();

    // A request that supplies nothing reads the installed source, once per input.
    let sampled = engine
        .query_governed_view(
            &dataset,
            request(QUERY),
            &QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let QueryCoordinates::Started(started) = sampled.coordinates() else {
        panic!("the default read started a context")
    };
    assert!(started.now().is_none() && started.entropy_seed().is_none());
    assert_eq!(
        started.effective_now().canonical_lexical(),
        "1970-01-01T00:00:01Z"
    );
    assert_eq!(started.effective_entropy_seed(), SEED);
    assert_eq!(SOURCE.clock_reads.load(Ordering::Relaxed), 1);
    assert_eq!(SOURCE.entropy_reads.load(Ordering::Relaxed), 1);

    // A supplied input is used as given and its host read is not made.
    let supplied = engine
        .query_governed_view(
            &dataset,
            request(QUERY),
            &QueryOptions::new().with_now(&now).with_entropy_seed(7),
            &QueryGovernors::METERED,
        )
        .unwrap();
    let QueryCoordinates::Started(started) = supplied.coordinates() else {
        panic!("the supplied read started a context")
    };
    assert!(started.effective_now().same_representation(&now));
    assert_eq!(started.effective_entropy_seed(), 7);
    assert_eq!(SOURCE.clock_reads.load(Ordering::Relaxed), 1);
    assert_eq!(SOURCE.entropy_reads.load(Ordering::Relaxed), 1);

    // An in-process SERVICE evaluates under the request's own time and a seed derived
    // from its random state: it makes no host read of its own, with inputs supplied
    // or sampled.
    let resolver = service_resolver(&dataset);
    let with_service = QueryOptions::new()
        .with_now(&now)
        .with_entropy_seed(7)
        .with_remote(Some(&resolver));
    let first = service_rows(&engine, &dataset, &with_service);
    assert_eq!(first[0][0], first[0][1], "one request has one NOW()");
    assert_ne!(first[0][2], first[0][3], "the forwarded stream is its own");
    assert_eq!(first, service_rows(&engine, &dataset, &with_service));
    assert_eq!(SOURCE.clock_reads.load(Ordering::Relaxed), 1);
    assert_eq!(SOURCE.entropy_reads.load(Ordering::Relaxed), 1);
    let sampled = service_rows(
        &engine,
        &dataset,
        &QueryOptions::new().with_remote(Some(&resolver)),
    );
    assert_eq!(sampled[0][0], sampled[0][1]);
    assert_eq!(SOURCE.clock_reads.load(Ordering::Relaxed), 2);
    assert_eq!(SOURCE.entropy_reads.load(Ordering::Relaxed), 2);

    // A wall deadline runs on the installed clock.
    let deadline = WallDeadline::try_after(Duration::from_millis(5)).unwrap();
    SOURCE.millis.store(1_504, Ordering::Relaxed);
    assert_eq!(deadline.poll(), None);
    SOURCE.millis.store(1_505, Ordering::Relaxed);
    assert_eq!(deadline.poll(), Some(StopCause::Deadline));

    // A source is installed once: a second is refused and the first is kept.
    assert_eq!(host_source::install(&REFUSED), Err(HostSourceInstalled));
    let again = engine
        .query_governed_view(
            &dataset,
            request(QUERY),
            &QueryOptions::EMPTY,
            &QueryGovernors::METERED,
        )
        .unwrap();
    let QueryCoordinates::Started(started) = again.coordinates() else {
        panic!("the default read started a context")
    };
    assert_eq!(started.effective_entropy_seed(), SEED);
    assert_eq!(REFUSED.clock_reads.load(Ordering::Relaxed), 0);
    assert_eq!(REFUSED.entropy_reads.load(Ordering::Relaxed), 0);
}

purrdf_testkit::harness_main!(
    the_installed_source_is_the_only_host_read_and_its_absence_is_a_typed_refusal,
);

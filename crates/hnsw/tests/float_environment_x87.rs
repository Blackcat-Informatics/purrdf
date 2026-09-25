// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! On the x87, a thread that loaded a directed rounding control is refused by name where
//! it computes, not where the index was built.
//!
//! Where binary64 runs on the x87 (`i586`, or `i686` without SSE2) there is no
//! flush-to-zero mode, so `float_environment`'s FTZ treatment has nothing to stand for.
//! The x87's own departure a caller can make is its control word's rounding-control
//! field: round down, up or toward zero, under which every binary64 operation rounds by
//! another rule than the arithmetic defines. (Its precision-control field is the
//! arithmetic's own to set -- every operation runs under a guard that sets 53 bits and
//! restores the caller's word -- so a caller's precision is served, not refused; that
//! is unit-tested in `purrdf_core::distance` and `purrdf_xsd::ieee`.)
//!
//! Every index and relation here is built on the test thread, in the default
//! environment, and shared with worker threads through plain references. One worker per
//! directed mode loads that mode into its own control word, in a guard that restores the
//! saved word on every exit, and makes the calls; another makes the same calls in the
//! default environment. The treatment: every call from a directed worker is refused as
//! `FloatEnvironmentError::RoundingMode`, naming the x87 control word and the value it
//! held. The valid neighbour: every call from the clean worker answers bit for bit what
//! the same calls answer on the test thread.

#![cfg(all(target_arch = "x86", not(target_feature = "sse2")))]

#[path = "support/purremb.rs"]
mod purremb;

use std::sync::Arc;

use purrdf_core::DistanceMetric;
use purrdf_core::distance::{Arithmetic, FloatEnvironmentError, FloatEnvironmentEvidence};
use purrdf_hnsw::{HnswError, HnswIndex, Params, Ranked};
use purrdf_sparql_eval::{
    EmbeddingKnnRelation, EmbeddingSpace, EvalError, KnnGuard, PfArgs, PfRow, PropertyFunction,
};
use purrdf_xsd::ieee::x87;

/// The directed rounding-control values: down, up, toward zero.
const DIRECTED: [(&str, u16); 3] = [
    ("down", 0b01 << 10),
    ("up", 0b10 << 10),
    ("toward zero", 0b11 << 10),
];

/// A directed rounding control loaded on this thread for as long as the guard lives.
struct Directed(u16);

impl Directed {
    /// Load `rounding` into this thread's control word; returns the guard and the word
    /// now in force.
    fn load(rounding: u16) -> (Self, u16) {
        let saved = x87::control_word();
        let word = (saved & !x87::ROUNDING_CONTROL) | rounding;
        // SAFETY: only the rounding-control field differs from this thread's word, so
        // every exception stays masked, and `Drop` restores the saved word on every exit.
        unsafe { x87::load_control_word(word) };
        (Self(saved), word)
    }
}

impl Drop for Directed {
    fn drop(&mut self) {
        // SAFETY: the word this thread had before `load`.
        unsafe { x87::load_control_word(self.0) };
    }
}

fn params() -> Params {
    Params::new(4, 8, 16, 8).expect("valid")
}

/// Whether `refusal` names the x87 control word holding `word`.
fn is_rounding_refusal(refusal: &FloatEnvironmentError, word: u16) -> bool {
    matches!(
        refusal,
        FloatEnvironmentError::RoundingMode {
            evidence: FloatEnvironmentEvidence::Register {
                name: "x87 control word",
                bits,
            },
        } if *bits == u64::from(word)
    )
}

fn is_refused(error: &HnswError, word: u16) -> bool {
    matches!(error, HnswError::FloatEnvironment(refusal) if is_rounding_refusal(refusal, word))
}

fn is_refused_eval(error: &EvalError, word: u16) -> bool {
    matches!(error, EvalError::FloatEnvironment(refusal) if is_rounding_refusal(refusal, word))
}

/// Every row of one kNN invocation seeded at `seed`: the ranked read of depth 3 when
/// `neighbour` is free, the membership lookup of `neighbour` when it is bound.
fn knn_invoke<A: Arithmetic>(
    relation: &EmbeddingKnnRelation<A>,
    seed: &purrdf_core::TermValue,
    neighbour: Option<&purrdf_core::TermValue>,
) -> Result<Vec<PfRow>, EvalError> {
    let count =
        purrdf_core::TermValue::typed_literal("3", "http://www.w3.org/2001/XMLSchema#integer");
    let subject = [neighbour];
    let object = [Some(seed), neighbour.is_none().then_some(&count), None];
    let args = PfArgs::new(&subject, &object);
    let mut cursor = relation.open(&args, None)?;
    let mut rows = Vec::new();
    while let Some(row) = cursor.next()? {
        rows.push(row);
    }
    Ok(rows)
}

/// A batch answer as its rows and distance bits, so equality is bit-identity.
fn bits(batch: &[Vec<Ranked>]) -> Vec<Vec<(usize, u64)>> {
    batch
        .iter()
        .map(|ranked| {
            ranked
                .iter()
                .map(|scored| (scored.row, scored.distance.to_bits()))
                .collect()
        })
        .collect()
}

/// What one thread's calls answered.
struct WorkerAnswers {
    exact_batch: Result<Vec<Vec<Ranked>>, HnswError>,
    fast_batch: Result<Vec<Vec<Ranked>>, HnswError>,
    exact_rows: Result<Vec<Ranked>, HnswError>,
    exact_pair: Result<f64, HnswError>,
    fast_pair: Result<f64, HnswError>,
    exact_scan: Result<Vec<PfRow>, EvalError>,
    fast_scan: Result<Vec<PfRow>, EvalError>,
    exact_lookup: Result<Vec<PfRow>, EvalError>,
    fast_lookup: Result<Vec<PfRow>, EvalError>,
}

#[test]
fn a_directed_x87_worker_is_refused_by_name_and_a_clean_worker_answers_the_same_bits() {
    let fixture = purremb::Fixture::new(40, 20, params());
    let matrix = fixture.matrix.clone();
    let exact = HnswIndex::build(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
        .expect("builds in the default environment");
    let fast =
        HnswIndex::build_reassociated(matrix.clone(), &DistanceMetric::SquaredEuclidean, params())
            .expect("builds in the default environment");
    let space = Arc::new(
        EmbeddingSpace::from_artifact(
            &fixture.without_index,
            fixture.target_set,
            fixture.vector_space,
            fixture.bindings(),
            KnnGuard::new(64, 8).expect("valid"),
        )
        .expect("the space opens in the default environment"),
    );
    let exact_knn = EmbeddingKnnRelation::new(Arc::clone(&space));
    let fast_knn = EmbeddingKnnRelation::new_reassociated(Arc::clone(&space))
        .expect("the reassociated relation constructs in the default environment");
    let seed = fixture.terms[0].clone();
    let held = fixture.terms[7].clone();
    let queries: Vec<usize> = (0..matrix.rows()).collect();

    let answers = || WorkerAnswers {
        exact_batch: exact.search_batch(&queries, 4),
        fast_batch: fast.search_batch(&queries, 4),
        exact_rows: exact.search_rows(3, 4),
        exact_pair: exact.row_distance(0, 7),
        fast_pair: fast.row_distance(0, 7),
        exact_scan: knn_invoke(&exact_knn, &seed, None),
        fast_scan: knn_invoke(&fast_knn, &seed, None),
        exact_lookup: knn_invoke(&exact_knn, &seed, Some(&held)),
        fast_lookup: knn_invoke(&fast_knn, &seed, Some(&held)),
    };

    // The single-thread oracle, on the thread that built everything.
    let single_exact: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| exact.search_rows(row, 4).expect("searches"))
        .collect();
    let single_fast: Vec<Vec<Ranked>> = queries
        .iter()
        .map(|&row| fast.search_rows(row, 4).expect("searches"))
        .collect();
    let here = answers();

    for (mode, rounding) in DIRECTED {
        let saved = x87::control_word();
        let (directed, clean) = std::thread::scope(|scope| {
            let directed = scope.spawn(|| {
                let (guard, word) = Directed::load(rounding);
                let answered = answers();
                drop(guard);
                (answered, word)
            });
            let clean = scope.spawn(answers);
            (
                directed.join().expect("the directed worker returns"),
                clean.join().expect("the clean worker returns"),
            )
        });
        assert_eq!(
            x87::control_word(),
            saved,
            "{mode}: the test thread is untouched"
        );
        let (directed, word) = directed;
        assert_ne!(word & x87::ROUNDING_CONTROL, 0, "{mode}: a directed mode");

        // The refused case: every compute entry, from the directed worker, named.
        assert!(
            is_refused(&directed.exact_batch.expect_err("refused"), word),
            "{mode}: exact search_batch"
        );
        assert!(
            is_refused(&directed.fast_batch.expect_err("refused"), word),
            "{mode}: reassociated search_batch"
        );
        assert!(
            is_refused(&directed.exact_rows.expect_err("refused"), word),
            "{mode}: exact search_rows"
        );
        assert!(
            is_refused(&directed.exact_pair.expect_err("refused"), word),
            "{mode}: exact row_distance"
        );
        assert!(
            is_refused(&directed.fast_pair.expect_err("refused"), word),
            "{mode}: reassociated row_distance"
        );
        assert!(
            is_refused_eval(&directed.exact_scan.expect_err("refused"), word),
            "{mode}: exact kNN scan"
        );
        assert!(
            is_refused_eval(&directed.fast_scan.expect_err("refused"), word),
            "{mode}: reassociated kNN scan"
        );
        assert!(
            is_refused_eval(&directed.exact_lookup.expect_err("refused"), word),
            "{mode}: exact kNN membership lookup"
        );
        assert!(
            is_refused_eval(&directed.fast_lookup.expect_err("refused"), word),
            "{mode}: reassociated kNN membership lookup"
        );

        // The valid neighbour: the clean worker answers the test thread's bits.
        assert_eq!(
            bits(&clean.exact_batch.expect("the clean worker searches")),
            bits(&single_exact),
            "{mode}: exact batch"
        );
        assert_eq!(
            bits(&clean.fast_batch.expect("the clean worker searches")),
            bits(&single_fast),
            "{mode}: reassociated batch"
        );
        assert_eq!(
            bits(&[clean.exact_rows.expect("searches")]),
            bits(&[single_exact[3].clone()]),
            "{mode}: exact search_rows"
        );
        assert_eq!(
            clean.exact_pair.expect("answers").to_bits(),
            here.exact_pair.as_ref().expect("answers").to_bits()
        );
        assert_eq!(
            clean.fast_pair.expect("answers").to_bits(),
            here.fast_pair.as_ref().expect("answers").to_bits()
        );
        let exact_scan = here.exact_scan.as_ref().expect("answers");
        assert_eq!(exact_scan.len(), 3);
        assert_eq!(&clean.exact_scan.expect("answers"), exact_scan);
        assert_eq!(
            &clean.fast_scan.expect("answers"),
            here.fast_scan.as_ref().expect("answers")
        );
        let lookup = here.exact_lookup.as_ref().expect("answers");
        assert_eq!(lookup.len(), 1, "the held term is answered by its row");
        assert_eq!(&clean.exact_lookup.expect("answers"), lookup);
        assert_eq!(
            &clean.fast_lookup.expect("answers"),
            here.fast_lookup.as_ref().expect("answers")
        );
    }
    assert_eq!(
        bits(&here.exact_batch.expect("searches")),
        bits(&single_exact)
    );
}

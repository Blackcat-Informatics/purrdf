// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Request-selected clock values retain their native coefficient admission.

use crate::workspace::LexicalFrame;
use crate::{EvalError, WorkspaceCapability};
use purrdf_lex::allocation::{Memory, SharedOwned};
use purrdf_xsd::{XsdValue, temporal::DateTime};

/// Supplied coordinates retained by one admitted prepared-query outcome.
/// This is not a query, source or snapshot identity. Missing supplied inputs
/// make no claim about sampled defaults. Clones share the original admitted
/// timestamp coefficient; they allocate no replacement storage or owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryCoordinates {
    /// No evaluation context was successfully created. Preparation or reads may
    /// already have occurred; this is not a claim that the request did no work.
    NotStarted,
    /// The context was created, whether evaluation completed or exhausted a budget.
    Started(SuppliedQueryCoordinates),
}

/// Supplied inputs captured from an admitted context before its first draw.
/// Private fields preserve the original timestamp owner across every clone.
#[derive(Debug, Clone)]
pub struct SuppliedQueryCoordinates {
    now: Option<DateTime>,
    entropy_seed: Option<u64>,
}
impl SuppliedQueryCoordinates {
    /// Effective supplied time, including exact seconds and declared timezone.
    #[must_use]
    pub const fn now(&self) -> Option<&DateTime> {
        self.now.as_ref()
    }
    /// Effective supplied seed before the execution's first random draw.
    #[must_use]
    pub const fn entropy_seed(&self) -> Option<u64> {
        self.entropy_seed
    }
}
impl PartialEq for SuppliedQueryCoordinates {
    fn eq(&self, other: &Self) -> bool {
        self.entropy_seed == other.entropy_seed
            && match (&self.now, &other.now) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_representation(right),
                _ => false,
            }
    }
}
impl Eq for SuppliedQueryCoordinates {}

impl QueryCoordinates {
    pub(super) fn started(
        now: &XsdValue,
        seed: u64,
        supplied_now: bool,
        supplied_seed: bool,
    ) -> Result<Self, EvalError> {
        let now = if supplied_now {
            let XsdValue::DateTime(now) = now else {
                return Err(EvalError::Invariant(
                    crate::EvaluatorInvariant::QueryClockNotDateTime,
                ));
            };
            Some(now.clone())
        } else {
            None
        };
        Ok(Self::Started(SuppliedQueryCoordinates {
            now,
            entropy_seed: supplied_seed.then_some(seed),
        }))
    }
}

pub(crate) fn now(
    supplied: Option<&DateTime>,
    workspace: &WorkspaceCapability,
) -> Result<XsdValue, EvalError> {
    let Some(supplied) = supplied else {
        return Ok(XsdValue::DateTime(crate::clock::wall_clock_now()));
    };
    if supplied.owned_heap_bytes() == 0 {
        return Ok(XsdValue::DateTime(supplied.clone()));
    }
    let mut frame = LexicalFrame::new(workspace);
    let prepared = supplied.detached_with_memory(&mut Memory::new(&mut frame));
    let mut value =
        prepared.map_err(|error| frame.storage_error(error, "query clock coordinate"))?;
    let admitted = frame.admitted_bytes();
    let replacement = LexicalFrame::new(workspace);
    let owner = match SharedOwned::retain_current(
        &mut frame,
        replacement,
        admitted,
        std::alloc::Layout::new::<()>(),
    ) {
        Ok((owner, _)) => owner,
        Err(error) => {
            drop(value);
            return Err(frame.storage_error(error, "query clock coordinate"));
        }
    };
    if let Err(error) = value.publish_detached(&owner) {
        drop(value);
        drop(owner);
        return Err(workspace.storage_error(error, "query clock coordinate"));
    }
    Ok(XsdValue::DateTime(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InternedGoverned, InternedOutcome, NativeSparqlEngine, QueryGovernors, QueryOptions,
    };
    use purrdf_core::{RdfDatasetBuilder, TermValue};
    use purrdf_testkit::harness::without_host_clock_or_entropy;
    use std::sync::atomic::Ordering;

    fn time(value: &str) -> DateTime {
        let XsdValue::DateTime(value) =
            purrdf_xsd::parse(value, purrdf_xsd::XsdDatatype::DateTime).unwrap()
        else {
            panic!("dateTime input");
        };
        value
    }

    /// The receipt follows the actual prepared context, before RAND/UUID advance it.
    #[test]
    fn admitted_receipt_tracks_fresh_inputs_and_original_seed() {
        let engine = NativeSparqlEngine::new();
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let capability = WorkspaceCapability::resident();
        let query = "SELECT (NOW() AS ?time) (RAND() AS ?random) (UUID() AS ?uuid) WHERE {}";
        let mut prepared = engine
            .prepare_execution_admitted(query, None, &[], QueryOptions::EMPTY, &capability)
            .unwrap();
        let first = time("2024-02-29T12:34:56.125+05:30");
        let shifted = time("2024-02-29T07:04:56.125Z");
        let mut execute = |now: &DateTime, seed| {
            let receipt = without_host_clock_or_entropy(|| {
                engine.execute_governed_admitted(
                    &mut prepared,
                    &dataset,
                    QueryOptions::new().with_now(now).with_entropy_seed(seed),
                    &QueryGovernors::METERED,
                    &capability,
                    |outcome| {
                        let InternedOutcome::Solutions(rows) = outcome else {
                            panic!("SELECT rows")
                        };
                        (0..3)
                            .map(|column| {
                                rows.try_cell_owned(&rows.rows()[0], column, &capability)
                                    .unwrap()
                                    .map(|cell| (*cell).clone())
                            })
                            .collect::<Vec<Option<TermValue>>>()
                    },
                )
            })
            .unwrap();
            let coordinates = receipt.coordinates().clone();
            let rows = receipt.consume(|rows, _, _| rows, |_| panic!("metered complete result"));
            (coordinates, rows)
        };
        let (coordinates, rows) = execute(&first, 7);
        assert_eq!(
            coordinates,
            QueryCoordinates::Started(SuppliedQueryCoordinates {
                now: Some(first.clone()),
                entropy_seed: Some(7)
            })
        );
        assert_eq!((coordinates.clone(), rows.clone()), execute(&first, 7));
        let (other, shifted_rows) = execute(&shifted, 7);
        assert_ne!(
            coordinates, other,
            "equal instants with different offsets have different NOW text"
        );
        assert_ne!(rows[0], shifted_rows[0]);
        assert_eq!(rows[1..], shifted_rows[1..]);
        let (other, seeded_rows) = execute(&first, 8);
        assert_ne!(coordinates, other);
        assert_eq!(rows[0], seeded_rows[0]);
        assert_ne!(rows[1..], seeded_rows[1..]);
    }

    /// Missing inputs remain missing even though the context has sampled defaults.
    #[test]
    fn started_receipt_distinguishes_partial_and_absent_supplied_inputs() {
        let engine = NativeSparqlEngine::new();
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let capability = WorkspaceCapability::resident();
        let mut prepared = engine
            .prepare_execution_admitted(
                "SELECT (1 AS ?value) WHERE {}",
                None,
                &[],
                QueryOptions::EMPTY,
                &capability,
            )
            .unwrap();
        let now = time("2024-02-29T12:34:56.125+05:30");
        for (options, expected) in [
            (
                QueryOptions::EMPTY,
                QueryCoordinates::Started(SuppliedQueryCoordinates {
                    now: None,
                    entropy_seed: None,
                }),
            ),
            (
                QueryOptions::new().with_now(&now),
                QueryCoordinates::Started(SuppliedQueryCoordinates {
                    now: Some(now.clone()),
                    entropy_seed: None,
                }),
            ),
            (
                QueryOptions::new().with_entropy_seed(0),
                QueryCoordinates::Started(SuppliedQueryCoordinates {
                    now: None,
                    entropy_seed: Some(0),
                }),
            ),
        ] {
            let receipt = engine
                .execute_governed_admitted(
                    &mut prepared,
                    &dataset,
                    options,
                    &QueryGovernors::METERED,
                    &capability,
                    |_| (),
                )
                .unwrap();
            assert_eq!(receipt.coordinates(), &expected);
            assert_ne!(receipt.coordinates(), &QueryCoordinates::NotStarted);
        }
    }

    /// Admission refusal never samples coordinates; an execution trip retains its starting inputs.
    #[test]
    fn refusal_and_execution_exhaustion_have_distinct_coordinate_states() {
        let engine = NativeSparqlEngine::new();
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let capability = WorkspaceCapability::resident();
        let mut prepared = engine
            .prepare_execution_admitted(
                "SELECT ?value WHERE { VALUES ?value { 1 2 3 } }",
                None,
                &[],
                QueryOptions::EMPTY,
                &capability,
            )
            .unwrap();
        let refused = without_host_clock_or_entropy(|| {
            engine.execute_governed_admitted(
                &mut prepared,
                &dataset,
                QueryOptions::EMPTY,
                &QueryGovernors::METERED.with_max_intermediate_cells(0),
                &capability,
                |_| panic!("refused visitor"),
            )
        })
        .unwrap();
        assert!(matches!(
            refused.outcome(),
            InternedGoverned::BudgetExhausted(_)
        ));
        assert_eq!(refused.coordinates(), &QueryCoordinates::NotStarted);
        let now = time("2024-02-29T12:34:56.125Z");
        let stopped = without_host_clock_or_entropy(|| {
            engine.execute_governed_admitted(
                &mut prepared,
                &dataset,
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
                &QueryGovernors::METERED.with_max_answers(1),
                &capability,
                |_| panic!("truncated visitor"),
            )
        })
        .unwrap();
        assert!(matches!(
            stopped.outcome(),
            InternedGoverned::BudgetExhausted(_)
        ));
        assert_eq!(
            stopped.coordinates(),
            &QueryCoordinates::Started(SuppliedQueryCoordinates {
                now: Some(now),
                entropy_seed: Some(7)
            })
        );
    }

    /// The coordinate is the only returned timestamp owner, while producers stay alive between runs.
    #[test]
    fn coordinate_receipts_keep_original_grants_and_repeated_runs_balance() {
        let (workspace, ledger, _) = crate::workspace::owned_row_regressions::account(u64::MAX);
        let capability = workspace.capability();
        let engine = NativeSparqlEngine::new();
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let mut prepared = engine
            .prepare_execution_admitted(
                "SELECT (1 AS ?value) WHERE {}",
                None,
                &[],
                QueryOptions::EMPTY,
                &capability,
            )
            .unwrap();
        let now = time(&format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096)));
        let parsed = ledger.live.load(Ordering::Relaxed);
        // The original prepared plan numbers its immutable tree lazily and
        // retains that admitted owner even in bounded execution. Initialize
        // exactly that owner before measuring per-execution receipt balance.
        drop(
            prepared
                .prepared
                .plan
                .handle_admitted(prepared.query().pattern(), &capability)
                .unwrap(),
        );
        let baseline = ledger.live.load(Ordering::Relaxed);
        println!(
            "Original retained numbered plan: {} bytes",
            baseline - parsed
        );
        for _ in 0..3 {
            let receipt = without_host_clock_or_entropy(|| {
                engine.execute_governed_admitted(
                    &mut prepared,
                    &dataset,
                    QueryOptions::new().with_now(&now).with_entropy_seed(7),
                    &QueryGovernors::METERED,
                    &capability,
                    |_| (),
                )
            })
            .unwrap();
            let charged = ledger.live.load(Ordering::Relaxed);
            assert!(charged > baseline);
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let coordinates = receipt.coordinates().clone();
            let measured = window.close();
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            drop(receipt);
            assert!(ledger.live.load(Ordering::Relaxed) > baseline);
            let QueryCoordinates::Started(SuppliedQueryCoordinates {
                now: Some(retained),
                entropy_seed: Some(7),
            }) = &coordinates
            else {
                panic!("supplied receipt")
            };
            assert!(retained.same_representation(&now));
            drop(coordinates);
            assert_eq!(ledger.live.load(Ordering::Relaxed), baseline);
        }
        let receipt = without_host_clock_or_entropy(|| {
            engine.execute_governed_admitted(
                &mut prepared,
                &dataset,
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
                &QueryGovernors::METERED,
                &capability,
                |_| (),
            )
        })
        .unwrap();
        let coordinates = receipt.coordinates().clone();
        drop(receipt);
        drop(now);
        drop(prepared);
        drop(engine);
        drop(capability);
        drop(workspace);
        assert!(ledger.live.load(Ordering::Relaxed) > 0);
        drop(coordinates);
        assert_eq!(ledger.live.load(Ordering::Relaxed), 0);
    }

    /// Physical refusal retains its original cause; cancellation is still a governed outcome.
    #[test]
    fn native_refusal_and_cancellation_keep_their_original_causes() {
        let engine = NativeSparqlEngine::new();
        let dataset = RdfDatasetBuilder::new().freeze().unwrap();
        let resident = WorkspaceCapability::resident();
        let mut prepared = engine
            .prepare_execution_admitted(
                "SELECT ?value WHERE { VALUES ?value { 1 2 3 } }",
                None,
                &[],
                QueryOptions::EMPTY,
                &resident,
            )
            .unwrap();
        let base = crate::workspace::QueryWorkspace::<
            crate::workspace::owned_row_regressions::Refusal,
        >::control_bytes()
        .unwrap();
        let (workspace, ledger, _) = crate::workspace::owned_row_regressions::account(base);
        let refused = without_host_clock_or_entropy(|| {
            engine.execute_governed_admitted(
                &mut prepared,
                &dataset,
                QueryOptions::EMPTY,
                &QueryGovernors::METERED,
                &workspace.capability(),
                |_| panic!("physical refusal cannot visit"),
            )
        });
        assert!(matches!(
            refused,
            Err(crate::EvaluationFailure::Evaluation(
                EvalError::WorkspaceStopped
            ))
        ));
        assert_eq!(workspace.take_failure().unwrap().identity, 7);
        assert_eq!(ledger.live.load(Ordering::Relaxed), base);

        let now = time("2024-02-29T12:34:56.125Z");
        let stop = std::sync::Arc::new(crate::CancellationFlag::new());
        stop.cancel();
        let cancelled = without_host_clock_or_entropy(|| {
            engine.execute_governed_admitted(
                &mut prepared,
                &dataset,
                QueryOptions::new().with_now(&now).with_entropy_seed(7),
                &QueryGovernors::METERED.with_stop_signal(stop),
                &resident,
                |_| panic!("cancelled visitor"),
            )
        })
        .unwrap();
        let InternedGoverned::BudgetExhausted(exhausted) = cancelled.outcome() else {
            panic!("governed cancellation");
        };
        assert_eq!(
            exhausted.tripped,
            purrdf_core::TrippedGovernor::Stopped {
                cause: purrdf_core::StopCause::Cancelled
            }
        );
        assert!(matches!(
            cancelled.coordinates(),
            QueryCoordinates::Started(_)
        ));
    }
}

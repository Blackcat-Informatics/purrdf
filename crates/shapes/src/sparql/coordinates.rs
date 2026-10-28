// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller inputs for one serial rule execution and its nested SPARQL requests.

use purrdf_sparql_eval::{EvalError, ExecutionCoordinateObserver, QueryCoordinates};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use purrdf_hash::hex::Digest32;
use purrdf_xsd::temporal::DateTime;

/// Inputs and actual query contexts of one rule inference, including failures.
///
/// Supplied getters retain caller controls; each nested execution records its
/// actual clock and seed, including sampled defaults. Empty request options
/// can inherit an outer supplied scope; a partially supplied request replaces it.
/// The exact time shares the original caller-input owner, extending its lifetime
/// without copying its coefficient or allocating another owner. This retention is
/// not a new execution allocation. A complete execution identity is available
/// when the source and every nested query expose certified identities.
#[derive(Debug, Clone, Default)]
pub struct RuleCoordinates {
    now: Option<Arc<DateTime>>,
    entropy_seed: Option<u64>,
    executions: Arc<[QueryCoordinates]>,
    pub(crate) program: Option<Digest32>,
    pub(crate) snapshot: Option<Digest32>,
    pub(crate) shapes: Option<Digest32>,
    pub(crate) contract: Option<Digest32>,
    pub(crate) source_rules: bool,
    execution: Option<Digest32>,
}

impl RuleCoordinates {
    /// Certified semantic program identity, when available.
    #[must_use]
    pub const fn program_identity(&self) -> Option<&Digest32> {
        self.program.as_ref()
    }
    /// Certified immutable source identity, when available.
    #[must_use]
    pub const fn snapshot_identity(&self) -> Option<&Digest32> {
        self.snapshot.as_ref()
    }
    /// Certified shapes graph identity, when available.
    #[must_use]
    pub const fn shapes_identity(&self) -> Option<&Digest32> {
        self.shapes.as_ref()
    }
    /// Identity of the program, input snapshots and ordered actual query contexts.
    #[must_use]
    pub const fn execution_identity(&self) -> Option<&Digest32> {
        self.execution.as_ref()
    }
    /// Actual nested query contexts in execution order, including sampled defaults.
    #[must_use]
    pub fn executions(&self) -> &[QueryCoordinates] {
        &self.executions
    }

    /// The effective supplied time, including exact seconds and declared timezone.
    #[must_use]
    pub fn now(&self) -> Option<&DateTime> {
        self.now.as_deref()
    }

    /// Starting state of this inference's `SplitMix64` query-seed sequence.
    /// This is not an individual query's seed or its evolved random state.
    /// Capturing the receipt consumes no draw.
    #[must_use]
    pub const fn entropy_seed(&self) -> Option<u64> {
        self.entropy_seed
    }
}

impl PartialEq for RuleCoordinates {
    fn eq(&self, other: &Self) -> bool {
        self.executions == other.executions
            && self.program == other.program
            && self.snapshot == other.snapshot
            && self.shapes == other.shapes
            && self.contract == other.contract
            && self.source_rules == other.source_rules
            && self.execution == other.execution
            && self.entropy_seed == other.entropy_seed
            && match (&self.now, &other.now) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    // Equal instants with different offsets can emit different
                    // STR(NOW()) values. Compare the retained representation,
                    // whose Decimal seconds already normalize lexical zeros.
                    left.same_representation(right)
                }
                _ => false,
            }
    }
}

impl Eq for RuleCoordinates {}

/// Borrow the effective runtime before any query advances its seed sequence.
pub(crate) fn snapshot() -> RuleCoordinates {
    CURRENT.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or_else(RuleCoordinates::default, |runtime| RuleCoordinates {
                now: runtime.now.clone(),
                entropy_seed: runtime.entropy,
                executions: Arc::default(),
                program: None,
                snapshot: None,
                shapes: None,
                contract: None,
                source_rules: false,
                execution: None,
            })
    })
}

#[derive(Debug)]
pub(crate) struct Runtime {
    now: Option<Arc<DateTime>>,
    entropy: Option<u64>,
    recorder: Option<Arc<Recorder>>,
}

thread_local! {
    static CURRENT: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

#[must_use]
pub(crate) struct Scope {
    previous: Option<Runtime>,
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let _ = replace(self.previous.take());
    }
}

pub(crate) fn enter(now: Option<Arc<DateTime>>, entropy: Option<u64>) -> Option<Scope> {
    if now.is_none() && entropy.is_none() {
        return None;
    }
    Some(Scope {
        previous: replace(Some(Runtime {
            now,
            entropy,
            recorder: recorder(),
        })),
        _not_send: PhantomData,
    })
}

pub(super) fn replace(next: Option<Runtime>) -> Option<Runtime> {
    CURRENT.with(|slot| slot.replace(next))
}

pub(super) fn now() -> Option<Arc<DateTime>> {
    CURRENT.with(|slot| slot.borrow().as_ref()?.now.clone())
}

/// Draw only at the common execution-options seam, never during preparation.
pub(super) fn next_seed() -> Option<u64> {
    CURRENT.with(|slot| {
        let mut runtime = slot.borrow_mut();
        let state = runtime.as_mut()?.entropy.as_mut()?;
        Some(purrdf_hash::mix::splitmix64_next(state))
    })
}

/// Request-owned capture; it travels with the existing suspended ambient runtime.
#[derive(Debug)]
pub(crate) struct Recorder {
    entries: Mutex<Vec<QueryCoordinates>>,
    parent: Option<Arc<Self>>,
    implementation: std::sync::OnceLock<Digest32>,
}
impl Recorder {
    pub(crate) fn implementation_identity(&self) -> Option<&Digest32> {
        self.implementation.get()
    }
}
impl ExecutionCoordinateObserver for Recorder {
    fn observe(&self, coordinates: &QueryCoordinates) -> Result<(), EvalError> {
        let mut entries = self.entries.lock().map_err(|_| {
            EvalError::Invariant(purrdf_sparql_eval::EvaluatorInvariant::CoordinateObserverPoisoned)
        })?;
        entries
            .try_reserve(1)
            .map_err(|_| EvalError::AllocationFailed {
                construct: "rule execution coordinates",
            })?;
        entries.push(coordinates.clone());
        drop(entries);
        if let Some(parent) = &self.parent {
            parent.observe(coordinates)?;
        }
        Ok(())
    }
}

pub(super) fn recorder() -> Option<Arc<Recorder>> {
    CURRENT.with(|slot| slot.borrow().as_ref()?.recorder.clone())
}

/// A distinct record for this inference, without restarting an inherited seed stream.
pub(crate) struct RecordingScope {
    recorder: Arc<Recorder>,
    previous: Option<Arc<Recorder>>,
    inserted: bool,
    _not_send: PhantomData<Rc<()>>,
}
impl RecordingScope {
    pub(crate) fn enter() -> Self {
        let recorder = Arc::new(Recorder {
            entries: Mutex::new(Vec::new()),
            parent: recorder(),
            implementation: std::sync::OnceLock::new(),
        });
        let (previous, inserted) = CURRENT.with(|slot| {
            let mut current = slot.borrow_mut();
            let inserted = current.is_none();
            let runtime = current.get_or_insert_with(|| Runtime {
                now: None,
                entropy: None,
                recorder: None,
            });
            (runtime.recorder.replace(Arc::clone(&recorder)), inserted)
        });
        Self {
            recorder,
            previous,
            inserted,
            _not_send: PhantomData,
        }
    }
    pub(crate) fn bind_implementation(&self, identity: Digest32) -> Result<(), EvalError> {
        self.recorder.implementation.set(identity).map_err(|_| {
            EvalError::Invariant(
                purrdf_sparql_eval::EvaluatorInvariant::CoordinateEnvironmentChanged,
            )
        })
    }
    pub(crate) fn finish(
        &self,
        mut coordinates: RuleCoordinates,
    ) -> Result<RuleCoordinates, EvalError> {
        let mut entries = self.recorder.entries.lock().map_err(|_| {
            EvalError::Invariant(purrdf_sparql_eval::EvaluatorInvariant::CoordinateObserverPoisoned)
        })?;
        coordinates.executions = std::mem::take(&mut *entries).into();
        drop(entries);
        coordinates.execution = execution_identity(&coordinates);
        Ok(coordinates)
    }
}

const EXECUTION: purrdf_hash::Domain = purrdf_hash::Domain::new(b"purrdf-shapes/rule-execution/v1");

fn execution_identity(coordinates: &RuleCoordinates) -> Option<Digest32> {
    let mut hash = purrdf_hash::blake3::Hasher::new();
    let mut field = |bytes: &[u8]| {
        purrdf_hash::frame::frame_le_with(bytes, &mut |part| {
            hash.update(part);
        });
    };
    field(EXECUTION.as_bytes());
    field(coordinates.program.as_ref()?.as_bytes());
    field(coordinates.snapshot.as_ref()?.as_bytes());
    field(coordinates.shapes.as_ref()?.as_bytes());
    field(coordinates.contract.as_ref()?.as_bytes());
    field(&[u8::from(coordinates.source_rules)]);
    match coordinates.now() {
        Some(now) => {
            field(b"supplied-now");
            field(now.representation_identity().as_bytes());
        }
        None => field(b"unsupplied-now"),
    }
    match coordinates.entropy_seed() {
        Some(seed) => {
            field(b"supplied-seed");
            field(&seed.to_le_bytes());
        }
        None => field(b"unsupplied-seed"),
    }
    field(&(coordinates.executions.len() as u64).to_le_bytes());
    for execution in coordinates.executions() {
        let QueryCoordinates::Started(started) = execution else {
            return None;
        };
        field(started.execution_identity()?.as_bytes());
    }
    Some(Digest32::from(*hash.finalize().as_bytes()))
}
impl Drop for RecordingScope {
    fn drop(&mut self) {
        CURRENT.with(|slot| {
            let mut current = slot.borrow_mut();
            if self.inserted {
                *current = None;
            } else if let Some(runtime) = current.as_mut() {
                runtime.recorder = self.previous.take();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspended_and_nested_scopes_preserve_the_seed_counter() {
        let mut reference = 7;
        let first = purrdf_hash::mix::splitmix64_next(&mut reference);
        let second = purrdf_hash::mix::splitmix64_next(&mut reference);
        let outer = enter(None, Some(7)).unwrap();
        assert_eq!(snapshot().entropy_seed(), Some(7));
        assert_eq!(next_seed(), Some(first));
        let suspended = replace(None);
        assert_eq!(next_seed(), None);
        assert!(enter(None, None).is_none());
        let inner = enter(None, Some(100)).unwrap();
        let mut reference = 100;
        assert_eq!(
            next_seed(),
            Some(purrdf_hash::mix::splitmix64_next(&mut reference))
        );
        drop(inner);
        assert_eq!(next_seed(), None);
        assert!(replace(suspended).is_none());
        assert_eq!(next_seed(), Some(second));
        drop(outer);
        assert_eq!(next_seed(), None);
        let _fresh = enter(None, Some(7)).unwrap();
        assert_eq!(next_seed(), Some(first));
    }

    #[test]
    fn suspended_time_keeps_the_original_exact_input_owner() {
        let lexical = format!("2024-02-29T12:34:56.{}+05:30", "7".repeat(4096));
        let caller = Arc::new(purrdf_xsd::temporal::parse_datetime(&lexical).unwrap());
        let identity = Arc::as_ptr(&caller);
        let expected = caller.canonical_lexical();
        let scope = enter(Some(Arc::clone(&caller)), Some(7)).unwrap();
        drop(caller);
        let selected = now().unwrap();
        assert_eq!(Arc::as_ptr(&selected), identity);
        assert_eq!(selected.canonical_lexical(), expected);
        drop(selected);
        let suspended = replace(None);
        assert!(now().is_none());
        let nested = enter(
            Some(Arc::new(
                purrdf_xsd::temporal::parse_datetime("2025-01-01T00:00:00Z").unwrap(),
            )),
            Some(8),
        )
        .unwrap();
        assert_ne!(Arc::as_ptr(&now().unwrap()), identity);
        drop(nested);
        assert!(now().is_none());
        assert!(replace(suspended).is_none());
        let selected = now().unwrap();
        assert_eq!(Arc::as_ptr(&selected), identity);
        assert_eq!(selected.canonical_lexical(), expected);
        drop(scope);
        assert!(now().is_none());
        assert_eq!(selected.canonical_lexical(), expected);
    }

    #[test]
    fn inference_receipt_inherits_current_seed_and_releases_original_time_owner() {
        use crate::srl::{self, InferOptions};

        let document = srl::parse_and_check(
            "PREFIX : <http://example.org/> RULE { :s :time ?time } WHERE { SET(?time := NOW()) }",
            None,
        )
        .unwrap();
        let source = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        let caller = Arc::new(
            purrdf_xsd::temporal::parse_datetime(&format!(
                "2024-02-29T12:34:56.{}+05:30",
                "7".repeat(4096)
            ))
            .unwrap(),
        );
        let original = Arc::as_ptr(&caller);
        let weak = Arc::downgrade(&caller);
        let scope = enter(Some(caller), Some(7)).unwrap();
        let mut expected_seed = 7;
        assert_eq!(
            next_seed(),
            Some(purrdf_hash::mix::splitmix64_next(&mut expected_seed))
        );
        let result = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
            srl::infer(&document, &source, &InferOptions::default())
        })
        .unwrap();
        assert_eq!(result.coordinates().entropy_seed(), Some(expected_seed));
        assert!(result.coordinates().program_identity().is_some());
        assert!(result.coordinates().snapshot_identity().is_some());
        assert!(result.coordinates().execution_identity().is_some());
        let mut next = expected_seed;
        let first_seed = purrdf_hash::mix::splitmix64_next(&mut next);
        let QueryCoordinates::Started(first) = &result.coordinates().executions()[0] else {
            panic!("rule query context")
        };
        assert_eq!(first.effective_entropy_seed(), first_seed);
        assert_eq!(
            std::ptr::from_ref(result.coordinates().now().unwrap()),
            original
        );

        // A partial request replaces the complete outer scope, rather than
        // silently inheriting its missing input. Returning restores the outer run.
        let time_only = InferOptions::default()
            .with_now(purrdf_xsd::temporal::parse_datetime("2025-01-01T00:00:00Z").unwrap());
        let replaced = srl::infer(&document, &source, &time_only).unwrap();
        assert!(replaced.coordinates().entropy_seed().is_none());
        assert_ne!(result.coordinates(), replaced.coordinates());
        let seed_only = srl::infer(
            &document,
            &source,
            &InferOptions::default().with_entropy_seed(11),
        )
        .unwrap();
        assert!(seed_only.coordinates().now().is_none());
        assert_eq!(seed_only.coordinates().entropy_seed(), Some(11));
        assert_eq!(Arc::as_ptr(&now().unwrap()), original);

        let retained = result.clone();
        let receipt = result.coordinates().clone();
        drop(result);
        drop(scope);
        drop(source);
        drop(document);
        assert!(now().is_none());
        assert!(weak.upgrade().is_some());
        assert_eq!(retained.coordinates(), &receipt);
        drop(retained);
        assert!(weak.upgrade().is_some());
        drop(receipt);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn query_free_rules_bind_the_program_and_keep_limit_failure_receipts() {
        use crate::srl::{self, InferOptions};
        let first = srl::parse_and_check(
            "PREFIX : <https://example.org/> RULE { :s :p :first } WHERE {}",
            None,
        )
        .unwrap();
        let second = srl::parse_and_check(
            "PREFIX : <https://example.org/> RULE { :s :p :second } WHERE {}",
            None,
        )
        .unwrap();
        let source = purrdf_core::RdfDatasetBuilder::new().freeze().unwrap();
        let options = InferOptions::default();
        let first_result = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
            srl::infer(&first, &source, &options)
        })
        .unwrap();
        let repeated = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
            srl::infer(&first, &source, &options)
        })
        .unwrap();
        let second_result = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
            srl::infer(&second, &source, &options)
        })
        .unwrap();
        assert_eq!(first_result.coordinates().executions(), []);
        assert!(first_result.coordinates().execution_identity().is_some());
        assert_eq!(first_result.coordinates(), repeated.coordinates());
        assert_ne!(
            first_result.coordinates().program_identity(),
            second_result.coordinates().program_identity()
        );
        assert_ne!(
            first_result.coordinates().execution_identity(),
            second_result.coordinates().execution_identity()
        );
        let error = srl::infer(
            &first,
            &source,
            &InferOptions::default().with_max_stored_facts(0),
        )
        .unwrap_err();
        let srl::SrlError::Execution(error) = error else {
            panic!("receipt-bearing rule failure")
        };
        assert_eq!(
            error.coordinates.program_identity(),
            first_result.coordinates().program_identity()
        );
        assert_eq!(
            error.coordinates.snapshot_identity(),
            first_result.coordinates().snapshot_identity()
        );
        assert!(error.coordinates.execution_identity().is_some());
        assert!(matches!(
            *error.cause,
            crate::rules::RulesError::LimitExceeded(_)
        ));
    }
}

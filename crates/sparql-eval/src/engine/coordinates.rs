// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Effective execution coordinates retain their original native admission.

use crate::workspace::LexicalFrame;
use crate::{EvalError, WorkspaceCapability};
use purrdf_lex::allocation::{Memory, SharedOwned};
use purrdf_xsd::{XsdValue, temporal::DateTime};

/// Receives the actual context coordinates before evaluation or random draws.
/// A refusal aborts execution through the evaluator's native error channel.
pub trait ExecutionCoordinateObserver: Send + Sync {
    /// Retain or incorporate one context's original coordinate receipt.
    ///
    /// # Errors
    /// Returns the observer's admission or publication failure.
    fn observe(&self, coordinates: &QueryCoordinates) -> Result<(), EvalError>;
}

/// Actual coordinates retained by native execution outcomes and evidence.
/// Supplied and sampled inputs share the original admitted timestamp owner.
/// UPDATE records its first constructed WHERE context; operations without one
/// leave the context receipt unstarted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryCoordinates {
    /// No evaluation context was successfully created. Preparation or reads may
    /// already have occurred; this is not a claim that the request did no work.
    NotStarted,
    /// The context was created, whether evaluation completed or exhausted a budget.
    Started(SuppliedQueryCoordinates),
}

/// Effective inputs and identities captured before the context's first draw.
/// Private fields preserve the original timestamp owner across every clone.
#[derive(Debug, Clone)]
pub struct SuppliedQueryCoordinates {
    now: DateTime,
    entropy_seed: u64,
    supplied_now: bool,
    supplied_seed: bool,
    snapshot: Option<purrdf_hash::hex::Digest32>,
    execution: Option<purrdf_hash::hex::Digest32>,
}
impl SuppliedQueryCoordinates {
    /// Effective supplied time, including exact seconds and declared timezone.
    #[must_use]
    pub const fn now(&self) -> Option<&DateTime> {
        if self.supplied_now {
            Some(&self.now)
        } else {
            None
        }
    }
    /// Effective supplied seed before the execution's first random draw.
    #[must_use]
    pub const fn entropy_seed(&self) -> Option<u64> {
        if self.supplied_seed {
            Some(self.entropy_seed)
        } else {
            None
        }
    }
    /// The context's actual clock value, including a sampled default.
    #[must_use]
    pub const fn effective_now(&self) -> &DateTime {
        &self.now
    }
    /// The context's actual initial entropy state, before its first draw.
    #[must_use]
    pub const fn effective_entropy_seed(&self) -> u64 {
        self.entropy_seed
    }
    /// Certified immutable source identity when the carrier supplies one.
    #[must_use]
    pub const fn snapshot_identity(&self) -> Option<&purrdf_hash::hex::Digest32> {
        self.snapshot.as_ref()
    }
    /// Identity binding the request, source and actual effective coordinates.
    #[must_use]
    pub const fn execution_identity(&self) -> Option<&purrdf_hash::hex::Digest32> {
        self.execution.as_ref()
    }
}
impl PartialEq for SuppliedQueryCoordinates {
    fn eq(&self, other: &Self) -> bool {
        self.entropy_seed == other.entropy_seed
            && self.supplied_now == other.supplied_now
            && self.supplied_seed == other.supplied_seed
            && self.now.same_representation(&other.now)
            && self.snapshot == other.snapshot
            && self.execution == other.execution
    }
}
impl Eq for SuppliedQueryCoordinates {}

impl QueryCoordinates {
    pub(crate) fn started(
        now: &XsdValue,
        seed: u64,
        supplied_now: bool,
        supplied_seed: bool,
    ) -> Result<Self, EvalError> {
        let XsdValue::DateTime(now) = now else {
            return Err(EvalError::Invariant(
                crate::EvaluatorInvariant::QueryClockNotDateTime,
            ));
        };
        Ok(Self::Started(SuppliedQueryCoordinates {
            now: now.clone(),
            entropy_seed: seed,
            supplied_now,
            supplied_seed,
            snapshot: None,
            execution: None,
        }))
    }
}

/// Capture the request's exact algebra and effective bindings under its account.
pub(crate) fn capture_query<D: purrdf_core::DatasetView + Sync>(
    ctx: &crate::eval::EvalCtx<'_, D>,
    prepared: &super::PreparedQuery,
    bindings: crate::substitute::Prebindings<'_>,
    options: super::QueryOptions<'_, '_>,
) -> Result<QueryCoordinates, EvalError> {
    let request: Result<[u8; 32], EvalError> = (|| {
        let mut frame = LexicalFrame::new(&ctx.growth);
        let mut memory = Memory::new(&mut frame);
        let query = match prepared.coordinate_identity.get() {
            Some(identity) => *identity,
            None => {
                let identity = purrdf_sparql_algebra::identity::query_with_memory(
                    &prepared.query,
                    &mut memory,
                )
                .map_err(|error| ctx.growth.storage_error(error, "execution query identity"))?;
                *prepared.coordinate_identity.get_or_init(|| identity)
            }
        };
        let mut request = purrdf_hash::blake3::Streaming::<1024>::new();
        const DOMAIN: purrdf_hash::Domain =
            purrdf_hash::Domain::new(b"purrdf-sparql-eval/bound-request/v1");
        request.update(DOMAIN.as_bytes());
        request.update(query.as_bytes());
        request.update(&(bindings.len() as u64).to_le_bytes());
        for index in 0..bindings.len() {
            purrdf_hash::frame::frame_le_with(bindings.name(index).as_bytes(), &mut |part| {
                request.update(part);
            });
            let ground = bindings
                .ground_with_memory(index, &mut memory)
                .map_err(crate::substitute::GroundFailure::into_eval_error)?;
            let identity = purrdf_sparql_algebra::identity::node_with_memory(
                purrdf_sparql_algebra::walk::NodeRef::Ground(&ground),
                &mut memory,
            )
            .map_err(|error| {
                ctx.growth
                    .storage_error(error, "execution binding identity")
            })?;
            request.update(identity.as_bytes());
        }
        request.update(&prepared.geo.framed_bytes());
        Ok(*request.finalize().as_bytes())
    })();
    match request {
        Ok(request) => capture_request(ctx, request, options, false),
        Err(error) => {
            if let Some(observer) = options.coordinate_observer {
                observer.observe(&QueryCoordinates::started(
                    &ctx.now,
                    ctx.rng_state,
                    options.now.is_some(),
                    options.entropy_seed.is_some(),
                )?)?;
            }
            Err(error)
        }
    }
}

/// Bind a request digest to its actual immutable source and initial coordinates.
pub(crate) fn capture_request<D: purrdf_core::DatasetView + Sync>(
    ctx: &crate::eval::EvalCtx<'_, D>,
    request: [u8; 32],
    options: super::QueryOptions<'_, '_>,
    external_source: bool,
) -> Result<QueryCoordinates, EvalError> {
    let mut coordinates = QueryCoordinates::started(
        &ctx.now,
        ctx.rng_state,
        options.now.is_some(),
        options.entropy_seed.is_some(),
    )?;
    let identity: Result<(), EvalError> = (|| {
        let QueryCoordinates::Started(value) = &mut coordinates else {
            unreachable!()
        };
        value.snapshot = ctx
            .dataset
            .snapshot_identity()
            .map_err(|error| ctx.workspace.source_error(error))?;
        let opaque = external_source
            || options.candidate_scope.is_some()
            || options.remote.is_some()
            || options.load.is_some()
            || !options.property_functions().is_empty()
            || !options.aggregates().is_empty()
            || options.functions.declarations().has_opaque_bodies();
        if let Some(snapshot) = value.snapshot
            && (!opaque || options.implementation_identity.is_some())
        {
            const DOMAIN: purrdf_hash::Domain =
                purrdf_hash::Domain::new(b"purrdf-sparql-eval/execution-coordinates/v1");
            let mut digest = purrdf_hash::blake3::Streaming::<1024>::new();
            digest.update(DOMAIN.as_bytes());
            digest.update(&request);
            match options.implementation_identity {
                Some(identity) => {
                    digest.update(&[1]);
                    digest.update(identity.as_bytes());
                }
                None => {
                    digest.update(&[0]);
                }
            }
            digest.update(&[match options.graph_existence {
                purrdf_core::GraphExistenceMode::Implicit => 0,
                purrdf_core::GraphExistenceMode::RememberEmpty => 1,
                _ => {
                    return Err(EvalError::unsupported(
                        "unrecognized graph lifetime policy in execution identity",
                    ));
                }
            }]);
            digest.update(snapshot.as_bytes());
            digest.update(value.now.representation_identity().as_bytes());
            digest.update(&value.entropy_seed.to_le_bytes());
            let mut frame = LexicalFrame::new(&ctx.growth);
            let mut memory = Memory::new(&mut frame);
            digest.update(
                options
                    .env
                    .content_fingerprint_with_memory(&mut memory)?
                    .as_bytes(),
            );
            for population in [
                crate::user_fn::FnPopulation::Declared,
                crate::user_fn::FnPopulation::Injected,
            ] {
                digest.update(
                    crate::user_fn::content_fingerprint_with_memory(
                        options.functions.declarations(),
                        population,
                        &mut memory,
                    )?
                    .as_bytes(),
                );
            }
            purrdf_hash::frame::frame_le_with(
                options.bnode_mint_prefix.unwrap_or("").as_bytes(),
                &mut |part| {
                    digest.update(part);
                },
            );
            digest.update(&[u8::from(options.disjoint_language_strings)]);
            // Canonical policy text is emitted directly into a bounded-stack hash
            // sink; no formatted String or unowned scratch allocation is created.
            let mut policy = purrdf_hash::blake3::Streaming::<1024>::new();
            std::fmt::write(
                &mut DigestText(&mut policy),
                format_args!("{}", options.division),
            )
            .map_err(|_| {
                EvalError::Invariant(crate::EvaluatorInvariant::CoordinateIdentityFormatting)
            })?;
            digest.update(policy.finalize().as_bytes());
            if let Some(selection) = ctx.xpath_regex {
                use purrdf_core::xsd_regex::xpath::Resource;
                digest.update(&[1]);
                purrdf_hash::frame::frame_le_with(
                    selection.profile.name().as_bytes(),
                    &mut |part| {
                        digest.update(part);
                    },
                );
                for resource in [
                    Resource::PatternBytes,
                    Resource::CompileSteps,
                    Resource::ProgramNodes,
                    Resource::CompileSlots,
                    Resource::MatchSteps,
                    Resource::MatchStates,
                    Resource::MatchSlots,
                    Resource::OutputBytes,
                ] {
                    digest.update(&selection.limits.limit(resource).to_le_bytes());
                }
            } else {
                digest.update(&[0]);
            }
            digest.update(&[
                u8::from(ctx.options.exists_memo),
                u8::from(ctx.options.force_structural_bgp_order),
                u8::from(ctx.options.force_sequential),
                u8::from(ctx.options.disable_shapes),
            ]);
            digest.update(&options.call_depth.to_le_bytes());
            match &ctx.standpoint_predicates {
                Some(predicates) => {
                    digest.update(&[1]);
                    for text in [&predicates.according_to, &predicates.sharpens] {
                        purrdf_hash::frame::frame_le_with(text.as_bytes(), &mut |part| {
                            digest.update(part);
                        });
                    }
                }
                None => {
                    digest.update(&[0]);
                }
            }
            match &ctx.loss_vocabulary {
                Some(vocabulary) => {
                    digest.update(&[1]);
                    for text in [
                        &vocabulary.projection_loss,
                        &vocabulary.loss_code,
                        &vocabulary.lost_reifies,
                    ] {
                        purrdf_hash::frame::frame_le_with(text.as_bytes(), &mut |part| {
                            digest.update(part);
                        });
                    }
                }
                None => {
                    digest.update(&[0]);
                }
            }
            if let Some(graph) = options.focus_graph {
                digest.update(&[1]);
                match purrdf_core::DatasetView::snapshot_identity(graph.as_ref()) {
                    Ok(Some(identity)) => {
                        digest.update(identity.as_bytes());
                    }
                    Ok(None) => {
                        return Err(EvalError::Invariant(
                            crate::EvaluatorInvariant::CoordinateFocusIdentityMissing,
                        ));
                    }
                    Err(never) => match never {},
                }
            } else {
                digest.update(&[0]);
            }
            value.execution = Some(purrdf_hash::hex::Digest32::new(
                *digest.finalize().as_bytes(),
            ));
        }
        Ok(())
    })();
    if let Some(observer) = options.coordinate_observer {
        observer.observe(&coordinates)?;
    }
    identity?;
    Ok(coordinates)
}

struct DigestText<'a>(&'a mut purrdf_hash::blake3::Streaming<1024>);
impl std::fmt::Write for DigestText<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0.update(text.as_bytes());
        Ok(())
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
            let materialized = without_host_clock_or_entropy(|| {
                engine.query_governed_view(
                    &dataset,
                    purrdf_core::SparqlRequest {
                        query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    QueryOptions::new().with_now(now).with_entropy_seed(seed),
                    &QueryGovernors::METERED,
                )
            })
            .unwrap();
            assert_eq!(materialized.coordinates(), &coordinates);

            let rows = receipt.consume(|rows, _, _| rows, |_| panic!("metered complete result"));
            (coordinates, rows)
        };
        let (coordinates, rows) = execute(&first, 7);
        let QueryCoordinates::Started(started) = &coordinates else {
            panic!("started query")
        };
        assert!(started.now().unwrap().same_representation(&first));
        assert_eq!(started.entropy_seed(), Some(7));
        assert!(started.execution_identity().is_some());
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

    /// Every entry reports actual defaults while keeping supplied inputs distinguishable.
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
        for (options, supplied_now, supplied_seed) in [
            (QueryOptions::EMPTY, false, None),
            (QueryOptions::new().with_now(&now), true, None),
            (QueryOptions::new().with_entropy_seed(0), false, Some(0)),
        ] {
            let assert_receipt = |coordinates: &QueryCoordinates| {
                let QueryCoordinates::Started(started) = coordinates else {
                    panic!("started query")
                };
                assert_eq!(started.now().is_some(), supplied_now);
                if supplied_now {
                    assert!(started.effective_now().same_representation(&now));
                }
                assert_eq!(started.entropy_seed(), supplied_seed);
                if let Some(seed) = supplied_seed {
                    assert_eq!(started.effective_entropy_seed(), seed);
                }
                assert!(started.snapshot_identity().is_some());
                assert!(started.execution_identity().is_some());
            };
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
            assert_receipt(receipt.coordinates());
            let query = "SELECT (1 AS ?value) WHERE {}";
            let request = purrdf_core::SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            };
            let materialized = engine
                .query_governed_view(&dataset, request, options, &QueryGovernors::METERED)
                .unwrap();
            assert_receipt(materialized.coordinates());
            let state = std::sync::Arc::new(crate::GovernorState::new(&QueryGovernors::METERED));
            let interned = engine
                .query_governed_interned_in_operation(
                    &dataset,
                    super::super::InternedRequest {
                        query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    options,
                    &state,
                    |_| (),
                )
                .unwrap();
            assert_receipt(interned.coordinates());
            let converted = engine
                .execute_governed_in_operation(&mut prepared, &dataset, options, &state, |_| ())
                .unwrap();
            assert_receipt(converted.coordinates());

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
        let QueryCoordinates::Started(started) = stopped.coordinates() else {
            panic!("started query")
        };
        assert!(started.effective_now().same_representation(&now));
        assert_eq!(started.entropy_seed(), Some(7));
        assert!(started.execution_identity().is_some());
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
            let QueryCoordinates::Started(started) = &coordinates else {
                panic!("supplied receipt")
            };
            assert!(started.effective_now().same_representation(&now));
            assert_eq!(started.entropy_seed(), Some(7));
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

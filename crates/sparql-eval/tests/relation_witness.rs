// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The relation-attestation channel, driven end to end through the production entry
//! points a host actually calls.
//!
//! Two facts about a host's index can change a query's answer while every input this
//! engine can see stays identical: which generation of the index answered, and whether
//! that index was whole. Both are known by the relation's cursor and by nothing else.
//! Every test here drives a real engine entry — `query_prepared_governed_view`,
//! `query_with_options_view`, `update_with_options` — because a channel that only lines
//! up when the evaluator is called from inside the crate is a channel no host can read.

use std::collections::BTreeSet;
use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    BindingPattern, EvalError, EvalOptions, ExtensionEnv, GovernedOutcome, GovernorState,
    IndexGeneration, InternedGoverned, InternedOutcome, InternedRequest, NativeSparqlEngine,
    PfArgs, PfArity, PfAttestation, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    QueryGovernors, QueryOptions, RelationAttestations, RelationWitness, ServiceLevel, Volatility,
};

/// The relation IRI every query below calls. PurRDF mints no vocabulary: without this
/// host-supplied registration the same predicate is an ordinary triple pattern.
const REL_IRI: &str = "https://example.org/rel/memberOf";

/// The data namespace of the fixture terms.
const EX: &str = "https://example.org/d/";

/// The reason string the incomplete fixtures declare, verbatim.
const SHARD_REASON: &str = "shard 3 of 4 is still rebuilding";

// ---------------------------------------------------------------------------
// Fixture relations
// ---------------------------------------------------------------------------

/// What a fixture cursor declares, and when.
#[derive(Clone, Copy)]
enum Declares {
    /// Overrides NEITHER cursor method: the defaults answer for it.
    Nothing,
    /// A generation at open, nothing at close.
    Generation(&'static str),
    /// A generation at open, an incompleteness at close.
    GenerationThenIncomplete(&'static str, &'static str),
    /// Nothing at open, an incompleteness at close — the shard-found-missing-late case.
    UndeclaredThenIncomplete(&'static str),
    /// Panics when asked for its generation.
    PanicOnGeneration,
    /// Panics when asked for its service level.
    PanicOnServiceLevel,
}

/// A relation whose cursor attests whatever `declares` says, over `modes`.
///
/// One row per invocation: the bound subject echoed back (so the engine's equality
/// filter keeps it) beside a fixed team IRI.
struct AttestingRelation {
    modes: Vec<BindingPattern>,
    declares: Declares,
}

impl AttestingRelation {
    fn new(mode_code: &str, declares: Declares) -> Self {
        Self {
            modes: vec![BindingPattern::from_code(mode_code)],
            declares,
        }
    }
}

impl PropertyFunction for AttestingRelation {
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        PfArity::new(1, 1)
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
        1
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        _ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let subject = args
            .subject()
            .first()
            .copied()
            .flatten()
            .cloned()
            .unwrap_or_else(|| TermValue::iri(format!("{EX}ada")));
        let row = vec![subject, TermValue::iri(format!("{EX}alpha"))];
        match self.declares {
            Declares::Nothing => Ok(Box::new(SilentCursor { row: Some(row) })),
            declares => Ok(Box::new(AttestingCursor {
                row: Some(row),
                declares,
            })),
        }
    }
}

/// A cursor that overrides NEITHER attestation method — every relation written before
/// the channel existed, and the control case for T2.3.
struct SilentCursor {
    row: Option<PfRow>,
}

impl PfCursor for SilentCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }
}

/// A cursor that answers both attestation methods from its `declares` script.
struct AttestingCursor {
    row: Option<PfRow>,
    declares: Declares,
}

impl PfCursor for AttestingCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        Ok(self.row.take())
    }

    fn generation(&self) -> IndexGeneration {
        match self.declares {
            Declares::Generation(value) | Declares::GenerationThenIncomplete(value, _) => {
                IndexGeneration::declared(value)
            }
            Declares::PanicOnGeneration => panic!("the-generation-panic-payload"),
            Declares::Nothing
            | Declares::UndeclaredThenIncomplete(_)
            | Declares::PanicOnServiceLevel => IndexGeneration::Undeclared,
        }
    }

    fn service_level(&self) -> ServiceLevel {
        match self.declares {
            Declares::GenerationThenIncomplete(_, reason)
            | Declares::UndeclaredThenIncomplete(reason) => ServiceLevel::Incomplete {
                reason: reason.to_owned(),
            },
            Declares::PanicOnServiceLevel => panic!("the-service-level-panic-payload"),
            Declares::Nothing | Declares::Generation(_) | Declares::PanicOnGeneration => {
                ServiceLevel::Undeclared
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Scaffolding
// ---------------------------------------------------------------------------

/// A registry holding one relation under [`REL_IRI`].
fn registry(mode_code: &str, declares: Declares) -> ExtensionEnv {
    let mut registry = PropertyFunctionRegistry::new();
    registry.register(
        REL_IRI.to_owned(),
        Arc::new(AttestingRelation::new(mode_code, declares)),
    );
    ExtensionEnv::over_relations(registry).expect("the fixture declarations read cleanly")
}

/// Three ordinary triples, so a driving pattern has three rows to lateral over.
fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    for local in ["ada", "brian", "chen"] {
        let s = builder.intern_iri(&format!("{EX}{local}"));
        let o = builder.intern_iri(&format!("{EX}team"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze fixture")
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn with_relations(env: &ExtensionEnv) -> QueryOptions<'_> {
    QueryOptions::new().with_env(env)
}

/// Drive `query` through the governed entry `query_prepared_governed_view` under
/// `QueryGovernors::UNBOUNDED` — the lane whose outcome carries a witness.
fn governed(engine: &NativeSparqlEngine, env: &ExtensionEnv, query: &str) -> GovernedOutcome {
    let dataset = dataset();
    let prepared = engine
        .prepare_query_with_options(query, None, with_relations(env))
        .expect("the query prepares against the registry");
    engine
        .query_prepared_governed_view(
            &*dataset,
            &prepared,
            &[],
            with_relations(env),
            &QueryGovernors::UNBOUNDED,
        )
        .expect("a governed run of a valid query is an outcome, never an error")
}

/// Drive `query` through the UNGOVERNED entry, whose result type has no witness slot.
fn ungoverned(
    engine: &NativeSparqlEngine,
    env: &ExtensionEnv,
    query: &str,
) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    let dataset = dataset();
    engine.query_with_options_view(&*dataset, request(query), with_relations(env))
}

/// Suppress the default panic-hook stderr dump for an EXPECTED, caught panic.
fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = body();
    std::panic::set_hook(default_hook);
    out
}

fn declared(value: &str) -> BTreeSet<IndexGeneration> {
    BTreeSet::from([IndexGeneration::declared(value)])
}

fn row_count(result: &SparqlResult) -> usize {
    match result {
        SparqlResult::Solutions { rows, .. } => rows.len(),
        other => panic!("a SELECT returns solutions, got {other:?}"),
    }
}

/// A lone call, driven by the identity table: exactly one invocation.
const ONE_CALL: &str = "SELECT ?p ?t WHERE { ?p <https://example.org/rel/memberOf> ?t }";

/// A call correlated with a three-row driving pattern: one invocation per input row.
const PER_ROW_CALL: &str = "PREFIX ex: <https://example.org/d/>\n\
                            SELECT ?s ?t WHERE {\n\
                              ?s ex:p ?o .\n\
                              ?s <https://example.org/rel/memberOf> ?t\n\
                            }";

// ---------------------------------------------------------------------------
// T2.1 — the generation reaches the governed receipt
// ---------------------------------------------------------------------------

/// A cursor declaring `gen-7` is recorded verbatim, once, against the relation's own
/// IRI — on the receipt of the governed entry a host calls.
#[test]
fn a_declared_generation_reaches_the_governed_receipt() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::Generation("gen-7"));
    let outcome = governed(&engine, &relations, ONE_CALL);

    let witness = &outcome.relations().witness;
    let attested = witness
        .get(REL_IRI)
        .expect("the relation this query invoked must appear on the receipt");
    assert_eq!(attested.generations, declared("gen-7"));
    assert_eq!(attested.invocations, 1);
    assert!(
        attested.incompleteness.is_empty(),
        "a relation that declared no incompleteness must contribute none: {:?}",
        attested.incompleteness
    );
}

/// The other governed egress — the borrowed, still-interned one an operation drives per
/// focus node — carries the same witness, filled from the same place.
///
/// It builds its own context and resolves its own verdict, so it is exactly the lane
/// a second copy of the witnessing wiring could have forgotten: a receipt with an
/// empty ledger for a run that invoked a relation. Both halves of the record are
/// asserted — the generation at open, and the incompleteness at close — because the
/// second is the one that turns a hard refusal on the ungoverned lane into a report
/// on this one, and a report nobody filled would be the silent short bag the refusal
/// exists to forbid.
#[test]
fn a_declared_generation_and_incompleteness_reach_the_interned_governed_receipt() {
    let engine = NativeSparqlEngine::new();
    let relations = registry(
        "ff",
        Declares::GenerationThenIncomplete("gen-7", SHARD_REASON),
    );
    let dataset = dataset();
    let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED));
    let request = InternedRequest {
        query: ONE_CALL,
        base_iri: None,
        substitutions: &[],
    };
    let outcome = engine
        .query_governed_interned_in_operation(
            &*dataset,
            request,
            with_relations(&relations),
            &state,
            |interned| match interned {
                InternedOutcome::Solutions(solutions) => solutions.rows().len(),
                other => panic!("a SELECT returns solutions, got {other:?}"),
            },
        )
        .expect("a governed run of a valid query is an outcome, never an error");

    let InternedGoverned::Complete {
        value: rows,
        relations: identity,
        ..
    } = outcome
    else {
        panic!("an unbounded governor never trips");
    };
    assert_eq!(rows, 1, "the relation answered its one row");
    let attested = identity
        .witness
        .get(REL_IRI)
        .expect("the relation this query invoked must appear on the interned receipt");
    assert_eq!(attested.generations, declared("gen-7"));
    assert_eq!(attested.invocations, 1);
    assert_eq!(
        attested.incompleteness,
        BTreeSet::from([SHARD_REASON.to_owned()]),
        "the incompleteness declared at close is reported on this receipt, verbatim"
    );
}

/// Driven over three input rows the same relation is invoked three times and still
/// answered from ONE index — the count and the generation set are independent facts and
/// the record keeps them so.
#[test]
fn three_invocations_count_three_and_still_name_one_generation() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("bf", Declares::Generation("gen-7"));
    let outcome = governed(&engine, &relations, PER_ROW_CALL);

    let GovernedOutcome::Complete { ref result, .. } = outcome else {
        panic!("UNBOUNDED bounds nothing, so this must complete");
    };
    assert_eq!(row_count(result), 3, "one row per driving row");

    let attested = outcome
        .relations()
        .witness
        .get(REL_IRI)
        .expect("the relation attested");
    assert_eq!(attested.invocations, 3);
    assert_eq!(
        attested.generations,
        declared("gen-7"),
        "three invocations of one index are one generation, not three"
    );
}

// ---------------------------------------------------------------------------
// T2.2 — witnessed or fatal
// ---------------------------------------------------------------------------

/// The governed lane has somewhere to put the declaration, so it RETURNS THE ROWS and
/// records the reason beside them. The short bag is not short — it is labelled.
#[test]
fn a_governed_entry_answers_and_records_the_incompleteness() {
    let engine = NativeSparqlEngine::new();
    let relations = registry(
        "ff",
        Declares::GenerationThenIncomplete("gen-7", SHARD_REASON),
    );
    let outcome = governed(&engine, &relations, ONE_CALL);

    let GovernedOutcome::Complete { ref result, .. } = outcome else {
        panic!("an incomplete relation is not a governor trip");
    };
    assert_eq!(
        row_count(result),
        1,
        "the rows the relation DID serve still cross"
    );

    let attested = outcome
        .relations()
        .witness
        .get(REL_IRI)
        .expect("the relation attested");
    assert_eq!(
        attested.incompleteness,
        BTreeSet::from([SHARD_REASON.to_owned()])
    );
    assert_eq!(attested.generations, declared("gen-7"));
}

/// The ungoverned lane returns a bare `SparqlResult`, which has nowhere to carry the
/// declaration — so it refuses, with a code a caller can match on, naming the relation
/// and quoting its reason.
#[test]
fn an_ungoverned_entry_refuses_an_incomplete_relation() {
    let engine = NativeSparqlEngine::new();
    let relations = registry(
        "ff",
        Declares::GenerationThenIncomplete("gen-7", SHARD_REASON),
    );
    let diagnostic = ungoverned(&engine, &relations, ONE_CALL)
        .expect_err("an unlabelled short bag is the one answer this engine may not give");
    assert_eq!(diagnostic.code, EvalError::RELATION_INCOMPLETE_CODE);
    assert!(
        diagnostic.message.contains(REL_IRI),
        "the diagnostic must name the relation: {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains(SHARD_REASON),
        "the diagnostic must quote the relation's own reason: {}",
        diagnostic.message
    );
}

/// The neighbouring VALID case, proving the refusal above is a rule and not a blanket
/// ban: the very same ungoverned entry, the same query, a relation that declares no
/// incompleteness — answered, not refused.
#[test]
fn an_ungoverned_entry_still_answers_a_relation_that_declares_nothing_short() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::Generation("gen-7"));
    let result = ungoverned(&engine, &relations, ONE_CALL)
        .expect("a relation that declared no shortfall must still be answered");
    assert_eq!(row_count(&result), 1);
}

/// The witnessed prepared door hands back the run's witness beside the answer, and —
/// having somewhere to put it — reports a relation's declared shortfall instead of
/// refusing the run. Its unwitnessed twin, [`NativeSparqlEngine::execute`], over the
/// same prepared execution and the same relation, refuses it: the neighbour that
/// shows the report is a property of the return type, not a relaxation.
#[test]
fn the_witnessed_prepared_door_reports_what_the_unwitnessed_one_must_refuse() {
    let engine = NativeSparqlEngine::new();
    let dataset = dataset();
    let count = |outcome: InternedOutcome<'_, '_, RdfDataset>| match outcome {
        InternedOutcome::Solutions(solutions) => solutions.len(),
        InternedOutcome::Boolean(_) | InternedOutcome::Graph(_) => {
            panic!("a SELECT returns solutions")
        }
    };

    for (declares, short) in [
        (Declares::Generation("gen-7"), false),
        (
            Declares::GenerationThenIncomplete("gen-7", SHARD_REASON),
            true,
        ),
    ] {
        let relations = registry("ff", declares);
        let mut execution = engine
            .prepare_execution(ONE_CALL, None, &[], with_relations(&relations))
            .expect("the query prepares against the registry");

        let (rows, witness) = engine
            .execute_witnessed(&mut execution, &*dataset, with_relations(&relations), count)
            .expect("the witnessed door answers, whole or short");
        assert_eq!(rows, 1, "the rows the relation served cross");
        let attested = witness.get(REL_IRI).expect("the relation attested");
        assert_eq!(attested.generations, declared("gen-7"));
        assert_eq!(attested.invocations, 1);
        let expected: BTreeSet<String> = if short {
            BTreeSet::from([SHARD_REASON.to_owned()])
        } else {
            BTreeSet::new()
        };
        assert_eq!(attested.incompleteness, expected);

        let unwitnessed =
            engine.execute(&mut execution, &*dataset, with_relations(&relations), count);
        if short {
            let diagnostic = unwitnessed.expect_err("the unwitnessed door cannot label it");
            assert_eq!(diagnostic.code, EvalError::RELATION_INCOMPLETE_CODE);
        } else {
            assert_eq!(unwitnessed.expect("a whole relation is answered"), 1);
        }
    }
}

/// The shard-found-missing-late case: nothing at open, an incompleteness at close. The
/// declaration is read when the invocation ENDS, so it is recorded — a reading taken only
/// at open would have lost it silently.
#[test]
fn an_incompleteness_discovered_at_close_is_still_recorded() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::UndeclaredThenIncomplete(SHARD_REASON));
    let outcome = governed(&engine, &relations, ONE_CALL);

    let attested = outcome
        .relations()
        .witness
        .get(REL_IRI)
        .expect("the relation attested");
    assert_eq!(
        attested.generations,
        BTreeSet::from([IndexGeneration::Undeclared]),
        "it declared no generation, and silence is recorded as silence"
    );
    assert_eq!(
        attested.incompleteness,
        BTreeSet::from([SHARD_REASON.to_owned()]),
        "and the late declaration still reached the receipt"
    );
}

// ---------------------------------------------------------------------------
// T2.3 — a relation that overrides neither method
// ---------------------------------------------------------------------------

/// A relation written before this channel existed keeps working on BOTH lanes, and says
/// the one true thing about itself: an undeclared generation and no incompleteness.
/// Silence is never read as a certificate of wholeness.
#[test]
fn a_relation_overriding_neither_method_is_undeclared_on_both_lanes() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::Nothing);

    let outcome = governed(&engine, &relations, ONE_CALL);
    let attested = outcome
        .relations()
        .witness
        .get(REL_IRI)
        .expect("a relation that ran appears on the receipt even when it declared nothing");
    assert_eq!(attested.invocations, 1);
    assert_eq!(
        attested.generations,
        BTreeSet::from([IndexGeneration::Undeclared])
    );
    assert!(attested.incompleteness.is_empty());
    assert_eq!(
        PfAttestation::UNDECLARED,
        PfAttestation {
            generation: IndexGeneration::Undeclared,
            service: ServiceLevel::Undeclared,
        },
        "the named both-undeclared value and the assembled one are the same value"
    );

    let result = ungoverned(&engine, &relations, ONE_CALL)
        .expect("the ungoverned lane is untouched by a relation that declares nothing");
    assert_eq!(row_count(&result), 1);
}

// ---------------------------------------------------------------------------
// T2.4 — panic containment on both new reads
// ---------------------------------------------------------------------------

/// A cursor that panics while reporting its generation becomes a clean, payload-free
/// host-function error — never an aborted worker, and never a message whose text depends
/// on what the host's `panic!` happened to say.
///
/// Driven through the GOVERNED lane, because that is the lane that asks: an entry point
/// with no witness slot never reads a generation at all (the test below pins that), so it
/// is not the place to check what happens when the read panics.
#[test]
fn a_panicking_generation_is_contained_payload_free() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::PanicOnGeneration);
    let prepared = engine
        .prepare_query_with_options(ONE_CALL, None, with_relations(&relations))
        .expect("the query prepares against the registry");
    let dataset = dataset();
    let diagnostic = without_panic_output(|| {
        engine
            .query_prepared_governed_view(
                &*dataset,
                &prepared,
                &[],
                with_relations(&relations),
                &QueryGovernors::UNBOUNDED,
            )
            .expect_err("a panicking read must not escape")
    });
    assert!(
        diagnostic
            .message
            .contains("panicked while reporting its index generation"),
        "got {}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains(REL_IRI),
        "the message must name the relation: {}",
        diagnostic.message
    );
    assert!(
        !diagnostic.message.contains("the-generation-panic-payload"),
        "the panic payload must never be interpolated: {}",
        diagnostic.message
    );
}

/// An entry point with nowhere to carry a witness never asks a cursor for its generation.
///
/// The fixture proves it by being unable to answer the question quietly: its
/// `generation()` panics. The ungoverned run ANSWERS, so the method was not called — the
/// one observation that distinguishes "asked and the value was thrown away" from "never
/// asked", without instrumenting the engine.
#[test]
fn an_ungoverned_entry_never_asks_for_a_generation_it_cannot_carry() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::PanicOnGeneration);
    let result = ungoverned(&engine, &relations, PER_ROW_CALL)
        .expect("a lane that cannot report a generation has no reason to read one");
    assert_eq!(
        row_count(&result),
        3,
        "one row per driving row, so the relation really was invoked three times and \
         would have panicked three times had the generation been read"
    );
}

/// The refusal that lane DOES owe still fires, and it fires on the same fixture shape
/// whose generation is never read: the service level is read on every lane, because an
/// unwitnessed short bag has to be refused rather than labelled.
#[test]
fn an_ungoverned_entry_still_refuses_an_incompleteness_it_cannot_label() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::UndeclaredThenIncomplete(SHARD_REASON));
    let diagnostic = ungoverned(&engine, &relations, PER_ROW_CALL)
        .expect_err("an unlabelled short bag is the one answer this engine may not give");
    assert_eq!(diagnostic.code, EvalError::RELATION_INCOMPLETE_CODE);
    assert!(
        diagnostic.message.contains(SHARD_REASON),
        "the diagnostic must quote the relation's own reason: {}",
        diagnostic.message
    );

    // The neighbouring VALID case over the same lane and the same query: a relation that
    // declares nothing short is answered, so the refusal is a rule about incompleteness
    // and not a lane that fails whenever a relation declares anything at all.
    let quiet = registry("ff", Declares::Generation("gen-7"));
    let result = ungoverned(&engine, &quiet, PER_ROW_CALL)
        .expect("a relation that declared no shortfall must still be answered");
    assert_eq!(row_count(&result), 3);
}

/// And the governed lane still records what the ungoverned one skips — the control that
/// makes the skip a lane property rather than a feature that was switched off.
#[test]
fn the_governed_lane_still_records_the_generation_the_other_lane_skips() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("bf", Declares::Generation("gen-7"));
    let attested = governed(&engine, &relations, PER_ROW_CALL)
        .relations()
        .witness
        .get(REL_IRI)
        .expect("the witnessed lane records every invocation that entered host code")
        .clone();
    assert_eq!(attested.generations, declared("gen-7"));
    assert_eq!(attested.invocations, 3);
}

/// The same containment on the other new read, at the other instant.
#[test]
fn a_panicking_service_level_is_contained_payload_free() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::PanicOnServiceLevel);
    let diagnostic = without_panic_output(|| {
        ungoverned(&engine, &relations, ONE_CALL).expect_err("a panicking read must not escape")
    });
    assert!(
        diagnostic
            .message
            .contains("panicked while reporting its service level"),
        "got {}",
        diagnostic.message
    );
    assert!(
        !diagnostic
            .message
            .contains("the-service-level-panic-payload"),
        "the panic payload must never be interpolated: {}",
        diagnostic.message
    );
}

// ---------------------------------------------------------------------------
// T2.5 — determinism, including across the fork
// ---------------------------------------------------------------------------

/// A `UNION` whose left branch is a call: `eval_union` forks a child context per branch
/// under an ungoverned-in-effect budget, so the relation is invoked ON A WORKER and its
/// attestation only reaches the receipt because the join site merges the child's witness
/// back.
const UNION_CALL: &str = "PREFIX ex: <https://example.org/d/>\n\
                          SELECT ?a ?b WHERE {\n\
                            { ?a <https://example.org/rel/memberOf> ?b }\n\
                            UNION\n\
                            { ?a ex:p ?b }\n\
                          }";

/// Every DECLARATION a run records is a function of what was attested and of nothing
/// else: the same query declares the same things on repeated runs, and the
/// forced-sequential engine and the fork-join one record the same declarations.
///
/// The comparison is over the declaration sets rather than over the whole ledger on
/// purpose. `RelationAttestations::invocations` is a fact about the schedule — the field's
/// own docs name the lane where it moves with the chunk count — so an assertion that two
/// engines agree on it would be pinning a scheduling coincidence, and would fail the day
/// the row loop chunked differently without anything about the index having changed.
#[test]
fn the_declarations_are_identical_across_runs_and_across_the_fork() {
    let relations = registry("ff", Declares::GenerationThenIncomplete("gen-7", "partial"));

    let parallel_engine = NativeSparqlEngine::new();
    let sequential_engine = NativeSparqlEngine::new().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    });

    /// What one run declared, per relation, with the schedule-dependent count left out.
    fn declarations(
        engine: &NativeSparqlEngine,
        env: &ExtensionEnv,
    ) -> Vec<(String, BTreeSet<IndexGeneration>, BTreeSet<String>)> {
        governed(engine, env, UNION_CALL)
            .relations()
            .witness
            .iter()
            .map(|(iri, attested)| {
                (
                    iri.to_owned(),
                    attested.generations.clone(),
                    attested.incompleteness.clone(),
                )
            })
            .collect()
    }

    let first = declarations(&parallel_engine, &relations);
    assert_eq!(
        first,
        vec![(
            REL_IRI.to_owned(),
            declared("gen-7"),
            BTreeSet::from(["partial".to_owned()])
        )],
        "the forked branch's attestation must reach the parent's receipt, so the record is \
         not vacuously equal below"
    );
    assert_eq!(
        first,
        declarations(&parallel_engine, &relations),
        "a second run of the same query over the same data must declare identically"
    );
    assert_eq!(
        first,
        declarations(&parallel_engine, &relations),
        "and a third"
    );
    assert_eq!(
        first,
        declarations(&sequential_engine, &relations),
        "the fork-join path and the forced-sequential path must record the same thing: a \
         worker's attestation that died with the worker would show up exactly here"
    );
}

/// The record distinguishes what it has to: two runs that attested DIFFERENT generations
/// are different records, or the comparison above would pass for the wrong reason.
#[test]
fn the_record_separates_two_different_generations() {
    let engine = NativeSparqlEngine::new();
    let generations_of = |declares| {
        governed(&engine, &registry("ff", declares), ONE_CALL)
            .relations()
            .witness
            .get(REL_IRI)
            .expect("the relation attested")
            .generations
            .clone()
    };
    assert_ne!(
        generations_of(Declares::Generation("gen-7")),
        generations_of(Declares::Generation("gen-8")),
        "a rebuild between two otherwise identical queries must be visible in the record"
    );
}

/// A query that invoked no relation carries an EMPTY witness — present, not absent, and
/// never a claim that some index was whole.
#[test]
fn a_query_that_invokes_no_relation_carries_an_empty_witness() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::Generation("gen-7"));
    let outcome = governed(
        &engine,
        &relations,
        "PREFIX ex: <https://example.org/d/>\nSELECT ?s WHERE { ?s ex:p ?o }",
    );
    let identity = outcome.relations();
    assert!(
        identity.witness.is_empty(),
        "no relation was invoked, so nothing attested"
    );
    assert!(
        !identity.is_empty(),
        "but the registry was in scope, and the identity says so — the two emptinesses \
         are independent facts"
    );
    assert_eq!(identity.witness, RelationWitness::default());
}

// ---------------------------------------------------------------------------
// T2.6 — an UPDATE reaches a relation, and refuses without writing
// ---------------------------------------------------------------------------

/// The `INSERT … WHERE` used by both update tests: its `WHERE` is a triple-pattern
/// context, so a registered relation's predicate dispatches a call there exactly as it
/// would in a `SELECT`.
const INSERT_FROM_RELATION: &str = "PREFIX ex: <https://example.org/d/>\n\
                                    INSERT { ?p ex:member ?t }\n\
                                    WHERE { ?p <https://example.org/rel/memberOf> ?t }";

/// How many `ex:member` triples the store holds.
fn member_count(dataset: &Arc<RdfDataset>) -> usize {
    let result = NativeSparqlEngine::new()
        .query(
            dataset,
            request("PREFIX ex: <https://example.org/d/>\nSELECT ?p ?t WHERE { ?p ex:member ?t }"),
        )
        .expect("the probe query evaluates");
    row_count(&result)
}

/// The UPDATE lane's outcome types have no slot for evidence about a relation, so an
/// incomplete relation refuses the request — and the store is untouched, because a
/// half-applied mutation is not an incomplete answer, it is a corrupt store.
#[test]
fn an_update_over_an_incomplete_relation_refuses_and_writes_nothing() {
    let engine = NativeSparqlEngine::new();
    let relations = registry(
        "ff",
        Declares::GenerationThenIncomplete("gen-7", SHARD_REASON),
    );
    let mut dataset = dataset();
    let before = member_count(&dataset);
    assert_eq!(before, 0, "the fixture starts with no ex:member triple");

    let diagnostic = engine
        .update_with_options(
            &mut dataset,
            request(INSERT_FROM_RELATION),
            with_relations(&relations),
        )
        .expect_err("an UPDATE has nowhere to carry the declaration, so it refuses");
    assert_eq!(diagnostic.code, EvalError::RELATION_INCOMPLETE_CODE);
    assert!(
        diagnostic.message.contains(SHARD_REASON),
        "the diagnostic quotes the relation's own reason: {}",
        diagnostic.message
    );
    assert_eq!(
        member_count(&dataset),
        0,
        "a refused UPDATE writes nothing at all"
    );
}

/// The neighbouring VALID case: the SAME update, over a relation that declares no
/// shortfall, commits normally. The refusal above is about the declaration, not about
/// relations in updates.
#[test]
fn the_same_update_over_a_relation_declaring_nothing_short_commits() {
    let engine = NativeSparqlEngine::new();
    let relations = registry("ff", Declares::Generation("gen-7"));
    let mut dataset = dataset();

    engine
        .update_with_options(
            &mut dataset,
            request(INSERT_FROM_RELATION),
            with_relations(&relations),
        )
        .expect("a relation that declared no shortfall must let the mutation through");
    assert_eq!(
        member_count(&dataset),
        1,
        "the relation's one row became one inserted triple"
    );
}

/// The row-loop fork lane, which is a different join site from the `UNION` branch fork
/// above: `crate::expr::eval_filter` forks one child per CHUNK of driving rows, and a
/// `FILTER EXISTS` re-enters pattern evaluation — so the relation inside the `EXISTS` is
/// invoked on a worker, with its attestation landing on that worker's context.
///
/// Above the parallel threshold, so the fork gate actually engages rather than taking the
/// sequential fallback every small input takes.
const PARALLEL_ROW_FLOOR: usize = 1024;

/// How many driving rows [`wide_dataset`] carries: clear of [`PARALLEL_ROW_FLOOR`], and
/// not a multiple of any plausible chunk size, so a count that tracked chunks rather than
/// rows could not coincide with it.
const WIDE_ROWS: u64 = PARALLEL_ROW_FLOOR as u64 + 100;

/// A dataset with more driving rows than the parallel threshold, so the row-loop fork is
/// genuinely reached.
fn wide_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let o = builder.intern_iri(&format!("{EX}team"));
    for index in 0..WIDE_ROWS {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("freeze wide fixture")
}

/// A relation reached from inside a `FILTER EXISTS`, over enough rows to fork.
const FILTER_EXISTS_CALL: &str = "PREFIX ex: <https://example.org/d/>\n\
                                  SELECT ?s WHERE {\n\
                                    ?s ex:p ?o .\n\
                                    FILTER EXISTS { ?a <https://example.org/rel/memberOf> ?b }\n\
                                  }";

/// An attestation collected on a per-chunk row-loop worker reaches the parent's receipt.
///
/// The declarations are asserted, and so is the COUNT — exactly, at [`WIDE_ROWS`]. That is
/// not a scheduling coincidence on this query: a `FILTER EXISTS` re-enters pattern
/// evaluation once per driving row, so every row invokes the relation once and the total is
/// the row count however the rows were split. The chunk boundaries decide which worker's
/// ledger each invocation lands in and nothing else, so the parent's count after the join
/// is the sum over the workers, which is the same number the forced-sequential lane reaches
/// in one ledger. Both lanes are driven here and both are asserted against that number.
///
/// The exact count is the whole point of the test. A join that kept one worker's ledger and
/// dropped the rest — the failure this lane exists to catch — leaves every DECLARATION
/// matching, because a union with one identical member is that member; it shows up only as
/// a count short of the rows. An inequality (`>= 1`, or "more than the sequential lane")
/// would pass for a receipt that lost all but one chunk.
#[test]
fn an_attestation_collected_on_a_row_loop_worker_reaches_the_receipt() {
    /// Drive `FILTER_EXISTS_CALL` over the wide fixture on `engine`, returning what the
    /// relation attested.
    fn attested_over_wide_rows(
        engine: &NativeSparqlEngine,
        env: &ExtensionEnv,
    ) -> RelationAttestations {
        let dataset = wide_dataset();
        let prepared = engine
            .prepare_query_with_options(FILTER_EXISTS_CALL, None, with_relations(env))
            .expect("the query prepares against the registry");
        let outcome = engine
            .query_prepared_governed_view(
                &*dataset,
                &prepared,
                &[],
                with_relations(env),
                &QueryGovernors::UNBOUNDED,
            )
            .expect("a governed run of a valid query is an outcome, never an error");
        outcome
            .relations()
            .witness
            .get(REL_IRI)
            .expect("a worker's attestation that died with the worker would be missing here")
            .clone()
    }

    let relations = registry("ff", Declares::Generation("gen-7"));
    let attested = attested_over_wide_rows(&NativeSparqlEngine::new(), &relations);
    assert_eq!(attested.generations, declared("gen-7"));

    let sequential = attested_over_wide_rows(
        &NativeSparqlEngine::new().with_eval_options(EvalOptions {
            force_sequential: true,
            ..EvalOptions::default()
        }),
        &relations,
    );
    assert_eq!(
        sequential.invocations, WIDE_ROWS,
        "one ledger, one invocation per driving row: the number every chunk's share of the \
         run has to add back up to"
    );
    assert_eq!(
        attested.invocations, WIDE_ROWS,
        "and the fork-join lane's count after the join is that same total, so no worker's \
         share of it was dropped on the way to the parent"
    );
    assert_eq!(
        attested.generations, sequential.generations,
        "with the same declarations on both lanes, so the count above is invocations of \
         one index rather than a second index answering"
    );
}

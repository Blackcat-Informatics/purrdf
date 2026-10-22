// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit large-source qualification of native affected-part maintenance work.

use purrdf_datalog::{
    clause::{ClauseAtom, ClauseTerm, DlClause},
    dictionary::DictionaryTerm,
    guard::NoGuards,
    maintenance::ChangeBatch,
    seminaive::{EvalOptions, Maintained, ResumeProgress, RoundExecution, SessionContext, compile},
    store::{RelationStore, TermInterner},
};
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::Resident;

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

fn one_fact_updates_visit_only_the_affected_star_paths() {
    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::triple(
            ClauseTerm::var(s),
            ClauseTerm::literal(p),
            ClauseTerm::var(o),
        )
    };
    let program = compile(vec![
        DlClause::datalog(atom("x", "path", "y"), vec![atom("x", "edge", "y")]),
        DlClause::datalog(
            atom("x", "path", "z"),
            vec![atom("x", "path", "y"), atom("y", "edge", "z")],
        ),
    ])
    .unwrap();
    let options = EvalOptions::default()
        .with_round_execution(RoundExecution::Sequential)
        .with_max_stored_facts(10_000_000)
        .with_max_join_steps(u64::MAX);
    let mut observations = Vec::new();
    for edges in [1_000_usize, 1_000_000] {
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for node in 1..=edges {
            input
                .insert_row(["root", "edge", &format!("n{node:08}")])
                .unwrap();
        }
        let mut model = Maintained::<u64, Resident, _, 3>::new(
            &program,
            input,
            &NoGuards,
            SessionContext {
                snapshot: Digest32::new([0xb1; 32]),
                guards: Digest32::new([0xb2; 32]),
                evidence: Digest32::new([0xb3; 32]),
            },
            options,
        )
        .unwrap();
        assert!(matches!(
            model.resume(options, None),
            Ok(ResumeProgress::Complete)
        ));
        assert_eq!(model.facts().unwrap().row_count(), edges * 2);
        let sample =
            ["root", "path", "n00000002"].map(|s| model.facts().unwrap().term_id(s).unwrap());
        let surviving = model
            .facts()
            .unwrap()
            .rows_from(0)
            .find(|(_, row)| *row == sample)
            .unwrap()
            .0;
        let insert = model
            .admit_terms(["n00000001", "edge", "new-node"].map(DictionaryTerm::Surface))
            .unwrap();
        let remove =
            ["root", "edge", "n00000001"].map(|s| model.facts().unwrap().term_id(s).unwrap());
        let mut work = Vec::new();
        for (position, (row, weight)) in [(insert, 1), (remove, -1)].into_iter().enumerate() {
            let before = model.budget().join_steps();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let batch = ChangeBatch::new(position as u64, [(row, weight)], Resident).unwrap();
            model.apply(batch).unwrap();
            assert!(matches!(
                model.resume(options, None),
                Ok(ResumeProgress::Complete)
            ));
            let memory = window.close();
            let steps = model.budget().join_steps() - before;
            let delta = model.model_delta().unwrap().unwrap();
            assert_eq!(delta.changes().len(), 3);
            assert!(delta.changes().iter().all(|(_, actual)| *actual == weight));
            println!(
                "edges={edges} update={position} join_steps={steps} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
                memory.allocations,
                memory.requested_bytes,
                memory.retained_bytes,
                memory.peak_working_bytes
            );
            assert!(
                steps <= 128,
                "one affected branch must not enumerate the unrelated star: edges={edges}, steps={steps}"
            );
            work.push((steps, memory));
        }
        assert_eq!(model.facts().unwrap().row_count(), edges * 2);
        assert_eq!(
            model
                .facts()
                .unwrap()
                .rows_from(0)
                .find(|(_, row)| *row == sample)
                .unwrap()
                .0,
            surviving
        );
        observations.push(work);
    }
    assert_eq!(
        observations[0], observations[1],
        "native work and all four allocation quantities are independent of the unrelated source prefix"
    );
}

fn repeated_retractions_release_row_metadata_without_reusing_coordinates() {
    let atom = |p: &str| {
        ClauseAtom::triple(
            ClauseTerm::var("x"),
            ClauseTerm::literal(p),
            ClauseTerm::var("y"),
        )
    };
    let program = compile(vec![DlClause::datalog(atom("out"), vec![atom("edge")])]).unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    input.insert_row(["a", "edge", "b"]).unwrap();
    let ids = ["a", "edge", "b"].map(|s| input.term_id(s).unwrap());
    input.remove_ids(ids).unwrap();
    let options = EvalOptions::default().with_round_execution(RoundExecution::Sequential);
    let mut model = Maintained::<u64, Resident, _, 3>::new(
        &program,
        input,
        &NoGuards,
        SessionContext {
            snapshot: Digest32::new([0xd1; 32]),
            guards: Digest32::new([0xd2; 32]),
            evidence: Digest32::new([0xd3; 32]),
        },
        options,
    )
    .unwrap();
    assert!(matches!(
        model.resume(options, None),
        Ok(ResumeProgress::Complete)
    ));
    let mut position = 0_u64;
    let mut cycle = |model: &mut Maintained<'_, u64, Resident, TermInterner, 3>| {
        for weight in [1, -1] {
            model
                .apply(ChangeBatch::new(position, [(ids, weight)], Resident).unwrap())
                .unwrap();
            assert!(matches!(
                model.resume(options, None),
                Ok(ResumeProgress::Complete)
            ));
            position += 1;
        }
        assert_eq!(model.facts().unwrap().row_count(), 0);
        assert_eq!(
            model.budget().stored_facts(),
            0,
            "retired metadata is released, rather than quota-refused or remapped"
        );
    };
    for _ in 0..1024 {
        cycle(&mut model);
    }
    for block in 0..4 {
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        for _ in 0..1024 {
            cycle(&mut model);
        }
        let memory = window.close();
        println!(
            "churn_block={block} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
            memory.allocations,
            memory.requested_bytes,
            memory.retained_bytes,
            memory.peak_working_bytes
        );
        assert_eq!(
            memory.retained_bytes, 0,
            "fixed-vocabulary churn must not retain historical row metadata"
        );
    }
    assert_eq!(model.facts().unwrap().row_slots(), 10_241);
}

fn hot_partition_compaction_retains_large_original_buffers_across_work_refusal() {
    use purrdf_datalog::seminaive::MaintenanceError;
    let program = compile(Vec::<DlClause<3>>::new()).unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    // The original merge law admits a batch up to twice the incoming span;
    // its full-consolidation boundaries are Fibonacci chunk counts. The next
    // tail at 17,711 chunks merges all 1,133,440 preceding original rows.
    let prefix = 17_711_usize * 64 - 64;
    for index in 0..prefix {
        input
            .insert_row([&format!("s{index:08}"), "hot", "o"])
            .unwrap();
    }
    let original = input.rows_from(0).next().unwrap();
    let options = EvalOptions::default()
        .with_max_stored_facts(u64::MAX)
        .with_max_join_steps(u64::MAX);
    let context = SessionContext {
        snapshot: Digest32::new([0xc1; 32]),
        guards: Digest32::new([0xc2; 32]),
        evidence: Digest32::new([0xc3; 32]),
    };
    let mut observations = Vec::new();
    for paused in [false, true] {
        let mut model = Maintained::<u64, Resident, _, 3>::new(
            &program,
            input.clone(),
            &NoGuards,
            context,
            options,
        )
        .unwrap();
        assert!(matches!(
            model.resume(options, None),
            Ok(ResumeProgress::Complete)
        ));
        let mut final_row = None;
        for offset in 0..64 {
            let row = model
                .admit_terms([
                    DictionaryTerm::Surface(&format!("s{:08}", prefix + offset)),
                    DictionaryTerm::Surface("hot"),
                    DictionaryTerm::Surface("o"),
                ])
                .unwrap();
            model
                .apply(ChangeBatch::new(offset as u64, [(row, 1)], Resident).unwrap())
                .unwrap();
            if offset == 63 {
                final_row = Some(row);
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let mut limit = if paused { 0 } else { u128::MAX };
                let mut cuts = 0;
                loop {
                    match model.resume_until(options, limit, None) {
                        Ok(ResumeProgress::Complete) => break,
                        Err(MaintenanceError::WorkLimit { needed, .. }) => {
                            assert!(paused);
                            assert_eq!(model.pending_watermark(), Some(&63));
                            assert_eq!(model.applied_watermark(), Some(&62));
                            assert!(model.facts().is_err());
                            limit = needed + 1_000_000;
                            cuts += 1;
                        }
                        other => panic!("large original compaction: {other:?}"),
                    }
                }
                let memory = window.close();
                println!(
                    "hot_rows={} paused={paused} cuts={cuts} work={} allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
                    prefix + 64,
                    model.transaction_work(),
                    memory.allocations,
                    memory.requested_bytes,
                    memory.retained_bytes,
                    memory.peak_working_bytes
                );
                assert!(!paused || cuts > 4, "refusals span actual large merge work");
                observations.push((model.transaction_work(), memory));
            } else {
                assert!(matches!(
                    model.resume(options, None),
                    Ok(ResumeProgress::Complete)
                ));
            }
        }
        assert_eq!(model.facts().unwrap().row_count(), prefix + 64);
        assert_eq!(model.facts().unwrap().rows_from(0).next(), Some(original));
        assert_eq!(
            model.model_delta().unwrap().unwrap().changes(),
            &[(final_row.unwrap(), 1)]
        );
        assert_eq!(model.budget().join_steps(), 0);
    }
    assert_eq!(
        observations[0], observations[1],
        "pausing repeats no compaction work or physical factory"
    );
}

fn main() -> std::process::ExitCode {
    use purrdf_testkit::harness::{self, Trial};
    harness::main([
        Trial::test(
            "hot_partition_compaction_retains_large_original_buffers_across_work_refusal",
            || {
                hot_partition_compaction_retains_large_original_buffers_across_work_refusal();
                Ok(())
            },
        )
        .with_ignored_flag(true),
        Trial::test(
            "one_fact_updates_visit_only_the_affected_star_paths",
            || {
                one_fact_updates_visit_only_the_affected_star_paths();
                Ok(())
            },
        )
        .with_ignored_flag(true),
        Trial::test(
            "repeated_retractions_release_row_metadata_without_reusing_coordinates",
            || {
                repeated_retractions_release_row_metadata_without_reusing_coordinates();
                Ok(())
            },
        )
        .with_ignored_flag(true),
    ])
}

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

fn main() -> std::process::ExitCode {
    use purrdf_testkit::harness::{self, Trial};
    harness::main([
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

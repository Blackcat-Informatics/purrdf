// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native fixed-work proof preparation and extraction measurements.
//!
//! Run `proof_extraction_work measure NEW_DIRECTORY`. Each sample runs in a fresh
//! perf worker. Fixture evaluation is excluded. Preparation retains the complete
//! eager arena or the lazy index; extraction reuses that prepared object and
//! retains all requested output arenas. Peaks and retention describe each phase
//! separately, not whole-query ownership. Verification and destruction are excluded.
//! Raw perf coverage and worker allocation/work records remain in the directory.
//! Fixture evaluation uses the sequential policy so no background Rayon worker
//! contributes unrelated instructions to the inherited counter window.

#[cfg(target_os = "linux")]
mod native {
    use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow, Measurement};
    use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
    use purrdf_datalog::guard::NoGuards;
    use purrdf_datalog::id::ProofId;
    use purrdf_datalog::proof::{
        EvaluationProofIndex, EvaluationProofs, ProofArena, ProofContext, ProofIndexWork,
    };
    use purrdf_datalog::seminaive::{
        DerivationMode, EvalOptions, RoundExecution, compile, evaluate_guarded,
    };
    use purrdf_datalog::store::{Fact, RelationStore};
    use purrdf_testkit::bench::counter::{CounterCommand, CounterControl, collect_instructions};
    use std::error::Error;
    use std::ffi::OsString;
    use std::path::Path;

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    type Result<T> = std::result::Result<T, Box<dyn Error>>;
    type Proof = (ProofArena, ProofId);
    const DEPTH: usize = 8;

    fn atom(level: usize) -> ClauseAtom {
        ClauseAtom::positive(
            ClauseTerm::var("?s"),
            format!("https://example.org/p{level}"),
            ClauseTerm::var("?o"),
        )
    }

    fn target(branch: usize) -> Fact {
        Fact {
            subject: format!("<https://example.org/s{branch}>"),
            predicate: format!("<https://example.org/p{DEPTH}>"),
            object: "<https://example.org/o>".to_owned(),
            graph: RelationStore::DEFAULT_GRAPH.to_owned(),
        }
    }

    fn measured<T>(
        counter: &mut CounterControl,
        run: impl FnOnce() -> Result<T>,
    ) -> Result<(T, Measurement)> {
        counter.command(CounterCommand::Enable)?;
        let window = CurrentThreadWindow::open();
        let value = run()?;
        std::hint::black_box(&value);
        let allocation = window.close();
        counter.command(CounterCommand::Disable)?;
        Ok((value, allocation))
    }

    fn eager_extract(proofs: &EvaluationProofs, goal: &Fact) -> Result<Proof> {
        let root = proofs
            .root_for(goal)
            .ok_or("fixture goal has no eager proof")?;
        Ok(ProofArena::decode(&proofs.arena().encode(root))?)
    }

    fn report(allocation: Measurement, work: Option<ProofIndexWork>) {
        println!(
            "allocations={} requested_bytes={} retained_bytes={} peak_working_bytes={}",
            allocation.allocations,
            allocation.requested_bytes,
            allocation.retained_bytes,
            allocation.peak_working_bytes
        );
        if let Some(work) = work {
            println!(
                "derivations={} source_slots={} index_comparisons={}",
                work.derivations, work.source_slots, work.index_comparisons
            );
        }
    }

    fn verify(
        outputs: &[Proof],
        goals: &[Fact],
        eager: &EvaluationProofs,
        context: &ProofContext<'_>,
    ) -> Result<()> {
        for ((arena, root), goal) in outputs.iter().zip(goals) {
            if arena.check(*root, context)? != *goal {
                return Err("independent proof answer mismatch".into());
            }
            let expected = eager.root_for(goal).ok_or("missing oracle goal")?;
            if arena.encode(*root) != eager.arena().encode(expected) {
                return Err("canonical proof bytes differ".into());
            }
        }
        println!("verified_targets={}", outputs.len());
        Ok(())
    }

    pub(super) fn worker(phase: &str, width: usize, count: usize) -> Result<()> {
        if count == 0 || count > width {
            return Err("target count must be between one and fixture width".into());
        }
        let program: Vec<_> = (0..DEPTH)
            .map(|level| DlClause::datalog(atom(level + 1), vec![atom(level)]))
            .collect();
        let mut edb = RelationStore::new();
        for branch in 0..width {
            let goal = target(branch);
            edb.insert(
                &goal.subject,
                "<https://example.org/p0>",
                &goal.object,
                &goal.graph,
            )?;
        }
        let executable = compile(program.clone())?;
        let evaluation = evaluate_guarded(
            &executable,
            edb.clone(),
            &NoGuards,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_round_execution(RoundExecution::Sequential),
            None,
        )?;
        let context = ProofContext::new(&program, &edb, evaluation.facts());
        let goals: Vec<_> = (0..count).map(target).collect();
        let mut counter = CounterControl::required()?;
        match phase {
            "eager-prepare" => {
                let (eager, allocation) =
                    measured(&mut counter, || Ok(EvaluationProofs::of(&evaluation)?))?;
                report(allocation, None);
                let outputs = goals
                    .iter()
                    .map(|goal| eager_extract(&eager, goal))
                    .collect::<Result<Vec<_>>>()?;
                verify(&outputs, &goals, &eager, &context)?;
            }
            "lazy-prepare" => {
                let (index, allocation) =
                    measured(&mut counter, || Ok(EvaluationProofIndex::of(&evaluation)?))?;
                report(allocation, Some(index.preparation_work()));
                let outputs = goals
                    .iter()
                    .map(|goal| Ok(index.extract(goal)?.ok_or("missing lazy goal")?))
                    .collect::<Result<Vec<_>>>()?;
                verify(
                    &outputs,
                    &goals,
                    &EvaluationProofs::of(&evaluation)?,
                    &context,
                )?;
            }
            "eager-extract" => {
                let eager = EvaluationProofs::of(&evaluation)?;
                let (outputs, allocation) = measured(&mut counter, || {
                    goals
                        .iter()
                        .map(|goal| eager_extract(&eager, goal))
                        .collect::<Result<Vec<_>>>()
                })?;
                report(allocation, None);
                verify(&outputs, &goals, &eager, &context)?;
            }
            "lazy-extract" => {
                let index = EvaluationProofIndex::of(&evaluation)?;
                let mut work = ProofIndexWork::default();
                let (outputs, allocation) = measured(&mut counter, || {
                    goals
                        .iter()
                        .map(|goal| {
                            let mut query_work = ProofIndexWork::default();
                            let proof = index
                                .extract_counted(goal, &mut query_work)?
                                .ok_or("missing lazy goal")?;
                            for (total, query) in [
                                (&mut work.derivations, query_work.derivations),
                                (&mut work.source_slots, query_work.source_slots),
                                (&mut work.index_comparisons, query_work.index_comparisons),
                            ] {
                                *total = total.checked_add(query).ok_or("work counter overflow")?;
                            }
                            Ok(proof)
                        })
                        .collect::<Result<Vec<_>>>()
                })?;
                report(allocation, Some(work));
                verify(
                    &outputs,
                    &goals,
                    &EvaluationProofs::of(&evaluation)?,
                    &context,
                )?;
            }
            _ => return Err("unknown measurement phase".into()),
        }
        Ok(())
    }

    pub(super) fn measure(directory: &Path) -> Result<()> {
        std::fs::create_dir(directory)?;
        let executable = std::env::current_exe()?;
        for width in [16, 64, 256] {
            for phase in [
                "eager-prepare",
                "lazy-prepare",
                "eager-extract",
                "lazy-extract",
            ] {
                let counts: &[usize] = if phase.ends_with("prepare") {
                    &[1]
                } else {
                    &[1, 8, 16]
                };
                for count in counts {
                    for repetition in 0..3 {
                        let name =
                            format!("{phase}-width{width}-targets{count}-sample{repetition}");
                        let args = [
                            OsString::from("worker"),
                            phase.into(),
                            width.to_string().into(),
                            count.to_string().into(),
                        ];
                        let sample =
                            collect_instructions(&executable, &args, &directory.join(&name))?;
                        println!(
                            "{name} instructions={} enabled={} running={}",
                            sample.instructions, sample.enabled, sample.running
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, directory] if mode == "measure" => native::measure(std::path::Path::new(directory)),
        [mode, phase, width, count] if mode == "worker" => {
            native::worker(phase, width.parse()?, count.parse()?)
        }
        _ => Err(
            "usage: proof_extraction_work measure NEW_DIRECTORY | worker PHASE WIDTH TARGETS"
                .into(),
        ),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("instruction measurement requires Linux perf".into())
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use purrdf_datalog::maintenance::ChangeBatch;
use purrdf_datalog::seminaive::{MaintenanceError, ResumeProgress, SessionContext};
use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{Admission, Resident, StorageError};

fn context() -> SessionContext {
    SessionContext {
        snapshot: Digest32::new([0x91; 32]),
        guards: Digest32::new([0x92; 32]),
        evidence: Digest32::new([0x93; 32]),
    }
}

#[derive(Debug)]
struct Original(Arc<AtomicUsize>);
impl Admission for Original {
    fn resize(&mut self, _live_bytes: usize) -> Result<(), StorageError> {
        Ok(())
    }
}
impl Drop for Original {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct Grant(Arc<AtomicUsize>);
impl Admission for Grant {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.0.load(Ordering::Relaxed) {
            Err(StorageError::AdmissionFailed)
        } else {
            Ok(())
        }
    }
}

fn updates<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    seed: &[ClauseAtom<N>],
    changes: &[(ClauseAtom<N>, i64)],
) where
    Arity<N>: StoreShape<N>,
{
    let input = stratified::input(seed);
    let prepared = prepare_demanded(rules, goal, input.interner()).unwrap();
    let options = EvalOptions::default();
    let ceiling = Arc::new(AtomicUsize::new(0));
    let mut model = prepared
        .maintain_in::<Box<Original>, Original, _, _>(
            input,
            context(),
            options,
            Grant(ceiling.clone()),
        )
        .unwrap();
    assert!(matches!(
        model.answers(),
        Err(MaintenanceError::InitialIncomplete)
    ));
    assert!(matches!(
        model.resume_until(options, u128::MAX, None),
        Ok(ResumeProgress::Paused(EvalError::Storage(
            StorageError::AdmissionFailed
        )))
    ));
    ceiling.store(usize::MAX, Ordering::Relaxed);
    if goal.predicate().is_var() {
        let paused = model
            .resume_until(options.with_max_generated_terms(0), u128::MAX, None)
            .unwrap();
        assert!(
            matches!(paused, ResumeProgress::Paused(EvalError::TermLimitExceeded { report, .. }) if report.generated_terms() == 1)
        );
    }
    assert!(matches!(
        model.resume_until(options, u128::MAX, None),
        Ok(ResumeProgress::Complete)
    ));
    let mut asserted: BTreeMap<_, i64> = seed
        .iter()
        .map(|atom| (atom.terms().map(|term| term.surface().unwrap()), 1))
        .collect();
    let mut previous = full(rules, goal, stratified::input(seed));
    assert_eq!(
        surfaces(model.dictionary().unwrap(), model.answers().unwrap()),
        previous
    );
    let mut owners: Vec<Arc<AtomicUsize>> = Vec::new();
    for (index, (atom, weight)) in changes.iter().enumerate() {
        let row = atom.terms().map(|term| term.surface().unwrap());
        let ids = model
            .admit_terms(row.each_ref().map(|term| DictionaryTerm::Surface(term)))
            .unwrap();
        let drops = Arc::new(AtomicUsize::new(0));
        let watermark = Box::new(Original(drops.clone()));
        let pointer = &raw const *watermark;
        let batch = ChangeBatch::new(watermark, [(ids, *weight)], Original(drops.clone())).unwrap();
        model.begin(batch).unwrap();
        assert!(matches!(model.answers(), Err(MaintenanceError::Busy)));
        assert!(matches!(model.delta(), Err(MaintenanceError::Busy)));
        assert!(matches!(
            model.resume_until(options, 0, None),
            Err(MaintenanceError::WorkLimit { .. })
        ));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        let stopped = Stopped;
        assert!(matches!(
            model.resume_until(options, u128::MAX, Some(&stopped)),
            Ok(ResumeProgress::Paused(EvalError::Stopped { .. }))
        ));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert!(matches!(
            model.resume_until(options, u128::MAX, None),
            Ok(ResumeProgress::Complete)
        ));
        assert_eq!(&raw const **model.applied_watermark().unwrap(), pointer);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        if index != 0 {
            assert_eq!(owners[index - 1].load(Ordering::Relaxed), 2);
        }
        *asserted.entry(row).or_default() += weight;
        let mut original = Store::with_dictionary(TermInterner::default());
        for (row, count) in &asserted {
            if *count > 0 {
                original
                    .insert_terms(row.each_ref().map(|term| DictionaryTerm::Surface(term)))
                    .unwrap();
            }
        }
        let expected = full(rules, goal, original);
        assert_eq!(
            surfaces(model.dictionary().unwrap(), model.answers().unwrap()),
            expected
        );
        let delta = model.delta().unwrap().unwrap();
        assert_eq!(&raw const **delta.watermark(), pointer);
        let actual: BTreeMap<_, _> = delta
            .changes()
            .iter()
            .map(|(row, weight)| {
                (
                    row.map(|id| {
                        model
                            .dictionary()
                            .unwrap()
                            .resolve_term(id)
                            .unwrap()
                            .to_owned()
                    }),
                    *weight,
                )
            })
            .collect();
        let mut wanted: BTreeMap<_, _> = expected
            .difference(&previous)
            .cloned()
            .map(|row| (row, 1))
            .collect();
        wanted.extend(previous.difference(&expected).cloned().map(|row| (row, -1)));
        assert_eq!(actual, wanted);
        previous = expected;
        owners.push(drops);
    }
    drop(model);
    assert!(
        owners
            .iter()
            .all(|drops| drops.load(Ordering::Relaxed) == 2)
    );
}

#[test]
fn signed_demand_updates_keep_full_model_parity_goal_deltas_and_original_paused_owners() {
    let changes = [
        (atom("0", "edge", "1"), 1),
        (atom("0", "edge", "1"), -1),
        (atom("0", "edge", "1"), -1),
        (atom("0", "edge", "3"), 1),
        (atom("3", "edge", "4"), 1),
        (atom("3", "edge", "4"), -1),
    ];
    updates(
        &transitive(),
        &atom("0", "reach", "?target"),
        &[
            atom("0", "edge", "1"),
            atom("1", "edge", "2"),
            atom("2", "edge", "1"),
            atom("8", "edge", "9"),
        ],
        &changes,
    );
    let changes = changes.map(|(atom, weight)| {
        let terms = atom.terms();
        (
            ClauseAtom::quad(
                terms[0].clone(),
                terms[1].clone(),
                terms[2].clone(),
                term("1"),
            ),
            weight,
        )
    });
    let seed = [
        quad("0", "edge", "1", "1"),
        quad("1", "edge", "2", "1"),
        quad("2", "edge", "1", "1"),
        quad("0", "edge", "8", "other"),
    ];
    for goal in [
        quad("0", "reach", "?target", "?graph"),
        quad("?same", "reach", "?same", "?same"),
    ] {
        updates(&quad_rules(), &goal, &seed, &changes);
    }
}

#[test]
fn reserved_input_rejection_preserves_the_previous_demand_snapshot_and_allows_valid_neighbours() {
    let rules = transitive();
    let goal = atom("0", "reach", "?target");
    let input = store(&[(0, 1)]);
    let prepared = prepare_demanded(&rules, &goal, input.interner()).unwrap();
    let options = EvalOptions::default();
    let mut model = prepared
        .maintain::<usize, Resident, _>(input, context(), options)
        .unwrap();
    assert!(matches!(
        model.resume_until(options, u128::MAX, None),
        Ok(ResumeProgress::Complete)
    ));
    let first = model
        .admit_terms(["<urn:1>", "<urn:edge>", "<urn:2>"].map(DictionaryTerm::Surface))
        .unwrap();
    model
        .begin(ChangeBatch::new(7, [(first, 1)], Resident).unwrap())
        .unwrap();
    assert!(matches!(
        model.resume_until(options, u128::MAX, None),
        Ok(ResumeProgress::Complete)
    ));
    let answers = model.answers().unwrap();
    let delta = model.delta().unwrap().unwrap().changes().to_vec();
    let names = internal_terms(model.dictionary().unwrap());
    assert_ne!(names.len(), 0);
    for name in names {
        let reserved = model
            .dictionary()
            .unwrap()
            .lookup_term(&name)
            .unwrap()
            .unwrap();
        for at in 0..3 {
            let mut row = first;
            row[at] = reserved;
            model
                .begin(ChangeBatch::new(8, [(row, 1)], Resident).unwrap())
                .unwrap();
            assert!(
                matches!(model.resume_until(options, u128::MAX, None), Err(MaintenanceError::ReservedInputTerm { term }) if term == name)
            );
            assert_eq!(model.answers().unwrap(), answers);
            assert_eq!(model.delta().unwrap().unwrap().changes(), delta);
            assert_eq!(model.applied_watermark(), Some(&7));
            let mut terms = [
                DictionaryTerm::Surface("<urn:new>"),
                DictionaryTerm::Surface("<urn:edge>"),
                DictionaryTerm::Surface("<urn:fresh>"),
            ];
            terms[at] = DictionaryTerm::Surface(&name);
            assert!(matches!(
                model.admit_terms(terms),
                Err(MaintenanceError::ReservedInputTerm { .. })
            ));
            terms[at] = DictionaryTerm::Id(reserved);
            assert!(matches!(
                model.admit_terms(terms),
                Err(MaintenanceError::ReservedInputTerm { .. })
            ));
            assert_eq!(
                model
                    .dictionary()
                    .unwrap()
                    .lookup_term("<urn:new>")
                    .unwrap(),
                None
            );
            assert_eq!(
                model
                    .dictionary()
                    .unwrap()
                    .lookup_term("<urn:fresh>")
                    .unwrap(),
                None
            );
        }
    }
    let valid = model
        .admit_terms(["<urn:new>", "<urn:edge>", "<urn:fresh>"].map(DictionaryTerm::Surface))
        .unwrap();
    model
        .begin(ChangeBatch::new(9, [(valid, 1)], Resident).unwrap())
        .unwrap();
    assert!(matches!(
        model.resume_until(options, u128::MAX, None),
        Ok(ResumeProgress::Complete)
    ));
    assert_eq!(model.answers().unwrap(), answers);
    assert_eq!(model.applied_watermark(), Some(&9));

    let negative = vec![DlClause::datalog(
        atom("?x", "out", "?y"),
        vec![
            atom("?x", "edge", "?y"),
            atom("?x", "blocked", "?y").into_negated(),
        ],
    )];
    updates(
        &negative,
        &atom("0", "out", "?y"),
        &[atom("0", "edge", "1")],
        &[
            (atom("0", "blocked", "1"), 1),
            (atom("0", "blocked", "1"), 1),
            (atom("0", "blocked", "1"), -1),
            (atom("0", "blocked", "1"), -1),
            (atom("0", "edge", "1"), -1),
            (atom("0", "edge", "2"), 1),
        ],
    );
    let variable = vec![DlClause::datalog(
        atom("?s", "out", "?o"),
        vec![
            atom("?s", "?p", "?o"),
            atom("?s", "blocked", "?o").into_negated(),
        ],
    )];
    updates(
        &variable,
        &atom("?s", "?p", "?o"),
        &[atom("0", "edge", "1")],
        &[
            (atom("0", "blocked", "1"), 1),
            (atom("0", "blocked", "1"), -1),
            (atom("7", "new", "8"), 1),
            (atom("7", "new", "8"), -1),
            (atom("0", "edge", "1"), -1),
        ],
    );
    let variable = vec![DlClause::datalog(
        quad("?s", "out", "?o", "?g"),
        vec![
            quad("?s", "?p", "?o", "?g"),
            quad("?s", "blocked", "?o", "?g").into_negated(),
        ],
    )];
    updates(
        &variable,
        &quad("?s", "?p", "?o", "?g"),
        &[quad("0", "edge", "1", ""), quad("0", "edge", "1", "named")],
        &[
            (quad("0", "blocked", "1", ""), 1),
            (quad("0", "blocked", "1", "named"), 1),
            (quad("0", "blocked", "1", ""), -1),
            (quad("0", "blocked", "1", "named"), -1),
            (quad("7", "new", "8", ""), 1),
            (quad("7", "new", "8", ""), -1),
            (quad("0", "edge", "1", "named"), -1),
            (quad("0", "edge", "1", ""), -1),
        ],
    );
}

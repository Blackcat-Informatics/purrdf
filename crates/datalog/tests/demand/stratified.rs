// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use purrdf_datalog::guard::Negation;

fn input<const N: usize>(atoms: &[ClauseAtom<N>]) -> Store<N>
where
    Arity<N>: StoreShape<N>,
{
    let mut store = Store::with_dictionary(TermInterner::default());
    for atom in atoms {
        let terms = atom.terms().map(|term| term.surface().unwrap());
        store
            .insert_terms(terms.each_ref().map(|term| DictionaryTerm::Surface(term)))
            .unwrap();
    }
    store
}

fn parity<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    input: Store<N>,
) -> BTreeSet<[String; N]>
where
    Arity<N>: StoreShape<N>,
{
    let expected = full(rules, goal, input.clone());
    let result = evaluate_demanded(rules, goal, input, &EvalOptions::default(), None).unwrap();
    let actual = surfaces(result.dictionary(), result.answers().unwrap());
    assert_eq!(actual, expected, "goal={goal:?}, rules={rules:?}");
    actual
}

#[test]
fn generated_stratified_answers_match_full_triple_and_quad_models() {
    generated(atom);
    generated(|s, p, o| quad(s, p, o, "?g"));
}

fn generated<const N: usize>(atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>)
where
    Arity<N>: StoreShape<N>,
{
    let mut rules = closure::closure_rules("edge", "reach", &atom);
    rules.extend([
        DlClause::datalog(atom("?x", "bypass", "?y"), vec![atom("?x", "edge", "?y")]),
        DlClause::datalog(
            atom("?x", "blocked", "?y"),
            vec![
                atom("?x", "reach", "?y"),
                atom("?x", "bypass", "?y").into_negated(),
            ],
        ),
        DlClause::datalog(
            atom("?x", "accepted", "?y"),
            vec![
                atom("?x", "blocked", "?y").into_negated(),
                atom("?x", "reach", "?y"),
            ],
        ),
        DlClause::datalog(
            atom("?x", "terminal", "?y"),
            vec![atom("?x", "reach", "?y")],
        )
        .with_negations(vec![Negation::new(
            vec![
                atom("?y", "reach", "?local"),
                atom("?local", "blocked", "?y"),
            ],
            Vec::new(),
        )]),
    ]);
    for seed in 0_u64..12 {
        let mut rules = rules.clone();
        let mut predecessor = "blocked".to_owned();
        for level in 0..=seed % 4 {
            let predicate = format!("generated{level}");
            let mut body = vec![
                atom("?x", "reach", "?y"),
                atom("?x", &predecessor, "?y").into_negated(),
            ];
            if seed & 1 == 0 {
                body.reverse();
            }
            rules.push(DlClause::datalog(atom("?x", &predicate, "?y"), body));
            predecessor = predicate;
        }
        let mut draw = seed + 1;
        let mut rows = Vec::new();
        for from in 0..6 {
            for to in 0..6 {
                draw = purrdf_hash::mix::splitmix64_next(&mut draw);
                if draw % 5 == 0 {
                    let mut terms = atom(&from.to_string(), "edge", &to.to_string())
                        .terms()
                        .map(Clone::clone);
                    if N == 4 {
                        terms[3] = ClauseTerm::iri(if draw & 1 == 0 { "urn:g" } else { "urn:h" });
                    }
                    // Constructors remain shape-specific at the public boundary.
                    let surfaces = terms.map(|term| term.surface().unwrap());
                    rows.push(surfaces);
                }
            }
        }
        let mut store = Store::<N>::with_dictionary(TermInterner::default());
        for row in rows {
            store
                .insert_terms(row.each_ref().map(|s| DictionaryTerm::Surface(s)))
                .unwrap();
        }
        for predicate in ["accepted", "terminal", predecessor.as_str()] {
            for goal in [
                atom("0", predicate, "?y"),
                atom("?x", predicate, "2"),
                atom("?same", predicate, "?same"),
            ] {
                parity(&rules, &goal, store.clone());
            }
        }
    }
}

#[test]
fn negative_locals_and_later_binders_do_not_leak_across_conjunctions() {
    let rules = vec![
        DlClause::datalog(atom("?x", "bad", "?y"), vec![atom("?x", "edge", "?y")]),
        DlClause::datalog(
            atom("?x", "chosen", "?y"),
            vec![
                atom("?x", "bad", "?later").into_negated(),
                atom("?x", "candidate", "?y"),
                atom("?y", "bind", "?later"),
            ],
        ),
        DlClause::datalog(
            atom("?x", "grouped", "?y"),
            vec![atom("?x", "candidate", "?y")],
        )
        .with_negations(vec![
            Negation::new(
                vec![atom("?x", "edge", "?local"), atom("?local", "bad", "?y")],
                Vec::new(),
            ),
            Negation::new(
                vec![atom("?y", "edge", "?local"), atom("?local", "bad", "?x")],
                Vec::new(),
            ),
        ]),
        DlClause::datalog(
            atom("?x", "isolated", "?y"),
            vec![
                atom("?x", "candidate", "?y"),
                atom("?y", "bad", "?exists").into_negated(),
            ],
        ),
    ];
    let store = input(&[
        atom("a", "candidate", "b"),
        atom("b", "bind", "safe"),
        atom("a", "edge", "other"),
        atom("b", "edge", "different"),
        atom("different", "edge", "a"),
    ]);
    assert_eq!(
        parity(&rules, &atom("a", "chosen", "?y"), store.clone()),
        BTreeSet::from([["<urn:a>".into(), "<urn:chosen>".into(), "<urn:b>".into()]])
    );
    assert!(parity(&rules, &atom("a", "grouped", "?y"), store.clone()).is_empty());
    assert!(parity(&rules, &atom("a", "isolated", "?y"), store).is_empty());
}

#[test]
fn exact_negative_graphs_keep_default_named_and_repeated_identity() {
    let rules = vec![
        DlClause::datalog(
            quad("?x", "bad", "?y", "?g"),
            vec![quad("?x", "edge", "?y", "?g")],
        ),
        DlClause::datalog(
            quad("?x", "chosen", "?y", "?g"),
            vec![
                quad("?x", "candidate", "?y", "?g"),
                quad("?x", "bad", "?y", "?g").into_negated(),
            ],
        ),
        DlClause::datalog(
            quad("?x", "cross", "?y", "?g"),
            vec![quad("?x", "candidate", "?y", "?g")],
        )
        .with_negations(vec![Negation::new(
            vec![
                quad("?x", "bad", "?local", ""),
                quad("?local", "bad", "?y", "?g"),
            ],
            Vec::new(),
        )]),
    ];
    let store = input(&[
        quad("g", "candidate", "a", "g"),
        quad("g", "candidate", "a", ""),
        quad("g", "edge", "a", ""),
        quad("a", "edge", "a", "g"),
    ]);
    let result = parity(
        &rules,
        &quad("?same", "chosen", "?y", "?same"),
        store.clone(),
    );
    assert_eq!(
        result,
        BTreeSet::from([[
            "<urn:g>".into(),
            "<urn:chosen>".into(),
            "<urn:a>".into(),
            "<urn:g>".into()
        ]])
    );
    assert!(parity(&rules, &quad("?x", "chosen", "?y", ""), store.clone()).is_empty());
    assert!(parity(&rules, &quad("?x", "cross", "?y", "g"), store).is_empty());
}

#[test]
fn original_invalid_rules_refuse_before_source_mutation_and_shadow_seeds_keep_ownership() {
    let mut rules = transitive();
    rules.push(DlClause::datalog(
        atom("?x", "selected", "?y"),
        vec![
            atom("?x", "reach", "?y"),
            atom("?y", "absent", "?local").into_negated(),
        ],
    ));
    answer_ownership(
        rules.clone(),
        atom("0", "selected", "?y"),
        ["<urn:0>", "<urn:edge>", "<urn:1>"],
    );
    governed(&rules, &atom("0", "selected", "?y"), || {
        store(&[(0, 1), (1, 2)])
    });
    for group in [
        Negation::new(
            vec![atom("?x", "edge", "?y")],
            vec![Guard::not_equal("?x", "?y")],
        ),
        Negation::new(vec![atom("?x", "?predicate", "?y")], Vec::new()),
    ] {
        let mut invalid = rules.clone();
        invalid.push(
            DlClause::datalog(
                atom("?x", "unreachable", "?y"),
                vec![atom("?x", "edge", "?y")],
            )
            .with_negations(vec![group]),
        );
        let dictionary = CheckedDictionary::default();
        let control = dictionary.control.clone();
        control.lookup_failure.store(true, Ordering::Relaxed);
        assert!(matches!(
            evaluate_demanded(
                &invalid,
                &atom("0", "selected", "?y"),
                RelationStore::with_dictionary(dictionary),
                &EvalOptions::default(),
                None
            ),
            Err(DemandError::Unsupported { .. })
        ));
        assert_eq!(control.writes.load(Ordering::Relaxed), 0);
    }
    let mut invalid = rules.clone();
    invalid.push(DlClause::datalog(
        atom("?x", "cycle", "?y"),
        vec![
            atom("?x", "edge", "?y"),
            atom("?x", "cycle", "?y").into_negated(),
        ],
    ));
    let dictionary = CheckedDictionary::default();
    let control = dictionary.control.clone();
    control.lookup_failure.store(true, Ordering::Relaxed);
    assert!(matches!(
        evaluate_demanded(
            &invalid,
            &atom("0", "selected", "?y"),
            RelationStore::with_dictionary(dictionary),
            &EvalOptions::default(),
            None
        ),
        Err(DemandError::Compile(EvalError::NonStratifiable { .. }))
    ));
    assert_eq!(control.writes.load(Ordering::Relaxed), 0);
    let mut owned =
        RelationStore::<CheckedDictionary, 3>::with_dictionary(CheckedDictionary::default());
    for row in [
        ["<urn:0>", "<urn:edge>", "<urn:1>"],
        ["<urn:1>", "<urn:edge>", "<urn:2>"],
    ] {
        owned
            .insert_terms(row.map(DictionaryTerm::Surface))
            .unwrap();
    }
    let control = owned.interner().control.clone();
    control.fail_write_at.store(
        control.writes.load(Ordering::Relaxed) + 2,
        Ordering::Relaxed,
    );
    assert!(matches!(
        evaluate_demanded(
            &rules,
            &atom("0", "selected", "?y"),
            owned,
            &EvalOptions::default(),
            None
        ),
        Err(DemandError::Source(Lost))
    ));
}

#[test]
fn negative_conjunction_names_and_shadow_collisions_preserve_answers() {
    let mut rules = transitive();
    rules.push(
        DlClause::datalog(
            atom("?x", "selected", "?y"),
            vec![atom("?x", "reach", "?y")],
        )
        .with_negations(vec![Negation::new(
            vec![atom("?x", "absent", "only-in-conjunction")],
            Vec::new(),
        )]),
    );
    let goal = atom("0", "selected", "?y");
    let base = evaluate_demanded(
        &rules,
        &goal,
        store(&[(0, 1)]),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    let expected = surfaces(base.dictionary(), base.answers().unwrap());
    let names = internal_terms(base.dictionary());
    let mut colliding = store(&[(0, 1)]);
    for name in &names {
        colliding
            .insert_terms([
                DictionaryTerm::Surface(name),
                DictionaryTerm::Surface("<urn:unused>"),
                DictionaryTerm::Surface("<urn:unused>"),
            ])
            .unwrap();
    }
    let answer =
        evaluate_demanded(&rules, &goal, colliding, &EvalOptions::default(), None).unwrap();
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        expected
    );
    let selected = rules.last().unwrap();
    let changed = DlClause::datalog(
        selected.datalog_head().unwrap().clone(),
        selected.body().to_vec(),
    )
    .with_negations(vec![Negation::new(
        vec![atom("?x", "absent", "different-conjunction-only")],
        Vec::new(),
    )]);
    *rules.last_mut().unwrap() = changed;
    let answer = evaluate_demanded(
        &rules,
        &goal,
        store(&[(0, 1)]),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    assert!(
        internal_terms(answer.dictionary())
            .iter()
            .all(|name| !names.contains(name))
    );
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        expected
    );
}

#[test]
fn selective_negative_support_reports_seed_overhead_and_deterministic_work() {
    let mut rules = transitive();
    rules.push(DlClause::datalog(
        atom("?x", "selected", "?y"),
        vec![
            atom("?x", "candidate", "?y"),
            atom("?x", "reach", "?y").into_negated(),
        ],
    ));
    let goal = atom("0", "selected", "?y");
    let options = EvalOptions::default()
        .with_round_execution(purrdf_datalog::seminaive::RoundExecution::Sequential);
    let mut committed = None;
    for extra in [0, 16, 64] {
        let mut facts = vec![
            atom("0", "candidate", "unreachable"),
            atom("0", "edge", "1"),
            atom("1", "edge", "2"),
        ];
        facts.extend(
            (10..10 + extra).map(|node| atom(&node.to_string(), "edge", &(node + 1).to_string())),
        );
        let source = input(&facts);
        let expected = full(&rules, &goal, source.clone());
        let mut previous = None;
        for _ in 0..2 {
            let input = source.clone();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let answer = evaluate_demanded(&rules, &goal, input, &options, None).unwrap();
            let measured = window.close();
            let preparation = answer.preparation();
            assert_eq!(preparation.original_facts, extra + 3);
            assert_eq!(preparation.shadow_facts, extra + 3);
            assert_eq!(
                answer.budget().input_terms(),
                preparation.original_terms + preparation.shadow_terms
            );
            let work = (answer.work().committed_facts(), answer.work().join_steps());
            if let Some(previous) = previous {
                assert_eq!(work, previous);
            }
            previous = Some(work);
            assert_eq!(
                surfaces(answer.dictionary(), answer.answers().unwrap()),
                expected
            );
            if let Some(committed) = committed {
                assert_eq!(
                    work.0, committed,
                    "unrelated recursive closure is never derived"
                );
            }
            committed = Some(work.0);
            println!(
                "negative-demand extra={extra} original_facts={} shadow_facts={} original_terms={} shadow_terms={} committed={} joins={} allocations={} requested_bytes={} net_retained_bytes={}",
                preparation.original_facts,
                preparation.shadow_facts,
                preparation.original_terms,
                preparation.shadow_terms,
                work.0,
                work.1,
                measured.allocations,
                measured.requested_bytes,
                measured.retained_bytes
            );
        }
    }
}

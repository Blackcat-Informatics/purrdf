// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native/wasm execution of connected and genuinely independent production rules.

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::guard::{Guard, GuardCall, GuardEvaluator, GuardRows, NoGuards};
use purrdf_datalog::seminaive::compile;
use purrdf_datalog::seminaive::{DerivationMode, EvalOptions, evaluate_guarded};
use purrdf_datalog::store::RelationStore;
use purrdf_datalog::{dictionary, seminaive};

#[path = "../src/test_support/host_dictionary.rs"]
mod host_dictionary;

struct Distinct;

impl GuardEvaluator for Distinct {
    fn evaluate(
        &self,
        call: &GuardCall<'_>,
    ) -> Result<GuardRows, purrdf_datalog::guard::CallbackError> {
        assert_eq!(call.inputs.len(), 2);
        Ok(if call.inputs[0] == call.inputs[1] {
            Vec::new()
        } else {
            vec![Vec::new()]
        })
    }
}

fn later_partial_delta_keeps_both_productive_anchors() {
    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(s),
            format!("https://example.org/{p}"),
            ClauseTerm::var(o),
        )
    };
    let program = compile(vec![
        DlClause::datalog(atom("?x", "p", "?y"), vec![atom("?x", "a", "?y")]),
        DlClause::datalog(atom("?y", "q", "?z"), vec![atom("?y", "b", "?z")]),
        DlClause::datalog(
            atom("?x", "r", "?z"),
            vec![atom("?x", "p", "?y"), atom("?y", "q", "?z")],
        ),
    ])
    .expect("partial-delta fixture compiles");
    let mut input = RelationStore::new();
    for (s, p, o) in [
        ("a", "p", "b"),
        ("b", "q", "c"),
        ("d", "a", "b"),
        ("b", "b", "e"),
    ] {
        input
            .insert(
                &format!("<https://example.org/{s}>"),
                &format!("<https://example.org/{p}>"),
                &format!("<https://example.org/{o}>"),
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
    }
    let evaluation = evaluate_guarded(
        &program,
        input,
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .expect("later productive anchors complete");
    let proofs: Vec<_> = evaluation
        .derivations()
        .expect("derivation recording was requested")
        .iter()
        .filter(|proof| proof.rule() == 2)
        .collect();
    assert_eq!(proofs.len(), 4);
    for (s, o) in [("a", "c"), ("d", "c"), ("a", "e"), ("d", "e")] {
        let proof = proofs
            .iter()
            .find(|proof| {
                proof.fact().subject == format!("<https://example.org/{s}>")
                    && proof.fact().object == format!("<https://example.org/{o}>")
            })
            .expect("all old/new combinations survive");
        assert_eq!(proof.sources().len(), 2);
        assert_eq!(proof.sources()[0].predicate, "<https://example.org/p>");
        assert_eq!(proof.sources()[1].predicate, "<https://example.org/q>");
        assert_eq!(proof.sources()[0].subject, proof.fact().subject);
        assert_eq!(proof.sources()[0].object, proof.sources()[1].subject);
        assert_eq!(proof.sources()[1].object, proof.fact().object);
    }
}

fn original_guarded_pair_has_exact_connected_work_and_authored_proofs() {
    const NS: &str = "https://example.invalid/k#";
    let atom = |s: ClauseTerm, p: &str, o: ClauseTerm| ClauseAtom::positive(s, p, o);
    let kind = purrdf_core::vocab::rdf::TYPE;
    let target = format!("{NS}target");
    let rule = DlClause::datalog(
        atom(
            ClauseTerm::var("?v"),
            &format!("{NS}hasTwoSpools"),
            ClauseTerm::literal(format!(
                "\"true\"^^<{}>",
                purrdf_core::datatype::XSD_BOOLEAN
            )),
        ),
        vec![
            atom(
                ClauseTerm::var("?v"),
                kind,
                ClauseTerm::iri(format!("{NS}Vault")),
            ),
            atom(ClauseTerm::var("?v"), &target, ClauseTerm::var("?s1")),
            atom(ClauseTerm::var("?v"), &target, ClauseTerm::var("?s2")),
            atom(
                ClauseTerm::var("?s1"),
                kind,
                ClauseTerm::iri(format!("{NS}Spool")),
            ),
            atom(
                ClauseTerm::var("?s2"),
                kind,
                ClauseTerm::iri(format!("{NS}Spool")),
            ),
        ],
    )
    .with_guards(vec![Guard::filter(
        "?s1 != ?s2",
        vec!["?s1".into(), "?s2".into()],
    )]);
    let program = compile(vec![rule]).expect("original guarded pair compiles");
    for two in [false, true] {
        let mut input = RelationStore::new();
        for i in 0..100 {
            let v = format!("<{NS}v{i}>");
            input
                .insert(
                    &v,
                    &format!("<{kind}>"),
                    &format!("<{NS}Vault>"),
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
            for suffix in ['a', 'b', 'c'] {
                let s = format!("<{NS}v{i}{suffix}>");
                input
                    .insert(&v, &format!("<{target}>"), &s, RelationStore::DEFAULT_GRAPH)
                    .unwrap();
                let class = if suffix == 'c' || (two && suffix == 'b') {
                    "Spool"
                } else {
                    "Store"
                };
                input
                    .insert(
                        &s,
                        &format!("<{kind}>"),
                        &format!("<{NS}{class}>"),
                        RelationStore::DEFAULT_GRAPH,
                    )
                    .unwrap();
            }
        }
        let evaluation = evaluate_guarded(
            &program,
            input,
            &Distinct,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .expect("unchanged defaults admit the original pair");
        assert_eq!(
            evaluation
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            if two { 100 } else { 0 }
        );
        // The whole frozen store is initially delta, so only the last anchor
        // can have a nonempty OldOnly suffix. Its five connected expansions
        // cost (1+3+1+3+1)N, or (1+3+2+6+4)N with two Spools. The guard's
        // two distinct assignments additionally charge 2N emitted heads.
        assert_eq!(
            evaluation.budget().join_steps(),
            if two { 1800 } else { 900 }
        );
        for proof in evaluation
            .derivations()
            .expect("derivation recording was requested")
        {
            let sources = proof.sources();
            assert_eq!(sources.len(), 5);
            assert_eq!(proof.fact().predicate, format!("<{NS}hasTwoSpools>"));
            assert_eq!(
                proof.fact().object,
                format!("\"true\"^^<{}>", purrdf_core::datatype::XSD_BOOLEAN)
            );
            assert_eq!(sources[0].predicate, format!("<{kind}>"));
            assert_eq!(sources[1].predicate, format!("<{target}>"));
            assert_eq!(sources[2].predicate, format!("<{target}>"));
            assert_eq!(sources[3].predicate, format!("<{kind}>"));
            assert_eq!(sources[4].predicate, format!("<{kind}>"));
            let v = &proof.fact().subject;
            let stem = v.strip_suffix('>').expect("IRI subject");
            assert_eq!(sources[0].subject, *v);
            assert_eq!(sources[0].object, format!("<{NS}Vault>"));
            assert_eq!(sources[1].subject, *v);
            assert_eq!(sources[2].subject, *v);
            assert_eq!(sources[1].object, format!("{stem}b>"));
            assert_eq!(sources[2].object, format!("{stem}c>"));
            assert_eq!(sources[3].subject, sources[1].object);
            assert_eq!(sources[4].subject, sources[2].object);
            assert_eq!(sources[3].object, format!("<{NS}Spool>"));
            assert_eq!(sources[4].object, format!("<{NS}Spool>"));
        }
    }
}

fn connected_rule_restores_all_authored_premises() {
    let atom = |s: ClauseTerm, p: &str, o: ClauseTerm| ClauseAtom::positive(s, p, o);
    let a = "https://example.org/a";
    let edge = "https://example.org/edge";
    let b = "https://example.org/b";
    let program = compile(vec![DlClause::datalog(
        atom(
            ClauseTerm::var("?x"),
            "https://example.org/result",
            ClauseTerm::var("?y"),
        ),
        vec![
            atom(
                ClauseTerm::var("?x"),
                a,
                ClauseTerm::iri("https://example.org/yes"),
            ),
            atom(ClauseTerm::var("?x"), edge, ClauseTerm::var("?y")),
            atom(
                ClauseTerm::var("?y"),
                b,
                ClauseTerm::iri("https://example.org/yes"),
            ),
        ],
    )])
    .expect("connected fixture compiles");
    let mut input = RelationStore::new();
    for i in 0..100 {
        let x = format!("<https://example.org/x{i}>");
        let y = format!("<https://example.org/y{i}>");
        input
            .insert(
                &x,
                &format!("<{a}>"),
                "<https://example.org/yes>",
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        input
            .insert(&x, &format!("<{edge}>"), &y, RelationStore::DEFAULT_GRAPH)
            .unwrap();
        input
            .insert(
                &y,
                &format!("<{b}>"),
                "<https://example.org/yes>",
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
    }
    let evaluation = evaluate_guarded(
        &program,
        input,
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .expect("unchanged target defaults admit linear connected work");
    assert_eq!(
        evaluation
            .derivations()
            .expect("derivation recording was requested")
            .len(),
        100
    );
    // All seeds are new in round one. The first two decompositions have empty
    // OldOnly suffixes and are skipped; Full/Full/Delta admits 3N frames.
    // Round two's result delta addresses none of the three body predicates.
    assert_eq!(evaluation.budget().join_steps(), 300);
    for proof in evaluation
        .derivations()
        .expect("derivation recording was requested")
    {
        let sources = proof.sources();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].predicate, format!("<{a}>"));
        assert_eq!(sources[1].predicate, format!("<{edge}>"));
        assert_eq!(sources[2].predicate, format!("<{b}>"));
        assert_eq!(sources[0].subject, proof.fact().subject);
        assert_eq!(sources[1].subject, proof.fact().subject);
        assert_eq!(sources[1].object, proof.fact().object);
        assert_eq!(sources[2].subject, proof.fact().object);
    }
}

fn independent_head_projects_without_cartesian_work() {
    let atom = |v: &str, p: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(v),
            p,
            ClauseTerm::iri("https://example.org/yes"),
        )
    };
    let program = compile(vec![DlClause::datalog(
        atom("?x", "https://example.org/result"),
        vec![
            atom("?y", "https://example.org/b"),
            atom("?x", "https://example.org/a"),
        ],
    )])
    .expect("independent fixture compiles");
    let mut input = RelationStore::new();
    for i in 0..100 {
        for p in ["a", "b"] {
            input
                .insert(
                    &format!("<https://example.org/x{i:03}>"),
                    &format!("<https://example.org/{p}>"),
                    "<https://example.org/yes>",
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
        }
    }
    let evaluation = evaluate_guarded(
        &program,
        input,
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .expect("unchanged target defaults admit additive factors");
    assert_eq!(
        evaluation
            .derivations()
            .expect("derivation recording was requested")
            .len(),
        100
    );
    assert_eq!(evaluation.budget().join_steps(), 300);
    for proof in evaluation
        .derivations()
        .expect("derivation recording was requested")
    {
        assert_eq!(proof.sources().len(), 2);
        assert_eq!(proof.sources()[0].subject, "<https://example.org/x000>");
        assert_eq!(proof.sources()[0].predicate, "<https://example.org/b>");
        assert_eq!(proof.sources()[1].subject, proof.fact().subject);
        assert_eq!(proof.sources()[1].predicate, "<https://example.org/a>");
    }
}

fn recursive_factor_preserves_authored_proofs_and_typed_limits() {
    use purrdf_datalog::seminaive::{BudgetResource, EvalError};

    let reach = "https://example.org/reach";
    let edge = "https://example.org/edge";
    let condition = "https://example.org/condition";
    let node = |index| format!("<https://example.org/node{index}>");
    let program = compile(vec![DlClause::datalog(
        ClauseAtom::positive(ClauseTerm::var("?x"), reach, ClauseTerm::var("?z")),
        vec![
            ClauseAtom::positive(ClauseTerm::var("?x"), reach, ClauseTerm::var("?y")),
            ClauseAtom::positive(ClauseTerm::var("?y"), edge, ClauseTerm::var("?z")),
            ClauseAtom::positive(
                ClauseTerm::iri("https://example.org/enabled"),
                condition,
                ClauseTerm::iri("https://example.org/value"),
            ),
        ],
    )])
    .expect("recursive factor compiles");
    let mut input = RelationStore::new();
    input
        .insert(
            "<https://example.org/enabled>",
            &format!("<{condition}>"),
            "<https://example.org/value>",
            RelationStore::DEFAULT_GRAPH,
        )
        .unwrap();
    const N: usize = 16;
    for index in 0..N - 1 {
        for predicate in [reach, edge] {
            input
                .insert(
                    &node(index),
                    &format!("<{predicate}>"),
                    &node(index + 1),
                    RelationStore::DEFAULT_GRAPH,
                )
                .unwrap();
        }
    }
    let reference = evaluate_guarded(
        &program,
        input.clone(),
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .expect("default limits admit recursion");
    assert_eq!(
        reference
            .derivations()
            .expect("derivation recording was requested")
            .len(),
        (N - 1) * (N - 2) / 2
    );
    assert_eq!(
        reference.facts().row_count(),
        input.row_count()
            + reference
                .derivations()
                .expect("derivation recording was requested")
                .len()
    );
    for start in 0..N - 2 {
        for end in start + 2..N {
            let proof = reference
                .derivations()
                .expect("derivation recording was requested")
                .iter()
                .find(|proof| {
                    proof.fact().subject == node(start) && proof.fact().object == node(end)
                })
                .expect("every inferred chain pair has a proof");
            assert_eq!(proof.rule(), 0);
            assert_eq!(proof.fact().predicate, format!("<{reach}>"));
            assert_eq!(proof.fact().graph, RelationStore::DEFAULT_GRAPH);
            let sources = proof.sources();
            assert_eq!(sources.len(), 3);
            for (source, subject, predicate, object) in [
                (&sources[0], node(start), reach, node(end - 1)),
                (&sources[1], node(end - 1), edge, node(end)),
                (
                    &sources[2],
                    "<https://example.org/enabled>".into(),
                    condition,
                    "<https://example.org/value>".into(),
                ),
            ] {
                assert_eq!(source.subject, subject);
                assert_eq!(source.predicate, format!("<{predicate}>"));
                assert_eq!(source.object, object);
                assert_eq!(source.graph, RelationStore::DEFAULT_GRAPH);
            }
        }
    }
    let expected = reference.budget().join_steps();
    let exact = evaluate_guarded(
        &program,
        input.clone(),
        &NoGuards,
        &EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_join_steps(expected),
        None,
    )
    .expect("exact credits admit partial factor rounds");
    assert_eq!(
        exact.facts().facts_sorted(),
        reference.facts().facts_sorted()
    );
    assert_eq!(
        exact
            .derivations()
            .expect("derivation recording was requested"),
        reference
            .derivations()
            .expect("derivation recording was requested")
    );
    assert_eq!(exact.budget().join_steps(), expected);
    assert_eq!(
        exact.budget().stored_facts(),
        reference.budget().stored_facts()
    );
    for ceiling in [0, expected - 1] {
        let error = evaluate_guarded(
            &program,
            input.clone(),
            &NoGuards,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(ceiling),
            None,
        )
        .expect_err("insufficient credits refuse without a partial result");
        let EvalError::BudgetExhausted { resource, report } = error else {
            panic!("{error:?}");
        };
        assert_eq!(resource, BudgetResource::JoinSteps);
        assert_eq!(report.join_steps(), ceiling + 1);
    }
}

fn public_host_guarded_evidence_fixpoint_retains_ids_and_first_source_error() {
    use host_dictionary::{HostDictionary, SourceLost, evidence_scope};
    use purrdf_core::evidence::{
        EvidenceRecord, Frame, Information, InformationMap, SearchCompleteness,
    };
    use purrdf_datalog::guard::CallbackError;
    use purrdf_datalog::id::RowId;
    use purrdf_datalog::seminaive::{EvalError, evaluate_evidenced};
    use purrdf_lex::allocation::Resident;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    const P: &str = "https://example.org/p";
    const Q: &str = "https://example.org/q";
    const R: &str = "https://example.org/r";
    let atom =
        |s: &str, p: &str, o: &str| ClauseAtom::positive(ClauseTerm::var(s), p, ClauseTerm::var(o));
    const TRIPLE: u64 = 0xa000_0000_1234_5678;
    struct Producer {
        fail: bool,
        calls: AtomicUsize,
    }
    impl GuardEvaluator<HostDictionary> for Producer {
        fn evaluate(
            &self,
            call: &GuardCall<'_, HostDictionary>,
        ) -> Result<GuardRows<u64>, CallbackError<SourceLost>> {
            call.evaluate_lexical(|inputs| {
                self.calls.fetch_add(1, Ordering::Relaxed);
                assert_eq!(inputs, ["triple", "value"]);
                assert_eq!(call.model.interner().pins.load(Ordering::Relaxed), 2);
                if self.fail {
                    call.model.interner().lost.store(true, Ordering::Relaxed);
                    return Err(CallbackError::Source(SourceLost(17)));
                }
                Ok(vec![vec!["output".into()]])
            })
        }
    }
    let exe = compile(vec![
        DlClause::datalog(atom("?s", Q, "?out"), vec![atom("?s", P, "?o")]).with_guards(vec![
            Guard::assign("produce", vec!["?s".into(), "?o".into()], "?out"),
        ]),
        DlClause::datalog(atom("?s", R, "?o"), vec![atom("?s", Q, "?o")]),
    ])
    .unwrap();
    for (information, fail) in [
        (Information::Supported, false),
        (Information::Opposed, false),
        (Information::Opposed, true),
    ] {
        let mut store = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (TRIPLE, "triple".into()),
                (u64::MAX, "value".into()),
                (7, format!("<{P}>")),
                (9, format!("<{Q}>")),
                (11, format!("<{R}>")),
                (12, "output".into()),
                (1, String::new()),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            extra_terms: 0,
        });
        store.insert_ids([TRIPLE, 7, u64::MAX, 1]).unwrap().unwrap();
        let mut rows = purrdf_datalog::evidence::DerivationEvidence::with_admission(1, Resident);
        if information != Information::Supported {
            rows.seed(RowId::from_index(0).unwrap(), |memory| {
                EvidenceRecord::from_parts(
                    &InformationMap::uniform(information),
                    SearchCompleteness::Complete,
                    &Frame::actual(),
                    &[],
                    &[],
                    memory,
                )
            })
            .unwrap();
        }
        let producer = Producer {
            fail,
            calls: AtomicUsize::new(0),
        };
        let result = evaluate_evidenced(
            &exe,
            store,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            &producer,
            None,
            rows,
            evidence_scope(),
        );
        if fail {
            let Err(error) = result else {
                panic!("original source failure must refuse the model")
            };
            assert_eq!(error, EvalError::Source(SourceLost(17)));
            assert_eq!(
                std::error::Error::source(&error)
                    .unwrap()
                    .downcast_ref::<SourceLost>(),
                Some(&SourceLost(17))
            );
        } else {
            let result = result.unwrap();
            assert_eq!(result.evaluation.facts().row_count(), 3);
            assert_eq!(
                result
                    .evaluation
                    .derivations()
                    .expect("derivation recording was requested")
                    .len(),
                2
            );
            assert_eq!(result.evidence.row_count(), 3);
            for (row, ids) in result.evaluation.facts().rows_from(1) {
                assert_eq!(ids[0], TRIPLE);
                assert!(matches!(ids[1], 9 | 11));
                assert_eq!([ids[2], ids[3]], [12, 1]);
                assert_eq!(
                    result
                        .evidence
                        .record(row)
                        .unwrap()
                        .information()
                        .fallback(),
                    information
                );
            }
            assert_eq!(
                result
                    .evaluation
                    .facts()
                    .interner()
                    .pins
                    .load(Ordering::Relaxed),
                0
            );
            assert_eq!(result.evaluation.budget().generated_terms(), 0);
        }
        assert_eq!(producer.calls.load(Ordering::Relaxed), 1);
    }
}

fn triple_compiler_preserves_static_arity_and_original_admission() {
    use purrdf_datalog::admission::{
        AdmissionRefusal, DeclarationLayer, DeclarationSource, NeverDeriveDeclarations,
    };
    use purrdf_datalog::cache::{PlanCache, canonical_rule_hash};
    use purrdf_datalog::proof::{EvaluationProofs, ProofArena, ProofContext};
    use purrdf_datalog::schedule::{
        NoHooks, compile_scheduled, evaluate_scheduled, stratify_rules,
    };
    use purrdf_datalog::seminaive::{EvalError, compile_with_declarations};
    use purrdf_datalog::store::{Arity, StoreShape, TermInterner};

    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::triple(ClauseTerm::var(s), ClauseTerm::iri(p), ClauseTerm::var(o))
    };
    let predicate = "https://example.org/transitive";
    let rule = DlClause::datalog(
        atom("?s", predicate, "?o"),
        vec![atom("?s", predicate, "?m"), atom("?m", predicate, "?o")],
    );
    let rules = vec![rule];
    let program = compile(rules.clone()).unwrap();
    let (rule, plan) = program.rule_entry(0);
    assert_eq!(rule.body()[0].terms().len(), 3);
    assert_eq!(plan.variables(), &["?s", "?m", "?o"]);
    for operator in plan.operators() {
        assert_eq!(operator.shape().positions().len(), 3);
        assert_eq!(operator.shape().adornment().arity(), 3);
        assert!(operator.shape().addresses_one_partition());
    }
    assert_eq!(
        size_of::<ClauseAtom<4>>() - size_of::<ClauseAtom<3>>(),
        size_of::<ClauseTerm>()
    );
    let quad = DlClause::datalog(
        ClauseAtom::positive(ClauseTerm::var("?s"), predicate, ClauseTerm::var("?o")),
        rules[0]
            .body()
            .iter()
            .map(|atom| {
                ClauseAtom::quad(
                    atom.subject().clone(),
                    atom.predicate().clone(),
                    atom.object().clone(),
                    ClauseTerm::DefaultGraph,
                )
            })
            .collect(),
    );
    assert_ne!(
        canonical_rule_hash(&rules),
        canonical_rule_hash(std::slice::from_ref(&quad))
    );
    let mut triple_input =
        RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    let mut quad_input = RelationStore::new();
    let p = format!("<{predicate}>");
    for (s, o) in [("a", "b"), ("b", "c"), ("c", "d"), ("e", "b")] {
        triple_input.insert_row([s, &p, o]).unwrap();
        quad_input
            .insert(s, &p, o, RelationStore::DEFAULT_GRAPH)
            .unwrap();
    }
    let triples = evaluate_guarded(
        &program,
        triple_input.clone(),
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    let quads = evaluate_guarded(
        &compile(vec![quad]).unwrap(),
        quad_input,
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(triples.facts().row_count(), 9);
    assert_eq!(triples.facts().partition_count(), 1);
    assert!(triples.facts().interner().lookup("").is_none());
    assert_eq!(
        triples
            .facts()
            .facts_sorted()
            .iter()
            .map(|fact| Arity::<3>::fact_terms(fact).map(str::to_owned))
            .collect::<Vec<_>>(),
        quads
            .facts()
            .facts_sorted()
            .into_iter()
            .map(|fact| [fact.subject, fact.predicate, fact.object])
            .collect::<Vec<_>>()
    );
    assert_eq!(
        triples
            .derivations()
            .expect("derivation recording was requested")
            .len(),
        quads
            .derivations()
            .expect("derivation recording was requested")
            .len()
    );
    for (triple, quad) in triples
        .derivations()
        .expect("derivation recording was requested")
        .iter()
        .zip(
            quads
                .derivations()
                .expect("derivation recording was requested"),
        )
    {
        assert_eq!(triple.rule(), quad.rule());
        assert_eq!(triple.proof_height(), quad.proof_height());
        assert_eq!(triple.sources().len(), 2);
        for (actual, expected) in triple.sources().iter().zip(quad.sources()) {
            assert_eq!(
                Arity::<3>::fact_terms(actual),
                [
                    expected.subject.as_str(),
                    expected.predicate.as_str(),
                    expected.object.as_str()
                ]
            );
        }
    }
    let proofs = EvaluationProofs::of(&triples).unwrap();
    let context = ProofContext::new(&rules, &triple_input, triples.facts());
    for (fact, root) in proofs.roots() {
        assert_eq!(&proofs.arena().check(*root, &context).unwrap(), fact);
        let bytes = proofs.arena().encode(*root);
        let (decoded, decoded_root) = ProofArena::<3>::decode(&bytes).unwrap();
        assert_eq!(decoded.encode(decoded_root), bytes);
        assert_eq!(decoded.digest(decoded_root), proofs.arena().digest(*root));
        assert_eq!(&decoded.check(decoded_root, &context).unwrap(), fact);
        assert!(ProofArena::<4>::decode(&bytes).is_err());
    }
    let chase = purrdf_datalog::chase::chase(&rules, triple_input.clone()).unwrap();
    assert_eq!(chase.facts().facts_sorted(), triples.facts().facts_sorted());
    let mut cache = PlanCache::<3>::new(2);
    let cold = cache.get_or_compile("https://example.org/contract", rules.clone());
    assert!(!cold.cache_hit());
    assert!(cold.plan().is_ok());
    let warm = cache.get_or_compile("https://example.org/contract", rules.clone());
    assert!(warm.cache_hit());
    assert_eq!(warm.planning_units(), 0);
    let schedule = stratify_rules(&rules, &[false]).unwrap();
    let scheduled = compile_scheduled(rules.clone(), schedule).unwrap();
    assert_eq!(scheduled.rules(), rules);
    let scheduled_result = evaluate_scheduled(
        &scheduled,
        triple_input,
        &NoGuards,
        &mut NoHooks,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(
        scheduled_result.facts().facts_sorted(),
        triples.facts().facts_sorted()
    );
    assert_eq!(
        scheduled_result
            .derivations()
            .expect("derivation recording was requested"),
        triples
            .derivations()
            .expect("derivation recording was requested")
    );
    let mut declarations = NeverDeriveDeclarations::default();
    declarations.declare(
        predicate.into(),
        DeclarationSource {
            layer: DeclarationLayer::StoreProfile,
            identity: "https://example.org/authority".into(),
        },
    );
    assert!(matches!(
        compile_with_declarations(rules, &declarations),
        Err(EvalError::NeverDerive(
            AdmissionRefusal::ProtectedHead { .. }
        ))
    ));
    assert!(matches!(
        compile(vec![DlClause::datalog(
            atom("?unbound", predicate, "?o"),
            vec![atom("?s", predicate, "?o")]
        )]),
        Err(EvalError::UnboundHeadVariable { .. })
    ));
}

fn triple_base_authority_survives_data_growth_and_layer_retraction() {
    use purrdf_datalog::cache::canonical_rule_hash;
    use purrdf_datalog::guard::{CallbackError, Negation, NoGuards};
    use purrdf_datalog::proof::{EvaluationProofs, ProofContext};
    use purrdf_datalog::schedule::{
        Layer, LayerHooks, NoHooks, Schedule, compile_scheduled, evaluate_scheduled, stratify_rules,
    };
    use purrdf_datalog::seminaive::{BudgetResource, EvalError};
    use purrdf_datalog::store::{Fact, TermInterner};

    struct WithdrawOriginal;
    impl LayerHooks<3> for WithdrawOriginal {
        fn retract(
            &mut self,
            layer: usize,
            _: &RelationStore<TermInterner, 3>,
            _: &[Fact<()>],
        ) -> Result<Vec<Fact<()>>, CallbackError> {
            Ok(if layer == 0 {
                vec![Fact {
                    subject: "a".into(),
                    predicate: "p".into(),
                    object: "b".into(),
                    graph: (),
                }]
            } else {
                Vec::new()
            })
        }
    }
    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::triple(
            ClauseTerm::var(s),
            ClauseTerm::literal(p),
            ClauseTerm::var(o),
        )
    };
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    input.insert_row(["a", "p", "b"]).unwrap();
    let missing_base = input.clone();
    input = input.with_base_snapshot().unwrap();
    // DATA is visible to ordinary patterns but not to WHERE DATA.
    input.insert_row(["c", "p", "d"]).unwrap();
    let rules = vec![
        DlClause::datalog(
            ClauseAtom::triple(
                ClauseTerm::literal("b"),
                ClauseTerm::literal("p"),
                ClauseTerm::literal("e"),
            ),
            Vec::new(),
        ),
        DlClause::datalog(
            atom("?s", "q", "?o"),
            vec![atom("?s", "p", "?o").from_base()],
        ),
        DlClause::datalog(atom("?s", "r", "?o"), vec![atom("?s", "p", "?o")]),
        DlClause::datalog(
            atom("?s", "n", "?o"),
            vec![
                atom("?s", "p", "?o"),
                atom("?s", "p", "?o").from_base().into_negated(),
            ],
        ),
        DlClause::datalog(atom("?s", "g", "?o"), vec![atom("?s", "p", "?o")]).with_negations(vec![
            Negation::new(vec![atom("?s", "p", "?o").from_base()], Vec::new()),
        ]),
    ];
    let same_evaluation_read =
        DlClause::datalog(atom("?s", "q", "?o"), vec![atom("?s", "p", "?o")]);
    assert_ne!(
        canonical_rule_hash(&rules[1..2]),
        canonical_rule_hash(&[same_evaluation_read])
    );
    assert!(matches!(
        compile(vec![rules[1].clone()]),
        Err(EvalError::BaseReadRequiresSchedule { rule: 0 })
    ));
    // A closed Base read does not acquire a recursive dependency on model writes.
    let base_negative = DlClause::datalog(
        atom("?s", "p", "?o"),
        vec![
            atom("?s", "p", "?o").from_base(),
            atom("?s", "p", "?o").from_base().into_negated(),
        ],
    );
    assert!(stratify_rules(&[base_negative], &[false]).is_ok());
    let schedule = Schedule::new(vec![
        Layer::new(vec![vec![0]], Vec::new()),
        Layer::new(vec![vec![1, 2, 3, 4]], Vec::new()),
    ]);
    let program = compile_scheduled(rules.clone(), schedule).unwrap();
    let budget_program = compile_scheduled(
        vec![rules[0].clone()],
        Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]),
    )
    .unwrap();
    let initial_term_bytes = input.interner().byte_len() + missing_base.interner().byte_len();
    assert!(input.interner().lookup("e").is_none());
    let refused = evaluate_scheduled(
        &budget_program,
        input.clone(),
        &NoGuards,
        &mut NoHooks,
        &EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts(3),
        None,
    );
    let Err(EvalError::BudgetExhausted {
        resource: BudgetResource::StoredFacts,
        report,
    }) = refused
    else {
        panic!("the retained base and projected fact exceed the limit");
    };
    assert_eq!(report.stored_facts(), 3);
    assert_eq!(report.observed_stored_facts(), 4);
    assert_eq!(
        report.generated_terms(),
        0,
        "refusal precedes sink interning"
    );
    assert_eq!(report.term_arena_bytes(), initial_term_bytes);
    let admitted = evaluate_scheduled(
        &budget_program,
        input.clone(),
        &NoGuards,
        &mut NoHooks,
        &EvalOptions::default()
            .with_derivation_mode(DerivationMode::Record)
            .with_max_stored_facts(4),
        None,
    )
    .expect("the exact retained-row boundary admits the fact");
    assert_eq!(admitted.budget().stored_facts(), 4);
    assert_eq!(admitted.budget().generated_terms(), 1);
    assert!(admitted.facts().contains_row(["b", "p", "e"]));
    assert!(matches!(
        evaluate_scheduled(
            &program,
            missing_base,
            &NoGuards,
            &mut WithdrawOriginal,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None
        ),
        Err(EvalError::MissingBase { rule: 1 })
    ));
    let result = evaluate_scheduled(
        &program,
        input.clone(),
        &NoGuards,
        &mut WithdrawOriginal,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    let model = result.facts();
    assert!(!model.contains_row(["a", "p", "b"]));
    assert!(model.contains_row(["a", "q", "b"]));
    assert!(!model.contains_row(["c", "q", "d"]));
    assert!(!model.contains_row(["b", "q", "e"]));
    for predicate in ["r", "n", "g"] {
        assert!(model.contains_row(["c", predicate, "d"]));
        assert!(model.contains_row(["b", predicate, "e"]));
        assert!(!model.contains_row(["a", predicate, "b"]));
    }
    assert_eq!(model.row_count(), 9);
    assert_eq!(result.budget().stored_facts(), 10); // nine live model rows plus one retained base row
    assert!(model.interner().lookup("").is_none());
    let base_derivation = result
        .derivations()
        .expect("derivation recording was requested")
        .iter()
        .find(|d| d.fact().predicate == "q")
        .unwrap();
    assert_eq!(base_derivation.proof_height(), 1);
    assert_eq!(base_derivation.sources()[0].subject, "a");
    let proofs = EvaluationProofs::of(&result).unwrap();
    let context = ProofContext::new(&rules, &input, model);
    let (fact, root) = proofs
        .roots()
        .iter()
        .find(|(fact, _)| fact.predicate == "q")
        .unwrap();
    assert_eq!(&proofs.arena().check(*root, &context).unwrap(), fact);
}

fn triple_negated_base_preserves_seminaive_work_and_limits() {
    use purrdf_datalog::guard::{Negation, NoGuards};
    use purrdf_datalog::schedule::{
        Layer, NoHooks, Schedule, compile_scheduled, evaluate_scheduled,
    };
    use purrdf_datalog::store::TermInterner;

    fn filtered_rule<const N: usize>(
        head: ClauseAtom<N>,
        mut body: Vec<ClauseAtom<N>>,
        absent: ClauseAtom<N>,
        grouped: bool,
    ) -> DlClause<N> {
        let negations = if grouped {
            vec![Negation::new(vec![absent], Vec::new())]
        } else {
            body.push(absent.into_negated());
            Vec::new()
        };
        DlClause::datalog(head, body).with_negations(negations)
    }

    const NODES: usize = 12;
    for grouped in [false, true] {
        let (s, m, o) = (
            ClauseTerm::var("?s"),
            ClauseTerm::var("?m"),
            ClauseTerm::var("?o"),
        );
        let triple_rule = filtered_rule(
            ClauseAtom::triple(s.clone(), ClauseTerm::iri("urn:path"), o.clone()),
            vec![
                ClauseAtom::triple(s.clone(), ClauseTerm::iri("urn:path"), m.clone()),
                ClauseAtom::triple(m.clone(), ClauseTerm::iri("urn:path"), o.clone()),
            ],
            ClauseAtom::triple(s.clone(), ClauseTerm::iri("urn:blocked"), o.clone()).from_base(),
            grouped,
        );
        let quad_rule = filtered_rule(
            ClauseAtom::positive(s.clone(), "urn:path", o.clone()),
            vec![
                ClauseAtom::positive(s.clone(), "urn:path", m.clone()),
                ClauseAtom::positive(m, "urn:path", o.clone()),
            ],
            ClauseAtom::quad(
                s,
                ClauseTerm::iri("urn:blocked"),
                o,
                ClauseTerm::iri("urn:base"),
            ),
            grouped,
        );
        let schedule = Schedule::new(vec![Layer::new(Vec::new(), vec![vec![0]])]);
        let triple_program = compile_scheduled(vec![triple_rule], schedule.clone()).unwrap();
        let quad_program = compile_scheduled(vec![quad_rule], schedule).unwrap();
        let mut compact =
            RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default())
                .with_base_snapshot()
                .unwrap();
        let mut quad = RelationStore::new();
        for index in 0..NODES - 1 {
            let subject = format!("<urn:node{index}>");
            let object = format!("<urn:node{}>", index + 1);
            compact
                .insert_row([&subject, "<urn:path>", &object])
                .unwrap();
            quad.insert(
                &subject,
                "<urn:path>",
                &object,
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        }
        let reference = evaluate_scheduled(
            &quad_program,
            quad,
            &NoGuards,
            &mut NoHooks,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .unwrap();
        let expected_steps = reference.budget().join_steps();
        let actual = evaluate_scheduled(
            &triple_program,
            compact,
            &NoGuards,
            &mut NoHooks,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(expected_steps),
            None,
        )
        .expect("immutable negative filters retain the quad semi-naive work bound");
        assert_eq!(actual.budget().join_steps(), expected_steps);
        assert_eq!(actual.facts().row_count(), NODES * (NODES - 1) / 2);
        assert_eq!(actual.facts().row_count(), reference.facts().row_count());
        for start in 0..NODES - 1 {
            for end in start + 1..NODES {
                assert!(actual.facts().contains_row([
                    &format!("<urn:node{start}>"),
                    "<urn:path>",
                    &format!("<urn:node{end}>"),
                ]));
            }
        }
    }
}

fn triple_no_output_runs_the_body_without_fabricating_a_fact() {
    use purrdf_datalog::cache::canonical_rule_hash;
    use purrdf_datalog::clause::HeadForm;
    use purrdf_datalog::schedule::{
        Layer, NoHooks, Schedule, compile_scheduled, evaluate_scheduled,
    };
    use purrdf_datalog::seminaive::EvalError;
    use purrdf_datalog::store::TermInterner;

    let body = vec![ClauseAtom::triple(
        ClauseTerm::var("?s"),
        ClauseTerm::literal("p"),
        ClauseTerm::var("?o"),
    )];
    let rule = DlClause::no_output(body.clone());
    let logical_false = DlClause::<3>::new(Vec::new(), Vec::new(), body);
    assert_eq!(rule.head_form(), HeadForm::NoOutput);
    assert_ne!(
        canonical_rule_hash(std::slice::from_ref(&rule)),
        canonical_rule_hash(std::slice::from_ref(&logical_false))
    );
    assert!(matches!(
        compile(vec![rule.clone()]),
        Err(EvalError::NonDatalogHead {
            form: HeadForm::NoOutput,
            ..
        })
    ));
    let schedule = Schedule::new(vec![Layer::new(vec![vec![0]], Vec::new())]);
    let program = compile_scheduled(vec![rule.clone()], schedule.clone()).unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    input.insert_row(["a", "p", "b"]).unwrap();
    let result = evaluate_scheduled(
        &program,
        input.clone(),
        &NoGuards,
        &mut NoHooks,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(result.facts().facts_sorted(), input.facts_sorted());
    assert_eq!(
        result
            .derivations()
            .expect("derivation recording was requested"),
        []
    );
    assert_eq!(result.facts().interner().len(), 3);
    assert!(matches!(
        evaluate_scheduled(
            &program,
            input.clone(),
            &NoGuards,
            &mut NoHooks,
            &EvalOptions::default()
                .with_derivation_mode(DerivationMode::Record)
                .with_max_join_steps(0),
            None,
        ),
        Err(EvalError::BudgetExhausted { .. })
    ));
    let guarded = rule.with_guards(vec![Guard::filter("must-run", vec!["?s".into()])]);
    let program = compile_scheduled(vec![guarded], schedule).unwrap();
    assert!(matches!(
        evaluate_scheduled(
            &program, input, &NoGuards, &mut NoHooks, &EvalOptions::default().with_derivation_mode(DerivationMode::Record), None,
        ),
        Err(EvalError::Guard { guard, .. }) if guard == "must-run"
    ));
}

fn triple_store_has_predicate_only_keys_and_transactional_original_ids() {
    use host_dictionary::{HostDictionary, SourceLost};
    use purrdf_datalog::dictionary::{DictionaryError, DictionaryTerm};
    use purrdf_datalog::store::{Arity, ShapeFact, StoreKey, StoreShape, TermInterner};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    assert_eq!(size_of::<StoreKey<u64, 3>>(), size_of::<u64>());
    assert_eq!(size_of::<StoreKey<u64, 4>>(), 2 * size_of::<u64>());
    assert_eq!(size_of::<ShapeFact<3>>(), 3 * size_of::<String>());
    assert_eq!(size_of::<ShapeFact<4>>(), 4 * size_of::<String>());
    let mut triples = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for row in [["a", "p", "b"], ["b", "p", "c"], ["a", "p", "b"]] {
        triples
            .insert_terms(row.map(DictionaryTerm::Surface))
            .unwrap();
    }
    assert_eq!(triples.row_count(), 2);
    assert_eq!(triples.partition_count(), 1);
    assert_eq!(triples.interner().len(), 4);
    assert!(triples.interner().lookup("").is_none());
    assert_eq!(triples.interner().lookup("p").unwrap().index(), 0);
    let rows = triples
        .rows_from(0)
        .map(|(_, row)| row.map(|id| triples.interner().resolve(id)))
        .collect::<Vec<_>>();
    assert_eq!(rows, [["a", "p", "b"], ["b", "p", "c"]]);
    let facts = triples.facts_sorted();
    assert_eq!(
        facts.iter().map(Arity::<3>::fact_terms).collect::<Vec<_>>(),
        rows
    );
    let partition = triples.partitions_for([None; 3]).next().unwrap();
    assert_eq!(partition.row_count(), 2);
    let predicate = partition.predicate();
    assert_eq!(
        triples
            .partition_for(&[None, Some(predicate), None])
            .unwrap()
            .id(),
        partition.id()
    );

    const P: u64 = 0xf000_0000_0000_0007;
    const S: u64 = 0xa000_0000_1234_5678;
    let dictionary = HostDictionary {
        surfaces: std::iter::once((P, "<https://example.org/p>".into())).collect(),
        extra_terms: 0,
        resolves: AtomicUsize::default(),
        lookups: AtomicUsize::default(),
        lost: AtomicBool::default(),
        lose_after_lookup: AtomicBool::default(),
        pins: AtomicUsize::default(),
    };
    let mut host = RelationStore::<_, 3>::with_dictionary(dictionary);
    host.insert_ids([S, P, 0]).unwrap().unwrap();
    assert!(host.insert_ids([S, P, 0]).unwrap().is_none());
    assert_eq!(host.rows_from(0).next().unwrap().1, [S, P, 0]);
    assert_eq!(host.interner().pins.load(Ordering::Relaxed), 0);
    host.interner().lost.store(true, Ordering::Relaxed);
    assert!(matches!(
        host.insert_ids([0, P, S]),
        Err(DictionaryError::Source(SourceLost(99)))
    ));
    assert_eq!(host.row_count(), 1);
}

fn model_only_and_recorded_models_have_explicit_evidence_and_equal_counts() {
    use purrdf_datalog::proof::{EvaluationProofs, ProofContext, ProofError};
    use purrdf_datalog::seminaive::Executable;

    let atom = |predicate: &str| {
        ClauseAtom::positive(ClauseTerm::var("?s"), predicate, ClauseTerm::var("?o"))
    };
    let rules = vec![DlClause::datalog(
        atom("https://example.org/q"),
        vec![atom("https://example.org/p")],
    )];
    let program: Executable = compile(rules.clone()).unwrap();
    let mut input = RelationStore::new();
    input
        .insert(
            "a",
            "<https://example.org/p>",
            "b",
            RelationStore::DEFAULT_GRAPH,
        )
        .unwrap();
    let model = seminaive::evaluate(&program, input.clone()).unwrap();
    assert!(model.derivations().is_none());
    assert!(matches!(
        EvaluationProofs::of(&model),
        Err(ProofError::DerivationsNotRecorded)
    ));
    let recorded = evaluate_guarded(
        &program,
        input.clone(),
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(
        model.facts().facts_sorted(),
        recorded.facts().facts_sorted()
    );
    assert_eq!(model.work(), recorded.work());
    assert_eq!(model.work().input_facts(), 1);
    assert_eq!(model.work().committed_facts(), 1);
    assert_eq!(
        model.work().candidates_per_committed_fact(),
        Some((model.budget().join_steps(), 1))
    );
    assert_eq!(model.work().join_steps(), model.budget().join_steps());
    assert_eq!(
        model.work().planner_version(),
        purrdf_datalog::cache::PLAN_SOLVER_VERSION
    );
    let proofs = EvaluationProofs::of(&recorded).unwrap();
    for (fact, root) in proofs.roots() {
        assert_eq!(
            &proofs
                .arena()
                .check(*root, &ProofContext::new(&rules, &input, recorded.facts()))
                .unwrap(),
            fact
        );
    }
    let empty = compile(Vec::<DlClause>::new()).unwrap();
    let recorded_empty = evaluate_guarded(
        &empty,
        input,
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(recorded_empty.derivations(), Some([].as_slice()));
    assert_eq!(recorded_empty.work().candidates_per_committed_fact(), None);
}

fn term_arena_limits_are_caller_selected_and_reach_contracts() {
    use purrdf_datalog::cache::contract_hash_with;
    use purrdf_datalog::seminaive::{BudgetResource, DEFAULT_MAX_TERM_ARENA_BYTES, EvalError};

    let rules: Vec<DlClause> = Vec::new();
    let program = compile(rules.clone()).unwrap();
    let mut input = RelationStore::new();
    input
        .insert(
            &"x".repeat(DEFAULT_MAX_TERM_ARENA_BYTES + 1),
            "p",
            "o",
            RelationStore::DEFAULT_GRAPH,
        )
        .unwrap();
    let bytes = input.term_bytes() as u64;
    let default = EvalOptions::default();
    let refused = seminaive::evaluate(&program, input.clone()).unwrap_err();
    assert!(
        refused
            .to_string()
            .ends_with("raise it with EvalOptions::with_max_term_arena_bytes")
    );
    let EvalError::BudgetExhausted { resource, report } = refused else {
        panic!("arena refusal")
    };
    assert_eq!(resource, BudgetResource::TermArenaBytes);
    assert_eq!(resource.limit(report), DEFAULT_MAX_TERM_ARENA_BYTES as u64);
    assert!(!report.term_arena_limit_stated());
    let raised = default.with_max_term_arena_bytes(bytes);
    let admitted = evaluate_guarded(&program, input.clone(), &NoGuards, &raised, None).unwrap();
    assert_eq!(admitted.budget().term_arena_limit(), bytes);
    assert!(admitted.budget().term_arena_limit_stated());
    let unlimited = default.without_term_arena_limit();
    let admitted = evaluate_guarded(&program, input, &NoGuards, &unlimited, None).unwrap();
    assert_eq!(admitted.budget().term_arena_limit(), u64::MAX);
    assert_ne!(
        contract_hash_with(&rules, &default),
        contract_hash_with(&rules, &raised)
    );
    assert_ne!(
        contract_hash_with(&rules, &raised),
        contract_hash_with(&rules, &unlimited)
    );
    assert_eq!(
        contract_hash_with(&rules, &default),
        contract_hash_with(
            &rules,
            &default.with_max_term_arena_bytes(DEFAULT_MAX_TERM_ARENA_BYTES as u64)
        )
    );
}

fn stored_fact_admission_refuses_the_first_extra_fact_in_both_engines() {
    use purrdf_datalog::chase::{ChaseError, chase_with};
    use purrdf_datalog::seminaive::{BudgetResource, EvalError};

    let atom = |predicate: &str| {
        ClauseAtom::positive(ClauseTerm::var("?s"), predicate, ClauseTerm::var("?o"))
    };
    let rules = vec![
        DlClause::datalog(
            atom("https://example.org/q"),
            vec![atom("https://example.org/p")],
        ),
        DlClause::datalog(
            atom("https://example.org/q"),
            vec![atom("https://example.org/p")],
        ),
        DlClause::datalog(
            atom("https://example.org/r"),
            vec![atom("https://example.org/p")],
        ),
    ];
    let program = compile(rules.clone()).unwrap();
    let mut input = RelationStore::new();
    for subject in ["a", "b", "c"] {
        input
            .insert(
                subject,
                "<https://example.org/p>",
                "o",
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
    }
    let options = EvalOptions::default().with_max_stored_facts(4);
    let error = evaluate_guarded(&program, input.clone(), &NoGuards, &options, None).unwrap_err();
    let EvalError::BudgetExhausted { resource, report } = error else {
        panic!("fact refusal")
    };
    assert_eq!(resource, BudgetResource::StoredFacts);
    assert_eq!(report.stored_facts(), 4);
    assert_eq!(report.observed_stored_facts(), 5);
    assert_eq!(resource.observed(report), 5);
    let error = chase_with(&rules, input.clone(), &options, None).unwrap_err();
    let ChaseError::BudgetExhausted { resource, report } = error else {
        panic!("chase fact refusal")
    };
    assert_eq!(resource, BudgetResource::StoredFacts);
    assert_eq!(report.stored_facts(), 4);
    assert_eq!(report.observed_stored_facts(), 5);
    let exact = options.with_max_stored_facts(9);
    let model = evaluate_guarded(&program, input, &NoGuards, &exact, None).unwrap();
    assert_eq!(model.facts().row_count(), 9);
    assert_eq!(
        model.work().committed_facts(),
        6,
        "duplicate heads do not count twice"
    );
}

fn interrupted_guarded_commit_counts_only_completed_generating_rounds() {
    struct Mint;
    impl GuardEvaluator for Mint {
        fn evaluate(
            &self,
            call: &GuardCall<'_>,
        ) -> Result<GuardRows, purrdf_datalog::guard::CallbackError> {
            call.evaluate_lexical(|inputs| Ok(vec![vec![format!("minted-{}", inputs[0]).into()]]))
        }
    }
    let program = compile(vec![
        DlClause::datalog(
            ClauseAtom::positive(
                ClauseTerm::var("?s"),
                "https://example.org/q",
                ClauseTerm::var("?minted"),
            ),
            vec![ClauseAtom::positive(
                ClauseTerm::var("?s"),
                "https://example.org/p",
                ClauseTerm::var("?o"),
            )],
        )
        .with_guards(vec![Guard::assign("mint", vec!["?s".into()], "?minted")]),
    ])
    .unwrap();
    let mut input = RelationStore::new();
    for subject in ["a", "b", "c"] {
        input
            .insert(
                subject,
                "<https://example.org/p>",
                "o",
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
    }
    let error = evaluate_guarded(
        &program,
        input,
        &Mint,
        &EvalOptions::default().with_max_stored_facts(4),
        None,
    )
    .unwrap_err();
    let seminaive::EvalError::BudgetExhausted { resource, report } = error else {
        panic!("fact admission refusal")
    };
    assert_eq!(resource, seminaive::BudgetResource::StoredFacts);
    assert_eq!(report.stored_facts(), 4);
    assert_eq!(report.observed_stored_facts(), 5);
    assert_eq!(
        report.term_generating_rounds(),
        0,
        "the interrupted round did not complete"
    );
    assert!(
        report.generated_terms() > 0,
        "the admitted prefix minted terms"
    );
}

fn public_sequential_execution_preserves_models_proofs_and_counters() {
    use purrdf_datalog::seminaive::RoundExecution;

    let atom = |predicate: &str| {
        ClauseAtom::positive(ClauseTerm::var("?s"), predicate, ClauseTerm::var("?o"))
    };
    let program = compile(vec![
        DlClause::datalog(
            atom("https://example.org/q"),
            vec![atom("https://example.org/p")],
        ),
        DlClause::datalog(
            atom("https://example.org/r"),
            vec![atom("https://example.org/p")],
        ),
    ])
    .unwrap();
    let mut input = RelationStore::new();
    input
        .insert(
            "a",
            "<https://example.org/p>",
            "b",
            RelationStore::DEFAULT_GRAPH,
        )
        .unwrap();
    let options = EvalOptions::default().with_derivation_mode(DerivationMode::Record);
    let parallel = evaluate_guarded(&program, input.clone(), &NoGuards, &options, None).unwrap();
    let sequential = evaluate_guarded(
        &program,
        input,
        &NoGuards,
        &options.with_round_execution(RoundExecution::Sequential),
        None,
    )
    .unwrap();
    assert_eq!(
        parallel.facts().facts_sorted(),
        sequential.facts().facts_sorted()
    );
    assert_eq!(parallel.derivations(), sequential.derivations());
    assert_eq!(parallel.budget(), sequential.budget());
    assert_eq!(parallel.work(), sequential.work());
}

fn predicate_rows_keep_native_identities_and_lexical_graph_order() {
    use purrdf_datalog::store::TermInterner;

    let mut triples = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    triples.insert_row(["s", "p", "o"]).unwrap();
    triples.insert_row(["s", "q", "o"]).unwrap();
    let predicate = triples.term_id("p").unwrap();
    let selected = triples.rows_for_predicate(predicate).collect::<Vec<_>>();
    assert_eq!(
        selected,
        triples
            .rows_from(0)
            .filter(|(_, row)| row[1] == predicate)
            .collect::<Vec<_>>()
    );
    assert_eq!(selected.len(), 1);

    let mut quads = RelationStore::default();
    for (predicate, graph) in [("p", "z"), ("q", "b"), ("p", "a"), ("r", "c"), ("p", "m")] {
        quads.insert("s", predicate, "o", graph).unwrap();
    }
    let predicate = quads.term_id("p").unwrap();
    let selected = quads.rows_for_predicate(predicate).collect::<Vec<_>>();
    assert_eq!(selected.len(), 3);
    assert_eq!(
        selected
            .iter()
            .map(|(_, row)| quads.interner().resolve(row[3]))
            .collect::<Vec<_>>(),
        ["a", "m", "z"]
    );
    for (row_id, row) in selected {
        assert_eq!(
            quads.rows_from(row_id.address().unwrap()).next(),
            Some((row_id, row))
        );
    }
}

fn late_delta_seed_completes_under_a_prefix_independent_work_limit() {
    use purrdf_datalog::seminaive::RoundExecution;
    let atom = |subject: &str, predicate: &str, object: &str| {
        ClauseAtom::positive(
            ClauseTerm::var(subject),
            format!("https://example.org/{predicate}"),
            ClauseTerm::var(object),
        )
    };
    let program = compile(vec![
        DlClause::datalog(atom("?m", "q", "?o"), vec![atom("?m", "seed", "?o")]),
        DlClause::datalog(
            atom("?s", "out", "?o"),
            vec![atom("?s", "p", "?m"), atom("?m", "q", "?o")],
        ),
    ])
    .unwrap();
    for count in [100, 1000] {
        let mut input = RelationStore::default();
        for index in 0..count {
            input
                .insert(
                    &format!("s{index}"),
                    "<https://example.org/p>",
                    &format!("m{index}"),
                    "",
                )
                .unwrap();
        }
        input
            .insert("m0", "<https://example.org/seed>", "end", "")
            .unwrap();
        let evaluation = evaluate_guarded(
            &program,
            input,
            &NoGuards,
            &EvalOptions::default()
                .with_max_join_steps(10)
                .with_round_execution(RoundExecution::Sequential),
            None,
        )
        .unwrap();
        assert!(
            evaluation
                .facts()
                .contains("s0", "<https://example.org/out>", "end", "")
        );
        assert_eq!(evaluation.facts().row_count(), count + 3);
        assert_eq!(evaluation.work().join_steps(), 3);
    }
}

fn narrow_triple_recursion_preserves_models_proofs_and_linear_candidate_work() {
    use purrdf_datalog::store::TermInterner;
    const LABEL: &str = "https://example.org/label";
    const EDGE: &str = "https://example.org/edge";
    let label = |subject: &str| {
        ClauseAtom::<3>::triple(
            ClauseTerm::var(subject),
            ClauseTerm::iri(LABEL),
            ClauseTerm::iri("https://example.org/marked"),
        )
    };
    let program = compile(vec![DlClause::datalog(
        label("?y"),
        vec![
            label("?x"),
            ClauseAtom::<3>::triple(
                ClauseTerm::var("?x"),
                ClauseTerm::iri(EDGE),
                ClauseTerm::var("?y"),
            ),
        ],
    )])
    .unwrap();
    for depth in [32, 64, 128] {
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        input
            .insert_row(["n0", &format!("<{LABEL}>"), "<https://example.org/marked>"])
            .unwrap();
        for index in 0..depth {
            input
                .insert_row([
                    &format!("n{index}"),
                    &format!("<{EDGE}>"),
                    &format!("n{}", index + 1),
                ])
                .unwrap();
        }
        let recorded = evaluate_guarded(
            &program,
            input.clone(),
            &NoGuards,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .unwrap();
        let model =
            evaluate_guarded(&program, input, &NoGuards, &EvalOptions::default(), None).unwrap();
        assert_eq!(
            model.facts().facts_sorted(),
            recorded.facts().facts_sorted()
        );
        assert_eq!(model.work(), recorded.work());
        assert_eq!(model.facts().row_count(), 2 * depth + 1);
        assert_eq!(model.work().join_steps(), (2 * depth + 1) as u64);
        assert!(model.derivations().is_none());
        assert_eq!(recorded.derivations().unwrap().len(), depth);
        let predicate = model.facts().term_id(&format!("<{LABEL}>")).unwrap();
        assert_eq!(
            model.facts().rows_for_predicate(predicate).count(),
            depth + 1
        );
        assert!(
            recorded
                .derivations()
                .unwrap()
                .iter()
                .all(|derivation| derivation.sources().len() == 2)
        );
    }
}

fn conjunctive_heads_share_joins_and_check_the_authored_head() {
    use purrdf_datalog::clause::HeadDisjunct;
    use purrdf_datalog::proof::{EvaluationProofs, ProofArena, ProofContext, ProofError};
    use purrdf_datalog::store::TermInterner;

    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::triple(
            ClauseTerm::var(s),
            ClauseTerm::literal(p),
            ClauseTerm::var(o),
        )
    };
    let body = vec![atom("?s", "p", "?m"), atom("?m", "q", "?o")];
    let heads = vec![
        atom("?s", "a", "?o"),
        atom("?s", "b", "?o"),
        atom("?s", "c", "?o"),
        atom("?s", "a", "?o"),
    ];
    let rules = vec![DlClause::new(
        vec![HeadDisjunct::new(heads.clone())],
        Vec::new(),
        body.clone(),
    )];
    let shared = compile(rules.clone()).unwrap();
    let split = compile(
        heads
            .into_iter()
            .map(|head| DlClause::datalog(head, body.clone()))
            .collect(),
    )
    .unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for index in 0..16 {
        let s = format!("s{index}");
        let m = format!("m{index}");
        input.insert_row([&s, "p", &m]).unwrap();
        input.insert_row([&m, "q", "o"]).unwrap();
    }
    let options = EvalOptions::default().with_derivation_mode(DerivationMode::Record);
    let result = evaluate_guarded(&shared, input.clone(), &NoGuards, &options, None).unwrap();
    let reference = evaluate_guarded(&split, input.clone(), &NoGuards, &options, None).unwrap();
    assert_eq!(
        result.facts().facts_sorted(),
        reference.facts().facts_sorted()
    );
    assert_eq!(result.work().committed_facts(), 48);
    assert!(result.work().join_steps() < reference.work().join_steps());
    let proofs = EvaluationProofs::of(&result).unwrap();
    let context = ProofContext::new(&rules, &input, result.facts());
    for derivation in result.derivations().unwrap() {
        let head = derivation.head();
        assert!(head < 3, "the first repeated head wins canonically");
        let root = proofs.root_for(derivation.fact()).unwrap();
        assert_eq!(proofs.arena().head(root), Some(head));
        assert_eq!(
            proofs.arena().check(root, &context).unwrap(),
            *derivation.fact()
        );
        let bytes = proofs.arena().encode(root);
        let (mut decoded, root) = ProofArena::<3>::decode(&bytes).unwrap();
        assert_eq!(decoded.encode(root), bytes);
        assert_eq!(decoded.head(root), Some(head));
        let premises = decoded.premises(root).to_vec();
        let wrong = decoded
            .by_rule_head(derivation.fact().clone(), 0, (head + 1) % 3, &premises)
            .unwrap();
        assert!(matches!(
            decoded.check(wrong, &context),
            Err(ProofError::GoalMismatch { .. })
        ));
        let missing = decoded
            .by_rule_head(derivation.fact().clone(), 0, 4, &premises)
            .unwrap();
        assert!(matches!(
            decoded.check(missing, &context),
            Err(ProofError::UnknownHead {
                rule: 0,
                head: 4,
                heads: 4
            })
        ));
    }
    let refused = evaluate_guarded(
        &shared,
        input,
        &NoGuards,
        &options.with_max_stored_facts(79),
        None,
    )
    .unwrap_err();
    let seminaive::EvalError::BudgetExhausted { report, .. } = refused else {
        panic!("expected exact per-head fact admission")
    };
    assert_eq!(report.stored_facts(), 79);
    assert_eq!(report.observed_stored_facts(), 80);
}

fn conjunctive_recursion_and_factor_projection_match_split_programs() {
    use purrdf_datalog::clause::HeadDisjunct;
    use purrdf_datalog::proof::{EvaluationProofs, ProofContext};
    use purrdf_datalog::store::TermInterner;
    let atom = |s: &str, p: &str, o: &str| {
        ClauseAtom::triple(
            ClauseTerm::var(s),
            ClauseTerm::literal(p),
            ClauseTerm::var(o),
        )
    };
    let cases = [
        (
            vec![atom("?s", "path", "?o"), atom("?s", "mark", "?o")],
            vec![atom("?s", "path", "?m"), atom("?m", "edge", "?o")],
            vec![["a", "path", "b"], ["b", "edge", "c"], ["c", "edge", "d"]],
        ),
        (
            vec![atom("?s", "left", "?s"), atom("?o", "right", "?o")],
            vec![atom("?s", "p", "?x"), atom("?o", "q", "?y")],
            vec![
                ["a", "p", "b"],
                ["c", "p", "d"],
                ["x", "q", "y"],
                ["z", "q", "w"],
            ],
        ),
    ];
    for (heads, body, rows) in cases {
        let rules = vec![DlClause::new(
            vec![HeadDisjunct::new(heads.clone())],
            Vec::new(),
            body.clone(),
        )];
        let program = compile(rules.clone()).unwrap();
        let split = compile(
            heads
                .into_iter()
                .map(|head| DlClause::datalog(head, body.clone()))
                .collect(),
        )
        .unwrap();
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        for row in rows {
            input.insert_row(row).unwrap();
        }
        let options = EvalOptions::default().with_derivation_mode(DerivationMode::Record);
        let actual = evaluate_guarded(&program, input.clone(), &NoGuards, &options, None).unwrap();
        let reference = evaluate_guarded(&split, input.clone(), &NoGuards, &options, None).unwrap();
        assert_eq!(
            actual.facts().facts_sorted(),
            reference.facts().facts_sorted()
        );
        let proofs = EvaluationProofs::of(&actual).unwrap();
        let context = ProofContext::new(&rules, &input, actual.facts());
        for (fact, root) in proofs.roots() {
            assert_eq!(proofs.arena().check(*root, &context).unwrap(), *fact);
        }
    }
}

fn conjunctive_heads_settle_at_each_predicates_actual_stratum() {
    use purrdf_datalog::clause::HeadDisjunct;
    use purrdf_datalog::store::TermInterner;

    let atom = |p: &str| {
        ClauseAtom::triple(
            ClauseTerm::var("?s"),
            ClauseTerm::literal(p),
            ClauseTerm::var("?o"),
        )
    };
    let conjunction = DlClause::new(
        vec![HeadDisjunct::new(vec![atom("p"), atom("q")])],
        Vec::new(),
        vec![atom("seed")],
    );
    let rest = vec![
        DlClause::datalog(atom("r"), vec![atom("p")]),
        DlClause::datalog(atom("q"), vec![atom("seed"), atom("r").into_negated()]),
        DlClause::datalog(atom("out"), vec![atom("seed"), atom("q").into_negated()]),
    ];
    let mut rules = vec![conjunction];
    rules.extend(rest.clone());
    let program = compile(rules).unwrap();
    assert_eq!(
        program.stratum_head_predicates(0).collect::<Vec<_>>(),
        ["p", "r"]
    );
    assert_eq!(
        program.stratum_head_predicates(1).collect::<Vec<_>>(),
        ["q"]
    );
    assert_eq!(
        program.stratum_head_predicates(2).collect::<Vec<_>>(),
        ["out"]
    );
    let mut split = vec![
        DlClause::datalog(atom("p"), vec![atom("seed")]),
        DlClause::datalog(atom("q"), vec![atom("seed")]),
    ];
    split.extend(rest);
    let split = compile(split).unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    input.insert_row(["a", "seed", "b"]).unwrap();
    let actual = seminaive::evaluate(&program, input.clone()).unwrap();
    let reference = seminaive::evaluate(&split, input).unwrap();
    assert_eq!(
        actual.facts().facts_sorted(),
        reference.facts().facts_sorted()
    );
    assert!(!actual.facts().contains_row(["a", "out", "b"]));
}

fn triple_pattern(subject: &str, predicate: &str, object: &str) -> ClauseAtom<3> {
    ClauseAtom::triple(
        ClauseTerm::var(subject),
        ClauseTerm::literal(predicate),
        ClauseTerm::var(object),
    )
}

fn native_guard_identities_preserve_recursive_cycle_sources_at_every_barrier() {
    use purrdf_datalog::clause::BodyLiteral::{Atom, Guard as Compute};
    use purrdf_datalog::dictionary::DictionaryValue;
    use purrdf_datalog::id::TermId;
    use purrdf_datalog::store::TermInterner;
    struct Choose(TermId);
    impl GuardEvaluator<TermInterner, 3> for Choose {
        fn evaluate(
            &self,
            _: &GuardCall<'_, TermInterner, 3>,
        ) -> Result<GuardRows, purrdf_datalog::guard::CallbackError> {
            Ok(vec![
                vec![DictionaryValue::Id(self.0)],
                vec!["absent-key".into()],
            ])
        }
    }
    for prefix in [0, 1, 3] {
        let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
        let mut body = Vec::new();
        let mut expected = Vec::new();
        match prefix {
            0 => {}
            1 => {
                input.insert_row(["a", "seed", "a"]).unwrap();
                body.push(Atom(triple_pattern("?s", "seed", "?x")));
                expected.push("seed");
            }
            3 => {
                for row in [["a", "f", "m"], ["m", "g", "n"], ["n", "h", "a"]] {
                    input.insert_row(row).unwrap();
                }
                body.extend([
                    Atom(triple_pattern("?s", "f", "?m")),
                    Atom(triple_pattern("?m", "g", "?n")),
                    Atom(triple_pattern("?n", "h", "?s")),
                ]);
                expected.extend(["f", "g", "h"]);
            }
            _ => unreachable!(),
        }
        body.push(Compute(Guard::assign("choose", Vec::new(), "?k")));
        body.extend([
            Atom(triple_pattern("?k", "out", "?y")),
            Atom(triple_pattern("?y", "q", "?z")),
            Atom(triple_pattern("?z", "r", "?k")),
        ]);
        expected.extend(["out", "q", "r"]);
        for row in [
            ["a", "out", "b"],
            ["b", "q", "c"],
            ["c", "q", "d"],
            ["d", "q", "e"],
            ["c", "r", "a"],
            ["d", "r", "a"],
            ["e", "r", "a"],
        ] {
            input.insert_row(row).unwrap();
        }
        let choose = Choose(input.interner().lookup("a").unwrap());
        let head = triple_pattern(if prefix == 0 { "?k" } else { "?s" }, "out", "?z");
        let program = compile(vec![
            DlClause::datalog(head, Vec::new()).with_ordered_body(body),
        ])
        .unwrap();
        assert!(program.rule_entry(0).1.has_cyclic_subplan());
        let result = evaluate_guarded(
            &program,
            input,
            &choose,
            &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
            None,
        )
        .unwrap();
        assert_eq!(result.work().committed_facts(), 3, "prefix {prefix}");
        for value in ["b", "c", "d", "e"] {
            assert!(
                result.facts().contains_row(["a", "out", value]),
                "prefix {prefix}: {value}"
            );
        }
        assert!(result.facts().interner().lookup("absent-key").is_none());
        let derivations = result.derivations().unwrap();
        assert_eq!(derivations.len(), 3);
        for derivation in derivations {
            assert_eq!(derivation.head(), 0);
            assert_eq!(
                derivation
                    .sources()
                    .iter()
                    .map(|fact| fact.predicate.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(derivation.sources()[prefix].subject, "a");
            let previous = match derivation.fact().object.as_str() {
                "c" => "b",
                "d" => "c",
                "e" => "d",
                value => panic!("unexpected {value}"),
            };
            assert_eq!(derivation.sources()[prefix].object, previous);
        }
    }
}

fn native_integer_columns_and_indexes_preserve_wide_original_handles() {
    use purrdf_datalog::store::{RowColumns, RowIndex, RowIndexError};
    let mut rows = RowColumns::<u32, 6, u64>::default();
    let first = [0, 1, 2, 3, u32::MAX, 5];
    let second = [u32::MAX, 6, 7, 8, 9, 10];
    let third = [0, 99, 98, 97, u32::MAX, 96];
    let wide_handle = 1_u64 << 50;
    for (row, handle) in [(first, wide_handle), (second, 7), (third, wide_handle + 17)] {
        rows.try_push(row, handle).unwrap();
    }
    let index = RowIndex::new(&rows, &[4, 0]).unwrap();
    assert_eq!(
        index.select(&[u32::MAX, 0]).unwrap().collect::<Vec<_>>(),
        [(first, wide_handle), (third, wide_handle + 17)],
    );
    assert_eq!(index.select(&[9]).unwrap().next_back(), Some((second, 7)));
    assert_eq!(
        index.positions(&[0, 0, 0]),
        Err(RowIndexError::PrefixTooLong {
            provided: 3,
            indexed: 2
        }),
    );
    assert!(matches!(
        RowIndex::new(&rows, &[6]),
        Err(RowIndexError::ColumnOutOfBounds {
            column: 6,
            width: 6
        })
    ));
    let mut wide = RowColumns::<u32, 65, u64>::default();
    let cells = core::array::from_fn(|column| u32::try_from(column).unwrap());
    wide.try_push(cells, wide_handle).unwrap();
    let index = RowIndex::new(&wide, &[64, 0]).unwrap();
    assert_eq!(
        index.select(&[64, 0]).unwrap().next(),
        Some((cells, wide_handle))
    );
}

fn ordered_guard_outputs_probe_later_atoms_without_intermediate_relations() {
    use purrdf_datalog::clause::BodyLiteral::{Atom, Guard as Compute};
    use purrdf_datalog::store::TermInterner;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Keys {
        keys: AtomicUsize,
        labels: AtomicUsize,
    }
    impl GuardEvaluator<TermInterner, 3> for Keys {
        fn evaluate(
            &self,
            call: &GuardCall<'_, TermInterner, 3>,
        ) -> Result<GuardRows, purrdf_datalog::guard::CallbackError> {
            call.evaluate_lexical(|inputs| match call.guard.name() {
                "place" => {
                    self.keys.fetch_add(1, Ordering::Relaxed);
                    assert_eq!(inputs, ["module", "name"]);
                    Ok(vec![vec!["module/name".into()], vec!["missing".into()]])
                }
                "label" => {
                    self.labels.fetch_add(1, Ordering::Relaxed);
                    Ok(vec![vec![format!("label:{}", inputs[0]).into()]])
                }
                name => panic!("unexpected guard {name}"),
            })
        }
    }
    let program = compile(vec![
        DlClause::datalog(triple_pattern("?St", "out", "?Final"), Vec::new()).with_ordered_body(
            vec![
                Atom(triple_pattern("?St", "at", "?X")),
                Atom(triple_pattern("?St", "pstep", "?N")),
                Compute(Guard::assign("place", vec!["?X".into(), "?N".into()], "?P")),
                Atom(triple_pattern("?P", "pub", "?Dv")),
                Compute(Guard::assign("label", vec!["?Dv".into()], "?Label")),
                Atom(triple_pattern("?Label", "value", "?Final")),
            ],
        ),
    ])
    .unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for row in [
        ["step", "at", "module"],
        ["step", "pstep", "name"],
        ["module/name", "pub", "one"],
        ["module/name", "pub", "two"],
        ["label:one", "value", "first"],
        ["label:two", "value", "second"],
        ["unrelated", "pub", "wrong"],
        ["label:wrong", "value", "unexpected"],
    ] {
        input.insert_row(row).unwrap();
    }
    let keys = Keys::default();
    let result = evaluate_guarded(
        &program,
        input,
        &keys,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(result.work().committed_facts(), 2);
    assert!(result.facts().contains_row(["step", "out", "first"]));
    assert!(result.facts().contains_row(["step", "out", "second"]));
    assert!(!result.facts().contains_row(["step", "out", "wrong"]));
    assert!(!result.facts().contains_row(["step", "out", "unexpected"]));
    assert_eq!(keys.keys.load(Ordering::Relaxed), 1);
    assert_eq!(keys.labels.load(Ordering::Relaxed), 2);
    for derivation in result.derivations().unwrap() {
        assert_eq!(
            derivation
                .sources()
                .iter()
                .map(|source| source.predicate.as_str())
                .collect::<Vec<_>>(),
            ["at", "pstep", "pub", "value"],
            "guard barriers preserve authored positive provenance",
        );
    }
    let must_run = compile(vec![
        DlClause::datalog(triple_pattern("?St", "out", "?Dv"), Vec::new()).with_ordered_body(vec![
            Atom(triple_pattern("?St", "seed", "?X")),
            Compute(Guard::filter("must-run", vec!["?X".into()])),
            Atom(triple_pattern("?X", "absent-predicate", "?Dv")),
        ]),
    ])
    .unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    input.insert_row(["step", "seed", "key"]).unwrap();
    assert!(matches!(
        evaluate_guarded(&must_run, input, &NoGuards, &EvalOptions::default(), None),
        Err(seminaive::EvalError::Guard { guard, .. }) if guard == "must-run"
    ));
}

fn ordered_guard_outputs_constrain_certified_cycles_and_unknown_terms() {
    use purrdf_datalog::clause::BodyLiteral::{Atom, Guard as Compute};
    use purrdf_datalog::store::TermInterner;
    struct Choose;
    impl GuardEvaluator<TermInterner, 3> for Choose {
        fn evaluate(
            &self,
            call: &GuardCall<'_, TermInterner, 3>,
        ) -> Result<GuardRows, purrdf_datalog::guard::CallbackError> {
            use purrdf_datalog::dictionary::{DictionaryTerm, DictionaryValue};
            let DictionaryTerm::Id(id) = call.inputs[0] else {
                panic!("a stored binding must reach the guard as its original identity");
            };
            Ok(vec![
                vec![DictionaryValue::Id(id)],
                vec!["absent-key".into()],
            ])
        }
    }
    let program = compile(vec![
        DlClause::datalog(triple_pattern("?St", "out", "?Z"), Vec::new()).with_ordered_body(vec![
            Atom(triple_pattern("?St", "seed", "?X")),
            Compute(Guard::assign("choose", vec!["?X".into()], "?K")),
            Atom(triple_pattern("?K", "p", "?Y")),
            Atom(triple_pattern("?Y", "q", "?Z")),
            Atom(triple_pattern("?Z", "r", "?K")),
        ]),
        DlClause::datalog(triple_pattern("?St", "reach", "?Y"), Vec::new()).with_ordered_body(
            vec![
                Atom(triple_pattern("?St", "reach", "?X")),
                Compute(Guard::assign("choose", vec!["?X".into()], "?K")),
                Atom(triple_pattern("?K", "next", "?Y")),
            ],
        ),
    ])
    .unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for row in [
        ["step", "seed", "a"],
        ["a", "p", "b"],
        ["b", "q", "c"],
        ["c", "r", "a"],
        ["x", "p", "y"],
        ["y", "q", "z"],
        ["z", "r", "x"],
        ["step", "reach", "a"],
        ["a", "next", "b"],
        ["b", "next", "c"],
    ] {
        input.insert_row(row).unwrap();
    }
    let result = evaluate_guarded(&program, input, &Choose, &EvalOptions::default(), None).unwrap();
    assert_eq!(result.work().committed_facts(), 3);
    assert!(result.facts().contains_row(["step", "out", "c"]));
    assert!(!result.facts().contains_row(["step", "out", "z"]));
    assert!(result.facts().contains_row(["step", "reach", "b"]));
    assert!(result.facts().contains_row(["step", "reach", "c"]));
}

fn native_inequality_needs_no_callback_and_is_independently_checked() {
    use purrdf_datalog::cache::canonical_rule_hash;
    use purrdf_datalog::proof::{EvaluationProofs, ProofArena, ProofContext, ProofError};
    use purrdf_datalog::store::TermInterner;
    let atom = |p: &str| {
        ClauseAtom::triple(
            ClauseTerm::var("?s"),
            ClauseTerm::literal(p),
            ClauseTerm::var("?o"),
        )
    };
    let rule = DlClause::datalog(atom("out"), vec![atom("p")]);
    let rules = vec![rule.clone().with_guards(vec![Guard::not_equal("?s", "?o")])];
    let callback =
        vec![rule.with_guards(vec![Guard::filter("!=", vec!["?s".into(), "?o".into()])])];
    assert_ne!(canonical_rule_hash(&rules), canonical_rule_hash(&callback));
    // Both programs now contain a native guard and use the same identity domain.
    // The second guard's operation still distinguishes identical names/arguments.
    let mixed = |second| {
        rules[0]
            .clone()
            .with_guards(vec![Guard::not_equal("?s", "?o"), second])
    };
    assert_ne!(
        canonical_rule_hash(&[mixed(Guard::not_equal("?s", "?o"))]),
        canonical_rule_hash(&[mixed(Guard::filter("!=", vec!["?s".into(), "?o".into()]))]),
    );
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for row in [["a", "p", "b"], ["a", "p", "a"], ["01", "p", "1"]] {
        input.insert_row(row).unwrap();
    }
    let result = evaluate_guarded(
        &compile(rules.clone()).unwrap(),
        input.clone(),
        &NoGuards,
        &EvalOptions::default().with_derivation_mode(DerivationMode::Record),
        None,
    )
    .unwrap();
    assert_eq!(result.work().committed_facts(), 2);
    assert!(result.facts().contains_row(["a", "out", "b"]));
    assert!(result.facts().contains_row(["01", "out", "1"]));
    assert!(!result.facts().contains_row(["a", "out", "a"]));
    let context = ProofContext::new(&rules, &input, result.facts());
    let proofs = EvaluationProofs::of(&result).unwrap();
    for (fact, root) in proofs.roots() {
        assert_eq!(proofs.arena().check(*root, &context).unwrap(), *fact);
    }
    let equal = input
        .facts_sorted()
        .into_iter()
        .find(|fact| fact.subject == fact.object)
        .unwrap();
    let mut claimed = equal.clone();
    claimed.predicate = "out".into();
    let mut arena = ProofArena::<3>::default();
    let premise = arena.axiom(equal).unwrap();
    let invalid = arena.by_rule(claimed, 0, &[premise]).unwrap();
    let error = arena.check(invalid, &context).unwrap_err();
    assert!(
        matches!(&error, ProofError::GuardRejected { rule: 0, guard: 0, operation: purrdf_datalog::guard::GuardOperation::NotEqual, inputs } if inputs.as_ref() == ["a", "a"])
    );
    let fields = purrdf_testkit::error_capture::Capture::read(&error);
    assert_eq!(fields.code, "datalog-proof-guard-rejected");
    assert_eq!(fields.child("operation").code, "datalog-guard-not-equal");
    assert!(matches!(
        evaluate_guarded(
            &compile(callback).unwrap(),
            input,
            &NoGuards,
            &EvalOptions::default(),
            None,
        ),
        Err(seminaive::EvalError::Guard { .. })
    ));
}

fn native_inequality_in_a_negative_group_uses_the_same_filter() {
    use purrdf_datalog::guard::Negation;
    use purrdf_datalog::store::TermInterner;
    let atom = triple_pattern;
    let rule = DlClause::datalog(atom("?s", "out", "?o"), vec![atom("?s", "seed", "?o")])
        .with_negations(vec![Negation::new(
            vec![atom("?s", "p", "?other")],
            vec![Guard::not_equal("?other", "?o")],
        )]);
    let program = compile(vec![rule]).unwrap();
    let mut input = RelationStore::<TermInterner, 3>::with_dictionary(TermInterner::default());
    for row in [
        ["a", "seed", "a"],
        ["x", "seed", "y"],
        ["a", "p", "a"],
        ["a", "p", "b"],
        ["x", "p", "y"],
    ] {
        input.insert_row(row).unwrap();
    }
    let result = seminaive::evaluate(&program, input).unwrap();
    assert_eq!(result.work().committed_facts(), 1);
    assert!(!result.facts().contains_row(["a", "out", "a"]));
    assert!(result.facts().contains_row(["x", "out", "y"]));
}

purrdf_testkit::harness_main!(
    native_guard_identities_preserve_recursive_cycle_sources_at_every_barrier,
    native_integer_columns_and_indexes_preserve_wide_original_handles,
    narrow_triple_recursion_preserves_models_proofs_and_linear_candidate_work,
    late_delta_seed_completes_under_a_prefix_independent_work_limit,
    ordered_guard_outputs_constrain_certified_cycles_and_unknown_terms,
    ordered_guard_outputs_probe_later_atoms_without_intermediate_relations,
    native_inequality_needs_no_callback_and_is_independently_checked,
    native_inequality_in_a_negative_group_uses_the_same_filter,
    conjunctive_recursion_and_factor_projection_match_split_programs,
    conjunctive_heads_share_joins_and_check_the_authored_head,
    conjunctive_heads_settle_at_each_predicates_actual_stratum,
    predicate_rows_keep_native_identities_and_lexical_graph_order,
    interrupted_guarded_commit_counts_only_completed_generating_rounds,
    model_only_and_recorded_models_have_explicit_evidence_and_equal_counts,
    term_arena_limits_are_caller_selected_and_reach_contracts,
    stored_fact_admission_refuses_the_first_extra_fact_in_both_engines,
    public_sequential_execution_preserves_models_proofs_and_counters,
    triple_base_authority_survives_data_growth_and_layer_retraction,
    triple_negated_base_preserves_seminaive_work_and_limits,
    triple_no_output_runs_the_body_without_fabricating_a_fact,
    triple_store_has_predicate_only_keys_and_transactional_original_ids,
    triple_compiler_preserves_static_arity_and_original_admission,
    public_host_guarded_evidence_fixpoint_retains_ids_and_first_source_error,
    later_partial_delta_keeps_both_productive_anchors,
    original_guarded_pair_has_exact_connected_work_and_authored_proofs,
    connected_rule_restores_all_authored_premises,
    independent_head_projects_without_cartesian_work,
    recursive_factor_preserves_authored_proofs_and_typed_limits
);

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original term and proof refusal data through the portable public API.

use purrdf_datalog::term::{RetainedTerm, TermDag};
use purrdf_hash::error::ErrorValue;
use purrdf_testkit::error_capture::Capture;

fn structural_answers_share_terms_and_variables_after_producer_destruction() {
    use purrdf_datalog::resolve_fol::{
        FolAnswerContract, FolBudget, FolClause, FolControl, FolProgram, check_fol_proof,
        resolve_fol,
    };
    use purrdf_datalog::term::NodeData;
    use purrdf_datalog::unify::SortContext;
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let p = dag.intern_leaf("p").unwrap();
    let f = dag.intern_leaf("f").unwrap();
    let a = dag
        .intern_rdf(purrdf_core::TermValue::simple_literal("a"))
        .unwrap();
    let shared = dag.intern_app(f, vec![a, a]).unwrap();
    let one = dag.intern_app(p, vec![shared, shared]).unwrap();
    let two = dag.intern_app(p, vec![shared, a]).unwrap();
    let (_, x) = dag.fresh_meta().unwrap();
    let (_, y) = dag.fresh_meta().unwrap();
    let goal = dag.intern_app(p, vec![x, y]).unwrap();
    let clauses = [one, two]
        .into_iter()
        .map(|head| FolClause::new(&dag, head, vec![]).unwrap())
        .collect();
    let program = FolProgram::new(
        &dag,
        clauses,
        goal,
        vec![(x, "X".into()), (x, "alias".into()), (y, "Y".into())],
        BTreeMap::new(),
    )
    .unwrap();
    let FolControl::Decided(outcome) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("ground facts must be decided");
    };
    assert_eq!(outcome.answers.len(), 2);
    let answer = outcome
        .answers
        .iter()
        .find(|row| row.get("Y").unwrap().source_id() == shared)
        .unwrap()
        .clone();
    let other = outcome
        .answers
        .iter()
        .find(|row| row.get("Y").unwrap().source_id() == a)
        .unwrap()
        .clone();
    assert!(answer.get("X").unwrap().same_node(answer.get("Y").unwrap()));
    assert!(answer.get("X").unwrap().same_node(other.get("X").unwrap()));
    let variables: Vec<_> = answer
        .bindings()
        .map(purrdf_datalog::resolve_fol::FolBindingEntry::variable)
        .collect();
    assert!(variables[0].same_node(variables[1]));
    assert!(!variables[0].same_node(variables[2]));
    assert!(variables[0].same_node(other.bindings().next().unwrap().variable()));
    assert!(matches!(variables[0].data(), NodeData::Meta(_)));
    let FolControl::Decided(other_scope) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("the repeated query must also decide");
    };
    assert!(!variables[0].same_node(other_scope.answers[0].bindings().next().unwrap().variable()));
    drop(other_scope);
    let checked = check_fol_proof(
        &mut dag,
        answer.proof(),
        program.clauses(),
        program.meta_sorts(),
        &SortContext::default(),
        &|_, _| false,
    )
    .unwrap();
    assert!(answer.term(checked).unwrap().same_node(answer.atom()));
    let identity = answer.derivation_id();
    drop(outcome);
    drop(program);
    drop(dag);
    let clone = answer.clone();
    drop(answer);
    assert_eq!(clone.answer_contract(), FolAnswerContract::GroundOnly);
    assert_eq!(clone.derivation_id(), identity);
    let term = clone.get("X").unwrap();
    let NodeData::App { args, .. } = term.data() else {
        panic!("the compound answer stays structural");
    };
    assert!(
        term.term(args[0])
            .unwrap()
            .same_node(term.term(args[1]).unwrap())
    );
    assert_eq!(term.render(), "f(\"a\", \"a\")");
}

fn rdf_answers_retain_original_literal_blank_and_quoted_identity() {
    use purrdf_core::{BlankScope, RdfTextDirection, TermBox, TermValue};
    use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
    use purrdf_datalog::resolve_fol::{
        FolBudget, FolControl, FolError, FolRdfSource, solve_datalog_goal_rdf,
    };
    use purrdf_datalog::term::NodeData;
    use std::collections::BTreeMap;

    struct Source(BTreeMap<String, TermValue>);
    impl FolRdfSource for Source {
        fn resolve(&self, identity: &str) -> Option<&TermValue> {
            self.0.get(identity)
        }
    }
    let blank = |scope| TermValue::Blank {
        label: "same".into(),
        scope: BlankScope(scope),
    };
    let directed = TermValue::Literal {
        lexical_form: "same\nquoted\"".into(),
        datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString".into(),
        language: Some("ar".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let quoted = TermValue::Triple {
        s: TermBox::new(blank(1)),
        p: TermBox::new(TermValue::iri("urn:p")),
        o: TermBox::new(directed.clone()),
    };
    // These are original dictionary addresses, deliberately not parsed RDF or
    // human displays. Only that source's native values enter the resolver.
    let mut source = Source(BTreeMap::from([
        ("<urn:p>".into(), TermValue::iri("urn:p")),
        ("blank one".into(), blank(1)),
        ("blank two".into(), blank(2)),
        ("directed".into(), directed.clone()),
        ("quoted".into(), quoted.clone()),
        ("plain".into(), TermValue::simple_literal("same\nquoted\"")),
    ]));
    let clauses = [
        ("blank one", "directed"),
        ("blank two", "directed"),
        ("blank one", "quoted"),
        ("blank one", "plain"),
    ]
    .into_iter()
    .map(|(s, o)| {
        DlClause::datalog(
            ClauseAtom::positive(ClauseTerm::literal(s), "urn:p", ClauseTerm::literal(o)),
            vec![],
        )
    })
    .collect::<Vec<_>>();
    let goal = ClauseAtom::positive(ClauseTerm::var("S"), "urn:p", ClauseTerm::var("O"));
    let budget = FolBudget { max_steps: 1000 };
    let (dag, control) = solve_datalog_goal_rdf(&clauses, &goal, &budget, &source).unwrap();
    let FolControl::Decided(outcome) = control else {
        panic!("native ground RDF facts must be decided");
    };
    assert_eq!(outcome.answers.len(), 4);
    let native = |term: purrdf_datalog::resolve_fol::FolTerm<'_>| match term.data() {
        NodeData::Rdf(value) => value.value().clone(),
        _ => panic!("RDF inputs must retain native components"),
    };
    assert!(
        outcome
            .answers
            .iter()
            .any(|row| native(row.get("S").unwrap()) == blank(1))
    );
    assert!(
        outcome
            .answers
            .iter()
            .any(|row| native(row.get("S").unwrap()) == blank(2))
    );
    assert!(
        outcome
            .answers
            .iter()
            .any(|row| native(row.get("O").unwrap()) == directed)
    );
    let retained = outcome
        .answers
        .iter()
        .find(|row| native(row.get("O").unwrap()) == quoted)
        .unwrap()
        .clone();
    let key = retained.get("O").unwrap().canonical_key();
    source.0.remove("directed");
    let error = solve_datalog_goal_rdf(&clauses, &goal, &budget, &source).unwrap_err();
    let FolError::Rdf(cause) = &error else {
        panic!("a missing original operand is a hard typed failure");
    };
    assert_eq!(cause.identity(), "directed");
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .is::<purrdf_datalog::resolve_fol::FolRdfError>()
    );
    drop(source);
    drop(outcome);
    drop(dag);
    assert_eq!(native(retained.get("O").unwrap()), quoted);
    assert_eq!(retained.get("O").unwrap().canonical_key(), key);
}

fn negative_only_demand_observes_asserted_dependency() {
    use purrdf_datalog::resolve_fol::{
        FolBudget, FolClause, FolControl, FolLit, FolProgram, FolStatus, FolUnestablished,
        FolVerdict, Truth, check_fol_proof, resolve_fol,
    };
    use purrdf_datalog::unify::SortContext;
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let a = dag.intern_leaf("a").unwrap();
    let p = dag.intern_leaf("p").unwrap();
    let q = dag.intern_leaf("q").unwrap();
    let pa = dag.intern_app(p, vec![a]).unwrap();
    let qa = dag.intern_app(q, vec![a]).unwrap();
    let untouched = dag.intern_leaf("untouched").unwrap();
    let (_, variable) = dag.fresh_meta().unwrap();
    let clauses = vec![
        FolClause::new(&dag, qa, vec![]).unwrap(),
        FolClause::new(&dag, pa, vec![FolLit::Neg(qa)]).unwrap(),
        FolClause::new(&dag, untouched, vec![]).unwrap(),
    ];
    let program = FolProgram::new(&dag, clauses, pa, vec![], BTreeMap::new()).unwrap();
    let FolControl::Decided(outcome) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        purrdf_datalog::resolve_fol::FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("ground negative demands must be decided");
    };
    assert_eq!(outcome.status(), FolStatus::Complete);
    assert_eq!(
        outcome.truth_of(&dag, pa),
        FolVerdict::Established(Truth::False)
    );
    assert_eq!(
        outcome.truth_of(&dag, qa),
        FolVerdict::Established(Truth::True)
    );
    assert_eq!(
        outcome.answers,
        [] as [purrdf_datalog::resolve_fol::FolBinding; 0]
    );
    assert_eq!(
        outcome.truth_of(&dag, untouched),
        FolVerdict::NotEstablished(FolUnestablished::Unevaluated)
    );
    assert_eq!(
        outcome.truth_of(&dag, variable),
        FolVerdict::NotEstablished(FolUnestablished::NonGround)
    );

    let FolControl::Decided(partial) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        purrdf_datalog::resolve_fol::FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 1 },
    )
    .unwrap() else {
        panic!("budget interruption must retain a partial outcome");
    };
    assert_eq!(partial.status(), FolStatus::Partial);
    assert_eq!(
        partial.truth_of(&dag, pa),
        FolVerdict::NotEstablished(FolUnestablished::Incomplete)
    );
    assert_eq!(
        partial.truth_of(&dag, qa),
        FolVerdict::NotEstablished(FolUnestablished::Incomplete)
    );
    assert_eq!(
        partial.answers,
        [] as [purrdf_datalog::resolve_fol::FolBinding; 0]
    );

    // The neighboring program with no q fact establishes its demanded absence.
    let absent_program = FolProgram::new(
        &dag,
        vec![FolClause::new(&dag, pa, vec![FolLit::Neg(qa)]).unwrap()],
        pa,
        vec![],
        BTreeMap::new(),
    )
    .unwrap();
    let FolControl::Decided(absent) = resolve_fol(
        &mut dag,
        &absent_program,
        &SortContext::default(),
        purrdf_datalog::resolve_fol::FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("a completed missing dependency must be decided");
    };
    assert_eq!(
        absent.truth_of(&dag, qa),
        FolVerdict::Established(Truth::False)
    );
    assert_eq!(
        absent.truth_of(&dag, pa),
        FolVerdict::Established(Truth::True)
    );
    assert_eq!(absent.answers.len(), 1);
    check_fol_proof(
        &mut dag,
        absent.answers[0].proof(),
        absent_program.clauses(),
        absent_program.meta_sorts(),
        &SortContext::default(),
        &|dag, node| absent.truth_of(dag, node) != FolVerdict::Established(Truth::False),
    )
    .unwrap();
}

fn indirect_negative_demands_reach_facts_and_preserve_cycles() {
    use purrdf_datalog::resolve_fol::{
        FolBudget, FolClause, FolControl, FolLit, FolProgram, FolStatus, FolVerdict, Truth,
        resolve_fol,
    };
    use purrdf_datalog::unify::SortContext;
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let p = dag.intern_leaf("p").unwrap();
    let q = dag.intern_leaf("q").unwrap();
    let r = dag.intern_leaf("r").unwrap();
    for (dependency, expected) in [
        (FolLit::Pos(r), Truth::False),
        (FolLit::Neg(r), Truth::True),
        (FolLit::Neg(p), Truth::Undefined),
    ] {
        let clauses = vec![
            FolClause::new(&dag, p, vec![FolLit::Neg(q)]).unwrap(),
            FolClause::new(&dag, q, vec![dependency]).unwrap(),
            FolClause::new(&dag, r, vec![]).unwrap(),
        ];
        let program = FolProgram::new(&dag, clauses, p, vec![], BTreeMap::new()).unwrap();
        let FolControl::Decided(outcome) = resolve_fol(
            &mut dag,
            &program,
            &SortContext::default(),
            purrdf_datalog::resolve_fol::FolAnswerContract::GroundOnly,
            &FolBudget { max_steps: 100 },
        )
        .unwrap() else {
            panic!("finite ground negative dependencies must be decided");
        };
        assert_eq!(outcome.status(), FolStatus::Complete);
        assert_eq!(outcome.truth_of(&dag, p), FolVerdict::Established(expected));
        assert_eq!(!outcome.answers.is_empty(), expected == Truth::True);
    }
}

fn fol_construction_checks_resident_handles_and_original_positions() {
    use purrdf_datalog::id::NodeId;
    use purrdf_datalog::resolve_fol::{
        FolClause, FolConstructionError, FolLit, FolPosition, FolProgram,
    };
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let head = dag.intern_leaf("head").unwrap();
    let unknown = NodeId::from_index(dag.len() as u64).unwrap();
    assert_eq!(
        FolClause::new(&dag, unknown, vec![]).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Head,
            clause: None,
            nodes: dag.len(),
        }
    );
    assert_eq!(
        FolClause::new(&dag, head, vec![FolLit::Neg(unknown)]).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Body { literal: 0 },
            clause: None,
            nodes: dag.len(),
        }
    );
    let clause = FolClause::new(&dag, head, vec![FolLit::Pos(head)]).unwrap();
    assert_eq!(
        FolProgram::new(&dag, vec![clause.clone()], unknown, vec![], BTreeMap::new()).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::Goal,
            clause: None,
            nodes: dag.len(),
        }
    );

    assert_eq!(
        FolProgram::new(
            &dag,
            vec![clause.clone()],
            head,
            vec![(unknown, "original name".into())],
            BTreeMap::new()
        )
        .unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown,
            position: FolPosition::GoalVariable {
                index: 0,
                name: "original name".into()
            },
            clause: None,
            nodes: dag.len(),
        }
    );

    // Revalidation catches a previously valid clause supplied with a smaller arena.
    let empty = TermDag::default();
    assert_eq!(
        FolProgram::new(&empty, vec![clause.clone()], head, vec![], BTreeMap::new()).unwrap_err(),
        FolConstructionError::UnknownNode {
            node: head,
            position: FolPosition::Head,
            clause: Some(0),
            nodes: 0,
        }
    );
    FolProgram::new(&dag, vec![clause], head, vec![], BTreeMap::new()).unwrap();
}

fn fol_projection_validation_preserves_names_and_admits_aliases() {
    use purrdf_datalog::resolve_fol::{
        FolBudget, FolClause, FolConstructionError, FolControl, FolProgram, resolve_fol,
    };
    use purrdf_datalog::unify::SortContext;
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let pred = dag.intern_leaf("p").unwrap();
    let value = dag.intern_leaf("value").unwrap();
    let (_, var) = dag.fresh_meta().unwrap();
    let (_, other) = dag.fresh_meta().unwrap();
    let goal = dag.intern_app(pred, vec![var]).unwrap();
    let asserted = dag.intern_app(pred, vec![value]).unwrap();
    let fact = FolClause::new(&dag, asserted, vec![]).unwrap();
    let construct =
        |projection| FolProgram::new(&dag, vec![fact.clone()], goal, projection, BTreeMap::new());
    assert_eq!(
        construct(vec![(value, "constant".into())]).unwrap_err(),
        FolConstructionError::NotMetavariable {
            index: 0,
            node: value,
            name: "constant".into()
        }
    );
    assert_eq!(
        construct(vec![(other, "unrelated".into())]).unwrap_err(),
        FolConstructionError::OutsideGoal {
            index: 0,
            node: other,
            name: "unrelated".into(),
            goal
        }
    );
    let error = construct(vec![
        (var, "original name".into()),
        (var, "original name".into()),
    ])
    .unwrap_err();
    assert_eq!(
        error,
        FolConstructionError::DuplicateName {
            name: "original name".into(),
            first: 0,
            second: 1,
            first_node: var,
            second_node: var,
        }
    );
    let retained = error.clone();
    drop(error);
    assert!(matches!(
        Capture::read(&retained).field("name"),
        ErrorValue::Text("original name")
    ));

    let program = construct(vec![(var, "X".into()), (var, "alias".into())]).unwrap();
    let FolControl::Decided(outcome) = resolve_fol(
        &mut dag,
        &program,
        &SortContext::default(),
        purrdf_datalog::resolve_fol::FolAnswerContract::GroundOnly,
        &FolBudget { max_steps: 100 },
    )
    .unwrap() else {
        panic!("a ground fact must be decidable");
    };
    assert_eq!(outcome.answers.len(), 1);
    assert_eq!(outcome.answers[0].atom().source_id(), asserted);
    assert_eq!(outcome.answers[0].get("X").unwrap().render(), "value");
    assert_eq!(outcome.answers[0].get("alias").unwrap().render(), "value");
}

fn fol_sort_construction_checks_handles_without_rejecting_unused_declarations() {
    use purrdf_datalog::id::{MetaId, NodeId};
    use purrdf_datalog::resolve_fol::{FolConstructionError, FolPosition, FolProgram};
    use std::collections::BTreeMap;

    let mut dag = TermDag::default();
    let goal = dag.intern_leaf("goal").unwrap();
    let sort = dag.intern_leaf("sort").unwrap();
    let (unused, _) = dag.fresh_meta().unwrap();
    let unknown = MetaId::from_index(unused.index() + 1).unwrap();
    assert_eq!(
        FolProgram::new(
            &dag,
            vec![],
            goal,
            vec![],
            BTreeMap::from([(unknown, sort)])
        )
        .unwrap_err(),
        FolConstructionError::UnknownMetavariable {
            variable: unknown,
            sort
        }
    );
    let unknown_sort = NodeId::from_index(dag.len() as u64).unwrap();
    assert_eq!(
        FolProgram::new(
            &dag,
            vec![],
            goal,
            vec![],
            BTreeMap::from([(unused, unknown_sort)])
        )
        .unwrap_err(),
        FolConstructionError::UnknownNode {
            node: unknown_sort,
            position: FolPosition::Sort { variable: unused },
            clause: None,
            nodes: dag.len(),
        }
    );
    FolProgram::new(&dag, vec![], goal, vec![], BTreeMap::from([(unused, sort)])).unwrap();
}

fn reachable_snapshot_preserves_sharing_binders_and_deep_source_death() {
    let mut dag = TermDag::default();
    let op = dag.intern_leaf("operator").unwrap();
    let body = dag.intern_bound(u32::MAX, u16::MAX).unwrap();
    let binder = dag.intern_binder(op, vec![op], body).unwrap();
    let unreachable = dag.intern_leaf("unreachable").unwrap();
    let mut root = binder;
    for _ in 0..10_000 {
        root = dag.intern_app(op, vec![root, binder]).unwrap();
    }
    let snapshot = RetainedTerm::capture(&dag, root).unwrap();
    drop(dag);
    let clone = snapshot.clone();
    drop(snapshot);
    let captured = Capture::read(&clone);
    assert!(
        matches!(captured.field("root"), ErrorValue::Unsigned(value) if value == u128::from(root.index()))
    );
    let nodes = captured.records("nodes");
    assert_eq!(nodes.len(), 10_003);
    assert!(!nodes.iter().any(|node| matches!(node.field("id"), ErrorValue::Unsigned(value) if value == u128::from(unreachable.index()))));
    assert!(
        !nodes
            .iter()
            .any(|node| matches!(node.field("symbol"), ErrorValue::Text("unreachable")))
    );
    let bound = nodes
        .iter()
        .map(|node| node.child("data"))
        .find(|node| matches!(node.field("kind"), ErrorValue::Code("bound")))
        .unwrap();
    assert!(
        matches!(bound.field("debruijn"), ErrorValue::Unsigned(value) if value == u128::from(u32::MAX))
    );
    assert!(
        matches!(bound.field("slot"), ErrorValue::Unsigned(value) if value == u128::from(u16::MAX))
    );
}

fn actual_proof_decoder_preserves_rejected_bytes_and_native_lengths() {
    use purrdf_datalog::proof::{ProofArena, ProofMalformed};

    let error = ProofArena::<4>::decode(&[]).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    assert_eq!(fields.code, "proof-truncated");
    assert!(matches!(fields.field("required"), ErrorValue::ByteCount(8)));
    assert!(matches!(
        fields.field("remaining"),
        ErrorValue::ByteCount(0)
    ));
    let error = ProofArena::<4>::decode(&u64::MAX.to_le_bytes()).unwrap_err();
    let fields = Capture::read(&error).child("detail");
    if usize::BITS < 64 {
        assert_eq!(fields.code, "proof-length");
        assert!(
            matches!(fields.field("value"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(
            matches!(fields.field("maximum"), ErrorValue::Unsigned(value) if value == usize::MAX as u128)
        );
    } else {
        assert_eq!(fields.code, "proof-truncated");
        assert!(
            matches!(fields.field("required"), ErrorValue::ByteCount(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            fields.field("remaining"),
            ErrorValue::ByteCount(0)
        ));
    }
    let mut input = Vec::new();
    purrdf_hash::frame::frame_le(&mut input, &[0xff, 0, 0xab]);
    let error = ProofArena::<4>::decode(&input).unwrap_err();
    drop(input);
    let copy = error.clone();
    drop(error);
    let fields = Capture::read(&copy).child("detail");
    assert_eq!(fields.code, "proof-encoding-tag");
    assert!(matches!(fields.field("actual"), ErrorValue::Bytes(bytes) if bytes == [0xff, 0, 0xab]));
    assert!(
        std::error::Error::source(&copy)
            .unwrap()
            .is::<ProofMalformed>()
    );
}

purrdf_testkit::harness_main!(
    structural_answers_share_terms_and_variables_after_producer_destruction,
    rdf_answers_retain_original_literal_blank_and_quoted_identity,
    negative_only_demand_observes_asserted_dependency,
    indirect_negative_demands_reach_facts_and_preserve_cycles,
    fol_construction_checks_resident_handles_and_original_positions,
    fol_projection_validation_preserves_names_and_admits_aliases,
    fol_sort_construction_checks_handles_without_rejecting_unused_declarations,
    reachable_snapshot_preserves_sharing_binders_and_deep_source_death,
    actual_proof_decoder_preserves_rejected_bytes_and_native_lengths,
);

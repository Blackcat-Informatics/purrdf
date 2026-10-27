// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native acceptance of the complete, source-bound demand query entry point.

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::demand::{DemandError, evaluate_demanded};
use purrdf_datalog::dictionary::{DictionaryError, DictionaryTerm, TermDictionary};
use purrdf_datalog::guard::{Guard, NoGuards};
use purrdf_datalog::id::TermId;
use purrdf_datalog::seminaive::{EvalError, EvalOptions, compile, evaluate_guarded};
use purrdf_datalog::store::{RelationStore, TermInterner};
use std::collections::BTreeSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

type Store = RelationStore<TermInterner, 3>;
fn term(value: &str) -> ClauseTerm {
    if value.starts_with('?') {
        ClauseTerm::var(value)
    } else {
        ClauseTerm::iri(format!("urn:{value}"))
    }
}
fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom<3> {
    ClauseAtom::triple(term(subject), term(predicate), term(object))
}
#[path = "../src/test_support/closure.rs"]
mod closure;
fn transitive() -> Vec<DlClause<3>> {
    closure::closure_rules("edge", "reach", atom)
}

fn store(edges: &[(usize, usize)]) -> Store {
    let mut store = Store::with_dictionary(TermInterner::default());
    for (subject, object) in edges {
        store
            .insert_terms(
                [
                    format!("<urn:{subject}>"),
                    "<urn:edge>".into(),
                    format!("<urn:{object}>"),
                ]
                .each_ref()
                .map(|value| DictionaryTerm::Surface(value)),
            )
            .unwrap();
    }
    store
}
fn surfaces<D: TermDictionary>(dictionary: &D, rows: Vec<[D::Id; 3]>) -> BTreeSet<[String; 3]> {
    rows.into_iter()
        .map(|row| row.map(|id| dictionary.resolve_term(id).unwrap().as_ref().to_owned()))
        .collect()
}
fn full(rules: &[DlClause<3>], goal: &ClauseAtom<3>, input: Store) -> BTreeSet<[String; 3]> {
    let evaluated = evaluate_guarded(
        &compile(rules.to_vec()).unwrap(),
        input,
        &NoGuards,
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    evaluated
        .facts()
        .rows_from(0)
        .filter_map(|(_, row)| {
            let surface = row.map(|id| evaluated.facts().interner().resolve(id).to_owned());
            let matches = goal
                .terms()
                .into_iter()
                .zip(&surface)
                .all(|(term, value)| term.surface().is_none_or(|constant| constant == *value));
            let repeated = goal.subject().is_var() && goal.subject() == goal.object();
            (matches && (!repeated || row[0] == row[2])).then_some(surface)
        })
        .collect()
}
fn compare(rules: &[DlClause<3>], goal: &ClauseAtom<3>, edges: &[(usize, usize)]) {
    let expected = full(rules, goal, store(edges));
    let answer =
        evaluate_demanded(rules, goal, store(edges), &EvalOptions::default(), None).unwrap();
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        expected,
        "goal {goal:?}"
    );
}

#[test]
fn recursive_and_mutual_answers_keep_constants_equality_and_empty_results() {
    let mut rules = transitive();
    rules.extend([
        DlClause::datalog(atom("?x", "alias", "?y"), vec![atom("?x", "reach", "?y")]),
        DlClause::datalog(atom("?x", "reach", "?y"), vec![atom("?x", "alias", "?y")]),
        DlClause::datalog(atom("?x", "reach", "2"), vec![atom("?x", "edge", "1")]),
    ]);
    let edges = [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5)];
    for goal in [
        atom("0", "reach", "?y"),
        atom("?x", "reach", "2"),
        atom("0", "reach", "2"),
        atom("0", "reach", "4"),
        atom("?x", "reach", "?x"),
        atom("missing", "reach", "?y"),
        atom("?x", "reach", "?y"),
        atom("3", "edge", "?y"),
    ] {
        compare(&rules, &goal, &edges);
    }
}

#[test]
fn generated_positive_programs_match_full_model_filtering() {
    for seed in 0..16_u64 {
        let mut random = seed;
        let edges: Vec<_> = (0..7)
            .flat_map(|s| (0..7).map(move |o| (s, o)))
            .filter(|_| purrdf_hash::mix::splitmix64_next(&mut random).is_multiple_of(5))
            .collect();
        let mut rules = transitive();
        rules.push(DlClause::datalog(
            atom("?x", "reach", "?y"),
            vec![atom("?x", "reach", "?z"), atom("?z", "reach", "?y")],
        ));
        if seed % 2 == 0 {
            rules.push(DlClause::datalog(
                atom("?x", "reach", "?x"),
                vec![atom("?x", "edge", "?y")],
            ));
        }
        for goal in [
            atom("0", "reach", "?y"),
            atom("?x", "reach", "1"),
            atom("2", "reach", "3"),
            atom("?x", "reach", "?x"),
        ] {
            compare(&rules, &goal, &edges);
        }
    }
}

#[test]
fn disconnected_component_is_not_derived_and_work_is_reduced() {
    let rules = transitive();
    let goal = atom("0", "reach", "?y");
    let small = [(0, 1), (1, 2)];
    let mut large = small.to_vec();
    large.extend((10..35).map(|node| (node, node + 1)));
    let selected =
        evaluate_demanded(&rules, &goal, store(&small), &EvalOptions::default(), None).unwrap();
    let answer =
        evaluate_demanded(&rules, &goal, store(&large), &EvalOptions::default(), None).unwrap();
    assert_eq!(
        answer.work().committed_facts(),
        selected.work().committed_facts(),
        "unrelated component adds no derived facts, including demands"
    );
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        surfaces(selected.dictionary(), selected.answers().unwrap())
    );
    let evaluated = evaluate_guarded(
        &compile(rules).unwrap(),
        store(&large),
        &NoGuards,
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    assert!(answer.work().committed_facts() < evaluated.work().committed_facts());
    assert!(answer.work().join_steps() < evaluated.work().join_steps());
}

fn internal_terms(dictionary: &TermInterner) -> Vec<String> {
    (0..dictionary.len())
        .map(|at| dictionary.resolve(TermId::from_index(at as u64).unwrap()))
        .filter(|surface| surface.starts_with("<urn:purrdf:demand:"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn namespace_is_deterministic_avoids_unused_terms_and_binds_the_program() {
    let rules = transitive();
    let goal = atom("0", "reach", "?y");
    let edges = [(0, 1), (1, 2)];
    let first =
        evaluate_demanded(&rules, &goal, store(&edges), &EvalOptions::default(), None).unwrap();
    let names = internal_terms(first.dictionary());
    assert_ne!(names, Vec::<String>::new());
    let second =
        evaluate_demanded(&rules, &goal, store(&edges), &EvalOptions::default(), None).unwrap();
    assert_eq!(names, internal_terms(second.dictionary()));
    // An unused term still belongs to the caller's dictionary and cannot be minted internally.
    let mut dictionary = TermInterner::default();
    dictionary.intern(&names[0]).unwrap();
    let input = RelationStore::<_, 3>::with_dictionary(dictionary);
    let collision = evaluate_demanded(&rules, &goal, input, &EvalOptions::default(), None).unwrap();
    let revised = internal_terms(collision.dictionary());
    assert!(revised.iter().any(|name| !names.contains(name)));
    assert_eq!(collision.answers().unwrap(), Vec::<[TermId; 3]>::new());
    // An unreachable authored rule changes the program identity and namespace.
    let mut authored = rules;
    let reserved = names[0].trim_start_matches('<').trim_end_matches('>');
    authored.push(DlClause::datalog(
        ClauseAtom::triple(ClauseTerm::iri(reserved), term("unused"), term("0")),
        Vec::new(),
    ));
    let result = evaluate_demanded(
        &authored,
        &goal,
        store(&edges),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    assert!(
        internal_terms(result.dictionary())
            .iter()
            .all(|name| !names.contains(name))
    );
    assert_eq!(
        surfaces(result.dictionary(), result.answers().unwrap()),
        surfaces(first.dictionary(), first.answers().unwrap())
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Lost;
impl std::fmt::Display for Lost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("original dictionary unavailable")
    }
}
impl std::error::Error for Lost {}
impl purrdf_hash::error::StructuredError for Lost {
    fn visit<'a>(&'a self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'a>) {
        visitor.variant("original-dictionary-unavailable");
    }
}
#[derive(Debug, Default)]
struct Control {
    lookup_failure: AtomicBool,
    read_failure: AtomicBool,
    writes: AtomicUsize,
}
#[derive(Debug, Default)]
struct CheckedDictionary {
    terms: TermInterner,
    control: Arc<Control>,
}
impl TermDictionary for CheckedDictionary {
    type Id = TermId;
    type Error = Lost;
    type Lexical<'a> = &'a str;
    fn lookup_term(&self, surface: &str) -> Result<Option<TermId>, Lost> {
        if self.control.lookup_failure.load(Ordering::Relaxed) {
            return Err(Lost);
        }
        Ok(self.terms.lookup(surface))
    }
    fn resolve_term(&self, id: TermId) -> Result<&str, Lost> {
        self.check_read()?;
        Ok(self.terms.resolve(id))
    }
    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, TermId>,
        right: DictionaryTerm<'_, TermId>,
    ) -> Result<std::cmp::Ordering, Lost> {
        purrdf_datalog::dictionary::compare_lexical_terms(self, left, right)
    }
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, TermId>; N],
        validate: impl FnOnce(&Self, [TermId; N]) -> Result<T, DictionaryError<Lost>>,
    ) -> Result<T, DictionaryError<Lost>> {
        self.control.writes.fetch_add(1, Ordering::Relaxed);
        let mut candidate = Self {
            terms: self.terms.clone(),
            control: self.control.clone(),
        };
        let mut ids = Vec::new();
        for term in terms {
            ids.push(match term {
                DictionaryTerm::Id(id) => id,
                DictionaryTerm::Surface(surface) => candidate
                    .terms
                    .intern(surface)
                    .map_err(DictionaryError::Storage)?,
            });
        }
        let ids = ids.try_into().expect("fixed input arity");
        let result = validate(&candidate, ids)?;
        self.terms = candidate.terms;
        Ok(result)
    }
    fn check_read(&self) -> Result<(), Lost> {
        if self.control.read_failure.load(Ordering::Relaxed) {
            Err(Lost)
        } else {
            Ok(())
        }
    }
    fn term_count(&self) -> u64 {
        self.terms.len() as u64
    }
    fn resident_bytes(&self) -> usize {
        self.terms.byte_len()
    }
}

#[test]
fn answer_owns_original_dictionary_and_propagates_late_and_preparation_failures() {
    let mut input =
        RelationStore::<CheckedDictionary, 3>::with_dictionary(CheckedDictionary::default());
    input
        .insert_terms(["<urn:0>", "<urn:edge>", "<urn:1>"].map(DictionaryTerm::Surface))
        .unwrap();
    let control = input.interner().control.clone();
    let original = input.interner().lookup_term("<urn:0>").unwrap().unwrap();
    let rules = transitive();
    let goal = atom("0", "reach", "?y");
    let answer = evaluate_demanded(&rules, &goal, input, &EvalOptions::default(), None).unwrap();
    drop(rules);
    drop(goal);
    assert!(Arc::ptr_eq(&control, &answer.dictionary().control));
    assert_eq!(answer.answers().unwrap()[0][0], original);
    control.lookup_failure.store(true, Ordering::Relaxed);
    assert_eq!(answer.answers(), Err(Lost));
    control.lookup_failure.store(false, Ordering::Relaxed);
    control.read_failure.store(true, Ordering::Relaxed);
    assert_eq!(answer.answers(), Err(Lost));
    let dictionary = CheckedDictionary::default();
    dictionary
        .control
        .lookup_failure
        .store(true, Ordering::Relaxed);
    assert!(matches!(
        evaluate_demanded(
            &transitive(),
            &atom("0", "reach", "?y"),
            RelationStore::with_dictionary(dictionary),
            &EvalOptions::default(),
            None
        ),
        Err(DemandError::Source(Lost))
    ));
}

#[test]
fn unsupported_programs_refuse_before_source_access_or_execution() {
    let head = atom("?x", "reach", "?y");
    let body = atom("?x", "edge", "?y");
    let bad = [
        DlClause::datalog(head.clone(), vec![body.clone().into_negated()]),
        DlClause::datalog(head.clone(), vec![body.clone().from_base()]),
        DlClause::datalog(head.clone(), vec![atom("?x", "?p", "?y")]),
        DlClause::datalog(head.clone(), vec![body.clone()])
            .with_guards(vec![Guard::not_equal("?x", "?y")]),
        DlClause::new(
            vec![HeadDisjunct::new(vec![head.clone(), head.clone()])],
            Vec::new(),
            vec![body.clone()],
        ),
        DlClause::new(
            vec![HeadDisjunct::atom(atom("?x", "reach", "?fresh"))],
            vec!["?fresh".into()],
            vec![body.clone()],
        ),
        DlClause::new(
            vec![
                HeadDisjunct::atom(head.clone()),
                HeadDisjunct::atom(head.clone()),
            ],
            Vec::new(),
            vec![body.clone()],
        ),
        DlClause::inconsistency(vec![body.clone()]),
        DlClause::no_output(vec![body]),
        DlClause::datalog(head, Vec::new()),
    ];
    for rule in bad {
        let dictionary = CheckedDictionary::default();
        let control = dictionary.control.clone();
        control.lookup_failure.store(true, Ordering::Relaxed);
        let failure = evaluate_demanded(
            &[rule],
            &atom("0", "unreachable", "?y"),
            RelationStore::with_dictionary(dictionary),
            &EvalOptions::default(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            failure,
            DemandError::Unsupported { rule: Some(0), .. }
        ));
        let captured = purrdf_testkit::error_capture::Capture::read(&failure);
        assert!(matches!(
            captured.field("rule"),
            purrdf_hash::error::ErrorValue::Unsigned(0)
        ));
        assert!(captured.child("reason").code.starts_with("datalog-demand-"));
        assert_eq!(control.writes.load(Ordering::Relaxed), 0);
    }
    for goal in [
        atom("0", "?p", "?y"),
        atom("0", "reach", "?y").from_base(),
        atom("0", "reach", "?y").into_negated(),
    ] {
        assert!(matches!(
            evaluate_demanded(
                &[],
                &goal,
                Store::with_dictionary(TermInterner::default()),
                &EvalOptions::default(),
                None
            ),
            Err(DemandError::Unsupported { rule: None, .. })
        ));
    }
}

#[test]
fn budgets_and_cancellation_return_no_partial_answer_owner() {
    let rules = transitive();
    let goal = atom("0", "reach", "?y");
    for options in [
        EvalOptions::default().with_max_join_steps(0),
        EvalOptions::default().with_max_stored_facts(1),
        EvalOptions::default().with_max_term_arena_bytes(1),
    ] {
        assert!(matches!(
            evaluate_demanded(&rules, &goal, store(&[(0, 1), (1, 2)]), &options, None),
            Err(DemandError::Evaluation(EvalError::BudgetExhausted { .. }))
        ));
    }
    #[derive(Debug)]
    struct Stopped;
    impl purrdf_datalog::stop::StopSignal for Stopped {
        fn stopped(&self) -> bool {
            true
        }
    }
    let stopped = Stopped;
    assert!(matches!(
        evaluate_demanded(
            &rules,
            &goal,
            store(&[(0, 1)]),
            &EvalOptions::default(),
            Some(&stopped)
        ),
        Err(DemandError::Evaluation(EvalError::Stopped { .. }))
    ));
}

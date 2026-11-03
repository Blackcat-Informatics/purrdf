// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native acceptance of the complete, source-bound demand query entry point.

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::demand::{DemandError, evaluate_demanded, prepare_demanded};
use purrdf_datalog::dictionary::{DictionaryError, DictionaryTerm, TermDictionary};
use purrdf_datalog::guard::{Guard, NoGuards};
use purrdf_datalog::id::TermId;
use purrdf_datalog::seminaive::{EvalError, EvalOptions, compile, evaluate_guarded};
use purrdf_datalog::store::{Arity, RelationStore, StoreShape, TermInterner};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[global_allocator]
static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

type Store<const N: usize = 3> = RelationStore<TermInterner, N>;
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
fn surfaces<D: TermDictionary, const N: usize>(
    dictionary: &D,
    rows: Vec<[D::Id; N]>,
) -> BTreeSet<[String; N]> {
    rows.into_iter()
        .map(|row| row.map(|id| dictionary.resolve_term(id).unwrap().as_ref().to_owned()))
        .collect()
}
fn full<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    input: Store<N>,
) -> BTreeSet<[String; N]>
where
    Arity<N>: StoreShape<N>,
{
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
            let mut variables = BTreeMap::new();
            let matches =
                goal.terms()
                    .into_iter()
                    .zip(&surface)
                    .all(|(term, value)| match term.variable() {
                        Some(name) => variables
                            .insert(name, value)
                            .is_none_or(|previous| previous == value),
                        None => term.surface().as_ref() == Some(value),
                    });
            matches.then_some(surface)
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
fn variable_predicates_preserve_original_answers_repeated_variables_and_bound_residuals() {
    let programs = [
        vec![DlClause::datalog(
            atom("?s", "reach", "?o"),
            vec![atom("?s", "?p", "?o")],
        )],
        vec![DlClause::datalog(
            atom("?p", "?p", "?o"),
            vec![atom("?p", "edge", "?o")],
        )],
        vec![DlClause::datalog(
            atom("?s", "out", "?o"),
            vec![
                atom("?s", "?p", "?o"),
                atom("?s", "blocked", "?o").into_negated(),
            ],
        )],
        vec![
            DlClause::datalog(
                atom("?s", "?p", "?o"),
                vec![atom("?s", "edge", "?p"), atom("?s", "edge", "?o")],
            ),
            DlClause::datalog(
                atom("?s", "out", "?o"),
                vec![
                    atom("?s", "edge", "?o"),
                    atom("?s", "blocked", "?o").into_negated(),
                ],
            ),
        ],
    ];
    for rules in programs {
        for goal in [
            atom("?s", "?p", "?o"),
            atom("0", "?p", "1"),
            atom("0", "0", "1"),
            atom("?same", "?same", "?o"),
            atom("0", "out", "?o"),
        ] {
            compare(&rules, &goal, &[(0, 1), (1, 2), (8, 9)]);
        }
    }
    let rules = vec![DlClause::datalog(
        quad("?s", "reach", "?o", "?g"),
        vec![quad("?s", "?p", "?o", "?g")],
    )];
    for goal in [
        quad("?s", "?p", "?o", "?g"),
        quad("0", "?p", "1", ""),
        quad("0", "edge", "1", ""),
    ] {
        let input = || quad_store(&[("0", "1", ""), ("1", "2", "named")]);
        let expected = full(&rules, &goal, input());
        let answer =
            evaluate_demanded(&rules, &goal, input(), &EvalOptions::default(), None).unwrap();
        assert_eq!(
            surfaces(answer.dictionary(), answer.answers().unwrap()),
            expected
        );
    }
    let source = stratified::input(&[
        quad("0", "0", "1", ""),
        quad("0", "0", "2", ""),
        quad("0", "0", "1", "0"),
        quad("0", "0", "2", "0"),
    ]);
    let rules = vec![DlClause::datalog(
        quad("?s", "out", "?o", "?g"),
        vec![quad("?s", "?s", "?o", "?g")],
    )];
    for goal in [quad("0", "out", "1", ""), quad("0", "out", "1", "0")] {
        let expected = full(&rules, &goal, source.clone());
        assert_eq!(expected.len(), 1);
        let answer =
            evaluate_demanded(&rules, &goal, source.clone(), &EvalOptions::default(), None)
                .unwrap();
        assert_eq!(
            surfaces(answer.dictionary(), answer.answers().unwrap()),
            expected
        );
    }
    let rules = vec![DlClause::datalog(
        quad("?o", "?p", "?s", "?p"),
        vec![quad("?s", "?p", "?o", "?p")],
    )];
    let goal = quad("?s", "?p", "0", "?p");
    let expected = full(&rules, &goal, source.clone());
    assert_eq!(expected.len(), 2);
    let answer = evaluate_demanded(&rules, &goal, source, &EvalOptions::default(), None).unwrap();
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        expected
    );
}

fn quad(subject: &str, predicate: &str, object: &str, graph: &str) -> ClauseAtom<4> {
    ClauseAtom::quad(
        term(subject),
        term(predicate),
        term(object),
        if graph.is_empty() {
            ClauseTerm::DefaultGraph
        } else {
            term(graph)
        },
    )
}

fn quad_store(edges: &[(&str, &str, &str)]) -> Store<4> {
    let mut store = Store::with_dictionary(TermInterner::default());
    for &(subject, object, graph) in edges {
        let row = quad(subject, "edge", object, graph)
            .terms()
            .map(|term| term.surface().unwrap());
        store
            .insert_terms(row.each_ref().map(|value| DictionaryTerm::Surface(value)))
            .unwrap();
    }
    store
}

fn quad_rules() -> Vec<DlClause<4>> {
    closure::closure_rules("edge", "reach", |s, p, o| quad(s, p, o, "?g"))
}

#[test]
fn quad_recursion_and_mutual_recursion_preserve_all_eight_binding_patterns() {
    let mut rules = quad_rules();
    rules.extend([
        DlClause::datalog(
            quad("?x", "alias", "?y", "?g"),
            vec![quad("?x", "reach", "?y", "?g")],
        ),
        DlClause::datalog(
            quad("?x", "reach", "?y", "?g"),
            vec![quad("?x", "alias", "?y", "?g")],
        ),
    ]);
    let edges = [
        ("0", "1", ""),
        ("1", "2", ""),
        ("0", "3", "g"),
        ("3", "2", "g"),
        ("4", "5", "h"),
    ];
    let closure: BTreeSet<_> = [
        ("0", "1", ""),
        ("0", "2", ""),
        ("1", "2", ""),
        ("0", "3", "g"),
        ("0", "2", "g"),
        ("3", "2", "g"),
        ("4", "5", "h"),
    ]
    .map(|(s, o, g)| {
        quad(s, "reach", o, g)
            .terms()
            .map(|term| term.surface().unwrap())
    })
    .into_iter()
    .collect();
    for graph in ["", "g", "h", "missing"] {
        for pattern in 0..8 {
            let goal = quad(
                if pattern & 1 == 0 { "?x" } else { "0" },
                "reach",
                if pattern & 2 == 0 { "?y" } else { "2" },
                if pattern & 4 == 0 { "?g" } else { graph },
            );
            let expected: BTreeSet<_> = closure
                .iter()
                .filter(|row| {
                    goal.terms().iter().zip(*row).all(|(term, value)| {
                        term.surface().is_none_or(|constant| constant == *value)
                    })
                })
                .cloned()
                .collect();
            let answer = evaluate_demanded(
                &rules,
                &goal,
                quad_store(&edges),
                &EvalOptions::default(),
                None,
            )
            .unwrap();
            assert_eq!(
                surfaces(answer.dictionary(), answer.answers().unwrap()),
                expected,
                "{goal:?}"
            );
            assert_eq!(full(&rules, &goal, quad_store(&edges)), expected);
            if pattern == 4 && graph.is_empty() {
                let default = answer.dictionary().lookup("").unwrap();
                assert!(
                    answer
                        .answers()
                        .unwrap()
                        .iter()
                        .all(|row| row[3] == default)
                );
            }
        }
    }
}

#[test]
fn cross_graph_recursion_shares_broader_demands_and_keeps_graph_equalities() {
    let rules = vec![
        DlClause::datalog(
            quad("?x", "reach", "?y", "?g"),
            vec![quad("?x", "edge", "?y", "?g")],
        ),
        DlClause::datalog(
            quad("?x", "reach", "?y", "?g"),
            vec![
                quad("?x", "reach", "?z", "?previous"),
                quad("?z", "edge", "?y", "?g"),
            ],
        ),
    ];
    let edges = [("0", "1", ""), ("1", "2", "left"), ("2", "3", "right")];
    let goal = quad("0", "reach", "?y", "right");
    let expected = BTreeSet::from([quad("0", "reach", "3", "right")
        .terms()
        .map(|term| term.surface().unwrap())]);
    let answer = evaluate_demanded(
        &rules,
        &goal,
        quad_store(&edges),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        expected
    );
    assert_eq!(full(&rules, &goal, quad_store(&edges)), expected);
    // A bound graph in the goal becomes free in the recursive call; the broader
    // descriptor must still preserve the goal's original graph and equalities.
    let edges = [
        ("g", "x", "g"),
        ("x", "g", "g"),
        ("g", "g", "g"),
        ("x", "y", "h"),
    ];
    for (goal, expected) in [
        (
            quad("?g", "reach", "?o", "?g"),
            vec![("g", "x"), ("g", "g")],
        ),
        (
            quad("?s", "reach", "?g", "?g"),
            vec![("x", "g"), ("g", "g")],
        ),
        (quad("?g", "reach", "?g", "?g"), vec![("g", "g")]),
    ] {
        let expected: BTreeSet<_> = expected
            .into_iter()
            .map(|(s, o)| {
                quad(s, "reach", o, "g")
                    .terms()
                    .map(|term| term.surface().unwrap())
            })
            .collect();
        let answer = evaluate_demanded(
            &rules,
            &goal,
            quad_store(&edges),
            &EvalOptions::default(),
            None,
        )
        .unwrap();
        assert_eq!(
            surfaces(answer.dictionary(), answer.answers().unwrap()),
            expected,
            "{goal:?}"
        );
        assert_eq!(full(&rules, &goal, quad_store(&edges)), expected);
    }
}

#[test]
fn authored_default_and_named_graph_constants_keep_their_positions() {
    let rules = vec![
        DlClause::datalog(
            quad("?s", "reach", "?o", ""),
            vec![quad("?s", "edge", "?o", "input")],
        ),
        DlClause::datalog(
            quad("?s", "reach", "?o", "output"),
            vec![quad("?s", "reach", "?o", "")],
        ),
    ];
    let edges = [("0", "1", "input"), ("2", "3", "unrelated")];
    for graph in ["", "output", "?g"] {
        let goal = quad("?s", "reach", "?o", graph);
        let expected: BTreeSet<_> = ["", "output"]
            .into_iter()
            .filter(|name| graph == "?g" || graph == *name)
            .map(|name| {
                quad("0", "reach", "1", name)
                    .terms()
                    .map(|term| term.surface().unwrap())
            })
            .collect();
        let answer = evaluate_demanded(
            &rules,
            &goal,
            quad_store(&edges),
            &EvalOptions::default(),
            None,
        )
        .unwrap();
        assert_eq!(
            surfaces(answer.dictionary(), answer.answers().unwrap()),
            expected
        );
        assert_eq!(full(&rules, &goal, quad_store(&edges)), expected);
    }
}

#[test]
fn graph_demand_reduces_work_and_names_bind_graph_identity_without_collisions() {
    let rules = quad_rules();
    let goal = quad("?s", "reach", "?o", "selected");
    let edges = [("0", "1", "selected"), ("1", "2", "selected")];
    let selected = evaluate_demanded(
        &rules,
        &goal,
        quad_store(&edges),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    let mut input = quad_store(&edges);
    for node in 10..35 {
        let row = [
            format!("<urn:{node}>"),
            "<urn:edge>".into(),
            format!("<urn:{}>", node + 1),
            "<urn:unrelated>".into(),
        ];
        input
            .insert_terms(row.each_ref().map(|value| DictionaryTerm::Surface(value)))
            .unwrap();
    }
    let full = evaluate_guarded(
        &compile(rules.clone()).unwrap(),
        input.clone(),
        &NoGuards,
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    let answer = evaluate_demanded(&rules, &goal, input, &EvalOptions::default(), None).unwrap();
    assert_eq!(
        answer.work().committed_facts(),
        selected.work().committed_facts()
    );
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        surfaces(selected.dictionary(), selected.answers().unwrap())
    );
    assert!(answer.work().join_steps() < full.work().join_steps());
    let names = internal_terms(answer.dictionary());
    assert_eq!(names, internal_terms(selected.dictionary()));
    let other = evaluate_demanded(
        &rules,
        &quad("?s", "reach", "?o", ""),
        quad_store(&edges),
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    assert!(
        internal_terms(other.dictionary())
            .iter()
            .all(|name| !names.contains(name))
    );
    let mut collision = quad_store(&edges);
    // Use the exact internal surface in a source graph, not the fixture's urn prefix.
    let row = [
        "<urn:reserved>".into(),
        "<urn:edge>".into(),
        "<urn:value>".into(),
        names[0].clone(),
    ];
    collision
        .insert_terms(row.each_ref().map(|value| DictionaryTerm::Surface(value)))
        .unwrap();
    let answer =
        evaluate_demanded(&rules, &goal, collision, &EvalOptions::default(), None).unwrap();
    assert!(
        internal_terms(answer.dictionary())
            .iter()
            .any(|name| !names.contains(name))
    );
    assert_eq!(
        surfaces(answer.dictionary(), answer.answers().unwrap()),
        surfaces(selected.dictionary(), selected.answers().unwrap())
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
        rules.push(DlClause::datalog(
            atom("?x", "any", "?y"),
            vec![atom("?x", "?predicate", "?y")],
        ));
        rules.push(DlClause::datalog(
            atom("?predicate", "?predicate", "?y"),
            vec![atom("?predicate", "edge", "?y")],
        ));
        for goal in [
            atom("0", "reach", "?y"),
            atom("?x", "reach", "1"),
            atom("2", "reach", "3"),
            atom("?x", "reach", "?x"),
            atom("0", "?predicate", "?y"),
            atom("0", "?predicate", "1"),
            atom("0", "0", "?y"),
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
    read_failure_on_lookup: AtomicBool,
    read_failure: AtomicBool,
    writes: AtomicUsize,
    fail_write_at: AtomicUsize,
    alias_private: AtomicBool,
}
#[derive(Clone, Debug, Default)]
struct CheckedDictionary {
    terms: TermInterner,
    control: Arc<Control>,
    aliases: BTreeMap<TermId, TermId>,
}
impl TermDictionary for CheckedDictionary {
    type Id = TermId;
    type Error = Lost;
    type Lexical<'a> = &'a str;
    fn lookup_term(&self, surface: &str) -> Result<Option<TermId>, Lost> {
        if self.control.lookup_failure.load(Ordering::Relaxed) {
            return Err(Lost);
        }
        let result = self.terms.lookup(surface);
        if self.control.read_failure_on_lookup.load(Ordering::Relaxed) {
            self.control.read_failure.store(true, Ordering::Relaxed);
        }
        Ok(result)
    }
    fn resolve_term(&self, id: TermId) -> Result<&str, Lost> {
        self.check_read()?;
        Ok(self
            .terms
            .resolve(self.aliases.get(&id).copied().unwrap_or(id)))
    }
    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, TermId>,
        right: DictionaryTerm<'_, TermId>,
    ) -> Result<std::cmp::Ordering, Lost> {
        let left = left.canonicalize(self)?;
        let right = right.canonicalize(self)?;
        let lexical = purrdf_datalog::dictionary::compare_lexical_terms(self, left, right)?;
        Ok(lexical.then_with(|| match (left, right) {
            (DictionaryTerm::Id(left), DictionaryTerm::Id(right)) => left.cmp(&right),
            _ => std::cmp::Ordering::Equal,
        }))
    }
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, TermId>; N],
        validate: impl FnOnce(&Self, [TermId; N]) -> Result<T, DictionaryError<Lost>>,
    ) -> Result<T, DictionaryError<Lost>> {
        let write = self.control.writes.fetch_add(1, Ordering::Relaxed) + 1;
        if write == self.control.fail_write_at.load(Ordering::Relaxed) {
            return Err(DictionaryError::Source(Lost));
        }
        let mut candidate = Self {
            terms: self.terms.clone(),
            control: self.control.clone(),
            aliases: self.aliases.clone(),
        };
        let mut ids = Vec::new();
        for term in terms {
            ids.push(match term {
                DictionaryTerm::Id(id) => id,
                DictionaryTerm::Surface(surface) => {
                    if candidate.control.alias_private.load(Ordering::Relaxed)
                        && surface.starts_with("<urn:purrdf:demand:")
                    {
                        candidate
                            .terms
                            .lookup("<urn:edge>")
                            .expect("fixture original identity")
                    } else {
                        candidate
                            .terms
                            .intern(surface)
                            .map_err(DictionaryError::Storage)?
                    }
                }
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
fn shadow_mapping_preserves_distinct_original_ids_with_identical_spellings() {
    let mut dictionary = CheckedDictionary::default();
    let original = dictionary.terms.intern("<urn:edge>").unwrap();
    let alias = dictionary.terms.intern("<urn:edge-alias>").unwrap();
    dictionary.aliases.insert(alias, original);
    dictionary.terms.intern("<urn:out>").unwrap();
    dictionary.terms.intern("<urn:blocked>").unwrap();
    let mut input = RelationStore::<_, 3>::with_dictionary(dictionary);
    input
        .insert_terms(["<urn:s>", "<urn:edge>", "<urn:o>"].map(DictionaryTerm::Surface))
        .unwrap();
    let first = ["<urn:s>", "<urn:edge>", "<urn:o>"]
        .map(|surface| input.interner().lookup_term(surface).unwrap().unwrap());
    let mut second = first;
    second[1] = alias;
    input.insert_terms(second.map(DictionaryTerm::Id)).unwrap();
    let rules = vec![DlClause::datalog(
        atom("?s", "out", "?o"),
        vec![
            atom("?s", "?p", "?o"),
            atom("?s", "blocked", "?o").into_negated(),
        ],
    )];
    let goal = atom("?s", "?p", "?o");
    let expected = evaluate_guarded(
        &compile(rules.clone()).unwrap(),
        input.clone(),
        &NoGuards,
        &EvalOptions::default(),
        None,
    )
    .unwrap();
    let expected: BTreeSet<_> = expected.facts().rows_from(0).map(|(_, row)| row).collect();
    let answer = evaluate_demanded(&rules, &goal, input, &EvalOptions::default(), None).unwrap();
    assert_eq!(
        answer
            .answers()
            .unwrap()
            .into_iter()
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(answer.answers().unwrap().contains(&first));
    assert!(answer.answers().unwrap().contains(&second));
}

#[test]
fn private_predicate_identity_aliases_fail_at_the_original_dictionary_factory() {
    let mut input = RelationStore::<_, 3>::with_dictionary(CheckedDictionary::default());
    input
        .insert_terms(["<urn:s>", "<urn:edge>", "<urn:o>"].map(DictionaryTerm::Surface))
        .unwrap();
    input
        .interner()
        .control
        .alias_private
        .store(true, Ordering::Relaxed);
    let error = evaluate_demanded(
        &[],
        &atom("?s", "?p", "?o"),
        input,
        &EvalOptions::default(),
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        DemandError::Evaluation(EvalError::DemandInput { .. })
    ));
}

#[test]
fn answer_owns_original_dictionary_and_propagates_late_and_preparation_failures() {
    answer_ownership(
        transitive(),
        atom("0", "reach", "?y"),
        ["<urn:0>", "<urn:edge>", "<urn:1>"],
    );
    answer_ownership(
        quad_rules(),
        quad("0", "reach", "?y", ""),
        ["<urn:0>", "<urn:edge>", "<urn:1>", ""],
    );
}

fn answer_ownership<const N: usize>(rules: Vec<DlClause<N>>, goal: ClauseAtom<N>, row: [&str; N])
where
    Arity<N>: StoreShape<N>,
{
    let dictionary = CheckedDictionary::default();
    dictionary
        .control
        .lookup_failure
        .store(true, Ordering::Relaxed);
    assert!(matches!(
        evaluate_demanded(
            &rules,
            &goal,
            RelationStore::with_dictionary(dictionary),
            &EvalOptions::default(),
            None
        ),
        Err(DemandError::Source(Lost))
    ));
    let dictionary = CheckedDictionary::default();
    let control = dictionary.control.clone();
    let prepared = prepare_demanded(&rules, &goal, &dictionary).unwrap();
    control.lookup_failure.store(true, Ordering::Relaxed);
    assert!(matches!(
        prepared.evaluate(
            RelationStore::with_dictionary(dictionary),
            &EvalOptions::default(),
            None
        ),
        Err(DemandError::Source(Lost))
    ));
    assert_eq!(control.writes.load(Ordering::Relaxed), 0);
    let mut input =
        RelationStore::<CheckedDictionary, N>::with_dictionary(CheckedDictionary::default());
    input
        .insert_terms(row.map(DictionaryTerm::Surface))
        .unwrap();
    let control = input.interner().control.clone();
    let original = row.map(|surface| input.interner().lookup_term(surface).unwrap().unwrap());
    let answer = evaluate_demanded(&rules, &goal, input, &EvalOptions::default(), None).unwrap();
    drop(rules);
    drop(goal);
    assert!(Arc::ptr_eq(&control, &answer.dictionary().control));
    let answers = answer.answers().unwrap();
    assert_eq!(answers.len(), 1);
    for at in (0..N).filter(|at| *at != 1) {
        assert_eq!(answers[0][at], original[at]);
    }
    control.lookup_failure.store(true, Ordering::Relaxed);
    assert_eq!(answer.answers(), Err(Lost));
    control.lookup_failure.store(false, Ordering::Relaxed);
    control.read_failure.store(true, Ordering::Relaxed);
    assert_eq!(answer.answers(), Err(Lost));
}

#[test]
fn unsupported_programs_refuse_before_source_access_or_execution() {
    unsupported_programs(
        atom,
        vec![
            DlClause::datalog(
                atom("?x", "reach", "?y"),
                vec![atom("?x", "edge", "?y").from_base()],
            ),
            DlClause::no_output(vec![atom("?x", "edge", "?y")]),
        ],
        vec![atom("0", "reach", "?y").from_base()],
    );
    unsupported_programs(|s, p, o| quad(s, p, o, "?g"), Vec::new(), Vec::new());
    let dictionary = CheckedDictionary::default();
    let control = dictionary.control.clone();
    control.lookup_failure.store(true, Ordering::Relaxed);
    let error = evaluate_demanded(
        &[DlClause::datalog(
            quad("?s", "reach", "?o", "?unbound"),
            vec![quad("?s", "edge", "?o", "?g")],
        )],
        &quad("?s", "reach", "?o", "?g"),
        RelationStore::with_dictionary(dictionary),
        &EvalOptions::default(),
        None,
    )
    .unwrap_err();
    assert!(
        matches!(error, DemandError::Unsupported { reason: purrdf_datalog::demand::Unsupported::UnboundVariable(variable), .. } if variable == "?unbound")
    );
    assert_eq!(control.writes.load(Ordering::Relaxed), 0);
}

fn unsupported_programs<const N: usize>(
    atom: impl Fn(&str, &str, &str) -> ClauseAtom<N>,
    extra_rules: Vec<DlClause<N>>,
    extra_goals: Vec<ClauseAtom<N>>,
) where
    Arity<N>: StoreShape<N>,
{
    let head = atom("?x", "reach", "?y");
    let body = atom("?x", "edge", "?y");
    let bad = [
        DlClause::datalog(head.clone(), vec![body.clone().into_negated()]),
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
        DlClause::inconsistency(vec![body]),
        DlClause::datalog(head, Vec::new()),
    ];
    for rule in bad.into_iter().chain(extra_rules) {
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
    for goal in [atom("0", "reach", "?y").into_negated()]
        .into_iter()
        .chain(extra_goals)
    {
        assert!(matches!(
            evaluate_demanded(
                &[],
                &goal,
                Store::<N>::with_dictionary(TermInterner::default()),
                &EvalOptions::default(),
                None
            ),
            Err(DemandError::Unsupported { rule: None, .. })
        ));
    }
}

#[test]
fn budgets_and_cancellation_return_no_partial_answer_owner() {
    governed(&transitive(), &atom("0", "reach", "?y"), || {
        store(&[(0, 1), (1, 2)])
    });
    governed(&quad_rules(), &quad("0", "reach", "?y", ""), || {
        quad_store(&[("0", "1", ""), ("1", "2", "")])
    });
}

#[derive(Debug)]
struct Stopped;
impl purrdf_datalog::stop::StopSignal for Stopped {
    fn stopped(&self) -> bool {
        true
    }
}

fn governed<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    input: impl Fn() -> Store<N>,
) where
    Arity<N>: StoreShape<N>,
{
    for options in [
        EvalOptions::default().with_max_join_steps(0),
        EvalOptions::default().with_max_stored_facts(1),
        EvalOptions::default().with_max_term_arena_bytes(1),
    ] {
        assert!(matches!(
            evaluate_demanded(rules, goal, input(), &options, None),
            Err(DemandError::Evaluation(EvalError::BudgetExhausted { .. }))
        ));
    }
    let stopped = Stopped;
    assert!(matches!(
        evaluate_demanded(
            rules,
            goal,
            input(),
            &EvalOptions::default(),
            Some(&stopped)
        ),
        Err(DemandError::Evaluation(EvalError::Stopped { .. }))
    ));
}

#[path = "demand/stratified.rs"]
mod stratified;

#[path = "demand/maintained.rs"]
mod maintained;

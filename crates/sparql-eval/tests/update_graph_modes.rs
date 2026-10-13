// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent slot-and-row expectations for both UPDATE graph lifetime modes.
//! Each registered case drives ordinary and metered public UPDATE, with frozen,
//! query and same-request observations. The transfer inventory is exactly 312.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::process::ExitCode;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetMut, GraphExistenceMode, GraphMatchValue, MutableDataset, RdfDataset,
    RdfDatasetBuilder, RdfDiagnostic, SparqlEngine, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    GovernedUpdateOutcome, GraphResolveRequest, GraphResolver, LoadError, NativeSparqlEngine,
    QueryGovernors, QueryOptions,
};
use purrdf_testkit::harness::{self, Trial};

const EX: &str = "http://example.org/";
const MODES: [GraphExistenceMode; 2] = [
    GraphExistenceMode::Implicit,
    GraphExistenceMode::RememberEmpty,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum State {
    Absent,
    Empty,
    Populated,
    DefaultEmpty,
    DefaultPopulated,
}

impl State {
    const ALL: [Self; 5] = [
        Self::Absent,
        Self::Empty,
        Self::Populated,
        Self::DefaultEmpty,
        Self::DefaultPopulated,
    ];

    const fn is_default(self) -> bool {
        matches!(self, Self::DefaultEmpty | Self::DefaultPopulated)
    }

    fn graph(self, named: &str) -> Option<String> {
        (!self.is_default()).then(|| named.to_owned())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Row {
    graph: Option<String>,
    subject: String,
    predicate: String,
    object: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Image {
    slots: BTreeSet<String>,
    rows: BTreeSet<Row>,
}

impl Image {
    fn insert(&mut self, graph: Option<String>, subject: &str, predicate: &str, object: &str) {
        if let Some(name) = &graph {
            self.slots.insert(name.clone());
        }
        self.rows.insert(Row {
            graph,
            subject: subject.to_owned(),
            predicate: predicate.to_owned(),
            object: object.to_owned(),
        });
    }

    fn seed(&mut self, state: State, name: &str, subject: &str) {
        match state {
            State::Absent | State::DefaultEmpty => {}
            State::Empty => {
                self.slots.insert(name.to_owned());
            }
            State::Populated | State::DefaultPopulated => {
                self.insert(state.graph(name), subject, "p", "value");
            }
        }
    }

    fn clear(&mut self, graph: Option<&str>, withdraw: bool) {
        self.rows.retain(|row| row.graph.as_deref() != graph);
        if withdraw && let Some(name) = graph {
            self.slots.remove(name);
        }
    }

    fn exists(&self, graph: Option<&str>) -> bool {
        graph.is_none_or(|name| self.slots.contains(name))
    }

    fn freeze(&self) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        for name in &self.slots {
            let graph = builder.intern_iri(&format!("{EX}{name}"));
            builder.declare_named_graph(graph);
        }
        for row in &self.rows {
            let subject = builder.intern_iri(&format!("{EX}{}", row.subject));
            let predicate = builder.intern_iri(&format!("{EX}{}", row.predicate));
            let object = builder.intern_iri(&format!("{EX}{}", row.object));
            let graph = row
                .graph
                .as_ref()
                .map(|name| builder.intern_iri(&format!("{EX}{name}")));
            builder.push_quad(subject, predicate, object, graph);
        }
        builder.freeze().expect("fixture freezes")
    }
}

fn fixture(source: State, destination: Option<State>) -> Image {
    let mut image = Image::default();
    image.slots.insert("untouched-empty".to_owned());
    image.insert(Some("untouched-data".to_owned()), "unrelated", "p", "value");
    image.seed(source, "source", "source-row");
    if let Some(state) = destination {
        image.seed(state, "destination", "destination-row");
    }
    if !source.is_default() && destination.is_none_or(|state| !state.is_default()) {
        image.insert(None, "default-row", "p", "value");
    }
    image
}

fn local(value: &TermValue) -> String {
    let TermValue::Iri(iri) = value else {
        panic!("this independent fixture contains only IRIs: {value:?}");
    };
    iri.strip_prefix(EX).expect("fixture namespace").to_owned()
}

fn image(dataset: &Arc<RdfDataset>) -> Image {
    let view = MutableDataset::new(Arc::clone(dataset));
    Image {
        slots: dataset
            .named_graphs()
            .map(|id| local(&RdfDataset::term_value(dataset, id)))
            .collect(),
        rows: view
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .into_iter()
            .map(|quad| Row {
                graph: quad.g.as_ref().map(local),
                subject: local(&quad.s),
                predicate: local(&quad.p),
                object: local(&quad.o),
            })
            .collect(),
    }
}

fn request(text: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query: text,
        base_iri: None,
        substitutions: &[],
    }
}

fn execute(
    engine: &NativeSparqlEngine,
    dataset: &mut Arc<RdfDataset>,
    text: &str,
    options: QueryOptions<'_>,
    governed: bool,
) -> Result<(), RdfDiagnostic> {
    if governed {
        let outcome =
            engine.update_governed(dataset, request(text), options, &QueryGovernors::METERED)?;
        assert!(
            matches!(outcome, GovernedUpdateOutcome::Applied { .. }),
            "metering has no ceiling: {outcome:?}"
        );
        Ok(())
    } else {
        engine.update_with_options(dataset, request(text), options)
    }
}

fn assert_queries(engine: &NativeSparqlEngine, dataset: &Arc<RdfDataset>, expected: &Image) {
    let SparqlResult::Solutions { rows, .. } = engine
        .query(dataset, request("SELECT ?g WHERE { GRAPH ?g {} }"))
        .expect("variable GRAPH query")
    else {
        panic!("SELECT solutions")
    };
    let names: BTreeSet<_> = rows
        .iter()
        .map(|row| local(row[0].as_ref().expect("bound graph")))
        .collect();
    assert_eq!(names.len(), rows.len(), "one solution per graph slot");
    assert_eq!(names, expected.slots);
    for candidate in [
        "source",
        "destination",
        "untouched-empty",
        "untouched-data",
        "missing",
    ] {
        let text = format!("PREFIX ex: <{EX}> ASK {{ GRAPH ex:{candidate} {{ BIND(1 AS ?x) }} }}");
        let SparqlResult::Boolean(present) = engine
            .query(dataset, request(&text))
            .expect("constant GRAPH query")
        else {
            panic!("ASK boolean")
        };
        assert_eq!(
            present,
            expected.slots.contains(candidate),
            "constant GRAPH {candidate}"
        );
    }
    assert!(matches!(
        engine
            .query(dataset, request("ASK { BIND(1 AS ?x) }"))
            .expect("mandatory default graph"),
        SparqlResult::Boolean(true)
    ));
}

/// Compare a whole-state independent expectation at both public doors, then use
/// a fresh copy to observe the same slots at a WHERE snapshot inside the request.
fn check(
    before: &Image,
    expected: &Image,
    operation: &str,
    options: QueryOptions<'_>,
    failure: Option<&str>,
) {
    let engine = NativeSparqlEngine::new();
    let text = format!("PREFIX ex: <{EX}> {operation}");
    let base = before.freeze();
    assert_eq!(image(&base), *before);
    for governed in [false, true] {
        let mut dataset = Arc::clone(&base);
        let result = execute(&engine, &mut dataset, &text, options, governed);
        if let Some(code) = failure {
            assert_eq!(result.expect_err("semantic refusal").code, code);
            assert!(Arc::ptr_eq(&dataset, &base), "refusal does not publish");
            assert_eq!(image(&dataset), *before);
            // Refusal must also roll back an earlier successful operation in the
            // same request, rather than merely protect this operation's target.
            let text = format!(
                "PREFIX ex: <{EX}> INSERT DATA {{ ex:prelude ex:p ex:value }}; {operation}"
            );
            let mut partial = Arc::clone(&base);
            assert_eq!(
                execute(&engine, &mut partial, &text, options, governed)
                    .expect_err("later semantic refusal")
                    .code,
                code
            );
            assert!(Arc::ptr_eq(&partial, &base));
            assert_eq!(image(&partial), *before);
            continue;
        }
        result.expect("successful update");
        assert_eq!(
            image(&dataset),
            *expected,
            "complete frozen state, governed={governed}"
        );
        assert_queries(&engine, &dataset, expected);

        let mut observed = expected.clone();
        let mut text =
            format!("{text}; INSERT {{ ex:observer ex:saw ?g }} WHERE {{ GRAPH ?g {{}} }}");
        for name in &expected.slots {
            observed.insert(None, "observer", "saw", name);
        }
        for candidate in [
            "source",
            "destination",
            "untouched-empty",
            "untouched-data",
            "missing",
        ] {
            write!(text, "; INSERT {{ ex:observer ex:hit ex:{candidate} }} WHERE {{ GRAPH ex:{candidate} {{ BIND(1 AS ?x) }} }}").expect("String write");
            if expected.slots.contains(candidate) {
                observed.insert(None, "observer", "hit", candidate);
            }
        }
        let mut within = Arc::clone(&base);
        execute(&engine, &mut within, &text, options, governed).expect("same-request observation");
        assert_eq!(
            image(&within),
            observed,
            "same-request variable and constant GRAPH snapshots"
        );
    }
}

fn target(graph: Option<&str>) -> String {
    graph.map_or_else(|| "DEFAULT".to_owned(), |name| format!("GRAPH ex:{name}"))
}

fn silent_word(silent: bool) -> &'static str {
    if silent { "SILENT " } else { "" }
}

fn transfer(
    operation: &str,
    source: State,
    destination: Option<State>,
    mode: GraphExistenceMode,
    silent: bool,
) {
    let before = fixture(source, destination);
    let mut expected = before.clone();
    let src = source.graph("source");
    let dst = destination.map_or_else(|| src.clone(), |state| state.graph("destination"));
    let remember = mode == GraphExistenceMode::RememberEmpty;
    let self_noop = src == dst && (operation != "ADD" || !remember);
    let missing = remember && !self_noop && !before.exists(src.as_deref());
    if !self_noop && !missing && src != dst {
        let rows: Vec<_> = before
            .rows
            .iter()
            .filter(|row| row.graph == src)
            .cloned()
            .collect();
        if operation != "ADD" {
            expected.clear(dst.as_deref(), !remember);
        }
        for row in rows {
            expected.insert(dst.clone(), &row.subject, &row.predicate, &row.object);
        }
        if remember && let Some(name) = &dst {
            expected.slots.insert(name.clone());
        }
        if operation == "MOVE" {
            expected.clear(src.as_deref(), true);
        }
    }
    let operation = format!(
        "{operation} {}{} TO {}",
        silent_word(silent),
        target(src.as_deref()),
        target(dst.as_deref())
    );
    check(
        &before,
        &expected,
        &operation,
        QueryOptions::EMPTY.with_graph_existence(mode),
        (missing && !silent).then_some("native-sparql-update-graph-missing"),
    );
}

fn create(state: State, mode: GraphExistenceMode, silent: bool) {
    let before = fixture(state, None);
    let mut expected = before.clone();
    let remember = mode == GraphExistenceMode::RememberEmpty;
    let duplicate = remember && before.slots.contains("source");
    if remember && !duplicate {
        expected.slots.insert("source".to_owned());
    }
    check(
        &before,
        &expected,
        &format!("CREATE {}GRAPH ex:source", silent_word(silent)),
        QueryOptions::EMPTY.with_graph_existence(mode),
        (duplicate && !silent).then_some("rdf-ir-graph-already-exists"),
    );
}

fn clear_one(operation: &str, state: State, mode: GraphExistenceMode, silent: bool) {
    let before = fixture(state, None);
    let mut expected = before.clone();
    let graph = state.graph("source");
    let missing = mode == GraphExistenceMode::RememberEmpty && !before.exists(graph.as_deref());
    if !missing {
        expected.clear(
            graph.as_deref(),
            operation == "DROP" || mode == GraphExistenceMode::Implicit,
        );
    }
    check(
        &before,
        &expected,
        &format!(
            "{operation} {}{}",
            silent_word(silent),
            target(graph.as_deref())
        ),
        QueryOptions::EMPTY.with_graph_existence(mode),
        (missing && !silent).then_some("native-sparql-update-graph-missing"),
    );
}

fn clear_bulk(
    operation: &str,
    population: &str,
    all: bool,
    mode: GraphExistenceMode,
    silent: bool,
) {
    let mut before = Image::default();
    before.insert(None, "default-row", "p", "value");
    if population != "none" {
        before.slots.insert("source".to_owned());
    }
    if population == "mixed" {
        before.insert(Some("destination".to_owned()), "named-row", "p", "value");
    }
    let mut expected = before.clone();
    expected.rows.retain(|row| !all && row.graph.is_none());
    if operation == "DROP" || mode == GraphExistenceMode::Implicit {
        expected.slots.clear();
    }
    check(
        &before,
        &expected,
        &format!(
            "{operation} {}{}",
            silent_word(silent),
            if all { "ALL" } else { "NAMED" }
        ),
        QueryOptions::EMPTY.with_graph_existence(mode),
        None,
    );
}

fn row_update(operation: usize, mode: GraphExistenceMode) {
    let before = fixture(State::Populated, None);
    let mut expected = before.clone();
    expected.clear(Some("source"), mode == GraphExistenceMode::Implicit);
    let text = match operation {
        0 => "DELETE DATA { GRAPH ex:source { ex:source-row ex:p ex:value } }",
        1 => "DELETE WHERE { GRAPH ex:source { ?s ?p ?o } }",
        2 => {
            expected.insert(Some("destination".to_owned()), "source-row", "p", "value");
            "DELETE { GRAPH ex:source { ?s ?p ?o } } INSERT { GRAPH ex:destination { ?s ?p ?o } } WHERE { GRAPH ex:source { ?s ?p ?o } }"
        }
        3 => {
            expected = before.clone();
            if mode == GraphExistenceMode::RememberEmpty {
                expected.slots.insert("destination".to_owned());
            }
            "INSERT DATA { GRAPH ex:destination { ex:x ex:p ex:value } }; DELETE DATA { GRAPH ex:destination { ex:x ex:p ex:value } }"
        }
        4 => {
            expected.insert(Some("source".to_owned()), "replacement", "p", "value");
            "DELETE WHERE { GRAPH ex:source { ?s ?p ?o } }; INSERT DATA { GRAPH ex:source { ex:replacement ex:p ex:value } }"
        }
        _ => panic!("authored row operation"),
    };
    check(
        &before,
        &expected,
        text,
        QueryOptions::EMPTY.with_graph_existence(mode),
        None,
    );
}

#[derive(Clone, Copy)]
enum Document {
    Empty,
    Populated,
    Failed,
}

impl GraphResolver for Document {
    fn resolve(&self, request: GraphResolveRequest<'_>) -> Result<Arc<RdfDataset>, LoadError> {
        assert_eq!(request.iri, format!("{EX}document"));
        if matches!(self, Self::Failed) {
            return Err(LoadError::Transport(
                "authored unreachable source".to_owned(),
            ));
        }
        let mut document = Image::default();
        if matches!(self, Self::Populated) {
            document.insert(None, "loaded", "p", "value");
        }
        Ok(document.freeze())
    }
}

fn load(state: State, document: Document, mode: GraphExistenceMode, silent: bool) {
    let before = fixture(state, None);
    let mut expected = before.clone();
    let graph = state.graph("source");
    if !matches!(document, Document::Failed) {
        if matches!(document, Document::Populated) {
            expected.insert(graph.clone(), "loaded", "p", "value");
        }
        if mode == GraphExistenceMode::RememberEmpty
            && let Some(name) = &graph
        {
            expected.slots.insert(name.clone());
        }
    }
    let destination = graph.map_or_else(String::new, |name| format!(" INTO GRAPH ex:{name}"));
    check(
        &before,
        &expected,
        &format!("LOAD {}ex:document{destination}", silent_word(silent)),
        QueryOptions::EMPTY
            .with_graph_existence(mode)
            .with_load(Some(&document)),
        (matches!(document, Document::Failed) && !silent).then_some("native-sparql-load-failed"),
    );
}

/// Build side-table graph membership directly, without using UPDATE or its row
/// projection. Annotation-only membership has an unrelated default reifier.
fn role_fixture(role: usize, state: State, default_row: bool, blank: bool) -> Arc<RdfDataset> {
    assert!(!state.is_default(), "named role state");
    let named_rows = state == State::Populated;
    let mut builder = RdfDatasetBuilder::new();
    let graph = builder.intern_iri(&format!("{EX}source"));
    let subject = builder.intern_iri(&format!("{EX}subject"));
    let predicate = builder.intern_iri(&format!("{EX}p"));
    let object = builder.intern_iri(&format!("{EX}value"));
    let reifier = builder.intern_iri(&format!("{EX}reifier"));
    let triple = builder.intern_triple(subject, predicate, object);
    if state == State::Empty {
        builder.declare_named_graph(graph);
    }
    if blank {
        let graph = builder.intern_blank("empty", BlankScope::DEFAULT);
        builder.declare_named_graph(graph);
    }
    match role {
        0 if named_rows => {
            builder.push_quad(subject, predicate, object, Some(graph));
        }
        1 if named_rows => {
            builder.push_reifier_in_graph(reifier, triple, Some(graph));
        }
        2 => {
            if default_row {
                builder.push_reifier(reifier, triple);
            }
            if named_rows {
                builder.push_annotation_in_graph(reifier, predicate, object, Some(graph));
            }
        }
        0 | 1 => {}
        _ => panic!("authored role"),
    }
    builder.freeze().expect("role fixture freezes")
}

fn role_image(dataset: &RdfDataset) -> [BTreeSet<String>; 4] {
    [
        dataset
            .owned_quads()
            .map(|value| format!("{value:?}"))
            .collect(),
        dataset
            .owned_reifiers()
            .map(|value| format!("{value:?}"))
            .collect(),
        dataset
            .owned_annotations()
            .map(|value| format!("{value:?}"))
            .collect(),
        dataset
            .owned_named_graphs()
            .map(|value| format!("{value:?}"))
            .collect(),
    ]
}

fn role_membership(role: usize, mode: GraphExistenceMode) {
    let base = role_fixture(role, State::Populated, true, true);
    let counts = role_image(&base).map(|values| values.len());
    assert_eq!(
        counts,
        match role {
            0 => [1, 0, 0, 2],
            1 => [0, 1, 0, 2],
            2 => [0, 1, 1, 2],
            _ => unreachable!(),
        }
    );
    let engine = NativeSparqlEngine::new();
    let options = QueryOptions::EMPTY.with_graph_existence(mode);
    for governed in [false, true] {
        for operation in [
            "CREATE GRAPH ex:source",
            "CLEAR GRAPH ex:source",
            "DROP GRAPH ex:source",
            "CLEAR NAMED",
            "DROP NAMED",
            "CLEAR ALL",
            "DROP ALL",
        ] {
            let mut dataset = Arc::clone(&base);
            let text = format!("PREFIX ex: <{EX}> {operation}");
            let result = execute(&engine, &mut dataset, &text, options, governed);
            if operation.starts_with("CREATE") && mode == GraphExistenceMode::RememberEmpty {
                assert_eq!(
                    result
                        .expect_err("role membership refuses duplicate CREATE")
                        .code,
                    "rdf-ir-graph-already-exists"
                );
                assert!(Arc::ptr_eq(&dataset, &base));
                assert_eq!(role_image(&dataset), role_image(&base));
                continue;
            }
            result.expect("role update");
            let create = operation.starts_with("CREATE");
            let keep = create
                || (operation.starts_with("CLEAR") && mode == GraphExistenceMode::RememberEmpty);
            let single = operation.ends_with("ex:source");
            let state = if create {
                State::Populated
            } else if keep {
                State::Empty
            } else {
                State::Absent
            };
            let expected = role_fixture(
                role,
                state,
                create || !operation.ends_with("ALL"),
                create || single || keep,
            );
            assert_eq!(
                role_image(&dataset),
                role_image(&expected),
                "exact roles and blank slot: {operation}"
            );
            let expected_names: BTreeSet<_> = expected
                .named_graphs()
                .map(|id| format!("{:?}", RdfDataset::term_value(&expected, id)))
                .collect();
            let query_names = |data: &Arc<RdfDataset>, query: &str| {
                let SparqlResult::Solutions { rows, .. } = engine
                    .query(data, request(query))
                    .expect("role graph query")
                else {
                    panic!("SELECT solutions")
                };
                rows.into_iter()
                    .map(|row| format!("{:?}", row[0].as_ref().expect("bound graph")))
                    .collect::<BTreeSet<_>>()
            };
            assert_eq!(
                query_names(&dataset, "SELECT ?g WHERE { GRAPH ?g {} }"),
                expected_names
            );
            let SparqlResult::Boolean(present) = engine
                .query(
                    &dataset,
                    request(&format!(
                        "PREFIX ex: <{EX}> ASK {{ GRAPH ex:source {{ BIND(1 AS ?x) }} }}"
                    )),
                )
                .expect("constant role membership")
            else {
                panic!("ASK boolean")
            };
            assert_eq!(present, keep);
            let mut within = Arc::clone(&base);
            let text =
                format!("{text}; INSERT {{ ex:observer ex:saw ?g }} WHERE {{ GRAPH ?g {{}} }}");
            execute(&engine, &mut within, &text, options, governed)
                .expect("role snapshot observation");
            assert_eq!(
                query_names(
                    &within,
                    &format!("PREFIX ex: <{EX}> SELECT ?g WHERE {{ ex:observer ex:saw ?g }}")
                ),
                expected_names
            );
        }
    }
}

fn trials() -> Vec<Trial> {
    let mut trials = Vec::new();
    let mut pair_counts = [0; 3];
    for source in State::ALL {
        for destination in State::ALL {
            if source.is_default() && destination.is_default() {
                continue;
            }
            let family = if source.is_default() {
                2
            } else {
                usize::from(destination.is_default())
            };
            pair_counts[family] += 1;
            for operation in ["ADD", "COPY", "MOVE"] {
                for mode in MODES {
                    for silent in [false, true] {
                        trials.push(Trial::test(format!("transfer/distinct/{operation}/{source:?}/{destination:?}/{mode:?}/silent={silent}"), move || { transfer(operation, source, Some(destination), mode, silent); Ok(()) }));
                    }
                }
            }
        }
    }
    assert_eq!(pair_counts, [9, 6, 6]);
    assert_eq!(trials.len(), 252);
    for source in State::ALL {
        for operation in ["ADD", "COPY", "MOVE"] {
            for mode in MODES {
                for silent in [false, true] {
                    trials.push(Trial::test(
                        format!("transfer/self/{operation}/{source:?}/{mode:?}/silent={silent}"),
                        move || {
                            transfer(operation, source, None, mode, silent);
                            Ok(())
                        },
                    ));
                }
            }
        }
    }
    assert_eq!(trials.len(), 312);
    for mode in MODES {
        for silent in [false, true] {
            for state in State::ALL {
                if !state.is_default() {
                    trials.push(Trial::test(
                        format!("create/{state:?}/{mode:?}/silent={silent}"),
                        move || {
                            create(state, mode, silent);
                            Ok(())
                        },
                    ));
                }
                for operation in ["CLEAR", "DROP"] {
                    trials.push(Trial::test(
                        format!("clear/one/{operation}/{state:?}/{mode:?}/silent={silent}"),
                        move || {
                            clear_one(operation, state, mode, silent);
                            Ok(())
                        },
                    ));
                }
                for (name, document) in [
                    ("empty", Document::Empty),
                    ("populated", Document::Populated),
                    ("failed", Document::Failed),
                ] {
                    trials.push(Trial::test(
                        format!("load/{state:?}/{name}/{mode:?}/silent={silent}"),
                        move || {
                            load(state, document, mode, silent);
                            Ok(())
                        },
                    ));
                }
            }
            for operation in ["CLEAR", "DROP"] {
                for population in ["none", "empty", "mixed"] {
                    for all in [false, true] {
                        trials.push(Trial::test(format!("clear/bulk/{operation}/{population}/all={all}/{mode:?}/silent={silent}"), move || { clear_bulk(operation, population, all, mode, silent); Ok(()) }));
                    }
                }
            }
        }
        for operation in 0..5 {
            trials.push(Trial::test(
                format!("row/{operation}/{mode:?}"),
                move || {
                    row_update(operation, mode);
                    Ok(())
                },
            ));
        }
        for role in 0..3 {
            trials.push(Trial::test(format!("roles/{role}/{mode:?}"), move || {
                role_membership(role, mode);
                Ok(())
            }));
        }
    }
    assert_eq!(trials.len(), 488);
    let names: BTreeSet<_> = trials.iter().map(Trial::name).collect();
    assert_eq!(
        names.len(),
        trials.len(),
        "every generated registration is unique"
    );
    trials
}

fn main() -> ExitCode {
    harness::main(trials())
}

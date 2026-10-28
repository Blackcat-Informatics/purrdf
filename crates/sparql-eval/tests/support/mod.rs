// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixtures and result readers shared by the `purrdf-sparql-eval` integration tests and
//! benches: each is written once here rather than copied into every file that needs it.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

pub mod boundary_joins;
pub mod fallible;
#[cfg(target_os = "linux")]
pub mod governor_counts;
pub mod governor_workloads;
pub mod segmented;
pub mod statement_claims;

/// Scan-default adapters preserve resident receipts and statistical inputs.
/// Physical-access descriptors and metadata provenance identify the actual
/// provider; neither difference may hide a changed count or execution receipt.
pub fn assert_scan_explanation_matches_resident(
    actual: &purrdf_sparql_eval::QueryExplanation,
    resident: &str,
) {
    use purrdf_core::StatementIndexSupport;
    let scanning = StatementIndexSupport::default();
    assert_eq!(actual.statement_indexes(), scanning);
    let indexed_line = format!("statement-indexes {}\n", StatementIndexSupport::INDEXED);
    let scanning_line = format!("statement-indexes {scanning}\n");
    assert_eq!(resident.matches(&indexed_line).count(), 1);
    let rendered = actual.render();
    let expected = resident.replacen(&indexed_line, &scanning_line, 1);
    assert_eq!(rendered.lines().count(), expected.lines().count());
    for (observed, expected) in rendered.lines().zip(expected.lines()) {
        if let Some((expected_prefix, expected_statistics)) = expected.split_once(" source=") {
            let (observed_prefix, observed_statistics) = observed
                .split_once(" source=")
                .expect("the adapter retains statistical evidence");
            assert_eq!(observed_prefix, expected_prefix);
            let (provenance, observed_counts) = observed_statistics
                .rsplit_once(" row-input=")
                .expect("statistical input counts");
            let (_, expected_counts) = expected_statistics
                .rsplit_once(" row-input=")
                .expect("resident statistical input counts");
            let (source, revision) = provenance.rsplit_once(" revision=").unwrap();
            assert!(source.starts_with('"') && source.ends_with('"') && source.len() > 2);
            revision.parse::<u64>().expect("metadata revision");
            assert_eq!(observed_counts, expected_counts);
        } else {
            assert_eq!(observed, expected);
        }
    }
}

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, StopCause, TermValue,
};
use purrdf_sparql_algebra::{GraphPattern, Query};
use purrdf_sparql_eval::{ExtensionEnv, NativeSparqlEngine, QueryOptions, StopSignal};

/// A frozen dataset holding nothing.
pub fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new().freeze().expect("an empty dataset")
}

/// A `SELECT` of the length of `?x{steps}`, where `?x0` is a `digits`-digit
/// integer of sevens and each `?x{n}` squares `?x{n-1}`: a product chain whose
/// operands double in length at every step.
pub fn squaring_chain(digits: usize, steps: usize) -> String {
    squaring_chain_from(&"7".repeat(digits), steps)
}

/// [`squaring_chain`] from the numeric literal `base`: `0.1` keeps a one-digit
/// coefficient while its scale doubles at every step, `1.0` stays a machine word.
pub fn squaring_chain_from(base: &str, steps: usize) -> String {
    let mut binds = format!("BIND({base} AS ?x0)");
    for step in 1..=steps {
        let previous = step - 1;
        let _ = write!(binds, " BIND(?x{previous} * ?x{previous} AS ?x{step})");
    }
    format!("SELECT (STRLEN(STR(?x{steps})) AS ?len) WHERE {{ {binds} }}")
}

/// One result cell as `lexical^^local-name` with the XSD namespace dropped from the
/// datatype (`5^^integer`), `-` when unbound, and any other term in its `Debug` form:
/// the spelling the numeric suites compare cells in.
pub fn numeric_cell(value: Option<&TermValue>) -> String {
    match value {
        None => "-".to_owned(),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => format!(
            "{lexical_form}^^{}",
            datatype
                .strip_prefix(purrdf_xsd::XSD_NS)
                .unwrap_or(datatype)
        ),
        Some(other) => format!("{other:?}"),
    }
}

/// The namespace the local-name fixtures below mint their IRIs under.
pub const EX: &str = "http://example.org/";

/// The fixture IRI `local` names under [`EX`].
pub fn iri(local: &str) -> TermValue {
    TermValue::iri(format!("{EX}{local}"))
}

/// Query options carrying `env`, the extension environment a call is admitted against.
pub fn with_env<'options, 'providers>(
    env: &'options ExtensionEnv<'providers>,
) -> QueryOptions<'options, 'providers> {
    QueryOptions::new().with_env(env)
}

/// An `ASK` over `pattern`, with no dataset clause, base or version.
pub fn ask(pattern: GraphPattern) -> Query {
    Query::ask(pattern)
}

// ── datasets ────────────────────────────────────────────────────────────────

/// A dataset of `(subject, predicate, object)` local names under [`EX`].
pub fn local_dataset<S, P, O>(triples: impl IntoIterator<Item = (S, P, O)>) -> Arc<RdfDataset>
where
    S: AsRef<str>,
    P: AsRef<str>,
    O: AsRef<str>,
{
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = builder.intern_iri(&format!("{EX}{}", s.as_ref())).unwrap();
        let p = builder.intern_iri(&format!("{EX}{}", p.as_ref())).unwrap();
        let o = builder.intern_iri(&format!("{EX}{}", o.as_ref())).unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().expect("the fixture dataset")
}

/// `subject(i) predicate object(i)` for `i` in `0..count`: one edge per index on a single
/// predicate.
pub fn fan_out(
    count: usize,
    predicate: &str,
    subject: impl Fn(usize) -> String,
    object: impl Fn(usize) -> String,
) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri(predicate).unwrap();
    for index in 0..count {
        let s = builder.intern_iri(&subject(index)).unwrap();
        let o = builder.intern_iri(&object(index)).unwrap();
        builder.push_quad(s, predicate, o, None).unwrap();
    }
    builder.freeze().expect("the fan-out fixture")
}

/// `<s1> <p> 1` and `<s2> <p> 2`, the objects `xsd:integer` literals.
pub fn two_integer_objects() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p")).unwrap();
    for n in 1..=2 {
        let s = builder.intern_iri(&format!("{EX}s{n}")).unwrap();
        let o = builder
            .intern_literal(RdfLiteral::typed(
                n.to_string(),
                "http://www.w3.org/2001/XMLSchema#integer",
            ))
            .unwrap();
        builder.push_quad(s, p, o, None).unwrap();
    }
    builder.freeze().expect("the fixture dataset")
}

/// `ex:a`, `ex:b` and `ex:c`, each `ex:p ex:value`.
pub fn three_subjects_one_value() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let predicate = builder.intern_iri("http://example.org/p").unwrap();
    let value = builder.intern_iri("http://example.org/value").unwrap();
    for name in ["a", "b", "c"] {
        let subject = builder
            .intern_iri(&format!("http://example.org/{name}"))
            .unwrap();
        builder.push_quad(subject, predicate, value, None).unwrap();
    }
    builder.freeze().unwrap()
}

/// A skewed star: one hub linked to `N` leaves per predicate, for the `(name, N)` pairs
/// given. The per-predicate cardinalities are deliberately uneven so the join order
/// materially changes the intermediate-result sizes.
pub fn skewed_star(spec: &[(&str, usize)]) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let hub = b.intern_iri("http://ex/hub").unwrap();
    for &(name, count) in spec {
        let pred = b.intern_iri(&format!("http://ex/{name}")).unwrap();
        for i in 0..count {
            let leaf = b.intern_iri(&format!("http://ex/{name}{i}")).unwrap();
            b.push_quad(hub, pred, leaf, None).unwrap();
        }
    }
    b.freeze().expect("freeze")
}

// ── running and reading results ─────────────────────────────────────────────

/// Evaluate `prefix` followed by `query_body` against `ds`, panicking with the query text
/// on an error.
pub fn run_prefixed(ds: &RdfDataset, prefix: &str, query_body: &str) -> SparqlResult {
    let text = format!("{prefix}{query_body}");
    NativeSparqlEngine::new()
        .query_with_options_view(
            ds,
            SparqlRequest {
                query: &text,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|e| panic!("query failed: {e:?}\nquery: {text}"))
}

/// `<https://example.org/d/unrelated> <https://example.org/d/p> <https://example.org/d/o>`:
/// one row that no relation under test mentions, so a query's answers come from the
/// relation and a solution exists to extend.
pub fn unrelated_quad() -> Arc<RdfDataset> {
    purrdf_core::term_fixture::one_quad(
        "https://example.org/d/unrelated",
        "https://example.org/d/p",
        "https://example.org/d/o",
    )
    .expect("authored fixture")
}

/// Borrowed variables and rows of a SELECT result.
pub type SolutionsView<'a> = (&'a [String], &'a [Vec<Option<TermValue>>]);

/// The shape name of a result, for a failure message that names the wrong shape
/// without formatting the result's contents.
pub trait ResultView: std::fmt::Debug {
    fn solutions_view(&self) -> Option<SolutionsView<'_>>;
    fn graph_view(&self) -> Option<&RdfDataset>;
    fn boolean_view(&self) -> Option<bool>;
    fn output_size(&self) -> usize;
}

impl ResultView for SparqlResult {
    fn solutions_view(&self) -> Option<SolutionsView<'_>> {
        self.solutions()
    }
    fn graph_view(&self) -> Option<&RdfDataset> {
        if let Self::Graph(graph) = self {
            Some(graph)
        } else {
            None
        }
    }
    fn boolean_view(&self) -> Option<bool> {
        if let Self::Boolean(value) = self {
            Some(*value)
        } else {
            None
        }
    }
    fn output_size(&self) -> usize {
        match self {
            Self::Solutions { rows, .. } => rows.len(),
            Self::Graph(graph) => graph.quad_count(),
            Self::Boolean(value) => usize::from(*value),
        }
    }
}

impl ResultView for purrdf_sparql_eval::RetainedSparqlResult {
    fn solutions_view(&self) -> Option<SolutionsView<'_>> {
        self.solutions()
    }
    fn graph_view(&self) -> Option<&RdfDataset> {
        self.graph()
    }
    fn boolean_view(&self) -> Option<bool> {
        self.boolean()
    }
    fn output_size(&self) -> usize {
        self.solutions().map_or_else(
            || {
                self.graph().map_or_else(
                    || usize::from(self.boolean().expect("ASK shape")),
                    RdfDataset::quad_count,
                )
            },
            |(_, rows)| rows.len(),
        )
    }
}

/// The variables and rows of a SELECT result, panicking on any other shape.
pub fn solutions(result: impl ResultView) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let (variables, rows) = result.solutions_view().expect("expected SELECT solutions");
    let copied = (variables.to_vec(), rows.to_vec());
    drop(result);
    copied
}

/// The row count of a `SELECT` result.
pub fn row_count(result: &impl ResultView) -> usize {
    result
        .solutions_view()
        .expect("expected SELECT solutions")
        .1
        .len()
}

/// The size of any result: its rows, its graph's quads, or `1` for a `true` `ASK`.
pub fn result_size(result: &impl ResultView) -> usize {
    result.output_size()
}

/// One result cell, rendered for comparison: `<iri>`, a literal's bare lexical form,
/// `_:label`, the `Debug` form of a triple term, and `UNBOUND` — a distinct, assertable
/// value — for an unbound cell.
pub fn render_cell(value: Option<&TermValue>) -> String {
    match value {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(iri)) => format!("<{iri}>"),
        Some(TermValue::Literal { lexical_form, .. }) => lexical_form.clone(),
        Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
        Some(other) => format!("{other:?}"),
    }
}

/// A solution keyed by variable name, each cell rendered.
pub type Row = BTreeMap<String, String>;

/// A `SELECT` result's rows as variable-name-keyed maps with each cell rendered by
/// `render`, SORTED — the rows compare as a set, never by column or row position.
pub fn sorted_rows(result: &impl ResultView, render: fn(Option<&TermValue>) -> String) -> Vec<Row> {
    let (variables, rows) = result.solutions_view().expect("expected SELECT solutions");
    let mut out: Vec<Row> = rows
        .iter()
        .map(|row| {
            variables
                .iter()
                .cloned()
                .zip(row.iter().map(|c| render(c.as_ref())))
                .collect()
        })
        .collect();
    out.sort();
    out
}

/// One expected row from `(variable, rendered-value)` pairs.
pub fn row(pairs: &[(&str, &str)]) -> Row {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

// ── governance ──────────────────────────────────────────────────────────────

/// A stop signal that reports clear for its first `quiet` polls and `Cancelled` for every
/// poll after that.
///
/// It honours the latching contract — once it says `Some`, it says `Some` forever — while
/// letting a test place the firing edge at a chosen poll.
#[derive(Debug)]
pub struct PollCountdown {
    quiet: usize,
    polls: AtomicUsize,
    latched: AtomicBool,
}

impl PollCountdown {
    /// A countdown that fires on poll `quiet + 1`.
    pub fn new(quiet: usize) -> Arc<Self> {
        Arc::new(Self {
            quiet,
            polls: AtomicUsize::new(0),
            latched: AtomicBool::new(false),
        })
    }

    /// Polls observed before the signal latched, including its firing poll.
    pub fn polls(&self) -> usize {
        self.polls.load(Ordering::Relaxed)
    }
}

impl StopSignal for PollCountdown {
    fn poll(&self) -> Option<StopCause> {
        if self.latched.load(Ordering::Relaxed) {
            return Some(StopCause::Cancelled);
        }
        if self.polls.fetch_add(1, Ordering::Relaxed) < self.quiet {
            return None;
        }
        self.latched.store(true, Ordering::Relaxed);
        Some(StopCause::Cancelled)
    }
}

/// Run `body` with the default panic-hook stderr dump suppressed, for an EXPECTED, caught
/// panic.
pub fn without_panic_output<R>(body: impl FnOnce() -> R) -> R {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let out = body();
    std::panic::set_hook(default_hook);
    out
}

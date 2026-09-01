// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What row ceiling the evaluator actually offers this crate's relation, and on
//! which query shapes.
//!
//! The ceiling is the seam's "tell the index that only ten rows matter" licence,
//! and it is an **optimization**: the relation may honour it or ignore it, and
//! the answer is the same either way. So this file is written as an
//! **observation**, not as a requirement. It records, per invocation, the
//! ceiling the engine offered a relation that delegates to
//! [`TextSearchRelation`], and reports it. What it *does* assert as a
//! requirement is the one thing that is not optional: that the answer under a
//! `LIMIT` is exactly the prefix of the answer without one.
//!
//! The recorder is modelled on the evaluator's own `CeilingSpy`
//! (`purrdf-sparql-eval`, `property_fn_eval.rs`) — "a relation that records the
//! row ceiling of every invocation it is opened with" — except that this one
//! delegates to the real relation instead of replaying a fixture table, so what
//! it records is what this crate is offered.

use std::sync::{Arc, Mutex};

use pretty_assertions::assert_eq;
use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    EvalError, NativeSparqlEngine, PfArgs, PfArity, PfCursor, PropertyFunction,
    PropertyFunctionRegistry, QueryOptions, Volatility,
};
use purrdf_text::{GraphSelector, TextIndex, TextIndexConfig, TextSearchRelation};

/// The caller-supplied predicate this host calls ranked retrieval by.
const SEARCH: &str = "http://example.org/pf#search";

/// The one predicate whose literals the fixture indexes.
const NOTE: &str = "http://example.org/note";

/// A second, ordinary data predicate — the one the correlated shape drives the
/// relation from.
const KIND: &str = "http://example.org/kind";

/// The datatype `?rank` comes back as.
const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

// ── the recorder ─────────────────────────────────────────────────────────────

/// A relation that delegates every method to a real [`TextSearchRelation`] while
/// recording the ceiling each invocation was opened with.
///
/// Delegation rather than a fixture table matters: the declared arity, modes,
/// volatility and — critically — the declared row bound are the real ones, and
/// the row bound is an input to the evaluator's own admission arithmetic. A
/// stand-in that declared something else would be measuring a different query.
#[derive(Debug)]
struct CeilingRecorder {
    /// The relation under observation.
    inner: TextSearchRelation,
    /// The ceiling of every invocation, in the order they were opened.
    ceilings: Mutex<Vec<Option<u64>>>,
}

impl CeilingRecorder {
    /// A recorder around ranked retrieval over `index`.
    fn new(index: Arc<TextIndex>) -> Self {
        Self {
            inner: TextSearchRelation::new(index),
            ceilings: Mutex::new(Vec::new()),
        }
    }

    /// What was recorded, in invocation order.
    fn ceilings(&self) -> Vec<Option<u64>> {
        self.ceilings.lock().expect("uncontended").clone()
    }
}

impl PropertyFunction for CeilingRecorder {
    fn volatility(&self) -> Volatility {
        self.inner.volatility()
    }

    fn arity(&self) -> PfArity {
        self.inner.arity()
    }

    fn modes(&self) -> &[BindingPattern] {
        self.inner.modes()
    }

    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        self.inner.rows_per_invocation(mode)
    }

    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.ceilings.lock().expect("uncontended").push(ceiling);
        self.inner.open(args, ceiling)
    }
}

// ── rendering ────────────────────────────────────────────────────────────────

/// One answer cell in an exact, unambiguous textual form.
fn render(cell: Option<&TermValue>) -> String {
    match cell {
        None => "UNBOUND".to_owned(),
        Some(TermValue::Iri(iri)) => format!("<{iri}>"),
        Some(TermValue::Blank { label, scope }) => format!("_:{label}/{}", scope.ordinal()),
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        }) => match language {
            Some(tag) => format!("{lexical_form:?}@{tag}"),
            None if datatype == "http://www.w3.org/2001/XMLSchema#string" => {
                format!("{lexical_form:?}")
            }
            None => format!("{lexical_form:?}^^<{datatype}>"),
        },
        Some(TermValue::Triple { s, p, o }) => format!(
            "<<{} {} {}>>",
            render(Some(s)),
            render(Some(p)),
            render(Some(o))
        ),
    }
}

/// A typed literal cell as [`render`] writes it.
fn typed(lexical: &str, datatype: &str) -> String {
    format!("{lexical:?}^^<{datatype}>")
}

/// An IRI cell under the fixture namespace, as [`render`] writes it.
fn subject(local: &str) -> String {
    format!("<http://example.org/{local}>")
}

// ── the fixture ──────────────────────────────────────────────────────────────

/// Five documents of four tokens each in one partition, four of which the needle
/// `"quick brown"` retrieves, ranking `1, 2, 3, 4` in subject order:
///
/// ```text
/// d1  "quick quick quick brown"   tf(quick) = 3, holds brown
/// d2  "quick quick brown fox"     tf(quick) = 2, holds brown
/// d3  "quick brown fox jumps"     tf(quick) = 1, holds brown
/// d4  "quick fox jumps high"      tf(quick) = 1, holds NO brown
/// d5  "lazy dog sleeps late"      holds neither, so it is not a row
/// ```
///
/// Every document also carries `ex:kind ex:doc`, an ordinary data triple with
/// nothing to do with the index — that is what the correlated shape drives the
/// relation from, one invocation per document.
fn corpus() -> Arc<RdfDataset> {
    const ROWS: [(&str, &str); 5] = [
        ("d1", "quick quick quick brown"),
        ("d2", "quick quick brown fox"),
        ("d3", "quick brown fox jumps"),
        ("d4", "quick fox jumps high"),
        ("d5", "lazy dog sleeps late"),
    ];
    let mut builder = RdfDatasetBuilder::new();
    let note = builder.intern_iri(NOTE);
    let kind = builder.intern_iri(KIND);
    let document = builder.intern_iri("http://example.org/doc");
    for (local, text) in ROWS {
        let s = builder.intern_iri(&format!("http://example.org/{local}"));
        let o = builder.intern_literal(RdfLiteral::simple(text));
        builder.push_quad(s, note, o, None);
        builder.push_quad(s, kind, document, None);
    }
    builder.freeze().expect("the fixture must validate")
}

/// An index over the fixture.
fn index_of(dataset: &RdfDataset) -> Arc<TextIndex> {
    Arc::new(
        TextIndex::from_dataset(
            dataset,
            &TextIndexConfig::new(vec![TermValue::iri(NOTE)], GraphSelector::Any)
                .expect("the fixture configuration is well formed"),
        )
        .expect("the fixture indexes"),
    )
}

/// Evaluate `query` through a fresh recorder, returning the rendered answer and
/// the ceilings that were offered.
fn drive(dataset: &RdfDataset, query: &str) -> (Vec<Vec<String>>, Vec<Option<u64>>) {
    let recorder = Arc::new(CeilingRecorder::new(index_of(dataset)));
    let mut relations = PropertyFunctionRegistry::new();
    relations.register(SEARCH.to_owned(), recorder.clone());

    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions {
                property_functions: &relations,
                ..QueryOptions::EMPTY
            },
        )
        .unwrap_or_else(|error| panic!("the query must evaluate: {error}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions");
    };
    let rendered = rows
        .iter()
        .map(|row| row.iter().map(|cell| render(cell.as_ref())).collect())
        .collect();
    (rendered, recorder.ceilings())
}

/// The standalone call, projecting `?doc` and `?rank`, with `tail` appended.
fn standalone(tail: &str) -> String {
    format!(
        "SELECT ?doc ?rank WHERE {{ \
         ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }}{tail}"
    )
}

/// The whole answer, which every prefix assertion below is measured against.
fn full_answer() -> Vec<Vec<String>> {
    vec![
        vec![subject("d1"), typed("1", INTEGER)],
        vec![subject("d2"), typed("2", INTEGER)],
        vec![subject("d3"), typed("3", INTEGER)],
        vec![subject("d4"), typed("4", INTEGER)],
    ]
}

// ── the observations ─────────────────────────────────────────────────────────

/// A bare `LIMIT` offers a ceiling: the node's own output is the answer's
/// prefix, so stopping early is sound and the licence is granted.
#[test]
fn a_bare_limit_offers_a_ceiling() {
    let dataset = corpus();
    let (rows, ceilings) = drive(&dataset, &standalone(" LIMIT 3"));
    assert_eq!(rows, full_answer()[..3].to_vec());
    assert_eq!(
        ceilings,
        vec![Some(3)],
        "one invocation, driven over the identity table, offered the whole limit"
    );
}

/// `ORDER BY` + `LIMIT` offers **no** ceiling.
///
/// `crates/sparql-eval/src/governor/soundness.rs` calls this "a top-k problem
/// with no certified lower bound": the row that sorts first can be produced
/// last, so no prefix of this node's output is the answer's prefix, and the plan
/// licenses nothing.
///
/// The `ORDER BY DESC(?score)` written here is **redundant** — emission order is
/// already rank order, and rank order is score order within a partition — so
/// this query pays for a sort it did not need and gives up the pushdown as well.
/// That is precisely why the bare `LIMIT` above is the documented idiom for
/// "the top three", and why `ORDER BY ?rank` (which states the order rather than
/// changing it) is the idiom for reproducibility.
#[test]
fn order_by_limit_offers_no_ceiling() {
    let dataset = corpus();
    let (rows, ceilings) = drive(&dataset, &standalone(" ORDER BY DESC(?score) LIMIT 3"));
    assert_eq!(
        rows,
        full_answer()[..3].to_vec(),
        "the answer is unchanged; only the licence is"
    );
    assert_eq!(
        ceilings,
        vec![None],
        "a ceiling under a sort would let a relation stop before producing the \
         row that sorts first"
    );
}

/// A variable repeated across two argument positions withholds the ceiling.
///
/// `args_are_admission_transparent` returns false for a repeated slot: the
/// relation is handed two FREE positions and cannot know they must agree, so a
/// licence to stop after `k` rows would be counted against rows the engine then
/// drops for a reason the relation never saw.
///
/// Here `?x` is both the document subject (an IRI) and `?matched` (an
/// `xsd:integer`), so every row the relation emits is dropped and the answer is
/// empty. The relation is still opened, and what it is opened with is the
/// measurement.
#[test]
fn a_repeated_variable_withholds_the_ceiling() {
    let dataset = corpus();
    let (rows, ceilings) = drive(
        &dataset,
        &format!("SELECT ?x WHERE {{ ?x <{SEARCH}> ( \"quick brown\" ?s ?r ?l ?x ) }} LIMIT 1"),
    );
    assert_eq!(
        rows,
        Vec::<Vec<String>>::new(),
        "an IRI subject can never equal an xsd:integer match count"
    );
    assert_eq!(
        ceilings,
        vec![None],
        "the engine withholds a ceiling it would have to trust the relation to \
         account for against something the relation was never told"
    );
}

/// The requirement the observations above are optimizations of: for every `k`,
/// the answer under `LIMIT k` is exactly the first `k` rows of the answer
/// without one.
///
/// Swept past the end of the result, and compared as whole row vectors rather
/// than by length, so a relation that honoured a ceiling by dropping the wrong
/// rows would fail here even where the counts agreed.
#[test]
fn limit_k_equals_the_prefix_of_the_unlimited_answer() {
    let dataset = corpus();
    let (unlimited, _) = drive(&dataset, &standalone(""));
    assert_eq!(unlimited, full_answer());

    for k in 0..=(unlimited.len() + 2) {
        let (limited, _) = drive(&dataset, &standalone(&format!(" LIMIT {k}")));
        assert_eq!(
            limited,
            unlimited[..k.min(unlimited.len())].to_vec(),
            "LIMIT {k} is not the prefix"
        );
    }
}

/// A correlated call driven by a preceding data pattern is opened once per
/// driving row, and the ceiling **shrinks**: the evaluator offers what is left
/// of the node's licence once the invocations already driven have contributed.
///
/// The accumulated bag is the node's output, so the tightest honest number is
/// what this invocation could still add to it. Three documents each contribute
/// exactly one row, so the sequence decreases by one each time and the fourth
/// invocation never happens — the node had already produced its three rows.
#[test]
fn a_ceiling_shrinks_across_invocations() {
    let dataset = corpus();
    let (rows, ceilings) = drive(
        &dataset,
        &format!(
            "SELECT ?doc ?rank WHERE {{ \
             ?doc <{KIND}> <http://example.org/doc> . \
             ?doc <{SEARCH}> ( \"quick brown\" ?score ?rank ?lang ?matched ) }} LIMIT 3"
        ),
    );
    assert_eq!(rows, full_answer()[..3].to_vec());
    assert_eq!(
        ceilings,
        vec![Some(3), Some(2), Some(1)],
        "each invocation is offered the limit minus what its predecessors \
         already put in the bag"
    );
}

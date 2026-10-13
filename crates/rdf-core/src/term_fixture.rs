// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generated RDF terms for the traversal regression tests of every crate that
//! walks a [`TermValue`]: arbitrarily shaped terms from a caller's
//! deterministic stream, and triple terms nested to any depth.
//!
//! This is test support, not API: it is hidden from the documentation and
//! carries no stability promise. It lives in the library so that each crate's
//! tests import one definition rather than compiling a copy of it; the
//! pseudo-random stream is the caller's, so the library takes no test-only
//! dependency.

use std::cell::Cell;
use std::sync::Arc;

use crate::ir::{QuadIds, RdfDataset, RdfDatasetBuilder, TermId, TermRef};
use crate::{
    BlankScope, DatasetView, GraphMatch, RdfStoreCapabilities, RdfTextDirection, TermBox,
    TermValue, ViewOperationStatus,
};

/// Which positions of a generated triple term [`term_value`] may fill with which terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermShape {
    /// Any term in any position, a non-IRI predicate included — a walk that refuses one
    /// meets it.
    Any,
    /// An IRI predicate, and any term as subject and object.
    IriPredicates,
    /// RDF 1.2 as a frozen dataset admits it: an IRI or blank-node subject, an IRI
    /// predicate, and triple terms nested only in the object.
    WellFormed,
}

/// A term value drawn from the caller's deterministic stream — `next` advances
/// `state` and returns the next draw — holding at most `budget` triple terms, its
/// triple terms shaped by `shape`.
///
/// Every IRI is absolute and under `http://example.org/`; blank nodes come in two
/// labels over three scopes; literals are plain, language-tagged (with and without a
/// base direction) and typed, and one lexical form carries a quote, a backslash, a
/// newline and a non-ASCII letter, so a writer's escaping is exercised.
///
/// This draws recursively: a generated term nests at most `budget` levels, and the
/// generator is for tests of the walks, not one of them.
pub fn term_value(
    state: &mut u64,
    next: fn(&mut u64) -> u64,
    budget: &mut usize,
    shape: TermShape,
) -> TermValue {
    use purrdf_iri::vocab::rdf::DIR_LANG_STRING as RDF_DIR_LANG_STRING;
    use purrdf_iri::vocab::rdf::LANG_STRING as RDF_LANG_STRING;
    use purrdf_xsd::datatype::XSD_INTEGER;
    use purrdf_xsd::datatype::XSD_STRING;
    let draw = |state: &mut u64, n: u64| next(state) % n;
    if *budget > 0 && draw(state, 3) == 0 {
        *budget -= 1;
        let s = match shape {
            TermShape::WellFormed if draw(state, 2) == 0 => {
                TermValue::iri(format!("http://example.org/i{}", draw(state, 3)))
            }
            TermShape::WellFormed => TermValue::Blank {
                label: format!("b{}", draw(state, 2)),
                scope: BlankScope(u32::try_from(draw(state, 3)).unwrap_or(0)),
            },
            TermShape::Any | TermShape::IriPredicates => term_value(state, next, budget, shape),
        };
        let p = match shape {
            TermShape::Any => term_value(state, next, budget, shape),
            TermShape::IriPredicates | TermShape::WellFormed => {
                TermValue::iri(format!("http://example.org/p{}", draw(state, 2)))
            }
        };
        let o = term_value(state, next, budget, shape);
        return TermValue::Triple {
            s: TermBox::new(s),
            p: TermBox::new(p),
            o: TermBox::new(o),
        };
    }
    let literal =
        |lexical: &str, datatype: &str, language: Option<&str>, direction| TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: datatype.to_owned(),
            language: language.map(str::to_owned),
            direction,
        };
    match draw(state, 8) {
        0 | 1 => TermValue::iri(format!("http://example.org/i{}", draw(state, 3))),
        2 => TermValue::Blank {
            label: format!("b{}", draw(state, 2)),
            scope: BlankScope(u32::try_from(draw(state, 3)).unwrap_or(0)),
        },
        3 => literal("plain", XSD_STRING, None, None),
        4 => literal("tagged", RDF_LANG_STRING, Some("en"), None),
        5 => literal(
            "directed",
            RDF_DIR_LANG_STRING,
            Some("ar"),
            Some(RdfTextDirection::Rtl),
        ),
        6 => literal("7", XSD_INTEGER, None, None),
        _ => literal("q\"b\\n\n\u{e9}", XSD_STRING, None, None),
    }
}

/// The IRI `http://example.org/{name}` as a term value: the fixture IRI every
/// suite that builds terms by name spells its terms with.
#[must_use]
pub fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("http://example.org/{name}"))
}

/// One `(subject, predicate, object)` triple of term values, as the page fixtures
/// take them.
pub type Triple = (TermValue, TermValue, TermValue);

/// Intern one dataset-independent value into `builder`, triple terms included: the
/// by-value inverse every paged and packed fixture builds its pages with.
pub fn intern_value(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::resume(
        &mut resident,
        builder
            .interner_buffer_bytes()
            .expect("resident interner layout"),
    );
    intern_value_with_memory(builder, value, &mut memory)
        .expect("resident term interning allocation failed")
}

/// Freeze one page (or a single reference dataset) from `triples`, all in the
/// default graph.
///
/// # Panics
///
/// If the builder refuses to freeze the triples.
#[must_use]
pub fn build_page(triples: &[Triple]) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    for (s, p, o) in triples {
        let s = intern_value(&mut builder, s);
        let p = intern_value(&mut builder, p);
        let o = intern_value(&mut builder, o);
        builder.push_quad(s, p, o, None);
    }
    builder.freeze().expect("page freeze")
}

/// The pack bytes of `dataset`: the fixture-to-pack path every pack test builds
/// its `PackView` from.
///
/// # Panics
///
/// If the dataset does not pack, which a well-formed fixture always does.
#[must_use]
pub fn pack_bytes(dataset: &RdfDataset) -> Vec<u8> {
    crate::PackBuilder::build_bytes(dataset)
        .expect("pack build must succeed for a well-formed fixture")
}

/// The empty dataset: no quad, no graph — the dataset a query answered entirely by
/// a registered relation or a `VALUES` block is evaluated over.
///
/// # Panics
///
/// Never: an empty default graph is structurally valid.
#[must_use]
pub fn empty_dataset() -> Arc<RdfDataset> {
    RdfDatasetBuilder::new()
        .freeze()
        .expect("an empty default graph is structurally valid")
}

/// A dataset holding the one default-graph quad `subject predicate object`, each
/// an absolute IRI: the smallest dataset a query can match a row in.
#[must_use]
pub fn one_quad(subject: &str, predicate: &str, object: &str) -> Arc<RdfDataset> {
    build_page(&[(
        TermValue::iri(subject),
        TermValue::iri(predicate),
        TermValue::iri(object),
    )])
}

/// A dataset whose terms and named graphs differ: `s p phantom{tag}` in the default
/// graph, `s p o` in named graph `g{tag}`, and `empty{tag}` declared with no quads, all
/// under `http://example.org/`. `phantom{tag}` is a term that names no graph; `tag`
/// keeps several such datasets' graphs apart.
///
/// # Panics
///
/// Never: every term is an absolute IRI in a valid position.
#[must_use]
pub fn graph_slots(tag: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let [s, p, o, phantom, graph, empty] = [
        "s".to_owned(),
        "p".to_owned(),
        "o".to_owned(),
        format!("phantom{tag}"),
        format!("g{tag}"),
        format!("empty{tag}"),
    ]
    .map(|name| intern_value(&mut builder, &iri(&name)));
    builder.push_quad(s, p, phantom, None);
    builder.push_quad(s, p, o, Some(graph));
    builder.declare_named_graph(empty);
    builder.freeze().expect("graph-slot fixture freezes")
}

/// The graph-addressing corpus the constant-`GRAPH` benches share: `graphs` named
/// graphs `g{tag}{i}`, each holding `s p o{tag}{i}`, and in the default graph a row
/// `s{i} p g{tag}{i}` naming each graph as an object, plus `s p phantom` whose object
/// names no graph. A `LATERAL { GRAPH ?o { ... } }` over the default graph therefore
/// addresses every graph by constant, once per row.
///
/// # Panics
///
/// Never: every term is an absolute IRI in a valid position.
#[must_use]
pub fn graph_membership_corpus(tag: &str, graphs: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let mut term = |name: String| intern_value(&mut builder, &iri(&name));
    let [s, p, phantom] = ["s", "p", "phantom"].map(|name| term(name.to_owned()));
    let mut rows = vec![(s, p, phantom, None)];
    for i in 0..graphs {
        let graph = term(format!("g{tag}{i}"));
        rows.push((term(format!("s{i}")), p, graph, None));
        rows.push((s, p, term(format!("o{tag}{i}")), Some(graph)));
    }
    for (s, p, o, g) in rows {
        builder.push_quad(s, p, o, g);
    }
    builder.freeze().expect("graph-membership corpus freezes")
}

/// Split `triples` round-robin across `page_count` quad-disjoint pages, so
/// consecutive triples land on different pages.
#[must_use]
pub fn split_pages(triples: &[Triple], page_count: usize) -> Vec<Arc<RdfDataset>> {
    let mut buckets: Vec<Vec<Triple>> = vec![Vec::new(); page_count];
    for (index, triple) in triples.iter().enumerate() {
        buckets[index % page_count].push(triple.clone());
    }
    buckets.iter().map(|bucket| build_page(bucket)).collect()
}

/// A term value nesting `levels` triple terms, each holding the next in its object
/// slot under the subject `http://example.org/s` and the predicate
/// `http://example.org/p`, around the innermost object `http://example.org/o`. Built
/// by a loop, so any depth is cheap to make.
#[must_use]
pub fn triple_chain(levels: usize) -> TermValue {
    let mut term = TermValue::iri("http://example.org/o");
    for _ in 0..levels {
        term = TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/s")),
            p: TermBox::new(TermValue::iri("http://example.org/p")),
            o: TermBox::new(term),
        };
    }
    term
}

/// A view over a real dataset that, when `foreign` is set, answers every literal with
/// `datatype` as its datatype id — pointed at a term that is not an IRI, that is what a
/// view hands back for an id another view minted. Every other answer is the dataset's.
#[derive(Debug)]
pub struct ForeignDatatypeView {
    /// The dataset every answer comes from.
    pub inner: Arc<RdfDataset>,
    /// The id every literal names as its datatype while `foreign` is set.
    pub datatype: TermId,
    /// Whether literals name `datatype` rather than their own datatype.
    pub foreign: bool,
}

impl DatasetView for ForeignDatatypeView {
    type Id = TermId;
    type ReadError = std::convert::Infallible;
    type TermGuard<'a>
        = TermRef<'a, Self::Id>
    where
        Self: 'a;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }

    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        Ok(match self.inner.as_ref().resolve(id) {
            TermRef::Literal {
                lexical,
                language,
                direction,
                ..
            } if self.foreign => TermRef::Literal {
                lexical,
                datatype: self.datatype,
                language,
                direction,
            },
            other => other,
        })
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<Self::Id>, Self::ReadError> {
        Ok(self.inner.as_ref().term_id_by_value(value))
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }

    fn probe_plan(&self, _s: bool, _p: bool, _o: bool, _g: GraphMatch) {}

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }

    fn term_count(&self) -> u64 {
        self.inner.term_count()
    }
}

/// The operational root cause a [`RowBudget`] reports once its rows are spent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeFault(pub &'static str);

impl std::fmt::Display for ProbeFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ProbeFault {}

/// The row budget of a view that faults the way a lazy operational backend does:
/// once its rows are spent it STOPS YIELDING rather than erroring, so nothing in
/// the row stream tells the truncation from an honest end, and only the
/// operational checkpoint ([`Self::status`]) can.
#[derive(Debug)]
pub struct RowBudget {
    rows: Cell<usize>,
    faulted: Cell<bool>,
}

impl RowBudget {
    /// A budget of `rows` ordinary rows.
    #[must_use]
    pub const fn new(rows: usize) -> Self {
        Self {
            rows: Cell::new(rows),
            faulted: Cell::new(false),
        }
    }

    /// A budget already spent: the view has failed before its first read.
    #[must_use]
    pub const fn faulted() -> Self {
        Self {
            rows: Cell::new(usize::MAX),
            faulted: Cell::new(true),
        }
    }

    /// Spend one row; `false` (and the view faulted) once none is left.
    pub fn spend(&self) -> bool {
        if self.faulted.get() {
            return false;
        }
        match self.rows.get().checked_sub(1) {
            Some(left) => {
                self.rows.set(left);
                true
            }
            None => {
                self.faulted.set(true);
                false
            }
        }
    }

    /// `rows` for as long as the budget lasts: the stream simply ends when it runs
    /// out, and the view is faulted from then on.
    pub fn take<'a, I: Iterator + 'a>(&'a self, rows: I) -> impl Iterator<Item = I::Item> + 'a {
        rows.take_while(|_| self.spend())
    }

    /// The operational checkpoint: `Failed` once a read ran past the budget (or it
    /// was spent from the start), `Ready` otherwise, with the rows left as evidence.
    pub fn status(&self) -> ViewOperationStatus<ProbeFault, usize> {
        let evidence = self.rows.get();
        if self.faulted.get() {
            ViewOperationStatus::Failed {
                error: ProbeFault("the probe view exhausted its row budget"),
                evidence,
            }
        } else {
            ViewOperationStatus::Ready { evidence }
        }
    }

    /// The same typed sticky root returned by point reads after row refusal.
    #[must_use]
    pub fn read_error(&self) -> Option<ProbeFault> {
        self.status().error().cloned()
    }
}

/// Intern borrowed values at their original producer, including every triple
/// component. Temporary walk arrays die before their original admission shrinks.
///
/// # Errors
/// Returns physical layout, admission or allocator refusal.
pub fn intern_value_with_memory<S: purrdf_lex::allocation::Admission + ?Sized>(
    builder: &mut RdfDatasetBuilder,
    value: &TermValue,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> Result<TermId, purrdf_lex::allocation::StorageError> {
    enum Step<'a> {
        Enter(&'a TermValue),
        Assemble,
    }
    let mut steps = Vec::new();
    let mut answers = Vec::new();
    memory.push(&mut steps, Step::Enter(value))?;
    while let Some(step) = steps.pop() {
        let answer = match step {
            Step::Enter(TermValue::Triple { s, p, o }) => {
                for next in [
                    Step::Assemble,
                    Step::Enter(o),
                    Step::Enter(p),
                    Step::Enter(s),
                ] {
                    memory.push(&mut steps, next)?;
                }
                continue;
            }
            Step::Enter(TermValue::Iri(iri)) => builder.intern_iri_with_memory(iri, memory)?,
            Step::Enter(TermValue::Blank { label, scope }) => {
                builder.intern_blank_with_memory(label, *scope, memory)?
            }
            Step::Enter(TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            }) => builder.intern_literal_parts_with_memory(
                lexical_form,
                Some(datatype),
                language.as_deref(),
                *direction,
                memory,
            )?,
            Step::Assemble => {
                let o = answers.pop().expect("the object has been interned");
                let p = answers.pop().expect("the predicate has been interned");
                let s = answers.pop().expect("the subject has been interned");
                builder.intern_triple_with_memory(s, p, o, memory)?
            }
        };
        memory.push(&mut answers, answer)?;
    }
    let output = answers.pop().expect("the root has been interned");
    memory.release_vec(answers)?;
    memory.release_vec(steps)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::RowBudget;

    #[test]
    fn a_refused_row_budget_stays_closed_across_new_streams() {
        let budget = RowBudget::new(1);
        assert_eq!(budget.take([1, 2].into_iter()).collect::<Vec<_>>(), [1]);
        let error = budget.read_error().expect("the second row was refused");
        assert_eq!(budget.take([3, 4].into_iter()).next(), None);
        assert_eq!(budget.read_error(), Some(error));

        let failed = RowBudget::faulted();
        assert_eq!(failed.take(std::iter::once(5)).next(), None);
        assert!(failed.read_error().is_some());
    }
}

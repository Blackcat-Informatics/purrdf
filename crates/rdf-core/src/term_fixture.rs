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

use crate::ir::{QuadIds, QuadRef, RdfDataset, RdfDatasetBuilder, TermId, TermRef};
use crate::{
    BlankScope, DatasetView, GraphMatch, RdfLiteral, RdfStoreCapabilities, RdfTextDirection,
    TermBox, TermValue, ViewOperationStatus,
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
    match value {
        TermValue::Iri(iri) => builder.intern_iri(iri),
        TermValue::Blank { label, scope } => builder.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => builder.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let s = intern_value(builder, s);
            let p = intern_value(builder, p);
            let o = intern_value(builder, o);
            builder.intern_triple(s, p, o)
        }
    }
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
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        DatasetView::quad_refs(&*self.inner)
    }

    fn resolve(&self, id: TermId) -> TermRef<'_> {
        match self.inner.resolve(id) {
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
        }
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        self.inner.term_id_by_value(value)
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

    fn term_count(&self) -> usize {
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
}

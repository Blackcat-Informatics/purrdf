// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical document is the sorted, deduplicated set of the lines
//! `write_component` renders, however the lines are held while it is built.
//!
//! Every case here compares the document an entry point returns with an
//! independent oracle: each component rendered as text by the existing
//! renderer, then a plain sort, dedup and concatenation. Views that present
//! text a validated dataset cannot (an unvalidated language tag, an IRI the
//! renderer must escape) come from [`Table`], which answers every resolution
//! from a table of its own.

use super::*;
use crate::dataset_view::GraphMatch;
use crate::ir::QuadIds;
use crate::ir::RdfDatasetBuilder;
use crate::{
    CompositeDatasetView, DatasetMut as _, MutableDataset, QuadValues, RdfLiteral,
    RdfStoreCapabilities, TermValue, ViewLimits,
};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

/// The reference the new construction must equal: one rendered line per
/// component, sorted bytewise and deduplicated, with the labels the search issued.
fn oracle<D: DatasetView>(view: &D) -> String {
    let run = SearchRun::new(SearchPolicy::Unlimited, None);
    let state = CanonState::new(
        view,
        CanonScope::Dataset,
        CanonPresentation::Overlay,
        CanonHash::Sha256,
        &run,
    )
    .issue_labels()
    .expect("the oracle's labels issue");
    let mut lines: Vec<String> = Vec::new();
    collect_components::<_, D::ReadError>(
        view,
        CanonScope::Dataset,
        CanonPresentation::Overlay,
        None,
        &mut |comp| {
            let mut line = String::new();
            state.write_component(
                comp,
                BlankRender::Canonical {
                    issuer: &state.canonical,
                },
                &mut line,
                &mut Memory::new(&mut purrdf_lex::allocation::Resident),
            )?;
            lines.push(line);
            Ok(())
        },
    )
    .expect("the oracle renders every component");
    lines.sort_unstable();
    lines.dedup();
    lines.concat()
}

fn document<D: DatasetView>(view: &D) -> Result<String, CanonError<D::ReadError>> {
    try_canonicalize_view(
        view,
        CanonHash::Sha256,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )
    .map(|canonical| canonical.nquads().to_owned())
}

/// The document equals the oracle's; returns it for a case that also pins bytes.
fn check<D: DatasetView>(view: &D) -> String {
    let expected = oracle(view);
    let actual = document(view).expect("the view canonicalizes");
    assert!(
        actual == expected,
        "the document differs from the sorted, deduplicated rendering of its lines"
    );
    actual
}

// ---------------------------------------------------------------------------
// A view that presents terms a validated dataset cannot.

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableFault;

impl std::fmt::Display for TableFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the table view faulted")
    }
}

impl std::error::Error for TableFault {}

impl purrdf_hash::error::StructuredError for TableFault {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        visitor.variant("table-fault");
    }
}

enum Over {
    Iri(String),
    Literal {
        lexical: String,
        language: Option<String>,
    },
}

impl Over {
    /// The same term with one more byte, for a source that disagrees with itself.
    fn longer(&self) -> Self {
        match self {
            Self::Iri(text) => Self::Iri(format!("{text}!")),
            Self::Literal { lexical, language } => Self::Literal {
                lexical: format!("{lexical}!"),
                language: language.clone(),
            },
        }
    }
}

/// A dataset whose placeholder IRIs are answered as the terms registered for
/// them. `fail_at` makes the n-th resolution fail and latch; `flicker` makes one
/// term spell one byte longer on its n-th read only.
struct Table {
    inner: Arc<RdfDataset>,
    datatype: TermId,
    over: Vec<(TermId, Over)>,
    flicker: Option<(TermId, usize, Over)>,
    fail_at: Option<usize>,
    reads: Cell<usize>,
    flicker_reads: Cell<usize>,
    latched: RefCell<Option<TableFault>>,
}

impl Table {
    fn resolved<'a>(&'a self, over: &'a Over) -> TermRef<'a, TermId> {
        match over {
            Over::Iri(text) => TermRef::Iri(text),
            Over::Literal { lexical, language } => TermRef::Literal {
                lexical,
                datatype: self.datatype,
                language: language.as_deref(),
                direction: None,
            },
        }
    }
}

impl DatasetView for Table {
    type Id = TermId;
    type ReadError = TableFault;
    type TermGuard<'a>
        = TermRef<'a, TermId>
    where
        Self: 'a;
    type ProbePlan = ();

    fn read_error(&self) -> Option<TableFault> {
        self.latched.borrow().clone()
    }

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }

    fn resolve(&self, id: TermId) -> Result<Self::TermGuard<'_>, TableFault> {
        let read = self.reads.get() + 1;
        self.reads.set(read);
        if self.fail_at == Some(read) {
            *self.latched.borrow_mut() = Some(TableFault);
            return Err(TableFault);
        }
        if let Some((target, nth, longer)) = &self.flicker
            && *target == id
        {
            let seen = self.flicker_reads.get() + 1;
            self.flicker_reads.set(seen);
            if seen == *nth {
                return Ok(self.resolved(longer));
            }
        }
        if let Some((_, over)) = self.over.iter().find(|(at, _)| *at == id) {
            return Ok(self.resolved(over));
        }
        DatasetView::resolve(&*self.inner, id).map_err(|never| match never {})
    }

    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, TableFault> {
        DatasetView::term_id_by_value(&*self.inner, value).map_err(|never| match never {})
    }

    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl crate::WorkspaceReservation<Error = TableFault> + '_, TableFault> {
        Ok(crate::NoopReservation::<TableFault>::default())
    }

    fn reserve_owned_workspace(
        &self,
        _: u64,
    ) -> Option<Result<crate::OwnedWorkspaceReservation<TableFault>, TableFault>> {
        Some(Ok(crate::NoopReservation::owned()))
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

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.reifier_quads()
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.annotation_quads()
    }
}

/// Builds a [`Table`]: ordinary IRIs and blanks are real terms, and each
/// literal or irregular IRI is a placeholder answered from the table.
struct TableBuilder {
    b: RdfDatasetBuilder,
    datatype: TermId,
    over: Vec<(TermId, Over)>,
    next: usize,
}

impl TableBuilder {
    fn new() -> Self {
        let mut b = RdfDatasetBuilder::new();
        let datatype = b
            .intern_iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#langString")
            .unwrap();
        Self {
            b,
            datatype,
            over: Vec::new(),
            next: 0,
        }
    }

    fn placeholder(&mut self, over: Over) -> TermId {
        self.next += 1;
        let id = self
            .b
            .intern_iri(&format!("http://example.org/placeholder{}", self.next))
            .unwrap();
        self.over.push((id, over));
        id
    }

    /// An IRI the dataset admits as it is, or else one spelled as given.
    fn iri(&mut self, text: &str) -> TermId {
        if text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b":/._-%".contains(&byte))
        {
            self.b.intern_iri(text).unwrap()
        } else {
            self.raw_iri(text)
        }
    }

    /// An IRI spelled as given, whatever the renderer must do to it.
    fn raw_iri(&mut self, text: &str) -> TermId {
        self.placeholder(Over::Iri(text.to_owned()))
    }

    fn blank(&mut self, label: &str) -> TermId {
        self.b.intern_blank(label, BlankScope::DEFAULT).unwrap()
    }

    /// A language-tagged literal whose tag is not validated.
    fn tagged(&mut self, lexical: &str, tag: &str) -> TermId {
        self.placeholder(Over::Literal {
            lexical: lexical.to_owned(),
            language: Some(tag.to_owned()),
        })
    }

    fn quad(&mut self, s: TermId, p: TermId, o: TermId, g: Option<TermId>) {
        self.b.push_quad(s, p, o, g).unwrap();
    }

    fn build(self) -> Table {
        Table {
            inner: self.b.freeze().unwrap(),
            datatype: self.datatype,
            over: self.over,
            flicker: None,
            fail_at: None,
            reads: Cell::new(0),
            flicker_reads: Cell::new(0),
            latched: RefCell::new(None),
        }
    }
}

const S: &str = "http://example.org/s";
const P: &str = "http://example.org/p";
const G: &str = "http://example.org/g";

// ---------------------------------------------------------------------------
// The review's two counterexamples to ordering by symbol rank alone.

/// Rank order of whole symbols disagrees with byte order when a tag holds a
/// space: `"x"@en ` is a prefix of `"x"@en US `, yet the line holding the longer
/// symbol sorts first because its next byte (`U`) is below `_`. The document
/// must keep today's bytewise order, which the irregular line makes the merge
/// decide.
#[test]
fn counterexample_a_a_tag_with_a_space_keeps_the_bytewise_line_order() {
    let mut t = TableBuilder::new();
    let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
    let en =
        t.b.intern_literal(RdfLiteral::language_tagged("x", "en"))
            .unwrap();
    let blank_graph = t.blank("g");
    let spaced = t.tagged("x", "en US");
    t.quad(s, p, en, Some(blank_graph));
    t.quad(s, p, spaced, Some(g));
    let view = t.build();
    assert_eq!(
        check(&view),
        "<http://example.org/s> <http://example.org/p> \"x\"@en US <http://example.org/g> .\n\
         <http://example.org/s> <http://example.org/p> \"x\"@en _:c14n0 .\n"
    );
}

/// A tag that spells a whole following symbol makes two different symbol
/// sequences render the same bytes; the line must still appear once, however
/// many irregular spellings and regular spellings of it exist.
#[test]
fn counterexample_b_equal_lines_from_different_symbols_appear_once() {
    let mut t = TableBuilder::new();
    let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
    let en =
        t.b.intern_literal(RdfLiteral::language_tagged("x", "en"))
            .unwrap();
    let glued_one = t.tagged("x", "en <http://example.org/g>");
    let glued_two = t.tagged("x", "en <http://example.org/g>");
    let regular_twin = t.tagged("x", "en");
    t.quad(s, p, glued_one, None);
    t.quad(s, p, glued_two, None);
    t.quad(s, p, en, Some(g));
    t.quad(s, p, regular_twin, Some(g));
    let view = t.build();
    assert_eq!(
        check(&view),
        "<http://example.org/s> <http://example.org/p> \"x\"@en <http://example.org/g> .\n"
    );
}

/// Two irregular lines that differ only after an interior newline are two lines,
/// ordered by their bytes and not merged by a prefix.
#[test]
fn irregular_lines_that_differ_after_an_interior_newline_stay_apart_and_ordered() {
    let mut t = TableBuilder::new();
    let (s, p) = (t.iri(S), t.iri(P));
    let low = t.tagged("x", "en\nA");
    let high = t.tagged("x", "en\nB");
    let plain = t.tagged("x", "en");
    t.quad(s, p, high, None);
    t.quad(s, p, low, None);
    t.quad(s, p, plain, None);
    let document = check(&t.build());
    assert_eq!(
        document.lines().count(),
        5,
        "three lines, two with a newline"
    );
}

/// Tags that are odd but one token each: empty, `.\n`, a quote, a backslash. The
/// tokenizer must neither span the next symbol nor refuse the line.
#[test]
fn unusual_tags_keep_today_s_bytes() {
    for tag in ["", ".\n", "\"x", "a\\", "<", "x y", "x\ny", "\"", "\\\""] {
        let mut t = TableBuilder::new();
        let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
        let odd = t.tagged("x", tag);
        let plain =
            t.b.intern_literal(RdfLiteral::language_tagged("x", "en"))
                .unwrap();
        let blank = t.blank("b");
        t.quad(s, p, odd, None);
        t.quad(s, p, odd, Some(g));
        t.quad(s, p, plain, Some(blank));
        t.quad(blank, p, odd, Some(g));
        check(&t.build());
    }
}

// ---------------------------------------------------------------------------
// IRIs whose rendering is, or is not, their text between angle brackets.

/// Clean and unclean IRIs that share a prefix sort where their bytes put them:
/// `<http://e/a>` below `<http://e/a b>` below `<http://e/a\u{a0}>`.
#[test]
fn clean_and_unclean_iris_sharing_a_prefix_keep_bytewise_order() {
    let spellings = [
        "http://e/a",
        "http://e/a b",
        "http://e/a\u{a0}",
        "http://e/a\u{2028}",
        "http://e/a>",
        "http://e/a\"",
        "http://e/a\\",
        "http://e/a{",
        "http://e/ab",
        "http://e/a%20",
    ];
    let mut t = TableBuilder::new();
    let (s, p) = (t.iri(S), t.iri(P));
    for (at, spelling) in spellings.iter().enumerate() {
        let o = t.raw_iri(spelling);
        let plain = t.iri(spelling.split_whitespace().next().unwrap_or(spelling));
        t.quad(s, p, o, None);
        t.quad(plain, p, o, None);
        if at % 2 == 0 {
            let g = t.raw_iri(spelling);
            t.quad(s, p, plain, Some(g));
        }
    }
    check(&t.build());
}

// ---------------------------------------------------------------------------
// Valid datasets, through each view kind.

fn mixed() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p").unwrap();
    let q = b.intern_iri("https://example.org/q").unwrap();
    let s = b.intern_iri("https://example.org/s").unwrap();
    let blanks: Vec<_> = (0..12)
        .map(|k| {
            b.intern_blank(&format!("b{k}"), BlankScope::DEFAULT)
                .unwrap()
        })
        .collect();
    let t1 = b.intern_triple(s, p, blanks[0]).unwrap();
    let t2 = b.intern_triple(blanks[1], q, t1).unwrap();
    let t3 = b.intern_triple(blanks[2], p, t2).unwrap();
    b.push_quad(s, p, t3, None).unwrap();
    b.push_quad(blanks[3], q, t2, Some(blanks[4])).unwrap();
    for (k, pair) in blanks.windows(2).enumerate() {
        b.push_quad(pair[0], if k % 2 == 0 { p } else { q }, pair[1], None)
            .unwrap();
    }
    let list = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";
    let map = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map";
    let lits = [
        RdfLiteral::simple("plain"),
        RdfLiteral::simple("tab\t quote\" back\\ nl\n cr\r"),
        RdfLiteral::simple(""),
        RdfLiteral::language_tagged("hello", "en-us"),
        RdfLiteral::typed("42", "http://www.w3.org/2001/XMLSchema#integer"),
        RdfLiteral {
            lexical_form: "dir".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(crate::RdfTextDirection::Rtl),
        },
        RdfLiteral::typed("[_:b3, 1, _:b5]", list),
        RdfLiteral::typed("{\"k\": _:b7}", map),
        RdfLiteral::typed("[_:b10, [_:b11, _:b1]]", list),
    ];
    for (k, lit) in lits.into_iter().enumerate() {
        let o = b.intern_literal(lit).unwrap();
        b.push_quad(blanks[5 + k % 6], p, o, (k % 3 == 0).then_some(blanks[11]))
            .unwrap();
    }
    b.freeze().unwrap()
}

/// A statement layer in both spellings: native rows and the sentinel-lowered
/// quads that parse back to them.
fn statements() -> [Arc<RdfDataset>; 2] {
    [false, true].map(|lowered| {
        let mut b = RdfDatasetBuilder::new();
        let (p, q, o, g) = (
            b.intern_iri("http://example.org/p").unwrap(),
            b.intern_iri("http://example.org/q").unwrap(),
            b.intern_iri("http://example.org/o").unwrap(),
            b.intern_iri(G).unwrap(),
        );
        let n = b.intern_blank("n", BlankScope::DEFAULT).unwrap();
        let r = b.intern_blank("r", BlankScope(7)).unwrap();
        let named = b.intern_iri("http://example.org/named").unwrap();
        b.push_quad(n, p, o, None).unwrap();
        let triple = b.intern_triple(n, p, o).unwrap();
        // An interned `rdf:reifies` beside the profile's sentinel.
        let reifies = b.intern_iri(purrdf_iri::vocab::rdf::REIFIES).unwrap();
        b.push_quad(named, reifies, o, None).unwrap();
        if lowered {
            let sentinel_reifies = b.intern_iri(SENTINEL_REIFIES).unwrap();
            let sentinel_annotation = b.intern_iri(SENTINEL_ANNOTATION_GRAPH).unwrap();
            b.push_quad(r, sentinel_reifies, triple, None).unwrap();
            b.push_quad(named, sentinel_reifies, triple, Some(g))
                .unwrap();
            b.push_quad(r, q, o, Some(sentinel_annotation)).unwrap();
        } else {
            b.push_reifier_in_graph(r, triple, None).unwrap();
            b.push_reifier_in_graph(named, triple, Some(g)).unwrap();
            b.push_annotation_in_graph(r, q, o, None).unwrap();
        }
        b.freeze().unwrap()
    })
}

/// Nested triple terms at depth 2 and 3, escapes, composite literals holding
/// blank labels and more than ten blanks (so `c14n10` and `c14n9` cross), as one
/// flat dataset.
#[test]
fn flat_datasets_with_nested_terms_escapes_and_many_blanks_match_the_oracle() {
    check(&*mixed());
    for dataset in statements() {
        check(&*dataset);
    }
}

/// The two spellings of one statement layer canonicalize to one document.
#[test]
fn a_statement_layer_has_one_document_in_either_spelling() {
    let [native, lowered] = statements();
    assert_eq!(check(&*native), check(&*lowered));
}

/// A composite view renumbers every term into its own id space; the document
/// is still its lines, sorted.
#[test]
fn a_composite_view_matches_the_oracle() {
    let sources = vec![mixed(), statements()[0].clone(), statements()[1].clone()];
    let view = CompositeDatasetView::new(sources, ViewLimits::default()).unwrap();
    check(&view);
}

/// A delta view over a base: added rows with blanks and removed base rows.
#[test]
fn a_delta_view_matches_the_oracle() {
    let base = mixed();
    let mut delta = MutableDataset::new(Arc::clone(&base));
    for k in 0..5 {
        let row = QuadValues::triple(
            TermValue::blank(format!("added{k}")),
            TermValue::iri("https://example.org/p"),
            TermValue::lang_literal(format!("v{k} \"q\""), "en"),
        );
        assert!(delta.insert(row).unwrap());
    }
    let gone = QuadValues::triple(
        TermValue::iri("https://example.org/s"),
        TermValue::iri("https://example.org/p"),
        TermValue::iri("https://example.org/none"),
    );
    assert!(delta.insert(gone.clone()).unwrap());
    assert!(delta.remove(&gone));
    let view = delta.snapshot_view().unwrap();
    check(&view);
}

/// A line far longer than any buffer a short fixture would size, in a document
/// of short lines.
#[test]
fn long_lines_match_the_oracle() {
    let mut b = RdfDatasetBuilder::new();
    let (s, p) = (b.intern_iri(S).unwrap(), b.intern_iri(P).unwrap());
    let blank = b.intern_blank("x", BlankScope::DEFAULT).unwrap();
    for (k, len) in [3_usize, 70_000, 300_001, 70_000].into_iter().enumerate() {
        let o = b
            .intern_literal(RdfLiteral::simple(format!("{}{k}", "a\n".repeat(len / 2))))
            .unwrap();
        b.push_quad(if k % 2 == 0 { s } else { blank }, p, o, None)
            .unwrap();
    }
    check(&*b.freeze().unwrap());
}

// ---------------------------------------------------------------------------
// Generated fixtures: whatever mix of clean, unclean, irregular and blank terms.

#[test]
fn generated_documents_match_the_oracle_for_every_mix_of_terms() {
    const TAGS: [&str; 12] = [
        "en",
        "en-us",
        "en US",
        "",
        "en\n",
        ".\n",
        "\"x",
        "a\\",
        "en <http://example.org/g>",
        "x y",
        "en\nA",
        "en\nB",
    ];
    const LEXICALS: [&str; 6] = ["x", "", "a b", "q\"uote", "back\\slash", "nl\nline"];
    const IRIS: [&str; 8] = [
        "http://e/a",
        "http://e/a b",
        "http://e/a\u{a0}",
        "http://e/ab",
        "http://e/a>",
        "http://e/",
        "http://e/a\u{2028}",
        "http://e/a%20",
    ];
    for seed in 0..96_u64 {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32;
        let mut draw = |bound: usize| -> usize {
            usize::try_from(purrdf_hash::mix::splitmix64_next(&mut state) % bound as u64).unwrap()
        };
        let mut t = TableBuilder::new();
        let p = t.iri(P);
        let blanks: Vec<_> = (0..4).map(|k| t.blank(&format!("b{k}"))).collect();
        let mut terms = Vec::new();
        for _ in 0..10 {
            let kind = draw(3);
            let id = if kind == 0 {
                t.raw_iri(IRIS[draw(IRIS.len())])
            } else if kind == 1 {
                t.tagged(LEXICALS[draw(LEXICALS.len())], TAGS[draw(TAGS.len())])
            } else {
                t.iri(IRIS[draw(IRIS.len())])
            };
            terms.push(id);
        }
        for _ in 0..(6 + draw(14)) {
            let subject = if draw(2) == 0 {
                blanks[draw(blanks.len())]
            } else {
                t.iri(IRIS[draw(IRIS.len())])
            };
            let object = if draw(3) == 0 {
                blanks[draw(blanks.len())]
            } else {
                terms[draw(terms.len())]
            };
            let graph = match draw(3) {
                0 => None,
                1 => Some(blanks[draw(blanks.len())]),
                _ => Some(t.iri(G)),
            };
            t.quad(subject, p, object, graph);
        }
        check(&t.build());
    }
}

// ---------------------------------------------------------------------------
// Refusals, each with the neighbouring input that must still succeed.

/// A source that fails while the symbols are ranked, or at any other read, is
/// refused with its own failure; the same source without the failure succeeds.
#[test]
fn a_read_failure_at_any_resolution_is_refused_and_its_twin_succeeds() {
    let build = |fail_at: Option<usize>| {
        let mut t = TableBuilder::new();
        let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
        let b = t.blank("b");
        let tagged = t.tagged("x", "en US");
        let unclean = t.raw_iri("http://e/a b");
        t.quad(s, p, tagged, None);
        t.quad(b, p, unclean, Some(g));
        t.quad(s, p, b, None);
        let mut table = t.build();
        table.fail_at = fail_at;
        table
    };
    let stable = build(None);
    assert_eq!(document(&stable).unwrap(), oracle(&build(None)));
    let reads = stable.reads.get();
    assert!(reads > 6, "the run resolves terms more than once");
    for at in 1..=reads {
        assert_eq!(
            document(&build(Some(at))),
            Err(CanonError::Read(TableFault)),
            "a failure at resolution {at} must be the run's result"
        );
    }
    assert!(document(&build(Some(reads + 1))).is_ok());
}

/// A term that spells itself longer when read again is refused with
/// `SourceInconsistent` wherever the second spelling is used; a read that
/// changes nothing the document uses still succeeds with the stable bytes.
#[test]
fn a_term_that_changes_length_between_reads_is_refused_and_its_twin_succeeds() {
    let build = |nth: Option<usize>| {
        let mut t = TableBuilder::new();
        let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
        let blank = t.blank("b");
        let tagged = t.tagged("x", "en US");
        let other = t.tagged("y", "en");
        t.quad(s, p, tagged, None);
        t.quad(blank, p, tagged, Some(g));
        t.quad(s, p, other, Some(blank));
        let mut table = t.build();
        if let Some(nth) = nth {
            let (_, over) = table
                .over
                .iter()
                .find(|(id, _)| *id == tagged)
                .expect("the term is registered");
            table.flicker = Some((tagged, nth, over.longer()));
        }
        table
    };
    let expected = document(&build(None)).unwrap();
    assert_eq!(expected, oracle(&build(None)));
    let mut refused = 0;
    for nth in 1..=12 {
        match document(&build(Some(nth))) {
            Err(CanonError::SourceInconsistent) => refused += 1,
            Ok(bytes) => assert_eq!(
                bytes, expected,
                "a read after the last use of the term changes nothing"
            ),
            Err(other) => panic!("the only refusal is SourceInconsistent, got {other:?}"),
        }
    }
    assert!(refused >= 2, "a changed length is refused, not written");
}

// ---------------------------------------------------------------------------
// Each faster ordering agrees with comparator C on every pair of symbols.

/// Number the distinct symbols of `view` as a run does, then compare every pair
/// of them with `faster` and with comparator C, which reads both spellings again.
fn every_pair_agrees_with_c<D: DatasetView>(
    view: &D,
    faster: impl Fn(&order::Symbols<D::Id>, u32, u32, &str, &str) -> Option<Ordering>,
) -> usize {
    let run = SearchRun::new(SearchPolicy::Unlimited, None);
    let state = CanonState::new(
        view,
        CanonScope::Dataset,
        CanonPresentation::Overlay,
        CanonHash::Sha256,
        &run,
    )
    .issue_labels()
    .expect("labels issue");
    let mut resident = purrdf_lex::allocation::Resident;
    let mut memory = Memory::new(&mut resident);
    let (symbols, _) = state.discover_symbols(&mut memory).expect("symbols number");
    let (mut first, mut second) = (String::new(), String::new());
    let mut decided = 0;
    for a in 0..u32::try_from(symbols.count()).unwrap() {
        for b in 0..u32::try_from(symbols.count()).unwrap() {
            let by_text = state
                .compare_text(&symbols, &mut memory, a, b, &mut first, &mut second)
                .expect("both symbols spell");
            // `first` and `second` still hold the two spellings C compared.
            if let Some(ordering) = faster(&symbols, a, b, &first, &second) {
                assert_eq!(
                    ordering, by_text,
                    "symbols {a} and {b}: {first:?} {second:?}"
                );
                decided += 1;
            }
        }
    }
    decided
}

/// Symbols of different first-byte classes are ordered by their classes
/// without being read; the class of each symbol is the first byte of its
/// spelling, so the partition cannot disagree with the text order.
#[test]
fn the_first_byte_partition_agrees_with_the_text_comparison_on_every_pair() {
    let classes = *b"\").<_";
    let class_byte = |class: u8| classes[usize::from(class)];
    let by_class = |symbols: &order::Symbols<_>, a: u32, b: u32, x: &str, y: &str| {
        let (ca, cb) = (
            order::first_byte_class(a, symbols.meta_of(a)),
            order::first_byte_class(b, symbols.meta_of(b)),
        );
        assert_eq!(
            class_byte(ca),
            x.as_bytes()[0],
            "symbol {a} starts with its class"
        );
        assert_eq!(
            class_byte(cb),
            y.as_bytes()[0],
            "symbol {b} starts with its class"
        );
        (ca != cb).then(|| ca.cmp(&cb))
    };
    let mut cross_class = 0;
    cross_class += every_pair_agrees_with_c(&*mixed(), by_class);
    for dataset in statements() {
        cross_class += every_pair_agrees_with_c(&*dataset, by_class);
    }
    let mut t = TableBuilder::new();
    let (s, p, g) = (t.iri(S), t.iri(P), t.iri(G));
    let blank = t.blank("b");
    let odd = t.tagged("x", "en US");
    let unclean = t.raw_iri("http://e/a b");
    t.quad(s, p, odd, Some(blank));
    t.quad(blank, p, unclean, Some(g));
    cross_class += every_pair_agrees_with_c(&t.build(), by_class);
    assert!(
        cross_class > 100,
        "classes decide many pairs ({cross_class})"
    );
}

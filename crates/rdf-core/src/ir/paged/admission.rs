// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The page-admission law: whether a global `(s, p, o, g)` pattern can possibly
//! match a base quad on ONE page, decided from that page's sealed
//! [`PageSummary`](super::summary::PageSummary) alone — never by materializing the
//! page.
//!
//! # Why counts and not mere term presence
//!
//! The obvious-looking predicate — "does this page's term table contain the bound
//! term" — is [`PageTranslation::to_local`], and it is role-agnostic: it succeeds
//! for a term that appears ANYWHERE on the page (subject, predicate, object, or as
//! a graph name), regardless of the position the query binds it to. A page that
//! mentions `<g>` only as a *subject* of some other quad, and owns no row *in*
//! graph `<g>`, would pass a presence check on the graph axis and be materialized
//! for nothing. [`admit_pattern`] instead asks the exact question each axis
//! actually needs — "does this page hold at least one base row with this term in
//! THIS position" (or, for the graph axis, "at least one base row IN this graph")
//! — answered in `O(1)`/`O(log n)` from the page's [`PageSummary`] counts.
//!
//! # Soundness, not completeness
//!
//! The conjunction of per-axis checks below is a SOUND filter: it never skips a
//! page that could contribute a matching row. It is NOT complete: an admitted page
//! may still yield zero rows for the full `(s, p, o, g)` conjunction, because the
//! summary only proves each axis independently occurs on the page, not that the
//! SAME row satisfies all bound axes together. That is correct and intentional —
//! the summary is a page-sized sketch, not a join index — so callers must still run
//! the exact indexed scan on every admitted page; [`admit_pattern`] only spares
//! that scan (and the page materialization it would require) for pages it can PROVE
//! cannot match.

use crate::dataset_view::GraphMatch;
use crate::ir::{GlobalTermId, TermId};

use super::graph_index::GraphPageIndex;
use super::provider::PageId;
use super::summary::PageStream;
use super::translation::PageTranslation;

/// A global `(s, p, o, g)` pattern translated into ONE page's LOCAL [`TermId`]
/// space — the payload of an [`PageAdmission::Admit`] verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalPattern {
    pub(crate) s: Option<TermId>,
    pub(crate) p: Option<TermId>,
    pub(crate) o: Option<TermId>,
    pub(crate) g: GraphMatch<TermId>,
}

/// Which axis of the pattern proved a page cannot match — reported so a caller (or
/// a test) can name the exact reason a page was skipped, deterministically, per the
/// evaluation order in [`admit_pattern`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// The bound subject term is absent from this page, or occurs but never as a
    /// base-quad subject.
    Subject,
    /// The bound predicate term is absent from this page, or occurs but never as a
    /// base-quad predicate.
    Predicate,
    /// The bound object term is absent from this page, or occurs but never as a
    /// base-quad object.
    Object,
    /// The bound named graph is absent from this page, or this page owns no
    /// base-quad row in it.
    NamedGraph,
    /// The default graph was requested and this page owns no default-graph
    /// base-quad row.
    DefaultGraph,
}

/// The admission verdict for one page against one global pattern: either a proven
/// [`SkipReason`] this page cannot match on, or the pattern translated to this
/// page's local id space for the exact indexed scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageAdmission {
    /// This page cannot hold a matching base quad; the axis that proved it.
    Skip(SkipReason),
    /// This page may hold a matching base quad; the pattern in its local id space.
    Admit(LocalPattern),
}

/// Decide whether `translation`'s page can possibly hold a base quad matching the
/// global `(s, p, o, g)` pattern, from the page's sealed summary alone (never
/// materializing the page).
///
/// Evaluated in this fixed order — subject, then predicate, then object, then the
/// graph axis — so the reported [`SkipReason`] is deterministic: the first axis
/// that proves the page cannot match is the one named. See the [module docs](self)
/// for why each check is an exact row count rather than mere term presence, and for
/// why the admitted result is sound but not complete.
pub(crate) fn admit_pattern(
    translation: &PageTranslation,
    s: Option<GlobalTermId>,
    p: Option<GlobalTermId>,
    o: Option<GlobalTermId>,
    g: GraphMatch<GlobalTermId>,
) -> PageAdmission {
    let summary = translation.summary();

    let local_s = match s {
        None => None,
        Some(global) => match translation.to_local(global) {
            Some(local) if summary.base_rows_as_subject(local) > 0 => Some(local),
            _ => return PageAdmission::Skip(SkipReason::Subject),
        },
    };
    let local_p = match p {
        None => None,
        Some(global) => match translation.to_local(global) {
            Some(local) if summary.base_rows_as_predicate(local) > 0 => Some(local),
            _ => return PageAdmission::Skip(SkipReason::Predicate),
        },
    };
    let local_o = match o {
        None => None,
        Some(global) => match translation.to_local(global) {
            Some(local) if summary.base_rows_as_object(local) > 0 => Some(local),
            _ => return PageAdmission::Skip(SkipReason::Object),
        },
    };
    let local_g = match g {
        GraphMatch::Any => GraphMatch::Any,
        GraphMatch::Default => {
            if summary.default_rows(PageStream::Base) == 0 {
                return PageAdmission::Skip(SkipReason::DefaultGraph);
            }
            GraphMatch::Default
        }
        GraphMatch::Named(global_graph) => match translation.to_local(global_graph) {
            Some(local) if summary.graph_rows(local, PageStream::Base) > 0 => {
                GraphMatch::Named(local)
            }
            _ => return PageAdmission::Skip(SkipReason::NamedGraph),
        },
    };

    PageAdmission::Admit(LocalPattern {
        s: local_s,
        p: local_p,
        o: local_o,
        g: local_g,
    })
}

/// A zero-allocation cursor over the candidate `PageId`s for one graph constraint:
/// either every page (`GraphMatch::Any`) in ascending order, or a graph index
/// posting list (`Default`/`Named`), which is already ascending by construction
/// ([`GraphPageIndex::derive`]). Mirrors `QuadCandidates` in `ir/dataset.rs`.
pub(crate) enum PageCandidates<'a> {
    /// Every page, ascending — used when the graph axis is unconstrained.
    All(std::ops::Range<u32>),
    /// A graph index posting list, already ascending.
    Listed(std::slice::Iter<'a, PageId>),
}

impl Iterator for PageCandidates<'_> {
    type Item = PageId;

    #[inline]
    fn next(&mut self) -> Option<PageId> {
        match self {
            Self::All(range) => range.next().map(PageId),
            Self::Listed(iter) => iter.next().copied(),
        }
    }
}

/// Choose the candidate page set for graph constraint `g`, narrowing to the graph
/// index's posting list for `Default`/`Named` and falling back to every page
/// (`0..page_count`, ascending) for `Any`. Reads no page; both branches preserve
/// today's ascending-`PageId` egress order.
pub(crate) fn candidate_pages(
    graph_index: &GraphPageIndex,
    page_count: u32,
    g: GraphMatch<GlobalTermId>,
) -> PageCandidates<'_> {
    match g {
        GraphMatch::Named(global_graph) => PageCandidates::Listed(
            graph_index
                .pages_for_named(global_graph, PageStream::Base)
                .iter(),
        ),
        GraphMatch::Default => {
            PageCandidates::Listed(graph_index.pages_for_default(PageStream::Base).iter())
        }
        GraphMatch::Any => PageCandidates::All(0..page_count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{GlobalDictionary, RdfDatasetBuilder};

    #[test]
    fn subject_bound_skips_when_term_never_a_subject_and_admits_when_it_is() {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let b = builder.intern_iri("http://example.org/b");
        builder.push_quad(a, p, b, None);
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        let global_b = translation.to_global(b);
        let global_a = translation.to_global(a);

        // `b` is present on the page (as an object) but never a subject: bound as
        // the subject, the page must be provably unable to match.
        assert_eq!(
            admit_pattern(&translation, Some(global_b), None, None, GraphMatch::Any),
            PageAdmission::Skip(SkipReason::Subject)
        );
        // Neighbouring valid case: `a` genuinely occurs as a subject here.
        assert!(matches!(
            admit_pattern(&translation, Some(global_a), None, None, GraphMatch::Any),
            PageAdmission::Admit(_)
        ));
    }

    #[test]
    fn predicate_bound_skips_when_term_never_a_predicate_and_admits_when_it_is() {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let b = builder.intern_iri("http://example.org/b");
        builder.push_quad(a, p, b, None);
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        let global_a = translation.to_global(a);
        let global_p = translation.to_global(p);

        // `a` is present on the page (as a subject) but never a predicate.
        assert_eq!(
            admit_pattern(&translation, None, Some(global_a), None, GraphMatch::Any),
            PageAdmission::Skip(SkipReason::Predicate)
        );
        // Neighbouring valid case: `p` genuinely occurs as a predicate here.
        assert!(matches!(
            admit_pattern(&translation, None, Some(global_p), None, GraphMatch::Any),
            PageAdmission::Admit(_)
        ));
    }

    #[test]
    fn object_bound_skips_when_term_never_an_object_and_admits_when_it_is() {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let b = builder.intern_iri("http://example.org/b");
        builder.push_quad(a, p, b, None);
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        let global_p = translation.to_global(p);
        let global_b = translation.to_global(b);

        // `p` is present on the page (as the predicate) but never an object.
        assert_eq!(
            admit_pattern(&translation, None, None, Some(global_p), GraphMatch::Any),
            PageAdmission::Skip(SkipReason::Object)
        );
        // Neighbouring valid case: `b` genuinely occurs as an object here.
        assert!(matches!(
            admit_pattern(&translation, None, None, Some(global_b), GraphMatch::Any),
            PageAdmission::Admit(_)
        ));
    }

    #[test]
    fn named_graph_bound_skips_when_page_owns_no_rows_in_it_and_admits_when_it_does() {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri("http://example.org/a");
        let p = builder.intern_iri("http://example.org/p");
        let c = builder.intern_iri("http://example.org/c");
        let g1 = builder.intern_iri("http://example.org/g1");
        let g2 = builder.intern_iri("http://example.org/g2");
        builder.push_quad(a, p, c, Some(g1));
        // `g2` is declared (so it is present in the page's term table, and
        // `to_local` succeeds for it) but owns no rows at all.
        builder.declare_named_graph(g2);
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        let global_g1 = translation.to_global(g1);
        let global_g2 = translation.to_global(g2);

        // `g2` is a known term on this page (term-table presence alone would pass
        // `to_local`) but the page owns no row IN it: must skip.
        assert_eq!(
            admit_pattern(&translation, None, None, None, GraphMatch::Named(global_g2)),
            PageAdmission::Skip(SkipReason::NamedGraph)
        );
        // Neighbouring valid case: `g1` genuinely owns a base row on this page.
        assert!(matches!(
            admit_pattern(&translation, None, None, None, GraphMatch::Named(global_g1)),
            PageAdmission::Admit(_)
        ));
    }

    #[test]
    fn default_graph_bound_skips_when_page_has_no_default_rows_and_admits_when_it_does() {
        // Page with only a named-graph row: no default-graph rows at all.
        let mut builder = RdfDatasetBuilder::new();
        let x = builder.intern_iri("http://example.org/x");
        let p = builder.intern_iri("http://example.org/p");
        let y = builder.intern_iri("http://example.org/y");
        let g = builder.intern_iri("http://example.org/g");
        builder.push_quad(x, p, y, Some(g));
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        assert_eq!(
            admit_pattern(&translation, None, None, None, GraphMatch::Default),
            PageAdmission::Skip(SkipReason::DefaultGraph)
        );

        // Neighbouring valid case: a page with a genuine default-graph row admits.
        let mut builder2 = RdfDatasetBuilder::new();
        let a = builder2.intern_iri("http://example.org/a");
        let p2 = builder2.intern_iri("http://example.org/p");
        let b = builder2.intern_iri("http://example.org/b");
        builder2.push_quad(a, p2, b, None);
        let page2 = builder2.freeze().expect("page freezes");
        let mut dict2 = GlobalDictionary::new();
        let translation2 = PageTranslation::build(&page2, &mut dict2);
        assert!(matches!(
            admit_pattern(&translation2, None, None, None, GraphMatch::Default),
            PageAdmission::Admit(_)
        ));
    }

    #[test]
    fn graph_match_any_never_skips_on_the_graph_axis() {
        // A page with only a named-graph row and no default rows: on the graph
        // axis alone, `Any` must still admit — only the s/p/o axes may skip it.
        let mut builder = RdfDatasetBuilder::new();
        let x = builder.intern_iri("http://example.org/x");
        let p = builder.intern_iri("http://example.org/p");
        let y = builder.intern_iri("http://example.org/y");
        let g = builder.intern_iri("http://example.org/g");
        builder.push_quad(x, p, y, Some(g));
        let page = builder.freeze().expect("page freezes");
        let mut dict = GlobalDictionary::new();
        let translation = PageTranslation::build(&page, &mut dict);

        assert!(matches!(
            admit_pattern(&translation, None, None, None, GraphMatch::Any),
            PageAdmission::Admit(_)
        ));
    }
}

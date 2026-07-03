// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `RdfDataset`-direct structural comparator (C1/C2): the equality oracle
//! for importer equivalence and downstream tests.
//!
//! [`datasets_isomorphic`] decides whether two frozen datasets are
//! **RDF-structurally isomorphic**: the same quads (under a blank-node bijection),
//! the same reifier bindings, and the same annotations. It operates **directly on
//! [`RdfDataset`]** and **NEVER consults oxigraph**. That is deliberate and is the
//! acceptance gate of (design doc *Appendix C0*, point 4): oxigraph
//! canonicalizes typed-literal lexical forms (`0.70` → `0.7`, `+00:00` → `Z`) and
//! drops the reifier/annotation overlay entirely — so two datasets that differ only
//! in lexical spelling or in reifier COUNT would compare *equal* through oxigraph,
//! exactly the differences this comparator must catch.
//!
//! ## Identity contract
//!
//! - **Ground terms** (IRIs; literals incl. datatype / language / direction; and
//!   triple terms whose components are all ground) compare by their exact resolved
//!   value. The interned literal-identity policy (C0.1) already lives in the dataset,
//!   so e.g. `"x"` and `"x"^^xsd:string` resolve identically and compare equal, while
//!   two distinct directions or lexical spellings compare unequal.
//! - **Blank nodes** compare by **bijection**, never by `(label, scope)`: the two
//!   importer paths assign different [`BlankScope`](super::term::BlankScope) numbers
//!   and labels, yet a structurally identical graph must compare equal.
//! - **Reifiers and annotations are part of the structure**: two datasets that differ
//!   only in how many reifiers bind one triple term, or in an annotation triple,
//!   compare UNEQUAL.
//!
//! ## Implementation: full RDFC-1.0 (no false negatives)
//!
//! The verdict is computed by the native full W3C RDFC-1.0 canonicalizer
//! ([`super::canon::canonicalize`]): two datasets are isomorphic **iff** their
//! canonical N-Quads strings are byte-equal. RDFC-1.0 resolves blank-node
//! automorphisms via hash-partition + permutation backtracking, so — unlike the
//! simplified FNV signature refinement this comparator used to carry — the
//! oracle is **exact**: never a false positive *and* never a false negative, even on
//! pathologically symmetric blank graphs. The canonicalizer folds the RDF-1.2
//! reifier/annotation overlay and triple terms into the canonical form, so reifier
//! count and annotation presence remain part of the compared structure.

use super::canon;
use super::dataset::RdfDataset;

/// IR-direct structural comparison. Returns `true` iff the two datasets are
/// RDF-structurally isomorphic: the same quads (under a blank-node bijection), the
/// same reifier bindings, and the same annotations. **Oxigraph is NEVER consulted.**
///
/// Backed by full RDFC-1.0 canonicalization, this is an **exact** oracle: it never
/// reports a false positive *or* a false negative (the simplified comparator's
/// pathological-symmetry false negative is gone).
pub fn datasets_isomorphic(a: &RdfDataset, b: &RdfDataset) -> bool {
    // Cheap structural rejections that do not depend on blank labeling — they avoid
    // running the (poison-guarded) canonicalizer on obviously-different inputs.
    if a.quad_count() != b.quad_count() {
        return false;
    }
    if a.reifiers().count() != b.reifiers().count() {
        return false;
    }
    if a.annotations().count() != b.annotations().count() {
        return false;
    }
    if canon::blank_count(a) != canon::blank_count(b) {
        return false;
    }
    // The exact oracle: byte-equal canonical N-Quads ⇔ RDF isomorphism.
    canon::canonicalize(a).nquads == canon::canonicalize(b).nquads
}

/// A structural diff between two datasets, for test diagnostics. Counts only; the
/// blank-aware verdict is [`datasets_isomorphic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDiff {
    /// `(a, b)` quad counts.
    pub quad_counts: (usize, usize),
    /// `(a, b)` reifier-binding counts.
    pub reifier_counts: (usize, usize),
    /// `(a, b)` annotation counts.
    pub annotation_counts: (usize, usize),
    /// `(a, b)` blank-node counts.
    pub blank_counts: (usize, usize),
    /// The blank-aware structural verdict.
    pub isomorphic: bool,
}

/// A richer diff for test diagnostics: structural counts plus the isomorphism verdict.
pub fn dataset_diff(a: &RdfDataset, b: &RdfDataset) -> DatasetDiff {
    DatasetDiff {
        quad_counts: (a.quad_count(), b.quad_count()),
        reifier_counts: (a.reifiers().count(), b.reifiers().count()),
        annotation_counts: (a.annotations().count(), b.annotations().count()),
        blank_counts: (canon::blank_count(a), canon::blank_count(b)),
        isomorphic: datasets_isomorphic(a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::{RdfLiteral, RdfTextDirection};
    use std::sync::Arc;

    use super::super::term::TermId;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    fn ground_triple() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None);
        b.freeze().expect("valid")
    }

    #[test]
    fn identical_ground_datasets_are_isomorphic() {
        let a = ground_triple();
        let b = ground_triple();
        assert!(datasets_isomorphic(&a, &b));
    }

    #[test]
    fn differing_ground_iri_is_not_isomorphic() {
        let a = ground_triple();
        let mut bb = RdfDatasetBuilder::new();
        let (s, p, o) = (
            iri(&mut bb, "s"),
            iri(&mut bb, "p"),
            iri(&mut bb, "DIFFERENT"),
        );
        bb.push_quad(s, p, o, None);
        let b = bb.freeze().expect("valid");
        assert!(!datasets_isomorphic(&a, &b));
    }

    /// HEADLINE GATE: differing only in reifier COUNT for the same triple →
    /// NOT isomorphic. Oxigraph canonicalization would hide this.
    #[test]
    fn reifier_count_difference_is_not_isomorphic() {
        let build = |reifiers: &[&str]| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            b.push_quad(s, p, o, None);
            for r in reifiers {
                let rid = iri(&mut b, r);
                b.push_reifier(rid, triple);
            }
            b.freeze().expect("valid")
        };
        let one = build(&["r1"]);
        let two = build(&["r1", "r2"]);
        assert!(
            !datasets_isomorphic(&one, &two),
            "TWO reifiers vs ONE must compare unequal"
        );
        // And the same reifier set IS isomorphic.
        let one_again = build(&["r1"]);
        assert!(datasets_isomorphic(&one, &one_again));
    }

    #[test]
    fn annotation_difference_is_not_isomorphic() {
        let build = |with_annotation: bool| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            let r = iri(&mut b, "r");
            b.push_quad(s, p, o, None);
            b.push_reifier(r, triple);
            if with_annotation {
                let ap = iri(&mut b, "ap");
                let ao = iri(&mut b, "ao");
                b.push_annotation(r, ap, ao);
            }
            b.freeze().expect("valid")
        };
        assert!(!datasets_isomorphic(&build(true), &build(false)));
        assert!(datasets_isomorphic(&build(true), &build(true)));
    }

    /// Directional / datatype literal differences → NOT isomorphic (ground equality).
    #[test]
    fn directional_literal_difference_is_not_isomorphic() {
        let build = |dir: Option<RdfTextDirection>| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let lit = b.intern_literal(RdfLiteral {
                lexical_form: "x".to_owned(),
                datatype: None,
                language: Some("en".to_owned()),
                direction: dir,
            });
            b.push_quad(s, p, lit, None);
            b.freeze().expect("valid")
        };
        assert!(!datasets_isomorphic(
            &build(Some(RdfTextDirection::Ltr)),
            &build(Some(RdfTextDirection::Rtl))
        ));
        assert!(datasets_isomorphic(
            &build(Some(RdfTextDirection::Ltr)),
            &build(Some(RdfTextDirection::Ltr))
        ));
    }

    /// BLANK BIJECTION: same structure, different blank labels AND scopes → TRUE.
    #[test]
    fn blank_bijection_same_structure_different_labels_is_isomorphic() {
        use super::super::term::BlankScope;
        let build = |label: &str, scope: u32| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let blank = b.intern_blank(label, BlankScope(scope));
            b.push_quad(s, p, blank, None);
            b.freeze().expect("valid")
        };
        let a = build("b1", 0);
        let b = build("xyz", 7);
        assert!(
            datasets_isomorphic(&a, &b),
            "blanks differ only by label/scope; structure is identical"
        );
    }

    /// BLANK BIJECTION negative: genuinely different blank wiring → FALSE.
    /// `a`: one blank linked to ex:o1. `b`: one blank linked to ex:o2.
    #[test]
    fn blank_different_wiring_is_not_isomorphic() {
        use super::super::term::BlankScope;
        let build = |neighbour: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, link) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "link"));
            let blank = b.intern_blank("b", BlankScope::DEFAULT);
            let nb = iri(&mut b, neighbour);
            b.push_quad(s, p, blank, None);
            b.push_quad(blank, link, nb, None);
            b.freeze().expect("valid")
        };
        assert!(!datasets_isomorphic(&build("o1"), &build("o2")));
    }

    /// Two blanks with swapped-but-equivalent wiring stay isomorphic under bijection.
    #[test]
    fn two_blanks_relabeled_is_isomorphic() {
        use super::super::term::BlankScope;
        // a: _:x ex:p ex:A ; _:y ex:p ex:B
        // b: _:m ex:p ex:A ; _:n ex:p ex:B  (different labels/scopes)
        let build = |l1: &str, s1: u32, l2: &str, s2: u32| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "p");
            let a_node = iri(&mut b, "A");
            let b_node = iri(&mut b, "B");
            let x = b.intern_blank(l1, BlankScope(s1));
            let y = b.intern_blank(l2, BlankScope(s2));
            b.push_quad(x, p, a_node, None);
            b.push_quad(y, p, b_node, None);
            b.freeze().expect("valid")
        };
        let a = build("x", 0, "y", 0);
        let b = build("m", 3, "n", 9);
        assert!(datasets_isomorphic(&a, &b));
    }

    /// The classic symmetric blank ring (`_:x p _:y ; _:y p _:x`): a pure-hash
    /// refinement could not split this automorphism and the old comparator conceded a
    /// false negative; full RDFC-1.0 proves the two relabelings isomorphic.
    #[test]
    fn symmetric_ring_relabeled_is_isomorphic() {
        use super::super::term::BlankScope;
        let build = |l1: &str, l2: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "p");
            let x = b.intern_blank(l1, BlankScope(0));
            let y = b.intern_blank(l2, BlankScope(0));
            b.push_quad(x, p, y, None);
            b.push_quad(y, p, x, None);
            b.freeze().expect("valid")
        };
        assert!(datasets_isomorphic(&build("x", "y"), &build("m", "n")));
    }

    #[test]
    fn dataset_diff_reports_counts_and_verdict() {
        let a = ground_triple();
        let b = ground_triple();
        let diff = dataset_diff(&a, &b);
        assert_eq!(diff.quad_counts, (1, 1));
        assert!(diff.isomorphic);
    }
}

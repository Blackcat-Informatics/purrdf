// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The IR-direct structural comparator (C1/C2): the equality oracle
//! for importer equivalence and downstream tests.
//!
//! [`datasets_isomorphic`] decides whether two datasets are
//! **RDF-structurally isomorphic**: the same quads (under a blank-node bijection),
//! the same reifier bindings, and the same annotations. It operates **directly on
//! the IR** — on any [`DatasetView`], the frozen
//! [`RdfDataset`](super::dataset::RdfDataset) and the composite/delta views alike,
//! with the two sides independently typed so a view and a dataset compare without
//! either being materialized. That is deliberate
//! and is the
//! acceptance gate of (design doc *Appendix C0*, point 4): a comparator that
//! canonicalizes typed-literal lexical forms (`0.70` → `0.7`, `+00:00` → `Z`) or
//! drops the reifier/annotation overlay would let two datasets that differ only
//! in lexical spelling or in reifier COUNT compare *equal* — exactly the
//! differences this comparator must catch.
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
//! ([`super::canon::try_canonicalize`]): two datasets are isomorphic **iff** their
//! canonical N-Quads strings are byte-equal. RDFC-1.0 resolves blank-node
//! automorphisms via hash-partition + permutation backtracking, so — unlike the
//! simplified FNV signature refinement this comparator used to carry — the
//! oracle is **exact**: never a false positive *and* never a false negative, even on
//! pathologically symmetric blank graphs. The canonicalizer folds the RDF-1.2
//! reifier/annotation overlay and triple terms into the canonical form, so reifier
//! count and annotation presence remain part of the compared structure.

use super::canon;
use crate::dataset_view::DatasetView;

/// IR-direct structural comparison. Returns `true` iff the two views are
/// RDF-structurally isomorphic: the same quads (under a blank-node bijection), the
/// same reifier bindings, and the same annotations.
///
/// Backed by full RDFC-1.0 canonicalization, this is an **exact** oracle: it never
/// reports a false positive *or* a false negative (the simplified comparator's
/// pathological-symmetry false negative is gone).
///
/// The two sides are **independently** typed, so a composite view, a delta view and a
/// frozen [`RdfDataset`](super::dataset::RdfDataset) are all comparable against each
/// other, in any combination, without materializing either side: identity is decided
/// from the canonical bytes, which carry no view-local ids (see
/// [`canon::try_canonicalize_view`]). `&RdfDataset` call sites are the `RdfDataset`
/// instantiation.
///
/// # Errors
/// Returns the original canonicalization refusal instead of an equality verdict.
pub fn datasets_isomorphic<
    A: DatasetView<ReadError = core::convert::Infallible>,
    B: DatasetView<ReadError = core::convert::Infallible>,
>(
    a: &A,
    b: &B,
    control: &mut canon::CanonicalizationControl<'_>,
) -> Result<bool, canon::CanonError> {
    // Cheap structural rejections that do not depend on blank labeling — they avoid
    // running the (poison-guarded) canonicalizer on obviously-different inputs. Each
    // is an EXACT count off the view's own accessors, never `len_hint` (a hint a view
    // is free to approximate or omit would turn a pre-reject into a wrong verdict).
    let same_counts = prepare_comparison(a, b, control, |a, b, stop| {
        if count_rows(a.quads(), stop)? != count_rows(b.quads(), stop)? {
            return Ok(false);
        }
        if count_rows(a.reifier_quads(), stop)? != count_rows(b.reifier_quads(), stop)? {
            return Ok(false);
        }
        if count_rows(a.annotation_quads(), stop)? != count_rows(b.annotation_quads(), stop)? {
            return Ok(false);
        }
        Ok(canon::blank_count_with_stop(a, stop)? == canon::blank_count_with_stop(b, stop)?)
    })?;
    if !same_counts {
        return Ok(false);
    }
    canonical_equal(a, b, control)
}

fn prepare_comparison<
    A: DatasetView<ReadError = core::convert::Infallible>,
    B: DatasetView<ReadError = core::convert::Infallible>,
    T,
>(
    a: &A,
    b: &B,
    control: &mut canon::CanonicalizationControl<'_>,
    operation: impl FnOnce(&A, &B, Option<&dyn crate::StopSignal>) -> Result<T, canon::CanonError>,
) -> Result<T, canon::CanonError> {
    canon::comparison_preparation(control, |stop| {
        a.checked_read(|a| {
            b.checked_read(|b| {
                canon::check_stop(stop)?;
                operation(a, b, stop)
            })
            .map_err(canon::CanonError::Read)?
        })
        .map_err(canon::CanonError::Read)?
    })
}

fn count_rows<T>(
    mut rows: impl Iterator<Item = T>,
    stop: Option<&dyn crate::StopSignal>,
) -> Result<usize, canon::CanonError> {
    rows.try_fold(0usize, |count, _| {
        canon::check_stop(stop)?;
        count
            .checked_add(1)
            .ok_or(canon::CanonError::CounterCapacity)
    })
}

fn canonical_equal<
    A: DatasetView<ReadError = core::convert::Infallible>,
    B: DatasetView<ReadError = core::convert::Infallible>,
>(
    a: &A,
    b: &B,
    control: &mut canon::CanonicalizationControl<'_>,
) -> Result<bool, canon::CanonError> {
    // The exact oracle: byte-equal canonical N-Quads ⇔ RDF isomorphism.
    Ok(
        canon::try_canonicalize_view(a, canon::CanonHash::Sha256, control)?.nquads()
            == canon::try_canonicalize_view(b, canon::CanonHash::Sha256, control)?.nquads(),
    )
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
/// View-generic and cross-type on the same terms as [`datasets_isomorphic`].
///
/// # Errors
/// Returns the original canonicalization refusal without publishing a diff.
pub fn dataset_diff<
    A: DatasetView<ReadError = core::convert::Infallible>,
    B: DatasetView<ReadError = core::convert::Infallible>,
>(
    a: &A,
    b: &B,
    control: &mut canon::CanonicalizationControl<'_>,
) -> Result<DatasetDiff, canon::CanonError> {
    let mut diff = prepare_comparison(a, b, control, |a, b, stop| {
        Ok(DatasetDiff {
            quad_counts: (count_rows(a.quads(), stop)?, count_rows(b.quads(), stop)?),
            reifier_counts: (
                count_rows(a.reifier_quads(), stop)?,
                count_rows(b.reifier_quads(), stop)?,
            ),
            annotation_counts: (
                count_rows(a.annotation_quads(), stop)?,
                count_rows(b.annotation_quads(), stop)?,
            ),
            blank_counts: (
                canon::blank_count_with_stop(a, stop)?,
                canon::blank_count_with_stop(b, stop)?,
            ),
            isomorphic: false,
        })
    })?;
    if [
        diff.quad_counts,
        diff.reifier_counts,
        diff.annotation_counts,
        diff.blank_counts,
    ]
    .into_iter()
    .all(|(left, right)| left == right)
    {
        diff.isomorphic = canonical_equal(a, b, control)?;
    }
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{RdfDataset, RdfDatasetBuilder};
    use crate::{RdfLiteral, RdfTextDirection};
    use std::sync::Arc;

    use super::super::term::TermId;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}")).unwrap()
    }

    fn ground_triple() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None).unwrap();
        b.freeze().expect("valid")
    }

    /// Refusal is neither an equality verdict nor a published structural diff.
    #[test]
    fn reserved_vocabulary_comparison_returns_the_native_refusal() {
        let mut builder = RdfDatasetBuilder::new();
        let node = iri(&mut builder, "node");
        let reserved = builder.intern_iri(crate::RESERVED_NAMESPACE).unwrap();
        builder.push_quad(node, reserved, node, None).unwrap();
        let dataset = builder.freeze().unwrap();
        assert!(matches!(
            datasets_isomorphic(
                &dataset,
                &dataset,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            ),
            Err(canon::CanonError::ReservedVocabulary(_))
        ));
        assert!(matches!(
            dataset_diff(
                &dataset,
                &dataset,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            ),
            Err(canon::CanonError::ReservedVocabulary(_))
        ));
    }

    #[test]
    fn identical_ground_datasets_are_isomorphic() {
        let a = ground_triple();
        let b = ground_triple();
        assert!(
            datasets_isomorphic(
                &a,
                &b,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
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
        bb.push_quad(s, p, o, None).unwrap();
        let b = bb.freeze().expect("valid");
        assert!(
            !datasets_isomorphic(
                &a,
                &b,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
    }

    /// HEADLINE GATE: differing only in reifier COUNT for the same triple →
    /// NOT isomorphic. A lexical-canonicalizing comparison would hide this.
    #[test]
    fn reifier_count_difference_is_not_isomorphic() {
        let build = |reifiers: &[&str]| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o).unwrap();
            b.push_quad(s, p, o, None).unwrap();
            for r in reifiers {
                let rid = iri(&mut b, r);
                b.push_reifier(rid, triple).unwrap();
            }
            b.freeze().expect("valid")
        };
        let one = build(&["r1"]);
        let two = build(&["r1", "r2"]);
        assert!(
            !datasets_isomorphic(
                &one,
                &two,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap(),
            "TWO reifiers vs ONE must compare unequal"
        );
        // And the same reifier set IS isomorphic.
        let one_again = build(&["r1"]);
        assert!(
            datasets_isomorphic(
                &one,
                &one_again,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
    }

    #[test]
    fn annotation_difference_is_not_isomorphic() {
        let build = |with_annotation: bool| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o).unwrap();
            let r = iri(&mut b, "r");
            b.push_quad(s, p, o, None).unwrap();
            b.push_reifier(r, triple).unwrap();
            if with_annotation {
                let ap = iri(&mut b, "ap");
                let ao = iri(&mut b, "ao");
                b.push_annotation(r, ap, ao).unwrap();
            }
            b.freeze().expect("valid")
        };
        assert!(
            !datasets_isomorphic(
                &build(true),
                &build(false),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
        assert!(
            datasets_isomorphic(
                &build(true),
                &build(true),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
    }

    /// Directional / datatype literal differences → NOT isomorphic (ground equality).
    #[test]
    fn directional_literal_difference_is_not_isomorphic() {
        let build = |dir: Option<RdfTextDirection>| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let lit = b
                .intern_literal(RdfLiteral {
                    lexical_form: "x".to_owned(),
                    datatype: None,
                    language: Some("en".to_owned()),
                    direction: dir,
                })
                .unwrap();
            b.push_quad(s, p, lit, None).unwrap();
            b.freeze().expect("valid")
        };
        assert!(
            !datasets_isomorphic(
                &build(Some(RdfTextDirection::Ltr)),
                &build(Some(RdfTextDirection::Rtl)),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
        assert!(
            datasets_isomorphic(
                &build(Some(RdfTextDirection::Ltr)),
                &build(Some(RdfTextDirection::Ltr)),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
    }

    /// BLANK BIJECTION: same structure, different blank labels AND scopes → TRUE.
    #[test]
    fn blank_bijection_same_structure_different_labels_is_isomorphic() {
        use super::super::term::BlankScope;
        let build = |label: &str, scope: u32| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let blank = b.intern_blank(label, BlankScope(scope)).unwrap();
            b.push_quad(s, p, blank, None).unwrap();
            b.freeze().expect("valid")
        };
        let a = build("b1", 0);
        let b = build("xyz", 7);
        assert!(
            datasets_isomorphic(
                &a,
                &b,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap(),
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
            let blank = b.intern_blank("b", BlankScope::DEFAULT).unwrap();
            let nb = iri(&mut b, neighbour);
            b.push_quad(s, p, blank, None).unwrap();
            b.push_quad(blank, link, nb, None).unwrap();
            b.freeze().expect("valid")
        };
        assert!(
            !datasets_isomorphic(
                &build("o1"),
                &build("o2"),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
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
            let x = b.intern_blank(l1, BlankScope(s1)).unwrap();
            let y = b.intern_blank(l2, BlankScope(s2)).unwrap();
            b.push_quad(x, p, a_node, None).unwrap();
            b.push_quad(y, p, b_node, None).unwrap();
            b.freeze().expect("valid")
        };
        let a = build("x", 0, "y", 0);
        let b = build("m", 3, "n", 9);
        assert!(
            datasets_isomorphic(
                &a,
                &b,
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
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
            let x = b.intern_blank(l1, BlankScope(0)).unwrap();
            let y = b.intern_blank(l2, BlankScope(0)).unwrap();
            b.push_quad(x, p, y, None).unwrap();
            b.push_quad(y, p, x, None).unwrap();
            b.freeze().expect("valid")
        };
        assert!(
            datasets_isomorphic(
                &build("x", "y"),
                &build("m", "n"),
                &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None)
            )
            .unwrap()
        );
    }

    #[test]
    fn dataset_diff_reports_counts_and_verdict() {
        let a = ground_triple();
        let b = ground_triple();
        let diff = dataset_diff(
            &a,
            &b,
            &mut crate::CanonicalizationControl::new(crate::RDFC_V2_CORPUS_POLICY, None),
        )
        .unwrap();
        assert_eq!(diff.quad_counts, (1, 1));
        assert!(diff.isomorphic);
    }
}

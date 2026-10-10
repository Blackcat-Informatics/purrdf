// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Normalization with caller-owned annotations, over the same scalar kernels
//! as the ordinary streaming stages. Metadata follows decomposition and
//! reordering; the caller combines contributing metadata during composition.

use super::{canonical_order, ccc, decompose_scalar, unblocked_composite};

/// A Unicode scalar and its caller-selected provenance or other annotation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaggedScalar<M> {
    /// The scalar value.
    pub value: char,
    /// The annotation, duplicated on decomposition and merged on composition.
    pub metadata: M,
}

/// Replace `scalars` by its NFD/NFKD form, preserving each contributor through
/// decomposition and stable canonical ordering. `scratch` is reusable storage.
/// Exact native decomposition count, including pinned compatibility expansions.
pub fn decomposed_len<const COMPAT: bool, M>(scalars: &[TaggedScalar<M>]) -> Option<usize> {
    let mut count = Some(0usize);
    for scalar in scalars {
        decompose_scalar::<COMPAT>(scalar.value, |_| {
            count = count.and_then(|n| n.checked_add(1));
        });
    }
    count
}

/// Decompose and canonically order annotated scalars using reusable storage.
pub fn decompose_tagged<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>,
    scratch: &mut Vec<TaggedScalar<M>>,
) {
    decompose_tagged_with::<COMPAT, M>(scalars, scratch, |run| {
        canonical_order(run, |scalar| ccc(scalar.value));
    });
}

/// The same decomposition law with already-admitted destination and sort work.
pub fn decompose_tagged_preallocated<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>,
    scratch: &mut Vec<TaggedScalar<M>>,
    order: &mut Vec<usize>,
) {
    assert!(
        scratch.capacity()
            >= decomposed_len::<COMPAT, _>(scalars).expect("admitted decomposition count")
    );
    decompose_tagged_with::<COMPAT, M>(scalars, scratch, |run| {
        super::canonical_order_by_index(run, order, |scalar| ccc(scalar.value));
    });
}

fn decompose_tagged_with<const COMPAT: bool, M: Clone>(
    scalars: &mut Vec<TaggedScalar<M>>,
    scratch: &mut Vec<TaggedScalar<M>>,
    mut order: impl FnMut(&mut [TaggedScalar<M>]),
) {
    scratch.clear();
    for scalar in scalars.iter() {
        decompose_scalar::<COMPAT>(scalar.value, |value| {
            scratch.push(TaggedScalar {
                value,
                metadata: scalar.metadata.clone(),
            });
        });
    }
    let mut start = 0;
    for at in 0..scratch.len() {
        if ccc(scratch[at].value) == 0 {
            order(&mut scratch[start..at]);
            start = at + 1;
        }
    }
    order(&mut scratch[start..]);
    std::mem::swap(scalars, scratch);
}

/// Compose already decomposed, canonically ordered scalars, combining every
/// contributing annotation with `merge`. This is NFC/NFKC's composition step.
pub fn compose_tagged<M>(
    scalars: &mut Vec<TaggedScalar<M>>,
    scratch: &mut Vec<TaggedScalar<M>>,
    mut merge: impl FnMut(&mut M, M),
) {
    scratch.clear();
    let mut starter: Option<usize> = None;
    let mut last_class = 0;
    for scalar in scalars.drain(..) {
        let class = ccc(scalar.value);
        if let Some(at) = starter
            && let Some(composite) = unblocked_composite(
                scratch[at].value,
                scalar.value,
                class,
                last_class,
                at + 1 < scratch.len(),
            )
        {
            scratch[at].value = composite;
            merge(&mut scratch[at].metadata, scalar.metadata);
            continue;
        }
        if class == 0 {
            starter = Some(scratch.len());
        }
        last_class = class;
        scratch.push(scalar);
    }
    std::mem::swap(scalars, scratch);
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::hash::Hash;

use crate::v2::fields::push;
use crate::v2::{BlobEntry, FoldError, RdfFold, ValueName};
use crate::{FastMap, FastSet};

use super::{
    Binding, Digest, FrameLocation, FrameOwners, Referent, ResolvedLeaf, ScopedLeaf, Suppression,
    SuppressionClaim, VisibleFold,
};

/// Location of a binding or suppression frame in the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceLocation {
    /// File-relative segment ordinal.
    pub segment: u64,
    /// Byte offset of the frame.
    pub offset: usize,
}

/// A run of segments and the commit that closes one graph state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct StateLocation {
    /// File-relative ordinal of the run's first segment.
    pub first_segment: u64,
    /// Byte offset of the run's first opener.
    pub start: usize,
    /// Exclusive byte offset of the naming commit.
    pub end: usize,
    /// Position of the commit that names the state.
    pub commit: FrameLocation,
    pub(in crate::v2) healed_commit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::v2) enum StateResolution {
    Named(StateLocation),
    Missing,
    Unavailable,
}

/// Commit identifiers in file order, retaining the first commit that names a state.
#[derive(Debug, Default)]
pub(in crate::v2) struct StateCatalog(FastMap<Digest, Option<StateLocation>>);

impl StateCatalog {
    pub(in crate::v2) fn record_commit(
        &mut self,
        identifier: Digest,
        state: Option<StateLocation>,
    ) -> Result<(), FoldError> {
        if let Some(earlier) = self.0.get_mut(&identifier) {
            if earlier.is_none() {
                *earlier = state;
            }
        } else {
            self.0.try_reserve(1)?;
            self.0.insert(identifier, state);
        }
        Ok(())
    }

    pub(in crate::v2) fn lookup(&self, identifier: &Digest) -> StateResolution {
        match self.0.get(identifier) {
            Some(Some(location)) => StateResolution::Named(*location),
            Some(None) => StateResolution::Unavailable,
            None => StateResolution::Missing,
        }
    }

    pub(in crate::v2) fn named(&self, identifier: &Digest) -> Option<StateLocation> {
        self.0.get(identifier).copied().flatten()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// Reason why a binding could not resolve its referent within the file.
pub enum MissingReferent {
    /// The file has no declaration of the blob.
    Blob,
    /// The file has no ordinary frame with the effective identifier.
    Frame,
    /// The file has no commit with the effective identifier.
    State,
    /// A commit has the identifier but the reader could not establish its state.
    StateUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
/// A file-wide lookup of a binding's referent.
pub enum ResolvedReferent {
    /// Blob declaration, including the declared length selected by the fold.
    Blob(ValueName),
    /// Position of the first ordinary frame with this identifier.
    Frame(FrameLocation),
    /// Span and commit of the first readable state with this identifier.
    State(StateLocation),
    /// No matching object or state could be established in the file.
    Missing(MissingReferent),
}

#[derive(Default)]
pub(in crate::v2) struct ReferenceClaims {
    bindings: Vec<(ReferenceLocation, Binding<ScopedLeaf>)>,
    suppressions: Vec<(ReferenceLocation, Suppression<ScopedLeaf>)>,
}

/// File-wide reference records and their visibility after resolving all leaves.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ReferenceFold {
    /// Distinct admitted bindings in first occurrence order.
    pub bindings: Vec<Binding<ResolvedLeaf>>,
    /// Distinct admitted suppression entries in first occurrence order.
    pub suppressions: Vec<SuppressionClaim>,
    /// Resolution of each distinct referent named by admitted bindings.
    pub referents: FastMap<Referent, ResolvedReferent>,
    /// Unresolved referents, ordered by their first binding frame.
    pub missing: Vec<(ReferenceLocation, Referent, MissingReferent)>,
    /// Frames with a binding term that names no node in the file.
    pub unanchored: Vec<ReferenceLocation>,
    /// Visibility of the admitted content under this suppression record.
    pub visible: VisibleFold,
}

impl ReferenceClaims {
    pub(in crate::v2) fn binding(
        &mut self,
        at: ReferenceLocation,
        binding: Binding<ScopedLeaf>,
    ) -> Result<(), FoldError> {
        push(&mut self.bindings, (at, binding)).map_err(FoldError::from)
    }

    pub(in crate::v2) fn suppression(
        &mut self,
        at: ReferenceLocation,
        entry: Suppression<ScopedLeaf>,
    ) -> Result<(), FoldError> {
        push(&mut self.suppressions, (at, entry)).map_err(FoldError::from)
    }

    pub(in crate::v2) fn finish(
        self,
        fold: &mut RdfFold,
        frames: &FrameOwners,
        blobs: &[BlobEntry<'_>],
        states: &StateCatalog,
    ) -> Result<ReferenceFold, FoldError> {
        let mut result = ReferenceFold::default();
        if self.bindings.is_empty() && self.suppressions.is_empty() {
            return Ok(result);
        }

        let mut bindings = Vec::new();
        bindings.try_reserve_exact(self.bindings.len())?;
        for (at, binding) in self.bindings {
            let binding = binding.try_map(|leaf| leaf.resolve(at.segment, frames, fold))?;
            if matches!(binding.term, ResolvedLeaf::Ground(_)) {
                push(&mut result.unanchored, at)?;
            }
            bindings.push((at, binding));
        }
        result.unanchored.sort_unstable_by_key(|at| at.offset);
        result.unanchored.dedup();
        keep_first(&mut bindings, |(_, binding)| binding)?;
        result.resolve_referents(bindings, frames, blobs, states)?;

        result
            .suppressions
            .try_reserve_exact(self.suppressions.len())?;
        for (at, entry) in self.suppressions {
            result.suppressions.push(SuppressionClaim {
                offset: at.offset,
                entry: entry.try_map(|leaf| leaf.resolve(at.segment, frames, fold))?,
            });
        }
        result.visible = VisibleFold::apply(
            fold,
            &result.bindings,
            blobs.iter().map(|blob| blob.name.digest),
            &result.suppressions,
        )?;
        keep_first(&mut result.suppressions, |claim| &claim.entry)?;
        Ok(result)
    }
}

impl ReferenceFold {
    fn resolve_referents(
        &mut self,
        bindings: Vec<(ReferenceLocation, Binding<ResolvedLeaf>)>,
        frames: &FrameOwners,
        blobs: &[BlobEntry<'_>],
        states: &StateCatalog,
    ) -> Result<(), FoldError> {
        let mut blob_names = FastMap::default();
        if bindings
            .iter()
            .any(|(_, binding)| matches!(binding.referent, Referent::Blob(_)))
        {
            blob_names.try_reserve(blobs.len())?;
            for blob in blobs {
                blob_names.insert(blob.name.digest, blob.name);
            }
        }
        self.bindings.try_reserve_exact(bindings.len())?;
        self.referents.try_reserve(bindings.len())?;
        for (at, binding) in bindings {
            let referent = binding.referent;
            if let std::collections::hash_map::Entry::Vacant(slot) = self.referents.entry(referent)
            {
                let resolution = match &referent {
                    Referent::Blob(digest) => blob_names.get(digest).copied().map_or(
                        ResolvedReferent::Missing(MissingReferent::Blob),
                        ResolvedReferent::Blob,
                    ),
                    Referent::Frame(digest) => frames.first(digest).map_or(
                        ResolvedReferent::Missing(MissingReferent::Frame),
                        ResolvedReferent::Frame,
                    ),
                    Referent::State(digest) => match states.lookup(digest) {
                        StateResolution::Named(location) => ResolvedReferent::State(location),
                        StateResolution::Missing => {
                            ResolvedReferent::Missing(MissingReferent::State)
                        }
                        StateResolution::Unavailable => {
                            ResolvedReferent::Missing(MissingReferent::StateUnavailable)
                        }
                    },
                };
                if let ResolvedReferent::Missing(reason) = resolution {
                    push(&mut self.missing, (at, referent, reason))?;
                }
                slot.insert(resolution);
            }
            self.bindings.push(binding);
        }
        Ok(())
    }
}

fn keep_first<T, K: Eq + Hash + ?Sized>(
    entries: &mut Vec<T>,
    key: impl Fn(&T) -> &K,
) -> Result<(), FoldError> {
    let mut keep = Vec::new();
    keep.try_reserve_exact(entries.len())?;
    {
        let mut seen = FastSet::default();
        seen.try_reserve(entries.len())?;
        keep.extend(entries.iter().map(|entry| seen.insert(key(entry))));
    }
    let mut keep = keep.into_iter();
    entries.retain(|_| keep.next().expect("one retention decision per entry"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::v2::Term;

    use super::super::ScopedNode;
    use super::{
        Binding, BlobEntry, FrameLocation, FrameOwners, MissingReferent, RdfFold, ReferenceClaims,
        ReferenceLocation, Referent, ScopedLeaf, StateCatalog, StateLocation, Suppression,
        ValueName,
    };

    #[test]
    fn a_referent_uses_its_first_binding_and_suppression_keeps_the_binding_in_the_fold() {
        let mut fold = RdfFold::default();
        let a = fold
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/a")))
            .unwrap();
        let b = fold
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/b")))
            .unwrap();
        let location = |offset| ReferenceLocation { segment: 0, offset };
        let leaf = |id| ScopedLeaf(vec![ScopedNode::Term(id)]);
        let missing = Referent::Frame([5; 32]);
        let unavailable = Referent::State([7; 32]);
        let available = Referent::State([8; 32]);
        let name = ValueName {
            digest: [9; 32],
            length: 2,
        };
        let external = Referent::Blob(name.digest);
        let mut claims = ReferenceClaims::default();
        for (at, term, referent) in [
            (10, a, missing),
            (30, b, missing),
            (40, a, missing),
            (50, a, external),
            (60, b, unavailable),
            (70, b, available),
        ] {
            claims
                .binding(
                    location(at),
                    Binding {
                        term: leaf(term),
                        referent,
                    },
                )
                .unwrap();
        }
        claims
            .suppression(
                location(80),
                Suppression::Binding(Binding {
                    term: leaf(a),
                    referent: missing,
                }),
            )
            .unwrap();
        let blobs = [BlobEntry {
            name,
            present: false,
            media_types: Vec::new(),
        }];
        let state = StateLocation {
            first_segment: 0,
            start: 0,
            end: 200,
            commit: FrameLocation {
                segment: 0,
                position: 9,
                offset: 190,
            },
            healed_commit: false,
        };
        let mut states = StateCatalog::default();
        states.record_commit([7; 32], None).unwrap();
        states.record_commit([8; 32], None).unwrap();
        states.record_commit([8; 32], Some(state)).unwrap();
        let later = StateLocation {
            first_segment: 1,
            start: 210,
            end: 400,
            commit: FrameLocation {
                segment: 1,
                offset: 390,
                ..state.commit
            },
            healed_commit: false,
        };
        states.record_commit([8; 32], Some(later)).unwrap();
        let result = claims
            .finish(&mut fold, &FrameOwners::default(), &blobs, &states)
            .unwrap();
        assert_eq!(result.bindings.len(), 5);
        assert_eq!(
            result.referents[&available],
            super::ResolvedReferent::State(state)
        );
        assert_eq!(result.suppressions.len(), 1);
        assert_eq!(
            result.missing,
            [
                (location(10), missing, MissingReferent::Frame),
                (location(60), unavailable, MissingReferent::StateUnavailable),
            ]
        );
        assert_eq!(
            result.referents[&external],
            super::ResolvedReferent::Blob(name)
        );
        assert!(result.visible.hides_binding(0));
        assert!(!result.visible.hides_binding(1));
    }
}

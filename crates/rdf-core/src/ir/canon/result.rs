// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable canonical output retained under its source's original admission.

use purrdf_lex::allocation::{SharedOwned, StorageError};

use super::ownership::SourceGrant;
use super::{CanonError, CanonPresentation};
use crate::{DatasetView, TermId, ViewTermId};

#[derive(Debug, PartialEq, Eq)]
struct Fields<Id> {
    nquads: String,
    labels: Vec<(Id, String)>,
    presentation: Option<CanonPresentation>,
}

/// The result of canonicalizing a dataset or a view of one.
///
/// Generic over the id type `Id` — the [`Id`](DatasetView::Id) of the view
/// that produced it — and **defaulted to [`TermId`]**, so the bare spelling
/// `Canonicalized` continues to name the `&RdfDataset` result everywhere.
/// Cloning shares the immutable payload and its original admission. Text and
/// labels are borrowed from this owner and cannot be detached by moving fields.
#[derive(Clone, Debug)]
pub struct Canonicalized<Id = TermId> {
    payload: SharedOwned<Fields<Id>>,
}

impl<Id: ViewTermId> Canonicalized<Id> {
    /// The canonical N-Quads document: every line `'\n'`-terminated.
    ///
    /// For the default family ([`Self::presentation`] is `None`) it is the
    /// canonical line sequence of the nested-refinement labelling spelled as
    /// N-Quads 1.2, one text line per statement line in the order of the
    /// sequence (not the bytewise order of the text), each blank node `_:` and
    /// its position in decimal. For the RDFC-1.0 family (`rdfc12`) it is the
    /// set of lines sorted bytewise ascending and deduplicated, each blank node
    /// its `_:c14nN` label.
    ///
    /// What an RDFC-1.0 reifier or annotation row renders AS depends on
    /// [`Self::presentation`], reported alongside this document rather than left
    /// implicit: under [`CanonPresentation::Overlay`] such a row renders through
    /// the profile's reserved `urn:purrdf:rdfc:` sentinels; under
    /// [`CanonPresentation::FlatAssertion`] no sentinel is ever minted — the same
    /// row renders as an ordinary quad carrying its own real predicate. A document
    /// produced by one of this module's four flat entry points therefore NEVER
    /// contains a sentinel, and these bytes alone cannot tell a reader which
    /// presentation produced them — read [`Self::presentation`] for that.
    ///
    /// This document carries NO view-local ids, which is what makes it comparable
    /// across view kinds: the canonical bytes of a composite view and of the flat
    /// dataset holding the same content are equal.
    #[must_use]
    pub fn nquads(&self) -> &str {
        &self.payload.nquads
    }

    /// Which RDFC-1.0 [`CanonPresentation`] produced [`Self::nquads`], or `None`
    /// when the document is the canonical line sequence of the
    /// nested-refinement labelling, which has one presentation
    /// ([`CANON_PRESENTATION_LINES_ID`](super::CANON_PRESENTATION_LINES_ID)).
    /// Only the functions of [`crate::rdfc12`] report a presentation. Where
    /// present, it is the fourth of the legacy consumer pin's four coordinates
    /// `(profile, version, presentation, hash)`. [`try_canonicalize`](super::try_canonicalize), [`try_canonicalize_view`](super::try_canonicalize_view),
    /// [`try_canonicalize_graph_view`](super::try_canonicalize_graph_view) and their kin always report
    /// [`CanonPresentation::Overlay`]; [`try_canonicalize_flat_view`](super::try_canonicalize_flat_view),
    /// [`try_canonicalize_flat_graph_view`](super::try_canonicalize_flat_graph_view) and the empty-selection cases of both
    /// families always report the presentation of the family that produced them.
    #[must_use]
    pub fn presentation(&self) -> Option<CanonPresentation> {
        self.payload.presentation
    }

    /// Each blank id mapped to its canonical label WITHOUT the leading `_:`:
    /// its position in decimal (`"0"`, …) for the default family, `"c14n0"`, …
    /// for the RDFC-1.0 family. The ids are those of the view that was canonicalized and
    /// are meaningful only within it.
    ///
    /// This assignment is the PRINCIPLED blank-label assignment: the labels are issued
    /// purely from graph structure, so they are isomorphism-invariant (two
    /// isomorphic datasets assign corresponding blanks the same label), and their
    /// alphabet is ASCII alphanumerics only — legal under every constrained
    /// egress alphabet (`BLANK_NODE_LABEL`, XML `NCName`; see
    /// [`crate::blank_label`]). [`canonical_relabel`](super::canonical_relabel) applies it as a dataset
    /// rewrite.
    pub fn labels(&self) -> impl ExactSizeIterator<Item = (Id, &str)> {
        self.payload
            .labels
            .iter()
            .map(|(id, label)| (*id, label.as_str()))
    }

    /// The canonical label for one view-local blank id, without `_:`.
    #[must_use]
    pub fn label(&self, id: &Id) -> Option<&str> {
        self.payload
            .labels
            .binary_search_by_key(id, |(key, _)| *key)
            .ok()
            .map(|index| self.payload.labels[index].1.as_str())
    }

    /// Number of issued blank labels.
    #[must_use]
    pub fn label_count(&self) -> usize {
        self.payload.labels.len()
    }
}

impl<Id: ViewTermId> AsRef<str> for Canonicalized<Id> {
    fn as_ref(&self) -> &str {
        self.nquads()
    }
}

impl<Id: PartialEq> PartialEq for Canonicalized<Id> {
    fn eq(&self, other: &Self) -> bool {
        *self.payload == *other.payload
    }
}
impl<Id: Eq> Eq for Canonicalized<Id> {}

// Fields precede the grant on failed construction as well as in SharedOwned.
pub(super) struct ResultBuilder<Id, E> {
    fields: Fields<Id>,
    grant: SourceGrant<E>,
}

impl<Id: ViewTermId, E: Send + Sync + 'static> ResultBuilder<Id, E> {
    pub(super) fn new<D: DatasetView<Id = Id, ReadError = E>>(
        view: &D,
        presentation: CanonPresentation,
    ) -> Result<Self, CanonError<E>> {
        Self::with_presentation(view, Some(presentation))
    }

    /// A builder for the document of the nested-refinement labelling, which
    /// has no RDFC-1.0 presentation.
    pub(super) fn lines<D: DatasetView<Id = Id, ReadError = E>>(
        view: &D,
    ) -> Result<Self, CanonError<E>> {
        Self::with_presentation(view, None)
    }

    fn with_presentation<D: DatasetView<Id = Id, ReadError = E>>(
        view: &D,
        presentation: Option<CanonPresentation>,
    ) -> Result<Self, CanonError<E>> {
        Ok(Self {
            fields: Fields {
                nquads: String::new(),
                labels: Vec::new(),
                presentation,
            },
            grant: SourceGrant::new(view)?,
        })
    }

    pub(super) fn reserve_text(&mut self, bytes: usize) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.reserve_string(&mut self.fields.nquads, bytes))
    }

    pub(super) fn append(&mut self, line: &str) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.push_str(&mut self.fields.nquads, line))
    }

    pub(super) fn reserve_labels(&mut self, count: usize) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.reserve(&mut self.fields.labels, count))
    }

    /// Take over an issued label without copying it. The issuer allocated the box
    /// outside every account, so the owned account is charged exactly the bytes
    /// an admitted copy of the label would have charged.
    pub(super) fn label(&mut self, id: Id, label: Box<str>) -> Result<(), CanonError<E>> {
        let label = label.into_string();
        self.grant.memory(|memory| {
            memory.add_bytes(label.len())?;
            memory.push(&mut self.fields.labels, (id, label))
        })
    }

    pub(super) fn finish(
        mut self,
        stop: Option<&dyn crate::StopSignal>,
    ) -> Result<Canonicalized<Id>, CanonError<E>> {
        let bytes = SharedOwned::<Fields<Id>>::allocation_layout::<SourceGrant<E>>().size();
        self.grant.memory(|memory| memory.add_bytes(bytes))?;
        super::check_stop(stop).map_err(CanonError::with_read_error)?;
        SharedOwned::try_from_admitted(self.fields, self.grant)
            .map(|payload| Canonicalized { payload })
            .map_err(|_| CanonError::Storage(StorageError::AllocationFailed))
    }
}

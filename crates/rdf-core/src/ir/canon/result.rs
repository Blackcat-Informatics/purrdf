// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable canonical output retained under its source's original admission.

use purrdf_lex::allocation::{SharedOwned, StorageError};
use std::ops::Range;

use super::ownership::SourceGrant;
use super::{CanonError, CanonPresentation};
use crate::{DatasetView, TermId, ViewTermId};

#[derive(Debug, PartialEq, Eq)]
struct Fields<Id> {
    nquads: String,
    labels: Box<[(Id, Range<usize>)]>,
    label_text: Box<str>,
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
    presentation: Option<CanonPresentation>,
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
    /// ([`CANON_PRESENTATION_LINES_ID`](crate::ir::CANON_PRESENTATION_LINES_ID)).
    /// Only the functions of [`crate::rdfc12`] report a presentation. Where
    /// present, it is the fourth of the legacy consumer pin's four coordinates
    /// `(profile, version, presentation, hash)`. [`try_canonicalize`](super::try_canonicalize), [`try_canonicalize_view`](super::try_canonicalize_view),
    /// [`try_canonicalize_graph_view`](super::try_canonicalize_graph_view) and their kin always report
    /// [`CanonPresentation::Overlay`]; [`try_canonicalize_flat_view`](super::try_canonicalize_flat_view),
    /// [`try_canonicalize_flat_graph_view`](super::try_canonicalize_flat_graph_view) and the empty-selection cases of both
    /// families always report the presentation of the family that produced them.
    #[must_use]
    pub fn presentation(&self) -> Option<CanonPresentation> {
        self.presentation
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
            .map(|(id, label)| (*id, &self.payload.label_text[label.clone()]))
    }

    /// The canonical label for one view-local blank id, without `_:`.
    #[must_use]
    pub fn label(&self, id: &Id) -> Option<&str> {
        self.payload
            .labels
            .binary_search_by_key(id, |(key, _)| *key)
            .ok()
            .map(|index| &self.payload.label_text[self.payload.labels[index].1.clone()])
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
        self.presentation == other.presentation && *self.payload == *other.payload
    }
}
impl<Id: Eq> Eq for Canonicalized<Id> {}

// Fields precede the grant on failed construction as well as in SharedOwned.
pub(super) struct ResultBuilder<Id, E> {
    nquads: String,
    labels: Vec<(Id, Range<usize>)>,
    label_text: String,
    presentation: Option<CanonPresentation>,
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
            nquads: String::new(),
            labels: Vec::new(),
            label_text: String::new(),
            presentation,
            grant: SourceGrant::new(view)?,
        })
    }

    pub(super) fn reserve_text(&mut self, bytes: usize) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.reserve_string(&mut self.nquads, bytes))
    }

    pub(super) fn append(&mut self, line: &str) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.push_str(&mut self.nquads, line))
    }

    pub(super) fn reserve_labels(&mut self, count: usize) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.reserve(&mut self.labels, count))
    }

    pub(super) fn reserve_label_text(&mut self, bytes: usize) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.reserve_string(&mut self.label_text, bytes))
    }

    pub(super) fn decimal_label(&mut self, id: Id, position: u64) -> Result<(), CanonError<E>> {
        self.grant.memory(|memory| {
            let start = self.label_text.len();
            memory.append_text(&mut self.label_text, |out| {
                // Decimal formatting itself cannot fail; the admitted sink retains failures.
                let _ = write!(out, "{position}");
            })?;
            memory.push(&mut self.labels, (id, start..self.label_text.len()))
        })
    }

    /// Index an already issued label, then adopt its entire backing arena once.
    pub(super) fn issued_label(
        &mut self,
        id: Id,
        label: Range<usize>,
    ) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.push(&mut self.labels, (id, label)))
    }

    pub(super) fn adopt_label_text(&mut self, text: String) -> Result<(), CanonError<E>> {
        self.grant
            .memory(|memory| memory.add_bytes(text.capacity()))?;
        self.label_text = text;
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        stop: Option<&dyn crate::StopSignal>,
    ) -> Result<Canonicalized<Id>, CanonError<E>> {
        let labels = self
            .grant
            .memory(|memory| memory.boxed_slice(self.labels))?;
        // Both paths reserve the exact label byte count, so freezing transfers
        // the string's original allocation rather than shrinking it.
        debug_assert_eq!(self.label_text.len(), self.label_text.capacity());
        let fields = Fields {
            nquads: self.nquads,
            labels,
            label_text: self.label_text.into_boxed_str(),
        };
        let bytes = SharedOwned::<Fields<Id>>::allocation_layout::<SourceGrant<E>>().size();
        self.grant.memory(|memory| memory.add_bytes(bytes))?;
        super::check_stop(stop).map_err(CanonError::with_read_error)?;
        SharedOwned::try_from_admitted(fields, self.grant)
            .map(|payload| Canonicalized {
                payload,
                presentation: self.presentation,
            })
            .map_err(|_| CanonError::Storage(StorageError::AllocationFailed))
    }
}

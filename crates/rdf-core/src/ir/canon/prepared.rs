// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A canonical state kept between calls over the same frozen dataset.
//!
//! One canonical run spends most of its work finding the label of each blank
//! node and the order of the lines. A [`PreparedCanon`] keeps exactly those two
//! results, so a later document or digest run over the same snapshot only
//! spells each line from the view. It is built under the source's owned
//! account, like [`Canonicalized`](super::Canonicalized), and the caller decides
//! when it is dropped.

use std::collections::BTreeMap;

use purrdf_hash::hex::Digest32;
use purrdf_lex::allocation::{SharedOwned, StorageError};

use super::order::Retained;
use super::ownership::SourceGrant;
use super::{CanonError, CanonHash, CanonPresentation, CanonSelection};
use crate::{DatasetView, ViewTermId};

/// What a canonical run computes: the label hash, the presentation and the scope.
///
/// A [`PreparedCanon`] is built for one request and is refused for any other,
/// so the same value is presented to the preparing call and to every call that
/// reuses its product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CanonRequest<'a> {
    /// The blank-label hash algorithm.
    pub hash: CanonHash,
    /// How the statement layer is spelled.
    pub presentation: CanonPresentation,
    /// The named graph to canonicalize by its exact IRI, or the whole dataset.
    pub graph: Option<&'a str>,
}

impl<'a> CanonRequest<'a> {
    /// The whole dataset under `presentation`.
    #[must_use]
    pub const fn new(hash: CanonHash, presentation: CanonPresentation) -> Self {
        Self {
            hash,
            presentation,
            graph: None,
        }
    }

    /// The same request restricted to one named graph, selected by its exact IRI.
    #[must_use]
    pub const fn in_graph(mut self, graph: &'a str) -> Self {
        self.graph = Some(graph);
        self
    }
}

/// Why a [`PreparedCanon`] cannot be built or used. Never a silent rebuild and
/// never a silent reuse: the call fails before any canonical work is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PreparedRefusal {
    /// The view carries no snapshot identity, so nothing can bind a product to it.
    SnapshotUnavailable,
    /// The product was prepared over a different snapshot.
    Snapshot,
    /// The product was prepared under a different blank-label hash.
    Hash,
    /// The product was prepared under a different presentation.
    Presentation,
    /// The product was prepared for a different scope (whole dataset or graph).
    Selection,
}

impl std::fmt::Display for PreparedRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SnapshotUnavailable => {
                "the view carries no snapshot identity to bind a prepared canonical state to"
            }
            Self::Snapshot => "the prepared canonical state belongs to a different snapshot",
            Self::Hash => "the prepared canonical state was built under a different label hash",
            Self::Presentation => {
                "the prepared canonical state was built under a different presentation"
            }
            Self::Selection => "the prepared canonical state was built for a different scope",
        })
    }
}

impl std::error::Error for PreparedRefusal {}

impl purrdf_hash::error::StructuredError for PreparedRefusal {
    purrdf_hash::error_visit! {
        SnapshotUnavailable => "canonical-prepared-snapshot-unavailable" {},
        Snapshot => "canonical-prepared-snapshot" {},
        Hash => "canonical-prepared-hash" {},
        Presentation => "canonical-prepared-presentation" {},
        Selection => "canonical-prepared-selection" {},
    }
}

/// Everything a product is bound to. The snapshot identity is the view's own.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Key {
    hash: CanonHash,
    presentation: CanonPresentation,
    selection: CanonSelection,
    snapshot: Digest32,
}

impl Key {
    pub(super) fn new(request: CanonRequest<'_>, snapshot: Digest32) -> Self {
        Self {
            hash: request.hash,
            presentation: request.presentation,
            selection: request.graph.map_or(CanonSelection::Dataset, |iri| {
                CanonSelection::NamedGraph(iri.to_owned())
            }),
            snapshot,
        }
    }

    /// The first component in which `request` and `snapshot` differ from this key.
    pub(super) fn check(
        &self,
        request: CanonRequest<'_>,
        snapshot: Digest32,
    ) -> Result<(), PreparedRefusal> {
        if self.hash != request.hash {
            return Err(PreparedRefusal::Hash);
        }
        if self.presentation != request.presentation {
            return Err(PreparedRefusal::Presentation);
        }
        let same_selection = match (&self.selection, request.graph) {
            (CanonSelection::Dataset, None) => true,
            (CanonSelection::NamedGraph(iri), Some(graph)) => iri == graph,
            _ => false,
        };
        if !same_selection {
            return Err(PreparedRefusal::Selection);
        }
        if self.snapshot != snapshot {
            return Err(PreparedRefusal::Snapshot);
        }
        Ok(())
    }
}

/// The issued label of each blank node: ids ascending, their labels back to
/// back in one text and the end of each label in it.
pub(super) struct Labels<Id> {
    ids: Vec<Id>,
    ends: Vec<u32>,
    text: String,
}

impl<Id: ViewTermId> Labels<Id> {
    const fn empty() -> Self {
        Self {
            ids: Vec::new(),
            ends: Vec::new(),
            text: String::new(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.ids.len()
    }

    /// The label of `id` without `_:`, if a label was issued to it.
    pub(super) fn get(&self, id: Id) -> Option<&str> {
        self.ids.binary_search(&id).ok().map(|at| self.nth(at).1)
    }

    /// Every blank and its label, ascending by id.
    pub(super) fn iter(&self) -> impl Iterator<Item = (Id, &str)> {
        (0..self.ids.len()).map(|at| self.nth(at))
    }

    fn nth(&self, at: usize) -> (Id, &str) {
        let start = if at == 0 {
            0
        } else {
            self.ends[at - 1] as usize
        };
        (self.ids[at], &self.text[start..self.ends[at] as usize])
    }
}

/// The retained fields, bound to one key.
pub(super) struct Fields<Id> {
    pub(super) key: Key,
    pub(super) labels: Labels<Id>,
    pub(super) lines: Retained<Id>,
    /// Bytes admitted to the source's owned account for this product.
    admitted: usize,
}

/// A canonical state kept for reuse: the label of every blank node and the
/// canonical lines in document order, for one snapshot, label hash,
/// presentation and scope.
///
/// A document, SHA-256 or BLAKE3 run against it ([`try_canonicalize_prepared_view`],
/// [`try_digest_prepared_view`], [`try_blake3_prepared_view`]) gives exactly the
/// bytes, labels and digests of the matching entry that takes no product, without
/// repeating the label search, the symbol discovery or the ranking.
///
/// The product is retained memory. It is admitted to the source's owned account
/// when it is built, so a bounded source can refuse it, and the account is
/// released when the last clone is dropped. [`retained_bytes`](Self::retained_bytes)
/// states what it holds. Cloning shares one immutable payload.
///
/// For `W` rank words (one per symbol occurrence plus one per line), `S` distinct
/// symbols, `B` blank nodes and `T` bytes of label and rewritten-literal text,
/// `retained_bytes()` is at most `4W + 12S + 8B + T` plus a header of fixed size and
/// the capacity slack of each array. For data without triple terms, `W <= 5L` and
/// `S <= 4L` for `L` canonical lines.
///
/// [`try_canonicalize_prepared_view`]: super::try_canonicalize_prepared_view
/// [`try_digest_prepared_view`]: super::try_digest_prepared_view
/// [`try_blake3_prepared_view`]: super::try_blake3_prepared_view
#[derive(Clone)]
pub struct PreparedCanon<Id = crate::TermId> {
    payload: SharedOwned<Fields<Id>>,
}

impl<Id: ViewTermId> std::fmt::Debug for PreparedCanon<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedCanon")
            .field("key", &self.payload.key)
            .field("lines", &self.payload.lines.lines())
            .field("blank_nodes", &self.payload.labels.ids.len())
            .field("retained_bytes", &self.payload.admitted)
            .finish()
    }
}

impl<Id: ViewTermId> PreparedCanon<Id> {
    pub(super) fn fields(&self) -> &Fields<Id> {
        &self.payload
    }

    /// The blank-label hash this product was built under.
    #[must_use]
    pub fn hash(&self) -> CanonHash {
        self.payload.key.hash
    }

    /// The presentation this product was built under.
    #[must_use]
    pub fn presentation(&self) -> CanonPresentation {
        self.payload.key.presentation
    }

    /// The scope this product was built for.
    #[must_use]
    pub fn selection(&self) -> &CanonSelection {
        &self.payload.key.selection
    }

    /// The snapshot identity of the view this product was built from.
    #[must_use]
    pub fn snapshot_identity(&self) -> Digest32 {
        self.payload.key.snapshot
    }

    /// The number of canonical lines the product holds.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.payload.lines.lines()
    }

    /// The number of blank nodes the product holds a label for.
    #[must_use]
    pub fn blank_count(&self) -> usize {
        self.payload.labels.len()
    }

    /// The length in bytes of the canonical document the product reproduces.
    #[must_use]
    pub fn document_len(&self) -> usize {
        self.payload.lines.total()
    }

    /// Bytes admitted to the source's owned account for this product, including
    /// its shared header. Released when the last clone is dropped.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.payload.admitted
    }
}

/// Builds a product under the source's owned account.
pub(super) struct Builder<Id, E> {
    key: Option<Key>,
    labels: Labels<Id>,
    lines: Retained<Id>,
    grant: SourceGrant<E>,
}

impl<Id: ViewTermId, E: Send + Sync + 'static> Builder<Id, E> {
    pub(super) fn new<D: DatasetView<Id = Id, ReadError = E>>(
        view: &D,
        key: Key,
    ) -> Result<Self, CanonError<E>> {
        Ok(Self {
            key: Some(key),
            labels: Labels::empty(),
            lines: Retained::empty(),
            grant: SourceGrant::new(view)?,
        })
    }

    /// Keep the issued labels, which iterate ascending by id.
    pub(super) fn labels(&mut self, issued: &BTreeMap<Id, Box<str>>) -> Result<(), CanonError<E>> {
        let text: usize = issued.values().map(|label| label.len()).sum();
        let labels = &mut self.labels;
        self.grant.memory(|memory| {
            memory.reserve(&mut labels.ids, issued.len())?;
            memory.reserve(&mut labels.ends, issued.len())?;
            memory.reserve_string(&mut labels.text, text)?;
            for (&id, label) in issued {
                memory.push_str(&mut labels.text, label)?;
                let end =
                    u32::try_from(labels.text.len()).map_err(|_| StorageError::SizeOverflow)?;
                memory.push(&mut labels.ids, id)?;
                memory.push(&mut labels.ends, end)?;
            }
            Ok(())
        })
    }

    /// Copy the ordered lines under this builder's account.
    pub(super) fn retain(
        &mut self,
        capture: impl FnOnce(
            &mut purrdf_lex::allocation::Memory<'_, SourceGrant<E>>,
        ) -> Result<Retained<Id>, StorageError>,
    ) -> Result<(), CanonError<E>> {
        self.lines = self.grant.memory(capture)?;
        Ok(())
    }

    pub(super) fn finish(
        mut self,
        stop: Option<&dyn crate::StopSignal>,
    ) -> Result<PreparedCanon<Id>, CanonError<E>> {
        let control = SharedOwned::<Fields<Id>>::allocation_layout::<SourceGrant<E>>().size();
        self.grant.memory(|memory| memory.add_bytes(control))?;
        super::check_stop(stop).map_err(CanonError::with_read_error)?;
        let admitted = self.grant.live();
        let fields = Fields {
            key: self
                .key
                .take()
                .expect("a builder holds its key until finished"),
            labels: self.labels,
            lines: self.lines,
            admitted,
        };
        SharedOwned::try_from_admitted(fields, self.grant)
            .map(|payload| PreparedCanon { payload })
            .map_err(|_| CanonError::Storage(StorageError::AllocationFailed))
    }
}

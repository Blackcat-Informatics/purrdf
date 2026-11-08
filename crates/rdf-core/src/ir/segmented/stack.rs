// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable chronological descriptors and sealed, source-local metadata owners.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use purrdf_lex::allocation::{Memory, Resident};

use crate::RdfDataset;

use super::session::SegmentedOperation;
use super::{SegmentedError, SegmentedProvider, SegmentedReadLimits, SegmentedReceipt};

/// Frozen metadata belonging to exactly one chronological layer. Removals keep
/// their own validated dictionary; explicit graph declarations are a separate
/// declaration-only dataset, so graphs mentioned by removals are not declarations.
#[derive(Debug, Clone, Default)]
pub struct SegmentedLayerMetadata {
    removals: Option<Arc<RdfDataset>>,
    declarations: Option<Arc<RdfDataset>>,
    max_owned_term_bytes: u64,
    pub(super) statistics_revision: u64,
}

impl SegmentedLayerMetadata {
    /// Adopt already-normalized immutable carriers without copying their terms.
    ///
    /// # Errors
    /// A declaration carrier must contain graph slots only, never statements.
    pub fn new(
        removals: Option<Arc<RdfDataset>>,
        declarations: Option<Arc<RdfDataset>>,
    ) -> Result<Self, SegmentedError> {
        if declarations.as_ref().is_some_and(|dataset| {
            dataset.quad_count() != 0
                || dataset.reifiers().next().is_some()
                || dataset.annotations().next().is_some()
        }) {
            return Err(SegmentedError::Corrupt(
                "graph declaration carrier contains statements",
            ));
        }
        let max_owned_term_bytes =
            removals
                .iter()
                .chain(declarations.iter())
                .try_fold(0, |widest, dataset| {
                    super::build::resident_owned_term_bound(dataset).map(|bound| widest.max(bound))
                })?;
        Ok(Self {
            statistics_revision: purrdf_hash::fixed::hash_one(&(
                removals
                    .as_ref()
                    .map(|dataset| resident_statistics_revision(dataset))
                    .transpose()?,
                declarations
                    .as_ref()
                    .map(|dataset| resident_statistics_revision(dataset))
                    .transpose()?,
            )),
            removals,
            declarations,
            max_owned_term_bytes,
        })
    }

    /// Original normalized removal carrier, shallow-pinned across publications.
    #[must_use]
    pub fn removals(&self) -> Option<&Arc<RdfDataset>> {
        self.removals.as_ref()
    }

    /// Original explicit graph declarations, distinct from implicit row presence.
    #[must_use]
    pub fn declarations(&self) -> Option<&Arc<RdfDataset>> {
        self.declarations.as_ref()
    }

    pub(super) const fn max_owned_term_bytes(&self) -> u64 {
        self.max_owned_term_bytes
    }
}

/// One certified source plus metadata frozen once at its publication boundary.
#[derive(Debug)]
pub struct SegmentedLayer {
    pub(super) provider: Arc<dyn SegmentedProvider>,
    pub(super) receipt: SegmentedReceipt,
    pub(super) metadata: SegmentedLayerMetadata,
}

impl SegmentedLayer {
    /// Attach an exact certified descriptor. Block authentication is performed
    /// under the read operation's limits, rather than decoding a dictionary here.
    ///
    /// # Errors
    /// Refuses a provider already reporting another generation or encoded length.
    pub fn new(
        provider: Arc<dyn SegmentedProvider>,
        receipt: SegmentedReceipt,
        metadata: SegmentedLayerMetadata,
    ) -> Result<Self, SegmentedError> {
        let layer = Self {
            provider,
            receipt,
            metadata,
        };
        layer.check_descriptor()?;
        Ok(layer)
    }

    pub(super) fn check_descriptor(&self) -> Result<(), SegmentedError> {
        if self.provider.snapshot() != self.receipt.snapshot()
            || self.provider.byte_len() != self.receipt.byte_len()
        {
            return Err(SegmentedError::SnapshotMismatch);
        }
        Ok(())
    }

    /// Original source-local certification receipt.
    #[must_use]
    pub const fn receipt(&self) -> &SegmentedReceipt {
        &self.receipt
    }

    /// Immutable metadata; obtaining it performs no value traversal.
    #[must_use]
    pub const fn metadata(&self) -> &SegmentedLayerMetadata {
        &self.metadata
    }
}

/// The current resident batch, replaced atomically by its caller. Sharing the
/// frozen batch allows an old snapshot to survive later head mutations without
/// copying any historical dictionary or removal set.
#[derive(Debug, Clone)]
pub struct SegmentedResidentHead {
    pub(super) dataset: Arc<RdfDataset>,
    pub(super) metadata: SegmentedLayerMetadata,
    pub(super) max_owned_term_bytes: u64,
    pub(super) statistics_revision: u64,
}

impl SegmentedResidentHead {
    /// Adopt a frozen current batch and its chronological metadata.
    ///
    /// # Errors
    /// Refuses allocation or arithmetic failure while certifying the original
    /// expanded-term footprint. The certificate is retained across snapshots.
    pub fn new(
        dataset: Arc<RdfDataset>,
        metadata: SegmentedLayerMetadata,
    ) -> Result<Self, SegmentedError> {
        let max_owned_term_bytes =
            super::build::resident_owned_term_bound(&dataset)?.max(metadata.max_owned_term_bytes());
        Ok(Self {
            statistics_revision: purrdf_hash::fixed::hash_one(&(
                resident_statistics_revision(&dataset)?,
                metadata.statistics_revision,
            )),
            dataset,
            metadata,
            max_owned_term_bytes,
        })
    }

    /// The original current-batch dataset.
    #[must_use]
    pub const fn dataset(&self) -> &Arc<RdfDataset> {
        &self.dataset
    }

    /// Current-batch removals and explicit declarations.
    #[must_use]
    pub const fn metadata(&self) -> &SegmentedLayerMetadata {
        &self.metadata
    }
}

fn resident_statistics_revision(dataset: &RdfDataset) -> Result<u64, SegmentedError> {
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    for index in 0..dataset.term_count() {
        let index = u32::try_from(index).map_err(|_| SegmentedError::AddressExhausted)?;
        let id =
            crate::TermId::from_index(index).map_err(purrdf_lex::allocation::StorageError::from)?;
        dataset.resolve(id).hash(&mut hash);
    }
    if let Some(statistics) = crate::DatasetView::statistics(dataset) {
        statistics.fingerprint().hash(&mut hash);
    }
    Ok(hash.finish())
}

/// Consumer-owned chronological sealed descriptors and a replaceable resident
/// head. Persistence and which segments to rewrite remain caller decisions.
#[derive(Debug, Default)]
pub struct SegmentedStack {
    layers: Vec<Arc<SegmentedLayer>>,
    head: Option<SegmentedResidentHead>,
}

impl SegmentedStack {
    /// Publish a complete replacement descriptor list, oldest first. Reuse the
    /// original Arcs for untouched segments; retained snapshots keep replaced
    /// providers independently until their final owner is released.
    ///
    /// # Errors
    /// Every successor header is authenticated under one cumulative operation.
    /// Refusal or descriptor drift leaves the current publication unchanged.
    pub fn publish(
        &mut self,
        layers: Vec<Arc<SegmentedLayer>>,
        limits: SegmentedReadLimits,
    ) -> Result<(), SegmentedError> {
        validate_layers(&layers, limits)?;
        self.layers = layers;
        Ok(())
    }

    /// Replace only the mutable publication head, preserving sealed chronology.
    pub fn set_head(&mut self, head: Option<SegmentedResidentHead>) {
        self.head = head;
    }

    /// Pin the current descriptors and head without traversing any dictionary,
    /// historical removal row, lexical payload or explicit graph declaration.
    ///
    /// # Errors
    /// Refuses descriptor drift or allocation of the descriptor vector.
    pub fn snapshot(&self) -> Result<SegmentedStackSnapshot, SegmentedError> {
        check_layers(&self.layers)?;
        let layers = Memory::new(&mut Resident).collect(self.layers.iter().map(Arc::clone))?;
        Ok(SegmentedStackSnapshot {
            layers: Arc::new(layers),
            head: self.head.clone(),
        })
    }
}

/// An immutable chronological publication. Only descriptor pointers and the
/// current head owner are copied; historical metadata stays with each layer.
#[derive(Debug, Clone)]
pub struct SegmentedStackSnapshot {
    pub(super) layers: Arc<Vec<Arc<SegmentedLayer>>>,
    pub(super) head: Option<SegmentedResidentHead>,
}

impl PartialEq for SegmentedStackSnapshot {
    fn eq(&self, other: &Self) -> bool {
        let same =
            |left: Option<&Arc<RdfDataset>>, right: Option<&Arc<RdfDataset>>| match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                _ => false,
            };
        self.layers.len() == other.layers.len()
            && self
                .layers
                .iter()
                .zip(other.layers.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
            && match (&self.head, &other.head) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    Arc::ptr_eq(&left.dataset, &right.dataset)
                        && same(left.metadata.removals(), right.metadata.removals())
                        && same(left.metadata.declarations(), right.metadata.declarations())
                }
                _ => false,
            }
    }
}
impl Eq for SegmentedStackSnapshot {}

impl SegmentedStackSnapshot {
    pub(super) fn content_identity(&self) -> purrdf_hash::hex::Digest32 {
        const SNAPSHOT: purrdf_hash::Domain =
            purrdf_hash::Domain::new(b"purrdf-core/segmented-stack-snapshot/v1");
        let mut hash = purrdf_hash::blake3::Hasher::new();
        hash.update(SNAPSHOT.as_bytes());
        hash.update(&(self.layers.len() as u64).to_le_bytes());
        fn metadata(hash: &mut purrdf_hash::blake3::Hasher, metadata: &SegmentedLayerMetadata) {
            for owner in [metadata.removals(), metadata.declarations()] {
                hash.update(&[u8::from(owner.is_some())]);
                if let Some(owner) = owner {
                    hash.update(owner.frozen_snapshot_identity().as_bytes());
                }
            }
        }
        for layer in self.layers.iter() {
            hash.update(layer.receipt.snapshot.as_bytes());
            metadata(&mut hash, &layer.metadata);
        }
        hash.update(&[u8::from(self.head.is_some())]);
        if let Some(head) = &self.head {
            hash.update(head.dataset.frozen_snapshot_identity().as_bytes());
            metadata(&mut hash, &head.metadata);
        }
        purrdf_hash::hex::Digest32::new(*hash.finalize().as_bytes())
    }

    /// Original chronological source descriptors, including empty layers.
    #[must_use]
    pub fn layers(&self) -> &[Arc<SegmentedLayer>] {
        &self.layers
    }

    /// The frozen current batch retained by this publication.
    #[must_use]
    pub const fn head(&self) -> Option<&SegmentedResidentHead> {
        self.head.as_ref()
    }

    /// Authenticate every source header without reading dictionary or row blocks.
    ///
    /// # Errors
    /// One budget and exact request ledger apply across all sources. A final
    /// descriptor check includes sources already evicted from the shared cache.
    pub fn validate(&self, limits: SegmentedReadLimits) -> Result<(), SegmentedError> {
        validate_layers(&self.layers, limits)
    }
}

fn check_layers(layers: &[Arc<SegmentedLayer>]) -> Result<(), SegmentedError> {
    for layer in layers {
        layer.check_descriptor()?;
    }
    Ok(())
}

fn validate_layers(
    layers: &[Arc<SegmentedLayer>],
    limits: SegmentedReadLimits,
) -> Result<(), SegmentedError> {
    if layers.is_empty() {
        return Ok(());
    }
    let operation = SegmentedOperation::new(limits)?;
    for layer in layers {
        operation.open(Arc::clone(&layer.provider), &layer.receipt)?;
    }
    check_layers(layers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RdfDatasetBuilder, SegmentedBuildLimits, SegmentedBuilder, TermValue};

    fn layer(label: &str, metadata: SegmentedLayerMetadata) -> Arc<SegmentedLayer> {
        let mut builder =
            SegmentedBuilder::new(SegmentedBuildLimits::new(32, 4096, 32, 512, 4).unwrap())
                .unwrap();
        builder
            .intern_batch(&[TermValue::iri(label)], |_, _| {})
            .unwrap();
        let image = builder.seal().unwrap();
        Arc::new(
            SegmentedLayer::new(
                Arc::new(image.provider()),
                image.receipt().clone(),
                metadata,
            )
            .unwrap(),
        )
    }

    fn resident(label: &str) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let id = builder.intern_iri(label).unwrap();
        builder.push_quad(id, id, id, None).unwrap();
        builder.freeze().unwrap()
    }

    #[test]
    fn snapshots_pin_original_metadata_and_atomic_replacement_releases_old_owners() {
        let limits = SegmentedReadLimits::new(1_000_000, 2, 2000, 1_000_000, 4);
        let removed = resident("urn:removed");
        let head = resident("urn:old-head");
        let old = layer(
            "urn:old",
            SegmentedLayerMetadata::new(Some(Arc::clone(&removed)), None).unwrap(),
        );
        let old_weak = Arc::downgrade(&old);
        let removed_weak = Arc::downgrade(&removed);
        let mut stack = SegmentedStack::default();
        stack.publish(vec![Arc::clone(&old)], limits).unwrap();
        stack.set_head(Some(
            SegmentedResidentHead::new(Arc::clone(&head), SegmentedLayerMetadata::default())
                .unwrap(),
        ));
        let historical = stack.snapshot().unwrap();
        let sibling = stack.snapshot().unwrap();
        assert!(Arc::ptr_eq(&historical.layers()[0], &sibling.layers()[0]));
        assert!(Arc::ptr_eq(
            historical.layers()[0].metadata().removals().unwrap(),
            &removed
        ));
        assert!(Arc::ptr_eq(historical.head().unwrap().dataset(), &head));
        let replacement = layer("urn:new", SegmentedLayerMetadata::default());
        assert!(
            stack
                .publish(
                    vec![Arc::clone(&replacement)],
                    SegmentedReadLimits::new(0, 1, 1, 0, 0)
                )
                .is_err()
        );
        assert!(Arc::ptr_eq(&stack.snapshot().unwrap().layers()[0], &old));
        stack.publish(vec![replacement], limits).unwrap();
        stack.set_head(None);
        drop(old);
        drop(removed);
        assert!(old_weak.upgrade().is_some());
        historical.validate(limits).unwrap();
        drop(historical);
        assert!(old_weak.upgrade().is_some());
        drop(sibling);
        assert!(old_weak.upgrade().is_none());
        assert!(removed_weak.upgrade().is_none());
        assert!(stack.snapshot().unwrap().head().is_none());
    }

    #[test]
    fn declaration_carrier_rejects_implicit_row_graph_presence() {
        assert!(matches!(
            SegmentedLayerMetadata::new(None, Some(resident("urn:row"))),
            Err(SegmentedError::Corrupt(_))
        ));
        let mut builder = RdfDatasetBuilder::new();
        let graph = builder.intern_iri("urn:empty-graph").unwrap();
        builder.declare_named_graph(graph).unwrap();
        let declarations = builder.freeze().unwrap();
        let metadata = SegmentedLayerMetadata::new(None, Some(Arc::clone(&declarations))).unwrap();
        assert!(Arc::ptr_eq(metadata.declarations().unwrap(), &declarations));
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `GtsBundle` — the frozen hot graph paired with its out-of-band envelope
//! (#819 C2).
//!
//! An [`RdfDataset`] is the immutable, value-interned RDF 1.2 graph: the *hot*
//! triple/quad surface every consumer reasons over. Material that travels *with*
//! the graph but is not part of it — GTS metadata, segment ledgers, blobs,
//! suppression overlays, opaque nodes, signatures — lives in the [`RdfEnvelope`],
//! keyed through the crate's existing [`RdfLookaside`] (`store.rs`).
//!
//! Both [`GtsBundle`] and [`RdfEnvelope`] are `#[non_exhaustive]`: #820 extends the
//! envelope additively with provenance / units / artifacts / blob fields, and a
//! `#[non_exhaustive]` struct lets those land without a breaking change. Consumers
//! therefore construct these only through the provided constructors.

use std::sync::Arc;

use super::dataset::RdfDataset;
use crate::RdfLookaside;

/// The frozen RDF 1.2 hot graph plus its out-of-band envelope.
///
/// The dataset is the value-interned, immutable graph (C0/C1); the envelope
/// carries everything that travels alongside it but is not a triple (C0.6).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct GtsBundle {
    /// The immutable, value-interned RDF 1.2 dataset — the hot graph.
    pub dataset: Arc<RdfDataset>,
    /// Out-of-band material that travels with the dataset (C0.6).
    pub envelope: RdfEnvelope,
}

impl GtsBundle {
    /// Pair a frozen dataset with its envelope.
    pub fn new(dataset: Arc<RdfDataset>, envelope: RdfEnvelope) -> Self {
        Self { dataset, envelope }
    }
}

/// Out-of-band material that travels with an [`RdfDataset`] but is not part of the
/// hot graph (C0.6).
///
/// `#[non_exhaustive]` because #820 grows this envelope additively (provenance,
/// units, artifacts, decoded blobs); construct it via [`RdfEnvelope::new`] or
/// [`RdfEnvelope::default`].
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RdfEnvelope {
    /// Structured non-triple companion material (GTS metadata, segments, blobs,
    /// suppressions, opaque nodes, signatures), reusing the crate's existing
    /// [`RdfLookaside`].
    pub lookaside: RdfLookaside,
}

impl RdfEnvelope {
    /// Build an envelope from a lookaside.
    pub fn new(lookaside: RdfLookaside) -> Self {
        Self { lookaside }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;

    #[test]
    fn bundle_pairs_dataset_with_envelope() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s".to_string());
        let p = b.intern_iri("http://example.org/p".to_string());
        let o = b.intern_iri("http://example.org/o".to_string());
        b.push_quad(s, p, o, None);
        let dataset = b.freeze().expect("valid");

        let bundle = GtsBundle::new(dataset, RdfEnvelope::default());
        assert_eq!(bundle.dataset.quad_count(), 1);
        assert!(bundle.envelope.lookaside.is_empty());
    }

    #[test]
    fn envelope_default_is_empty() {
        assert!(RdfEnvelope::default().lookaside.is_empty());
        assert_eq!(
            RdfEnvelope::default(),
            RdfEnvelope::new(RdfLookaside::default())
        );
    }
}

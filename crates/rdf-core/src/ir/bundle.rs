// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The `GtsBundle` — the frozen hot graph paired with its out-of-band envelope
//! (C2).
//!
//! An [`RdfDataset`] is the immutable, value-interned RDF 1.2 graph: the *hot*
//! triple/quad surface every consumer reasons over. Material that travels *with*
//! the graph but is not part of it — GTS metadata, segment ledgers, blobs,
//! suppression overlays, opaque nodes, signatures — lives in the [`RdfEnvelope`],
//! keyed through the crate's existing [`RdfLookaside`] (`store.rs`).
//!
//! Both [`GtsBundle`] and [`RdfEnvelope`] are `#[non_exhaustive]`: extends the
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

/// How many distinct language-tagged literals a read saw, by the case form of the tag as
/// written.
///
/// A dataset holds a tag folded to ASCII lower case, so the written case of a tag is gone once
/// the read has interned it. A reader that sees the tag as written records the figure here,
/// one count per distinct dataset literal. A tag with no upper-case letter is `lower`, one
/// with upper-case and no lower-case letters is `upper`, and one with both is `mixed`. A
/// literal written under several case variants of its tag counts once, under the greatest of
/// `lower`, `upper` and `mixed` that any of them has, so the three counts sum to the number of
/// language-tagged literals the read produced.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LanguageTagForms {
    /// Literals every written tag of which has no upper-case letter.
    pub lower: u64,
    /// Literals with a written tag of upper-case and no lower-case letters, and none mixed.
    pub upper: u64,
    /// Literals with a written tag of both upper-case and lower-case letters.
    pub mixed: u64,
}

impl LanguageTagForms {
    /// The figure of `lower`, `upper` and `mixed` literals.
    #[must_use]
    pub const fn new(lower: u64, upper: u64, mixed: u64) -> Self {
        Self {
            lower,
            upper,
            mixed,
        }
    }
}

/// Out-of-band material that travels with an [`RdfDataset`] but is not part of the
/// hot graph (C0.6).
///
/// `#[non_exhaustive]` because grows this envelope additively (provenance,
/// units, artifacts, decoded blobs); construct it via [`RdfEnvelope::new`] or
/// [`RdfEnvelope::default`].
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RdfEnvelope {
    /// Structured non-triple companion material (GTS metadata, segments, blobs,
    /// suppressions, opaque nodes, signatures), reusing the crate's existing
    /// [`RdfLookaside`].
    pub lookaside: RdfLookaside,
    /// The losses the read incurred that no RDF dataset can represent, such as
    /// the written spelling of a language tag the dataset folds to lower case.
    pub losses: crate::loss::LossLedger,
    /// The written case forms of the language-tagged literals of the read.
    pub language_tag_forms: LanguageTagForms,
}

impl RdfEnvelope {
    /// Build an envelope from a lookaside.
    pub fn new(lookaside: RdfLookaside) -> Self {
        Self {
            lookaside,
            losses: crate::loss::LossLedger::new(),
            language_tag_forms: LanguageTagForms::default(),
        }
    }

    /// This envelope carrying the losses of the read.
    #[must_use]
    pub fn with_losses(mut self, losses: crate::loss::LossLedger) -> Self {
        self.losses = losses;
        self
    }

    /// This envelope carrying the written case forms of the language tags of the read.
    #[must_use]
    pub const fn with_language_tag_forms(mut self, forms: LanguageTagForms) -> Self {
        self.language_tag_forms = forms;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;

    #[test]
    fn bundle_pairs_dataset_with_envelope() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s").unwrap();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let o = b.intern_iri("http://example.org/o").unwrap();
        b.push_quad(s, p, o, None).unwrap();
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

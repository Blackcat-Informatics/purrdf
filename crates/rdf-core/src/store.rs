// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Capability flags exposed by an RDF dataset/import boundary.
// Each capability is an independent yes/no feature probe, not an encoded state
// machine — a bitflags/enum rewrite would only obscure the public API.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfStoreCapabilities {
    /// Whether quads outside the default graph (named graphs) are representable.
    pub named_graphs: bool,
    /// Whether RDF 1.2 triple terms (quoted triples) are representable.
    pub quoted_triples: bool,
    /// Whether RDF 1.2 reifier bindings are representable.
    pub reifiers: bool,
    /// Whether RDF 1.2 statement annotations are representable.
    pub annotations: bool,
    /// Whether source/location context (`RdfLocation`) is preserved.
    pub source_locations: bool,
    /// Whether conversion loss records (`RdfLoss`) are preserved.
    pub loss_records: bool,
    /// Whether structured non-triple lookaside material is preserved.
    pub lookaside: bool,
}

impl RdfStoreCapabilities {
    /// The plain-RDF baseline: every capability flag off.
    pub const fn plain_rdf() -> Self {
        Self {
            named_graphs: false,
            quoted_triples: false,
            reifiers: false,
            annotations: false,
            source_locations: false,
            loss_records: false,
            lookaside: false,
        }
    }

    /// The field-wise logical OR of two capability sets: a flag is on in the result
    /// iff it is on in EITHER input. A composite view over several backing stores
    /// (e.g. a paged dataset folding many pages) reports a capability if any page
    /// surfaces it, so the union is the honest aggregate capability.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            named_graphs: self.named_graphs || other.named_graphs,
            quoted_triples: self.quoted_triples || other.quoted_triples,
            reifiers: self.reifiers || other.reifiers,
            annotations: self.annotations || other.annotations,
            source_locations: self.source_locations || other.source_locations,
            loss_records: self.loss_records || other.loss_records,
            lookaside: self.lookaside || other.lookaside,
        }
    }
}

impl Default for RdfStoreCapabilities {
    fn default() -> Self {
        Self::plain_rdf()
    }
}

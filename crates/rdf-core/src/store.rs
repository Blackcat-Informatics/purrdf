// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Capability flags exposed by an RDF dataset/import boundary.
// Each capability is an independent yes/no feature probe, not an encoded state
// machine — a bitflags/enum rewrite would only obscure the public API.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfStoreCapabilities {
    pub named_graphs: bool,
    pub quoted_triples: bool,
    pub reifiers: bool,
    pub annotations: bool,
    pub source_locations: bool,
    pub loss_records: bool,
    pub lookaside: bool,
}

impl RdfStoreCapabilities {
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
}

impl Default for RdfStoreCapabilities {
    fn default() -> Self {
        Self::plain_rdf()
    }
}

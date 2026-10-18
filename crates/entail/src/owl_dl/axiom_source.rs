// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original normalization anchors, retained in native TBox insertion order.
//!
//! An anchor identifies the input operation that created an inclusion. It is
//! not a complete syntax-read trace, a causal premise set or a second parser.

/// Native origin of one normalized inclusion, before absorption splits it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AxiomSource {
    /// No original-source claim, including deliberately synthetic test knowledge bases.
    #[default]
    Unrecorded,
    /// A per-triple axiom read by the original reverse mapper.
    Triple(u32, u32, u32),
    /// N-ary axiom or negative property assertion read from its source node.
    Node {
        /// Original axiom node.
        node: u32,
        /// Actual typing trigger; absent for vocabulary-identified negative assertions.
        kind: Option<u32>,
    },
    /// Native defining equation for a source class-expression cycle cut point.
    RecursiveDefinition(u32),
    /// Defining equation read from query syntax, never an asserted source axiom.
    QueryDefinition(u32),
    /// Fixed empty-extension law of a mentioned standard bottom property.
    EmptyRole(u32),
}

#[cfg(test)]
mod tests;

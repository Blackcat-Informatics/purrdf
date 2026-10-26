// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit semantics and finite work for recursive SHACL validation.

pub(crate) mod cache;
pub(crate) mod closed;
pub use closed::{ClosedRecursionRefusal, RecursionDefinition, RecursionDefinitionKind};
pub(crate) mod evidence;
pub use evidence::{RecursionCycle, RecursionPosition, RecursiveValidationReport};
pub(crate) mod graph;
pub(crate) mod guards;
pub(crate) mod model;

/// The fixed point selected for a stratified recursive shapes graph.
///
/// Both selections use the same constraint evaluator and freeze lower strata.
/// A recursive component must be monotone in its own conformance answers;
/// selecting a fixed point does not admit negative or nonmonotone recursion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecursionSemantics {
    /// The least fixed point, starting with no conforming pairs.
    Lfp,
    /// The greatest fixed point, starting with every demanded pair conforming.
    Gfp,
}

/// A caller's recursive semantics and finite native work limits.
///
/// There is no default recursive semantics. Zero is a valid work limit: it
/// refuses the first operation of that kind. Limits count original
/// focus/definition pairs and fixed-point rounds, rather than native stack depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursionOptions {
    /// The least or greatest fixed point requested by this validation.
    pub semantics: RecursionSemantics,
    /// Maximum distinct focus/definition pairs admitted in one native run.
    pub max_pairs: u64,
    /// Maximum fixed-point rounds charged across recursive components in a run.
    pub max_iterations: u64,
}

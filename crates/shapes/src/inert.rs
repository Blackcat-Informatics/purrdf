// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Declarations no shape reaches, and what is wrong with them.
//!
//! # Why a defect in an unreached declaration does not refuse the load
//!
//! A shapes graph routinely imports a library that declares far more than any one
//! shapes graph uses: DASH declares dozens of constraint components, SPARQL functions
//! and validator alternatives for SHACL Core components. Some of those declarations are
//! not well-formed SHACL 1.2 — an ASK validator under `sh:nodeValidator`, a `MINUS` in a
//! pre-bound query, a function parameter named `value`. If the load refused them, a
//! shapes graph that uses none of them could not be validated at all, although nothing
//! it asks for is ill-formed.
//!
//! So loading — the HOT admit every validation pays for — judges a declaration's
//! well-formedness where a shape REACHES it, and there the refusal is unchanged:
//!
//! * the validators of a custom constraint component, where a shape of the shapes
//!   graph uses the component;
//! * a `sh:SPARQLFunction` declaration, where a node expression or a SPARQL text a
//!   shape reaches calls the function, directly or through another function's body.
//!
//! A validator declared for a built-in constraint component is never reached: the
//! native implementation is the one that runs (see [`crate::validator_alternatives`]).
//!
//! A defect nothing reaches is not silenced. It is an [`InertDefect`], and the COLD
//! certify surface, [`crate::lint`], reports every one as a finding.

use std::fmt;

/// A defect in a declaration no shape reaches: loading accepts it, [`crate::lint`]
/// reports it as a finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InertDefect {
    /// The declaration, as the engine renders it: e.g. `component
    /// <http://example.org/ns#C> validator _:b0`, or `sh:SPARQLFunction
    /// <http://example.org/ns#f>`.
    pub declaration: String,
    /// The refusal the loader would raise were the declaration reached.
    pub message: String,
}

impl fmt::Display for InertDefect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.declaration, self.message)
    }
}

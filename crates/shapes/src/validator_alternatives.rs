// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The validators a shapes graph declares for a built-in constraint component —
//! alternatives the native implementation supersedes, each named.
//!
//! # Why a built-in may carry validators
//!
//! SHACL 1.2 SPARQL Extensions, "Validators": "For a given constraint, a validator is
//! selected from the constraint component using the following rules, in order: For node
//! shapes, use one of the values of sh:nodeValidator, if present. For property shapes,
//! use one of the values of sh:propertyValidator, if present. Otherwise, use one of the
//! values of sh:validator." A component may therefore declare several validators, each
//! an implementation of the same component, and "SHACL processors may choose
//! alternative approaches as long as the outcome is equivalent" ("Validation with
//! SPARQL-based Constraint Components"). Vocabularies use this: DASH gives SHACL Core
//! components SPARQL validators beside the processor's own implementation.
//!
//! For a component this engine implements natively — every SHACL Core component — the
//! native implementation IS the validator it chooses. A declared validator of such a
//! component binds as an alternative: checked against the syntax rules (its type, its
//! query's form and grammar) but never executed, so its query may call functions
//! this engine does not have. The component's semantics are the
//! specification's either way, so nothing is dropped; the alternative is simply not the
//! implementation that runs. Because an alternative is invisible in a validation report,
//! [`crate::lint`] lists every one.
//!
//! # What is not an alternative
//!
//! A SPARQL alternative must still be a well-formed validator of its attachment: "The
//! values of sh:nodeValidator must be SELECT-based validators. The values of
//! sh:propertyValidator must be SELECT-based validators" and "The values of
//! sh:validator must be ASK-based validators" (SHACL 1.2 SPARQL Extensions,
//! "SELECT-based Validators" and "ASK-based Validators"). An untyped node, or a SPARQL
//! validator of the other query form, violates those rules, and SHACL 1.2 Core,
//! "Handling of Ill-formed Shapes Graphs", says "A SHACL processor SHOULD produce a
//! failure in this case" — so each is a load error ([`crate::ShapesError::IllFormed`]),
//! on a built-in as on a custom component, although the native implementation means an
//! alternative never runs: the sentence has no reachability qualifier.
//!
//! A pre-binding violation is different. SHACL 1.2 SPARQL Extensions, Appendix A, requires
//! a failure for a query "executed with pre-bound variables", and an alternative is never
//! executed, so a `MINUS` (or another restricted construct) in an alternative's query
//! refuses nothing: it loads, and [`crate::lint`] reports it as a finding in its
//! `unexecuted` section. The same query as the selected validator of a custom component
//! a shape uses refuses the load ([`crate::ShapesError::Prebinding`]).
//!
//! # A SHACL-JS validator is not a validator of any attachment
//!
//! A SHACL JavaScript Extensions `sh:JSValidator` is neither ASK- nor SELECT-based, so as
//! a value of `sh:validator`, `sh:nodeValidator` or `sh:propertyValidator` it violates
//! the attachment's class rule exactly as an untyped node does, and refuses the load
//! ([`crate::ShapesError::IllFormed`]) on a built-in as on a custom component. It is
//! never an alternative, so no language label names it.

use crate::term::Term;

/// The query form of a declared SPARQL validator, as its SHACL type says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValidatorLanguage {
    /// A SHACL instance of `sh:SPARQLAskValidator`.
    SparqlAsk,
    /// A SHACL instance of `sh:SPARQLSelectValidator`.
    SparqlSelect,
}

impl ValidatorLanguage {
    /// The stable kebab-case label every host prints: `sparql-ask` or `sparql-select`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SparqlAsk => "sparql-ask",
            Self::SparqlSelect => "sparql-select",
        }
    }
}

/// One validator declared for a built-in component, superseded by the native
/// implementation. Ordered by component, attachment, validator (canonical term order)
/// and language.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AlternativeValidator {
    /// The built-in constraint component's IRI.
    pub component: String,
    /// The attaching predicate's IRI: `sh:validator`, `sh:nodeValidator` or
    /// `sh:propertyValidator`.
    pub attachment: String,
    /// The validator node.
    pub validator: Term,
    /// Its query form.
    pub language: ValidatorLanguage,
}

impl Ord for AlternativeValidator {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.component
            .cmp(&other.component)
            .then_with(|| self.attachment.cmp(&other.attachment))
            .then_with(|| crate::term::canonical_cmp(&self.validator, &other.validator))
            .then_with(|| self.language.cmp(&other.language))
    }
}

impl PartialOrd for AlternativeValidator {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one rules engine: SHACL rules and SPARQL 1.2 RL on `purrdf-datalog`.
//!
//! Every rule PurRDF executes is first a rule of the rule-set IR ([`ir`]). The IR is
//! lowered onto `purrdf-datalog`'s clause IR, scheduled — by the rules' declared
//! `sh:layer` / `sh:order` / `sh:runOnce` for SHACL rules, by SPARQL 1.2 RL's rule-level
//! stratification otherwise ([`purrdf_datalog::schedule::stratify_rules`]) — and
//! evaluated by that crate's ordered schedule
//! ([`purrdf_datalog::schedule::evaluate_scheduled`]). This module supplies what the
//! clause IR cannot say itself: what a SPARQL expression, a SHACL node expression or a
//! SPARQL CONSTRUCT means (the guard evaluator), and the SHACL layer-boundary actions
//! (expected derived triples, their reifiers, temporary triples).
//!
//! SPARQL 1.2 RL text (W3C Working Draft 19 September 2026,
//! <https://www.w3.org/TR/2026/WD-sparql12-rl-20260919/>) enters through [`parse`] /
//! [`parse_and_check`] — the §7 grammar, then §4.2 well-formedness and §4.4
//! stratification, each refusal typed by its stage ([`SrlError`]) — and is evaluated by
//! [`infer`], SPARQL 1.2 RL's "infer" operation.
//!
//! There is no second fixpoint anywhere in this crate: [`crate::rules`] holds the SHACL
//! rule model and the producers that execute one SHACL rule once, and nothing else.

pub mod ir;

mod depend;
mod document;
mod eval;
mod lower;
mod syntax;

pub use document::{
    ImportResolver, InferOptions, RuleSetDocument, SrlError, SrlRule, Stratum, infer, parse,
    parse_and_check,
};
pub use eval::{Explanation, Inference, evaluate};

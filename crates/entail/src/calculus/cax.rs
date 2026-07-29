// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cax-*` — the semantics of class axioms, OWL 2 Profiles §4.3 Table 7.
//!
//! This family's table is empty: every `cax-*` rule this chase states today is `cax-sco`,
//! which is the OWL 2 RL name for `rdfs9`, and one clause with two names is stated once, in
//! [`super::rdfs`], where the RDFS numbering orders it. [`super::ChaseRule::rule_id`]
//! answers with `cax-sco` under the `OWL-RL` lane, so an `OWL-RL` report does name the rule
//! — this module simply is not where it is written.
//!
//! The rules this family will hold are the rest of Table 7: `cax-eqc1`, `cax-eqc2`,
//! `cax-dw` and `cax-adc`. The last two conclude an inconsistency rather than a triple,
//! which the chase reports as a witness rather than materializes.

/// The `cax-*` rules this chase states here, in OWL 2 Profiles Table 7 order: none.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry would mean. This
/// family adds nothing to the accumulated table, so the calculus is exactly what the other
/// families state.
macro_rules! cax_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)* }
    };
}

pub(crate) use cax_rules;

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cls-*` — the semantics of classes, OWL 2 Profiles §4.3 Table 6.
//!
//! This chase states NONE of Table 6 today, so the family's table is empty and it
//! contributes no clause to any lane's program. That is a statement about the calculus, and
//! it is the same statement [`crate::implemented`] makes: no `cls-*` rule is reported as
//! fired, because none can fire.
//!
//! The rules this family will hold are the whole of Table 6: `cls-thing`, `cls-nothing1`,
//! `cls-nothing2`, `cls-int1`, `cls-int2`, `cls-uni`, `cls-com`, `cls-svf1`, `cls-svf2`,
//! `cls-avf`, `cls-hv1`, `cls-hv2`, `cls-maxc1`, `cls-maxc2`, `cls-maxqc1`, `cls-maxqc2`,
//! `cls-maxqc3`, `cls-maxqc4` and `cls-oo`. Most of them read an RDF collection
//! (`owl:intersectionOf`, `owl:unionOf`, `owl:oneOf`) or a class expression, which is the
//! reason they are not stated as plain DL clauses yet.

/// The `cls-*` rules this chase states, in OWL 2 Profiles Table 6 order: none.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry would mean. This
/// family adds nothing to the accumulated table, so the calculus is exactly what the other
/// families state.
macro_rules! cls_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)* }
    };
}

pub(crate) use cls_rules;

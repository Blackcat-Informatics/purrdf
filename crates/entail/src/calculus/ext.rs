// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `ext-*` — the rules this chase fires that NO specification table states.
//!
//! Every other family module in [`super`] transcribes a published rule table. This one
//! does not, and it is kept separate for exactly that reason: a reader who wants to know
//! what PurRDF adds to OWL 2 Profiles §4.3 reads one file, and a reader who wants to know
//! what §4.3 says never has to filter this out of a table that claims to be it.
//!
//! # What an entry here owes, and what it may not do
//!
//! * It must be SOUND under the semantics of the vocabulary it reads — the same standard
//!   every normative rule meets, and the only one a rule with no specification to appeal
//!   to can meet.
//! * It must not change what the normative rules DECIDE. A lane holding a rule that
//!   concludes `false` can be made to refuse a run it would otherwise have closed, so an
//!   extension's conclusions have to be shown to reach no `false` the table did not
//!   already reach. [`different_from_symmetric`] carries that argument.
//! * It is declared with a [`RuleId`](crate::RuleId) whose canonical spelling begins
//!   `ext-`, so [`RuleId::is_extension`](crate::RuleId::is_extension) answers `true` for
//!   it and [`extensions`](crate::extensions) — not [`rules`](crate::rules), and not
//!   [`implemented`](crate::implemented) — is where it appears. `OWL-RL 78 / 78` stays a
//!   statement about Tables 4–9 however many rules land here.
//!
//! # This family is concatenated LAST, and that is load-bearing
//!
//! A rule whose head is `false` is lowered into a clause naming a clash marker built from
//! the rule's DECLARATION INDEX (see [`super::clash_marker`]), so inserting a rule ahead
//! of an existing one renumbers every marker after it and moves the contract hash of every
//! lane that fires any of them. Appending this family after the six OWL tables renumbers
//! nothing: only the lane that actually gains a clause sees its digest move, which is the
//! smallest true statement the digest can make about this change.

use purrdf_datalog::clause::DlClause;

use super::{atom, var};
use crate::vocab::OWL_DIFFERENTFROM;

/// `ext-eq-diff-sym`: `?x owl:differentFrom ?y` ⇒ `?y owl:differentFrom ?x`.
///
/// # Why it is not in the table
///
/// OWL 2 Profiles §4.3 mentions `owl:differentFrom` in exactly three rules — `eq-diff1`,
/// `eq-diff2` and `eq-diff3` — and all three read it in a BODY and conclude `false`. No
/// rule of Tables 4–9 has an `owl:differentFrom` head at all, so the property is closed
/// under nothing: `a owl:differentFrom b` licenses no triple, and in particular not
/// `b owl:differentFrom a`. W3C's own `webont-differentfrom-001` publishes precisely that
/// entailment as positive, which is how a rule set can be complete for the table and still
/// stop one triple short of a published entailment.
///
/// # Why it is sound
///
/// `owl:differentFrom` is interpreted as inequality: `a owl:differentFrom b` holds in an
/// interpretation exactly when `a` and `b` denote different individuals. Inequality is
/// symmetric, so every model of the premise is a model of the conclusion. There is no
/// side condition and no appeal to the closed world — this is the same one-line argument
/// `prp-symp` makes for a property the ontology DECLARES symmetric, made once and for all
/// for a property the specification defines to be.
///
/// # Why it decides no run the table did not already decide
///
/// The concern a new conclusion raises in this calculus is not a wrong triple but a wrong
/// REFUSAL: seventeen rules conclude `false`, and widening a relation they read could make
/// `materialize` refuse a run it used to close. It does not, and the argument is short
/// enough to check. The only rules with `owl:differentFrom` in a body are `eq-diff1..3`,
/// and each pairs it with `owl:sameAs`:
///
/// * `eq-diff1` is `?x owl:sameAs ?y`, `?x owl:differentFrom ?y` ⇒ `false`. Suppose it
///   fires on `(y, x)` only because this rule supplied `y owl:differentFrom x` from
///   `x owl:differentFrom y`. Then `y owl:sameAs x` is asserted, and `eq-sym` — which is
///   in the table — already derives `x owl:sameAs y`, so `eq-diff1` already fired on
///   `(x, y)`. The clash was reachable before, from the same data.
/// * `eq-diff2` and `eq-diff3` do not read `owl:differentFrom` at all: their premises are
///   an `owl:AllDifferent` axiom, its list, and `owl:sameAs`. This rule cannot reach them.
///
/// So the set of runs that refuse is unchanged. What can move is WHICH pair a witness
/// names when several clashes exist, and that is already decided by the evaluator's own
/// total derivation order rather than by rule membership.
pub(super) fn different_from_symmetric() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom(var("?y"), OWL_DIFFERENTFROM, var("?x")),
        vec![atom(var("?x"), OWL_DIFFERENTFROM, var("?y"))],
    )]
}

/// The `ext-*` rules this chase states.
///
/// Handed to [`super::collect_families`], which asks each family in turn; see that macro
/// for the protocol and [`super::declare_chase_rules`] for what an entry means. This
/// family is asked LAST — see the [module docs](self) for why the position is part of the
/// change and not a filing convention.
macro_rules! ext_rules {
    ($continue:ident, $rest:tt, $($rules:tt)*) => {
        $continue! { $rest $($rules)*
            /// `ext-eq-diff-sym` — `owl:differentFrom` is symmetric. NOT a rule of
            /// OWL 2 Profiles §4.3; an EXTENSION this crate declares, sound and reported
            /// as one. `OWL-RL` only.
            DifferentFromSymmetric {
                id: ExtEqDiffSym,
                lanes: [OwlRl],
                clauses: ext::different_from_symmetric,
            },
        }
    };
}

pub(crate) use ext_rules;

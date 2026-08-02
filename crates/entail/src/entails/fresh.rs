// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fresh blank nodes, with freshness **decided** rather than assumed.
//!
//! # Why a generator at all, and why it checks
//!
//! Two of this module tree's mechanisms have to name a term the input does not name.
//! [`freeze`](super::freeze) needs constants that occur nowhere in the premise, because the
//! theorem on constants — "if `Γ ⊢ φ(c)` and `c` does not occur in `Γ`, then `Γ ⊢ ∀x.φ(x)`"
//! — has that non-occurrence as its hypothesis.
//!
//! A colliding name there is not an inconvenience, it is an UNSOUNDNESS: a frozen constant
//! that already occurs in the premise makes the derivation a statement about that particular
//! individual, and generalising it to a `∀` is then a non-sequitur.
//!
//! So freshness is decided against the actual labels rather than assumed from an unlikely
//! prefix. This is the discipline
//! [`FreshSymbols`](crate::reasoner::axiom) applies inside the DL tableau, transplanted to
//! the RDF term space; it is deliberately NOT the `owl_dl::query` `Fresh` counter, which
//! emits `purrdfDLq{n}` with no collision check at all and is sound there only because its
//! consumer builds the whole term table it mints into.
//!
//! # The check is over LABELS, not over `(label, scope)`
//!
//! Two blank nodes with one label in different scopes are different nodes (`purrdf` C0.2), so
//! a label check is strictly stronger than the identity check and cannot miss a collision.
//! It is also the only check that stays right when a caller later re-scopes a dataset: a
//! label nobody uses is still unused after any renumbering, while a `(label, scope)` pair
//! that was free before a merge need not be free after one.
//!
//! # Determinism
//!
//! The prefix is a function of the input labels alone (lengthen while any of them starts with
//! it) and the ordinals are handed out in call order, so two runs over one input mint the
//! same names — which is what lets a warrant produced by one run be compared against a
//! re-lowering performed by another.

use std::collections::BTreeSet;

use purrdf_core::{BlankScope, RdfDataset, TermValue};

/// The label prefix a minted node starts from, before collision-avoidance lengthens it.
///
/// Not an IRI and not a namespace: a blank-node label is local to the graph it occurs in, so
/// this mints no vocabulary. It is spelled distinctively so that a label collision is a
/// deliberate adversarial input rather than an accident, and the lengthening below is what
/// handles the deliberate case.
const FRESH_PREFIX: &str = "purrdfEntailsFresh";

/// The character appended to the prefix on a collision.
const LENGTHEN: char = 'x';

/// A generator of blank nodes no input dataset names.
pub(crate) struct FreshBlanks {
    /// A label prefix no observed blank node begins with.
    prefix: String,
    /// The next ordinal, so successive nodes are distinct from each other too.
    next: u64,
}

impl FreshBlanks {
    /// A generator whose labels are absent from every dataset in `datasets`.
    ///
    /// Terminates because the observed labels are finitely many and finitely long, and each
    /// round adds a character: a prefix longer than the longest observed label is a prefix no
    /// observed label can begin with.
    pub(crate) fn avoiding(datasets: &[&RdfDataset]) -> Self {
        let mut labels: BTreeSet<String> = BTreeSet::new();
        for ds in datasets {
            labels.extend(labels_of(ds));
        }
        Self::avoiding_labels(&labels)
    }

    /// A generator whose labels are absent from `labels`.
    ///
    /// The primitive [`Self::avoiding`] is written in terms of, and the entry point for a
    /// caller whose question is not a dataset yet: a BASIC GRAPH PATTERN's blank nodes are
    /// labels with no graph to read them out of, and its projected variables have to become
    /// terms before any mechanism can read the question at all.
    pub(crate) fn avoiding_labels(labels: &BTreeSet<String>) -> Self {
        let mut prefix = FRESH_PREFIX.to_owned();
        while labels.iter().any(|label| label.starts_with(&prefix)) {
            prefix.push(LENGTHEN);
        }
        Self { prefix, next: 0 }
    }

    /// Mint the next fresh blank node.
    ///
    /// [`BlankScope::DEFAULT`] rather than a fresh scope, because the label is already known
    /// absent from every scope of every observed dataset — a fresh scope would add a second
    /// distinctness on top of one that already holds, and would then have to be threaded
    /// through every check that re-decides freshness.
    pub(crate) fn mint(&mut self) -> TermValue {
        let label = format!("{}{}", self.prefix, self.next);
        self.next += 1;
        TermValue::Blank {
            label,
            scope: BlankScope::DEFAULT,
        }
    }
}

/// Every blank-node label `ds` mentions, in any position of any graph.
///
/// Over EVERY graph and to any depth, because the question this answers is "could a name
/// collide with something in here", and a triple term in a named graph is somewhere in here.
///
/// # "Any position" is [`term_positions`](crate::engine::term_positions), not `quads()`
///
/// A blank node can occur in a dataset and in NO quad of it: as the reifier of a reified
/// triple, inside an annotation row, or as a declared named graph nobody put a quad in.
/// Those positions are content — [`copy_into`](crate::engine::copy_into) carries all of
/// them — so a label that occurs only there is a label that is TAKEN.
///
/// Missing one is not a cosmetic gap here. This set is the freshness hypothesis of
/// [`freeze`](super::freeze)'s theorem on constants, and a "fresh" constant that already
/// names the caller's reifier makes the derivation a statement about that reifier — which
/// is exactly the unsoundness the [module docs](self) exist to rule out. So the survey is
/// the same enumeration the copy writes, and cannot drift from it.
pub(crate) fn labels_of(ds: &RdfDataset) -> BTreeSet<String> {
    fn walk(term: &TermValue, out: &mut BTreeSet<String>) {
        match term {
            TermValue::Blank { label, .. } => {
                out.insert(label.clone());
            }
            TermValue::Triple { s, p, o } => {
                walk(s, out);
                walk(p, out);
                walk(o, out);
            }
            TermValue::Iri(_) | TermValue::Literal { .. } => {}
        }
    }
    let mut out = BTreeSet::new();
    for id in crate::engine::term_positions(ds) {
        walk(&ds.term_value(id), &mut out);
    }
    out
}

/// Whether `term` mentions a blank node `labels` names, at any depth.
///
/// The measurement side of [`FreshBlanks`]: a test asserts the non-occurrence over a whole
/// document rather than trusting that the generator was used. Production code re-decides the
/// same hypothesis against the constant list it was handed, which is a `contains` on a label
/// and needs no walk.
#[cfg(test)]
pub(crate) fn mentions_any(term: &TermValue, labels: &BTreeSet<String>) -> bool {
    match term {
        TermValue::Blank { label, .. } => labels.contains(label),
        TermValue::Triple { s, p, o } => {
            mentions_any(s, labels) || mentions_any(p, labels) || mentions_any(o, labels)
        }
        TermValue::Iri(_) | TermValue::Literal { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use purrdf_core::{BlankScope, RdfDatasetBuilder, TermValue};

    use super::{FRESH_PREFIX, FreshBlanks, mentions_any};

    /// A dataset whose one triple's subject is the blank node `label`.
    fn with_blank(label: &str, scope: u32) -> std::sync::Arc<purrdf_core::RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_blank(label, BlankScope(scope));
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze")
    }

    /// The ordinary case: nothing collides, so the prefix is the unlengthened one and
    /// successive mints differ.
    #[test]
    fn minted_nodes_are_distinct_and_absent_from_the_input() {
        let ds = with_blank("b", 0);
        let mut fresh = FreshBlanks::avoiding(&[&ds]);
        let first = fresh.mint();
        let second = fresh.mint();
        assert_ne!(first, second);
        for minted in [&first, &second] {
            let TermValue::Blank { label, .. } = minted else {
                panic!("a minted node is a blank node");
            };
            assert!(label.starts_with(FRESH_PREFIX), "{label}");
        }
    }

    /// A LABEL COLLISION IS DECIDED AGAINST THE ACTUAL LABELS. An input that already uses
    /// the unlengthened prefix pushes the generator off it — which is the whole reason this
    /// is a generator rather than a `format!`.
    #[test]
    fn a_colliding_input_lengthens_the_prefix() {
        let ds = with_blank(&format!("{FRESH_PREFIX}0"), 0);
        let mut fresh = FreshBlanks::avoiding(&[&ds]);
        let TermValue::Blank { label, .. } = fresh.mint() else {
            panic!("a minted node is a blank node");
        };
        assert_ne!(label, format!("{FRESH_PREFIX}0"));
        assert!(label.starts_with(FRESH_PREFIX), "{label}");
        assert!(label.len() > FRESH_PREFIX.len() + 1, "{label}");
    }

    /// The check is over LABELS, so a colliding label in a NON-default scope still moves the
    /// generator. A `(label, scope)` check would not have.
    #[test]
    fn a_collision_in_another_scope_still_counts() {
        let ds = with_blank(&format!("{FRESH_PREFIX}0"), 7);
        let mut fresh = FreshBlanks::avoiding(&[&ds]);
        let TermValue::Blank { label, .. } = fresh.mint() else {
            panic!("a minted node is a blank node");
        };
        assert_ne!(label, format!("{FRESH_PREFIX}0"));
    }

    /// Every observed dataset counts, not only the first — the premise and the conclusion
    /// are separate documents and a witness has to be absent from both.
    #[test]
    fn every_observed_dataset_is_avoided() {
        let premise = with_blank("b", 0);
        let conclusion = with_blank(&format!("{FRESH_PREFIX}0"), 0);
        let mut fresh = FreshBlanks::avoiding(&[&premise, &conclusion]);
        let TermValue::Blank { label, .. } = fresh.mint() else {
            panic!("a minted node is a blank node");
        };
        assert_ne!(label, format!("{FRESH_PREFIX}0"));
    }

    /// A LABEL IS TAKEN WHEREVER IT OCCURS, AND A QUAD IS NOT THE ONLY PLACE.
    ///
    /// This dataset has NO quad. Its three blank-node labels occur only as a reifier, only
    /// as an annotation object, and only as a declared named graph — every one of them a
    /// position [`copy_into`](crate::engine::copy_into) carries and a `quads()`-only survey
    /// cannot see. A survey that missed them would report an empty label set and let
    /// [`FreshBlanks`] mint a "fresh" constant that is one of the caller's own nodes.
    #[test]
    fn a_label_in_a_side_table_or_a_graph_declaration_is_collected() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let triple = b.intern_triple(s, p, o);
        let reifier = b.intern_blank("onlyReifier", BlankScope::DEFAULT);
        b.push_reifier_in_graph(reifier, triple, None);
        let annotated = b.intern_blank("onlyAnnotation", BlankScope::DEFAULT);
        b.push_annotation_in_graph(reifier, p, annotated, None);
        let graph = b.intern_blank("onlyGraph", BlankScope::DEFAULT);
        b.declare_named_graph(graph);
        let ds = b.freeze().expect("freeze");
        assert_eq!(ds.quads().count(), 0, "the fixture asserts no quad at all");

        let labels = super::labels_of(&ds);
        for expected in ["onlyReifier", "onlyAnnotation", "onlyGraph"] {
            assert!(
                labels.contains(expected),
                "{expected} occurs in the dataset and must be collected: {labels:?}"
            );
        }
    }

    /// …and the consequence that makes it a SOUNDNESS fix rather than a tidy-up: a premise
    /// whose reifier already carries the generator's prefix must push the generator off it,
    /// exactly as a colliding quad subject does. Minting that label would have made
    /// `freeze`'s constant one of the premise's own nodes, and the theorem on constants has
    /// its non-occurrence as a hypothesis.
    #[test]
    fn a_colliding_label_in_a_reifier_position_lengthens_the_prefix() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let triple = b.intern_triple(s, p, o);
        let reifier = b.intern_blank(&format!("{FRESH_PREFIX}0"), BlankScope::DEFAULT);
        b.push_reifier_in_graph(reifier, triple, None);
        let ds = b.freeze().expect("freeze");

        let mut fresh = FreshBlanks::avoiding(&[&ds]);
        let TermValue::Blank { label, .. } = fresh.mint() else {
            panic!("a minted node is a blank node");
        };
        assert_ne!(
            label,
            format!("{FRESH_PREFIX}0"),
            "a reifier's label is taken, so the generator must not mint it"
        );
    }

    /// The re-check sees a blank node at any depth, triple terms included.
    #[test]
    fn the_recheck_looks_inside_a_triple_term() {
        let mut labels = BTreeSet::new();
        labels.insert("c".to_owned());
        let nested = TermValue::Triple {
            s: Box::new(TermValue::iri("http://example.org/s")),
            p: Box::new(TermValue::iri("http://example.org/p")),
            o: Box::new(TermValue::blank("c")),
        };
        assert!(mentions_any(&nested, &labels));
        assert!(!mentions_any(&TermValue::blank("d"), &labels));
        assert!(!mentions_any(
            &TermValue::iri("http://example.org/c"),
            &labels
        ));
    }
}

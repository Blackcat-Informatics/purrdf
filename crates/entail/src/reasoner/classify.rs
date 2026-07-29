// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The classifier: the entailed subsumption relation over an ontology's named classes.
//!
//! Two pieces, deliberately separable, because they have different reasons to change and
//! very different costs:
//!
//! * the subsumption MATRIX is the expensive half — one tableau refutation per ORDERED pair of
//!   named classes. It is a matrix of [`Verdict`]s, so an undecided pair stays visibly
//!   undecided instead of collapsing into "not subsumed".
//! * [`ClassHierarchy`] is the cheap half — a pure derivation from that matrix, with no
//!   reasoner in sight: equivalence classes, the unsatisfiable classes, and the transitive
//!   reduction a reader actually wants to look at. It is also what the realizer consumes
//!   to decide which of an individual's entailed types are its MOST SPECIFIC ones, which
//!   is why the matrix is a value rather than a private detail of one function.
//!
//! # Subsumption is decided against the whole knowledge base
//!
//! `KB ⊨ C ⊑ D` exactly when `KB ∪ {x : C ⊓ ¬D}` is inconsistent for a fresh `x`, and the
//! `KB` there includes the ABox. That is not pedantry: with nominals an ASSERTION changes
//! the class hierarchy — `C ≡ {a}` together with `a : D` entails `C ⊑ D`, and a TBox-only
//! test cannot see it. So every refutation here runs with the ABox loaded, and
//! `a_nominal_class_is_subsumed_through_an_assertion` is the fixture that would fail if
//! anyone narrowed it back.
//!
//! # Determinism
//!
//! Classes are visited in ascending interned-term-id order (which is parse order), the
//! matrix is a dense `Vec<Verdict>` indexed by position, and every emitted sequence is
//! sorted by a total, dataset-independent term key. Nothing is read out of a hash map.

use purrdf_core::TermValue;

use super::certificate::{Session, Verdict};
use super::term_key;
use crate::owl_dl::Kb;
use crate::owl_dl::tableau::Assumptions;

/// The entailed subsumption relation over a fixed, ordered list of named classes.
///
/// Row-major and dense: `verdict(sub, sup)` is the answer for the `sub`-th class being
/// subsumed by the `sup`-th, both indices into the `classes` slice the matrix was built
/// over. Undecided pairs are [`Verdict::Unknown`] rather than absent, so a consumer cannot
/// mistake "the budget ran out" for "no".
pub(crate) struct Subsumptions {
    /// The number of classes; the matrix is `n × n`.
    n: usize,
    /// `n × n` verdicts, row-major, indexed `sub * n + sup`.
    verdicts: Vec<Verdict>,
}

impl Subsumptions {
    /// Decide every ordered pair of `classes` (a slice of `(term id, concept id)`).
    ///
    /// Reflexive pairs are not sent to the tableau: `C ⊑ C` holds in every interpretation,
    /// so asking would spend a decision to learn an axiom of the logic.
    pub(crate) fn decide(session: &mut Session<'_>, classes: &[(u32, u32)]) -> Self {
        let n = classes.len();
        let mut verdicts = vec![Verdict::False; n * n];
        for (i, &(_, sub)) in classes.iter().enumerate() {
            for (j, &(_, sup)) in classes.iter().enumerate() {
                verdicts[i * n + j] = if i == j {
                    Verdict::True
                } else {
                    subsumes(session, sub, sup)
                };
            }
        }
        Self { n, verdicts }
    }

    /// The verdict for `classes[sub] ⊑ classes[sup]`.
    pub(crate) fn verdict(&self, sub: usize, sup: usize) -> Verdict {
        self.verdicts[sub * self.n + sup]
    }

    /// Whether `classes[sub] ⊑ classes[sup]` was ESTABLISHED — an undecided pair is not.
    pub(crate) fn holds(&self, sub: usize, sup: usize) -> bool {
        self.verdict(sub, sup).is_true()
    }

    /// Whether the two classes were established equivalent.
    pub(crate) fn equivalent(&self, a: usize, b: usize) -> bool {
        self.holds(a, b) && self.holds(b, a)
    }

    /// The index of the canonical member of `i`'s equivalence class — the lowest index it
    /// was established equivalent to, so a whole equivalence class collapses to one
    /// representative deterministically.
    pub(crate) fn representative(&self, i: usize) -> usize {
        (0..=i).find(|&j| self.equivalent(i, j)).unwrap_or(i)
    }

    /// Whether `classes[sub]` is strictly below `classes[sup]` — subsumed and not
    /// equivalent.
    fn strictly_below(&self, sub: usize, sup: usize) -> bool {
        self.holds(sub, sup) && !self.holds(sup, sub)
    }
}

/// Whether `KB ⊨ sub ⊑ sup`, by refuting `x : sub ⊓ ¬sup` over a fresh anonymous witness.
///
/// The witness is a node of the completion graph with no name at all — not a minted IRI,
/// not even a blank node, because a subsumption question needs an ARBITRARY element rather
/// than a nameable one. See [`super::axiom`] for the case that does need nameable fresh
/// symbols, and for why they are blank nodes.
pub(crate) fn subsumes(session: &mut Session<'_>, sub: u32, sup: u32) -> Verdict {
    let neg_sup = session.kb().table.negate(sup);
    session.refutes(&Assumptions {
        fresh_types: &[sub, neg_sup],
        ..Assumptions::of_kb()
    })
}

/// The classified hierarchy of an ontology's named classes.
///
/// `owl:Thing` and `owl:Nothing` participate: they are read as `⊤` and `⊥` rather than as
/// opaque atomic classes, so `⊥ ⊑ C ⊑ ⊤` appears for every named `C`, and a class the
/// ontology forces empty shows up equivalent to `owl:Nothing` rather than in a separate
/// list nobody joins against.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassHierarchy {
    /// Every ESTABLISHED subsumption `C ⊑ D` with `C` and `D` distinct terms.
    subsumptions: Vec<(TermValue, TermValue)>,
    /// Every established equivalence, each unordered pair once.
    equivalences: Vec<(TermValue, TermValue)>,
    /// Named classes established equivalent to `owl:Nothing`.
    unsatisfiable: Vec<TermValue>,
    /// The transitive reduction of [`Self::subsumptions`].
    direct: Vec<(TermValue, TermValue)>,
}

impl ClassHierarchy {
    /// Every established subsumption `C ⊑ D` between two DISTINCT named class terms,
    /// sorted.
    ///
    /// The full relation, transitively closed — not the reduction. Reflexive pairs are
    /// omitted: `C ⊑ C` is a theorem of the logic rather than a fact about this ontology,
    /// and listing it once per class would bury the ones that are.
    #[must_use]
    pub fn subsumptions(&self) -> &[(TermValue, TermValue)] {
        &self.subsumptions
    }

    /// Every established equivalence `C ≡ D`, each unordered pair listed once with the
    /// lexicographically smaller term first, sorted.
    #[must_use]
    pub fn equivalences(&self) -> &[(TermValue, TermValue)] {
        &self.equivalences
    }

    /// The named classes the ontology forces empty — those established equivalent to
    /// `owl:Nothing`, sorted.
    ///
    /// `owl:Nothing` itself is in the list, because it IS empty; a caller looking for the
    /// ontology's own modelling errors filters it out, and one asking "which of these
    /// classes are empty" gets a correct answer without a special case.
    #[must_use]
    pub fn unsatisfiable(&self) -> &[TermValue] {
        &self.unsatisfiable
    }

    /// The transitive reduction of [`Self::subsumptions`]: `(C, D)` where `D` is a DIRECT
    /// subsumer of `C`, sorted.
    ///
    /// Computed over equivalence-class representatives, so a cycle of mutually subsuming
    /// classes contributes one node rather than an unreadable clique, and only the
    /// representative appears here — the other members are in [`Self::equivalences`].
    ///
    /// # What a `BudgetExhausted` certificate does to this list
    ///
    /// The reduction is derived from the subsumptions that were ESTABLISHED. If the
    /// certificate reports [`DlCompleteness::BudgetExhausted`](super::DlCompleteness), a
    /// pair listed here may have an intermediate class the search did not get to, so
    /// "direct" means "direct as far as this run decided". Every pair listed is still a
    /// genuine subsumption; it is the DIRECTNESS that weakens, and the certificate is
    /// where a caller finds that out.
    #[must_use]
    pub fn direct_subsumptions(&self) -> &[(TermValue, TermValue)] {
        &self.direct
    }

    /// Derive the hierarchy from a decided subsumption matrix.
    pub(crate) fn derive(kb: &Kb, classes: &[(u32, u32)], m: &Subsumptions) -> Self {
        let name = |i: usize| kb.interner.value(classes[i].0).clone();
        let n = classes.len();

        let mut subsumptions = Vec::new();
        let mut equivalences = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if i == j || !m.holds(i, j) {
                    continue;
                }
                subsumptions.push((name(i), name(j)));
                if i < j && m.holds(j, i) {
                    equivalences.push((name(i), name(j)));
                }
            }
        }

        // A class is unsatisfiable exactly when it is subsumed by `owl:Nothing`'s concept,
        // which the signature always carries; if the signature somehow lacks it there is
        // nothing to compare against and the list is empty rather than guessed.
        let bottom = classes.iter().position(|&(_, cid)| cid == kb.bottom);
        let unsatisfiable: Vec<TermValue> = bottom.map_or_else(Vec::new, |b| {
            (0..n).filter(|&i| m.holds(i, b)).map(name).collect()
        });

        // The transitive reduction over equivalence-class representatives.
        let mut direct = Vec::new();
        for i in 0..n {
            if m.representative(i) != i {
                continue;
            }
            for j in 0..n {
                if m.representative(j) != j || !m.strictly_below(i, j) {
                    continue;
                }
                let interposed = (0..n).any(|k| {
                    m.representative(k) == k && m.strictly_below(i, k) && m.strictly_below(k, j)
                });
                if !interposed {
                    direct.push((name(i), name(j)));
                }
            }
        }

        let mut out = Self {
            subsumptions,
            equivalences,
            unsatisfiable,
            direct,
        };
        out.sort();
        out
    }

    /// Put every sequence into the crate's canonical term order.
    fn sort(&mut self) {
        let pair = |(a, b): &(TermValue, TermValue)| (term_key(a), term_key(b));
        self.subsumptions.sort_by_key(pair);
        self.equivalences.sort_by_key(pair);
        self.unsatisfiable.sort_by_key(term_key);
        self.direct.sort_by_key(pair);
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The question side of a match: a triple with variable positions.
//!
//! # One variable space, two kinds of variable
//!
//! A conclusion-directed service is asked two shapes of question and they differ in exactly
//! one respect — whether the caller wants to SEE what a variable was bound to:
//!
//! * a **conclusion graph**, whose blank nodes RDF 1.2 Semantics reads as existentials.
//!   Nothing is projected; the answer is yes or no, and the bindings are the *warrant* for
//!   a yes rather than the answer itself.
//! * a **basic graph pattern**, whose `?v` variables SPARQL projects and whose blank nodes
//!   SPARQL also reads as existentials (non-distinguished variables).
//!
//! So there is one variable space with two inhabitants, [`VarKey`], and one solver over it.
//! Writing two solvers — one that answers a boolean and one that enumerates rows — is how a
//! repository ends up with two blank-node matchers that disagree in a corner; the
//! specialisation is in what the caller reads OUT of the binding, not in how the binding is
//! found.
//!
//! # Blank-node identity carries its scope
//!
//! Two blank nodes with the same label in different scopes are different nodes (`purrdf`
//! C0.2), so a variable's identity is `(label, scope)` and never the bare label. A
//! conclusion parsed from its own document and a premise parsed from another can therefore
//! both use `_:b` without the match conflating them.

use std::collections::BTreeSet;
use std::convert::Infallible;
use std::ops::ControlFlow;

use purrdf_core::{BlankScope, Nested, RdfDataset, TermValue, try_fold_nested, visit_nested};

use crate::entails::graph::{Triple, default_graph_triples};
use crate::owl_dl::query::{QNode, QTriple};

/// A variable position in a pattern.
///
/// Ordered so a solution can live in a `BTreeMap` and be read back in an order that is a
/// function of the question alone. The order is for determinism; it means nothing else.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VarKey {
    /// A blank node, read as an EXISTENTIAL: "there is some term such that …".
    ///
    /// This is what a conclusion graph's blank nodes are under RDF 1.2 Semantics, and what
    /// a basic graph pattern's blank nodes are under SPARQL — a non-distinguished variable
    /// the caller cannot project.
    Blank {
        /// The blank-node label, without the `_:` prefix.
        label: String,
        /// The scope the label is local to (`purrdf` C0.2).
        scope: BlankScope,
    },
    /// A projected (distinguished) query variable, by its name — the part after `?`/`$`.
    Projected(String),
}

/// One position of a pattern triple.
#[derive(Debug, Clone)]
pub(crate) enum Pat {
    /// A term that must be matched exactly.
    Ground(TermValue),
    /// A variable position.
    Var(VarKey),
    /// An RDF 1.2 triple term whose own positions may be variables.
    Triple(Box<[Self; 3]>),
}

/// A `(subject, predicate, object)` pattern triple.
pub(crate) type PatTriple = [Pat; 3];

/// How many variable positions a pattern mentions, counting a triple term's own positions.
///
/// The solver orders patterns by this, most-constrained first: a fully ground pattern
/// either fails outright or fixes nothing, and either way it does so before the search
/// branches.
pub(crate) fn var_count(pat: &Pat) -> usize {
    let mut count = 0;
    let ControlFlow::Continue(()) = pat.visit(|pat| -> ControlFlow<Infallible> {
        count += usize::from(matches!(pat, Pat::Var(_)));
        ControlFlow::Continue(())
    });
    count
}

impl Pat {
    /// Visit every position of this pattern in pre-order over [`visit_nested`]'s work
    /// list — a triple term first, then its subject with everything below it, then its
    /// predicate, then its object. The first `Break` ends the visit and is returned.
    pub(crate) fn visit<'p, B>(
        &'p self,
        mut visit: impl FnMut(&'p Self) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        visit_nested(self, |pat| {
            visit(pat)?;
            ControlFlow::Continue(match pat {
                Self::Triple(inner) => Some([&inner[0], &inner[1], &inner[2]]),
                Self::Ground(_) | Self::Var(_) => None,
            })
        })
    }

    /// Fold this pattern bottom-up over [`try_fold_nested`]'s work list: `leaf`
    /// answers for every position that is not a triple term, and `triple` combines a
    /// triple term's three answers — each folded fully, in order — into its own. The
    /// first error ends the fold.
    ///
    /// # Errors
    ///
    /// The first error `leaf` or `triple` returns.
    pub(crate) fn try_fold<T, E>(
        &self,
        mut leaf: impl FnMut(&Self) -> Result<T, E>,
        mut triple: impl FnMut(T, T, T) -> Result<T, E>,
    ) -> Result<T, E> {
        try_fold_nested(
            self,
            &mut (),
            |(), pat| match pat {
                Self::Triple(inner) => Ok(Nested::Triple(&inner[0], &inner[1], &inner[2])),
                leaf_pat => leaf(leaf_pat).map(Nested::Leaf),
            },
            |(), _, s, p, o| triple(s, p, o),
        )
    }
}

/// Read a term of a conclusion GRAPH as a pattern: every blank node is an existential.
///
/// A triple term is read bottom-up over [`TermValue::try_fold_owned`]'s work list.
pub(crate) fn conclusion_node(term: TermValue) -> Pat {
    let pat = term.try_fold_owned(
        |term| {
            Ok::<_, Infallible>(match term {
                TermValue::Blank { label, scope } => Pat::Var(VarKey::Blank { label, scope }),
                ground => Pat::Ground(ground),
            })
        },
        |s, p, o| Ok(Pat::Triple(Box::new([s, p, o]))),
    );
    match pat {
        Ok(pat) => pat,
    }
}

/// The conclusion graph `ds`, read as patterns.
///
/// This is the zero-projected-variable question: an RDF graph is a conjunction of triples
/// whose blank nodes are existentially quantified, so its patterns mention only
/// [`VarKey::Blank`]. That is not a convention this module imposes — it is what an RDF
/// graph MEANS, and it is why a warrant for a graph entailment is exactly a blank-node
/// mapping.
pub(crate) fn conclusion_patterns(ds: &RdfDataset) -> Vec<PatTriple> {
    default_graph_triples(ds)
        .into_iter()
        .map(|[s, p, o]| [conclusion_node(s), conclusion_node(p), conclusion_node(o)])
        .collect()
}

/// The triples of `triples` whose index is in `keep`, as patterns, in `triples` order.
///
/// The one place a residual becomes a question again. `keep` is an index set rather than a
/// triple list because [`entails`](super::entails) subtracts one lane's discharge from
/// another's obligation, and two lanes can only agree about which triple they mean if they
/// name it by its position in the conclusion's own frozen order.
pub(crate) fn patterns_at(triples: &[Triple], keep: &BTreeSet<usize>) -> Vec<PatTriple> {
    keep.iter()
        .filter_map(|&index| triples.get(index))
        .map(|triple| {
            [
                conclusion_node(triple[0].clone()),
                conclusion_node(triple[1].clone()),
                conclusion_node(triple[2].clone()),
            ]
        })
        .collect()
}

/// Read a basic-graph-pattern node as a pattern position.
///
/// A [`QNode::Triple`] becomes a [`Pat::Triple`] whose own positions are read the same
/// way, so a `?v` inside an RDF 1.2 triple term is the SAME [`VarKey::Projected`] as one
/// outside it: [`try_unify`](super::homomorphism) keys the binding by that name at every
/// depth, so one name is one variable and a pattern that uses it in both places is joined
/// rather than split into two.
///
/// A nested [`QNode::Triple`] is read bottom-up over [`try_fold_nested`]'s work list.
fn bgp_node(node: &QNode) -> Pat {
    let pat = try_fold_nested(
        node,
        &mut (),
        |(), node| {
            Ok::<_, Infallible>(match node {
                QNode::Var(name) => Nested::Leaf(Pat::Var(VarKey::Projected(name.clone()))),
                QNode::Term(term) => Nested::Leaf(conclusion_node(term.clone())),
                QNode::Triple { s, p, o } => Nested::Triple(&**s, &**p, &**o),
            })
        },
        |(), _, s, p, o| Ok(Pat::Triple(Box::new([s, p, o]))),
    );
    match pat {
        Ok(pat) => pat,
    }
}

/// The basic graph pattern `bgp`, read as patterns.
///
/// A `?v` becomes [`VarKey::Projected`] wherever it occurs — inside an RDF 1.2 triple term
/// as well as at top level, which is what [`projected_vars`] walks for; a blank node
/// becomes [`VarKey::Blank`], because SPARQL reads a blank node in a query as a
/// non-distinguished variable — the caller may constrain it but may not see it.
pub(crate) fn bgp_patterns(bgp: &[QTriple]) -> Vec<PatTriple> {
    bgp.iter()
        .map(|triple| {
            [
                bgp_node(&triple.s),
                bgp_node(&triple.p),
                bgp_node(&triple.o),
            ]
        })
        .collect()
}

/// Every projected variable the pattern set mentions, in first-occurrence order.
///
/// First-occurrence order rather than sorted order, because it is the order the caller
/// WROTE and therefore the one that makes a row readable beside the query. It is still a
/// function of the question alone, so it is still deterministic.
pub(crate) fn projected_vars(pats: &[PatTriple]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for triple in pats {
        for position in triple {
            let ControlFlow::Continue(()) = position.visit(|pat| -> ControlFlow<Infallible> {
                if let Pat::Var(VarKey::Projected(name)) = pat
                    && !names.iter().any(|seen| seen == name)
                {
                    names.push(name.clone());
                }
                ControlFlow::Continue(())
            });
        }
    }
    names
}

#[cfg(test)]
pub(crate) mod term_walk_tests {
    //! The pattern readers and walks against their recursive references, and at a
    //! hundred thousand levels on a 128 KiB thread; and the generated patterns the other
    //! entailment walks are tested over.

    use core::convert::Infallible;
    use core::ops::ControlFlow;

    use purrdf_core::TermValue;

    use super::{Pat, QNode, VarKey, bgp_node, conclusion_node, projected_vars, var_count};

    /// A generated basic-graph-pattern node: a generated term whose IRIs
    /// `http://example.org/i1` and `…/i2` stand as the projected variables `?x` and
    /// `?y`, so a pattern mixes ground terms, blank existentials and projected variables
    /// at every depth.
    pub(crate) fn generated_qnode(seed: u64) -> QNode {
        fn to_qnode(value: &TermValue) -> QNode {
            match value {
                TermValue::Triple { s, p, o } => QNode::Triple {
                    s: Box::new(to_qnode(s)),
                    p: Box::new(to_qnode(p)),
                    o: Box::new(to_qnode(o)),
                },
                TermValue::Iri(iri) if iri == "http://example.org/i1" => QNode::Var("x".to_owned()),
                TermValue::Iri(iri) if iri == "http://example.org/i2" => QNode::Var("y".to_owned()),
                other => QNode::Term(other.clone()),
            }
        }
        let mut state = seed;
        let mut budget = 8;
        to_qnode(&purrdf_core::term_fixture::term_value(
            &mut state,
            purrdf_testkit::rng::splitmix64_next,
            &mut budget,
            purrdf_core::term_fixture::TermShape::Any,
        ))
    }

    /// A generated pattern position, read from [`generated_qnode`].
    pub(crate) fn generated_pat(seed: u64) -> Pat {
        bgp_node(&generated_qnode(seed))
    }

    /// Take a pattern chain nested in its object slot apart one level at a time: the
    /// pattern's own drop is derived, and descends once per level.
    pub(crate) fn dismantle(mut pat: Pat) {
        while let Pat::Triple(inner) = pat {
            let [_, _, o] = *inner;
            pat = o;
        }
    }

    /// A pattern chain `levels` deep: `<<( ?x <p> <<( ?x <p> … _:b … )>> )>>`.
    pub(crate) fn pattern_chain(levels: usize) -> Pat {
        let mut pat = Pat::Var(VarKey::Blank {
            label: "b".to_owned(),
            scope: purrdf_core::BlankScope::DEFAULT,
        });
        for _ in 0..levels {
            pat = Pat::Triple(Box::new([
                Pat::Var(VarKey::Projected("x".to_owned())),
                Pat::Ground(TermValue::iri("http://example.org/p")),
                pat,
            ]));
        }
        pat
    }

    fn reference_conclusion(term: TermValue) -> Pat {
        match term {
            TermValue::Blank { label, scope } => Pat::Var(VarKey::Blank { label, scope }),
            TermValue::Triple { s, p, o } => Pat::Triple(Box::new([
                reference_conclusion(s.into_inner()),
                reference_conclusion(p.into_inner()),
                reference_conclusion(o.into_inner()),
            ])),
            ground => Pat::Ground(ground),
        }
    }

    fn reference_bgp(node: &QNode) -> Pat {
        match node {
            QNode::Var(name) => Pat::Var(VarKey::Projected(name.clone())),
            QNode::Term(term) => reference_conclusion(term.clone()),
            QNode::Triple { s, p, o } => Pat::Triple(Box::new([
                reference_bgp(s),
                reference_bgp(p),
                reference_bgp(o),
            ])),
        }
    }

    fn reference_var_count(pat: &Pat) -> usize {
        match pat {
            Pat::Var(_) => 1,
            Pat::Triple(inner) => inner.iter().map(reference_var_count).sum(),
            Pat::Ground(_) => 0,
        }
    }

    fn reference_projected(pat: &Pat, names: &mut Vec<String>) {
        match pat {
            Pat::Var(VarKey::Projected(name)) => {
                if !names.iter().any(|seen| seen == name) {
                    names.push(name.clone());
                }
            }
            Pat::Triple(inner) => {
                for position in &**inner {
                    reference_projected(position, names);
                }
            }
            Pat::Var(VarKey::Blank { .. }) | Pat::Ground(_) => {}
        }
    }

    /// Every pattern reader and walk answers every generated node exactly as its
    /// recursive reference does.
    #[test]
    fn the_pattern_walks_agree_with_their_recursive_references_on_generated_nodes() {
        let mut nested = 0;
        for seed in 0..400_u64 {
            let node = generated_qnode(seed);
            nested += usize::from(matches!(&node, QNode::Triple { .. }));
            let pat = bgp_node(&node);
            assert_eq!(
                format!("{pat:?}"),
                format!("{:?}", reference_bgp(&node)),
                "seed {seed}"
            );
            if let QNode::Term(term) = &node {
                assert_eq!(
                    format!("{:?}", conclusion_node(term.clone())),
                    format!("{:?}", reference_conclusion(term.clone())),
                    "seed {seed}"
                );
            }
            assert_eq!(var_count(&pat), reference_var_count(&pat), "seed {seed}");
            let triple = [pat.clone(), generated_pat(seed + 1), pat];
            let mut expected = Vec::new();
            for position in &triple {
                reference_projected(position, &mut expected);
            }
            assert_eq!(projected_vars(&[triple]), expected, "seed {seed}");
        }
        assert!(nested > 0, "some generated node is a triple term");
    }

    /// A pattern a hundred thousand levels deep is read, counted, visited and folded on
    /// a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_pattern_is_walked_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let pat = conclusion_node(purrdf_core::term_fixture::triple_chain(LEVELS));
                assert_eq!(var_count(&pat), 0, "a chain of IRIs has no blank node");
                dismantle(pat);
                let pat = pattern_chain(LEVELS);
                assert_eq!(var_count(&pat), LEVELS + 1);
                // The pattern's derived copy descends once per level, so the chain is
                // moved into the triple and back out rather than copied.
                let triples = [[pat, pattern_chain(0), pattern_chain(0)]];
                assert_eq!(projected_vars(&triples), ["x"]);
                let [[pat, _, _]] = triples;
                let mut visited = 0_usize;
                let ControlFlow::Continue(()) = pat.visit(|_| -> ControlFlow<Infallible> {
                    visited += 1;
                    ControlFlow::Continue(())
                });
                // Each level is its triple term, its variable subject and its ground
                // predicate — its object is the next level — and the innermost object is
                // the one blank-node variable.
                assert_eq!(visited, 3 * LEVELS + 1);
                let depth = pat.try_fold(
                    |_| Ok::<_, Infallible>(0_usize),
                    |s, p, o| Ok(1 + s.max(p).max(o)),
                );
                assert_eq!(depth, Ok(LEVELS));
                dismantle(pat);
            })
            .expect("the thread starts")
            .join()
            .expect("no walk overflowed the thread's stack");
    }
}

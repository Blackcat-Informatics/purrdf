// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use purrdf_core::{BlankScope, RdfDataset, TermValue};

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
    match pat {
        Pat::Var(_) => 1,
        Pat::Triple(inner) => inner.iter().map(var_count).sum(),
        Pat::Ground(_) => 0,
    }
}

/// Read a term of a conclusion GRAPH as a pattern: every blank node is an existential.
pub(crate) fn conclusion_node(term: TermValue) -> Pat {
    match term {
        TermValue::Blank { label, scope } => Pat::Var(VarKey::Blank { label, scope }),
        TermValue::Triple { s, p, o } => Pat::Triple(Box::new([
            conclusion_node(*s),
            conclusion_node(*p),
            conclusion_node(*o),
        ])),
        ground => Pat::Ground(ground),
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
fn bgp_node(node: &QNode) -> Pat {
    match node {
        QNode::Var(name) => Pat::Var(VarKey::Projected(name.clone())),
        QNode::Term(term) => conclusion_node(term.clone()),
    }
}

/// The basic graph pattern `bgp`, read as patterns.
///
/// A `?v` becomes [`VarKey::Projected`]; a blank node becomes [`VarKey::Blank`], because
/// SPARQL reads a blank node in a query as a non-distinguished variable — the caller may
/// constrain it but may not see it.
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
    fn walk(pat: &Pat, names: &mut Vec<String>) {
        match pat {
            Pat::Var(VarKey::Projected(name)) => {
                if !names.iter().any(|seen| seen == name) {
                    names.push(name.clone());
                }
            }
            Pat::Triple(inner) => {
                for position in inner.iter() {
                    walk(position, names);
                }
            }
            Pat::Var(VarKey::Blank { .. }) | Pat::Ground(_) => {}
        }
    }
    for triple in pats {
        for position in triple {
            walk(position, &mut names);
        }
    }
    names
}

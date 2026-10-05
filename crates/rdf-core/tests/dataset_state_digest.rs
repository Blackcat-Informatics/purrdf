// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete-state identity witnesses and an independent finite-bijection oracle.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, CanonHash, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId, canonicalize,
    graph_digest_view, try_flat_digest_view,
};
use purrdf_iri::vocab::rdf::REIFIES;

const P: &str = "http://example.org/p";
const Q: &str = "http://example.org/q";
const G: &str = "http://example.org/g";
const H: &str = "http://example.org/h";

/// A small fixture grammar, independent of production canonicalization.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Iri(&'static str),
    Blank(u8),
    Triple(Box<[Self; 3]>),
    Composite {
        chunks: Vec<&'static str>,
        blanks: Vec<u8>,
    },
}

impl Node {
    fn blanks(&self, out: &mut BTreeSet<u8>) {
        match self {
            Self::Iri(_) => {}
            Self::Blank(id) => {
                out.insert(*id);
            }
            Self::Triple(parts) => {
                for part in parts.as_ref() {
                    part.blanks(out);
                }
            }
            Self::Composite { blanks, .. } => out.extend(blanks),
        }
    }

    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        let mapped = |a, b| map.iter().any(|&(left, right)| left == a && right == b);
        match (self, other) {
            (Self::Iri(a), Self::Iri(b)) => a == b,
            (Self::Blank(a), Self::Blank(b)) => mapped(*a, *b),
            (Self::Triple(a), Self::Triple(b)) => {
                a.iter().zip(b.iter()).all(|(a, b)| a.agrees(b, map))
            }
            (
                Self::Composite {
                    chunks: a,
                    blanks: x,
                },
                Self::Composite {
                    chunks: b,
                    blanks: y,
                },
            ) => a == b && x.len() == y.len() && x.iter().zip(y).all(|(&a, &b)| mapped(a, b)),
            _ => false,
        }
    }

    fn intern(&self, builder: &mut RdfDatasetBuilder) -> TermId {
        match self {
            Self::Iri(iri) => builder.intern_iri(iri),
            Self::Blank(id) => builder.intern_blank(&format!("n{id}"), BlankScope::DEFAULT),
            Self::Triple(parts) => {
                let [s, p, o] = parts.each_ref().map(|part| part.intern(builder));
                builder.intern_triple(s, p, o)
            }
            Self::Composite { chunks, blanks } => {
                assert_eq!(chunks.len(), blanks.len() + 1);
                let mut lexical = chunks[0].to_owned();
                for (&blank, chunk) in blanks.iter().zip(&chunks[1..]) {
                    write!(lexical, "_:n{blank}").expect("writing to a string cannot fail");
                    lexical.push_str(chunk);
                }
                builder.intern_literal(RdfLiteral::typed(lexical, purrdf_cdt::CDT_LIST))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Ordinary,
    Reifier,
    Annotation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Row {
    role: Role,
    terms: [Node; 3],
    graph: Option<Node>,
}

impl Row {
    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        self.role == other.role
            && self
                .terms
                .iter()
                .zip(&other.terms)
                .all(|(a, b)| a.agrees(b, map))
            && match (&self.graph, &other.graph) {
                (Some(a), Some(b)) => a.agrees(b, map),
                (None, None) => true,
                _ => false,
            }
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    graphs: Vec<Node>,
    rows: Vec<Row>,
}

impl State {
    fn blanks(&self) -> Vec<u8> {
        let mut nodes = BTreeSet::new();
        for graph in &self.graphs {
            graph.blanks(&mut nodes);
        }
        for row in &self.rows {
            for term in &row.terms {
                term.blanks(&mut nodes);
            }
            if let Some(graph) = &row.graph {
                graph.blanks(&mut nodes);
            }
        }
        nodes.into_iter().collect()
    }

    fn agrees(&self, other: &Self, map: &[(u8, u8)]) -> bool {
        self.graphs.len() == other.graphs.len()
            && self.rows.len() == other.rows.len()
            && self
                .graphs
                .iter()
                .all(|a| other.graphs.iter().any(|b| a.agrees(b, map)))
            && self
                .rows
                .iter()
                .all(|a| other.rows.iter().any(|b| a.agrees(b, map)))
    }

    fn materialize(&self) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        for graph in &self.graphs {
            let graph = graph.intern(&mut builder);
            builder.declare_named_graph(graph);
        }
        for row in &self.rows {
            let [s, p, o] = row.terms.each_ref().map(|term| term.intern(&mut builder));
            let graph = row.graph.as_ref().map(|graph| graph.intern(&mut builder));
            match row.role {
                Role::Ordinary => builder.push_quad(s, p, o, graph),
                Role::Reifier => {
                    assert_eq!(row.terms[1], Node::Iri(REIFIES));
                    builder.push_reifier_in_graph(s, o, graph);
                }
                Role::Annotation => builder.push_annotation_in_graph(s, p, o, graph),
            }
        }
        builder
            .freeze()
            .expect("the finite fixture is structurally valid")
    }
}

/// Enumerate whole-state bijections, without any production labeling or codec.
fn isomorphic(a: &State, b: &State) -> bool {
    fn search(a: &State, b: &State, left: &[u8], right: &[u8], map: &mut Vec<(u8, u8)>) -> bool {
        let Some((&head, tail)) = left.split_first() else {
            return a.agrees(b, map);
        };
        for (index, &candidate) in right.iter().enumerate() {
            map.push((head, candidate));
            let mut remaining = right.to_vec();
            remaining.remove(index);
            let matched = search(a, b, tail, &remaining, map);
            map.pop();
            if matched {
                return true;
            }
        }
        false
    }
    let left = a.blanks();
    let right = b.blanks();
    left.len() == right.len() && search(a, b, &left, &right, &mut Vec::new())
}

fn role_fixture(role: Role) -> State {
    let quoted = Node::Triple(Box::new([Node::Iri(G), Node::Iri(P), Node::Iri(H)]));
    let binding = Row {
        role: Role::Reifier,
        terms: [Node::Blank(0), Node::Iri(REIFIES), quoted],
        graph: None,
    };
    State {
        graphs: Vec::new(),
        rows: vec![
            binding,
            Row {
                role,
                terms: [Node::Blank(0), Node::Iri(Q), Node::Iri(H)],
                graph: None,
            },
        ],
    }
}

fn crossing_fixture(crossed: bool) -> State {
    State {
        graphs: vec![Node::Iri(G), Node::Iri(H)],
        rows: [(P, G, 0, 1), (Q, H, u8::from(crossed), u8::from(!crossed))]
            .into_iter()
            .map(|(predicate, graph, s, o)| Row {
                role: Role::Ordinary,
                terms: [Node::Blank(s), Node::Iri(predicate), Node::Blank(o)],
                graph: Some(Node::Iri(graph)),
            })
            .collect(),
    }
}

fn legacy_canonical_form_omits_an_empty_named_declaration() {
    let a = State::default();
    let b = State {
        graphs: vec![Node::Iri(G)],
        rows: Vec::new(),
    };
    assert!(!isomorphic(&a, &b));
    assert_eq!(
        canonicalize(&a.materialize()).nquads,
        canonicalize(&b.materialize()).nquads
    );
}

fn legacy_flat_digest_erases_ordinary_annotation_roles() {
    let a = role_fixture(Role::Ordinary);
    let b = role_fixture(Role::Annotation);
    assert!(!isomorphic(&a, &b));
    assert_eq!(
        try_flat_digest_view(&*a.materialize(), CanonHash::Sha256).unwrap(),
        try_flat_digest_view(&*b.materialize(), CanonHash::Sha256).unwrap(),
    );
}

fn legacy_flat_digest_erases_ordinary_reifier_roles() {
    let mut a = role_fixture(Role::Ordinary);
    a.rows.pop();
    let mut b = a.clone();
    b.rows[0].role = Role::Ordinary;
    assert!(!isomorphic(&a, &b));
    assert_eq!(
        try_flat_digest_view(&*a.materialize(), CanonHash::Sha256).unwrap(),
        try_flat_digest_view(&*b.materialize(), CanonHash::Sha256).unwrap(),
    );
}

fn per_graph_digests_do_not_enforce_a_global_bijection() {
    let a = crossing_fixture(false);
    let b = crossing_fixture(true);
    assert!(!isomorphic(&a, &b));
    for graph in [G, H] {
        assert_eq!(
            graph_digest_view(&*a.materialize(), graph),
            graph_digest_view(&*b.materialize(), graph)
        );
    }
    for kept in 0..2 {
        let mut left = a.clone();
        let mut right = b.clone();
        left.rows = vec![left.rows.remove(kept)];
        right.rows = vec![right.rows.remove(kept)];
        assert!(
            isomorphic(&left, &right),
            "either row's removal eliminates the witness"
        );
    }
}

fn oracle_has_one_mapping_across_graphs_nested_and_composite_terms() {
    let make = |inside| State {
        graphs: vec![Node::Blank(0)],
        rows: vec![Row {
            role: Role::Ordinary,
            terms: [
                Node::Blank(0),
                Node::Iri(Q),
                Node::Triple(Box::new([
                    Node::Blank(1),
                    Node::Iri(P),
                    Node::Composite {
                        chunks: vec!["[", ", ", "]"],
                        blanks: vec![inside, 1],
                    },
                ])),
            ],
            graph: Some(Node::Blank(0)),
        }],
    };
    let a = make(0);
    assert!(isomorphic(&a, &a));
    assert!(!isomorphic(&a, &make(1)));
    assert!(a.materialize().quad_count() > 0);
}

fn cycle_fixture(split: bool, names: [u8; 6]) -> State {
    State {
        graphs: vec![Node::Iri(G)],
        rows: (0..6)
            .map(|i| {
                let next = if split {
                    (i / 3) * 3 + (i + 1) % 3
                } else {
                    (i + 1) % 6
                };
                Row {
                    role: Role::Ordinary,
                    terms: [
                        Node::Blank(names[i]),
                        Node::Iri(P),
                        Node::Blank(names[next]),
                    ],
                    graph: Some(Node::Iri(G)),
                }
            })
            .collect(),
    }
}

fn oracle_distinguishes_same_color_connected_and_disconnected_cycles() {
    let a = cycle_fixture(false, [0, 1, 2, 3, 4, 5]);
    let b = cycle_fixture(true, [0, 1, 2, 3, 4, 5]);
    assert!(!isomorphic(&a, &b));
    assert_eq!(a.materialize().quad_count(), 6);
    assert_eq!(b.materialize().quad_count(), 6);
}

fn oracle_accepts_global_relabeling_and_insertion_permutations() {
    let a = cycle_fixture(false, [0, 1, 2, 3, 4, 5]);
    let mut b = cycle_fixture(false, [9, 3, 7, 2, 8, 1]);
    b.rows.reverse();
    assert!(isomorphic(&a, &b));
    assert_eq!(
        canonicalize(&a.materialize()).nquads,
        canonicalize(&b.materialize()).nquads
    );
}

purrdf_testkit::harness_main!(
    legacy_canonical_form_omits_an_empty_named_declaration,
    legacy_flat_digest_erases_ordinary_annotation_roles,
    legacy_flat_digest_erases_ordinary_reifier_roles,
    per_graph_digests_do_not_enforce_a_global_bijection,
    oracle_has_one_mapping_across_graphs_nested_and_composite_terms,
    oracle_distinguishes_same_color_connected_and_disconnected_cycles,
    oracle_accepts_global_relabeling_and_insertion_permutations,
);

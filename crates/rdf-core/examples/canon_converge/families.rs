// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic generators for the blank-node shapes the labelling is measured on.
//!
//! Every family is a [`Fold`]: ordinary default-graph quads over numbered blank
//! nodes and invented `http://example.org/` IRIs, so one description feeds the
//! present search (through an `RdfDataset`) and the colour labelling (through its
//! line list). Nothing here reads a clock or a random source.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermId};

/// A term of a family: a numbered blank node, an IRI name or a plain literal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    Blank(u32),
    Iri(String),
    Lit(String),
}

/// A set of default-graph statements `(subject, predicate-name, object)`.
#[derive(Debug, Default)]
pub(crate) struct Fold {
    pub(crate) blanks: u32,
    pub(crate) quads: Vec<(Node, String, Node)>,
}

impl Fold {
    fn blank(&mut self) -> u32 {
        self.blanks += 1;
        self.blanks - 1
    }

    fn edge(&mut self, s: u32, p: &str, o: u32) {
        self.quads
            .push((Node::Blank(s), p.to_owned(), Node::Blank(o)));
    }

    fn typed(&mut self, s: u32) {
        self.quads.push((
            Node::Blank(s),
            "type".to_owned(),
            Node::Iri("Node".to_owned()),
        ));
    }

    fn undirected(&mut self, a: u32, b: u32) {
        self.edge(a, "e", b);
        self.edge(b, "e", a);
    }

    /// The dataset holding this fold, blank nodes labelled `b<number>`.
    pub(crate) fn dataset(&self) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let mut blanks: Vec<Option<TermId>> = vec![None; self.blanks as usize];
        let mut term = |b: &mut RdfDatasetBuilder, n: &Node| match n {
            Node::Blank(i) => *blanks[*i as usize].get_or_insert_with(|| {
                b.intern_blank(&format!("b{i}"), BlankScope::DEFAULT)
                    .expect("a blank label interns")
            }),
            Node::Iri(name) => b
                .intern_iri(&format!("http://example.org/{name}"))
                .expect("an example IRI interns"),
            Node::Lit(text) => b
                .intern_literal(RdfLiteral::simple(text.clone()))
                .expect("a simple literal interns"),
        };
        for (s, p, o) in &self.quads {
            let s = term(&mut b, s);
            let p = b
                .intern_iri(&format!("http://example.org/{p}"))
                .expect("an example IRI interns");
            let o = term(&mut b, o);
            b.push_quad(s, p, o, None).expect("a quad pushes");
        }
        b.freeze().expect("the fold freezes")
    }
}

/// A chain of `len` typed nodes hanging below `parent`; the last node's index.
fn chain(fold: &mut Fold, parent: u32, len: u32) {
    let mut at = parent;
    for _ in 0..len {
        let next = fold.blank();
        fold.typed(next);
        fold.edge(at, "child", next);
        at = next;
    }
}

/// Hubs below spines. The root has `hubs` branches; branch `j` is a spine of
/// `depth + j` typed nodes ending in a hub with `fan` children. With `distinct`
/// the child `i` heads a chain of `i + 1` typed nodes (siblings differ only in
/// their subtrees); without it every child is a single leaf (identical
/// siblings). Branch lengths differ, so the fold has no automorphism that
/// exchanges two hubs, yet all hubs share one first-degree hash when `hubs >= 2`;
/// with `hubs == 1` the hub is first-degree unique.
pub(crate) fn hubs(fan: u32, depth: u32, hubs: u32, distinct: bool) -> Fold {
    let mut fold = Fold::default();
    let root = fold.blank();
    fold.typed(root);
    for j in 0..hubs {
        let mut at = root;
        for _ in 0..depth + j {
            let next = fold.blank();
            fold.typed(next);
            fold.edge(at, "child", next);
            at = next;
        }
        for i in 0..fan {
            chain(&mut fold, at, if distinct { i + 1 } else { 1 });
        }
    }
    fold
}

/// A complete `fan`-ary tree of `depth` levels below a typed root: every
/// sibling is isomorphic to every other.
pub(crate) fn tree(fan: u32, depth: u32) -> Fold {
    let mut fold = Fold::default();
    let root = fold.blank();
    fold.typed(root);
    let mut level = vec![root];
    for _ in 0..depth {
        let mut next_level = Vec::new();
        for &parent in &level {
            for _ in 0..fan {
                let c = fold.blank();
                fold.typed(c);
                fold.edge(parent, "child", c);
                next_level.push(c);
            }
        }
        level = next_level;
    }
    fold
}

/// `n` blank nodes in a directed path, both ends linked to the same IRI (so the
/// path is mirror-symmetric).
pub(crate) fn path(n: u32) -> Fold {
    let mut fold = Fold::default();
    let nodes: Vec<u32> = (0..n).map(|_| fold.blank()).collect();
    let end = Node::Iri("end".to_owned());
    fold.quads
        .push((end.clone(), "link".to_owned(), Node::Blank(nodes[0])));
    for pair in nodes.windows(2) {
        fold.edge(pair[0], "next", pair[1]);
    }
    fold.quads
        .push((Node::Blank(nodes[n as usize - 1]), "link".to_owned(), end));
    fold
}

/// An RDF-list-shaped chain of `n` blank cells. With `distinct` every cell has
/// its own item; otherwise all cells hold the same item.
pub(crate) fn list(n: u32, distinct: bool) -> Fold {
    let mut fold = Fold::default();
    let cells: Vec<u32> = (0..n).map(|_| fold.blank()).collect();
    fold.quads.push((
        Node::Iri("owner".to_owned()),
        "items".to_owned(),
        Node::Blank(cells[0]),
    ));
    for (i, &cell) in cells.iter().enumerate() {
        let item = if distinct {
            Node::Lit(format!("item {i}"))
        } else {
            Node::Lit("item".to_owned())
        };
        fold.quads
            .push((Node::Blank(cell), "first".to_owned(), item));
        match cells.get(i + 1) {
            Some(&next) => fold.edge(cell, "rest", next),
            None => fold.quads.push((
                Node::Blank(cell),
                "rest".to_owned(),
                Node::Iri("nil".to_owned()),
            )),
        }
    }
    fold
}

/// The complete bipartite graph on `n + n` blank nodes. Symmetric: every pair
/// is joined in both directions; otherwise a left node points to every right
/// node (the left nodes are then twins).
pub(crate) fn bipartite(n: u32, symmetric: bool) -> Fold {
    let mut fold = Fold::default();
    let left: Vec<u32> = (0..n).map(|_| fold.blank()).collect();
    let right: Vec<u32> = (0..n).map(|_| fold.blank()).collect();
    for &a in &left {
        for &b in &right {
            if symmetric {
                fold.undirected(a, b);
            } else {
                fold.edge(a, "e", b);
            }
        }
    }
    fold
}

/// The `n` x `n` rook graph: cells joined when they share a row or a column.
pub(crate) fn rook(n: u32) -> Fold {
    let mut fold = Fold::default();
    for _ in 0..n * n {
        fold.blank();
    }
    for a in 0..n * n {
        for b in a + 1..n * n {
            if a / n == b / n || a % n == b % n {
                fold.undirected(a, b);
            }
        }
    }
    fold
}

/// The hypercube `Q_q` on `2^q` blank nodes.
pub(crate) fn cube(q: u32) -> Fold {
    let mut fold = Fold::default();
    for _ in 0..1u32 << q {
        fold.blank();
    }
    for a in 0..1u32 << q {
        for bit in 0..q {
            let b = a ^ (1 << bit);
            if a < b {
                fold.undirected(a, b);
            }
        }
    }
    fold
}

/// A fold read from a file of N-Quads lines over blank nodes, IRIs and plain
/// literals in the default graph (the reproducer's shape). Predicates are kept
/// as the full IRI text; the caller's `example.org` prefix is stripped so the
/// fold builds the same IRIs back.
pub(crate) fn from_nquads(text: &str) -> Fold {
    let mut fold = Fold::default();
    let mut labels: Vec<String> = Vec::new();
    let mut node = |token: &str, fold: &mut Fold| -> Node {
        if let Some(label) = token.strip_prefix("_:") {
            let index = labels.iter().position(|l| l == label).unwrap_or_else(|| {
                labels.push(label.to_owned());
                fold.blanks += 1;
                labels.len() - 1
            });
            Node::Blank(index as u32)
        } else if let Some(iri) = token.strip_prefix('<') {
            let iri = iri.trim_end_matches('>');
            Node::Iri(
                iri.strip_prefix("http://example.org/")
                    .expect("an example.org IRI")
                    .to_owned(),
            )
        } else {
            Node::Lit(token.trim_matches('"').to_owned())
        }
    };
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let line = line.trim_end_matches(" .");
        let mut parts = line.splitn(3, ' ');
        let (s, p, o) = (
            parts.next().expect("subject"),
            parts.next().expect("predicate"),
            parts.next().expect("object"),
        );
        let s = node(s, &mut fold);
        let o = node(o, &mut fold);
        let p = p
            .trim_matches(['<', '>'])
            .strip_prefix("http://example.org/")
            .expect("an example.org predicate")
            .to_owned();
        fold.quads.push((s, p, o));
    }
    fold
}

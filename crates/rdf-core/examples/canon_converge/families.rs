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

/// A seeded xorshift-multiply generator for the random families.
struct Draw(u64);

impl Draw {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i as u64 + 1) as usize);
        }
    }
}

/// An undirected simple graph on `n` blank nodes, each edge written both ways
/// with one predicate.
fn graph(n: u32, edges: impl IntoIterator<Item = (u32, u32)>) -> Fold {
    let mut fold = Fold::default();
    for _ in 0..n {
        fold.blank();
    }
    for (a, b) in edges {
        fold.undirected(a, b);
    }
    fold
}

/// `k` disjoint directed triangles.
pub(crate) fn triangles(k: u32) -> Fold {
    let mut fold = Fold::default();
    for _ in 0..k {
        let (a, b, c) = (fold.blank(), fold.blank(), fold.blank());
        fold.edge(a, "p", b);
        fold.edge(b, "p", c);
        fold.edge(c, "p", a);
    }
    fold
}

/// A directed cycle of `n` blank nodes.
pub(crate) fn cycle(n: u32) -> Fold {
    let mut fold = Fold::default();
    for _ in 0..n {
        fold.blank();
    }
    for i in 0..n {
        fold.edge(i, "p", (i + 1) % n);
    }
    fold
}

/// The Paley graph of a prime `q` congruent to 1 modulo 4.
pub(crate) fn paley(q: u32) -> Fold {
    let squares: Vec<bool> = {
        let mut s = vec![false; q as usize];
        for x in 1..q {
            s[((x * x) % q) as usize] = true;
        }
        s
    };
    graph(
        q,
        (0..q)
            .flat_map(|a| (a + 1..q).map(move |b| (a, b)))
            .filter(|&(a, b)| squares[(b - a) as usize]),
    )
}

/// A Latin square of order `n` from the Jacobson-Matthews Markov chain started
/// at the cyclic square: `10 n^3` moves from `seed`, then moves until the
/// square is proper.
fn latin_square(n: usize, seed: u64) -> Vec<Vec<usize>> {
    let mut draw = Draw::new(seed);
    let at = |r: usize, c: usize, s: usize| (r * n + c) * n + s;
    let mut m = vec![0i8; n * n * n];
    for r in 0..n {
        for c in 0..n {
            m[at(r, c, (r + c) % n)] = 1;
        }
    }
    // The cell holding -1 when improper.
    let mut bad: Option<(usize, usize, usize)> = None;
    let steps = 10 * n * n * n;
    let mut step = 0;
    while step < steps || bad.is_some() {
        step += 1;
        let pick = |draw: &mut Draw, list: Vec<usize>| list[draw.below(list.len() as u64) as usize];
        let (r, c, s) = if let Some(cell) = bad {
            cell
        } else {
            loop {
                let (r, c, s) = (
                    draw.below(n as u64) as usize,
                    draw.below(n as u64) as usize,
                    draw.below(n as u64) as usize,
                );
                if m[at(r, c, s)] == 0 {
                    break (r, c, s);
                }
            }
        };
        let r2 = pick(&mut draw, (0..n).filter(|&x| m[at(x, c, s)] == 1).collect());
        let c2 = pick(&mut draw, (0..n).filter(|&x| m[at(r, x, s)] == 1).collect());
        let s2 = pick(&mut draw, (0..n).filter(|&x| m[at(r, c, x)] == 1).collect());
        m[at(r, c, s)] += 1;
        m[at(r, c2, s2)] += 1;
        m[at(r2, c, s2)] += 1;
        m[at(r2, c2, s)] += 1;
        m[at(r, c, s2)] -= 1;
        m[at(r, c2, s)] -= 1;
        m[at(r2, c, s)] -= 1;
        m[at(r2, c2, s2)] -= 1;
        bad = (m[at(r2, c2, s2)] < 0).then_some((r2, c2, s2));
    }
    (0..n)
        .map(|r| {
            (0..n)
                .map(|c| {
                    (0..n)
                        .find(|&s| m[at(r, c, s)] == 1)
                        .expect("a proper square")
                })
                .collect()
        })
        .collect()
}

/// The Latin-square graph of a Jacobson-Matthews Latin square of order `n`: the
/// `n * n` cells, joined when they share a row, a column or a symbol.
pub(crate) fn latin(n: u32, seed: u64) -> Fold {
    let sq = latin_square(n as usize, seed);
    let n = n as usize;
    let edges: Vec<(u32, u32)> = (0..n * n)
        .flat_map(|a| (a + 1..n * n).map(move |b| (a, b)))
        .filter(|&(a, b)| a / n == b / n || a % n == b % n || sq[a / n][a % n] == sq[b / n][b % n])
        .map(|(a, b)| (a as u32, b as u32))
        .collect();
    graph((n * n) as u32, edges)
}

/// A seeded Steiner triple system on `v` points (hill-climbing: add a triple
/// on two uncovered pairs of a live point, displacing the block that covered
/// the third pair).
fn steiner(v: usize, seed: u64) -> Vec<[usize; 3]> {
    const NONE: usize = usize::MAX;
    let mut draw = Draw::new(seed);
    let mut third = vec![vec![NONE; v]; v];
    let target = v * (v - 1) / 6;
    let mut blocks = 0;
    while blocks < target {
        let live: Vec<usize> = (0..v)
            .filter(|&x| (0..v).any(|y| y != x && third[x][y] == NONE))
            .collect();
        let x = live[draw.below(live.len() as u64) as usize];
        let partners: Vec<usize> = (0..v).filter(|&y| y != x && third[x][y] == NONE).collect();
        if partners.len() < 2 {
            continue;
        }
        let y = partners[draw.below(partners.len() as u64) as usize];
        let z = loop {
            let z = partners[draw.below(partners.len() as u64) as usize];
            if z != y {
                break z;
            }
        };
        let w = third[y][z];
        if w == NONE {
            blocks += 1;
        } else {
            for (a, b) in [(w, y), (w, z), (y, z)] {
                third[a][b] = NONE;
                third[b][a] = NONE;
            }
        }
        for (a, b, c) in [(x, y, z), (y, z, x), (x, z, y)] {
            third[a][b] = c;
            third[b][a] = c;
        }
    }
    let mut out = Vec::with_capacity(target);
    for (a, row) in third.iter().enumerate() {
        for (b, &c) in row.iter().enumerate().skip(a + 1) {
            if c != NONE && c > b {
                out.push([a, b, c]);
            }
        }
    }
    out
}

/// The block graph of a seeded Steiner triple system on `v` points: blocks
/// joined when they share a point.
pub(crate) fn sts(v: u32, seed: u64) -> Fold {
    let blocks = steiner(v as usize, seed);
    let k = blocks.len();
    let edges: Vec<(u32, u32)> = (0..k)
        .flat_map(|a| (a + 1..k).map(move |b| (a, b)))
        .filter(|&(a, b)| blocks[a].iter().any(|p| blocks[b].contains(p)))
        .map(|(a, b)| (a as u32, b as u32))
        .collect();
    graph(k as u32, edges)
}

/// The edges of a seeded random cubic simple graph on `n` (even) nodes.
fn cubic_edges(n: u32, seed: u64) -> Vec<(u32, u32)> {
    let mut draw = Draw::new(seed);
    loop {
        let mut points: Vec<u32> = (0..3 * n).map(|i| i / 3).collect();
        draw.shuffle(&mut points);
        let mut edges: Vec<(u32, u32)> = points
            .chunks(2)
            .map(|p| (p[0].min(p[1]), p[0].max(p[1])))
            .collect();
        edges.sort_unstable();
        let simple = edges.iter().all(|e| e.0 != e.1) && edges.windows(2).all(|w| w[0] != w[1]);
        if simple {
            return edges;
        }
    }
}

/// A seeded random cubic graph on `n` blank nodes.
pub(crate) fn cubic(n: u32, seed: u64) -> Fold {
    graph(n, cubic_edges(n, seed))
}

/// The Cai-Furer-Immerman graph over a cubic base: `K4` (4), the cube (8), the
/// Petersen graph (10), otherwise a seeded random cubic graph on `n` nodes.
/// Each base vertex has four middle nodes (even subsets of its three edges) and
/// two end nodes per edge; ends of one edge are joined across, crossed on the
/// first edge when `twisted`.
pub(crate) fn cfi(n: u32, seed: u64, twisted: bool) -> Fold {
    let base: Vec<(u32, u32)> = match n {
        4 => vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
        8 => (0..8u32)
            .flat_map(|a| (0..3).map(move |b| (a, a ^ (1 << b))))
            .filter(|&(a, b)| a < b)
            .collect(),
        10 => vec![
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (0, 4),
            (0, 5),
            (1, 6),
            (2, 7),
            (3, 8),
            (4, 9),
            (5, 7),
            (7, 9),
            (6, 9),
            (6, 8),
            (5, 8),
        ],
        _ => cubic_edges(n, seed),
    };
    // incident edges per vertex, in order
    let mut inc: Vec<Vec<usize>> = vec![Vec::new(); n as usize];
    for (e, &(a, b)) in base.iter().enumerate() {
        inc[a as usize].push(e);
        inc[b as usize].push(e);
    }
    let mut fold = Fold::default();
    // end node (vertex, local edge index, bit)
    let mut end = vec![[[0u32; 2]; 3]; n as usize];
    for ends in &mut end {
        for slot in ends.iter_mut() {
            *slot = [fold.blank(), fold.blank()];
        }
        for subset in 0..8u32 {
            if subset.count_ones() % 2 != 0 {
                continue;
            }
            let m = fold.blank();
            for (j, slot) in ends.iter().enumerate() {
                let bit = (subset >> j) & 1;
                fold.undirected(m, slot[bit as usize]);
            }
        }
    }
    for (e, &(a, b)) in base.iter().enumerate() {
        let ja = inc[a as usize]
            .iter()
            .position(|&x| x == e)
            .expect("incident");
        let jb = inc[b as usize]
            .iter()
            .position(|&x| x == e)
            .expect("incident");
        for bit in 0..2 {
            let other = if twisted && e == 0 { 1 - bit } else { bit };
            fold.undirected(end[a as usize][ja][bit], end[b as usize][jb][other]);
        }
    }
    fold
}

/// A claim dataset: one named graph holding one blank reifier of one triple
/// term (whose object is a literal of `literal_len` bytes), the reifier's type
/// and two IRI-valued annotations, and `spans` blank leaf spans each linked
/// from the reifier and carrying five statements (a type, an IRI link, two
/// `xsd:nonNegativeInteger` literals and an IRI kind). The last
/// `tie_percent` percent of the spans are identical in all four values.
pub(crate) fn claim(spans: u32, literal_len: usize, tie_percent: u32) -> Arc<RdfDataset> {
    use purrdf_iri::vocab::rdf::TYPE;
    use purrdf_xsd::datatype::XSD_NON_NEGATIVE_INTEGER;
    let mut b = RdfDatasetBuilder::new();
    let iri = |b: &mut RdfDatasetBuilder, name: &str| {
        b.intern_iri(&format!("http://example.org/{name}"))
            .expect("an example IRI interns")
    };
    let g = iri(&mut b, "claims/1");
    let rdf_type = b.intern_iri(TYPE).expect("rdf:type interns");
    let (s, p) = (iri(&mut b, "subject"), iri(&mut b, "predicate"));
    let text: String = (0..literal_len)
        .map(|i| char::from(b'a' + (i % 26) as u8))
        .collect();
    let o = b
        .intern_literal(RdfLiteral::simple(text))
        .expect("a literal interns");
    let triple = b.intern_triple(s, p, o).expect("a triple term interns");
    let r = b
        .intern_blank("r", BlankScope::DEFAULT)
        .expect("a blank interns");
    b.push_reifier_in_graph(r, triple, Some(g))
        .expect("the reifier pushes");
    let claim_class = iri(&mut b, "Claim");
    b.push_quad(r, rdf_type, claim_class, Some(g))
        .expect("a quad");
    for (pred, value) in [("source", "doc/a"), ("method", "m/1")] {
        let (pp, vv) = (iri(&mut b, pred), iri(&mut b, value));
        b.push_quad(r, pp, vv, Some(g)).expect("a quad");
    }
    let tied_from = spans - spans * tie_percent / 100;
    let (has_span, span_class) = (iri(&mut b, "span"), iri(&mut b, "Span"));
    let (link, start, end, kind) = (
        iri(&mut b, "link"),
        iri(&mut b, "start"),
        iri(&mut b, "end"),
        iri(&mut b, "kind"),
    );
    for i in 0..spans {
        let v = if i >= tied_from { 0 } else { i + 1 };
        let span = b
            .intern_blank(&format!("s{i}"), BlankScope::DEFAULT)
            .expect("a blank interns");
        b.push_quad(r, has_span, span, Some(g)).expect("a quad");
        b.push_quad(span, rdf_type, span_class, Some(g))
            .expect("a quad");
        let doc = iri(&mut b, &format!("doc/{}", v % 7));
        b.push_quad(span, link, doc, Some(g)).expect("a quad");
        for (pred, n) in [(start, 10 * v), (end, 10 * v + 5)] {
            let lit = b
                .intern_literal(RdfLiteral::typed(n.to_string(), XSD_NON_NEGATIVE_INTEGER))
                .expect("a literal interns");
            b.push_quad(span, pred, lit, Some(g)).expect("a quad");
        }
        let k = iri(&mut b, &format!("kind/{}", v % 3));
        b.push_quad(span, kind, k, Some(g)).expect("a quad");
    }
    b.freeze().expect("the claim freezes")
}

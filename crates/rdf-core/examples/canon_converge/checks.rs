// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Correctness checks of the revision 3.1 labelling: invariance under blank
//! relabelling and line shuffling, and the two modes against each other.

use std::fmt::Write as _;

use crate::r31::{Flag, Input, Options, literal, optimised};

/// SplitMix64 draws (the workspace's `purrdf_hash::mix`).
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) const fn next(&mut self) -> u64 {
        purrdf_hash::mix::splitmix64_next(&mut self.0)
    }

    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub(crate) fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i as u64 + 1) as usize);
        }
    }
}

/// The input's text with every blank node renamed by a seeded bijection
/// (`_:r<n>`) and the lines shuffled, both by Fisher-Yates over SplitMix64
/// from `seed`.
pub(crate) fn relabelled(input: &Input, seed: u64) -> String {
    let mut rng = Rng(seed);
    let mut images: Vec<usize> = (0..input.nodes()).collect();
    rng.shuffle(&mut images);
    let mut rows: Vec<String> = input
        .lines
        .iter()
        .map(|line| {
            let mut out = Vec::new();
            line.spell(&mut out, |u, o| {
                o.extend_from_slice(format!("_:r{}", images[u as usize]).as_bytes());
            });
            String::from_utf8(out).expect("UTF-8")
        })
        .chain(input.ground.iter().cloned())
        .collect();
    rng.shuffle(&mut rows);
    let mut text = rows.join("\n");
    text.push('\n');
    text
}

/// Optimised documents of the input and of three relabelled shuffles agree;
/// the literal document agrees too when it finishes inside `literal_limit`.
/// Returns a one-line report and counts (renamings, literal agreements,
/// literal disagreements).
pub(crate) fn invariance(name: &str, input: &Input, literal_limit: u64) -> (String, [u64; 3]) {
    let opt = |i: &Input| optimised::label_input(i, Options::DEFAULT);
    let Ok(base) = opt(input) else {
        return (format!("{name} optimised-refused"), [0; 3]);
    };
    let mut invariant = true;
    for seed in 1..=3 {
        let other = Input::read(&relabelled(input, seed)).expect("relabelled text reads");
        match opt(&other) {
            Ok(d) if d.document == base.document => {}
            _ => invariant = false,
        }
    }
    let lit = literal::label_input(
        input,
        Options {
            limit: literal_limit,
            ..Options::DEFAULT
        },
    );
    let (verdict, agree, differ) = match lit {
        Ok(l) if l.document == base.document => ("literal-equal", 1, 0),
        Ok(_) => ("LITERAL-DIFFERS", 0, 1),
        Err(_) => ("literal-over-limit", 0, 0),
    };
    (
        format!(
            "{name} nodes={} lines={} invariant={} {verdict} units={}",
            input.nodes(),
            input.lines.len(),
            if invariant { "yes" } else { "NO" },
            base.stats.units()
        ),
        [3, agree, differ],
    )
}

/// A random small input: `nodes` blank nodes, lines of one to three blank
/// positions (a node may repeat, a triple term may nest), a few IRIs and two
/// predicates; seeded.
pub(crate) fn random_input(seed: u64, nodes: u64, lines: u64) -> String {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut rows: Vec<String> = Vec::new();
    let term = |rng: &mut Rng| -> String {
        if rng.below(5) == 0 {
            format!("<http://example.org/c{}>", rng.below(2))
        } else {
            format!("_:n{}", rng.below(nodes))
        }
    };
    for _ in 0..lines {
        let p = format!("<http://example.org/p{}>", rng.below(2));
        let s = term(&mut rng);
        let row = match rng.below(6) {
            0 => {
                let (a, b) = (term(&mut rng), term(&mut rng));
                format!("{s} {p} <<( {a} <http://example.org/r> {b} )>> .")
            }
            1 => {
                let (o, g) = (term(&mut rng), format!("_:n{}", rng.below(nodes)));
                format!("{s} {p} {o} {g} .")
            }
            _ => format!("{s} {p} {} .", term(&mut rng)),
        };
        rows.push(row);
    }
    rows.join("\n") + "\n"
}

/// The text of a directed graph with one predicate over labels `prefix<i>`.
fn digraph(prefix: &str, n: u32, edges: u64) -> String {
    let mut out = String::new();
    let mut bit = 0;
    for a in 0..n {
        for b in 0..n {
            if a == b {
                continue;
            }
            if edges >> bit & 1 == 1 {
                let _ = writeln!(out, "_:{prefix}{a} <http://example.org/p> _:{prefix}{b} .");
            }
            bit += 1;
        }
    }
    out
}

fn weakly_connected(n: u32, edges: u64) -> bool {
    let mut parent: Vec<u32> = (0..n).collect();
    fn find(p: &[u32], x: u32) -> u32 {
        let mut r = x;
        while p[r as usize] != r {
            r = p[r as usize];
        }
        r
    }
    let mut bit = 0;
    for a in 0..n {
        for b in 0..n {
            if a == b {
                continue;
            }
            if edges >> bit & 1 == 1 {
                let (ra, rb) = (find(&parent, a), find(&parent, b));
                parent[ra as usize] = rb;
            }
            bit += 1;
        }
    }
    let r = find(&parent, 0);
    (0..n).all(|x| find(&parent, x) == r)
}

/// Search for two datasets `A + B` and `A + B'` (B, B' unrelated to A) in
/// which revision 3's single refinement over the whole fold gives component A
/// a different certificate than A alone; also checks that revision 3.1 never
/// does. Directed graphs with one predicate, A on 2 to `max_a` nodes and B on
/// 2 to `max_b` nodes, by total size ascending. Returns a report.
pub(crate) fn dependence_search(max_a: u32, max_b: u32) -> String {
    let whole = Options::DEFAULT.with(Flag::WholeFold, true);
    let mut tried = 0u64;
    let mut fresh_dependent = 0u64;
    for total in 4..=max_a + max_b {
        for na in 2..=max_a.min(total - 2) {
            let nb = total - na;
            if nb < 2 || nb > max_b {
                continue;
            }
            let a_bits = na * (na - 1);
            let b_bits = nb * (nb - 1);
            for ea in 1..1u64 << a_bits {
                if !weakly_connected(na, ea) {
                    continue;
                }
                let a_text = digraph("a", na, ea);
                let a = Input::read(&a_text).expect("reads");
                let alone = literal::label_input(&a, whole).expect("labels");
                for eb in 1..1u64 << b_bits {
                    if !weakly_connected(nb, eb) {
                        continue;
                    }
                    tried += 1;
                    let both_text = format!("{a_text}{}", digraph("z", nb, eb));
                    let both = Input::read(&both_text).expect("reads");
                    let joint = literal::label_input(&both, whole).expect("labels");
                    let fresh = literal::label_input(&both, Options::DEFAULT).expect("labels");
                    if !fresh.tops.iter().any(|t| *t == alone.tops[0]) {
                        fresh_dependent += 1;
                    }
                    if !joint.tops.iter().any(|t| *t == alone.tops[0]) {
                        return format!(
                            "FOUND after {tried} pairs (A {na} nodes, B {nb} nodes)\nA:\n{a_text}B:\n{}A alone (revision 3, whole fold = fresh):\n{}A + B under revision 3:\n{}A + B under revision 3.1:\n{}revision 3.1 dependent pairs so far: {fresh_dependent}",
                            digraph("z", nb, eb),
                            alone.document,
                            joint.document,
                            fresh.document
                        );
                    }
                }
            }
        }
    }
    format!(
        "none found: {tried} pairs, A up to {max_a} nodes, B up to {max_b} nodes (directed graphs, one predicate, every edge set, weakly connected); revision 3.1 dependent pairs: {fresh_dependent}"
    )
}

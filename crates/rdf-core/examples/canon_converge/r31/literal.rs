// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! LITERAL: the definition with no shortcut.
//!
//! Every partition is a list of cells (vectors) per side, colours are computed
//! from cell sizes when needed, every child of every branching call is computed
//! on its own copy, components are found by a full search of the remainder and
//! nothing is pruned. The recursion follows the text call by call; it runs on a
//! thread whose stack is sized from the number of nodes, the bound on the depth
//! (6.6), so the depth of an input cannot overflow it.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::Arc;

use super::{
    Cert, Chosen, Entry, Flag, Input, Labelled, Options, Refusal, Stats, cmp_cert, cmp_invariant,
    components, document, entry, formed_cost, label, pick_target, start_cost,
};

/// Ordered cells of one side.
type Cells = Vec<Vec<u32>>;

struct Engine<'a> {
    input: &'a Input,
    node_lines: Vec<Vec<u32>>,
    /// `free[u]`: `u` is not fixed (a constant) at this point of the recursion.
    free: Vec<bool>,
    /// Path position of each fixed node.
    pos: Vec<u64>,
    stats: Stats,
    options: Options,
    /// Target cells of the enclosing branching calls (the `Traces` selector).
    ancestors: Vec<Vec<u32>>,
}

impl Engine<'_> {
    fn check(&self) -> Result<(), Refusal> {
        if self.stats.units() > self.options.limit {
            return Err(Refusal::Limit(Box::new(self.stats.clone())));
        }
        Ok(())
    }

    /// `KEY(x, W)` (5.2). `side` is the side of `x`.
    fn key(&self, side: usize, x: u32, w: &BTreeSet<u32>) -> Vec<Vec<u64>> {
        let mut key = Vec::new();
        if side == 0 {
            for &l in &self.node_lines[x as usize] {
                if w.contains(&l) {
                    key.push(label(&self.input.lines[l as usize], x, &self.free));
                }
            }
        } else {
            let line = &self.input.lines[x as usize];
            for u in line.nodes() {
                if self.free[u as usize] && w.contains(&u) {
                    key.push(label(line, u, &self.free));
                }
            }
        }
        key.sort();
        key
    }

    /// Input labels of nodes, `L<n>` for lines (the trace).
    fn names(&self, side: usize, cell: &[u32]) -> Vec<String> {
        cell.iter()
            .map(|&x| {
                if side == 0 {
                    self.input.labels[x as usize].clone()
                } else {
                    format!("L{x}")
                }
            })
            .collect()
    }

    /// Incidences of a vertex (a node twice in a line is one incidence).
    fn degree(&self, side: usize, x: u32) -> u64 {
        if side == 0 {
            self.node_lines[x as usize].len() as u64
        } else {
            self.input.lines[x as usize]
                .nodes()
                .iter()
                .filter(|&&u| self.free[u as usize])
                .count() as u64
        }
    }

    /// Label elements of the incidences of a vertex.
    fn elements(&self, side: usize, x: u32) -> u64 {
        if side == 0 {
            self.node_lines[x as usize]
                .iter()
                .map(|&l| {
                    self.input.lines[l as usize]
                        .occ
                        .iter()
                        .filter(|&&u| u == x)
                        .count() as u64
                })
                .sum()
        } else {
            self.input.lines[x as usize]
                .occ
                .iter()
                .filter(|&&u| self.free[u as usize])
                .count() as u64
        }
    }

    fn neighbours(&self, side: usize, x: u32) -> Vec<u32> {
        if side == 0 {
            self.node_lines[x as usize].clone()
        } else {
            self.input.lines[x as usize]
                .nodes()
                .into_iter()
                .filter(|&u| self.free[u as usize])
                .collect()
        }
    }

    /// REFINE (5.5) from the given queue.
    fn refine(
        &mut self,
        part: &mut [Cells; 2],
        queue: &mut BTreeSet<(usize, u64)>,
    ) -> Result<(), Refusal> {
        while let Some((s, c)) = queue.pop_first() {
            self.stats.pops += 1;
            let w_index = cell_at(&part[s], c);
            let w: BTreeSet<u32> = part[s][w_index].iter().copied().collect();
            let other = 1 - s;
            let mut touched: BTreeSet<u32> = BTreeSet::new();
            for &x in &w {
                self.stats.visits += self.degree(s, x);
                self.stats.visit_elements += self.elements(s, x);
                touched.extend(self.neighbours(s, x));
            }
            self.check()?;
            // Non-singleton cells of the other side holding a touched vertex, by colour.
            let mut targets: Vec<u64> = Vec::new();
            let mut colour = 0u64;
            for cell in &part[other] {
                if cell.len() > 1 && cell.iter().any(|v| touched.contains(v)) {
                    targets.push(colour);
                }
                colour += cell.len() as u64;
            }
            if self.options.has(Flag::Trace) {
                eprintln!("pop({s},{c}) W={:?}", self.names(s, &part[s][w_index]));
            }
            for c_x in targets {
                let xi = cell_at(&part[other], c_x);
                let mut keyed: Vec<(Vec<Vec<u64>>, u32)> = part[other][xi]
                    .iter()
                    .map(|&x| (self.key(other, x, &w), x))
                    .collect();
                let mut compares = 0u64;
                keyed.sort_by(|a, b| {
                    compares += 1;
                    a.0.cmp(&b.0)
                });
                self.stats.key_compares += compares;
                let mut fragments: Vec<Vec<u32>> = Vec::new();
                for (i, (k, x)) in keyed.iter().enumerate() {
                    if i == 0 || *k != keyed[i - 1].0 {
                        fragments.push(Vec::new());
                    }
                    fragments.last_mut().expect("a fragment").push(*x);
                }
                if fragments.len() < 2 {
                    continue;
                }
                queue_rule(queue, other, c_x, &fragments);
                part[other].splice(xi..=xi, fragments);
                if self.options.has(Flag::Trace) {
                    let cells: Vec<Vec<String>> =
                        part[other].iter().map(|c| self.names(other, c)).collect();
                    eprintln!("  side {other} -> {cells:?} queue {queue:?}");
                }
            }
        }
        Ok(())
    }

    /// FIX the singleton node cells, form `E`, find the free components, and
    /// make the nested calls (rules 1 and 2, or a child under rule 3).
    fn fix_and_nest(&mut self, part: &[Cells; 2], p: u64, depth: u64) -> Result<Chosen, Refusal> {
        self.stats.max_depth = self.stats.max_depth.max(depth);
        let fixed: Vec<u32> = part[0]
            .iter()
            .filter(|c| c.len() == 1)
            .map(|c| c[0])
            .collect();
        for (i, &u) in fixed.iter().enumerate() {
            self.free[u as usize] = false;
            self.pos[u as usize] = p.checked_add(i as u64).ok_or(Refusal::Width)?;
        }
        let s = fixed.len() as u64;
        let lines: Vec<u32> = part[1].iter().flatten().copied().collect();
        let mut e: Vec<Entry> = Vec::new();
        for &l in &lines {
            let line = &self.input.lines[l as usize];
            if line.occ.iter().all(|&u| !self.free[u as usize]) {
                e.push(entry(line, &self.pos));
            }
        }
        self.stats.entries_formed += formed_cost(&e);
        let incidences: u64 = fixed
            .iter()
            .map(|&u| self.node_lines[u as usize].len() as u64)
            .sum();
        self.stats.restriction += s + e.len() as u64 + incidences;
        e.sort();
        let rest: Vec<u32> = part[0]
            .iter()
            .filter(|c| c.len() > 1)
            .flatten()
            .copied()
            .collect();
        let comps = components(
            self.input,
            &self.node_lines,
            &rest,
            &self.free,
            &mut self.stats.discovery_steps,
        );
        self.charge_split(&comps);
        self.check()?;
        let mut nested: Vec<Chosen> = Vec::with_capacity(comps.len());
        for (cn, cl) in &comps {
            let restricted = restrict(part, cn, cl);
            self.stats.copied += (cn.len() + cl.len()) as u64;
            nested.push(self.call(restricted, p + s, depth + 1)?);
        }
        let mut compared = 0u64;
        nested.sort_by(|a, b| cmp_cert(&a.cert, &b.cert, &mut compared));
        self.stats.entries_compared += compared;
        if nested.windows(2).any(|w| w[0].cert == w[1].cert) {
            self.stats.equal_certificates += 1;
        }
        let size = s + nested.iter().map(|n| n.size).sum::<u64>();
        let cert = Arc::new(Cert {
            e,
            c: nested.iter().map(|n| Arc::clone(&n.cert)).collect(),
        });
        Ok(Chosen {
            cert,
            fixed,
            nested,
            size,
        })
    }

    /// The restriction charge for a split into `comps` (8.1).
    fn charge_split(&mut self, comps: &[(Vec<u32>, Vec<u32>)]) {
        if comps.len() >= 2 {
            let sizes: Vec<u64> = comps
                .iter()
                .map(|(n, l)| (n.len() + l.len()) as u64)
                .collect();
            let largest = *sizes.iter().max().expect("two components");
            self.stats.restriction += sizes.iter().sum::<u64>() - largest;
        }
    }

    /// A call (6.1).
    fn call(&mut self, part: [Cells; 2], p: u64, depth: u64) -> Result<Chosen, Refusal> {
        self.stats.calls += 1;
        let part = if self.options.has(Flag::Wl2) && part[0].iter().all(|c| c.len() > 1) {
            let mut part = part;
            part[0] = super::wl2::refine_nodes(self.input, &self.free, &part, &mut self.stats);
            let mut queue = all_cells(&part);
            self.refine(&mut part, &mut queue)?;
            part
        } else {
            part
        };
        if part[0].iter().all(|c| c.len() == 1) {
            self.stats.leaves += 1;
        }
        if part[0].iter().any(|c| c.len() == 1) {
            return self.fix_and_nest(&part, p, depth);
        }
        // Rule 3.
        self.stats.branch_calls += 1;
        self.stats.branch_nodes += part[0].iter().map(|c| c.len() as u64).sum::<u64>();
        let cells: Vec<&[u32]> = part[0].iter().map(Vec::as_slice).collect();
        let ti = pick_target(self.options.selector, &cells, &self.ancestors);
        let target = part[0][ti].clone();
        self.ancestors.push(target.clone());
        self.stats.largest_target = self.stats.largest_target.max(target.len() as u64);
        let colour: u64 = part[0][..ti].iter().map(|c| c.len() as u64).sum();
        let mut best: Option<(Vec<u32>, Chosen)> = None;
        let mut ties = 0u64;
        for &v in &target {
            self.stats.children += 1;
            let mut child = part.clone();
            let mut queue = individualise(&mut child[0], ti, colour, v);
            self.refine(&mut child, &mut queue)?;
            let invariant: Vec<u32> = if self.options.has(Flag::Invariant) {
                let mut i: Vec<u32> = child[0].iter().map(|c| c.len() as u32).collect();
                i.push(0);
                i.extend(child[1].iter().map(|c| c.len() as u32));
                self.stats.invariant += i.len() as u64 - 1;
                i
            } else {
                Vec::new()
            };
            let saved_free = self.free.clone();
            let result = self.fix_and_nest(&child, p, depth)?;
            self.free = saved_free;
            let replace = match &best {
                None => true,
                Some((inv, b)) => {
                    let mut compared = 0u64;
                    let order = cmp_invariant(&invariant, inv, &mut self.stats.invariant)
                        .then_with(|| cmp_cert(&result.cert, &b.cert, &mut compared));
                    self.stats.entries_compared += compared;
                    if order == Ordering::Equal {
                        ties += 1;
                    }
                    order == Ordering::Less
                }
            };
            if replace {
                if best.is_some() {
                    ties = 0;
                }
                best = Some((invariant, result));
            }
            self.check()?;
        }
        self.ancestors.pop();
        if ties > 0 {
            self.stats.equal_certificates += 1;
        }
        Ok(best.expect("a child").1)
    }
}

/// The index of the cell of colour `c`.
fn cell_at(cells: &Cells, c: u64) -> usize {
    let mut colour = 0u64;
    for (i, cell) in cells.iter().enumerate() {
        if colour == c {
            return i;
        }
        colour += cell.len() as u64;
    }
    panic!("no cell of colour {c}")
}

/// THE QUEUE RULE (5.4) for a cell of side `s` and colour `c` split into `fragments`.
fn queue_rule(queue: &mut BTreeSet<(usize, u64)>, s: usize, c: u64, fragments: &[Vec<u32>]) {
    let was_queued = queue.contains(&(s, c));
    let largest = fragments
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.len().cmp(&b.1.len()).then(b.0.cmp(&a.0)))
        .map(|(i, _)| i)
        .expect("fragments");
    let mut colour = c;
    for (i, f) in fragments.iter().enumerate() {
        if was_queued || i != largest {
            queue.insert((s, colour));
        }
        colour += f.len() as u64;
    }
}

/// INDIVIDUALISATION (5.8) of `v` in node cell `ti` of colour `c`; the queue.
fn individualise(nodes: &mut Cells, ti: usize, c: u64, v: u32) -> BTreeSet<(usize, u64)> {
    let rest: Vec<u32> = nodes[ti].iter().copied().filter(|&x| x != v).collect();
    let fragments = vec![vec![v], rest];
    let mut queue = BTreeSet::new();
    queue_rule(&mut queue, 0, c, &fragments);
    nodes.splice(ti..=ti, fragments);
    queue
}

/// The partitions restricted to one component, order kept (5.7).
fn restrict(part: &[Cells; 2], nodes: &[u32], lines: &[u32]) -> [Cells; 2] {
    let keep = |cells: &Cells, members: &[u32]| -> Cells {
        cells
            .iter()
            .map(|c| {
                c.iter()
                    .copied()
                    .filter(|x| members.binary_search(x).is_ok())
                    .collect::<Vec<u32>>()
            })
            .filter(|c| !c.is_empty())
            .collect()
    };
    [keep(&part[0], nodes), keep(&part[1], lines)]
}

/// The fresh start of a set of nodes and lines (5.6): one node cell; line cells
/// by `[masked text, count]`; every cell queued.
fn fresh_start(
    input: &Input,
    nodes: &[u32],
    lines: &[u32],
) -> ([Cells; 2], BTreeSet<(usize, u64)>) {
    let mut keyed: Vec<((Vec<u8>, u64), u32)> = lines
        .iter()
        .map(|&l| {
            (
                (
                    input.plain_masked(l as usize),
                    input.lines[l as usize].count,
                ),
                l,
            )
        })
        .collect();
    keyed.sort();
    let mut line_cells: Cells = Vec::new();
    for (i, (k, l)) in keyed.iter().enumerate() {
        if i == 0 || *k != keyed[i - 1].0 {
            line_cells.push(Vec::new());
        }
        line_cells.last_mut().expect("a cell").push(*l);
    }
    let part = [vec![nodes.to_vec()], line_cells];
    let queue = all_cells(&part);
    (part, queue)
}

/// The pair of every cell of both sides.
fn all_cells(part: &[Cells; 2]) -> BTreeSet<(usize, u64)> {
    let mut queue = BTreeSet::new();
    for (s, cells) in part.iter().enumerate() {
        let mut colour = 0u64;
        for c in cells {
            queue.insert((s, colour));
            colour += c.len() as u64;
        }
    }
    queue
}

/// Label `input` by the definition, on a thread whose stack covers the depth.
pub(crate) fn label_input(input: &Input, options: Options) -> Result<Labelled, Refusal> {
    let stack = (1 << 20) | ((input.nodes() + 16) * 64 * 1024);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn_scoped(scope, || run(input, options))
            .expect("a thread for the literal recursion")
            .join()
            .expect("the literal recursion does not panic")
    })
}

fn run(input: &Input, options: Options) -> Result<Labelled, Refusal> {
    let n = input.nodes();
    let mut engine = Engine {
        input,
        node_lines: input.node_lines(),
        free: vec![true; n],
        pos: vec![0; n],
        stats: Stats {
            nodes: n as u64,
            lines: input.lines.len() as u64,
            ..Stats::default()
        },
        options,
        ancestors: Vec::new(),
    };
    let all: Vec<u32> = (0..n as u32).collect();
    let comps = components(
        input,
        &engine.node_lines,
        &all,
        &engine.free,
        &mut engine.stats.discovery_steps,
    );
    engine.stats.top_components = comps.len() as u64;
    engine.charge_split(&comps);
    let mut tops: Vec<Chosen> = Vec::with_capacity(comps.len());
    if options.has(Flag::WholeFold) {
        // Revision 3: one refinement over the whole fold; each top component
        // inherits its restriction.
        let lines: Vec<u32> = (0..input.lines.len() as u32).collect();
        engine.stats.start += start_cost(input, &lines);
        let (mut part, mut queue) = fresh_start(input, &all, &lines);
        engine.refine(&mut part, &mut queue)?;
        for (cn, cl) in &comps {
            tops.push(engine.call(restrict(&part, cn, cl), 0, 0)?);
        }
    } else {
        for (cn, cl) in &comps {
            engine.stats.start += start_cost(input, cl);
            let (mut part, mut queue) = fresh_start(input, cn, cl);
            engine.refine(&mut part, &mut queue)?;
            tops.push(engine.call(part, 0, 0)?);
        }
    }
    let mut compared = 0u64;
    tops.sort_by(|a, b| cmp_cert(&a.cert, &b.cert, &mut compared));
    engine.stats.entries_compared += compared;
    if tops.windows(2).any(|w| w[0].cert == w[1].cert) {
        engine.stats.equal_certificates += 1;
    }
    engine.check()?;
    let mut positions = vec![0u64; n];
    let mut o = 0u64;
    for t in &tops {
        t.assign(o, &mut positions);
        o += t.size;
    }
    Ok(Labelled {
        document: document(input, &positions),
        positions,
        tops: tops.iter().map(|t| Arc::clone(&t.cert)).collect(),
        stats: engine.stats,
    })
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Experiment, not the text: two-dimensional colour refinement of the node
//! partition of a call that reaches the branching rule.
//!
//! Pairs `(x, y)` of the call's nodes start from the tuple of the cells of `x`
//! and `y` and the sorted list of `(line cell, label of x, label of y)` over the
//! lines holding both. A round recolours a pair by its colour and the sorted
//! list over `z` of `(colour(x, z), colour(z, y))`. Colours are ranks of the
//! sorted distinct values (no hash), so the result is a function of the
//! isomorphism class. Rounds stop when the number of colours stops growing.
//! The node cells are then split by the diagonal colour, each cell's fragments
//! in ascending colour, the inherited order of cells kept.

use std::collections::BTreeSet;

use super::{Input, Stats, label};

/// `(line cell, label of the first node, label of the second)`.
type LineTriple = (u64, Vec<u64>, Vec<u64>);

/// The node cells of `part` refined by two-dimensional refinement.
pub(crate) fn refine_nodes(
    input: &Input,
    free: &[bool],
    part: &[Vec<Vec<u32>>; 2],
    stats: &mut Stats,
) -> Vec<Vec<u32>> {
    let nodes: Vec<u32> = part[0].iter().flatten().copied().collect();
    let k = nodes.len();
    let index = |u: u32| nodes.iter().position(|&x| x == u);
    // Initial pair values.
    let mut init: Vec<Vec<LineTriple>> = vec![Vec::new(); k * k];
    for (ci, cell) in part[1].iter().enumerate() {
        for &l in cell {
            let line = &input.lines[l as usize];
            let members: Vec<(usize, Vec<u64>)> = line
                .nodes()
                .into_iter()
                .filter(|&u| free[u as usize])
                .filter_map(|u| index(u).map(|i| (i, label(line, u, free))))
                .collect();
            for (a, la) in &members {
                for (b, lb) in &members {
                    init[a * k + b].push((ci as u64, la.clone(), lb.clone()));
                }
            }
        }
    }
    let mut node_cell = vec![0u64; k];
    let mut i = 0;
    for (ci, cell) in part[0].iter().enumerate() {
        for _ in cell {
            node_cell[i] = ci as u64;
            i += 1;
        }
    }
    let first: Vec<(u64, u64, Vec<LineTriple>)> = (0..k * k)
        .map(|p| {
            let mut v = std::mem::take(&mut init[p]);
            v.sort();
            stats.wl2_units += 2 + v.len() as u64;
            (node_cell[p / k], node_cell[p % k], v)
        })
        .collect();
    let mut colour = ranks(&first);
    let mut classes = colour.iter().copied().max().map_or(0, |m| m + 1);
    loop {
        let signature = |p: usize, colour: &[u64]| -> (u64, Vec<(u64, u64)>) {
            let (x, y) = (p / k, p % k);
            let mut v: Vec<(u64, u64)> = (0..k)
                .map(|z| (colour[x * k + z], colour[z * k + y]))
                .collect();
            v.sort_unstable();
            (colour[p], v)
        };
        let distinct: BTreeSet<(u64, Vec<(u64, u64)>)> =
            (0..k * k).map(|p| signature(p, &colour)).collect();
        stats.wl2_steps += (k * k * k) as u64;
        stats.wl2_units += (k * k * (k + 1)) as u64;
        let sorted: Vec<&(u64, Vec<(u64, u64)>)> = distinct.iter().collect();
        let next: Vec<u64> = (0..k * k)
            .map(|p| {
                let s = signature(p, &colour);
                sorted.binary_search(&&s).expect("a signature seen") as u64
            })
            .collect();
        let n = sorted.len() as u64;
        colour = next;
        if n == classes {
            break;
        }
        classes = n;
    }
    let mut out: Vec<Vec<u32>> = Vec::new();
    let mut i = 0;
    for cell in &part[0] {
        let mut keyed: Vec<(u64, u32)> = cell
            .iter()
            .map(|&u| {
                let d = colour[i * k + i];
                i += 1;
                (d, u)
            })
            .collect();
        keyed.sort_unstable();
        for (j, (d, u)) in keyed.iter().enumerate() {
            if j == 0 || *d != keyed[j - 1].0 {
                out.push(Vec::new());
            }
            out.last_mut().expect("a cell").push(*u);
        }
    }
    out
}

/// Ranks of values among their sorted distinct values.
fn ranks<T: Ord>(values: &[T]) -> Vec<u64> {
    let distinct: BTreeSet<&T> = values.iter().collect();
    let sorted: Vec<&T> = distinct.into_iter().collect();
    values
        .iter()
        .map(|v| sorted.binary_search(&v).expect("a value seen") as u64)
        .collect()
}

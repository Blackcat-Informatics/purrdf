// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The optimised engine is held to the reference engine: on every worked
//! example, on the named inputs of the definition under several namings and
//! both visiting orders, and on generated inputs, the two give the same
//! canonical map, identities, line sequence, digest, structural record and
//! first failures, whatever shortcuts are on. The stops of 10.4 fire where
//! the definition says, the stop signal is honoured, and the limit refuses
//! the same input the same way.

use super::optimised::{self, Options};
use super::reference;
use super::tests::{
    blocks, both_directions, corpus, failure_record, hub, quad, quadratic_residue_13, relabel,
};
use super::{Halt, Labelled, Line, Refusal, admit};
use crate::StopSignal;

/// Every combination of the three switches of the optimised engine.
fn every_option() -> Vec<Options> {
    let mut out = Vec::new();
    for stops in [false, true] {
        for orbits in [false, true] {
            for descending in [false, true] {
                out.push(Options {
                    stops,
                    orbits,
                    descending,
                });
            }
        }
    }
    out
}

/// Everything deterministic in a result, compared whole. The engine figures
/// (units of explored calls, twin units, comparison units, stops) are the
/// engine's own and are not compared.
fn assert_same(context: &str, reference: &Labelled, optimised: &Labelled) {
    assert_eq!(
        reference.placements, optimised.placements,
        "{context}: canonical map and identities"
    );
    assert_eq!(
        reference.lines, optimised.lines,
        "{context}: canonical line sequence"
    );
    assert_eq!(reference.digest, optimised.digest, "{context}: digest");
    assert_eq!(
        reference.record, optimised.record,
        "{context}: structural record"
    );
    assert_eq!(
        reference.diagnostic, optimised.diagnostic,
        "{context}: failures with their sources"
    );
    assert_eq!(reference.components.len(), optimised.components.len());
    for (r, o) in reference.components.iter().zip(&optimised.components) {
        assert_eq!(r.identity, o.identity, "{context}: component identity");
        assert_eq!(r.offset, o.offset, "{context}: offset");
        assert_eq!(r.nodes, o.nodes, "{context}: nodes");
        assert_eq!(r.copy_index, o.copy_index, "{context}: copy index");
        assert_eq!(r.smallest_node, o.smallest_node, "{context}: smallest node");
        assert_eq!(r.certificate, o.certificate, "{context}: certificate");
        assert_eq!(r.w, o.w, "{context}: W");
    }
}

fn reference_of(lines: &[Line]) -> Labelled {
    let set = admit(lines).expect("well formed");
    reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("labelled")
}

fn optimised_of(lines: &[Line], options: Options) -> Labelled {
    let set = admit(lines).expect("well formed");
    optimised::label_set(&set, optimised::ENGINE_LIMIT, false, options, None)
        .unwrap_or_else(|halt| panic!("the optimised engine did not label: {halt:?}"))
}

/// Property: the optimised engine computes the reference result. INVARIANT
/// held for every switch of the engine, so a shortcut that is wrong on an
/// input shows as a difference from the oracle.
fn assert_agrees(context: &str, lines: &[Line]) {
    let oracle = reference_of(lines);
    for options in every_option() {
        let result = optimised_of(lines, options);
        assert_same(&format!("{context} {options:?}"), &oracle, &result);
    }
}

#[test]
fn the_optimised_engine_agrees_with_the_reference_on_every_worked_example() {
    for (name, _, lines, _) in blocks() {
        assert_agrees(name, &lines);
    }
}

#[test]
fn the_optimised_engine_agrees_on_the_corpus_under_twenty_relabellings() {
    for (i, lines) in corpus().iter().enumerate() {
        assert_agrees(&format!("corpus {i}"), lines);
        for seed in 1..=3 {
            assert_agrees(&format!("corpus {i} seed {seed}"), &relabel(lines, seed));
        }
    }
}

// ---------------------------------------------------------------------------
// The named inputs of 13.2.

fn node(prefix: &str, i: usize, width: usize) -> String {
    format!("{prefix}{i:0width$}")
}

/// `COMPL(k1, k2)`: every ordered pair of distinct nodes that is not an edge
/// of the cycle on the first `k1` nodes or of the cycle on the next `k2`. The
/// nodes of the second cycle are named first when `second_first`.
fn complement_of_cycles(k1: usize, k2: usize, second_first: bool) -> Vec<Line> {
    let n = k1 + k2;
    let width = if n >= 10 { 2 } else { 1 };
    // Index of each node in its name: the first cycle `0..k1`, the second
    // `k1..n`, the names swapped by `second_first`.
    let name = |i: usize| {
        let at = if second_first {
            if i < k1 { i + k2 } else { i - k1 }
        } else {
            i
        };
        node("n", at, width)
    };
    let mut edge = vec![vec![false; n]; n];
    for (start, k) in [(0, k1), (k1, k2)] {
        for i in 0..k {
            let (a, b) = (start + i, start + (i + 1) % k);
            edge[a][b] = true;
            edge[b][a] = true;
        }
    }
    let mut lines = Vec::new();
    for (x, row) in edge.iter().enumerate() {
        for (y, &adjacent) in row.iter().enumerate() {
            if x != y && !adjacent {
                lines.push(quad(&name(x), "<p>", &name(y)));
            }
        }
    }
    lines
}

fn edges_of(pairs: &[(&str, &str)]) -> Vec<Line> {
    let owned: Vec<(String, String)> = pairs
        .iter()
        .map(|&(a, b)| (a.to_owned(), b.to_owned()))
        .collect();
    both_directions(&owned)
}

fn fifteen() -> Vec<Line> {
    let mut lines = vec![
        quad("c0", "<p>", "c1"),
        quad("c1", "<p>", "c2"),
        quad("c2", "<p>", "c0"),
    ];
    for x in ["a1", "a2"] {
        for y in ["c0", "c1", "c2"] {
            lines.push(quad(x, "<p>", y));
            lines.push(quad(y, "<p>", x));
        }
    }
    lines
}

fn cub8() -> Vec<Line> {
    edges_of(&[
        ("w0", "w1"),
        ("w0", "w4"),
        ("w0", "w5"),
        ("w1", "w6"),
        ("w1", "w7"),
        ("w2", "w3"),
        ("w2", "w4"),
        ("w2", "w6"),
        ("w3", "w5"),
        ("w3", "w6"),
        ("w4", "w7"),
        ("w5", "w7"),
    ])
}

fn k4_pair() -> Vec<Line> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for prefix in ["a", "b"] {
        for i in 0..4 {
            for j in i + 1..4 {
                if (i, j) != (2, 3) {
                    pairs.push((format!("{prefix}{i}"), format!("{prefix}{j}")));
                }
            }
        }
    }
    pairs.push(("a2".into(), "b2".into()));
    pairs.push(("a3".into(), "b3".into()));
    both_directions(&pairs)
}

fn z4_z4() -> Vec<Line> {
    let steps = [(1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)];
    let mut lines = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            for (di, dj) in steps {
                let (k, l) = ((i + di) % 4, (j + dj) % 4);
                lines.push(quad(&format!("v{i}{j}"), "<p>", &format!("v{k}{l}")));
            }
        }
    }
    lines
}

fn shared() -> Vec<Line> {
    vec![
        quad("x0", "<p>", "x1"),
        quad("x0", "<p>", "x2"),
        quad("y0", "<p>", "y3"),
        quad("y1", "<p>", "y0"),
        quad("y1", "<p>", "y2"),
        quad("y2", "<p>", "y0"),
        quad("y3", "<p>", "y0"),
    ]
}

fn named_inputs() -> Vec<(&'static str, Vec<Line>)> {
    vec![
        ("COMPL(4,6)", complement_of_cycles(4, 6, false)),
        ("COMPL(4,6) second first", complement_of_cycles(4, 6, true)),
        ("COMPL(3,5)", complement_of_cycles(3, 5, false)),
        ("COMPL(3,5) second first", complement_of_cycles(3, 5, true)),
        ("FIFTEEN", fifteen()),
        ("CUB8", cub8()),
        ("K4PAIR", k4_pair()),
        ("Z4Z4", z4_z4()),
        ("SHARED", shared()),
        ("QR13", quadratic_residue_13("r")),
    ]
}

/// The inputs of the stops and of the shortcuts of section 10, under the
/// namings of 13.2 (seeds 1 to 3) and both visiting orders: every shortcut
/// of the engine, and all of them together, equal the oracle.
#[test]
fn the_named_inputs_of_section_ten_agree_under_every_shortcut_and_naming() {
    for (name, lines) in named_inputs() {
        assert_agrees(name, &lines);
        for seed in 1..=3 {
            assert_agrees(&format!("{name} seed {seed}"), &relabel(&lines, seed));
        }
    }
}

const STOPS_ONLY: Options = Options {
    stops: true,
    orbits: false,
    descending: false,
};

/// 10.4: with no other shortcut and the children in ascending source order,
/// the stop on `s` fires three times at the branching calls of the result
/// path of `FIFTEEN` (the children of `c0`, `c1`, `c2` have `s = 3`, the best
/// `s = 2`), and the stop on `E` fires six times on `CUB8`.
#[test]
fn the_strict_stops_fire_where_the_definition_counts_them() {
    let on_fifteen = optimised_of(&fifteen(), STOPS_ONLY);
    assert_eq!(on_fifteen.stopped_on_s, 3, "stops on s on FIFTEEN");
    let on_cub8 = optimised_of(&cub8(), STOPS_ONLY);
    assert_eq!(on_cub8.stopped_on_e, 6, "stops on E on CUB8");
    // The neighbour: the oracle explores every child and stops none.
    let oracle = reference_of(&fifteen());
    assert_eq!(oracle.stopped_on_s + oracle.stopped_on_e, 0);
}

/// 10.4, why there is no stop on a nested call: on `COMPL(4, 6)` a child whose
/// nested call has the smaller `s` and the larger `E` is the smaller
/// certificate. The permanent test of the correction: the nodes of the
/// 6-cycle are visited first (its names are smallest) and the nodes of the
/// 4-cycle first, in both visiting orders, and the engine returns the
/// reference's minimum each time.
#[test]
fn the_stop_never_returns_a_certificate_that_is_not_the_minimum() {
    for second_first in [false, true] {
        let lines = complement_of_cycles(4, 6, second_first);
        let oracle = reference_of(&lines);
        for descending in [false, true] {
            let options = Options {
                stops: true,
                orbits: false,
                descending,
            };
            let result = optimised_of(&lines, options);
            assert_same(
                &format!("COMPL(4,6) second_first={second_first} {options:?}"),
                &oracle,
                &result,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generated inputs.

struct Stream(u64);

impl Stream {
    fn next(&mut self) -> u64 {
        purrdf_hash::mix::splitmix64_next(&mut self.0)
    }

    /// `DRAW(m)` of 13.2: the high half of the 128-bit product.
    fn draw(&mut self, m: usize) -> usize {
        let r = self.next();
        usize::try_from((u128::from(r) * m as u128) >> 64).expect("below m")
    }
}

/// A circulant on `n` nodes with a random connection set, undirected or not.
fn circulant(stream: &mut Stream, prefix: &str, n: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let both = stream.draw(2) == 0;
    let mut any = false;
    for step in 1..=n / 2 {
        if stream.draw(2) == 0 {
            continue;
        }
        any = true;
        for i in 0..n {
            let j = (i + step) % n;
            lines.push(quad(
                &format!("{prefix}{i:02}"),
                "<p>",
                &format!("{prefix}{j:02}"),
            ));
            if both {
                lines.push(quad(
                    &format!("{prefix}{j:02}"),
                    "<p>",
                    &format!("{prefix}{i:02}"),
                ));
            }
        }
    }
    if !any {
        for i in 0..n {
            lines.push(quad(
                &format!("{prefix}{i:02}"),
                "<p>",
                &format!("{prefix}{:02}", (i + 1) % n),
            ));
        }
    }
    lines.sort_by_key(|line| format!("{line:?}"));
    lines.dedup_by_key(|line| format!("{line:?}"));
    lines
}

/// A random digraph on up to seven nodes.
fn digraph(stream: &mut Stream) -> Vec<Line> {
    let n = 2 + stream.draw(6);
    let density = 1 + stream.draw(4);
    let mut lines = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i != j && stream.draw(5) < density {
                let predicate = if stream.draw(4) == 0 { "<q>" } else { "<p>" };
                lines.push(quad(&format!("g{i}"), predicate, &format!("g{j}")));
            }
        }
    }
    if lines.is_empty() {
        lines.push(quad("g0", "<p>", "g1"));
    }
    lines
}

/// A generated input: a circulant, two circulants side by side, a circulant
/// with a hub on a random subset of its nodes, or a random digraph.
fn generated(seed: u64) -> Vec<Line> {
    let mut stream = Stream(seed);
    match stream.draw(4) {
        0 => {
            let n = 4 + stream.draw(10);
            circulant(&mut stream, "c", n)
        }
        1 => {
            let (n, m) = (3 + stream.draw(6), 3 + stream.draw(6));
            let mut lines = circulant(&mut stream, "c", n);
            lines.extend(circulant(&mut stream, "d", m));
            lines
        }
        2 => {
            let n = 4 + stream.draw(8);
            let mut lines = circulant(&mut stream, "c", n);
            for i in 0..n {
                if stream.draw(2) == 0 {
                    lines.push(quad("hub", "<q>", &format!("c{i:02}")));
                }
            }
            lines
        }
        _ => digraph(&mut stream),
    }
}

/// Property over generated inputs: the optimised engine equals the oracle
/// with every switch, for the input as drawn and for a relabelling of it.
#[test]
fn the_optimised_engine_agrees_with_the_reference_on_generated_inputs() {
    for seed in 1..=160 {
        let lines = generated(seed);
        assert_agrees(&format!("generated {seed}"), &lines);
        assert_agrees(
            &format!("generated {seed} relabelled"),
            &relabel(&lines, seed),
        );
    }
}

// ---------------------------------------------------------------------------
// The stop signal and the limit.

#[derive(Debug)]
struct Raised(bool);

impl StopSignal for Raised {
    fn stopped(&self) -> bool {
        self.0
    }
}

/// A raised stop signal ends the run with the typed stop, outside the
/// definition's refusals; the same input under a signal not raised completes.
#[test]
fn a_raised_stop_signal_stops_the_optimised_engine() {
    let set = admit(&complement_of_cycles(4, 6, false)).expect("well formed");
    let raised = Raised(true);
    let stopped = optimised::label_set(
        &set,
        optimised::ENGINE_LIMIT,
        false,
        Options::PRODUCTION,
        Some(&raised),
    );
    assert!(matches!(stopped, Err(Halt::Stopped)), "{stopped:?}");
    let quiet = Raised(false);
    let done = optimised::label_set(
        &set,
        optimised::ENGINE_LIMIT,
        false,
        Options::PRODUCTION,
        Some(&quiet),
    );
    assert!(done.is_ok());
}

/// 7.10, 7.11 and 8.3: under a limit its search exceeds, an input that is not
/// guaranteed is refused as `limit` naming the first failure of every
/// component; under a large limit the same input is labelled with the same
/// failures; and no limit refuses a guaranteed input.
#[test]
fn the_optimised_limit_refuses_what_the_reference_refuses_and_nothing_guaranteed() {
    let mut lines = quadratic_residue_13("v");
    lines.extend(quadratic_residue_13("w"));
    let set = admit(&lines).expect("well formed");
    let qr = failure_record(0, 13, 13, 78);
    let plain = Options {
        stops: false,
        orbits: false,
        descending: false,
    };
    match optimised::label_set(&set, 0, false, plain, None) {
        Err(Halt::Refused(Refusal::Limit {
            broken_by, nodes, ..
        })) => {
            assert_eq!(broken_by, [qr, qr]);
            assert_eq!(nodes, 13);
        }
        other => panic!("expected a limit refusal, got {other:?}"),
    }
    let labelled = optimised::label_set(&set, optimised::ENGINE_LIMIT, false, plain, None)
        .expect("the search fits");
    assert_eq!(labelled.record.broken_by, [qr, qr]);
    let guaranteed = admit(&hub(false)).expect("well formed");
    assert!(optimised::label_set(&guaranteed, 0, false, Options::PRODUCTION, None).is_ok());
}

/// A signal that is clear for its first `polls` polls and set afterwards.
#[derive(Debug)]
struct After(std::sync::atomic::AtomicU64);

impl StopSignal for After {
    fn stopped(&self) -> bool {
        let left = self.0.load(std::sync::atomic::Ordering::Relaxed);
        if left == 0 {
            return true;
        }
        self.0.store(left - 1, std::sync::atomic::Ordering::Relaxed);
        false
    }
}

/// The engine polls the signal at each call and not only on entry: a signal
/// clear for the entry poll and set at the first call stops the run, and one
/// that stays clear completes it.
#[test]
fn the_optimised_engine_polls_the_stop_signal_at_each_call() {
    let set = admit(&[quad("a", "<p>", "b"), quad("b", "<p>", "c")]).expect("well formed");
    let run = |clear_polls: u64| {
        optimised::label_set(
            &set,
            optimised::ENGINE_LIMIT,
            false,
            Options::PRODUCTION,
            Some(&After(std::sync::atomic::AtomicU64::new(clear_polls))),
        )
    };
    assert!(matches!(run(1), Err(Halt::Stopped)));
    assert!(run(u64::MAX).is_ok());
}

/// A shortcut only skips calls: on the inputs of 10.1 where the group is
/// large, the optimised engine explores strictly fewer units than the
/// reference tree, and never more on any input of the section.
#[test]
fn the_shortcuts_never_explore_more_units_than_the_reference_tree() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        let result = optimised_of(&lines, Options::PRODUCTION);
        assert!(
            result.explored.total() <= oracle.explored.total(),
            "{name}: {} units explored against the reference tree's {}",
            result.explored.total(),
            oracle.explored.total()
        );
    }
}

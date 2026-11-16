// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The optimised engine is held to the reference engine: on every worked
//! example, on the named inputs of the definition under several namings and
//! both visiting orders, and on generated inputs, the two give the same
//! canonical map, identities, line sequence, digest, structural record and
//! first failures, whatever shortcuts are on. The stops of 10.4 fire where
//! the definition says, the stop signal is honoured, and the limit refuses
//! the same input the same way.

use super::optimised::{self, Options, Wrong};
use super::reference;
use super::tests::{
    blocks, both_directions, corpus, failure_record, hub, quad, quadratic_residue_13, record,
    relabel, term,
};
use super::{Halt, Labelled, Line, Refusal, Term, admit};
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
                    ..Options::PRODUCTION
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
fn the_optimised_engine_agrees_on_the_corpus_under_three_relabellings() {
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
    ..Options::PRODUCTION
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
                ..Options::PRODUCTION
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
        ..Options::PRODUCTION
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

/// The same refusal and its valid neighbour with every shortcut on. The search
/// of two copies of `QR13` spends more limited units than its allowance even
/// with the routes (the invalid input); the same input under a large limit,
/// and a guaranteed input under the least limit, are labelled (the valid
/// neighbours).
#[test]
fn the_limit_refuses_with_every_shortcut_on_and_its_neighbours_pass() {
    let mut lines = quadratic_residue_13("v");
    lines.extend(quadratic_residue_13("w"));
    let set = admit(&lines).expect("well formed");
    let qr = failure_record(0, 13, 13, 78);
    match optimised::label_set(&set, 0, false, Options::PRODUCTION, None) {
        Err(Halt::Refused(Refusal::Limit {
            broken_by, nodes, ..
        })) => {
            assert_eq!(broken_by, [qr, qr]);
            assert_eq!(nodes, 13);
        }
        other => panic!("expected a limit refusal, got {other:?}"),
    }
    let labelled = optimised::label_set(
        &set,
        optimised::ENGINE_LIMIT,
        false,
        Options::PRODUCTION,
        None,
    )
    .expect("the search fits");
    assert_eq!(labelled.record.broken_by, [qr, qr]);
}

/// The twin units are those of the search-free run, in the optimised engine as
/// in the reference, on an input the search-free run does not settle (`QR13`)
/// and on one it does: the twin tests a search makes spend limited units and
/// are not counted in the figure.
#[test]
fn the_twin_units_are_those_of_the_search_free_run_whatever_the_search_does() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        for options in every_option() {
            let result = optimised_of(&lines, options);
            assert_eq!(
                oracle.twin_units, result.twin_units,
                "{name} {options:?}: twin units"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The enumeration spaces of 13.2, sampled, with their lines in the order of the
// definition.

fn x(i: usize) -> String {
    format!("x{i}")
}

/// The lines of a space picked by the bits of `mask`, line `i` by bit `i`.
fn picked(space: &[Line], mask: u64) -> Vec<Line> {
    space
        .iter()
        .enumerate()
        .filter(|&(i, _)| mask >> i & 1 == 1)
        .map(|(_, line)| line.clone())
        .collect()
}

/// `N3P1` (`y_i`, bit `3 i + j`) and `Q4` (`x_i`, bit `4 i + j`): the lines
/// `quad(_i, <p>, _j)` with loops, numbered by mask (13.2 item (h)).
fn loops_space(prefix: &str, side: usize) -> Vec<Line> {
    let mut out = Vec::new();
    for i in 0..side {
        for j in 0..side {
            out.push(quad(
                &format!("{prefix}{i}"),
                "<p>",
                &format!("{prefix}{j}"),
            ));
        }
    }
    out
}

/// The twenty lines `quad(x_i, <p>, x_j)`, `i` and `j` different, over five nodes.
fn q5_lines() -> Vec<Line> {
    let mut out = Vec::new();
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                out.push(quad(&x(i), "<p>", &x(j)));
            }
        }
    }
    out
}

/// `RB3`: six reifier bindings then six annotations, `i != j` in `0, 1, 2`.
fn rb3_lines() -> Vec<Line> {
    let pairs: Vec<(usize, usize)> = (0..3)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .filter(|&(i, j)| i != j)
        .collect();
    let mut out: Vec<Line> = pairs
        .iter()
        .map(|&(i, j)| Line::ReifierBinding {
            reifier: term(&x(i)),
            triple: Term::triple(term(&x(j)), term("<p>"), term(&x(i))),
            graph: None,
        })
        .collect();
    out.extend(pairs.iter().map(|&(i, j)| Line::Annotation {
        count: 1,
        reifier: term(&x(i)),
        predicate: term("<q>"),
        object: term(&x(j)),
        graph: None,
    }));
    out
}

/// `REC3`: the six edges of the triangle, then nine records.
fn rec3_lines() -> Vec<Line> {
    let mut out = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            if i != j {
                out.push(quad(&x(i), "<p>", &x(j)));
            }
        }
    }
    for terms in [
        &[0][..],
        &[1],
        &[2],
        &[0, 1],
        &[0, 2],
        &[1, 0],
        &[1, 2],
        &[2, 0],
        &[2, 1],
    ] {
        let terms: Vec<Term> = terms.iter().map(|&i| term(&x(i))).collect();
        out.push(record("r", &terms));
    }
    out
}

/// Every `step`-th non-empty mask of a space of `size` lines, from `1`.
fn masks(size: u32, step: usize) -> impl Iterator<Item = u64> {
    (1..1u64 << size).step_by(step)
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_n3p1() {
    let space = loops_space("y", 3);
    for mask in masks(9, 1) {
        assert_agrees(&format!("N3P1 mask {mask}"), &picked(&space, mask));
    }
}

/// `Q4`: every sixty-first mask, about a thousand sets of its sixty-five
/// thousand, each as numbered and relabelled.
#[test]
fn the_optimised_engine_agrees_on_sampled_sets_of_q4() {
    let space = loops_space("x", 4);
    for mask in masks(16, 61) {
        let lines = picked(&space, mask);
        assert_agrees(&format!("Q4 mask {mask}"), &lines);
        assert_agrees(
            &format!("Q4 mask {mask} relabelled"),
            &relabel(&lines, mask),
        );
    }
}

/// `Q5DENSE`: the sets of fourteen to twenty of the twenty lines, the only
/// space whose children do not all attain and where the stops fire. Every
/// fortieth set in the order of the removed lines.
#[test]
fn the_optimised_engine_agrees_on_sampled_sets_of_q5dense() {
    let space = q5_lines();
    let mut seen = 0usize;
    for removed in 0..1u64 << 20 {
        if removed.count_ones() > 6 {
            continue;
        }
        seen += 1;
        if (seen - 1) % 40 != 0 {
            continue;
        }
        let lines = picked(&space, !removed & ((1 << 20) - 1));
        assert_agrees(&format!("Q5DENSE without {removed:#x}"), &lines);
    }
    assert_eq!(seen, 60_460, "the sets of Q5DENSE");
}

/// `RB3`: reifier bindings, triple terms and annotations; route (1) over lines
/// of those kinds. Every seventh set.
#[test]
fn the_optimised_engine_agrees_on_sampled_sets_of_rb3() {
    let space = rb3_lines();
    for mask in masks(12, 7) {
        let lines = picked(&space, mask);
        assert_agrees(&format!("RB3 mask {mask}"), &lines);
        assert_agrees(
            &format!("RB3 mask {mask} relabelled"),
            &relabel(&lines, mask),
        );
    }
}

/// `REC3`: records over the triangle. Every fifty-third set.
#[test]
fn the_optimised_engine_agrees_on_sampled_sets_of_rec3() {
    let space = rec3_lines();
    for mask in masks(15, 53) {
        let lines = picked(&space, mask);
        assert_agrees(&format!("REC3 mask {mask}"), &lines);
        assert_agrees(
            &format!("REC3 mask {mask} relabelled"),
            &relabel(&lines, mask),
        );
    }
}

// ---------------------------------------------------------------------------
// That routes (1) and (4) act, and that the shortcuts of 10.5 are wrong.

fn k33() -> Vec<Line> {
    let mut lines = Vec::new();
    for i in 0..3 {
        for j in 0..3 {
            lines.push(quad(&format!("a{i}"), "<p>", &format!("b{j}")));
        }
    }
    lines
}

fn only(maps: bool, twins: bool) -> Options {
    Options {
        maps,
        twins,
        exchanges: false,
        lifting: false,
        leaves: false,
        ..Options::PRODUCTION
    }
}

fn no_orbits() -> Options {
    Options {
        orbits: false,
        ..Options::PRODUCTION
    }
}

/// 10.1 routes (1) and (4) each explore strictly fewer units than the engine
/// with no route where they apply: the map between equal certificates on the
/// complement of two cycles and on `Z4Z4`, the twins on `K33`; and the result is
/// the oracle's.
#[test]
fn routes_one_and_four_each_explore_fewer_units_than_no_route() {
    for (name, lines, maps, twins) in [
        ("COMPL(4,6)", complement_of_cycles(4, 6, false), true, false),
        ("Z4Z4", z4_z4(), true, false),
        ("FIFTEEN", fifteen(), false, true),
    ] {
        let oracle = reference_of(&lines);
        let none = optimised_of(&lines, no_orbits());
        let route = optimised_of(&lines, only(maps, twins));
        assert_same(name, &oracle, &route);
        assert!(
            route.explored.total() < none.explored.total(),
            "{name}: {} units with the route, {} without",
            route.explored.total(),
            none.explored.total()
        );
    }
}

/// Whether two results differ in anything the definition fixes.
fn differs(a: &Labelled, b: &Labelled) -> bool {
    a.placements != b.placements
        || a.lines != b.lines
        || a.digest != b.digest
        || a.record != b.record
        || a.components.len() != b.components.len()
        || a.components
            .iter()
            .zip(&b.components)
            .any(|(r, o)| r.identity != o.identity || r.certificate != o.certificate || r.w != o.w)
}

fn wrong_on(lines: &[Line], options: Options) -> bool {
    let set = admit(lines).expect("well formed");
    let wrong = optimised::label_set(&set, optimised::ENGINE_LIMIT, false, options, None)
        .expect("a wrong shortcut still labels");
    differs(&reference_of(lines), &wrong)
}

/// 10.5: each wrong shortcut changes the result on the input the definition
/// prints, under the stated naming (seed `1`, or `3` for (b)) and ascending
/// order, with every proved shortcut on; the sound engine on the same input
/// equals the oracle.
#[test]
fn each_wrong_shortcut_changes_the_result_on_its_printed_input() {
    let sound = Options::PRODUCTION;
    let on = |wrong| Options { wrong, ..sound };
    for (name, lines) in [
        ("COMPL(3,5)", relabel(&complement_of_cycles(3, 5, false), 1)),
        ("COMPL(4,6)", relabel(&complement_of_cycles(4, 6, false), 1)),
        ("K4PAIR", relabel(&k4_pair(), 1)),
        ("Z4Z4", relabel(&z4_z4(), 1)),
    ] {
        assert!(!wrong_on(&lines, sound), "{name}: sound");
        assert!(wrong_on(&lines, on(Wrong::StopOnTie)), "(a) {name}");
    }
    let z = relabel(&z4_z4(), 3);
    assert!(!wrong_on(&z, sound), "Z4Z4 seed 3: sound");
    assert!(wrong_on(&z, on(Wrong::MapsEverywhere)), "(b) Z4Z4 seed 3");
    let invariant = Options {
        stops: false,
        orbits: false,
        wrong: Wrong::InvariantFirst,
        ..sound
    };
    assert!(wrong_on(&fifteen(), invariant), "(d) FIFTEEN");
    assert!(!wrong_on(&fifteen(), no_orbits()), "FIFTEEN: sound");
    assert!(wrong_on(&k33(), on(Wrong::FixTwinClass)), "(f) K33");
    assert!(!wrong_on(&k33(), sound), "K33: sound");
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

// ---------------------------------------------------------------------------
// The routes of 10.1, each alone.

/// The engine with orbits and none of the routes beyond (1) and (4).
const BASE_ROUTES: Options = Options {
    exchanges: false,
    lifting: false,
    leaves: false,
    ..Options::PRODUCTION
};

/// Each route alone, then all, over the base, in both visiting orders: the
/// result is the oracle's, and a route never explores more than the base.
fn route_variants(descending: bool) -> Vec<(&'static str, Options)> {
    let base = Options {
        descending,
        ..BASE_ROUTES
    };
    vec![
        ("base", base),
        (
            "exchanges",
            Options {
                exchanges: true,
                ..base
            },
        ),
        (
            "lifting",
            Options {
                lifting: true,
                ..base
            },
        ),
        (
            "leaves",
            Options {
                leaves: true,
                ..base
            },
        ),
        (
            "all",
            Options {
                exchanges: true,
                lifting: true,
                leaves: true,
                ..base
            },
        ),
    ]
}

fn explored_of(lines: &[Line], options: Options) -> u64 {
    optimised_of(lines, options).explored.total()
}

/// 10.1: every route alone and all together return the oracle's result in both
/// visiting orders on the named inputs and on generated inputs, and none
/// explores more than the engine with only routes (1) and (4): a map that is
/// used skips children and adds none.
#[test]
fn each_route_alone_and_all_together_agree_with_the_oracle_and_never_cost() {
    let mut inputs = named_inputs();
    for seed in 1..=60 {
        inputs.push(("generated", generated(seed)));
    }
    for (name, lines) in inputs {
        let oracle = reference_of(&lines);
        for descending in [false, true] {
            let variants = route_variants(descending);
            let base = explored_of(&lines, variants[0].1);
            for (route, options) in variants {
                let result = optimised_of(&lines, options);
                assert_same(&format!("{name} {route} {options:?}"), &oracle, &result);
                assert!(
                    result.explored.total() <= base,
                    "{name} {route} descending={descending}: {} explored units against {base}",
                    result.explored.total()
                );
            }
        }
    }
}

/// Lifting (10.1): a map used at a nested call is used at the call above it,
/// so the complement of two cycles, whose nested calls hold the symmetry,
/// explores fewer units with it, in both visiting orders and both namings.
#[test]
fn lifting_skips_children_the_nested_maps_already_cover() {
    for second_first in [false, true] {
        let lines = complement_of_cycles(4, 6, second_first);
        for descending in [false, true] {
            let variants = route_variants(descending);
            let base = explored_of(&lines, variants[0].1);
            let lifted = explored_of(&lines, variants[2].1);
            assert!(
                lifted < base,
                "COMPL(4,6) second_first={second_first} descending={descending}: {lifted} against {base}"
            );
        }
    }
}

/// Route (2) (10.1): the exchange of two tied nested calls joins children the
/// other routes leave apart, on at least one generated input, and with
/// lifting it joins more still on `K4PAIR`, whose two copies are exchanged.
#[test]
fn the_exchange_of_tied_nested_calls_joins_children_of_the_call_above() {
    let strict = (1..=160).any(|seed| {
        let lines = generated(seed);
        let variants = route_variants(false);
        explored_of(&lines, variants[1].1) < explored_of(&lines, variants[0].1)
    });
    assert!(strict, "no generated input is helped by the exchange");
    let variants = route_variants(false);
    let lines = k4_pair();
    let all = explored_of(&lines, variants[4].1);
    assert!(all < explored_of(&lines, variants[1].1));
    assert!(all < explored_of(&lines, variants[2].1));
}

/// Route (5) (10.1): two leaves of the top call with equal line sets give the
/// map that route (1) gives from equal certificates, so the engine that keys
/// the top call by its leaves explores what the engine that keys it by
/// certificate explores, and both return the oracle's result.
#[test]
fn leaves_of_the_top_call_join_the_same_children_as_equal_certificates() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        for descending in [false, true] {
            let variants = route_variants(descending);
            let (by_cert, by_leaf) = (variants[0].1, variants[3].1);
            let (a, b) = (optimised_of(&lines, by_cert), optimised_of(&lines, by_leaf));
            assert_same(name, &oracle, &b);
            assert_eq!(a.explored, b.explored, "{name} descending={descending}");
        }
    }
}

/// Orbit use in descending order (10.1 with 6.4): the children the classes
/// skip are not explored, the attaining child of smallest source is explored
/// when its class was skipped, and the result is the ascending one.
#[test]
fn descending_order_uses_the_classes_and_still_names_the_smallest_attaining_child() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        let plain = Options {
            orbits: false,
            descending: true,
            ..Options::PRODUCTION
        };
        let used = Options {
            descending: true,
            ..Options::PRODUCTION
        };
        let (a, b) = (optimised_of(&lines, plain), optimised_of(&lines, used));
        assert_same(name, &oracle, &b);
        assert!(b.explored.total() <= a.explored.total(), "{name}");
        if name == "COMPL(4,6)" || name == "Z4Z4" {
            assert!(b.explored.total() < a.explored.total(), "{name}");
        }
    }
}

/// 10.5(c), Lemma M (iii): on the complete graph on `a`, `b`, `c`, the state of
/// the child `c` carried onto `a` by the transposition of `a` and `c` gives
/// `a 0`, `c 1`, `b 2`, and the canonical map has `a 0`, `b 1`, `c 2`; the sound
/// engine, in the same descending order, and the reference give the canonical
/// map. The search is forced, since the search-free run settles the graph.
#[test]
fn carrying_a_choice_across_a_map_gives_the_state_of_the_wrong_child() {
    let edges: Vec<(String, String)> = [("a", "b"), ("a", "c"), ("b", "c")]
        .iter()
        .map(|&(x, y)| (x.to_owned(), y.to_owned()))
        .collect();
    let set = admit(&both_directions(&edges)).expect("well formed");
    let by_name = |labelled: &Labelled| -> Vec<(String, u64)> {
        let mut named: Vec<(String, u64)> = labelled
            .placements
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    String::from_utf8_lossy(set.source(i)).into_owned(),
                    p.position,
                )
            })
            .collect();
        named.sort();
        named
    };
    let canonical = |names: [(&str, u64); 3]| -> Vec<(String, u64)> {
        names.iter().map(|&(n, p)| (n.to_owned(), p)).collect()
    };
    let oracle = reference::label_set(&set, reference::ENGINE_LIMIT, true).expect("labelled");
    assert_eq!(by_name(&oracle), canonical([("a", 0), ("b", 1), ("c", 2)]));
    let options = |wrong| Options {
        descending: true,
        wrong,
        ..Options::PRODUCTION
    };
    let run = |wrong| {
        optimised::label_set(&set, optimised::ENGINE_LIMIT, true, options(wrong), None)
            .expect("labelled")
    };
    assert_eq!(by_name(&run(Wrong::Sound)), by_name(&oracle));
    assert_eq!(
        by_name(&run(Wrong::CarryChoice)),
        canonical([("a", 0), ("b", 2), ("c", 1)])
    );
}

/// 10.5(e): the second component of `SHARED`, refined alone, has the node cells
/// `[y1] [y2] [y3] [y0]`; refined with the first, `[y1] [y3] [y2] [y0]`. Every
/// cell is a singleton, so the engine that shares the partition returns another
/// canonical line sequence, and the sound engine the reference's.
#[test]
fn a_partition_shared_by_two_components_changes_the_second_one() {
    let set = admit(&shared()).expect("well formed");
    let cells = |names: &[&[&str]]| -> Vec<Vec<Vec<u8>>> {
        names
            .iter()
            .map(|cell| cell.iter().map(|n| n.as_bytes().to_vec()).collect())
            .collect()
    };
    assert_eq!(
        optimised::fresh_node_cells(&set, false)[1],
        cells(&[&["y1"], &["y2"], &["y3"], &["y0"]])
    );
    assert_eq!(
        optimised::fresh_node_cells(&set, true)[1],
        cells(&[&["y1"], &["y3"], &["y2"], &["y0"]])
    );
    let shared_options = Options {
        wrong: Wrong::SharedPartition,
        ..Options::PRODUCTION
    };
    assert!(!wrong_on(&shared(), Options::PRODUCTION), "sound");
    assert!(wrong_on(&shared(), shared_options), "(e) SHARED");
    let oracle = reference_of(&shared());
    let wrong = optimised_of(&shared(), shared_options);
    assert_ne!(oracle.lines, wrong.lines, "the canonical line sequence");
}

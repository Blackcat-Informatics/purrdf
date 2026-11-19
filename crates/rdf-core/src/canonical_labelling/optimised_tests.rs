// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The optimised engine is held to the reference engine: on every worked
//! example, on the named inputs of the definition under several namings and
//! on generated inputs, the two give the same canonical map, identities, line
//! sequence, digest, structural record and first failures, with the
//! search-free run and with every component labelled by its walk. The stops
//! of 10.4 fire in the walk, the stop signal is honoured, and the limit
//! refuses the same input with the same fields.

use super::optimised;
use super::reference;
use super::tests::{
    blocks, both_directions, corpus, failure_record, hub, nested_instance, quad,
    quadratic_residue_13, record, relabel, term,
};
use super::{Halt, Labelled, Line, Refusal, Term, admit};
use crate::StopSignal;

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

fn optimised_of(lines: &[Line], force_search: bool) -> Labelled {
    let set = admit(lines).expect("well formed");
    optimised::label_set(&set, optimised::ENGINE_LIMIT, force_search, None)
        .unwrap_or_else(|halt| panic!("the optimised engine did not label: {halt:?}"))
}

/// Property: the optimised engine computes the reference result and walks to
/// the reference's totals by kind. INVARIANT held with the search-free run and
/// with every component labelled by its walk (the search forced), so a skip, a
/// stop, a known map or a charge that is wrong on an input shows as a
/// difference from the oracle. A searched result equals the search-free one,
/// failures included (Theorem GR), so one oracle serves both.
fn assert_agrees(context: &str, lines: &[Line]) {
    let set = admit(lines).expect("well formed");
    let (oracle, _, oracle_walk) =
        reference::label_walked(&set, reference::ENGINE_LIMIT, false, false, None)
            .expect("labelled");
    let (result, walk) =
        optimised::label_walked(&set, optimised::ENGINE_LIMIT, false, None).expect("labelled");
    assert_same(context, &oracle, &result);
    assert_eq!(
        (oracle_walk.totals(), oracle_walk.total()),
        (walk.totals(), walk.total()),
        "{context}: the totals of the walk"
    );
    assert_same(
        &format!("{context} searched"),
        &oracle,
        &optimised_of(lines, true),
    );
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
pub(super) fn complement_of_cycles(k1: usize, k2: usize, second_first: bool) -> Vec<Line> {
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

pub(super) fn fifteen() -> Vec<Line> {
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

pub(super) fn cub8() -> Vec<Line> {
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

pub(super) fn k4_pair() -> Vec<Line> {
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
        ("NESTED", nested_instance()),
        ("QR13", quadratic_residue_13("r")),
    ]
}

/// The inputs of the stops and of the skips of section 10, under the namings
/// of 13.2 (seeds 1 to 3): the walk's result equals the oracle's. On
/// `COMPL(4, 6)`, under both namings, a child whose nested call has the
/// smaller `s` and the larger `E` is the smaller certificate, so a stop on a
/// nested call would return a certificate that is not the minimum.
#[test]
fn the_named_inputs_of_section_ten_agree_under_every_naming() {
    for (name, lines) in named_inputs() {
        assert_agrees(name, &lines);
        for seed in 1..=3 {
            assert_agrees(&format!("{name} seed {seed}"), &relabel(&lines, seed));
        }
    }
}

/// 10.4: the walk stops children on `s` on `FIFTEEN` (children of `s = 3`
/// against a best of `s = 2`) and on `E` on `CUB8`, at the branching calls of
/// the result path, when the walk labels them; the oracle, which explores
/// every child, stops none.
#[test]
fn the_strict_stops_fire_in_the_walk() {
    let on_fifteen = optimised_of(&fifteen(), true);
    assert!(on_fifteen.stopped_on_s > 0, "stops on s on FIFTEEN");
    let on_cub8 = optimised_of(&cub8(), true);
    assert!(on_cub8.stopped_on_e > 0, "stops on E on CUB8");
    let oracle = reference_of(&fifteen());
    assert_eq!(oracle.stopped_on_s + oracle.stopped_on_e, 0);
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

/// Property over generated inputs: the optimised engine equals the oracle,
/// for the input as drawn and for a relabelling of it.
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
    let stopped = optimised::label_set(&set, optimised::ENGINE_LIMIT, false, Some(&raised));
    assert!(matches!(stopped, Err(Halt::Stopped)), "{stopped:?}");
    let quiet = Raised(false);
    let done = optimised::label_set(&set, optimised::ENGINE_LIMIT, false, Some(&quiet));
    assert!(done.is_ok());
}

/// 7.10, 7.11 and 8.3: under a limit its walk exceeds, an input that is not
/// guaranteed is refused as `limit` naming the first failure of every
/// component, with the reference's fields; under a large limit the same input
/// is labelled with the same failures; and no limit refuses a guaranteed
/// input.
#[test]
fn the_optimised_limit_refuses_what_the_reference_refuses_and_nothing_guaranteed() {
    let mut lines = quadratic_residue_13("v");
    lines.extend(quadratic_residue_13("w"));
    let set = admit(&lines).expect("well formed");
    let qr = failure_record(0, 13, 13, 78);
    let refused = optimised::label_set(&set, 0, false, None);
    match &refused {
        Err(Halt::Refused(Refusal::Limit {
            broken_by, nodes, ..
        })) => {
            assert_eq!(broken_by, &[qr, qr]);
            assert_eq!(*nodes, 13);
        }
        other => panic!("expected a limit refusal, got {other:?}"),
    }
    let oracle = reference::label_set(&set, 0, false).map(|_| ());
    assert_eq!(refused.map(|_| ()).map_err(Halt::expect_refusal), oracle);
    let labelled =
        optimised::label_set(&set, optimised::ENGINE_LIMIT, false, None).expect("the search fits");
    assert_eq!(labelled.record.broken_by, [qr, qr]);
    let guaranteed = admit(&hub(false)).expect("well formed");
    assert!(optimised::label_set(&guaranteed, 0, false, None).is_ok());
}

/// The twin units are those of the search-free run, in the optimised engine as
/// in the reference, on an input the search-free run does not settle (`QR13`)
/// and on those it does: the twin tests of the walk are charged to the walk
/// and are not counted in the figure.
#[test]
fn the_twin_units_are_those_of_the_search_free_run_whatever_the_walk_does() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        let result = optimised_of(&lines, false);
        assert_eq!(oracle.twin_units, result.twin_units, "{name}: twin units");
    }
}

// ---------------------------------------------------------------------------
// The enumeration spaces of 13.2, each swept whole, with their lines in the
// order of the definition.

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

/// Every non-empty mask of a space of `size` lines (13.2 item (h)).
fn masks(size: u32) -> impl Iterator<Item = u64> {
    1..1u64 << size
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_n3p1() {
    let space = loops_space("y", 3);
    for mask in masks(9) {
        assert_agrees(&format!("N3P1 mask {mask}"), &picked(&space, mask));
    }
}

/// `Q4` (13.2 item (h)): every one of the 65,535 sets, by mask, in four
/// strides run side by side, each against the reference.
fn q4_stride(stride: u64) {
    let space = loops_space("x", 4);
    let mut visited = 0u64;
    for mask in (1..=65_535u64).filter(|m| m % 4 == stride) {
        assert_agrees(&format!("Q4 mask {mask}"), &picked(&space, mask));
        visited += 1;
    }
    assert_eq!(visited, (65_535 - stride) / 4 + 1 - u64::from(stride == 0));
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q4_stride_0() {
    q4_stride(0);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q4_stride_1() {
    q4_stride(1);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q4_stride_2() {
    q4_stride(2);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q4_stride_3() {
    q4_stride(3);
}

/// `Q5DENSE`: the sets of fourteen to twenty of the twenty lines, the only
/// space whose children do not all attain and where the stops fire; every
/// one of its 60,460 sets, in four strides of the enumeration by removed lines.
fn q5dense_stride(stride: u64) {
    let space = q5_lines();
    let (mut seen, mut visited) = (0u64, 0u64);
    for removed in 0..1u64 << 20 {
        if removed.count_ones() > 6 {
            continue;
        }
        seen += 1;
        if (seen - 1) % 4 != stride {
            continue;
        }
        visited += 1;
        let lines = picked(&space, !removed & ((1 << 20) - 1));
        assert_agrees(&format!("Q5DENSE without {removed:#x}"), &lines);
    }
    assert_eq!(seen, 60_460, "the sets of Q5DENSE");
    assert_eq!(visited, (60_460 - stride).div_ceil(4), "the stride's share");
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q5dense_stride_0() {
    q5dense_stride(0);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q5dense_stride_1() {
    q5dense_stride(1);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q5dense_stride_2() {
    q5dense_stride(2);
}

#[test]
fn the_optimised_engine_agrees_on_every_set_of_q5dense_stride_3() {
    q5dense_stride(3);
}

/// `RB3`: reifier bindings, triple terms and annotations; known maps over
/// lines of those kinds. Every one of its 4,095 sets.
#[test]
fn the_optimised_engine_agrees_on_every_set_of_rb3() {
    let space = rb3_lines();
    for mask in masks(12) {
        let lines = picked(&space, mask);
        assert_agrees(&format!("RB3 mask {mask}"), &lines);
        assert_agrees(
            &format!("RB3 mask {mask} relabelled"),
            &relabel(&lines, mask),
        );
    }
}

/// `REC3`: records over the triangle. Every one of its 32,767 sets.
#[test]
fn the_optimised_engine_agrees_on_every_set_of_rec3() {
    let space = rec3_lines();
    for mask in masks(15) {
        let lines = picked(&space, mask);
        assert_agrees(&format!("REC3 mask {mask}"), &lines);
        assert_agrees(
            &format!("REC3 mask {mask} relabelled"),
            &relabel(&lines, mask),
        );
    }
}

// ---------------------------------------------------------------------------
// The work of the walk, the stop signal and ties.

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
            Some(&After(std::sync::atomic::AtomicU64::new(clear_polls))),
        )
    };
    assert!(matches!(run(1), Err(Halt::Stopped)));
    assert!(run(u64::MAX).is_ok());
}

/// The walk only skips and stops children: on the inputs of section 10 it
/// explores no more units than the reference tree, with or without the
/// search-free run.
#[test]
fn the_walk_never_explores_more_units_than_the_reference_tree() {
    for (name, lines) in named_inputs() {
        let oracle = reference_of(&lines);
        for force_search in [false, true] {
            let result = optimised_of(&lines, force_search);
            assert!(
                result.explored.total() <= oracle.explored.total(),
                "{name} searched={force_search}: {} units explored against the reference tree's {}",
                result.explored.total(),
                oracle.explored.total()
            );
        }
    }
}

/// 10.4: a child that ties the best so far is not stopped. On the 2-cycle,
/// labelled by its walk (the search-free run settles it, so the search is
/// forced), the attaining count and the group order are `2` and `2` as in the
/// oracle, and no child is stopped on `E`.
#[test]
fn a_child_that_ties_the_best_is_not_stopped_on_the_two_cycle() {
    let lines = vec![quad("a", "<p>", "b"), quad("b", "<p>", "a")];
    let oracle = reference_of(&lines);
    let result = optimised_of(&lines, true);
    assert!(!differs(&oracle, &result));
    assert_eq!(result.stopped_on_e, 0, "no child stopped on E");
}

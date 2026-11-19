// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The walk of 8.3 in both engines, against the rows `walk`, `limit-rows`
//! and `limit-relabel` of 13.2. The walk decides whether an input is refused
//! with `limit` and the fields of that refusal; its totals are the ones every
//! engine reports, so these figures, totals by kind and the fields of each
//! refusal, are shared with the other engines of the labelling. Every outcome
//! here is computed by both engines and must be the same from each.

use super::optimised_tests::{complement_of_cycles, cub8, fifteen, k4_pair};
use super::reference::Classes;
use super::tests::{
    complete_bipartite_one_direction, cube_both_directions, cycle_with_records, directed_cycle,
    failure_record, hub, quad, quadratic_residue_13, relabel, rename_lines, term, two_cycles,
};
use super::walk::{Kind, Relation, Walk};
use super::{FailureRecord, Labelled, Limit, Line, Refusal, Term, admit};
use super::{optimised, reference};
use purrdf_hash::hex;
use purrdf_xsd::datatype::XSD_STRING;

/// The directed 4-cycle `C4D` of 13.2.
fn c4d() -> Vec<Line> {
    vec![
        quad("a", "<p>", "b"),
        quad("b", "<p>", "c"),
        quad("c", "<p>", "d"),
        quad("d", "<p>", "a"),
    ]
}

/// `C4DX`, the naming of `C4D` that tests opposite nodes.
fn c4dx() -> Vec<Line> {
    vec![
        quad("a", "<p>", "c"),
        quad("c", "<p>", "b"),
        quad("b", "<p>", "d"),
        quad("d", "<p>", "a"),
    ]
}

/// `CUBET`: `CUBE` written with triple terms. For every edge `{v_i, v_j}`,
/// `i < j`, the lines `quad(v_i, <p>, <<(v_j <q> v_i)>>)` and
/// `quad(v_j, <p>, <<(v_i <q> v_j)>>)`.
fn cubet() -> Vec<Line> {
    let mut lines = Vec::new();
    for i in 0..8usize {
        for bit in [1, 2, 4] {
            let j = i ^ bit;
            if i < j {
                let (vi, vj) = (format!("v{i}"), format!("v{j}"));
                for (x, y) in [(&vi, &vj), (&vj, &vi)] {
                    lines.push(Line::Quad {
                        subject: term(x),
                        predicate: term("<p>"),
                        object: Term::triple(term(y), term("<q>"), term(x)),
                        graph: None,
                    });
                }
            }
        }
    }
    lines
}

/// `CUBER`, a comparison input shared with the other engines and not a row of
/// the text: `CUBE`, and for every edge `{v_i, v_j}`, `i < j`, a blank reifier
/// `w_ij` of the triple term `<<(v_i <p> v_j)>>` with the annotation
/// `(2, w_ij, <q>, "x"^^xsd:string)`. It holds reifier bindings and
/// annotations, which no input of the text's rows does.
fn cuber() -> Vec<Line> {
    let mut edges = Vec::new();
    for i in 0..8usize {
        for bit in [1, 2, 4] {
            let j = i ^ bit;
            if i < j {
                edges.push((i, j));
            }
        }
    }
    edges.sort_unstable();
    let mut lines = Vec::new();
    for &(i, j) in &edges {
        let (vi, vj) = (format!("v{i}"), format!("v{j}"));
        lines.push(quad(&vi, "<p>", &vj));
        lines.push(quad(&vj, "<p>", &vi));
    }
    for &(i, j) in &edges {
        let reifier = term(&format!("w{i}{j}"));
        lines.push(Line::ReifierBinding {
            reifier: reifier.clone(),
            triple: Term::triple(term(&format!("v{i}")), term("<p>"), term(&format!("v{j}"))),
            graph: None,
        });
        lines.push(Line::Annotation {
            count: 2,
            reifier,
            predicate: term("<q>"),
            object: Term::typed(b"x", XSD_STRING.as_bytes()),
            graph: None,
        });
    }
    lines
}

/// The ordered pairs of node indices of the `quad` lines of `lines`, whose
/// nodes are a one-byte prefix followed by a decimal index.
fn pairs_of(lines: &[Line]) -> Vec<(usize, usize)> {
    let index = |t: &Term| match t {
        Term::Blank(source) => std::str::from_utf8(&source[1..])
            .expect("a decimal index")
            .parse::<usize>()
            .expect("a decimal index"),
        other => panic!("a blank node, not {other:?}"),
    };
    let mut pairs: Vec<(usize, usize)> = lines
        .iter()
        .filter_map(|line| match line {
            Line::Quad {
                subject, object, ..
            } => Some((index(subject), index(object))),
            _ => None,
        })
        .collect();
    pairs.sort_unstable();
    pairs
}

/// The shared comparison inputs built on a graph of named nodes: its lines,
/// then for each pair `(i, j)` of `pairs` in ascending order a blank reifier
/// `w_ij` (each index with `width` digits) of `<<(x_i <p> x_j)>>`, `x` the
/// node prefix, with the annotation `(2, w_ij, <q>, "x"^^xsd:string)`.
fn reified(
    lines: Vec<Line>,
    prefix: &str,
    node_width: usize,
    pairs: &[(usize, usize)],
    width: usize,
) -> Vec<Line> {
    let node = |i: usize| term(&format!("{prefix}{i:0node_width$}"));
    let mut out = lines;
    for &(i, j) in pairs {
        let reifier = term(&format!("w{i:0width$}{j:0width$}"));
        out.push(Line::ReifierBinding {
            reifier: reifier.clone(),
            triple: Term::triple(node(i), term("<p>"), node(j)),
            graph: None,
        });
        out.push(Line::Annotation {
            count: 2,
            reifier,
            predicate: term("<q>"),
            object: Term::typed(b"x", XSD_STRING.as_bytes()),
            graph: None,
        });
    }
    out
}

/// `COMPLR` (one reifier per edge, `i < j`) or `COMPLRD` (one per ordered
/// pair): shared comparison inputs on `COMPL(3, 5)`.
fn complr(ordered: bool) -> Vec<Line> {
    let lines = complement_of_cycles(3, 5, false);
    let mut pairs = pairs_of(&lines);
    if !ordered {
        pairs.retain(|&(i, j)| i < j);
    }
    reified(lines, "n", 1, &pairs, 1)
}

/// `QR13R` (one reifier per edge) or `QR13RD` (one per ordered pair): shared
/// comparison inputs on `QR13`.
fn qr13r(ordered: bool) -> Vec<Line> {
    let lines = quadratic_residue_13("r");
    let mut pairs = pairs_of(&lines);
    if !ordered {
        pairs.retain(|&(i, j)| i < j);
    }
    reified(lines, "r", 2, &pairs, 2)
}

/// `QR13` with `QR13S`: two top components of equal identity.
fn qr13_with_qr13s() -> Vec<Line> {
    let mut lines = quadratic_residue_13("r");
    lines.extend(quadratic_residue_13("s"));
    lines
}

/// The thirteen inputs of the row `walk`, by their names in 13.2.
fn walk_inputs() -> Vec<(&'static str, Vec<Line>)> {
    vec![
        ("QR13", quadratic_residue_13("r")),
        ("CUBE", cube_both_directions()),
        ("CUB8", cub8()),
        ("COMPL(3,5)", complement_of_cycles(3, 5, false)),
        ("COMPL(4,6)", complement_of_cycles(4, 6, false)),
        ("FIFTEEN", fifteen()),
        ("K4PAIR", k4_pair()),
        ("K33", complete_bipartite_one_direction(3)),
        ("C4D", c4d()),
        ("C4DX", c4dx()),
        ("12.2", two_cycles()),
        ("12.10", cycle_with_records(&["a", "c"])),
        ("CUBET", cubet()),
        ("CUBER (shared comparison input)", cuber()),
        ("COMPLR (shared comparison input)", complr(false)),
        ("QR13R (shared comparison input)", qr13r(false)),
        ("QR13RD (shared comparison input)", qr13r(true)),
        ("COMPLRD (shared comparison input)", complr(true)),
    ]
}

/// The outcome of the reference engine at the limit `limit`, with the walk
/// that decided it when it labels. INVARIANT: the optimised engine follows
/// the same walk, so its outcome is the same: equal totals by kind and
/// equal refusal fields, and when both label, the same result.
fn walked(lines: &[Line], limit: u64) -> Result<(Labelled, Walk), Refusal> {
    let set = admit(lines).expect("well formed");
    let reference = reference::label_walked(&set, limit, false, false, None)
        .map(|(labelled, _, walk)| (labelled, walk))
        .map_err(super::Halt::expect_refusal);
    let optimised =
        optimised::label_walked(&set, limit, false, None).map_err(super::Halt::expect_refusal);
    match (&reference, &optimised) {
        (Ok((r, r_walk)), Ok((o, o_walk))) => {
            assert_eq!(
                (r_walk.totals(), r_walk.total()),
                (o_walk.totals(), o_walk.total()),
                "L = {limit}: the totals of the optimised engine's walk"
            );
            assert_eq!(r.digest, o.digest, "L = {limit}: digest");
            assert_eq!(r.placements, o.placements, "L = {limit}: canonical map");
            assert_eq!(r.record, o.record, "L = {limit}: structural record");
        }
        (Err(r), Err(o)) => assert_eq!(r, o, "L = {limit}: the refusal"),
        (r, o) => panic!(
            "L = {limit}: the reference {} and the optimised engine {}",
            if r.is_ok() { "labels" } else { "refuses" },
            if o.is_ok() { "labels" } else { "refuses" },
        ),
    }
    reference
}

/// One row of `walk`: START, REFINE, FIX-SUM, SPLIT, TWIN, the total and `A`.
fn row(walk: &Walk, allowance: u64) -> [u64; 7] {
    let t = walk.totals();
    [
        t.start,
        t.refine,
        t.fix_sum,
        t.split,
        t.twin,
        walk.total(),
        allowance,
    ]
}

/// Row `walk` of 13.2: the totals by kind of the walk of each input in its
/// given naming, its total and its allowance, equal to the figures the text
/// prints, and those of the shared comparison inputs `CUBER`, `COMPLR`,
/// `QR13R`, `QR13RD` and `COMPLRD`; and the bounds of Lemma W (4): the units (a) to (d) of the walk
/// are at most those of the reference tree, and the total of a guaranteed
/// input is at most its allowance with the twin units of its search-free
/// run. A change in the order of the charges, in a skip, a stop or a known
/// map moves one of these figures.
#[test]
fn the_walk_charges_the_totals_of_the_text() {
    let expected: [(&str, [u64; 7]); 18] = [
        ("QR13", [234, 2_628, 3_770, 0, 1_512, 8_144, 5_473]),
        ("CUBE", [72, 368, 772, 48, 378, 1_638, 1_584]),
        ("CUB8", [72, 500, 710, 0, 354, 1_636, 1_584]),
        ("COMPL(3,5)", [120, 798, 1_764, 44, 780, 3_506, 2_608]),
        ("COMPL(4,6)", [210, 2_106, 4_602, 56, 1_986, 8_960, 4_900]),
        ("FIFTEEN", [45, 190, 275, 0, 192, 702, 910]),
        ("K4PAIR", [72, 388, 452, 24, 264, 1_200, 1_584]),
        ("K33", [27, 63, 78, 8, 36, 212, 510]),
        ("C4D", [12, 40, 72, 0, 30, 154, 232]),
        ("C4DX", [12, 52, 108, 0, 30, 202, 232]),
        ("12.2", [18, 56, 108, 6, 36, 224, 318]),
        ("12.10", [20, 50, 100, 0, 12, 182, 308]),
        ("CUBET", [120, 552, 1_060, 48, 630, 2_410, 2_376]),
        (
            "CUBER (shared comparison input)",
            [240, 717, 1_460, 0, 140, 2_557, 4_028],
        ),
        (
            "COMPLR (shared comparison input)",
            [400, 718, 828, 52, 0, 1_998, 6_676],
        ),
        (
            "QR13R (shared comparison input)",
            [780, 1_470, 1_612, 0, 0, 3_862, 13_832],
        ),
        (
            "QR13RD (shared comparison input)",
            [1_326, 5_736, 10_348, 0, 2_156, 19_566, 23_049],
        ),
        (
            "COMPLRD (shared comparison input)",
            [680, 5_468, 15_531, 0, 1_428, 23_107, 11_184],
        ),
    ];
    let mut got = Vec::new();
    for (name, lines) in walk_inputs() {
        let (labelled, walk) = walked(&lines, u64::MAX).expect("labelled with no limit");
        let figures = row(&walk, labelled.record.allowance);
        let t = walk.totals();
        let limited = t.start + t.refine + t.fix_sum + t.split;
        let set = admit(&lines).expect("well formed");
        let tree = reference::label_set(&set, u64::MAX, true).expect("the reference tree");
        assert!(
            limited <= tree.explored.total(),
            "{name}: {limited} units (a) to (d) above the reference tree's {}",
            tree.explored.total()
        );
        if labelled.record.guaranteed {
            assert!(
                figures[5] <= figures[6],
                "{name}: a guaranteed total above A"
            );
            assert_eq!(t.twin, labelled.twin_units, "{name}: twin units");
        }
        got.push((name, figures));
    }
    assert_eq!(got, expected);
    // `QR13` with `QR13S`: each component walked in full.
    let (_, walk) = walked(&qr13_with_qr13s(), u64::MAX).expect("labelled with no limit");
    assert_eq!(walk.total(), 16_379);
}

/// One outcome of `limit-rows`: labelled, or refused with `source`, `nodes`
/// and `units`.
#[derive(Clone, Copy, Debug)]
enum Outcome {
    Labelled,
    Limit(&'static str, u64, u64),
}

/// One input of `limit-rows` with its outcome at each limit.
type LimitRow = (&'static str, Vec<Line>, Vec<(u64, Outcome)>);

/// `lines` with every blank source identifier preceded by `prefix`.
fn prefixed(lines: &[Line], prefix: &str) -> Vec<Line> {
    rename_lines(lines, &|source: &[u8]| {
        let mut name = prefix.as_bytes().to_vec();
        name.extend_from_slice(source);
        Term::Blank(name.into())
    })
}

/// The outcome of `lines` at `limit` against `expected`, the source of a
/// refusal preceded by `prefix`, `broken_by` that of the labelled record.
fn assert_outcome(name: &str, lines: &[Line], limit: u64, expected: Outcome, prefix: &str) {
    let context = format!("{name} prefix {prefix:?} L = {limit}");
    let broken_by = |l: &Labelled| -> Vec<FailureRecord> { l.record.broken_by.clone() };
    match (walked(lines, limit), expected) {
        (Ok(_), Outcome::Labelled) => {}
        (
            Err(Refusal::Limit {
                broken_by: refused,
                source,
                nodes,
                units,
            }),
            Outcome::Limit(want_source, want_nodes, want_units),
        ) => {
            let (labelled, _) = walked(lines, u64::MAX).expect("labelled with no limit");
            assert_eq!(refused, broken_by(&labelled), "{context}: broken_by");
            assert_eq!(
                (&*source, nodes, units),
                (
                    format!("{prefix}{want_source}").as_bytes(),
                    want_nodes,
                    want_units
                ),
                "{context}"
            );
        }
        (got, want) => panic!(
            "{context}: expected {want:?}, got {:?}",
            got.map(|(l, _)| l.digest)
        ),
    }
}

/// Row `limit-rows` of 13.2: the outcome at each stated limit, every
/// refusal next to the labelled case one unit above it (a refusal is a claim:
/// the neighbouring valid limit must still label). `L = 0` leaves the
/// allowance as the bound. The rows hold again with every source identifier
/// preceded by one byte string, which keeps the order of 2.2. The `CUBET`
/// refusal carries its failure record `(0, 8, 8, 24)`.
#[test]
fn the_limit_refuses_where_the_walk_passes_the_limit() {
    use Outcome::{Labelled, Limit};
    let rows: Vec<LimitRow> = vec![
        (
            "QR13",
            quadratic_residue_13("r"),
            vec![
                (8_144, Labelled),
                (8_143, Limit("r00", 13, 8_144)),
                (0, Limit("r00", 13, 5_918)),
            ],
        ),
        (
            "COMPL(3,5)",
            complement_of_cycles(3, 5, false),
            vec![
                (3_506, Labelled),
                (3_505, Limit("n0", 8, 3_506)),
                (0, Limit("n0", 8, 2_659)),
            ],
        ),
        (
            "COMPL(4,6)",
            complement_of_cycles(4, 6, false),
            vec![
                (8_960, Labelled),
                (8_959, Limit("n00", 10, 8_960)),
                (0, Limit("n00", 10, 5_210)),
            ],
        ),
        (
            "QR13 with QR13S",
            qr13_with_qr13s(),
            vec![
                (16_379, Labelled),
                (16_378, Limit("s00", 13, 16_379)),
                (0, Limit("s00", 13, 11_110)),
            ],
        ),
        (
            "CUBE",
            cube_both_directions(),
            vec![
                (1_638, Labelled),
                (1_637, Limit("v0", 8, 1_638)),
                (0, Limit("v0", 8, 1_638)),
            ],
        ),
        (
            "CUB8",
            cub8(),
            vec![
                (1_636, Labelled),
                (1_635, Limit("w0", 8, 1_636)),
                (0, Limit("w0", 8, 1_610)),
            ],
        ),
        (
            "CUBET",
            cubet(),
            vec![
                (2_410, Labelled),
                (2_409, Limit("v0", 8, 2_410)),
                (0, Limit("v0", 8, 2_410)),
            ],
        ),
        ("FIFTEEN", fifteen(), vec![(0, Labelled)]),
        ("K4PAIR", k4_pair(), vec![(0, Labelled)]),
        ("C4D", c4d(), vec![(0, Labelled)]),
        ("C4DX", c4dx(), vec![(0, Labelled)]),
        (
            "K33",
            complete_bipartite_one_direction(3),
            vec![(0, Labelled)],
        ),
        ("12.2", two_cycles(), vec![(0, Labelled)]),
        (
            "12.10",
            cycle_with_records(&["a", "c"]),
            vec![(0, Labelled)],
        ),
    ];
    for (name, lines, outcomes) in rows {
        for prefix in ["", "pre/"] {
            let lines = if prefix.is_empty() {
                lines.clone()
            } else {
                prefixed(&lines, prefix)
            };
            for &(limit, outcome) in &outcomes {
                assert_outcome(name, &lines, limit, outcome, prefix);
            }
        }
    }
    let (cubet, _) = walked(&cubet(), u64::MAX).expect("labelled with no limit");
    assert_eq!(cubet.record.broken_by, [failure_record(0, 8, 8, 24)]);
}

/// Row `limit-relabel` of 13.2: the walk is a function of the input as given,
/// so its total moves with the source identifiers, while the labelled result
/// does not. The totals of the twenty relabellings of `COMPL(3, 5)`, `CUB8`
/// and `QR13`; at `L = 3,506` the relabellings of `COMPL(3, 5)` whose total
/// passes it are refused (seeds 5 and 7 with their stated fields) and the six
/// others are labelled; at `L = 0` every relabelling of `CUB8` and `QR13` is
/// refused, its total being above its allowance.
#[test]
fn the_limit_moves_with_the_naming_and_the_result_does_not() {
    let compl: [u64; 20] = [
        3506, 3506, 3566, 3506, 4539, 3566, 4539, 3566, 3506, 3566, 4539, 3506, 3506, 3566, 4539,
        3566, 4539, 3566, 3566, 4539,
    ];
    let cub8_totals: [u64; 20] = [
        1636, 1636, 1636, 1636, 2650, 2084, 3106, 3106, 2538, 1636, 2658, 2538, 1636, 2090, 2538,
        2090, 3106, 1636, 2090, 2090,
    ];
    let qr13: [u64; 20] = [
        8144, 7202, 8144, 8144, 7202, 6260, 7202, 8144, 8144, 7202, 6260, 8144, 7202, 6260, 7202,
        8144, 8144, 6260, 6260, 8144,
    ];
    let inputs: [(&str, Vec<Line>, [u64; 20]); 3] = [
        ("COMPL(3,5)", complement_of_cycles(3, 5, false), compl),
        ("CUB8", cub8(), cub8_totals),
        ("QR13", quadratic_residue_13("r"), qr13),
    ];
    for (name, lines, totals) in inputs {
        let (base, _) = walked(&lines, u64::MAX).expect("labelled");
        let mut got = Vec::new();
        for seed in 1..=20u64 {
            let renamed = relabel(&lines, seed);
            let (labelled, walk) = walked(&renamed, u64::MAX).expect("labelled");
            assert_eq!(labelled.digest, base.digest, "{name} seed {seed}");
            assert_eq!(labelled.record, base.record, "{name} seed {seed}");
            got.push(walk.total());
            if name == "COMPL(3,5)" {
                let outcome = walked(&renamed, 3_506);
                if [1, 2, 4, 9, 12, 13].contains(&seed) {
                    assert!(outcome.is_ok(), "{name} seed {seed} at L = 3,506");
                } else {
                    let Err(Refusal::Limit {
                        source,
                        nodes,
                        units,
                        ..
                    }) = outcome
                    else {
                        panic!("{name} seed {seed}: expected a limit refusal at L = 3,506");
                    };
                    if [5, 7].contains(&seed) {
                        assert_eq!(
                            (&*source, nodes, units),
                            (&[0u8; 8][..], 8, 3_510),
                            "{name} seed {seed}"
                        );
                    }
                }
            } else {
                assert!(
                    matches!(walked(&renamed, 0), Err(Refusal::Limit { .. })),
                    "{name} seed {seed} at L = 0"
                );
            }
        }
        assert_eq!(got, totals, "{name}");
    }
}

/// The public entry returns the walk's totals by kind: on `QR13` those of the
/// row `walk` of 13.2, their sum the printed total, with the result `label`
/// gives; and the limit it is given decides the refusal, one unit below the
/// total refused with the total as its units and the total itself labelled.
#[test]
fn the_public_entry_returns_the_totals_of_the_walk() {
    let lines = quadratic_residue_13("r");
    let (_, labelled, totals) =
        super::label_with_walk(&lines, Limit::ENGINE).expect("labelled under the engine limit");
    assert_eq!(
        [
            totals.start,
            totals.refine,
            totals.fix_sum,
            totals.split,
            totals.twin
        ],
        [234, 2_628, 3_770, 0, 1_512]
    );
    assert_eq!(totals.total(), 8_144);
    let (_, plain) = super::label(&lines).expect("labelled");
    assert_eq!(labelled.digest, plain.digest);
    assert!(super::label_with_walk(&lines, Limit::new(8_144)).is_ok());
    match super::label_with_walk(&lines, Limit::new(8_143)) {
        Err(Refusal::Limit { units, .. }) => assert_eq!(units, 8_144),
        other => panic!(
            "expected a limit refusal, got {:?}",
            other.map(|(_, l, _)| l.digest)
        ),
    }
}

/// The limit test at the saturation boundary (7.11, 8.3 step 6): `R` passes
/// `max(L, A)` exactly one unit above it; a charge that saturates `R` trips
/// the walk with `units = 2^64 - 1` and `R` stays there; a saturated allowance
/// disables the trip whatever is charged; and a limit of `2^64 - 1` is never
/// passed, even by a saturated `R`. Each refusal sits beside the neighbouring
/// charge that must still pass.
#[test]
fn the_walk_trips_exactly_above_its_bound_and_saturates() {
    let mut walk = Walk::new(5, 0, 0);
    assert!(walk.charge(Kind::Twin, 5).is_ok(), "R = bound passes");
    assert!(walk.charge(Kind::Twin, 1).is_err(), "R = bound + 1 trips");

    let mut walk = Walk::new(0, 10, 0);
    assert!(walk.charge(Kind::Twin, 10).is_ok());
    assert!(walk.charge(Kind::Twin, u64::MAX).is_err());
    assert!(
        walk.charge(Kind::Twin, u64::MAX).is_err(),
        "R stays saturated"
    );
    assert_eq!(walk.total(), u64::MAX);
    assert_eq!(walk.totals().twin, u64::MAX);
    match walk.refusal(b"s", Vec::new()) {
        Refusal::Limit { units, .. } => assert_eq!(units, u64::MAX),
        other => panic!("expected a limit refusal, got {other:?}"),
    }

    let mut walk = Walk::new(0, u64::MAX, 0);
    assert!(
        walk.charge(Kind::Twin, u64::MAX).is_ok(),
        "A saturated: no trip"
    );
    assert!(walk.charge(Kind::Refine, 1).is_ok());
    assert_eq!(walk.total(), u64::MAX);

    let mut walk = Walk::new(u64::MAX, 10, 0);
    assert!(
        walk.charge(Kind::Twin, u64::MAX).is_ok(),
        "L saturated: never passed"
    );
    assert!(walk.charge(Kind::Twin, 1).is_ok());
}

/// The walk ends inside a component the search-free run labels: `QR13`, then a
/// guaranteed hub whose sources sort after it, the hub's step charged after
/// `QR13`'s. One unit below the total, the refusal names the hub, whose step
/// made the last charge, with the units of that charge; at the total the input
/// is labelled. An engine that charges the hub's step whole must find the
/// same charge.
#[test]
fn the_walk_can_end_inside_a_component_the_search_free_run_labels() {
    let mut lines = quadratic_residue_13("a");
    lines.extend(prefixed(&hub(false), "z"));
    let (labelled, walk) = walked(&lines, u64::MAX).expect("labelled with no limit");
    assert_eq!(labelled.record.broken_by.len(), 1, "QR13 alone fails");
    let total = walk.total();
    assert!(total > labelled.record.allowance, "the walk passes A");
    assert!(walked(&lines, total).is_ok(), "labelled at the total");
    match walked(&lines, total - 1) {
        Err(Refusal::Limit { source, units, .. }) => {
            assert!(source.starts_with(b"z"), "the hub's step: {source:?}");
            assert_eq!(units, total, "the last charge of the hub's step");
        }
        other => panic!(
            "expected a limit refusal, got {:?}",
            other.map(|(l, _)| l.digest)
        ),
    }
}

/// The shared comparison input `CUBER`: not guaranteed (its first failure
/// is at the call that fixes two nodes), its walk total `2,557` below its
/// allowance `4,028`, so it is labelled at every limit, `0` and the total
/// itself included, with the digest the other engines give; its first five
/// relabellings walk to the same total and twin units and label to the same
/// digest.
#[test]
fn the_shared_input_cuber_is_labelled_at_every_limit() {
    let lines = cuber();
    let (labelled, walk) = walked(&lines, u64::MAX).expect("labelled with no limit");
    assert!(!labelled.record.guaranteed);
    assert_eq!(labelled.record.broken_by, [failure_record(2, 6, 20, 48)]);
    assert_eq!(
        hex::encode(&labelled.digest),
        "d0dc836e6ec438226c09ed3b849c08cf85c1c38054d29859cf979cd7484dc6e9"
    );
    assert_eq!(walk.total(), 2_557);
    for limit in [0, 2_556, 2_557] {
        let (again, _) = walked(&lines, limit).expect("labelled at every limit");
        assert_eq!(again.digest, labelled.digest, "L = {limit}");
    }
    for seed in 1..=5 {
        let (again, walk) = walked(&relabel(&lines, seed), u64::MAX).expect("labelled");
        assert_eq!(
            (walk.total(), walk.totals().twin),
            (2_557, 140),
            "seed {seed}"
        );
        assert_eq!(again.digest, labelled.digest, "seed {seed}");
    }
}

/// The shared comparison inputs with reifiers on `COMPL(3, 5)` and `QR13`:
/// their record, digest and outcome at each stated limit, and the totals of
/// their first five relabellings, every relabelling labelled to the same
/// digest. `COMPLR` and `QR13R` are guaranteed; `QR13RD` is not and stays
/// below its allowance; `COMPLRD` passes its allowance and is the refused row,
/// each refusal next to the labelled limit one unit above it.
#[test]
fn the_shared_inputs_with_reifiers_walk_to_their_figures() {
    use Outcome::{Labelled, Limit};
    type Case = (
        &'static str,
        Vec<Line>,
        Vec<FailureRecord>,
        &'static str,
        Vec<(u64, Outcome)>,
        Option<[u64; 5]>,
    );
    let cases: Vec<Case> = vec![
        (
            "COMPLR",
            complr(false),
            vec![],
            "ef79c35b1de94a222f9c723c48908c246582684da3fc7e12d20a3fb28691d2ec",
            vec![(0, Labelled), (1_997, Labelled), (1_998, Labelled)],
            Some([1_998; 5]),
        ),
        (
            "QR13R",
            qr13r(false),
            vec![],
            "de4d35e2cf8068210abc33f3eae5e015db16d2464ea842e4e65e3aa3d0a7e33a",
            vec![(0, Labelled), (3_861, Labelled), (3_862, Labelled)],
            Some([3_862; 5]),
        ),
        (
            "QR13RD",
            qr13r(true),
            vec![failure_record(0, 78, 91, 234)],
            "6ab7e9cea0bb5d9e930e35d6efd2557560101e41491a14c174deec45de49c68b",
            vec![(0, Labelled)],
            None,
        ),
        (
            "COMPLRD",
            complr(true),
            vec![failure_record(0, 40, 48, 120)],
            "edf826a95d4f64ed9e0d33f1662492da26f99203d91076139b69b24bb267ea17",
            vec![
                (23_107, Labelled),
                (23_106, Limit("n0", 48, 23_107)),
                (0, Limit("n0", 48, 12_348)),
            ],
            Some([13_990, 18_139, 13_171, 19_154, 23_107]),
        ),
    ];
    for (name, lines, broken_by, digest, outcomes, relabelled) in cases {
        let (labelled, _) = walked(&lines, u64::MAX).expect("labelled with no limit");
        assert_eq!(labelled.record.broken_by, broken_by, "{name}");
        assert_eq!(labelled.record.guaranteed, broken_by.is_empty(), "{name}");
        assert_eq!(hex::encode(&labelled.digest), digest, "{name}");
        for (limit, outcome) in outcomes {
            assert_outcome(name, &lines, limit, outcome, "");
        }
        if let Some(totals) = relabelled {
            let mut got = Vec::new();
            for seed in 1..=5 {
                let (again, walk) = walked(&relabel(&lines, seed), u64::MAX).expect("labelled");
                assert_eq!(again.digest, labelled.digest, "{name} seed {seed}");
                got.push(walk.total());
            }
            assert_eq!(got, totals, "{name}");
        }
    }
}

/// The relation of (K1) over a call that visits every member, as a directed
/// cycle of 2,000 blank nodes makes: the relation is needed at each visit, and
/// the twin classes change only when a twin test passes, so one pass serves
/// every visit until a join of the classes (by the call or a nested one)
/// changes them. A pass relates each member to the first of its class and no
/// other; a rescan per member is cubic over the visits.
#[test]
fn the_twin_relation_is_read_in_passes_not_per_member_scans() {
    const N: usize = 2_000;
    let target: Vec<usize> = (0..N).collect();
    let mut tw = Classes::new(N);
    let mut relation = Relation::new(N);
    for visit in 0..N {
        relation.twins(&target, &mut tw);
        assert!(!relation.skipped(visit), "no twins yet");
    }
    assert_eq!(relation.passes, 1, "no join of TW, one pass");

    // Members 0, 5 and 7 become one class, 3 and 4 another.
    let (a, b) = (tw.find(0), tw.find(5));
    tw.join(a, b);
    let (a, b) = (tw.find(0), tw.find(7));
    tw.join(a, b);
    let (a, b) = (tw.find(3), tw.find(4));
    tw.join(a, b);
    for _ in 0..N {
        relation.twins(&target, &mut tw);
    }
    assert_eq!(relation.passes, 2, "joins of TW, one more pass");
    let skipped: Vec<usize> = (0..N).filter(|&i| relation.skipped(i)).collect();
    assert_eq!(
        skipped,
        [4, 5, 7],
        "each later member of a class is skipped"
    );

    // A neighbouring input labels the same in both engines, and the 2,000-node
    // cycle labels in the optimised engine (the reference engine is the oracle
    // for small inputs).
    walked(&directed_cycle(12), Limit::ENGINE.get()).expect("the neighbour labels");
    let set = admit(&directed_cycle(N)).expect("well formed");
    optimised::label_walked(&set, Limit::ENGINE.get(), false, None)
        .map_err(super::Halt::expect_refusal)
        .expect("a directed cycle of 2,000 nodes labels");
}

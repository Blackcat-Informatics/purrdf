// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Work and allocation counts of the canonical labelling over named shapes,
//! in a separate process. Counts only: the units of the definition's section 8
//! (explored, twin), the calls the stops of 10.4 ended, the structural record's
//! rule counts, and the allocation counts and bytes of the whole labelling
//! (admission and search), read from the counting allocator. No clock is read,
//! and nothing here asserts a threshold: the report is for comparison across
//! changes of the engine. Set `PURRDF_LABELLING_BENCH_SCALE` to scale the
//! sizes of the paths, triangles and hub (default 1).
//!
//! With `--counts` (Linux and `perf`), the rows `path`, `quadratic_residue_13`,
//! `cube`, `z4_z4` and `claims` run in fresh child processes under the test
//! kit's counter protocol and print retired user-mode instructions beside the
//! allocation line of `label`; `PURRDF_LABELLING_COUNT_NODES` sets the path
//! (1,000 nodes by default) and `PURRDF_LABELLING_COUNT_SAMPLES` the samples.
//! `claims` is four messages of 20 claims, each a blank reifier of a triple
//! term with 6 to 8 annotations, the first three claims of a message being
//! like siblings: a guaranteed input that runs no search.

use std::hint::black_box;

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_core::canonical_labelling::{Line, Term, admit, label, label_reference};
use purrdf_testkit::bench::counter::{CountRows, CountWork, run_count_rows};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn blank(name: &str) -> Term {
    Term::blank(name.as_bytes())
}

fn iri(name: &str) -> Term {
    Term::iri(name.as_bytes())
}

fn quad(s: &str, p: &str, o: &str) -> Line {
    Line::Quad {
        subject: blank(s),
        predicate: Term::iri(p.as_bytes()),
        object: blank(o),
        graph: None,
    }
}

fn both(edges: &[(String, String)]) -> Vec<Line> {
    edges
        .iter()
        .flat_map(|(x, y)| [quad(x, "p", y), quad(y, "p", x)])
        .collect()
}

fn path(n: usize) -> Vec<Line> {
    (0..n - 1)
        .map(|i| quad(&format!("n{i}"), "p", &format!("n{}", i + 1)))
        .collect()
}

fn triangles(n: usize) -> Vec<Line> {
    (0..n)
        .flat_map(|i| (0..3).map(move |j| (i, j)))
        .map(|(i, j)| quad(&format!("t{i}_{j}"), "p", &format!("t{i}_{}", (j + 1) % 3)))
        .collect()
}

fn hub(n: usize) -> Vec<Line> {
    (0..n).map(|i| quad("h", "p", &format!("l{i}"))).collect()
}

fn cube() -> Vec<Line> {
    let mut edges = Vec::new();
    for i in 0..8usize {
        for bit in [1, 2, 4] {
            if i < i ^ bit {
                edges.push((format!("v{i}"), format!("v{}", i ^ bit)));
            }
        }
    }
    both(&edges)
}

fn z4_z4() -> Vec<Line> {
    let mut lines = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            for (di, dj) in [(1, 0), (3, 0), (0, 1), (0, 3), (1, 1), (3, 3)] {
                lines.push(quad(
                    &format!("v{i}{j}"),
                    "p",
                    &format!("v{}{}", (i + di) % 4, (j + dj) % 4),
                ));
            }
        }
    }
    lines
}

fn complement_of_cycles(k1: usize, k2: usize) -> Vec<Line> {
    let n = k1 + k2;
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
                lines.push(quad(&format!("n{x:02}"), "p", &format!("n{y:02}")));
            }
        }
    }
    lines
}

fn quadratic_residue_13() -> Vec<Line> {
    let squares: Vec<usize> = (1..13).map(|x| x * x % 13).collect();
    let mut edges = Vec::new();
    for u in 0..13 {
        for v in u + 1..13 {
            if squares.contains(&((v - u) % 13)) {
                edges.push((format!("r{u:02}"), format!("r{v:02}")));
            }
        }
    }
    both(&edges)
}

fn cub8() -> Vec<Line> {
    let pairs = [
        (0, 1),
        (0, 4),
        (0, 5),
        (1, 6),
        (1, 7),
        (2, 3),
        (2, 4),
        (2, 6),
        (3, 5),
        (3, 6),
        (4, 7),
        (5, 7),
    ];
    both(
        &pairs
            .iter()
            .map(|&(a, b)| (format!("w{a}"), format!("w{b}")))
            .collect::<Vec<_>>(),
    )
}

/// Four messages of 20 claims. Claim `c` of message `m` is a blank reifier of
/// `<<(s_m_c p o_m_c)>>` with `6 + c % 3` annotations, held by the message
/// node; the first three claims of a message share one triple and one set of
/// annotations, so they are like siblings.
fn claims() -> Vec<Line> {
    let mut lines = Vec::new();
    for m in 0..4 {
        let message = format!("m{m}");
        for c in 0..20usize {
            let reifier = format!("m{m}c{c:02}");
            let key = if c < 3 { 0 } else { c };
            lines.push(Line::Quad {
                subject: blank(&message),
                predicate: iri("claim"),
                object: blank(&reifier),
                graph: None,
            });
            lines.push(Line::ReifierBinding {
                reifier: blank(&reifier),
                triple: Term::triple(
                    iri(&format!("s{m}_{key}")),
                    iri("p"),
                    iri(&format!("o{m}_{key}")),
                ),
                graph: None,
            });
            for a in 0..6 + key % 3 {
                lines.push(Line::Annotation {
                    count: 1,
                    reifier: blank(&reifier),
                    predicate: iri(&format!("q{a}")),
                    object: iri(&format!("v{m}_{key}_{a}")),
                    graph: None,
                });
            }
        }
    }
    lines
}

fn prepare(row: &str, size: usize) -> CountWork {
    let lines = match row {
        "path" => {
            assert!(
                size >= 1,
                "PURRDF_LABELLING_COUNT_NODES must be at least 1, got {size}"
            );
            path(size)
        }
        "quadratic_residue_13" => quadratic_residue_13(),
        "cube" => cube(),
        "z4_z4" => z4_z4(),
        "claims" => claims(),
        other => panic!("unknown row {other}"),
    };
    Box::new(move || {
        let (_, labelled) = label(&lines).expect("the shape is labelled");
        u64::from_le_bytes(labelled.digest[..8].try_into().expect("eight bytes"))
    })
}

const COUNT_ROWS: CountRows<'static> = CountRows {
    bench: "canonical_labelling",
    rows: &["path", "quadratic_residue_13", "cube", "z4_z4", "claims"],
    size: ("PURRDF_LABELLING_COUNT_NODES", 1000),
    samples: "PURRDF_LABELLING_COUNT_SAMPLES",
};

fn scale() -> usize {
    std::env::var("PURRDF_LABELLING_BENCH_SCALE")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&value| value >= 1)
        .unwrap_or(1)
}

fn measure(name: &str, lines: &[Line]) {
    let set = admit(lines).expect("the shape is well formed");
    let nodes = set.blank_nodes();
    drop(set);
    let window = WholeProcessWindow::open();
    let result = black_box(label(black_box(lines)));
    let measured = window.close();
    let (set, labelled) = result.expect("the shape is labelled");
    let record = &labelled.record;
    // The oracle explores every child of every branching call it searches:
    // its explored units are the reference tree's.
    let (_, oracle) = label_reference(lines).expect("the oracle labels the shape");
    assert_eq!(oracle.digest, labelled.digest, "{name}: engines agree");
    println!(
        "[canonical_labelling] {name}: nodes={nodes} lines={} guaranteed={} \
         rule3_calls={} explored_units={} reference_explored_units={} twin_units={} \
         path_units={} allowance={} stopped_on_s={} stopped_on_e={} allocations={} \
         requested_bytes={} retained_bytes={} peak_working_bytes={}",
        set.len(),
        record.guaranteed,
        record.calls.branch,
        labelled.explored.total(),
        oracle.explored.total(),
        labelled.twin_units,
        record.units.total(),
        record.allowance,
        labelled.stopped_on_s,
        labelled.stopped_on_e,
        measured.allocations,
        measured.requested_bytes,
        measured.retained_bytes,
        measured.peak_working_bytes,
    );
}

fn main() {
    if std::env::args().any(|arg| arg == "--counts" || arg == "--child") {
        run_count_rows(
            &COUNT_ROWS,
            &prepare,
            &purrdf_alloc_probe::whole_process_report,
        );
        return;
    }
    let scale = scale();
    for (name, lines) in [
        ("path", path(1_000 * scale)),
        ("triangles", triangles(1_000 * scale)),
        ("hub_with_leaves", hub(1_000 * scale)),
        ("complement_of_cycles_4_6", complement_of_cycles(4, 6)),
        ("complement_of_cycles_3_5", complement_of_cycles(3, 5)),
        ("quadratic_residue_13", quadratic_residue_13()),
        ("cub8", cub8()),
        ("z4_z4", z4_z4()),
    ] {
        measure(name, &lines);
    }
}

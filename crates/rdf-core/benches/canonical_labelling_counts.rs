// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Allocations, requested bytes and retired user-mode instructions of
//! [`label`] over fixed shapes.
//!
//! `cargo bench -p purrdf-core --bench canonical_labelling_counts [-- <row filter>]`
//! prints one allocation line per row, measured in process. With `--counts`
//! (Linux and `perf`) each row also runs in fresh child processes under the
//! test kit's counter protocol and prints its instruction samples. Rows:
//!
//! * `path`: a directed path of `PURRDF_LABELLING_COUNT_NODES` nodes (1,000
//!   by default), labelled with no search.
//! * `qr13`: the quadratic-residue graph on 13 nodes, a search.
//! * `cube`: the 3-cube with both directions of every edge, a search.
//! * `z4_z4`: the Cayley graph of `Z4 x Z4`, a search with a large group.
//! * `claims`: four messages of 20 claims each. A claim is a blank reifier of a
//!   triple term over named nodes with 6 to 8 annotations; the first three
//!   claims of a message are like siblings. The input is guaranteed: no
//!   search runs.
//!
//! The shapes are built outside every window. The counting allocator also
//! runs in the instruction samples; its work is the same for every build
//! compared.

use purrdf_alloc_probe::CountingAllocator;
use purrdf_core::canonical_labelling::{Line, Term, label};
use purrdf_testkit::bench::counter::{CountRows, CountWork, run_count_rows};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

const ROWS: [&str; 5] = ["path", "qr13", "cube", "z4_z4", "claims"];

fn blank(name: &str) -> Term {
    Term::blank(name.as_bytes())
}

fn iri(name: &str) -> Term {
    Term::iri(name.as_bytes())
}

fn quad(s: &str, p: &str, o: &str) -> Line {
    Line::Quad {
        subject: blank(s),
        predicate: iri(p),
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
        "path" => path(size),
        "qr13" => quadratic_residue_13(),
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

const SPEC: CountRows<'static> = CountRows {
    bench: "canonical_labelling_counts",
    rows: &ROWS,
    size: ("PURRDF_LABELLING_COUNT_NODES", 1000),
    samples: "PURRDF_LABELLING_COUNT_SAMPLES",
};

fn main() {
    run_count_rows(&SPEC, &prepare, &purrdf_alloc_probe::whole_process_report);
}

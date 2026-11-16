// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The worked examples of the definition byte for byte, its stated facts
//! about the guaranteed class, and the laws every result obeys: REBUILD of
//! each certificate relabels to the same certificate, seeded relabellings and
//! line orders change nothing but the canonical map, the search agrees with
//! the search-free run, and every ill-formed input gets its typed refusal.
//!
//! The files under `vectors/` are the value blocks of section 12 of the
//! definition, copied verbatim. The examples test renders each input in the
//! same notation and compares the text whole, so every certificate, entry,
//! refinement step, unit, record field, position, line and digest is checked.

use std::fmt::Write as _;

use purrdf_hash::hex;

use super::lines::Leaf;
use super::reference::{self, ComponentTrace, Step, StepKind, Tree};
use super::{
    Factor, FactorKind, FailureRecord, GraphTerm, InputReason, Labelled, Line, LineSet, Refusal,
    Term, admit, rebuild,
};

/// A term in the examples' notation: `<x>` is the IRI of the bytes `x`, any
/// other name is the blank node of that source identifier.
pub(super) fn term(name: &str) -> Term {
    match name.strip_prefix('<').and_then(|n| n.strip_suffix('>')) {
        Some(iri) => Term::iri(iri.as_bytes()),
        None => Term::blank(name.as_bytes()),
    }
}

pub(super) fn quad(s: &str, p: &str, o: &str) -> Line {
    Line::Quad {
        subject: term(s),
        predicate: term(p),
        object: term(o),
        graph: None,
    }
}

fn quads(spec: &[(&str, &str, &str)]) -> Vec<Line> {
    spec.iter().map(|&(s, p, o)| quad(s, p, o)).collect()
}

// ---------------------------------------------------------------------------
// The inputs of section 12.

fn path() -> Vec<Line> {
    quads(&[("a", "<p>", "b"), ("b", "<p>", "c"), ("c", "<p>", "d")])
}

fn two_cycles() -> Vec<Line> {
    quads(&[
        ("a", "<p>", "b"),
        ("b", "<p>", "c"),
        ("c", "<p>", "a"),
        ("x", "<p>", "y"),
        ("y", "<p>", "z"),
        ("z", "<p>", "x"),
    ])
}

fn hub_with_triple_term() -> Vec<Line> {
    let mut lines = quads(&[("h", "<p>", "l1"), ("h", "<p>", "l2"), ("h", "<p>", "l3")]);
    lines.push(Line::Quad {
        subject: term("l1"),
        predicate: term("<q>"),
        object: Term::triple(term("l2"), term("<r>"), term("l1")),
        graph: Some(term("h")),
    });
    lines
}

/// `G1` of 12.4 and 12.6 on the nodes in the roles `a1 a2 b1 b2`.
fn g1(names: [&str; 4]) -> Vec<Line> {
    let [a1, a2, b1, b2] = names;
    quads(&[
        (a1, "<p>", b1),
        (a1, "<p>", b2),
        (a2, "<p>", b1),
        (a2, "<p>", b2),
    ])
}

/// `G2` of 12.6 on the nodes in the roles `u v1 v2 c1 c2`.
fn g2(names: [&str; 5]) -> Vec<Line> {
    let [u, v1, v2, c1, c2] = names;
    quads(&[
        (u, "<p>", c1),
        (v1, "<p>", c1),
        (u, "<p>", c2),
        (v2, "<p>", c2),
    ])
}

fn leaves() -> Vec<Line> {
    quads(&[
        ("<s>", "<p>", "x"),
        ("<s>", "<p>", "y"),
        ("a", "<q>", "b"),
        ("b", "<q>", "a"),
    ])
}

fn g1_and_g2(first: [&str; 4], second: [&str; 5]) -> Vec<Line> {
    let mut lines = g1(first);
    lines.extend(g2(second));
    lines
}

fn every_kind() -> Vec<Line> {
    vec![
        quad("s", "<p>", "o"),
        Line::ReifierBinding {
            reifier: term("r"),
            triple: Term::triple(term("s"), term("<p>"), term("o")),
            graph: None,
        },
        Line::Annotation {
            count: 2,
            reifier: term("r"),
            predicate: term("<q>"),
            object: Term::typed(b"v", b"t"),
            graph: None,
        },
        Line::EmptyGraph(term("g")),
        quad("g", "<q>", "r"),
        Line::EmptyGraph(term("<h>")),
    ]
}

/// `record("payload", [terms])` in the examples' notation.
pub(super) fn record(payload: &str, terms: &[Term]) -> Line {
    Line::Record {
        payload: payload.as_bytes().into(),
        terms: terms.to_vec(),
    }
}

/// The directed 4-cycle of 12.8 to 12.10, with the records on the nodes named.
fn cycle_with_records(on: &[&str]) -> Vec<Line> {
    let mut lines = quads(&[
        ("a", "<p>", "b"),
        ("b", "<p>", "c"),
        ("c", "<p>", "d"),
        ("d", "<p>", "a"),
    ]);
    lines.extend(on.iter().map(|&n| record("m", &[term(n)])));
    lines
}

fn empty_records() -> Vec<Line> {
    let mut lines = quads(&[("a", "<p>", "b"), ("b", "<p>", "a")]);
    lines.push(record("n", &[]));
    lines.push(record("", &[term("a")]));
    lines
}

fn record_only_nodes() -> Vec<Line> {
    vec![
        quad("a", "<p>", "b"),
        record(
            "t",
            &[term("x"), Term::triple(term("y"), term("<p>"), term("x"))],
        ),
    ]
}

/// Each block of section 12: its file, its input, and whether it is printed
/// in full or abbreviated (identities, map, lines and digest only).
pub(super) fn blocks() -> Vec<(&'static str, &'static str, Vec<Line>, bool)> {
    let mut five_b = leaves();
    five_b.push(quad("<s>", "<p>", "xm"));
    let roles_g1 = ["a1", "a2", "b1", "b2"];
    let roles_g2 = ["u", "v1", "v2", "c1", "c2"];
    vec![
        (
            "example-1",
            include_str!("vectors/example-1.txt"),
            path(),
            true,
        ),
        (
            "example-2",
            include_str!("vectors/example-2.txt"),
            two_cycles(),
            true,
        ),
        (
            "example-3",
            include_str!("vectors/example-3.txt"),
            hub_with_triple_term(),
            true,
        ),
        (
            "example-4",
            include_str!("vectors/example-4.txt"),
            g1(roles_g1),
            true,
        ),
        (
            "example-5",
            include_str!("vectors/example-5.txt"),
            leaves(),
            true,
        ),
        (
            "example-5b",
            include_str!("vectors/example-5b.txt"),
            five_b,
            false,
        ),
        (
            "example-6",
            include_str!("vectors/example-6.txt"),
            g1_and_g2(roles_g1, roles_g2),
            true,
        ),
        (
            "example-6/2",
            include_str!("vectors/example-6-2.txt"),
            g1_and_g2(["z1", "z2", "z3", "z4"], roles_g2),
            false,
        ),
        (
            "example-6/3",
            include_str!("vectors/example-6-3.txt"),
            g1_and_g2(["m", "a", "z", "c"], ["y", "b", "n", "x", "d"]),
            false,
        ),
        (
            "example-7",
            include_str!("vectors/example-7.txt"),
            every_kind(),
            true,
        ),
        (
            "example-8",
            include_str!("vectors/example-8.txt"),
            cycle_with_records(&["a"]),
            true,
        ),
        (
            "example-9",
            include_str!("vectors/example-9.txt"),
            cycle_with_records(&["a", "b"]),
            true,
        ),
        (
            "example-10",
            include_str!("vectors/example-10.txt"),
            cycle_with_records(&["a", "c"]),
            true,
        ),
        (
            "example-11",
            include_str!("vectors/example-11.txt"),
            empty_records(),
            true,
        ),
        (
            "example-12",
            include_str!("vectors/example-12.txt"),
            record_only_nodes(),
            true,
        ),
    ]
}

// ---------------------------------------------------------------------------
// The block notation of section 12.

/// The number of hexadecimal digits on one line of a printed value.
const WRAP: usize = 96;

/// `prefix` and `value`, continued on following lines that start in the
/// column of the value's first digit.
fn wrapped(out: &mut String, prefix: &str, value: &str) {
    let indent = " ".repeat(prefix.len());
    let mut rest = value;
    let mut first = true;
    loop {
        let (head, tail) = rest.split_at(rest.len().min(WRAP));
        let lead = if first { prefix } else { &indent };
        let _ = writeln!(out, "{lead}{head}");
        first = false;
        rest = tail;
        if rest.is_empty() {
            break;
        }
    }
}

fn list<T>(items: &[T], item: impl Fn(&T) -> String) -> String {
    let inner: Vec<String> = items.iter().map(item).collect();
    format!("[{}]", inner.join(", "))
}

fn factors(group: &[Factor]) -> String {
    list(group, |f| {
        let kind = match f.kind {
            FactorKind::Attain => "attain",
            FactorKind::Sym => "sym",
        };
        format!("({kind}, {}, {})", f.value, f.exponent)
    })
}

fn failure(f: &FailureRecord) -> String {
    format!(
        "({}, {}, {}, {})",
        f.path_position, f.tied_cell, f.component_nodes, f.component_lines
    )
}

/// A traced top component with the names its notation prints.
struct Names<'a> {
    set: &'a LineSet,
    trace: &'a ComponentTrace,
}

impl Names<'_> {
    fn node(&self, local: usize) -> String {
        String::from_utf8_lossy(self.set.source(self.trace.nodes[local])).into_owned()
    }

    /// Local nodes in source order, by name.
    fn nodes(&self, cell: &[usize]) -> Vec<String> {
        let mut cell = cell.to_vec();
        cell.sort_by(|&a, &b| {
            self.set
                .source(self.trace.nodes[a])
                .cmp(self.set.source(self.trace.nodes[b]))
        });
        cell.into_iter().map(|v| self.node(v)).collect()
    }

    fn step(&self, step: &Step) -> String {
        let kind = match step.kind {
            StepKind::Start => "start".to_owned(),
            StepKind::Indiv(v) => format!("indiv({})", self.node(v)),
            StepKind::Pop(side, colour) => format!("pop({side},{colour})"),
        };
        let mut nodes = String::new();
        for cell in &step.nodes {
            let _ = write!(nodes, "[{}]", self.nodes(cell).join(","));
        }
        let mut lines = String::new();
        for cell in &step.lines {
            let mut global: Vec<usize> = cell.iter().map(|&l| self.trace.lines[l]).collect();
            global.sort_unstable();
            let names: Vec<String> = global.iter().map(|g| format!("L{}", g + 1)).collect();
            let _ = write!(lines, "[{}]", names.join(","));
        }
        let queue = if step.queue.is_empty() {
            "-".to_owned()
        } else {
            step.queue.iter().fold(String::new(), |mut out, (s, c)| {
                let _ = write!(out, "({s},{c})");
                out
            })
        };
        format!("{kind} N={nodes} L={lines} Q={queue}")
    }

    fn fixes(&self, tree: &Tree) -> String {
        let fixes: Vec<String> = tree
            .fixed
            .iter()
            .map(|&(v, q)| format!("{} {q}", self.node(v)))
            .collect();
        fixes.join(", ")
    }

    /// A call of the call structure at `indent`, every child expanded.
    fn call(&self, tree: &Tree, indent: usize, out: &mut String) {
        let pad = " ".repeat(indent);
        if tree.rule != 3 {
            self.fix_and_nest(tree, indent, out);
            return;
        }
        let _ = writeln!(
            out,
            "{pad}call: rule 3, p {}, target cell {}, attaining {}, one twin class {}",
            tree.p,
            self.nodes(&tree.target).join(" "),
            tree.attaining,
            tree.one_twin_class
        );
        for (k, child) in tree.children.iter().enumerate() {
            let _ = writeln!(out, "{pad}  child {}:", self.node(child.v));
            for step in &child.steps {
                let _ = writeln!(out, "{pad}      {}", self.step(step));
            }
            let earlier = tree.children[..k]
                .iter()
                .find(|other| other.tree.cert == child.tree.cert);
            if let Some(earlier) = earlier {
                let _ = writeln!(
                    out,
                    "{pad}    certificate equal to that of child {}; fixes {}",
                    self.node(earlier.v),
                    self.fixes(&child.tree)
                );
            } else {
                let certificate = hex::encode(&child.tree.cert.encoding());
                wrapped(out, &format!("{pad}    certificate "), &certificate);
                self.fix_and_nest(&child.tree, indent + 4, out);
            }
        }
    }

    /// A call that took rule 1 or 2, or a child.
    fn fix_and_nest(&self, tree: &Tree, indent: usize, out: &mut String) {
        let pad = " ".repeat(indent);
        let _ = writeln!(
            out,
            "{pad}rule {}, p {}, s {}, fixes {}; units REFINE {}, FIX {}, ENTRIES {}",
            tree.rule,
            tree.p,
            tree.fixed.len(),
            self.fixes(tree),
            tree.units.refine,
            tree.units.fix,
            tree.units.entries
        );
        for entry in &tree.cert.entries {
            let prefix = format!("{pad}  entry m {}: ", entry.m);
            wrapped(out, &prefix, &hex::encode(&entry.encoding));
        }
        for nested in &tree.nested {
            let prefix = format!("{pad}  nested call, certificate ");
            wrapped(out, &prefix, &hex::encode(&nested.cert.encoding()));
            self.call(nested, indent + 4, out);
        }
    }
}

/// The block of `lines` in the notation of section 12.
fn render(lines: &[Line], full: bool) -> String {
    let set = admit(lines).expect("well formed");
    let plain = reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("labelled");
    let (searched, traces) =
        reference::label_traced(&set, reference::ENGINE_LIMIT, true, true, None).expect("labelled");
    assert_eq!(
        searched.record, plain.record,
        "the search agrees with the run"
    );
    assert_eq!(searched.lines, plain.lines);
    let mut out = String::new();
    let mut top_of = vec![0usize; set.blank_nodes()];
    for (t, ((component, trace), record)) in plain
        .components
        .iter()
        .zip(&traces)
        .zip(&plain.record.components)
        .enumerate()
    {
        for &g in &trace.nodes {
            top_of[g] = t;
        }
        let names = Names { set: &set, trace };
        let all: Vec<usize> = (0..trace.nodes.len()).collect();
        let _ = writeln!(
            out,
            "top component {t} (offset {}): nodes {}",
            component.offset,
            names.nodes(&all).join(" ")
        );
        if !full {
            let _ = writeln!(out, "  identity    {}", hex::encode(&component.identity));
            let _ = writeln!(out, "  copy index  {}", component.copy_index);
            continue;
        }
        let _ = writeln!(
            out,
            "  start line cells, encodings with every leaf free, in colour order:"
        );
        for cell in &trace.steps[0].lines {
            let mut encoding = Vec::new();
            set.encode(trace.lines[cell[0]], |_| Leaf::Free, &mut encoding);
            wrapped(&mut out, "    ", &hex::encode(&encoding));
        }
        let _ = writeln!(out, "  refinement of the fresh start:");
        for step in &trace.steps {
            let _ = writeln!(out, "    {}", names.step(step));
        }
        let _ = writeln!(out, "  call structure:");
        names.call(&trace.tree, 4, &mut out);
        wrapped(
            &mut out,
            "  certificate ",
            &hex::encode(&component.certificate.encoding()),
        );
        let _ = writeln!(out, "  identity    {}", hex::encode(&component.identity));
        let _ = writeln!(out, "  copy index  {}", component.copy_index);
        let _ = writeln!(
            out,
            "  allowance   S {}, O {}, M {}, V {}, W {}, A {}",
            record.units.start,
            record.label_elements,
            record.incidences,
            record.blank_nodes + record.lines,
            component.w,
            record.allowance
        );
        let _ = writeln!(out, "  twin units  {}", component.twin_units);
        let _ = writeln!(out, "  record:");
        let r = record;
        let units =
            |u: &super::Units| format!("[{}, {}, {}, {}]", u.start, u.refine, u.fix, u.entries);
        let fields = [
            ("blank_nodes", r.blank_nodes.to_string()),
            ("lines", r.lines.to_string()),
            ("label_elements", r.label_elements.to_string()),
            ("incidences", r.incidences.to_string()),
            ("line_bytes", r.line_bytes.to_string()),
            ("singled_out", r.singled_out.to_string()),
            ("unsettled", r.unsettled.to_string()),
            (
                "largest_cell",
                r.largest_cell
                    .map_or_else(|| "absent".to_owned(), |c| format!("present({c})")),
            ),
            ("forest", r.forest.to_string()),
            (
                "calls",
                format!(
                    "[{}, {}, {}]",
                    r.calls.discrete, r.calls.nest, r.calls.branch
                ),
            ),
            (
                "chosen_children",
                format!(
                    "[{}, {}]",
                    r.chosen_children.discrete, r.chosen_children.nest
                ),
            ),
            ("target_sizes", list(&r.target_sizes, u64::to_string)),
            ("attaining", list(&r.attaining, u64::to_string)),
            ("nestings", list(&r.nestings, |n| list(n, u64::to_string))),
            ("nesting_depth", r.nesting_depth.to_string()),
            ("branching_depth", r.branching_depth.to_string()),
            ("units", units(&r.units)),
            ("allowance", r.allowance.to_string()),
            ("group_order", factors(&r.group_order)),
        ];
        for (name, value) in fields {
            let _ = writeln!(out, "    {name} {value}");
        }
    }
    let _ = writeln!(
        out,
        "positions and stable identities (top component, local position, copy index):"
    );
    let mut by_position: Vec<usize> = (0..set.blank_nodes()).collect();
    by_position.sort_by_key(|&b| plain.placements[b].position);
    for b in by_position {
        let p = plain.placements[b];
        let _ = writeln!(
            out,
            "  {}  position {}  ({}, {}, {})",
            String::from_utf8_lossy(set.source(b)),
            p.position,
            top_of[b],
            p.identity.local_position,
            p.identity.copy_index
        );
    }
    let _ = writeln!(out, "canonical line sequence:");
    for (i, line) in plain.lines.iter().enumerate() {
        wrapped(&mut out, &format!("  {i}  "), &hex::encode(line));
    }
    let _ = writeln!(out, "digest {}", hex::encode(&plain.digest));
    if !full {
        return out;
    }
    let r = &plain.record;
    let _ = writeln!(out, "record of the whole input:");
    let fields = [
        ("top_components", r.top_components.to_string()),
        (
            "size_histogram",
            list(&r.size_histogram, |(k, c)| format!("({k}, {c})")),
        ),
        (
            "single_node_components",
            r.single_node_components.to_string(),
        ),
        ("blank_nodes", r.blank_nodes.to_string()),
        ("label_elements", r.label_elements.to_string()),
        ("incidences", r.incidences.to_string()),
        ("singled_out", r.singled_out.to_string()),
        ("unsettled", r.unsettled.to_string()),
        ("blank_bearing_lines", r.blank_bearing_lines.to_string()),
        ("blank_bearing_bytes", r.blank_bearing_bytes.to_string()),
        (
            "calls",
            format!(
                "[{}, {}, {}]",
                r.calls.discrete, r.calls.nest, r.calls.branch
            ),
        ),
        (
            "chosen_children",
            format!(
                "[{}, {}]",
                r.chosen_children.discrete, r.chosen_children.nest
            ),
        ),
        (
            "units",
            format!(
                "[{}, {}, {}, {}]",
                r.units.start, r.units.refine, r.units.fix, r.units.entries
            ),
        ),
        ("allowance", r.allowance.to_string()),
        (
            "top_multiplicities",
            list(&r.top_multiplicities, u64::to_string),
        ),
        ("group_order", factors(&r.group_order)),
        ("broken_by", list(&r.broken_by, failure)),
        ("guaranteed", r.guaranteed.to_string()),
    ];
    for (name, value) in fields {
        let _ = writeln!(out, "  {name} {value}");
    }
    let diagnostic = list(&plain.diagnostic, |d| {
        format!(
            "({}, {}, {})",
            failure(&d.record),
            String::from_utf8_lossy(&d.component_source),
            String::from_utf8_lossy(&d.cell_source)
        )
    });
    let _ = writeln!(out, "diagnostic {diagnostic}");
    let e = searched.explored;
    let _ = writeln!(
        out,
        "units over the reference tree: START {}, REFINE {}, FIX {}, ENTRIES {}; comparison units {}",
        e.start, e.refine, e.fix, e.entries, searched.comparison_units
    );
    out
}

/// Every value block of section 12, rendered from its input, equals the
/// block of the definition byte for byte.
#[test]
fn every_worked_example_reproduces_byte_for_byte() {
    let mut mismatches = Vec::new();
    for (name, expected, lines, full) in blocks() {
        let got = render(&lines, full);
        if got != expected {
            let line = got
                .lines()
                .zip(expected.lines())
                .position(|(g, e)| g != e)
                .unwrap_or_else(|| got.lines().count().min(expected.lines().count()));
            mismatches.push(format!(
                "{name}: first difference at line {}\n  got:      {:?}\n  expected: {:?}",
                line + 1,
                got.lines().nth(line),
                expected.lines().nth(line)
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

// ---------------------------------------------------------------------------
// The facts of 7.10 and the shared corpus of 13.2.

fn label(lines: &[Line]) -> (LineSet, Labelled) {
    let set = admit(lines).expect("well formed");
    let labelled = reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("labelled");
    (set, labelled)
}

/// The order a factorisation list denotes: `attain` contributes
/// `value^exponent` and `sym` contributes `(value!)^exponent` (Lemma G).
fn group_order(group: &[Factor]) -> u64 {
    group
        .iter()
        .map(|f| {
            let base = match f.kind {
                FactorKind::Attain => f.value,
                FactorKind::Sym => (1..=f.value).product(),
            };
            base.pow(u32::try_from(f.exponent).expect("small exponent"))
        })
        .product()
}

/// `quad(x, <p>, y)` and `quad(y, <p>, x)` for every edge `{x, y}`.
pub(super) fn both_directions(edges: &[(String, String)]) -> Vec<Line> {
    let mut lines = Vec::new();
    for (x, y) in edges {
        lines.push(quad(x, "<p>", y));
        lines.push(quad(y, "<p>", x));
    }
    lines
}

fn named(prefix: &str, i: usize) -> String {
    format!("{prefix}{i}")
}

fn complete_bipartite_one_direction(side: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    for i in 0..side {
        for j in 0..side {
            lines.push(quad(&named("a", i), "<p>", &named("b", j)));
        }
    }
    lines
}

fn twisted_bipartite() -> Vec<Line> {
    let mut lines = Vec::new();
    for i in 0..3 {
        let (a, b, next) = (named("a", i), named("b", i), named("a", (i + 1) % 3));
        lines.push(quad(&a, "<p>", &b));
        lines.push(quad(&b, "<p>", &next));
        lines.push(quad(&a, "<p>", &next));
    }
    lines
}

pub(super) fn hub(triangle: bool) -> Vec<Line> {
    let mut lines = quads(&[("h", "<p>", "a"), ("h", "<p>", "b"), ("h", "<p>", "c")]);
    if triangle {
        lines.extend(quads(&[
            ("a", "<q>", "b"),
            ("b", "<q>", "c"),
            ("c", "<q>", "a"),
        ]));
    }
    lines
}

fn complete_both_directions(n: usize) -> Vec<Line> {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            edges.push((named("n", i), named("n", j)));
        }
    }
    both_directions(&edges)
}

fn bipartite_both_directions() -> Vec<Line> {
    let mut edges = Vec::new();
    for i in 0..2 {
        for j in 0..2 {
            edges.push((named("a", i), named("b", j)));
        }
    }
    both_directions(&edges)
}

fn directed_cycle(n: usize) -> Vec<Line> {
    (0..n)
        .map(|i| quad(&named("n", i), "<p>", &named("n", (i + 1) % n)))
        .collect()
}

fn cube_both_directions() -> Vec<Line> {
    let mut edges = Vec::new();
    for u in 0..8usize {
        for bit in [1, 2, 4] {
            let v = u ^ bit;
            if u < v {
                edges.push((named("v", u), named("v", v)));
            }
        }
    }
    both_directions(&edges)
}

pub(super) fn quadratic_residue_13(prefix: &str) -> Vec<Line> {
    let squares: Vec<usize> = (1..13).map(|x| x * x % 13).collect();
    let mut edges = Vec::new();
    for u in 0..13 {
        for v in u + 1..13 {
            if squares.contains(&((v - u) % 13)) {
                edges.push((named(prefix, u), named(prefix, v)));
            }
        }
    }
    both_directions(&edges)
}

fn reifier_shape() -> Vec<Line> {
    vec![
        quad("s", "<p>", "o"),
        Line::ReifierBinding {
            reifier: term("r"),
            triple: Term::triple(term("s"), term("<p>"), term("o")),
            graph: None,
        },
        Line::Annotation {
            count: 1,
            reifier: term("r"),
            predicate: term("<q>"),
            object: term("<v>"),
            graph: None,
        },
    ]
}

/// The nested instance of 13.2: `G1` and two copies of `G2` hung under `h`.
fn nested_instance() -> Vec<Line> {
    let mut lines = g1(["a1", "a2", "b1", "b2"]);
    lines.extend(quads(&[("h", "<q>", "b1"), ("h", "<q>", "b2")]));
    lines.extend(g2(["u", "v1", "v2", "c1", "c2"]));
    lines.extend(quads(&[("h", "<q>", "c1"), ("h", "<q>", "c2")]));
    lines.extend(g2(["U", "V1", "V2", "C1", "C2"]));
    lines.extend(quads(&[("h", "<q>", "C1"), ("h", "<q>", "C2")]));
    lines
}

pub(super) fn failure_record(p: u64, tied: u64, nodes: u64, lines: u64) -> FailureRecord {
    FailureRecord {
        path_position: p,
        tied_cell: tied,
        component_nodes: nodes,
        component_lines: lines,
    }
}

/// One fact of 7.10: the input, its first failures, its target sizes when
/// stated, and its group order when stated (0 when not).
type FactCase = (
    &'static str,
    Vec<Line>,
    Vec<FailureRecord>,
    Option<Vec<u64>>,
    u64,
);

/// The facts of 7.10: which inputs are guaranteed, their first failures,
/// their target sizes and the orders of their automorphism groups. The root
/// refinement does not decide the class: the first two inputs and the two
/// hubs share their root statistics.
#[test]
fn the_facts_of_the_guaranteed_class_hold() {
    // (input, guaranteed failures, target sizes or None, group order)
    let cases: Vec<FactCase> = vec![
        (
            "K3,3 one direction",
            complete_bipartite_one_direction(3),
            vec![],
            Some(vec![3, 2]),
            36,
        ),
        (
            "twisted K3,3",
            twisted_bipartite(),
            vec![failure_record(0, 3, 6, 9)],
            None,
            0,
        ),
        ("hub", hub(false), vec![], None, 6),
        (
            "hub with triangle",
            hub(true),
            vec![failure_record(1, 3, 4, 6)],
            None,
            0,
        ),
        ("2-cycle", directed_cycle(2), vec![], Some(vec![2]), 2),
        (
            "K4 both directions",
            complete_both_directions(4),
            vec![],
            Some(vec![4, 3, 2]),
            24,
        ),
        (
            "leaves on a singled-out hub",
            quads(&[("h", "<q>", "<s>"), ("h", "<p>", "l1"), ("h", "<p>", "l2")]),
            vec![],
            Some(vec![]),
            2,
        ),
        (
            "K2,2 both directions",
            bipartite_both_directions(),
            vec![failure_record(0, 4, 4, 8)],
            None,
            8,
        ),
        (
            "directed 4-cycle",
            directed_cycle(4),
            vec![failure_record(0, 4, 4, 4)],
            None,
            0,
        ),
        (
            "cube both directions",
            cube_both_directions(),
            vec![],
            Some(vec![8, 3]),
            48,
        ),
        (
            "QR(13) both directions",
            quadratic_residue_13("v"),
            vec![failure_record(0, 13, 13, 78)],
            None,
            0,
        ),
        ("reifier shape", reifier_shape(), vec![], Some(vec![]), 1),
    ];
    for (name, lines, failures, targets, order) in cases {
        let (_, l) = label(&lines);
        let r = &l.record;
        if name == "cube both directions" {
            assert!(!r.guaranteed, "{name}");
        } else {
            assert_eq!(r.broken_by, failures, "{name}");
            assert_eq!(r.guaranteed, failures.is_empty(), "{name}");
        }
        if let Some(targets) = targets {
            assert_eq!(r.components[0].target_sizes, targets, "{name}");
        }
        if order != 0 {
            assert_eq!(group_order(&r.group_order), order, "{name}");
        }
    }
    // The two pairs that share their root statistics.
    let (_, a) = label(&complete_bipartite_one_direction(3));
    let (_, b) = label(&twisted_bipartite());
    let root = |l: &Labelled| {
        let c = &l.record.components[0];
        (
            c.blank_nodes,
            c.lines,
            c.label_elements,
            c.singled_out,
            c.unsettled,
            c.largest_cell,
            c.forest,
        )
    };
    assert_eq!(root(&a), (6, 9, 18, 0, 6, Some(3), false));
    assert_eq!(root(&a), root(&b));
}

/// The certificate encoding with `s` left out at every level: the pair
/// `(E, C)` of 6.8, which does not determine the component.
fn without_s(certificate: &super::Certificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(certificate.entries.len() as u64).to_be_bytes());
    for entry in &certificate.entries {
        out.extend_from_slice(&entry.m.to_be_bytes());
        out.extend_from_slice(&entry.encoding);
    }
    out.extend_from_slice(&(certificate.nested.len() as u64).to_be_bytes());
    for nested in &certificate.nested {
        out.extend(without_s(nested));
    }
    out
}

/// The nested instance of 13.2: the top call fixes `h` and nests three calls
/// of 4, 5 and 5 nodes whose pairs `(E, C)` are equal; its group has order 32.
/// Only `s` orders them: the two calls on copies of `G2` fix one node
/// (`s = 1`) and precede the call on `G1`, which fixes two.
#[test]
fn the_nested_instance_has_group_order_thirty_two() {
    let (_, l) = label(&nested_instance());
    assert_eq!(l.components.len(), 1);
    let c = &l.record.components[0];
    assert_eq!(c.blank_nodes, 15);
    assert_eq!(group_order(&c.group_order), 32);
    let top = &l.components[0].certificate;
    assert_eq!(top.s, 1);
    let sizes: Vec<u64> = top.nested.iter().map(super::Certificate::size).collect();
    assert_eq!(sizes, [5, 5, 4]);
    let fixes: Vec<u64> = top.nested.iter().map(|n| n.s).collect();
    assert_eq!(fixes, [1, 1, 2]);
    let pairs: Vec<Vec<u8>> = top.nested.iter().map(without_s).collect();
    assert!(pairs.windows(2).all(|w| w[0] == w[1]));
    let whole: Vec<Vec<u8>> = top
        .nested
        .iter()
        .map(super::Certificate::encoding)
        .collect();
    assert!(whole.windows(2).any(|w| w[0] != w[1]));
}

/// 7.10 and 7.11: the search-free run covers every top component before any
/// search, so a `limit` refusal names the first failure of every component
/// that is not guaranteed, not only of the one being searched. Under a limit
/// that the search fits the same input is labelled with the same failures,
/// and no limit refuses a guaranteed input.
#[test]
fn a_limit_refusal_names_the_first_failure_of_every_component() {
    let mut lines = quadratic_residue_13("v");
    lines.extend(quadratic_residue_13("w"));
    let set = admit(&lines).expect("well formed");
    let qr = failure_record(0, 13, 13, 78);
    // The engine limit 0 leaves the allowance as the limit, which a search of
    // a component with no twins exceeds.
    match reference::label_set(&set, 0, false) {
        Err(Refusal::Limit {
            broken_by, nodes, ..
        }) => {
            assert_eq!(broken_by, [qr, qr]);
            assert_eq!(nodes, 13);
        }
        other => panic!("expected a limit refusal, got {other:?}"),
    }
    let labelled =
        reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("the search fits");
    assert_eq!(labelled.record.broken_by, [qr, qr]);
    let guaranteed = admit(&hub(false)).expect("well formed");
    assert!(reference::label_set(&guaranteed, 0, false).is_ok());
}

/// Every input of the shared corpus this module runs.
pub(super) fn corpus() -> Vec<Vec<Line>> {
    let mut out: Vec<Vec<Line>> = blocks().into_iter().map(|(_, _, lines, _)| lines).collect();
    out.extend([
        complete_bipartite_one_direction(3),
        twisted_bipartite(),
        hub(false),
        hub(true),
        directed_cycle(2),
        complete_both_directions(4),
        bipartite_both_directions(),
        directed_cycle(4),
        cube_both_directions(),
        quadratic_residue_13("v"),
        reifier_shape(),
        nested_instance(),
    ]);
    out
}

/// Theorem RB: labelling `rebuild(K)` alone gives `cert(K)` again, and the
/// node `u64(i)` of the rebuilt lines takes the position `i`.
#[test]
fn every_certificate_rebuilds_to_itself() {
    for lines in corpus() {
        let (_, l) = label(&lines);
        for component in &l.components {
            let rebuilt = rebuild(&component.certificate).expect("rebuild");
            let again =
                reference::label_set(&rebuilt, reference::ENGINE_LIMIT, false).expect("label");
            assert_eq!(again.components.len(), 1);
            assert_eq!(again.components[0].certificate, component.certificate);
            for (node, placement) in again.placements.iter().enumerate() {
                let local = u64::from_be_bytes(rebuilt.source(node).try_into().expect("u64"));
                assert_eq!(placement.position, local);
            }
        }
    }
}

/// `PERM(n)` of 13.2 over the `MIX64` stream: Fisher-Yates from the top,
/// each draw the high half of the 128-bit product.
fn perm(n: usize, state: &mut u64) -> Vec<usize> {
    let mut out: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        let r = purrdf_hash::mix::splitmix64_next(state);
        let j = usize::try_from((u128::from(r) * (i as u128 + 1)) >> 64).expect("below n");
        out.swap(i, j);
    }
    out
}

fn rename_term(term: &Term, rename: &dyn Fn(&[u8]) -> Term) -> Term {
    match term {
        Term::Blank(source) => rename(source),
        Term::Triple(parts) => Term::triple(
            rename_term(&parts[0], rename),
            rename_term(&parts[1], rename),
            rename_term(&parts[2], rename),
        ),
        other => other.clone(),
    }
}

/// The lines with the node numbered `k` (ascending source identifier)
/// renamed `u64(pi[k])`, and the line at place `i` the input's line
/// `tau[i]`, both permutations drawn from `S(seed)`.
pub(super) fn relabel(lines: &[Line], seed: u64) -> Vec<Line> {
    let set = admit(lines).expect("well formed");
    let mut sources: Vec<&[u8]> = (0..set.blank_nodes()).map(|b| set.source(b)).collect();
    sources.sort_unstable();
    let mut state = seed;
    let pi = perm(sources.len(), &mut state);
    let tau = perm(lines.len(), &mut state);
    let rename = |source: &[u8]| {
        let k = sources.binary_search(&source).expect("node");
        Term::Blank((pi[k] as u64).to_be_bytes().into())
    };
    let renamed = rename_lines(lines, &rename);
    tau.into_iter().map(|i| renamed[i].clone()).collect()
}

/// `lines` with every blank leaf, at any depth, renamed by `rename`.
fn rename_lines(lines: &[Line], rename: &dyn Fn(&[u8]) -> Term) -> Vec<Line> {
    let r = |t: &Term| rename_term(t, rename);
    lines
        .iter()
        .map(|line| match line {
            Line::Quad {
                subject,
                predicate,
                object,
                graph,
            } => Line::Quad {
                subject: r(subject),
                predicate: r(predicate),
                object: r(object),
                graph: graph.as_ref().map(r),
            },
            Line::ReifierBinding {
                reifier,
                triple,
                graph,
            } => Line::ReifierBinding {
                reifier: r(reifier),
                triple: r(triple),
                graph: graph.as_ref().map(r),
            },
            Line::Annotation {
                count,
                reifier,
                predicate,
                object,
                graph,
            } => Line::Annotation {
                count: *count,
                reifier: r(reifier),
                predicate: r(predicate),
                object: r(object),
                graph: graph.as_ref().map(r),
            },
            Line::EmptyGraph(g) => Line::EmptyGraph(r(g)),
            Line::Record { payload, terms } => Line::Record {
                payload: payload.clone(),
                terms: terms.iter().map(r).collect(),
            },
        })
        .collect()
}

/// Lemma I (7.9): an input whose blank nodes are renamed `u64(pos(b))`, a
/// naming increasing in the source order, is labelled by the identity map,
/// whatever its line order: the node named `u64(i)` takes the position `i`
/// and the identity of the node of position `i`, and the canonical line
/// sequence, digest, certificates and record are those of the input.
#[test]
fn an_input_renamed_by_its_positions_is_labelled_by_the_identity() {
    for lines in corpus() {
        let (set, first) = label(&lines);
        let rename = |source: &[u8]| {
            let node = (0..set.blank_nodes())
                .find(|&b| set.source(b) == source)
                .expect("node");
            Term::Blank(first.placements[node].position.to_be_bytes().into())
        };
        let mut renamed = rename_lines(&lines, &rename);
        renamed.reverse();
        let (again_set, again) = label(&renamed);
        let mut by_position = vec![None; set.blank_nodes()];
        for (node, placement) in first.placements.iter().enumerate() {
            by_position[usize::try_from(placement.position).expect("small")] = Some(node);
        }
        for (node, placement) in again.placements.iter().enumerate() {
            let i = u64::from_be_bytes(again_set.source(node).try_into().expect("u64 name"));
            assert_eq!(placement.position, i);
            let original = by_position[usize::try_from(i).expect("small")].expect("position");
            assert_eq!(placement.identity, first.placements[original].identity);
        }
        assert_eq!(again.lines, first.lines);
        assert_eq!(again.digest, first.digest);
        assert_eq!(again.record, first.record);
        let certificates = |l: &Labelled| -> Vec<_> {
            l.components.iter().map(|c| c.certificate.clone()).collect()
        };
        assert_eq!(certificates(&again), certificates(&first));
    }
}

/// Twin units are determined by the input as given, not by its isomorphism
/// class: which pairs the search-free run tests depends on the source order.
/// Two namings of one directed 4-cycle test adjacent nodes (three lines hold
/// them: 9 units) or opposite ones (four lines: 12 units), while the record,
/// the canonical line sequence and the digest, which hold no twin unit, are
/// equal.
#[test]
fn twin_units_follow_the_source_order_and_nothing_else_does() {
    let adjacent = quads(&[
        ("a", "<p>", "b"),
        ("b", "<p>", "c"),
        ("c", "<p>", "d"),
        ("d", "<p>", "a"),
    ]);
    let opposite = quads(&[
        ("a", "<p>", "c"),
        ("c", "<p>", "b"),
        ("b", "<p>", "d"),
        ("d", "<p>", "a"),
    ]);
    let ((_, first), (_, second)) = (label(&adjacent), label(&opposite));
    assert_eq!((first.twin_units, second.twin_units), (9, 12));
    assert_eq!(first.record, second.record);
    assert_eq!(first.lines, second.lines);
    assert_eq!(first.digest, second.digest);
}

/// The empty input (3.3) is well formed and labelled: no component, an empty
/// canonical line sequence with the digest of `u64(0)`, zero in every number
/// of the record, the empty list in every list, and `guaranteed`.
#[test]
fn the_empty_input_is_labelled() {
    let (_, l) = label(&[]);
    assert!(l.lines.is_empty() && l.components.is_empty() && l.placements.is_empty());
    assert_eq!(
        hex::encode(&l.digest),
        "bf4b40a613c1cc97ababeccefc1c1c810dde52818d8928603f2180376f06fb5e"
    );
    let empty = super::StatisticsRecord {
        guaranteed: true,
        ..super::StatisticsRecord::default()
    };
    assert_eq!(l.record, empty);
}

/// Property 1 (test `relabel`): twenty relabellings with line reorderings
/// give the same canonical line sequence, digest, structural record and
/// identities on every input of the corpus.
#[test]
fn relabelling_and_line_order_change_only_the_map() {
    for lines in corpus() {
        let (_, base) = label(&lines);
        let identities = |l: &Labelled| {
            let mut ids: Vec<_> = l
                .placements
                .iter()
                .map(|p| (p.identity.component, p.identity.local_position))
                .collect();
            ids.sort_unstable();
            ids
        };
        for seed in 1..=20 {
            let (_, other) = label(&relabel(&lines, seed));
            assert_eq!(other.lines, base.lines, "seed {seed}");
            assert_eq!(other.digest, base.digest, "seed {seed}");
            assert_eq!(other.record, base.record, "seed {seed}");
            assert_eq!(identities(&other), identities(&base), "seed {seed}");
        }
    }
}

/// Theorem GR: where the search-free run labels an input, a search of every
/// child gives the same result; where it fails, the first failures it
/// records are those read from the searched result path.
#[test]
fn the_search_agrees_with_the_search_free_run() {
    for lines in corpus() {
        let set = admit(&lines).expect("well formed");
        let free = reference::label_set(&set, reference::ENGINE_LIMIT, false).expect("labelled");
        let full = reference::label_set(&set, reference::ENGINE_LIMIT, true).expect("labelled");
        assert_eq!(full.lines, free.lines);
        assert_eq!(full.record, free.record);
        assert_eq!(full.placements, free.placements);
        assert_eq!(full.diagnostic, free.diagnostic);
    }
}

/// 3.3 and 7.11: every ill-formed input gets its typed refusal with the
/// precedence of the smallest code, and a neighbouring well-formed input
/// passes.
#[test]
fn ill_formed_inputs_get_their_typed_refusal_and_neighbours_pass() {
    let refusal = |lines: &[Line]| admit(lines).err();
    // A predicate that is not an IRI, against the same line with an IRI.
    let slot = Some(Refusal::Input {
        reason: InputReason::Slot,
    });
    assert_eq!(refusal(&[quad("a", "b", "c")]), slot);
    assert_eq!(refusal(&[quad("a", "<p>", "c")]), None);
    // A reifier binding whose triple slot holds no triple term.
    let binding = |triple| Line::ReifierBinding {
        reifier: term("r"),
        triple,
        graph: None,
    };
    assert_eq!(refusal(&[binding(term("<t>"))]), slot);
    assert_eq!(
        refusal(&[binding(Term::triple(term("s"), term("<p>"), term("o")))]),
        None
    );
    // Count zero against count one.
    let annotation = |count| Line::Annotation {
        count,
        reifier: term("r"),
        predicate: term("<p>"),
        object: term("o"),
        graph: None,
    };
    assert_eq!(
        refusal(&[annotation(0)]),
        Some(Refusal::Input {
            reason: InputReason::Count
        })
    );
    assert_eq!(refusal(&[annotation(1)]), None);
    // Two lines equal but for the count; one of them alone passes.
    let repeated = Some(Refusal::Input {
        reason: InputReason::Repeated,
    });
    assert_eq!(refusal(&[annotation(1), annotation(2)]), repeated);
    assert_eq!(
        refusal(&[quad("a", "<p>", "c"), quad("a", "<p>", "c")]),
        repeated
    );
    // A record is compared whole, the order of its list included; the kind
    // is part of the line.
    let (a, b) = (term("a"), term("b"));
    assert_eq!(
        refusal(&[
            record("m", std::slice::from_ref(&a)),
            record("m", std::slice::from_ref(&a))
        ]),
        repeated
    );
    assert_eq!(
        refusal(&[
            record("m", &[a.clone(), b.clone()]),
            record("m", &[b, a.clone()])
        ]),
        None
    );
    assert_eq!(
        refusal(&[record("m", std::slice::from_ref(&a)), record("", &[a])]),
        None
    );
    let same_fields = [
        quad("a", "<p>", "c"),
        Line::Annotation {
            count: 1,
            reifier: term("a"),
            predicate: term("<p>"),
            object: term("c"),
            graph: None,
        },
    ];
    assert_eq!(refusal(&same_fields), None);
    // The smallest code wins: slot (0) before repeated (2).
    assert_eq!(
        refusal(&[annotation(1), annotation(2), quad("a", "b", "c")]),
        slot
    );
    // An empty graph that a statement carries; the same blank name only as a
    // subject is well formed (example 12.7 holds that case).
    let carried = vec![
        Line::EmptyGraph(term("g")),
        Line::Quad {
            subject: term("a"),
            predicate: term("<p>"),
            object: term("b"),
            graph: Some(term("g")),
        },
    ];
    assert_eq!(
        refusal(&carried),
        Some(Refusal::Graph {
            term: GraphTerm::Blank,
            source: Some(b"g"[..].into())
        })
    );
    assert_eq!(refusal(&every_kind()), None);
    // W5 compares typed terms: a triple term holding blank nodes is equal to
    // another only with the same nodes, so a graph name `<<(b <p> a)>>` does
    // not carry the empty graph `<<(a <p> b)>>` and the input is labelled.
    let swapped = [
        Line::EmptyGraph(Term::triple(term("a"), term("<p>"), term("b"))),
        Line::Quad {
            subject: term("a"),
            predicate: term("<p>"),
            object: term("b"),
            graph: Some(Term::triple(term("b"), term("<p>"), term("a"))),
        },
    ];
    let set = admit(&swapped).expect("well formed");
    assert!(reference::label_set(&set, reference::ENGINE_LIMIT, false).is_ok());
    // The second example of W5: equal when every blank leaf reads `free`,
    // different typed values.
    let other_nodes = [
        Line::EmptyGraph(Term::triple(term("a"), term("<p>"), term("<o>"))),
        Line::Quad {
            subject: term("x"),
            predicate: term("<p>"),
            object: term("y"),
            graph: Some(Term::triple(term("b"), term("<p>"), term("<o>"))),
        },
    ];
    let set = admit(&other_nodes).expect("well formed");
    assert!(reference::label_set(&set, reference::ENGINE_LIMIT, false).is_ok());
    let same = [
        Line::EmptyGraph(Term::triple(term("a"), term("<p>"), term("b"))),
        Line::Quad {
            subject: term("a"),
            predicate: term("<p>"),
            object: term("b"),
            graph: Some(Term::triple(term("a"), term("<p>"), term("b"))),
        },
    ];
    assert_eq!(
        refusal(&same),
        Some(Refusal::Graph {
            term: GraphTerm::Triple,
            source: None
        })
    );
    // An IRI graph name carried only by a reifier binding names a non-empty
    // graph, so an empty-graph line for it is refused; the IRI code (0)
    // precedes the blank code (2).
    let mut both = carried;
    both.push(Line::EmptyGraph(term("<g>")));
    both.push(Line::ReifierBinding {
        reifier: term("r"),
        triple: Term::triple(term("s"), term("<p>"), term("o")),
        graph: Some(term("<g>")),
    });
    assert_eq!(
        refusal(&both),
        Some(Refusal::Graph {
            term: GraphTerm::Iri,
            source: None
        })
    );
    // Refusal precedes labelling: an input refusal wins over a graph one.
    both.push(quad("a", "b", "c"));
    assert_eq!(refusal(&both), slot);
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Correctness checks of the colour labelling: isomorphism invariance, the two
//! modes against each other and the forest claim against the full search.

use crate::colour::{MARK, Options, RawLine, label};

/// A deterministic shift-and-multiply generator.
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub(crate) fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The same lines with every blank node renamed by a seeded bijection and the
/// line order shuffled.
pub(crate) fn relabelled(lines: &[RawLine], seed: u64) -> Vec<RawLine> {
    let mut rng = Rng(seed | 1);
    let mut ids: Vec<u32> = lines.iter().flat_map(|l| l.tuple.iter().copied()).collect();
    ids.sort_unstable();
    ids.dedup();
    let mut images: Vec<u32> = (0..ids.len() as u32).map(|i| 5000 + i).collect();
    for i in (1..images.len()).rev() {
        images.swap(i, rng.below(i as u64 + 1) as usize);
    }
    let map: std::collections::BTreeMap<u32, u32> = ids.into_iter().zip(images).collect();
    let mut out: Vec<RawLine> = lines
        .iter()
        .map(|l| RawLine {
            text: l.text.clone(),
            tuple: l.tuple.iter().map(|t| map[t]).collect(),
            count: l.count,
        })
        .collect();
    for i in (1..out.len()).rev() {
        out.swap(i, rng.below(i as u64 + 1) as usize);
    }
    out
}

fn doc(lines: &[RawLine], options: Options) -> Result<String, u64> {
    label(lines, options)
        .map(|l| l.document)
        .map_err(|r| r.0.units())
}

/// Relabel-and-shuffle invariance of the optimised mode, and equality with the
/// literal mode when the literal run finishes inside `literal_limit` units.
/// Returns a one-line report.
pub(crate) fn invariance(name: &str, lines: &[RawLine], literal_limit: u64) -> String {
    let opt = doc(lines, Options::OPTIMISED);
    let Ok(base) = opt else {
        return format!("{name}: optimised refused");
    };
    let mut same = true;
    for seed in 1..=3 {
        same &= doc(&relabelled(lines, seed), Options::OPTIMISED).as_ref() == Ok(&base);
    }
    let stats = label(lines, Options::OPTIMISED)
        .expect("finished above")
        .stats;
    let literal = doc(
        lines,
        Options {
            limit: literal_limit,
            ..Options::LITERAL
        },
    );
    let lit = match literal {
        Ok(d) if d == base => "literal-equal".to_owned(),
        Ok(_) => "LITERAL-DIFFERS".to_owned(),
        Err(_) => "literal-skipped".to_owned(),
    };
    format!(
        "{name}: nodes={} lines={} comps={} residual_comps={} invariant={} {lit}",
        stats.nodes,
        stats.lines,
        stats.components,
        stats.residual_components,
        if same { "yes" } else { "NO" },
    )
}

/// A random forest in the incidence-graph sense: every line joins one existing
/// node to one or two new nodes, in a random order of positions, over a few
/// predicates, so most forests carry automorphisms. Some lines repeat a node or
/// share a tuple with a line of another text.
pub(crate) fn random_forest(rng: &mut Rng, nodes: usize) -> Vec<RawLine> {
    let mut lines = Vec::new();
    let mut count = 1u32;
    let line = |text: String, tuple: Vec<u32>| RawLine {
        text,
        tuple,
        count: 1,
    };
    let make = |tuple: &[u32], rng: &mut Rng| {
        let p = ["p", "q"][rng.below(2) as usize];
        let mut t = String::new();
        for (i, _) in tuple.iter().enumerate() {
            if i > 0 {
                t.push_str(" <http://example.org/");
                t.push_str(p);
                t.push_str("> ");
            }
            t.push(MARK);
        }
        t.push_str(" .");
        line(t, tuple.to_vec())
    };
    // The root and its typing line.
    lines.push(line(
        format!(
            "{MARK} <http://example.org/type> <http://example.org/T{}> .",
            rng.below(2)
        ),
        vec![0],
    ));
    while (count as usize) < nodes {
        let at = rng.below(u64::from(count)) as u32;
        let new = (nodes as u32 - count).min(1 + rng.below(2) as u32);
        let mut tuple = vec![at];
        tuple.extend(count..count + new);
        count += new;
        for i in (1..tuple.len()).rev() {
            tuple.swap(i, rng.below(i as u64 + 1) as usize);
        }
        let l = make(&tuple, rng);
        if rng.below(4) == 0 {
            // A second text over the same ordered tuple keeps the incidence graph a forest.
            let mut twin = make(&tuple, rng);
            twin.text = twin.text.replace(" .", " <http://example.org/extra> .");
            lines.push(twin);
        }
        lines.push(l);
        if rng.below(5) == 0 {
            // Typing lines on some nodes break symmetry only partly.
            let v = tuple[rng.below(tuple.len() as u64) as usize];
            lines.push(line(
                format!(
                    "{MARK} <http://example.org/type> <http://example.org/T{}> .",
                    rng.below(2)
                ),
                vec![v],
            ));
        }
    }
    lines
}

/// The forest claim: the single path of the optimised mode equals the full leaf
/// minimum (optimised without the single path) and the literal mode.
pub(crate) fn forest_claim(
    seeds: std::ops::Range<u64>,
    max_nodes: usize,
    literal_limit: u64,
) -> String {
    let (mut checked, mut literal_checked) = (0u64, 0u64);
    for seed in seeds {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let nodes = 2 + rng.below(max_nodes as u64 - 1) as usize;
        let lines = random_forest(&mut rng, nodes);
        let path = doc(&lines, Options::OPTIMISED);
        let full = doc(
            &lines,
            Options {
                no_forest_path: true,
                ..Options::OPTIMISED
            },
        );
        let literal = doc(
            &lines,
            Options {
                limit: literal_limit,
                ..Options::LITERAL
            },
        );
        checked += 1;
        let bad = path != full || literal.as_ref().is_ok_and(|d| Ok(d) != path.as_ref());
        literal_checked += u64::from(literal.is_ok());
        if bad {
            let mut text = format!("COUNTEREXAMPLE seed={seed} nodes={nodes}\n");
            for l in &lines {
                put(&mut text, format!("  {:?} {:?}\n", l.text, l.tuple));
            }
            put(
                &mut text,
                format!("path:\n{path:?}\nfull:\n{full:?}\nliteral:\n{literal:?}\n"),
            );
            return text;
        }
    }
    format!(
        "forest claim holds on {checked} random forests ({literal_checked} also against the literal mode)"
    )
}

fn put(out: &mut String, s: impl AsRef<str>) {
    out.push_str(s.as_ref());
}

/// A masked line over the given tuple with one predicate.
fn line(p: &str, tuple: &[u32]) -> RawLine {
    let mut t = String::new();
    for (i, _) in tuple.iter().enumerate() {
        if i > 0 {
            t.push_str(" <http://example.org/");
            t.push_str(p);
            t.push_str("> ");
        }
        t.push(MARK);
    }
    t.push_str(" .");
    RawLine {
        text: t,
        tuple: tuple.to_vec(),
        count: 1,
    }
}

fn run_units(lines: &[RawLine], options: Options) -> String {
    match label(lines, options) {
        Ok(done) => format!(
            "units={} rounds={} canon_calls={} children={} depth={}",
            done.stats.units(),
            done.stats.rounds,
            done.stats.canon_calls,
            done.stats.children,
            done.stats.max_depth
        ),
        Err(r) => format!("refused at units={}", r.0.units()),
    }
}

/// The counterexamples of the research report, confirmed or refuted by running them.
pub(crate) fn counterexamples() -> String {
    let mut out = String::new();
    // CE9: two disjoint directed 3-cycles under ONE global partition.
    let tri = |base: u32| -> Vec<RawLine> {
        (0..3)
            .map(|i| line("p", &[base + i, base + (i + 1) % 3]))
            .collect()
    };
    let mut two = tri(0);
    two.extend(tri(3));
    let root = crate::colour::root_colours(&two, false);
    let after = crate::colour::global_individualised(&two, 0);
    put(
        &mut out,
        format!(
            "CE9 root colours of a,b,c,x,y,z = {root:?}\nCE9 colours after individualising a (one global partition) = {after:?}\n"
        ),
    );
    let moved = after[3..].iter().all(|c| c.1 == after[3].1) && after[3].1 != root[3].1;
    put(
        &mut out,
        format!(
            "CE9 second cycle's colour moved by a choice in the first cycle: {}\n",
            if moved { "CONFIRMED" } else { "REFUTED" }
        ),
    );
    for k in [10u32, 100, 1000] {
        let lines: Vec<RawLine> = (0..k).flat_map(|c| tri(3 * c)).collect();
        put(
            &mut out,
            format!(
                "CE9 {k} identical triangles, nested components, optimised: {}\n",
                run_units(&lines, Options::OPTIMISED)
            ),
        );
        if k <= 100 {
            put(
                &mut out,
                format!(
                    "CE9 {k} identical triangles, nested components, literal:   {}\n",
                    run_units(&lines, Options::LITERAL)
                ),
            );
        }
    }
    // CE10: a directed path of alike nodes.
    for n in [250u32, 500, 1000, 2000, 4000, 8000] {
        let lines: Vec<RawLine> = (0..n - 1).map(|i| line("p", &[i, i + 1])).collect();
        let lit = if n <= 1000 {
            format!(" | literal {}", run_units(&lines, Options::LITERAL))
        } else {
            String::new()
        };
        put(
            &mut out,
            format!(
                "CE10 path n={n}: optimised {}{lit}\n",
                run_units(&lines, Options::OPTIMISED)
            ),
        );
    }
    // CE4: a bundle of parallel lines against its double cover.
    let ce4 = vec![
        RawLine {
            text: format!("{MARK} <http://example.org/type> <http://example.org/Root> ."),
            tuple: vec![0],
            count: 1,
        },
        line("h", &[0, 1]),
        line("s", &[1, 2]),
        line("t", &[1, 2]),
        line("h", &[0, 3]),
        line("s", &[3, 4]),
        line("t", &[3, 5]),
        line("h", &[0, 6]),
        line("s", &[6, 5]),
        line("t", &[6, 4]),
    ];
    let root = crate::colour::root_colours(&ce4, false);
    let doc = |o: Options| {
        label(&ce4, o)
            .map(|l| (l.document, l.stats.forest_paths))
            .map_err(|r| r.0.units())
    };
    let (lit, opt, full) = (
        doc(Options::LITERAL),
        doc(Options::OPTIMISED),
        doc(Options {
            no_forest_path: true,
            ..Options::OPTIMISED
        }),
    );
    put(
        &mut out,
        format!(
            "CE4 root colours of r,x,y,x1,y1,y2,x2 = {:?}\nCE4 literal == optimised == optimised-without-forest-path: {}; forest path taken: {}\n",
            root,
            lit.as_ref().ok().map(|l| &l.0) == opt.as_ref().ok().map(|l| &l.0)
                && opt.as_ref().ok().map(|l| &l.0) == full.as_ref().ok().map(|l| &l.0),
            opt.as_ref().map_or(0, |l| l.1),
        ),
    );
    // R1: an entry never compares an integer with a byte string in this definition.
    let r1 = vec![
        line("p", &[0, 1]),
        RawLine {
            text: format!(
                "{MARK} <http://example.org/q> <<( {MARK} <http://example.org/r> {MARK} )>> ."
            ),
            tuple: vec![0, 1, 2],
            count: 1,
        },
    ];
    put(
        &mut out,
        format!(
            "R1 mixed-arity certificate (text before positions, so integer and bytes never meet): {}\n",
            run_units(&r1, Options::LITERAL)
        ),
    );
    out
}

/// A random graph of binary lines over few predicates.
fn random_graph(rng: &mut Rng, nodes: u32) -> Vec<RawLine> {
    let mut lines = Vec::new();
    for _ in 0..nodes + rng.below(u64::from(nodes) + 1) as u32 {
        let a = rng.below(u64::from(nodes)) as u32;
        let b = rng.below(u64::from(nodes)) as u32;
        let p = if rng.below(2) == 0 { "p" } else { "q" };
        lines.push(line(p, &[a, b]));
    }
    lines
}

/// Synchronous literal rounds against worklist rounds: equal root colours (the
/// cell order, not only the partition) and equal documents on random inputs.
pub(crate) fn refine_order_check(seeds: std::ops::Range<u64>) -> String {
    let mut checked = 0;
    for seed in seeds {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let nodes = 3 + rng.below(8) as u32;
        let lines = if seed % 2 == 0 {
            random_graph(&mut rng, nodes)
        } else {
            random_forest(&mut rng, nodes as usize)
        };
        checked += 1;
        let (a, b) = (
            crate::colour::root_colours(&lines, false),
            crate::colour::root_colours(&lines, true),
        );
        let docs = (
            doc(
                &lines,
                Options {
                    limit: 3_000_000,
                    ..Options::LITERAL
                },
            ),
            doc(&lines, Options::OPTIMISED),
        );
        let doc_bad = matches!(&docs, (Ok(x), Ok(y)) if x != y);
        if a != b || doc_bad {
            let mut text = format!(
                "DIFFERENCE seed={seed} nodes={nodes} literal-root={a:?} worklist-root={b:?} documents-differ={doc_bad}\n"
            );
            for l in &lines {
                put(&mut text, format!("  {:?} {:?}\n", l.text, l.tuple));
            }
            return text;
        }
    }
    format!(
        "synchronous and worklist refinement agree on the root colour of every node, and the two modes on every document, over {checked} random inputs"
    )
}

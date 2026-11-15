// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Measure blank-node labelling on deterministic shapes and shared inputs.
//!
//! ```text
//! canon_converge fam <mode> <family> <a> <b> <c> [limit]
//! canon_converge label <mode> <file> [limit]
//! canon_converge current <family> <a> <b> <c> [budget]
//! canon_converge build <family> <a> <b> <c>
//! canon_converge docs <vectors-dir> <out-dir> <literal-limit> <revision> [extra files...]
//! canon_converge iso <directory-or-file> <literal-limit>
//! canon_converge random-check <first-seed> <count> <nodes> <lines>
//! canon_converge write <family> <a> <b> <c> <out-file>
//! ```
//!
//! `mode` is `lit` (the definition, no shortcut) or `opt` (every permitted
//! shortcut), followed by comma-separated switches: `-orbits`, `-twins`,
//! `-prune-e`, `-prune-nested`, `eager`, `full-search`, `inv` (the node
//! invariant experiment), `small`/`large` (target-cell experiments), `wl2`
//! (two-dimensional refinement on branching calls), `whole` (revision 3's one
//! refinement over the whole fold, `lit` only). One RESULT line is printed; run
//! each cell in its own process under `perf stat -e instructions:u`.

use std::fmt::Write as _;

mod checks;
mod families;
mod r31;
#[cfg(test)]
mod tests;
mod view_lines;

use purrdf_core::{
    CanonHash, CanonicalizationControl, SearchPolicy, SearchWork, try_canonical_sha256_view,
};

use families::Fold;
use r31::{Flag, Input, Labelled, Options, Refusal, Selector, literal, optimised};

fn fold_of(family: &str, a: u32, b: u32, c: u32) -> Fold {
    match family {
        // fan, spine depth, hubs
        "hubs" => families::hubs(a, b, c, true),
        "hubs-id" => families::hubs(a, b, c, false),
        "tree" => families::tree(a, b),
        "path" => families::path(a),
        "list" => families::list(a, true),
        "list-same" => families::list(a, false),
        "kmn" => families::bipartite(a, true),
        "kmn-dir" => families::bipartite(a, false),
        "rook" => families::rook(a),
        "cube" => families::cube(a),
        "triangles" => families::triangles(a),
        "paley" => families::paley(a),
        "latin" => families::latin(a, u64::from(b)),
        "sts" => families::sts(a, u64::from(b)),
        "cubic" => families::cubic(a, u64::from(b)),
        "cfi" => families::cfi(a, u64::from(b), c != 0),
        "cycle" => families::cycle(a),
        other => panic!("unknown family {other}"),
    }
}

/// The counted line text of a fold, blank nodes `_:b<number>`.
pub(crate) fn fold_text(fold: &Fold) -> String {
    let term = |n: &families::Node| match n {
        families::Node::Blank(i) => format!("_:b{i}"),
        families::Node::Iri(name) => format!("<http://example.org/{name}>"),
        families::Node::Lit(text) => format!("\"{text}\""),
    };
    let mut out = String::new();
    for (s, p, o) in &fold.quads {
        let _ = writeln!(out, "{} <http://example.org/{p}> {} .", term(s), term(o));
    }
    out
}

/// Parse a mode specification.
pub(crate) fn parse_mode(spec: &str, limit: u64) -> (bool, Options) {
    let mut parts = spec.split(',');
    let literal = match parts.next() {
        Some("lit") => true,
        Some("opt") => false,
        other => panic!("unknown mode {other:?}"),
    };
    let mut o = Options {
        limit,
        ..Options::DEFAULT
    };
    for switch in parts {
        match switch {
            "-orbits" => o = o.with(Flag::Orbits, false),
            "-twins" => o = o.with(Flag::Twins, false),
            "-prune-e" => o = o.with(Flag::PruneE, false),
            "-prune-nested" => o = o.with(Flag::PruneNestedE, false),
            "eager" => o = o.with(Flag::Lazy, false),
            "full-search" => o = o.with(Flag::Incremental, false),
            "inv" => o = o.with(Flag::Invariant, true),
            "small" => o.selector = Selector::Smallest,
            "large" => o.selector = Selector::Largest,
            "traces" => o.selector = Selector::Traces,
            "wl2" => o = o.with(Flag::Wl2, true),
            "whole" => o = o.with(Flag::WholeFold, true),
            other => panic!("unknown switch {other}"),
        }
    }
    (literal, o)
}

pub(crate) fn run(
    literal_mode: bool,
    input: &Input,
    options: Options,
) -> Result<Labelled, Refusal> {
    if literal_mode {
        literal::label_input(input, options)
    } else {
        optimised::label_input(input, options)
    }
}

fn result_line(cell: &str, input: &Input, outcome: &Result<Labelled, Refusal>) -> String {
    use sha2::{Digest, Sha256};
    let (verdict, s) = match outcome {
        Ok(done) => (
            format!(
                "ok doc_sha256={} automorphisms={}",
                purrdf_hash::hex::encode(&Sha256::digest(done.document.as_bytes())),
                done.has_automorphisms()
            ),
            done.stats.clone(),
        ),
        Err(Refusal::Limit(s)) => ("refused limit".to_owned(), (**s).clone()),
        Err(Refusal::Width) => ("refused width".to_owned(), r31::Stats::default()),
    };
    format!(
        "RESULT {cell} nodes={} lines={} stats_nodes={} stats_lines={} units={} visits={} visit_elements={} entries_formed={} entries_compared={} restriction={} key_compares={} pops={} calls={} branch_calls={} children={} leaves={} pruned_orbit={} pruned_twin={} pruned_e={} pruned_nested_e={} pruned_invariant={} generators={} discovery_steps={} copied={} trail={} max_depth={} branch_nodes={} largest_target={} wl2_steps={} {verdict}",
        input.nodes(),
        input.lines.len(),
        s.nodes,
        s.lines,
        s.units(),
        s.visits,
        s.visit_elements,
        s.entries_formed,
        s.entries_compared,
        s.restriction,
        s.key_compares,
        s.pops,
        s.calls,
        s.branch_calls,
        s.children,
        s.leaves,
        s.pruned_orbit,
        s.pruned_twin,
        s.pruned_e,
        s.pruned_nested_e,
        s.pruned_invariant,
        s.generators,
        s.discovery_steps,
        s.copied,
        s.trail,
        s.max_depth,
        s.branch_nodes,
        s.largest_target,
        s.wl2_steps,
    )
}

fn number(rest: &[String], i: usize) -> u64 {
    rest.get(i)
        .map_or(0, |s| s.parse::<u64>().expect("a number"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [command, rest @ ..] = args.as_slice() else {
        panic!("usage: canon_converge <command> ...");
    };
    match command.as_str() {
        "fam" => {
            let fold = fold_of(
                &rest[1],
                number(rest, 2) as u32,
                number(rest, 3) as u32,
                number(rest, 4) as u32,
            );
            let limit = rest
                .get(5)
                .map_or(u64::MAX, |s| s.parse().expect("a limit"));
            let input = Input::read(&fold_text(&fold)).expect("family text reads");
            let (lit, options) = parse_mode(&rest[0], limit);
            let outcome = run(lit, &input, options);
            let cell = format!(
                "{} {} {} {} {}",
                rest[0], rest[1], rest[2], rest[3], rest[4]
            );
            println!("{}", result_line(&cell, &input, &outcome));
        }
        "label" => {
            let text = std::fs::read_to_string(&rest[1]).expect("a readable file");
            let input = Input::read(&text).unwrap_or_else(|e| panic!("{}", e.0));
            let limit = rest
                .get(2)
                .map_or(u64::MAX, |s| s.parse().expect("a limit"));
            let (lit, options) = parse_mode(&rest[0], limit);
            let outcome = run(lit, &input, options);
            println!(
                "{}",
                result_line(&format!("{} {}", rest[0], rest[1]), &input, &outcome)
            );
        }
        "write" => {
            let fold = fold_of(
                &rest[0],
                number(rest, 1) as u32,
                number(rest, 2) as u32,
                number(rest, 3) as u32,
            );
            std::fs::write(&rest[4], fold_text(&fold)).expect("write the input");
        }
        "docs" => docs(&rest[0], &rest[1], number(rest, 2), &rest[3], &rest[4..]),
        "iso" => iso(&rest[0], number(rest, 1)),
        "random-check" => {
            let (first, count) = (number(rest, 0), number(rest, 1));
            let (nodes, lines) = (number(rest, 2), number(rest, 3));
            let mut totals = [0u64; 3];
            for seed in first..first + count {
                let input = Input::read(&checks::random_input(seed, nodes, lines))
                    .expect("random text reads");
                let (report, counts) = checks::invariance(&format!("seed{seed}"), &input, u64::MAX);
                for (t, c) in totals.iter_mut().zip(counts) {
                    *t += c;
                }
                if report.contains("NO") || report.contains("DIFFERS") {
                    println!("{report}");
                }
            }
            println!(
                "random-check inputs={count} renamings={} literal_equal={} literal_differs={}",
                totals[0], totals[1], totals[2]
            );
        }
        "current" | "build" => baseline(command, rest),
        "exp" => experiment(rest),
        "big-vector" => std::fs::write(&rest[1], big_vector(&rest[0])).expect("write the input"),
        "dependence" => println!(
            "{}",
            checks::dependence_search(number(rest, 0) as u32, number(rest, 1) as u32)
        ),
        "trace" => {
            // trace <file> [whole]: the literal REFINE steps on standard error.
            let text = std::fs::read_to_string(&rest[0]).expect("a readable file");
            let input = Input::read(&text).unwrap_or_else(|e| panic!("{}", e.0));
            let options = Options::DEFAULT
                .with(Flag::Trace, true)
                .with(Flag::WholeFold, rest.get(1).is_some_and(|w| w == "whole"));
            let done = literal::label_input(&input, options).expect("labels");
            print!("{}", done.document);
        }
        "view" => {
            // The same labelling over the lines a dataset view writes.
            let presentation = match rest[0].as_str() {
                "overlay" => view_lines::Presentation::Overlay,
                "flat" => view_lines::Presentation::Flat,
                other => panic!("unknown presentation {other}"),
            };
            let fold = fold_of(
                &rest[1],
                number(rest, 2) as u32,
                number(rest, 3) as u32,
                number(rest, 4) as u32,
            );
            let text = view_lines::lines_from_view(&*fold.dataset(), presentation);
            let input = Input::read(&text).unwrap_or_else(|e| panic!("{}", e.0));
            let outcome = optimised::label_input(&input, Options::DEFAULT);
            println!("{}", result_line(&rest.join(" "), &input, &outcome));
        }
        other => panic!("unknown command {other}"),
    }
}

/// `exp <mode> <relabellings> <family> <a> <b> <c> [limit]` or
/// `exp <mode> <relabellings> file <path> [limit]`: run every relabelling
/// (seeds 1 to R), check that the documents are identical, print medians.
fn experiment(rest: &[String]) {
    let relabellings = number(rest, 1);
    let (text, limit_at) = if rest[2] == "file" {
        (
            std::fs::read_to_string(&rest[3]).expect("a readable file"),
            4,
        )
    } else {
        let fold = fold_of(
            &rest[2],
            number(rest, 3) as u32,
            number(rest, 4) as u32,
            number(rest, 5) as u32,
        );
        (fold_text(&fold), 6)
    };
    let limit = rest
        .get(limit_at)
        .map_or(u64::MAX, |s| s.parse().expect("a limit"));
    let base = Input::read(&text).unwrap_or_else(|e| panic!("{}", e.0));
    let (lit, options) = parse_mode(&rest[0], limit);
    let mut units = Vec::new();
    let mut leaves = Vec::new();
    let mut children = Vec::new();
    let mut docs: Vec<String> = Vec::new();
    let mut refused = 0u64;
    for seed in 1..=relabellings {
        let input = Input::read(&checks::relabelled(&base, seed)).expect("relabelled text reads");
        match run(lit, &input, options) {
            Ok(done) => {
                units.push(done.stats.units());
                leaves.push(done.stats.leaves);
                children.push(done.stats.children);
                docs.push(done.document);
            }
            Err(_) => refused += 1,
        }
    }
    let median = |v: &mut Vec<u64>| -> String {
        if v.is_empty() {
            return "-".to_owned();
        }
        v.sort_unstable();
        let n = v.len();
        let twice = if n % 2 == 1 {
            2 * v[n / 2]
        } else {
            v[n / 2 - 1] + v[n / 2]
        };
        if twice % 2 == 0 {
            (twice / 2).to_string()
        } else {
            format!("{}.5", twice / 2)
        }
    };
    docs.dedup();
    let (min, max) = (
        units.iter().min().copied().unwrap_or(0),
        units.iter().max().copied().unwrap_or(0),
    );
    println!(
        "EXP {} nodes={} lines={} runs={} refused={refused} identical_documents={} median_units={} min_units={min} max_units={max} median_leaves={} median_children={}",
        rest[..limit_at].join(" "),
        base.nodes(),
        base.lines.len(),
        relabellings,
        docs.len() <= 1,
        median(&mut units),
        median(&mut leaves),
        median(&mut children),
    );
}

/// The two shared inputs too large to ship, regenerated in the shared
/// generator's spelling (lines sorted bytewise).
fn big_vector(id: &str) -> String {
    const FIRST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>";
    const REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";
    const NIL: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";
    let mut rows: Vec<String> = Vec::new();
    match id {
        "list-identical-10000" => {
            let n = 10_000;
            rows.push("<http://e/a> <http://e/p> _:b0 .".to_owned());
            for i in 0..n {
                rows.push(format!("_:b{i} {FIRST} \"x\" ."));
                if i + 1 < n {
                    rows.push(format!("_:b{i} {REST} _:b{} .", i + 1));
                } else {
                    rows.push(format!("_:b{i} {REST} {NIL} ."));
                }
            }
        }
        "triangles-10000" => {
            for k in 0..10_000u32 {
                let (a, b, c) = (3 * k, 3 * k + 1, 3 * k + 2);
                for (x, y) in [(a, b), (b, c), (c, a)] {
                    rows.push(format!("_:b{x} <http://e/p> _:b{y} ."));
                }
            }
        }
        other => panic!("unknown oversized input {other}"),
    }
    rows.sort_unstable();
    let mut text = rows.join("\n");
    text.push('\n');
    text
}

fn baseline(engine: &str, rest: &[String]) {
    let fold = if rest[0] == "repro" {
        families::from_nquads(&std::fs::read_to_string(&rest[1]).expect("a readable file"))
    } else {
        fold_of(
            &rest[0],
            number(rest, 1) as u32,
            number(rest, 2) as u32,
            number(rest, 3) as u32,
        )
    };
    let budget = rest
        .get(4)
        .map_or(1 << 28, |s| s.parse().expect("a budget"));
    let cell = rest[..4].join(" ");
    let ds = fold.dataset();
    if engine == "build" {
        println!(
            "RESULT {cell} blanks={} quads={} built",
            fold.blanks,
            ds.quad_count()
        );
        return;
    }
    let mut control = CanonicalizationControl::new(SearchPolicy::Finite(budget), None);
    let outcome = try_canonical_sha256_view(&*ds, CanonHash::Sha256, &mut control);
    let receipt = control.receipts().last().expect("one receipt");
    let work: SearchWork = receipt.work;
    let verdict = match &outcome {
        Ok(digest) => format!("ok digest={}", digest.to_hex()),
        Err(error) => format!("refused {error:?}").chars().take(60).collect(),
    };
    println!(
        "RESULT {cell} blanks={} budget={budget} work={} completed={} {verdict}",
        fold.blanks,
        work.total(),
        receipt.completed,
    );
}

fn iso(path: &str, literal_limit: u64) {
    let p = std::path::Path::new(path);
    let mut files: Vec<std::path::PathBuf> = if p.is_dir() {
        std::fs::read_dir(p)
            .expect("readable directory")
            .map(|e| e.expect("entry").path())
            .filter(|f| {
                let n = f.to_string_lossy();
                n.ends_with(".nq") || n.ends_with(".input.txt")
            })
            .collect()
    } else {
        vec![p.to_path_buf()]
    };
    files.sort();
    let mut totals = [0u64; 3];
    let mut inputs = 0u64;
    for f in files {
        let Ok(input) = Input::read(&std::fs::read_to_string(&f).expect("readable file")) else {
            println!("{} unreadable", f.display());
            continue;
        };
        inputs += 1;
        let name = f
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let (report, counts) = checks::invariance(&name, &input, literal_limit);
        for (t, c) in totals.iter_mut().zip(counts) {
            *t += c;
        }
        println!("{report}");
    }
    println!(
        "iso inputs={inputs} renamings={} literal_equal={} literal_differs={}",
        totals[0], totals[1], totals[2]
    );
}

/// Documents of every `*.input.txt` of a vector directory (and any extra
/// files), with a manifest.
fn docs(vectors: &str, out: &str, literal_limit: u64, revision: &str, extra: &[String]) {
    use sha2::{Digest, Sha256};
    let sha = |bytes: &[u8]| purrdf_hash::hex::encode(&Sha256::digest(bytes));
    std::fs::create_dir_all(out).expect("output directory");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(vectors)
        .expect("readable directory")
        .map(|e| e.expect("entry").path())
        .filter(|f| f.to_string_lossy().ends_with(".input.txt"))
        .chain(extra.iter().map(std::path::PathBuf::from))
        .collect();
    files.sort();
    let mut entries = Vec::new();
    let (mut literal_count, mut agree) = (0u64, 0u64);
    for f in files {
        let name = f
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let id = name
            .trim_end_matches(".input.txt")
            .trim_end_matches(".nq")
            .to_owned();
        let bytes = std::fs::read(&f).expect("readable file");
        let input = Input::read(&String::from_utf8(bytes.clone()).expect("UTF-8"))
            .expect("the input reads as lines");
        let done = optimised::label_input(&input, Options::DEFAULT).expect("labelled");
        let literal = literal::label_input(
            &input,
            Options {
                limit: literal_limit,
                ..Options::DEFAULT
            },
        );
        let checked = match literal {
            Ok(l) => {
                literal_count += 1;
                let same = l.document == done.document;
                agree += u64::from(same);
                assert!(same, "{id}: literal and optimised documents differ");
                "literal-equal"
            }
            Err(_) => "optimised-only",
        };
        let labels: Vec<String> = input
            .labels
            .iter()
            .zip(&done.positions)
            .map(|(l, q)| format!("\"{l}\": \"c14n{q}\""))
            .collect();
        std::fs::write(
            std::path::Path::new(out).join(format!("{id}.labels.json")),
            format!("{{{}}}\n", labels.join(", ")),
        )
        .expect("write labels");
        let doc_name = format!("{id}.document");
        std::fs::write(
            std::path::Path::new(out).join(&doc_name),
            done.document.as_bytes(),
        )
        .expect("write document");
        entries.push(format!(
            "    {{\"id\": \"{id}\", \"input\": \"{name}\", \"input_sha256\": \"{}\", \"document\": \"{doc_name}\", \"document_sha256\": \"{}\", \"has_automorphisms\": {}, \"work_units\": {}, \"checked\": \"{checked}\", \"revision\": \"{revision}\"}}",
            sha(&bytes),
            sha(done.document.as_bytes()),
            done.has_automorphisms(),
            done.stats.units(),
        ));
    }
    let manifest = format!(
        "{{\n  \"revision\": \"{revision}\",\n  \"cases\": [\n{}\n  ]\n}}\n",
        entries.join(",\n")
    );
    std::fs::write(std::path::Path::new(out).join("manifest.json"), manifest)
        .expect("write manifest");
    println!(
        "wrote {} cases to {out}; literal ran on {literal_count}, equal on {agree}",
        entries.len()
    );
}

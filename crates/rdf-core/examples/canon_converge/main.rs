// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Measure blank-node labelling on a family of deterministic shapes.
//!
//! ```text
//! canon_converge <engine> <family> <a> <b> <c> [budget]
//! canon_converge iso <directory-or-file> [literal-limit]
//! canon_converge forest <first-seed> <count> <max-nodes>
//! ```
//!
//! `engine` is `build` (construct the fold only, the zero point for an
//! instruction count), `current` (the shipped search, budget in charged
//! events), `colour` or `colour-literal` (the colour labelling, budget in work
//! units), or `colour-overlay` / `colour-flat` (the same through a dataset view
//! in either presentation). One result line is printed; run each cell in its
//! own process under `perf stat -e instructions:u`.

mod checks;
mod colour;
mod families;
mod nquads;
#[cfg(test)]
mod tests;
mod view_lines;

use purrdf_core::{
    CanonHash, CanonicalizationControl, SearchPolicy, SearchWork, try_canonical_sha256_view,
};

use colour::{MARK, Mode, Options, RawLine};
use families::Fold;

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
        other => panic!("unknown family {other}"),
    }
}

/// The masked lines of a fold.
pub(crate) fn fold_lines(fold: &Fold) -> Vec<RawLine> {
    let tuple_text = |tuple: &mut Vec<u32>, n: &families::Node| match n {
        families::Node::Blank(i) => {
            tuple.push(*i);
            MARK.to_string()
        }
        families::Node::Iri(name) => format!("<http://example.org/{name}>"),
        families::Node::Lit(text) => format!("\"{text}\""),
    };
    fold.quads
        .iter()
        .map(|(s, p, o)| {
            let mut tuple = Vec::new();
            let s = tuple_text(&mut tuple, s);
            let o = tuple_text(&mut tuple, o);
            RawLine {
                text: format!("{s} <http://example.org/{p}> {o} ."),
                tuple,
                count: 1,
            }
        })
        .filter(|l| !l.tuple.is_empty())
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [engine, rest @ ..] = args.as_slice() else {
        panic!("usage: canon_converge <engine> ...");
    };
    match engine.as_str() {
        "ce" => return println!("{}", checks::counterexamples()),
        "refine-check" => {
            let n = |i: usize| rest[i].parse::<u64>().expect("a number");
            return println!("{}", checks::refine_order_check(n(0)..n(0) + n(1)));
        }
        "docs" => {
            return docs(
                &rest[0],
                &rest[1],
                rest.get(2)
                    .map_or(2_000_000, |s| s.parse().expect("a limit")),
            );
        }
        "iso" => {
            return iso(
                &rest[0],
                rest.get(1)
                    .map_or(2_000_000, |s| s.parse().expect("a limit")),
            );
        }
        "forest" => {
            let n = |i: usize| rest[i].parse::<u64>().expect("a number");
            println!(
                "{}",
                checks::forest_claim(n(0)..n(0) + n(1), n(2) as usize, 5_000_000)
            );
            return;
        }
        _ => {}
    }
    let [family, rest @ ..] = rest else {
        panic!("usage: canon_converge <engine> <family> <a> <b> <c> [budget]");
    };
    let number = |i: usize| {
        rest.get(i)
            .map_or(0, |s| s.parse::<u64>().expect("a number"))
    };
    let fold = if family == "repro" {
        let path = rest.first().expect("a file path");
        families::from_nquads(&std::fs::read_to_string(path).expect("readable file"))
    } else {
        fold_of(family, number(0) as u32, number(1) as u32, number(2) as u32)
    };
    let budget = rest
        .get(3)
        .map_or(1 << 28, |s| s.parse::<u64>().expect("a budget"));
    let cell = format!(
        "{family} {}",
        rest.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
    );
    match engine.as_str() {
        "build" => {
            let ds = fold.dataset();
            println!(
                "RESULT {cell} blanks={} quads={} built",
                fold.blanks,
                ds.quad_count()
            );
        }
        "build-lines" => {
            let lines = fold_lines(&fold);
            println!(
                "RESULT {cell} blanks={} lines={} built",
                fold.blanks,
                lines.len()
            );
        }
        "current" => {
            let ds = fold.dataset();
            let mut control = CanonicalizationControl::new(SearchPolicy::Finite(budget), None);
            let outcome = try_canonical_sha256_view(&*ds, CanonHash::Sha256, &mut control);
            let receipt = control.receipts().last().expect("one receipt");
            let work: SearchWork = receipt.work;
            let verdict = match &outcome {
                Ok(digest) => format!("ok digest={}", digest.to_hex()),
                Err(error) => format!("refused {error:?}").chars().take(60).collect(),
            };
            println!(
                "RESULT {cell} blanks={} budget={budget} work={} frames={} perms={} completed={} {verdict}",
                fold.blanks,
                work.total(),
                work.count(purrdf_core::SearchEvent::RdfcFrame),
                work.count(purrdf_core::SearchEvent::RdfcPermutation),
                receipt.completed,
            );
        }
        "colour" | "colour-literal" | "colour-noorbit" | "colour-overlay" | "colour-flat" => {
            let lines = match engine.as_str() {
                "colour-overlay" => {
                    view_lines::lines_from_view(&*fold.dataset(), view_lines::Presentation::Overlay)
                }
                "colour-flat" => {
                    view_lines::lines_from_view(&*fold.dataset(), view_lines::Presentation::Flat)
                }
                _ => fold_lines(&fold),
            };
            let mode = if engine == "colour-literal" {
                Mode::Literal
            } else {
                Mode::Optimised
            };
            colour_engine(&cell, &lines, budget, mode, engine == "colour-noorbit");
        }
        other => panic!("unknown engine {other}"),
    }
}

fn colour_engine(cell: &str, lines: &[RawLine], budget: u64, mode: Mode, no_orbits: bool) {
    let options = Options {
        mode,
        limit: budget,
        no_forest_path: false,
        no_orbits,
    };
    let (outcome, s) = match colour::label(lines, options) {
        Ok(done) => {
            let digest = purrdf_core::ContentDigest::of(done.document.as_bytes()).to_hex();
            (
                format!("ok labelled={} doc_digest={digest}", done.labels.len()),
                done.stats,
            )
        }
        Err(colour::Refusal(stats)) => ("refused limit".to_owned(), *stats),
    };
    println!(
        "RESULT {cell} blanks={} lines={} budget={budget} units={} visits={} sig_compares={} cert_formed={} cert_compared={} rounds={} canon_calls={} children={} max_depth={} pruned_twin={} pruned_orbit={} forest_paths={} comps={} residual_comps={} largest_comp={} root_nonsingleton={} root_largest_cell={} {outcome}",
        s.nodes,
        s.lines,
        s.units(),
        s.visits,
        s.sig_compares,
        s.cert_formed,
        s.cert_compared,
        s.rounds,
        s.canon_calls,
        s.children,
        s.max_depth,
        s.pruned_twin,
        s.pruned_orbit,
        s.forest_paths,
        s.components,
        s.residual_components,
        s.largest_component,
        s.root_nonsingleton_nodes,
        s.root_largest_cell,
    );
}

fn iso(path: &str, literal_limit: u64) {
    let p = std::path::Path::new(path);
    let mut files: Vec<std::path::PathBuf> = if p.is_dir() {
        std::fs::read_dir(p)
            .expect("readable directory")
            .map(|e| e.expect("entry").path())
            .filter(|f| f.extension().is_some_and(|e| e == "nq" || e == "txt"))
            .collect()
    } else {
        vec![p.to_path_buf()]
    };
    files.sort();
    for f in files {
        let lines = nquads::read(&std::fs::read_to_string(&f).expect("readable file"));
        let name = f
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .into_owned();
        println!("{}", checks::invariance(&name, &lines, literal_limit));
    }
}

/// Canonical documents of every `*.input.nq` of a vector directory, with a manifest.
fn docs(vectors: &str, out: &str, literal_limit: u64) {
    use sha2::{Digest, Sha256};
    let sha = |bytes: &[u8]| purrdf_hash::hex::encode(&Sha256::digest(bytes));
    std::fs::create_dir_all(out).expect("output directory");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(vectors)
        .expect("readable directory")
        .map(|e| e.expect("entry").path())
        .filter(|f| {
            let n = f.to_string_lossy();
            n.ends_with(".input.txt") || n.ends_with("reproducer-sibling-tree.nq")
        })
        .collect();
    files.sort();
    let mut entries = Vec::new();
    for f in files {
        let name = f
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .into_owned();
        let id = name
            .trim_end_matches(".input.txt")
            .trim_end_matches(".nq")
            .to_owned();
        let bytes = std::fs::read(&f).expect("readable file");
        let lines = nquads::read(&String::from_utf8(bytes.clone()).expect("utf-8"));
        let literal = Options {
            limit: literal_limit,
            ..Options::LITERAL
        };
        let (mode, done) = match colour::label(&lines, literal) {
            Ok(done) => ("literal", Some(done)),
            Err(_) => (
                "optimised",
                colour::label(
                    &lines,
                    Options {
                        limit: 2_000_000_000,
                        ..Options::OPTIMISED
                    },
                )
                .ok(),
            ),
        };
        let Some(done) = done else {
            entries.push(format!(
                "    {{\"id\": \"{id}\", \"input\": \"{name}\", \"input_sha256\": \"{}\", \"revision\": \"2\", \"labelled\": false}}",
                sha(&bytes)
            ));
            continue;
        };
        let doc_name = format!("{id}.nq");
        std::fs::write(
            std::path::Path::new(out).join(&doc_name),
            done.document.as_bytes(),
        )
        .expect("write document");
        entries.push(format!(
            "    {{\"id\": \"{id}\", \"input\": \"{name}\", \"input_sha256\": \"{}\", \"document\": \"{doc_name}\", \"document_sha256\": \"{}\", \"has_automorphisms\": {}, \"work_units\": {}, \"produced_by\": \"{mode}\", \"revision\": \"2\", \"labelled\": true}}",
            sha(&bytes),
            sha(done.document.as_bytes()),
            done.stats.equal_leaves > 0 || done.stats.pruned_twin > 0 || done.stats.forest_paths > 0,
            done.stats.units(),
        ));
    }
    let manifest = format!(
        "{{\n  \"revision\": \"2\",\n  \"cases\": [\n{}\n  ]\n}}\n",
        entries.join(",\n")
    );
    std::fs::write(std::path::Path::new(out).join("manifest.json"), manifest)
        .expect("write manifest");
    println!("wrote {} cases to {out}", entries.len());
}

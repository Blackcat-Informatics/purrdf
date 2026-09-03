// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parse every fenced ` ```sparql ` block under one or more Markdown trees,
//! and fail on the first that does not parse.
//!
//! The sibling of `tests/serializer_roundtrip_sweep.rs` for a tree that test
//! cannot see: a RENDERED, TRANSLATED book. That test sweeps the fenced
//! blocks under `docs/**` — the English source — and a gettext `.po` file is
//! reached by no test and no script at all, so a translator who edits a code
//! block (a translated keyword, a full-width `｛` or `。` inside the query)
//! would ship a book whose examples no longer parse with every gate green.
//! `scripts/check-i18n-render.py` renders the translated book to Markdown
//! (`mdbook build` with the `markdown` renderer and the `gettext`
//! preprocessor) and runs this over the result.
//!
//! Every block must parse, as a query or as an update — the same routing the
//! round-trip sweep applies to a doc example — with no ceiling and no ledger:
//! the English source's fences all parse (`shipped_sparql_examples.rs`
//! hard-fails on the first that does not), an untranslated block renders as
//! its English source, and a translated block that stops parsing is a
//! translation defect, never a parser regression to be tallied. A tree with
//! no fenced block at all is a failure too, not a pass: this program's whole
//! failure mode is printing OK over a tree it never read.
//!
//! ```sh
//! cargo run -p purrdf-sparql-algebra --example sweep_sparql_fences -- DIR [DIR ...]
//! ```
//!
//! Exit status 0 when every block parsed, 1 otherwise (or on a usage error).
//! Output goes to stdout; failures name the file and the block's ordinal.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use purrdf_sparql_algebra::SparqlParser;

/// Every `.md` file under `dir`, recursively, in a stable (sorted) order.
fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every fenced ` ```sparql ` block in `text`, whole, as `(ordinal, body)`.
/// The same extraction `serializer_roundtrip_sweep.rs` applies: a fence
/// opener whose info string starts with `sparql`, closed by the next fence.
fn sparql_fences(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut body = String::new();
    let mut ordinal = 0usize;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if in_fence {
            if trimmed.starts_with("```") {
                in_fence = false;
                ordinal += 1;
                out.push((ordinal, std::mem::take(&mut body)));
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        if trimmed.starts_with("```sparql") {
            in_fence = true;
            body.clear();
        }
    }
    out
}

/// Why a block did not parse: the query-lane error and the update-lane error,
/// since a block is tried as both and neither succeeded.
fn parse_failure(parser: &SparqlParser, text: &str) -> Option<String> {
    let Err(query_err) = parser.parse_query(text) else {
        return None;
    };
    match parser.parse_update(text) {
        Ok(_) => None,
        Err(update_err) => Some(format!(
            "as a query: {query_err}\n    as an update: {update_err}"
        )),
    }
}

fn main() -> ExitCode {
    let dirs: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if dirs.is_empty() {
        eprintln!("usage: sweep_sparql_fences DIR [DIR ...]");
        return ExitCode::from(1);
    }
    // The same base the round-trip sweep parses with: a fixture-style relative
    // IRI (`<g>`) in an example is a legitimate query, not a syntax error, and
    // `example.org` is the repository's fixture convention.
    let parser = SparqlParser::new().with_base_iri("https://example.org/corpus/");
    let mut files = 0usize;
    let mut blocks = 0usize;
    let mut failures = Vec::new();
    for dir in &dirs {
        if !dir.is_dir() {
            eprintln!("sweep_sparql_fences: {} is not a directory", dir.display());
            return ExitCode::from(1);
        }
        for file in markdown_files(dir) {
            files += 1;
            let text = match std::fs::read_to_string(&file) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("sweep_sparql_fences: read {}: {e}", file.display());
                    return ExitCode::from(1);
                }
            };
            for (ordinal, body) in sparql_fences(&text) {
                blocks += 1;
                if let Some(why) = parse_failure(&parser, &body) {
                    failures.push(format!(
                        "{}#sparql-block-{ordinal}: does not parse\n    {why}",
                        file.display()
                    ));
                }
            }
        }
    }
    if blocks == 0 {
        eprintln!(
            "sweep_sparql_fences: no fenced ```sparql block in {files} Markdown file(s) under \
             the given tree(s) — a sweep that read nothing proves nothing"
        );
        return ExitCode::from(1);
    }
    if !failures.is_empty() {
        eprintln!(
            "sweep_sparql_fences: {} of {blocks} fenced sparql block(s) do not parse:\n\n{}",
            failures.len(),
            failures.join("\n\n")
        );
        return ExitCode::from(1);
    }
    println!(
        "sweep_sparql_fences: OK — {blocks} fenced sparql block(s) in {files} Markdown file(s) \
         all parse as a query or an update"
    );
    ExitCode::SUCCESS
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parser's output over every SPARQL request text in the repository, pinned.
//!
//! Every `.rq` (query) and `.ru` (update) file under the in-repo corpora is parsed and
//! its outcome recorded in one committed golden, `tests/goldens/ast_snapshot.golden`:
//!
//! * a text that parses is recorded as the `{:?}` rendering of the [`Query`] or
//!   [`Update`] it produced — the whole algebra, every node and every term;
//! * a text that is refused is recorded as the [`ParseError`] variant and the byte
//!   offset it names, never the message, whose wording is free to change.
//!
//! Any difference fails. The golden is a snapshot of what the parser does today, so a
//! change to the parser that keeps its observable output is proven to keep it here, and
//! a change that moves it has to regenerate the golden and show the diff.
//!
//! # The corpora
//!
//! [`CORPORA`] names them: the W3C SPARQL 1.1/1.2 and first-party manifests the
//! conformance harness loads, that crate's CONSTRUCT/DESCRIBE corpus, the SEP-0009
//! composite-datatype vectors, the governor vectors, this crate's own update corpus,
//! and the repository's hand-authored and generated query trees. They are read by path;
//! no dependency on the crates that own them is needed.
//!
//! # Determinism
//!
//! Files are walked in sorted relative-path order. Each is parsed against a base IRI
//! derived from its relative path, with the extension- and property-function
//! namespaces the conformance harness declares, so every file is read the way the
//! harness reads it and no two runs can differ. Nothing the parser does depends on the
//! stack of the thread it runs on, so the snapshot is rendered on the test's own thread.
//!
//! # Regenerating
//!
//! ```text
//! cargo test -p purrdf-sparql-algebra --test ast_snapshot -- --ignored regenerate_ast_snapshot
//! ```

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use purrdf_sparql_algebra::{ParseError, ParserOptions, SparqlParser};
use purrdf_testkit::paths::workspace_root;

/// Every directory walked, relative to the repository root.
const CORPORA: &[&str] = &[
    "crates/sparql-conformance/suite",
    "crates/sparql-conformance/corpus",
    "crates/sparql-algebra/tests/update",
    "vectors/sparql-cdt",
    "vectors/sparql-governors/cases",
    "queries",
    "generated/queries",
];

/// The base every file's relative path is appended to.
const BASE_ROOT: &str = "http://purrdf.test/snapshot/";

/// The extension-function namespace the conformance harness declares.
const EXT_NS: &str = "https://example.org/ext/";

/// The property-function namespace the conformance harness declares.
const REL_NS: &str = "https://example.org/rel/";

/// A floor on the number of files walked, so a corpus that moved away cannot leave the
/// snapshot passing over nothing.
const MIN_FILES: usize = 1500;

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/goldens/ast_snapshot.golden")
}

/// Which grammar a file is read with, by extension.
#[derive(Clone, Copy)]
enum Form {
    Query,
    Update,
}

impl Form {
    const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Update => "update",
        }
    }
}

fn form_of(path: &Path) -> Option<Form> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rq") => Some(Form::Query),
        Some("ru") => Some(Form::Update),
        _ => None,
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("read corpus directory {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("corpus directory entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if form_of(&path).is_some() {
            out.push(path);
        }
    }
}

/// Every corpus file, as `(relative path with '/' separators, absolute path)`, sorted.
fn corpus_files() -> Vec<(String, PathBuf)> {
    let root = workspace_root();
    let mut files = Vec::new();
    for corpus in CORPORA {
        let mut found = Vec::new();
        collect(&root.join(corpus), &mut found);
        assert!(!found.is_empty(), "corpus {corpus} holds no .rq/.ru file");
        for path in found {
            let relative = path
                .strip_prefix(&root)
                .expect("a corpus file lies under the repository root")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((relative, path));
        }
    }
    files.sort();
    files.dedup_by(|a, b| a.0 == b.0);
    files
}

/// The variant and the byte offset. Never the message.
fn render_error(error: &ParseError) -> String {
    let at = error
        .byte_offset()
        .map_or_else(|| "-".to_owned(), |at| at.to_string());
    match error {
        ParseError::Lex { .. } => format!("Lex at={at}"),
        ParseError::Syntax { .. } => format!("Syntax at={at}"),
        ParseError::Unsupported(_) => format!("Unsupported at={at}"),
        ParseError::Iri { .. } => format!("Iri at={at}"),
        ParseError::CdtArity { found, .. } => format!("CdtArity at={at} found={found}"),
        other => panic!("unpinned ParseError variant {other:?}"),
    }
}

fn render_one(relative: &str, path: &Path, out: &mut String) {
    let form = form_of(path).expect("only .rq/.ru files are collected");
    writeln!(out, "== {relative} {}", form.label()).expect("writing to a String cannot fail");
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("read {relative}: {error}"));
    let Ok(text) = String::from_utf8(bytes) else {
        out.push_str("not-utf8\n");
        return;
    };
    let parser = SparqlParser::new().with_base_iri(format!("{BASE_ROOT}{relative}"));
    let options = ParserOptions {
        extension_fn_namespaces: vec![EXT_NS.to_owned()],
        property_fn_namespaces: vec![REL_NS.to_owned()],
        property_fn_iris: Vec::new(),
    };
    let line = match form {
        Form::Query => parser
            .parse_query_with(&text, &options)
            .map(|query| format!("ok {query:?}")),
        Form::Update => parser
            .parse_update_with(&text, &options)
            .map(|update| format!("ok {update:?}")),
    }
    .unwrap_or_else(|error| format!("err {}", render_error(&error)));
    out.push_str(&line);
    out.push('\n');
}

/// The whole snapshot.
fn render_snapshot() -> String {
    let files = corpus_files();
    assert!(
        files.len() >= MIN_FILES,
        "the snapshot corpus shrank: only {} files were found",
        files.len()
    );
    let mut out = String::new();
    for (relative, path) in &files {
        render_one(relative, path, &mut out);
    }
    out
}

/// Each entry of a snapshot, as `(header, body)`.
fn entries(snapshot: &str) -> Vec<(&str, &str)> {
    let mut lines = snapshot.lines();
    let mut out = Vec::new();
    while let Some(header) = lines.next() {
        out.push((header, lines.next().unwrap_or("")));
    }
    out
}

/// A bounded prefix of a possibly very long line, for a failure message.
fn clip(line: &str) -> &str {
    &line[..line.floor_char_boundary(600)]
}

#[test]
fn the_parser_output_over_every_in_repo_request_matches_the_golden() {
    let actual = render_snapshot();
    let golden = std::fs::read_to_string(golden_path()).unwrap_or_else(|error| {
        panic!(
            "read {}: {error}; regenerate with `cargo test -p purrdf-sparql-algebra --test \
             ast_snapshot -- --ignored regenerate_ast_snapshot`",
            golden_path().display()
        )
    });
    if actual == golden {
        return;
    }
    let (actual_entries, golden_entries) = (entries(&actual), entries(&golden));
    let mut report = String::new();
    let mut differing = 0_usize;
    let actual_headers: std::collections::BTreeMap<&str, &str> =
        actual_entries.iter().copied().collect();
    let golden_headers: std::collections::BTreeMap<&str, &str> =
        golden_entries.iter().copied().collect();
    for (header, body) in &golden_entries {
        match actual_headers.get(header) {
            None => {
                differing += 1;
                if differing <= 10 {
                    writeln!(report, "  missing now: {header}").expect("write");
                }
            }
            Some(now) if now != body => {
                differing += 1;
                if differing <= 10 {
                    writeln!(
                        report,
                        "  {header}\n    golden: {}\n    actual: {}",
                        clip(body),
                        clip(now)
                    )
                    .expect("write");
                }
            }
            Some(_) => {}
        }
    }
    for (header, _) in &actual_entries {
        if !golden_headers.contains_key(header) {
            differing += 1;
            if differing <= 10 {
                writeln!(report, "  new file: {header}").expect("write");
            }
        }
    }
    panic!(
        "the parser's output moved on {differing} file(s) (first ten shown):\n{report}\nif the \
         change is intended, regenerate with `cargo test -p purrdf-sparql-algebra --test \
         ast_snapshot -- --ignored regenerate_ast_snapshot` and review the diff"
    );
}

/// Regeneration path for the snapshot. Ignored by default because it WRITES the
/// committed golden.
#[test]
#[ignore = "regeneration path: writes the committed golden"]
fn regenerate_ast_snapshot() {
    let path = golden_path();
    std::fs::create_dir_all(path.parent().expect("the golden has a parent directory"))
        .expect("create tests/goldens");
    std::fs::write(&path, render_snapshot()).expect("write the golden");
    println!("wrote {}", path.display());
}

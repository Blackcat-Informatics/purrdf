// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ShEx 2.1 syntax conformance over the vendored shexTest corpus
//! (`vectors/shexTest`, upstream tag v2.1.0).
//!
//! Phase-1 harness: driven by directory listings (the Turtle manifests take
//! over with the validator wave):
//!
//! * every `negativeSyntax/*.shex` MUST fail [`purrdf_shex::parse_shexc`];
//! * every `negativeStructure/*.shex` MUST parse and then fail
//!   [`purrdf_shex::check_structure`];
//! * every `schemas/*.shex` MUST parse, and its paired `.json` MUST parse via
//!   [`purrdf_shex::parse_shexj`] — modulo the explicit XFAIL ledgers below.
//!
//! XFAIL entries must actually fail: a passing xfail is a test error (it
//! means the ledger is stale).
//!
//! Every parse here — ShExC *and* ShExJ — is given the document's own retrieval
//! IRI as base, the convention `validation_conformance.rs` and the upstream
//! harness both use. Nineteen corpus schemas carry a relative `IMPORT` (`IMPORT
//! <1dot>`) and no `BASE` directive, and their frozen `.json` ground truth stores
//! the same relative reference (`"imports": ["1dot"]`); without a base neither
//! side has any way to denote what it imports. ShExJ is a JSON-LD dialect, so its
//! IRI-valued members are document-relative exactly as ShExC's IRIREFs are, which
//! is why the two syntaxes reach identical ASTs from one base rather than needing
//! the ground truth patched up after the fact.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use purrdf_shex::{check_structure, parse_shexc, parse_shexj};

/// Exact corpus sizes (the vendored tree is byte-frozen; a change here means
/// the vectors were touched, which the harness must notice).
const NEGATIVE_SYNTAX_COUNT: usize = 99;
const NEGATIVE_STRUCTURE_COUNT: usize = 14;
const SCHEMAS_SHEXC_COUNT: usize = 425;
const SCHEMAS_SHEXJ_COUNT: usize = 420;

/// The URL prefix the vendored tree mirrors.
const CORPUS_URL: &str = "https://raw.githubusercontent.com/shexSpec/shexTest/master/";

/// ShExC schemas we cannot parse yet, each with a reason.
const XFAIL_SHEXC: &[(&str, &str)] = &[];

/// ShExJ ground-truth documents we cannot parse yet, each with a reason.
const XFAIL_SHEXJ: &[(&str, &str)] = &[];

/// Pairs whose ShExC parse is not expected to equal the ShExJ ground truth,
/// each with a reason.
const XFAIL_CROSS: &[(&str, &str)] = &[(
    "start2RefS2",
    "upstream corpus inconsistency: start2RefS2.shex constrains predicate \
     <http://a.example/p2> but its frozen .json ground truth says \
     <http://a.example/p1>",
)];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors/shexTest")
}

fn shex_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().is_some_and(|x| x == "shex")).then_some(path)
        })
        .collect();
    files.sort();
    files
}

fn stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
}

/// The retrieval IRI of a vendored corpus document: the base its relative IRI
/// references resolve against (RFC-3986 §5.1.3).
fn document_url(path: &Path) -> String {
    let dir = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| panic!("corpus document outside a corpus directory: {path:?}"));
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| panic!("corpus document has no file name: {path:?}"));
    format!("{CORPUS_URL}{dir}/{file}")
}

#[test]
fn negative_syntax_all_rejected() {
    let dir = corpus().join("negativeSyntax");
    let files = shex_files(&dir);
    assert_eq!(
        files.len(),
        NEGATIVE_SYNTAX_COUNT,
        "corpus drift in {dir:?}"
    );
    let mut wrongly_accepted = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).expect("read .shex");
        if parse_shexc(&source, Some(&document_url(path))).is_ok() {
            wrongly_accepted.push(stem(path).to_owned());
        }
    }
    assert!(
        wrongly_accepted.is_empty(),
        "negativeSyntax documents wrongly accepted: {wrongly_accepted:?}"
    );
}

#[test]
fn negative_structure_all_parse_then_fail_structure() {
    let dir = corpus().join("negativeStructure");
    let files = shex_files(&dir);
    assert_eq!(
        files.len(),
        NEGATIVE_STRUCTURE_COUNT,
        "corpus drift in {dir:?}"
    );
    let mut parse_failures = Vec::new();
    let mut wrongly_well_formed = Vec::new();
    for path in &files {
        let source = fs::read_to_string(path).expect("read .shex");
        match parse_shexc(&source, Some(&document_url(path))) {
            Err(e) => parse_failures.push(format!("{}: {e}", stem(path))),
            Ok(schema) => {
                if check_structure(&schema).is_ok() {
                    wrongly_well_formed.push(stem(path).to_owned());
                }
            }
        }
    }
    assert!(
        parse_failures.is_empty(),
        "negativeStructure documents must parse: {parse_failures:?}"
    );
    assert!(
        wrongly_well_formed.is_empty(),
        "negativeStructure documents wrongly accepted by check_structure: {wrongly_well_formed:?}"
    );
}

#[test]
fn schemas_shexc_all_parse() {
    let dir = corpus().join("schemas");
    let files = shex_files(&dir);
    assert_eq!(files.len(), SCHEMAS_SHEXC_COUNT, "corpus drift in {dir:?}");
    let xfail: BTreeSet<&str> = XFAIL_SHEXC.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        xfail.len(),
        XFAIL_SHEXC.len(),
        "duplicate entries in XFAIL_SHEXC"
    );
    let mut failures = Vec::new();
    let mut stale_xfails = Vec::new();
    for path in &files {
        let name = stem(path).to_owned();
        let source = fs::read_to_string(path).expect("read .shex");
        let result = parse_shexc(&source, Some(&document_url(path)));
        if xfail.contains(name.as_str()) {
            if result.is_ok() {
                stale_xfails.push(name);
            }
        } else if let Err(e) = result {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} schemas failed ShExC parse:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        stale_xfails.is_empty(),
        "XFAIL_SHEXC entries now pass (remove them): {stale_xfails:?}"
    );
}

#[test]
fn schemas_shexj_all_parse() {
    let dir = corpus().join("schemas");
    // Paired ground truth only: every .shex stem with a .json sibling. The two
    // non-schema JSON files (coverage.json, representationTests.json) have no
    // .shex pair and are excluded by construction.
    let mut pairs: Vec<PathBuf> = shex_files(&dir)
        .into_iter()
        .map(|p| p.with_extension("json"))
        .filter(|p| p.exists())
        .collect();
    pairs.sort();
    assert_eq!(pairs.len(), SCHEMAS_SHEXJ_COUNT, "corpus drift in {dir:?}");
    let xfail: BTreeSet<&str> = XFAIL_SHEXJ.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        xfail.len(),
        XFAIL_SHEXJ.len(),
        "duplicate entries in XFAIL_SHEXJ"
    );
    let mut failures = Vec::new();
    let mut stale_xfails = Vec::new();
    for path in &pairs {
        let name = stem(path).to_owned();
        let source = fs::read_to_string(path).expect("read .json");
        let result = parse_shexj(&source, Some(&document_url(path)));
        if xfail.contains(name.as_str()) {
            if result.is_ok() {
                stale_xfails.push(name);
            }
        } else if let Err(e) = result {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} ground-truth documents failed ShExJ parse:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        stale_xfails.is_empty(),
        "XFAIL_SHEXJ entries now pass (remove them): {stale_xfails:?}"
    );
}

/// Stronger-than-required gate: for every paired schema, the ShExC parse must
/// produce exactly the AST of its ShExJ ground truth (this is what the
/// upstream "representation" tests assert).
#[test]
fn schemas_shexc_matches_shexj_ground_truth() {
    let dir = corpus().join("schemas");
    let xfail: BTreeSet<&str> = XFAIL_CROSS.iter().map(|(name, _)| *name).collect();
    let mut mismatches = Vec::new();
    let mut stale_xfails = Vec::new();
    let mut compared = 0usize;
    for shex_path in shex_files(&dir) {
        let json_path = shex_path.with_extension("json");
        if !json_path.exists() {
            continue;
        }
        let name = stem(&shex_path).to_owned();
        let url = document_url(&shex_path);
        let shex = fs::read_to_string(&shex_path).expect("read .shex");
        let json = fs::read_to_string(&json_path).expect("read .json");
        // The SAME base for both syntaxes: the ShExC document's retrieval IRI. The
        // `.json` sibling differs from it only in its final extension, and no
        // corpus document's meaning depends on that segment, so one base keeps the
        // comparison about the parse rather than about which file it came from.
        let (Ok(from_shexc), Ok(truth)) = (
            parse_shexc(&shex, Some(&url)),
            parse_shexj(&json, Some(&url)),
        ) else {
            continue; // parse failures are owned by the two tests above
        };
        compared += 1;
        let matches = from_shexc == truth;
        if xfail.contains(name.as_str()) {
            if matches {
                stale_xfails.push(name);
            }
        } else if !matches {
            mismatches.push(name);
        }
    }
    assert_eq!(
        compared, SCHEMAS_SHEXJ_COUNT,
        "cross-check coverage drifted"
    );
    assert!(
        mismatches.is_empty(),
        "{} ShExC parses diverge from the ShExJ ground truth: {mismatches:?}",
        mismatches.len()
    );
    assert!(
        stale_xfails.is_empty(),
        "XFAIL_CROSS entries now pass (remove them): {stale_xfails:?}"
    );
}

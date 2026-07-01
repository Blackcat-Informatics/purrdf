// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared corpus-classification helpers for the native golden-capture binary
//! (EPIC #906 Task 2/8).
//!
//! These pure helpers — corpus enumeration, the nondeterministic / multi-query /
//! deferred-construct classifiers, and the stable solution-row key — are used by the
//! `capture_sparql_goldens` binary to freeze the native engine's outputs as the
//! committed conformance goldens. They are oxigraph-free and ride the always-on `gts`
//! feature.

use std::path::{Path, PathBuf};

/// Nondeterministic SPARQL builtins: results vary per-call, so a frozen oxigraph
/// golden is not meaningful. The capture writes a `.nondeterministic` marker (the
/// Task-4 gate runs native for well-formedness only), and the parity sweep runs
/// native only and asserts well-formed output.
#[must_use]
pub fn is_nondeterministic(query_text: &str) -> bool {
    let lower = query_text.to_lowercase();
    lower.contains("now(")
        || lower.contains("rand(")
        || lower.contains("uuid(")
        || lower.contains("struuid(")
}

/// Returns true if the error message matches a known-deferred SPARQL construct
/// (property paths, SERVICE federation, LATERAL, DESCRIBE, RDF-1.2 triple terms in
/// patterns). These are in-scope for later S8 (#914) / S6b (#928) / SPARQL-1.2 work;
/// an Err here is expected, not a gap.
#[must_use]
pub fn is_deferred_construct(err_msg: &str) -> bool {
    let lower = err_msg.to_lowercase();
    lower.contains("property path")
        || lower.contains("path expression")
        || lower.contains("service")
        || lower.contains("lateral")
        || lower.contains("describe")
        // The algebra type names the parser surfaces for path operators:
        || lower.contains("pathexpr")
        || lower.contains("unsupported path")
        || lower.contains("path operator")
        // Catch-all: any "not supported" / "not implemented" mentioning path
        || (lower.contains("not support") && lower.contains("path"))
        || (lower.contains("not implement") && lower.contains("path"))
        // The algebra uses ZeroOrMore / OneOrMore / ZeroOrOne for * + ? paths
        || lower.contains("zeroormore")
        || lower.contains("oneormore")
        || lower.contains("zeroorone")
        || lower.contains("alternative path")
        || lower.contains("inverse path")
        || lower.contains("sequence path")
        || lower.contains("negated path")
        // RDF-1.2 / SPARQL 1.2: variable inside a quoted triple term (triple term
        // pattern matching with unbound variables). The native engine explicitly scopes
        // this out of S6 — "S6 scope" in the error. Deferred to SPARQL-1.2 work.
        || lower.contains("variable inside a quoted triple")
        || lower.contains("quoted triple term")
        || (lower.contains("s6 scope") && lower.contains("quoted"))
}

/// Returns true if the query text is a multi-query file (contains more than one
/// top-level SPARQL query statement). SPARQL allows only one query per invocation;
/// some corpus files contain multiple queries separated by comments (e.g. for
/// documentation purposes). Such files cannot be run by a single engine invocation
/// and are skipped with a `.skip-multi` marker / a log note.
///
/// Detection: count top-level SELECT/CONSTRUCT/ASK/DESCRIBE keywords that appear
/// at the start of a non-comment line (after stripping leading whitespace).
#[must_use]
pub fn is_multi_query_file(query_text: &str) -> bool {
    let mut count = 0usize;
    for line in query_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let upper = trimmed.to_uppercase();
        if upper.starts_with("SELECT ")
            || upper.starts_with("SELECT\t")
            || upper.starts_with("CONSTRUCT ")
            || upper.starts_with("CONSTRUCT\t")
            || upper.starts_with("CONSTRUCT{")
            || upper.starts_with("ASK ")
            || upper.starts_with("ASK\t")
            || upper.starts_with("ASK{")
            || upper.starts_with("DESCRIBE ")
            || upper.starts_with("DESCRIBE\t")
        {
            count += 1;
            if count > 1 {
                return true;
            }
        }
    }
    false
}

/// Repo root as the corpus enumerator derives it (`crates/rdf/../..`). Used both to
/// enumerate the corpus and to derive the stable repo-relative key for mirroring
/// goldens.
#[must_use]
pub fn corpus_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Collect every `*.rq` file under the two corpus roots (`queries/**` +
/// `generated/queries/**`), sorted for determinism.
#[must_use]
pub fn collect_corpus_files() -> Vec<PathBuf> {
    let repo_root = corpus_repo_root();
    let roots = [
        repo_root.join("queries"),
        repo_root.join("generated").join("queries"),
    ];
    let mut files = Vec::new();
    for root in &roots {
        if !root.exists() {
            continue;
        }
        collect_rq_recursive(root, &mut files);
    }
    files.sort();
    files
}

fn collect_rq_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rq_recursive(&path, out);
        } else if path.extension().is_some_and(|e| e == "rq") {
            out.push(path);
        }
    }
}

/// A stable, order-insensitive key for a solution row. SELECT goldens are the
/// sorted multiset of these.
#[must_use]
pub fn row_key(row: &[Option<crate::TermValue>]) -> String {
    format!("{row:?}")
}

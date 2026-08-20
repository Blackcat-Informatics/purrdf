// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared corpus-classification helpers for the native golden-capture binary.
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

/// Returns true if the error message matches the SPARQL evaluator's actual,
/// enumerated unsupported residue — see `purrdf_sparql_eval`'s crate docs
/// (`lib.rs`'s "Hard-fail, no degraded fallback" section) for the authoritative
/// list: a variable-bound quoted-triple-term component in a BGP or
/// property-path pattern (`convert::ground_term_pattern_to_value` /
/// `ground_triple_pattern_to_value`), an unresolved custom SPARQL function IRI
/// (`expr::eval_function`'s `Function::Custom` fallthrough), `heldIn` called
/// without a caller-supplied standpoint-predicate configuration
/// (`expr::eval_held_in`), or a manually constructed graph pattern whose
/// nesting exceeds the parser's safety bound
/// (`governor::soundness::validate_graph_pattern_depth`).
///
/// Property paths, `SERVICE` federation, `LATERAL`, `DESCRIBE`, property
/// functions, custom aggregates, and `UPDATE` are ALL evaluated in-engine —
/// none of them is out of scope, so an error naming one of THOSE is a real
/// gap, not an expected deferral, and must not be classified here (a prior,
/// broader substring match once misrouted exactly these implemented features).
#[must_use]
pub fn is_deferred_construct(err_msg: &str) -> bool {
    let lower = err_msg.to_lowercase();
    lower.contains("quoted triple term")
        || lower.contains("custom sparql function <")
        || lower.contains("heldin requires a standpoint predicate configuration")
        || lower.contains("graph pattern nesting exceeds the safety limit")
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

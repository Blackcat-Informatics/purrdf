// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hygiene gate: PurRDF mints no vocabulary IRIs (`AGENTS.md`).
//!
//! Two prior mistakes both minted `https://purrdf.dev/ns/...#` as PurRDF's own
//! vocabulary: the SPARQL-XML results writer once emitted `purrdf:dir` under
//! `https://purrdf.dev/ns/results#` (deleted — the writer no longer has a
//! `PURRDF_NS` constant at all), and this crate's manifest-extension vocabulary
//! once minted `https://purrdf.dev/ns/conformance-manifest#` (respelled under
//! `example.org`, alongside `EXT_NS`/`REL_NS`/`LOSS_NS`/`TABLE_NS` in
//! `crate::run`, which never minted a `purrdf.dev` IRI in the first place).
//!
//! `purrdf.dev` is one spelling of the same mistake, not the whole mistake: any
//! host that brands an IRI as "PurRDF's own" reproduces it just as surely —
//! `purrdf.io`, `purrdf.org`, `purrdf.example`, or the organisation domain
//! `AGENTS.md` names outright, `blackcatinformatics.ca`, wearing a `purrdf`-
//! branded subdomain (`ns.blackcatinformatics.ca`, …). [`BANNED_HOSTS`] is the
//! closed list this gate checks; a tracked line naming any of them, anywhere
//! outside [`EXEMPT_SITES`], fails the same way a `purrdf.dev` line does. This
//! test proves none of them is reachable again: a banned host anywhere in the
//! TRACKED tree is a repeat of the mistake, not a new feature.
//!
//! The standing exemption is:
//! - `crates/sparql-results/src/xml_read.rs`'s
//!   `tolerates_legacy_dir_spellings`/`its_dir_takes_priority_over_legacy_spellings`
//!   tests. They construct SPARQL-XML fixtures carrying the OLD (now-deleted)
//!   results-XML namespace to prove the READER still tolerates data written by
//!   that prior, since-fixed PurRDF build — or by anyone else who happened to
//!   pick the same string. Recognizing a legacy spelling on READ is not minting
//!   it, the same way recognizing the third-party `its:dir` attribute the same
//!   two tests also cover is not minting `http://www.w3.org/2005/11/its`.
//!   `crates/rdf-core/src/turtle.rs`'s reifier/annotation-emission tests spell
//!   their sample predicates under `example.org` and carry no exemption here.
//!
//! Enumeration is driven by `git ls-files` rather than a filesystem walk, so
//! the scan covers exactly the committed tree — untracked build artifacts
//! (`target/`, `bindings/python/.venv`, …) are never scanned, matching
//! `scripts/check-issue-refs.py`'s rationale for the same choice.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The closed set of project-branded vocabulary hosts this gate refuses. Each
/// is a plain substring match against a tracked line — no regex, so the
/// predicate is exactly as easy to audit as [`EXEMPT_SITES`] below.
///
/// - `purrdf.dev` — the original mistake (see the module docs).
/// - `purrdf.io` / `purrdf.org` / `purrdf.example` — the same mistake under a
///   different TLD; nothing in the tracked tree currently needs any of them.
/// - `ns.blackcatinformatics.ca` — the organisation domain `AGENTS.md` (~85)
///   names outright ("Never hardcode a `blackcatinformatics.ca` namespace in
///   library code") wearing a vocabulary-shaped subdomain. The bare apex
///   `blackcatinformatics.ca` is NOT banned here: it is the project's real
///   homepage (`Cargo.toml`'s `homepage`, the READMEs) and, in test fixtures
///   across `crates/slice`/`crates/shapes`, the GMEOW ontology's real
///   namespace — GMEOW is a *consumer* of PurRDF, not PurRDF's own mint, and
///   AGENTS.md carves that case out by name. A vocabulary-shaped subdomain
///   like `ns.blackcatinformatics.ca` has no such consumer to point to.
const BANNED_HOSTS: &[&str] = &[
    "purrdf.dev",
    "purrdf.io",
    "purrdf.org",
    "purrdf.example",
    "ns.blackcatinformatics.ca",
];

/// `(file relative to the workspace root, the exact line's trimmed content)` —
/// every occurrence of a [`BANNED_HOSTS`] entry outside this list fails the
/// gate. An entry whose file no longer contains that exact trimmed line is
/// ALSO a failure: a stale exemption means either the mint was never actually
/// removed or the sanctioned fixture drifted out from under its own
/// justification above, and either way the exemption list is no longer
/// describing the tree.
const EXEMPT_SITES: &[(&str, &str)] = &[
    (
        "crates/sparql-results/src/xml_read.rs",
        r#"<literal xml:lang="en" purrdf:dir="ltr" xmlns:purrdf="https://purrdf.dev/ns/results#">hello</literal>"#,
    ),
    (
        "crates/sparql-results/src/xml_read.rs",
        r#"<literal xml:lang="en" xmlns:its="http://www.w3.org/2005/11/its" its:dir="rtl" dir="ltr" purrdf:dir="ltr" xmlns:purrdf="https://purrdf.dev/ns/results#">hello</literal>"#,
    ),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// This file's own path, relative to the workspace root. The scan below must
/// skip it: this file's doc comment, [`BANNED_HOSTS`]/[`EXEMPT_SITES`]
/// literals, and predicate strings all legitimately spell every banned host
/// to name the very strings the scan is looking for, and once the file is
/// tracked `git ls-files` includes it like any other file — it would
/// otherwise flag itself.
fn own_relative_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("no_purrdf_dev_vocabulary.rs")
        .canonicalize()
        .expect("this test's own source file resolves")
        .strip_prefix(repo_root())
        .expect("this test file lives under the workspace root")
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every path `git` tracks in the working tree, relative to `root`, forward-slash
/// separated regardless of host path convention.
fn tracked_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files", "-z"])
        .output()
        .expect("git ls-files runs");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("git ls-files output is UTF-8");
    let mut files: Vec<String> = stdout
        .split('\0')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    files.sort();
    files
}

#[test]
fn no_branded_vocabulary_iri_is_minted_outside_the_closed_exemption_list() {
    let root = repo_root();
    let files = tracked_files(&root);
    assert!(
        files.len() > 500,
        "git ls-files returned only {} paths — the enumeration is broken, not the repo",
        files.len()
    );

    let own = own_relative_path();
    let mut unexempted = Vec::new();
    let mut exempt_seen = vec![false; EXEMPT_SITES.len()];
    for rel in &files {
        if *rel == own {
            continue;
        }
        let path = root.join(rel);
        let Ok(content) = std::fs::read_to_string(&path) else {
            // Not UTF-8 text (a binary fixture) — no IRI string can live in it.
            continue;
        };
        if !BANNED_HOSTS.iter().any(|host| content.contains(host)) {
            continue;
        }
        for (line_no, line) in content.lines().enumerate() {
            if !BANNED_HOSTS.iter().any(|host| line.contains(host)) {
                continue;
            }
            let trimmed = line.trim();
            match EXEMPT_SITES
                .iter()
                .position(|(file, exempt_line)| *file == rel.as_str() && *exempt_line == trimmed)
            {
                Some(idx) => exempt_seen[idx] = true,
                None => unexempted.push(format!("{rel}:{}: {trimmed}", line_no + 1)),
            }
        }
    }

    assert!(
        unexempted.is_empty(),
        "a project-branded vocabulary IRI ({:?}) is minted outside the closed exemption list — \
         PurRDF mints no vocabulary IRIs (AGENTS.md); respell it under example.org, or add it \
         to `EXEMPT_SITES` in this file with a justification if it is genuine read-tolerance or \
         round-tripped third-party test data:\n{}",
        BANNED_HOSTS,
        unexempted.join("\n")
    );
    for (idx, seen) in exempt_seen.iter().enumerate() {
        assert!(
            *seen,
            "EXEMPT_SITES[{idx}] ({:?}) no longer matches any tracked line — the exemption \
             is stale, delete it",
            EXEMPT_SITES[idx]
        );
    }
}

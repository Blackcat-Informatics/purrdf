// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Prefix-consistency lint (#1009 §2).
//!
//! The article's §2 win — "a prefix-consistency lint falls out for free" once the
//! prefix authority ([`crate::mapping_support::PREFIX_REGISTRY`]) is the single named
//! set. This lint enforces that authored sources never *shadow* a registry prefix
//! with a different namespace: a `@prefix mf:` (or SPARQL `PREFIX mf:`) declaration
//! bound to anything other than the registry's `mf` namespace is a latent
//! correctness bug, because the SSSOM/EDOAL/SPARQL CURIE shortener resolves CURIEs
//! through the *registry*, not the per-file `@prefix` line. A shadowed prefix means
//! an authored CURIE and its emitted/shortened form disagree on what `mf:` means.
//!
//! Scope (deliberately narrow to avoid false positives):
//!   * Only **registry** prefix *names* are policed. A prefix absent from the
//!     registry (the per-example `ex:`, or an alternative local name like `bf:` for
//!     the registry's `bibframe`) is free to bind any namespace — those never feed
//!     the registry-driven shortener. The registry comment documents this: sources
//!     legitimately use *different prefix names for the same namespace*.
//!   * A registry prefix bound to its own registry namespace (the common case) is
//!     fine, declared in any number of files.
//!   * A registry prefix bound to a *different* namespace anywhere is an ERROR.
//!
//! Hard-fail, no warning-only (CONSTITUTION / no-optionality). Surfaced as a
//! `mapping-compile.prefix-consistency` [`ProjectionDiagnostic`], and enforced as a
//! hard error in the `mappings` stage so `regenerate` / `check-generated` /
//! `make check` all fail on a shadow.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::diagnostics::ProjectionDiagnostic;
use crate::error::SliceError;
use crate::mapping_support::effective_registry;
use crate::vocab::SliceVocab;

/// Authored source roots scanned for prefix declarations. The `generated/` tree is
/// excluded: those artifacts are *emitted* from the registry and are consistent by
/// construction (scanning them would only re-check the emitter).
const SCAN_ROOTS: &[&str] = &[
    "slices",
    "dsl",
    "ontology",
    "queries",
    "metadata",
    "governance",
];

/// One `@prefix`/`PREFIX name: <ns>` declaration: Turtle's `@prefix`/`PREFIX` and
/// SPARQL's case-insensitive `PREFIX`, capturing the prefix name and the namespace.
fn declaration_re() -> Regex {
    Regex::new(r"(?im)^\s*@?prefix\s+([A-Za-z][A-Za-z0-9_.-]*)\s*:\s*<([^>]*)>")
        .expect("static prefix-declaration regex")
}

/// Recursively collect `.ttl` / `.rq` files under `dir` (a missing root yields
/// nothing; a transient FS error surfaces — no silent drop).
fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), SliceError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        // `read_dir` already populates the entry's file type, so `file_type()` avoids
        // the extra `stat()` syscalls that `path.is_dir()` + `path.is_symlink()` would
        // each issue per entry. `FileType::is_dir()` does NOT follow symlinks, so a
        // symlinked directory is `false` here and is naturally skipped (no recursion)
        // without a separate `is_symlink()` check — while a symlinked `.ttl`/`.rq`
        // file still falls through to the extension test and is scanned, as before.
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_sources(&path, out)?;
        } else if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("ttl" | "rq")
        ) {
            out.push(path);
        }
    }
    Ok(())
}

/// Run the prefix-consistency lint over the authored sources under `root`.
///
/// Returns one `prefix-consistency` ERROR [`ProjectionDiagnostic`] per registry
/// prefix that is bound to a non-registry namespace, sorted by `(prefix, namespace,
/// file)`. An empty result means every registry prefix is used consistently.
///
/// # Errors
///
/// Returns [`SliceError`] on a filesystem error reading the scanned tree (a missing
/// root is not an error — it simply contributes no sources).
pub fn lint_prefix_consistency(
    root: &Path,
    vocab: &SliceVocab,
) -> Result<Vec<ProjectionDiagnostic>, SliceError> {
    let effective = effective_registry(vocab);
    let registry: BTreeMap<&str, &str> = effective
        .iter()
        .map(|(p, n)| (p.as_str(), n.as_str()))
        .collect();
    let re = declaration_re();

    let mut files: Vec<PathBuf> = Vec::new();
    for scan in SCAN_ROOTS {
        collect_sources(&root.join(scan), &mut files)?;
    }
    files.sort();

    // Deterministic accumulation: (prefix, namespace) → first relative file seen.
    let mut shadows: BTreeMap<(String, String), String> = BTreeMap::new();
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .into_owned();
        for caps in re.captures_iter(&text) {
            let prefix = caps[1].to_owned();
            let namespace = caps[2].to_owned();
            // Only registry prefixes are policed; only a NON-registry namespace is a shadow.
            if let Some(&canonical) = registry.get(prefix.as_str()) {
                if namespace != canonical {
                    shadows
                        .entry((prefix, namespace))
                        .or_insert_with(|| rel.clone());
                }
            }
        }
    }

    let mut diagnostics: Vec<ProjectionDiagnostic> = shadows
        .into_iter()
        .map(|((prefix, namespace), file)| {
            let canonical = registry[prefix.as_str()];
            ProjectionDiagnostic {
                severity: "ERROR".to_owned(),
                check: "prefix-consistency".to_owned(),
                code: "prefix-consistency".to_owned(),
                message: format!(
                    "prefix `{prefix}:` is bound to <{namespace}> in {file}, shadowing the \
                     canonical registry namespace <{canonical}>. Rename the local prefix \
                     (registry prefixes drive CURIE shortening; a shadow means authored and \
                     emitted CURIEs disagree on `{prefix}:`)."
                ),
                instance: Some(namespace),
                subject_id: None,
                predicate_id: None,
                object_id: None,
            }
        })
        .collect();
    diagnostics.sort_by(ProjectionDiagnostic::cmp_severity_check_instance);
    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .unwrap()
    }

    fn committed_vocab() -> SliceVocab {
        SliceVocab::for_namespace("https://blackcatinformatics.ca/purrdf/")
    }

    #[test]
    fn authored_corpus_has_no_registry_prefix_shadows() {
        let diagnostics =
            lint_prefix_consistency(&repo_root(), &committed_vocab()).expect("scan corpus");
        assert!(
            diagnostics.is_empty(),
            "registry-prefix shadows found:\n{}",
            diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn non_registry_prefix_is_not_a_shadow() {
        // `ex:` is not a registry prefix — per-example local namespaces are exempt,
        // so the corpus's varied `ex:` bindings must NOT be flagged.
        let effective = effective_registry(&committed_vocab());
        assert!(
            !effective.iter().any(|(p, _)| p == "ex"),
            "test assumes `ex` is not a registry prefix"
        );
    }

    #[test]
    fn config_prefixes_match_the_rust_registry() {
        // Dual-authority parity (#933): the Python `config.PREFIXES` mirror must
        // agree with the Rust authority exactly — same prefix→namespace pairs, same
        // insertion order. Parsing failure or any divergence is a hard test failure
        // (the registry comment pins them as mirrors; this is the missing guard).
        let config = repo_root()
            .join("src")
            .join("purrdf_tools")
            .join("config.py");
        if !config.exists() {
            eprintln!(
                "skipping config.PREFIXES comparison; {} is absent",
                config.display()
            );
            return;
        }
        let text = std::fs::read_to_string(&config).expect("read config.py");
        let python = parse_config_prefixes(&text);
        let rust: Vec<(String, String)> = effective_registry(&committed_vocab());
        assert_eq!(
            python, rust,
            "config.PREFIXES drifted from PREFIX_REGISTRY (pairs and/or order)"
        );
    }

    /// Parse `config.py`'s `PREFIXES: dict[str, str] = { ... }` literal into ordered
    /// (prefix, namespace) pairs. Resolves the two computed values the dict opens
    /// with (`NAMESPACE` → purrdf, `LOGIC_NAMESPACE` → logic), which are defined
    /// elsewhere in the module; every other entry is a string literal.
    fn parse_config_prefixes(text: &str) -> Vec<(String, String)> {
        // `NAMESPACE = ONTOLOGY_IRI + "/"` (computed); `LOGIC_NAMESPACE` is a literal.
        let ontology_iri =
            parse_str_const(text, "ONTOLOGY_IRI").expect("ONTOLOGY_IRI const in config.py");
        let namespace = format!("{ontology_iri}/");
        let logic_ns =
            parse_str_const(text, "LOGIC_NAMESPACE").expect("LOGIC_NAMESPACE const in config.py");

        let start = text
            .find("PREFIXES: dict[str, str] = {")
            .expect("PREFIXES dict literal in config.py");
        let body = &text[start..];
        let end = body.find("\n}").expect("PREFIXES dict close");
        let body = &body[..end];

        // Each entry line: `    "prefix": <value>,` where value is "literal",
        // NAMESPACE, or LOGIC_NAMESPACE.
        let entry = Regex::new(r#"(?m)^\s*"([A-Za-z][A-Za-z0-9_.-]*)"\s*:\s*([^,\n]+),"#).unwrap();
        let mut out: Vec<(String, String)> = Vec::new();
        for caps in entry.captures_iter(body) {
            let prefix = caps[1].to_owned();
            let raw = caps[2].trim();
            let value = if raw == "NAMESPACE" {
                namespace.clone()
            } else if raw == "LOGIC_NAMESPACE" {
                logic_ns.clone()
            } else if let Some(inner) = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                inner.to_owned()
            } else {
                panic!("unresolvable PREFIXES value for {prefix}: {raw}");
            };
            out.push((prefix, value));
        }
        out
    }

    /// Parse a module-level `NAME = "literal"` string constant.
    fn parse_str_const(text: &str, name: &str) -> Option<String> {
        let re = Regex::new(&format!(r#"(?m)^{name}\s*=\s*"([^"]*)""#)).unwrap();
        re.captures(text).map(|c| c[1].to_owned())
    }
}

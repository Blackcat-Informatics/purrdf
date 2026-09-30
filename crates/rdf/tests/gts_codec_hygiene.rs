// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! GTS-codec-hygiene gate — the boundary lock for the native RDF codec seam.
//!
//! The whole RDF codec seam is now native. `crates/rdf/src/native_codecs/` parses and
//! serializes RDF on the first-party IR with NO `purrdf_gts` codec in the middle, and the
//! JSON-LD / YAML-LD surfaces no longer call purrdf-gts codecs. The ONLY legitimate
//! remaining `purrdf_gts::` use is purrdf.gts CONTAINER I/O — the file/structural
//! seam (reader / writer / model / verify / policy / codec::CodecError / ulid / …). This
//! gate is a STRUCTURAL source scan, mirroring the carrier-purity gate's shape (line-comment
//! stripping + an allow-list + negative-arm self-tests so the detector can never silently
//! pass), that LOCKS that boundary in three rules:
//!
//!   RULE 1 — the codec seam is TOTALLY clean: no `purrdf_gts` token anywhere (production
//!            OR test) under `crates/rdf/src/native_codecs/`.
//!
//!   RULE 2 — RDF-codec ENTRYPOINTS are banned in PRODUCTION AND TEST across all
//!            `crates/*/src`: the `purrdf_gts::` codec call surfaces (nquads / trig / yamlld /
//!            rdf_xml / rdf_codecs / rdf:: dataset model). purrdf-gts is the purrdf.gts container
//!            layer ONLY, so even test oracles must re-render through the native `purrdf`
//!            codecs. purrdf.gts CONTAINER symbols (reader, writer, model, verify, policy, wire,
//!            ulid, codec::, openpgp, examples) are EXPLICITLY allowed and must not be
//!            flagged.
//!
//!   RULE 3 — purrdf-gts CODEC feature edges (`rdf-codecs` / `yaml-ld`) are banned in every
//!            `crates/*/Cargo.toml`. A dead-but-enabled codec feature LINKS the purrdf-gts
//!            codec surface even when no source token (RULE 2) calls it, so this manifest
//!            scan fails closed where the source scan is blind.
//!
//! A violation of any rule turns this test red. The `tests` module proves the detector is
//! not vacuous: synthetic sources/manifests carrying a forbidden token must be flagged, and
//! an allowed container token must NOT be.

use std::path::{Path, PathBuf};

use purrdf_testkit::paths::workspace_root;

/// The `purrdf_gts::` RDF-codec ENTRYPOINTS banned in production. These are the codec call
/// surfaces (text RDF serialize/parse + the purrdf-gts RDF-dataset model/adapters). Each is
/// a `purrdf_gts::`-qualified path prefix, so a CONTAINER symbol like `purrdf_gts::reader`
/// can never match one.
const FORBIDDEN_GTS_CODEC_PATHS: [&str; 9] = [
    "purrdf_gts::rdf_codecs::",
    "purrdf_gts::nquads::",
    "purrdf_gts::from_nquads",
    "purrdf_gts::trig::",
    "purrdf_gts::from_trig",
    "purrdf_gts::yamlld",
    "purrdf_gts::from_yamlld",
    "purrdf_gts::rdf_xml",
    "purrdf_gts::rdf::",
];

/// purrdf.gts CONTAINER symbols that are ALLOWED in production — the file/structural seam.
/// These must NEVER be flagged by RULE 2. Listed so a reviewer sees exactly what stays.
const ALLOWED_GTS_CONTAINER_PREFIXES: [&str; 11] = [
    "purrdf_gts::reader",
    "purrdf_gts::writer",
    "purrdf_gts::model",
    "purrdf_gts::verify",
    "purrdf_gts::policy",
    "purrdf_gts::wire",
    "purrdf_gts::ulid",
    "purrdf_gts::codec::",
    "purrdf_gts::openpgp",
    "purrdf_gts::examples",
    "purrdf_gts::Error", // CodecError et al. re-exported at the crate root are container errors
];

/// purrdf-gts CODEC feature names. Enabling any of these in a manifest LINKS the purrdf-gts RDF
/// codec surface (text + RDF/XML + JSON-LD-star), which is banned: purrdf-gts is the purrdf.gts
/// CONTAINER layer only, and all RDF codec work is native (`crates/rdf/src/native_codecs/`).
/// The container features (e.g. `duckdb`) and the value model are fine; only these are
/// forbidden. RULE 3 fails closed if a manifest re-introduces one (the source-token RULE 2 is
/// blind to a dead-but-linked Cargo feature).
const FORBIDDEN_GTS_CODEC_FEATURES: [&str; 2] = ["rdf-codecs", "yaml-ld"];

/// Every `.rs` file under each `crates/*/src` directory, as `(crate-relative label, path)`.
/// The label is workspace-root-relative for legible violation messages.
fn crate_src_rust_files(root: &Path) -> Vec<(String, PathBuf)> {
    let crates_dir = root.join("crates");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&crates_dir).unwrap_or_else(|e| {
        panic!(
            "gts-codec-hygiene: cannot read {}: {e}",
            crates_dir.display()
        )
    }) {
        let entry = entry.expect("dir entry");
        let src = entry.path().join("src");
        if src.is_dir() {
            collect_rust_files(&src, root, &mut out);
        }
    }
    out.sort();
    out
}

/// Every `crates/*/Cargo.toml` manifest, as `(workspace-relative label, path)`.
fn crate_manifests(root: &Path) -> Vec<(String, PathBuf)> {
    let crates_dir = root.join("crates");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&crates_dir).unwrap_or_else(|e| {
        panic!(
            "gts-codec-hygiene: cannot read {}: {e}",
            crates_dir.display()
        )
    }) {
        let entry = entry.expect("dir entry");
        let manifest = entry.path().join("Cargo.toml");
        if manifest.is_file() {
            let label = manifest
                .strip_prefix(root)
                .unwrap_or(&manifest)
                .to_string_lossy()
                .into_owned();
            out.push((label, manifest));
        }
    }
    out.sort();
    out
}

/// Every `.rs` file under a directory tree, recursively.
fn rust_files_under(dir: &Path, root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    collect_rust_files(dir, root, &mut out);
    out.sort();
    out
}

fn collect_rust_files(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("gts-codec-hygiene: cannot read {}: {e}", dir.display()))
    {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, root, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let label = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            out.push((label, path));
        }
    }
}

/// Strip Rust line-comments so a doc-comment NAMING a forbidden token is not a false
/// positive (the carrier-purity gate's idiom; block comments are not used for these
/// mentions, so line-stripping suffices).
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------
// RULE 1 — native_codecs is totally clean (no purrdf_gts, prod OR test).
// ---------------------------------------------------------------------------------------

/// Scan the WHOLE source (production AND test) for any `purrdf_gts` token. Returns `(token, line snippet)` violations.
fn scan_native_codecs_violations(label: &str, source: &str) -> Vec<(String, String)> {
    let code = strip_line_comments(source);
    let mut violations = Vec::new();
    for (lineno, line) in code.lines().enumerate() {
        if line.contains("purrdf_gts") {
            violations.push((
                "purrdf_gts".to_string(),
                format!("{label}:{} | {}", lineno + 1, line.trim()),
            ));
        }
    }
    violations
}

// ---------------------------------------------------------------------------------------
// RULE 2 — purrdf_gts RDF-codec entrypoints banned in PRODUCTION (container symbols allowed).
// ---------------------------------------------------------------------------------------

/// Scan a module's WHOLE source (production AND test) for a banned `purrdf_gts::` codec
/// entrypoint. purrdf-gts is the purrdf.gts container layer ONLY — using it for general RDF
/// codec work is banned everywhere, not merely in production, so test oracles must re-render
/// through the native `purrdf` codecs too. Container symbols never match (the forbidden
/// list is the codec paths only); the allow-list is asserted not-flagged by the negative-arm
/// self-test.
fn scan_gts_codec_entrypoints(label: &str, source: &str) -> Vec<(String, String)> {
    let code = strip_line_comments(source);
    let mut violations = Vec::new();
    for (lineno, line) in code.lines().enumerate() {
        for token in FORBIDDEN_GTS_CODEC_PATHS {
            if line.contains(token) {
                violations.push((
                    token.to_string(),
                    format!("{label}:{} | {}", lineno + 1, line.trim()),
                ));
            }
        }
    }
    violations
}

// ---------------------------------------------------------------------------------------
// RULE 3 — purrdf-gts CODEC feature edges banned in every crates/*/Cargo.toml manifest.
// ---------------------------------------------------------------------------------------

/// Strip TOML line comments (`# …`) so a comment NAMING a forbidden feature is not a false
/// positive. Feature/dependency lines never legitimately carry a `#`, so naive splitting is
/// safe here (the same simplification the source-line stripper relies on).
fn strip_toml_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find('#') {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scan a `Cargo.toml` for a forbidden purrdf-gts codec feature edge — either form leaves the
/// purrdf-gts codec surface linked even when no source token (RULE 2) calls it:
///   * the cross-crate `purrdf-gts/<codec-feature>` enable in any feature array, or
///   * a `purrdf-gts = { … features = […, "<codec-feature>", …] }` dependency.
fn scan_gts_codec_feature_edges(label: &str, source: &str) -> Vec<(String, String)> {
    let code = strip_toml_comments(source);
    let mut violations = Vec::new();
    for (lineno, line) in code.lines().enumerate() {
        let is_gts_dep_with_features =
            line.trim_start().starts_with("purrdf-gts") && line.contains("features");
        for feat in FORBIDDEN_GTS_CODEC_FEATURES {
            let cross_crate = line.contains(&format!("purrdf-gts/{feat}"));
            let dep_feature = is_gts_dep_with_features && line.contains(&format!("\"{feat}\""));
            if cross_crate || dep_feature {
                violations.push((
                    feat.to_string(),
                    format!("{label}:{} | {}", lineno + 1, line.trim()),
                ));
            }
        }
    }
    violations
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("gts-codec-hygiene: cannot read {}: {e}", path.display()))
}

#[test]
fn native_codecs_seam_is_totally_clean() {
    let root = workspace_root();
    let native_codecs = root
        .join("crates")
        .join("rdf")
        .join("src")
        .join("native_codecs");
    assert!(
        native_codecs.is_dir(),
        "gts-codec-hygiene: expected the native codec seam at {}",
        native_codecs.display()
    );
    let mut all: Vec<(String, String)> = Vec::new();
    for (label, path) in rust_files_under(&native_codecs, &root) {
        all.extend(scan_native_codecs_violations(&label, &read(&path)));
    }
    assert!(
        all.is_empty(),
        "RULE 1 FAILED: the native RDF codec seam (crates/rdf/src/native_codecs/) must be \
         100% free of purrdf_gts tokens (production OR test). The codec operates on the \
         first-party IR with NO purrdf-gts codec in the middle. Violations:\n{}",
        render(&all)
    );
}

#[test]
fn no_gts_rdf_codec_entrypoint_anywhere() {
    let root = workspace_root();
    let mut all: Vec<(String, String)> = Vec::new();
    for (label, path) in crate_src_rust_files(&root) {
        all.extend(scan_gts_codec_entrypoints(&label, &read(&path)));
    }
    assert!(
        all.is_empty(),
        "RULE 2 FAILED: a purrdf_gts RDF-codec entrypoint appears in source (production OR test). \
         The RDF codec seam is native (crates/rdf/src/native_codecs/); purrdf_gts is for purrdf.gts \
         CONTAINER I/O ONLY (reader/writer/model/verify/policy/codec::/ulid/…). Route RDF \
         text/dataset I/O through purrdf instead — including in test oracles. Violations:\n{}",
        render(&all)
    );
}

#[test]
fn no_gts_codec_feature_edge_in_manifests() {
    let root = workspace_root();
    let mut all: Vec<(String, String)> = Vec::new();
    for (label, path) in crate_manifests(&root) {
        all.extend(scan_gts_codec_feature_edges(&label, &read(&path)));
    }
    assert!(
        all.is_empty(),
        "RULE 3 FAILED: a crates/*/Cargo.toml enables a purrdf-gts RDF codec feature \
         (rdf-codecs / yaml-ld). That LINKS the purrdf-gts codec surface even though no source \
         calls it (RULE 2 cannot see a dead Cargo feature). purrdf-gts is purrdf.gts CONTAINER \
         I/O ONLY — drop the codec feature; RDF codec work is native (crates/rdf/src/\
         native_codecs/). Violations:\n{}",
        render(&all)
    );
}

fn render(violations: &[(String, String)]) -> String {
    violations
        .iter()
        .map(|(tok, loc)| format!("  - `{tok}` at {loc}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- RULE 1 negative arm -----------------------------------------------------------

    #[test]
    fn rule1_flags_purrdf_gts_anywhere_in_native_codecs() {
        let with_codec = r"
fn serialize(d: &RdfDataset) -> String {
    purrdf_gts::nquads::to_nquads(&gts_graph_from(d))
}
";
        let v = scan_native_codecs_violations("crates/rdf/src/native_codecs/x.rs", with_codec);
        assert!(
            v.iter().any(|(t, _)| t == "purrdf_gts"),
            "RULE 1 detector must flag any purrdf_gts token in native_codecs, got {v:?}"
        );
    }

    #[test]
    fn rule1_clean_native_source_is_not_flagged() {
        let clean = r"
use crate::native_codecs::ser_model::SerGraph;
fn serialize(g: &SerGraph) -> String { native_nquads(g) }
";
        let v = scan_native_codecs_violations("crates/rdf/src/native_codecs/x.rs", clean);
        assert!(
            v.is_empty(),
            "clean native source must not be flagged, got {v:?}"
        );
    }

    // --- RULE 2 negative + allow-list arms ---------------------------------------------

    #[test]
    fn rule2_flags_each_forbidden_codec_entrypoint() {
        for token in FORBIDDEN_GTS_CODEC_PATHS {
            let src = format!("fn f() {{ let _ = {token}foo(); }}\n");
            let v = scan_gts_codec_entrypoints("crates/x/src/y.rs", &src);
            assert!(
                v.iter().any(|(t, _)| t == token),
                "RULE 2 detector must flag the codec entrypoint `{token}`, got {v:?}"
            );
        }
    }

    #[test]
    fn rule2_allows_every_container_symbol() {
        // The allow-listed purrdf.gts CONTAINER symbols are the legitimate file/structural
        // seam and MUST NOT be flagged by RULE 2.
        for prefix in ALLOWED_GTS_CONTAINER_PREFIXES {
            let src = format!("use {prefix};\nfn f() {{ let _ = {prefix}::thing(); }}\n");
            let v = scan_gts_codec_entrypoints("crates/x/src/y.rs", &src);
            assert!(
                v.is_empty(),
                "RULE 2 must NOT flag the allowed container symbol `{prefix}`, got {v:?}"
            );
        }
    }

    #[test]
    fn rule2_flags_codec_call_in_cfg_test_region() {
        // purrdf-gts is the purrdf.gts container layer ONLY: a codec call in a #[cfg(test)]
        // oracle is JUST AS BANNED as in production. Test oracles must re-render through the
        // native purrdf codecs, so RULE 2 scans the test region too.
        let with_test_codec = r"
fn f() { native(); }

#[cfg(test)]
mod tests {
    #[test]
    fn oracle() { let nq = purrdf_gts::nquads::to_nquads(&g); }
}
";
        let v = scan_gts_codec_entrypoints("crates/x/src/y.rs", with_test_codec);
        assert!(
            v.iter().any(|(t, _)| t == "purrdf_gts::nquads::"),
            "RULE 2 must flag a codec entrypoint inside the #[cfg(test)] region, got {v:?}"
        );
    }

    #[test]
    fn rule2_doc_comment_mention_is_not_flagged() {
        let doc = r"
/// The native inverse of the old `purrdf_gts::nquads::to_nquads` + re-parse round-trip.
fn f() { native(); }
";
        let v = scan_gts_codec_entrypoints("crates/x/src/y.rs", doc);
        assert!(
            v.is_empty(),
            "RULE 2 must not flag a doc-comment mention of the retired codec path, got {v:?}"
        );
    }

    // --- RULE 3 manifest-feature arms --------------------------------------------------

    #[test]
    fn rule3_flags_the_cross_crate_codec_feature_edge() {
        for feat in FORBIDDEN_GTS_CODEC_FEATURES {
            let manifest =
                format!("[features]\ngts = [\"dep:purrdf-gts\", \"purrdf-gts/{feat}\"]\n");
            let v = scan_gts_codec_feature_edges("crates/x/Cargo.toml", &manifest);
            assert!(
                v.iter().any(|(t, _)| t == feat),
                "RULE 3 must flag the cross-crate `purrdf-gts/{feat}` feature edge, got {v:?}"
            );
        }
    }

    #[test]
    fn rule3_flags_the_dependency_feature_edge() {
        for feat in FORBIDDEN_GTS_CODEC_FEATURES {
            let manifest = format!(
                "[dependencies]\npurrdf-gts = {{ version = \"0.9.11\", features = [\"{feat}\"] }}\n"
            );
            let v = scan_gts_codec_feature_edges("crates/x/Cargo.toml", &manifest);
            assert!(
                v.iter().any(|(t, _)| t == feat),
                "RULE 3 must flag the `purrdf-gts {{ features = [\"{feat}\"] }}` dependency edge, got {v:?}"
            );
        }
    }

    #[test]
    fn rule3_allows_container_only_gts_dependencies() {
        // The plain container dependency and the `duckdb` container feature are fine — only the
        // RDF codec features are forbidden. A comment naming a codec feature is not an edge.
        let manifest = r#"
[dependencies]
purrdf-gts = { version = "0.9.11", features = ["duckdb"] }
# historically this enabled purrdf-gts/rdf-codecs; the codec is native now.

[features]
gts = ["dep:purrdf-gts", "dep:regex"]
"#;
        let v = scan_gts_codec_feature_edges("crates/x/Cargo.toml", manifest);
        assert!(
            v.is_empty(),
            "RULE 3 must NOT flag a container-only purrdf-gts dependency nor a comment, got {v:?}"
        );
    }
}

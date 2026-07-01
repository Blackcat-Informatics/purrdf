// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shared SHACL shape-file set (#700).
//!
//! Replicates EXACTLY the shape union the Python validator builds in
//! `src/purrdf_tools/validate.py::_shapes_turtle` so the JSON-Schema emitter sees
//! the SAME shapes as the live validator — no drift. The file order is:
//!
//! 1. every `shapes/*.ttl` (sorted) EXCEPT the four DSL/manifest lints
//!    (`mapping-dsl-shapes.ttl`, `statement-dsl-shapes.ttl`, `test-dsl-shapes.ttl`,
//!    `slice-manifest-shapes.ttl`);
//! 2. every `generated/shapes/*.ttl` (sorted) — FAIL CLOSED if none exist
//!    (mirrors `validate.py`: the generated frame constraints replaced the
//!    hand-written ones, so their absence would silently stop enforcing P11);
//! 3. every `slices/*/*/shapes.ttl` (sorted) — exactly two directory levels.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ::purrdf::parse_dataset;
use ::purrdf::{RdfDataset, RdfDatasetBuilder};

use crate::shapes::{self, Shapes};

/// Shape files excluded from the data-graph union (DSL / manifest lints).
pub const EXCLUDED: &[&str] = &[
    "mapping-dsl-shapes.ttl",
    "statement-dsl-shapes.ttl",
    "test-dsl-shapes.ttl",
    "slice-manifest-shapes.ttl",
];

/// The ordered list of SHACL shape files that constrain the data graph.
///
/// # Errors
///
/// Returns `Err` when no `generated/shapes/*.ttl` exist (fail-closed, mirroring
/// `validate.py`) or when a directory cannot be read.
pub fn shape_files(repo_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();

    // 1. shapes/*.ttl minus the excluded DSL/manifest lints.
    let shapes_dir = repo_root.join("shapes");
    let mut base = ttl_files(&shapes_dir)?;
    base.retain(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| !EXCLUDED.contains(&n))
            .unwrap_or(false)
    });
    files.extend(base);

    // 2. generated/shapes/*.ttl — fail closed if none.
    let generated_dir = repo_root.join("generated").join("shapes");
    let generated = ttl_files(&generated_dir)?;
    if generated.is_empty() {
        return Err(format!(
            "no generated shapes under {} — run `purrdf regenerate frame-shapes` (P11 enforcement lives there)",
            generated_dir.display()
        ));
    }
    files.extend(generated);

    // 3. slices/*/*/shapes.ttl — exactly two directory levels under slices/.
    files.extend(slice_shape_files(&repo_root.join("slices"))?);

    Ok(files)
}

/// Parse every shape file from [`shape_files`] into ONE frozen [`RdfDataset`], then
/// parse it into a typed [`Shapes`]. Returns both so a caller (e.g. the instance
/// projector) can reuse the dataset.
///
/// The union's document `@prefix` declarations are recovered and threaded into
/// [`shapes::from_dataset_with_prefixes`] (the frozen IR does not retain prefix
/// maps): SHACL-AF `sh:select` queries — e.g. the music `MetricGroupShape`
/// uniqueness constraint — use prefixed names like `purrdf:` and fail to parse
/// without them. This mirrors the live validator (`engine::parse_shapes`, #578).
/// When the same prefix is declared in multiple files, the last declaration wins
/// (deterministic via the merge order over the sorted file list).
///
/// Blank labels are scoped per source file: a NodeShape's anonymous property shapes
/// (`sh:property [ … ]`) restart the blank counter per file, so merging several
/// shape files unscoped would fuse distinct property shapes (#909).
/// [`RdfDataset::union`] standardizes each input's blanks apart under a fresh scope,
/// providing exactly that per-file isolation.
///
/// # Errors
///
/// Returns `Err` when a file cannot be read, fails to parse as Turtle, or when
/// [`shapes::from_dataset_with_prefixes`] rejects an unsupported SHACL construct.
pub fn load_shapes(repo_root: &Path) -> Result<(Arc<RdfDataset>, Shapes), String> {
    let files = shape_files(repo_root)?;
    let mut prefix_map: BTreeMap<String, String> = BTreeMap::new();
    let mut per_file: Vec<Arc<RdfDataset>> = Vec::with_capacity(files.len());
    for file in &files {
        let bytes = std::fs::read(file)
            .map_err(|e| format!("failed to read shape file {}: {e}", file.display()))?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|e| format!("shape file {} is not UTF-8: {e}", file.display()))?;
        // Parse via the native codecs (#909). The native codec drops document
        // prefixes once it folds to the IR, so the per-file `@prefix` map is
        // recovered by scanning the source text — see the doc comment above.
        let dataset = parse_dataset(&bytes, "text/turtle", None)
            .map_err(|e| format!("failed to parse Turtle shape file {}: {e}", file.display()))?;
        per_file.push(dataset);
        for (prefix, namespace) in crate::text_ingest::extract_prefixes(text) {
            prefix_map.insert(prefix, namespace);
        }
    }
    // Union all per-file datasets into one, standardizing blanks apart per file.
    let merged = if per_file.is_empty() {
        RdfDatasetBuilder::new()
            .freeze()
            .map_err(|e| format!("failed to build empty shapes dataset: {e}"))?
    } else {
        let refs: Vec<&RdfDataset> = per_file.iter().map(AsRef::as_ref).collect();
        Arc::new(RdfDataset::union(&refs))
    };
    let doc_prefixes: Vec<(String, String)> = prefix_map.into_iter().collect();
    let shapes = shapes::from_dataset_with_prefixes(&merged, &doc_prefixes)?;
    Ok((merged, shapes))
}

/// Every `*.ttl` directly under `dir`, sorted. An absent directory yields an
/// empty list (the caller decides whether that is fail-closed).
fn ttl_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<PathBuf> = Vec::new();
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read dir entry in {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ttl") && path.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Every `slices/*/*/shapes.ttl` (exactly two directory levels), sorted.
fn slice_shape_files(slices_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !slices_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<PathBuf> = Vec::new();
    let groups = std::fs::read_dir(slices_dir)
        .map_err(|e| format!("failed to read {}: {e}", slices_dir.display()))?;
    for group in groups {
        let group = group.map_err(|e| format!("dir entry error: {e}"))?;
        let group_path = group.path();
        if !group_path.is_dir() {
            continue;
        }
        // A read error here would silently drop an entire slice subtree from the
        // shape union — under-validating instances and shrinking the compiled JSON
        // Schema. Hard-fail instead (no-optionality), matching every sibling
        // read_dir in this file.
        let slices = std::fs::read_dir(&group_path)
            .map_err(|e| format!("failed to read {}: {e}", group_path.display()))?;
        for slice in slices {
            let slice = slice.map_err(|e| format!("dir entry error: {e}"))?;
            let slice_path = slice.path();
            if !slice_path.is_dir() {
                continue;
            }
            let candidate = slice_path.join("shapes.ttl");
            if candidate.is_file() {
                out.push(candidate);
            }
        }
    }
    out.sort();
    Ok(out)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a tiny mock repo tree under a temp dir and assert ordering +
    /// exclusion + fail-closed behavior.
    fn mock_repo() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "purrdf-shape-union-{}-{}",
            std::process::id(),
            // a per-call salt so parallel tests do not collide
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("shapes")).unwrap();
        fs::create_dir_all(base.join("generated/shapes")).unwrap();
        fs::create_dir_all(base.join("slices/core/alpha")).unwrap();
        fs::create_dir_all(base.join("slices/core/beta")).unwrap();
        base
    }

    fn touch(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_shape_files_order_and_exclusions() {
        let repo = mock_repo();
        touch(&repo.join("shapes/purrdf-shapes.ttl"), "# base\n");
        touch(&repo.join("shapes/zzz-shapes.ttl"), "# extra\n");
        // excluded files MUST NOT appear:
        touch(&repo.join("shapes/mapping-dsl-shapes.ttl"), "# excluded\n");
        touch(
            &repo.join("shapes/statement-dsl-shapes.ttl"),
            "# excluded\n",
        );
        touch(&repo.join("shapes/test-dsl-shapes.ttl"), "# excluded\n");
        touch(
            &repo.join("shapes/slice-manifest-shapes.ttl"),
            "# excluded\n",
        );
        touch(&repo.join("generated/shapes/frame-shapes.ttl"), "# gen\n");
        touch(&repo.join("slices/core/alpha/shapes.ttl"), "# alpha\n");
        touch(&repo.join("slices/core/beta/shapes.ttl"), "# beta\n");

        let files = shape_files(&repo).expect("shape_files must succeed");
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();

        // No excluded files.
        for ex in EXCLUDED {
            assert!(!names.contains(&(*ex).to_owned()), "{ex} must be excluded");
        }
        // base shapes first (sorted), then generated, then slices.
        let base_idx = names.iter().position(|n| n == "purrdf-shapes.ttl").unwrap();
        let gen_idx = files
            .iter()
            .position(|p| p.ends_with("generated/shapes/frame-shapes.ttl"))
            .unwrap();
        let slice_idx = files
            .iter()
            .position(|p| p.ends_with("slices/core/alpha/shapes.ttl"))
            .unwrap();
        assert!(base_idx < gen_idx, "base shapes precede generated");
        assert!(gen_idx < slice_idx, "generated precede slice shapes");

        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_fail_closed_without_generated_shapes() {
        let repo = mock_repo();
        touch(&repo.join("shapes/purrdf-shapes.ttl"), "# base\n");
        // No generated/shapes/*.ttl present.
        let result = shape_files(&repo);
        assert!(result.is_err(), "absent generated shapes must fail closed");
        assert!(result.unwrap_err().contains("generated shapes"));
        let _ = fs::remove_dir_all(&repo);
    }

    #[test]
    fn test_load_shapes_parses_union() {
        let repo = mock_repo();
        let prefixes = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .\n";
        touch(
            &repo.join("shapes/purrdf-shapes.ttl"),
            &format!(
                "{prefixes}purrdf:PersonShape a sh:NodeShape ; sh:targetClass purrdf:Person .\n"
            ),
        );
        touch(
            &repo.join("generated/shapes/frame-shapes.ttl"),
            &format!(
                "{prefixes}purrdf:FrameShape a sh:NodeShape ; sh:targetClass purrdf:Frame .\n"
            ),
        );
        touch(
            &repo.join("slices/core/alpha/shapes.ttl"),
            &format!(
                "{prefixes}purrdf:AlphaShape a sh:NodeShape ; sh:targetClass purrdf:Alpha .\n"
            ),
        );
        let (_store, shapes) = load_shapes(&repo).expect("load_shapes must succeed");
        // 3 node shapes loaded from the union.
        assert_eq!(shapes.node_shapes.len(), 3, "all three shapes loaded");
        let _ = fs::remove_dir_all(&repo);
    }
}

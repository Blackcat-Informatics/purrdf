// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native `purrdf:sliceDependsOn` reconciliation patcher (#820 G8).
//!
//! Replaces the former Python `slice_fix_deps` line-regex Turtle surgery with an
//! **RDF-aware, surgical** manifest patcher driven by the native ownership
//! analyzer:
//!
//! * The depending manifest is located via the catalog's retained on-disk slice
//!   directory ([`crate::catalog::SliceRecord::manifest_path`]) — never a
//!   `rglob` scan or substring match (the HIGH-8 wrong-manifest bug).
//! * Additions (undeclared semantic edges) and removals (stale declarations) are
//!   computed from the [`crate::ownership::OwnershipReport`], deduped, and scoped
//!   to the manifest's own slice subject.
//! * The patch is applied as a **targeted textual edit validated against the
//!   parsed graph**, not a blind regex and not a whole-file re-serialization. The
//!   manifest is parsed to confirm the slice subject and its existing
//!   `purrdf:sliceDependsOn` objects (RDF-aware), the edit reuses the author's
//!   formatting for every unchanged line, and the patched text is **re-parsed**
//!   to prove it is well-formed Turtle that declares the corrected dependency
//!   set before it is returned (HIGH-7: never emit malformed Turtle).
//!
//! Why surgical-validated rather than full re-serialization: re-serializing the
//! whole manifest via oxigraph would reorder/reformat the entire file (losing the
//! author's comments and ordering), producing an enormous diff and risking the
//! producer/CITATION projections and `make validate`. The surgical edit keeps the
//! diff minimal while the re-parse gives full RDF correctness.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::SliceCatalog;
use crate::error::SliceError;
use crate::ownership::{OwnershipAnalyzer, ReconciliationStatus, SliceIri};
use crate::rdf_query::Dataset;

const PURRDF_NS: &str = "https://blackcatinformatics.ca/purrdf/";
const PURRDF_SLICE_DEPENDS_ON: &str = "https://blackcatinformatics.ca/purrdf/sliceDependsOn";

/// A computed manifest patch: the original and patched Turtle text plus the
/// on-disk path. `original == patched` is never returned (callers receive only
/// non-empty edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPatch {
    /// The on-disk path to the patched `manifest.ttl`.
    pub manifest_path: String,
    /// The original manifest text.
    pub original_text: String,
    /// The patched manifest text (well-formed Turtle, re-parse validated).
    pub patched_text: String,
}

/// Per-manifest add/remove sets, deduped.
struct DepProposal {
    slice_iri: SliceIri,
    manifest_path: String,
    to_add: BTreeSet<SliceIri>,
    to_remove: BTreeSet<SliceIri>,
}

/// Compute the RDF-aware `purrdf:sliceDependsOn` reconciliation patches for every
/// manifest with undeclared (add) or stale (remove) semantic edges.
///
/// # Errors
///
/// Returns a [`SliceError`] on any manifest read/parse failure, or if a patched
/// manifest fails its post-edit re-parse validation — no silent skips.
pub fn compute_fix_deps(catalog: &SliceCatalog) -> Result<Vec<ManifestPatch>, SliceError> {
    let report = OwnershipAnalyzer::new(catalog).analyze()?;

    // Group undeclared/stale semantic edges by depending slice, deduped.
    let mut proposals: BTreeMap<SliceIri, DepProposal> = BTreeMap::new();
    for edge in &report.edges {
        match edge.reconciliation {
            ReconciliationStatus::Undeclared | ReconciliationStatus::Stale => {}
            _ => continue,
        }
        if !edge.edge_kind.is_semantic() {
            continue;
        }
        let Some(record) = catalog.get(&edge.from_slice) else {
            // Every edge endpoint is a discovered slice; absence is unexpected.
            return Err(SliceError::InvalidManifest(format!(
                "fix-deps: depending slice {} not found in catalog",
                edge.from_slice
            )));
        };
        let entry = proposals
            .entry(edge.from_slice.clone())
            .or_insert_with(|| DepProposal {
                slice_iri: edge.from_slice.clone(),
                manifest_path: record.manifest_path().to_string_lossy().to_string(),
                to_add: BTreeSet::new(),
                to_remove: BTreeSet::new(),
            });
        match edge.reconciliation {
            ReconciliationStatus::Undeclared => {
                entry.to_add.insert(edge.to_slice.clone());
            }
            ReconciliationStatus::Stale => {
                entry.to_remove.insert(edge.to_slice.clone());
            }
            _ => unreachable!(),
        }
    }

    let mut patches = Vec::new();
    for (_from, proposal) in proposals {
        if proposal.to_add.is_empty() && proposal.to_remove.is_empty() {
            continue;
        }
        let original = std::fs::read_to_string(&proposal.manifest_path).map_err(SliceError::Io)?;
        let patched = apply_proposal(&original, &proposal)?;
        if patched != original {
            patches.push(ManifestPatch {
                manifest_path: proposal.manifest_path,
                original_text: original,
                patched_text: patched,
            });
        }
    }
    Ok(patches)
}

/// Apply one proposal's add/remove sets to a manifest's Turtle text via a
/// targeted, RDF-validated textual edit.
fn apply_proposal(original: &str, proposal: &DepProposal) -> Result<String, SliceError> {
    // ── RDF-aware confirmation: parse the manifest, confirm the slice subject,
    // and read its existing sliceDependsOn object set. ──────────────────────────
    let store = parse_turtle(original.as_bytes(), &proposal.manifest_path)?;
    let subject = proposal.slice_iri.as_str();
    let existing: BTreeSet<String> = store
        .object_iris(subject, PURRDF_SLICE_DEPENDS_ON)?
        .into_iter()
        .collect();

    // Only remove targets that are actually declared; only add ones not already
    // present (idempotent, no duplicate patch lines).
    let to_remove: BTreeSet<&String> = proposal
        .to_remove
        .iter()
        .filter(|t| existing.contains(*t))
        .collect();
    let to_add: BTreeSet<&String> = proposal
        .to_add
        .iter()
        .filter(|t| !existing.contains(*t))
        .collect();
    if to_remove.is_empty() && to_add.is_empty() {
        return Ok(original.to_string());
    }

    // The authoritative target object set = existing − removed + added.
    let mut desired = existing.clone();
    for t in &to_remove {
        desired.remove(*t);
    }
    for t in &to_add {
        desired.insert((*t).clone());
    }

    let purrdf_ns = extract_purrdf_prefix(original);
    let patched = surgical_edit(original, &purrdf_ns, &desired, !existing.is_empty())?;

    // ── Post-edit validation: re-parse and confirm the corrected dependency set
    // is present on the slice subject (well-formed Turtle, no terminator slips). ─
    let patched_store = parse_turtle(patched.as_bytes(), &proposal.manifest_path)?;
    let result: BTreeSet<String> = patched_store
        .object_iris(subject, PURRDF_SLICE_DEPENDS_ON)?
        .into_iter()
        .collect();
    // Expected = existing − removed + added.
    let mut expected = existing.clone();
    for t in &to_remove {
        expected.remove(*t);
    }
    for t in &to_add {
        expected.insert((*t).clone());
    }
    if result != expected {
        return Err(SliceError::InvalidManifest(format!(
            "fix-deps: patched {} did not yield the expected sliceDependsOn set \
             (got {:?}, expected {:?})",
            proposal.manifest_path, result, expected
        )));
    }
    // Confirm the slice subject still typed as purrdf:Slice (no structural damage).
    let still_a_slice = patched_store
        .subjects_of_type("https://blackcatinformatics.ca/purrdf/Slice")?
        .iter()
        .any(|s| s == subject);
    if !still_a_slice {
        return Err(SliceError::InvalidManifest(format!(
            "fix-deps: patched {} no longer declares its slice subject {}",
            proposal.manifest_path, proposal.slice_iri
        )));
    }

    Ok(patched)
}

/// Parse Turtle into a native dataset (lenient for `@x-purrdf-*` lang tags),
/// hard-failing on any syntax error.
fn parse_turtle(bytes: &[u8], path: &str) -> Result<Dataset, SliceError> {
    Dataset::parse_turtle(bytes, path)
}

/// Extract the `purrdf:` prefix IRI declared in the Turtle (defaults to the
/// canonical namespace if no explicit `@prefix purrdf:` is present).
fn extract_purrdf_prefix(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("@prefix") {
            let rest = rest.trim_start();
            if let Some(after) = rest.strip_prefix("purrdf:") {
                if let Some(open) = after.find('<') {
                    if let Some(close) = after[open + 1..].find('>') {
                        return after[open + 1..open + 1 + close].to_string();
                    }
                }
            }
        }
    }
    PURRDF_NS.to_string()
}

/// Render a target slice IRI as a Turtle object. Slice IRIs use the full `<IRI>`
/// form (matching every authored manifest, whose `purrdf:slices/<name>` locals are
/// awkward as prefixed names because of the embedded `/`).
fn render_object(iri: &str, _purrdf_ns: &str) -> String {
    format!("<{iri}>")
}

/// Apply the targeted textual edit by **rewriting the whole `purrdf:sliceDependsOn`
/// predicate block** (predicate token through its `;`/`.` terminator) with the
/// desired object set rendered as a deterministically-ordered comma list. This is
/// robust to the authored forms — single object per predicate line *and*
/// multi-line comma-separated object lists (the real manifests use the latter) —
/// because it operates on the parsed predicate span, not per-line regex matching.
/// Only this one predicate block changes; every other line keeps the author's
/// formatting. The caller re-parses the result to confirm correctness.
///
/// `desired` is the authoritative target set (existing − removed + added),
/// already computed from the parsed graph by [`apply_proposal`].
fn surgical_edit(
    original: &str,
    purrdf_ns: &str,
    desired: &BTreeSet<String>,
    had_existing_block: bool,
) -> Result<String, SliceError> {
    match find_depends_on_block(original) {
        Some(block) => rewrite_block(original, purrdf_ns, desired, &block),
        None => {
            if !had_existing_block && !desired.is_empty() {
                insert_new_block(original, purrdf_ns, desired)
            } else {
                // No textual block found but the graph said there were objects:
                // the manifest uses a form we don't recognize. Hard-fail rather
                // than silently mangle it.
                Err(SliceError::InvalidManifest(
                    "fix-deps: could not locate the purrdf:sliceDependsOn predicate \
                     block for a surgical edit"
                        .to_string(),
                ))
            }
        }
    }
}

/// The byte span of a `purrdf:sliceDependsOn` predicate block: from the start of
/// the predicate token to (and including) its terminator (`;` or `.`).
struct DependsBlock {
    /// Byte offset of the predicate token start.
    start: usize,
    /// Byte offset just past the terminator char.
    end: usize,
    /// The terminator that ended the block.
    terminator: char,
    /// The leading indentation of the predicate line.
    indent: String,
    /// The newline style in use (`"\n"` / `"\r\n"`).
    newline: &'static str,
}

/// Locate the `purrdf:sliceDependsOn` predicate block in the Turtle text. Scans
/// for the predicate token at a token boundary, then advances past its object
/// list (objects separated by `,`) until the predicate-list separator `;` or the
/// statement terminator `.` — tracking `<...>` IRIs and `"..."` strings so a `;`
/// inside one is never mistaken for the terminator.
fn find_depends_on_block(text: &str) -> Option<DependsBlock> {
    let needle = "purrdf:sliceDependsOn";
    let bytes = text.as_bytes();
    let mut search_from = 0;
    let pred_start = loop {
        let rel = text[search_from..].find(needle)?;
        let idx = search_from + rel;
        // Require a token boundary before and after (not part of a longer name).
        let before_ok = idx == 0 || is_token_boundary_before(bytes[idx - 1]);
        let after_idx = idx + needle.len();
        let after_ok = after_idx >= bytes.len() || is_token_boundary_after(bytes[after_idx]);
        if before_ok && after_ok {
            break idx;
        }
        search_from = idx + needle.len();
    };

    // Scan forward from the end of the predicate token to the terminator.
    let mut i = pred_start + needle.len();
    let mut in_iri = false;
    let mut in_str = false;
    let mut terminator = None;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '<' if !in_str => in_iri = true,
            '>' if !in_str => in_iri = false,
            '"' if !in_iri => in_str = !in_str,
            ';' | '.' if !in_iri && !in_str => {
                terminator = Some((i, c));
                break;
            }
            _ => {}
        }
        i += 1;
    }
    let (term_idx, terminator) = terminator?;

    // Leading indentation of the predicate line.
    let line_start = text[..pred_start].rfind('\n').map_or(0, |n| n + 1);
    let indent: String = text[line_start..pred_start]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };

    Some(DependsBlock {
        start: pred_start,
        end: term_idx + 1,
        terminator,
        indent,
        newline,
    })
}

fn is_token_boundary_before(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b';' | b',' | b'.')
}

fn is_token_boundary_after(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Rewrite an existing predicate block with the desired object set. When the set
/// becomes empty the whole block is removed and its terminator is preserved by
/// promoting the block's `.`/`;` onto the surrounding statement.
fn rewrite_block(
    original: &str,
    purrdf_ns: &str,
    desired: &BTreeSet<String>,
    block: &DependsBlock,
) -> Result<String, SliceError> {
    let mut out = String::with_capacity(original.len());
    out.push_str(&original[..block.start]);

    if desired.is_empty() {
        // Remove the block entirely. If it ended the statement (`.`), the prior
        // predicate's `;` must become the statement terminator `.`; if it was a
        // mid-statement `;`, simply dropping it (and the trailing object list) is
        // fine. We re-terminate by rewriting the preceding `;` to the block's
        // terminator when the block ended with `.`.
        // The text before block.start ends with the previous predicate's `;`
        // (plus whitespace/newline). Trim trailing whitespace, swap a trailing
        // `;` to the block terminator if the block was the statement's `.`.
        if block.terminator == '.' {
            let trimmed = out.trim_end_matches([' ', '\t', '\n', '\r']);
            if let Some(stripped) = trimmed.strip_suffix(';') {
                let removed = &out[stripped.len()..]; // the `;` + trailing ws
                let trailing_ws = &removed[1..]; // whitespace after the `;`
                out = format!("{stripped}.{trailing_ws}");
            } else {
                return Err(SliceError::InvalidManifest(
                    "fix-deps: removing the terminal sliceDependsOn block left no \
                     predicate to carry the statement terminator"
                        .to_string(),
                ));
            }
            out.push_str(&original[block.end..]);
        } else {
            // Mid-statement `;`: drop the block and its separator, keep the rest.
            // Skip a single following newline to avoid a blank line.
            let mut rest = &original[block.end..];
            if let Some(s) = rest.strip_prefix("\r\n") {
                rest = s;
            } else if let Some(s) = rest.strip_prefix('\n') {
                rest = s;
            }
            // Also trim the now-trailing whitespace before the block.
            let trimmed_len = out.trim_end_matches([' ', '\t']).len();
            out.truncate(trimmed_len);
            out.push_str(rest);
        }
        return Ok(out);
    }

    // Render the predicate with a deterministic (sorted) comma list. A single
    // object stays on the predicate line; multiple objects use one-per-line with
    // an extra indent level, matching the authored multi-object style.
    let objects: Vec<String> = desired
        .iter()
        .map(|o| render_object(o, purrdf_ns))
        .collect();
    let rendered = if objects.len() == 1 {
        format!("purrdf:sliceDependsOn {}{}", objects[0], block.terminator)
    } else {
        let inner_indent = format!("{}    ", block.indent);
        let body = objects
            .iter()
            .enumerate()
            .map(|(idx, obj)| {
                let sep = if idx + 1 < objects.len() { "," } else { "" };
                format!("{inner_indent}{obj}{sep}")
            })
            .collect::<Vec<_>>()
            .join(block.newline);
        format!(
            "purrdf:sliceDependsOn{nl}{body}{nl}{indent}{term}",
            nl = block.newline,
            indent = block.indent,
            term = block.terminator,
        )
    };
    out.push_str(&rendered);
    out.push_str(&original[block.end..]);
    Ok(out)
}

/// Insert a brand-new `purrdf:sliceDependsOn` predicate block after the
/// `a purrdf:Slice` declaration when the manifest declared no dependencies yet.
fn insert_new_block(
    original: &str,
    purrdf_ns: &str,
    desired: &BTreeSet<String>,
) -> Result<String, SliceError> {
    // Find the `a purrdf:Slice` line to anchor after, and its indentation.
    let anchor_rel = original
        .find("a purrdf:Slice")
        .or_else(|| original.find("a\tpurrdf:Slice"))
        .ok_or_else(|| {
            SliceError::InvalidManifest(
                "fix-deps: manifest has no `a purrdf:Slice` line to anchor a new \
                 sliceDependsOn predicate"
                    .to_string(),
            )
        })?;
    let line_end = original[anchor_rel..]
        .find('\n')
        .map_or(original.len(), |n| anchor_rel + n + 1);
    let line_start = original[..anchor_rel].rfind('\n').map_or(0, |n| n + 1);
    let indent: String = original[line_start..anchor_rel]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let newline = if original.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let objects: Vec<String> = desired
        .iter()
        .map(|o| render_object(o, purrdf_ns))
        .collect();
    let block = if objects.len() == 1 {
        format!("{indent}purrdf:sliceDependsOn {} ;{newline}", objects[0])
    } else {
        let inner_indent = format!("{indent}    ");
        let body = objects
            .iter()
            .enumerate()
            .map(|(idx, obj)| {
                let sep = if idx + 1 < objects.len() { "," } else { "" };
                format!("{inner_indent}{obj}{sep}")
            })
            .collect::<Vec<_>>()
            .join(newline);
        format!("{indent}purrdf:sliceDependsOn{newline}{body} ;{newline}")
    };

    let mut out = String::with_capacity(original.len() + block.len());
    out.push_str(&original[..line_end]);
    out.push_str(&block);
    out.push_str(&original[line_end..]);
    Ok(out)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = "https://blackcatinformatics.ca/purrdf/";

    fn proposal(slice_local: &str, add: &[&str], remove: &[&str]) -> DepProposal {
        DepProposal {
            slice_iri: format!("{NS}slices/{slice_local}"),
            manifest_path: format!("slices/core/{slice_local}/manifest.ttl"),
            to_add: add.iter().map(|s| format!("{NS}slices/{s}")).collect(),
            to_remove: remove.iter().map(|s| format!("{NS}slices/{s}")).collect(),
        }
    }

    /// Removal of a STALE dependency whose line carries the terminal `.` must
    /// leave well-formed Turtle (the CodeRabbit terminal-`.` case).
    #[test]
    fn remove_terminal_dot_dependency_stays_well_formed() {
        let manifest = "\
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<https://blackcatinformatics.ca/purrdf/slices/sliceB> a purrdf:Slice ;
    purrdf:sliceTier purrdf:tierCore ;
    purrdf:sliceDependsOn <https://blackcatinformatics.ca/purrdf/slices/sliceA> .
";
        let p = proposal("sliceB", &[], &["sliceA"]);
        let patched = apply_proposal(manifest, &p).expect("patch must succeed");
        // Re-parse: well-formed Turtle, and the sliceDependsOn is gone.
        let store = parse_turtle(patched.as_bytes(), "test").expect("must re-parse");
        let mut count = 0usize;
        store.for_each_quad(|_, p, _, _| {
            if p == PURRDF_SLICE_DEPENDS_ON {
                count += 1;
            }
        });
        assert_eq!(count, 0, "stale dependency must be removed");
        // The slice subject must remain typed and the previous predicate must now
        // carry the terminal `.`.
        assert!(patched.contains("purrdf:sliceTier purrdf:tierCore ."));
    }

    /// An UNDECLARED edge add must produce well-formed Turtle parseable by
    /// oxigraph, declaring the new dependency.
    #[test]
    fn add_undeclared_dependency_is_well_formed() {
        let manifest = "\
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<https://blackcatinformatics.ca/purrdf/slices/sliceB> a purrdf:Slice ;
    purrdf:sliceTier purrdf:tierCore ;
    rdfs:label \"sliceB\"@x-purrdf-english .
";
        let p = proposal("sliceB", &["sliceA"], &[]);
        let patched = apply_proposal(manifest, &p).expect("patch must succeed");
        let store = parse_turtle(patched.as_bytes(), "test").expect("must re-parse");
        let subject = format!("{NS}slices/sliceB");
        let target = format!("{NS}slices/sliceA");
        let found = store
            .object_iris(&subject, PURRDF_SLICE_DEPENDS_ON)
            .unwrap()
            .iter()
            .any(|n| n == &target);
        assert!(found, "added dependency must be present and parseable");
    }

    /// A patched manifest re-parses, still declares its slice subject, and
    /// carries exactly the corrected dependency set with no duplicate lines.
    #[test]
    fn patched_manifest_has_corrected_set_no_duplicates() {
        let manifest = "\
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<https://blackcatinformatics.ca/purrdf/slices/sliceB> a purrdf:Slice ;
    purrdf:sliceDependsOn <https://blackcatinformatics.ca/purrdf/slices/stale1> ;
    purrdf:sliceTier purrdf:tierCore ;
    rdfs:label \"sliceB\"@x-purrdf-english .
";
        // Remove stale1, add sliceA — and request adding sliceA twice via deduped
        // BTreeSet semantics (set dedupes by construction).
        let p = proposal("sliceB", &["sliceA"], &["stale1"]);
        let patched = apply_proposal(manifest, &p).expect("patch must succeed");
        let store = parse_turtle(patched.as_bytes(), "test").expect("must re-parse");
        let subject = format!("{NS}slices/sliceB");
        let deps: BTreeSet<String> = store
            .object_iris(&subject, PURRDF_SLICE_DEPENDS_ON)
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(
            deps,
            BTreeSet::from([format!("{NS}slices/sliceA")]),
            "corrected set must be exactly {{sliceA}}"
        );
        // No duplicate physical lines.
        let add_line_count = patched.matches("slices/sliceA").count();
        assert_eq!(add_line_count, 1, "no duplicate sliceDependsOn lines");
        // Slice subject still typed.
        assert!(store
            .subjects_of_type("https://blackcatinformatics.ca/purrdf/Slice")
            .unwrap()
            .iter()
            .any(|s| s == &subject));
    }

    /// Adding a dependency already declared is a no-op (idempotent, no dupes).
    #[test]
    fn add_existing_dependency_is_noop() {
        let manifest = "\
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .

<https://blackcatinformatics.ca/purrdf/slices/sliceB> a purrdf:Slice ;
    purrdf:sliceDependsOn <https://blackcatinformatics.ca/purrdf/slices/sliceA> .
";
        let p = proposal("sliceB", &["sliceA"], &[]);
        let patched = apply_proposal(manifest, &p).expect("patch must succeed");
        assert_eq!(patched, manifest, "adding an existing dep is a no-op");
    }

    #[test]
    fn render_object_uses_full_iri_form() {
        // Slice IRIs always render as full `<IRI>` (matching authored manifests).
        assert_eq!(
            render_object(&format!("{NS}slices/foo"), NS),
            format!("<{NS}slices/foo>")
        );
        assert_eq!(
            render_object("https://other.example/x", NS),
            "<https://other.example/x>"
        );
    }

    /// A real-world manifest using a multi-line comma-separated object list: a
    /// stale removal and an undeclared add must rewrite the block correctly,
    /// preserving the list form and the statement terminator.
    #[test]
    fn rewrites_multiline_object_list_block() {
        let manifest = "\
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

<https://blackcatinformatics.ca/purrdf/slices/agentic> a purrdf:Slice ;
    purrdf:sliceTier purrdf:tierExtension ;
    purrdf:sliceDependsOn
        <https://blackcatinformatics.ca/purrdf/slices/ai> ,
        <https://blackcatinformatics.ca/purrdf/slices/stale> ,
        <https://blackcatinformatics.ca/purrdf/slices/kernel> ;
    rdfs:label \"agentic\"@x-purrdf-english .
";
        let p = proposal("agentic", &["entities"], &["stale"]);
        let patched = apply_proposal(manifest, &p).expect("patch must succeed");
        let store = parse_turtle(patched.as_bytes(), "test").expect("must re-parse");
        let subject = format!("{NS}slices/agentic");
        let deps: BTreeSet<String> = store
            .object_iris(&subject, PURRDF_SLICE_DEPENDS_ON)
            .unwrap()
            .into_iter()
            .collect();
        assert_eq!(
            deps,
            BTreeSet::from([
                format!("{NS}slices/ai"),
                format!("{NS}slices/entities"),
                format!("{NS}slices/kernel"),
            ]),
            "stale removed, entities added, ai+kernel retained"
        );
        // The label predicate (after the block) is untouched.
        assert!(patched.contains("rdfs:label \"agentic\"@x-purrdf-english ."));
    }
}

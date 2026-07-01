// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Slice catalog: manifest-based discovery, typed artifact inventory,
//! and content-addressed IDs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;
use sha2::{Digest, Sha256};

use purrdf::RdfDataset;

use crate::artifact::{ArtifactRecord, ArtifactRole};
use crate::error::SliceError;
use crate::rdf_query::{Dataset, Object};

// ── Namespace constants ───────────────────────────────────────────────────────

const PURRDF: &str = "https://blackcatinformatics.ca/purrdf/";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const DCTERMS_TITLE: &str = "http://purl.org/dc/terms/title";
const DCTERMS_CREATOR: &str = "http://purl.org/dc/terms/creator";
const DCTERMS_IDENTIFIER: &str = "http://purl.org/dc/terms/identifier";
const PURRDF_SLICE_TIER: &str = "https://blackcatinformatics.ca/purrdf/sliceTier";
const PURRDF_SLICE_CONSUMER: &str = "https://blackcatinformatics.ca/purrdf/sliceConsumer";
const PURRDF_SLICE_PROFILE: &str = "https://blackcatinformatics.ca/purrdf/sliceProfile";
const PURRDF_SLICE_DEPENDS_ON: &str = "https://blackcatinformatics.ca/purrdf/sliceDependsOn";
const PURRDF_SLICE_CLASS: &str = "https://blackcatinformatics.ca/purrdf/Slice";

/// The tier of a slice in the PurRDF taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SliceTier {
    Core,
    Extension,
    Domain,
    Unknown(String),
}

impl SliceTier {
    fn from_iri(iri: &str) -> Self {
        let base = PURRDF;
        match iri.strip_prefix(base) {
            Some("tierCore") => Self::Core,
            Some("tierExtension") => Self::Extension,
            Some("tierDomain") => Self::Domain,
            _ => Self::Unknown(iri.to_string()),
        }
    }
}

/// A parsed view of the mandatory `manifest.ttl` fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestView {
    /// The IRI of the slice resource (`a purrdf:Slice`).
    pub slice_iri: String,
    /// `rdfs:label` (first, in any language).
    pub label: Option<String>,
    /// `dcterms:title` (first, in any language).
    pub title: Option<String>,
    /// `dcterms:creator` values.
    pub creators: Vec<String>,
    /// `dcterms:identifier` (e.g. DOI).
    pub identifier: Option<String>,
    /// `purrdf:sliceTier`.
    pub tier: Option<SliceTier>,
    /// `purrdf:sliceConsumer` values.
    pub consumers: Vec<String>,
    /// `purrdf:sliceProfile` values — the named profiles this slice declares
    /// membership in (e.g. `claims`, `memory`, `narrative`). Profile-document
    /// generation lives in the pipeline; this view exposes the raw declarations
    /// so docs consumers can compute per-term profile membership (#1026).
    pub profiles: Vec<String>,
    /// `purrdf:sliceDependsOn` values — the slice IRIs this slice depends on.
    /// A named profile's membership is the closure of its declared members over
    /// this relation (#330); docs reuse it for the same closure (#1026).
    pub depends_on: Vec<String>,
}

/// A fully-loaded slice record: manifest view, manifest IR dataset, and artifact
/// inventory.
#[derive(Debug)]
pub struct SliceRecord {
    /// The parsed manifest fields.
    pub manifest: ManifestView,
    /// The manifest's full RDF graph as a frozen IR dataset — lossless round-trip.
    pub manifest_graph: Arc<RdfDataset>,
    /// All artifacts discovered under the slice directory.
    pub artifacts: Vec<ArtifactRecord>,
    /// The on-disk slice directory the record was loaded from. Retained so the
    /// authoritative `manifest.ttl` path is known without any scan/substring
    /// match (#820 G8: the discover walk already knows it).
    pub slice_dir: PathBuf,
}

impl SliceRecord {
    /// The on-disk path to this slice's `manifest.ttl`.
    pub fn manifest_path(&self) -> PathBuf {
        self.slice_dir.join("manifest.ttl")
    }

    /// Find an artifact by role and logical path.
    pub fn find_artifact(&self, role: &ArtifactRole, path: &str) -> Option<&ArtifactRecord> {
        self.artifacts
            .iter()
            .find(|a| &a.role == role && a.logical_path == path)
    }

    /// Find an artifact by its raw SHA-256 digest.
    pub fn find_by_digest(&self, digest: &str) -> Option<&ArtifactRecord> {
        self.artifacts.iter().find(|a| a.raw_digest == digest)
    }
}

/// The slice catalog: a collection of discovered and loaded slice records.
#[derive(Debug)]
pub struct SliceCatalog {
    records: Vec<SliceRecord>,
}

impl SliceCatalog {
    /// Recursively discover all slice directories under `root` (directories
    /// containing a `manifest.ttl`) and load each one.
    pub fn discover(root: &Path) -> Result<Self, SliceError> {
        let dirs = find_slice_dirs(root)?;
        let records: Result<Vec<_>, _> = dirs
            .par_iter()
            .map(|dir| Self::from_slice_dir(dir))
            .collect();
        let records = records?;
        Ok(Self { records })
    }

    /// Load a single slice from `dir` (which must contain `manifest.ttl`).
    pub fn from_slice_dir(dir: &Path) -> Result<SliceRecord, SliceError> {
        let manifest_path = dir.join("manifest.ttl");
        let manifest_bytes = std::fs::read(&manifest_path).map_err(SliceError::Io)?;

        // Parse Turtle once into the native IR (lenient: accepts @x-purrdf-* lang tags).
        // The frozen dataset serves BOTH the manifest-view extraction and the
        // lossless `manifest_graph` — no store→IR round-trip.
        let dataset = parse_manifest(&manifest_bytes, &manifest_path)?;

        // Extract manifest view from the dataset.
        let manifest = extract_manifest_view(&dataset)?;

        // Keep the frozen RdfDataset for lossless round-trip.
        let manifest_graph = Arc::new(dataset.into_inner());

        // Discover all artifacts.
        let artifacts = discover_artifacts(dir)?;

        Ok(SliceRecord {
            manifest,
            manifest_graph,
            artifacts,
            slice_dir: dir.to_path_buf(),
        })
    }

    /// Returns all slice records.
    pub fn records(&self) -> &[SliceRecord] {
        &self.records
    }

    /// Look up a slice record by its IRI.
    pub fn get(&self, iri: &str) -> Option<&SliceRecord> {
        self.records.iter().find(|r| r.manifest.slice_iri == iri)
    }
}

// ── Turtle parsing ────────────────────────────────────────────────────────────

fn parse_manifest(bytes: &[u8], path: &Path) -> Result<Dataset, SliceError> {
    Dataset::parse_turtle(bytes, &path.display().to_string())
}

// ── Manifest extraction ───────────────────────────────────────────────────────

fn extract_manifest_view(ds: &Dataset) -> Result<ManifestView, SliceError> {
    // Find the slice IRI: subject of `a purrdf:Slice`.
    let slice_iri = find_slice_iri(ds)?;

    let mut label: Option<String> = None;
    let mut title: Option<String> = None;
    let mut creators: Vec<String> = Vec::new();
    let mut identifier: Option<String> = None;
    let mut tier: Option<SliceTier> = None;
    let mut consumers: Vec<String> = Vec::new();
    let mut profiles: Vec<String> = Vec::new();
    let mut depends_on: Vec<String> = Vec::new();

    for (predicate, object) in ds.predicate_objects_of(&slice_iri)? {
        match predicate.as_str() {
            p if p == RDFS_LABEL => {
                if label.is_none() {
                    label = Some(literal_value(&object));
                }
            }
            p if p == DCTERMS_TITLE => {
                if title.is_none() {
                    title = Some(literal_value(&object));
                }
            }
            p if p == DCTERMS_CREATOR => {
                creators.push(literal_value(&object));
            }
            p if p == DCTERMS_IDENTIFIER => {
                if identifier.is_none() {
                    identifier = Some(literal_value(&object));
                }
            }
            p if p == PURRDF_SLICE_TIER => {
                if tier.is_none() {
                    if let Object::Named(nn) = &object {
                        tier = Some(SliceTier::from_iri(nn));
                    }
                }
            }
            p if p == PURRDF_SLICE_CONSUMER => {
                consumers.push(literal_value(&object));
            }
            p if p == PURRDF_SLICE_PROFILE => {
                profiles.push(literal_value(&object));
            }
            p if p == PURRDF_SLICE_DEPENDS_ON => {
                if let Object::Named(nn) = &object {
                    depends_on.push(nn.clone());
                }
            }
            _ => {}
        }
    }

    // Deterministic order — quad iteration order is not stable.
    profiles.sort_unstable();
    profiles.dedup();
    depends_on.sort_unstable();
    depends_on.dedup();

    Ok(ManifestView {
        slice_iri,
        label,
        title,
        creators,
        identifier,
        tier,
        consumers,
        profiles,
        depends_on,
    })
}

fn find_slice_iri(ds: &Dataset) -> Result<String, SliceError> {
    let mut subjects: Vec<String> = ds.subjects_of_type(PURRDF_SLICE_CLASS)?;

    match subjects.len() {
        0 => Err(SliceError::InvalidManifest(
            "no `a purrdf:Slice` triple found in manifest.ttl".to_string(),
        )),
        1 => Ok(subjects.remove(0)),
        _ => {
            subjects.sort();
            Err(SliceError::InvalidManifest(format!(
                "manifest.ttl declares {} `a purrdf:Slice` subjects (must be exactly one): {}",
                subjects.len(),
                subjects.join(", ")
            )))
        }
    }
}

/// The string projection of an object term (a literal's lexical value; an IRI/blank
/// rendered the way rdflib/oxigraph surfaced them through `.value()`).
fn literal_value(term: &Object) -> String {
    match term {
        Object::Literal { value } => value.clone(),
        Object::Named(nn) => nn.clone(),
        Object::Blank(label) => format!("_:{label}"),
        Object::Triple => "<triple>".to_string(),
    }
}

// ── Artifact discovery ────────────────────────────────────────────────────────

fn discover_artifacts(dir: &Path) -> Result<Vec<ArtifactRecord>, SliceError> {
    let mut artifacts = Vec::new();
    collect_artifacts(dir, dir, &mut artifacts)?;
    artifacts.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    Ok(artifacts)
}

fn collect_artifacts(
    root: &Path,
    current: &Path,
    out: &mut Vec<ArtifactRecord>,
) -> Result<(), SliceError> {
    for entry in std::fs::read_dir(current).map_err(SliceError::Io)? {
        let entry = entry.map_err(SliceError::Io)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(SliceError::Io)?;

        if file_type.is_dir() {
            collect_artifacts(root, &path, out)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        // Compute relative logical path.
        let rel = path.strip_prefix(root).map_err(|_| {
            SliceError::InvalidPath(format!("path not under root: {}", path.display()))
        })?;

        // Validate: no absolute components, no `..`.
        for component in rel.components() {
            use std::path::Component;
            match component {
                Component::Normal(_) => {}
                Component::CurDir => {}
                other => {
                    return Err(SliceError::InvalidPath(format!(
                        "unsafe path component {other:?} in {}",
                        rel.display()
                    )));
                }
            }
        }

        let logical_path = rel.to_string_lossy().to_string();
        let content = std::fs::read(&path).map_err(SliceError::Io)?;
        let raw_digest = hex_sha256(&content);

        let role = classify_role(&logical_path);
        let media_type = infer_media_type(&logical_path);

        // For RDF files, compute semantic digest via sorted N-Triples.
        let semantic_digest = if is_rdf_file(&logical_path) {
            compute_semantic_digest(&content, &path).ok()
        } else {
            None
        };

        out.push(ArtifactRecord {
            role,
            logical_path,
            media_type,
            raw_digest,
            semantic_digest,
            content,
        });
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn parse_rdf_to_dataset(bytes: &[u8], path: &Path) -> Result<Dataset, SliceError> {
    let media_type = crate::rdf_query::media_type_for_path(path);
    Dataset::parse(bytes, media_type, &path.display().to_string())
}

fn compute_semantic_digest(bytes: &[u8], path: &Path) -> Result<String, SliceError> {
    let dataset = parse_rdf_to_dataset(bytes, path)?;

    // Canonicalize blank-node labels BEFORE digesting. Parsing assigns blank-node IDs
    // non-deterministically, so a plain sorted-N-Triples digest would differ
    // run-to-run for any artifact that uses blank nodes (e.g. SHACL property shapes,
    // OWL restrictions). Canonicalization makes the *semantic* digest a stable
    // function of the graph's identity (RFC #820 §12 — semantic, path-independent,
    // deterministic keys).
    //
    // Native full RDFC-1.0 (#910): the `canonical_nquads_flat` projection flattens the
    // RDF 1.2 statement overlay back to plain `rdf:reifies`/annotation triples and
    // canonicalizes that flat set, byte-identical to the prior oxigraph-quad path.
    let canonical = dataset.canonical_nquads_flat()?;
    Ok(hex_sha256(canonical.as_bytes()))
}

// Artifact classification is deliberately byte-exact on lowercase extensions.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn is_rdf_file(path: &str) -> bool {
    path.ends_with(".ttl") || path.ends_with(".nt") || path.ends_with(".nq")
}

fn classify_role(path: &str) -> ArtifactRole {
    let name = path.split('/').next_back().unwrap_or(path);
    // Top-level well-known files.
    match name {
        "manifest.ttl" => return ArtifactRole::Manifest,
        "module.ttl" => return ArtifactRole::Module,
        "shapes.ttl" => return ArtifactRole::Shapes,
        "docs.md" => return ArtifactRole::Documentation,
        "CITATION.cff" => return ArtifactRole::Citation,
        _ => {}
    }
    // Directory-based classification.
    if path.starts_with("mappings/") {
        return ArtifactRole::Mapping;
    }
    if path.starts_with("queries/competency/") {
        return ArtifactRole::CompetencyQuery;
    }
    if path.starts_with("queries/verify/") {
        return ArtifactRole::VerifyQuery;
    }
    if path.starts_with("tests/counter-examples/") {
        return ArtifactRole::CounterExample;
    }
    if path.starts_with("tests/") {
        return ArtifactRole::TestDsl;
    }
    if path.starts_with("examples/") {
        return ArtifactRole::Example;
    }
    if path.starts_with("i18n/") {
        return ArtifactRole::TranslationCatalog;
    }
    ArtifactRole::Other(path.to_string())
}

fn infer_media_type(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "ttl" => "text/turtle",
        "nt" => "application/n-triples",
        "nq" => "application/n-quads",
        "sparql" | "rq" => "application/sparql-query",
        "md" => "text/markdown",
        "yaml" | "yml" => "application/yaml",
        "json" => "application/json",
        "cff" => "application/yaml",
        _ => "application/octet-stream",
    }
    .to_string()
}

// ── Recursive slice-dir discovery ─────────────────────────────────────────────

fn find_slice_dirs(root: &Path) -> Result<Vec<PathBuf>, SliceError> {
    let mut dirs = Vec::new();
    find_slice_dirs_inner(root, &mut dirs)?;
    dirs.sort();
    Ok(dirs)
}

fn find_slice_dirs_inner(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), SliceError> {
    let manifest = dir.join("manifest.ttl");
    if manifest.exists() {
        out.push(dir.to_path_buf());
        // Don't recurse into a slice dir — slices are not nested.
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(SliceError::Io)? {
        let entry = entry.map_err(SliceError::Io)?;
        if entry.file_type().map_err(SliceError::Io)?.is_dir() {
            find_slice_dirs_inner(&entry.path(), out)?;
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Fix 1: a manifest declaring two `a purrdf:Slice` subjects must hard-fail
    /// with `SliceError::InvalidManifest` naming both subjects.
    #[test]
    fn find_slice_iri_rejects_multiple_slice_subjects() {
        let ttl = r"
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .

<https://example.org/slice/alpha> a purrdf:Slice .
<https://example.org/slice/beta>  a purrdf:Slice .
";
        let path = Path::new("manifest.ttl");
        let ds = parse_manifest(ttl.as_bytes(), path).expect("should parse without error");
        let result = find_slice_iri(&ds);
        match result {
            Err(SliceError::InvalidManifest(msg)) => {
                assert!(
                    msg.contains("https://example.org/slice/alpha"),
                    "error message must name first subject, got: {msg}"
                );
                assert!(
                    msg.contains("https://example.org/slice/beta"),
                    "error message must name second subject, got: {msg}"
                );
                assert!(
                    msg.contains('2') || msg.contains("two"),
                    "error message must state the count, got: {msg}"
                );
            }
            other => panic!("expected InvalidManifest error, got: {other:?}"),
        }
    }

    /// Fix 2: an N-Triples artifact produces a correct, non-empty semantic
    /// digest that equals the digest of the same triples written as Turtle.
    #[test]
    fn compute_semantic_digest_handles_nt_artifacts() {
        // The same single triple expressed in two formats.
        let turtle_bytes = br"
@prefix ex: <https://example.org/> .
ex:subject ex:predicate ex:object .
";
        let nt_bytes = b"<https://example.org/subject> <https://example.org/predicate> <https://example.org/object> .\n";

        let ttl_path = Path::new("data.ttl");
        let nt_path = Path::new("data.nt");

        let digest_ttl = compute_semantic_digest(turtle_bytes, ttl_path)
            .expect("Turtle semantic digest must not fail");
        let digest_nt = compute_semantic_digest(nt_bytes, nt_path)
            .expect("N-Triples semantic digest must not fail");

        assert!(
            !digest_ttl.is_empty(),
            "Turtle semantic digest must not be empty"
        );
        assert!(
            !digest_nt.is_empty(),
            "N-Triples semantic digest must not be empty"
        );
        assert_eq!(
            digest_ttl, digest_nt,
            "Turtle and N-Triples digests for identical triples must match"
        );
    }
}

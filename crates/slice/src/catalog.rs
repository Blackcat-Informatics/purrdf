// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Slice catalog: manifest-based discovery, typed artifact inventory,
//! and content-addressed IDs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rayon::prelude::*;
use sha2::{Digest, Sha256};

use purrdf_lex::json::{Object as JsonObject, Value as JsonValue};
use purrdf_rdf::RdfDataset;

use crate::artifact::{ArtifactInput, ArtifactRecord, ArtifactRole};
use crate::error::SliceError;
use crate::json_form;
use crate::rdf_query::{Dataset, Object};
use crate::vocab::SliceVocab;

// ── Namespace constants ───────────────────────────────────────────────────────
//
// Only W3C/DCMI terms are hardcoded; every slice-framework term (`Slice`,
// `sliceTier`, `sliceDependsOn`, …) comes from the caller's [`SliceVocab`].

use purrdf_iri::vocab::rdfs::LABEL as RDFS_LABEL;
const DCTERMS_TITLE: &str = "http://purl.org/dc/terms/title";
const DCTERMS_CREATOR: &str = "http://purl.org/dc/terms/creator";
const DCTERMS_IDENTIFIER: &str = "http://purl.org/dc/terms/identifier";

/// The tier of a slice in the slice taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SliceTier {
    /// A core slice (`<vocab>tierCore`).
    Core,
    /// An extension slice (`<vocab>tierExtension`).
    Extension,
    /// A domain slice (`<vocab>tierDomain`).
    Domain,
    /// A tier IRI outside the caller's vocabulary; carries the raw IRI.
    Unknown(String),
}

impl SliceTier {
    fn from_iri(iri: &str, vocab: &SliceVocab) -> Self {
        let base = vocab.ns();
        match iri.strip_prefix(base) {
            Some("tierCore") => Self::Core,
            Some("tierExtension") => Self::Extension,
            Some("tierDomain") => Self::Domain,
            _ => Self::Unknown(iri.to_string()),
        }
    }
}

/// A parsed view of the mandatory `manifest.ttl` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestView {
    /// The IRI of the slice resource (`a <vocab>Slice`).
    pub slice_iri: String,
    /// `rdfs:label` (first, in any language).
    pub label: Option<String>,
    /// `dcterms:title` (first, in any language).
    pub title: Option<String>,
    /// `dcterms:creator` values.
    pub creators: Vec<String>,
    /// `dcterms:identifier` (e.g. DOI).
    pub identifier: Option<String>,
    /// `<vocab>sliceTier`.
    pub tier: Option<SliceTier>,
    /// `<vocab>sliceConsumer` values.
    pub consumers: Vec<String>,
    /// `<vocab>sliceProfile` values — the named profiles this slice declares
    /// membership in (e.g. `claims`, `memory`, `narrative`). Profile-document
    /// generation lives in the pipeline; this view exposes the raw declarations
    /// so docs consumers can compute per-term profile membership.
    pub profiles: Vec<String>,
    /// `<vocab>sliceDependsOn` values — the slice IRIs this slice depends on.
    /// A named profile's membership is the closure of its declared members over
    /// this relation; docs reuse it for the same closure.
    pub depends_on: Vec<String>,
}

impl SliceTier {
    /// The tier as JSON: a named tier is its variant name (`"Core"`), and
    /// [`SliceTier::Unknown`] is `{"Unknown": <iri>}`.
    pub fn to_json(&self) -> JsonValue {
        match self {
            Self::Core => JsonValue::from("Core"),
            Self::Extension => JsonValue::from("Extension"),
            Self::Domain => JsonValue::from("Domain"),
            Self::Unknown(iri) => JsonValue::from(JsonObject::new().with("Unknown", iri)),
        }
    }

    /// The tier [`SliceTier::to_json`] wrote.
    ///
    /// # Errors
    ///
    /// [`SliceError::Json`] for any other value.
    pub fn from_json(value: &JsonValue) -> Result<Self, SliceError> {
        match value.as_str() {
            Some("Core") => Ok(Self::Core),
            Some("Extension") => Ok(Self::Extension),
            Some("Domain") => Ok(Self::Domain),
            Some(other) => Err(SliceError::Json(crate::error::JsonRefusal::Unknown {
                category: "slice tier",
                name: other.to_owned(),
            })),
            None => json_form::newtype_variant(value, "Unknown").map(Self::Unknown),
        }
    }
}

impl ManifestView {
    /// The view as a JSON object whose members are its fields, in declaration
    /// order; an absent optional field is `null`.
    pub fn to_json(&self) -> JsonValue {
        JsonValue::from(
            JsonObject::new()
                .with("slice_iri", &self.slice_iri)
                .with("label", self.label.as_deref())
                .with("title", self.title.as_deref())
                .with("creators", self.creators.as_slice())
                .with("identifier", self.identifier.as_deref())
                .with("tier", self.tier.as_ref().map(SliceTier::to_json))
                .with("consumers", self.consumers.as_slice())
                .with("profiles", self.profiles.as_slice())
                .with("depends_on", self.depends_on.as_slice()),
        )
    }

    /// The view [`ManifestView::to_json`] wrote.
    ///
    /// # Errors
    ///
    /// [`SliceError::Json`] when a field is missing or has the wrong type.
    pub fn from_json(value: &JsonValue) -> Result<Self, SliceError> {
        let object = json_form::object(value, "a manifest view")?;
        let tier = match object.get("tier") {
            None | Some(JsonValue::Null) => None,
            Some(tier) => Some(SliceTier::from_json(tier)?),
        };
        Ok(Self {
            slice_iri: json_form::string(object, "slice_iri")?,
            label: json_form::optional_string(object, "label")?,
            title: json_form::optional_string(object, "title")?,
            creators: json_form::strings(object, "creators")?,
            identifier: json_form::optional_string(object, "identifier")?,
            tier,
            consumers: json_form::strings(object, "consumers")?,
            profiles: json_form::strings(object, "profiles")?,
            depends_on: json_form::strings(object, "depends_on")?,
        })
    }
}

/// Physical inputs for one slice with an admitted manifest and unadmitted artifacts.
///
/// Artifact bytes and roles come from the same discovery used by [`SliceCatalog`].
/// Only the mandatory manifest has been parsed; other RDF artifacts have not been
/// parsed or assigned semantic identities. Use [`Self::into_record`] before a
/// semantic catalog operation.
#[derive(Debug)]
pub struct SliceInput {
    /// The required manifest's parsed fields, including its slice owner IRI.
    manifest: ManifestView,
    /// The required manifest's complete native RDF dataset.
    manifest_graph: Arc<RdfDataset>,
    /// All physical artifacts, ordered by their relative logical paths.
    artifacts: Vec<ArtifactInput>,
    /// The directory containing the required `manifest.ttl`.
    slice_dir: PathBuf,
}

impl SliceInput {
    /// The admitted manifest fields, including the owner IRI of these physical inputs.
    pub fn manifest(&self) -> &ManifestView {
        &self.manifest
    }

    /// Physical artifacts in relative path order, without semantic identity claims.
    pub fn artifacts(&self) -> &[ArtifactInput] {
        &self.artifacts
    }

    /// The original directory containing this slice's required manifest.
    pub fn slice_dir(&self) -> &Path {
        &self.slice_dir
    }

    /// Admit every retained RDF artifact and consume these inputs into a catalog record.
    ///
    /// Uses the original bytes and retrieval IRI without rereading files or deriving
    /// another base from the filesystem. Non-RDF files retain no semantic digest;
    /// every RDF artifact must acquire one.
    ///
    /// # Errors
    /// Returns the original artifact parse or canonicalization refusal.
    /// No partially admitted record is returned.
    pub fn into_record(
        self,
        control: &mut purrdf_core::CanonicalizationControl<'_>,
    ) -> Result<SliceRecord, SliceError> {
        let mut artifacts = Vec::with_capacity(self.artifacts.len());
        for artifact in self.artifacts {
            let semantic_digest = if let Some(base) = &artifact.rdf_base {
                Some(compute_semantic_digest(
                    &artifact.content,
                    &self.slice_dir.join(&artifact.logical_path),
                    base,
                    control,
                )?)
            } else {
                None
            };
            artifacts.push(ArtifactRecord {
                role: artifact.role,
                logical_path: artifact.logical_path,
                media_type: artifact.media_type,
                raw_digest: artifact.raw_digest,
                semantic_digest,
                content: artifact.content,
            });
        }
        Ok(SliceRecord {
            manifest: self.manifest,
            manifest_graph: self.manifest_graph,
            artifacts,
            slice_dir: self.slice_dir,
        })
    }
}

/// A physical slice inventory with required manifest admission and no RDF artifact digests.
#[derive(Debug)]
pub struct SliceInventory {
    records: Vec<SliceInput>,
    vocab: SliceVocab,
}

impl SliceInventory {
    /// Discover the same slice directories, manifests and artifact roles as [`SliceCatalog`].
    ///
    /// This operation selects physical inputs; it does not canonicalize RDF artifacts.
    /// It returns no partial inventory if a required input cannot be loaded.
    ///
    /// # Errors
    /// Returns directory, file, path or mandatory manifest failures.
    pub fn discover(root: &Path, vocab: SliceVocab) -> Result<Self, SliceError> {
        let records = find_slice_dirs(root)?
            .par_iter()
            .map(|dir| Self::from_slice_dir(dir, &vocab))
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { records, vocab })
    }

    /// Load one physical slice, parsing its required manifest and retaining artifact bytes.
    ///
    /// # Errors
    /// Returns directory, file, path or mandatory manifest failures. Invalid nonmanifest
    /// RDF remains a physical input until [`SliceInput::into_record`] admits it.
    pub fn from_slice_dir(dir: &Path, vocab: &SliceVocab) -> Result<SliceInput, SliceError> {
        let manifest_path = dir.join("manifest.ttl");
        let manifest_bytes = std::fs::read(&manifest_path).map_err(SliceError::from)?;
        let dataset = Dataset::parse_file(&manifest_bytes, &manifest_path)?;
        let manifest = extract_manifest_view(&dataset, vocab)?;
        let manifest_graph = Arc::new(dataset.into_inner());
        let artifacts = discover_artifacts(dir)?;
        Ok(SliceInput {
            manifest,
            manifest_graph,
            artifacts,
            slice_dir: dir.to_path_buf(),
        })
    }

    /// The vocabulary used to interpret the required manifests.
    pub fn vocab(&self) -> &SliceVocab {
        &self.vocab
    }

    /// Physical slice inputs in the catalog's directory order.
    pub fn records(&self) -> &[SliceInput] {
        &self.records
    }
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
    /// match (the discover walk already knows it).
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

/// The slice catalog: a collection of discovered and loaded slice records,
/// plus the caller's [`SliceVocab`] the manifests were interpreted with.
#[derive(Debug)]
pub struct SliceCatalog {
    records: Vec<SliceRecord>,
    vocab: SliceVocab,
}

impl SliceCatalog {
    /// Recursively discover all slice directories under `root` (directories
    /// containing a `manifest.ttl`) and load each one, interpreting manifests
    /// with the caller's `vocab`.
    pub fn discover(
        root: &Path,
        vocab: SliceVocab,
        control: &mut purrdf_core::CanonicalizationControl<'_>,
    ) -> Result<Self, SliceError> {
        let dirs = find_slice_dirs(root)?;
        let completed = std::sync::Mutex::new(Vec::new());
        // The unit error only stops dispatch. Every started task first retains its
        // actual result and completed evidence, including an original native error.
        let _dispatch = dirs.par_iter().enumerate().try_for_each(|(index, dir)| {
            let mut child = control.fork();
            let result = Self::from_slice_dir(dir, &vocab, &mut child);
            let failed = result.is_err();
            completed
                .lock()
                .expect("catalog receipt lock is not held during task execution")
                .push((index, result, child));
            if failed { Err(()) } else { Ok(()) }
        });
        let mut completed = completed
            .into_inner()
            .expect("catalog receipt lock is not held during task execution");
        completed.sort_unstable_by_key(|(index, _, _)| *index);
        let mut records = Vec::new();
        let mut first_error = None;
        for (_, result, child) in completed {
            control.append_completed(child);
            match result {
                Ok(record) => records.push(record),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        // Directory order chooses among actual failed tasks. Unstarted tasks have
        // neither fabricated results nor receipts; no partial catalog is published.
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(Self { records, vocab })
    }

    /// The vocabulary this catalog's manifests were interpreted with.
    pub fn vocab(&self) -> &SliceVocab {
        &self.vocab
    }

    /// Load a single slice from `dir` (which must contain `manifest.ttl`).
    pub fn from_slice_dir(
        dir: &Path,
        vocab: &SliceVocab,
        control: &mut purrdf_core::CanonicalizationControl<'_>,
    ) -> Result<SliceRecord, SliceError> {
        SliceInventory::from_slice_dir(dir, vocab)?.into_record(control)
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

// ── Manifest extraction ───────────────────────────────────────────────────────

fn extract_manifest_view(ds: &Dataset, vocab: &SliceVocab) -> Result<ManifestView, SliceError> {
    // Find the slice IRI: subject of `a <vocab>Slice`.
    let slice_iri = find_slice_iri(ds, vocab)?;
    let slice_tier = vocab.slice_tier();
    let slice_consumer = vocab.slice_consumer();
    let slice_profile = vocab.slice_profile();
    let slice_depends_on = vocab.slice_depends_on();

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
            p if p == slice_tier => {
                if tier.is_none()
                    && let Object::Named(nn) = &object
                {
                    tier = Some(SliceTier::from_iri(nn, vocab));
                }
            }
            p if p == slice_consumer => {
                consumers.push(literal_value(&object));
            }
            p if p == slice_profile => {
                profiles.push(literal_value(&object));
            }
            p if p == slice_depends_on => {
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

fn find_slice_iri(ds: &Dataset, vocab: &SliceVocab) -> Result<String, SliceError> {
    let mut subjects: Vec<String> = ds.subjects_of_type(&vocab.slice_class())?;
    let prefix = vocab.prefix_name();

    match subjects.len() {
        0 => Err(SliceError::InvalidManifest(
            crate::error::ManifestRefusal::SliceSubjects {
                prefix: prefix.to_owned(),
                subjects,
            },
        )),
        1 => Ok(subjects.remove(0)),
        _ => {
            subjects.sort();
            Err(SliceError::InvalidManifest(
                crate::error::ManifestRefusal::SliceSubjects {
                    prefix: prefix.to_owned(),
                    subjects,
                },
            ))
        }
    }
}

/// The string projection of an object term (a literal's lexical value; an IRI/blank
/// rendered as its IRI string or `_:label`).
fn literal_value(term: &Object) -> String {
    match term {
        Object::Literal { value, .. } => value.clone(),
        Object::Named(nn) => nn.clone(),
        Object::Blank(label) => format!("_:{label}"),
        Object::Triple(_) => "<triple>".to_string(),
    }
}

// ── Artifact discovery ────────────────────────────────────────────────────────

fn discover_artifacts(dir: &Path) -> Result<Vec<ArtifactInput>, SliceError> {
    let mut artifacts = Vec::new();
    collect_artifacts(dir, dir, &mut artifacts)?;
    artifacts.sort_by(|a, b| a.logical_path.cmp(&b.logical_path));
    Ok(artifacts)
}

fn collect_artifacts(
    root: &Path,
    current: &Path,
    out: &mut Vec<ArtifactInput>,
) -> Result<(), SliceError> {
    for entry in std::fs::read_dir(current).map_err(SliceError::from)? {
        let entry = entry.map_err(SliceError::from)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(SliceError::from)?;

        if file_type.is_dir() {
            collect_artifacts(root, &path, out)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        // Compute relative logical path.
        let rel = path.strip_prefix(root).map_err(|_| {
            SliceError::InvalidPath(crate::error::PathRefusal::OutsideRoot {
                path: path.clone(),
                root: root.to_path_buf(),
            })
        })?;

        // Validate: no absolute components, no `..`.
        for component in rel.components() {
            use std::path::Component;
            match component {
                Component::Normal(_) => {}
                Component::CurDir => {}
                other => {
                    return Err(SliceError::InvalidPath(
                        crate::error::PathRefusal::Component {
                            path: rel.to_path_buf(),
                            component: other.as_os_str().to_owned(),
                        },
                    ));
                }
            }
        }

        let logical_path = rel.to_string_lossy().to_string();
        let content = std::fs::read(&path).map_err(SliceError::from)?;
        let raw_digest = hex_sha256(&content);

        let role = classify_role(&logical_path);
        let media_type =
            purrdf_gts::files::media_type_for_path(Path::new(&logical_path)).to_string();
        let rdf_base = if is_rdf_file(&logical_path) {
            Some(crate::retrieval::retrieval_base_iri(&path)?)
        } else {
            None
        };

        out.push(ArtifactInput {
            role,
            logical_path,
            media_type,
            raw_digest,
            content,
            rdf_base,
        });
    }
    Ok(())
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    purrdf_hash::hex::encode(&digest)
}

/// The semantic (canonical N-Triples) digest of one RDF artifact.
///
/// # Errors
///
/// Every failure is propagated and names the artifact: a file the extension declares to
/// be RDF that does not parse or canonicalize is a defect in the slice, not an
/// artifact without a semantic identity. The base was captured during physical discovery.
fn compute_semantic_digest(
    bytes: &[u8],
    path: &Path,
    base: &purrdf_iri::BaseIri,
    control: &mut purrdf_core::CanonicalizationControl<'_>,
) -> Result<String, SliceError> {
    let dataset = Dataset::parse_with_retained_base(bytes, path, base)?;

    // Canonical label search and digest output share one measured native run.
    let digest = match purrdf_core::try_flat_digest_view(
        dataset.inner(),
        purrdf_core::CanonHash::Sha256,
        control,
    ) {
        Ok(digest) => digest,
        Err(purrdf_core::ViewCanonError::Refused(cause)) => {
            return Err(match cause {
                purrdf_core::CanonError::Storage(storage) => SliceError::Storage(storage),
                cause => SliceError::Canonicalize {
                    path: path.to_path_buf(),
                    cause: Box::new(cause),
                },
            });
        }
        Err(purrdf_core::ViewCanonError::NotReady { error, .. }) => match error {},
    };
    Ok(digest.to_hex())
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
    for entry in std::fs::read_dir(dir).map_err(SliceError::from)? {
        let entry = entry.map_err(SliceError::from)?;
        if entry.file_type().map_err(SliceError::from)?.is_dir() {
            find_slice_dirs_inner(&entry.path(), out)?;
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_view_json_round_trips_every_tier() {
        for tier in [
            None,
            Some(SliceTier::Core),
            Some(SliceTier::Extension),
            Some(SliceTier::Domain),
            Some(SliceTier::Unknown("https://example.org/tierX".to_owned())),
        ] {
            let view = ManifestView {
                slice_iri: "https://example.org/slice/a".to_owned(),
                label: Some("A".to_owned()),
                title: None,
                creators: vec!["https://example.org/c".to_owned()],
                identifier: None,
                tier,
                consumers: Vec::new(),
                profiles: vec!["claims".to_owned()],
                depends_on: vec!["https://example.org/slice/b".to_owned()],
            };
            assert_eq!(ManifestView::from_json(&view.to_json()).unwrap(), view);
        }
    }

    #[test]
    fn slice_tier_from_json_refuses_an_unknown_name_and_accepts_a_known_one() {
        assert!(SliceTier::from_json(&JsonValue::from("Kernel")).is_err());
        assert_eq!(
            SliceTier::from_json(&JsonValue::from("Domain")).unwrap(),
            SliceTier::Domain
        );
        let unknown = JsonValue::from(JsonObject::new().with("Unknown", "https://example.org/t"));
        assert_eq!(
            purrdf_lex::json::write_compact(&unknown),
            r#"{"Unknown":"https://example.org/t"}"#
        );
        assert!(SliceTier::from_json(&unknown).is_ok());
    }

    /// Fix 1: a manifest declaring two `a <vocab>Slice` subjects must hard-fail
    /// with `SliceError::InvalidManifest` naming both subjects. The vocabulary
    /// is caller-supplied; the fixture uses example.org.
    #[test]
    fn find_slice_iri_rejects_multiple_slice_subjects() {
        let ttl = r"
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix vocab: <https://example.org/vocab/> .

<https://example.org/slice/alpha> a vocab:Slice .
<https://example.org/slice/beta>  a vocab:Slice .
";
        let dir = purrdf_testkit::TempDir::for_unit_test().expect("tempdir");
        let path = dir.path().join("manifest.ttl");
        std::fs::write(&path, ttl).expect("write the manifest");
        let ds = Dataset::parse_file(ttl.as_bytes(), &path).expect("should parse without error");
        let vocab = SliceVocab::for_namespace("https://example.org/vocab/");
        let result = find_slice_iri(&ds, &vocab);
        drop(ds);
        drop(vocab);
        match result {
            Err(SliceError::InvalidManifest(msg)) => {
                let fields = purrdf_testkit::error_capture::Capture::read(&msg);
                assert_eq!(fields.code, "slice-subject-count");
                assert!(matches!(
                    fields.field("required"),
                    purrdf_hash::error::ErrorValue::Unsigned(1)
                ));
                for subject in [
                    "https://example.org/slice/alpha",
                    "https://example.org/slice/beta",
                ] {
                    assert!(fields.events.iter().any(|event| matches!(event, purrdf_testkit::error_capture::Event::Value(None, purrdf_hash::error::ErrorValue::Iri(value)) if *value == subject)));
                }
                let msg = msg.to_string();
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

        let dir = purrdf_testkit::TempDir::for_unit_test().expect("tempdir");
        let ttl_path = dir.path().join("data.ttl");
        let nt_path = dir.path().join("data.nt");
        std::fs::write(&ttl_path, turtle_bytes).expect("write the Turtle artifact");
        std::fs::write(&nt_path, nt_bytes).expect("write the N-Triples artifact");

        let digest_ttl = compute_semantic_digest(
            turtle_bytes,
            &ttl_path,
            &crate::retrieval::retrieval_base_iri(&ttl_path).expect("retrieval IRI"),
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .expect("Turtle semantic digest must not fail");
        let digest_nt = compute_semantic_digest(
            nt_bytes,
            &nt_path,
            &crate::retrieval::retrieval_base_iri(&nt_path).expect("retrieval IRI"),
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
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

    /// An artifact carrying a RELATIVE IRI is digestible: it resolves against the
    /// artifact's own RFC-8089 retrieval IRI (RFC-3986 §5.1.3) instead of hard-failing
    /// with `iri-relative-no-base` while that IRI sits unused in `path`.
    #[test]
    fn compute_semantic_digest_resolves_relative_iris_against_the_artifact() {
        let dir = purrdf_testkit::TempDir::for_unit_test().expect("tempdir");
        let path = dir.path().join("module.ttl");
        let relative = b"<> <https://example.org/declares> <thing> .\n";
        std::fs::write(&path, relative).expect("write the artifact");

        let digest = compute_semantic_digest(
            relative,
            &path,
            &crate::retrieval::retrieval_base_iri(&path).expect("retrieval IRI"),
            &mut purrdf_core::CanonicalizationControl::new(
                purrdf_core::RDFC_V2_CORPUS_POLICY,
                None,
            ),
        )
        .expect("a relative-IRI artifact resolves under its own retrieval IRI");
        assert_ne!(digest, "");

        // The digest is of the RESOLVED graph: writing the same document with the
        // references already expanded against that base gives the same digest.
        let base = crate::retrieval::retrieval_base_iri(&path).expect("retrieval IRI");
        let expanded = format!(
            "<{}> <https://example.org/declares> <{}> .\n",
            base.as_str(),
            base.resolve("thing").expect("resolve").as_str()
        );
        let expanded_path = dir.path().join("expanded.ttl");
        std::fs::write(&expanded_path, &expanded).expect("write the expanded artifact");
        assert_eq!(
            digest,
            compute_semantic_digest(
                expanded.as_bytes(),
                &expanded_path,
                &crate::retrieval::retrieval_base_iri(&expanded_path).expect("retrieval IRI"),
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None
                )
            )
            .expect("absolute artifact digests"),
            "the relative form must denote exactly what the resolved form denotes"
        );
    }

    /// An all-absolute artifact's semantic digest does NOT depend on where the tree sits
    /// on disk: the base is only ever consulted for a relative reference, so supplying
    /// the retrieval IRI cannot make a committed digest machine-dependent.
    #[test]
    fn absolute_artifacts_digest_identically_from_two_locations() {
        let absolute =
            b"<https://example.org/a> <https://example.org/p> <https://example.org/b> .\n";
        let digest_at = |name: &str| {
            let dir = purrdf_testkit::TempDir::for_unit_test().expect("tempdir");
            let path = dir.path().join(name);
            std::fs::write(&path, absolute).expect("write");
            compute_semantic_digest(
                absolute,
                &path,
                &crate::retrieval::retrieval_base_iri(&path).expect("retrieval IRI"),
                &mut purrdf_core::CanonicalizationControl::new(
                    purrdf_core::RDFC_V2_CORPUS_POLICY,
                    None,
                ),
            )
            .expect("digest")
        };
        assert_eq!(digest_at("one.ttl"), digest_at("two.ttl"));
    }
}

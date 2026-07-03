// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The self-describing bundle resource layer (S3).
//!
//! [`RdfBundle`] is the kernel-generic, repo-free package that carries
//! *everything* needed to recover a compilation product without a filesystem:
//!
//! ```text
//! RdfBundle
//! ├── dataset:    RdfDataset          // — the hot graph
//! ├── provenance: DatasetProvenance   // S2 — units / occurrences / origins
//! ├── units:      UnitCatalog         // UnitId -> metadata
//! ├── artifacts:  ArtifactIndex       // ArtifactId -> ArtifactRecord (no bytes)
//! └── blobs:      ContentStore        // the actual bytes, content-addressed
//! ```
//!
//! This module replaces the old lookaside blob records that carried *no bytes*
//! and the project-wide tar aggregates with individually indexed,
//! content-addressed artifacts: each [`ArtifactRecord`] holds a [`ContentDigest`]
//! reference into the [`ContentStore`], never the payload itself
//! (blob-by-reference doctrine). A multi-gigabyte artifact moves through the
//! index as one 32-byte digest.
//!
//! ## Kernel purity
//!
//! There is **no** `SliceId` here. The bundle is keyed by the generic
//! [`UnitId`] / [`ArtifactId`] newtypes (S0.2). The `purrdf-slice` layer interprets
//! a unit's *kind*; this module only carries opaque ids and string metadata.
//!
//! ## Hard-fail loading
//!
//! [`RdfBundle::load`] never silently repairs. It rejects, with a typed `Err`:
//! a content-digest mismatch, a duplicate logical artifact path, an absolute
//! path, a `..` traversal component, and conflicting manifests for one unit IRI.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::content_store::{ContentDigest, ContentStore, ContentStoreError};
use crate::ir::RdfDataset;
use crate::provenance::{ArtifactId, DatasetProvenance, UnitId};

// ─── Unit catalog ─────────────────────────────────────────────────────────────

/// Metadata describing one compilation unit (the kernel-generic projection of a
/// slice / root ontology / import / generated graph / runtime input).
///
/// `iri` is the unit's public identity (e.g. a slice IRI) used for the
/// conflicting-manifest check at load time; `name` is a human label. Neither is
/// interpreted by the kernel beyond equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitMetadata {
    /// Public IRI of the unit (e.g. the slice IRI). Two units sharing an IRI but
    /// disagreeing on metadata are a conflicting-manifest hard error at load.
    pub iri: String,
    /// Human-readable name/label for the unit.
    pub name: String,
}

impl UnitMetadata {
    /// Build unit metadata from an IRI and a name.
    pub fn new(iri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            iri: iri.into(),
            name: name.into(),
        }
    }
}

/// Maps each [`UnitId`] to its [`UnitMetadata`].
///
/// The catalog is dense by `UnitId` index, mirroring the provenance unit
/// interner. Inserting a unit twice with conflicting metadata is rejected by the
/// loader (S3 conflicting-manifest rule).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitCatalog {
    entries: HashMap<UnitId, UnitMetadata>,
}

impl UnitCatalog {
    /// A fresh, empty catalog.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record metadata for `unit`. The last write wins; conflict detection is the
    /// loader's job (see [`RdfBundle::load`]).
    pub fn insert(&mut self, unit: UnitId, meta: UnitMetadata) {
        self.entries.insert(unit, meta);
    }

    /// Look up a unit's metadata.
    pub fn get(&self, unit: UnitId) -> Option<&UnitMetadata> {
        self.entries.get(&unit)
    }

    /// The number of catalogued units.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no units are catalogued.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate `(unit, metadata)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&UnitId, &UnitMetadata)> {
        self.entries.iter()
    }
}

// ─── Artifact index ───────────────────────────────────────────────────────────

/// One packaged artifact: a content-addressed reference into the
/// [`ContentStore`], with **no inline payload bytes**.
///
/// `role` is an opaque string in the kernel (the slice layer maps its typed
/// `ArtifactRole` onto it). `blob_id` is the only handle to the bytes — recovery
/// is `ContentStore::get(blob_id)`. `logical_path` is normalized: relative, no
/// `..`, no leading `/` (enforced at load).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// The artifact's id within the bundle's provenance.
    pub artifact_id: ArtifactId,
    /// Normalized, relative logical path (no leading `/`, no `..` component).
    pub logical_path: String,
    /// Opaque role string (e.g. `"Module"`, `"Shapes"`, `"Documentation"`).
    pub role: String,
    /// Content id of the payload bytes in the [`ContentStore`].
    pub blob_id: ContentDigest,
    /// The compilation unit this artifact belongs to.
    pub unit_id: UnitId,
}

/// Index of [`ArtifactRecord`]s with lookup by [`ArtifactId`], by logical path,
/// and by [`UnitId`].
///
/// Insertion order is preserved for deterministic iteration; the secondary maps
/// are maintained alongside the primary store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArtifactIndex {
    records: Vec<ArtifactRecord>,
    by_id: HashMap<ArtifactId, usize>,
    by_path: HashMap<String, usize>,
    by_unit: HashMap<UnitId, Vec<usize>>,
}

impl ArtifactIndex {
    /// A fresh, empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an artifact record. Duplicate logical paths and duplicate
    /// `ArtifactId`s are *not* rejected here — the loader enforces those rules so
    /// builder code can stage freely; [`RdfBundle::load`] is the gate.
    pub fn insert(&mut self, record: ArtifactRecord) {
        let position = self.records.len();
        self.by_id.entry(record.artifact_id).or_insert(position);
        self.by_path
            .entry(record.logical_path.clone())
            .or_insert(position);
        self.by_unit
            .entry(record.unit_id)
            .or_default()
            .push(position);
        self.records.push(record);
    }

    /// Look up a record by its `ArtifactId`.
    pub fn by_id(&self, id: ArtifactId) -> Option<&ArtifactRecord> {
        self.by_id.get(&id).map(|&i| &self.records[i])
    }

    /// Look up a record by its normalized logical path.
    pub fn by_path(&self, path: &str) -> Option<&ArtifactRecord> {
        self.by_path.get(path).map(|&i| &self.records[i])
    }

    /// All records belonging to `unit`, in insertion order.
    pub fn by_unit(&self, unit: UnitId) -> impl Iterator<Item = &ArtifactRecord> {
        self.by_unit
            .get(&unit)
            .into_iter()
            .flatten()
            .map(move |&i| &self.records[i])
    }

    /// All records in insertion order.
    pub fn records(&self) -> &[ArtifactRecord] {
        &self.records
    }

    /// The number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True when the index holds no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

// ─── Segment ↔ unit map ───────────────────────────────────────────────────────

/// A set-valued mapping between GTS segments and compilation units (S0.7).
///
/// GTS segmentation is a packaging optimization, **never** slice identity. The
/// relation is set-valued in *both* directions:
///
/// ```text
/// segment ↔ zero or more compilation units
/// unit    ↔ one or more segments
/// ```
///
/// The invariant `one segment == one slice` is rejected. This replaces the
/// single-valued `slice_iri` that a per-segment record cannot represent:
/// semantic provenance must survive concatenation, compaction, resegmentation, a
/// unit split across segments, and global generated segments owned by no unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SegmentUnitMap {
    /// segment index -> set of units (sorted, deduped).
    seg_to_units: HashMap<usize, Vec<UnitId>>,
    /// unit -> set of segment indices (sorted, deduped).
    unit_to_segs: HashMap<UnitId, Vec<usize>>,
}

impl SegmentUnitMap {
    /// A fresh, empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Associate `segment` with `unit`. Idempotent: a repeated `(segment, unit)`
    /// pair does not duplicate either side.
    pub fn associate(&mut self, segment: usize, unit: UnitId) {
        let units = self.seg_to_units.entry(segment).or_default();
        if let Err(pos) = units.binary_search(&unit) {
            units.insert(pos, unit);
        }
        let segs = self.unit_to_segs.entry(unit).or_default();
        if let Err(pos) = segs.binary_search(&segment) {
            segs.insert(pos, segment);
        }
    }

    /// The units associated with `segment` (sorted, deduped). Empty for a global
    /// generated segment owned by no unit.
    pub fn units_of_segment(&self, segment: usize) -> &[UnitId] {
        self.seg_to_units.get(&segment).map_or(&[], Vec::as_slice)
    }

    /// The segments a `unit` spans (sorted, deduped). A unit may span many
    /// segments (a slice split across the append log).
    pub fn segments_of_unit(&self, unit: UnitId) -> &[usize] {
        self.unit_to_segs.get(&unit).map_or(&[], Vec::as_slice)
    }

    /// True when no associations exist.
    pub fn is_empty(&self) -> bool {
        self.seg_to_units.is_empty()
    }

    /// The number of segments that have at least one associated unit.
    pub fn segment_count(&self) -> usize {
        self.seg_to_units.len()
    }
}

// ─── Bundle load error ────────────────────────────────────────────────────────

/// A hard error from [`RdfBundle::load`]. The loader never silently repairs;
/// every malformed structure is a typed `Err`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleError {
    /// A blob's bytes do not match the content id they are filed under.
    DigestMismatch {
        /// The id the bytes were filed under.
        stored: ContentDigest,
        /// The id the bytes actually hash to.
        actual: ContentDigest,
    },
    /// Two artifacts share one logical path.
    DuplicateLogicalPath {
        /// The colliding path.
        path: String,
    },
    /// Two artifact records share one [`ArtifactId`].
    DuplicateArtifactId {
        /// The colliding artifact id.
        id: ArtifactId,
    },
    /// An artifact's logical path is absolute (starts with `/`).
    AbsolutePath {
        /// The offending path.
        path: String,
    },
    /// An artifact's logical path contains a `..` traversal component.
    PathTraversal {
        /// The offending path.
        path: String,
    },
    /// Two units share an IRI but carry conflicting metadata.
    ConflictingManifest {
        /// The shared unit IRI.
        iri: String,
    },
    /// An artifact references a blob id that is absent from the content store.
    MissingBlob {
        /// The dangling content id.
        blob_id: ContentDigest,
        /// The artifact's logical path, for diagnosis.
        path: String,
    },
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DigestMismatch { stored, actual } => write!(
                f,
                "bundle blob digest mismatch: filed under {stored}, bytes hash to {actual}"
            ),
            Self::DuplicateLogicalPath { path } => {
                write!(f, "duplicate logical artifact path: {path}")
            }
            Self::DuplicateArtifactId { id } => {
                write!(f, "duplicate artifact id: {id}")
            }
            Self::AbsolutePath { path } => {
                write!(f, "absolute artifact path is not allowed: {path}")
            }
            Self::PathTraversal { path } => {
                write!(f, "`..` traversal in artifact path is not allowed: {path}")
            }
            Self::ConflictingManifest { iri } => {
                write!(f, "conflicting manifests for one unit IRI: {iri}")
            }
            Self::MissingBlob { blob_id, path } => write!(
                f,
                "artifact {path} references blob {blob_id} absent from the content store"
            ),
        }
    }
}

impl std::error::Error for BundleError {}

impl From<ContentStoreError> for BundleError {
    fn from(e: ContentStoreError) -> Self {
        match e {
            ContentStoreError::DigestMismatch { stored, actual } => {
                Self::DigestMismatch { stored, actual }
            }
        }
    }
}

// ─── RdfBundle ────────────────────────────────────────────────────────────────

/// The self-describing, repo-free bundle: dataset + provenance + unit catalog +
/// artifact index + the actual blob bytes.
///
/// Construct with [`RdfBundle::new`] then stage units / artifacts / blobs with the
/// builder methods, or recover a serialized bundle with [`RdfBundle::load`] (which
/// validates every digest and every structural invariant before returning `Ok`).
#[derive(Debug)]
pub struct RdfBundle {
    /// The immutable, value-interned RDF 1.2 dataset — the hot graph. Shared
    /// via `Arc` because `RdfDataset` is a frozen, non-clonable value (C1).
    pub dataset: Arc<RdfDataset>,
    /// Generic provenance sidecar (units / occurrences / origin sets).
    pub provenance: DatasetProvenance,
    /// `UnitId` -> unit metadata.
    pub units: UnitCatalog,
    /// `ArtifactId` -> artifact record (content-addressed, no inline bytes).
    pub artifacts: ArtifactIndex,
    /// The actual blob bytes, content-addressed.
    pub blobs: ContentStore,
    /// Set-valued segment ↔ unit packaging map (S0.7).
    pub segments: SegmentUnitMap,
}

impl RdfBundle {
    /// Begin a bundle from a frozen dataset and its provenance sidecar. Units,
    /// artifacts, blobs, and segment associations are added with the builder
    /// methods below.
    pub fn new(dataset: Arc<RdfDataset>, provenance: DatasetProvenance) -> Self {
        Self {
            dataset,
            provenance,
            units: UnitCatalog::new(),
            artifacts: ArtifactIndex::new(),
            blobs: ContentStore::new(),
            segments: SegmentUnitMap::new(),
        }
    }

    /// Record metadata for a unit (builder).
    pub fn add_unit(&mut self, unit: UnitId, meta: UnitMetadata) -> &mut Self {
        self.units.insert(unit, meta);
        self
    }

    /// Insert blob bytes into the content store, returning the content id
    /// (builder). The id is what an [`ArtifactRecord`] references.
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> ContentDigest {
        self.blobs.insert(bytes)
    }

    /// Author an artifact whose bytes are `content`: stores the bytes in the
    /// content store and indexes a content-addressed [`ArtifactRecord`]
    /// referencing them (builder). The bytes never travel inside the record.
    pub fn add_artifact(
        &mut self,
        artifact_id: ArtifactId,
        unit_id: UnitId,
        logical_path: impl Into<String>,
        role: impl Into<String>,
        content: Vec<u8>,
    ) -> ContentDigest {
        let blob_id = self.blobs.insert(content);
        self.artifacts.insert(ArtifactRecord {
            artifact_id,
            logical_path: logical_path.into(),
            role: role.into(),
            blob_id,
            unit_id,
        });
        blob_id
    }

    /// Associate a GTS segment with a unit (builder, set-valued).
    pub fn associate_segment(&mut self, segment: usize, unit: UnitId) -> &mut Self {
        self.segments.associate(segment, unit);
        self
    }

    /// Recover the exact bytes for an artifact by its logical path — repo-free.
    /// Returns `None` if there is no such artifact or its blob is absent.
    pub fn artifact_bytes(&self, logical_path: &str) -> Option<&Vec<u8>> {
        let record = self.artifacts.by_path(logical_path)?;
        self.blobs.get(&record.blob_id)
    }

    /// Validate every structural invariant of an already-assembled bundle, the
    /// same way [`load`](Self::load) does. Used after construction to assert the
    /// bundle is well-formed before serialization.
    ///
    /// # Errors
    ///
    /// The first [`BundleError`] found: digest mismatch, duplicate logical path,
    /// absolute path, `..` traversal, conflicting manifest, or a dangling blob.
    pub fn validate(&self) -> Result<(), BundleError> {
        // 1. Every stored blob hashes to its key.
        self.blobs.verify_all()?;

        // 2. Artifact paths: no duplicates, no absolute, no `..`; blob present;
        //    and no two records may share one `ArtifactId`.
        let mut seen: HashMap<&str, ()> = HashMap::new();
        let mut seen_ids: HashMap<ArtifactId, ()> = HashMap::new();
        for record in self.artifacts.records() {
            let path = record.logical_path.as_str();
            check_logical_path(path)?;
            if seen.insert(path, ()).is_some() {
                return Err(BundleError::DuplicateLogicalPath {
                    path: path.to_string(),
                });
            }
            if seen_ids.insert(record.artifact_id, ()).is_some() {
                return Err(BundleError::DuplicateArtifactId {
                    id: record.artifact_id,
                });
            }
            if !self.blobs.contains(&record.blob_id) {
                return Err(BundleError::MissingBlob {
                    blob_id: record.blob_id,
                    path: path.to_string(),
                });
            }
        }

        // 3. No two units may share an IRI with conflicting metadata.
        let mut by_iri: HashMap<&str, &UnitMetadata> = HashMap::new();
        for (_, meta) in self.units.iter() {
            match by_iri.get(meta.iri.as_str()) {
                Some(existing) if *existing != meta => {
                    return Err(BundleError::ConflictingManifest {
                        iri: meta.iri.clone(),
                    });
                }
                _ => {
                    by_iri.insert(meta.iri.as_str(), meta);
                }
            }
        }

        Ok(())
    }

    /// Load a bundle from its parts, validating every content digest and every
    /// structural invariant. This is the hard-fail recovery path: a serialized
    /// bundle is reconstructed into these parts, then handed here.
    ///
    /// Rejects (never repairs): a content-digest mismatch, a duplicate logical
    /// artifact path, an absolute path, a `..` traversal component, conflicting
    /// manifests for one unit IRI, and a dangling blob reference.
    ///
    /// # Errors
    ///
    /// The first [`BundleError`] encountered.
    pub fn load(
        dataset: Arc<RdfDataset>,
        provenance: DatasetProvenance,
        units: UnitCatalog,
        artifacts: ArtifactIndex,
        blobs: ContentStore,
        segments: SegmentUnitMap,
    ) -> Result<Self, BundleError> {
        let bundle = Self {
            dataset,
            provenance,
            units,
            artifacts,
            blobs,
            segments,
        };
        bundle.validate()?;
        Ok(bundle)
    }
}

/// Reject an absolute path or a `..` traversal component.
fn check_logical_path(path: &str) -> Result<(), BundleError> {
    if path.starts_with('/') {
        return Err(BundleError::AbsolutePath {
            path: path.to_string(),
        });
    }
    if path.split('/').any(|component| component == "..") {
        return Err(BundleError::PathTraversal {
            path: path.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::provenance::OriginKind;

    /// A tiny frozen dataset with one triple — hermetic, no `slices/` tree.
    fn tiny_dataset() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None);
        b.freeze().expect("valid dataset")
    }

    /// Build a small, fully-authored bundle in-test.
    fn sample_bundle() -> RdfBundle {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("slices/core/sample", OriginKind::Source);
        let art_mod = prov.register_artifact("module.ttl");
        let art_doc = prov.register_artifact("docs.md");

        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        bundle.add_unit(
            unit,
            UnitMetadata::new("https://example.org/slice/sample", "Sample slice"),
        );
        bundle.add_artifact(
            art_mod,
            unit,
            "module.ttl",
            "Module",
            b"@prefix ex: <http://example.org/> .".to_vec(),
        );
        bundle.add_artifact(
            art_doc,
            unit,
            "docs.md",
            "Documentation",
            b"# Sample".to_vec(),
        );
        bundle.associate_segment(0, unit);
        bundle
    }

    // ── Headline S3 test: resegmentation invariance ──────────────────────────

    #[test]
    fn resegmentation_invariance() {
        // Build a bundle, then re-package its GTS segmentation differently
        // (split the single unit across two segments instead of one). Slice
        // identity (unit metadata), artifacts, provenance, and the RDF dataset
        // semantics must all be UNCHANGED — only `segments` differs.
        let original = sample_bundle();

        // A second "resegmented" view: same dataset/provenance/artifacts/units,
        // but the unit now spans segments 0 AND 1 (and segment 1 also carries no
        // exclusive unit — a packaging choice).
        let mut resegmented = sample_bundle();
        let unit = resegmented.provenance.units.intern("slices/core/sample"); // idempotent: existing id
        resegmented.segments = SegmentUnitMap::new();
        resegmented.associate_segment(0, unit);
        resegmented.associate_segment(1, unit);

        // Segmentation differs.
        assert_ne!(
            original.segments.segment_count(),
            resegmented.segments.segment_count(),
            "the two packagings segment differently"
        );

        // Everything semantic is identical.
        assert!(
            datasets_equal(&original.dataset, &resegmented.dataset),
            "dataset semantics unchanged by resegmentation"
        );
        assert_eq!(
            original.artifacts, resegmented.artifacts,
            "artifacts unchanged by resegmentation"
        );
        assert_eq!(
            original.units, resegmented.units,
            "unit catalog (slice identity) unchanged by resegmentation"
        );
        assert_eq!(
            original.provenance.occurrences, resegmented.provenance.occurrences,
            "provenance occurrences unchanged by resegmentation"
        );
        // Both packagings still pass the structural gate.
        assert!(original.validate().is_ok());
        assert!(resegmented.validate().is_ok());
    }

    fn datasets_equal(a: &RdfDataset, b: &RdfDataset) -> bool {
        a.quad_count() == b.quad_count() && a.term_count() == b.term_count()
    }

    // ── Repo-free recoverability ─────────────────────────────────────────────

    #[test]
    fn repo_free_recoverability_by_role_path_and_digest() {
        let bundle = sample_bundle();
        // Every authored artifact must be recoverable with EXACT bytes purely
        // from role + logical_path + digest — no filesystem, no repo.
        let expected: &[(&str, &str, &[u8])] = &[
            (
                "Module",
                "module.ttl",
                b"@prefix ex: <http://example.org/> .",
            ),
            ("Documentation", "docs.md", b"# Sample"),
        ];
        for (role, path, bytes) in expected {
            let record = bundle
                .artifacts
                .by_path(path)
                .expect("artifact present by logical path");
            assert_eq!(&record.role, role, "role matches");
            // Digest reference resolves to the exact bytes.
            assert_eq!(
                record.blob_id,
                ContentDigest::of(bytes),
                "digest matches bytes"
            );
            let recovered = bundle
                .blobs
                .get(&record.blob_id)
                .expect("blob recoverable by digest");
            assert_eq!(recovered.as_slice(), *bytes, "exact bytes recovered");
            // Convenience accessor round-trips too.
            assert_eq!(bundle.artifact_bytes(path).map(Vec::as_slice), Some(*bytes));
        }
    }

    // ── Five rejection tests (one assertion each) ────────────────────────────

    #[test]
    fn reject_digest_mismatch() {
        // A blob mis-filed under the wrong content id must hard-fail: the store's
        // `insert_checked` rejects it, and that maps to a bundle `DigestMismatch`.
        let mut store = ContentStore::new();
        let wrong_id = ContentDigest::of(b"the original bytes");
        let err = store
            .insert_checked(wrong_id, b"tampered bytes".to_vec())
            .unwrap_err();
        let bundle_err: BundleError = err.into();
        assert!(matches!(bundle_err, BundleError::DigestMismatch { .. }));
    }

    #[test]
    fn reject_duplicate_logical_paths() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("u", OriginKind::Source);
        let a0 = prov.register_artifact("a");
        let a1 = prov.register_artifact("b");
        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        bundle.add_unit(unit, UnitMetadata::new("iri", "u"));
        bundle.add_artifact(a0, unit, "same.ttl", "Module", b"x".to_vec());
        bundle.add_artifact(a1, unit, "same.ttl", "Shapes", b"y".to_vec());
        assert!(matches!(
            bundle.validate(),
            Err(BundleError::DuplicateLogicalPath { .. })
        ));
    }

    #[test]
    fn reject_duplicate_artifact_ids() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("u", OriginKind::Source);
        let a0 = prov.register_artifact("a");
        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        bundle.add_unit(unit, UnitMetadata::new("iri", "u"));
        // Two records with DISTINCT logical paths but the SAME ArtifactId.
        bundle.add_artifact(a0, unit, "first.ttl", "Module", b"x".to_vec());
        bundle.add_artifact(a0, unit, "second.ttl", "Shapes", b"y".to_vec());
        assert!(matches!(
            bundle.validate(),
            Err(BundleError::DuplicateArtifactId { .. })
        ));
    }

    #[test]
    fn reject_absolute_path() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("u", OriginKind::Source);
        let a0 = prov.register_artifact("a");
        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        bundle.add_unit(unit, UnitMetadata::new("iri", "u"));
        bundle.add_artifact(a0, unit, "/foo/bar", "Module", b"x".to_vec());
        assert!(matches!(
            bundle.validate(),
            Err(BundleError::AbsolutePath { .. })
        ));
    }

    #[test]
    fn reject_path_traversal() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("u", OriginKind::Source);
        let a0 = prov.register_artifact("a");
        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        bundle.add_unit(unit, UnitMetadata::new("iri", "u"));
        bundle.add_artifact(a0, unit, "foo/../bar", "Module", b"x".to_vec());
        assert!(matches!(
            bundle.validate(),
            Err(BundleError::PathTraversal { .. })
        ));
    }

    #[test]
    fn reject_conflicting_manifests_for_one_unit_iri() {
        let mut prov = DatasetProvenance::new();
        let u0 = prov.register_unit("u0", OriginKind::Source);
        let u1 = prov.register_unit("u1", OriginKind::Source);
        let mut bundle = RdfBundle::new(tiny_dataset(), prov);
        // Two distinct units claim the SAME IRI but disagree on `name`.
        bundle.add_unit(u0, UnitMetadata::new("https://x/slice", "name A"));
        bundle.add_unit(u1, UnitMetadata::new("https://x/slice", "name B"));
        assert!(matches!(
            bundle.validate(),
            Err(BundleError::ConflictingManifest { .. })
        ));
    }

    // ── load() round-trips a well-formed bundle ──────────────────────────────

    #[test]
    fn load_accepts_well_formed_bundle() {
        let bundle = sample_bundle();
        let RdfBundle {
            dataset,
            provenance,
            units,
            artifacts,
            blobs,
            segments,
        } = bundle;
        let loaded = RdfBundle::load(dataset, provenance, units, artifacts, blobs, segments);
        assert!(
            loaded.is_ok(),
            "well-formed bundle loads: {:?}",
            loaded.err()
        );
    }
}

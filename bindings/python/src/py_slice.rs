// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PyO3 Python bindings for `purrdf-slice` (#820 S8).
//!
//! # Engine core separation
//!
//! Only this binding file imports pyo3. The engine modules in `purrdf-slice` are
//! PyO3-free so the rlib links
//! into the future Rust compiler without any Python dependency; the `python`
//! feature gates this module.
//!
//! # What is exposed
//!
//! This binding makes the native slice machinery the authoritative slice
//! catalog/analyzer for the Python tooling (#820 S8), retiring the redundant
//! Python `slice_ownership_lint` / `module_specs` plumbing:
//!
//! * [`PySliceCatalog`] — manifest-based discovery (`SliceCatalog::discover`),
//!   exposing per-slice records + typed artifact inventory.
//! * [`PyOwnershipAnalyzer`] — the S4 ownership + dependency analyzer over a
//!   catalog. Its [`PyOwnershipReport`] carries the computed dependency edges
//!   (with reconciliation status) and the ownership diagnostics that replace the
//!   path-derived `slice_ownership_lint`.
//! * [`PyOwnershipReport::ownership_errors`] — the ownership-defect findings as
//!   plain error strings, the same diagnostics the retired lint produced (but
//!   physical-origin based, not directory-name derived).

use std::collections::HashMap;
use std::path::Path;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use purrdf_slice::analysis::emit_analysis_graph;
use purrdf_slice::artifact::{ArtifactRecord, ArtifactRole};
use purrdf_slice::cache::ToolchainContext;
use purrdf_slice::catalog::{ManifestView, SliceCatalog, SliceRecord, SliceTier};
use purrdf_slice::fix_deps::{compute_fix_deps, ManifestPatch};
use purrdf_slice::ownership::{
    DependencyEdge, OwnershipAnalyzer, OwnershipDiagnostic, OwnershipReport, OwnershipStatus,
    ReconciliationStatus, SliceIri,
};
use purrdf_slice::vocab::SliceVocab;

/// The stable lowercase token a [`ReconciliationStatus`] is exposed as.
fn reconciliation_token(status: ReconciliationStatus) -> &'static str {
    match status {
        ReconciliationStatus::Matched => "matched",
        ReconciliationStatus::Undeclared => "undeclared",
        ReconciliationStatus::Stale => "stale",
        ReconciliationStatus::Forbidden => "forbidden",
    }
}

/// The stable string name an [`ArtifactRole`] is exposed as (matches the Rust
/// variant name so Python can compare against `"Manifest"`, `"Module"`, …).
fn artifact_role_name(role: &ArtifactRole) -> String {
    match role {
        ArtifactRole::Manifest => "Manifest".to_owned(),
        ArtifactRole::Module => "Module".to_owned(),
        ArtifactRole::Shapes => "Shapes".to_owned(),
        ArtifactRole::Mapping => "Mapping".to_owned(),
        ArtifactRole::CompetencyQuery => "CompetencyQuery".to_owned(),
        ArtifactRole::VerifyQuery => "VerifyQuery".to_owned(),
        ArtifactRole::TestDsl => "TestDsl".to_owned(),
        ArtifactRole::Example => "Example".to_owned(),
        ArtifactRole::CounterExample => "CounterExample".to_owned(),
        ArtifactRole::Documentation => "Documentation".to_owned(),
        ArtifactRole::TranslationCatalog => "TranslationCatalog".to_owned(),
        ArtifactRole::Citation => "Citation".to_owned(),
        ArtifactRole::Other(iri) => format!("Other({iri})"),
    }
}

/// The stable string name a [`SliceTier`] is exposed as: `"core"` / `"extension"`
/// / `"domain"` / `"unknown"` (matching the Python `Slice.tier` contract for
/// core/extension).
fn tier_token(tier: &SliceTier) -> String {
    match tier {
        SliceTier::Core => "core".to_owned(),
        SliceTier::Extension => "extension".to_owned(),
        SliceTier::Domain => "domain".to_owned(),
        SliceTier::Unknown(_) => "unknown".to_owned(),
    }
}

// ── ArtifactRecord ─────────────────────────────────────────────────────────────

/// A single packaged artifact within a slice (role + logical path + digests).
#[pyclass(name = "ArtifactRecord", module = "purrdf_slice", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyArtifactRecord {
    inner: ArtifactRecord,
}

#[pymethods]
impl PyArtifactRecord {
    /// The artifact role as a string (`"Manifest"`, `"Module"`, …).
    #[getter]
    fn role(&self) -> String {
        artifact_role_name(&self.inner.role)
    }

    /// The normalized, slice-relative logical path.
    #[getter]
    fn logical_path(&self) -> String {
        self.inner.logical_path.clone()
    }

    /// The media type (MIME).
    #[getter]
    fn media_type(&self) -> String {
        self.inner.media_type.clone()
    }

    /// SHA-256 hex digest of the raw bytes.
    #[getter]
    fn raw_digest(&self) -> String {
        self.inner.raw_digest.clone()
    }

    /// SHA-256 hex of the canonical N-Triples (RDF artifacts only).
    #[getter]
    fn semantic_digest(&self) -> Option<String> {
        self.inner.semantic_digest.clone()
    }

    /// The raw artifact bytes (content cache). Exposed so the GTS producer can
    /// fold each ontology artifact into the self-describing S3 bundle as a
    /// content-addressed blob without a second disk read (#820 S3).
    #[getter]
    fn content<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, &self.inner.content)
    }
}

// ── ManifestView ───────────────────────────────────────────────────────────────

/// The validated, typed projection of one slice's `manifest.ttl`.
#[pyclass(name = "ManifestView", module = "purrdf_slice", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyManifestView {
    inner: ManifestView,
}

#[pymethods]
impl PyManifestView {
    /// The slice IRI (sole identity).
    #[getter]
    fn slice_iri(&self) -> String {
        self.inner.slice_iri.clone()
    }

    /// `rdfs:label` (lexical form), if present.
    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    /// `dcterms:title` (lexical form), if present.
    #[getter]
    fn title(&self) -> Option<String> {
        self.inner.title.clone()
    }

    /// `dcterms:creator` token literals.
    #[getter]
    fn creators(&self) -> Vec<String> {
        self.inner.creators.clone()
    }

    /// `dcterms:identifier`, if present.
    #[getter]
    fn identifier(&self) -> Option<String> {
        self.inner.identifier.clone()
    }

    /// The tier token (`"core"` / `"extension"` / `"domain"` / `"unknown"`),
    /// or `None` when the manifest declares no tier.
    #[getter]
    fn tier(&self) -> Option<String> {
        self.inner.tier.as_ref().map(tier_token)
    }

    /// `purrdf:sliceConsumer` prose literals.
    #[getter]
    fn consumers(&self) -> Vec<String> {
        self.inner.consumers.clone()
    }
}

// ── SliceRecord ────────────────────────────────────────────────────────────────

/// One discovered slice: its manifest view plus its typed artifact inventory.
#[pyclass(name = "SliceRecord", module = "purrdf_slice", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PySliceRecord {
    manifest: PyManifestView,
    artifacts: Vec<PyArtifactRecord>,
    slice_dir: String,
    manifest_path: String,
}

impl PySliceRecord {
    fn from_record(rec: &SliceRecord) -> Self {
        Self {
            manifest: PyManifestView {
                inner: rec.manifest.clone(),
            },
            artifacts: rec
                .artifacts
                .iter()
                .map(|a| PyArtifactRecord { inner: a.clone() })
                .collect(),
            slice_dir: rec.slice_dir.to_string_lossy().to_string(),
            manifest_path: rec.manifest_path().to_string_lossy().to_string(),
        }
    }
}

#[pymethods]
impl PySliceRecord {
    /// The typed manifest projection.
    #[getter]
    fn manifest(&self) -> PyManifestView {
        self.manifest.clone()
    }

    /// The typed artifact inventory.
    #[getter]
    fn artifacts(&self) -> Vec<PyArtifactRecord> {
        self.artifacts.clone()
    }

    /// The on-disk slice directory this record was discovered from. Authoritative
    /// — no scan or substring match is needed to locate the slice's files (#820 G8).
    #[getter]
    fn slice_dir(&self) -> String {
        self.slice_dir.clone()
    }

    /// The on-disk path to this slice's `manifest.ttl` (= `slice_dir`/manifest.ttl).
    #[getter]
    fn manifest_path(&self) -> String {
        self.manifest_path.clone()
    }
}

// ── SliceCatalog ───────────────────────────────────────────────────────────────

/// The native slice catalog: manifest-based discovery of every slice under a
/// root directory, the authoritative slice machinery (#820 S8).
#[pyclass(name = "SliceCatalog", module = "purrdf_slice")]
#[derive(Debug)]
pub struct PySliceCatalog {
    inner: SliceCatalog,
    /// The caller-supplied slice vocabulary (ontology namespace); captured at
    /// discovery so every downstream consumer (ownership analysis, emitters)
    /// uses the same vocabulary by construction.
    vocab: SliceVocab,
}

#[pymethods]
impl PySliceCatalog {
    /// Discover every slice under `root` (globs `*/*/manifest.ttl`), parsing
    /// each manifest and inventorying its artifacts.
    ///
    /// `namespace` is the ontology namespace the slice manifests use for the
    /// slice vocabulary (`{namespace}sliceDependsOn`, `{namespace}Slice`, …) —
    /// e.g. `"https://blackcatinformatics.ca/gmeow/"`.
    #[staticmethod]
    fn discover(py: Python<'_>, root: &str, namespace: &str) -> PyResult<Self> {
        // Manifest discovery (disk walk + Turtle parses) runs detached (GIL released).
        py.detach(|| {
            let vocab = SliceVocab::for_namespace(namespace);
            let catalog = SliceCatalog::discover(Path::new(root), vocab.clone()).map_err(|e| {
                PyValueError::new_err(format!("slice catalog discovery failed: {e}"))
            })?;
            Ok(Self {
                inner: catalog,
                vocab,
            })
        })
    }

    /// Every discovered slice record (sorted by IRI).
    fn records(&self) -> Vec<PySliceRecord> {
        self.inner
            .records()
            .iter()
            .map(PySliceRecord::from_record)
            .collect()
    }

    /// The slice IRIs of every core-tier slice (for lint-config construction).
    fn core_slice_iris(&self) -> Vec<String> {
        self.inner
            .records()
            .iter()
            .filter(|r| matches!(r.manifest.tier, Some(SliceTier::Core)))
            .map(|r| r.manifest.slice_iri.clone())
            .collect()
    }

    /// Compute the RDF-aware `purrdf:sliceDependsOn` reconciliation patches for
    /// every manifest with undeclared (add) or stale (remove) semantic edges
    /// (#820 G8). Each result carries the manifest path, original text, and the
    /// surgically-patched, re-parse-validated text. Hard-fails (`ValueError`) on
    /// any manifest read/parse error or post-patch validation failure — no silent
    /// skips, no wrong-manifest matching, no malformed Turtle.
    fn fix_deps(&self, py: Python<'_>) -> PyResult<Vec<PyManifestPatch>> {
        // The RDF-aware reconciliation (parse + patch + re-validate every
        // manifest) runs detached (GIL released).
        let catalog = &self.inner;
        let patches = py
            .detach(|| compute_fix_deps(catalog))
            .map_err(|e| PyValueError::new_err(format!("slice fix-deps failed: {e}")))?;
        Ok(patches
            .into_iter()
            .map(|p| PyManifestPatch { inner: p })
            .collect())
    }
}

// ── ManifestPatch ──────────────────────────────────────────────────────────────

/// One computed manifest patch: the on-disk path plus original and patched text.
#[pyclass(name = "ManifestPatch", module = "purrdf_slice", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyManifestPatch {
    inner: ManifestPatch,
}

#[pymethods]
impl PyManifestPatch {
    /// The on-disk path to the patched `manifest.ttl`.
    #[getter]
    fn manifest_path(&self) -> String {
        self.inner.manifest_path.clone()
    }

    /// The original (authored) manifest Turtle text.
    #[getter]
    fn original_text(&self) -> String {
        self.inner.original_text.clone()
    }

    /// The patched manifest Turtle text (well-formed, re-parse validated).
    #[getter]
    fn patched_text(&self) -> String {
        self.inner.patched_text.clone()
    }
}

// ── DependencyEdge ─────────────────────────────────────────────────────────────

/// One computed cross-slice dependency edge with its reconciliation verdict.
#[pyclass(name = "DependencyEdge", module = "purrdf_slice", skip_from_py_object)]
#[derive(Clone, Debug)]
pub struct PyDependencyEdge {
    from_slice: String,
    to_slice: String,
    reconciliation: &'static str,
    is_semantic: bool,
}

impl PyDependencyEdge {
    fn from_edge(edge: &DependencyEdge) -> Self {
        Self {
            from_slice: edge.from_slice.clone(),
            to_slice: edge.to_slice.clone(),
            reconciliation: reconciliation_token(edge.reconciliation),
            is_semantic: edge.edge_kind.is_semantic(),
        }
    }
}

#[pymethods]
impl PyDependencyEdge {
    /// The depending slice IRI. (Exposed to Python as the `from_slice`
    /// attribute; the Rust method is renamed to dodge `wrong_self_convention`,
    /// since a `from_*` getter taking `&self` is otherwise flagged.)
    #[getter(from_slice)]
    fn get_from_slice(&self) -> String {
        self.from_slice.clone()
    }

    /// The depended-upon slice IRI.
    #[getter]
    fn to_slice(&self) -> String {
        self.to_slice.clone()
    }

    /// The reconciliation token: `"matched"`, `"undeclared"`, `"stale"`, or
    /// `"forbidden"`.
    #[getter]
    fn reconciliation(&self) -> &'static str {
        self.reconciliation
    }

    /// Whether this edge reconciles against `purrdf:sliceDependsOn` (a semantic
    /// ontology/shape/mapping/query reference, not a doc/test/example link).
    #[getter]
    fn is_semantic(&self) -> bool {
        self.is_semantic
    }
}

// ── OwnershipReport ────────────────────────────────────────────────────────────

/// Format one ownership diagnostic as a single error string, equivalent to the
/// retired path-derived `slice_ownership_lint` finding (#329) but derived from
/// the term's *physical origin*, not its directory name.
fn ownership_error_string(diag: &OwnershipDiagnostic) -> Option<String> {
    match diag {
        OwnershipDiagnostic::Conflict { term, claimants } => Some(format!(
            "{term} rdfs:isDefinedBy is claimed by multiple slices {claimants:?} — \
             a term must have exactly one owning slice (#329)",
            term = term.as_str(),
        )),
        OwnershipDiagnostic::Mismatch {
            term,
            declared,
            physical,
        } => Some(format!(
            "{term} rdfs:isDefinedBy {declared} — must equal the owning slice IRI \
             {physical} (#329)",
            term = term.as_str(),
        )),
        // Undeclared / stale dependencies and unparsable queries are dependency
        // *findings*, not ownership defects: they are not lint errors here (the
        // retired lint only flagged ownership equality).
        _ => None,
    }
}

/// The complete result of an ownership + dependency analysis.
#[pyclass(name = "OwnershipReport", module = "purrdf_slice")]
#[derive(Debug)]
pub struct PyOwnershipReport {
    inner: OwnershipReport,
}

#[pymethods]
impl PyOwnershipReport {
    /// Every computed cross-slice dependency edge (sorted, deterministic).
    #[getter]
    fn edges(&self) -> Vec<PyDependencyEdge> {
        self.inner
            .edges
            .iter()
            .map(PyDependencyEdge::from_edge)
            .collect()
    }

    /// The ownership-defect findings as plain error strings — the same
    /// diagnostics the retired `slice_ownership_lint` produced (a term whose
    /// `rdfs:isDefinedBy` does not equal its true owning slice, or is claimed by
    /// multiple slices). Sorted for determinism.
    fn ownership_errors(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // Unowned terms (an isDefinedBy with no physical origin) are surfaced
        // via the per-term status table.
        for owner in self.inner.ownership.values() {
            if owner.status == OwnershipStatus::Unowned {
                out.push(format!(
                    "{term} rdfs:isDefinedBy {declared} — no slice physically \
                     defines this term (#329)",
                    term = owner.term.as_str(),
                    declared = owner.declared_owner,
                ));
            }
        }
        for diag in &self.inner.diagnostics {
            if let Some(msg) = ownership_error_string(diag) {
                out.push(msg);
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Whether the analysis found any ownership defect (conflict / mismatch /
    /// unowned term).
    fn has_ownership_defect(&self) -> bool {
        self.inner.has_ownership_defect()
    }
}

// ── OwnershipAnalyzer ──────────────────────────────────────────────────────────

/// Map a [`SliceTier`] to the numeric tier the analysis-graph emitter expects:
/// `0` = core, `1` = extension, `2` = domain / unknown (RFC §10 / Principle 16).
/// Slices absent from the tier map (no `purrdf:sliceTier` in the manifest) are
/// treated as `2` (unknown) — they never trip the core→ext / ext→ext forbidden
/// edge rule, matching the emitter's documented contract.
fn tier_priority(tier: Option<&SliceTier>) -> u8 {
    match tier {
        Some(SliceTier::Core) => 0,
        Some(SliceTier::Extension) => 1,
        Some(SliceTier::Domain | SliceTier::Unknown(_)) | None => 2,
    }
}

/// The native ownership + dependency analyzer over a [`PySliceCatalog`].
#[pyclass(name = "OwnershipAnalyzer", module = "purrdf_slice")]
#[derive(Debug)]
pub struct PyOwnershipAnalyzer {
    report: OwnershipReport,
    /// Per-slice numeric tier, resolved once from the catalog manifests, so the
    /// analysis-graph emitter's `tier_of` closure is a pure lookup (the emitter
    /// module stays PyO3-free; tier resolution happens here).
    tier_of: HashMap<SliceIri, u8>,
    /// The slice vocabulary inherited from the catalog at construction.
    vocab: SliceVocab,
    /// Every authored artifact raw digest in the catalog (drives the analysis
    /// graph's bundle content-ID). Sorted for stable iteration.
    raw_digests: Vec<String>,
}

#[pymethods]
impl PyOwnershipAnalyzer {
    /// Build the analyzer for a catalog and immediately compute the report
    /// (the analysis borrows the catalog, so it is run eagerly at construction
    /// and the owned report is retained). The per-slice tier map and the set of
    /// all authored artifact raw digests are captured here too, so the
    /// PyO3-free analysis-graph emitter can be driven entirely from owned data.
    #[new]
    fn new(py: Python<'_>, catalog: &PySliceCatalog) -> PyResult<Self> {
        // The eager ownership + dependency analysis runs detached (GIL released).
        py.detach(|| {
            let report = OwnershipAnalyzer::new(&catalog.inner)
                .analyze()
                .map_err(|e| PyValueError::new_err(format!("ownership analysis failed: {e}")))?;
            let mut tier_of: HashMap<SliceIri, u8> = HashMap::new();
            let mut raw_digests: Vec<String> = Vec::new();
            for record in catalog.inner.records() {
                tier_of.insert(
                    record.manifest.slice_iri.clone(),
                    tier_priority(record.manifest.tier.as_ref()),
                );
                for artifact in &record.artifacts {
                    raw_digests.push(artifact.raw_digest.clone());
                }
            }
            raw_digests.sort_unstable();
            Ok(Self {
                report,
                tier_of,
                vocab: catalog.vocab.clone(),
                raw_digests,
            })
        })
    }

    /// Return the computed [`PyOwnershipReport`].
    fn analyze(&self) -> PyOwnershipReport {
        PyOwnershipReport {
            inner: self.report.clone(),
        }
    }

    /// Emit the computed `purrdf:graph/slice-analysis` named graph as a Turtle
    /// body string (#820 S7, gap G5).
    ///
    /// This is the production consumer of `purrdf_slice::analysis::emit_analysis_graph`:
    /// it builds the `tier_of` / `term_count_of` closures from the analyzer's own
    /// owned state (the catalog tier map captured at construction, and the
    /// validated ownership table from the computed report), passes every authored
    /// artifact raw digest as the bundle-content-ID inputs, and stamps the
    /// supplied toolchain provenance.
    ///
    /// `authored_input_text` is the serialized authored base graph; the emitter's
    /// self-attestation guard hard-fails if it contains the analysis graph IRI
    /// (the analysis graph must never be consumed as its own input).
    ///
    /// # Errors
    ///
    /// Raises `ValueError` on any `purrdf_slice::analysis::AnalysisError` (notably the
    /// self-attestation guard violation) — never returns an empty string.
    fn analysis_graph_turtle(
        &self,
        py: Python<'_>,
        authored_input_text: &str,
        compiler_version: &str,
        reasoning_profile: &str,
    ) -> PyResult<String> {
        // The graph emission (including the self-attestation scan over the
        // authored input text) runs detached (GIL released).
        py.detach(|| {
            self.analysis_graph_turtle_core(
                authored_input_text,
                compiler_version,
                reasoning_profile,
            )
        })
    }
}

impl PyOwnershipAnalyzer {
    /// The pure-Rust core of [`Self::analysis_graph_turtle`]; runs without the GIL.
    fn analysis_graph_turtle_core(
        &self,
        authored_input_text: &str,
        compiler_version: &str,
        reasoning_profile: &str,
    ) -> PyResult<String> {
        let toolchain = ToolchainContext::new(compiler_version, reasoning_profile);
        let digests: Vec<&str> = self.raw_digests.iter().map(String::as_str).collect();

        // `term_count_of`: the number of VALIDATED vocabulary terms owned by the
        // slice (the authoritative term-coverage count — a conflicted / mismatched
        // / unowned term is not a clean owned term).
        let term_count_of = |slice: &SliceIri| -> usize {
            self.report
                .ownership
                .values()
                .filter(|o| {
                    matches!(o.status, OwnershipStatus::Validated) && &o.declared_owner == slice
                })
                .count()
        };

        // `tier_of`: pure lookup into the captured manifest tier map. Slices not
        // in the map (none, in practice — every edge endpoint is a discovered
        // slice) default to 2 (unknown), which is never forbidden.
        let tier_of = |slice: &SliceIri| -> u8 { self.tier_of.get(slice).copied().unwrap_or(2) };

        let graph = emit_analysis_graph(
            &self.vocab,
            &self.report.edges,
            authored_input_text,
            &digests,
            &toolchain,
            tier_of,
            term_count_of,
        )
        .map_err(|e| PyValueError::new_err(format!("slice-analysis graph emission failed: {e}")))?;
        Ok(graph.turtle_body)
    }
}

/// Register the `purrdf_slice` engine surface into the given module.
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyArtifactRecord>()?;
    m.add_class::<PyManifestView>()?;
    m.add_class::<PySliceRecord>()?;
    m.add_class::<PySliceCatalog>()?;
    m.add_class::<PyManifestPatch>()?;
    m.add_class::<PyDependencyEdge>()?;
    m.add_class::<PyOwnershipReport>()?;
    m.add_class::<PyOwnershipAnalyzer>()?;
    Ok(())
}

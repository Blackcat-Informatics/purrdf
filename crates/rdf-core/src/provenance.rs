// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generic provenance sidecar for the immutable RDF 1.2 dataset (#820 S2).
//!
//! This module realizes the normative S0 provenance contract from
//! `docs/design/820-slices-first-class.md`. The types here are **kernel-generic**:
//! there is **no** `SliceId` or any PURRDF-specific concept in this module. The
//! slice layer (`purrdf-slice`) is responsible for interpreting unit kinds.
//!
//! ## Core types
//!
//! - [`UnitId`] — opaque id for a compilation/source unit.
//! - [`ArtifactId`] — opaque id for a packaged artifact within a unit.
//! - [`OriginSetId`] — opaque id for an interned set of origins.
//!
//! ## Set-valued invariant
//!
//! The canonical dataset is set-valued: two identical quads authored by two
//! different artifacts collapse to **one** `QuadHandle`. To preserve both origins,
//! each physical assertion produces one [`AssertionOccurrence`]. A quad therefore
//! has **one** row in the dataset but potentially **many** `AssertionOccurrence`
//! entries — one per asserting `(unit, artifact)` pair.
//!
//! ## No-optionality / hard-fail
//!
//! A missing or unknown origin is an `Err`, never a default. The [`OriginKind`]
//! enum has no `Unknown` variant: the caller must supply a concrete kind or the
//! provenance gate fails.

use std::collections::HashMap;
use std::fmt;

use crate::ir::QuadHandle;

// ─── Opaque ID newtypes ───────────────────────────────────────────────────────

/// Opaque id for a compilation/source unit (file set, import, generated graph,
/// or runtime data input). Runtime-only: MUST NOT enter persistent serialization,
/// cache keys, or derivation hashes (S0.5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct UnitId(u32);

impl UnitId {
    /// The dense index this id addresses in the unit interner.
    ///
    /// Crate-internal: external code may not forge or compare `UnitId`s across
    /// provenance sidecars.
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// Construct a `UnitId` from a dense table index. Only the interner mints
    /// ids, in allocation order.
    pub(crate) fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unit#{}", self.0)
    }
}

/// Opaque id for a packaged artifact within a unit (module file, shapes file,
/// mapping, query, …). Runtime-only (S0.5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ArtifactId(u32);

impl ArtifactId {
    /// The dense index this id addresses in the artifact interner.
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// Construct an `ArtifactId` from a dense table index.
    pub(crate) fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "artifact#{}", self.0)
    }
}

/// Opaque id for an interned set of origins. Two quads with the same set of
/// `(UnitId, ArtifactId)` pairs share an `OriginSetId`. Runtime-only (S0.5).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct OriginSetId(u32);

impl OriginSetId {
    /// The dense index this id addresses in the origin-set interner.
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    /// Construct an `OriginSetId` from a dense table index.
    pub(crate) fn from_index(index: u32) -> Self {
        Self(index)
    }
}

impl fmt::Display for OriginSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "origin-set#{}", self.0)
    }
}

// ─── Origin kind ─────────────────────────────────────────────────────────────

/// The kind of a compilation unit. Generic — **no** `SliceId` here; the
/// `purrdf-slice` layer interprets `Slice`-kind units by wrapping `UnitId`.
///
/// There is deliberately **no** `Unknown` variant: an unattributable origin is a
/// hard failure (no-optionality / hard-fail, S0.2).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum OriginKind {
    /// A source artifact authored directly (e.g. a Turtle module file belonging
    /// to a slice). The slice layer annotates these with a `SliceId`.
    Source,
    /// The root/top-level ontology file (e.g. `ontology/purrdf.ttl`).
    RootOntology,
    /// An OWL/RDF import loaded transitively from `owl:imports`.
    Import,
    /// A quad emitted by a code generator (e.g. the crossref, GTS producer,
    /// statement compiler, or derived reasoning output).
    Generated,
    /// Runtime data provided as input (e.g. an external graph injected at
    /// validation time or a test fixture graph).
    RuntimeInput,
}

impl fmt::Display for OriginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Source => "source",
            Self::RootOntology => "root-ontology",
            Self::Import => "import",
            Self::Generated => "generated",
            Self::RuntimeInput => "runtime-input",
        })
    }
}

// ─── Attribution types ────────────────────────────────────────────────────────

/// The role of a compilation unit in a structured attribution (S0.3 / §9).
///
/// A single diagnostic or SHACL result can involve multiple compilation units
/// in **different** roles: the shape owner is distinct from the data origin, the
/// rule owner is distinct from the focus node origin, and so on. A scalar
/// `SliceId` field cannot represent this; `AttributionRole` keeps the roles
/// distinct and auditable.
///
/// These variants are kernel-generic — no PURRDF-specific concept here. The slice
/// layer interprets which `UnitId` maps to which slice IRI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttributionRole {
    /// The unit that asserted the RDF data from which the finding originates.
    AssertionOrigin,
    /// The unit that defines (via `rdfs:isDefinedBy`) the vocabulary term at
    /// issue.
    DefinitionOwner,
    /// The unit that owns the SHACL shape that triggered the result.
    ShapeOwner,
    /// The unit that owns the logic rule that produced a derived fact.
    RuleOwner,
    /// The unit that asserted the focus node of a SHACL result.
    FocusOrigin,
    /// The unit that contributed the offending value in a SHACL result.
    ValueOrigin,
    /// The unit that contributed a premise fact to a derivation.
    DerivationSupport,
    /// The unit that defines the evaluation scope over which a result was
    /// computed (e.g. the graph/profile being validated).
    EvaluationScope,
}

impl AttributionRole {
    /// A stable lowercase string identifier for the role (used in SARIF
    /// properties and RDF projections — must never change once published).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AssertionOrigin => "assertion-origin",
            Self::DefinitionOwner => "definition-owner",
            Self::ShapeOwner => "shape-owner",
            Self::RuleOwner => "rule-owner",
            Self::FocusOrigin => "focus-origin",
            Self::ValueOrigin => "value-origin",
            Self::DerivationSupport => "derivation-support",
            Self::EvaluationScope => "evaluation-scope",
        }
    }
}

impl fmt::Display for AttributionRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A structured attribution: which compilation unit played which role in
/// producing a finding, derivation, or SHACL result (S0.3 / §9).
///
/// At serialization boundaries (`purrdf-validate`, SARIF, RDF projection) the
/// `unit` field is resolved to a public slice IRI via the `UnitInterner`; the
/// numeric id MUST NOT enter any persistent serialization (S0.5).
///
/// An optional `evidence` string carries a human-readable provenance note (e.g.
/// the literal location string or a term IRI that provides the evidence for the
/// attribution).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribution {
    /// The compilation unit (resolved to an IRI only at the output boundary).
    /// Runtime-only per S0.5.
    pub unit: UnitId,
    /// The role this unit plays in the attribution.
    pub role: AttributionRole,
    /// Optional provenance note (human-readable; does NOT enter fingerprints).
    pub evidence: Option<String>,
}

// ─── Record types ────────────────────────────────────────────────────────────

/// One physical assertion: the pair `(unit, artifact)` that asserted the quad
/// identified by `quad` (a `QuadHandle` into the associated `RdfDataset`).
///
/// The optional `location` carries a source-file coordinate when it is available
/// (e.g. parsed from a Turtle file with line/column info). Multiple
/// `AssertionOccurrence`s may share the same `quad` handle — one per
/// distinct `(unit, artifact)` that authored the same triple content (S0.3).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AssertionOccurrence {
    /// The quad in the associated `RdfDataset` that this occurrence asserts.
    pub quad: QuadHandle,
    /// The compilation unit that provided this assertion.
    pub unit: UnitId,
    /// The artifact within that unit that contains the assertion.
    pub artifact: ArtifactId,
    /// An optional source-file location string (e.g. `"path/to/file.ttl:12"`)
    /// for the asserted statement. `None` when no location is available (e.g.
    /// for generated quads or runtime-input quads where no file path is
    /// meaningful).
    pub location: Option<String>,
}

// ─── Interners ───────────────────────────────────────────────────────────────

/// Interner for `UnitId`s — maps a logical unit name to a dense numeric id.
///
/// The interner is the single minter of `UnitId`s for one `DatasetProvenance`.
/// Equal names yield the same id (idempotent). Names are opaque strings from the
/// caller's perspective (they could be filesystem paths, slice IRIs, or any
/// label); the kernel does not interpret them.
#[derive(Debug, Clone)]
pub struct UnitInterner {
    /// Dense table of unit names, addressed by `UnitId::index`.
    names: Vec<String>,
    /// Value → id index.
    index: HashMap<String, UnitId>,
}

impl UnitInterner {
    /// A fresh, empty interner.
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Intern a unit name, returning its (possibly existing) `UnitId`.
    /// Idempotent: equal names yield the same id.
    pub fn intern(&mut self, name: impl Into<String>) -> UnitId {
        let name = name.into();
        if let Some(&id) = self.index.get(&name) {
            return id;
        }
        let id = UnitId::from_index(
            u32::try_from(self.names.len()).expect("unit table exceeds u32::MAX entries"),
        );
        self.index.insert(name.clone(), id);
        self.names.push(name);
        id
    }

    /// Resolve a `UnitId` to its name. Panics if the id is out of range (which
    /// cannot happen for ids minted by this interner).
    pub fn name(&self, id: UnitId) -> &str {
        &self.names[id.index()]
    }

    /// The number of distinct units interned.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// True when no units have been interned.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

impl Default for UnitInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Interner for `ArtifactId`s — maps a logical artifact path to a dense numeric
/// id. The path is a string the caller controls (e.g. a repo-relative file path
/// or a content-addressed digest); the kernel does not interpret it.
#[derive(Debug, Clone)]
pub struct ArtifactInterner {
    /// Dense table of artifact logical paths.
    paths: Vec<String>,
    /// Value → id index.
    index: HashMap<String, ArtifactId>,
}

impl ArtifactInterner {
    /// A fresh, empty interner.
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Intern an artifact logical path, returning its (possibly existing)
    /// `ArtifactId`. Idempotent: equal paths yield the same id.
    pub fn intern(&mut self, path: impl Into<String>) -> ArtifactId {
        let path = path.into();
        if let Some(&id) = self.index.get(&path) {
            return id;
        }
        let id = ArtifactId::from_index(
            u32::try_from(self.paths.len()).expect("artifact table exceeds u32::MAX entries"),
        );
        self.index.insert(path.clone(), id);
        self.paths.push(path);
        id
    }

    /// Resolve an `ArtifactId` to its logical path.
    pub fn path(&self, id: ArtifactId) -> &str {
        &self.paths[id.index()]
    }

    /// The number of distinct artifacts interned.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// True when no artifacts have been interned.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

impl Default for ArtifactInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Interner for `OriginSetId`s — maps a canonical sorted set of
/// `(UnitId, ArtifactId)` pairs to a dense numeric id.
///
/// Two quads asserted by the same set of `(unit, artifact)` pairs share an
/// `OriginSetId` regardless of assertion order.
#[derive(Debug, Clone)]
pub struct OriginSetInterner {
    /// Dense table of interned origin sets, as sorted `Vec`s.
    sets: Vec<Vec<(UnitId, ArtifactId)>>,
    /// Value → id index, keyed by the sorted set.
    index: HashMap<Vec<(UnitId, ArtifactId)>, OriginSetId>,
}

impl OriginSetInterner {
    /// A fresh, empty interner.
    pub fn new() -> Self {
        Self {
            sets: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Intern an origin set. The set is canonicalized (sorted, deduped) before
    /// interning, so insertion order is irrelevant. Idempotent: the same set
    /// of pairs always yields the same `OriginSetId`.
    pub fn intern(&mut self, mut pairs: Vec<(UnitId, ArtifactId)>) -> OriginSetId {
        pairs.sort_unstable();
        pairs.dedup();
        if let Some(&id) = self.index.get(&pairs) {
            return id;
        }
        let id = OriginSetId::from_index(
            u32::try_from(self.sets.len()).expect("origin-set table exceeds u32::MAX entries"),
        );
        self.index.insert(pairs.clone(), id);
        self.sets.push(pairs);
        id
    }

    /// Resolve an `OriginSetId` to its canonical set of `(UnitId, ArtifactId)`
    /// pairs (sorted, deduped).
    pub fn set(&self, id: OriginSetId) -> &[(UnitId, ArtifactId)] {
        &self.sets[id.index()]
    }

    /// The number of distinct origin sets interned.
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// True when no origin sets have been interned.
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

impl Default for OriginSetInterner {
    fn default() -> Self {
        Self::new()
    }
}

// ─── DatasetProvenance ────────────────────────────────────────────────────────

/// The provenance sidecar for one `RdfDataset`.
///
/// Holds the three interners (units, artifacts, origin-sets) plus the
/// occurrence table (one [`AssertionOccurrence`] per physical assertion).
///
/// Invariant (enforced by [`check_provenance`]):
/// - Every `AssertionOccurrence` has exactly one `UnitId` and one `ArtifactId`.
/// - A quad MAY have multiple `AssertionOccurrence`s (one per `(unit, artifact)`
///   pair that independently authored the same quad content).
/// - There is no occurrence with an unknown/unset origin — any such occurrence
///   must not be inserted (the no-optionality doctrine means you return `Err`
///   rather than inserting a placeholder).
#[derive(Debug, Clone)]
pub struct DatasetProvenance {
    /// Interner for compilation/source units.
    pub units: UnitInterner,
    /// Interner for packaged artifacts within units.
    pub artifacts: ArtifactInterner,
    /// Interner for sets of `(unit, artifact)` pairs.
    pub origin_sets: OriginSetInterner,
    /// Every physical assertion, one per `(quad, unit, artifact)` triple.
    pub occurrences: Vec<AssertionOccurrence>,
    /// Unit kind table — parallel to `units.names`, indexed by `UnitId::index`.
    unit_kinds: Vec<OriginKind>,
}

impl DatasetProvenance {
    /// A fresh, empty provenance sidecar.
    pub fn new() -> Self {
        Self {
            units: UnitInterner::new(),
            artifacts: ArtifactInterner::new(),
            origin_sets: OriginSetInterner::new(),
            occurrences: Vec::new(),
            unit_kinds: Vec::new(),
        }
    }

    /// Register a new compilation unit with its kind.
    ///
    /// Returns the `UnitId`. If the name was already interned the existing id
    /// is returned and `kind` is ignored (the first registration wins).
    pub fn register_unit(&mut self, name: impl Into<String>, kind: OriginKind) -> UnitId {
        let name = name.into();
        let before = self.units.len();
        let id = self.units.intern(name);
        if self.units.len() > before {
            // New unit — record its kind at the corresponding index.
            self.unit_kinds.push(kind);
        }
        id
    }

    /// Register a new artifact.
    ///
    /// Returns the `ArtifactId`. Equal logical paths always yield the same id.
    pub fn register_artifact(&mut self, path: impl Into<String>) -> ArtifactId {
        self.artifacts.intern(path)
    }

    /// Record one physical assertion occurrence.
    ///
    /// This adds one [`AssertionOccurrence`] to `self.occurrences`. Callers
    /// MUST supply a concrete `unit` and `artifact`; there is no way to
    /// represent an unknown origin in the occurrence table.
    pub fn record_occurrence(
        &mut self,
        quad: QuadHandle,
        unit: UnitId,
        artifact: ArtifactId,
        location: Option<String>,
    ) {
        self.occurrences.push(AssertionOccurrence {
            quad,
            unit,
            artifact,
            location,
        });
    }

    /// Intern an origin set, returning its `OriginSetId`.
    ///
    /// Canonicalizes (sorts + dedups) the `pairs` vector before interning.
    pub fn intern_origin_set(&mut self, pairs: Vec<(UnitId, ArtifactId)>) -> OriginSetId {
        self.origin_sets.intern(pairs)
    }

    /// The kind of a unit, or `None` if the id is out of range.
    pub fn unit_kind(&self, id: UnitId) -> Option<&OriginKind> {
        self.unit_kinds.get(id.index())
    }

    /// A deterministic, runtime-id-free **public projection** of this provenance,
    /// for content addressing (S0.5).
    ///
    /// Every row is expressed through the interners' PUBLIC strings — unit NAMES and
    /// artifact PATHS — and the `OriginKind` string; the runtime-only numeric
    /// [`UnitId`] / [`ArtifactId`] / [`OriginSetId`] NEVER appear. The rows are sorted
    /// so the projection is independent of the order units/artifacts were interned or
    /// occurrences recorded: re-allocating the same public provenance in a different
    /// internal order yields the identical projection (and so the identical digest).
    ///
    /// The shape is `(quad_index, unit_name, unit_kind, artifact_path, location)` per
    /// occurrence, sorted and deduplicated. The `quad_index` is the dense ordinal of
    /// the asserted quad (`QuadHandle::index()`) — it is CONTENT-STABLE within one
    /// frozen `RdfDataset` (derived from freeze-sort order, not insertion order) and is
    /// included so that two occurrences identical in `(unit, artifact, location)` but
    /// asserting DIFFERENT quads are preserved as distinct rows rather than collapsing.
    ///
    /// Two `DatasetProvenance`s with the same public provenance — regardless of
    /// internal id numbering — produce equal projections.
    #[must_use]
    pub fn public_projection(&self) -> Vec<(usize, String, String, String, Option<String>)> {
        let mut rows: Vec<(usize, String, String, String, Option<String>)> = self
            .occurrences
            .iter()
            .map(|occ| {
                let quad_index = occ.quad.index();
                let unit_name = self.units.name(occ.unit).to_owned();
                // A unit always has a registered kind for a gate-valid provenance; an
                // out-of-range id (forged, never minted by `register_unit`) projects as
                // the explicit "unknown-kind" marker rather than panicking, so the
                // public projection is total.
                let kind = self
                    .unit_kind(occ.unit)
                    .map(OriginKind::to_string)
                    .unwrap_or_else(|| "unknown-kind".to_owned());
                let artifact_path = self.artifacts.path(occ.artifact).to_owned();
                (
                    quad_index,
                    unit_name,
                    kind,
                    artifact_path,
                    occ.location.clone(),
                )
            })
            .collect();
        rows.sort();
        rows.dedup();
        rows
    }
}

impl Default for DatasetProvenance {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Provenance gate ──────────────────────────────────────────────────────────

/// An error from the provenance gate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProvenanceError {
    /// An occurrence references a `UnitId` that has no registered kind.
    UnknownUnit {
        occurrence_index: usize,
        unit: UnitId,
    },
    /// An occurrence references an `ArtifactId` that is out of range.
    UnknownArtifact {
        occurrence_index: usize,
        artifact: ArtifactId,
    },
    /// A `QuadHandle` has zero occurrences but the gate was asked to enforce
    /// full coverage (every semantic quad must have ≥1 occurrence).
    MissingOccurrence { quad_index: usize },
}

impl fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownUnit {
                occurrence_index,
                unit,
            } => write!(
                f,
                "occurrence[{occurrence_index}] references {unit} which has no registered kind \
                 (no-optionality: unknown origin is a hard failure)"
            ),
            Self::UnknownArtifact {
                occurrence_index,
                artifact,
            } => write!(
                f,
                "occurrence[{occurrence_index}] references {artifact} which is out of range"
            ),
            Self::MissingOccurrence { quad_index } => write!(
                f,
                "quad handle {quad_index} has no assertion occurrence \
                 (every semantic quad must have ≥1 occurrence)"
            ),
        }
    }
}

impl std::error::Error for ProvenanceError {}

/// Validate the provenance sidecar against a set of expected quad handles.
///
/// Enforces:
/// 1. Every `AssertionOccurrence` references a `UnitId` with a registered kind
///    (no-optionality: there is no `Unknown` variant).
/// 2. Every `AssertionOccurrence` references an `ArtifactId` in range.
/// 3. If `expected_quads` is non-empty, every quad handle in that set has at
///    least one occurrence.
///
/// Non-`Source` units (`RootOntology`, `Import`, `Generated`, `RuntimeInput`)
/// are explicitly representable and pass the gate without error.
///
/// # Errors
///
/// Returns all violations found (not just the first).
pub fn check_provenance(
    prov: &DatasetProvenance,
    expected_quads: &[QuadHandle],
) -> Result<(), Vec<ProvenanceError>> {
    let mut errors: Vec<ProvenanceError> = Vec::new();

    // 1 & 2: every occurrence has a valid unit kind and artifact.
    for (idx, occ) in prov.occurrences.iter().enumerate() {
        if prov.unit_kinds.get(occ.unit.index()).is_none() {
            errors.push(ProvenanceError::UnknownUnit {
                occurrence_index: idx,
                unit: occ.unit,
            });
        }
        if occ.artifact.index() >= prov.artifacts.len() {
            errors.push(ProvenanceError::UnknownArtifact {
                occurrence_index: idx,
                artifact: occ.artifact,
            });
        }
    }

    // 3: every expected quad handle has ≥1 occurrence.
    if !expected_quads.is_empty() {
        // Build a set of handles that appear in at least one occurrence.
        let covered: std::collections::HashSet<usize> =
            prov.occurrences.iter().map(|o| o.quad.index()).collect();
        for handle in expected_quads {
            if !covered.contains(&handle.index()) {
                errors.push(ProvenanceError::MissingOccurrence {
                    quad_index: handle.index(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a QuadHandle from a raw index.
    fn qh(i: u32) -> QuadHandle {
        QuadHandle::from_index(i)
    }

    // ── UnitId ─────────────────────────────────────────────────────────────

    #[test]
    fn unit_id_index_round_trips() {
        for raw in [0u32, 1, 42, 999] {
            let id = UnitId::from_index(raw);
            assert_eq!(id.index(), raw as usize);
        }
    }

    #[test]
    fn unit_id_display() {
        assert_eq!(UnitId::from_index(7).to_string(), "unit#7");
    }

    // ── ArtifactId ─────────────────────────────────────────────────────────

    #[test]
    fn artifact_id_index_round_trips() {
        for raw in [0u32, 1, 42, 999] {
            let id = ArtifactId::from_index(raw);
            assert_eq!(id.index(), raw as usize);
        }
    }

    #[test]
    fn artifact_id_display() {
        assert_eq!(ArtifactId::from_index(3).to_string(), "artifact#3");
    }

    // ── OriginSetId ────────────────────────────────────────────────────────

    #[test]
    fn origin_set_id_index_round_trips() {
        for raw in [0u32, 1, 42, 999] {
            let id = OriginSetId::from_index(raw);
            assert_eq!(id.index(), raw as usize);
        }
    }

    #[test]
    fn origin_set_id_display() {
        assert_eq!(OriginSetId::from_index(5).to_string(), "origin-set#5");
    }

    // ── UnitInterner ───────────────────────────────────────────────────────

    #[test]
    fn unit_interner_is_idempotent() {
        let mut i = UnitInterner::new();
        let a = i.intern("slices/core/epistemics");
        let b = i.intern("slices/core/epistemics");
        let c = i.intern("slices/core/observations");
        assert_eq!(a, b, "same name → same id");
        assert_ne!(a, c, "different names → different ids");
        assert_eq!(i.len(), 2);
    }

    #[test]
    fn unit_interner_name_resolves() {
        let mut i = UnitInterner::new();
        let id = i.intern("root-ontology");
        assert_eq!(i.name(id), "root-ontology");
    }

    #[test]
    fn unit_interner_starts_empty() {
        let i = UnitInterner::new();
        assert!(i.is_empty());
    }

    // ── ArtifactInterner ───────────────────────────────────────────────────

    #[test]
    fn artifact_interner_is_idempotent() {
        let mut i = ArtifactInterner::new();
        let a = i.intern("slices/core/epistemics/epistemics.ttl");
        let b = i.intern("slices/core/epistemics/epistemics.ttl");
        let c = i.intern("slices/core/epistemics/shapes.ttl");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(i.len(), 2);
    }

    #[test]
    fn artifact_interner_path_resolves() {
        let mut i = ArtifactInterner::new();
        let id = i.intern("ontology/purrdf.ttl");
        assert_eq!(i.path(id), "ontology/purrdf.ttl");
    }

    // ── OriginSetInterner ──────────────────────────────────────────────────

    #[test]
    fn origin_set_interner_is_idempotent_and_order_independent() {
        let u0 = UnitId::from_index(0);
        let u1 = UnitId::from_index(1);
        let a0 = ArtifactId::from_index(0);
        let a1 = ArtifactId::from_index(1);

        let mut i = OriginSetInterner::new();
        // Same set in two different insertion orders.
        let id1 = i.intern(vec![(u0, a0), (u1, a1)]);
        let id2 = i.intern(vec![(u1, a1), (u0, a0)]);
        assert_eq!(id1, id2, "order-independent: same set → same id");

        // A distinct singleton set.
        let id3 = i.intern(vec![(u0, a0)]);
        assert_ne!(id1, id3, "different sets → different ids");

        assert_eq!(i.len(), 2);
    }

    #[test]
    fn origin_set_interner_dedupes_pairs() {
        let u0 = UnitId::from_index(0);
        let a0 = ArtifactId::from_index(0);

        let mut i = OriginSetInterner::new();
        // Duplicate pair in the input — should collapse.
        let id1 = i.intern(vec![(u0, a0), (u0, a0)]);
        let id2 = i.intern(vec![(u0, a0)]);
        assert_eq!(id1, id2, "duplicate pairs collapse");
    }

    #[test]
    fn origin_set_resolves() {
        let u0 = UnitId::from_index(0);
        let a0 = ArtifactId::from_index(0);
        let u1 = UnitId::from_index(1);
        let a1 = ArtifactId::from_index(1);

        let mut i = OriginSetInterner::new();
        let id = i.intern(vec![(u1, a1), (u0, a0)]);
        // The stored set is canonically sorted.
        let set = i.set(id);
        assert_eq!(set.len(), 2);
        // Sorted order: u0 < u1.
        assert_eq!(set[0], (u0, a0));
        assert_eq!(set[1], (u1, a1));
    }

    // ── OriginKind ─────────────────────────────────────────────────────────

    #[test]
    fn origin_kind_display() {
        assert_eq!(OriginKind::Source.to_string(), "source");
        assert_eq!(OriginKind::RootOntology.to_string(), "root-ontology");
        assert_eq!(OriginKind::Import.to_string(), "import");
        assert_eq!(OriginKind::Generated.to_string(), "generated");
        assert_eq!(OriginKind::RuntimeInput.to_string(), "runtime-input");
    }

    // ── DatasetProvenance ──────────────────────────────────────────────────

    #[test]
    fn dataset_provenance_register_unit_is_idempotent() {
        let mut prov = DatasetProvenance::new();
        let id1 = prov.register_unit("slices/core/epistemics", OriginKind::Source);
        let id2 = prov.register_unit("slices/core/epistemics", OriginKind::Source);
        assert_eq!(id1, id2);
        assert_eq!(prov.units.len(), 1);
    }

    #[test]
    fn dataset_provenance_unit_kind_accessible() {
        let mut prov = DatasetProvenance::new();
        let id = prov.register_unit("root", OriginKind::RootOntology);
        assert_eq!(prov.unit_kind(id), Some(&OriginKind::RootOntology));
    }

    #[test]
    fn dataset_provenance_all_non_source_kinds_representable() {
        // S0.2 explicitly states every non-slice unit kind is representable.
        let mut prov = DatasetProvenance::new();
        let _root = prov.register_unit("root", OriginKind::RootOntology);
        let _import = prov.register_unit("import:owl", OriginKind::Import);
        let _gen = prov.register_unit("generated:crossref", OriginKind::Generated);
        let _rt = prov.register_unit("runtime:fixture", OriginKind::RuntimeInput);
        assert_eq!(prov.units.len(), 4);
    }

    // ── Duplicate-origin retention test ────────────────────────────────────
    //
    // Two source files each asserting the SAME triple → ONE QuadHandle but
    // TWO AssertionOccurrences, one per (unit, artifact) pair. This is the
    // central set-valued invariant of S0.3.

    #[test]
    fn duplicate_origin_retention_two_occurrences_one_quad() {
        let mut prov = DatasetProvenance::new();

        // Register two distinct source units (e.g. two slice module files).
        let unit_a = prov.register_unit("slices/core/epistemics", OriginKind::Source);
        let unit_b = prov.register_unit("slices/ext/beliefs", OriginKind::Source);

        // Register one artifact per unit.
        let artifact_a = prov.register_artifact("slices/core/epistemics/epistemics.ttl");
        let artifact_b = prov.register_artifact("slices/ext/beliefs/beliefs.ttl");

        // Both files contain the same triple — the dataset deduplicates to one
        // QuadHandle (handle 0 in this example).
        let shared_quad = qh(0);

        prov.record_occurrence(
            shared_quad,
            unit_a,
            artifact_a,
            Some("epistemics.ttl:12".into()),
        );
        prov.record_occurrence(
            shared_quad,
            unit_b,
            artifact_b,
            Some("beliefs.ttl:7".into()),
        );

        // There must be exactly TWO occurrences, both pointing at the same handle.
        assert_eq!(
            prov.occurrences.len(),
            2,
            "two source files → two occurrences"
        );
        assert_eq!(
            prov.occurrences[0].quad, shared_quad,
            "first occurrence references the shared quad"
        );
        assert_eq!(
            prov.occurrences[1].quad, shared_quad,
            "second occurrence references the same quad"
        );
        assert_ne!(
            prov.occurrences[0].unit, prov.occurrences[1].unit,
            "distinct units"
        );
        assert_ne!(
            prov.occurrences[0].artifact, prov.occurrences[1].artifact,
            "distinct artifacts"
        );
    }

    // ── Provenance gate tests ──────────────────────────────────────────────

    #[test]
    fn gate_passes_on_valid_single_file_provenance() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("slices/core/observations", OriginKind::Source);
        let artifact = prov.register_artifact("slices/core/observations/observations.ttl");
        let quad = qh(0);
        prov.record_occurrence(quad, unit, artifact, None);

        let result = check_provenance(&prov, &[quad]);
        assert!(
            result.is_ok(),
            "valid single-file provenance must pass gate"
        );
    }

    #[test]
    fn gate_passes_multi_file_fixture() {
        let mut prov = DatasetProvenance::new();
        let unit_a = prov.register_unit("slices/core/a", OriginKind::Source);
        let unit_b = prov.register_unit("slices/core/b", OriginKind::Source);
        let art_a = prov.register_artifact("a/a.ttl");
        let art_b = prov.register_artifact("b/b.ttl");

        let q0 = qh(0);
        let q1 = qh(1);
        let q2 = qh(2);

        prov.record_occurrence(q0, unit_a, art_a, Some("a.ttl:1".into()));
        prov.record_occurrence(q1, unit_a, art_a, Some("a.ttl:2".into()));
        prov.record_occurrence(q2, unit_b, art_b, None);

        let result = check_provenance(&prov, &[q0, q1, q2]);
        assert!(result.is_ok(), "multi-file fixture must pass gate");
    }

    #[test]
    fn gate_fails_when_occurrence_missing_for_quad() {
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("root", OriginKind::RootOntology);
        let artifact = prov.register_artifact("ontology/purrdf.ttl");
        let q0 = qh(0);
        let q1 = qh(1); // No occurrence for q1.
        prov.record_occurrence(q0, unit, artifact, None);

        let result = check_provenance(&prov, &[q0, q1]);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(
            matches!(
                errs[0],
                ProvenanceError::MissingOccurrence { quad_index: 1 }
            ),
            "expected MissingOccurrence for quad 1, got: {:?}",
            errs[0]
        );
    }

    #[test]
    fn gate_fails_when_unit_has_no_kind() {
        // Craft a provenance where occurrences reference a UnitId that was
        // forged (no register_unit call), so unit_kinds is shorter than needed.
        let mut prov = DatasetProvenance::new();
        // Register unit 0 normally.
        let _good_unit = prov.register_unit("good", OriginKind::Source);
        let artifact = prov.register_artifact("good.ttl");

        // Forge a UnitId beyond the registered range.
        let bad_unit = UnitId::from_index(99);
        let q = qh(0);
        // Directly push an occurrence with the bad unit (bypassing register_unit).
        prov.occurrences.push(AssertionOccurrence {
            quad: q,
            unit: bad_unit,
            artifact,
            location: None,
        });

        let result = check_provenance(&prov, &[]);
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(
            errs.iter().any(
                |e| matches!(e, ProvenanceError::UnknownUnit { unit, .. } if unit.index() == 99)
            ),
            "expected UnknownUnit error for forged id"
        );
    }

    #[test]
    fn gate_non_source_units_pass() {
        // All non-Source OriginKind variants must be accepted by the gate.
        for kind in [
            OriginKind::RootOntology,
            OriginKind::Import,
            OriginKind::Generated,
            OriginKind::RuntimeInput,
        ] {
            let mut prov = DatasetProvenance::new();
            let unit = prov.register_unit(kind.to_string(), kind);
            let artifact = prov.register_artifact("some/artifact.ttl");
            let q = qh(0);
            prov.record_occurrence(q, unit, artifact, None);
            let result = check_provenance(&prov, &[q]);
            assert!(result.is_ok(), "non-Source kind must pass gate");
        }
    }

    // ── OriginSetId interning: same set → same id ──────────────────────────

    #[test]
    fn origin_set_interning_shared_id_for_equal_sets() {
        // Two quads with the same `(unit, artifact)` origin set must share an
        // OriginSetId regardless of the order the sets are interned.
        let mut prov = DatasetProvenance::new();
        let u0 = prov.register_unit("u0", OriginKind::Source);
        let u1 = prov.register_unit("u1", OriginKind::Source);
        let a0 = prov.register_artifact("a0.ttl");
        let a1 = prov.register_artifact("a1.ttl");

        let id_1 = prov.intern_origin_set(vec![(u0, a0), (u1, a1)]);
        // Same set but reversed order — must produce the same OriginSetId.
        let id_2 = prov.intern_origin_set(vec![(u1, a1), (u0, a0)]);
        assert_eq!(id_1, id_2, "equal origin sets must share an OriginSetId");

        // A different singleton set must produce a new id.
        let id_3 = prov.intern_origin_set(vec![(u0, a0)]);
        assert_ne!(id_1, id_3, "distinct sets must have distinct OriginSetIds");
    }

    // ── Attribution types ─────────────────────────────────────────────────────

    #[test]
    fn attribution_role_as_str_is_stable() {
        // Stable string identifiers must never change once published (S0.5 / §9).
        assert_eq!(
            AttributionRole::AssertionOrigin.as_str(),
            "assertion-origin"
        );
        assert_eq!(
            AttributionRole::DefinitionOwner.as_str(),
            "definition-owner"
        );
        assert_eq!(AttributionRole::ShapeOwner.as_str(), "shape-owner");
        assert_eq!(AttributionRole::RuleOwner.as_str(), "rule-owner");
        assert_eq!(AttributionRole::FocusOrigin.as_str(), "focus-origin");
        assert_eq!(AttributionRole::ValueOrigin.as_str(), "value-origin");
        assert_eq!(
            AttributionRole::DerivationSupport.as_str(),
            "derivation-support"
        );
        assert_eq!(
            AttributionRole::EvaluationScope.as_str(),
            "evaluation-scope"
        );
    }

    #[test]
    fn attribution_role_display_matches_as_str() {
        for role in [
            AttributionRole::AssertionOrigin,
            AttributionRole::DefinitionOwner,
            AttributionRole::ShapeOwner,
            AttributionRole::RuleOwner,
            AttributionRole::FocusOrigin,
            AttributionRole::ValueOrigin,
            AttributionRole::DerivationSupport,
            AttributionRole::EvaluationScope,
        ] {
            assert_eq!(role.to_string(), role.as_str());
        }
    }

    #[test]
    fn attribution_construction_and_equality() {
        let u0 = UnitId::from_index(0);
        let u1 = UnitId::from_index(1);

        let a1 = Attribution {
            unit: u0,
            role: AttributionRole::ShapeOwner,
            evidence: Some("shapes/core/epistemics/shapes.ttl".to_owned()),
        };
        let a1b = Attribution {
            unit: u0,
            role: AttributionRole::ShapeOwner,
            evidence: Some("shapes/core/epistemics/shapes.ttl".to_owned()),
        };
        let a2 = Attribution {
            unit: u1,
            role: AttributionRole::FocusOrigin,
            evidence: None,
        };

        assert_eq!(a1, a1b, "same fields → equal");
        assert_ne!(a1, a2, "different unit/role → not equal");
    }

    #[test]
    fn attribution_vec_carries_multiple_roles() {
        // A single finding can carry multiple attributions with different roles.
        // This is the core of §9: structured attribution, not a scalar slice field.
        let u_shape = UnitId::from_index(0);
        let u_data = UnitId::from_index(1);

        let attributions = [
            Attribution {
                unit: u_shape,
                role: AttributionRole::ShapeOwner,
                evidence: None,
            },
            Attribution {
                unit: u_data,
                role: AttributionRole::FocusOrigin,
                evidence: Some("http://example.org/FocusNode".to_owned()),
            },
        ];

        assert_eq!(attributions.len(), 2);
        assert_eq!(attributions[0].role, AttributionRole::ShapeOwner);
        assert_eq!(attributions[1].role, AttributionRole::FocusOrigin);
        // The two attributions reference different units (cross-slice).
        assert_ne!(attributions[0].unit, attributions[1].unit);
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Phase-specific, path-independent, semantic Merkle cache keys plus
//! SCC / profile composition (RFC #820 §12 / §8, child S6a).
//!
//! # Principle 12 — semantic, phase-specific, path-independent cache keys
//!
//! The legacy cache hashed *physical relative paths, file sizes, and raw bytes*
//! (`src/purrdf_tools/generator.py::source_hash`), which conflicts with the
//! doctrine that the slice-group path carries no semantics: moving
//! `slices/core/x` to another group must **not** invalidate its semantic
//! compilation. This module replaces that with a Merkle-style key built **only**
//! from content / IRI / version-string inputs. **No filesystem directory name
//! ever enters a key.** Every input is one of:
//!
//! - the slice IRI (public, persistent identity — never a numeric interner ID),
//! - a content digest (raw byte digest or canonical-RDF *semantic* digest),
//! - a normalized *logical* artifact path (intra-slice, group-prefix-free),
//! - the dependency-output digests of upstream units (Merkle composition),
//! - a [`ToolchainContext`] version string + reasoning-profile id.
//!
//! Different [`Phase`]s select different digest **roots** so that a change that
//! is invisible to a phase does not invalidate that phase's key:
//!
//! - [`Phase::Parse`] / [`Phase::Syntax`] are **byte-sensitive** — they include
//!   the *raw* artifact digest, so a comment-only edit *does* change them.
//! - [`Phase::Reason`] is **semantics-sensitive** — it includes the *semantic*
//!   (canonical N-Triples) digest and **excludes raw bytes / comments**, so a
//!   comment-only edit that leaves the canonical RDF unchanged produces the
//!   **same** reasoning key (the headline acceptance).
//! - [`Phase::Shacl`] reasons over semantic module/data + shapes (semantic).
//! - [`Phase::Bundle`] packages raw bytes + metadata (byte-sensitive).
//!
//! # Principle 8 — source / link / product units
//!
//! - **Source unit** = one slice (parse, lint, inventory, hash).
//! - **Link unit** = a dependency strongly-connected component; mutually
//!   dependent slices collapse into one [`LinkUnit`] (so a core cycle reasons as
//!   one union), while singletons remain their own link unit. Member slice IRIs
//!   are retained on the link unit so **output attribution stays at slice
//!   granularity even when execution occurs over an SCC**.
//! - **Product unit** = a dependency-closed profile / bundle (validate full
//!   composition) — see [`ProductUnit`] / [`dependency_closure`].
//!
//! # Persistent-vs-runtime ID rule (S0.5)
//!
//! Cache keys hash **content / IRIs / version strings**, never numeric runtime
//! interner IDs (`UnitId` / `TermId` / `QuadId` / …). The slice IRI is the
//! persistent identity; the group path is never read.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::{DiGraph, NodeIndex};
use sha2::{Digest, Sha256};

use crate::artifact::{ArtifactRecord, ArtifactRole};
use crate::catalog::{SliceCatalog, SliceRecord};
use crate::error::SliceError;
use crate::ownership::{DependencyEdge, ReconciliationStatus, SliceIri};

// ── Phases ──────────────────────────────────────────────────────────────────

/// A compilation phase. Each phase selects a different Merkle root so a change
/// invisible to the phase does not invalidate its key (RFC §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Raw-byte parse of source artifacts. Byte-sensitive (raw digest).
    Parse,
    /// Turtle / RDF syntax check. Byte-sensitive (raw digest).
    Syntax,
    /// SHACL validation: semantic module/data + shapes + config. Semantic.
    Shacl,
    /// OWL/logic reasoning: semantic module + dependency closure + rules.
    /// Semantic — **excludes raw bytes / comments** (comment-only invariance).
    Reason,
    /// Packaging into the GTS bundle: raw bytes + metadata. Byte-sensitive.
    Bundle,
}

impl Phase {
    /// A stable, path-free discriminator string folded into the key so the same
    /// inputs under different phases never collide.
    fn tag(self) -> &'static str {
        match self {
            Self::Parse => "phase:parse",
            Self::Syntax => "phase:syntax",
            Self::Shacl => "phase:shacl",
            Self::Reason => "phase:reason",
            Self::Bundle => "phase:bundle",
        }
    }

    /// Whether this phase is **byte-sensitive** (folds the raw artifact digest,
    /// so a comment-only edit changes the key). The complement — [`Phase::Shacl`]
    /// and [`Phase::Reason`] — is **semantics-sensitive** and folds the
    /// canonical-RDF *semantic* digest instead, achieving comment-only
    /// invariance for the reasoning phase.
    pub fn is_byte_sensitive(self) -> bool {
        match self {
            Self::Parse | Self::Syntax | Self::Bundle => true,
            Self::Shacl | Self::Reason => false,
        }
    }

    /// Whether this phase folds the dependency closure of upstream units (RFC
    /// §12: reasoning = semantic module + dependency closure + rules). Parse /
    /// syntax of a single source unit are intra-slice and ignore the closure.
    fn folds_dependencies(self) -> bool {
        match self {
            Self::Reason | Self::Shacl | Self::Bundle => true,
            Self::Parse | Self::Syntax => false,
        }
    }

    /// The artifact roles whose digests feed *this* phase's root. Selecting
    /// roles per phase is what makes the key phase-specific: a docs-only or
    /// example-only change cannot invalidate the reasoning key.
    fn includes_role(self, role: &ArtifactRole) -> bool {
        match self {
            // Parse / syntax look at every authored artifact's bytes.
            Self::Parse | Self::Syntax | Self::Bundle => true,
            // Reasoning closure = ontology modules + shapes + rules; not docs,
            // examples, citations, translations, or test/query prose. The
            // manifest is ALWAYS semantically load-bearing (tier / sliceDependsOn
            // / profile) and so is folded under the semantic phases too — its
            // *semantic* digest, so a comment-only manifest edit is still
            // invisible. (manifest.ttl is RDF, so it always carries a semantic
            // digest — see catalog.rs `compute_semantic_digest`.)
            Self::Reason => matches!(
                role,
                ArtifactRole::Module | ArtifactRole::Shapes | ArtifactRole::Manifest
            ),
            // SHACL = semantic module/data + shapes + manifest-borne facts.
            Self::Shacl => matches!(
                role,
                ArtifactRole::Module | ArtifactRole::Shapes | ArtifactRole::Manifest
            ),
        }
    }
}

// ── Toolchain context ───────────────────────────────────────────────────────

/// The toolchain / configuration context folded into every key: compiler/rule
/// version and the active reasoning-profile id. These are **version strings**,
/// never numeric runtime IDs (S0.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainContext {
    /// The compiler / rule-engine version string (e.g. `"purrdf-logic v1"`).
    pub compiler_version: String,
    /// The active reasoning-profile id (e.g. `"el"`, `"dl"`, `"native"`).
    pub reasoning_profile: String,
}

impl ToolchainContext {
    /// Construct a toolchain context.
    pub fn new(compiler_version: impl Into<String>, reasoning_profile: impl Into<String>) -> Self {
        Self {
            compiler_version: compiler_version.into(),
            reasoning_profile: reasoning_profile.into(),
        }
    }
}

// ── Compilation units (RFC §8) ──────────────────────────────────────────────

/// A **link unit**: a dependency strongly-connected component (RFC §8). Mutually
/// dependent slices collapse into one link unit (reason them as one union);
/// singletons are their own unit. The member slice IRIs are retained so output
/// attribution stays at slice granularity even when execution is over the SCC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkUnit {
    /// The slice IRIs that form this SCC, sorted for determinism. A single-slice
    /// (acyclic) unit has exactly one member.
    pub members: Vec<SliceIri>,
}

impl LinkUnit {
    /// Whether this link unit is a genuine cycle (more than one member).
    pub fn is_cycle(&self) -> bool {
        self.members.len() > 1
    }

    /// Whether the given slice IRI is a member of this link unit (attribution at
    /// slice granularity).
    pub fn contains(&self, slice: &str) -> bool {
        self.members.iter().any(|m| m == slice)
    }
}

/// A **product unit**: a dependency-closed profile / bundle (RFC §8). It names a
/// root slice (or profile seed) and the transitive closure of its dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductUnit {
    /// The seed slice IRI(s) the product is built around (sorted).
    pub seeds: Vec<SliceIri>,
    /// The dependency-closed member set: seeds ∪ all transitive dependencies
    /// (sorted, deduplicated).
    pub closure: Vec<SliceIri>,
}

// ── SCC composition ─────────────────────────────────────────────────────────

/// Which dependency edges drive composition. Per RFC §10 only *semantic* edges
/// reconcile with a real build dependency; we further require the edge be
/// `Matched` or `Undeclared` (i.e. backed by semantic evidence), never a
/// `Stale` synthetic edge (declared-but-unobserved).
fn is_build_edge(edge: &DependencyEdge) -> bool {
    edge.edge_kind.is_semantic() && edge.reconciliation != ReconciliationStatus::Stale
}

/// Build a directed dependency graph projected from S4 edges, over the slice
/// IRIs of a catalog: a `from → to` edge means *from depends on to*. Every slice
/// in the catalog is a node, even with no edges (singletons must still appear as
/// their own link unit / product seed).
fn build_unit_graph(catalog: &SliceCatalog, edges: &[DependencyEdge]) -> DiGraph<SliceIri, ()> {
    let mut graph = DiGraph::new();
    let mut index: BTreeMap<SliceIri, NodeIndex> = BTreeMap::new();

    // Sort slice IRIs for deterministic node insertion order.
    let mut slices: Vec<SliceIri> = catalog
        .records()
        .iter()
        .map(|r| r.manifest.slice_iri.clone())
        .collect();
    slices.sort();
    slices.dedup();
    for slice in &slices {
        let idx = graph.add_node(slice.clone());
        index.insert(slice.clone(), idx);
    }

    // Add build-relevant edges (semantic, non-stale). Deduplicate so multiple
    // evidence rows for one (from,to) do not multiply edges.
    let mut seen: BTreeSet<(SliceIri, SliceIri)> = BTreeSet::new();
    for edge in edges {
        if !is_build_edge(edge) {
            continue;
        }
        let key = (edge.from_slice.clone(), edge.to_slice.clone());
        if !seen.insert(key) {
            continue;
        }
        let (Some(&from), Some(&to)) = (index.get(&edge.from_slice), index.get(&edge.to_slice))
        else {
            // An edge to/from a slice not in the catalog is ignored — the
            // catalog node set is authoritative.
            continue;
        };
        graph.add_edge(from, to, ());
    }

    graph
}

/// Compute the **link units** (SCCs) of a catalog under its S4 dependency edges
/// (RFC §8). Mutually dependent slices collapse to one [`LinkUnit`]; singletons
/// remain individually nameable. The result is deterministic: members are
/// sorted within each unit, and units are sorted by their smallest member.
pub fn link_units(catalog: &SliceCatalog, edges: &[DependencyEdge]) -> Vec<LinkUnit> {
    let graph = build_unit_graph(catalog, edges);
    let mut units: Vec<LinkUnit> = petgraph::algo::tarjan_scc(&graph)
        .into_iter()
        .map(|component| {
            let mut members: Vec<SliceIri> =
                component.into_iter().map(|n| graph[n].clone()).collect();
            members.sort();
            LinkUnit { members }
        })
        .collect();
    units.sort_by(|a, b| a.members.first().cmp(&b.members.first()));
    units
}

/// The dependency-closed member set of `seeds` over the catalog's S4 edges:
/// `seeds ∪ all transitive dependencies` (RFC §8 product unit / §4 closure).
/// Path-independent — built from slice IRIs only.
pub fn dependency_closure(
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    seeds: &[SliceIri],
) -> Vec<SliceIri> {
    // Adjacency: from → {to} over build edges.
    let valid: BTreeSet<&SliceIri> = catalog
        .records()
        .iter()
        .map(|r| &r.manifest.slice_iri)
        .collect();
    let mut adj: BTreeMap<SliceIri, BTreeSet<SliceIri>> = BTreeMap::new();
    for edge in edges {
        if !is_build_edge(edge) {
            continue;
        }
        adj.entry(edge.from_slice.clone())
            .or_default()
            .insert(edge.to_slice.clone());
    }

    let mut closure: BTreeSet<SliceIri> = BTreeSet::new();
    let mut stack: Vec<SliceIri> = Vec::new();
    for seed in seeds {
        if valid.contains(seed) {
            stack.push(seed.clone());
        }
    }
    while let Some(node) = stack.pop() {
        if !closure.insert(node.clone()) {
            continue;
        }
        if let Some(targets) = adj.get(&node) {
            for t in targets {
                if valid.contains(t) && !closure.contains(t) {
                    stack.push(t.clone());
                }
            }
        }
    }
    closure.into_iter().collect()
}

/// Build a [`ProductUnit`] for `seeds`: a dependency-closed profile (RFC §8).
pub fn product_unit(
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    seeds: &[SliceIri],
) -> ProductUnit {
    let mut seed_vec: Vec<SliceIri> = seeds.to_vec();
    seed_vec.sort();
    seed_vec.dedup();
    let closure = dependency_closure(catalog, edges, &seed_vec);
    ProductUnit {
        seeds: seed_vec,
        closure,
    }
}

// ── Per-slice phase digest (the Merkle leaf) ────────────────────────────────

/// Select the per-artifact digest a phase folds. Byte-sensitive phases fold the
/// **raw** digest (comments matter); semantics-sensitive phases fold the
/// **semantic** (canonical N-Triples) digest (comments do not). This single
/// selection is the mechanism behind the comment-only reasoning invariance.
fn phase_artifact_digest(phase: Phase, artifact: &ArtifactRecord) -> Result<String, SliceError> {
    if phase.is_byte_sensitive() {
        return Ok(artifact.raw_digest.clone());
    }
    // Semantic phase: an RDF artifact must carry a semantic digest; a
    // semantics-sensitive phase that only includes RDF roles (Module/Shapes)
    // therefore always has one. Missing it is a hard failure (no-optionality).
    match &artifact.semantic_digest {
        Some(d) => Ok(d.clone()),
        None => Err(SliceError::InvalidManifest(format!(
            "phase {} requires a semantic digest for artifact {} (role {:?}) but none was computed",
            phase.tag(),
            artifact.logical_path,
            artifact.role
        ))),
    }
}

/// Compute the **per-slice phase leaf digest**: a SHA-256 over the phase tag, the
/// slice IRI, and the selected (role-filtered) artifact digests keyed by logical
/// path. Path-independent: only the slice IRI and *logical* (intra-slice) paths
/// enter — never the group directory.
fn slice_phase_leaf(
    phase: Phase,
    record: &SliceRecord,
    toolchain: &ToolchainContext,
) -> Result<String, SliceError> {
    let mut hasher = Sha256::new();
    hasher.update(phase.tag().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(b"slice-iri\x1f");
    hasher.update(record.manifest.slice_iri.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(b"compiler\x1f");
    hasher.update(toolchain.compiler_version.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(b"reasoning-profile\x1f");
    hasher.update(toolchain.reasoning_profile.as_bytes());
    hasher.update(b"\x1f");

    // The manifest is always semantically load-bearing (tier / deps / profiles):
    // fold its digest under every phase, selecting raw vs semantic per phase.
    // We fold every included artifact in sorted logical-path order.
    let mut leaves: BTreeMap<&str, (String, &ArtifactRole)> = BTreeMap::new();
    for artifact in &record.artifacts {
        if !phase.includes_role(&artifact.role) {
            continue;
        }
        let digest = phase_artifact_digest(phase, artifact)?;
        leaves.insert(artifact.logical_path.as_str(), (digest, &artifact.role));
    }

    hasher.update(b"artifacts\x1f");
    for (logical_path, (digest, role)) in &leaves {
        // The logical path is intra-slice (group-prefix-free) and stays in the
        // key for artifact attribution; it is NOT a filesystem directory name.
        hasher.update(logical_path.as_bytes());
        hasher.update(b"\x1f");
        hasher.update(format!("{role:?}").as_bytes());
        hasher.update(b"\x1f");
        hasher.update(digest.as_bytes());
        hasher.update(b"\x1e");
    }

    Ok(hex(hasher.finalize().as_slice()))
}

// ── Merkle cache key ────────────────────────────────────────────────────────

/// A computed cache key for a (phase, unit) pair: the Merkle root plus the unit
/// members it covers (for diagnostics / attribution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheKey {
    /// The phase this key is for.
    pub phase: Phase,
    /// The slice IRIs covered by this key (one for a source unit, the SCC
    /// members for a link unit, the closure for a product unit).
    pub members: Vec<SliceIri>,
    /// The hex SHA-256 Merkle root.
    pub root: String,
}

/// Internal Merkle-root builder shared by source / link / product keys.
///
/// Folds, in deterministic order: the phase tag, the toolchain context, each
/// member slice's phase leaf digest, and — for dependency-folding phases — the
/// phase leaf digests of every transitive dependency (the upstream
/// "dependency-output digests" of RFC §12). Path-independent throughout.
fn merkle_root(
    phase: Phase,
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    members: &[SliceIri],
    toolchain: &ToolchainContext,
) -> Result<String, SliceError> {
    // The set of slices whose leaves enter the key: the members, plus their
    // dependency closure when the phase folds dependencies.
    let mut covered: BTreeSet<SliceIri> = members.iter().cloned().collect();
    if phase.folds_dependencies() {
        for dep in dependency_closure(catalog, edges, members) {
            covered.insert(dep);
        }
    }

    // Compute each covered slice's phase leaf; hard-fail on an unknown slice.
    let mut leaves: BTreeMap<SliceIri, String> = BTreeMap::new();
    for slice in &covered {
        let record = catalog.get(slice).ok_or_else(|| {
            SliceError::InvalidManifest(format!("cache key references unknown slice IRI {slice}"))
        })?;
        leaves.insert(slice.clone(), slice_phase_leaf(phase, record, toolchain)?);
    }

    let mut hasher = Sha256::new();
    hasher.update(b"merkle-root\x1f");
    hasher.update(phase.tag().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(toolchain.compiler_version.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(toolchain.reasoning_profile.as_bytes());
    hasher.update(b"\x1f");
    // Sorted by slice IRI → order-independent of catalog discovery order and of
    // the filesystem group path.
    for (slice, leaf) in &leaves {
        hasher.update(slice.as_bytes());
        hasher.update(b"\x1f");
        hasher.update(leaf.as_bytes());
        hasher.update(b"\x1e");
    }
    Ok(hex(hasher.finalize().as_slice()))
}

/// Compute the cache key for a **source unit** (one slice) at `phase`.
///
/// For dependency-folding phases the upstream closure's leaves are folded too,
/// so a change to a dependency invalidates the dependent's key (Merkle
/// composition). Path-independent: moving the slice's directory changes nothing.
pub fn source_unit_key(
    phase: Phase,
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    slice: &str,
    toolchain: &ToolchainContext,
) -> Result<CacheKey, SliceError> {
    let members = vec![slice.to_string()];
    let root = merkle_root(phase, catalog, edges, &members, toolchain)?;
    Ok(CacheKey {
        phase,
        members,
        root,
    })
}

/// Compute the cache key for a **link unit** (SCC) at `phase`. All SCC members
/// are folded together (they reason as one union), and — for dependency-folding
/// phases — the union's dependency closure as well.
pub fn link_unit_key(
    phase: Phase,
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    unit: &LinkUnit,
    toolchain: &ToolchainContext,
) -> Result<CacheKey, SliceError> {
    let mut members = unit.members.clone();
    members.sort();
    let root = merkle_root(phase, catalog, edges, &members, toolchain)?;
    Ok(CacheKey {
        phase,
        members,
        root,
    })
}

/// Compute the cache key for a **product unit** (dependency-closed profile) at
/// `phase`. The whole closure's leaves are folded (validate full composition).
pub fn product_unit_key(
    phase: Phase,
    catalog: &SliceCatalog,
    edges: &[DependencyEdge],
    unit: &ProductUnit,
    toolchain: &ToolchainContext,
) -> Result<CacheKey, SliceError> {
    // The product key covers the full closure regardless of phase folding (the
    // product *is* the composition), so we pass the closure as members.
    let root = merkle_root(phase, catalog, edges, &unit.closure, toolchain)?;
    Ok(CacheKey {
        phase,
        members: unit.closure.clone(),
        root,
    })
}

// ── Hex helper ──────────────────────────────────────────────────────────────

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests;

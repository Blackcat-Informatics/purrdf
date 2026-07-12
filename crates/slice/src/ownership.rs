// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native ownership + dependency analyzer (§10 / S4).
//!
//! This module derives an **evidence-bearing dependency graph** from validated
//! term ownership. It is the manifest-driven replacement for the path-derived
//! `slice_ownership_lint` (RFC §10): it discovers the slice IRI from the
//! manifest, attaches manifest identity as load origin, derives declared term
//! ownership from `rdfs:isDefinedBy`, compares declared ownership against
//! physical origin (which artifact file actually asserts the definition), and
//! builds dependencies only from *validated* ownership data.
//!
//! Two concerns are kept strictly separate, per the S0 Frozen Semantic Contract:
//!
//! - **Source origin** — which physical artifact file asserted an occurrence.
//! - **Semantic ownership** — which slice *declares* (via `rdfs:isDefinedBy`)
//!   that it defines a vocabulary term. Ownership is an *authored declaration*
//!   and is never trusted as physical provenance; load origin is always retained
//!   even when `rdfs:isDefinedBy` is wrong.
//!
//! Edges are classified by source-artifact role. Only the *semantic* edge kinds
//! (`Ontology`, `Shape`, `Mapping`, `Query`) reconcile against the authored
//! `purrdf:sliceDependsOn` declaration — a documentation cross-reference must
//! never silently become a build dependency (RFC §10).
//!
//! SPARQL queries are **parsed** with the native `purrdf-sparql-algebra` (not
//! text-searched) so that an IRI mentioned only inside a string literal never
//! produces a dependency edge.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use purrdf::{RdfDataset, TermId, TermRef};

use crate::artifact::{ArtifactRecord, ArtifactRole};
use crate::catalog::{SliceCatalog, SliceRecord};
use crate::error::SliceError;
use crate::rdf_query::{Dataset, NamedNode};

// ── Namespace constants ───────────────────────────────────────────────────────
//
// Only W3C terms are hardcoded; the slice-framework vocabulary (the ownership
// namespace, `sliceDependsOn`, …) comes from the catalog's caller-supplied
// [`SliceVocab`](crate::vocab::SliceVocab).

const RDFS_IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// The `rdf:type` object IRIs whose subjects are considered declared vocabulary
/// terms subject to ownership checking.  Subjects in the vocab namespace typed
/// with any of these are "declared terms" even when they have no
/// `rdfs:isDefinedBy`.
const VOCAB_TERM_TYPES: &[&str] = &[
    "http://www.w3.org/2002/07/owl#Class",
    "http://www.w3.org/2002/07/owl#ObjectProperty",
    "http://www.w3.org/2002/07/owl#DatatypeProperty",
    "http://www.w3.org/2002/07/owl#AnnotationProperty",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property",
    "http://www.w3.org/2000/01/rdf-schema#Class",
    "http://www.w3.org/2000/01/rdf-schema#Datatype",
];

/// A slice IRI (the public, persistent identity of a compilation unit). Never a
/// graph-local numeric ID (S0.5: persistent attribution serializes the public
/// slice IRI).
pub type SliceIri = String;

// ── Evidence types ────────────────────────────────────────────────────────────

/// Physical evidence: which artifact file actually asserted something, and the
/// raw content digest of that file (for content-addressed, path-independent
/// reference — S0.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEvidence {
    /// The owning slice of the artifact.
    pub slice: SliceIri,
    /// The artifact's role (module, shapes, query, …).
    pub role: ArtifactRole,
    /// The normalized logical path of the artifact within its slice
    /// (path-independent: the slice-group prefix is *not* part of this).
    pub logical_path: String,
    /// The raw SHA-256 digest of the artifact file.
    pub raw_digest: String,
}

/// Per-edge evidence: which artifact in the *from* slice referenced which term
/// owned by the *to* slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeEvidence {
    /// The artifact in the depending (from) slice that triggered this edge.
    pub from_artifact: ArtifactEvidence,
    /// The owned term (in the to-slice) that was referenced.
    pub referenced_term: NamedNode,
}

// ── Ownership types ───────────────────────────────────────────────────────────

/// The validated ownership status of a single vocabulary term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipStatus {
    /// The declared owner (`rdfs:isDefinedBy`) matches the physical origin (the
    /// artifact that defines the term).
    Validated,
    /// Multiple slices each declare `rdfs:isDefinedBy` for the term.
    Conflict(Vec<SliceIri>),
    /// No `rdfs:isDefinedBy` declaration was found for the term.
    Unowned,
    /// The declared owner and the physical origin disagree.
    Mismatch {
        /// The slice named by `rdfs:isDefinedBy`.
        declared: SliceIri,
        /// The slice whose artifact physically asserts the definition.
        physical: SliceIri,
    },
}

/// The validated ownership record for one vocabulary term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermOwnership {
    /// The vocabulary term.
    pub term: NamedNode,
    /// The slice declared as owner via `rdfs:isDefinedBy`.
    pub declared_owner: SliceIri,
    /// The artifact that *physically* asserts the defining occurrence, if any.
    pub physical_origin: Option<ArtifactEvidence>,
    /// The validated status.
    pub status: OwnershipStatus,
}

// ── Dependency types ──────────────────────────────────────────────────────────

/// The classification of a cross-slice edge by its *source artifact role*
/// (RFC §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    /// From an ontology module (`module.ttl`). Semantic.
    Ontology,
    /// From SHACL shapes (`shapes.ttl`). Semantic.
    Shape,
    /// From a mapping artifact (`mappings/`). Semantic.
    Mapping,
    /// From a SPARQL query (competency / verify). Semantic.
    Query,
    /// From a test-DSL artifact. Not semantic for reconciliation.
    Test,
    /// From an example / counter-example. Not semantic.
    Example,
    /// From documentation. Not semantic.
    Documentation,
    /// From a generated artifact. Not semantic.
    Generated,
}

impl EdgeKind {
    /// Whether this edge kind reconciles against `<vocab>sliceDependsOn`
    /// (a documentation link must not become a build dependency — RFC §10).
    pub fn is_semantic(self) -> bool {
        matches!(
            self,
            Self::Ontology | Self::Shape | Self::Mapping | Self::Query
        )
    }
}

/// How a computed edge reconciles with the authored `<vocab>sliceDependsOn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStatus {
    /// A semantic edge that *is* declared in `<vocab>sliceDependsOn`.
    Matched,
    /// A semantic edge that is *not* declared in `<vocab>sliceDependsOn`.
    Undeclared,
    /// Declared in `<vocab>sliceDependsOn` but no semantic evidence was found.
    Stale,
    /// A computed dependency edge that violates the tier model: a core slice
    /// depending on an extension, or an extension depending on another extension
    /// (Principle 16 / RFC §10).  Never produced by the analyzer itself;
    /// assigned by the analysis graph emitter after tier resolution.
    Forbidden,
}

/// A single computed cross-slice dependency edge with retained evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    /// The depending slice.
    pub from_slice: SliceIri,
    /// The depended-upon slice.
    pub to_slice: SliceIri,
    /// The artifact-role classification.
    pub edge_kind: EdgeKind,
    /// Per-reference evidence (which artifact + which term triggered the edge).
    pub evidence: Vec<EdgeEvidence>,
    /// The reconciliation verdict against `<vocab>sliceDependsOn`.
    pub reconciliation: ReconciliationStatus,
}

// ── Diagnostics ───────────────────────────────────────────────────────────────

/// A non-fatal observation produced during analysis (conflicts, mismatches,
/// stale/undeclared dependencies, unparsable queries).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipDiagnostic {
    /// A term is claimed by multiple slices.
    Conflict {
        /// The contested vocabulary term.
        term: NamedNode,
        /// Every slice declaring `rdfs:isDefinedBy` for the term.
        claimants: Vec<SliceIri>,
    },
    /// The declared owner and physical origin of a term disagree.
    Mismatch {
        /// The affected vocabulary term.
        term: NamedNode,
        /// The slice named by `rdfs:isDefinedBy`.
        declared: SliceIri,
        /// The slice whose artifact physically asserts the definition.
        physical: SliceIri,
    },
    /// A semantic edge has no authored `<vocab>sliceDependsOn` declaration.
    UndeclaredDependency {
        /// The depending slice.
        from_slice: SliceIri,
        /// The depended-upon slice.
        to_slice: SliceIri,
        /// The artifact-role classification of the undeclared edge.
        edge_kind: EdgeKind,
    },
    /// A `<vocab>sliceDependsOn` declaration has no semantic evidence.
    StaleDependency {
        /// The slice authoring the stale declaration.
        from_slice: SliceIri,
        /// The declared (but unevidenced) dependency target.
        to_slice: SliceIri,
    },
    /// A vocabulary term in the vocab namespace (typed as an OWL/RDFS concept)
    /// has no `rdfs:isDefinedBy` declaration in any slice.  Non-fatal: the term
    /// is recorded in the ownership table with `OwnershipStatus::Unowned` and
    /// surfaced as a diagnostic, but does not by itself fail validation.
    Unowned {
        /// The unowned vocabulary term.
        term: NamedNode,
    },
    /// A SPARQL query artifact failed to parse and was skipped.
    UnparseableQuery {
        /// The slice owning the query artifact.
        slice: SliceIri,
        /// The artifact's normalized logical path within its slice.
        logical_path: String,
        /// The parser's error message.
        message: String,
    },
}

/// The complete result of an ownership + dependency analysis.
#[derive(Debug, Clone)]
pub struct OwnershipReport {
    /// The validated ownership table, keyed by term IRI.
    pub ownership: HashMap<NamedNode, TermOwnership>,
    /// All computed dependency edges, with evidence and reconciliation status.
    pub edges: Vec<DependencyEdge>,
    /// All diagnostics surfaced during analysis.
    pub diagnostics: Vec<OwnershipDiagnostic>,
}

impl OwnershipReport {
    /// Whether the analysis found any ownership defect: a conflict, a mismatch,
    /// or an unowned term. (Undeclared/stale dependencies are *findings*, not
    /// ownership defects.)
    pub fn has_ownership_defect(&self) -> bool {
        self.ownership
            .values()
            .any(|o| !matches!(o.status, OwnershipStatus::Validated))
    }
}

// ── Analyzer ──────────────────────────────────────────────────────────────────

/// The native ownership + dependency analyzer.
///
/// Holds a borrow of the [`SliceCatalog`]; [`OwnershipAnalyzer::analyze`]
/// produces the full [`OwnershipReport`].
#[derive(Debug)]
pub struct OwnershipAnalyzer<'a> {
    catalog: &'a SliceCatalog,
}

impl<'a> OwnershipAnalyzer<'a> {
    /// Create an analyzer over a discovered catalog.
    pub fn new(catalog: &'a SliceCatalog) -> Self {
        Self { catalog }
    }

    /// Run the full analysis: build the validated ownership table, then derive
    /// the evidence-bearing dependency graph from *validated* ownership only.
    pub fn analyze(&self) -> Result<OwnershipReport, SliceError> {
        let mut diagnostics = Vec::new();
        let mut rdf_facts: HashMap<(usize, usize), RdfArtifactFacts> = HashMap::new();

        // ── Phase 1: declared ownership (rdfs:isDefinedBy), per slice ────────
        //
        // `claims[term]` = set of slices declaring `term rdfs:isDefinedBy slice`.
        // `physical[term]` = the artifact that physically asserts the
        // isDefinedBy occurrence (its raw load origin, per RFC §10 step 2/4).
        let mut claims: BTreeMap<NamedNode, BTreeSet<SliceIri>> = BTreeMap::new();
        let mut physical: BTreeMap<NamedNode, ArtifactEvidence> = BTreeMap::new();
        // `declared_terms` = every PurRDF subject typed as an OWL/RDFS vocabulary
        // construct (owl:Class, owl:ObjectProperty, …) in any ownership-bearing
        // artifact.  Terms here but absent from `claims` have no rdfs:isDefinedBy
        // and are emitted as OwnershipStatus::Unowned.
        let mut declared_terms: BTreeSet<NamedNode> = BTreeSet::new();

        for (record_index, record) in self.catalog.records().iter().enumerate() {
            let slice_iri = record.manifest.slice_iri.clone();
            for (artifact_index, artifact) in record.artifacts.iter().enumerate() {
                // Only ontology/shape RDF artifacts declare ownership.
                if !is_ownership_bearing(&artifact.role) {
                    continue;
                }
                let store = parse_rdf_artifact(artifact)?;
                let facts = inspect_rdf_dataset(store.inner(), self.catalog.vocab().ns());
                // Collect rdfs:isDefinedBy claims.
                for (subject, owner) in &facts.is_defined_by {
                    claims
                        .entry(subject.clone())
                        .or_default()
                        .insert(owner.clone());
                    // Physical origin = the artifact that carries this triple,
                    // keyed once (first wins — artifacts iterate in sorted
                    // logical-path order for determinism).
                    physical
                        .entry(subject.clone())
                        .or_insert_with(|| ArtifactEvidence {
                            slice: slice_iri.clone(),
                            role: artifact.role.clone(),
                            logical_path: artifact.logical_path.clone(),
                            raw_digest: artifact.raw_digest.clone(),
                        });
                }
                // Collect declared vocabulary terms (typed subjects in the
                // caller's vocab namespace).
                declared_terms.extend(facts.declared_terms.iter().cloned());
                rdf_facts.insert((record_index, artifact_index), facts);
            }
        }

        // ── Phase 2: validate ownership ─────────────────────────────────────
        //
        // Iterate over the UNION of `declared_terms` (typed as OWL/RDFS vocab
        // constructs) and `claims` (terms with rdfs:isDefinedBy).  Terms that
        // are declared but have no rdfs:isDefinedBy yield OwnershipStatus::Unowned.
        let mut ownership: HashMap<NamedNode, TermOwnership> = HashMap::new();
        let all_terms: BTreeSet<NamedNode> = declared_terms
            .iter()
            .chain(claims.keys())
            .cloned()
            .collect();
        for term in &all_terms {
            let owners_opt = claims.get(term);
            if let Some(owners) = owners_opt {
                // Term has at least one rdfs:isDefinedBy claim.
                let owners_vec: Vec<SliceIri> = owners.iter().cloned().collect();
                let physical_origin = physical.get(term).cloned();
                let declared_owner = owners_vec.first().cloned().unwrap_or_default();

                let status = if owners_vec.len() > 1 {
                    diagnostics.push(OwnershipDiagnostic::Conflict {
                        term: term.clone(),
                        claimants: owners_vec.clone(),
                    });
                    OwnershipStatus::Conflict(owners_vec.clone())
                } else {
                    match &physical_origin {
                        Some(origin) if origin.slice == declared_owner => {
                            OwnershipStatus::Validated
                        }
                        Some(origin) => {
                            diagnostics.push(OwnershipDiagnostic::Mismatch {
                                term: term.clone(),
                                declared: declared_owner.clone(),
                                physical: origin.slice.clone(),
                            });
                            OwnershipStatus::Mismatch {
                                declared: declared_owner.clone(),
                                physical: origin.slice.clone(),
                            }
                        }
                        // Both maps were populated in lockstep from isDefinedBy
                        // triples so `physical` is always Some here; the None arm
                        // is structurally unreachable but kept for exhaustiveness.
                        None => OwnershipStatus::Unowned,
                    }
                };

                ownership.insert(
                    term.clone(),
                    TermOwnership {
                        term: term.clone(),
                        declared_owner,
                        physical_origin,
                        status,
                    },
                );
            } else {
                // Term is declared (typed as OWL/RDFS construct) but has NO
                // rdfs:isDefinedBy in any slice — genuinely Unowned.
                diagnostics.push(OwnershipDiagnostic::Unowned { term: term.clone() });
                ownership.insert(
                    term.clone(),
                    TermOwnership {
                        term: term.clone(),
                        declared_owner: String::new(),
                        physical_origin: None,
                        status: OwnershipStatus::Unowned,
                    },
                );
            }
        }

        // ── Phase 3: validated owner map (only Validated terms drive deps) ──
        //
        // RFC §10 step 5: build dependencies only from *validated* ownership
        // data. A conflicted / mismatched / unowned term contributes no edge.
        let mut validated_owner: HashMap<NamedNode, SliceIri> = HashMap::new();
        for (term, rec) in &ownership {
            if matches!(rec.status, OwnershipStatus::Validated) {
                validated_owner.insert(term.clone(), rec.declared_owner.clone());
            }
        }

        // ── Phase 4: authored declarations (<vocab>sliceDependsOn) ───────────
        let mut declared_deps: BTreeMap<SliceIri, BTreeSet<SliceIri>> = BTreeMap::new();
        for record in self.catalog.records() {
            let from = record.manifest.slice_iri.clone();
            let targets = collect_slice_depends_on(record);
            if !targets.is_empty() {
                declared_deps.entry(from).or_default().extend(targets);
            }
        }

        // ── Phase 5: computed edges from artifact references ─────────────────
        //
        // `edge_evidence[(from, to, kind)]` accumulates the per-reference
        // evidence. We dedup evidence entries to keep the report compact.
        let mut edge_evidence: BTreeMap<(SliceIri, SliceIri, EdgeKind), Vec<EdgeEvidence>> =
            BTreeMap::new();

        for (record_index, record) in self.catalog.records().iter().enumerate() {
            let from = record.manifest.slice_iri.clone();
            for (artifact_index, artifact) in record.artifacts.iter().enumerate() {
                let Some(kind) = edge_kind_for_role(&artifact.role) else {
                    continue;
                };
                let from_evidence = ArtifactEvidence {
                    slice: from.clone(),
                    role: artifact.role.clone(),
                    logical_path: artifact.logical_path.clone(),
                    raw_digest: artifact.raw_digest.clone(),
                };

                if kind == EdgeKind::Query {
                    let referenced = match extract_query_iris(artifact) {
                        Ok(set) => set,
                        Err(message) => {
                            diagnostics.push(OwnershipDiagnostic::UnparseableQuery {
                                slice: from.clone(),
                                logical_path: artifact.logical_path.clone(),
                                message,
                            });
                            continue;
                        }
                    };
                    collect_reference_evidence(
                        referenced.iter(),
                        &validated_owner,
                        &from,
                        kind,
                        &from_evidence,
                        &mut edge_evidence,
                    );
                    continue;
                }

                let key = (record_index, artifact_index);
                if let std::collections::hash_map::Entry::Vacant(entry) = rdf_facts.entry(key) {
                    let Ok(store) = parse_rdf_artifact(artifact) else {
                        continue;
                    };
                    entry.insert(inspect_rdf_dataset(
                        store.inner(),
                        self.catalog.vocab().ns(),
                    ));
                }
                let referenced = &rdf_facts[&key].referenced_iris;
                collect_reference_evidence(
                    referenced.iter(),
                    &validated_owner,
                    &from,
                    kind,
                    &from_evidence,
                    &mut edge_evidence,
                );
            }
        }

        // ── Phase 6: reconcile against authored sliceDependsOn ──────────────
        let mut edges: Vec<DependencyEdge> = Vec::new();
        // Track which (from,to) pairs got a semantic edge, for stale detection.
        let mut semantic_pairs: BTreeSet<(SliceIri, SliceIri)> = BTreeSet::new();

        for ((from, to, kind), mut evidence) in edge_evidence {
            evidence.sort_by(|a, b| {
                a.referenced_term
                    .as_str()
                    .cmp(b.referenced_term.as_str())
                    .then_with(|| {
                        a.from_artifact
                            .logical_path
                            .cmp(&b.from_artifact.logical_path)
                    })
            });

            let declared = declared_deps.get(&from).is_some_and(|s| s.contains(&to));

            let reconciliation = if !kind.is_semantic() {
                // Non-semantic edges never reconcile; they are evidence-only.
                ReconciliationStatus::Undeclared
            } else {
                semantic_pairs.insert((from.clone(), to.clone()));
                if declared {
                    ReconciliationStatus::Matched
                } else {
                    diagnostics.push(OwnershipDiagnostic::UndeclaredDependency {
                        from_slice: from.clone(),
                        to_slice: to.clone(),
                        edge_kind: kind,
                    });
                    ReconciliationStatus::Undeclared
                }
            };

            edges.push(DependencyEdge {
                from_slice: from,
                to_slice: to,
                edge_kind: kind,
                evidence,
                reconciliation,
            });
        }

        // ── Phase 7: stale declarations (declared, no semantic evidence) ────
        //
        // Emitted as synthetic Stale edges (no evidence) so the dependency graph
        // surfaces them, plus a diagnostic.
        for (from, targets) in &declared_deps {
            for to in targets {
                if !semantic_pairs.contains(&(from.clone(), to.clone())) {
                    diagnostics.push(OwnershipDiagnostic::StaleDependency {
                        from_slice: from.clone(),
                        to_slice: to.clone(),
                    });
                    edges.push(DependencyEdge {
                        from_slice: from.clone(),
                        to_slice: to.clone(),
                        edge_kind: EdgeKind::Ontology,
                        evidence: Vec::new(),
                        reconciliation: ReconciliationStatus::Stale,
                    });
                }
            }
        }

        // Deterministic edge ordering.
        edges.sort_by(|a, b| {
            a.from_slice
                .cmp(&b.from_slice)
                .then_with(|| a.to_slice.cmp(&b.to_slice))
                .then_with(|| a.edge_kind.cmp(&b.edge_kind))
        });

        Ok(OwnershipReport {
            ownership,
            edges,
            diagnostics,
        })
    }
}

fn collect_reference_evidence<'a>(
    referenced: impl Iterator<Item = &'a NamedNode>,
    validated_owner: &HashMap<NamedNode, SliceIri>,
    from: &SliceIri,
    kind: EdgeKind,
    from_evidence: &ArtifactEvidence,
    edge_evidence: &mut BTreeMap<(SliceIri, SliceIri, EdgeKind), Vec<EdgeEvidence>>,
) {
    for term in referenced {
        let Some(owner) = validated_owner.get(term) else {
            continue;
        };
        // No self-edges: a slice does not depend on itself.
        if owner == from {
            continue;
        }
        let key = (from.clone(), owner.clone(), kind);
        let ev = EdgeEvidence {
            from_artifact: from_evidence.clone(),
            referenced_term: term.clone(),
        };
        let bucket = edge_evidence.entry(key).or_default();
        if !bucket.contains(&ev) {
            bucket.push(ev);
        }
    }
}

// ── Role classification ───────────────────────────────────────────────────────

/// Whether an artifact role declares term ownership (`rdfs:isDefinedBy`).
fn is_ownership_bearing(role: &ArtifactRole) -> bool {
    matches!(role, ArtifactRole::Module | ArtifactRole::Shapes)
}

/// Map an artifact role to its dependency-edge kind, or `None` if the role
/// produces no dependency edges (the manifest, citation, translations).
fn edge_kind_for_role(role: &ArtifactRole) -> Option<EdgeKind> {
    match role {
        ArtifactRole::Module => Some(EdgeKind::Ontology),
        ArtifactRole::Shapes => Some(EdgeKind::Shape),
        ArtifactRole::Mapping => Some(EdgeKind::Mapping),
        ArtifactRole::CompetencyQuery | ArtifactRole::VerifyQuery => Some(EdgeKind::Query),
        ArtifactRole::TestDsl | ArtifactRole::CounterExample => Some(EdgeKind::Test),
        ArtifactRole::Example => Some(EdgeKind::Example),
        ArtifactRole::Documentation => Some(EdgeKind::Documentation),
        // Manifest, Citation, TranslationCatalog, Other → no edges.
        ArtifactRole::Manifest | ArtifactRole::Citation | ArtifactRole::TranslationCatalog => None,
        ArtifactRole::Other(_) => None,
    }
}

// ── RDF helpers ───────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct RdfArtifactFacts {
    is_defined_by: Vec<(NamedNode, SliceIri)>,
    declared_terms: BTreeSet<NamedNode>,
    referenced_iris: BTreeSet<NamedNode>,
}

/// Parse an RDF artifact's bytes into a native dataset (lenient for `@x-purrdf-*`
/// language tags). Hard-fails on a syntax error.
fn parse_rdf_artifact(artifact: &ArtifactRecord) -> Result<Dataset, SliceError> {
    // A malformed ownership-bearing artifact must FAIL LOUDLY, never be silently
    // dropped — a swallowed parse error would hide a term from the
    // one-validated-owner gate and miscompute the dependency graph
    // (no-optionality / hard-fail doctrine).
    Dataset::parse_turtle(artifact.content.as_slice(), &artifact.logical_path)
}

/// Inspect one parsed RDF artifact once. Borrowed IRI slices are deduplicated
/// while walking the interned IR, then only the unique retained values are owned.
fn inspect_rdf_dataset(store: &RdfDataset, vocab_ns: &str) -> RdfArtifactFacts {
    let mut is_defined_by: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut declared_terms: BTreeSet<&str> = BTreeSet::new();
    let mut referenced_iris: BTreeSet<&str> = BTreeSet::new();

    for quad in store.quads() {
        collect_term_iri_refs(store, quad.s, &mut referenced_iris);
        collect_term_iri_refs(store, quad.p, &mut referenced_iris);
        collect_term_iri_refs(store, quad.o, &mut referenced_iris);
        if let Some(graph) = quad.g
            && let TermRef::Iri(iri) = store.resolve(graph)
        {
            referenced_iris.insert(iri);
        }

        let (TermRef::Iri(subject), TermRef::Iri(predicate), TermRef::Iri(object)) = (
            store.resolve(quad.s),
            store.resolve(quad.p),
            store.resolve(quad.o),
        ) else {
            continue;
        };

        if predicate == RDFS_IS_DEFINED_BY && subject.starts_with(vocab_ns) {
            is_defined_by.insert((subject, object));
        }
        if predicate == RDF_TYPE
            && subject.starts_with(vocab_ns)
            && VOCAB_TERM_TYPES.contains(&object)
        {
            declared_terms.insert(subject);
        }
    }

    RdfArtifactFacts {
        is_defined_by: is_defined_by
            .into_iter()
            .map(|(subject, owner)| {
                (
                    NamedNode::new_unchecked(subject.to_owned()),
                    owner.to_owned(),
                )
            })
            .collect(),
        declared_terms: declared_terms
            .into_iter()
            .map(|term| NamedNode::new_unchecked(term.to_owned()))
            .collect(),
        referenced_iris: referenced_iris
            .into_iter()
            .map(|term| NamedNode::new_unchecked(term.to_owned()))
            .collect(),
    }
}

/// Collect the `<vocab>sliceDependsOn` targets declared in a slice's manifest,
/// scoped to the manifest's own slice subject ONLY. A `sliceDependsOn` triple
/// whose subject is some *other* resource in the manifest (e.g. a blank-node
/// description or an unrelated IRI) is never picked up — only edges authored on
/// the slice itself reconcile against computed dependencies (G8 MED).
fn collect_slice_depends_on(record: &SliceRecord) -> BTreeSet<SliceIri> {
    record.manifest.depends_on.iter().cloned().collect()
}

/// Extract every NamedNode IRI that appears anywhere in an RDF artifact:
/// subject, predicate, object, datatype IRI, graph name, and nested
/// triple-term components (RFC §10). Literal lexical forms are *not* mined for
/// IRIs — only the datatype IRI of a literal counts.
///
/// Mine every NamedNode IRI from one term: an IRI itself, a literal's expanded
/// datatype IRI (the lexical form is NOT mined), and a quoted triple's components.
/// A blank node contributes no IRI. The frozen IR always expands a literal's
/// datatype (C0.1), so a plain `xsd:string` / `rdf:langString` literal mines the
/// expanded datatype exactly as oxigraph's `lit.datatype()` did.
fn collect_term_iri_refs<'a>(store: &'a RdfDataset, term: TermId, out: &mut BTreeSet<&'a str>) {
    match store.resolve(term) {
        TermRef::Iri(iri) => {
            out.insert(iri);
        }
        TermRef::Blank { .. } => {}
        TermRef::Literal { datatype, .. } => {
            collect_term_iri_refs(store, datatype, out);
        }
        TermRef::Triple { s, p, o } => {
            collect_term_iri_refs(store, s, out);
            collect_term_iri_refs(store, p, out);
            collect_term_iri_refs(store, o, out);
        }
    }
}

// ── SPARQL helpers (parsed, never text-searched) ──────────────────────────────

/// Parse a SPARQL query artifact and extract every NamedNode IRI it references
/// in *term position* (subjects/predicates/objects/paths/functions/datatypes),
/// using the native `purrdf-sparql-algebra` parser. An IRI mentioned only inside
/// a string literal is never returned — that is the whole point of parsing
/// rather than text-searching.
fn extract_query_iris(artifact: &ArtifactRecord) -> Result<BTreeSet<NamedNode>, String> {
    use purrdf_sparql_algebra::SparqlParser;

    let text = std::str::from_utf8(&artifact.content)
        .map_err(|e| format!("query is not valid UTF-8: {e}"))?;
    let query = SparqlParser::new()
        .parse_query(text)
        .map_err(|e| e.to_string())?;

    let mut out: BTreeSet<NamedNode> = BTreeSet::new();
    match &query {
        purrdf_sparql_algebra::Query::Select { pattern, .. }
        | purrdf_sparql_algebra::Query::Ask { pattern, .. } => {
            walk_graph_pattern(pattern, &mut out);
        }
        purrdf_sparql_algebra::Query::Describe {
            pattern, targets, ..
        } => {
            // DESCRIBE <iri> carries dependency IRIs in `targets`; the pattern
            // may be the empty unit pattern, so walking only `pattern` would
            // drop the described-resource edge entirely.
            for target in targets {
                walk_named_node_pattern(target, &mut out);
            }
            walk_graph_pattern(pattern, &mut out);
        }
        purrdf_sparql_algebra::Query::Construct {
            template, pattern, ..
        } => {
            for tp in template {
                walk_triple_pattern(tp, &mut out);
            }
            walk_graph_pattern(pattern, &mut out);
        }
    }
    Ok(out)
}

fn insert_oxiri(node: &purrdf_sparql_algebra::NamedNode, out: &mut BTreeSet<NamedNode>) {
    if let Ok(nn) = NamedNode::new(node.as_str()) {
        out.insert(nn);
    }
}

fn walk_named_node_pattern(
    p: &purrdf_sparql_algebra::NamedNodePattern,
    out: &mut BTreeSet<NamedNode>,
) {
    if let purrdf_sparql_algebra::NamedNodePattern::NamedNode(n) = p {
        insert_oxiri(n, out);
    }
}

fn walk_term_pattern(p: &purrdf_sparql_algebra::TermPattern, out: &mut BTreeSet<NamedNode>) {
    match p {
        purrdf_sparql_algebra::TermPattern::NamedNode(n) => insert_oxiri(n, out),
        purrdf_sparql_algebra::TermPattern::Literal(lit) => {
            // Only the datatype IRI counts, never the lexical form.
            insert_oxiri(&literal_datatype(lit), out);
        }
        purrdf_sparql_algebra::TermPattern::Triple(t) => walk_triple_pattern(t, out),
        purrdf_sparql_algebra::TermPattern::BlankNode(_)
        | purrdf_sparql_algebra::TermPattern::Variable(_) => {}
    }
}

/// A SPARQL `Literal` exposes its datatype; clone the NamedNode from it.
fn literal_datatype(lit: &purrdf_sparql_algebra::Literal) -> purrdf_sparql_algebra::NamedNode {
    lit.datatype().clone()
}

fn walk_triple_pattern(t: &purrdf_sparql_algebra::TriplePattern, out: &mut BTreeSet<NamedNode>) {
    walk_term_pattern(&t.subject, out);
    walk_named_node_pattern(&t.predicate, out);
    walk_term_pattern(&t.object, out);
}

fn walk_path(p: &purrdf_sparql_algebra::PropertyPathExpression, out: &mut BTreeSet<NamedNode>) {
    use purrdf_sparql_algebra::PropertyPathExpression as P;
    match p {
        P::NamedNode(n) => insert_oxiri(n, out),
        P::Reverse(a) | P::ZeroOrMore(a) | P::OneOrMore(a) | P::ZeroOrOne(a) => walk_path(a, out),
        P::Range { inner, .. } => walk_path(inner, out),
        P::Sequence(a, b) | P::Alternative(a, b) => {
            walk_path(a, out);
            walk_path(b, out);
        }
        P::NegatedPropertySet(elems) => {
            for e in elems {
                insert_oxiri(&e.predicate, out);
            }
        }
        // A predicate wildcard references no named predicate to collect.
        P::Wildcard { .. } => {}
    }
}

fn walk_expression(e: &purrdf_sparql_algebra::Expression, out: &mut BTreeSet<NamedNode>) {
    use purrdf_sparql_algebra::Expression as E;
    match e {
        E::NamedNode(n) => insert_oxiri(n, out),
        // A literal in an expression (e.g. a FILTER comparison string) is NOT a
        // term reference; only its datatype IRI is.
        E::Literal(lit) => insert_oxiri(&literal_datatype(lit), out),
        E::Variable(_) | E::Bound(_) => {}
        E::Or(a, b)
        | E::And(a, b)
        | E::Equal(a, b)
        | E::SameTerm(a, b)
        | E::Greater(a, b)
        | E::GreaterOrEqual(a, b)
        | E::Less(a, b)
        | E::LessOrEqual(a, b)
        | E::Add(a, b)
        | E::Subtract(a, b)
        | E::Multiply(a, b)
        | E::Divide(a, b) => {
            walk_expression(a, out);
            walk_expression(b, out);
        }
        E::UnaryPlus(a) | E::UnaryMinus(a) | E::Not(a) => walk_expression(a, out),
        E::In(a, list) => {
            walk_expression(a, out);
            for x in list {
                walk_expression(x, out);
            }
        }
        E::If(a, b, c) => {
            walk_expression(a, out);
            walk_expression(b, out);
            walk_expression(c, out);
        }
        E::Coalesce(list) => {
            for x in list {
                walk_expression(x, out);
            }
        }
        E::FunctionCall(func, args) => {
            match func {
                // An IRI-named external function references the slice defining it.
                purrdf_sparql_algebra::Function::Custom(n) => insert_oxiri(n, out),
                // A recognized extension function (e.g. heldIn) depends on the
                // slice that declares its vocabulary term. The parsed call keeps
                // the ORIGINAL IRI from the query text (the extension namespace
                // is caller configuration — purrdf mints no vocabulary), so the
                // dependency edge uses that IRI verbatim.
                purrdf_sparql_algebra::Function::Purrdf(call) => {
                    if let Ok(nn) = NamedNode::new(call.iri.clone()) {
                        out.insert(nn);
                    }
                }
                _ => {}
            }
            for x in args {
                walk_expression(x, out);
            }
        }
        E::Exists(pattern) => walk_graph_pattern(pattern, out),
    }
}

fn walk_graph_pattern(g: &purrdf_sparql_algebra::GraphPattern, out: &mut BTreeSet<NamedNode>) {
    use purrdf_sparql_algebra::GraphPattern as G;
    match g {
        G::Bgp { patterns } => {
            for tp in patterns {
                walk_triple_pattern(tp, out);
            }
        }
        G::Path {
            subject,
            path,
            object,
        } => {
            walk_term_pattern(subject, out);
            walk_path(path, out);
            walk_term_pattern(object, out);
        }
        G::Join { left, right }
        | G::Union { left, right }
        | G::Lateral { left, right }
        | G::Minus { left, right } => {
            walk_graph_pattern(left, out);
            walk_graph_pattern(right, out);
        }
        G::LeftJoin {
            left,
            right,
            expression,
        } => {
            walk_graph_pattern(left, out);
            walk_graph_pattern(right, out);
            if let Some(expr) = expression {
                walk_expression(expr, out);
            }
        }
        G::Filter { expr, inner } => {
            walk_expression(expr, out);
            walk_graph_pattern(inner, out);
        }
        G::Graph { name, inner } => {
            walk_named_node_pattern(name, out);
            walk_graph_pattern(inner, out);
        }
        G::Extend {
            inner, expression, ..
        } => {
            walk_graph_pattern(inner, out);
            walk_expression(expression, out);
        }
        G::Service { name, inner, .. } => {
            walk_named_node_pattern(name, out);
            walk_graph_pattern(inner, out);
        }
        G::OrderBy { inner, expression } => {
            walk_graph_pattern(inner, out);
            // Sort keys can carry IRI-bearing data (custom functions, IRI
            // constants); walk each ORDER BY expression, not just `inner`.
            for order in expression {
                use purrdf_sparql_algebra::OrderExpression as OE;
                match order {
                    OE::Asc(e) | OE::Desc(e) => walk_expression(e, out),
                }
            }
        }
        G::Project { inner, .. }
        | G::Distinct { inner }
        | G::Reduced { inner }
        | G::Slice { inner, .. } => walk_graph_pattern(inner, out),
        G::Group {
            inner, aggregates, ..
        } => {
            walk_graph_pattern(inner, out);
            for (_var, agg_expr) in aggregates {
                use purrdf_sparql_algebra::AggregateExpression as AE;
                use purrdf_sparql_algebra::AggregateFunction as AF;
                match agg_expr {
                    AE::CountStar { .. } => {}
                    AE::FunctionCall {
                        function,
                        expression,
                        ..
                    } => {
                        if let AF::Custom(n) = function {
                            insert_oxiri(n, out);
                        }
                        walk_expression(expression, out);
                    }
                }
            }
        }
        G::Values { bindings, .. } => {
            for row in bindings {
                for cell in row.iter().flatten() {
                    walk_ground_term(cell, out);
                }
            }
        }
    }
}

/// Walk a `VALUES` ground term, recursing into RDF 1.2 ground quoted triples so
/// IRIs inside `<<( s p o )>>` cells become dependency edges too.
fn walk_ground_term(t: &purrdf_sparql_algebra::GroundTerm, out: &mut BTreeSet<NamedNode>) {
    use purrdf_sparql_algebra::GroundTerm as GT;
    match t {
        GT::NamedNode(n) => insert_oxiri(n, out),
        GT::Literal(lit) => insert_oxiri(&literal_datatype(lit), out),
        GT::Triple(tri) => {
            walk_ground_term(&tri.subject, out);
            insert_oxiri(&tri.predicate, out);
            walk_ground_term(&tri.object, out);
        }
        // Injection-only variant (native `$this` substitution): never produced by
        // the parser, so it cannot appear in a VALUES clause, and a blank node
        // carries no IRI dependency edge — a no-op.
        GT::BlankNode(_) => {}
    }
}

#[cfg(test)]
mod rdf_fact_tests {
    use super::*;

    const EX: &str = "https://example.org/vocab/";

    #[test]
    fn one_ir_walk_collects_ownership_and_nested_rdf12_references() {
        let input = format!(
            "@prefix ex: <{EX}> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             ex:term a owl:Class ; rdfs:isDefinedBy ex:slice .\n\
             ex:record ex:mentions <<( ex:term ex:edge \"value\"^^ex:datatype )>> .\n"
        );
        let store = Dataset::parse_turtle(input.as_bytes(), "facts.ttl").expect("valid RDF 1.2");
        let facts = inspect_rdf_dataset(store.inner(), EX);

        assert_eq!(
            facts.is_defined_by,
            vec![(
                NamedNode::new_unchecked(format!("{EX}term")),
                format!("{EX}slice")
            )]
        );
        assert!(
            facts
                .declared_terms
                .contains(&NamedNode::new_unchecked(format!("{EX}term")))
        );
        for local in ["term", "slice", "record", "mentions", "edge", "datatype"] {
            assert!(
                facts
                    .referenced_iris
                    .contains(&NamedNode::new_unchecked(format!("{EX}{local}"))),
                "missing {local}"
            );
        }
    }
}

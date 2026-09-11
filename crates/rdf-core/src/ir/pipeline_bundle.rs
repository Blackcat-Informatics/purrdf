// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The pipeline carriers: the frozen hot graph (or a shared VIEW of one), its
//! out-of-band lookaside, the content-addressed blob store, the provenance sidecar,
//! and a typed-handle lane.
//!
//! Two carriers travel the same pipeline and are interchangeable by content:
//!
//! * [`PipelineBundle`] owns one frozen [`RdfDataset`]. Folding a graph into it
//!   unions a NEW dataset — a deep copy, paid once per accumulation.
//! * [`PipelineViewBundle`] owns an [`Arc<CompositeDatasetView>`]. Folding a graph
//!   into it APPENDS a retained source
//!   ([`CompositeDatasetView::extend`]) — no rows are copied and nothing is frozen,
//!   and the bases it retains are reported once to a shared [`RetentionLedger`]
//!   however many carriers hold them.
//!
//! Both delegate their whole non-dataset state — lookaside, blobs, provenance,
//! typed handles, the digest fold, the per-graph memo and the pin check — to ONE
//! internal implementation, so the two carriers cannot drift into disagreeing about
//! identity. Equal content yields equal [`digest`](PipelineBundle::digest), equal
//! per-graph digests and equal [`pipeline_root`](PipelineBundle::pipeline_root) on
//! either carrier.
//!
//! ## Kernel boundary
//!
//! The kernel owns the bundle SHAPE but NOT the concrete handle payloads. The
//! payload type `H` is generic so that pipeline-side types (logic programs,
//! rendered docs, reasoning results) never enter `purrdf-core` — the
//! oxigraph-free / PyO3-free ring-fence stays intact. A handle bundles its payload
//! with a PINNED [`ContentDigest`] of the named graph it projects.
//!
//! ## Content addressing
//!
//! [`PipelineBundle::digest`] is a SHA-256 fold over, in a fixed order:
//! 1. the canonical N-Quads hash of the dataset ([`canonicalize`]),
//! 2. each lookaside resource's `content_digest` (collected and SORTED),
//! 3. each blob's [`ContentDigest`] in the store (SORTED),
//! 4. the provenance's runtime-id-free PUBLIC projection
//!    ([`DatasetProvenance::public_projection`], S0.5).
//!
//! [`PipelineViewBundle::digest`] folds exactly the same four sections, reading
//! section 1 through [`try_canonicalize_view`] instead of [`canonicalize`]. A
//! composite view and the flat dataset holding the same content canonicalize
//! byte-identically, so the two folds agree by construction rather than by
//! coincidence.
//!
//! The typed-handle lane contributes NOTHING to the digest: attaching or detaching
//! a handle leaves [`digest`](PipelineBundle::digest) byte-stable. This is the
//! contract a downstream cache keys on — the dataset/lookaside/blobs/public-
//! provenance are the content, the handles are derived views over it.
//!
//! ## The pipeline root — an ADDITIONAL identity, never a substitute
//!
//! [`pipeline_root`](PipelineBundle::pipeline_root) is a second, independently
//! versioned identity for STAGE CACHING: a domain-separated fold over the carrier's
//! per-graph leaves rather than over one whole-dataset canonical document. It exists
//! so a scheduler can see WHICH leaf moved when a carrier changes, which a single
//! whole-dataset digest cannot express. It is NEVER a substitute for
//! [`digest`](PipelineBundle::digest): the root addresses graphs by IRI, so content
//! a per-graph leaf cannot name (see [`PIPELINE_ROOT_DOMAIN`]) reaches it only
//! through the residue leaf, and the two identities are pinned together or not at
//! all.
//!
//! ## Pin invariant (hard-fail)
//!
//! Attaching a handle ALWAYS checks that its pinned digest equals the canonical
//! digest of its backing named graph; a mismatch is a HARD failure
//! ([`PipelineBundleError::HandleDigestMismatch`]). The check runs on every attach,
//! not only in tests, so a bundle can never carry a handle that disagrees with the
//! graph it claims to project. Concrete pipeline-side handle types plug into this
//! lane unchanged.
//!
//! ## Containment invariant (hard-fail)
//!
//! [`accumulate_named_graph`](PipelineBundle::accumulate_named_graph) folds ONE
//! named graph in. Every quad, reifier declaration, annotation row and named-graph
//! declaration of the contribution must therefore carry that graph in its own graph
//! slot; anything else is refused with
//! [`PipelineBundleError::GraphContainment`]. The rule is not decoration. On the
//! view carrier the contribution is placed with
//! [`GraphPlacement::Named`], which REWRITES every graph slot it is given, so an
//! unchecked contribution would be silently relocated and the two carriers would
//! stop agreeing; on either carrier it is what makes the exact per-graph
//! invalidation below sound.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use super::canon::{
    CanonError, CanonHash, canonicalize, canonicalize_view, try_canonicalize_view,
    try_graph_digest_view,
};
use super::composite::{CompositeDatasetView, CompositeSource, GraphPlacement, owned_value};
use super::dataset::RdfDataset;
use super::mutable::DeltaDatasetView;
use super::view_accounting::{
    RetentionGuard, RetentionLedger, ViewAccountingReport, ViewLimits, ViewStats,
};
use crate::dataset_view::{DatasetView, GraphMatch};
use crate::provenance::DatasetProvenance;
use crate::{
    ContentDigest, ContentStore, QuadIds, QuadRef, RdfDiagnostic, RdfLookaside,
    RdfStoreCapabilities, TermRef, TermValue,
};

/// Field separator inside the digest fold (mirrors `StageProduct::from_artifacts`).
const SEP_FIELD: u8 = 0x1f;
/// Record separator inside the digest fold.
const SEP_RECORD: u8 = 0x1e;
/// Section separator between the four digest contributions.
const SEP_SECTION: u8 = 0x1d;

/// The versioned domain-separation tag hashed as the FIRST bytes of every
/// [`pipeline_root`](PipelineBundle::pipeline_root).
///
/// A bare token, deliberately not an IRI: nothing may dereference it, assert with
/// it, or mistake it for a vocabulary this project does not publish. It is present
/// so the root can never collide with [`digest`](PipelineBundle::digest) or with a
/// later root layout, and so a consumer pinning a root is pinning WHICH fold
/// produced it.
///
/// The version suffix moves whenever the bytes a given carrier folds would move:
/// the leaf set, the leaf order, the residue rule, or the sidecar sections. It does
/// not move for a refactor that cannot change output.
///
/// ## What the fold covers
///
/// 1. this tag,
/// 2. the IRI-named graphs of the carrier — every graph it addresses, quad-bearing
///    or declared empty — each as `(graph IRI bytes, per-graph canonical digest)`,
///    sorted by IRI,
/// 3. the RESIDUE digest: the canonical form of every row whose graph slot is the
///    default graph or a BLANK node graph name, that slot preserved. A per-graph
///    leaf is addressed by IRI, so those rows have no leaf of their own; folding
///    them as one residue is what keeps the root a function of the whole carrier
///    rather than of its IRI-addressable part.
/// 4. the same lookaside, blob and public-provenance sections
///    [`digest`](PipelineBundle::digest) folds, byte for byte.
///
/// A named graph that is DECLARED but holds no row contributes its IRI and the
/// canonical digest of the empty document. This is strictly more discriminating
/// than [`digest`](PipelineBundle::digest), whose canonical document has no place
/// to record an empty declaration at all. A declaration-only graph whose name is a
/// BLANK node is invisible to both, for the same reason: it owns no row, and its
/// name is not addressable.
pub const PIPELINE_ROOT_DOMAIN: &str = "purrdf.pipeline-root.v1";

/// The key identifying the named graph a typed handle backs. An IRI string is the
/// stable, dataset-independent name of the graph the handle projects.
pub type HandleKey = String;

/// Which of a dataset's three RDF tables — or its named-graph declarations — a
/// containment violation was found in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphLayer {
    /// An ordinary quad.
    Quad,
    /// An RDF 1.2 reifier declaration row.
    Reifier,
    /// An RDF 1.2 statement annotation row.
    Annotation,
    /// A named-graph declaration (a graph the dataset addresses, whether or not it
    /// holds rows).
    Declaration,
}

impl fmt::Display for GraphLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Quad => "quad",
            Self::Reifier => "reifier declaration",
            Self::Annotation => "statement annotation",
            Self::Declaration => "named-graph declaration",
        })
    }
}

/// Where a canonicalization refusal was raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonScopeName {
    /// The whole carrier, for [`digest`](PipelineViewBundle::digest).
    Carrier,
    /// One named graph, for [`graph_digest`](PipelineViewBundle::graph_digest).
    Graph(HandleKey),
    /// The pipeline root's residue leaf — the rows no IRI-named leaf addresses.
    Residue,
}

impl fmt::Display for CanonScopeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Carrier => f.write_str("the carrier"),
            Self::Graph(graph) => write!(f, "named graph <{graph}>"),
            Self::Residue => f.write_str("the pipeline-root residue"),
        }
    }
}

/// A typed handle: a pipeline-side payload `H` paired with the PINNED
/// [`ContentDigest`] of the named graph it projects.
///
/// The digest is checked against the backing graph on every attach
/// ([`PipelineBundle::pin_handle`]); a `HandleEntry` in a constructed bundle is
/// therefore always in agreement with the graph at the time it was attached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleEntry<H> {
    /// The pipeline-side typed payload (kernel-opaque).
    pub payload: H,
    /// The canonical digest of the backing named graph this handle projects,
    /// pinned at attach time.
    pub content_digest: ContentDigest,
}

impl<H> HandleEntry<H> {
    /// Pair a payload with the digest of the graph it projects.
    pub fn new(payload: H, content_digest: ContentDigest) -> Self {
        Self {
            payload,
            content_digest,
        }
    }
}

/// An error from attaching a typed handle to a [`PipelineBundle`], from folding a
/// named graph into either carrier, or from canonicalizing a view carrier's
/// untrusted content.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PipelineBundleError {
    /// A handle's pinned digest does not equal the canonical digest of the named
    /// graph it claims to back. Always a hard failure — the bundle never carries a
    /// handle that disagrees with its graph.
    HandleDigestMismatch {
        /// The graph IRI the handle keys on.
        graph: HandleKey,
        /// The digest the handle pinned.
        pinned: ContentDigest,
        /// The canonical digest the backing graph actually hashes to.
        actual: ContentDigest,
    },
    /// A contribution offered to
    /// [`accumulate_named_graph`](PipelineBundle::accumulate_named_graph) carries a
    /// row — or a graph declaration — outside the graph it was to be folded into.
    /// Always a hard failure: see the module docs for why the carrier cannot fold
    /// such a contribution and stay honest about what it holds.
    GraphContainment {
        /// The graph the contribution was to be folded into.
        graph: HandleKey,
        /// Which table the offending row lives in.
        layer: GraphLayer,
        /// The graph the offending row actually names, or `None` for the default
        /// graph.
        found: Option<TermValue>,
    },
    /// Canonicalization refused the carrier's content. A view carrier composes
    /// caller-supplied sources, so its content is UNTRUSTED and every digest on it
    /// runs through the fallible canonicalization entry points; this variant is
    /// that refusal, carried verbatim.
    Canonicalization {
        /// What was being canonicalized when the refusal was raised.
        scope: CanonScopeName,
        /// The refusal itself.
        source: CanonError,
    },
    /// Composing a contribution onto a view carrier breached that carrier's
    /// [`ViewLimits`]. Admission stays per view and unchanged: the ceilings are
    /// re-checked CUMULATIVELY as each source is appended, so a carrier that has
    /// grown past its ceilings refuses the append rather than publishing a view it
    /// was never admitted to hold.
    AdmissionBreach(RdfDiagnostic),
    /// Freezing a view carrier's composed surface into one owned dataset failed.
    /// Raised only at an explicit ownership boundary
    /// ([`PipelineViewBundle::to_flat_bundle`]); reading a view carrier never
    /// materializes anything and so can never raise it.
    Materialization(RdfDiagnostic),
}

impl fmt::Display for PipelineBundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandleDigestMismatch {
                graph,
                pinned,
                actual,
            } => write!(
                f,
                "handle for graph <{graph}> pins digest {pinned} but the backing graph \
                 canonicalizes to {actual}"
            ),
            Self::GraphContainment {
                graph,
                layer,
                found,
            } => match found {
                Some(found) => write!(
                    f,
                    "a contribution folded into <{graph}> carries a {layer} in {found:?} instead"
                ),
                None => write!(
                    f,
                    "a contribution folded into <{graph}> carries a {layer} in the default graph \
                     instead"
                ),
            },
            Self::Canonicalization { scope, source } => {
                write!(f, "canonicalizing {scope} was refused: {source}")
            }
            Self::AdmissionBreach(diagnostic) => write!(
                f,
                "composing the contribution breached view admission: {}",
                diagnostic.message
            ),
            Self::Materialization(diagnostic) => write!(
                f,
                "materializing the composed surface failed: {}",
                diagnostic.message
            ),
        }
    }
}

impl std::error::Error for PipelineBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canonicalization { source, .. } => Some(source),
            Self::AdmissionBreach(diagnostic) | Self::Materialization(diagnostic) => {
                Some(diagnostic)
            }
            _ => None,
        }
    }
}

/// Non-semantic counters proving WHERE a digest came from.
///
/// Never part of any identity: nothing here is folded into
/// [`digest`](PipelineBundle::digest) or
/// [`pipeline_root`](PipelineBundle::pipeline_root), and two carriers with equal
/// content and different counters digest identically. They exist so a consumer —
/// or a test — can demonstrate that folding one graph into a carrier did NOT
/// re-canonicalize the graphs it left alone.
///
/// Counts are monotonic and, like [`CompositeDatasetView`]'s own work counter, are
/// SHARED with a carrier's clones: a clone reads the work its whole family did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BundleDigestWork {
    /// Per-graph canonicalizations this carrier family actually ran.
    pub graph_canonicalizations: usize,
    /// Per-graph digests answered from the carrier's OWN memo, with no
    /// canonicalization and without consulting a ledger.
    pub graph_cache_hits: usize,
    /// Whole-carrier canonicalizations (the first section of
    /// [`digest`](PipelineBundle::digest)).
    pub dataset_canonicalizations: usize,
    /// Canonicalizations of the pipeline root's residue leaf.
    pub residue_canonicalizations: usize,
}

/// The shared, atomic backing of [`BundleDigestWork`].
#[derive(Debug, Default)]
struct DigestWorkCounter {
    graph_canonicalizations: AtomicUsize,
    graph_cache_hits: AtomicUsize,
    dataset_canonicalizations: AtomicUsize,
    residue_canonicalizations: AtomicUsize,
}

impl DigestWorkCounter {
    fn bump(counter: &AtomicUsize) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn get(&self) -> BundleDigestWork {
        BundleDigestWork {
            graph_canonicalizations: self.graph_canonicalizations.load(Ordering::Relaxed),
            graph_cache_hits: self.graph_cache_hits.load(Ordering::Relaxed),
            dataset_canonicalizations: self.dataset_canonicalizations.load(Ordering::Relaxed),
            residue_canonicalizations: self.residue_canonicalizations.load(Ordering::Relaxed),
        }
    }
}

/// Everything a pipeline carrier holds BESIDES its RDF: the out-of-band lookaside,
/// the blob store, the provenance sidecar, the typed-handle lane, the per-graph
/// digest memo and the work counters — plus the digest folds and the pin check that
/// read them.
///
/// One implementation, shared by [`PipelineBundle`] and [`PipelineViewBundle`].
/// Neither carrier reimplements the fold, so "the flat and the view carrier agree
/// about identity" is a property of the code rather than a pair of listings a reader
/// has to diff.
#[derive(Debug, Clone)]
struct BundleCore<H> {
    /// Structured non-triple companion material (typed sidecar resources, blobs by
    /// reference, segments, metadata, …). Folded into every identity.
    lookaside: RdfLookaside,
    /// The single owner of blob payload bytes (by-reference doctrine). Folded into
    /// every identity.
    blobs: Arc<ContentStore>,
    /// The provenance sidecar (units / artifacts / origin-sets / occurrences).
    /// Folded into every identity through its PUBLIC projection.
    provenance: DatasetProvenance,
    /// The typed-handle lane: backing-graph IRI → typed payload + pinned digest.
    /// Mutated only through the carriers' pin/detach/accumulate methods, so the pin
    /// invariant holds. EXCLUDED from every identity.
    handles: BTreeMap<HandleKey, HandleEntry<H>>,
    /// Memoized per-graph canonical digests. NON-semantic: a carrier's RDF is
    /// immutable for as long as the carrier is not accumulated into, so each graph's
    /// canonical digest is stable and caching it lets the
    /// `pinned = graph_digest(g); pin_handle(g, …, pinned)` pattern canonicalize each
    /// backing graph ONCE instead of twice.
    ///
    /// `Arc<Mutex<…>>` so it rides `Clone` and is `Send`+`Sync` across the reasoning
    /// engine lock. Accumulation RESEATS this `Arc` — it installs a fresh map on the
    /// carrier being accumulated into — rather than clearing the map in place. That
    /// distinction is load-bearing: clearing a SHARED map lets a clone taken before
    /// the accumulation read digests that were computed against the OTHER clone's
    /// content, which is a wrong answer, not a stale one. After a reseat the two
    /// carriers own independent memos, each consistent with its own content.
    digest_cache: Arc<Mutex<BTreeMap<HandleKey, ContentDigest>>>,
    /// Where digests came from. Shared with clones; never an identity input.
    work: Arc<DigestWorkCounter>,
}

impl<H> BundleCore<H> {
    fn new(
        lookaside: RdfLookaside,
        blobs: Arc<ContentStore>,
        provenance: DatasetProvenance,
    ) -> Self {
        Self {
            lookaside,
            blobs,
            provenance,
            handles: BTreeMap::new(),
            digest_cache: Arc::new(Mutex::new(BTreeMap::new())),
            work: Arc::default(),
        }
    }

    /// The memoized digest for `graph`, counting a hit when there is one.
    fn cached(&self, graph: &str) -> Option<ContentDigest> {
        let hit = self
            .digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .get(graph)
            .copied();
        if hit.is_some() {
            DigestWorkCounter::bump(&self.work.graph_cache_hits);
        }
        hit
    }

    fn remember(&self, graph: &str, digest: ContentDigest) {
        self.digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .insert(graph.to_owned(), digest);
    }

    fn cache_snapshot(&self) -> BTreeMap<HandleKey, ContentDigest> {
        self.digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .clone()
    }

    /// Install a FRESH memo on this carrier, leaving every other carrier that shared
    /// the old one untouched. See the [`digest_cache`](Self::digest_cache) docs.
    fn reseat_cache(&mut self, seed: BTreeMap<HandleKey, ContentDigest>) {
        self.digest_cache = Arc::new(Mutex::new(seed));
    }

    /// Every already-pinned graph and the digest it pinned, for the additive
    /// invariant.
    fn pinned(&self) -> Vec<(HandleKey, ContentDigest)> {
        self.handles
            .iter()
            .map(|(k, e)| (k.clone(), e.content_digest))
            .collect()
    }

    /// The one pin check: `pinned` must equal `actual` or the attach hard-fails.
    fn verify_pin(
        graph: &str,
        pinned: ContentDigest,
        actual: ContentDigest,
    ) -> Result<(), PipelineBundleError> {
        if pinned == actual {
            return Ok(());
        }
        Err(PipelineBundleError::HandleDigestMismatch {
            graph: graph.to_owned(),
            pinned,
            actual,
        })
    }

    /// Sections 2–4 of the identity fold: the SORTED lookaside resource identities
    /// and digests, the SORTED blob digests, and the runtime-id-free public
    /// provenance projection. Shared verbatim by
    /// [`digest`](Self::digest_over) and [`pipeline_root`](Self::pipeline_root_over).
    fn fold_sidecars(&self, hasher: &mut Sha256) {
        // 2. Each lookaside resource's identity + content_digest, collected + SORTED
        //    so the fold is order-independent. The resource IDENTITY (name, falling
        //    back to iri, then path) is included so two bundles with identical content
        //    bytes but different resource names/paths produce distinct digests.
        //    A resource without a declared digest contributes an empty content marker.
        let mut resource_entries: Vec<(&str, &str)> = self
            .lookaside
            .resources
            .iter()
            .map(|r| {
                let identity = r
                    .name
                    .as_deref()
                    .or(r.iri.as_deref())
                    .or(r.path.as_deref())
                    .unwrap_or("");
                let digest = r.content_digest.as_deref().unwrap_or("");
                (identity, digest)
            })
            .collect();
        resource_entries.sort_unstable();
        for (identity, digest) in resource_entries {
            hasher.update(identity.as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(digest.as_bytes());
            hasher.update([SEP_RECORD]);
        }
        hasher.update([SEP_SECTION]);

        // 3. Each blob's ContentDigest in the store, SORTED (the store is a hash map,
        //    so iteration order is otherwise nondeterministic).
        let mut blob_digests: Vec<ContentDigest> =
            self.blobs.iter().map(|(digest, _)| *digest).collect();
        blob_digests.sort_unstable();
        for d in blob_digests {
            hasher.update(d.as_bytes());
            hasher.update([SEP_RECORD]);
        }
        hasher.update([SEP_SECTION]);

        // 4. The PUBLIC provenance projection (quad index, unit names, kinds, artifact
        //    paths, locations) — NEVER the runtime numeric ids (S0.5). The projection
        //    is sorted by `public_projection`, so it is allocation-order-independent.
        //    The quad index is included so occurrences over distinct quads but sharing
        //    the same (unit, artifact, location) are preserved as distinct rows.
        for (quad_idx, unit, kind, artifact, location) in self.provenance.public_projection() {
            hasher.update(quad_idx.to_string().as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(unit.as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(kind.as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(artifact.as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(location.as_deref().unwrap_or("").as_bytes());
            hasher.update([SEP_RECORD]);
        }
    }

    /// The bundle digest over an already-computed canonical N-Quads document.
    fn digest_over(&self, canonical_nquads: &str) -> ContentDigest {
        let mut hasher = Sha256::new();
        hasher.update(canonical_nquads.as_bytes());
        hasher.update([SEP_SECTION]);
        self.fold_sidecars(&mut hasher);
        finish(hasher)
    }

    /// The pipeline root over already-computed leaves. See [`PIPELINE_ROOT_DOMAIN`]
    /// for what each section is and why.
    fn pipeline_root_over(
        &self,
        leaves: &BTreeMap<HandleKey, ContentDigest>,
        residue: ContentDigest,
    ) -> ContentDigest {
        let mut hasher = Sha256::new();
        hasher.update(PIPELINE_ROOT_DOMAIN.as_bytes());
        hasher.update([SEP_SECTION]);
        for (graph, digest) in leaves {
            hasher.update(graph.as_bytes());
            hasher.update([SEP_FIELD]);
            hasher.update(digest.as_bytes());
            hasher.update([SEP_RECORD]);
        }
        hasher.update([SEP_SECTION]);
        hasher.update(residue.as_bytes());
        hasher.update([SEP_SECTION]);
        self.fold_sidecars(&mut hasher);
        finish(hasher)
    }
}

/// Close a fold into a [`ContentDigest`].
fn finish(hasher: Sha256) -> ContentDigest {
    let out = hasher.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    ContentDigest::from_raw(buf)
}

/// The IRI-named graphs `view` addresses, sorted and deduplicated.
///
/// Blank-node graph names are deliberately absent: a per-graph digest is addressed
/// by IRI on BOTH carriers (see
/// [`canonicalize_graph_view`](super::canon::canonicalize_graph_view) and
/// [`RdfDataset::project_named_graph`]), and a blank name is not stable across
/// carriers anyway — composition standardizes blank scopes apart. Rows in a
/// blank-named graph are folded into the pipeline root's residue leaf instead.
fn iri_named_graphs<D: DatasetView>(view: &D) -> Vec<HandleKey> {
    let mut out: Vec<HandleKey> = view
        .named_graphs()
        .filter_map(|id| match view.resolve(id) {
            TermRef::Iri(iri) => Some(iri.to_owned()),
            _ => None,
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Whether every row and declaration of `contribution` names `graph` and nothing
/// else — the containment invariant the module docs state.
fn check_containment(contribution: &RdfDataset, graph: &str) -> Result<(), PipelineBundleError> {
    let names = |slot: Option<crate::TermId>| matches!(slot.map(|id| contribution.resolve(id)), Some(TermRef::Iri(iri)) if iri == graph);
    let violation =
        |layer: GraphLayer, slot: Option<crate::TermId>| PipelineBundleError::GraphContainment {
            graph: graph.to_owned(),
            layer,
            found: slot.map(|id| owned_value(contribution, id)),
        };
    for q in contribution.quads() {
        if !names(q.g) {
            return Err(violation(GraphLayer::Quad, q.g));
        }
    }
    for q in contribution.reifier_quads() {
        if !names(q.g) {
            return Err(violation(GraphLayer::Reifier, q.g));
        }
    }
    for q in contribution.annotation_quads() {
        if !names(q.g) {
            return Err(violation(GraphLayer::Annotation, q.g));
        }
    }
    for g in contribution.named_graphs() {
        if !names(Some(g)) {
            return Err(violation(GraphLayer::Declaration, Some(g)));
        }
    }
    Ok(())
}

/// The rows no IRI-named leaf of [`PIPELINE_ROOT_DOMAIN`] can address: every quad,
/// reifier declaration and annotation whose graph slot is the default graph or a
/// blank-node graph name, each keeping that slot.
///
/// Keeping the slot is what makes the residue a faithful leaf: a blank graph name
/// is itself canonicalized, so two carriers holding the same rows under
/// differently-numbered blank scopes produce the same residue bytes, while rows
/// moved BETWEEN the default graph and a blank-named one do not.
struct ResidueView<'a, D>(&'a D);

impl<D: DatasetView> ResidueView<'_, D> {
    fn keeps(&self, g: Option<D::Id>) -> bool {
        g.is_none_or(|id| !matches!(self.0.resolve(id), TermRef::Iri(_)))
    }
}

impl<D: DatasetView> DatasetView for ResidueView<'_, D> {
    type Id = D::Id;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.0.quads().filter(|q| self.keeps(q.g))
    }

    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_, Self::Id>> + '_ {
        self.quads().map(|q| QuadRef {
            s: self.resolve(q.s),
            p: self.resolve(q.p),
            o: self.resolve(q.o),
            g: q.g.map(|id| self.resolve(id)),
        })
    }

    fn resolve(&self, id: Self::Id) -> TermRef<'_, Self::Id> {
        self.0.resolve(id)
    }

    fn term_id_by_value(&self, value: &TermValue) -> Option<Self::Id> {
        self.0.term_id_by_value(value)
    }

    fn capabilities(&self) -> RdfStoreCapabilities {
        self.0.capabilities()
    }

    fn probe_plan(
        &self,
        _s: bool,
        _p: bool,
        _o: bool,
        _g: GraphMatch<Self::Id>,
    ) -> Self::ProbePlan {
    }

    fn quads_for_pattern_with_plan(
        &self,
        _plan: &Self::ProbePlan,
        s: Option<Self::Id>,
        p: Option<Self::Id>,
        o: Option<Self::Id>,
        g: GraphMatch<Self::Id>,
    ) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.quads_for_pattern(s, p, o, g)
    }

    fn term_count(&self) -> usize {
        self.0.term_count()
    }

    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.0.reifier_quads().filter(|q| self.keeps(q.g))
    }

    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds<Self::Id>> + '_ {
        self.0.annotation_quads().filter(|q| self.keeps(q.g))
    }
}

/// The pipeline carrier: the frozen hot graph plus its out-of-band material and a
/// typed-handle lane.
///
/// Construct through [`PipelineBundle::new`] and the builder methods; access fields
/// through the narrow accessor methods. All handle-critical fields are PRIVATE so
/// that downstream code cannot replace `dataset` after `pin_handle` or insert into
/// `handles` directly, which would silently break the pin invariant. Generic over
/// the typed-handle payload `H` — see the module docs for the kernel-boundary
/// rationale.
///
/// This carrier OWNS one frozen dataset and is deliberately ledger-free: it retains
/// nothing another carrier could be sharing, so there is nothing for a
/// [`RetentionLedger`] to deduplicate. [`to_view_bundle`](Self::to_view_bundle)
/// converts it to the view carrier, which does report into one.
#[derive(Debug, Clone)]
pub struct PipelineBundle<H> {
    /// The immutable, value-interned RDF 1.2 dataset — the hot graph.
    /// PRIVATE: replacing this after `pin_handle` would corrupt the pin invariant.
    dataset: Arc<RdfDataset>,
    /// Lookaside, blobs, provenance, handles, the digest memo and the folds.
    core: BundleCore<H>,
}

impl<H> PipelineBundle<H> {
    /// Assemble a pipeline bundle from its parts, with an empty handle lane.
    ///
    /// Mirrors [`GtsBundle::new`](super::bundle::GtsBundle::new): the dataset is the
    /// frozen hot graph and the remaining parts are the out-of-band material that
    /// travels with it. Attach typed handles afterwards via
    /// [`pin_handle`](Self::pin_handle).
    pub fn new(
        dataset: Arc<RdfDataset>,
        lookaside: RdfLookaside,
        blobs: Arc<ContentStore>,
        provenance: DatasetProvenance,
    ) -> Self {
        Self {
            dataset,
            core: BundleCore::new(lookaside, blobs, provenance),
        }
    }

    /// Borrow the frozen hot graph.
    pub fn dataset(&self) -> &RdfDataset {
        &self.dataset
    }

    /// Clone the `Arc` to the frozen hot graph (cheap reference-count bump) — for a
    /// consumer that needs to share the dataset by handle rather than borrow it.
    pub fn dataset_arc(&self) -> Arc<RdfDataset> {
        Arc::clone(&self.dataset)
    }

    /// Borrow the out-of-band lookaside.
    pub fn lookaside(&self) -> &RdfLookaside {
        &self.core.lookaside
    }

    /// Borrow the blob store.
    pub fn blobs(&self) -> &ContentStore {
        &self.core.blobs
    }

    /// Borrow the provenance sidecar.
    pub fn provenance(&self) -> &DatasetProvenance {
        &self.core.provenance
    }

    /// The typed handle for a backing graph IRI, if one is attached.
    pub fn handle(&self, graph: &str) -> Option<&HandleEntry<H>> {
        self.core.handles.get(graph)
    }

    /// Borrow the entire typed-handle map (read-only). The map is only mutated
    /// through [`pin_handle`](Self::pin_handle) / [`detach_handle`](Self::detach_handle)
    /// to preserve the pin invariant.
    pub fn handles(&self) -> &BTreeMap<HandleKey, HandleEntry<H>> {
        &self.core.handles
    }

    /// Replace the provenance sidecar in place.
    ///
    /// This is the controlled mutator for the provenance field; it does NOT affect
    /// the [`digest`](Self::digest) correctness since the digest reads the provenance
    /// through the public projection. Used by the pipeline scheduler to thread the
    /// per-stage provenance into the produced carrier.
    pub fn set_provenance(&mut self, provenance: DatasetProvenance) {
        self.core.provenance = provenance;
    }

    /// Where this carrier family's digests came from. Never an identity input — see
    /// [`BundleDigestWork`].
    #[must_use]
    pub fn digest_work(&self) -> BundleDigestWork {
        self.core.work.get()
    }

    /// Every IRI-named graph this carrier addresses — quad-bearing or declared
    /// empty — sorted and deduplicated. These are the leaves
    /// [`pipeline_root`](Self::pipeline_root) folds.
    #[must_use]
    pub fn named_graph_iris(&self) -> Vec<HandleKey> {
        iri_named_graphs(&*self.dataset)
    }

    /// Attach a typed handle for the named graph `graph`, pinning `payload` to the
    /// canonical digest of that graph's subgraph.
    ///
    /// The supplied `content_digest` MUST equal the canonical digest of the backing
    /// named graph (see [`Self::graph_digest`]); on mismatch this HARD-fails with
    /// [`PipelineBundleError::HandleDigestMismatch`] and the bundle is left
    /// unchanged. A previously attached handle for the same graph is replaced.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::HandleDigestMismatch`] if the pinned digest disagrees
    /// with the backing graph.
    pub fn pin_handle(
        &mut self,
        graph: impl Into<HandleKey>,
        payload: H,
        content_digest: ContentDigest,
    ) -> Result<(), PipelineBundleError> {
        let graph = graph.into();
        let actual = self.graph_digest(&graph);
        BundleCore::<H>::verify_pin(&graph, content_digest, actual)?;
        self.core
            .handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// Detach the typed handle for `graph`, returning it if present. Detaching does
    /// NOT change [`digest`](Self::digest) (the handle lane is excluded).
    pub fn detach_handle(&mut self, graph: &str) -> Option<HandleEntry<H>> {
        self.core.handles.remove(graph)
    }

    /// Fold an additional named graph into the carrier and pin a typed handle to it,
    /// preserving every already-pinned graph.
    ///
    /// The quads of `graph_quads` (which carry `g == graph`) union into the dataset;
    /// the handle for `graph` is then pinned to that graph's canonical digest. This is
    /// the carrier-accumulation primitive: a producing stage folds its named graph into
    /// the carrier AS IT FLOWS, so the terminal step never re-folds it from a byte
    /// artifact (the dataset is the single internal transport; the projection is
    /// transformed once, upstream).
    ///
    /// `graph_quads` must be CONTAINED in `graph`: every quad, reifier declaration,
    /// annotation row and named-graph declaration it carries names `graph`. That was
    /// once a parenthetical about what callers pass; it is now checked, before
    /// anything is folded, and violating it HARD-fails with
    /// [`PipelineBundleError::GraphContainment`]. See the module docs for why.
    ///
    /// Accumulation is ADDITIVE: it MUST NOT change any already-pinned graph's digest.
    /// A violation HARD-fails with [`PipelineBundleError::HandleDigestMismatch`] for the
    /// disturbed graph (no-optionality) — the carrier never silently rewrites a graph a
    /// downstream handle already pinned. This is the single-assembly integrity invariant
    /// (there is only ever one copy of each graph, so no cross-copy check is needed).
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::GraphContainment`] if `graph_quads` reaches outside
    /// `graph`; [`PipelineBundleError::HandleDigestMismatch`] if folding `graph_quads`
    /// would shift an already-pinned graph's digest.
    pub fn accumulate_named_graph(
        &mut self,
        graph: impl Into<HandleKey>,
        graph_quads: &RdfDataset,
        payload: H,
    ) -> Result<(), PipelineBundleError> {
        let graph = graph.into();
        // Containment first: nothing is folded until the contribution is known to
        // stay inside the graph it claims.
        check_containment(graph_quads, &graph)?;
        // Record every already-pinned graph's digest to enforce the additive invariant.
        let prior = self.core.pinned();
        // Fold the new named graph into the single carrier dataset.
        self.dataset = Arc::new(RdfDataset::union(&[&self.dataset, graph_quads]));
        // The dataset changed: RESEAT the per-graph digest memo — install a fresh,
        // EMPTY map on this carrier — so the additive invariant below (and every later
        // `graph_digest`) recomputes against the NEW dataset. Seeding it empty keeps the
        // long-standing fail-closed reading of this carrier: a stale entry would hide a
        // disturbed pinned graph. Reseating rather than clearing is what keeps a clone
        // taken BEFORE this call reading digests of its OWN content — the clone keeps
        // the map it already had, and this carrier never writes into it again.
        self.core.reseat_cache(BTreeMap::new());
        // Additive invariant: no previously pinned graph may have shifted.
        for (k, pinned) in prior {
            let actual = self.graph_digest(&k);
            BundleCore::<H>::verify_pin(&k, pinned, actual)?;
        }
        // Pin the new handle to its now-present backing graph.
        let content_digest = self.graph_digest(&graph);
        self.core
            .handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// The canonical [`ContentDigest`] of the named graph `graph` — the subgraph of
    /// the dataset asserted in `<graph>`, canonicalized to N-Quads.
    ///
    /// Built by [`RdfDataset::project_named_graph`] and hashed over the projection's
    /// canonical form. The RDF 1.2 reifier/annotation side-tables are keyed per graph
    /// in this IR (each row carries the graph its declaration or annotation was
    /// asserted in), so the projection admits exactly the rows whose own graph slot is
    /// `<graph>` — a handle over a reified subgraph pins over that graph's statement
    /// layer and nothing else. Consequently this digest is a function of `<graph>`'s
    /// content alone: adding, removing or annotating statements in ANY other graph
    /// leaves it untouched, which is what makes the additive invariant in
    /// [`accumulate_named_graph`](Self::accumulate_named_graph) meaningful rather than
    /// accidental. This is the value a handle's pinned digest is checked against in
    /// [`pin_handle`](Self::pin_handle).
    ///
    /// This carrier owns its dataset, so its content is TRUSTED and the panicking
    /// canonicalization entry point is the right one. The view carrier composes
    /// caller-supplied sources and therefore returns
    /// [`PipelineBundleError::Canonicalization`] instead of panicking — same bytes on
    /// success, different obligation on refusal.
    #[must_use]
    pub fn graph_digest(&self, graph: &str) -> ContentDigest {
        // Memoized: the frozen dataset makes each graph's canonical digest stable, so
        // the second call in the `pinned = graph_digest(g); pin_handle(g, …, pinned)`
        // pattern reuses the first canonicalization instead of recomputing it.
        if let Some(cached) = self.core.cached(graph) {
            return cached;
        }
        let subgraph = self.dataset.project_named_graph(graph);
        let digest = ContentDigest::of(canonicalize(&subgraph).nquads.as_bytes());
        DigestWorkCounter::bump(&self.core.work.graph_canonicalizations);
        self.core.remember(graph, digest);
        digest
    }

    /// The content [`ContentDigest`] of this bundle: a SHA-256 fold over the
    /// dataset's canonical hash, the SORTED lookaside resource digests, the SORTED
    /// blob digests, and the runtime-id-free public provenance projection. The
    /// typed-handle lane contributes NOTHING (see the module docs).
    #[must_use]
    pub fn digest(&self) -> ContentDigest {
        let canonical = canonicalize(&self.dataset).nquads;
        DigestWorkCounter::bump(&self.core.work.dataset_canonicalizations);
        self.core.digest_over(&canonical)
    }

    /// The residue leaf: the canonical digest of every row this carrier holds outside
    /// its IRI-named graphs. See [`PIPELINE_ROOT_DOMAIN`].
    fn residue_digest(&self) -> ContentDigest {
        let residue = ResidueView(&*self.dataset);
        let canonical = canonicalize_view(&residue, CanonHash::Sha256).nquads;
        DigestWorkCounter::bump(&self.core.work.residue_canonicalizations);
        ContentDigest::of(canonical.as_bytes())
    }

    /// The carrier's PIPELINE ROOT: a second, versioned, domain-separated identity
    /// over this carrier's per-graph leaves, for stage caching.
    ///
    /// See [`PIPELINE_ROOT_DOMAIN`] for the exact bytes and for why this is an
    /// ADDITIONAL identity rather than a replacement for [`digest`](Self::digest).
    /// A [`PipelineViewBundle`] over equal content produces an equal root.
    #[must_use]
    pub fn pipeline_root(&self) -> ContentDigest {
        let leaves: BTreeMap<HandleKey, ContentDigest> = self
            .named_graph_iris()
            .into_iter()
            .map(|graph| {
                let digest = self.graph_digest(&graph);
                (graph, digest)
            })
            .collect();
        self.core.pipeline_root_over(&leaves, self.residue_digest())
    }

    /// Carry this bundle's whole state onto a VIEW carrier that retains the same
    /// dataset as a composed source, reporting it to `ledger`.
    ///
    /// The lookaside, blobs, provenance, typed handles and per-graph memo travel
    /// verbatim; the dataset becomes the single source of a one-source
    /// [`CompositeDatasetView`] with its graphs PRESERVED, so the view carrier holds
    /// exactly this carrier's content and agrees with it about
    /// [`digest`](Self::digest), every per-graph digest and
    /// [`pipeline_root`](Self::pipeline_root).
    ///
    /// `limits` is explicit rather than defaulted: admission is a statement about
    /// what THIS carrier may retain, and a fabricated ceiling is a statement nobody
    /// made.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::AdmissionBreach`] if composing the one-source view
    /// would exceed `limits`.
    pub fn to_view_bundle(
        &self,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Result<PipelineViewBundle<H>, PipelineBundleError>
    where
        H: Clone,
    {
        self.clone().into_view_bundle(ledger, limits)
    }

    /// [`to_view_bundle`](Self::to_view_bundle) by value, for a payload `H` that is
    /// not `Clone`.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::AdmissionBreach`] if composing the one-source view
    /// would exceed `limits`.
    pub fn into_view_bundle(
        self,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Result<PipelineViewBundle<H>, PipelineBundleError> {
        PipelineViewBundle::over_dataset(&self.dataset, self.core, ledger, limits)
    }
}

impl RdfDataset {
    /// A fresh owned `RdfDataset` snapshotting this one's frozen tables. The
    /// fallback for the rare case a freshly-frozen `Arc` is shared; the lazy caches
    /// rebuild on demand. Crate-internal — the public deep-copy path is
    /// [`union`](RdfDataset::union) of a single input. A single-input union
    /// trivially "agrees" with itself, so `self`'s
    /// [`ContentIdScheme`](crate::ContentIdScheme) (if any) and derivation
    /// predicate carry forward too — the CONTENT-ADDRESSING CONFIGURATION is a
    /// lossless snapshot, not merely a statement-layer one.
    ///
    /// The snapshot is graph-ISOMORPHIC to `self`, not label-identical:
    /// [`union`](RdfDataset::union) re-scopes every input's blank nodes under a
    /// fresh [`BlankScope`](crate::BlankScope) (standardize-apart, C0.2) even in
    /// this single-input case, so a blank node's `(label, scope)` pair in the
    /// snapshot generally differs from the one in `self`, while the graph
    /// structure — including every blank node's co-reference pattern — is
    /// preserved exactly.
    pub(crate) fn owned_snapshot(&self) -> Self {
        Self::union(&[self])
    }
}

/// A named graph whose rows all come from ONE frozen base, and the ledger guard
/// that base is registered under.
///
/// The pair is what licenses a memoized per-graph digest: the memo is filed against
/// the BASE's ledger key, so its value must be the base's own digest for that graph
/// and nothing else. Recording the base alongside the guard means the memo is always
/// computed from the thing the key names, rather than from whatever composite
/// happens to hold it.
#[derive(Debug, Clone)]
struct SoleGraph {
    base: Arc<RdfDataset>,
    guard: RetentionGuard,
}

/// The VIEW pipeline carrier: an [`Arc<CompositeDatasetView>`] plus the same
/// out-of-band material and typed-handle lane [`PipelineBundle`] carries.
///
/// Interchangeable with [`PipelineBundle`] by content — equal content yields equal
/// [`digest`](Self::digest), equal per-graph digests and equal
/// [`pipeline_root`](Self::pipeline_root) — and different from it in what folding a
/// graph COSTS. Accumulation here appends a retained source through
/// [`CompositeDatasetView::extend`]: no rows are copied, no dataset is frozen, and
/// the sources already composed keep the aliasing they had. Freezing happens exactly
/// once, at an explicit ownership boundary the caller asks for
/// ([`materialize`](Self::materialize) / [`to_flat_bundle`](Self::to_flat_bundle)).
///
/// ## Retention
///
/// Every base this carrier retains is registered with an explicitly supplied
/// [`RetentionLedger`], which reports it ONCE however many carriers hold it and
/// releases it when the last reader drops. The ledger is required rather than
/// created-on-demand: a ledger nobody else shares reports nothing a per-view
/// [`ViewStats`] did not already say, so a carrier that invented its own would be
/// answering a question nobody asked. Per-view admission is untouched — [`ViewLimits`]
/// remains the sole gate and remains per view.
///
/// ## Untrusted content
///
/// A composite view holds caller-supplied sources, so every digest on this carrier
/// runs through the FALLIBLE canonicalization entry points and returns
/// [`PipelineBundleError::Canonicalization`] rather than panicking. The flat carrier
/// owns its dataset and keeps the trusted, panicking spelling; the bytes agree on
/// success.
#[derive(Debug, Clone)]
pub struct PipelineViewBundle<H> {
    /// The composed, immutable RDF surface. PRIVATE: replacing it after
    /// `pin_handle` would corrupt the pin invariant.
    view: Arc<CompositeDatasetView>,
    /// The shared retention scope every retained base reports into.
    ledger: Arc<RetentionLedger>,
    /// This carrier's own admission ceilings, re-checked cumulatively on every
    /// accumulation.
    limits: ViewLimits,
    /// One guard per registration of a retained owner. Held for its lifetime effect:
    /// dropping the carrier releases exactly this carrier's readers.
    retained: Vec<RetentionGuard>,
    /// The graphs whose rows come from exactly one frozen base, and therefore may be
    /// answered from that base's shared ledger memo. A graph absent here is
    /// canonicalized on the composite itself.
    sole: BTreeMap<HandleKey, SoleGraph>,
    /// Lookaside, blobs, provenance, handles, the digest memo and the folds.
    core: BundleCore<H>,
}

impl<H> PipelineViewBundle<H> {
    /// Assemble a view carrier over ONE frozen dataset, composed as a single source
    /// with its graphs preserved.
    ///
    /// The mirror of [`PipelineBundle::new`], and the carrier
    /// [`PipelineBundle::to_view_bundle`] produces. The dataset is taken by
    /// REFERENCE: this carrier shares the base rather than consuming it, which is the
    /// whole point of registering it with a ledger, and a caller that still holds the
    /// same `Arc` is the ordinary case rather than the exception.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::AdmissionBreach`] if composing the one-source view
    /// would exceed `limits`.
    pub fn from_dataset(
        dataset: &Arc<RdfDataset>,
        lookaside: RdfLookaside,
        blobs: Arc<ContentStore>,
        provenance: DatasetProvenance,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Result<Self, PipelineBundleError> {
        Self::over_dataset(
            dataset,
            BundleCore::new(lookaside, blobs, provenance),
            ledger,
            limits,
        )
    }

    /// Assemble a view carrier over an ALREADY-composed view.
    ///
    /// Accepts an owned [`CompositeDatasetView`] or a shared `Arc` of one. Every
    /// native source is registered with `ledger`; a delta source registers both the
    /// base it branched from and its delta, which is exactly what that source charges
    /// into its own [`ViewStats`].
    ///
    /// No graph of a caller-composed view is treated as SOLE-OWNED by one base:
    /// placement and source order are the composing caller's
    /// business and are not readable back off the view, so this carrier cannot prove
    /// that one base answers for one graph and declines to assume it. Per-graph
    /// digests are therefore canonicalized on the composite, and this carrier's own
    /// memo — not a shared one — serves the repeats. [`from_dataset`](Self::from_dataset)
    /// composes the view itself and does know, so it does memoize.
    ///
    /// `limits` is RECORDED, not re-checked. The view arrives already composed and
    /// already admitted by whoever composed it; re-running admission here would
    /// re-refuse a view that legitimately exists, which is why this is the one
    /// infallible constructor. The ceilings it records are enforced from the next
    /// [`accumulate_named_graph`](Self::accumulate_named_graph) onwards, cumulatively
    /// over everything the carrier then holds.
    #[must_use]
    pub fn from_view(
        view: impl Into<Arc<CompositeDatasetView>>,
        lookaside: RdfLookaside,
        blobs: Arc<ContentStore>,
        provenance: DatasetProvenance,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Self {
        let view = view.into();
        let mut retained = Vec::new();
        for source in view.sources() {
            if let Some(base) = source.dataset() {
                retained.push(ledger.retain_dataset(base));
            }
            if let Some(delta) = source.delta() {
                retained.push(ledger.retain_dataset(delta.base()));
                retained.push(ledger.retain_dataset(delta.delta()));
            }
        }
        Self {
            view,
            ledger: Arc::clone(ledger),
            limits,
            retained,
            sole: BTreeMap::new(),
            core: BundleCore::new(lookaside, blobs, provenance),
        }
    }

    /// Assemble a view carrier over a delta snapshot, WITHOUT compacting its base.
    ///
    /// The delta rides the carrier as a composed source: its base stays shared and
    /// un-copied, and both the base and the delta overlay are registered with
    /// `ledger`, so a second carrier over the same base is charged for it once
    /// between them.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::AdmissionBreach`] if composing the one-source view
    /// would exceed `limits`.
    pub fn from_delta(
        delta: &Arc<DeltaDatasetView>,
        lookaside: RdfLookaside,
        blobs: Arc<ContentStore>,
        provenance: DatasetProvenance,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Result<Self, PipelineBundleError> {
        let view = CompositeDatasetView::from_bound_sources(
            vec![CompositeSource::from_delta(Arc::clone(delta))],
            limits,
        )
        .map_err(PipelineBundleError::AdmissionBreach)?;
        let retained = vec![
            ledger.retain_dataset(delta.base()),
            ledger.retain_dataset(delta.delta()),
        ];
        Ok(Self {
            view: Arc::new(view),
            ledger: Arc::clone(ledger),
            limits,
            retained,
            sole: BTreeMap::new(),
            core: BundleCore::new(lookaside, blobs, provenance),
        })
    }

    /// The shared constructor behind [`from_dataset`](Self::from_dataset) and
    /// [`PipelineBundle::into_view_bundle`]: compose `dataset` as one
    /// graph-preserving source and record it as the sole owner of every graph it
    /// addresses.
    fn over_dataset(
        dataset: &Arc<RdfDataset>,
        core: BundleCore<H>,
        ledger: &Arc<RetentionLedger>,
        limits: ViewLimits,
    ) -> Result<Self, PipelineBundleError> {
        let view = CompositeDatasetView::from_bound_sources(
            vec![CompositeSource::new(Arc::clone(dataset))],
            limits,
        )
        .map_err(PipelineBundleError::AdmissionBreach)?;
        let guard = ledger.retain_dataset(dataset);
        // One source, placement preserved: every row of every graph this view
        // addresses comes from `dataset` and from nothing else.
        let sole = iri_named_graphs(&**dataset)
            .into_iter()
            .map(|graph| {
                (
                    graph,
                    SoleGraph {
                        base: Arc::clone(dataset),
                        guard: guard.clone(),
                    },
                )
            })
            .collect();
        Ok(Self {
            view: Arc::new(view),
            ledger: Arc::clone(ledger),
            limits,
            retained: vec![guard],
            sole,
            core,
        })
    }

    /// Borrow the composed RDF surface.
    pub fn view(&self) -> &CompositeDatasetView {
        &self.view
    }

    /// Clone the `Arc` to the composed surface (cheap reference-count bump).
    pub fn view_arc(&self) -> Arc<CompositeDatasetView> {
        Arc::clone(&self.view)
    }

    /// The shared retention scope this carrier reports into.
    pub fn ledger(&self) -> &Arc<RetentionLedger> {
        &self.ledger
    }

    /// This carrier's admission ceilings.
    #[must_use]
    pub const fn limits(&self) -> ViewLimits {
        self.limits
    }

    /// The composed view's own per-view accounting, unchanged by the ledger.
    #[must_use]
    pub fn view_stats(&self) -> ViewStats {
        self.view.stats()
    }

    /// The two accounting categories side by side: deduplicated ledger-wide
    /// retention, and this view's own incremental bookkeeping and work.
    #[must_use]
    pub fn accounting(&self) -> ViewAccountingReport {
        self.ledger.report(&self.view.stats())
    }

    /// Borrow the out-of-band lookaside.
    pub fn lookaside(&self) -> &RdfLookaside {
        &self.core.lookaside
    }

    /// Borrow the blob store.
    pub fn blobs(&self) -> &ContentStore {
        &self.core.blobs
    }

    /// Borrow the provenance sidecar.
    pub fn provenance(&self) -> &DatasetProvenance {
        &self.core.provenance
    }

    /// The typed handle for a backing graph IRI, if one is attached.
    pub fn handle(&self, graph: &str) -> Option<&HandleEntry<H>> {
        self.core.handles.get(graph)
    }

    /// Borrow the entire typed-handle map (read-only).
    pub fn handles(&self) -> &BTreeMap<HandleKey, HandleEntry<H>> {
        &self.core.handles
    }

    /// Replace the provenance sidecar in place. See
    /// [`PipelineBundle::set_provenance`].
    pub fn set_provenance(&mut self, provenance: DatasetProvenance) {
        self.core.provenance = provenance;
    }

    /// Where this carrier family's digests came from. Never an identity input — see
    /// [`BundleDigestWork`].
    #[must_use]
    pub fn digest_work(&self) -> BundleDigestWork {
        self.core.work.get()
    }

    /// Every IRI-named graph this carrier addresses — quad-bearing or declared
    /// empty — sorted and deduplicated.
    #[must_use]
    pub fn named_graph_iris(&self) -> Vec<HandleKey> {
        iri_named_graphs(&*self.view)
    }

    /// The canonical [`ContentDigest`] of the named graph `graph`, over exactly the
    /// rows whose own graph slot is `<graph>` — the same value, byte for byte, that
    /// [`PipelineBundle::graph_digest`] computes for equal content.
    ///
    /// Answered from, in order: this carrier's own memo; the SHARED ledger memo, when
    /// one frozen base is the SOLE source answering for the whole graph; otherwise
    /// [`try_graph_digest_view`] on the composite itself, which reads the composed
    /// rows in place and materializes nothing.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::Canonicalization`] if canonicalizing `<graph>`'s
    /// subgraph is refused.
    pub fn graph_digest(&self, graph: &str) -> Result<ContentDigest, PipelineBundleError> {
        if let Some(cached) = self.core.cached(graph) {
            return Ok(cached);
        }
        let digest = match self.sole.get(graph) {
            // The memo is filed against the BASE's ledger key, so it is computed from
            // that base — the thing the key names — not from this composite.
            Some(sole) => sole.guard.try_memoized_graph_digest(graph, || {
                let digest = try_graph_digest_view(&*sole.base, graph).map_err(|source| {
                    canon_refusal(CanonScopeName::Graph(graph.to_owned()), source)
                })?;
                DigestWorkCounter::bump(&self.core.work.graph_canonicalizations);
                Ok(digest)
            })?,
            None => {
                let digest = try_graph_digest_view(&*self.view, graph).map_err(|source| {
                    canon_refusal(CanonScopeName::Graph(graph.to_owned()), source)
                })?;
                DigestWorkCounter::bump(&self.core.work.graph_canonicalizations);
                digest
            }
        };
        self.core.remember(graph, digest);
        Ok(digest)
    }

    /// The content [`ContentDigest`] of this carrier — the same four-section fold
    /// [`PipelineBundle::digest`] performs, reading its first section through
    /// [`try_canonicalize_view`].
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::Canonicalization`] if canonicalizing the composed
    /// surface is refused.
    pub fn digest(&self) -> Result<ContentDigest, PipelineBundleError> {
        let canonical = try_canonicalize_view(&*self.view, CanonHash::Sha256)
            .map_err(|source| canon_refusal(CanonScopeName::Carrier, source))?
            .nquads;
        DigestWorkCounter::bump(&self.core.work.dataset_canonicalizations);
        Ok(self.core.digest_over(&canonical))
    }

    /// The residue leaf. See [`PIPELINE_ROOT_DOMAIN`].
    fn residue_digest(&self) -> Result<ContentDigest, PipelineBundleError> {
        let residue = ResidueView(&*self.view);
        let canonical = try_canonicalize_view(&residue, CanonHash::Sha256)
            .map_err(|source| canon_refusal(CanonScopeName::Residue, source))?
            .nquads;
        DigestWorkCounter::bump(&self.core.work.residue_canonicalizations);
        Ok(ContentDigest::of(canonical.as_bytes()))
    }

    /// The carrier's PIPELINE ROOT — see [`PIPELINE_ROOT_DOMAIN`] and
    /// [`PipelineBundle::pipeline_root`], whose value this equals for equal content.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::Canonicalization`] if any leaf, or the residue, is
    /// refused.
    pub fn pipeline_root(&self) -> Result<ContentDigest, PipelineBundleError> {
        let mut leaves = BTreeMap::new();
        for graph in self.named_graph_iris() {
            let digest = self.graph_digest(&graph)?;
            leaves.insert(graph, digest);
        }
        let residue = self.residue_digest()?;
        Ok(self.core.pipeline_root_over(&leaves, residue))
    }

    /// Attach a typed handle for the named graph `graph`. See
    /// [`PipelineBundle::pin_handle`] — identical contract, fallible
    /// canonicalization.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::HandleDigestMismatch`] if the pinned digest disagrees
    /// with the backing graph; [`PipelineBundleError::Canonicalization`] if that
    /// graph cannot be canonicalized.
    pub fn pin_handle(
        &mut self,
        graph: impl Into<HandleKey>,
        payload: H,
        content_digest: ContentDigest,
    ) -> Result<(), PipelineBundleError> {
        let graph = graph.into();
        let actual = self.graph_digest(&graph)?;
        BundleCore::<H>::verify_pin(&graph, content_digest, actual)?;
        self.core
            .handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// Detach the typed handle for `graph`, returning it if present.
    pub fn detach_handle(&mut self, graph: &str) -> Option<HandleEntry<H>> {
        self.core.handles.remove(graph)
    }

    /// Fold an additional named graph into the carrier and pin a typed handle to it,
    /// preserving every already-pinned graph — [`PipelineBundle::accumulate_named_graph`]
    /// without the deep copy.
    ///
    /// `graph_quads` is APPENDED as a retained source placed in
    /// [`GraphPlacement::Named`] carrying `graph`, through
    /// [`CompositeDatasetView::extend`]: the sources already composed keep their
    /// aliasing, only the new contribution is aliased against them, and no row is
    /// copied. Its blank scopes are standardized apart from everything already
    /// composed, exactly as [`RdfDataset::union`] would have standardized them on the
    /// flat carrier, so the two carriers stay in agreement about identity.
    ///
    /// Admission is re-checked CUMULATIVELY inside the append: a carrier that has
    /// grown past its own [`ViewLimits`] refuses the contribution
    /// ([`PipelineBundleError::AdmissionBreach`]) rather than publishing a view it
    /// was never admitted to hold.
    ///
    /// ## Exact invalidation
    ///
    /// Containment (checked first, and hard) means the append can only have touched
    /// `graph`. This carrier therefore RESEATS its per-graph memo with every entry
    /// except `graph`'s: the bundle digest, the pipeline root and `graph`'s own
    /// digest are the only values invalidated, and an untouched graph's digest is
    /// never recomputed. Reseating rather than clearing is also what keeps a clone
    /// taken BEFORE this call reading digests of its OWN content.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::GraphContainment`] if `graph_quads` reaches outside
    /// `graph`; [`PipelineBundleError::AdmissionBreach`] if appending it breaches this
    /// carrier's ceilings; [`PipelineBundleError::HandleDigestMismatch`] if the append
    /// would shift an already-pinned graph's digest;
    /// [`PipelineBundleError::Canonicalization`] if a digest cannot be computed.
    pub fn accumulate_named_graph(
        &mut self,
        graph: impl Into<HandleKey>,
        graph_quads: &Arc<RdfDataset>,
        payload: H,
    ) -> Result<(), PipelineBundleError> {
        let graph = graph.into();
        // Containment first: nothing is appended until the contribution is known to
        // stay inside the graph it claims. On the view carrier this is doubly
        // load-bearing — `GraphPlacement::Named` REWRITES graph slots, so an
        // unchecked contribution would be silently relocated.
        check_containment(graph_quads, &graph)?;
        let prior = self.core.pinned();
        // Whether some source already answers for this graph decides whether the new
        // base can be its sole owner. Read BEFORE the append.
        let already_addressed = self.named_graph_iris().contains(&graph);

        let source = CompositeSource::new(Arc::clone(graph_quads))
            .with_graph_placement(GraphPlacement::Named(TermValue::iri(graph.clone())));
        let next = self
            .view
            .extend(source, self.limits)
            .map_err(PipelineBundleError::AdmissionBreach)?;
        self.view = Arc::new(next);

        let guard = self.ledger.retain_dataset(graph_quads);
        self.retained.push(guard.clone());
        if already_addressed {
            // Two sources now answer for this graph, so no single base's memo can
            // stand for it.
            self.sole.remove(&graph);
        } else {
            self.sole.insert(
                graph.clone(),
                SoleGraph {
                    base: Arc::clone(graph_quads),
                    guard,
                },
            );
        }

        // Exact invalidation: every entry except this graph's stays valid.
        let mut seed = self.core.cache_snapshot();
        seed.remove(&graph);
        self.core.reseat_cache(seed);

        for (k, pinned) in prior {
            let actual = self.graph_digest(&k)?;
            BundleCore::<H>::verify_pin(&k, pinned, actual)?;
        }
        let content_digest = self.graph_digest(&graph)?;
        self.core
            .handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// Materialize the composed surface into one frozen [`RdfDataset`], at an
    /// explicit ownership boundary.
    ///
    /// Delegates to [`CompositeDatasetView::materialize`], which drives the single
    /// reconstruction loop in [`dataset_from_view`](super::pack::dataset_from_view)
    /// and charges the freeze and the materialization on the view's own
    /// [`ViewWork`](super::view_accounting::ViewWork). Nothing is reimplemented here,
    /// so the carrier cannot acquire a second, differently-accounted freeze path.
    ///
    /// # Errors
    ///
    /// Whatever the view's materialization returns, unchanged.
    pub fn materialize(&self) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
        self.view.materialize()
    }

    /// Carry this carrier's whole state back onto a FLAT [`PipelineBundle`] over the
    /// materialized surface — the inverse of
    /// [`PipelineBundle::to_view_bundle`].
    ///
    /// Every pinned handle is RE-VERIFIED against the materialized dataset before the
    /// flat carrier is published: materialization preserves content, so the pins hold,
    /// and checking rather than assuming is what keeps the conversion fail-closed.
    /// The conversion costs one freeze, charged on the view's own work counters.
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::Materialization`] if the surface cannot be frozen;
    /// [`PipelineBundleError::Canonicalization`] if a pinned graph cannot be
    /// canonicalized; [`PipelineBundleError::HandleDigestMismatch`] if a pin does not
    /// survive the boundary.
    pub fn to_flat_bundle(&self) -> Result<PipelineBundle<H>, PipelineBundleError>
    where
        H: Clone,
    {
        self.clone().into_flat_bundle()
    }

    /// [`to_flat_bundle`](Self::to_flat_bundle) by value, for a payload `H` that is
    /// not `Clone`.
    ///
    /// # Errors
    ///
    /// Exactly [`to_flat_bundle`](Self::to_flat_bundle)'s.
    pub fn into_flat_bundle(self) -> Result<PipelineBundle<H>, PipelineBundleError> {
        let dataset = self
            .materialize()
            .map_err(PipelineBundleError::Materialization)?;
        for (graph, entry) in &self.core.handles {
            let actual = try_graph_digest_view(&*dataset, graph)
                .map_err(|source| canon_refusal(CanonScopeName::Graph(graph.clone()), source))?;
            BundleCore::<H>::verify_pin(graph, entry.content_digest, actual)?;
        }
        let mut core = self.core;
        // The flat carrier canonicalizes its own dataset; the per-graph values are
        // equal, but the memo is reseated so the two carriers never share one.
        let seed = core.cache_snapshot();
        core.reseat_cache(seed);
        Ok(PipelineBundle { dataset, core })
    }
}

/// Wrap a canonicalization refusal with what was being canonicalized.
fn canon_refusal(scope: CanonScopeName, source: CanonError) -> PipelineBundleError {
    PipelineBundleError::Canonicalization { scope, source }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::provenance::OriginKind;
    use crate::{RdfLookasideKind, RdfLookasideResource, TermId};

    /// A trivial synthetic handle payload — C1 has no real pipeline handle types yet,
    /// so the pin check is exercised with this stand-in. Pipeline-side payloads plug
    /// into the same lane.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SyntheticHandle {
        note: String,
    }

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    /// Build a dataset with one default-graph quad and one quad in named graph `g`.
    fn dataset_with_named_graph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let go = iri(&mut b, "go");
        let g = b.intern_iri("http://example.org/graph");
        b.push_quad(s, p, o, None); // default graph
        b.push_quad(s, p, go, Some(g)); // named graph
        b.freeze().expect("valid")
    }

    fn empty_bundle() -> PipelineBundle<SyntheticHandle> {
        PipelineBundle::new(
            dataset_with_named_graph(),
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
        )
    }

    #[test]
    fn new_bundle_exposes_parts_and_empty_handles() {
        let bundle = empty_bundle();
        assert_eq!(bundle.dataset().quad_count(), 2);
        assert!(bundle.lookaside().is_empty());
        assert!(bundle.blobs().is_empty());
        assert!(bundle.handles().is_empty());
    }

    #[test]
    fn pin_handle_matching_digest_succeeds() {
        let mut bundle = empty_bundle();
        let graph = "http://example.org/graph";
        let digest = bundle.graph_digest(graph);
        let payload = SyntheticHandle {
            note: "logic-program".to_owned(),
        };
        bundle
            .pin_handle(graph, payload.clone(), digest)
            .expect("matching digest pins");
        assert_eq!(bundle.handle(graph).map(|h| &h.payload), Some(&payload));
    }

    #[test]
    fn pin_handle_mismatched_digest_hard_fails() {
        let mut bundle = empty_bundle();
        let graph = "http://example.org/graph";
        // A digest of unrelated bytes — cannot equal the backing graph's canon.
        let wrong = ContentDigest::of(b"not the graph");
        let err = bundle
            .pin_handle(
                graph,
                SyntheticHandle {
                    note: "bad".to_owned(),
                },
                wrong,
            )
            .expect_err("mismatched digest must hard-fail");
        assert!(matches!(
            err,
            PipelineBundleError::HandleDigestMismatch { .. }
        ));
        // The bundle is unchanged on failure.
        assert!(bundle.handle(graph).is_none());
    }

    /// A dataset with one quad in named graph `graph` (object `obj`), nothing else.
    fn named_graph_dataset(graph: &str, obj: &str) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
        let o = iri(&mut b, obj);
        let g = b.intern_iri(graph);
        b.push_quad(s, p, o, Some(g));
        b.freeze().expect("valid")
    }

    #[test]
    fn accumulate_named_graph_is_additive_and_pins() {
        let mut bundle = empty_bundle();
        // Pin a handle to the existing named graph first.
        let g1 = "http://example.org/graph";
        let d1 = bundle.graph_digest(g1);
        bundle
            .pin_handle(
                g1,
                SyntheticHandle {
                    note: "first".to_owned(),
                },
                d1,
            )
            .expect("first pin");
        // Accumulate a NEW disjoint named graph + handle.
        let g2 = "http://example.org/graph2";
        let g2_ds = named_graph_dataset(g2, "z");
        bundle
            .accumulate_named_graph(
                g2,
                &g2_ds,
                SyntheticHandle {
                    note: "second".to_owned(),
                },
            )
            .expect("additive accumulate succeeds");
        // The first handle's pinned digest is UNCHANGED (additive), and both are present.
        assert_eq!(bundle.handle(g1).map(|h| h.content_digest), Some(d1));
        assert!(bundle.handle(g2).is_some());
        // The new graph's content actually rides the carrier now.
        assert_ne!(
            bundle.graph_digest(g2),
            bundle.graph_digest("http://example.org/absent")
        );
    }

    #[test]
    fn accumulate_disturbing_a_pinned_graph_hard_fails() {
        let mut bundle = empty_bundle();
        let g1 = "http://example.org/graph";
        let d1 = bundle.graph_digest(g1);
        bundle
            .pin_handle(
                g1,
                SyntheticHandle {
                    note: "first".to_owned(),
                },
                d1,
            )
            .expect("first pin");
        // Folding more quads INTO the already-pinned graph shifts its digest → HARD fail.
        let extra = named_graph_dataset(g1, "extra");
        let err = bundle
            .accumulate_named_graph(
                g1,
                &extra,
                SyntheticHandle {
                    note: "x".to_owned(),
                },
            )
            .expect_err("disturbing a pinned graph must hard-fail");
        assert!(matches!(
            err,
            PipelineBundleError::HandleDigestMismatch { .. }
        ));
    }

    #[test]
    fn graph_digest_distinguishes_graphs_and_is_isolated() {
        let bundle = empty_bundle();
        // The named graph's projection (one quad) differs from an absent graph
        // (empty projection → canon of "").
        let present = bundle.graph_digest("http://example.org/graph");
        let absent = bundle.graph_digest("http://example.org/missing");
        assert_ne!(present, absent, "present vs empty projection differ");
        // The absent-graph digest is the canon of the empty dataset.
        let empty_ds = RdfDatasetBuilder::new().freeze().expect("empty");
        assert_eq!(
            absent,
            ContentDigest::of(canonicalize(&empty_ds).nquads.as_bytes())
        );
    }

    /// A dataset in which `<g1>` carries a base quad, a plain quad about the reifier
    /// id `r1`, and `r1`'s declaration + annotation. When `with_g2` is set, `<g2>`
    /// carries statement-layer rows for the SAME reifier id — content that belongs to
    /// `<g2>` alone and that `<g1>`'s digest must not see.
    fn shared_reifier_dataset(with_g2: bool) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = iri(&mut b, "a");
        let related = iri(&mut b, "related");
        let bo = iri(&mut b, "b");
        let c = iri(&mut b, "c");
        let r1 = iri(&mut b, "r1");
        let kind = iri(&mut b, "kind");
        let record = iri(&mut b, "record");
        let source = iri(&mut b, "source");
        let ledger = iri(&mut b, "ledger");
        let elsewhere = iri(&mut b, "elsewhere");
        let g1 = b.intern_iri("http://example.org/g1");
        let g2 = b.intern_iri("http://example.org/g2");

        b.push_quad(a, related, bo, Some(g1));
        b.push_quad(r1, kind, record, Some(g1));
        let t1 = b.intern_triple(a, related, bo);
        b.push_reifier_in_graph(r1, t1, Some(g1));
        b.push_annotation_in_graph(r1, source, ledger, Some(g1));

        if with_g2 {
            b.push_quad(a, related, c, Some(g2));
            let t2 = b.intern_triple(a, related, c);
            b.push_reifier_in_graph(r1, t2, Some(g2));
            b.push_annotation_in_graph(r1, source, elsewhere, Some(g2));
        }
        b.freeze().expect("valid")
    }

    fn bundle_over(dataset: Arc<RdfDataset>) -> PipelineBundle<SyntheticHandle> {
        PipelineBundle::new(
            dataset,
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
        )
    }

    /// `<g1>`'s digest is a function of `<g1>`'s content alone: adding a declaration
    /// and an annotation for the same reifier id to `<g2>` leaves it untouched. A
    /// digest that moved when an unrelated graph changed would break every pin over
    /// `<g1>` for a reason that has nothing to do with `<g1>`.
    #[test]
    fn graph_digest_ignores_another_graphs_statement_layer() {
        let without = bundle_over(shared_reifier_dataset(false));
        let with = bundle_over(shared_reifier_dataset(true));

        assert_eq!(
            without.graph_digest("http://example.org/g1"),
            with.graph_digest("http://example.org/g1"),
            "<g1>'s digest must not observe statement-layer rows asserted in <g2>"
        );

        // Non-vacuity of the fixture: <g2>'s own digest DOES move, and the two graphs
        // do not simply hash alike.
        assert_ne!(
            without.graph_digest("http://example.org/g2"),
            with.graph_digest("http://example.org/g2"),
            "the added rows must be observable somewhere"
        );
        assert_ne!(
            with.graph_digest("http://example.org/g1"),
            with.graph_digest("http://example.org/g2")
        );
    }

    /// The same isolation through the pinning lane: a handle pinned over `<g1>` still
    /// verifies after `<g2>`'s statement layer is folded into the carrier.
    #[test]
    fn accumulating_a_graphs_statement_layer_leaves_a_pinned_graph_alone() {
        let mut bundle = bundle_over(shared_reifier_dataset(false));
        let g1 = "http://example.org/g1";
        let pinned = bundle.graph_digest(g1);
        bundle
            .pin_handle(
                g1,
                SyntheticHandle {
                    note: "g1".to_owned(),
                },
                pinned,
            )
            .expect("pin over <g1>");

        // A fresh dataset holding ONLY <g2>'s rows — base quad plus the statement
        // layer for the reifier id <g1> also uses.
        let g2_only = {
            let mut b = RdfDatasetBuilder::new();
            let a = iri(&mut b, "a");
            let related = iri(&mut b, "related");
            let c = iri(&mut b, "c");
            let r1 = iri(&mut b, "r1");
            let source = iri(&mut b, "source");
            let elsewhere = iri(&mut b, "elsewhere");
            let g2 = b.intern_iri("http://example.org/g2");
            b.push_quad(a, related, c, Some(g2));
            let t2 = b.intern_triple(a, related, c);
            b.push_reifier_in_graph(r1, t2, Some(g2));
            b.push_annotation_in_graph(r1, source, elsewhere, Some(g2));
            b.freeze().expect("valid")
        };

        bundle
            .accumulate_named_graph(
                "http://example.org/g2",
                &g2_only,
                SyntheticHandle {
                    note: "g2".to_owned(),
                },
            )
            .expect("folding <g2> must not disturb the pinned <g1>");
        assert_eq!(bundle.graph_digest(g1), pinned);
    }

    #[test]
    fn digest_is_stable_across_handle_attach_and_detach() {
        let mut bundle = empty_bundle();
        let before = bundle.digest();
        let graph = "http://example.org/graph";
        let digest = bundle.graph_digest(graph);
        bundle
            .pin_handle(
                graph,
                SyntheticHandle {
                    note: "h".to_owned(),
                },
                digest,
            )
            .expect("pin");
        assert_eq!(
            bundle.digest(),
            before,
            "attaching a handle does not change the bundle digest"
        );
        let _ = bundle.detach_handle(graph);
        assert_eq!(
            bundle.digest(),
            before,
            "detaching a handle does not change the bundle digest"
        );
    }

    #[test]
    fn digest_is_sensitive_to_the_dataset() {
        let a = empty_bundle();
        let b = {
            let mut bld = RdfDatasetBuilder::new();
            let (s, p, o) = (
                iri(&mut bld, "s"),
                iri(&mut bld, "p"),
                iri(&mut bld, "DIFFERENT"),
            );
            bld.push_quad(s, p, o, None);
            PipelineBundle::<SyntheticHandle>::new(
                bld.freeze().expect("valid"),
                RdfLookaside::default(),
                Arc::new(ContentStore::new()),
                DatasetProvenance::new(),
            )
        };
        assert_ne!(
            a.digest(),
            b.digest(),
            "a different dataset changes the digest"
        );
    }

    #[test]
    fn digest_is_sensitive_to_a_lookaside_resource() {
        let base = empty_bundle();
        let base_digest = base.digest();
        let mut with_lookaside = RdfLookaside::default();
        with_lookaside.resources.push(
            RdfLookasideResource::new(RdfLookasideKind::Reasoning)
                .with_name("closure")
                .with_digest("deadbeef"),
        );
        let with_resource = PipelineBundle::<SyntheticHandle>::new(
            dataset_with_named_graph(),
            with_lookaside,
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
        );
        assert_ne!(
            with_resource.digest(),
            base_digest,
            "adding a lookaside resource changes the digest"
        );
    }

    #[test]
    fn digest_is_sensitive_to_a_blob() {
        let base_digest = empty_bundle().digest();
        let mut store = ContentStore::new();
        store.insert(b"a blob payload".to_vec());
        let with_blob = PipelineBundle::<SyntheticHandle>::new(
            dataset_with_named_graph(),
            RdfLookaside::default(),
            Arc::new(store),
            DatasetProvenance::new(),
        );
        assert_ne!(
            with_blob.digest(),
            base_digest,
            "adding a blob changes the digest"
        );
    }

    #[test]
    fn digest_is_sensitive_to_the_public_provenance() {
        let base_digest = empty_bundle().digest();
        let mut prov = DatasetProvenance::new();
        let unit = prov.register_unit("slices/core/epistemics", OriginKind::Source);
        let artifact = prov.register_artifact("slices/core/epistemics/epistemics.ttl");
        prov.record_occurrence(
            crate::ir::QuadHandle::from_index(0),
            unit,
            artifact,
            Some("epistemics.ttl:1".to_owned()),
        );
        let with_prov = PipelineBundle::<SyntheticHandle>::new(
            dataset_with_named_graph(),
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            prov,
        );
        assert_ne!(
            with_prov.digest(),
            base_digest,
            "a non-empty public provenance changes the digest"
        );
    }

    /// S0.5: the digest is over the PUBLIC projection, never runtime ids. Two
    /// provenances with the SAME public content but DIFFERENT internal id allocation
    /// order must produce the SAME bundle digest. We allocate the same two
    /// (unit, artifact) occurrences in opposite registration orders — the numeric
    /// `UnitId`/`ArtifactId` differ, but the public names/paths are identical.
    #[test]
    fn digest_excludes_runtime_ids_public_projection_only() {
        let build = |reversed: bool| -> ContentDigest {
            let mut prov = DatasetProvenance::new();
            // Two occurrences sharing one quad handle, registered in one of two
            // internal orders. The PUBLIC content (names, paths, locations) is the
            // same set either way; only the numeric ids differ.
            let specs = [
                ("unit-a", "art-a.ttl", "a:1"),
                ("unit-b", "art-b.ttl", "b:1"),
            ];
            let order: Vec<usize> = if reversed { vec![1, 0] } else { vec![0, 1] };
            for &i in &order {
                let (uname, apath, loc) = specs[i];
                let unit = prov.register_unit(uname, OriginKind::Source);
                let artifact = prov.register_artifact(apath);
                prov.record_occurrence(
                    crate::ir::QuadHandle::from_index(0),
                    unit,
                    artifact,
                    Some(loc.to_owned()),
                );
            }
            PipelineBundle::<SyntheticHandle>::new(
                dataset_with_named_graph(),
                RdfLookaside::default(),
                Arc::new(ContentStore::new()),
                prov,
            )
            .digest()
        };
        assert_eq!(
            build(false),
            build(true),
            "identical public provenance in a different internal id order must digest identically"
        );
    }
}

/// The surface [`BundleCore`] added to the FLAT carrier: containment, the reseated
/// memo, and the pipeline root. The cross-carrier agreement these underwrite is
/// proved end to end in the crate's shared-view integration suite.
#[cfg(test)]
mod core_tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::{BlankScope, RdfLiteral, TermId};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Note(&'static str);

    const G1: &str = "http://example.org/g1";
    const G2: &str = "http://example.org/g2";

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    /// Two named graphs, a default-graph quad and a declared-empty named graph, so
    /// every pipeline-root section has something to say.
    fn base() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        let g1 = b.intern_iri(G1);
        let g2 = b.intern_iri(G2);
        let declared = b.intern_iri("http://example.org/declared");
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g1));
        b.push_quad(o, p, s, Some(g2));
        b.declare_named_graph(declared);
        b.freeze().expect("valid")
    }

    fn bundle(dataset: Arc<RdfDataset>) -> PipelineBundle<Note> {
        PipelineBundle::new(
            dataset,
            RdfLookaside::default(),
            Arc::new(ContentStore::new()),
            DatasetProvenance::new(),
        )
    }

    /// One quad in `graph`, plus a co-referent blank subject so the contribution is
    /// not IRI-only.
    fn contribution(graph: &str) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let o = iri(&mut b, "o");
        let anon = b.intern_blank("n", BlankScope::DEFAULT);
        let g = b.intern_iri(graph);
        b.push_quad(anon, p, o, Some(g));
        b.freeze().expect("valid")
    }

    #[test]
    fn the_pipeline_root_names_every_iri_graph_including_a_declared_empty_one() {
        let carrier = bundle(base());
        assert_eq!(
            carrier.named_graph_iris(),
            vec![
                "http://example.org/declared".to_owned(),
                G1.to_owned(),
                G2.to_owned()
            ],
            "declared-but-empty graphs are leaves too"
        );
        // The root is domain-separated from the bundle digest, and it moves when a
        // leaf moves.
        assert_ne!(carrier.pipeline_root(), carrier.digest());

        let mut grown = bundle(base());
        grown
            .accumulate_named_graph(G2, &contribution(G2), Note("g2"))
            .expect("single-graph accumulate succeeds");
        assert_ne!(
            grown.pipeline_root(),
            carrier.pipeline_root(),
            "folding a graph in must move the root"
        );
        assert_eq!(
            grown.graph_digest(G1),
            carrier.graph_digest(G1),
            "and must leave every other leaf alone"
        );
    }

    /// A declaration-only graph is invisible to the canonical document — it owns no
    /// row — but the root addresses graphs by name, so it sees it. That is the whole
    /// reason the root is an ADDITIONAL identity rather than a restatement.
    #[test]
    fn the_root_discriminates_a_declaration_the_bundle_digest_cannot_see() {
        let without = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            b.push_quad(s, p, o, None);
            b.freeze().expect("valid")
        };
        let with = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let declared = b.intern_iri(G1);
            b.push_quad(s, p, o, None);
            b.declare_named_graph(declared);
            b.freeze().expect("valid")
        };
        assert_eq!(
            bundle(without.clone()).digest(),
            bundle(with.clone()).digest()
        );
        assert_ne!(
            bundle(without).pipeline_root(),
            bundle(with).pipeline_root()
        );
    }

    /// The residue leaf carries what no IRI-named leaf can address: the default graph
    /// and any blank-named graph. Moving a row between them moves the root.
    #[test]
    fn the_residue_leaf_separates_the_default_graph_from_a_blank_named_one() {
        let build = |blank_graph: bool| {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let o = b.intern_literal(RdfLiteral::language_tagged("مرحبا", "ar"));
            let g = b.intern_blank("anon-graph", BlankScope(3));
            b.push_quad(s, p, o, blank_graph.then_some(g));
            bundle(b.freeze().expect("valid"))
        };
        assert_ne!(build(false).pipeline_root(), build(true).pipeline_root());
        // Non-vacuity: the same content under a differently NUMBERED blank scope is
        // the same carrier, because canonicalization issues the label.
        let renumbered = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
            let o = b.intern_literal(RdfLiteral::language_tagged("مرحبا", "ar"));
            let g = b.intern_blank("anon-graph", BlankScope(11));
            b.push_quad(s, p, o, Some(g));
            bundle(b.freeze().expect("valid"))
        };
        assert_eq!(build(true).pipeline_root(), renumbered.pipeline_root());
    }

    /// Every layer of the containment rule refuses, and the neighbouring
    /// single-graph contribution — the case the rule exists to let through —
    /// still succeeds.
    #[test]
    fn containment_refuses_each_layer_and_admits_the_single_graph_twin() {
        let stray_default = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let g = b.intern_iri(G2);
            b.push_quad(s, p, o, Some(g));
            b.push_quad(o, p, s, None);
            b.freeze().expect("valid")
        };
        let stray_reifier = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let r = iri(&mut b, "r");
            let g = b.intern_iri(G2);
            let other = b.intern_iri(G1);
            b.push_quad(s, p, o, Some(g));
            let t = b.intern_triple(s, p, o);
            b.push_reifier_in_graph(r, t, Some(other));
            b.freeze().expect("valid")
        };
        let stray_annotation = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let r = iri(&mut b, "r");
            let g = b.intern_iri(G2);
            let other = b.intern_iri(G1);
            b.push_quad(s, p, o, Some(g));
            let t = b.intern_triple(s, p, o);
            b.push_reifier_in_graph(r, t, Some(g));
            b.push_annotation_in_graph(r, p, o, Some(other));
            b.freeze().expect("valid")
        };
        let stray_declaration = {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let g = b.intern_iri(G2);
            let other = b.intern_iri(G1);
            b.push_quad(s, p, o, Some(g));
            b.declare_named_graph(other);
            b.freeze().expect("valid")
        };

        for (name, contribution, layer) in [
            ("default-graph quad", stray_default, GraphLayer::Quad),
            ("reifier row", stray_reifier, GraphLayer::Reifier),
            ("annotation row", stray_annotation, GraphLayer::Annotation),
            (
                "graph declaration",
                stray_declaration,
                GraphLayer::Declaration,
            ),
        ] {
            let mut carrier = bundle(base());
            let before = carrier.digest();
            let err = carrier
                .accumulate_named_graph(G2, &contribution, Note("x"))
                .expect_err("a stray row outside the target graph must hard-fail");
            assert!(
                matches!(&err, PipelineBundleError::GraphContainment { graph, layer: found, .. }
                    if graph == G2 && *found == layer),
                "{name}: {err}"
            );
            assert_eq!(
                carrier.digest(),
                before,
                "{name}: a refused contribution folds nothing"
            );
            assert!(carrier.handles().is_empty());
        }

        // The twin that IS contained still folds.
        let mut carrier = bundle(base());
        carrier
            .accumulate_named_graph(G2, &contribution(G2), Note("ok"))
            .expect("a contained contribution must still be admitted");
        assert!(carrier.handle(G2).is_some());
    }

    /// Reseating rather than clearing: a clone taken BEFORE the accumulation reads
    /// digests of its OWN dataset, not of the carrier that grew.
    #[test]
    fn a_clone_taken_before_accumulate_keeps_its_own_memo() {
        let mut carrier = bundle(base());
        let before = carrier.graph_digest(G2);
        let snapshot = carrier.clone();
        carrier
            .accumulate_named_graph(G2, &contribution(G2), Note("grown"))
            .expect("accumulate");
        let after = carrier.graph_digest(G2);
        assert_ne!(before, after, "the fixture must actually move <g2>");
        assert_eq!(
            snapshot.graph_digest(G2),
            before,
            "the pre-accumulate clone must answer for its own content"
        );
        assert_eq!(snapshot.digest(), bundle(base()).digest());
    }
}

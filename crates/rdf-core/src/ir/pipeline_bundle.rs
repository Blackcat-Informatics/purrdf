// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `PipelineBundle<H>` — the carrier that travels through the build pipeline:
//! the frozen hot graph, its out-of-band lookaside, the content-addressed blob
//! store, the provenance sidecar, and a typed-handle lane.
//!
//! ## Kernel boundary (#885)
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
//! The typed-handle lane contributes NOTHING to the digest: attaching or detaching
//! a handle leaves [`digest`](PipelineBundle::digest) byte-stable. This is the
//! contract a downstream cache keys on — the dataset/lookaside/blobs/public-
//! provenance are the content, the handles are derived views over it.
//!
//! ## Pin invariant (hard-fail)
//!
//! Attaching a handle ALWAYS checks that its pinned digest equals the canonical
//! digest of its backing named graph; a mismatch is a HARD failure
//! ([`PipelineBundleError::HandleDigestMismatch`]). The check runs on every attach,
//! not only in tests, so a bundle can never carry a handle that disagrees with the
//! graph it claims to project. Concrete pipeline-side handle types plug into this
//! lane unchanged.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use super::canon::canonicalize;
use super::dataset::RdfDataset;
use crate::provenance::DatasetProvenance;
use crate::{ContentDigest, ContentStore, RdfLookaside};

/// Field separator inside the digest fold (mirrors `StageProduct::from_artifacts`).
const SEP_FIELD: u8 = 0x1f;
/// Record separator inside the digest fold.
const SEP_RECORD: u8 = 0x1e;
/// Section separator between the four digest contributions.
const SEP_SECTION: u8 = 0x1d;

/// The key identifying the named graph a typed handle backs. An IRI string is the
/// stable, dataset-independent name of the graph the handle projects.
pub type HandleKey = String;

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

/// An error from attaching a typed handle to a [`PipelineBundle`].
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
        }
    }
}

impl std::error::Error for PipelineBundleError {}

/// The pipeline carrier: the frozen hot graph plus its out-of-band material and a
/// typed-handle lane.
///
/// Construct through [`PipelineBundle::new`] and the builder methods; access fields
/// through the narrow accessor methods. All handle-critical fields are PRIVATE so
/// that downstream code cannot replace `dataset` after `pin_handle` or insert into
/// `handles` directly, which would silently break the pin invariant. Generic over
/// the typed-handle payload `H` — see the module docs for the kernel-boundary
/// rationale.
#[derive(Debug, Clone)]
pub struct PipelineBundle<H> {
    /// The immutable, value-interned RDF 1.2 dataset — the hot graph.
    /// PRIVATE: replacing this after `pin_handle` would corrupt the pin invariant.
    dataset: Arc<RdfDataset>,
    /// Structured non-triple companion material (typed sidecar resources, blobs by
    /// reference, segments, metadata, …).
    /// PRIVATE: folded into `digest()`.
    lookaside: RdfLookaside,
    /// The single owner of blob payload bytes (by-reference doctrine).
    /// PRIVATE: folded into `digest()`.
    blobs: Arc<ContentStore>,
    /// The provenance sidecar (units / artifacts / origin-sets / occurrences).
    /// PRIVATE: folded into `digest()`.
    provenance: DatasetProvenance,
    /// The typed-handle lane: backing-graph IRI → typed payload + pinned digest.
    /// PRIVATE: must only be mutated through `pin_handle` / `detach_handle` to
    /// preserve the pin invariant. EXCLUDED from [`digest`](Self::digest).
    handles: BTreeMap<HandleKey, HandleEntry<H>>,
    /// Memoized per-graph canonical digests for [`graph_digest`](Self::graph_digest).
    /// PRIVATE, NON-semantic: the `dataset` is frozen, so each named graph's canonical
    /// digest is stable for the bundle's life — caching it lets the
    /// `pinned = graph_digest(g); pin_handle(g, …, pinned)` pattern canonicalize each
    /// backing graph ONCE instead of twice. EXCLUDED from [`digest`](Self::digest) (a
    /// pure memo). `Arc<Mutex<…>>` so it rides `Clone` (a clone shares the same frozen
    /// dataset, so the cached digests stay valid) and is `Send`+`Sync` across the
    /// reasoning engine lock.
    digest_cache: Arc<Mutex<BTreeMap<String, ContentDigest>>>,
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
            lookaside,
            blobs,
            provenance,
            handles: BTreeMap::new(),
            digest_cache: Arc::new(Mutex::new(BTreeMap::new())),
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
        &self.lookaside
    }

    /// Borrow the blob store.
    pub fn blobs(&self) -> &ContentStore {
        &self.blobs
    }

    /// Borrow the provenance sidecar.
    pub fn provenance(&self) -> &DatasetProvenance {
        &self.provenance
    }

    /// The typed handle for a backing graph IRI, if one is attached.
    pub fn handle(&self, graph: &str) -> Option<&HandleEntry<H>> {
        self.handles.get(graph)
    }

    /// Borrow the entire typed-handle map (read-only). The map is only mutated
    /// through [`pin_handle`](Self::pin_handle) / [`detach_handle`](Self::detach_handle)
    /// to preserve the pin invariant.
    pub fn handles(&self) -> &BTreeMap<HandleKey, HandleEntry<H>> {
        &self.handles
    }

    /// Replace the provenance sidecar in place.
    ///
    /// This is the controlled mutator for the provenance field; it does NOT affect
    /// the [`digest`](Self::digest) correctness since the digest reads the provenance
    /// through the public projection. Used by the pipeline scheduler to thread the
    /// per-stage provenance into the produced carrier.
    pub fn set_provenance(&mut self, provenance: DatasetProvenance) {
        self.provenance = provenance;
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
        if actual != content_digest {
            return Err(PipelineBundleError::HandleDigestMismatch {
                graph,
                pinned: content_digest,
                actual,
            });
        }
        self.handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// Detach the typed handle for `graph`, returning it if present. Detaching does
    /// NOT change [`digest`](Self::digest) (the handle lane is excluded).
    pub fn detach_handle(&mut self, graph: &str) -> Option<HandleEntry<H>> {
        self.handles.remove(graph)
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
    /// Accumulation is ADDITIVE: it MUST NOT change any already-pinned graph's digest.
    /// A violation HARD-fails with [`PipelineBundleError::HandleDigestMismatch`] for the
    /// disturbed graph (no-optionality) — the carrier never silently rewrites a graph a
    /// downstream handle already pinned. This is the single-assembly integrity invariant
    /// (there is only ever one copy of each graph, so no cross-copy check is needed).
    ///
    /// # Errors
    ///
    /// [`PipelineBundleError::HandleDigestMismatch`] if folding `graph_quads` would shift
    /// an already-pinned graph's digest.
    pub fn accumulate_named_graph(
        &mut self,
        graph: impl Into<HandleKey>,
        graph_quads: &RdfDataset,
        payload: H,
    ) -> Result<(), PipelineBundleError> {
        let graph = graph.into();
        // Record every already-pinned graph's digest to enforce the additive invariant.
        let prior: Vec<(HandleKey, ContentDigest)> = self
            .handles
            .iter()
            .map(|(k, e)| (k.clone(), e.content_digest))
            .collect();
        // Fold the new named graph into the single carrier dataset.
        self.dataset = Arc::new(RdfDataset::union(&[&self.dataset, graph_quads]));
        // The dataset changed: invalidate the per-graph digest memo so the additive
        // invariant below (and every later `graph_digest`) recomputes against the NEW
        // dataset — a stale cache would hide a disturbed pinned graph (fail-closed).
        self.digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .clear();
        // Additive invariant: no previously pinned graph may have shifted.
        for (k, pinned) in prior {
            let actual = self.graph_digest(&k);
            if actual != pinned {
                return Err(PipelineBundleError::HandleDigestMismatch {
                    graph: k,
                    pinned,
                    actual,
                });
            }
        }
        // Pin the new handle to its now-present backing graph.
        let content_digest = self.graph_digest(&graph);
        self.handles
            .insert(graph, HandleEntry::new(payload, content_digest));
        Ok(())
    }

    /// The canonical [`ContentDigest`] of the named graph `graph` — the subgraph of
    /// the dataset whose quads carry `g == <graph>`, canonicalized to N-Quads.
    ///
    /// Built by projecting the matching quads into a fresh dataset and hashing its
    /// canonical form. The RDF 1.2 reifier/annotation side-tables are graph-scopeless
    /// in this IR (a reifier binding carries no graph dimension), so they travel with
    /// the projection in whole — a handle over a reified subgraph therefore pins over
    /// the same statement layer the dataset carries. This is the value a handle's
    /// pinned digest is checked against in [`pin_handle`](Self::pin_handle).
    #[must_use]
    pub fn graph_digest(&self, graph: &str) -> ContentDigest {
        // Memoized: the frozen dataset makes each graph's canonical digest stable, so
        // the second call in the `pinned = graph_digest(g); pin_handle(g, …, pinned)`
        // pattern reuses the first canonicalization instead of recomputing it.
        if let Some(cached) = self
            .digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .get(graph)
        {
            return *cached;
        }
        let subgraph = self.dataset.project_named_graph(graph);
        let digest = ContentDigest::of(canonicalize(&subgraph).nquads.as_bytes());
        self.digest_cache
            .lock()
            .expect("digest_cache mutex poisoned")
            .insert(graph.to_string(), digest);
        digest
    }

    /// The content [`ContentDigest`] of this bundle: a SHA-256 fold over the
    /// dataset's canonical hash, the SORTED lookaside resource digests, the SORTED
    /// blob digests, and the runtime-id-free public provenance projection. The
    /// typed-handle lane contributes NOTHING (see the module docs).
    #[must_use]
    pub fn digest(&self) -> ContentDigest {
        let mut hasher = Sha256::new();

        // 1. The canonical N-Quads hash of the dataset (RDF-1.2 overlay included).
        hasher.update(canonicalize(&self.dataset).nquads.as_bytes());
        hasher.update([SEP_SECTION]);

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

        let out = hasher.finalize();
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&out);
        ContentDigest::from_raw(buf)
    }
}

impl RdfDataset {
    /// A fresh owned `RdfDataset` snapshotting this one's frozen tables. The
    /// fallback for the rare case a freshly-frozen `Arc` is shared; the lazy caches
    /// rebuild on demand. Crate-internal — the public deep-copy path is
    /// [`union`](RdfDataset::union) of a single input.
    pub(crate) fn owned_snapshot(&self) -> Self {
        Self::union(&[self])
    }
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

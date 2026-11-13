// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical document path's stop checkpoints and provider reservations.
//!
//! A canonical document is a stored content identity, and a caller meters it
//! through two things this file pins from the public surface: how often the
//! caller's stop signal is polled, and the largest reservation each admission
//! account ever asks its provider for. Document rows are frozen against the values
//! the previous document construction produced: the poll count and the phase
//! boundary must not move, and a reservation may only shrink. Digest-only rows
//! record the digest route's own totals and keep the earlier maxima as the bound.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use purrdf_core::{
    BlankScope, CanonHash, CanonPhase, CanonicalizationControl, ContentDigest, DatasetView,
    FallibleDatasetView, GraphMatch, OwnedWorkspaceReservation, QuadIds, RDFC_V2_CORPUS_POLICY,
    RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, RdfTextDirection, StopSignal,
    TermId, TermRef, TermValue, ViewOperationStatus, WorkspaceReservation,
    try_canonical_blake3_view, try_canonical_sha256_view, try_canonicalize_flat_view,
    try_canonicalize_sha256_view, try_canonicalize_view, try_flat_blake3_view,
    try_flat_digest_view, try_graph_digest_view,
};

const GRAPH: &str = "https://example.org/g/1";
const LIST: &str = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/List";

/// The statement-free fixture: IRIs only, half the lines in named graphs.
fn blank_free() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let preds: Vec<_> = (0..4)
        .map(|k| b.intern_iri(&format!("https://example.org/p/{k}")).unwrap())
        .collect();
    let graphs: Vec<_> = (0..3)
        .map(|k| b.intern_iri(&format!("https://example.org/g/{k}")).unwrap())
        .collect();
    for i in 0..60 {
        let s = b
            .intern_iri(&format!("https://example.org/s/{i:04}"))
            .unwrap();
        let o = b
            .intern_iri(&format!("https://example.org/o/{i:04}"))
            .unwrap();
        let g = (i % 2 == 1).then(|| graphs[(i / 2) % 3]);
        b.push_quad(s, preds[i % 4], o, g).unwrap();
    }
    b.freeze().unwrap()
}

/// Blank reifiers with three annotations each; every blank differs at first degree.
fn reifier() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let preds: Vec<_> = (0..3)
        .map(|k| b.intern_iri(&format!("https://example.org/q/{k}")).unwrap())
        .collect();
    let notes: Vec<_> = (0..3)
        .map(|k| b.intern_iri(&format!("https://example.org/a/{k}")).unwrap())
        .collect();
    for j in 0..14 {
        let r = b
            .intern_blank(&format!("st{j}"), BlankScope::DEFAULT)
            .unwrap();
        let s = b
            .intern_iri(&format!("https://example.org/r/{j:03}/s"))
            .unwrap();
        let o = b
            .intern_iri(&format!("https://example.org/r/{j:03}/o"))
            .unwrap();
        let t = b.intern_triple(s, preds[j % 3], o).unwrap();
        b.push_reifier_in_graph(r, t, None).unwrap();
        let v0 = b
            .intern_iri(&format!("https://example.org/v/{j:03}"))
            .unwrap();
        let v1 = b
            .intern_literal(RdfLiteral::simple(format!("value {j} \"q\" \\ s")))
            .unwrap();
        let v2 = b
            .intern_literal(RdfLiteral::language_tagged(format!("label {j}"), "en"))
            .unwrap();
        for (k, v) in [v0, v1, v2].into_iter().enumerate() {
            b.push_annotation_in_graph(r, notes[k], v, None).unwrap();
        }
    }
    b.freeze().unwrap()
}

/// Nested triple terms, more than ten blanks, a blank graph, a composite literal
/// that embeds blank labels, and the language, direction and datatype set.
fn mixed() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri("https://example.org/p").unwrap();
    let q = b.intern_iri("https://example.org/q").unwrap();
    let s = b.intern_iri("https://example.org/s").unwrap();
    let blanks: Vec<_> = (0..12)
        .map(|k| {
            b.intern_blank(&format!("b{k}"), BlankScope::DEFAULT)
                .unwrap()
        })
        .collect();
    let t1 = b.intern_triple(s, p, blanks[0]).unwrap();
    let t2 = b.intern_triple(blanks[1], q, t1).unwrap();
    let t3 = b.intern_triple(blanks[2], p, t2).unwrap();
    b.push_quad(s, p, t3, None).unwrap();
    b.push_quad(blanks[3], q, t2, Some(blanks[4])).unwrap();
    for (k, pair) in blanks.windows(2).enumerate() {
        b.push_quad(pair[0], if k % 2 == 0 { p } else { q }, pair[1], None)
            .unwrap();
    }
    let lits = [
        RdfLiteral::simple("plain"),
        RdfLiteral::simple("tab\t quote\" back\\ nl\n"),
        RdfLiteral::simple(""),
        RdfLiteral::language_tagged("hello", "en-us"),
        RdfLiteral::typed("42", "http://www.w3.org/2001/XMLSchema#integer"),
        RdfLiteral {
            lexical_form: "dir".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        },
        RdfLiteral::typed("[_:b3, 1, _:b5]", LIST),
        RdfLiteral::typed(
            "{\"k\": _:b7}",
            "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map",
        ),
    ];
    for (k, lit) in lits.into_iter().enumerate() {
        let o = b.intern_literal(lit).unwrap();
        b.push_quad(blanks[5 + k % 6], p, o, (k % 3 == 0).then_some(blanks[11]))
            .unwrap();
    }
    b.freeze().unwrap()
}

fn fixtures() -> [(&'static str, Arc<RdfDataset>); 3] {
    [
        ("blank_free", blank_free()),
        ("reifier", reifier()),
        ("mixed", mixed()),
    ]
}

/// One admission account as a provider sees it: the largest reservation it was
/// asked for, and a ceiling above which it refuses.
#[derive(Debug)]
struct Account {
    max: AtomicU64,
    limit: u64,
}

impl Account {
    fn new(limit: u64) -> Arc<Self> {
        Arc::new(Self {
            max: AtomicU64::new(0),
            limit,
        })
    }
    fn max(&self) -> u64 {
        self.max.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Refused(u64);

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the provider refused a reservation of {} bytes", self.0)
    }
}
impl std::error::Error for Refused {}

struct Reservation(Arc<Account>);

impl WorkspaceReservation for Reservation {
    type Error = Refused;
    fn resize(&mut self, bytes: u64) -> Result<(), Refused> {
        if bytes > self.0.limit {
            return Err(Refused(bytes));
        }
        self.0.max.fetch_max(bytes, Ordering::Relaxed);
        Ok(())
    }
}

/// A frozen dataset that reports every reservation its callers make.
struct Recording {
    inner: Arc<RdfDataset>,
    scratch: Arc<Account>,
    owned: Arc<Account>,
    has_owned: bool,
}

impl Recording {
    fn new(inner: &Arc<RdfDataset>, scratch_limit: u64, owned_limit: u64) -> Self {
        Self {
            inner: Arc::clone(inner),
            scratch: Account::new(scratch_limit),
            owned: Account::new(owned_limit),
            has_owned: true,
        }
    }
}

impl Recording {
    fn without_owned(inner: &Arc<RdfDataset>) -> Self {
        let mut view = Self::new(inner, u64::MAX, u64::MAX);
        view.has_owned = false;
        view
    }
}

impl DatasetView for Recording {
    type Id = TermId;
    type ReadError = Refused;
    type TermGuard<'a> = TermRef<'a, TermId>;
    type ProbePlan = ();

    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads()
    }
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.reifier_quads()
    }
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.annotation_quads()
    }
    fn resolve(&self, id: TermId) -> Result<TermRef<'_, TermId>, Refused> {
        Ok(self.inner.as_ref().resolve(id))
    }
    fn term_id_by_value(&self, value: &TermValue) -> Result<Option<TermId>, Refused> {
        Ok(self.inner.as_ref().term_id_by_value(value))
    }
    fn capabilities(&self) -> RdfStoreCapabilities {
        self.inner.capabilities()
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.inner.quads_for_pattern(s, p, o, g)
    }
    fn term_count(&self) -> u64 {
        self.inner.term_count()
    }
    fn reserve_workspace(
        &self,
        _: u64,
    ) -> Result<impl WorkspaceReservation<Error = Refused> + '_, Refused> {
        Ok(Reservation(Arc::clone(&self.scratch)))
    }
    fn reserve_owned_workspace(
        &self,
        _: u64,
    ) -> Option<Result<OwnedWorkspaceReservation<Refused>, Refused>> {
        self.has_owned.then(|| {
            Ok(Box::new(Reservation(Arc::clone(&self.owned)))
                as OwnedWorkspaceReservation<Refused>)
        })
    }
}

impl FallibleDatasetView for Recording {
    type Error = Refused;
    type Evidence = ();
    fn operation_status(&self) -> ViewOperationStatus<Refused, ()> {
        ViewOperationStatus::Ready { evidence: () }
    }
}

/// Counts polls, and stops at the `n`th when armed.
#[derive(Debug)]
struct Polls {
    seen: AtomicUsize,
    stop_at: usize,
}

impl Polls {
    const fn counting() -> Self {
        Self {
            seen: AtomicUsize::new(0),
            stop_at: usize::MAX,
        }
    }
    const fn stopping_at(n: usize) -> Self {
        Self {
            seen: AtomicUsize::new(0),
            stop_at: n,
        }
    }
}

impl StopSignal for Polls {
    fn stopped(&self) -> bool {
        self.seen.fetch_add(1, Ordering::Relaxed) + 1 >= self.stop_at
    }
}

/// Every canonical entry that builds the document, and the digest entries over it.
#[derive(Debug, Clone, Copy)]
enum Entry {
    OverlayDocument,
    FlatDocument,
    FlatSha256,
    OverlaySha256,
    FlatBlake3,
    OverlayBlake3,
    OverlayDocumentAndSha256,
    GraphSha256,
}

const ALL_ENTRIES: [Entry; 8] = [
    Entry::OverlayDocument,
    Entry::FlatDocument,
    Entry::FlatSha256,
    Entry::OverlaySha256,
    Entry::FlatBlake3,
    Entry::OverlayBlake3,
    Entry::OverlayDocumentAndSha256,
    Entry::GraphSha256,
];

impl Entry {
    /// Entries that return only a digest, so build no document.
    const fn digest_only(self) -> bool {
        !matches!(
            self,
            Self::OverlayDocument | Self::FlatDocument | Self::OverlayDocumentAndSha256
        )
    }
}

/// The run's output identity: a digest of the document, or the digest itself.
fn run(
    entry: Entry,
    view: &Recording,
    control: &mut CanonicalizationControl<'_>,
) -> Result<String, String> {
    let hash = CanonHash::Sha256;
    let sha = |digest: ContentDigest| digest.to_hex();
    match entry {
        Entry::OverlayDocument => try_canonicalize_view(view, hash, control)
            .map(|d| sha(ContentDigest::of(d.nquads().as_bytes())))
            .map_err(|e| format!("{e:?}")),
        Entry::FlatDocument => try_canonicalize_flat_view(view, hash, control)
            .map(|d| sha(ContentDigest::of(d.nquads().as_bytes())))
            .map_err(|e| format!("{e:?}")),
        Entry::FlatSha256 => try_flat_digest_view(view, hash, control)
            .map(sha)
            .map_err(|e| format!("{e:?}")),
        Entry::OverlaySha256 => try_canonical_sha256_view(view, hash, control)
            .map(sha)
            .map_err(|e| format!("{e:?}")),
        Entry::FlatBlake3 => try_flat_blake3_view(view, hash, control)
            .map(|d| purrdf_hash::hex::encode(&d))
            .map_err(|e| format!("{e:?}")),
        Entry::OverlayBlake3 => try_canonical_blake3_view(view, hash, control)
            .map(|d| purrdf_hash::hex::encode(&d))
            .map_err(|e| format!("{e:?}")),
        Entry::OverlayDocumentAndSha256 => try_canonicalize_sha256_view(view, hash, control)
            .map(|(document, digest)| {
                assert_eq!(digest, ContentDigest::of(document.nquads().as_bytes()));
                sha(digest)
            })
            .map_err(|e| format!("{e:?}")),
        Entry::GraphSha256 => try_graph_digest_view(view, GRAPH, control)
            .map(sha)
            .map_err(|e| format!("{e:?}")),
    }
}

struct Measured {
    digest: String,
    total_polls: usize,
    first_output_poll: usize,
    scratch_max: u64,
    owned_max: u64,
}

fn measure(entry: Entry, dataset: &Arc<RdfDataset>) -> Measured {
    let view = Recording::new(dataset, u64::MAX, u64::MAX);
    let polls = Polls::counting();
    let mut control =
        CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
    let digest = run(entry, &view, &mut control).expect("an unconstrained run succeeds");
    let total_polls = polls.seen.load(Ordering::Relaxed);
    // The phase boundary: the smallest stop whose receipt says `Output`.
    let (mut low, mut high) = (1, total_polls);
    while low < high {
        let mid = low + (high - low) / 2;
        let polls = Polls::stopping_at(mid);
        let mut control =
            CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
        let probe = Recording::new(dataset, u64::MAX, u64::MAX);
        let stopped = run(entry, &probe, &mut control);
        assert!(
            stopped.is_err(),
            "a stop at poll {mid} of {total_polls} must refuse"
        );
        if control.receipts()[0].phase == CanonPhase::Output {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Measured {
        digest,
        total_polls,
        first_output_poll: low,
        scratch_max: view.scratch.max(),
        owned_max: view.owned.max(),
    }
}

/// One entry over one fixture, as the previous document construction (one owned
/// line per component, then a whole-document copy) measured it.
struct Frozen {
    fixture: &'static str,
    entry: Entry,
    /// Every poll of the caller's stop signal, start to finish.
    polls: usize,
    /// The smallest stop that the run's receipt places in the output phase.
    first_output: usize,
    /// The largest reservation the scratch account asked of its provider.
    scratch: u64,
    /// The largest reservation the owned-result account asked of its provider.
    owned: u64,
    /// SHA-256 of the canonical document (or the digest the entry returns).
    digest: &'static str,
    /// What the same entry asked of the scratch account when it built the document.
    before_scratch: u64,
    /// What the same entry asked of the owned account when it built the document.
    before_owned: u64,
}

const FROZEN: &[Frozen] = &[
    Frozen {
        fixture: "blank_free",
        entry: Entry::OverlayDocument,
        polls: 1092,
        first_output: 547,
        scratch: 11364,
        owned: 6044,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
        before_scratch: 11364,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::FlatDocument,
        polls: 1092,
        first_output: 547,
        scratch: 11364,
        owned: 6044,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
        before_scratch: 11364,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::FlatSha256,
        polls: 1092,
        first_output: 547,
        scratch: 8742,
        owned: 0,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
        before_scratch: 8742,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::OverlaySha256,
        polls: 1092,
        first_output: 547,
        scratch: 8742,
        owned: 0,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
        before_scratch: 8742,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::FlatBlake3,
        polls: 1092,
        first_output: 547,
        scratch: 8742,
        owned: 0,
        digest: "7e8a5d1e3084a3037568be98a21d0eda48a9c55aa4a7632fe563f196380f8d33",
        before_scratch: 8742,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::OverlayBlake3,
        polls: 1092,
        first_output: 547,
        scratch: 8742,
        owned: 0,
        digest: "7e8a5d1e3084a3037568be98a21d0eda48a9c55aa4a7632fe563f196380f8d33",
        before_scratch: 8742,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::OverlayDocumentAndSha256,
        polls: 1094,
        first_output: 547,
        scratch: 8742,
        owned: 6044,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
        before_scratch: 8742,
        before_owned: 6044,
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::GraphSha256,
        polls: 173,
        first_output: 88,
        scratch: 1846,
        owned: 0,
        digest: "6a35ad0beede68e7bef06e8fe10ec0695b772d6573510147afd85d2221dd9f1a",
        before_scratch: 1846,
        before_owned: 964,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::OverlayDocument,
        polls: 1804,
        first_output: 1183,
        scratch: 10112,
        owned: 6068,
        digest: "6de62d050e54492c976ae2a54621f77469ff3d960cd73a8950da52c9515ff399",
        before_scratch: 10112,
        before_owned: 6068,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::FlatDocument,
        polls: 1776,
        first_output: 1183,
        scratch: 7600,
        owned: 5228,
        digest: "af5cdb72d4b9ee9995853807bdcf6d74eabed2a7e2bc50781cd44eae165c4b5f",
        before_scratch: 7600,
        before_owned: 5228,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::FlatSha256,
        polls: 1776,
        first_output: 1183,
        scratch: 7094,
        owned: 0,
        digest: "af5cdb72d4b9ee9995853807bdcf6d74eabed2a7e2bc50781cd44eae165c4b5f",
        before_scratch: 7094,
        before_owned: 5228,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::OverlaySha256,
        polls: 1804,
        first_output: 1183,
        scratch: 6962,
        owned: 0,
        digest: "6de62d050e54492c976ae2a54621f77469ff3d960cd73a8950da52c9515ff399",
        before_scratch: 6962,
        before_owned: 6068,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::FlatBlake3,
        polls: 1776,
        first_output: 1183,
        scratch: 7094,
        owned: 0,
        digest: "53b31cae2c52621681e220eb75f4f9d1f3f1206c09cd200565aa966fb8c0a7e8",
        before_scratch: 7094,
        before_owned: 5228,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::OverlayBlake3,
        polls: 1804,
        first_output: 1183,
        scratch: 6962,
        owned: 0,
        digest: "1e4b1695a8742d61aaf49c04752e6414815bd008dd41ccc33a9e21259f9a0fc6",
        before_scratch: 6962,
        before_owned: 6068,
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::OverlayDocumentAndSha256,
        polls: 1806,
        first_output: 1183,
        scratch: 6962,
        owned: 6068,
        digest: "6de62d050e54492c976ae2a54621f77469ff3d960cd73a8950da52c9515ff399",
        before_scratch: 6962,
        before_owned: 6068,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::OverlayDocument,
        polls: 986,
        first_output: 750,
        scratch: 2848,
        owned: 1917,
        digest: "fe631726a55ce26676ac35d64a600f6a9603f52cd5a7255625da5f714b6a2f7b",
        before_scratch: 2848,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::FlatDocument,
        polls: 986,
        first_output: 750,
        scratch: 2848,
        owned: 1917,
        digest: "fe631726a55ce26676ac35d64a600f6a9603f52cd5a7255625da5f714b6a2f7b",
        before_scratch: 2848,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::FlatSha256,
        polls: 986,
        first_output: 750,
        scratch: 2514,
        owned: 0,
        digest: "fe631726a55ce26676ac35d64a600f6a9603f52cd5a7255625da5f714b6a2f7b",
        before_scratch: 2514,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::OverlaySha256,
        polls: 986,
        first_output: 750,
        scratch: 2514,
        owned: 0,
        digest: "fe631726a55ce26676ac35d64a600f6a9603f52cd5a7255625da5f714b6a2f7b",
        before_scratch: 2514,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::FlatBlake3,
        polls: 986,
        first_output: 750,
        scratch: 2514,
        owned: 0,
        digest: "75a612f366688be11eaad6977a483e2fcbcd9eadc4427f1d1dcbfec626e82d5c",
        before_scratch: 2514,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::OverlayBlake3,
        polls: 986,
        first_output: 750,
        scratch: 2514,
        owned: 0,
        digest: "75a612f366688be11eaad6977a483e2fcbcd9eadc4427f1d1dcbfec626e82d5c",
        before_scratch: 2514,
        before_owned: 1917,
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::OverlayDocumentAndSha256,
        polls: 988,
        first_output: 750,
        scratch: 2514,
        owned: 1917,
        digest: "fe631726a55ce26676ac35d64a600f6a9603f52cd5a7255625da5f714b6a2f7b",
        before_scratch: 2514,
        before_owned: 1917,
    },
];

fn dataset_of(fixture: &str) -> Arc<RdfDataset> {
    fixtures()
        .into_iter()
        .find_map(|(name, dataset)| (name == fixture).then_some(dataset))
        .expect("a frozen row names a fixture")
}

/// A caller meters canonicalization through its stop signal, so the number of
/// polls and where the output phase begins are part of what a run promises.
/// Document entries keep the checkpoints the caller already budgets for; digest
/// entries hash lines as they are produced and build no document, so their
/// totals are two lower than when the document was built first, with the phase
/// boundary unmoved. The digest is the stored content identity and must not move
/// for any entry.
#[test]
fn stop_checkpoints_and_identity_match_the_frozen_construction() {
    for row in FROZEN {
        let measured = measure(row.entry, &dataset_of(row.fixture));
        let at = format!("{} {:?}", row.fixture, row.entry);
        assert_eq!(measured.total_polls, row.polls, "poll total of {at}");
        assert_eq!(
            measured.first_output_poll, row.first_output,
            "output boundary of {at}"
        );
        assert_eq!(measured.digest, row.digest, "identity of {at}");
    }
}

/// A bounded provider admits a run against a ceiling it chose from earlier
/// observations. Construction may ask for less than before, never more, so a run
/// whose ceiling is exactly the frozen maximum of each account still succeeds,
/// with the same identity, and no account ever asks above its frozen maximum.
#[test]
fn provider_reservations_never_exceed_the_frozen_maxima() {
    for row in FROZEN {
        let dataset = dataset_of(row.fixture);
        let at = format!("{} {:?}", row.fixture, row.entry);
        let measured = measure(row.entry, &dataset);
        assert!(
            measured.scratch_max <= row.scratch,
            "scratch maximum of {at}"
        );
        assert!(measured.owned_max <= row.owned, "owned maximum of {at}");
        let capped = Recording::new(&dataset, row.scratch, row.owned);
        let mut control = CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
        let digest = run(row.entry, &capped, &mut control)
            .unwrap_or_else(|error| panic!("{at} refused at the frozen ceiling: {error}"));
        assert_eq!(digest, row.digest, "identity of {at} at the ceiling");
    }
}

/// A stop that lands on any of the last polls of a digest run, the combined
/// document-and-digest run included, refuses in the output phase with one
/// receipt that says the run did not complete. A caller that cancels a long
/// digest must be told the output, not the search, was interrupted.
#[test]
fn a_stop_on_the_last_polls_of_a_digest_run_ends_in_the_output_phase() {
    for (fixture, dataset) in fixtures() {
        for entry in ALL_ENTRIES
            .into_iter()
            .filter(|e| e.digest_only() || matches!(e, Entry::OverlayDocumentAndSha256))
            // The graph entry names a graph only the first fixture holds; an absent graph
            // is the unchanged empty-document shortcut, which stops before output.
            .filter(|e| !matches!(e, Entry::GraphSha256) || fixture == "blank_free")
        {
            let counted = measure(entry, &dataset);
            for n in [
                counted.total_polls - 2,
                counted.total_polls - 1,
                counted.total_polls,
            ] {
                let polls = Polls::stopping_at(n);
                let mut control = CanonicalizationControl::new(
                    RDFC_V2_CORPUS_POLICY,
                    Some(&polls as &dyn StopSignal),
                );
                let view = Recording::new(&dataset, u64::MAX, u64::MAX);
                let at = format!(
                    "{fixture} {entry:?} stopped at {n} of {}",
                    counted.total_polls
                );
                assert!(run(entry, &view, &mut control).is_err(), "{at} refuses");
                let receipts = control.receipts();
                assert_eq!(receipts.len(), 1, "one receipt for {at}");
                assert_eq!(receipts[0].phase, CanonPhase::Output, "phase of {at}");
                assert!(!receipts[0].completed, "{at} did not complete");
            }
        }
    }
}

/// A digest-only run stops charging the provider for a document it does not
/// build. Against what the same entry asked when it built the document, neither
/// account asks for more, the owned account asks for strictly less, and a run
/// still succeeds with both accounts capped at the earlier maxima.
#[test]
fn a_digest_run_asks_its_provider_for_no_more_than_the_document_run_did() {
    for row in FROZEN.iter().filter(|row| row.entry.digest_only()) {
        let at = format!("{} {:?}", row.fixture, row.entry);
        let dataset = dataset_of(row.fixture);
        let measured = measure(row.entry, &dataset);
        assert!(
            measured.scratch_max <= row.before_scratch,
            "scratch of {at}"
        );
        assert!(measured.owned_max <= row.before_owned, "owned of {at}");
        assert!(
            measured.owned_max < row.before_owned || row.before_owned == 0,
            "the document charge of {at} is gone"
        );
        let capped = Recording::new(&dataset, row.before_scratch, row.before_owned);
        let mut control = CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
        let digest = run(row.entry, &capped, &mut control)
            .unwrap_or_else(|error| panic!("{at} refused at the earlier ceiling: {error}"));
        assert_eq!(
            digest, row.digest,
            "identity of {at} at the earlier ceiling"
        );
    }
}

/// A view with no owned workspace is refused by a digest run exactly as by a
/// document run, and the neighbouring view that has one succeeds.
#[test]
fn a_digest_run_without_an_owned_workspace_is_refused_like_a_document_run() {
    let dataset = blank_free();
    for entry in ALL_ENTRIES {
        let mut control = CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
        let without = Recording::without_owned(&dataset);
        let refused = run(entry, &without, &mut control).expect_err("refused");
        assert!(
            refused.contains("OwnedWorkspaceUnavailable"),
            "{entry:?}: {refused}"
        );
        let mut control = CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None);
        let with = Recording::new(&dataset, u64::MAX, u64::MAX);
        run(entry, &with, &mut control).expect("the neighbouring view succeeds");
    }
}

/// One line several blocks long: a stop raised on any poll of a digest run, the
/// polls between the blocks of that line included, refuses, and a stop after the
/// last poll does not. A digest is never returned from a run whose stop fired.
#[test]
fn a_stop_during_a_line_of_several_blocks_is_honoured_by_every_digest_entry() {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri("https://example.org/s").unwrap();
    let p = b.intern_iri("https://example.org/p").unwrap();
    let small = b.intern_iri("https://example.org/o").unwrap();
    let long = b
        .intern_literal(RdfLiteral::simple("x".repeat(5 * 64 * 1024 + 17)))
        .unwrap();
    b.push_quad(s, p, small, None).unwrap();
    b.push_quad(s, p, long, None).unwrap();
    let dataset = b.freeze().unwrap();
    for entry in ALL_ENTRIES
        .into_iter()
        .filter(|e| e.digest_only() || matches!(e, Entry::OverlayDocumentAndSha256))
        .filter(|e| !matches!(e, Entry::GraphSha256))
    {
        let counted = measure(entry, &dataset);
        for n in 1..=counted.total_polls {
            let polls = Polls::stopping_at(n);
            let mut control = CanonicalizationControl::new(
                RDFC_V2_CORPUS_POLICY,
                Some(&polls as &dyn StopSignal),
            );
            let view = Recording::new(&dataset, u64::MAX, u64::MAX);
            assert!(
                run(entry, &view, &mut control).is_err(),
                "{entry:?} stopped at poll {n} of {} returned a digest",
                counted.total_polls
            );
        }
        let polls = Polls::stopping_at(counted.total_polls + 1);
        let mut control =
            CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
        let view = Recording::new(&dataset, u64::MAX, u64::MAX);
        assert_eq!(run(entry, &view, &mut control).unwrap(), counted.digest);
    }
}

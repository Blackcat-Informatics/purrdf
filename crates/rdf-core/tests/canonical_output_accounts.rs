// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical document path's stop checkpoints and provider reservations.
//!
//! A canonical document is a stored content identity, and a caller meters it
//! through two things this file pins from the public surface: how often the
//! caller's stop signal is polled, and the largest reservation each admission
//! account ever asks its provider for. Every row is frozen against what the
//! canonical labelling engine produced: the poll count and the phase boundary
//! must not move, and a reservation may only shrink.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use purrdf_hash::hex::Digest32;

use purrdf_core::{
    BlankScope, CANON_CORPUS_POLICY, CanonPhase, CanonicalizationControl, ContentDigest,
    DatasetView, FallibleDatasetView, GraphMatch, OwnedWorkspaceReservation, QuadIds, RdfDataset,
    RdfDatasetBuilder, RdfLiteral, RdfStoreCapabilities, RdfTextDirection, StopSignal, TermId,
    TermRef, TermValue, ViewOperationStatus, WorkspaceReservation,
    try_canonical_digest_checked_view, try_canonical_digest_view, try_canonicalize_checked_view,
    try_canonicalize_digest_view, try_canonicalize_view, try_graph_digest_view,
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
    Document,
    CheckedDocument,
    Digest,
    CheckedDigest,
    DocumentAndDigest,
    Graph,
}

const ALL_ENTRIES: [Entry; 6] = [
    Entry::Document,
    Entry::CheckedDocument,
    Entry::Digest,
    Entry::CheckedDigest,
    Entry::DocumentAndDigest,
    Entry::Graph,
];

impl Entry {
    /// Entries that return only a digest, so build no document.
    const fn digest_only(self) -> bool {
        matches!(self, Self::Digest | Self::CheckedDigest | Self::Graph)
    }
}

/// The run's output identity: a digest of the document text, or the digest the
/// entry returns.
fn run(
    entry: Entry,
    view: &Recording,
    control: &mut CanonicalizationControl<'_>,
) -> Result<String, String> {
    let hex = |digest: Digest32| purrdf_hash::hex::encode(digest.as_bytes());
    let text = |nquads: &str| ContentDigest::of(nquads.as_bytes()).to_hex();
    match entry {
        Entry::Document => try_canonicalize_view(view, control)
            .map(|d| text(d.nquads()))
            .map_err(|e| format!("{e:?}")),
        Entry::CheckedDocument => try_canonicalize_checked_view(view, control)
            .map(|d| text(d.nquads()))
            .map_err(|e| format!("{e:?}")),
        Entry::Digest => try_canonical_digest_view(view, control)
            .map(hex)
            .map_err(|e| format!("{e:?}")),
        Entry::CheckedDigest => try_canonical_digest_checked_view(view, control)
            .map(hex)
            .map_err(|e| format!("{e:?}")),
        Entry::DocumentAndDigest => try_canonicalize_digest_view(view, control)
            .map(|(_, digest)| hex(digest))
            .map_err(|e| format!("{e:?}")),
        Entry::Graph => try_graph_digest_view(view, GRAPH, control)
            .map(hex)
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
        CanonicalizationControl::new(CANON_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
    let digest = run(entry, &view, &mut control).expect("an unconstrained run succeeds");
    let total_polls = polls.seen.load(Ordering::Relaxed);
    // The phase boundary: the smallest stop whose receipt says `Output`.
    let (mut low, mut high) = (1, total_polls);
    while low < high {
        let mid = low + (high - low) / 2;
        let polls = Polls::stopping_at(mid);
        let mut control =
            CanonicalizationControl::new(CANON_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
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

/// One entry over one fixture, as the labelling engine measured it.
struct Frozen {
    fixture: &'static str,
    entry: Entry,
    /// Every poll of the caller's stop signal, start to finish.
    polls: usize,
    /// The smallest stop that the run's receipt places in the output phase;
    /// the poll total for an entry that never reaches it.
    first_output: usize,
    /// The largest reservation the scratch account asked of its provider.
    scratch: u64,
    /// The largest reservation the owned-result account asked of its provider.
    owned: u64,
    /// SHA-256 of the canonical document (or the digest the entry returns).
    digest: &'static str,
}

const FROZEN: &[Frozen] = &[
    Frozen {
        fixture: "blank_free",
        entry: Entry::Document,
        polls: 183,
        first_output: 63,
        scratch: 67416,
        owned: 6044,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::CheckedDocument,
        polls: 183,
        first_output: 63,
        scratch: 67416,
        owned: 6044,
        digest: "ffa5774c571d3645ac144ea04f63f63c73b6f5121fc23e5e204f60d772834c25",
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::Digest,
        polls: 62,
        first_output: 62,
        scratch: 67416,
        owned: 0,
        digest: "88a6fd566ab8882bc31c618498d1cd180ad8a813c4f4585ea5717ebd3cafed53",
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::CheckedDigest,
        polls: 62,
        first_output: 62,
        scratch: 67416,
        owned: 0,
        digest: "88a6fd566ab8882bc31c618498d1cd180ad8a813c4f4585ea5717ebd3cafed53",
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::DocumentAndDigest,
        polls: 183,
        first_output: 63,
        scratch: 67416,
        owned: 6044,
        digest: "88a6fd566ab8882bc31c618498d1cd180ad8a813c4f4585ea5717ebd3cafed53",
    },
    Frozen {
        fixture: "blank_free",
        entry: Entry::Graph,
        polls: 12,
        first_output: 12,
        scratch: 10396,
        owned: 0,
        digest: "763ec20a96475f19a075fcca8d06f0a0815a73d96ebd5513f1302028672928f1",
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::Document,
        polls: 227,
        first_output: 115,
        scratch: 107_278,
        owned: 4948,
        digest: "cd4bdc4e32befd000e1bc050486e7d8779799483c6749635f91f5d7cb2dd6a26",
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::CheckedDocument,
        polls: 227,
        first_output: 115,
        scratch: 107_278,
        owned: 4948,
        digest: "cd4bdc4e32befd000e1bc050486e7d8779799483c6749635f91f5d7cb2dd6a26",
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::Digest,
        polls: 114,
        first_output: 114,
        scratch: 107_278,
        owned: 0,
        digest: "442726b364df93606a2ca4587dddb76bded5a657f76b40b3c0b7ed92f61cbeee",
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::CheckedDigest,
        polls: 114,
        first_output: 114,
        scratch: 107_278,
        owned: 0,
        digest: "442726b364df93606a2ca4587dddb76bded5a657f76b40b3c0b7ed92f61cbeee",
    },
    Frozen {
        fixture: "reifier",
        entry: Entry::DocumentAndDigest,
        polls: 227,
        first_output: 115,
        scratch: 107_278,
        owned: 4948,
        digest: "442726b364df93606a2ca4587dddb76bded5a657f76b40b3c0b7ed92f61cbeee",
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::Document,
        polls: 75,
        first_output: 25,
        scratch: 34503,
        owned: 1710,
        digest: "b0b05309017ce5de3b29e77be310a44553a503939720516c5444759223c26a7b",
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::CheckedDocument,
        polls: 75,
        first_output: 25,
        scratch: 34503,
        owned: 1710,
        digest: "b0b05309017ce5de3b29e77be310a44553a503939720516c5444759223c26a7b",
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::Digest,
        polls: 24,
        first_output: 24,
        scratch: 34503,
        owned: 0,
        digest: "e6eff0ca06b95ea978482c13fda1daff23c1ca0405d61fd14da5f23b7cad09b2",
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::CheckedDigest,
        polls: 24,
        first_output: 24,
        scratch: 34503,
        owned: 0,
        digest: "e6eff0ca06b95ea978482c13fda1daff23c1ca0405d61fd14da5f23b7cad09b2",
    },
    Frozen {
        fixture: "mixed",
        entry: Entry::DocumentAndDigest,
        polls: 75,
        first_output: 25,
        scratch: 34503,
        owned: 1710,
        digest: "e6eff0ca06b95ea978482c13fda1daff23c1ca0405d61fd14da5f23b7cad09b2",
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
/// Digest entries build no document, so the output phase is their last poll;
/// document entries poll once per line of the document after it begins. The
/// digest is the stored content identity and must not move for any entry.
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
        let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
        let digest = run(row.entry, &capped, &mut control)
            .unwrap_or_else(|error| panic!("{at} refused at the frozen ceiling: {error}"));
        assert_eq!(digest, row.digest, "identity of {at} at the ceiling");
    }
}

/// A stop that lands on any of the last polls of a run refuses with one
/// receipt that says the run did not complete. A run that builds the document
/// reports the output phase; a digest run ends inside the labelling, which
/// reports an earlier phase, and never the output.
#[test]
fn a_stop_on_the_last_polls_of_a_run_refuses_with_an_incomplete_receipt() {
    for (fixture, dataset) in fixtures() {
        for entry in ALL_ENTRIES
            .into_iter()
            // The graph entry names a graph only the first fixture holds.
            .filter(|e| !matches!(e, Entry::Graph) || fixture == "blank_free")
        {
            let counted = measure(entry, &dataset);
            for n in counted.total_polls - 2..=counted.total_polls {
                let polls = Polls::stopping_at(n);
                let mut control = CanonicalizationControl::new(
                    CANON_CORPUS_POLICY,
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
                assert_eq!(
                    receipts[0].phase == CanonPhase::Output,
                    !entry.digest_only(),
                    "phase of {at}"
                );
                assert!(!receipts[0].completed, "{at} did not complete");
            }
        }
    }
}

/// A digest-only run does not charge the provider for a document it does not
/// build. Measured against the document entry over the same fixture, neither
/// account asks for more, and a run still succeeds with both accounts capped
/// at the document run's maxima.
#[test]
fn a_digest_run_asks_its_provider_for_no_more_than_the_document_run() {
    for (fixture, dataset) in fixtures() {
        let document = measure(Entry::Document, &dataset);
        for entry in [Entry::Digest, Entry::CheckedDigest] {
            let at = format!("{fixture} {entry:?}");
            let measured = measure(entry, &dataset);
            assert!(
                measured.scratch_max <= document.scratch_max,
                "scratch of {at}"
            );
            assert!(measured.owned_max <= document.owned_max, "owned of {at}");
            let capped = Recording::new(&dataset, document.scratch_max, document.owned_max);
            let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
            let digest = run(entry, &capped, &mut control)
                .unwrap_or_else(|error| panic!("{at} refused at the document ceiling: {error}"));
            assert_eq!(digest, measured.digest, "identity of {at} at the ceiling");
        }
    }
}

/// A document run needs an owned workspace and is refused without one; a
/// digest run builds no document, asks for none and succeeds on the same view.
/// The neighbouring view that has an owned workspace succeeds for every entry.
#[test]
fn only_a_document_run_needs_an_owned_workspace() {
    let dataset = blank_free();
    for entry in ALL_ENTRIES {
        let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
        let without = Recording::without_owned(&dataset);
        let outcome = run(entry, &without, &mut control);
        if entry.digest_only() {
            outcome.expect("a digest run needs no owned workspace");
        } else {
            let refused = outcome.expect_err("refused");
            assert!(
                refused.contains("OwnedWorkspaceUnavailable"),
                "{entry:?}: {refused}"
            );
        }
        let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
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
        .filter(|e| e.digest_only() || matches!(e, Entry::DocumentAndDigest))
        .filter(|e| !matches!(e, Entry::Graph))
    {
        let counted = measure(entry, &dataset);
        for n in 1..=counted.total_polls {
            let polls = Polls::stopping_at(n);
            let mut control =
                CanonicalizationControl::new(CANON_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
            let view = Recording::new(&dataset, u64::MAX, u64::MAX);
            assert!(
                run(entry, &view, &mut control).is_err(),
                "{entry:?} stopped at poll {n} of {} returned a digest",
                counted.total_polls
            );
        }
        let polls = Polls::stopping_at(counted.total_polls + 1);
        let mut control =
            CanonicalizationControl::new(CANON_CORPUS_POLICY, Some(&polls as &dyn StopSignal));
        let view = Recording::new(&dataset, u64::MAX, u64::MAX);
        assert_eq!(run(entry, &view, &mut control).unwrap(), counted.digest);
    }
}

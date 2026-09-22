// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The same fusion, executed on x86-64 and on `wasm32-unknown-unknown`,
//! compared against the same hand-computed decimals.**
//!
//! Every other test in this crate's suite proves something weaker: that fusion
//! is a pure function *of one target*. Run it fifty times on this machine and it
//! agrees with itself — which cannot distinguish a fusion that is
//! target-independent from one that merely happens to be self-consistent
//! wherever it was last compiled. `make wasm` does not close that gap either: it
//! proves the release crates **build** for wasm32, not that they **answer** the
//! same way there.
//!
//! # What is actually at risk
//!
//! A fused score is a sum of `weight * recip(K + rank)` terms, and the whole
//! ordering claim rests on those terms being the same value everywhere. If the
//! reciprocal were a `f64` division, a reassociated sum or a fused multiply-add
//! would move a last bit, two near-tied candidates would swap, and the browser
//! would return a different *answer* than the host — an answer divergence, not a
//! rounding detail, and one nothing downstream could detect. The reciprocal is
//! therefore a single integer division over `i128` fixed point, truncating
//! toward zero, with no floating-point value anywhere in the path.
//!
//! That is an argument. This file is where it becomes an executed test.
//!
//! # How it runs on both
//!
//! One test body, two attributes. Natively each is an ordinary `#[test]` picked
//! up by `cargo test`. On `wasm32-unknown-unknown` each is a
//! `#[wasm_bindgen_test]`, compiled to wasm and executed in Node by `make
//! wasm-test` (and by CI's wasm job):
//!
//! ```text
//! cargo test -p purrdf-retrieval --target wasm32-unknown-unknown --test wasm_determinism
//! ```
//!
//! # Why the expectations are what they are
//!
//! The scores are **hand computed, not recorded** — a test that fills its
//! expectation from the kernel it is testing agrees with any kernel at all,
//! including a wrong one. At `SCALE_DIGITS` twelve and a weight of exactly one,
//! a contribution at rank `r` under `K = 60` is `trunc(10^12 / (60 + r))`:
//!
//! ```text
//! rank 1 -> 10^12 / 61 = 16393442622 remainder 58 -> 0.016393442622
//! rank 2 -> 10^12 / 62 = 16129032258 remainder  4 -> 0.016129032258
//! rank 3 -> 10^12 / 63 = 15873015873 remainder  1 -> 0.015873015873
//! ```
//!
//! The two strata rank the same three candidates in different orders, so each
//! candidate's fused score is the sum of two *different* ranks' contributions
//! and no two scores are equal. Every expected sum below is those integers added
//! by hand.
//!
//! The recorded expectations are the two BLAKE3 digests — the fusion profile's
//! identity and an answer's evidence identity — because there is no hand
//! arithmetic that produces one. Pinning them is still the cross-target claim
//! this file exists to make: each digest is taken over a canonical,
//! length-framed encoding, so a target that framed an integer or ordered a map
//! differently would produce a different hex here. The evidence digest is also
//! pinned the stronger way, by writing out the **bytes** it is a digest of, which
//! is a claim about the layout that no digest alone can make.
//!
//! The comparison is on the **decimal lexical**, not on the raw `i128`, because
//! the lexical is what a consumer receives and what a serializer writes.
//!
//! # What else crosses the target boundary
//!
//! A fused score is not the only thing an answer carries, so two further facts
//! about the same fusion are pinned here rather than on one target only:
//!
//! * **the evidence identity**, which is what two holders of two answers compare
//!   to decide whether they were served from the same indexes — hand-written
//!   bytes, then their digest, then the exactness verdict a declared shortfall
//!   produces;
//! * **the depth a declared candidate domain licenses**, because a declaration
//!   that two strata draw from disjoint blocks lets a candidate certify before
//!   every stream has been consulted. That makes the number of ranks read part of
//!   the answer's contract: a stream stopped short reports a ceiling rather than
//!   exhaustion, so a target whose finality test read one rank further would hand
//!   back a different terminal report for identical streams.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::task::{Context, Poll, Waker};

use purrdf_retrieval::{
    CandidateDomains, DecayRule, DomainTag, DuplicatePolicy, EVIDENCE_VERSION, EvidenceId,
    ExclusionBasis, ExclusionVerdict, Fixed, FusionProfile, IndexGeneration, Iri, PfAttestation,
    ProducerReceipt, ProducerStatus, ProtocolError, RankFidelity, RankedRow, RankedStream,
    RowBlock, ScoreExactness, ServiceLevel, StreamContract, Term, TopK, contribution, fuse,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

/// The reciprocal-rank smoothing constant the fixture profile fixes.
const K: u32 = 60;

/// The first stratum, ranking `alpha`, `beta`, `gamma` in that order.
const STRATUM_ONE: &str = "http://example.org/stratum/wasm1";

/// The second stratum, ranking `beta`, `gamma`, `alpha` in that order.
const STRATUM_TWO: &str = "http://example.org/stratum/wasm2";

/// `trunc(10^12 / 61)`, the contribution of a unit weight at rank 1.
const RANK_1: i128 = 16_393_442_622;

/// `trunc(10^12 / 62)`, the contribution of a unit weight at rank 2.
const RANK_2: i128 = 16_129_032_258;

/// `trunc(10^12 / 63)`, the contribution of a unit weight at rank 3.
const RANK_3: i128 = 15_873_015_873;

/// The fusion profile's identity, as hex.
///
/// Recorded rather than derived: it is a BLAKE3 digest over the profile's
/// canonical encoding, and no hand arithmetic produces one. It is pinned so that
/// a target which framed an integer or ordered the weight map differently would
/// fail here rather than silently name a different law.
const PROFILE_ID_HEX: &str = "9b34ccfc3361417c90cbc70a6fc2685ef8c73b5270255b706e6a84c551720290";

// ---------------------------------------------------------------------------
// A single-threaded executor. No thread parking: `wasm32-unknown-unknown` has
// no threads, and the scripted streams below never return `Poll::Pending`.
// ---------------------------------------------------------------------------

fn block_on<F: Future>(future: F) -> F::Output {
    // `Waker::noop()` needs no thread and no allocation, so this drives a
    // future on any target, `wasm32-unknown-unknown` included.
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the scripted streams are synchronous and never pend"),
    }
}

// ---------------------------------------------------------------------------
// The fixture
// ---------------------------------------------------------------------------

fn iri(text: &str) -> Iri {
    Iri::parse(text).expect("the fixture IRIs are valid")
}

/// A producer whose rows are pre-scripted, in rank order.
struct ScriptedStream {
    steps: VecDeque<RankedRow<Term>>,
    emitted: u64,
    /// What this stream promises about its rows. Part of the determinism claim:
    /// the contract decides what the engine holds and refuses, so a target that
    /// read it differently would fuse differently.
    contract: StreamContract,
    /// What this stream attests about the index behind its rows. Most fixtures
    /// here descend from no index and say so; the evidence fixture states its
    /// own, because an evidence identity is a digest over exactly these values.
    attestation: PfAttestation,
}

// The trait's methods are `async`; this mock's body is synchronous because its
// rows are pre-scripted.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<RankedRow<Self::Item>>, ProtocolError> {
        match self.steps.pop_front() {
            Some(step) => {
                self.emitted += 1;
                Ok(Some(step))
            }
            None => Ok(None),
        }
    }

    async fn receipt(&mut self) -> Result<ProducerReceipt, ProtocolError> {
        Ok(ProducerReceipt::Exhausted {
            rows_emitted: self.emitted,
        })
    }

    fn contract(&self) -> StreamContract {
        self.contract.clone()
    }

    /// This stream declares no exclusion basis, so being asked for a verdict is
    /// the disagreement [`ProtocolError::ExclusionUnavailable`] names rather
    /// than a question it could answer. Fusion never asks it; a hand-written
    /// caller that did would be told so.
    async fn exclusion(&mut self, _candidate: &Term) -> Result<ExclusionVerdict, ProtocolError> {
        Err(ProtocolError::ExclusionUnavailable)
    }

    fn attestation(&self) -> PfAttestation {
        self.attestation.clone()
    }
}

/// The contract a fixture stream of distinct, strictly rank-ordered rows honestly
/// declares when it promises nothing about where its candidates lie.
fn unrestricted() -> StreamContract {
    StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
}

/// Script one stream's three rows, in the order `candidates` names them.
fn scripted(candidates: [&str; 3]) -> ScriptedStream {
    let steps = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let rank = u64::try_from(index + 1).expect("three rows");
            // The fixture declares `Unrestricted`, which owes no per-row block
            // and names none. That is part of the determinism claim too: an
            // absence must read identically on every target.
            RankedRow::new(
                rank,
                contribution(Fixed::ONE, rank, K).expect("the contribution fits"),
                Term::new(*candidate),
                RowBlock::Undeclared,
            )
        })
        .collect();
    ScriptedStream {
        steps,
        emitted: 0,
        contract: unrestricted(),
        attestation: PfAttestation::UNDECLARED,
    }
}

/// Both strata at unit weight and `K = 60`, which admits two contributions per
/// candidate — one per stratum, derived from the two weights.
fn profile() -> FusionProfile {
    FusionProfile::with_decay(
        BTreeMap::from([
            (iri(STRATUM_ONE), Fixed::ONE),
            (iri(STRATUM_TWO), Fixed::ONE),
        ]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("the fixture profile is valid")
}

/// The decimal lexical of a raw fixed-point integer, for an expectation written
/// as the sum of two hand-computed contributions.
fn decimal(raw: i128) -> String {
    Fixed::from_raw(raw).to_decimal_lexical()
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// The contributions themselves, before any fusion: one integer division each,
/// truncating toward zero, at exactly the values computed in this file's header.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_unit_weight_contribution_is_the_same_exact_decimal_on_both_targets() {
    for (rank, expected) in [(1_u64, RANK_1), (2, RANK_2), (3, RANK_3)] {
        let value = contribution(Fixed::ONE, rank, K).expect("the contribution fits");
        assert_eq!(
            value.to_decimal_lexical(),
            decimal(expected),
            "the contribution at rank {rank}"
        );
    }
}

/// The fused answer: three candidates the two strata rank in different orders,
/// so each score is the sum of two different ranks and the order is decided by
/// arithmetic rather than by a tie-break.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_fused_answer_is_the_same_rows_in_the_same_order_on_both_targets() {
    let profile = profile();
    let streams = vec![
        (iri(STRATUM_ONE), scripted(["alpha", "beta", "gamma"])),
        (iri(STRATUM_TWO), scripted(["beta", "gamma", "alpha"])),
    ];
    let result = block_on(fuse::<ScriptedStream, Term>(
        streams,
        &profile,
        TopK::new(8),
    ))
    .expect("the fixture streams fuse");

    // beta  = rank 2 in stratum one + rank 1 in stratum two
    // alpha = rank 1 in stratum one + rank 3 in stratum two
    // gamma = rank 3 in stratum one + rank 2 in stratum two
    let expected = [
        ("beta", RANK_2 + RANK_1),
        ("alpha", RANK_1 + RANK_3),
        ("gamma", RANK_3 + RANK_2),
    ];
    assert_eq!(result.rows.len(), expected.len());
    for (row, (candidate, score)) in result.rows.iter().zip(expected) {
        assert_eq!(row.entity.as_str(), candidate, "the row order");
        assert_eq!(
            row.score.to_decimal_lexical(),
            decimal(score),
            "the fused score of {candidate}"
        );
        assert_eq!(
            row.contributions.len(),
            2,
            "{candidate} must carry one contribution per stratum"
        );
    }

    // Every provenance triple is the same on both targets too, not only the sum.
    let provenance: Vec<String> = result
        .rows
        .iter()
        .flat_map(|row| {
            row.contributions.iter().map(move |(stratum, rank, value)| {
                format!(
                    "{} {} rank={rank} {}",
                    row.entity.as_str(),
                    stratum.as_str(),
                    value.to_decimal_lexical()
                )
            })
        })
        .collect();
    assert_eq!(
        provenance,
        vec![
            format!("beta {STRATUM_ONE} rank=2 {}", decimal(RANK_2)),
            format!("beta {STRATUM_TWO} rank=1 {}", decimal(RANK_1)),
            format!("alpha {STRATUM_ONE} rank=1 {}", decimal(RANK_1)),
            format!("alpha {STRATUM_TWO} rank=3 {}", decimal(RANK_3)),
            format!("gamma {STRATUM_ONE} rank=3 {}", decimal(RANK_3)),
            format!("gamma {STRATUM_TWO} rank=2 {}", decimal(RANK_2)),
        ]
    );

    // And the terminal report: each producer emitted exactly its three rows.
    assert_eq!(
        result.trailer.statuses.get(&iri(STRATUM_ONE)),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 })
    );
    assert_eq!(
        result.trailer.statuses.get(&iri(STRATUM_TWO)),
        Some(&ProducerStatus::Exhausted { rows_emitted: 3 })
    );
}

/// The law's identity: a digest over a canonical, length-framed encoding, which
/// must name the same law on every target.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_fusion_profile_names_the_same_identity_on_both_targets() {
    let profile = profile();
    assert_eq!(profile.id().to_hex(), PROFILE_ID_HEX);

    // The encoding the digest is taken over round-trips, so the digest is a
    // statement about the profile's fields and not about this build's layout.
    let bytes = profile.canonical_bytes();
    let decoded = FusionProfile::from_canonical_bytes(&bytes).expect("the canonical bytes decode");
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.id().to_hex(), PROFILE_ID_HEX);
}

/// **The collided regime is the same answer on both targets too.**
///
/// The other tests here fuse at a unit weight, where every adjacent rank carries
/// a distinct contribution and the fused score alone decides the order. Past the
/// depth a profile still separates, it does not: two candidates carry the same
/// score and their order falls through to the declared tie-break's later keys —
/// best stratum rank ascending, then canonical term bytes. That path is what the
/// whole "reading deeper costs resolution and nothing else" claim rests on, and
/// until now no test pinned it on a second target at all.
///
/// # The expectation, hand computed
///
/// A weight of one thousand raw units is `10^-9`, not the number one thousand.
/// Under `K = 60` the contribution at rank `r` is
/// `trunc(1000 * trunc(10^12 / (60 + r)) / 10^12)`:
///
/// ```text
/// rank 1 -> 1000 * 16393442622 / 10^12 = 16.393442622 -> 16
/// rank 2 -> 1000 * 16129032258 / 10^12 = 16.129032258 -> 16
/// ```
///
/// Both truncate to **16 raw units**, which is `0.000000000016`. The two ranks
/// are therefore indistinguishable by score, and the answer's order is decided
/// entirely by the tie-break. A target that ordered ties differently — or that
/// truncated either product differently — would return a different answer here
/// while agreeing on every unit-weight fixture in this file.
///
/// Two ranks, deliberately: the collision is reachable at the shortest possible
/// stream, so this costs nothing to run under Node.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_collided_pair_fuses_to_the_same_order_on_both_targets() {
    const COLLIDED: i128 = 16;

    let stratum = iri(STRATUM_ONE);
    let weight = Fixed::from_raw(1_000);
    let profile = FusionProfile::with_decay(
        BTreeMap::from([(stratum.clone(), weight)]),
        DecayRule::ReciprocalRank { k: K },
    )
    .expect("a strictly positive weight is a valid profile");

    // The arithmetic claim, independent of the fusion: both ranks land on one
    // value, and that value is the hand-computed one.
    for rank in [1_u64, 2] {
        assert_eq!(
            contribution(weight, rank, K)
                .expect("the contribution fits")
                .into_raw(),
            COLLIDED,
            "rank {rank} must truncate to the hand-computed 16 raw units"
        );
    }

    let stream = ScriptedStream {
        steps: VecDeque::from([
            RankedRow::new(
                1,
                Fixed::from_raw(COLLIDED),
                Term::new("alpha"),
                RowBlock::Undeclared,
            ),
            RankedRow::new(
                2,
                Fixed::from_raw(COLLIDED),
                Term::new("beta"),
                RowBlock::Undeclared,
            ),
        ]),
        emitted: 0,
        contract: unrestricted(),
        attestation: PfAttestation::UNDECLARED,
    };
    let fused = block_on(fuse::<ScriptedStream, Term>(
        vec![(stratum.clone(), stream)],
        &profile,
        TopK::new(2),
    ))
    .expect("a collided pair is not a protocol violation");

    let rows: Vec<(String, String)> = fused
        .rows
        .iter()
        .map(|row| {
            (
                row.entity.as_str().to_owned(),
                row.score.to_decimal_lexical(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("alpha".to_owned(), decimal(COLLIDED)),
            ("beta".to_owned(), decimal(COLLIDED)),
        ],
        "both targets order an exactly tied pair by best stratum rank ascending"
    );

    // And both agree that the tie was observed rather than inferred.
    assert_eq!(
        fused.trailer.resolution[&stratum].collisions_observed, 1,
        "one adjacent pair collided, on every target"
    );
}

// ---------------------------------------------------------------------------
// The evidence identity, and a fusion under a declared candidate domain
// ---------------------------------------------------------------------------

/// The block the documents stratum declares, and the block its rows are drawn
/// from.
const DOMAIN_DOCS: &str = "http://example.org/domain/documents";

/// The block the people stratum declares. Disjoint from [`DOMAIN_DOCS`], which
/// is what lets a candidate be final the moment it is read.
const DOMAIN_PEOPLE: &str = "http://example.org/domain/people";

/// The evidence identity of the two attestations
/// [`an_evidence_identity_is_the_same_bytes_and_digest_on_both_targets`] fuses,
/// as hex.
///
/// Recorded rather than derived, exactly as [`PROFILE_ID_HEX`] is and for the
/// same reason: it is a BLAKE3 digest and no hand arithmetic produces one. The
/// bytes it is a digest *of* are hand-written in that test, which is where the
/// layout claim actually lives — this constant then pins that the digest over
/// those bytes is the same number on every target.
const EVIDENCE_ID_HEX: &str = "8511177560e417f1bba69c0b3066820bb06e8fff5415fca0ef2c6df5afbb5567";

fn domain(tag: &str) -> DomainTag {
    DomainTag::parse(tag).expect("the fixture domain tags are valid IRIs")
}

/// One stream's rows, all drawn from `block`, at ranks `1..=candidates.len()`.
fn scripted_in(candidates: &[&str], block: &str) -> ScriptedStream {
    let steps = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let rank = u64::try_from(index + 1).expect("the fixture ranks fit");
            RankedRow::new(
                rank,
                contribution(Fixed::ONE, rank, K).expect("the contribution fits"),
                Term::new(*candidate),
                // A restricted stream owes a block on every row, and this is the
                // block the row is really in. An absence here would be an
                // unbacked declaration and a refusal, on every target.
                RowBlock::Declared(domain(block)),
            )
        })
        .collect();
    ScriptedStream {
        steps,
        emitted: 0,
        contract: StreamContract::new(
            DuplicatePolicy::Unique,
            RankFidelity::EXACT,
            CandidateDomains::within([domain(block)]),
            ExclusionBasis::Unavailable,
        ),
        attestation: PfAttestation::UNDECLARED,
    }
}

/// Write one length-framed part exactly as the canonical writer does: a
/// little-endian `u64` length, then the bytes.
///
/// Spelled out here rather than reached for from the crate, because the point is
/// to state the layout independently. A test that asked the encoder to describe
/// its own framing would agree with any framing at all.
fn framed(out: &mut Vec<u8>, part: &str) {
    out.extend_from_slice(&(part.len() as u64).to_le_bytes());
    out.extend_from_slice(part.as_bytes());
}

/// **An answer's evidence identity is the same bytes, and the same digest, on
/// both targets.**
///
/// The evidence identity is what two holders of two answers compare to decide
/// whether they were answered from the same indexes. A target that framed a
/// length differently, ordered the strata differently, or wrote an absence with a
/// different discriminant would hand them ids that disagreed about answers whose
/// evidence was identical — and nothing downstream could tell that apart from a
/// genuine index rebuild.
///
/// The bytes are hand-written from the layout: the version, the entry count, then
/// per stratum in canonical order the framed stratum IRI, the generation
/// discriminant with its framed spelling, and the service-level discriminant with
/// its framed reason. Every integer little-endian.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn an_evidence_identity_is_the_same_bytes_and_digest_on_both_targets() {
    const REASON: &str = "shard 3 rebuilding";

    let mut one = scripted(["alpha", "beta", "gamma"]);
    one.attestation = PfAttestation {
        generation: IndexGeneration::declared("gen-7"),
        service: ServiceLevel::Undeclared,
    };
    let mut two = scripted(["beta", "gamma", "alpha"]);
    two.attestation = PfAttestation {
        generation: IndexGeneration::declared("gen-8"),
        service: ServiceLevel::Incomplete {
            reason: REASON.to_owned(),
        },
    };

    let result = block_on(fuse::<ScriptedStream, Term>(
        vec![(iri(STRATUM_ONE), one), (iri(STRATUM_TWO), two)],
        &profile(),
        TopK::new(8),
    ))
    .expect("the fixture streams fuse");

    // What each stratum attested, carried through verbatim: the generation
    // spellings are the hosts' own bytes and the reason is the host's own words.
    assert_eq!(
        result.trailer.attestations[&iri(STRATUM_ONE)],
        PfAttestation {
            generation: IndexGeneration::declared("gen-7"),
            service: ServiceLevel::Undeclared,
        }
    );
    assert_eq!(
        result.trailer.attestations[&iri(STRATUM_TWO)],
        PfAttestation {
            generation: IndexGeneration::declared("gen-8"),
            service: ServiceLevel::Incomplete {
                reason: REASON.to_owned(),
            },
        }
    );

    // The layout, written out by hand.
    let mut expected = Vec::new();
    expected.extend_from_slice(&EVIDENCE_VERSION.to_le_bytes());
    expected.extend_from_slice(&2_u64.to_le_bytes());
    framed(&mut expected, STRATUM_ONE);
    expected.push(1); // the generation is declared
    framed(&mut expected, "gen-7");
    expected.push(0); // and nothing was said about its wholeness
    framed(&mut expected, STRATUM_TWO);
    expected.push(1);
    framed(&mut expected, "gen-8");
    expected.push(1); // this one declared itself short
    framed(&mut expected, REASON);
    assert_eq!(
        result.trailer.evidence_canonical_bytes(),
        expected,
        "the evidence encoding is the same bytes on both targets"
    );

    // And the identity an answer is compared by is the digest of exactly those
    // bytes — re-derived here rather than taken on the trailer's word, which is
    // the audit a holder of an answer performs.
    assert_eq!(
        result.trailer.evidence_id,
        EvidenceId::from_canonical(&expected),
        "the id is the digest of the bytes the trailer publishes"
    );
    assert_eq!(result.trailer.evidence_id.to_hex(), EVIDENCE_ID_HEX);

    // One stratum served from an index it called short, so every fused score is
    // an ESTIMATE rather than a value — and the answer names which stratum made
    // it one, on both sides. Fusion scores by rank, so a stratum that fails to
    // name a row both withholds that row's contribution and promotes every row
    // behind it: the error runs in both directions, which is why the same
    // stratum appears under `deficit` and under `inflation`. Its declared order
    // is faithful, so both directions stay finite and `unbounded` is empty.
    assert_eq!(
        result.trailer.exactness,
        ScoreExactness::Estimated {
            deficit: BTreeSet::from([iri(STRATUM_TWO)]),
            inflation: BTreeSet::from([iri(STRATUM_TWO)]),
            unbounded: BTreeSet::new(),
        },
        "the shortfall reaches the exactness verdict identically on both targets"
    );
}

/// **A fusion under a declared candidate domain reads the same depth, and
/// answers the same rows, on both targets.**
///
/// A declaration that the two strata draw from disjoint blocks is what licenses
/// the engine to certify a candidate before every stream has been consulted about
/// it. That makes the *depth read* part of the answer's cross-target contract and
/// not merely an optimization: a target that computed the finality test
/// differently would read a different number of ranks, and — because a stream
/// stopped short reports a ceiling rather than exhaustion — would hand back a
/// different terminal report for the same streams.
///
/// The scores are the header's hand-computed unit-weight contributions. The two
/// leading candidates tie exactly, so their order falls to the declared
/// tie-break's later keys, and the streams are longer than the bound so the
/// bounded stop is what ends them.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn a_declared_candidate_domain_bounds_the_same_read_on_both_targets() {
    let docs = ["doc-1", "doc-2", "doc-3", "doc-4"];
    let people = ["person-1", "person-2", "person-3", "person-4"];
    let result = block_on(fuse::<ScriptedStream, Term>(
        vec![
            (iri(STRATUM_ONE), scripted_in(&docs, DOMAIN_DOCS)),
            (iri(STRATUM_TWO), scripted_in(&people, DOMAIN_PEOPLE)),
        ],
        &profile(),
        TopK::new(2),
    ))
    .expect("declared blocks its rows back are not a protocol violation");

    // Each candidate is named by exactly one stratum, so each score is that one
    // contribution. `doc-1` and `person-1` are therefore tied exactly, and the
    // tie-break — best stratum rank, then canonical term bytes — orders them.
    let rows: Vec<(String, String)> = result
        .rows
        .iter()
        .map(|row| {
            (
                row.entity.as_str().to_owned(),
                row.score.to_decimal_lexical(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("doc-1".to_owned(), decimal(RANK_1)),
            ("person-1".to_owned(), decimal(RANK_1)),
        ],
        "both targets return the same two rows, in the same tie-broken order"
    );

    // The depth read, which is what the declaration buys. Two ranks per stream:
    // the rank-one row that becomes the answer, and one more head whose
    // contribution is the threshold the second row is certified against.
    for stratum in [STRATUM_ONE, STRATUM_TWO] {
        assert_eq!(
            result.trailer.resolution[&iri(stratum)].ranks_pulled,
            2,
            "the declaration stopped {stratum} two ranks in, on every target"
        );
        assert_eq!(
            result.trailer.statuses.get(&iri(stratum)),
            Some(&ProducerStatus::CeilingReached {
                bound: Fixed::from_raw(RANK_2)
            }),
            "a stream the bound stopped is closed at the contribution of the last \
             rank read from it, never drained into an exhaustion it did not earn"
        );
    }
}

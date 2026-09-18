// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! The one recorded expectation is the fusion profile's identity, because it is
//! a BLAKE3 digest and there is no hand arithmetic that produces one. Pinning it
//! is still the cross-target claim this file exists to make: the digest is taken
//! over the profile's canonical, length-framed encoding, so a target that framed
//! a `u32` or ordered a map differently would produce a different hex here.
//!
//! The comparison is on the **decimal lexical**, not on the raw `i128`, because
//! the lexical is what a consumer receives and what a serializer writes.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::task::{Context, Poll, Waker};

use purrdf_retrieval::{
    DecayRule, DuplicatePolicy, Fixed, FusionProfile, Iri, ProducerReceipt, ProducerStatus,
    ProtocolError, RankedStream, StreamContract, Term, TopK, contribution, fuse,
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
    steps: VecDeque<(u64, Fixed, Term)>,
    emitted: u64,
}

// The trait's methods are `async`; this mock's body is synchronous because its
// rows are pre-scripted.
#[allow(clippy::unused_async_trait_impl)]
impl RankedStream for ScriptedStream {
    type Item = Term;

    async fn next(&mut self) -> Result<Option<(u64, Fixed, Self::Item)>, ProtocolError> {
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

    /// The fixture's scripted rows are distinct and strictly rank-ordered, so
    /// this is what they honestly declare. It is part of the determinism claim:
    /// the contract decides what the engine holds and refuses, so a target that
    /// read it differently would fuse differently.
    fn contract(&self) -> StreamContract {
        StreamContract::new(DuplicatePolicy::Unique)
    }
}

/// Script one stream's three rows, in the order `candidates` names them.
fn scripted(candidates: [&str; 3]) -> ScriptedStream {
    let steps = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let rank = u64::try_from(index + 1).expect("three rows");
            (
                rank,
                contribution(Fixed::ONE, rank, K).expect("the contribution fits"),
                Term::new(*candidate),
            )
        })
        .collect();
    ScriptedStream { steps, emitted: 0 }
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

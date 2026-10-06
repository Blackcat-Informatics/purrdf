// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fusion fixtures' shared pieces: a synchronous poll, the strata and
//! profile they fuse under, the permuted universe their lazy streams emit, and
//! a deterministic rendering of a fusion result.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use std::collections::BTreeMap;
use std::future::Future;
use std::task::{Context, Poll, Waker};

use purrdf_retrieval::{
    CandidateDomains, DecayRule, DuplicatePolicy, ExclusionBasis, Fixed, FusionProfile,
    FusionResult, Iri, RankFidelity, StreamContract, Term,
};

/// Poll a future the fixture streams drive exactly once.
///
/// `Waker::noop()` needs no thread and no allocation, so this drives a future on
/// any target, `wasm32-unknown-unknown` included — where
/// [`purrdf_retrieval::block_on`]'s thread parking is not available.
///
/// # Panics
///
/// When the future pends: the fixture streams are synchronous and never do.
pub fn ready<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("the fixture streams are synchronous and never pend"),
    }
}

/// The fixture stratum `http://example.org/stratum/{suffix}`.
pub fn stratum(suffix: &str) -> Iri {
    Iri::parse(&format!("http://example.org/stratum/{suffix}")).expect("fixture IRIs are valid")
}

/// A fixture profile over the named strata.
///
/// There is no contribution-maximum argument to pass: the profile derives it
/// from `weights`, because a candidate may surface at most once per stratum.
pub fn profile(weights: &[(&str, Fixed)], k: u32) -> FusionProfile {
    let map: BTreeMap<Iri, Fixed> = weights
        .iter()
        .map(|(name, weight)| (stratum(name), *weight))
        .collect();
    FusionProfile::with_decay(map, DecayRule::ReciprocalRank { k })
        .expect("fixture profile is valid")
}

/// The contract every fixture producer here declares, and the one a hand-built
/// stream in this file states: no repeats. It is spelled once so the registered
/// declaration above and the streams below cannot drift into describing two
/// different promises. The contiguous, ascending ranks these streams emit are
/// not part of it — that law holds for every stream and is checked rank by rank
/// rather than declared.
pub fn unique_items() -> StreamContract {
    StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
        ExclusionBasis::Unavailable,
    )
}

/// How many candidates the strata may disagree about at once.
///
/// Each stratum emits the same universe of candidates permuted **within** blocks
/// of this size, so a candidate's ranks differ across strata by less than one
/// block and the frontier holds the candidates of at most a couple of blocks at
/// any moment. It is the disagreement window, and it is what the frontier's size
/// is proportional to — not the stream length, which is the claim.
pub const DISAGREEMENT_BLOCK: u64 = 4;

/// The candidate one stratum emits at 1-based `rank`.
///
/// Stratum 0 emits the universe in order; stratum 1 reverses each block; stratum
/// 2 rotates each block by half its width. Each is a permutation of the same
/// universe, so every stream emits distinct items (the protocol's uniqueness
/// rule) and every candidate is eventually seen by every stratum — which is what
/// makes the frontier hold candidates awaiting confirmation rather than sit
/// empty. A single-stratum fixture proves nothing here: with one stream there is
/// nothing to await, and the frontier never holds anything at all.
pub fn permuted_index(stream: usize, rank: u64) -> u64 {
    let index = rank - 1;
    let block = index / DISAGREEMENT_BLOCK;
    let offset = index % DISAGREEMENT_BLOCK;
    let permuted = match stream {
        0 => offset,
        1 => DISAGREEMENT_BLOCK - 1 - offset,
        _ => (offset + DISAGREEMENT_BLOCK / 2) % DISAGREEMENT_BLOCK,
    };
    block * DISAGREEMENT_BLOCK + permuted
}

/// Render a fusion result as deterministic text: rows by rank, then statuses by
/// stratum, then the profile identity. Nothing here depends on a `HashMap`.
pub fn render(result: &FusionResult<Term>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for row in &result.rows {
        let _ = writeln!(
            out,
            "row {} score={} witness={}",
            row.entity.as_str(),
            row.score.to_decimal_lexical(),
            row.threshold_witness.to_decimal_lexical()
        );
        for (stratum, rank, score) in &row.contributions {
            let _ = writeln!(
                out,
                "  {} rank={rank} score={}",
                stratum.as_str(),
                score.to_decimal_lexical()
            );
        }
    }
    for (stratum, status) in &result.trailer.statuses {
        let _ = writeln!(out, "status {} {status:?}", stratum.as_str());
    }
    let _ = writeln!(out, "profile {}", result.trailer.profile_id.to_hex());
    out
}

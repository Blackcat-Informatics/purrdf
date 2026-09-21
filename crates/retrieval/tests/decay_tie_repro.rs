// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! **The decay-tie reproduction, kept runnable.**
//!
//! The defect: fusion refused a well-formed producer stream because the
//! profile's own fixed-point decay gave two adjacent ranks one contribution.
//! The stream's ranks were strictly increasing, contiguous and unique, and its
//! declared contract was truthful; nothing the producer controlled was wrong.
//!
//! This file keeps that reproduction as assertions rather than as printed
//! lines, so the behaviour it measures cannot quietly regress. It reaches the
//! defect through the public surface — [`RankedStreamAdapter`] over
//! [`RankedStreamImpl`], fused through [`fuse`] — which is the documented seam
//! for stopping and resuming between stages, and a different path from the
//! hand-built streams `fusion.rs` uses.
//!
//! # Two API changes that removed the cause
//!
//! `StreamContract::new` no longer takes a rank-ordering declaration. That
//! declaration was read in contribution space, where the producer supplies no
//! term of the value, and that reading is what manufactured this refusal; the
//! one rank law — contiguous, ascending — is held for every stream regardless,
//! row by row as the ranks arrive. `FusionProfile::new` is now
//! [`FusionProfile::with_decay`], which requires naming the decay rule rather
//! than defaulting to one. Both are spelled out below, so this file also shows
//! the current spelling of a stream and a profile end to end.
//!
//! # What each case is for
//!
//! The first and third used to be refused and now answer. The second and fourth
//! were always fine and must stay that way — they are the neighbouring valid
//! cases that catch a fix which merely moved the boundary. The fifth is the
//! trap: a small row bound certifies early, never pulls to the collision, and
//! reports a false all-clear, which is how a regression test written that way
//! passes while the defect is still present.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use purrdf_retrieval::{
    CandidateDomains, DecayRule, DuplicatePolicy, Fixed, FusionProfile, Iri, RankFidelity,
    RankedStreamAdapter, RankedStreamImpl, RowBlock, StreamContract, StreamEnding, Term, TopK,
    fuse,
};

/// A minimal executor. The adapter's rows are already materialized, so nothing
/// here ever actually pends; the crate is runtime-agnostic by design.
fn block_on<F: Future>(future: F) -> F::Output {
    struct Park(std::thread::Thread);
    impl Wake for Park {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Park(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// Fuse `ranks` contiguous ranks under one stratum at weight `weight`.
///
/// The rows are as well-formed as a producer can make them: ranks one upward
/// with no gaps, distinct items, and a contract that truthfully declares no
/// repeats. Every contribution is minted by the adapter from the profile itself,
/// so the producer never even supplies the value that used to be refused.
fn run(weight: Fixed, ranks: u64, top_k: usize) -> Result<usize, String> {
    let stratum = Iri::parse("https://example.org/stratum/a").expect("a valid fixture IRI");
    let profile = FusionProfile::with_decay(
        BTreeMap::from([(stratum.clone(), weight)]),
        DecayRule::ReciprocalRank { k: 60 },
    )
    .expect("a strictly positive weight is a valid profile");
    let rows: Vec<(u64, Term, RowBlock)> = (1..=ranks)
        .map(|rank| {
            (
                rank,
                Term::new(format!("d{rank:07}")),
                // The contract below declares `Unrestricted`, which owes no
                // per-row block: this repro is about decay ties, not domains.
                RowBlock::Undeclared,
            )
        })
        .collect();
    let contract = StreamContract::new(
        DuplicatePolicy::Unique,
        RankFidelity::EXACT,
        CandidateDomains::Unrestricted,
    );
    let adapter = RankedStreamAdapter::new(
        RankedStreamImpl::new(rows, StreamEnding::Exhausted),
        contract,
        &profile,
        &stratum,
    )
    .expect("the profile weights this stratum");
    block_on(fuse(vec![(stratum, adapter)], &profile, TopK::new(top_k)))
        .map(|fused| fused.rows.len())
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn two_ranks_at_a_weight_whose_decay_collides_immediately() {
    // Was: Err(Protocol(RepeatedContribution { rank: 2, value: Fixed(16) })).
    // A thousand raw units is `10^-9`; 1000/61 and 1000/62 both truncate to 16,
    // so the shortest stream that can collide already does.
    assert_eq!(run(Fixed::from_raw(1_000), 2, 10), Ok(2));
}

#[test]
fn two_ranks_at_a_whole_unit_weight_are_unaffected() {
    // The neighbouring valid case, and the control: this one always answered,
    // and a fix that broke it would have moved the boundary rather than removed
    // it. Only the weight differs from the case above.
    let one = Fixed::from_integer(1).expect("one is representable");
    assert_eq!(run(one, 2, 10), Ok(2));
}

#[test]
fn fourteen_hundred_ranks_at_the_weight_that_collided_at_rank_973() {
    // Was: Err(Protocol(RepeatedContribution { rank: 973, value: Fixed(968) })).
    // The collision is real and is simply no longer a refusal: past it the fused
    // score stops separating ranks and the declared total tie-break orders them.
    assert_eq!(run(Fixed::from_raw(1_000_000), 1_400, 1_400), Ok(1_400));
}

#[test]
fn fourteen_hundred_ranks_at_a_weight_that_never_collides_in_range() {
    // The other neighbouring valid case: a hundred million raw units does not
    // collide until about rank ten thousand, well past this stream, so this one
    // always answered and must continue to.
    assert_eq!(run(Fixed::from_raw(100_000_000), 1_400, 1_400), Ok(1_400));
}

#[test]
fn a_small_row_bound_is_a_false_all_clear() {
    // The trap, pinned. Identical weight and identical rows to the 1400-rank
    // case, bounded at five: fusion certifies the top five and never pulls as
    // far as rank 973, so this answered even while the defect was present. A
    // regression test for this defect written with a small bound proves nothing.
    assert_eq!(run(Fixed::from_raw(1_000_000), 1_400, 5), Ok(5));
}

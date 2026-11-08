// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The caller-owned **stop signal** a long fixpoint polls at owned work boundaries.
//!
//! # A stop signal is not a limit, and that distinction is the whole module
//!
//! This crate's [limits refuse; they never truncate](crate#limits-refuse-they-never-truncate):
//! the stored-fact and join-step ceilings are optional caller settings, and
//! each is a NUMBER the caller states before the run — every caller passing the same
//! options gets the same answer or the same refusal, and the effective limits are part of
//! the result's contract hash.
//!
//! A [`StopSignal`] is the other thing. It carries no number and is part of no identity,
//! because it changes no answer. A run that is not stopped returns precisely the answer
//! it would have returned with no signal attached. Scheduled evaluation also polls
//! candidate, scan and callback boundaries within a round. Unscheduled and session
//! entry points retain their documented boundary checks. A
//! complete-only call that IS stopped returns a typed refusal
//! ([`EvalError::Stopped`](crate::seminaive::EvalError::Stopped),
//! [`ChaseError::Stopped`](crate::chase::ChaseError::Stopped)) carrying the consumption
//! measured up to that point. [`crate::seminaive::EvaluationSession`] retains a
//! stopped execution, exposes its certified facts through a distinct
//! [`crate::seminaive::PartialModel`], and accepts a fresh signal on resume.
//! It cannot return a complete model before the original execution finishes.
//!
//! What this buys is the honesty of a host's wall deadline. A materialized closure is
//! routinely the expensive half of an entailment-regime query, so a deadline that bounds
//! only the SPARQL evaluation over the finished closure is a deadline in name: the call
//! that overruns it is precisely the one that never reaches the evaluator.
//!
//! # The contract: latching, cheap, and answer-blind
//!
//! An implementation MUST latch — once [`StopSignal::stopped`] answers `true` it answers
//! `true` forever. A signal that could un-fire would make "did this run finish?" depend on
//! *when* the fixpoint happened to look, which is the nondeterminism this crate exists
//! without. It MUST also be cheap (scheduled execution polls inside hot work loops)
//! and it MUST NOT depend on
//! the data: a signal that fires as a function of what has been derived would be a budget
//! wearing a different hat, and would reintroduce exactly the answer-affecting optionality
//! the module opens by refusing.
//!
//! The crate reads no clock and owns no cancellation bit. Both live in the HOST — a wall
//! deadline needs a clock, which `wasm32-unknown-unknown` does not have and which would
//! make a run irreproducible if this crate read one — so the signal arrives as a trait
//! object and this crate only ever asks it a yes/no question.

pub use purrdf_core::StopSignal;
pub(crate) use purrdf_core::governor::is_stopped;

#[cfg(test)]
mod tests {
    use super::*;

    /// A signal that answers `true` from its `n`-th poll onward, and latches.
    #[derive(Debug)]
    struct AfterNPolls {
        fire_at: core::sync::atomic::AtomicU64,
    }

    impl StopSignal for AfterNPolls {
        fn stopped(&self) -> bool {
            use core::sync::atomic::Ordering;
            let left = self.fire_at.load(Ordering::Relaxed);
            if left == 0 {
                return true;
            }
            self.fire_at.store(left - 1, Ordering::Relaxed);
            false
        }
    }

    /// The absent signal never stops, and a present one is asked.
    #[test]
    fn an_absent_signal_never_stops_and_a_present_one_is_asked() {
        assert!(!is_stopped(None));
        let signal = AfterNPolls {
            fire_at: core::sync::atomic::AtomicU64::new(2),
        };
        assert!(!is_stopped(Some(&signal)));
        assert!(!is_stopped(Some(&signal)));
        assert!(is_stopped(Some(&signal)));
        // Latched: it never un-fires.
        assert!(is_stopped(Some(&signal)));
    }
}

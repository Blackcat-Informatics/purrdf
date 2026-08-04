// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The caller-owned **stop signal** a long fixpoint polls at its round boundary.
//!
//! # A stop signal is not a budget, and that distinction is the whole module
//!
//! This crate states, and keeps, that [budgets are constants, not
//! knobs](crate#budgets-are-constants-not-knobs): step, fact and arena ceilings are fixed
//! and merely *reported*, because a caller-supplied ceiling would mean two callers running
//! the same program over the same input get different **answers** — semantic optionality
//! arriving through a parameter instead of through a Cargo feature.
//!
//! A [`StopSignal`] is the other thing, and it is admitted for exactly the reason a budget
//! is refused. It changes no answer. A run that is not stopped returns precisely the answer
//! it would have returned with no signal attached — the poll is a load and a branch at a
//! round boundary, and the rounds are the ones the fixpoint was going to run anyway — and a
//! run that IS stopped returns **no answer at all**: a typed refusal
//! ([`EvalError::Stopped`](crate::seminaive::EvalError::Stopped),
//! [`ChaseError::Stopped`](crate::chase::ChaseError::Stopped)) carrying the consumption
//! measured up to that point. There is no third outcome, so there is no partial closure a
//! consumer could mistake for a complete one and no schedule of charges that would have to
//! be versioned, frozen or pinned.
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
//! without. It MUST also be cheap (it is polled once per round) and it MUST NOT depend on
//! the data: a signal that fires as a function of what has been derived would be a budget
//! wearing a different hat, and would reintroduce exactly the answer-affecting optionality
//! the module opens by refusing.
//!
//! The crate reads no clock and owns no cancellation bit. Both live in the HOST — a wall
//! deadline needs a clock, which `wasm32-unknown-unknown` does not have and which would
//! make a run irreproducible if this crate read one — so the signal arrives as a trait
//! object and this crate only ever asks it a yes/no question.

use core::fmt::Debug;

/// A caller-owned, latching stop request polled at a fixpoint's round boundary.
///
/// See the [module documentation](self) for the contract every implementation is bound by
/// (latching, cheap, data-independent) and for why a stop signal is admitted where a
/// caller-supplied budget is refused.
///
/// `Send + Sync` because a host builds one signal and shares it across whatever threads it
/// runs work on; `Debug` because it is reachable from types this crate derives `Debug` for.
pub trait StopSignal: Send + Sync + Debug {
    /// Whether the caller has asked this run to stop. Latching: see the trait
    /// documentation.
    fn stopped(&self) -> bool;
}

/// Poll `stop`, treating "no signal at all" as "not stopped".
///
/// The one place `Option<&dyn StopSignal>` is read, so an ungoverned run pays exactly one
/// null check per round and every governed one polls through the same expression.
#[inline]
pub(crate) fn is_stopped(stop: Option<&dyn StopSignal>) -> bool {
    stop.is_some_and(StopSignal::stopped)
}

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

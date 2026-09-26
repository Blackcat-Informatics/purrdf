// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Property-based testing on a recorded choice sequence.
//!
//! A property states something that must hold for every input a
//! [`Strategy`] generates; [`crate::prop_test!`] runs it over many generated
//! inputs and, when one fails, shrinks it to a simple failing input and prints
//! that input together with the choice sequence that reproduces it.
//!
//! ```
//! use purrdf_testkit::prop::prelude::*;
//!
//! prop_test! {
//!     #![prop_config(Config::with_cases(64))]
//!
//!     // In a test file this function carries `#[test]`.
//!     fn reversing_twice_is_the_identity(items in prop::collection::vec(any::<u8>(), 0..32)) {
//!         let mut twice = items.clone();
//!         twice.reverse();
//!         twice.reverse();
//!         prop_assert_eq!(twice, items);
//!     }
//! }
//!
//! reversing_twice_is_the_identity();
//! ```
//!
//! # The model
//!
//! Every generator draws bounded integers from a [`Choices`] source, which
//! records them (see the [`choices`](Choices) documentation). A value is a pure
//! function of its record, so shrinking never touches values: it edits the
//! record — deletes spans of it, zeroes and lowers entries, swaps adjacent
//! entries into order — and replays it, keeping any edit that still fails and
//! draws a shortlex-smaller record. Whatever the replay produces is, by
//! construction, a value the generator can produce, so a shrunk input always
//! satisfies its generator's constraints, and shrinking works through
//! [`Strategy::prop_map`], [`Strategy::prop_flat_map`],
//! [`Strategy::prop_filter`] and stateful runs alike. Each generator maps the
//! choice 0 to its simplest value: the low end of a range, `false`, `None`,
//! the first alternative, the shortest collection.
//!
//! # Determinism
//!
//! Nothing reads OS entropy. A property's seed is [`seed_for`] its name — the
//! module path and function name under [`crate::prop_test!`] — so every run on
//! every machine sees the same cases. `PURRDF_PROP_SEED` (decimal, or hex
//! after `0x`) replaces the seed for a run, and a failure prints the seed it
//! used. The random stream is an in-house xoshiro256** seeded through
//! SplitMix64.
//!
//! # Replaying a failure
//!
//! A failure prints the shrunk input and its choice sequence in hexadecimal.
//! `replay(&strategy, hex)` ([`replay`]) regenerates the input from it, which is how a
//! counterexample is kept as an ordinary named `#[test]`.

mod arbitrary;
mod choices;
pub mod collection;
mod runner;
pub mod sample;
pub mod state_machine;
mod strategy;
pub mod string;

pub use arbitrary::{AnyBool, AnyChar, AnyInt, Arbitrary, any};
pub use choices::{Choices, HexError, Invalid};
pub use runner::{
    CASES_VARIABLE, Config, FailedCase, Failure, RunSummary, Runner, SEED_VARIABLE, TestCaseError,
    cases_from_env, parse_seed, replay, run_test, seed_for,
};
pub use strategy::{BoxedStrategy, Filter, FilterMap, FlatMap, Just, Map, Strategy, Union};

/// Optional values.
pub mod option {
    use super::choices::{Choices, Invalid};
    use super::strategy::Strategy;

    /// `None` or `Some` of a value from `inner`, each with probability ½;
    /// shrinks to `None`.
    pub fn of<S: Strategy>(inner: S) -> OptionStrategy<S> {
        weighted(0.5, inner)
    }

    /// `Some` of a value from `inner` with probability `p_some`, `None`
    /// otherwise; shrinks to `None`.
    pub fn weighted<S: Strategy>(p_some: f64, inner: S) -> OptionStrategy<S> {
        assert!(
            (0.0..=1.0).contains(&p_some),
            "the probability {p_some} is outside [0, 1]"
        );
        OptionStrategy { inner, p_some }
    }

    /// See [`of`].
    #[derive(Debug, Clone)]
    pub struct OptionStrategy<S> {
        inner: S,
        p_some: f64,
    }

    impl<S: Strategy> Strategy for OptionStrategy<S> {
        type Value = Option<S::Value>;

        fn generate(&self, choices: &mut Choices) -> Result<Self::Value, Invalid> {
            if choices.weighted_bool(self.p_some)? {
                self.inner.generate(choices).map(Some)
            } else {
                Ok(None)
            }
        }
    }
}

/// Booleans.
pub mod bool {
    use super::arbitrary::AnyBool;
    use super::choices::{Choices, Invalid};
    use super::strategy::Strategy;

    /// Either value, equally likely; shrinks to `false`.
    pub const ANY: AnyBool = AnyBool;

    /// `true` with probability `p_true`; shrinks to `false`.
    pub fn weighted(p_true: f64) -> Weighted {
        assert!(
            (0.0..=1.0).contains(&p_true),
            "the probability {p_true} is outside [0, 1]"
        );
        Weighted { p_true }
    }

    /// See [`weighted`].
    #[derive(Debug, Clone, Copy)]
    pub struct Weighted {
        p_true: f64,
    }

    impl Strategy for Weighted {
        type Value = bool;

        fn generate(&self, choices: &mut Choices) -> Result<bool, Invalid> {
            choices.weighted_bool(self.p_true)
        }
    }
}

/// Everything a property file needs: `use purrdf_testkit::prop::prelude::*;`.
///
/// It brings the strategy vocabulary into scope, the property and assertion
/// macros, and this module itself as `prop`, so `prop::collection::vec` and
/// `prop::sample::select` resolve.
pub mod prelude {
    pub use super::{Arbitrary, BoxedStrategy, Config, Just, Strategy, TestCaseError, Union, any};
    pub use crate::prop;
    pub use crate::{
        prop_assert, prop_assert_eq, prop_assert_ne, prop_assume, prop_oneof, prop_test,
    };
}

/// Declare property tests.
///
/// ```text
/// prop_test! {
///     #![prop_config(Config::with_cases(512))]   // optional; default 256 cases
///
///     /// Documentation and attributes pass through.
///     #[test]
///     fn name(a in strategy_a, (b, c) in strategy_bc) {
///         prop_assert!(…);
///     }
/// }
/// ```
///
/// Each function runs its body over inputs drawn from the strategies (as one
/// tuple), under the configuration, seeded by [`seed_for`] the function's
/// module path and name. The body may `return Ok(())` early, use `?` on a
/// `Result<_, TestCaseError>` (or any error type), and use the `prop_assert*`
/// macros; a panic fails the case as a returned failure does.
#[macro_export]
macro_rules! prop_test {
    (#![prop_config($config:expr)] $($rest:tt)*) => {
        $crate::__prop_test_functions! { ($config) $($rest)* }
    };
    ($($rest:tt)*) => {
        $crate::__prop_test_functions! { ($crate::prop::Config::default()) $($rest)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __prop_test_functions {
    (($config:expr)) => {};
    (
        ($config:expr)
        $(#[$meta:meta])*
        fn $name:ident($($argument:pat in $strategy:expr),+ $(,)?) $body:block
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[allow(unreachable_code)]
        fn $name() {
            $crate::prop::run_test(
                &$config,
                ::core::concat!(::core::module_path!(), "::", ::core::stringify!($name)),
                &($($strategy,)+),
                |($($argument,)+)| {
                    $body;
                    ::core::result::Result::Ok(())
                },
            );
        }
        $crate::__prop_test_functions! { ($config) $($rest)* }
    };
}

/// A weighted or equally weighted choice between strategies with one value
/// type: `prop_oneof![a, b]` or `prop_oneof![3 => a, 1 => b]`. The first
/// alternative is the shrink target.
#[macro_export]
macro_rules! prop_oneof {
    ($($weight:expr => $strategy:expr),+ $(,)?) => {
        $crate::prop::Union::new_weighted(::std::vec![
            $(($weight, $crate::prop::Strategy::boxed($strategy))),+
        ])
    };
    ($($strategy:expr),+ $(,)?) => {
        $crate::prop::Union::new(::std::vec![
            $($crate::prop::Strategy::boxed($strategy)),+
        ])
    };
}

/// Fail the case unless `condition` holds, with an optional formatted message.
#[macro_export]
macro_rules! prop_assert {
    ($condition:expr $(,)?) => {
        $crate::prop_assert!(
            $condition,
            "assertion failed: {}",
            ::core::stringify!($condition)
        )
    };
    ($condition:expr, $($format:tt)+) => {
        if !$condition {
            return ::core::result::Result::Err($crate::prop::TestCaseError::fail(
                ::std::format!($($format)+),
            ));
        }
    };
}

/// Fail the case unless the two values are equal, showing both.
#[macro_export]
macro_rules! prop_assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left, right) => {
                if !(*left == *right) {
                    return ::core::result::Result::Err($crate::prop::TestCaseError::fail(
                        ::std::format!(
                            "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                            left, right
                        ),
                    ));
                }
            }
        }
    };
    ($left:expr, $right:expr, $($format:tt)+) => {
        match (&$left, &$right) {
            (left, right) => {
                if !(*left == *right) {
                    return ::core::result::Result::Err($crate::prop::TestCaseError::fail(
                        ::std::format!(
                            "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`: {}",
                            left, right, ::std::format!($($format)+)
                        ),
                    ));
                }
            }
        }
    };
}

/// Fail the case if the two values are equal, showing them.
#[macro_export]
macro_rules! prop_assert_ne {
    ($left:expr, $right:expr $(,)?) => {
        match (&$left, &$right) {
            (left, right) => {
                if *left == *right {
                    return ::core::result::Result::Err($crate::prop::TestCaseError::fail(
                        ::std::format!(
                            "assertion failed: `(left != right)`\n  left: `{:?}`,\n right: `{:?}`",
                            left, right
                        ),
                    ));
                }
            }
        }
    };
    ($left:expr, $right:expr, $($format:tt)+) => {
        match (&$left, &$right) {
            (left, right) => {
                if *left == *right {
                    return ::core::result::Result::Err($crate::prop::TestCaseError::fail(
                        ::std::format!(
                            "assertion failed: `(left != right)`\n  left: `{:?}`,\n right: `{:?}`: {}",
                            left, right, ::std::format!($($format)+)
                        ),
                    ));
                }
            }
        }
    };
}

/// Reject the case — draw another input, charged to the reject budget —
/// unless `condition` holds.
#[macro_export]
macro_rules! prop_assume {
    ($condition:expr $(,)?) => {
        $crate::prop_assume!($condition, "assumption failed: {}", ::core::stringify!($condition))
    };
    ($condition:expr, $($format:tt)+) => {
        if !$condition {
            return ::core::result::Result::Err($crate::prop::TestCaseError::reject(
                ::std::format!($($format)+),
            ));
        }
    };
}

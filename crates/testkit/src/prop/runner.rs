// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Running a property: cases, rejects, shrinking and the failure report.

use std::any::Any;
use std::cmp::Ordering;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

use super::choices::{Choices, Invalid, SplitMix64, Xoshiro256};
use super::strategy::Strategy;

/// The environment variable that replaces every property's default seed.
pub const SEED_VARIABLE: &str = "PURRDF_PROP_SEED";

/// The environment variable [`cases_from_env`] reads.
pub const CASES_VARIABLE: &str = "PURRDF_PROP_CASES";

/// How a property runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Passing cases required. Rejected cases do not count.
    pub cases: u32,
    /// Rejected candidates — filter misses, unmet sizes, failed transition
    /// preconditions and [`crate::prop_assume!`] — tolerated over the whole
    /// run before it fails.
    pub max_rejects: u32,
    /// Candidate replays the shrinker may spend on one failure.
    pub max_shrink_iters: u32,
}

impl Default for Config {
    /// 256 cases, 65 536 rejects, 8 192 shrink replays.
    fn default() -> Self {
        Self {
            cases: 256,
            max_rejects: 65_536,
            max_shrink_iters: 8_192,
        }
    }
}

impl Config {
    /// The default configuration with `cases` cases.
    pub fn with_cases(cases: u32) -> Self {
        Self {
            cases,
            ..Self::default()
        }
    }
}

/// `PURRDF_PROP_CASES` when it is set, `default` otherwise: the case count of
/// a property whose owner wants a deeper search available on demand. A value
/// that is not a positive integer is refused rather than ignored.
pub fn cases_from_env(default: u32) -> u32 {
    match std::env::var(CASES_VARIABLE) {
        Ok(text) => match text.trim().parse::<u32>() {
            Ok(cases) if cases > 0 => cases,
            _ => panic!("{CASES_VARIABLE} is `{text}`, which is not a positive integer"),
        },
        Err(std::env::VarError::NotPresent) => default,
        Err(std::env::VarError::NotUnicode(text)) => {
            panic!("{CASES_VARIABLE} is not Unicode: {text:?}")
        }
    }
}

/// The deterministic default seed of the property named `name`: FNV-1a over
/// the name, finalised by SplitMix64. No property ever reads OS entropy.
pub fn seed_for(name: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    SplitMix64::new(hash).next_u64()
}

/// Parse a seed as `PURRDF_PROP_SEED` spells it: decimal, or hexadecimal
/// after `0x`.
pub fn parse_seed(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => text.parse::<u64>(),
    };
    parsed.map_err(|error| format!("`{text}` is not a seed (decimal, or hex after 0x): {error}"))
}

fn seed_from_env(name: &str) -> u64 {
    match std::env::var(SEED_VARIABLE) {
        Ok(text) => parse_seed(&text).unwrap_or_else(|error| panic!("{SEED_VARIABLE}: {error}")),
        Err(std::env::VarError::NotPresent) => seed_for(name),
        Err(std::env::VarError::NotUnicode(text)) => {
            panic!("{SEED_VARIABLE} is not Unicode: {text:?}")
        }
    }
}

/// Why one case did not pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestCaseError {
    /// The property is false for this input.
    Fail(String),
    /// The input is outside the property's domain; draw another.
    Reject(String),
}

impl TestCaseError {
    /// A failure with `message`.
    pub fn fail(message: impl Into<String>) -> Self {
        Self::Fail(message.into())
    }

    /// A rejection with `reason`.
    pub fn reject(reason: impl Into<String>) -> Self {
        Self::Reject(reason.into())
    }
}

impl fmt::Display for TestCaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fail(message) => write!(f, "{message}"),
            Self::Reject(reason) => write!(f, "rejected: {reason}"),
        }
    }
}

impl<E: std::error::Error> From<E> for TestCaseError {
    fn from(error: E) -> Self {
        Self::Fail(error.to_string())
    }
}

/// What a passing run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    /// Passing cases, which is always the configured count.
    pub cases: u32,
    /// Rejected candidates along the way.
    pub rejects: u32,
}

/// A failing property, shrunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedCase {
    /// The property's name.
    pub name: String,
    /// The seed the run used.
    pub seed: u64,
    /// Cases that passed before the failure.
    pub passed: u32,
    /// The failure message of the shrunk input.
    pub message: String,
    /// The shrunk input, `Debug`-formatted.
    pub minimal: String,
    /// The shrunk input's choice sequence, as [`Choices::from_hex`] reads it.
    pub choices_hex: String,
    /// Shrinks accepted on the way from the first failure to `minimal`.
    pub shrinks: u32,
}

/// Why a run failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// A case failed; the report carries the shrunk input.
    Failed(Box<FailedCase>),
    /// The run rejected more candidates than [`Config::max_rejects`].
    TooManyRejects {
        /// The property's name.
        name: String,
        /// The seed the run used.
        seed: u64,
        /// Cases that passed before the budget ran out.
        passed: u32,
        /// The rejecter that exhausted the budget.
        reason: String,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(case) => write!(
                f,
                "property `{}` failed after {} passing case(s): {}\n\
                 minimal failing input: {}\n\
                 choice sequence (hex, for Choices::from_hex): {}\n\
                 seed: {:#018x} ({} shrink(s)); rerun the same cases with {}={:#x}",
                case.name,
                case.passed,
                case.message,
                case.minimal,
                if case.choices_hex.is_empty() {
                    "(empty)"
                } else {
                    &case.choices_hex
                },
                case.seed,
                case.shrinks,
                SEED_VARIABLE,
                case.seed,
            ),
            Self::TooManyRejects {
                name,
                seed,
                passed,
                reason,
            } => write!(
                f,
                "property `{name}` exhausted its reject budget after {passed} passing case(s); \
                 the last rejection was: {reason} (seed {seed:#018x})"
            ),
        }
    }
}

impl std::error::Error for Failure {}

/// Runs one property under a [`Config`] and a deterministic seed.
#[derive(Debug, Clone)]
pub struct Runner {
    config: Config,
    name: String,
    seed: u64,
}

enum Outcome {
    Pass,
    Reject(String),
    Fail(String),
    Invalid(Invalid),
}

impl Runner {
    /// A runner for the property named `name`, seeded from
    /// `PURRDF_PROP_SEED` when it is set and from `seed_for(name)`
    /// ([`seed_for`]) otherwise.
    pub fn new(config: Config, name: &str) -> Self {
        Self::with_seed(config, name, seed_from_env(name))
    }

    /// A runner with an explicit seed, ignoring the environment.
    pub fn with_seed(config: Config, name: &str, seed: u64) -> Self {
        Self {
            config,
            name: name.to_owned(),
            seed,
        }
    }

    /// The seed the runner draws its cases from.
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Run `test` over `config.cases` passing inputs from `strategy`. The
    /// first failure — a returned [`TestCaseError::Fail`] or a panic — is
    /// shrunk and reported.
    pub fn run<S, F>(&self, strategy: &S, test: F) -> Result<RunSummary, Failure>
    where
        S: Strategy + ?Sized,
        F: Fn(S::Value) -> Result<(), TestCaseError>,
    {
        let mut stream = Xoshiro256::from_seed(self.seed);
        let mut rejects_left = self.config.max_rejects;
        let mut passed = 0u32;
        while passed < self.config.cases {
            let mut choices = Choices::random_with_budget(stream.next_u64(), rejects_left);
            let outcome = evaluate(strategy, &test, &mut choices);
            rejects_left = choices.rejects_left();
            let reason = match outcome {
                Outcome::Pass => {
                    passed += 1;
                    continue;
                }
                Outcome::Fail(message) => {
                    return Err(self.shrink(strategy, &test, choices.values(), message, passed));
                }
                Outcome::Invalid(Invalid::RejectLimit) => choices
                    .take_reject_reason()
                    .unwrap_or_else(|| "a generator rejected its candidates".to_owned()),
                Outcome::Invalid(Invalid::Overrun) => Invalid::Overrun.to_string(),
                Outcome::Reject(reason) => reason,
            };
            if rejects_left == 0 {
                return Err(Failure::TooManyRejects {
                    name: self.name.clone(),
                    seed: self.seed,
                    passed,
                    reason,
                });
            }
            rejects_left -= 1;
        }
        Ok(RunSummary {
            cases: passed,
            rejects: self.config.max_rejects - rejects_left,
        })
    }

    fn shrink<S, F>(
        &self,
        strategy: &S,
        test: &F,
        values: Vec<u64>,
        message: String,
        passed: u32,
    ) -> Failure
    where
        S: Strategy + ?Sized,
        F: Fn(S::Value) -> Result<(), TestCaseError>,
    {
        let mut shrinker = Shrinker {
            strategy,
            test,
            best: values,
            message,
            budget: self.config.max_shrink_iters,
            shrinks: 0,
        };
        shrinker.run();
        let mut replay = Choices::replay_values(shrinker.best.clone(), Choices::replay_budget());
        let minimal = match panic::catch_unwind(AssertUnwindSafe(|| strategy.generate(&mut replay)))
        {
            Ok(Ok(value)) => format!("{value:?}"),
            Ok(Err(invalid)) => format!("(the shrunk sequence no longer generates: {invalid})"),
            Err(payload) => format!("(generation panicked: {})", panic_message(payload.as_ref())),
        };
        Failure::Failed(Box::new(FailedCase {
            name: self.name.clone(),
            seed: self.seed,
            passed,
            message: shrinker.message,
            minimal,
            choices_hex: replay.to_hex(),
            shrinks: shrinker.shrinks,
        }))
    }
}

/// Run `test` over `strategy` under `config`, seeded for `name`, and panic
/// with the shrunk report if it fails. This is what [`crate::prop_test!`]
/// expands to.
pub fn run_test<S, F>(config: &Config, name: &str, strategy: &S, test: F)
where
    S: Strategy + ?Sized,
    F: Fn(S::Value) -> Result<(), TestCaseError>,
{
    if let Err(failure) = Runner::new(config.clone(), name).run(strategy, test) {
        panic!("{failure}");
    }
}

/// The value `strategy` generates from the hexadecimal choice sequence `hex`
/// — how a stored counterexample is replayed.
pub fn replay<S: Strategy + ?Sized>(strategy: &S, hex: &str) -> S::Value {
    let mut choices = Choices::from_hex(hex).unwrap_or_else(|error| panic!("{error}"));
    strategy.generate(&mut choices).unwrap_or_else(|invalid| {
        panic!("the choice sequence does not generate a value: {invalid}")
    })
}

fn evaluate<S, F>(strategy: &S, test: &F, choices: &mut Choices) -> Outcome
where
    S: Strategy + ?Sized,
    F: Fn(S::Value) -> Result<(), TestCaseError>,
{
    let result = panic::catch_unwind(AssertUnwindSafe(|| match strategy.generate(choices) {
        Err(invalid) => Outcome::Invalid(invalid),
        Ok(value) => match test(value) {
            Ok(()) => Outcome::Pass,
            Err(TestCaseError::Fail(message)) => Outcome::Fail(message),
            Err(TestCaseError::Reject(reason)) => Outcome::Reject(reason),
        },
    }));
    result.unwrap_or_else(|payload| {
        Outcome::Fail(format!("panicked: {}", panic_message(payload.as_ref())))
    })
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic with a non-string payload".to_owned()
    }
}

/// Shortlex order: a shorter sequence is simpler; between equal lengths the
/// lexicographically smaller one is.
fn simpler(candidate: &[u64], best: &[u64]) -> bool {
    match candidate.len().cmp(&best.len()) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => candidate < best,
    }
}

/// Minimises a failing choice sequence by replaying edited copies of it.
struct Shrinker<'a, S: ?Sized, F> {
    strategy: &'a S,
    test: &'a F,
    best: Vec<u64>,
    message: String,
    budget: u32,
    shrinks: u32,
}

impl<S, F> Shrinker<'_, S, F>
where
    S: Strategy + ?Sized,
    F: Fn(S::Value) -> Result<(), TestCaseError>,
{
    /// Replay `candidate`; keep what it actually drew if that still fails and
    /// is simpler than the best so far.
    fn consider(&mut self, candidate: Vec<u64>) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        let mut choices = Choices::replay_values(candidate, Choices::replay_budget());
        if let Outcome::Fail(message) = evaluate(self.strategy, self.test, &mut choices) {
            let drawn = choices.values();
            if simpler(&drawn, &self.best) {
                self.best = drawn;
                self.message = message;
                self.shrinks += 1;
                return true;
            }
        }
        false
    }

    fn run(&mut self) {
        loop {
            let before = self.best.clone();
            self.delete_spans();
            self.lower_and_delete();
            self.zero_spans();
            self.minimise_each();
            self.redistribute();
            self.sort_adjacent();
            if self.best == before || self.budget == 0 {
                return;
            }
        }
    }

    /// Remove runs of 8, 4, 2 and 1 choices, from the end backwards.
    fn delete_spans(&mut self) {
        for width in [8, 4, 2, 1] {
            let mut start = self.best.len().saturating_sub(width);
            loop {
                if start + width <= self.best.len() {
                    let mut candidate = self.best.clone();
                    candidate.drain(start..start + width);
                    self.consider(candidate);
                }
                if start == 0 || self.budget == 0 {
                    break;
                }
                start -= 1;
            }
        }
    }

    /// Lower a choice by one and delete a nearby later choice with it: the
    /// edit that shortens a collection whose length was drawn up front (a
    /// `prop_flat_map` from a length to a vector of exactly that length),
    /// which neither edit makes alone.
    fn lower_and_delete(&mut self) {
        const REACH: usize = 16;
        let mut index = 0;
        while index < self.best.len() && self.budget > 0 {
            if self.best[index] > 0 {
                for later in index + 1..(index + 1 + REACH).min(self.best.len()) {
                    let mut candidate = self.best.clone();
                    candidate[index] -= 1;
                    candidate.remove(later);
                    if self.consider(candidate) || self.budget == 0 {
                        break;
                    }
                }
            }
            index += 1;
        }
    }

    /// Zero runs of 8, 4 and 2 choices that are not already zero.
    fn zero_spans(&mut self) {
        for width in [8, 4, 2] {
            let mut start = 0;
            while start + width <= self.best.len() && self.budget > 0 {
                if self.best[start..start + width]
                    .iter()
                    .any(|&value| value != 0)
                {
                    let mut candidate = self.best.clone();
                    candidate[start..start + width].fill(0);
                    self.consider(candidate);
                }
                start += 1;
            }
        }
    }

    /// Lower each choice as far as it will go: zero first, then a binary
    /// search between the highest value known to pass and the lowest known to
    /// fail, then one below the result.
    fn minimise_each(&mut self) {
        let mut index = 0;
        while index < self.best.len() && self.budget > 0 {
            self.minimise(index);
            index += 1;
        }
    }

    fn minimise(&mut self, index: usize) {
        let with = |best: &[u64], value: u64| {
            let mut candidate = best.to_vec();
            candidate[index] = value;
            candidate
        };
        let current = self.best[index];
        if current == 0 || self.consider(with(&self.best, 0)) {
            return;
        }
        let mut low = 0u64;
        let mut high = current;
        while high - low > 1 && self.budget > 0 {
            let middle = low + (high - low) / 2;
            if self.consider(with(&self.best, middle)) {
                if index >= self.best.len() {
                    return;
                }
                high = self.best[index].min(middle);
            } else {
                low = middle;
            }
        }
    }

    /// Move as much as possible of a choice's value onto a nearby later one.
    /// The total is kept, so a property over a sum still fails, while the
    /// earlier choice — and with it the whole sequence, lexicographically —
    /// gets smaller; a zeroed choice is then deletable.
    fn redistribute(&mut self) {
        const REACH: usize = 16;
        let mut index = 0;
        while index < self.best.len() && self.budget > 0 {
            let mut later = index + 1;
            while later < (index + 1 + REACH).min(self.best.len())
                && self.best[index] > 0
                && self.budget > 0
            {
                let moved = |best: &[u64], amount: u64| {
                    let mut candidate = best.to_vec();
                    candidate[index] -= amount;
                    candidate[later] = candidate[later].saturating_add(amount);
                    candidate
                };
                // The whole value first, then halves of it down to one.
                let mut amount = self.best[index];
                while amount > 0 && self.budget > 0 && !self.consider(moved(&self.best, amount)) {
                    amount /= 2;
                }
                if later >= self.best.len() || index >= self.best.len() {
                    return;
                }
                later += 1;
            }
            index += 1;
        }
    }

    /// Swap adjacent choices that are out of order.
    fn sort_adjacent(&mut self) {
        let mut index = 0;
        while index + 1 < self.best.len() && self.budget > 0 {
            if self.best[index] > self.best[index + 1] {
                let mut candidate = self.best.clone();
                candidate.swap(index, index + 1);
                self.consider(candidate);
            }
            index += 1;
        }
    }
}

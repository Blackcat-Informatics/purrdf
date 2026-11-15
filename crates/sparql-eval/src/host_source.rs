// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The host's clock and entropy, read through a source the embedder installs.
//!
//! Evaluation samples two inputs it cannot compute: the wall clock (`NOW()` and a
//! [`WallDeadline`](crate::WallDeadline)) and an entropy seed (`RAND()`, `UUID()`,
//! `STRUUID()`). This crate imports neither from a host. An embedder that has them
//! implements [`HostSource`] and calls [`install`] once, before its first evaluation.
//!
//! With no source installed, a native build reads the operating system, as it always
//! has. A `wasm32-unknown-unknown` build has no operating system to read, so a request
//! that leaves an input to the host is refused with [`HostSourceMissing`], which names
//! the input and how to supply it. No value is ever substituted for a missing one.
//!
//! An installed source is the one every target reads, so an embedder that controls its
//! clock and entropy controls them on a native build too.

use std::sync::OnceLock;

/// A host's wall clock and entropy.
///
/// Both methods are called on the evaluating thread, once per sampled input.
pub trait HostSource: Send + Sync + core::fmt::Debug {
    /// Milliseconds since the Unix epoch on the host's wall clock.
    fn unix_millis(&self) -> f64;

    /// Sixty-four fresh bits to seed one execution's random stream.
    fn entropy_seed(&self) -> u64;
}

/// The host input a request left to a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HostInput {
    /// The wall clock: the default query time and a wall deadline.
    Clock,
    /// The entropy seed of the random stream.
    Entropy,
}

impl HostInput {
    /// The stable name of this input.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Entropy => "entropy",
        }
    }
}

/// A request left an input to the host, and this build has no source to read it from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HostSourceMissing {
    input: HostInput,
}

impl HostSourceMissing {
    /// The refusal of a read of `input`.
    #[must_use]
    pub const fn new(input: HostInput) -> Self {
        Self { input }
    }

    /// The input that could not be read.
    #[must_use]
    pub const fn input(&self) -> HostInput {
        self.input
    }
}

impl core::fmt::Display for HostSourceMissing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let remedy = match self.input {
            HostInput::Clock => "supply the query time (`QueryOptions::with_now`)",
            HostInput::Entropy => "supply the entropy seed (`QueryOptions::with_entropy_seed`)",
        };
        write!(
            f,
            "no host {} source is installed: {remedy}, or install a source with \
             `purrdf_sparql_eval::host_source::install`",
            self.input.name()
        )
    }
}

impl std::error::Error for HostSourceMissing {}

/// [`install`] was called when a source was already installed. The first source stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HostSourceInstalled;

impl core::fmt::Display for HostSourceInstalled {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("a host source is already installed; it is installed once and never replaced")
    }
}

impl std::error::Error for HostSourceInstalled {}

/// The installed source. Written once: an evaluation under way never sees it change.
static INSTALLED: OnceLock<&'static dyn HostSource> = OnceLock::new();

/// Install the source every later evaluation reads.
///
/// # Errors
///
/// [`HostSourceInstalled`] when a source is already installed; that source is kept.
pub fn install(source: &'static dyn HostSource) -> Result<(), HostSourceInstalled> {
    INSTALLED.set(source).map_err(|_| HostSourceInstalled)
}

/// The installed source, if any.
#[must_use]
pub fn installed() -> Option<&'static dyn HostSource> {
    INSTALLED.get().copied()
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original decoder causes and native profile/invariant refusals.

use super::PlanError;
use crate::{Iri, Term};

/// The native reason carried by [`super::FusionError::MalformedProfile`].
#[derive(Debug)]
#[non_exhaustive]
pub enum ProfileRefusal {
    /// An original canonical-reader or IRI-parser refusal.
    Decode(PlanError),
    /// The encoded profile belongs to another version.
    Version {
        /// Encoded version.
        found: u16,
        /// Version understood by this reader.
        expected: u16,
    },
    /// No decay rule has the encoded discriminator.
    DecayTag {
        /// Encoded discriminator.
        found: u8,
    },
    /// The profile names the same stratum twice.
    DuplicateStratum {
        /// The original stratum owner.
        stratum: Iri,
    },
    /// The canonical stratum map is not ascending.
    StratumOrder {
        /// Previous stratum.
        previous: Iri,
        /// Current stratum.
        current: Iri,
    },
    /// The tie-break discriminator is unknown.
    TieBreakTag {
        /// Encoded discriminator.
        found: u8,
        /// Accepted discriminator.
        expected: u8,
    },
    /// A stratum count cannot be represented by the wire count.
    StrataCount {
        /// Original count.
        count: usize,
        /// Largest representable count.
        limit: u32,
        /// Original integer-conversion refusal.
        source: core::num::TryFromIntError,
    },
    /// The declared contribution maximum disagrees with the stratum count.
    Contributions {
        /// Encoded maximum.
        found: u32,
        /// Actual number of strata.
        strata: u32,
    },
    /// The rounding discriminator is unknown.
    RoundingTag {
        /// Encoded discriminator.
        found: u8,
        /// Accepted discriminator.
        expected: u8,
    },
    /// The item-encoding discriminator is unknown.
    ItemEncodingTag {
        /// Encoded discriminator.
        found: u8,
        /// Accepted discriminator.
        expected: u8,
    },
    /// A selected candidate is absent from the frontier.
    CandidateVanished {
        /// Original candidate owner.
        item: Term,
    },
    /// A frontier candidate is absent from its selection index.
    CandidateUnindexed {
        /// Original candidate owner.
        item: Term,
    },
    /// A stream contributes only after the candidate was certified.
    LateContribution {
        /// Original stream index.
        stream: usize,
        /// Original candidate owner.
        item: Term,
    },
}

purrdf_lex::variant_from!(ProfileRefusal { Decode(PlanError) });

impl ProfileRefusal {
    pub(super) fn boxed(source: impl Into<Self>) -> Box<Self> {
        Box::new(source.into())
    }
}

impl purrdf_hash::error::StructuredError for ProfileRefusal {
    purrdf_hash::error_visit! {
        Decode(source) => "fusion-profile-decode" { "source" => Cause(source) },
        Version { found, expected } => "fusion-profile-version" {
            "found" => Unsigned(u128::from(*found)),
            "expected" => Unsigned(u128::from(*expected)),
        },
        DecayTag { found } => "fusion-profile-decay-tag" { "found" => Unsigned(u128::from(*found)) },
        DuplicateStratum { stratum } => "fusion-profile-duplicate-stratum" { "stratum" => Iri(stratum.as_str()) },
        StratumOrder { previous, current } => "fusion-profile-stratum-order" { "previous" => Iri(previous.as_str()), "current" => Iri(current.as_str()) },
        TieBreakTag { found, expected } => "fusion-profile-tie-break-tag" {
            "found" => Unsigned(u128::from(*found)),
            "expected" => Unsigned(u128::from(*expected)),
        },
        StrataCount { count, limit, source } => "fusion-profile-strata-count" {
            "count" => Unsigned(*count as u128),
            "limit" => Unsigned(u128::from(*limit)),
            "source" => Cause(source),
        },
        Contributions { found, strata } => "fusion-profile-contributions" {
            "found" => Unsigned(u128::from(*found)),
            "strata" => Unsigned(u128::from(*strata)),
        },
        RoundingTag { found, expected } => "fusion-profile-rounding-tag" {
            "found" => Unsigned(u128::from(*found)),
            "expected" => Unsigned(u128::from(*expected)),
        },
        ItemEncodingTag { found, expected } => "fusion-profile-item-encoding-tag" {
            "found" => Unsigned(u128::from(*found)),
            "expected" => Unsigned(u128::from(*expected)),
        },
        CandidateVanished { item } => "fusion-profile-candidate-vanished" { "item" => Record(item) },
        CandidateUnindexed { item } => "fusion-profile-candidate-unindexed" { "item" => Record(item) },
        LateContribution { stream, item } => "fusion-profile-late-contribution" {
            "stream" => Unsigned(*stream as u128),
            "item" => Record(item),
        },
    }
}

impl core::fmt::Display for ProfileRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(source) => source.fmt(f),
            Self::Version { found, expected } => write!(
                f,
                "unsupported fusion profile version {found}; this build writes {expected}"
            ),
            Self::DecayTag { found } => write!(f, "unknown decay rule tag {found}"),
            Self::DuplicateStratum { .. } => f.write_str("fusion profile repeats a stratum"),
            Self::StratumOrder { .. } => {
                f.write_str("fusion profile strata must be ordered ascending")
            }
            Self::TieBreakTag { found, .. } => write!(f, "unknown tie-break tag {found}"),
            Self::StrataCount { .. } => f.write_str("too many strata"),
            Self::Contributions { found, strata } => write!(
                f,
                "profile states a contribution maximum of {found} over {strata} strata; a candidate may surface at most once per stratum"
            ),
            Self::RoundingTag { found, .. } => write!(f, "unknown rounding tag {found}"),
            Self::ItemEncodingTag { found, .. } => write!(f, "unknown item-encoding tag {found}"),
            Self::CandidateVanished { .. } => {
                f.write_str("selected candidate vanished from the frontier")
            }
            Self::CandidateUnindexed { item } => write!(
                f,
                "frontier candidate {} is missing from the selection index",
                item.as_str()
            ),
            Self::LateContribution { stream, item } => write!(
                f,
                "stream {stream} named candidate {} after it certified without having contributed to it, which certification forbids",
                item.as_str()
            ),
        }
    }
}

impl std::error::Error for ProfileRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            Self::StrataCount { source, .. } => Some(source),
            _ => None,
        }
    }
}

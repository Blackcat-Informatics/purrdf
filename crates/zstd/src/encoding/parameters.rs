#![allow(
    clippy::return_self_not_must_use,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Fine-grained compression parameters — the drop-in equivalent of C
//! zstd's advanced `ZSTD_CCtx_setParameter` surface (#27).
//!
//! [`CompressionLevel`](crate::encoding::CompressionLevel) selects a
//! whole tuning preset in one knob. This module exposes the individual
//! knobs underneath it — window/hash/chain/search logs, the match
//! strategy, and the long-distance-matching (LDM) block — so callers
//! can override a level's defaults for domain-specific tuning.
//!
//! # Builder
//!
//! [`CompressionParameters`] is built through
//! [`CompressionParameters::builder`], which takes an explicit base
//! [`CompressionLevel`](crate::encoding::CompressionLevel) (there is no
//! implicit default). Every knob left unset inherits that base level's
//! resolved value, so a builder that overrides nothing reproduces plain
//! level-based compression byte-for-byte.
//!
//! ```rust
//! use purrdf_zstd::encoding::{CompressionLevel, CompressionParameters, Strategy};
//!
//! let params = CompressionParameters::builder(CompressionLevel::Level(19))
//!     .window_log(22)
//!     .strategy(Strategy::Btultra2)
//!     .enable_long_distance_matching(true)
//!     .build()
//!     .expect("parameters within bounds");
//! ```
//!
//! # Bounds
//!
//! Every knob has an inclusive `[lower, upper]` range, queryable via
//! [`CParameter::bounds`] (the analogue of `ZSTD_cParam_getBounds`).
//! [`CompressionParametersBuilder::build`] validates each set knob and
//! returns [`ParameterError::OutOfBounds`] for the first violation.
//!
//! # Long-distance matching (LDM)
//!
//! LDM is **off at every [`CompressionLevel`](crate::encoding::CompressionLevel)
//! preset**, matching upstream `libzstd.so.1` where `ZSTD_compress(..., level)`
//! never enables LDM — even at level 22. It is activated either by
//! [`CompressionParametersBuilder::enable_long_distance_matching`] or by any of
//! the `ldm_*` setters, which each imply `enable_long_distance_matching(true)`.
//! When enabled, the LDM producer attaches to the optimal (`btopt` / `btultra`
//! / `btultra2`) match-finder; pair it with an optimal [`Strategy`] (or a level
//! ≥ 16) for it to take effect.

use crate::encoding::CompressionLevel;

/// Match-finder strategy — the drop-in equivalent of C zstd's
/// `ZSTD_strategy` enum (`ZSTD_fast` … `ZSTD_btultra2`). The numeric
/// ordinals match upstream (`fast = 1` … `btultra2 = 9`), so
/// [`Strategy::ordinal`] / [`Strategy::from_ordinal`] round-trip with
/// the C `ZSTD_c_strategy` parameter value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strategy {
    /// `ZSTD_fast` (1) — single-table fast finder.
    Fast,
    /// `ZSTD_dfast` (2) — two parallel hash tables.
    Dfast,
    /// `ZSTD_greedy` (3) — commit the first acceptable match, no lookahead.
    Greedy,
    /// `ZSTD_lazy` (4) — one-position lazy lookahead.
    Lazy,
    /// `ZSTD_lazy2` (5) — two-position lazy lookahead.
    Lazy2,
    /// `ZSTD_btlazy2` (6) — binary-tree-assisted lazy2.
    Btlazy2,
    /// `ZSTD_btopt` (7) — optimal parser, no ultra refinements.
    Btopt,
    /// `ZSTD_btultra` (8) — optimal parser with refined price tables.
    Btultra,
    /// `ZSTD_btultra2` (9) — optimal parser with two-pass dynamic stats.
    Btultra2,
}

impl Strategy {
    /// Upstream `ZSTD_strategy` ordinal (`fast = 1` … `btultra2 = 9`).
    pub const fn ordinal(self) -> u32 {
        match self {
            Self::Fast => 1,
            Self::Dfast => 2,
            Self::Greedy => 3,
            Self::Lazy => 4,
            Self::Lazy2 => 5,
            Self::Btlazy2 => 6,
            Self::Btopt => 7,
            Self::Btultra => 8,
            Self::Btultra2 => 9,
        }
    }

    /// Construct from an upstream `ZSTD_strategy` ordinal. Returns
    /// `None` outside `1..=9`.
    pub const fn from_ordinal(ordinal: u32) -> Option<Self> {
        Some(match ordinal {
            1 => Self::Fast,
            2 => Self::Dfast,
            3 => Self::Greedy,
            4 => Self::Lazy,
            5 => Self::Lazy2,
            6 => Self::Btlazy2,
            7 => Self::Btopt,
            8 => Self::Btultra,
            9 => Self::Btultra2,
            _ => return None,
        })
    }

    /// Internal runtime strategy tag.
    pub(crate) const fn tag(self) -> crate::encoding::strategy::StrategyTag {
        use crate::encoding::strategy::StrategyTag;
        match self {
            Self::Fast => StrategyTag::Fast,
            Self::Dfast => StrategyTag::Dfast,
            Self::Greedy => StrategyTag::Greedy,
            // Lazy / Lazy2 ride the runtime `Lazy` tag (the lazy lookahead
            // depth carries the variance, see `lazy_depth`). `Btlazy2`
            // keeps its own tag: `Lazy` resolves to the Row finder, while
            // btlazy2 is a binary-tree search and must stay on the
            // HashChain/BT storage.
            Self::Lazy | Self::Lazy2 => StrategyTag::Lazy,
            Self::Btlazy2 => StrategyTag::Btlazy2,
            Self::Btopt => StrategyTag::BtOpt,
            Self::Btultra => StrategyTag::BtUltra,
            Self::Btultra2 => StrategyTag::BtUltra2,
        }
    }

    /// Lazy lookahead depth for the greedy/lazy band (0/1/2). `Optimal`
    /// strategies report 2 (the depth their hash-chain seed walk runs at).
    pub(crate) const fn lazy_depth(self) -> u8 {
        match self {
            Self::Fast | Self::Dfast | Self::Greedy => 0,
            Self::Lazy => 1,
            _ => 2,
        }
    }
}

/// One tunable compression parameter — the analogue of a C zstd
/// `ZSTD_cParameter`. Used to query bounds via [`CParameter::bounds`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CParameter {
    /// Maximum back-reference distance, `log2`. C `ZSTD_c_windowLog`.
    WindowLog,
    /// Match-finder hash table size, `log2`. C `ZSTD_c_hashLog`.
    HashLog,
    /// Match-finder chain table size, `log2`. C `ZSTD_c_chainLog`.
    ChainLog,
    /// Number of search attempts, `log2`. C `ZSTD_c_searchLog`.
    SearchLog,
    /// Minimum match length in bytes. C `ZSTD_c_minMatch`.
    MinMatch,
    /// "Good enough" match length that ends the search. C `ZSTD_c_targetLength`.
    TargetLength,
    /// Match-finder [`Strategy`] (1..=9). C `ZSTD_c_strategy`.
    Strategy,
    /// LDM enable flag (0/1). C `ZSTD_c_enableLongDistanceMatching`.
    EnableLongDistanceMatching,
    /// LDM hash table size, `log2`. C `ZSTD_c_ldmHashLog`.
    LdmHashLog,
    /// LDM minimum match length in bytes. C `ZSTD_c_ldmMinMatch`.
    LdmMinMatch,
    /// LDM bucket size, `log2`. C `ZSTD_c_ldmBucketSizeLog`.
    LdmBucketSizeLog,
    /// LDM hash-insertion rate, `log2`. C `ZSTD_c_ldmHashRateLog`.
    LdmHashRateLog,
}

/// Inclusive `[lower_bound, upper_bound]` range for a [`CParameter`],
/// the drop-in equivalent of C zstd's `ZSTD_bounds`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Smallest accepted value (inclusive).
    pub lower_bound: i64,
    /// Largest accepted value (inclusive).
    pub upper_bound: i64,
}

impl Bounds {
    /// Whether `value` falls within `[lower_bound, upper_bound]`.
    pub const fn contains(&self, value: i64) -> bool {
        value >= self.lower_bound && value <= self.upper_bound
    }
}

impl CParameter {
    /// Inclusive value bounds for this parameter, mirroring
    /// `ZSTD_cParam_getBounds`. Window/hash/chain logs cap at 30 (the
    /// encoder's match-finder ceiling) rather than the 31 C allows on
    /// 64-bit, because the back-reference history is indexed with `u32`
    /// positions over a `2 * window` eviction band.
    pub const fn bounds(self) -> Bounds {
        let (lower_bound, upper_bound) = match self {
            // ZSTD_WINDOWLOG_MIN .. encoder ceiling.
            Self::WindowLog => (10, 30),
            // ZSTD_HASHLOG_MIN .. ZSTD_HASHLOG_MAX.
            Self::HashLog => (6, 30),
            // ZSTD_CHAINLOG_MIN .. ZSTD_CHAINLOG_MAX (64-bit).
            Self::ChainLog => (6, 30),
            // ZSTD_SEARCHLOG_MIN .. ZSTD_SEARCHLOG_MAX (64-bit).
            Self::SearchLog => (1, 30),
            // ZSTD_MINMATCH_MIN .. ZSTD_MINMATCH_MAX.
            Self::MinMatch => (3, 7),
            // ZSTD_TARGETLENGTH_MIN .. ZSTD_TARGETLENGTH_MAX.
            Self::TargetLength => (0, 131_072),
            // ZSTD_fast .. ZSTD_btultra2.
            Self::Strategy => (1, 9),
            // Boolean flag.
            Self::EnableLongDistanceMatching => (0, 1),
            // ZSTD_LDM_HASHLOG_MIN .. ZSTD_LDM_HASHLOG_MAX.
            Self::LdmHashLog => (6, 30),
            // ZSTD_LDM_MINMATCH_MIN .. ZSTD_LDM_MINMATCH_MAX.
            Self::LdmMinMatch => (4, 4096),
            // ZSTD_LDM_BUCKETSIZELOG_MIN .. ZSTD_LDM_BUCKETSIZELOG_MAX.
            Self::LdmBucketSizeLog => (1, 8),
            // ZSTD_LDM_HASHRATELOG_MIN .. ZSTD_WINDOWLOG_MAX - ZSTD_HASHLOG_MIN.
            Self::LdmHashRateLog => (0, 24),
        };
        Bounds {
            lower_bound,
            upper_bound,
        }
    }
}

/// Error returned by [`CompressionParametersBuilder::build`] when a knob
/// is set outside its [`CParameter::bounds`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParameterError {
    /// A parameter was set to a value outside its inclusive bounds.
    OutOfBounds {
        /// Which parameter violated its range.
        parameter: CParameter,
        /// The rejected value.
        value: i64,
        /// The inclusive `[lower, upper]` range it had to fall within.
        bounds: Bounds,
    },
}

impl core::fmt::Display for ParameterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfBounds {
                parameter,
                value,
                bounds,
            } => write!(
                f,
                "compression parameter {parameter:?} = {value} out of bounds \
                 [{}, {}]",
                bounds.lower_bound, bounds.upper_bound
            ),
        }
    }
}

impl std::error::Error for ParameterError {}

/// LDM tuning overrides — every knob is `Option`, falling back to the
/// strategy-derived upstream zstd default (`LdmParams::adjust_for`) when unset.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LdmOverride {
    pub(crate) hash_log: Option<u32>,
    pub(crate) min_match: Option<u32>,
    pub(crate) bucket_size_log: Option<u32>,
    pub(crate) hash_rate_log: Option<u32>,
}

/// Internal per-knob override set consumed by the match-generator's
/// `reset` path. Every field left `None` inherits the base level's
/// resolved value, so the default path is byte-identical to level-based
/// compression.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ParamOverrides {
    pub(crate) window_log: Option<u8>,
    pub(crate) hash_log: Option<u32>,
    pub(crate) chain_log: Option<u32>,
    pub(crate) search_log: Option<u32>,
    pub(crate) min_match: Option<u32>,
    pub(crate) target_length: Option<u32>,
    pub(crate) strategy: Option<Strategy>,
    /// `Some` when `enable_long_distance_matching(true)` was set; carries
    /// the (possibly empty) LDM knob overrides.
    pub(crate) ldm: Option<LdmOverride>,
}

impl ParamOverrides {
    /// Whether any knob overrides the base level. An all-`None`
    /// override is a no-op the `reset` path can skip entirely, keeping
    /// the default level-based geometry byte-identical.
    pub(crate) fn is_empty(&self) -> bool {
        self.window_log.is_none()
            && self.hash_log.is_none()
            && self.chain_log.is_none()
            && self.search_log.is_none()
            && self.min_match.is_none()
            && self.target_length.is_none()
            && self.strategy.is_none()
            && self.ldm.is_none()
    }
}

/// Fully-resolved fine-grained compression parameters. Build through
/// [`CompressionParameters::builder`]; pass to
/// [`FrameCompressor::set_parameters`](crate::encoding::FrameCompressor::set_parameters)
/// or [`compress_with_parameters`](crate::encoding::compress_with_parameters).
///
/// Wraps a base [`CompressionLevel`](crate::encoding::CompressionLevel)
/// plus the set of knobs that override it. A parameter set that
/// overrides nothing is equivalent to compressing at its base level.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CompressionParameters {
    level: CompressionLevel,
    overrides: ParamOverrides,
}

impl CompressionParameters {
    /// Start a builder from a base compression level. Knobs left unset
    /// inherit that level's resolved defaults.
    pub fn builder(level: CompressionLevel) -> CompressionParametersBuilder {
        CompressionParametersBuilder {
            level,
            window_log: None,
            hash_log: None,
            chain_log: None,
            search_log: None,
            min_match: None,
            target_length: None,
            strategy: None,
            enable_ldm: false,
            ldm: LdmOverride::default(),
        }
    }

    /// The base compression level these parameters override.
    pub fn level(&self) -> CompressionLevel {
        self.level
    }

    /// Whether long-distance matching is enabled.
    pub fn long_distance_matching_enabled(&self) -> bool {
        self.overrides.ldm.is_some()
    }

    pub(crate) fn overrides(&self) -> ParamOverrides {
        self.overrides
    }
}

/// Builder for [`CompressionParameters`]. Each setter records one knob;
/// [`Self::build`] validates them against [`CParameter::bounds`].
#[derive(Copy, Clone, Debug)]
pub struct CompressionParametersBuilder {
    level: CompressionLevel,
    window_log: Option<u32>,
    hash_log: Option<u32>,
    chain_log: Option<u32>,
    search_log: Option<u32>,
    min_match: Option<u32>,
    target_length: Option<u32>,
    strategy: Option<Strategy>,
    enable_ldm: bool,
    ldm: LdmOverride,
}

impl CompressionParametersBuilder {
    /// Override the maximum back-reference distance (`log2`). C
    /// `ZSTD_c_windowLog`.
    pub fn window_log(mut self, value: u32) -> Self {
        self.window_log = Some(value);
        self
    }

    /// Override the match-finder hash table size (`log2`). C `ZSTD_c_hashLog`.
    pub fn hash_log(mut self, value: u32) -> Self {
        self.hash_log = Some(value);
        self
    }

    /// Override the match-finder chain table size (`log2`). C `ZSTD_c_chainLog`.
    pub fn chain_log(mut self, value: u32) -> Self {
        self.chain_log = Some(value);
        self
    }

    /// Override the search-attempts count (`log2`). C `ZSTD_c_searchLog`.
    pub fn search_log(mut self, value: u32) -> Self {
        self.search_log = Some(value);
        self
    }

    /// Override the minimum match length in bytes. C `ZSTD_c_minMatch`.
    pub fn min_match(mut self, value: u32) -> Self {
        self.min_match = Some(value);
        self
    }

    /// Override the "good enough" target match length. C `ZSTD_c_targetLength`.
    pub fn target_length(mut self, value: u32) -> Self {
        self.target_length = Some(value);
        self
    }

    /// Override the match-finder [`Strategy`]. C `ZSTD_c_strategy`.
    pub fn strategy(mut self, value: Strategy) -> Self {
        self.strategy = Some(value);
        self
    }

    /// Enable or disable long-distance matching. C
    /// `ZSTD_c_enableLongDistanceMatching`. Off at every level preset.
    /// This is the explicit activation toggle; the `ldm_*` knob setters
    /// also enable LDM implicitly. The flag is plain last-write-wins, so
    /// a trailing `enable_long_distance_matching(false)` disables LDM even
    /// if an earlier `ldm_*` call set a knob (the knob is then ignored at
    /// [`build`](Self::build)).
    pub fn enable_long_distance_matching(mut self, enable: bool) -> Self {
        self.enable_ldm = enable;
        self
    }

    /// Override the LDM hash table size (`log2`). C `ZSTD_c_ldmHashLog`.
    /// Implies [`Self::enable_long_distance_matching(true)`](Self::enable_long_distance_matching).
    pub fn ldm_hash_log(mut self, value: u32) -> Self {
        self.enable_ldm = true;
        self.ldm.hash_log = Some(value);
        self
    }

    /// Override the LDM minimum match length. C `ZSTD_c_ldmMinMatch`.
    /// Implies [`Self::enable_long_distance_matching(true)`](Self::enable_long_distance_matching).
    pub fn ldm_min_match(mut self, value: u32) -> Self {
        self.enable_ldm = true;
        self.ldm.min_match = Some(value);
        self
    }

    /// Override the LDM bucket size (`log2`). C `ZSTD_c_ldmBucketSizeLog`.
    /// Implies [`Self::enable_long_distance_matching(true)`](Self::enable_long_distance_matching).
    pub fn ldm_bucket_size_log(mut self, value: u32) -> Self {
        self.enable_ldm = true;
        self.ldm.bucket_size_log = Some(value);
        self
    }

    /// Override the LDM hash-insertion rate (`log2`). C `ZSTD_c_ldmHashRateLog`.
    /// Implies [`Self::enable_long_distance_matching(true)`](Self::enable_long_distance_matching).
    pub fn ldm_hash_rate_log(mut self, value: u32) -> Self {
        self.enable_ldm = true;
        self.ldm.hash_rate_log = Some(value);
        self
    }

    /// Validate every set knob against [`CParameter::bounds`] and
    /// produce the resolved [`CompressionParameters`].
    ///
    /// # Errors
    ///
    /// Returns [`ParameterError::OutOfBounds`] for the first knob whose
    /// value falls outside its inclusive range.
    pub fn build(self) -> Result<CompressionParameters, ParameterError> {
        check(CParameter::WindowLog, self.window_log)?;
        check(CParameter::HashLog, self.hash_log)?;
        check(CParameter::ChainLog, self.chain_log)?;
        check(CParameter::SearchLog, self.search_log)?;
        check(CParameter::MinMatch, self.min_match)?;
        check(CParameter::TargetLength, self.target_length)?;
        if let Some(s) = self.strategy {
            check(CParameter::Strategy, Some(s.ordinal()))?;
        }
        let ldm = if self.enable_ldm {
            check(CParameter::LdmHashLog, self.ldm.hash_log)?;
            check(CParameter::LdmMinMatch, self.ldm.min_match)?;
            check(CParameter::LdmBucketSizeLog, self.ldm.bucket_size_log)?;
            check(CParameter::LdmHashRateLog, self.ldm.hash_rate_log)?;
            Some(self.ldm)
        } else {
            None
        };
        Ok(CompressionParameters {
            level: self.level,
            overrides: ParamOverrides {
                // `window_log` is bounds-checked at <= 30, so the cast is lossless.
                window_log: self.window_log.map(|v| v as u8),
                hash_log: self.hash_log,
                chain_log: self.chain_log,
                search_log: self.search_log,
                min_match: self.min_match,
                target_length: self.target_length,
                strategy: self.strategy,
                ldm,
            },
        })
    }
}

/// Validate one optional knob against its bounds.
fn check(parameter: CParameter, value: Option<u32>) -> Result<(), ParameterError> {
    if let Some(value) = value {
        let bounds = parameter.bounds();
        let value = i64::from(value);
        if !bounds.contains(value) {
            return Err(ParameterError::OutOfBounds {
                parameter,
                value,
                bounds,
            });
        }
    }
    Ok(())
}

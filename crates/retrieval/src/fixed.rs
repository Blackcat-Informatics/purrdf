// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The exact fixed-point base every fused score is computed in.
//!
//! Fusion reuses `purrdf-text`'s [`Fixed`] rather than defining a second
//! base-10 type: an `i128` read as a multiple of `10^-12`, with truncation
//! toward zero, checked operations, and a wide-intermediate multiply/divide that
//! refuses to report an overflow the answer does not have. Two fixed-point types
//! in one workspace would be two conventions for what a rounded reciprocal means,
//! so there is exactly one and it is re-exported here.
//!
//! The re-export exists so a caller building a fusion profile can name the weight
//! type without depending on `purrdf-text` directly; it is the same type the
//! crate root already re-exports, not a wrapper.

pub use purrdf_text::Fixed;

/// Sixty, the conventional reciprocal-rank smoothing constant, offered as a
/// value a caller may name — **never** as a default.
///
/// `K` is the rank at which an item's contribution is halved: `recip(K + K) =
/// recip(K)/2`. Sixty is the value the reciprocal-rank-fusion literature and the
/// operators this layer was extracted from converged on, and naming it here
/// spares a caller from re-deriving where it came from.
///
/// It is not a fallback and nothing substitutes it. [`FusionProfile::with_decay`]
/// takes `k` as a required argument, so a profile is impossible to build
/// without stating its own smoothing constant, and the value that governs a
/// fusion is always the one that profile carries — covered by the profile's
/// identity, so an answer names exactly which `K` produced it. A caller that
/// wants sixty passes this constant; a caller that wants anything else passes
/// that, and neither path reads a value nobody chose.
///
/// [`FusionProfile`]: crate::FusionProfile
/// [`FusionProfile::with_decay`]: crate::FusionProfile::with_decay
pub const RECIP_K: i128 = 60;

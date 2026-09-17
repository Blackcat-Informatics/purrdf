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

/// The default smoothing constant `K` of the reciprocal-rank decay.
///
/// `K` is the rank at which an item's contribution is halved: `recip(K + K) =
/// recip(K)/2`. Sixty is the conventional value chosen by the operators this
/// layer was extracted from, and it is a *default*, never a hardwired law — the
/// value that governs a fusion is the one the [`FusionProfile`] carries, and the
/// profile's identity covers it. A caller selects this constant deliberately or
/// supplies its own; nothing here silently substitutes it.
///
/// [`FusionProfile`]: crate::FusionProfile
pub const RECIP_K: i128 = 60;

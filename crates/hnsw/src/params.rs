// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Required configuration and the **fail-closed validation matrix**.
//!
//! HNSW's four numbers are not tuning knobs bolted onto a fixed algorithm; they *are*
//! the index identity. Two builds with different `M` are different indexes, and a query
//! answered under a different `ef_search` is a different question. So there is no
//! `Default`, no builder that fills in a "sensible" value, and no query-time override.
//! A caller states all four or builds nothing.
//!
//! Validation is fail-closed at construction: every relationship the build relies on is
//! checked before a single distance is computed, and a violation names the parameter and
//! the reason rather than surfacing later as a short or malformed graph.

use crate::error::{HnswError, Result};

/// The four required HNSW parameters.
///
/// Constructed only through [`Params::new`], which enforces the validation matrix; the
/// fields are private so no value can exist that skipped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Params {
    /// Neighbours retained per node above layer 0. At least 2.
    m: usize,
    /// Neighbours retained per node at layer 0. At least `m`.
    m0: usize,
    /// Beam width used while selecting neighbours during construction. At least `m`.
    ef_construction: usize,
    /// Beam width used while answering a query. At least 1.
    ef_search: usize,
}

impl Params {
    /// The only constructor, applying the full validation matrix.
    ///
    /// # Errors
    ///
    /// [`HnswError::InvalidParameter`] for any of:
    ///
    /// * `m < 2` — a node with fewer than two neighbours cannot form a navigable graph,
    ///   and the level formula's `floor(log2(m))` is undefined for `m < 2`;
    /// * `m0 < m` — layer 0 is the layer every query traverses, so it may not be *less*
    ///   connected than the layers above it;
    /// * `ef_construction < m` — the candidate beam must be able to hold at least the
    ///   neighbours it is asked to select, or the selection silently returns fewer than
    ///   `m` and the degree bound becomes a lie;
    /// * `ef_search < 1` — a beam of zero examines nothing and would answer every query
    ///   with an empty (but not empty-by-data) result.
    pub fn new(m: usize, m0: usize, ef_construction: usize, ef_search: usize) -> Result<Self> {
        if m < 2 {
            return Err(HnswError::InvalidParameter {
                name: "M",
                value: m.to_string(),
                reason: "must be at least 2",
            });
        }
        if m0 < m {
            return Err(HnswError::InvalidParameter {
                name: "M0",
                value: m0.to_string(),
                reason: "must be at least M",
            });
        }
        if ef_construction < m {
            return Err(HnswError::InvalidParameter {
                name: "ef_construction",
                value: ef_construction.to_string(),
                reason: "must be at least M",
            });
        }
        if ef_search < 1 {
            return Err(HnswError::InvalidParameter {
                name: "ef_search",
                value: ef_search.to_string(),
                reason: "must be at least 1",
            });
        }
        Ok(Self {
            m,
            m0,
            ef_construction,
            ef_search,
        })
    }

    /// `M`: the degree bound above layer 0.
    #[must_use]
    pub const fn m(self) -> usize {
        self.m
    }

    /// `M0`: the degree bound at layer 0.
    #[must_use]
    pub const fn m0(self) -> usize {
        self.m0
    }

    /// The construction beam width.
    #[must_use]
    pub const fn ef_construction(self) -> usize {
        self.ef_construction
    }

    /// The query beam width.
    #[must_use]
    pub const fn ef_search(self) -> usize {
        self.ef_search
    }

    /// The number of neighbours a node may retain at `layer`.
    ///
    /// Layer 0 uses `M0`; every layer above uses `M`.
    #[must_use]
    pub const fn degree_bound(self, layer: u32) -> usize {
        if layer == 0 { self.m0 } else { self.m }
    }

    /// Check this parameter set against a concrete matrix shape.
    ///
    /// # Errors
    ///
    /// [`HnswError::ParameterValidation`] if the matrix has zero rows or zero
    /// dimensions: an index over no vectors answers nothing, and an index of zero-width
    /// vectors ranks every candidate equally.
    /// [`HnswError::ArithmeticOverflow`] if the total element count does not fit the
    /// platform's `usize`.
    /// [`HnswError::AddressSpaceExceeded`] on a 32-bit target when the estimated
    /// footprint — vectors, norms, node records, and adjacency — exceeds the addressable
    /// range. This is a construction-time admission check rather than an allocation that
    /// fails halfway through a build.
    pub fn validate_against(&self, rows: usize, dims: usize) -> Result<()> {
        if rows == 0 {
            return Err(HnswError::ParameterValidation {
                description: "the matrix has zero rows".to_owned(),
            });
        }
        if dims == 0 {
            return Err(HnswError::ParameterValidation {
                description: "the matrix has zero dimensions".to_owned(),
            });
        }
        // Every product the build performs, checked once, here.
        let elements = rows
            .checked_mul(dims)
            .ok_or(HnswError::ArithmeticOverflow)?;
        let _ = elements
            .checked_mul(size_of::<f64>())
            .ok_or(HnswError::ArithmeticOverflow)?;

        #[cfg(target_pointer_width = "32")]
        {
            const ADDRESSABLE: u64 = u32::MAX as u64;
            let required = self.estimated_footprint(rows, dims)?;
            if required > ADDRESSABLE {
                return Err(HnswError::AddressSpaceExceeded {
                    required,
                    maximum: ADDRESSABLE,
                });
            }
        }
        Ok(())
    }

    /// A conservative upper estimate of the in-memory footprint in bytes.
    ///
    /// Used only by the 32-bit admission check. Vectors and norms are exact; adjacency is
    /// estimated with the expected node population per level (`n / m^l`) and the full
    /// degree bound, so the estimate is an over- rather than under-count.
    #[cfg(target_pointer_width = "32")]
    fn estimated_footprint(&self, rows: usize, dims: usize) -> Result<u64> {
        let vectors = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(u64::try_from(dims).ok()?))
            .and_then(|e| e.checked_mul(size_of::<f64>() as u64))
            .ok_or(HnswError::ArithmeticOverflow)?;
        let norms = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(size_of::<f64>() as u64))
            .ok_or(HnswError::ArithmeticOverflow)?;
        let node_records = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(24))
            .ok_or(HnswError::ArithmeticOverflow)?;

        let cap = u64::from(crate::level::level_cap(rows, self.m));
        let base = u64::try_from(self.m).map_err(|_| HnswError::ArithmeticOverflow)?;
        let mut population = u64::try_from(rows).map_err(|_| HnswError::ArithmeticOverflow)?;
        let mut edges: u64 = 0;
        for layer in 0..=cap {
            let degree = if layer == 0 {
                u64::try_from(self.m0)
            } else {
                u64::try_from(self.m)
            }
            .map_err(|_| HnswError::ArithmeticOverflow)?;
            edges = edges
                .checked_add(
                    population
                        .checked_mul(degree)
                        .ok_or(HnswError::ArithmeticOverflow)?,
                )
                .ok_or(HnswError::ArithmeticOverflow)?;
            population /= base.max(1);
        }
        // A `Ranked` edge is a `usize` row plus an `f64` distance.
        let edge_bytes = edges
            .checked_mul(size_of::<crate::Ranked>() as u64)
            .ok_or(HnswError::ArithmeticOverflow)?;

        vectors
            .checked_add(norms)
            .and_then(|t| t.checked_add(node_records))
            .and_then(|t| t.checked_add(edge_bytes))
            .ok_or(HnswError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_set_round_trips() {
        let params = Params::new(16, 32, 200, 64).expect("valid");
        assert_eq!(params.m(), 16);
        assert_eq!(params.m0(), 32);
        assert_eq!(params.ef_construction(), 200);
        assert_eq!(params.ef_search(), 64);
    }

    #[test]
    fn m_below_two_is_refused() {
        let error = Params::new(1, 16, 100, 64).expect_err("M = 1");
        assert!(matches!(
            error,
            HnswError::InvalidParameter { name: "M", .. }
        ));
    }

    #[test]
    fn m0_below_m_is_refused() {
        let error = Params::new(16, 8, 100, 64).expect_err("M0 < M");
        assert!(matches!(
            error,
            HnswError::InvalidParameter { name: "M0", .. }
        ));
    }

    #[test]
    fn ef_construction_below_m_is_refused() {
        let error = Params::new(16, 32, 8, 64).expect_err("ef_construction < M");
        assert!(matches!(
            error,
            HnswError::InvalidParameter {
                name: "ef_construction",
                ..
            }
        ));
    }

    #[test]
    fn ef_search_zero_is_refused() {
        let error = Params::new(16, 32, 100, 0).expect_err("ef_search = 0");
        assert!(matches!(
            error,
            HnswError::InvalidParameter {
                name: "ef_search",
                ..
            }
        ));
    }

    #[test]
    fn the_boundaries_are_admitted_not_merely_approached() {
        // Every comparison is an admission boundary, so the value *at* the bound must be
        // accepted and only the value past it refused. A `<` that should be `<=` passes a
        // suite that only tests the refusals.
        assert!(Params::new(2, 2, 2, 1).is_ok(), "the minimum set is valid");
        assert!(Params::new(16, 16, 16, 1).is_ok(), "M0 = M, ef_c = M");
    }

    #[test]
    fn degree_bound_splits_at_layer_zero() {
        let params = Params::new(16, 32, 100, 64).expect("valid");
        assert_eq!(params.degree_bound(0), 32);
        assert_eq!(params.degree_bound(1), 16);
        assert_eq!(params.degree_bound(9), 16);
    }

    #[test]
    fn zero_rows_and_zero_dimensions_are_refused() {
        let params = Params::new(16, 32, 100, 64).expect("valid");
        assert!(params.validate_against(0, 8).is_err());
        assert!(params.validate_against(8, 0).is_err());
        assert!(params.validate_against(8, 8).is_ok());
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
    /// * `ef_construction < m0` — the candidate beam must be able to hold at least the
    ///   neighbours it is asked to select, or the selection silently returns fewer than the
    ///   degree bound and the bound becomes a lie. Layer 0 is the binding case because its
    ///   bound is `m0`, which is never below `m`; a beam that clears `m0` clears `m` too.
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
        if ef_construction < m0 {
            return Err(HnswError::InvalidParameter {
                name: "ef_construction",
                value: ef_construction.to_string(),
                reason: "must be at least M0",
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
        self.admits_on_32_bit(rows, dims)?;
        Ok(())
    }

    /// Whether a `rows x dims` build would fit a 32-bit address space.
    ///
    /// This is the admission check `validate_against` applies when it is *itself* running on
    /// a 32-bit target, exposed so a 64-bit host can ask the same question before shipping
    /// an artifact to `wasm32`. The answer does not depend on the target this code was
    /// compiled for: it always describes a 32-bit layout.
    ///
    /// # Errors
    ///
    /// [`HnswError::AddressSpaceExceeded`] when the estimate exceeds the addressable range,
    /// [`HnswError::ArithmeticOverflow`] if the estimate itself does not fit `u64`.
    pub fn admits_on_32_bit(self, rows: usize, dims: usize) -> Result<()> {
        const ADDRESSABLE: u64 = u32::MAX as u64;
        let required = self.footprint_bytes_32(rows, dims)?;
        if required > ADDRESSABLE {
            return Err(HnswError::AddressSpaceExceeded {
                required,
                maximum: ADDRESSABLE,
            });
        }
        Ok(())
    }

    /// A conservative upper estimate, in bytes, of what this build costs a 32-bit target.
    ///
    /// Compiled on every target and stated in explicit 32-bit widths rather than in
    /// `size_of::<usize>()`, so the number means the same thing wherever it is computed and
    /// both sides of the admission check can be exercised on a 64-bit host. A check that no
    /// gate ever compiles is a refusal nobody has proven either way, and a size estimator is
    /// the worst place to leave that untested: an over-count rejects a build that would
    /// have fitted, and the failure looks exactly like correct strictness.
    ///
    /// Vectors and norms are exact. Adjacency is estimated with the expected node population
    /// per level (`n / m^l`) at the **full** degree bound, which the graph does not reach, so
    /// the total is an upper bound.
    ///
    /// # Errors
    ///
    /// [`HnswError::ArithmeticOverflow`] if any product leaves `u64`.
    pub fn footprint_bytes_32(self, rows: usize, dims: usize) -> Result<u64> {
        /// Width of an `f64` on any target.
        const F64_BYTES: u64 = 8;
        /// A 32-bit `Vec` header: pointer, capacity, length.
        const VEC_HEADER_BYTES: u64 = 12;
        /// A 32-bit `Ranked`: an `f64` and a `usize`, padded to the `f64` alignment.
        const RANKED_BYTES: u64 = 16;
        let vectors = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(u64::try_from(dims).ok()?))
            .and_then(|e| e.checked_mul(F64_BYTES))
            .ok_or(HnswError::ArithmeticOverflow)?;
        let norms = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(F64_BYTES))
            .ok_or(HnswError::ArithmeticOverflow)?;
        // One `Vec` header per node for its layer list, plus one per layer it occupies.
        let node_records = u64::try_from(rows)
            .ok()
            .and_then(|r| r.checked_mul(VEC_HEADER_BYTES.checked_mul(2)?))
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
        let edge_bytes = edges
            .checked_mul(RANKED_BYTES)
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
        assert!(Params::new(16, 16, 16, 1).is_ok(), "M0 = M, ef_c = M0");
        assert!(Params::new(4, 32, 32, 1).is_ok(), "ef_c exactly at M0");
    }

    #[test]
    fn a_beam_too_narrow_for_layer_zero_is_refused() {
        // Layer 0's bound is M0, so a beam that clears M but not M0 cannot fill the layer
        // every query traverses, and the bound would be a number the graph never reaches.
        assert!(
            Params::new(4, 32, 8, 1).is_err(),
            "ef_construction 8 cannot fill a layer-0 bound of 32"
        );
        assert!(
            Params::new(4, 32, 32, 1).is_ok(),
            "and the neighbouring valid set must still be admitted"
        );
    }

    #[test]
    fn the_thirty_two_bit_admission_check_refuses_and_admits() {
        // Both directions, on whatever host runs the suite. An untested size estimator is a
        // refusal nobody has proven, and an over-count looks exactly like correct strictness
        // until a caller meets a build that should have fitted and does not.
        let params = Params::new(16, 32, 64, 16).expect("valid");

        // Find the smallest row count this parameter set refuses, by doubling then bisecting,
        // so the boundary is discovered rather than hard-coded against the estimator.
        let mut over = 1_usize;
        while params.admits_on_32_bit(over, 256).is_ok() {
            over = over
                .checked_mul(2)
                .expect("a refusal is reached well before usize wraps");
        }
        let mut under = over / 2;
        while under + 1 < over {
            let mid = under + (over - under) / 2;
            if params.admits_on_32_bit(mid, 256).is_ok() {
                under = mid;
            } else {
                over = mid;
            }
        }

        assert!(
            params.admits_on_32_bit(under, 256).is_ok(),
            "the row count just under the bound must still be admitted"
        );
        assert!(
            params.admits_on_32_bit(over, 256).is_err(),
            "the row count just over the bound must be refused"
        );
        assert!(
            u32::try_from(params.footprint_bytes_32(under, 256).expect("estimates")).is_ok(),
            "the admitted estimate must be within the addressable range"
        );
    }

    #[test]
    fn the_thirty_two_bit_estimate_is_the_same_on_every_host() {
        // Stated in explicit 32-bit widths, so a 64-bit host computing it for a wasm32
        // target gets the number that target would see rather than its own layout.
        let params = Params::new(16, 32, 64, 16).expect("valid");
        let vectors_and_norms = 1_000_u64 * 256 * 8 + 1_000 * 8;
        let estimate = params.footprint_bytes_32(1_000, 256).expect("estimates");
        assert!(
            estimate > vectors_and_norms,
            "the estimate must account for adjacency as well as the matrix"
        );
        assert_eq!(
            estimate,
            params.footprint_bytes_32(1_000, 256).expect("estimates"),
            "the estimate is a pure function of its inputs"
        );
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

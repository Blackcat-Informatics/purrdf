// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Integer-only geodetic-normal cube cells with a quadratic warp and Hilbert order.
//!
//! The assignment law is [`CubeHilbertQ62V1`]. Its numerical sequence, ownership,
//! charts and encoding are frozen in `cells/LAW.md`. No admission limit, floating
//! environment, compiler identity or proof tightness enters a grid profile.

mod carrier;
mod constants;
mod cover;
mod fixed;
mod hierarchy;
mod index;
mod profile;
mod region;
mod scale;

pub(crate) use carrier::{CellCarrierBounds, cell_carrier_bounds};
pub use cover::{
    CellCover, ClosedBox, CoverLawId, CoverLevels, CoverReceipt, FixedCoverLimits, MixedCoverLimits,
};
pub use hierarchy::{Ancestors, CellId, CellRange};
pub use index::{PointCellIndex, PointCellIndexId, PointIndexLimits, PointIndexPoint};
pub use profile::{GridProfileId, NativeGridProfile};
pub use scale::{
    CellScaleBounds, level_for_max_edge_length, level_for_max_edge_length_in_policy,
    level_for_max_edge_length_in_policy_metered, physical_scale_bounds,
    physical_scale_bounds_and_level_in_policy, physical_scale_bounds_and_level_in_policy_metered,
    physical_scale_bounds_in_policy, physical_scale_bounds_in_policy_metered,
};

use crate::error::GeoError;
use crate::geographic::LonLat;
use crate::metric::Metres;
use crate::{ExecutionPolicy, MetricWorkObserver};

/// The greatest supported cell level; its face path has sixty bits.
pub const MAX_LEVEL: u8 = 30;

/// The native, deterministic quadratic-warped geodetic-normal cube grid.
///
/// The two native ellipsoids share angular assignment and have distinct profile
/// identities. This type performs no datum conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubeHilbertQ62V1 {
    profile: NativeGridProfile,
}

impl CubeHilbertQ62V1 {
    /// Select an explicit native geographic profile.
    #[must_use]
    pub const fn new(profile: NativeGridProfile) -> Self {
        Self { profile }
    }

    /// The mathematical grid and geographic profile identity.
    #[must_use]
    pub fn profile_id(self) -> GridProfileId {
        self.profile.id()
    }

    /// Certified physical edge-scale bounds for this native geographic profile.
    ///
    /// # Errors
    ///
    /// Refuses levels outside zero through thirty.
    pub fn physical_scale_bounds(self, level: u8) -> Result<CellScaleBounds, GeoError> {
        physical_scale_bounds(&self.profile.ellipsoid(), level)
    }

    /// Smallest level whose guarded upper edge bound meets an exact metre target.
    ///
    /// # Errors
    ///
    /// Refuses nonpositive and unattainably small targets.
    pub fn level_for_max_edge_length(self, target: &Metres) -> Result<u8, GeoError> {
        level_for_max_edge_length(&self.profile.ellipsoid(), target)
    }

    /// Admit the complete exact physical-scale construction under one policy.
    /// # Errors
    /// Refuses invalid resolution or incomplete integer resource admission.
    pub fn physical_scale_bounds_in_policy(
        self,
        level: u8,
        policy: ExecutionPolicy,
    ) -> Result<CellScaleBounds, GeoError> {
        physical_scale_bounds_in_policy(&self.profile.ellipsoid(), level, policy)
    }

    /// Meter physical scales through the shared integer-only observer.
    /// # Errors
    /// Adds observer refusal to complete scale admission.
    pub fn physical_scale_bounds_in_policy_metered(
        self,
        level: u8,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellScaleBounds, GeoError> {
        physical_scale_bounds_in_policy_metered(&self.profile.ellipsoid(), level, policy, observer)
    }

    /// Share the admitted exact factors across bounds and optional level selection.
    /// # Errors
    /// Preserves invalid target, level and complete resource refusals.
    pub fn physical_scale_bounds_and_level_in_policy(
        self,
        level: u8,
        target: Option<&Metres>,
        policy: ExecutionPolicy,
    ) -> Result<(CellScaleBounds, Option<u8>), GeoError> {
        physical_scale_bounds_and_level_in_policy(&self.profile.ellipsoid(), level, target, policy)
    }

    /// Meter one combined scale record with integer-only callbacks.
    /// # Errors
    /// Preserves complete scale/selection admission and observer refusal.
    pub fn physical_scale_bounds_and_level_in_policy_metered(
        self,
        level: u8,
        target: Option<&Metres>,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(CellScaleBounds, Option<u8>), GeoError> {
        physical_scale_bounds_and_level_in_policy_metered(
            &self.profile.ellipsoid(),
            level,
            target,
            policy,
            observer,
        )
    }

    /// Select the exact physical level with original-target arithmetic admitted.
    /// # Errors
    /// Refuses invalid target or incomplete integer resource admission.
    pub fn level_for_max_edge_length_in_policy(
        self,
        target: &Metres,
        policy: ExecutionPolicy,
    ) -> Result<u8, GeoError> {
        level_for_max_edge_length_in_policy(&self.profile.ellipsoid(), target, policy)
    }

    /// Meter exact level selection through integer-only governor callbacks.
    /// # Errors
    /// Adds observer refusal to complete physical target admission.
    pub fn level_for_max_edge_length_in_policy_metered(
        self,
        target: &Metres,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<u8, GeoError> {
        level_for_max_edge_length_in_policy_metered(
            &self.profile.ellipsoid(),
            target,
            policy,
            observer,
        )
    }

    /// Assign a validated geographic point, deriving every level from its leaf.
    ///
    /// Longitude and latitude remain the caller's exact rationals. All work is
    /// integer arithmetic, including angle reduction and boundary comparisons.
    ///
    /// # Errors
    ///
    /// Refuses levels above thirty and a violated arithmetic invariant.
    pub fn assign(self, point: &LonLat, level: u8) -> Result<CellId, GeoError> {
        validate_level(level)?;
        let leaf = Self::assign_leaf(point, self.profile_id())?;
        leaf.ancestor(level)
    }

    /// Assign a batch into an equally sized caller buffer, validating its shape
    /// before writing any result. The profile is prepared once per batch.
    ///
    /// # Errors
    ///
    /// Refuses an invalid level, mismatched output length or arithmetic failure.
    pub fn assign_batch(
        self,
        points: &[LonLat],
        level: u8,
        output: &mut [CellId],
    ) -> Result<(), GeoError> {
        self.assign_batch_with(points.iter(), level, output, &mut fixed::PureAssignment)
    }

    /// Assign under an explicit resource policy without inspecting the floating
    /// environment. Original rational arithmetic is admitted before execution.
    ///
    /// # Errors
    /// Adds complete work, memory and output refusal to [`Self::assign`].
    pub fn assign_in_policy(
        self,
        point: &LonLat,
        level: u8,
        policy: ExecutionPolicy,
    ) -> Result<CellId, GeoError> {
        self.assign_in_admission(point, level, policy, None)
    }

    /// Assign with integer-only governor charging and cancellation.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::assign_in_policy`].
    pub fn assign_in_policy_metered(
        self,
        point: &LonLat,
        level: u8,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellId, GeoError> {
        self.assign_in_admission(point, level, policy, Some(observer))
    }

    fn assign_in_admission(
        self,
        point: &LonLat,
        level: u8,
        policy: ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<CellId, GeoError> {
        let mut admission = assignment_admission(policy, 1, observer)?;
        let result = self.assign_admitted(point, level, &mut admission)?;
        admission.poll()?;
        Ok(result)
    }

    /// Assign borrowed exact sources into a caller buffer under one policy.
    /// The iterator's declared shape and complete output admission are checked
    /// before writing. Source coordinates are never cloned into a batch vector.
    ///
    /// # Errors
    /// Refuses source/output shape, resource admission, or arithmetic failure.
    /// On a later arithmetic or dishonest iterator error, earlier buffer slots
    /// may have been written; callers publish the buffer only after success.
    pub fn assign_batch_in_policy<'a>(
        self,
        points: impl ExactSizeIterator<Item = &'a LonLat>,
        level: u8,
        output: &mut [CellId],
        policy: ExecutionPolicy,
    ) -> Result<(), GeoError> {
        self.assign_batch_in_admission(points, level, output, policy, None)
    }

    /// Charge a complete borrowed batch through the integer-only governor seam.
    ///
    /// # Errors
    /// Adds cancellation/observer refusal to [`Self::assign_batch_in_policy`].
    pub fn assign_batch_in_policy_metered<'a>(
        self,
        points: impl ExactSizeIterator<Item = &'a LonLat>,
        level: u8,
        output: &mut [CellId],
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.assign_batch_in_admission(points, level, output, policy, Some(observer))
    }

    fn assign_batch_in_admission<'a>(
        self,
        points: impl ExactSizeIterator<Item = &'a LonLat>,
        level: u8,
        output: &mut [CellId],
        policy: ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let mut admission = assignment_admission(policy, points.len(), observer)?;
        self.assign_batch_with(points, level, output, &mut admission)?;
        admission.poll()
    }

    fn assign_batch_with<'a>(
        self,
        mut points: impl ExactSizeIterator<Item = &'a LonLat>,
        level: u8,
        output: &mut [CellId],
        admission: &mut impl fixed::IntegerAdmission,
    ) -> Result<(), GeoError> {
        validate_level(level)?;
        let expected = points.len();
        let actual = output.len();
        if actual != expected {
            return Err(GeoError::InvalidOutputLength { expected, actual });
        }
        let profile = self.profile_id();
        for (index, result) in output.iter_mut().enumerate() {
            let point = points.next().ok_or(GeoError::InvalidOutputLength {
                expected: index,
                actual,
            })?;
            *result = Self::assign_leaf_admitted(point, profile, admission)?.ancestor(level)?;
        }
        if points.next().is_some() {
            return Err(GeoError::InvalidOutputLength {
                expected: actual.saturating_add(1),
                actual,
            });
        }
        Ok(())
    }

    fn assign_leaf(point: &LonLat, profile: GridProfileId) -> Result<CellId, GeoError> {
        Self::assign_leaf_admitted(point, profile, &mut fixed::PureAssignment)
    }
    fn assign_admitted(
        self,
        point: &LonLat,
        level: u8,
        admission: &mut impl fixed::IntegerAdmission,
    ) -> Result<CellId, GeoError> {
        validate_level(level)?;
        Self::assign_leaf_admitted(point, self.profile_id(), admission)?.ancestor(level)
    }
    fn assign_leaf_admitted(
        point: &LonLat,
        profile: GridProfileId,
        admission: &mut impl fixed::IntegerAdmission,
    ) -> Result<CellId, GeoError> {
        // At most fifty fixed-width rounded products, sixty-two exact bin trials,
        // thirty Hilbert digits, six face trials and bounded encoding remain.
        admission.bounded(160)?;
        let [x, y, z] = fixed::normal_admitted(point, admission)?;
        // Face order doubles as the exact dominant-component tie order.
        let components = [x, y, z, -x, -y, -z];
        let mut face = 0_usize;
        for candidate in 1..components.len() {
            if components[candidate] > components[face] {
                face = candidate;
            }
        }
        let dominant = components[face];
        if dominant <= 0 {
            return Err(GeoError::ArithmeticOverflow("zero cube normal"));
        }
        let (u, v) = chart_coordinates(face, [x, y, z]);
        let mut i = owned_bin(u, dominant);
        let mut j = owned_bin(v, dominant);
        if face & 1 == 1 {
            core::mem::swap(&mut i, &mut j);
        }
        let path = hilbert_path(i, j, MAX_LEVEL);
        let face =
            u8::try_from(face).map_err(|_| GeoError::ArithmeticOverflow("cube face conversion"))?;
        CellId::from_path(profile, face, path, MAX_LEVEL)
    }
}

fn assignment_admission(
    policy: ExecutionPolicy,
    count: usize,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<cover::CoverBudget<'_>, GeoError> {
    let metered = observer.is_some();
    let mut progress = crate::context::WorkProgress::integer(observer);
    progress.initial()?;
    let limits = policy.limits();
    if count as u64 > limits.max_output_elements {
        return Err(GeoError::OutputExhausted {
            limit: limits.max_output_elements,
        });
    }
    cover::CoverBudget::from_progress(
        MixedCoverLimits {
            max_emitted_cells: limits.max_output_elements,
            max_work_items: limits.max_work_items,
            max_workspace_bytes: limits.max_workspace_bytes,
        },
        progress,
        metered,
    )
}

fn chart_coordinates(face: usize, [x, y, z]: [i128; 3]) -> (i128, i128) {
    [(y, z), (-x, z), (-x, -y), (-z, -y), (-z, x), (y, x)][face]
}

pub(super) fn validate_level(level: u8) -> Result<(), GeoError> {
    if level > MAX_LEVEL {
        Err(GeoError::InvalidResolution(level))
    } else {
        Ok(())
    }
}

/// Greatest dyadic boundary not above the coordinate; the outer endpoint
/// belongs to the final bin. Both cross-products fit signed 128 bits.
fn owned_bin(numerator: i128, denominator: i128) -> u32 {
    const N: u32 = 1 << MAX_LEVEL;
    let scaled_coordinate = numerator
        * i128::try_from(warp_denominator(u64::from(N))).expect("bounded dyadic denominator");
    let mut low = 0;
    let mut high = N;
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let boundary = denominator * warp_numerator(u64::from(middle), u64::from(N));
        if boundary <= scaled_coordinate {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low.min(N - 1)
}

fn warp_numerator(k: u64, n: u64) -> i128 {
    debug_assert!(k <= n && n <= (1 << MAX_LEVEL));
    let a = 2 * i128::from(k) - i128::from(n);
    a * (2 * i128::from(n) + a.abs())
}
fn warp_denominator(n: u64) -> u128 {
    debug_assert!(n > 0 && n <= (1 << MAX_LEVEL));
    3 * u128::from(n) * u128::from(n)
}

fn hilbert_path(mut x: u32, mut y: u32, level: u8) -> u64 {
    let mut size = 1_u32 << level;
    let mut path = 0;
    while size > 1 {
        let half = size / 2;
        let digit = if x < half && y < half {
            core::mem::swap(&mut x, &mut y);
            0
        } else if x < half {
            y -= half;
            1
        } else if y >= half {
            x -= half;
            y -= half;
            2
        } else {
            (x, y) = (half - 1 - y, size - 1 - x);
            3
        };
        path = (path << 2) | digit;
        size = half;
    }
    path
}

#[cfg(test)]
mod tests;

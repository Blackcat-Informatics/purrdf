// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conservative closed-set covers with a frozen classifier and canonical unions.

use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_into, hex::Digest32};
use purrdf_xsd::integer::ExactArithmeticCost;
use purrdf_xsd::integer::ExactOperation;
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

use super::{CellId, CellRange, CubeHilbertQ62V1, MAX_LEVEL, NativeGridProfile, validate_level};
use crate::context::WorkProgress;
use crate::numerical::{fixed_from_rat, frozen_decimal as decimal, math_quantized_decimal};
use crate::{
    ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference, Int, LonLat, Metres,
    MetricContext, MetricWorkObserver, Rat, XsdDoubleMetres,
};

const COVER_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/cover-law/v2");
const CLASSIFIER: &[u8] = b"cube-closed-footprint-caps;box=cube-rational-midpoint-cap;angular-radius=du+dv+2^-59;axis=RN-even-degree15;axis-guard=pi-upper*10^-15/180;closed-cap-bands;cos-lower=1-abs(latitude)/90;v1";

/// Identity of completed-cover classification and canonicalization, independent
/// of point assignment, admission limits and proof tightness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoverLawId(Digest32);

impl CoverLawId {
    /// The frozen native conservative-cover law.
    #[must_use]
    pub fn native() -> Self {
        let mut hash = Hasher::new();
        for field in [COVER_DOMAIN.as_bytes(), CLASSIFIER, disk::CLASSIFIER] {
            frame_le_into(&mut hash, field);
        }
        Self(Digest32::new(*hash.finalize().as_bytes()))
    }
    pub(super) fn from_classifier(classifier: &[u8]) -> Self {
        let mut hash = Hasher::new();
        for field in [COVER_DOMAIN.as_bytes(), classifier] {
            frame_le_into(&mut hash, field);
        }
        Self(Digest32::new(*hash.finalize().as_bytes()))
    }

    /// The fixed-width content digest.
    #[must_use]
    pub const fn digest(self) -> Digest32 {
        self.0
    }
}

/// Closed interval of levels admitted by the geometric output contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CoverLevels {
    min: u8,
    max: u8,
}
impl CoverLevels {
    /// Validate `0 <= min <= max <= 30`.
    ///
    /// # Errors
    /// Refuses an inverted or unsupported interval.
    pub fn new(min: u8, max: u8) -> Result<Self, GeoError> {
        if min > max || max > MAX_LEVEL {
            return Err(GeoError::InvalidCoverLevels { min, max });
        }
        Ok(Self { min, max })
    }
    /// Lowest emitted/coalesced level.
    #[must_use]
    pub const fn min(self) -> u8 {
        self.min
    }
    /// Finest geometric classification level.
    #[must_use]
    pub const fn max(self) -> u8 {
        self.max
    }
}

/// Fixed-resolution admission counts logical cells, including compressed ranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedCoverLimits {
    /// Maximum complete logical-cell count.
    pub max_logical_cells: u64,
    /// Maximum visited and numerical work items.
    pub max_work_items: u64,
    /// Maximum simultaneously retained workspace bytes.
    pub max_workspace_bytes: u64,
}
impl FixedCoverLimits {
    /// Frozen ordinary fixed cover admission.
    pub const DEFAULT: Self = Self {
        max_logical_cells: 65_536,
        max_work_items: 262_144,
        max_workspace_bytes: 64 * 1024 * 1024,
    };
}
purrdf_hash::default_from_new!(FixedCoverLimits => defaults);
impl FixedCoverLimits {
    const fn defaults() -> Self {
        Self::DEFAULT
    }
}

/// Mixed admission counts final emitted cells, never their leaf descendants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MixedCoverLimits {
    /// Maximum final canonical emitted-cell count.
    pub max_emitted_cells: u64,
    /// Maximum visited and numerical work items.
    pub max_work_items: u64,
    /// Maximum simultaneously retained workspace bytes.
    pub max_workspace_bytes: u64,
}
impl MixedCoverLimits {
    /// Frozen ordinary mixed cover admission.
    pub const DEFAULT: Self = Self {
        max_emitted_cells: 65_536,
        max_work_items: 262_144,
        max_workspace_bytes: 64 * 1024 * 1024,
    };
}
purrdf_hash::default_from_new!(MixedCoverLimits => defaults);
impl MixedCoverLimits {
    const fn defaults() -> Self {
        Self::DEFAULT
    }
}

/// Invocation evidence; none of these fields enters cover geometry or identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverReceipt {
    /// Number of classified cells.
    pub visited_nodes: u64,
    /// Total charged geometric and numerical work.
    pub work_items: u64,
    /// Conservative greatest simultaneous retained/scratch workspace.
    pub workspace_peak: u64,
    /// Final emitted cells at levels zero through thirty.
    pub level_histogram: [u64; 31],
}

/// Complete sorted, unique, non-overlapping conservative cell cover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellCover {
    cells: Vec<CellId>,
    levels: CoverLevels,
    profile: super::GridProfileId,
    receipt: CoverReceipt,
    retained_bytes: u64,
    law: CoverLawId,
}
impl CellCover {
    /// Canonical hierarchy-ordered cells.
    #[must_use]
    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }
    /// Output's geometric level contract.
    #[must_use]
    pub const fn levels(&self) -> CoverLevels {
        self.levels
    }
    /// Native assignment/reference identity.
    #[must_use]
    pub const fn profile(&self) -> super::GridProfileId {
        self.profile
    }
    /// Independent classifier/canonicalization law.
    #[must_use]
    pub const fn law_id(&self) -> CoverLawId {
        self.law
    }
    pub(super) fn with_law(mut self, law: CoverLawId) -> Self {
        self.law = law;
        self
    }
    /// Separate invocation resource evidence.
    #[must_use]
    pub const fn receipt(&self) -> &CoverReceipt {
        &self.receipt
    }
    pub(super) const fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }
    /// Complete retained native cover storage, excluding invocation scratch.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_bytes()
    }
    /// Convert each disjoint subtree to its exact stored-level interval.
    ///
    /// # Errors
    /// Refuses a stored level shallower than an emitted cell or above thirty.
    pub fn ranges(&self, stored_level: u8) -> Result<Vec<CellRange>, GeoError> {
        validate_level(stored_level)?;
        self.cells
            .iter()
            .map(|cell| cell.descendant_range(stored_level))
            .collect()
    }
    /// Test hierarchy containment of a key without materializing descendants.
    ///
    /// # Errors
    /// Refuses a different profile or a point key shallower than cover cells.
    pub fn contains(&self, cell: CellId) -> Result<bool, GeoError> {
        if cell.profile() != self.profile {
            return Err(GeoError::GridProfileMismatch);
        }
        if let Some(region) = self
            .cells
            .iter()
            .find(|region| region.level() > cell.level())
        {
            return Err(GeoError::InvalidStoredLevel {
                cell: region.level(),
                stored: cell.level(),
            });
        }
        for region in &self.cells {
            if region.level() <= cell.level() && cell.ancestor(region.level())? == *region {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// A closed geographic box. West greater than east wraps through the cut;
/// equal longitudes describe a meridian and `[-180,180]` is full longitude.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosedBox {
    west: Rat,
    south: Rat,
    east: Rat,
    north: Rat,
}
impl ClosedBox {
    /// Validate original angular coordinates and south no greater than north.
    ///
    /// # Errors
    /// Refuses angular range or an inverted latitude interval.
    pub fn new(west: Rat, south: Rat, east: Rat, north: Rat) -> Result<Self, GeoError> {
        LonLat::new(west.clone(), south.clone())?;
        LonLat::new(east.clone(), north.clone())?;
        if south > north {
            return Err(GeoError::domain("box south exceeds north"));
        }
        Ok(Self {
            west,
            south,
            east,
            north,
        })
    }
    /// Exact closed membership with seam/pole identification.
    #[must_use]
    pub fn contains(&self, point: &LonLat) -> bool {
        if point.latitude() < &self.south || point.latitude() > &self.north {
            return false;
        }
        point.is_pole() || self.longitude_contains(point.longitude())
    }
    fn full_longitude(&self) -> bool {
        self.west == Rat::from_i64(-180) && self.east == Rat::from_i64(180)
    }
    fn longitude_contains(&self, longitude: &Rat) -> bool {
        if self.full_longitude() {
            return true;
        }
        let contains = |value: &Rat| {
            if self.west <= self.east {
                value >= &self.west && value <= &self.east
            } else {
                value >= &self.west || value <= &self.east
            }
        };
        contains(longitude) || (longitude.abs() == Rat::from_i64(180) && contains(&longitude.neg()))
    }
    fn intervals(&self) -> Vec<(Rat, Rat)> {
        if self.full_longitude() {
            return vec![(Rat::from_i64(-180), Rat::from_i64(180))];
        }
        if self.west <= self.east {
            vec![(self.west.clone(), self.east.clone())]
        } else {
            vec![
                (Rat::from_i64(-180), self.east.clone()),
                (self.west.clone(), Rat::from_i64(180)),
            ]
        }
    }
}

impl CubeHilbertQ62V1 {
    /// Cover a closed physical-radius disk at one fixed level.
    ///
    /// # Errors
    /// Refuses negative radius and precision/work/memory/logical-cell exhaustion.
    pub fn cover_disk_fixed(
        self,
        center: &LonLat,
        radius: &Metres,
        level: u8,
        limits: FixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_disk_mixed(
            center,
            radius,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
        )
    }
    /// Cover a closed physical-radius disk under the strict mixed-level law.
    ///
    /// # Errors
    /// Refuses negative radius and incomplete numerical/resource admission.
    pub fn cover_disk_mixed(
        self,
        center: &LonLat,
        radius: &Metres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_disk(center, radius, levels, limits, None)
    }
    fn cover_disk(
        self,
        center: &LonLat,
        radius: &Metres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<CellCover, GeoError> {
        let mut budget = CoverBudget::new_metered(limits, observer)?;
        self.cover_disk_in(center, radius, levels, limits, &mut budget)
    }
    fn cover_disk_in(
        self,
        center: &LonLat,
        radius: &Metres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        budget: &mut CoverBudget<'_>,
    ) -> Result<CellCover, GeoError> {
        if radius.exact().signum() < 0 {
            return budget.rational(ExactOperation::Linear, &[radius.exact()], 1, || {
                Err(GeoError::NegativePhysicalRadius(radius.exact().clone()))
            });
        }
        let mut disk = disk::Disk::new(self.profile, center, radius, budget)?;
        build(self, levels, limits, budget, |cell, budget| {
            disk.classify(cell, budget)
        })
    }
    /// Cover precisely the candidate superset for reported-double comparison.
    ///
    /// Finite negative thresholds return an empty cover. Positive thresholds are
    /// padded by one micrometre plus their certified binary64 half-ULP.
    ///
    /// # Errors
    /// Refuses incomplete numerical/resource admission.
    pub fn cover_reported_disk_mixed(
        self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_reported_disk(center, threshold, levels, limits, None)
    }
    fn cover_reported_disk(
        self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<CellCover, GeoError> {
        if threshold.is_negative() {
            return empty(self, levels, limits, observer);
        }
        let mut budget = CoverBudget::new_metered(limits, observer)?;
        let mut retained = 0_u64;
        let result = (|| {
            let conversion = ExactArithmeticCost::binary64_rational(threshold.bits())
                .ok_or(GeoError::ArithmeticOverflow("reported threshold admission"))?;
            let half_ulp = crate::metric::binary64_half_ulp_cost(threshold.bits())
                .ok_or(GeoError::ArithmeticOverflow("reported padding admission"))?;
            let quantum = ExactArithmeticCost::decimal(7, 6)
                .ok_or(GeoError::ArithmeticOverflow("reported quantum admission"))?;
            let owners = conversion
                .workspace_bytes
                .checked_add(half_ulp.workspace_bytes)
                .and_then(|bytes| bytes.checked_add(quantum.workspace_bytes))
                .ok_or(GeoError::ArithmeticOverflow("reported threshold owners"))?;
            let cost = conversion
                .followed_by(half_ulp)
                .and_then(|cost| cost.followed_by(quantum))
                .ok_or(GeoError::ArithmeticOverflow("reported threshold admission"))?;
            budget.reserve(owners)?;
            retained = owners;
            let (exact, half_ulp, quantum) = budget.exact(cost, || {
                Ok((
                    Rat::from_binary64(threshold.reported()).expect("validated finite threshold"),
                    crate::metric::binary64_half_ulp(threshold.bits()),
                    decimal("0.000001"),
                ))
            })?;
            let padding_cost = crate::numerical::rational_cost(
                ExactOperation::RationalAdd,
                &[&quantum, &half_ulp],
                1,
            )
            .ok_or(GeoError::ArithmeticOverflow("reported padding addition"))?;
            let next = retained
                .checked_add(padding_cost.workspace_bytes)
                .ok_or(GeoError::ArithmeticOverflow("reported padding owner"))?;
            budget.reserve(padding_cost.workspace_bytes)?;
            retained = next;
            let padding = budget.exact(padding_cost, || Ok(quantum.add(&half_ulp)))?;
            let radius_cost = crate::numerical::rational_cost(
                ExactOperation::RationalAdd,
                &[&exact, &padding],
                1,
            )
            .ok_or(GeoError::ArithmeticOverflow("reported radius addition"))?;
            let next = retained
                .checked_add(radius_cost.workspace_bytes)
                .ok_or(GeoError::ArithmeticOverflow("reported radius owner"))?;
            budget.reserve(radius_cost.workspace_bytes)?;
            retained = next;
            let radius = budget.exact(radius_cost, || Ok(Metres::new(exact.add(&padding))))?;
            self.cover_disk_in(center, &radius, levels, limits, &mut budget)
        })();
        // Every conversion/addition owner above, and the temporary disk built
        // from it, drops before its allowance. Returned cover cells retain their
        // own independent storage receipt from the original traversal.
        budget.release(retained)?;
        result
    }
    /// Fixed-resolution candidate cover for reported-double comparison.
    ///
    /// # Errors
    /// Refuses invalid level and incomplete admission.
    pub fn cover_reported_disk_fixed(
        self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        level: u8,
        limits: FixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_reported_disk_mixed(
            center,
            threshold,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
        )
    }
    /// Cover a closed geographic box at one fixed resolution.
    ///
    /// # Errors
    /// Refuses invalid level and incomplete admission.
    pub fn cover_box_fixed(
        self,
        area: &ClosedBox,
        level: u8,
        limits: FixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_box_mixed(area, CoverLevels::new(level, level)?, fixed_limits(limits))
    }
    /// Cover a closed geographic box under the strict mixed-level law.
    ///
    /// # Errors
    /// Refuses incomplete numerical/resource admission.
    pub fn cover_box_mixed(
        self,
        area: &ClosedBox,
        levels: CoverLevels,
        limits: MixedCoverLimits,
    ) -> Result<CellCover, GeoError> {
        self.cover_box(area, levels, limits, None)
    }
    fn cover_box(
        self,
        area: &ClosedBox,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<CellCover, GeoError> {
        let mut budget = CoverBudget::new_metered(limits, observer)?;
        let source_bits = [&area.west, &area.south, &area.east, &area.north]
            .into_iter()
            .fold(0_u64, |bits, value| {
                bits.saturating_add(value.numerator().bit_len())
                    .saturating_add(value.denominator().bit_len())
            });
        budget.reserve(source_bits.div_ceil(8).saturating_mul(32))?;
        let operands = [&area.west, &area.south, &area.east, &area.north];
        let (full_longitude, seam, full_latitude) =
            budget.rational(ExactOperation::RationalCompare, &operands, 14, || {
                Ok((
                    area.full_longitude(),
                    area.longitude_contains(&Rat::from_i64(-180)),
                    area.south == Rat::from_i64(-90) && area.north == Rat::from_i64(90),
                ))
            })?;
        if full_longitude && full_latitude {
            // The exact closed whole-surface box proves every footprint inside.
            // Traversal still obeys the same minimum level, canonicalization,
            // logical output/work/memory limits and governor law.
            return build(self, levels, limits, &mut budget, |_, _| {
                Ok(Classification::Inside)
            });
        }
        let target =
            budget.rational(
                ExactOperation::Linear,
                &operands,
                4,
                || Ok(area.intervals()),
            )?;
        build(self, levels, limits, &mut budget, |cell, budget| {
            classify_box(
                area,
                &target,
                full_longitude,
                seam,
                &Footprint::new_admitted(cell, budget)?,
                budget,
            )
        })
    }
    /// Meter a complete mixed physical disk cover through bounded work chunks.
    /// # Errors
    /// Adds observer refusal to the ordinary complete cover contract.
    pub fn cover_disk_mixed_metered(
        self,
        center: &LonLat,
        radius: &Metres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_disk(center, radius, levels, limits, Some(observer))
    }
    /// Meter a complete fixed physical disk cover.
    /// # Errors
    /// Refuses invalid level or incomplete observer/resource admission.
    pub fn cover_disk_fixed_metered(
        self,
        center: &LonLat,
        radius: &Metres,
        level: u8,
        limits: FixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_disk(
            center,
            radius,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
            Some(observer),
        )
    }
    /// Meter a complete mixed reported-threshold disk cover.
    /// # Errors
    /// Adds observer refusal to the reported cover contract.
    pub fn cover_reported_disk_mixed_metered(
        self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_reported_disk(center, threshold, levels, limits, Some(observer))
    }
    /// Meter a complete fixed reported-threshold disk cover.
    /// # Errors
    /// Refuses invalid level or incomplete observer/resource admission.
    pub fn cover_reported_disk_fixed_metered(
        self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        level: u8,
        limits: FixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_reported_disk(
            center,
            threshold,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
            Some(observer),
        )
    }
    /// Meter a complete mixed closed-box cover.
    /// # Errors
    /// Adds observer refusal to the ordinary box cover contract.
    pub fn cover_box_mixed_metered(
        self,
        area: &ClosedBox,
        levels: CoverLevels,
        limits: MixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_box(area, levels, limits, Some(observer))
    }
    /// Meter a complete fixed closed-box cover.
    /// # Errors
    /// Refuses invalid level or incomplete observer/resource admission.
    pub fn cover_box_fixed_metered(
        self,
        area: &ClosedBox,
        level: u8,
        limits: FixedCoverLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<CellCover, GeoError> {
        self.cover_box(
            area,
            CoverLevels::new(level, level)?,
            fixed_limits(limits),
            Some(observer),
        )
    }
}

pub(super) fn fixed_limits(limits: FixedCoverLimits) -> MixedCoverLimits {
    MixedCoverLimits {
        max_emitted_cells: limits.max_logical_cells,
        max_work_items: limits.max_work_items,
        max_workspace_bytes: limits.max_workspace_bytes,
    }
}
fn empty(
    grid: CubeHilbertQ62V1,
    levels: CoverLevels,
    limits: MixedCoverLimits,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<CellCover, GeoError> {
    let budget = CoverBudget::new_metered(limits, observer)?;
    Ok(CellCover {
        cells: Vec::new(),
        levels,
        profile: grid.profile_id(),
        receipt: budget.receipt([0; 31]),
        retained_bytes: 0,
        law: CoverLawId::native(),
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Classification {
    Outside,
    Inside,
    Straddling,
}

pub(super) struct CoverBudget<'a> {
    limits: MixedCoverLimits,
    work: u64,
    visited: u64,
    retained: u64,
    peak: u64,
    pub(super) progress: WorkProgress<'a>,
    pub(super) metered: bool,
    polled_work: u64,
    integer_scratch: Option<purrdf_xsd::integer::LimbScratch>,
    borrowed_scratch: bool,
    execution: ExecutionLimits,
}
impl super::fixed::IntegerAdmission for CoverBudget<'_> {
    const METERED: bool = true;
    fn bounded(&mut self, count: u64) -> Result<(), GeoError> {
        self.charge(count)
    }
    fn exact<T>(
        &mut self,
        cost: ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        Self::exact(self, cost, evaluate)
    }
}
impl<'a> CoverBudget<'a> {
    #[cfg(test)]
    pub(super) fn new(limits: MixedCoverLimits) -> Result<Self, GeoError> {
        Self::new_metered(limits, None)
    }
    pub(super) fn new_metered(
        limits: MixedCoverLimits,
        observer: Option<&'a mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let metered = observer.is_some();
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        Self::from_progress(limits, progress, metered)
    }
    pub(super) fn from_progress(
        limits: MixedCoverLimits,
        progress: WorkProgress<'a>,
        metered: bool,
    ) -> Result<Self, GeoError> {
        if limits.max_emitted_cells == 0
            || limits.max_work_items == 0
            || limits.max_workspace_bytes == 0
        {
            return Err(GeoError::InvalidExecutionPolicy(
                "cover admission limits must be positive",
            ));
        }
        let mut value = Self {
            limits,
            work: 0,
            visited: 0,
            retained: 0,
            peak: 0,
            progress,
            metered,
            polled_work: 0,
            integer_scratch: None,
            borrowed_scratch: false,
            execution: ExecutionLimits::GEOMETRY,
        };
        // Exact geometry temporaries, bounded traversal stack and numerical
        // endpoint bridges have a checked reservation before construction.
        value.reserve(65_536)?;
        Ok(value)
    }
    pub(super) fn charge(&mut self, count: u64) -> Result<(), GeoError> {
        self.work = self
            .work
            .checked_add(count)
            .ok_or(GeoError::WorkExhausted {
                limit: self.limits.max_work_items,
            })?;
        if self.work > self.limits.max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: self.limits.max_work_items,
            });
        }
        if self.work.saturating_sub(self.polled_work) >= 64 {
            self.poll()?;
        }
        Ok(())
    }
    pub(super) fn poll(&mut self) -> Result<(), GeoError> {
        self.progress.charge_counts(self.work, self.peak)?;
        self.polled_work = self.work;
        Ok(())
    }
    pub(super) fn nested(&mut self) -> impl MetricWorkObserver + '_ {
        self.progress.nested(self.work, self.retained, self.peak)
    }
    pub(super) const fn work_items(&self) -> u64 {
        self.work
    }
    pub(super) const fn workspace_peak(&self) -> u64 {
        self.peak
    }
    pub(super) fn context(
        &mut self,
        reference: GeographicReference,
    ) -> Result<MetricContext, GeoError> {
        self.context_for_sources(reference, &[])
    }
    fn context_for_sources(
        &mut self,
        reference: GeographicReference,
        sources: &[&Rat],
    ) -> Result<MetricContext, GeoError> {
        let source_bits = crate::numerical::scratch_source_bits(sources, reference.ellipsoid());
        let mut preparation = MetricContext::new(reference.clone(), self.policy()?)?;
        preparation.set_retained_workspace(self.retained)?;
        if let Some(scratch) = &self.integer_scratch {
            preparation.set_borrowed_integer_scratch(scratch.clone())?;
        }
        let result = {
            let mut nested = self.nested();
            let mut progress = WorkProgress::new(Some(&mut nested));
            progress.initial()?;
            preparation.prepare_integer_scratch_for_observed(source_bits, &mut progress)
        };
        self.context_receipt(&preparation)?;
        result.map_err(|error| self.invocation_error(error))?;
        let scratch = preparation
            .integer_scratch()
            .ok_or(GeoError::ArithmeticOverflow("prepared cover limb arena"))?;
        if self.integer_scratch.as_ref().is_none_or(|previous| {
            previous.limb_capacity() < scratch.limb_capacity()
                || previous.destination_capacity() < scratch.destination_capacity()
        }) {
            let old_bytes = self
                .integer_scratch
                .as_ref()
                .map_or(0, |previous| previous.retained_bytes() as u64);
            // The shared context admitted old+new heap overlap before growth.
            // Retain the new arena once for all subsequent sequential phases;
            // replacing a still-held old destination already refuses there.
            self.reserve(scratch.retained_bytes() as u64)?;
            self.integer_scratch = Some(scratch.clone());
            if !self.borrowed_scratch {
                self.release(old_bytes)?;
            }
            self.borrowed_scratch = false;
        }
        let mut context = MetricContext::new(reference, self.policy()?)?;
        context.set_retained_workspace(self.retained)?;
        context.set_borrowed_integer_scratch(
            self.integer_scratch
                .as_ref()
                .expect("cover arena prepared before borrowing")
                .clone(),
        )?;
        Ok(context)
    }
    pub(super) fn context_receipt(&mut self, context: &MetricContext) -> Result<(), GeoError> {
        self.charge(context.work_items())?;
        self.scratch(context.workspace_peak().saturating_sub(self.retained))
    }
    pub(super) fn reserve(&mut self, count: u64) -> Result<(), GeoError> {
        let next = self
            .retained
            .checked_add(count)
            .ok_or(GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            })?;
        if next > self.limits.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            });
        }
        self.retained = next;
        self.peak = self.peak.max(next);
        self.poll()
    }
    pub(super) fn scratch(&mut self, bytes: u64) -> Result<(), GeoError> {
        self.reserve(bytes)?;
        self.release(bytes)
    }
    pub(super) fn exact<T>(
        &mut self,
        cost: ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        self.reserve(cost.workspace_bytes)?;
        let result = (|| {
            self.charge(cost.work_items)?;
            self.poll()?;
            let result = evaluate();
            self.poll()?;
            result
        })();
        self.release(cost.workspace_bytes)?;
        result
    }
    pub(super) fn rational<T>(
        &mut self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let cost = crate::numerical::rational_cost(operation, operands, count).ok_or(
            GeoError::ArithmeticOverflow("cover exact arithmetic admission"),
        )?;
        self.exact(cost, evaluate)
    }
    #[cfg(test)]
    fn compare(&mut self, left: &Rat, right: &Rat) -> Result<core::cmp::Ordering, GeoError> {
        self.rational(ExactOperation::RationalCompare, &[left, right], 1, || {
            Ok(left.cmp(right))
        })
    }
    pub(super) fn release(&mut self, bytes: u64) -> Result<(), GeoError> {
        self.retained = self
            .retained
            .checked_sub(bytes)
            .ok_or(GeoError::ArithmeticOverflow(
                "cover workspace accounting underflow",
            ))?;
        Ok(())
    }
    fn receipt(&self, level_histogram: [u64; 31]) -> CoverReceipt {
        CoverReceipt {
            visited_nodes: self.visited,
            work_items: self.work,
            workspace_peak: self.peak,
            level_histogram,
        }
    }
    fn math<T>(
        &mut self,
        action: impl FnMut(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<T, MathError>,
    ) -> Result<T, GeoError> {
        self.math_for_sources(&[], action)
    }
    pub(super) fn math_for_sources<T>(
        &mut self,
        sources: &[&Rat],
        mut action: impl FnMut(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<T, MathError>,
    ) -> Result<T, GeoError> {
        let limit = self.execution.max_precision_bits;
        let mut precision = 128.min(limit);
        loop {
            let result = self.math_for_sources_at_precision(precision, sources, &mut action);
            match result {
                Ok(value) => return Ok(value),
                Err(GeoError::PrecisionExhausted { .. } | GeoError::Domain(_))
                    if precision < limit =>
                {
                    precision = precision.saturating_mul(2).min(limit);
                }
                Err(GeoError::Domain(_)) => {
                    return Err(GeoError::PrecisionExhausted { bits: limit });
                }
                Err(error) => return Err(self.invocation_error(error)),
            }
        }
    }
    pub(super) fn math_for_sources_at_precision<T>(
        &mut self,
        precision: u32,
        sources: &[&Rat],
        action: impl FnOnce(&mut CoordinateMath, &mut WorkProgress<'_>) -> Result<T, MathError>,
    ) -> Result<T, GeoError> {
        let mut context = self.context_for_sources(GeographicReference::wgs84(), sources)?;
        let policy = context.policy();
        let result = {
            let mut observer = self.nested();
            let mut progress = WorkProgress::new(Some(&mut observer));
            crate::numerical::with_math_for_sources(
                &mut context,
                &mut progress,
                precision,
                0,
                sources,
                |math, progress| {
                    action(math, progress)
                        .map_err(|error| crate::numerical::geo_math_error(&error, policy))
                },
            )
        };
        self.context_receipt(&context)?;
        result.map_err(|error| self.invocation_error(error))
    }
    pub(super) const fn max_precision_bits(&self) -> u32 {
        self.execution.max_precision_bits
    }
    pub(super) fn invocation_error(&self, error: GeoError) -> GeoError {
        match error {
            GeoError::WorkExhausted { .. } => GeoError::WorkExhausted {
                limit: self.limits.max_work_items,
            },
            GeoError::MemoryExhausted { .. } => GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            },
            error => error,
        }
    }
    pub(super) fn policy(&self) -> Result<ExecutionPolicy, GeoError> {
        ExecutionPolicy::new(ExecutionLimits {
            max_work_items: self.limits.max_work_items.saturating_sub(self.work).max(1),
            max_workspace_bytes: self.limits.max_workspace_bytes,
            ..self.execution
        })
    }

    /// Sequential caller phases may borrow the already admitted parent arena.
    /// The parent retains its heap while this budget admits only local growth.
    pub(super) fn borrow_context(&mut self, context: &MetricContext) {
        self.execution = *context.policy().limits();
        self.integer_scratch = context.integer_scratch().cloned();
        self.borrowed_scratch = self.integer_scratch.is_some();
    }
}

pub(super) fn build(
    grid: CubeHilbertQ62V1,
    levels: CoverLevels,
    limits: MixedCoverLimits,
    budget: &mut CoverBudget<'_>,
    mut classify: impl FnMut(CellId, &mut CoverBudget<'_>) -> Result<Classification, GeoError>,
) -> Result<CellCover, GeoError> {
    let mut output = Vec::new();
    let profile = grid.profile_id();
    // At most six roots plus three pending siblings at each of thirty levels.
    // The shared work list therefore remains in its checked inline capacity.
    let mut pending = purrdf_lex::walk::WorkList::<CellId, 96>::new();
    // One work item admits the six roots' classification, one each refinement
    // (the bounded classification of a cell's four children) and one each
    // emitted cell. Work therefore tracks the cover's structure, not its leaves.
    for face in (0..6).rev() {
        budget.charge(1)?;
        pending.push(CellId::root(profile, face)?);
    }
    while let Some(cell) = pending.pop() {
        budget.visited += 1;
        let status = classify(cell, budget)?;
        if matches!(status, Classification::Outside) {
            continue;
        }
        if cell.level() >= levels.min
            && (matches!(status, Classification::Inside) || cell.level() == levels.max)
        {
            emit(cell, budget, &mut output)?;
            coalesce(&mut output, levels.min)?;
        } else {
            budget.charge(1)?;
            for child in cell.children()?.into_iter().rev() {
                pending.push(child);
            }
        }
    }
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > limits.max_emitted_cells {
        return Err(GeoError::CoverCellsExhausted {
            limit: limits.max_emitted_cells,
        });
    }
    // Disjoint DFS subtrees are already in unsigned key order.
    let mut histogram = [0; 31];
    for cell in &output {
        budget.charge(1)?;
        histogram[usize::from(cell.level())] += 1;
    }
    budget.poll()?;
    Ok(CellCover {
        retained_bytes: u64::try_from(output.capacity())
            .unwrap_or(u64::MAX)
            .saturating_mul(size_of::<CellId>() as u64),
        cells: output,
        levels,
        profile,
        law: CoverLawId::native(),
        receipt: budget.receipt(histogram),
    })
}
fn coalesce(output: &mut Vec<CellId>, minimum: u8) -> Result<(), GeoError> {
    while output.len() >= 4 {
        let start = output.len() - 4;
        let first = output[start];
        if first.level() <= minimum {
            break;
        }
        let parent = first.parent()?;
        if !output[start..].iter().copied().eq(parent.children()?) {
            break;
        }
        output.truncate(start);
        output.push(parent);
    }
    Ok(())
}
fn emit(
    cell: CellId,
    budget: &mut CoverBudget<'_>,
    output: &mut Vec<CellId>,
) -> Result<(), GeoError> {
    if output.len() == output.capacity() {
        let old = output.capacity();
        let new = old.saturating_mul(2).max(16);
        let bytes = u64::try_from(new.checked_mul(size_of::<CellId>()).ok_or(
            GeoError::MemoryExhausted {
                limit: budget.limits.max_workspace_bytes,
            },
        )?)
        .map_err(|_| GeoError::MemoryExhausted {
            limit: budget.limits.max_workspace_bytes,
        })?;
        budget.reserve(bytes)?;
        output
            .try_reserve_exact(new - old)
            .map_err(|_| GeoError::MemoryExhausted {
                limit: budget.limits.max_workspace_bytes,
            })?;
        budget.retained -=
            u64::try_from(old * size_of::<CellId>()).expect("previously admitted capacity");
    }
    output.push(cell);
    Ok(())
}

pub(super) struct Footprint {
    axis: [Rat; 3],
    radius: Rat,
}
impl Footprint {
    #[cfg(test)]
    pub(super) fn new(cell: CellId) -> Result<Self, GeoError> {
        Self::new_with(cell, None)
    }
    pub(super) fn new_admitted(
        cell: CellId,
        budget: &mut CoverBudget<'_>,
    ) -> Result<Self, GeoError> {
        Self::new_with(cell, Some(budget))
    }
    fn new_with(cell: CellId, mut budget: Option<&mut CoverBudget<'_>>) -> Result<Self, GeoError> {
        let (mut i, mut j) = coordinates(cell);
        if cell.face() & 1 == 1 {
            core::mem::swap(&mut i, &mut j);
        }
        let n = 1_u64 << cell.level();
        let [u0, u1, v0, v1] = [i, i + 1, j, j + 1].map(|k| super::warp_numerator(k, n));
        let raw_denominator = 2 * super::warp_denominator(n);
        let u = u0 + u1;
        let v = v0 + v1;
        let radius = footprint_ratio(u1 - u0 + v1 - v0, raw_denominator, &mut budget)?;
        let guard = footprint_ratio(1, 1_u128 << 59, &mut budget)?;
        let radius = match &mut budget {
            Some(budget) => {
                budget.rational(ExactOperation::RationalAdd, &[&radius, &guard], 1, || {
                    Ok(radius.add(&guard))
                })?
            }
            None => radius.add(&guard),
        };
        let d = i128::try_from(raw_denominator).expect("bounded cube midpoint denominator");
        let raw_axis = match cell.face() {
            0 => [d, u, v],
            1 => [-u, d, v],
            2 => [-u, -v, d],
            3 => [-d, -v, -u],
            4 => [v, -d, -u],
            5 => [v, u, -d],
            _ => return Err(GeoError::InvalidCellId(cell.key())),
        };
        let axis = [
            footprint_ratio(raw_axis[0], raw_denominator, &mut budget)?,
            footprint_ratio(raw_axis[1], raw_denominator, &mut budget)?,
            footprint_ratio(raw_axis[2], raw_denominator, &mut budget)?,
        ];
        Ok(Self { axis, radius })
    }
    pub(super) fn point(&self, budget: &mut CoverBudget<'_>) -> Result<LonLat, GeoError> {
        let [x, y, z] = &self.axis;
        if x == &Rat::zero() && y == &Rat::zero() {
            return LonLat::new(
                Rat::zero(),
                Rat::from_i64(if z > &Rat::zero() { 90 } else { -90 }),
            );
        }
        let (mut lon, lat) = budget.math(|math, progress| {
            let x = fixed_from_rat(x, math)?;
            let y = fixed_from_rat(y, math)?;
            let z = fixed_from_rat(z, math)?;
            progress.math_poll(math)?;
            let horizontal = x.mul(&x, math)?.add(&y.mul(&y, math)?, math)?.sqrt(math)?;
            progress.math_poll(math)?;
            let pi = FixedInterval::pi(math)?;
            let degrees = FixedInterval::from_i64(180, math)?.div(&pi, math)?;
            progress.math_poll(math)?;
            let longitude = FixedInterval::atan2(&y, &x, math)?.mul(&degrees, math)?;
            progress.math_poll(math)?;
            let latitude = FixedInterval::atan2(&z, &horizontal, math)?.mul(&degrees, math)?;
            progress.math_poll(math)?;
            Ok((
                rounded(&longitude, 15, math, progress)?,
                rounded(&latitude, 15, math, progress)?,
            ))
        })?;
        if lon == Rat::from_i64(180) {
            lon = Rat::from_i64(-180);
        }
        LonLat::new(lon, lat)
    }
}
fn footprint_ratio(
    numerator: i128,
    denominator: u128,
    budget: &mut Option<&mut CoverBudget<'_>>,
) -> Result<Rat, GeoError> {
    let evaluate = || {
        Ok(
            Rat::new(Int::from_i128(numerator), Int::from_u128(denominator))
                .expect("positive frozen footprint denominator"),
        )
    };
    match budget {
        Some(budget) => {
            let cost = ExactArithmeticCost::for_rational_operands(
                ExactOperation::RationalReduce,
                [(
                    u64::from(128 - numerator.unsigned_abs().leading_zeros()),
                    u64::from(128 - denominator.leading_zeros()),
                )],
                1,
            )
            .ok_or(GeoError::ArithmeticOverflow("footprint ratio admission"))?;
            budget.exact(cost, evaluate)
        }
        None => evaluate(),
    }
}
fn coordinates(cell: CellId) -> (u64, u64) {
    let level = cell.level();
    let path = (cell.key() & ((1_u64 << 61) - 1)) >> (61 - 2 * level);
    let (mut x, mut y) = (0, 0);
    let mut size = 1;
    for shift in 0..level {
        match (path >> (2 * shift)) & 3 {
            0 => core::mem::swap(&mut x, &mut y),
            1 => y += size,
            2 => {
                x += size;
                y += size;
            }
            _ => {
                (x, y) = (2 * size - 1 - y, size - 1 - x);
            }
        }
        size *= 2;
    }
    (x, y)
}

fn pi_lower() -> Rat {
    decimal("3.14159265358979323846264338327950288419716939937510")
}
fn pi_upper() -> Rat {
    decimal("3.14159265358979323846264338327950288419716939937511")
}
fn axis_guard() -> Rat {
    pi_upper()
        .mul(&decimal("0.000000000000001"))
        .div(&Rat::from_i64(180))
        .expect("positive")
}
fn rounded(
    value: &FixedInterval,
    places: u32,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, MathError> {
    let (low, high) = value.round_decimal(places, math)?;
    if low != high {
        return Err(MathError::PrecisionExhausted);
    }
    math_quantized_decimal(&low, places, math, progress)
}

pub(super) fn native_reference(profile: NativeGridProfile) -> GeographicReference {
    match profile {
        NativeGridProfile::Wgs84 => GeographicReference::wgs84(),
        NativeGridProfile::Cgcs2000 => GeographicReference::cgcs2000(),
    }
}
pub(super) struct ChartBounds {
    pub(super) axis: LonLat,
    pub(super) south: Rat,
    pub(super) north: Rat,
    pub(super) longitudes: Vec<(Rat, Rat)>,
}
pub(in crate::cells) mod chart;

use chart::classify_box;

mod disk;
#[cfg(test)]
pub(in crate::cells) mod tests;

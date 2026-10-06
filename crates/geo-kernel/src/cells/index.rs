// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compact ordered point buckets with exact public-law refinement.

use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_into, hex::Digest32};

use super::cover::{CoverBudget, native_reference};
use super::{CellId, CoverLevels, CubeHilbertQ62V1, MixedCoverLimits, validate_level};
use crate::context::WorkProgress;
use crate::geodesic::PreparedGeodesic;
use crate::{
    GeoBindingId, GeoError, GeographicReference, LonLat, Metres, MetricWorkObserver,
    XsdDoubleMetres,
};
use purrdf_xsd::math::PreparedBinary64;

const INDEX_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/point-cell-index/v1");

/// A stable caller key and an original exact point in the declared target reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointIndexPoint {
    /// Stable unique application key; never inferred from an input row position.
    pub key: u64,
    /// Original exact target-reference coordinate.
    pub point: LonLat,
}

/// Checked point-index construction and complete-result admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointIndexLimits {
    /// Maximum complete input/result point count.
    pub max_points: u64,
    /// Aggregate assignment or refinement work admission.
    pub max_work_items: u64,
    /// Greatest simultaneously retained scratch/index/result workspace bytes.
    pub max_workspace_bytes: u64,
}
impl PointIndexLimits {
    /// Ordinary point-index admission.
    pub const DEFAULT: Self = Self {
        max_points: 131_072,
        max_work_items: 262_144,
        max_workspace_bytes: 64 * 1024 * 1024,
    };
}
purrdf_hash::default_from_new!(PointIndexLimits => defaults);
impl PointIndexLimits {
    const fn defaults() -> Self {
        Self::DEFAULT
    }
}

/// Content identity of grid, resolution, reference, original exact source content
/// and explicit conversion identity. Admission limits are excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointCellIndexId(Digest32);
impl PointCellIndexId {
    /// Fixed-width index content digest.
    #[must_use]
    pub const fn digest(self) -> Digest32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    cell: CellId,
    key: u64,
    point: LonLat,
}

/// A reusable immutable index sorted by cell and then stable caller key.
///
/// The target reference must be the exact native grid reference. Provider or
/// other-reference inputs must first undergo an explicit operation, whose
/// binding identity is supplied as `conversion` and enters the index identity.
#[derive(Clone, Debug)]
pub struct PointCellIndex {
    grid: CubeHilbertQ62V1,
    level: u8,
    reference: GeographicReference,
    conversion: Option<GeoBindingId>,
    id: PointCellIndexId,
    entries: Vec<Entry>,
    retained_bytes: u64,
    prepared: PreparedGeodesic,
    arithmetic: PreparedBinary64,
}
impl PartialEq for PointCellIndex {
    fn eq(&self, other: &Self) -> bool {
        self.grid == other.grid
            && self.level == other.level
            && self.reference == other.reference
            && self.conversion == other.conversion
            && self.id == other.id
            && self.entries == other.entries
    }
}
impl Eq for PointCellIndex {}
impl PointCellIndex {
    /// Consume exact target-reference points and build canonical ordered buckets.
    ///
    /// Input order does not affect identity. Original source coordinate spelling
    /// as exact rational values is retained, including seam/pole longitudes.
    ///
    /// # Errors
    /// Refuses mismatched reference, duplicate caller keys, unsupported level,
    /// and insufficient complete input/work/workspace admission.
    pub fn new(
        grid: CubeHilbertQ62V1,
        level: u8,
        reference: GeographicReference,
        conversion: Option<GeoBindingId>,
        points: Vec<PointIndexPoint>,
        limits: PointIndexLimits,
    ) -> Result<Self, GeoError> {
        Self::build(grid, level, reference, conversion, points, limits, None)
    }
    /// Construct canonical point buckets with bounded governor/cancellation charging.
    /// # Errors
    /// Adds observer refusal to the complete construction contract.
    pub fn new_metered(
        grid: CubeHilbertQ62V1,
        level: u8,
        reference: GeographicReference,
        conversion: Option<GeoBindingId>,
        points: Vec<PointIndexPoint>,
        limits: PointIndexLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::build(
            grid,
            level,
            reference,
            conversion,
            points,
            limits,
            Some(observer),
        )
    }
    fn build(
        grid: CubeHilbertQ62V1,
        level: u8,
        reference: GeographicReference,
        conversion: Option<GeoBindingId>,
        mut points: Vec<PointIndexPoint>,
        limits: PointIndexLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        validate_level(level)?;
        validate_limits(limits)?;
        let metered = observer.is_some();
        let mut progress = WorkProgress::integer(observer);
        progress.initial()?;
        let mut budget = CoverBudget::from_progress(index_cover_limits(limits), progress, metered)?;
        if reference != native_reference(grid.profile) {
            return Err(GeoError::PointIndexReferenceMismatch);
        }
        let count = u64::try_from(points.len()).map_err(|_| GeoError::OutputExhausted {
            limit: limits.max_points,
        })?;
        if count > limits.max_points {
            return Err(GeoError::OutputExhausted {
                limit: limits.max_points,
            });
        }
        if count > limits.max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: limits.max_work_items,
            });
        }
        let source = points.iter().try_fold(0_u64, |sum, point| {
            budget.charge(1)?;
            sum.checked_add(point_bytes(&point.point))
                .ok_or(GeoError::MemoryExhausted {
                    limit: limits.max_workspace_bytes,
                })
        })?;
        let retained = source
            .checked_add(count.checked_mul(size_of::<Entry>() as u64).ok_or(
                GeoError::MemoryExhausted {
                    limit: limits.max_workspace_bytes,
                },
            )?)
            .ok_or(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            })?;
        // Both input and destination vectors coexist while ownership is moved;
        // the 32x exact-coordinate reservation also bounds hash spelling bridges.
        let peak = source
            .checked_mul(32)
            .and_then(|value| value.checked_add(count.saturating_mul(size_of::<Entry>() as u64)))
            .and_then(|value| {
                value.checked_add(
                    u64::try_from(points.capacity())
                        .unwrap_or(u64::MAX)
                        .saturating_mul(size_of::<PointIndexPoint>() as u64),
                )
            })
            .and_then(|value| value.checked_add(65_536))
            .ok_or(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            })?;
        if peak > limits.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            });
        }
        budget.reserve(peak.saturating_sub(65_536))?;
        purrdf_lex::walk::try_sort_unstable_by(&mut points, |a, b| {
            budget.charge(1)?;
            Ok(a.key.cmp(&b.key))
        })?;
        for pair in points.windows(2) {
            budget.charge(1)?;
            if pair[0].key == pair[1].key {
                return Err(GeoError::DuplicatePointKey(pair[0].key));
            }
        }
        let id = index_identity(grid, level, &reference, conversion, &points, &mut budget)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(points.len())
            .map_err(|_| GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            })?;
        for PointIndexPoint { key, point } in points {
            entries.push(Entry {
                cell: grid.assign_admitted(&point, level, &mut budget)?,
                key,
                point,
            });
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut entries, |a, b| {
            budget.charge(1)?;
            Ok((a.cell.key(), a.key).cmp(&(b.cell.key(), b.key)))
        })?;
        let mut context = budget.context(reference.clone())?;
        let preparation = if budget.metered {
            PreparedGeodesic::prepare_metered(reference.clone(), &mut context, &mut budget.nested())
        } else {
            PreparedGeodesic::prepare(reference.clone(), &mut context)
        };
        budget.context_receipt(&context)?;
        let prepared = preparation.map_err(|error| budget.invocation_error(error))?;
        let arithmetic = context
            .prepared_arithmetic()
            .ok_or(GeoError::ArithmeticOverflow("prepared index arithmetic"))?;
        let retained = retained
            .saturating_add(prepared.retained_workspace_bytes())
            .saturating_add(arithmetic.workspace_bytes() as u64);
        if retained > limits.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            });
        }
        budget.poll()?;
        Ok(Self {
            grid,
            level,
            reference,
            conversion,
            id,
            entries,
            retained_bytes: retained,
            prepared,
            arithmetic,
        })
    }
    /// Actual retained index, source-coordinate and immutable coefficient storage.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Canonical content identity, independent of admission limits and input order.
    #[must_use]
    pub const fn id(&self) -> PointCellIndexId {
        self.id
    }
    /// Storage resolution.
    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }
    /// Exact declared native target reference.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }
    /// Explicit source-to-target operation identity, when supplied.
    #[must_use]
    pub const fn conversion(&self) -> Option<GeoBindingId> {
        self.conversion
    }
    /// Stored point count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether the complete source is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Scan conservative physical cover intervals and refine true unrounded distance.
    ///
    /// Results are sorted by caller key. No partial result is returned on refusal.
    ///
    /// # Errors
    /// Refuses negative physical radius, incomplete cover/refinement and limits.
    pub fn search_physical(
        &self,
        center: &LonLat,
        radius: &Metres,
        cover_limits: MixedCoverLimits,
        limits: PointIndexLimits,
    ) -> Result<Vec<u64>, GeoError> {
        self.search(
            center,
            SearchThreshold::Physical(radius),
            cover_limits,
            limits,
            None,
        )
    }
    /// Scan padded reported-threshold cover intervals and refine precisely the
    /// public reported-double comparator, including numeric promotion.
    ///
    /// # Errors
    /// Refuses incomplete cover/refinement and limits. Negative thresholds return empty.
    pub fn search_reported(
        &self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        cover_limits: MixedCoverLimits,
        limits: PointIndexLimits,
    ) -> Result<Vec<u64>, GeoError> {
        self.search(
            center,
            SearchThreshold::Reported(threshold),
            cover_limits,
            limits,
            None,
        )
    }
    /// Search the physical predicate with bounded governor/cancellation charging.
    /// # Errors
    /// Adds observer refusal to the complete search contract.
    pub fn search_physical_metered(
        &self,
        center: &LonLat,
        radius: &Metres,
        cover_limits: MixedCoverLimits,
        limits: PointIndexLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Vec<u64>, GeoError> {
        self.search(
            center,
            SearchThreshold::Physical(radius),
            cover_limits,
            limits,
            Some(observer),
        )
    }
    /// Search the reported-double predicate with bounded charging.
    /// # Errors
    /// Adds observer refusal to the complete search contract.
    pub fn search_reported_metered(
        &self,
        center: &LonLat,
        threshold: XsdDoubleMetres,
        cover_limits: MixedCoverLimits,
        limits: PointIndexLimits,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Vec<u64>, GeoError> {
        self.search(
            center,
            SearchThreshold::Reported(threshold),
            cover_limits,
            limits,
            Some(observer),
        )
    }
    fn search(
        &self,
        center: &LonLat,
        threshold: SearchThreshold<'_>,
        mut cover_limits: MixedCoverLimits,
        limits: PointIndexLimits,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Vec<u64>, GeoError> {
        validate_limits(limits)?;
        let metered = observer.is_some();
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        let baseline = self
            .retained_bytes
            .checked_add(point_bytes(center).saturating_mul(32))
            .ok_or(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            })?;
        cover_limits.max_workspace_bytes = cover_limits
            .max_workspace_bytes
            .min(limits.max_workspace_bytes)
            .checked_sub(baseline)
            .filter(|available| *available > 0)
            .ok_or(GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            })?;
        cover_limits.max_work_items = cover_limits.max_work_items.min(limits.max_work_items);
        progress.charge_counts(0, baseline)?;
        if threshold.is_physical_zero() {
            // Positive ellipsoid axes prove D=0 exactly when the original
            // physical positions agree. Identical pole/seam positions have
            // identical assignment at every level. This is an index candidate
            // reduction only; the standalone zero-radius cover keeps its
            // complete frozen cap-classifier output.
            cover_limits.max_workspace_bytes = cover_limits
                .max_workspace_bytes
                .checked_add(baseline)
                .ok_or(GeoError::MemoryExhausted {
                    limit: limits.max_workspace_bytes,
                })?;
            let mut assignment = CoverBudget::from_progress(cover_limits, progress, metered)?;
            assignment.reserve(baseline)?;
            let cell = self
                .grid
                .assign_admitted(center, self.level, &mut assignment)?;
            assignment.poll()?;
            let work = assignment.work_items();
            let peak = assignment.workspace_peak();
            let mut budget = CoverBudget::from_progress(
                index_cover_limits(limits),
                assignment.progress,
                metered,
            )?;
            budget.charge(work)?;
            budget.reserve(baseline)?;
            budget.scratch(peak.saturating_sub(baseline))?;
            return self.scan(center, &[cell], threshold, limits, &mut budget);
        }
        let levels = CoverLevels::new(0, self.level)?;
        let covered = if metered {
            let mut nested = progress.nested(0, baseline, baseline);
            match threshold {
                SearchThreshold::Physical(radius) => self.grid.cover_disk_mixed_metered(
                    center,
                    radius,
                    levels,
                    cover_limits,
                    &mut nested,
                ),
                SearchThreshold::Reported(value) => self.grid.cover_reported_disk_mixed_metered(
                    center,
                    value,
                    levels,
                    cover_limits,
                    &mut nested,
                ),
            }
        } else {
            match threshold {
                SearchThreshold::Physical(radius) => {
                    self.grid
                        .cover_disk_mixed(center, radius, levels, cover_limits)
                }
                SearchThreshold::Reported(value) => {
                    self.grid
                        .cover_reported_disk_mixed(center, value, levels, cover_limits)
                }
            }
        };
        let cover = covered.map_err(|error| match error {
            GeoError::MemoryExhausted { .. } => GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            },
            error => error,
        })?;
        let mut budget = CoverBudget::from_progress(index_cover_limits(limits), progress, metered)?;
        budget.charge(cover.receipt().work_items)?;
        budget.reserve(baseline.checked_add(cover.retained_bytes()).ok_or(
            GeoError::MemoryExhausted {
                limit: limits.max_workspace_bytes,
            },
        )?)?;
        budget.scratch(
            cover
                .receipt()
                .workspace_peak
                .saturating_sub(cover.retained_bytes()),
        )?;
        self.scan(center, cover.cells(), threshold, limits, &mut budget)
    }
    fn scan(
        &self,
        center: &LonLat,
        cells: &[CellId],
        threshold: SearchThreshold<'_>,
        limits: PointIndexLimits,
        budget: &mut CoverBudget<'_>,
    ) -> Result<Vec<u64>, GeoError> {
        let mut result = Vec::new();
        let prepared_center = if threshold.is_physical_zero() {
            None
        } else {
            let mut context = budget.context(self.reference.clone())?;
            context.set_prepared_arithmetic(self.arithmetic.clone());
            let preparation = if budget.metered {
                self.prepared
                    .prepare_point_metered(center, &mut context, &mut budget.nested())
            } else {
                self.prepared.prepare_point(center, &mut context)
            };
            budget.context_receipt(&context)?;
            let center = preparation.map_err(|error| budget.invocation_error(error))?;
            budget.reserve(center.retained_workspace_bytes())?;
            Some(center)
        };
        for cell in cells {
            budget.charge(1)?;
            let range = cell.descendant_range(self.level)?;
            let low = lower_bound(&self.entries, range.min(), false, budget)?;
            let high = lower_bound(&self.entries, range.max(), true, budget)?;
            for entry in &self.entries[low..high] {
                budget.charge(1)?;
                let mut context = budget.context(self.reference.clone())?;
                context.set_prepared_arithmetic(self.arithmetic.clone());
                let accepted = match (threshold, prepared_center.as_ref(), budget.metered) {
                    (SearchThreshold::Physical(radius), None, true) => {
                        self.prepared.within_physical_metered(
                            center,
                            &entry.point,
                            radius,
                            &mut context,
                            &mut budget.nested(),
                        )
                    }
                    (SearchThreshold::Physical(radius), None, false) => self
                        .prepared
                        .within_physical(center, &entry.point, radius, &mut context),
                    (SearchThreshold::Physical(radius), Some(center), true) => {
                        self.prepared.within_physical_from_prepared_metered(
                            center,
                            &entry.point,
                            radius,
                            &mut context,
                            &mut budget.nested(),
                        )
                    }
                    (SearchThreshold::Physical(radius), Some(center), false) => self
                        .prepared
                        .within_physical_from_prepared(center, &entry.point, radius, &mut context),
                    (SearchThreshold::Reported(value), Some(center), true) => self
                        .prepared
                        .distance_from_prepared_metered(
                            center,
                            &entry.point,
                            &mut context,
                            &mut budget.nested(),
                        )
                        .map(|distance| distance.within_reported(value)),
                    (SearchThreshold::Reported(value), Some(center), false) => self
                        .prepared
                        .distance_from_prepared(center, &entry.point, &mut context)
                        .map(|distance| distance.within_reported(value)),
                    (SearchThreshold::Reported(_), None, _) => {
                        unreachable!("reported comparison always prepares its original center")
                    }
                };
                budget.context_receipt(&context)?;
                if accepted.map_err(|error| budget.invocation_error(error))? {
                    if result.len() as u64 >= limits.max_points {
                        return Err(GeoError::OutputExhausted {
                            limit: limits.max_points,
                        });
                    }
                    if result.len() == result.capacity() {
                        let old = result.capacity();
                        let new = old.saturating_mul(2).max(16);
                        budget.reserve((new as u64).saturating_mul(8))?;
                        result.try_reserve_exact(new - old).map_err(|_| {
                            GeoError::MemoryExhausted {
                                limit: limits.max_workspace_bytes,
                            }
                        })?;
                        budget.release((old as u64).saturating_mul(8))?;
                    }
                    result.push(entry.key);
                }
            }
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut result, |a, b| {
            budget.charge(1)?;
            Ok(a.cmp(b))
        })?;
        budget.poll()?;
        debug_assert!(
            budget.work_items() <= limits.max_work_items
                && budget.workspace_peak() <= limits.max_workspace_bytes
        );
        Ok(result)
    }
}
fn validate_limits(limits: PointIndexLimits) -> Result<(), GeoError> {
    if limits.max_points == 0 || limits.max_work_items == 0 || limits.max_workspace_bytes == 0 {
        return Err(GeoError::InvalidExecutionPolicy(
            "point-index limits must be positive",
        ));
    }
    Ok(())
}
fn point_bytes(point: &LonLat) -> u64 {
    [point.longitude(), point.latitude()]
        .into_iter()
        .map(|value| {
            u64::try_from(value.numerator().allocated_bytes())
                .unwrap_or(u64::MAX)
                .saturating_add(
                    u64::try_from(value.denominator().allocated_bytes()).unwrap_or(u64::MAX),
                )
        })
        .fold(0, u64::saturating_add)
}
fn index_identity(
    grid: CubeHilbertQ62V1,
    level: u8,
    reference: &GeographicReference,
    conversion: Option<GeoBindingId>,
    points: &[PointIndexPoint],
    budget: &mut CoverBudget<'_>,
) -> Result<PointCellIndexId, GeoError> {
    let mut hash = Hasher::new();
    for field in [
        INDEX_DOMAIN.as_bytes(),
        b"canonical-caller-key-exact-target-content;ordered-cell-key-v1",
    ] {
        frame_le_into(&mut hash, field);
    }
    frame_le_into(&mut hash, grid.profile_id().digest().as_bytes());
    frame_le_into(&mut hash, &[level]);
    frame_le_into(&mut hash, reference.id().digest().as_bytes());
    frame_le_into(&mut hash, &[u8::from(conversion.is_some())]);
    if let Some(id) = conversion {
        frame_le_into(&mut hash, id.digest().as_bytes());
    }
    for point in points {
        budget.charge(1)?;
        frame_le_into(&mut hash, &point.key.to_be_bytes());
        for value in [point.point.longitude(), point.point.latitude()] {
            let cost = crate::numerical::rational_cost(
                purrdf_xsd::integer::ExactOperation::DecimalRender,
                &[value],
                2,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "point-index source identity admission",
            ))?;
            budget.exact(cost, || {
                frame_le_into(&mut hash, value.numerator().to_string().as_bytes());
                frame_le_into(&mut hash, value.denominator().to_string().as_bytes());
                Ok(())
            })?;
        }
    }
    Ok(PointCellIndexId(Digest32::new(*hash.finalize().as_bytes())))
}
fn index_cover_limits(limits: PointIndexLimits) -> MixedCoverLimits {
    MixedCoverLimits {
        max_emitted_cells: limits.max_points,
        max_work_items: limits.max_work_items,
        max_workspace_bytes: limits.max_workspace_bytes,
    }
}

#[derive(Clone, Copy)]
enum SearchThreshold<'a> {
    Physical(&'a Metres),
    Reported(XsdDoubleMetres),
}
impl SearchThreshold<'_> {
    fn is_physical_zero(self) -> bool {
        matches!(self, Self::Physical(radius) if radius.exact().signum() == 0)
    }
}
fn lower_bound(
    entries: &[Entry],
    key: u64,
    closed: bool,
    budget: &mut CoverBudget<'_>,
) -> Result<usize, GeoError> {
    let (mut low, mut high) = (0, entries.len());
    while low < high {
        budget.charge(1)?;
        let mid = low + (high - low) / 2;
        let current = entries[mid].cell.key();
        if current < key || (closed && current == key) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    Ok(low)
}

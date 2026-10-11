// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict fixed and mixed cover requests over the one native classifier.

use super::{
    GeoCallError, PointInput, decode::Decode, encode, invocation::Invocation, output, request,
};
use purrdf_geo_kernel::{
    GeoProfile, Metres, PreparationBudget, PreparedRegion, XsdDoubleMetres,
    cells::{
        ClosedBox, CoverLevels, CubeHilbertQ62V1, FixedCoverLimits, MixedCoverLimits,
        NativeGridProfile,
    },
};
use purrdf_lex::json::{
    Object, Value,
    record::{DecodeError, Record},
};

/// The fixed or mixed geometric resolution contract.
#[derive(Clone, Copy, Debug)]
pub enum CoverResolution {
    /// One fixed storage level; admission counts every logical cell.
    Fixed(u8, FixedCoverLimits),
    /// A strict minimum/maximum interval; admission counts final emitted cells.
    Mixed(CoverLevels, MixedCoverLimits),
}

/// Exact closed shape to cover.
#[derive(Clone, Debug)]
pub enum CoverShape {
    /// A physical radius in metres.
    Physical(PointInput, Metres),
    /// A radius under the reported-double comparison law.
    Reported(PointInput, XsdDoubleMetres),
    /// A closed longitude/latitude box, including wrap and meridian cases.
    Box(ClosedBox),
    /// A prepared polygon/multipolygon carrier, retaining its exact source terms.
    Region(super::GeometryInput),
    /// Complement of the complete prepared polygon union, retaining source terms.
    Complement(super::GeometryInput),
    /// The empty surface region.
    Empty,
    /// The whole identified geographic surface.
    Whole,
}

/// A complete conservative native-grid cover request.
#[derive(Clone, Debug)]
pub struct CoverRequest {
    /// Exact native geographic grid reference.
    pub grid: NativeGridProfile,
    /// Exact source geometry and comparison law.
    pub shape: CoverShape,
    /// Frozen geometric levels and separately admitted resources.
    pub resolution: CoverResolution,
    /// External-store resolution used to encode descendant ranges.
    pub stored_level: u8,
}

pub(super) fn decode(
    fields: &mut Record<'_>,
    operation: &str,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<CoverRequest, GeoCallError> {
    let grid = request::grid(fields)?;
    let shape = match operation {
        "cover-disk" => CoverShape::Physical(
            fields.required_with("center", |value| {
                PointInput::from_value_native(value, decode)
            })?,
            Metres::new(fields.required_with("radius_metres", |value| decode.decimal(value))?),
        ),
        "cover-reported" => CoverShape::Reported(
            fields.required_with("center", |value| {
                PointInput::from_value_native(value, decode)
            })?,
            fields.required_with("threshold_metres", |value| {
                request::threshold(value, decode)
            })?,
        ),
        "cover-region" => {
            match fields.tag("region_kind", &["geometry", "complement", "empty", "whole"])? {
                "geometry" => CoverShape::Region(fields.required_with("geometry", |value| {
                    super::GeometryInput::from_value_native(value, decode)
                })?),
                "complement" => {
                    CoverShape::Complement(fields.required_with("geometry", |value| {
                        super::GeometryInput::from_value_native(value, decode)
                    })?)
                }
                "empty" => CoverShape::Empty,
                _ => CoverShape::Whole,
            }
        }
        _ => CoverShape::Box(fields.required_with("box", |value| closed_box(value, decode))?),
    };
    let resolution = match fields.tag("mode", &["fixed", "mixed"])? {
        "fixed" => CoverResolution::Fixed(
            fields.required_native("level")?,
            fields
                .optional_with("limits", |value| fixed_limits(value, decode))?
                .unwrap_or_default(),
        ),
        _ => CoverResolution::Mixed(
            fields.required_with("levels", |value| levels(value, decode))?,
            fields
                .optional_with("limits", |value| mixed_limits(value, decode))?
                .unwrap_or_default(),
        ),
    };
    Ok(CoverRequest {
        grid,
        shape,
        resolution,
        stored_level: fields.optional_native("stored_level")?.unwrap_or(30),
    })
}

fn levels(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<CoverLevels, GeoCallError> {
    let mut fields = decode.record(value, "cover geometric levels")?;
    let levels = CoverLevels::new(
        fields.required_native("min")?,
        fields.required_native("max")?,
    )?;
    fields.deny_unknown()?;
    Ok(levels)
}
fn closed_box(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<ClosedBox, GeoCallError> {
    let mut fields = decode.record(value, "closed geographic box")?;
    let shape = decode.closed_box(
        fields.required_with("west", |value| decode.decimal(value))?,
        fields.required_with("south", |value| decode.decimal(value))?,
        fields.required_with("east", |value| decode.decimal(value))?,
        fields.required_with("north", |value| decode.decimal(value))?,
    )?;
    fields.deny_unknown()?;
    Ok(shape)
}
fn fixed_limits(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<FixedCoverLimits, DecodeError> {
    let mut fields = decode.record(value, "fixed cover admission")?;
    let limits = FixedCoverLimits {
        max_logical_cells: fields
            .optional_native("max_logical_cells")?
            .unwrap_or(FixedCoverLimits::DEFAULT.max_logical_cells),
        max_work_items: fields
            .optional_native("max_work_items")?
            .unwrap_or(FixedCoverLimits::DEFAULT.max_work_items),
        max_workspace_bytes: fields
            .optional_native("max_workspace_bytes")?
            .unwrap_or(FixedCoverLimits::DEFAULT.max_workspace_bytes),
    };
    fields.deny_unknown()?;
    Ok(limits)
}
pub(super) fn mixed_limits(
    value: &Value,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<MixedCoverLimits, DecodeError> {
    let mut fields = decode.record(value, "mixed cover admission")?;
    let limits = MixedCoverLimits {
        max_emitted_cells: fields
            .optional_native("max_emitted_cells")?
            .unwrap_or(MixedCoverLimits::DEFAULT.max_emitted_cells),
        max_work_items: fields
            .optional_native("max_work_items")?
            .unwrap_or(MixedCoverLimits::DEFAULT.max_work_items),
        max_workspace_bytes: fields
            .optional_native("max_workspace_bytes")?
            .unwrap_or(MixedCoverLimits::DEFAULT.max_workspace_bytes),
    };
    fields.deny_unknown()?;
    Ok(limits)
}
pub(super) fn admitted_mixed_in_policy(
    policy: purrdf_geo_kernel::ExecutionPolicy,
    mut limits: MixedCoverLimits,
) -> MixedCoverLimits {
    let limits_profile = policy.limits();
    limits.max_emitted_cells = limits
        .max_emitted_cells
        .min(limits_profile.max_output_elements);
    limits.max_work_items = limits.max_work_items.min(limits_profile.max_work_items);
    limits.max_workspace_bytes = limits
        .max_workspace_bytes
        .min(limits_profile.max_workspace_bytes);
    limits
}

pub(super) fn call(
    profile: &GeoProfile,
    request: &CoverRequest,
    budget: PreparationBudget,
    invocation: &Invocation,
    decode: &Decode<'_, '_, '_, '_>,
) -> Result<Value, GeoCallError> {
    let mut budget = invocation.track_budget(budget);
    let grid = CubeHilbertQ62V1::new(request.grid);
    let policy = profile.policy();
    let profile_limits = policy.limits();
    let prepared = if let CoverShape::Region(term) | CoverShape::Complement(term) = &request.shape {
        Some(term.prepare_in_policy_observed(profile, budget.remaining()?, invocation)?)
    } else {
        None
    };
    let complement = if matches!(&request.shape, CoverShape::Complement(_)) {
        Some(
            prepared
                .as_ref()
                .expect("prepared complement source")
                .geometry
                .region()
                .clone()
                .complement(),
        )
    } else {
        None
    };
    let region = match &request.shape {
        CoverShape::Region(_) => prepared.as_ref().map(|prepared| prepared.geometry.region()),
        CoverShape::Complement(_) => complement.as_ref(),
        CoverShape::Empty => Some(&PreparedRegion::Empty),
        CoverShape::Whole => Some(&PreparedRegion::Whole),
        _ => None,
    };
    if let Some(prepared) = &prepared {
        budget.retain(prepared.work_items, prepared.workspace_bytes)?;
    }
    let remaining = budget.remaining()?;
    let remaining_work = remaining.limits().max_work_items;
    let remaining_workspace = remaining.limits().max_workspace_bytes;
    let mut phase = invocation.phase(*budget);
    let cover = match request.resolution {
        CoverResolution::Fixed(level, mut limits) => {
            limits.max_logical_cells = limits
                .max_logical_cells
                .min(profile_limits.max_output_elements);
            limits.max_work_items = limits.max_work_items.min(remaining_work);
            limits.max_workspace_bytes = limits.max_workspace_bytes.min(remaining_workspace);
            match &request.shape {
                CoverShape::Physical(center, radius) => {
                    grid.cover_disk_fixed_metered(&center.0, radius, level, limits, &mut phase)?
                }
                CoverShape::Reported(center, threshold) => grid.cover_reported_disk_fixed_metered(
                    &center.0, *threshold, level, limits, &mut phase,
                )?,
                CoverShape::Box(shape) => {
                    grid.cover_box_fixed_metered(shape, level, limits, &mut phase)?
                }
                CoverShape::Region(_)
                | CoverShape::Complement(_)
                | CoverShape::Empty
                | CoverShape::Whole => grid.cover_region_fixed_metered(
                    region.expect("prepared region shape"),
                    level,
                    limits,
                    &mut phase,
                )?,
            }
        }
        CoverResolution::Mixed(levels, limits) => {
            let mut limits = admitted_mixed_in_policy(policy, limits);
            limits.max_work_items = limits.max_work_items.min(remaining_work);
            limits.max_workspace_bytes = limits.max_workspace_bytes.min(remaining_workspace);
            match &request.shape {
                CoverShape::Physical(center, radius) => {
                    grid.cover_disk_mixed_metered(&center.0, radius, levels, limits, &mut phase)?
                }
                CoverShape::Reported(center, threshold) => grid.cover_reported_disk_mixed_metered(
                    &center.0, *threshold, levels, limits, &mut phase,
                )?,
                CoverShape::Box(shape) => {
                    grid.cover_box_mixed_metered(shape, levels, limits, &mut phase)?
                }
                CoverShape::Region(_)
                | CoverShape::Complement(_)
                | CoverShape::Empty
                | CoverShape::Whole => grid.cover_region_mixed_metered(
                    region.expect("prepared region shape"),
                    levels,
                    limits,
                    &mut phase,
                )?,
            }
        }
    };
    budget.retain(cover.receipt().work_items, cover.retained_workspace_bytes())?;
    let count = cover.cells().len();
    let range_bytes = (count as u64)
        .checked_mul(size_of::<purrdf_geo_kernel::cells::CellRange>() as u64)
        .ok_or(purrdf_geo_kernel::GeoError::ArithmeticOverflow(
            "cover range output storage",
        ))?;
    budget.retain(count as u64, range_bytes)?;
    let cell_layout = output::array(count, output::cell()?)?;
    let range_layout = output::array(count, output::range()?)?;
    let histogram = output::array(31, output::record(1, 20)?)?;
    let receipt_layout = output::record(4, 128)?.with_child(histogram).ok_or(
        purrdf_geo_kernel::GeoError::ArithmeticOverflow("cover receipt layout"),
    )?;
    let layout = output::record(6, 256)?
        .with_child(output::record(2, 4)?)
        .and_then(|layout| layout.with_child(cell_layout))
        .and_then(|layout| layout.with_child(range_layout))
        .and_then(|layout| layout.with_child(receipt_layout))
        .ok_or(purrdf_geo_kernel::GeoError::ArithmeticOverflow(
            "cover response layout",
        ))?;
    output::admit(*budget, layout)?;
    let ranges = decode
        .storage
        .native(|memory| cover.ranges_with_memory(request.stored_level, memory))?;
    let cells = decode.array(cover.cells().iter().copied(), |cell| {
        encode::cell(cell, decode)
    })?;
    let ranges = decode.owned_array(ranges, |range| encode::range(range, decode))?;
    let histogram = decode.array(cover.receipt().level_histogram.iter(), |count| {
        decode.format(count).map(Value::String)
    })?;
    Ok(Object::new()
        .with_memory_cell(
            "profile",
            decode.format(&cover.profile().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "cover_law",
            decode.format(&cover.law_id().digest())?,
            decode.storage.memory(),
        )?
        .with_memory_cell(
            "levels",
            Object::new()
                .with_memory_cell("min", cover.levels().min(), decode.storage.memory())?
                .with_memory_cell("max", cover.levels().max(), decode.storage.memory())?,
            decode.storage.memory(),
        )?
        .with_memory_cell("cells", cells, decode.storage.memory())?
        .with_memory_cell("ranges", ranges, decode.storage.memory())?
        .with_memory_cell(
            "receipt",
            Object::new()
                .with_memory_cell(
                    "visited_nodes",
                    decode.format(&cover.receipt().visited_nodes)?,
                    decode.storage.memory(),
                )?
                .with_memory_cell(
                    "work_items",
                    decode.format(&cover.receipt().work_items)?,
                    decode.storage.memory(),
                )?
                .with_memory_cell(
                    "workspace_peak",
                    decode.format(&cover.receipt().workspace_peak)?,
                    decode.storage.memory(),
                )?
                .with_memory_cell("level_histogram", histogram, decode.storage.memory())?,
            decode.storage.memory(),
        )?
        .into())
}

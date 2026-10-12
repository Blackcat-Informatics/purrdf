// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reusable immutable ordered point indexes through the shared strict codec.

use super::{
    GeoCallError, GeoSession, PointInput, cover, decode::Decode, encode, invocation::Invocation,
    output, request,
};
use purrdf_geo_kernel::{
    GeoError, Metres, PreparationBudget,
    cells::{
        CubeHilbertQ62V1, MixedCoverLimits, PointCellIndex, PointIndexLimits, PointIndexPoint,
    },
};
use purrdf_lex::json::{
    self, Object, Value,
    record::{DecodeError, Record},
};

/// A prepared reusable index; clones share immutable ordered point buckets.
#[derive(Clone, Debug)]
pub struct GeoPointIndex {
    profile: super::GeoProfileSnapshot,
    identity: purrdf_geo_kernel::binding::GeoQueryIdentity,
    index: purrdf_lex::allocation::Shared<PointCellIndex>,
}

impl GeoSession {
    /// Build a reusable index from original exact target-reference points.
    ///
    /// # Errors
    /// Refuses unknown/duplicate fields, mismatched native reference, duplicate
    /// keys and insufficient complete-source admission. No partial index exists.
    pub fn point_index(&self, text: &str) -> Result<GeoPointIndex, GeoCallError> {
        let invocation = Invocation::new(self.profile().policy());
        let mut phase = invocation.phase(PreparationBudget::new(self.profile().policy()));
        super::decode::record(
            text,
            self.profile().policy(),
            Some(&mut phase),
            |value, decode| {
                self.point_index_value(value, decode.receipt()?, &invocation, decode)
                    .map(|(index, _)| index)
            },
        )
        .map(|(index, _)| index)
    }

    pub(super) fn point_index_value(
        &self,
        value: &Value,
        budget: PreparationBudget,
        invocation: &Invocation,
        decode: &Decode<'_, '_, '_, '_>,
    ) -> Result<(GeoPointIndex, PreparationBudget), GeoCallError> {
        let mut budget = invocation.track_budget(budget);
        self.identity()?;
        let mut fields = decode.record(value, "point index preparation version one")?;
        version(&mut fields, decode)?;
        fields.tag("operation", &["point-index"])?;
        let grid = CubeHilbertQ62V1::new(request::grid(&mut fields)?);
        let crs = request::crs(&mut fields, decode)?;
        let reference = self.profile().reference(&crs)?.clone();
        let level = fields.required_native("level")?;
        let conversion = fields.optional_with("conversion", |value| decode.crs(value))?;
        let conversion = conversion
            .map(|name| {
                let binding = self.profile().operation(&name)?;
                if binding.target() != &crs {
                    return Err(GeoError::PointIndexReferenceMismatch);
                }
                Ok(binding.chain().binding_id())
            })
            .transpose()?;
        let points = fields.required_with("points", |value| {
            decode.items(value, |value| point(value, decode))
        })?;
        let limits = admitted_limits(
            budget.remaining()?,
            fields
                .optional_with("limits", |value| limits(value, decode))?
                .unwrap_or_default(),
        );
        fields.deny_unknown()?;
        let mut work = invocation.phase(*budget);
        let index = PointCellIndex::new_metered(
            grid, level, reference, conversion, points, limits, &mut work,
        )?;
        // The following search's native admission includes retained index storage.
        // Retain construction work once; source-record storage remains cumulative.
        budget.retain(work.work, 0)?;
        let mut metadata_budget = *budget;
        metadata_budget.retain(0, index.retained_workspace_bytes())?;
        output::admit(metadata_budget, output::record(5, 256)?)?;
        Ok((
            GeoPointIndex {
                profile: self.profile_snapshot(),
                identity: self.identity()?,
                index: decode.shared(index)?,
            },
            *budget,
        ))
    }
}

impl GeoPointIndex {
    /// The same immutable native index exposed to Rust callers.
    #[must_use]
    pub fn native(&self) -> &PointCellIndex {
        &self.index
    }

    /// Stable content and reference identities, independent of admission limits.
    #[must_use]
    pub fn metadata(&self) -> Value {
        super::decode::produce(
            purrdf_geo_kernel::ExecutionPolicy::unbounded(),
            None,
            |decode| self.metadata_native(decode),
        )
        .expect("resident point-index metadata allocation")
    }

    /// Construct metadata under the selected profile's native storage limit.
    /// # Errors
    /// Returns original physical allocation or checked layout refusal.
    pub fn try_metadata(&self) -> Result<Value, GeoCallError> {
        super::decode::produce(self.profile.policy(), None, |decode| {
            self.metadata_native(decode)
        })
    }

    pub(super) fn metadata_native(
        &self,
        decode: &Decode<'_, '_, '_, '_>,
    ) -> Result<Value, GeoCallError> {
        Ok(Object::new()
            .with_memory_cell(
                "id",
                decode.format(&self.index.id().digest())?,
                decode.storage.memory(),
            )?
            .with_memory_cell("level", self.index.level(), decode.storage.memory())?
            .with_memory_cell(
                "reference",
                decode.format(&self.index.reference().id().digest())?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "conversion",
                self.index
                    .conversion()
                    .map(|id| decode.format(&id.digest()))
                    .transpose()?,
                decode.storage.memory(),
            )?
            .with_memory_cell(
                "points",
                decode.format(&self.index.len())?,
                decode.storage.memory(),
            )?
            .into())
    }

    /// Execute a strict reported or physical radius search against retained buckets.
    ///
    /// # Errors
    /// Carries typed reference, comparison and resource refusals; no partial result.
    pub fn call(&self, text: &str) -> Result<Value, GeoCallError> {
        let invocation = Invocation::new(self.profile.policy());
        let mut phase = invocation.phase(PreparationBudget::new(self.profile.policy()));
        super::decode::record(
            text,
            self.profile.policy(),
            Some(&mut phase),
            |value, decode| self.call_value(value, decode.receipt()?, &invocation, decode),
        )
        .map(|(value, _)| value)
    }

    pub(super) fn call_value(
        &self,
        value: &Value,
        budget: PreparationBudget,
        invocation: &Invocation,
        decode: &Decode<'_, '_, '_, '_>,
    ) -> Result<Value, GeoCallError> {
        let mut budget = invocation.track_budget(budget);
        let mut fields = decode.record(value, "point index search version one")?;
        version(&mut fields, decode)?;
        let operation = fields.tag(
            "operation",
            &["point-index", "search-physical", "search-reported"],
        )?;
        if operation == "point-index" {
            fields.deny_unknown()?;
            budget.retain(0, self.index.retained_workspace_bytes())?;
            output::admit(*budget, output::record(5, 256)?)?;
            return self.metadata_native(decode);
        }
        let center = fields.required_with("center", |value| {
            PointInput::from_value_native(value, decode)
        })?;
        let cover_limits = cover::admitted_mixed_in_policy(
            budget.remaining()?,
            fields
                .optional_with("cover_limits", |value| cover::mixed_limits(value, decode))?
                .unwrap_or(MixedCoverLimits::DEFAULT),
        );
        let limits = admitted_limits(
            budget.remaining()?,
            fields
                .optional_with("limits", |value| limits(value, decode))?
                .unwrap_or_default(),
        );
        let mut work = invocation.phase(*budget);
        let keys = if operation == "search-physical" {
            let radius =
                Metres::new(fields.required_with("radius_metres", |value| decode.decimal(value))?);
            fields.deny_unknown()?;
            self.index.search_physical_metered(
                &center.0,
                &radius,
                cover_limits,
                limits,
                &mut work,
            )?
        } else {
            let threshold = fields.required_with("threshold_metres", |value| {
                request::threshold(value, decode)
            })?;
            fields.deny_unknown()?;
            self.index.search_reported_metered(
                &center.0,
                threshold,
                cover_limits,
                limits,
                &mut work,
            )?
        };
        budget.retain(work.work, self.index.retained_workspace_bytes())?;
        budget.retain(
            keys.len() as u64,
            u64::try_from(keys.retained_workspace_bytes()).map_err(|_| {
                GeoError::ArithmeticOverflow("point search original output storage")
            })?,
        )?;
        output::admit(
            *budget,
            output::record(2, 64)?
                .with_child(output::array(
                    keys.len(),
                    json::OutputLayout::record(1, 20)
                        .ok_or(GeoError::ArithmeticOverflow("point key output"))?,
                )?)
                .ok_or(GeoError::ArithmeticOverflow("point index output layout"))?,
        )?;
        let keys = decode.array(keys.into_iter(), |key| {
            decode.format(&key).map(Value::String)
        })?;
        Ok(Object::new()
            .with_memory_cell(
                "index",
                decode.format(&self.index.id().digest())?,
                decode.storage.memory(),
            )?
            .with_memory_cell("keys", keys, decode.storage.memory())?
            .into())
    }

    /// Carry complete search success or refusal bytes unchanged into every host.
    #[must_use]
    pub fn call_string(&self, text: &str) -> String {
        self.response(text).0
    }

    /// Canonical response and success flag for process/ABI error channels.
    #[must_use]
    pub fn response(&self, text: &str) -> (String, bool) {
        self.try_response(text).unwrap_or_else(|error| {
            super::decode::produce(
                purrdf_geo_kernel::ExecutionPolicy::unbounded(),
                None,
                |decode| {
                    encode::response(
                        Some(&self.identity),
                        Some(self.index.id().digest()),
                        Err(error),
                        None,
                        decode,
                    )
                },
            )
            .expect("resident point-index response allocation")
        })
    }

    /// Preserve the original index and invocation account through host output.
    /// Operational refusal moves as its typed cause before envelope allocation.
    /// # Errors
    /// Returns original work, source, checked layout or physical refusal.
    pub fn try_response(&self, text: &str) -> Result<(String, bool), GeoCallError> {
        let invocation = Invocation::new(self.profile.policy());
        let mut phase = invocation.phase(PreparationBudget::new(self.profile.policy()));
        super::decode::record_outcome(
            text,
            self.profile.policy(),
            Some(&mut phase),
            |value, decode| self.call_value(value, decode.receipt()?, &invocation, decode),
            |outcome, decode| {
                encode::response(
                    Some(&self.identity),
                    Some(self.index.id().digest()),
                    outcome,
                    Some(&invocation),
                    decode,
                )
            },
        )
        .map(|(response, _)| response)
    }
}
fn version(fields: &mut Record<'_>, decode: &Decode<'_, '_, '_, '_>) -> Result<(), DecodeError> {
    if fields.required_native::<u32>("version")? != 1 {
        return Err(decode.custom("geographic request version must be 1"));
    }
    Ok(())
}
fn point(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<PointIndexPoint, GeoCallError> {
    let mut fields = decode.record(value, "stable point index entry")?;
    let key: String = fields.required_native("key")?;
    let key = key
        .parse()
        .map_err(|_| decode.custom("point key must be an unsigned decimal u64 string"))?;
    let point = fields
        .required_with("point", |value| {
            PointInput::from_value_native(value, decode)
        })?
        .0;
    fields.deny_unknown()?;
    Ok(PointIndexPoint { key, point })
}
fn limits(value: &Value, decode: &Decode<'_, '_, '_, '_>) -> Result<PointIndexLimits, DecodeError> {
    let mut fields = decode.record(value, "point index admission")?;
    let limits = PointIndexLimits {
        max_points: fields
            .optional_native("max_points")?
            .unwrap_or(PointIndexLimits::DEFAULT.max_points),
        max_work_items: fields
            .optional_native("max_work_items")?
            .unwrap_or(PointIndexLimits::DEFAULT.max_work_items),
        max_workspace_bytes: fields
            .optional_native("max_workspace_bytes")?
            .unwrap_or(PointIndexLimits::DEFAULT.max_workspace_bytes),
    };
    fields.deny_unknown()?;
    Ok(limits)
}
fn admitted_limits(
    policy: purrdf_geo_kernel::ExecutionPolicy,
    mut limits: PointIndexLimits,
) -> PointIndexLimits {
    let admitted = policy.limits();
    limits.max_points = limits.max_points.min(admitted.max_output_elements);
    limits.max_work_items = limits.max_work_items.min(admitted.max_work_items);
    limits.max_workspace_bytes = limits.max_workspace_bytes.min(admitted.max_workspace_bytes);
    limits
}

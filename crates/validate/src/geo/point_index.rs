// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reusable immutable ordered point indexes through the shared strict codec.

use super::{
    GeoCallError, GeoSession, PointInput, cover, encode, invocation::Invocation, output,
    profile::decimal, request,
};
use purrdf_geo_kernel::{
    Crs, GeoError, GeoProfile, Metres, PreparationBudget,
    cells::{
        CubeHilbertQ62V1, MixedCoverLimits, PointCellIndex, PointIndexLimits, PointIndexPoint,
    },
};
use purrdf_lex::json::{
    self, Object, Value,
    record::{DecodeError, Record, items_with},
};
use std::sync::Arc;

/// A prepared reusable index; clones share immutable ordered point buckets.
#[derive(Clone, Debug)]
pub struct GeoPointIndex {
    profile: Arc<GeoProfile>,
    identity: purrdf_geo_kernel::binding::GeoQueryIdentity,
    index: Arc<PointCellIndex>,
}

impl GeoSession {
    /// Build a reusable index from original exact target-reference points.
    ///
    /// # Errors
    /// Refuses unknown/duplicate fields, mismatched native reference, duplicate
    /// keys and insufficient complete-source admission. No partial index exists.
    pub fn point_index(&self, text: &str) -> Result<GeoPointIndex, GeoCallError> {
        let invocation = Invocation::new(self.profile().policy());
        let (value, budget) =
            super::read_record_for_invocation(text, self.profile().policy(), &invocation)?;
        self.point_index_value(&value, budget, &invocation)
            .map(|(index, _)| index)
    }

    pub(super) fn point_index_value(
        &self,
        value: &Value,
        budget: PreparationBudget,
        invocation: &Invocation,
    ) -> Result<(GeoPointIndex, PreparationBudget), GeoCallError> {
        let mut budget = invocation.track_budget(budget);
        self.identity()?;
        let mut fields = Record::new(value, "point index preparation version one")?;
        version(&mut fields)?;
        fields.tag("operation", &["point-index"])?;
        let grid = CubeHilbertQ62V1::new(request::grid(&mut fields)?);
        let crs: Option<String> = fields.optional("crs")?;
        let crs = Crs::new(crs.as_deref().unwrap_or(purrdf_iri::vocab::ogc::CRS84))?;
        let reference = self.profile().reference(&crs)?.clone();
        let level = fields.required("level")?;
        let conversion: Option<String> = fields.optional("conversion")?;
        let conversion = conversion
            .map(|name| {
                let binding = self.profile().operation(&Crs::new(name)?)?;
                if binding.target() != &crs {
                    return Err(GeoError::PointIndexReferenceMismatch);
                }
                Ok(binding.chain().binding_id())
            })
            .transpose()?;
        let points = fields.required_with("points", |value| items_with(value, point))?;
        let limits = admitted_limits(
            budget.remaining()?,
            fields.optional_with("limits", limits)?.unwrap_or_default(),
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
                index: Arc::new(index),
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
        Object::new()
            .with("id", self.index.id().digest().to_string())
            .with("level", self.index.level())
            .with(
                "reference",
                self.index.reference().id().digest().to_string(),
            )
            .with(
                "conversion",
                self.index.conversion().map(|id| id.digest().to_string()),
            )
            .with("points", self.index.len().to_string())
            .into()
    }

    /// Execute a strict reported or physical radius search against retained buckets.
    ///
    /// # Errors
    /// Carries typed reference, comparison and resource refusals; no partial result.
    pub fn call(&self, text: &str) -> Result<Value, GeoCallError> {
        let invocation = Invocation::new(self.profile.policy());
        let (value, budget) =
            super::read_record_for_invocation(text, self.profile.policy(), &invocation)?;
        self.call_value(&value, budget, &invocation)
    }

    pub(super) fn call_value(
        &self,
        value: &Value,
        budget: PreparationBudget,
        invocation: &Invocation,
    ) -> Result<Value, GeoCallError> {
        let mut budget = invocation.track_budget(budget);
        let mut fields = Record::new(value, "point index search version one")?;
        version(&mut fields)?;
        let operation = fields.tag(
            "operation",
            &["point-index", "search-physical", "search-reported"],
        )?;
        if operation == "point-index" {
            fields.deny_unknown()?;
            budget.retain(0, self.index.retained_workspace_bytes())?;
            output::admit(*budget, output::record(5, 256)?)?;
            return Ok(self.metadata());
        }
        let center = fields.required_with("center", PointInput::from_value)?;
        let cover_limits = cover::admitted_mixed_in_policy(
            budget.remaining()?,
            fields
                .optional_with("cover_limits", cover::mixed_limits)?
                .unwrap_or(MixedCoverLimits::DEFAULT),
        );
        let limits = admitted_limits(
            budget.remaining()?,
            fields.optional_with("limits", limits)?.unwrap_or_default(),
        );
        let mut work = invocation.phase(*budget);
        let keys = if operation == "search-physical" {
            let radius = Metres::new(fields.required_with("radius_metres", decimal)?);
            fields.deny_unknown()?;
            self.index.search_physical_metered(
                &center.0,
                &radius,
                cover_limits,
                limits,
                &mut work,
            )?
        } else {
            let threshold = fields.required_with("threshold_metres", request::threshold)?;
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
            (keys.len() as u64)
                .checked_mul(size_of::<u64>() as u64)
                .ok_or(GeoError::ArithmeticOverflow("point search output storage"))?,
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
        Ok(Object::new()
            .with("index", self.index.id().digest().to_string())
            .with(
                "keys",
                Value::Array(
                    keys.into_iter()
                        .map(|key| Value::from(key.to_string()))
                        .collect(),
                ),
            )
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
        let invocation = Invocation::new(self.profile.policy());
        let answer = super::read_record_for_invocation(text, self.profile.policy(), &invocation)
            .and_then(|(value, budget)| self.call_value(&value, budget, &invocation));
        let success = answer.is_ok();
        let fields = Object::new()
            .with("version", 1_u32)
            .with("identity", encode::identity(&self.identity))
            .with("index", self.index.id().digest().to_string());
        let fields = match answer {
            Ok(value) => fields.with("result", value),
            Err(error) => fields.with("error", encode::refusal_admitted(&error, &invocation)),
        };
        (json::write_compact(&fields.into()), success)
    }
}
fn version(fields: &mut Record<'_>) -> Result<(), DecodeError> {
    if fields.required::<u32>("version")? != 1 {
        return Err(DecodeError::custom("geographic request version must be 1"));
    }
    Ok(())
}
fn point(value: &Value) -> Result<PointIndexPoint, GeoCallError> {
    let mut fields = Record::new(value, "stable point index entry")?;
    let key: String = fields.required("key")?;
    let key = key
        .parse()
        .map_err(|_| DecodeError::custom("point key must be an unsigned decimal u64 string"))?;
    let point = fields.required_with("point", PointInput::from_value)?.0;
    fields.deny_unknown()?;
    Ok(PointIndexPoint { key, point })
}
fn limits(value: &Value) -> Result<PointIndexLimits, DecodeError> {
    let mut fields = Record::new(value, "point index admission")?;
    let limits = PointIndexLimits {
        max_points: fields
            .optional("max_points")?
            .unwrap_or(PointIndexLimits::DEFAULT.max_points),
        max_work_items: fields
            .optional("max_work_items")?
            .unwrap_or(PointIndexLimits::DEFAULT.max_work_items),
        max_workspace_bytes: fields
            .optional("max_workspace_bytes")?
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

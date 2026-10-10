// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One bounded physical schedule for a pure positive UNION region. Driver choices
//! use a bottom-up forecast once; the selected schedule is then surveyed with its
//! incoming bindings. This is a deterministic cost heuristic, not exhaustive search.

use super::{
    ActiveDataset, BgpOrder, Binary64Scope, DatasetView, EvalError, GraphMatch, GraphPattern,
    PlanEstimate, PlanSurvey, SeedEstimate, VarSchema, forecast_bgp, record_bgp_forecast,
    step_size, survey_bgp_seeded,
};
use crate::WorkspaceCapability;
use crate::workspace::{AdmittedMap, AdmittedVec, QueryWorkspace, SharedWorkspace};

struct Summary {
    schema: VarSchema,
    certain: VarSchema,
    estimate: PlanEstimate,
}

/// The retained driver choice for each join in one pure BGP/Join/Union region.
/// Addresses identify borrowed nodes only and never enter emitted identities.
pub(crate) struct PositivePlan {
    unit_estimates: AdmittedMap<usize, PlanEstimate>,
    /// Only multi-pattern leaf orders, whose total index count is source-sized.
    unit_orders: AdmittedMap<usize, BgpOrder>,
    root_schema: VarSchema,
    drivers: AdmittedMap<usize, bool>,
    disjoint_children: AdmittedMap<usize, ()>,
}

purrdf_hash::debug_non_exhaustive!(PositivePlan { drivers });

/// Reuse the authoritative scope censuses without retaining a full variable set
/// for every prefix of a wide join. Compound nodes retain only scalar forecasts.
fn scope_summary(
    node: &GraphPattern,
    estimate: PlanEstimate,
    workspace: &WorkspaceCapability,
) -> Result<Summary, EvalError> {
    Ok(Summary {
        schema: crate::eval::syntactic_schema_admitted(node, workspace)?
            .as_ref()
            .clone(),
        certain: crate::property_fn_plan::collect_certainly_bound_admitted(node, workspace)?,
        estimate,
    })
}

fn union_width(left: &VarSchema, right: &VarSchema) -> Result<u64, EvalError> {
    let columns = left
        .len()
        .checked_add(
            right
                .vars()
                .iter()
                .filter(|variable| !left.contains(variable))
                .count(),
        )
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    u64::try_from(columns).map_err(|_| EvalError::WorkspaceBoundOverflow)
}

impl PositivePlan {
    /// Refuse scope boundaries and disconnected operands before constructing
    /// any seed or forecast. The small traversal stack stays inline on ordinary
    /// prepared-query shapes, so checking an ineligible operand allocates nothing.
    pub(crate) fn seed_eligible_admitted(
        root: &GraphPattern,
        schema: &VarSchema,
        workspace: &WorkspaceCapability,
    ) -> Result<bool, EvalError> {
        if schema.is_empty() {
            return Ok(false);
        }
        Ok(matches!(
            Self::region_facts(root, Some(schema), workspace)?,
            Some((_, true))
        ))
    }

    /// Whether the subtree can receive physical bindings without crossing a
    /// logical expression or solution-modifier boundary.
    pub(crate) fn pure_eligible_admitted(
        root: &GraphPattern,
        workspace: &WorkspaceCapability,
    ) -> Result<bool, EvalError> {
        Ok(Self::region_facts(root, None, workspace)?.is_some())
    }

    /// The single positive-shape census: UNION presence and borrowed shared slots.
    fn region_facts(
        root: &GraphPattern,
        schema: Option<&VarSchema>,
        workspace: &WorkspaceCapability,
    ) -> Result<Option<(bool, bool)>, EvalError> {
        let mut shared = false;
        let mut has_union = false;
        let mut storage = crate::workspace::LexicalFrame::new(workspace);
        let mut pending = purrdf_lex::walk::WorkList::<&GraphPattern, 16>::new();
        let mut memory = purrdf_lex::allocation::Memory::new(&mut storage);
        pending
            .try_push_admitted(root, &mut memory)
            .map_err(|error| {
                memory
                    .admission_mut()
                    .storage_error(error, "positive region census")
            })?;
        while let Some(node) = pending.pop() {
            match node {
                GraphPattern::Bgp { patterns } => {
                    if let Some(schema) = schema {
                        for pattern in patterns {
                            super::visit_triple_slots_admitted(pattern, workspace, |slot| {
                                if let super::SlotKey::Variable(variable) = slot {
                                    shared |= schema.contains(variable);
                                }
                                Ok(())
                            })?;
                        }
                    }
                }
                GraphPattern::Join { left, right } => {
                    for child in [&**right, &**left] {
                        pending
                            .try_push_admitted(child, &mut memory)
                            .map_err(|error| {
                                memory
                                    .admission_mut()
                                    .storage_error(error, "positive region census")
                            })?;
                    }
                }
                GraphPattern::Union { arms } => {
                    has_union = true;
                    for arm in arms {
                        pending
                            .try_push_admitted(arm, &mut memory)
                            .map_err(|error| {
                                memory
                                    .admission_mut()
                                    .storage_error(error, "positive region census")
                            })?;
                    }
                }
                _ => return Ok(None),
            }
        }
        pending.release_admitted(&mut memory).map_err(|error| {
            memory
                .admission_mut()
                .storage_error(error, "positive region census")
        })?;
        Ok(Some((has_union, shared)))
    }

    /// Forecast a complete pure region once. Other operators and regions without
    /// a UNION use their existing evaluation law.
    #[cfg(test)]
    pub(crate) fn build<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
    ) -> Result<Option<Self>, EvalError> {
        Self::build_admitted(
            dataset,
            active_dataset,
            active_graph,
            root,
            &QueryWorkspace::resident(),
        )
    }

    pub(crate) fn build_admitted<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        workspace: &QueryWorkspace<D::ReadError>,
    ) -> Result<Option<Self>, EvalError> {
        Self::build_with_seed(dataset, active_dataset, active_graph, root, None, workspace)
    }

    /// Ordinary joins may drive a pure relation from any upstream binding bag.
    /// This does not substitute expressions or cross a scope/modifier boundary.
    #[cfg(test)]
    pub(crate) fn build_seeded<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        seed: &SeedEstimate,
    ) -> Result<Option<Self>, EvalError> {
        Self::build_seeded_admitted(
            dataset,
            active_dataset,
            active_graph,
            root,
            seed,
            &QueryWorkspace::resident(),
        )
    }

    pub(crate) fn build_seeded_admitted<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        seed: &SeedEstimate,
        workspace: &QueryWorkspace<D::ReadError>,
    ) -> Result<Option<Self>, EvalError> {
        if !Self::seed_eligible_admitted(root, &seed.schema, &workspace.capability())? {
            return Ok(None);
        }
        Self::build_with_seed(
            dataset,
            active_dataset,
            active_graph,
            root,
            Some(seed),
            workspace,
        )
    }

    fn build_with_seed<D: DatasetView>(
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        seed: Option<&SeedEstimate>,
        account: &QueryWorkspace<D::ReadError>,
    ) -> Result<Option<Self>, EvalError> {
        let workspace = account.capability();
        let Some((has_union, _)) = Self::region_facts(root, None, &workspace)? else {
            return Ok(None);
        };
        if !has_union && seed.is_none() {
            return Ok(None);
        }
        let mut pending = AdmittedVec::new(&workspace);
        pending.push((root, false))?;
        let mut summaries = AdmittedMap::<usize, Summary>::default();
        let mut unit_estimates = AdmittedMap::default();
        let mut unit_orders = AdmittedMap::default();
        let mut disjoint_children = AdmittedMap::default();
        while let Some((node, close)) = pending.pop() {
            if !close {
                pending.push((node, true))?;
                match node {
                    GraphPattern::Bgp { .. } => {}
                    GraphPattern::Join { left, right } => {
                        pending.push((&**right, false))?;
                        pending.push((&**left, false))?;
                    }
                    GraphPattern::Union { arms } => {
                        for arm in arms.iter().rev() {
                            pending.push((arm, false))?;
                        }
                    }
                    _ => return Ok(None),
                }
                continue;
            }
            let summary = match node {
                GraphPattern::Bgp { patterns } => {
                    let seed = SeedEstimate::default();
                    let (schema, estimate, order) = forecast_bgp(
                        dataset,
                        active_dataset,
                        active_graph,
                        patterns,
                        &seed,
                        account,
                    )?;
                    if patterns.len() >= 2 {
                        unit_orders.insert_admitted(
                            std::ptr::from_ref::<GraphPattern>(node) as usize,
                            BgpOrder::from_admitted(order, &workspace)?,
                            &workspace,
                        )?;
                    }
                    Summary {
                        certain: schema.clone(),
                        schema,
                        estimate,
                    }
                }
                GraphPattern::Join { left, right } => {
                    let mut left = summaries
                        .remove(&(std::ptr::from_ref::<GraphPattern>(left) as usize))
                        .expect("left summary is complete");
                    let right = summaries
                        .remove(&(std::ptr::from_ref::<GraphPattern>(right) as usize))
                        .expect("right summary is complete");
                    if !left
                        .schema
                        .vars()
                        .iter()
                        .any(|variable| right.schema.index_of(variable).is_some())
                    {
                        disjoint_children.insert_admitted(
                            std::ptr::from_ref::<GraphPattern>(node) as usize,
                            (),
                            &workspace,
                        )?;
                    }
                    let shared = left
                        .certain
                        .vars()
                        .iter()
                        .filter(|variable| right.certain.contains(variable))
                        .count();
                    let precision = Binary64Scope::enter();
                    let rows = step_size(
                        precision.ops(),
                        left.estimate.rows as f64,
                        right.estimate.rows as f64,
                        shared,
                        dataset.term_count().max(1) as f64,
                    ) as u64;
                    left.schema = left.schema.union_admitted(&right.schema, &workspace)?;
                    // Connected children can probe from the driver's guaranteed
                    // slots. Forecast both directions with the same cardinality
                    // law; a child's independent peak is not its seeded peak.
                    // Nullable overlap is excluded by the certainty intersection.
                    let peak_rows = if shared == 0 {
                        rows.max(left.estimate.peak_rows)
                            .max(right.estimate.peak_rows)
                            .max(
                                left.estimate
                                    .peak_rows
                                    .saturating_mul(right.estimate.peak_rows),
                            )
                    } else {
                        let directed = |driver: &PlanEstimate, driven: &PlanEstimate| {
                            let seeded_peak = step_size(
                                precision.ops(),
                                driver.rows as f64,
                                driven.peak_rows as f64,
                                shared,
                                dataset.term_count().max(1) as f64,
                            ) as u64;
                            rows.max(driver.peak_rows).max(seeded_peak)
                        };
                        directed(&left.estimate, &right.estimate)
                            .min(directed(&right.estimate, &left.estimate))
                    };
                    left.certain = left.certain.union_admitted(&right.certain, &workspace)?;
                    Summary {
                        certain: left.certain,
                        estimate: PlanEstimate {
                            rows,
                            peak_rows,
                            columns: left.schema.len() as u64,
                        },
                        schema: left.schema,
                    }
                }
                GraphPattern::Union { arms } => {
                    let mut schema = VarSchema::default();
                    let mut rows = 0_u64;
                    let mut peak_rows = 0_u64;
                    let mut certain: Option<VarSchema> = None;
                    for arm in arms {
                        let arm = summaries
                            .remove(&(std::ptr::from_ref::<GraphPattern>(arm) as usize))
                            .expect("arm summary is complete");
                        for variable in arm.schema.vars() {
                            schema.push_admitted(variable.clone(), &workspace)?;
                        }
                        rows = rows.saturating_add(arm.estimate.rows);
                        peak_rows = peak_rows.max(arm.estimate.peak_rows);
                        certain = Some(match certain {
                            None => arm.certain,
                            Some(previous) => VarSchema::from_vars_admitted(
                                previous
                                    .vars()
                                    .iter()
                                    .filter(|variable| arm.certain.contains(variable))
                                    .cloned(),
                                &workspace,
                            )?,
                        });
                    }
                    Summary {
                        certain: certain.unwrap_or_default(),
                        estimate: PlanEstimate {
                            rows,
                            peak_rows: peak_rows.max(rows),
                            columns: schema.len() as u64,
                        },
                        schema,
                    }
                }
                _ => unreachable!("only pure nodes reach the closing step"),
            };
            unit_estimates.insert_admitted(
                std::ptr::from_ref::<GraphPattern>(node) as usize,
                summary.estimate.clone(),
                &workspace,
            )?;
            summaries.insert_admitted(
                std::ptr::from_ref::<GraphPattern>(node) as usize,
                summary,
                &workspace,
            )?;
        }
        let _ = summaries
            .remove(&(std::ptr::from_ref::<GraphPattern>(root) as usize))
            .expect("root summary is complete");
        debug_assert!(summaries.is_empty());
        // The egress layout comes from the authoritative logical census;
        // physical BGP widths may also price non-observable local blank slots.
        let root_schema = crate::eval::syntactic_schema_admitted(root, &workspace)?
            .as_ref()
            .clone();
        enum ChoiceStep<'a> {
            Visit(&'a GraphPattern, SharedWorkspace<SeedEstimate>, bool),
            AfterDriver {
                driver: &'a GraphPattern,
                driven: &'a GraphPattern,
                input: SharedWorkspace<SeedEstimate>,
                independent: bool,
            },
        }
        let mut drivers = AdmittedMap::default();
        let mut pending = AdmittedVec::new(&workspace);
        pending.push(ChoiceStep::Visit(
            root,
            SharedWorkspace::new_admitted(seed.cloned().unwrap_or_default(), &workspace)?,
            seed.is_some(),
        ))?;
        while let Some(step) = pending.pop() {
            let ChoiceStep::Visit(node, input, has_input) = step else {
                let ChoiceStep::AfterDriver {
                    driver,
                    driven,
                    input,
                    independent,
                } = step
                else {
                    unreachable!()
                };
                if independent {
                    pending.push(ChoiceStep::Visit(
                        driven,
                        SharedWorkspace::new_admitted(SeedEstimate::default(), &workspace)?,
                        false,
                    ))?;
                    continue;
                }
                let summary = scope_summary(
                    driver,
                    unit_estimates
                        .get(&(std::ptr::from_ref::<GraphPattern>(driver) as usize))
                        .expect("driver forecast")
                        .clone(),
                    &workspace,
                )?;
                let extended = SeedEstimate {
                    schema: input.schema.union_admitted(&summary.schema, &workspace)?,
                    bound: input.bound.union_admitted(&summary.certain, &workspace)?,
                    rows: 1,
                };
                pending.push(ChoiceStep::Visit(
                    driven,
                    SharedWorkspace::new_admitted(extended, &workspace)?,
                    true,
                ))?;
                continue;
            };
            match node {
                GraphPattern::Bgp { .. } => {}
                GraphPattern::Union { arms } => {
                    for arm in arms.iter().rev() {
                        pending.push(ChoiceStep::Visit(arm, input.clone(), has_input))?;
                    }
                }
                GraphPattern::Join { left, right } => {
                    let left_summary = scope_summary(
                        left,
                        unit_estimates
                            .get(&(std::ptr::from_ref::<GraphPattern>(left) as usize))
                            .expect("left forecast")
                            .clone(),
                        &workspace,
                    )?;
                    let right_summary = scope_summary(
                        right,
                        unit_estimates
                            .get(&(std::ptr::from_ref::<GraphPattern>(right) as usize))
                            .expect("right forecast")
                            .clone(),
                        &workspace,
                    )?;
                    let connected = |summary: &Summary| {
                        summary
                            .schema
                            .vars()
                            .iter()
                            .any(|v| input.bound.contains(v))
                    };
                    let score = |summary: &Summary| -> Result<(u64, u64), EvalError> {
                        Ok((
                            summary
                                .estimate
                                .peak_rows
                                .saturating_mul(union_width(&input.schema, &summary.schema)?),
                            summary.estimate.rows,
                        ))
                    };
                    let driver_left = match (connected(&left_summary), connected(&right_summary)) {
                        (true, false) => true,
                        (false, true) => false,
                        _ => score(&left_summary)? <= score(&right_summary)?,
                    };
                    drivers.insert_admitted(
                        std::ptr::from_ref::<GraphPattern>(node) as usize,
                        driver_left,
                        &workspace,
                    )?;
                    let (driver, driven) = if driver_left {
                        (&**left, &**right)
                    } else {
                        (&**right, &**left)
                    };
                    pending.push(ChoiceStep::AfterDriver {
                        driver,
                        driven,
                        input: input.clone(),
                        independent: !has_input
                            && disjoint_children
                                .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
                                .is_some(),
                    })?;
                    pending.push(ChoiceStep::Visit(driver, input, has_input))?;
                }
                _ => unreachable!("only pure nodes receive driver choices"),
            }
        }
        Ok(Some(Self {
            unit_estimates,
            unit_orders,
            root_schema,
            drivers,
            disjoint_children,
        }))
    }

    pub(crate) fn contains(&self, node: &GraphPattern) -> bool {
        self.unit_estimates
            .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
            .is_some()
    }

    /// Reuse the exact native unit order only when execution receives no seed.
    /// Single-pattern leaves keep the existing allocation-free singleton home.
    pub(crate) fn unit_order(&self, node: &GraphPattern) -> Option<BgpOrder> {
        self.unit_orders
            .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
            .cloned()
    }

    /// The same retained choice is read by execution and by its survey.
    pub(crate) fn driver_left(&self, node: &GraphPattern) -> bool {
        *self
            .drivers
            .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
            .expect("retained positive driver")
    }

    /// Logical child columns share no variable, including internal path witnesses.
    /// With no incoming seed each factor is evaluated once before its Cartesian join.
    pub(crate) fn children_disjoint(&self, node: &GraphPattern) -> bool {
        self.disjoint_children
            .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
            .is_some()
    }

    /// The region's source-ordered observable columns; internal joins remain
    /// name-indexed and need no row permutation before the region exits.
    pub(crate) const fn root_schema(&self) -> &VarSchema {
        &self.root_schema
    }

    fn extend_seed(
        &self,
        node: &GraphPattern,
        input: &SeedEstimate,
        rows: u64,
        workspace: &WorkspaceCapability,
    ) -> Result<SeedEstimate, EvalError> {
        let summary = scope_summary(
            node,
            self.unit_estimates
                .get(&(std::ptr::from_ref::<GraphPattern>(node) as usize))
                .expect("unit forecast")
                .clone(),
            workspace,
        )?;
        Ok(SeedEstimate {
            schema: input.schema.union_admitted(&summary.schema, workspace)?,
            bound: input.bound.union_admitted(&summary.certain, workspace)?,
            rows,
        })
    }

    /// Survey only the selected schedule, with precisely the same driver choices
    /// execution reads. Each BGP is priced once, with the bindings it receives.
    #[cfg(test)]
    pub(crate) fn survey<D: DatasetView>(
        &self,
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        survey: &mut PlanSurvey,
    ) -> Result<(), EvalError> {
        self.survey_admitted(
            dataset,
            active_dataset,
            active_graph,
            root,
            survey,
            &QueryWorkspace::resident(),
        )
    }

    pub(crate) fn survey_admitted<D: DatasetView>(
        &self,
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        root: &GraphPattern,
        survey: &mut PlanSurvey,
        account: &QueryWorkspace<D::ReadError>,
    ) -> Result<(), EvalError> {
        self.survey_seeded_admitted(
            dataset,
            active_dataset,
            active_graph,
            (root, None),
            survey,
            account,
        )
    }

    /// Forecast the exact seeded schedule that execution will consume.
    pub(crate) fn survey_seeded_admitted<D: DatasetView>(
        &self,
        dataset: &D,
        active_dataset: &ActiveDataset<D::Id>,
        active_graph: GraphMatch<D::Id>,
        seeded: (&GraphPattern, Option<&SeedEstimate>),
        survey: &mut PlanSurvey,
        account: &QueryWorkspace<D::ReadError>,
    ) -> Result<(), EvalError> {
        let (root, input) = seeded;
        let workspace = account.capability();
        enum Step<'a> {
            Visit(&'a GraphPattern, SharedWorkspace<SeedEstimate>, bool),
            Second {
                node: &'a GraphPattern,
                first: &'a GraphPattern,
                second: &'a GraphPattern,
                seed: SharedWorkspace<SeedEstimate>,
                independent: bool,
            },
            Join {
                node: &'a GraphPattern,
                first: &'a GraphPattern,
                second: &'a GraphPattern,
                independent: bool,
            },
            Union(&'a GraphPattern, SharedWorkspace<SeedEstimate>),
        }
        let mut steps = AdmittedVec::new(&workspace);
        steps.push(Step::Visit(
            root,
            SharedWorkspace::new_admitted(input.cloned().unwrap_or_default(), &workspace)?,
            input.is_some(),
        ))?;
        while let Some(step) = steps.pop() {
            match step {
                Step::Visit(node, seed, has_input) => match node {
                    GraphPattern::Bgp { patterns } => {
                        if has_input {
                            survey_bgp_seeded(
                                dataset,
                                active_dataset,
                                active_graph,
                                node,
                                &seed,
                                survey,
                                account,
                            )?;
                        } else {
                            let key = std::ptr::from_ref::<GraphPattern>(node) as usize;
                            record_bgp_forecast(
                                node,
                                patterns,
                                self.unit_estimates
                                    .get(&key)
                                    .expect("unit forecast")
                                    .clone(),
                                self.unit_orders.get(&key).map_or(&[][..], |order| &**order),
                                survey,
                            )?;
                        }
                    }
                    GraphPattern::Join { left, right } => {
                        let (first, second) = if self.driver_left(node) {
                            (&**left, &**right)
                        } else {
                            (&**right, &**left)
                        };
                        steps.push(Step::Second {
                            node,
                            first,
                            second,
                            seed: seed.clone(),
                            independent: !has_input && self.children_disjoint(node),
                        })?;
                        steps.push(Step::Visit(first, seed, has_input))?;
                    }
                    GraphPattern::Union { arms } => {
                        steps.push(Step::Union(node, seed.clone()))?;
                        for arm in arms.iter().rev() {
                            steps.push(Step::Visit(arm, seed.clone(), has_input))?;
                        }
                    }
                    _ => unreachable!("the plan certifies a pure region"),
                },
                Step::Second {
                    node,
                    first,
                    second,
                    seed,
                    independent,
                } => {
                    let rows = survey
                        .estimate_of(first)
                        .expect("first driver was surveyed")
                        .rows;
                    steps.push(Step::Join {
                        node,
                        first,
                        second,
                        independent,
                    })?;
                    if independent {
                        steps.push(Step::Visit(
                            second,
                            SharedWorkspace::new_admitted(SeedEstimate::default(), &workspace)?,
                            false,
                        ))?;
                    } else {
                        steps.push(Step::Visit(
                            second,
                            SharedWorkspace::new_admitted(
                                self.extend_seed(first, &seed, rows, &workspace)?,
                                &workspace,
                            )?,
                            true,
                        ))?;
                    }
                }
                Step::Join {
                    node,
                    first,
                    second,
                    independent,
                } => {
                    let second = survey
                        .estimate_of(second)
                        .expect("second child was surveyed");
                    let estimate = if independent {
                        let first = survey
                            .estimate_of(first)
                            .expect("first driver was surveyed");
                        let rows = first.rows.saturating_mul(second.rows);
                        PlanEstimate {
                            rows,
                            peak_rows: rows,
                            columns: u64::try_from(
                                crate::eval::syntactic_schema_admitted(node, &workspace)?.len(),
                            )
                            .map_err(|_| EvalError::WorkspaceBoundOverflow)?,
                        }
                    } else {
                        second.clone()
                    };
                    survey.record(node, estimate);
                }
                Step::Union(node, seed) => {
                    let GraphPattern::Union { arms } = node else {
                        unreachable!()
                    };
                    let mut rows = 0_u64;
                    let mut columns = 0_u64;
                    for arm in arms {
                        let estimate = survey.estimate_of(arm).expect("every arm was surveyed");
                        rows = rows.saturating_add(estimate.rows);
                        columns = columns.max(estimate.columns);
                    }
                    columns = columns.max(union_width(
                        &*crate::eval::syntactic_schema_admitted(node, &workspace)?,
                        &seed.schema,
                    )?);
                    survey.record(
                        node,
                        PlanEstimate {
                            rows,
                            peak_rows: rows,
                            columns,
                        },
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::RdfDatasetBuilder;
    use purrdf_sparql_algebra::{Query, SparqlParser};

    #[test]
    fn independent_factors_are_priced_once_before_the_product() {
        let mut builder = RdfDatasetBuilder::new();
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        let r = builder.intern_iri("http://example.org/r");
        let subject = builder.intern_iri("http://example.org/subject");
        let object = builder.intern_iri("http://example.org/object");
        for predicate in [q, r] {
            builder.push_quad(subject, predicate, object, None);
        }
        for index in 0..2 {
            let value = builder.intern_iri(&format!("http://example.org/value{index}"));
            builder.push_quad(subject, p, value, None);
        }
        let dataset = builder.freeze().expect("independent fixture");
        let Query::Select { pattern: GraphPattern::Project { inner, .. }, .. } = SparqlParser::new().parse_query(
            "PREFIX ex: <http://example.org/> SELECT * WHERE { ?x ex:p ?v . { ?a ex:q ?b } UNION { ?a ex:r ?b } }",
        ).expect("independent query") else { unreachable!() };
        let active = ActiveDataset::store_default();
        let plan = PositivePlan::build(&*dataset, &active, GraphMatch::Default, &inner)
            .expect("independent forecast")
            .expect("positive region");
        assert!(plan.children_disjoint(&inner));
        let GraphPattern::Join { left, right } = &*inner else {
            unreachable!()
        };
        let tree = crate::plan::Tree::build(&inner);
        let mut survey = PlanSurvey::for_shape(tree.shape());
        plan.survey(&*dataset, &active, GraphMatch::Default, &inner, &mut survey)
            .expect("independent survey");
        for child in [&**left, &**right] {
            let estimate = survey.estimate_of(child).expect("unit child forecast");
            assert_eq!((estimate.rows, estimate.columns), (2, 2));
        }
        let estimate = survey.estimate_of(&inner).expect("product forecast");
        assert_eq!(
            (estimate.rows, estimate.peak_rows, estimate.columns),
            (4, 4, 4)
        );
    }

    #[test]
    fn canonical_hidden_match_keys_keep_positive_children_connected() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let middle = builder.intern_iri("http://example.org/middle");
        let object = builder.intern_iri("http://example.org/object");
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        builder.intern_iri("http://example.org/r");
        builder.push_quad(subject, p, middle, None);
        builder.push_quad(middle, q, object, None);
        let dataset = builder.freeze().expect("hidden-key fixture");
        let Query::Select {
            pattern: GraphPattern::Project { inner, .. },
            ..
        } = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/> SELECT ?s ?o WHERE { ?s ex:p/(ex:q|ex:r) ?o }",
            )
            .expect("hidden-key query")
        else {
            unreachable!()
        };
        let plan = PositivePlan::build(
            &*dataset,
            &ActiveDataset::store_default(),
            GraphMatch::Default,
            &inner,
        )
        .expect("hidden-key forecast")
        .expect("positive region");
        assert!(!plan.children_disjoint(&inner));
    }

    #[test]
    fn retained_unit_orders_preserve_native_survey_provenance() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/subject");
        let target = builder.intern_iri("http://example.org/target");
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        let r = builder.intern_iri("http://example.org/r");
        let t = builder.intern_iri("http://example.org/t");
        for predicate in [q, r, t] {
            builder.push_quad(subject, predicate, target, None);
        }
        for index in 0..16 {
            let value = builder.intern_iri(&format!("http://example.org/value{index}"));
            builder.push_quad(subject, p, value, None);
        }
        let dataset = builder.freeze().expect("unit-order fixture");
        let Query::Select { pattern: GraphPattern::Project { inner, .. }, .. } = SparqlParser::new().parse_query(
            "PREFIX ex: <http://example.org/> SELECT * WHERE { ?s ex:p ?v . ?s ex:q ex:target . { ?a ex:r ?b } UNION { ?a ex:t ?b } }",
        ).expect("unit-order query") else { unreachable!() };
        let GraphPattern::Join { left, .. } = &*inner else {
            unreachable!()
        };
        let GraphPattern::Bgp { patterns } = &**left else {
            unreachable!()
        };
        let active = ActiveDataset::store_default();
        let plan = PositivePlan::build(&*dataset, &active, GraphMatch::Default, &inner)
            .expect("unit-order forecast")
            .expect("positive region");
        assert_eq!(
            plan.unit_order(left)
                .expect("retained multi-pattern order")
                .as_ref(),
            [1, 0]
        );
        assert_eq!(
            plan.unit_orders
                .iter()
                .map(|(_, order)| order)
                .map(|order| order.len())
                .sum::<usize>(),
            2
        );
        let GraphPattern::Join { right, .. } = &*inner else {
            unreachable!()
        };
        let GraphPattern::Union { arms } = &**right else {
            unreachable!()
        };
        assert!(arms.iter().all(|arm| plan.unit_order(arm).is_none()));
        let tree = crate::plan::Tree::build(&inner);
        let mut native = PlanSurvey::for_shape(tree.shape());
        super::super::survey_bgp(
            &*dataset,
            &active,
            GraphMatch::Default,
            left,
            patterns,
            &mut native,
            &QueryWorkspace::resident(),
        )
        .expect("native forecast");
        let mut retained = PlanSurvey::for_shape(tree.shape());
        plan.survey(
            &*dataset,
            &active,
            GraphMatch::Default,
            &inner,
            &mut retained,
        )
        .expect("retained unit survey");
        assert_eq!(retained.orders.len(), 2);
        assert_eq!(&*retained.orders, &*native.orders);
    }

    #[test]
    fn eligibility_is_retained_across_a_deep_plain_join_chain_and_a_barrier() {
        purrdf_stack::on_stack(128 * 1024, || {
            let mut chain = GraphPattern::empty_bgp();
            for _ in 0..100_000 {
                chain = GraphPattern::Join {
                    left: purrdf_sparql_algebra::Child::new(chain),
                    right: purrdf_sparql_algebra::Child::new(GraphPattern::empty_bgp()),
                };
            }
            let union = GraphPattern::union(chain, GraphPattern::empty_bgp());
            let project = GraphPattern::Project {
                inner: purrdf_sparql_algebra::Child::new(union),
                variables: Vec::new(),
            };
            let tree = crate::plan::Tree::build(&project);
            assert!(!tree.shape().positive_region(crate::plan::NodeId::ROOT));
            let GraphPattern::Project { inner, .. } = &project else {
                unreachable!()
            };
            assert!(
                tree.shape()
                    .positive_region(tree.shape().node_of(inner).expect("union node"))
            );
            let GraphPattern::Union { arms } = &**inner else {
                panic!("a union");
            };
            assert!(
                !tree
                    .shape()
                    .positive_region(tree.shape().node_of(&arms[0]).expect("plain join"))
            );
            for index in 2..tree.shape().len() {
                assert!(
                    !tree
                        .shape()
                        .positive_region(crate::plan::NodeId::from_index(index))
                );
            }
        })
        .expect("classification uses constant machine stack");
    }

    #[test]
    fn seeded_union_prices_the_full_combined_header() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("http://example.org/a");
        let object = builder.intern_iri("http://example.org/b");
        let p = builder.intern_iri("http://example.org/p");
        let q = builder.intern_iri("http://example.org/q");
        let r = builder.intern_iri("http://example.org/r");
        builder.push_quad(subject, p, object, None);
        for index in 0..100 {
            let value = builder.intern_iri(&format!("http://example.org/value{index}"));
            builder.push_quad(subject, q, value, None);
            builder.push_quad(object, r, value, None);
        }
        let dataset = builder.freeze().expect("the fixture is valid");
        let Query::Select { pattern: GraphPattern::Project { inner, .. }, .. } = SparqlParser::new().parse_query(
            "PREFIX ex: <http://example.org/> SELECT * WHERE { ?a ex:p ?b . { ?a ex:q ?c } UNION { ?b ex:r ?d } }",
        ).expect("the source parses") else { panic!("projected SELECT"); };
        let active = ActiveDataset::store_default();
        let plan = PositivePlan::build(&*dataset, &active, GraphMatch::Default, &inner)
            .expect("the forecast succeeds")
            .expect("a pure UNION region");
        assert!(plan.driver_left(&inner), "the one-row BGP is the driver");
        let GraphPattern::Join { right, .. } = &*inner else {
            panic!("joined UNION");
        };
        let tree = crate::plan::Tree::build(&inner);
        let mut survey = PlanSurvey::for_shape(tree.shape());
        plan.survey(&*dataset, &active, GraphMatch::Default, &inner, &mut survey)
            .expect("the selected schedule is priced");
        assert_eq!(
            survey.estimate_of(right).expect("union estimate").columns,
            4
        );
        assert_eq!(
            survey.estimate_of(&inner).expect("join estimate").columns,
            4
        );
    }
}

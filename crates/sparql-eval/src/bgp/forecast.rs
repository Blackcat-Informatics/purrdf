// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact native forecast keys and their operation-owned physical inputs.

use super::{
    AdmittedVec, BgpOrder, CompiledPattern, DatasetView, EvalError, GraphMatch, GraphScope, Pos,
    SharedWorkspace, TermId, ViewTermId, components_of, constant_of,
};
use crate::plan_cache::{CachedBgp, OrderCacheRef, OrderKey};
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ForecastKey {
    owner: purrdf_core::StatisticsOwnerToken,
    fields: AdmittedVec<u8>,
}

impl Hash for ForecastKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.fields.as_slice().hash(state);
    }
}

impl ForecastKey {
    fn field(&mut self, bytes: &[u8]) -> Result<(), EvalError> {
        purrdf_hash::frame::try_frame_le_with(bytes, |part| {
            for &byte in part {
                self.fields.push(byte)?;
            }
            Ok(())
        })
    }

    fn number(&mut self, value: u64) -> Result<(), EvalError> {
        self.field(&value.to_le_bytes())
    }

    fn id<I: ViewTermId>(&mut self, id: I) -> Result<(), EvalError> {
        // prepare checks the concrete ID type before constructing any key.
        let id = (&id as &dyn Any)
            .downcast_ref::<TermId>()
            .expect("native forecast ID gate");
        self.number(id.index() as u64)
    }

    fn graph<I: ViewTermId>(&mut self, graph: GraphMatch<I>) -> Result<(), EvalError> {
        match graph {
            GraphMatch::Any => self.number(0),
            GraphMatch::Default => self.number(1),
            GraphMatch::Named(id) => {
                self.number(2)?;
                self.id(id)
            }
        }
    }

    fn position<I: ViewTermId>(
        &mut self,
        position: &Pos<I>,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<(), EvalError> {
        let mut pending = AdmittedVec::new(workspace);
        pending.push(position)?;
        while let Some(position) = pending.pop() {
            match position {
                Pos::Slot(column) => {
                    self.number(0)?;
                    self.number(*column as u64)?;
                }
                Pos::Bound(id) => {
                    self.number(1)?;
                    self.id(*id)?;
                }
                Pos::Triple(triple) => {
                    self.number(2)?;
                    for child in [&triple.o, &triple.p, &triple.s] {
                        pending.push(child)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) struct PhysicalInput<I: ViewTermId> {
    pub(super) graph: GraphMatch<I>,
    pub(super) cost: purrdf_core::AccessCost,
    pub(super) cardinality: u64,
    pub(super) statements: Option<u64>,
}

pub(super) type PhysicalInputs<I> = AdmittedVec<AdmittedVec<PhysicalInput<I>>>;

pub(super) fn physical_input<D: DatasetView>(
    dataset: &D,
    pattern: &CompiledPattern<D::Id>,
    graph: GraphMatch<D::Id>,
) -> PhysicalInput<D::Id> {
    let probe = purrdf_core::ProbePattern {
        s: constant_of(&pattern.s),
        p: constant_of(&pattern.p),
        o: constant_of(&pattern.o),
        g: graph,
    };
    let plan = dataset.probe_plan(
        probe.s.is_some(),
        probe.p.is_some(),
        probe.o.is_some(),
        graph,
    );
    PhysicalInput {
        graph,
        cost: dataset.cost(probe, &plan),
        cardinality: dataset.cardinality_estimate(probe.s, probe.p, probe.o, graph),
        statements: dataset
            .statement_cardinality_estimate(probe, components_of(&pattern.o, constant_of)),
    }
}

pub(super) struct Preparation<'a, I: ViewTermId> {
    cache: &'a crate::plan_cache::BoundedOrderCache,
    key: OrderKey,
    bytes: usize,
    pub(super) inputs: PhysicalInputs<I>,
}

impl<I: ViewTermId> Preparation<'_, I> {
    pub(super) fn get(&self) -> Option<CachedBgp> {
        self.cache
            .lock()
            .expect("order cache lock poisoned")
            .get(&self.key)
    }

    pub(super) fn insert(
        &self,
        order: BgpOrder,
        rows: u64,
        peak: u64,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<(), EvalError> {
        let bytes = self
            .bytes
            .checked_add(size_of::<CachedBgp>())
            .and_then(|bytes| bytes.checked_add(order.len().checked_mul(size_of::<usize>())?))
            .and_then(|bytes| {
                bytes.checked_add(SharedWorkspace::<AdmittedVec<usize>>::control_bytes())
            })
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        self.cache
            .lock()
            .expect("order cache lock poisoned")
            .insert_admitted(
                self.key.clone(),
                CachedBgp {
                    order,
                    forecast: Some((rows, peak)),
                },
                bytes,
                workspace,
            )?;
        Ok(())
    }
}

pub(super) fn prepare<'a, D: DatasetView>(
    dataset: &D,
    compiled: &[CompiledPattern<D::Id>],
    scope: &GraphScope<D::Id>,
    initial: &[bool],
    driving_rows: u64,
    cache: Option<OrderCacheRef<'a>>,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<Preparation<'a, D::Id>>, EvalError> {
    if compiled.is_empty() || matches!(scope, GraphScope::Merge(graphs) if graphs.is_empty()) {
        return Ok(None);
    }
    let Some(OrderCacheRef::Bounded(cache)) = cache else {
        return Ok(None);
    };
    if workspace.is_bounded() || TypeId::of::<D::Id>() != TypeId::of::<TermId>() {
        return Ok(None);
    }
    if !cache
        .lock()
        .expect("order cache lock poisoned")
        .retention_enabled()
    {
        return Ok(None);
    }
    let Some(statistics) = dataset.statistics() else {
        return Ok(None);
    };
    let Some(identity) = statistics.cache_identity() else {
        return Ok(None);
    };
    let mut key = ForecastKey {
        owner: identity,
        fields: AdmittedVec::new(workspace),
    };
    const DOMAIN: purrdf_hash::Domain =
        purrdf_hash::Domain::new(b"purrdf-sparql-eval/numeric-forecast-key/v1");
    key.field(DOMAIN.as_bytes())?;
    key.field(statistics.source().as_bytes())?;
    key.number(statistics.fingerprint())?;
    key.number(dataset.term_count())?;
    let support = dataset.statement_indexes();
    key.field(&[
        u8::from(support.reifier_targets),
        u8::from(support.reifier_components),
        u8::from(support.annotation_values),
    ])?;
    key.number(driving_rows)?;
    key.number(initial.len() as u64)?;
    for &bound in initial {
        key.number(u64::from(bound))?;
    }
    match scope {
        GraphScope::One(graph) => {
            key.number(0)?;
            key.graph(*graph)?;
        }
        GraphScope::Merge(graphs) => {
            key.number(1)?;
            key.number(graphs.len() as u64)?;
            for &graph in graphs {
                key.graph(GraphMatch::Named(graph))?;
            }
        }
    }
    key.number(compiled.len() as u64)?;
    let mut inputs = AdmittedVec::with_capacity(compiled.len(), workspace)?;
    for pattern in compiled {
        for position in [&pattern.s, &pattern.p, &pattern.o] {
            key.position(position, workspace)?;
        }
        let mut probes = AdmittedVec::new(workspace);
        let mut capture = |graph| -> Result<(), EvalError> {
            let input = physical_input(dataset, pattern, graph);
            key.number(input.cost.work())?;
            key.number(input.cost.rows())?;
            // Keep every AccessCost discriminator, including absent measurements.
            for number in [
                input.cost.pages(),
                input.cost.cardinality_fallback(),
                input.statements,
            ] {
                key.number(u64::from(number.is_some()))?;
                key.number(number.unwrap_or(0))?;
            }
            key.number(match input.cost.residency() {
                None => 0,
                Some(purrdf_core::ReadResidency::Resident) => 1,
                Some(purrdf_core::ReadResidency::NonResident) => 2,
                Some(purrdf_core::ReadResidency::Mixed) => 3,
            })?;
            key.number(input.cardinality)?;
            probes.push(input)
        };
        match scope {
            GraphScope::One(graph) => capture(*graph)?,
            GraphScope::Merge(graphs) => {
                for &graph in graphs {
                    capture(GraphMatch::Named(graph))?;
                }
            }
        }
        inputs.push_reserved(probes);
    }
    let ForecastKey { owner, fields } = key;
    let (values, allocation) = fields.into_parts();
    let bytes = values
        .capacity()
        .checked_add(SharedWorkspace::<ForecastKey>::control_bytes())
        .and_then(|bytes| bytes.checked_add(3 * size_of::<OrderKey>()))
        .ok_or(EvalError::WorkspaceBoundOverflow)?;
    let key = ForecastKey {
        owner,
        fields: AdmittedVec::from_parts(values, allocation, workspace),
    };
    let key = OrderKey::Forecast(SharedWorkspace::new_admitted(key, workspace)?);
    Ok(Some(Preparation {
        cache,
        key,
        bytes,
        inputs,
    }))
}

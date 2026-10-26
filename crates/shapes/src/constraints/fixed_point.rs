// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed native work attached to an execution-owned question relation.

pub(crate) mod occurrence;
mod runtime;

use std::cell::{Cell, OnceCell, RefCell};
use std::sync::Arc;

use purrdf_lex::allocation::{Memory, Resident, Shared, StorageError};

use super::{AnyViolation, ConformanceMemo, ValidationContext, walk_shape};
use crate::data::ShaclData;
use crate::engine::FocusNode;
use crate::error::ShapesError;
use crate::plan::{LoweredExpr, PropertyPlan, ShapePlan};
use crate::shapes::PropertyReference;

#[derive(Clone, Copy)]
enum Body<'a> {
    Shape(ShapePlan<'a>),
    Property {
        definition: ShapePlan<'a>,
        edge: &'a PropertyReference,
        deactivation: &'a [LoweredExpr],
    },
}

impl<'a> Body<'a> {
    fn plan(self) -> ShapePlan<'a> {
        match self {
            Self::Shape(plan) => plan,
            Self::Property { definition, .. } => definition,
        }
    }
}

pub(super) fn original_plan(plan: ShapePlan<'_>) -> Result<ShapePlan<'_>, ShapesError> {
    runtime::original(plan)
}

pub(super) struct FixedPoint<'a> {
    execution: Shared<runtime::Execution<'a>>,
    source: OnceCell<Shared<ShaclData>>,
    report_active: RefCell<Vec<crate::recursion::RecursionPosition>>,
    report_cycles: RefCell<Vec<crate::recursion::RecursionCycle>>,
    report_live: Cell<usize>,
    occurrences: RefCell<occurrence::Index>,
}

impl<'a> FixedPoint<'a> {
    pub(super) fn for_plan(plan: ShapePlan<'a>) -> Result<Option<Self>, ShapesError> {
        Ok(runtime::Execution::new(plan)?.map(|execution| Self {
            execution,
            source: OnceCell::new(),
            report_active: RefCell::new(Vec::new()),
            report_cycles: RefCell::new(Vec::new()),
            report_live: Cell::new(0),
            occurrences: RefCell::new(occurrence::Index::default()),
        }))
    }

    fn ask(&self, body: Body<'a>, focus: &FocusNode) -> Result<bool, ShapesError> {
        let source = self.source.get().ok_or_else(|| {
            ShapesError::internal(
                "recursive work was requested before its data source was retained",
            )
        })?;
        let edge = match body {
            Body::Shape(_) => 0,
            Body::Property { edge, .. } => std::ptr::from_ref(edge).addr(),
        };
        let key = runtime::identity(source, body.plan(), edge);
        self.execution.question(key, focus, body)
    }

    pub(super) fn ask_shape(
        &self,
        plan: ShapePlan<'a>,
        focus: &FocusNode,
    ) -> Result<bool, ShapesError> {
        self.ask(Body::Shape(runtime::original(plan)?), focus)
    }

    pub(super) fn ask_property(
        &self,
        definition: ShapePlan<'a>,
        edge: &'a PropertyReference,
        deactivation: &'a [LoweredExpr],
        focus: &FocusNode,
    ) -> Result<bool, ShapesError> {
        if edge.deactivated {
            return Ok(true);
        }
        if edge.deactivated_when.is_empty() && edge.severity.is_none() && edge.messages.is_empty() {
            return self.ask_shape(definition, focus);
        }
        self.ask(
            Body::Property {
                definition,
                edge,
                deactivation,
            },
            focus,
        )
    }

    pub(super) fn solve(
        &self,
        context: ValidationContext<'a, '_>,
        focus: &FocusNode,
    ) -> Result<bool, ShapesError> {
        let source = Shared::try_new(context.store.retained())
            .map_err(|_| StorageError::AllocationFailed)?;
        self.source.set(source).map_err(|_| {
            ShapesError::internal("one recursive workspace was asked to start two root executions")
        })?;
        self.ask_shape(context.plan, focus)?;
        self.execution.solve(|body, focus| {
            let memo = ConformanceMemo::default();
            let plan = body.plan();
            let inner = ValidationContext {
                plan,
                depth: 0,
                memo: &memo,
                fixed: Some(self),
                capture: None,
                report_run: None,
                box_role_vocab: None,
                ..context
            };
            let mut sink = AnyViolation::new(plan.binding().conformance_disallows(), plan);
            sink.exhaustive = true;
            match body {
                Body::Shape(_) => {
                    walk_shape(inner, focus, &mut sink)?;
                }
                Body::Property {
                    definition,
                    edge,
                    deactivation,
                } => {
                    let (property, property_plan) =
                        PropertyPlan::from_reference(definition, edge, deactivation)?;
                    super::eval_property_shape(
                        inner,
                        focus,
                        property,
                        property_plan,
                        &[],
                        &mut sink,
                    )?;
                }
            }
            Ok(sink.conforms())
        })
    }

    pub(super) fn enter_report(
        &self,
        store: &ShaclData,
        plan: ShapePlan<'_>,
        focus: &FocusNode,
        edge: Option<(&Arc<crate::shapes::ModelOwner>, usize, usize)>,
    ) -> Result<bool, ShapesError> {
        let owner = plan
            .model()
            .ok_or(crate::PreparationRefusal::UnboundModel)?;
        let identity = runtime::identity(
            store,
            runtime::original(plan)?,
            edge.as_ref().map_or(0, |(_, _, key)| *key),
        );
        let (conforms, retained_focus) = self.execution.settled(identity, focus)?;
        let (original, retained_edge) = {
            let mut occurrences = self.occurrences.borrow_mut();
            let original = occurrences.shape(
                plan.origin().unwrap_or(owner),
                std::ptr::from_ref(plan.shape()).addr(),
            )?;
            let retained_edge = edge
                .as_ref()
                .map(|(owner, address, _)| occurrences.property(owner, *address))
                .transpose()?;
            (original, retained_edge)
        };
        let key = crate::recursion::RecursionPosition {
            source: self
                .source
                .get()
                .ok_or_else(|| ShapesError::internal("recursive report has no original data"))?
                .clone(),
            owner: Arc::clone(owner),
            origin: Arc::clone(plan.origin().unwrap_or(owner)),
            body: std::ptr::from_ref(plan.shape()).addr(),
            original,
            edge_key: edge.as_ref().map_or(0, |(_, _, key)| *key),
            edge: retained_edge,
            focus: retained_focus,
        };
        let mut active = self.report_active.borrow_mut();
        if let Some(start) = active.iter().position(|position| position.same_pair(&key)) {
            let mut resident = Resident;
            let mut memory = Memory::resume(&mut resident, self.report_live.get());
            let mut positions = Vec::new();
            for position in &active[start..] {
                memory.push(&mut positions, position.clone())?;
            }
            memory.push(&mut positions, key)?;
            let positions = Shared::try_new(memory.boxed_slice(positions)?)
                .map_err(|_| StorageError::AllocationFailed)?;
            memory.push(
                &mut self.report_cycles.borrow_mut(),
                crate::recursion::RecursionCycle {
                    conforms,
                    semantics: plan
                        .binding()
                        .validation_request()
                        .recursion
                        .as_ref()
                        .ok_or_else(|| {
                            ShapesError::internal("recursive report lost its selected law")
                        })?
                        .semantics,
                    positions,
                },
            )?;
            self.report_live.set(memory.admitted_bytes());
            return Ok(false);
        }
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.report_live.get());
        memory.push(&mut active, key)?;
        self.report_live.set(memory.admitted_bytes());
        Ok(true)
    }

    pub(super) fn leave_report(&self) {
        self.report_active.borrow_mut().pop();
    }

    pub(super) fn publish_report(
        &self,
        outcome: Option<&crate::recursion::evidence::ReportOutcome>,
        conforms: bool,
        finite_leaf: bool,
    ) -> Result<(), ShapesError> {
        if !conforms && !finite_leaf && self.report_cycles.borrow().is_empty() {
            return Err(ShapesError::internal(
                "negative recursive report has neither a disallowed leaf nor a cycle certificate",
            ));
        }
        let outcome = outcome.ok_or_else(|| {
            ShapesError::internal(
                "recursive report has no channel for its settled verdict and cycle evidence",
            )
        })?;
        let cycles = std::mem::take(&mut *self.report_cycles.borrow_mut());
        self.report_live.set(
            self.report_active
                .borrow()
                .capacity()
                .checked_mul(size_of::<crate::recursion::RecursionPosition>())
                .ok_or(StorageError::SizeOverflow)?,
        );
        outcome.publish(conforms, cycles)?;
        Ok(())
    }
}

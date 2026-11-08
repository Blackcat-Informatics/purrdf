// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native cycle evidence, separate from standard SHACL leaf results.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use purrdf_lex::allocation::{Memory, Resident, Shared};

use super::RecursionSemantics;
use crate::data::ShaclData;
use crate::engine::FocusNode;
use crate::error::ShapesError;
use crate::shapes::{ModelOwner, PropertyReference, Shape};
use crate::term::Term;

/// An original focus/definition pair on a closed report path. The retained data
/// and model domains distinguish equal RDF names belonging to different sources.
#[derive(Clone)]
pub struct RecursionPosition {
    pub(crate) source: Shared<ShaclData>,
    pub(crate) owner: Arc<ModelOwner>,
    pub(crate) origin: Arc<ModelOwner>,
    pub(crate) original: crate::constraints::fixed_point::occurrence::ShapeOccurrence,
    pub(crate) body: usize,
    pub(crate) edge: Option<crate::constraints::fixed_point::occurrence::PropertyOccurrence>,
    pub(crate) focus: Shared<FocusNode>,
    pub(crate) edge_key: usize,
}

purrdf_hash::debug_non_exhaustive!(RecursionPosition {});

impl RecursionPosition {
    /// The actual focus in this position's retained data namespace.
    /// # Errors
    /// Returns the original physical storage refusal while copying the term.
    pub fn focus(&self) -> Result<Term, ShapesError> {
        Ok(match &*self.focus {
            FocusNode::Interned(id) => crate::term::try_report_term(self.source.core_view(), *id)?,
            FocusNode::Foreign(term) => term.try_clone()?,
        })
    }
    /// The actual data acquisition used by this position.
    #[must_use]
    pub fn data(&self) -> &ShaclData {
        &self.source
    }
    /// The original reference-resolution domain, retained without copying bodies.
    #[must_use]
    pub fn model(&self) -> &Arc<ModelOwner> {
        &self.owner
    }
    /// The original shape occurrence, resolved safely in its retained owner.
    /// # Errors
    /// Returns an invariant failure if the retained occurrence cannot be found.
    pub fn shape(&self) -> Result<&Shape, ShapesError> {
        Ok(&self.original)
    }
    /// The original incoming property edge, including its authored metadata.
    /// # Errors
    /// Returns an invariant failure if the retained edge cannot be found.
    pub fn property_edge(&self) -> Result<Option<&PropertyReference>, ShapesError> {
        let Some(original) = &self.edge else {
            return Ok(None);
        };
        match &**original {
            crate::shapes::Constraint::PropertyReference(edge) => Ok(Some(edge)),
            _ => Err(ShapesError::internal(
                "cycle evidence no longer identifies a property edge",
            )),
        }
    }
    pub(crate) fn same_pair(&self, other: &Self) -> bool {
        self.source.identity() == other.source.identity()
            && Arc::ptr_eq(&self.owner, &other.owner)
            && Arc::ptr_eq(&self.origin, &other.origin)
            && self.body == other.body
            && self.edge_key == other.edge_key
            && *self.focus == *other.focus
    }
}

/// An observed closed path in the settled original-pair relation. The first and
/// last positions name the same pair; incoming edge occurrences remain distinct.
/// This native certificate invents neither a SHACL leaf violation nor a severity.
#[derive(Debug, Clone)]
pub struct RecursionCycle {
    /// The caller-selected law used to settle the relation.
    pub semantics: RecursionSemantics,
    /// The settled answer for the first/last pair, read from the original run.
    pub conforms: bool,
    /// The closed original path, retaining its model/data and edge identities.
    pub positions: Shared<Box<[RecursionPosition]>>,
}

/// A native recursive report. Its verdict includes the settled fixed-point
/// answer; standard SHACL results and cycle certificates are separate evidence.
#[derive(Debug, Clone)]
pub struct RecursiveValidationReport {
    /// Standard report projection, preserving the settled verdict, policy and
    /// original leaf results. A false LFP cycle need not have a finite leaf.
    pub report: crate::report::ValidationReport,
    /// Finite closed paths encountered while collecting the settled report.
    /// Order across independent target roots is unspecified; positions within
    /// each path preserve the original traversal and repeated closing pair.
    pub cycles: Vec<RecursionCycle>,
}

#[derive(Default)]
pub(crate) struct ReportOutcome {
    negative: AtomicBool,
    cycles: Mutex<StoredCycles>,
}
#[derive(Default)]
struct StoredCycles {
    values: Vec<RecursionCycle>,
    live: usize,
}
impl ReportOutcome {
    pub(crate) fn publish(
        &self,
        conforms: bool,
        cycles: Vec<RecursionCycle>,
    ) -> Result<(), ShapesError> {
        let mut stored = self
            .cycles
            .lock()
            .map_err(|_| ShapesError::internal("recursive report evidence lock was poisoned"))?;
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, stored.live);
        let result = memory.extend(&mut stored.values, cycles);
        stored.live = memory.admitted_bytes();
        drop(stored);
        result?;
        if !conforms {
            self.negative.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    pub(crate) fn conforms(&self) -> bool {
        !self.negative.load(Ordering::Relaxed)
    }
    pub(crate) fn take_cycles(&self) -> Result<Vec<RecursionCycle>, ShapesError> {
        let mut stored = self
            .cycles
            .lock()
            .map_err(|_| ShapesError::internal("recursive report evidence lock was poisoned"))?;
        stored.live = 0;
        Ok(std::mem::take(&mut stored.values))
    }
}

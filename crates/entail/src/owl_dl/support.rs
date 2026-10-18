// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The actual finite positive prefixes needed by recorded clause contradictions.
//!
//! Unlike legacy structural clash grounding, replay starts with the consumer's
//! source assertions and selected refutation assumptions. A licensed head creates
//! its actual witnesses/edges/identifications through the original native graph
//! operations. No producer label or asserted cache boolean establishes a premise.

use super::bounds::{
    Failure, Products, SchemaBoundEvidence, SchemaObstruction, SchemaPreparationBudget,
    SchemaPreparationStats,
};
use super::graph::{GeneratedRoot, NominalId};
use super::hyper::Ground;
use super::proof::{CurrentSupportEvidence, MAX_RECORDED_STEPS, SchemaClashEvidence};
use core::convert::Infallible;
use purrdf_lex::walk::VecReserve;

/// Disabled recording or the latest admitted predecessor in this tableau arm.
#[derive(Clone, Copy)]
pub(crate) enum Cursor {
    /// No recording was requested, or its first physical refusal occurred.
    Disabled,
    /// The empty prefix has no predecessor; later prefixes name an arena entry.
    Active(Option<usize>),
}

/// One observed application, before its head changes the current graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Application {
    pub(crate) clause: usize,
    pub(crate) frame: Vec<usize>,
    pub(crate) branch: Option<(usize, usize)>,
    pub(crate) head: Vec<Ground<usize>>,
}

/// A branch copies only its latest predecessor index. The recorder owns every
/// application once, including applications in alternatives later discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    pub(crate) previous: Option<usize>,
    pub(crate) application: Application,
}

#[derive(Debug, Default)]
pub(crate) struct Journal {
    links: Vec<Link>,
    budget: SchemaPreparationBudget,
    stats: SchemaPreparationStats,
    live: usize,
}

type RecordingProducts<'a> = Products<'a, Infallible, fn() -> Result<(), Infallible>>;

impl Journal {
    #[cfg(test)]
    pub(crate) fn with_budget(budget: SchemaPreparationBudget) -> Self {
        Self {
            budget,
            ..Self::default()
        }
    }
    pub(crate) const fn obstruction(&self) -> Option<SchemaObstruction> {
        self.stats.obstruction
    }

    pub(crate) fn into_links(self) -> Vec<Link> {
        self.links
    }

    fn run<T>(
        &mut self,
        body: impl FnOnce(&mut RecordingProducts<'_>, &mut Vec<Link>) -> Result<T, Failure<Infallible>>,
    ) -> Result<T, SchemaObstruction> {
        if let Some(error) = self.stats.obstruction {
            return Err(error);
        }
        let mut poll: fn() -> Result<(), Infallible> = || Ok(());
        let mut memory = Products::new(self.budget, self.stats, self.live, &mut poll);
        let result = body(&mut memory, &mut self.links);
        self.stats = memory.stats;
        self.live = memory.live;
        result.map_err(|failure| {
            let error = match failure {
                Failure::Obstruction(error) => error,
                Failure::Boundary(never) => match never {},
            };
            self.stats.obstruction = Some(error);
            error
        })
    }

    pub(crate) fn record(
        &mut self,
        previous: Option<usize>,
        clause: usize,
        frame: &[usize],
        branch: Option<(usize, usize)>,
        head: &[Ground<usize>],
    ) -> Result<usize, SchemaObstruction> {
        if self.links.len() >= MAX_RECORDED_STEPS {
            let error = SchemaObstruction::Work {
                spent: self.links.len() as u64,
                limit: MAX_RECORDED_STEPS as u64,
            };
            self.stats.obstruction.get_or_insert(error);
            return Err(self
                .stats
                .obstruction
                .expect("the first recording refusal is retained"));
        }
        self.run(|memory, links| {
            memory.reserve(links, 1)?;
            let frame = memory.copy(frame)?;
            let head = copy_head(head, memory)?;
            let index = links.len();
            links.push(Link {
                previous,
                application: Application {
                    clause,
                    frame,
                    branch,
                    head,
                },
            });
            Ok(index)
        })
    }

    pub(crate) fn extract(
        &mut self,
        bounds: Option<&SchemaBoundEvidence>,
        latest: Option<usize>,
        frame: &[usize],
    ) -> Result<(Option<SchemaClashEvidence>, CurrentSupportEvidence), SchemaObstruction> {
        self.run(|memory, _| {
            let schema = bounds
                .map(|bounds| {
                    copy_bounds(bounds, memory).map(|bounds| SchemaClashEvidence { bounds })
                })
                .transpose()?;
            Ok((
                schema,
                CurrentSupportEvidence {
                    latest,
                    frame: memory.copy(frame)?,
                },
            ))
        })
    }
}

fn copy_bounds(
    bounds: &SchemaBoundEvidence,
    memory: &mut RecordingProducts<'_>,
) -> Result<SchemaBoundEvidence, Failure<Infallible>> {
    let class_steps = memory.copy(&bounds.class_steps)?;
    let qualifier = match &bounds.qualifier {
        super::bounds::QualifierProof::Object(steps) => {
            super::bounds::QualifierProof::Object(memory.copy(steps)?)
        }
        qualifier => qualifier.clone(),
    };
    let roles = memory.copy(&bounds.roles)?;
    Ok(SchemaBoundEvidence {
        class: bounds.class,
        lower: bounds.lower,
        upper: bounds.upper,
        class_steps,
        qualifier,
        roles,
    })
}

/// Borrow the selected applications in source order; never copy their frames/heads.
pub(crate) fn prefix<'a>(
    links: &'a [Link],
    latest: Option<usize>,
    stop: Option<&dyn purrdf_core::StopSignal>,
) -> Result<Vec<&'a Application>, super::proof::DlProofError> {
    use super::proof::{DlProofError, malformed};
    use purrdf_lex::allocation::{Memory, Resident};
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut ordered = Vec::new();
    let mut previous = latest;
    while let Some(index) = previous {
        if purrdf_core::governor::is_stopped(stop) {
            return Err(DlProofError::Stopped);
        }
        let link = links
            .get(index)
            .ok_or_else(|| malformed("missing current-support predecessor"))?;
        if link.previous.is_some_and(|previous| previous >= index) {
            return Err(malformed(
                "current-support predecessors must precede their application",
            ));
        }
        memory
            .reserve_for_push(&mut ordered)
            .map_err(DlProofError::Storage)?;
        ordered.push(&link.application);
        previous = link.previous;
    }
    ordered.reverse();
    Ok(ordered)
}

fn copy_head<E>(
    input: &[Ground<usize>],
    memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
) -> Result<Vec<Ground<usize>>, Failure<E>> {
    let mut head = Vec::new();
    memory.reserve(&mut head, input.len())?;
    for atom in input {
        head.push(match atom {
            Ground::EqualReserved(node, root) => {
                Ground::EqualReserved(*node, copy_root(root, memory)?)
            }
            // Every other native atom is scalar, so this clone allocates nothing.
            atom => atom.clone(),
        });
    }
    Ok(head)
}

fn copy_root<E>(
    root: &GeneratedRoot,
    memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
) -> Result<GeneratedRoot, Failure<E>> {
    let mut chain = Vec::new();
    let mut current = root;
    let origin = loop {
        memory.push(&mut chain, (current.role, current.filler, current.index))?;
        match &current.origin {
            NominalId::Named(individual) => break *individual,
            NominalId::Generated(parent) => current = parent,
        }
    };
    let mut origin = NominalId::Named(origin);
    while let Some((role, filler, index)) = chain.pop() {
        let root = GeneratedRoot {
            origin,
            role,
            filler,
            index,
        };
        if chain.is_empty() {
            let bytes = chain.capacity() * size_of::<(super::concept::Role, u32, u32)>();
            drop(chain);
            memory.live -= bytes;
            return Ok(root);
        }
        origin = NominalId::Generated(memory.boxed(root)?);
    }
    unreachable!("a generated root always contributes one chain link")
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_alloc_probe::CurrentThreadWindow;

    #[test]
    fn support_metadata_refusal_precedes_payload_allocation_and_keeps_original_cause() {
        let mut journal = Journal {
            budget: SchemaPreparationBudget {
                bytes: Some(0),
                work: None,
            },
            ..Journal::default()
        };
        let window = CurrentThreadWindow::open();
        let first = journal
            .record(None, 0, &[0], None, &[Ground::Concept(0, 1)])
            .unwrap_err();
        assert!(matches!(first, SchemaObstruction::Storage { limit: 0, .. }));
        assert_eq!(window.sample().allocations, 0);
        assert_eq!(journal.record(None, 0, &[0], None, &[]), Err(first));
        drop(journal);
        assert_eq!(window.sample().retained_bytes, 0);
    }

    #[test]
    fn scalar_branch_predecessors_share_one_admitted_application_arena() {
        let window = CurrentThreadWindow::open();
        let mut journal = Journal::default();
        let common = journal
            .record(None, 0, &[0], None, &[Ground::Concept(0, 1)])
            .unwrap();
        let left = journal
            .record(
                Some(common),
                1,
                &[0],
                Some((0, 0)),
                &[Ground::Concept(0, 2)],
            )
            .unwrap();
        let right = journal
            .record(
                Some(common),
                1,
                &[0],
                Some((0, 1)),
                &[Ground::Concept(0, 3)],
            )
            .unwrap();
        assert_eq!(journal.links[left].previous, Some(common));
        assert_eq!(journal.links[right].previous, Some(common));
        assert_eq!(journal.links.len(), 3);
        let measured = window.sample();
        assert_eq!(
            measured.retained_bytes,
            i64::try_from(journal.live).unwrap()
        );
        assert!(measured.peak_working_bytes <= i64::try_from(journal.stats.peak_bytes).unwrap());
        drop(journal);
        assert_eq!(window.sample().retained_bytes, 0);
    }

    #[test]
    fn clash_extraction_retains_shared_application_identities_without_copying_prefixes() {
        let mut journal = Journal::default();
        let common = journal
            .record(None, 0, &[0], None, &[Ground::Concept(0, 1)])
            .unwrap();
        let left = journal
            .record(
                Some(common),
                1,
                &[0],
                Some((0, 0)),
                &[Ground::Concept(0, 2)],
            )
            .unwrap();
        let right = journal
            .record(
                Some(common),
                1,
                &[0],
                Some((0, 1)),
                &[Ground::Concept(0, 3)],
            )
            .unwrap();
        let (left_bounds, left) = journal.extract(None, Some(left), &[0]).unwrap();
        let (right_bounds, right) = journal.extract(None, Some(right), &[0]).unwrap();
        assert!(left_bounds.is_none() && right_bounds.is_none());
        let links = journal.into_links();
        let left = prefix(&links, left.latest, None).unwrap();
        let right = prefix(&links, right.latest, None).unwrap();
        assert_eq!(links.len(), 3);
        assert_eq!(left.len() + right.len(), 4);
        assert!(core::ptr::eq(left[0], right[0]));
        assert!(!core::ptr::eq(left[1], right[1]));
    }
}

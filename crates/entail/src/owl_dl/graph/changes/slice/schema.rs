// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Backward dependencies of an already checked bound proof, never schema search.

use super::{Error, Kb};
use crate::owl_dl::{
    bounds::{SchemaBoundEvidence, SchemaRule, SchemaStep, SchemaUpperBound},
    concept::Decomp,
};
use purrdf_lex::allocation::{Memory, OrderedTable, Resident};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Part {
    Class,
    Qualifier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Read {
    part: Part,
    index: usize,
}

impl Read {
    pub(super) fn step(self, bounds: &SchemaBoundEvidence) -> &SchemaStep {
        &steps(bounds, self.part)[self.index]
    }
}

fn steps(bounds: &SchemaBoundEvidence, part: Part) -> &[SchemaStep] {
    match part {
        Part::Class => bounds.class_derivation(),
        Part::Qualifier => bounds.qualifier_derivation(),
    }
}

pub(super) fn select(
    kb: &Kb,
    bounds: &SchemaBoundEvidence,
    memory: &mut Memory<'_, Resident>,
) -> Result<Vec<Read>, Error> {
    let mut index = OrderedTable::default();
    for part in [Part::Class, Part::Qualifier] {
        for (ordinal, step) in steps(bounds, part).iter().enumerate() {
            poll(kb)?;
            index.insert((part, step.concept()), ordinal, memory)?;
        }
    }
    let mut pending = Vec::new();
    memory.push(&mut pending, (Part::Class, bounds.lower_restriction()))?;
    if let SchemaUpperBound::Restriction(upper) = bounds.upper_bound() {
        memory.push(&mut pending, (Part::Class, upper))?;
        if !bounds.qualifier_derivation().is_empty() {
            let Decomp::Max(_, _, filler) = kb.table.decomp(upper) else {
                return Err(Error::UnexpectedSchema);
            };
            memory.push(&mut pending, (Part::Qualifier, *filler))?;
        }
    }
    let mut visited = OrderedTable::default();
    let mut selected = Vec::new();
    while let Some((part, concept)) = pending.pop() {
        poll(kb)?;
        let ordinal = *index.get(&(part, concept)).ok_or(Error::UnexpectedSchema)?;
        let read = Read {
            part,
            index: ordinal,
        };
        if visited.insert(read, (), memory)?.is_some() {
            continue;
        }
        memory.push(&mut selected, read)?;
        match read.step(bounds).rule {
            SchemaRule::Assume | SchemaRule::Top => {}
            SchemaRule::Conjunct(parent) => memory.push(&mut pending, (part, parent))?,
            SchemaRule::Inclusion(inclusion) => {
                let &(sub, _) = kb.tbox.get(inclusion).ok_or(Error::UnexpectedSchema)?;
                memory.push(&mut pending, (part, sub))?;
            }
            SchemaRule::Conjunction => {
                let Decomp::And(children) = kb.table.decomp(concept) else {
                    return Err(Error::UnexpectedSchema);
                };
                memory.extend(&mut pending, children.iter().map(|&child| (part, child)))?;
            }
        }
    }
    Ok(selected)
}

fn poll(kb: &Kb) -> Result<(), Error> {
    if kb
        .stop
        .as_deref()
        .is_some_and(purrdf_datalog::StopSignal::stopped)
    {
        Err(Error::Stopped)
    } else {
        Ok(())
    }
}

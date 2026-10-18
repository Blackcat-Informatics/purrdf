// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native compiler decisions, shared by transitions instead of copied source sets.

use super::{Role, source::Input};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) enum Licence {
    Inclusion { sub: Role, sup: Role },
    Source(Input),
    Join { first: usize, second: usize },
    Lower { role: Role, transition: usize },
}

impl std::fmt::Debug for Licence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inclusion { sub, sup } => {
                f.debug_tuple("Inclusion").field(sub).field(sup).finish()
            }
            Self::Source(input) => f.debug_tuple("Source").field(input).finish(),
            Self::Join { first, second } => {
                f.debug_tuple("Join").field(first).field(second).finish()
            }
            Self::Lower { role, transition } => f
                .debug_tuple("Lower")
                .field(role)
                .field(transition)
                .finish(),
        }
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Storage(purrdf_lex::allocation::StorageError),
    Stopped,
    MissingTransition,
    MissingInput(Input),
    MissingOrder { sub: Role, sup: Role },
}

#[cfg(test)]
purrdf_lex::variant_from!(Error { Storage(purrdf_lex::allocation::StorageError) });

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Pending {
    Machine(usize, usize),
    Order(usize),
    Component(usize),
}

#[cfg(test)]
impl super::RoleProgram {
    /// Follow only retained compiler decisions, never another role search.
    pub(crate) fn path_inputs<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        transitions: impl IntoIterator<Item = (Role, usize)>,
        stop: Option<&dyn purrdf_datalog::StopSignal>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<Vec<Input>, Error> {
        use purrdf_lex::allocation::OrderedTable;
        memory.try_scope(|memory| {
            if stop.is_some_and(purrdf_datalog::StopSignal::stopped) {
                return Err(Error::Stopped);
            }
            let mut pending = Vec::new();
            let mut visited = OrderedTable::default();
            let mut inputs = OrderedTable::default();
            let mut selected = Vec::new();
            for (role, index) in transitions {
                if stop.is_some_and(purrdf_datalog::StopSignal::stopped) {
                    return Err(Error::Stopped);
                }
                self.select_transition(role, index, &mut pending, memory)?;
            }
            while let Some(task) = pending.pop() {
                if stop.is_some_and(purrdf_datalog::StopSignal::stopped) {
                    return Err(Error::Stopped);
                }
                if visited.insert(task, (), memory)?.is_some() {
                    continue;
                }
                let Pending::Machine(machine, id) = task else {
                    self.select_order(task, &mut pending, &mut inputs, &mut selected, memory)?;
                    continue;
                };
                match *self
                    .machines
                    .get(machine)
                    .and_then(|machine| machine.licences.get(id))
                    .ok_or(Error::MissingTransition)?
                {
                    Licence::Inclusion { sub, sup } => {
                        self.select_inclusion(sub, sup, &mut pending, memory)?;
                    }
                    Licence::Source(input) => {
                        self.select_input(input, &mut inputs, &mut selected, memory)?;
                    }
                    Licence::Join { first, second } => {
                        memory.push(&mut pending, Pending::Machine(machine, second))?;
                        memory.push(&mut pending, Pending::Machine(machine, first))?;
                    }
                    Licence::Lower { role, transition } => {
                        self.select_transition(role, transition, &mut pending, memory)?;
                    }
                }
            }
            memory.release_vec(pending)?;
            visited.release(memory)?;
            inputs.release(memory)?;
            Ok(selected)
        })
    }

    fn select_input<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        input: Input,
        seen: &mut purrdf_lex::allocation::OrderedTable<Input, ()>,
        selected: &mut Vec<Input>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), Error> {
        self.source
            .sources
            .get(input)
            .ok_or(Error::MissingInput(input))?;
        if seen.insert(input, (), memory)?.is_none() {
            memory.push(selected, input)?;
        }
        Ok(())
    }

    fn select_inclusion<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        sub: Role,
        sup: Role,
        pending: &mut Vec<Pending>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), Error> {
        if sub == sup {
            return Ok(());
        }
        let missing = || Error::MissingOrder { sub, sup };
        let relation = self.simple.as_ref().ok_or_else(missing)?;
        let witnesses = relation.witnesses.as_ref().ok_or_else(missing)?;
        let from = relation.component[self.roles.binary_search(&sub).map_err(|_| missing())?];
        let to = relation.component[self.roles.binary_search(&sup).map_err(|_| missing())?];
        if from != to {
            memory.push(
                pending,
                Pending::Order(witnesses.pair(from, to).ok_or_else(missing)?),
            )?;
        } else if witnesses.components[from].is_none() {
            return Err(missing());
        }
        memory.push(pending, Pending::Component(from))?;
        memory.push(pending, Pending::Component(to))?;
        Ok(())
    }

    fn select_order<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        task: Pending,
        pending: &mut Vec<Pending>,
        seen: &mut purrdf_lex::allocation::OrderedTable<Input, ()>,
        selected: &mut Vec<Input>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), Error> {
        use super::order::Step;
        let witnesses = self
            .simple
            .as_ref()
            .and_then(|relation| relation.witnesses.as_ref())
            .ok_or(Error::MissingTransition)?;
        let id = match task {
            Pending::Component(component) => {
                if let Some(root) = *witnesses
                    .components
                    .get(component)
                    .ok_or(Error::MissingTransition)?
                {
                    memory.push(pending, Pending::Order(root))?;
                }
                return Ok(());
            }
            Pending::Order(id) => id,
            Pending::Machine(..) => unreachable!("machine tasks use the native transition owner"),
        };
        match *witnesses.steps.get(id).ok_or(Error::MissingTransition)? {
            Step::Input(input) => self.select_input(input, seen, selected, memory)?,
            Step::TopInverse => {}
            Step::Join { first, second, via } => {
                memory.push(pending, Pending::Order(second))?;
                memory.push(pending, Pending::Order(first))?;
                if let Some(via) = via {
                    memory.push(pending, Pending::Component(via))?;
                }
            }
        }
        Ok(())
    }

    fn select_transition<S: purrdf_lex::allocation::Admission + ?Sized>(
        &self,
        role: Role,
        index: usize,
        pending: &mut Vec<Pending>,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> Result<(), Error> {
        let position = self
            .roles
            .binary_search(&role)
            .map_err(|_| Error::MissingTransition)?;
        let id = *self
            .machine_for
            .get(position)
            .ok_or(Error::MissingTransition)?;
        let head = *self
            .roles
            .get(
                self.machine_for
                    .iter()
                    .position(|&machine| machine == id)
                    .ok_or(Error::MissingTransition)?,
            )
            .ok_or(Error::MissingTransition)?;
        self.select_inclusion(head, role, pending, memory)?;
        let transition = self
            .machines
            .get(id)
            .and_then(|machine| machine.transitions.get(index))
            .ok_or(Error::MissingTransition)?;
        memory.push(pending, Pending::Machine(id, transition.licence))?;
        Ok(())
    }
}

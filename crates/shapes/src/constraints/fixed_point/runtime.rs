// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One borrowed original-pair relation for a proved closed catalog.

use super::Body;
use crate::data::{DatasetIdentity, ShaclData};
use crate::engine::FocusNode;
use crate::error::{RecursionRefusal, RecursionRefusalReason, ShapesError};
use crate::plan::ShapePlan;
use crate::recursion::{RecursionOptions, RecursionSemantics};
use purrdf_core::graph::{GraphError, scc_component_index_with_memory};
use purrdf_lex::allocation::{Memory, OrderedTable, Resident, Shared, StorageError};
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Identity {
    source: DatasetIdentity,
    model: usize,
    shape: usize,
    edge: usize,
}

pub(super) fn identity(store: &ShaclData, plan: ShapePlan<'_>, edge: usize) -> Identity {
    Identity {
        source: store.identity(),
        model: plan.model().map_or(0, |owner| Arc::as_ptr(owner).addr()),
        shape: std::ptr::from_ref(plan.shape()).addr(),
        edge,
    }
}

pub(super) fn original(plan: ShapePlan<'_>) -> Result<ShapePlan<'_>, ShapesError> {
    if crate::recursion::model::canonical_handle(plan.shape()) {
        plan.definition()
    } else {
        Ok(plan)
    }
}

fn fingerprint(identity: Identity, focus: &FocusNode) -> u64 {
    let mut hash = purrdf_hash::fixed::FixedHasher::default();
    identity.hash(&mut hash);
    focus.hash(&mut hash);
    hash.finish()
}

struct Pair<'a> {
    identity: Identity,
    focus: Shared<FocusNode>,
    body: Body<'a>,
    value: bool,
    dependencies: Vec<usize>,
}

#[derive(Default)]
struct State<'a> {
    pairs: Vec<Pair<'a>>,
    index: OrderedTable<u64, Vec<usize>>,
    current: Option<usize>,
    generation: u64,
    iterations: u64,
    settled: bool,
    live: usize,
}

impl State<'_> {
    fn memory<T>(
        &mut self,
        operation: impl FnOnce(&mut Self, &mut Memory<'_, Resident>) -> Result<T, ShapesError>,
    ) -> Result<T, ShapesError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.live);
        let result = operation(self, &mut memory);
        self.live = memory.admitted_bytes();
        result
    }
    fn find(&self, identity: Identity, focus: &FocusNode) -> Option<usize> {
        self.index
            .get(&fingerprint(identity, focus))?
            .iter()
            .copied()
            .find(|&index| {
                self.pairs[index].identity == identity && *self.pairs[index].focus == *focus
            })
    }
}

pub(super) struct Execution<'a> {
    _proof: crate::recursion::closed::ClosedProof,
    options: RecursionOptions,
    state: RefCell<State<'a>>,
}

impl<'a> Execution<'a> {
    pub(super) fn new(plan: ShapePlan<'_>) -> Result<Option<Shared<Self>>, ShapesError> {
        let Some(options) = plan
            .binding()
            .validation_request()
            .recursion
            .as_ref()
            .copied()
        else {
            return Ok(None);
        };
        let proof = crate::recursion::model::prove_plan(plan)?;
        Shared::try_new(Self {
            options,
            _proof: proof,
            state: RefCell::new(State::default()),
        })
        .map(Some)
        .map_err(|_| StorageError::AllocationFailed.into())
    }
    fn seed(&self) -> bool {
        self.options.semantics == RecursionSemantics::Gfp
    }
    fn refuse(reason: RecursionRefusalReason) -> ShapesError {
        ShapesError::Recursion(RecursionRefusal::work(reason))
    }
    pub(super) fn question(
        &self,
        identity: Identity,
        focus: &FocusNode,
        body: Body<'a>,
    ) -> Result<bool, ShapesError> {
        let mut state = self.state.borrow_mut();
        let index = match state.find(identity, focus) {
            Some(index) => index,
            None => {
                if state.settled {
                    return Err(ShapesError::internal(
                        "report requested a pair outside the settled recursive run",
                    ));
                }
                let consumed =
                    u64::try_from(state.pairs.len()).map_err(|_| StorageError::SizeOverflow)?;
                if consumed >= self.options.max_pairs {
                    return Err(Self::refuse(RecursionRefusalReason::PairLimit {
                        limit: self.options.max_pairs,
                        consumed,
                    }));
                }
                let owned = Shared::try_new(focus.try_clone()?)
                    .map_err(|_| StorageError::AllocationFailed)?;
                let hash = fingerprint(identity, focus);
                state.memory(|state, memory| {
                    let index = state.pairs.len();
                    memory.push(
                        &mut state.pairs,
                        Pair {
                            identity,
                            focus: owned,
                            body,
                            value: self.seed(),
                            dependencies: Vec::new(),
                        },
                    )?;
                    if let Some(bucket) = state.index.get_mut(&hash) {
                        memory.push(bucket, index)?;
                    } else {
                        state.index.try_insert_with(
                            hash,
                            memory,
                            |a, b| Ok::<_, ShapesError>(a.cmp(b)),
                            |memory, _, _| {
                                let mut bucket = Vec::new();
                                memory.push(&mut bucket, index)?;
                                Ok::<_, ShapesError>(bucket)
                            },
                        )?;
                    }
                    state.generation = state
                        .generation
                        .checked_add(1)
                        .ok_or(StorageError::SizeOverflow)?;
                    Ok(index)
                })?
            }
        };
        if let Some(current) = state.current
            && !state.pairs[current].dependencies.contains(&index)
        {
            state.memory(|state, memory| {
                memory.push(&mut state.pairs[current].dependencies, index)?;
                state.generation = state
                    .generation
                    .checked_add(1)
                    .ok_or(StorageError::SizeOverflow)?;
                Ok(())
            })?;
        }
        Ok(state.pairs[index].value)
    }

    pub(super) fn settled(
        &self,
        identity: Identity,
        focus: &FocusNode,
    ) -> Result<(bool, Shared<FocusNode>), ShapesError> {
        let state = self.state.borrow();
        if !state.settled {
            return Err(ShapesError::internal(
                "cycle evidence requested before recursive settlement",
            ));
        }
        let index = state.find(identity, focus).ok_or_else(|| {
            ShapesError::internal("cycle is outside the settled original-pair relation")
        })?;
        Ok((state.pairs[index].value, state.pairs[index].focus.clone()))
    }

    pub(super) fn solve(
        &self,
        mut evaluate: impl FnMut(Body<'a>, &FocusNode) -> Result<bool, ShapesError>,
    ) -> Result<bool, ShapesError> {
        'graph: loop {
            let mut resident = Resident;
            let mut memory = Memory::new(&mut resident);
            let mut adjacency: Vec<Vec<usize>> = Vec::new();
            let generation = {
                let mut state = self.state.borrow_mut();
                for pair in &mut state.pairs {
                    pair.value = self.seed();
                    let row = memory.collect(pair.dependencies.iter().copied())?;
                    memory.push(&mut adjacency, row)?;
                }
                state.generation
            };
            let components = scc_component_index_with_memory::<Vec<usize>, _>(
                &adjacency,
                &mut memory,
            )
            .map_err(|error| match error {
                GraphError::Storage(error) => ShapesError::Storage(error),
                error @ GraphError::InvalidEdge { .. } => ShapesError::internal(error.to_string()),
            })?;
            for component in 0..components.iter().max().map_or(0, |last| last + 1) {
                let members = memory.collect(
                    components
                        .iter()
                        .enumerate()
                        .filter_map(|(index, &found)| (found == component).then_some(index)),
                )?;
                let recursive = members.len() > 1
                    || members
                        .first()
                        .is_some_and(|&index| adjacency[index].contains(&index));
                loop {
                    if recursive {
                        let mut state = self.state.borrow_mut();
                        let consumed = state.iterations;
                        if consumed >= self.options.max_iterations {
                            return Err(Self::refuse(RecursionRefusalReason::IterationLimit {
                                limit: self.options.max_iterations,
                                consumed,
                            }));
                        }
                        state.iterations += 1;
                    }
                    let mut next = Vec::new();
                    for &index in &members {
                        let (body, focus) = {
                            let mut state = self.state.borrow_mut();
                            state.current = Some(index);
                            (state.pairs[index].body, state.pairs[index].focus.clone())
                        };
                        let result = evaluate(body, &focus);
                        self.state.borrow_mut().current = None;
                        let answer = result?;
                        if self.state.borrow().generation != generation {
                            continue 'graph;
                        }
                        memory.push(&mut next, answer)?;
                    }
                    let mut changed = false;
                    let mut state = self.state.borrow_mut();
                    for (&index, &value) in members.iter().zip(&next) {
                        changed |= state.pairs[index].value != value;
                        state.pairs[index].value = value;
                    }
                    drop(state);
                    memory.release_vec(next)?;
                    if !changed || !recursive {
                        break;
                    }
                }
                memory.release_vec(members)?;
            }
            let mut state = self.state.borrow_mut();
            state.settled = true;
            return Ok(state.pairs[0].value);
        }
    }
}

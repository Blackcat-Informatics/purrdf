// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent positive relations and exact projected witness frontiers.
//!
//! This is a private physical path through the ordinary join and emission kernels.
//! It never collapses a callback's input multiplicity. Needed factor modes are matched
//! against the frozen store; projection tables address the resulting relations, and
//! only the product actually named by a head is visited. Partial rounds discover New
//! first and build Old/Full context only for productive anchors. Source coordinates
//! always remain authored.

use crate::store::{Arity, StoreShape};

mod execution;
pub(super) mod filters;
mod matches;
mod product;
mod relations;

pub(super) use execution::Frame;

#[cfg(test)]
use super::{
    DeltaIndexes, GroupProbe, JoinGroup, LeapfrogRun, NoGuards, Scan, delta_can_match,
    extend_rebound_solutions, extend_slot_solutions, extend_solutions_leapfrog,
};

use super::{
    ArgShape, BTreeMap, BTreeSet, Delta, EvalError, GroupArg, HeadEvidence, JoinScan, JoinSnapshot,
    PositionPlan, RelationStore, RoundBuffer, RoundSnapshot, RuleEntry, RulePlan, RuleRuntime,
    SlotSolution, StepGovernor, TermDictionary, TermId, TermIdentity, emit_solution,
    source_heights, source_preference,
};

/// One component of the positive-variable incidence graph, including negative coupling.
#[derive(Debug, Clone)]
pub(super) struct Factor {
    /// Positive operator positions, in physical order.
    positions: Vec<usize>,
    /// Global frame slots owned by this factor.
    slots: BTreeSet<usize>,
    /// Single-atom negative filters owned by this factor.
    negated: Vec<usize>,
    /// Guard-free negative conjunctions owned by this factor.
    negations: Vec<usize>,
}

/// Union every positive component touched by a negative predicate's OUTER slots.
fn couple(components: &mut [usize], slots: &[BTreeSet<usize>], outer: &[usize]) {
    let touched: Vec<_> = slots
        .iter()
        .enumerate()
        .filter(|(_, owned)| outer.iter().any(|slot| owned.contains(slot)))
        .map(|(position, _)| components[position])
        .collect();
    let Some(&first) = touched.iter().min() else {
        return;
    };
    for component in components {
        if touched.contains(component) {
            *component = first;
        }
    }
}

/// Statically certify factor independence in the existing global frame.
pub(super) fn certify<const N: usize>(plan: &RulePlan<N>, runtime: &RuleRuntime<N>) -> Vec<Factor>
where
    Arity<N>: StoreShape<N>,
{
    // Immutable-base atoms use the original indexed binary join. Their row identities
    // belong to another source, so evaluation-model delta frontiers do not apply.
    if plan
        .operators()
        .iter()
        .any(|operator| operator.shape().read_source() == crate::clause::ReadSource::Base)
        || runtime
            .negated
            .iter()
            .any(|atom| atom.source == crate::clause::ReadSource::Base)
        || runtime
            .negations
            .iter()
            .any(|group| group.sources.contains(&crate::clause::ReadSource::Base))
    {
        return Vec::new();
    }
    // Even a bindings-only guard can mint state. A negative group's callbacks also
    // observe the whole model and authored evaluation order, so neither is replayable.
    if !runtime.guards.is_empty()
        || runtime
            .negations
            .iter()
            .any(|group| !group.guards.is_empty())
    {
        return Vec::new();
    }
    let slots: Vec<BTreeSet<_>> = plan
        .operators()
        .iter()
        .map(|operator| {
            operator
                .shape()
                .positions()
                .iter()
                .filter_map(PositionPlan::slot)
                .collect()
        })
        .collect();
    let mut components: Vec<_> = (0..slots.len()).collect();
    for outer in &slots {
        couple(
            &mut components,
            &slots,
            &outer.iter().copied().collect::<Vec<_>>(),
        );
    }
    let negative_slots: Vec<Vec<_>> = runtime
        .negated
        .iter()
        .map(|atom| {
            atom.args
                .iter()
                .filter_map(|arg| match arg {
                    ArgShape::Slot(slot) => Some(*slot),
                    ArgShape::Const(_) => None,
                })
                .collect()
        })
        .collect();
    let group_slots: Vec<Vec<_>> = runtime
        .negations
        .iter()
        .map(|group| {
            group
                .atoms
                .iter()
                .flatten()
                .filter_map(|arg| match arg {
                    GroupArg::Outer(slot) => Some(*slot),
                    GroupArg::Local(_) | GroupArg::Const(_) => None,
                })
                .collect()
        })
        .collect();
    for outer in negative_slots.iter().chain(&group_slots) {
        couple(&mut components, &slots, outer);
    }
    let mut grouped: BTreeMap<usize, Factor> = BTreeMap::new();
    for (position, component) in components.into_iter().enumerate() {
        let factor = grouped.entry(component).or_insert_with(|| Factor {
            positions: Vec::new(),
            slots: BTreeSet::new(),
            negated: Vec::new(),
            negations: Vec::new(),
        });
        factor
            .positions
            .push(plan.operators()[position].positive_position());
        factor.slots.extend(&slots[position]);
    }
    let mut factors: Vec<_> = grouped.into_values().collect();
    if factors.len() < 2 {
        return Vec::new();
    }
    let owner = |outer: &[usize]| {
        factors
            .iter()
            .position(|factor| outer.iter().any(|slot| factor.slots.contains(slot)))
    };
    let atom_owners: Vec<_> = negative_slots.iter().map(|outer| owner(outer)).collect();
    let group_owners: Vec<_> = group_slots.iter().map(|outer| owner(outer)).collect();
    for (index, owner) in atom_owners.into_iter().enumerate() {
        if let Some(owner) = owner {
            factors[owner].negated.push(index);
        }
    }
    for (index, owner) in group_owners.into_iter().enumerate() {
        if let Some(owner) = owner {
            factors[owner].negations.push(index);
        }
    }
    factors
}

/// Evaluate one factor Full with the existing indexed and certified cyclic kernels.
#[cfg(test)]
fn full_matches<D: TermDictionary, const N: usize>(
    factor: &Factor,
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    factor_matches(
        factor,
        entry,
        snapshot,
        governor,
        JoinScan::Uniform(Scan::Full),
        evidence,
    )
}

/// Match one factor using the existing scan-aware binary and cyclic kernels.
#[cfg(test)]
fn factor_matches<D: TermDictionary, const N: usize>(
    factor: &Factor,
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    scan: JoinScan,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let owned_indexes = if snapshot.delta_indexes.is_none() {
        Some(DeltaIndexes::build(snapshot.rel, [entry.delta])?)
    } else {
        None
    };
    let snapshot = JoinSnapshot {
        seed: None,
        rel: snapshot.rel,
        delta: entry.delta,
        indexes: snapshot
            .delta_indexes
            .or(owned_indexes.as_ref())
            .expect("the factor indexes were prepared"),
    };
    let mut frame =
        matches::Frame::new(factor, entry, snapshot, scan).map_err(EvalError::Source)?;
    frame.advance(factor, entry, snapshot, governor, evidence)?;
    debug_assert!(frame.is_complete() || governor.spent());
    Ok(frame.into_output())
}

/// Original stack orchestration, retained solely as a differential test oracle.
#[cfg(test)]
fn reference_matches<D: TermDictionary, const N: usize>(
    factor: &Factor,
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    scan: JoinScan,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let owned_indexes = if snapshot.delta_indexes.is_none() {
        Some(DeltaIndexes::build(snapshot.rel, [entry.delta])?)
    } else {
        None
    };
    let snapshot = RoundSnapshot {
        delta_indexes: snapshot.delta_indexes.or(owned_indexes.as_ref()),
        ..snapshot
    };
    let mut partial = vec![SlotSolution::empty(entry.plan.variables().len())];
    let seed = match scan {
        JoinScan::DeltaPosition(anchor) if entry.delta != Delta::all(snapshot.rel.row_slots()) => {
            Some(anchor)
        }
        _ => None,
    };
    let seed_in_cycle = entry.plan.has_cyclic_subplan() && seed.is_some_and(|anchor| entry.plan.join_groups().iter().any(|group| {
        matches!(group, JoinGroup::Leapfrog(cycle) if cycle.atoms().iter().any(|atom| atom.positive_position() == anchor))
    }));
    if let Some(anchor) = seed {
        partial = extend_rebound_solutions(
            entry.plan.operator_at(anchor),
            snapshot.rel,
            entry.delta,
            snapshot
                .delta_indexes
                .expect("the factor's delta indexes were prepared"),
            Scan::Delta,
            partial,
            governor,
        )
        .map_err(EvalError::Source)?;
    }
    let binary =
        |position, partial: Vec<SlotSolution<D::Id, N>>, governor: &mut StepGovernor<'_>| {
            let extend = if seed.is_some() {
                extend_rebound_solutions
            } else {
                extend_slot_solutions
            };
            extend(
                entry.plan.operator_at(position),
                snapshot.rel,
                entry.delta,
                snapshot
                    .delta_indexes
                    .expect("the factor's delta indexes were prepared"),
                scan.at(position),
                partial,
                governor,
            )
        };
    if !entry.plan.has_cyclic_subplan() {
        for &position in &factor.positions {
            if seed == Some(position) {
                continue;
            }
            partial = binary(position, partial, governor).map_err(EvalError::Source)?;
            if partial.is_empty() || governor.spent() {
                break;
            }
        }
    } else {
        for group in entry.plan.join_groups() {
            if matches!(group, JoinGroup::Binary(atom) if seed == Some(atom.positive_position())) {
                continue;
            }
            let position = match group {
                JoinGroup::Binary(atom) => atom.positive_position(),
                JoinGroup::Leapfrog(cycle) => cycle.atoms()[0].positive_position(),
            };
            if !factor.positions.contains(&position) {
                continue;
            }
            partial = match group {
                JoinGroup::Binary(atom) => binary(atom.positive_position(), partial, governor)
                    .map_err(EvalError::Source)?,
                JoinGroup::Leapfrog(cycle) => extend_solutions_leapfrog(
                    &LeapfrogRun {
                        plan: entry.plan,
                        cycle,
                        scan,
                        rel: snapshot.rel,
                        delta: entry.delta,
                        indexes: snapshot
                            .delta_indexes
                            .expect("the factor's delta indexes were prepared"),
                    },
                    partial,
                    governor,
                )
                .map_err(EvalError::Source)?,
            };
            if partial.is_empty() || governor.spent() {
                break;
            }
        }
    }
    let mut matches = Vec::with_capacity(partial.len());
    for mut solution in partial {
        if governor.spent() {
            break;
        }
        if seed_in_cycle {
            solution.sources.remove(0);
        }
        solution.sources.sort_by_key(|source| source.body_index);
        let mut blocked = false;
        for &index in &factor.negated {
            if governor.spent()
                || entry.runtime.negated[index].blocks(
                    &mut solution,
                    snapshot.rel,
                    governor,
                    evidence,
                )?
            {
                blocked = true;
                break;
            }
        }
        if blocked {
            continue;
        }
        for &index in &factor.negations {
            let (holds, negative) = entry.runtime.negations[index].blocks(
                GroupProbe {
                    solution: &solution,
                    rel: snapshot.rel,
                    guards: &NoGuards,
                    rule: entry.index,
                    clause: entry.rule,
                },
                governor,
                evidence,
            )?;
            if let (Some(id), Some(owner)) = (negative, evidence.as_deref_mut()) {
                solution.retain_negative(id, owner)?;
            }
            if holds {
                blocked = true;
                break;
            }
        }
        if !blocked {
            matches.push(solution);
        }
    }
    Ok(matches)
}

/// A coordinate in the original factor relation, with its numeric prefix.
#[derive(Debug, Clone, Copy)]
struct Witness {
    /// The stored factor match; mode and projection tables never clone it.
    row: usize,
    /// Raw maximum source height, before the head's saturating increment.
    maximum: u32,
    /// Saturated sum of source heights.
    sum: u64,
}

/// Match only tuples containing a delta source, using disjoint last-new-atom anchors.
#[cfg(test)]
fn new_matches<D: TermDictionary, const N: usize>(
    factor: &Factor,
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<Vec<SlotSolution<D::Id, N>>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let join = JoinSnapshot {
        seed: None,
        rel: snapshot.rel,
        delta: entry.delta,
        indexes: snapshot
            .delta_indexes
            .expect("the factor's delta indexes were prepared"),
    };
    let mut matches = Vec::new();
    for &position in &factor.positions {
        if !delta_can_match(entry.plan.operator_at(position), join).map_err(EvalError::Source)? {
            continue;
        }
        matches.extend(factor_matches(
            factor,
            entry,
            snapshot,
            governor,
            JoinScan::DeltaPosition(position),
            evidence,
        )?);
        if governor.spent() {
            break;
        }
    }
    Ok(matches)
}

/// Matching solutions for each independent positive factor.
type FactorRelations<I, const N: usize> = Vec<Vec<SlotSolution<I, N>>>;

/// Discover productive anchors before building the old/full context they actually need.
#[cfg(test)]
fn partial_relations<D: TermDictionary, const N: usize>(
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<FactorRelations<D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut relations = Vec::with_capacity(entry.runtime.factors.len());
    for factor in &entry.runtime.factors {
        relations.push(new_matches(factor, entry, snapshot, governor, evidence)?);
        if governor.spent() {
            return Ok(Vec::new());
        }
    }
    let productive: Vec<_> = relations
        .iter()
        .map(|matches| !matches.is_empty())
        .collect();
    if !productive.iter().any(|&active| active) {
        return Ok(Vec::new());
    }
    for (position, factor) in entry.runtime.factors.iter().enumerate() {
        if productive[..position].iter().any(|&active| active) {
            // A preceding New anchor requires this factor Full. Reuse that owned
            // relation for every mode instead of retaining a second New copy.
            relations[position] = full_matches(factor, entry, snapshot, governor, evidence)?;
        } else if productive[position + 1..].iter().any(|&active| active) {
            // Only later anchors need this factor, so Full tuples are unnecessary:
            // retain its discovered New tuples and match the disjoint Old context.
            relations[position].extend(factor_matches(
                factor,
                entry,
                snapshot,
                governor,
                JoinScan::Uniform(Scan::OldOnly),
                evidence,
            )?);
        }
        if relations[position].is_empty() || governor.spent() {
            return Ok(Vec::new());
        }
    }
    Ok(relations)
}

/// Both prefix winners at a raw-height threshold.
#[derive(Debug, Clone, Copy)]
struct Threshold {
    /// Inclusive raw source-height threshold.
    height: u32,
    /// Best sum followed by lexical witness at or below this threshold.
    summed: Witness,
    /// Best purely lexical witness at or below this threshold.
    lexical: Witness,
}

/// Exact prefix frontiers for one projected binding and one scan mode.
#[derive(Debug)]
struct Frontier {
    /// Sorted distinct height thresholds; each stores both cumulative winners.
    thresholds: Vec<Threshold>,
    /// Original projected witnesses retained only by an evidence-enabled query.
    witnesses: Option<Vec<Witness>>,
}

impl Frontier {
    /// Build cumulative frontiers without assuming local height preference composes.
    fn new<D: TermDictionary, const N: usize>(
        mut witnesses: Vec<Witness>,
        matches: &[SlotSolution<D::Id, N>],
        rel: &RelationStore<D, N>,
        retain_witnesses: bool,
    ) -> Result<Self, D::Error>
    where
        Arity<N>: StoreShape<N>,
    {
        rel.interner().check_read()?;
        witnesses.sort_by_key(|witness| witness.maximum);
        let mut thresholds: Vec<Threshold> = Vec::new();
        for witness in witnesses.iter().copied() {
            let mut next = thresholds.last().copied().unwrap_or(Threshold {
                height: witness.maximum,
                summed: witness,
                lexical: witness,
            });
            let lexical = |left: Witness, right: Witness| {
                source_preference(
                    &matches[left.row].sources,
                    0,
                    &matches[right.row].sources,
                    0,
                    rel,
                )
                .map(core::cmp::Ordering::is_lt)
            };
            if witness.sum < next.summed.sum
                || (witness.sum == next.summed.sum && lexical(witness, next.summed)?)
            {
                next.summed = witness;
            }
            if lexical(witness, next.lexical)? {
                next.lexical = witness;
            }
            next.height = witness.maximum;
            if thresholds
                .last()
                .is_some_and(|previous| previous.height == next.height)
            {
                *thresholds.last_mut().expect("the last threshold exists") = next;
            } else {
                thresholds.push(next);
            }
        }
        rel.interner().check_read()?;
        Ok(Self {
            thresholds,
            witnesses: retain_witnesses.then_some(witnesses),
        })
    }

    /// Minimum attainable raw maximum.
    fn minimum_height(&self) -> u32 {
        self.thresholds[0].height
    }

    /// Both exact winners under a feasible global raw-height threshold.
    fn at(&self, maximum: u32) -> Threshold {
        let end = self
            .thresholds
            .partition_point(|threshold| threshold.height <= maximum);
        self.thresholds[end - 1]
    }
}

/// Projected head identities and their source witness frontiers.
type FrontierTable = Vec<Frontier>;

/// A drive-local view pairing retained coordinates with their original relation.
/// The execution owns the relation and frontiers separately; no self-borrow persists.
pub(super) struct Witnesses<'a, I: TermIdentity = TermId, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    frontier: &'a Frontier,
    relation: &'a [SlotSolution<I, N>],
}

impl<I: TermIdentity, const N: usize> Witnesses<'_, I, N>
where
    Arity<N>: StoreShape<N>,
{
    pub(super) fn witnesses(&self) -> impl Iterator<Item = &SlotSolution<I, N>> {
        self.frontier
            .witnesses
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|witness| &self.relation[witness.row])
    }
}

/// Mode frontiers addressing one frozen factor relation containing its needed modes.
#[derive(Debug)]
struct Projection {
    /// Any matching source tuple.
    full: FrontierTable,
    /// Tuples having no source in the current delta.
    old: FrontierTable,
    /// Tuples having at least one source in the current delta.
    new: FrontierTable,
}

/// The original disjoint first-new-factor decomposition, in factor order.
fn mode_tables(projections: &[Projection], anchor: usize) -> Vec<&[Frontier]> {
    projections
        .iter()
        .enumerate()
        .map(|(position, projection)| match position.cmp(&anchor) {
            std::cmp::Ordering::Less => projection.old.as_slice(),
            std::cmp::Ordering::Equal => projection.new.as_slice(),
            std::cmp::Ordering::Greater => projection.full.as_slice(),
        })
        .collect()
}

/// Project one head's factor relation; callers consume only its required modes.
fn project<D: TermDictionary, const N: usize>(
    matches: &[SlotSolution<D::Id, N>],
    slots: &[usize],
    snapshot: RoundSnapshot<'_, D, N>,
    delta: Delta,
    retain_witnesses: bool,
) -> Result<Projection, D::Error>
where
    Arity<N>: StoreShape<N>,
{
    snapshot.rel.interner().check_read()?;
    let mut full: BTreeMap<Vec<D::Id>, Vec<Witness>> = BTreeMap::new();
    let mut old: BTreeMap<Vec<D::Id>, Vec<Witness>> = BTreeMap::new();
    let mut new: BTreeMap<Vec<D::Id>, Vec<Witness>> = BTreeMap::new();
    for (row, solution) in matches.iter().enumerate() {
        let key: Vec<_> = slots
            .iter()
            .map(|&slot| {
                solution
                    .get(slot)
                    .expect("a factor binds its positive slots")
            })
            .collect();
        let (maximum, sum) = source_heights(&solution.sources, snapshot.depth);
        let witness = Witness { row, maximum, sum };
        full.entry(key.clone()).or_default().push(witness);
        let mode = if solution
            .sources
            .iter()
            .any(|source| delta.contains(source.row))
        {
            &mut new
        } else {
            &mut old
        };
        mode.entry(key).or_default().push(witness);
    }
    let seal = |table: BTreeMap<Vec<D::Id>, Vec<Witness>>| {
        table
            .into_values()
            .map(|witnesses| Frontier::new(witnesses, matches, snapshot.rel, retain_witnesses))
            .collect::<Result<_, D::Error>>()
    };
    let projection = Projection {
        full: seal(full)?,
        old: seal(old)?,
        new: seal(new)?,
    };
    snapshot.rel.interner().check_read()?;
    Ok(projection)
}

/// Feasible raw threshold for the minimum saturated global head proof height.
fn global_threshold(minimum_raw: u32) -> u32 {
    let proof = minimum_raw.saturating_add(1);
    if proof == u32::MAX {
        u32::MAX
    } else {
        proof - 1
    }
}

/// Whether the global minimum sum makes every feasible sum tie by saturation.
fn saturated_sum(sums: impl IntoIterator<Item = u64>) -> bool {
    sums.into_iter().fold(0u64, u64::saturating_add) == u64::MAX
}

/// Immutable state for streamed projected products.
struct Product<'a, 'r, D: TermDictionary, const N: usize = 4>
where
    Arity<N>: StoreShape<N>,
{
    /// One mode table per certified factor.
    tables: Vec<&'a [Frontier]>,
    /// Original owned relations addressed by the corresponding mode tables.
    relations: &'a [Vec<SlotSolution<D::Id, N>>],
    /// The authored head currently emitted.
    head: &'r [ArgShape; N],
    /// The position in the authored head conjunction.
    head_ordinal: usize,
    /// Rule identity and variable-frame width.
    entry: RuleEntry<'r, N>,
    /// Store and source proof heights.
    snapshot: RoundSnapshot<'a, D, N>,
}

impl<'a, 'r, D: TermDictionary, const N: usize> Product<'a, 'r, D, N>
where
    Arity<N>: StoreShape<N>,
{
    fn new(
        entry: RuleEntry<'r, N>,
        snapshot: RoundSnapshot<'a, D, N>,
        relations: &'a [Vec<SlotSolution<D::Id, N>>],
        projections: &'a [Projection],
        head_ordinal: usize,
        anchor: usize,
    ) -> Self {
        Self {
            tables: mode_tables(projections, anchor),
            relations,
            head: &entry.runtime.head[head_ordinal],
            head_ordinal,
            entry,
            snapshot,
        }
    }

    /// Stream necessary projected combinations through the owned coordinate cursor.
    #[cfg(test)]
    fn visit(
        &self,
        buffer: &mut RoundBuffer<'r, D::Id, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        let mut frame = product::Frame::new(&self.tables);
        frame.advance(self, buffer, governor, evidence)?;
        debug_assert!(frame.is_complete() || governor.spent());
        Ok(())
    }

    /// Original full-run traversal, retained only as an independent cursor oracle.
    #[cfg(test)]
    fn reference_visit(
        &self,
        buffer: &mut RoundBuffer<'r, D::Id, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
    ) -> Result<(), EvalError<D::Error>> {
        let mut cursors: Vec<_> = self.tables.iter().map(|table| table.iter()).collect();
        let mut chosen = Vec::with_capacity(self.tables.len());
        let mut position = 0;
        while !governor.spent() {
            if let Some(frontier) = cursors[position].next() {
                chosen.push(Witnesses {
                    frontier,
                    relation: &self.relations[position],
                });
                if chosen.len() == self.tables.len() {
                    self.emit(&chosen, buffer, governor, evidence)?;
                    chosen.pop();
                } else {
                    position += 1;
                }
            } else if position == 0 {
                break;
            } else {
                cursors[position] = self.tables[position].iter();
                position -= 1;
                chosen.pop();
            }
        }
        Ok(())
    }

    /// Choose the exact global witness for one projected combination, then emit it.
    fn emit(
        &self,
        chosen: &[Witnesses<'_, D::Id, N>],
        buffer: &mut RoundBuffer<'r, D::Id, N>,
        governor: &mut StepGovernor<'_>,
        evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
    ) -> Result<bool, EvalError<D::Error>> {
        if !governor.charge() {
            return Ok(false);
        }
        // Height masking means a factor's locally shortest proof may not minimize the
        // global sum. Saturated sums in turn mask that sum: pure lex then decides.
        let threshold = global_threshold(
            chosen
                .iter()
                .map(|view| view.frontier.minimum_height())
                .max()
                .unwrap_or(0),
        );
        let winners: Vec<_> = chosen
            .iter()
            .map(|view| view.frontier.at(threshold))
            .collect();
        let lexical = saturated_sum(winners.iter().map(|winner| winner.summed.sum));
        let mut solution = SlotSolution::empty(self.entry.plan.variables().len());
        for (winner, view) in winners.into_iter().zip(chosen) {
            let witness = if lexical {
                winner.lexical
            } else {
                winner.summed
            };
            let original = &view.relation[witness.row];
            for (slot, binding) in original.bindings.iter().enumerate() {
                if binding.is_some() {
                    solution.bindings[slot] = *binding;
                }
            }
            solution.sources.extend_from_slice(&original.sources);
        }
        solution.sources.sort_by_key(|source| source.body_index);
        let witness = match evidence.as_mut() {
            Some(owner) => Some(owner.product(chosen)?),
            None => None,
        };
        emit_solution(
            buffer,
            self.head,
            &solution,
            self.entry.index,
            self.snapshot,
            evidence,
            HeadEvidence {
                head: self.head_ordinal,
                witness,
                uses_absence: !self.entry.runtime.negated.is_empty()
                    || !self.entry.runtime.negations.is_empty(),
            },
        )?;
        Ok(true)
    }
}

/// Evaluate independent factors and stream each conjunctive head's necessary product.
#[cfg(test)]
pub(super) fn evaluate<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    evaluate_with_evidence(entry, snapshot, governor, None)
}

#[cfg(test)]
pub(super) fn evaluate_with_evidence<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    mut evidence: Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let owned_indexes = if snapshot.delta_indexes.is_none() {
        Some(DeltaIndexes::build(snapshot.rel, [entry.delta])?)
    } else {
        None
    };
    let snapshot = RoundSnapshot {
        delta_indexes: snapshot.delta_indexes.or(owned_indexes.as_ref()),
        ..snapshot
    };
    let mut frame = Frame::new();
    frame.advance(entry, snapshot, governor, &mut evidence)?;
    debug_assert!(frame.is_complete() || governor.spent());
    let mut buffer = frame.into_buffer();
    buffer.join_steps = governor.consumed;
    Ok(buffer)
}

/// Original whole-factor orchestration, retained only as a full-run test oracle.
#[cfg(test)]
fn reference_evaluate_with_evidence<'r, D: TermDictionary, const N: usize>(
    entry: RuleEntry<'r, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    mut evidence: Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let owned_indexes = if snapshot.delta_indexes.is_none() {
        Some(DeltaIndexes::build(snapshot.rel, [entry.delta])?)
    } else {
        None
    };
    let snapshot = RoundSnapshot {
        delta_indexes: snapshot.delta_indexes.or(owned_indexes.as_ref()),
        ..snapshot
    };
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    let result = (|| -> Result<RoundBuffer<'r, D::Id, N>, EvalError<D::Error>> {
        let mut buffer = RoundBuffer::new();
        let join = JoinSnapshot {
            seed: None,
            rel: snapshot.rel,
            delta: entry.delta,
            indexes: snapshot
                .delta_indexes
                .expect("the factor's delta indexes were prepared"),
        };
        let mut can_match = false;
        for operator in entry.plan.operators() {
            if delta_can_match(operator, join).map_err(EvalError::Source)? {
                can_match = true;
                break;
            }
        }
        if !can_match {
            return Ok(buffer);
        }
        let (blocked, ground_evidence) = ground_blocked(entry, snapshot, governor, &mut evidence)?;
        if blocked {
            buffer.join_steps = governor.consumed;
            return Ok(buffer);
        }
        let mut relations = if entry.delta == Delta::all(snapshot.rel.row_slots()) {
            let mut relations = Vec::with_capacity(entry.runtime.factors.len());
            for factor in &entry.runtime.factors {
                let matches = full_matches(factor, entry, snapshot, governor, &mut evidence)?;
                if matches.is_empty() || governor.spent() {
                    buffer.join_steps = governor.consumed;
                    return Ok(buffer);
                }
                relations.push(matches);
            }
            relations
        } else {
            partial_relations(entry, snapshot, governor, &mut evidence)?
        };
        if relations.is_empty() {
            buffer.join_steps = governor.consumed;
            return Ok(buffer);
        }
        // A round-wide negative premise belongs to every product. Attach it to one
        // existing factor's witnesses so the original product law carries it once.
        if let (Some(id), Some(owner)) = (ground_evidence, evidence.as_deref_mut()) {
            for solution in &mut relations[0] {
                solution.retain_negative(id, owner)?;
            }
        }
        for (head_ordinal, head) in entry.runtime.head.iter().enumerate() {
            let projections: Vec<_> = entry
                .runtime
                .factors
                .iter()
                .zip(&relations)
                .map(|(factor, matches)| {
                    let slots: BTreeSet<_> = head
                        .iter()
                        .filter_map(|arg| match arg {
                            ArgShape::Slot(slot) if factor.slots.contains(slot) => Some(*slot),
                            ArgShape::Slot(_) | ArgShape::Const(_) => None,
                        })
                        .collect();
                    project(
                        matches,
                        &slots.into_iter().collect::<Vec<_>>(),
                        snapshot,
                        entry.delta,
                        evidence.is_some(),
                    )
                })
                .collect::<Result<_, _>>()
                .map_err(EvalError::Source)?;
            for anchor in 0..projections.len() {
                let product = Product::new(
                    entry,
                    snapshot,
                    &relations,
                    &projections,
                    head_ordinal,
                    anchor,
                );
                if product.tables.iter().any(|table| table.is_empty()) {
                    continue;
                }
                product.visit(&mut buffer, governor, &mut evidence)?;
                if governor.spent() {
                    break;
                }
            }
            if governor.spent() {
                break;
            }
        }
        buffer.join_steps = governor.consumed;
        Ok(buffer)
    })()?;
    snapshot
        .rel
        .interner()
        .check_read()
        .map_err(EvalError::Source)?;
    Ok(result)
}

/// Negative predicates with no positive outer slot are round-wide preconditions.
/// Local variables remain local even when their authored name appears in another group.
#[cfg(test)]
fn ground_blocked<D: TermDictionary, const N: usize>(
    entry: RuleEntry<'_, N>,
    snapshot: RoundSnapshot<'_, D, N>,
    governor: &mut StepGovernor<'_>,
    evidence: &mut Option<&mut dyn super::evidence::RuleEvidence<D, N>>,
) -> Result<(bool, Option<purrdf_core::evidence::EvidenceId>), EvalError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut empty = SlotSolution::empty(entry.plan.variables().len());
    for (index, atom) in entry.runtime.negated.iter().enumerate() {
        if governor.spent() {
            return Ok((true, empty.negative_evidence));
        }
        if !entry
            .runtime
            .factors
            .iter()
            .any(|factor| factor.negated.contains(&index))
            && atom.blocks(&mut empty, snapshot.rel, governor, evidence)?
        {
            return Ok((true, empty.negative_evidence));
        }
    }
    for (index, group) in entry.runtime.negations.iter().enumerate() {
        if governor.spent() {
            return Ok((true, empty.negative_evidence));
        }
        if !entry
            .runtime
            .factors
            .iter()
            .any(|factor| factor.negations.contains(&index))
        {
            let (blocked, negative) = group.blocks(
                GroupProbe {
                    solution: &empty,
                    rel: snapshot.rel,
                    guards: &NoGuards,
                    rule: entry.index,
                    clause: entry.rule,
                },
                governor,
                evidence,
            )?;
            if let (Some(id), Some(owner)) = (negative, evidence.as_deref_mut()) {
                empty.retain_negative(id, owner)?;
            }
            if blocked {
                return Ok((true, empty.negative_evidence));
            }
        }
    }
    Ok((false, empty.negative_evidence))
}

#[cfg(test)]
mod tests {
    use super::{
        Frontier, Witness, Witnesses, full_matches, global_threshold, project, saturated_sum,
    };
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
    use crate::guard::{Guard, Negation, NoGuards};
    use crate::plan::RulePlan;
    use crate::seminaive::{
        Delta, DeltaIndexes, EvalOptions, JoinStrategy, RoundExecution, RoundSnapshot, RuleEntry,
        RuleRuntime, SlotSolution, SourceRow, StepGovernor, compile, evaluate_rule, evaluate_with,
        source_heights, source_preference,
    };
    use crate::store::RelationStore;
    use crate::test_support::permute;

    const A: &str = "https://example.org/a";
    const B: &str = "https://example.org/b";
    const C: &str = "https://example.org/c";
    const Q: &str = "https://example.org/q";

    fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
        ClauseAtom::positive(ClauseTerm::var(subject), predicate, ClauseTerm::var(object))
    }

    fn constant(name: &str) -> ClauseTerm {
        ClauseTerm::iri(format!("https://example.org/{name}"))
    }

    fn surface(name: &str) -> String {
        format!("<https://example.org/{name}>")
    }

    fn seeded(rows: &[(String, String, String, String)]) -> RelationStore {
        let mut store = RelationStore::new();
        for (subject, predicate, object, graph) in rows {
            store.insert(subject, predicate, object, graph).unwrap();
        }
        store
    }

    fn rows(n: usize) -> Vec<(String, String, String, String)> {
        (0..n)
            .flat_map(|index| {
                [
                    (
                        surface(&format!("a{index}")),
                        format!("<{A}>"),
                        surface("value"),
                        RelationStore::DEFAULT_GRAPH.to_owned(),
                    ),
                    (
                        surface(&format!("b{index}")),
                        format!("<{B}>"),
                        surface("value"),
                        RelationStore::DEFAULT_GRAPH.to_owned(),
                    ),
                ]
            })
            .collect()
    }

    #[test]
    fn host_projection_retains_ids_and_fallible_lexical_frontiers() {
        use crate::seminaive::{JoinSnapshot, join_body};
        use crate::test_support::{HostDictionary, SourceLost};
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        const FIRST: u64 = 0xa000_0000_1234_5678;
        let mut rel = RelationStore::with_dictionary(HostDictionary {
            surfaces: [
                (FIRST, "aaa".into()),
                (0, "zzz".into()),
                (u64::MAX, "value".into()),
                (7, format!("<{A}>")),
                (9, format!("<{Q}>")),
                (11, format!("<{B}>")),
                (1, String::new()),
            ]
            .into_iter()
            .collect(),
            resolves: AtomicUsize::new(0),
            lookups: AtomicUsize::new(0),
            lost: AtomicBool::new(false),
            lose_after_lookup: AtomicBool::new(false),
            pins: AtomicUsize::new(0),
            refuse_after_interned: AtomicBool::new(false),
            extra_terms: 1u64 << 40,
        });
        for subject in [0, FIRST] {
            rel.insert_ids([subject, 7, u64::MAX, 1]).unwrap();
        }
        let rule = DlClause::datalog(atom("?x", Q, "?v"), vec![atom("?x", A, "?v")]);
        let plan = RulePlan::for_rule(&rule);
        let matches = join_body(
            &plan,
            JoinSnapshot {
                seed: None,
                rel: &rel,
                delta: Delta::all(2),
                indexes: &DeltaIndexes { runs: Vec::new() },
            },
            JoinStrategy::Planned,
            &mut StepGovernor::new(u64::MAX),
            None,
        )
        .unwrap();
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: (&[0, 0][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        let delta = Delta { lo: 1, hi: 2 };
        let projection = project(&matches, &[], snapshot, delta, true).unwrap();
        // Frontiers must survive an actual relation move without a self-borrow.
        // Allocate the replacement while the old buffer is still alive.
        let original = matches.as_ptr();
        let mut relocated = Vec::with_capacity(matches.len() + 1);
        relocated.extend(matches);
        assert_ne!(original, relocated.as_ptr());
        let matches = relocated;
        let full = projection.full.first().unwrap();
        assert_eq!(matches[full.at(0).lexical.row].sources[0].terms[0], FIRST);
        assert_eq!(matches[full.at(0).summed.row].sources[0].terms[0], FIRST);
        assert_eq!(
            Witnesses {
                frontier: full,
                relation: &matches
            }
            .witnesses()
            .count(),
            2
        );
        assert_eq!(
            matches[projection.old.first().unwrap().at(0).lexical.row].sources[0].terms[0],
            0
        );
        assert_eq!(
            matches[projection.new.first().unwrap().at(0).lexical.row].sources[0].terms[0],
            FIRST
        );
        let slot = plan
            .variables()
            .iter()
            .position(|name| name == "?x")
            .unwrap();
        let bound = project(&matches, &[slot], snapshot, delta, false).unwrap();
        assert_eq!(
            bound
                .full
                .iter()
                .map(|frontier| matches[frontier.at(0).lexical.row].get(slot).unwrap())
                .collect::<Vec<_>>(),
            vec![0, FIRST]
        );
        assert_eq!(rel.interner().pins.load(Ordering::Relaxed), 0);
        drop((projection, bound));
        // Both production rule strategies consume the same original host IDs,
        // including disjoint factor products and each semi-naive delta anchor.
        let factor_rule = DlClause::datalog(
            atom("?x", Q, "?y"),
            vec![atom("?x", A, "?v"), atom("?y", A, "?w")],
        );
        let factor_plan = RulePlan::for_rule(&factor_rule);
        let runtime = RuleRuntime::new(&factor_rule, &factor_plan);
        assert_eq!(runtime.factors.len(), 2);
        for delta in [
            Delta::all(2),
            Delta { lo: 0, hi: 1 },
            Delta { lo: 1, hi: 2 },
        ] {
            let entry = RuleEntry {
                index: 0,
                rule: &factor_rule,
                plan: &factor_plan,
                runtime: &runtime,
                delta,
            };
            let planned = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::Planned,
                StepGovernor::new(u64::MAX),
                &NoGuards,
                None,
            )
            .unwrap();
            let binary = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::ForcedBinary,
                StepGovernor::new(u64::MAX),
                &NoGuards,
                None,
            )
            .unwrap();
            assert_eq!(
                planned.entries.len(),
                if delta.lo == 0 && delta.hi == 2 { 4 } else { 3 }
            );
            assert_eq!(planned.entries.len(), binary.entries.len());
            for (key, witness) in &planned.entries {
                assert!(matches!(key.positions[0].interned(), Some(0 | FIRST)));
                assert!(matches!(key.positions[2].interned(), Some(0 | FIRST)));
                assert_eq!(key.positions[3].interned(), Some(1));
                assert_eq!(
                    witness.source_facts(&rel).unwrap(),
                    binary.entries.get(key).unwrap().source_facts(&rel).unwrap()
                );
            }
        }
        assert_eq!(rel.interner().pins.load(Ordering::Relaxed), 0);
        let entry = RuleEntry {
            index: 0,
            rule: &factor_rule,
            plan: &factor_plan,
            runtime: &runtime,
            delta: Delta::all(2),
        };
        let entries = [entry, RuleEntry { index: 1, ..entry }];
        for execution in [RoundExecution::Sequential, RoundExecution::Parallel] {
            let round = crate::seminaive::evaluate_round(
                &entries,
                snapshot,
                execution,
                JoinStrategy::Planned,
                u64::MAX,
                super::super::RoundControl {
                    guards: &NoGuards,
                    stop: None,
                },
            )
            .unwrap();
            assert_eq!(round.entries.len(), 4);
            assert!(round.entries.values().all(|candidate| candidate.rule == 0));
            for key in round.entries.keys() {
                assert!(matches!(key.positions[0].interned(), Some(0 | FIRST)));
                assert!(matches!(key.positions[2].interned(), Some(0 | FIRST)));
            }
        }
        // An omitted dictionary entry fails the actual lexical tie comparison and
        // releases the earlier lexical pins; it must not choose by numeric ID.
        let mut missing = matches;
        missing[1].sources[0].terms[2] = 17;
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: (&[0, 0][..]).into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        assert!(matches!(
            project(&missing, &[], snapshot, delta, true),
            Err(SourceLost(17))
        ));
        assert_eq!(rel.interner().pins.load(Ordering::Relaxed), 0);
        rel.interner().lost.store(true, Ordering::Relaxed);
        assert!(matches!(
            project(&[], &[], snapshot, delta, false),
            Err(SourceLost(99))
        ));
        assert!(matches!(
            crate::seminaive::evaluate_round(
                &[],
                snapshot,
                RoundExecution::Parallel,
                JoinStrategy::Planned,
                0,
                super::super::RoundControl {
                    guards: &NoGuards,
                    stop: None
                },
            ),
            Err(crate::seminaive::EvalError::Source(SourceLost(99)))
        ));
        for strategy in [JoinStrategy::Planned, JoinStrategy::ForcedBinary] {
            let entry = RuleEntry {
                index: 0,
                rule: &factor_rule,
                plan: &factor_plan,
                runtime: &runtime,
                delta: Delta { lo: 2, hi: 2 },
            };
            assert!(matches!(
                evaluate_rule(
                    entry,
                    snapshot,
                    strategy,
                    StepGovernor::new(0),
                    &NoGuards,
                    None
                ),
                Err(crate::seminaive::EvalError::Source(SourceLost(99)))
            ));
        }
        rel.interner().lost.store(false, Ordering::Relaxed);
        let exe = compile(vec![
            factor_rule,
            DlClause::datalog(atom("?x", B, "?y"), vec![atom("?x", Q, "?y")]),
        ])
        .unwrap();
        let result = crate::seminaive::evaluate_guarded(
            &exe,
            rel,
            &NoGuards,
            &EvalOptions::default().with_derivation_mode(crate::seminaive::DerivationMode::Record),
            None,
        )
        .unwrap();
        assert_eq!(result.facts().row_count(), 10);
        assert_eq!(
            result
                .derivations()
                .expect("derivation recording was requested")
                .len(),
            8
        );
        assert_eq!(result.budget().input_terms(), (1u64 << 40) + 7);
        assert_eq!(result.budget().generated_terms(), 0);
        for (_, ids) in result.facts().rows_from(2) {
            assert!(matches!(ids[0], 0 | FIRST));
            assert!(matches!(ids[1], 9 | 11));
            assert!(matches!(ids[2], 0 | FIRST));
            assert_eq!(ids[3], 1);
        }
        assert_eq!(result.facts().interner().pins.load(Ordering::Relaxed), 0);
        result
            .facts()
            .interner()
            .lost
            .store(true, Ordering::Relaxed);
        assert!(matches!(
            crate::seminaive::evaluate_guarded(
                &compile(Vec::new()).unwrap(),
                result.into_facts(),
                &NoGuards,
                &EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                None
            ),
            Err(crate::seminaive::EvalError::Source(SourceLost(99)))
        ));
    }

    /// Exhaustive sparse input subsets, insertion permutations and every head projection.
    /// The existing binary evaluator is an independent non-factorized instrument, while
    /// the analytic answer and authored minimum lexical premises establish its oracle.
    #[test]
    fn projected_factors_match_exhaustive_facts_and_canonical_authored_witnesses() {
        let universe = rows(3);
        for mask in 0..64u32 {
            let input: Vec<_> = universe
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, row)| row.clone())
                .collect();
            for projection in 0..3 {
                let head = match projection {
                    0 => ClauseAtom::positive(constant("constant"), Q, constant("result")),
                    1 => ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
                    _ => atom("?x", Q, "?y"),
                };
                let rule = DlClause::datalog(head, vec![atom("?y", B, "?v"), atom("?x", A, "?u")]);
                let exe = compile(vec![rule]).expect("a positive program compiles");
                for seed in 0..2 {
                    let input = permute(&input, seed);
                    let planned = evaluate_with(
                        &exe,
                        seeded(&input),
                        None,
                        &NoGuards,
                        EvalOptions::default()
                            .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                        RoundExecution::Sequential,
                        JoinStrategy::Planned,
                    )
                    .expect("small factored input completes");
                    let binary = evaluate_with(
                        &exe,
                        seeded(&input),
                        None,
                        &NoGuards,
                        EvalOptions::default()
                            .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                        RoundExecution::Sequential,
                        JoinStrategy::ForcedBinary,
                    )
                    .expect("small exhaustive input completes");
                    assert_eq!(
                        planned.facts().facts_sorted(),
                        binary.facts().facts_sorted(),
                        "mask {mask}, projection {projection}, seed {seed}"
                    );
                    assert_eq!(
                        planned
                            .derivations()
                            .expect("derivation recording was requested"),
                        binary
                            .derivations()
                            .expect("derivation recording was requested"),
                        "mask {mask}, projection {projection}, seed {seed}"
                    );
                    let a: Vec<_> = input
                        .iter()
                        .filter(|row| row.1 == format!("<{A}>"))
                        .collect();
                    let b: Vec<_> = input
                        .iter()
                        .filter(|row| row.1 == format!("<{B}>"))
                        .collect();
                    let expected = if a.is_empty() || b.is_empty() {
                        0
                    } else {
                        match projection {
                            0 => 1,
                            1 => a.len(),
                            _ => a.len() * b.len(),
                        }
                    };
                    assert_eq!(
                        planned
                            .derivations()
                            .expect("derivation recording was requested")
                            .len(),
                        expected
                    );
                    for derivation in planned
                        .derivations()
                        .expect("derivation recording was requested")
                    {
                        assert_eq!(derivation.proof_height, 1);
                        assert_eq!(derivation.sources.len(), 2);
                        assert_eq!(derivation.sources[0].predicate, format!("<{B}>"));
                        assert_eq!(derivation.sources[1].predicate, format!("<{A}>"));
                        let wanted_b = if projection == 2 {
                            &derivation.fact.object
                        } else {
                            &b.iter()
                                .map(|row| &row.0)
                                .min()
                                .expect("a firing has a B witness")
                                .clone()
                        };
                        assert_eq!(&derivation.sources[0].subject, wanted_b);
                        let wanted_a = if projection == 0 {
                            a.iter()
                                .map(|row| &row.0)
                                .min()
                                .expect("a firing has an A witness")
                        } else {
                            &derivation.fact.subject
                        };
                        assert_eq!(&derivation.sources[1].subject, wanted_a);
                    }
                }
            }
        }
    }

    /// Existential components contribute linear matching work rather than an unused
    /// Cartesian product; a head that actually needs both components still emits it.
    #[test]
    fn existential_work_is_additive_and_required_head_products_are_streamed() {
        for n in [10, 100, 1000] {
            let input = rows(n);
            let exe = compile(vec![DlClause::datalog(
                ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
                vec![atom("?y", B, "?v"), atom("?x", A, "?u")],
            )])
            .expect("the rule compiles");
            let evaluation = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("linear input stays within default limits");
            assert_eq!(
                evaluation
                    .derivations()
                    .expect("derivation recording was requested")
                    .len(),
                n
            );
            assert_eq!(evaluation.budget().join_steps, 3 * n as u64);
        }
    }

    /// A recursive component must join its delta, not rebuild the growing closure
    /// merely because a separate ground precondition also occurs in the body.
    #[test]
    fn recursive_factor_with_ground_precondition_keeps_quadratic_work() {
        let rule = DlClause::datalog(
            atom("?x", A, "?z"),
            vec![
                atom("?x", A, "?y"),
                atom("?y", B, "?z"),
                ClauseAtom::positive(constant("enabled"), C, constant("value")),
            ],
        );
        let exe = compile(vec![rule]).expect("positive recursion compiles");
        for n in [32usize, 64, 128, 256] {
            let mut input = vec![(
                surface("enabled"),
                format!("<{C}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            )];
            for position in 0..n - 1 {
                for predicate in [A, B] {
                    input.push((
                        surface(&format!("node{position}")),
                        format!("<{predicate}>"),
                        surface(&format!("node{}", position + 1)),
                        RelationStore::DEFAULT_GRAPH.to_owned(),
                    ));
                }
            }
            let run = |strategy| {
                evaluate_with(
                    &exe,
                    seeded(&input),
                    None,
                    &NoGuards,
                    EvalOptions::default()
                        .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                    RoundExecution::Sequential,
                    strategy,
                )
                .expect("the complete chain fits unchanged default limits")
            };
            let planned = run(JoinStrategy::Planned);
            let binary = run(JoinStrategy::ForcedBinary);
            assert_eq!(
                planned.facts().facts_sorted(),
                binary.facts().facts_sorted()
            );
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested"),
                binary
                    .derivations()
                    .expect("derivation recording was requested")
            );
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested")
                    .len(),
                (n - 1) * (n - 2) / 2
            );
            assert!(
                planned.budget().join_steps <= 4 * (n * n) as u64,
                "recursive factor work must stay quadratic: n={n}, report={:?}",
                planned.budget()
            );
        }
    }

    /// Partial-delta evaluation still reduces a large independent existential
    /// dimension instead of returning to a cross-factor binary product.
    #[test]
    fn partial_delta_one_factor_heads_keep_additive_work_and_exact_witnesses() {
        for reverse in [false, true] {
            let mut body = vec![atom("?y", B, "?v"), atom("?x", A, "?u")];
            if reverse {
                body.reverse();
            }
            let rule = DlClause::datalog(
                ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
                body,
            );
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            for n in [10usize, 100, 1000] {
                let mut input = rows(n);
                let old = input.len();
                input.extend((0..n).map(|index| {
                    (
                        surface(&format!("new{index}")),
                        format!("<{A}>"),
                        surface("value"),
                        RelationStore::DEFAULT_GRAPH.to_owned(),
                    )
                }));
                let rel = seeded(&input);
                let depth = vec![0; input.len()];
                let snapshot = RoundSnapshot {
                    probe: None,
                    rel: &rel,
                    depth: depth.as_slice().into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                };
                let entry = RuleEntry {
                    index: 0,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta: Delta {
                        lo: old as u64,
                        hi: input.len() as u64,
                    },
                };
                let mut governor = StepGovernor::new(u64::MAX);
                let planned =
                    super::evaluate(entry, snapshot, &mut governor).expect("partial factors match");
                assert_eq!(planned.entries.len(), n);
                assert!(
                    governor.consumed <= 4 * n as u64,
                    "unused existential products must remain additive"
                );
                if n == 1000 {
                    // The small cases compare every exhaustive witness. This larger
                    // growth control need not allocate a million oracle source frames.
                    continue;
                }
                let binary = evaluate_rule(
                    entry,
                    snapshot,
                    JoinStrategy::ForcedBinary,
                    StepGovernor::new(u64::MAX),
                    &NoGuards,
                    None,
                )
                .expect("exhaustive partial join matches");
                assert_eq!(planned.entries.len(), binary.entries.len());
                for (key, candidate) in &planned.entries {
                    let expected = binary.entries.get(key).expect("same projected fact");
                    assert_eq!(
                        candidate.source_facts(&rel).unwrap(),
                        expected.source_facts(&rel).unwrap()
                    );
                }
            }
        }
    }

    /// Stored Full relations and projected prefix entries grow additively. Mode
    /// frontiers retain source coordinates, not cloned Cartesian assignments.
    #[test]
    fn factor_relations_and_projected_frontier_storage_grow_additively() {
        let rule = DlClause::datalog(
            ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
            vec![atom("?y", B, "?v"), atom("?x", A, "?u")],
        );
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let head_slot = plan
            .variables()
            .iter()
            .position(|name| name == "?x")
            .expect("head slot exists");
        for n in [10, 100, 1000] {
            let input = rows(n);
            let rel = seeded(&input);
            let depth = vec![0; input.len()];
            let snapshot = RoundSnapshot {
                probe: None,
                rel: &rel,
                depth: depth.as_slice().into(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            };
            let entry = RuleEntry {
                index: 0,
                rule: &rule,
                plan: &plan,
                runtime: &runtime,
                delta: Delta::all(input.len() as u64),
            };
            let mut governor = StepGovernor::new(u64::MAX);
            let mut stored_rows = 0;
            let mut stored_thresholds = 0;
            for factor in &runtime.factors {
                let matches = full_matches(factor, entry, snapshot, &mut governor, &mut None)
                    .expect("Full factor completes");
                stored_rows += matches.len();
                let slots = if factor.slots.contains(&head_slot) {
                    vec![head_slot]
                } else {
                    Vec::new()
                };
                let projection = project(&matches, &slots, snapshot, entry.delta, false).unwrap();
                assert!(projection.old.is_empty());
                for frontier in projection.full.iter().chain(&projection.new) {
                    stored_thresholds += frontier.thresholds.len();
                    for threshold in &frontier.thresholds {
                        for witness in [threshold.summed, threshold.lexical] {
                            let original = &matches[witness.row];
                            assert_eq!(original.sources.len(), factor.positions.len());
                            assert_eq!(
                                source_heights(&original.sources, depth.as_slice().into()),
                                (witness.maximum, witness.sum)
                            );
                        }
                    }
                }
            }
            assert_eq!(stored_rows, 2 * n);
            assert_eq!(stored_thresholds, 2 * (n + 1));
            assert_eq!(governor.consumed, 2 * n as u64);
        }
    }

    /// The first-new-factor decomposition is disjoint and excludes wholly old firings.
    #[test]
    fn factor_modes_preserve_delta_eligibility_and_share_conjunctive_bodies() {
        let rule = DlClause::new(
            vec![HeadDisjunct::new(vec![
                atom("?x", Q, "?y"),
                ClauseAtom::positive(constant("constant"), C, constant("result")),
            ])],
            Vec::new(),
            vec![atom("?x", A, "?u"), atom("?y", B, "?v")],
        );
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let input = rows(3);
        let rel = seeded(&input);
        let depth = vec![0; input.len()];
        for lo in 0..=input.len() {
            let entry = RuleEntry {
                index: 0,
                rule: &rule,
                plan: &plan,
                runtime: &runtime,
                delta: Delta {
                    lo: lo as u64,
                    hi: input.len() as u64,
                },
            };
            let snapshot = RoundSnapshot {
                probe: None,
                rel: &rel,
                depth: depth.as_slice().into(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            };
            let planned = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::Planned,
                StepGovernor::new(u64::MAX),
                &NoGuards,
                None,
            )
            .expect("factor round completes");
            let binary = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::ForcedBinary,
                StepGovernor::new(u64::MAX),
                &NoGuards,
                None,
            )
            .expect("exhaustive round completes");
            assert_eq!(planned.entries, binary.entries, "delta lower bound {lo}");
            assert!(
                planned.join_steps <= 6 + 12 + 2,
                "both heads reuse the single Full factor relations"
            );
        }
    }

    /// A certified cyclic factor supports uniform OldOnly and productive Delta
    /// matching through the same cursors, including simultaneous factor anchors.
    #[test]
    fn partial_hybrid_factor_modes_preserve_exhaustive_witnesses() {
        let independent = "https://example.org/independent";
        let rule = DlClause::datalog(
            atom("?x", Q, "?y"),
            vec![
                atom("?x", A, "?y"),
                atom("?y", B, "?z"),
                atom("?z", C, "?x"),
                atom("?u", independent, "?v"),
            ],
        );
        let plan = RulePlan::for_rule(&rule);
        assert!(plan.has_cyclic_subplan());
        let runtime = RuleRuntime::new(&rule, &plan);
        assert_eq!(runtime.factors.len(), 2);
        let mut input = Vec::new();
        for index in 0..2 {
            for (subject, predicate, object) in [
                ("a", A, "b"),
                ("b", B, "c"),
                ("c", C, "a"),
                ("u", independent, "v"),
            ] {
                input.push((
                    surface(&format!("{subject}{index}")),
                    format!("<{predicate}>"),
                    surface(&format!("{object}{index}")),
                    RelationStore::DEFAULT_GRAPH.to_owned(),
                ));
            }
        }
        for seed in 0..4 {
            let shuffled = permute(&input, seed);
            let rel = seeded(&shuffled);
            let depth: Vec<_> = (0..input.len())
                .map(|index| u32::try_from(index % 3).expect("three source heights"))
                .collect();
            let snapshot = RoundSnapshot {
                probe: None,
                rel: &rel,
                depth: depth.as_slice().into(),
                assumed: &purrdf_core::FastSet::default(),
                delta_indexes: None,
            };
            for lo in 0..=input.len() {
                for hi in lo..=input.len() {
                    let entry = RuleEntry {
                        index: 0,
                        rule: &rule,
                        plan: &plan,
                        runtime: &runtime,
                        delta: Delta {
                            lo: lo as u64,
                            hi: hi as u64,
                        },
                    };
                    let run = |strategy| {
                        evaluate_rule(
                            entry,
                            snapshot,
                            strategy,
                            StepGovernor::new(u64::MAX),
                            &NoGuards,
                            None,
                        )
                        .expect("hybrid factor round matches")
                    };
                    let planned = run(JoinStrategy::Planned);
                    let binary = run(JoinStrategy::ForcedBinary);
                    assert_eq!(
                        planned.entries, binary.entries,
                        "seed={seed}, delta={lo}..{hi}"
                    );
                }
            }
        }

        // The new cycle edge has no complete cyclic match. The large independent
        // relation must not be rebuilt merely because its partition exists.
        let mut input = vec![
            (
                surface("a"),
                format!("<{A}>"),
                surface("b"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("b"),
                format!("<{B}>"),
                surface("c"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("c"),
                format!("<{C}>"),
                surface("a"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
        ];
        input.extend((0..1000).map(|index| {
            (
                surface(&format!("u{index}")),
                format!("<{independent}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            )
        }));
        let old = input.len();
        input.push((
            surface("unmatched"),
            format!("<{A}>"),
            surface("missing"),
            RelationStore::DEFAULT_GRAPH.to_owned(),
        ));
        let rel = seeded(&input);
        let depth = vec![0; input.len()];
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: depth.as_slice().into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        let entry = RuleEntry {
            index: 0,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta: Delta {
                lo: old as u64,
                hi: input.len() as u64,
            },
        };
        let mut governor = StepGovernor::new(u64::MAX);
        let buffer = super::evaluate(entry, snapshot, &mut governor)
            .expect("empty New factor refuses no query");
        assert!(buffer.entries.is_empty());
        assert!(
            governor.consumed < 20,
            "unproductive New must skip unrelated Full/Old matching"
        );
    }

    /// Productive recursive partial rounds share exact credits across workers,
    /// including the refused observation at zero and one below completion.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn partial_factor_rounds_preserve_exact_worker_budget_refusals() {
        use crate::seminaive::{BudgetResource, EvalError};

        let mut input = vec![(
            surface("enabled"),
            format!("<{C}>"),
            surface("value"),
            RelationStore::DEFAULT_GRAPH.to_owned(),
        )];
        let mut rules = Vec::new();
        for index in 0..32 {
            let predicate = format!("https://example.org/reach{index}");
            rules.push(DlClause::datalog(
                atom("?x", &predicate, "?z"),
                vec![
                    atom("?x", &predicate, "?y"),
                    atom("?y", B, "?z"),
                    ClauseAtom::positive(constant("enabled"), C, constant("value")),
                ],
            ));
            for position in 0..7 {
                input.push((
                    surface(&format!("node{position}")),
                    format!("<{predicate}>"),
                    surface(&format!("node{}", position + 1)),
                    RelationStore::DEFAULT_GRAPH.to_owned(),
                ));
            }
        }
        input.extend((0..7).map(|position| {
            (
                surface(&format!("node{position}")),
                format!("<{B}>"),
                surface(&format!("node{}", position + 1)),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            )
        }));
        let exe = compile(rules).expect("independent recursive rules compile");
        let run = |options, execution| {
            evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                options,
                execution,
                JoinStrategy::Planned,
            )
        };
        let reference = run(
            EvalOptions::default().with_derivation_mode(crate::seminaive::DerivationMode::Record),
            RoundExecution::Sequential,
        )
        .expect("recursive reference completes");
        let expected = reference.budget().join_steps();
        let exact_reference = run(
            EvalOptions::default()
                .with_derivation_mode(crate::seminaive::DerivationMode::Record)
                .with_max_join_steps(expected),
            RoundExecution::Sequential,
        )
        .expect("sequential exact credits complete");
        let mut refused = Vec::new();
        for workers in [1, 4, 32] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .expect("test worker pool");
            pool.install(|| {
                let exact = run(
                    EvalOptions::default()
                        .with_derivation_mode(crate::seminaive::DerivationMode::Record)
                        .with_max_join_steps(expected),
                    RoundExecution::Parallel,
                )
                .expect("exact partial-round credits complete");
                assert_eq!(
                    exact.facts().facts_sorted(),
                    reference.facts().facts_sorted()
                );
                assert_eq!(
                    exact
                        .derivations()
                        .expect("derivation recording was requested"),
                    reference
                        .derivations()
                        .expect("derivation recording was requested")
                );
                assert_eq!(exact.budget(), exact_reference.budget());
                let reports: Vec<_> = [0, expected - 1]
                    .into_iter()
                    .map(|ceiling| {
                        let error = run(
                            EvalOptions::default()
                                .with_derivation_mode(crate::seminaive::DerivationMode::Record)
                                .with_max_join_steps(ceiling),
                            RoundExecution::Parallel,
                        )
                        .expect_err("insufficient partial-round credits refuse");
                        let EvalError::BudgetExhausted { resource, report } = error else {
                            panic!("{error:?}");
                        };
                        assert_eq!(resource, BudgetResource::JoinSteps);
                        assert_eq!(report.join_steps(), ceiling + 1);
                        report
                    })
                    .collect();
                if refused.is_empty() {
                    refused = reports;
                } else {
                    assert_eq!(reports, refused, "workers={workers}");
                }
            });
        }
    }

    /// Negative outer variables couple factors, while a conjunction's local variable
    /// does not. Ground negative predicates retain their existential truth value.
    #[test]
    fn negative_coupling_and_local_scopes_match_exhaustive_evaluation() {
        let base = vec![
            atom("?x", A, "?u"),
            atom("?y", B, "?v"),
            atom("?z", C, "?w"),
        ];
        let negative = ClauseAtom::negated_quad(
            ClauseTerm::var("?x"),
            ClauseTerm::iri("https://example.org/block"),
            ClauseTerm::var("?y"),
            ClauseTerm::DefaultGraph,
        );
        let mut body = base.clone();
        body.push(negative);
        let rules = [
            DlClause::datalog(atom("?x", Q, "?y"), body),
            DlClause::datalog(atom("?x", Q, "?y"), base.clone()).with_negations(vec![
                Negation::new(
                    vec![
                        atom("?x", "https://example.org/block", "?local"),
                        atom("?local", "https://example.org/end", "?y"),
                    ],
                    Vec::new(),
                ),
            ]),
            DlClause::datalog(atom("?x", Q, "?y"), base.clone()).with_negations(vec![
                Negation::new(
                    vec![atom("?x", "https://example.org/block", "?local")],
                    Vec::new(),
                ),
            ]),
            DlClause::datalog(atom("?x", Q, "?y"), base).with_negations(vec![Negation::new(
                vec![ClauseAtom::positive(
                    constant("ground"),
                    "https://example.org/block",
                    constant("ground"),
                )],
                Vec::new(),
            )]),
        ];
        let mut input = rows(3);
        input.extend([
            (
                surface("c0"),
                format!("<{C}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("a0"),
                surface("block"),
                surface("b0"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("b0"),
                surface("end"),
                surface("b1"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
        ]);
        for (index, rule) in rules.into_iter().enumerate() {
            let plan = RulePlan::for_rule(&rule);
            let runtime = RuleRuntime::new(&rule, &plan);
            assert_eq!(runtime.factors.len(), if index < 2 { 2 } else { 3 });
            let exe = compile(vec![rule.clone()]).expect("stratified negatives compile");
            for seed in 0..4 {
                let shuffled = permute(&input, seed);
                let rel = seeded(&shuffled);
                let depth = vec![0; shuffled.len()];
                let snapshot = RoundSnapshot {
                    probe: None,
                    rel: &rel,
                    depth: depth.as_slice().into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                };
                for lo in 0..=shuffled.len() {
                    for hi in lo..=shuffled.len() {
                        let entry = RuleEntry {
                            index: 0,
                            rule: &rule,
                            plan: &plan,
                            runtime: &runtime,
                            delta: Delta {
                                lo: lo as u64,
                                hi: hi as u64,
                            },
                        };
                        let run = |strategy| {
                            evaluate_rule(
                                entry,
                                snapshot,
                                strategy,
                                StepGovernor::new(u64::MAX),
                                &NoGuards,
                                None,
                            )
                            .expect("negative factor round completes")
                        };
                        assert_eq!(
                            run(JoinStrategy::Planned).entries,
                            run(JoinStrategy::ForcedBinary).entries,
                            "negative case={index}, seed={seed}, delta={lo}..{hi}"
                        );
                    }
                }
                let planned = evaluate_with(
                    &exe,
                    seeded(&shuffled),
                    None,
                    &NoGuards,
                    EvalOptions::default()
                        .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                    RoundExecution::Sequential,
                    JoinStrategy::Planned,
                )
                .expect("small factor model completes");
                let binary = evaluate_with(
                    &exe,
                    seeded(&shuffled),
                    None,
                    &NoGuards,
                    EvalOptions::default()
                        .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                    RoundExecution::Sequential,
                    JoinStrategy::ForcedBinary,
                )
                .expect("small exhaustive model completes");
                assert_eq!(
                    planned.facts().facts_sorted(),
                    binary.facts().facts_sorted()
                );
                assert_eq!(
                    planned
                        .derivations()
                        .expect("derivation recording was requested"),
                    binary
                        .derivations()
                        .expect("derivation recording was requested")
                );
            }
        }
    }

    #[test]
    fn opaque_body_and_negative_callbacks_disable_factor_collapse() {
        let body = vec![atom("?x", A, "?u"), atom("?y", B, "?v")];
        let body_guard = DlClause::datalog(atom("?x", Q, "?y"), body.clone())
            .with_guards(vec![Guard::filter("stateful", vec!["?x".to_owned()])]);
        let negative_guard =
            DlClause::datalog(atom("?x", Q, "?y"), body).with_negations(vec![Negation::new(
                Vec::new(),
                vec![Guard::filter("stateful", vec!["?x".to_owned()])],
            )]);
        for rule in [body_guard, negative_guard] {
            let plan = RulePlan::for_rule(&rule);
            assert!(RuleRuntime::new(&rule, &plan).factors.is_empty());
        }
    }

    #[test]
    fn ground_negative_preconditions_block_before_positive_matching() {
        let rule = DlClause::datalog(
            atom("?x", Q, "?y"),
            vec![atom("?x", A, "?u"), atom("?y", B, "?v")],
        )
        .with_negations(vec![Negation::new(
            vec![ClauseAtom::positive(
                constant("ground"),
                C,
                constant("ground"),
            )],
            Vec::new(),
        )]);
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let mut input = rows(1000);
        input.push((
            surface("ground"),
            format!("<{C}>"),
            surface("ground"),
            RelationStore::DEFAULT_GRAPH.to_owned(),
        ));
        let rel = seeded(&input);
        let depth = vec![0; input.len()];
        let snapshot = RoundSnapshot {
            probe: None,
            rel: &rel,
            depth: depth.as_slice().into(),
            assumed: &purrdf_core::FastSet::default(),
            delta_indexes: None,
        };
        let entry = RuleEntry {
            index: 0,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta: Delta::all(input.len() as u64),
        };
        let buffer = evaluate_rule(
            entry,
            snapshot,
            JoinStrategy::Planned,
            StepGovernor::new(3),
            &NoGuards,
            None,
        )
        .expect("a false body requires no positive product");
        assert!(buffer.entries.is_empty());
        assert_eq!(
            buffer.join_steps, 3,
            "group, partition and matching row; no positive product"
        );
    }

    /// Predicate and graph are projected values and connectivity edges, just like
    /// subject and object. The analytic head product here needs all four factors.
    #[test]
    fn all_four_positions_participate_in_factor_certification_and_head_projection() {
        let quad =
            |subject, predicate, object, graph| ClauseAtom::quad(subject, predicate, object, graph);
        let body = vec![
            ClauseAtom::positive(ClauseTerm::var("?s"), A, constant("value")),
            quad(
                constant("predicate-source"),
                ClauseTerm::var("?p"),
                constant("value"),
                ClauseTerm::DefaultGraph,
            ),
            ClauseAtom::positive(constant("object-source"), C, ClauseTerm::var("?o")),
            quad(
                constant("graph-source"),
                ClauseTerm::iri("https://example.org/d"),
                constant("value"),
                ClauseTerm::var("?g"),
            ),
        ];
        let rule = DlClause::datalog(
            quad(
                ClauseTerm::var("?s"),
                ClauseTerm::var("?p"),
                ClauseTerm::var("?o"),
                ClauseTerm::var("?g"),
            ),
            body,
        );
        let plan = RulePlan::for_rule(&rule);
        assert_eq!(RuleRuntime::new(&rule, &plan).factors.len(), 4);
        let mut input = Vec::new();
        for index in 0..2 {
            input.extend([
                (
                    surface(&format!("s{index}")),
                    format!("<{A}>"),
                    surface("value"),
                    RelationStore::DEFAULT_GRAPH.to_owned(),
                ),
                (
                    surface("predicate-source"),
                    surface(&format!("p{index}")),
                    surface("value"),
                    RelationStore::DEFAULT_GRAPH.to_owned(),
                ),
                (
                    surface("object-source"),
                    format!("<{C}>"),
                    surface(&format!("o{index}")),
                    RelationStore::DEFAULT_GRAPH.to_owned(),
                ),
                (
                    surface("graph-source"),
                    surface("d"),
                    surface("value"),
                    surface(&format!("g{index}")),
                ),
            ]);
        }
        let exe = compile(vec![rule]).expect("the variable-predicate program is positive");
        for seed in 0..4 {
            let input = permute(&input, seed);
            let planned = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("the factor model completes");
            let binary = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::ForcedBinary,
            )
            .expect("the exhaustive model completes");
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested"),
                binary
                    .derivations()
                    .expect("derivation recording was requested")
            );
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested")
                    .len(),
                16
            );
            for proof in planned
                .derivations()
                .expect("derivation recording was requested")
            {
                assert_eq!(proof.fact.subject, proof.sources[0].subject);
                assert_eq!(proof.fact.predicate, proof.sources[1].predicate);
                assert_eq!(proof.fact.object, proof.sources[2].object);
                assert_eq!(proof.fact.graph, proof.sources[3].graph);
            }
        }
        let coupled = DlClause::datalog(
            atom("?x", Q, "?y"),
            vec![
                quad(
                    ClauseTerm::var("?x"),
                    ClauseTerm::var("?p"),
                    constant("value"),
                    ClauseTerm::var("?g"),
                ),
                quad(
                    ClauseTerm::var("?y"),
                    ClauseTerm::var("?p"),
                    constant("other"),
                    ClauseTerm::DefaultGraph,
                ),
                quad(
                    constant("fixed"),
                    ClauseTerm::iri(C),
                    ClauseTerm::var("?z"),
                    ClauseTerm::var("?g"),
                ),
                atom("?independent", B, "?unused"),
            ],
        );
        let plan = RulePlan::for_rule(&coupled);
        let runtime = RuleRuntime::new(&coupled, &plan);
        assert_eq!(runtime.factors.len(), 2);
        assert!(
            runtime
                .factors
                .iter()
                .any(|factor| factor.positions.len() == 3)
        );
    }

    /// Identical sorted premise multisets still compare authored source order.
    #[test]
    fn equal_sorted_source_facts_preserve_the_authored_lexical_tie_break() {
        let rule = DlClause::datalog(
            ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
            vec![
                atom("?z", B, "?y"),
                atom("?y", B, "?z"),
                atom("?x", A, "?u"),
            ],
        );
        let input = vec![
            (
                surface("a"),
                format!("<{B}>"),
                surface("z"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("z"),
                format!("<{B}>"),
                surface("a"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("head"),
                format!("<{A}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
        ];
        let exe = compile(vec![rule]).expect("the positive program compiles");
        for seed in 0..4 {
            let input = permute(&input, seed);
            let planned = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("factor evaluation completes");
            let binary = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default()
                    .with_derivation_mode(crate::seminaive::DerivationMode::Record),
                RoundExecution::Sequential,
                JoinStrategy::ForcedBinary,
            )
            .expect("exhaustive evaluation completes");
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested"),
                binary
                    .derivations()
                    .expect("derivation recording was requested")
            );
            assert_eq!(
                planned
                    .derivations()
                    .expect("derivation recording was requested")[0]
                    .sources[0]
                    .subject,
                surface("a")
            );
        }
    }

    /// A larger local height can be globally invisible but have a smaller summed height.
    /// Saturation at MAX also admits both MAX-1 and MAX raw source heights.
    #[test]
    fn global_height_masking_selects_the_exact_exhaustive_witness() {
        let body = vec![
            atom("?x", A, "?u"),
            atom("?y", B, "?z"),
            atom("?z", C, "?w"),
        ];
        let rule = DlClause::datalog(
            ClauseAtom::positive(ClauseTerm::var("?x"), Q, constant("result")),
            body,
        );
        let plan = RulePlan::for_rule(&rule);
        let runtime = RuleRuntime::new(&rule, &plan);
        let input = vec![
            (
                surface("head"),
                format!("<{A}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("b0"),
                format!("<{B}>"),
                surface("z0"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("z0"),
                format!("<{C}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("b1"),
                format!("<{B}>"),
                surface("z1"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
            (
                surface("z1"),
                format!("<{C}>"),
                surface("value"),
                RelationStore::DEFAULT_GRAPH.to_owned(),
            ),
        ];
        for seed in 0..4 {
            let input = permute(&input, seed);
            let rel = seeded(&input);
            for heights in [
                [10, 4, 4, 5, 0],
                [u32::MAX - 1, 4, 4, 5, 0],
                [u32::MAX, 4, 4, 5, 0],
                [u32::MAX - 1, u32::MAX - 1, u32::MAX - 1, u32::MAX, 0],
            ] {
                let mut depth = vec![0; input.len()];
                for fact in rel.facts_sorted() {
                    let index = match (fact.subject.as_str(), fact.predicate.as_str()) {
                        (subject, _) if subject == surface("head") => 0,
                        (subject, _) if subject == surface("b0") => 1,
                        (subject, _) if subject == surface("z0") => 2,
                        (subject, _) if subject == surface("b1") => 3,
                        _ => 4,
                    };
                    let source = input
                        .iter()
                        .position(|quad| quad.0 == fact.subject && quad.1 == fact.predicate)
                        .expect("the fact is seeded");
                    depth[source] = heights[index];
                }
                let snapshot = RoundSnapshot {
                    probe: None,
                    rel: &rel,
                    depth: depth.as_slice().into(),
                    assumed: &purrdf_core::FastSet::default(),
                    delta_indexes: None,
                };
                for lo in 0..=input.len() {
                    for hi in lo..=input.len() {
                        let delta = Delta {
                            lo: lo as u64,
                            hi: hi as u64,
                        };
                        let entry = RuleEntry {
                            index: 0,
                            rule: &rule,
                            plan: &plan,
                            runtime: &runtime,
                            delta,
                        };
                        let planned = evaluate_rule(
                            entry,
                            snapshot,
                            JoinStrategy::Planned,
                            StepGovernor::new(u64::MAX),
                            &NoGuards,
                            None,
                        )
                        .expect("factored round completes");
                        let binary = evaluate_rule(
                            entry,
                            snapshot,
                            JoinStrategy::ForcedBinary,
                            StepGovernor::new(u64::MAX),
                            &NoGuards,
                            None,
                        )
                        .expect("exhaustive round completes");
                        assert_eq!(
                            planned.entries, binary.entries,
                            "seed={seed}, heights={heights:?}, delta={lo}..{hi}"
                        );
                        if delta == Delta::all(input.len() as u64) {
                            let winner = planned.entries.values().next().expect("one head fires");
                            assert_eq!(
                                winner.source_facts(&rel).unwrap()[1].subject,
                                surface("b1")
                            );
                        }
                    }
                }
            }
        }
    }

    /// Synthetic u64 sums exercise a reachable arithmetic law without fabricating an
    /// impossible multi-billion-row fixture merely to saturate the real source fold.
    #[test]
    fn saturation_thresholds_and_both_frontiers_obey_neighboring_boundaries() {
        assert_eq!(global_threshold(0), 0);
        assert_eq!(global_threshold(u32::MAX - 2), u32::MAX - 2);
        assert_eq!(global_threshold(u32::MAX - 1), u32::MAX);
        assert_eq!(global_threshold(u32::MAX), u32::MAX);
        assert!(!saturated_sum([u64::MAX - 2, 1]));
        assert!(saturated_sum([u64::MAX - 2, 2]));
        assert!(saturated_sum([u64::MAX - 2, 3]));
        let mut rel = RelationStore::new();
        let mut solution = |name: &str, body_index| {
            let subject = surface(name);
            let predicate = format!("<{A}>");
            let object = surface("value");
            let (_, _, row) = rel
                .insert(&subject, &predicate, &object, RelationStore::DEFAULT_GRAPH)
                .unwrap()
                .expect("unique source");
            let mut solution = SlotSolution::empty(0);
            solution.sources.push(SourceRow {
                source: crate::clause::ReadSource::Evaluation,
                body_index,
                terms: [
                    rel.term_id(&subject).expect("interned subject"),
                    rel.term_id(&predicate).expect("interned predicate"),
                    rel.term_id(&object).expect("interned object"),
                    rel.term_id(RelationStore::DEFAULT_GRAPH)
                        .expect("interned graph"),
                ],
                row,
            });
            solution
        };
        let lexical = solution("a", 0);
        let summed = solution("z", 0);
        assert!(
            source_preference(&lexical.sources, 0, &lexical.sources, 1, &rel)
                .unwrap()
                .is_lt()
        );
        assert!(
            source_preference(&lexical.sources, 9, &summed.sources, 0, &rel)
                .unwrap()
                .is_lt()
        );
        let matches = [lexical, summed];
        let frontier = Frontier::new(
            vec![
                Witness {
                    row: 0,
                    maximum: u32::MAX,
                    sum: u64::MAX,
                },
                Witness {
                    row: 1,
                    maximum: u32::MAX - 1,
                    sum: u64::MAX - 1,
                },
            ],
            &matches,
            &rel,
            false,
        )
        .unwrap();
        assert_eq!(matches[frontier.at(u32::MAX - 1).lexical.row], matches[1]);
        assert_eq!(matches[frontier.at(u32::MAX).summed.row], matches[1]);
        assert_eq!(matches[frontier.at(u32::MAX).lexical.row], matches[0]);
        assert!(saturated_sum([frontier.at(u32::MAX).summed.sum, 1]));
        assert_eq!(
            source_heights(&matches[0].sources, (&[u32::MAX, u32::MAX][..]).into()),
            (u32::MAX, u64::from(u32::MAX))
        );
    }
}
